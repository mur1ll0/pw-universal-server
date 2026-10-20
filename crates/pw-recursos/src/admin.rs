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

/// A última publicação que gravou arquivo no `config` do realm (B249): o painel a compara com o `iniciado_em` do GS.
async fn ultima_do_servidor(State(e): State<Arc<Estado>>, Path(v): Path<String>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(match e.gov.repo.ultima_do_servidor(&v).await? {
        Some((numero, quando)) => json!({ "numero": numero, "publicada_em": quando }),
        None => json!({ "numero": null, "publicada_em": null }),
    }))
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
    /// B280: a ficha do `.ecm` lê o rascunho quando há um.
    rascunho: Option<i64>,
    /// B281: habitat do mascote (`inhabit_type` 0–6) para o diagnóstico de mascote.
    habitat: Option<u32>,
    /// B283: modo de voo (`fly_mode`: 0 espada, 1 asa, 2 rodas) para o diagnóstico de voo.
    voo: Option<u32>,
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

// ------------------------------------------------------------------ modelos 3D (S5, B272)

/// Os modelos da versão publicada cujo caminho contém `busca` (até 200): `.smd` (o modelo inteiro, B274) e `.ski`.
async fn estudio_modelos(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<Busca>) -> R<Json<Value>> {
    validar(&v)?;
    let todos = e.gov.buscar(&v, q.busca.as_deref().unwrap_or(""), usize::MAX)?;
    // O `.ecm` (o modelo do jogo, B277), o `.smd` (o modelo inteiro) e as peles — cada tipo com a sua cota (B278: com
    // uma cota só, os `.ecm` enchiam a lista e os `.smd` sumiam numa busca larga como "怪物").
    let limite = q.limite.unwrap_or(200).min(500);
    let mut linhas: Vec<&Value> = Vec::new();
    let mut totais = serde_json::Map::new();
    for ext in [".ecm", ".smd", ".ski"] {
        let deste: Vec<&Value> = todos.iter().filter(|x| x["caminho"].as_str().is_some_and(|c| c.to_lowercase().ends_with(ext))).collect();
        totais.insert(ext.trim_start_matches('.').to_string(), json!(deste.len()));
        linhas.extend(deste.into_iter().take(limite));
    }
    let total: u64 = totais.values().filter_map(Value::as_u64).sum();
    Ok(Json(json!({ "total": total, "por_tipo": totais, "linhas": linhas })))
}

/// B283: o diagnóstico de voo de um `.ecm` (`pw_data_loader::ecm::diagnostico_de_voo`). Os nomes enviados vêm dos `.ecm`
/// de jogador (`Models\Players\形象\…`); o vocabulário do modo, dos outros modelos de voo do elements com o mesmo modo.
#[allow(clippy::too_many_arguments)]
async fn diagnostico_de_voo(
    e: &Estado,
    v: &str,
    rascunho: Option<i64>,
    nome: &str,
    m: &pw_data_loader::ecm::Ecm,
    modo: u32,
    tipo: &str,
    gancho_do_jogador: Option<&str>,
    encaixe: &str,
) -> R<Value> {
    use std::collections::BTreeMap;
    let ganchos: Result<Vec<String>, String> = ler_smd(e, v, &m.smd).map_err(|x| x.1).and_then(|(_, _, _, b)| b).map(|b| b.ganchos.iter().map(|g| g.nome.clone()).collect());
    let mut enviadas: BTreeMap<String, usize> = BTreeMap::new();
    let mut jogadores = 0usize;
    let de_jogador: Vec<String> = e
        .gov
        .buscar(v, "形象", usize::MAX)?
        .iter()
        .filter_map(|x| x["caminho"].as_str().map(str::to_string))
        .filter(|c| {
            let cl = c.to_lowercase();
            cl.ends_with(".ecm") && cl.contains("players\\形象\\")
        })
        .collect();
    for lido in e.gov.conteudos(v, &de_jogador)? {
        let Some(j) = lido.and_then(|(_, b)| pw_data_loader::ecm::ler(&b)) else { continue };
        jogadores += 1;
        let mut deste: Vec<&str> = j.acoes.iter().flat_map(|a| a.filhas.iter()).filter(|f| pw_data_loader::ecm::chega_ao_voo(f, modo)).map(|f| f.nome.as_str()).collect();
        deste.sort_unstable();
        deste.dedup();
        for n in deste {
            *enviadas.entry(n.to_string()).or_default() += 1;
        }
    }
    let mut dos_colegas: BTreeMap<String, usize> = BTreeMap::new();
    let mut colegas = 0usize;
    let do_modo: Vec<String> = e
        .estudio
        .modelos_de_voo(&e.gov, v, rascunho)
        .await
        .map_err(|x| Erro(StatusCode::BAD_REQUEST, x.to_string()))?
        .into_iter()
        .filter(|(c, mo)| *mo == modo && !c.eq_ignore_ascii_case(nome))
        .map(|(c, _)| c)
        .collect();
    for lido in e.gov.conteudos(v, &do_modo)? {
        let Some(o) = lido.and_then(|(_, b)| pw_data_loader::ecm::ler(&b)) else { continue };
        colegas += 1;
        let mut deste: Vec<&str> = o.acoes.iter().map(|a| a.nome.as_str()).collect();
        deste.sort_unstable();
        deste.dedup();
        for n in deste {
            *dos_colegas.entry(n.to_string()).or_default() += 1;
        }
    }
    let linhas = pw_data_loader::ecm::diagnostico_de_voo(&m.acoes, &enviadas, &dos_colegas, colegas);
    Ok(json!({
        "tipo": tipo, "gancho_do_jogador": gancho_do_jogador, "encaixe": encaixe,
        "encaixe_presente": ganchos.as_ref().ok().map(|g| g.iter().any(|x| x == encaixe)),
        "ganchos": ganchos.as_ref().ok(), "esqueleto_erro": ganchos.as_ref().err(),
        "jogadores": jogadores, "colegas": colegas, "acoes": linhas,
    }))
}

