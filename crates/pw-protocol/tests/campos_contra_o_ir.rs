//! Confere, campo a campo, o que cada pacote escreve no fio contra o IR.
//!
//! Um `encode()` é código, não dado, então não dá para inspecioná-lo em tempo de
//! execução. A saída é a mesma técnica que o `pw-rpcgen` usa com o C++: **ler o fonte** e
//! extrair a sequência de `write_*`/`read_*` de cada `encode`/`decode`, achatar a
//! estrutura correspondente do `gnet_153.json` na sequência de escalares que ela põe no
//! fio, e comparar as duas.
//!
//! Isto encontrou **nove pacotes com o layout errado**, entre eles:
//!
//! * `ChatBroadCast` escrevia um `sender_name` que não existe no protocolo e omitia
//!   `emotion` e `data` — tudo depois do primeiro campo saía deslocado;
//! * `CreateRole_Re` mandava 3 campos dos 27: o `RoleInfo` inteiro ficava de fora,
//!   embora o personagem já estivesse na struct;
//! * `GetFriends_Re` tinha um campo `result` inexistente e o `localsid` na posição
//!   errada;
//! * `SetUIConfig` e `SetCustomData` liam a configuração a partir dos bytes do
//!   `localsid`, que faltava;
//! * `PlayerHeartBeat` lia um `i8` onde o protocolo tem três campos de 4 bytes.
//!
//! # O que o extrator entende, e o que não entende
//!
//! Ele reconhece as chamadas `stream.write_*` / `stream.read_*` e **segue chamadas a
//! funções auxiliares livres** que recebem o `stream` (como `write_role_info`), sem o
//! que extrair uma estrutura repetida para uma função pareceria uma regressão.
//!
//! Ele **não** entende ramificação: um `match` cujos braços escrevam sequências
//! diferentes conta as duas. Isso é intencional — é uma pressão para que o layout tenha
//! um caminho de escrita só, que é o que evita duas listas de campos saindo de sincronia.
//! Um `if` que apenas acrescenta campos no fim (o corte de versão do 1.2.6) funciona,
//! porque o IR é o do 1.5.3, que os inclui.

use serde_json::Value;
use std::collections::BTreeMap;

/// Pacote → protocolo do IR. Quem não está aqui não é conferido: os pacotes sem
/// correspondência no IR estão listados em `opcodes::nao_no_ir` e são dívida conhecida.
const MAPA: &[(&str, &str)] = &[
    ("S2CChallenge", "Challenge"),
    ("S2CKeyExchange", "KeyExchange"),
    ("S2COnlineAnnounce", "OnlineAnnounce"),
    ("S2CErrorInfo", "ErrorInfo"),
    ("S2CStatusAnnounce", "StatusAnnounce"),
    ("S2CRoleListResponse", "RoleList_Re"),
    ("S2CCreateRoleResponse", "CreateRole_Re"),
    ("S2CDeleteRoleResponse", "DeleteRole_Re"),
    ("S2CUndoDeleteRoleResponse", "UndoDeleteRole_Re"),
    ("S2CSelectRoleResponse", "SelectRole_Re"),
    ("S2CChatBroadcast", "ChatBroadCast"),
    ("S2CGetUIConfigRe", "GetUIConfig_Re"),
    ("S2CGetFriendListRe", "GetFriends_Re"),
    ("S2CGetHelpStatesRe", "GetHelpStates_Re"),
    ("C2SChallengeResponse", "Response"),
    ("C2SKeyExchange", "KeyExchange"),
    ("C2SRoleList", "RoleList"),
    ("C2SCreateRole", "CreateRole"),
    ("C2SSelectRole", "SelectRole"),
    ("C2SDeleteRole", "DeleteRole"),
    ("C2SUndoDeleteRole", "UndoDeleteRole"),
    ("C2SEnterWorld", "EnterWorld"),
    ("C2SPlayerChat", "ChatMessage"),
    ("C2SHeartbeat", "PlayerHeartBeat"),
    ("C2SGetUIConfig", "GetUIConfig"),
    ("C2SSetUIConfig", "SetUIConfig"),
    ("C2SGetFriendList", "GetFriends"),
    ("C2SGetHelpStates", "GetHelpStates"),
    ("C2SSetHelpStates", "SetHelpStates"),
    ("C2SACReport", "ACReport"),
    ("C2SSetCustomData", "SetCustomData"),
];

