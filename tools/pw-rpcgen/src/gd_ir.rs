//! IR dos subcomandos do `GamedataSend` e as verificações que o sustentam.
//!
//! Este módulo junta o que o `gd_cxx` leu de dois arquivos independentes e confere um
//! contra o outro. As verificações são o ponto do exercício — um IR que apenas repete
//! o que o parser achou não prova nada:
//!
//! 1. **Âncoras do enum.** Os fontes numeram uma entrada a cada cinco (`// 5`, `// 10`,
//!    … `// 395`). Cada âncora tem que bater com o valor calculado sequencialmente.
//! 2. **Fixo vs. variável.** `CalcS2CCmdDataSize` escolhe entre `sizeof(T)` e
//!    `CHECK_VALID(T)`. Essa escolha tem que concordar com a presença de um método
//!    `CheckValid` na declaração da struct, que está no *outro* arquivo.
//! 3. **Referências resolvidas.** Toda struct citada pela tabela e todo tipo de campo
//!    que seja uma struct precisam existir na tabela de declarações.
//! 4. **Tamanho calculável.** Uma struct de tamanho fixo cujo tamanho não fecha (campo
//!    não resolvido, array simbólico) não pode ser usada como payload fixo.

use crate::gd_cxx::{CmdEnumEntry, CmdPayload, Ns, PackedStruct, ParseWarning, PayloadKind, VarList};
use crate::gd_srv::ServerSide;
use crate::gd_ty::GdTy;
use crate::json::Json;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: &'static str,
    pub subject: String,
    pub detail: String,
}

/// Um campo já resolvido, com deslocamento a partir do início da struct.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: GdTy,
    pub array_len: Option<usize>,
    /// Deslocamento em bytes, ou `None` a partir do primeiro campo cujo tamanho é
    /// desconhecido — daí em diante nenhum deslocamento é confiável.
    pub offset: Option<usize>,
    pub bytes: Option<usize>,
    pub cxx: String,
    /// Para um campo cujo tipo é outra struct: o nome qualificado a que ele resolveu.
    /// Guardar isto evita repetir a busca de escopo em cada consumidor do IR.
    pub resolved: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    /// Escopo de declaração (`S2C`, `SRV::C2S::CMD`, ou vazio para o escopo global).
    pub scope: String,
    pub fields: Vec<Field>,
    /// Tamanho fixo em bytes. `None` para structs de tamanho variável ou cujo tamanho
    /// não pôde ser calculado.
    pub bytes: Option<usize>,
    pub var_list: Option<VarList>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub id: i64,
    pub ns: Ns,
    pub payload: Option<PayloadKind>,
    /// Nome qualificado da struct do payload **no cliente**, quando resolvida.
    pub payload_struct: Option<String>,
    /// Como o servidor chama este mesmo comando. Os nomes divergem
    /// (`EXG_IVTR_ITEM` ↔ `EXCHANGE_INVENTORY_ITEM`), então o casamento é por id.
    pub server_name: Option<String>,
    /// Struct do payload **no servidor**, incluindo o cabeçalho de 2 bytes.
    pub server_struct: Option<String>,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Schema {
    pub game_version: String,
    pub commands: Vec<Command>,
    pub structs: BTreeMap<String, Struct>,
    pub diagnostics: Vec<Diagnostic>,
    /// Quantas âncoras `// 5`, `// 10`, … foram conferidas. Um número baixo aqui
    /// significa que a verificação não está de fato exercendo o parser.
    pub anchors_checked: usize,
    pub cross: CrossCheck,
}

pub struct Inputs {
    pub game_version: String,
    pub s2c_enum: Vec<CmdEnumEntry>,
    pub c2s_enum: Vec<CmdEnumEntry>,
    pub structs: Vec<PackedStruct>,
    pub payloads: Vec<CmdPayload>,
    pub warnings: Vec<ParseWarning>,
    /// O mesmo protocolo lido dos cabeçalhos do servidor, quando disponível.
    pub server: Option<ServerSide>,
}

/// Resultado da conferência entre os dois lados.
#[derive(Debug, Default, Clone)]
pub struct CrossCheck {
    /// Comandos presentes nos dois enums com o mesmo id.
    pub commands_matched: usize,
    /// Pares de struct comparados campo a campo.
    pub structs_compared: usize,
    /// Pares cujo layout bate exatamente (descontado o cabeçalho do servidor).
    pub structs_agree: usize,
    /// Escalares individuais conferidos.
    pub fields_compared: usize,
    /// Pares que não puderam ser comparados (tamanho variável ou campo irresolúvel de
    /// um dos lados). Contados à parte para que a cobertura não pareça maior do que é.
    pub structs_skipped: usize,
    /// Escalares no mesmo lugar e do mesmo tamanho, mas declarados com sinais
    /// diferentes pelos dois lados. Não afeta os bytes; afeta a interpretação.
    pub sign_mismatches: usize,
}

/// Diz se dois tipos escalares ocupam o mesmo espaço e diferem só na interpretação.
///
/// Cobre `i16`/`u16` e afins, e também `bool` contra um inteiro de 8 bits: os três
/// ocupam um byte, e qual deles o cabeçalho escolheu não muda nada no fio.
fn same_width_class(a: &str, b: &str) -> bool {
    if a == b {
        return false;
    }
    let classe = |k: &str| -> Option<&'static str> {
        match k {
            "bool" | "i8" | "u8" => Some("8"),
            "i16" | "u16" => Some("16"),
            "i32" | "u32" => Some("32"),
            "i64" | "u64" => Some("64"),
            _ => None,
        }
    };
    matches!((classe(a), classe(b)), (Some(x), Some(y)) if x == y)
}

