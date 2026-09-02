use futures::{SinkExt, StreamExt};
use pw_core::{CharacterClass, CharacterSummary, Vector3};
use pw_crypto::generate_login_challenge;
use pw_protocol::{
    create_protocol_adapter, GameVersion, InboundPacket, OctetsStream, OutboundPacket, ProtocolAdapter,
    PwPacketCodec, S2CChatBroadcast, S2CChallenge, S2CCreateRoleResponse, S2CDeleteRoleResponse,
    S2CErrorInfo, S2CGamedataSend, S2CGetFriendListRe, S2CGetHelpStatesRe, S2CGetUIConfigRe, S2CGetWaitDelRolesRe,
    S2COnlineAnnounce, S2CPlayerMoveBroadcast, S2CRoleListResponse, S2CSelectRoleResponse,
    S2CSetCustomDataRe, S2CSetHelpStatesRe, S2CSetUIConfigRe, S2CUndoDeleteRoleResponse,
};
use pw_data_loader::GameDataManager;
use pw_protocol::{Edition, VersaoDoCliente};
use pw_storage::{AccountRepository, CacheManager, CharacterRepository};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, info, trace, warn};

use crate::session::ClientSession;
use crate::uplink::BusUplink;
use pw_bus::BusMessage;

pub struct LinkGateway {
    pub realm_id: String,
    pub game_version: GameVersion,
    pub adapter: Arc<dyn ProtocolAdapter>,
    pub listen_port: u16,
    pub account_repo: AccountRepository,
    pub char_repo: CharacterRepository,
    pub cache_manager: CacheManager,
    pub data_manager: Arc<GameDataManager>,
    /// As constantes de compilação do **cliente** que este realm serve.
    ///
    /// Vêm do padrão da versão e podem ser sobrescritas por realm no ambiente
    /// (`ELEMENTDATA_VERSION` e `TASK_TEMPL_VERSION`) — ver [`pw_protocol::edition`]:
    /// dois realms "1.5.3" podem servir builds diferentes do cliente, e cada build tem o
    /// seu par.
    pub versao_do_cliente: VersaoDoCliente,
    /// A ligação com o servidor de mundo deste realm, quando há uma.
    ///
    /// `None` faz o link rodar sozinho, exatamente como antes desta fase. É o que
    /// mantém o desenvolvimento e os testes possíveis sem subir o `pw-gs` junto — e o
    /// que garante que ligar o barramento não muda o que o jogador vê enquanto os
    /// subcomandos ainda são tratados aqui.
    pub uplink: Option<Arc<BusUplink>>,
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
        // Um `GAME_VERSION` que não parseia é erro de configuração, não motivo para
        // adivinhar. Antes isto caía em 1.2.6 em silêncio: um realm 1.5.3 com um erro de
        // digitação subia falando o protocolo errado, e o sintoma aparecia lá adiante
        // como "o cliente conecta e recusa o login", sem nada no log apontando a causa.
        // Com vários realms no mesmo `docker-compose`, isso vira questão de tempo.
        let game_version = version_str.parse::<GameVersion>().unwrap_or_else(|_| {
            panic!(
                "GAME_VERSION inválido para o realm '{realm_id}': {version_str:?}. \
                 Valores aceitos: 1.2.6, 1.4.8, 1.5.3."
            )
        });
        let adapter = create_protocol_adapter(game_version);

