//! Leitor de `rpcalls.xml`.
//!
//! O XML é a declaração de origem do protocolo: dá os identificadores numéricos, os
//! limites de tamanho, as prioridades, os valores padrão dos campos e o mapeamento
//! RPC → (argumento, resultado). Ele **não** substitui os arquivos gerados em `inl/`,
//! que contêm protocolos ausentes do XML e são a autoridade sobre a ordem dos campos;
//! serve para enriquecer o que foi lido de lá.
//!
//! O parser cobre apenas o subconjunto que este arquivo usa: elementos, atributos,
//! aninhamento e comentários. Não há entidades customizadas, CDATA nem namespaces
//! em `rpcalls.xml`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Element>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    pub fn attr_int(&self, name: &str) -> Option<i64> {
        let text = self.attr(name)?.trim();
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            return i64::from_str_radix(hex, 16).ok();
        }
        text.parse().ok()
    }

    /// Filhos diretos com o nome dado.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
}

#[derive(Debug)]
pub struct XmlError {
    pub offset: usize,
    pub reason: String,
}

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.reason)
    }
}

/// Interpreta o documento e devolve o elemento raiz.
pub fn parse(text: &str) -> Result<Element, XmlError> {
    Parser {
        bytes: text.as_bytes(),
        pos: 0,
    }
    .document()
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn document(mut self) -> Result<Element, XmlError> {
        loop {
            self.skip_trivia();
            if self.starts_with("<?") {
                self.skip_until("?>")?;
                continue;
            }
            if self.starts_with("<!--") {
                self.skip_until("-->")?;
                continue;
            }
            if self.starts_with("<!") {
                self.skip_until(">")?;
                continue;
            }
            break;
        }
        self.element()
    }

    fn element(&mut self) -> Result<Element, XmlError> {
        self.expect("<")?;
        let name = self.name()?;
        let mut attrs = BTreeMap::new();

        loop {
            self.skip_whitespace();
            if self.starts_with("/>") {
                self.pos += 2;
                return Ok(Element { name, attrs, children: Vec::new() });
            }
            if self.starts_with(">") {
                self.pos += 1;
                break;
            }
            let key = self.name()?;
            self.skip_whitespace();
            self.expect("=")?;
            self.skip_whitespace();
            let value = self.quoted()?;
            attrs.insert(key, value);
        }

        let mut children = Vec::new();
        loop {
            self.skip_trivia();
            if self.starts_with("<!--") {
                self.skip_until("-->")?;
                continue;
            }
            if self.starts_with("</") {
                self.pos += 2;
                let closing = self.name()?;
                self.skip_whitespace();
                self.expect(">")?;
                if closing != name {
                    return Err(self.error(format!("</{closing}> fecha <{name}>")));
                }
                return Ok(Element { name, attrs, children });
            }
            if self.starts_with("<") {
                children.push(self.element()?);
                continue;
            }
            if self.pos >= self.bytes.len() {
                return Err(self.error(format!("<{name}> não foi fechado")));
            }
            // Texto solto entre elementos: `rpcalls.xml` não o usa, então é descartado.
            self.pos += 1;
        }
    }

    fn name(&mut self) -> Result<String, XmlError> {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b':' || c == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(self.error("esperava um nome".to_string()));
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }

    fn quoted(&mut self) -> Result<String, XmlError> {
        let quote = *self
            .bytes
            .get(self.pos)
            .ok_or_else(|| self.error("esperava um valor entre aspas".to_string()))?;
        if quote != b'"' && quote != b'\'' {
            return Err(self.error("esperava um valor entre aspas".to_string()));
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos] != quote {
            self.pos += 1;
        }
        if self.pos >= self.bytes.len() {
            return Err(self.error("aspas não fechadas".to_string()));
        }
        let raw = String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned();
        self.pos += 1;
        Ok(unescape(&raw))
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn skip_trivia(&mut self) {
        self.skip_whitespace();
    }

    fn skip_until(&mut self, marker: &str) -> Result<(), XmlError> {
        match self.bytes[self.pos..]
            .windows(marker.len())
            .position(|w| w == marker.as_bytes())
        {
            Some(at) => {
                self.pos += at + marker.len();
                Ok(())
            }
            None => Err(self.error(format!("não encontrei `{marker}`"))),
        }
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.bytes[self.pos..].starts_with(prefix.as_bytes())
    }

    fn expect(&mut self, token: &str) -> Result<(), XmlError> {
        if self.starts_with(token) {
            self.pos += token.len();
            Ok(())
        } else {
            Err(self.error(format!("esperava `{token}`")))
        }
    }

    fn error(&self, reason: String) -> XmlError {
        XmlError { offset: self.pos, reason }
    }
}

fn unescape(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    raw.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_elementos_atributos_e_aninhamento() {
        let doc = parse(
            r#"<?xml version="1.0"?>
            <application namespace="GNET">
              <!-- comentário -->
              <protocol name="Challenge" type="1" maxsize="64">
                <variable name="nonce" type="Octets"/>
              </protocol>
            </application>"#,
        )
        .unwrap();

        assert_eq!(doc.name, "application");
        assert_eq!(doc.attr("namespace"), Some("GNET"));

        let protocol = doc.children_named("protocol").next().unwrap();
        assert_eq!(protocol.attr("name"), Some("Challenge"));
        assert_eq!(protocol.attr_int("type"), Some(1));
        assert_eq!(protocol.children.len(), 1);
        assert_eq!(protocol.children[0].attr("name"), Some("nonce"));
    }

    #[test]
    fn desfaz_entidades_em_tipos_genericos() {
        let doc = parse(r#"<r><variable type="std::vector&lt;int&gt;"/></r>"#).unwrap();
        assert_eq!(doc.children[0].attr("type"), Some("std::vector<int>"));
    }

    #[test]
    fn aceita_hexadecimal_em_atributo_numerico() {
        let doc = parse(r#"<r><p type="0x53"/></r>"#).unwrap();
        assert_eq!(doc.children[0].attr_int("type"), Some(0x53));
    }

    #[test]
    fn tag_de_fechamento_trocada_e_erro() {
        let err = parse("<a><b></c></a>").unwrap_err();
        assert!(err.reason.contains("fecha"));
    }

    #[test]
    fn elemento_nao_fechado_e_erro() {
        assert!(parse("<a><b/>").is_err());
    }
}
