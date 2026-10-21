//! B332 — envio do cliente original pelo painel, para importar a versão 0 no servidor externo (decisão do B233:
//! "pasta enviada arquivo a arquivo, retomável"; spec 07 §5).
//!
//! O navegador escolhe a pasta da instalação (`<input webkitdirectory>`) e manda cada arquivo em blocos. Aqui:
//! - a área de recebimento é `<dados>/.envios/<id>/` — o ponto inicial impede colidir com o nome de uma versão
//!   (`importar::validar_versao` recusa nome que começa por ponto);
//! - o caminho de cada arquivo é relativo, com `/` ou `\`, sem `..`, sem raiz nem letra de unidade
//!   ([`caminho_seguro`]);
//! - o bloco só **acrescenta**: o `deslocamento` tem de ser o tamanho que já chegou, senão 409 com o tamanho atual —
//!   é o que deixa retomar depois de uma queda sem corromper o arquivo ([`gravar_bloco`]);
//! - a importação é a mesma do comando `pw-recursos importar` ([`importar_versao_zero`]), em segundo plano.
use crate::assinatura::Chave;
use crate::fonte::FonteDePacote;
use crate::importar::{self, Importacao};
use anyhow::{bail, Context, Result};
use pw_storage::RecursosRepository;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

/// O maior bloco aceito por pedido (16 MiB): o painel manda 8 MiB.
pub const MAIOR_BLOCO: usize = 16 << 20;

/// A pasta de todos os envios.
pub fn raiz(dados: &Path) -> PathBuf {
    dados.join(".envios")
}

/// Id de envio: letras, números e `-` (até 40).
pub fn validar_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 40 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("id de envio inválido");
    }
    Ok(())
}

/// O caminho dentro do envio, ou erro se ele sair da pasta.
pub fn caminho_seguro(pasta: &Path, caminho: &str) -> Result<PathBuf> {
    let normal = caminho.replace('\\', "/");
    if normal.is_empty() || normal.len() > 1024 || normal.starts_with('/') || normal.contains(':') || normal.contains('\0') {
        bail!("caminho inválido: {caminho:?}");
    }
    let mut p = pasta.to_path_buf();
    for parte in normal.split('/') {
        let c = Path::new(parte).components().collect::<Vec<_>>();
        if parte.is_empty() || c.len() != 1 || !matches!(c[0], Component::Normal(_)) {
            bail!("caminho inválido: {caminho:?}");
        }
        p.push(parte);
    }
    Ok(p)
}

/// Grava um bloco de `caminho` a partir de `deslocamento`, que tem de ser o tamanho que já chegou. Devolve o tamanho
/// novo. `Err(Ok(tamanho))` quando o deslocamento não bate (o painel retoma dali).
pub fn gravar_bloco(pasta: &Path, caminho: &str, deslocamento: u64, bytes: &[u8]) -> Result<std::result::Result<u64, u64>> {
    use std::io::Write;
    if bytes.len() > MAIOR_BLOCO {
        bail!("bloco acima de {} MiB", MAIOR_BLOCO >> 20);
    }
    let alvo = caminho_seguro(pasta, caminho)?;
    let atual = std::fs::metadata(&alvo).map(|m| m.len()).unwrap_or(0);
    if atual != deslocamento {
        return Ok(Err(atual));
    }
    if let Some(pai) = alvo.parent() {
        std::fs::create_dir_all(pai)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&alvo).with_context(|| format!("abrir {caminho}"))?;
    f.write_all(bytes)?;
    f.sync_data()?;
    Ok(Ok(atual + bytes.len() as u64))
}

/// Os arquivos recebidos (caminho com `/`, tamanho), em ordem.
pub fn listar(pasta: &Path) -> Vec<(String, u64)> {
    fn andar(base: &Path, p: &Path, v: &mut Vec<(String, u64)>) {
        let Ok(it) = std::fs::read_dir(p) else { return };
        for e in it.flatten() {
            let caminho = e.path();
            if caminho.is_dir() {
                andar(base, &caminho, v);
            } else if let (Ok(rel), Ok(m)) = (caminho.strip_prefix(base), e.metadata()) {
                v.push((rel.to_string_lossy().replace('\\', "/"), m.len()));
            }
        }
    }
    let mut v = Vec::new();
    andar(pasta, pasta, &mut v);
    v.sort();
    v
}

