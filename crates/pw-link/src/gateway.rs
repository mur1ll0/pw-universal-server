use futures::{SinkExt, StreamExt};
use pw_core::{CharacterClass, CharacterSummary, Vector3};
use pw_crypto::generate_login_challenge;
use pw_protocol::{
    create_protocol_adapter, GameVersion, InboundPacket, OctetsStream, OutboundPacket, ProtocolAdapter,
    PwPacketCodec, S2CChatBroadcast, S2CChallenge, S2CCreateRoleResponse, S2CDeleteRoleResponse,
    S2CErrorInfo, S2CGamedataSend, S2CGetFriendListRe, S2CGetHelpStatesRe, S2CGetUIConfigRe, S2CGetWaitDelRolesRe,
    S2COnlineAnnounce, S2CPlayerLogout, S2CPlayerMoveBroadcast, S2CRoleListResponse, S2CSelectRoleResponse,
    S2CSetCustomDataRe, S2CSetHelpStatesRe, S2CSetUIConfigRe, S2CUndoDeleteRoleResponse,
};
use pw_data_loader::GameDataManager;
use pw_storage::{AccountRepository, CacheManager, CharacterRepository};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, info, trace, warn};

use crate::session::ClientSession;

pub struct LinkGateway {
    pub realm_id: String,
    pub game_version: GameVersion,
    pub adapter: Arc<dyn ProtocolAdapter>,
    pub listen_port: u16,
    pub account_repo: AccountRepository,
    pub char_repo: CharacterRepository,
    pub cache_manager: CacheManager,
    pub data_manager: Arc<GameDataManager>,
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

        let mut data_manager = GameDataManager::new();
        let possible_dirs = [
            format!("data/{}/config", realm_id),
            "data/config".to_string(),
            "/app/data/config".to_string(),
            format!("/app/data/{}/config", realm_id),
            "config".to_string(),
        ];
        let mut loaded = false;
        for dir_str in &possible_dirs {
            let p = std::path::Path::new(dir_str);
            if p.exists() {
                info!("Carregando arquivos de configuração e mapas de {:?}", p);
                if let Err(e) = data_manager.load_from_directory(p) {
                    warn!("Aviso: Erro ao carregar templates de dados de {:?}: {:?}", p, e);
                } else {
                    loaded = true;
                    break;
                }
            }
        }
        if !loaded {
            warn!("Nenhum diretório de configuração .data foi encontrado nas buscas: {:?}", possible_dirs);
        }

