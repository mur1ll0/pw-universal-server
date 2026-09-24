//! Os codificadores de subcomando S2C, conferidos contra o IR do mundo 3D.
//!
//! # Duas perguntas, com confianças diferentes
//!
//! **1. O id do comando está certo?** Cobrado com rigor. A tabela [`INTENCAO`] abaixo diz
//! qual comando do protocolo cada função *pretende* enviar; o teste extrai o cabeçalho
//! que ela de fato escreve e compara com o id que o IR dá àquele nome. Foi assim que se
//! descobriu que `mall_item_price` mandava `197` — que é `REVIVAL_INQUIRE`, e não a
//! tabela de preços.
//!
//! A tabela é escrita **por intenção**, a partir do nome e do comentário de cada função —
//! nunca gerada a partir do id que o código escreve. Gerá-la do código produziria uma
//! tabela que concorda com qualquer bug.
//!
//! **2. O layout está certo?** Apenas inventariado, e a razão importa: **o IR é do 1.5.3
//! e estes codificadores foram escritos para o 1.2.6.** Um tamanho diferente pode ser
//! erro ou pode ser a versão. O `self_info_1` é o exemplo que separa os dois casos: o
//! comentário dele diz "34 bytes no 1.2.6" e o IR do 1.5.3 diz 38 — diferença real de
//! versão, com um `state2` a mais.
//!
//! Então o teste não "corrige" layout nenhum. Ele **fixa a lista** de quem diverge: uma
//! divergência nova falha (alguém escreveu um codificador errado), e uma que sumiu
//! também falha (foi resolvida — tire da lista). A lista só pode encolher, e nunca em
//! silêncio.
//!
//! # Por que ler o código-fonte em vez de chamar as funções
//!
//! São 79 funções com 79 assinaturas diferentes; construir argumentos para todas seria
//! mais código do que o que está sob teste. Ler o `write_*` do fonte é a mesma técnica
//! que o `pw-rpcgen` usa no C++ e que `campos_contra_o_ir.rs` já usa aqui.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const FONTE: &str = include_str!("../src/packets/s2c.rs");
const OPCODES: &str = include_str!("../src/opcodes.rs");