        let mut data_manager = GameDataManager::new();
        let possible_dirs = [
            format!("data/{}/config", realm_id),
            "data/config".to_string(),
            "/app/data/config".to_string(),
            format!("/app/data/{}/config", realm_id),
            "config".to_string(),
        ];
        // A primeira pasta que **existe** é a pasta deste realm, dê certo ou não a carga.
        //
        // Antes, um erro em qualquer arquivo fazia o laço seguir para o próximo caminho da
        // lista — que não existe — e terminar com "nenhum diretório encontrado", escondendo
        // que a pasta certa estava lá e um arquivo dentro dela é que estava ruim. Hoje a
        // carga não aborta mais no primeiro erro (ver `RelatorioDeCarga`), então o que sobra
        // é relatar arquivo por arquivo.
        let mut loaded = false;
        for dir_str in &possible_dirs {
            let p = std::path::Path::new(dir_str);
            if p.exists() {
                info!("Carregando arquivos de configuração e mapas de {:?}", p);
                let relatorio = data_manager.load_from_directory(p);
                for falha in &relatorio.falhas {
                    warn!(
                        arquivo = %falha.arquivo,
                        motivo = %falha.motivo,
                        "pw-link: arquivo de dados não carregado"
                    );
                }
                loaded = true;
                break;
            }
        }
        if !loaded {
            warn!("Nenhum diretório de configuração .data foi encontrado nas buscas: {:?}", possible_dirs);
        }

        // As duas constantes do `edition` saem dos `.data` deste realm — o cliente não
        // carrega dados de outra versão, então o número dentro do arquivo é o número dele.
        // O ambiente ainda sobrescreve; um valor ilegível ali é erro de configuração, como
        // o `GAME_VERSION`: seguir com o padrão produziria um `edition` que o cliente
        // recusa, e a mensagem que ele mostra não fala em variável de ambiente nenhuma.
        let versao_do_cliente = VersaoDoCliente::resolver(
            game_version,
            data_manager.versao_do_elements,
            data_manager.versao_das_tasks,
            |k| std::env::var(k).ok(),
        )
        .unwrap_or_else(|e| panic!("Realm '{realm_id}': {e}"));
        info!(
            "Realm {}: constantes do cliente — ELEMENTDATA_VERSION={:#x}, task_templ={} \
             (elements.data={:?}, tasks.data={:?})",
            realm_id,
            versao_do_cliente.elements_data,
            versao_do_cliente.task_templ,
            data_manager.versao_do_elements.map(|v| format!("{v:#x}")),
            data_manager.versao_das_tasks,
        );

