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

use crate::entity::PlayerEntity;
use crate::combat::{self, CombatEngine};
use crate::habilidades::Habilidade;
use crate::comandos::{
    ids, CastSkill, ConsultaDeIds, EmoteAction, GetAllData, GetIvtrDetail, Logout, MoveIvtrItem,
    NormalAttack, ParDeSlots, PlayerMove, SelectTarget, SevnpcHello, StopMove, TaskNotify,
    TipoDeSaida, UseItem,
};
use crate::npc::{self, servico, PedidoAoNpc};
use crate::world::{EventoDoMundo, WorldInstance};
use pw_bus::{BusListener, BusMessage};
use pw_core::{ContainerType, Vector3};
use pw_protocol::{versions::create_world_protocol, GameVersion, S2CGamedataSend, WorldProtocol};
use pw_wire::gamedata::Reader;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, trace, warn};

mod habilidades;
mod jogo;
mod mascote;

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

// Índices compartilhados por toda poção da mesma família. A ordem é a enumeração
// `COOLDOWN_INDEX_*` de `gs/cooldowncfg.h:62-78`; `item_potion.cpp:18-110` confere o índice
// antes de consumir e o arma com o `cool_time` do `MEDICINE_ESSENCE`.
const RECARGA_POCAO_REJUVENESCEDORA: i32 = 3;
const RECARGA_POCAO_DE_VIDA: i32 = 11;
const RECARGA_POCAO_DE_MANA: i32 = 12;
const RECARGA_ANTIDOTO: i32 = 13;

/// `id_major_type` → família de recarga.
///
/// Quem decide **não** é o que a poção restaura, e sim a classe do item, que
/// `set_to_classid` tira do `id_major_type` do `MEDICINE_ESSENCE`
/// (`gs/template/setclassid.cpp:81-101`): 1794 `CLS_ITEM_HEALING_POTION`, 1802
/// `CLS_ITEM_MANA_POTION`, 1810 `CLS_ITEM_REJUVENATION_POTION`, 1815 e 2038 os antídotos.
/// Cada `OnUse` arma o índice da sua classe (`gs/item/item_potion.cpp:18-110`).
///
/// Sem o campo — o leitor tipado do 1.2.6 não o traz — sobra o que o remédio restaura, que
/// é a mesma divisão nas poções que existem lá: `falta` medir o `id_major_type` do 1.2.6.
fn familia_de_recarga(tipo_maior: Option<i32>, hp_total: i32, mp_total: i32) -> i32 {
    match tipo_maior {
        Some(1794) => RECARGA_POCAO_DE_VIDA,
        Some(1802) => RECARGA_POCAO_DE_MANA,
        Some(1810) => RECARGA_POCAO_REJUVENESCEDORA,
        Some(1815) | Some(2038) => RECARGA_ANTIDOTO,
        _ if hp_total > 0 && mp_total > 0 => RECARGA_POCAO_REJUVENESCEDORA,
        _ if mp_total > 0 && hp_total <= 0 => RECARGA_POCAO_DE_MANA,
        _ => RECARGA_POCAO_DE_VIDA,
    }
}

/// `attack_speed` de um golpe de habilidade.
///
/// O original só preenche `attack_msg.speed` no golpe **normal** (`MakeAttackMsg`,
/// `actobject.cpp:826`); o golpe de habilidade monta a mensagem no próprio `cskill` e deixa o
/// campo zerado. Por isso zero aqui não é chute: é o que o original manda.
const VELOCIDADE_PADRAO: u8 = 0;

/// O `speed` que vai no `HOST_ATTACKRESULT` de um golpe normal: o **atraso do golpe da arma**,
/// `attack_delay = (int)(attack_speed × 0,8) − 1` em tiques de 50 ms
/// (`playertemplate.h:980`, mandado em `actobject.cpp:826`). O cliente usa esse número como a
/// duração da animação do golpe (`PlayAttackEffect(..., attack_speed × 50, …)`,
/// `EC_HostMsg.cpp:943-955`), e mandar a cadência cheia (30 tiques do arco em vez de 23)
/// deixava a animação ~350 ms mais lenta que a do original (B58).
fn atraso_do_golpe(attack_speed_s: f32) -> u8 {
    let ticks = (attack_speed_s * 20.0).round().clamp(4.0, 300.0);
    (((ticks * 0.8) as i32) - 1).clamp(1, 255) as u8
}

/// O `reason` dos comandos de saída de grupo: o jogador pediu para sair.
///
/// Zero é a saída voluntária. Expulsão e queda de conexão têm outros códigos, e o dia em
/// que existirem entram aqui como constantes próprias em vez de números soltos.
const SAIDA_VOLUNTARIA: i16 = 0;

/// `move_mode` do `OBJECT_MOVE` para quem anda no chão. É o mesmo valor que o cliente
/// manda no `PLAYER_MOVE` de um jogador a pé.
const MODO_DE_MOVIMENTO_ANDANDO: u8 = 0;

/// Quanto dura uma conjuração, em milissegundos.
///
/// Um valor só para os dois lados: é ele que vai no `time` do `OBJECT_CAST_SKILL` (com o
/// qual o cliente arma o contador da barra) e é ele que a tarefa do fim espera. Se os dois
/// se separassem, a barra fecharia antes ou depois do efeito.
///
/// **É um valor fixo, e o certo viria da habilidade**: cada stub do `ElementSkill` tem seu
/// `GetExecutetime`/`GetCoolingtime` (a Prece da Clareza, por exemplo, executa em 1000 ms).
/// O servidor ainda não lê a tabela de habilidades — quando ler, este número sai daqui.
const TEMPO_DE_CONJURACAO_MS: u16 = 1000;

/// Em que nível uma habilidade é conjurada quando o jogador **não a tem** no
/// `character_skills`.
///
/// Até 2026-09-09 era o nível de toda conjuração, de todo jogador: o `CAST_SKILL` do
/// cliente não manda o nível — quem tem de saber é o servidor — e o mundo não carregava a
/// tabela. Subir uma habilidade não mudava nada em jogo. Agora o nível sai de
/// `PlayerEntity::habilidades`, e este valor é só o piso de quem conjura o que não
/// aprendeu.
///
/// Nível 1 e não zero: as fórmulas do `Habilidade` indexam tabelas por nível a partir de
/// 1, e o cliente também recusaria o nível zero (`ElementSkill::Condition`).
const NIVEL_MINIMO_DA_HABILIDADE: i32 = 1;

/// `ERR_EQUIPMENT_IS_LOCKED` (`common/protocol.h:720`, "// 40").
const ERRO_EQUIPAMENTO_TRANCADO: i32 = 40;

/// `EQUIPIVTR_FLYSWORD` do `EC_IvtrTypes.h`: o slot do item de voo (espada voadora para os
/// humanos, asa para os Alados).
const SLOT_DE_VOO: u16 = 12;

/// Até onde o jogador enxerga NPCs e monstros, em metros.
///
/// O mesmo raio que o login usava. O cliente tem o raio ativo dele
/// (`SevActiveRadius`) e descarta o que passa disso; mandar mais do que ele guarda é
/// desperdício de fila.
const RAIO_DE_VISAO: f32 = 120.0;

/// Quanto o jogador precisa andar para o mundo em volta ser recalculado, em metros.
///
/// O cliente manda movimento 20 vezes por segundo. Sem esta histerese, cada jogador faria
/// 20 varreduras da grade por segundo para achar quase sempre o mesmo conjunto.
const PASSO_PARA_RECALCULAR: f32 = 20.0;

/// Quanto acima do chão o teleporte deposita o jogador, em metros.
///
/// O original põe o jogador exatamente na altura do terreno
/// (`playercmd.cpp:4926`) e deixa a física do cliente assentar. Aqui vai uma folga
/// pequena de propósito: a nossa altura vem da mesma interpolação do `.hmap`, mas o
/// cliente tem a malha real do mapa por cima (`VertRayTrace`), e um telhado, uma ponte ou
/// uma escada ficam **acima** do terreno. Chegar rente ao chão nesses lugares põe o
/// personagem dentro da geometria; chegar meio metro acima deixa a queda resolver.
const FOLGA_AO_TELEPORTAR: f32 = 0.5;

/// Quantas entidades no máximo um jogador acompanha de uma vez.
///
/// Este mapa tem 21.846 monstros e 3.911 NPCs. Numa região densa, o raio de 120 m pega
/// centenas — e cada uma é um pacote. O teto é orçamento de fila, não regra do jogo: os
/// mais próximos entram primeiro, e o resto chega na próxima atualização.
const TETO_DE_VISIVEIS: usize = 80;

/// Quantos recursos de mapa (minério, erva) um jogador acompanha de uma vez.
///
/// Orçamento **separado** do de [`TETO_DE_VISIVEIS`], de propósito. O `npcgen.data` deste
/// mapa tem 5.125 instâncias de matéria, e um campo de mineração as concentra: no mesmo
/// balde que monstro e NPC, elas comeriam o teto inteiro e fariam os NPCs sumirem perto de
/// uma mina — trocando um buraco por outro.
const TETO_DE_MATERIA: usize = 40;

/// O que entrou no campo de visão de um jogador, com o que o comando de entrada precisa.
///
/// Existe porque o comando **não é o mesmo** para as duas famílias: `NPC_ENTER_SLICE` (11)
/// para NPC e monstro, `PLAYER_ENTER_SLICE` (12) para jogador. O cliente roteia pelo
/// comando (`EC_GameDataPrtc.cpp:832-853`), então um jogador mandado pelo 11 cai no
/// gerente de NPCs. Ver [`BusServer::atualizar_visiveis`].
/// Em que nível este jogador tem esta habilidade.
///
/// O `CAST_SKILL` do cliente manda o id da habilidade e o alvo, **não o nível**
/// (`cmd_cast_skill`): quem tem de saber é o servidor. O nível vem do `character_skills`,
/// carregado com o personagem no login (`PlayerEntity::habilidades`).
///
/// Quem conjura o que não aprendeu cai em [`NIVEL_MINIMO_DA_HABILIDADE`]. Isso não é
/// permissão: é o piso de dano de um caso que o cliente já não deveria produzir, e recusar
/// a conjuração aqui deixaria o cliente preso em estado de feitiço — a checagem de "pode
/// conjurar" é outra conversa, e não existe ainda.
/// O marcador de cada conjuração aberta — ver [`crate::entity::Conjuracao`].
static PROXIMA_CONJURACAO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn nivel_da_habilidade(jogador: &PlayerEntity, skill_id: i32) -> i32 {
    let id = if skill_id < 0 { return NIVEL_MINIMO_DA_HABILIDADE } else { skill_id as u32 };
    jogador
        .habilidades
        .get(&id)
        .map(|n| (*n as i32).max(NIVEL_MINIMO_DA_HABILIDADE))
        .unwrap_or(NIVEL_MINIMO_DA_HABILIDADE)
}

/// `ISMATTERID` do cliente (`EC_GPDataType.h:27`): os dois bits mais altos ligados.
///
/// É por esta máscara que o cliente decide para qual gerente mandar um id numa lista
/// mista, e é por ela que este servidor decide qual comando de saída usar.
fn e_materia(id: i64) -> bool {
    (id as u32) & 0xC000_0000 == 0xC000_0000
}

enum QuemChegou {
    Criatura { id: i32, tid: i32, pos: pw_core::Vector3, dir: u8 },
    Jogador { id: i32, vista: pw_core::VistaDoJogador },
    Materia { id: i32, tid: i32, pos: pw_core::Vector3 },
    /// Mascote de combate: `info_npc` com a marca de mascote, o dono e o nome.
    Mascote { id: i32, tid: i32, vis: i32, pos: pw_core::Vector3, dir: u8, dono: i32, nome: Vec<u8> },
}

/// Canal por onde o mundo devolve mensagens àquele jogador.
pub type EnvioAoCliente = mpsc::Sender<BusMessage>;

/// Estado de um jogador que este servidor de mundo está atendendo.
struct Sessao {
    localsid: u32,
    envio: EnvioAoCliente,
}

/// `DROP_TYPE_PLAYER` = 1: o jogador jogou o item fora (`common/protocol.h:928-942`).
const DROP_TYPE_PLAYER: u8 = 1;

/// A ponta de rede do servidor de mundo.
///
/// Clonar é barato e **compartilha o mesmo mundo**: `world` e `sessoes` são `Arc`, e
/// `sub` é um `Copy` de um byte. Serve para tarefas que precisam terminar depois de a
/// mensagem já ter sido respondida — ver [`Self::conjurar`].
#[derive(Clone)]
pub struct BusServer {
    world: Arc<RwLock<WorldInstance>>,
    /// Os subcomandos cujo layout depende da versão do realm.
    ///
    /// Trinta e dois comandos medidos num servidor 1.2.6 real têm layout diferente do
    /// 1.5.3 (item 56). Mandar o layout errado não dá erro: o cliente **descarta o comando
    /// inteiro** (item 46). Por isso a versão vive aqui dentro, e não como um argumento
    /// que se pode esquecer numa chamada.
    /// A estratégia de protocolo desta versão — uma implementação de [`WorldProtocol`] por
    /// versão (`pw_protocol::versions`). Os comandos cujo layout muda entre versões saem
    /// daqui; os que são iguais em todas ficam em `S2CGamedataSend`.
    sub: Arc<dyn WorldProtocol>,
    /// Jogadores atendidos, por `roleid`. É o que permite ao mundo devolver uma
    /// mensagem a um jogador específico sem saber nada sobre conexões.
    sessoes: Arc<RwLock<HashMap<i32, Sessao>>>,
    /// Para onde mandar o pedido de trocar de mapa — ligado por
    /// [`crate::mapas::RoteadorDeMapas::ligar_trocas`]. Vazio num mapa avulso.
    trocas: std::sync::OnceLock<mpsc::UnboundedSender<PedidoDeTroca>>,
    /// Referência fraca a si mesmo, para tarefas que terminam depois do comando (a coleta).
    eu: std::sync::OnceLock<std::sync::Weak<BusServer>>,
    /// Golpe normal que chegou com uma conjuração em curso, esperando a vez: `roleid → alvo`.
    ///
    /// No original a habilidade é a **sessão corrente** enquanto roda, e o `NORMAL_ATTACK`
    /// que chega só entra na fila (`AddSession` devolve `false`, `actobject.cpp:1180-1212`);
    /// ele começa quando a habilidade termina (`SafeDeleteCurSession` → `StartSession`), e aí
    /// o `CheckAttack` recusa alvo morto. Sem esta fila os dois danos caíam no mesmo instante
    /// (B57).
    golpe_na_fila: Arc<RwLock<HashMap<i32, i64>>>,
}

/// Um jogador que tem de passar para outro mapa do mesmo processo.
#[derive(Debug, Clone, Copy)]
pub struct PedidoDeTroca {
    pub roleid: i32,
    pub mundo: i32,
    pub pos: Vector3,
}

/// O que sai de um mapa e entra no outro na troca.
pub struct JogadorEmTroca {
    sessao: Sessao,
    jogador: PlayerEntity,
}

