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
}

impl RoteadorDeMapas {
    /// `mapas` na ordem da configuração; o primeiro é o padrão.
    pub fn new(mapas: Vec<(i32, Arc<BusServer>)>, repo: CharacterRepository) -> Self {
        assert!(!mapas.is_empty(), "um servidor de mundo sem mapa nenhum");
        let padrao = mapas[0].0;
        Self {
            mapas: mapas.into_iter().collect(),
            padrao,
            donos: RwLock::new(HashMap::new()),
            repo,
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

    async fn trocar(&self, p: crate::bus_server::PedidoDeTroca) {
        let Some(destino) = self.mapas.get(&p.mundo) else {
            warn!("mundo: {} pediu o mapa {}, que este processo não serve ({:?})", p.roleid, p.mundo, self.mapas());
            return;
        };
        let Some(origem) = self.mapa_de(p.roleid).await.and_then(|m| self.mapas.get(&m)) else {
            return;
        };
        let Some(vindo) = origem.retirar_para_troca(p.roleid).await else {
            return;
        };
        self.donos.write().await.insert(p.roleid, p.mundo);
        destino.receber_de_outro_mapa(p.roleid, vindo, p.pos).await;
    }

    /// Os ids dos mapas servidos, em ordem.
    pub fn mapas(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.mapas.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// Em que mapa este jogador está sendo atendido agora.
    pub async fn mapa_de(&self, roleid: i32) -> Option<i32> {
        self.donos.read().await.get(&roleid).copied()
    }

    /// Aceita conexões de daemons de link até a escuta cair.
    pub async fn executar(self: Arc<Self>, escuta: BusListener) {
        info!("servidor de mundo escutando o barramento pelos mapas {:?}", self.mapas());
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
                                desta_conexao.retain(|x| *x != r);
                                self.donos.write().await.remove(&r);
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
        let mut donos = self.donos.write().await;
        for roleid in desta_conexao {
            if let Some(mapa) = donos.remove(&roleid).and_then(|m| self.mapas.get(&m)) {
                mapa.esquecer_sessoes(&[roleid]).await;
            }
        }
    }

    async fn entregar(&self, msg: BusMessage, envio: &EnvioAoCliente) {
        let roleid = match &msg {
            BusMessage::EnterWorld { roleid, .. }
            | BusMessage::PlayerLogout { roleid, .. }
            | BusMessage::ClientToGame { roleid, .. }
            | BusMessage::GameToClient { roleid, .. } => *roleid,
        };

        let mapa = if matches!(msg, BusMessage::EnterWorld { .. }) {
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
                warn!("mundo: não consegui ler o mapa de {roleid}: {e} — vai para o {}", self.padrao);
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
