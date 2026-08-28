use pw_core::AccountId;
use pw_crypto::{generate_session_ticket, hash_password, verify_password};
use pw_storage::{AccountRepository, CacheManager, StorageError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AuthError {
    #[error("Credenciais inválidas")]
    InvalidCredentials,

    #[error("Conta banida: {reason} (Expira em: {expires_at})")]
    AccountBanned {
        reason: String,
        expires_at: String,
    },

    #[error("Erro de persistência: {0}")]
    Storage(#[from] StorageError),

    #[error("Erro de criptografia: {0}")]
    Crypto(String),
}

pub type Result<T> = std::result::Result<T, AuthError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub account_id: AccountId,
    pub username: String,
    pub session_ticket: String,
    pub gm_privileges: i32,
    pub gold_balance: i64,
}

pub struct AuthService {
    account_repo: AccountRepository,
    cache_manager: CacheManager,
    ticket_ttl_seconds: u64,
}

impl AuthService {
    pub fn new(account_repo: AccountRepository, cache_manager: CacheManager) -> Self {
        Self {
            account_repo,
            cache_manager,
            ticket_ttl_seconds: 3600 * 12, // 12 horas de duração da sessão
        }
    }

    /// Registra uma nova conta com senha segura Argon2id
    pub async fn register(
        &self,
        username: &str,
        raw_password: &str,
        email: Option<String>,
    ) -> Result<AccountId> {
        let password_hash = hash_password(raw_password)
            .map_err(|e| AuthError::Crypto(e.to_string()))?;
        let account = self
            .account_repo
            .create_account(username, &password_hash, email.as_deref())
            .await?;

        info!("Nova conta registrada com sucesso: '{}' (ID: {})", username, account.id);
        Ok(account.id)
    }

    /// Autentica o jogador, verifica status de ban, valida credenciais e gera Session Ticket
    pub async fn authenticate(
        &self,
        username: &str,
        raw_password: &str,
        client_ip: &str,
        _realm_id: &str,
    ) -> Result<LoginResult> {
        let account = self
            .account_repo
            .find_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        // 1. Verifica se a conta está banida
        if account.is_banned {
            let reason = account.ban_reason.unwrap_or_else(|| "Violação dos Termos de Serviço".into());
            let expires_at = account
                .ban_expires_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "Permanente".into());

            warn!("Tentativa de login em conta banida: '{}' - Motivo: {}", username, reason);
            return Err(AuthError::AccountBanned { reason, expires_at });
        }

        // 2. Valida a senha (com suporte a MD5 legado e migração automática)
        let verification = verify_password(username, raw_password, &account.password_hash);
        if !verification.is_valid {
            warn!("Tentativa de login com senha incorreta para a conta '{}' de {}", username, client_ip);
            return Err(AuthError::InvalidCredentials);
        }

        // 3. Se a senha ainda estava em MD5 legado, migra transparentemente para Argon2id
        if verification.needs_rehash {
            if let Ok(new_hash) = hash_password(raw_password) {
                let _ = self.account_repo.update_password(account.id, &new_hash).await;
                info!("Senha da conta '{}' migrada automaticamente de MD5 para Argon2id!", username);
            }
        }

        // 4. Atualiza metadados de último login
        let _ = self.account_repo.update_last_login(account.id, client_ip).await;

        // 5. Gera Session Ticket criptograficamente seguro
        let session_ticket = generate_session_ticket();

        // 6. Registra sessão no cache DragonflyDB
        let cache_payload = serde_json::to_string(&LoginResult {
            account_id: account.id,
            username: account.username.clone(),
            session_ticket: session_ticket.clone(),
            gm_privileges: account.gm_privileges,
            gold_balance: account.gold_balance,
        })
        .unwrap_or_default();

        let ticket_key = format!("ticket:{}", session_ticket);
        let _ = self
            .cache_manager
            .set_session(&ticket_key, &cache_payload, self.ticket_ttl_seconds)
            .await;
        let _ = self
            .cache_manager
            .publish_event(&format!("auth:login:{}", account.id), &cache_payload)
            .await;

        info!(
            "Login bem-sucedido: Usuário '{}' (ID: {}), GM: {}",
            account.username, account.id, account.gm_privileges
        );

        Ok(LoginResult {
            account_id: account.id,
            username: account.username,
            session_ticket,
            gm_privileges: account.gm_privileges,
            gold_balance: account.gold_balance,
        })
    }

    /// Adiciona CUBI / Gold na conta (para faturamento e eventos)
    pub async fn add_gold(&self, account_id: AccountId, amount: i64) -> Result<i64> {
        let new_balance = self.account_repo.add_gold_balance(account_id, amount).await?;
        info!("Adicionado {} Gold para a conta ID {}. Novo saldo: {}", amount, account_id, new_balance);
        Ok(new_balance)
    }
}