pub fn build(inputs: Inputs) -> Schema {
    let mut schema = Schema {
        game_version: inputs.game_version,
        ..Default::default()
    };

    for w in inputs.warnings {
        schema.diagnostics.push(Diagnostic {
            kind: w.kind,
            subject: w.subject,
            detail: w.detail,
        });
    }

    // --- structs -----------------------------------------------------------
    // As do servidor entram na mesma tabela, sob o prefixo de escopo `SRV::`, para que
    // a resolução de tipos e o cálculo de tamanho sejam exatamente os mesmos.
    let mut todas = inputs.structs;
    if let Some(srv) = &inputs.server {
        todas.extend(srv.structs.iter().cloned());
        for w in &srv.warnings {
            schema.diagnostics.push(Diagnostic {
                kind: w.kind,
                subject: w.subject.clone(),
                detail: w.detail.clone(),
            });
        }
    }

    let mut declared: BTreeMap<String, PackedStruct> = BTreeMap::new();
    for s in todas {
        let key = s.qualified();
        if let Some(prev) = declared.insert(key.clone(), s) {
            // Duas declarações do mesmo nome no mesmo namespace: uma sobrescreveria a
            // outra em silêncio, e metade dos campos sairia errada.
            schema.diagnostics.push(Diagnostic {
                kind: "struct-duplicada",
                subject: key,
                detail: format!("já declarada na linha {}", prev.line),
            });
        }
    }

    for (key, s) in &declared {
        let resolved = resolve_struct(s, &declared, &mut schema.diagnostics);
        schema.structs.insert(key.clone(), resolved);
    }

    // --- comandos ----------------------------------------------------------
    let payloads: BTreeMap<&str, &CmdPayload> = inputs
        .payloads
        .iter()
        .map(|p| (p.command.as_str(), p))
        .collect();

    // Tabelas do servidor indexadas por **id**, nunca por nome: os dois lados chamam
    // o mesmo comando de formas diferentes.
    let mut conflitos: Vec<(String, String, String, usize)> = Vec::new();
    let srv_por_id = |ns: Ns, conflitos: &mut Vec<(String, String, String, usize)>| -> BTreeMap<i64, (String, Option<String>)> {
        let Some(srv) = &inputs.server else {
            return BTreeMap::new();
        };
        let (entries, bindings) = match ns {
            Ns::S2C => (&srv.s2c_enum, &srv.s2c_bindings),
            Ns::C2S => (&srv.c2s_enum, &srv.c2s_bindings),
        };
        // Uma ligação só é aceita se todos os sítios que a observam concordarem. Em
        // `playercmd.cpp` um mesmo comando aparece em vários `switch`, e um conflito
        // entre eles significa que a regra de leitura está errada — melhor não ligar
        // nada do que ligar a struct errada.
        let mut por_comando: BTreeMap<&str, Option<&str>> = BTreeMap::new();
        for b in bindings {
            let e = por_comando
                .entry(b.command.as_str())
                .or_insert(Some(b.struct_name.as_str()));
            if *e != Some(b.struct_name.as_str()) {
                // Descartar em silêncio esconderia o motivo. Um conflito significa que
                // a regra de leitura do `playercmd.cpp` está errada em algum sítio, e a
                // linha é o que permite ir olhar.
                conflitos.push((
                    b.command.clone(),
                    e.unwrap_or("(vários)").to_string(),
                    b.struct_name.clone(),
                    b.line,
                ));
                *e = None;
            }
        }
        entries
            .iter()
            .map(|e| {
                let st = por_comando
                    .get(e.name.as_str())
                    .and_then(|o| *o)
                    .map(str::to_string);
                (e.value, (e.name.clone(), st))
            })
            .collect()
    };
    let srv_s2c = srv_por_id(Ns::S2C, &mut conflitos);
    let srv_c2s = srv_por_id(Ns::C2S, &mut conflitos);
    for (comando, primeira, segunda, linha) in conflitos {
        schema.diagnostics.push(Diagnostic {
            kind: "ligacao-conflitante",
            subject: comando,
            detail: format!(
                "linha {linha}: um sítio liga a `{primeira}`, outro a `{segunda}` — \
                 nenhuma das duas entra no IR"
            ),
        });
    }

    for (entries, ns) in [(&inputs.s2c_enum, Ns::S2C), (&inputs.c2s_enum, Ns::C2S)] {
        let mut seen_ids: BTreeMap<i64, &str> = BTreeMap::new();
        for e in entries {
            // Verificação 1: a âncora do comentário contra o valor sequencial.
            if let Some(anchor) = e.comment_value {
                schema.anchors_checked += 1;
                if anchor != e.value {
                    schema.diagnostics.push(Diagnostic {
                        kind: "ancora-divergente",
                        subject: format!("{}::{}", ns.as_str(), e.name),
                        detail: format!(
                            "linha {}: comentário diz {anchor}, sequência dá {}",
                            e.line, e.value
                        ),
                    });
                }
            }
            if let Some(prev) = seen_ids.insert(e.value, &e.name) {
                schema.diagnostics.push(Diagnostic {
                    kind: "id-duplicado",
                    subject: format!("{}::{}", ns.as_str(), e.name),
                    detail: format!("id {} já usado por {prev}", e.value),
                });
            }

            // A tabela vem de `CalcS2CCmdDataSize`, que só cobre o sentido
            // servidor→cliente. 30 nomes de comando existem nos **dois** enums
            // (`TEAM_LEAVE_PARTY`, `GM_INVISIBLE`, `SELECT_TARGET`, …), então aplicar a
            // tabela ao C2S ligaria comandos do cliente a structs do servidor — layout
            // errado, e sem nada apontando o erro.
            //
            // Ligar o C2S por convenção de nome também não serve: `cmd_<nome
            // minúsculo>` discorda da tabela autoritativa em 46 dos 361 comandos S2C
            // que ela cobre (13%). O mapeamento do C2S precisa vir dos manipuladores do
            // servidor, não daqui.
            let payload = match ns {
                Ns::S2C => payloads.get(e.name.as_str()).map(|p| p.kind.clone()),
                Ns::C2S => None,
            };
            let payload_struct = match &payload {
                Some(PayloadKind::Fixed(t)) | Some(PayloadKind::Variable(t)) => {
                    let qualified = ns.qualify(t);
                    if !declared.contains_key(&qualified) {
                        schema.diagnostics.push(Diagnostic {
                            kind: "struct-ausente",
                            subject: qualified.clone(),
                            detail: format!("citada pelo comando {} mas não declarada", e.name),
                        });
                        None
                    } else {
                        // Verificação 2: a forma escolhida na tabela contra a presença
                        // de `CheckValid` na declaração, que está no outro arquivo.
                        check_shape(&qualified, &payload, &declared, &mut schema.diagnostics);
                        Some(qualified)
                    }
                }
                _ => None,
            };

            // O lado do servidor, casado por id.
            let srv = match ns {
                Ns::S2C => srv_s2c.get(&e.value),
                Ns::C2S => srv_c2s.get(&e.value),
            };
            let (server_name, server_struct) = match srv {
                Some((n, s)) => {
                    schema.cross.commands_matched += 1;
                    let s = s.as_ref().filter(|q| {
                        if declared.contains_key(q.as_str()) {
                            true
                        } else {
                            schema.diagnostics.push(Diagnostic {
                                kind: "struct-servidor-ausente",
                                subject: (*q).clone(),
                                detail: format!(
                                    "ligada ao comando {n} (id {}) mas não declarada em protocol.h",
                                    e.value
                                ),
                            });
                            false
                        }
                    });
                    (Some(n.clone()), s.cloned())
                }
                None => (None, None),
            };

            schema.commands.push(Command {
                name: e.name.clone(),
                id: e.value,
                ns,
                payload,
                payload_struct,
                server_name,
                server_struct,
                line: e.line,
            });
        }
    }

    // Um comando citado pela tabela que não existe no enum significa que o parser do
    // enum parou cedo.
    let known: std::collections::BTreeSet<&str> =
        inputs.s2c_enum.iter().map(|e| e.name.as_str()).collect();
    for p in &inputs.payloads {
        if !known.contains(p.command.as_str()) {
            schema.diagnostics.push(Diagnostic {
                kind: "comando-ausente",
                subject: p.command.clone(),
                detail: format!("linha {}: na tabela, mas fora do enum", p.line),
            });
        }
    }

    if inputs.server.is_some() {
        cross_check(&mut schema);
    }

    schema
}

