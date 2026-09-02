//! `pw-rpcgen` — extrai o esquema canônico do protocolo GNET dos fontes C++ originais.
//!
//! Esta ferramenta **não gera o código de produção**. Ela produz um IR em JSON,
//! versionado no repositório, que descreve cada protocolo e cada estrutura de dados
//! exatamente como o servidor original os coloca no fio. O `pw-protocol` é escrito à
//! mão sobre esse IR, e testes de conformidade comparam os dois: se um campo mudar de
//! posição ou de tipo, o teste aponta a divergência com o C++ original.
//!
//! Uso:
//!
//! ```text
//! pw-rpcgen --server-src <dir> [--game-version 1.5.3] --out specs/protocol/gnet_153.json
//! ```
//!
//! `--server-src` aponta para a raiz dos fontes do servidor (a pasta que contém
//! `inl/`, `rpcdata/`, `rpcalls.xml` e os `<daemon>/callid.hxx`).
//!
//! A ferramenta extrai **dois** esquemas, de dois modelos de fio distintos que
//! convivem na mesma conexão:
//!
//! * `--server-src` + `--out` → os protocolos GNET (big-endian, `CompactUINT`), a
//!   partir dos fontes do servidor;
//! * `--client-src` + `--out-gamedata` → os subcomandos do `GamedataSend`, o
//!   protocolo do mundo 3D (little-endian, `pack(1)`, `memcpy` cru), a partir dos
//!   fontes do cliente.
//!
//! Os dois modos são independentes; pelo menos um precisa ser pedido.

mod callid;
mod cxx;
mod gd_cxx;
mod gd_ir;
mod gd_srv;
mod gd_ty;
mod ir;
mod json;
mod ty;
mod xml;

use callid::CallIds;
use cxx::{CxxClass, CxxRpcBinding};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Daemons cujos `callid.hxx` são lidos. A união deles cobre todos os identificadores
/// de protocolo e RPC; valores divergentes entre daemons viram diagnóstico.
const DAEMONS: &[&str] = &[
    "glinkd",
    "gdeliveryd",
    "gamed",
    "gamedbd",
    "uniquenamed",
    "gfaction",
];

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            eprintln!("{report}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("pw-rpcgen: {message}");
            ExitCode::FAILURE
        }
    }
}

struct Options {
    server_src: Option<PathBuf>,
    out: Option<PathBuf>,
    client_src: Option<PathBuf>,
    out_gamedata: Option<PathBuf>,
    game_version: String,
    /// Falha se houver diagnósticos, para uso em verificação automatizada.
    strict: bool,
    /// Nome da subpasta do cliente que contém `Network/EC_GPDataType.h` e
    /// `EC_RoleTypes.h`. O 1.5.3 chama essa pasta `CElementClient`; outras árvores de
    /// fonte (o 1.5.5 da EvolvedPW, por exemplo) usam `ElementClient`, sem o `C`.
    client_subdir: String,
}

fn parse_options() -> Result<Options, String> {
    let mut server_src = None;
    let mut out = None;
    let mut client_src = None;
    let mut out_gamedata = None;
    let mut game_version = "1.5.3".to_string();
    let mut strict = false;
    let mut client_subdir = "CElementClient".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("`{name}` precisa de um valor"))
        };
        match arg.as_str() {
            "--server-src" => server_src = Some(PathBuf::from(value("--server-src")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--client-src" => client_src = Some(PathBuf::from(value("--client-src")?)),
            "--out-gamedata" => out_gamedata = Some(PathBuf::from(value("--out-gamedata")?)),
            "--game-version" => game_version = value("--game-version")?,
            "--client-subdir" => client_subdir = value("--client-subdir")?,
            "--strict" => strict = true,
            "-h" | "--help" => return Err(usage()),
            other => return Err(format!("opção desconhecida: `{other}`\n\n{}", usage())),
        }
    }

    // Cada modo precisa do seu par completo — uma fonte sem destino (ou o contrário)
    // é quase sempre um erro de digitação que passaria despercebido como "nada a fazer".
    if server_src.is_some() != out.is_some() {
        return Err(format!("`--server-src` e `--out` andam juntos\n\n{}", usage()));
    }
    if client_src.is_some() != out_gamedata.is_some() {
        return Err(format!(
            "`--client-src` e `--out-gamedata` andam juntos\n\n{}",
            usage()
        ));
    }
    if server_src.is_none() && client_src.is_none() {
        return Err(format!("nada a fazer\n\n{}", usage()));
    }

    Ok(Options {
        server_src,
        out,
        client_src,
        out_gamedata,
        game_version,
        strict,
        client_subdir,
    })
}