/// Um caminho relativo à pasta de outro (`A3DSkinModelMan::LoadModelFromFile`: `strPath + "\\" + strFile`).
fn ao_lado(caminho: &str, nome: &str) -> String {
    match caminho.rsplit_once('\\') {
        Some((pasta, _)) => format!("{pasta}\\{nome}"),
        None => nome.to_string(),
    }
}

/// O modelo (`.smd`): a cabeça, as peles lidas (caminho e `.ski`) e o esqueleto (`Ok(.bon)`, ou o motivo de não ter).
#[allow(clippy::type_complexity)]
fn ler_smd(e: &Estado, v: &str, caminho: &str) -> Result<(String, pw_data_loader::smd::CabecaDoSmd, Vec<(String, Result<pw_data_loader::ski::Ski, String>)>, Result<pw_data_loader::bon::Bon, String>), Erro> {
    let (nome, dados) = e.gov.conteudo(v, caminho)?;
    let c = pw_data_loader::smd::cabeca(&dados).map_err(|x| Erro(StatusCode::BAD_REQUEST, format!("{nome}: {x}")))?;
    let peles = c
        .peles
        .iter()
        .map(|p| {
            let cp = ao_lado(&nome, p);
            let s = e.gov.conteudo(v, &cp).map_err(|x| x.to_string()).and_then(|(_, b)| pw_data_loader::ski::Ski::ler(&b).map_err(|x| x.to_string()));
            (cp, s)
        })
        .collect();
    let esqueleto = if c.esqueleto.is_empty() {
        Err("o modelo não cita esqueleto".to_string())
    } else {
        e.gov.conteudo(v, &ao_lado(&nome, &c.esqueleto)).map_err(|x| x.to_string()).and_then(|(_, b)| pw_data_loader::bon::Bon::ler(&b).map_err(|x| x.to_string()))
    };
    Ok((nome, c, peles, esqueleto))
}

fn ler_ski(e: &Estado, v: &str, caminho: &str) -> Result<(String, pw_data_loader::ski::Ski), Erro> {
    if !caminho.to_lowercase().ends_with(".ski") {
        return Err(Erro(StatusCode::BAD_REQUEST, "só .ski por enquanto".into()));
    }
    let (nome, dados) = e.gov.conteudo(v, caminho)?;
    let s = pw_data_loader::ski::Ski::ler(&dados).map_err(|x| Erro(StatusCode::BAD_REQUEST, format!("{nome}: {x}")))?;
    Ok((nome, s))
}

/// Onde o cliente acha a textura `nome` do `.ski` em `caminho` (`A3DSkin::Load`, `A3DSkin.cpp`: primeiro
/// `<pasta>\Textures\<nome>`, depois `<pasta>\Tex_<título>\<nome>`). Devolve o caminho e os bytes.
fn achar_textura(e: &Estado, v: &str, caminho: &str, nome: &str) -> Option<(String, Vec<u8>)> {
    let (pasta, arquivo) = caminho.rsplit_once('\\')?;
    let titulo = arquivo.rsplit_once('.').map_or(arquivo, |x| x.0);
    [format!("{pasta}\\Textures\\{nome}"), format!("{pasta}\\Tex_{titulo}\\{nome}")]
        .into_iter()
        .find_map(|c| e.gov.conteudo(v, &c).ok())
}

/// A textura em PNG (para o GLB), pelo decodificador do `pw-data-loader` (DDS DXT1/3/5, RGBA, TGA).
fn textura_em_png(bytes: &[u8]) -> Option<pw_data_loader::glb::Textura> {
    let img = pw_data_loader::imagem::decodificar(bytes)?;
    let mut png = Vec::new();
    {
        let mut c = png::Encoder::new(&mut png, img.largura, img.altura);
        c.set_color(png::ColorType::Rgba);
        c.set_depth(png::BitDepth::Eight);
        let mut w = c.write_header().ok()?;
        w.write_image_data(&img.rgba).ok()?;
    }
    Some(pw_data_loader::glb::Textura { png, transparente: img.tem_transparencia() })
}

