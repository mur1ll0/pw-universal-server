//! A ponta do `pw-link` no barramento: por onde ele fala com o servidor de mundo.
//!
//! # O que este módulo resolve
//!
//! Um daemon de link atende centenas de jogadores, cada um numa tarefa própria, mas fala
//! com o servidor de mundo por **uma conexão só**. Então é preciso alguém no meio que
//! saiba duas coisas:
//!
//! - multiplexar a saída: várias tarefas mandando para o mesmo socket sem embaralhar
//!   quadros — daí a fila única e a tarefa única que escreve;
//! - demultiplexar a entrada: o que volta vem endereçado por `roleid`
//!   (`S2CGamedataSend`, opcode 74), e precisa achar a tarefa daquele jogador — daí o
//!   registro de sessões.
//!
//! # O que ele deliberadamente **não** faz
//!
//! Não interpreta o `data`. O que trafega ali é o formato do mundo 3D — little-endian,
//! `pack(1)` — e quem o lê é o `pw-gs`. Aqui ele é um bloco de bytes opaco, e é assim
//! que a fronteira entre os dois formatos fica em um lugar só.
//!
//! # Se o mundo estiver fora do ar
//!
//! O barramento é melhor-esforço: [`BusUplink::enviar`] devolve `false` e segue. Um
//! servidor de mundo caído não pode derrubar a sessão do jogador no link — ele ainda
//! precisa poder receber a mensagem de erro e sair. A reconexão é automática, com espera
//! crescente até 30s, então o mundo pode voltar sem que o link seja reiniciado.

use pw_bus::{BusClient, BusMessage};
use pw_protocol::{OutboundPacket, S2CGamedataSend, S2CPlayerLogout};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, trace, warn};

/// Canal de saída de uma sessão de cliente — o mesmo `tx` que o gateway já usa.
pub type EnvioAoCliente = mpsc::Sender<OutboundPacket>;

/// Quantas mensagens podem esperar na fila de saída antes de começarem a ser recusadas.
///
/// A fila protege o link: se o mundo parar de ler, o que enche é esta fila e não a
/// memória do processo.
const CAPACIDADE_DA_FILA: usize = 1024;

/// Espera máxima entre tentativas de reconexão.
const ESPERA_MAXIMA: Duration = Duration::from_secs(30);

/// A ligação deste daemon de link com o servidor de mundo do seu realm.
pub struct BusUplink {
    /// Fila de saída. Escrever aqui nunca bloqueia a tarefa do jogador.
    saida: mpsc::Sender<BusMessage>,
    /// Quem está em jogo por este link, por `roleid`.
    sessoes: Arc<RwLock<HashMap<i32, EnvioAoCliente>>>,
}

impl BusUplink {
    /// Conecta ao servidor de mundo e deixa a ligação rodando em segundo plano.
    ///
    /// Não falha: se o mundo ainda não subiu, a tarefa de fundo fica tentando. Isso é
    /// proposital — no `docker-compose` os dois contêineres sobem juntos, e exigir que o
    /// mundo esteja pronto primeiro tornaria a ordem de subida uma condição de corrida.
    pub fn iniciar(endereco: String) -> Arc<Self> {
        let (saida, fila) = mpsc::channel::<BusMessage>(CAPACIDADE_DA_FILA);
        let sessoes = Arc::new(RwLock::new(HashMap::new()));

        let este = Arc::new(Self {
            saida,
            sessoes: Arc::clone(&sessoes),
        });

        tokio::spawn(manter_ligacao(endereco, fila, sessoes));
        este
    }

    /// Anuncia ao mundo que este jogador é atendido por este link.
    pub async fn registrar(&self, roleid: i32, envio: EnvioAoCliente) {
        self.sessoes.write().await.insert(roleid, envio);
    }

    /// Tira o jogador do registro. O que vier do mundo para ele depois disso é
    /// descartado com um aviso, e não entregue à sessão errada.
    pub async fn desregistrar(&self, roleid: i32) {
        self.sessoes.write().await.remove(&roleid);
    }

