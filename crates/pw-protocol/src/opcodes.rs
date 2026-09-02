//! Opcodes dos protocolos GNET, **conferidos contra o IR** extraído dos fontes C++.
//!
//! Cada constante da primeira seção traz o símbolo `PROTOCOL_*` do servidor original e
//! entra na tabela [`CONFERIDOS`]. O teste de integração `opcodes_contra_o_ir` compara a
//! tabela inteira com `specs/protocol/gnet_153.json`, de modo que um número escrito à
//! mão não sobrevive a um `cargo test`.
//!
//! # Por que isto passou a existir
//!
//! Uma auditoria de todas as constantes contra o IR encontrou **doze opcodes com o valor
//! errado e cinco que não correspondem a protocolo nenhum**. Vários não eram apenas
//! errados — apontavam para outro protocolo de verdade, o que é bem pior do que um valor
//! inexistente:
//!
//! | Constante | Valor antigo | O que aquele valor de fato é | Certo |
//! | :--- | ---: | :--- | ---: |
//! | `OP_C2S_RESPONSE` | 2 | `KeyExchange` | **3** |
//! | `OP_*_KEYEXCHANGE` | 3 | `Response` | **2** |
//! | `OP_C2S_CHAT` | 112 | `GetTaskData_Re` | **80** |
//! | `OP_S2C_CHAT_BROADCAST` | 113 | `SetTaskData` | **120** |
//! | `OP_C2S_HEARTBEAT` | 90 | `KeepAlive` (entre daemons) | **93** |
//! | `OP_C2S_SET_CUSTOM_DATA` | 102 | `SetUIConfig` | **100** |
//! | `OP_S2C_SET_CUSTOM_DATA_RE` | 103 | `SetUIConfig_Re` | **101** |
//! | `OP_C2S_SET_UI_CONFIG` | 106 | `DisconnectPlayer` | **102** |
//! | `OP_S2C_SET_UI_CONFIG_RE` | 107 | `GetPlayerBriefInfo` | **103** |
//!
//! Um `OP_C2S_CHAT` valendo 112 não faz o chat "não funcionar": faz o servidor
//! **decodificar uma resposta de dados de missão como se fosse uma mensagem de chat**.
//! É o tipo de erro que só aparece quando alguém abre a boca dentro do jogo.
//!
//! # `KeepAlive` (90) não é o heartbeat do jogador
//!
//! São dois protocolos distintos, e o IR separa os dois pelos daemons que os falam:
//! `KeepAlive` (90) é falado por `glinkd`, `gdeliveryd`, `gamed`, `uniquenamed` e
//! `gfaction` — é o keepalive **entre daemons**. `PlayerHeartBeat` (93) é falado só por
//! `glinkd` e `gamed`, e é o do jogador.

// ---------------------------------------------------------------------------
// Conferidos contra o IR
// ---------------------------------------------------------------------------

pub const OP_S2C_CHALLENGE: u32 = 1; // PROTOCOL_CHALLENGE

// ---------------------------------------------------------------------------
// Os dois que **trocam de número** entre as versões
// ---------------------------------------------------------------------------
//
// No 1.5.3 (IR): 2 = `KeyExchange`, 3 = `Response`.
// No 1.2.6 (medido): **2 = `Response`, 3 = `KeyExchange`** — o contrário.
//
// A medida está em `docs/HANDSHAKE_DO_126.md`: numa captura do login de um servidor
// 1.2.6 real, o cliente manda o opcode 2 com `Octets("teste") + Octets(16 bytes)` — o
// nome de usuário em claro e o resumo da senha, que é um `Response` e não uma troca de
// chaves — e o servidor responde com o opcode 3 valendo `Octets(16) + i8(0)`, que é
// exatamente `KeyExchange { nonce, blkickuser }`.
//
// Enquanto valeu a numeração do 1.5.3 para todo mundo, o `Response` do cliente 1.2.6
// caía no ramo do `KeyExchange`, que só escreve uma linha de log: o login **nunca
// acontecia**, o cliente ficava em "Conectando ao jogo" e a conexão morria sem erro
// nenhum de nenhum dos dois lados.
//
// Estas constantes ficam com o nome da versão para que ninguém as use sem escolher:
// quem precisa do número certo chama [`crate::GameVersion::opcode_response`] e
// [`crate::GameVersion::opcode_key_exchange`].

