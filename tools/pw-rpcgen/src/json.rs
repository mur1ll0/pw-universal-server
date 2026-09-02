//! Escritor de JSON mínimo.
//!
//! O `pw-rpcgen` não tem dependências externas de propósito (ver `Cargo.toml`), então
//! traz seu próprio serializador. O escopo é deliberadamente pequeno: só o suficiente
//! para emitir o IR de protocolo de forma estável e legível por humanos — a saída é
//! versionada no repositório e revisada em diffs.

use std::fmt::Write as _;

/// Valor JSON. Objetos preservam a ordem de inserção para que a saída seja estável
/// entre execuções e os diffs no repositório sejam legíveis.
#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Int(i64),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Self {
        Json::Str(s.into())
    }

    pub fn object(entries: impl IntoIterator<Item = (&'static str, Json)>) -> Self {
        Json::Object(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn array(items: impl IntoIterator<Item = Json>) -> Self {
        Json::Array(items.into_iter().collect())
    }

    /// Serializa com indentação de dois espaços.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Str(s) => write_quoted(out, s),
            Json::Array(items) if items.is_empty() => out.push_str("[]"),
            Json::Array(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent(out, depth);
                out.push(']');
            }
            Json::Object(entries) if entries.is_empty() => out.push_str("{}"),
            Json::Object(entries) => {
                out.push_str("{\n");
                for (i, (key, value)) in entries.iter().enumerate() {
                    indent(out, depth + 1);
                    write_quoted(out, key);
                    out.push_str(": ");
                    value.write(out, depth + 1);
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escreve_objeto_aninhado_com_ordem_preservada() {
        let value = Json::object([
            ("nome", Json::str("Challenge")),
            ("id", Json::Int(1)),
            (
                "campos",
                Json::array([Json::object([
                    ("nome", Json::str("nonce")),
                    ("tipo", Json::str("Octets")),
                ])]),
            ),
        ]);

        let expected = "\
{
  \"nome\": \"Challenge\",
  \"id\": 1,
  \"campos\": [
    {
      \"nome\": \"nonce\",
      \"tipo\": \"Octets\"
    }
  ]
}
";
        assert_eq!(value.to_pretty(), expected);
    }

    #[test]
    fn escapa_caracteres_especiais() {
        let value = Json::str("a\"b\\c\nd\te");
        assert_eq!(value.to_pretty(), "\"a\\\"b\\\\c\\nd\\te\"\n");
    }

    #[test]
    fn colecoes_vazias_sao_compactas() {
        assert_eq!(Json::Array(vec![]).to_pretty(), "[]\n");
        assert_eq!(Json::Object(vec![]).to_pretty(), "{}\n");
    }
}
