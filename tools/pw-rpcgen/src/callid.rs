//! Leitor dos `callid.hxx`, que dão os identificadores numéricos de protocolo e RPC.
//!
//! Cada daemon tem o seu, contendo apenas os identificadores que aquele daemon usa.
//! Os valores são consistentes entre daemons — o mesmo `PROTOCOL_ROLELIST_RE` tem o
//! mesmo número em `glinkd` e em `gdeliveryd` —, então a leitura combina todos os
//! arquivos e trata uma divergência de valor como erro, não como sobrescrita.

use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct CallIds {
    pub protocols: BTreeMap<String, i64>,
    pub rpcs: BTreeMap<String, i64>,
    /// Daemons em que cada símbolo aparece, para o IR registrar quem fala o quê.
    pub daemons: BTreeMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct Conflict {
    pub symbol: String,
    pub existing: i64,
    pub found: i64,
    pub daemon: String,
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} vale {} em {}, mas já havia sido lido como {}",
            self.symbol, self.found, self.daemon, self.existing
        )
    }
}

impl CallIds {
    /// Incorpora um `callid.hxx`. `daemon` é o nome do daemon dono do arquivo.
    pub fn absorb(&mut self, text: &str, daemon: &str) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for (symbol, value) in parse_enum_entries(text) {
            let table = if symbol.starts_with("PROTOCOL_") {
                &mut self.protocols
            } else if symbol.starts_with("RPC_") {
                &mut self.rpcs
            } else {
                continue;
            };

            match table.get(&symbol) {
                Some(&existing) if existing != value => conflicts.push(Conflict {
                    symbol: symbol.clone(),
                    existing,
                    found: value,
                    daemon: daemon.to_string(),
                }),
                Some(_) => {}
                None => {
                    table.insert(symbol.clone(), value);
                }
            }

            let owners = self.daemons.entry(symbol).or_default();
            if !owners.iter().any(|d| d == daemon) {
                owners.push(daemon.to_string());
            }
        }

        conflicts
    }
}

/// Extrai todos os pares `NOME = valor` do texto.
///
/// Varre por tokens em vez de por linhas: os `callid.hxx` originais escrevem uma
/// entrada por linha, mas nada no C++ obriga isso, e um `enum` compactado em uma linha
/// só não pode fazer identificadores desaparecerem em silêncio. Aceita decimal e
/// hexadecimal, e ignora comentários de linha e de bloco.
fn parse_enum_entries(text: &str) -> Vec<(String, i64)> {
    let cleaned = strip_comments(text);
    let bytes = cleaned.as_bytes();
    let mut entries = Vec::new();
    let mut pos = 0usize;

    while pos < bytes.len() {
        if bytes[pos] != b'=' {
            pos += 1;
            continue;
        }
        // `==`, `<=`, `>=` e `!=` não são atribuições de enum.
        let is_comparison = matches!(bytes.get(pos + 1), Some(b'='))
            || matches!(pos.checked_sub(1).and_then(|p| bytes.get(p)), Some(b'<' | b'>' | b'!' | b'='));
        if is_comparison {
            pos += 1;
            continue;
        }

        let symbol = identifier_before(bytes, pos);
        let (value, next) = integer_after(&cleaned, pos + 1);
        pos = next.max(pos + 1);

        if let (Some(symbol), Some(value)) = (symbol, value) {
            entries.push((symbol, value));
        }
    }

    entries
}

/// Último identificador que termina antes de `at`, ignorando espaços.
fn identifier_before(bytes: &[u8], at: usize) -> Option<String> {
    let mut end = at;
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    if start == end {
        return None;
    }
    // Um identificador não começa com dígito.
    if bytes[start].is_ascii_digit() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}

/// Primeiro inteiro a partir de `from`, junto da posição logo após ele.
fn integer_after(text: &str, from: usize) -> (Option<i64>, usize) {
    let bytes = text.as_bytes();
    let mut start = from;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    let negative = matches!(bytes.get(start), Some(b'-'));
    if negative {
        start += 1;
    }
    let mut end = start;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || (end == start + 1 && bytes[end] == b'x'))
    {
        end += 1;
    }
    if start == end {
        return (None, from);
    }
    let digits = String::from_utf8_lossy(&bytes[start..end]);
    let value = parse_int(&digits).map(|v| if negative { -v } else { v });
    (value, end)
}