/// A ficha do `.ski`: versão, ossos, texturas (com onde foram achadas), materiais e as malhas (sem os vértices). Para
/// um `.smd` (B274): o esqueleto (ossos, juntas, ganchos, ou por que não foi lido) e a ficha de cada pele.
async fn estudio_modelo(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QCaminho>) -> R<Json<Value>> {
    validar(&v)?;
    if q.caminho.to_lowercase().ends_with(".ecm") {
        // B277: o modelo do jogo — o .smd, as ações combinadas e o diagnóstico de monstro (os nomes que o cliente pede,
        // `pw_data_loader::ecm::pedidos_de_monstro`, pela tabela `configs\actions_npc.txt` do próprio cliente).
        let (nome, mut dados) = e.gov.conteudo(&v, &q.caminho)?;
        if let Some(r) = q.rascunho {
            dados = e.estudio.bytes_do_cliente(&e.gov, &v, &q.caminho, Some(r)).await?;
        }
        let m = pw_data_loader::ecm::ler(&dados).ok_or_else(|| Erro(StatusCode::BAD_REQUEST, format!("{nome}: .ecm ilegível")))?;
        let bases_do_smd: Option<Vec<String>> = e.gov.conteudo(&v, &m.smd).ok().and_then(|(_, b)| pw_data_loader::smd::cabeca(&b).ok()).map(|c| c.lista_de_acoes.into_iter().map(|a| a.nome).collect());
        let smd_caminho = e.gov.conteudo(&v, &m.smd).ok().map(|(n, _)| n);
        let combinadas: Vec<Value> = m.acoes.iter().map(|a| {
            let faltam: Vec<&String> = bases_do_smd.as_ref().map_or(vec![], |b| a.bases.iter().filter(|x| !b.contains(x)).collect());
            json!({ "nome": a.nome, "bases": a.bases, "bases_ausentes": faltam, "filhas": a.filhas })
        }).collect();
        let tabela = e.gov.conteudo(&v, "configs\\actions_npc.txt").ok().map(|(_, b)| pw_data_loader::ecm::tabela_de_nomes(&b)).unwrap_or_default();
        let diagnosticar = |pedidos: Vec<(usize, String, Option<String>)>| -> Vec<Value> { pedidos.into_iter().map(|(i, pedido, alternativa)| {
            // Ferido: o motor tenta o nome e, sem a combinada, a alternativa (`EC_NPCModel.cpp:692-702`).
            let achada = m.acoes.iter().find(|a| a.nome == pedido).or_else(|| alternativa.as_ref().and_then(|x| m.acoes.iter().find(|a| &a.nome == x)));
            let so_base = bases_do_smd.as_ref().is_some_and(|b| b.contains(&pedido) || alternativa.as_ref().is_some_and(|x| b.contains(x)));
            let estado = match achada {
                // O comentário de `EC_NPCModel.cpp:693-697`: o ferido antigo ficava só no .bon/.smd, não no .ecm — o motor
                // 1.5.5 só toca combinadas (`GetComActByName`, `EC_Model.h:577-583`).
                None if so_base => "só existe como ação-base no .smd (o motor 1.5.5 só toca combinadas do .ecm)",
                None => "sem ação combinada no .ecm",
                Some(a) if a.bases.is_empty() => "ação combinada sem ação-base (não anima)",
                Some(a) if bases_do_smd.as_ref().is_some_and(|b| a.bases.iter().any(|x| !b.contains(x))) => "ação-base ausente no .smd",
                Some(_) => "ok",
            };
            json!({ "indice": i, "pedido": pedido, "alternativa": alternativa, "estado": estado })
        }).collect() };
        let diagnostico = diagnosticar(pw_data_loader::ecm::pedidos_de_monstro(&tabela));
        // B281: mascote (invocado, de combate ou evolutivo) — os nomes com o ambiente do habitat.
        let de_mascote = q.habitat.filter(|h| *h <= 6).map(|h| diagnosticar(pw_data_loader::ecm::pedidos_de_mascote(&tabela, h)));
        // B283: voo — o encaixe no esqueleto e as ações que os modelos de jogador mandam aos filhos.
        let de_voo = match q.voo.and_then(|x| pw_data_loader::ecm::encaixe_de_voo(x).map(|e| (x, e))) {
            Some((modo, (tipo, gancho_do_jogador, encaixe))) => Some(diagnostico_de_voo(&e, &v, q.rascunho, &nome, &m, modo, tipo, gancho_do_jogador, encaixe).await?),
            None => None,
        };
        return Ok(Json(json!({ "caminho": nome, "tipo": "ecm", "versao": m.versao, "smd": smd_caminho.unwrap_or(m.smd), "smd_lido": bases_do_smd.is_some(), "voo": q.voo, "diagnostico_de_voo": de_voo,
            "combinadas": combinadas, "bases_do_smd": bases_do_smd.clone().unwrap_or_default(), "tabela_de_acoes": tabela.len(), "diagnostico_de_monstro": diagnostico,
            "habitat": q.habitat, "diagnostico_de_mascote": de_mascote })));
    }
    if q.caminho.to_lowercase().ends_with(".smd") {
        let (nome, c, peles, esqueleto) = ler_smd(&e, &v, &q.caminho)?;
        let esq = match &esqueleto {
            Ok(b) => json!({ "arquivo": c.esqueleto, "versao": b.versao, "ossos": b.ossos.iter().map(|o| json!([o.nome, o.mae])).collect::<Vec<_>>(),
                "juntas": b.juntas.len(), "ganchos": b.ganchos.iter().map(|g| g.nome.clone()).collect::<Vec<_>>(), "fps": b.fps }),
            Err(motivo) => json!({ "arquivo": c.esqueleto, "erro": motivo }),
        };
        let peles: Vec<Value> = peles
            .iter()
            .map(|(cp, s)| match s {
                Ok(s) => json!({ "caminho": cp, "versao": s.versao, "texturas": s.texturas, "malhas": s.malhas_de_pele.len() + s.malhas_rigidas.len(),
                    "triangulos": s.malhas_de_pele.iter().map(|m| m.indices.len() / 3).sum::<usize>() + s.malhas_rigidas.iter().map(|m| m.indices.len() / 3).sum::<usize>() }),
                Err(motivo) => json!({ "caminho": cp, "erro": motivo }),
            })
            .collect();
        // B276: as ações (nome e intervalo em ms pelo FPS do esqueleto; o motor as descarta com .smd < 7 e .bon v6).
        let fps = esqueleto.as_ref().map_or(0, |b| b.fps);
        let descartadas = c.versao < 7 && esqueleto.as_ref().is_ok_and(|b| b.versao >= 6);
        let acoes: Vec<Value> = c
            .lista_de_acoes
            .iter()
            .map(|a| {
                let ms = |q: f32| if fps > 0 { q * 1000.0 / fps as f32 } else { 0.0 };
                json!({ "nome": a.nome, "inicio_ms": ms(a.quadro_inicial), "fim_ms": ms(a.quadro_final), "stck": a.stck })
            })
            .collect();
        return Ok(Json(json!({ "caminho": nome, "tipo": "modelo", "versao": c.versao, "acoes": c.acoes, "lista_de_acoes": acoes,
            "acoes_descartadas": descartadas, "esqueleto": esq, "peles": peles })));
    }
    let (nome, s) = ler_ski(&e, &v, &q.caminho)?;
    let mut malhas: Vec<Value> = s.malhas_de_pele.iter().map(|m| json!({ "nome": m.nome, "tipo": "pele", "vertices": m.vertices.len(), "triangulos": m.indices.len() / 3, "textura": m.textura, "material": m.material })).collect();
    malhas.extend(s.malhas_rigidas.iter().map(|m| json!({ "nome": m.nome, "tipo": "rígida", "osso": m.osso, "vertices": m.vertices.len(), "triangulos": m.indices.len() / 3, "textura": m.textura, "material": m.material })));
    let achadas: Vec<Value> = s.texturas.iter().map(|t| achar_textura(&e, &v, &nome, t).map_or(Value::Null, |x| json!(x.0))).collect();
    Ok(Json(json!({
        "caminho": nome, "versao": s.versao, "ossos": s.ossos, "ossos_do_esqueleto": s.ossos_do_esqueleto, "texturas": s.texturas,
        "texturas_achadas": achadas,
        "materiais": s.materiais.iter().map(|m| json!({ "nome": m.nome, "difusa": m.difusa, "dois_lados": m.dois_lados })).collect::<Vec<_>>(),
        "malhas": malhas,
    })))
}

