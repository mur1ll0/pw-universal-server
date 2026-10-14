//! API administrativa do pw-recursos (P3, B235): só para o painel (`pw-admin-api`), na porta
//! **8101**, **sem `ports:` no compose** (só a rede interna do Docker a alcança) e com o segredo
//! `PW_RECURSOS_SEGREDO` no cabeçalho `x-recursos-segredo`. O GM que pediu vai em `autor` (o
//! painel já autenticou a sessão e confere o CSRF).
//!
//! ```text
//! GET    /admin/versoes                                    [{versao, atual, minima}]
//! GET    /admin/{v}/alteracoes?limite=                     histórico (com itens)
//! GET    /admin/{v}/arquivos?busca=&limite=                arquivos da versão publicada
//! GET    /admin/{v}/arquivo?caminho=                       conteúdo atual (download)
//! POST   /admin/{v}/rascunhos      {autor, descricao, operacao_id}
//! PUT    /admin/{v}/rascunhos/{id}/arquivo?caminho=&destino=   corpo = conteúdo novo
//! DELETE /admin/{v}/rascunhos/{id}/arquivo?caminho=        tira do rascunho
//! POST   /admin/{v}/rascunhos/{id}/remover?caminho=        marca remoção
//! POST   /admin/{v}/rascunhos/{id}/descartar
//! POST   /admin/{v}/publicar       {rascunhos, autor, minima}
//! POST   /admin/{v}/alteracoes/{id}/reverter {autor, operacao_id}
//! ```

use crate::governanca::Governanca;
use crate::nomes;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use pw_storage::StorageError;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Maior arquivo aceito num rascunho (os `.data` do cliente ficam em dezenas de MB).
pub const MAIOR_ARQUIVO: usize = 512 << 20;

pub struct Erro(StatusCode, String);

impl IntoResponse for Erro {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "detail": self.1 }))).into_response()
    }
}

impl From<anyhow::Error> for Erro {
    fn from(e: anyhow::Error) -> Self {
        let status = match e.downcast_ref::<StorageError>() {
            Some(StorageError::Duplicate(_)) => StatusCode::CONFLICT,
            Some(StorageError::NotFound(_)) => StatusCode::NOT_FOUND,
            Some(StorageError::InvalidData(_)) => StatusCode::BAD_REQUEST,
            Some(_) => StatusCode::INTERNAL_SERVER_ERROR,
            None => StatusCode::BAD_REQUEST,
        };
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("admin: {e:#}");
            return Erro(status, "Falha interna no pw-recursos.".into());
        }
        Erro(status, format!("{e:#}"))
    }
}

impl From<StorageError> for Erro {
    fn from(e: StorageError) -> Self {
        anyhow::Error::from(e).into()
    }
}

type R<T> = Result<T, Erro>;

struct Estado {
    gov: Governanca,
    segredo: String,
}

fn validar(versao: &str) -> R<()> {
    crate::importar::validar_versao(versao).map_err(|e| Erro(StatusCode::NOT_FOUND, e.to_string()))
}

async fn exigir_segredo(State(e): State<Arc<Estado>>, req: Request, next: Next) -> Response {
    let recebido = req.headers().get("x-recursos-segredo").and_then(|v| v.to_str().ok()).unwrap_or("");
    let ok = recebido.len() == e.segredo.len()
        && recebido.bytes().zip(e.segredo.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0;
    if !ok {
        return Erro(StatusCode::UNAUTHORIZED, "segredo inválido".into()).into_response();
    }
    next.run(req).await
}

async fn versoes(State(e): State<Arc<Estado>>) -> R<Json<Value>> {
    let mut lista = vec![];
    if let Ok(d) = std::fs::read_dir(&e.gov.dados) {
        for x in d.flatten() {
            let Some(v) = x.file_name().into_string().ok() else { continue };
            if validar(&v).is_err() || !x.path().join("atualizacao").join("versao.json").exists() {
                continue;
            }
            let (atual, minima) = e.gov.repo.versao_atual(&v).await?.unwrap_or((0, 0));
            lista.push(json!({ "versao": v, "atual": atual, "minima": minima }));
        }
    }
    lista.sort_by(|a, b| a["versao"].as_str().cmp(&b["versao"].as_str()));
    Ok(Json(json!(lista)))
}

#[derive(Deserialize)]
struct Limite {
    limite: Option<i64>,
}

fn item_json(i: &pw_storage::ItemDeRecurso) -> Value {
    json!({
        "caminho": nomes::legivel(&i.caminho),
        "destino": i.destino,
        "acao": i.acao,
        "sha256_antes": i.sha256_antes.as_ref().map(hex::encode),
        "sha256_depois": i.sha256_depois.as_ref().map(hex::encode),
        "tamanho": i.tamanho,
        "resumo": i.resumo,
    })
}

async fn alteracoes(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<Limite>) -> R<Json<Value>> {
    validar(&v)?;
    let lista = e.gov.repo.alteracoes(&v, q.limite.unwrap_or(50).clamp(1, 500)).await?;
    let mut saida = vec![];
    for a in lista {
        let itens: Vec<Value> = e.gov.repo.itens(a.id).await?.iter().map(item_json).collect();
        let mut x = serde_json::to_value(&a).map_err(anyhow::Error::from)?;
        x["lista"] = json!(itens);
        saida.push(x);
    }
    Ok(Json(json!(saida)))
}

#[derive(Deserialize)]
struct Busca {
    busca: Option<String>,
    limite: Option<usize>,
}

async fn arquivos(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<Busca>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(json!(e.gov.buscar(&v, q.busca.as_deref().unwrap_or(""), q.limite.unwrap_or(50).min(500))?)))
}

