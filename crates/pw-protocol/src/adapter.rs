use crate::octets::OctetsStream;
use crate::packets::s2c::S2CEnterWorldResponse;
use crate::version::GameVersion;
use pw_core::CharacterSummary;
use std::sync::Arc;

/// Trait que abstrai a serialização e regras de protocolo por versão específica de Realm
pub trait ProtocolAdapter: Send + Sync {
    fn version(&self) -> GameVersion;

    /// Encodifica o pacote Challenge (Opcode 1)
    fn encode_challenge(&self, stream: &mut OctetsStream, nonce: &[u8]) {
        let version_code = self.version().server_version_code();
        stream.write_octets(nonce);
        stream.write_u32(version_code);
        stream.write_i8(0); // algo = 0

        // Versões 1.4.8 e 1.5.3 enviam os campos edition (Octets) e exp_rate (u8)
        if self.version() != GameVersion::V1_2_6 {
            stream.write_octets(&[]);
            stream.write_u8(1);
        }
    }

    /// Encodifica o pacote KeyExchange (Opcode 3)
    fn encode_key_exchange(&self, stream: &mut OctetsStream, nonce: &[u8], blkickuser: i8) {
        stream.write_octets(nonce);
        stream.write_i8(blkickuser);
    }

    /// Encodifica o pacote OnlineAnnounce (Opcode 4)
    fn encode_online_announce(&self, stream: &mut OctetsStream, userid: i32, localsid: u32) {
        stream.write_i32(userid);
        stream.write_u32(localsid);
        stream.write_i32(0); // remain_time
        stream.write_i8(1);  // zoneid
        stream.write_i32(0); // free_time_left
        stream.write_i32(0); // free_time_end
        stream.write_i32(0); // creatime

        if self.version() != GameVersion::V1_2_6 {
            stream.write_i8(0); // referrer_flag
            stream.write_i8(0); // passwd_flag
            stream.write_i8(0); // usbbind
            stream.write_i8(0); // accountinfo_flag
        }
    }

    /// Encodifica o pacote StatusAnnounce (Opcode 6) para transição de estado da GUI do cliente
    fn encode_status_announce(&self, stream: &mut OctetsStream, userid: i32, localsid: u32, status: u8) {
        stream.write_i32(userid);
        stream.write_u32(localsid);
        stream.write_u8(status);
    }

    /// Encodifica uma estrutura RoleInfo individual respeitando a versão da engine
    fn encode_role_info(&self, stream: &mut OctetsStream, c: &CharacterSummary) {
        stream.write_i32(c.id);
        stream.write_u8(c.gender as u8);
        stream.write_u8(c.race as u8);
        stream.write_u8(c.cls as u8);
        stream.write_i32(c.level);
        stream.write_i32(0); // level2
        stream.write_string_utf16le(&c.name);

        // Custom appearance (face/hair/body binary octets)
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

        stream.write_i8(if c.is_deleted { 2 } else { 1 });  // status (1 = Active, 2 = Deleting)
        stream.write_i32(0);  // delete_time
        stream.write_i32(0);  // create_time
        stream.write_i32(0);  // lastlogin_time
        stream.write_f32(c.position.x);
        stream.write_f32(c.position.y);
        stream.write_f32(c.position.z);
        stream.write_i32(c.world_id);
        stream.write_octets(&[]); // custom_status
        stream.write_octets(&[]); // charactermode

        // Campos introduzidos a partir do 1.4.8 (23 campos no total)
        if self.version() != GameVersion::V1_2_6 {
            stream.write_i32(0);      // referrer_role
            stream.write_i32(0);      // cash_add
            stream.write_octets(&[]); // reincarnation_data
            stream.write_octets(&[]); // realm_data
        }
    }

    /// Encodifica a lista de personagens RoleList_Re (Opcode 0x53)
    fn encode_role_list(
        &self,
        stream: &mut OctetsStream,
        userid: i32,
        localsid: u32,
        characters: &[CharacterSummary],
    ) {
        stream.write_i32(0);  // result = 0 (ERR_SUCCESS)
        stream.write_i32(-1); // handle = -1 (fim da lista)
        stream.write_i32(userid);
        stream.write_u32(localsid);

        // Vetor de RoleInfo
        stream.write_compact_uint(characters.len() as u32);
        for c in characters {
            self.encode_role_info(stream, c);
        }
    }

