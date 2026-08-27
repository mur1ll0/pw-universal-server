use pw_core::{InventoryItem, RoleId};
use pw_storage::{MailRecord, MailRepository};
use tracing::info;

#[derive(Clone)]
pub struct MailManager {
    mail_repo: MailRepository,
}

impl MailManager {
    pub fn new(mail_repo: MailRepository) -> Self {
        Self { mail_repo }
    }

    /// Envia uma mensagem de correio normal entre jogadores
    pub async fn send_player_mail(
        &self,
        realm_id: &str,
        sender_id: RoleId,
        receiver_id: RoleId,
        title: &str,
        message: &str,
        attached_money: i64,
        attached_item: Option<InventoryItem>,
    ) -> anyhow::Result<i32> {
        let mail_id = self
            .mail_repo
            .send_mail(
                realm_id,
                Some(sender_id),
                receiver_id,
                title,
                message,
                attached_money,
                attached_item,
            )
            .await?;

        info!(
            "[{}] Correio enviado de #{} para #{}: '{}' (ID: {})",
            realm_id, sender_id, receiver_id, title, mail_id
        );
        Ok(mail_id)
    }

    /// Envia um correio oficial do sistema (SysMail / Recompensa de Evento / Painel Web)
    pub async fn send_system_mail(
        &self,
        realm_id: &str,
        receiver_id: RoleId,
        title: &str,
        message: &str,
        attached_money: i64,
        attached_item: Option<InventoryItem>,
    ) -> anyhow::Result<i32> {
        let mail_id = self
            .mail_repo
            .send_mail(
                realm_id,
                None, // Sender nulo = Sistema
                receiver_id,
                title,
                message,
                attached_money,
                attached_item,
            )
            .await?;

        info!(
            "[{}] SysMail oficial enviado para #{}: '{}' (ID: {})",
            realm_id, receiver_id, title, mail_id
        );
        Ok(mail_id)
    }

    /// Consulta caixa de entrada
    pub async fn get_inbox(&self, receiver_id: RoleId) -> anyhow::Result<Vec<MailRecord>> {
        let mails = self.mail_repo.list_inbox(receiver_id).await?;
        Ok(mails)
    }
}