/// O GLB do `.ski` (`pw_data_loader::glb`), para o visualizador e para levar ao Blender.
#[derive(Deserialize)]
struct QModelo {
    caminho: String,
    acao: Option<String>,
}

async fn estudio_modelo_glb(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QModelo>) -> R<Response> {
    validar(&v)?;
    let ext = q.caminho.to_lowercase();
    if ext.ends_with(".smd") || ext.ends_with(".ecm") {
        // B274: o modelo inteiro — as peles lidas, com as texturas de cada uma, e o esqueleto (sem ele, fica parado).
        // B282: um `.ecm` monta o modelo como o jogo (`CECNPC::LoadNPCModel`, `EC_NPC.cpp:186-222`): o `.smd` dele SEM
        // as peles do `.smd` (`LSF_NOSKIN`) e a pele `<.ecm com extensão .ski>`; sem ela, as `AddiSkinPath`
        // (`"Models\" + caminho`, `EC_Model.cpp:678-689`) e, por fim, as do `.smd`.
        let (nome, cabeca, peles, esqueleto) = if ext.ends_with(".ecm") {
            let (nome_ecm, dados) = e.gov.conteudo(&v, &q.caminho)?;
            let m = pw_data_loader::ecm::ler(&dados).ok_or_else(|| Erro(StatusCode::BAD_REQUEST, format!("{nome_ecm}: .ecm ilegível")))?;
            let (_, cabeca, peles_do_smd, esqueleto) = ler_smd(&e, &v, &m.smd)?;
            let ler_pele = |c: &str| e.gov.conteudo(&v, c).map_err(|x| x.to_string()).and_then(|(n, b)| pw_data_loader::ski::Ski::ler(&b).map(|s| (n, s)).map_err(|x| x.to_string()));
            let irma = format!("{}.ski", nome_ecm.rsplit_once('.').map_or(nome_ecm.as_str(), |x| x.0));
            let mut peles: Vec<(String, Result<pw_data_loader::ski::Ski, String>)> = Vec::new();
            match ler_pele(&irma) {
                Ok((n, s)) => peles.push((n, Ok(s))),
                Err(_) => {
                    for a in &m.peles_adicionais {
                        let c = format!("Models\\{a}");
                        match ler_pele(&c) {
                            Ok((n, s)) => peles.push((n, Ok(s))),
                            Err(x) => peles.push((c, Err(x))),
                        }
                    }
                    if !peles.iter().any(|p| p.1.is_ok()) {
                        peles.extend(peles_do_smd);
                    }
                }
            }
            (nome_ecm, cabeca, peles, esqueleto)
        } else {
            ler_smd(&e, &v, &q.caminho)?
        };
        let lidas: Vec<(String, pw_data_loader::ski::Ski)> = peles.into_iter().filter_map(|(cp, s)| s.ok().map(|s| (cp, s))).collect();
        if lidas.is_empty() && esqueleto.is_err() {
            // B278: sem pele lida (o .smd não cita nenhuma — p.ex. corpo de jogador, cuja pele vem do equipamento — ou as
            // citadas não existem no cliente) e sem esqueleto não há o que mostrar; com esqueleto, sai só ele.
            let citadas: Vec<&str> = cabeca.peles.iter().map(String::as_str).collect();
            return Err(Erro(StatusCode::BAD_REQUEST, if citadas.is_empty() {
                format!("{nome}: o .smd não cita pele nem tem esqueleto lido")
            } else {
                format!("{nome}: nenhuma das peles citadas existe no cliente ({}) e o esqueleto não foi lido", citadas.join(", "))
            }));
        }
        let texturas: Vec<Vec<Option<pw_data_loader::glb::Textura>>> = lidas
            .iter()
            .map(|(cp, s)| s.texturas.iter().map(|t| achar_textura(&e, &v, cp, t).and_then(|(_, b)| textura_em_png(&b))).collect())
            .collect();
        let fornecedores: Vec<Box<dyn Fn(usize) -> Option<pw_data_loader::glb::Textura>>> = texturas
            .iter()
            .map(|lista| Box::new(move |i: usize| lista.get(i).and_then(|t| t.as_ref()).map(|t| pw_data_loader::glb::Textura { png: t.png.clone(), transparente: t.transparente })) as Box<dyn Fn(usize) -> Option<pw_data_loader::glb::Textura>>)
            .collect();
        let lista: Vec<pw_data_loader::glb::PeleDoModelo> = lidas.iter().zip(&fornecedores).map(|((_, s), f)| pw_data_loader::glb::PeleDoModelo { ski: s, textura: f.as_ref() }).collect();
        // B276: com `acao`, a animação dela — trilhas embutidas no .bon v5 ou o .stck da ação (pasta das trilhas: a do
        // .smd v8, ou `tcks_<título>`).
        if let (Some(nome_acao), Ok(b)) = (q.acao.as_deref(), esqueleto.as_ref()) {
            let a = cabeca.lista_de_acoes.iter().find(|a| a.nome == nome_acao).ok_or_else(|| Erro(StatusCode::BAD_REQUEST, format!("ação {nome_acao} não existe")))?;
            if b.fps <= 0 || (cabeca.versao < 7 && b.versao >= 6) {
                return Err(Erro(StatusCode::BAD_REQUEST, "o motor descarta as ações deste modelo (.smd < 7 com .bon v6)".into()));
            }
            let stck;
            let trilhas: Vec<(i32, &pw_data_loader::trilha::TrilhaDeOsso)> = if b.versao < 6 {
                b.trilhas.iter().enumerate().map(|(i, t)| (i as i32, t)).collect()
            } else {
                let arquivo = a.stck.clone().ok_or_else(|| Erro(StatusCode::BAD_REQUEST, "a ação não cita .stck".into()))?;
                let titulo = nome.rsplit('\\').next().and_then(|x| x.rsplit_once('.')).map_or("", |x| x.0).to_string();
                let pasta = if cabeca.versao >= 8 { cabeca.pasta_das_trilhas.clone() } else { format!("tcks_{titulo}") };
                let caminho = ao_lado(&nome, &format!("{pasta}\\{arquivo}"));
                let (_, bytes) = e.gov.conteudo(&v, &caminho).map_err(|x| Erro(StatusCode::BAD_REQUEST, format!("{caminho}: {x}")))?;
                stck = pw_data_loader::stck::ler(&bytes).map_err(|x| Erro(StatusCode::BAD_REQUEST, format!("{caminho}: {x}")))?;
                stck.trilhas.iter().map(|(j, t)| (*j, t)).collect()
            };
            let fps = b.fps as f32;
            let anim = pw_data_loader::glb::AnimacaoDoModelo { nome: a.nome.clone(), trilhas, inicio_ms: a.quadro_inicial * 1000.0 / fps, fim_ms: a.quadro_final * 1000.0 / fps };
            let glb = pw_data_loader::glb::do_modelo_animado(&lista, b, &anim, &nome);
            return glb_como_resposta(&nome, glb);
        }
        let glb = pw_data_loader::glb::do_modelo(&lista, esqueleto.as_ref().ok(), &nome);
        return glb_como_resposta(&nome, glb);
    }
    let (nome, s) = ler_ski(&e, &v, &q.caminho)?;
    // B273: as texturas, achadas como o cliente acha e convertidas para PNG; a que falta fica só com a cor.
    let texturas: Vec<Option<pw_data_loader::glb::Textura>> =
        s.texturas.iter().map(|t| achar_textura(&e, &v, &nome, t).and_then(|(_, b)| textura_em_png(&b))).collect();
    let glb = pw_data_loader::glb::do_ski_com_texturas(&s, &nome, &|i| texturas.get(i).and_then(|t| t.as_ref()).map(|t| pw_data_loader::glb::Textura { png: t.png.clone(), transparente: t.transparente }));
    glb_como_resposta(&nome, glb)
}

