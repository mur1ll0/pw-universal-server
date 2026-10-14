//! `pw-recursos importar`: monta a **versão 0** de uma versão de cliente a partir da
//! instalação original (com os `.pck`), uma vez só (§8.6 do
//! `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`, decisões do B233).
//!
//! 1. Cada `.pck` (com o `.pkx`) pelo extrator, nome em bytes GBK do índice.
//! 2. Os soltos da instalação: nome Unicode → bytes GBK. Quando o mesmo arquivo está no pacote e
//!    solto, **vale o do pacote**: o cliente abre o pacote primeiro e só procura no disco se não
//!    achar (`AngelicaFile/Source/AFileImage.cpp:94-104`).
//! 3. O que é nosso: `serverlist.txt` com linha de grupo (1.5.5) e `elementclient.bat` com
//!    `sepfile`. Tira o que não serve ao cliente sem `.pck`.
//! 4. Objetos por SHA-256 (zstd), pacotes `.tar.zst` por pasta (tar GNU, nome GBK), manifesto
//!    e `versao.json` assinados.

use crate::assinatura::Chave;
use crate::fonte::FonteDePacote;
use crate::manifesto::{self, Arquivo, Manifesto, Pacote, Versao};
use crate::nomes;
use crate::objetos::{md5, sha256, Objetos, Sha256Bytes};
use crate::tar_gnu;
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origem {
    Pacote,
    Solto,
    Nosso,
}