/// O andamento da importação de um envio.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Importando {
    pub versao: String,
    /// `em_andamento`, `concluida` ou `falha`.
    pub estado: &'static str,
    pub mensagem: String,
}

/// O estado dos envios em memória (só a importação; os arquivos estão no disco).
#[derive(Default)]
pub struct Envios {
    pub importacoes: Mutex<HashMap<String, Importando>>,
}

/// O resumo de uma importação, numa linha (o comando imprime também os detalhes).
pub fn resumo(versao: &str, r: &importar::Relatorio) -> String {
    let mb = |b: u64| b as f64 / 1_048_576.0;
    format!(
        "versão 0 de {versao}: {} arquivos, {:.1} MB; pacotes .pck lidos {}, falhas {}, repetidos entre pacotes {}",
        r.total_arquivos,
        mb(r.total_bytes),
        r.pacotes_lidos.len(),
        r.falhas_de_pacote.len(),
        r.repetidos_entre_pacotes.len()
    )
}

/// A versão 0 de `versao` a partir de `cliente` — o corpo do comando `pw-recursos importar` (`main.rs`), aqui para o
/// comando e para a API usarem o mesmo caminho.
#[allow(clippy::too_many_arguments)]
pub async fn importar_versao_zero(
    repo: &RecursosRepository,
    dados: &Path,
    chave: Arc<Chave>,
    fonte: Arc<dyn FonteDePacote>,
    versao: &str,
    cliente: &Path,
    nivel: i32,
    refazer: bool,
    origem: &str,
) -> Result<importar::Resultado> {
    importar::validar_versao(versao)?;
    if !repo.versoes(versao).await?.is_empty() {
        if !refazer {
            bail!("a versão {versao} já tem a versão 0 registrada (use refazer)");
        }
        if !repo.apagar_versao_zero(versao).await? {
            bail!("a versão {versao} já tem versões publicadas ou alterações: não dá para refazer a 0");
        }
    }
    let atualizacao = importar::pasta_da_versao(dados, versao).join("atualizacao");
    if refazer && atualizacao.exists() {
        std::fs::remove_dir_all(&atualizacao)?;
    }
    let (v, c, d) = (versao.to_string(), cliente.to_path_buf(), dados.to_path_buf());
    let resultado = tokio::task::spawn_blocking(move || {
        importar::importar(&Importacao {
            versao: &v,
            cliente: &c,
            dados: &d,
            nivel,
            linha_de_grupo: importar::precisa_linha_de_grupo(&v),
            fonte: fonte.as_ref(),
            chave: &chave,
        })
    })
    .await??;
    repo.gravar_versao_zero(
        versao,
        origem,
        &resultado.manifesto_sha256,
        &resultado.assinatura,
        &crate::para_o_banco(&resultado.registros),
    )
    .await?;
    Ok(resultado)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn caminho_nao_sai_da_pasta() {
        let p = Path::new("/x/e1");
        assert_eq!(caminho_seguro(p, "element/data/elements.data").unwrap(), p.join("element").join("data").join("elements.data"));
        assert_eq!(caminho_seguro(p, "element\\configs.pck").unwrap(), p.join("element").join("configs.pck"));
        for ruim in ["", "/abs", "../fora", "a/../../b", "a//b", "c:/x", "a/./b", "a/b/", "a\0b"] {
            assert!(caminho_seguro(p, ruim).is_err(), "{ruim:?}");
        }
    }

    #[test]
    fn bloco_so_acrescenta_e_retoma() {
        let pasta = std::env::temp_dir().join(format!("pw_envio_teste_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        assert_eq!(gravar_bloco(&pasta, "a/b.bin", 0, b"abc").unwrap(), Ok(3));
        assert_eq!(gravar_bloco(&pasta, "a/b.bin", 0, b"xyz").unwrap(), Err(3), "deslocamento velho: diz onde retomar");
        assert_eq!(gravar_bloco(&pasta, "a/b.bin", 3, b"def").unwrap(), Ok(6));
        assert_eq!(std::fs::read(pasta.join("a").join("b.bin")).unwrap(), b"abcdef");
        assert_eq!(listar(&pasta), vec![("a/b.bin".to_string(), 6)]);
        assert!(gravar_bloco(&pasta, "../fora", 0, b"x").is_err());
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
