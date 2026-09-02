//! Leitura dos fontes do cliente 1.5.3: enums de comando, structs empacotadas e a
//! tabela que liga um ao outro.
//!
//! Três coisas são extraídas, de dois arquivos:
//!
//! * `EC_GPDataType.h` — os dois enums de Command ID (`namespace S2C` e `namespace
//!   C2S`) e as 555 declarações de struct entre o `#pragma pack(1)` e o `#pragma
//!   pack()`;
//! * `EC_GameDataPrtc.cpp` — `CECGameSession::CalcS2CCmdDataSize`, que é a ligação
//!   autoritativa entre cada comando e a struct que carrega seu payload.
//!
//! O parser não tenta ser um compilador C++. Ele reconhece as formas que estes dois
//! arquivos de fato usam e **registra um diagnóstico** para tudo que não encaixa,
//! porque uma linha ignorada em silêncio aqui vira um campo faltando no fio.

use crate::gd_ty::{self, GdTy};

/// Namespace de origem. Os dois enums e as structs são declarados em espaços de nome
/// separados, e 30 nomes de struct aparecem **nos dois** (`cmd_header`,
/// `cmd_equip_item`, `cmd_select_target`, …) com campos diferentes. Sem qualificar o
/// nome, uma declaração sobrescreveria a outra e metade dos comandos sairia errada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ns {
    S2C,
    C2S,
}

impl Ns {
    pub fn as_str(self) -> &'static str {
        match self {
            Ns::S2C => "S2C",
            Ns::C2S => "C2S",
        }
    }

    /// Nome qualificado, como aparece no IR.
    pub fn qualify(self, name: &str) -> String {
        format!("{}::{}", self.as_str(), name)
    }
}

/// Uma entrada do enum de Command ID.
#[derive(Debug, Clone)]
pub struct CmdEnumEntry {
    pub name: String,
    pub value: i64,
    /// Número escrito no comentário à direita (`// 5`, `// 375`), quando havia um.
    /// É a âncora de verificação: os fontes originais numeram uma entrada a cada
    /// cinco, e se o valor calculado não bater com o comentário, o parser errou.
    pub comment_value: Option<i64>,
    pub line: usize,
}

/// Um campo de struct empacotada.
#[derive(Debug, Clone)]
pub struct PackedField {
    pub name: String,
    pub ty: GdTy,
    /// Comprimento de um array de tamanho fixo (`int reserved[10]`).
    pub array_len: Option<usize>,
    /// Texto C++ original do tipo, preservado para revisão.
    pub cxx: String,
    pub line: usize,
}

/// Como uma struct de tamanho variável carrega sua lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarList {
    /// `BYTE placeholder;` seguido de elementos que têm seu próprio `CheckValid` —
    /// elementos de tamanho variável, lidos um a um.
    Placeholder { element: String },
    /// `info_matter list[1];` com `sz += count * sizeof(info_matter)` — elementos de
    /// tamanho fixo, no idioma de membro-array flexível.
    FlexArray { element: String, field: String },
    /// Tem `bool Initialize(...)`: serialização manual campo a campo com `Extract()`,
    /// e não `memcpy`. Os `abase::vector<T>` que a struct contém viajam como contagem
    /// seguida dos elementos.
    Initialize,
    /// Campos finais condicionais a bits de um campo `state`
    /// (`if (state & GP_STATE_ADV_MODE) sz += ...`). O tamanho depende do conteúdo.
    Conditional,
    /// Tem `CheckValid` mas o parser não reconheceu o elemento.
    Unknown,
}

/// Uma declaração de struct empacotada.
#[derive(Debug, Clone)]
pub struct PackedStruct {
    /// Escopo em que a struct foi declarada (`S2C`, `C2S`, `S2C::CMD`, ou vazio para
    /// escopo global). Vira o prefixo do nome qualificado.
    pub scope: String,
    /// Escopos absolutos que a declaração torna visíveis por `using namespace X;`.
    /// O `protocol.h` do servidor abre os blocos `CMD` com `using namespace INFO;`, e
    /// sem isso metade dos campos vira referência pendurada.
    pub usings: Vec<String>,
    pub name: String,
    pub fields: Vec<PackedField>,
    /// `Some` se a struct declara `bool CheckValid(...)`, isto é, se tem tamanho
    /// variável. `CalcS2CCmdDataSize` usa exatamente esse critério para escolher entre
    /// `sizeof(T)` e a macro `CHECK_VALID(T)`, o que dá uma verificação cruzada entre
    /// os dois arquivos.
    pub var_list: Option<VarList>,
    pub line: usize,
}

impl PackedStruct {
    pub fn qualified(&self) -> String {
        qualify(&self.scope, &self.name)
    }
}

/// Junta escopo e nome. Escopo vazio = escopo global, e o nome vai sozinho.
pub fn qualify(scope: &str, name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else {
        format!("{scope}::{name}")
    }
}

/// O que `CalcS2CCmdDataSize` diz sobre o payload de um comando.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadKind {
    /// `dwSize = sizeof (cmd_x);` — tamanho fixo.
    Fixed(String),
    /// `CHECK_VALID(cmd_x)` — tamanho variável, validado pelo `CheckValid` da struct.
    Variable(String),
    /// `case X: break;` sem atribuir `dwSize` — o cliente original deixa como
    /// desconhecido. Não é erro do parser; é uma lacuna do próprio C++.
    Unhandled,
    /// `dwSize = 0;` — comando sem payload.
    Empty,
}

/// Uma entrada da tabela comando → struct.
#[derive(Debug, Clone)]
pub struct CmdPayload {
    pub command: String,
    pub kind: PayloadKind,
    pub line: usize,
}

/// Algo que o parser não conseguiu interpretar. Vai para os diagnósticos do IR.
#[derive(Debug, Clone)]
pub struct ParseWarning {
    pub kind: &'static str,
    pub subject: String,
    pub detail: String,
}

// ---------------------------------------------------------------------------
// Enums de Command ID
// ---------------------------------------------------------------------------