fn glb_como_resposta(nome: &str, glb: Vec<u8>) -> R<Response> {
    let arquivo = nome.rsplit('\\').next().unwrap_or("modelo").replace(['"', '\r', '\n'], "_");
    let mut h = HeaderMap::new();
    h.insert(header::CONTENT_TYPE, "model/gltf-binary".parse().unwrap());
    let disposicao = format!("inline; filename*=UTF-8''{}.glb", urlencoding(&arquivo));
    h.insert(header::CONTENT_DISPOSITION, disposicao.parse().map_err(|_| Erro(StatusCode::BAD_REQUEST, "nome".into()))?);
    Ok((h, glb).into_response())
}

#[derive(Deserialize)]
struct PedidoAcaoNova {
    caminho: String,
    origem: String,
    nova: String,
    base: Option<String>,
    /// B285: copiar de outro `.ecm` (mesma versão, ações-base existentes no `.smd` do destino).
    de: Option<String>,
}

/// B280: ação combinada nova no `.ecm`, clonando outra (corrige uma ação que o diagnóstico acusa).
async fn estudio_acao_nova(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QEstudio>, Json(p): Json<PedidoAcaoNova>) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    if let Some(de) = p.de {
        if p.base.is_some() {
            return Err(Erro(StatusCode::BAD_REQUEST, "copiar de outro .ecm não troca a ação-base".into()));
        }
        return Ok(Json(e.estudio.acao_copiada_no_ecm(&e.gov, &v, &p.caminho, r, de, p.origem, p.nova).await?));
    }
    Ok(Json(e.estudio.acao_nova_no_ecm(&e.gov, &v, &p.caminho, r, p.origem, p.nova, p.base).await?))
}

