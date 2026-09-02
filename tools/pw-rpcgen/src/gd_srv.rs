//! O mesmo protocolo do mundo 3D, lido pelo **lado do servidor**.
//!
//! O cliente e o servidor têm cabeçalhos independentes que descrevem o mesmo formato de
//! fio. Como os dois precisam concordar byte a byte para o jogo funcionar, um serve de
//! prova do outro — e é essa a razão principal deste módulo existir, além de preencher
//! a lacuna do C2S.
//!
//! Três arquivos são lidos, em `cgame/`:
//!
//! * `common/types.h` — `S2C::single_data_header` e `C2S::cmd_header`, os dois campos
//!   de 2 bytes que abrem as structs do servidor;
//! * `common/protocol.h` — os enums de comando dos dois sentidos e as structs, em
//!   `S2C::{INFO,CMD}` e `C2S::{INFO,CMD}`;
//! * `common/protocol_imp.h` — as especializações `Make<CMD::x>` que **emitem** cada
//!   comando S2C, e que por isso ligam id → struct;
//! * `gs/playercmd.cpp` — o `switch` que **recebe** cada comando C2S, ligando id →
//!   struct pelo outro sentido.
//!
//! ## Duas diferenças de forma que importam
//!
//! 1. **As structs do servidor incluem o cabeçalho de 2 bytes** (`single_data_header`
//!    no S2C, `cmd_header` no C2S) como primeiro campo; as do cliente começam depois
//!    dele. Comparar os dois lados sem descontar isso dá 2 bytes de diferença em todo
//!    campo.
//! 2. **Os nomes não batem.** `EXG_IVTR_ITEM` no cliente é `EXCHANGE_INVENTORY_ITEM` no
//!    servidor; `cmd_exg_ivtr_item` é `exchange_inventory_item`. O casamento entre os
//!    dois lados é **por id numérico**, nunca por nome.

use crate::gd_cxx::{
    self, CmdEnumEntry, Consts, PackedStruct, ParseWarning, Region,
};

/// Prefixo de escopo das declarações do servidor, para que não colidam com as do
/// cliente na mesma tabela de structs.
pub const SRV: &str = "SRV";

