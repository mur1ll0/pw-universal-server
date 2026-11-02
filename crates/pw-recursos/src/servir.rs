//! Serviço HTTP do atualizador (decisão B233: porta 8100, só leitura). Serve
//! `GET /atualizacao/<versao>/<resto>` a partir de `<dados>/<versao>/atualizacao/<resto>`, com
//! `Range` (download retomável, §8.6) pelo `ServeFile` do tower-http. Importar e publicar são
//! de linha de comando — não ficam na rede.

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;
use tower_http::services::ServeFile;

fn seguro(versao: &str, resto: &str) -> bool {
    crate::importar::validar_versao(versao).is_ok()
        && !resto.is_empty()
        && resto.split('/').all(|p| {
            !p.is_empty() && p != "." && p != ".." && !p.contains(['\\', ':']) && !p.starts_with('.')
        })
}

async fn arquivo(
    State(dados): State<Arc<PathBuf>>,
    Path((versao, resto)): Path<(String, String)>,
    req: Request,
) -> Response {
    if !seguro(&versao, &resto) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // `objetos/<sha256>`: um arquivo do cliente pelo conteúdo (zstd), para o launcher reparar e
    // atualizar sem baixar o pacote inteiro (B234). Mora fora de `atualizacao/` no disco.
    let caminho = match resto.strip_prefix("objetos/") {
        Some(sha) if sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) => {
            dados.join(&versao).join("objetos").join(&sha[..2]).join(format!("{sha}.zst"))
        }
        Some(_) => return StatusCode::NOT_FOUND.into_response(),
        None => dados.join(&versao).join("atualizacao").join(&resto),
    };
    match ServeFile::new(caminho).oneshot(req).await {
        Ok(r) => r.map(Body::new).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Versões de cliente com a versão 0 pronta (`<dados>/<versao>/atualizacao/versao.json`).
async fn versoes(State(dados): State<Arc<PathBuf>>) -> Response {
    let mut lista: Vec<String> = match std::fs::read_dir(dados.as_ref()) {
        Ok(d) => d
            .filter_map(|e| e.ok())
            .filter(|e| e.path().join("atualizacao").join("versao.json").exists())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|v| crate::importar::validar_versao(v).is_ok())
            .collect(),
        Err(_) => vec![],
    };
    lista.sort();
    axum::Json(lista).into_response()
}

pub fn rotas(dados: PathBuf) -> Router {
    rotas_com_painel(dados, None)
}

/// `painel.json` assinado (B391): o modelo da URL do painel web que o launcher abre no botão
/// "Criar conta / Painel". Montado uma vez ao subir, de `PW_PAINEL_URL`.
pub struct PainelAssinado {
    pub json: Vec<u8>,
    pub assinatura_hex: String,
}

pub fn painel_assinado(modelo: &str, chave: &crate::assinatura::Chave) -> anyhow::Result<PainelAssinado> {
    let modelo = modelo.trim();
    pw_atualizador::painel::validar_modelo(modelo)?;
    let json = serde_json::to_vec(&pw_atualizador::painel::Painel { formato: 1, url: modelo.to_string() })?;
    let assinatura_hex = hex::encode(chave.assinar(&json));
    Ok(PainelAssinado { json, assinatura_hex })
}

/// As rotas públicas; com `painel`, também `GET /painel.json` e `GET /painel.json.sig`.
pub fn rotas_com_painel(dados: PathBuf, painel: Option<PainelAssinado>) -> Router {
    let mut r = Router::new()
        .route("/saude", get(|| async { "ok" }))
        .route("/versoes", get(versoes))
        .route("/atualizacao/{versao}/{*resto}", get(arquivo))
        .with_state(Arc::new(dados));
    if let Some(p) = painel {
        use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
        let p = Arc::new(p);
        let (pj, ps) = (p.clone(), p);
        r = r
            .route(
                "/painel.json",
                get(move || {
                    let p = pj.clone();
                    async move { ([(CONTENT_TYPE, "application/json"), (CACHE_CONTROL, "no-cache")], p.json.clone()) }
                }),
            )
            .route(
                "/painel.json.sig",
                get(move || {
                    let p = ps.clone();
                    async move { ([(CONTENT_TYPE, "text/plain"), (CACHE_CONTROL, "no-cache")], p.assinatura_hex.clone()) }
                }),
            );
    }
    r
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn caminho_fora_da_pasta_e_recusado() {
        assert!(seguro("1.5.5", "versao.json"));
        assert!(seguro("1.5.5", "0/pacotes/models.tar.zst"));
        assert!(!seguro("1.5.5", "../segredo"));
        assert!(!seguro("..", "versao.json"));
        assert!(!seguro("1.5.5", "0/../../x"));
        assert!(!seguro("1.5.5", ".oculto"));
    }
}