fn usage() -> String {
    "uso: pw-rpcgen [--server-src <fontes do servidor> --out <gnet.json>]\n\
     \x20              [--client-src <fontes do cliente> --out-gamedata <gamedata.json>]\n\
     \x20              [--game-version 1.5.3] [--client-subdir CElementClient] [--strict]\n\n\
     `--server-src` aponta para a pasta com `inl/`, `rpcdata/`, `rpcalls.xml` e os\n\
     `<daemon>/callid.hxx`. `--client-src` aponta para a pasta que contém\n\
     `<client-subdir>/Network/EC_GPDataType.h` e `EC_GameDataPrtc.cpp` —\n\
     `<client-subdir>` é `CElementClient` por padrão (1.5.3); outras árvores (a\n\
     EvolvedPW do 1.5.5, por exemplo) usam `ElementClient`, sem o `C`."
        .to_string()
}

fn run() -> Result<String, String> {
    let options = parse_options()?;
    let mut reports = Vec::new();
    let mut total_diagnostics = 0usize;

    if options.server_src.is_some() {
        let (report, diagnostics) = run_gnet(&options)?;
        reports.push(report);
        total_diagnostics += diagnostics;
    }
    if options.client_src.is_some() {
        let (report, diagnostics) = run_gamedata(&options)?;
        reports.push(report);
        total_diagnostics += diagnostics;
    }

    let report = reports.join("\n\n");
    if options.strict && total_diagnostics > 0 {
        return Err(format!(
            "{report}\n\n--strict: {total_diagnostics} diagnóstico(s) impedem a conclusão"
        ));
    }
    Ok(report)
}