impl Origem {
    pub fn como_texto(self) -> &'static str {
        match self {
            Origem::Pacote => "pacote",
            Origem::Solto => "solto",
            Origem::Nosso => "nosso",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Registro {
    pub caminho: Vec<u8>,
    pub origem: Origem,
    pub pacote_original: Option<String>,
    pub pacote: String,
    pub tamanho: u64,
    pub sha256: Sha256Bytes,
    pub md5: [u8; 16],
}

#[derive(Debug, Default, Serialize)]
pub struct Relatorio {
    pub versao: String,
    pub origem: String,
    pub pacotes_lidos: BTreeMap<String, u64>,
    pub falhas_de_pacote: Vec<(String, String)>,
    pub repetidos_entre_pacotes: Vec<String>,
    pub soltos_lidos: u64,
    pub soltos_repetidos_do_pacote: u64,
    pub excluidos: BTreeMap<String, u64>,
    pub fora_do_gbk: Vec<String>,
    pub nossos: Vec<String>,
    pub total_arquivos: u64,
    pub total_bytes: u64,
    pub pacotes: Vec<Pacote>,
    pub segundos: BTreeMap<String, f64>,
}

pub struct Importacao<'a> {
    pub versao: &'a str,
    pub cliente: &'a Path,
    /// Raiz do volume (`data/clientes`): tudo vai para `<dados>/<versao>/`.
    pub dados: &'a Path,
    pub nivel: i32,
    /// Se a `serverlist.txt` precisa começar por uma linha de grupo (1.5.5:
    /// `EC_ServerList.cpp:146-157`; o 1.2.6 dispensa, §8.5).
    pub linha_de_grupo: bool,
    pub fonte: &'a dyn FonteDePacote,
    pub chave: &'a Chave,
}

pub struct Resultado {
    pub registros: Vec<Registro>,
    pub manifesto_sha256: Sha256Bytes,
    pub assinatura: [u8; 64],
    pub relatorio: Relatorio,
}

/// A linha de grupo vale para clientes 1.5 em diante. Evidência: 1.5.5 cai sem ela (§7.7,
/// `EC_ServerList.cpp:146-157` — servidor antes de qualquer grupo fica fora de todos); 1.2.6
/// confirmado em jogo sem ela (§8.5). Outras versões: a conferir quando forem importadas.
pub fn precisa_linha_de_grupo(versao: &str) -> bool {
    let mut partes = versao.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let (a, b) = (partes.next().unwrap_or(0), partes.next().unwrap_or(0));
    (a, b) >= (1, 5)
}

pub fn validar_versao(versao: &str) -> Result<()> {
    if versao.is_empty()
        || versao.len() > 16
        || !versao.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        || versao.starts_with('.')
    {
        bail!("versão de cliente inválida: {versao:?}");
    }
    Ok(())
}

pub fn pasta_da_versao(dados: &Path, versao: &str) -> PathBuf {
    dados.join(versao)
}

/// O que fica fora da instalação (§8.6): pacotes (o cliente roda com `sepfile`), as
/// ferramentas e sobras de extração/reempacotamento, logs e capturas de tela da máquina de
/// origem. Devolve o motivo.
fn excluido(partes_minusculas: &[String]) -> Option<&'static str> {
    let nome = partes_minusculas.last()?;
    let topo = &partes_minusculas[0];
    if nome.ends_with(".pck") || nome.ends_with(".pkx") {
        return Some("pacote .pck/.pkx (o cliente roda com sepfile)");
    }
    if partes_minusculas.iter().any(|p| p.ends_with(".pck.files")) {
        return Some("sobra de extração (*.pck.files)");
    }
    if partes_minusculas.len() == 1
        && matches!(
            nome.as_str(),
            "spck.exe" | "pwrepack.exe" | "pwrepack.bat" | "extract_pck.bat" | "update5.rar"
        )
    {
        return Some("ferramenta de pacote/atualização antiga");
    }
    // Instaladores avulsos da raiz do BR (360, GT, DirectX, Unity): não são o cliente
    // (decisão do Murillo, B233).
    if partes_minusculas.len() == 1
        && matches!(
            nome.as_str(),
            "360inst_wanmeigj.exe"
                | "gt_setup.exe"
                | "gt_updater.exe"
                | "dxwebsetup.exe"
                | "unitywebplayerdevelopment.exe"
        )
    {
        return Some("instalador avulso");
    }
    // Scripts do BR que mudam o `increaseUserVa` do Windows 32 bits por `bcdedit`: mexem no
    // sistema, não no cliente (decisão do Murillo, B233).
    if partes_minusculas.len() == 1
        && matches!(
            nome.as_str(),
            "enablemoremem_vista_win7_32bit.bat" | "disablemoremem_vista_win7_32bit.bat"
        )
    {
        return Some("ajuste de memória do Windows 32 bits (bcdedit)");
    }
    if topo == "data" {
        // Sobras de edição no data\ do 1.2.6 (decisão do Murillo, B233): cópias, exportações,
        // arquivos compactados e anotações. O data\ do 1.5.5 BR não tem nenhuma.
        let n = partes_minusculas.len();
        if n > 2 && matches!(partes_minusculas[1].as_str(), "bck" | "data export" | "editados" | "tasks") {
            return Some("sobra de edição em data\\");
        }
        if n == 2 && (nome.ends_with(".rar") || nome.ends_with(".zip") || nome == "notas.txt") {
            return Some("sobra de edição em data\\");
        }
        // Arquivo do servidor: nenhum .cpp do cliente abre o npcgen.data (a única ocorrência
        // é um campo de estrutura, ElementClient/CCommon/elementdataman.cpp:2073).
        if n == 2 && nome == "npcgen.data" {
            return Some("arquivo do servidor (npcgen.data)");
        }
    }
    if nome.ends_with(".log") {
        return Some("log");
    }
    if partes_minusculas.len() > 1 && matches!(topo.as_str(), "logs" | "screenshots" | "novosmodels") {
        return Some("logs, capturas ou sobras de reempacotamento da máquina de origem");
    }
    None
}

/// Pacote de instalação de um caminho: a primeira pasta (em minúsculas), ou `raiz`.
pub fn pacote_de(caminho: &[u8]) -> String {
    match caminho.iter().position(|&b| b == b'\\') {
        None => "raiz".to_string(),
        Some(i) => nomes::legivel(&caminho[..i])
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
            .collect(),
    }
}