/// Remove comentários de linha e de bloco, preservando o comprimento das quebras de
/// linha o suficiente para que o texto continue legível em depuração.
fn strip_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut pos = 0usize;

    while pos < bytes.len() {
        if bytes[pos..].starts_with(b"//") {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }
        if bytes[pos..].starts_with(b"/*") {
            pos += 2;
            while pos < bytes.len() && !bytes[pos..].starts_with(b"*/") {
                if bytes[pos] == b'\n' {
                    out.push('\n');
                }
                pos += 1;
            }
            pos = (pos + 2).min(bytes.len());
            continue;
        }
        out.push(bytes[pos] as char);
        pos += 1;
    }

    out
}

fn parse_int(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
namespace GNET
{
enum CallID
{
	RPC_GQUERYPASSWD	=	502,
	RPC_USERLOGIN	=	15,
	PROTOCOL_CHALLENGE = 1,       // handshake inicial
	PROTOCOL_ROLELIST_RE = 0x53,
	SOMETHING_ELSE = 7,
};
};
"#;

    #[test]
    fn separa_protocolos_de_rpcs_e_ignora_o_resto() {
        let mut ids = CallIds::default();
        let conflicts = ids.absorb(SAMPLE, "gdeliveryd");

        assert!(conflicts.is_empty());
        assert_eq!(ids.protocols.get("PROTOCOL_CHALLENGE"), Some(&1));
        assert_eq!(ids.protocols.get("PROTOCOL_ROLELIST_RE"), Some(&0x53));
        assert_eq!(ids.rpcs.get("RPC_USERLOGIN"), Some(&15));
        assert!(!ids.protocols.contains_key("SOMETHING_ELSE"));
        assert!(!ids.rpcs.contains_key("SOMETHING_ELSE"));
    }

    #[test]
    fn registra_todos_os_daemons_que_declaram_o_simbolo() {
        let mut ids = CallIds::default();
        ids.absorb(SAMPLE, "gdeliveryd");
        ids.absorb(SAMPLE, "glinkd");
        assert_eq!(
            ids.daemons.get("PROTOCOL_CHALLENGE").map(Vec::as_slice),
            Some(["gdeliveryd".to_string(), "glinkd".to_string()].as_slice())
        );
    }

    #[test]
    fn enum_compactado_em_uma_linha_nao_perde_entradas() {
        let mut ids = CallIds::default();
        ids.absorb(
            "enum CallID { PROTOCOL_A = 1, PROTOCOL_B = 2, RPC_C = 0x1F, };",
            "glinkd",
        );
        assert_eq!(ids.protocols.get("PROTOCOL_A"), Some(&1));
        assert_eq!(ids.protocols.get("PROTOCOL_B"), Some(&2));
        assert_eq!(ids.rpcs.get("RPC_C"), Some(&0x1F));
    }

    #[test]
    fn comentarios_de_bloco_sao_removidos() {
        let mut ids = CallIds::default();
        ids.absorb(
            "enum X {\n PROTOCOL_A = 1,\n /* PROTOCOL_OCULTO = 99, */\n PROTOCOL_B = 2,\n};",
            "glinkd",
        );
        assert_eq!(ids.protocols.get("PROTOCOL_A"), Some(&1));
        assert_eq!(ids.protocols.get("PROTOCOL_B"), Some(&2));
        assert!(!ids.protocols.contains_key("PROTOCOL_OCULTO"));
    }

    #[test]
    fn valor_divergente_entre_daemons_vira_conflito_e_nao_sobrescreve() {
        let mut ids = CallIds::default();
        ids.absorb(SAMPLE, "gdeliveryd");
        let conflicts = ids.absorb("enum X { PROTOCOL_CHALLENGE = 99, };", "glinkd");

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].symbol, "PROTOCOL_CHALLENGE");
        assert_eq!(conflicts[0].existing, 1);
        assert_eq!(conflicts[0].found, 99);
        // O primeiro valor lido permanece; o conflito é reportado para revisão.
        assert_eq!(ids.protocols.get("PROTOCOL_CHALLENGE"), Some(&1));
    }
}
