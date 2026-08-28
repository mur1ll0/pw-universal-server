use crate::octets::OctetsStream;
use pw_core::{CharacterSummary, RoleId, Vector3, WorldId};
use serde::{Deserialize, Serialize};

/// S2C: Desafio inicial de conexão enviado ao cliente com chave de sessão (Opcode 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CChallenge {
    pub nonce: Vec<u8>,
    pub server_version: u32,
    pub algo: i8,
    pub edition: Vec<u8>,
    pub exp_rate: u8,
}

impl S2CChallenge {
    pub fn new(nonce: Vec<u8>) -> Self {
        Self {
            nonce,
            server_version: 804,
            algo: 0,
            edition: Vec::new(),
            exp_rate: 1,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_octets(&self.nonce);
        stream.write_u32(self.server_version);
        stream.write_i8(self.algo);
        
        if version != "1.2.6" {
            stream.write_octets(&self.edition);
            stream.write_u8(self.exp_rate);
        }
    }
}

/// S2C: Troca de Chaves de Criptografia (Opcode 3 / KeyExchange)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CKeyExchange {
    pub nonce: Vec<u8>,
    pub blkickuser: i8,
}

impl S2CKeyExchange {
    pub fn new(nonce: Vec<u8>) -> Self {
        Self {
            nonce,
            blkickuser: 0,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_octets(&self.nonce);
        stream.write_i8(self.blkickuser);
    }
}

/// S2C: Anúncio de Login Online Aprovado (Opcode 4 / OnlineAnnounce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2COnlineAnnounce {
    pub userid: i32,
    pub localsid: u32,
    pub remain_time: i32,
    pub zoneid: i8,
    pub free_time_left: i32,
    pub free_time_end: i32,
    pub creatime: i32,
}

impl S2COnlineAnnounce {
    pub fn new(userid: i32, localsid: u32) -> Self {
        Self {
            userid,
            localsid,
            remain_time: 0,
            zoneid: 1,
            free_time_left: 0,
            free_time_end: 0,
            creatime: 0,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_i32(self.userid);
        stream.write_u32(self.localsid);
        stream.write_i32(self.remain_time);
        stream.write_i8(self.zoneid);
        stream.write_i32(self.free_time_left);
        stream.write_i32(self.free_time_end);
        stream.write_i32(self.creatime);

        if version != "1.2.6" {
            stream.write_i8(0); // referrer_flag
            stream.write_i8(0); // passwd_flag
            stream.write_i8(0); // usbbind
            stream.write_i8(0); // accountinfo_flag
        }
    }
}

/// S2C: Mensagem de Erro de Conexão ou Login (Opcode 5 / ErrorInfo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CErrorInfo {
    pub error_code: i32,
    pub info: String,
}

impl S2CErrorInfo {
    pub fn new(error_code: i32, info: &str) -> Self {
        Self {
            error_code,
            info: info.to_string(),
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.error_code);
        stream.write_string_utf8(&self.info);
    }
}

/// S2C: Anúncio de Status do Jogador para a GUI do Cliente (Opcode 6 / StatusAnnounce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CStatusAnnounce {
    pub userid: i32,
    pub localsid: u32,
    pub status: u8,
}

impl S2CStatusAnnounce {
    pub fn new(userid: i32, localsid: u32, status: u8) -> Self {
        Self {
            userid,
            localsid,
            status,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.userid);
        stream.write_u32(self.localsid);
        stream.write_u8(self.status);
    }
}

/// S2C: Lista de Personagens da Conta (Opcode 0x53 / RoleList_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CRoleListResponse {
    pub result: i32,
    pub handle: i32,
    pub userid: i32,
    pub localsid: u32,
    pub characters: Vec<CharacterSummary>,
}

