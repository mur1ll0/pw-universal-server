//! `TALK_PROC` estruturado e gravável (Estúdio S3, B240): o `talk_proc::save`/`load` do cliente
//! (`EvolvedPWClient/ElementClient/CCommon/ExpTypes.h:3872`), simétricos.
//!
//! ```text
//! u32 contagem
//! conversa: u32 id_talk, u16 text[64], i32 num_window, janelas
//! janela:   u32 id, u32 id_parent (-1 = raiz), i32 talk_text_len, u16 talk_text[len], i32 num_option, opções
//! opção:    u32 id, u16 text[64], u32 param         (136 B; id com o bit alto = função, SERVICE_TYPE)
//! ```
//!
//! Os textos ficam como unidades UTF-16 cruas (inclusive o que vem depois do NUL e o NUL final do
//! texto da janela, quando há): gravar sem editar devolve os mesmos bytes.

use crate::generic_elements::{GenericElementsError, Result};

pub const TAMANHO_TEXTO_FIXO: usize = 64;
/// Bit que marca uma opção como função pré-definida (`enum SERVICE_TYPE`, `ExpTypes.h:36-102`).
pub const BIT_DE_FUNCAO: u32 = 0x8000_0000;

#[derive(Debug, Clone, PartialEq)]
pub struct Opcao {
    pub id: u32,
    pub texto: Vec<u16>,
    pub param: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Janela {
    pub id: u32,
    pub id_pai: u32,
    pub texto: Vec<u16>,
    pub opcoes: Vec<Opcao>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conversa {
    pub id: u32,
    pub texto: Vec<u16>,
    pub janelas: Vec<Janela>,
}

struct Leitor<'a> {
    b: &'a [u8],
    p: usize,
}

impl Leitor<'_> {
    fn u32(&mut self) -> Result<u32> {
        let v = self.b.get(self.p..self.p + 4).ok_or(GenericElementsError::OutOfBounds(self.p + 4, self.b.len()))?;
        self.p += 4;
        Ok(u32::from_le_bytes(v.try_into().unwrap()))
    }
    fn contagem(&mut self) -> Result<usize> {
        let n = self.u32()? as i32;
        if n < 0 || n as usize > self.b.len() {
            return Err(GenericElementsError::OutOfBounds(self.p, self.b.len()));
        }
        Ok(n as usize)
    }
    fn u16s(&mut self, n: usize) -> Result<Vec<u16>> {
        let fim = self.p + n * 2;
        let v = self.b.get(self.p..fim).ok_or(GenericElementsError::OutOfBounds(fim, self.b.len()))?;
        self.p = fim;
        Ok(v.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
    }
}

/// Lê o pedaço do `TALK_PROC` (começa na contagem). Devolve as conversas e onde terminou.
pub fn ler(buf: &[u8], inicio: usize) -> Result<(Vec<Conversa>, usize)> {
    let mut l = Leitor { b: buf, p: inicio };
    let n = l.contagem()?;
    let mut conversas = Vec::with_capacity(n.min(65_536));
    for _ in 0..n {
        let id = l.u32()?;
        let texto = l.u16s(TAMANHO_TEXTO_FIXO)?;
        let nj = l.contagem()?;
        let mut janelas = Vec::with_capacity(nj.min(1024));
        for _ in 0..nj {
            let id = l.u32()?;
            let id_pai = l.u32()?;
            let len = l.contagem()?;
            let texto = l.u16s(len)?;
            let no = l.contagem()?;
            let mut opcoes = Vec::with_capacity(no.min(1024));
            for _ in 0..no {
                let id = l.u32()?;
                let texto = l.u16s(TAMANHO_TEXTO_FIXO)?;
                let param = l.u32()?;
                opcoes.push(Opcao { id, texto, param });
            }
            janelas.push(Janela { id, id_pai, texto, opcoes });
        }
        conversas.push(Conversa { id, texto, janelas });
    }
    Ok((conversas, l.p))
}

pub fn gravar(conversas: &[Conversa]) -> Vec<u8> {
    let mut s = Vec::new();
    let u16s = |s: &mut Vec<u8>, v: &[u16]| v.iter().for_each(|u| s.extend_from_slice(&u.to_le_bytes()));
    s.extend_from_slice(&(conversas.len() as u32).to_le_bytes());
    for c in conversas {
        s.extend_from_slice(&c.id.to_le_bytes());
        u16s(&mut s, &c.texto);
        s.extend_from_slice(&(c.janelas.len() as u32).to_le_bytes());
        for j in &c.janelas {
            s.extend_from_slice(&j.id.to_le_bytes());
            s.extend_from_slice(&j.id_pai.to_le_bytes());
            s.extend_from_slice(&(j.texto.len() as u32).to_le_bytes());
            u16s(&mut s, &j.texto);
            s.extend_from_slice(&(j.opcoes.len() as u32).to_le_bytes());
            for o in &j.opcoes {
                s.extend_from_slice(&o.id.to_le_bytes());
                u16s(&mut s, &o.texto);
                s.extend_from_slice(&o.param.to_le_bytes());
            }
        }
    }
    s
}

/// Texto até o primeiro NUL.
pub fn texto(u: &[u16]) -> String {
    String::from_utf16_lossy(&u[..u.iter().position(|&c| c == 0).unwrap_or(u.len())])
}

/// Texto de tamanho fixo (64 unidades): até 63 caracteres + NUL, completado com zeros
/// ("最多63个汉字", `ExpTypes.h:3874`).
pub fn texto_fixo(s: &str) -> std::result::Result<Vec<u16>, String> {
    let mut u: Vec<u16> = s.encode_utf16().collect();
    if u.len() > TAMANHO_TEXTO_FIXO - 1 || u.contains(&0) {
        return Err(format!("cabe até {} caracteres", TAMANHO_TEXTO_FIXO - 1));
    }
    u.resize(TAMANHO_TEXTO_FIXO, 0);
    Ok(u)
}

/// Texto de janela (tamanho variável): o texto com o NUL final — `talk_text_len` conta o NUL.
pub fn texto_variavel(s: &str) -> std::result::Result<Vec<u16>, String> {
    let mut u: Vec<u16> = s.encode_utf16().collect();
    if u.contains(&0) || u.len() > 4096 {
        return Err("texto inválido (NUL no meio ou mais de 4096 caracteres)".into());
    }
    u.push(0);
    Ok(u)
}
