//! Launcher do pw-universal (P2, B234; usabilidade no B235). A lógica é toda do `pw-atualizador`;
//! aqui ficam a janela (Tauri, `ui/index.html`), a configuração ao lado do `.exe`
//! (`pw-launcher.json`: servidor, versão escolhida e **uma pasta por versão**), a escolha de pasta
//! pela janela do Windows (`tauri-plugin-dialog`) e a chave pública embutida (`chave_publica.txt`).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pw_atualizador::cliente::{self, Origem, Progresso, Servidor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

const CHAVE_PUBLICA: &str = include_str!("../chave_publica.txt");
const SERVIDOR_PADRAO: &str = "http://127.0.0.1:8100";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    servidor: String,
    /// Última versão escolhida.
    #[serde(default)]
    versao: String,
    /// Pasta `element` de cada versão.
    #[serde(default)]
    pastas: BTreeMap<String, String>,
    /// Configuração do B234 (uma pasta só): migrada para `pastas` ao ler.
    #[serde(default, skip_serializing)]
    pasta: String,
}

fn caminho_config() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .unwrap_or_default()
        .join("pw-launcher.json")
}

fn ler_config() -> Config {
    let mut c: Config = std::fs::read(caminho_config())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if c.servidor.is_empty() {
        c.servidor = SERVIDOR_PADRAO.into();
    }
    if !c.pasta.is_empty() && !c.versao.is_empty() {
        let pasta = std::mem::take(&mut c.pasta);
        c.pastas.entry(c.versao.clone()).or_insert(pasta);
    }
    c
}

fn gravar_config(c: &Config) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(c).map_err(|e| e.to_string())?;
    std::fs::write(caminho_config(), bytes).map_err(|e| format!("gravar {}: {e}", caminho_config().display()))
}

fn servidor(url: &str) -> Servidor {
    Servidor::new(Origem::Http(url.trim().trim_end_matches('/').to_string()), CHAVE_PUBLICA.trim())
}

fn erro(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn pasta_da_versao(versao: &str) -> Result<PathBuf, String> {
    ler_config()
        .pastas
        .get(versao)
        .map(PathBuf::from)
        .ok_or_else(|| format!("nenhuma pasta escolhida para o {versao}"))
}

fn lembrar_pasta(versao: &str, pasta: &Path) -> Result<(), String> {
    let mut c = ler_config();
    c.versao = versao.into();
    c.pastas.insert(versao.into(), pasta.display().to_string());
    gravar_config(&c)
}

/// Roda `f` fora da thread da janela, mandando o progresso como evento (no máximo ~5 por segundo).
async fn em_segundo_plano<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&mut dyn FnMut(Progresso)) -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut ultimo = Instant::now() - Duration::from_secs(1);
        let mut ao_progredir = |p: Progresso| {
            if ultimo.elapsed() >= Duration::from_millis(200) || p.feito == p.total {
                ultimo = Instant::now();
                let _ = app.emit("progresso", &p);
            }
        };
        f(&mut ao_progredir).map_err(erro)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct Inicio {
    servidor: String,
    versao: String,
    versoes: Vec<String>,
    erro_servidor: Option<String>,
}

/// Ao abrir: a configuração e as versões que o servidor oferece.
#[tauri::command]
async fn iniciar() -> Inicio {
    let c = ler_config();
    let url = c.servidor.clone();
    let r = tauri::async_runtime::spawn_blocking(move || servidor(&url).versoes().map_err(erro))
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
    let (versoes, erro_servidor) = match r {
        Ok(v) => (v, None),
        Err(e) => (vec![], Some(e)),
    };
    Inicio { servidor: c.servidor, versao: c.versao, versoes, erro_servidor }
}

#[tauri::command]
fn trocar_servidor(url: String) -> Result<(), String> {
    let mut c = ler_config();
    c.servidor = url.trim().to_string();
    gravar_config(&c)
}

#[derive(Serialize)]
struct Situacao {
    /// `sem_pasta`, `pasta_sumiu`, `sem_cliente`, `interrompida`, `instalado`.
    estado: &'static str,
    pasta: Option<String>,
    numero: Option<u32>,
    /// Versão atual no servidor (se ele respondeu).
    remota: Option<u32>,
    erro_servidor: Option<String>,
}

/// O que mostrar para a versão escolhida (e grava a escolha).
#[tauri::command]
async fn situacao(versao: String) -> Result<Situacao, String> {
    let mut c = ler_config();
    if c.versao != versao {
        c.versao = versao.clone();
        gravar_config(&c)?;
    }
    let url = c.servidor.clone();
    let v = versao.clone();
    let remota = tauri::async_runtime::spawn_blocking(move || servidor(&url).versao_anunciada(&v).map_err(erro))
        .await
        .map_err(|e| e.to_string())?;
    let (remota, erro_servidor) = match remota {
        Ok(r) => (Some(r.atual), None),
        Err(e) => (None, Some(e)),
    };
    let Some(pasta) = c.pastas.get(&versao).cloned() else {
        return Ok(Situacao { estado: "sem_pasta", pasta: None, numero: None, remota, erro_servidor });
    };
    let p = PathBuf::from(&pasta);
    let (estado, numero) = if !p.exists() {
        ("pasta_sumiu", None)
    } else {
        match cliente::ler_estado(&p).map_err(erro)? {
            Some(e) if e.versao_cliente != versao => ("sem_cliente", None),
            Some(e) if e.numero.is_some() => ("instalado", e.numero),
            Some(_) => ("interrompida", None),
            None => ("sem_cliente", None),
        }
    };
    Ok(Situacao { estado, pasta: Some(pasta), numero, remota, erro_servidor })
}