/// Confere, comando a comando, o layout que o cliente declara contra o que o servidor
/// declara.
///
/// Esta é a verificação mais forte do IR. Cliente e servidor têm cabeçalhos escritos
/// separadamente, com nomes diferentes para tudo — e mesmo assim precisam produzir
/// exatamente os mesmos bytes, ou o jogo não funciona. Se os dois concordam campo a
/// campo, o layout extraído não é uma leitura plausível de um arquivo: é o formato de
/// fio.
///
/// A única diferença sistemática é o cabeçalho: as structs do servidor abrem com os 2
/// bytes do comando (`single_data_header` no S2C, `cmd_header` no C2S) e as do cliente
/// começam depois dele. Por isso a comparação desconta o primeiro campo do servidor e
/// subtrai 2 de cada deslocamento.
/// Uma folha do layout: um escalar concreto, com onde ele cai e quanto ocupa.
///
/// É nisto que a comparação entre os dois lados acontece. Comparar listas de campos
/// não serviria: o servidor agrupa o payload numa struct aninhada (`info`, `data`) onde
/// o cliente escreve os campos soltos. São arranjos diferentes dos **mesmos bytes** —
/// e é a sequência de bytes que o fio carrega.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Leaf {
    offset: usize,
    bytes: usize,
    kind: String,
}

/// Expande uma struct na sequência de escalares que ela põe no fio.
///
/// Devolve `None` se qualquer parte for irresolúvel — um layout parcial comparado
/// contra outro completo produziria uma divergência falsa.
fn flatten(key: &str, structs: &BTreeMap<String, Struct>, base: usize, depth: u32) -> Option<Vec<Leaf>> {
    if depth > 8 {
        return None;
    }
    let s = structs.get(key)?;
    if s.var_list.is_some() {
        return None;
    }

    let mut out = Vec::new();
    for f in &s.fields {
        let offset = base + f.offset?;
        let n = f.array_len.unwrap_or(1);
        match &f.ty {
            GdTy::Struct(_) => {
                // O tipo do campo já foi resolvido para um nome qualificado no
                // `bytes`; aqui precisamos do nome para recursar.
                let inner_key = f.resolved.as_deref()?;
                let unit = structs.get(inner_key)?.bytes?;
                for i in 0..n {
                    out.extend(flatten(inner_key, structs, offset + i * unit, depth + 1)?);
                }
            }
            GdTy::Vec3 => {
                for i in 0..n {
                    for c in 0..3 {
                        out.push(Leaf {
                            offset: offset + i * 12 + c * 4,
                            bytes: 4,
                            kind: "f32".into(),
                        });
                    }
                }
            }
            GdTy::Prim(prim) => {
                let unit = prim.wire_size();
                for i in 0..n {
                    out.push(Leaf {
                        offset: offset + i * unit,
                        bytes: unit,
                        kind: prim.as_str().to_string(),
                    });
                }
            }
            GdTy::Placeholder | GdTy::Unresolved(_) => return None,
        }
    }
    Some(out)
}

