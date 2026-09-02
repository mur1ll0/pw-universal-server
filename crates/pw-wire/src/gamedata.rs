//! O formato dos subcomandos do `GamedataSend`: **little-endian**, `#pragma pack(1)`.
//!
//! Isto não é serialização: é a memória do processo i386 do cliente copiada crua para o
//! fio por `memcpy`. Daí as três propriedades que o separam do [`gnet`](crate::gnet):
//!
//! * **little-endian**, porque é a ordem de bytes do i386, sem conversão;
//! * **sem preenchimento**, porque `#pragma pack(1)` vale em toda a região — o
//!   deslocamento de um campo é a soma dos tamanhos dos anteriores, e nada mais;
//! * **sem prefixo de tamanho**: uma lista traz um campo `count` explícito, declarado
//!   como qualquer outro campo, e os elementos vêm em seguida.
//!
//! Como o layout é posicional, o leitor aceita ser endereçado por deslocamento
//! ([`Reader::at`]) além de sequencialmente. Os deslocamentos vêm do IR em
//! `specs/protocol/gamedata_153.json`, que foi conferido campo a campo contra o
//! compilador C++ de 32 bits.
//!
//! # Tamanhos do alvo, não os do host
//!
//! O cliente original é um binário Win32 de 32 bits. `size_t` e `long` têm **4** bytes
//! lá, não 8, e é por isso que este módulo nunca usa `usize` como tipo de campo: um
//! campo `size_t` do protocolo é [`Reader::u32`]. As constantes em [`size`] existem
//! para que esse detalhe apareça no código em vez de ficar implícito.

use crate::error::{WireError, WireResult};

/// Tamanhos dos tipos C++ no alvo original (Win32/i386, 32 bits).
///
/// Estão aqui porque dois deles não são os do host onde este código roda, e trocá-los
/// desloca todos os campos seguintes de uma struct.
pub mod size {
    /// `char`, `unsigned char`, `byte`, `BYTE`, `bool`.
    pub const BYTE: usize = 1;
    /// `short`, `unsigned short`, `WORD`.
    pub const SHORT: usize = 2;
    /// `int`, `unsigned int`, `DWORD`, `float` — e também **`long` e `size_t`**, que
    /// no alvo de 32 bits têm 4 bytes, não 8.
    pub const WORD32: usize = 4;
    /// `__int64`, `double`.
    pub const WORD64: usize = 8;
    /// `A3DVECTOR3` / `A3DVECTOR`: três `float` consecutivos, sem preenchimento.
    pub const VEC3: usize = 12;
}

/// Três `float` consecutivos — posição, destino ou direção no mundo 3D.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