/// Ligação id de comando → struct, com a proveniência registrada.
#[derive(Debug, Clone)]
pub struct Binding {
    /// Nome do comando **como o servidor o chama**.
    pub command: String,
    /// Nome qualificado da struct (`SRV::C2S::CMD::player_move`).
    pub struct_name: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct ServerSide {
    pub structs: Vec<PackedStruct>,
    pub s2c_enum: Vec<CmdEnumEntry>,
    pub c2s_enum: Vec<CmdEnumEntry>,
    pub s2c_bindings: Vec<Binding>,
    pub c2s_bindings: Vec<Binding>,
    pub warnings: Vec<ParseWarning>,
}

/// Lê `types.h`: os dois cabeçalhos de comando, que as structs de `protocol.h`
/// referenciam pelo nome curto.
pub fn parse_types_h(lines: &[&str], consts: &Consts, out: &mut ServerSide) {
    for ns in ["S2C", "C2S"] {
        let Some(region) = gd_cxx::find_namespace(lines, ns) else {
            continue;
        };
        let scope = format!("{SRV}::{ns}");
        out.structs.extend(gd_cxx::parse_packed_structs_with(
            lines,
            region.start,
            region.end,
            &scope,
            &[],
            consts,
            &mut out.warnings,
        ));
    }
}

/// Lê `protocol.h`: os dois enums de comando e as structs de `INFO` e `CMD`.
pub fn parse_protocol_h(lines: &[&str], consts: &Consts, out: &mut ServerSide) -> Result<(), String> {
    for ns in ["S2C", "C2S"] {
        let region = gd_cxx::find_namespace(lines, ns)
            .ok_or_else(|| format!("não achei `namespace {ns}` em protocol.h"))?;

        // Os sub-namespaces `INFO` (tipos compartilhados) e `CMD` (as structs de
        // comando). O bloco `CMD` abre com `using namespace INFO;`, sem o qual metade
        // dos campos vira referência pendurada.
        let mut info_scope = None;
        for sub in ["INFO", "CMD"] {
            let Some(sub_region) = find_namespace_in(lines, region, sub) else {
                continue;
            };
            let scope = format!("{SRV}::{ns}::{sub}");
            let usings: Vec<String> = info_scope.iter().cloned().collect();
            out.structs.extend(gd_cxx::parse_packed_structs_with(
                lines,
                sub_region.start,
                sub_region.end,
                &scope,
                &usings,
                consts,
                &mut out.warnings,
            ));
            if sub == "INFO" {
                info_scope = Some(scope);
            }
        }

        let cmd_enum = find_command_enum(lines, region).ok_or_else(|| {
            format!("não achei o enum de comandos em `namespace {ns}` de protocol.h")
        })?;
        let entries = gd_cxx::parse_command_enum(lines, cmd_enum.start, cmd_enum.end);
        match ns {
            "S2C" => out.s2c_enum = entries,
            _ => out.c2s_enum = entries,
        }
    }
    Ok(())
}

/// Encontra um `namespace <nome>` dentro de outra região.
fn find_namespace_in(lines: &[&str], outer: Region, name: &str) -> Option<Region> {
    let slice = &lines[outer.start..outer.end];
    let inner = gd_cxx::find_namespace(slice, name)?;
    Some(Region {
        start: outer.start + inner.start,
        end: outer.start + inner.end,
    })
}

/// Encontra o enum de comandos: o **primeiro `enum` anônimo** diretamente dentro do
/// namespace.
///
/// Os enums nomeados que aparecem antes (`MOVE_MODE`, `FORCE_ATTACK_MASK`,
/// `REFUSE_BLESS_MASK`) descrevem outra coisa, e os anônimos que aparecem depois estão
/// mais fundo ou fora do caminho. Se a escolha estiver errada, a conferência cruzada
/// por id acusa na hora: os ids não alinhariam com os do cliente em lugar nenhum.
fn find_command_enum(lines: &[&str], region: Region) -> Option<Region> {
    let mut depth = 0i32;
    let mut i = region.start;
    while i < region.end {
        let code = strip_line_comment(lines[i]);
        let trimmed = code.trim();

        // Profundidade 1 = dentro do namespace, fora de qualquer outro bloco.
        if depth == 1 && trimmed.starts_with("enum") {
            let after = trimmed["enum".len()..].trim();
            let anonimo = after.is_empty() || after.starts_with('{');
            if anonimo {
                let open = (i..region.end).find(|&k| lines[k].contains('{'))?;
                let close = (open..region.end).find(|&k| strip_line_comment(lines[k]).contains("};"))?;
                return Some(Region { start: open + 1, end: close });
            }
        }
        depth += code.matches('{').count() as i32 - code.matches('}').count() as i32;
        i += 1;
    }
    None
}

/// Lê `protocol_imp.h` e devolve as ligações id → struct do sentido S2C.
///
/// Cada especialização `struct Make<CMD::x>` monta um pacote, e a primeira coisa que
/// ela escreve é o cabeçalho com a constante do comando:
///
/// ```text
/// struct Make<CMD::exchange_inventory_item>
/// {
///     ...
///         Make<single_data_header>::From(wrapper, EXCHANGE_INVENTORY_ITEM);
/// ```
///
/// É a ligação autoritativa do lado que **envia** — a contrapartida exata do
/// `CalcS2CCmdDataSize`, que é do lado que recebe.
pub fn parse_s2c_bindings(lines: &[&str]) -> Vec<Binding> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let code = strip_line_comment(lines[i]);
        let Some(struct_name) = between_once(&code, "struct Make<CMD::", ">") else {
            i += 1;
            continue;
        };
        let struct_name = struct_name.trim();
        if struct_name.is_empty() || !is_ident(struct_name) {
            i += 1;
            continue;
        }