#[derive(Deserialize)]
struct QCaminho {
    caminho: String,
    destino: Option<String>,
}

async fn conteudo(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QCaminho>) -> R<Response> {
    validar(&v)?;
    let (nome, dados) = e.gov.conteudo(&v, &q.caminho)?;
    let arquivo = nome.rsplit('\\').next().unwrap_or("arquivo").replace(['"', '\r', '\n'], "_");
    let mut h = HeaderMap::new();
    h.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    let disposicao = format!("attachment; filename*=UTF-8''{}", urlencoding(&arquivo));
    h.insert(header::CONTENT_DISPOSITION, disposicao.parse().map_err(|_| Erro(StatusCode::BAD_REQUEST, "nome".into()))?);
    Ok((h, dados).into_response())
}

fn urlencoding(t: &str) -> String {
    t.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

#[derive(Deserialize)]
struct NovoRascunho {
    autor: Option<i32>,
    descricao: String,
    operacao_id: String,
}

async fn criar_rascunho(State(e): State<Arc<Estado>>, Path(v): Path<String>, Json(p): Json<NovoRascunho>) -> R<Json<Value>> {
    validar(&v)?;
    if e.gov.repo.versao_atual(&v).await?.is_none() {
        return Err(Erro(StatusCode::NOT_FOUND, format!("a versão {v} não foi importada")));
    }
    if p.operacao_id.is_empty() || p.operacao_id.len() > 64 {
        return Err(Erro(StatusCode::BAD_REQUEST, "operacao_id inválido".into()));
    }
    let id = e.gov.repo.criar_rascunho(&v, p.autor, p.descricao.trim(), &p.operacao_id, None).await?;
    Ok(Json(json!({ "id": id })))
}

async fn gravar_arquivo(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, i64)>,
    Query(q): Query<QCaminho>,
    corpo: Bytes,
) -> R<Json<Value>> {
    validar(&v)?;
    let destino = q.destino.as_deref().unwrap_or("cliente");
    let item = e.gov.gravar_arquivo(&v, id, &q.caminho, corpo.to_vec(), destino).await?;
    Ok(Json(item_json(&item)))
}

async fn tirar(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, i64)>, Query(q): Query<QCaminho>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(json!({ "tirado": e.gov.tirar(&v, id, &q.caminho).await? })))
}

async fn remover(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, i64)>, Query(q): Query<QCaminho>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(item_json(&e.gov.remover_arquivo(&v, id, &q.caminho).await?)))
}

async fn descartar(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, i64)>) -> R<Json<Value>> {
    validar(&v)?;
    let a = e.gov.repo.alteracao(id).await?.ok_or(Erro(StatusCode::NOT_FOUND, "alteração inexistente".into()))?;
    if a.linha != v {
        return Err(Erro(StatusCode::NOT_FOUND, "alteração de outra versão".into()));
    }
    Ok(Json(json!({ "descartado": e.gov.repo.descartar(id).await? })))
}

#[derive(Deserialize)]
struct PedidoPublicar {
    rascunhos: Vec<i64>,
    autor: Option<i32>,
    minima: Option<i32>,
}

async fn publicar(State(e): State<Arc<Estado>>, Path(v): Path<String>, Json(p): Json<PedidoPublicar>) -> R<Json<Value>> {
    validar(&v)?;
    let numero = e.gov.publicar(&v, &p.rascunhos, p.autor, p.minima).await?;
    Ok(Json(json!({ "numero": numero })))
}

#[derive(Deserialize)]
struct PedidoReverter {
    autor: Option<i32>,
    operacao_id: String,
}

async fn reverter(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, i64)>,
    Json(p): Json<PedidoReverter>,
) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(json!({ "rascunho": e.gov.reverter(&v, id, p.autor, &p.operacao_id).await? })))
}

pub fn rotas(gov: Governanca, segredo: String) -> Router {
    let estado = Arc::new(Estado { gov, segredo });
    Router::new()
        .route("/admin/versoes", get(versoes))
        .route("/admin/{v}/alteracoes", get(alteracoes))
        .route("/admin/{v}/arquivos", get(arquivos))
        .route("/admin/{v}/arquivo", get(conteudo))
        .route("/admin/{v}/rascunhos", post(criar_rascunho))
        .route("/admin/{v}/rascunhos/{id}/arquivo", put(gravar_arquivo).delete(tirar))
        .route("/admin/{v}/rascunhos/{id}/remover", post(remover))
        .route("/admin/{v}/rascunhos/{id}/descartar", post(descartar))
        .route("/admin/{v}/publicar", post(publicar))
        .route("/admin/{v}/alteracoes/{id}/reverter", post(reverter))
        .layer(DefaultBodyLimit::max(MAIOR_ARQUIVO))
        .route_layer(middleware::from_fn_with_state(estado.clone(), exigir_segredo))
        .with_state(estado)
}
