//! Vários mapas num processo só: o roteador do barramento.
//!
//! # Por que existe
//!
//! Até 2026-09-14 cada mapa era um contêiner (`pw-world-155` para o mundo 1,
//! `pw-world-155-161` para o 161), e cada um carregava sozinho o `GameDataManager`
//! inteiro: **1,1 GB** o do 161, com 1.269 monstros, quase o mesmo que o do mundo 1 com
//! 29.620. O `gs.conf` do 1.5.5 lista ~80 mapas; um contêiner por mapa não escala.
//!
//! O original sobe vários mapas com **um comando** (`./gs gs01 gs.conf gmserver.conf
//! gsalias.conf is61`, no `start` do `pwserver_155v156`): carrega os dados comuns uma vez
//! (`FirstStepInit`) e faz `fork()` de um processo por mapa, que compartilham a memória já
//! carregada (`cgame/gs/start.cpp:185-234`). Aqui o equivalente é um processo com um
//! [`WorldInstance`] por mapa — cada um com seu próprio laço de tick — sobre o **mesmo**
//! `Arc<GameDataManager>`.
//!
//! # Como o jogador chega ao mapa certo
//!
//! O `pw-link` manda tudo para um endereço só. No `EnterWorld`, o roteador pergunta ao
//! banco em que mapa o personagem está gravado e passa a entregar a esse mapa tudo o que
//! vier daquele `roleid`: subcomandos e a saída. Cada mapa continua sendo um [`BusServer`]
//! completo — sessões, eventos do tick, visibilidade —, só que atrás do roteador em vez de
//! escutar o barramento sozinho.
//!
//! Separar um mapa pesado noutro processo continua possível: o link aceita
//! `GS_BUS=1=a:29100,161=b:29100`, e o roteador de cada processo só conhece os seus mapas.

use crate::bus_server::{BusServer, EnvioAoCliente};
use crate::world::WorldInstance;
use pw_bus::{BusListener, BusMessage};
use pw_storage::CharacterRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Os mapas deste processo, e de qual mapa é cada jogador.
pub struct RoteadorDeMapas {
    mapas: HashMap<i32, Arc<BusServer>>,
    /// O mapa que recebe um personagem cujo mapa gravado não é servido aqui.
    padrao: i32,
    donos: RwLock<HashMap<i32, i32>>,
    repo: CharacterRepository,
    /// Só transições de presença. Consulta administrativa nunca segura isto durante I/O.
    presenca: RwLock<()>,
}

