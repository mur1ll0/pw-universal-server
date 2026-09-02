//! Leitor dos arquivos C++ gerados pelo `rpcgen` original (`inl/` e `rpcdata/`).
//!
//! Esses arquivos são a fonte canônica do formato de fio: o corpo de `marshal` dá a
//! **ordem** exata dos campos e as declarações de membro dão os **tipos**. O
//! `rpcalls.xml` é a declaração de origem, mas nem todo protocolo está lá — `Challenge`
//! e `Response`, por exemplo, só existem em `inl/`. Por isso os arquivos gerados são
//! tratados como autoridade e o XML apenas os enriquece.

use crate::ty::{parse_cxx_type, Ty, TypeAliases};
use std::collections::BTreeMap;

/// Um item que o `marshal` escreve no fluxo, na ordem em que escreve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// Um membro declarado da classe.
    Field { name: String, cxx_type: String },
    /// Um literal escrito diretamente, como o byte de versão `os << (char)(1);` que
    /// prefixa `GRoleBase`. Precisa aparecer no IR ou o registro sai desalinhado.
    Literal { cxx: String },
}

/// Uma classe de protocolo ou de dados, como extraída dos fontes gerados.
#[derive(Debug, Clone)]
pub struct CxxClass {
    pub name: String,
    /// Nome do arquivo de origem, para rastreabilidade no IR.
    pub source: String,
    /// Identificador simbólico do protocolo (`PROTOCOL_CHALLENGE`), quando houver.
    pub protocol_symbol: Option<String>,
    pub prior_policy: Option<i64>,
    pub size_limit: Option<i64>,
    /// Itens na ordem em que `marshal` os escreve.
    pub items: Vec<Item>,
    /// `typedef GNET::RpcDataVector<X> XVector;` declarado junto da classe.
    pub vector_alias: Option<String>,
}

/// Erro de leitura de um arquivo gerado. Traz o arquivo para que a mensagem seja
/// acionável sem precisar procurar.
#[derive(Debug)]
pub struct ParseError {
    pub source: String,
    pub reason: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.source, self.reason)
    }
}

/// Extrai a classe descrita por um arquivo de `inl/` ou `rpcdata/`.
///
/// `source` é o nome do arquivo, usado tanto para mensagens de erro quanto como
/// último recurso para descobrir o nome da classe.
pub fn parse_class(text: &str, source: &str) -> Result<CxxClass, ParseError> {
    let declarations = collect_member_declarations(text);
    let items = parse_marshal_body(text, &declarations).ok_or_else(|| ParseError {
        source: source.to_string(),
        reason: "não encontrei um corpo de `marshal` no arquivo".to_string(),
    })?;

    let name = class_name(text).ok_or_else(|| ParseError {
        source: source.to_string(),
        reason: "não consegui determinar o nome da classe".to_string(),
    })?;

    Ok(CxxClass {
        name,
        source: source.to_string(),
        protocol_symbol: protocol_symbol(text),
        prior_policy: capture_number(text, "PriorPolicy"),
        size_limit: capture_number(text, "SizePolicy"),
        items,
        vector_alias: vector_alias(text),
    })
}

/// Vínculo de um RPC com seu identificador e suas políticas.
///
/// Em `inl/`, um RPC não tem `marshal` próprio — quem vai para o fio são as structs de
/// argumento e resultado. O arquivo carrega o símbolo `RPC_*` e os limites, que
/// enriquecem a entrada declarada no `rpcalls.xml`.
#[derive(Debug, Clone)]
pub struct CxxRpcBinding {
    pub name: String,
    pub source: String,
    pub symbol: String,
    pub prior_policy: Option<i64>,
    pub size_limit: Option<i64>,
    pub time_limit: Option<i64>,
}

/// Lê um arquivo de `inl/` que descreve um RPC em vez de um protocolo.
///
/// Devolve `None` para qualquer outra forma, inclusive protocolos — que se distinguem
/// por terem símbolo `PROTOCOL_*` e um corpo de `marshal`.
pub fn parse_rpc_binding(text: &str, source: &str) -> Option<CxxRpcBinding> {
    let symbol = protocol_symbol(text)?;
    if !symbol.starts_with("RPC_") {
        return None;
    }
    Some(CxxRpcBinding {
        name: class_name(text)?,
        source: source.to_string(),
        symbol,
        prior_policy: capture_number(text, "PriorPolicy"),
        size_limit: capture_number(text, "SizePolicy"),
        time_limit: capture_number(text, "TimePolicy"),
    })
}

