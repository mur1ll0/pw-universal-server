//! Confere as constantes de opcode contra o IR extraído dos fontes C++ originais.
//!
//! Um opcode é um número escrito à mão, e nada no compilador impede que ele esteja
//! errado. O `specs/protocol/gnet_153.json` sabe o valor certo de cada `PROTOCOL_*`
//! porque foi extraído dos `callid.hxx` do servidor, então este teste transforma
//! "alguém digitou 112" em "o número bate com o C++, e vai continuar batendo".
//!
//! Ele existe porque uma auditoria encontrou **doze constantes com valor errado e cinco
//! sem protocolo correspondente** — várias apontando para outro protocolo de verdade,
//! como um `OP_C2S_CHAT` valendo 112, que é `GetTaskData_Re`.

use pw_protocol::opcodes;
use serde_json::Value;

fn carregar(nome: &str) -> Value {
    let caminho = format!(
        "{}/../../specs/protocol/{nome}",
        env!("CARGO_MANIFEST_DIR")
    );
    let texto = std::fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    serde_json::from_str(&texto).expect("o IR não é JSON válido")
}

#[test]
fn cada_opcode_bate_com_o_protocolo_do_ir() {
    let ir = carregar("gnet_153.json");

    // Símbolo `PROTOCOL_*` → id, direto do IR.
    let por_simbolo: std::collections::BTreeMap<&str, i64> = ir["protocols"]
        .as_array()
        .expect("`protocols` deveria ser lista")
        .iter()
        .filter_map(|p| Some((p["symbol"].as_str()?, p["id"].as_i64()?)))
        .collect();

    let mut divergencias = Vec::new();
    for (constante, simbolo, valor) in opcodes::CONFERIDOS {
        match por_simbolo.get(simbolo) {
            Some(&esperado) if esperado == i64::from(*valor) => {}
            Some(&esperado) => divergencias.push(format!(
                "{constante}: vale {valor}, mas {simbolo} é {esperado}"
            )),
            None => divergencias.push(format!("{constante}: {simbolo} não existe no IR")),
        }
    }

    assert!(
        divergencias.is_empty(),
        "{} opcode(s) divergem do IR:\n  {}",
        divergencias.len(),
        divergencias.join("\n  ")
    );
    eprintln!("opcodes: {} constantes conferidas", opcodes::CONFERIDOS.len());
}

#[test]
fn nenhum_opcode_conferido_esconde_uma_colisao() {
    // Dois protocolos diferentes nunca compartilham um id. Então, se duas constantes
    // com **símbolos diferentes** tiverem o mesmo valor, uma delas está errada — foi
    // exatamente esse o caso de `SetCustomData` e `SetUIConfig`, que reivindicavam
    // 102/103 ao mesmo tempo.
    let mut por_valor: std::collections::BTreeMap<u32, Vec<&str>> = Default::default();
    for (_, simbolo, valor) in opcodes::CONFERIDOS {
        let entrada = por_valor.entry(*valor).or_default();
        if !entrada.contains(simbolo) {
            entrada.push(simbolo);
        }
    }
    let colisoes: Vec<String> = por_valor
        .iter()
        .filter(|(_, simbolos)| simbolos.len() > 1)
        .map(|(valor, simbolos)| format!("{valor} reivindicado por {simbolos:?}"))
        .collect();

    assert!(
        colisoes.is_empty(),
        "símbolos diferentes com o mesmo opcode:\n  {}",
        colisoes.join("\n  ")
    );
}

#[test]
fn os_sem_correspondencia_sao_exatamente_os_conhecidos() {
    // A lista de dívida não pode crescer sem alguém notar. Se um opcode novo for
    // inventado, ele cai aqui; se um dos atuais for resolvido, o teste cobra que a
    // lista encolha junto.
    let esperados = [
        "OP_S2C_ENTER_WORLD",
        "OP_C2S_GET_WAIT_DEL_ROLES",
        "OP_S2C_GET_WAIT_DEL_ROLES_RE",
        "OP_C2S_QUERY_SERVER_TIME",
        "OP_S2C_QUERY_SERVER_TIME_RE",
        "OP_S2C_PLAYER_MOVE_BROADCAST",
    ];
    let atuais: Vec<&str> = opcodes::nao_no_ir::SEM_CORRESPONDENCIA
        .iter()
        .map(|(nome, _)| *nome)
        .collect();
    assert_eq!(
        atuais, esperados,
        "a lista de opcodes sem correspondência mudou — atualize o teste junto"
    );
}

