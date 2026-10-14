//! `pw-recursos conferir`: a prova do P1 e o núcleo do que o launcher (P2) fará. Baixa (HTTP)
//! ou lê (pasta) o `versao.json` e o manifesto, confere as assinaturas, extrai cada pacote numa
//! pasta limpa convertendo o nome pela página de código deste Windows (§7.7) e confere tamanho e
//! SHA-256 de cada arquivo e de cada pacote contra o manifesto.

use crate::assinatura;
use crate::manifesto::{Manifesto, Versao};
use crate::nomes;
use crate::objetos::sha256;
use crate::tar_gnu;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub enum Origem {
    /// `http://host:8100` — o mesmo caminho que o launcher usará.
    Http(String),
    /// Raiz do volume (`data/clientes`).
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

    fn abrir(&self, versao: &str, rel: &str) -> Result<Box<dyn Read>> {
        match self {
            Origem::Http(base) => {
                let url = format!("{base}/atualizacao/{versao}/{rel}");
                let r = ureq::get(&url).call().with_context(|| format!("GET {url}"))?;
                Ok(Box::new(r.into_reader()))
            }
            Origem::Pasta(raiz) => {
                let p = raiz.join(versao).join("atualizacao").join(rel);
                Ok(Box::new(fs::File::open(&p).with_context(|| format!("abrir {}", p.display()))?))
            }
        }
    }

    fn ler(&self, versao: &str, rel: &str) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        self.abrir(versao, rel)?.read_to_end(&mut v)?;
        Ok(v)
    }
}

#[derive(Debug, Default)]
pub struct Conferencia {
    pub pacotes: u64,
    pub arquivos: u64,
    pub bytes: u64,
    pub range_conferido: bool,
}

struct LeitorComHash<R: Read> {
    r: R,
    hash: Sha256,
    tamanho: u64,
}

impl<R: Read> Read for LeitorComHash<R> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.r.read(b)?;
        self.hash.update(&b[..n]);
        self.tamanho += n as u64;
        Ok(n)
    }
}

fn ler_assinatura(origem: &Origem, versao: &str, rel: &str) -> Result<Vec<u8>> {
    let texto = origem.ler(versao, &format!("{rel}.sig"))?;
    hex::decode(String::from_utf8_lossy(&texto).trim()).context("assinatura não está em hex")
}

/// Pede os 16 primeiros bytes do pacote com `Range` e confere a resposta 206 (download retomável).
fn conferir_range(base: &str, versao: &str, rel: &str, inicio_esperado: &[u8]) -> Result<()> {
    let url = format!("{base}/atualizacao/{versao}/{rel}");
    let r = ureq::get(&url).set("Range", "bytes=0-15").call()?;
    if r.status() != 206 {
        bail!("GET {url} com Range devolveu {} (esperado 206)", r.status());
    }
    let mut v = Vec::new();
    r.into_reader().read_to_end(&mut v)?;
    if v != inicio_esperado {
        bail!("os bytes do Range não batem com o início do pacote");
    }
    Ok(())
}

pub fn conferir(origem: &Origem, versao: &str, destino: &Path, chave_publica: &str) -> Result<Conferencia> {
    if destino.exists() && fs::read_dir(destino)?.next().is_some() {
        bail!("a pasta {} não está vazia (a prova é numa pasta limpa)", destino.display());
    }
    let v_bytes = origem.ler(versao, "versao.json")?;
    assinatura::verificar(chave_publica, &v_bytes, &ler_assinatura(origem, versao, "versao.json")?)
        .context("versao.json")?;
    let v: Versao = serde_json::from_slice(&v_bytes)?;
    let m_bytes = origem.ler(versao, &v.manifesto)?;
    if hex::encode(sha256(&m_bytes)) != v.manifesto_sha256 {
        bail!("SHA-256 do manifesto não bate com o versao.json");
    }
    assinatura::verificar(chave_publica, &m_bytes, &ler_assinatura(origem, versao, &v.manifesto)?)
        .context("manifesto.json")?;
    let m: Manifesto = serde_json::from_slice(&m_bytes)?;
    let base_rel = v.manifesto.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();

    let mut esperados: HashMap<Vec<u8>, (&str, &str, u64)> = HashMap::new();
    for a in &m.arquivos {
        let nome = hex::decode(&a.caminho_gbk).context("caminho_gbk")?;
        if esperados.insert(nome, (&a.sha256, &a.pacote, a.tamanho)).is_some() {
            bail!("caminho repetido no manifesto: {}", a.caminho_gbk);
        }
    }
    let mut c = Conferencia::default();
    for p in &m.pacotes {
        let rel = format!("{base_rel}/{}", p.arquivo);
        let leitor = LeitorComHash { r: origem.abrir(versao, &rel)?, hash: Sha256::new(), tamanho: 0 };
        let zst = zstd::stream::Decoder::new(leitor)?;
        let mut tar = tar_gnu::Leitor::new(zst);
        while let Some((nome, dados)) = tar.proximo().with_context(|| format!("ler o pacote {}", p.nome))? {
            let Some((sha, pacote, tamanho)) = esperados.remove(&nome) else {
                bail!("{} traz um arquivo fora do manifesto (ou repetido): {}", p.nome, nomes::legivel(&nome));
            };
            if pacote != p.nome || tamanho != dados.len() as u64 || hex::encode(sha256(&dados)) != sha {
                bail!("{} não confere com o manifesto", nomes::legivel(&nome));
            }
            let caminho = destino.join(nomes::nome_no_disco(&nome)?);
            fs::create_dir_all(caminho.parent().unwrap())?;
            fs::write(&caminho, &dados).with_context(|| format!("gravar {}", caminho.display()))?;
            c.arquivos += 1;
            c.bytes += dados.len() as u64;
        }
        // Lê o que sobrou (fim do quadro zstd) para o hash cobrir o pacote inteiro.
        let mut resto = tar.into_inner().finish();
        io::copy(&mut resto, &mut io::sink())?;
        let leitor = resto.into_inner();
        if leitor.tamanho != p.tamanho || hex::encode(leitor.hash.finalize()) != p.sha256 {
            bail!("pacote {}: tamanho/SHA-256 não confere com o manifesto", p.nome);
        }
        c.pacotes += 1;
    }
    if !esperados.is_empty() {
        bail!("{} arquivos do manifesto não vieram em nenhum pacote", esperados.len());
    }
    if let (Origem::Http(base), Some(p)) = (origem, m.pacotes.first()) {
        let rel = format!("{base_rel}/{}", p.arquivo);
        let mut inicio = vec![0u8; 16.min(p.tamanho as usize)];
        origem.abrir(versao, &rel)?.read_exact(&mut inicio)?;
        conferir_range(base, versao, &rel, &inicio)?;
        c.range_conferido = true;
    }
    Ok(c)
}
