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

use crate::estudio::TipoDeArea;
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
    estudio: crate::estudio::Estudio,
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
    let r = e.gov.publicar(&v, &p.rascunhos, p.autor, p.minima).await?;
    Ok(Json(json!({ "numero": r.numero, "aplicados": r.aplicados })))
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

// ------------------------------------------------------------------ Estúdio (S1, B237)

#[derive(Deserialize)]
struct QEstudio {
    rascunho: Option<i64>,
    busca: Option<String>,
    pagina: Option<usize>,
    por_pagina: Option<usize>,
}

fn tabela_valida(t: &str) -> R<()> {
    if t.is_empty() || t.len() > 64 || !t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
        return Err(Erro(StatusCode::NOT_FOUND, "tabela inválida".into()));
    }
    Ok(())
}

fn rascunho_obrigatorio(q: &QEstudio) -> R<i64> {
    q.rascunho.ok_or(Erro(StatusCode::BAD_REQUEST, "escolha o rascunho onde a edição fica".into()))
}

async fn estudio_tabelas(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.tabelas(&e.gov, &v, q.rascunho).await?))
}

async fn estudio_listar(
    State(e): State<Arc<Estado>>,
    Path((v, tabela)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    tabela_valida(&tabela)?;
    let r = e
        .estudio
        .listar(&e.gov, &v, q.rascunho, &tabela, q.busca.as_deref().unwrap_or(""), q.pagina.unwrap_or(0), q.por_pagina.unwrap_or(50))
        .await?;
    Ok(Json(r))
}

async fn estudio_ficha(
    State(e): State<Arc<Estado>>,
    Path((v, tabela, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    tabela_valida(&tabela)?;
    Ok(Json(e.estudio.ficha(&e.gov, &v, q.rascunho, &tabela, indice).await?))
}

#[derive(Deserialize)]
struct PedidoEditar {
    campos: serde_json::Map<String, Value>,
}

async fn estudio_editar(
    State(e): State<Arc<Estado>>,
    Path((v, tabela, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoEditar>,
) -> R<Json<Value>> {
    validar(&v)?;
    tabela_valida(&tabela)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar(&e.gov, &v, r, &tabela, indice, p.campos).await?))
}

async fn estudio_clonar(
    State(e): State<Arc<Estado>>,
    Path((v, tabela, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    tabela_valida(&tabela)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.clonar(&e.gov, &v, r, &tabela, indice).await?))
}

async fn estudio_remover(
    State(e): State<Arc<Estado>>,
    Path((v, tabela, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    tabela_valida(&tabela)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover(&e.gov, &v, r, &tabela, indice).await?))
}

#[derive(Deserialize)]
struct QBusca {
    rascunho: Option<i64>,
    busca: Option<String>,
    limite: Option<usize>,
}

async fn estudio_buscar_no_espaco(
    State(e): State<Arc<Estado>>,
    Path((v, espaco)): Path<(String, String)>,
    Query(q): Query<QBusca>,
) -> R<Json<Value>> {
    validar(&v)?;
    if !matches!(espaco.as_str(), "ESSENCE" | "CONFIG" | "FACE" | "RECIPE" | "ADDON" | "TALK") {
        return Err(Erro(StatusCode::NOT_FOUND, "espaço inválido".into()));
    }
    let r = e
        .estudio
        .buscar_no_espaco(&e.gov, &v, q.rascunho, &espaco, q.busca.as_deref().unwrap_or(""), q.limite.unwrap_or(30))
        .await?;
    Ok(Json(r))
}

// ------------------------------------------------------------------ mapas (S4, B242)

async fn estudio_mapas(State(e): State<Arc<Estado>>, Path(v): Path<String>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.mapas(&e.gov, &v).await?))
}

async fn estudio_peca(State(e): State<Arc<Estado>>, Path((v, caminho, peca)): Path<(String, String, String)>) -> R<Response> {
    validar(&v)?;
    let llcc = peca.strip_suffix(".png").unwrap_or(&peca);
    let png = e.estudio.peca(&e.gov, &v, &caminho, llcc).await.map_err(|err| Erro(StatusCode::NOT_FOUND, format!("{err:#}")))?;
    let mut h = HeaderMap::new();
    h.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    h.insert(header::CACHE_CONTROL, "private, max-age=86400".parse().unwrap());
    Ok((h, png.as_ref().clone()).into_response())
}

async fn estudio_criaturas(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.criaturas(&e.gov, &v, &caminho).await?))
}