const FONTES: &[(&str, &str)] = &[
    ("s2c.rs", include_str!("../src/packets/s2c.rs")),
    ("c2s.rs", include_str!("../src/packets/c2s.rs")),
];

/// O `adapter.rs` é uma **segunda implementação** dos mesmos layouts, e é a que o
/// `codec.rs` usa para boa parte dos pacotes S2C. Ter duas cópias é a origem exata deste
/// tipo de bug, então enquanto a duplicação existir ela é conferida junto.
const ADAPTER: &str = include_str!("../src/adapter.rs");

/// Método do adapter → protocolo do IR.
const MAPA_ADAPTER: &[(&str, &str)] = &[
    ("encode_key_exchange", "KeyExchange"),
    ("encode_online_announce", "OnlineAnnounce"),
    ("encode_status_announce", "StatusAnnounce"),
    ("encode_role_list", "RoleList_Re"),
    ("encode_create_role_response", "CreateRole_Re"),
    ("encode_delete_role_response", "DeleteRole_Re"),
    ("encode_undo_delete_role_response", "UndoDeleteRole_Re"),
    ("encode_select_role_response", "SelectRole_Re"),
];

/// Nome da chamada → tipo escalar que ela põe no fio.
fn tipo_da_chamada(nome: &str) -> Option<&'static str> {
    Some(match nome {
        "u8" => "u8",
        "i8" => "i8",
        "u16" => "u16",
        "i16" => "i16",
        "u32" => "u32",
        "i32" => "i32",
        "u64" => "u64",
        "i64" => "i64",
        "f32" => "f32",
        "f64" => "f64",
        // `Octets`, `std::string` e `seq<u8>` têm o mesmo formato: CompactUINT(n) e n
        // bytes. Os três são a mesma coisa no fio.
        "octets" | "string_utf16le" | "string_utf8" => "octets",
        "compact_uint" => "seq_len",
        _ => return None,
    })
}

/// Um passo extraído do fonte.
#[derive(Debug, Clone)]
struct Passo {
    tipo: String,
    linha: usize,
}

/// Extrai a sequência de escalares que um bloco de código põe no fio.
///
/// `chamada_auxiliar` recebe o nome de uma função livre chamada com `stream` e devolve
/// os passos dela, para que a expansão aconteça no lugar da chamada.
fn passos_do_bloco(
    linhas: &[&str],
    inicio: usize,
    auxiliares: &BTreeMap<String, Vec<Passo>>,
) -> (Vec<Passo>, usize) {
    let mut passos = Vec::new();
    let mut profundidade = 0i32;
    let mut abriu = false;
    let mut i = inicio;

    while i < linhas.len() {
        let linha = linhas[i];
        profundidade += linha.matches('{').count() as i32 - linha.matches('}').count() as i32;
        if linha.contains('{') {
            abriu = true;
        }

        for (antes, chamada) in ocorrencias(linha) {
            match antes {
                // `stream.write_u32(...)` / `stream.read_u32(...)`
                Antes::Metodo => {
                    let base = chamada
                        .strip_prefix("write_")
                        .or_else(|| chamada.strip_prefix("read_"))
                        .unwrap_or(chamada);
                    if let Some(t) = tipo_da_chamada(base) {
                        passos.push(Passo {
                            tipo: t.to_string(),
                            linha: i + 1,
                        });
                    }
                }
                // `write_role_info(stream, ...)`
                Antes::Livre => {
                    if let Some(sub) = auxiliares.get(chamada) {
                        passos.extend(sub.iter().cloned());
                    }
                }
            }
        }

        if abriu && profundidade <= 0 {
            return (passos, i);
        }
        i += 1;
    }
    (passos, linhas.len())
}

enum Antes {
    Metodo,
    Livre,
}

/// Encontra chamadas relevantes numa linha, distinguindo método de função livre.
fn ocorrencias(linha: &str) -> Vec<(Antes, &str)> {
    let bytes = linha.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
            continue;
        }
        let inicio = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let nome = &linha[inicio..i];
        if !linha[i..].starts_with('(') {
            continue;
        }

        let metodo = inicio > 0 && bytes[inicio - 1] == b'.';
        if metodo {
            if nome.starts_with("write_") || nome.starts_with("read_") {
                out.push((Antes::Metodo, nome));
            } else if nome.starts_with("encode_") {
                // `self.encode_role_info(stream, c)` — auxiliar do adapter, chamada
                // como método. Sem seguir, `encode_role_list` pareceria escrever 5
                // campos em vez dos 38 que escreve.
                out.push((Antes::Livre, nome));
            }
        } else if nome.starts_with("write_") && linha[i..].starts_with("(stream") {
            out.push((Antes::Livre, nome));
        }
    }
    out
}

