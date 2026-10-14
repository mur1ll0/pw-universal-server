//! Launcher do pw-universal (P2, B234). A lógica é toda do `pw-atualizador`; aqui ficam a janela
//! (Tauri, `ui/index.html`), a configuração ao lado do `.exe` (`pw-launcher.json`) e a chave
//! pública embutida (`chave_publica.txt`, a do `pw-recursos` que publica as versões).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pw_atualizador::cliente::{self, Origem, Progresso, Servidor};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const CHAVE_PUBLICA: &str = include_str!("../chave_publica.txt");
const SERVIDOR_PADRAO: &str = "http://127.0.0.1:8100";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Config {
    servidor: String,
    versao: String,
    pasta: String,
}

fn caminho_config() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .unwrap_or_default()
        .join("pw-launcher.json")
}

#[tauri::command]
fn carregar_config() -> Config {
    let mut c: Config = std::fs::read(caminho_config())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if c.servidor.is_empty() {
        c.servidor = SERVIDOR_PADRAO.into();
    }
    c
}

#[tauri::command]
fn salvar_config(config: Config) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(caminho_config(), bytes).map_err(|e| e.to_string())
}

fn servidor(url: &str) -> Servidor {
    Servidor::new(Origem::Http(url.trim().trim_end_matches('/').to_string()), CHAVE_PUBLICA.trim())
}

fn erro(e: anyhow::Error) -> String {
    format!("{e:#}")
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

#[tauri::command]
async fn versoes(servidor_url: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || servidor(&servidor_url).versoes().map_err(erro))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct EstadoDaPasta {
    versao_cliente: String,
    numero: Option<u32>,
}

#[tauri::command]
fn estado(pasta: String) -> Result<Option<EstadoDaPasta>, String> {
    Ok(cliente::ler_estado(Path::new(&pasta))
        .map_err(erro)?
        .map(|e| EstadoDaPasta { versao_cliente: e.versao_cliente, numero: e.numero }))
}

/// Instala (ou retoma a instalação) se a pasta não tem o cliente; senão atualiza.
#[tauri::command]
async fn instalar_ou_atualizar(app: AppHandle, servidor_url: String, versao: String, pasta: String) -> Result<String, String> {
    em_segundo_plano(app, move |ao_progredir| {
        let serv = servidor(&servidor_url);
        let pasta = PathBuf::from(&pasta);
        let instalado = cliente::ler_estado(&pasta)?.is_some_and(|e| e.numero.is_some());
        if instalado {
            Ok(match cliente::atualizar(&serv, &pasta, ao_progredir)? {
                None => "O cliente já está na versão mais nova.".to_string(),
                Some(r) => format!(
                    "Atualizado para a versão {}: {} arquivos novos ou trocados, {} removidos.",
                    r.numero, r.arquivos, r.removidos
                ),
            })
        } else {
            let r = cliente::instalar(&serv, &versao, &pasta, ao_progredir)?;
            Ok(format!(
                "Instalado: versão {}, {} arquivos ({:.1} GB).",
                r.numero,
                r.arquivos,
                r.bytes as f64 / 1_073_741_824.0
            ))
        }
    })
    .await
}

#[tauri::command]
async fn reparar(app: AppHandle, servidor_url: String, pasta: String) -> Result<String, String> {
    em_segundo_plano(app, move |ao_progredir| {
        let n = cliente::reparar(&servidor(&servidor_url), Path::new(&pasta), ao_progredir)?;
        Ok(if n == 0 {
            "Tudo confere: nenhum arquivo precisou de reparo.".to_string()
        } else {
            format!("{n} arquivos reparados.")
        })
    })
    .await
}

#[tauri::command]
fn jogar(pasta: String) -> Result<(), String> {
    cliente::jogar(Path::new(&pasta)).map_err(erro)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            carregar_config,
            salvar_config,
            versoes,
            estado,
            instalar_ou_atualizar,
            reparar,
            jogar
        ])
        .run(tauri::generate_context!())
        .expect("abrir a janela do launcher");
}