impl BusServer {
    /// Monta o servidor de mundo para a versão daquele realm.
    pub fn new(world: Arc<RwLock<WorldInstance>>, versao: GameVersion) -> Self {
        Self {
            world,
            sub: create_world_protocol(versao),
            sessoes: Arc::new(RwLock::new(HashMap::new())),
            trocas: std::sync::OnceLock::new(),
            eu: std::sync::OnceLock::new(),
            golpe_na_fila: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Este servidor como `Arc`, se ele já foi ligado ([`Self::ligar_eventos_do_mundo`]).
    pub(crate) fn clone_arc(&self) -> Option<Arc<BusServer>> {
        self.eu.get().and_then(|w| w.upgrade())
    }

    /// Liga este mapa ao roteador que executa as trocas de mapa.
    pub fn ligar_trocas(&self, envio: mpsc::UnboundedSender<PedidoDeTroca>) {
        let _ = self.trocas.set(envio);
    }

    /// `gplayer_imp::LongJump` (`player.cpp:8617-8680`): no mesmo mapa, `notify_pos` e
    /// reposiciona; noutro, `PlaneSwitch` — aqui, o pedido ao roteador.
    pub(crate) async fn transportar(&self, roleid: i32, mundo: i32, pos: Vector3) {
        let este = self.world.read().await.world_id;
        if mundo != este {
            match self.trocas.get() {
                Some(t) if t.send(PedidoDeTroca { roleid, mundo, pos }).is_ok() => {
                    info!("mundo: {roleid} vai do mapa {este} para o {mundo} em {pos:?}");
                }
                _ => warn!("mundo: {roleid} devia ir para o mapa {mundo}, que este processo não serve"),
            }
            return;
        }
        let Some(envio) = self.sessoes.read().await.get(&roleid).map(|s| s.envio.clone()) else {
            return;
        };
        {
            let mut mundo_ = self.world.write().await;
            let Some(j) = mundo_.players.get_mut(&(roleid as i64)) else { return };
            j.position = pos;
            mundo_.grid.update_position(roleid as i64, pos);
        }
        info!("mundo: {roleid} teleportado para {pos:?} no mapa {este}");
        self.enviar_ao_jogador(roleid, S2CGamedataSend::notify_hostpos(pos, este, 0).data).await;
        self.transmitir_a_outros(roleid, self.sub.object_stop_move(roleid, pos, 0, 0, MODO_DE_MOVIMENTO_ANDANDO).data).await;
        self.atualizar_visiveis(roleid, &envio, true).await;
    }

    /// Tira o jogador deste mapa para outro: some da vista de todos, larga sessão e entidade.
    pub(crate) async fn retirar_para_troca(&self, roleid: i32) -> Option<JogadorEmTroca> {
        self.tirar_da_vista_de_todos(roleid as i64).await;
        let sessao = self.sessoes.write().await.remove(&roleid)?;
        let jogador = self.world.write().await.remove_player(roleid)?;
        Some(JogadorEmTroca { sessao, jogador })
    }

    /// A outra metade da troca: o que o `gs` original faz ao receber um jogador de fora
    /// (`global_message.cpp:111-117`) — `notify_pos` com o mapa novo, e o mundo em volta.
    /// Grava mapa e posição na hora, para que um relogar caia aqui.
    pub(crate) async fn receber_de_outro_mapa(&self, roleid: i32, vindo: JogadorEmTroca, pos: Vector3) {
        let JogadorEmTroca { sessao, mut jogador } = vindo;
        let (este, repo, chao) = {
            let m = self.world.read().await;
            (m.world_id, m.char_repo.clone(), m.terreno.altura_em(pos.x, pos.z))
        };
        let mut pos = pos;
        // `if (pos.y < height) pos.y = height` (`global_message.cpp:100-101`).
        if let Some(c) = chao {
            pos.y = pos.y.max(c);
        }
        jogador.position = pos;
        jogador.centro_do_stream = pos;
        jogador.visiveis.clear();
        jogador.target_id = None;
        let envio = sessao.envio.clone();
        // Grava antes de o jogador existir no mapa novo: ninguém vê o estado novo sem o banco.
        let g = (jogador.level, jogador.cultivation, jogador.exp, jogador.sp, jogador.hp, jogador.mp, jogador.money);
        if let Err(e) = repo.save_status(roleid, g.0, g.1, g.2, g.3, g.4, g.5, g.6, este, &pos).await {
            warn!("mundo: não consegui gravar {roleid} no mapa {este}: {e}");
        }
        self.sessoes.write().await.insert(roleid, sessao);
        self.enviar_ao_jogador(roleid, S2CGamedataSend::notify_hostpos(pos, este, 0).data).await;
        self.world.write().await.add_player(jogador);
        info!("mundo: {roleid} chegou ao mapa {este} em {pos:?}");
        self.atualizar_visiveis(roleid, &envio, true).await;
    }

    /// O mundo que este servidor atende.
    pub fn mundo(&self) -> &Arc<RwLock<WorldInstance>> {
        &self.world
    }

    /// A versão que este mundo fala.
    pub fn versao(&self) -> GameVersion {
        self.sub.version()
    }

    /// Liga a saída de eventos da simulação a este servidor e começa a entregá-los.
    ///
    /// Sem isto, o que acontece no tick — um monstro batendo no jogador, o jogador
    /// morrendo — fica dentro do processo e o cliente nunca sabe. Era literalmente o
    /// estado anterior: o HP caía em silêncio até o jogador morrer sem aviso.
    pub async fn ligar_eventos_do_mundo(self: &Arc<Self>) {
        let _ = self.eu.set(Arc::downgrade(self));
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
            ev @ (EventoDoMundo::MascoteApareceu { .. }
            | EventoDoMundo::MascoteRecolhido { .. }
            | EventoDoMundo::MascoteMorreu { .. }
            | EventoDoMundo::GolpeEntreCriaturas { .. }
            | EventoDoMundo::VidaDoMascote { .. }
            | EventoDoMundo::ExpDoMascote { .. }
            | EventoDoMundo::IaDoMascote { .. }
            | EventoDoMundo::FomeDoMascote { .. }
            | EventoDoMundo::ReviverMascote { .. }
            | EventoDoMundo::MascoteConjurou { .. }
            | EventoDoMundo::MascoteUsouHabilidade { .. }
            | EventoDoMundo::RecargaDoMascote { .. }
            | EventoDoMundo::ErroDoMascote { .. }) => self.evento_de_mascote(ev).await,
            EventoDoMundo::Renasceu { objeto } => {
                // `SendClientEnchantResult(GetSelfID(), 1085, 1, false, 0, 0)`.
                let pacote = self.sub.enchant_result(objeto as i32, objeto as i32, 1085, 1, false, 0, 0).data;
                if self.world.read().await.players.contains_key(&objeto) {
                    self.enviar_ao_jogador(objeto as i32, pacote.clone()).await;
                }
                self.transmitir_a_quem_ve(objeto, pacote).await;
            }
            EventoDoMundo::DanoRecebido {
                roleid,
                atacante,
                dano,
                hp,
                max_hp,
            } => {
                // Dois avisos: o golpe em si, e a vida que sobrou.
                //
                // O `speed` é o `_damage_delay` do monstro (`npc.cpp:2118`), em tiques de
                // 50 ms: o cliente o usa como duração da animação do golpe
                // (`CECNPC::OnMsgAttackHostResult`, `EC_NPC.cpp:2043-2064`). Ia zero (B60).
                let atraso = self
                    .world
                    .read()
                    .await
                    .monsters
                    .get(&atacante)
                    .map(|(m, _)| m.atraso_do_dano_em_ticks.clamp(0, 255) as u8)
                    .unwrap_or(0);
                // Levar dano desgasta uma peça sorteada, e é o índice dela que vai no
                // comando (`gplayer_imp::OnDamage` → `be_damaged(..., index, ...)`,
                // `player.cpp:9552-9570`). Sem peça no slot sorteado vai `0x7f`, e o cliente
                // não desgasta nada (`EC_HostMsg.cpp:974-981`).
                let peca = self.desgastar_peca(roleid).await;
                self.enviar_ao_jogador(
                    roleid,
                    self.sub
                        .host_attacked(
                            atacante as i32,
                            dano,
                            peca,
                            SEM_MARCACAO,
                            atraso,
                        )
                        .data,
                )
                .await;
                // A vida **não** vai aqui. Este evento é o anúncio do golpe, e a vida só cai
                // quando o dano adiado vence (`InsertDamageEntry`): quem emite
                // [`EventoDoMundo::EstadoMudou`] é o `aplicar_dano_no_jogador`, e é de lá que
                // sai o `SELF_INFO_00` com o valor já descontado. Mandar a barra aqui era
                // mandá-la **antes** do dano, e o cliente recebia a vida velha depois do
                // golpe — o banco lento é que escondia isso (B72).
                let _ = (hp, max_hp);
            }

            EventoDoMundo::MonstroAndou {
                id,
                destino,
                tempo_ms,
                velocidade,
                modo,
            } => {
                // `OBJECT_MOVE` (15) é o mesmo comando que anuncia jogador andando — o
                // cliente distingue pelo id do objeto. Unidades do original
                // (`gs/npcsession.cpp:258`): tempo em ms, velocidade em 1/256 de m/s. Ver
                // a nota de `crate::ai`.
                let speed = crate::ai::AcaoDoMonstro::velocidade_no_protocolo(velocidade);
                let pacote = self
                    .sub
                    .object_move(id as i32, destino, tempo_ms, speed, modo)
                    .data;
                self.transmitir_a_quem_ve(id, pacote).await;
            }

            EventoDoMundo::MonstroParou {
                id,
                posicao,
                velocidade,
                direcao,
                modo,
            } => {
                let speed = crate::ai::AcaoDoMonstro::velocidade_no_protocolo(velocidade);
                let pacote = self
                    .sub
                    .object_stop_move(id as i32, posicao, speed, direcao, modo)
                    .data;
                self.transmitir_a_quem_ve(id, pacote).await;
            }

            EventoDoMundo::JogadorMorreu {
                roleid,
                matador,
                pos,
            } => {
                // Morrer cancela golpe e conjuração (`world.rs`), e com eles o golpe que
                // esperava a vez (B57).
                self.golpe_na_fila.write().await.remove(&roleid);
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

            EventoDoMundo::MontariaCaiuNaAgua { roleid } => {
                // `mount_petdata_imp::TestUnderWater` (`gs/petman.cpp:402-410`): tira o
                // `mount_filter` e limpa o mascote ativo.
                //
                // O original **só** limpa o estado interno ali — não manda `recall_pet`. Nós
                // mandamos, reusando o mesmo caminho do recolher voluntário: sem ele o
                // cliente ficaria com o mascote marcado como ativo e o botão de recolher
                // apagado, que é exatamente o travamento que o B79 corrigiu.
                let montaria = self.world.read().await.players.get(&(roleid as i64)).and_then(|p| p.montaria);
                if let Some(m) = montaria {
                    info!("mundo: a montaria de {roleid} caiu — entrou na água");
                    let envio = self.envio_de(roleid).await;
                    if let Some(envio) = envio {
                        self.desmontar(roleid, m, &envio).await;
                    }
                }
            }

            EventoDoMundo::AmuletoDisparou { roleid, slot, item_id, restou, bloco, indice_de_recarga, recarga_ms } => {
                // A recarga vai ao cliente: `gplayer_imp::SetCoolDown` grava e **sempre**
                // manda `set_cooldown(idx, msec)` (`gs/player.cpp:12701-12709`); é ela que
                // escurece o ícone do amuleto. Sem este comando o item parecia nunca entrar
                // em recarga, embora o servidor já a respeitasse (B74).
                if recarga_ms > 0 {
                    self.enviar_ao_jogador(
                        roleid,
                        S2CGamedataSend::set_cooldown(indice_de_recarga, recarga_ms).data,
                    )
                    .await;
                }
                // O que sobrou fica gravado nos octetos do próprio item — é o `Save` do
                // `amulet_essence` do original. Em zero o item sai do corpo e o cliente é
                // avisado com `PLAYER_DROP_ITEM` tipo `DROP_TYPE_USE` (11); enquanto resta,
                // vai o `item_info` com o número novo (`gs/player_imp.h:3568-3590`).
                const DROP_POR_USO: u8 = 11;
                let pacote = ContainerType::Equipment.pacote_do_cliente().unwrap_or(1);
                if restou <= 0 {
                    self.enviar_ao_jogador(
                        roleid,
                        self.sub.player_drop_item(pacote, slot as u8, 1, item_id as i32, DROP_POR_USO).data,
                    )
                    .await;
                } else if let Ok(Some(mut i)) =
                    self.itens().await.get_item_by_slot(roleid, ContainerType::Equipment, slot).await
                {
                    i.octets = bloco.clone();
                    let dados = self.world.read().await.data_manager.clone();
                    self.enviar_ao_jogador(roleid, Self::info_de(pacote, &i, &dados)).await;
                }
                // O banco depois, fora do caminho do jogo (B72).
                if let Some(este) = self.clone_arc() {
                    tokio::spawn(async move {
                        let repo = este.itens().await;
                        if restou <= 0 {
                            if let Err(e) = repo.delete_item_by_slot(roleid, ContainerType::Equipment, slot).await {
                                warn!("mundo: não consegui tirar o amuleto gasto de {roleid}: {e}");
                            }
                        } else if let Ok(Some(mut i)) =
                            repo.get_item_by_slot(roleid, ContainerType::Equipment, slot).await
                        {
                            i.octets = bloco;
                            if let Err(e) = repo.upsert_item(&i).await {
                                warn!("mundo: não consegui gravar o amuleto de {roleid}: {e}");
                            }
                        }
                    });
                }
            }

            EventoDoMundo::EstadoMudou { roleid } => self.avisar_vida_propria(roleid).await,

            EventoDoMundo::VidaDoMonstro { id, hp, max_hp, alvo, para } => {
                let pacote = self.sub.npc_info_00(id as i32, hp, max_hp, alvo).data;
                for roleid in para {
                    self.enviar_ao_jogador(roleid, pacote.clone()).await;
                }
            }

            EventoDoMundo::MonstroSumiu { id } | EventoDoMundo::DropSumiu { id } => {
                self.transmitir_a_outros(0, S2CGamedataSend::object_disappear(id as i32).data).await;
            }

            EventoDoMundo::EfeitosMudaram { objeto, atributos } => self.avisar_efeitos(objeto, atributos).await,
            EventoDoMundo::MonstroMorreu { id, matador } => self.anunciar_morte_do_monstro(id, matador).await,

            EventoDoMundo::GolpeDoJogador { roleid } => {
                let envio = self.sessoes.read().await.get(&roleid).map(|s| s.envio.clone());
                if let Some(envio) = envio {
                    // Com um golpe na fila, a sessão atual termina e a da fila começa
                    // (`EndCurSession` + `StartSession`, `actobject.cpp:185-189`).
                    let sessao = self.world.read().await.players.get(&(roleid as i64)).and_then(|p| p.ataque);
                    match sessao {
                        Some(s) if s.proximo.is_some() || s.cancelar || s.andar => {
                            self.encerrar_ataque(roleid, 0).await;
                            if let Some(alvo) = s.proximo {
                                self.abrir_sessao_de_golpe(roleid, alvo, &envio).await;
                                if s.andar {
                                    if let Some(n) = self.world.write().await.players.get_mut(&(roleid as i64)).and_then(|p| p.ataque.as_mut()) {
                                        n.andar = true;
                                    }
                                }
                            }
                        }
                        Some(_) => self.golpear(roleid, &envio).await,
                        None => {}
                    }
                }
            }

            EventoDoMundo::MinaRenasceu { id } => {
                let (perto, pacote) = {
                    let mut mundo = self.world.write().await;
                    let Some(m) = mundo.matters.get(&id) else { return };
                    let (pos, tid) = (m.position, m.template_id);
                    let ids: Vec<i64> = mundo
                        .players
                        .iter()
                        .filter(|(_, p)| p.position.distance(&pos) <= RAIO_DE_VISAO)
                        .map(|(pid, _)| *pid)
                        .collect();
                    for pid in &ids {
                        if let Some(p) = mundo.players.get_mut(pid) {
                            p.visiveis.insert(id);
                        }
                    }
                    (ids, S2CGamedataSend::matter_enter_world(id as i32, tid as i32, pos).data)
                };
                for pid in perto {
                    self.enviar_ao_jogador(pid as i32, pacote.clone()).await;
                }
            }

            EventoDoMundo::MonstroRenasceu { id } => {
                // Quem está perto volta a ver o monstro sem precisar andar: o streaming só
                // recalcula depois de 20 m (`PASSO_PARA_RECALCULAR`).
                let (perto, pacote) = {
                    let mut mundo = self.world.write().await;
                    let Some((m, ia)) = mundo.monsters.get(&id) else { return };
                    let (pos, tid, dir) = (m.position, m.template_id, ia.direcao);
                    let ids: Vec<i64> = mundo
                        .players
                        .iter()
                        .filter(|(_, p)| p.position.distance(&pos) <= RAIO_DE_VISAO)
                        .map(|(pid, _)| *pid)
                        .collect();
                    for pid in &ids {
                        if let Some(p) = mundo.players.get_mut(pid) {
                            p.visiveis.insert(id);
                        }
                    }
                    // `NPC_ENTER_WORLD` (16), como o original no renascimento (captura do 1.2.6:
                    // o Filhote de Mandrágora volta com 16, sem `disappear` antes — o cliente
                    // ainda tem o corpo com o mesmo id, e é o 16 que o põe de pé no ponto novo).
                    (ids, self.sub.npc_enter_world(id as i32, tid as i32, pos, dir).data)
                };
                for pid in perto {
                    self.enviar_ao_jogador(pid as i32, pacote.clone()).await;
                }
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
        let Some((nivel, nivel2, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap)) = dados else {
            return;
        };
        self.enviar_ao_jogador(
            roleid,
            S2CGamedataSend::self_info_00(nivel, nivel2, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap).data,
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
        self.esquecer_sessoes(&donos).await;
    }

    /// Esquece as sessões destes jogadores — a conexão por onde vinham caiu.
    pub(crate) async fn esquecer_sessoes(&self, roleids: &[i32]) {
        let mut sessoes = self.sessoes.write().await;
        for roleid in roleids {
            sessoes.remove(roleid);
        }
    }

    pub(crate) async fn tratar(&self, msg: BusMessage, envio: &EnvioAoCliente) {
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
                self.colocar_no_mundo(roleid, envio).await;
            }

            BusMessage::PlayerLogout { roleid, .. } => {
                // Antes de tirar do mundo: quem estava vendo este jogador precisa receber
                // o `PLAYER_LEAVE_WORLD`, senão o avatar dele fica parado na tela dos
                // outros. Era o `gateway.rs` que fazia isto; passou para cá junto com o
                // resto da visibilidade entre jogadores (ver `atualizar_visiveis`).
                self.tirar_da_vista_de_todos(roleid as i64).await;
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

    /// Carrega o personagem e o põe no mundo simulado.
    ///
    /// # Por que isto não existia
    ///
    /// `world.players` **nunca era populado**: `PlayerEntity` só era construído em teste.
    /// A consequência é que tudo que começa com "olhe o jogador" saía cedo sem fazer
    /// nada — `NORMAL_ATTACK` e `CAST_SKILL` retornam no `mundo.players.get()`, a tabela
    /// de ameaça nunca recebia nada, e o `MonsterAi` inteiro era código morto. O
    /// `remove_player` já era chamado nos dois caminhos de saída desde antes; faltava só
    /// a entrada.
    ///
    /// # O que acontece quando dá errado
    ///
    /// Personagem que não existe, banco fora do ar ou mapa trocado **não** derrubam a
    /// sessão: o jogador continua conectado e recebendo pacotes pelo caminho antigo, e o
    /// log diz o que faltou. Cair aqui e desconectar seria trocar "combate não funciona"
    /// por "não dá para jogar".
    async fn colocar_no_mundo(&self, roleid: i32, envio: &EnvioAoCliente) {
        let (repo, world_id, dados) = {
            let mundo = self.world.read().await;
            (
                mundo.char_repo.clone(),
                mundo.world_id,
                Arc::clone(&mundo.data_manager),
            )
        };

        // Carimba a entrada numa tarefa à parte.
        //
        // É este carimbo que faz o cliente vir com o último personagem jogado selecionado
        // na próxima vez — mas é um dado de conveniência, e **não pode atrasar a entrada
        // no mundo**. Esperar a ida ao banco aqui acrescentava uma viagem de rede no meio
        // da sequência de entrada, o suficiente para deslocar a ordem dos pacotes que o
        // cliente recebe logo depois.
        {
            let repo = repo.clone();
            tokio::spawn(async move {
                if let Err(e) = repo.marcar_entrada_no_mundo(roleid).await {
                    warn!("mundo: não consegui marcar a entrada de {roleid}: {e}");
                }
            });
        }

        let detalhes = match repo.get_details_por_role(roleid).await {
            Ok(Some(d)) => d,
            Ok(None) => {
                warn!("mundo: jogador {roleid} entrou mas não existe no banco — sem entidade");
                return;
            }
            Err(e) => {
                warn!("mundo: não consegui carregar o jogador {roleid} do banco: {e}");
                return;
            }
        };

        if detalhes.world_id != world_id {
            // O `pw-link` roteia por realm, não por mapa: um personagem gravado noutro
            // mapa chega aqui do mesmo jeito. Entrar no mundo errado poria o jogador num
            // lugar onde ninguém o vê, então é melhor recusar e dizer.
            warn!(
                "mundo: jogador {roleid} é do mapa {} e este é o {world_id} — não entrou",
                detalhes.world_id
            );
            return;
        }

        let mut jogador = PlayerEntity::do_personagem(
            &detalhes,
            &dados.classes,
            Some(&dados.base_das_classes).filter(|b| !b.is_empty()),
        );
        // O privilégio de GM não vem no `EnterWorld` nem no `CharacterDetails`; é uma
        // leitura por login, e daqui em diante viaja em todo `PLAYER_ENTER_SLICE` que
        // apresenta este jogador aos outros.
        jogador.sec_level = repo.nivel_de_gm(roleid).await.clamp(0, 255) as u8;
        jogador.pontos_de_atributo = repo.pontos_de_atributo(roleid).await.unwrap_or(0);
        match repo.task_lists().carregar(roleid).await {
            Ok(Some(l)) => {
                jogador.missoes = crate::missoes::ListasDeMissao::de_blocos(
                    [&l.ativa, &l.concluidas, &l.tempos, &l.contagens, &l.deposito],
                    &dados.tasks,
                );
            }
            Ok(None) => {}
            Err(e) => warn!("mundo: não consegui ler as missões de {roleid}: {e}"),
        }

        if dados.classes.is_empty() {
            warn!(
                "mundo: o realm não tem CHARRACTER_CLASS_CONFIG — o jogador {roleid} entrou                  sem precisão, evasão nem dano por nível"
            );
        }
        if dados.base_das_classes.is_empty() {
            warn!(
                "mundo: o realm não tem ptemplate.conf — a vida máxima do jogador {roleid}                  é a que estava gravada no banco, não a calculada"
            );
        }

        // **Ninguém entra no mundo por baixo do chão.**
        //
        // É a regra do próprio original, do gerador de posição em volume
        // (`box_gen_pos::GenerateY`, `npcgenerator.cpp:4340-4345`): o terreno é piso,
        // nunca teto.
        //
        // ```cpp
        // float height = plane->GetHeightAt(x,z);
        // if (y < height) y = height;
        // return y + offset;
        // ```
        //
        // Medido em 2026-09-11, com o mapa de alturas já disponível: das seis posições de
        // nascimento de `CharacterClass::default_spawn_position`, três são coordenadas de
        // cidade de verdade e caem no chão (diferença de até 2,4 m), e **três são
        // palpites** — Abissais a 71 m **abaixo** do terreno, Guardiões a 11 m e Sombrios
        // a 40 m acima. As de cima o cliente resolve caindo; a de baixo prende o
        // personagem dentro da montanha.
        //
        // Levantar só quem está abaixo é de propósito: altura acima do chão é legítima
        // (voo, andar de cima de prédio, ponte), e forçar todo mundo ao terreno derrubaria
        // quem estivesse num deles.
        {
            let mundo = self.world.read().await;
            if let Some(chao) = mundo.terreno.altura_em(jogador.position.x, jogador.position.z) {
                if jogador.position.y < chao {
                    warn!(
                        "mundo: {roleid} entraria {:.1} m abaixo do chão em ({:.0}, {:.0}) —                          subido para a superfície",
                        chao - jogador.position.y,
                        jogador.position.x,
                        jogador.position.z
                    );
                    jogador.position.y = chao;
                }
            }
        }

        info!(
            "mundo: {} (#{roleid}) nível {} entrou no mapa {world_id} — {}/{} de vida,              dano {}, defesa {}, precisão {}, evasão {}",
            jogador.name, jogador.level, jogador.hp, jogador.max_hp, jogador.attack_min,
            jogador.def_phys, jogador.attack_rate, jogador.armor
        );
        self.world.write().await.add_player(jogador);
        self.recalcular_equipamento(roleid, false).await;

        // A carga inicial do mundo em volta. Quem manda os NPCs é o mundo, não o link:
        // ele tem a grade espacial, sabe quais monstros estão vivos, e é ele que vai
        // continuar mandando conforme o jogador anda. Ver [`Self::atualizar_visiveis`].
        self.atualizar_visiveis(roleid, envio, true).await;
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
            ids::PLAYER_MOVE => {
                self.andar_na_fila(roleid).await;
                self.mover(roleid, &cmd.payload, envio).await
            }
            ids::LOGOUT => self.sair(roleid, &cmd.payload, envio).await,
            ids::SELECT_TARGET => self.selecionar_alvo(roleid, &cmd.payload, envio).await,
            ids::UNSELECT => self.desmarcar(roleid, envio).await,
            ids::STOP_MOVE => {
                self.andar_na_fila(roleid).await;
                self.parar(roleid, &cmd.payload, envio).await
            }
            ids::NORMAL_ATTACK => self.atacar(roleid, &cmd.payload, envio).await,
            ids::REVIVE_VILLAGE => self.reviver(roleid).await,
            ids::GET_ITEM_INFO => self.info_do_item(roleid, &cmd.payload, envio).await,
            ids::PICKUP => self.pegar(roleid, &cmd.payload).await,
            ids::PICKUP_ALL => self.pegar_todos(roleid, &cmd.payload).await,
            ids::GET_IVTR_DETAIL => self.detalhe_do_container(roleid, &cmd.payload, envio).await,
            ids::EXG_IVTR_ITEM => self.trocar_slots(roleid, &cmd.payload, ContainerType::Inventory, envio).await,
            ids::EXG_EQUIP_ITEM => self.trocar_slots(roleid, &cmd.payload, ContainerType::Equipment, envio).await,
            ids::MOVE_IVTR_ITEM => self.mover_item(roleid, &cmd.payload, envio).await,
            ids::EQUIP_ITEM => self.equipar(roleid, &cmd.payload, envio).await,
            ids::MOVE_ITEM_TO_EQUIP => self.mover_para_equipar(roleid, &cmd.payload, envio).await,
            ids::SIT_DOWN => self.postura(roleid, true, envio).await,
            ids::STAND_UP => self.postura(roleid, false, envio).await,
            ids::CANCEL_ACTION => {
                // `CANCEL_ACTION` (`playercmd.cpp:2136-2153`) põe `session_cancel_action` na
                // fila — que tira o golpe que estava na fila (máscara exclusiva) — e tenta
                // `TerminateSession(false)`, que a sessão de golpe recusa
                // (`actsession.h:109-115`). A sessão termina no **próximo golpe**, quando
                // `HasNextSession` a encerra (`actobject.cpp:180-189`). B53 deixava o
                // cancelamento sem efeito: Esc não parava o ataque (teste de 2026-09-17).
                if let Some(s) = self.world.write().await.players.get_mut(&(roleid as i64)).and_then(|p| p.ataque.as_mut()) {
                    s.proximo = None;
                    s.andar = false;
                    s.cancelar = true;
                }
                self.interromper_conjuracao(roleid, 2, envio).await;
                self.postura(roleid, false, envio).await
            }
            ids::EMOTE_ACTION => self.emote(roleid, &cmd.payload, envio).await,
            ids::SEVNPC_SERVE => self.servico_de_npc(roleid, &cmd.payload, envio).await,
            ids::SEVNPC_HELLO => self.dizer_ola_ao_npc(roleid, &cmd.payload, envio).await,
            ids::TASK_NOTIFY => self.notificar_tarefa(roleid, &cmd.payload, envio).await,
            ids::CHECK_SECURITY_PASSWD => self.conferir_senha(roleid, &cmd.payload, envio).await,
            ids::USE_ITEM => self.usar_item(roleid, &cmd.payload, envio).await,
            ids::SUMMON_PET => self.invocar_mascote(roleid, &cmd.payload, envio).await,
            ids::RECALL_PET => self.recolher_mascote(roleid, envio).await,
            ids::BANISH_PET => self.soltar_mascote(roleid, &cmd.payload, envio).await,
            ids::PET_CTRL => self.ordem_ao_mascote(roleid, &cmd.payload).await,
            ids::TEAM_INVITE => self.convidar(roleid, &cmd.payload).await,
            ids::TEAM_AGREE_INVITE => self.aceitar_grupo(roleid, &cmd.payload).await,
            ids::TEAM_REJECT_INVITE => self.recusar_grupo(roleid).await,
            ids::TEAM_LEAVE_PARTY => self.deixar_grupo(roleid).await,
            ids::SWITCH_FASHION_MODE => self.trocar_modo_roupa(roleid, envio).await,
            ids::GOTO => self.teleportar(roleid, &cmd.payload, envio).await,
            ids::CAST_SKILL | ids::CAST_INSTANT_SKILL => {
                self.conjurar(roleid, &cmd.payload, envio).await
            }
            ids::ENTER_SANCTUARY => {
                debug!("mundo: {roleid} entrou em zona segura");
                self.responder(roleid, self.sub.enter_sanctuary(roleid).data, envio)
                    .await;
            }
            ids::GET_EXT_PROP => self.estado_proprio(roleid, envio).await,
            ids::SET_STATUS_POINT => self.distribuir_pontos(roleid, &cmd.payload).await,
            ids::CONTINUE_ACTION => self.soltar_carga(roleid, envio).await,
            ids::GATHER_MATERIAL => self.coletar(roleid, &cmd.payload).await,
            ids::QUERY_CASH_INFO => self.saldo(roleid, envio).await,
            ids::GET_ALL_DATA => self.todos_os_dados(roleid, &cmd.payload, envio).await,
            ids::QUERY_PLAYER_INFO_1 => self.consultar_jogadores(roleid, &cmd.payload, envio).await,
            ids::QUERY_NPC_INFO_1 => self.consultar_npcs(roleid, &cmd.payload, envio).await,
            ids::GET_OTHER_EQUIP => self.equipamento_de_outro(roleid, &cmd.payload, envio).await,
            ids::CALC_NETWORK_DELAY => self.medir_latencia(roleid, &cmd.payload, envio).await,
            ids::QUERY_TITLE => {
                // Sem título nenhum no banco ainda: a lista vai vazia, que é o que destrava o
                // sistema de missões do cliente (ver [`ids::QUERY_TITLE`], B60).
                self.responder(roleid, S2CGamedataSend::query_title_re(roleid, &[], &[]).data, envio).await;
            }
            ids::ACTIVATE_REGION_WAYPOINTS => self.ativar_waypoints(roleid, &cmd.payload, envio).await,
            ids::DROP_IVTR_ITEM => self.descartar_item(roleid, 0, &cmd.payload, envio).await,
            ids::DROP_EQUIP_ITEM => self.descartar_item(roleid, 1, &cmd.payload, envio).await,
            outro => {
                debug!("mundo: subcomando {outro} de {roleid} ainda não tratado aqui");
                // **A rede de segurança do original.** Comando de item que o servidor não
                // executa tem de destravar os slots que o cliente congelou ao mandá-lo —
                // é o `UnLockInventoryHandler` (`gs/playercmd.cpp:183-230`, e o `case` de
                // estado inválido em `:654-678`). Sem isto, um comando que falte deixa o
                // item apagado na bolsa até o relogue (B84).
                self.destravar_slots_do_comando(roleid, outro, &cmd.payload, envio).await;
            }
        }
    }

    /// `C2S::PLAYER_MOVE` (0) — o jogador reporta onde está.
    ///
    /// # A causa raiz de "movimento não sincroniza entre jogadores"
    ///
    /// Isto só atualizava o mundo em memória — nunca avisava mais ninguém. O
    /// `pw-link` tinha um mecanismo próprio (`InboundPacket::PlayerMove`/
    /// `OutboundPacket::PlayerMoveBroadcast`, opcode GNET 33) que parecia cobrir isto,
    /// mas **opcode 33 não existe na tabela de protocolos GNET real**
    /// (`specs/protocol/gnet_155.json`) — nunca existiu, e o próprio decodificador do
    /// cliente nunca produz `InboundPacket::PlayerMove` (todo `PLAYER_MOVE` chega como
    /// `GamedataSend`, tratado aqui). Achado e confirmado em 2026-09-05
    /// (`docs/ESTADO_E_RETOMADA.md`, item 23) depois de o Murillo testar em jogo com
    /// dois clients e nada se mover na tela um do outro.
    ///
    /// Corrigido usando o comando real do protocolo, `OBJECT_MOVE` (S2C 15) — 21 bytes,
    /// idêntico no 1.2.6 e no 1.5.3+ (`docs/MEDIDAS_DO_126.md`, o comando mais frequente
    /// da captura inteira, 17294 ocorrências) — mandado pra todo outro jogador conectado
    /// a este mundo (`transmitir_a_outros`).
    async fn mover(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
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
        self.interromper_coleta(roleid).await;
        self.interromper_conjuracao(roleid, 2, envio).await;
        if !self.world.write().await.mover_jogador(roleid, pos) {
            trace!("mundo: movimento de {roleid}, que ainda não tem entidade neste mundo");
        }

        let dest = Vector3::new(m.next_pos.x, m.next_pos.y, m.next_pos.z);
        let pacote = self
            .sub
            .object_move(roleid, dest, m.use_time, m.speed as i16, m.move_mode)
            .data;
        self.transmitir_a_outros(roleid, pacote).await;

        // E o mundo em volta acompanha quem anda.
        self.atualizar_visiveis(roleid, envio, false).await;
    }

    /// `C2S::LOGOUT` (1) — o jogador pediu para sair.
    ///
    /// O mundo não fala com o cliente: quem faz isso é o daemon de link. Então a saída
    /// vira um `PlayerLogout` (69) **no barramento**, e é o link que traduz aquilo no
    /// pacote que o cliente espera. É a mesma divisão do servidor original, e é o que
    /// mantém o formato do cliente fora daqui.
    async fn sair(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        // Golpe que esperava o fim de uma conjuração não sobrevive à saída (B57).
        self.golpe_na_fila.write().await.remove(&roleid);
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
        //
        // Antes disso, porém, quem o via precisa saber que ele foi embora — depois de
        // `remove_player` não há mais como descobrir quem era.
        self.tirar_da_vista_de_todos(roleid as i64).await;
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

    /// `C2S::CALC_NETWORK_DELAY` (128) — o cliente quer medir a latência.
    ///
    /// Devolve o `timestamp` recebido, sem tocar nele: o cliente compara com o que
    /// guardou e descarta a resposta se não bater (`EC_GameRun.cpp:3155`).
    ///
    /// É o comando mais frequente que o servidor ignorava — 191 pedidos numa sessão de
    /// teste de 2026-09-11. O efeito de não responder é só o indicador de ping parado,
    /// mas o custo era um log de "subcomando não tratado" a cada poucos segundos, por
    /// jogador, enterrando o que importa.
    async fn medir_latencia(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        if payload.len() < 4 {
            warn!("mundo: calc_network_delay de {roleid} com {} bytes", payload.len());
            return;
        }
        let timestamp = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        self.responder(
            roleid,
            S2CGamedataSend::calc_network_delay_re(timestamp).data,
            envio,
        )
        .await;
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
                    .map(|(nivel, nivel2, lutando, hp, max_hp, mp, max_mp, alvo)| {
                        self.sub.player_info_00(
                            sel.id, nivel, nivel2, lutando, hp, max_hp, mp, max_mp, alvo,
                        )
                        .data
                    })
            });
        // Quem seleciona recebe a vida agora (`query_info00`, `actobject.cpp:1610`); o
        // batimento só repete quando ela mudar.
        mundo.vida_ja_informada(sel.id as i64);
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
    /// aqui também, um `UPDATE` por parada. Mesma causa raiz e mesma correção do
    /// `mover` acima — `OBJECT_STOP_MOVE` (S2C 35) pra quem mais está neste mundo.
    async fn parar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(m) = StopMove::ler(payload) else {
            warn!("mundo: stop_move de {roleid} com payload curto");
            return;
        };
        let pos = Vector3::new(m.pos.x, m.pos.y, m.pos.z);
        self.world.write().await.mover_jogador(roleid, pos);

        let pacote = self
            .sub
            .object_stop_move(roleid, pos, m.speed as i16, m.dir, m.move_mode)
            .data;
        self.transmitir_a_outros(roleid, pacote).await;

        self.atualizar_visiveis(roleid, envio, false).await;
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
    ///
    /// # A sessão de golpe (B52)
    ///
    /// O comando **não** dá um golpe: abre `session_normal_attack` (`playercmd.cpp:1319-1344`,
    /// `actsession.cpp:350-418`). Com uma sessão aberta, outro `NORMAL_ATTACK` só entra na
    /// fila e não golpeia (`AddSession` devolve `false`, `actobject.cpp:1180-1213`) — era o
    /// "cada clique é um golpe". A sessão confere alvo e alcance (`CheckAttack`,
    /// `actobject.cpp:1220-1252`: distância ≤ `attack_range` + corpo do alvo), manda
    /// `HOST_START_ATTACK` (84), golpeia na hora e repete a cada `attack_speed`; quando o
    /// alvo morre ou sai do alcance, `HOST_STOPATTACK` (23).
    async fn atacar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        // O `force_attack` ainda não muda nada; ler é o que garante que o pacote é o que
        // dizemos que é.
        if NormalAttack::ler(payload).is_none() && !payload.is_empty() {
            warn!("mundo: normal_attack de {roleid} com payload ilegível");
        }

        enum Destino {
            Abrir(i64),
            /// Conjurando: o golpe espera a habilidade acabar (B57).
            Fila(i64),
            Nada,
        }
        let destino = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            // O alvo vem do `SELECT_TARGET` anterior, não do pacote — ver [`NormalAttack`].
            let Some(alvo) = p.target_id else {
                trace!("mundo: {roleid} atacou sem alvo selecionado");
                return;
            };
            if p.conjuracao.is_some() {
                Destino::Fila(alvo)
            } else if let Some(s) = p.ataque.as_mut() {
                trace!("mundo: {roleid} já está golpeando — o NORMAL_ATTACK entra na fila");
                // O golpe novo tira da fila o golpe anterior; cancelar e andar que vieram antes
                // dele são consumidos antes de ele abrir.
                s.proximo = Some(alvo);
                s.cancelar = false;
                s.andar = false;
                Destino::Nada
            } else {
                Destino::Abrir(alvo)
            }
        };
        match destino {
            Destino::Abrir(alvo) => self.abrir_sessao_de_golpe(roleid, alvo, envio).await,
            Destino::Fila(alvo) => {
                trace!("mundo: {roleid} conjura — o NORMAL_ATTACK entra na fila");
                self.golpe_na_fila.write().await.insert(roleid, alvo);
            }
            Destino::Nada => {}
        }
    }

    /// `session_normal_attack::StartSession` (`actsession.cpp:350-378`).
    async fn abrir_sessao_de_golpe(&self, roleid: i32, alvo: i64, envio: &EnvioAoCliente) {
        let inicio = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            if let Err(motivo) = pode_golpear(&mundo, roleid, alvo) {
                debug!("mundo: {roleid} não pode golpear {alvo} (motivo {motivo})");
                return;
            }
            let ticks = ((p.attack_speed * 20.0).round() as u32).clamp(4, 300);
            let arma_de_longe = p.equipamento.arma.is_some_and(|a| a.de_longe);
            // `session_normal_attack::StartSession` → `Notify_StartAttack`
            // (`actsession.cpp:361`): o mascote automático ataca junto.
            mundo.dono_comecou_a_atacar(roleid, alvo);
            if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
                p.ataque = Some(crate::entity::SessaoDeAtaque {
                    alvo,
                    falta_ms: ticks * 50,
                    municao_restante: 0,
                    arma_de_longe,
                    proximo: None,
                    cancelar: false,
                    andar: false,
                });
            }
            (alvo, ticks, arma_de_longe)
        };
        let municao = if inicio.2 {
            self.itens()
                .await
                .get_item_by_slot(roleid, ContainerType::Equipment, 11)
                .await
                .ok()
                .flatten()
                .map(|i| i.count.min(u16::MAX as u32) as u16)
                .unwrap_or(0)
        } else {
            0
        };
        if let Some(s) = self.world.write().await.players.get_mut(&(roleid as i64)).and_then(|p| p.ataque.as_mut()) {
            s.municao_restante = municao;
        }
        self.responder(roleid, S2CGamedataSend::host_start_attack(inicio.0 as i32, municao, inicio.1 as u8).data, envio)
            .await;
        self.golpear(roleid, envio).await;
    }

    /// `cmd_user_move`/`cmd_user_stop_move` põem `session_move` na fila
    /// (`playercmd.cpp:9297-9303`): com golpe em andamento, ele termina no próximo golpe.
    async fn andar_na_fila(&self, roleid: i32) {
        if let Some(s) = self.world.write().await.players.get_mut(&(roleid as i64)).and_then(|p| p.ataque.as_mut()) {
            s.andar = true;
        }
    }

    /// Encerra a sessão de golpe de quem batia num alvo que morreu. O original pega isso no
    /// golpe seguinte (`CheckAttack` com o alvo morto); aqui é na hora, porque o próximo
    /// golpe pode achar o monstro já renascido com o mesmo id.
    pub(crate) async fn encerrar_ataques_ao_alvo(&self, alvo: i64) {
        let quem: Vec<i32> = self
            .world
            .read()
            .await
            .players
            .values()
            .filter(|p| p.ataque.is_some_and(|s| s.alvo == alvo))
            .map(|p| p.role_id)
            .collect();
        for roleid in quem {
            self.encerrar_ataque(roleid, 2).await;
        }
    }

    /// Encerra a sessão de golpe e avisa o cliente (`EndSession` → `stop_attack`).
    pub(crate) async fn encerrar_ataque(&self, roleid: i32, motivo: i32) {
        let tinha = self
            .world
            .write()
            .await
            .players
            .get_mut(&(roleid as i64))
            .and_then(|p| p.ataque.take())
            .is_some();
        if tinha {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::host_stop_attack(motivo).data).await;
        }
    }

