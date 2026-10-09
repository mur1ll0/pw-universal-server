//! As mensagens que trafegam **entre daemons**, e não com o cliente.
//!
//! Nada aqui é invenção deste projeto: são protocolos GNET de verdade, catalogados em
//! `specs/protocol/gnet_153.json`, e o `daemons` de cada um no IR diz exatamente quem
//! fala com quem.
//!
//! # A diferença que define o barramento
//!
//! O cliente manda `GamedataSend` (34) com **um único campo**: `data`. Ele não precisa
//! dizer quem é — a conexão já sabe.
//!
//! Entre `glinkd` e `gamed` os mesmos bytes viajam em `C2SGamedataSend` (75) e
//! `S2CGamedataSend` (74), que acrescentam **`roleid` e `localsid`**. É por aí que o
//! daemon de link diz ao servidor de jogo de quem é aquele payload, e por onde a
//! resposta volta. Esse par de campos *é* o barramento.
//!
//! ```text
//! cliente  --GamedataSend(34){data}-->  glinkd
//! glinkd   --C2SGamedataSend(75){roleid, localsid, data}-->  gamed
//! gamed    --S2CGamedataSend(74){roleid, localsid, data}-->  glinkd
//! glinkd   --GamedataSend(34){data}-->  cliente
//! ```
//!
//! O `data` é opaco para o barramento: são os subcomandos do mundo 3D, com **outro
//! formato de fio** (little-endian, `pack(1)`), descritos em
//! `specs/protocol/gamedata_153.json` e lidos pelo `pw_wire::gamedata`. O envelope aqui
//! é GNET (big-endian, `CompactUINT`), e confundir os dois é o erro clássico deste
//! protocolo.

use pw_wire::gnet::{Reader, Writer};
use pw_wire::WireError;

/// Opcodes das mensagens de barramento, conferidos contra o IR pelo teste.
pub mod opcode {
    /// `PROTOCOL_PLAYERLOGOUT` — `glinkd`, `gdeliveryd`, `gamed`.
    pub const PLAYER_LOGOUT: u32 = 69;
    /// `PROTOCOL_ENTERWORLD` — `glinkd`, `gdeliveryd`, `gamed`.
    pub const ENTER_WORLD: u32 = 72;
    /// `PROTOCOL_S2CGAMEDATASEND` — `gamed` → `glinkd`.
    pub const S2C_GAMEDATA_SEND: u32 = 74;
    /// `PROTOCOL_C2SGAMEDATASEND` — `glinkd` → `gamed`.
    pub const C2S_GAMEDATA_SEND: u32 = 75;
    /// `PROTOCOL_CHATSINGLECAST` — `gamed` → `glinkd`: uma fala para um jogador.
    pub const CHAT_SINGLE_CAST: u32 = 94;
    /// `PROTOCOL_SETCUSTOMDATA` — o cliente manda ao `glinkd`; com o personagem em jogo, o
    /// link o repassa ao `gamed` dono, que guarda o bilhete de rosto (B199).
    pub const SET_CUSTOM_DATA: u32 = 100;
    /// `PROTOCOL_SETCUSTOMDATA_RE` — a resposta, do `gamed` ao link e do link ao cliente.
    pub const SET_CUSTOM_DATA_RE: u32 = 101;
}

/// Uma mensagem entre daemons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusMessage {
    /// `C2SGamedataSend` (75): o payload que veio do cliente, agora com dono.
    ClientToGame {
        roleid: i32,
        localsid: u32,
        data: Vec<u8>,
    },
    /// `S2CGamedataSend` (74): o que o servidor de jogo quer mandar àquele jogador.
    GameToClient {
        roleid: i32,
        localsid: u32,
        data: Vec<u8>,
    },
    /// `EnterWorld` (72): o jogador entrou; o servidor de jogo passa a ser dono dele.
    EnterWorld {
        roleid: i32,
        provider_link_id: i32,
        locktime: i32,
        timeout: i32,
        settime: i32,
        localsid: u32,
    },
    /// `PlayerLogout` (69): o jogador saiu.
    PlayerLogout {
        result: i32,
        roleid: i32,
        provider_link_id: i32,
        localsid: u32,
    },
    /// `ChatSingleCast` (94): uma fala que o mundo manda a **um** jogador — a do monstro
    /// (`SaySomething`, `ainpc.cpp:47-55`). `msg` em UTF-16LE, como no cliente. O link a
    /// entrega como `ChatMessage` (80), o pacote que o cliente trata para fala de NPC
    /// (`EC_GameSession.cpp:4953-4965`).
    ChatSingleCast {
        channel: u8,
        emotion: u8,
        srcroleid: i32,
        dstroleid: i32,
        dstlocalsid: u32,
        msg: Vec<u8>,
        data: Vec<u8>,
    },
    /// `SetCustomData` (100): a aparência nova que o cliente mandou ao fim da troca de rosto
    /// (`CECCustomizeHostPolicy::OnOK`, `EC_CustomizePolicy.cpp:73-85`). No original quem
    /// confere o bilhete é o `gdeliveryd` (`setcustomdata.hpp`); aqui o link repassa ao GS
    /// dono, que tem a sessão cosmética.
    SetCustomData {
        roleid: i32,
        localsid: u32,
        custom_data: Vec<u8>,
    },
    /// `SetCustomData_Re` (101): `result` (0 ou `ERR_NOFACETICKET` 201) e o carimbo novo.
    SetCustomDataRe {
        result: i32,
        crc: u32,
        roleid: i32,
        localsid: u32,
    },
}

