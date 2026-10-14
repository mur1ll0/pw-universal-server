//! Assinatura Ed25519 do `versao.json` e do `manifesto.json` (§7.2, decisão B233: a chave
//! nasce sozinha no primeiro uso, num volume que só o pw-recursos monta).
//!
//! A chave pública vai embutida no launcher; quem não tem a privada não consegue fazer o
//! launcher aceitar um manifesto — nem num servidor falso, nem em HTTP sem TLS.

use anyhow::{Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use std::fs;
use std::path::Path;

pub struct Chave {
    privada: SigningKey,
}

impl Chave {
    /// Lê a chave (32 bytes da semente em hex) ou cria uma nova se o arquivo não existe.
    /// Devolve também se foi criada agora.
    pub fn carregar_ou_criar(caminho: &Path) -> Result<(Self, bool)> {
        if caminho.exists() {
            let texto = fs::read_to_string(caminho)
                .with_context(|| format!("ler a chave {}", caminho.display()))?;
            let semente: [u8; 32] = hex::decode(texto.trim())
                .ok()
                .and_then(|v| v.try_into().ok())
                .with_context(|| format!("chave {} não tem 32 bytes em hex", caminho.display()))?;
            return Ok((Self { privada: SigningKey::from_bytes(&semente) }, false));
        }
        let mut semente = [0u8; 32];
        getrandom::getrandom(&mut semente).map_err(|e| anyhow::anyhow!("gerar a chave: {e}"))?;
        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta)?;
        }
        let mut opcoes = fs::OpenOptions::new();
        opcoes.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opcoes.mode(0o600);
        }
        use std::io::Write;
        let mut f = opcoes
            .open(caminho)
            .with_context(|| format!("criar a chave {}", caminho.display()))?;
        writeln!(f, "{}", hex::encode(semente))?;
        Ok((Self { privada: SigningKey::from_bytes(&semente) }, true))
    }

    pub fn assinar(&self, dados: &[u8]) -> [u8; 64] {
        self.privada.sign(dados).to_bytes()
    }

    pub fn publica_hex(&self) -> String {
        hex::encode(self.privada.verifying_key().to_bytes())
    }
}

/// A conferência é a do launcher (um caminho só).
pub use pw_atualizador::assinatura::verificar;

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn criar_assinar_conferir_e_recarregar() {
        let pasta = std::env::temp_dir().join(format!("pw_recursos_chave_{}", std::process::id()));
        let _ = fs::remove_dir_all(&pasta);
        let caminho = pasta.join("assinatura_ed25519.key");
        let (chave, criada) = Chave::carregar_ou_criar(&caminho).unwrap();
        assert!(criada);
        let assinatura = chave.assinar(b"manifesto");
        verificar(&chave.publica_hex(), b"manifesto", &assinatura).unwrap();
        assert!(verificar(&chave.publica_hex(), b"manifestO", &assinatura).is_err());
        let (de_novo, criada) = Chave::carregar_ou_criar(&caminho).unwrap();
        assert!(!criada);
        assert_eq!(de_novo.publica_hex(), chave.publica_hex());
        let _ = fs::remove_dir_all(&pasta);
    }
}
