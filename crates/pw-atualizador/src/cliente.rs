//! Instalar, verificar, reparar, atualizar e jogar (P2, B234).
//!
//! Na pasta do cliente o launcher guarda só `.pw-atualizacao/`:
//!
//! ```text
//! .pw-atualizacao/estado.json      versão do cliente, número aplicado (ou alvo de uma instalação em curso)
//! .pw-atualizacao/manifesto.json   o manifesto aplicado (bytes exatos, assinatura conferida)
//! .pw-atualizacao/<pacote>.parcial download em curso (retomado com Range)
//! .pw-atualizacao/objetos/<sha>    arquivos de uma atualização já baixados e conferidos
//! ```
//!
//! Tudo retoma de onde parou: pacote baixado não é baixado de novo, pacote extraído fica marcado no
//! estado, objeto de atualização baixado fica na pasta até a atualização terminar.

use crate::assinatura;
use crate::manifesto::{Arquivo, Manifesto, Versao};
use crate::nomes;
use crate::tar_gnu;
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

pub const PASTA_ESTADO: &str = ".pw-atualizacao";

fn sha256_hex(dados: &[u8]) -> String {
    hex::encode(Sha256::digest(dados))
}

/// Etapa e andamento, para a barra de progresso do launcher.
#[derive(Debug, Clone, Serialize)]
pub struct Progresso {
    pub etapa: String,
    pub feito: u64,
    pub total: u64,
}

pub type AoProgredir<'a> = &'a mut dyn FnMut(Progresso);

pub enum Origem {
    /// `http://host:8100` — o `pw-recursos`.
    Http(String),
    /// Raiz do volume (`data/clientes`), para testes e para a prova no próprio servidor.
    Pasta(PathBuf),
}

impl Origem {
    pub fn de_texto(t: &str) -> Self {
        if t.starts_with("http://") || t.starts_with("https://") {
            Origem::Http(t.trim_end_matches('/').to_string())
        } else {
            Origem::Pasta(t.into())
        }
    }

    fn url(&self, versao: &str, rel: &str) -> String {
        match self {
            Origem::Http(base) => format!("{base}/atualizacao/{versao}/{rel}"),
            Origem::Pasta(raiz) => raiz.join(versao).join("atualizacao").join(rel).display().to_string(),
        }
    }

    /// Abre `rel` a partir do byte `desde`. Devolve o leitor e se ele começa mesmo em `desde`
    /// (`false`: o servidor ignorou o Range e mandou desde o início).
    fn abrir_desde(&self, versao: &str, rel: &str, desde: u64) -> Result<(Box<dyn Read>, bool)> {
        match self {
            Origem::Http(_) => {
                let url = self.url(versao, rel);
                let mut pedido = ureq::get(&url);
                if desde > 0 {
                    pedido = pedido.set("Range", &format!("bytes={desde}-"));
                }
                let r = pedido.call().with_context(|| format!("GET {url}"))?;
                let continua = desde > 0 && r.status() == 206;
                Ok((Box::new(r.into_reader()), continua || desde == 0))
            }
            Origem::Pasta(raiz) => {
                let p = raiz.join(versao).join("atualizacao").join(rel);
                let mut f = fs::File::open(&p).with_context(|| format!("abrir {}", p.display()))?;
                f.seek(io::SeekFrom::Start(desde))?;
                Ok((Box::new(f), true))
            }
        }
    }

    fn ler(&self, versao: &str, rel: &str) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        self.abrir_desde(versao, rel, 0)?.0.read_to_end(&mut v)?;
        Ok(v)
    }
}

/// O servidor e a chave pública que o launcher confia.
pub struct Servidor {
    pub origem: Origem,
    pub chave_publica: String,
}

/// O que veio do servidor para uma versão: tudo conferido.
pub struct VersaoRemota {
    pub versao: Versao,
    pub manifesto_bytes: Vec<u8>,
    pub manifesto: Manifesto,
    /// Pasta do manifesto relativa a `atualizacao/<versao>/` (`"0"`, `"1"`…).
    pub base: String,
}

