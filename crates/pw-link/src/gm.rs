//! B363 — os comandos GNET de GM no link (papel do `glinkd` + `gdeliveryd`), com o IR `specs/protocol/gnet_155.json`:
//!
//! | opcode | protocolo | o que faz (`cnet/gdeliveryd/*.hpp`, `cnet/glinkd/*.hpp`) |
//! | ---: | :--- | :--- |
//! | 350 | `GMOnlineNum {gmroleid; u32 localsid}` | `GMOnlineNum_Re(gmroleid, localsid, UserContainer.Size(), 0)` (351) |
//! | 354 | `GMKickoutUser {gmroleid; localsid; kickuserid; forbid_time; octets reason}` | o `kickuserid` é um **roleid** (`Roleid2Uid`): com `forbid_time ≥ 1` derruba a conta e, `> 0`, proíbe a entrada; `_Re` (355) |
//! | 356 | `GMShutup {gmroleid; u32 localsid; dstuserid; forbid_time; reason}` | cala a **conta** do roleid (`ForbidUserTalk`); `_Re` (357) `{retcode; dstuserid; forbid_time}` |
//! | 360 | `GMKickoutRole {gmroleid; localsid; kickroleid; forbid_time; reason}` | `forbid_time ≤ 0` tira a proibição, `> 0` proíbe e derruba (`KickoutUser(…, ERR_KICKOUT)`); `_Re` (361) |
//! | 362 | `GMShutupRole {gmroleid; u32 localsid; dstroleid; forbid_time; reason}` | cala o **personagem** (`shutuprolemap`); `_Re` (363) `{retcode; dstroleid; forbid_time}` |
//! | 364 | `GMToggleChat {gmroleid; localsid; i8 enable}` | `MasterContainer.SetSilent(gmroleid, !enable)`; `_Re` (365) |
//!
//! O `glinkd` confere o privilégio (`PrivilegeCheck`: `PRV_LISTUSER` 11, `PRV_FORCE_OFFLINE` 100, `PRV_FORBID_TALK` 101,
//! `gdeliveryd/gamemaster.h:25-27`) e, sem ele, **ignora** o pedido. Aqui, como no resto do projeto, vale o nível de GM
//! da conta (`sec_level`, coluna `gm_privileges`): acima de zero, todos. Derrubar manda `ErrorInfo(ERR_KICKOUT 32,
//! "Be kicked out.")` e fecha (`GLinkServer::SessionError`, `glinkserver.cpp:120-125`). O chat de quem está calado
//! responde `AnnounceForbidInfo` (123, `{userid; u32 localsid; GRoleForbid forbid; char disconnect}`) com o tempo que
//! falta e não é repassado (`publicchat.hpp`, `IsForbidChat`).
//!
//! **Falta:** gravar a proibição no banco (`RPC_PUTROLEFORBID`/`SendDBForbidUser`): hoje vale até o link reiniciar;
//! `GMListOnlineUser` (352, a paginação `_HANDLE_*` não está no fonte); `GMForbidRole` (366).

use crate::gateway::LinkGateway;
use crate::uplink::EnvioAoCliente;
use pw_protocol::codec::OutboundPacket;
use pw_protocol::OctetsStream;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

pub const OP_GM_ONLINE_NUM: u32 = 350;
pub const OP_GM_ONLINE_NUM_RE: u32 = 351;
pub const OP_GM_KICKOUT_USER: u32 = 354;
pub const OP_GM_KICKOUT_USER_RE: u32 = 355;
pub const OP_GM_SHUTUP: u32 = 356;
pub const OP_GM_SHUTUP_RE: u32 = 357;
pub const OP_GM_KICKOUT_ROLE: u32 = 360;
pub const OP_GM_KICKOUT_ROLE_RE: u32 = 361;
pub const OP_GM_SHUTUP_ROLE: u32 = 362;
pub const OP_GM_SHUTUP_ROLE_RE: u32 = 363;
pub const OP_GM_TOGGLE_CHAT: u32 = 364;
pub const OP_GM_TOGGLE_CHAT_RE: u32 = 365;
pub const OP_ANNOUNCE_FORBID_INFO: u32 = 123;
/// `ERR_KICKOUT` (`share/rpc/errcode.h:45`).
pub const ERR_KICKOUT: i32 = 32;
/// `Privilege::PRV_FORBID_TALK` — o `type` do `GRoleForbid` que o `glinkd` guarda para o calado.
pub const PRV_FORBID_TALK: u8 = 101;
/// `_SID_INVALID`.
const SID_INVALIDO: u32 = 0xFFFF_FFFF;

