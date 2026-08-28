/// Opcodes oficiais dos pacotes C2S e S2C do Perfect World (Wanmei Engine)

pub const OP_S2C_CHALLENGE: u32 = 1;
pub const OP_C2S_RESPONSE: u32 = 2;
pub const OP_S2C_KEYEXCHANGE: u32 = 3;
pub const OP_C2S_KEYEXCHANGE: u32 = 3;
pub const OP_S2C_ONLINEANNOUNCE: u32 = 4;
pub const OP_S2C_ERRORINFO: u32 = 5;
pub const OP_S2C_STATUSANNOUNCE: u32 = 6;
pub const OP_S2C_ROLESTATUSANNOUNCE: u32 = 7;

pub const OP_C2S_SELECT_ROLE: u32 = 0x46;       // 70
pub const OP_S2C_SELECT_ROLE_RE: u32 = 0x47;    // 71

pub const OP_C2S_ROLE_LIST: u32 = 0x52;         // 82
pub const OP_S2C_ROLE_LIST_RES: u32 = 0x53;     // 83

pub const OP_C2S_CREATE_ROLE: u32 = 0x54;       // 84
pub const OP_S2C_CREATE_ROLE_RES: u32 = 0x55;   // 85

pub const OP_C2S_DELETE_ROLE: u32 = 0x56;       // 86
pub const OP_S2C_DELETE_ROLE_RES: u32 = 0x57;   // 87

pub const OP_C2S_UNDO_DELETE_ROLE: u32 = 0x58;  // 88
pub const OP_S2C_UNDO_DELETE_ROLE_RES: u32 = 0x59; // 89

pub const OP_C2S_ENTER_WORLD: u32 = 0x48;       // 72
pub const OP_S2C_ENTER_WORLD: u32 = 0x45;       // 69

pub const OP_C2S_GAMEDATASEND: u32 = 0x20;       // 32 (GamedataSend C2S)
pub const OP_S2C_GAMEDATASEND: u32 = 0x22;       // 34 (GamedataSend S2C)
pub const OP_S2C_PLAYER_MOVE_BROADCAST: u32 = 0x21;

// Subcomandos internos transportados pelo GamedataSend (Opcode 0x22 S2C / 0x20 C2S)
pub const CMD_S2C_PLAYER_INFO_1: u16 = 0;
pub const CMD_S2C_SELF_INFO_1: u16 = 8;
pub const CMD_S2C_NPC_ENTER_SLICE: u16 = 11;
pub const CMD_S2C_PLAYER_ENTER_SLICE: u16 = 12;
pub const CMD_S2C_OBJECT_LEAVE_SLICE: u16 = 13;
pub const CMD_S2C_NOTIFY_HOSTPOS: u16 = 14;
pub const CMD_S2C_OBJECT_MOVE: u16 = 15;
pub const CMD_S2C_PLAYER_ENTER_WORLD: u16 = 17;
pub const CMD_S2C_PLAYER_LEAVE_WORLD: u16 = 19;
pub const CMD_S2C_RECEIVE_EXP: u16 = 36;
pub const CMD_S2C_LEVEL_UP: u16 = 37;
pub const CMD_S2C_SELF_INFO_00: u16 = 38;

pub const OP_C2S_USE_SKILL: u32 = 0x29;         // 41
pub const OP_S2C_SKILL_CAST_BROADCAST: u32 = 0x2A;

pub const OP_C2S_CHAT: u32 = 0x70;            // 112
pub const OP_S2C_CHAT_BROADCAST: u32 = 0x71;

pub const OP_C2S_HEARTBEAT: u32 = 0x5A;       // 90
pub const OP_S2C_HEARTBEAT_ACK: u32 = 0x5B;

pub const OP_S2C_SPAWN_PLAYER: u32 = 0x0A;    // 10
pub const OP_S2C_DESPAWN_PLAYER: u32 = 0x0B;

pub const OP_S2C_UPDATE_STATUS: u32 = 0x14;   // 20

pub const OP_S2C_PLAYER_LOGOUT: u32 = 0x45;   // 69 (PROTOCOL_PLAYERLOGOUT)

pub const OP_C2S_SET_CUSTOM_DATA: u32 = 0x66;     // 102 (PROTOCOL_SETCUSTOMDATA)
pub const OP_S2C_SET_CUSTOM_DATA_RE: u32 = 0x67;  // 103 (PROTOCOL_SETCUSTOMDATA_RE)

pub const OP_C2S_GET_UI_CONFIG: u32 = 0x68;       // 104
pub const OP_S2C_GET_UI_CONFIG_RE: u32 = 0x69;    // 105
pub const OP_C2S_SET_UI_CONFIG: u32 = 0x6A;       // 106
pub const OP_S2C_SET_UI_CONFIG_RE: u32 = 0x6B;    // 107

pub const OP_C2S_GET_FRIEND_LIST: u32 = 0xCE;     // 206
pub const OP_S2C_GET_FRIEND_LIST_RE: u32 = 0xCF;  // 207

pub const OP_C2S_GET_WAIT_DEL_ROLES: u32 = 0xD9;   // 217
pub const OP_S2C_GET_WAIT_DEL_ROLES_RE: u32 = 0xDA;// 218

pub const OP_C2S_QUERY_SERVER_TIME: u32 = 0x352;   // 850
pub const OP_S2C_QUERY_SERVER_TIME_RE: u32 = 0x353;// 851

pub const OP_C2S_SET_HELP_STATES: u32 = 0x80;      // 128 (PROTOCOL_SETHELPSTATES)
pub const OP_S2C_SET_HELP_STATES_RE: u32 = 0x81;   // 129 (PROTOCOL_SETHELPSTATES_RE)

pub const OP_C2S_GET_HELP_STATES: u32 = 0x82;      // 130 (PROTOCOL_GETHELPSTATES)
pub const OP_S2C_GET_HELP_STATES_RE: u32 = 0x83;   // 131 (PROTOCOL_GETHELPSTATES_RE)

pub const OP_C2S_ACREPORT: u32 = 0x1389;           // 5001 (PROTOCOL_ACREPORT / Anti-Cheat Report)