impl S2CRoleListResponse {
    pub fn new(userid: i32, localsid: u32, characters: Vec<CharacterSummary>) -> Self {
        Self {
            result: 0,
            handle: -1,
            userid,
            localsid,
            characters,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.handle);
        stream.write_i32(self.userid);
        stream.write_u32(self.localsid);

        // Vetor de RoleInfo
        stream.write_compact_uint(self.characters.len() as u32);

        for c in &self.characters {
            stream.write_i32(c.id);
            stream.write_u8(c.gender as u8);
            stream.write_u8(c.race as u8);
            stream.write_u8(c.cls as u8);
            stream.write_i32(c.level);
            stream.write_i32(0); // level2
            stream.write_string_utf16le(&c.name);

            // Custom appearance (face/hair/body)
            let appearance_bytes = serde_json::to_vec(&c.custom_appearance).unwrap_or_default();
            stream.write_octets(&appearance_bytes);

            // Equipment (GRoleInventoryVector)
            stream.write_compact_uint(c.equipment.len() as u32);
            for item in &c.equipment {
                stream.write_u32(item.item_id);
                stream.write_i32(item.slot as i32);
                stream.write_i32(item.count as i32);
                stream.write_i32(item.max_count as i32);
                stream.write_octets(&[]); // data
                stream.write_i32(0);     // proctype
                stream.write_i32(0);     // expire_date
                stream.write_i32(0);     // guid1
                stream.write_i32(0);     // guid2
                stream.write_i32(0);     // mask
            }

            stream.write_i8(1); // status (1 = Active)
            stream.write_i32(0); // delete_time
            stream.write_i32(0); // create_time
            stream.write_i32(0); // lastlogin_time
            stream.write_f32(c.position.x);
            stream.write_f32(c.position.y);
            stream.write_f32(c.position.z);
            stream.write_i32(c.world_id);
            stream.write_octets(&[]); // custom_status
            stream.write_octets(&[]); // charactermode

            if version != "1.2.6" {
                stream.write_i32(0); // referrer_role
                stream.write_i32(0); // cash_add
                stream.write_octets(&[]); // reincarnation_data
                stream.write_octets(&[]); // realm_data
            }
        }
    }
}

/// S2C: Resposta da Criação de Personagem (Opcode 0x55 / CreateRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CCreateRoleResponse {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub character: Option<CharacterSummary>,
}

impl S2CCreateRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// S2C: Resposta da Exclusão de Personagem (Opcode 0x57 / DeleteRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CDeleteRoleResponse {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CDeleteRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// S2C: Resposta da Restauração de Personagem (Opcode 0x59 / UndoDeleteRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CUndoDeleteRoleResponse {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CUndoDeleteRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// S2C: Resposta da Seleção de Personagem (Opcode 0x47 / SelectRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSelectRoleResponse {
    pub result: i32,
    pub auth: Vec<u8>,
}

impl S2CSelectRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_compact_uint(self.auth.len() as u32);
        stream.write_octets(&self.auth);
    }
}

/// S2C: Confirmação de entrada no mundo de jogo (Opcode 0x45)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CEnterWorldResponse {
    pub role_id: RoleId,
    pub world_id: WorldId,
    pub position: Vector3,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub sp: i64,
    pub level: i32,
}

impl S2CEnterWorldResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.role_id);
        stream.write_i32(self.world_id);
        stream.write_f32(self.position.x);
        stream.write_f32(self.position.y);
        stream.write_f32(self.position.z);
        stream.write_i32(self.hp);
        stream.write_i32(self.max_hp);
        stream.write_i32(self.mp);
        stream.write_i32(self.max_mp);
        stream.write_i64(self.exp);
        stream.write_i64(self.sp);
        stream.write_i32(self.level);
    }
}

/// S2C: Transmissão de movimentação de um jogador para outros ao redor (Opcode 0x21)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CPlayerMoveBroadcast {
    pub role_id: RoleId,
    pub mode: u8,
    pub position: Vector3,
    pub target: Vector3,
    pub speed: f32,
    pub timestamp: u32,
}

impl S2CPlayerMoveBroadcast {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.role_id);
        stream.write_u8(self.mode);
        stream.write_f32(self.position.x);
        stream.write_f32(self.position.y);
        stream.write_f32(self.position.z);
        stream.write_f32(self.target.x);
        stream.write_f32(self.target.y);
        stream.write_f32(self.target.z);
        stream.write_f32(self.speed);
        stream.write_u32(self.timestamp);
    }
}

/// S2C: Transmissão de mensagem de Chat (Opcode 0x71)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CChatBroadcast {
    pub channel: u8,
    pub sender_id: RoleId,
    pub sender_name: String,
    pub message: String,
}