#[derive(Deserialize)]
struct QDoadores {
    caminho: String,
    acao: String,
}

/// B285: quem pode doar uma ação a um `.ecm` (mesmo esqueleto, mesma versão de arquivo) — `Estudio::doadores`.
async fn estudio_doadores(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QDoadores>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.doadores(&e.gov, &v, &q.caminho, &q.acao).await?))
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

#[derive(Deserialize)]
struct QNascimentos {
    id: i64,
    rascunho: Option<i64>,
}

/// B284: onde um id nasce (todos os mapas com npcgen) — `Estudio::onde_nasce`.
async fn estudio_nascimentos(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QNascimentos>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.onde_nasce(&e.gov, &v, q.id, q.rascunho).await?))
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

// ------------------------------------------------------------------ distritos e portais novos (B251)

#[derive(Deserialize)]
struct PedidoNovo {
    base: usize,
    #[serde(default)]
    campos: serde_json::Map<String, Value>,
}

async fn estudio_novo_distrito(
    State(e): State<Arc<Estado>>,
    Path((v, caminho)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoNovo>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.novo_distrito(&e.gov, &v, &caminho, r, p.base, p.campos).await?))
}

async fn estudio_remover_distrito(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_distrito(&e.gov, &v, &caminho, r, indice).await?))
}

async fn estudio_novo_portal(
    State(e): State<Arc<Estado>>,
    Path((v, caminho)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoNovo>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.novo_portal(&e.gov, &v, &caminho, r, p.base).await?))
}

async fn estudio_remover_portal(
    State(e): State<Arc<Estado>>,
    Path((v, caminho, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_portal(&e.gov, &v, &caminho, r, indice).await?))
}

