//! A rede do servidor de mundo: onde o `pw-gs` entra no caminho do jogo.
//!
//! Até aqui o `pw-gs` tinha um tick loop e **nenhuma rede** — nada chegava nele, e a
//! simulação do mundo rodava dentro do `gateway.rs` do `pw-link`. É essa a causa
//! registrada de "o cliente entra mas nada funciona".
//!
//! Este módulo é a ponta que escuta o barramento. O que chega é [`BusMessage`]; o que
//! ele faz com cada uma:
//!
//! | Mensagem | O que significa |
//! | :--- | :--- |
//! | `EnterWorld` (72) | aquele jogador passa a ser deste servidor de mundo |
//! | `ClientToGame` (75) | um subcomando do mundo 3D, vindo do cliente |
//! | `PlayerLogout` (69) | o jogador saiu; solta os recursos dele |
//! | `GameToClient` (74) | **não** deveria chegar aqui: é o sentido de saída |
//!
//! # Os dois formatos, na fronteira
//!
//! O envelope é GNET (big-endian), e o `pw-bus` já o desfez. O `data` que sobra está no
//! **outro** formato — little-endian, `pack(1)` — e é aqui, no [`SubComando`], que ele
//! começa a ser lido, com o `pw_wire::gamedata`. Esta é literalmente a linha onde um
//! formato vira o outro.
//!
//! # Os dois sentidos
//!
//! Comandos **entram** pelo barramento e são tratados em `tratar_subcomando`. Mas o mundo
//! também decide coisas sozinho — um monstro que bate, um jogador que morre — e isso
//! **sai** pelo canal de [`EventoDoMundo`], que `entregar_evento` traduz em subcomandos.
//! Sem esse segundo caminho a simulação acontecia em silêncio: o HP do jogador caía no
//! tick e o cliente nunca era avisado.
//!
//! # O que ainda não está aqui
//!
//! A maior parte dos ~390 subcomandos continua no `gateway.rs`. O que já mudou de lado é
//! movimento, saída, alvo, ataque básico, parada, desmarcar, renascimento, bolsa (detalhe
//! e troca de slot), emote, uso de item, conjuração, os serviços de NPC (compra, venda e
//! missão) e o **grupo**.
//!
//! O que fica no `gateway.rs` é sobretudo consulta (`GET_ALL_DATA`, `GET_EXT_PROP`,
//! `QUERY_*_INFO`), `TASK_NOTIFY`, moda, duelo e Mall.

use crate::combat::CombatEngine;
use crate::comandos::{
    ids, CastSkill, ConsultaDeIds, EmoteAction, GetAllData, GetIvtrDetail, Logout, MoveIvtrItem,
    NormalAttack, ParDeSlots, PlayerMove, SelectTarget, SevnpcHello, StopMove, TaskNotify,
    TipoDeSaida, UseItem,
};
use crate::npc::{self, servico, PedidoAoNpc};
use crate::world::{EventoDoMundo, WorldInstance};
use pw_bus::{BusListener, BusMessage};
use pw_core::{ContainerType, Vector3};
use pw_protocol::{GameVersion, PorVersao, S2CGamedataSend};
use pw_wire::gamedata::Reader;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, trace, warn};

/// Um subcomando do mundo 3D, já com o cabeçalho separado do corpo.
///
/// O cabeçalho é um `unsigned short` **little-endian** (`S2C::cmd_header` /
/// `C2S::cmd_header` no IR): dois bytes, e o resto é o payload daquele comando.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubComando {
    pub id: u16,
    pub payload: Vec<u8>,
}

impl SubComando {
    /// Separa o cabeçalho de 2 bytes do corpo.
    ///
    /// Devolve `None` se não houver nem o cabeçalho — um payload curto demais é pacote
    /// malformado, e não motivo para derrubar a conexão do barramento.
    pub fn ler(data: &[u8]) -> Option<Self> {
        let mut r = Reader::new(data);
        let id = r.u16().ok()?;
        Some(Self {
            id,
            payload: r.rest().to_vec(),
        })
    }
}