impl Servidor {
    pub fn new(origem: Origem, chave_publica: impl Into<String>) -> Self {
        Self { origem, chave_publica: chave_publica.into() }
    }

    /// Versões de cliente que o servidor oferece (`GET /versoes`).
    pub fn versoes(&self) -> Result<Vec<String>> {
        match &self.origem {
            Origem::Http(base) => {
                let url = format!("{base}/versoes");
                let texto = ureq::get(&url).call().with_context(|| format!("GET {url}"))?.into_string()?;
                Ok(serde_json::from_str(&texto)?)
            }
            Origem::Pasta(raiz) => {
                let mut v: Vec<String> = fs::read_dir(raiz)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().join("atualizacao").join("versao.json").exists())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect();
                v.sort();
                Ok(v)
            }
        }
    }

    fn assinatura_de(&self, versao: &str, rel: &str) -> Result<Vec<u8>> {
        let texto = self.origem.ler(versao, &format!("{rel}.sig"))?;
        hex::decode(String::from_utf8_lossy(&texto).trim()).context("assinatura não está em hex")
    }

    /// `versao.json` e o manifesto atual, com as duas assinaturas e o SHA-256 conferidos.
    pub fn versao_remota(&self, versao: &str) -> Result<VersaoRemota> {
        let v_bytes = self.origem.ler(versao, "versao.json")?;
        assinatura::verificar(&self.chave_publica, &v_bytes, &self.assinatura_de(versao, "versao.json")?)
            .context("versao.json")?;
        let v: Versao = serde_json::from_slice(&v_bytes)?;
        if v.versao_cliente != versao {
            bail!("o servidor respondeu a versão {} quando se pediu {versao}", v.versao_cliente);
        }
        let m_bytes = self.origem.ler(versao, &v.manifesto)?;
        if sha256_hex(&m_bytes) != v.manifesto_sha256 {
            bail!("SHA-256 do manifesto não bate com o versao.json");
        }
        assinatura::verificar(&self.chave_publica, &m_bytes, &self.assinatura_de(versao, &v.manifesto)?)
            .context("manifesto.json")?;
        let manifesto: Manifesto = serde_json::from_slice(&m_bytes)?;
        if manifesto.numero != v.atual || manifesto.versao_cliente != versao {
            bail!("o manifesto não é o da versão anunciada");
        }
        let base = v.manifesto.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
        Ok(VersaoRemota { versao: v, manifesto_bytes: m_bytes, manifesto, base })
    }

    /// Só o `versao.json` (assinatura conferida): a consulta leve que o launcher faz ao abrir para
    /// saber se há atualização, sem baixar o manifesto (25 MB no 1.5.5).
    pub fn versao_anunciada(&self, versao: &str) -> Result<Versao> {
        let v_bytes = self.origem.ler(versao, "versao.json")?;
        assinatura::verificar(&self.chave_publica, &v_bytes, &self.assinatura_de(versao, "versao.json")?)
            .context("versao.json")?;
        let v: Versao = serde_json::from_slice(&v_bytes)?;
        if v.versao_cliente != versao {
            bail!("o servidor respondeu a versão {} quando se pediu {versao}", v.versao_cliente);
        }
        Ok(v)
    }

    /// Um arquivo pelo SHA-256 (`objetos/<sha>`, zstd), conferido.
    pub fn objeto(&self, versao: &str, sha: &str, tamanho: u64) -> Result<Vec<u8>> {
        let comprimido = self.origem.ler(versao, &format!("objetos/{sha}"))?;
        let dados = zstd::bulk::decompress(&comprimido, (tamanho as usize).max(1))?;
        if dados.len() as u64 != tamanho || sha256_hex(&dados) != sha {
            bail!("objeto {sha} não confere");
        }
        Ok(dados)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Estado {
    pub formato: u32,
    pub versao_cliente: String,
    /// Número aplicado por inteiro; `None` enquanto a instalação não termina.
    pub numero: Option<u32>,
    /// Manifesto em instalação/aplicado.
    pub manifesto_sha256: String,
    /// Pacotes já extraídos e conferidos de uma instalação em curso.
    #[serde(default)]
    pub pacotes_prontos: Vec<String>,
}

fn pasta_estado(pasta: &Path) -> PathBuf {
    pasta.join(PASTA_ESTADO)
}

pub fn ler_estado(pasta: &Path) -> Result<Option<Estado>> {
    let p = pasta_estado(pasta).join("estado.json");
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(&p)?).with_context(|| format!("ler {}", p.display()))?))
}