/// Extrai os subcomandos do `GamedataSend` dos fontes do cliente.
fn run_gamedata(options: &Options) -> Result<(String, usize), String> {
    let client_src = options.client_src.as_ref().expect("checado em parse_options");
    let out = options.out_gamedata.as_ref().expect("checado em parse_options");

    let types_path = client_src.join(&options.client_subdir).join("Network/EC_GPDataType.h");
    let prtc_path = client_src.join(&options.client_subdir).join("Network/EC_GameDataPrtc.cpp");

    let types_text = read_latin1(&types_path)
        .ok_or_else(|| format!("não consegui ler {}", types_path.display()))?;
    let prtc_text = read_latin1(&prtc_path)
        .ok_or_else(|| format!("não consegui ler {}", prtc_path.display()))?;

    // Os blocos `/* */` guardam código de exemplo que declara structs. Se ele
    // sobreviver até o parser, vira campo inventado e desloca todos os campos
    // seguintes, sem nada acusar.
    let types_raw: Vec<&str> = types_text.lines().collect();
    let prtc_raw: Vec<&str> = prtc_text.lines().collect();
    let types_owned = gd_cxx::strip_block_comments(&types_raw);
    let prtc_owned = gd_cxx::strip_block_comments(&prtc_raw);
    let types_lines: Vec<&str> = types_owned.iter().map(String::as_str).collect();
    let prtc_lines: Vec<&str> = prtc_owned.iter().map(String::as_str).collect();

    let mut warnings = Vec::new();
    let mut structs = Vec::new();
    let mut enums = BTreeMap::new();

    // `ROLEEXTPROP` e suas quatro partes moram em `EC_RoleTypes.h`, não em
    // `EC_GPDataType.h`, mas seis structs de comando as usam como campo. Lidas em
    // escopo global, a busca de escopo do IR as encontra a partir de qualquer
    // namespace.
    let roles_path = client_src.join(&options.client_subdir).join("EC_RoleTypes.h");
    if let Some(text) = read_latin1(&roles_path) {
        let raw: Vec<&str> = text.lines().collect();
        let owned = gd_cxx::strip_block_comments(&raw);
        let lines: Vec<&str> = owned.iter().map(String::as_str).collect();
        let consts = gd_cxx::parse_consts(&lines);
        structs.extend(gd_cxx::parse_packed_structs_with(
            &lines,
            0,
            lines.len(),
            "",
            &[],
            &consts,
            &mut warnings,
        ));
    }

    // `DWORD states[OBJECT_EXT_STATE_COUNT]` só tem tamanho se soubermos o valor da
    // constante. Elas são declaradas fora da região empacotada, então a varredura
    // cobre o arquivo inteiro antes de qualquer struct ser lida.
    let consts = gd_cxx::parse_consts(&types_lines);

    for ns in [gd_cxx::Ns::S2C, gd_cxx::Ns::C2S] {
        let region = gd_cxx::find_namespace(&types_lines, ns.as_str()).ok_or_else(|| {
            format!(
                "não achei `namespace {}` em {}",
                ns.as_str(),
                types_path.display()
            )
        })?;

        let cmd_enum = gd_cxx::find_command_enum(&types_lines, region).ok_or_else(|| {
            format!(
                "não achei o enum de Command ID em `namespace {}`",
                ns.as_str()
            )
        })?;
        enums.insert(
            ns,
            gd_cxx::parse_command_enum(&types_lines, cmd_enum.start, cmd_enum.end),
        );

        structs.extend(gd_cxx::parse_packed_structs_with(
            &types_lines,
            region.start,
            region.end,
            ns.as_str(),
            &[],
            &consts,
            &mut warnings,
        ));
    }

    // O lado do servidor: o mesmo protocolo descrito por cabeçalhos independentes. É
    // ele que preenche as structs do C2S — que o cliente não descreve em lugar nenhum —
    // e que serve de segunda opinião sobre o S2C.
    let server = match options.server_src.as_ref() {
        Some(root) => Some(read_server_side(root, &options.game_version)?),
        None => None,
    };

    let calc = gd_cxx::find_calc_fn(&prtc_lines).ok_or_else(|| {
        format!(
            "não achei `CalcS2CCmdDataSize` em {}",
            prtc_path.display()
        )
    })?;
    let payloads = gd_cxx::parse_payload_table(&prtc_lines, calc.start, calc.end);

    let schema = gd_ir::build(gd_ir::Inputs {
        game_version: options.game_version.clone(),
        s2c_enum: enums.remove(&gd_cxx::Ns::S2C).unwrap_or_default(),
        c2s_enum: enums.remove(&gd_cxx::Ns::C2S).unwrap_or_default(),
        structs,
        payloads,
        warnings,
        server,
    });

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("não consegui criar {}: {e}", parent.display()))?;
    }
    std::fs::write(out, schema.to_json().to_pretty())
        .map_err(|e| format!("não consegui escrever {}: {e}", out.display()))?;

    let s2c = schema.commands.iter().filter(|c| c.ns == gd_cxx::Ns::S2C).count();
    let c2s = schema.commands.len() - s2c;
    let com_payload = schema.commands.iter().filter(|c| c.payload_struct.is_some()).count();
    let variaveis = schema.structs.values().filter(|s| s.var_list.is_some()).count();

    let c2s_srv = schema
        .commands
        .iter()
        .filter(|c| c.ns == gd_cxx::Ns::C2S && c.server_struct.is_some())
        .count();

    let mut report = format!("pw-rpcgen gamedata {} → {}", schema.game_version, out.display());
    report.push_str(&format!(
        "\n  {} comandos ({s2c} S2C, {c2s} C2S)",
        schema.commands.len()
    ));
    report.push_str(&format!(
        "\n  {com_payload} S2C com struct do cliente, {c2s_srv} C2S com struct do servidor"
    ));
    report.push_str(&format!(
        "\n  {} structs empacotadas ({variaveis} de tamanho variável)",
        schema.structs.len()
    ));
    report.push_str(&format!(
        "\n  {} âncoras de enum conferidas",
        schema.anchors_checked
    ));

    // A conferência cruzada só existe quando os dois lados foram lidos.
    let x = &schema.cross;
    if x.commands_matched > 0 {
        report.push_str(&format!(
            "\n  cruzamento cliente×servidor: {} comandos casados por id, \
             {}/{} layouts idênticos ({} escalares, {} pulados)",
            x.commands_matched, x.structs_agree, x.structs_compared, x.fields_compared,
            x.structs_skipped,
        ));
        if x.sign_mismatches > 0 {
            report.push_str(&format!(
                "\n  {} escalares com sinal divergente entre os dois lados",
                x.sign_mismatches
            ));
        }
    }

    let diagnostics = schema.diagnostics.len();
    if diagnostics == 0 {
        report.push_str("\n  sem diagnósticos");
    } else {
        let mut por_tipo: BTreeMap<&str, usize> = BTreeMap::new();
        for d in &schema.diagnostics {
            *por_tipo.entry(d.kind).or_default() += 1;
        }
        report.push_str(&format!("\n  {diagnostics} diagnóstico(s):"));
        for (kind, count) in por_tipo {
            report.push_str(&format!("\n    {kind}: {count}"));
        }
        report.push_str("\n  (detalhes no campo `diagnostics` do JSON)");
    }

    Ok((report, diagnostics))
}

