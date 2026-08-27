use crate::session::{ClientSession, SessionState};
use futures::{SinkExt, StreamExt};
use pw_crypto::{generate_login_challenge, Rc4};
use pw_protocol::{
    InboundPacket, OutboundPacket, PwPacketCodec, S2CChatBroadcast, S2CChallenge, S2CEnterWorldResponse,
    S2CLoginSuccess, S2CPlayerMoveBroadcast, S2CRoleListResponse,
};
use pw_storage::{AccountRepository, CacheManager, CharacterRepository};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct LinkGateway {
    realm_id: String,
    game_version: String,
    listen_port: u16,
    account_repo: AccountRepository,
    char_repo: CharacterRepository,
    cache_manager: CacheManager,
}

impl LinkGateway {
    pub fn new(
        realm_id: String,
        game_version: String,
        listen_port: u16,
        account_repo: AccountRepository,
        char_repo: CharacterRepository,
        cache_manager: CacheManager,
    ) -> Self {
        Self {
            realm_id,
            game_version,
            listen_port,
            account_repo,
            char_repo,
            cache_manager,
        }
    }

    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        info!(
            "pw-link (Gateway de Rede) ativo para o Realm '{}' (v{}) na porta {}",
            self.realm_id, self.game_version, self.listen_port
        );

        loop {
            let (socket, remote_addr) = listener.accept().await?;
            let gateway = self.clone();
            let session_id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);