        // Varre o corpo da especialização atrás da constante do comando.
        let mut depth = 0i32;
        let mut aberto = false;
        let mut k = i;
        while k < lines.len() {
            let body = strip_line_comment(lines[k]);
            if body.contains('{') {
                aberto = true;
            }
            depth += body.matches('{').count() as i32 - body.matches('}').count() as i32;

            if let Some(args) = between_once(&body, "Make<single_data_header>::From(", ")") {
                if let Some(cmd) = args.split(',').nth(1) {
                    let cmd = cmd.trim();
                    if is_enum_ident(cmd) {
                        out.push(Binding {
                            command: cmd.to_string(),
                            struct_name: format!("{SRV}::S2C::CMD::{struct_name}"),
                            line: k + 1,
                        });
                        break;
                    }
                }
            }
            if aberto && depth <= 0 {
                break;
            }
            k += 1;
        }
        i = k.max(i) + 1;
    }
    out
}

/// Lê `playercmd.cpp` e devolve as ligações id → struct do sentido C2S.
///
/// O arquivo tem mais de dez `switch(cmd_type)`, e **nem todos decodificam payload**:
/// vários são listas de permissão ou roteamento, com longas sequências de rótulos que
/// terminam em `return CommandHandler(...)` sem tocar em struct nenhuma. Por isso a
/// ligação só é registrada quando aparece uma struct `C2S::CMD::` no trecho que
/// pertence àqueles rótulos — a mera presença de um `case` não liga nada.
pub fn parse_c2s_bindings(lines: &[&str]) -> Vec<Binding> {
    let auxiliares = collect_helpers(lines);
    let mut out = Vec::new();
    let mut pendentes: Vec<(String, usize)> = Vec::new();

    for (i, raw) in lines.iter().enumerate() {
        let code = strip_line_comment(raw);
        let trimmed = code.trim();

        // `case  C2S::QUERY_PLAYER_INFO_1:` aparece com espaço duplo depois de `case`
        // nos fontes; colapsar os espaços evita perder o rótulo por um detalhe de
        // formatação.
        let normalizado = normalizar_espacos(trimmed);
        if let Some(cmd) = normalizado
            .strip_prefix("case C2S::")
            .and_then(|r| r.split(':').next())
        {
            let cmd = cmd.trim();
            if is_enum_ident(cmd) {
                pendentes.push((cmd.to_string(), i + 1));
                continue;
            }
        }

        if pendentes.is_empty() {
            continue;
        }

        // O trecho que pertence aos rótulos acumulados vai até o próximo `case` ou o
        // fim do bloco. Se uma struct aparecer aqui, ela é o payload de todos eles.
        if let Some(nome) = find_cmd_type(&code) {
            for (command, line) in pendentes.drain(..) {
                out.push(Binding {
                    command,
                    struct_name: format!("{SRV}::C2S::CMD::{nome}"),
                    line,
                });
            }
            continue;
        }

        // Vários comandos não decodificam nada no próprio `case`: eles delegam a um
        // método auxiliar (`cmd_user_move(buf,size)`) cujo corpo faz o cast. A ligação
        // continua sendo do C++, só que a um salto de distância.
        if let Some(nome) = chamada_auxiliar(&code, &auxiliares) {
            for (command, line) in pendentes.drain(..) {
                out.push(Binding {
                    command,
                    struct_name: format!("{SRV}::C2S::CMD::{nome}"),
                    line,
                });
            }
            continue;
        }

        // Um `return` ou uma chamada a outro manipulador encerra o trecho sem ligar
        // nada — é o caso dos switches de roteamento.
        if trimmed.starts_with("return") || trimmed.starts_with("break") {
            pendentes.clear();
        }
    }
    out
}