    /// Enfileira uma mensagem para o servidor de mundo.
    ///
    /// `false` quando a fila está cheia ou a ligação morreu de vez. O chamador registra e
    /// segue: perder um subcomando é ruim, derrubar o jogador é pior.
    pub fn enviar(&self, msg: BusMessage) -> bool {
        match self.saida.try_send(msg) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(m)) => {
                warn!(
                    "barramento: fila cheia, descartando opcode {} do jogador {}",
                    m.opcode(),
                    m.roleid()
                );
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    }

    /// Quantos jogadores deste link estão registrados no mundo.
    pub async fn jogadores_registrados(&self) -> usize {
        self.sessoes.read().await.len()
    }
}

/// Mantém a conexão com o servidor de mundo, reconectando quando ela cai.
async fn manter_ligacao(
    endereco: String,
    mut fila: mpsc::Receiver<BusMessage>,
    sessoes: Arc<RwLock<HashMap<i32, EnvioAoCliente>>>,
) {
    let mut espera = Duration::from_millis(500);

    loop {
        match BusClient::conectar(&endereco).await {
            Ok(mut conexao) => {
                info!("barramento: ligado ao servidor de mundo em {endereco}");
                espera = Duration::from_millis(500);

                loop {
                    tokio::select! {
                        saindo = fila.recv() => {
                            let Some(msg) = saindo else {
                                // A fila fechou: o gateway está encerrando.
                                info!("barramento: fila de saída fechada, encerrando a ligação");
                                return;
                            };
                            if let Err(e) = conexao.enviar(msg).await {
                                warn!("barramento: erro escrevendo para o mundo: {e}");
                                break;
                            }
                        }
                        entrando = conexao.receber() => {
                            match entrando {
                                Ok(Some(msg)) => entregar(msg, &sessoes).await,
                                Ok(None) => {
                                    warn!("barramento: o servidor de mundo fechou a conexão");
                                    break;
                                }
                                Err(e) => {
                                    warn!("barramento: erro lendo do mundo: {e}");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("barramento: {endereco} ainda não responde ({e})");
            }
        }

        tokio::time::sleep(espera).await;
        espera = (espera * 2).min(ESPERA_MAXIMA);
    }
}

/// Entrega ao jogador certo o que o mundo mandou.
async fn entregar(msg: BusMessage, sessoes: &RwLock<HashMap<i32, EnvioAoCliente>>) {
    match msg {
        BusMessage::GameToClient { roleid, data, .. } => {
            let sessoes = sessoes.read().await;
            let Some(envio) = sessoes.get(&roleid) else {
                // O jogador saiu entre o mundo mandar e a mensagem chegar. Normal numa
                // desconexão; só não pode ser entregue a mais ninguém.
                debug!("barramento: chegou algo para {roleid}, que não está mais neste link");
                return;
            };
            trace!("barramento: {} bytes do mundo para {roleid}", data.len());
            // O `data` atravessa opaco: é o subcomando do mundo 3D, e o envelope que o
            // cliente espera é o `GamedataSend` (34) — não o do barramento.
            if envio
                .try_send(OutboundPacket::GamedataSend(S2CGamedataSend::new(data)))
                .is_err()
            {
                warn!("barramento: a fila do jogador {roleid} está cheia ou fechada");
            }
        }
        BusMessage::PlayerLogout {
            result,
            roleid,
            localsid,
            ..
        } => {
            // O mundo decidiu que este jogador está saindo. Quem fala com o cliente é o
            // link, então é aqui que aquilo vira o pacote GNET que o cliente espera.
            let sessoes = sessoes.read().await;
            let Some(envio) = sessoes.get(&roleid) else {
                debug!("barramento: saída de {roleid}, que não está mais neste link");
                return;
            };
            info!("barramento: o mundo encerrou a sessão de {roleid} (result {result})");
            if envio
                .try_send(OutboundPacket::PlayerLogout(S2CPlayerLogout::new(
                    result, roleid, localsid,
                )))
                .is_err()
            {
                warn!("barramento: não consegui entregar a saída a {roleid}");
            }
        }

        outra => {
            // Sobram `EnterWorld` e `ClientToGame`, que são o sentido de entrada. Recebê-
            // los aqui significa que alguém ligou dois links um no outro.
            warn!(
                "barramento: o mundo mandou opcode {} — sentido inesperado",
                outra.opcode()
            );
        }
    }
}
