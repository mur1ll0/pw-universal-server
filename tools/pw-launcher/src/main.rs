//! Launcher do pw-universal (P2, B234; usabilidade no B235; Win32 nativo desde o B275 para rodar
//! no Windows 7). A lógica é toda do `pw-atualizador`; aqui ficam a configuração ao lado do `.exe`
//! (`pw-launcher.json`: servidor, versão escolhida e **uma pasta por versão**), as operações que a
//! janela chama (`acoes`) e a chave pública embutida (`chave_publica.txt`). A janela é `janela.rs`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod janela;

use pw_atualizador::cliente::{self, Origem, Progresso, Servidor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

/// Operações da janela. As que falam com o servidor rodam fora da thread da janela
/// (`janela::em_segundo_plano`); as demais são rápidas e rodam nela.
pub mod acoes {
    use super::*;

    pub type AoProgredir<'a> = &'a mut dyn FnMut(Progresso);

    pub struct Inicio {
        pub servidor: String,
        pub versao: String,
        pub versoes: Vec<String>,
        pub erro_servidor: Option<String>,
    }

    /// Ao abrir: a configuração e as versões que o servidor oferece.
    pub fn iniciar() -> Inicio {
        let c = ler_config();
        let (versoes, erro_servidor) = match servidor(&c.servidor).versoes().map_err(erro) {
            Ok(v) => (v, None),
            Err(e) => (vec![], Some(e)),
        };
        Inicio { servidor: c.servidor, versao: c.versao, versoes, erro_servidor }
    }

    pub fn trocar_servidor(url: &str) -> Result<(), String> {
        let mut c = ler_config();
        c.servidor = url.trim().to_string();
        gravar_config(&c)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Estado {
        SemPasta,
        PastaSumiu,
        SemCliente,
        Interrompida,
        Instalado,
    }

    pub struct Situacao {
        pub estado: Estado,
        pub pasta: Option<String>,
        pub numero: Option<u32>,
        /// Versão atual no servidor (se ele respondeu).
        pub remota: Option<u32>,
        pub erro_servidor: Option<String>,
    }

    /// O que mostrar para a versão escolhida (e grava a escolha).
    pub fn situacao(versao: &str) -> Result<Situacao, String> {
        let mut c = ler_config();
        if c.versao != versao {
            c.versao = versao.into();
            gravar_config(&c)?;
        }
        let (remota, erro_servidor) = match servidor(&c.servidor).versao_anunciada(versao).map_err(erro) {
            Ok(r) => (Some(r.atual), None),
            Err(e) => (None, Some(e)),
        };
        let Some(pasta) = c.pastas.get(versao).cloned() else {
            return Ok(Situacao { estado: Estado::SemPasta, pasta: None, numero: None, remota, erro_servidor });
        };
        let p = PathBuf::from(&pasta);
        let (estado, numero) = if !p.exists() {
            (Estado::PastaSumiu, None)
        } else {
            match cliente::ler_estado(&p).map_err(erro)? {
                Some(e) if e.versao_cliente != versao => (Estado::SemCliente, None),
                Some(e) if e.numero.is_some() => (Estado::Instalado, e.numero),
                Some(_) => (Estado::Interrompida, None),
                None => (Estado::SemCliente, None),
            }
        };
        Ok(Situacao { estado, pasta: Some(pasta), numero, remota, erro_servidor })
    }

    /// Baixa o jogo para a pasta escolhida: nela mesma se estiver vazia, senão numa subpasta
    /// `PW Universal <versao>`. A pasta é lembrada antes de começar (para retomar).
    pub fn baixar(versao: &str, escolhida: &Path, ao_progredir: AoProgredir) -> Result<String, String> {
        let vazia = std::fs::read_dir(escolhida).map(|mut d| d.next().is_none()).unwrap_or(true);
        let destino = if vazia { escolhida.to_path_buf() } else { escolhida.join(format!("PW Universal {versao}")) };
        lembrar_pasta(versao, &destino)?;
        continuar(versao, ao_progredir)
    }

    /// Retoma a instalação, ou atualiza se já está instalado.
    pub fn continuar(versao: &str, ao_progredir: AoProgredir) -> Result<String, String> {
        let pasta = pasta_da_versao(versao)?;
        let serv = servidor(&ler_config().servidor);
        let instalado = cliente::ler_estado(&pasta).map_err(erro)?.is_some_and(|e| e.numero.is_some());
        if instalado {
            Ok(match cliente::atualizar(&serv, &pasta, ao_progredir).map_err(erro)? {
                None => "O jogo já está na versão mais nova.".to_string(),
                Some(r) => format!(
                    "Atualizado para a versão {}: {} arquivos novos ou trocados, {} removidos.",
                    r.numero, r.arquivos, r.removidos
                ),
            })
        } else {
            let r = cliente::instalar(&serv, versao, &pasta, ao_progredir).map_err(erro)?;
            Ok(format!("Jogo instalado em {} ({} arquivos).", pasta.display(), r.arquivos))
        }
    }

    /// O cliente já está em `escolhida` (ou em `escolhida\element`): lembra a pasta, adota e repara.
    pub fn usar_existente(versao: &str, escolhida: &Path, ao_progredir: AoProgredir) -> Result<String, String> {
        let tem_cliente = |p: &Path| p.join(cliente::PASTA_ESTADO).exists() || p.join("elementclient.exe").exists();
        let pasta = if tem_cliente(escolhida) {
            escolhida.to_path_buf()
        } else if tem_cliente(&escolhida.join("element")) {
            escolhida.join("element")
        } else {
            return Err("Essa pasta não tem o jogo (não achei o elementclient.exe). Escolha a pasta element.".into());
        };
        lembrar_pasta(versao, &pasta)?;
        let n = cliente::adotar(&servidor(&ler_config().servidor), versao, &pasta, ao_progredir).map_err(erro)?;
        Ok(if n == 0 {
            format!("Pasta {} reconhecida: tudo confere.", pasta.display())
        } else {
            format!("Pasta {} reconhecida: {n} arquivos baixados para deixá-la igual à versão do servidor.", pasta.display())
        })
    }

    pub fn reparar(versao: &str, ao_progredir: AoProgredir) -> Result<String, String> {
        let pasta = pasta_da_versao(versao)?;
        let n = cliente::reparar(&servidor(&ler_config().servidor), &pasta, ao_progredir).map_err(erro)?;
        Ok(if n == 0 {
            "Tudo confere: nenhum arquivo precisou de reparo.".to_string()
        } else {
            format!("{n} arquivos reparados.")
        })
    }

    pub fn jogar(versao: &str) -> Result<(), String> {
        cliente::jogar(&pasta_da_versao(versao)?).map_err(erro)
    }

    /// Mostra a pasta da versão no Explorer.
    pub fn abrir_no_explorer(versao: &str) -> Result<(), String> {
        let pasta = pasta_da_versao(versao)?;
        std::process::Command::new("explorer").arg(&pasta).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn main() {
    janela::executar();
}
