//! Modelo de tipos do protocolo GNET e tradução a partir das declarações C++.
//!
//! As regras de codificação vêm de `share/common/marshal_i386.h` e
//! `share/common/byteorder_i386.h` dos fontes originais:
//!
//! * Todo escalar vai para o fio em **big-endian** (`byteorder_32` é `bswap` em host
//!   little-endian). `float`/`double` também, via bitcast para inteiro.
//! * `Octets` e `std::string` = `CompactUINT(len)` seguido dos bytes.
//! * Contêineres (`std::vector`, `std::set`, `std::list`, `std::deque`, `std::map` e
//!   `GNET::RpcDataVector`) = `CompactUINT(count)` seguido dos elementos.
//! * `std::pair` = os dois elementos em sequência, sem prefixo.

use crate::json::Json;
use std::collections::BTreeMap;

/// Escalares primitivos, nomeados pela largura e sinal — não pelo alias C++ de origem
/// (`long` em i386 tem 32 bits, `int64_t` tem 64, e assim por diante).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prim {
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Prim {
    pub fn as_str(self) -> &'static str {
        match self {
            Prim::Bool => "bool",
            Prim::I8 => "i8",
            Prim::U8 => "u8",
            Prim::I16 => "i16",
            Prim::U16 => "u16",
            Prim::I32 => "i32",
            Prim::U32 => "u32",
            Prim::I64 => "i64",
            Prim::U64 => "u64",
            Prim::F32 => "f32",
            Prim::F64 => "f64",
        }
    }

    /// Quantidade de bytes que o escalar ocupa no fio.
    pub fn wire_size(self) -> usize {
        match self {
            Prim::Bool | Prim::I8 | Prim::U8 => 1,
            Prim::I16 | Prim::U16 => 2,
            Prim::I32 | Prim::U32 | Prim::F32 => 4,
            Prim::I64 | Prim::U64 | Prim::F64 => 8,
        }
    }
}

/// Tipo de um campo do protocolo, já resolvido para a forma como aparece no fio.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ty {
    Prim(Prim),
    /// Sequência de bytes crus com prefixo `CompactUINT` de tamanho.
    Octets,
    /// Cadeia de bytes com prefixo `CompactUINT` de tamanho. Sem terminador nulo.
    Str,
    /// `CompactUINT(count)` seguido dos elementos.
    Seq(Box<Ty>),
    /// `CompactUINT(count)` seguido de pares chave/valor.
    Map(Box<Ty>, Box<Ty>),
    /// Dois elementos em sequência, sem prefixo de contagem.
    Pair(Box<Ty>, Box<Ty>),
    /// Referência a uma estrutura declarada em `rpcdata/`.
    Struct(String),
    /// Tipo que o parser não soube resolver. Mantido no IR com o texto original para
    /// que a divergência apareça na revisão em vez de virar um campo silenciosamente
    /// errado.
    Unresolved(String),
}

impl Ty {
    pub fn to_json(&self) -> Json {
        match self {
            Ty::Prim(p) => Json::object([
                ("kind", Json::str("prim")),
                ("prim", Json::str(p.as_str())),
                ("bytes", Json::Int(p.wire_size() as i64)),
            ]),
            Ty::Octets => Json::object([("kind", Json::str("octets"))]),
            Ty::Str => Json::object([("kind", Json::str("string"))]),
            Ty::Seq(inner) => Json::object([("kind", Json::str("seq")), ("item", inner.to_json())]),
            Ty::Map(k, v) => Json::object([
                ("kind", Json::str("map")),
                ("key", k.to_json()),
                ("value", v.to_json()),
            ]),
            Ty::Pair(a, b) => Json::object([
                ("kind", Json::str("pair")),
                ("first", a.to_json()),
                ("second", b.to_json()),
            ]),
            Ty::Struct(name) => {
                Json::object([("kind", Json::str("struct")), ("name", Json::str(name.clone()))])
            }
            Ty::Unresolved(raw) => Json::object([
                ("kind", Json::str("unresolved")),
                ("cxx", Json::str(raw.clone())),
            ]),
        }
    }

    pub fn is_unresolved(&self) -> bool {
        match self {
            Ty::Unresolved(_) => true,
            Ty::Seq(inner) => inner.is_unresolved(),
            Ty::Map(k, v) => k.is_unresolved() || v.is_unresolved(),
            Ty::Pair(a, b) => a.is_unresolved() || b.is_unresolved(),
            _ => false,
        }
    }
}

/// Tabela de apelidos de tipo (`typedef`) coletada dos fontes, usada para resolver
/// nomes como `RoleInfoVector` ou `IntVector`.
#[derive(Debug, Default, Clone)]
pub struct TypeAliases {
    aliases: BTreeMap<String, Ty>,
}

