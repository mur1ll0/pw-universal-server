use rand::RngCore;

/// Gera um Session Ticket criptograficamente seguro com 32 bytes (64 caracteres hexadecimais)
pub fn generate_session_ticket() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Gera um nonce / desafio aleatório de 16 bytes para handshake de login
pub fn generate_login_challenge() -> Vec<u8> {
    let mut bytes = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}