        Self {
            realm_id,
            game_version,
            adapter,
            listen_port,
            account_repo,
            char_repo,
            cache_manager,
            data_manager: Arc::new(data_manager),
            versao_do_cliente,
            uplink: None,
        }
    }

    /// Liga este daemon de link ao servidor de mundo em `endereco` (`host:porta`).
    ///
    /// A conexão é feita em segundo plano e reconecta sozinha, então chamar isto não
    /// exige que o `pw-gs` já esteja no ar.
    pub fn com_barramento(mut self, endereco: &str) -> Self {
        info!("pw-link: barramento apontado para o servidor de mundo em {endereco}");
        self.uplink = Some(BusUplink::iniciar(endereco.to_string()));
        self
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
        // Os dois timestamps saem dos `.data` do realm — `gshop.data` e `gshop1.data`,
        // arquivos diferentes. Se algum não estiver presente, o timestamp fica zero e o
        // cliente vai recusar o login; o aviso abaixo diz exatamente isso, porque a
        // mensagem que o cliente mostra é genérica ("versão errada") e não ajuda.
        let gshop3_timestamp = self
            .game_version
            .challenge_edition_tem_terceiro_gshop()
            .then_some(self.data_manager.gshop3.timestamp);
        let edition = Edition::com_versao_do_cliente(
            self.versao_do_cliente,
            self.data_manager.gshop.timestamp,
            self.data_manager.gshop2.timestamp,
            gshop3_timestamp,
        );
        if self.game_version.challenge_has_edition()
            && (edition.gshop_timestamp == 0 || edition.gshop2_timestamp == 0)
        {
            warn!(
                "Realm {}: timestamps de gshop = {} e {} — um zerado faz o cliente \
                 recusar o login no Challenge, com mensagem de versão errada. A pasta de \
                 configuração precisa do par `gshop.data`+`gshop1.data` (empacotamento do \
                 cliente) ou `gshopsev.data`+`gshopsev1.data` (do servidor).",
                self.realm_id, edition.gshop_timestamp, edition.gshop2_timestamp
            );
        }
        if gshop3_timestamp == Some(0) {
            warn!(
                "Realm {}: esta versão espera um terceiro timestamp de gshop \
                 (`gshop2.data`/`gshopsev2.data`) e ele está zerado — o `edition` vai sair \
                 errado e o cliente recusa o login.",
                self.realm_id
            );
        }

        let challenge_packet = OutboundPacket::Challenge(S2CChallenge::new(
            server_nonce.to_vec(),
            self.game_version,
            edition,
        ));

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

        // A sessão acabou — por logout, por queda ou por erro, dá no mesmo para o mundo:
        // aquele jogador não é mais alcançável por este link. Avisar aqui, e não só no
        // caminho de logout limpo, é o que impede um personagem de ficar "preso" no
        // mundo depois de uma queda de conexão.
        if let (Some(uplink), Some(roleid)) = (self.uplink.as_ref(), session.role_id) {
            uplink.enviar(BusMessage::PlayerLogout {
                result: 0,
                roleid,
                provider_link_id: 0,
                localsid: session.localsid,
            });
            uplink.desregistrar(roleid).await;
        }

        info!("Sessão #{} ({}) finalizada.", session_id, client_ip);
        Ok(())
    }

    /// Este pacote só faz sentido depois do login?
    ///
    /// A lista é por inclusão — um pacote novo é tratado como **exigindo** login até
    /// alguém dizer o contrário. O contrário (lista de bloqueados) deixaria cada pacote
    /// novo aberto por omissão, que é o tipo de falha que ninguém percebe ao adicionar
    /// um comando.
    fn exige_autenticacao(packet: &InboundPacket) -> bool {
        !matches!(
            packet,
            InboundPacket::Response(_)          // é o próprio login
                | InboundPacket::KeyExchange(_) // negociação de cifra, antes do login
                | InboundPacket::Heartbeat(_)
                | InboundPacket::QueryServerTime(_)
                | InboundPacket::Unknown { .. } // já é registrado e descartado
        )
    }

    async fn dispatch_packet(
        &self,
        tx: &tokio::sync::mpsc::Sender<OutboundPacket>,
        session: &mut ClientSession,
        packet: InboundPacket,
    ) -> anyhow::Result<()> {
        // Barreira única: sem conta na sessão, nada que toque dados de personagem passa.
        //
        // Cada tratador confere conta e realm por conta própria, mas todos partiam de
        // `session.account_id.unwrap_or(0)` — o que faria a checagem depender de nunca
        // existir uma conta de id 0. Isso é uma suposição sobre a sequência do banco, e
        // não uma garantia. Aqui a suposição deixa de ser necessária.
        if Self::exige_autenticacao(&packet) && session.account_id.is_none() {
            warn!(
                "Sessão #{} mandou um pacote que exige login antes de autenticar — ignorado",
                session.session_id
            );
            return Ok(());
        }

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
                        let details_opt = self.char_repo
                            .get_details(new_role_id, acc_id, &self.realm_id)
                            .await
                            .unwrap_or(None);
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

                // O `role_id` veio do cliente: o repositório só apaga se ele for desta
                // conta **neste** realm. Enquanto essa checagem não existia, qualquer
                // jogador autenticado apagava o personagem de qualquer outro, bastando
                // adivinhar o número — e ele é sequencial.
                let apagado = self
                    .char_repo
                    .delete_character(delete_role.role_id, acc_id, &self.realm_id)
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Falha ao excluir personagem ID {}: {:?}", delete_role.role_id, e);
                        false
                    });

                if !apagado {
                    // Recusa sem dizer por quê: "não é seu" e "não existe" têm que ser a
                    // mesma resposta, senão ela vira um oráculo de quais ids existem.
                    warn!(
                        "Exclusão recusada: personagem {} não é da conta {} no realm '{}'",
                        delete_role.role_id, acc_id, self.realm_id
                    );
                }

                // 1. Envia confirmação DeleteRole_Re (Opcode 0x57)
                tx.send(OutboundPacket::DeleteRoleResponse(S2CDeleteRoleResponse {
                    // TODO: o código de erro não foi conferido contra o C++ original; o
                    // que importa aqui é não responder sucesso a uma operação recusada.
                    result: if apagado { 0 } else { 1 },
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

                let restaurado = self
                    .char_repo
                    .restore_character(undo_delete.role_id, acc_id, &self.realm_id)
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Falha ao restaurar personagem ID {}: {:?}", undo_delete.role_id, e);
                        false
                    });

                if !restaurado {
                    warn!(
                        "Restauração recusada: personagem {} não é da conta {} no realm '{}'",
                        undo_delete.role_id, acc_id, self.realm_id
                    );
                }

                // 1. Envia confirmação UndoDeleteRole_Re (Opcode 0x59)
                tx.send(OutboundPacket::UndoDeleteRoleResponse(S2CUndoDeleteRoleResponse {
                    result: if restaurado { 0 } else { 1 },
                    role_id: undo_delete.role_id,
                    localsid: undo_delete.localsid,
                })).await?;
            }

            InboundPacket::SelectRole(select_role) => {
                info!(
                    "Personagem ID {} selecionado para entrar no mundo (Realm: '{}')",
                    select_role.role_id, self.realm_id
                );

                // Sem a conta e o realm na consulta, este era o caminho para entrar no
                // mundo como **qualquer** personagem do servidor: basta mandar outro
                // `role_id`, que é sequencial.
                let acc_id = session.account_id.unwrap_or(0);
                let char_details_opt = self
                    .char_repo
                    .get_details(select_role.role_id, acc_id, &self.realm_id)
                    .await?;

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

                let acc_id = session.account_id.unwrap_or(0);
                let char_details_opt = self
                    .char_repo
                    .get_details(enter_world.role_id, acc_id, &self.realm_id)
                    .await?;
                if let Some(details) = char_details_opt {
                    info!("Personagem '{}' (ID: {}) entrando no mundo 3D...", details.name, details.id);

                    // Anuncia o jogador ao servidor de mundo antes de qualquer coisa: a
                    // partir daqui ele pode receber do mundo, e o registro precisa
                    // existir quando a primeira resposta voltar.
                    //
                    // Os campos do `EnterWorld` do barramento são os mesmos que o cliente
                    // mandou — é o mesmo protocolo GNET (opcode 72), repassado.
                    session.localsid = enter_world.localsid;
                    if let Some(uplink) = self.uplink.as_ref() {
                        uplink.registrar(details.id, tx.clone()).await;
                        uplink.enviar(BusMessage::EnterWorld {
                            roleid: details.id,
                            provider_link_id: enter_world.provider_link_id,
                            locktime: enter_world.locktime,
                            timeout: enter_world.timeout,
                            settime: enter_world.settime,
                            localsid: enter_world.localsid,
                        });
                    }

                    // 1. INST_DATA_CHECKOUT (206) — os carimbos de tempo dos dados do
                    //    servidor. **O layout depende da versão**: o 1.2.6 tem quatro
                    //    campos e o 1.5.3 tem cinco (item 56), e mandar o tamanho errado
                    //    faz o cliente descartar o comando sem avisar (item 46).
                    let sub = pw_protocol::PorVersao::new(self.game_version);
                    let gshop3 = self
                        .game_version
                        .challenge_edition_tem_terceiro_gshop()
                        .then_some(self.data_manager.gshop3.timestamp);
                    tx.send(OutboundPacket::GamedataSend(sub.inst_data_checkout(
                        1, 2097199, 2097199, 1206433535, gshop3
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

                    // 10.5 Saldo inicial. Era `50000` escrito no código — o mesmo saldo
                    //      para todo personagem de todo realm. O valor de verdade já
                    //      estava carregado aqui, em `details.money`, e nunca era lido.
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(
                        details.money.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                    ))).await?;

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

                // Repassa ao servidor de mundo. Hoje o `pw-gs` apenas registra o que
                // chega — o tratamento continua logo abaixo, neste arquivo — mas é por
                // esta linha que cada comando vai migrar: quando um passar a ser tratado
                // no `pw-gs`, o braço correspondente sai daqui, e nada mais muda.
                //
                // O `data` vai como veio, sem interpretação: o envelope do barramento é
                // GNET, e o conteúdo é o formato do mundo 3D.
                if let (Some(uplink), Some(roleid)) = (self.uplink.as_ref(), session.role_id) {
                    uplink.enviar(BusMessage::ClientToGame {
                        roleid,
                        localsid: session.localsid,
                        data: gamedata.data.clone(),
                    });
                }

                if gamedata.data.len() >= 2 {
                    let cmd = u16::from_le_bytes([gamedata.data[0], gamedata.data[1]]);
                    let role_id = session.role_id.unwrap_or(0);
                    match cmd {
                        // MIGRADOS PARA O `pw-gs` (`bus_server::tratar_subcomando`):
                        //
                        //   0  PLAYER_MOVE  — agora atualiza o mundo em memória, e o
                        //                     autosave grava. Antes era um `UPDATE` no
                        //                     PostgreSQL **por pacote de movimento**.
                        //   1  LOGOUT       — o mundo tira o jogador da simulação e
                        //                     devolve um `PlayerLogout` (69) pelo
                        //                     barramento; o `uplink.rs` traduz aquilo no
                        //                     pacote que o cliente espera.
                        //   2  SELECT_TARGET— e agora com o HP **real** do alvo: aqui o
                        //                     link mandava 1000/1000 fixo, porque não
                        //                     sabe o estado das criaturas.
                        //   3  NORMAL_ATTACK— dano do `CombatEngine` com os atributos dos
                        //                     dois lados, HP debitado de verdade, monstro
                        //                     que morre. Aqui era dano 35 fixo, HP
                        //                     965/1000 fixo, e abate de missão notificado
                        //                     a cada golpe com a criatura `13641` fixa.
                        //   7  STOP_MOVE    — atualiza mundo e grade, sem `UPDATE` por
                        //                     parada.
                        //   8  UNSELECT     — desmarca no mundo, que é quem guarda o alvo
                        //                     desde que o comando 2 migrou.
                        //   4  REVIVE_VILLAGE — não existia; quem zerava a vida ficava
                        //                     preso até reconectar.
                        //   9, 11, 12, 13, 16, 17, 18  — itens: consulta, troca de slot,
                        //                     mover e equipar. Todos passam pelo mesmo
                        //                     repositório, agora transacionado e sem
                        //                     apagar os octetos do item.
                        //   42, 46, 47, 48, 75 — postura, emote e zona segura.
                        //   37  SEVNPC_SERVE — os treze serviços de NPC. A compra e a
                        //                     venda estavam **invertidas**: os nomes do
                        //                     enum são do ponto de vista do NPC.
                        //   40  USE_ITEM     — poção cura pelo `elements.data`, e não
                        //                     por dois ids escritos no código com
                        //                     HP/MP 120/280 fixos.
                        //   41, 80 CAST_SKILL — dano do `CombatEngine` no alvo lido do
                        //                     deslocamento certo; era 150 fixo e o alvo
                        //                     saía de `data[7..11]`, que pega o
                        //                     `target_count` junto.
                        //   27, 28, 29, 30 — grupo, agora com **estado**. Aqui o convite
                        //                     era mandado de volta a quem convidou, a
                        //                     lista de membros vinha com vida e posição
                        //                     escritas no código, e sair era um eco só
                        //                     para o próprio jogador.
                        //
                        // Sem `GS_BUS` configurado, estes dois deixam de ter tratamento —
                        // é o preço declarado da separação, e o `main.rs` avisa no log ao
                        // subir sem barramento.
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
                        35 => {
                            // C2S 35: SEVNPC_HELLO (abrir diálogo com NPC).
                            //
                            // Era `32 | 35`. O 32 é `TEAM_MEMBER_POS` no IR — uma consulta
                            // de posição de companheiro de grupo, que passava a receber um
                            // diálogo de NPC como resposta.
                            if gamedata.data.len() >= 6 {
                                let nid = i32::from_le_bytes([gamedata.data[2], gamedata.data[3], gamedata.data[4], gamedata.data[5]]);
                                info!("Jogador ID {} iniciou diálogo com o NPC ID {}", role_id, nid);
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::npc_greeting(nid))).await?;
                            }
                        }
                        // C2S 21 (`GET_EXT_PROP`) e 39 (`GET_ALL_DATA`) migraram para o
                        // `pw-gs`. Os dois respondiam com números escritos no código —
                        // `120/120/280/280` de vida e mana e `50000` de dinheiro, iguais
                        // para qualquer personagem — porque o daemon de link não tem a
                        // simulação de onde tirar os valores de verdade.
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
                        // C2S 67 (`QUERY_PLAYER_INFO_1`) e 68 (`QUERY_NPC_INFO_1`)
                        // migraram para o `pw-gs`. O 67 **não respondia nada** — lia a
                        // contagem, escrevia uma linha de log e devolvia. O 68 respondia
                        // `1000/1000` de vida para qualquer criatura, e como é uma consulta
                        // periódica, redesenhava a barra cheia logo depois de cada golpe.
                        // C2S 76 **não** é `LEAVE_SANCTUARY`: é `OPEN_BOOTH`, abrir uma
                        // barraca de venda pessoal. Não existe `LEAVE_SANCTUARY` na tabela
                        // C2S do IR. O braço foi removido em vez de corrigido porque não
                        // sabemos o que responder a uma barraca — e responder "você saiu da
                        // zona segura" é pior do que não responder.
                        // C2S 110 (`QUERY_CASH_INFO`) migrou para o `pw-gs`: respondia
                        // `50000` escrito no código, porque o daemon de link não tem o
                        // personagem carregado de onde tirar o saldo.
                        118 => {
                            // C2S 118: GET_MALL_ITEM_PRICE
                            debug!("Cliente solicitou tabela de preços do Mall (C2S 118)");
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::mall_item_price())).await?;
                        }
                        // C2S 106 (`MALL_SHOPPING`) foi **removido**, e não migrado.
                        //
                        // O que havia aqui não era uma compra: gravava o item comprado
                        // sempre no **slot 12**, escrito no código, apagando o que
                        // estivesse lá — a mesma classe de perda de item que os itens 38 e
                        // 39 já tinham custado —, com durabilidade 10000 fixa, e depois
                        // mandava `player_cash(49000)`, de modo que qualquer compra
                        // deixava o jogador com exatamente esse saldo, comprasse o que
                        // comprasse.
                        //
                        // Uma compra de verdade precisa de saldo, preço e slot livre, que
                        // são três coisas que só o mundo tem. Enquanto ela não existe,
                        // não responder é melhor do que destruir um item e inventar um
                        // saldo. Está anotado como dívida em `docs/ESTADO_E_RETOMADA.md`.
                        85 => {
                            // C2S 85: SWITCH_FASHION_MODE.
                            //
                            // Era `85 | 192`. O 192 **não existe** na tabela C2S do IR.
                            let enable = if gamedata.data.len() >= 3 { gamedata.data[2] == 1 } else { true };
                            info!("Jogador ID {} alternou modo de moda para: {}", role_id, enable);
                            tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_enable_fashion(role_id, enable))).await?;
                        }
                        92 => {
                            // C2S 92: DUEL_REQUEST — 6 bytes, `target` no deslocamento 2,
                            // que é exatamente o que a leitura abaixo faz.
                            //
                            // Era `214..=220`. Naquela faixa, 214 a 217 **não existem** no
                            // IR e 218 a 220 são comandos de **GM**
                            // (`GM_QUERY_SPEC_ITEM`, `GM_REMOVE_SPEC_ITEM`,
                            // `GM_OPEN_ACTIVITY`) — que este braço engolia.
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
                // Listas vazias por enquanto — o que importa aqui é a forma do pacote
                // estar certa; a lista de amigos ainda não vem do armazenamento.
                tx.send(OutboundPacket::GetFriendListRe(S2CGetFriendListRe {
                    role_id: req.role_id,
                    groups: Vec::new(),
                    friends: Vec::new(),
                    status: Vec::new(),
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

                // O `ChatBroadCast` não carrega o nome do remetente: o cliente o
                // resolve a partir do `srcroleid`. O nome abaixo é só para o log.
                let sender_name = session
                    .character_name
                    .clone()
                    .or_else(|| session.username.clone())
                    .unwrap_or_else(|| "Jogador".to_string());
                debug!("Chat de {sender_name} (canal {})", chat_pkt.channel);

                let broadcast_pkt = OutboundPacket::ChatBroadcast(S2CChatBroadcast {
                    channel: chat_pkt.channel,
                    emotion: chat_pkt.emotion,
                    src_role_id: session.role_id.unwrap_or(0),
                    message: chat_pkt.message,
                    data: chat_pkt.data,
                });

                tx.send(broadcast_pkt).await?;
            }

            InboundPacket::Heartbeat(_hb) => {
                debug!("Heartbeat recebido na Sessão #{}", session.session_id);
            }

            InboundPacket::ACReport(ac) => {
                debug!("Relatório Anti-Cheat ({} bytes) recebido na Sessão #{}", ac.report.len(), session.session_id);
            }

            InboundPacket::Unknown { opcode, payload } => {
                debug!("Pacote bruto recebido (Opcode: 0x{:X}, {} bytes) na Sessão #{}", opcode, payload.len(), session.session_id);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod testes_da_barreira_de_login {
    use super::*;
    use pw_protocol::{
        C2SChallengeResponse, C2SDeleteRole, C2SEnterWorld, C2SGamedataSend, C2SHeartbeat,
        C2SPlayerChat, C2SSelectRole, C2SUndoDeleteRole,
    };

    /// Os pacotes que tocam dados de personagem **têm** que exigir login.
    ///
    /// Esta lista é o resumo do que a falha permitia: sem conta na sessão, cada um destes
    /// chegava ao banco com um `role_id` escolhido pelo cliente.
    #[test]
    fn os_pacotes_de_personagem_exigem_login() {
        let perigosos = [
            InboundPacket::SelectRole(C2SSelectRole { role_id: 1, flag: 0 }),
            InboundPacket::EnterWorld(C2SEnterWorld {
                role_id: 1,
                provider_link_id: 0,
                locktime: 0,
                timeout: 0,
                settime: 0,
                localsid: 0,
            }),
            InboundPacket::DeleteRole(C2SDeleteRole { role_id: 1, localsid: 0 }),
            InboundPacket::UndoDeleteRole(C2SUndoDeleteRole { role_id: 1, localsid: 0 }),
            InboundPacket::GamedataSend(C2SGamedataSend { data: vec![0, 0] }),
        ];
        for p in perigosos {
            assert!(
                LinkGateway::exige_autenticacao(&p),
                "{p:?} passou sem exigir login"
            );
        }
    }

    /// E o punhado que precisa passar antes do login continua passando — senão ninguém
    /// consegue se autenticar.
    #[test]
    fn o_proprio_login_e_o_heartbeat_passam_sem_conta() {
        let login = InboundPacket::Response(C2SChallengeResponse {
            username: "x".into(),
            password_response: Vec::new(),
            use_token: false,
            cli_fingerprint: Vec::new(),
        });
        assert!(!LinkGateway::exige_autenticacao(&login));

        let hb = InboundPacket::Heartbeat(C2SHeartbeat {
            role_id: 0,
            link_id: 0,
            localsid: 0,
        });
        assert!(!LinkGateway::exige_autenticacao(&hb));
    }

    /// A lista é por inclusão: o que for acrescentado ao `InboundPacket` amanhã já nasce
    /// exigindo login, em vez de nascer aberto.
    #[test]
    fn o_padrao_e_exigir_login() {
        let chat = InboundPacket::PlayerChat(C2SPlayerChat {
            channel: 0,
            emotion: 0,
            src_role_id: 0,
            message: String::new(),
            data: Vec::new(),
            src_level: 0,
        });
        assert!(LinkGateway::exige_autenticacao(&chat));
    }
}
