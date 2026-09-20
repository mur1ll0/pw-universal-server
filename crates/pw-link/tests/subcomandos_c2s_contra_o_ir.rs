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
        // Um braço pode ter guarda (`178 if ...`): o id é o que vem antes do `if`, e o
        // comando continua tratado aqui — só que sob condição.
        let padrao = padrao.split(" if ").next().unwrap_or(padrao).trim();

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

/// Os ids que o `BusServer::tratar_subcomando` do `pw-gs` trata, lidos do próprio fonte.
///
/// Até 2026-09-14 esta lista era escrita à mão aqui, e ficou para trás: `SEVNPC_HELLO`
/// (35), `TASK_NOTIFY` (49) e `SWITCH_FASHION_MODE` (85) migraram para o mundo sem sair do
/// `gateway.rs`, e o cliente recebia duas respostas — no 49, uma delas com o `reason`
/// errado. Lendo do fonte do mundo, um comando que migra entra na conferência sozinho.
fn ids_tratados_pelo_mundo() -> BTreeMap<u16, String> {
    const COMANDOS: &str = include_str!("../../pw-gs/src/comandos.rs");
    const MUNDO: &str = include_str!("../../pw-gs/src/bus_server.rs");

    // `pub const NOME: u16 = N;` do módulo `ids`.
    let mut valor_de = BTreeMap::new();
    for linha in COMANDOS.lines() {
        let Some(resto) = linha.trim().strip_prefix("pub const ") else {
            continue;
        };
        let Some((nome, valor)) = resto.split_once(": u16 = ") else {
            continue;
        };
        if let Ok(v) = valor.trim_end_matches(';').trim().parse::<u16>() {
            valor_de.insert(nome.to_string(), v);
        }
    }

    // Braços `ids::A | ids::B => ...` do `match` de `tratar_subcomando`.
    let mut tratados = BTreeMap::new();
    for linha in MUNDO.lines() {
        let t = linha.trim();
        let Some((padrao, _)) = t.split_once(" => ") else {
            continue;
        };
        if !padrao.starts_with("ids::") {
            continue;
        }
        for parte in padrao.split('|') {
            let nome = parte.trim().trim_start_matches("ids::");
            let id = *valor_de
                .get(nome)
                .unwrap_or_else(|| panic!("`ids::{nome}` não está em `comandos.rs`"));
            tratados.insert(id, nome.to_string());
        }
    }

    // Guarda contra extração quebrada, que faria o teste passar por vacuidade. O mundo
    // trata dezenas de comandos e só cresce.
    assert!(
        tratados.len() >= 30,
        "só {} braços lidos do `tratar_subcomando` — a extração quebrou",
        tratados.len()
    );
    tratados
}

#[test]
fn os_comandos_ja_migrados_nao_sobraram_no_gateway() {
    // Um braço esquecido aqui depois de o comando migrar para o `pw-gs` faz os dois
    // tratarem o mesmo pedido: o `gateway.rs` repassa todo `GamedataSend` ao mundo **e**
    // executa o próprio braço — duas respostas, dois movimentos, dois débitos.
    let no_link = ids_tratados();
    let duplicados: Vec<String> = ids_tratados_pelo_mundo()
        .into_iter()
        .filter(|(id, _)| no_link.contains(id))
        .map(|(id, nome)| format!("{id} ({nome})"))
        .collect();

    assert!(
        duplicados.is_empty(),
        "estes já são tratados pelo `pw-gs` e continuam no `gateway.rs`: {duplicados:?}"
    );
}
