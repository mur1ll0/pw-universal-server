use crate::octets::{OctetsStream, Result};
use pw_core::{CharacterClass, Gender, Race, RoleId, Vector3};
use serde::{Deserialize, Serialize};

/// C2S: Resposta ao desafio de conexão com credenciais do jogador (Opcode 2 / Response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SChallengeResponse {
    pub username: String,
    pub password_response: Vec<u8>,
    pub use_token: bool,
    pub cli_fingerprint: Vec<u8>,
}

impl C2SChallengeResponse {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let username_bytes = stream.read_octets()?;
        let username = if let Ok(s) = String::from_utf8(username_bytes.clone()) {
            s
        } else if username_bytes.len() % 2 == 0 {
            let u16_vec: Vec<u16> = username_bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u16_vec)
        } else {
            String::from_utf8_lossy(&username_bytes).to_string()
        };

        let password_response = stream.read_octets()?;

        let mut use_token = false;
        let mut cli_fingerprint = Vec::new();

        // Se houver mais bytes na stream (v1.5.3+)
        if !stream.is_empty() {
            if let Ok(tok) = stream.read_i8() {
                use_token = tok != 0;
            }
            if let Ok(fp) = stream.read_octets() {
                cli_fingerprint = fp;
            }
        }

        Ok(Self {
            username,
            password_response,
            use_token,
            cli_fingerprint,
        })
    }
}

/// C2S: Troca de Chaves de Criptografia do Cliente (Opcode 3 / KeyExchange)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SKeyExchange {
    pub nonce: Vec<u8>,
    pub blkickuser: i8,
}

impl C2SKeyExchange {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let nonce = stream.read_octets()?;
        let blkickuser = stream.read_i8().unwrap_or(0);
        Ok(Self { nonce, blkickuser })
    }
}

/// C2S: Solicitação da lista de personagens da conta (Opcode 0x52 / RoleList)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SRoleList {
    pub userid: i32,
    pub localsid: u32,
    pub handle: i32,
}

impl C2SRoleList {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let userid = stream.read_i32()?;
        let localsid = stream.read_u32()?;
        let handle = stream.read_i32().unwrap_or(-1);
        Ok(Self { userid, localsid, handle })
    }
}

/// C2S: Criação de novo personagem (Opcode 0x54 / CreateRole)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SCreateRole {
    pub userid: i32,
    pub localsid: u32,
    pub gender: Gender,
    pub race: Race,
    pub cls: CharacterClass,
    pub name: String,
    pub custom_appearance: Vec<u8>,
}

impl C2SCreateRole {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let userid = stream.read_i32()?;
        let localsid = stream.read_u32()?;
        
        // Estrutura RoleInfo encapsulada no CreateRole oficial
        let _role_id = stream.read_i32().unwrap_or(0);
        let gender_raw = stream.read_u8()?;
        let race_raw = stream.read_u8()?;
        let cls_raw = stream.read_u8()?;
        let _level = stream.read_i32().unwrap_or(1);
        let _level2 = stream.read_i32().unwrap_or(0);
        let name = stream.read_string_utf16le()?;
        let custom_appearance = stream.read_octets()?;

        Ok(Self {
            userid,
            localsid,
            gender: Gender::from_u8(gender_raw),
            race: Race::from_u8(race_raw).unwrap_or(Race::Human),
            cls: CharacterClass::from_u8(cls_raw).unwrap_or(CharacterClass::Blademaster),
            name,
            custom_appearance,
        })
    }
}

/// C2S: Selecionar personagem para entrar no mundo (Opcode 0x46 / SelectRole)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SSelectRole {
    pub role_id: RoleId,
    pub flag: i8,
}

impl C2SSelectRole {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32()?;
        let flag = stream.read_i8().unwrap_or(0);
        Ok(Self { role_id, flag })
    }
}

/// C2S: Excluir personagem (Opcode 0x56 / DeleteRole)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SDeleteRole {
    pub role_id: RoleId,
    pub localsid: u32,
}

impl C2SDeleteRole {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32()?;
        let localsid = stream.read_u32()?;
        Ok(Self { role_id, localsid })
    }
}

/// C2S: Restaurar personagem excluído (Opcode 0x58 / UndoDeleteRole)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SUndoDeleteRole {
    pub role_id: RoleId,
    pub localsid: u32,
}

impl C2SUndoDeleteRole {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32()?;
        let localsid = stream.read_u32()?;
        Ok(Self { role_id, localsid })
    }
}

/// C2S: Entrar no mundo após carregamento do mapa (Opcode 0x48 / EnterWorld)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SEnterWorld {
    pub role_id: RoleId,
    pub provider_link_id: i32,
    pub locktime: i32,
    pub timeout: i32,
    pub settime: i32,
    pub localsid: u32,
}