#[test]
fn os_subcomandos_do_gamedata_batem_com_o_ir() {
    let ir = carregar("gamedata_153.json");

    let por_nome: std::collections::BTreeMap<&str, i64> = ir["commands"]["s2c"]
        .as_array()
        .expect("`commands.s2c` deveria ser lista")
        .iter()
        .filter_map(|c| Some((c["name"].as_str()?, c["id"].as_i64()?)))
        .collect();

    let mut divergencias = Vec::new();
    for (nome, valor) in opcodes::gamedata_s2c::CONFERIDOS {
        match por_nome.get(nome) {
            Some(&esperado) if esperado == i64::from(*valor) => {}
            Some(&esperado) => {
                divergencias.push(format!("{nome}: vale {valor}, mas o IR diz {esperado}"))
            }
            None => divergencias.push(format!("{nome}: não existe no enum S2C do IR")),
        }
    }

    assert!(
        divergencias.is_empty(),
        "{} subcomando(s) divergem do IR:\n  {}",
        divergencias.len(),
        divergencias.join("\n  ")
    );
    eprintln!(
        "gamedata: {} subcomandos conferidos",
        opcodes::gamedata_s2c::CONFERIDOS.len()
    );
}

#[test]
fn os_subcomandos_usados_sao_comandos_de_verdade() {
    // O IR marca entradas que não são comandos despacháveis (`PROTOCOL_COMMAND`, que é
    // reservado, e os contadores `NUM_*`). Registrar manipulador para uma delas criaria
    // um comando fantasma.
    let ir = carregar("gamedata_153.json");
    let por_nome: std::collections::BTreeMap<&str, &str> = ir["commands"]["s2c"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| Some((c["name"].as_str()?, c["role"].as_str()?)))
        .collect();

    for (nome, _) in opcodes::gamedata_s2c::CONFERIDOS {
        assert_eq!(
            por_nome.get(nome),
            Some(&"command"),
            "{nome} não é um comando despachável no IR"
        );
    }
}

#[test]
fn o_codigo_de_versao_do_153_e_o_game_version_do_cliente() {
    // Vem de `CElementClient/EC_Game.cpp:115` dos fontes do cliente:
    //
    //     DWORD GAME_VERSION = ((0 << 24) | (1 << 16) | (5 << 8) | 2);
    //
    // O cliente compara este número com o campo `version` do `Challenge` e derruba a
    // conexão antes de olhar a senha. Note o `2` final: o cliente que todo mundo chama
    // de "1.5.3" carrega 1.5.**2** aqui, e deduzir o valor a partir do nome da versão
    // foi exatamente o erro que existia (`0x00010503`, e antes disso um `804` fixo).
    let esperado = (0 << 24) | (1 << 16) | (5 << 8) | 2;
    assert_eq!(esperado, 0x0001_0502);
    assert_eq!(
        pw_protocol::GameVersion::V1_5_3.server_version_code(),
        esperado
    );
}

#[test]
fn o_challenge_leva_o_codigo_de_versao_e_nao_um_numero_fixo() {
    use pw_protocol::{Edition, GameVersion, S2CChallenge};

    // Versões diferentes precisam produzir códigos diferentes: se alguém voltar a
    // fixar o valor, os dois lados abaixo passam a ser iguais e o teste falha.
    let v153 = S2CChallenge::new(
        vec![0; 8],
        GameVersion::V1_5_3,
        Edition::new(GameVersion::V1_5_3, 1, 2, None),
    );
    let v126 = S2CChallenge::new(
        vec![0; 8],
        GameVersion::V1_2_6,
        Edition::new(GameVersion::V1_2_6, 1, 2, None),
    );

    assert_eq!(v153.server_version, 0x0001_0502);
    assert_ne!(v153.server_version, v126.server_version);
}