fn cross_check(schema: &mut Schema) {
    let comandos: Vec<(String, i64, String, String)> = schema
        .commands
        .iter()
        .filter_map(|c| {
            Some((
                c.name.clone(),
                c.id,
                c.payload_struct.clone()?,
                c.server_struct.clone()?,
            ))
        })
        .collect();

    for (nome, id, cli_key, srv_key) in comandos {
        let (Some(_cli), Some(srv)) = (
            schema.structs.get(&cli_key).cloned(),
            schema.structs.get(&srv_key).cloned(),
        ) else {
            continue;
        };

        // O primeiro campo do servidor tem que ser um dos cabeçalhos de comando. São
        // três, e a diferença importa: `single_data_header` e `cmd_header` têm só o
        // opcode (2 bytes), mas `multi_data_header` tem opcode **e contagem** (4
        // bytes) — e a contagem faz parte do payload do lado do cliente, cujas structs
        // de lista abrem justamente com um `unsigned short count`. Descontar 2 bytes
        // uniformemente é o que alinha os dois casos.
        let cabecalho_ok = srv.fields.first().is_some_and(|f| {
            f.resolved.as_deref().is_some_and(|r| {
                r.ends_with("::cmd_header")
                    || r.ends_with("::single_data_header")
                    || r.ends_with("::multi_data_header")
            })
        });
        if !cabecalho_ok {
            schema.diagnostics.push(Diagnostic {
                kind: "cabecalho-servidor-inesperado",
                subject: srv_key.clone(),
                detail: format!(
                    "comando {nome} (id {id}): o primeiro campo não é um cabeçalho de comando"
                ),
            });
            continue;
        }

        // Achata os dois lados na sequência de escalares que cada um põe no fio. Os
        // dois usam arranjos diferentes (o servidor agrupa numa struct aninhada), mas
        // os bytes têm que ser os mesmos.
        let (Some(cli_leaves), Some(srv_leaves)) = (
            flatten(&cli_key, &schema.structs, 0, 0),
            flatten(&srv_key, &schema.structs, 0, 0),
        ) else {
            // Um dos lados tem tamanho variável ou campo irresolúvel. Não dá para
            // comparar, e forçar produziria uma divergência falsa.
            schema.cross.structs_skipped += 1;
            continue;
        };

        schema.cross.structs_compared += 1;
        let mut divergencias: Vec<String> = Vec::new();

        // Descarta os 2 bytes de cabeçalho do servidor e realinha o resto.
        let srv_payload: Vec<Leaf> = srv_leaves
            .iter()
            .filter(|l| l.offset >= 2)
            .map(|l| Leaf { offset: l.offset - 2, ..l.clone() })
            .collect();

        if srv_payload.len() != cli_leaves.len() {
            divergencias.push(format!(
                "quantidade de escalares: cliente {}, servidor {}",
                cli_leaves.len(),
                srv_payload.len()
            ));
        }

        let mut sinais: Vec<String> = Vec::new();
        for (i, (c, s)) in cli_leaves.iter().zip(srv_payload.iter()).enumerate() {
            schema.cross.fields_compared += 1;
            if c == s {
                continue;
            }
            // Mesmo lugar, mesmo tamanho, só o sinal difere: os bytes no fio são
            // idênticos, o que muda é como cada lado os interpreta. Isso é um fato
            // sobre o protocolo, não um erro de extração — mas quem for escrever o
            // decodificador precisa saber, porque `-1` e `65535` são o mesmo byte e
            // significados opostos.
            if c.offset == s.offset && c.bytes == s.bytes && same_width_class(&c.kind, &s.kind) {
                sinais.push(format!("escalar {i}@{}: {} / {}", c.offset, c.kind, s.kind));
                continue;
            }
            divergencias.push(format!(
                "escalar {i}: cliente {}@{} ({}), servidor {}@{} ({})",
                c.bytes, c.offset, c.kind, s.bytes, s.offset, s.kind
            ));
        }
        divergencias.truncate(4);

        if !sinais.is_empty() {
            schema.cross.sign_mismatches += sinais.len();
            sinais.truncate(6);
            schema.diagnostics.push(Diagnostic {
                kind: "sinal-divergente",
                subject: format!("{cli_key} vs {srv_key}"),
                detail: format!("comando {nome} (id {id}): {}", sinais.join("; ")),
            });
        }

        if divergencias.is_empty() {
            schema.cross.structs_agree += 1;
        } else {
            schema.diagnostics.push(Diagnostic {
                kind: "layout-divergente",
                subject: format!("{cli_key} vs {srv_key}"),
                detail: format!("comando {nome} (id {id}): {}", divergencias.join("; ")),
            });
        }
    }
}

/// Resolve o nome de um tipo a partir do escopo onde ele foi escrito.
///
/// C++ procura um nome no escopo atual e depois nos escopos que o contêm, mais os que
/// um `using namespace` tornou visíveis. É essa busca que este função reproduz — e sem
/// ela nada do lado do servidor resolve: `C2S::CMD::player_move` referencia
/// `move_info`, que mora em `C2S::INFO` e só é visível pelo `using namespace INFO;` no
/// topo do bloco `CMD`.
///
/// A ordem importa: do mais específico para o mais geral, para que um tipo aninhado
/// sombreie um homônimo de fora, como em C++.
fn resolve_ref(
    owner: &str,
    scope: &str,
    usings: &[String],
    name: &str,
    declared: &BTreeMap<String, PackedStruct>,
) -> Option<String> {
    let mut candidatos: Vec<String> = Vec::new();

    // 1. tipo aninhado na própria struct
    candidatos.push(format!("{owner}::{name}"));

    // 2. o escopo da struct e cada escopo que o contém, até o global
    let mut atual = scope;
    loop {
        candidatos.push(crate::gd_cxx::qualify(atual, name));
        match atual.rfind("::") {
            Some(i) => atual = &atual[..i],
            None => break,
        }
    }
    candidatos.push(name.to_string());

    // 3. os escopos abertos por `using namespace`
    for u in usings {
        candidatos.push(format!("{u}::{name}"));
    }

    candidatos.into_iter().find(|c| declared.contains_key(c))
}

/// Distingue comandos de verdade dos marcadores que dividem espaço no mesmo enum.
///
/// Duas entradas dos enums não são comandos despacháveis: `PROTOCOL_COMMAND = -1`, que
/// os fontes anotam como "Reserved for protocol", e `NUM_C2SCMD`, que é a **contagem**
/// de comandos C2S e por acaso cai no valor 180. Quem montar uma tabela de despacho a
/// partir deste IR registraria um manipulador fantasma para 180 se o IR não dissesse
/// que aquilo não é um comando.
fn command_role(name: &str, id: i64) -> &'static str {
    if id < 0 {
        "reserved"
    } else if name.starts_with("NUM_") {
        "count"
    } else {
        "command"
    }
}