/// `Response` no 1.5.3 e no 1.4.8 (não medido no 1.4.8 — ver a nota nos métodos).
pub const OP_RESPONSE_153: u32 = 3;
/// `KeyExchange` no 1.5.3 e no 1.4.8.
pub const OP_KEYEXCHANGE_153: u32 = 2;
/// `Response` no 1.2.6, medido na captura de 2026-09-01.
pub const OP_RESPONSE_126: u32 = 2;
/// `KeyExchange` no 1.2.6, medido na mesma captura.
pub const OP_KEYEXCHANGE_126: u32 = 3;
pub const OP_S2C_ONLINEANNOUNCE: u32 = 4; // PROTOCOL_ONLINEANNOUNCE
pub const OP_S2C_ERRORINFO: u32 = 5; // PROTOCOL_ERRORINFO
pub const OP_S2C_STATUSANNOUNCE: u32 = 6; // PROTOCOL_STATUSANNOUNCE
pub const OP_S2C_ROLESTATUSANNOUNCE: u32 = 7; // PROTOCOL_ROLESTATUSANNOUNCE

/// `GamedataSend` é **um só opcode nos dois sentidos** entre cliente e `glinkd`: o que
/// distingue ida de volta é o subcomando no payload, não o opcode.
pub const OP_C2S_GAMEDATASEND: u32 = 34; // PROTOCOL_GAMEDATASEND
pub const OP_S2C_GAMEDATASEND: u32 = 34; // PROTOCOL_GAMEDATASEND

pub const OP_S2C_PLAYER_LOGOUT: u32 = 69; // PROTOCOL_PLAYERLOGOUT
pub const OP_C2S_SELECT_ROLE: u32 = 70; // PROTOCOL_SELECTROLE
pub const OP_S2C_SELECT_ROLE_RE: u32 = 71; // PROTOCOL_SELECTROLE_RE
pub const OP_C2S_ENTER_WORLD: u32 = 72; // PROTOCOL_ENTERWORLD
pub const OP_C2S_CHAT: u32 = 80; // PROTOCOL_CHATMESSAGE
pub const OP_C2S_ROLE_LIST: u32 = 82; // PROTOCOL_ROLELIST
pub const OP_S2C_ROLE_LIST_RES: u32 = 83; // PROTOCOL_ROLELIST_RE
pub const OP_C2S_CREATE_ROLE: u32 = 84; // PROTOCOL_CREATEROLE
pub const OP_S2C_CREATE_ROLE_RES: u32 = 85; // PROTOCOL_CREATEROLE_RE
pub const OP_C2S_DELETE_ROLE: u32 = 86; // PROTOCOL_DELETEROLE
pub const OP_S2C_DELETE_ROLE_RES: u32 = 87; // PROTOCOL_DELETEROLE_RE
pub const OP_C2S_UNDO_DELETE_ROLE: u32 = 88; // PROTOCOL_UNDODELETEROLE
pub const OP_S2C_UNDO_DELETE_ROLE_RES: u32 = 89; // PROTOCOL_UNDODELETEROLE_RE

/// O heartbeat **do jogador**. Não confundir com `KeepAlive` (90), que é entre daemons.
pub const OP_C2S_HEARTBEAT: u32 = 93; // PROTOCOL_PLAYERHEARTBEAT

