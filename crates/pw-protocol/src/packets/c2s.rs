use crate::octets::{OctetsStream, Result};
use pw_core::{CharacterClass, Gender, Race, RoleId, Vector3};

/// C2S: Resposta ao desafio de conexão com credenciais do jogador
#[derive(Debug, Clone)]
pub struct C2SChallengeResponse {
    pub username: String,
    pub password_hash: String,
    pub client_version: u32,
    pub response_token: Vec<u8>,
}

impl C2SChallengeResponse {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let username = stream.read_string_utf16le()?;
        let password_hash = stream.read_string_utf8()?;
        let client_version = stream.read_u32_le()?;
        let response_token = stream.read_octets()?;

        Ok(Self {
            username,
            password_hash,
            client_version,
            response_token,
        })
    }
}

/// C2S: Solicitação da lista de personagens da conta
#[derive(Debug, Clone)]
pub struct C2SRoleList {
    pub account_id: i32,
}

impl C2SRoleList {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let account_id = stream.read_i32_le()?;
        Ok(Self { account_id })
    }
}

/// C2S: Criação de novo personagem
#[derive(Debug, Clone)]
pub struct C2SCreateRole {
    pub name: String,
    pub race: Race,
    pub cls: CharacterClass,
    pub gender: Gender,
    pub custom_appearance: Vec<u8>,
}

impl C2SCreateRole {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let name = stream.read_string_utf16le()?;
        let race_raw = stream.read_u8()?;
        let cls_raw = stream.read_u8()?;
        let gender_raw = stream.read_u8()?;
        let custom_appearance = stream.read_octets()?;

        Ok(Self {
            name,
            race: Race::from_u8(race_raw).unwrap_or(Race::Human),
            cls: CharacterClass::from_u8(cls_raw).unwrap_or(CharacterClass::Blademaster),
            gender: Gender::from_u8(gender_raw),
            custom_appearance,
        })
    }
}

/// C2S: Selecionar personagem e entrar no mundo de jogo
#[derive(Debug, Clone)]
pub struct C2SSelectRole {
    pub role_id: RoleId,
}

impl C2SSelectRole {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32_le()?;
        Ok(Self { role_id })
    }
}

/// C2S: Pacote de Movimentação do Jogador no Mundo 3D
#[derive(Debug, Clone)]
pub struct C2SPlayerMove {
    pub mode: u8,          // 0: Andando, 1: Correndo, 2: Nadando, 3: Voando, 4: Queda
    pub position: Vector3, // Posição atual (X, Y, Z)
    pub target: Vector3,   // Destino (X, Y, Z)
    pub speed: f32,        // Velocidade atual
    pub timestamp: u32,
}

impl C2SPlayerMove {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let mode = stream.read_u8()?;
        let pos_x = stream.read_f32_le()?;
        let pos_y = stream.read_f32_le()?;
        let pos_z = stream.read_f32_le()?;
        let tgt_x = stream.read_f32_le()?;
        let tgt_y = stream.read_f32_le()?;
        let tgt_z = stream.read_f32_le()?;
        let speed = stream.read_f32_le()?;
        let timestamp = stream.read_u32_le()?;

        Ok(Self {
            mode,
            position: Vector3::new(pos_x, pos_y, pos_z),
            target: Vector3::new(tgt_x, tgt_y, tgt_z),
            speed,
            timestamp,
        })
    }
}

/// C2S: Envio de mensagem de Chat (Geral, Clã, Mundo, Sussurro)
#[derive(Debug, Clone)]
pub struct C2SPlayerChat {
    pub channel: u8, // 0: Geral, 1: Mundo, 2: Clã, 3: Grupo, 4: Sussurro
    pub target_name: Option<String>,
    pub message: String,
}

impl C2SPlayerChat {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let channel = stream.read_u8()?;
        let target_name = if channel == 4 {
            Some(stream.read_string_utf16le()?)
        } else {
            None
        };
        let message = stream.read_string_utf16le()?;

        Ok(Self {
            channel,
            target_name,
            message,
        })
    }
}

/// C2S: Heartbeat / Keep-Alive
#[derive(Debug, Clone)]
pub struct C2SHeartbeat {
    pub timestamp: u32,
}

impl C2SHeartbeat {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let timestamp = stream.read_u32_le()?;
        Ok(Self { timestamp })
    }
}