/// Garante que a 1ª linha não vazia da `serverlist.txt` (UTF-16LE com BOM, como as dos dois
/// clientes) seja de grupo — sem tabulação (`EC_ServerList.cpp:151-157`). `None` se já é.
pub fn com_linha_de_grupo(original: &[u8]) -> Result<Option<Vec<u8>>> {
    let texto = if original.is_empty() {
        String::new()
    } else {
        if original.len() % 2 != 0 || original[..2] != [0xFF, 0xFE] {
            bail!("serverlist.txt não está em UTF-16LE com BOM");
        }
        let unidades: Vec<u16> = original[2..]
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&unidades).context("serverlist.txt com UTF-16 inválido")?
    };
    let primeira = texto.split(['\r', '\n']).find(|l| !l.is_empty());
    if matches!(primeira, Some(l) if !l.contains('\t')) {
        return Ok(None);
    }
    let novo = format!("pw-universal\r\n{texto}");
    let mut bytes = vec![0xFF, 0xFE];
    for u in novo.encode_utf16() {
        bytes.extend(u.to_le_bytes());
    }
    Ok(Some(bytes))
}

/// `elementclient.bat` com a chave `sepfile` (o cliente sem `.pck` só abre com ela, §7.1/§8.5).
pub fn com_sepfile(original: &[u8]) -> Vec<u8> {
    let texto = String::from_utf8_lossy(original);
    let linha = texto.trim_end();
    if linha.split_whitespace().any(|p| p.eq_ignore_ascii_case("sepfile")) {
        return original.to_vec();
    }
    if linha.is_empty() {
        // Sem .bat na instalação: a linha dos dois clientes (sem os extras do BR).
        return b"elementclient.exe game:cpw console=1 sepfile".to_vec();
    }
    format!("{linha} sepfile").into_bytes()
}

struct Coletor<'a> {
    objetos: &'a Objetos,
    registros: Vec<Registro>,
    por_chave: HashMap<String, usize>,
    pendentes: Vec<(Sha256Bytes, Vec<u8>)>,
    na_fila: HashSet<Sha256Bytes>,
    bytes_pendentes: usize,
}

const LOTE: usize = 256 << 20;

impl<'a> Coletor<'a> {
    fn existe(&self, caminho: &[u8]) -> bool {
        self.por_chave.contains_key(&nomes::chave(caminho))
    }

    fn adicionar(&mut self, caminho: Vec<u8>, dados: Vec<u8>, origem: Origem, pacote_original: Option<String>) -> Result<()> {
        let sha = sha256(&dados);
        let registro = Registro {
            pacote: pacote_de(&caminho),
            caminho,
            origem,
            pacote_original,
            tamanho: dados.len() as u64,
            sha256: sha,
            md5: md5(&dados),
        };
        let chave = nomes::chave(&registro.caminho);
        match self.por_chave.get(&chave) {
            Some(&i) => self.registros[i] = registro, // só o "nosso" substitui
            None => {
                self.por_chave.insert(chave, self.registros.len());
                self.registros.push(registro);
            }
        }
        if self.na_fila.insert(sha) {
            self.bytes_pendentes += dados.len();
            self.pendentes.push((sha, dados));
            if self.bytes_pendentes >= LOTE {
                self.gravar_pendentes()?;
            }
        }
        Ok(())
    }

    fn gravar_pendentes(&mut self) -> Result<()> {
        let lote = std::mem::take(&mut self.pendentes);
        self.bytes_pendentes = 0;
        lote.par_iter()
            .try_for_each(|(sha, dados)| self.objetos.gravar(sha, dados))
            .context("gravar objetos")?;
        Ok(())
    }