/// Que comando do protocolo cada codificador **pretende** enviar.
///
/// Escrita à mão a partir do nome e do comentário de cada função. Nomes diferentes para o
/// mesmo comando são normais (`self_*` para `HOST_*`, `party` para `team`) — o que vale é
/// o comando, não a grafia.
const INTENCAO: &[(&str, &str)] = &[
    ("self_info_1", "SELF_INFO_1"),
    ("add_status_point", "ADD_STATUS_POINT"),
    ("attack_once", "ATTACK_ONCE"),
    ("query_title_re", "QUERY_TITLE_RE"),
    ("equip_damaged", "EQUIP_DAMAGED"),
    ("task_deliver_level2", "TASK_DELIVER_LEVEL2"),
    ("gain_pet", "GAIN_PET"),
    ("pet_room", "PET_ROOM"),
    ("host_start_attack", "HOST_START_ATTACK"),
    ("host_stop_attack", "HOST_STOPATTACK"),
    ("update_ext_state", "UPDATE_EXT_STATE"),
    ("icon_state_notify", "ICON_STATE_NOTIFY"),
    ("enchant_result", "ENCHANT_RESULT"),
    ("obtain_item", "HOST_OBTAIN_ITEM"),
    ("player_gather_start", "PLAYER_GATHER_START"),
    ("player_gather_stop", "PLAYER_GATHER_STOP"),
    ("player_waypoint_list", "WAYPOINT_LIST"),
    ("activate_waypoint", "ACTIVATE_WAYPOINT"),
    ("get_own_money", "GET_OWN_MONEY"),
    ("host_reputation", "HOST_REPUTATION"),
    ("pvp_mode", "PVP_MODE"),
    ("self_country_notify", "SELF_COUNTRY_NOTIFY"),
    ("server_time", "SERVER_TIME"),
    ("trashbox_pwd_state", "TRASHBOX_PWD_STATE"),
    ("security_passwd_checked", "SECURITY_PASSWD_CHECKED"),
    ("own_ext_prop", "OWN_EXT_PROP"),
    ("host_skill_attacked", "HOST_SKILL_ATTACKED"),
    ("player_hp_steal", "PLAYER_HP_STEAL"),
    ("object_leave_slice", "OBJECT_LEAVE_SLICE"),
    ("host_correct_pos", "HOST_CORRECT_POS"),
    ("pet_room_capacity", "PET_ROOM_CAPACITY"),
    ("self_king_notify", "SELF_KING_NOTIFY"),
    ("faction_contrib_notify", "FACTION_CONTRIB_NOTIFY"),
    ("player_leadership", "PLAYER_LEADERSHIP"),
    ("player_world_contribution", "PLAYER_WORLD_CONTRIBUTION"),
    ("player_dividend", "PLAYER_DIVIDEND"),
    ("available_double_exp_time", "AVAILABLE_DOUBLE_EXP_TIME"),
    ("double_exp_time", "DOUBLE_EXP_TIME"),
    ("pariah_time", "PARIAH_TIME"),
    ("self_info_00", "SELF_INFO_00"),
    ("notify_hostpos", "NOTIFY_HOSTPOS"),
    ("mall_item_price", "MALL_ITEM_PRICE"),
    ("inst_data_checkout", "INST_DATA_CHECKOUT"),
    ("ext_prop_move", "PLAYER_EXT_PROP_MOVE"),
    ("ext_prop_base", "PLAYER_EXT_PROP_BASE"),
    ("task_data", "TASK_DATA"),
    ("task_var_data", "TASK_VAR_DATA"),
    ("pickup_money", "PICKUP_MONEY"),
    ("pickup_item", "PICKUP_ITEM"),
    ("task_deliver_item", "TASK_DELIVER_ITEM"),
    ("task_deliver_exp", "TASK_DELIVER_EXP"),
    ("task_deliver_money", "TASK_DELIVER_MONEY"),
    ("spend_money", "SPEND_MONEY"),
    ("player_drop_item", "PLAYER_DROP_ITEM"),
    ("set_cooldown", "SET_COOLDOWN"),
    ("elf_exp", "ELF_EXP"),
    ("player_mounting", "PLAYER_MOUNTING"),
    ("player_change_shape", "PLAYER_CHGSHAPE"),
    ("summon_pet", "SUMMON_PET"),
    ("recall_pet", "RECALL_PET"),
    ("player_start_pet_op", "PLAYER_START_PET_OP"),
    ("player_stop_pet_op", "PLAYER_STOP_PET_OP"),
    ("matter_pickup", "MATTER_PICKUP"),
    ("error_message", "ERROR_MESSAGE"),
    ("purchase_item", "PURCHASE_ITEM"),
    ("item_to_money", "ITEM_TO_MONEY"),
    ("item_info", "OWN_ITEM_INFO"),
    ("exg_ivtr_item", "EXG_IVTR_ITEM"),
    ("move_ivtr_item", "MOVE_IVTR_ITEM"),
    ("exg_equip_item", "EXG_EQUIP_ITEM"),
    ("equip_item", "EQUIP_ITEM"),
    ("move_item_to_equip", "MOVE_EQUIP_ITEM"),
    ("unfreeze_ivtr_slot", "UNFREEZE_IVTR_SLOT"),
    ("host_use_item", "HOST_USE_ITEM"),
    ("object_sit_down", "OBJECT_SIT_DOWN"),
    ("object_stand_up", "OBJECT_STAND_UP"),
    ("object_do_emote", "OBJECT_DO_EMOTE"),
    ("skill_data_from_records", "SKILL_DATA"),
    ("skill_data", "SKILL_DATA"),
    ("npc_info_list", "NPC_INFO_LIST"),
    ("team_member_data", "TEAM_MEMBER_DATA"),
    ("own_ivtr_from_items", "OWN_IVTR_DATA"),
    ("player_enter_world", "PLAYER_ENTER_WORLD"),
    ("player_enter_slice", "PLAYER_ENTER_SLICE"),
    ("matter_enter_world", "MATTER_ENTER_WORLD"),
    ("calc_network_delay_re", "CALC_NETWORK_DELAY_RE"),
    ("out_of_sight_list", "OUT_OF_SIGHT_LIST"),
    ("npc_enter_slice", "NPC_ENTER_SLICE"),
    ("npc_enter_world", "NPC_ENTER_WORLD"),
    ("npc_info_00", "NPC_INFO_00"),
    ("equip_data", "EQUIP_DATA"),
    ("object_move", "OBJECT_MOVE"),
    ("object_stop_move", "OBJECT_STOP_MOVE"),
    ("unselect", "UNSELECT"),
    ("object_cast_skill", "OBJECT_CAST_SKILL"),
    ("skill_perform", "SKILL_PERFORM"),
    ("self_skill_attack_result", "HOST_SKILL_ATTACK_RESULT"),
    ("object_skill_attack_result", "OBJECT_SKILL_ATTACK_RESULT"),
    ("self_stop_skill", "HOST_STOP_SKILL"),
    ("skill_interrupted", "SKILL_INTERRUPTED"),
    ("self_skill_interrupted", "SELF_SKILL_INTERRUPTED"),
    ("scene_service_npc_list", "SCENE_SERVICE_NPC_LIST"),
    ("select_target", "SELECT_TARGET"),
    ("npc_greeting", "NPC_GREETING"),
    // Mesmo comando que o `npc_greeting`, sob o nome antigo. Duas funções escrevendo o
    // mesmo layout é a duplicação que o projeto evita; fica anotado.
    ("sevnpc_hello_re", "NPC_GREETING"),
    ("object_disappear", "OBJECT_DISAPPEAR"),
    ("host_attack_result", "HOST_ATTACKRESULT"),
    ("npc_died", "NPC_DIED"),
    // Estes nasceram do IR, e não de engenharia reversa do 1.2.6 — então não entram
    // em `LAYOUT_DIVERGE`: se algum divergir, é bug de quem escreveu.
    ("host_attacked", "HOST_ATTACKED"),
    ("host_died", "HOST_DIED"),
    ("player_revive", "PLAYER_REVIVE"),
    ("player_info_00", "PLAYER_INFO_00"),
    ("team_member_leave", "TEAM_MEMBER_LEAVE"),
    ("receive_exp", "RECEIVE_EXP"),
    ("level_up", "LEVEL_UP"),
    ("object_start_attack", "OBJECT_STARTATTACK"),
    ("repair_all", "REPAIR_ALL"),
    ("repair", "REPAIR"),
    ("learn_skill", "LEARN_SKILL"),
    ("cost_skill_point", "COST_SKILL_POINT"),
    ("produce_start", "PRODUCE_START"),
    ("produce_once", "PRODUCE_ONCE"),
    ("produce_end", "PRODUCE_END"),
    ("decompose_start", "DECOMPOSE_START"),
    ("decompose_end", "DECOMPOSE_END"),
    ("embed_item", "EMBED_ITEM"),
    ("clear_tessera", "CLEAR_TESSERA"),
    ("object_takeoff", "OBJECT_TAKEOFF"),
    ("object_landing", "OBJECT_LANDING"),
    ("flysword_time", "FLYSWORD_TIME"),
    ("team_leader_invite", "TEAM_LEADER_INVITE"),
    ("team_join_party", "TEAM_JOIN_TEAM"),
    ("team_leave_party", "TEAM_LEAVE_PARTY"),
    ("trashbox_open", "TRASHBOX_OPEN"),
    ("trashbox_wealth", "TRASHBOX_WEALTH"),
    ("enter_sanctuary", "ENTER_SANCTUARY"),
    ("leave_sanctuary", "LEAVE_SANCTUARY"),
    ("player_leave_world", "PLAYER_LEAVE_WORLD"),
    ("player_enable_fashion", "PLAYER_ENABLE_FASHION"),
    ("player_cash", "PLAYER_CASH"),
    ("mall_item_buy_failed", "MALL_ITEM_BUY_FAILED"),
    ("invader_rise", "INVADER_RISE"),
    ("pariah_rise", "PARIAH_RISE"),
    ("invader_fade", "INVADER_FADE"),
    ("duel_prepare", "DUEL_PREPARE"),
    ("host_duel_start", "HOST_DUEL_START"),
    ("duel_result", "DUEL_RESULT"),
];

