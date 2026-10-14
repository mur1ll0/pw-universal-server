//! Objetos por conteúdo: `<dados>/<versao>/objetos/<sha256[0..2]>/<sha256>.zst`, imutáveis
//! (§3 do `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`). O mesmo conteúdo é gravado uma vez só.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub type Sha256Bytes = [u8; 32];

pub fn sha256(dados: &[u8]) -> Sha256Bytes {
    Sha256::digest(dados).into()
}

pub fn md5(dados: &[u8]) -> [u8; 16] {
    use md5::Md5;
    Md5::digest(dados).into()
}

#[derive(Clone)]
pub struct Objetos {
    raiz: PathBuf,
    nivel: i32,
}

impl Objetos {
    pub fn new(raiz: impl Into<PathBuf>, nivel: i32) -> Self {
        Self { raiz: raiz.into(), nivel }
    }

    pub fn caminho(&self, sha: &Sha256Bytes) -> PathBuf {
        let h = hex::encode(sha);
        self.raiz.join(&h[..2]).join(format!("{h}.zst"))
    }

    /// Grava se ainda não existe. Escreve num temporário e renomeia: um objeto interrompido
    /// nunca fica com o nome definitivo.
    pub fn gravar(&self, sha: &Sha256Bytes, dados: &[u8]) -> io::Result<()> {
        let destino = self.caminho(sha);
        if destino.exists() {
            return Ok(());
        }
        fs::create_dir_all(destino.parent().unwrap())?;
        let comprimido = zstd::bulk::compress(dados, self.nivel)?;
        static CONTADOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = CONTADOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temporario = destino.with_extension(format!("tmp{}_{n}", std::process::id()));
        fs::write(&temporario, comprimido)?;
        match fs::rename(&temporario, &destino) {
            // Outro fio gravou o mesmo conteúdo antes: o objeto é imutável, vale o que está lá.
            Err(_) if destino.exists() => fs::remove_file(&temporario),
            r => r,
        }
    }

    pub fn ler(&self, sha: &Sha256Bytes, tamanho: usize) -> io::Result<Vec<u8>> {
        let comprimido = fs::read(self.caminho(sha))?;
        let dados = zstd::bulk::decompress(&comprimido, tamanho.max(1))?;
        if dados.len() != tamanho || sha256(&dados) != *sha {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("objeto {} corrompido", hex::encode(sha)),
            ));
        }
        Ok(dados)
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }
}
