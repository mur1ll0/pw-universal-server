//! O formato GNET: **big-endian**, com `CompactUINT` prefixando tamanhos.
//!
//! As regras vêm de `share/common/marshal_i386.h` e `share/common/byteorder_i386.h`
//! dos fontes originais do servidor:
//!
//! * todo escalar vai para o fio em ordem de rede (`byteorder_32` é `bswap` num host
//!   little-endian). `float` e `double` também, por *bitcast* para inteiro — não é o
//!   formato IEEE "nativo do host", é o mesmo padrão de bits em ordem invertida;
//! * `Octets` e `std::string` = `CompactUINT(len)` seguido dos bytes, **sem
//!   terminador nulo**;
//! * contêineres (`std::vector`, `set`, `list`, `deque`, `map`, `RpcDataVector`) =
//!   `CompactUINT(count)` seguido dos elementos;
//! * `std::pair` = os dois elementos em sequência, **sem** prefixo de contagem.
//!
//! # `CompactUINT`
//!
//! Quatro formas, escolhidas pela magnitude do valor. O prefixo nos bits altos do
//! primeiro byte diz qual delas foi usada:
//!
//! | Faixa | Bytes | Codificação |
//! | :--- | :--- | :--- |
//! | `0 ..= 0x7F` | 1 | o valor cru (bit 7 em zero) |
//! | `0x80 ..= 0x3FFF` | 2 | `valor \| 0x8000`, big-endian |
//! | `0x4000 ..= 0x1FFF_FFFF` | 4 | `valor \| 0xC000_0000`, big-endian |
//! | acima | 5 | o byte `0xE0`, depois o valor em 4 bytes big-endian |
//!
//! As fronteiras são exatamente onde erros de implementação se escondem, então elas
//! têm teste próprio (e apenas elas mudam de forma: `0x7F`/`0x80`, `0x3FFF`/`0x4000`,
//! `0x1FFF_FFFF`/`0x2000_0000`).

use crate::error::{WireError, WireResult};

/// Limite superior (exclusivo) de cada forma do `CompactUINT`.
const COMPACT_1BYTE_MAX: u32 = 0x80;
const COMPACT_2BYTE_MAX: u32 = 0x4000;
const COMPACT_4BYTE_MAX: u32 = 0x2000_0000;

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