// ------------------------------------------------------------------ edição do npcgen (S4, B243)

fn tipo_de_area(t: &str) -> R<TipoDeArea> {
    TipoDeArea::de(t).map_err(|err| Erro(StatusCode::NOT_FOUND, format!("{err:#}")))
}

async fn estudio_npcgen(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.npcgen(&e.gov, &v, &caminho, q.rascunho).await?))
}

async fn estudio_area(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, tipo, indice)): Path<(String, String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    let t = tipo_de_area(&tipo)?;
    Ok(Json(e.estudio.area_do_npcgen(&e.gov, &v, &caminho, q.rascunho, t, indice).await?))
}

#[derive(Deserialize)]
struct PedidoArea {
    #[serde(default)]
    campos: serde_json::Map<String, Value>,
    filhos: Option<Vec<serde_json::Map<String, Value>>>,
}

async fn estudio_editar_area(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, tipo, indice)): Path<(String, String, String, usize)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoArea>,
) -> R<Json<Value>> {
    validar(&v)?;
    let t = tipo_de_area(&tipo)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_area_do_npcgen(&e.gov, &v, &caminho, r, t, indice, p.campos, p.filhos).await?))
}

async fn estudio_clonar_area(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, tipo, indice)): Path<(String, String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    let t = tipo_de_area(&tipo)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.clonar_area_do_npcgen(&e.gov, &v, &caminho, r, t, indice).await?))
}

async fn estudio_remover_area(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, tipo, indice)): Path<(String, String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    let t = tipo_de_area(&tipo)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_area_do_npcgen(&e.gov, &v, &caminho, r, t, indice).await?))
}

// ------------------------------------------------------------------ distritos e portais (S4, B244)

async fn estudio_distritos(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.distritos(&e.gov, &v, &caminho, q.rascunho).await?))
}

async fn estudio_portais(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.portais(&e.gov, &v, &caminho, q.rascunho).await?))
}

#[derive(Deserialize)]
struct PedidoCampos {
    campos: serde_json::Map<String, Value>,
}

async fn estudio_editar_distrito(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoCampos>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_distrito(&e.gov, &v, &caminho, r, indice, p.campos).await?))
}

async fn estudio_editar_portal(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoCampos>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_portal(&e.gov, &v, &caminho, r, indice, p.campos).await?))
}

// ------------------------------------------------------------------ teleportes (S4, B245)

async fn estudio_teleportes(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.teleportes(&e.gov, &v, &caminho, q.rascunho).await?))
}

async fn estudio_editar_teleporte(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, i32)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoCampos>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_teleporte(&e.gov, &v, r, id, p.campos).await?))
}

// ------------------------------------------------------------------ rastreador de NPCs (S4, B246)

async fn estudio_rastreador(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.rastreador(&e.gov, &v, &caminho, q.rascunho).await?))
}

async fn estudio_editar_rastreador(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, u32)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoCampos>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_rastreador(&e.gov, &v, r, id, p.campos).await?))
}

// ------------------------------------------------------------------ conversas (S3, B240)

async fn estudio_funcoes(State(e): State<Arc<Estado>>, Path(v): Path<String>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.funcoes_de_conversa()))
}

async fn estudio_listar_conversas(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    let r = e
        .estudio
        .listar_conversas(&e.gov, &v, q.rascunho, q.busca.as_deref().unwrap_or(""), q.pagina.unwrap_or(0), q.por_pagina.unwrap_or(50))
        .await?;
    Ok(Json(r))
}

async fn estudio_conversa(
    State(e): State<Arc<Estado>>,
    Path((v, indice)): Path<(String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.conversa(&e.gov, &v, q.rascunho, indice).await?))
}

