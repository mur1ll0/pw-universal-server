use crate::error::Result;
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::RealmId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RealmRecord {
    pub id: RealmId,
    pub name: String,
    pub version: String,
    pub host: String,
    pub port: i32,
    pub is_online: bool,
    pub max_players: i32,
    pub double_exp_multiplier: f32,
    pub double_sp_multiplier: f32,
    pub double_drop_multiplier: f32,
    pub double_gold_multiplier: f32,
    pub config: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct RealmRepository {
    pool: PostgresPool,
}

impl RealmRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Lista todos os Realms configurados
    pub async fn list_realms(&self) -> Result<Vec<RealmRecord>> {
        let recs = sqlx::query_as::<_, RealmRecord>(
            r#"
            SELECT * FROM realms ORDER BY id ASC
            "#,
        )
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(recs)
    }

    /// Busca um Realm por ID (ex: "realm_126")
    pub async fn get_realm(&self, realm_id: &str) -> Result<Option<RealmRecord>> {
        let rec = sqlx::query_as::<_, RealmRecord>(
            r#"
            SELECT * FROM realms WHERE id = $1
            "#,
        )
        .bind(realm_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(rec)
    }

    /// Atualiza multiplicadores de Double Events em tempo real
    pub async fn update_multipliers(
        &self,
        realm_id: &str,
        exp: f32,
        sp: f32,
        drop: f32,
        gold: f32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE realms 
            SET double_exp_multiplier = $1,
                double_sp_multiplier = $2,
                double_drop_multiplier = $3,
                double_gold_multiplier = $4
            WHERE id = $5
            "#,
        )
        .bind(exp)
        .bind(sp)
        .bind(drop)
        .bind(gold)
        .bind(realm_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Liga ou desliga o Realm
    pub async fn set_online_status(&self, realm_id: &str, is_online: bool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE realms SET is_online = $1 WHERE id = $2
            "#,
        )
        .bind(is_online)
        .bind(realm_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }
}