impl TypeAliases {
    /// Apelidos declarados em `share/rpc/rpcdefs.h`, que não aparecem em nenhum
    /// arquivo gerado e por isso precisam ser conhecidos de antemão.
    pub fn with_builtins() -> Self {
        let mut this = Self::default();
        let builtins: &[(&str, Ty)] = &[
            ("CharVector", Ty::Seq(Box::new(Ty::Prim(Prim::I8)))),
            ("ByteVector", Ty::Seq(Box::new(Ty::Prim(Prim::U8)))),
            ("ShortVector", Ty::Seq(Box::new(Ty::Prim(Prim::I16)))),
            ("WordVector", Ty::Seq(Box::new(Ty::Prim(Prim::U16)))),
            ("IntVector", Ty::Seq(Box::new(Ty::Prim(Prim::I32)))),
            ("UintVector", Ty::Seq(Box::new(Ty::Prim(Prim::U32)))),
            ("Int64Vector", Ty::Seq(Box::new(Ty::Prim(Prim::I64)))),
            ("OctetsVector", Ty::Seq(Box::new(Ty::Octets))),
            ("IntOctetsVector", Ty::Seq(Box::new(Ty::Struct("IntOctets".into())))),
        ];
        for (name, ty) in builtins {
            this.aliases.insert((*name).to_string(), ty.clone());
        }
        this
    }

    pub fn insert(&mut self, alias: impl Into<String>, ty: Ty) {
        self.aliases.insert(alias.into(), ty);
    }

    pub fn get(&self, alias: &str) -> Option<&Ty> {
        self.aliases.get(alias)
    }
}

/// Traduz uma declaração de tipo C++ para o modelo de fio.
///
/// `known_structs` decide se um nome desconhecido vira `Struct` (declarado em
/// `rpcdata/`) ou `Unresolved` (a ser revisado à mão).
pub fn parse_cxx_type(raw: &str, aliases: &TypeAliases, known_structs: &dyn Fn(&str) -> bool) -> Ty {
    let text = normalize(raw);

    if let Some(ty) = parse_primitive(&text) {
        return Ty::Prim(ty);
    }

    match text.as_str() {
        "Octets" | "GNET::Octets" => return Ty::Octets,
        "std::string" | "string" => return Ty::Str,
        _ => {}
    }

    if let Some(args) = generic_args(&text, &["std::vector", "vector", "std::set", "set", "std::list", "list", "std::deque", "deque", "GNET::RpcDataVector", "RpcDataVector"]) {
        if args.len() == 1 {
            return Ty::Seq(Box::new(parse_cxx_type(&args[0], aliases, known_structs)));
        }
    }

    if let Some(args) = generic_args(&text, &["std::map", "map", "std::multimap"]) {
        if args.len() == 2 {
            return Ty::Map(
                Box::new(parse_cxx_type(&args[0], aliases, known_structs)),
                Box::new(parse_cxx_type(&args[1], aliases, known_structs)),
            );
        }
    }

    if let Some(args) = generic_args(&text, &["std::pair", "pair"]) {
        if args.len() == 2 {
            return Ty::Pair(
                Box::new(parse_cxx_type(&args[0], aliases, known_structs)),
                Box::new(parse_cxx_type(&args[1], aliases, known_structs)),
            );
        }
    }

    let bare = text.rsplit("::").next().unwrap_or(&text).to_string();

    if let Some(ty) = aliases.get(&bare) {
        return ty.clone();
    }

    if known_structs(&bare) {
        return Ty::Struct(bare);
    }

    Ty::Unresolved(raw.trim().to_string())
}

fn parse_primitive(text: &str) -> Option<Prim> {
    // `long` e `unsigned long` têm 32 bits no alvo i386 do servidor original, que é o
    // que define o formato de fio. Tratá-los como 64 bits quebraria o alinhamento.
    Some(match text {
        "bool" => Prim::Bool,
        "char" | "signed char" | "int8_t" => Prim::I8,
        "unsigned char" | "uint8_t" | "byte" => Prim::U8,
        "short" | "signed short" | "short int" | "int16_t" => Prim::I16,
        "unsigned short" | "unsigned short int" | "uint16_t" => Prim::U16,
        "int" | "signed int" | "long" | "long int" | "int32_t" => Prim::I32,
        // `size_t` e `time_t` têm 32 bits no alvo i386 do servidor original, que é o
        // que define a largura no fio.
        "unsigned" | "unsigned int" | "unsigned long" | "unsigned long int" | "uint32_t"
        | "size_t" => Prim::U32,
        "time_t" => Prim::I32,
        "long long" | "long long int" | "int64_t" => Prim::I64,
        "unsigned long long" | "unsigned long long int" | "uint64_t" => Prim::U64,
        "float" => Prim::F32,
        "double" => Prim::F64,
        _ => return None,
    })
}

/// Se `text` for uma instanciação de um dos templates dados, devolve seus argumentos.
fn generic_args(text: &str, templates: &[&str]) -> Option<Vec<String>> {
    let open = text.find('<')?;
    if !text.ends_with('>') {
        return None;
    }
    let head = text[..open].trim();
    if !templates.contains(&head) {
        return None;
    }
    Some(split_top_level(&text[open + 1..text.len() - 1]))
}

