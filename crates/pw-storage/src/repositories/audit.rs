use crate::error::Result;
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::{AccountId, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogRecord {
    pub id: i32,
    pub admin_account_id: Option<AccountId>,
    pub action_type: String,
    pub target_account_id: Option<AccountId>,
    pub target_character_id: Option<RoleId>,
    pub realm_id: Option<String>,
    pub details: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AuditLogRepository {
    pool: PostgresPool,
}

impl AuditLogRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    pub async fn log_action(
        &self,
        admin_account_id: Option<AccountId>,
        action_type: &str,
        target_account_id: Option<AccountId>,
        target_character_id: Option<RoleId>,
        realm_id: Option<&str>,
        details: serde_json::Value,
    ) -> Result<i32> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO admin_audit_logs (
                admin_account_id, action_type, target_account_id, target_character_id, realm_id, details
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(admin_account_id)
        .bind(action_type)
        .bind(target_account_id)
        .bind(target_character_id)
        .bind(realm_id)
        .bind(sqlx::types::Json(details))
        .fetch_one(self.pool.get_ref())
        .await?;

        Ok(id)
    }

    pub async fn list_recent_logs(&self, limit: i64) -> Result<Vec<AuditLogRecord>> {
        let recs = sqlx::query_as::<_, AuditLogRecord>(
            r#"
            SELECT * FROM admin_audit_logs 
            ORDER BY created_at DESC 
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(recs)
    }
}
