//! Conferência da assinatura Ed25519 do `versao.json` e do `manifesto.json` (§7.2). A chave
//! pública vai embutida no launcher; quem assina é o `pw-recursos` (`pw_recursos::assinatura`).

use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Confere `assinatura` (64 bytes) de `dados` com a chave pública em hex.
pub fn verificar(publica_hex: &str, dados: &[u8], assinatura: &[u8]) -> Result<()> {
    let publica: [u8; 32] = hex::decode(publica_hex.trim())
        .ok()
        .and_then(|v| v.try_into().ok())
        .context("chave pública não tem 32 bytes em hex")?;
    let publica = VerifyingKey::from_bytes(&publica).context("chave pública inválida")?;
    let assinatura: [u8; 64] = assinatura.try_into().ok().context("assinatura não tem 64 bytes")?;
    if publica.verify(dados, &Signature::from_bytes(&assinatura)).is_err() {
        bail!("assinatura não confere");
    }
    Ok(())
}