/// Lê o enum de Command ID que segue o marcador `// Commands` dentro de um namespace.
///
/// As duas formas presentes nos fontes:
///
/// ```text
/// PROTOCOL_COMMAND = -1,   //  Reserved for protocol   (valor explícito)
/// PLAYER_INFO_2,           //                          (sequencial)
/// MATTER_INFO_LIST,        //  10                      (com âncora de verificação)
/// OWN_ITEM_INFO,           //  40, Own item information (âncora + texto)
/// SCENE_SERVICE_NPC_LIST,  //  390<texto em chinês>     (âncora colada no texto)
/// ```
pub fn parse_command_enum(lines: &[&str], start: usize, end: usize) -> Vec<CmdEnumEntry> {
    let mut entries = Vec::new();
    let mut next_value: i64 = 0;

    for (offset, raw) in lines[start..end].iter().enumerate() {
        let line_no = start + offset + 1;
        let (code, comment) = split_comment(raw);
        let code = code.trim();
        if code.is_empty() || code == "{" || code == "};" || code == "}" {
            continue;
        }

        for item in code.split(',') {
            let item = item.trim().trim_end_matches('}').trim();
            if item.is_empty() || item == "{" {
                continue;
            }
            let (name, value) = match item.split_once('=') {
                Some((n, v)) => {
                    let parsed = parse_int_literal(v.trim());
                    match parsed {
                        Some(value) => (n.trim(), value),
                        // Um valor que não sabemos ler quebraria toda a numeração
                        // seguinte. Parar é melhor do que emitir ids inventados.
                        None => return entries,
                    }
                }
                None => (item, next_value),
            };
            if !is_enum_name(name) {
                continue;
            }
            entries.push(CmdEnumEntry {
                name: name.to_string(),
                value,
                comment_value: comment.as_deref().and_then(leading_number),
                line: line_no,
            });
            next_value = value + 1;
        }
    }
    entries
}

/// Extrai o primeiro número de um comentário de âncora.
///
/// Aceita `// 10`, `// 40, Own item information` e `// 390<texto>`, e rejeita
/// comentários que começam com texto (`// Reserved for protocol`) ou que trazem um
/// número no meio de uma frase, que não é âncora de posição.
fn leading_number(comment: &str) -> Option<i64> {
    let t = comment.trim_start();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn is_enum_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn parse_int_literal(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    s.parse().ok()
}

/// Apaga o conteúdo de comentários `/* ... */`, preservando a contagem de linhas.
///
/// Os fontes usam blocos `/* */` para guardar **código de exemplo**, e esse código
/// declara structs:
///
/// ```text
/// struct cmd_player_info_2_list
/// {
///     unsigned short count;
///     /* struct { int cid; player_info_2 info; } list[]; */
///     char data[1];
/// };
/// ```
///
/// Sem remover o bloco, o parser lia `list` como um campo real e a struct ganhava
/// membros que não existem — e, pior, os deslocamentos de tudo que vinha depois saíam
/// errados sem nada acusar. Os comentários `//` ficam intactos de propósito: é neles
/// que moram as âncoras numéricas dos enums.
pub fn strip_block_comments(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut in_block = false;

    for line in lines {
        let chars: Vec<char> = line.chars().collect();
        let mut result = String::with_capacity(line.len());
        let mut i = 0;

        while i < chars.len() {
            if in_block {
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    in_block = false;
                    i += 2;
                } else {
                    i += 1;
                }
            } else if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
                // Comentário de linha: o resto fica como está.
                result.extend(&chars[i..]);
                break;
            } else if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                in_block = true;
                i += 2;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        out.push(result);
    }
    out
}


/// Separa o código do comentário `//` de uma linha.
fn split_comment(line: &str) -> (String, Option<String>) {
    match line.find("//") {
        Some(i) => (line[..i].to_string(), Some(line[i + 2..].to_string())),
        None => (line.to_string(), None),
    }
}
// ---------------------------------------------------------------------------
// Structs empacotadas
// ---------------------------------------------------------------------------

/// Constantes inteiras do cabeçalho, para resolver comprimentos simbólicos de array.
///
/// `int reserved[GP_PET_SKILL_NUM]` só tem tamanho conhecido se soubermos que
/// `GP_PET_SKILL_NUM` é 8. As duas constantes que os fontes usam assim
/// (`GP_PET_SKILL_NUM`, `OBJECT_EXT_STATE_COUNT`) são declaradas em `enum`s de escopo
/// global antes do `#pragma pack(1)`.
pub type Consts = std::collections::BTreeMap<String, i64>;

/// Varre o arquivo inteiro atrás de `#define N valor` e de entradas de `enum` com valor
/// explícito.
pub fn parse_consts(lines: &[&str]) -> Consts {
    let mut out = Consts::new();
    for line in lines {
        let (code, _) = split_comment(line);
        let s = code.trim();

        if let Some(rest) = s.strip_prefix("#define ") {
            let mut it = rest.split_whitespace();
            if let (Some(name), Some(value)) = (it.next(), it.next()) {
                if it.next().is_none() {
                    if let Some(v) = parse_int_literal(value) {
                        out.insert(name.to_string(), v);
                    }
                }
            }
            continue;
        }

        // `NOME = 6,` dentro de um enum. Só formas com valor explícito entram: uma
        // entrada sequencial dependeria de rastrear o enum inteiro, e nenhum
        // comprimento de array nos fontes usa uma dessas.
        for item in s.split(',') {
            let item = item.trim().trim_end_matches('}').trim();
            if let Some((name, value)) = item.split_once('=') {
                let name = name.trim();
                if is_enum_name(name) {
                    if let Some(v) = parse_int_literal(value.trim().trim_end_matches(';')) {
                        out.entry(name.to_string()).or_insert(v);
                    }
                }
            }
        }
    }
    out
}

/// Como [`parse_packed_structs_with`], mas sem tabela de constantes. Só para testes:
/// a extração de verdade sempre tem as constantes do cabeçalho em mãos.
#[cfg(test)]
pub fn parse_packed_structs(
    lines: &[&str],
    start: usize,
    end: usize,
    scope: &str,
    warnings: &mut Vec<ParseWarning>,
) -> Vec<PackedStruct> {
    parse_packed_structs_with(lines, start, end, scope, &[], &Consts::new(), warnings)
}