/// Mapeia método auxiliar → struct que ele decodifica.
///
/// Um auxiliar é uma função que recebe o buffer cru (`const void * buf`) e faz um único
/// cast para uma struct `C2S::CMD::`. O "único" é a condição: se o corpo mencionar mais
/// de uma struct, a ligação seria ambígua e o método fica de fora.
fn collect_helpers(lines: &[&str]) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();

    for (i, raw) in lines.iter().enumerate() {
        let code = strip_line_comment(raw);
        if !code.contains("buf") || !code.contains("void") || !code.contains("::") {
            continue;
        }
        let Some(nome) = nome_do_metodo(&code) else {
            continue;
        };

        // Percorre o corpo e coleta as structs mencionadas.
        let mut vistas: Vec<String> = Vec::new();
        let mut depth = 0i32;
        let mut aberto = false;
        for line in lines.iter().skip(i).take(400) {
            let body = strip_line_comment(line);
            if body.contains('{') {
                aberto = true;
            }
            depth += body.matches('{').count() as i32 - body.matches('}').count() as i32;
            if let Some(s) = find_cmd_type(&body) {
                if !vistas.contains(&s) {
                    vistas.push(s);
                }
            }
            if aberto && depth <= 0 {
                break;
            }
        }
        if vistas.len() == 1 {
            out.insert(nome, vistas.remove(0));
        }
    }
    out
}

/// Extrai `metodo` de uma linha como `gplayer_controller::cmd_user_move(const void * buf, size_t size)`.
fn nome_do_metodo(code: &str) -> Option<String> {
    let (antes, _) = code.split_once('(')?;
    let nome = antes.rsplit("::").next()?.trim();
    is_ident(nome).then(|| nome.to_string())
}

/// Devolve a struct do auxiliar chamado nesta linha com o buffer cru, se houver.
fn chamada_auxiliar(
    code: &str,
    auxiliares: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    for (nome, st) in auxiliares {
        let chamada = format!("{nome}(buf");
        if code.contains(&chamada) {
            return Some(st.clone());
        }
    }
    None
}

/// Extrai o nome de uma struct em `C2S::CMD::<nome>`, se a linha tiver uma.
///
/// Também reconhece a macro `DEFCMD(<nome>)`, que os comandos de GM usam e que expande
/// exatamente para o mesmo cast:
///
/// ```text
/// #define DEFCMD(type) C2S::CMD::type & cmd = *(C2S::CMD::type*)buf; ...
/// ```
fn find_cmd_type(code: &str) -> Option<String> {
    if let Some(rest) = code.split_once("DEFCMD(") {
        // A própria linha do `#define` não é um uso.
        if !code.trim_start().starts_with("#define") {
            let nome: String = rest
                .1
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !nome.is_empty() && nome != "type" {
                return Some(nome);
            }
        }
    }
    let rest = code.split_once("C2S::CMD::")?.1;
    let nome: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    (!nome.is_empty()).then_some(nome)
}