/// Funções que não escrevem cabeçalho porque **delegam** a outra que escreve.
const DELEGAM: &[&str] = &[
    "task_dyn_time_mark",
    "task_dyn_data",
    "task_notify_new",
    "task_notify_complete",
    "task_notify_monster_killed",
    "task_notify_base",
    "task_notify_error",
];

/// Codificadores cujo layout diverge do IR do 1.5.3.
///
/// # O que mudou na leitura desta lista
///
/// Ela nasceu como inventário: "cada um é 1.2.6 legítimo ou palpite não verificado, e não
/// temos como separar os dois". Duas descobertas mudaram isso.
///
/// **A primeira: divergir de tamanho não é cosmético.** O cliente calcula o tamanho
/// esperado de cada comando a partir do `sizeof` da struct e **descarta o comando inteiro**
/// quando não bate (`CalcS2CCmdDataSize` + `ASSERT(dwCmdSize == dwDataSize)`, em
/// `EC_GameDataPrtc.cpp`). Um nome nesta lista não é uma curiosidade de layout: é um
/// comando que o cliente 1.5.3 joga fora.
///
/// **A segunda: dava para separar os dois casos.** O critério é a evidência no próprio
/// código. O `self_info_1` — o exemplo que o projeto sempre citou como divergência real de
/// versão — traz o comentário "struct no 1.2.6 (34 bytes total)", uma medição. Nenhum dos
/// 27 nomes que estavam aqui trazia nada parecido: eram palpites, não layouts medidos do
/// 1.2.6. Onde há palpite de um lado e cabeçalho do cliente do outro, o cabeçalho ganha.
///
/// Treze saíram por isso, conferidos um a um contra o `EC_GPDataType.h`. Os que ficam são
/// comandos que ninguém chama ainda; quando alguém for usá-los, é a mesma conferência.
///
/// Só entram comandos de **tamanho fixo**. Onde o payload varia — uma lista de itens, um
/// bloco de octetos — o IR não declara tamanho, e não há o que comparar. **Isso é um
/// buraco**, e o `team_member_data` caiu nele: escrevia um cabeçalho de 1 byte onde o
/// cliente lê 6, e sobreviveu porque comando de tamanho variável não era conferido. Ver
/// `o_prefixo_fixo_dos_comandos_variaveis_bate_com_o_ir`.
///
/// **Esta lista só encolhe.** Quem resolver um caso tira o nome daqui.
const LAYOUT_DIVERGE: &[&str] = &[
    "repair",
    "produce_start",
    "produce_once",
    "decompose_start",
    "embed_item",
    "clear_tessera",
    "flysword_time",
    "trashbox_open",
    "trashbox_wealth",
    "mall_item_buy_failed",
    "pariah_rise",
    "duel_result",
    // Estes quatro passaram a divergir em 2026-09-04, quando `bytes_do_comando` parou
    // de pular structs "de tamanho variável" que só têm um rabo condicional (ver o
    // comentário da função). São divergências REAIS e MEDIDAS — cada um tem o layout
    // certo em `PorVersao` (1.2.6 menor, 1.5.3+ com o(s) campo(s) extra do IR), e o
    // que aparece aqui é só a base `S2CGamedataSend::*` (o layout do 1.2.6), que nunca
    // teve por que mudar. Ver `docs/ESTADO_E_RETOMADA.md`, itens 14 e 15.
    "task_data",             // TASK_DATA: 3 blocos no 1.2.6, 5 do 1.5.3 em diante
    "npc_enter_world",       // NPC_ENTER_WORLD: sem vis_tid/state2 no 1.2.6
    "npc_enter_slice",       // NPC_ENTER_SLICE: mesma struct de NPC_ENTER_WORLD
    "self_info_1",           // SELF_INFO_1: sem state2 no 1.2.6/1.5.3 (só o 1.5.5 tem
                              // evidência de fonte pra esse campo — ver `PorVersao`)
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

/// Os corpos das funções `pub fn nome(...) -> Self` do `impl S2CGamedataSend`.
fn codificadores() -> BTreeMap<String, String> {
    let ini = FONTE
        .find("impl S2CGamedataSend")
        .expect("o `impl S2CGamedataSend` sumiu — este teste precisa ser revisto");
    let corpo = &FONTE[ini..];

    let mut fns = BTreeMap::new();
    let mut resto = corpo;
    while let Some(p) = resto.find("pub fn ") {
        let depois = &resto[p + 7..];
        let Some(par) = depois.find('(') else { break };
        let nome = depois[..par].trim().to_string();
        // O corpo vai até o fecho de função na indentação do `impl`.
        let fim = depois.find("\n    }").unwrap_or(depois.len());
        let corpo_fn = depois[..fim].to_string();
        if nome != "new" && corpo_fn.contains("-> Self") {
            fns.insert(nome, corpo_fn);
        }
        resto = &depois[fim.min(depois.len())..];
        if resto.is_empty() {
            break;
        }
    }
    assert!(
        fns.len() > 60,
        "só {} codificadores foram lidos do fonte — a extração quebrou",
        fns.len()
    );
    fns
}

/// Constantes reexportadas como `NOME as CMD_S2C_X`, para resolver cabeçalhos nomeados.
fn apelidos() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for linha in OPCODES.lines() {
        let mut resto = linha;
        while let Some(p) = resto.find(" as CMD_S2C_") {
            let antes: String = resto[..p]
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let depois = &resto[p + 4..];
            let apelido: String = depois
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            m.insert(apelido, antes);
            resto = &resto[p + 4..];
        }
    }
    m
}