/// Colhe as funções livres `fn write_*(stream, ...)` e a sequência de cada uma.
fn colher_auxiliares(linhas: &[&str]) -> BTreeMap<String, Vec<Passo>> {
    let mut out = BTreeMap::new();
    for (i, linha) in linhas.iter().enumerate() {
        let t = linha.trim_start();
        let resto = t.strip_prefix("pub fn ").or_else(|| t.strip_prefix("fn "));
        let Some(resto) = resto else { continue };
        let nome: String = resto
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !(nome.starts_with("write_") || nome.starts_with("encode_")) {
            continue;
        }
        let (passos, _) = passos_do_bloco(linhas, i, &BTreeMap::new());
        out.insert(nome, passos);
    }
    out
}

/// Extrai, de um fonte, a sequência de cada `encode`/`decode` por struct.
fn extrair(fonte: &str) -> BTreeMap<String, Vec<Passo>> {
    let linhas: Vec<&str> = fonte.lines().collect();
    let auxiliares = colher_auxiliares(&linhas);

    let mut out = BTreeMap::new();
    let mut struct_atual: Option<String> = None;
    let mut i = 0;

    while i < linhas.len() {
        let t = linhas[i].trim_start();
        if let Some(resto) = t.strip_prefix("impl ") {
            let nome: String = resto
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !nome.is_empty() {
                struct_atual = Some(nome);
            }
        }
        let e_encode = t.starts_with("pub fn encode(") || t.starts_with("pub fn decode(");
        if e_encode {
            if let Some(s) = struct_atual.clone() {
                let (passos, fim) = passos_do_bloco(&linhas, i, &auxiliares);
                out.insert(s, passos);
                i = fim;
            }
        }
        i += 1;
    }
    out
}

/// Achata os campos de uma estrutura do IR na sequência de escalares do fio.
///
/// Um `seq` vira a contagem seguida do corpo do elemento **uma vez** — que é a forma do
/// laço no código.
fn achatar(
    campos: &[Value],
    structs: &BTreeMap<&str, &Value>,
    prof: u32,
    out: &mut Vec<(String, String)>,
) {
    if prof > 4 {
        return;
    }
    for f in campos {
        let nome = f["name"].as_str().unwrap_or("?").to_string();
        let t = &f["type"];
        match t["kind"].as_str().unwrap_or("") {
            "prim" => out.push((t["prim"].as_str().unwrap().to_string(), nome)),
            "octets" | "string" => out.push(("octets".into(), nome)),
            "seq" => {
                let item = &t["item"];
                // `seq<u8>` é byte a byte o mesmo que `Octets`.
                if item["kind"] == "prim" && item["prim"] == "u8" {
                    out.push(("octets".into(), nome));
                    continue;
                }
                out.push(("seq_len".into(), nome.clone()));
                match item["kind"].as_str().unwrap_or("") {
                    "prim" => out.push((item["prim"].as_str().unwrap().to_string(), nome)),
                    "octets" | "string" => out.push(("octets".into(), nome)),
                    "struct" => {
                        let alvo = structs[item["name"].as_str().unwrap()];
                        achatar(alvo["fields"].as_array().unwrap(), structs, prof + 1, out);
                    }
                    outro => out.push((format!("?{outro}"), nome)),
                }
            }
            "struct" => {
                let alvo = structs[t["name"].as_str().unwrap()];
                achatar(alvo["fields"].as_array().unwrap(), structs, prof + 1, out);
            }
            outro => out.push((format!("?{outro}"), nome)),
        }
    }
}