/// Leitor sequencial sobre um buffer emprestado.
///
/// `Octets` e strings são devolvidos como fatias do buffer original, sem cópia: o
/// consumidor decide se precisa de uma cópia própria.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Consome `n` bytes crus.
    pub fn raw(&mut self, n: usize) -> WireResult<&'a [u8]> {
        if self.remaining() < n {
            return Err(WireError::Underflow {
                offset: self.pos,
                needed: n,
                available: self.remaining(),
            });
        }
        let out = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    fn array<const N: usize>(&mut self) -> WireResult<[u8; N]> {
        let slice = self.raw(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    pub fn u8(&mut self) -> WireResult<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub fn i8(&mut self) -> WireResult<i8> {
        Ok(self.u8()? as i8)
    }

    /// `bool` ocupa um byte; qualquer valor diferente de zero é verdadeiro.
    pub fn bool(&mut self) -> WireResult<bool> {
        Ok(self.u8()? != 0)
    }

    pub fn u16(&mut self) -> WireResult<u16> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub fn i16(&mut self) -> WireResult<i16> {
        Ok(i16::from_be_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> WireResult<u32> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub fn i32(&mut self) -> WireResult<i32> {
        Ok(i32::from_be_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> WireResult<u64> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub fn i64(&mut self) -> WireResult<i64> {
        Ok(i64::from_be_bytes(self.array()?))
    }

    /// `float` vai para o fio por *bitcast*: os mesmos 4 bytes do `f32`, em ordem de
    /// rede. Não é a representação nativa do host reordenada por acaso — é o que o
    /// `marshal` original faz explicitamente.
    pub fn f32(&mut self) -> WireResult<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn f64(&mut self) -> WireResult<f64> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// Lê um `CompactUINT`.
    pub fn compact_uint(&mut self) -> WireResult<u32> {
        let first = self.u8()?;
        if first & 0x80 == 0 {
            // 0xxxxxxx
            return Ok(u32::from(first));
        }
        if first & 0xC0 == 0x80 {
            // 10xxxxxx xxxxxxxx
            let b = self.u8()?;
            return Ok((u32::from(first & 0x3F) << 8) | u32::from(b));
        }
        if first & 0xE0 == 0xC0 {
            // 110xxxxx xxxxxxxx xxxxxxxx xxxxxxxx
            let rest = self.array::<3>()?;
            return Ok((u32::from(first & 0x1F) << 24)
                | (u32::from(rest[0]) << 16)
                | (u32::from(rest[1]) << 8)
                | u32::from(rest[2]));
        }
        if first == 0xE0 {
            return self.u32();
        }
        Err(WireError::InvalidCompactUint(first))
    }

    /// Lê um bloco `Octets`: `CompactUINT(len)` seguido dos bytes.
    ///
    /// O tamanho anunciado é conferido contra o que resta **antes** de qualquer
    /// alocação, para que um pacote corrompido não vire um pedido de memória enorme.
    pub fn octets(&mut self) -> WireResult<&'a [u8]> {
        let len = self.compact_uint()? as usize;
        if len > self.remaining() {
            return Err(WireError::LengthTooLarge {
                announced: len as u64,
                available: self.remaining(),
            });
        }
        self.raw(len)
    }

    /// Lê uma `std::string`. Tem o mesmo formato de `Octets` e **não** traz terminador
    /// nulo; a interpretação dos bytes é de quem chama.
    pub fn string(&mut self) -> WireResult<&'a [u8]> {
        self.octets()
    }

    /// Lê a contagem de um contêiner (`vector`, `set`, `list`, `deque`, `map`).
    ///
    /// Confere a contagem contra o buffer restante, para que uma contagem corrompida
    /// não vire uma alocação gigante antes de alguém perceber que o pacote é lixo.
    ///
    /// A premissa é que **todo elemento ocupa pelo menos um byte**, e portanto não
    /// pode haver mais elementos do que bytes restantes. Isso vale para todas as 620
    /// estruturas do IR — a única forma de quebrar seria um contêiner de estruturas
    /// sem nenhum campo, que escreveriam zero byte cada. Se um esquema futuro tiver
    /// isso, é aqui que ele vai falhar, e o conserto é conferir no chamador, que
    /// conhece o tamanho do elemento.
    pub fn seq_len(&mut self) -> WireResult<usize> {
        let n = self.compact_uint()? as usize;
        if n > self.remaining() {
            return Err(WireError::LengthTooLarge {
                announced: n as u64,
                available: self.remaining(),
            });
        }
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

/// Escritor sequencial.
#[derive(Debug, Clone, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            buf: Vec::with_capacity(n),
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    pub fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn i8(&mut self, v: i8) {
        self.buf.push(v as u8);
    }

    pub fn bool(&mut self, v: bool) {
        self.buf.push(u8::from(v));
    }

    pub fn u16(&mut self, v: u16) {
        self.raw(&v.to_be_bytes());
    }

    pub fn i16(&mut self, v: i16) {
        self.raw(&v.to_be_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.raw(&v.to_be_bytes());
    }

    pub fn i32(&mut self, v: i32) {
        self.raw(&v.to_be_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.raw(&v.to_be_bytes());
    }

    pub fn i64(&mut self, v: i64) {
        self.raw(&v.to_be_bytes());
    }

    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    pub fn f64(&mut self, v: f64) {
        self.u64(v.to_bits());
    }

    /// Escreve um `CompactUINT` na forma mais curta que couber.
    pub fn compact_uint(&mut self, v: u32) {
        if v < COMPACT_1BYTE_MAX {
            self.u8(v as u8);
        } else if v < COMPACT_2BYTE_MAX {
            self.u16((v | 0x8000) as u16);
        } else if v < COMPACT_4BYTE_MAX {
            self.u32(v | 0xC000_0000);
        } else {
            self.u8(0xE0);
            self.u32(v);
        }
    }

    pub fn octets(&mut self, data: &[u8]) {
        self.compact_uint(data.len() as u32);
        self.raw(data);
    }

    pub fn string(&mut self, data: &[u8]) {
        self.octets(data);
    }

    pub fn seq_len(&mut self, n: usize) {
        self.compact_uint(n as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalares_vao_em_ordem_de_rede() {
        let mut w = Writer::new();
        w.u32(0x1122_3344);
        w.i16(-2);
        assert_eq!(w.as_slice(), &[0x11, 0x22, 0x33, 0x44, 0xFF, 0xFE]);

        let mut r = Reader::new(w.as_slice());
        assert_eq!(r.u32().unwrap(), 0x1122_3344);
        assert_eq!(r.i16().unwrap(), -2);
        assert!(r.is_empty());
    }

    #[test]
    fn float_vai_por_bitcast_e_nao_pela_representacao_do_host() {
        // 1.0f32 é 0x3F800000. Em ordem de rede são esses quatro bytes na ordem em que
        // se lê o número — o host é little-endian, então uma cópia crua daria o
        // inverso.
        let mut w = Writer::new();
        w.f32(1.0);
        assert_eq!(w.as_slice(), &[0x3F, 0x80, 0x00, 0x00]);
        assert_eq!(Reader::new(w.as_slice()).f32().unwrap(), 1.0);
    }

    #[test]
    fn compact_uint_muda_de_forma_exatamente_nas_fronteiras() {
        // É aqui que erro de implementação se esconde: um `<=` no lugar de um `<` só
        // aparece nestes seis valores.
        let casos: &[(u32, usize)] = &[
            (0, 1),
            (0x7F, 1),
            (0x80, 2),
            (0x3FFF, 2),
            (0x4000, 4),
            (0x1FFF_FFFF, 4),
            (0x2000_0000, 5),
            (u32::MAX, 5),
        ];
        for &(valor, bytes) in casos {
            let mut w = Writer::new();
            w.compact_uint(valor);
            assert_eq!(w.len(), bytes, "tamanho errado para {valor:#x}");
            let lido = Reader::new(w.as_slice()).compact_uint().unwrap();
            assert_eq!(lido, valor, "ida e volta falhou para {valor:#x}");
        }
    }

    #[test]
    fn compact_uint_usa_a_codificacao_do_marshal_original() {
        // Padrões de bits exatos, conferidos contra as quatro formas do
        // `marshal_i386.h`. Uma ida-e-volta consigo mesmo passaria mesmo com a
        // codificação errada; estes bytes não.
        let esperado: &[(u32, &[u8])] = &[
            (0x7F, &[0x7F]),
            (0x80, &[0x80, 0x80]),
            (0x3FFF, &[0xBF, 0xFF]),
            (0x4000, &[0xC0, 0x00, 0x40, 0x00]),
            (0x1FFF_FFFF, &[0xDF, 0xFF, 0xFF, 0xFF]),
            (0x2000_0000, &[0xE0, 0x20, 0x00, 0x00, 0x00]),
        ];
        for &(valor, bytes) in esperado {
            let mut w = Writer::new();
            w.compact_uint(valor);
            assert_eq!(w.as_slice(), bytes, "codificação errada para {valor:#x}");
        }
    }

    #[test]
    fn octets_nao_tem_terminador_nulo() {
        let mut w = Writer::new();
        w.octets(b"abc");
        assert_eq!(w.as_slice(), &[3, b'a', b'b', b'c']);
        assert_eq!(Reader::new(w.as_slice()).octets().unwrap(), b"abc");
    }

    #[test]
    fn octets_vazio_e_um_byte_zero() {
        let mut w = Writer::new();
        w.octets(b"");
        assert_eq!(w.as_slice(), &[0]);
        assert_eq!(Reader::new(w.as_slice()).octets().unwrap(), b"");
    }

    #[test]
    fn tamanho_maior_que_o_buffer_e_recusado_antes_de_alocar() {
        // Um `CompactUINT` corrompido anunciando 0x1FFFFFFF não pode virar um pedido
        // de meio gigabyte.
        let mut w = Writer::new();
        w.compact_uint(0x1FFF_FFFF);
        let bytes = w.into_vec();
        let erro = Reader::new(&bytes).octets().unwrap_err();
        assert!(matches!(erro, WireError::LengthTooLarge { .. }), "{erro:?}");
    }

    #[test]
    fn contagem_de_contêiner_maior_que_o_buffer_e_recusada() {
        let mut w = Writer::new();
        w.seq_len(10_000);
        let bytes = w.into_vec();
        let erro = Reader::new(&bytes).seq_len().unwrap_err();
        assert!(matches!(erro, WireError::LengthTooLarge { .. }), "{erro:?}");
    }

    #[test]
    fn underflow_diz_onde_e_quanto_faltou() {
        let bytes = [0x01u8, 0x02];
        let erro = Reader::new(&bytes).u32().unwrap_err();
        assert_eq!(
            erro,
            WireError::Underflow {
                offset: 0,
                needed: 4,
                available: 2
            }
        );
    }

    #[test]
    fn prefixo_invalido_de_compact_uint_e_recusado() {
        // 0xE1..0xFF não corresponde a nenhuma das quatro formas.
        for primeiro in [0xE1u8, 0xF0, 0xFF] {
            let bytes = [primeiro, 0, 0, 0, 0];
            let erro = Reader::new(&bytes).compact_uint().unwrap_err();
            assert_eq!(erro, WireError::InvalidCompactUint(primeiro));
        }
    }

    #[test]
    fn pair_e_so_os_dois_elementos_sem_prefixo() {
        // `std::pair` não leva contagem: dois elementos em sequência e nada mais. Se
        // alguém acrescentar um prefixo aqui, todo `map` sai deslocado.
        let mut w = Writer::new();
        w.i32(7);
        w.octets(b"x");
        assert_eq!(w.len(), 4 + 1 + 1);
    }
}
