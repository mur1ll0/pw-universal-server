//! Endereço do painel web (conta do jogador, criar conta, administração) anunciado pelo servidor de
//! atualização (B391).
//!
//! O painel (`pw-admin-api`) roda no mesmo Docker que o `pw-recursos`. Quem sabe em que porta e por
//! que endereço ele é alcançado é a configuração do servidor (`PW_PAINEL_URL` no `pw-recursos`), não
//! o launcher. O servidor publica `GET /painel.json` (+ `.sig`, Ed25519 com a mesma chave dos
//! manifestos) com um **modelo** de URL; `{host}` no modelo é o host pelo qual o launcher alcança o
//! servidor de atualização (o mesmo que vai para a `serverlist.txt`, B280). Assim o mesmo servidor
//! serve quem testa em `127.0.0.1`, na rede local por IP e na internet por DNS. Atrás de proxy/HTTPS
//! o modelo é uma URL fixa (`https://conta.exemplo.com/`) sem `{host}`.
//!
//! A assinatura existe porque o launcher abre essa URL no navegador para o jogador digitar senha:
//! um intermediário na rede (HTTP sem TLS) não pode trocá-la por uma página falsa.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Marcador do modelo trocado pelo host do servidor de atualização.
pub const MARCADOR_HOST: &str = "{host}";

/// `painel.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Painel {
    pub formato: u32,
    /// Modelo da URL: `http://{host}:8000/` ou uma URL fixa.
    pub url: String,
}

/// Confere o modelo (no servidor, ao subir, e no launcher, depois da assinatura): só `http`/`https`,
/// só ASCII visível (nada de espaço, aspas ou controle: vai para o `ShellExecuteW`), e `{host}`, se
/// houver, só como o host inteiro.
pub fn validar_modelo(modelo: &str) -> Result<()> {
    let Some(resto) = modelo.strip_prefix("http://").or_else(|| modelo.strip_prefix("https://")) else {
        bail!("a URL do painel precisa começar com http:// ou https://");
    };
    if modelo.len() > 512 || !modelo.bytes().all(|b| (0x21..=0x7e).contains(&b) && !b"\"<>\\^`|".contains(&b)) {
        bail!("a URL do painel tem caractere inválido");
    }
    let autoridade = resto.split(['/', '?', '#']).next().unwrap_or("");
    if autoridade.is_empty() || autoridade.contains('@') {
        bail!("a URL do painel não tem host");
    }
    let sem_marcador = modelo.replacen(MARCADOR_HOST, "", 1);
    if sem_marcador.contains(['{', '}']) {
        bail!("a URL do painel só aceita o marcador {MARCADOR_HOST}, uma vez");
    }
    if modelo.contains(MARCADOR_HOST) {
        let host = autoridade.rsplit_once(':').filter(|(_, p)| p.bytes().all(|b| b.is_ascii_digit())).map_or(autoridade, |(h, _)| h);
        if host != MARCADOR_HOST {
            bail!("{MARCADOR_HOST} tem de ser o host inteiro da URL do painel");
        }
    }
    Ok(())
}

/// A URL que o launcher abre: o modelo com `{host}` trocado pelo host do servidor de atualização.
/// IPv6 vai entre colchetes. Host com caractere fora de nome/IP é recusado.
pub fn montar_url(modelo: &str, host: &str) -> Result<String> {
    validar_modelo(modelo)?;
    if !modelo.contains(MARCADOR_HOST) {
        return Ok(modelo.to_string());
    }
    let host = host.trim();
    if host.is_empty() || !host.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._:".contains(&b)) {
        bail!("host do servidor de atualização inválido para o painel: {host:?}");
    }
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    Ok(modelo.replacen(MARCADOR_HOST, &host, 1))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn host_do_servidor_entra_no_modelo() {
        let m = "http://{host}:8000/";
        assert_eq!(montar_url(m, "127.0.0.1").unwrap(), "http://127.0.0.1:8000/");
        assert_eq!(montar_url(m, "192.168.1.13").unwrap(), "http://192.168.1.13:8000/");
        assert_eq!(montar_url(m, "pw.exemplo.com.br").unwrap(), "http://pw.exemplo.com.br:8000/");
        assert_eq!(montar_url(m, "::1").unwrap(), "http://[::1]:8000/");
        assert_eq!(montar_url("https://{host}/conta", "pw.exemplo.com").unwrap(), "https://pw.exemplo.com/conta");
    }

    #[test]
    fn url_fixa_ignora_o_host() {
        assert_eq!(montar_url("https://conta.exemplo.com/", "10.0.0.1").unwrap(), "https://conta.exemplo.com/");
    }

    #[test]
    fn modelo_ou_host_estranho_e_recusado() {
        for m in [
            "file:///c:/windows/system32/calc.exe",
            "cmd.exe",
            "http://{host}:8000/ x",
            "http://{host}.mal.com/",
            "http://user@{host}/",
            "http://{host}/{host}",
            "http://{outro}/",
            "http:///x",
            "javascript:alert(1)",
        ] {
            assert!(validar_modelo(m).is_err(), "{m}");
        }
        assert!(montar_url("http://{host}:8000/", "a b").is_err());
        assert!(montar_url("http://{host}:8000/", "x/y").is_err());
        assert!(montar_url("http://{host}:8000/", "").is_err());
    }
}