fn gravar_estado(pasta: &Path, e: &Estado) -> Result<()> {
    let dir = pasta_estado(pasta);
    fs::create_dir_all(&dir)?;
    gravar_atomico(&dir.join("estado.json"), &serde_json::to_vec_pretty(e)?)
}

fn gravar_atomico(destino: &Path, dados: &[u8]) -> Result<()> {
    if let Some(p) = destino.parent() {
        fs::create_dir_all(p)?;
    }
    let mut tmp = destino.as_os_str().to_owned();
    tmp.push(".pw-tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, dados).with_context(|| format!("gravar {}", tmp.display()))?;
    fs::rename(&tmp, destino).with_context(|| format!("gravar {}", destino.display()))?;
    Ok(())
}

/// O manifesto aplicado na pasta (o que a verificação e a atualização comparam).
pub fn manifesto_local(pasta: &Path) -> Result<Manifesto> {
    let p = pasta_estado(pasta).join("manifesto.json");
    serde_json::from_slice(&fs::read(&p).with_context(|| format!("ler {}", p.display()))?)
        .context("manifesto local")
}

/// Caminho no disco (desta máquina) de um arquivo do manifesto; recusa a pasta de estado.
fn no_disco(pasta: &Path, caminho_gbk_hex: &str) -> Result<PathBuf> {
    let gbk = hex::decode(caminho_gbk_hex).context("caminho_gbk")?;
    let rel = nomes::nome_no_disco(&gbk)?;
    if rel
        .components()
        .next()
        .is_some_and(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case(PASTA_ESTADO))
    {
        bail!("o manifesto aponta para a pasta de estado do launcher");
    }
    Ok(pasta.join(rel))
}

/// Recusa instalar se a página de código deste Windows não converte algum nome (§7.7).
fn conferir_nomes(pasta: &Path, m: &Manifesto) -> Result<()> {
    let ruins: Vec<String> = m
        .arquivos
        .iter()
        .filter(|a| no_disco(pasta, &a.caminho_gbk).is_err())
        .map(|a| nomes::legivel(&hex::decode(&a.caminho_gbk).unwrap_or_default()))
        .collect();
    if !ruins.is_empty() {
        bail!(
            "{} nomes de arquivo não têm conversão na página de código deste Windows (ex.: {}); \
             o cliente não conseguiria abri-los",
            ruins.len(),
            ruins[0]
        );
    }
    Ok(())
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Resumo {
    pub numero: u32,
    pub pacotes: u64,
    pub arquivos: u64,
    pub bytes: u64,
    pub removidos: u64,
    /// Arquivos do pacote que a versão não usa mais (ou trocou): não gravados.
    pub pulados: u64,
    /// Arquivos baixados um a um (os trocados depois da versão do pacote).
    pub por_objeto: u64,
}

fn rel_do_pacote(base: &str, p: &crate::manifesto::Pacote) -> String {
    match p.numero {
        Some(n) => format!("{n}/{}", p.arquivo),
        None => format!("{base}/{}", p.arquivo),
    }
}

#[allow(clippy::too_many_arguments)]
/// Baixa o pacote para `.pw-atualizacao/<nome>.parcial`, retomando com Range, e confere o hash.
fn baixar_pacote(
    serv: &Servidor,
    versao: &str,
    rel: &str,
    destino: &Path,
    tamanho: u64,
    sha: &str,
    ja_feito: u64,
    ao_progredir: AoProgredir,
    total: u64,
) -> Result<()> {
    let mut atual = destino.metadata().map(|m| m.len()).unwrap_or(0);
    if atual > tamanho {
        fs::remove_file(destino)?;
        atual = 0;
    }
    if atual < tamanho {
        let (mut r, continua) = serv.origem.abrir_desde(versao, rel, atual)?;
        let mut f = if continua && atual > 0 {
            fs::OpenOptions::new().append(true).open(destino)?
        } else {
            atual = 0;
            fs::File::create(destino)?
        };
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            f.write_all(&buf[..n])?;
            atual += n as u64;
            ao_progredir(Progresso { etapa: format!("baixando {rel}"), feito: ja_feito + atual, total });
        }
        f.sync_all()?;
    }
    let mut hash = Sha256::new();
    let n = io::copy(&mut fs::File::open(destino)?, &mut hash)?;
    if n != tamanho || hex::encode(hash.finalize()) != sha {
        fs::remove_file(destino)?;
        bail!("o pacote {rel} chegou diferente do manifesto (apagado; tente de novo)");
    }
    Ok(())
}

/// Extrai um pacote baixado conferindo cada arquivo contra o manifesto.
fn extrair_pacote(
    pacote: &Path,
    nome_pacote: &str,
    pasta: &Path,
    esperados: &HashMap<Vec<u8>, &Arquivo>,
    resumo: &mut Resumo,
) -> Result<()> {
    let zst = zstd::stream::Decoder::new(fs::File::open(pacote)?)?;
    let mut tar = tar_gnu::Leitor::new(zst);
    while let Some((nome, dados)) = tar.proximo().with_context(|| format!("ler o pacote {nome_pacote}"))? {
        // O pacote já teve o SHA-256 conferido contra o manifesto assinado. Numa versão N que
        // reaproveita os pacotes da 0, um arquivo removido ou trocado depois vem no pacote com o
        // conteúdo antigo: fica de fora aqui e o que faltar vem por objeto no fim da instalação.
        let Some(a) = esperados.get(&nome) else {
            resumo.pulados += 1;
            continue;
        };
        if a.tamanho != dados.len() as u64 || sha256_hex(&dados) != a.sha256 {
            resumo.pulados += 1;
            continue;
        }
        gravar_atomico(&no_disco(pasta, &a.caminho_gbk)?, &dados)?;
        resumo.arquivos += 1;
        resumo.bytes += dados.len() as u64;
    }
    Ok(())
}

/// Instala a versão atual de `versao` numa pasta vazia (ou retoma uma instalação interrompida).
pub fn instalar(serv: &Servidor, versao: &str, pasta: &Path, ao_progredir: AoProgredir) -> Result<Resumo> {
    let estado = ler_estado(pasta)?;
    match &estado {
        Some(e) if e.versao_cliente != versao => {
            bail!("a pasta tem o cliente {}, não o {versao}", e.versao_cliente)
        }
        Some(e) if e.numero.is_some() => bail!("o cliente já está instalado nesta pasta (use atualizar)"),
        None if pasta.exists() && fs::read_dir(pasta)?.next().is_some() => {
            bail!("a pasta {} não está vazia", pasta.display())
        }
        _ => {}
    }
    ao_progredir(Progresso { etapa: "conferindo a versão no servidor".into(), feito: 0, total: 0 });
    let remota = serv.versao_remota(versao)?;
    conferir_nomes(pasta, &remota.manifesto)?;
    let manifesto_sha256 = remota.versao.manifesto_sha256.clone();
    let mut e = match estado {
        // Mesma instalação: retoma; se o servidor publicou outra versão no meio, recomeça.
        Some(e) if e.manifesto_sha256 == manifesto_sha256 => e,
        _ => Estado { formato: 1, versao_cliente: versao.into(), numero: None, manifesto_sha256, pacotes_prontos: vec![] },
    };
    gravar_estado(pasta, &e)?;
    let esperados: HashMap<Vec<u8>, &Arquivo> = remota
        .manifesto
        .arquivos
        .iter()
        .map(|a| Ok((hex::decode(&a.caminho_gbk)?, a)))
        .collect::<Result<_>>()?;
    let total: u64 = remota.manifesto.pacotes.iter().map(|p| p.tamanho).sum();
    let mut feito: u64 = remota
        .manifesto
        .pacotes
        .iter()
        .filter(|p| e.pacotes_prontos.contains(&p.nome))
        .map(|p| p.tamanho)
        .sum();
    let mut resumo = Resumo { numero: remota.versao.atual, ..Default::default() };
    for p in &remota.manifesto.pacotes {
        if e.pacotes_prontos.contains(&p.nome) {
            continue;
        }
        let rel = rel_do_pacote(&remota.base, p);
        let parcial = pasta_estado(pasta).join(format!("{}.parcial", p.nome));
        baixar_pacote(serv, versao, &rel, &parcial, p.tamanho, &p.sha256, feito, ao_progredir, total)?;
        ao_progredir(Progresso { etapa: format!("extraindo {}", p.nome), feito: feito + p.tamanho, total });
        extrair_pacote(&parcial, &p.nome, pasta, &esperados, &mut resumo)?;
        fs::remove_file(&parcial)?;
        feito += p.tamanho;
        resumo.pacotes += 1;
        e.pacotes_prontos.push(p.nome.clone());
        gravar_estado(pasta, &e)?;
    }
    // O que os pacotes não trouxeram (trocado ou acrescentado depois da versão do pacote) vem por
    // objeto; numa instalação retomada, confere também o que foi extraído antes.
    let faltam = verificar_com(pasta, &remota.manifesto, false, &mut |_| {})?;
    let por_caminho: HashMap<&str, &Arquivo> =
        remota.manifesto.arquivos.iter().map(|a| (a.caminho_gbk.as_str(), a)).collect();
    for (i, d) in faltam.iter().enumerate() {
        let a = por_caminho[d.caminho_gbk.as_str()];
        ao_progredir(Progresso { etapa: format!("baixando {}", d.caminho), feito: i as u64, total: faltam.len() as u64 });
        let dados = serv.objeto(versao, &a.sha256, a.tamanho)?;
        gravar_atomico(&no_disco(pasta, &a.caminho_gbk)?, &dados)?;
        resumo.por_objeto += 1;
        resumo.arquivos += 1;
    }
    let faltam = verificar_com(pasta, &remota.manifesto, false, &mut |_| {})?;
    if !faltam.is_empty() {
        bail!("{} arquivos faltando ou diferentes depois da instalação", faltam.len());
    }
    gravar_atomico(&pasta_estado(pasta).join("manifesto.json"), &remota.manifesto_bytes)?;
    e.numero = Some(remota.versao.atual);
    e.pacotes_prontos.clear();
    gravar_estado(pasta, &e)?;
    ao_progredir(Progresso { etapa: "instalado".into(), feito: total, total });
    Ok(resumo)
}

/// Arquivo do jogador (B280): tudo sob `userdata\`. O próprio cliente reescreve esses arquivos
/// (opções em `systemsettings.ini` por `CECConfigs::SaveSystemSettings`, `EC_Configs.cpp:480-577`;
/// servidor escolhido em `currentserver.ini`, `EC_ServerList.cpp:525-536`; layout por personagem)
/// e o launcher ajusta o endereço da `serverlist.txt` e as opções de vídeo. Por isso
/// `verificar`/`reparar` só repõem o que **faltar**: conteúdo diferente é do jogador, não estrago.
pub fn do_jogador(legivel: &str) -> bool {
    legivel.len() >= 9 && legivel.is_char_boundary(9) && legivel[..9].eq_ignore_ascii_case("userdata\\")
}

/// A `serverlist.txt` é do jogador para `verificar` (o launcher troca o endereço), mas segue o
/// servidor ao `atualizar`: uma lista nova publicada substitui a local e o launcher reaplica o
/// endereço depois. Os demais arquivos do jogador só são trocados se ainda forem os da versão antiga.
pub fn segue_o_servidor(legivel: &str) -> bool {
    legivel.eq_ignore_ascii_case("userdata\\server\\serverlist.txt")
}

/// Arquivo do jogador que ele já mudou (difere do `antigo`, ou não havia antigo): `atualizar`
/// não grava por cima nem apaga.
fn mudado_pelo_jogador(pasta: &Path, a: &Arquivo, antigo: Option<&Arquivo>) -> bool {
    let legivel = nomes::legivel(&hex::decode(&a.caminho_gbk).unwrap_or_default());
    if !do_jogador(&legivel) || segue_o_servidor(&legivel) {
        return false;
    }
    let Ok(p) = no_disco(pasta, &a.caminho_gbk) else { return false };
    match fs::read(&p) {
        Err(_) => false,
        Ok(d) => antigo.is_none_or(|v| sha256_hex(&d) != v.sha256),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Divergencia {
    pub caminho: String,
    pub caminho_gbk: String,
    pub motivo: &'static str,
}

fn verificar_com(pasta: &Path, m: &Manifesto, completo: bool, ao_progredir: AoProgredir) -> Result<Vec<Divergencia>> {
    let total = m.arquivos.len() as u64;
    let feito = std::sync::atomic::AtomicU64::new(0);
    let mut fora = Vec::new();
    for bloco in m.arquivos.chunks(2048) {
        let r: Vec<Option<Divergencia>> = bloco
            .par_iter()
            .map(|a| {
                let legivel = nomes::legivel(&hex::decode(&a.caminho_gbk).unwrap_or_default());
                let Ok(p) = no_disco(pasta, &a.caminho_gbk) else {
                    return Some(Divergencia { caminho: legivel, caminho_gbk: a.caminho_gbk.clone(), motivo: "nome sem conversão" });
                };
                let motivo = match p.metadata() {
                    Err(_) => Some("faltando"),
                    Ok(_) if do_jogador(&legivel) => None,
                    Ok(md) if md.len() != a.tamanho => Some("tamanho diferente"),
                    Ok(_) if completo => match fs::read(&p) {
                        Ok(d) if sha256_hex(&d) == a.sha256 => None,
                        _ => Some("conteúdo diferente"),
                    },
                    Ok(_) => None,
                };
                feito.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                motivo.map(|motivo| Divergencia { caminho: legivel, caminho_gbk: a.caminho_gbk.clone(), motivo })
            })
            .collect();
        fora.extend(r.into_iter().flatten());
        ao_progredir(Progresso {
            etapa: "verificando".into(),
            feito: feito.load(std::sync::atomic::Ordering::Relaxed),
            total,
        });
    }
    Ok(fora)
}

/// Compara a pasta com o manifesto aplicado. `completo`: confere também o SHA-256 (lê tudo).
pub fn verificar(pasta: &Path, completo: bool, ao_progredir: AoProgredir) -> Result<Vec<Divergencia>> {
    verificar_com(pasta, &manifesto_local(pasta)?, completo, ao_progredir)
}

fn instalado(pasta: &Path) -> Result<(Estado, u32)> {
    let e = ler_estado(pasta)?.context("não há cliente instalado por este launcher nesta pasta")?;
    let n = e.numero.context("a instalação não terminou (use instalar para retomar)")?;
    Ok((e, n))
}

/// Adota um cliente que já está na pasta (instalado antes, copiado de outra máquina ou por outro
/// launcher): grava o manifesto atual e o estado, e repara o que divergir. Devolve quantos
/// arquivos precisaram ser baixados. Arquivos a mais na pasta (como `.pck` de uma instalação
/// original) ficam onde estão — com `sepfile` o cliente não os abre.
pub fn adotar(serv: &Servidor, versao: &str, pasta: &Path, ao_progredir: AoProgredir) -> Result<u64> {
    match ler_estado(pasta)? {
        Some(e) if e.versao_cliente != versao => bail!("a pasta tem o cliente {}, não o {versao}", e.versao_cliente),
        Some(e) if e.numero.is_some() => return reparar(serv, pasta, ao_progredir),
        _ => {}
    }
    ao_progredir(Progresso { etapa: "conferindo a versão no servidor".into(), feito: 0, total: 0 });
    let remota = serv.versao_remota(versao)?;
    conferir_nomes(pasta, &remota.manifesto)?;
    gravar_atomico(&pasta_estado(pasta).join("manifesto.json"), &remota.manifesto_bytes)?;
    gravar_estado(
        pasta,
        &Estado {
            formato: 1,
            versao_cliente: versao.into(),
            numero: Some(remota.versao.atual),
            manifesto_sha256: remota.versao.manifesto_sha256.clone(),
            pacotes_prontos: vec![],
        },
    )?;
    reparar(serv, pasta, ao_progredir)
}

/// Verificação completa e download, arquivo a arquivo, do que divergir.
pub fn reparar(serv: &Servidor, pasta: &Path, ao_progredir: AoProgredir) -> Result<u64> {
    let (e, _) = instalado(pasta)?;
    let m = manifesto_local(pasta)?;
    let fora = verificar_com(pasta, &m, true, ao_progredir)?;
    let por_caminho: HashMap<&str, &Arquivo> = m.arquivos.iter().map(|a| (a.caminho_gbk.as_str(), a)).collect();
    let total = fora.len() as u64;
    for (i, d) in fora.iter().enumerate() {
        let a = por_caminho[d.caminho_gbk.as_str()];
        ao_progredir(Progresso { etapa: format!("reparando {}", d.caminho), feito: i as u64, total });
        let dados = serv.objeto(&e.versao_cliente, &a.sha256, a.tamanho)?;
        gravar_atomico(&no_disco(pasta, &a.caminho_gbk)?, &dados)?;
    }
    Ok(total)
}

/// Leva a pasta à versão atual do servidor baixando só o que mudou. `None` se já está nela.
pub fn atualizar(serv: &Servidor, pasta: &Path, ao_progredir: AoProgredir) -> Result<Option<Resumo>> {
    let (mut e, local) = instalado(pasta)?;
    let remota = serv.versao_remota(&e.versao_cliente)?;
    if remota.versao.atual == local && remota.versao.manifesto_sha256 == e.manifesto_sha256 {
        return Ok(None);
    }
    conferir_nomes(pasta, &remota.manifesto)?;
    let antigo: BTreeMap<String, Arquivo> =
        manifesto_local(pasta)?.arquivos.into_iter().map(|a| (a.caminho_gbk.clone(), a)).collect();
    let mudados: Vec<&Arquivo> = remota
        .manifesto
        .arquivos
        .iter()
        .filter(|a| antigo.get(&a.caminho_gbk).is_none_or(|v| v.sha256 != a.sha256))
        .filter(|a| !mudado_pelo_jogador(pasta, a, antigo.get(&a.caminho_gbk)))
        .collect();
    let novos: std::collections::HashSet<&str> =
        remota.manifesto.arquivos.iter().map(|a| a.caminho_gbk.as_str()).collect();
    let removidos: Vec<&Arquivo> = antigo
        .values()
        .filter(|a| !novos.contains(a.caminho_gbk.as_str()))
        .filter(|a| !mudado_pelo_jogador(pasta, a, Some(a)))
        .collect();

    // 1. Baixa e confere tudo antes de mexer no cliente (retoma: o que já está aqui fica).
    let pasta_objetos = pasta_estado(pasta).join("objetos");
    fs::create_dir_all(&pasta_objetos)?;
    let total: u64 = mudados.iter().map(|a| a.tamanho).sum();
    let mut feito = 0u64;
    for a in &mudados {
        let local_obj = pasta_objetos.join(&a.sha256);
        let pronto = fs::read(&local_obj).is_ok_and(|d| sha256_hex(&d) == a.sha256);
        if !pronto {
            ao_progredir(Progresso { etapa: "baixando a atualização".into(), feito, total });
            let dados = serv.objeto(&e.versao_cliente, &a.sha256, a.tamanho)?;
            gravar_atomico(&local_obj, &dados)?;
        }
        feito += a.tamanho;
    }
    // 2. Aplica: grava os mudados, apaga os removidos, e só então troca o manifesto e o estado.
    ao_progredir(Progresso { etapa: "aplicando a atualização".into(), feito: total, total });
    let mut resumo = Resumo { numero: remota.versao.atual, ..Default::default() };
    for a in &mudados {
        let dados = fs::read(pasta_objetos.join(&a.sha256))?;
        gravar_atomico(&no_disco(pasta, &a.caminho_gbk)?, &dados)?;
        resumo.arquivos += 1;
        resumo.bytes += a.tamanho;
    }
    for a in &removidos {
        let p = no_disco(pasta, &a.caminho_gbk)?;
        if p.exists() {
            fs::remove_file(&p)?;
        }
        resumo.removidos += 1;
    }
    gravar_atomico(&pasta_estado(pasta).join("manifesto.json"), &remota.manifesto_bytes)?;
    e.numero = Some(remota.versao.atual);
    e.manifesto_sha256 = remota.versao.manifesto_sha256.clone();
    gravar_estado(pasta, &e)?;
    let _ = fs::remove_dir_all(&pasta_objetos);
    Ok(Some(resumo))
}

/// Pede os 16 primeiros bytes de um pacote com `Range` e confere a resposta 206 (download
/// retomável) — usado pela prova do servidor.
pub fn conferir_range(serv: &Servidor, versao: &str) -> Result<()> {
    let Origem::Http(_) = serv.origem else { bail!("Range só se confere por HTTP") };
    let remota = serv.versao_remota(versao)?;
    let p = remota.manifesto.pacotes.first().context("manifesto sem pacotes")?;
    let rel = rel_do_pacote(&remota.base, p);
    let mut inicio = vec![0u8; 16.min(p.tamanho as usize)];
    serv.origem.abrir_desde(versao, &rel, 0)?.0.read_exact(&mut inicio)?;
    let url = serv.origem.url(versao, &rel);
    let r = ureq::get(&url).set("Range", "bytes=0-15").call()?;
    if r.status() != 206 {
        bail!("GET {url} com Range devolveu {} (esperado 206)", r.status());
    }
    let mut v = Vec::new();
    r.into_reader().read_to_end(&mut v)?;
    if v != inicio {
        bail!("os bytes do Range não batem com o início do pacote");
    }
    Ok(())
}

/// Abre o jogo como o `elementclient.bat` da própria versão manda (traz o `sepfile`).
pub fn jogar(pasta: &Path) -> Result<()> {
    instalado(pasta)?;
    let bat = fs::read(pasta.join("elementclient.bat")).context("ler elementclient.bat")?;
    let linha = String::from_utf8_lossy(&bat)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('@') && !l.to_lowercase().starts_with("rem"))
        .map(str::to_string)
        .context("elementclient.bat vazio")?;
    let mut partes = linha.split_whitespace();
    let exe = partes.next().context("elementclient.bat sem programa")?;
    let exe = pasta.join(exe);
    if !exe.exists() || exe.extension().is_none_or(|x| !x.eq_ignore_ascii_case("exe")) {
        bail!("o elementclient.bat não aponta para um .exe da pasta");
    }
    std::process::Command::new(&exe)
        .args(partes)
        .current_dir(pasta)
        .spawn()
        .with_context(|| format!("abrir {}", exe.display()))?;
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn arquivos_do_jogador() {
        assert!(do_jogador(r"userdata\systemsettings.ini"));
        assert!(do_jogador(r"UserData\Layout\39.ini"));
        assert!(do_jogador(r"userdata\server\serverlist.txt"));
        assert!(!do_jogador(r"userdata.ini"));
        assert!(!do_jogador(r"data\elements.data"));
        assert!(!do_jogador(r"用户"));
        assert!(segue_o_servidor(r"userdata\server\serverlist.txt"));
        assert!(!segue_o_servidor(r"userdata\systemsettings.ini"));
    }
}
