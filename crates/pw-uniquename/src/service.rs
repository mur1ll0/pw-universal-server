use pw_storage::PostgresPool;
use regex::Regex;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum NameCheckError {
    #[error("Nome muito curto (mínimo 2 caracteres)")]
    TooShort,

    #[error("Nome muito longo (máximo 16 caracteres)")]
    TooLong,

    #[error("Nome contém caracteres inválidos ou proibidos")]
    InvalidCharacters,

    #[error("Nome contém palavras reservadas (GM/Admin)")]
    ReservedWord,

    #[error("Nome já está em uso neste Realm")]
    AlreadyExists,

    #[error("Erro no banco de dados: {0}")]
    Storage(#[from] pw_storage::StorageError),
}

pub type Result<T> = std::result::Result<T, NameCheckError>;

#[derive(Clone)]
pub struct UniqueNameService {
    pool: PostgresPool,
    valid_name_regex: Regex,
}

impl UniqueNameService {
    pub fn new(pool: PostgresPool) -> Self {
        // Permite letras, números, underscores e hífens
        let valid_name_regex = Regex::new(r"^[\p{L}\p{N}_\-]+$").expect("Regex inválido");
        Self {
            pool,
            valid_name_regex,
        }
    }

    /// Valida se um nome de personagem está disponível e segue as regras de formação
    pub async fn check_character_name(
        &self,
        realm_id: &str,
        name: &str,
        is_gm: bool,
    ) -> Result<()> {
        let trimmed = name.trim();

        // 1. Validação de tamanho
        if trimmed.chars().count() < 2 {
            return Err(NameCheckError::TooShort);
        }
        if trimmed.chars().count() > 16 {
            return Err(NameCheckError::TooLong);
        }

        // 2. Validação de caracteres
        if !self.valid_name_regex.is_match(trimmed) {
            return Err(NameCheckError::InvalidCharacters);
        }

        // 3. Palavras reservadas (GM, Admin, GameMaster)
        let lower = trimmed.to_lowercase();
        if !is_gm && (lower.contains("gm") || lower.contains("admin") || lower.contains("gamemaster") || lower.contains("system")) {
            return Err(NameCheckError::ReservedWord);
        }

        // 4. Verificação no banco de dados para o Realm específico
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM characters 
            WHERE realm_id = $1 AND LOWER(name) = LOWER($2) AND is_deleted = FALSE
            "#,
        )
        .bind(realm_id)
        .bind(trimmed)
        .fetch_one(self.pool.get_ref())
        .await
        .map_err(pw_storage::StorageError::Database)?;

        if count > 0 {
            return Err(NameCheckError::AlreadyExists);
        }

        info!("Nome de personagem '{}' disponível para o Realm '{}'", trimmed, realm_id);
        Ok(())
    }

    /// Valida se um nome de clã/facção está disponível
    pub async fn check_faction_name(&self, realm_id: &str, name: &str) -> Result<()> {
        let trimmed = name.trim();

        if trimmed.chars().count() < 2 {
            return Err(NameCheckError::TooShort);
        }
        if trimmed.chars().count() > 16 {
            return Err(NameCheckError::TooLong);
        }

        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM factions 
            WHERE realm_id = $1 AND LOWER(name) = LOWER($2)
            "#,
        )
        .bind(realm_id)
        .bind(trimmed)
        .fetch_one(self.pool.get_ref())
        .await
        .map_err(pw_storage::StorageError::Database)?;

        if count > 0 {
            return Err(NameCheckError::AlreadyExists);
        }

        info!("Nome de clã '{}' disponível para o Realm '{}'", trimmed, realm_id);
        Ok(())
    }
}
