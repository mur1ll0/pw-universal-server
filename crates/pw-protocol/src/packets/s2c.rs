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
            let appearance_bytes = if let Some(raw_hex) = c.custom_appearance.get("raw").and_then(|v| v.as_str()) {
                hex::decode(raw_hex).unwrap_or_default()
            } else {
                serde_json::to_vec(&c.custom_appearance).unwrap_or_default()
            };
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

    /// Cria o comando SERVER_CONFIG_DATA / INST_DATA_CHECKOUT (Comando 206) para sincronizar timestamps do gshop e instâncias
    pub fn inst_data_checkout(id_inst: i32, region_ts: u32, precinct_ts: u32, gshop_ts: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(206);             // CMD_S2C_SERVER_CONFIG_DATA = 206
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

    /// Cria o comando TASK_DATA (Comando 105) inicializando as listas de tarefas oficiais
    pub fn task_data() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(105);             // CMD_S2C_TASK_DATA = 105
        stream.write_u32_le(0);               // len1 = 0
        stream.write_u32_le(0);               // len2 = 0
        stream.write_u32_le(0);               // len3 = 0
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TASK_VAR_DATA (Comando 106) enviando resposta para o sistema de tarefas dinâmicas do cliente
    pub fn task_var_data(data: &[u8]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(106);              // CMD_S2C_TASK_VAR_DATA = 106
        stream.write_u32_le(data.len() as u32);
        stream.write_raw_bytes(data);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria notificação de nova missão aceita/entregue ao jogador (TASK_SVR_NOTIFY_NEW = 1)
    pub fn task_notify_new(task_id: u16, timestamp: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(1);                    // reason = TASK_SVR_NOTIFY_NEW (1)
        stream.write_u16_le(task_id);          // task ID (2B)
        stream.write_u32_le(timestamp);        // cur_time (4B)
        stream.write_u32_le(0);                // cap_task = 0 (4B)
        stream.write_u16_le(0);                // sub_task = 0 (2B) - 0 indicates root task in v1.2.6
        stream.write_u8(0);                    // sz = 0 (1B)
        Self::task_var_data(&stream.into_bytes())
    }

    /// Cria notificação de missão concluída com sucesso (TASK_SVR_NOTIFY_COMPLETE = 2)
    pub fn task_notify_complete(task_id: u16, timestamp: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(2);                    // reason = TASK_SVR_NOTIFY_COMPLETE (2)
        stream.write_u16_le(task_id);          // task ID (2B)
        stream.write_u32_le(timestamp);        // cur_time (4B)
        stream.write_u16_le(0);                // sub_task = 0 (2B)
        stream.write_u8(0);                    // sz = 0 (1B)
        Self::task_var_data(&stream.into_bytes())
    }

    /// Cria notificação de monstro abatido para progresso de missão (TASK_SVR_NOTIFY_MONSTER_KILLED = 4)
    pub fn task_notify_monster_killed(task_id: u16, monster_id: u32, monster_num: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(4);                    // reason = TASK_SVR_NOTIFY_MONSTER_KILLED (4)
        stream.write_u16_le(task_id);          // task ID (2B)
        stream.write_u32_le(monster_id);       // monster_id (4B)
        stream.write_u16_le(monster_num);      // monster_num (2B) - Exactly 9 bytes total (1+2+4+2)
        Self::task_var_data(&stream.into_bytes())
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

        // Se for arma (ou equipamento com durabilidade), constrói o bloco de essence
        let is_weapon = matches!(item_id, 2097 | 2867 | 2258 | 2250 | 4508 | 4532 | 4567 | 4616);
        let is_equip_package = by_package == 1;

        if is_weapon || is_equip_package {
            let mut content = OctetsStream::new();
            content.write_i16_le(1);              // m_iLevelReq = 1
            content.write_i16_le(-1);             // m_iProfReq = -1 (Todas as classes sem restrição)

            let (req_str, req_agi, req_vit, req_eng, w_type, w_class, req_proj, dmg_l, dmg_h, mdmg_l, mdmg_h, spd, rng) = match item_id {
                2097 => (5, 5, 0, 0, 1, 1, 0, 3, 5, 0, 0, 16, 3.0f32),       // Espada de Madeira (Guerreiro)
                2867 => (5, 3, 0, 0, 5, 5, 0, 2, 3, 10, 15, 12, 3.0f32),     // Graveto de Madeira / Varinha Mágica (Mago / Feiticeira / Sacerdote)
                2258 => (5, 5, 0, 0, 9, 9, 0, 4, 8, 0, 0, 14, 3.5f32),       // Porrete com Espinhos (Bárbaro)
                2250 => (5, 5, 0, 0, 13, 13, 1, 5, 10, 0, 0, 15, 20.0f32),   // Arco de Madeira (Arqueiro)
                _ => (0, 0, 0, 0, 1, 1, 0, 3, 5, 0, 0, 10, 3.5f32),
            };

            content.write_i16_le(req_str);        // m_iStrengthReq
            content.write_i16_le(req_vit);        // m_iVitalityReq
            content.write_i16_le(req_agi);        // m_iAgilityReq
            content.write_i16_le(req_eng);        // m_iEnergyReq
            content.write_i32_le(cur_endurance);  // m_iCurEndurance (2800 = 28.00)
            content.write_i32_le(max_endurance);  // m_iMaxEndurance (2800 = 28.00)
            content.write_i16_le(44);             // iEssenceSize = 44 (sizeof(IVTR_ESSENCE_WEAPON))
            content.write_u8(0);                  // m_byMadeFrom = 0
            content.write_u8(0);                  // iMakerLen = 0

            // IVTR_ESSENCE_WEAPON (44 bytes)
            content.write_i16_le(w_type);         // weapon_type
            content.write_i16_le(0);              // weapon_delay = 0
            content.write_i32_le(w_class);        // weapon_class
            content.write_i32_le(1);              // weapon_level = 1
            content.write_i32_le(req_proj);       // require_projectile (1 para arco)
            content.write_i32_le(dmg_l);          // damage_low
            content.write_i32_le(dmg_h);          // damage_high
            content.write_i32_le(mdmg_l);         // magic_damage_low
            content.write_i32_le(mdmg_h);         // magic_damage_high
            content.write_i32_le(spd);            // attack_speed
            content.write_f32_le(rng);            // attack_range
            content.write_f32_le(0.0);            // attack_short_range

            content.write_i16_le(0);              // iNumHole = 0
            content.write_u16_le(0);              // m_wStoneMask = 0
            content.write_i32_le(0);              // iNumProp = 0

            let content_bytes = content.into_bytes();
            stream.write_u16_le(content_bytes.len() as u16);
            stream.write_raw_bytes(&content_bytes);
        } else {
            stream.write_u16_le(0);               // Sem essence para consumíveis (poções, pergaminhos)
        }

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

    /// Cria o comando MOVE_IVTR_ITEM (Comando 45) com struct oficial { u8 src, u8 dest, u16 count }
    pub fn move_ivtr_item(src: u8, dest: u8, count: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(45);
        stream.write_u8(src);
        stream.write_u8(dest);
        stream.write_u16_le(count);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EXG_EQUIP_ITEM (Comando 47) com struct oficial { u8 index1, u8 index2 }
    pub fn exg_equip_item(index1: u8, index2: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(47);
        stream.write_u8(index1);
        stream.write_u8(index2);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EQUIP_ITEM (Comando 48) com struct oficial { u8 idx_ivtr, u8 idx_equip, u16 count_ivtr, u16 count_equip }
    pub fn equip_item(idx_ivtr: u8, idx_equip: u8, count_ivtr: u16, count_equip: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(48);
        stream.write_u8(idx_ivtr);
        stream.write_u8(idx_equip);
        stream.write_u16_le(count_ivtr);
        stream.write_u16_le(count_equip);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando MOVE_ITEM_TO_EQUIP (Comando 49) com struct oficial { u8 idx_ivtr, u8 idx_eq, u16 amount }
    pub fn move_item_to_equip(idx_ivtr: u8, idx_eq: u8, amount: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(49);
        stream.write_u8(idx_ivtr);
        stream.write_u8(idx_eq);
        stream.write_u16_le(amount);
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

    /// Cria o comando SKILL_DATA (Comando 90) a partir de registros do banco de dados (LearnedSkill)
    pub fn skill_data_from_records(skills: &[pw_core::LearnedSkill]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(90);             // CMD_S2C_SKILL_DATA = 90
        stream.write_u32_le(skills.len() as u32);
        for skill in skills {
            stream.write_i16_le(skill.skill_id as i16);
            stream.write_u8(skill.level);
            stream.write_i16_le(0);          // ability
        }
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OWN_IVTR_DATA (Comando 42) dinamicamente a partir dos itens do banco de dados
    pub fn own_ivtr_from_items(by_package: u8, bag_size: u8, items: &[pw_core::ItemRecord]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(42);             // CMD_S2C_OWN_IVTR_DATA = 42
        stream.write_u8(by_package);
        stream.write_u8(bag_size);

        let mut slot_map = std::collections::HashMap::new();
        for item in items {
            slot_map.insert(item.slot as usize, item);
        }

        let mut content = OctetsStream::new();
        for s in 0..bag_size as usize {
            if let Some(item) = slot_map.get(&s) {
                content.write_i32_le(item.item_id as i32);
                content.write_i32_le(0);     // expire_date
                content.write_i32_le(item.count as i32);
            } else {
                content.write_i32_le(-1);    // -1 = slot vazio
            }
        }

        let content_bytes = content.into_bytes();
        stream.write_u32_le(content_bytes.len() as u32);
        stream.write_raw_bytes(&content_bytes);

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

    /// Cria o comando NPC_ENTER_SLICE (Comando 11) com a struct oficial exata de 27 bytes (desmontada do gs v1.2.6)
    pub fn npc_enter_slice(nid: i32, tid: i32, pos: Vector3, dir: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(11);               // CMD_S2C_NPC_ENTER_SLICE = 11 (2B)
        stream.write_i32_le(nid);              // nid (4B)
        stream.write_i32_le(tid);              // tid (4B)
        stream.write_f32_le(pos.x);            // pos.x (4B)
        stream.write_f32_le(pos.y);            // pos.y (4B)
        stream.write_f32_le(pos.z);            // pos.z (4B)
        stream.write_u16_le(0);                // seed (2B)
        stream.write_u8(dir);                  // dir (1B)
        stream.write_u32_le(0);                // state (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_ENTER_WORLD (Comando 16) com a struct oficial exata de 27 bytes (desmontada do gs v1.2.6)
    pub fn npc_enter_world(nid: i32, tid: i32, pos: Vector3, dir: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(16);               // CMD_S2C_NPC_ENTER_WORLD = 16 (2B)
        stream.write_i32_le(nid);              // nid (4B)
        stream.write_i32_le(tid);              // tid (4B)
        stream.write_f32_le(pos.x);            // pos.x (4B)
        stream.write_f32_le(pos.y);            // pos.y (4B)
        stream.write_f32_le(pos.z);            // pos.z (4B)
        stream.write_u16_le(0);                // seed (2B)
        stream.write_u8(dir);                  // dir (1B)
        stream.write_u32_le(0);                // state (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_INFO_00 (Comando 33) enviando HP, MaxHP e Alvo do NPC/Monstro
    pub fn npc_info_00(nid: i32, hp: i32, max_hp: i32, target_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(33);               // CMD_S2C_NPC_INFO_00 = 33
        stream.write_i32_le(nid);
        stream.write_i32_le(hp);
        stream.write_i32_le(max_hp);
        stream.write_i32_le(target_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando UNSELECT (Comando 39) desmarcando o alvo atual
    pub fn unselect() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(39);               // CMD_S2C_UNSELECT = 39
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_CAST_SKILL (Comando 85) disparando a animação e barra de conjuração da magia
    pub fn object_cast_skill(caster: i32, target: i32, skill_id: i32, cast_time_ms: u16, skill_level: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(85);               // CMD_S2C_OBJECT_CAST_SKILL = 85
        stream.write_i32_le(caster);
        stream.write_i32_le(target);
        stream.write_i32_le(skill_id);
        stream.write_u16_le(cast_time_ms);
        stream.write_u8(skill_level);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SKILL_PERFORM (Comando 88) liberando o jogador do estado de conjuração
    pub fn skill_perform() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(88);               // CMD_S2C_SKILL_PERFORM = 88
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SELF_SKILL_ATTACK_RESULT (Comando 142) aplicando o dano da habilidade do jogador
    pub fn self_skill_attack_result(target_id: i32, skill_id: i32, damage: i32, attack_flag: i32, speed: u8, section: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(142);              // CMD_S2C_SELF_SKILL_ATTACK_RESULT = 142
        stream.write_i32_le(target_id);
        stream.write_i32_le(skill_id);
        stream.write_i32_le(damage);
        stream.write_i32_le(attack_flag);
        stream.write_u8(speed);
        stream.write_u8(section);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_SKILL_ATTACK_RESULT (Comando 143) aplicando o dano de habilidade entre entidades
    pub fn object_skill_attack_result(attacker_id: i32, target_id: i32, skill_id: i32, damage: i32, attack_flag: i32, speed: u8, section: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(143);              // CMD_S2C_OBJECT_SKILL_ATTACK_RESULT = 143
        stream.write_i32_le(attacker_id);
        stream.write_i32_le(target_id);
        stream.write_i32_le(skill_id);
        stream.write_i32_le(damage);
        stream.write_i32_le(attack_flag);
        stream.write_u8(speed);
        stream.write_u8(section);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SELECT_TARGET (Comando 52)
    pub fn select_target(target_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(52);               // CMD_S2C_SELECT_TARGET = 52
        stream.write_i32_le(target_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_GREETING (Comando 70) para abrir diálogo com NPC
    pub fn npc_greeting(nid: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(70);               // CMD_S2C_NPC_GREETING = 70
        stream.write_i32_le(nid);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_DISAPPEAR (Comando 21) para remover objeto que saiu de vista ou morreu
    pub fn object_disappear(id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(21);               // CMD_S2C_OBJECT_DISAPPEAR = 21
        stream.write_i32_le(id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando HOST_ATTACKRESULT (Comando 24) retornando dano infligido
    pub fn host_attack_result(target_id: i32, damage: i32, hit_type: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(24);               // CMD_S2C_HOST_ATTACKRESULT = 24
        stream.write_i32_le(target_id);        // target_id (4B)
        stream.write_i32_le(damage);           // damage (4B)
        stream.write_u8(hit_type);             // 0=Normal, 1=Crítico, 2=Esquiva
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_DIED (Comando 20) informando a morte do monstro
    pub fn npc_died(nid: i32, killer_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(20);               // CMD_S2C_NPC_DIED = 20
        stream.write_i32_le(nid);              // nid (4B)
        stream.write_i32_le(killer_id);        // killer_id (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando RECEIVE_EXP (Comando 36) entregando EXP e Alma
    pub fn receive_exp(exp: i32, sp: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(36);               // CMD_S2C_RECEIVE_EXP = 36
        stream.write_i32_le(exp);              // exp (4B)
        stream.write_i32_le(sp);               // sp (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando LEVEL_UP (Comando 37) tocando a animação de subir de nível
    pub fn level_up(role_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(37);               // CMD_S2C_LEVEL_UP = 37
        stream.write_i32_le(role_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SEVNPC_HELLO_RE (Comando 70) abrindo a janela de diálogo com o NPC
    pub fn sevnpc_hello_re(nid: i32, _talk_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(70);               // CMD_S2C_NPC_GREETING = 70
        stream.write_i32_le(nid);
        Self { data: stream.into_bytes().to_vec() }
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