/// Divide por vírgulas que estão no nível mais externo de `<>`.
fn split_top_level(args: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in args.chars() {
        match ch {
            '<' => {
                depth += 1;
                current.push(ch);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    let last = current.trim();
    if !last.is_empty() {
        parts.push(last.to_string());
    }
    parts
}

/// Normaliza espaços e remove qualificadores que não afetam o formato de fio.
fn normalize(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    for qualifier in ["const ", "volatile ", "typename ", "struct ", "class "] {
        text = text.replace(qualifier, "");
    }
    text = text.replace('&', " ").replace('\t', " ");
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    // `std::vector< int >` e `std::vector<int>` devem convergir.
    out.replace("< ", "<").replace(" >", ">").replace(" <", "<").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_structs(_: &str) -> bool {
        false
    }

    fn aliases() -> TypeAliases {
        TypeAliases::with_builtins()
    }

    #[test]
    fn primitivos_seguem_a_largura_do_alvo_i386() {
        let a = aliases();
        assert_eq!(parse_cxx_type("int", &a, &no_structs), Ty::Prim(Prim::I32));
        assert_eq!(parse_cxx_type("unsigned int", &a, &no_structs), Ty::Prim(Prim::U32));
        // `long` em i386 tem 32 bits, e é a largura de fio que vale.
        assert_eq!(parse_cxx_type("unsigned long", &a, &no_structs), Ty::Prim(Prim::U32));
        assert_eq!(parse_cxx_type("int64_t", &a, &no_structs), Ty::Prim(Prim::I64));
        assert_eq!(parse_cxx_type("unsigned char", &a, &no_structs), Ty::Prim(Prim::U8));
        assert_eq!(parse_cxx_type("char", &a, &no_structs), Ty::Prim(Prim::I8));
        assert_eq!(parse_cxx_type("float", &a, &no_structs), Ty::Prim(Prim::F32));
    }

    #[test]
    fn octets_e_string_sao_distintos() {
        let a = aliases();
        assert_eq!(parse_cxx_type("Octets", &a, &no_structs), Ty::Octets);
        assert_eq!(parse_cxx_type("const Octets&", &a, &no_structs), Ty::Octets);
        assert_eq!(parse_cxx_type("std::string", &a, &no_structs), Ty::Str);
    }

    #[test]
    fn conteineres_viram_sequencias() {
        let a = aliases();
        assert_eq!(
            parse_cxx_type("std::vector<int>", &a, &no_structs),
            Ty::Seq(Box::new(Ty::Prim(Prim::I32)))
        );
        assert_eq!(
            parse_cxx_type("std::vector< Octets >", &a, &no_structs),
            Ty::Seq(Box::new(Ty::Octets))
        );
        assert_eq!(
            parse_cxx_type("std::vector<std::vector<int> >", &a, &no_structs),
            Ty::Seq(Box::new(Ty::Seq(Box::new(Ty::Prim(Prim::I32)))))
        );
    }

    #[test]
    fn map_e_pair_preservam_os_dois_argumentos() {
        let a = aliases();
        assert_eq!(
            parse_cxx_type("std::map<int,Octets>", &a, &no_structs),
            Ty::Map(Box::new(Ty::Prim(Prim::I32)), Box::new(Ty::Octets))
        );
        assert_eq!(
            parse_cxx_type("std::pair<int, unsigned char>", &a, &no_structs),
            Ty::Pair(Box::new(Ty::Prim(Prim::I32)), Box::new(Ty::Prim(Prim::U8)))
        );
    }

    #[test]
    fn apelidos_embutidos_do_rpcdefs_resolvem() {
        let a = aliases();
        assert_eq!(
            parse_cxx_type("IntVector", &a, &no_structs),
            Ty::Seq(Box::new(Ty::Prim(Prim::I32)))
        );
        assert_eq!(
            parse_cxx_type("ByteVector", &a, &no_structs),
            Ty::Seq(Box::new(Ty::Prim(Prim::U8)))
        );
    }

    #[test]
    fn apelido_de_vetor_de_rpcdata_resolve_para_sequencia_de_struct() {
        let mut a = aliases();
        a.insert("RoleInfoVector", Ty::Seq(Box::new(Ty::Struct("RoleInfo".into()))));
        assert_eq!(
            parse_cxx_type("RoleInfoVector", &a, &no_structs),
            Ty::Seq(Box::new(Ty::Struct("RoleInfo".into())))
        );
    }

    #[test]
    fn nome_conhecido_vira_struct_e_desconhecido_vira_unresolved() {
        let a = aliases();
        let known = |n: &str| n == "GRoleBase";
        assert_eq!(
            parse_cxx_type("GRoleBase", &a, &known),
            Ty::Struct("GRoleBase".into())
        );
        assert!(matches!(
            parse_cxx_type("SomethingElse", &a, &known),
            Ty::Unresolved(_)
        ));
    }

    #[test]
    fn namespace_e_ignorado_na_resolucao() {
        let a = aliases();
        let known = |n: &str| n == "GRoleBase";
        assert_eq!(
            parse_cxx_type("GNET::GRoleBase", &a, &known),
            Ty::Struct("GRoleBase".into())
        );
    }
}