impl RoteadorDeMapas {
    pub async fn iniciar_coordenacao_gm(self: &Arc<Self>, contas: pw_storage::AccountRepository,
        processo: String) -> anyhow::Result<tokio::task::JoinHandle<()>> {
        let repo = contas.coordenacao_gm();
        let encarnacao = hex::encode(pw_crypto::generate_login_challenge());
        let mut conexao = repo.registrar(&processo,&encarnacao).await?;
        let este = Arc::clone(self);
        let tarefa = tokio::spawn(async move {
            let mut intervalo = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                intervalo.tick().await;
                // Exclui entrada, saída, transferência e comandos enquanto reconcilia.
                // I/O só sob guarda de coordenação; nenhum lock do mundo durante banco.
                let _barreira = este.presenca.write().await;
                let fotografia = tokio::time::timeout(std::time::Duration::from_secs(2),repo.fotografia()).await;
                let f = match fotografia { Ok(Ok(f)) => Some(f), _ => None };
                for mapa in este.mapas.values() { mapa.reconciliar_gm(f.as_ref()).await; }
                if let Some(f) = f {
                    if pw_storage::CoordenacaoGmRepository::confirmar(&mut conexao,&processo,&encarnacao,f.revisao).await.is_err() {
                        // O jogo não cai por isso (MEMORIA_DA_REFORMA §6.2 item 3): efeitos GM
                        // removidos e coordenação parada; comandos GM seguem conferidos no banco.
                        tracing::error!("coordenação GM: conexão de fencing perdida; coordenação desligada até reiniciar o GS");
                        for mapa in este.mapas.values() { mapa.reconciliar_gm(None).await; }
                        break;
                    }
                } else { tracing::warn!("coordenação GM: banco indisponível, efeitos removidos, sem recibo"); }
            }
        });
        Ok(tarefa)
    }
    /// `mapas` na ordem da configuração; o primeiro é o padrão.
    pub fn new(mapas: Vec<(i32, Arc<BusServer>)>, repo: CharacterRepository) -> Self {
        assert!(!mapas.is_empty(), "um servidor de mundo sem mapa nenhum");
        let padrao = mapas[0].0;
        Self {
            mapas: mapas.into_iter().collect(),
            padrao,
            donos: RwLock::new(HashMap::new()),
            repo,
            presenca: RwLock::new(()),
        }
    }

    /// Começa a atender os pedidos de troca de mapa dos mapas deste processo.
    ///
    /// O original troca o jogador de processo (`SwitchSvr` → `PlaneSwitch`, `player.cpp:3191`);
    /// aqui os mapas são tarefas do mesmo processo, então a troca é tirar de um e pôr no outro,
    /// e passar a entregar aquele `roleid` ao novo. Mapa que este processo não serve não tem
    /// troca (`falta`: seria o link reencaminhar a sessão).
    pub fn ligar_trocas(self: &Arc<Self>) {
        let (envio, mut fila) = mpsc::unbounded_channel::<crate::bus_server::PedidoDeTroca>();
        for mapa in self.mapas.values() {
            mapa.ligar_trocas(envio.clone());
            mapa.ligar_roteador(Arc::downgrade(self));
        }
        let este = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(p) = fila.recv().await {
                este.trocar(p).await;
            }
        });
    }

    /// Leva o jogador a `pos` do mapa `mundo` — o mesmo caminho do teleporte de missão.
    pub async fn transportar(&self, roleid: i32, mundo: i32, pos: pw_core::Vector3) {
        if let Some(atual) = self.mapa_de(roleid).await.and_then(|m| self.mapas.get(&m)) {
            atual.transportar(roleid, mundo, pos).await;
        }
    }

    async fn trocar(self: &Arc<Self>, p: crate::bus_server::PedidoDeTroca) {
        let _transicao = self.presenca.write().await;
        let controle=self.repo.controle_de_gravacao(p.roleid);
        let _guarda=controle.alterar().await;
        let Some(destino) = self.mapas.get(&p.mundo) else {
            warn!(
                "mundo: {} pediu o mapa {}, que este processo não serve ({:?})",
                p.roleid,
                p.mundo,
                self.mapas()
            );
            return;
        };
        let Some(origem) = self
            .mapa_de(p.roleid)
            .await
            .and_then(|m| self.mapas.get(&m))
        else {
            return;
        };
        let Some(vindo) = origem.retirar_para_troca(p.roleid).await else {
            return;
        };
        let envio=vindo.envio();
        let mascote=vindo.mascote.clone();
        match destino.receber_de_outro_mapa(p.roleid,vindo,p.pos).await {
            Ok(())=> {
                let (saiu,resultado)=origem.concluir_troca_na_origem(p.roleid,mascote.clone()).await;
                self.donos.write().await.insert(p.roleid,p.mundo);
                if saiu {destino.encerrar_apos_transferencia(p.roleid,&envio,resultado).await;}
            },
            Err((vindo,false))=> {origem.restaurar_troca(p.roleid,vindo).await;},
            Err((vindo,true))=> {
                // Não restaurar por inferência: o commit pode ter ocorrido. Fotografia
                // congelada e mesma sessão ficam reservadas até confirmar o mesmo destino.
                let este=Arc::clone(self);
                let origem=Arc::clone(origem);
                let destino=Arc::clone(destino);
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        let _transicao=este.presenca.write().await;
                        let controle=este.repo.controle_de_gravacao(p.roleid);
                        let _guarda=controle.alterar().await;
                        if destino.receber_de_outro_mapa(p.roleid,vindo.clone(),p.pos).await.is_ok() {
                            let (saiu,resultado)=origem.concluir_troca_na_origem(p.roleid,mascote.clone()).await;
                            este.donos.write().await.insert(p.roleid,p.mundo);
                            if saiu {destino.encerrar_apos_transferencia(p.roleid,&envio,resultado).await;}
                            break;
                        }
                    }
                });
            }
        }
    }

    /// Os ids dos mapas servidos, em ordem.
    pub fn mapas(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.mapas.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// Ausência é observação deste processo, nunca autorização para editar offline.
    pub async fn consultar_administrativamente(&self, roleid: i32) -> serde_json::Value {
        let Ok(_leitura) = self.presenca.try_read() else {
            return serde_json::json!({"presenca":"em_transicao"});
        };
        let Some(mapa) = self.mapa_de(roleid).await else {
            // A queda do link esquece sessões, mas pode deixar uma entidade residual
            // (`atender`, abaixo). Ausência de roteamento não prova ausência em memória.
            for servidor in self.mapas.values() {
                if servidor.mundo().read().await.players.contains_key(&(roleid as i64)) {
                    return serde_json::json!({"presenca":"inconsistente","motivo":"entidade_sem_roteamento"});
                }
            }
            return serde_json::json!({"presenca":"ausente"});
        };
        let servidor = &self.mapas[&mapa];
        let ficha = {
            let mundo = servidor.mundo().read().await;
            mundo.players.get(&(roleid as i64)).map(|j| crate::administracao::FichaViva::do_jogador(j, mapa))
        };
        if servidor.tem_sessao(roleid).await && ficha.is_some() {
            serde_json::json!({"presenca":"online","ficha":ficha})
        } else {
            serde_json::json!({"presenca":"em_transicao"})
        }
    }

    pub async fn resumo_administrativo(&self) -> serde_json::Value {
        let Ok(_leitura) = self.presenca.try_read() else {
            return serde_json::json!({"presenca":"em_transicao"});
        };
        let donos = self.donos.read().await.clone();
        let mut mapas = Vec::new();
        for mapa in self.mapas() {
            let servidor = &self.mapas[&mapa];
            let ids: Vec<i32> = {
                let mundo = servidor.mundo().read().await;
                mundo.players.keys().filter_map(|id| i32::try_from(*id).ok()).collect()
            };
            let mut online = 0;
            for id in ids {
                if donos.get(&id) == Some(&mapa) && servidor.tem_sessao(id).await { online += 1; }
            }
            mapas.push(serde_json::json!({"mapa":mapa,"jogadores_online":online}));
        }
        serde_json::json!({"presenca":"observada","mapas":mapas})
    }

    /// Mapa e posição de um jogador atendido por este processo (comandos de GM).
    pub(crate) async fn posicao_de(&self, roleid: i32) -> Option<(i32, pw_core::Vector3)> {
        let mapa = self.mapa_de(roleid).await?;
        let pos = self.mapas.get(&mapa)?.posicao_do_jogador(roleid).await?;
        Some((mapa, pos))
    }

    /// Em que mapa este jogador está sendo atendido agora.
    pub async fn mapa_de(&self, roleid: i32) -> Option<i32> {
        self.donos.read().await.get(&roleid).copied()
    }

    /// Aceita conexões de daemons de link até a escuta cair.
    pub async fn executar(self: Arc<Self>, escuta: BusListener) {
        info!(
            "servidor de mundo escutando o barramento pelos mapas {:?}",
            self.mapas()
        );
        loop {
            match escuta.aceitar().await {
                Ok(conexao) => {
                    let este = Arc::clone(&self);
                    tokio::spawn(async move { este.atender(conexao).await });
                }
                Err(e) => {
                    warn!("barramento: falha ao aceitar conexão: {e}");
                    return;
                }
            }
        }
    }

    /// Como [`BusServer::atender`], mas entregando cada mensagem ao mapa do jogador.
    async fn atender(&self, mut conexao: pw_bus::transport::BusConnection) {
        let par = conexao.par().to_string();
        let (envio, mut fila) = mpsc::channel::<BusMessage>(256);
        let mut desta_conexao: Vec<i32> = Vec::new();

        loop {
            tokio::select! {
                entrada = conexao.receber() => {
                    match entrada {
                        Ok(Some(msg)) => {
                            if let BusMessage::EnterWorld { roleid, .. } = &msg {
                                desta_conexao.push(*roleid);
                            }
                            let saiu = match &msg {
                                BusMessage::PlayerLogout { roleid, .. } => Some(*roleid),
                                _ => None,
                            };
                            self.entregar(msg, &envio).await;
                            if let Some(r) = saiu {
                                let ainda_dona=if let Some(m)=self.mapa_de(r).await {
                                    self.mapas[&m].pertence_a(r,&envio).await
                                } else {false};
                                if !ainda_dona {desta_conexao.retain(|x|*x!=r);}
                                if let Some(m)=self.mapa_de(r).await {
                                    if !self.mapas[&m].tem_sessao(r).await {self.donos.write().await.remove(&r);}
                                }
                            }
                        }
                        Ok(None) => {
                            debug!("barramento: {par} desconectou");
                            break;
                        }
                        Err(e) => {
                            warn!("barramento: erro lendo de {par}: {e}");
                            break;
                        }
                    }
                }
                saida = fila.recv() => {
                    match saida {
                        Some(msg) => {
                            if let Err(e) = conexao.enviar(msg).await {
                                warn!("barramento: erro escrevendo para {par}: {e}");
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        // A conexão caiu: cada mapa esquece as sessões que vinham por ela.
        let _transicao = self.presenca.write().await;
        for roleid in desta_conexao {
            if let Some(mapa)=self.mapa_de(roleid).await.and_then(|m|self.mapas.get(&m)) {
                mapa.esquecer_sessoes(&[roleid],&envio).await;
            }
        }
    }

    async fn entregar(&self, msg: BusMessage, envio: &EnvioAoCliente) {
        // Mesmo cabeçalho já decodificado por bus_server.rs::SubComando::ler;
        // apenas reconhece logout para a guarda, sem novo layout/pacote de cliente.
        let transicao = matches!(&msg, BusMessage::EnterWorld { .. } | BusMessage::PlayerLogout { .. })
            || matches!(&msg, BusMessage::ClientToGame { data, .. }
                if data.get(..2) == Some(&crate::comandos::ids::LOGOUT.to_le_bytes()));
        let _presenca = if transicao { Some(self.presenca.write().await) } else { None };
        let _comando = if !transicao { Some(self.presenca.read().await) } else { None };
        let roleid = match &msg {
            BusMessage::EnterWorld { roleid, .. }
            | BusMessage::PlayerLogout { roleid, .. }
            | BusMessage::ClientToGame { roleid, .. }
            | BusMessage::GameToClient { roleid, .. } => *roleid,
            BusMessage::ChatSingleCast { dstroleid, .. } => *dstroleid,
        };

        let mapa = if matches!(msg, BusMessage::EnterWorld { .. }) {
            if let Some(m)=self.mapa_de(roleid).await {
                if self.mapas[&m].tem_sessao(roleid).await {
                    warn!("entrada: {roleid} já possui sessão ou saída pendente; rejeitada duplicata");
                    return;
                }
            }
            let mapa = self.mapa_para_entrar(roleid).await;
            self.donos.write().await.insert(roleid, mapa);
            mapa
        } else {
            match self.mapa_de(roleid).await {
                Some(m) => m,
                None => {
                    debug!("mundo: mensagem de {roleid}, que não entrou em mapa nenhum — ignorada");
                    return;
                }
            }
        };

        self.mapas[&mapa].tratar(msg, envio).await;
    }

    pub(crate) async fn apagar_rota(&self,role:i32,mapa:i32) {
        let mut donos=self.donos.write().await;
        if donos.get(&role)==Some(&mapa) {donos.remove(&role);}
    }
    pub(crate) async fn repetir_saida(&self,mapa:&BusServer,role:i32)->bool {
        let _transicao=self.presenca.write().await;
        let controle=self.repo.controle_de_gravacao(role);
        let _guarda=controle.alterar().await;
        mapa.confirmar_saida(role).await
    }

    /// O mapa gravado do personagem, se este processo o serve; senão o padrão, com aviso.
    async fn mapa_para_entrar(&self, roleid: i32) -> i32 {
        match self.repo.mundo_do_personagem(roleid).await {
            Ok(Some(m)) if self.mapas.contains_key(&m) => m,
            Ok(Some(m)) => {
                warn!(
                    "mundo: {roleid} está gravado no mapa {m}, que este processo não serve \
                     ({:?}) — vai para o mapa {}",
                    self.mapas(),
                    self.padrao
                );
                self.padrao
            }
            Ok(None) => self.padrao,
            Err(e) => {
                warn!(
                    "mundo: não consegui ler o mapa de {roleid}: {e} — vai para o {}",
                    self.padrao
                );
                self.padrao
            }
        }
    }

    /// Monta um mapa pronto para o roteador: mundo com spawns, laço de tick e eventos.
    pub async fn preparar_mapa(
        mut mundo: WorldInstance,
        versao: pw_protocol::GameVersion,
    ) -> (i32, Arc<BusServer>) {
        let id = mundo.world_id;
        mundo.init_spawns();
        let servidor = Arc::new(crate::server::GameServer::new(mundo));
        let bus = Arc::new(BusServer::new(Arc::clone(&servidor.world), versao));
        bus.ligar_eventos_do_mundo().await;
        // Um pânico no tick de um mapa derruba só a tarefa daquele mapa — e sem este aviso,
        // em silêncio: os outros continuariam, e aquele ficaria parado.
        let tick = tokio::spawn(servidor.run_tick_loop());
        tokio::spawn(async move {
            if let Err(e) = tick.await {
                tracing::error!("mapa {id}: o laço de tick parou: {e}");
            }
        });
        (id, bus)
    }
}