/// `{int gmroleid; int localsid; int alvo; int forbid_time; octets reason}` (354, 356, 360, 362).
#[derive(Debug, Clone, PartialEq)]
pub struct PedidoDeGm {
    pub gmroleid: i32,
    pub localsid: u32,
    pub alvo: i32,
    pub tempo_s: i32,
    pub motivo: Vec<u8>,
}

pub fn ler_pedido(p: &[u8]) -> Option<PedidoDeGm> {
    let mut s = OctetsStream::from_bytes(p);
    Some(PedidoDeGm {
        gmroleid: s.read_i32().ok()?,
        localsid: s.read_u32().ok()?,
        alvo: s.read_i32().ok()?,
        tempo_s: s.read_i32().ok()?,
        motivo: s.read_octets().ok()?,
    })
}

fn bytes(s: OctetsStream) -> Vec<u8> {
    s.into_bytes().to_vec()
}

/// `GMOnlineNum_Re {gmroleid; u32 localsid; total_num; local_num}`.
pub fn online_num_re(gmroleid: i32, localsid: u32, total: i32, local: i32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i32(gmroleid);
    s.write_u32(localsid);
    s.write_i32(total);
    s.write_i32(local);
    bytes(s)
}

/// `GMKickoutUser_Re`/`GMKickoutRole_Re {retcode; gmroleid; localsid; alvo}`.
pub fn kickout_re(retcode: i32, gmroleid: i32, localsid: u32, alvo: i32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i32(retcode);
    s.write_i32(gmroleid);
    s.write_u32(localsid);
    s.write_i32(alvo);
    bytes(s)
}

/// `GMShutup_Re`/`GMShutupRole_Re {retcode; alvo; forbid_time}`.
pub fn shutup_re(retcode: i32, alvo: i32, tempo_s: i32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i32(retcode);
    s.write_i32(alvo);
    s.write_i32(tempo_s);
    bytes(s)
}

/// `GMToggleChat_Re {retcode; gmroleid; localsid; i8 enable}`.
pub fn toggle_chat_re(retcode: i32, gmroleid: i32, localsid: u32, ligado: bool) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i32(retcode);
    s.write_i32(gmroleid);
    s.write_u32(localsid);
    s.write_i8(ligado as i8);
    bytes(s)
}

/// `AnnounceForbidInfo {userid; u32 localsid; GRoleForbid {u8 type; int time; int createtime; octets reason}; char
/// disconnect}` (`rpcalls.xml:626-631`).
pub fn announce_forbid_info(roleid: i32, tipo: u8, falta_s: i32, motivo: &[u8]) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i32(roleid);
    s.write_u32(SID_INVALIDO);
    s.write_u8(tipo);
    s.write_i32(falta_s);
    s.write_i32(0);
    s.write_octets(motivo);
    s.write_i8(0);
    bytes(s)
}

/// Uma proibição com prazo: o fim e o motivo.
#[derive(Debug, Clone)]
pub struct Proibicao {
    pub ate: Instant,
    pub motivo: Vec<u8>,
}

impl Proibicao {
    fn nova(tempo_s: i32, motivo: Vec<u8>) -> Self {
        Self { ate: Instant::now() + Duration::from_secs(tempo_s.max(0) as u64), motivo }
    }
    pub fn falta_s(&self) -> i32 {
        self.ate.saturating_duration_since(Instant::now()).as_secs() as i32
    }
}

/// O que o link guarda dos comandos de GM (o `shutuprolemap`/`shutupusermap` do `glinkd`, o `ForbidRoleLogin` e o
/// `MasterContainer` do `gdeliveryd`).
#[derive(Debug, Default)]
pub struct EstadoDeGm {
    pub calados_por_role: RwLock<HashMap<i32, Proibicao>>,
    pub calados_por_conta: RwLock<HashMap<i64, Proibicao>>,
    pub sem_entrar_por_role: RwLock<HashMap<i32, Proibicao>>,
    pub sem_entrar_por_conta: RwLock<HashMap<i64, Proibicao>>,
    pub gms_silenciosos: RwLock<HashSet<i32>>,
}