/// O id de comando que a função escreve no cabeçalho.
fn cabecalho_escrito(corpo: &str, apelidos: &BTreeMap<String, String>, ids: &BTreeMap<String, i64>) -> Option<i64> {
    let p = corpo.find("write_u16_le(")?;
    let arg: String = corpo[p + 13..]
        .chars()
        .take_while(|c| *c != ')')
        .collect::<String>()
        .trim()
        .to_string();
    if let Ok(n) = arg.parse::<i64>() {
        return Some(n);
    }
    let curto = arg.rsplit("::").next()?.to_string();
    let nome_ir = apelidos.get(&curto)?;
    ids.get(nome_ir).copied()
}

#[test]
fn cada_codificador_escreve_o_id_que_o_ir_da_ao_comando() {
    let ir = ir();
    let ids: BTreeMap<String, i64> = ir["commands"]["s2c"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap().to_string(),
                c["id"].as_i64().unwrap(),
            )
        })
        .collect();

    let fns = codificadores();
    let ap = apelidos();
    let mut erros = Vec::new();

    for (funcao, comando) in INTENCAO {
        let Some(corpo) = fns.get(*funcao) else {
            erros.push(format!("`{funcao}` está na tabela mas não existe no fonte"));
            continue;
        };
        let esperado = match ids.get(*comando) {
            Some(v) => *v,
            None => {
                erros.push(format!("`{comando}` não existe no IR"));
                continue;
            }
        };
        match cabecalho_escrito(corpo, &ap, &ids) {
            Some(escrito) if escrito == esperado => {}
            Some(escrito) => {
                let outro = ir["commands"]["s2c"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"].as_i64() == Some(escrito))
                    .map(|c| c["name"].as_str().unwrap())
                    .unwrap_or("nenhum comando");
                erros.push(format!(
                    "`{funcao}` quer mandar {comando} ({esperado}) mas escreve {escrito}, \
                     que é {outro}"
                ));
            }
            None => erros.push(format!("`{funcao}`: não consegui ler o cabeçalho")),
        }
    }

    assert!(erros.is_empty(), "\n  - {}", erros.join("\n  - "));
}