            tokio::spawn(async move {
                let client_ip = remote_addr.ip().to_string();
                if let Err(e) = gateway.handle_client(socket, session_id, client_ip).await {
                    debug!("Sessão #{} desconectada: {:?}", session_id, e);
                }
            });
        }
    }

    async fn handle_client(
        &self,
        socket: TcpStream,
        session_id: u64,
        client_ip: String,
    ) -> anyhow::Result<()> {
        let codec = PwPacketCodec::new(&self.game_version);
        let mut framed = Framed::new(socket, codec);

        let mut session = ClientSession::new(
            session_id,
            client_ip.clone(),
            self.realm_id.clone(),
            self.game_version.clone(),
        );

        // 1. Envia o desafio de conexão inicial (S2CChallenge)
        let nonce = generate_login_challenge();
        let challenge = S2CChallenge {
            server_version: if self.game_version == "1.2.6" { 10 } else { 153 },
            nonce: nonce.clone(),
        };

        framed.send(OutboundPacket::Challenge(challenge)).await?;
        info!("Sessão #{}: Desafio enviado para {}", session_id, client_ip);

        // 2. Loop de processamento de pacotes
        while let Some(packet_res) = framed.next().await {
            let packet = match packet_res {
                Ok(p) => p,
                Err(e) => {
                    warn!("Sessão #{}: Erro no codec de pacotes: {:?}", session_id, e);
                    break;
                }
            };

            match packet {
                InboundPacket::ChallengeResponse(res) => {
                    info!(
                        "Sessão #{}: Tentativa de login para o usuário '{}'",
                        session_id, res.username
                    );

                    // Valida credenciais no banco de contas global
                    let account = match self.account_repo.find_by_username(&res.username).await? {
                        Some(acc) => acc,
                        None => {
                            warn!("Sessão #{}: Usuário '{}' não encontrado", session_id, res.username);
                            break;
                        }
                    };

                    let verification = pw_crypto::verify_password(
                        &res.username,
                        &res.password_hash,
                        &account.password_hash,
                    );

                    if !verification.is_valid {
                        warn!("Sessão #{}: Senha incorreta para o usuário '{}'", session_id, res.username);
                        break;
                    }

                    // Autenticado com sucesso
                    session.set_authenticated(account.id);
                    let session_ticket = pw_crypto::generate_session_ticket();

                    framed
                        .send(OutboundPacket::LoginSuccess(S2CLoginSuccess {
                            account_id: account.id,
                            gm_privileges: account.gm_privileges,
                            session_ticket,
                        }))
                        .await?;

                    // Carrega e envia a lista de personagens deste Realm específico
                    let characters = self
                        .char_repo
                        .list_by_account_and_realm(account.id, &self.realm_id)
                        .await?;

                    framed
                        .send(OutboundPacket::RoleListResponse(S2CRoleListResponse {
                            characters,
                        }))
                        .await?;
                }

                InboundPacket::RoleList(_) => {
                    if let Some(account_id) = session.account_id {
                        let characters = self
                            .char_repo
                            .list_by_account_and_realm(account_id, &self.realm_id)
                            .await?;

                        framed
                            .send(OutboundPacket::RoleListResponse(S2CRoleListResponse {
                                characters,
                            }))
                            .await?;
                    }
                }

                InboundPacket::CreateRole(req) => {
                    if let Some(account_id) = session.account_id {
                        let appearance_json: serde_json::Value =
                            serde_json::from_slice(&req.custom_appearance).unwrap_or(serde_json::json!({}));

                        let role_id = self
                            .char_repo
                            .create_character(
                                account_id,
                                &self.realm_id,
                                &req.name,
                                req.race,
                                req.cls,
                                req.gender,
                                appearance_json,
                            )
                            .await?;

                        info!(
                            "Sessão #{}: Personagem '{}' criado com sucesso (ID: {})",
                            session_id, req.name, role_id
                        );

                        // Reenvia lista atualizada
                        let characters = self
                            .char_repo
                            .list_by_account_and_realm(account_id, &self.realm_id)
                            .await?;

                        framed
                            .send(OutboundPacket::RoleListResponse(S2CRoleListResponse {
                                characters,
                            }))
                            .await?;
                    }
                }

                InboundPacket::SelectRole(req) => {
                    if let Some(details) = self.char_repo.get_details(req.role_id).await? {
                        session.set_in_world(details.id);

                        // Registra sessão ativa no cache DragonflyDB
                        if let Some(acc_id) = session.account_id {
                            let _ = self
                                .cache_manager
                                .set_player_session(&self.realm_id, details.id, acc_id, 3600)
                                .await;
                        }

                        info!(
                            "Sessão #{}: Jogador '{}' (ID: {}) entrou no mundo!",
                            session_id, details.name, details.id
                        );

                        framed
                            .send(OutboundPacket::EnterWorld(S2CEnterWorldResponse {
                                role_id: details.id,
                                world_id: details.world_id,
                                position: details.position,
                                hp: details.hp,
                                max_hp: 1000,
                                mp: details.mp,
                                max_mp: 1000,
                                exp: details.exp,
                                sp: details.sp,
                                level: details.level,
                            }))
                            .await?;
                    }
                }

                InboundPacket::PlayerMove(req) => {
                    if let Some(role_id) = session.role_id {
                        // Transmite o movimento para outros jogadores do mundo
                        framed
                            .send(OutboundPacket::PlayerMoveBroadcast(S2CPlayerMoveBroadcast {
                                role_id,
                                mode: req.mode,
                                position: req.position,
                                target: req.target,
                                speed: req.speed,
                                timestamp: req.timestamp,
                            }))
                            .await?;
                    }
                }

                InboundPacket::PlayerChat(req) => {
                    if let Some(role_id) = session.role_id {
                        framed
                            .send(OutboundPacket::ChatBroadcast(S2CChatBroadcast {
                                channel: req.channel,
                                sender_id: role_id,
                                sender_name: "Jogador".to_string(),
                                message: req.message,
                            }))
                            .await?;
                    }
                }

                InboundPacket::Heartbeat(req) => {
                    debug!("Sessão #{}: Heartbeat recebido ({})", session_id, req.timestamp);
                }

                InboundPacket::Unknown { opcode, .. } => {
                    debug!("Sessão #{}: Pacote 0x{:X} ignorado", session_id, opcode);
                }
            }
        }

        // Limpeza de sessão ao desconectar
        if let Some(role_id) = session.role_id {
            let _ = self
                .cache_manager
                .remove_player_session(&self.realm_id, role_id)
                .await;
            info!("Sessão #{}: Jogador ID {} deslogado do Realm", session_id, role_id);
        }

        Ok(())
    }
}