impl EstadoDeGm {
    /// `IsForbidChat(role, userid)`: a da conta primeiro, depois a do personagem; vencidas saem.
    pub async fn calado(&self, roleid: i32, conta: Option<i64>) -> Option<Proibicao> {
        if let Some(c) = conta {
            let mut m = self.calados_por_conta.write().await;
            match m.get(&c) {
                Some(p) if p.falta_s() > 0 => return Some(p.clone()),
                Some(_) => {
                    m.remove(&c);
                }
                None => {}
            }
        }
        let mut m = self.calados_por_role.write().await;
        match m.get(&roleid) {
            Some(p) if p.falta_s() > 0 => Some(p.clone()),
            Some(_) => {
                m.remove(&roleid);
                None
            }
            None => None,
        }
    }

    /// `ForbidRoleLogin` / a proibição de conta: `true` quando ainda vale.
    pub async fn sem_entrar(&self, roleid: Option<i32>, conta: Option<i64>) -> bool {
        if let Some(c) = conta {
            if self.sem_entrar_por_conta.read().await.get(&c).is_some_and(|p| p.falta_s() > 0) {
                return true;
            }
        }
        if let Some(r) = roleid {
            if self.sem_entrar_por_role.read().await.get(&r).is_some_and(|p| p.falta_s() > 0) {
                return true;
            }
        }
        false
    }
}

fn mandar(envio: &EnvioAoCliente, opcode: u32, payload: Vec<u8>) {
    let _ = envio.try_send(OutboundPacket::Raw { opcode, payload });
}