/// Janela de pastas do Windows.
#[tauri::command]
async fn escolher_pasta(app: AppHandle, titulo: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(titulo)
            .blocking_pick_folder()
            .and_then(|f| f.into_path().ok())
            .map(|p| p.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// Baixa o jogo para a pasta escolhida: nela mesma se estiver vazia, senão numa subpasta
/// `PW Universal <versao>`. A pasta é lembrada antes de começar (para retomar).
#[tauri::command]
async fn baixar(app: AppHandle, versao: String, escolhida: String) -> Result<String, String> {
    let escolhida = PathBuf::from(escolhida);
    let vazia = std::fs::read_dir(&escolhida).map(|mut d| d.next().is_none()).unwrap_or(true);
    let destino = if vazia { escolhida } else { escolhida.join(format!("PW Universal {versao}")) };
    lembrar_pasta(&versao, &destino)?;
    continuar(app, versao).await
}

/// Retoma a instalação, ou atualiza se já está instalado.
#[tauri::command]
async fn continuar(app: AppHandle, versao: String) -> Result<String, String> {
    let pasta = pasta_da_versao(&versao)?;
    let url = ler_config().servidor;
    em_segundo_plano(app, move |ao_progredir| {
        let serv = servidor(&url);
        let instalado = cliente::ler_estado(&pasta)?.is_some_and(|e| e.numero.is_some());
        if instalado {
            Ok(match cliente::atualizar(&serv, &pasta, ao_progredir)? {
                None => "O jogo já está na versão mais nova.".to_string(),
                Some(r) => format!(
                    "Atualizado para a versão {}: {} arquivos novos ou trocados, {} removidos.",
                    r.numero, r.arquivos, r.removidos
                ),
            })
        } else {
            let r = cliente::instalar(&serv, &versao, &pasta, ao_progredir)?;
            Ok(format!("Jogo instalado em {} ({} arquivos).", pasta.display(), r.arquivos))
        }
    })
    .await
}

/// O cliente já está em `escolhida` (ou em `escolhida\element`): lembra a pasta, adota e repara.
#[tauri::command]
async fn usar_existente(app: AppHandle, versao: String, escolhida: String) -> Result<String, String> {
    let escolhida = PathBuf::from(escolhida);
    let tem_cliente = |p: &Path| p.join(cliente::PASTA_ESTADO).exists() || p.join("elementclient.exe").exists();
    let pasta = if tem_cliente(&escolhida) {
        escolhida
    } else if tem_cliente(&escolhida.join("element")) {
        escolhida.join("element")
    } else {
        return Err("Essa pasta não tem o jogo (não achei o elementclient.exe). Escolha a pasta element.".into());
    };
    lembrar_pasta(&versao, &pasta)?;
    let url = ler_config().servidor;
    em_segundo_plano(app, move |ao_progredir| {
        let n = cliente::adotar(&servidor(&url), &versao, &pasta, ao_progredir)?;
        Ok(if n == 0 {
            format!("Pasta {} reconhecida: tudo confere.", pasta.display())
        } else {
            format!("Pasta {} reconhecida: {n} arquivos baixados para deixá-la igual à versão do servidor.", pasta.display())
        })
    })
    .await
}

#[tauri::command]
async fn reparar(app: AppHandle, versao: String) -> Result<String, String> {
    let pasta = pasta_da_versao(&versao)?;
    let url = ler_config().servidor;
    em_segundo_plano(app, move |ao_progredir| {
        let n = cliente::reparar(&servidor(&url), &pasta, ao_progredir)?;
        Ok(if n == 0 {
            "Tudo confere: nenhum arquivo precisou de reparo.".to_string()
        } else {
            format!("{n} arquivos reparados.")
        })
    })
    .await
}

#[tauri::command]
fn jogar(versao: String) -> Result<(), String> {
    cliente::jogar(&pasta_da_versao(&versao)?).map_err(erro)
}

/// Mostra a pasta da versão no Explorer.
#[tauri::command]
fn abrir_no_explorer(versao: String) -> Result<(), String> {
    let pasta = pasta_da_versao(&versao)?;
    std::process::Command::new("explorer").arg(&pasta).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            iniciar,
            trocar_servidor,
            situacao,
            escolher_pasta,
            baixar,
            continuar,
            usar_existente,
            reparar,
            jogar,
            abrir_no_explorer
        ])
        .run(tauri::generate_context!())
        .expect("abrir a janela do launcher");
}
