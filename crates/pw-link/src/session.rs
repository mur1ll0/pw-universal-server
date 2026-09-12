use pw_core::{AccountId, RoleId};
use pw_crypto::Rc4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SessionState {
    Handshaking,       // Enviou Challenge, aguardando resposta
    Authenticated,     // Login aprovado
    CharacterSelect,   // Visualizando personagens do Realm
    InWorld,           // Jogando no mundo 3D
    Disconnected,      // Conexão encerrada
}

#[allow(dead_code)]
pub struct ClientSession {
    pub session_id: u64,
    pub state: SessionState,
    pub account_id: Option<AccountId>,
    pub username: Option<String>,
    pub role_id: Option<RoleId>,
    pub character_name: Option<String>,
    /// O `localsid` que veio no `EnterWorld` do cliente.
    ///
    /// Guardado porque o servidor de mundo endereça a resposta por ele, e no logout já
    /// não há pacote de onde tirá-lo — o que o link tem naquele momento é só esta sessão.
    pub localsid: u32,
    pub target_id: Option<i32>,
    pub sec_level: u8,
    /// O mundo (`worldtag`) em que o personagem desta sessão está — decide para qual
    /// servidor de mundo os comandos dele vão. Ver `LinkGateway::uplink_da_sessao`.
    pub world_id: Option<i32>,
    /// Se já mandamos `GetUIConfig_Re` pro personagem atual nesta sessão.
    ///
    /// Achado em 2026-09-03: o client real manda um pedido `GetUIConfig` (opcode 104)
    /// sozinho, disparado por `LoadConfigData()` quando processa o `TASK_DATA` do
    /// `EnterWorld` — só que, testando ponta a ponta, esse pedido **nunca chegou** no
    /// nosso servidor (log de `TASK_DATA enviado` aparece, log de `GetUIConfig pedido`
    /// nunca aparece), e sem essa resposta o client fica preso na tela "Entrando em
    /// Perfect World" pra sempre (`OnPrtcGetConfigRe` — que chama `EnableUI(true)` e
    /// fecha essa tela — só roda ao receber `GetUIConfig_Re`).
    ///
    /// A correção anterior a este achado (2026-09-03, mais cedo) tinha um envio
    /// proativo de `GetUIConfig_Re` no fim do `EnterWorld`, sem esperar o pedido do
    /// client — e foi removida porque, quando o client TAMBÉM pedia sozinho, o
    /// servidor respondia duas vezes, e a segunda passada por `OnPrtcGetConfigRe`
    /// derrubava o client (o hook do `LogicCheck.dll` não é seguro rodar duas vezes).
    /// Essa flag traz o envio proativo de volta, mas manda **no máximo uma vez** por
    /// personagem/sessão — cobre os dois casos (client pede sozinho, ou não pede nunca)
    /// sem repetir o crash antigo.
    pub ui_config_enviado: bool,
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
            username: None,
            role_id: None,
            character_name: None,
            localsid: 0,
            target_id: None,
            sec_level: 0,
            world_id: None,
            ui_config_enviado: false,
            client_ip,
            realm_id,
            game_version,
            client_rc4: None,
            server_rc4: None,
        }
    }

    pub fn set_authenticated(&mut self, account_id: AccountId, username: String) {
        self.account_id = Some(account_id);
        self.username = Some(username);
        self.state = SessionState::Authenticated;
    }

    pub fn set_in_world(&mut self, role_id: RoleId, name: String) {
        self.role_id = Some(role_id);
        self.character_name = Some(name);
        self.state = SessionState::InWorld;
        // Achado em 2026-09-04: `ui_config_enviado` nunca era resetada ao trocar de
        // personagem na MESMA conexão (voltar à seleção — "meia saída" — e entrar com
        // outro). A flag ficava `true` do personagem anterior, então o `GetUIConfig_Re`
        // proativo do novo personagem era pulado (achando que "já tinha sido mandado"),
        // e o pedido que o client manda sozinho também era ignorado pelo mesmo motivo —
        // sem nenhum `GetUIConfig_Re` chegar, o client preso pra sempre em "Entrando em
        // Perfect World" (ver o comentário do campo, mais abaixo). Resetar aqui, no
        // início de cada personagem, é a correção: o campo já documenta a intenção
        // "por personagem/sessão", só faltava isto pra valer de verdade quando os dois
        // coincidem mas mudam no meio da mesma conexão.
        self.ui_config_enviado = false;
    }

    pub fn set_target(&mut self, target: i32) {
        self.target_id = Some(target);
    }

    pub fn clear_target(&mut self) {
        self.target_id = None;
    }
}