    /// Encodifica resposta de criação de personagem CreateRole_Re (Opcode 0x55)
    fn encode_create_role_response(
        &self,
        stream: &mut OctetsStream,
        result: i32,
        role_id: i32,
        localsid: u32,
        character: Option<&CharacterSummary>,
    ) {
        stream.write_i32(result);
        stream.write_i32(role_id);
        stream.write_u32(localsid);

        if let Some(c) = character {
            self.encode_role_info(stream, c);
        } else {
            let default_summary = CharacterSummary {
                id: role_id,
                account_id: 0,
                realm_id: String::new(),
                name: String::new(),
                race: pw_core::Race::Human,
                cls: pw_core::CharacterClass::Blademaster,
                gender: pw_core::Gender::Male,
                level: 1,
                cultivation: 0,
                world_id: 1,
                position: pw_core::Vector3::zero(),
                equipment: Vec::new(),
                custom_appearance: serde_json::json!({}),
                is_deleted: false,
                delete_time: None,
            };
            self.encode_role_info(stream, &default_summary);
        }

        // Versões 1.4.8 e 1.5.3 possuem refretcode no final
        if self.version() != GameVersion::V1_2_6 {
            stream.write_i32(0);
        }
    }

    /// Encodifica resposta de exclusão de personagem DeleteRole_Re (Opcode 0x57)
    fn encode_delete_role_response(
        &self,
        stream: &mut OctetsStream,
        result: i32,
        role_id: i32,
        localsid: u32,
    ) {
        stream.write_i32(result);
        stream.write_i32(role_id);
        stream.write_u32(localsid);
    }

    /// Encodifica resposta de restauração de personagem UndoDeleteRole_Re (Opcode 0x59)
    fn encode_undo_delete_role_response(
        &self,
        stream: &mut OctetsStream,
        result: i32,
        role_id: i32,
        localsid: u32,
    ) {
        stream.write_i32(result);
        stream.write_i32(role_id);
        stream.write_u32(localsid);
    }

    /// Encodifica resposta de seleção de personagem SelectRole_Re (Opcode 0x47)
    fn encode_select_role_response(
        &self,
        stream: &mut OctetsStream,
        result: i32,
        auth: &[u8],
    ) {
        stream.write_i32(result);
        stream.write_compact_uint(auth.len() as u32);
        for &b in auth {
            stream.write_u8(b);
        }
    }

    /// Encodifica confirmação de entrada no mundo EnterWorld (Opcode 0x45)
    fn encode_enter_world(&self, stream: &mut OctetsStream, enter: &S2CEnterWorldResponse) {
        stream.write_i32(enter.role_id);
        stream.write_i32(enter.world_id);
        stream.write_f32(enter.position.x);
        stream.write_f32(enter.position.y);
        stream.write_f32(enter.position.z);
        stream.write_i32(enter.hp);
        stream.write_i32(enter.max_hp);
        stream.write_i32(enter.mp);
        stream.write_i32(enter.max_mp);
        stream.write_i64(enter.exp);
        stream.write_i64(enter.sp);
        stream.write_i32(enter.level);
    }

    /// Encodifica pacote GamedataSend (Opcode 0x20 / PROTOCOL_GAMEDATASEND)
    fn encode_gamedata_send(&self, stream: &mut OctetsStream, data: &[u8]) {
        stream.write_octets(data);
    }
}

/// Adaptador para o Realm Perfect World Classic (v1.2.6)
pub struct Protocol126Adapter;
impl ProtocolAdapter for Protocol126Adapter {
    fn version(&self) -> GameVersion {
        GameVersion::V1_2_6
    }
}

/// Adaptador para o Realm Perfect World Tides / Genesis (v1.4.8)
pub struct Protocol148Adapter;
impl ProtocolAdapter for Protocol148Adapter {
    fn version(&self) -> GameVersion {
        GameVersion::V1_4_8
    }
}

/// Adaptador para o Realm Perfect World Eclipse (v1.5.3)
pub struct Protocol153Adapter;
impl ProtocolAdapter for Protocol153Adapter {
    fn version(&self) -> GameVersion {
        GameVersion::V1_5_3
    }
}

/// Factory para obter o adaptador correspondente à versão solicitada
pub fn create_protocol_adapter(version: GameVersion) -> Arc<dyn ProtocolAdapter> {
    match version {
        GameVersion::V1_2_6 => Arc::new(Protocol126Adapter),
        GameVersion::V1_4_8 => Arc::new(Protocol148Adapter),
        GameVersion::V1_5_3 => Arc::new(Protocol153Adapter),
    }
}