    fn ler(&self, caminho: &[u8]) -> Result<Option<Vec<u8>>> {
        let Some(&i) = self.por_chave.get(&nomes::chave(caminho)) else { return Ok(None) };
        let r = &self.registros[i];
        if let Some((_, d)) = self.pendentes.iter().find(|(s, _)| *s == r.sha256) {
            return Ok(Some(d.clone()));
        }
        Ok(Some(self.objetos.ler(&r.sha256, r.tamanho as usize)?))
    }
}

fn listar_pcks(raiz: &Path, saida: &mut Vec<PathBuf>) -> Result<()> {
    for e in fs::read_dir(raiz).with_context(|| format!("listar {}", raiz.display()))? {
        let e = e?;
        let tipo = e.file_type()?;
        let p = e.path();
        if tipo.is_dir() {
            let nome = e.file_name().to_string_lossy().to_lowercase();
            if !nome.ends_with(".pck.files") {
                listar_pcks(&p, saida)?;
            }
        } else if tipo.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pck")) {
            saida.push(p);
        }
    }
    Ok(())
}

fn soltos(raiz: &Path, partes: &mut Vec<String>, saida: &mut Vec<(Vec<String>, PathBuf)>) -> Result<()> {
    let pasta = partes.iter().fold(raiz.to_path_buf(), |p, x| p.join(x));
    let mut entradas: Vec<_> = fs::read_dir(&pasta)
        .with_context(|| format!("listar {}", pasta.display()))?
        .collect::<io::Result<_>>()?;
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        let tipo = e.file_type()?;
        let nome = e
            .file_name()
            .into_string()
            .map_err(|n| anyhow::anyhow!("nome de arquivo sem Unicode válido: {n:?}"))?;
        partes.push(nome);
        if tipo.is_dir() {
            soltos(raiz, partes, saida)?;
        } else if tipo.is_file() {
            saida.push((partes.clone(), e.path()));
        }
        partes.pop();
    }
    Ok(())
}

struct EscritorComHash<W: Write> {
    w: W,
    hash: Sha256,
    tamanho: u64,
}

impl<W: Write> Write for EscritorComHash<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.w.write(b)?;
        self.hash.update(&b[..n]);
        self.tamanho += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

struct ContadorDeBytes(u64);

fn gerar_pacote(
    objetos: &Objetos,
    nome: &str,
    registros: &[&Registro],
    destino: &Path,
    nivel: i32,
) -> Result<Pacote> {
    let temporario = destino.with_extension("zst.tmp");
    let arquivo = io::BufWriter::with_capacity(1 << 20, fs::File::create(&temporario)?);
    let saida = EscritorComHash { w: arquivo, hash: Sha256::new(), tamanho: 0 };
    let mut zst = zstd::stream::Encoder::new(saida, nivel)?;
    zst.include_checksum(true)?;
    let fios = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
    zst.multithread(fios)?;
    let mut tar = tar_gnu::Escritor::new(ContarEscrita { w: zst, c: ContadorDeBytes(0) });
    // Lê os objetos em paralelo, em blocos, e escreve no tar em ordem: lendo num fio só (bind
    // mount do Windows + zstd), a geração dos pacotes do 1.5.5 levou 37 min no B233.
    for bloco in registros.chunks(64) {
        let dados: Vec<Vec<u8>> = bloco
            .par_iter()
            .map(|r| objetos.ler(&r.sha256, r.tamanho as usize))
            .collect::<io::Result<_>>()?;
        for (r, d) in bloco.iter().zip(&dados) {
            tar.arquivo(&r.caminho, d)?;
        }
    }
    let ContarEscrita { w: zst, c } = tar.terminar()?;
    let mut saida = zst.finish()?;
    saida.flush()?;
    let EscritorComHash { w, hash, tamanho } = saida;
    w.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    fs::rename(&temporario, destino)?;
    Ok(Pacote {
        nome: nome.to_string(),
        arquivo: format!("pacotes/{nome}.tar.zst"),
        sha256: hex::encode(hash.finalize()),
        tamanho,
        tamanho_tar: c.0,
        arquivos: registros.len() as u64,
        numero: None,
    })
}

