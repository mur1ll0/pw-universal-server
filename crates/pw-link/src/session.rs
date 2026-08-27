use pw_core::{AccountId, RoleId};
use pw_crypto::Rc4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Handshaking,       // Enviou Challenge, aguardando resposta
    Authenticated,     // Login aprovado
    CharacterSelect,   // Visualizando personagens do Realm
    InWorld,           // Jogando no mundo 3D
    Disconnected,      // Conexão encerrada
}

pub struct ClientSession {
    pub session_id: u64,
    pub state: SessionState,
    pub account_id: Option<AccountId>,
    pub role_id: Option<RoleId>,
    pub client_ip: String,
    pub realm_id: String,
    pub game_version: String,
    
    // Cifras simétricas RC4 por direção de fluxo
    pub client_rc4: Option<Rc4>, // Decripta pacotes recebidos do cliente
    pub server_rc4: Option<Rc4>, // Encripta pacotes enviados ao cliente
}

impl ClientSession {
    pub fn new(session_id: u64, client_ip: String, realm_id: String, game_version: String) -> Self {
        Self {
            session_id,
            state: SessionState::Handshaking,
            account_id: None,
            role_id: None,
            client_ip,
            realm_id,
            game_version,
            client_rc4: None,
            server_rc4: None,
        }
    }

    pub fn set_authenticated(&mut self, account_id: AccountId) {
        self.account_id = Some(account_id);
        self.state = SessionState::Authenticated;
    }

    pub fn set_in_world(&mut self, role_id: RoleId) {
        self.role_id = Some(role_id);
        self.state = SessionState::InWorld;
    }
}
