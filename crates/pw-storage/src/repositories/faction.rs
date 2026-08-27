use crate::error::{Result, StorageError};
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::{FactionId, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FactionRecord {
    pub id: FactionId,
    pub realm_id: String,
    pub name: String,
    pub level: i32,
    pub master_character_id: RoleId,
    pub announcement: Option<String>,
    pub members: sqlx::types::Json<serde_json::Value>,
    pub fortress: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct FactionRepository {
    pool: PostgresPool,
}

impl FactionRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    pub async fn create_faction(
        &self,
        realm_id: &str,
        name: &str,
        master_id: RoleId,
    ) -> Result<FactionId> {
        let id = sqlx::query_scalar::<_, FactionId>(
            r#"
            INSERT INTO factions (realm_id, name, master_character_id)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(realm_id)
        .bind(name)
        .bind(master_id)
        .fetch_one(self.pool.get_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
                StorageError::Duplicate(format!("Clã '{}' já existe neste Realm", name))
            }
            _ => StorageError::Database(e),
        })?;

        Ok(id)
    }

    pub async fn get_faction(&self, faction_id: FactionId) -> Result<Option<FactionRecord>> {
        let rec = sqlx::query_as::<_, FactionRecord>(
            r#"
            SELECT * FROM factions WHERE id = $1
            "#,
        )
        .bind(faction_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(rec)
    }
}