#[test]
fn todo_codificador_esta_declarado_em_algum_lugar() {
    // Sem isto, acrescentar um codificador novo e esquecer a tabela faria o teste acima
    // passar sem verificar o novo — que é como uma rede de proteção deixa de proteger.
    let fns = codificadores();
    let declarados: BTreeSet<&str> = INTENCAO
        .iter()
        .map(|(f, _)| *f)
        .chain(DELEGAM.iter().copied())
        .collect();

    let faltando: Vec<&String> = fns
        .keys()
        .filter(|n| !declarados.contains(n.as_str()))
        .collect();

    assert!(
        faltando.is_empty(),
        "codificadores sem entrada na tabela `INTENCAO` (ou em `DELEGAM`): {faltando:?}"
    );
}

#[test]
fn as_funcoes_que_delegam_realmente_delegam() {
    let fns = codificadores();
    for nome in DELEGAM {
        let corpo = fns
            .get(*nome)
            .unwrap_or_else(|| panic!("`{nome}` não existe no fonte"));
        assert!(
            corpo.contains("Self::"),
            "`{nome}` está listada como delegante mas não chama outra construtora — se \
             ela passou a escrever o próprio cabeçalho, mova-a para a `INTENCAO`"
        );
    }
}

#[test]
fn a_lista_de_divergencias_de_layout_esta_em_dia() {
    // Uma divergência nova é erro de quem escreveu o codificador. Uma que sumiu é boa
    // notícia — e mesmo assim falha, para que a lista não fique mentindo.
    let ir = ir();
    let fns = codificadores();
    let ap = apelidos();
    let ids: BTreeMap<String, i64> = ir["commands"]["s2c"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap().to_string(),
                c["id"].as_i64().unwrap(),
            )
        })
        .collect();

    let mut divergem = BTreeSet::new();
    for (funcao, comando) in INTENCAO {
        let Some(corpo) = fns.get(*funcao) else { continue };
        let Some(bytes_ir) = bytes_do_comando(&ir, comando) else {
            continue;
        };
        // Payload de tamanho variável não entra nesta conta: contar os `write_*` de
        // dentro de um laço daria um número que não corresponde a nada.
        if variavel(corpo) {
            continue;
        }
        let Some(_) = cabecalho_escrito(corpo, &ap, &ids) else {
            continue;
        };
        let escritos = bytes_escritos(corpo);
        if escritos != bytes_ir {
            divergem.insert(*funcao);
        }
    }

    let esperados: BTreeSet<&str> = LAYOUT_DIVERGE.iter().copied().collect();

    let novas: Vec<_> = divergem.difference(&esperados).collect();
    assert!(
        novas.is_empty(),
        "divergências de layout NOVAS — o codificador não bate com o IR: {novas:?}"
    );

    let resolvidas: Vec<_> = esperados.difference(&divergem).collect();
    assert!(
        resolvidas.is_empty(),
        "estes já batem com o IR: tire-os de `LAYOUT_DIVERGE` {resolvidas:?}"
    );
}