pub const OP_C2S_SET_CUSTOM_DATA: u32 = 100; // PROTOCOL_SETCUSTOMDATA
pub const OP_S2C_SET_CUSTOM_DATA_RE: u32 = 101; // PROTOCOL_SETCUSTOMDATA_RE
pub const OP_C2S_SET_UI_CONFIG: u32 = 102; // PROTOCOL_SETUICONFIG
pub const OP_S2C_SET_UI_CONFIG_RE: u32 = 103; // PROTOCOL_SETUICONFIG_RE
pub const OP_C2S_GET_UI_CONFIG: u32 = 104; // PROTOCOL_GETUICONFIG
pub const OP_S2C_GET_UI_CONFIG_RE: u32 = 105; // PROTOCOL_GETUICONFIG_RE
pub const OP_S2C_CHAT_BROADCAST: u32 = 120; // PROTOCOL_CHATBROADCAST
pub const OP_C2S_SET_HELP_STATES: u32 = 128; // PROTOCOL_SETHELPSTATES
pub const OP_S2C_SET_HELP_STATES_RE: u32 = 129; // PROTOCOL_SETHELPSTATES_RE
pub const OP_C2S_GET_HELP_STATES: u32 = 130; // PROTOCOL_GETHELPSTATES
pub const OP_S2C_GET_HELP_STATES_RE: u32 = 131; // PROTOCOL_GETHELPSTATES_RE
pub const OP_C2S_GET_FRIEND_LIST: u32 = 206; // PROTOCOL_GETFRIENDS
pub const OP_S2C_GET_FRIEND_LIST_RE: u32 = 207; // PROTOCOL_GETFRIENDS_RE
pub const OP_C2S_CHECK_NEW_MAIL: u32 = 4200; // PROTOCOL_CHECKNEWMAIL

/// O aviso de correio novo é `AnnounceNewMail`, um protocolo próprio — **não** é a
/// resposta de `CheckNewMail`, apesar de o id ser o seguinte.
pub const OP_S2C_ANNOUNCE_NEW_MAIL: u32 = 4201; // PROTOCOL_ANNOUNCENEWMAIL

pub const OP_C2S_ACREPORT: u32 = 5001; // PROTOCOL_ACREPORT

