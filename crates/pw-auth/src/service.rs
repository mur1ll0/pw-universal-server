use chrono::Utc;
use pw_core::AccountId;
use pw_crypto::{generate_session_ticket, hash_password, verify_password};
use pw_storage::{AccountRepository, CacheManager, StorageError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Credenciais inválidas")]
    InvalidCredentials,

    #[error("Conta banida: {reason} (Expira em: {expires_at})")]
    AccountBanned {
        reason: String,
        expires_at: String,
    },

    #[error("Conta não encontrada")]
    AccountNotFound,

    #[error("Conta já existe: {0}")]
    AccountAlreadyExists(String),

    #[error("Ticket de sessão inválido ou expirado")]
    InvalidSessionTicket,

    #[error("Erro de persistência/banco: {0}")]
    Storage(#[from] StorageError),

    #[error("Erro criptográfico: {0}")]
    Crypto(#[from] pw_crypto::PasswordError),
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

#[derive(Clone)]
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

    /// Autentica o usuário no login do jogo
    pub async fn login(
        &self,
        username: &str,
        password: &str,
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
            if let Some(exp) = account.ban_expires_at {
                if exp > Utc::now() {
                    return Err(AuthError::AccountBanned {
                        reason: account.ban_reason.unwrap_or_else(|| "Sem motivo".to_string()),
                        expires_at: exp.to_rfc3339(),
                    });
                }
            } else {
                return Err(AuthError::AccountBanned {
                    reason: account.ban_reason.unwrap_or_else(|| "Banimento Permanente".to_string()),
                    expires_at: "Permanente".to_string(),
                });
            }
        }

        // 2. Valida a senha (suportando Argon2id e MD5 legado)
        let verification = verify_password(username, password, &account.password_hash);
        if !verification.is_valid {
            warn!("Tentativa de login com senha incorreta para o usuário: {}", username);
            return Err(AuthError::InvalidCredentials);
        }

        // 3. Atualização automática transparente de hash legado para Argon2id
        if verification.needs_rehash {
            info!("Atualizando hash de senha do usuário '{}' para Argon2id...", username);
            if let Ok(new_argon_hash) = hash_password(password) {
                if let Err(e) = self.account_repo.update_password(account.id, &new_argon_hash).await {
                    error!("Falha ao salvar novo hash Argon2id para usuário {}: {:?}", username, e);
                }
            }
        }

        // 4. Atualiza timestamp de último login e IP
        let _ = self.account_repo.update_last_login(account.id, client_ip).await;

        // 5. Gera Session Ticket criptográfico e armazena no DragonflyDB
        let session_ticket = generate_session_ticket();
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

    /// Registra uma nova conta global
    pub async fn register(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<AccountId> {
        let password_hash = hash_password(password)?;
        let account = self
            .account_repo
            .create_account(username, &password_hash, email)
            .await
            .map_err(|e| match e {
                StorageError::Duplicate(msg) => AuthError::AccountAlreadyExists(msg),
                other => AuthError::Storage(other),
            })?;

        info!("Nova conta registrada com sucesso: '{}' (ID: {})", username, account.id);
        Ok(account.id)
    }

    /// Injeta Gold / CUBI na conta
    pub async fn add_gold(&self, account_id: AccountId, amount: i64) -> Result<i64> {
        let new_balance = self.account_repo.add_gold_balance(account_id, amount).await?;
        info!("Gold adicionado para conta ID {}: +{} (Novo saldo: {})", account_id, amount, new_balance);
        Ok(new_balance)
    }
}