/// Lê todas as declarações de struct de uma faixa de linhas.
///
/// Structs aninhadas entram no resultado com o caminho no nome (`info_pet::evo_prop`),
/// e o campo que as contém referencia esse caminho. Achatá-las ou descartá-las faria o
/// tamanho da struct externa sair errado **em silêncio**, que é a falha que este
/// projeto não pode ter.
///
/// `consts` resolve comprimentos simbólicos de array (`states[OBJECT_EXT_STATE_COUNT]`).
pub fn parse_packed_structs_with(
    lines: &[&str],
    start: usize,
    end: usize,
    scope: &str,
    usings: &[String],
    consts: &Consts,
    warnings: &mut Vec<ParseWarning>,
) -> Vec<PackedStruct> {
    let mut out = Vec::new();
    let ns_aliases = collect_ns_aliases(lines, start, end);
    let mut i = start;

    while i < end {
        let (code, _) = split_comment(lines[i]);
        let trimmed = code.trim();

        let Some(name) = struct_decl_name(trimmed) else {
            i += 1;
            continue;
        };
        let Some((body_start, _)) = find_body_start(lines, i, end) else {
            i += 1;
            continue;
        };

        let ctx = Ctx {
            scope,
            usings,
            consts,
            ns_aliases: ns_aliases.clone(),
        };
        let parsed = parse_struct_body(lines, body_start, end, &name, &ctx, &mut out, warnings);

        out.push(PackedStruct {
            scope: scope.to_string(),
            usings: usings.to_vec(),
            name,
            fields: parsed.fields,
            var_list: parsed.var_list,
            line: i + 1,
        });
        i = parsed.next;
    }

    out
}

struct Ctx<'a> {
    scope: &'a str,
    usings: &'a [String],
    consts: &'a Consts,
    /// `typedef abase::vector<IconState> IconStates;` declarado no escopo do
    /// namespace, fora de qualquer struct. Sem isso, o campo que usa o apelido vira
    /// uma referência pendurada a uma struct que nunca existiu.
    ns_aliases: Aliases,
}

pub type Aliases = std::collections::BTreeMap<String, String>;

/// Colhe os `typedef` declarados diretamente no escopo do namespace.
fn collect_ns_aliases(lines: &[&str], start: usize, end: usize) -> Aliases {
    let mut out = Aliases::new();
    let mut depth = 0i32;
    for line in &lines[start..end] {
        let (code, _) = split_comment(line);
        let s = code.trim();
        // Profundidade 1 = dentro do namespace, fora de qualquer struct.
        if depth == 1 {
            if let Some(rest) = s.strip_prefix("typedef ") {
                if let Some((target, alias)) = split_typedef(rest) {
                    out.insert(alias, target);
                }
            }
        }
        depth += count_braces(&code);
    }
    out
}

struct ParsedBody {
    fields: Vec<PackedField>,
    var_list: Option<VarList>,
    /// Índice da linha logo após o `}` que fecha o corpo.
    next: usize,
}

/// Lê o corpo de uma struct, a partir da linha seguinte à que abre a chave.
///
/// Structs aninhadas encontradas no caminho são empurradas para `out` com o caminho
/// completo no nome, e o campo correspondente passa a referenciá-las.
fn parse_struct_body(
    lines: &[&str],
    body_start: usize,
    end: usize,
    path: &str,
    ctx: &Ctx,
    out: &mut Vec<PackedStruct>,
    warnings: &mut Vec<ParseWarning>,
) -> ParsedBody {
    let mut fields: Vec<PackedField> = Vec::new();
    let mut aliases: Aliases = ctx.ns_aliases.clone();
    let mut check_valid: Option<Vec<String>> = None;
    let mut has_initialize = false;
    let mut depth = 1i32;
    let mut j = body_start;

    while j < end && depth > 0 {
        let raw = lines[j];
        let (code, _) = split_comment(raw);
        let s = code.trim();

        if s.is_empty() || s.starts_with('#') {
            j += 1;
            continue;
        }

        // `typedef abase::vector<score_rank_entry> ScoreRankContainer;` — um apelido
        // local. Sem registrá-lo, o campo que o usa vira uma referência pendurada a
        // uma struct que não existe.
        if let Some(rest) = s.strip_prefix("typedef ") {
            if let Some((target, alias)) = split_typedef(rest) {
                aliases.insert(alias, target);
            }
            j += 1;
            continue;
        }

        // `bool Initialize(...)` é uma **terceira** forma de serialização, ao lado do
        // `memcpy` de tamanho fixo e do `CheckValid`: o corpo extrai campo a campo com
        // `Extract()`, e os `abase::vector` que a struct contém viajam como contagem
        // seguida dos elementos. Structs assim não são copiáveis por `memcpy`.
        if s.contains("Initialize") && s.contains('(') && !s.ends_with(';') {
            has_initialize = true;
            let (_, next) = skip_block(lines, j, end);
            j = next;
            continue;
        }

        if s.contains("CheckValid") && s.contains('(') && !s.ends_with(';') {
            let (body, next) = skip_block(lines, j, end);
            if check_valid.is_none() {
                check_valid = Some(body);
            }
            j = next;
            continue;
        }

        // Struct aninhada, nomeada (`struct _evo_prop { ... } evo_prop;`) ou anônima
        // (`struct { ... } skills[GP_PET_SKILL_NUM];`).
        if abre_struct(s) {
            if let Some((body_start2, _)) = find_body_start(lines, j, end) {
                let declared_name = struct_decl_name(s);
                let (nested, after) =
                    read_nested(lines, j, body_start2, end, path, declared_name, ctx, out);
                match nested {
                    Nested::Member(field) => fields.push(field),
                    // `struct building_data { ... };` sem declarador é uma **declaração
                    // de tipo** de escopo local, não um membro: não ocupa bytes, mas os
                    // campos irmãos a referenciam pelo nome curto.
                    Nested::TypeDecl { short, qualified } => {
                        aliases.insert(short, qualified);
                    }
                    Nested::Anonymous => warnings.push(ParseWarning {
                        kind: "struct-aninhada-sem-membro",
                        subject: path.to_string(),
                        detail: format!("linha {}: bloco anônimo sem declarador", j + 1),
                    }),
                }
                j = after;
                continue;
            }
        }

        // Construtores e outros métodos: `score_rank_entry():roleid(0){}`.
        if s.contains('(') {
            if s.ends_with(';') && !s.contains(')') {
                // Declaração de campo com parêntese solto — improvável, mas não some.
            } else {
                let (_, next) = skip_block(lines, j, end);
                j = next;
                continue;
            }
        }

        depth += count_braces(&code);
        if depth <= 0 {
            break;
        }

        if let Some(field) = parse_field(&code, j + 1, &aliases, ctx.consts) {
            fields.push(field);
        }
        j += 1;
    }

    let var_list = if let Some(body) = check_valid {
        Some(classify_var_list(&body, &fields))
    } else if has_initialize {
        Some(VarList::Initialize)
    } else {
        None
    };

    ParsedBody {
        fields,
        var_list,
        next: j + 1,
    }
}

