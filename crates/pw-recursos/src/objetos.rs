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

    /// Apaga todo objeto fora de `em_uso` e os temporários que uma gravação interrompida
    /// deixou. Devolve `(arquivos, bytes)` apagados. Só arquivos `<aa>/<64 hex>.zst` e
    /// `<aa>/*.tmp*` são considerados: nada mais da pasta é tocado.
    pub fn limpar(&self, em_uso: &std::collections::HashSet<Sha256Bytes>) -> io::Result<(u64, u64)> {
        let (mut n, mut bytes) = (0u64, 0u64);
        if !self.raiz.exists() {
            return Ok((0, 0));
        }
        for sub in fs::read_dir(&self.raiz)? {
            let sub = sub?;
            if !sub.file_type()?.is_dir() {
                continue;
            }
            for e in fs::read_dir(sub.path())? {
                let e = e?;
                let nome = e.file_name().to_string_lossy().into_owned();
                let lixo = if let Some(h) = nome.strip_suffix(".zst") {
                    match hex::decode(h).ok().and_then(|v| <Sha256Bytes>::try_from(v).ok()) {
                        Some(sha) => !em_uso.contains(&sha),
                        None => false,
                    }
                } else {
                    nome.contains(".tmp")
                };
                if lixo && e.file_type()?.is_file() {
                    bytes += e.metadata()?.len();
                    fs::remove_file(e.path())?;
                    n += 1;
                }
            }
            // Pasta de prefixo vazia: remove (falha em silêncio se ainda tem algo).
            let _ = fs::remove_dir(sub.path());
        }
        Ok((n, bytes))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn limpar_apaga_so_o_que_nao_esta_em_uso() {
        let raiz = std::env::temp_dir().join(format!("pw_recursos_objetos_{}", std::process::id()));
        let _ = fs::remove_dir_all(&raiz);
        let o = Objetos::new(&raiz, 3);
        let (a, b) = (b"fica".to_vec(), b"sai".to_vec());
        let (sa, sb) = (sha256(&a), sha256(&b));
        o.gravar(&sa, &a).unwrap();
        o.gravar(&sb, &b).unwrap();
        fs::write(o.caminho(&sa).with_extension("tmp1_0"), b"x").unwrap();
        let em_uso = [sa].into_iter().collect();
        let (n, _) = o.limpar(&em_uso).unwrap();
        assert_eq!(n, 2); // o objeto "sai" e o temporário
        assert_eq!(o.ler(&sa, a.len()).unwrap(), a);
        assert!(!o.caminho(&sb).exists());
        let _ = fs::remove_dir_all(&raiz);
    }
}
