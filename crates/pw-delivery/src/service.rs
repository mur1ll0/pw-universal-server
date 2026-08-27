use crate::chat::{ChatManager, ChatMessage};
use crate::mail::MailManager;
use crate::party::{PartyManager, PartyMember};
use pw_core::{InventoryItem, RoleId, WorldId};
use pw_storage::{CacheManager, MailRepository};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct DeliveryService {
    pub realm_id: String,
    pub chat: ChatManager,
    pub party: PartyManager,
    pub mail: MailManager,
    pub cache: CacheManager,
}

impl DeliveryService {
    pub fn new(realm_id: String, mail_repo: MailRepository, cache: CacheManager) -> Self {
        let chat = ChatManager::new(cache.clone());
        let party = PartyManager::new();
        let mail = MailManager::new(mail_repo);

        Self {
            realm_id,
            chat,
            party,
            mail,
            cache,
        }
    }

    /// Trata a troca de mapa / instância (World Switch / Handoff)
    pub async fn handle_world_switch(
        &self,
        role_id: RoleId,
        from_world: WorldId,
        to_world: WorldId,
    ) -> anyhow::Result<()> {
        info!(
            "[{}] Jogador #{} transferindo do Mapa {} para o Mapa {}",
            self.realm_id, role_id, from_world, to_world
        );
        Ok(())
    }

    /// Transmite anúncio do sistema para todo o Realm
    pub async fn broadcast_system_announcement(&self, text: &str) -> anyhow::Result<()> {
        let msg = ChatMessage {
            realm_id: self.realm_id.clone(),
            channel: crate::chat::ChatChannel::System,
            sender_id: 0,
            sender_name: "SISTEMA".to_string(),
            target_id: None,
            target_name: None,
            content: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.chat.dispatch_chat(&msg).await?;
        Ok(())
    }
}