/// O que um bloco `struct` aninhado acabou sendo.
enum Nested {
    /// `struct { ... } skills[8];` — um membro, que ocupa bytes na struct externa.
    Member(PackedField),
    /// `struct building_data { ... };` — só uma declaração de tipo em escopo local.
    TypeDecl { short: String, qualified: String },
    /// Bloco anônimo sem declarador: não deveria existir, e vira diagnóstico.
    Anonymous,
}

/// Lê uma struct aninhada e o declarador que vem depois do `}` que a fecha.
#[allow(clippy::too_many_arguments)]
fn read_nested(
    lines: &[&str],
    decl_line: usize,
    body_start: usize,
    end: usize,
    path: &str,
    declared_name: Option<String>,
    ctx: &Ctx,
    out: &mut Vec<PackedStruct>,
) -> (Nested, usize) {
    let mut warnings = Vec::new();
    let parsed = parse_struct_body(lines, body_start, end, path, ctx, out, &mut warnings);
    let close = parsed.next.saturating_sub(1);

    // O declarador está depois do `}` na linha de fechamento: `} evo_prop;` ou
    // `} skills [GP_PET_SKILL_NUM];`.
    let (close_code, _) = split_comment(lines[close.min(end.saturating_sub(1))]);
    let after = match close_code.rfind('}') {
        Some(i) => close_code[i + 1..].trim().trim_end_matches(';').trim().to_string(),
        None => String::new(),
    };

    if after.is_empty() {
        // Sem declarador: se o bloco tinha nome, é uma declaração de tipo local.
        return match declared_name {
            Some(short) => {
                let qualified = format!("{path}::{short}");
                out.push(PackedStruct {
                    scope: ctx.scope.to_string(),
            usings: ctx.usings.to_vec(),
                    name: qualified.clone(),
                    fields: parsed.fields,
                    var_list: parsed.var_list,
                    line: decl_line + 1,
                });
                (Nested::TypeDecl { short, qualified }, parsed.next)
            }
            None => (Nested::Anonymous, parsed.next),
        };
    }

    // `} *data;` declara um **ponteiro** para a struct aninhada, não um membro
    // embutido. Um ponteiro é um endereço do processo do cliente: ocupa 4 bytes no alvo
    // i386, mas não carrega dado nenhum para o fio. Tratá-lo como membro embutido
    // somaria o tamanho da struct inteira e deslocaria tudo que viesse depois.
    if let Some(target) = after.strip_prefix('*') {
        let member = target.trim().to_string();
        let nested_name = format!("{path}::{member}");
        out.push(PackedStruct {
            scope: ctx.scope.to_string(),
            usings: ctx.usings.to_vec(),
            name: nested_name.clone(),
            fields: parsed.fields,
            var_list: parsed.var_list,
            line: decl_line + 1,
        });
        return (
            Nested::Member(PackedField {
                name: member,
                ty: GdTy::Unresolved(format!("{nested_name}* (ponteiro, não serializável)")),
                array_len: None,
                cxx: format!("struct {nested_name}*"),
                line: decl_line + 1,
            }),
            parsed.next,
        );
    }

    // Separa nome do membro e comprimento do array.
    let (member, array_len) = match after.find('[') {
        Some(i) => {
            let close_br = after.rfind(']').unwrap_or(after.len());
            let inner = after[i + 1..close_br].trim();
            let len = inner
                .parse::<usize>()
                .ok()
                .or_else(|| ctx.consts.get(inner).and_then(|v| usize::try_from(*v).ok()));
            (after[..i].trim().to_string(), Some(len))
        }
        None => (after.clone(), None),
    };

    let nested_name = format!("{path}::{member}");
    out.push(PackedStruct {
        scope: ctx.scope.to_string(),
            usings: ctx.usings.to_vec(),
        name: nested_name.clone(),
        fields: parsed.fields,
        var_list: parsed.var_list,
        line: decl_line + 1,
    });

    let (ty, array_len) = match array_len {
        Some(Some(n)) => (GdTy::Struct(nested_name.clone()), Some(n)),
        Some(None) => (
            GdTy::Unresolved(format!("struct {member}[?] (comprimento simbólico)")),
            None,
        ),
        None => (GdTy::Struct(nested_name.clone()), None),
    };

    (
        Nested::Member(PackedField {
            name: member,
            ty,
            array_len,
            cxx: format!("struct {nested_name}"),
            line: decl_line + 1,
        }),
        parsed.next,
    )
}

/// Separa `typedef <tipo> <apelido>;` em (tipo, apelido).
fn split_typedef(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim().trim_end_matches(';').trim();
    let idx = rest.rfind(char::is_whitespace)?;
    let alias = rest[idx + 1..].trim();
    let target = rest[..idx].trim();
    if alias.is_empty() || target.is_empty() {
        return None;
    }
    Some((target.to_string(), alias.to_string()))
}

/// Diz se a linha começa uma declaração de struct.
///
/// Aceita `struct nome`, `struct` sozinho e **`struct{`** — esta última forma, sem
/// espaço antes da chave, aparece em `force_global_data` e `public_quest_ranks` do
/// servidor. Não reconhecê-la fazia os campos da struct aninhada serem lidos como
/// campos da externa, deslocando tudo em silêncio; quem apontou foi o compilador.
fn abre_struct(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("struct") else {
        return false;
    };
    rest.is_empty() || rest.starts_with([' ', '\t', '{'])
}

/// Devolve o nome se a linha declara uma struct (e não um typedef ou uma variável).
///
/// Uma struct anônima (`struct` sozinho, ou `struct {`) recebe o nome vazio; quem
/// chama descobre o nome pelo membro declarado depois do `}`.
fn struct_decl_name(trimmed: &str) -> Option<String> {
    if trimmed.starts_with("typedef") {
        return None;
    }
    let rest = if let Some(r) = trimmed.strip_prefix("struct ") {
        r
    } else if trimmed == "struct" {
        ""
    } else {
        return None;
    };
    let name: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    // `struct x;` é declaração antecipada; `struct x var;` é uma variável.
    let after = rest.trim_start()[name.len()..].trim();
    if after.starts_with(';') {
        return None;
    }
    Some(name)
}

