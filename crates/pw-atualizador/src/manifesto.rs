//! `versao.json` e `manifesto.json` (§4 e §8.6 do `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`).
//!
//! Disco (volume `data/clientes`) e HTTP (`/atualizacao/<versao>/…`) têm a mesma árvore:
//!
//! ```text
//! <versao>/atualizacao/versao.json (+ .sig)        aponta a versão atual e a mínima
//! <versao>/atualizacao/<n>/manifesto.json (+ .sig) estado completo da versão n
//! <versao>/atualizacao/<n>/pacotes/<pasta>.tar.zst instalação: um pacote por pasta
//! ```
//!
//! Os `.sig` são a assinatura Ed25519 (64 bytes, em hex) dos bytes exatos do `.json`.

use serde::{Deserialize, Serialize};

pub const FORMATO: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versao {
    pub formato: u32,
    pub versao_cliente: String,
    pub atual: u32,
    pub minima: u32,
    /// Caminho relativo a `atualizacao/<versao>/`.
    pub manifesto: String,
    pub manifesto_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifesto {
    pub formato: u32,
    pub versao_cliente: String,
    pub numero: u32,
    pub gerado_em: String,
    pub pacotes: Vec<Pacote>,
    pub arquivos: Vec<Arquivo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pacote {
    pub nome: String,
    /// Relativo a `atualizacao/<versao>/<numero>/`.
    pub arquivo: String,
    pub sha256: String,
    pub tamanho: u64,
    pub tamanho_tar: u64,
    pub arquivos: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arquivo {
    /// Bytes GBK do caminho relativo a `element\`, separador `\`, em hex. O launcher calcula o
    /// nome no disco com `MultiByteToWideChar(CP_ACP)` (§7.7).
    pub caminho_gbk: String,
    pub sha256: String,
    pub tamanho: u64,
    pub pacote: String,
}