/// Ligação constante → símbolo `PROTOCOL_*` → valor, conferida contra o IR pelo teste.
pub const CONFERIDOS: &[(&str, &str, u32)] = &[
    ("OP_S2C_CHALLENGE", "PROTOCOL_CHALLENGE", OP_S2C_CHALLENGE),
    // Estes dois são conferidos contra o IR **do 1.5.3**, que é o que o IR descreve. O
    // par do 1.2.6 é o inverso e tem teste próprio, contra a captura.
    ("OP_KEYEXCHANGE_153", "PROTOCOL_KEYEXCHANGE", OP_KEYEXCHANGE_153),
    ("OP_RESPONSE_153", "PROTOCOL_RESPONSE", OP_RESPONSE_153),
    ("OP_S2C_ONLINEANNOUNCE", "PROTOCOL_ONLINEANNOUNCE", OP_S2C_ONLINEANNOUNCE),
    ("OP_S2C_ERRORINFO", "PROTOCOL_ERRORINFO", OP_S2C_ERRORINFO),
    ("OP_S2C_STATUSANNOUNCE", "PROTOCOL_STATUSANNOUNCE", OP_S2C_STATUSANNOUNCE),
    (
        "OP_S2C_ROLESTATUSANNOUNCE",
        "PROTOCOL_ROLESTATUSANNOUNCE",
        OP_S2C_ROLESTATUSANNOUNCE,
    ),
    ("OP_C2S_GAMEDATASEND", "PROTOCOL_GAMEDATASEND", OP_C2S_GAMEDATASEND),
    ("OP_S2C_GAMEDATASEND", "PROTOCOL_GAMEDATASEND", OP_S2C_GAMEDATASEND),
    ("OP_S2C_PLAYER_LOGOUT", "PROTOCOL_PLAYERLOGOUT", OP_S2C_PLAYER_LOGOUT),
    ("OP_C2S_SELECT_ROLE", "PROTOCOL_SELECTROLE", OP_C2S_SELECT_ROLE),
    ("OP_S2C_SELECT_ROLE_RE", "PROTOCOL_SELECTROLE_RE", OP_S2C_SELECT_ROLE_RE),
    ("OP_C2S_ENTER_WORLD", "PROTOCOL_ENTERWORLD", OP_C2S_ENTER_WORLD),
    ("OP_C2S_CHAT", "PROTOCOL_CHATMESSAGE", OP_C2S_CHAT),
    ("OP_C2S_ROLE_LIST", "PROTOCOL_ROLELIST", OP_C2S_ROLE_LIST),
    ("OP_S2C_ROLE_LIST_RES", "PROTOCOL_ROLELIST_RE", OP_S2C_ROLE_LIST_RES),
    ("OP_C2S_CREATE_ROLE", "PROTOCOL_CREATEROLE", OP_C2S_CREATE_ROLE),
    ("OP_S2C_CREATE_ROLE_RES", "PROTOCOL_CREATEROLE_RE", OP_S2C_CREATE_ROLE_RES),
    ("OP_C2S_DELETE_ROLE", "PROTOCOL_DELETEROLE", OP_C2S_DELETE_ROLE),
    ("OP_S2C_DELETE_ROLE_RES", "PROTOCOL_DELETEROLE_RE", OP_S2C_DELETE_ROLE_RES),
    (
        "OP_C2S_UNDO_DELETE_ROLE",
        "PROTOCOL_UNDODELETEROLE",
        OP_C2S_UNDO_DELETE_ROLE,
    ),
    (
        "OP_S2C_UNDO_DELETE_ROLE_RES",
        "PROTOCOL_UNDODELETEROLE_RE",
        OP_S2C_UNDO_DELETE_ROLE_RES,
    ),
    ("OP_C2S_HEARTBEAT", "PROTOCOL_PLAYERHEARTBEAT", OP_C2S_HEARTBEAT),
    ("OP_C2S_SET_CUSTOM_DATA", "PROTOCOL_SETCUSTOMDATA", OP_C2S_SET_CUSTOM_DATA),
    (
        "OP_S2C_SET_CUSTOM_DATA_RE",
        "PROTOCOL_SETCUSTOMDATA_RE",
        OP_S2C_SET_CUSTOM_DATA_RE,
    ),
    ("OP_C2S_SET_UI_CONFIG", "PROTOCOL_SETUICONFIG", OP_C2S_SET_UI_CONFIG),
    (
        "OP_S2C_SET_UI_CONFIG_RE",
        "PROTOCOL_SETUICONFIG_RE",
        OP_S2C_SET_UI_CONFIG_RE,
    ),
    ("OP_C2S_GET_UI_CONFIG", "PROTOCOL_GETUICONFIG", OP_C2S_GET_UI_CONFIG),
    (
        "OP_S2C_GET_UI_CONFIG_RE",
        "PROTOCOL_GETUICONFIG_RE",
        OP_S2C_GET_UI_CONFIG_RE,
    ),
    ("OP_S2C_CHAT_BROADCAST", "PROTOCOL_CHATBROADCAST", OP_S2C_CHAT_BROADCAST),
    ("OP_C2S_SET_HELP_STATES", "PROTOCOL_SETHELPSTATES", OP_C2S_SET_HELP_STATES),
    (
        "OP_S2C_SET_HELP_STATES_RE",
        "PROTOCOL_SETHELPSTATES_RE",
        OP_S2C_SET_HELP_STATES_RE,
    ),
    ("OP_C2S_GET_HELP_STATES", "PROTOCOL_GETHELPSTATES", OP_C2S_GET_HELP_STATES),
    (
        "OP_S2C_GET_HELP_STATES_RE",
        "PROTOCOL_GETHELPSTATES_RE",
        OP_S2C_GET_HELP_STATES_RE,
    ),
    ("OP_C2S_GET_FRIEND_LIST", "PROTOCOL_GETFRIENDS", OP_C2S_GET_FRIEND_LIST),
    (
        "OP_S2C_GET_FRIEND_LIST_RE",
        "PROTOCOL_GETFRIENDS_RE",
        OP_S2C_GET_FRIEND_LIST_RE,
    ),
    ("OP_C2S_CHECK_NEW_MAIL", "PROTOCOL_CHECKNEWMAIL", OP_C2S_CHECK_NEW_MAIL),
    (
        "OP_S2C_ANNOUNCE_NEW_MAIL",
        "PROTOCOL_ANNOUNCENEWMAIL",
        OP_S2C_ANNOUNCE_NEW_MAIL,
    ),
    ("OP_C2S_ACREPORT", "PROTOCOL_ACREPORT", OP_C2S_ACREPORT),
];