/// Lê os quatro arquivos de `cgame/` que descrevem o protocolo do mundo 3D pelo lado
/// do servidor.
fn read_server_side(root: &Path, _game_version: &str) -> Result<gd_srv::ServerSide, String> {
    let cgame = root.join("cgame");
    let ler = |rel: &str| -> Result<String, String> {
        let path = cgame.join(rel);
        read_latin1(&path).ok_or_else(|| format!("não consegui ler {}", path.display()))
    };

    let types_text = ler("common/types.h")?;
    let protocol_text = ler("common/protocol.h")?;
    let imp_text = ler("common/protocol_imp.h")?;
    let cmd_text = ler("gs/playercmd.cpp")?;

    // Mesma armadilha do lado do cliente: código de exemplo dentro de `/* */` vira
    // campo inventado se sobreviver até o parser.
    let limpar = |t: &str| -> Vec<String> {
        let raw: Vec<&str> = t.lines().collect();
        gd_cxx::strip_block_comments(&raw)
    };
    let types_owned = limpar(&types_text);
    let protocol_owned = limpar(&protocol_text);
    let imp_owned = limpar(&imp_text);
    let cmd_owned = limpar(&cmd_text);

    let as_lines = |v: &[String]| -> Vec<String> { v.to_vec() };
    let types_lines: Vec<&str> = types_owned.iter().map(String::as_str).collect();
    let protocol_lines: Vec<&str> = protocol_owned.iter().map(String::as_str).collect();
    let imp_lines: Vec<&str> = imp_owned.iter().map(String::as_str).collect();
    let cmd_lines: Vec<&str> = cmd_owned.iter().map(String::as_str).collect();
    let _ = as_lines;

    let mut consts = gd_cxx::parse_consts(&types_lines);
    consts.extend(gd_cxx::parse_consts(&protocol_lines));

    let mut out = gd_srv::ServerSide::default();
    gd_srv::parse_types_h(&types_lines, &consts, &mut out);
    gd_srv::parse_protocol_h(&protocol_lines, &consts, &mut out)?;
    out.s2c_bindings = gd_srv::parse_s2c_bindings(&imp_lines);
    out.c2s_bindings = gd_srv::parse_c2s_bindings(&cmd_lines);
    Ok(out)
}

/// Extrai os protocolos GNET dos fontes do servidor.
fn run_gnet(options: &Options) -> Result<(String, usize), String> {
    let server_src = options.server_src.as_ref().expect("checado em parse_options");
    let out_path = options.out.as_ref().expect("checado em parse_options");

    let mut data_dir = read_class_dir(&server_src.join("rpcdata"), "rpcdata")?;
    let protocol_dir = read_class_dir(&server_src.join("inl"), "inl")?;

    // `RpcRetcode`, `IntOctets` e `OctetsTree` são usados por dezenas de RPCs mas
    // moram em `rpcdefs.h`, não em `rpcdata/`. Sem eles, o IR teria referências
    // penduradas — então o cabeçalho entra como mais uma fonte de estruturas.
    let rpcdefs_path = server_src.join("share/rpc/rpcdefs.h");
    let rpcdefs = read_latin1(&rpcdefs_path).ok_or_else(|| {
        format!(
            "não consegui ler {} (necessário para RpcRetcode/IntOctets/OctetsTree)",
            rpcdefs_path.display()
        )
    })?;
    data_dir
        .classes
        .extend(cxx::parse_header(&rpcdefs, "share/rpc/rpcdefs.h"));

    let mut call_ids = CallIds::default();
    let mut id_conflicts = Vec::new();
    let mut daemons_lidos = 0usize;
    for daemon in DAEMONS {
        let path = server_src.join(daemon).join("callid.hxx");
        let Some(text) = read_latin1(&path) else {
            continue;
        };
        daemons_lidos += 1;
        id_conflicts.extend(call_ids.absorb(&text, daemon));
    }
    if daemons_lidos == 0 {
        return Err(format!(
            "nenhum callid.hxx encontrado em {} (procurei em {})",
            server_src.display(),
            DAEMONS.join(", ")
        ));
    }

    let xml_path = server_src.join("rpcalls.xml");
    let xml_text = read_latin1(&xml_path)
        .ok_or_else(|| format!("não consegui ler {}", xml_path.display()))?;
    let xml = Some(xml::parse(&xml_text).map_err(|e| format!("{}: {e}", xml_path.display()))?);

    let mut schema = ir::build(ir::Inputs {
        game_version: options.game_version.clone(),
        data_classes: data_dir.classes,
        protocol_classes: protocol_dir.classes,
        rpc_bindings: protocol_dir.rpc_bindings,
        call_ids,
        xml,
    });

    for conflict in &id_conflicts {
        schema.diagnostics.push(ir::Diagnostic {
            kind: "callid-divergente",
            subject: conflict.symbol.clone(),
            detail: conflict.to_string(),
        });
    }
    for source in data_dir.ignored.iter().chain(protocol_dir.ignored.iter()) {
        schema.diagnostics.push(ir::Diagnostic {
            kind: "arquivo-ignorado",
            subject: source.clone(),
            detail: "sem um corpo de `marshal` reconhecível".to_string(),
        });
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("não consegui criar {}: {e}", parent.display()))?;
    }
    std::fs::write(out_path, schema.to_json().to_pretty())
        .map_err(|e| format!("não consegui escrever {}: {e}", out_path.display()))?;

    let diagnostics = schema.diagnostics.len();
    Ok((format_report(&schema, out_path), diagnostics))
}

