use pw_core::RoleId;
use pw_storage::CacheManager;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatChannel {
    General = 0, // Proximidade local
    World = 1,   // Chat Global do Realm
    Faction = 2, // Chat de Clã
    Party = 3,   // Chat de Grupo
    Whisper = 4, // Mensagem Privada (1 para 1)
    System = 5,  // Anúncio do Sistema / GM (Texto Amarelo)
}

impl ChatChannel {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => ChatChannel::World,
            2 => ChatChannel::Faction,
            3 => ChatChannel::Party,
            4 => ChatChannel::Whisper,
            5 => ChatChannel::System,
            _ => ChatChannel::General,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub realm_id: String,
    pub channel: ChatChannel,
    pub sender_id: RoleId,
    pub sender_name: String,
    pub target_id: Option<RoleId>,
    pub target_name: Option<String>,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct ChatManager {
    cache_manager: CacheManager,
}

impl ChatManager {
    pub fn new(cache_manager: CacheManager) -> Self {
        Self { cache_manager }
    }

    /// Roteia e transmite a mensagem de chat para o canal apropriado
    pub async fn dispatch_chat(&self, msg: &ChatMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_string(msg)?;

        match msg.channel {
            ChatChannel::World | ChatChannel::System => {
                // Publica no canal global do Realm via DragonflyDB Pub/Sub
                let channel_key = format!("chat:{}:world", msg.realm_id);
                self.cache_manager.publish_event(&channel_key, &payload).await?;
                info!(
                    "[{}] Chat {}: [{}] {}",
                    msg.realm_id,
                    if msg.channel == ChatChannel::System { "SYSTEM" } else { "WORLD" },
                    msg.sender_name,
                    msg.content
                );
            }
            ChatChannel::Faction => {
                let channel_key = format!("chat:{}:faction", msg.realm_id);
                self.cache_manager.publish_event(&channel_key, &payload).await?;
            }
            ChatChannel::Party => {
                let channel_key = format!("chat:{}:party", msg.realm_id);
                self.cache_manager.publish_event(&channel_key, &payload).await?;
            }
            ChatChannel::Whisper => {
                if let Some(target_id) = msg.target_id {
                    let channel_key = format!("chat:{}:whisper:{}", msg.realm_id, target_id);
                    self.cache_manager.publish_event(&channel_key, &payload).await?;
                }
            }
            ChatChannel::General => {
                // Transmitido localmente pelo Game Server (GS) para o grid espacial de visão
            }
        }

        Ok(())
    }
}