impl LinkGateway {
    /// B363 — um pacote de GM vindo do cliente. `true` se era de GM (tratado ou recusado).
    pub async fn gm_pacote(&self, sessao: &crate::session::ClientSession, tx: &EnvioAoCliente, opcode: u32, p: &[u8]) -> bool {
        if !matches!(
            opcode,
            OP_GM_ONLINE_NUM | OP_GM_KICKOUT_USER | OP_GM_SHUTUP | OP_GM_KICKOUT_ROLE | OP_GM_SHUTUP_ROLE | OP_GM_TOGGLE_CHAT
        ) {
            return false;
        }
        let Some(eu) = sessao.role_id else { return true };
        // `PrivilegeCheck`: sem o privilégio, o `glinkd` registra e ignora.
        if sessao.sec_level == 0 {
            tracing::warn!("gm: {eu} sem privilégio tentou o protocolo {opcode}");
            return true;
        }
        match opcode {
            OP_GM_ONLINE_NUM => {
                let mut s = OctetsStream::from_bytes(p);
                let (Ok(gm), Ok(_)) = (s.read_i32(), s.read_u32()) else { return true };
                let total = self.jogadores_online().await as i32;
                // `this->localsid = sid` no `glinkd`: a resposta leva a sessão de quem pediu.
                mandar(tx, OP_GM_ONLINE_NUM_RE, online_num_re(gm, sessao.localsid, total, 0));
            }
            OP_GM_TOGGLE_CHAT => {
                let mut s = OctetsStream::from_bytes(p);
                let (Ok(gm), Ok(lsid), Ok(ligado)) = (s.read_i32(), s.read_u32(), s.read_i8()) else { return true };
                {
                    let mut m = self.gm.gms_silenciosos.write().await;
                    if ligado != 0 {
                        m.remove(&gm);
                    } else {
                        m.insert(gm);
                    }
                }
                mandar(tx, OP_GM_TOGGLE_CHAT_RE, toggle_chat_re(0, gm, lsid, ligado != 0));
            }
            _ => {
                let Some(pedido) = ler_pedido(p) else { return true };
                info!("gm: {eu} — protocolo {opcode} em {} por {} s", pedido.alvo, pedido.tempo_s);
                match opcode {
                    OP_GM_KICKOUT_ROLE => {
                        let mut m = self.gm.sem_entrar_por_role.write().await;
                        if pedido.tempo_s <= 0 {
                            m.remove(&pedido.alvo);
                        } else {
                            m.insert(pedido.alvo, Proibicao::nova(pedido.tempo_s, pedido.motivo.clone()));
                        }
                        drop(m);
                        if pedido.tempo_s > 0 {
                            self.derrubar_role(pedido.alvo).await;
                        }
                        mandar(tx, OP_GM_KICKOUT_ROLE_RE, kickout_re(0, pedido.gmroleid, pedido.localsid, pedido.alvo));
                    }
                    OP_GM_KICKOUT_USER => {
                        let conta = self.conta_do_role(pedido.alvo).await;
                        if pedido.tempo_s >= 1 {
                            self.derrubar_role(pedido.alvo).await;
                        }
                        if let (Some(c), true) = (conta, pedido.tempo_s > 0) {
                            self.gm.sem_entrar_por_conta.write().await.insert(c, Proibicao::nova(pedido.tempo_s, pedido.motivo.clone()));
                        }
                        let alvo = conta.map_or(pedido.alvo, |c| c as i32);
                        mandar(tx, OP_GM_KICKOUT_USER_RE, kickout_re(0, pedido.gmroleid, pedido.localsid, alvo));
                    }
                    OP_GM_SHUTUP_ROLE => {
                        self.gm.calados_por_role.write().await.insert(pedido.alvo, Proibicao::nova(pedido.tempo_s, pedido.motivo.clone()));
                        mandar(tx, OP_GM_SHUTUP_ROLE_RE, shutup_re(0, pedido.alvo, pedido.tempo_s));
                    }
                    OP_GM_SHUTUP => {
                        // `dstuserid = Roleid2Uid(dstuserid)`; sem conta, nada.
                        if let Some(c) = self.conta_do_role(pedido.alvo).await {
                            self.gm.calados_por_conta.write().await.insert(c, Proibicao::nova(pedido.tempo_s, pedido.motivo.clone()));
                            mandar(tx, OP_GM_SHUTUP_RE, shutup_re(0, pedido.alvo, pedido.tempo_s));
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }

    /// `KickoutUser(userid, localsid, ERR_KICKOUT)` → `SessionError`: o aviso e o fim da sessão do personagem.
    pub async fn derrubar_role(&self, roleid: i32) {
        if let Some(tx) = self.jogadores_visiveis.read().await.get(&roleid) {
            let _ = tx.try_send(OutboundPacket::Derrubar { codigo: ERR_KICKOUT, info: "Be kicked out.".into() });
        }
    }

    async fn conta_do_role(&self, roleid: i32) -> Option<i64> {
        self.char_repo.get_details_por_role(roleid).await.ok().flatten().map(|d| d.account_id as i64)
    }

    async fn jogadores_online(&self) -> usize {
        self.jogadores_visiveis.read().await.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Os layouts do IR (`gnet_155.json`): o pedido de 4 inteiros + octets e as respostas.
    #[test]
    fn layouts_dos_protocolos_de_gm() {
        let mut s = OctetsStream::new();
        for v in [7i32, 9, 1024, 600] {
            s.write_i32(v);
        }
        s.write_octets(b"spam");
        let p = ler_pedido(&s.into_bytes()).unwrap();
        assert_eq!((p.gmroleid, p.localsid, p.alvo, p.tempo_s, p.motivo.as_slice()), (7, 9, 1024, 600, &b"spam"[..]));
        assert_eq!(online_num_re(7, 9, 3, 0), [0, 0, 0, 7, 0, 0, 0, 9, 0, 0, 0, 3, 0, 0, 0, 0]);
        assert_eq!(kickout_re(0, 7, 9, 1024).len(), 16);
        assert_eq!(shutup_re(0, 1024, 600), [0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 2, 88]);
        assert_eq!(toggle_chat_re(0, 7, 9, true).len(), 13);
        // `AnnounceForbidInfo`: 4 + 4 + (1 + 4 + 4 + 1 + 4) + 1.
        assert_eq!(announce_forbid_info(1024, PRV_FORBID_TALK, 30, b"spam").len(), 23);
    }

    /// Com o banco: o GM conta os online (351), cala o personagem (363 e o `calado`), cala a conta (357), derruba e
    /// proíbe a entrada (361 + `Derrubar` no alvo + `sem_entrar`); quem não é GM é ignorado.
    #[tokio::test]
    async fn comandos_de_gm_no_link() {
        use pw_core::{CharacterClass, Gender, Race};
        use pw_storage::{AccountRepository, CacheManager, CharacterRepository, PostgresPool, StorageConfig};
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("sem TEST_DATABASE_URL — teste ignorado");
            return;
        };
        let cfg = StorageConfig { database_url: url, max_connections: 3, min_connections: 1, ..Default::default() };
        let pool = PostgresPool::new(&cfg).await.expect("banco");
        let m = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;
        let realm = format!("t_gm_{m}");
        sqlx::query("INSERT INTO realms (id, name, version, host, port, max_players, config) VALUES ($1, 'Teste', '1.5.5', '127.0.0.1', 1, 10, '{}'::jsonb)")
            .bind(&realm)
            .execute(pool.get_ref())
            .await
            .unwrap();
        let conta: i32 = sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
            .bind(format!("gm_{m}"))
            .fetch_one(pool.get_ref())
            .await
            .unwrap();
        let repo = CharacterRepository::new(pool.clone());
        let alvo = repo
            .create_character(conta, &realm, &format!("Alvo{m}"), Race::Human, CharacterClass::Wizard, Gender::Female, Vec::new(), None)
            .await
            .unwrap();
        let cache = CacheManager::new(&StorageConfig::default()).await.expect("cache");
        let gw = LinkGateway::new(format!("sem_dados_{m}"), "1.5.5", 0, AccountRepository::new(pool.clone()), repo, cache);
        let (tx_gm, mut rx_gm) = tokio::sync::mpsc::channel(16);
        let (tx_alvo, mut rx_alvo) = tokio::sync::mpsc::channel(16);
        gw.jogadores_visiveis.write().await.insert(alvo, tx_alvo);
        let mut gm = crate::session::ClientSession::new(1, "127.0.0.1".into(), realm.clone(), "1.5.5".into());
        gm.set_in_world(777, "GM".into());
        gm.localsid = 5;
        let pedido = |alvo: i32, t: i32| {
            let mut s = OctetsStream::new();
            for v in [777, 5, alvo, t] {
                s.write_i32(v);
            }
            s.write_octets(b"teste");
            s.into_bytes().to_vec()
        };
        let resposta = |rx: &mut tokio::sync::mpsc::Receiver<OutboundPacket>| match rx.try_recv() {
            Ok(OutboundPacket::Raw { opcode, payload }) => Some((opcode, payload)),
            _ => None,
        };
        // Sem privilégio: tratado (é de GM), mas sem efeito nem resposta.
        assert!(gw.gm_pacote(&gm, &tx_gm, OP_GM_SHUTUP_ROLE, &pedido(alvo, 60)).await);
        assert!(resposta(&mut rx_gm).is_none() && gw.gm.calado(alvo, None).await.is_none());
        gm.sec_level = 1;
        let mut num = OctetsStream::new();
        num.write_i32(777);
        num.write_u32(5);
        assert!(gw.gm_pacote(&gm, &tx_gm, OP_GM_ONLINE_NUM, &num.into_bytes()).await);
        assert_eq!(resposta(&mut rx_gm), Some((OP_GM_ONLINE_NUM_RE, online_num_re(777, 5, 1, 0))));
        assert!(gw.gm_pacote(&gm, &tx_gm, OP_GM_SHUTUP_ROLE, &pedido(alvo, 60)).await);
        assert_eq!(resposta(&mut rx_gm), Some((OP_GM_SHUTUP_ROLE_RE, shutup_re(0, alvo, 60))));
        assert!(gw.gm.calado(alvo, None).await.is_some_and(|p| p.falta_s() > 50));
        assert!(gw.gm_pacote(&gm, &tx_gm, OP_GM_SHUTUP, &pedido(alvo, 30)).await);
        assert_eq!(resposta(&mut rx_gm), Some((OP_GM_SHUTUP_RE, shutup_re(0, alvo, 30))));
        assert!(gw.gm.calado(-1, Some(conta as i64)).await.is_some(), "a conta calada");
        // Derrubar e proibir; `forbid_time` 0 tira a proibição.
        assert!(gw.gm_pacote(&gm, &tx_gm, OP_GM_KICKOUT_ROLE, &pedido(alvo, 120)).await);
        assert_eq!(resposta(&mut rx_gm), Some((OP_GM_KICKOUT_ROLE_RE, kickout_re(0, 777, 5, alvo))));
        assert!(matches!(rx_alvo.try_recv(), Ok(OutboundPacket::Derrubar { codigo: ERR_KICKOUT, .. })));
        assert!(gw.gm.sem_entrar(Some(alvo), None).await);
        assert!(gw.gm_pacote(&gm, &tx_gm, OP_GM_KICKOUT_ROLE, &pedido(alvo, 0)).await);
        assert!(!gw.gm.sem_entrar(Some(alvo), None).await, "forbid_time 0 tira a proibição");
        assert!(rx_alvo.try_recv().is_err(), "0 não derruba");
    }
}
