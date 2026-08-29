use crate::adapter::{create_protocol_adapter, ProtocolAdapter};
use crate::octets::{OctetsError, OctetsStream, Result};
use crate::opcodes::*;
use crate::packets::c2s::*;
use crate::packets::s2c::*;
use crate::version::GameVersion;
use bytes::{Buf, BytesMut};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
use tracing::{trace, warn};

/// Enum que encapsula todos os pacotes decodificados vindos do cliente (C2S)
#[derive(Debug, Clone)]
pub enum InboundPacket {
    Response(C2SChallengeResponse),
    KeyExchange(C2SKeyExchange),
    RoleList(C2SRoleList),
    CreateRole(C2SCreateRole),
    DeleteRole(C2SDeleteRole),
    UndoDeleteRole(C2SUndoDeleteRole),
    SelectRole(C2SSelectRole),
    EnterWorld(C2SEnterWorld),
    GamedataSend(C2SGamedataSend),
    GetUIConfig(C2SGetUIConfig),
    SetUIConfig(C2SSetUIConfig),
    SetCustomData(C2SSetCustomData),
    GetFriendList(C2SGetFriendList),
    GetWaitDelRoles(C2SGetWaitDelRoles),
    QueryServerTime(C2SQueryServerTime),
    GetHelpStates(C2SGetHelpStates),
    SetHelpStates(C2SSetHelpStates),
    ACReport(C2SACReport),
    PlayerMove(C2SPlayerMove),
    PlayerChat(C2SPlayerChat),
    Heartbeat(C2SHeartbeat),
    Unknown { opcode: u32, payload: Vec<u8> },
}

/// Enum que encapsula todos os pacotes enviados do servidor para o cliente (S2C)
#[derive(Debug, Clone)]
pub enum OutboundPacket {
    Challenge(S2CChallenge),
    KeyExchange(S2CKeyExchange),
    OnlineAnnounce(S2COnlineAnnounce),
    StatusAnnounce(S2CStatusAnnounce),
    ErrorInfo(S2CErrorInfo),
    RoleListResponse(S2CRoleListResponse),
    CreateRoleResponse(S2CCreateRoleResponse),
    DeleteRoleResponse(S2CDeleteRoleResponse),
    UndoDeleteRoleResponse(S2CUndoDeleteRoleResponse),
    SelectRoleResponse(S2CSelectRoleResponse),
    EnterWorld(S2CEnterWorldResponse),
    GamedataSend(S2CGamedataSend),
    GetUIConfigRe(S2CGetUIConfigRe),
    SetUIConfigRe(S2CSetUIConfigRe),
    SetCustomDataRe(S2CSetCustomDataRe),
    PlayerLogout(S2CPlayerLogout),
    GetFriendListRe(S2CGetFriendListRe),
    GetWaitDelRolesRe(S2CGetWaitDelRolesRe),
    GetHelpStatesRe(S2CGetHelpStatesRe),
    SetHelpStatesRe(S2CSetHelpStatesRe),
    ServerTimeRe(S2CServerTimeRe),
    PlayerMoveBroadcast(S2CPlayerMoveBroadcast),
    ChatBroadcast(S2CChatBroadcast),
    Raw { opcode: u32, payload: Vec<u8> },
}

/// Codec de framing e decodificação do protocolo de rede do Perfect World
/// Suporta framing por tamanho compacto CUint32 oficial do CNet (Wanmei Engine)
/// e desacoplamento de serialização por ProtocolAdapter
pub struct PwPacketCodec {
    pub adapter: Arc<dyn ProtocolAdapter>,
}

impl PwPacketCodec {
    pub fn new(version_str: &str) -> Self {
        let version = version_str.parse::<GameVersion>().unwrap_or(GameVersion::V1_2_6);
        Self {
            adapter: create_protocol_adapter(version),
        }
    }