/// O **prefixo fixo** de um comando de tamanho variável bate com o IR?
///
/// # O buraco que este teste fecha
///
/// A conferência de layout acima pula tudo que tem laço, porque contar `write_*` dentro de
/// um `for` daria um número sem significado. O efeito colateral era que o **cabeçalho**
/// desses comandos — a parte que é de tamanho fixo, antes da lista — não era conferido
/// por ninguém.
///
/// Foi por aí que passou o `team_member_data`: ele escrevia `member_count` e ia direto
/// para a lista, quando o cliente lê `member_count`, `data_count` e `idLeader` — 1 byte
/// onde ele conta 6. Como o cliente calcula o tamanho esperado com
/// `sizeof(*this) - sizeof(data) + data_count * sizeof(MEMBER)`, faltar o `data_count`
/// não erra por 5 bytes: erra por 5 mais o tamanho de todos os membros.
///
/// O IR marca o começo da parte variável com `array_len`, e o `offset` desse campo é
/// exatamente o tamanho do prefixo fixo. É o que este teste compara com o que a função
/// escreve **antes** do primeiro laço.
#[test]
fn o_prefixo_fixo_dos_comandos_variaveis_bate_com_o_ir() {
    let ir = ir();
    let fns = codificadores();
    let ap = apelidos();
    let ids: BTreeMap<String, i64> = ir["commands"]["s2c"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap().to_string(),
                c["id"].as_i64().unwrap(),
            )
        })
        .collect();

    let mut errados = Vec::new();
    let mut conferidos = 0usize;

    for (funcao, comando) in INTENCAO {
        let Some(corpo) = fns.get(*funcao) else { continue };
        if !variavel(corpo) {
            continue;
        }
        // Sem cabeçalho escrito, a função delega — não há prefixo próprio para conferir.
        if cabecalho_escrito(corpo, &ap, &ids).is_none() {
            continue;
        }
        let Some(prefixo_ir) = prefixo_fixo_do_ir(&ir, comando) else {
            // O IR não marca parte variável neste comando: ou o payload é opaco (um bloco
            // de octetos sem estrutura declarada) ou a struct não foi resolvida. Nos dois
            // casos não há prefixo a comparar.
            continue;
        };

        let escrito = bytes_escritos(prefixo_do_corpo(corpo));
        conferidos += 1;
        if escrito != prefixo_ir {
            errados.push(format!(
                "{funcao} ({comando}): escreve {escrito} bytes antes da lista, o IR diz {prefixo_ir}"
            ));
        }
    }

    assert!(
        conferidos > 0,
        "nenhum comando de tamanho variável foi conferido — o teste deixou de valer alguma coisa"
    );
    assert!(
        errados.is_empty(),
        "prefixo fixo errado em comando de tamanho variável:\n  {}",
        errados.join("\n  ")
    );
}

