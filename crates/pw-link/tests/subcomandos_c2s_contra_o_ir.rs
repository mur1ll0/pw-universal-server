//! Cada braço do `match` de subcomandos trata o comando que diz tratar.
//!
//! # O que isto pega
//!
//! O `gateway.rs` despacha subcomandos do mundo 3D por um `match` sobre o id. Um id errado
//! ali não dá erro nenhum: o servidor simplesmente executa o tratador errado para o pedido
//! do jogador. Foi assim que se descobriu, de uma vez:
//!
//! | id | o que o código dizia | o que o IR diz |
//! | ---: | :--- | :--- |
//! | 32 | `SEVNPC_HELLO` | `TEAM_MEMBER_POS` |
//! | 33 | `SEVNPC_SERVE` | `GET_OTHER_EQUIP` |
//! | 76 | `LEAVE_SANCTUARY` | `OPEN_BOOTH` |
//! | 106 | consulta de saldo | `MALL_SHOPPING` (comprar) |
//! | 107 | comprar na loja | `GET_WALLOW_INFO` |
//! | 120 | comprar na loja | `CHECK_SECURITY_PASSWD` |
//! | 192 | modo de moda | **não existe** |
//! | 214–217 | duelo | **não existem** |
//! | 218–220 | duelo | comandos de **GM** |
//!
//! A compra e a consulta de saldo estavam **trocadas entre si**: comprar devolvia saldo, e
//! uma consulta de embriaguez disparava uma compra.
//!
//! # A tabela é por intenção
//!
//! [`INTENCAO`] diz, para cada id tratado, qual comando do protocolo aquele braço
//! *pretende* atender. Ela é escrita à mão a partir do comentário do braço — **nunca**
//! gerada a partir dos ids do código, que produziria uma tabela concordando com qualquer
//! erro.
//!
//! # Sobre 1.2.6 contra 1.5.3
//!
//! O IR é do 1.5.3 e o `gateway.rs` atende o 1.2.6, então "diverge" poderia ser versão. Não
//! é o caso aqui: mais de vinte ids batem exatamente (27, 28, 30, 35, 37, 39, 40, 41, 42,
//! 46, 47, 48, 49, 67, 68, 75, 80, 85, 92, 110, 118…). Numa tabela em que quase tudo bate,
//! os poucos que destoam são engano, e não outra numeração. Todos os corrigidos eram, além
//! disso, o id **extra** de um par `A | B` — palpites acrescentados a um id certo.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const FONTE: &str = include_str!("../src/gateway.rs");

/// Que comando do protocolo cada braço do `match` pretende atender.
const INTENCAO: &[(u16, &str)] = &[
    (23, "GET_EXT_PROP_BASE"),
    (24, "GET_EXT_PROP_MOVE"),
    (25, "GET_EXT_PROP_ATK"),
    (26, "GET_EXT_PROP_DEF"),
    (35, "SEVNPC_HELLO"),
    (49, "TASK_NOTIFY"),
    (85, "SWITCH_FASHION_MODE"),
    (92, "DUEL_REQUEST"),
    (118, "GET_MALL_ITEM_PRICE"),
];

fn ir() -> Value {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gamedata_153.json"
    );
    let texto = std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    serde_json::from_str(&texto).expect("o IR não é JSON válido")
}

/// Os ids que o `match cmd` do `gateway.rs` trata.
///
/// Os braços de nível superior têm 24 espaços de indentação; os `match` aninhados (o de
/// `service_type`, dentro do `SEVNPC_SERVE`) têm mais, e por isso não entram.
fn ids_tratados() -> BTreeSet<u16> {
    const INDENT: &str = "                        ";
    let mut ids = BTreeSet::new();

    for linha in FONTE.lines() {
        let Some(resto) = linha.strip_prefix(INDENT) else {
            continue;
        };
        // Descarta linhas mais indentadas que isto.
        if resto.starts_with(' ') {
            continue;
        }
        let Some(padrao) = resto.strip_suffix(" => {") else {
            continue;
        };
        let padrao = padrao.trim();

        if let Some((a, b)) = padrao.split_once("..=") {
            if let (Ok(a), Ok(b)) = (a.trim().parse::<u16>(), b.trim().parse::<u16>()) {
                ids.extend(a..=b);
            }
            continue;
        }
        for parte in padrao.split('|') {
            if let Ok(v) = parte.trim().parse::<u16>() {
                ids.insert(v);
            }
        }
    }

    // Guarda contra uma extração quebrada, que devolveria um conjunto vazio e faria os
    // testes passarem por vacuidade. O número é baixo de propósito: o `match` **encolhe** a
    // cada comando que migra para o `pw-gs`, e um piso alto viraria falso positivo — como
    // já virou uma vez, quando ele estava em 15.
    assert!(
        !ids.is_empty(),
        "nenhum braço foi lido do `gateway.rs` — a extração quebrou"
    );
    ids
}