    pub fn from_adapter(adapter: Arc<dyn ProtocolAdapter>) -> Self {
        Self { adapter }
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

        trace!(
            "[{}] Decodificando pacote opcode: 0x{:X} (Tamanho: {} bytes)",
            self.adapter.version(),
            opcode,
            packet_len
        );

        // 2. Decodifica o payload conforme o opcode
        let packet = match opcode {
            OP_C2S_RESPONSE => InboundPacket::Response(C2SChallengeResponse::decode(&mut payload_stream)?),
            OP_C2S_KEYEXCHANGE => InboundPacket::KeyExchange(C2SKeyExchange::decode(&mut payload_stream)?),
            OP_C2S_ROLE_LIST => InboundPacket::RoleList(C2SRoleList::decode(&mut payload_stream)?),
            OP_C2S_CREATE_ROLE => InboundPacket::CreateRole(C2SCreateRole::decode(&mut payload_stream)?),
            OP_C2S_DELETE_ROLE => InboundPacket::DeleteRole(C2SDeleteRole::decode(&mut payload_stream)?),
            OP_C2S_UNDO_DELETE_ROLE => InboundPacket::UndoDeleteRole(C2SUndoDeleteRole::decode(&mut payload_stream)?),
            OP_C2S_SELECT_ROLE => InboundPacket::SelectRole(C2SSelectRole::decode(&mut payload_stream)?),
            OP_C2S_ENTER_WORLD => InboundPacket::EnterWorld(C2SEnterWorld::decode(&mut payload_stream)?),
            OP_C2S_GAMEDATASEND | OP_S2C_GAMEDATASEND => InboundPacket::GamedataSend(C2SGamedataSend::decode(&mut payload_stream)?),
            OP_C2S_GET_UI_CONFIG => InboundPacket::GetUIConfig(C2SGetUIConfig::decode(&mut payload_stream)?),
            OP_C2S_SET_UI_CONFIG => InboundPacket::SetUIConfig(C2SSetUIConfig::decode(&mut payload_stream)?),
            OP_C2S_SET_CUSTOM_DATA => InboundPacket::SetCustomData(C2SSetCustomData::decode(&mut payload_stream)?),
            OP_C2S_GET_FRIEND_LIST => InboundPacket::GetFriendList(C2SGetFriendList::decode(&mut payload_stream)?),
            OP_C2S_GET_WAIT_DEL_ROLES => InboundPacket::GetWaitDelRoles(C2SGetWaitDelRoles::decode(&mut payload_stream)?),
            OP_C2S_QUERY_SERVER_TIME => InboundPacket::QueryServerTime(C2SQueryServerTime::decode(&mut payload_stream)?),
            OP_C2S_GET_HELP_STATES => InboundPacket::GetHelpStates(C2SGetHelpStates::decode(&mut payload_stream)?),
            OP_C2S_SET_HELP_STATES => InboundPacket::SetHelpStates(C2SSetHelpStates::decode(&mut payload_stream)?),
            OP_C2S_ACREPORT => InboundPacket::ACReport(C2SACReport::decode(&mut payload_stream)?),
            OP_C2S_CHAT => InboundPacket::PlayerChat(C2SPlayerChat::decode(&mut payload_stream)?),
            OP_C2S_HEARTBEAT => InboundPacket::Heartbeat(C2SHeartbeat::decode(&mut payload_stream)?),
            _ => {
                warn!(
                    "Opcode C2S desconhecido ou não tratado recebido do cliente: 0x{:X} (Dec: {}, Tamanho: {} bytes, Hex: {})",
                    opcode, opcode, packet_len, hex::encode(&payload)
                );
                InboundPacket::Unknown {
                    opcode,
                    payload: payload.to_vec(),
                }
            }
        };

        Ok(Some(packet))
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>> {
        match self.decode(buf)? {
            Some(frame) => Ok(Some(frame)),
            None => {
                if buf.is_empty() {
                    Ok(None)
                } else {
                    Err(OctetsError::BufferUnderflow)
                }
            }
        }
    }
}

impl Encoder<OutboundPacket> for PwPacketCodec {
    type Error = OctetsError;

