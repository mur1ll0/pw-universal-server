use crate::error::{Result, StorageError};
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccountRecord {
    pub id: AccountId,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub gold_balance: i64,
    pub silver_balance: i64,
    pub gm_privileges: i32,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
    pub ban_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub last_login_ip: Option<String>,
}

#[derive(Clone)]
pub struct AccountRepository {
    pool: PostgresPool,
}

impl AccountRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Cria uma nova conta global
    pub async fn create_account(
        &self,
        username: &str,
        password_hash: &str,
        email: Option<&str>,
    ) -> Result<AccountRecord> {
        let rec = sqlx::query_as::<_, AccountRecord>(
            r#"
            INSERT INTO accounts (username, password_hash, email)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(password_hash)
        .bind(email)
        .fetch_one(self.pool.get_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
                StorageError::Duplicate(format!("Conta '{}' já existe", username))
            }
            _ => StorageError::Database(e),
        })?;

        Ok(rec)
    }

    /// Busca conta por nome de usuário (case-insensitive)
    pub async fn find_by_username(&self, username: &str) -> Result<Option<AccountRecord>> {
        let rec = sqlx::query_as::<_, AccountRecord>(
            r#"
            SELECT * FROM accounts 
            WHERE LOWER(username) = LOWER($1)
            "#,
        )
        .bind(username)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(rec)
    }

    /// Busca conta por ID
    pub async fn find_by_id(&self, id: AccountId) -> Result<Option<AccountRecord>> {
        let rec = sqlx::query_as::<_, AccountRecord>(
            r#"
            SELECT * FROM accounts WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(rec)
    }

    /// Atualiza senha da conta
    pub async fn update_password(&self, account_id: AccountId, password_hash: &str) -> Result<()> {
        let res = sqlx::query(
            r#"
            UPDATE accounts 
            SET password_hash = $1 
            WHERE id = $2
            "#,
        )
        .bind(password_hash)
        .bind(account_id)
        .execute(self.pool.get_ref())
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Conta ID {}", account_id)));
        }
        Ok(())
    }

    /// Injeta ou deduz saldo de CUBI / GOLD / Cash
    pub async fn add_gold_balance(&self, account_id: AccountId, amount: i64) -> Result<i64> {
        let rec = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE accounts 
            SET gold_balance = gold_balance + $1 
            WHERE id = $2 
            RETURNING gold_balance
            "#,
        )
        .bind(amount)
        .bind(account_id)
        .fetch_one(self.pool.get_ref())
        .await?;

        Ok(rec)
    }

    /// Altera nível de privilégios de GM (0: Normal, 1..32: Níveis de GM)
    pub async fn set_gm_privileges(&self, account_id: AccountId, gm_level: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE accounts 
            SET gm_privileges = $1 
            WHERE id = $2
            "#,
        )
        .bind(gm_level)
        .bind(account_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Aplica ou remove banimento
    pub async fn set_ban_status(
        &self,
        account_id: AccountId,
        is_banned: bool,
        reason: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE accounts 
            SET is_banned = $1, ban_reason = $2, ban_expires_at = $3 
            WHERE id = $4
            "#,
        )
        .bind(is_banned)
        .bind(reason)
        .bind(expires_at)
        .bind(account_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Atualiza timestamp de último login e IP
    pub async fn update_last_login(&self, account_id: AccountId, ip: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE accounts 
            SET last_login_at = CURRENT_TIMESTAMP, last_login_ip = $1 
            WHERE id = $2
            "#,
        )
        .bind(ip)
        .bind(account_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }
}
