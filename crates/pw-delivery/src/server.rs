use crate::chat::{ChatChannel, ChatMessage};
use crate::party::PartyMember;
use crate::service::DeliveryService;
use pw_core::{InventoryItem, RoleId, WorldId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum DeliveryRequest {
    SendChat(ChatMessage),
    SendMail {
        sender_id: Option<RoleId>,
        receiver_id: RoleId,
        title: String,
        message: String,
        attached_money: i64,
        attached_item: Option<InventoryItem>,
    },
    CreateParty {
        leader: PartyMember,
    },
    JoinParty {
        party_id: u64,
        member: PartyMember,
    },
    LeaveParty {
        role_id: RoleId,
    },
    WorldSwitch {
        role_id: RoleId,
        from_world: WorldId,
        to_world: WorldId,
    },
    SystemBroadcast {
        text: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", content = "result")]
pub enum DeliveryResponse {
    Ok(serde_json::Value),
    Error(String),
}

pub struct DeliveryServer {
    service: Arc<DeliveryService>,
    listen_port: u16,
}

impl DeliveryServer {
    pub fn new(service: DeliveryService, listen_port: u16) -> Self {
        Self {
            service: Arc::new(service),
            listen_port,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        info!("pw-delivery daemon escutando em {}", addr);

        loop {
            let (socket, remote_addr) = listener.accept().await?;
            let service = self.service.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, service).await {
                    warn!("Erro na conexão com delivery de {}: {:?}", remote_addr, e);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, service: Arc<DeliveryService>) -> anyhow::Result<()> {
    let mut buffer = vec![0u8; 8192];
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let req_str = std::str::from_utf8(&buffer[..n])?;
    let req: DeliveryRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(e) => {
            let res = DeliveryResponse::Error(format!("Requisição inválida: {}", e));
            let bytes = serde_json::to_vec(&res)?;
            socket.write_all(&bytes).await?;
            return Ok(());
        }
    };

    let response = match req {
        DeliveryRequest::SendChat(msg) => match service.chat.dispatch_chat(&msg).await {
            Ok(_) => DeliveryResponse::Ok(serde_json::json!({ "sent": true })),
            Err(e) => DeliveryResponse::Error(e.to_string()),
        },
        DeliveryRequest::SendMail {
            sender_id,
            receiver_id,
            title,
            message,
            attached_money,
            attached_item,
        } => {
            let res = if let Some(sid) = sender_id {
                service
                    .mail
                    .send_player_mail(
                        &service.realm_id,
                        sid,
                        receiver_id,
                        &title,
                        &message,
                        attached_money,
                        attached_item,
                    )
                    .await
            } else {
                service
                    .mail
                    .send_system_mail(
                        &service.realm_id,
                        receiver_id,
                        &title,
                        &message,
                        attached_money,
                        attached_item,
                    )
                    .await
            };

            match res {
                Ok(mail_id) => DeliveryResponse::Ok(serde_json::json!({ "mail_id": mail_id })),
                Err(e) => DeliveryResponse::Error(e.to_string()),
            }
        }
        DeliveryRequest::CreateParty { leader } => {
            let party_id = service.party.create_party(leader).await;
            DeliveryResponse::Ok(serde_json::json!({ "party_id": party_id }))
        }
        DeliveryRequest::JoinParty { party_id, member } => {
            match service.party.add_member(party_id, member).await {
                Ok(joined) => DeliveryResponse::Ok(serde_json::json!({ "joined": joined })),
                Err(e) => DeliveryResponse::Error(e.to_string()),
            }
        }
        DeliveryRequest::LeaveParty { role_id } => {
            let left = service.party.remove_member(role_id).await;
            DeliveryResponse::Ok(serde_json::json!({ "left_party_id": left }))
        }
        DeliveryRequest::WorldSwitch {
            role_id,
            from_world,
            to_world,
        } => match service.handle_world_switch(role_id, from_world, to_world).await {
            Ok(_) => DeliveryResponse::Ok(serde_json::json!({ "success": true })),
            Err(e) => DeliveryResponse::Error(e.to_string()),
        },
        DeliveryRequest::SystemBroadcast { text } => {
            match service.broadcast_system_announcement(&text).await {
                Ok(_) => DeliveryResponse::Ok(serde_json::json!({ "success": true })),
                Err(e) => DeliveryResponse::Error(e.to_string()),
            }
        }
    };

    let bytes = serde_json::to_vec(&response)?;
    socket.write_all(&bytes).await?;
    Ok(())
}