impl BusMessage {
    pub fn opcode(&self) -> u32 {
        match self {
            BusMessage::ClientToGame { .. } => opcode::C2S_GAMEDATA_SEND,
            BusMessage::GameToClient { .. } => opcode::S2C_GAMEDATA_SEND,
            BusMessage::EnterWorld { .. } => opcode::ENTER_WORLD,
            BusMessage::PlayerLogout { .. } => opcode::PLAYER_LOGOUT,
            BusMessage::ChatSingleCast { .. } => opcode::CHAT_SINGLE_CAST,
            BusMessage::SetCustomData { .. } => opcode::SET_CUSTOM_DATA,
            BusMessage::SetCustomDataRe { .. } => opcode::SET_CUSTOM_DATA_RE,
        }
    }

    /// O jogador a que a mensagem se refere. Toda mensagem do barramento tem um: é
    /// justamente o que a distingue do que trafega com o cliente.
    pub fn roleid(&self) -> i32 {
        match self {
            BusMessage::ClientToGame { roleid, .. }
            | BusMessage::GameToClient { roleid, .. }
            | BusMessage::EnterWorld { roleid, .. }
            | BusMessage::PlayerLogout { roleid, .. }
            | BusMessage::SetCustomData { roleid, .. }
            | BusMessage::SetCustomDataRe { roleid, .. } => *roleid,
            BusMessage::ChatSingleCast { dstroleid, .. } => *dstroleid,
        }
    }

    /// Escreve só o corpo, na ordem do IR. O enquadramento fica com o codec.
    pub fn write_payload(&self, w: &mut Writer) {
        match self {
            BusMessage::ClientToGame {
                roleid,
                localsid,
                data,
            }
            | BusMessage::GameToClient {
                roleid,
                localsid,
                data,
            } => {
                w.i32(*roleid);
                w.u32(*localsid);
                w.octets(data);
            }
            BusMessage::EnterWorld {
                roleid,
                provider_link_id,
                locktime,
                timeout,
                settime,
                localsid,
            } => {
                w.i32(*roleid);
                w.i32(*provider_link_id);
                w.i32(*locktime);
                w.i32(*timeout);
                w.i32(*settime);
                w.u32(*localsid);
            }
            BusMessage::PlayerLogout {
                result,
                roleid,
                provider_link_id,
                localsid,
            } => {
                w.i32(*result);
                w.i32(*roleid);
                w.i32(*provider_link_id);
                w.u32(*localsid);
            }
            BusMessage::ChatSingleCast { channel, emotion, srcroleid, dstroleid, dstlocalsid, msg, data } => {
                w.u8(*channel);
                w.u8(*emotion);
                w.i32(*srcroleid);
                w.i32(*dstroleid);
                w.u32(*dstlocalsid);
                w.octets(msg);
                w.octets(data);
            }
            BusMessage::SetCustomData { roleid, localsid, custom_data } => {
                w.i32(*roleid);
                w.u32(*localsid);
                w.octets(custom_data);
            }
            BusMessage::SetCustomDataRe { result, crc, roleid, localsid } => {
                w.i32(*result);
                w.u32(*crc);
                w.i32(*roleid);
                w.u32(*localsid);
            }
        }
    }