#[test]
fn cada_pacote_escreve_os_campos_na_ordem_do_ir() {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gnet_153.json"
    );
    let texto = std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    let ir: Value = serde_json::from_str(&texto).expect("o IR não é JSON válido");

    let protocolos: BTreeMap<&str, &Value> = ir["protocols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p))
        .collect();
    let structs: BTreeMap<&str, &Value> = ir["structs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["name"].as_str().unwrap(), s))
        .collect();

    let mut codigo = BTreeMap::new();
    for (_, fonte) in FONTES {
        codigo.extend(extrair(fonte));
    }

    let mut falhas: Vec<String> = Vec::new();
    let mut conferidos = 0usize;
    let mut escalares = 0usize;

    for (pacote, protocolo) in MAPA {
        let Some(obtido) = codigo.get(*pacote) else {
            falhas.push(format!("{pacote}: não achei encode()/decode() no fonte"));
            continue;
        };
        let p = protocolos
            .get(protocolo)
            .unwrap_or_else(|| panic!("{protocolo} não existe no IR"));

        let mut esperado = Vec::new();
        achatar(p["fields"].as_array().unwrap(), &structs, 0, &mut esperado);

        let mut div = Vec::new();
        if esperado.len() != obtido.len() {
            div.push(format!(
                "quantidade de campos: IR {}, código {}",
                esperado.len(),
                obtido.len()
            ));
        }
        for (i, (e, o)) in esperado.iter().zip(obtido.iter()).enumerate() {
            if e.0 != o.tipo {
                div.push(format!(
                    "campo {i} (`{}`): o IR diz {}, o código escreve {} (linha {})",
                    e.1, e.0, o.tipo, o.linha
                ));
            }
        }

        if div.is_empty() {
            conferidos += 1;
            escalares += esperado.len();
        } else {
            div.truncate(5);
            falhas.push(format!("\n  {pacote} vs {protocolo}:\n    {}", div.join("\n    ")));
        }
    }

    assert!(
        falhas.is_empty(),
        "{} pacote(s) divergem do IR:{}",
        falhas.len(),
        falhas.join("")
    );
    eprintln!("campos: {conferidos} pacotes conferidos ({escalares} escalares)");
    assert_eq!(
        conferidos,
        MAPA.len(),
        "nem todos os pacotes do mapa foram conferidos"
    );
}

#[test]
fn a_segunda_implementacao_no_adapter_tambem_bate_com_o_ir() {
    // O `adapter.rs` reimplementa os mesmos layouts, e é **ele** que o `codec.rs` usa
    // para boa parte dos pacotes S2C. Enquanto as duas cópias existirem, as duas
    // precisam ser conferidas — foi só ao auditar esta que ficou claro que o
    // `encode_challenge` do adapter ignorava o campo `edition` da struct e mandava
    // sempre vazio, o que tornava inútil preencher o campo do outro lado.
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gnet_153.json"
    );
    let texto = std::fs::read_to_string(caminho).expect("não consegui ler o IR");
    let ir: Value = serde_json::from_str(&texto).expect("o IR não é JSON válido");

    let protocolos: BTreeMap<&str, &Value> = ir["protocols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p))
        .collect();
    let structs: BTreeMap<&str, &Value> = ir["structs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["name"].as_str().unwrap(), s))
        .collect();

    let linhas: Vec<&str> = ADAPTER.lines().collect();
    let auxiliares = colher_auxiliares(&linhas);

    let mut falhas = Vec::new();
    let mut conferidos = 0usize;

    for (metodo, protocolo) in MAPA_ADAPTER {
        let alvo = format!("fn {metodo}(");
        let Some(i) = linhas.iter().position(|l| l.trim_start().starts_with(&alvo)) else {
            falhas.push(format!("{metodo}: não achei no adapter.rs"));
            continue;
        };
        let (obtido, _) = passos_do_bloco(&linhas, i, &auxiliares);

        let p = protocolos[protocolo];
        let mut esperado = Vec::new();
        achatar(p["fields"].as_array().unwrap(), &structs, 0, &mut esperado);

        let mut div = Vec::new();
        if esperado.len() != obtido.len() {
            div.push(format!(
                "quantidade de campos: IR {}, adapter {}",
                esperado.len(),
                obtido.len()
            ));
        }
        for (k, (e, o)) in esperado.iter().zip(obtido.iter()).enumerate() {
            if e.0 != o.tipo {
                div.push(format!(
                    "campo {k} (`{}`): o IR diz {}, o adapter escreve {} (linha {})",
                    e.1, e.0, o.tipo, o.linha
                ));
            }
        }

        if div.is_empty() {
            conferidos += 1;
        } else {
            div.truncate(5);
            falhas.push(format!(
                "\n  adapter::{metodo} vs {protocolo}:\n    {}",
                div.join("\n    ")
            ));
        }
    }

    assert!(
        falhas.is_empty(),
        "{} método(s) do adapter divergem do IR:{}",
        falhas.len(),
        falhas.join("")
    );
    eprintln!("campos: {conferidos} métodos do adapter conferidos");
    assert_eq!(conferidos, MAPA_ADAPTER.len());
}
