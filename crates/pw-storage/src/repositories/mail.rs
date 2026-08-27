use crate::error::Result;
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::{InventoryItem, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MailRecord {
    pub id: i32,
    pub realm_id: String,
    pub sender_id: Option<RoleId>,
    pub receiver_id: RoleId,
    pub title: String,
    pub message: String,
    pub attached_money: i64,
    pub attached_item: Option<sqlx::types::Json<InventoryItem>>,
    pub is_read: bool,
    pub is_collected: bool,
    pub sent_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct MailRepository {
    pool: PostgresPool,
}

impl MailRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    pub async fn send_mail(
        &self,
        realm_id: &str,
        sender_id: Option<RoleId>,
        receiver_id: RoleId,
        title: &str,
        message: &str,
        attached_money: i64,
        attached_item: Option<InventoryItem>,
    ) -> Result<i32> {
        let mail_id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO mails (
                realm_id, sender_id, receiver_id, title, message, attached_money, attached_item
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(realm_id)
        .bind(sender_id)
        .bind(receiver_id)
        .bind(title)
        .bind(message)
        .bind(attached_money)
        .bind(attached_item.map(sqlx::types::Json))
        .fetch_one(self.pool.get_ref())
        .await?;

        Ok(mail_id)
    }

    pub async fn list_inbox(&self, receiver_id: RoleId) -> Result<Vec<MailRecord>> {
        let recs = sqlx::query_as::<_, MailRecord>(
            r#"
            SELECT * FROM mails 
            WHERE receiver_id = $1 AND expires_at > CURRENT_TIMESTAMP 
            ORDER BY sent_at DESC
            "#,
        )
        .bind(receiver_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(recs)
    }
}
