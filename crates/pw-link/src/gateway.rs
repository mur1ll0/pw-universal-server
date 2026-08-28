use futures::{SinkExt, StreamExt};
use pw_core::{CharacterSummary, Vector3};
use pw_crypto::generate_login_challenge;
use pw_protocol::{
    create_protocol_adapter, GameVersion, InboundPacket, OutboundPacket, ProtocolAdapter,
    PwPacketCodec, S2CChatBroadcast, S2CChallenge, S2CCreateRoleResponse, S2CDeleteRoleResponse,
    S2CErrorInfo, S2CGamedataSend, S2CGetFriendListRe, S2CGetHelpStatesRe, S2CGetUIConfigRe, S2CGetWaitDelRolesRe,
    S2COnlineAnnounce, S2CPlayerLogout, S2CPlayerMoveBroadcast, S2CRoleListResponse, S2CSelectRoleResponse,
    S2CSetCustomDataRe, S2CSetHelpStatesRe, S2CSetUIConfigRe, S2CUndoDeleteRoleResponse,
};
use pw_storage::{AccountRepository, CacheManager, CharacterRepository};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, info, warn};

use crate::session::ClientSession;

pub struct LinkGateway {
    pub realm_id: String,
    pub game_version: GameVersion,
    pub adapter: Arc<dyn ProtocolAdapter>,
    pub listen_port: u16,
    pub account_repo: AccountRepository,
    pub char_repo: CharacterRepository,
    pub cache_manager: CacheManager,
}