    /// Um golpe da sessão (`DoAttack`), depois de `CheckAttack`.
    async fn golpear(&self, roleid: i32, envio: &EnvioAoCliente) {
        let mut mundo = self.world.write().await;
        let Some((alvo, de_longe)) =
            mundo.players.get(&(roleid as i64)).and_then(|p| p.ataque.as_ref()).map(|s| (s.alvo, s.arma_de_longe))
        else {
            return;
        };
        if let Err(motivo) = pode_golpear(&mundo, roleid, alvo) {
            drop(mundo);
            self.encerrar_ataque(roleid, motivo).await;
            return;
        }
        let Some(atacante) = mundo.players.get(&(roleid as i64)).cloned() else {
            return;
        };
        if atacante.efeitos.sem_acao() {
            return;
        }
        let Some((monstro, _)) = mundo.monsters.get(&alvo) else {
            return;
        };
        let velocidade = atraso_do_golpe(atacante.attack_speed);

        // A distância entra no cálculo (atenuação por perto/longe do original); o alvo já
        // foi validado como selecionado, então usar a distância real é o certo.
        let distancia = atacante.position.distance(&monstro.position);
        let resultado = CombatEngine::jogador_ataca_monstro(&atacante, monstro, distancia);
        let (dano, critico) = (resultado.dano() as i64, resultado.foi_critico());

        // A ameaça e o dano são adiados juntos (PostLazyMessage(GM_MSG_GEN_AGGRO, speed + 1)
        // no original, npc.cpp:1867 e npc.cpp:2354-2364): o monstro só reage e persegue
        // quando o projétil/golpe atinge o alvo (aplicar_dano_no_monstro). Chamar add_threat
        // aqui fazia o monstro correr antes da flecha sair (B67).
        // Atacar põe em combate por 15 s (`DoAttack`, `player.cpp:3062`) e **enche o chi**:
        // `if (_ap_per_hit > 0) ModifyAP(_ap_per_hit)` no fim do `DoAttack`
        // (`player.cpp:3091-3093`), com o `ap_per_hit` da classe (`angro_increase`).
        let mut chi_mudou = false;
        // A flecha sai aqui, no `DoAttack` já validado: `_equipment.DecAmount(
        // EQUIP_INDEX_PROJECTILE, 1)` (`player.cpp:3064-3070`). Tirá-la antes das conferências
        // gastava munição em golpe que nem acontecia. `dec_arrow` do original vale **1 sempre
        // que a arma é de longe** — o retorno do `DecAmount` é ignorado (`:3068`) —, por isso o
        // `ATTACK_ONCE` não depende de ter sobrado flecha; só a baixa depende.
        let mut gasta_municao = false;
        if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
            p.combate_s = crate::progressao::COMBATE_AO_ATACAR_S;
            if let Some(s) = p.ataque.as_mut() {
                if s.arma_de_longe && s.municao_restante > 0 {
                    s.municao_restante -= 1;
                    gasta_municao = true;
                }
            }
            if p.ap_por_golpe > 0 {
                chi_mudou = p.mexer_no_chi(p.ap_por_golpe);
            }
        }
        // A vida só cai depois da animação: `InsertDamageEntry(dano, attack.speed)`
        // (`actobject.cpp:1758-1776`) adia o `GM_MSG_HURT` em `attack_delay` tiques de 50 ms
        // — o mesmo número que vai no comando abaixo. Aplicar na hora fazia o monstro perder
        // vida no clique, antes de a flecha sair, e parecia um golpe a mais (B62).
        mundo.adiar_dano(alvo, roleid as i64, dano, velocidade as u32 * 50, false);
        drop(mundo);
        debug!("mundo: golpe normal de {roleid} em {alvo}: dano {dano} (vida cai em {} ms)", velocidade as u32 * 50);
        if chi_mudou {
            // `SetRefreshState()` do `ModifyAP`: o cliente recebe a barra nova.
            self.avisar_vida_propria(roleid).await;
        }
        // `FillAttackMsg` tira a flecha e chama `ATTACK_ONCE` antes do resultado
        // (`player.cpp:3063-3134`). Ambos saem imediatamente: o original altera a
        // `item_list` em memória, portanto latência de persistência nunca alonga a cadência.
        self.responder(roleid, S2CGamedataSend::attack_once(u8::from(de_longe)).data, envio).await;

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
                velocidade,
            )
            .data,
            envio,
        )
        .await;

        // Munição e durabilidade são persistidas depois do fio. Os SQLs decrementam o valor
        // atual de forma atômica; tarefas sobrepostas não perdem golpe. `gastar_arma` ainda
        // notifica a quebra quando o retorno chegar.
        let este = self.clone_arc();
        tokio::spawn(async move {
            let Some(este) = este else { return };
            if gasta_municao {
                let repo = este.itens().await;
                if let Err(e) = repo.consume_item(roleid, ContainerType::Equipment, 11, 1).await {
                    warn!("mundo: não consegui gastar a munição de {roleid}: {e}");
                }
            }
            // A arma se gasta no golpe normal, não na habilidade (`FillAttackMsg` chama
            // `DoWeaponOperation<0>`, `player.cpp:3133`; `FillEnchantMsg`, :3174, não chama).
            este.gastar_arma(roleid).await;
        });

        // 2. A barra de vida do alvo **não** vai aqui: o original a manda no heartbeat de
        // 1 s a quem tem o monstro selecionado (`RefreshSubscibeList`,
        // `actobject.cpp:1346-1353`) — ver [`EventoDoMundo::VidaDoMonstro`]. Mandada junto
        // do golpe, a barra caía no clique, antes de a flecha sair (B56).
        //
        // 3. E a morte também não: quem a resolve é o tique, quando o dano adiado vence
        // (`EventoDoMundo::MonstroMorreu`).
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
        // Meditar liga o `sit_down_filter`: regeneração dobrada a partir do 2º batimento e,
        // no 1.5.5, 15 de chi por batimento (`gs/sitdown_filter.cpp:19-34`) — o chi vem da versão.
        let chi = self.sub.chi_por_meditacao();
        if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
            p.sentado = sentado;
            p.chi_ao_meditar = chi;
        }
        let cmd = if sentado {
            S2CGamedataSend::object_sit_down(roleid)
        } else {
            S2CGamedataSend::object_stand_up(roleid)
        };
        // Os dois comandos carregam o id do jogador justamente porque são sobre o que os
        // **outros** veem. Iam só de volta para quem sentou, e em jogo (2026-09-08) a
        // meditação não aparecia para o outro jogador.
        self.responder(roleid, cmd.data.clone(), envio).await;
        self.transmitir_a_outros(roleid, cmd.data).await;
    }

    /// `EMOTE_ACTION` (48) — o jogador executou um gesto.
    async fn emote(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(e) = EmoteAction::ler(payload) else {
            warn!("mundo: emote de {roleid} com payload curto");
            return;
        };
        // Mesma história da postura: um gesto que só quem gesticulou vê não é um gesto.
        let pacote = S2CGamedataSend::object_do_emote(roleid, e.action).data;
        self.responder(roleid, pacote.clone(), envio).await;
        self.transmitir_a_outros(roleid, pacote).await;
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

        // Carta da Sorte (`TASKDICE_ESSENCE`): entrega uma missão e se gasta.
        if ct == ContainerType::Inventory {
            let carta = self.world.read().await.data_manager.cartas.get(&(u.item_id as u32)).cloned();
            if let Some(carta) = carta {
                self.usar_carta_de_missao(roleid, &u, &carta, envio).await;
                return;
            }
            // Comida de mascote (`PET_FOOD_ESSENCE`, `item_pet_food::OnUse`).
            let comida = self.world.read().await.data_manager.comidas_de_mascote.get(&(u.item_id as u32)).copied();
            if let Some(comida) = comida {
                self.alimentar_mascote(roleid, &u, comida, envio).await;
                return;
            }
            // Caixa de Cartas de General (`POKER_DICE_ESSENCE`): sorteia uma carta e se gasta.
            let caixa = self.world.read().await.data_manager.cartas_de_general.caixas.get(&(u.item_id as u32)).cloned();
            if let Some(caixa) = caixa {
                self.usar_caixa_de_cartas(roleid, &u, &caixa, envio).await;
                return;
            }
        }

        // **Só consumível é consumido.**
        //
        // Este tratamento obedecia ao cliente em tudo: o container e o slot vinham do
        // pacote, e `consume_item` apaga a linha quando a quantidade chega a zero. Em
        // jogo, 2026-09-08, isso **apagou a asa do Sacerdote**: o jogador clicou nela para
        // voar, o cliente mandou `USE_ITEM` apontando para o container de equipamento, e o
        // servidor comeu o item. Não havia log nenhum, porque este caminho só registra
        // falha.
        //
        // Usar um equipamento não é gastá-lo. A regra aqui é a do `elements.data`: se o
        // item não é remédio, nada é consumido — e nunca se mexe no container de
        // equipamento, aconteça o que acontecer.
        let remedio = {
            let mundo = self.world.read().await;
            mundo.data_manager.quanto_o_remedio_restaura_no_tempo(u.item_id as u32)
        };
        let e_consumivel = remedio.is_some();

        // Usar o item do slot de voo é **decolar**, não gastar o item.
        //
        // Não existe comando C2S de decolar: o cliente manda `USE_ITEM` apontando para o
        // slot 12 (`EQUIPIVTR_FLYSWORD`) e espera o `OBJECT_TAKEOFF` (96). Ver
        // `S2CGamedataSend::object_takeoff`.
        if ct == ContainerType::Equipment && u.slot == SLOT_DE_VOO {
            self.alternar_voo(roleid, envio).await;
            return;
        }

        if ct == ContainerType::Equipment || !e_consumivel {
            // Nada é gasto, e — o que importa tanto quanto — **nada de `HOST_USE_ITEM`**.
            // Aquele comando é a confirmação de que o item foi consumido, e o cliente
            // apaga o item da tela ao recebê-lo. Em jogo, 2026-09-08, foi assim que a asa
            // "desapareceu" mesmo continuando no banco.
            debug!(
                "mundo: {roleid} usou o item {} do container {:?}, que não se gasta",
                u.item_id, ct
            );
            return;
        }

        let (hp_total, hp_s, mp_total, mp_s, recarga_ms) = remedio.expect("remédio já conferido");
        let indice_de_recarga = {
            let mundo = self.world.read().await;
            familia_de_recarga(mundo.data_manager.tipo_maior_do_remedio(u.item_id as u32), hp_total, mp_total)
        };

        // O original faz `CheckCoolDown` **antes** de gastar o item e devolve
        // `ERR_OBJECT_IS_COOLING` (53) quando o índice da família ainda está armado
        // (`item_potion.cpp:18-69`). Poções diferentes de vida compartilham o índice 11;
        // as de mana, o 12; as instantâneas de vida+mana, o 3 (`cooldowncfg.h:59-76`).
        let em_recarga = {
            let mundo = self.world.read().await;
            let agora = std::time::Instant::now();
            mundo.players
                .get(&(roleid as i64))
                .and_then(|p| p.recargas.get(&indice_de_recarga))
                .is_some_and(|ate| *ate > agora)
        };
        if em_recarga {
            debug!("mundo: {roleid} tentou usar a poção {} durante a recarga {indice_de_recarga}", u.item_id);
            self.responder(roleid, Self::erro_de_recarga(), envio).await;
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

        if recarga_ms > 0 {
            if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
                p.recargas.insert(
                    indice_de_recarga,
                    std::time::Instant::now() + std::time::Duration::from_millis(recarga_ms as u64),
                );
            }
            self.responder(
                roleid,
                S2CGamedataSend::set_cooldown(indice_de_recarga, recarga_ms).data,
                envio,
            )
            .await;
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

        // Se for remédio, cura pelo que o `elements.data` diz — e **ao longo do tempo**
        // quando o modelo dá um tempo (`hp_add_time`/`mp_add_time`).
        //
        // No original são três itens: `healing_potion` e `mana_potion` criam um filtro que
        // entrega `total / tempo` por batimento de 1 s (`gs/item/item_potion.cpp:18-52`,
        // `gs/potion_filter.h:6-130`), e a `rejuvenation_potion` — a que tem vida **e** mana,
        // sem tempo — cura na hora (`item_potion.cpp:55-70`). Curávamos tudo de uma vez em
        // qualquer caso (B67).
        let curou = {
            let mut mundo = self.world.write().await;
            let n = quantos as i32;
            let mut no_tempo = Vec::new();
            if hp_total > 0 && hp_s > 0 {
                no_tempo.push((crate::efeitos::Efeito::PocaoDeVida, hp_total * n, hp_s));
            }
            if mp_total > 0 && mp_s > 0 {
                no_tempo.push((crate::efeitos::Efeito::PocaoDeMana, mp_total * n, mp_s));
            }
            if no_tempo.is_empty() {
                mundo.curar_jogador(roleid, hp_total * n, mp_total * n)
            } else {
                for (efeito, total, tempo_s) in no_tempo {
                    mundo.pocao_no_tempo(roleid, efeito, total, tempo_s);
                }
                // A vida de agora, que o `SELF_INFO_00` abaixo leva: o primeiro
                // pedaço entra no batimento seguinte, como no original.
                mundo.players.get(&(roleid as i64)).map(|p| (p.hp, p.max_hp, p.mp, p.max_mp))
            }
        };

        if let Some((hp, max_hp, mp, max_mp)) = curou {
            // O `level2` é o **cultivo**, e o cliente toca o efeito de avanço sempre que
            // ele sobe (`SetLevel2` → `CanPlayTaoistEffect`, `EC_Player.cpp:7434-7454`:
            // `originalLevel2 < newLevel2`). Mandar zero aqui derrubava o cultivo para 0 e
            // o `SELF_INFO_00` seguinte, com o valor certo, virava um avanço — era a tela
            // de cultivo ao usar poção (B71).
            let (nivel, cultivo, lutando, exp, sp, ap, max_ap) = {
                let mundo = self.world.read().await;
                mundo
                    .players
                    .get(&(roleid as i64))
                    .map(|p| (p.level, p.cultivation.clamp(0, u8::MAX as i32) as u8, p.combate_s > 0, p.exp, p.sp, p.ap, p.max_ap))
                    .unwrap_or((1, 0, false, 0, 0, 0, 0))
            };
            info!("mundo: {roleid} usou o item {} e ficou com {hp}/{max_hp}", u.item_id);
            self.responder(
                roleid,
                S2CGamedataSend::self_info_00(
                    nivel as i16,
                    cultivo,
                    lutando,
                    hp,
                    max_hp,
                    mp,
                    max_mp,
                    exp as i32,
                    sp as i32,
                    ap,
                    max_ap,
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

        let mut mundo = self.world.write().await;
        // `MODE_INDEX_SILENT`/`STUN`/`SLEEP` (`filter_Sealed`, `filter_Dizzy`, `filter_Sleep`).
        if mundo.players.get(&(roleid as i64)).is_some_and(|p| p.efeitos.selado()) {
            drop(mundo);
            debug!("mundo: {roleid} conjurou {} selado/atordoado — recusado", c.skill_id);
            self.responder(roleid, S2CGamedataSend::self_stop_skill().data, envio).await;
            return;
        }
        let alvo = c
            .alvos
            .first()
            .map(|a| *a as i64)
            .or_else(|| mundo.players.get(&(roleid as i64)).and_then(|p| p.target_id))
            .unwrap_or(roleid as i64);
        // `session_skill::StartSession` → `Notify_StartAttack(_target_list[0])`
        // (`actsession.cpp:491`).
        mundo.dono_comecou_a_atacar(roleid, alvo);

        // A conjuração tem começo e fim, separados pelo tempo de conjuração.
        //
        // `OBJECT_CAST_SKILL` (85) é o começo, e vai para todo mundo: quem conjura monta
        // com ele um `CECHPWorkSpell`, chama `PlaySkillCastAction` e arma um contador com
        // o `time` que mandamos (`EC_HostMsg.cpp:6000-6055`); quem está por perto vê a
        // animação pelo gerente dos outros jogadores.
        drop(mundo);
        // Conjurar encerra o golpe normal em andamento.
        self.encerrar_ataque(roleid, 0).await;
        // `CheckTarget(1.0 + GetPraydistance)` (`skill.cpp:153-157`,
        // `playerwrapper.cpp:1751-1778`): distância < corpo + 1 + alcance da habilidade +
        // corpo do alvo. Para o Arqueiro o alcance é o da arma (`GetRange()`).
        if let Some(longe) = self.alvo_longe_demais(roleid, c.skill_id, alvo).await {
            debug!("mundo: {roleid} conjurou {} a {longe:.1} m do alvo — fora do alcance", c.skill_id);
            self.responder(roleid, S2CGamedataSend::self_stop_skill().data, envio).await;
            return;
        }
        // O tempo de conjuração é o da **habilidade**, não um número fixo. Ver
        // `Habilidade::conjuracao_ms`: vai de 67 ms a 3.000 ms, e mandar 1.000 para todas
        // fazia a cura do Sacerdote sair três vezes mais rápida do que devia.
        // A recarga é conferida e armada antes de a conjuração começar
        // (`SkillWrapper::StartSkill`, `skillwrapper.cpp:261`).
        match self.armar_recarga(roleid, c.skill_id).await {
            Some(None) => {
                debug!("mundo: {roleid} conjurou {} ainda em recarga", c.skill_id);
                self.responder(roleid, Self::erro_de_recarga(), envio).await;
                self.responder(roleid, S2CGamedataSend::self_stop_skill().data, envio).await;
                return;
            }
            Some(Some(ms)) if ms > 0 => {
                self.responder(roleid, S2CGamedataSend::set_cooldown(c.skill_id + 1024, ms).data, envio).await;
            }
            _ => {}
        }
        // `State1::GetTime` do stub, no nível do jogador (`skill.cpp:797`); sem tabela, o
        // valor do `habilidades.rs`.
        let nivel_conjurado = {
            let mundo = self.world.read().await;
            mundo
                .players
                .get(&(roleid as i64))
                .map(|p| nivel_da_habilidade(p, c.skill_id))
                .unwrap_or(NIVEL_MINIMO_DA_HABILIDADE)
        };
        let conjuracao_ms = self
            .world
            .read()
            .await
            .data_manager
            .habilidades
            .get(c.skill_id.max(0) as u32)
            .and_then(|h| h.conjuracao_ms(nivel_conjurado))
            .map(|ms| ms.clamp(0, u16::MAX as i32) as u16)
            .or_else(|| Habilidade::conhecida(c.skill_id).map(|h| h.conjuracao_ms))
            .unwrap_or(TEMPO_DE_CONJURACAO_MS);
        let cast_pkt =
            S2CGamedataSend::object_cast_skill(roleid, alvo as i32, c.skill_id, conjuracao_ms, 1)
                .data;
        self.responder(roleid, cast_pkt.clone(), envio).await;
        self.transmitir_a_outros(roleid, cast_pkt).await;

        // O fim vai numa tarefa própria, depois do tempo de conjuração.
        //
        // Em jogo, 2026-09-08: mandando o fim junto com o começo, quem conjurava não via
        // animação nenhuma (o `HOST_STOP_SKILL` cancelava o trabalho de feitiço no mesmo
        // quadro em que ele nascia) enquanto o outro jogador via a conjuração inteira —
        // porque o 123 não é transmitido aos outros. Esperar aqui na própria mensagem não
        // serve: uma conexão de barramento carrega **vários** jogadores, e dormir nela
        // travaria todo mundo por um segundo.
        let marcador = PROXIMA_CONJURACAO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
            p.conjuracao = Some(crate::entity::Conjuracao {
                skill_id: c.skill_id,
                alvo,
                inicio: std::time::Instant::now(),
                duracao_ms: conjuracao_ms as u32,
                marcador,
            });
        }
        let este = self.clone();
        let envio = envio.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(conjuracao_ms as u64)).await;
            // Só conclui se esta ainda for a conjuração aberta: soltar a carga já pode ter
            // concluído, ou outra conjuração pode ter começado.
            let minha = {
                let mut mundo = este.world.write().await;
                let p = mundo.players.get_mut(&(roleid as i64));
                // **Não** tira a conjuração aqui: ela fica aberta até o dano ser aplicado,
                // como a `session_skill` do original, que só é a sessão corrente até o
                // `RunSkill` (B57). É o que põe na fila o `NORMAL_ATTACK` que o cliente manda
                // assim que vê a conjuração acabar.
                match p {
                    Some(p) if p.conjuracao.is_some_and(|x| x.marcador == marcador) => p.conjuracao,
                    _ => None,
                }
            };
            if let Some(c) = minha {
                este.concluir_conjuracao(roleid, c.skill_id, c.alvo, &envio, 1.0).await;
            }
        });
    }

    /// `C2S::CONTINUE_ACTION` (51): soltar uma habilidade de carga antes do fim. O tempo
    /// carregado vira `GetCharging()` (`SkillWrapper::Continue`, `skillwrapper.cpp:310-333`),
    /// que escala o `ratio` (ex.: `skill234.h`, `ratio × charging / (5200 − 200 × nível)`).
    async fn soltar_carga(&self, roleid: i32, envio: &EnvioAoCliente) {
        let solta = {
            let mut mundo = self.world.write().await;
            let dados = Arc::clone(&mundo.data_manager);
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            let de_carga = p
                .conjuracao
                .and_then(|c| dados.habilidades.get(c.skill_id.max(0) as u32))
                .is_some_and(|h| h.e_de_carga());
            if de_carga { p.conjuracao } else { None }
        };
        let Some(c) = solta else {
            trace!("mundo: {roleid} mandou CONTINUE_ACTION sem carga aberta");
            return;
        };
        let carga = if c.duracao_ms == 0 {
            1.0
        } else {
            (c.inicio.elapsed().as_millis() as f32 / c.duracao_ms as f32).min(1.0)
        };
        debug!("mundo: {roleid} soltou a {} com {:.0}% de carga", c.skill_id, carga * 100.0);
        self.concluir_conjuracao(roleid, c.skill_id, c.alvo, envio, carga).await;
    }

    /// Fecha a conjuração aberta e solta o golpe normal que esperava a vez.
    ///
    /// No original é o `SafeDeleteCurSession` → `StartSession` (`actobject.cpp:150-190`): a
    /// sessão da habilidade sai e a próxima da fila começa — e o `CheckAttack` dela recusa
    /// alvo morto, que é o que impede o golpe de cair junto com o dano da habilidade (B57).
    async fn fechar_conjuracao_e_soltar_fila(&self, roleid: i32, envio: &EnvioAoCliente) {
        if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
            p.conjuracao = None;
        }
        let Some(alvo) = self.golpe_na_fila.write().await.remove(&roleid) else { return };
        let atacando = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .is_some_and(|p| p.ataque.is_some());
        if !atacando {
            self.abrir_sessao_de_golpe(roleid, alvo, envio).await;
        }
    }

    /// Interrompe uma conjuração aberta do jogador (player.cpp:4017-4028).
    /// Envia SELF_SKILL_INTERRUPTED (87) para o jogador (fecha a barra de cast)
    /// e SKILL_INTERRUPTED (86) para outros jogadores ao redor.
    /// Corta a conjuração em andamento. `motivo` 2 é o movimento.
    ///
    /// **Habilidade de conjurar andando não é cortada pelo movimento**: o original a
    /// despacha por `moving_skill` em vez de `session_skill` justamente para isso
    /// (`gs/playercmd.cpp:2066-2088`), e só o `moving_skill_interrupt_filter` a encerra.
    /// No 1.5.5 são cinco habilidades, todas da classe 11 — a do Tormentador (B78).
    async fn interromper_conjuracao(&self, roleid: i32, motivo: u8, envio: &EnvioAoCliente) -> bool {
        const POR_MOVIMENTO: u8 = 2;
        let mut mundo = self.world.write().await;
        let dados = Arc::clone(&mundo.data_manager);
        let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return false; };
        if motivo == POR_MOVIMENTO
            && p.conjuracao
                .as_ref()
                .and_then(|c| dados.habilidades.get(c.skill_id.max(0) as u32))
                .is_some_and(|h| h.conjura_andando())
        {
            return false;
        }
        if p.conjuracao.take().is_some() {
            drop(mundo);
            self.responder(roleid, self.sub.self_skill_interrupted(motivo).data, envio).await;
            self.transmitir_a_outros(roleid, self.sub.skill_interrupted(roleid).data).await;
            return true;
        }
        false
    }

    /// `ACTIVATE_REGION_WAYPOINTS` (C2S 178) — o cliente lista os pontos de teleporte da
    /// região onde está, e o servidor ativa os que ainda não são dele.
    ///
    /// `gplayer_imp::ActivateRegionWaypoints` (`gs/player.cpp:25196-25220`) cruza o que
    /// chegou com os pontos daquela região e chama `ActivateWaypoint` para cada um
    /// (`player_imp.h:2534-2544`), que **só faz algo se o ponto for novo**: guarda em
    /// `_waypoint_list` e manda `activate_waypoint` (179) — o comando que faz o cliente
    /// escrever "novo ponto de teleporte" com o nome do lugar.
    ///
    /// `falta`: a tabela por região (`world_manager::GetRegionWaypoints`), que vive num
    /// arquivo do servidor original que ainda não lemos. Sem ela aceitamos o que o cliente
    /// diz haver na região dele — que é o que ele já sabe pelo mapa local.
    async fn ativar_waypoints(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        // `{ unsigned char num; int waypoints[num]; }` — o corpo vem sem o cabeçalho.
        let Some(&num) = payload.first() else { return };
        let mut novos = Vec::new();
        {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            for i in 0..num as usize {
                let off = 1 + i * 4;
                let Some(b) = payload.get(off..off + 4) else { break };
                // `waypoints[i] & 0xFFFF` (`player.cpp:25215`).
                let wp = (i32::from_le_bytes(b.try_into().unwrap()) & 0xFFFF) as u16;
                if !p.waypoints.contains(&wp) {
                    p.waypoints.push(wp);
                    novos.push(wp);
                }
            }
        }
        if novos.is_empty() {
            return;
        }
        for wp in &novos {
            self.responder(roleid, S2CGamedataSend::activate_waypoint(*wp).data, envio).await;
        }
        let lista = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .map(|p| p.waypoints.clone())
            .unwrap_or_default();
        info!("mundo: {roleid} descobriu {} ponto(s) de teleporte: {novos:?}", novos.len());
        if let Err(e) = self.repo().await.salvar_waypoints(roleid, &lista).await {
            warn!("mundo: não consegui gravar os pontos de teleporte de {roleid}: {e}");
        }
    }

    /// `Some(distância)` quando o alvo está além de `corpo + 1 + GetPraydistance + corpo do
    /// alvo`; `None` quando está ao alcance, é o próprio jogador, ou não há dado de alcance.
    async fn alvo_longe_demais(&self, roleid: i32, skill_id: i32, alvo: i64) -> Option<f32> {
        if alvo == roleid as i64 {
            return None;
        }
        let mundo = self.world.read().await;
        let p = mundo.players.get(&(roleid as i64))?;
        let h = mundo.data_manager.habilidades.get(skill_id.max(0) as u32)?;
        let alcance = h.alcance(nivel_da_habilidade(p, skill_id), p.attack_range)?;
        let (pos, corpo) = if let Some((m, _)) = mundo.monsters.get(&alvo) {
            (m.position, mundo.data_manager.monstros.get(m.template_id).map(|t| t.tamanho).unwrap_or(0.0))
        } else if let Some(o) = mundo.players.get(&alvo) {
            (o.position, crate::entity::CORPO_DO_JOGADOR)
        } else {
            return None;
        };
        let d = p.position.distance(&pos);
        (d >= crate::entity::CORPO_DO_JOGADOR + 1.0 + alcance + corpo).then_some(d)
    }

    /// O fim de uma conjuração: solta o conjurador e aplica o efeito.
    ///
    /// Roda depois do tempo de conjuração — ver [`Self::conjurar`].
    async fn concluir_conjuracao(
        &self,
        roleid: i32,
        skill_id: i32,
        alvo: i64,
        envio: &EnvioAoCliente,
        carga: f32,
    ) {
        // Se o jogador saiu no meio da conjuração, não há a quem responder.
        if !self.sessoes.read().await.contains_key(&roleid) {
            return;
        }

        // `gplayer_dispatcher::skill_perform` envia só ao dono; o broadcast do original
        // está comentado (`gs/player.cpp:4066-4073`). O 88 recebido por outro jogador
        // altera o estado da habilidade *dele* (`EC_HostMsg.cpp:5929-5937`).
        self.responder(roleid, S2CGamedataSend::skill_perform().data, envio).await;

        // O efeito da habilidade vem **antes** do fim da sessão, como no original: o dano sai
        // do `RunSkill` (`session_skill::RepeatSession`, `actsession.cpp:576-600`) e só depois
        // o `EndSession` manda `stop_skill` (`actsession.cpp:558-574`). Mandando o 123 antes,
        // o cliente retomava o golpe normal e os dois danos caíam juntos (B57).
        self.aplicar_conjuracao(roleid, skill_id, alvo, envio, carga).await;

        // `HOST_STOP_SKILL` (123) é o que **fecha a conjuração de quem conjurou**, e é
        // sem corpo (o cliente exige `dwSize == 0`, `EC_GameDataPrtc.cpp:305`).
        //
        // O `SKILL_PERFORM` (88) acima não serve para isso: ele é roteado para
        // `MAN_PLAYER` (`EC_GameDataPrtc.cpp:1385-1388`), o gerente dos **outros**
        // jogadores. Quem conjurou só é solto pelo `case HOST_STOP_SKILL`
        // (`EC_HostMsg.cpp:6065-6096`), que zera `m_pCurSkill`, chama `EndCharging()`,
        // `StopSkillAttackAction()` e encerra o trabalho de feitiço. Sem ele o cliente
        // fica em estado de conjuração para sempre e recusa a próxima habilidade.
        //
        // Vai **antes** de qualquer coisa depender do alvo: uma cura em si mesmo, uma
        // bênção num companheiro e um ataque num monstro terminam todos aqui.
        //
        // **Mas só depois da fase de execução** (B67): a sessão do original é um laço de
        // estados — `StartSkill` dá o tempo do primeiro, `RunSkill` o do seguinte, e o
        // `EndSession` (que manda o `stop_skill`) só vem quando não há próximo
        // (`gs/actsession.cpp:466-600`). A Flecha Fulgurante tem 3.000 ms de conjuração e
        // **800 ms de execução** (`GetExecutetime`, `cskill/skills/skill244.h:20-80`); é
        // nessa fase que o cliente anima o personagem recebendo a bênção. Mandando o 123
        // logo depois do efeito, a animação era cortada e só o ícone do buff aparecia.
        let nivel_conjurado = {
            let mundo = self.world.read().await;
            mundo.players.get(&(roleid as i64)).map(|p| nivel_da_habilidade(p, skill_id)).unwrap_or(1)
        };
        let execucao_ms = self
            .world
            .read()
            .await
            .data_manager
            .habilidades
            .get(skill_id.max(0) as u32)
            .and_then(|h| h.fase_de_execucao_ms(nivel_conjurado))
            .unwrap_or(0)
            .clamp(0, 10_000);
        if execucao_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(execucao_ms as u64)).await;
            if !self.sessoes.read().await.contains_key(&roleid) {
                return;
            }
        }
        self.responder(roleid, S2CGamedataSend::self_stop_skill().data, envio)
            .await;
        self.fechar_conjuracao_e_soltar_fila(roleid, envio).await;
    }

    /// O efeito da habilidade em si (dano, cura, estados), entre o `SKILL_PERFORM` e o
    /// `HOST_STOP_SKILL` — ver [`Self::concluir_conjuracao`].
    async fn aplicar_conjuracao(
        &self,
        roleid: i32,
        skill_id: i32,
        alvo: i64,
        envio: &EnvioAoCliente,
        carga: f32,
    ) {
        let mut mundo = self.world.write().await;
        let Some(atacante) = mundo.players.get(&(roleid as i64)).cloned() else {
            return;
        };
        let habilidade = Habilidade::conhecida(skill_id);

        // O nível em que **este jogador** tem esta habilidade. Vem do `character_skills`,
        // carregado no login: o cliente não manda o nível no `CAST_SKILL`.
        let nivel = nivel_da_habilidade(&atacante, skill_id);

        // Custo de mana. O cliente já confere antes de mandar (`ElementSkill::Condition`
        // devolve 2 quando falta), então chegar aqui sem mana é raro — mas o servidor não
        // pode acreditar no cliente, e sem cobrar a mana nunca acabaria.
        let custo_do_stub = mundo
            .data_manager
            .habilidades
            .get(skill_id.max(0) as u32)
            .and_then(|h| h.mana(nivel))
            .map(|m| m.round() as i32);
        if let Some(custo) = custo_do_stub.or_else(|| habilidade.map(|h| h.custo_de_mp(nivel))) {
            let tem = mundo.players.get(&(roleid as i64)).map(|p| p.mp).unwrap_or(0);
            if tem < custo {
                debug!("mundo: {roleid} conjurou {skill_id} com {tem} de mana, precisa de {custo}");
                return;
            }
            if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
                p.mp -= custo;
            }
        }

        // Chi. Cada habilidade tem um `apcost` e um `apgain` fixos no stub
        // (`cskill/skill/skill.h:239,588`): `SkillStub::Condition` recusa com
        // `GetAp() < apcost` (`skill.cpp:125`) — sem mandar erro, porque o cliente já
        // barra —, e a execução aplica a diferença de uma vez:
        // `int ap = GetApgain() - GetApcost(); if (ap) ModifyAP(ap)`
        // (`playerwrapper.cpp:170-177`). A Flecha Glacial (245) custa 25, a Barreira de
        // Asa (249) custa 45, a Flecha Fulgurante (244) **dá** 10 e a 235 dá 5.
        let (apcost, apgain) = mundo
            .data_manager
            .habilidades
            .get(skill_id.max(0) as u32)
            .map(|h| (h.apcost.unwrap_or(0).max(0), h.apgain.unwrap_or(0).max(0)))
            .unwrap_or((0, 0));
        let chi_mudou = {
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            if p.ap < apcost {
                debug!("mundo: {roleid} conjurou {skill_id} com {} de chi, precisa de {apcost}", p.ap);
                return;
            }
            let delta = apgain - apcost;
            delta != 0 && p.mexer_no_chi(delta)
        };
        if chi_mudou {
            // `SetRefreshState()` do `ModifyAP`: a barra nova vai ao cliente.
            let dados = mundo.dados_do_proprio(roleid);
            drop(mundo);
            if let Some((nivel, nivel2, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap)) = dados {
                self.responder(
                    roleid,
                    S2CGamedataSend::self_info_00(nivel, nivel2, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap).data,
                    envio,
                )
                .await;
            }
            mundo = self.world.write().await;
        }

        // B53: a habilidade pelo stub (área, flechas, precisão, efeitos), quando ele tem o que
        // fazer; senão o caminho antigo.
        drop(mundo);
        if self.aplicar_habilidade(roleid, skill_id, alvo, nivel, carga, envio).await {
            return;
        }
        let mut mundo = self.world.write().await;
        if !mundo.monsters.contains_key(&alvo) {
            // Alvo que não é monstro: o próprio conjurador, ou outro jogador.
            drop(mundo);
            self.efeito_em_jogador(roleid, skill_id, alvo, habilidade, envio)
                .await;
            return;
        }
        let monstro = &mundo.monsters[&alvo].0;
        if monstro.is_dead {
            return;
        }

        // Contra monstro, a habilidade com conta portada usa a conta dela; as outras
        // batem como um golpe básico, que é o que este tratamento fazia para todas.
        let distancia = atacante.position.distance(&monstro.position);
        // A conta do stub do servidor, quando extraída (1.123 habilidades); senão a tabela
        // antiga; senão um golpe normal.
        let do_stub = mundo
            .data_manager
            .habilidades
            .get(skill_id.max(0) as u32)
            .and_then(|h| h.dano.clone())
            .and_then(|d| CombatEngine::golpe_de_habilidade(&atacante, &d, nivel, carga));
        let dano = if let Some(g) = do_stub {
            combat::resolver(&g, &CombatEngine::defesa_do_monstro(monstro), distancia, false, combat::Rolagens::sortear()).dano() as i64
        } else { match habilidade.and_then(|h| {
            h.dano(
                nivel,
                (atacante.attack_min + atacante.attack_max) / 2,
                (atacante.magic_attack_min + atacante.magic_attack_max) / 2,
            )
        }) {
            Some(d) => {
                let reducao = combat::reducao_por_defesa(monstro.def_phys, atacante.level);
                (((d as f32) * (1.0 - reducao)).round() as i64).max(1)
            }
            None => CombatEngine::jogador_ataca_monstro(&atacante, monstro, distancia).dano() as i64,
        } };
        let (hp, max_hp, morreu, template, exp, sp) = {
            let (m, ai) = mundo.monsters.get_mut(&alvo).expect("conferido acima");
            ai.add_threat(roleid as i64, dano);
            let real = dano.min(m.hp);
            m.hp = (m.hp - dano).max(0);
            m.registrar_dano(roleid as i64, real);
            let morreu = m.hp == 0;
            if morreu {
                m.is_dead = true;
            }
            (m.hp, m.max_hp, morreu, m.template_id, m.exp, m.sp)
        };
        if morreu {
            mundo.grid.remove_entity(alvo);
        }
        if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
            p.combate_s = crate::progressao::COMBATE_AO_ATACAR_S;
        }
        drop(mundo);
        let _ = (template, exp, sp);

        self.responder(
            roleid,
            self.sub.self_skill_attack_result(
                alvo as i32,
                skill_id,
                saturar(dano),
                SEM_MARCACAO,
                VELOCIDADE_PADRAO,
                SECAO_UNICA,
            )
                .data,
            envio,
        )
        .await;
        // A barra de vida vai no batimento de 1 s ([`EventoDoMundo::VidaDoMonstro`]), como o
        // golpe normal (B56).
        debug!("mundo: habilidade {skill_id} de {roleid} em {alvo}: dano {dano}, vida {hp}/{max_hp}");

        if morreu {
            info!("mundo: {roleid} matou {alvo} com a habilidade {}", skill_id);
            let morte = S2CGamedataSend::npc_died(alvo as i32, roleid).data;
            self.responder(roleid, morte.clone(), envio).await;
            self.transmitir_a_outros(roleid, morte).await;
            self.monstro_morreu(alvo).await;
        }
    }

    /// `C2S::SWITCH_FASHION_MODE` (85) — o botão que alterna entre mostrar a armadura e
    /// mostrar a roupa.
    ///
    /// O comando vem **sem corpo**: o cliente não decide nada sozinho, ele pede a troca e
    /// espera o servidor dizer qual é o estado novo. Quem guarda o estado é o mundo, e a
    /// resposta é o `PLAYER_ENABLE_FASHION` (192) — que vai para todo mundo, inclusive
    /// para quem apertou o botão, porque é assim que o cliente descobre o resultado
    /// (`EC_ManPlayer.cpp:1355-1358` acha o jogador pelo `idPlayer` do corpo).
    ///
    /// Em jogo, 2026-09-08: "mudei para modo roupa, não sincronizou para o outro jogador,
    /// e ao clicar de novo não voltou". Os dois sintomas são o mesmo defeito — o comando
    /// caía no `outro =>` silencioso deste `match` e nada acontecia, nem no log.
    async fn trocar_modo_roupa(&self, roleid: i32, envio: &EnvioAoCliente) {
        let ativo = {
            let mut mundo = self.world.write().await;
            let Some(jogador) = mundo.players.get_mut(&(roleid as i64)) else {
                warn!("mundo: {roleid} pediu modo roupa mas não está no mundo");
                return;
            };
            jogador.modo_roupa = !jogador.modo_roupa;
            jogador.modo_roupa
        };

        debug!("mundo: {roleid} passou para o modo {}", if ativo { "roupa" } else { "armadura" });

        // A escolha vai ao banco, senão morre no logout — e a **tela de seleção** lê o
        // `charactermode` de lá para desenhar o avatar (`CECLoginPlayer::Load`,
        // `EC_LoginPlayer.cpp:172-189`). Fora do fio do jogo, pela regra de nunca esperar o
        // banco no caminho do comando: são 8 bytes e ninguém depende do resultado.
        let repo = self.repo().await;
        tokio::spawn(async move {
            if let Err(e) = repo.salvar_modo_roupa(roleid, ativo).await {
                warn!("mundo: não gravei o modo roupa de {roleid}: {e}");
            }
        });

        let pacote = S2CGamedataSend::player_enable_fashion(roleid, ativo).data;
        self.responder(roleid, pacote.clone(), envio).await;
        self.transmitir_a_outros(roleid, pacote).await;
    }

    /// O efeito de uma habilidade cujo alvo é um jogador — o próprio conjurador ou outro.
    ///
    /// # Cura
    ///
    /// A conta é a do stub (ver [`crate::habilidades`]). O alvo recebe `SELF_INFO_00`,
    /// que é o comando que sincroniza a própria vida (o mesmo que a poção usa), e o
    /// conjurador recebe `HOST_SKILL_ATTACK_RESULT` (142) com o valor curado — é assim
    /// que o número aparece na tela.
    ///
    /// # Dano entre jogadores
    ///
    /// Funciona, e **não há trava de PvP**. O original só deixa um jogador machucar o
    /// outro em modo de duelo, facção em guerra ou mapa de PK (`pvp_mode`, o comando 79);
    /// nada disso existe aqui ainda, então qualquer um pode acertar qualquer um. Está
    /// escrito para ninguém descobrir isso em produção.
    ///
    /// O alvo recebe `HOST_SKILL_ATTACKED` (144), sem o qual ele não toca efeito nenhum
    /// nem entra em combate, mais o `SELF_INFO_00` com a vida nova. Quem está por perto
    /// recebe `OBJECT_SKILL_ATTACK_RESULT` (143).
    async fn efeito_em_jogador(
        &self,
        roleid: i32,
        skill_id: i32,
        alvo: i64,
        habilidade: Option<&'static Habilidade>,
        envio: &EnvioAoCliente,
    ) {
        let Some(h) = habilidade else {
            debug!("mundo: {roleid} conjurou {skill_id}, que não tem conta portada — sem efeito");
            return;
        };

        let (valor, alvo_vivo) = {
            let mundo = self.world.read().await;
            let Some(conjurador) = mundo.players.get(&(roleid as i64)) else {
                return;
            };
            let Some(vitima) = mundo.players.get(&alvo) else {
                debug!("mundo: {roleid} conjurou {skill_id} em {alvo}, que não é jogador deste mundo");
                return;
            };
            if vitima.hp <= 0 {
                return;
            }
            let magico = (conjurador.magic_attack_min + conjurador.magic_attack_max) / 2;
            let fisico = (conjurador.attack_min + conjurador.attack_max) / 2;
            // O nível em que o **conjurador** tem esta habilidade — não o do alvo, e não
            // o 1 fixo que valia para todo mundo até 2026-09-09.
            let nivel = nivel_da_habilidade(conjurador, skill_id);
            let valor = if h.e_cura() {
                h.cura(nivel, magico).unwrap_or(0)
            } else {
                let bruto = h.dano(nivel, fisico, magico).unwrap_or(0);
                let reducao = combat::reducao_por_defesa(vitima.def_phys, conjurador.level);
                (((bruto as f32) * (1.0 - reducao)).round() as i32).max(1)
            };
            (valor, true)
        };
        if !alvo_vivo {
            return;
        }

        // Aplica no mundo e devolve a vida nova do alvo.
        let estado = {
            let mut mundo = self.world.write().await;
            let Some(vitima) = mundo.players.get_mut(&alvo) else {
                return;
            };
            if h.e_cura() {
                vitima.hp = (vitima.hp + valor).min(vitima.max_hp);
            } else {
                vitima.hp = (vitima.hp - valor).max(0);
            }
            (
                vitima.hp,
                vitima.max_hp,
                vitima.mp,
                vitima.max_mp,
                vitima.level,
                vitima.cultivation.clamp(0, u8::MAX as i32) as u8,
                vitima.combate_s > 0,
                vitima.exp,
                vitima.sp,
                vitima.ap,
                vitima.max_ap,
            )
        };
        let (hp, max_hp, mp, max_mp, nivel, cultivo, lutando, exp, sp, ap, max_ap) = estado;

        info!(
            "mundo: {roleid} conjurou {skill_id} em {alvo} — {} de {}, alvo com {hp}/{max_hp}",
            valor,
            if h.e_cura() { "cura" } else { "dano" }
        );

        // O número na tela.
        //
        // Cura **não** vai pelo `HOST_SKILL_ATTACK_RESULT` (142): aquele caminho termina
        // em `CECPlayer::Damaged`, que só sabe desenhar `BUBBLE_DAMAGE` — vermelho — ou
        // "errou" (`EC_Player.cpp:3459-3489`). Em jogo, 2026-09-08, a Prece da Clareza
        // curava de verdade no servidor e aparecia como 35 de dano na tela.
        //
        // O número verde é outro comando: `PLAYER_HP_STEAL` (279), que o cliente traduz em
        // `BubbleText(BUBBLE_ADD, hp)` (`EC_HostMsg.cpp:5772-5781`). Ele vai para **quem
        // recebeu** a cura, que é quem vê o número subir.
        if h.e_cura() {
            // Duas coisas, e cada uma resolve metade do problema.
            //
            // O número verde é o `PLAYER_HP_STEAL` (279), que vira
            // `BubbleText(BUBBLE_ADD, hp)` (`EC_HostMsg.cpp:5772-5781`).
            let verde = S2CGamedataSend::player_hp_steal(valor).data;
            if alvo as i32 == roleid {
                self.responder(roleid, verde, envio).await;
            } else {
                let _ = self.enviar_ao_jogador(alvo as i32, verde).await;
            }

            // O **efeito visual** (a luz sobre o alvo) é a máquina de ataque de
            // habilidade do cliente, e ela só roda com o `HOST_SKILL_ATTACK_RESULT`
            // (142). Mandá-lo com o dano de verdade pintava o número em vermelho; o
            // valor certo é **-2**, que `CECPlayer::Damaged` trata como "isto veio de uma
            // habilidade de ajuda": não desenha número, não toca animação de ferido, e
            // deixa o efeito da habilidade acontecer (`EC_Player.cpp:3435-3443`).
            //
            // Sem isto a cura funcionava e não aparecia nada — relatado em jogo em
            // 2026-09-08 ("casta mas o efeito não ativa no final").
            const DANO_DE_HABILIDADE_DE_AJUDA: i32 = -2;
            self.responder(
                roleid,
                self.sub
                    .self_skill_attack_result(
                        alvo as i32,
                        skill_id,
                        DANO_DE_HABILIDADE_DE_AJUDA,
                        SEM_MARCACAO,
                        VELOCIDADE_PADRAO,
                        SECAO_UNICA,
                    )
                    .data,
                envio,
            )
            .await;
            self.transmitir_a_outros(
                roleid,
                self.sub
                    .object_skill_attack_result(
                        roleid,
                        alvo as i32,
                        skill_id,
                        DANO_DE_HABILIDADE_DE_AJUDA,
                        SEM_MARCACAO,
                        VELOCIDADE_PADRAO,
                        SECAO_UNICA,
                    )
                    .data,
            )
            .await;
        } else {
            self.responder(
                roleid,
                self.sub
                    .self_skill_attack_result(
                        alvo as i32,
                        skill_id,
                        saturar(valor as i64),
                        SEM_MARCACAO,
                        VELOCIDADE_PADRAO,
                        SECAO_UNICA,
                    )
                    .data,
                envio,
            )
            .await;
        }

        // A vida nova do alvo, e — se doeu — o efeito de ter sido acertado.
        let alvo_id = alvo as i32;
        if alvo_id != roleid && !h.e_cura() {
            let _ = self.enviar_ao_jogador(
                alvo_id,
                self.sub.host_skill_attacked(
                    roleid,
                    skill_id,
                    saturar(valor as i64),
                    SEM_MARCACAO,
                    VELOCIDADE_PADRAO,
                    SECAO_UNICA,
                )
                .data,
            )
            .await;
        }
        // O cultivo verdadeiro, não zero: ver `EC_Player.cpp:7434-7454` (B71).
        let vida = S2CGamedataSend::self_info_00(
            nivel as i16,
            cultivo,
            lutando,
            hp,
            max_hp,
            mp,
            max_mp,
            exp as i32,
            sp as i32,
            ap,
            max_ap,
        )
        .data;
        if alvo_id == roleid {
            self.responder(roleid, vida, envio).await;
        } else {
            let _ = self.enviar_ao_jogador(alvo_id, vida).await;
        }

        // Quem está por perto vê o número entre os dois. A cura já mandou o seu 143 com
        // dano -2 no bloco acima.
        if h.e_cura() {
            return;
        }
        self.transmitir_a_outros(
            roleid,
            self.sub
                .object_skill_attack_result(
                    roleid,
                    alvo_id,
                    skill_id,
                    saturar(valor as i64),
                    SEM_MARCACAO,
                    VELOCIDADE_PADRAO,
                    SECAO_UNICA,
                )
                .data,
        )
        .await;
    }

    /// `C2S::GOTO` (19) — o Ctrl+clique do GM: "me ponha nesse ponto do mapa".
    ///
    /// `struct cmd_goto { A3DVECTOR3 vDest; }` — 12 bytes, só o destino. O cliente **não**
    /// se move sozinho: ele pede e espera o servidor confirmar com `HOST_CORRECT_POS`
    /// (177). Sem resposta, nada acontece — que é o que se via em jogo, com o comando
    /// aparecendo no log como "subcomando 19 ainda não tratado".
    ///
    /// # Por que é só para GM
    ///
    /// Porque é teleporte livre. `is_gm` vem do banco (a coluna que o `sec_level` do login
    /// já usa); um jogador comum que mande este comando à mão é recusado e fica no log.
    async fn teleportar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        if payload.len() < 12 {
            warn!("mundo: goto de {roleid} com payload curto ({} bytes)", payload.len());
            return;
        }
        let f = |i: usize| f32::from_le_bytes([payload[i], payload[i + 1], payload[i + 2], payload[i + 3]]);
        let destino = pw_core::Vector3::new(f(0), f(4), f(8));

        let nivel_de_gm = self.repo().await.nivel_de_gm(roleid).await;
        if nivel_de_gm <= 0 {
            warn!("mundo: {roleid} pediu teleporte para {destino:?} sem ser GM");
            return;
        }

        // **O `y` do cliente não é a altura do chão.**
        //
        // Os cliques de mapa mandam `y = 1.0` como marcador (`c2s_CmdGoto(fX, 1.0f, fZ)`
        // em `DlgWorldMap.cpp:1174`, `DlgRandomMap.cpp:262`, `DlgCountryWarMap.cpp:330`), e
        // o servidor original **substitui** o campo pela altura do terreno:
        //
        // ```cpp
        // pos.y = pImp->_plane->GetHeightAt(pos.x, pos.z);
        // pImp->PlayerGoto(pos);
        // ```
        //
        // (`EvolvedPWServer/cgame/gs/playercmd.cpp:4926-4927`.) Obedecer ao `y` recebido
        // punha o personagem dentro do chão, e foi o que se viu em jogo (2026-09-08).
        //
        // O item 37 não tinha o mapa e escolheu **manter a altura atual do jogador** —
        // remendo que funciona em terreno plano e falha em qualquer encosta. Em 2026-09-11
        // o Murillo relatou o enterro de volta, e o mapa de alturas passou a existir
        // (`pw_data_loader::terreno`). Agora a altura sai do `.hmap`, como no original.
        //
        // Sem terreno (mapa fora do catálogo, pasta sem `map/`) o remendo antigo continua
        // valendo: é pior do que a altura de verdade e melhor do que enterrar.
        let destino = {
            let mut mundo = self.world.write().await;
            let altura_do_chao = mundo.terreno.altura_em(destino.x, destino.z);
            let Some(jogador) = mundo.players.get_mut(&(roleid as i64)) else {
                return;
            };
            let y = match altura_do_chao {
                Some(chao) => chao + FOLGA_AO_TELEPORTAR,
                None => {
                    debug!(
                        "mundo: sem altura de chão em ({:.0}, {:.0}) — o teleporte de \
                         {roleid} mantém a altura atual",
                        destino.x, destino.z
                    );
                    jogador.position.y
                }
            };
            let destino = pw_core::Vector3::new(destino.x, y, destino.z);
            jogador.position = destino;
            mundo.grid.update_position(roleid as i64, destino);
            destino
        };
        // O `stamp` é o contador de correções que o cliente usa para descartar correção
        // fora de ordem. Zero enquanto só há uma correção em voo por vez.
        let stamp = 0;

        info!("mundo: GM {roleid} se teleportou para {destino:?}");
        self.responder(
            roleid,
            S2CGamedataSend::host_correct_pos(destino, stamp).data,
            envio,
        )
        .await;
        // Quem está por perto precisa ver o corpo mudar de lugar.
        self.transmitir_a_outros(
            roleid,
            self.sub
                .object_stop_move(roleid, destino, 0, 0, MODO_DE_MOVIMENTO_ANDANDO)
                .data,
        )
        .await;

        // E o jogador precisa ver o que existe onde chegou. O teleporte é justamente o
        // caso em que a histerese de [`Self::atualizar_visiveis`] não atrapalha: a
        // distância percorrida é sempre maior do que o passo mínimo.
        self.atualizar_visiveis(roleid, envio, true).await;
    }

    /// Manda o que entrou no alcance do jogador e retira o que saiu.
    ///
    /// # O problema que isto resolve
    ///
    /// Até 2026-09-09 os NPCs eram mandados **uma vez só**, no login, num raio em volta da
    /// posição de entrada. Não havia streaming: o cliente descarta sozinho o que sai do
    /// raio ativo dele, e o servidor nunca reenviava. Andando a pé isso passava
    /// despercebido porque o raio do login cobria a vila inteira; o teleporte de GM
    /// escancarou — o destino chegava vazio, e a vila de origem também, na volta.
    ///
    /// O mesmo valia para **jogador**, e sobreviveu à primeira correção: o `gateway.rs`
    /// mandava `PLAYER_ENTER_WORLD` mútuo no login e `PLAYER_LEAVE_WORLD` na queda da
    /// conexão, sem raio e sem streaming. Dois jogadores que se afastassem além do raio
    /// ativo do cliente sumiam um para o outro **para sempre** — o cliente descarta, e
    /// nada reenviava. Desde 2026-09-09 jogador entra aqui também.
    ///
    /// # Como funciona
    ///
    /// A grade espacial do mundo (`SpatialGrid`) já indexa monstros, NPCs e jogadores por
    /// posição. Aqui se pergunta a ela quem está dentro de [`RAIO_DE_VISAO`], compara com
    /// o que o jogador já tem (`PlayerEntity::visiveis`) e manda só a diferença:
    /// `NPC_ENTER_SLICE` (11), `PLAYER_ENTER_SLICE` (12) ou `MATTER_ENTER_WORLD` (18) para
    /// quem entrou; `OBJECT_LEAVE_SLICE` (13) ou `OUT_OF_SIGHT_LIST` (34) para quem saiu.
    ///
    /// # Três famílias, três comandos de entrada
    ///
    /// `NPC_ENTER_SLICE` (11) para NPC e monstro, `PLAYER_ENTER_SLICE` (12) para jogador,
    /// `MATTER_ENTER_WORLD` (18) para recurso de mapa. O cliente roteia pelo **comando**,
    /// não pelo id: o 11 vai para `MAN_NPC` (`EC_GameDataPrtc.cpp:851`), o 12 para
    /// `MAN_PLAYER` (`:832-833`) e o 18 para `MAN_MATTER` (`:902-904`) — mandar o errado
    /// entrega a entidade ao gerente errado.
    ///
    /// A matéria (minério, erva) não saía de lugar nenhum até 2026-09-09: o `npcgen.data`
    /// tem as 5.125 instâncias deste mapa, e nenhum ponto do servidor mandava o comando —
    /// o mapa vinha sem recurso algum.
    ///
    /// E `PLAYER_ENTER_SLICE` não é `PLAYER_ENTER_WORLD`: a struct é a mesma
    /// (`S2C::info_player_1` no IR para os dois), mas o cliente usa o comando para
    /// escolher o efeito de aparição — `APPEAR_ENTERWORLD` para o 17, `APPEAR_RUNINTOVIEW`
    /// para o 12 (`EC_ManPlayer.cpp:1845`). Quem vem andando não deve surgir com efeito de
    /// teleporte.
    ///
    /// Na saída são dois, e a divisão é outra: o `OBJECT_LEAVE_SLICE` (13) serve a jogador
    /// **e** NPC, porque esse o cliente roteia pelo id
    /// (`ISPLAYERID`/`ISNPCID`, `EC_GameDataPrtc.cpp:891-899`) — mas ele não conhece
    /// matéria. Matéria sai pelo `OUT_OF_SIGHT_LIST` (34), a lista que o cliente separa id
    /// a id pelas três máscaras (`:1056-1071`).
    ///
    /// # A visibilidade entre jogadores é mútua, e por isso escrita nos dois
    ///
    /// Se eu ando na direção de alguém parado, **só a minha** atualização roda: quem está
    /// parado não recalcula nada. Então, quando um jogador entra ou sai do meu alcance,
    /// este método escreve nos dois lados — manda o comando para ele e mexe no `visiveis`
    /// dele — em vez de esperar que a atualização dele chegue à mesma conclusão. A
    /// distância é simétrica e o raio é o mesmo para todos, então as duas visões
    /// concordam.
    ///
    /// # As três decisões que fazem isto não derrubar o servidor
    ///
    /// 1. **Histerese**: a conta só é refeita depois que o jogador anda
    ///    [`PASSO_PARA_RECALCULAR`]. O cliente manda movimento 20 vezes por segundo, e
    ///    varrer a grade a cada pacote seria varrer 20 vezes por segundo por jogador para
    ///    achar quase sempre o mesmo conjunto.
    /// 2. **Teto por atualização, e um por família**: este mapa tem 21.846 monstros, 3.911
    ///    NPCs e 5.125 recursos. Uma região densa pode ter centenas dentro do raio, e
    ///    mandar tudo de uma vez enche a fila de saída. Criatura tem
    ///    [`TETO_DE_VISIVEIS`] e matéria tem [`TETO_DE_MATERIA`], **separados**: num campo
    ///    de mineração, um teto só faria as pedras expulsarem os NPCs. Nos dois, os mais
    ///    próximos primeiro; o que sobra entra na próxima atualização.
    /// 3. **Jogador não entra no teto.** São poucos — o limite é a capacidade do servidor
    ///    de mundo, não a densidade do mapa — e cortar um jogador por causa de uma
    ///    multidão de monstros quebraria a simetria do parágrafo acima: eu deixaria de
    ///    vê-lo sem que ele deixasse de me ver.
    ///
    /// `forcar` pula a histerese. Serve para os dois momentos em que a posição muda sem o
    /// jogador andar: a entrada no mundo e o teleporte.
    async fn atualizar_visiveis(&self, roleid: i32, envio: &EnvioAoCliente, forcar: bool) {
        let eu = roleid as i64;

        // 1. Vale a pena recalcular?
        let Some((centro, anterior)) = ({
            let mundo = self.world.read().await;
            mundo.players.get(&eu).map(|p| (p.position, p.centro_do_stream))
        }) else {
            return;
        };
        if !forcar && centro.distance(&anterior) < PASSO_PARA_RECALCULAR {
            return;
        }

        // 2. Quem está por perto agora, em três baldes: criatura (monstro e NPC) e
        //    matéria com tetos próprios, do mais próximo para o mais distante; jogador sem
        //    teto, pela simetria (ver a documentação).
        let (criaturas, materias, jogadores_perto): (Vec<i64>, Vec<i64>, Vec<i64>) = {
            let mundo = self.world.read().await;
            let perto = mundo.grid.get_entities_in_range(&centro, RAIO_DE_VISAO);

            let mut jogadores = Vec::new();
            let mut criaturas: Vec<(i64, f32)> = Vec::new();
            let mut materias: Vec<(i64, f32)> = Vec::new();

            for id in perto {
                if id == eu {
                    continue;
                }
                if mundo.players.contains_key(&id) {
                    jogadores.push(id);
                    continue;
                }
                if let Some(m) = mundo.matters.get(&id) {
                    materias.push((id, centro.distance(&m.position)));
                    continue;
                }
                if let Some(d) = mundo.drops.get(&id) {
                    materias.push((id, centro.distance(&d.position)));
                    continue;
                }
                if let Some(m) = mundo.mascotes.get(&id) {
                    criaturas.push((id, centro.distance(&m.corpo.position)));
                    continue;
                }
                let pos = match mundo.monsters.get(&id) {
                    Some((m, _)) if !m.is_dead => m.position,
                    // Monstro morto não é ausência de dado: é uma criatura que não deve
                    // ser mandada. Sair aqui evita procurá-la entre os NPCs.
                    Some(_) => continue,
                    None => match mundo.npcs.get(&id) {
                        Some(n) => n.position,
                        None => continue,
                    },
                };
                criaturas.push((id, centro.distance(&pos)));
            }

            criaturas.sort_by(|a, b| a.1.total_cmp(&b.1));
            materias.sort_by(|a, b| a.1.total_cmp(&b.1));

            (
                criaturas.into_iter().take(TETO_DE_VISIVEIS).map(|(id, _)| id).collect(),
                materias.into_iter().take(TETO_DE_MATERIA).map(|(id, _)| id).collect(),
                jogadores,
            )
        };

        // 3. A diferença, sobre o conjunto inteiro — `visiveis` guarda os dois tipos.
        let (entraram, sairam) = {
            let mut mundo = self.world.write().await;
            let Some(jogador) = mundo.players.get_mut(&eu) else {
                return;
            };
            let tudo = || criaturas.iter().chain(&materias).chain(&jogadores_perto).copied();
            let novos: std::collections::HashSet<i64> = tudo().collect();
            let entraram: Vec<i64> = tudo()
                .filter(|id| !jogador.visiveis.contains(id))
                .collect();
            let sairam: Vec<i64> = jogador
                .visiveis
                .iter()
                .copied()
                .filter(|id| !novos.contains(id))
                .collect();
            jogador.visiveis = novos;
            jogador.centro_do_stream = centro;
            (entraram, sairam)
        };

        if entraram.is_empty() && sairam.is_empty() {
            return;
        }

        // 4. Os dados de quem entrou, separados por família — o comando é outro. E o meu
        //    próprio pacote, para quem passou a me ver.
        let (chegando, eu_mesmo) = {
            let mundo = self.world.read().await;
            let chegando: Vec<QuemChegou> = entraram
                .iter()
                .filter_map(|id| {
                    if let Some(p) = mundo.players.get(id) {
                        return Some(QuemChegou::Jogador { id: *id as i32, vista: p.vista() });
                    }
                    if let Some(m) = mundo.matters.get(id) {
                        return Some(QuemChegou::Materia {
                            id: *id as i32,
                            tid: m.template_id as i32,
                            pos: m.position,
                        });
                    }
                    if let Some(d) = mundo.drops.get(id) {
                        return Some(QuemChegou::Materia {
                            id: *id as i32,
                            tid: d.item_id as i32,
                            pos: d.position,
                        });
                    }
                    if let Some(m) = mundo.mascotes.get(id) {
                        return Some(QuemChegou::Mascote {
                            id: *id as i32,
                            tid: m.info.pet_tid,
                            vis: m.vis_tid as i32,
                            pos: m.corpo.position,
                            dir: m.ai.direcao,
                            dono: m.dono as i32,
                            nome: m.nome.clone(),
                        });
                    }
                    match mundo.monsters.get(id) {
                        Some((m, ia)) => Some(QuemChegou::Criatura {
                            id: *id as i32,
                            tid: m.template_id as i32,
                            pos: m.position,
                            dir: ia.direcao,
                        }),
                        None => mundo.npcs.get(id).map(|n| QuemChegou::Criatura {
                            id: *id as i32,
                            tid: n.template_id as i32,
                            pos: n.position,
                            dir: n.direcao,
                        }),
                    }
                })
                .collect();
            let eu_mesmo = mundo.players.get(&eu).map(|p| p.vista());
            (chegando, eu_mesmo)
        };

        debug!(
            "mundo: {roleid} passou a ver {} e deixou de ver {}",
            chegando.len(),
            sairam.len()
        );

        for c in chegando {
            let pacote = match c {
                QuemChegou::Criatura { id, tid, pos, dir } => {
                    self.sub.npc_enter_slice(id, tid, pos, dir).data
                }
                QuemChegou::Jogador { id, vista } => {
                    self.sub.player_enter_slice(id, vista).data
                }
                QuemChegou::Materia { id, tid, pos } => {
                    S2CGamedataSend::matter_enter_world(id, tid, pos).data
                }
                QuemChegou::Mascote { id, tid, vis, pos, dir, dono, nome } => {
                    self.sub.mascote_entra(11, id, tid, vis, pos, dir, dono, &nome).data
                }
            };
            self.responder(roleid, pacote, envio).await;
        }
        // A saída depende da família, e por outro motivo que a entrada: o
        // `OBJECT_LEAVE_SLICE` (13) só trata `ISPLAYERID` e `ISNPCID`
        // (`EC_GameDataPrtc.cpp:891-899`). Um id de matéria mandado por ele não faz nada —
        // nem erro, nem efeito. Matéria sai pelo `OUT_OF_SIGHT_LIST` (34), que o cliente
        // roteia id a id (`:1056-1071`).
        let (materia_saiu, resto_saiu): (Vec<i64>, Vec<i64>) =
            sairam.iter().partition(|id| e_materia(**id));
        for id in &resto_saiu {
            self.responder(
                roleid,
                S2CGamedataSend::object_leave_slice(*id as i32).data,
                envio,
            )
            .await;
        }
        if !materia_saiu.is_empty() {
            let ids: Vec<i32> = materia_saiu.iter().map(|id| *id as i32).collect();
            self.responder(roleid, S2CGamedataSend::out_of_sight_list(&ids).data, envio)
                .await;
        }

        // 5. O outro lado da visibilidade entre jogadores: quem eu passei a ver precisa
        //    passar a me ver, e quem eu deixei de ver precisa deixar de me ver — sem
        //    depender de aquele jogador se mexer.
        let Some(minha_vista) = eu_mesmo else {
            return;
        };
        let meu_pacote = self.sub.player_enter_slice(roleid, minha_vista).data;
        let minha_saida = S2CGamedataSend::object_leave_slice(roleid).data;

        for outro in entraram {
            if self.passou_a_ver(outro, eu).await {
                self.enviar_ao_jogador(outro as i32, meu_pacote.clone()).await;
            }
        }
        for outro in sairam {
            if self.deixou_de_ver(outro, eu).await {
                self.enviar_ao_jogador(outro as i32, minha_saida.clone()).await;
            }
        }
    }

    /// Anota, no jogador `outro`, que ele passou a ver `quem`. `false` quando `outro` não
    /// é um jogador deste mundo, ou já o via.
    ///
    /// Existe porque a visibilidade entre jogadores é escrita nos dois lados por quem se
    /// move (ver [`Self::atualizar_visiveis`]). Sem isto, a atualização de `outro` — se e
    /// quando ele se mexesse — veria `quem` como novidade e mandaria o comando de novo.
    async fn passou_a_ver(&self, outro: i64, quem: i64) -> bool {
        let mut mundo = self.world.write().await;
        let Some(p) = mundo.players.get_mut(&outro) else {
            return false;
        };
        p.visiveis.insert(quem)
    }

    /// O espelho: tira `quem` do campo de visão de `outro`. `false` quando `outro` não é
    /// jogador, ou já não o via.
    async fn deixou_de_ver(&self, outro: i64, quem: i64) -> bool {
        let mut mundo = self.world.write().await;
        let Some(p) = mundo.players.get_mut(&outro) else {
            return false;
        };
        p.visiveis.remove(&quem)
    }

    /// Tira um jogador da vista de todos os outros, e avisa cada um.
    ///
    /// Chamado quando ele sai do mundo — pelo `LOGOUT` ou pela queda da conexão. Sem isto
    /// o avatar de quem saiu ficaria parado na tela dos outros até que eles andassem para
    /// longe o bastante para o streaming reparar.
    ///
    /// Vai `PLAYER_LEAVE_WORLD` (19), não `OBJECT_LEAVE_SLICE` (13): quem saiu do jogo não
    /// saiu do alcance, e o cliente distingue os dois (`bExit` em
    /// `CECPlayerMan::ElsePlayerLeave`).
    async fn tirar_da_vista_de_todos(&self, quem: i64) {
        let interessados: Vec<i64> = {
            let mut mundo = self.world.write().await;
            let ids: Vec<i64> = mundo
                .players
                .iter()
                .filter(|(id, p)| **id != quem && p.visiveis.contains(&quem))
                .map(|(id, _)| *id)
                .collect();
            for id in &ids {
                if let Some(p) = mundo.players.get_mut(id) {
                    p.visiveis.remove(&quem);
                }
            }
            ids
        };

        if interessados.is_empty() {
            return;
        }
        let pacote = S2CGamedataSend::player_leave_world(quem as i32).data;
        for id in interessados {
            self.enviar_ao_jogador(id as i32, pacote.clone()).await;
        }
    }

    /// Decola ou pousa o jogador.
    ///
    /// Chamado quando o cliente "usa" o item do slot de voo. O estado vive no mundo
    /// (`PlayerEntity::voando`) porque é o servidor que manda: o cliente só liga
    /// `GP_STATE_FLY` ao receber o `OBJECT_TAKEOFF` (`EC_HostMsg.cpp:5936-5960`).
    ///
    /// O comando vai para quem pediu **e** para quem está por perto — é assim que os
    /// outros veem as asas abrirem.
    ///
    /// **Sabidamente incompleto**: o voo não custa mana nem tem altura máxima, e o
    /// `GP_STATE_FLY` não entra no `state` dos pacotes de visão, então quem chegar depois
    /// vê o jogador andando no ar em vez de voando.
    async fn alternar_voo(&self, roleid: i32, envio: &EnvioAoCliente) {
        let voando = {
            let mut mundo = self.world.write().await;
            let Some(jogador) = mundo.players.get_mut(&(roleid as i64)) else {
                warn!("mundo: {roleid} pediu voo sem estar no mundo");
                return;
            };
            jogador.voando = !jogador.voando;
            jogador.voando
        };

        let pacote = if voando {
            S2CGamedataSend::object_takeoff(roleid).data
        } else {
            S2CGamedataSend::object_landing(roleid).data
        };
        debug!("mundo: {roleid} {}", if voando { "decolou" } else { "pousou" });
        self.responder(roleid, pacote.clone(), envio).await;
        self.transmitir_a_outros(roleid, pacote).await;
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

        let mut mundo = self.world.write().await;
        let existe = mundo.dados_do_npc(pedido.target as i64).is_some() || mundo.is_scene_service_npc(pedido.target);
        if existe {
            if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
                p.npc_em_conversa = Some(pedido.target as i64);
            }
        }
        drop(mundo);

        if !existe {
            debug!(
                "mundo: {roleid} disse olá para {}, que não é um NPC deste mundo",
                pedido.target
            );
            return;
        }

        info!("mundo: {roleid} abriu diálogo com NPC {}", pedido.target);

        self.responder(
            roleid,
            S2CGamedataSend::npc_greeting(pedido.target).data,
            envio,
        )
        .await;
    }

    /// `C2S::TASK_NOTIFY` (49) — o cliente reporta algo ao sistema de missões.
    ///
    /// O original despacha pelo `reason` em `OnClientNotify` (`task/TaskServer.cpp:330`).
    /// Um só é respondido aqui, o que o cliente manda ao entrar no mundo:
    ///
    /// **`TASK_CLT_NOTIFY_DYN_TIMEMARK` (7)** — a marca do pacote de missões dinâmicas. O
    /// original responde com a marca do seu `dyn_tasks.data` e **não responde** quando não
    /// tem marca (`ATaskTemplMan::OnTaskGetDynTasksTimeMark`, `TaskTemplMan.cpp:299-309`).
    /// Com a marca igual à do arquivo local, o cliente carrega as missões dinâmicas dele e
    /// só então monta a lista de missões ativas (`OnDynTasksTimeMark`,
    /// `TaskTemplMan.cpp:166-178`).
    ///
    /// Os outros `reason` (concluir, desistir, chegar ao local, prêmio especial, depósito…)
    /// só são registrados: são o motor de missões, que ainda não existe no `pw-gs`.
    ///
    /// Até 2026-09-14 o `gateway.rs` respondia este comando por conta própria, com `reason`
    /// 7 — que no cliente é `TASK_SVR_NOTIFY_FORGET_SKILL` — e reenviava `TASK_DATA` a
    /// qualquer outra notificação.
    async fn notificar_tarefa(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        /// `TASK_CLT_NOTIFY_DYN_TIMEMARK` (`task/TaskTempl.h:109`).
        const PEDIDO_DA_MARCA_DINAMICA: u8 = 7;
        /// `TASK_CLT_NOTIFY_DYN_DATA` (`task/TaskTempl.h:110`): o cliente não tem (ou tem
        /// diferente) o pacote de missões dinâmicas e pede o arquivo.
        const PEDIDO_DOS_DADOS_DINAMICOS: u8 = 8;

        let Some(tn) = TaskNotify::ler(payload) else {
            warn!("mundo: task_notify de {roleid} com payload curto ou size inconsistente");
            return;
        };

        if tn.reason == Some(PEDIDO_DA_MARCA_DINAMICA) {
            let marca = self.world.read().await.data_manager.marca_das_missoes_dinamicas;
            match marca {
                Some(marca) => {
                    debug!("mundo: {roleid} pediu a marca das missões dinâmicas: {marca:#x}");
                    self.responder(roleid, S2CGamedataSend::task_dyn_time_mark(marca).data, envio)
                        .await;
                }
                None => debug!(
                    "mundo: {roleid} pediu a marca das missões dinâmicas, e o realm não tem                      dyn_tasks.data — sem resposta, como o original"
                ),
            }
            return;
        }

        if tn.reason == Some(PEDIDO_DOS_DADOS_DINAMICOS) {
            let dados = self.world.read().await.data_manager.missoes_dinamicas.clone();
            let Some(dados) = dados else {
                debug!("mundo: {roleid} pediu as missões dinâmicas, e o realm não tem dyn_tasks.data");
                return;
            };
            let passo = S2CGamedataSend::PEDACO_DAS_MISSOES_DINAMICAS;
            let total = dados.len();
            debug!("mundo: {roleid} pediu as missões dinâmicas ({total} bytes)");
            let mut enviados = 0;
            while enviados < total {
                let fim = (enviados + passo).min(total);
                let ultimo = fim == total;
                self.responder(roleid, S2CGamedataSend::task_dyn_data(&dados[enviados..fim], ultimo).data, envio)
                    .await;
                enviados = fim;
            }
            return;
        }

        if let (Some(motivo), Some(task)) = (tn.reason, tn.task) {
            if self.aviso_de_missao(roleid, motivo, task as u32).await {
                return;
            }
        }
        debug!(
            "mundo: {roleid} mandou task_notify (reason={:?}, task={:?}, {} bytes) — motivo não tratado",
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
            servico::NPC_VENDE => self.comprar(roleid, c).await,
            // O NPC **compra**: o jogador está vendendo.
            servico::NPC_COMPRA => self.vender(roleid, c).await,

            servico::REPARAR => {
                // TODO: o custo é fixo enquanto a durabilidade dos itens não for lida. O
                // dinheiro sai da entidade (o autosave grava a entidade por cima do banco).
                const CUSTO: i64 = 150;
                let pagou = self.com_contexto(roleid, |ctx| ctx.gastar_dinheiro(CUSTO)).await.unwrap_or(false);
                if pagou {
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
                let (nivel, cultivo, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap) = (
                    p.level,
                    p.cultivation.clamp(0, u8::MAX as i32) as u8,
                    p.combate_s > 0,
                    p.hp,
                    p.max_hp,
                    p.mp,
                    p.max_mp,
                    p.exp,
                    p.sp,
                    p.ap,
                    p.max_ap,
                );
                drop(mundo);

                self.responder(
                    roleid,
                    S2CGamedataSend::self_info_00(
                        nivel as i16,
                        cultivo,
                        lutando,
                        hp,
                        max_hp,
                        mp,
                        max_mp,
                        exp as i32,
                        sp as i32,
                        ap,
                        max_ap,
                    )
                    .data,
                    envio,
                )
                .await;
            }

            servico::ACEITAR_MISSAO => self.aceitar_missao(roleid, c).await,
            servico::ENTREGAR_MISSAO => self.entregar_missao(roleid, c).await,
            servico::ITEM_DE_MISSAO => {
                debug!("mundo: {roleid} pediu item de missão ao NPC (serviço ainda não tratado)");
            }

            servico::TELEPORTAR => self.teleportar_pela_transportadora(roleid, c, envio).await,
            servico::APRENDER_HABILIDADE => self.aprender(roleid, c).await,
            servico::INCUBAR_PET => self.incubar_mascote(roleid, c).await,
            servico::RENOMEAR_MASCOTE => self.renomear_mascote(roleid, c).await,
            servico::ESQUECER_HABILIDADE_DE_MASCOTE => self.esquecer_habilidade_de_mascote(roleid, c).await,
            servico::APRENDER_HABILIDADE_DE_MASCOTE => self.aprender_habilidade_de_mascote(roleid, c).await,

            outro => {
                debug!("mundo: {roleid} pediu o serviço de NPC {outro}, ainda não tratado");
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
        let Some((nivel, nivel2, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap)) = dados else {
            debug!("mundo: {roleid} pediu o próprio estado sem estar neste mundo");
            return;
        };

        self.responder(
            roleid,
            S2CGamedataSend::self_info_00(nivel, nivel2, lutando, hp, max_hp, mp, max_mp, exp, sp, ap, max_ap).data,
            envio,
        )
        .await;
        // O que o original responde a este pedido é o `OWN_EXT_PROP` (`PlayerGetProperty`,
        // `player.cpp:8588-8596`) — é por ele que a janela de atributos se refaz depois de
        // gastar ponto (`OnMsgHstAddStatusPt` pede este comando, `EC_HostMsg.cpp:1624`).
        let ficha = self.world.read().await.players.get(&(roleid as i64)).map(|p| self.ficha_propria(p));
        if let Some(f) = ficha {
            self.responder(roleid, f, envio).await;
        }
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

        for (id, (nivel, nivel2, lutando, hp, max_hp, mp, max_mp, alvo)) in respostas {
            self.responder(
                roleid,
                self.sub.player_info_00(id, nivel, nivel2, lutando, hp, max_hp, mp, max_mp, alvo)
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

    /// `C2S::GET_OTHERS_EQUIPMENT` (33) — o cliente pede o equipamento visível de outro
    /// jogador logo depois de receber `PLAYER_ENTER_WORLD` dele
    /// (`EC_ManPlayer.cpp::OnMsgPlayerInfo`, `if (!pPlayer->IsEquipDataReady())
    /// pSession->c2s_CmdGetOtherEquip(...)`).
    ///
    /// # A causa confirmada de "só aparece a caixa de colisão, o modelo nunca carrega"
    ///
    /// Este comando nunca tinha resposta nenhuma — caía no `outro =>` genérico deste
    /// `match` e ficava mudo pra sempre. Sem uma resposta, `CECElsePlayer::
    /// IsEquipDataReady()` nunca vira `true` (só `ChangeEquipments`, chamado ao processar
    /// `EQUIP_DATA`/`EQUIP_DATA_CHANGED`, muda essa flag — `EC_ElsePlayer.cpp:1669-1675`),
    /// e o modelo 3D só é criado quando `IsBaseInfoReady() && IsCustomDataReady() &&
    /// IsEquipDataReady()` são true ao mesmo tempo (`EC_ElsePlayer.cpp:671`). Confirmado
    /// em teste real com dois clientes (2026-09-04/05): o servidor mandava
    /// `PLAYER_ENTER_WORLD` e respondia `PlayerBaseInfo` certo nos dois sentidos, os dois
    /// clientes pediam `GetOtherEquip` (visto no log do realm, `cmd=33`), e nenhum dos
    /// dois via o modelo do outro — só a caixa de colisão.
    ///
    /// # O que vai em `mask` e em `data[]`
    ///
    /// Até 2026-09-07 a resposta ia com `mask = 0` — "nenhum item" — sob o argumento de
    /// que isso já bastava para destravar o modelo. Destrava, mas descreve o outro
    /// jogador como se estivesse **pelado**: `ChangeEquipments` faz
    /// `memset(m_aNewEquips, 0, ...)` e, sem bit ligado nenhum, nada volta a ser
    /// preenchido (`EC_ElsePlayer.cpp:1700-1712`). Em jogo o relato foi "não é possível
    /// ver o modelo 3D **e equipamentos** do outro jogador".
    ///
    /// Agora vai o equipamento de verdade. O formato veio do próprio cliente:
    ///
    ///   * `mask` é um `__int64` com **um bit por slot** de `EQUIPIVTR_*`
    ///     (`EC_IvtrTypes.h:56-96`): 0 arma, 1 cabeça, … 11 munição, … até 39
    ///     (`SIZE_ALL_EQUIPIVTR`).
    ///   * `data[]` traz **um inteiro por bit ligado, em ordem crescente de slot** — é
    ///     assim que o cliente lê (`m_aNewEquips[i] = aAddedEquip[iCount++]` dentro de um
    ///     laço de `i` crescente).
    ///   * cada inteiro é o **id do item no `elements.data` nos 16 bits baixos**:
    ///     `CECPlayer::GetRealElementID` faz `dwEquipID & 0x0000ffff` porque os 16 bits
    ///     altos guardam cor, usada só pelos slots de moda (`EC_Player.cpp:9635-9644`).
    ///     Como ainda não temos cor de moda no banco, a parte alta vai zerada.
    ///
    /// Slot fora de 0..40 é descartado: o cliente indexa `m_aNewEquips` direto pelo bit,
    /// e um bit acima de `SIZE_ALL_EQUIPIVTR` escreveria fora do array dele.
    async fn equipamento_de_outro(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(consulta) = ConsultaDeIds::ler(payload) else {
            warn!("mundo: get_others_equipment de {roleid} com payload curto");
            return;
        };
        debug!("mundo: get_other_equip de {roleid}, ids: {:?}", consulta.ids);

        let itens = self.repo().await.item_repo().clone();
        for id in consulta.ids {
            let equipado = itens
                .list_by_container(id, ContainerType::Equipment)
                .await
                .unwrap_or_default();
            let (mascara, ids) = Self::mascara_de_equipamento(&equipado);
            let pacote = self.sub.equip_data(id, 0, mascara, &ids).data;
            debug!(
                "mundo: equip_data pra {roleid} sobre {id} — máscara {mascara:#x}, {} item(ns)",
                ids.len()
            );
            self.responder(roleid, pacote, envio).await;
        }
    }

    /// Monta o par `(mask, data[])` do `EQUIP_DATA` a partir das linhas de equipamento.
    ///
    /// Devolve os ids **ordenados por slot**, que é a ordem em que o cliente os consome.
    /// Ver a documentação de [`Self::equipamento_de_outro`] para as fontes no cliente.
    fn mascara_de_equipamento(equipado: &[pw_core::ItemRecord]) -> (u64, Vec<i32>) {
        /// `SIZE_ALL_EQUIPIVTR` do `EC_IvtrTypes.h` — o tamanho do array `m_aNewEquips`.
        const TOTAL_DE_SLOTS: u16 = 40;

        let mut por_slot: std::collections::BTreeMap<u16, i32> = std::collections::BTreeMap::new();
        for item in equipado {
            if item.slot >= TOTAL_DE_SLOTS {
                warn!(
                    "mundo: equipamento no slot {} está fora de 0..{TOTAL_DE_SLOTS}, ignorado",
                    item.slot
                );
                continue;
            }
            // 16 bits baixos: o id do item. Os altos são cor de moda, que ainda não temos.
            por_slot.insert(item.slot, (item.item_id as i32) & 0xffff);
        }

        let mut mascara = 0u64;
        for slot in por_slot.keys() {
            mascara |= 1u64 << slot;
        }
        (mascara, por_slot.into_values().collect())
    }

    /// `C2S::CHECK_SECURITY_PASSWD` (120) — a senha do guarda-roupa.
    ///
    /// O cliente manda isto com senha **vazia** assim que sabe que a conta não tem senha
    /// (`EC_HostMsg.cpp`, `case TRASHBOX_PWD_STATE`), e espera `SECURITY_PASSWD_CHECKED`
    /// (277) para liberar a primeira abertura do guarda-roupa
    /// (`CECHostPlayer::OnMsgPlayerPasswdChecked` zera `m_bFirstFashionOpen`).
    ///
    /// Sem resposta o par nunca fecha, e o jogador vê "protegido por senha" numa conta que
    /// nunca teve senha — relatado em jogo em 2026-09-07.
    ///
    /// # Por que qualquer senha passa, por enquanto
    ///
    /// Não existe senha de guarda-roupa no nosso banco: nenhuma coluna a guarda e nenhum
    /// caminho a define. Recusar seria trancar todo mundo para sempre; aceitar é o mesmo
    /// que o servidor original faz para conta sem senha configurada. Quando a senha
    /// existir, é **aqui** que ela é conferida — e o `else` passa a mandar o
    /// `ERROR_MESSAGE` correspondente.
    async fn conferir_senha(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        // `struct cmd_check_security_passwd { size_t passwd_size; }` mais os bytes da
        // senha. O tamanho é lido só para o log: o conteúdo não decide nada ainda.
        let tamanho = payload
            .get(..4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0);
        if tamanho != 0 {
            info!("mundo: {roleid} mandou senha de guarda-roupa com {tamanho} bytes —                    ainda não há senha no banco, então passa");
        }
        self.responder(roleid, S2CGamedataSend::security_passwd_checked().data, envio)
            .await;
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
        // As tabelas de jogo do realm, para o bloco de dados de cada item sair do
        // `elements.data` em vez de um chute — ou de lugar nenhum.
        let dados = self.world.read().await.data_manager.clone();

        let pedido = GetAllData::ler(payload).unwrap_or(GetAllData {
            // Um payload curto vem de cliente de outra versão. Mandar tudo é o
            // comportamento antigo, e é o seguro: falta de dado trava a entrada no mundo.
            detalhe_bolsa: 1,
            detalhe_equipamento: 1,
            detalhe_missoes: 1,
        });

        let itens = self.itens().await;

        // Bolsa 0 (Inventário): no original (`player.cpp:13233-13238`), OWN_IVTR_DATA (42) vai SEMPRE.
        // Se detalhe_bolsa != 0, envia também os blocos detalhados (item_info 40) de cada item.
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
        if pedido.detalhe_bolsa != 0 {
            for item in &bolsa {
                self.responder(roleid, Self::info_de(0, item, &dados), envio).await;
            }
        }

        // Bolsa 1 (Equipamento): no original (`player.cpp:13239-13242`), OWN_IVTR_DATA (42) vai SEMPRE.
        // Se detalhe_equipamento != 0, envia também os blocos detalhados (item_info 40) de cada item.
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
        if pedido.detalhe_equipamento != 0 {
            for item in &equipado {
                self.responder(roleid, Self::info_de(1, item, &dados), envio).await;
            }
        }

        if let Some(d) = self.world.read().await.dinheiro(roleid) {
            self.responder(roleid, S2CGamedataSend::player_cash(d).data, envio)
                .await;
        }

        // Bolsa 2 (Missão, `IL_TASK_INVENTORY`): no original (`player.cpp:13243-13247`),
        // `SendAllData` manda as três bolsas SEMPRE. O cliente 1.5.5 pede `GetAllData(true, true, false)`
        // (`EC_HostPlayer.cpp:559`), ou seja, `detalhe_missoes = 0`. O OWN_IVTR_DATA (42) precisa ir
        // para o cliente inicializar o inventário de missão; os detalhes só vão se pedidos.
        let bolsa_missao = itens
            .list_by_container(roleid, ContainerType::TaskInventory)
            .await
            .unwrap_or_default();
        self.responder(
            roleid,
            S2CGamedataSend::own_ivtr_from_items(2, crate::economia::TAMANHO_DA_BOLSA_DE_MISSAO as u8, &bolsa_missao).data,
            envio,
        )
        .await;
        if pedido.detalhe_missoes != 0 {
            for item in &bolsa_missao {
                self.responder(roleid, Self::info_de(2, item, &dados), envio).await;
            }
        }

        // As habilidades aprendidas.
        //
        // **Isto nunca era enviado.** O `own_ivtr_data`, o equipamento, o dinheiro e as
        // missões saíam; o `SKILL_DATA` (90), não — e o cliente monta a barra de
        // habilidades a partir dele, então o jogador ficava sem nenhuma habilidade
        // utilizável. Em jogo, 2026-09-07: "meus personagens não têm skills que possam ser
        // usadas". O codificador já existia, sem chamador.
        //
        // Vai sempre, como o `TASK_DATA`: o `GET_ALL_DATA` não tem sinalizador para
        // habilidade, e a barra precisa estar montada antes de o jogador poder agir.
        let habilidades = self
            .repo()
            .await
            .skill_repo()
            .list_skills(roleid)
            .await
            .unwrap_or_default();
        if habilidades.is_empty() {
            warn!("mundo: jogador {roleid} entrou sem habilidade nenhuma no banco");
        }
        self.responder(
            roleid,
            S2CGamedataSend::skill_data_from_records(&habilidades).data,
            envio,
        )
        .await;

        // A lista de NPCs de serviço da cena (SCENE_SERVICE_NPC_LIST 390):
        // No original (`player.cpp:11735`), só envia NPCs com `_serve_distance_unlimited`
        // (ex: serviços remotos globais). NPCs normais do mapa NUNCA entram aqui, pois o
        // cliente (`EC_HostMsg.cpp:2627`) intercepta o greeting de qualquer NPC listado e
        // suprime a abertura da caixa de diálogo.
        let npcs_servico = self.world.read().await.scene_service_npcs();
        if !npcs_servico.is_empty() {
            if let Some(pacote) = self.sub.scene_service_npc_list(&npcs_servico) {
                self.responder(roleid, pacote.data, envio).await;
            }
        }

        // Os pontos de teleporte já descobertos (`WAYPOINT_LIST` 180). No original sai do
        // `SendAllData` com o `GetWaypointBuffer` (`gs/player_imp.h:2545-2550`); é esta lista
        // que o cliente usa para saber o que **não** é novo — sem ela ele repete o
        // `ACTIVATE_REGION_WAYPOINTS` a cada quadro (B67).
        let waypoints = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .map(|p| p.waypoints.clone())
            .unwrap_or_default();
        self.responder(roleid, S2CGamedataSend::player_waypoint_list(&waypoints).data, envio)
            .await;

        // Mascotes: capacidade da sala de mascotes (PET_ROOM_CAPACITY 240) e lista de mascotes (PET_ROOM 239).
        // No original (`player.cpp:13265`), SendAllData envia os dois comandos.
        let pets_corral = itens
            .list_by_container(roleid, ContainerType::PetCorral)
            .await
            .unwrap_or_default();
        let vagas = (pets_corral.len() as u32 + 1).max(1);
        self.responder(
            roleid,
            S2CGamedataSend::pet_room_capacity(vagas).data,
            envio,
        )
        .await;

        let mut pets_buf = Vec::new();
        for p in &pets_corral {
            pets_buf.extend_from_slice(&(p.slot as i32).to_le_bytes());
            if p.octets.len() >= pw_core::TAMANHO_INFO_PET {
                pets_buf.extend_from_slice(&p.octets[..pw_core::TAMANHO_INFO_PET]);
            } else {
                let mut info = pw_core::InfoPet::default();
                info.pet_tid = p.item_id as i32;
                pets_buf.extend_from_slice(&info.para_bytes());
            }
        }
        self.responder(
            roleid,
            S2CGamedataSend::pet_room(pets_corral.len() as u16, &pets_buf).data,
            envio,
        )
        .await;

        // A ficha do jogador: `OWN_EXT_PROP` (50).
        //
        // É o **único** comando que preenche `CECHostPlayer::m_ExtProps`
        // (`EC_HostMsg.cpp:1583`), e é de lá que `CanUseEquipment`
        // (`EC_HostPlayer.cpp:4907-4916`) lê força, agilidade, vitalidade e energia. Sem
        // ele os quatro ficam em zero e **todo** equipamento com exigência de atributo é
        // desenhado em `A3DCOLORRGB(192, 0, 0)` — a "arma vermelha" relatada em jogo.
        //
        // Sai daqui, e não do login no `pw-link`, por dois motivos que se somam: aqui o
        // dono da tela já existe (o cliente é que pediu os dados), e aqui os números são
        // os **calculados** — precisão, evasão, defesa e dano do `PlayerEntity`, em vez
        // dos zeros que o link tinha para oferecer.
        if let Some(p) = self.world.read().await.players.get(&(roleid as i64)).cloned() {
            self.responder(
                roleid,
                self.ficha_propria(&p),
                envio,
            )
            .await;
        } else {
            warn!("mundo: {roleid} pediu todos os dados sem estar no mundo — sem OWN_EXT_PROP");
        }

        // Sempre, mesmo sem missões: é o marcador de fim da carga.
        // Via `self.sub` porque o número de blocos depende da versão (3 no 1.2.6, 5 do
        // 1.5.3 em diante) — ver `WorldProtocol::task_data`.
        let listas = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .map(|p| p.missoes.blocos())
            .unwrap_or_else(|| crate::missoes::ListasDeMissao::default().blocos());
        let [a, b, c, d, e] = &listas;
        self.responder(roleid, self.sub.task_data_com_listas([a, b, c, d, e]).data, envio)
            .await;
    }

    /// O `item_info` de um item já carregado, para não repetir a conversão em dois lugares.
    ///
    /// A ficha sai do `elements.data` (`equipamentos`), da tabela da família certa. Quando
    /// o item não é equipamento — ou o realm não tem as tabelas — vai `None`, e o comando
    /// segue **sem** bloco de dados. Ver `S2CGamedataSend::item_info`: inventar requisito
    /// aqui tranca o item no cliente, e omitir o bloco tranca a armadura.
    /// A durabilidade já está na escala interna no banco (ver
    /// [`pw_core::ESCALA_DA_DURABILIDADE`]) e vai como está. Quando o item tem bloco
    /// gravado, a durabilidade **do bloco** é regravada com a da coluna: é a coluna que o
    /// servidor desgasta a cada golpe, e o bloco é o que o cliente lê (B61).
    fn info_de(
        onde: u8,
        item: &pw_core::ItemRecord,
        dados: &pw_data_loader::GameDataManager,
    ) -> Vec<u8> {
        let mut octetos = item.octets.clone();
        if octetos.is_empty() && dados.eh_ovo_de_pet(item.item_id) {
            if let Some(o) = dados.gerar_octetos_do_ovo(item.item_id) {
                octetos = o;
            }
        }
        // Amuleto de vida (`AUTOHP_ESSENCE`) e hierograma de mana (`AUTOMP_ESSENCE`): o
        // conteúdo são os 8 bytes do `amulet_essence` (`gs/item/item_amulet.h:16-19`,
        // `generate_item_temp.h:2296-2310`). Sem eles o cliente desenhava zeros e números
        // negativos no item (relato de 2026-09-19, B67).
        if octetos.is_empty() {
            if let Some(o) = dados.conteudo_do_amuleto(item.item_id) {
                octetos = o;
            }
        }
        // Item de voo (`FLYSWORD_ESSENCE`): sem o bloco, a máscara de classes vai zerada e
        // o cliente recusa usá-lo — era a "Glória de Shalim" (B68).
        if octetos.is_empty() {
            if let Some(o) = dados.conteudo_do_item_de_voo(item.item_id) {
                octetos = o;
            }
        }
        // Daimon (`GOBLIN_ESSENCE`): o bloco **é** o estado dele — experiência, nível,
        // atributos, gênios, refino e vigor (`elf_item::Save`, `gs/item/item_elf.cpp:172-185`;
        // `generate_elf`, `generate_item_temp.h:2442-2524`). Sem ele o cliente não desenha
        // ficha nenhuma e o Daimon parece inerte (B75).
        if octetos.is_empty() {
            if let Some((_, iniciais)) = dados.dados_do_daimon(item.item_id) {
                octetos = crate::entity::Daimon::novo(&iniciais).bloco();
            }
        }
        if item.max_durability > 0 {
            pw_core::escrever_durabilidade(&mut octetos, item.durability as i32, item.max_durability as i32);
        }
        S2CGamedataSend::item_info(
            onde,
            item.slot as u8,
            item.item_id as i32,
            item.durability as i32,
            item.max_durability as i32,
            item.count,
            &octetos,
            dados.equipamentos.ficha(item.item_id),
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
            let dados = self.world.read().await.data_manager.clone();
            self.responder(roleid, Self::info_de(onde, &i, &dados), envio)
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
    /// `_lock_equipment` (o `filter_Fairyform` o liga): o original recusa vestir, trocar
    /// peças, mover para o corpo e descartar peça com `ERR_EQUIPMENT_IS_LOCKED` (40,
    /// `common/protocol.h:720`; `gs/player.cpp:7874, 7991, 8077, 8258`). Aqui também se
    /// destravam os slots que o cliente congelou ao mandar o comando (B84). `true` quando
    /// recusou.
    async fn equipamento_travado(&self, roleid: i32, slots: &[(u8, u16)], envio: &EnvioAoCliente) -> bool {
        let travado = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .is_some_and(|p| p.efeitos.equipamento_travado());
        if !travado {
            return false;
        }
        debug!("mundo: {roleid} está com o equipamento trancado (Forma Sombria)");
        self.responder(roleid, S2CGamedataSend::error_message(ERRO_EQUIPAMENTO_TRANCADO).data, envio).await;
        for &(onde, slot) in slots {
            self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(onde, slot).data, envio).await;
        }
        true
    }

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
        if onde == ContainerType::Equipment && self.equipamento_travado(roleid, &[(1, p.a as u16), (1, p.b as u16)], envio).await {
            return;
        }

        let itens = self.itens().await;
        if let Err(e) = itens.swap_slots(roleid, onde, p.a as u16, p.b as u16).await {
            warn!("mundo: falha ao trocar slots de {roleid}: {e:?}");
            return;
        }

        if onde == ContainerType::Equipment {
            self.recalcular_equipamento(roleid, true).await;
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

    /// `C2S::DROP_IVTR_ITEM` (14) e `DROP_EQUIP_ITEM` (15) — jogar um item fora.
    ///
    /// `onde` é o pacote do cliente: 0 é a bolsa (`IVTRTYPE_PACK`) e 1 é o corpo
    /// (`IVTRTYPE_EQUIPPACK`, `EC_IvtrTypes.h:36-38`). O descarte da bolsa traz a
    /// quantidade; o do corpo não traz nada além do índice, e leva a peça inteira.
    ///
    /// O original joga o item **no chão**, a até meio metro do jogador, e só então avisa:
    /// `DropItemFromData`, `DecAmount` e `player_drop_item(onde, índice, tid, count,
    /// DROP_TYPE_PLAYER)` (`ThrowEquipItem`, `gs/player.cpp:7932-7980`). Sem dono: item que
    /// se joga fora é de quem pegar (o `XID(0,0)` do terceiro ramo, `:7968`).
    ///
    /// E o `UNFREEZE_IVTR_SLOT` no fim não é enfeite: o cliente congelou o slot ao mandar o
    /// comando, e é só ele que destrava.
    async fn descartar_item(&self, roleid: i32, onde: u8, payload: &[u8], envio: &EnvioAoCliente) {
        if payload.is_empty() {
            warn!("mundo: descarte de {roleid} sem corpo");
            return;
        }
        let slot = payload[0];
        if onde == 1 && self.equipamento_travado(roleid, &[(1, slot as u16)], envio).await {
            return;
        }
        let pedido = if onde == 0 && payload.len() >= 5 {
            u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]])
        } else {
            u32::MAX // o corpo não manda quantidade: vai a peça inteira
        };
        let recipiente = if onde == 0 { ContainerType::Inventory } else { ContainerType::Equipment };

        let itens = self.itens().await;
        let Ok(Some(item)) = itens.get_item_by_slot(roleid, recipiente, slot as u16).await else {
            // Nada ali: destrava e sai, senão o slot fica preso.
            self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(onde, slot as u16).data, envio).await;
            return;
        };
        let quantos = pedido.min(item.count).max(1);
        let sobra = item.count.saturating_sub(quantos);

        if sobra == 0 {
            if let Err(e) = itens.delete_item_by_slot(roleid, recipiente, slot as u16).await {
                warn!("mundo: não apaguei o item descartado de {roleid}: {e:?}");
                self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(onde, slot as u16).data, envio).await;
                return;
            }
        } else {
            let mut restante = item.clone();
            restante.count = sobra;
            if let Err(e) = itens.upsert_item(&restante).await {
                warn!("mundo: não gravei a sobra do descarte de {roleid}: {e:?}");
                self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(onde, slot as u16).data, envio).await;
                return;
            }
        }

        // No chão, sem dono — e com o conteúdo do item, que é o que guarda refino,
        // durabilidade e o resto (mesmo caminho do drop de monstro).
        let drop = {
            let pos = self.world.read().await.players.get(&(roleid as i64)).map(|p| p.position);
            match pos {
                Some(pos) => Some(self.world.write().await.criar_drop_com_octetos(
                    item.item_id,
                    quantos,
                    pos,
                    None,
                    item.octets.clone(),
                )),
                None => None,
            }
        };
        if let Some(d) = drop {
            self.mostrar_drop(&d).await;
        }
        info!("mundo: {roleid} jogou fora {quantos}× o item {} do pacote {onde}", item.item_id);

        self.responder(
            roleid,
            self.sub.player_drop_item(onde, slot, quantos, item.item_id as i32, DROP_TYPE_PLAYER).data,
            envio,
        )
        .await;
        self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(onde, slot as u16).data, envio).await;
        if onde == 1 {
            self.recalcular_equipamento(roleid, true).await;
        }
    }

    /// O `UnLockInventoryHandler` do original (`gs/playercmd.cpp:183-230`): destrava os
    /// slots que um comando de item congela, para os comandos que este servidor ainda não
    /// executa.
    ///
    /// O cliente congela **antes** de mandar e nunca destrava sozinho: só o
    /// `UNFREEZE_IVTR_SLOT` (181) ou o fim de uma troca limpam o `m_bNetFrozen`
    /// (`EC_HostMsg.cpp:2060-2064`; `CECIvtrItem::NetFreeze`, `EC_IvtrItem.h:292`). Um
    /// comando que falte, então, não deixa só de funcionar: **apaga o item na bolsa** até o
    /// jogador relogar. Foi o que aconteceu com o amuleto do RT (B84).
    ///
    /// A tabela é a dos comandos que o cliente congela ao enviar
    /// (`Network/EC_GameSession.cpp:6304-6390`), com os pacotes de cada um.
    async fn destravar_slots_do_comando(&self, roleid: i32, cmd: u16, payload: &[u8], envio: &EnvioAoCliente) {
        let b = |i: usize| payload.get(i).copied().unwrap_or(0) as u16;
        // (pacote, índice) de cada slot que aquele comando congelou.
        let slots: &[(u8, u16)] = &match cmd {
            ids::EXG_IVTR_ITEM | ids::MOVE_IVTR_ITEM => vec![(0, b(0)), (0, b(1))],
            ids::DROP_IVTR_ITEM => vec![(0, b(0))],
            ids::DROP_EQUIP_ITEM => vec![(1, b(0))],
            ids::EXG_EQUIP_ITEM => vec![(1, b(0)), (1, b(1))],
            ids::EQUIP_ITEM | ids::MOVE_ITEM_TO_EQUIP => vec![(0, b(0)), (1, b(1))],
            _ => return,
        };
        for (onde, slot) in slots {
            self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(*onde, *slot).data, envio).await;
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
        if self.equipamento_travado(roleid, &[(0, idx_bolsa as u16), (1, idx_corpo as u16)], envio).await {
            return;
        }

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
        self.recalcular_equipamento(roleid, true).await;
    }

    /// `C2S::MOVE_ITEM_TO_EQUIP` (18) — mover da bolsa direto para um slot do corpo.
    async fn mover_para_equipar(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(p) = ParDeSlots::ler(payload) else {
            warn!("mundo: move_item_to_equip de {roleid} com payload curto");
            return;
        };
        if self.equipamento_travado(roleid, &[(0, p.a as u16), (1, p.b as u16)], envio).await {
            return;
        }

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
        self.recalcular_equipamento(roleid, true).await;
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
    /// O canal de saída de um jogador, para quem precisa passar um `EnvioAoCliente` adiante
    /// (os tratadores que vieram de um evento do tique, e não de um comando).
    async fn envio_de(&self, roleid: i32) -> Option<EnvioAoCliente> {
        self.sessoes.read().await.get(&roleid).map(|s| s.envio.clone())
    }

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

    /// Manda o mesmo pacote pra todo jogador conectado a este servidor de mundo, **menos**
    /// quem originou a ação (`exceto`) — o padrão de toda notificação de movimento/ação
    /// visível a terceiros (`object_move`, `object_stop_move`, e o que vier depois na
    /// mesma família).
    ///
    /// Mesma limitação sabida do `LinkGateway::jogadores_visiveis` (`pw-link`): sem
    /// grade espacial, todo mundo deste servidor recebe, não só quem está perto.
    /// `self.sessoes` é por realm/mundo (populado no `EnterWorld`, independente de
    /// `WorldInstance::players`), então isto funciona mesmo sem o mundo saber quem é
    /// vizinho de quem.
    /// Manda a quem **vê** aquele objeto, e só a eles.
    ///
    /// É o alcance do original: o movimento de um NPC sai por `AutoBroadcastCSMsg` na fatia
    /// dele (`gnpc_dispatcher::move`, `npc.cpp:85-98`), não para o mapa inteiro. Mandando a
    /// todos, o cliente recebia comandos de monstros que nunca viu entrar — 319 deles no teste
    /// de 2026-09-17 — e os punha na fila de "NPC desconhecido", perguntando por eles de 10 em
    /// 10 s para sempre (`CECNPCMan::SeekOutNPC`/`UpdateUnknownNPCs`, `EC_ManNPC.cpp:967-975`,
    /// `1144-1164`). (B57.)
    async fn transmitir_a_quem_ve(&self, objeto: i64, data: Vec<u8>) {
        let quem: Vec<i32> = {
            let mundo = self.world.read().await;
            mundo
                .players
                .values()
                .filter(|p| p.visiveis.contains(&objeto))
                .map(|p| p.role_id)
                .collect()
        };
        let sessoes = self.sessoes.read().await;
        for roleid in quem {
            if let Some(s) = sessoes.get(&roleid) {
                let _ = s.envio.try_send(BusMessage::GameToClient {
                    roleid,
                    localsid: s.localsid,
                    data: data.clone(),
                });
            }
        }
    }

    async fn transmitir_a_outros(&self, exceto: i32, data: Vec<u8>) {
        let sessoes = self.sessoes.read().await;
        for (id, s) in sessoes.iter() {
            if *id == exceto {
                continue;
            }
            let _ = s.envio.try_send(BusMessage::GameToClient {
                roleid: *id,
                localsid: s.localsid,
                data: data.clone(),
            });
        }
    }

    /// Quantos jogadores este servidor de mundo está atendendo.
    pub async fn jogadores_atendidos(&self) -> usize {
        self.sessoes.read().await.len()
    }
}


/// `gactive_imp::CheckAttack(target, &flag, …)` (`actobject.cpp:1254-1292`) para monstro:
/// vivo e a no máximo `attack_range + body_size` (o `attack_range` do jogador já inclui o
/// corpo dele, `playertemplate.h:954`). `Err` com o bit do motivo: 2 alvo inválido, 4 longe.
fn pode_golpear(mundo: &crate::world::WorldInstance, roleid: i32, alvo: i64) -> Result<(), i32> {
    let Some(p) = mundo.players.get(&(roleid as i64)) else { return Err(1) };
    let Some((m, _)) = mundo.monsters.get(&alvo) else { return Err(2) };
    if m.is_dead {
        return Err(2);
    }
    let corpo = mundo.data_manager.monstros.get(m.template_id).map(|t| t.tamanho).unwrap_or(0.0);
    if p.position.distance(&m.position) > p.attack_range + corpo {
        return Err(4);
    }
    Ok(())
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

    /// Ajuda a montar linhas de equipamento sem repetir o struct inteiro.
    fn equipado(slot: u16, item_id: u32) -> pw_core::ItemRecord {
        pw_core::ItemRecord {
            id: None,
            character_id: 1,
            container_type: ContainerType::Equipment,
            slot,
            item_id,
            count: 1,
            max_count: 1,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 2800,
            max_durability: 2800,
            bind_status: 0,
            octets: Vec::new(),
            custom_attributes: serde_json::json!({}),
        }
    }

    /// O cliente lê `data[]` com um `iCount++` dentro de um laço de slot **crescente**
    /// (`ChangeEquipments`, `EC_ElsePlayer.cpp:1700-1712`). Se os ids saírem em outra
    /// ordem, a arma vai parar no slot da bota.
    #[test]
    fn a_mascara_de_equipamento_sai_em_ordem_de_slot() {
        // De propósito fora de ordem na entrada.
        let itens = vec![equipado(4, 1234), equipado(0, 2251), equipado(11, 2271)];
        let (mascara, ids) = BusServer::mascara_de_equipamento(&itens);

        assert_eq!(mascara, (1 << 0) | (1 << 4) | (1 << 11));
        assert_eq!(ids, vec![2251, 1234, 2271], "arma, corpo, munição — nessa ordem");
        assert_eq!(
            ids.len(),
            mascara.count_ones() as usize,
            "um inteiro por bit ligado, senão o cliente lê fora do array"
        );
    }

    /// `GetRealElementID` faz `dwEquipID & 0x0000ffff` (`EC_Player.cpp:9641`): os 16 bits
    /// altos são cor de moda, não fazem parte do id.
    #[test]
    fn a_mascara_de_equipamento_manda_so_os_16_bits_baixos_do_id() {
        let (_, ids) = BusServer::mascara_de_equipamento(&[equipado(0, 0x0004_08D3)]);
        assert_eq!(ids, vec![0x08D3]);
    }

    /// `m_aNewEquips` tem `SIZE_ALL_EQUIPIVTR` (40) posições e é indexado direto pelo bit.
    /// Um slot acima disso faria o cliente escrever fora do array.
    #[test]
    fn slot_fora_do_array_do_cliente_e_descartado() {
        let itens = vec![equipado(0, 2251), equipado(40, 999), equipado(64, 999)];
        let (mascara, ids) = BusServer::mascara_de_equipamento(&itens);
        assert_eq!(mascara, 1 << 0);
        assert_eq!(ids, vec![2251]);
    }

    /// Sem nada equipado a resposta continua válida — e é ela que destrava
    /// `IsEquipDataReady()` no cliente.
    #[test]
    fn sem_equipamento_a_mascara_e_zero() {
        let (mascara, ids) = BusServer::mascara_de_equipamento(&[]);
        assert_eq!(mascara, 0);
        assert!(ids.is_empty());
    }
}