impl S2CChatBroadcast {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_u8(self.channel);
        stream.write_i32(self.sender_id);
        stream.write_string_utf16le(&self.sender_name);
        stream.write_string_utf16le(&self.message);
    }
}

/// S2C: Pacote de Dados de Jogo / Mundo 3D (Opcode 0x20 / PROTOCOL_GAMEDATASEND)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGamedataSend {
    pub data: Vec<u8>,
}

impl S2CGamedataSend {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Cria o comando SELF_INFO_1 (Comando 8) que cancela o timeout OT_ENTERGAME e spawna o jogador no mundo
    pub fn self_info_1(exp: i32, sp: i32, world_id: i32, pos: Vector3) -> Self {
        let mut stream = OctetsStream::new();
        // Header do comando (u16 little-endian = 8)
        stream.write_u16_le(crate::opcodes::CMD_S2C_SELF_INFO_1);

        // cmd_self_info_1 struct no 1.2.6 (34 bytes total)
        stream.write_i32_le(exp);      // int iExp (4B)
        stream.write_i32_le(sp);       // int iSP (4B)
        stream.write_i32_le(world_id); // int cid (4B)
        stream.write_f32_le(pos.x);    // A3DVECTOR3 pos (12B)
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        stream.write_u16_le(0);        // unsigned short crc_e (2B)
        stream.write_u16_le(0);        // unsigned short crc_c (2B)
        stream.write_u8(0);            // unsigned char dir (1B)
        stream.write_u8(0);            // unsigned char level2 (1B)
        stream.write_i32_le(0);        // int state (4B)

        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando INST_DATA_CHECKOUT (Comando 4) para sincronizar timestamps do gshop e instâncias
    pub fn inst_data_checkout(id_inst: i32, region_ts: u32, precinct_ts: u32, gshop_ts: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(4);              // CMD_S2C_INST_DATA_CHECKOUT = 4
        stream.write_i32_le(id_inst);        // int idInst (1 = mundo aberto)
        stream.write_u32_le(region_ts);      // unsigned int region_time_stamp
        stream.write_u32_le(precinct_ts);    // unsigned int precinct_time_stamp
        stream.write_u32_le(gshop_ts);       // unsigned int gshop_time_stamp (1206433535 / 0x47e8b6ff)
        stream.write_u32_le(0);              // unsigned int gshop_time_stamp2
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando PLAYER_EXT_PROP_MOVE (Comando 54) definindo as velocidades oficiais de movimento
    pub fn ext_prop_move(id_player: i32, walk_speed: f32, run_speed: f32, swim_speed: f32, flight_speed: f32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(54);            // CMD_S2C_PLAYER_EXT_PROP_MOVE = 54
        stream.write_i32_le(id_player);     // int idPlayer (4B)
        stream.write_f32_le(walk_speed);    // float walk_speed (4B) = 4.8
        stream.write_f32_le(run_speed);     // float run_speed (4B) = 4.8
        stream.write_f32_le(swim_speed);    // float swim_speed (4B) = 4.0
        stream.write_f32_le(flight_speed);  // float flight_speed (4B) = 5.0
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando PLAYER_EXT_PROP_BASE (Comando 53) definindo atributos vitais base
    pub fn ext_prop_base(id_player: i32, vitality: i32, energy: i32, strength: i32, agility: i32, max_hp: i32, max_mp: i32, hp_gen: i32, mp_gen: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(53);            // CMD_S2C_PLAYER_EXT_PROP_BASE = 53
        stream.write_i32_le(id_player);     // int idPlayer (4B)
        stream.write_i32_le(vitality);      // int vitality (4B)
        stream.write_i32_le(energy);        // int energy (4B)
        stream.write_i32_le(strength);      // int strength (4B)
        stream.write_i32_le(agility);       // int agility (4B)
        stream.write_i32_le(max_hp);        // int max_hp (4B)
        stream.write_i32_le(max_mp);        // int max_mp (4B)
        stream.write_i32_le(hp_gen);        // int hp_gen (4B)
        stream.write_i32_le(mp_gen);        // int mp_gen (4B)
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando TASK_DATA (Comando 105) inicializando o buffer de missões vazio oficial do 1.2.6 (3 listas)
    pub fn task_data() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(105);           // CMD_S2C_TASK_DATA = 105
        stream.write_u32_le(0);             // active_list_size = 0
        stream.write_u32_le(0);             // finished_list_size = 0
        stream.write_u32_le(0);             // finished_time_list_size = 0
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OWN_ITEM_INFO (Comando 40) enviando os atributos de durabilidade e requisitos do item
    pub fn item_info(by_package: u8, by_slot: u8, item_id: i32, cur_endurance: i32, max_endurance: i32, count: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(40);              // CMD_S2C_OWN_ITEM_INFO = 40
        stream.write_u8(by_package);          // byPackage
        stream.write_u8(by_slot);             // bySlot
        stream.write_i32_le(item_id);         // type (tid)
        stream.write_i32_le(0);               // expire_date
        stream.write_i32_le(0);               // state
        stream.write_u32_le(count);           // count
        stream.write_u16_le(0);               // crc

        // Content: atributos de equipamento (durabilidade, requisitos, essence de arma de 44B, slots)
        let mut content = OctetsStream::new();
        content.write_i16_le(1);              // m_iLevelReq = 1
        content.write_i16_le(-1);             // m_iProfReq = -1 (0xFFFF: Todas as classes sem restrição)
        content.write_i16_le(0);              // m_iStrengthReq = 0
        content.write_i16_le(0);              // m_iVitalityReq = 0
        content.write_i16_le(0);              // m_iAgilityReq = 0
        content.write_i16_le(0);              // m_iEnergyReq = 0
        content.write_i32_le(cur_endurance);  // m_iCurEndurance = 10000 (100.00 / 100.00)
        content.write_i32_le(max_endurance);  // m_iMaxEndurance = 10000
        content.write_i16_le(44);             // iEssenceSize = 44 (sizeof(IVTR_ESSENCE_WEAPON))
        content.write_u8(0);                  // m_byMadeFrom = 0
        content.write_u8(0);                  // iMakerLen = 0

        // IVTR_ESSENCE_WEAPON (44 bytes)
        content.write_i16_le(1);              // weapon_type = 1
        content.write_i16_le(0);              // weapon_delay = 0
        content.write_i32_le(1);              // weapon_class = 1
        content.write_i32_le(1);              // weapon_level = 1
        content.write_i32_le(0);              // require_projectile = 0
        content.write_i32_le(10);             // damage_low = 10
        content.write_i32_le(20);             // damage_high = 20
        content.write_i32_le(10);             // magic_damage_low = 10
        content.write_i32_le(20);             // magic_damage_high = 20
        content.write_i32_le(10);             // attack_speed = 10
        content.write_f32_le(3.5);            // attack_range = 3.5
        content.write_f32_le(0.0);            // attack_short_range = 0.0

        content.write_i16_le(0);              // iNumHole = 0
        content.write_u16_le(0);              // m_wStoneMask = 0
        content.write_i32_le(0);              // iNumProp = 0

        let content_bytes = content.into_bytes();
        stream.write_u16_le(content_bytes.len() as u16);
        stream.write_raw_bytes(&content_bytes);

        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando EXG_IVTR_ITEM (Comando 44)
    pub fn exg_ivtr_item(index1: u8, index2: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(44);
        stream.write_u8(index1);
        stream.write_u8(index2);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando MOVE_IVTR_ITEM (Comando 45)
    pub fn move_ivtr_item(src: u8, dest: u8, count: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(45);
        stream.write_u8(src);
        stream.write_u8(dest);
        stream.write_u32_le(count);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EXG_EQUIP_ITEM (Comando 47)
    pub fn exg_equip_item(index1: u8, index2: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(47);
        stream.write_u8(index1);
        stream.write_u8(index2);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EQUIP_ITEM (Comando 48)
    pub fn equip_item(idx_ivtr: u8, idx_equip: u8, count_ivtr: u32, count_equip: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(48);
        stream.write_u8(idx_ivtr);
        stream.write_u8(idx_equip);
        stream.write_u32_le(count_ivtr);
        stream.write_u32_le(count_equip);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando MOVE_ITEM_TO_EQUIP (Comando 49)
    pub fn move_item_to_equip(idx_ivtr: u8, idx_eq: u8, amount: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(49);
        stream.write_u8(idx_ivtr);
        stream.write_u8(idx_eq);
        stream.write_u32_le(amount);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando UNFREEZE_IVTR_SLOT (Comando 181) para destravar o slot após mover/trocar itens
    pub fn unfreeze_ivtr_slot(where_pack: u8, index: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(181);
        stream.write_u8(where_pack);
        stream.write_u16_le(index);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando HOST_USE_ITEM (Comando 91)
    pub fn host_use_item(by_package: u8, by_slot: u8, item_id: i32, count: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(91);
        stream.write_u8(by_package);
        stream.write_u8(by_slot);
        stream.write_i32_le(item_id);
        stream.write_u16_le(count);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_INFO_LIST (Comando 9) para instanciar NPCs e monstros visíveis no mundo
    pub fn npc_info_list(npcs: &[(i32, i32, (f32, f32, f32))]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(9);                 // CMD_S2C_NPC_INFO_LIST = 9
        stream.write_u16_le(npcs.len() as u16); // count
        for &(nid, tid, (x, y, z)) in npcs {
            stream.write_i32_le(nid);           // nid
            stream.write_i32_le(tid);           // tid (template id)
            stream.write_i32_le(tid);           // vis_tid
            stream.write_f32_le(x);             // pos.x
            stream.write_f32_le(y);             // pos.y
            stream.write_f32_le(z);             // pos.z
            stream.write_u16_le(0);             // seed
            stream.write_u8(0);                 // dir
            stream.write_i32_le(0);             // state
            stream.write_i32_le(0);             // state2
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_SIT_DOWN (Comando 111) para sentar / meditar
    pub fn object_sit_down(id_player: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(111);           // CMD_S2C_OBJECT_SIT_DOWN = 111
        stream.write_i32_le(id_player);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OBJECT_STAND_UP (Comando 112) para levantar da meditação
    pub fn object_stand_up(id_player: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(112);           // CMD_S2C_OBJECT_STAND_UP = 112
        stream.write_i32_le(id_player);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OBJECT_DO_EMOTE (Comando 113) para executar animações de emote
    pub fn object_do_emote(id_player: i32, emotion: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(113);           // CMD_S2C_OBJECT_DO_EMOTE = 113
        stream.write_i32_le(id_player);
        stream.write_u16_le(emotion);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando SKILL_DATA (Comando 90) para carregar a lista de habilidades do jogador
    pub fn skill_data(skills: &[(i16, u8, i16)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(90);             // CMD_S2C_SKILL_DATA = 90
        stream.write_u32_le(skills.len() as u32); // size_t skill_count
        for &(id_skill, level, ability) in skills {
            stream.write_i16_le(id_skill);   // short id_skill (2B)
            stream.write_u8(level);          // unsigned char level (1B)
            stream.write_i16_le(ability);     // short ability (2B)
        }
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OWN_IVTR_DATA (Comando 42) com inventário inicial (bolsa principal = 0, tamanho = 32 slots)
    pub fn own_ivtr_data(bag_size: u8, weapon_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(42);       // CMD_S2C_OWN_IVTR_DATA = 42
        stream.write_u8(0);            // byPackage = 0 (IVTRTYPE_PACK)
        stream.write_u8(bag_size);     // ivtr_size = 32 slots
        
        // Slot 0: Arma Inicial da Classe (qtd: 1) -> 12 bytes
        // Slot 1: Pergaminho de Retorno (ID 2100, qtd: 5) -> 12 bytes
        // Slot 2: Poção de Vida Pequena (ID 1796, qtd: 10) -> 12 bytes
        // Slot 3: Poção de Mana Pequena (ID 1801, qtd: 10) -> 12 bytes
        // Slots 4..31 (28 slots vazios x 4 bytes = 112 bytes)
        let content_len = 12 + 12 + 12 + 12 + ((bag_size - 4) as u32) * 4;
        stream.write_u32_le(content_len);

        // Slot 0: Arma Inicial da Classe
        stream.write_i32_le(weapon_id);
        stream.write_i32_le(0);
        stream.write_i32_le(1);

        // Slot 1: 5x Pergaminho de Retorno
        stream.write_i32_le(2100);
        stream.write_i32_le(0);
        stream.write_i32_le(5);

        // Slot 2: 10x Poção de Vida
        stream.write_i32_le(1796);
        stream.write_i32_le(0);
        stream.write_i32_le(10);

        // Slot 3: 10x Poção de Mana
        stream.write_i32_le(1801);
        stream.write_i32_le(0);
        stream.write_i32_le(10);

        for _ in 4..bag_size {
            stream.write_i32_le(-1);   // -1 = slot vazio (tidItem < 0)
        }
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OWN_IVTR_DATA (Comando 42) para os slots de equipamentos (byPackage = 1)
    pub fn own_equip_data() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(42);       // CMD_S2C_OWN_IVTR_DATA = 42 (Equipment usa byPackage = 1)
        stream.write_u8(1);            // byPackage = 1 (IVTRTYPE_EQUIPPACK)
        stream.write_u8(32);           // ivtr_size = 32 slots

        let content_len = 32 * 4;
        stream.write_u32_le(content_len);
        for _ in 0..32 {
            stream.write_i32_le(-1);   // -1 = slot vazio
        }
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando PLAYER_ENTER_WORLD (Comando 17) para instanciar o avatar
    pub fn player_enter_world(role_id: RoleId, world_tag: i32, pos: Vector3) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_PLAYER_ENTER_WORLD);
        stream.write_i32_le(role_id);
        stream.write_i32_le(world_tag);
        stream.write_f32_le(pos.x);
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando SELF_INFO_00 (Comando 38) para sincronizar status de vida, mana, nível e atributos
    pub fn self_info_00(
        level: i16,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        exp: i32,
        sp: i32,
    ) -> Self {
        let mut stream = OctetsStream::new();
        // Header do comando (u16 little-endian = 38)
        stream.write_u16_le(crate::opcodes::CMD_S2C_SELF_INFO_00);

        // struct cmd_self_info_00 (36 bytes)
        stream.write_i16_le(level);    // short sLevel (2B)
        stream.write_u8(0);            // unsigned char State (1B)
        stream.write_u8(0);            // unsigned char Level2 (1B)
        stream.write_i32_le(hp);       // int iHP (4B)
        stream.write_i32_le(max_hp);   // int iMaxHP (4B)
        stream.write_i32_le(mp);       // int iMP (4B)
        stream.write_i32_le(max_mp);   // int iMaxMP (4B)
        stream.write_i32_le(exp);      // int iExp (4B)
        stream.write_i32_le(sp);       // int iSP (4B)
        stream.write_i32_le(0);        // int iAP (4B)
        stream.write_i32_le(0);        // int iMaxAP (4B)

        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_octets(&self.data);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetUIConfigRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub ui_config: Vec<u8>,
}

impl S2CGetUIConfigRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_octets(&self.ui_config);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetFriendListRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CGetFriendListRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_compact_uint(0); // lista de amigos vazia
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetWaitDelRolesRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CGetWaitDelRolesRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_compact_uint(0); // lista vazia
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CServerTimeRe {
    pub timestamp: i32,
    pub time_zone: i32,
}

impl S2CServerTimeRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.timestamp);
        stream.write_i32(self.time_zone);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetHelpStatesRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub help_states: Vec<u8>,
}

impl S2CGetHelpStatesRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_octets(&self.help_states);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSetHelpStatesRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CSetHelpStatesRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSetUIConfigRe {
    pub result: i32,
    pub role_id: i32,
}

impl S2CSetUIConfigRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSetCustomDataRe {
    pub result: i32,
    pub role_id: i32,
}

impl S2CSetCustomDataRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
    }
}

/// Pacote oficial S2C de Logout (Opcode 69 / 0x45 - PROTOCOL_PLAYERLOGOUT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CPlayerLogout {
    pub result: i32,
    pub role_id: i32,
    pub provider_link_id: i32,
    pub localsid: u32,
}

impl S2CPlayerLogout {
    pub fn new(result: i32, role_id: i32, localsid: u32) -> Self {
        Self {
            result,
            role_id,
            provider_link_id: -1,
            localsid,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_i32(self.provider_link_id);
        stream.write_u32(self.localsid);
    }
}