impl LinkGateway {
    pub fn new(
        realm_id: String,
        version_str: &str,
        listen_port: u16,
        account_repo: AccountRepository,
        char_repo: CharacterRepository,
        cache_manager: CacheManager,
    ) -> Self {
        let game_version = version_str.parse::<GameVersion>().unwrap_or(GameVersion::V1_2_6);
        let adapter = create_protocol_adapter(game_version);

        Self {
            realm_id,
            game_version,
            adapter,
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
            "Gateway pw-link escutando na porta {} para o Realm '{}' (v{} / server_code: {})",
            self.listen_port,
            self.realm_id,
            self.game_version,
            self.game_version.server_version_code()
        );

        let mut session_counter: u64 = 0;

        loop {
            let (socket, remote_addr) = listener.accept().await?;
            session_counter += 1;
            let session_id = session_counter;
            let gateway = self.clone();
            let client_ip = remote_addr.ip().to_string();

            tokio::spawn(async move {
                if let Err(e) = gateway.handle_client(socket, session_id, client_ip).await {
                    warn!("Sessão #{} desconectada: {:?}", session_id, e);
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
        debug!("Nova conexão recebida de {} (Sessão #{})", client_ip, session_id);

        let mut framed = Framed::new(socket, PwPacketCodec::from_adapter(self.adapter.clone()));
        let mut session = ClientSession::new(
            session_id,
            client_ip.clone(),
            self.realm_id.clone(),
            self.game_version.to_string(),
        );

        // 1. Envia Challenge de Login inicial para o cliente
        let server_nonce = generate_login_challenge();
        let challenge_packet = OutboundPacket::Challenge(S2CChallenge::new(server_nonce.to_vec()));

        framed.send(challenge_packet).await?;
        debug!("Challenge v{} enviado para a Sessão #{}", self.game_version, session_id);

        // 2. Loop de processamento de pacotes do cliente
        while let Some(packet_result) = framed.next().await {
            let packet = match packet_result {
                Ok(p) => p,
                Err(e) => {
                    warn!("Erro de codec na Sessão #{}: {:?}", session_id, e);
                    break;
                }
            };

            self.dispatch_packet(&mut framed, &mut session, packet).await?;
        }

        info!("Sessão #{} ({}) finalizada.", session_id, client_ip);
        Ok(())
    }

    async fn dispatch_packet(
        &self,
        framed: &mut Framed<TcpStream, PwPacketCodec>,
        session: &mut ClientSession,
        packet: InboundPacket,
    ) -> anyhow::Result<()> {
        match packet {
            InboundPacket::Response(login) => {
                debug!(
                    "Recebida resposta de login para o usuário '{}' na Sessão #{}",
                    login.username, session.session_id
                );

                let account_opt = self.account_repo.find_by_username(&login.username).await?;

                let account = if let Some(acc) = account_opt {
                    acc
                } else if login.username.to_lowercase() == "admin" {
                    // Auto-seed admin/admin se a tabela ainda não tiver o registro
                    let hash = pw_crypto::hash_legacy_pw_md5("admin", "admin");
                    match self.account_repo.create_account("admin", &hash, Some("admin@pwserver.local")).await {
                        Ok(acc) => acc,
                        Err(_) => {
                            if let Ok(Some(existing)) = self.account_repo.find_by_username("admin").await {
                                existing
                            } else {
                                framed.send(OutboundPacket::ErrorInfo(S2CErrorInfo::new(2, "Credenciais inválidas"))).await?;
                                return Ok(());
                            }
                        }
                    }
                } else {
                    warn!("Tentativa de login com usuário inexistente: '{}'", login.username);
                    framed.send(OutboundPacket::ErrorInfo(S2CErrorInfo::new(2, "Conta inexistente"))).await?;
                    return Ok(());
                };

                if account.is_banned {
                    warn!("Login rejeitado: Conta '{}' está banida", account.username);
                    framed.send(OutboundPacket::ErrorInfo(S2CErrorInfo::new(3, "Conta banida"))).await?;
                    return Ok(());
                }

                session.set_authenticated(account.id, account.username.clone());
                let _ = self.account_repo.update_last_login(account.id, &session.client_ip).await;

                // Envia OnlineAnnounce (Opcode 4) para transição de estado da GUI do cliente
                framed.send(OutboundPacket::OnlineAnnounce(S2COnlineAnnounce::new(
                    account.id,
                    session.session_id as u32,
                ))).await?;

                info!(
                    "Login autenticado com sucesso: '{}' (ID: {}) na Sessão #{} (Realm: {})",
                    account.username, account.id, session.session_id, self.realm_id
                );
            }

            InboundPacket::KeyExchange(_key_ex) => {
                debug!("KeyExchange recebido do cliente na Sessão #{}", session.session_id);
            }

            InboundPacket::RoleList(role_list_req) => {
                let acc_id = if role_list_req.userid > 0 {
                    role_list_req.userid
                } else {
                    session.account_id.unwrap_or(0)
                };

                debug!(
                    "Listando personagens para a Conta ID {} no Realm '{}' (v{})",
                    acc_id, self.realm_id, self.game_version
                );

                let characters = self
                    .char_repo
                    .list_by_account_and_realm(acc_id, &self.realm_id)
                    .await?;

                framed
                    .send(OutboundPacket::RoleListResponse(S2CRoleListResponse::new(
                        acc_id,
                        role_list_req.localsid,
                        characters,
                    )))
                    .await?;
            }

            InboundPacket::CreateRole(create_role) => {
                let acc_id = session.account_id.unwrap_or(create_role.userid);
                
                // Valida compatibilidade da classe com a versão do Realm
                if !self.game_version.is_class_supported(create_role.cls) {
                    warn!(
                        "Criação rejeitada: Classe '{:?}' não é permitida no Realm '{}' (v{})",
                        create_role.cls, self.realm_id, self.game_version
                    );
                    framed.send(OutboundPacket::CreateRoleResponse(S2CCreateRoleResponse {
                        result: 1, // ERR_CREATEROLE
                        role_id: 0,
                        localsid: create_role.localsid,
                        character: None,
                    })).await?;
                    return Ok(());
                }

                info!(
                    "Criando personagem '{}' (Classe: {:?}) para a Conta ID {} no Realm '{}'",
                    create_role.name, create_role.cls, acc_id, self.realm_id
                );

                let raw_appearance_hex = hex::encode(&create_role.custom_appearance);
                let new_role_id_result = self
                    .char_repo
                    .create_character(
                        acc_id,
                        &self.realm_id,
                        &create_role.name,
                        create_role.race,
                        create_role.cls,
                        create_role.gender,
                        create_role.custom_appearance,
                    )
                    .await;

                match new_role_id_result {
                    Ok(new_role_id) => {
                        info!("Personagem '{}' criado com sucesso! (ID: {})", create_role.name, new_role_id);
                        
                        let (sx, sy, sz) = create_role.cls.default_spawn_position();
                        let new_char_summary = CharacterSummary {
                            id: new_role_id,
                            account_id: acc_id,
                            realm_id: self.realm_id.clone(),
                            name: create_role.name.clone(),
                            race: create_role.race,
                            cls: create_role.cls,
                            gender: create_role.gender,
                            level: 1,
                            cultivation: 0,
                            world_id: 1,
                            position: Vector3::new(sx, sy, sz),
                            equipment: Vec::new(),
                            custom_appearance: serde_json::json!({ "raw": raw_appearance_hex }),
                            is_deleted: false,
                            delete_time: None,
                        };

                        // 1. Envia CreateRole_Re (Opcode 0x55) contendo a struct RoleInfo completa
                        framed.send(OutboundPacket::CreateRoleResponse(S2CCreateRoleResponse {
                            result: 0,
                            role_id: new_role_id,
                            localsid: create_role.localsid,
                            character: Some(new_char_summary),
                        })).await?;
                    }
                    Err(e) => {
                        warn!("Falha ao criar personagem '{}': {:?}", create_role.name, e);
                        framed.send(OutboundPacket::CreateRoleResponse(S2CCreateRoleResponse {
                            result: 1,
                            role_id: 0,
                            localsid: create_role.localsid,
                            character: None,
                        })).await?;
                    }
                }
            }

            InboundPacket::DeleteRole(delete_role) => {
                let acc_id = session.account_id.unwrap_or(0);
                info!(
                    "Excluindo personagem ID {} para a Conta ID {} no Realm '{}'",
                    delete_role.role_id, acc_id, self.realm_id
                );

                if let Err(e) = self.char_repo.delete_character(delete_role.role_id).await {
                    warn!("Falha ao excluir personagem ID {}: {:?}", delete_role.role_id, e);
                }

                // 1. Envia confirmação DeleteRole_Re (Opcode 0x57)
                framed.send(OutboundPacket::DeleteRoleResponse(S2CDeleteRoleResponse {
                    result: 0,
                    role_id: delete_role.role_id,
                    localsid: delete_role.localsid,
                })).await?;
            }

            InboundPacket::UndoDeleteRole(undo_delete) => {
                let acc_id = session.account_id.unwrap_or(0);
                info!(
                    "Restaurando personagem ID {} para a Conta ID {} no Realm '{}'",
                    undo_delete.role_id, acc_id, self.realm_id
                );

                if let Err(e) = self.char_repo.restore_character(undo_delete.role_id).await {
                    warn!("Falha ao restaurar personagem ID {}: {:?}", undo_delete.role_id, e);
                }

                // 1. Envia confirmação UndoDeleteRole_Re (Opcode 0x59)
                framed.send(OutboundPacket::UndoDeleteRoleResponse(S2CUndoDeleteRoleResponse {
                    result: 0,
                    role_id: undo_delete.role_id,
                    localsid: undo_delete.localsid,
                })).await?;
            }

            InboundPacket::SelectRole(select_role) => {
                info!(
                    "Personagem ID {} selecionado para entrar no mundo (Realm: '{}')",
                    select_role.role_id, self.realm_id
                );

                let char_details_opt = self.char_repo.get_details(select_role.role_id).await?;

                if let Some(details) = char_details_opt {
                    session.set_in_world(details.id, details.name.clone());

                    // Envia SelectRole_Re (Opcode 0x47) liberando o cliente para iniciar a tela de Loading
                    framed.send(OutboundPacket::SelectRoleResponse(S2CSelectRoleResponse {
                        result: 0,
                        auth: Vec::new(),
                    })).await?;

                    info!("Personagem '{}' (ID: {}) autorizado. Iniciando carregamento da instância...", details.name, details.id);
                } else {
                    warn!("Personagem ID {} não encontrado para entrar no mundo", select_role.role_id);
                    framed.send(OutboundPacket::SelectRoleResponse(S2CSelectRoleResponse {
                        result: 1,
                        auth: Vec::new(),
                    })).await?;
                }
            }

            InboundPacket::EnterWorld(enter_world) => {
                info!(
                    "Personagem ID {} completou o carregamento e enviou EnterWorld (Realm: '{}')",
                    enter_world.role_id, self.realm_id
                );

                let char_details_opt = self.char_repo.get_details(enter_world.role_id).await?;
                if let Some(details) = char_details_opt {
                    info!("Personagem '{}' (ID: {}) entrando no mundo 3D...", details.name, details.id);

                    // 1. Envia INST_DATA_CHECKOUT (Comando 4) - Sincroniza os timestamps exatos de mapa e gshop
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::inst_data_checkout(
                        1, 1156141381, 1156141381, 1206433535
                    ))).await?;

                    // 2. Envia SELF_INFO_00 (Comando 38) - Status vitais e nível do jogador
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                        details.level as i16,
                        details.hp,
                        details.hp,
                        details.mp,
                        details.mp,
                        details.exp as i32,
                        details.sp as i32,
                    ))).await?;

                    // 3. Envia PLAYER_EXT_PROP_MOVE (Comando 54) - Velocidades de movimento (4.8 m/s corrida, 5.0 m/s voo)
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::ext_prop_move(
                        details.id, 4.8, 4.8, 4.0, 5.0
                    ))).await?;

                    // 4. Envia PLAYER_EXT_PROP_BASE (Comando 53) - Atributos básicos
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::ext_prop_base(
                        details.id, 5, 5, 5, 5, details.hp, details.mp, 2, 2
                    ))).await?;

                    // 5. Envia SELF_INFO_1 (Comando 8) - Instancia a entidade local do jogador
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_1(
                        details.exp as i32,
                        details.sp as i32,
                        details.id,
                        details.position,
                    ))).await?;

                    // 6. Envia SKILL_DATA (Comando 90) - Habilidades iniciais oficiais da classe (Voo, Heals, Ataques)
                    let class_skills = details.cls.default_skills();
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::skill_data(&class_skills))).await?;

                    // 7. Envia TASK_DATA (Comando 105) - Inicializa a interface de missões com 3 listas vazias (formato oficial 1.2.6)
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_data())).await?;

                    // 8. Envia OWN_IVTR_DATA (Comando 42) com itens iniciais na bolsa e slots de equipamentos vazios
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_data(32, details.cls.default_weapon_id()))).await?;
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_equip_data())).await?;

                    // 9. Envia OWN_ITEM_INFO (Comando 40) com durabilidade e stats dos itens da bolsa
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                        0, 0, details.cls.default_weapon_id(), 10000, 10000, 1
                    ))).await?;
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                        0, 1, 2100, 10000, 10000, 5
                    ))).await?;
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                        0, 2, 1796, 10000, 10000, 10
                    ))).await?;
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                        0, 3, 1801, 10000, 10000, 10
                    ))).await?;

                    // 10. Envia NPC_INFO_LIST (Comando 9) - Instancia os NPCs e monstros iniciais ao redor do Vale das Plumas
                    let starter_npcs = vec![
                        (20001, 2000, (-741.5, 219.1, -1234.8)), // Ancião / Guia dos Alados
                        (20002, 2125, (-746.7, 219.0, -1257.9)), // Instrutor de Habilidades
                        (20003, 2126, (-772.2, 218.7, -1153.8)), // Mestre dos Alados
                        (20004, 1001, (-730.0, 219.0, -1220.0)), // Monstro Inicial (Besouro / Nível 1)
                        (20005, 1001, (-720.0, 219.0, -1240.0)), // Monstro Inicial (Besouro / Nível 1)
                        (20006, 1001, (-750.0, 219.0, -1210.0)), // Monstro Inicial (Besouro / Nível 1)
                    ];
                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_info_list(&starter_npcs))).await?;

                    // 11. Envia GetUIConfig_Re (Opcode 105 / 0x69) - Dispara OnAllInitDataReady e libera a HUD e o mundo 3D
                    framed.send(OutboundPacket::GetUIConfigRe(S2CGetUIConfigRe {
                        result: 0,
                        role_id: details.id,
                        localsid: session.session_id as u32,
                        ui_config: Vec::new(),
                    })).await?;

                    info!("Personagem '{}' (ID: {}) spawnado com sucesso no mundo 3D (Pos: {:?}, Skills: {}, NPCs: {})!", details.name, details.id, details.position, class_skills.len(), starter_npcs.len());
                }
            }

            InboundPacket::GamedataSend(gamedata) => {
                debug!(
                    "Gamedata recebido do cliente ({} bytes) na Sessão #{}",
                    gamedata.data.len(), session.session_id
                );
                if gamedata.data.len() >= 2 {
                    let cmd = u16::from_le_bytes([gamedata.data[0], gamedata.data[1]]);
                    let role_id = session.role_id.unwrap_or(0);
                    match cmd {
                        0 => {
                            debug!("Movimento do jogador recebido via GamedataSend");
                        }
                        1 => {
                            // C2S::LOGOUT (Gamedata subcomando 1): struct { u16 cmd = 1, i32 iOutType }
                            // iOutType: 0 = _PLAYER_LOGOUT_FULL (Sair do Jogo), 1 = _PLAYER_LOGOUT_HALF (Seleção de Personagem)
                            let out_type = if gamedata.data.len() >= 6 {
                                i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]])
                            } else if gamedata.data.len() >= 3 {
                                gamedata.data[2] as i32
                            } else {
                                1
                            };
                            info!("Jogador ID {} solicitou Logout com out_type: {} (0=Sair do Jogo, 1=Seleção de Personagens)", role_id, out_type);
                            framed.send(OutboundPacket::PlayerLogout(S2CPlayerLogout::new(
                                out_type,
                                role_id,
                                session.session_id as u32,
                            ))).await?;
                        }
                        7 => {
                            debug!("Parada de movimento do jogador recebida via GamedataSend");
                        }
                        9 => {
                            // C2S::GET_ITEM_INFO (Consulta de detalhes de item / durabilidade)
                            if gamedata.data.len() >= 4 {
                                let by_package = gamedata.data[2];
                                let by_slot = gamedata.data[3];
                                debug!("Cliente consultou item_info para pacote {} slot {}", by_package, by_slot);
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                                    by_package, by_slot, 2097, 10000, 10000, 1
                                ))).await?;
                            }
                        }
                        11 => {
                            // C2S::GET_IVTR_DETAIL
                            let by_package = if gamedata.data.len() >= 3 { gamedata.data[2] } else { 0 };
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_data(32, 2097))).await?;
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(by_package, 0, 2097, 10000, 10000, 1))).await?;
                        }
                        12 => {
                            // C2S::EXG_IVTR_ITEM (Troca de posição na bolsa)
                            if gamedata.data.len() >= 4 {
                                let idx1 = gamedata.data[2];
                                let idx2 = gamedata.data[3];
                                info!("Trocando itens nos slots {} e {} da bolsa", idx1, idx2);
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::exg_ivtr_item(idx1, idx2))).await?;
                                // Descongela ambos os slots para liberar a interface no cliente
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx1 as u16))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx2 as u16))).await?;
                            }
                        }
                        13 => {
                            // C2S::MOVE_IVTR_ITEM (Mover item na bolsa)
                            if gamedata.data.len() >= 8 {
                                let src = gamedata.data[2];
                                let dest = gamedata.data[3];
                                let count = u32::from_le_bytes([gamedata.data[4], gamedata.data[5], gamedata.data[6], gamedata.data[7]]);
                                info!("Movendo item do slot {} para {} (qtd: {})", src, dest, count);
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::move_ivtr_item(src, dest, count))).await?;
                                // Descongela os slots de origem e destino
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, src as u16))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, dest as u16))).await?;
                            }
                        }
                        16 => {
                            // C2S::EXG_EQUIP_ITEM (Troca de equipamentos)
                            if gamedata.data.len() >= 4 {
                                let idx1 = gamedata.data[2];
                                let idx2 = gamedata.data[3];
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::exg_equip_item(idx1, idx2))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx1 as u16))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx2 as u16))).await?;
                            }
                        }
                        17 => {
                            // C2S::EQUIP_ITEM (Equipar item da bolsa no corpo)
                            if gamedata.data.len() >= 4 {
                                let idx_inv = gamedata.data[2];
                                let idx_eq = gamedata.data[3];
                                info!("Equipando item da bolsa slot {} no corpo slot {}", idx_inv, idx_eq);
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::equip_item(idx_inv, idx_eq, 1, 0))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(1, idx_eq, 2097, 10000, 10000, 1))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx_inv as u16))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx_eq as u16))).await?;
                            }
                        }
                        18 => {
                            // C2S::MOVE_ITEM_TO_EQUIP
                            if gamedata.data.len() >= 4 {
                                let idx_inv = gamedata.data[2];
                                let idx_eq = gamedata.data[3];
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::move_item_to_equip(idx_inv, idx_eq, 1))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx_inv as u16))).await?;
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx_eq as u16))).await?;
                            }
                        }
                        23..=26 => {
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::ext_prop_move(role_id, 4.8, 4.8, 4.0, 5.0))).await?;
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::ext_prop_base(role_id, 5, 5, 5, 5, 120, 280, 2, 2))).await?;
                        }
                        39 => {
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_data(32, 2097))).await?;
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_equip_data())).await?;
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(0, 0, 2097, 10000, 10000, 1))).await?;
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_data())).await?;
                        }
                        40 => {
                            // C2S::USE_ITEM (Usar item do inventário: where, byCount, index (u16), item_id (i32))
                            if gamedata.data.len() >= 10 {
                                let where_pack = gamedata.data[2];
                                let by_count = gamedata.data[3];
                                let slot = u16::from_le_bytes([gamedata.data[4], gamedata.data[5]]) as u8;
                                let item_id = i32::from_le_bytes([gamedata.data[6], gamedata.data[7], gamedata.data[8], gamedata.data[9]]);
                                info!("Jogador ID {} usou item ID {} do pacote {} slot {} (qtd: {})", role_id, item_id, where_pack, slot, by_count);
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::host_use_item(where_pack, slot, item_id, by_count as u16))).await?;
                                // Descongela o slot de uso
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(where_pack, slot as u16))).await?;

                                // Se for poção de HP (1796) ou MP (1801), atualiza os status vitais
                                if item_id == 1796 || item_id == 1801 {
                                    framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                                        1, 120, 120, 280, 280, 0, 0
                                    ))).await?;
                                }
                            }
                        }
                        41 => {
                            if gamedata.data.len() >= 6 {
                                let skill_id = i16::from_le_bytes([gamedata.data[2], gamedata.data[3]]);
                                debug!("Jogador ID {} usou a habilidade ID {}", role_id, skill_id);
                            }
                        }
                        42 => {
                            // C2S::CANCEL_ACTION (Cancelar ação / Parar / Levantar)
                            info!("Jogador ID {} cancelou ação / levantou", role_id);
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_stand_up(role_id))).await?;
                        }
                        46 => {
                            // C2S::SIT_DOWN (Meditar / Sentar)
                            info!("Jogador ID {} sentou / iniciou meditação", role_id);
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_sit_down(role_id))).await?;
                        }
                        47 => {
                            // C2S::STAND_UP (Levantar)
                            info!("Jogador ID {} levantou da meditação", role_id);
                            framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_stand_up(role_id))).await?;
                        }
                        48 => {
                            // C2S::EMOTE_ACTION (Ações / Emotes)
                            if gamedata.data.len() >= 4 {
                                let emotion = u16::from_le_bytes([gamedata.data[2], gamedata.data[3]]);
                                info!("Jogador ID {} executou emote {}", role_id, emotion);
                                framed.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_do_emote(role_id, emotion))).await?;
                            }
                        }
                        75 => {
                            // C2S::ENTER_SANCTUARY
                            debug!("Jogador ID {} entrou em santuário / zona segura", role_id);
                        }
                        110 => {
                            debug!("Cliente consultou saldo de Cash (Mall)");
                        }
                        _ => {
                            debug!("Gamedata subcomando {} recebido do cliente", cmd);
                        }
                    }
                }
            }

            InboundPacket::GetUIConfig(req) => {
                framed.send(OutboundPacket::GetUIConfigRe(S2CGetUIConfigRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                    ui_config: Vec::new(),
                })).await?;
            }

            InboundPacket::SetUIConfig(req) => {
                debug!("Salvando UIConfig ({} bytes) para o personagem ID {}", req.ui_config.len(), req.role_id);
                framed.send(OutboundPacket::SetUIConfigRe(S2CSetUIConfigRe {
                    result: 0,
                    role_id: req.role_id,
                })).await?;
            }

            InboundPacket::SetCustomData(req) => {
                debug!("Salvando CustomData ({} bytes) para o personagem ID {}", req.data.len(), req.role_id);
                framed.send(OutboundPacket::SetCustomDataRe(S2CSetCustomDataRe {
                    result: 0,
                    role_id: req.role_id,
                })).await?;
            }

            InboundPacket::GetFriendList(req) => {
                framed.send(OutboundPacket::GetFriendListRe(S2CGetFriendListRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::GetWaitDelRoles(req) => {
                framed.send(OutboundPacket::GetWaitDelRolesRe(S2CGetWaitDelRolesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::GetHelpStates(req) => {
                framed.send(OutboundPacket::GetHelpStatesRe(S2CGetHelpStatesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                    help_states: vec![0u8; 32],
                })).await?;
            }

            InboundPacket::SetHelpStates(req) => {
                debug!("Salvando HelpStates ({} bytes) para o personagem ID {}", req.help_states.len(), req.role_id);
                framed.send(OutboundPacket::SetHelpStatesRe(S2CSetHelpStatesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::QueryServerTime(_) => {
                debug!("Pacote 0x352 (BattleGetMap) recebido na Sessão #{}", session.session_id);
            }

            InboundPacket::PlayerMove(move_pkt) => {
                let move_broadcast = OutboundPacket::PlayerMoveBroadcast(S2CPlayerMoveBroadcast {
                    role_id: session.role_id.unwrap_or(0),
                    mode: move_pkt.mode,
                    position: move_pkt.position,
                    target: move_pkt.target,
                    speed: move_pkt.speed,
                    timestamp: move_pkt.timestamp,
                });

                let payload = serde_json::to_string(&move_pkt).unwrap_or_default();
                let _ = self
                    .cache_manager
                    .publish_event(&format!("grid:{}:move", self.realm_id), &payload)
                    .await;

                framed.send(move_broadcast).await?;
            }

            InboundPacket::PlayerChat(chat_pkt) => {
                info!(
                    "Chat [Canal {}] de Sessão #{}: {}",
                    chat_pkt.channel, session.session_id, chat_pkt.message
                );

                let sender_name = session
                    .character_name
                    .clone()
                    .or_else(|| session.username.clone())
                    .unwrap_or_else(|| "Jogador".to_string());

                let broadcast_pkt = OutboundPacket::ChatBroadcast(S2CChatBroadcast {
                    channel: chat_pkt.channel,
                    sender_id: session.role_id.unwrap_or(0),
                    sender_name,
                    message: chat_pkt.message,
                });

                framed.send(broadcast_pkt).await?;
            }

            InboundPacket::Heartbeat(_hb) => {
                debug!("Heartbeat recebido na Sessão #{}", session.session_id);
            }

            InboundPacket::ACReport(ac) => {
                debug!("Relatório Anti-Cheat ({} bytes) recebido na Sessão #{}", ac.data.len(), session.session_id);
            }

            InboundPacket::Unknown { opcode, payload } => {
                debug!("Pacote bruto recebido (Opcode: 0x{:X}, {} bytes) na Sessão #{}", opcode, payload.len(), session.session_id);
            }
        }

        Ok(())
    }
}