/// O trecho do corpo antes da primeira escrita de conteúdo variável **no fluxo de saída**.
///
/// A distinção importa. O `own_ivtr_from_items` monta a lista num fluxo auxiliar
/// (`content`) e só depois escreve o comprimento e o bloco no fluxo de saída — cortar no
/// primeiro `for` do texto pararia antes do `write_u32_le(content_bytes.len())`, que é
/// parte do prefixo fixo, e o teste acusaria um erro que não existe.
///
/// Então: corta no primeiro `write_raw_bytes`/`write_octets`/`write_string` do fluxo de
/// saída, ou no primeiro `for` cujo corpo escreve **nele**.
fn prefixo_do_corpo(corpo: &str) -> &str {
    let mut corte = corpo.len();

    for marca in ["stream.write_raw_bytes", "stream.write_octets", "stream.write_string"] {
        if let Some(p) = corpo.find(marca) {
            corte = corte.min(p);
        }
    }

    // Um `for` só encerra o prefixo se o corpo dele escrever no fluxo de saída.
    let mut resto = corpo;
    let mut base = 0usize;
    while let Some(p) = resto.find("for ") {
        let depois = &resto[p..];
        let fim = depois.find("\n        }").map(|f| f + 10).unwrap_or(depois.len());
        if depois[..fim].contains("stream.write_") {
            corte = corte.min(base + p);
            break;
        }
        base += p + 4;
        resto = &resto[p + 4..];
    }

    &corpo[..corte]
}

