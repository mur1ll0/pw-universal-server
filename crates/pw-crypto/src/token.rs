use rand::RngCore;

/// Gera um Session Ticket criptograficamente seguro com 32 bytes (64 caracteres hexadecimais)
pub fn generate_session_ticket() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Gera um nonce / desafio de 16 bytes para handshake de login Wanmei CNet
/// nonce[0..4] = server_attr (0 = padrão)
/// nonce[4..8] = free_creatime (0 = padrão)
/// nonce[8..16] = 8 bytes aleatórios de segurança
pub fn generate_login_challenge() -> Vec<u8> {
    let mut bytes = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes[8..16]);
    bytes
}
