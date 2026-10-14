//! Tar no formato GNU com o nome em **bytes crus** (GBK), escrito e lido aqui.
//!
//! Por que não o cabeçalho PAX: o PAX guarda o nome em UTF-8, e o nome certo depende da página
//! de código de quem extrai (§7.7/§8.6 do `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`). O GNU guarda
//! o nome como bytes: até 100 no próprio cabeçalho, mais que isso numa entrada
//! `././@LongLink` (tipo `L`) logo antes. O separador fica o do cliente (`\`), sem conversão:
//! um segundo byte GBK pode ser 0x5C, e trocar `\` por `/` o estragaria.

use std::io::{self, Read, Write};

const BLOCO: usize = 512;

fn octal(campo: &mut [u8], valor: u64) {
    // `largura-1` dígitos octais e um NUL, como o GNU tar.
    let largura = campo.len() - 1;
    let texto = format!("{:0largura$o}", valor, largura = largura);
    assert!(texto.len() == largura, "valor grande demais para o campo octal do tar");
    campo[..largura].copy_from_slice(texto.as_bytes());
    campo[largura] = 0;
}

fn cabecalho(nome: &[u8], tamanho: u64, tipo: u8) -> [u8; BLOCO] {
    let mut h = [0u8; BLOCO];
    let n = nome.len().min(100);
    h[..n].copy_from_slice(&nome[..n]);
    octal(&mut h[100..108], 0o644);
    octal(&mut h[108..116], 0);
    octal(&mut h[116..124], 0);
    octal(&mut h[124..136], tamanho); // 11 dígitos: até 8 GiB por arquivo
    octal(&mut h[136..148], 0); // mtime 0: pacote reprodutível
    h[156] = tipo;
    h[257..263].copy_from_slice(b"ustar ");
    h[263..265].copy_from_slice(b" \0");
    h[148..156].copy_from_slice(b"        ");
    let soma: u32 = h.iter().map(|&b| b as u32).sum();
    let texto = format!("{:06o}\0 ", soma);
    h[148..156].copy_from_slice(texto.as_bytes());
    h
}

fn preencher<W: Write>(w: &mut W, tamanho: u64) -> io::Result<()> {
    let resto = (tamanho as usize) % BLOCO;
    if resto != 0 {
        w.write_all(&[0u8; BLOCO][..BLOCO - resto])?;
    }
    Ok(())
}

pub struct Escritor<W: Write> {
    w: W,
}

impl<W: Write> Escritor<W> {
    pub fn new(w: W) -> Self {
        Self { w }
    }

    pub fn arquivo(&mut self, nome: &[u8], dados: &[u8]) -> io::Result<()> {
        if nome.is_empty() || nome.contains(&0) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "nome vazio ou com NUL"));
        }
        if nome.len() > 100 {
            let mut longo = nome.to_vec();
            longo.push(0);
            self.w.write_all(&cabecalho(b"././@LongLink", longo.len() as u64, b'L'))?;
            self.w.write_all(&longo)?;
            preencher(&mut self.w, longo.len() as u64)?;
        }
        self.w.write_all(&cabecalho(nome, dados.len() as u64, b'0'))?;
        self.w.write_all(dados)?;
        preencher(&mut self.w, dados.len() as u64)
    }

    /// Dois blocos zerados de fim e devolve o escritor de baixo.
    pub fn terminar(mut self) -> io::Result<W> {
        self.w.write_all(&[0u8; BLOCO * 2])?;
        Ok(self.w)
    }
}

pub struct Leitor<R: Read> {
    r: R,
}

fn ler_octal(campo: &[u8]) -> io::Result<u64> {
    let texto: Vec<u8> = campo.iter().copied().take_while(|&b| b != 0 && b != b' ').collect();
    let texto = std::str::from_utf8(&texto).map_err(|_| invalido("campo octal"))?;
    let texto = texto.trim_start_matches(' ');
    if texto.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(texto, 8).map_err(|_| invalido("campo octal"))
}

fn invalido(o_que: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("tar inválido: {o_que}"))
}

impl<R: Read> Leitor<R> {
    pub fn new(r: R) -> Self {
        Self { r }
    }

    fn ler_dados(&mut self, tamanho: u64) -> io::Result<Vec<u8>> {
        let mut dados = vec![0u8; tamanho as usize];
        self.r.read_exact(&mut dados)?;
        let resto = (tamanho as usize) % BLOCO;
        if resto != 0 {
            let mut lixo = [0u8; BLOCO];
            self.r.read_exact(&mut lixo[..BLOCO - resto])?;
        }
        Ok(dados)
    }

    /// Próximo arquivo `(nome em bytes, dados)`, ou `None` no fim do tar.
    pub fn proximo(&mut self) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        let mut nome_longo: Option<Vec<u8>> = None;
        loop {
            let mut h = [0u8; BLOCO];
            self.r.read_exact(&mut h)?;
            if h.iter().all(|&b| b == 0) {
                return Ok(None);
            }
            let gravada = ler_octal(&h[148..156])?;
            let mut copia = h;
            copia[148..156].copy_from_slice(b"        ");
            let soma: u64 = copia.iter().map(|&b| b as u64).sum();
            if soma != gravada {
                return Err(invalido("soma do cabeçalho"));
            }
            let tamanho = ler_octal(&h[124..136])?;
            match h[156] {
                b'L' => {
                    let mut n = self.ler_dados(tamanho)?;
                    while n.last() == Some(&0) {
                        n.pop();
                    }
                    nome_longo = Some(n);
                }
                b'0' | 0 => {
                    let nome = match nome_longo.take() {
                        Some(n) => n,
                        None => h[..100].iter().copied().take_while(|&b| b != 0).collect(),
                    };
                    let dados = self.ler_dados(tamanho)?;
                    return Ok(Some((nome, dados)));
                }
                _ => {
                    // Pastas e outros tipos: o pw-recursos não gera; pula o conteúdo.
                    self.ler_dados(tamanho)?;
                    nome_longo = None;
                }
            }
        }
    }

    pub fn into_inner(self) -> R {
        self.r
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ida_e_volta_com_nome_longo_e_segundo_byte_0x5c() {
        // 0x95 0x5C é um caractere GBK cujo 2º byte é `\`: tem de voltar intacto.
        let curto = b"configs\\\x95\x5c.txt".to_vec();
        let mut longo = b"models\\players\\".to_vec();
        longo.extend(std::iter::repeat(0xD0).take(150));
        let mut esc = Escritor::new(Vec::new());
        esc.arquivo(&curto, b"abc").unwrap();
        esc.arquivo(&longo, &vec![7u8; 1000]).unwrap();
        esc.arquivo(b"vazio.txt", b"").unwrap();
        let tar = esc.terminar().unwrap();
        assert_eq!(tar.len() % BLOCO, 0);
        let mut l = Leitor::new(&tar[..]);
        assert_eq!(l.proximo().unwrap().unwrap(), (curto, b"abc".to_vec()));
        assert_eq!(l.proximo().unwrap().unwrap(), (longo, vec![7u8; 1000]));
        assert_eq!(l.proximo().unwrap().unwrap(), (b"vazio.txt".to_vec(), vec![]));
        assert!(l.proximo().unwrap().is_none());
    }

    #[test]
    fn cabecalho_corrompido_e_recusado() {
        let mut esc = Escritor::new(Vec::new());
        esc.arquivo(b"a.txt", b"x").unwrap();
        let mut tar = esc.terminar().unwrap();
        tar[0] = b'b';
        assert!(Leitor::new(&tar[..]).proximo().is_err());
    }
}
