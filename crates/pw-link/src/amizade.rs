//! B363 — a lista de amigos (o que o `gdeliveryd` faz em `cnet/gdeliveryd/addfriend.hpp`,
//! `addfriendrqst.hrp`, `delfriend.hpp`, `setfriendgroup.hpp`, `setgroupname.hpp`, `getfriends.hpp`,
//! `friendstatus.hpp`, `gametalkmanager.cpp:199-219`, `logoutroletask.h:43-56`).
//!
//! Os pacotes GNET (`specs/protocol/gnet_155.json`), em big-endian com `Octets` de tamanho compacto:
//! `AddFriend` 202 `{int srcroleid; int dstroleid; Octets dstname; u32 srclsid}`,
//! `AddFriend_Re` 203 `{char retcode; GFriendInfo info; u32 srclsid}`,
//! `AddFriendRqst` 204 (RPC: `xid`, pedido `{int srcroleid; Octets srcname; u32 dstlsid}`, resposta
//! `{char retcode}`), `SetGroupName` 208/209, `SetFriendGroup` 210/211, `DelFriend` 212/213,
//! `FriendStatus` 214 `{int roleid; char status; u32 localsid}` e `GetFriends`/`_Re` 206/207.
//!
//! Aqui só a forma dos pacotes e as regras puras; o `gateway.rs` faz a ponte com as sessões e o banco.

use pw_protocol::OctetsStream;
use pw_storage::{Amigo, ListaDeAmigos};

pub const OP_ADD_FRIEND: u32 = 202;
pub const OP_ADD_FRIEND_RE: u32 = 203;
pub const OP_ADD_FRIEND_RQST: u32 = 204;
pub const OP_SET_GROUP_NAME: u32 = 208;
pub const OP_SET_GROUP_NAME_RE: u32 = 209;
pub const OP_SET_FRIEND_GROUP: u32 = 210;
pub const OP_SET_FRIEND_GROUP_RE: u32 = 211;
pub const OP_DEL_FRIEND: u32 = 212;
pub const OP_DEL_FRIEND_RE: u32 = 213;
pub const OP_FRIEND_STATUS: u32 = 214;

/// `ERR_FS_*` (`share/rpc/errcode.h:220-227`).
pub mod erro {
    pub const OFFLINE: i8 = 1;
    pub const RECUSOU: i8 = 2;
    pub const TEMPO: i8 = 3;
    pub const SEM_ESPACO: i8 = 4;
    pub const NAO_ACHOU: i8 = 5;
    pub const PARAMETRO: i8 = 6;
    pub const REPETIDO: i8 = 7;
}

/// `GT_OFFLINE`/`GT_ONLINE_GAME` (`gametalkmanager.h:28-30`).
pub const OFFLINE: i8 = 0;
pub const ONLINE: i8 = 1;
/// `plist->size()>=100` (`addfriendrqst.hrp`) e `plist->size()>=10` (`setgroupname.hpp`).
pub const MAXIMO_DE_AMIGOS: usize = 100;
pub const MAXIMO_DE_GRUPOS: usize = 10;
/// `name.size()>20` → `ERR_FS_ERRPARAM` (`setgroupname.hpp`).
pub const NOME_DE_GRUPO_MAXIMO: usize = 20;
/// `AddFriendRqst`: `timeout 60` (o IR) → `ERR_FS_TIMEOUT` (`OnTimeout`).
pub const PRAZO_DO_PEDIDO_S: u64 = 60;
/// O bit de pedido do `XID` (`unsigned int count:31; unsigned int is_request:1`, `gnrpc.h:24-33`).
pub const XID_PEDIDO: u32 = 0x8000_0000;