impl C2SEnterWorld {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32()?;
        let provider_link_id = stream.read_i32().unwrap_or(-1);
        let locktime = stream.read_i32().unwrap_or(0);
        let timeout = stream.read_i32().unwrap_or(0);
        let settime = stream.read_i32().unwrap_or(0);
        let localsid = stream.read_u32().unwrap_or(0);
        Ok(Self {
            role_id,
            provider_link_id,
            locktime,
            timeout,
            settime,
            localsid,
        })
    }
}

/// C2S: Pacote de Movimentação do Jogador no Mundo 3D (Opcode 0x20)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let pos_x = stream.read_f32()?;
        let pos_y = stream.read_f32()?;
        let pos_z = stream.read_f32()?;
        let tgt_x = stream.read_f32()?;
        let tgt_y = stream.read_f32()?;
        let tgt_z = stream.read_f32()?;
        let speed = stream.read_f32()?;
        let timestamp = stream.read_u32()?;

        Ok(Self {
            mode,
            position: Vector3::new(pos_x, pos_y, pos_z),
            target: Vector3::new(tgt_x, tgt_y, tgt_z),
            speed,
            timestamp,
        })
    }
}

/// C2S: Envio de mensagem de Chat (Opcode 0x70)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SPlayerChat {
    pub channel: u8,
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

/// C2S: Heartbeat / Keep-Alive (Opcode 0x5A / PROTOCOL_KEEPALIVE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SHeartbeat {
    pub code: i8,
}

impl C2SHeartbeat {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let code = stream.read_i8().unwrap_or(0);
        Ok(Self { code })
    }
}

/// C2S: Pacote de Dados de Jogo / Movimentação / Ações de Mundo (Opcode 0x20 / 0x22 / PROTOCOL_GAMEDATASEND)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SGamedataSend {
    pub data: Vec<u8>,
}

impl C2SGamedataSend {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let data = stream.read_octets()?;
        Ok(Self { data })
    }
}

/// C2S: GetUIConfig (Opcode 0x68 / 104)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SGetUIConfig {
    pub role_id: i32,
    pub localsid: u32,
}

impl C2SGetUIConfig {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let localsid = stream.read_u32().unwrap_or(0);
        Ok(Self { role_id, localsid })
    }
}

/// C2S: GetFriendList (Opcode 0xCE / 206)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SGetFriendList {
    pub role_id: i32,
    pub localsid: u32,
}

impl C2SGetFriendList {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let localsid = stream.read_u32().unwrap_or(0);
        Ok(Self { role_id, localsid })
    }
}

/// C2S: GetWaitDelRoles (Opcode 0xD9 / 217)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SGetWaitDelRoles {
    pub role_id: i32,
    pub localsid: u32,
}

impl C2SGetWaitDelRoles {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let localsid = stream.read_u32().unwrap_or(0);
        Ok(Self { role_id, localsid })
    }
}

/// C2S: QueryServerTime (Opcode 0x352 / 850)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SQueryServerTime {
    pub reserved: i32,
}

impl C2SQueryServerTime {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let reserved = stream.read_i32().unwrap_or(0);
        Ok(Self { reserved })
    }
}

/// C2S: GetHelpStates (Opcode 0x82 / 130)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SGetHelpStates {
    pub role_id: i32,
    pub localsid: u32,
}

impl C2SGetHelpStates {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let localsid = stream.read_u32().unwrap_or(0);
        Ok(Self { role_id, localsid })
    }
}

/// C2S: SetHelpStates (Opcode 0x80 / 128)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SSetHelpStates {
    pub role_id: i32,
    pub localsid: u32,
    pub help_states: Vec<u8>,
}

impl C2SSetHelpStates {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let localsid = stream.read_u32().unwrap_or(0);
        let help_states = stream.read_octets().unwrap_or_default();
        Ok(Self { role_id, localsid, help_states })
    }
}

/// C2S: SetUIConfig (Opcode 0x6A / 106)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SSetUIConfig {
    pub role_id: i32,
    pub ui_config: Vec<u8>,
}

impl C2SSetUIConfig {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let ui_config = stream.read_octets().unwrap_or_default();
        Ok(Self { role_id, ui_config })
    }
}

/// C2S: ACReport (Opcode 0x1389 / 5001)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SACReport {
    pub data: Vec<u8>,
}

impl C2SACReport {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        Ok(Self {
            data: stream.as_slice().to_vec(),
        })
    }
}

/// C2S: SetCustomData (Opcode 0x66 / 102)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SSetCustomData {
    pub role_id: i32,
    pub data: Vec<u8>,
}

impl C2SSetCustomData {
    pub fn decode(stream: &mut OctetsStream) -> Result<Self> {
        let role_id = stream.read_i32().unwrap_or(0);
        let data = stream.read_octets().unwrap_or_default();
        Ok(Self { role_id, data })
    }
}