/// Colapsa sequências de espaços e tabulações em um único espaço.
fn normalizar_espacos(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_line_comment(line: &str) -> String {
    match line.find("//") {
        Some(i) => line[..i].to_string(),
        None => line.to_string(),
    }
}

fn between_once(text: &str, open: &str, close: &str) -> Option<String> {
    let rest = text.split_once(open)?.1;
    let end = rest.find(close)?;
    Some(rest[..end].to_string())
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_enum_ident(s: &str) -> bool {
    is_ident(s) && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(s: &str) -> Vec<&str> {
        s.lines().collect()
    }

    #[test]
    fn liga_s2c_pela_especializacao_que_emite_o_comando() {
        let src = split(
            "\
\t\tstruct Make<CMD::exchange_inventory_item>
\t\t{
\t\t\ttemplate <typename WRAPPER>
\t\t\tinline static WRAPPER & From(WRAPPER & wrapper,unsigned char idx1)
\t\t\t{
\t\t\t\tMake<single_data_header>::From(wrapper,EXCHANGE_INVENTORY_ITEM);
\t\t\t\treturn wrapper;
\t\t\t}
\t\t};",
        );
        let b = parse_s2c_bindings(&src);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].command, "EXCHANGE_INVENTORY_ITEM");
        assert_eq!(b[0].struct_name, "SRV::S2C::CMD::exchange_inventory_item");
    }

    #[test]
    fn liga_c2s_pelo_cast_dentro_do_case() {
        let src = split(
            "\
\tswitch(cmd_type)
\t{
\t\tcase C2S::EXCHANGE_INVENTORY_ITEM:
\t\t{
\t\t\tC2S::CMD::exchange_inventory_item & eii = *(C2S::CMD::exchange_inventory_item*) buf;
\t\t\tbreak;
\t\t}
\t}",
        );
        let b = parse_c2s_bindings(&src);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].command, "EXCHANGE_INVENTORY_ITEM");
        assert_eq!(b[0].struct_name, "SRV::C2S::CMD::exchange_inventory_item");
    }

    #[test]
    fn switch_de_roteamento_nao_liga_struct_nenhuma() {
        // Este é o modo de falha que importa: `playercmd.cpp` tem switches de permissão
        // com dezenas de rótulos em sequência que só redirecionam. Ligar qualquer
        // struct a eles inventaria layout para comandos que nem payload têm ali.
        let src = split(
            "\
\tswitch(cmd_type)
\t{
\t\tcase C2S::GET_ITEM_INFO:
\t\tcase C2S::GET_INVENTORY:
\t\tcase C2S::LOGOUT:
\t\t\treturn CommandHandler(cmd_type,buf,size);
\t}",
        );
        assert!(parse_c2s_bindings(&src).is_empty());
    }

    #[test]
    fn rotulos_em_sequencia_compartilham_a_struct_do_bloco() {
        let src = split(
            "\
\t\tcase C2S::A:
\t\tcase C2S::B:
\t\t{
\t\t\tC2S::CMD::comum & c = *(C2S::CMD::comum*) buf;
\t\t}",
        );
        let b = parse_c2s_bindings(&src);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].command, "A");
        assert_eq!(b[1].command, "B");
        assert!(b.iter().all(|x| x.struct_name == "SRV::C2S::CMD::comum"));
    }

    #[test]
    fn acha_o_primeiro_enum_anonimo_e_ignora_os_nomeados() {
        // `namespace C2S` abre com três enums NOMEADOS (MOVE_MODE, FORCE_ATTACK_MASK,
        // REFUSE_BLESS_MASK) antes do enum de comandos, que é anônimo.
        let src = split(
            "\
namespace C2S
{
\tenum MOVE_MODE
\t{
\t\tMOVE_MODE_WALK = 0x00,
\t};
\tenum
\t{
\t\tPLAYER_MOVE,
\t\tLOGOUT,
\t};
}",
        );
        let region = gd_cxx::find_namespace(&src, "C2S").unwrap();
        let e = find_command_enum(&src, region).unwrap();
        let entries = gd_cxx::parse_command_enum(&src, e.start, e.end);
        assert_eq!(entries.len(), 2);
        assert_eq!((entries[0].name.as_str(), entries[0].value), ("PLAYER_MOVE", 0));
        assert_eq!((entries[1].name.as_str(), entries[1].value), ("LOGOUT", 1));
    }

    #[test]
    fn cmd_do_servidor_enxerga_os_tipos_de_info() {
        // `namespace CMD` abre com `using namespace INFO;`. Sem registrar isso, o campo
        // `move_info info;` de `player_move` vira referência pendurada.
        let src = split(
            "\
namespace S2C
{
\tenum
\t{
\t\tPLAYER_INFO_1,
\t};
}

namespace C2S
{
\tnamespace INFO
\t{
\t\tstruct move_info
\t\t{
\t\t\tint a;
\t\t};
\t}
\tenum
\t{
\t\tPLAYER_MOVE,
\t};
\tnamespace CMD
\t{
\t\tusing namespace INFO;
\t\tstruct player_move
\t\t{
\t\t\tmove_info info;
\t\t};
\t}
}",
        );
        let mut out = ServerSide::default();
        parse_protocol_h(&src, &Consts::new(), &mut out).unwrap();
        let cmd = out
            .structs
            .iter()
            .find(|s| s.name == "player_move")
            .expect("player_move não foi lida");
        assert_eq!(cmd.scope, "SRV::C2S::CMD");
        assert_eq!(cmd.usings, vec!["SRV::C2S::INFO".to_string()]);
        assert!(out.structs.iter().any(|s| s.qualified() == "SRV::C2S::INFO::move_info"));
    }
}
