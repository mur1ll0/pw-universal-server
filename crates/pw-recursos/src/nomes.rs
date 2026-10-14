//! Nomes de arquivo do cliente: bytes GBK ↔ nome no disco.
//!
//! O cliente monta cada caminho com os **bytes GBK** que vêm dos dados e abre o arquivo solto
//! com `fopen` ANSI (`AngelicaFile/Source/AFileImage.cpp:127`); o Windows converte esses bytes
//! pela página de código ANSI do sistema (ACP). Por isso o pw-recursos guarda e distribui o
//! nome em bytes GBK, e só na máquina que extrai se calcula o nome no disco
//! (`RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` §7.7).

use anyhow::{bail, Result};
use std::path::PathBuf;

/// Bytes GBK de um nome Unicode (arquivo solto da instalação). `None` se algum caractere não
/// existe no GBK — caso dos `Padrão (N).ini` que o próprio cliente gravou nesta máquina pela
/// ACP 1252: não há bytes GBK que o cliente usaria para achá-los.
pub fn gbk_de_unicode(nome: &str) -> Option<Vec<u8>> {
    let (bytes, _, com_erro) = encoding_rs::GBK.encode(nome);
    if com_erro {
        None
    } else {
        Some(bytes.into_owned())
    }
}

/// Texto legível de um nome GBK (relatórios e logs; nunca para gravar no disco).
pub fn legivel(gbk: &[u8]) -> String {
    encoding_rs::GBK.decode_without_bom_handling(gbk).0.into_owned()
}

/// Chave de comparação: o Windows não diferencia maiúsculas, então `Models\x` do pacote e
/// `models\X` solto são o mesmo arquivo para o cliente.
pub fn chave(gbk: &[u8]) -> String {
    legivel(gbk).to_lowercase()
}

/// Normaliza um nome vindo do índice do `.pck`: separador `\` (o índice pode trazer `/`, que
/// nunca é segundo byte de GBK — segundos bytes ficam em 0x40..0xFE).
pub fn normalizar(gbk: &[u8]) -> Vec<u8> {
    gbk.iter().map(|&b| if b == b'/' { b'\\' } else { b }).collect()
}

/// Caminho relativo no disco **desta máquina** para um nome GBK, como o `fopen` do cliente o
/// abriria. No Windows: `MultiByteToWideChar(CP_ACP)` — a mesma conversão do `fopen` ANSI — e
/// os `\` resultantes são separadores (inclusive um segundo byte GBK 0x5C numa ACP de um byte:
/// o `fopen` do cliente faz o mesmo). Fora do Windows, o nome GBK decodificado (só inspeção).
pub fn nome_no_disco(gbk: &[u8]) -> Result<PathBuf> {
    let texto = converter_pela_acp(gbk)?;
    let mut caminho = PathBuf::new();
    for parte in texto.split(['\\', '/']) {
        if parte.is_empty() || parte == "." || parte == ".." || parte.contains(':') {
            bail!("nome inseguro ou vazio no manifesto: {}", legivel(gbk));
        }
        caminho.push(parte);
    }
    if caminho.as_os_str().is_empty() {
        bail!("nome vazio no manifesto");
    }
    Ok(caminho)
}

#[cfg(windows)]
fn converter_pela_acp(gbk: &[u8]) -> Result<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            flags: u32,
            origem: *const u8,
            tamanho_origem: i32,
            destino: *mut u16,
            tamanho_destino: i32,
        ) -> i32;
    }
    const CP_ACP: u32 = 0;
    const MB_ERR_INVALID_CHARS: u32 = 0x8;
    if gbk.is_empty() {
        return Ok(String::new());
    }
    // SAFETY: ponteiros e tamanhos vêm de fatias válidas; a 1ª chamada só mede.
    let n = unsafe {
        MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS, gbk.as_ptr(), gbk.len() as i32, std::ptr::null_mut(), 0)
    };
    if n <= 0 {
        bail!("a página de código deste Windows não converte o nome {}", legivel(gbk));
    }
    let mut largo = vec![0u16; n as usize];
    let m = unsafe {
        MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS, gbk.as_ptr(), gbk.len() as i32, largo.as_mut_ptr(), n)
    };
    if m != n {
        bail!("falha ao converter o nome {}", legivel(gbk));
    }
    String::from_utf16(&largo).map_err(|_| anyhow::anyhow!("nome sem UTF-16 válido: {}", legivel(gbk)))
}

#[cfg(not(windows))]
fn converter_pela_acp(gbk: &[u8]) -> Result<String> {
    Ok(legivel(gbk))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ida_e_volta_gbk_do_nome_chines() {
        // `形象` = D0 CE CF F3 (exemplo do §7.7).
        let gbk = gbk_de_unicode("models\\players\\形象").unwrap();
        assert_eq!(&gbk[15..], &[0xD0, 0xCE, 0xCF, 0xF3]);
        assert_eq!(legivel(&gbk), "models\\players\\形象");
    }

    #[test]
    fn nome_fora_do_gbk_e_recusado() {
        assert!(gbk_de_unicode("userdata\\Padrão (1).ini").is_none());
    }

    #[test]
    fn chave_ignora_caixa() {
        assert_eq!(chave(b"Models\\A.ECM"), chave(b"models\\a.ecm"));
    }

    #[test]
    fn nome_inseguro_e_recusado() {
        assert!(nome_no_disco(b"..\\fora.txt").is_err());
        assert!(nome_no_disco(b"c:\\x").is_err());
        assert!(nome_no_disco(b"configs\\a.txt").is_ok());
    }
}