/// Lê um arquivo dos fontes originais decodificando como Latin-1.
///
/// Os fontes têm comentários em chinês (GBK) e `rpcalls.xml` se declara ISO-8859-1, de
/// modo que nada ali é UTF-8 válido. Tratar isso como "arquivo ilegível" faria classes
/// inteiras sumirem do IR em silêncio — que é exatamente a falha que este projeto não
/// pode ter. Só a estrutura e os identificadores ASCII importam para o esquema.
fn read_latin1(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(bytes.iter().map(|&b| b as char).collect())
}

/// Resultado da leitura de um diretório de classes geradas.
struct ClassDir {
    classes: Vec<CxxClass>,
    /// Vínculos de RPC: arquivos de `inl/` que descrevem um RPC, não um protocolo.
    rpc_bindings: Vec<CxxRpcBinding>,
    /// Arquivos que não se pareceram com nenhuma das duas formas. Vão para o relatório:
    /// um salto aqui significa que o parser deixou de reconhecer alguma forma.
    ignored: Vec<String>,
}

/// Lê todos os arquivos de um diretório de classes geradas.
fn read_class_dir(dir: &Path, label: &str) -> Result<ClassDir, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("não consegui ler {}: {e}", dir.display()))?;

    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    paths.sort();

    let mut classes = Vec::new();
    let mut rpc_bindings = Vec::new();
    let mut ignored = Vec::new();
    for path in paths {
        let Some(text) = read_latin1(&path) else {
            continue;
        };
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let source = format!("{label}/{file_name}");
        match cxx::parse_class(&text, &source) {
            Ok(class) => classes.push(class),
            // Sem `marshal`, o arquivo ainda pode ser o vínculo de um RPC — que não
            // serializa nada por conta própria, e sim suas structs de argumento e
            // resultado.
            Err(_) => match cxx::parse_rpc_binding(&text, &source) {
                Some(binding) => rpc_bindings.push(binding),
                None => ignored.push(source),
            },
        }
    }

    if classes.is_empty() {
        return Err(format!("nenhuma classe encontrada em {}", dir.display()));
    }
    Ok(ClassDir { classes, rpc_bindings, ignored })
}

fn format_report(schema: &ir::Schema, out: &Path) -> String {
    let com_id = schema.protocols.values().filter(|p| p.id.is_some()).count();
    let rpc_com_id = schema.rpcs.values().filter(|r| r.id.is_some()).count();
    let mut report = format!(
        "pw-rpcgen {} → {}\n  {} estruturas, {} protocolos ({} com id), {} RPCs ({} com id)",
        schema.game_version,
        out.display(),
        schema.structs.len(),
        schema.protocols.len(),
        com_id,
        schema.rpcs.len(),
        rpc_com_id,
    );

    if schema.diagnostics.is_empty() {
        report.push_str("\n  sem diagnósticos");
        return report;
    }

    let mut por_tipo: BTreeMap<&str, usize> = BTreeMap::new();
    for diagnostic in &schema.diagnostics {
        *por_tipo.entry(diagnostic.kind).or_default() += 1;
    }
    report.push_str(&format!("\n  {} diagnóstico(s):", schema.diagnostics.len()));
    for (kind, count) in por_tipo {
        report.push_str(&format!("\n    {kind}: {count}"));
    }
    report.push_str("\n  (detalhes no campo `diagnostics` do JSON)");
    report
}
