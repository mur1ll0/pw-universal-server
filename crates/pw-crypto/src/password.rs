use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use md5::{Digest, Md5};
use subtle::ConstantTimeEq;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PasswordError {
    #[error("Erro ao gerar hash da senha: {0}")]
    HashError(String),

    #[error("Senha incorreta")]
    InvalidPassword,
}

/// Resultado da verificação de senha
pub struct VerificationResult {
    pub is_valid: bool,
    pub needs_rehash: bool,
}

/// Gera hash moderno Argon2id para uma senha pura
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| PasswordError::HashError(e.to_string()))?
        .to_string();

    Ok(hash)
}

/// Gera hash legado MD5 no formato clássico do Perfect World: MD5(username + password)
pub fn hash_legacy_pw_md5(username: &str, password: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(username.to_lowercase().as_bytes());
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Gera hash puro MD5(password)
pub fn hash_raw_md5(password: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Verifica a senha suportando hashes modernos (Argon2id) e hashes legados (MD5) com sinalização de rehash automático
pub fn verify_password(
    username: &str,
    input_password: &str,
    stored_hash: &str,
) -> VerificationResult {
    // 1. Se o hash armazenado começar com $argon2, usamos o verificador Argon2
    if stored_hash.starts_with("$argon2") {
        if let Ok(parsed_hash) = PasswordHash::new(stored_hash) {
            let is_valid = Argon2::default()
                .verify_password(input_password.as_bytes(), &parsed_hash)
                .is_ok();
            return VerificationResult {
                is_valid,
                needs_rehash: false,
            };
        }
    }

    // 2. Verificação de hash legado: MD5(username + password)
    let legacy_pw_hash = hash_legacy_pw_md5(username, input_password);
    if legacy_pw_hash.as_bytes().ct_eq(stored_hash.as_bytes()).unwrap_u8() == 1 {
        return VerificationResult {
            is_valid: true,
            needs_rehash: true, // Avisa que a senha deve ser atualizada para Argon2 no banco
        };
    }

    // 3. Verificação de hash legado simples: MD5(password)
    let raw_md5_hash = hash_raw_md5(input_password);
    if raw_md5_hash.as_bytes().ct_eq(stored_hash.as_bytes()).unwrap_u8() == 1 {
        return VerificationResult {
            is_valid: true,
            needs_rehash: true,
        };
    }

    // 4. Comparação direta (caso de senhas de teste em texto plano em ambientes de desenvolvimento)
    if input_password.as_bytes().ct_eq(stored_hash.as_bytes()).unwrap_u8() == 1 {
        return VerificationResult {
            is_valid: true,
            needs_rehash: true,
        };
    }

    VerificationResult {
        is_valid: false,
        needs_rehash: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2_hashing_and_verification() {
        let password = "MinhaSenhaSegura2026!";
        let hash = hash_password(password).expect("Falha ao gerar hash");

        let result = verify_password("admin", password, &hash);
        assert!(result.is_valid);
        assert!(!result.needs_rehash);

        let wrong_result = verify_password("admin", "SenhaErrada", &hash);
        assert!(!wrong_result.is_valid);
    }

    #[test]
    fn test_legacy_md5_migration() {
        let username = "jogador1";
        let password = "pw123password";
        let legacy_hash = hash_legacy_pw_md5(username, password);

        // Deve validar com sucesso e sinalizar needs_rehash = true
        let result = verify_password(username, password, &legacy_hash);
        assert!(result.is_valid);
        assert!(result.needs_rehash);
    }
}