/// Encontra a linha onde o corpo da struct começa e a profundidade após abri-lo.
fn find_body_start(lines: &[&str], decl: usize, end: usize) -> Option<(usize, i32)> {
    for k in decl..(decl + 3).min(end) {
        let (code, _) = split_comment(lines[k]);
        let delta = count_braces(&code);
        if delta > 0 {
            return Some((k + 1, delta));
        }
        if code.contains(';') {
            return None;
        }
    }
    None
}

/// Consome um bloco `{...}` a partir de uma linha, devolvendo suas linhas e o índice
/// seguinte.
fn skip_block(lines: &[&str], from: usize, end: usize) -> (Vec<String>, usize) {
    let mut body = Vec::new();
    let mut depth = 0i32;
    let mut seen = false;
    let mut k = from;
    while k < end {
        let (code, _) = split_comment(lines[k]);
        body.push(lines[k].to_string());

        // `seen` marca que o bloco de fato **abriu**. Testar `delta != 0` em vez disso
        // era um erro sutil: um corpo inline como
        // `score_rank_entry():roleid(0), rank(0){}` tem delta zero (abre e fecha na
        // mesma linha), o bloco nunca era dado por aberto, e a varredura seguia até
        // engolir o `};` da própria struct — e daí em diante todas as structs
        // seguintes eram lidas como aninhadas.
        if code.contains('{') {
            seen = true;
        }
        depth += count_braces(&code);
        if seen && depth <= 0 {
            return (body, k + 1);
        }
        // Um método apenas declarado, sem corpo.
        if !seen && code.contains(';') {
            return (body, k + 1);
        }
        k += 1;
    }
    (body, end)
}

fn count_braces(code: &str) -> i32 {
    code.chars().fold(0i32, |acc, c| match c {
        '{' => acc + 1,
        '}' => acc - 1,
        _ => acc,
    })
}

/// Interpreta uma linha de declaração de campo.
///
/// Reconhece `int id;`, `unsigned short count;`, `A3DVECTOR3 vPos;`,
/// `int reserved[10];`, `DWORD states[OBJECT_EXT_STATE_COUNT];` e `BYTE placeholder;`.
/// Devolve `None` para tudo que não seja uma declaração de campo.
fn parse_field(
    code: &str,
    line: usize,
    aliases: &std::collections::BTreeMap<String, String>,
    consts: &Consts,
) -> Option<PackedField> {
    let s = code.trim();
    if s.is_empty() || s.starts_with('#') || !s.ends_with(';') {
        return None;
    }
    let s = s.trim_end_matches(';').trim();
    if s.starts_with("typedef")
        || s.starts_with("return")
        || s.starts_with("static")
        || s.starts_with("enum")
        || s.starts_with("struct ")
        || s.contains('(')
        || s.contains('=')
    {
        return None;
    }

    let (decl, array_len) = match s.find('[') {
        Some(i) => {
            let close = s.rfind(']')?;
            let inner = s[i + 1..close].trim();
            let len = inner
                .parse::<usize>()
                .ok()
                .or_else(|| consts.get(inner).and_then(|v| usize::try_from(*v).ok()));
            (s[..i].trim().to_string(), Some(len))
        }
        None => (s.to_string(), None),
    };

    let idx = decl.rfind(|c: char| c.is_whitespace() || c == '*')?;
    let name = decl[idx + 1..].trim();
    let cxx_ty = decl[..=idx].trim();
    // O nome do campo pode começar com `_` (`int _task_id;` em `public_quest_ranks`).
    // Exigir letra aqui descartava o campo em silêncio e deslocava todos os seguintes —
    // outro erro que só o compilador de 32 bits apontou.
    let inicial = name.chars().next()?;
    if name.is_empty() || cxx_ty.is_empty() || !(inicial.is_ascii_alphabetic() || inicial == '_') {
        return None;
    }

    // Resolve apelidos locais antes de traduzir: `ScoreRankContainer` é
    // `abase::vector<score_rank_entry>`, que não sobrevive a um `memcpy`.
    let resolved = aliases
        .get(cxx_ty.trim_start_matches("const ").trim())
        .map(String::as_str)
        .unwrap_or(cxx_ty);

    let mut ty = gd_ty::translate(resolved);

    // O `placeholder` não é um campo de dados: é o endereço onde a lista começa.
    if name == "placeholder" {
        ty = GdTy::Placeholder;
    }

    let array_len = match array_len {
        Some(Some(n)) => Some(n),
        Some(None) => {
            ty = GdTy::Unresolved(format!("{cxx_ty} {name}[?] (comprimento simbólico)"));
            None
        }
        None => None,
    };

    Some(PackedField {
        name: name.to_string(),
        ty,
        array_len,
        cxx: cxx_ty.to_string(),
        line,
    })
}