// ------------------------------------------------------------------ alinhar o servidor com o cliente (B253)

async fn estudio_alinhamento(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.alinhamento(&e.gov, &v, q.rascunho).await?))
}

async fn estudio_alinhar(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.alinhar_com_o_cliente(&e.gov, &v, &caminho, r).await?))
}

// ------------------------------------------------------------------ Loja Gold (S7, B254)

async fn estudio_loja(State(e): State<Arc<Estado>>, Path((v, arquivo)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.loja(&e.gov, &v, &arquivo, q.rascunho).await?))
}

async fn estudio_categoria_da_loja(
    State(e): State<Arc<Estado>>,
    Path((v, arquivo)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<Value>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.categoria_da_loja(&e.gov, &v, &arquivo, r, p).await?))
}

async fn estudio_oferta(State(e): State<Arc<Estado>>, Path((v, arquivo, indice)): Path<(String, String, usize)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.oferta(&e.gov, &v, &arquivo, q.rascunho, indice).await?))
}

async fn estudio_editar_oferta(
    State(e): State<Arc<Estado>>,
    Path((v, arquivo, indice)): Path<(String, String, usize)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoCampos>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_oferta(&e.gov, &v, &arquivo, r, indice, p.campos).await?))
}

async fn estudio_clonar_oferta(
    State(e): State<Arc<Estado>>,
    Path((v, arquivo)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoNovo>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.clonar_oferta(&e.gov, &v, &arquivo, r, p.base).await?))
}

async fn estudio_remover_oferta(State(e): State<Arc<Estado>>, Path((v, arquivo, indice)): Path<(String, String, usize)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_oferta(&e.gov, &v, &arquivo, r, indice).await?))
}

// ------------------------------------------------------------------ missões (S7, B255)

#[derive(Deserialize)]
struct QMissoes {
    rascunho: Option<i64>,
    busca: Option<String>,
    topo: Option<bool>,
    pagina: Option<usize>,
    por_pagina: Option<usize>,
}

async fn estudio_missoes(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QMissoes>) -> R<Json<Value>> {
    validar(&v)?;
    let r = e.estudio.missoes(&e.gov, &v, q.rascunho, q.busca.as_deref().unwrap_or(""), q.topo.unwrap_or(true), q.pagina.unwrap_or(0),
        q.por_pagina.unwrap_or(100).clamp(1, 500)).await?;
    Ok(Json(r))
}

async fn estudio_missao(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, u32)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.missao(&e.gov, &v, q.rascunho, id).await?))
}

async fn estudio_editar_missao(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, u32)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoCampos>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_missao(&e.gov, &v, r, id, p.campos).await?))
}

#[derive(Deserialize)]
struct PedidoClone {
    novo: Option<u32>,
}

async fn estudio_clonar_missao(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, u32)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoClone>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.clonar_missao(&e.gov, &v, r, id, p.novo).await?))
}

async fn estudio_remover_missao(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, u32)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_missao(&e.gov, &v, r, id).await?))
}

// ------------------------------------------------------------------ comportamento (aipolicy, S7, B258)

async fn estudio_comportamentos(State(e): State<Arc<Estado>>, Path(v): Path<String>, Query(q): Query<QBusca>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.comportamentos(&e.gov, &v, q.busca.as_deref().unwrap_or("")).await?))
}

async fn estudio_comportamento(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, u32)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.comportamento(&e.gov, &v, q.rascunho, id).await?))
}

async fn estudio_editar_comportamento(
    State(e): State<Arc<Estado>>,
    Path((v, id)): Path<(String, u32)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<Value>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_comportamento(&e.gov, &v, r, id, p).await?))
}

// ------------------------------------------------------------------ textos do cliente (S7, B260)

#[derive(Deserialize)]
struct QTextos {
    rascunho: Option<i64>,
    busca: Option<String>,
    pagina: Option<usize>,
    por_pagina: Option<usize>,
}

async fn estudio_textos(State(e): State<Arc<Estado>>, Path((v, arquivo)): Path<(String, String)>, Query(q): Query<QTextos>) -> R<Json<Value>> {
    validar(&v)?;
    let r = e.estudio.textos(&e.gov, &v, &arquivo, q.rascunho, q.busca.as_deref().unwrap_or(""), q.pagina.unwrap_or(0),
        q.por_pagina.unwrap_or(100).clamp(1, 500)).await?;
    Ok(Json(r))
}

#[derive(Deserialize)]
struct PedidoTexto {
    texto: String,
}

async fn estudio_editar_texto(
    State(e): State<Arc<Estado>>,
    Path((v, arquivo, chave)): Path<(String, String, i64)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoTexto>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_texto(&e.gov, &v, &arquivo, r, chave, p.texto, false).await?))
}

#[derive(Deserialize)]
struct PedidoTextoNovo {
    id: i64,
    texto: String,
}

async fn estudio_novo_texto(
    State(e): State<Arc<Estado>>,
    Path((v, arquivo)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoTextoNovo>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.editar_texto(&e.gov, &v, &arquivo, r, p.id, p.texto, true).await?))
}

