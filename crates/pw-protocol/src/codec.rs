use crate::octets::{OctetsError, OctetsStream, Result};
use crate::opcodes::*;
use crate::packets::c2s::*;
use crate::packets::s2c::*;
use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{debug, warn};

/// Enum que encapsula todos os pacotes decodificados vindos do cliente (C2S)
#[derive(Debug, Clone)]
pub enum InboundPacket {
    ChallengeResponse(C2SChallengeResponse),
    RoleList(C2SRoleList),
    CreateRole(C2SCreateRole),
    SelectRole(C2SSelectRole),
    PlayerMove(C2SPlayerMove),
    PlayerChat(C2SPlayerChat),
    Heartbeat(C2SHeartbeat),
    Unknown { opcode: u32, payload: Vec<u8> },
}

/// Enum que encapsula todos os pacotes enviados do servidor para o cliente (S2C)
#[derive(Debug, Clone)]
pub enum OutboundPacket {
    Challenge(S2CChallenge),
    LoginSuccess(S2CLoginSuccess),
    RoleListResponse(S2CRoleListResponse),
    EnterWorld(S2CEnterWorldResponse),
    PlayerMoveBroadcast(S2CPlayerMoveBroadcast),
    ChatBroadcast(S2CChatBroadcast),
    Raw { opcode: u32, payload: Vec<u8> },
}

/// Codec de framing e decodificação do protocolo de rede do Perfect World
/// Suporta framing por tamanho compacto CUint32 ou tamanho fixo.
pub struct PwPacketCodec {
    pub game_version: String, // "1.2.6", "1.5.3"
}

impl PwPacketCodec {
    pub fn new(game_version: &str) -> Self {
        Self {
            game_version: game_version.to_string(),
        }
    }
}

impl Decoder for PwPacketCodec {
    type Item = InboundPacket;
    type Error = OctetsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut stream = OctetsStream::from_bytes(src);
        
        // 1. Tenta ler o opcode e o tamanho do pacote
        let opcode = match stream.read_compact_uint() {
            Ok(op) => op,
            Err(OctetsError::BufferUnderflow) => return Ok(None),
            Err(e) => return Err(e),
        };

        let packet_len = match stream.read_compact_uint() {
            Ok(len) => len as usize,
            Err(OctetsError::BufferUnderflow) => return Ok(None),
            Err(e) => return Err(e),
        };

        let header_consumed = src.len() - stream.len();
        if src.len() < header_consumed + packet_len {
            // Buffer incompleto, aguarda mais bytes da rede
            return Ok(None);
        }

        // Descarta o cabeçalho consumido do buffer de entrada
        src.advance(header_consumed);
        let payload = src.split_to(packet_len);
        let mut payload_stream = OctetsStream::from_bytes(&payload);

        debug!("Decodificando pacote opcode: 0x{:X} (Tamanho: {} bytes)", opcode, packet_len);

        // 2. Decodifica o payload conforme o opcode e a versão
        let packet = match opcode {
            OP_C2S_CHALLENGE_RES => InboundPacket::ChallengeResponse(C2SChallengeResponse::decode(&mut payload_stream)?),
            OP_C2S_ROLE_LIST => InboundPacket::RoleList(C2SRoleList::decode(&mut payload_stream)?),
            OP_C2S_CREATE_ROLE => InboundPacket::CreateRole(C2SCreateRole::decode(&mut payload_stream)?),
            OP_C2S_SELECT_ROLE => InboundPacket::SelectRole(C2SSelectRole::decode(&mut payload_stream)?),
            OP_C2S_PLAYER_MOVE => InboundPacket::PlayerMove(C2SPlayerMove::decode(&mut payload_stream)?),
            OP_C2S_CHAT => InboundPacket::PlayerChat(C2SPlayerChat::decode(&mut payload_stream)?),
            OP_C2S_HEARTBEAT => InboundPacket::Heartbeat(C2SHeartbeat::decode(&mut payload_stream)?),
            _ => {
                warn!("Opcode desconhecido recebido do cliente: 0x{:X}", opcode);
                InboundPacket::Unknown {
                    opcode,
                    payload: payload.to_vec(),
                }
            }
        };

        Ok(Some(packet))
    }
}

impl Encoder<OutboundPacket> for PwPacketCodec {
    type Error = OctetsError;

    fn encode(&mut self, item: OutboundPacket, dst: &mut BytesMut) -> Result<()> {
        let mut payload_stream = OctetsStream::new();

        match item {
            OutboundPacket::Challenge(p) => p.encode(&mut payload_stream),
            OutboundPacket::LoginSuccess(p) => p.encode(&mut payload_stream),
            OutboundPacket::RoleListResponse(p) => p.encode(&mut payload_stream),
            OutboundPacket::EnterWorld(p) => p.encode(&mut payload_stream),
            OutboundPacket::PlayerMoveBroadcast(p) => p.encode(&mut payload_stream),
            OutboundPacket::ChatBroadcast(p) => p.encode(&mut payload_stream),
            OutboundPacket::Raw { opcode, payload } => {
                let mut frame_stream = OctetsStream::new();
                frame_stream.write_compact_uint(opcode);
                frame_stream.write_compact_uint(payload.len() as u32);
                dst.extend_from_slice(frame_stream.as_slice());
                dst.extend_from_slice(&payload);
                return Ok(());
            }
        }

        // Escreve o cabeçalho com tamanho
        let mut header_stream = OctetsStream::new();
        header_stream.write_compact_uint(payload_stream.len() as u32);

        dst.extend_from_slice(payload_stream.as_slice());
        Ok(())
    }
}