/// Quantos bytes o IR põe antes do primeiro campo de tamanho variável.
///
/// É o `offset` do primeiro campo com `array_len`: tudo antes dele é o cabeçalho fixo.
fn prefixo_fixo_do_ir(ir: &Value, comando: &str) -> Option<usize> {
    let c = ir["commands"]["s2c"]
        .as_array()?
        .iter()
        .find(|c| c["name"] == comando)?;
    let nome = c["struct"].as_str()?;
    ir["structs"][nome]["fields"]
        .as_array()?
        .iter()
        .find(|f| !f["array_len"].is_null())
        .and_then(|f| f["offset"].as_u64())
        .map(|o| o as usize)
}

/// A função escreve um payload de tamanho variável?
///
/// Um laço, uma lista de octetos ou uma string tornam o tamanho dependente dos dados —
/// e a comparação com o `bytes` fixo do IR deixa de fazer sentido.
fn variavel(corpo: &str) -> bool {
    corpo.contains("for ")
        || corpo.contains("write_octets")
        || corpo.contains("write_raw_bytes")
        || corpo.contains("write_string")
        || corpo.contains("Self::")
}

/// Tamanho, em bytes, do payload que o IR dá ao comando (sem o cabeçalho: as structs do
/// **cliente** não o incluem, ao contrário das do servidor).
///
/// **Achado em 2026-09-04**: quando a struct não tem `bytes` fixo no IR (o `pw-rpcgen`
/// marca como "variável" toda struct com campos condicionais no fim, mesmo que o começo
/// seja fixo), esta função devolvia `None` e o chamador simplesmente pulava a
/// comparação — sem contar como divergência nem como confirmação. Foi assim que
/// `TASK_DATA` (3 blocos escritos contra 5 do IR) e `NPC_ENTER_WORLD`/`NPC_ENTER_SLICE`
/// (27 bytes contra 35, faltando `vis_tid`/`state2`) atravessaram este teste sem
/// acusar nada — os dois derrubaram o crash de render e a visibilidade de NPC no
/// 1.5.5 (ver `docs/ESTADO_E_RETOMADA.md`, itens 14 e 15). Agora cai pro
/// **tamanho-base**: a soma dos campos que TÊM tamanho fixo, que é o mínimo que
/// qualquer registro real desse comando ocupa (os campos condicionais só aumentam,
/// nunca diminuem esse piso). Só continua devolvendo `None` quando algum campo em si
/// não tem tamanho fixo — aí a struct é variável de verdade, não só "tem um rabo
/// condicional", e a comparação não se aplica.
fn bytes_do_comando(ir: &Value, comando: &str) -> Option<usize> {
    let c = ir["commands"]["s2c"]
        .as_array()?
        .iter()
        .find(|c| c["name"] == comando)?;
    let nome = c["struct"].as_str()?;
    let s = &ir["structs"][nome];
    if let Some(b) = s["bytes"].as_u64() {
        return Some(b as usize);
    }
    let campos = s["fields"].as_array()?;
    if campos.is_empty() {
        return None;
    }
    let mut total = 0usize;
    for campo in campos {
        total += campo["bytes"].as_u64()? as usize;
    }
    Some(total)
}

/// Quantos bytes a função escreve **no fluxo de saída**, depois do cabeçalho.
///
/// Conta só `stream.write_*`, e não qualquer `write_`: um codificador pode montar um
/// bloco num fluxo auxiliar antes de escrevê-lo (é o que o `own_ivtr_from_items` faz), e
/// somar esses bytes aqui daria o total do payload em vez do tamanho do prefixo.
fn bytes_escritos(corpo: &str) -> usize {
    let mut total = 0usize;
    let mut primeiro = true;
    let mut resto = corpo;
    while let Some(p) = resto.find("stream.write_") {
        let d = &resto[p + 13..];
        let tipo: String = d.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let n = match tipo.trim_end_matches("_le") {
            "u8" | "i8" | "bool" => 1,
            "u16" | "i16" => 2,
            "u32" | "i32" | "f32" => 4,
            "u64" | "i64" | "f64" => 8,
            _ => 0,
        };
        if n > 0 {
            if primeiro {
                primeiro = false; // o cabeçalho não conta
            } else {
                total += n;
            }
        }
        resto = &d[tipo.len()..];
    }
    total
}