        Self {
            realm_id,
            game_version,
            adapter,
            listen_port,
            account_repo,
            char_repo,
            cache_manager,
            data_manager: Arc::new(data_manager),
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

        let (tx, mut rx) = tokio::sync::mpsc::channel::<OutboundPacket>(256);
        tx.send(challenge_packet).await?;
        debug!("Challenge v{} enviado para a Sessão #{}", self.game_version, session_id);

        // 2. Loop concorrente de processamento e envio de pacotes
        loop {
            tokio::select! {
                Some(out_pkt) = rx.recv() => {
                    if let Err(e) = framed.send(out_pkt).await {
                        warn!("Erro ao enviar pacote para Sessão #{}: {:?}", session_id, e);
                        break;
                    }
                }
                msg = framed.next() => {
                    match msg {
                        Some(Ok(packet)) => {
                            if let Err(e) = self.dispatch_packet(&tx, &mut session, packet).await {
                                warn!("Erro no dispatch da Sessão #{}: {:?}", session_id, e);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            warn!("Erro de codec na Sessão #{}: {:?}", session_id, e);
                            break;
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }

        info!("Sessão #{} ({}) finalizada.", session_id, client_ip);
        Ok(())
    }

    async fn dispatch_packet(
        &self,
        tx: &tokio::sync::mpsc::Sender<OutboundPacket>,
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
                                tx.send(OutboundPacket::ErrorInfo(S2CErrorInfo::new(2, "Credenciais inválidas"))).await?;
                                return Ok(());
                            }
                        }
                    }
                } else {
                    warn!("Tentativa de login com usuário inexistente: '{}'", login.username);
                    tx.send(OutboundPacket::ErrorInfo(S2CErrorInfo::new(2, "Conta inexistente"))).await?;
                    return Ok(());
                };

                if account.is_banned {
                    warn!("Login rejeitado: Conta '{}' está banida", account.username);
                    tx.send(OutboundPacket::ErrorInfo(S2CErrorInfo::new(3, "Conta banida"))).await?;
                    return Ok(());
                }

                session.set_authenticated(account.id, account.username.clone());
                session.sec_level = account.gm_privileges.clamp(0, 32) as u8;
                let _ = self.account_repo.update_last_login(account.id, &session.client_ip).await;

                // Envia OnlineAnnounce (Opcode 4) para transição de estado da GUI do cliente
                tx.send(OutboundPacket::OnlineAnnounce(S2COnlineAnnounce::new(
                    account.id,
                    session.session_id as u32,
                ))).await?;

                info!(
                    "Login autenticado com sucesso: '{}' (ID: {}, GM Priv: {}) na Sessão #{} (Realm: {})",
                    account.username, account.id, session.sec_level, session.session_id, self.realm_id
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

                tx.send(OutboundPacket::RoleListResponse(S2CRoleListResponse::new(
                    acc_id,
                    role_list_req.localsid,
                    characters,
                ))).await?;
            }

            InboundPacket::CreateRole(create_role) => {
                let acc_id = session.account_id.unwrap_or(create_role.userid);
                
                // Valida compatibilidade da classe com a versão do Realm
                if !self.game_version.is_class_supported(create_role.cls) {
                    warn!(
                        "Criação rejeitada: Classe '{:?}' não é permitida no Realm '{}' (v{})",
                        create_role.cls, self.realm_id, self.game_version
                    );
                    tx.send(OutboundPacket::CreateRoleResponse(S2CCreateRoleResponse {
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
                        
                        let equips = self.char_repo.item_repo().list_by_container(new_role_id, pw_core::ContainerType::Equipment).await.unwrap_or_default();
                        let details_opt = self.char_repo.get_details(new_role_id).await.unwrap_or(None);
                        let (level, cultivation, world_id, pos) = if let Some(ref d) = details_opt {
                            (d.level, d.cultivation, d.world_id, d.position)
                        } else {
                            let (sx, sy, sz) = create_role.cls.default_spawn_position();
                            (1, 0, 1, Vector3::new(sx, sy, sz))
                        };

                        let new_char_summary = CharacterSummary {
                            id: new_role_id,
                            account_id: acc_id,
                            realm_id: self.realm_id.clone(),
                            name: create_role.name.clone(),
                            race: create_role.race,
                            cls: create_role.cls,
                            gender: create_role.gender,
                            level,
                            cultivation,
                            world_id,
                            position: pos,
                            equipment: equips,
                            custom_appearance: serde_json::json!({ "raw": raw_appearance_hex }),
                            is_deleted: false,
                            delete_time: None,
                        };

                        // 1. Envia CreateRole_Re (Opcode 0x55) contendo a struct RoleInfo completa
                        tx.send(OutboundPacket::CreateRoleResponse(S2CCreateRoleResponse {
                            result: 0,
                            role_id: new_role_id,
                            localsid: create_role.localsid,
                            character: Some(new_char_summary),
                        })).await?;
                    }
                    Err(e) => {
                        warn!("Falha ao criar personagem '{}': {:?}", create_role.name, e);
                        tx.send(OutboundPacket::CreateRoleResponse(S2CCreateRoleResponse {
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
                tx.send(OutboundPacket::DeleteRoleResponse(S2CDeleteRoleResponse {
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
                tx.send(OutboundPacket::UndoDeleteRoleResponse(S2CUndoDeleteRoleResponse {
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
                    let mut auth = Vec::new();
                    if session.sec_level > 0 {
                        auth = vec![0xFF; 32]; // 256 bits de GM privilege
                    }
                    tx.send(OutboundPacket::SelectRoleResponse(S2CSelectRoleResponse {
                        result: 0,
                        auth,
                    })).await?;

                    info!("Personagem '{}' (ID: {}, GM: {}) autorizado. Iniciando carregamento da instância...", details.name, details.id, session.sec_level);
                } else {
                    warn!("Personagem ID {} não encontrado para entrar no mundo", select_role.role_id);
                    tx.send(OutboundPacket::SelectRoleResponse(S2CSelectRoleResponse {
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

                    // 1. Envia INST_DATA_CHECKOUT / SERVER_CONFIG_DATA (Comando 206) - Sincroniza timestamps do mundo (1, 2097199, 2097199, 1206433535)
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::inst_data_checkout(
                        1, 2097199, 2097199, 1206433535
                    ))).await?;

                    // 2. Envia SELF_INFO_00 (Comando 38) - Status vitais, nível e permissão de GM
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                        details.level as i16,
                        session.sec_level,
                        details.hp,
                        details.hp,
                        details.mp,
                        details.mp,
                        details.exp as i32,
                        details.sp as i32,
                    ))).await?;

                    // 3. Envia PLAYER_EXT_PROP_MOVE (Comando 54) - Velocidades de movimento (4.8 m/s corrida, 5.0 m/s voo)
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::ext_prop_move(
                        details.id, 4.8, 4.8, 4.0, 5.0
                    ))).await?;

                    // 4. Envia PLAYER_EXT_PROP_BASE (Comando 53) - Atributos básicos
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::ext_prop_base(
                        details.id, 5, 5, 5, 5, details.hp, details.mp, 2, 2
                    ))).await?;

                    // 5. Envia SELF_INFO_1 (Comando 8) - Instancia a entidade local do jogador
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_1(
                        details.exp as i32,
                        details.sp as i32,
                        details.id,
                        details.position,
                        session.sec_level,
                    ))).await?;

                    // 6. Envia SKILL_DATA (Comando 90) - Habilidades carregadas da tabela character_skills
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::skill_data_from_records(&details.skills))).await?;

                    // 7. Envia TASK_DATA (Comando 105) e inicializa o subsistema de missões do cliente
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_data())).await?;
                    let mut dyn_mark = OctetsStream::new();
                    dyn_mark.write_u8(8);       // reason = TASK_SVR_NOTIFY_DYN_TIME_MARK (8)
                    dyn_mark.write_u16_le(0);   // task = 0 (2B)
                    dyn_mark.write_u32_le(0);   // time_mark = 0 (4B)
                    dyn_mark.write_u16_le(0);   // dyn_task_count = 0 (2B) - Exactly 9 bytes
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_var_data(&dyn_mark.into_bytes()))).await?;

                    // Carrega e sincroniza missões ativas do personagem
                    let role_quests = self.char_repo.quest_repo().list_quests(details.id).await.unwrap_or_default();
                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;

                    if role_quests.is_empty() {
                        // Novo personagem: entrega a missão inicial de nascimento configurada no tasks.data
                        let initial_task_id = match details.cls {
                            CharacterClass::Cleric | CharacterClass::Archer => 9374,       // Missão dos Alados
                            CharacterClass::Blademaster | CharacterClass::Wizard => 1,      // Missão dos Humanos
                            CharacterClass::Barbarian | CharacterClass::Venomancer => 9375, // Missão dos Selvagens
                            _ => 1,
                        };
                        info!("Registrando missão inicial ID {} para o novo personagem '{}'", initial_task_id, details.name);
                        let _ = self.char_repo.quest_repo().save_quest(details.id, initial_task_id, pw_core::QuestStatus::Active, &[0, 0, 0], None).await;
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_notify_new(initial_task_id as u16, now_ts))).await?;
                    } else {
                        for q in role_quests {
                            if q.status == pw_core::QuestStatus::Active {
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_notify_new(q.quest_id as u16, now_ts))).await?;
                            }
                        }
                    }

                    // 8. Envia OWN_IVTR_DATA (Comando 42)
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(0, 32, &details.inventory))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(1, 32, &details.equipment))).await?;

                    // 9. Envia OWN_ITEM_INFO (Comando 40) para cada item
                    for item in &details.inventory {
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                            0, item.slot as u8, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets
                        ))).await?;
                    }
                    for item in &details.equipment {
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                            1, item.slot as u8, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets
                        ))).await?;
                    }

                    // 10. Envia NPC_ENTER_SLICE / NPC_ENTER_WORLD e NPC_INFO_00 no raio de 120m
                    let mut nearby_npcs = Vec::new();
                    if let Some(world_spawns) = self.data_manager.map_spawns.get(&1) {
                        let nearby = world_spawns.query_nearby(details.position, 120.0);
                        for spawn in nearby.into_iter().take(60) {
                            let dir_byte = pw_data_loader::compress_dir_h(spawn.dir.x, spawn.dir.z);
                            nearby_npcs.push((spawn.instance_id, spawn.template_id as i32, (spawn.pos.x, spawn.pos.y, spawn.pos.z), dir_byte));
                        }
                    }

                    if nearby_npcs.is_empty() {
                        let anc_id = (0x80000000u32 | 1001) as i32;
                        let ins_id = (0x80000000u32 | 1002) as i32;
                        let mes_id = (0x80000000u32 | 1003) as i32;
                        let alq_id = (0x80000000u32 | 1004) as i32;
                        let fer_id = (0x80000000u32 | 1005) as i32;
                        let mon_id = (0x80000000u32 | 1006) as i32;
                        nearby_npcs = match details.cls {
                            CharacterClass::Cleric | CharacterClass::Archer => vec![
                                (anc_id, 2191, (-722.0, 219.1, -1222.6), 64),  // Anciã do Vale das Plumas
                                (mes_id, 2190, (-746.7, 219.0, -1257.9), 128), // Mestre dos Alados
                                (ins_id, 2182, (-727.0, 219.2, -1244.8), 64),  // Instrutor de Habilidades
                                (alq_id, 2187, (-755.2, 221.8, -1353.9), 32),  // Alquimista do Vale das Plumas
                                (fer_id, 2189, (-797.9, 219.5, -1309.3), 0),   // Ferreiro do Vale das Plumas
                                (mon_id, 13641, (-726.3, 219.4, -1096.8), 0),  // Monstro Inicial
                            ],
                            CharacterClass::Blademaster | CharacterClass::Wizard => vec![
                                (anc_id, 2175, (438.0, 21.0, 676.0), 0),        // Ancião da Cidade das Espadas
                                (mes_id, 4469, (435.0, 21.0, 670.0), 64),       // Mestre Guerreiro
                                (ins_id, 4472, (440.0, 21.0, 670.0), 128),      // Mestre Mago
                                (mon_id, 1001, (430.0, 21.0, 650.0), 0),        // Monstro Inicial
                            ],
                            CharacterClass::Barbarian | CharacterClass::Venomancer => vec![
                                (anc_id, 2206, (-141.0, 21.0, -289.0), 0),      // Ancião da Cidade das Feras
                                (mes_id, 4475, (-145.0, 21.0, -285.0), 64),     // Mestre Bárbaro
                                (ins_id, 4480, (-138.0, 21.0, -285.0), 128),    // Mestre Feiticeira
                                (mon_id, 1001, (-150.0, 21.0, -300.0), 0),      // Monstro Inicial
                            ],
                            _ => vec![
                                (anc_id, 2191, (-722.0, 219.1, -1222.6), 64),
                            ],
                        };
                    }

                    info!("Enviando {} entidades (NPCs/Monstros) com HP e dados exatos ao redor da posição {:?} para o jogador '{}'", nearby_npcs.len(), details.position, details.name);
                    for spawn in nearby_npcs {
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_enter_world(
                            spawn.0,
                            spawn.1,
                            Vector3::new(spawn.2.0, spawn.2.1, spawn.2.2),
                            spawn.3,
                        ))).await?;
                    }

                    // 10.5 Envia saldo inicial de Gold/Cash (500.00 Gold = 50000 centavos)
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(50000, 0))).await?;

                    // 11. Envia GetUIConfig_Re (Opcode 105 / 0x69) - Desperta OnPrtcGetConfigRe, ativa OnAllInitDataReady e destrava a tela de Loading
                    tx.send(OutboundPacket::GetUIConfigRe(S2CGetUIConfigRe::new(
                        details.id,
                        session.session_id as u32,
                        &[],
                    ))).await?;

                    info!("Personagem '{}' (ID: {}) spawnado com sucesso no mundo 3D (Pos: {:?}, Skills: {}, Itens: {})!", details.name, details.id, details.position, details.skills.len(), details.inventory.len());
                }
            }

            InboundPacket::GamedataSend(gamedata) => {
                let cmd = if gamedata.data.len() >= 2 {
                    u16::from_le_bytes([gamedata.data[0], gamedata.data[1]])
                } else {
                    0
                };
                debug!(
                    "Gamedata recebido do cliente ({} bytes, cmd={}): {:02x?}",
                    gamedata.data.len(), cmd, gamedata.data
                );
                if gamedata.data.len() >= 2 {
                    let cmd = u16::from_le_bytes([gamedata.data[0], gamedata.data[1]]);
                    let role_id = session.role_id.unwrap_or(0);
                    match cmd {
                        0 => {
                            // C2S 0: PLAYER_MOVE (Sincronização de movimento e persistência no banco)
                            if gamedata.data.len() >= 14 {
                                let px = f32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                let py = f32::from_le_bytes([gamedata.data[6], gamedata.data[7], gamedata.data[8], gamedata.data[9]]);
                                let pz = f32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                trace!("Movimento do jogador {}: ({:.2}, {:.2}, {:.2})", role_id, px, py, pz);
                                let pos = Vector3::new(px, py, pz);
                                let _ = self.char_repo.update_position(role_id, &pos).await;
                            }
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
                            tx.send(OutboundPacket::PlayerLogout(S2CPlayerLogout::new(
                                out_type,
                                role_id,
                                session.session_id as u32,
                            ))).await?;
                        }
                        2 => {
                            // C2S 2: SELECT_TARGET (Selecionar Alvo e atualizar HP no HUD)
                            if gamedata.data.len() >= 6 {
                                let target_id = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                session.set_target(target_id);
                                trace!("Jogador ID {} selecionou alvo ID {}", role_id, target_id);
                                if target_id == 0 {
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unselect())).await?;
                                } else {
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::select_target(target_id))).await?;
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_info_00(target_id, 1000, 1000))).await?;
                                }
                            }
                        }
                        3 => {
                            // C2S 3: NORMAL_ATTACK (Ataque Básico)
                            let target_id = if gamedata.data.len() >= 6 {
                                i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]])
                            } else {
                                session.target_id.unwrap_or(0)
                            };
                            if target_id != 0 {
                                info!("Jogador ID {} atacou o alvo ID {}", role_id, target_id);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::host_attack_result(target_id, 35, 0))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_info_00(target_id, 965, 1000))).await?;

                                // Se for monstro/entidade, envia notificação de progresso de abate para missões ativas
                                if (target_id as u32 & 0x80000000) != 0 {
                                    let active_quests = self.char_repo.quest_repo().list_quests(role_id).await.unwrap_or_default();
                                    for q in active_quests {
                                        if q.status == pw_core::QuestStatus::Active {
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_notify_monster_killed(q.quest_id as u16, 13641, 1))).await?;
                                        }
                                    }
                                }
                            }
                        }
                        7 => {
                            trace!("Parada de movimento do jogador recebida via GamedataSend");
                            if gamedata.data.len() >= 14 {
                                let px = f32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                let py = f32::from_le_bytes([gamedata.data[6], gamedata.data[7], gamedata.data[8], gamedata.data[9]]);
                                let pz = f32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                let pos = Vector3::new(px, py, pz);
                                let _ = self.char_repo.update_position(role_id, &pos).await;
                            }
                        }
                        8 => {
                            // C2S 8: UNSELECT
                            session.clear_target();
                            trace!("Jogador ID {} desmarcou alvo", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unselect())).await?;
                        }
                        9 => {
                            // C2S::GET_ITEM_INFO (Consulta de detalhes de item / durabilidade do banco)
                            if gamedata.data.len() >= 4 {
                                let by_package = gamedata.data[2];
                                let by_slot = gamedata.data[3];
                                let ctype = pw_core::ContainerType::from_i16(by_package as i16);
                                if let Ok(Some(item)) = self.char_repo.item_repo().get_item_by_slot(role_id, ctype, by_slot as u16).await {
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                                        by_package, by_slot, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets
                                    ))).await?;
                                }
                            }
                        }
                        11 => {
                            // C2S::GET_IVTR_DETAIL
                            let by_package = if gamedata.data.len() >= 3 { gamedata.data[2] } else { 0 };
                            let ctype = pw_core::ContainerType::from_i16(by_package as i16);
                            let items = self.char_repo.item_repo().list_by_container(role_id, ctype).await.unwrap_or_default();
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(by_package, 32, &items))).await?;
                        }
                        12 => {
                            // C2S::EXG_IVTR_ITEM (Troca de posição na bolsa)
                            if gamedata.data.len() >= 4 {
                                let idx1 = gamedata.data[2];
                                let idx2 = gamedata.data[3];
                                info!("Trocando itens nos slots {} e {} da bolsa no banco de dados", idx1, idx2);
                                let _ = self.char_repo.item_repo().swap_slots(role_id, pw_core::ContainerType::Inventory, idx1 as u16, idx2 as u16).await;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::exg_ivtr_item(idx1, idx2))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx1 as u16))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx2 as u16))).await?;
                            }
                        }
                        13 => {
                            // C2S::MOVE_IVTR_ITEM (Mover item na bolsa)
                            if gamedata.data.len() >= 8 {
                                let src = gamedata.data[2];
                                let dest = gamedata.data[3];
                                let count = u32::from_le_bytes([gamedata.data[4], gamedata.data[5], gamedata.data[6], gamedata.data[7]]) as u16;
                                info!("Movendo item do slot {} para {} (qtd: {}) no banco de dados", src, dest, count);
                                let _ = self.char_repo.item_repo().swap_slots(role_id, pw_core::ContainerType::Inventory, src as u16, dest as u16).await;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::move_ivtr_item(src, dest, count))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, src as u16))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, dest as u16))).await?;
                            }
                        }
                        16 => {
                            // C2S::EXG_EQUIP_ITEM (Troca de equipamentos)
                            if gamedata.data.len() >= 4 {
                                let idx1 = gamedata.data[2];
                                let idx2 = gamedata.data[3];
                                let _ = self.char_repo.item_repo().swap_slots(role_id, pw_core::ContainerType::Equipment, idx1 as u16, idx2 as u16).await;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::exg_equip_item(idx1, idx2))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx1 as u16))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx2 as u16))).await?;
                            }
                        }
                        17 => {
                            // C2S::EQUIP_ITEM (Equipar ou Desequipar com suporte bidirecional)
                            if gamedata.data.len() >= 4 {
                                let idx_inv = gamedata.data[2];
                                let idx_eq = gamedata.data[3];
                                
                                let inv_before = self.char_repo.item_repo().get_item_by_slot(role_id, pw_core::ContainerType::Inventory, idx_inv as u16).await.unwrap_or(None);
                                let eq_before = self.char_repo.item_repo().get_item_by_slot(role_id, pw_core::ContainerType::Equipment, idx_eq as u16).await.unwrap_or(None);
                                
                                info!(
                                    "EQUIP_ITEM: role={}, bolsa slot {} (tem: {}), corpo slot {} (tem: {})",
                                    role_id, idx_inv, inv_before.is_some(), idx_eq, eq_before.is_some()
                                );
                                
                                let _ = self.char_repo.item_repo().move_between_containers(
                                    role_id,
                                    pw_core::ContainerType::Inventory, idx_inv as u16,
                                    pw_core::ContainerType::Equipment, idx_eq as u16
                                ).await;
                                
                                let inv_after = self.char_repo.item_repo().get_item_by_slot(role_id, pw_core::ContainerType::Inventory, idx_inv as u16).await.unwrap_or(None);
                                let eq_after = self.char_repo.item_repo().get_item_by_slot(role_id, pw_core::ContainerType::Equipment, idx_eq as u16).await.unwrap_or(None);
                                
                                let count_inv = if inv_after.is_some() { 1 } else { 0 };
                                let count_eq = if eq_after.is_some() { 1 } else { 0 };
                                
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::equip_item(idx_inv, idx_eq, count_inv, count_eq))).await?;
                                
                                if let Some(item) = inv_after {
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                                        0, idx_inv, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets
                                    ))).await?;
                                }
                                if let Some(item) = eq_after {
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                                        1, idx_eq, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets
                                    ))).await?;
                                }
                                
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx_inv as u16))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx_eq as u16))).await?;
                            }
                        }
                        18 => {
                            // C2S::MOVE_ITEM_TO_EQUIP
                            if gamedata.data.len() >= 4 {
                                let idx_inv = gamedata.data[2];
                                let idx_eq = gamedata.data[3];
                                let _ = self.char_repo.item_repo().move_between_containers(role_id, pw_core::ContainerType::Inventory, idx_inv as u16, pw_core::ContainerType::Equipment, idx_eq as u16).await;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::move_item_to_equip(idx_inv, idx_eq, 1))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, idx_inv as u16))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(1, idx_eq as u16))).await?;
                            }
                        }
                        23..=26 => {
                            // Subcomandos de ação de movimento / pulo / voo (Takeoff / Landing)
                            if gamedata.data.len() >= 3 {
                                let flight_act = gamedata.data[2];
                                if flight_act == 1 {
                                    info!("Jogador ID {} decolou para voo", role_id);
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_takeoff(role_id))).await?;
                                } else if flight_act == 2 {
                                    info!("Jogador ID {} pousou do voo", role_id);
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_landing(role_id))).await?;
                                }
                            }
                        }
                        27 => {
                            // C2S 27: TEAM_INVITE (Convidar jogador para grupo)
                            if gamedata.data.len() >= 6 {
                                let dst_roleid = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                info!("Jogador ID {} convidou jogador ID {} para grupo", role_id, dst_roleid);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::team_leader_invite(role_id))).await?;
                            }
                        }
                        28 => {
                            // C2S 28: TEAM_AGREE_INVITE (Aceitar convite de grupo)
                            if gamedata.data.len() >= 6 {
                                let leader_id = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                info!("Jogador ID {} aceitou entrar no grupo do líder {}", role_id, leader_id);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::team_join_party(role_id, leader_id))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::team_member_data(&[
                                    (leader_id, 1, 120, 120, 280, 280, (-718.0, 218.0, -1217.0)),
                                    (role_id, 1, 120, 120, 280, 280, (-718.4, 218.9, -1217.0)),
                                ]))).await?;
                            }
                        }
                        30 => {
                            // C2S 30: TEAM_LEAVE_PARTY (Sair do grupo)
                            info!("Jogador ID {} saiu do grupo", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::team_leave_party(role_id, 0))).await?;
                        }
                        32 | 35 => {
                            // C2S 32 / 35: SEVNPC_HELLO (Abrir diálogo com NPC)
                            if gamedata.data.len() >= 6 {
                                let nid = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                info!("Jogador ID {} iniciou diálogo com o NPC ID {}", role_id, nid);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_greeting(nid))).await?;
                            }
                        }
                        33 | 37 => {
                            // C2S 33 / 37: SEVNPC_SERVE (Serviços de NPC: Quests, Loja, Reparo, Forja, Skills, etc.)
                            if gamedata.data.len() >= 6 {
                                let service_type = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;

                                match service_type {
                                    1 => {
                                        // GP_NPCSEV_SELL (Vender item para NPC)
                                        if gamedata.data.len() >= 12 {
                                            let slot = gamedata.data[10];
                                            let count = if gamedata.data.len() >= 12 { u16::from_le_bytes([gamedata.data[10], gamedata.data[11]]) } else { 1 };
                                            info!("Jogador ID {} vendeu item no slot {} (qtd: {}) para NPC", role_id, slot, count);
                                            let _ = self.char_repo.item_repo().delete_item_by_slot(role_id, pw_core::ContainerType::Inventory, slot as u16).await;
                                            let _ = self.char_repo.add_money(role_id, 50).await;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, slot as u16))).await?;
                                        }
                                    }
                                    2 => {
                                        // GP_NPCSEV_BUY (Comprar item da loja do NPC)
                                        if gamedata.data.len() >= 14 {
                                            let item_id = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} comprou item ID {} do NPC", role_id, item_id);
                                            let _ = self.char_repo.deduct_money(role_id, 100).await;
                                            let _ = self.char_repo.item_repo().upsert_item(&pw_core::ItemRecord {
                                                id: None,
                                                character_id: role_id,
                                                container_type: pw_core::ContainerType::Inventory,
                                                slot: 10,
                                                item_id: item_id as u32,
                                                count: 1,
                                                max_count: 100,
                                                refine_level: 0,
                                                sockets_count: 0,
                                                sockets: vec![],
                                                durability: 10000,
                                                max_durability: 10000,
                                                bind_status: 0,
                                                octets: vec![],
                                                custom_attributes: serde_json::json!({}),
                                            }).await;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(0, 10, item_id, 10000, 10000, 1, &[]))).await?;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, 10))).await?;
                                        }
                                    }
                                    3 => {
                                        // GP_NPCSEV_REPAIR (Reparo de equipamentos)
                                        info!("Jogador ID {} reparou todos os equipamentos no Ferreiro", role_id);
                                        let _ = self.char_repo.deduct_money(role_id, 150).await;
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::repair_all(150))).await?;
                                    }
                                    4 => {
                                        // GP_NPCSEV_HEAL (Cura e restauração completa de HP/MP)
                                        info!("Jogador ID {} curou HP/MP no NPC", role_id);
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                                            1, session.sec_level, 120, 120, 280, 280, 0, 0
                                        ))).await?;
                                    }
                                    5 => {
                                        // GP_NPCSEV_TRANSMIT (Teleporte de mapa)
                                        info!("Jogador ID {} utilizou teleporte do NPC", role_id);
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::notify_hostpos(
                                            Vector3::new(-718.4, 218.9, -1217.0), 0
                                        ))).await?;
                                    }
                                    6 => {
                                        // GP_NPCSEV_TASK_RETURN (Entregar / Concluir Missão)
                                        if gamedata.data.len() >= 14 {
                                            let id_task = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} entregou/completou a missão ID {}", role_id, id_task);
                                            let _ = self.char_repo.quest_repo().save_quest(role_id, id_task as u32, pw_core::QuestStatus::Completed, &[0, 0, 0], None).await;
                                            let _ = self.char_repo.add_exp_sp(role_id, 1500, 320).await;
                                            let _ = self.char_repo.add_money(role_id, 500).await;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_notify_complete(id_task as u16, now_ts))).await?;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::receive_exp(1500, 320))).await?;
                                        }
                                    }
                                    7 => {
                                        // GP_NPCSEV_TASK_ACCEPT (Aceitar Missão)
                                        if gamedata.data.len() >= 14 {
                                            let id_task = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} aceitou missão ID {} com sucesso", role_id, id_task);
                                            let _ = self.char_repo.quest_repo().save_quest(role_id, id_task as u32, pw_core::QuestStatus::Active, &[0, 0, 0], None).await;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_notify_new(id_task as u16, now_ts))).await?;
                                        }
                                    }
                                    8 => {
                                        // GP_NPCSEV_TASK_MATTER (Item de Missão)
                                        if gamedata.data.len() >= 14 {
                                            let id_task = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} solicitou item de missão ID {}", role_id, id_task);
                                        }
                                    }
                                    9 => {
                                        // GP_NPCSEV_LEARN (Aprender Habilidade no Mestre)
                                        if gamedata.data.len() >= 14 {
                                            let skill_id = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} aprendeu/subiu de nível a habilidade ID {}", role_id, skill_id);
                                            let _ = self.char_repo.skill_repo().learn_or_upgrade(role_id, skill_id as u32, 2).await;
                                            let _ = self.char_repo.deduct_money(role_id, 200).await;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::learn_skill(skill_id, 2))).await?;
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::cost_skill_point(150))).await?;
                                            if let Ok(skills) = self.char_repo.skill_repo().list_skills(role_id).await {
                                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::skill_data_from_records(&skills))).await?;
                                            }
                                        }
                                    }
                                    10 => {
                                        // GP_NPCSEV_EMBED (Fusão de Pedra de Alma)
                                        if gamedata.data.len() >= 14 {
                                            let stone_id = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} fundiu pedra de alma ID {} no equipamento", role_id, stone_id);
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::embed_item(0, stone_id))).await?;
                                        }
                                    }
                                    11 => {
                                        // GP_NPCSEV_CLEAR_TESSERA (Limpeza de Pedras de Alma)
                                        info!("Jogador ID {} limpou pedras de alma do equipamento", role_id);
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::clear_tessera(0))).await?;
                                    }
                                    12 => {
                                        // GP_NPCSEV_MAKEITEM (Forjar Item / Produção)
                                        if gamedata.data.len() >= 14 {
                                            let recipe_id = i32::from_le_bytes([gamedata.data[10], gamedata.data[11], gamedata.data[12], gamedata.data[13]]);
                                            info!("Jogador ID {} forjou item usando a receita ID {}", role_id, recipe_id);
                                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::produce_start(recipe_id, 2000))).await?;
                                            let tx_c = tx.clone();
                                            tokio::spawn(async move {
                                                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                                                let _ = tx_c.send(OutboundPacket::GamedataSend(S2CGamedataSend::produce_once(recipe_id))).await;
                                                let _ = tx_c.send(OutboundPacket::GamedataSend(S2CGamedataSend::produce_end())).await;
                                            });
                                        }
                                    }
                                    13 => {
                                        // GP_NPCSEV_BREAKITEM (Decomposição de Item em Pedras Celestiais)
                                        info!("Jogador ID {} decompôs equipamento", role_id);
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::decompose_start(1))).await?;
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::decompose_end())).await?;
                                    }
                                    15 => {
                                        // GP_NPCSEV_OPENTRASH (Abrir Armazém / Banqueiro)
                                        info!("Jogador ID {} abriu o Armazém/Banqueiro", role_id);
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::trashbox_open(32))).await?;
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::trashbox_wealth(0))).await?;
                                    }
                                    _ => {
                                        debug!("SEVNPC_SERVE tipo {} recebido de jogador {}", service_type, role_id);
                                    }
                                }
                            }
                        }
                        21 => {
                            // C2S 21: SELF_GET_PROPERTY (Consulta de atributos e saldo da conta)
                            debug!("Jogador ID {} solicitou SELF_GET_PROPERTY (C2S 21)", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(50000, 0))).await?;
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                                1, session.sec_level, 120, 120, 280, 280, 0, 0
                            ))).await?;
                        }
                        39 => {
                            // C2S::GET_ALL_DATA (Comando 39) - Solicitação de inventário, equipamentos, saldo e missões
                            let items = self.char_repo.item_repo().list_by_container(role_id, pw_core::ContainerType::Inventory).await.unwrap_or_default();
                            let equips = self.char_repo.item_repo().list_by_container(role_id, pw_core::ContainerType::Equipment).await.unwrap_or_default();
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(0, 32, &items))).await?;
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(1, 32, &equips))).await?;
                            for item in &items {
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(0, item.slot as u8, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets))).await?;
                            }
                            for item in &equips {
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(1, item.slot as u8, item.item_id as i32, item.durability as i32 * 100, item.max_durability as i32 * 100, item.count, &item.octets))).await?;
                            }
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(50000, 0))).await?;
                            // S2C::TASK_DATA (cmd 105) é o marcador oficial de término do GET_ALL_DATA que dispara o LoadConfigData no cliente (EC_HostMsg.cpp:3841)
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_data())).await?;
                        }
                        49 => {
                            // C2S 49: TASK_NOTIFY (Notificações e verificação de missões do cliente)
                            trace!("TASK_NOTIFY recebido de jogador {}", role_id);
                            let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;
                            if gamedata.data.len() >= 7 && gamedata.data[6] == 7 {
                                let mut dyn_mark = OctetsStream::new();
                                dyn_mark.write_u8(7);       // reason = TASK_SVR_NOTIFY_DYN_TIME_MARK (7)
                                dyn_mark.write_u16_le(0);   // task = 0
                                dyn_mark.write_u32_le(0);   // time_mark = 0
                                dyn_mark.write_u32_le(1);   // version = 1
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_var_data(&dyn_mark.into_bytes()))).await?;
                            } else {
                                let role_quests = self.char_repo.quest_repo().list_quests(role_id).await.unwrap_or_default();
                                for q in role_quests {
                                    if q.status == pw_core::QuestStatus::Active {
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_notify_new(q.quest_id as u16, now_ts))).await?;
                                    }
                                }
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::task_data())).await?;
                            }
                        }
                        40 => {
                            // C2S::USE_ITEM (Usar item do inventário persistente no banco)
                            if gamedata.data.len() >= 10 {
                                let where_pack = gamedata.data[2];
                                let by_count = gamedata.data[3];
                                let slot = u16::from_le_bytes([gamedata.data[4], gamedata.data[5]]) as u8;
                                let item_id = i32::from_le_bytes([gamedata.data[6], gamedata.data[7], gamedata.data[8], gamedata.data[9]]);
                                info!("Jogador ID {} usou item ID {} do pacote {} slot {} (qtd: {})", role_id, item_id, where_pack, slot, by_count);
                                let ctype = pw_core::ContainerType::from_i16(where_pack as i16);
                                let _ = self.char_repo.item_repo().consume_item(role_id, ctype, slot as u16, by_count as u32).await;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::host_use_item(where_pack, slot, item_id, by_count as u16))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(where_pack, slot as u16))).await?;

                                // Se for poção de HP (1796) ou MP (1801), atualiza os status vitais
                                if item_id == 1796 || item_id == 1801 {
                                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                                        1, session.sec_level, 120, 120, 280, 280, 0, 0
                                    ))).await?;
                                }
                            }
                        }
                        41 | 80 => {
                            // C2S 41 / 80: CAST_SKILL / CAST_INSTANT_SKILL
                            if gamedata.data.len() >= 6 {
                                let skill_id = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                let target_id = if gamedata.data.len() >= 11 {
                                    i32::from_le_bytes([gamedata.data[7], gamedata.data[8], gamedata.data[9], gamedata.data[10]])
                                } else if let Some(t) = session.target_id {
                                    t
                                } else {
                                    role_id
                                };
                                info!("Jogador ID {} conjurou habilidade ID {} no alvo {} (iniciando canalização)", role_id, skill_id, target_id);
                                
                                // Envia OBJECT_CAST_SKILL com 1000ms de canalização
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_cast_skill(
                                    role_id,
                                    target_id,
                                    skill_id,
                                    1000,   // cast time in ms = 1000ms
                                    1,      // skill level
                                ))).await?;

                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                    
                                    // 1. Libera o jogador do estado de conjuração (SKILL_PERFORM = 88)
                                    let _ = tx_clone.send(OutboundPacket::GamedataSend(S2CGamedataSend::skill_perform())).await;

                                    // 2. Aplica o resultado do ataque com a habilidade (SELF_SKILL_ATTACK_RESULT = 142)
                                    let _ = tx_clone.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_skill_attack_result(
                                        target_id,
                                        skill_id,
                                        150,
                                        0,
                                        0,
                                    ))).await;

                                    // 3. Informa a terceiros (OBJECT_SKILL_ATTACK_RESULT = 143)
                                    let _ = tx_clone.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_skill_attack_result(
                                        role_id,
                                        target_id,
                                        skill_id,
                                        150,
                                        0,
                                        0,
                                    ))).await;

                                    // 4. Finaliza o feitiço no cliente (SELF_STOP_SKILL = 123)
                                    let _ = tx_clone.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_stop_skill())).await;

                                    // 5. Se for monstro/entidade, atualiza o HP no HUD
                                    if (target_id as u32 & 0x80000000) != 0 {
                                        let _ = tx_clone.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_info_00(
                                            target_id,
                                            850,
                                            1000,
                                        ))).await;
                                    }
                                });
                            }
                        }
                        42 => {
                            // C2S::CANCEL_ACTION (Cancelar ação / Parar / Levantar)
                            info!("Jogador ID {} cancelou ação / levantou", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_stand_up(role_id))).await?;
                        }
                        46 => {
                            // C2S::SIT_DOWN (Meditar / Sentar)
                            info!("Jogador ID {} sentou / iniciou meditação", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_sit_down(role_id))).await?;
                        }
                        47 => {
                            // C2S::STAND_UP (Levantar)
                            info!("Jogador ID {} levantou da meditação", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_stand_up(role_id))).await?;
                        }
                        48 => {
                            // C2S::EMOTE_ACTION (Ações / Emotes)
                            if gamedata.data.len() >= 4 {
                                let emotion = u16::from_le_bytes([gamedata.data[2], gamedata.data[3]]);
                                info!("Jogador ID {} executou emote {}", role_id, emotion);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::object_do_emote(role_id, emotion))).await?;
                            }
                        }
                        67 => {
                            // C2S 67: QUERY_PLAYER_INFO_1
                            if gamedata.data.len() >= 4 {
                                let count = u16::from_le_bytes([gamedata.data[2], gamedata.data[3]]) as usize;
                                trace!("QUERY_PLAYER_INFO_1 recebido com {} players", count);
                            }
                        }
                        68 => {
                            // C2S 68: QUERY_NPC_INFO_1 (Consulta periódica de HP e propriedades de monstros/NPCs)
                            if gamedata.data.len() >= 4 {
                                let count = u16::from_le_bytes([gamedata.data[2], gamedata.data[3]]) as usize;
                                let mut offset = 4;
                                for _ in 0..count {
                                    if offset + 4 <= gamedata.data.len() {
                                        let nid = i32::from_le_bytes([
                                            gamedata.data[offset],
                                            gamedata.data[offset + 1],
                                            gamedata.data[offset + 2],
                                            gamedata.data[offset + 3],
                                        ]);
                                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_info_00(
                                            nid, 1000, 1000
                                        ))).await?;
                                        offset += 4;
                                    }
                                }
                            }
                        }
                        75 => {
                            // C2S::ENTER_SANCTUARY (Zona Segura)
                            debug!("Jogador ID {} entrou em santuário / zona segura", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::enter_sanctuary())).await?;
                        }
                        76 => {
                            // C2S::LEAVE_SANCTUARY (Sair da Zona Segura)
                            debug!("Jogador ID {} saiu do santuário", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::leave_sanctuary())).await?;
                        }
                        106 | 110 => {
                            // C2S 106 / 110: Consulta de saldo de Cash da Loja
                            debug!("Jogador ID {} consultou saldo de Cash", role_id);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(50000, 0))).await?;
                        }
                        118 => {
                            // C2S 118: GET_MALL_ITEM_PRICE
                            debug!("Cliente solicitou tabela de preços do Mall (C2S 118)");
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::mall_item_price())).await?;
                        }
                        107 | 120 => {
                            // C2S 107 / 120: MALL_SHOPPING (Comprar item na Loja Gold/Cash)
                            if gamedata.data.len() >= 10 {
                                let item_id = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                let count = if gamedata.data.len() >= 8 { u16::from_le_bytes([gamedata.data[6], gamedata.data[7]]) } else { 1 };
                                info!("Jogador ID {} comprou item ID {} (qtd: {}) na Loja Gold", role_id, item_id, count);
                                let _ = self.char_repo.item_repo().upsert_item(&pw_core::ItemRecord {
                                    id: None,
                                    character_id: role_id,
                                    container_type: pw_core::ContainerType::Inventory,
                                    slot: 12,
                                    item_id: item_id as u32,
                                    count: count as u32,
                                    max_count: 100,
                                    refine_level: 0,
                                    sockets_count: 0,
                                    sockets: vec![],
                                    durability: 10000,
                                    max_durability: 10000,
                                    bind_status: 0,
                                    octets: vec![],
                                    custom_attributes: serde_json::json!({}),
                                }).await;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(0, 12, item_id, 10000, 10000, count as u32, &[]))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::unfreeze_ivtr_slot(0, 12))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(49000, 0))).await?;
                            }
                        }
                        85 | 192 => {
                            // C2S 85 / 192: Alternar entre Armadura e Visual de Moda (Fashion Mode)
                            let enable = if gamedata.data.len() >= 3 { gamedata.data[2] == 1 } else { true };
                            info!("Jogador ID {} alternou modo de moda para: {}", role_id, enable);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_enable_fashion(enable))).await?;
                        }
                        214..=220 => {
                            // Subcomandos de Duelo (Duel Prepare, Start, Result)
                            if gamedata.data.len() >= 6 {
                                let opponent_id = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                info!("Duelo entre jogador {} e oponente {}", role_id, opponent_id);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::duel_prepare(role_id, opponent_id))).await?;
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::host_duel_start(opponent_id))).await?;
                            }
                        }
                        _ => {
                            debug!("Gamedata subcomando {} recebido do cliente", cmd);
                        }
                    }
                }
            }

            InboundPacket::GetUIConfig(req) => {
                tx.send(OutboundPacket::GetUIConfigRe(S2CGetUIConfigRe::new(
                    req.role_id,
                    req.localsid,
                    &[],
                ))).await?;
            }

            InboundPacket::SetUIConfig(req) => {
                debug!("Salvando UIConfig ({} bytes) para o personagem ID {}", req.ui_config.len(), req.role_id);
                tx.send(OutboundPacket::SetUIConfigRe(S2CSetUIConfigRe {
                    result: 0,
                    role_id: req.role_id,
                })).await?;
            }

            InboundPacket::SetCustomData(req) => {
                debug!("Salvando CustomData ({} bytes) para o personagem ID {}", req.data.len(), req.role_id);
                tx.send(OutboundPacket::SetCustomDataRe(S2CSetCustomDataRe {
                    result: 0,
                    role_id: req.role_id,
                })).await?;
            }

            InboundPacket::GetFriendList(req) => {
                tx.send(OutboundPacket::GetFriendListRe(S2CGetFriendListRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::GetWaitDelRoles(req) => {
                tx.send(OutboundPacket::GetWaitDelRolesRe(S2CGetWaitDelRolesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::GetHelpStates(req) => {
                tx.send(OutboundPacket::GetHelpStatesRe(S2CGetHelpStatesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                    help_states: vec![0u8; 32],
                })).await?;
            }

            InboundPacket::SetHelpStates(req) => {
                debug!("Salvando HelpStates ({} bytes) para o personagem ID {}", req.help_states.len(), req.role_id);
                tx.send(OutboundPacket::SetHelpStatesRe(S2CSetHelpStatesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::QueryServerTime(_) => {
                debug!("Pacote 0x352 (BattleGetMap) recebido na Sessão #{}", session.session_id);
            }

            InboundPacket::PlayerMove(move_pkt) => {
                let role_id = session.role_id.unwrap_or(0);
                let _ = self.char_repo.update_position(role_id, &move_pkt.position).await;

                let move_broadcast = OutboundPacket::PlayerMoveBroadcast(S2CPlayerMoveBroadcast {
                    role_id,
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

                tx.send(move_broadcast).await?;
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

                tx.send(broadcast_pkt).await?;
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