struct ContarEscrita<W: Write> {
    w: W,
    c: ContadorDeBytes,
}

impl<W: Write> Write for ContarEscrita<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.w.write(b)?;
        self.c.0 += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

/// Grava `dados` e `<caminho>.sig` (assinatura em hex). A assinatura vai antes, e o `.json` por
/// troca atômica: quem lê nunca vê um `.json` novo com a assinatura velha.
pub(crate) fn gravar_assinado(caminho: &Path, dados: &[u8], chave: &Chave) -> Result<[u8; 64]> {
    let assinatura = chave.assinar(dados);
    let mut tmp = caminho.as_os_str().to_owned();
    tmp.push(".tmp");
    fs::write(&tmp, dados)?;
    let mut sig = caminho.as_os_str().to_owned();
    sig.push(".sig");
    fs::write(PathBuf::from(sig), hex::encode(assinatura))?;
    fs::rename(&tmp, caminho)?;
    Ok(assinatura)
}

pub fn importar(imp: &Importacao) -> Result<Resultado> {
    validar_versao(imp.versao)?;
    let pasta = pasta_da_versao(imp.dados, imp.versao);
    let atualizacao = pasta.join("atualizacao");
    if atualizacao.join("versao.json").exists() {
        bail!(
            "a versão 0 de {} já existe em {} (use --refazer para gerar de novo)",
            imp.versao,
            atualizacao.display()
        );
    }
    let _ = fs::remove_dir_all(atualizacao.join("0"));
    let objetos = Objetos::new(pasta.join("objetos"), imp.nivel);
    let mut rel = Relatorio {
        versao: imp.versao.to_string(),
        origem: imp.cliente.display().to_string(),
        ..Default::default()
    };
    let mut col = Coletor {
        objetos: &objetos,
        registros: Vec::new(),
        por_chave: HashMap::new(),
        pendentes: Vec::new(),
        na_fila: HashSet::new(),
        bytes_pendentes: 0,
    };

    // 1. Pacotes.
    let t = Instant::now();
    let mut pcks = Vec::new();
    listar_pcks(imp.cliente, &mut pcks)?;
    pcks.sort();
    if pcks.is_empty() {
        bail!("nenhum .pck em {} — a importação lê a instalação original, com os pacotes", imp.cliente.display());
    }
    for pck in &pcks {
        let nome_pck = pck.strip_prefix(imp.cliente).unwrap_or(pck).to_string_lossy().replace('/', "\\");
        let mut repetidos = Vec::new();
        let resultado = imp.fonte.ler(pck, &mut |nome, dados| {
            let nome = nomes::normalizar(&nome);
            if col.existe(&nome) {
                repetidos.push(nomes::legivel(&nome));
                return Ok(());
            }
            col.adicionar(nome, dados, Origem::Pacote, Some(nome_pck.clone()))
        })?;
        tracing::info!("{nome_pck}: {} arquivos, {} falhas", resultado.entradas, resultado.falhas.len());
        rel.pacotes_lidos.insert(nome_pck.clone(), resultado.entradas);
        rel.repetidos_entre_pacotes.extend(repetidos);
        rel.falhas_de_pacote
            .extend(resultado.falhas.into_iter().map(|(n, e)| (nomes::legivel(&n), e)));
    }
    rel.segundos.insert("pacotes".into(), t.elapsed().as_secs_f64());

    // 2. Soltos.
    let t = Instant::now();
    let mut lista = Vec::new();
    soltos(imp.cliente, &mut Vec::new(), &mut lista)?;
    for (partes, caminho) in lista {
        let minusculas: Vec<String> = partes.iter().map(|p| p.to_lowercase()).collect();
        if let Some(motivo) = excluido(&minusculas) {
            *rel.excluidos.entry(motivo.to_string()).or_default() += 1;
            continue;
        }
        let unicode = partes.join("\\");
        let Some(gbk) = nomes::gbk_de_unicode(&unicode) else {
            rel.fora_do_gbk.push(unicode);
            continue;
        };
        if col.existe(&gbk) {
            rel.soltos_repetidos_do_pacote += 1;
            continue;
        }
        let dados = fs::read(&caminho).with_context(|| format!("ler {}", caminho.display()))?;
        rel.soltos_lidos += 1;
        col.adicionar(gbk, dados, Origem::Solto, None)?;
    }
    rel.segundos.insert("soltos".into(), t.elapsed().as_secs_f64());

    // 3. O que é nosso.
    let serverlist = b"userdata\\server\\serverlist.txt".to_vec();
    if imp.linha_de_grupo {
        let atual = col.ler(&serverlist)?.unwrap_or_default();
        if let Some(nova) = com_linha_de_grupo(&atual)? {
            col.adicionar(serverlist, nova, Origem::Nosso, None)?;
            rel.nossos.push("userdata\\server\\serverlist.txt (linha de grupo)".into());
        }
    }
    let bat = b"elementclient.bat".to_vec();
    let atual = col.ler(&bat)?.unwrap_or_default();
    let novo = com_sepfile(&atual);
    if novo != atual {
        col.adicionar(bat, novo, Origem::Nosso, None)?;
        rel.nossos.push("elementclient.bat (sepfile)".into());
    }
    col.gravar_pendentes()?;

    // 4. Pacotes de instalação.
    let t = Instant::now();
    let mut registros = col.registros;
    registros.sort_by(|a, b| (&a.pacote, &a.caminho).cmp(&(&b.pacote, &b.caminho)));
    let dir_pacotes = atualizacao.join("0").join("pacotes");
    fs::create_dir_all(&dir_pacotes)?;
    let mut grupos: BTreeMap<&str, Vec<&Registro>> = BTreeMap::new();
    for r in &registros {
        grupos.entry(r.pacote.as_str()).or_default().push(r);
    }
    for (nome, lista) in &grupos {
        let p = gerar_pacote(&objetos, nome, lista, &dir_pacotes.join(format!("{nome}.tar.zst")), imp.nivel)
            .with_context(|| format!("gerar o pacote {nome}"))?;
        tracing::info!("pacote {nome}: {} arquivos, tar {} → {} bytes", p.arquivos, p.tamanho_tar, p.tamanho);
        rel.pacotes.push(p);
    }
    rel.segundos.insert("pacotes_de_instalacao".into(), t.elapsed().as_secs_f64());

    // 5. Manifesto e versao.json, assinados. O versao.json é o último: só aparece completo.
    let mut arquivos: Vec<Arquivo> = registros
        .iter()
        .map(|r| Arquivo {
            caminho_gbk: hex::encode(&r.caminho),
            sha256: hex::encode(r.sha256),
            tamanho: r.tamanho,
            pacote: r.pacote.clone(),
        })
        .collect();
    arquivos.sort_by(|a, b| a.caminho_gbk.cmp(&b.caminho_gbk));
    let m = Manifesto {
        formato: manifesto::FORMATO,
        versao_cliente: imp.versao.to_string(),
        numero: 0,
        gerado_em: chrono::Utc::now().to_rfc3339(),
        pacotes: rel.pacotes.clone(),
        arquivos,
    };
    let bytes = serde_json::to_vec(&m)?;
    let manifesto_sha256 = sha256(&bytes);
    let assinatura = gravar_assinado(&atualizacao.join("0").join("manifesto.json"), &bytes, imp.chave)?;
    let v = Versao {
        formato: manifesto::FORMATO,
        versao_cliente: imp.versao.to_string(),
        atual: 0,
        minima: 0,
        manifesto: "0/manifesto.json".into(),
        manifesto_sha256: hex::encode(manifesto_sha256),
    };
    rel.total_arquivos = registros.len() as u64;
    rel.total_bytes = registros.iter().map(|r| r.tamanho).sum();
    fs::write(pasta.join("importacao_0.json"), serde_json::to_vec_pretty(&rel)?)?;
    gravar_assinado(&atualizacao.join("versao.json"), &serde_json::to_vec_pretty(&v)?, imp.chave)?;

    Ok(Resultado { registros, manifesto_sha256, assinatura, relatorio: rel })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn partes(c: &str) -> Vec<String> {
        c.split('\\').map(|s| s.to_lowercase()).collect()
    }

    #[test]
    fn exclusoes_do_8_6() {
        assert!(excluido(&partes("configs.pck")).is_some());
        assert!(excluido(&partes("litmodels.pkx")).is_some());
        assert!(excluido(&partes("gfx.pck.files\\a.gfx")).is_some());
        assert!(excluido(&partes("sPCK.exe")).is_some());
        assert!(excluido(&partes("update5.rar")).is_some());
        assert!(excluido(&partes("models.pck.log")).is_some());
        assert!(excluido(&partes("logs\\ec.txt")).is_some());
        assert!(excluido(&partes("360inst_wanmeigj.exe")).is_some());
        assert!(excluido(&partes("gt_updater.exe")).is_some());
        assert!(excluido(&partes("enablemoremem_vista_win7_32bit.bat")).is_some());
        assert!(excluido(&partes("disablemoremem_vista_win7_32bit.bat")).is_some());
        assert!(excluido(&partes("elementclient.bat")).is_none());
        assert!(excluido(&partes("data\\BCK\\elements.data")).is_some());
        assert!(excluido(&partes("data\\Data Export\\x.txt")).is_some());
        assert!(excluido(&partes("data\\tasks\\a.data")).is_some());
        assert!(excluido(&partes("data\\update1.rar")).is_some());
        assert!(excluido(&partes("data\\Notas.txt")).is_some());
        assert!(excluido(&partes("data\\npcgen.data")).is_some());
        assert!(excluido(&partes("data\\elements.data")).is_none());
        assert!(excluido(&partes("data\\tasks.data")).is_none());
        assert!(excluido(&partes("data\\aipolicy.data")).is_none());
        assert!(excluido(&partes("elementclient.exe")).is_none());
        assert!(excluido(&partes("maps\\world\\a.hmap")).is_none());
        assert!(excluido(&partes("userdata\\server\\serverlist.txt")).is_none());
    }

    #[test]
    fn linha_de_grupo_entra_uma_vez() {
        let sem: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("Universal 1.5.5\t29004:127.0.0.1\t2\r\n".encode_utf16().flat_map(|u| u.to_le_bytes()))
            .collect();
        let com = com_linha_de_grupo(&sem).unwrap().unwrap();
        let texto = String::from_utf16(
            &com[2..].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(texto.starts_with("pw-universal\r\nUniversal 1.5.5\t"));
        assert!(com_linha_de_grupo(&com).unwrap().is_none());
        assert!(com_linha_de_grupo(b"abc").is_err());
    }

    #[test]
    fn sepfile_acrescentado_sem_repetir() {
        let br = b"elementclient.exe game:cpw console=1 startbypatcher logiccheck:0";
        assert_eq!(com_sepfile(br), b"elementclient.exe game:cpw console=1 startbypatcher logiccheck:0 sepfile");
        let ja = com_sepfile(br);
        assert_eq!(com_sepfile(&ja), ja);
    }

    #[test]
    fn versoes_com_grupo() {
        assert!(precisa_linha_de_grupo("1.5.5"));
        assert!(!precisa_linha_de_grupo("1.2.6"));
    }

    #[test]
    fn pacote_pela_primeira_pasta() {
        assert_eq!(pacote_de(b"Models\\a.ecm"), "models");
        assert_eq!(pacote_de(b"elementclient.exe"), "raiz");
    }
}