fn nomes_do_ir(ir: &Value) -> BTreeMap<u16, String> {
    ir["commands"]["c2s"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| {
            let id = c["id"].as_i64()?;
            (id >= 0).then(|| (id as u16, c["name"].as_str().unwrap().to_string()))
        })
        .collect()
}

#[test]
fn cada_braco_trata_o_comando_que_diz_tratar() {
    let ir = ir();
    let nomes = nomes_do_ir(&ir);
    let mut erros = Vec::new();

    for (id, pretende) in INTENCAO {
        match nomes.get(id) {
            Some(real) if real == pretende => {}
            Some(real) => erros.push(format!(
                "o braço {id} quer tratar `{pretende}`, mas {id} é `{real}` no IR"
            )),
            None => erros.push(format!(
                "o braço {id} quer tratar `{pretende}`, mas o id {id} não existe na tabela C2S"
            )),
        }
    }

    assert!(erros.is_empty(), "\n  - {}", erros.join("\n  - "));
}

#[test]
fn todo_braco_do_match_esta_declarado() {
    // Sem isto, acrescentar um braço e esquecer a tabela deixaria o novo id sem
    // conferência — que é exatamente como os ids errados entraram.
    let tratados = ids_tratados();
    let declarados: BTreeSet<u16> = INTENCAO.iter().map(|(i, _)| *i).collect();

    let sem_declaracao: Vec<u16> = tratados.difference(&declarados).copied().collect();
    assert!(
        sem_declaracao.is_empty(),
        "braços do `match` sem entrada em `INTENCAO`: {sem_declaracao:?}"
    );

    let declarados_a_mais: Vec<u16> = declarados.difference(&tratados).copied().collect();
    assert!(
        declarados_a_mais.is_empty(),
        "`INTENCAO` declara ids que o `match` não trata mais — tire-os: {declarados_a_mais:?}"
    );
}

#[test]
fn nenhum_braco_pisa_em_comando_de_gm() {
    // O braço de duelo cobria `214..=220`, e 218 a 220 são `GM_QUERY_SPEC_ITEM`,
    // `GM_REMOVE_SPEC_ITEM` e `GM_OPEN_ACTIVITY`. Um tratador de jogo não deve responder a
    // comando de GM por engano de faixa — nem que hoje ele só devolva pacotes inofensivos.
    let ir = ir();
    let nomes = nomes_do_ir(&ir);
    let tratados = ids_tratados();

    let de_gm: Vec<String> = tratados
        .iter()
        .filter_map(|id| {
            let n = nomes.get(id)?;
            n.starts_with("GM_").then(|| format!("{id} ({n})"))
        })
        .collect();

    assert!(
        de_gm.is_empty(),
        "o `match` de gameplay está tratando comandos de GM: {de_gm:?}"
    );
}

#[test]
fn os_comandos_ja_migrados_nao_sobraram_no_gateway() {
    // Um braço esquecido aqui depois de o comando migrar para o `pw-gs` faria os dois
    // tratarem o mesmo pedido — dois movimentos, duas respostas, dois débitos de HP.
    let tratados = ids_tratados();
    let migrados: &[(u16, &str)] = &[
        (0, "PLAYER_MOVE"),
        (1, "LOGOUT"),
        (2, "SELECT_TARGET"),
        (3, "NORMAL_ATTACK"),
        (4, "REVIVE_VILLAGE"),
        (7, "STOP_MOVE"),
        (8, "UNSELECT"),
        (9, "GET_ITEM_INFO"),
        (11, "GET_IVTR_DETAIL"),
        (12, "EXG_IVTR_ITEM"),
        (13, "MOVE_IVTR_ITEM"),
        (16, "EXG_EQUIP_ITEM"),
        (17, "EQUIP_ITEM"),
        (18, "MOVE_ITEM_TO_EQUIP"),
        (42, "CANCEL_ACTION"),
        (46, "SIT_DOWN"),
        (47, "STAND_UP"),
        (48, "EMOTE_ACTION"),
        (75, "ENTER_SANCTUARY"),
        (37, "SEVNPC_SERVE"),
        (40, "USE_ITEM"),
        (41, "CAST_SKILL"),
        (80, "CAST_INSTANT_SKILL"),
        (27, "TEAM_INVITE"),
        (28, "TEAM_AGREE_INVITE"),
        (29, "TEAM_REJECT_INVITE"),
        (30, "TEAM_LEAVE_PARTY"),
    ];

    let duplicados: Vec<&str> = migrados
        .iter()
        .filter(|(id, _)| tratados.contains(id))
        .map(|(_, n)| *n)
        .collect();

    assert!(
        duplicados.is_empty(),
        "estes já são tratados pelo `pw-gs` e continuam no `gateway.rs`: {duplicados:?}"
    );
}