/// Extrai todas as classes de um cabeçalho que declara várias, como
/// `share/rpc/rpcdefs.h` (que define `RpcRetcode`, `IntOctets` e `OctetsTree` —
/// tipos usados por RPCs mas ausentes de `rpcdata/`).
///
/// Templates são ignorados: `RpcDataVector<T>` não tem forma de fio própria, seu
/// comportamento já está modelado como sequência em `ty::TypeAliases`.
pub fn parse_header(text: &str, source: &str) -> Vec<CxxClass> {
    let mut classes = Vec::new();

    for (offset, _) in text.match_indices("class ") {
        // Só declarações de classe, não a palavra dentro de outro identificador.
        let preceded_by_boundary = text[..offset]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || c == ';' || c == '{' || c == '}');
        if !preceded_by_boundary {
            continue;
        }
        if last_meaningful_line_before(text, offset).starts_with("template") {
            continue;
        }
        let Some(block) = balanced_block_range(&text[offset..]) else {
            continue;
        };

        if let Ok(class) = parse_class(&text[offset..offset + block.end], source) {
            classes.push(class);
        }
    }

    classes
}

/// Última linha não vazia antes de `offset`, já sem espaços nas pontas.
fn last_meaningful_line_before(text: &str, offset: usize) -> &str {
    text[..offset]
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
}

impl CxxClass {
    /// Resolve os tipos C++ dos campos para o modelo de fio.
    pub fn resolve(
        &self,
        aliases: &TypeAliases,
        known_structs: &dyn Fn(&str) -> bool,
    ) -> Vec<ResolvedField> {
        self.items
            .iter()
            .map(|item| match item {
                Item::Field { name, cxx_type } => ResolvedField {
                    name: name.clone(),
                    ty: parse_cxx_type(cxx_type, aliases, known_structs),
                    cxx_type: cxx_type.clone(),
                    literal: None,
                },
                Item::Literal { cxx } => ResolvedField {
                    name: "_literal".to_string(),
                    ty: literal_type(cxx),
                    cxx_type: cxx.clone(),
                    literal: Some(cxx.clone()),
                },
            })
            .collect()
    }
}

/// Campo com o tipo já traduzido para o modelo de fio.
#[derive(Debug, Clone)]
pub struct ResolvedField {
    pub name: String,
    pub ty: Ty,
    pub cxx_type: String,
    /// Presente quando o campo é um literal escrito pelo `marshal`.
    pub literal: Option<String>,
}

fn literal_type(cxx: &str) -> Ty {
    // Formas observadas nos fontes: `(char)(1)`.
    let aliases = TypeAliases::default();
    if let Some(rest) = cxx.strip_prefix('(') {
        if let Some(end) = rest.find(')') {
            return parse_cxx_type(&rest[..end], &aliases, &|_| false);
        }
    }
    Ty::Unresolved(cxx.to_string())
}