/// Confere a forma declarada contra a forma usada na tabela.
fn check_shape(
    qualified: &str,
    payload: &Option<PayloadKind>,
    declared: &BTreeMap<String, PackedStruct>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(s) = declared.get(qualified) else { return };
    let has_check_valid = s.var_list.is_some();
    match payload {
        Some(PayloadKind::Fixed(_)) if has_check_valid => diagnostics.push(Diagnostic {
            kind: "forma-divergente",
            subject: qualified.to_string(),
            detail: "a tabela usa sizeof(), mas a struct declara CheckValid".to_string(),
        }),
        Some(PayloadKind::Variable(_)) if !has_check_valid => diagnostics.push(Diagnostic {
            kind: "forma-divergente",
            subject: qualified.to_string(),
            detail: "a tabela usa CHECK_VALID, mas a struct não declara CheckValid".to_string(),
        }),
        _ => {}
    }
}

/// Calcula deslocamentos e tamanho de uma struct empacotada.
fn resolve_struct(
    s: &PackedStruct,
    declared: &BTreeMap<String, PackedStruct>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Struct {
    let mut fields = Vec::new();
    let mut offset: Option<usize> = Some(0);

    for f in &s.fields {
        let mut resolved: Option<String> = None;
        let unit = match &f.ty {
            GdTy::Struct(name) => {
                let owner = s.qualified();
                match resolve_ref(&owner, &s.scope, &s.usings, name, declared) {
                    Some(q) => {
                        resolved = Some(q.clone());
                        struct_bytes(&declared[&q], declared, &mut Vec::new())
                    }
                    None => {
                        let qualified = crate::gd_cxx::qualify(&s.scope, name);
                        // Tipo usado por um campo mas nunca declarado neste cabeçalho:
                        // mora em outro header do cliente que não está nos fontes que
                        // temos (`ROLEEXTPROP*`, `player_info_2`, `player_info_3`).
                        // O campo fica sem tamanho, e por isso a struct inteira também
                        // — em vez de receber um deslocamento inventado.
                        diagnostics.push(Diagnostic {
                            kind: "tipo-externo",
                            subject: qualified,
                            detail: format!(
                                "campo `{}` de {} (linha {}): declarado fora de EC_GPDataType.h",
                                f.name,
                                s.qualified(),
                                f.line
                            ),
                        });
                        None
                    }
                }
            }
            other => other.inline_bytes(),
        };

        let bytes = unit.map(|u| u * f.array_len.unwrap_or(1));
        let this_offset = offset;
        offset = match (offset, bytes) {
            (Some(o), Some(b)) => Some(o + b),
            // A partir de um campo de tamanho desconhecido, nenhum deslocamento
            // seguinte é confiável — e um deslocamento errado é pior que nenhum.
            _ => None,
        };

        fields.push(Field {
            name: f.name.clone(),
            ty: f.ty.clone(),
            array_len: f.array_len,
            offset: this_offset,
            bytes,
            cxx: f.cxx.clone(),
            resolved,
        });
    }

    // O tipo dos elementos da lista também é um nome escrito no escopo da struct, e
    // precisa da mesma resolução dos campos — senão o IR publica `info_npc` em vez de
    // `S2C::info_npc`, e quem consumir não acha a declaração.
    let owner = s.qualified();
    let var_list = s.var_list.as_ref().map(|v| match v {
        VarList::Placeholder { element } => VarList::Placeholder {
            element: resolve_ref(&owner, &s.scope, &s.usings, element, declared)
                .unwrap_or_else(|| crate::gd_cxx::qualify(&s.scope, element)),
        },
        VarList::FlexArray { element, field } => VarList::FlexArray {
            element: resolve_ref(&owner, &s.scope, &s.usings, element, declared)
                .unwrap_or_else(|| crate::gd_cxx::qualify(&s.scope, element)),
            field: field.clone(),
        },
        other => other.clone(),
    });

    // Structs de tamanho variável não têm um tamanho fixo, por definição. Uma struct
    // sem nenhum membro de dados também não recebe tamanho: `common_data_notify` e
    // `common_data_list` existem só para declarar um `_node` aninhado, e em C++
    // `sizeof` de uma struct vazia é 1, não 0 — anunciar 0 seria afirmar um número que
    // o compilador contradiz.
    let bytes = if var_list.is_some() || fields.is_empty() {
        None
    } else {
        offset
    };

    Struct {
        name: s.name.clone(),
        scope: s.scope.clone(),
        fields,
        bytes,
        var_list,
        line: s.line,
    }
}

/// Tamanho de uma struct, com proteção contra ciclos de declaração.
fn struct_bytes(
    s: &PackedStruct,
    declared: &BTreeMap<String, PackedStruct>,
    stack: &mut Vec<String>,
) -> Option<usize> {
    let key = s.qualified();
    if stack.contains(&key) {
        return None;
    }
    stack.push(key);

    let mut total = 0usize;
    for f in &s.fields {
        let unit = match &f.ty {
            GdTy::Struct(name) => {
                let owner = s.qualified();
                let q = resolve_ref(&owner, &s.scope, &s.usings, name, declared)?;
                struct_bytes(&declared[&q], declared, stack)?
            }
            other => other.inline_bytes()?,
        };
        total += unit * f.array_len.unwrap_or(1);
    }
    stack.pop();
    Some(total)
}

impl Schema {
    pub fn to_json(&self) -> Json {
        let commands = |ns: Ns| {
            Json::array(self.commands.iter().filter(|c| c.ns == ns).map(|c| {
                Json::object([
                    ("name", Json::str(c.name.clone())),
                    ("id", Json::Int(c.id)),
                    (
                        "payload",
                        match &c.payload {
                            Some(PayloadKind::Fixed(_)) => Json::str("fixed"),
                            Some(PayloadKind::Variable(_)) => Json::str("variable"),
                            Some(PayloadKind::Empty) => Json::str("empty"),
                            Some(PayloadKind::Unhandled) => Json::str("unhandled"),
                            None => Json::Null,
                        },
                    ),
                    (
                        "struct",
                        match &c.payload_struct {
                            Some(s) => Json::str(s.clone()),
                            None => Json::Null,
                        },
                    ),
                    (
                        "server_name",
                        match &c.server_name {
                            Some(n) => Json::str(n.clone()),
                            None => Json::Null,
                        },
                    ),
                    (
                        "server_struct",
                        match &c.server_struct {
                            Some(s) => Json::str(s.clone()),
                            None => Json::Null,
                        },
                    ),
                    ("role", Json::str(command_role(&c.name, c.id))),
                    ("source_line", Json::Int(c.line as i64)),
                ])
            }))
        };

        Json::object([
            ("ir_version", Json::Int(1)),
            ("game_version", Json::str(self.game_version.clone())),
            ("kind", Json::str("gamedata-commands")),
            (
                "wire",
                Json::object([
                    ("byte_order", Json::str("little-endian")),
                    ("packing", Json::Int(1)),
                    ("length_prefix", Json::str("none")),
                    ("target", Json::str("i386-32")),
                    (
                        "note",
                        Json::str(
                            "Payload copiado por memcpy cru. Modelo distinto do GNET \
                             (big-endian, CompactUINT) descrito em gnet_153.json.",
                        ),
                    ),
                ]),
            ),
            (
                "verification",
                Json::object([
                    ("enum_anchors_checked", Json::Int(self.anchors_checked as i64)),
                    ("diagnostics", Json::Int(self.diagnostics.len() as i64)),
                    (
                        "structs_sized",
                        Json::Int(self.structs.values().filter(|s| s.bytes.is_some()).count() as i64),
                    ),
                    (
                        "fields_unresolved",
                        Json::Int(
                            self.structs
                                .values()
                                .flat_map(|s| s.fields.iter())
                                .filter(|f| f.ty.is_unresolved())
                                .count() as i64,
                        ),
                    ),
                ]),
            ),
            (
                "cross_check",
                Json::object([
                    ("commands_matched", Json::Int(self.cross.commands_matched as i64)),
                    ("structs_compared", Json::Int(self.cross.structs_compared as i64)),
                    ("structs_agree", Json::Int(self.cross.structs_agree as i64)),
                    ("structs_skipped", Json::Int(self.cross.structs_skipped as i64)),
                    ("scalars_compared", Json::Int(self.cross.fields_compared as i64)),
                    ("sign_mismatches", Json::Int(self.cross.sign_mismatches as i64)),
                    (
                        "note",
                        Json::str(
                            "Cliente e servidor descrevem o mesmo protocolo em cabeçalhos \
                             independentes, com nomes diferentes para tudo. O casamento é \
                             por id numérico, e a comparação é sobre a sequência de \
                             escalares no fio (o servidor agrupa o payload numa struct \
                             aninhada onde o cliente achata os campos), descontados os 2 \
                             bytes de cabeçalho que só as structs do servidor incluem.",
                        ),
                    ),
                ]),
            ),
            (
                "coverage",
                Json::object([
                    (
                        "s2c_with_payload",
                        Json::Int(
                            self.commands
                                .iter()
                                .filter(|c| c.ns == Ns::S2C && c.payload_struct.is_some())
                                .count() as i64,
                        ),
                    ),
                    (
                        "c2s_with_payload",
                        Json::Int(
                            self.commands
                                .iter()
                                .filter(|c| c.ns == Ns::C2S && c.server_struct.is_some())
                                .count() as i64,
                        ),
                    ),
                    (
                        "c2s_note",
                        Json::str(
                            "O cliente 1.5.3 só traz tabela de tamanhos para o sentido \
                             S2C (CalcS2CCmdDataSize). Ligar comandos C2S às suas \
                             structs por convenção de nome não serve: `cmd_<nome \
                             minúsculo>` discorda da tabela autoritativa em 46 dos 361 \
                             comandos S2C que ela cobre. O mapeamento C2S precisa vir \
                             dos manipuladores do servidor.",
                        ),
                    ),
                ]),
            ),
            (
                "commands",
                Json::object([("s2c", commands(Ns::S2C)), ("c2s", commands(Ns::C2S))]),
            ),
            (
                "structs",
                Json::Object(
                    self.structs
                        .iter()
                        .map(|(k, s)| (k.clone(), s.to_json()))
                        .collect(),
                ),
            ),
            (
                "diagnostics",
                Json::array(self.diagnostics.iter().map(|d| {
                    Json::object([
                        ("kind", Json::str(d.kind)),
                        ("subject", Json::str(d.subject.clone())),
                        ("detail", Json::str(d.detail.clone())),
                    ])
                })),
            ),
        ])
    }
}

impl Struct {
    fn to_json(&self) -> Json {
        Json::object([
            ("name", Json::str(self.name.clone())),
            ("scope", Json::str(self.scope.clone())),
            (
                "bytes",
                match self.bytes {
                    Some(b) => Json::Int(b as i64),
                    None => Json::Null,
                },
            ),
            (
                "role",
                if self.fields.is_empty() {
                    Json::str("scope-only")
                } else {
                    Json::str("data")
                },
            ),
            (
                "variable",
                match &self.var_list {
                    Some(VarList::Placeholder { element }) => Json::object([
                        ("form", Json::str("placeholder")),
                        ("element", Json::str(element.clone())),
                    ]),
                    Some(VarList::FlexArray { element, field }) => Json::object([
                        ("form", Json::str("flex-array")),
                        ("element", Json::str(element.clone())),
                        ("field", Json::str(field.clone())),
                    ]),
                    Some(VarList::Initialize) => Json::object([
                        ("form", Json::str("initialize")),
                        (
                            "note",
                            Json::str(
                                "Serialização manual por Extract(); não é memcpy. \
                                 Contém contêineres com contagem explícita.",
                            ),
                        ),
                    ]),
                    Some(VarList::Conditional) => Json::object([
                        ("form", Json::str("conditional")),
                        (
                            "note",
                            Json::str(
                                "Campos finais condicionais a bits do campo `state`. \
                                 O tamanho depende do conteúdo.",
                            ),
                        ),
                    ]),
                    Some(VarList::Unknown) => Json::object([("form", Json::str("unknown"))]),
                    None => Json::Null,
                },
            ),
            (
                "fields",
                Json::array(self.fields.iter().map(|f| {
                    Json::object([
                        ("name", Json::str(f.name.clone())),
                        ("type", f.ty.to_json()),
                        (
                            "array_len",
                            match f.array_len {
                                Some(n) => Json::Int(n as i64),
                                None => Json::Null,
                            },
                        ),
                        (
                            "offset",
                            match f.offset {
                                Some(o) => Json::Int(o as i64),
                                None => Json::Null,
                            },
                        ),
                        (
                            "bytes",
                            match f.bytes {
                                Some(b) => Json::Int(b as i64),
                                None => Json::Null,
                            },
                        ),
                        ("cxx", Json::str(f.cxx.clone())),
                        (
                            // Nome qualificado a que um campo de tipo struct resolveu.
                            // Publicado para que quem consome o IR não tenha de repetir
                            // a busca de escopo (que precisa conhecer os `using
                            // namespace` do servidor para dar o mesmo resultado).
                            "resolved",
                            match &f.resolved {
                                Some(r) => Json::str(r.clone()),
                                None => Json::Null,
                            },
                        ),
                    ])
                })),
            ),
            ("source_line", Json::Int(self.line as i64)),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gd_cxx::{self, Ns};

    fn structs_from(src: &str, ns: Ns) -> Vec<PackedStruct> {
        let lines: Vec<&str> = src.lines().collect();
        let mut w = Vec::new();
        gd_cxx::parse_packed_structs(&lines, 0, lines.len(), ns.as_str(), &mut w)
    }

    fn base(structs: Vec<PackedStruct>) -> Inputs {
        Inputs {
            game_version: "1.5.3".into(),
            s2c_enum: vec![],
            c2s_enum: vec![],
            structs,
            payloads: vec![],
            warnings: vec![],
            server: None,
        }
    }

    #[test]
    fn deslocamentos_sao_a_soma_crua_sem_alinhamento() {
        // Com alinhamento natural, `id` cairia em 4 e a struct teria 12 bytes. Sob
        // pack(1) são 7 — é essa a diferença que quebra o protocolo se for ignorada.
        let s = structs_from(
            "struct s\n{\n\tunsigned char flag;\n\tint id;\n\tunsigned short n;\n};",
            Ns::S2C,
        );
        let schema = build(base(s));
        let st = &schema.structs["S2C::s"];
        assert_eq!(st.fields[0].offset, Some(0));
        assert_eq!(st.fields[1].offset, Some(1));
        assert_eq!(st.fields[2].offset, Some(5));
        assert_eq!(st.bytes, Some(7));
    }

    #[test]
    fn arrays_e_vetores_entram_no_tamanho() {
        let s = structs_from(
            "struct s\n{\n\tA3DVECTOR3 pos;\n\tint reserved[10];\n};",
            Ns::S2C,
        );
        let schema = build(base(s));
        assert_eq!(schema.structs["S2C::s"].bytes, Some(12 + 40));
    }

    #[test]
    fn struct_aninhada_contribui_com_seu_tamanho() {
        let s = structs_from(
            "struct inner\n{\n\tint a;\n\tshort b;\n};\nstruct outer\n{\n\tinner x;\n\tchar c;\n};",
            Ns::S2C,
        );
        let schema = build(base(s));
        assert_eq!(schema.structs["S2C::inner"].bytes, Some(6));
        assert_eq!(schema.structs["S2C::outer"].bytes, Some(7));
    }

    #[test]
    fn campo_nao_resolvido_apaga_os_deslocamentos_seguintes() {
        // Um deslocamento errado é pior que um deslocamento ausente: ele passa
        // despercebido na revisão e produz um campo lido do lugar errado.
        let s = structs_from(
            "struct s\n{\n\tint a;\n\tabase::vector<int> v;\n\tint b;\n};",
            Ns::S2C,
        );
        let schema = build(base(s));
        let st = &schema.structs["S2C::s"];
        assert_eq!(st.fields[0].offset, Some(0));
        assert_eq!(st.fields[2].offset, None);
        assert_eq!(st.bytes, None);
    }

    #[test]
    fn struct_variavel_nao_recebe_tamanho_fixo() {
        let s = structs_from(
            "struct s\n{\n\tunsigned short count;\n\tBYTE placeholder;\n\tbool CheckValid(size_t b, size_t& sz) const\n\t{\n\t\treinterpret_cast<const info_npc*>(0)->CheckValid(b, sz);\n\t\treturn true;\n\t}\n};",
            Ns::S2C,
        );
        let schema = build(base(s));
        assert_eq!(schema.structs["S2C::s"].bytes, None);
    }

    #[test]
    fn marcadores_do_enum_nao_sao_comandos_despachaveis() {
        // `NUM_C2SCMD` é a contagem de comandos C2S e cai no valor 180; despachá-lo
        // como comando criaria um manipulador fantasma. `PROTOCOL_COMMAND = -1` é
        // reservado nos próprios fontes.
        assert_eq!(command_role("PROTOCOL_COMMAND", -1), "reserved");
        assert_eq!(command_role("NUM_C2SCMD", 180), "count");
        assert_eq!(command_role("PLAYER_MOVE", 0), "command");
        // `PRODUCE_END` é um comando de verdade ("produção terminou"), não um marcador.
        assert_eq!(command_role("PRODUCE_END", 102), "command");
    }

    #[test]
    fn struct_sem_membros_de_dados_nao_recebe_tamanho_zero() {
        // Em C++, `sizeof` de uma struct vazia é 1, não 0. `common_data_notify` e
        // `common_data_list` só existem para declarar um `_node` aninhado; anunciar
        // 0 bytes seria afirmar um número que o compilador contradiz.
        let s = structs_from(
            "struct so_escopo\n{\n\tstruct _node\n\t{\n\t\tint key;\n\t\tint value;\n\t};\n};",
            Ns::S2C,
        );
        let schema = build(base(s));
        let st = &schema.structs["S2C::so_escopo"];
        assert!(st.fields.is_empty());
        assert_eq!(st.bytes, None);
        // O tipo aninhado que ela declara continua no IR, com seu tamanho.
        assert_eq!(schema.structs["S2C::so_escopo::_node"].bytes, Some(8));
    }

    #[test]
    fn ancora_divergente_do_enum_vira_diagnostico() {
        let mut inputs = base(vec![]);
        inputs.s2c_enum = vec![
            CmdEnumEntry { name: "A".into(), value: 0, comment_value: None, line: 1 },
            // Diz 5, mas a sequência dá 1: exatamente o erro que a âncora existe para pegar.
            CmdEnumEntry { name: "B".into(), value: 1, comment_value: Some(5), line: 2 },
        ];
        let schema = build(inputs);
        assert_eq!(schema.anchors_checked, 1);
        assert_eq!(schema.diagnostics.len(), 1);
        assert_eq!(schema.diagnostics[0].kind, "ancora-divergente");
    }

    #[test]
    fn tabela_e_declaracao_em_desacordo_viram_diagnostico() {
        // A tabela diz tamanho fixo; a declaração tem CheckValid. Os dois fatos vêm de
        // arquivos diferentes, então essa é uma verificação cruzada de verdade.
        let s = structs_from(
            "struct v\n{\n\tunsigned short count;\n\tBYTE placeholder;\n\tbool CheckValid(size_t b, size_t& sz) const\n\t{\n\t\treinterpret_cast<const e*>(0)->CheckValid(b, sz);\n\t\treturn true;\n\t}\n};",
            Ns::S2C,
        );
        let mut inputs = base(s);
        inputs.s2c_enum = vec![CmdEnumEntry {
            name: "CMD".into(), value: 0, comment_value: None, line: 1,
        }];
        inputs.payloads = vec![CmdPayload {
            command: "CMD".into(),
            kind: PayloadKind::Fixed("v".into()),
            line: 10,
        }];
        let schema = build(inputs);
        assert!(schema.diagnostics.iter().any(|d| d.kind == "forma-divergente"));
    }

    #[test]
    fn struct_citada_e_nao_declarada_vira_diagnostico() {
        let mut inputs = base(vec![]);
        inputs.s2c_enum = vec![CmdEnumEntry {
            name: "CMD".into(), value: 0, comment_value: None, line: 1,
        }];
        inputs.payloads = vec![CmdPayload {
            command: "CMD".into(),
            kind: PayloadKind::Fixed("nao_existe".into()),
            line: 10,
        }];
        let schema = build(inputs);
        assert!(schema.diagnostics.iter().any(|d| d.kind == "struct-ausente"));
        assert_eq!(schema.commands[0].payload_struct, None);
    }

    #[test]
    fn a_tabela_do_cliente_nao_contamina_os_comandos_c2s() {
        // `CalcS2CCmdDataSize` só descreve o sentido servidor→cliente, mas 30 nomes de
        // comando existem nos dois enums. Aplicar a tabela ao C2S ligaria um comando do
        // cliente ao layout do servidor — errado, e sem nada apontando o erro.
        let mut s = structs_from("struct cmd_x\n{\n\tint a;\n};", Ns::S2C);
        s.extend(structs_from("struct cmd_x\n{\n\tint a;\n\tint b;\n};", Ns::C2S));
        let mut inputs = base(s);
        inputs.s2c_enum = vec![CmdEnumEntry { name: "X".into(), value: 0, comment_value: None, line: 1 }];
        inputs.c2s_enum = vec![CmdEnumEntry { name: "X".into(), value: 0, comment_value: None, line: 2 }];
        inputs.payloads = vec![CmdPayload {
            command: "X".into(), kind: PayloadKind::Fixed("cmd_x".into()), line: 10,
        }];
        let schema = build(inputs);
        let s2c = schema.commands.iter().find(|c| c.ns == Ns::S2C).unwrap();
        let c2s = schema.commands.iter().find(|c| c.ns == Ns::C2S).unwrap();
        assert_eq!(s2c.payload_struct.as_deref(), Some("S2C::cmd_x"));
        assert_eq!(c2s.payload_struct, None);
        assert_eq!(c2s.payload, None);
        assert!(schema.diagnostics.is_empty(), "{:?}", schema.diagnostics);
    }

    #[test]
    fn comandos_homonimos_resolvem_para_namespaces_diferentes() {
        let mut s = structs_from("struct cmd_x\n{\n\tint a;\n};", Ns::S2C);
        s.extend(structs_from("struct cmd_x\n{\n\tint a;\n\tint b;\n};", Ns::C2S));
        let mut inputs = base(s);
        inputs.s2c_enum = vec![CmdEnumEntry { name: "X".into(), value: 0, comment_value: None, line: 1 }];
        inputs.c2s_enum = vec![CmdEnumEntry { name: "X".into(), value: 0, comment_value: None, line: 2 }];
        inputs.payloads = vec![CmdPayload {
            command: "X".into(), kind: PayloadKind::Fixed("cmd_x".into()), line: 10,
        }];
        let schema = build(inputs);
        // O mesmo nome de struct nos dois namespaces tem campos diferentes; qualificar
        // o nome é o que impede uma declaração de sobrescrever a outra.
        assert_eq!(schema.structs["S2C::cmd_x"].bytes, Some(4));
        assert_eq!(schema.structs["C2S::cmd_x"].bytes, Some(8));
    }
}