    /// Lê o corpo de uma mensagem, dado o opcode.
    ///
    /// Um opcode que não é do barramento vira `Ok(None)` — não é erro de fio, é uma
    /// mensagem que este daemon não trata, e quem chama decide o que fazer.
    pub fn read_payload(opcode: u32, r: &mut Reader) -> Result<Option<Self>, WireError> {
        let msg = match opcode {
            opcode::C2S_GAMEDATA_SEND | opcode::S2C_GAMEDATA_SEND => {
                let roleid = r.i32()?;
                let localsid = r.u32()?;
                let data = r.octets()?.to_vec();
                if opcode == opcode::C2S_GAMEDATA_SEND {
                    BusMessage::ClientToGame {
                        roleid,
                        localsid,
                        data,
                    }
                } else {
                    BusMessage::GameToClient {
                        roleid,
                        localsid,
                        data,
                    }
                }
            }
            opcode::ENTER_WORLD => BusMessage::EnterWorld {
                roleid: r.i32()?,
                provider_link_id: r.i32()?,
                locktime: r.i32()?,
                timeout: r.i32()?,
                settime: r.i32()?,
                localsid: r.u32()?,
            },
            opcode::CHAT_SINGLE_CAST => BusMessage::ChatSingleCast {
                channel: r.u8()?,
                emotion: r.u8()?,
                srcroleid: r.i32()?,
                dstroleid: r.i32()?,
                dstlocalsid: r.u32()?,
                msg: r.octets()?.to_vec(),
                data: r.octets()?.to_vec(),
            },
            opcode::SET_CUSTOM_DATA => BusMessage::SetCustomData {
                roleid: r.i32()?,
                localsid: r.u32()?,
                custom_data: r.octets()?.to_vec(),
            },
            opcode::SET_CUSTOM_DATA_RE => BusMessage::SetCustomDataRe {
                result: r.i32()?,
                crc: r.u32()?,
                roleid: r.i32()?,
                localsid: r.u32()?,
            },
            opcode::PLAYER_LOGOUT => BusMessage::PlayerLogout {
                result: r.i32()?,
                roleid: r.i32()?,
                provider_link_id: r.i32()?,
                localsid: r.u32()?,
            },
            _ => return Ok(None),
        };
        Ok(Some(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ida_e_volta(m: BusMessage) {
        let mut w = Writer::new();
        m.write_payload(&mut w);
        let bytes = w.into_vec();
        let mut r = Reader::new(&bytes);
        let lido = BusMessage::read_payload(m.opcode(), &mut r)
            .expect("falha de fio")
            .expect("opcode não reconhecido");
        assert_eq!(lido, m);
        assert_eq!(r.remaining(), 0, "sobraram bytes");
    }

    #[test]
    fn as_quatro_mensagens_fazem_ida_e_volta() {
        ida_e_volta(BusMessage::ClientToGame {
            roleid: 1024,
            localsid: 0xDEAD_BEEF,
            data: vec![1, 2, 3],
        });
        ida_e_volta(BusMessage::ChatSingleCast {
            channel: 0,
            emotion: 0,
            srcroleid: -2147476694,
            dstroleid: 11457,
            dstlocalsid: 3,
            msg: "Morra!".encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
            data: vec![],
        });
        ida_e_volta(BusMessage::SetCustomData { roleid: 7, localsid: 8, custom_data: vec![1; 176] });
        ida_e_volta(BusMessage::SetCustomDataRe { result: 201, crc: 0xABCD, roleid: 7, localsid: 8 });
        ida_e_volta(BusMessage::GameToClient {
            roleid: -1,
            localsid: 0,
            data: Vec::new(),
        });
        ida_e_volta(BusMessage::EnterWorld {
            roleid: 7,
            provider_link_id: 2,
            locktime: 30,
            timeout: 60,
            settime: 1,
            localsid: 9,
        });
        ida_e_volta(BusMessage::PlayerLogout {
            result: 0,
            roleid: 7,
            provider_link_id: 2,
            localsid: 9,
        });
    }

    #[test]
    fn os_dois_sentidos_do_gamedata_tem_o_mesmo_corpo_e_opcodes_diferentes() {
        // O par 74/75 carrega exatamente os mesmos campos; o que diz o sentido é o
        // opcode. Ler um como se fosse o outro não quebraria no fio — quebraria na
        // lógica, que é pior.
        let ida = BusMessage::ClientToGame {
            roleid: 5,
            localsid: 6,
            data: vec![9],
        };
        let volta = BusMessage::GameToClient {
            roleid: 5,
            localsid: 6,
            data: vec![9],
        };
        let mut a = Writer::new();
        ida.write_payload(&mut a);
        let mut b = Writer::new();
        volta.write_payload(&mut b);
        assert_eq!(a.as_slice(), b.as_slice(), "os corpos são iguais");
        assert_ne!(ida.opcode(), volta.opcode(), "os opcodes é que separam");
    }

    #[test]
    fn opcode_de_fora_do_barramento_nao_e_erro_de_fio() {
        // O `GamedataSend` do cliente (34) não pertence a este barramento: falta o dono.
        let bytes = [0u8; 16];
        let mut r = Reader::new(&bytes);
        assert_eq!(BusMessage::read_payload(34, &mut r).unwrap(), None);
    }

    #[test]
    fn o_payload_do_mundo_e_opaco_para_o_barramento() {
        // O `data` é little-endian e `pack(1)`; o envelope é big-endian. O barramento
        // não interpreta os bytes, só os carrega — e é isso que impede a troca de um
        // formato pelo outro.
        let data = vec![0x01, 0x00, 0xFF, 0xFE];
        let m = BusMessage::ClientToGame {
            roleid: 1,
            localsid: 2,
            data: data.clone(),
        };
        let mut w = Writer::new();
        m.write_payload(&mut w);
        let bytes = w.into_vec();
        // roleid (4, big-endian) + localsid (4) + CompactUINT(4) + os 4 bytes crus
        assert_eq!(&bytes[..4], &[0, 0, 0, 1], "o envelope é big-endian");
        assert_eq!(&bytes[bytes.len() - 4..], &data[..], "o payload passa intacto");
    }
}