/// Converte um HP de 64 bits para os 32 do fio, sem estourar.
///
/// Saturar em vez de truncar importa: `as i32` num valor grande dá negativo, e o cliente
/// desenha barra de vida vazia num monstro cheio.
fn saturar(v: i64) -> i32 {
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// A regra de divisão de despojos do grupo (`wPickFlag`).
///
/// Zero é "livre para todos", o padrão do jogo quando ninguém mudou nada. A simulação
/// ainda não guarda essa preferência por grupo; quando guardar, é este valor que sai daqui.
const PICK_FLAG_PADRAO: u16 = 0;

/// `attack_flag` sem nenhum bit ligado.
///
/// O campo existe nos comandos de resultado de ataque e o comentário do
/// `protocol.h` original diz que ele marca runas de ataque, runas de defesa e crítico —
/// mas **as posições dos bits não estão em nenhuma fonte que temos**, nem no cliente nem
/// no servidor vazado. Zero é o único valor que sabemos ser correto: nenhum efeito
/// especial. O dano crítico continua sendo calculado e debitado; o que falta é o aviso
/// visual, e está anotado como dívida em `docs/ESTADO_E_RETOMADA.md`.
const SEM_MARCACAO: i32 = 0;

/// `section` de uma habilidade que causa dano uma vez só.
///
/// O campo existe porque uma habilidade pode aplicar dano em várias etapas (o `section`
/// diz qual etapa é esta). Enquanto as habilidades daqui derem um golpe só, zero é a
/// primeira e única etapa.
const SECAO_UNICA: u8 = 0;

/// O `cEquipment` do `HOST_ATTACKED`, que diz com o que o golpe acertou.
///
/// A captura do 1.2.6 traz `0x7f` nas 25 ocorrências — todas de monstro batendo em
/// jogador. A simulação ainda não modela por onde o golpe entrou; `0x7f` é o que aquele
/// servidor manda, e portanto o que sabemos ser aceito.
const EQUIPAMENTO_PADRAO: u8 = 0x7f;

/// `attack_speed` de um golpe comum.
///
/// A simulação ainda não modela velocidade de ataque por golpe; o `PlayerEntity` tem
/// `attack_speed` como `f32` de outra escala. Enquanto os dois não se encontrarem, o
/// valor neutro é este.
const VELOCIDADE_PADRAO: u8 = 0;

/// O `reason` dos comandos de saída de grupo: o jogador pediu para sair.
///
/// Zero é a saída voluntária. Expulsão e queda de conexão têm outros códigos, e o dia em
/// que existirem entram aqui como constantes próprias em vez de números soltos.
const SAIDA_VOLUNTARIA: i16 = 0;

/// Canal por onde o mundo devolve mensagens àquele jogador.
pub type EnvioAoCliente = mpsc::Sender<BusMessage>;

/// Estado de um jogador que este servidor de mundo está atendendo.
struct Sessao {
    localsid: u32,
    envio: EnvioAoCliente,
}

/// A ponta de rede do servidor de mundo.
pub struct BusServer {
    world: Arc<RwLock<WorldInstance>>,
    /// Os subcomandos cujo layout depende da versão do realm.
    ///
    /// Trinta e dois comandos medidos num servidor 1.2.6 real têm layout diferente do
    /// 1.5.3 (item 56). Mandar o layout errado não dá erro: o cliente **descarta o comando
    /// inteiro** (item 46). Por isso a versão vive aqui dentro, e não como um argumento
    /// que se pode esquecer numa chamada.
    sub: PorVersao,
    /// Jogadores atendidos, por `roleid`. É o que permite ao mundo devolver uma
    /// mensagem a um jogador específico sem saber nada sobre conexões.
    sessoes: Arc<RwLock<HashMap<i32, Sessao>>>,
}

impl BusServer {
    /// Monta o servidor de mundo para a versão daquele realm.
    pub fn new(world: Arc<RwLock<WorldInstance>>, versao: GameVersion) -> Self {
        Self {
            world,
            sub: PorVersao::new(versao),
            sessoes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// A versão que este mundo fala.
    pub fn versao(&self) -> GameVersion {
        self.sub.versao()
    }

    /// Liga a saída de eventos da simulação a este servidor e começa a entregá-los.
    ///
    /// Sem isto, o que acontece no tick — um monstro batendo no jogador, o jogador
    /// morrendo — fica dentro do processo e o cliente nunca sabe. Era literalmente o
    /// estado anterior: o HP caía em silêncio até o jogador morrer sem aviso.
    pub async fn ligar_eventos_do_mundo(self: &Arc<Self>) {
        let (envio, mut fila) = mpsc::channel::<EventoDoMundo>(1024);
        self.world.write().await.definir_canal_de_eventos(envio);

        let este = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(ev) = fila.recv().await {
                este.entregar_evento(ev).await;
            }
            debug!("mundo: canal de eventos fechado");
        });
    }

    /// Traduz um evento da simulação nos subcomandos que o cliente entende.
    ///
    /// É aqui — e só aqui — que o que aconteceu no mundo vira protocolo.
    async fn entregar_evento(&self, ev: EventoDoMundo) {
        match ev {
            EventoDoMundo::DanoRecebido {
                roleid,
                atacante,
                dano,
                hp,
                max_hp,
            } => {
                // Dois avisos: o golpe em si, e a vida que sobrou.
                self.enviar_ao_jogador(
                    roleid,
                    self.sub
                        .host_attacked(
                            atacante as i32,
                            dano,
                            EQUIPAMENTO_PADRAO,
                            SEM_MARCACAO,
                            VELOCIDADE_PADRAO,
                        )
                        .data,
                )
                .await;
                let _ = (hp, max_hp);
                self.avisar_vida_propria(roleid).await;
            }

            EventoDoMundo::JogadorMorreu {
                roleid,
                matador,
                pos,
            } => {
                self.enviar_ao_jogador(
                    roleid,
                    S2CGamedataSend::host_died(matador as i32, pos).data,
                )
                .await;
            }

            EventoDoMundo::JogadorReviveu {
                roleid,
                pos,
                hp,
                max_hp,
            } => {
                // `sReviveType` 0 = renascimento na cidade.
                self.enviar_ao_jogador(
                    roleid,
                    S2CGamedataSend::player_revive(roleid, 0, pos).data,
                )
                .await;
                let _ = (hp, max_hp);
                self.avisar_vida_propria(roleid).await;
            }
        }
    }

    /// Manda ao jogador o seu próprio bloco de estado (`SELF_INFO_00`, 38).
    ///
    /// # Era o comando errado
    ///
    /// Isto usava `npc_info_00` (33) com o `roleid` do jogador no lugar do id do NPC. Os
    /// dois comandos não vão para o mesmo lugar no cliente: o `EC_GameDataPrtc.cpp`
    /// entrega `NPC_INFO_00` ao `MAN_NPC` e `SELF_INFO_00` ao `MSG_HST_INFO00`. O id de um
    /// jogador procurado entre os NPCs não é encontrado, e o aviso morria ali — mesmo
    /// depois de o tamanho ter sido corrigido.
    ///
    /// Foi um erro meu, de uma etapa anterior: o caminho de saída do mundo nasceu certo na
    /// intenção e errado no comando.
    async fn avisar_vida_propria(&self, roleid: i32) {
        let dados = self.world.read().await.dados_do_proprio(roleid);
        let Some((nivel, nivel2, hp, max_hp, mp, max_mp, exp, sp)) = dados else {
            return;
        };
        self.enviar_ao_jogador(
            roleid,
            S2CGamedataSend::self_info_00(nivel, nivel2, hp, max_hp, mp, max_mp, exp, sp).data,
        )
        .await;
    }

    /// Aceita conexões de daemons de link até a escuta cair.
    pub async fn executar(self: Arc<Self>, escuta: BusListener) {
        info!("servidor de mundo escutando o barramento");
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

    /// Atende uma conexão de barramento até ela fechar.
    async fn atender(&self, mut conexao: pw_bus::transport::BusConnection) {
        let par = conexao.par().to_string();

        // Fila de saída: o mundo escreve aqui, e uma tarefa só escreve no socket. Sem
        // isso, dois pontos do mundo poderiam escrever no mesmo socket ao mesmo tempo.
        let (envio, mut fila) = mpsc::channel::<BusMessage>(256);
        let mut donos: Vec<i32> = Vec::new();

        loop {
            tokio::select! {
                entrada = conexao.receber() => {
                    match entrada {
                        Ok(Some(msg)) => {
                            if let BusMessage::EnterWorld { roleid, .. } = &msg {
                                donos.push(*roleid);
                            }
                            if let BusMessage::PlayerLogout { roleid, .. } = &msg {
                                donos.retain(|r| r != roleid);
                            }
                            self.tratar(msg, &envio).await;
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

        // A conexão caiu: os jogadores que vinham por ela não estão mais acessíveis.
        // Deixá-los registrados faria o mundo tentar responder num canal morto.
        let mut sessoes = self.sessoes.write().await;
        for roleid in donos {
            sessoes.remove(&roleid);
        }
    }

    async fn tratar(&self, msg: BusMessage, envio: &EnvioAoCliente) {
        match msg {
            BusMessage::EnterWorld {
                roleid, localsid, ..
            } => {
                self.sessoes.write().await.insert(
                    roleid,
                    Sessao {
                        localsid,
                        envio: envio.clone(),
                    },
                );
                info!("mundo: jogador {roleid} entrou (localsid {localsid})");
            }

            BusMessage::PlayerLogout { roleid, .. } => {
                self.sessoes.write().await.remove(&roleid);
                self.world.write().await.remove_player(roleid);
                info!("mundo: jogador {roleid} saiu");
            }

            BusMessage::ClientToGame { roleid, data, .. } => {
                let Some(cmd) = SubComando::ler(&data) else {
                    warn!("mundo: payload de {roleid} sem cabeçalho de subcomando");
                    return;
                };
                self.tratar_subcomando(roleid, cmd, envio).await;
            }

            BusMessage::GameToClient { roleid, .. } => {
                // Este é o sentido de saída: recebê-lo significa que alguém ligou dois
                // servidores de mundo um no outro, ou trocou os opcodes do par 74/75.
                warn!("mundo: recebi um GameToClient (74) de {roleid} — sentido invertido");
            }
        }
    }

    /// Ponto de entrada dos subcomandos do mundo 3D.
    ///
    /// É para cá que o braço `GamedataSend` do `gateway.rs` migra, comando a comando. O
    /// catálogo dos 592 comandos, com campos e deslocamentos, está em
    /// `specs/protocol/gamedata_153.json`; os decodificadores estão em
    /// [`crate::comandos`], com os deslocamentos conferidos contra aquele arquivo.
    ///
    /// Um comando ainda não migrado é registrado e ignorado — **sem** derrubar a conexão,
    /// que tiraria do ar todos os jogadores daquele link por causa de um comando só.
    async fn tratar_subcomando(&self, roleid: i32, cmd: SubComando, envio: &EnvioAoCliente) {
        match cmd.id {
            ids::PLAYER_MOVE => self.mover(roleid, &cmd.payload).await,
            ids::LOGOUT => self.sair(roleid, &cmd.payload, envio).await,
            ids::SELECT_TARGET => self.selecionar_alvo(roleid, &cmd.payload, envio).await,
            ids::UNSELECT => self.desmarcar(roleid, envio).await,
            ids::STOP_MOVE => self.parar(roleid, &cmd.payload).await,
            ids::NORMAL_ATTACK => self.atacar(roleid, &cmd.payload, envio).await,
            ids::REVIVE_VILLAGE => self.reviver(roleid).await,
            ids::GET_ITEM_INFO => self.info_do_item(roleid, &cmd.payload, envio).await,
            ids::GET_IVTR_DETAIL => self.detalhe_do_container(roleid, &cmd.payload, envio).await,
            ids::EXG_IVTR_ITEM => self.trocar_slots(roleid, &cmd.payload, ContainerType::Inventory, envio).await,
            ids::EXG_EQUIP_ITEM => self.trocar_slots(roleid, &cmd.payload, ContainerType::Equipment, envio).await,
            ids::MOVE_IVTR_ITEM => self.mover_item(roleid, &cmd.payload, envio).await,
            ids::EQUIP_ITEM => self.equipar(roleid, &cmd.payload, envio).await,
            ids::MOVE_ITEM_TO_EQUIP => self.mover_para_equipar(roleid, &cmd.payload, envio).await,
            ids::SIT_DOWN => self.postura(roleid, true, envio).await,
            ids::STAND_UP | ids::CANCEL_ACTION => self.postura(roleid, false, envio).await,
            ids::EMOTE_ACTION => self.emote(roleid, &cmd.payload, envio).await,
            ids::SEVNPC_SERVE => self.servico_de_npc(roleid, &cmd.payload, envio).await,
            ids::SEVNPC_HELLO => self.dizer_ola_ao_npc(roleid, &cmd.payload, envio).await,
            ids::TASK_NOTIFY => self.notificar_tarefa(roleid, &cmd.payload),
            ids::USE_ITEM => self.usar_item(roleid, &cmd.payload, envio).await,
            ids::TEAM_INVITE => self.convidar(roleid, &cmd.payload).await,
            ids::TEAM_AGREE_INVITE => self.aceitar_grupo(roleid, &cmd.payload).await,
            ids::TEAM_REJECT_INVITE => self.recusar_grupo(roleid).await,
            ids::TEAM_LEAVE_PARTY => self.deixar_grupo(roleid).await,
            ids::CAST_SKILL | ids::CAST_INSTANT_SKILL => {
                self.conjurar(roleid, &cmd.payload, envio).await
            }
            ids::ENTER_SANCTUARY => {
                debug!("mundo: {roleid} entrou em zona segura");
                self.responder(roleid, self.sub.enter_sanctuary(roleid).data, envio)
                    .await;
            }
            ids::GET_EXT_PROP => self.estado_proprio(roleid, envio).await,
            ids::QUERY_CASH_INFO => self.saldo(roleid, envio).await,
            ids::GET_ALL_DATA => self.todos_os_dados(roleid, &cmd.payload, envio).await,
            ids::QUERY_PLAYER_INFO_1 => self.consultar_jogadores(roleid, &cmd.payload, envio).await,
            ids::QUERY_NPC_INFO_1 => self.consultar_npcs(roleid, &cmd.payload, envio).await,
            outro => {
                debug!("mundo: subcomando {outro} de {roleid} ainda não tratado aqui");
            }
        }
    }

    /// `C2S::PLAYER_MOVE` (0) — o jogador reporta onde está.
    async fn mover(&self, roleid: i32, payload: &[u8]) {
        let Some(m) = PlayerMove::ler(payload) else {
            warn!(
                "mundo: movimento de {roleid} com {} bytes — curto até para a posição",
                payload.len()
            );
            return;
        };

        if !PlayerMove::completo(payload) {
            // Esperado no 1.2.6, cujo layout não temos como conferir. Vale registrar
            // porque é a única pista que temos de qual é o tamanho real naquela versão.
            trace!(
                "mundo: movimento de {roleid} com {} bytes (o IR do 1.5.3 diz {})",
                payload.len(),
                PlayerMove::BYTES
            );
        }

        let pos = Vector3::new(m.cur_pos.x, m.cur_pos.y, m.cur_pos.z);
        if !self.world.write().await.mover_jogador(roleid, pos) {
            trace!("mundo: movimento de {roleid}, que ainda não tem entidade neste mundo");
        }
    }

    /// `C2S::LOGOUT` (1) — o jogador pediu para sair.
    ///
    /// O mundo não fala com o cliente: quem faz isso é o daemon de link. Então a saída
    /// vira um `PlayerLogout` (69) **no barramento**, e é o link que traduz aquilo no
    /// pacote que o cliente espera. É a mesma divisão do servidor original, e é o que
    /// mantém o formato do cliente fora daqui.
    async fn sair(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let tipo = Logout::ler(payload)
            .map(|l| l.tipo())
            .unwrap_or(TipoDeSaida::SairDoJogo);

        info!("mundo: jogador {roleid} pediu saída ({tipo:?})");

        let localsid = self
            .sessoes
            .read()
            .await
            .get(&roleid)
            .map(|s| s.localsid)
            .unwrap_or(0);

        // Tira do mundo antes de avisar: se a ordem fosse a outra, o link poderia
        // derrubar a conexão e mandar o `PlayerLogout` de volta enquanto o personagem
        // ainda estivesse na simulação.
        self.world.write().await.remove_player(roleid);
        self.sessoes.write().await.remove(&roleid);

        let resultado = match tipo {
            TipoDeSaida::SelecaoDePersonagem => 1,
            _ => 0,
        };

        if envio
            .try_send(BusMessage::PlayerLogout {
                result: resultado,
                roleid,
                provider_link_id: 0,
                localsid,
            })
            .is_err()
        {
            warn!("mundo: não consegui avisar a saída de {roleid} ao link");
        }
    }

    /// `C2S::SELECT_TARGET` (2) — o jogador clicou num alvo.
    ///
    /// A diferença em relação ao `gateway.rs` não é só o arquivo: lá o HP do alvo era
    /// **1000/1000 fixo**, porque o daemon de link não sabe o estado das criaturas. Aqui
    /// o mundo sabe, e manda o HP de verdade — que é a razão de o tratamento pertencer a
    /// este lado.
    async fn selecionar_alvo(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(sel) = SelectTarget::ler(payload) else {
            warn!("mundo: select_target de {roleid} com payload curto");
            return;
        };

        let mut mundo = self.world.write().await;
        if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
            // `0` é como o cliente desmarca.
            p.target_id = (sel.id != 0).then_some(sel.id as i64);
        }

        if sel.id == 0 {
            drop(mundo);
            self.responder(roleid, S2CGamedataSend::unselect().data, envio)
                .await;
            return;
        }

        // O alvo pode ser monstro/NPC ou outro jogador, e **os dois comandos não são o
        // mesmo**: o `EC_GameDataPrtc.cpp` entrega `NPC_INFO_00` (33) ao `MAN_NPC` e
        // `PLAYER_INFO_00` (32) ao `MAN_PLAYER`. Mandar o 33 com o id de um jogador faz o
        // cliente procurá-lo entre os NPCs e não encontrar.
        //
        // O HP do monstro é `i64` na entidade e `i32` no fio (`iHP` no IR). A conversão
        // é saturante e não truncante: um valor absurdo vira `i32::MAX` em vez de virar
        // negativo por estouro, que o cliente mostraria como barra de vida vazia.
        let resposta = mundo
            .dados_do_monstro(sel.id as i64)
            .or_else(|| mundo.dados_do_npc(sel.id as i64))
            .map(|(hp, max_hp, alvo)| self.sub.npc_info_00(sel.id, hp, max_hp, alvo).data)
            .or_else(|| {
                mundo
                    .dados_do_jogador(sel.id)
                    .map(|(nivel, nivel2, hp, max_hp, mp, max_mp, alvo)| {
                        self.sub.player_info_00(
                            sel.id, nivel, nivel2, hp, max_hp, mp, max_mp, alvo,
                        )
                        .data
                    })
            });
        drop(mundo);

        self.responder(roleid, S2CGamedataSend::select_target(sel.id).data, envio)
            .await;

        if let Some(data) = resposta {
            trace!("mundo: {roleid} selecionou {}", sel.id);
            self.responder(roleid, data, envio).await;
        } else {
            // Alvo que este mundo não conhece: o cliente fica com a seleção, sem barra de
            // vida. Inventar 1000/1000 aqui seria mostrar um número falso ao jogador.
            debug!("mundo: {roleid} selecionou {}, que não está neste mundo", sel.id);
        }
    }

    /// `C2S::UNSELECT` (8) — o jogador desmarcou o alvo.
    ///
    /// Comando de cabeçalho só, sem payload.
    async fn desmarcar(&self, roleid: i32, envio: &EnvioAoCliente) {
        if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
            p.target_id = None;
        }
        self.responder(roleid, S2CGamedataSend::unselect().data, envio)
            .await;
    }

    /// `C2S::STOP_MOVE` (7) — o jogador parou.
    ///
    /// Mesma atualização do movimento: entidade e grade. O `gateway.rs` gravava no banco
    /// aqui também, um `UPDATE` por parada.
    async fn parar(&self, roleid: i32, payload: &[u8]) {
        let Some(m) = StopMove::ler(payload) else {
            warn!("mundo: stop_move de {roleid} com payload curto");
            return;
        };
        let pos = Vector3::new(m.pos.x, m.pos.y, m.pos.z);
        self.world.write().await.mover_jogador(roleid, pos);
    }

    /// `C2S::NORMAL_ATTACK` (3) — ataque básico no alvo já selecionado.
    ///
    /// # O que era, e o que passa a ser
    ///
    /// No `gateway.rs` este comando era inteiramente fictício: dano **35 fixo**, HP do
    /// alvo respondido como **965/1000 fixo**, monstro que nunca morria, e uma
    /// notificação de abate de missão disparada **a cada golpe**, com o id de criatura
    /// `13641` escrito no código — mesmo que o alvo fosse outro e mesmo que nada tivesse
    /// morrido.
    ///
    /// Aqui o dano sai do `CombatEngine` com os atributos dos dois lados, o HP é
    /// debitado de verdade, o monstro morre quando chega a zero, e o abate só é
    /// notificado **quando ele morre** — com o `template_id` real da criatura.
    async fn atacar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        // O `force_attack` ainda não muda nada; ler é o que garante que o pacote é o que
        // dizemos que é.
        if NormalAttack::ler(payload).is_none() && !payload.is_empty() {
            warn!("mundo: normal_attack de {roleid} com payload ilegível");
        }

        let mut mundo = self.world.write().await;

        // O alvo vem do `SELECT_TARGET` anterior, não do pacote — ver [`NormalAttack`].
        let Some(alvo) = mundo
            .players
            .get(&(roleid as i64))
            .and_then(|p| p.target_id)
        else {
            trace!("mundo: {roleid} atacou sem alvo selecionado");
            return;
        };

        let Some(atacante) = mundo.players.get(&(roleid as i64)).cloned() else {
            return;
        };
        let Some((monstro, _)) = mundo.monsters.get(&alvo) else {
            debug!("mundo: {roleid} atacou {alvo}, que não é um monstro deste mundo");
            return;
        };
        if monstro.is_dead {
            return;
        }

        let (dano, critico) = CombatEngine::calculate_player_to_monster_damage(&atacante, monstro);

        // Aplica e lê o resultado numa única tomada do lock, para que dois golpes
        // simultâneos não leiam o mesmo HP e matem o monstro duas vezes.
        let (hp, max_hp, morreu, template, exp, sp) = {
            let (m, ai) = mundo.monsters.get_mut(&alvo).expect("conferido acima");

            // Bater gera ameaça, e ameaça é o que faz o monstro revidar.
            //
            // Nada em produção alimentava esta tabela — só um teste de unidade. Ou seja,
            // o `MonsterAi` inteiro e o `calculate_monster_to_player_damage` eram código
            // morto: os monstros levavam dano e nunca reagiam. Esta linha é o que liga a
            // outra metade do combate.
            ai.add_threat(roleid as i64, dano);

            m.hp = (m.hp - dano).max(0);
            let morreu = m.hp == 0;
            if morreu {
                m.is_dead = true;
                m.respawn_timer_ms = 0;
            }
            (m.hp, m.max_hp, morreu, m.template_id, m.exp, m.sp)
        };

        if morreu {
            mundo.grid.remove_entity(alvo);
        }
        drop(mundo);

        // 1. O resultado do golpe.
        //
        // O `attack_flag` vai em `SEM_MARCACAO`: os bits dele não estão em nenhuma das
        // fontes que temos (ver a constante). O crítico já foi aplicado ao dano; o que se
        // perde é o *aviso visual* de crítico, e é uma dívida anotada — não um palpite.
        let _ = critico;
        self.responder(
            roleid,
            self.sub.host_attack_result(
                alvo as i32,
                saturar(dano),
                SEM_MARCACAO,
                VELOCIDADE_PADRAO,
            )
            .data,
            envio,
        )
        .await;

        // 2. A barra de vida do alvo, com o HP que sobrou de verdade.
        let alvo_do_alvo = self
            .world
            .read()
            .await
            .dados_do_monstro(alvo)
            .map(|(_, _, a)| a)
            .unwrap_or(0);
        self.responder(
            roleid,
            self.sub.npc_info_00(alvo as i32, saturar(hp), saturar(max_hp), alvo_do_alvo)
                .data,
            envio,
        )
        .await;

        if !morreu {
            return;
        }

        info!("mundo: {roleid} matou {alvo} (template {template})");

        self.responder(
            roleid,
            S2CGamedataSend::npc_died(alvo as i32, roleid).data,
            envio,
        )
        .await;
        self.responder(
            roleid,
            self.sub.receive_exp(saturar(exp), saturar(sp)).data,
            envio,
        )
        .await;

        self.notificar_abate(roleid, template, envio).await;
    }

    /// Avisa as missões ativas de que o jogador abateu uma criatura.
    ///
    /// Só é chamado **na morte**, e leva o `template_id` real do que morreu. No
    /// `gateway.rs` isto disparava a cada golpe, sempre com `13641` — então qualquer
    /// missão de caça completava atacando qualquer coisa.
    async fn notificar_abate(&self, roleid: i32, template: u32, envio: &EnvioAoCliente) {
        let repo = { self.world.read().await.char_repo.clone() };
        let missoes = repo.quest_repo().list_quests(roleid).await.unwrap_or_default();

        for q in missoes {
            if q.status != pw_core::QuestStatus::Active {
                continue;
            }
            self.responder(
                roleid,
                S2CGamedataSend::task_notify_monster_killed(q.quest_id as u16, template, 1)
                    .data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::REVIVE_VILLAGE` (4) — o jogador pediu para renascer na cidade.
    ///
    /// Não havia tratamento nenhum para este comando: quem chegava a zero de vida ficava
    /// preso, sem nada que o tirasse de lá a não ser reconectar. O aviso ao cliente sai
    /// pelo canal de eventos, como o resto do que a simulação decide.
    async fn reviver(&self, roleid: i32) {
        if self.world.write().await.reviver_jogador(roleid).is_none() {
            debug!("mundo: {roleid} pediu para reviver sem estar morto");
        }
    }

    /// `SIT_DOWN` (46), `STAND_UP` (47) e `CANCEL_ACTION` (42) — sentar e levantar.
    ///
    /// Os três só têm cabeçalho. O `CANCEL_ACTION` cai no mesmo lugar do `STAND_UP` porque
    /// cancelar uma ação em curso é, para o corpo do personagem, ficar de pé.
    async fn postura(&self, roleid: i32, sentado: bool, envio: &EnvioAoCliente) {
        let cmd = if sentado {
            S2CGamedataSend::object_sit_down(roleid)
        } else {
            S2CGamedataSend::object_stand_up(roleid)
        };
        self.responder(roleid, cmd.data, envio).await;
    }

    /// `EMOTE_ACTION` (48) — o jogador executou um gesto.
    async fn emote(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(e) = EmoteAction::ler(payload) else {
            warn!("mundo: emote de {roleid} com payload curto");
            return;
        };
        self.responder(
            roleid,
            S2CGamedataSend::object_do_emote(roleid, e.action).data,
            envio,
        )
        .await;
    }

    /// `C2S::TEAM_INVITE` (27) — convidar alguém para o grupo.
    ///
    /// O convite vai para **o convidado**. No `gateway.rs` ele era mandado de volta a
    /// quem convidou, então o convidado nunca ficava sabendo de nada — e o grupo, que não
    /// existia em lugar nenhum, jamais se formava.
    async fn convidar(&self, roleid: i32, payload: &[u8]) {
        let Some(alvo) = SelectTarget::ler(payload).map(|s| s.id) else {
            warn!("mundo: team_invite de {roleid} com payload curto");
            return;
        };

        if !self.world.write().await.convidar_para_grupo(roleid, alvo) {
            debug!("mundo: convite de {roleid} para {alvo} recusado (já tem grupo?)");
            return;
        }

        info!("mundo: {roleid} convidou {alvo} para o grupo");
        // O convidado é quem precisa ver a caixa de convite. O `seq` identifica este
        // convite; como só guardamos um convite pendente por jogador, o id de quem
        // convidou já o identifica sem ambiguidade.
        if !self
            .enviar_ao_jogador(
                alvo,
                S2CGamedataSend::team_leader_invite(roleid, roleid, PICK_FLAG_PADRAO).data,
            )
            .await
        {
            debug!("mundo: {alvo} não está neste servidor de mundo");
        }
    }

    /// `C2S::TEAM_AGREE_INVITE` (28) — aceitar o convite.
    ///
    /// Todos os membros recebem a lista atualizada, com os valores **reais** de cada um.
    /// O `gateway.rs` mandava a lista só a quem aceitou, com vida e posição escritas no
    /// código.
    async fn aceitar_grupo(&self, roleid: i32, payload: &[u8]) {
        let Some(lider) = SelectTarget::ler(payload).map(|s| s.id) else {
            warn!("mundo: team_agree de {roleid} com payload curto");
            return;
        };

        let (membros, dados) = {
            let mut mundo = self.world.write().await;
            let Some(membros) = mundo.aceitar_convite(roleid, lider) else {
                // Sem convite pendente daquele jogador. Recusar aqui é o que impede
                // alguém de entrar em qualquer grupo mandando o comando com um id alheio.
                debug!("mundo: {roleid} disse aceitar convite de {lider}, que não existe");
                return;
            };
            let dados = mundo.dados_dos_membros(&membros);
            (membros, dados)
        };

        info!(
            "mundo: {roleid} entrou no grupo de {lider} ({} membros)",
            membros.len()
        );
        for m in &membros {
            self.enviar_ao_jogador(
                *m,
                S2CGamedataSend::team_join_party(lider, PICK_FLAG_PADRAO).data,
            )
            .await;
            self.enviar_ao_jogador(*m, S2CGamedataSend::team_member_data(lider, &dados).data)
                .await;
        }
    }

    /// `C2S::TEAM_REJECT_INVITE` (29) — recusar o convite.
    async fn recusar_grupo(&self, roleid: i32) {
        if let Some(quem_convidou) = self.world.write().await.recusar_convite(roleid) {
            debug!("mundo: {roleid} recusou o convite de {quem_convidou}");
        }
    }

    /// `C2S::TEAM_LEAVE_PARTY` (30) — sair do grupo.
    ///
    /// # Dois comandos diferentes, para dois destinatários diferentes
    ///
    /// Quem sai recebe `TEAM_LEAVE_PARTY` (61) — "seu grupo acabou", que é o comando que
    /// fecha a interface de grupo. Quem fica recebe `TEAM_MEMBER_LEAVE` (60) — "o fulano
    /// saiu" —, que leva o id de quem saiu e por isso é o único que permite tirar a
    /// pessoa certa da lista.
    ///
    /// Antes os dois lados recebiam o 61 com o id de quem saiu no lugar do líder: para
    /// quem ficava, a mensagem lida era "seu grupo acabou".
    async fn deixar_grupo(&self, roleid: i32) {
        let (lider, restantes, dados) = {
            let mut mundo = self.world.write().await;
            let Some((lider, restantes)) = mundo.sair_do_grupo(roleid) else {
                debug!("mundo: {roleid} pediu para sair de um grupo que não tem");
                return;
            };
            let dados = mundo.dados_dos_membros(&restantes);
            (lider, restantes, dados)
        };

        info!("mundo: {roleid} saiu do grupo; ficaram {}", restantes.len());
        // O próprio, para fechar a interface de grupo.
        self.enviar_ao_jogador(
            roleid,
            S2CGamedataSend::team_leave_party(lider, SAIDA_VOLUNTARIA).data,
        )
        .await;
        // E os que ficaram, com o id de quem saiu e a lista nova.
        for m in &restantes {
            self.enviar_ao_jogador(
                *m,
                S2CGamedataSend::team_member_leave(lider, roleid, SAIDA_VOLUNTARIA).data,
            )
            .await;
            self.enviar_ao_jogador(*m, S2CGamedataSend::team_member_data(lider, &dados).data)
                .await;
        }
    }

    /// `C2S::USE_ITEM` (40) — usar um item da bolsa.
    ///
    /// # Poção agora cura de verdade
    ///
    /// O `gateway.rs` reconhecia poção comparando o id com `1796` e `1801`, escritos no
    /// código, e respondia HP/MP **120/280 fixos** para qualquer personagem de qualquer
    /// nível — sem alterar nada no mundo. Aqui o quanto vem do `elements.data`
    /// (`MedicineTemplate`), que já estava carregado e nunca era lido, e a cura é aplicada
    /// à entidade.
    async fn usar_item(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(u) = UseItem::ler(payload) else {
            warn!("mundo: use_item de {roleid} com payload curto");
            return;
        };

        let ct = ContainerType::from_i16(u.onde as i16);
        let itens = self.itens().await;

        // Confere que o item está mesmo ali antes de consumir: sem isso o cliente escolhe
        // o que quer usar, inclusive o que não tem.
        let Ok(Some(guardado)) = itens.get_item_by_slot(roleid, ct, u.slot).await else {
            debug!("mundo: {roleid} tentou usar o slot {} , que está vazio", u.slot);
            return;
        };
        if guardado.item_id != u.item_id as u32 {
            warn!(
                "mundo: {roleid} disse usar o item {} do slot {}, onde está o {}",
                u.item_id, u.slot, guardado.item_id
            );
            return;
        }

        let quantos = u.quantos.max(1) as u32;
        if itens
            .consume_item(roleid, ct, u.slot, quantos)
            .await
            .is_err()
        {
            debug!("mundo: {roleid} não tinha {quantos} do item {}", u.item_id);
            return;
        }

        self.responder(
            roleid,
            S2CGamedataSend::host_use_item(u.onde, u.slot as u8, u.item_id, quantos as u16).data,
            envio,
        )
        .await;
        self.responder(
            roleid,
            S2CGamedataSend::unfreeze_ivtr_slot(u.onde, u.slot).data,
            envio,
        )
        .await;

        // Se for remédio, cura pelo que o `elements.data` diz.
        let curou = {
            let mut mundo = self.world.write().await;
            match mundo.quanto_o_remedio_restaura(u.item_id as u32) {
                Some((hp, mp)) => mundo.curar_jogador(roleid, hp * quantos as i32, mp * quantos as i32),
                None => None,
            }
        };

        if let Some((hp, max_hp, mp, max_mp)) = curou {
            let (nivel, exp, sp) = {
                let mundo = self.world.read().await;
                mundo
                    .players
                    .get(&(roleid as i64))
                    .map(|p| (p.level, p.exp, p.sp))
                    .unwrap_or((1, 0, 0))
            };
            info!("mundo: {roleid} usou o item {} e ficou com {hp}/{max_hp}", u.item_id);
            self.responder(
                roleid,
                S2CGamedataSend::self_info_00(
                    nivel as i16,
                    0,
                    hp,
                    max_hp,
                    mp,
                    max_mp,
                    exp as i32,
                    sp as i32,
                )
                .data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::CAST_SKILL` (41) e `CAST_INSTANT_SKILL` (80) — conjurar habilidade.
    ///
    /// # O que muda em relação ao `gateway.rs`
    ///
    /// Lá o alvo era lido em `data[7..11]`, que começa no `target_count` e engole três
    /// bytes do primeiro alvo — a lista começa no deslocamento 8. E o dano era **150
    /// fixo**, mandado por uma tarefa que dormia um segundo e respondia sem olhar para
    /// nada.
    ///
    /// Aqui o alvo vem da lista (ou da seleção corrente, se o cliente não mandar nenhum),
    /// e o dano sai do `CombatEngine` com os atributos dos dois lados.
    ///
    /// TODO: falta o coeficiente da habilidade. O `elements.data` que carregamos não traz
    /// a tabela de skills, então por ora uma habilidade causa o mesmo que um golpe básico
    /// — o que é honesto enquanto o número certo não estiver disponível, e melhor do que
    /// os 150 fixos.
    async fn conjurar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(c) = CastSkill::ler(payload) else {
            warn!("mundo: cast_skill de {roleid} com payload curto");
            return;
        };

        let mundo = self.world.write().await;
        let alvo = c
            .alvos
            .first()
            .map(|a| *a as i64)
            .or_else(|| mundo.players.get(&(roleid as i64)).and_then(|p| p.target_id));

        let Some(alvo) = alvo else {
            trace!("mundo: {roleid} conjurou {} sem alvo", c.skill_id);
            return;
        };

        // A animação sai antes do resultado — é o que o cliente espera ver.
        drop(mundo);
        self.responder(
            roleid,
            S2CGamedataSend::object_cast_skill(roleid, alvo as i32, c.skill_id, 1000, 1).data,
            envio,
        )
        .await;
        self.responder(roleid, S2CGamedataSend::skill_perform().data, envio)
            .await;

        let mut mundo = self.world.write().await;
        let Some(atacante) = mundo.players.get(&(roleid as i64)).cloned() else {
            return;
        };
        let Some((monstro, _)) = mundo.monsters.get(&alvo) else {
            debug!("mundo: {roleid} conjurou em {alvo}, que não é um monstro deste mundo");
            return;
        };
        if monstro.is_dead {
            return;
        }

        let (dano, _) = CombatEngine::calculate_player_to_monster_damage(&atacante, monstro);
        let (hp, max_hp, morreu, template, exp, sp) = {
            let (m, ai) = mundo.monsters.get_mut(&alvo).expect("conferido acima");
            ai.add_threat(roleid as i64, dano);
            m.hp = (m.hp - dano).max(0);
            let morreu = m.hp == 0;
            if morreu {
                m.is_dead = true;
                m.respawn_timer_ms = 0;
            }
            (m.hp, m.max_hp, morreu, m.template_id, m.exp, m.sp)
        };
        if morreu {
            mundo.grid.remove_entity(alvo);
        }
        drop(mundo);

        self.responder(
            roleid,
            self.sub.self_skill_attack_result(
                alvo as i32,
                c.skill_id,
                saturar(dano),
                SEM_MARCACAO,
                VELOCIDADE_PADRAO,
                SECAO_UNICA,
            )
                .data,
            envio,
        )
        .await;
        let alvo_do_alvo = self
            .world
            .read()
            .await
            .dados_do_monstro(alvo)
            .map(|(_, _, a)| a)
            .unwrap_or(0);
        self.responder(
            roleid,
            self.sub.npc_info_00(alvo as i32, saturar(hp), saturar(max_hp), alvo_do_alvo)
                .data,
            envio,
        )
        .await;

        if morreu {
            info!("mundo: {roleid} matou {alvo} com a habilidade {}", c.skill_id);
            self.responder(
                roleid,
                S2CGamedataSend::npc_died(alvo as i32, roleid).data,
                envio,
            )
            .await;
            self.responder(
                roleid,
                self.sub.receive_exp(saturar(exp), saturar(sp)).data,
                envio,
            )
            .await;
            self.notificar_abate(roleid, template, envio).await;
        }
    }

    /// `C2S::SEVNPC_HELLO` (35) — o jogador abriu diálogo com um NPC.
    ///
    /// No servidor original isto passa por uma sessão (`session_say_hello`) que checa
    /// facção e distância antes de responder `NPC_GREETING` (70) — ver
    /// `cgame/gs/servicenpc.cpp` e `cgame/gs/player.cpp:SayHelloToNPC`. Aqui a versão é
    /// mínima: só confirma que o alvo é um NPC deste mundo e devolve o `NPC_GREETING`, sem
    /// checar facção nem distância ainda — anotado como TODO, não fingido como pronto.
    async fn dizer_ola_ao_npc(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(pedido) = SevnpcHello::ler(payload) else {
            warn!("mundo: sevnpc_hello de {roleid} com payload curto");
            return;
        };

        let mundo = self.world.read().await;
        let existe = mundo.dados_do_npc(pedido.target as i64).is_some();
        drop(mundo);

        if !existe {
            debug!(
                "mundo: {roleid} disse olá para {}, que não é um NPC deste mundo",
                pedido.target
            );
            return;
        }

        self.responder(
            roleid,
            S2CGamedataSend::npc_greeting(pedido.target).data,
            envio,
        )
        .await;
    }

    /// `C2S::TASK_NOTIFY` (49) — o cliente reporta algo ao sistema de missões.
    ///
    /// Só decodifica e loga por enquanto — não há motor de missões no `pw-gs` ainda (ver
    /// contexto A do roadmap salvo em memória). Sem isto o comando era descartado em
    /// silêncio; agora pelo menos fica visível qual `reason`/`task` o cliente mandou,
    /// para quando o motor existir.
    fn notificar_tarefa(&self, roleid: i32, payload: &[u8]) {
        let Some(tn) = TaskNotify::ler(payload) else {
            warn!("mundo: task_notify de {roleid} com payload curto ou size inconsistente");
            return;
        };
        debug!(
            "mundo: {roleid} mandou task_notify (reason={:?}, task={:?}, {} bytes) — motor de missões ainda não existe",
            tn.reason,
            tn.task,
            tn.buf.len()
        );
    }

    /// `C2S::SEVNPC_SERVE` (37) — tudo que se pede a um NPC.
    ///
    /// O `service_type` separa treze serviços; o corpo depois dele muda de forma conforme
    /// o serviço. Os layouts e a razão da inversão de loja estão em [`crate::npc`].
    async fn servico_de_npc(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(pedido) = PedidoAoNpc::ler(payload) else {
            warn!("mundo: sevnpc_serve de {roleid} com payload curto");
            return;
        };
        if !pedido.tamanho_confere() {
            warn!(
                "mundo: sevnpc_serve de {roleid} declara {} bytes de conteúdo e trouxe {}",
                pedido.len,
                pedido.conteudo.len()
            );
            return;
        }

        let c = pedido.conteudo;
        match pedido.service_type {
            // O NPC **vende**: o jogador está comprando.
            servico::NPC_VENDE => self.jogador_compra(roleid, c, envio).await,
            // O NPC **compra**: o jogador está vendendo.
            servico::NPC_COMPRA => self.jogador_vende(roleid, c, envio).await,

            servico::REPARAR => {
                // TODO: o custo é fixo enquanto a durabilidade dos itens não for lida.
                const CUSTO: i64 = 150;
                let repo = self.repo().await;
                if repo.deduct_money(roleid, CUSTO).await.unwrap_or(false) {
                    self.responder(roleid, S2CGamedataSend::repair_all(CUSTO as i32).data, envio)
                        .await;
                } else {
                    debug!("mundo: {roleid} não tem os {CUSTO} do reparo");
                }
            }

            servico::CURAR => {
                // Cura de verdade, com os valores do jogador — e não os fixos que o
                // `gateway.rs` mandava (120/280 para qualquer personagem, de qualquer
                // nível).
                let mut mundo = self.world.write().await;
                let Some(p) = mundo.players.get_mut(&(roleid as i64)) else {
                    return;
                };
                p.hp = p.max_hp;
                p.mp = p.max_mp;
                let (nivel, hp, max_hp, mp, max_mp, exp, sp) =
                    (p.level, p.hp, p.max_hp, p.mp, p.max_mp, p.exp, p.sp);
                drop(mundo);

                self.responder(
                    roleid,
                    S2CGamedataSend::self_info_00(
                        nivel as i16,
                        0,
                        hp,
                        max_hp,
                        mp,
                        max_mp,
                        exp as i32,
                        sp as i32,
                    )
                    .data,
                    envio,
                )
                .await;
            }

            servico::ACEITAR_MISSAO | servico::ENTREGAR_MISSAO | servico::ITEM_DE_MISSAO => {
                self.missao(roleid, pedido.service_type, c, envio).await
            }

            outro => {
                debug!("mundo: {roleid} pediu o serviço de NPC {outro}, ainda não tratado");
            }
        }
    }

    /// `GP_NPCSEV_SELL` — o NPC vende, o jogador **compra**.
    ///
    /// O `gateway.rs` fazia o contrário aqui: apagava um item do jogador e lhe dava
    /// dinheiro. Ver [`crate::npc`] para a confirmação no fonte do cliente.
    async fn jogador_compra(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let itens = npc::itens_comprados(conteudo);
        if itens.is_empty() {
            debug!("mundo: {roleid} mandou uma compra sem itens");
            return;
        }

        // TODO: o preço tem que sair do `elements.data`, e não ser fixo. Enquanto isso, o
        // custo é por unidade e a compra é recusada se o jogador não tiver como pagar —
        // que já é melhor do que entregar mercadoria de graça.
        const PRECO_UNITARIO: i64 = 100;
        let repo = self.repo().await;
        let itens_repo = self.itens().await;

        for i in itens {
            let total = PRECO_UNITARIO * i64::from(i.count.max(1));
            if !repo.deduct_money(roleid, total).await.unwrap_or(false) {
                debug!("mundo: {roleid} não tem {total} para comprar o item {}", i.tid);
                continue;
            }

            let slot = i.index as u16;
            let _ = itens_repo
                .upsert_item(&pw_core::ItemRecord {
                    id: None,
                    character_id: roleid,
                    container_type: ContainerType::Inventory,
                    slot,
                    item_id: i.tid as u32,
                    count: i.count.max(1),
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    octets: vec![],
                    custom_attributes: serde_json::json!({}),
                })
                .await;

            info!("mundo: {roleid} comprou o item {} por {total}", i.tid);
            self.mandar_info(roleid, 0, slot as u8, envio).await;
            self.responder(
                roleid,
                S2CGamedataSend::unfreeze_ivtr_slot(0, slot).data,
                envio,
            )
            .await;
        }
    }

    /// `GP_NPCSEV_BUY` — o NPC compra, o jogador **vende**.
    async fn jogador_vende(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let itens = npc::itens_vendidos(conteudo);
        if itens.is_empty() {
            debug!("mundo: {roleid} mandou uma venda sem itens");
            return;
        }

        // TODO: mesmo caso da compra — o valor tem que vir do `elements.data`. O `price`
        // que o cliente manda é **ignorado** de propósito: aceitá-lo deixaria o jogador
        // escolher quanto ganha.
        const VALOR_UNITARIO: i64 = 50;
        let repo = self.repo().await;
        let itens_repo = self.itens().await;

        for i in itens {
            let slot = i.index as u16;
            // Confere que o item existe e é daquele slot antes de pagar — senão o jogador
            // vende slots vazios.
            let Ok(Some(guardado)) = itens_repo
                .get_item_by_slot(roleid, ContainerType::Inventory, slot)
                .await
            else {
                debug!("mundo: {roleid} tentou vender o slot {slot}, que está vazio");
                continue;
            };
            if guardado.item_id != i.tid as u32 {
                warn!(
                    "mundo: {roleid} disse vender o item {} do slot {slot}, onde está o {}",
                    i.tid, guardado.item_id
                );
                continue;
            }

            let _ = itens_repo
                .delete_item_by_slot(roleid, ContainerType::Inventory, slot)
                .await;
            let ganho = VALOR_UNITARIO * i64::from(guardado.count.max(1));
            let _ = repo.add_money(roleid, ganho).await;

            info!("mundo: {roleid} vendeu o item {} por {ganho}", guardado.item_id);
            self.responder(
                roleid,
                S2CGamedataSend::unfreeze_ivtr_slot(0, slot).data,
                envio,
            )
            .await;
        }
    }

    /// Aceitar, entregar ou pedir item de missão.
    ///
    /// O `idTask` é o primeiro `int` do conteúdo nos três — confirmado em
    /// `c2s_SendCmdNPCSevAcceptTask` e `c2s_SendCmdNPCSevReturnTask`.
    async fn missao(&self, roleid: i32, tipo: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let Some(id_missao) = npc::id_da_missao(conteudo) else {
            warn!("mundo: pedido de missão de {roleid} sem id");
            return;
        };
        let agora = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        let repo = self.repo().await;

        match tipo {
            servico::ACEITAR_MISSAO => {
                let _ = repo
                    .quest_repo()
                    .save_quest(
                        roleid,
                        id_missao as u32,
                        pw_core::QuestStatus::Active,
                        &[0, 0, 0],
                        None,
                    )
                    .await;
                info!("mundo: {roleid} aceitou a missão {id_missao}");
                self.responder(
                    roleid,
                    S2CGamedataSend::task_notify_new(id_missao as u16, agora).data,
                    envio,
                )
                .await;
            }

            servico::ENTREGAR_MISSAO => {
                // TODO: a recompensa é fixa enquanto o `tasks.data` não for consultado.
                const EXP: i64 = 1500;
                const SP: i64 = 320;
                const MOEDAS: i64 = 500;

                let _ = repo
                    .quest_repo()
                    .save_quest(
                        roleid,
                        id_missao as u32,
                        pw_core::QuestStatus::Completed,
                        &[0, 0, 0],
                        None,
                    )
                    .await;
                let _ = repo.add_exp_sp(roleid, EXP, SP).await;
                let _ = repo.add_money(roleid, MOEDAS).await;

                info!("mundo: {roleid} entregou a missão {id_missao}");
                self.responder(
                    roleid,
                    S2CGamedataSend::task_notify_complete(id_missao as u16, agora).data,
                    envio,
                )
                .await;
                self.responder(
                    roleid,
                    self.sub.receive_exp(EXP as i32, SP as i32).data,
                    envio,
                )
                .await;
            }

            _ => {
                debug!("mundo: {roleid} pediu o item da missão {id_missao} (ainda sem tratamento)");
            }
        }
    }

    /// `C2S::GET_EXT_PROP` (21) — o cliente pede o próprio bloco de estado.
    ///
    /// O `gateway.rs` respondia `self_info_00(1, sec_level, 120, 120, 280, 280, 0, 0)`:
    /// nível 1, vida 120, mana 280, experiência zero, para **qualquer** personagem. Era a
    /// terceira aparição do mesmo `120/280` escrito no código (itens 37 e 45) — e a razão
    /// é sempre a mesma: o daemon de link não tem a simulação, então não tinha de onde
    /// tirar o número certo.
    ///
    /// Manda também o dinheiro, que era `50000` fixo para todo mundo.
    async fn estado_proprio(&self, roleid: i32, envio: &EnvioAoCliente) {
        let (dados, dinheiro) = {
            let mundo = self.world.read().await;
            (mundo.dados_do_proprio(roleid), mundo.dinheiro(roleid))
        };
        let Some((nivel, nivel2, hp, max_hp, mp, max_mp, exp, sp)) = dados else {
            debug!("mundo: {roleid} pediu o próprio estado sem estar neste mundo");
            return;
        };

        self.responder(
            roleid,
            S2CGamedataSend::self_info_00(nivel, nivel2, hp, max_hp, mp, max_mp, exp, sp).data,
            envio,
        )
        .await;
        if let Some(d) = dinheiro {
            self.responder(roleid, S2CGamedataSend::player_cash(d).data, envio)
                .await;
        }
    }

    /// `C2S::QUERY_CASH_INFO` (110) — o cliente pergunta o saldo.
    ///
    /// O `gateway.rs` respondia `50000` escrito no código, para qualquer jogador, sempre.
    async fn saldo(&self, roleid: i32, envio: &EnvioAoCliente) {
        let Some(d) = self.world.read().await.dinheiro(roleid) else {
            debug!("mundo: {roleid} pediu o saldo sem estar neste mundo");
            return;
        };
        self.responder(roleid, S2CGamedataSend::player_cash(d).data, envio)
            .await;
    }

    /// `C2S::QUERY_PLAYER_INFO_1` (67) — barra de vida dos outros jogadores.
    ///
    /// O `gateway.rs` **lia a contagem, escrevia uma linha de log e devolvia**. Nenhum
    /// outro jogador tinha barra de vida na tela, e nada indicava por quê: o comando
    /// chegava, era "tratado", e não produzia resposta nenhuma.
    ///
    /// Jogador que não está neste mundo é omitido. Responder zeros por ele desenharia
    /// barra vazia em quem está vivo do outro lado do mapa.
    async fn consultar_jogadores(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(consulta) = ConsultaDeIds::ler(payload) else {
            warn!("mundo: query_player_info_1 de {roleid} com payload curto");
            return;
        };

        let respostas: Vec<_> = {
            let mundo = self.world.read().await;
            consulta
                .ids
                .iter()
                .filter_map(|id| mundo.dados_do_jogador(*id).map(|d| (*id, d)))
                .collect()
        };

        for (id, (nivel, nivel2, hp, max_hp, mp, max_mp, alvo)) in respostas {
            self.responder(
                roleid,
                self.sub.player_info_00(id, nivel, nivel2, hp, max_hp, mp, max_mp, alvo)
                    .data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::QUERY_NPC_INFO_1` (68) — barra de vida de monstros e NPCs.
    ///
    /// # O que desfazia o combate
    ///
    /// O `gateway.rs` respondia `npc_info_00(nid, 1000, 1000)` — **vida cheia fixa** —
    /// para qualquer id, porque o daemon de link não sabe o estado das criaturas. É a
    /// mesma razão que já tinha feito o `SELECT_TARGET` mudar de lado (item 2), mas com
    /// uma consequência pior: esta consulta é **periódica**. O golpe tirava vida de
    /// verdade no mundo, o `SELECT_TARGET` mostrava o valor certo, e a consulta seguinte
    /// redesenhava a barra cheia.
    async fn consultar_npcs(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(consulta) = ConsultaDeIds::ler(payload) else {
            warn!("mundo: query_npc_info_1 de {roleid} com payload curto");
            return;
        };

        let respostas: Vec<_> = {
            let mundo = self.world.read().await;
            consulta
                .ids
                .iter()
                .filter_map(|id| mundo.dados_do_monstro(*id as i64).map(|d| (*id, d)))
                .collect()
        };

        for (id, (hp, max_hp, alvo)) in respostas {
            self.responder(
                roleid,
                self.sub.npc_info_00(id, hp, max_hp, alvo).data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::GET_ALL_DATA` (39) — a carga inicial ao entrar no mundo.
    ///
    /// Bolsa, equipamento, dinheiro e missões, **conforme os três sinalizadores** que o
    /// comando traz. O `gateway.rs` não lia nenhum dos três e mandava sempre tudo.
    ///
    /// O dinheiro era `50000` fixo. Agora sai do personagem.
    ///
    /// O `TASK_DATA` (105) fecha a sequência: é o marcador que dispara o
    /// `LoadConfigData` no cliente (`EC_HostMsg.cpp:3841`), então ele vai **sempre**,
    /// mesmo quando o cliente não pediu missões — sem ele o cliente fica esperando.
    async fn todos_os_dados(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let pedido = GetAllData::ler(payload).unwrap_or(GetAllData {
            // Um payload curto vem de cliente de outra versão. Mandar tudo é o
            // comportamento antigo, e é o seguro: falta de dado trava a entrada no mundo.
            detalhe_bolsa: 1,
            detalhe_equipamento: 1,
            detalhe_missoes: 1,
        });

        let itens = self.itens().await;

        if pedido.detalhe_bolsa != 0 {
            let bolsa = itens
                .list_by_container(roleid, ContainerType::Inventory)
                .await
                .unwrap_or_default();
            self.responder(
                roleid,
                S2CGamedataSend::own_ivtr_from_items(0, 32, &bolsa).data,
                envio,
            )
            .await;
            for item in &bolsa {
                self.responder(roleid, Self::info_de(0, item), envio).await;
            }
        }

        if pedido.detalhe_equipamento != 0 {
            let equipado = itens
                .list_by_container(roleid, ContainerType::Equipment)
                .await
                .unwrap_or_default();
            self.responder(
                roleid,
                S2CGamedataSend::own_ivtr_from_items(1, 32, &equipado).data,
                envio,
            )
            .await;
            for item in &equipado {
                self.responder(roleid, Self::info_de(1, item), envio).await;
            }
        }

        if let Some(d) = self.world.read().await.dinheiro(roleid) {
            self.responder(roleid, S2CGamedataSend::player_cash(d).data, envio)
                .await;
        }

        if pedido.detalhe_missoes != 0 {
            let repo = self.repo().await;
            let agora = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as u32)
                .unwrap_or(0);
            for q in repo.quest_repo().list_quests(roleid).await.unwrap_or_default() {
                if q.status == pw_core::QuestStatus::Active {
                    self.responder(
                        roleid,
                        S2CGamedataSend::task_notify_new(q.quest_id as u16, agora).data,
                        envio,
                    )
                    .await;
                }
            }
        }

        // Sempre, mesmo sem missões: é o marcador de fim da carga.
        self.responder(roleid, S2CGamedataSend::task_data().data, envio)
            .await;
    }

    /// O `item_info` de um item já carregado, para não repetir a conversão em dois lugares.
    fn info_de(onde: u8, item: &pw_core::ItemRecord) -> Vec<u8> {
        S2CGamedataSend::item_info(
            onde,
            item.slot as u8,
            item.item_id as i32,
            item.durability as i32 * 100,
            item.max_durability as i32 * 100,
            item.count,
            &item.octets,
        )
        .data
    }

    /// O repositório de personagens deste mundo.
    async fn repo(&self) -> pw_storage::CharacterRepository {
        self.world.read().await.char_repo.clone()
    }

    /// O repositório de itens deste mundo.
    async fn itens(&self) -> pw_storage::ItemRepository {
        self.world.read().await.char_repo.item_repo().clone()
    }

    /// Manda o `item_info` de um item, se ele existir naquele slot.
    async fn mandar_info(
        &self,
        roleid: i32,
        onde: u8,
        slot: u8,
        envio: &EnvioAoCliente,
    ) {
        let itens = self.itens().await;
        let ct = ContainerType::from_i16(onde as i16);
        if let Ok(Some(i)) = itens.get_item_by_slot(roleid, ct, slot as u16).await {
            self.responder(
                roleid,
                S2CGamedataSend::item_info(
                    onde,
                    slot,
                    i.item_id as i32,
                    i.durability as i32 * 100,
                    i.max_durability as i32 * 100,
                    i.count,
                    &i.octets,
                )
                .data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::GET_ITEM_INFO` (9) — o cliente pediu os detalhes de um slot.
    async fn info_do_item(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(p) = ParDeSlots::ler(payload) else {
            warn!("mundo: get_item_info de {roleid} com payload curto");
            return;
        };
        self.mandar_info(roleid, p.a, p.b, envio).await;
    }

    /// `C2S::GET_IVTR_DETAIL` (11) — o cliente pediu um contêiner inteiro.
    async fn detalhe_do_container(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let onde = GetIvtrDetail::ler(payload).map(|d| d.onde).unwrap_or(0);
        let itens = self.itens().await;
        let lista = itens
            .list_by_container(roleid, ContainerType::from_i16(onde as i16))
            .await
            .unwrap_or_default();
        self.responder(
            roleid,
            S2CGamedataSend::own_ivtr_from_items(onde, 32, &lista).data,
            envio,
        )
        .await;
    }

    /// `C2S::EXG_IVTR_ITEM` (12) e `EXG_EQUIP_ITEM` (16) — trocar dois slots de lugar.
    ///
    /// Os dois comandos têm o mesmo layout e a mesma lógica; muda só o contêiner. Escrever
    /// duas vezes seria convidar as duas cópias a divergirem.
    async fn trocar_slots(
        &self,
        roleid: i32,
        payload: &[u8],
        onde: ContainerType,
        envio: &EnvioAoCliente,
    ) {
        let Some(p) = ParDeSlots::ler(payload) else {
            warn!("mundo: troca de slots de {roleid} com payload curto");
            return;
        };

        let itens = self.itens().await;
        if let Err(e) = itens.swap_slots(roleid, onde, p.a as u16, p.b as u16).await {
            warn!("mundo: falha ao trocar slots de {roleid}: {e:?}");
            return;
        }

        let n = onde.to_i16() as u8;
        let confirmacao = match onde {
            ContainerType::Equipment => S2CGamedataSend::exg_equip_item(p.a, p.b),
            _ => S2CGamedataSend::exg_ivtr_item(p.a, p.b),
        };
        self.responder(roleid, confirmacao.data, envio).await;
        // Destrava os dois slots na interface — sem isso eles ficam acinzentados.
        for s in [p.a, p.b] {
            self.responder(
                roleid,
                S2CGamedataSend::unfreeze_ivtr_slot(n, s as u16).data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::MOVE_IVTR_ITEM` (13) — mover item dentro da bolsa.
    ///
    /// # Uma dívida que fica registrada
    ///
    /// O comando traz um `amount`: mover 5 de uma pilha de 20 deveria **dividir** a pilha.
    /// O tratamento herdado do `gateway.rs` chama `swap_slots`, que troca as pilhas
    /// inteiras — então mover 5 move os 20. Manter o comportamento é deliberado: dividir
    /// pilha exige uma operação nova no repositório, e trocá-la aqui sem isso seria
    /// inventar uma semântica pela metade. O campo já é decodificado e registrado, para
    /// que a correção tenha por onde começar.
    async fn mover_item(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(m) = MoveIvtrItem::ler(payload) else {
            warn!("mundo: move_ivtr_item de {roleid} com payload curto");
            return;
        };
        if m.amount > 1 {
            debug!(
                "mundo: {roleid} pediu mover {} de uma pilha; a pilha inteira vai junto \
                 (divisão de pilha ainda não implementada)",
                m.amount
            );
        }

        let itens = self.itens().await;
        if let Err(e) = itens
            .swap_slots(roleid, ContainerType::Inventory, m.src as u16, m.dest as u16)
            .await
        {
            warn!("mundo: falha ao mover item de {roleid}: {e:?}");
            return;
        }

        self.responder(
            roleid,
            S2CGamedataSend::move_ivtr_item(m.src, m.dest, m.amount).data,
            envio,
        )
        .await;
        for s in [m.src, m.dest] {
            self.responder(
                roleid,
                S2CGamedataSend::unfreeze_ivtr_slot(0, s as u16).data,
                envio,
            )
            .await;
        }
    }

    /// `C2S::EQUIP_ITEM` (17) — equipar ou desequipar.
    ///
    /// Bidirecional: o mesmo comando tira da bolsa para o corpo e o contrário, porque
    /// `move_between_containers` troca os dois lados. Depois da troca o cliente recebe o
    /// que ficou em cada slot.
    async fn equipar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(p) = ParDeSlots::ler(payload) else {
            warn!("mundo: equip_item de {roleid} com payload curto");
            return;
        };
        let (idx_bolsa, idx_corpo) = (p.a, p.b);

        let itens = self.itens().await;
        if let Err(e) = itens
            .move_between_containers(
                roleid,
                ContainerType::Inventory,
                idx_bolsa as u16,
                ContainerType::Equipment,
                idx_corpo as u16,
            )
            .await
        {
            warn!("mundo: falha ao equipar para {roleid}: {e:?}");
            return;
        }

        let na_bolsa = itens
            .get_item_by_slot(roleid, ContainerType::Inventory, idx_bolsa as u16)
            .await
            .unwrap_or(None);
        let no_corpo = itens
            .get_item_by_slot(roleid, ContainerType::Equipment, idx_corpo as u16)
            .await
            .unwrap_or(None);

        self.responder(
            roleid,
            self.sub.equip_item(
                idx_bolsa,
                idx_corpo,
                u32::from(na_bolsa.is_some()),
                u32::from(no_corpo.is_some()),
            )
            .data,
            envio,
        )
        .await;

        if na_bolsa.is_some() {
            self.mandar_info(roleid, 0, idx_bolsa, envio).await;
        }
        if no_corpo.is_some() {
            self.mandar_info(roleid, 1, idx_corpo, envio).await;
        }

        self.responder(
            roleid,
            S2CGamedataSend::unfreeze_ivtr_slot(0, idx_bolsa as u16).data,
            envio,
        )
        .await;
        self.responder(
            roleid,
            S2CGamedataSend::unfreeze_ivtr_slot(1, idx_corpo as u16).data,
            envio,
        )
        .await;
    }

    /// `C2S::MOVE_ITEM_TO_EQUIP` (18) — mover da bolsa direto para um slot do corpo.
    async fn mover_para_equipar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(p) = ParDeSlots::ler(payload) else {
            warn!("mundo: move_item_to_equip de {roleid} com payload curto");
            return;
        };

        let itens = self.itens().await;
        if let Err(e) = itens
            .move_between_containers(
                roleid,
                ContainerType::Inventory,
                p.a as u16,
                ContainerType::Equipment,
                p.b as u16,
            )
            .await
        {
            warn!("mundo: falha ao mover para o corpo de {roleid}: {e:?}");
            return;
        }

        self.responder(
            roleid,
            S2CGamedataSend::move_item_to_equip(p.a, p.b, 1).data,
            envio,
        )
        .await;
        self.responder(
            roleid,
            S2CGamedataSend::unfreeze_ivtr_slot(0, p.a as u16).data,
            envio,
        )
        .await;
        self.responder(
            roleid,
            S2CGamedataSend::unfreeze_ivtr_slot(1, p.b as u16).data,
            envio,
        )
        .await;
    }

    /// Manda um subcomando já codificado ao jogador, pela conexão de onde ele veio.
    async fn responder(&self, roleid: i32, data: Vec<u8>, envio: &EnvioAoCliente) {
        let localsid = self
            .sessoes
            .read()
            .await
            .get(&roleid)
            .map(|s| s.localsid)
            .unwrap_or(0);

        if envio
            .try_send(BusMessage::GameToClient {
                roleid,
                localsid,
                data,
            })
            .is_err()
        {
            warn!("mundo: fila cheia ao responder a {roleid}");
        }
    }

    /// Envia uma mensagem a um jogador específico.
    ///
    /// `false` quando o jogador não está neste servidor de mundo, ou quando a fila dele
    /// está cheia — os dois casos são do chamador decidir, não deste módulo.
    #[allow(clippy::let_underscore_future)]
    pub async fn enviar_ao_jogador(&self, roleid: i32, data: Vec<u8>) -> bool {
        let sessoes = self.sessoes.read().await;
        let Some(s) = sessoes.get(&roleid) else {
            return false;
        };
        s.envio
            .try_send(BusMessage::GameToClient {
                roleid,
                localsid: s.localsid,
                data,
            })
            .is_ok()
    }

    /// Quantos jogadores este servidor de mundo está atendendo.
    pub async fn jogadores_atendidos(&self) -> usize {
        self.sessoes.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_cabecalho_do_subcomando_e_little_endian() {
        // `cmd_header { unsigned short cmd; }` — dois bytes, little-endian, porque o
        // payload do mundo 3D é memória i386 copiada crua. Ler como big-endian daria
        // 0x0F00 (3840) em vez de 15 (`OBJECT_MOVE`).
        let cmd = SubComando::ler(&[0x0F, 0x00, 0xAA, 0xBB]).unwrap();
        assert_eq!(cmd.id, 15);
        assert_eq!(cmd.payload, vec![0xAA, 0xBB]);
    }

    #[test]
    fn subcomando_sem_payload_e_valido() {
        // Vários comandos são só o cabeçalho: `UNSELECT`, `SIT_DOWN`, `STAND_UP`.
        let cmd = SubComando::ler(&[0x08, 0x00]).unwrap();
        assert_eq!(cmd.id, 8);
        assert!(cmd.payload.is_empty());
    }

    #[test]
    fn payload_curto_demais_nao_vira_subcomando() {
        assert_eq!(SubComando::ler(&[]), None);
        assert_eq!(SubComando::ler(&[0x0F]), None);
    }
}