// ---------------------------------------------------------------------------
// Sem correspondência no IR — dívida herdada, mantida à vista
// ---------------------------------------------------------------------------

/// Opcodes que **não correspondem a nenhum protocolo** do IR do 1.5.3.
///
/// Vieram da fase em que o `gateway.rs` encenava o mundo à mão, e o `codec.rs` ainda
/// depende deles, então apagá-los exigiria reescrever aqueles caminhos agora. Ficam
/// aqui, separados e nomeados, em vez de misturados aos conferidos — e o teste
/// `os_sem_correspondencia_sao_exatamente_os_conhecidos` impede que a lista cresça sem
/// que alguém note.
///
/// **Cinco deles são pior que inexistentes: o valor pertence a outro protocolo de
/// verdade.** Enviar a resposta de `EnterWorld` no opcode 69 é enviar um `PlayerLogout`.
///
/// Para a maior parte, a saída não é achar "o opcode certo", e sim parar de tratá-los
/// como protocolo GNET: **movimento, skills, spawn e status do mundo 3D viajam dentro do
/// `GamedataSend` (34) como subcomandos**, catalogados em
/// `specs/protocol/gamedata_153.json`. Isso é trabalho do desmonte do `gateway.rs`.
pub mod nao_no_ir {
    /// Não existe `EnterWorld_Re` no IR; **69 é `PlayerLogout`**.
    pub const OP_S2C_ENTER_WORLD: u32 = 69;
    /// **217 é `GetSavedMsg`**.
    pub const OP_C2S_GET_WAIT_DEL_ROLES: u32 = 217;
    /// **218 é `GetSavedMsg_Re`**.
    pub const OP_S2C_GET_WAIT_DEL_ROLES_RE: u32 = 218;
    /// Não há protocolo de hora do servidor; **850 é `BattleGetMap`**.
    pub const OP_C2S_QUERY_SERVER_TIME: u32 = 850;
    /// **851 é `BattleGetMap_Re`**.
    pub const OP_S2C_QUERY_SERVER_TIME_RE: u32 = 851;
    /// Nenhum protocolo tem id 33. Movimento é o subcomando `OBJECT_MOVE` (15) dentro
    /// do `GamedataSend`.
    pub const OP_S2C_PLAYER_MOVE_BROADCAST: u32 = 33;

    /// A lista completa, para o teste conferir que não apareceu entrada nova.
    pub const SEM_CORRESPONDENCIA: &[(&str, u32)] = &[
        ("OP_S2C_ENTER_WORLD", OP_S2C_ENTER_WORLD),
        ("OP_C2S_GET_WAIT_DEL_ROLES", OP_C2S_GET_WAIT_DEL_ROLES),
        ("OP_S2C_GET_WAIT_DEL_ROLES_RE", OP_S2C_GET_WAIT_DEL_ROLES_RE),
        ("OP_C2S_QUERY_SERVER_TIME", OP_C2S_QUERY_SERVER_TIME),
        ("OP_S2C_QUERY_SERVER_TIME_RE", OP_S2C_QUERY_SERVER_TIME_RE),
        ("OP_S2C_PLAYER_MOVE_BROADCAST", OP_S2C_PLAYER_MOVE_BROADCAST),
    ];
}

// ---------------------------------------------------------------------------
// Subcomandos do GamedataSend
// ---------------------------------------------------------------------------