/// Leitor sobre um payload de comando.
///
/// Pode ser usado em sequência ou endereçado por deslocamento com [`Reader::at`] — os
/// dois modos convivem, e `at` reposiciona o cursor.
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

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Reposiciona o cursor num deslocamento absoluto.
    ///
    /// É a forma natural de usar este formato: o IR diz que `cmd_object_move.dest`
    /// mora em 4, e o leitor vai direto para lá em vez de depender de ter lido tudo
    /// que vem antes na ordem certa.
    pub fn at(&mut self, offset: usize) -> WireResult<&mut Self> {
        if offset > self.buf.len() {
            return Err(WireError::OutOfBounds {
                offset,
                len: self.buf.len(),
            });
        }
        self.pos = offset;
        Ok(self)
    }

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

    /// O que resta do buffer, sem consumir. É por aqui que se lê a cauda de uma struct
    /// de tamanho variável, depois do campo `count`.
    pub fn rest(&self) -> &'a [u8] {
        &self.buf[self.pos..]
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

    pub fn bool(&mut self) -> WireResult<bool> {
        Ok(self.u8()? != 0)
    }

    pub fn u16(&mut self) -> WireResult<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub fn i16(&mut self) -> WireResult<i16> {
        Ok(i16::from_le_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> WireResult<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub fn i32(&mut self) -> WireResult<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> WireResult<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub fn i64(&mut self) -> WireResult<i64> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    pub fn f32(&mut self) -> WireResult<f32> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    pub fn f64(&mut self) -> WireResult<f64> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    /// Lê um `A3DVECTOR3`: três `float` seguidos, 12 bytes ao todo.
    pub fn vec3(&mut self) -> WireResult<Vec3> {
        Ok(Vec3 {
            x: self.f32()?,
            y: self.f32()?,
            z: self.f32()?,
        })
    }
}

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

/// Escritor de payload.
///
/// Escreve campo a campo, sem preenchimento nenhum entre eles — é o que reproduz o
/// `#pragma pack(1)` do cliente. [`Writer::len`] depois de escrever um campo é o
/// deslocamento do **próximo**, o que permite conferir contra o IR sem instrumentação
/// extra.
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
        self.raw(&v.to_le_bytes());
    }

    pub fn i16(&mut self, v: i16) {
        self.raw(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.raw(&v.to_le_bytes());
    }

    pub fn i32(&mut self, v: i32) {
        self.raw(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.raw(&v.to_le_bytes());
    }

    pub fn i64(&mut self, v: i64) {
        self.raw(&v.to_le_bytes());
    }

    pub fn f32(&mut self, v: f32) {
        self.raw(&v.to_le_bytes());
    }

    pub fn f64(&mut self, v: f64) {
        self.raw(&v.to_le_bytes());
    }

    pub fn vec3(&mut self, v: Vec3) {
        self.f32(v.x);
        self.f32(v.y);
        self.f32(v.z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalares_vao_em_little_endian() {
        let mut w = Writer::new();
        w.u32(0x1122_3344);
        w.i16(-2);
        assert_eq!(w.as_slice(), &[0x44, 0x33, 0x22, 0x11, 0xFE, 0xFF]);

        let mut r = Reader::new(w.as_slice());
        assert_eq!(r.u32().unwrap(), 0x1122_3344);
        assert_eq!(r.i16().unwrap(), -2);
    }

    #[test]
    fn o_gamedata_e_o_inverso_do_gnet_no_mesmo_valor() {
        // Os dois formatos convivem na mesma conexão. Este teste existe para que a
        // troca de um pelo outro apareça como falha, e não como um campo estranho.
        let mut g = crate::gnet::Writer::new();
        g.u32(1);
        let mut d = Writer::new();
        d.u32(1);
        assert_eq!(g.as_slice(), &[0, 0, 0, 1]);
        assert_eq!(d.as_slice(), &[1, 0, 0, 0]);
    }

    #[test]
    fn nao_ha_preenchimento_entre_campos() {
        // Sob alinhamento natural, `id` cairia em 4 e a struct teria 12 bytes. Sob
        // pack(1) são 7 — é essa a diferença que quebra o protocolo se for ignorada.
        let mut w = Writer::new();
        w.u8(1);
        assert_eq!(w.len(), 1, "o próximo campo começa em 1");
        w.i32(2);
        assert_eq!(w.len(), 5, "o próximo campo começa em 5");
        w.u16(3);
        assert_eq!(w.len(), 7, "a struct inteira tem 7 bytes");
    }

    #[test]
    fn size_t_e_long_tem_quatro_bytes_no_alvo() {
        // O host é de 64 bits; o cliente original não. Um campo `size_t` do protocolo
        // ocupa 4 bytes, e tratá-lo como `usize` deslocaria tudo que vem depois.
        assert_eq!(size::WORD32, 4);
        let mut w = Writer::new();
        w.u32(0xDEAD_BEEF); // um `size_t` do protocolo
        assert_eq!(w.len(), size::WORD32);
    }

    #[test]
    fn vec3_sao_doze_bytes_e_volta_igual() {
        let mut w = Writer::new();
        w.vec3(Vec3::new(1.0, -2.0, 0.5));
        assert_eq!(w.len(), size::VEC3);
        let v = Reader::new(w.as_slice()).vec3().unwrap();
        assert_eq!(v, Vec3::new(1.0, -2.0, 0.5));
    }

    #[test]
    fn leitura_endereçada_por_deslocamento() {
        // Como o IR usaria: `cmd_notify_hostpos` = A3DVECTOR3 em 0, int em 12, int em 16.
        let mut w = Writer::new();
        w.vec3(Vec3::new(1.0, 2.0, 3.0));
        w.i32(42);
        w.i32(7);
        let bytes = w.into_vec();

        let mut r = Reader::new(&bytes);
        assert_eq!(r.at(12).unwrap().i32().unwrap(), 42);
        assert_eq!(r.at(16).unwrap().i32().unwrap(), 7);
        assert_eq!(r.at(0).unwrap().vec3().unwrap(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn deslocamento_fora_do_buffer_e_recusado() {
        let bytes = [0u8; 4];
        let erro = Reader::new(&bytes).at(5).unwrap_err();
        assert_eq!(erro, WireError::OutOfBounds { offset: 5, len: 4 });
    }

    #[test]
    fn rest_expoe_a_cauda_de_uma_lista_variavel() {
        // Uma struct de lista abre com `count` e os elementos vêm em seguida, sem
        // prefixo nenhum.
        let mut w = Writer::new();
        w.u16(2);
        w.i32(10);
        w.i32(20);
        let bytes = w.into_vec();

        let mut r = Reader::new(&bytes);
        let count = r.u16().unwrap();
        assert_eq!(count, 2);
        assert_eq!(r.rest().len(), 8);
        for esperado in [10, 20] {
            assert_eq!(r.i32().unwrap(), esperado);
        }
        assert_eq!(r.remaining(), 0);
    }
}
