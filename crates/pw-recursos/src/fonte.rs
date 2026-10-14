//! Leitura dos `.pck` pelo extrator já validado (`tools/pw-pck-extract/pw_pck_extract.py`,
//! 0 falhas nos dois clientes, B229) — o pw-recursos não tem leitor de `.pck` próprio. O
//! extrator entrega cada arquivo pela saída padrão no formato do subcomando `fluxo`
//! (nome em bytes GBK do índice + dados).

use anyhow::{bail, Context, Result};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// O que um pacote entregou: arquivos lidos e entradas corrompidas na origem (puladas).
#[derive(Debug, Default, Clone)]
pub struct ResultadoDoPacote {
    pub entradas: u64,
    pub falhas: Vec<(Vec<u8>, String)>,
}

/// Uma fonte de arquivos de `.pck`. Na produção, o extrator em Python; nos testes, uma lista.
pub trait FonteDePacote: Send + Sync {
    fn ler(
        &self,
        pck: &Path,
        receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>,
    ) -> Result<ResultadoDoPacote>;
}

pub struct ExtratorPython {
    pub python: String,
    pub script: PathBuf,
}

impl ExtratorPython {
    /// `PYTHON` (padrão `python3`, ou `python` no Windows) e `PW_PCK_EXTRACT` (padrão o caminho
    /// na imagem Docker, `/opt/pw-pck-extract/pw_pck_extract.py`).
    pub fn do_ambiente() -> Self {
        let python = std::env::var("PYTHON").unwrap_or_else(|_| {
            if cfg!(windows) { "python" } else { "python3" }.to_string()
        });
        let script = std::env::var("PW_PCK_EXTRACT")
            .unwrap_or_else(|_| "/opt/pw-pck-extract/pw_pck_extract.py".to_string());
        Self { python, script: script.into() }
    }
}

fn u32le<R: Read>(r: &mut R) -> Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

/// Interpreta o fluxo do subcomando `fluxo` (formato documentado em `cmd_fluxo`).
pub fn ler_fluxo<R: Read>(
    r: &mut R,
    receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>,
) -> Result<ResultadoDoPacote> {
    let mut resultado = ResultadoDoPacote::default();
    loop {
        let mut marca = [0u8; 1];
        r.read_exact(&mut marca).context("fluxo do extrator terminou sem a marca de fim")?;
        match marca[0] {
            b'E' => {
                let n = u32le(r)? as usize;
                let mut nome = vec![0u8; n];
                r.read_exact(&mut nome)?;
                let mut b = [0u8; 8];
                r.read_exact(&mut b)?;
                let mut dados = vec![0u8; u64::from_le_bytes(b) as usize];
                r.read_exact(&mut dados)?;
                resultado.entradas += 1;
                receber(nome, dados)?;
            }
            b'F' => {
                let n = u32le(r)? as usize;
                let mut nome = vec![0u8; n];
                r.read_exact(&mut nome)?;
                let m = u32le(r)? as usize;
                let mut erro = vec![0u8; m];
                r.read_exact(&mut erro)?;
                resultado.falhas.push((nome, String::from_utf8_lossy(&erro).into_owned()));
            }
            b'Z' => {
                let total = u32le(r)? as u64;
                if total != resultado.entradas + resultado.falhas.len() as u64 {
                    bail!(
                        "o extrator anunciou {total} entradas e entregou {}",
                        resultado.entradas + resultado.falhas.len() as u64
                    );
                }
                return Ok(resultado);
            }
            outra => bail!("marca desconhecida no fluxo do extrator: {outra:#04x}"),
        }
    }
}

impl FonteDePacote for ExtratorPython {
    fn ler(
        &self,
        pck: &Path,
        receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>,
    ) -> Result<ResultadoDoPacote> {
        let mut filho = Command::new(&self.python)
            .arg("-I")
            .arg(&self.script)
            .arg("fluxo")
            .arg(pck)
            .stdout(Stdio::piped())
            // Erros do extrator vão direto para o log (não ficam num cano que ninguém esvazia).
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("iniciar {} {}", self.python, self.script.display()))?;
        let mut saida = BufReader::with_capacity(1 << 20, filho.stdout.take().unwrap());
        let resultado = ler_fluxo(&mut saida, receber);
        drop(saida);
        let estado = filho.wait()?;
        let resultado = resultado.with_context(|| format!("ler {}", pck.display()))?;
        if !estado.success() {
            bail!("extrator saiu com {estado} em {}", pck.display());
        }
        Ok(resultado)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn fluxo_com_arquivo_falha_e_fim() {
        let mut f = Vec::new();
        f.push(b'E');
        f.extend(3u32.to_le_bytes());
        f.extend(b"a\\b");
        f.extend(2u64.to_le_bytes());
        f.extend(b"xy");
        f.push(b'F');
        f.extend(1u32.to_le_bytes());
        f.extend(b"c");
        f.extend(4u32.to_le_bytes());
        f.extend(b"zlib");
        f.push(b'Z');
        f.extend(2u32.to_le_bytes());
        let mut vistos = Vec::new();
        let r = ler_fluxo(&mut &f[..], &mut |n, d| {
            vistos.push((n, d));
            Ok(())
        })
        .unwrap();
        assert_eq!(vistos, vec![(b"a\\b".to_vec(), b"xy".to_vec())]);
        assert_eq!(r.entradas, 1);
        assert_eq!(r.falhas.len(), 1);
    }

    #[test]
    fn fluxo_cortado_e_erro() {
        assert!(ler_fluxo(&mut &b"E\x03\x00"[..], &mut |_, _| Ok(())).is_err());
    }
}