fn utf16(texto: &str) -> Vec<u8> {
    texto.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn de_utf16(b: &[u8]) -> String {
    let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&u)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedidoDeAmizade {
    pub origem: i32,
    pub destino: i32,
    pub nome_do_destino: String,
    pub localsid: u32,
}

pub fn ler_add_friend(p: &[u8]) -> Option<PedidoDeAmizade> {
    let mut s = OctetsStream::from_bytes(p);
    Some(PedidoDeAmizade {
        origem: s.read_i32().ok()?,
        destino: s.read_i32().ok()?,
        nome_do_destino: de_utf16(&s.read_octets().ok()?),
        localsid: s.read_u32().ok()?,
    })
}

/// `DelFriend {int roleid; int friendid; u32 localsid}`.
pub fn ler_del_friend(p: &[u8]) -> Option<(i32, i32, u32)> {
    let mut s = OctetsStream::from_bytes(p);
    Some((s.read_i32().ok()?, s.read_i32().ok()?, s.read_u32().ok()?))
}

/// `SetFriendGroup {char gid; int roleid; int friendid; u32 localsid}`.
pub fn ler_set_friend_group(p: &[u8]) -> Option<(i8, i32, i32, u32)> {
    let mut s = OctetsStream::from_bytes(p);
    Some((s.read_i8().ok()?, s.read_i32().ok()?, s.read_i32().ok()?, s.read_u32().ok()?))
}

/// `SetGroupName {int roleid; char gid; Octets name; u32 localsid}` — o nome em bytes (UTF-16).
pub fn ler_set_group_name(p: &[u8]) -> Option<(i32, i8, Vec<u8>, u32)> {
    let mut s = OctetsStream::from_bytes(p);
    Some((s.read_i32().ok()?, s.read_i8().ok()?, s.read_octets().ok()?, s.read_u32().ok()?))
}

/// A resposta do cliente ao `AddFriendRqst`: o `xid` (sem o bit de pedido) e o `retcode`.
pub fn ler_resposta_do_pedido(p: &[u8]) -> Option<(u32, i8)> {
    let mut s = OctetsStream::from_bytes(p);
    let xid = s.read_u32().ok()?;
    if xid & XID_PEDIDO != 0 {
        return None;
    }
    Some((xid, s.read_i8().ok()?))
}

fn info(s: &mut OctetsStream, a: &Amigo) {
    s.write_i32(a.id);
    s.write_i8(a.classe);
    s.write_i8(a.grupo);
    s.write_octets(&utf16(&a.nome));
}

pub fn add_friend_re(retcode: i8, amigo: &Amigo, localsid: u32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i8(retcode);
    info(&mut s, amigo);
    s.write_u32(localsid);
    s.into_bytes().to_vec()
}

/// O pedido ao destino (`Rpc::Call(RPC_ADDFRIENDRQST, &arg)`): `xid` com o bit de pedido e o argumento.
pub fn add_friend_rqst(xid: u32, origem: i32, nome_da_origem: &str, localsid_do_destino: u32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_u32(xid | XID_PEDIDO);
    s.write_i32(origem);
    s.write_octets(&utf16(nome_da_origem));
    s.write_u32(localsid_do_destino);
    s.into_bytes().to_vec()
}

pub fn del_friend_re(retcode: i8, roleid: i32, amigo: i32, localsid: u32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i8(retcode);
    s.write_i32(roleid);
    s.write_i32(amigo);
    s.write_u32(localsid);
    s.into_bytes().to_vec()
}

pub fn set_friend_group_re(retcode: i8, grupo: i8, roleid: i32, amigo: i32, localsid: u32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i8(retcode);
    s.write_i8(grupo);
    s.write_i32(roleid);
    s.write_i32(amigo);
    s.write_u32(localsid);
    s.into_bytes().to_vec()
}

pub fn set_group_name_re(retcode: i8, roleid: i32, grupo: i8, nome: &[u8], localsid: u32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i8(retcode);
    s.write_i32(roleid);
    s.write_i8(grupo);
    s.write_octets(nome);
    s.write_u32(localsid);
    s.into_bytes().to_vec()
}

pub fn friend_status(roleid: i32, status: i8, localsid: u32) -> Vec<u8> {
    let mut s = OctetsStream::new();
    s.write_i32(roleid);
    s.write_i8(status);
    s.write_u32(localsid);
    s.into_bytes().to_vec()
}

/// `AddFriendRqst::Client` com a resposta aceita: o destino entra na lista da origem (ou, se já
/// estava, tem classe, nome e grupo 0 refeitos). `Err` com o `ERR_FS_*`.
pub fn aceitar(lista: &mut ListaDeAmigos, destino: i32, classe: i8, nome: &str) -> Result<(), i8> {
    if let Some(a) = lista.amigos.iter_mut().find(|a| a.id == destino) {
        a.classe = classe;
        a.grupo = 0;
        a.nome = nome.to_string();
        return Ok(());
    }
    if lista.amigos.len() >= MAXIMO_DE_AMIGOS {
        return Err(erro::SEM_ESPACO);
    }
    lista.amigos.push(Amigo { id: destino, classe, grupo: 0, nome: nome.to_string() });
    Ok(())
}

/// `DelFriend::Process`: `ERR_FS_NOFOUND` se não está na lista.
pub fn remover(lista: &mut ListaDeAmigos, amigo: i32) -> i8 {
    match lista.amigos.iter().position(|a| a.id == amigo) {
        Some(i) => {
            lista.amigos.remove(i);
            0
        }
        None => erro::NAO_ACHOU,
    }
}

/// `SetFriendGroup::Process`.
pub fn mudar_de_grupo(lista: &mut ListaDeAmigos, amigo: i32, grupo: i8) -> i8 {
    match lista.amigos.iter_mut().find(|a| a.id == amigo) {
        Some(a) => {
            a.grupo = grupo;
            0
        }
        None => erro::NAO_ACHOU,
    }
}

/// `SetGroupName::Process`: nome acima de 20 bytes ou grupo 0 → parâmetro; nome vazio apaga o grupo e
/// devolve os amigos dele ao 0; grupo existente troca o nome; grupo novo entra se houver menos de 10.
pub fn nomear_grupo(lista: &mut ListaDeAmigos, grupo: i8, nome: &[u8]) -> i8 {
    if nome.len() > NOME_DE_GRUPO_MAXIMO || grupo == 0 {
        return erro::PARAMETRO;
    }
    if let Some(i) = lista.grupos.iter().position(|(g, _)| *g == grupo) {
        if nome.is_empty() {
            lista.grupos.remove(i);
            for a in lista.amigos.iter_mut().filter(|a| a.grupo == grupo) {
                a.grupo = 0;
            }
        } else {
            lista.grupos[i].1 = de_utf16(nome);
        }
        return 0;
    }
    if lista.grupos.len() >= MAXIMO_DE_GRUPOS {
        return erro::SEM_ESPACO;
    }
    lista.grupos.push((grupo, de_utf16(nome)));
    0
}

/// O `status` do `GetFriends_Re` (`getfriends.hpp`): os pares `(índice, status)` dos amigos online
/// que também têm o jogador na lista. `tem_o_jogador(id)` diz se o amigo está online e o tem.
pub fn status_da_lista(lista: &ListaDeAmigos, tem_o_jogador: impl Fn(i32) -> bool) -> Vec<i8> {
    let mut v = Vec::new();
    for (i, a) in lista.amigos.iter().enumerate() {
        if tem_o_jogador(a.id) {
            v.push(i as i8);
            v.push(ONLINE);
        }
    }
    v
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lista() -> ListaDeAmigos {
        ListaDeAmigos {
            grupos: vec![(1, "Guilda".into())],
            amigos: vec![
                Amigo { id: 10, classe: 1, grupo: 1, nome: "Ana".into() },
                Amigo { id: 20, classe: 4, grupo: 0, nome: "Bia".into() },
            ],
        }
    }

    #[test]
    fn os_pacotes_tem_a_forma_do_ir() {
        let mut s = OctetsStream::new();
        s.write_i32(7);
        s.write_i32(0);
        s.write_octets(&utf16("Lu"));
        s.write_u32(99);
        assert_eq!(
            ler_add_friend(s.as_slice()),
            Some(PedidoDeAmizade { origem: 7, destino: 0, nome_do_destino: "Lu".into(), localsid: 99 })
        );
        let r = add_friend_re(0, &lista().amigos[0], 5);
        // retcode, rid (BE), cls, gid, octets(6 B: compacto 1 + "Ana" em UTF-16), localsid.
        assert_eq!(r.len(), 1 + 4 + 1 + 1 + 1 + 6 + 4);
        assert_eq!(&r[1..5], &10i32.to_be_bytes());
        let p = add_friend_rqst(3, 7, "Lu", 42);
        assert_eq!(u32::from_be_bytes(p[0..4].try_into().unwrap()), 3 | XID_PEDIDO);
        assert_eq!(friend_status(7, ONLINE, 9), [0, 0, 0, 7, 1, 0, 0, 0, 9]);
        let mut resp = 3u32.to_be_bytes().to_vec();
        resp.push(0);
        assert_eq!(ler_resposta_do_pedido(&resp), Some((3, 0)));
        assert_eq!(ler_resposta_do_pedido(&p[..5]), None, "o próprio pedido não é resposta");
    }

    #[test]
    fn as_regras_da_lista_sao_as_do_gdeliveryd() {
        let mut l = lista();
        assert_eq!(aceitar(&mut l, 30, 2, "Cid"), Ok(()));
        assert_eq!(l.amigos.len(), 3);
        assert_eq!(aceitar(&mut l, 10, 3, "Ana2"), Ok(()), "repetido refaz a entrada");
        assert_eq!((l.amigos[0].classe, l.amigos[0].grupo, l.amigos.len()), (3, 0, 3));
        assert_eq!(remover(&mut l, 99), erro::NAO_ACHOU);
        assert_eq!(remover(&mut l, 30), 0);
        assert_eq!(mudar_de_grupo(&mut l, 20, 1), 0);
        assert_eq!(nomear_grupo(&mut l, 0, &utf16("x")), erro::PARAMETRO);
        assert_eq!(nomear_grupo(&mut l, 2, &[0u8; 22]), erro::PARAMETRO, "mais de 20 bytes");
        assert_eq!(nomear_grupo(&mut l, 2, &utf16("Novo")), 0);
        assert_eq!(nomear_grupo(&mut l, 1, &[]), 0, "nome vazio apaga");
        assert_eq!(l.grupos, vec![(2, "Novo".to_string())]);
        assert!(l.amigos.iter().all(|a| a.grupo == 0), "os do grupo apagado voltam ao 0");
        let mut cheia = ListaDeAmigos::default();
        for i in 0..100 {
            cheia.amigos.push(Amigo { id: i, classe: 0, grupo: 0, nome: String::new() });
        }
        assert_eq!(aceitar(&mut cheia, 500, 0, "x"), Err(erro::SEM_ESPACO));
        assert_eq!(status_da_lista(&lista(), |id| id == 20), vec![1, ONLINE]);
    }
}

// =============================================================================
// A ponte com as sessões e o banco (o papel do `gdeliveryd`).
// =============================================================================

use crate::gateway::LinkGateway;
use crate::uplink::EnvioAoCliente;
use pw_protocol::OutboundPacket;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Um jogador online neste link (o `PlayerInfo` do `UserContainer`).
#[derive(Clone)]
pub struct Presenca {
    pub envio: EnvioAoCliente,
    pub localsid: u32,
    pub nome: String,
    pub classe: i8,
}

/// Um `AddFriendRqst` esperando a resposta do destino.
#[derive(Clone)]
pub struct Pendente {
    pub origem: i32,
    pub envio_da_origem: EnvioAoCliente,
    pub localsid_da_origem: u32,
    pub destino: i32,
    pub classe_do_destino: i8,
    pub nome_do_destino: String,
    pub envio_do_destino: EnvioAoCliente,
    pub localsid_do_destino: u32,
}

/// O estado da amizade no link.
#[derive(Default)]
pub struct Amizades {
    pub online: tokio::sync::RwLock<HashMap<i32, Presenca>>,
    pub pendentes: Arc<tokio::sync::Mutex<HashMap<u32, Pendente>>>,
    pub proximo_xid: std::sync::atomic::AtomicU32,
}

fn mandar(envio: &EnvioAoCliente, opcode: u32, payload: Vec<u8>) {
    let _ = envio.try_send(OutboundPacket::Raw { opcode, payload });
}

impl LinkGateway {
    async fn lista(&self, roleid: i32) -> ListaDeAmigos {
        self.char_repo.lista_de_amigos(roleid).await.unwrap_or_else(|e| {
            warn!("amizade: não consegui ler a lista de {roleid}: {e}");
            ListaDeAmigos::default()
        })
    }

    async fn gravar(&self, roleid: i32, lista: &ListaDeAmigos) {
        if let Err(e) = self.char_repo.gravar_lista_de_amigos(roleid, lista).await {
            warn!("amizade: não consegui gravar a lista de {roleid}: {e}");
        }
    }

    /// Entrou no mundo: fica online e avisa os amigos online que o têm na lista
    /// (`GameTalkManager::_SendStatusToFriends`, `gametalkmanager.cpp:199-219`).
    pub async fn amizade_ao_entrar(&self, roleid: i32, presenca: Presenca) {
        self.amizades.online.write().await.insert(roleid, presenca);
        let minha = self.lista(roleid).await;
        let online = self.amizades.online.read().await.clone();
        for a in &minha.amigos {
            let Some(p) = online.get(&a.id) else { continue };
            if self.lista(a.id).await.amigos.iter().any(|x| x.id == roleid) {
                mandar(&p.envio, OP_FRIEND_STATUS, friend_status(roleid, ONLINE, p.localsid));
            }
        }
    }

    /// Saiu: `LogoutRoleTask::Run` (`logoutroletask.h:43-56`) avisa cada amigo online da lista dele.
    pub async fn amizade_ao_sair(&self, roleid: i32) {
        if self.amizades.online.write().await.remove(&roleid).is_none() {
            return;
        }
        let minha = self.lista(roleid).await;
        let online = self.amizades.online.read().await;
        for a in &minha.amigos {
            if let Some(p) = online.get(&a.id) {
                mandar(&p.envio, OP_FRIEND_STATUS, friend_status(roleid, OFFLINE, p.localsid));
            }
        }
    }

    /// `GetFriends` (206) → `GetFriends_Re` (207) com a lista do banco e o `status` em pares.
    pub async fn amizade_lista(&self, roleid: i32) -> pw_protocol::S2CGetFriendListRe {
        let lista = self.lista(roleid).await;
        let online: Vec<i32> = self.amizades.online.read().await.keys().copied().collect();
        let mut tem_o_jogador = std::collections::HashSet::new();
        for a in lista.amigos.iter().filter(|a| online.contains(&a.id)) {
            if self.lista(a.id).await.amigos.iter().any(|x| x.id == roleid) {
                tem_o_jogador.insert(a.id);
            }
        }
        let status = status_da_lista(&lista, |id| tem_o_jogador.contains(&id));
        pw_protocol::S2CGetFriendListRe {
            role_id: roleid,
            groups: lista.grupos.iter().map(|(g, n)| pw_protocol::FriendGroup { gid: *g, name: n.clone() }).collect(),
            friends: lista
                .amigos
                .iter()
                .map(|a| pw_protocol::FriendEntry { rid: a.id, cls: a.classe, gid: a.grupo, name: a.nome.clone() })
                .collect(),
            status,
            localsid: 0,
        }
    }

    /// Os pacotes de amizade que chegam como `Unknown`. `true` se o `opcode` é de amizade.
    pub async fn amizade_pacote(&self, roleid: i32, tx: &EnvioAoCliente, opcode: u32, p: &[u8]) -> bool {
        match opcode {
            OP_ADD_FRIEND => self.adicionar(roleid, tx, p).await,
            OP_ADD_FRIEND_RQST => self.resposta_do_pedido(roleid, p).await,
            OP_DEL_FRIEND => {
                if let Some((dono, amigo, lsid)) = ler_del_friend(p).filter(|x| x.0 == roleid) {
                    let mut l = self.lista(dono).await;
                    let r = remover(&mut l, amigo);
                    if r == 0 {
                        self.gravar(dono, &l).await;
                    }
                    mandar(tx, OP_DEL_FRIEND_RE, del_friend_re(r, dono, amigo, lsid));
                }
            }
            OP_SET_FRIEND_GROUP => {
                if let Some((g, dono, amigo, lsid)) = ler_set_friend_group(p).filter(|x| x.1 == roleid) {
                    let mut l = self.lista(dono).await;
                    let r = mudar_de_grupo(&mut l, amigo, g);
                    if r == 0 {
                        self.gravar(dono, &l).await;
                    }
                    mandar(tx, OP_SET_FRIEND_GROUP_RE, set_friend_group_re(r, g, dono, amigo, lsid));
                }
            }
            OP_SET_GROUP_NAME => {
                if let Some((dono, g, nome, lsid)) = ler_set_group_name(p).filter(|x| x.0 == roleid) {
                    let mut l = self.lista(dono).await;
                    let r = nomear_grupo(&mut l, g, &nome);
                    if r == 0 {
                        self.gravar(dono, &l).await;
                    }
                    mandar(tx, OP_SET_GROUP_NAME_RE, set_group_name_re(r, dono, g, &nome, lsid));
                }
            }
            _ => return false,
        }
        true
    }

    /// `AddFriend::Process` (`cnet/gdeliveryd/addfriend.hpp`).
    async fn adicionar(&self, roleid: i32, tx: &EnvioAoCliente, p: &[u8]) {
        let Some(pedido) = ler_add_friend(p).filter(|x| x.origem == roleid) else { return };
        let online = self.amizades.online.read().await.clone();
        let destino = if pedido.destino > 0 {
            pedido.destino
        } else {
            online.iter().find(|(_, pr)| pr.nome == pedido.nome_do_destino).map(|(id, _)| *id).unwrap_or(0)
        };
        if destino == roleid {
            return;
        }
        let Some(origem) = online.get(&roleid).cloned() else { return };
        let Some(dst) = online.get(&destino).cloned().filter(|_| destino > 0) else {
            let a = Amigo { id: destino, classe: 0, grupo: 0, nome: pedido.nome_do_destino.clone() };
            mandar(tx, OP_ADD_FRIEND_RE, add_friend_re(erro::OFFLINE, &a, pedido.localsid));
            return;
        };
        if self.lista(roleid).await.amigos.iter().any(|a| a.id == destino && a.nome == dst.nome) {
            let a = Amigo { id: destino, classe: 0, grupo: 0, nome: dst.nome.clone() };
            mandar(tx, OP_ADD_FRIEND_RE, add_friend_re(erro::REPETIDO, &a, pedido.localsid));
            return;
        }
        let xid = self.amizades.proximo_xid.fetch_add(1, std::sync::atomic::Ordering::Relaxed) & !XID_PEDIDO;
        let pendente = Pendente {
            origem: roleid,
            envio_da_origem: origem.envio.clone(),
            localsid_da_origem: pedido.localsid,
            destino,
            classe_do_destino: dst.classe,
            nome_do_destino: dst.nome.clone(),
            envio_do_destino: dst.envio.clone(),
            localsid_do_destino: dst.localsid,
        };
        self.amizades.pendentes.lock().await.insert(xid, pendente);
        mandar(&dst.envio, OP_ADD_FRIEND_RQST, add_friend_rqst(xid, roleid, &origem.nome, dst.localsid));
        info!("amizade: {roleid} pediu amizade a {destino} (xid {xid})");
        // `OnTimeout` do RPC: 60 s sem resposta → `ERR_FS_TIMEOUT`.
        let pendentes = Arc::clone(&self.amizades.pendentes);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(PRAZO_DO_PEDIDO_S)).await;
            if let Some(p) = pendentes.lock().await.remove(&xid) {
                let a = Amigo { id: p.destino, classe: 0, grupo: 0, nome: p.nome_do_destino };
                mandar(&p.envio_da_origem, OP_ADD_FRIEND_RE, add_friend_re(erro::TEMPO, &a, p.localsid_da_origem));
            }
        });
    }

    /// `AddFriendRqst::Client` (`cnet/gdeliveryd/addfriendrqst.hrp`).
    async fn resposta_do_pedido(&self, roleid: i32, p: &[u8]) {
        let Some((xid, retcode)) = ler_resposta_do_pedido(p) else { return };
        let Some(pend) = self.amizades.pendentes.lock().await.remove(&xid) else { return };
        if pend.destino != roleid {
            return;
        }
        let ret = if retcode != 0 {
            erro::RECUSOU
        } else {
            let mut l = self.lista(pend.origem).await;
            match aceitar(&mut l, pend.destino, pend.classe_do_destino, &pend.nome_do_destino) {
                Ok(()) => {
                    self.gravar(pend.origem, &l).await;
                    0
                }
                Err(e) => e,
            }
        };
        let a = Amigo { id: pend.destino, classe: pend.classe_do_destino, grupo: 0, nome: pend.nome_do_destino.clone() };
        mandar(&pend.envio_da_origem, OP_ADD_FRIEND_RE, add_friend_re(ret, &a, pend.localsid_da_origem));
        if ret == 0 {
            // `dsm->Send(dstsid, inform)`: o destino fica sabendo do status da origem.
            mandar(&pend.envio_do_destino, OP_FRIEND_STATUS, friend_status(pend.origem, ONLINE, pend.localsid_do_destino));
            info!("amizade: {} entrou na lista de {}", pend.destino, pend.origem);
        }
    }
}