/// Descobre o tipo dos elementos da lista a partir do corpo de `CheckValid`.
fn classify_var_list(body: &[String], fields: &[PackedField]) -> VarList {
    let text = body.join("\n");

    if let Some(elem) = between(&text, "reinterpret_cast<", ">")
        .into_iter()
        .filter_map(|s| {
            let s = s.trim().trim_start_matches("const").trim();
            let s = s.trim_end_matches('*').trim();
            (s != "BYTE" && !s.is_empty()).then(|| s.to_string())
        })
        .next()
    {
        return VarList::Placeholder { element: elem };
    }

    if let Some(elem) = between(&text, "sizeof(", ")")
        .into_iter()
        .map(|s| s.trim().to_string())
        .find(|s| !s.starts_with('*') && s != "count" && !fields.iter().any(|f| &f.name == s))
    {
        if let Some(field) = fields.iter().find(|f| f.ty == GdTy::Struct(elem.clone())) {
            return VarList::FlexArray {
                element: elem,
                field: field.name.clone(),
            };
        }
    }

    // Campos condicionais por bits de estado: `if (state & GP_STATE_ADV_MODE) sz +=
    // sizeof(DWORD)*2`. O tamanho depende do conteúdo, não só da contagem.
    if text.contains("state &") || text.contains("state&") {
        return VarList::Conditional;
    }

    VarList::Unknown
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        rest = &rest[i + open.len()..];
        if let Some(j) = rest.find(close) {
            out.push(&rest[..j]);
            rest = &rest[j + close.len()..];
        } else {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tabela comando → struct
// ---------------------------------------------------------------------------

/// Lê `CECGameSession::CalcS2CCmdDataSize` e devolve a tabela comando → payload.
///
/// Vários `case` podem compartilhar a mesma struct por *fall-through*: os rótulos se
/// acumulam até que uma atribuição de `dwSize` ou um `CHECK_VALID` apareça, e então
/// todos recebem aquele payload.
pub fn parse_payload_table(lines: &[&str], start: usize, end: usize) -> Vec<CmdPayload> {
    let mut out = Vec::new();
    let mut pending: Vec<(String, usize)> = Vec::new();

    for (offset, raw) in lines[start..end].iter().enumerate() {
        let line_no = start + offset + 1;
        let (code, _) = split_comment(raw);
        let s = code.trim();
        if s.is_empty() {
            continue;
        }

        // Um `case` pode vir sozinho na linha ou já trazer o corpo:
        //   case OBJECT_LEAVE_SLICE:  dwSize = sizeof (cmd_leave_slice);  break;
        let mut rest = s;
        while let Some(after) = rest.strip_prefix("case ") {
            let Some(colon) = after.find(':') else { break };
            let name = after[..colon].trim();
            if is_enum_name(name) {
                pending.push((name.to_string(), line_no));
            }
            rest = after[colon + 1..].trim_start();
        }
        if rest.starts_with("default") {
            pending.clear();
            continue;
        }
        if pending.is_empty() {
            continue;
        }

        let kind = if let Some(t) = between(rest, "CHECK_VALID(", ")").first() {
            Some(PayloadKind::Variable(t.trim().to_string()))
        } else if rest.contains("dwSize") && rest.contains('=') {
            let value = rest.split('=').nth(1).unwrap_or("").trim();
            if let Some(t) = between(value, "sizeof", ")").first() {
                Some(PayloadKind::Fixed(
                    t.trim().trim_start_matches('(').trim().to_string(),
                ))
            } else if value.starts_with('0') {
                Some(PayloadKind::Empty)
            } else {
                // `(DWORD)(-1)` / `(DWORD)(-2)`: o próprio cliente declara que não
                // sabe calcular. Fica registrado como tal.
                Some(PayloadKind::Unhandled)
            }
        } else if rest.starts_with("break") {
            Some(PayloadKind::Unhandled)
        } else {
            None
        };

        if let Some(kind) = kind {
            for (command, line) in pending.drain(..) {
                out.push(CmdPayload {
                    command,
                    kind: kind.clone(),
                    line,
                });
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Localização das regiões nos arquivos
// ---------------------------------------------------------------------------

/// Extremos de uma região de linhas (índices 0-based, `end` exclusivo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub start: usize,
    pub end: usize,
}

/// Encontra `namespace S2C { ... }` ou `namespace C2S { ... }` por contagem de chaves.
pub fn find_namespace(lines: &[&str], name: &str) -> Option<Region> {
    let needle = format!("namespace {name}");
    let start = lines.iter().position(|l| l.trim().starts_with(&needle))?;

    let mut depth = 0i32;
    let mut seen = false;
    for (i, line) in lines.iter().enumerate().skip(start) {
        let (code, _) = split_comment(line);
        let delta = count_braces(&code);
        if delta != 0 {
            seen = true;
        }
        depth += delta;
        if seen && depth == 0 {
            return Some(Region { start, end: i });
        }
    }
    None
}

/// Encontra o `enum` de Command ID dentro de uma região de namespace.
///
/// Ancorado no comentário `// Commands` que precede o enum nos dois namespaces, e não
/// na primeira ocorrência de `enum` — o `namespace S2C` tem outros enums antes.
pub fn find_command_enum(lines: &[&str], region: Region) -> Option<Region> {
    let marker = (region.start..region.end)
        .find(|&i| lines[i].contains("Commands ---"))?;

    let enum_line = (marker..region.end).find(|&i| lines[i].trim().starts_with("enum"))?;
    let open = (enum_line..region.end).find(|&i| lines[i].contains('{'))?;
    let close = (open..region.end).find(|&i| lines[i].contains("};"))?;
    Some(Region {
        start: open + 1,
        end: close,
    })
}

/// Encontra o corpo de `CECGameSession::CalcS2CCmdDataSize`, pulando a macro
/// `CHECK_VALID` que é definida logo no início da função.
pub fn find_calc_fn(lines: &[&str]) -> Option<Region> {
    let start = lines
        .iter()
        .position(|l| l.contains("CalcS2CCmdDataSize") && l.contains('{') == false && l.contains("DWORD"))?;

    let mut depth = 0i32;
    let mut seen = false;
    let mut in_macro = false;
    for (i, line) in lines.iter().enumerate().skip(start) {
        let (code, _) = split_comment(line);
        // O corpo da macro tem chaves que não pertencem à função.
        if code.trim_start().starts_with("#define") {
            in_macro = true;
        }
        if in_macro {
            if !code.trim_end().ends_with('\\') {
                in_macro = false;
            }
            continue;
        }
        let delta = count_braces(&code);
        if delta != 0 {
            seen = true;
        }
        depth += delta;
        if seen && depth == 0 {
            return Some(Region { start, end: i });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::Prim;

    fn split(s: &str) -> Vec<&str> {
        s.lines().collect()
    }

    #[test]
    fn enum_sequencial_com_valor_inicial_negativo() {
        let src = split(
            "\
PROTOCOL_COMMAND = -1,
PLAYER_INFO_1 = 0,
PLAYER_INFO_2,
PLAYER_INFO_3,",
        );
        let e = parse_command_enum(&src, 0, src.len());
        assert_eq!(e.len(), 4);
        assert_eq!((e[0].name.as_str(), e[0].value), ("PROTOCOL_COMMAND", -1));
        assert_eq!((e[1].name.as_str(), e[1].value), ("PLAYER_INFO_1", 0));
        assert_eq!((e[2].name.as_str(), e[2].value), ("PLAYER_INFO_2", 1));
        assert_eq!((e[3].name.as_str(), e[3].value), ("PLAYER_INFO_3", 2));
    }

    #[test]
    fn le_a_ancora_numerica_nas_tres_formas_dos_fontes() {
        let src = split(
            "\
A,
B,
C,
D,
E,
F,\t\t// 5
G,\t\t// 40, Own item information
H,\t\t// 390 texto colado
I,\t\t// Reserved for protocol
J,\t\t// comentario com 7 no meio",
        );
        let e = parse_command_enum(&src, 0, src.len());
        assert_eq!(e[5].comment_value, Some(5));
        assert_eq!(e[6].comment_value, Some(40));
        assert_eq!(e[7].comment_value, Some(390));
        // Texto puro e número no meio de uma frase não são âncoras de posição.
        assert_eq!(e[8].comment_value, None);
        assert_eq!(e[9].comment_value, None);
    }

    #[test]
    fn a_ancora_do_comentario_bate_com_o_valor_calculado() {
        let src = split("A,\nB,\nC,\nD,\nE,\nF,\t// 5\nG,\nH,\nI,\nJ,\nK,\t// 10");
        for entry in parse_command_enum(&src, 0, src.len()) {
            if let Some(anchor) = entry.comment_value {
                assert_eq!(entry.value, anchor, "âncora divergente em {}", entry.name);
            }
        }
    }

    #[test]
    fn campos_simples_e_arrays() {
        let mut w = Vec::new();
        let src = split(
            "\
struct cmd_notify_hostpos
{
\tA3DVECTOR3 vPos;
\tint tag;
\tint line;
\tint reserved[10];
\tunsigned char flag;
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        assert_eq!(s.len(), 1);
        let s = &s[0];
        assert_eq!(s.qualified(), "S2C::cmd_notify_hostpos");
        assert_eq!(s.fields.len(), 5);
        assert_eq!(s.fields[0].ty, GdTy::Vec3);
        assert_eq!(s.fields[1].ty, GdTy::Prim(Prim::I32));
        assert_eq!(s.fields[3].array_len, Some(10));
        assert_eq!(s.fields[4].ty, GdTy::Prim(Prim::U8));
        assert!(s.var_list.is_none());
        assert!(w.is_empty());
    }

    #[test]
    fn lista_variavel_por_placeholder_expoe_o_tipo_do_elemento() {
        let mut w = Vec::new();
        let src = split(
            "\
struct cmd_npc_info_list
{
\tunsigned short count;
\tBYTE placeholder;
\tbool CheckValid(size_t buf_size, size_t& sz) const
\t{
\t\tconst BYTE* pData = &placeholder;
\t\tfor (unsigned short i = 0; i < count; i++)
\t\t{
\t\t\tif (!reinterpret_cast<const info_npc*>(pData)->CheckValid(buf_size, sz))
\t\t\t\treturn false;
\t\t}
\t\treturn true;
\t}
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].fields.len(), 2);
        assert_eq!(s[0].fields[1].ty, GdTy::Placeholder);
        assert_eq!(
            s[0].var_list,
            Some(VarList::Placeholder {
                element: "info_npc".into()
            })
        );
    }

    #[test]
    fn lista_variavel_por_membro_array_flexivel() {
        let mut w = Vec::new();
        let src = split(
            "\
struct cmd_matter_info_list
{
\tunsigned short count;
\tinfo_matter list[1];
\tbool CheckValid(size_t buf_size, size_t& sz) const
\t{
\t\tsz = sizeof(*this) - sizeof(list);
\t\tsz += count * sizeof(info_matter);
\t\treturn buf_size >= sz;
\t}
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        assert_eq!(
            s[0].var_list,
            Some(VarList::FlexArray {
                element: "info_matter".into(),
                field: "list".into()
            })
        );
    }

    #[test]
    fn o_mesmo_nome_nos_dois_namespaces_nao_colide() {
        let mut w = Vec::new();
        let src = split("struct cmd_header\n{\n\tunsigned short cmd;\n};");
        let a = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        let b = parse_packed_structs(&src, 0, src.len(), "C2S", &mut w);
        assert_eq!(a[0].qualified(), "S2C::cmd_header");
        assert_eq!(b[0].qualified(), "C2S::cmd_header");
        assert_ne!(a[0].qualified(), b[0].qualified());
    }

    #[test]
    fn tabela_de_payload_com_fall_through() {
        let src = split(
            "\
\tswitch (iCmd)
\t{
\tcase PLAYER_INFO_1:
\tcase PLAYER_ENTER_WORLD:
\tcase PLAYER_ENTER_SLICE:

\t\tCHECK_VALID(info_player_1)
\t\tbreak;

\tcase OBJECT_LEAVE_SLICE:\t\tdwSize = sizeof (cmd_leave_slice);\tbreak;
\tcase PLAYER_INFO_2:\t\t\tbreak;
\t}",
        );
        let t = parse_payload_table(&src, 0, src.len());
        assert_eq!(t.len(), 5);
        // Os três primeiros rótulos compartilham a mesma struct por fall-through.
        for i in 0..3 {
            assert_eq!(t[i].kind, PayloadKind::Variable("info_player_1".into()));
        }
        assert_eq!(t[0].command, "PLAYER_INFO_1");
        assert_eq!(t[2].command, "PLAYER_ENTER_SLICE");
        assert_eq!(t[3].kind, PayloadKind::Fixed("cmd_leave_slice".into()));
        // `case X: break;` sem atribuição é lacuna do próprio cliente, não do parser.
        assert_eq!(t[4].kind, PayloadKind::Unhandled);
    }

    #[test]
    fn corpo_de_metodo_nao_vira_campo() {
        let mut w = Vec::new();
        let src = split(
            "\
struct s
{
\tint id;
\tbool CheckValid(size_t buf_size, size_t& sz) const
\t{
\t\tsize_t sz_org = buf_size;
\t\tint local_que_nao_e_campo;
\t\treturn true;
\t}
\tint depois;
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        let names: Vec<&str> = s[0].fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["id", "depois"]);
    }

    #[test]
    fn codigo_dentro_de_bloco_comentado_nao_vira_campo() {
        // Regressão real: `cmd_player_info_2_list` guarda um exemplo de struct dentro
        // de `/* */`. Lido como código, ele criava um membro `list` inexistente e
        // deslocava todos os campos seguintes — o compilador de 32 bits foi quem
        // apontou a divergência.
        let raw = split(
            "\
struct cmd_player_info_2_list
{
\tunsigned short count;
\t/*
\t   struct
\t   {
\t   int cid;
\t   player_info_2 info;
\t   }list[];
\t */
\tchar data[1];
};",
        );
        let cleaned = strip_block_comments(&raw);
        let src: Vec<&str> = cleaned.iter().map(String::as_str).collect();
        let mut w = Vec::new();
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        assert_eq!(s.len(), 1, "o exemplo comentado virou struct: {s:?}");
        let nomes: Vec<&str> = s[0].fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(nomes, vec!["count", "data"]);
    }

    #[test]
    fn strip_preserva_comentarios_de_linha_e_a_contagem_de_linhas() {
        // As âncoras numéricas dos enums vivem em comentários `//`: removê-las junto
        // apagaria a única verificação independente que o enum tem.
        let raw = split("A,\t// 5\n/* fora */ B,\nC, /* meio */ D,");
        let out = strip_block_comments(&raw);
        assert_eq!(out.len(), 3, "a contagem de linhas mudou");
        assert!(out[0].contains("// 5"));
        assert_eq!(out[1].trim(), "B,");
        assert_eq!(out[2].replace("  ", " ").trim(), "C, D,");
    }


    #[test]
    fn construtor_inline_nao_engole_o_fecho_da_struct() {
        // Regressão: `score_rank_entry():roleid(0), rank(0){}` abre e fecha na mesma
        // linha, então a contagem de chaves dá zero. Tratar isso como "bloco ainda não
        // aberto" fazia a varredura seguir até engolir o `};` da própria struct — e
        // daí em diante todas as structs seguintes viravam aninhadas, em silêncio.
        // A struct seguinte ter sido lida é a prova de que isso não acontece mais.
        let mut w = Vec::new();
        let src = split(
            "\
struct score_rank_entry
{
\tint roleid;
\tint rank;
\tscore_rank_entry():roleid(0), rank(0){}
};
struct depois
{
\tint x;
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        let nomes: Vec<&str> = s.iter().map(|x| x.name.as_str()).collect();
        assert!(nomes.contains(&"depois"), "structs lidas: {nomes:?}");
        assert_eq!(
            s.iter().find(|x| x.name == "score_rank_entry").unwrap().fields.len(),
            2
        );
    }

    #[test]
    fn struct_aninhada_anonima_vira_membro_com_tamanho() {
        let mut w = Vec::new();
        let mut consts = Consts::new();
        consts.insert("GP_PET_SKILL_NUM".to_string(), 8);
        let src = split(
            "\
struct info_pet
{
\tint id;
\tstruct
\t{
\t\tint skill;
\t\tint level;
\t} skills [GP_PET_SKILL_NUM];
\tint reserved[10];
};",
        );
        let s = parse_packed_structs_with(&src, 0, src.len(), "S2C", &[], &consts, &mut w);
        let outer = s.iter().find(|x| x.name == "info_pet").unwrap();
        let nomes: Vec<&str> = outer.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(nomes, vec!["id", "skills", "reserved"]);
        assert_eq!(outer.fields[1].array_len, Some(8));
        assert_eq!(outer.fields[1].ty, GdTy::Struct("info_pet::skills".into()));
        assert!(s.iter().any(|x| x.name == "info_pet::skills"));
        assert!(w.is_empty(), "avisos inesperados: {w:?}");
    }

    #[test]
    fn struct_aninhada_sem_declarador_e_tipo_local_e_nao_membro() {
        // `struct building_data { ... };` dentro de outra struct declara um tipo, não
        // um membro: não ocupa bytes, mas os campos irmãos a referenciam pelo nome.
        let mut w = Vec::new();
        let src = split(
            "\
struct externa
{
\tstruct building_data
\t{
\t\tint id;
\t\tint finish_time;
\t};
\tbuilding_data primeiro;
\tint health;
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        let outer = s.iter().find(|x| x.name == "externa").unwrap();
        let nomes: Vec<&str> = outer.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(nomes, vec!["primeiro", "health"]);
        assert_eq!(
            outer.fields[0].ty,
            GdTy::Struct("externa::building_data".into())
        );
        assert!(w.is_empty(), "avisos inesperados: {w:?}");
    }

    #[test]
    fn typedef_de_namespace_resolve_campo_de_struct() {
        let mut w = Vec::new();
        let src = split(
            "\
namespace S2C
{
\ttypedef abase::vector<IconState> IconStates;

\tstruct cmd_icon_state_notify
\t{
\t\tint id;
\t\tIconStates states;
\t};
}",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        let st = s.iter().find(|x| x.name == "cmd_icon_state_notify").unwrap();
        // Resolvido para o alvo do typedef, que não é copiável por memcpy.
        assert!(st.fields[1].ty.is_unresolved(), "ficou {:?}", st.fields[1].ty);
    }

    #[test]
    fn initialize_marca_a_terceira_forma_de_serializacao() {
        let mut w = Vec::new();
        let src = split(
            "\
struct cmd_x
{
\tint faction_id;
\tint building_count;
\tbool Initialize(const void *pDataBuf, DWORD dwDataSize)
\t{
\t\tif (!Extract(faction_id, p, s)) return false;
\t\treturn true;
\t}
};",
        );
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        assert_eq!(s[0].var_list, Some(VarList::Initialize));
        assert_eq!(s[0].fields.len(), 2);
    }

    #[test]
    fn consts_do_cabecalho_alimentam_comprimentos_de_array() {
        let src = split("enum\n{\n\tOBJECT_EXT_STATE_COUNT = 6,\n};\n#define OUTRA 4");
        let c = parse_consts(&src);
        assert_eq!(c.get("OBJECT_EXT_STATE_COUNT"), Some(&6));
        assert_eq!(c.get("OUTRA"), Some(&4));
    }


    #[test]
    fn campo_com_nome_iniciado_em_underscore_nao_e_descartado() {
        // `int _task_id;` existe em `public_quest_ranks`. Descartá-lo não deixava
        // buraco visível: só empurrava todos os campos seguintes 4 bytes para trás.
        let mut w = Vec::new();
        let src = split("struct s\n{\n\tint a;\n\tint _task_id;\n\tint b;\n};");
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        let nomes: Vec<&str> = s[0].fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(nomes, vec!["a", "_task_id", "b"]);
    }

    #[test]
    fn array_com_tamanho_simbolico_e_marcado_em_vez_de_chutado() {
        let mut w = Vec::new();
        let src = split("struct s\n{\n\tint reserved[MAX_COUNT];\n};");
        let s = parse_packed_structs(&src, 0, src.len(), "S2C", &mut w);
        assert!(s[0].fields[0].ty.is_unresolved());
        assert_eq!(s[0].fields[0].array_len, None);
    }
}
