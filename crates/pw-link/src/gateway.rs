use futures::{SinkExt, StreamExt};
use pw_core::{CharacterSummary, Vector3};
use pw_crypto::generate_login_challenge;
use pw_protocol::{
    create_protocol_adapter, GameVersion, InboundPacket, OutboundPacket, ProtocolAdapter,
    PwPacketCodec, S2CChatBroadcast, S2CChallenge, S2CCreateRoleResponse, S2CDeleteRoleResponse,
    S2CErrorInfo, S2CGamedataSend, S2CGetCustomDataRe, S2CGetFriendListRe, S2CGetHelpStatesRe, S2CGetUIConfigRe,
    S2CGetWaitDelRolesRe, S2COnlineAnnounce, S2CPlayerBaseInfoRe, S2CPlayerMoveBroadcast, S2CRoleListResponse,
    S2CSelectRoleResponse, S2CSetCustomDataRe, S2CSetHelpStatesRe, S2CSetUIConfigRe, S2CUndoDeleteRoleResponse,
};
use pw_data_loader::GameDataManager;
use pw_protocol::{Edition, VersaoDoCliente};
use pw_storage::{AccountRepository, CacheManager, CharacterRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_util::codec::Framed;
use tracing::{debug, info, warn};

use crate::session::ClientSession;
use crate::uplink::{BusUplink, EnvioAoCliente};
use pw_bus::BusMessage;

/// Um jogador com sessão aberta neste link: o canal direto por processo para a sessão
/// dele.
///
/// Já foi a base da visibilidade entre jogadores, e guardava posição, direção e nível de
/// GM para montar o `PLAYER_ENTER_WORLD` de cada um. Desde 2026-09-09 a visibilidade é do
/// mundo, com raio e streaming (`BusServer::atualizar_visiveis`), e os três campos foram
/// embora com ela — o mundo tem a posição de verdade, atualizada a cada movimento.
///
/// O que sobrou é o canal da fala, que é global e não depende de distância — ver
/// `LinkGateway::broadcast_para_todos`.

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
    /// Os servidores de mundo por `worldtag`, quando o realm tem mais de um.
    ///
    /// O original roda **um `gs` por mundo ou instância** (`gamed/gs.conf`, uma seção
    /// `[World_*]`/`[Instance_*]` por processo) e o `gdeliveryd` encaminha cada jogador ao
    /// do mundo em que ele está. Aqui é o mesmo: `GS_BUS=1=host:porta,161=host:porta`.
    /// Mundo sem entrada cai em [`Self::uplink`], o padrão.
    pub uplinks_por_mundo: HashMap<i32, Arc<BusUplink>>,
    /// Jogadores online neste link, pra `PLAYER_ENTER_WORLD`/`PLAYER_LEAVE_WORLD`
    /// entre eles.
    ///
    /// Existe aqui, e não no `pw-gs` (que já tem uma grade espacial de verdade,
    /// `WorldInstance::grid`), porque construir isto exigiria ou estender o
    /// `BusMessage::EnterWorld` — que hoje espelha byte a byte o pacote GNET que o
    /// cliente manda, opcode 72, de propósito — ou fazer o mundo carregar o personagem
    /// sozinho sem `account_id` (o `EnterWorld` do barramento não carrega isso). Aqui
    /// o `pw-link` já tem os dados completos do personagem (`details`, carregado com
    /// `account_id` da sessão) no exato momento em que ele entra no mundo, e todas as
    /// sessões do mesmo realm compartilham este processo — então "mandar pra outro
    /// jogador" é só achar o `envio` dele aqui e escrever no canal, sem round-trip
    /// pelo barramento.
    ///
    /// **Limitação sabida, documentada em vez de escondida**: sem grade espacial,
    /// todo jogador vê todos os outros deste link, não só os próximos — e, desde que um
    /// realm pode ter mais de um mundo (`uplinks_por_mundo`), também os de outro mapa.
    /// Não trava nada, mas não escala e não é o que o original faz. E a posição aqui só atualiza quando alguém ENTRA depois; um jogador já
    /// visível não se move na tela de quem já o viu (isso é o `PlayerMoveBroadcast`,
    /// que hoje só ecoa pro remetente — ver `InboundPacket::PlayerMove`). Migrar isto
    /// pra dentro do `pw-gs`, reaproveitando a grade espacial que já existe pra
    /// NPC/monstro, é o passo natural quando a população justificar o custo.
    jogadores_visiveis: RwLock<HashMap<i32, EnvioAoCliente>>,
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
            uplinks_por_mundo: HashMap::new(),
            jogadores_visiveis: RwLock::new(HashMap::new()),
        }
    }

    /// Manda o mesmo pacote pra todo jogador visível deste link (`jogadores_visiveis`) —
    /// mesmo canal direto por processo que `PLAYER_ENTER_WORLD`/`PLAYER_LEAVE_WORLD` já
    /// usam, sem round-trip pelo barramento. `try_send`, porque uma falha na entrega de
    /// um jogador é problema da sessão dele, não motivo pra derrubar quem mandou (nem
    /// quem está mandando pra si mesmo, já que o remetente também está nesta lista).
    ///
    /// Corrige o bug documentado em `docs/ESTADO_E_RETOMADA.md` (itens 16/20): antes,
    /// `PlayerChat`/`PlayerMove` só ecoavam pro `tx` da própria sessão, então ninguém
    /// nunca via a fala ou o movimento de outro jogador.
    ///
    /// Mesma limitação sabida de `jogadores_visiveis`: sem grade espacial, todo mundo
    /// deste link recebe, não só quem está perto — aceitável pro canal de grito/mundo
    /// (global mesmo no jogo real), mas o canal normal/local devia ter alcance quando a
    /// grade existir.
    async fn broadcast_para_todos(&self, pacote: OutboundPacket) {
        let visiveis = self.jogadores_visiveis.read().await;
        for envio in visiveis.values() {
            let _ = envio.try_send(pacote.clone());
        }
    }

    /// Liga este daemon de link ao servidor de mundo em `endereco` (`host:porta`).
    ///
    /// A conexão é feita em segundo plano e reconecta sozinha, então chamar isto não
    /// exige que o `pw-gs` já esteja no ar.
    ///
    /// `endereco` é um `host:porta` só (um mundo, o padrão) ou uma lista
    /// `mundo=host:porta` separada por vírgula. Na lista, o primeiro vira também o padrão
    /// para mundos que não estão nela.
    pub fn com_barramento(mut self, endereco: &str) -> Self {
        for (mundo, alvo) in Self::ler_barramentos(endereco) {
            let uplink = BusUplink::iniciar(alvo.clone());
            match mundo {
                Some(m) => {
                    info!("pw-link: mundo {m} → servidor de mundo em {alvo}");
                    if self.uplink.is_none() {
                        self.uplink = Some(Arc::clone(&uplink));
                    }
                    self.uplinks_por_mundo.insert(m, uplink);
                }
                None => {
                    info!("pw-link: barramento apontado para o servidor de mundo em {alvo}");
                    self.uplink = Some(uplink);
                }
            }
        }
        self
    }

    /// `"host:porta"` → `[(None, "host:porta")]`; `"1=a:1,161=b:2"` →
    /// `[(Some(1), "a:1"), (Some(161), "b:2")]`.
    pub fn ler_barramentos(endereco: &str) -> Vec<(Option<i32>, String)> {
        endereco
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| match p.split_once('=') {
                Some((m, alvo)) => (m.trim().parse().ok(), alvo.trim().to_string()),
                None => (None, p.to_string()),
            })
            .collect()
    }

    /// O servidor de mundo que atende esta sessão: o do mundo do personagem, ou o padrão.
    fn uplink_da_sessao(&self, session: &ClientSession) -> Option<&Arc<BusUplink>> {
        session
            .world_id
            .and_then(|m| self.uplinks_por_mundo.get(&m))
            .or(self.uplink.as_ref())
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
        if let (Some(uplink), Some(roleid)) = (self.uplink_da_sessao(&session), session.role_id) {
            uplink.enviar(BusMessage::PlayerLogout {
                result: 0,
                roleid,
                provider_link_id: 0,
                localsid: session.localsid,
            });
            uplink.desregistrar(roleid).await;
        }

        // Espelho da entrada: some da lista do canal de fala. O `PLAYER_LEAVE_WORLD` (19)
        // que tira o avatar da tela dos outros **não sai daqui** — sai do mundo, que é
        // quem sabe para quem este jogador era visível (`BusServer::tirar_da_vista_de_todos`,
        // no `PlayerLogout` que o `uplink.enviar` acima acabou de mandar).
        if let Some(roleid) = session.role_id {
            self.jogadores_visiveis.write().await.remove(&roleid);
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
                        // A ficha inicial da classe: os quatro atributos do
                        // `ptemplate.conf` e a vida/mana cheias para eles, pela mesma conta
                        // que o mundo usa. Sem o arquivo vai `None`, e o banco cai no padrão
                        // da coluna.
                        self.data_manager
                            .base_das_classes
                            .get(create_role.cls as i32)
                            .map(|b| {
                                b.ficha_inicial(
                                    self.data_manager.classes.get(create_role.cls as i32),
                                )
                            }),
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
                            // Personagem recém-criado nunca entrou no mundo.
                            last_login_at: None,
                            character_mode: Vec::new(),
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
                    session.world_id = Some(details.world_id);
                    if let Some(uplink) = self.uplink_da_sessao(session) {
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
                    //
                    //    `region`/`precinct` vêm de `<mapa>/region.sev`/`precinct.sev`
                    //    (achado em 2026-09-03: valores fixos aqui faziam o cliente
                    //    recusar a instância com "regionset timestamp error" e travar a
                    //    entrada no mundo, mesmo depois do handshake de login passar —
                    //    ver `docs/ESTADO_E_RETOMADA.md`). `id_inst` é o mundo do
                    //    personagem, o mesmo `worldtag` com que o cliente abriu o mapa
                    //    (`StartGame(ri.worldtag, …)`, `EC_LoginUIMan.cpp:1048`). Era `1`
                    //    fixo; com personagem nascendo no mapa 161, o cliente receberia os
                    //    carimbos de outro mapa.
                    let id_inst: i32 = details.world_id;
                    let sub = pw_protocol::versions::create_world_protocol(self.game_version);
                    let region = self
                        .data_manager
                        .region_timestamps
                        .get(&id_inst)
                        .copied()
                        .unwrap_or_else(|| {
                            warn!(
                                "Realm {}: sem region_timestamp pro world_id={id_inst} — \
                                 o cliente vai recusar a instância (\"regionset timestamp error\") e \
                                 travar a entrada no mundo. Falta world/region.sev na pasta do realm.",
                                self.realm_id
                            );
                            0
                        });
                    let precinct = self
                        .data_manager
                        .precinct_timestamps
                        .get(&id_inst)
                        .copied()
                        .unwrap_or_else(|| {
                            warn!(
                                "Realm {}: sem precinct_timestamp pro world_id={id_inst} — \
                                 mesmo problema do region_timestamp acima, com world/precinct.sev.",
                                self.realm_id
                            );
                            0
                        });
                    let gshop3 = self
                        .game_version
                        .challenge_edition_tem_terceiro_gshop()
                        .then_some(self.data_manager.gshop3.timestamp);
                    tx.send(OutboundPacket::GamedataSend(sub.inst_data_checkout(
                        id_inst,
                        region,
                        precinct,
                        self.data_manager.gshop.timestamp,
                        self.data_manager.gshop2.timestamp,
                        gshop3,
                    ))).await?;

                    // 2. Envia SELF_INFO_00 (38) — vitais, nível de **cultivo** (não o de GM:
                    // o `Level2` do comando é o cultivo, `EC_Player.cpp:7447`) e a barra de chi.
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_info_00(
                        details.level as i16,
                        details.cultivation.clamp(0, 255) as u8,
                        // Quem acabou de entrar no mundo não está em combate.
                        false,
                        details.hp,
                        details.hp,
                        details.mp,
                        details.mp,
                        details.exp as i32,
                        details.sp as i32,
                        details.ap,
                        details.max_ap,
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
                    // e cancela o timeout OT_ENTERGAME do client (30s -> "EnterWorld Overtime"
                    // se o tamanho do pacote não bater com o que o client espera).
                    tx.send(OutboundPacket::GamedataSend(sub.self_info_1(
                        details.exp as i32,
                        details.sp as i32,
                        details.id,
                        details.position,
                        session.sec_level,
                        // O modo roupa que o personagem tinha ao sair (B83/B86): é este bit
                        // que faz o cliente desenhar o **dono da tela** de roupa ao entrar.
                        details.modo_roupa,
                    ))).await?;

                    // O `OWN_EXT_PROP` (50) — a ficha do jogador, com os atributos que
                    // `CanUseEquipment` confere — **não sai daqui**. Ele saía, e não
                    // funcionava: mandado antes do `SELF_INFO_1` o dono da tela ainda não
                    // existia e o comando se perdia; mandado depois, os números que o link
                    // tem são zeros, porque quem calcula precisão, evasão, defesa e dano é
                    // o `pw-gs`. Agora ele sai do mundo, junto do `GET_ALL_DATA` — ver
                    // `BusServer::todos_os_dados`.

                    // 6. SKILL_DATA (90), OWN_IVTR_DATA (42) e OWN_ITEM_INFO (40)
                    // **não saem daqui com servidor de mundo ativo** (B65).
                    // Saíam antes do `GET_ALL_DATA` quando o Host do cliente ainda não
                    // estava pronto (`HostIsReady()` falso), fazendo o cliente duplicar
                    // as habilidades na janela (tecla R) e duplicar o inventário na entrada
                    // quando o pw-gs respondia ao `GET_ALL_DATA` com `todos_os_dados`.
                    // O servidor original (`player.cpp:13697-13727`) envia tudo isso
                    // exclusivamente na resposta do `GET_ALL_DATA`.
                    if self.uplink_da_sessao(&session).is_none() {
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::skill_data_from_records(&details.skills))).await?;
                    }

                    // 7. Envia TASK_DATA (Comando 105) e inicializa o subsistema de missões do cliente.
                    // No client, `OnMsgHstTaskData` (EC_HostMsg.cpp) trata esse comando como o
                    // "fim" do GET_ALL_DATA e chama `LoadConfigData()` — que manda o pedido de
                    // GetUIConfig (opcode 104) pro servidor. Logo abaixo. Log aqui pra dar pra
                    // cruzar com o log de `InboundPacket::GetUIConfig` e confirmar se o pedido
                    // do client realmente chega depois disso.
                    // O número de blocos depende da versão (3 no 1.2.6, 5 do 1.5.3 em
                    // diante) — ver `WorldProtocol::task_data`, que traz a desmontagem dos dois
                    // clients reais. Mandar 3 pro 1.5.5 deixava o cliente lendo 8 bytes de
                    // lixo depois do fim do buffer.
                    // As listas de missão **de verdade**, do banco (`character_task_lists`),
                    // que o `pw-gs` mantém e grava a cada mudança. Ia com os blocos vazios, e o
                    // cliente ficava com a lista ativa na versão 0 — que faz o
                    // `OnServerNotify` (`TaskClient.cpp:262`) descartar **todo** aviso de missão:
                    // aceitar no NPC não mostrava nada (teste em jogo de 2026-09-14). Personagem
                    // sem lista gravada recebe as listas vazias com a versão 1.
                    let listas = match self.char_repo.task_lists().carregar(details.id).await {
                        Ok(Some(l)) => [l.ativa, l.concluidas, l.tempos, l.contagens, l.deposito],
                        Ok(None) => listas_de_missao_vazias(),
                        Err(e) => {
                            warn!("não consegui ler as missões de {}: {e}", details.id);
                            listas_de_missao_vazias()
                        }
                    };
                    let [la, lb, lc, ld, le] = &listas;
                    // **Com servidor de mundo, o `TASK_DATA` é só dele** (B53). O cliente pede a
                    // configuração a cada `TASK_DATA`, e este chegava antes das habilidades e
                    // da bolsa que o mundo manda: o pedido que ele provocava era o respondido
                    // (o segundo é recusado — responder duas vezes derruba o cliente), as
                    // barras eram montadas sem nada para apontar e gravadas vazias ao sair
                    // (teste de 2026-09-16: 211 bytes enviados, 172 gravados). No original o
                    // `TASK_DATA` só existe no fim da resposta ao `GET_ALL_DATA`
                    // (`EC_HostMsg.cpp:3945-3949`), que é o que o mundo faz.
                    if self.uplink_da_sessao(&session).is_none() {
                        info!("TASK_DATA enviado pro personagem ID {} ({} missões ativas)", details.id, la.first().copied().unwrap_or(0));
                        tx.send(OutboundPacket::GamedataSend(sub.task_data_com_listas([la, lb, lc, ld, le]))).await?;
                    }
                    // A marca das missões dinâmicas **não** vai aqui. Ia, sem ninguém pedir,
                    // com `version = 0` — e o cliente descarta toda marca cuja versão não é
                    // `DYN_TASK_CUR_VERSION` (10, `TaskTemplMan.cpp:168`). Quem responde é o
                    // mundo, quando o cliente pede (`TASK_NOTIFY` com `reason` 7), como o
                    // original.
                    //
                    // A "missão inicial" que era gravada e anunciada aqui (9374/1/9375 por
                    // raça) saiu: era inventada, e um `TASK_SVR_NOTIFY_NEW` sem a missão na
                    // lista desalinha a cópia do cliente. Missão se pega no NPC.

                    // 8 e 9: OWN_IVTR_DATA (42) e OWN_ITEM_INFO (40)
                    if self.uplink_da_sessao(&session).is_none() {
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(0, 32, &details.inventory))).await?;
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(1, 32, &details.equipment))).await?;
                        let bolsa_de_missao = self
                            .char_repo
                            .item_repo()
                            .list_by_container(details.id, pw_core::ContainerType::TaskInventory)
                            .await
                            .unwrap_or_default();
                        tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::own_ivtr_from_items(2, 32, &bolsa_de_missao))).await?;

                        let equipamentos = &self.data_manager.equipamentos;
                        for (onde, lista) in [(0u8, &details.inventory), (1u8, &details.equipment)] {
                            for item in lista {
                                tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::item_info(
                                    onde,
                                    item.slot as u8,
                                    item.item_id as i32,
                                    item.durability as i32 * 100,
                                    item.max_durability as i32 * 100,
                                    item.count,
                                    &item.octets,
                                    equipamentos.ficha(item.item_id),
                                ))).await?;
                            }
                        }
                    }

                    // Os NPCs e monstros **não saem daqui**.
                    //
                    // Saíam: este passo mandava, uma vez só, o que estava num raio de 120 m
                    // da posição de entrada, com um teto de 60 e uma lista de reserva
                    // escrita no código para quando o `npcgen` não respondesse. Uma vez só
                    // era o problema — o cliente descarta o que sai do raio ativo dele, e
                    // ninguém reenviava. Andando a pé passava despercebido porque aquele
                    // raio cobria a vila inteira; o teleporte de GM mostrou o buraco de uma
                    // vez, com o destino chegando vazio (2026-09-09).
                    //
                    // Agora quem manda é o mundo, que tem a grade espacial, sabe quais
                    // monstros estão vivos e continua mandando conforme o jogador anda —
                    // ver `BusServer::atualizar_visiveis`.
                    // 10.5 Dinheiro (prata/ouro) do personagem — GET_OWN_MONEY (82), não
                    //      PLAYER_CASH (253). Achado em 2026-09-03 lendo `SendAllData`
                    //      (EvolvedPWServer, player.cpp): `player_cash` manda
                    //      `GetMallCash()` (saldo da loja de cash, sistema que este
                    //      servidor ainda não tem) e `get_own_money` manda `GetMoney()` (o
                    //      saldo normal, mostrado na HUD). O código antigo mandava
                    //      `details.money` pelo comando errado (253/cash), deixando o saldo
                    //      normal do jogador sempre em zero no client. `MONEY_CAPACITY_BASE`
                    //      é o teto de 2 bilhões do servidor real (`cgame/gs/config.h`).
                    const MONEY_CAPACITY_BASE: u32 = 2_000_000_000;
                    tx.send(OutboundPacket::GamedataSend(sub.get_own_money(
                        details.money.clamp(0, MONEY_CAPACITY_BASE as i64) as u32,
                        MONEY_CAPACITY_BASE,
                    ))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_cash(0))).await?;

                    // 10.6 Notificações "de status" que o `SendAllData` real manda no
                    // world-entry — achadas em 2026-09-03 lendo o source do 1.5.5 (sem
                    // captura disponível para essa versão). Valores neutros/zerados pra
                    // sistemas que este servidor ainda não implementa (facção, realeza,
                    // exp em dobro, pária) — mandar o comando com zero é o que o `SendAllData`
                    // real também faz pra quem não tem o dado; não mandar nada é o que
                    // fazia o client nunca inicializar esses painéis de UI.
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::host_reputation(details.reputation))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::pvp_mode(0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_country_notify(0))).await?;
                    let agora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i32;
                    // lua_version = 102: primeira linha de `global_api.lua` (ver o
                    // comentário em `S2CGamedataSend::server_time` — um valor errado aqui
                    // derruba o client, não é cosmético).
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::server_time(agora, 0, 102))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::trashbox_pwd_state(false))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::pet_room_capacity(0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::self_king_notify(false, 0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::faction_contrib_notify(0, 0, 0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_leadership(0, 0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_world_contribution(0, 0, 0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::player_dividend(0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::available_double_exp_time(0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::double_exp_time(0, 0))).await?;
                    tx.send(OutboundPacket::GamedataSend(S2CGamedataSend::pariah_time(0))).await?;

                    // 10.7 A visibilidade entre jogadores **não sai mais daqui**.
                    //
                    // Saía: este passo mandava `PLAYER_ENTER_WORLD` (17) mútuo entre todo
                    // mundo do link no momento do login, e o encerramento da sessão mandava
                    // `PLAYER_LEAVE_WORLD` (19). Uma vez só, sem raio e sem streaming —
                    // exatamente o que os NPCs tinham antes do item 39. Dois jogadores que
                    // se afastassem além do raio ativo do cliente sumiam um para o outro
                    // para sempre, porque o cliente descarta o que sai do raio e nada
                    // reenviava (2026-09-09).
                    //
                    // Agora quem manda é o mundo, que tem a grade espacial e recalcula a
                    // cada movimento — ver `BusServer::atualizar_visiveis`. Ele usa
                    // `PLAYER_ENTER_SLICE` (12), não o 17: o cliente escolhe o efeito de
                    // aparição pelo comando (`EC_ManPlayer.cpp:1845`), e quem vem andando
                    // não deve surgir com efeito de teleporte.
                    //
                    // A lista continua existindo para o canal de fala
                    // (`broadcast_para_todos`), que é global e não depende de distância.
                    self.jogadores_visiveis
                        .write()
                        .await
                        .insert(details.id, tx.clone());

                    // GetUIConfig_Re — envio proativo, protegido por `session.ui_config_enviado`.
                    //
                    // Histórico: a teoria original era que o client sempre pede sozinho
                    // (opcode 104) ao processar o TASK_DATA do passo 7 — `LoadConfigData()`,
                    // chamado de `CECHostPlayer::OnMsgHstTaskData` no client — e mandar aqui
                    // TAMBÉM, sem esperar, fazia o client receber dois `GetUIConfig_Re` e
                    // rodar `OnPrtcGetConfigRe` (que ativa `OnAllInitDataReady()` e o hook do
                    // LogicCheck.dll) duas vezes, derrubando o processo. Então tirei o envio
                    // proativo daqui.
                    //
                    // Só que, testando com log em cada ponta (2026-09-03): o `TASK_DATA
                    // enviado` aparece, mas o pedido `GetUIConfig` do client **nunca chega**
                    // no servidor — o client fica preso pra sempre em "Entrando em Perfect
                    // World" (`Win_EnterWait`, que só fecha quando `EnableUI(true)` roda
                    // dentro de `OnPrtcGetConfigRe`, e isso só roda ao receber
                    // `GetUIConfig_Re`). Então o pedido do client não é confiável — mas o
                    // double-send continua sendo um crash real se os dois casos colidirem.
                    //
                    // A flag resolve os dois: manda aqui se ainda não mandou (cobre o caso
                    // — que parece ser o de sempre — do client nunca pedir sozinho), e o
                    // handler de `InboundPacket::GetUIConfig` abaixo checa a mesma flag antes
                    // de responder ao pedido do client, caso ele chegue depois de tudo.
                    // O `GetUIConfig_Re` **não** sai daqui (B52): sai em resposta ao pedido que o
                    // cliente faz depois do `TASK_DATA` do mundo — ver `InboundPacket::GetUIConfig`.
                    // Mandado aqui, chegava antes da bolsa e das habilidades e as barras de
                    // atalho eram montadas vazias (teste em jogo de 2026-09-16).

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
                if let (Some(uplink), Some(roleid)) = (self.uplink_da_sessao(session), session.role_id) {
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
                        //   35  SEVNPC_HELLO, 49 TASK_NOTIFY, 85 SWITCH_FASHION_MODE — tratados
                        //                     no mundo desde 2026-09 e esquecidos aqui até
                        //                     2026-09-14: o cliente recebia **duas**
                        //                     respostas. O 35 abria o diálogo duas vezes
                        //                     (e para qualquer id, NPC ou não). O 85 lia um
                        //                     corpo que o comando não tem e respondia
                        //                     "roupa ligada" a todo clique, antes da
                        //                     resposta certa do mundo. O 49 respondia o
                        //                     pedido da marca das missões dinâmicas com
                        //                     `reason = 7`, que no cliente é
                        //                     `TASK_SVR_NOTIFY_FORGET_SKILL` — a ordem de
                        //                     esquecer a habilidade de produção
                        //                     (`TaskClient.cpp:283`); a marca é o 8.
                        //
                        // Sem `GS_BUS` configurado, estes deixam de ter tratamento — é o
                        // preço declarado da separação, e o `main.rs` avisa no log ao subir
                        // sem barramento.
                        //
                        // C2S 23 a 26 **não** são voo: são `GET_EXT_PROP_BASE`/`_MOVE`/
                        // `_ATK`/`_DEF`, consultas sem corpo. O braço que havia aqui lia um
                        // "tipo de voo" no byte 2, que o comando não tem, e por isso nunca
                        // fazia nada. Removido; decolar e pousar são do mundo, pelo item do
                        // slot de voo.
                        //
                        // C2S 21 (`GET_EXT_PROP`) e 39 (`GET_ALL_DATA`) migraram para o
                        // `pw-gs`. Os dois respondiam com números escritos no código —
                        // `120/120/280/280` de vida e mana e `50000` de dinheiro, iguais
                        // para qualquer personagem — porque o daemon de link não tem a
                        // simulação de onde tirar os valores de verdade.
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
                // O cliente pede a configuração a cada `TASK_DATA` (`OnMsgHstTaskData` →
                // `LoadConfigData`, `EC_HostMsg.cpp:3947-3949`), e o original só manda esse
                // comando no fim da resposta ao `GET_ALL_DATA`. Com mundo, o link não manda
                // mais `TASK_DATA` próprio (B53), então o pedido vem uma vez, depois dos dados.
                // Responde-se **uma vez**; responder duas vezes derruba o cliente
                // (`OnAllInitDataReady` duas vezes). A espera pelo `TASK_DATA` do mundo fica
                // como guarda.
                let pronto = match self.uplink_da_sessao(&session) {
                    Some(u) => u.dados_iniciais_entregues(req.role_id).await,
                    None => true,
                };
                if session.ui_config_enviado {
                    debug!("GetUIConfig de {} repetido — já respondido", req.role_id);
                } else if !pronto {
                    info!("GetUIConfig de {} antes dos dados do mundo — espera o pedido depois do TASK_DATA do mundo", req.role_id);
                } else {
                    session.ui_config_enviado = true;
                    let ui_config = match session.role_id {
                        Some(dono) if dono == req.role_id => {
                            let repo = self.char_repo.client_config();
                            let gravado = repo.ui_config(dono).await.unwrap_or_default();
                            if gravado.is_empty() {
                                // Sem configuração gravada: a do molde da classe, como o
                                // `gamedbd` faz ao criar o personagem (B53).
                                repo.ui_config_do_molde(dono).await.unwrap_or_default()
                            } else {
                                gravado
                            }
                        }
                        _ => Vec::new(),
                    };
                    tx.send(OutboundPacket::GetUIConfigRe(S2CGetUIConfigRe::new(
                        req.role_id,
                        req.localsid,
                        &ui_config,
                    ))).await?;
                    info!("GetUIConfig_Re enviado ao personagem {} ({} bytes)", req.role_id, ui_config.len());
                }
            }

            InboundPacket::SetUIConfig(req) => {
                debug!("Salvando UIConfig ({} bytes) para o personagem ID {}", req.ui_config.len(), req.role_id);
                // Só grava para o personagem desta sessão: o `roleid` vem do cliente.
                let mut resultado = 0;
                if session.role_id != Some(req.role_id)
                    || req.ui_config.len() > pw_storage::TAMANHO_MAXIMO_DA_CONFIGURACAO
                {
                    warn!("SetUIConfig recusado: roleid {} na sessão de {:?}, {} bytes", req.role_id, session.role_id, req.ui_config.len());
                    resultado = 1;
                } else if let Err(e) = self.char_repo.client_config().gravar_ui_config(req.role_id, &req.ui_config).await {
                    warn!("SetUIConfig de {} não gravado: {e}", req.role_id);
                    resultado = 1;
                }
                // O `localsid` é obrigatório aqui — ver o comentário de
                // `S2CSetUIConfigRe`: sem ele o cliente lê o resto do fluxo deslocado e
                // derruba a conexão com "Decode error 103" (item 18 do
                // docs/ESTADO_E_RETOMADA.md).
                tx.send(OutboundPacket::SetUIConfigRe(S2CSetUIConfigRe {
                    result: resultado,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::SetCustomData(req) => {
                debug!("Salvando CustomData ({} bytes) para o personagem ID {}", req.data.len(), req.role_id);
                tx.send(OutboundPacket::SetCustomDataRe(S2CSetCustomDataRe {
                    result: 0,
                    crc: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                })).await?;
            }

            InboundPacket::PlayerBaseInfo(req) => {
                // Visibilidade entre jogadores (docs/ESTADO_E_RETOMADA.md, itens 17/19):
                // o cliente manda isto sozinho, logo depois de receber PLAYER_ENTER_WORLD
                // de outro jogador (`EC_ManPlayer.cpp::OnMsgPlayerInfo`) — sem esta
                // resposta o avatar dele nunca materializa na tela, porque o cliente não
                // sabe que raça/classe/gênero desenhar (`info_player_1` não carrega
                // isso). Confirmado lendo `CECElsePlayer::OnMsgPlayerBaseInfo`: um
                // `name` vazio faz a função sair sem marcar `IsBaseInfoReady()`, e o
                // avatar fica pra sempre incompleto — então o nome tem que vir certo,
                // mesmo quando o personagem não é achado.
                info!("PlayerBaseInfo pedido pelo personagem ID {} pra {:?}", req.role_id, req.playerlist);
                for outro_id in &req.playerlist {
                    let info = self.char_repo.get_public_info(*outro_id, &self.realm_id).await?;
                    let resposta = match info {
                        Some(p) => S2CPlayerBaseInfoRe {
                            retcode: 0,
                            role_id: req.role_id,
                            localsid: req.localsid,
                            other_role_id: p.id,
                            name: p.name,
                            race: p.race,
                            cls: p.cls,
                            gender: p.gender,
                            // Achado em 2026-09-05, testando com dois clients reais: vazio
                            // é um caminho "válido" no sentido de não travar
                            // (`OnMsgPlayerBaseInfo` marca `IsCustomDataReady()` mesmo
                            // assim), mas sem os bytes reais `m_CustomizeData` nunca é
                            // preenchido por `ChangeCustomizeData` — e
                            // `LoadPlayerSkeleton` enfileira o carregamento do modelo com
                            // um `bodyID` de lixo, que a thread de carregamento descarta
                            // em silêncio. Resultado: nome aparece (base info funciona),
                            // mas o modelo nunca é criado — só a colisão. A captura real
                            // do 1.2.6 confirma: todo `PlayerBaseInfo_Re` de um jogador de
                            // verdade tem a aparência preenchida (172 bytes na amostra),
                            // nunca vazia. `custom_data` já vem carregado em `p` (a mesma
                            // coluna que `write_role_info` usa pro próprio personagem) —
                            // só faltava não descartar.
                            custom_data: p.custom_data,
                            status: 0,
                            create_time: p.created_at.timestamp() as i32,
                            lastlogin_time: p.updated_at.timestamp() as i32,
                            forbid: Vec::new(),
                        },
                        None => S2CPlayerBaseInfoRe {
                            retcode: 1,
                            role_id: req.role_id,
                            localsid: req.localsid,
                            other_role_id: *outro_id,
                            name: String::new(),
                            race: 0,
                            cls: 0,
                            gender: 0,
                            custom_data: Vec::new(),
                            status: 0,
                            create_time: 0,
                            lastlogin_time: 0,
                            forbid: Vec::new(),
                        },
                    };
                    tx.send(OutboundPacket::PlayerBaseInfoRe(resposta)).await?;
                }
            }

            InboundPacket::GetCustomData(req) => {
                // Só chega se o cliente pedir depois de já ter a base — ver o
                // comentário em `PlayerBaseInfo` acima sobre por que isto costuma nem
                // ser necessário.
                info!("GetCustomData pedido pelo personagem ID {} pra {:?}", req.role_id, req.playerlist);
                for outro_id in &req.playerlist {
                    let dados = self
                        .char_repo
                        .get_public_info(*outro_id, &self.realm_id)
                        .await?
                        .map(|p| p.custom_data)
                        .unwrap_or_default();
                    tx.send(OutboundPacket::GetCustomDataRe(S2CGetCustomDataRe {
                        retcode: 0,
                        role_id: req.role_id,
                        localsid: req.localsid,
                        cus_role_id: *outro_id as u32,
                        custom_data: dados,
                    })).await?;
                }
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
                let gravadas = match session.role_id {
                    Some(dono) if dono == req.role_id => {
                        self.char_repo.client_config().help_states(dono).await.unwrap_or(None)
                    }
                    _ => None,
                };
                tx.send(OutboundPacket::GetHelpStatesRe(S2CGetHelpStatesRe {
                    result: 0,
                    role_id: req.role_id,
                    localsid: req.localsid,
                    help_states: gravadas.unwrap_or_else(|| vec![0u8; 32]),
                })).await?;
            }

            InboundPacket::SetHelpStates(req) => {
                debug!("Salvando HelpStates ({} bytes) para o personagem ID {}", req.help_states.len(), req.role_id);
                if session.role_id == Some(req.role_id)
                    && req.help_states.len() <= pw_storage::TAMANHO_MAXIMO_DA_CONFIGURACAO
                {
                    if let Err(e) = self.char_repo.client_config().gravar_help_states(req.role_id, &req.help_states).await {
                        warn!("SetHelpStates de {} não gravado: {e}", req.role_id);
                    }
                }
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

                // A posição não é mais guardada aqui: quem apresenta um jogador aos
                // outros é o mundo, que já tem a posição de verdade na grade espacial.

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

                self.broadcast_para_todos(move_broadcast).await;
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

                self.broadcast_para_todos(broadcast_pkt).await;
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

/// As cinco listas de missão de quem nunca teve nenhuma, como `pw_gs::missoes::ListasDeMissao`
/// as serializa: lista ativa só com o cabeçalho (`m_Version` = 1, tempos absolutos), lista de
/// concluídas com `m_Version` = 1, as duas de contagem vazias e o depósito zerado
/// (`sizeof(StorageTaskList)` = 864, `task/TaskProcess.h:379-391`).
fn listas_de_missao_vazias() -> [Vec<u8>; 5] {
    [vec![0, 0, 1, 0, 0, 1, 0, 0], vec![0, 0, 1, 0], vec![0, 0], vec![0, 0], vec![0; 864]]
}
