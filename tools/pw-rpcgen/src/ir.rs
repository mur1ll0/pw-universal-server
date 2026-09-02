//! Montagem do IR: junta o que foi lido dos arquivos gerados, do `rpcalls.xml` e dos
//! `callid.hxx` em um único esquema, e registra o que não pôde ser resolvido.
//!
//! O IR é a saída versionada do `pw-rpcgen`. Ele não é compilado: é consumido pelos
//! testes de conformidade do `pw-protocol`, que comparam o Rust escrito à mão com o
//! que os fontes C++ originais dizem. Por isso o que importa aqui é fidelidade e
//! rastreabilidade — todo item carrega o arquivo de onde veio, e nada é descartado em
//! silêncio.

use crate::callid::CallIds;
use crate::cxx::{CxxClass, CxxRpcBinding, ResolvedField};
use crate::json::Json;
use crate::ty::{Ty, TypeAliases};
use crate::xml::Element;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
    pub cxx_type: String,
    /// Valor literal escrito pelo `marshal`, quando o campo não é um membro.
    pub literal: Option<String>,
    /// Valor padrão declarado no `rpcalls.xml`, quando houver.
    pub default: Option<String>,
}

impl Field {
    fn to_json(&self) -> Json {
        let mut entries = vec![
            ("name", Json::str(self.name.clone())),
            ("type", self.ty.to_json()),
            ("cxx", Json::str(self.cxx_type.clone())),
        ];
        if let Some(literal) = &self.literal {
            entries.push(("literal", Json::str(literal.clone())));
        }
        if let Some(default) = &self.default {
            entries.push(("default", Json::str(default.clone())));
        }
        Json::object(entries)
    }
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub source: String,
    pub vector_alias: Option<String>,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct ProtocolDef {
    pub name: String,
    pub id: Option<i64>,
    /// Símbolo `PROTOCOL_*` correspondente, para rastrear a origem do número.
    pub symbol: Option<String>,
    pub source: String,
    pub size_limit: Option<i64>,
    pub prior: Option<i64>,
    pub daemons: Vec<String>,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct RpcDef {
    pub name: String,
    pub id: Option<i64>,
    /// Símbolo `RPC_*` correspondente, quando o vínculo em `inl/` foi encontrado.
    pub symbol: Option<String>,
    pub argument: String,
    pub result: String,
    pub timeout: Option<i64>,
    pub size_limit: Option<i64>,
    pub prior: Option<i64>,
    pub daemons: Vec<String>,
}

/// Algo que o gerador não conseguiu resolver sozinho. Vai para o IR e para o relatório
/// do terminal: uma divergência visível é sempre melhor do que um campo errado calado.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: &'static str,
    pub subject: String,
    pub detail: String,
}

impl Diagnostic {
    fn to_json(&self) -> Json {
        Json::object([
            ("kind", Json::str(self.kind)),
            ("subject", Json::str(self.subject.clone())),
            ("detail", Json::str(self.detail.clone())),
        ])
    }
}

#[derive(Debug, Default)]
pub struct Schema {
    pub game_version: String,
    pub structs: BTreeMap<String, StructDef>,
    pub protocols: BTreeMap<String, ProtocolDef>,
    pub rpcs: BTreeMap<String, RpcDef>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Entradas necessárias para montar o esquema.
pub struct Inputs {
    pub game_version: String,
    /// Classes lidas de `rpcdata/`.
    pub data_classes: Vec<CxxClass>,
    /// Classes lidas de `inl/`.
    pub protocol_classes: Vec<CxxClass>,
    /// Vínculos de RPC lidos de `inl/`.
    pub rpc_bindings: Vec<CxxRpcBinding>,
    pub call_ids: CallIds,
    /// Raiz do `rpcalls.xml`, quando disponível.
    pub xml: Option<Element>,
}

pub fn build(inputs: Inputs) -> Schema {
    let Inputs {
        game_version,
        data_classes,
        protocol_classes,
        rpc_bindings,
        call_ids,
        xml,
    } = inputs;

    // Passo 1: descobrir os nomes de struct e os apelidos de vetor antes de resolver
    // qualquer tipo, para que uma struct citada por outra já seja conhecida.
    let struct_names: BTreeSet<String> = data_classes.iter().map(|c| c.name.clone()).collect();
    let mut aliases = TypeAliases::with_builtins();
    for class in &data_classes {
        if let Some(alias) = &class.vector_alias {
            aliases.insert(alias.clone(), Ty::Seq(Box::new(Ty::Struct(class.name.clone()))));
        }
    }
    // O XML também declara vetores, via `attr="vector"` em `<rpcdata>`.
    if let Some(root) = &xml {
        for data in root.children_named("rpcdata") {
            let (Some(name), Some("vector")) = (data.attr("name"), data.attr("attr")) else {
                continue;
            };
            aliases.insert(format!("{name}Vector"), Ty::Seq(Box::new(Ty::Struct(name.to_string()))));
        }
    }

    let is_known_struct = |name: &str| struct_names.contains(name);
    let defaults = collect_defaults(xml.as_ref());
    let mut schema = Schema { game_version, ..Schema::default() };

    // Passo 2: estruturas de dados.
    for class in &data_classes {
        let fields = to_fields(&class.resolve(&aliases, &is_known_struct), &defaults, &class.name);
        report_unresolved(&mut schema.diagnostics, &class.name, &class.source, &fields);
        schema.structs.insert(
            class.name.clone(),
            StructDef {
                name: class.name.clone(),
                source: class.source.clone(),
                vector_alias: class.vector_alias.clone(),
                fields,
            },
        );
    }

    // Passo 3: protocolos, cruzando o símbolo `PROTOCOL_*` com o número do `callid.hxx`
    // e, quando ausente lá, com o atributo `type` do XML.
    let xml_protocol_ids = collect_xml_protocol_ids(xml.as_ref());
    for class in &protocol_classes {
        let fields = to_fields(&class.resolve(&aliases, &is_known_struct), &defaults, &class.name);
        report_unresolved(&mut schema.diagnostics, &class.name, &class.source, &fields);

        let symbol = class.protocol_symbol.clone();
        let id = symbol
            .as_ref()
            .and_then(|s| call_ids.protocols.get(s).copied())
            .or_else(|| xml_protocol_ids.get(&class.name).copied());

        if id.is_none() {
            schema.diagnostics.push(Diagnostic {
                kind: "protocolo-sem-id",
                subject: class.name.clone(),
                detail: format!(
                    "nem `{}` nem o rpcalls.xml forneceram um identificador numérico",
                    symbol.clone().unwrap_or_else(|| "<sem símbolo>".into())
                ),
            });
        }

        let daemons = symbol
            .as_ref()
            .and_then(|s| call_ids.daemons.get(s).cloned())
            .unwrap_or_default();

        schema.protocols.insert(
            class.name.clone(),
            ProtocolDef {
                name: class.name.clone(),
                id,
                symbol,
                source: class.source.clone(),
                size_limit: class.size_limit,
                prior: class.prior_policy,
                daemons,
                fields,
            },
        );
    }

    // Passo 4: RPCs. A assinatura (argumento e resultado) só existe no XML; o símbolo
    // `RPC_*`, os limites e os daemons vêm dos vínculos em `inl/`.
    let bindings: BTreeMap<&str, &CxxRpcBinding> =
        rpc_bindings.iter().map(|b| (b.name.as_str(), b)).collect();

    if let Some(root) = &xml {
        for rpc in root.children_named("rpc") {
            let Some(name) = rpc.attr("name") else { continue };
            let argument = rpc.attr("argument").unwrap_or_default().to_string();
            let result = rpc.attr("result").unwrap_or_default().to_string();

            for (role, type_name) in [("argument", &argument), ("result", &result)] {
                if !type_name.is_empty() && !schema.structs.contains_key(type_name.as_str()) {
                    schema.diagnostics.push(Diagnostic {
                        kind: "rpc-tipo-ausente",
                        subject: name.to_string(),
                        detail: format!("{role} `{type_name}` não foi encontrado em rpcdata/"),
                    });
                }
            }

            let binding = bindings.get(name).copied();
            let symbol = binding.map(|b| b.symbol.clone());
            let daemons = symbol
                .as_ref()
                .and_then(|s| call_ids.daemons.get(s).cloned())
                .unwrap_or_default();

            // O número declarado no XML e o do `callid.hxx` precisam concordar: são a
            // mesma coisa vista de dois lugares, e uma divergência aqui significaria
            // chamar o RPC errado em produção.
            let xml_id = rpc.attr_int("type");
            let callid = symbol.as_ref().and_then(|s| call_ids.rpcs.get(s).copied());
            if let (Some(x), Some(c)) = (xml_id, callid) {
                if x != c {
                    schema.diagnostics.push(Diagnostic {
                        kind: "rpc-id-divergente",
                        subject: name.to_string(),
                        detail: format!("rpcalls.xml diz {x}, callid.hxx diz {c}"),
                    });
                }
            }

            schema.rpcs.insert(
                name.to_string(),
                RpcDef {
                    name: name.to_string(),
                    id: xml_id.or(callid),
                    symbol,
                    argument,
                    result,
                    timeout: rpc.attr_int("timeout").or_else(|| binding.and_then(|b| b.time_limit)),
                    size_limit: rpc.attr_int("maxsize").or_else(|| binding.and_then(|b| b.size_limit)),
                    prior: rpc.attr_int("prior").or_else(|| binding.and_then(|b| b.prior_policy)),
                    daemons,
                },
            );
        }
    }

    // Um vínculo em `inl/` sem declaração no XML é uma lacuna real no esquema.
    for binding in &rpc_bindings {
        if !schema.rpcs.contains_key(&binding.name) {
            schema.diagnostics.push(Diagnostic {
                kind: "rpc-sem-declaracao",
                subject: binding.name.clone(),
                detail: format!("{} não tem <rpc> correspondente no rpcalls.xml", binding.source),
            });
        }
    }

    schema
}

fn to_fields(
    resolved: &[ResolvedField],
    defaults: &BTreeMap<(String, String), String>,
    owner: &str,
) -> Vec<Field> {
    resolved
        .iter()
        .map(|f| Field {
            name: f.name.clone(),
            ty: f.ty.clone(),
            cxx_type: f.cxx_type.clone(),
            literal: f.literal.clone(),
            default: defaults.get(&(owner.to_string(), f.name.clone())).cloned(),
        })
        .collect()
}

fn report_unresolved(
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    source: &str,
    fields: &[Field],
) {
    for field in fields {
        if field.ty.is_unresolved() {
            diagnostics.push(Diagnostic {
                kind: "tipo-nao-resolvido",
                subject: format!("{owner}.{}", field.name),
                detail: format!("`{}` em {source}", field.cxx_type),
            });
        }
    }
}

/// `(classe, campo) -> valor padrão`, lido dos `<variable default="...">` do XML.
fn collect_defaults(root: Option<&Element>) -> BTreeMap<(String, String), String> {
    let mut defaults = BTreeMap::new();
    let Some(root) = root else { return defaults };

    for container in root.children.iter() {
        if container.name != "protocol" && container.name != "rpcdata" {
            continue;
        }
        let Some(owner) = container.attr("name") else { continue };
        for variable in container.children_named("variable") {
            let (Some(field), Some(default)) = (variable.attr("name"), variable.attr("default"))
            else {
                continue;
            };
            defaults.insert((owner.to_string(), field.to_string()), default.to_string());
        }
    }

    defaults
}

fn collect_xml_protocol_ids(root: Option<&Element>) -> BTreeMap<String, i64> {
    let mut ids = BTreeMap::new();
    let Some(root) = root else { return ids };
    for protocol in root.children_named("protocol") {
        if let (Some(name), Some(id)) = (protocol.attr("name"), protocol.attr_int("type")) {
            ids.insert(name.to_string(), id);
        }
    }
    ids
}

impl Schema {
    pub fn to_json(&self) -> Json {
        let structs = self.structs.values().map(|s| {
            let mut entries = vec![
                ("name", Json::str(s.name.clone())),
                ("source", Json::str(s.source.clone())),
            ];
            if let Some(alias) = &s.vector_alias {
                entries.push(("vector_alias", Json::str(alias.clone())));
            }
            entries.push(("fields", Json::array(s.fields.iter().map(Field::to_json))));
            Json::object(entries)
        });

        let protocols = self.protocols.values().map(|p| {
            Json::object([
                ("name", Json::str(p.name.clone())),
                ("id", p.id.map(Json::Int).unwrap_or(Json::Null)),
                (
                    "symbol",
                    p.symbol.clone().map(Json::Str).unwrap_or(Json::Null),
                ),
                ("source", Json::str(p.source.clone())),
                ("size_limit", p.size_limit.map(Json::Int).unwrap_or(Json::Null)),
                ("prior", p.prior.map(Json::Int).unwrap_or(Json::Null)),
                (
                    "daemons",
                    Json::array(p.daemons.iter().cloned().map(Json::Str)),
                ),
                ("fields", Json::array(p.fields.iter().map(Field::to_json))),
            ])
        });

        let rpcs = self.rpcs.values().map(|r| {
            Json::object([
                ("name", Json::str(r.name.clone())),
                ("id", r.id.map(Json::Int).unwrap_or(Json::Null)),
                ("symbol", r.symbol.clone().map(Json::Str).unwrap_or(Json::Null)),
                ("argument", Json::str(r.argument.clone())),
                ("result", Json::str(r.result.clone())),
                ("timeout", r.timeout.map(Json::Int).unwrap_or(Json::Null)),
                ("size_limit", r.size_limit.map(Json::Int).unwrap_or(Json::Null)),
                ("prior", r.prior.map(Json::Int).unwrap_or(Json::Null)),
                ("daemons", Json::array(r.daemons.iter().cloned().map(Json::Str))),
            ])
        });

        Json::object([
            (
                "_comment",
                Json::str(
                    "Gerado por tools/pw-rpcgen a partir dos fontes C++ originais. \
                     Não editar à mão. Escalares vão para o fio em big-endian; \
                     Octets/string/sequências usam prefixo CompactUINT.",
                ),
            ),
            ("game_version", Json::str(self.game_version.clone())),
            ("structs", Json::array(structs)),
            ("protocols", Json::array(protocols)),
            ("rpcs", Json::array(rpcs)),
            (
                "diagnostics",
                Json::array(self.diagnostics.iter().map(Diagnostic::to_json)),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxx::parse_class;
    use crate::ty::Prim;

    fn inputs_com(protocolo: &str, dados: &[(&str, &str)], xml: Option<&str>) -> Inputs {
        let mut call_ids = CallIds::default();
        call_ids.absorb("enum X { PROTOCOL_CHALLENGE = 1, };", "glinkd");

        Inputs {
            game_version: "1.5.3".to_string(),
            data_classes: dados
                .iter()
                .map(|(text, source)| parse_class(text, source).unwrap())
                .collect(),
            protocol_classes: vec![parse_class(protocolo, "inl/challenge").unwrap()],
            rpc_bindings: Vec::new(),
            call_ids,
            xml: xml.map(|t| crate::xml::parse(t).unwrap()),
        }
    }

    const CHALLENGE: &str = r#"
	public:
		Octets nonce;
		unsigned int version;
		enum { PROTOCOL_TYPE = PROTOCOL_CHALLENGE };
	public:
		Challenge() { type = PROTOCOL_CHALLENGE; }
		OctetsStream& marshal(OctetsStream & os) const
		{
			os << nonce;
			os << version;
			return os;
		}
		bool SizePolicy(size_t size) const { return size <= 64; }
"#;

    #[test]
    fn protocolo_recebe_id_do_callid_e_registra_o_daemon() {
        let schema = build(inputs_com(CHALLENGE, &[], None));
        let challenge = &schema.protocols["Challenge"];

        assert_eq!(challenge.id, Some(1));
        assert_eq!(challenge.symbol.as_deref(), Some("PROTOCOL_CHALLENGE"));
        assert_eq!(challenge.daemons, vec!["glinkd".to_string()]);
        assert_eq!(challenge.size_limit, Some(64));
        assert_eq!(challenge.fields.len(), 2);
        assert_eq!(challenge.fields[0].ty, Ty::Octets);
        assert_eq!(challenge.fields[1].ty, Ty::Prim(Prim::U32));
    }

    #[test]
    fn apelido_de_vetor_de_uma_struct_resolve_em_outra() {
        let role_info = r#"
	class RoleInfo : public GNET::Rpc::Data
	{
	public:
		int roleid;
		OctetsStream& marshal(OctetsStream & os) const { os << roleid; return os; }
	};
	typedef GNET::RpcDataVector<RoleInfo>	RoleInfoVector;
"#;
        let role_list = r#"
	class RoleList_Re : public GNET::Rpc::Data
	{
	public:
		RoleInfoVector rolelist;
		OctetsStream& marshal(OctetsStream & os) const { os << rolelist; return os; }
	};
"#;
        let schema = build(inputs_com(
            CHALLENGE,
            &[(role_info, "rpcdata/roleinfo"), (role_list, "rpcdata/rolelist_re")],
            None,
        ));

        assert_eq!(
            schema.structs["RoleList_Re"].fields[0].ty,
            Ty::Seq(Box::new(Ty::Struct("RoleInfo".into())))
        );
        assert!(schema.diagnostics.is_empty(), "{:?}", schema.diagnostics);
    }

    #[test]
    fn tipo_desconhecido_vira_diagnostico_em_vez_de_sumir() {
        let estranho = r#"
	class Estranho : public GNET::Rpc::Data
	{
	public:
		TipoQueNaoExiste campo;
		OctetsStream& marshal(OctetsStream & os) const { os << campo; return os; }
	};
"#;
        let schema = build(inputs_com(CHALLENGE, &[(estranho, "rpcdata/estranho")], None));

        assert_eq!(schema.diagnostics.len(), 1);
        assert_eq!(schema.diagnostics[0].kind, "tipo-nao-resolvido");
        assert_eq!(schema.diagnostics[0].subject, "Estranho.campo");
        // O campo continua no esquema, marcado como não resolvido.
        assert!(schema.structs["Estranho"].fields[0].ty.is_unresolved());
    }

    #[test]
    fn rpc_combina_assinatura_do_xml_com_simbolo_e_daemons_do_inl() {
        const RPC_INL: &str = r#"
		GNET::Protocol *Clone() const {  return new UserLogin(*this); }
	public:
		enum { PROTOCOL_TYPE = RPC_USERLOGIN };
		UserLogin(const UserLogin &rhs) : RPC_BASECLASS(rhs) { }
		int  PriorPolicy( ) const { return 1; }
		bool SizePolicy(size_t size) const { return size <= 128; }
		bool TimePolicy(int timeout) const { return timeout <= 10; }
"#;
        let arg = r#"class UserLoginArg : public GNET::Rpc::Data
	{ public: int userid;
		OctetsStream& marshal(OctetsStream & os) const { os << userid; return os; } };"#;
        let res = r#"class UserLoginRes : public GNET::Rpc::Data
	{ public: int retcode;
		OctetsStream& marshal(OctetsStream & os) const { os << retcode; return os; } };"#;

        let mut call_ids = CallIds::default();
        call_ids.absorb("enum X { PROTOCOL_CHALLENGE = 1, RPC_USERLOGIN = 15, };", "gdeliveryd");

        let schema = build(Inputs {
            game_version: "1.5.3".into(),
            data_classes: vec![
                parse_class(arg, "rpcdata/userloginarg").unwrap(),
                parse_class(res, "rpcdata/userloginres").unwrap(),
            ],
            protocol_classes: vec![parse_class(CHALLENGE, "inl/challenge").unwrap()],
            rpc_bindings: vec![crate::cxx::parse_rpc_binding(RPC_INL, "inl/userlogin").unwrap()],
            call_ids,
            xml: Some(
                crate::xml::parse(
                    r#"<application><rpc name="UserLogin" type="15"
                        argument="UserLoginArg" result="UserLoginRes" maxsize="128"/></application>"#,
                )
                .unwrap(),
            ),
        });

        let rpc = &schema.rpcs["UserLogin"];
        assert_eq!(rpc.id, Some(15));
        assert_eq!(rpc.symbol.as_deref(), Some("RPC_USERLOGIN"));
        assert_eq!(rpc.daemons, vec!["gdeliveryd".to_string()]);
        assert_eq!(rpc.size_limit, Some(128));
        // O `timeout` não está no XML deste caso, então vem da política do `inl/`.
        assert_eq!(rpc.timeout, Some(10));
        assert!(
            !schema.diagnostics.iter().any(|d| d.kind == "rpc-tipo-ausente"),
            "{:?}",
            schema.diagnostics
        );
    }

    #[test]
    fn id_de_rpc_divergente_entre_xml_e_callid_vira_diagnostico() {
        const RPC_INL: &str = r#"
		enum { PROTOCOL_TYPE = RPC_USERLOGIN };
		UserLogin(const UserLogin &rhs) : RPC_BASECLASS(rhs) { }
"#;
        let mut call_ids = CallIds::default();
        call_ids.absorb("enum X { RPC_USERLOGIN = 15, };", "gdeliveryd");

        let schema = build(Inputs {
            game_version: "1.5.3".into(),
            data_classes: Vec::new(),
            protocol_classes: vec![parse_class(CHALLENGE, "inl/challenge").unwrap()],
            rpc_bindings: vec![crate::cxx::parse_rpc_binding(RPC_INL, "inl/userlogin").unwrap()],
            call_ids,
            xml: Some(
                crate::xml::parse(r#"<application><rpc name="UserLogin" type="99"/></application>"#)
                    .unwrap(),
            ),
        });

        let divergence = schema
            .diagnostics
            .iter()
            .find(|d| d.kind == "rpc-id-divergente")
            .expect("esperava um diagnóstico de divergência");
        assert_eq!(divergence.subject, "UserLogin");
        assert!(divergence.detail.contains("99") && divergence.detail.contains("15"));
    }

    #[test]
    fn xml_fornece_defaults_e_ids_de_rpc() {
        let xml = r#"<application>
            <protocol name="Challenge" type="1">
              <variable name="nonce" type="Octets" default="Octets(0)"/>
            </protocol>
            <rpc name="UserLogin" type="15" argument="UserLoginArg" result="UserLoginRes" timeout="30"/>
        </application>"#;

        let schema = build(inputs_com(CHALLENGE, &[], Some(xml)));

        assert_eq!(
            schema.protocols["Challenge"].fields[0].default.as_deref(),
            Some("Octets(0)")
        );
        let rpc = &schema.rpcs["UserLogin"];
        assert_eq!(rpc.id, Some(15));
        assert_eq!(rpc.timeout, Some(30));
        // Os tipos do RPC não existem em rpcdata/ neste teste, o que deve virar aviso.
        assert_eq!(
            schema
                .diagnostics
                .iter()
                .filter(|d| d.kind == "rpc-tipo-ausente")
                .count(),
            2
        );
    }
}
