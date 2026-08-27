use crate::octets::OctetsStream;
use crate::opcodes::*;
use pw_core::{CharacterSummary, RoleId, Vector3, WorldId};

/// S2C: Desafio inicial de conexão enviado ao cliente com chave de sessão
#[derive(Debug, Clone)]
pub struct S2CChallenge {
    pub server_version: u32,
    pub nonce: Vec<u8>,
}

impl S2CChallenge {
    pub fn encode(&self, stream: &mut OctetsStream) {
        stream.write_compact_uint(OP_S2C_CHALLENGE);
        stream.write_u32_le(self.server_version);
        stream.write_octets(&self.nonce);
    }
}

/// S2C: Resposta de Login com Sucesso
#[derive(Debug, Clone)]
pub struct S2CLoginSuccess {
    pub account_id: i32,
    pub gm_privileges: i32,
    pub session_ticket: String,
}

impl S2CLoginSuccess {
    pub fn encode(&self, stream: &mut OctetsStream) {
        stream.write_compact_uint(OP_S2C_LOGIN_SUCCESS);
        stream.write_i32_le(self.account_id);
        stream.write_i32_le(self.gm_privileges);
        stream.write_string_utf8(&self.session_ticket);
    }
}

/// S2C: Lista de Personagens do Realm enviada para o cliente
#[derive(Debug, Clone)]
pub struct S2CRoleListResponse {
    pub characters: Vec<CharacterSummary>,
}

impl S2CRoleListResponse {
    pub fn encode(&self, stream: &mut OctetsStream) {
        stream.write_compact_uint(OP_S2C_ROLE_LIST_RES);
        stream.write_compact_uint(self.characters.len() as u32);

        for c in &self.characters {
            stream.write_i32_le(c.id);
            stream.write_string_utf16le(&c.name);
            stream.write_u8(c.race as u8);
            stream.write_u8(c.cls as u8);
            stream.write_u8(c.gender as u8);
            stream.write_i32_le(c.level);
            stream.write_i32_le(c.cultivation);
            stream.write_f32_le(c.position.x);
            stream.write_f32_le(c.position.y);
            stream.write_f32_le(c.position.z);
            stream.write_i32_le(c.world_id);

            // Itens equipados para renderização na tela de seleção
            stream.write_compact_uint(c.equipment.len() as u32);
            for item in &c.equipment {
                stream.write_u16_le(item.slot);
                stream.write_u32_le(item.item_id);
                stream.write_u8(item.refine_level);
                stream.write_u8(item.sockets_count);
            }

            // Customização de aparência
            let appearance_bytes = serde_json::to_vec(&c.custom_appearance).unwrap_or_default();
            stream.write_octets(&appearance_bytes);
        }
    }
}

/// S2C: Confirmação de entrada no mundo de jogo
#[derive(Debug, Clone)]
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
    pub fn encode(&self, stream: &mut OctetsStream) {
        stream.write_compact_uint(OP_S2C_ENTER_WORLD);
        stream.write_i32_le(self.role_id);
        stream.write_i32_le(self.world_id);
        stream.write_f32_le(self.position.x);
        stream.write_f32_le(self.position.y);
        stream.write_f32_le(self.position.z);
        stream.write_i32_le(self.hp);
        stream.write_i32_le(self.max_hp);
        stream.write_i32_le(self.mp);
        stream.write_i32_le(self.max_mp);
        stream.write_i64_le(self.exp);
        stream.write_i64_le(self.sp);
        stream.write_i32_le(self.level);
    }
}

/// S2C: Transmissão de movimentação de um jogador para outros ao redor
#[derive(Debug, Clone)]
pub struct S2CPlayerMoveBroadcast {
    pub role_id: RoleId,
    pub mode: u8,
    pub position: Vector3,
    pub target: Vector3,
    pub speed: f32,
    pub timestamp: u32,
}

impl S2CPlayerMoveBroadcast {
    pub fn encode(&self, stream: &mut OctetsStream) {
        stream.write_compact_uint(OP_S2C_PLAYER_MOVE_BROADCAST);
        stream.write_i32_le(self.role_id);
        stream.write_u8(self.mode);
        stream.write_f32_le(self.position.x);
        stream.write_f32_le(self.position.y);
        stream.write_f32_le(self.position.z);
        stream.write_f32_le(self.target.x);
        stream.write_f32_le(self.target.y);
        stream.write_f32_le(self.target.z);
        stream.write_f32_le(self.speed);
        stream.write_u32_le(self.timestamp);
    }
}

/// S2C: Transmissão de mensagem de Chat
#[derive(Debug, Clone)]
pub struct S2CChatBroadcast {
    pub channel: u8,
    pub sender_id: RoleId,
    pub sender_name: String,
    pub message: String,
}

impl S2CChatBroadcast {
    pub fn encode(&self, stream: &mut OctetsStream) {
        stream.write_compact_uint(OP_S2C_CHAT_BROADCAST);
        stream.write_u8(self.channel);
        stream.write_i32_le(self.sender_id);
        stream.write_string_utf16le(&self.sender_name);
        stream.write_string_utf16le(&self.message);
    }
}