/// Subcomandos S2C transportados pelo `GamedataSend` (34).
///
/// Estes **não** são opcodes GNET: são os comandos do mundo 3D, com outro formato de fio
/// (little-endian, `pack(1)`). O catálogo completo, com campos e deslocamentos, está em
/// `specs/protocol/gamedata_153.json`; as constantes abaixo são as que o código usa
/// hoje, e o teste as confere contra aquele IR.
pub mod gamedata_s2c {
    pub const PLAYER_INFO_1: u16 = 0;
    pub const SELF_INFO_1: u16 = 8;
    pub const NPC_ENTER_SLICE: u16 = 11;
    pub const PLAYER_ENTER_SLICE: u16 = 12;
    pub const OBJECT_LEAVE_SLICE: u16 = 13;
    pub const NOTIFY_HOSTPOS: u16 = 14;
    pub const OBJECT_MOVE: u16 = 15;
    pub const PLAYER_ENTER_WORLD: u16 = 17;
    pub const PLAYER_LEAVE_WORLD: u16 = 19;
    pub const RECEIVE_EXP: u16 = 36;
    pub const LEVEL_UP: u16 = 37;
    pub const SELF_INFO_00: u16 = 38;

    /// Ligação constante → nome do comando no IR, conferida pelo teste.
    pub const CONFERIDOS: &[(&str, u16)] = &[
        ("PLAYER_INFO_1", PLAYER_INFO_1),
        ("SELF_INFO_1", SELF_INFO_1),
        ("NPC_ENTER_SLICE", NPC_ENTER_SLICE),
        ("PLAYER_ENTER_SLICE", PLAYER_ENTER_SLICE),
        ("OBJECT_LEAVE_SLICE", OBJECT_LEAVE_SLICE),
        ("NOTIFY_HOSTPOS", NOTIFY_HOSTPOS),
        ("OBJECT_MOVE", OBJECT_MOVE),
        ("PLAYER_ENTER_WORLD", PLAYER_ENTER_WORLD),
        ("PLAYER_LEAVE_WORLD", PLAYER_LEAVE_WORLD),
        ("RECEIVE_EXP", RECEIVE_EXP),
        ("LEVEL_UP", LEVEL_UP),
        ("SELF_INFO_00", SELF_INFO_00),
    ];
}

// Nomes antigos reexportados, para não quebrar o `codec.rs` enquanto os caminhos que os
// usam não são reescritos.
pub use gamedata_s2c::{
    LEVEL_UP as CMD_S2C_LEVEL_UP, NOTIFY_HOSTPOS as CMD_S2C_NOTIFY_HOSTPOS,
    NPC_ENTER_SLICE as CMD_S2C_NPC_ENTER_SLICE, OBJECT_LEAVE_SLICE as CMD_S2C_OBJECT_LEAVE_SLICE,
    OBJECT_MOVE as CMD_S2C_OBJECT_MOVE, PLAYER_ENTER_SLICE as CMD_S2C_PLAYER_ENTER_SLICE,
    PLAYER_ENTER_WORLD as CMD_S2C_PLAYER_ENTER_WORLD, PLAYER_INFO_1 as CMD_S2C_PLAYER_INFO_1,
    PLAYER_LEAVE_WORLD as CMD_S2C_PLAYER_LEAVE_WORLD, RECEIVE_EXP as CMD_S2C_RECEIVE_EXP,
    SELF_INFO_00 as CMD_S2C_SELF_INFO_00, SELF_INFO_1 as CMD_S2C_SELF_INFO_1,
};
pub use nao_no_ir::{
    OP_C2S_GET_WAIT_DEL_ROLES, OP_C2S_QUERY_SERVER_TIME, OP_S2C_ENTER_WORLD,
    OP_S2C_GET_WAIT_DEL_ROLES_RE, OP_S2C_PLAYER_MOVE_BROADCAST, OP_S2C_QUERY_SERVER_TIME_RE,
};