    fn encode(&mut self, item: OutboundPacket, dst: &mut BytesMut) -> Result<()> {
        let (opcode, payload) = match item {
            OutboundPacket::Challenge(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_challenge(&mut ps, &p.nonce);
                (OP_S2C_CHALLENGE, ps.into_bytes())
            }
            OutboundPacket::KeyExchange(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_key_exchange(&mut ps, &p.nonce, p.blkickuser);
                (OP_S2C_KEYEXCHANGE, ps.into_bytes())
            }
            OutboundPacket::OnlineAnnounce(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_online_announce(&mut ps, p.userid, p.localsid);
                (OP_S2C_ONLINEANNOUNCE, ps.into_bytes())
            }
            OutboundPacket::StatusAnnounce(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_status_announce(&mut ps, p.userid, p.localsid, p.status);
                (OP_S2C_STATUSANNOUNCE, ps.into_bytes())
            }
            OutboundPacket::ErrorInfo(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_ERRORINFO, ps.into_bytes())
            }
            OutboundPacket::RoleListResponse(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_role_list(&mut ps, p.userid, p.localsid, &p.characters);
                (OP_S2C_ROLE_LIST_RES, ps.into_bytes())
            }
            OutboundPacket::CreateRoleResponse(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_create_role_response(&mut ps, p.result, p.role_id, p.localsid, p.character.as_ref());
                (OP_S2C_CREATE_ROLE_RES, ps.into_bytes())
            }
            OutboundPacket::DeleteRoleResponse(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_delete_role_response(&mut ps, p.result, p.role_id, p.localsid);
                (OP_S2C_DELETE_ROLE_RES, ps.into_bytes())
            }
            OutboundPacket::UndoDeleteRoleResponse(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_undo_delete_role_response(&mut ps, p.result, p.role_id, p.localsid);
                (OP_S2C_UNDO_DELETE_ROLE_RES, ps.into_bytes())
            }
            OutboundPacket::SelectRoleResponse(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_select_role_response(&mut ps, p.result, &p.auth);
                (OP_S2C_SELECT_ROLE_RE, ps.into_bytes())
            }
            OutboundPacket::EnterWorld(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_enter_world(&mut ps, &p);
                (OP_S2C_ENTER_WORLD, ps.into_bytes())
            }
            OutboundPacket::GamedataSend(p) => {
                let mut ps = OctetsStream::new();
                self.adapter.encode_gamedata_send(&mut ps, &p.data);
                (OP_S2C_GAMEDATASEND, ps.into_bytes())
            }
            OutboundPacket::GetUIConfigRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_GET_UI_CONFIG_RE, ps.into_bytes())
            }
            OutboundPacket::SetUIConfigRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_SET_UI_CONFIG_RE, ps.into_bytes())
            }
            OutboundPacket::SetCustomDataRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_SET_CUSTOM_DATA_RE, ps.into_bytes())
            }
            OutboundPacket::PlayerLogout(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_PLAYER_LOGOUT, ps.into_bytes())
            }
            OutboundPacket::GetFriendListRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_GET_FRIEND_LIST_RE, ps.into_bytes())
            }
            OutboundPacket::GetWaitDelRolesRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_GET_WAIT_DEL_ROLES_RE, ps.into_bytes())
            }
            OutboundPacket::GetHelpStatesRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_GET_HELP_STATES_RE, ps.into_bytes())
            }
            OutboundPacket::SetHelpStatesRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_SET_HELP_STATES_RE, ps.into_bytes())
            }
            OutboundPacket::ServerTimeRe(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_QUERY_SERVER_TIME_RE, ps.into_bytes())
            }
            OutboundPacket::PlayerMoveBroadcast(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_PLAYER_MOVE_BROADCAST, ps.into_bytes())
            }
            OutboundPacket::ChatBroadcast(p) => {
                let mut ps = OctetsStream::new();
                p.encode(&mut ps, self.adapter.version().as_str());
                (OP_S2C_CHAT_BROADCAST, ps.into_bytes())
            }
            OutboundPacket::Raw { opcode, payload } => (opcode, bytes::Bytes::from(payload)),
        };

        // Wanmei Network Framing: [CompactUINT(opcode)] [CompactUINT(payload_len)] [payload_bytes]
        let mut frame_header = OctetsStream::new();
        frame_header.write_compact_uint(opcode);
        frame_header.write_compact_uint(payload.len() as u32);

        dst.extend_from_slice(frame_header.as_slice());
        dst.extend_from_slice(&payload);
        Ok(())
    }
}