/// Recolhe `nome -> tipo` de todas as declarações simples de membro do arquivo.
///
/// Deliberadamente não tenta delimitar blocos `public:`/`private:`: a ordem vem do
/// `marshal`, então basta ter os tipos disponíveis por nome. Linhas que não são
/// declarações simples (funções, `typedef`, `enum`, inicializadores) são ignoradas.
fn collect_member_declarations(text: &str) -> BTreeMap<String, String> {
    let mut declarations = BTreeMap::new();

    for line in text.lines() {
        let trimmed = line.trim();
        let Some(body) = trimmed.strip_suffix(';') else {
            continue;
        };
        // Um `:` isolado marca rótulo de acesso ou lista de inicialização; `::` é só
        // qualificação de namespace e aparece em quase todo tipo (`std::vector<...>`),
        // então precisa sobreviver ao filtro.
        let colons_stripped = body.replace("::", "");

        if body.is_empty()
            || body.contains('(')
            || body.contains('=')
            || body.contains('{')
            || body.contains("<<")
            || body.contains(">>")
            || colons_stripped.contains(':')
            || body.starts_with("typedef")
            || body.starts_with("enum")
            || body.starts_with("return")
            || body.starts_with("using")
            || body.starts_with("friend")
            || body.starts_with("//")
        {
            continue;
        }

        // Último identificador é o nome; tudo antes é o tipo.
        let Some(split_at) = body.rfind(|c: char| c.is_whitespace() || c == '>' || c == '*' || c == '&')
        else {
            continue;
        };
        let (ty, name) = body.split_at(split_at + 1);
        let name = name.trim();
        let ty = ty.trim();
        if ty.is_empty() || name.is_empty() || !is_identifier(name) {
            continue;
        }
        declarations.entry(name.to_string()).or_insert_with(|| ty.to_string());
    }

    declarations
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Lê o corpo de `marshal` e devolve os itens na ordem em que são escritos.
///
/// O corpo é delimitado por chaves balanceadas e depois quebrado em instruções por
/// `;`, e não por linha: os arquivos gerados escrevem uma instrução por linha, mas
/// depender disso faria um corpo compactado numa linha só perder campos em silêncio.
/// Cada instrução `os << expr` vira um item; `os << a << b` também é aceito.
fn parse_marshal_body(text: &str, declarations: &BTreeMap<String, String>) -> Option<Vec<Item>> {
    let signature = text.find("OctetsStream& marshal")?;
    let body = balanced_block(&text[signature..])?;
    let mut items = Vec::new();

    for statement in body.split(';') {
        let statement = statement.trim();
        // `rpcdata/` escreve `os << campo;` numa instrução própria, mas `rpcdefs.h`
        // usa a forma condensada `return os << m_int << m_octets;`. Não aceitar o
        // `return` fazia `RpcRetcode` e `IntOctets` entrarem no IR **sem campo
        // nenhum** — e `IntOctets` é o elemento da lista de personagens em
        // `GetUserRolesRes`, no caminho crítico do login.
        let statement = statement.strip_prefix("return ").unwrap_or(statement).trim_start();
        let Some(rest) = statement.strip_prefix("os <<") else {
            continue;
        };
        for expr in rest.split("<<") {
            let expr = collapse_whitespace(expr);
            if expr.is_empty() {
                continue;
            }
            // `os << MarshalContainer(m_children)` é a forma explícita do que o
            // `operator<<` de `std::vector`/`set`/`list`/`deque`/`map` já faz por
            // dentro (ver `share/common/marshal_i386.h`): contagem em `CompactUINT`
            // seguida dos elementos. Desembrulhar deixa o campo aparecer com seu tipo
            // declarado, em vez de virar um literal irresolúvel.
            let expr = unwrap_marshal_container(&expr);
            items.push(match declarations.get(&expr) {
                Some(cxx_type) => Item::Field {
                    name: expr,
                    cxx_type: cxx_type.clone(),
                },
                None if is_identifier(&expr) => Item::Field {
                    // Um campo escrito pelo marshal sem declaração visível indica um
                    // arquivo com forma inesperada. Registrar o tipo como ausente faz
                    // isso virar diagnóstico em vez de um campo descartado.
                    cxx_type: format!("<sem declaração: {expr}>"),
                    name: expr,
                },
                None => Item::Literal { cxx: expr },
            });
        }
    }

    Some(items)
}

/// Desembrulha `MarshalContainer(campo)` para `campo`.
///
/// A macro não muda a codificação: ela **é** a codificação de contêiner que o
/// `operator<<` aplica. O que importa para o IR é o campo e seu tipo declarado.
fn unwrap_marshal_container(expr: &str) -> String {
    let Some(rest) = expr.strip_prefix("MarshalContainer(") else {
        return expr.to_string();
    };
    match rest.strip_suffix(')') {
        Some(inner) => inner.trim().to_string(),
        None => expr.to_string(),
    }
}

/// Conteúdo do primeiro bloco `{ ... }` de `text`, com as chaves balanceadas.
fn balanced_block(text: &str) -> Option<&str> {
    let range = balanced_block_range(text)?;
    Some(&text[range.start + 1..range.end - 1])
}

/// Intervalo de bytes do primeiro bloco `{ ... }` balanceado, incluindo as chaves.
fn balanced_block_range(text: &str) -> Option<std::ops::Range<usize>> {
    let open = text.find('{')?;
    let mut depth = 0usize;
    for (offset, byte) in text.as_bytes().iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open..offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.trim().chars() {
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
    out
}

/// Descobre o nome da classe.
///
/// Em `rpcdata/` existe uma declaração `class X : public ...`. Em `inl/` o arquivo é
/// incluído dentro do corpo da classe, então o nome vem do construtor padrão
/// (`X() { type = PROTOCOL_X; }`) ou do construtor de cópia.
fn class_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ") {
            let name = rest.split(|c: char| c == ':' || c.is_whitespace()).next()?;
            if is_identifier(name) {
                return Some(name.to_string());
            }
        }
    }

    for line in text.lines() {
        let trimmed = line.trim();
        // `Challenge() { type = PROTOCOL_CHALLENGE; }`
        if let Some(open) = trimmed.find("()") {
            let candidate = &trimmed[..open];
            if is_identifier(candidate) && trimmed[open..].contains('{') {
                return Some(candidate.to_string());
            }
        }
        // `Challenge(const Challenge &rhs)`
        if let Some(open) = trimmed.find("(const ") {
            let candidate = &trimmed[..open];
            if is_identifier(candidate) {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

fn protocol_symbol(text: &str) -> Option<String> {
    let at = text.find("PROTOCOL_TYPE")?;
    let rest = &text[at..];
    let eq = rest.find('=')?;
    let tail = rest[eq + 1..].trim_start();
    let end = tail.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    let symbol = &tail[..end];
    is_identifier(symbol).then(|| symbol.to_string())
}

/// Extrai o número de linhas como `int PriorPolicy() const { return 101; }` e
/// `bool SizePolicy(size_t size) const { return size <= 8192; }`.
fn capture_number(text: &str, marker: &str) -> Option<i64> {
    let at = text.find(marker)?;
    let line = text[at..].lines().next()?;
    let digits: String = line
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn vector_alias(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("typedef") || !trimmed.contains("RpcDataVector") {
            continue;
        }
        let name = trimmed.trim_end_matches(';').split_whitespace().last()?;
        if is_identifier(name) {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHALLENGE_INL: &str = r#"
	public:
		Octets nonce;
		unsigned int version;
		char algo;
		Octets edition;
		unsigned char exp_rate;
		enum { PROTOCOL_TYPE = PROTOCOL_CHALLENGE };
	public:
		Challenge() { type = PROTOCOL_CHALLENGE; }
		Challenge(void*) : Protocol(PROTOCOL_CHALLENGE) { }

		OctetsStream& marshal(OctetsStream & os) const
		{
			os << nonce;
			os << version;
			os << algo;
			os << edition;
			os << exp_rate;
			return os;
		}

		int PriorPolicy( ) const { return 101; }

		bool SizePolicy(size_t size) const { return size <= 64; }
"#;

    const GROLEBASE_RPCDATA: &str = r#"
namespace GNET
{
	class GRoleBase : public GNET::Rpc::Data
	{
	public:
		int id;
		Octets name;
		unsigned char race;

	public:
		GRoleBase (int l_id = 0)
			: id(l_id)
		{
		}

		OctetsStream& marshal(OctetsStream & os) const
		{
			os << (char)(1);
			os << id;
			os << name;
			os << race;
			return os;
		}
	};
	typedef GNET::RpcDataVector<GRoleBase>	GRoleBaseVector;
};
"#;

    #[test]
    fn le_protocolo_do_inl_com_ordem_do_marshal() {
        let class = parse_class(CHALLENGE_INL, "inl/challenge").unwrap();

        assert_eq!(class.name, "Challenge");
        assert_eq!(class.protocol_symbol.as_deref(), Some("PROTOCOL_CHALLENGE"));
        assert_eq!(class.prior_policy, Some(101));
        assert_eq!(class.size_limit, Some(64));
        assert_eq!(
            class.items,
            vec![
                Item::Field { name: "nonce".into(), cxx_type: "Octets".into() },
                Item::Field { name: "version".into(), cxx_type: "unsigned int".into() },
                Item::Field { name: "algo".into(), cxx_type: "char".into() },
                Item::Field { name: "edition".into(), cxx_type: "Octets".into() },
                Item::Field { name: "exp_rate".into(), cxx_type: "unsigned char".into() },
            ]
        );
    }

    #[test]
    fn le_rpcdata_com_literal_e_alias_de_vetor() {
        let class = parse_class(GROLEBASE_RPCDATA, "rpcdata/grolebase").unwrap();

        assert_eq!(class.name, "GRoleBase");
        assert_eq!(class.vector_alias.as_deref(), Some("GRoleBaseVector"));
        assert_eq!(class.items[0], Item::Literal { cxx: "(char)(1)".into() });
        assert_eq!(
            class.items[1],
            Item::Field { name: "id".into(), cxx_type: "int".into() }
        );
    }

    #[test]
    fn literal_de_versao_resolve_para_i8() {
        let class = parse_class(GROLEBASE_RPCDATA, "rpcdata/grolebase").unwrap();
        let fields = class.resolve(&TypeAliases::with_builtins(), &|_| false);
        assert_eq!(fields[0].ty, Ty::Prim(crate::ty::Prim::I8));
        assert_eq!(fields[0].literal.as_deref(), Some("(char)(1)"));
    }

    #[test]
    fn marshal_container_e_desembrulhado_para_o_campo() {
        // `os << MarshalContainer(m_children)` codifica igual a `os << m_children`.
        // Sem desembrulhar, `OctetsTree` perdia a lista de filhos.
        let source = "\
	class OctetsTree : public Rpc::Data
	{
	public:
		Octets m_self;
		std::vector<OctetsTree> m_children;
		OctetsStream& marshal(OctetsStream & os) const
		{
			os << m_self;
			return os << MarshalContainer(m_children);
		}
	};
";
        let classes = parse_header(source, "share/rpc/rpcdefs.h");
        let classe = classes
            .iter()
            .find(|c| c.name == "OctetsTree")
            .expect("OctetsTree não foi lida");
        let campos: Vec<(&str, &str)> = classe
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Field { name, cxx_type } => Some((name.as_str(), cxx_type.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            campos,
            vec![
                ("m_self", "Octets"),
                ("m_children", "std::vector<OctetsTree>")
            ]
        );
    }

    #[test]
    fn marshal_com_return_na_mesma_instrucao_nao_perde_campos() {
        // Forma usada em `share/rpc/rpcdefs.h`. Sem tratar o `return`, `IntOctets`
        // entrava no IR sem campo algum, e a lista de personagens decodificaria como
        // vazia — em silêncio, que é o pior modo de errar aqui.
        let source = "\
	class IntOctets : public Rpc::Data
	{
	public:
		int m_int;
		Octets m_octets;
		OctetsStream& marshal(OctetsStream & os) const { return os << m_int << m_octets; }
	};
";
        let classes = parse_header(source, "share/rpc/rpcdefs.h");
        let classe = classes
            .iter()
            .find(|c| c.name == "IntOctets")
            .expect("IntOctets não foi lida");
        let nomes: Vec<&str> = classe
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Field { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(nomes, vec!["m_int", "m_octets"]);
    }

    #[test]
    fn corpo_de_marshal_em_uma_linha_nao_perde_campos() {
        let text = r#"
	class Compacto : public GNET::Rpc::Data
	{
	public:
		int a;
		Octets b;
		OctetsStream& marshal(OctetsStream & os) const { os << a; os << b; return os; }
	};
"#;
        let class = parse_class(text, "rpcdata/compacto").unwrap();
        assert_eq!(
            class.items,
            vec![
                Item::Field { name: "a".into(), cxx_type: "int".into() },
                Item::Field { name: "b".into(), cxx_type: "Octets".into() },
            ]
        );
    }

    #[test]
    fn escritas_encadeadas_viram_campos_separados_na_ordem() {
        let text = r#"
	class Encadeado : public GNET::Rpc::Data
	{
	public:
		int a;
		int b;
		OctetsStream& marshal(OctetsStream & os) const { os << a << b; return os; }
	};
"#;
        let class = parse_class(text, "rpcdata/encadeado").unwrap();
        assert_eq!(
            class.items,
            vec![
                Item::Field { name: "a".into(), cxx_type: "int".into() },
                Item::Field { name: "b".into(), cxx_type: "int".into() },
            ]
        );
    }

    #[test]
    fn campo_escrito_sem_declaracao_visivel_e_marcado_e_nao_descartado() {
        let text = r#"
	class Faltando : public GNET::Rpc::Data
	{
	public:
		OctetsStream& marshal(OctetsStream & os) const { os << fantasma; return os; }
	};
"#;
        let class = parse_class(text, "rpcdata/faltando").unwrap();
        assert_eq!(class.items.len(), 1);
        match &class.items[0] {
            Item::Field { name, cxx_type } => {
                assert_eq!(name, "fantasma");
                assert!(cxx_type.contains("sem declaração"));
            }
            other => panic!("esperava um campo, veio {other:?}"),
        }
    }

    #[test]
    fn tipo_qualificado_por_namespace_nao_e_confundido_com_rotulo() {
        // `std::vector< std::pair<char,char> >` tem `::` e `<`; um filtro ingênuo de
        // `:` descartaria a declaração e o tipo do campo viraria lixo.
        let text = r#"
	class AutoTeamConfigData : public GNET::Rpc::Data
	{
	public:
		int goal_id;
		std::vector< std::pair<char,char> > occupation_info;

	public:
		OctetsStream& marshal(OctetsStream & os) const
		{
			os << goal_id;
			os << occupation_info;
			return os;
		}
	};
"#;
        let class = parse_class(text, "rpcdata/autoteamconfigdata").unwrap();
        assert_eq!(
            class.items[1],
            Item::Field {
                name: "occupation_info".into(),
                cxx_type: "std::vector< std::pair<char,char> >".into(),
            }
        );

        let fields = class.resolve(&TypeAliases::with_builtins(), &|_| false);
        assert_eq!(
            fields[1].ty,
            Ty::Seq(Box::new(Ty::Pair(
                Box::new(Ty::Prim(crate::ty::Prim::I8)),
                Box::new(Ty::Prim(crate::ty::Prim::I8)),
            )))
        );
    }

    #[test]
    fn cabecalho_com_varias_classes_rende_todas_e_ignora_templates() {
        let header = r#"
namespace GNET
{
	class RpcRetcode : public Rpc::Data
	{
	public:
		int retcode;
		OctetsStream& marshal(OctetsStream & os) const { return os << retcode; }
	};

	class IntOctets : public Rpc::Data
	{
	public:
		int m_int;
		Octets m_octets;
		OctetsStream& marshal(OctetsStream & os) const { os << m_int; os << m_octets; return os; }
	};

	template<typename T>
	class RpcDataVector : public Rpc::Data
	{
		std::vector<T> m_data;
	public:
		OctetsStream& marshal(OctetsStream & os) const { return os << MarshalContainer(m_data); }
	};
};
"#;
        let classes = parse_header(header, "share/rpc/rpcdefs.h");
        let names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();

        assert_eq!(names, vec!["RpcRetcode", "IntOctets"]);
        assert_eq!(classes[1].items.len(), 2);
    }

    #[test]
    fn arquivo_sem_marshal_e_rejeitado_com_o_nome_do_arquivo() {
        let err = parse_class("class Vazio {};", "rpcdata/vazio").unwrap_err();
        assert_eq!(err.source, "rpcdata/vazio");
        assert!(err.reason.contains("marshal"));
    }

    #[test]
    fn declaracoes_nao_confundem_funcoes_e_typedefs_com_campos() {
        let declarations = collect_member_declarations(GROLEBASE_RPCDATA);
        assert_eq!(declarations.get("id").map(String::as_str), Some("int"));
        assert_eq!(declarations.get("name").map(String::as_str), Some("Octets"));
        assert!(!declarations.contains_key("GRoleBaseVector"));
    }
}