// ------------------------------------------------------------------ teleportes (S4, B245)

async fn estudio_teleportes(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.teleportes(&e.gov, &v, &caminho, q.rascunho).await?))
}

#[derive(Deserialize)]
struct PedidoNovoDestino {
    base: i32,
    #[serde(default)]
    campos: serde_json::Map<String, Value>,
}

async fn estudio_novo_teleporte(
    State(e): State<Arc<Estado>>,
    Path((v, caminho)): Path<(String, String)>,
    Query(q): Query<QEstudio>,
    Json(p): Json<PedidoNovoDestino>,
) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.novo_teleporte(&e.gov, &v, &caminho, r, p.base, p.campos).await?))
}

async fn estudio_remover_teleporte(State(e): State<Arc<Estado>>, Path((v, id)): Path<(String, i32)>, Query(q): Query<QEstudio>) -> R<Json<Value>> {
    validar(&v)?;
    let r = rascunho_obrigatorio(&q)?;
    Ok(Json(e.estudio.remover_teleporte(&e.gov, &v, r, id).await?))
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

// ------------------------------------------------------------------ altura do chão (S4, B248)

#[derive(Deserialize)]
struct QAltura {
    x: f32,
    z: f32,
}

async fn estudio_altura(State(e): State<Arc<Estado>>, Path((v, caminho)): Path<(String, String)>, Query(q): Query<QAltura>) -> R<Json<Value>> {
    validar(&v)?;
    Ok(Json(e.estudio.altura(&e.gov, &v, &caminho, q.x, q.z).await?))
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
        .route("/admin/{v}/servidor/ultima", get(ultima_do_servidor))
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
        .route("/admin/{v}/estudio/nascimentos", get(estudio_nascimentos))
        .route("/admin/{v}/estudio/mapas/{caminho}/pecas/{peca}", get(estudio_peca))
        .route("/admin/{v}/estudio/mapas/{caminho}/criaturas", get(estudio_criaturas))
        .route("/admin/{v}/estudio/mapas/{caminho}/npcgen", get(estudio_npcgen))
        .route("/admin/{v}/estudio/mapas/{caminho}/distritos", get(estudio_distritos).post(estudio_novo_distrito))
        .route("/admin/{v}/estudio/mapas/{caminho}/distritos/{indice}", put(estudio_editar_distrito).delete(estudio_remover_distrito))
        .route("/admin/{v}/estudio/mapas/{caminho}/portais", get(estudio_portais).post(estudio_novo_portal))
        .route("/admin/{v}/estudio/mapas/{caminho}/portais/{indice}", put(estudio_editar_portal).delete(estudio_remover_portal))
        .route("/admin/{v}/estudio/mapas/{caminho}/teleportes", get(estudio_teleportes).post(estudio_novo_teleporte))
        .route("/admin/{v}/estudio/teleportes/{id}", put(estudio_editar_teleporte).delete(estudio_remover_teleporte))
        .route("/admin/{v}/estudio/mapas/{caminho}/rastreador", get(estudio_rastreador))
        .route("/admin/{v}/estudio/mapas/{caminho}/altura", get(estudio_altura))
        .route("/admin/{v}/estudio/alinhamento", get(estudio_alinhamento))
        .route("/admin/{v}/estudio/missoes", get(estudio_missoes))
        .route("/admin/{v}/estudio/comportamentos", get(estudio_comportamentos))
        .route("/admin/{v}/estudio/modelos", get(estudio_modelos))
        .route("/admin/{v}/estudio/modelos/ficha", get(estudio_modelo))
        .route("/admin/{v}/estudio/modelos/glb", get(estudio_modelo_glb))
        .route("/admin/{v}/estudio/modelos/acao", post(estudio_acao_nova))
        .route("/admin/{v}/estudio/modelos/doadores", get(estudio_doadores))
        .route("/admin/{v}/estudio/textos/{arquivo}", get(estudio_textos).post(estudio_novo_texto))
        .route("/admin/{v}/estudio/textos/{arquivo}/{chave}", put(estudio_editar_texto))
        .route("/admin/{v}/estudio/comportamentos/{id}", get(estudio_comportamento).put(estudio_editar_comportamento))
        .route("/admin/{v}/estudio/missoes/{id}", get(estudio_missao).put(estudio_editar_missao).delete(estudio_remover_missao))
        .route("/admin/{v}/estudio/missoes/{id}/clonar", post(estudio_clonar_missao))
        .route("/admin/{v}/estudio/loja/{arquivo}", get(estudio_loja).post(estudio_clonar_oferta))
        .route("/admin/{v}/estudio/loja/{arquivo}/categorias", post(estudio_categoria_da_loja))
        .route("/admin/{v}/estudio/loja/{arquivo}/{indice}", get(estudio_oferta).put(estudio_editar_oferta).delete(estudio_remover_oferta))
        .route("/admin/{v}/estudio/mapas/{caminho}/alinhar", post(estudio_alinhar))
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
