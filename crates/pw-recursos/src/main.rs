//! `pw-recursos` — linha de comando e serviço (B233).
//!
//! ```text
//! pw-recursos servir                       HTTP /atualizacao/<versao>/… (porta 8100)
//! pw-recursos importar --versao 1.5.5 --cliente <pasta element original, com .pck> [--refazer] [--nivel 12]
//! pw-recursos conferir --origem <http://host:8100 | pasta dados> --versao 1.5.5 --destino <pasta vazia> [--chave-publica <hex>]
//! pw-recursos limpar --versao 1.5.5        apaga objetos que nenhum registro usa (roda sozinho no fim do importar)
//! pw-recursos chave                        mostra a chave pública (cria o par se não existe)
//! ```
//!
//! Ambiente: `PW_RECURSOS_DADOS` (padrão `/dados`), `PW_RECURSOS_PORTA` (8100),
//! `PW_RECURSOS_CHAVE` (`/chave/assinatura_ed25519.key`), `DATABASE_URL`, `PYTHON`,
//! `PW_PCK_EXTRACT`.

use anyhow::{bail, Context, Result};
use pw_recursos::assinatura::Chave;
use pw_atualizador::cliente::{self, Origem, Servidor};
use pw_recursos::fonte::ExtratorPython;
use pw_recursos::importar::{self, Importacao};
use pw_storage::{PostgresPool, RecursosRepository, StorageConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

fn argumentos(lista: &[String]) -> Result<(HashMap<String, String>, Vec<String>)> {
    let mut valores = HashMap::new();
    let mut chaves = Vec::new();
    let mut i = 0;
    while i < lista.len() {
        let a = &lista[i];
        let Some(nome) = a.strip_prefix("--") else { bail!("argumento inesperado: {a}") };
        if nome == "refazer" {
            chaves.push(nome.to_string());
            i += 1;
            continue;
        }
        let v = lista.get(i + 1).with_context(|| format!("--{nome} sem valor"))?;
        valores.insert(nome.to_string(), v.clone());
        i += 2;
    }
    Ok((valores, chaves))
}

fn ambiente(nome: &str, padrao: &str) -> String {
    std::env::var(nome).unwrap_or_else(|_| padrao.to_string())
}

/// Apaga do volume os objetos da versão que nenhum registro usa mais (o que saiu numa
/// reimportação ou numa exclusão) — B233, pedido do Murillo: nada inútil no servidor.
async fn limpar(repo: &RecursosRepository, dados: &std::path::Path, versao: &str) -> Result<(u64, u64)> {
    let em_uso: std::collections::HashSet<[u8; 32]> = repo
        .objetos_em_uso(versao)
        .await?
        .into_iter()
        .filter_map(|v| v.try_into().ok())
        .collect();
    if em_uso.is_empty() {
        bail!("a versão {versao} não tem nenhum registro no banco: limpar apagaria tudo");
    }
    let objetos = pw_recursos::objetos::Objetos::new(importar::pasta_da_versao(dados, versao).join("objetos"), 0);
    Ok(tokio::task::spawn_blocking(move || objetos.limpar(&em_uso)).await??)
}

fn caminho_da_chave() -> PathBuf {
    ambiente("PW_RECURSOS_CHAVE", "/chave/assinatura_ed25519.key").into()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();
    let todos: Vec<String> = std::env::args().skip(1).collect();
    let comando = todos.first().map(String::as_str).unwrap_or("servir");
    let (args, chaves) = argumentos(todos.get(1..).unwrap_or(&[]))?;
    let dados: PathBuf = args
        .get("dados")
        .cloned()
        .unwrap_or_else(|| ambiente("PW_RECURSOS_DADOS", "/dados"))
        .into();
    match comando {
        "servir" => {
            let porta = ambiente("PW_RECURSOS_PORTA", "8100");
            let escuta = tokio::net::TcpListener::bind(format!("0.0.0.0:{porta}")).await?;
            tracing::info!("pw-recursos servindo {} em :{porta}", dados.display());
            axum::serve(escuta, pw_recursos::servir::rotas(dados)).await?;
        }
        "chave" => {
            let (chave, criada) = Chave::carregar_ou_criar(&caminho_da_chave())?;
            if criada {
                eprintln!("chave nova criada em {}", caminho_da_chave().display());
            }
            println!("{}", chave.publica_hex());
        }
        "importar" => {
            let versao = args.get("versao").context("--versao obrigatório")?.clone();
            importar::validar_versao(&versao)?;
            let cliente: PathBuf = args.get("cliente").context("--cliente obrigatório")?.into();
            let nivel: i32 = args.get("nivel").map(|n| n.parse()).transpose()?.unwrap_or(12);
            let refazer = chaves.iter().any(|c| c == "refazer");
            let pool = PostgresPool::new(&StorageConfig::default()).await?;
            let repo = RecursosRepository::new(pool);
            if !repo.versoes(&versao).await?.is_empty() {
                if !refazer {
                    bail!("a versão {versao} já tem a versão 0 registrada (use --refazer)");
                }
                if !repo.apagar_versao_zero(&versao).await? {
                    bail!("a versão {versao} já tem versões publicadas ou alterações: não dá para refazer a 0");
                }
            }
            let atualizacao = importar::pasta_da_versao(&dados, &versao).join("atualizacao");
            if refazer && atualizacao.exists() {
                std::fs::remove_dir_all(&atualizacao)?;
            }
            let (chave, criada) = Chave::carregar_ou_criar(&caminho_da_chave())?;
            if criada {
                tracing::warn!("chave de assinatura criada agora em {}", caminho_da_chave().display());
            }
            let chave = Arc::new(chave);
            let (v, c, d) = (versao.clone(), cliente.clone(), dados.clone());
            let resultado = tokio::task::spawn_blocking(move || {
                let fonte = ExtratorPython::do_ambiente();
                importar::importar(&Importacao {
                    versao: &v,
                    cliente: &c,
                    dados: &d,
                    nivel,
                    linha_de_grupo: importar::precisa_linha_de_grupo(&v),
                    fonte: &fonte,
                    chave: &chave,
                })
            })
            .await??;
            repo.gravar_versao_zero(
                &versao,
                &cliente.display().to_string(),
                &resultado.manifesto_sha256,
                &resultado.assinatura,
                &pw_recursos::para_o_banco(&resultado.registros),
            )
            .await?;
            let r = &resultado.relatorio;
            let mb = |b: u64| b as f64 / 1_048_576.0;
            println!(
                "versão 0 de {versao}: {} arquivos, {:.1} MB; pacotes .pck lidos {}, falhas {}, repetidos entre pacotes {}",
                r.total_arquivos,
                mb(r.total_bytes),
                r.pacotes_lidos.len(),
                r.falhas_de_pacote.len(),
                r.repetidos_entre_pacotes.len()
            );
            println!(
                "soltos lidos {}, soltos repetidos do pacote {}, fora do GBK {}, excluídos {}, nossos {:?}",
                r.soltos_lidos,
                r.soltos_repetidos_do_pacote,
                r.fora_do_gbk.len(),
                r.excluidos.values().sum::<u64>(),
                r.nossos
            );
            let total: u64 = r.pacotes.iter().map(|p| p.tamanho).sum();
            println!("pacotes de instalação: {} somando {:.1} MB; tempos {:?}", r.pacotes.len(), mb(total), r.segundos);
            println!("manifesto {}", hex::encode(resultado.manifesto_sha256));
            let (n, bytes) = limpar(&repo, &dados, &versao).await?;
            println!("objetos sem uso apagados: {n} ({:.1} MB)", mb(bytes));
        }
        "limpar" => {
            let versao = args.get("versao").context("--versao obrigatório")?.clone();
            importar::validar_versao(&versao)?;
            let repo = RecursosRepository::new(PostgresPool::new(&StorageConfig::default()).await?);
            let (n, bytes) = limpar(&repo, &dados, &versao).await?;
            println!("objetos sem uso apagados em {versao}: {n} ({:.1} MB)", bytes as f64 / 1_048_576.0);
        }
        "conferir" => {
            let versao = args.get("versao").context("--versao obrigatório")?;
            let origem = Origem::de_texto(args.get("origem").context("--origem obrigatório")?);
            let destino: PathBuf = args.get("destino").context("--destino obrigatório")?.into();
            let publica = match args.get("chave-publica") {
                Some(p) => p.clone(),
                None => Chave::carregar_ou_criar(&caminho_da_chave())?.0.publica_hex(),
            };
            // A prova é a própria instalação do launcher numa pasta vazia (um caminho só, B234).
            let http = matches!(origem, Origem::Http(_));
            let serv = Servidor::new(origem, publica);
            let c = cliente::instalar(&serv, versao, &destino, &mut |_| {})?;
            if http {
                cliente::conferir_range(&serv, versao)?;
            }
            println!(
                "conferido: {} pacotes, {} arquivos, {:.1} MB extraídos em {}; Range {}",
                c.pacotes,
                c.arquivos,
                c.bytes as f64 / 1_048_576.0,
                destino.display(),
                if http { "ok (206)" } else { "não testado (origem é pasta)" }
            );
        }
        outro => bail!("comando desconhecido: {outro} (servir, importar, conferir, limpar, chave)"),
    }
    Ok(())
}