#[derive(Deserialize)]
struct PedidoConversa {
    conversa: Value,
}

async fn estudio_salvar_conversa(
    State(e): State<Arc<Estado>>,
    Path((v, indice)): Path<(String, usize)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoConversa>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.salvar_conversa(&e.gov, &v, r, indice, p.conversa).await?))
}

#[derive(Deserialize)]
struct QNovaConversa {
    rascunho: Option<i64>,
    base: Option<usize>,
}

async fn estudio_nova_conversa(
    State(e): State<Arc<Estado>>,
    Path(v): Path<String>,
    Query(q): Query<QNovaConversa>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = q.rascunho.ok_or(Erro(StatusCode::BAD_REQUEST, "escolha o rascunho onde a edição fica".into()))?;
    Ok(Json(e.estudio.nova_conversa(&e.gov, &v, r, q.base).await?))
}

async fn estudio_remover_conversa(
    State(e): State<Arc<Estado>>,
    Path((v, indice)): Path<(String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_conversa(&e.gov, &v, r, indice).await?))
}

pub fn rotas(gov: Governanca, segredo: String) -> Router {
    let estado = Arc::new(Estado { gov, segredo, estudio: Default::default() });
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
        .route("/admin/{v}/estudio/tabelas", get(estudio_tabelas))
        .route("/admin/{v}/estudio/tabelas/{tabela}", get(estudio_listar))
        .route("/admin/{v}/estudio/tabelas/{tabela}/{indice}", get(estudio_ficha).put(estudio_editar).delete(estudio_remover))
        .route("/admin/{v}/estudio/tabelas/{tabela}/{indice}/clonar", post(estudio_clonar))
        .route("/admin/{v}/estudio/espacos/{espaco}", get(estudio_buscar_no_espaco))
        .route("/admin/{v}/estudio/funcoes", get(estudio_funcoes))
        .route("/admin/{v}/estudio/mapas", get(estudio_mapas))
        .route("/admin/{v}/estudio/mapas/{caminho}/pecas/{peca}", get(estudio_peca))
        .route("/admin/{v}/estudio/mapas/{caminho}/criaturas", get(estudio_criaturas))
        .route("/admin/{v}/estudio/mapas/{caminho}/npcgen", get(estudio_npcgen))
        .route("/admin/{v}/estudio/mapas/{caminho}/distritos", get(estudio_distritos))
        .route("/admin/{v}/estudio/mapas/{caminho}/distritos/{indice}", put(estudio_editar_distrito))
        .route("/admin/{v}/estudio/mapas/{caminho}/portais", get(estudio_portais))
        .route("/admin/{v}/estudio/mapas/{caminho}/portais/{indice}", put(estudio_editar_portal))
        .route("/admin/{v}/estudio/mapas/{caminho}/teleportes", get(estudio_teleportes))
        .route("/admin/{v}/estudio/teleportes/{id}", put(estudio_editar_teleporte))
        .route("/admin/{v}/estudio/mapas/{caminho}/rastreador", get(estudio_rastreador))
        .route("/admin/{v}/estudio/rastreador/{id}", put(estudio_editar_rastreador))
        .route(
            "/admin/{v}/estudio/mapas/{caminho}/npcgen/{tipo}/{indice}",
            get(estudio_area).put(estudio_editar_area).delete(estudio_remover_area),
        )
        .route("/admin/{v}/estudio/mapas/{caminho}/npcgen/{tipo}/{indice}/clonar", post(estudio_clonar_area))
        .route("/admin/{v}/estudio/conversas", get(estudio_listar_conversas).post(estudio_nova_conversa))
        .route(
            "/admin/{v}/estudio/conversas/{indice}",
            get(estudio_conversa).put(estudio_salvar_conversa).delete(estudio_remover_conversa),
        )
        .layer(DefaultBodyLimit::max(MAIOR_ARQUIVO))
        .route_layer(middleware::from_fn_with_state(estado.clone(), exigir_segredo))
        .with_state(estado)
}
