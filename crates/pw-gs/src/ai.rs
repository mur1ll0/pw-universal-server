//! A IA de movimento dos monstros: perseguir, voltar para casa e passear.
//!
//! # As regras são as do servidor original
//!
//! | comportamento | original | aqui |
//! | :--- | :--- | :--- |
//! | perseguir | `session_npc_follow_target` (`gs/npcsession.cpp:164`): um passo a cada `NPC_FOLLOW_TARGET_TIME` = 0,5 s, de `run_speed × 0,5` m | [`MonsterAi::PASSO_DE_PERSEGUICAO_MS`] |
//! | voltar para casa | `ai_returnhome_task` → `session_npc_patrol` correndo, passo de `NPC_PATROL_TIME` = 1 s | [`MonsterAi::PASSO_DE_PATRULHA_MS`] |
//! | passear | `ai_policy::HaveRest` (`gs/aipolicy.cpp:237`) + `ai_rest_task` + `session_npc_cruise` | [`MonsterAi::tick`] |
//! | altura | o *agent* de caminho do habitat devolve a posição já no chão (`pathfinding.cpp:74`) | [`Habitat`] + altura do `.hmap` |
//!
//! O passeio, em detalhe: a cada batimento (1 s) de um monstro **com jogador por perto**
//! (`idle_timer > 0`, renovado enquanto há alguém na vizinhança, `NPC_IDLE_TIMER` = 20
//! batimentos), sem tarefa, sem ódio e com `patroll_mode` no `elements.data`, o contador
//! `cruise_timer` anda uma casa de 32 (`ai_npcobject::CanRest`, `gs/ainpc.cpp:303`). Quando
//! dá a volta, o monstro sai andando (`walk_speed`, um passo por segundo) para um ponto a até
//! 10 m do lugar onde nasceu (`SetTarget(birth_place, 8, 10.0f)`), com no máximo 8 passos.
//! Ao chegar, há 10% de chance de emendar outro passeio (`ai_rest_task::OnSessionEnd`).
//!
//! # O que não é igual, e por quê
//!
//! O monstro **de chão** anda sobre o mapa de movimento como o original (B99): perseguir e
//! voltar para casa pelo `follow_target` (perseguição dispersa sobre o agente sem bloqueio,
//! com a busca `CPf2DBfs`), passear pelo `cruise` — ver [`crate::navegacao`]. Sem mapa de
//! movimento ([`MonsterAi::tick`]), tudo é alcançável e o passo é uma reta. Monstro de água e
//! de ar ainda anda em linha reta, sem os agentes do habitat dele.
//!
//! # A unidade do `OBJECT_MOVE`
//!
//! `use_time` em **milissegundos** e `speed` em **1/256 de m/s** (`gs/npcsession.cpp:258`,
//! `(unsigned short)(GetSpeed()* 256.0f + 0.5f)`). Até 2026-09-12 ia centésimo de segundo e
//! centésimo de m/s: o cliente recebia um trecho de 2 m "para fazer em 50 ms" e o monstro
//! disparava — o "persegue muito rápido" do teste do POTATO.

use crate::entity::{MonsterEntity, PlayerEntity};
use crate::navegacao::{InfoDePerseguicao, Mapa, Passeio, SeguirAlvo, V3};
use pw_core::Vector3;
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonsterState {
    #[default]
    Idle,
    Patrol,
    Chasing,
    Attacking,
    Dead,
}

/// Onde o monstro se move — `inhabit_type` do `MONSTER_ESSENCE`, reduzido ao
/// `_inhabit_mode` do original (`gs/npcgenerator.cpp:114-143`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Habitat {
    #[default]
    Chao,
    Agua,
    Ar,
}

impl Habitat {
    /// `inhabit_type`: 0 chão, 1 água, 2 ar, 3 chão+água, 4 chão+ar, 5 água+ar, 6 todos.
    pub fn do_elements(inhabit_type: i32) -> Self {
        match inhabit_type {
            1 | 3 => Habitat::Agua,
            2 | 5 => Habitat::Ar,
            _ => Habitat::Chao,
        }
    }

    /// `STATE_NPC_FLY` 0x10000 / `STATE_NPC_SWIM` 0x20000 do `object_state`
    /// (`gnpc_imp::SetInhabitMode`, `gs/npc.cpp:823-843`; `gs/object.h:192-193`).
    pub fn estado_de_ambiente(self) -> i32 {
        match self {
            Habitat::Chao => 0,
            Habitat::Ar => 0x10000,
            Habitat::Agua => 0x20000,
        }
    }

    /// `gnpc_imp::GetMoveModeByInhabitType` (`gs/npc.h:232`): o bit de ambiente que vai no
    /// `move_mode` (`MOVE_MASK_SKY` 0x40, `MOVE_MASK_WATER` 0x80).
    pub fn mascara_de_movimento(self) -> u8 {
        match self {
            Habitat::Chao => 0,
            Habitat::Ar => 0x40,
            Habitat::Agua => 0x80,
        }
    }
}

/// `C2S::MOVE_MODE_WALK` / `MOVE_MODE_RUN` (`common/protocol.h:4523`).
pub const MODO_ANDAR: u8 = 0x00;
pub const MODO_CORRER: u8 = 0x01;
/// `C2S::MOVE_MODE_RETURN` (`common/protocol.h:4530`): o `ReturnHome` que põe o monstro de
/// volta em casa de uma vez (`gs/ainpc.cpp:98-106`).
pub const MODO_VOLTAR: u8 = 0x07;

/// O que o monstro decidiu fazer neste tique.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AcaoDoMonstro {
    /// Bateu em alguém. `fisico` é o dano físico **bruto** do golpe (`attack_msg.physic_damage`,
    /// antes da defesa) quando ele acertou um jogador corpo a corpo — o que os espinhos
    /// (`filter_Retort`) devolvem —, e 0 nos outros casos.
    Atacou { alvo: i64, dano: i32, fisico: i32 },
    /// Deu um passo até `destino`, que o cliente percorre em `tempo_ms`.
    Andou {
        destino: Vector3,
        tempo_ms: u16,
        velocidade: f32,
        modo: u8,
    },
    /// Parou em `posicao` (`OBJECT_STOP_MOVE`).
    Parou {
        posicao: Vector3,
        velocidade: f32,
        direcao: u8,
        modo: u8,
    },
    /// Começou a conjurar (`session_npc_skill::StartSession` → `NPCStartSkill` →
    /// `SkillWrapper::NpcStart`, `npcsession.cpp:654-731`, `skillwrapper.cpp:974-1004`): quem
    /// vê recebe o `OBJECT_CAST_SKILL` com o tempo do canto. `instantanea` (canto zero): o
    /// efeito sai já (`NPCEndSkill` na hora, `npcsession.cpp:708-713`).
    Conjurou {
        habilidade: HabilidadeDeCriatura,
        alvo: i64,
        instantanea: bool,
    },
    /// Fim do canto: o efeito (`NPCEndSkill` → `SkillWrapper::NpcEnd` → `NpcRun`).
    UsouHabilidade {
        habilidade: HabilidadeDeCriatura,
        alvo: i64,
    },
}

/// Uma habilidade de criatura com o que a IA precisa: tipo, área, alcance, canto e execução.
/// É o mesmo registro do mascote — o motor do original também é um só (`session_npc_skill`).
pub type HabilidadeDeCriatura = crate::mascote::HabilidadeDoMascote;

/// `ai_policy::STRATEGY_*` (`aipolicy.h:916-927`), do `id_strategy` do `MONSTER_ESSENCE`
/// (`npcgenerator.cpp:216`, `2600`). Cada uma vira uma tarefa em `AddPrimaryTask`
/// (`aipolicy.h:1214-1277`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Estrategia {
    /// 0 — `ai_melee_task`: persegue e bate.
    #[default]
    CorpoACorpo,
    /// 1 — `ai_range_task`: bate de longe e se afasta de quem chega perto.
    Distancia,
    /// 2 — `ai_magic_task`: só conjura.
    Magia,
    /// 3 — `ai_magic_melee_task`: conjura de longe; perto, uma série de golpes e depois
    /// conjura de novo.
    CorpoACorpoEMagia,
    /// 4 — `ai_fix_melee_task`: não sai do lugar; bate em quem está no alcance.
    Fixo,
    /// 5 — `ai_runaway_task`: foge.
    Fugitivo,
    /// 6 — `STRATEGY_STUB`: não faz nada.
    Inerte,
    /// 7 — `ai_fix_magic_task`: não sai do lugar; conjura em quem está no alcance.
    FixoMagico,
}

impl Estrategia {
    /// O original faz `ASSERT(primary_strategy < STRATEGY_MAX)` (`aipolicy.cpp:56`); fora da
    /// faixa fica o corpo a corpo.
    pub fn do_elements(id_strategy: i32) -> Self {
        match id_strategy {
            1 => Self::Distancia,
            2 => Self::Magia,
            3 => Self::CorpoACorpoEMagia,
            4 => Self::Fixo,
            5 => Self::Fugitivo,
            6 => Self::Inerte,
            7 => Self::FixoMagico,
            _ => Self::CorpoACorpo,
        }
    }
}

/// O evento de um limiar de vida (`_event_list`, `aipolicy.h:1343-1370`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventoDeVida {
    /// `FLEE_SKILL_ID` 40 (`config.h:86`) → `ai_runaway_task`.
    Fugir,
    /// Qualquer outra → `ai_skill_task`: conjura de onde está.
    Habilidade(HabilidadeDeCriatura),
}

/// `FLEE_SKILL_ID` (`gs/config.h:86`).
pub const HABILIDADE_DE_FUGA: i32 = 40;
/// `FLEE_RANGE` (`aipolicy.cpp:814`).
pub const ALCANCE_DA_FUGA: f32 = 30.0;
/// `ST_KO_COUNT` (`aipolicy.h:324`): quantas vezes a tarefa se afasta do alvo.
pub const VEZES_DE_AFASTAR: i32 = 2;
/// `NPC_FLEE_TIME` 0,5 s (`gs/config.h:108`): o passo de quem se afasta ou foge.
pub const PASSO_DE_FUGA_MS: u32 = 500;

/// O que o monstro sabe fazer em combate, montado do `MONSTER_ESSENCE` quando ele nasce
/// (`npc_template`, `npcgenerator.cpp:216-290`, e o `ai_param`, `:2560-2605`).
#[derive(Debug, Clone, Default)]
pub struct PerfilDeCombate {
    pub estrategia: Estrategia,
    /// `body_size` do monstro.
    pub corpo: f32,
    /// `skills.attack_skills` / `bless_skills` / `curse_skills`: separadas pelo `GetType`
    /// (1, 2, 3) do catálogo (`npcgenerator.cpp:252-285`).
    pub ataque: Vec<HabilidadeDeCriatura>,
    pub bencao: Vec<HabilidadeDeCriatura>,
    pub maldicao: Vec<HabilidadeDeCriatura>,
    /// `_event_list[0..3]`: 25 %, 50 % e 75 % de vida, sorteados uma vez
    /// (`npcgenerator.cpp:2579-2587`).
    pub eventos: [Option<EventoDeVida>; 3],
    /// Tem política no `aipolicy.data` (`_at_policy`): os eventos de vida não valem
    /// (`aipolicy.cpp:157`, `344`).
    pub tem_politica: bool,
    /// As habilidades que a política cita (`o_use_skill`), resolvidas no catálogo quando o
    /// monstro nasce, por `(id, nível)`.
    pub habilidades_da_politica: HashMap<(i32, i32), HabilidadeDeCriatura>,
}

/// A sessão de combate em curso (as `session_npc_*` que não são andar atrás do alvo).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Combate {
    #[default]
    Nenhum,
    /// `session_npc_skill`: canto e, depois do efeito, a execução.
    Conjurando {
        habilidade: HabilidadeDeCriatura,
        alvo: i64,
        falta_ms: u32,
        aplicado: bool,
    },
    /// `session_npc_keep_out` / `session_npc_flee`: correr para longe até `alcance`.
    Afastando { alcance: f32, restantes: i32 },
    /// Uma `session_npc_attack`/`session_npc_range_attack`: golpes seguidos enquanto as
    /// condições valem.
    Serie(Serie),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Serie {
    /// `_attack_times`: golpes que faltam (`None` = sem limite).
    restantes: Option<i32>,
    /// `CheckAttack`: `attack_range + corpo do alvo`.
    alcance: f32,
    /// `dis < _short_range`. O `dis` do `CheckAttack` é **ao quadrado** e o `_short_range` do
    /// `session_npc_attack` é linear (`npcsession.cpp:58-61`) — comparado como está. No
    /// `session_npc_range_attack` os dois são ao quadrado (`SetRange`, `npcsession.h:104-108`).
    curto: f32,
    /// `session_npc_range_attack` com `_auto_interrupt`: acaba abaixo de 60 % do alcance.
    interrompe: bool,
}

/// O `info.body_size` do alvo (`QueryTarget`): `PLAYER_BODYSIZE` do jogador, o `size` do
/// mascote (B139).
fn corpo_do_alvo(alvo: &AlvoDeCombate) -> f32 {
    alvo.mascote.map(|m| m.tamanho.max(0.0)).unwrap_or(crate::entity::CORPO_DO_JOGADOR)
}

/// O alvo da tarefa, resolvido neste tique.
struct AlvoDeCombate<'a> {
    id: i64,
    jogador: Option<&'a PlayerEntity>,
    mascote: Option<&'a MonsterEntity>,
    posicao: Vector3,
    distancia: f32,
}

/// `_state` das tarefas mágicas (`STATE_START, TRACE, DODGE, MAGIC, PHYSC`,
/// `aipolicy.h:401-404`); a de distância usa 1 (afastou) e 2 (golpeando) e começa em 1
/// (`ai_range_task(target)`, `aipolicy.h:333`).
const ESTADO_INICIO: u8 = 0;
const ESTADO_PERSEGUIR: u8 = 1;
const ESTADO_AFASTAR: u8 = 2;
const ESTADO_MAGIA: u8 = 3;
const ESTADO_GOLPES: u8 = 4;
const DISTANCIA_AFASTOU: u8 = 1;
const DISTANCIA_GOLPEANDO: u8 = 2;

impl AcaoDoMonstro {
    /// A velocidade na unidade do protocolo: 1/256 de m/s.
    pub fn velocidade_no_protocolo(velocidade: f32) -> i16 {
        (velocidade * 256.0 + 0.5).clamp(0.0, u16::MAX as f32) as u16 as i16
    }
}

/// A "sessão" em curso, no sentido do original: uma coisa de cada vez.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Sessao {
    #[default]
    Nenhuma,
    Perseguindo,
    Voltando,
    Passeando {
        destino: Vector3,
        passos_restantes: i32,
    },
    /// A `session_npc_patrol` do `ai_patrol_task` (B136): o ponto da rota em que está, os passos
    /// que restam (`SetTarget(pos, 120, ...)`, `aipolicy.cpp:1716-1726`) e onde a tarefa começou.
    Patrulhando {
        alvo: Vector3,
        restantes: i32,
        inicio: Vector3,
    },
    /// A `session_npc_follow_target` do `ai_follow_master` (`SetTarget(leader, 7, 20, 7)`,
    /// `aipolicy.cpp:1566-1572`).
    SeguindoLider,
}

/// `base_patrol_agent` (`gs/patrol_agent.h`): os pontos da rota (`path_manager`) e onde o
/// monstro está nela. `tipo` é o `iLoopType` do gerador.
#[derive(Debug, Clone)]
pub struct Rota {
    pontos: std::sync::Arc<Vec<Vector3>>,
    tipo: i32,
    indice: usize,
    fim: bool,
    adiante: bool,
    /// `_speed_flag` (`iSpeedFlag`): corre na rota em vez de andar.
    pub corre: bool,
}

impl Rota {
    /// `base_patrol_agent::Init`: rota de menos de dois pontos é recusada.
    pub fn nova(pontos: std::sync::Arc<Vec<Vector3>>, tipo: i32, corre: bool) -> Option<Self> {
        (pontos.len() >= 2).then_some(Self { pontos, tipo, indice: 0, fim: false, adiante: true, corre })
    }

    /// `base_patrol_agent::Reset`: do começo, indo adiante.
    pub fn recomecar(&mut self) {
        self.indice = 0;
        self.fim = false;
        self.adiante = true;
    }

    /// `GetCurWayPoint`: o ponto do índice atual (o próximo a pegar).
    pub fn atual(&self) -> Vector3 {
        self.pontos[self.indice]
    }

    /// `GetNextWayPoint` (`patrol_agent.h:52-99`): devolve o ponto do índice e avança — 0 para
    /// no fim, 1 vai e volta, 2 (e qualquer outro) recomeça do primeiro.
    pub fn proximo(&mut self) -> Option<Vector3> {
        if self.fim {
            return None;
        }
        let p = self.pontos[self.indice];
        let ultimo = self.pontos.len() - 1;
        match self.tipo {
            0 => {
                if self.indice < ultimo {
                    self.indice += 1;
                } else {
                    self.fim = true;
                }
            }
            1 => {
                if self.adiante {
                    if self.indice < ultimo {
                        self.indice += 1;
                    } else {
                        self.indice -= 1;
                        self.adiante = false;
                    }
                } else if self.indice > 0 {
                    self.indice -= 1;
                } else {
                    self.indice += 1;
                    self.adiante = true;
                }
            }
            _ => {
                self.indice = if self.indice < ultimo { self.indice + 1 } else { 0 };
            }
        }
        Some(p)
    }
}

/// Quem sabe a altura do chão de um ponto do mapa.
pub type Chao<'a> = &'a dyn Fn(f32, f32) -> Option<f32>;

#[derive(Debug, Clone)]
pub struct MonsterAi {
    pub state: MonsterState,
    pub aggro_table: HashMap<i64, i64>, // (Target EntityId -> Threat Value)
    pub attack_cooldown_ms: u32,
    sessao: Sessao,
    /// Quanto falta para o próximo passo da sessão.
    espera_ms: u32,
    batimento_ms: u32,
    /// `gnpc::idle_timer`: batimentos que ainda restam desde o último jogador por perto.
    idle_timer: i32,
    /// `gnpc::cruise_timer`, contador de 32.
    cruise_timer: u8,
    /// `gnpc::dir`, da última direção de passo.
    /// Para onde o monstro olha, em 1/256 de volta. Começa com a direção do gerador
    /// (`direcao_do_gerador`) e passa a ser a do último passo.
    pub direcao: u8,
    /// Já avisou a parada (o `_stop_flag` do original): um `OBJECT_STOP_MOVE` só.
    parado: bool,
    /// O `follow_target` da perseguição em curso (monstro de chão), o alvo dela, a direção
    /// de dispersão guardada entre sessões (`CChaseInfo`) e o `_reachable_count`.
    seguir: Option<SeguirAlvo>,
    perseguindo: Option<i64>,
    info: InfoDePerseguicao,
    chegadas: i32,
    /// O `follow_target` da volta para casa (`session_npc_patrol`).
    volta: Option<SeguirAlvo>,
    /// O `follow_target` da perseguição do monstro de ar ou de água (B133).
    no_espaco: Option<crate::navegacao::SeguirNoEspaco>,
    /// A perseguição acabou com `NSRC_ERR_PATHFINDING` neste tique (B134).
    caminho_falhou: bool,
    /// `aggro_policy::_cur_time`: batimentos que o primeiro da lista de ódio ainda dura sem
    /// renovar. Zero com a lista vazia.
    odio_restante: i32,
    /// Pedido de pôr o `_cur_time` de volta em `aggro_time` no próximo batimento (quem pede
    /// não tem o monstro à mão: [`Self::add_threat`]).
    renovar_odio: bool,
    /// O `cruise` do passeio.
    passeio: Option<Passeio>,
    /// A estratégia, as habilidades e os eventos de vida (sem perfil: corpo a corpo puro).
    pub perfil: Option<PerfilDeCombate>,
    combate: Combate,
    /// O alvo da tarefa atual: trocar de alvo abre outra tarefa, com o `_state`, o
    /// `_ko_count` e a habilidade escolhida do zero.
    tarefa_alvo: Option<i64>,
    estado_da_tarefa: u8,
    vezes_de_afastar: i32,
    /// `_skill`/`_skill_level`/`_skill_type` da tarefa mágica.
    habilidade_da_tarefa: Option<(HabilidadeDeCriatura, u8)>,
    /// `_policy_flag` do `GetPrimarySkill` (`aipolicy.h:1013-1057`).
    ordem_das_habilidades: u8,
    /// `_cur_event_hp` (`None` = `_quarter_hp × 3`, o de quem acabou de nascer).
    vida_do_evento: Option<i64>,
    evento_pendente: Option<EventoDeVida>,
    em_combate: bool,
    /// A política do `aipolicy.data` (`_at_policy`) e o estado dela neste monstro.
    pub politica: Option<(std::sync::Arc<crate::politica::PoliticaDeIa>, crate::politica::EstadoDaPolitica)>,
    /// `world::_common_data`: as variáveis globais do mundo, as mesmas para todo monstro.
    pub globais: Option<std::sync::Arc<std::sync::Mutex<HashMap<i32, i32>>>>,
    /// O que a política pediu ao mundo (fala, controlador do `npcgen`, operação sem porte).
    pub pedidos_ao_mundo: Vec<crate::politica::Pedido>,
    /// As tarefas que a política pôs na fila (`ai_policy::AddTask`): começam quando a sessão
    /// atual acaba (`ai_target_task::OnHeartbeat`).
    tarefas_da_politica: std::collections::VecDeque<crate::politica::Pedido>,
    /// `ai_skill_task_2` em curso: a habilidade e o alvo.
    tarefa_de_habilidade: Option<(HabilidadeDeCriatura, i64)>,
    /// `op_attack` → `AddPrimaryTask(target, uType)`: a estratégia da tarefa até o fim do
    /// combate.
    estrategia_forcada: Option<Estrategia>,
    fim_de_combate_pendente: bool,
    /// A rota de patrulha (`ai_policy::_path_agent`, B136).
    pub rota: Option<Rota>,
    /// O líder do grupo (`gnpc_imp::_leader_id`), e onde ele está: o mundo põe a cada tique,
    /// `None` com o líder morto (`QueryTarget(leader) != TARGET_STATE_NORMAL`).
    pub lider: Option<i64>,
    pub lider_em: Option<Vector3>,
    /// É o chefe de um `boss_spawner` (`group_boss_policy`): repassa o ódio aos subordinados.
    pub chefe: bool,
    /// `group_boss_policy::_enemy` e o repasse que o mundo ainda vai entregar
    /// (`ForwardFirstAggro` → `GM_MSG_TRANSFER_AGGRO`).
    inimigo_repassado: Option<i64>,
    pub odio_a_repassar: Option<(i64, i64)>,
    /// O agente da `session_npc_patrol` e do `follow_target` atrás do líder.
    patrulha: Option<SeguirAlvo>,
    /// Para onde a volta vai: o ponto atual da rota (`RollBack` com `_path_agent`,
    /// `aipolicy.cpp:214-220`); `None` é a casa.
    volta_para: Option<Vector3>,
    /// Centro, raio e passos do passeio em volta do líder (`session_npc_cruise::SetTarget(info.pos,
    /// 6, 7)`, `aipolicy.cpp:1576-1580`); `None` é o passeio de sempre, em volta de casa.
    centro_do_passeio: Option<(Vector3, f32, i32)>,
    /// O `ReturnHome(leader, 7)` pendente: um `stop_move` com `MOVE_MODE_RETURN`.
    teleporte: Option<Vector3>,
    /// A `session_npc_attack` do corpo a corpo em curso (o alcance de continuar é maior).
    golpeando_perto: bool,
}

impl Default for MonsterAi {
    fn default() -> Self {
        Self::new()
    }
}

impl MonsterAi {
    /// `NPC_FOLLOW_TARGET_TIME` (`gs/config.h:107`).
    pub const PASSO_DE_PERSEGUICAO_MS: u32 = 500;
    /// `NPC_PATROL_TIME` (`gs/config.h:116`) e o temporizador de 20 tiques do `cruise`.
    pub const PASSO_DE_PATRULHA_MS: u32 = 1000;
    /// `world_manager::GetMaxMobSightRange()` — 15 m (`gs/worldmanager.cpp:48`): até onde o
    /// aviso de movimento do jogador chega aos monstros agressivos.
    pub const ALCANCE_DE_VISAO: f32 = 15.0;
    /// O batimento do NPC.
    pub const BATIMENTO_MS: u32 = 1000;
    /// `NPC_IDLE_TIMER` (`gs/config.h:40`).
    pub const BATIMENTOS_OCIOSO: i32 = 20;
    /// `SetTarget(pos, 8, 10.0f)` do `ai_rest_task::Execute`.
    pub const RAIO_DO_PASSEIO: f32 = 10.0;
    pub const PASSOS_DO_PASSEIO: i32 = 8;
    /// `abase::Rand(0,1) < 0.1f` do `ai_rest_task::OnSessionEnd`.
    pub const CHANCE_DE_EMENDAR_PASSEIO: f64 = 0.1;
    /// `session_npc_patrol::SetTarget(pos, 120, ...)` do `ai_patrol_task` (`aipolicy.cpp:1722`).
    pub const PASSOS_DA_PATRULHA: i32 = 120;
    /// `MAX_MASTER_MINOR_RANGE` (`config.h:105`), comparado com a distância horizontal ao
    /// quadrado: 20 m.
    pub const LONGE_DO_LIDER_AO_QUADRADO: f32 = 400.0;
    /// O 7 do `ReturnHome(info.pos, 7)`, do `SetTarget(_target, 7, 20, 7)` e do raio do passeio
    /// em volta do líder (`aipolicy.cpp:1560-1580`).
    pub const PERTO_DO_LIDER: f32 = 7.0;
    /// Até onde um jogador conta como "por perto" para o monstro não ficar ocioso. É o raio
    /// de visão do `bus_server`: quem está vendo o monstro.
    pub const RAIO_DE_ATIVIDADE: f32 = 120.0;
    /// Piso da distância de perseguição, para monstro cujo `aggro_range` é pequeno demais
    /// para ele sair do lugar.
    pub const PERSEGUICAO_MINIMA: f32 = 15.0;
    /// `ai_policy::GetReturnHomeRange()` = 10² (`aipolicy.h:1393`): no `RollBack`, só volta
    /// para casa (invencível) quem está a mais de 10 m de onde nasceu (`IsReturnHome`,
    /// `ainpc.cpp:28-37`, compara a distância ao quadrado).
    pub const DISTANCIA_PARA_VOLTAR_AO_QUADRADO: f32 = 100.0;
    /// `GetInvincibleTimeout()` = 22 (`aipolicy.h:1403`; no `gs` 1.2.6 o `push 0x16` do
    /// `ai_returnhome_task::StartTask`, VA 0x80db6e5): batimentos de invencível na volta.
    pub const INVENCIVEL_NA_VOLTA_S: i32 = 22;

    /// Até onde o agressivo nota quem anda perto: o aviso `GM_MSG_WATCHING_YOU` sai do
    /// jogador para `GetMaxMobSightRange()` metros (15, `playerctrl.cpp:265-276`,
    /// `worldmanager.cpp:48`), e o `gnpc_ai::AggroWatch` só o aceita a menos de
    /// `sight_range + body_size` (`ainpc.h:851-854`, `ainpc.cpp:251-252`). Os monstros do
    /// começo têm 6–8 m de `sight_range`: usar os 15 m para todos os fazia notar o jogador
    /// de longe demais (teste da Tsuko, 2026-09-26).
    pub fn raio_de_deteccao(monster: &MonsterEntity) -> f32 {
        (monster.sight_range.max(0) as f32 + monster.tamanho.max(0.0)).min(Self::ALCANCE_DE_VISAO)
    }

    pub fn new() -> Self {
        Self {
            state: MonsterState::Idle,
            aggro_table: HashMap::new(),
            attack_cooldown_ms: 0,
            sessao: Sessao::Nenhuma,
            // A fase do batimento de 1 s, sorteada por monstro. O original **não** bate em
            // todos ao mesmo tempo: o coletor pega `tamanho / TICK_PER_SEC` objetos por tique
            // (`obj_manager::CollectHeartbeatObject`, `objmanager.h:213-229`, com
            // `obj_manager<gnpc, TICK_PER_SEC>` em `worldmanager.h:262`), e cada NPC ainda
            // começa com `idle_timer_count = Rand(0, NPC_IDLE_HEARTBEAT)`
            // (`npcgenerator.cpp:2014`). Com todos batendo no mesmo limite de 1 s, dez
            // monstros davam o passo no mesmo quadro e o cliente tocava dez sons de passo
            // sobrepostos — o "ruído muito alto" do teste de 2026-09-17 (B58).
            espera_ms: rand::thread_rng().gen_range(0..Self::PASSO_DE_PATRULHA_MS),
            batimento_ms: rand::thread_rng().gen_range(0..Self::BATIMENTO_MS),
            idle_timer: 0,
            // O original não sincroniza os monstros: cada um começa num ponto do contador.
            cruise_timer: rand::thread_rng().gen_range(0..32),
            direcao: 0,
            parado: true,
            seguir: None,
            perseguindo: None,
            info: InfoDePerseguicao::default(),
            chegadas: 0,
            volta: None,
            no_espaco: None,
            caminho_falhou: false,
            odio_restante: 0,
            renovar_odio: false,
            passeio: None,
            perfil: None,
            combate: Combate::Nenhum,
            tarefa_alvo: None,
            estado_da_tarefa: ESTADO_INICIO,
            vezes_de_afastar: VEZES_DE_AFASTAR,
            habilidade_da_tarefa: None,
            ordem_das_habilidades: 0,
            vida_do_evento: None,
            evento_pendente: None,
            em_combate: false,
            politica: None,
            globais: None,
            pedidos_ao_mundo: Vec::new(),
            tarefas_da_politica: std::collections::VecDeque::new(),
            tarefa_de_habilidade: None,
            estrategia_forcada: None,
            fim_de_combate_pendente: false,
            rota: None,
            lider: None,
            lider_em: None,
            chefe: false,
            inimigo_repassado: None,
            odio_a_repassar: None,
            patrulha: None,
            volta_para: None,
            centro_do_passeio: None,
            teleporte: None,
            golpeando_perto: false,
        }
    }

    /// Roda um evento da política com o mundo à volta. As tarefas que ela pede vão para a
    /// fila da IA; o resto, para o mundo.
    fn rodar_politica(
        &mut self,
        monster: &MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        mascotes: &HashMap<i64, MonsterEntity>,
        ultimo_dano: i32,
        matador: Option<i64>,
        f: impl FnOnce(&crate::politica::PoliticaDeIa, &mut crate::politica::EstadoDaPolitica, &mut crate::politica::Contexto),
    ) {
        use crate::politica::{Contexto, InfoDoAlvo, Pedido};
        let Some((politica, mut estado)) = self.politica.take() else { return };
        let info = |id: i64| -> Option<InfoDoAlvo> {
            players
                .get(&id)
                .filter(|p| p.hp > 0)
                .map(|p| InfoDoAlvo { hp: p.hp, mp: p.mp, classe: Some(p.cls as i32), posicao: p.position, dono: None })
                .or_else(|| {
                    mascotes
                        .get(&id)
                        .filter(|m| !m.is_dead)
                        .map(|m| InfoDoAlvo { hp: m.hp as i32, mp: m.mp, classe: None, posicao: m.position, dono: None })
                })
        };
        let pos = monster.position;
        let perto = |raio: Option<f32>, caixa: Option<(f32, f32, f32, f32)>| -> i32 {
            players
                .values()
                .filter(|p| p.hp > 0)
                .filter(|p| match (raio, caixa) {
                    (Some(r), _) => p.position.distance(&pos) <= r,
                    (None, Some((x0, z0, x1, z1))) => {
                        (x0.min(x1)..=x0.max(x1)).contains(&p.position.x) && (z0.min(z1)..=z0.max(z1)).contains(&p.position.z)
                    }
                    _ => false,
                })
                .count() as i32
        };
        let mut sem_mundo = HashMap::new();
        let globais_arc = self.globais.clone();
        let mut guarda = globais_arc.as_ref().map(|g| g.lock().unwrap_or_else(|e| e.into_inner()));
        let globais: &mut HashMap<i32, i32> = match guarda.as_mut() {
            Some(g) => g,
            None => &mut sem_mundo,
        };
        let mut ctx = Contexto {
            eu: monster.id,
            hp: monster.hp,
            max_hp: monster.max_hp,
            posicao: pos,
            odio: &mut self.aggro_table,
            info: &info,
            jogadores_perto: &perto,
            globais,
            locais: estado.locais,
            ultimo_dano,
            matador,
            pedidos: Vec::new(),
        };
        f(&politica, &mut estado, &mut ctx);
        let pedidos = std::mem::take(&mut ctx.pedidos);
        drop(ctx);
        drop(guarda);
        self.politica = Some((politica, estado));
        for p in pedidos {
            match p {
                Pedido::Atacar { .. } | Pedido::Habilidade { .. } | Pedido::Fugir { .. } => self.tarefas_da_politica.push_back(p),
                outro => self.pedidos_ao_mundo.push(outro),
            }
        }
    }

    /// `ai_policy::OnDeath` → `_at_policy->OnDeath()` + `ResetAll()`.
    pub fn ao_morrer(&mut self, monster: &MonsterEntity, players: &HashMap<i64, PlayerEntity>, matador: Option<i64>) {
        self.rodar_politica(monster, players, &HashMap::new(), 0, matador, |p, e, c| p.morte(e, c));
        self.tarefas_da_politica.clear();
        self.tarefa_de_habilidade = None;
        self.estrategia_forcada = None;
        self.combate = Combate::Nenhum;
    }

    /// `gnpc_controller::OnDamage` → `_at_policy->OnDamage()`, com o `GetLastDamage`.
    pub fn ao_apanhar(&mut self, monster: &MonsterEntity, players: &HashMap<i64, PlayerEntity>, dano: i32) {
        if self.politica.is_some() {
            self.rodar_politica(monster, players, &HashMap::new(), dano, None, |p, e, c| p.dano(e, c));
        }
    }

    /// `ai_policy::KillTarget` (`npc.cpp:685`): o monstro matou o alvo.
    pub fn ao_matar_o_alvo(&mut self, monster: &MonsterEntity, players: &HashMap<i64, PlayerEntity>) {
        if self.politica.is_some() {
            self.rodar_politica(monster, players, &HashMap::new(), 0, None, |p, e, c| p.matou_alvo(e, c));
        }
    }

    /// `ai_skill_task_2::Execute` (`aipolicy.cpp:1963-2051`) e as outras tarefas da política.
    /// `None`: nada da política agora, a estratégia decide.
    ///
    /// **Não é igual:** o original persegue no máximo duas vezes (`_trace_count`) e depois
    /// conjura de onde estiver; aqui persegue até entrar no alcance (ou conjura parado se
    /// estiver preso).
    fn tarefa_da_politica(
        &mut self,
        monster: &mut MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        mascotes: &HashMap<i64, MonsterEntity>,
        mapa: &Mapa,
        chao: Chao,
    ) -> Option<Option<AcaoDoMonstro>> {
        use crate::politica::Pedido;
        if !matches!(self.combate, Combate::Nenhum | Combate::Serie(_)) {
            return None;
        }
        if self.tarefa_de_habilidade.is_none() {
            match self.tarefas_da_politica.pop_front()? {
                Pedido::Atacar { estrategia, .. } => {
                    self.estrategia_forcada = Some(Estrategia::do_elements(estrategia));
                    if let Some(a) = self.tarefa_alvo {
                        self.nova_tarefa(a);
                    }
                    return None;
                }
                Pedido::Fugir { .. } => {
                    let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
                    self.espera_ms = 0;
                    self.combate = Combate::Afastando { alcance: (ALCANCE_DA_FUGA + corpo) * 0.9, restantes: 8 };
                    return None;
                }
                Pedido::Habilidade { alvo, id, nivel } => {
                    let h = self.perfil.as_ref().and_then(|p| p.habilidades_da_politica.get(&(id, nivel)).copied());
                    match h {
                        Some(h) => self.tarefa_de_habilidade = Some((h, alvo)),
                        None => {
                            self.pedidos_ao_mundo.push(Pedido::NaoPortado("habilidade da política fora do catálogo"));
                            return None;
                        }
                    }
                }
                _ => return None,
            }
        }
        let (h, alvo_id) = self.tarefa_de_habilidade?;
        if crate::mascote::area_sem_alvo(h.area) || alvo_id == monster.id {
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(Some(a));
            }
            self.tarefa_de_habilidade = None;
            return Some(Some(self.conjurar(h, monster.id)));
        }
        let jogador = players.get(&alvo_id).filter(|p| p.hp > 0);
        let mascote = mascotes.get(&alvo_id).filter(|m| !m.is_dead && m.hp > 0);
        let Some(posicao) = jogador.map(|p| p.position).or(mascote.map(|m| m.position)) else {
            self.tarefa_de_habilidade = None;
            return None;
        };
        let distancia = monster.position.distance(&posicao);
        let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
        let corpo_alvo = mascote.map(|m| m.tamanho.max(0.0)).unwrap_or(crate::entity::CORPO_DO_JOGADOR);
        let alcance = h.alcance + corpo + corpo_alvo;
        let estrategia = self.estrategia_forcada.or(self.perfil.as_ref().map(|p| p.estrategia));
        let fixo = matches!(estrategia, Some(Estrategia::Fixo | Estrategia::FixoMagico));
        if fixo || distancia * distancia <= alcance * alcance * 0.81 || monster.efeitos.preso() {
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(Some(a));
            }
            self.tarefa_de_habilidade = None;
            return Some(Some(self.conjurar(h, alvo_id)));
        }
        let alvo = AlvoDeCombate { id: alvo_id, jogador, mascote, posicao, distancia };
        let a = self.aproximar(monster, &alvo, alcance * 0.9, mapa, chao);
        if !self.aggro_table.contains_key(&alvo_id) {
            self.tarefa_de_habilidade = None;
        }
        Some(a)
    }

    /// Um monstro novo com o perfil de combate.
    pub fn com_perfil(perfil: Option<PerfilDeCombate>) -> Self {
        let mut ia = Self::new();
        ia.perfil = perfil;
        ia
    }

    /// Está conjurando (a sessão de habilidade ocupa o monstro).
    pub fn conjurando(&self) -> bool {
        matches!(self.combate, Combate::Conjurando { .. })
    }

    /// Adiciona ameaça a um jogador (`aggro_policy::AddAggro`/`AggroGen`, `ainpc.h:188-231`).
    ///
    /// O `_cur_time` volta a `aggro_time` quando a lista estava vazia ou quando quem ganhou
    /// ódio é o primeiro dela (`AddRage(...) == 0`). Voltando para casa o monstro não aceita
    /// ódio: `ai_returnhome_task::OnAggro` o limpa (`aipolicy.cpp:1280-1288`).
    pub fn add_threat(&mut self, player_id: i64, threat: i64) {
        if matches!(self.sessao, Sessao::Voltando) {
            return;
        }
        let estava_vazia = self.aggro_table.is_empty();
        let entry = self.aggro_table.entry(player_id).or_insert(0);
        *entry += threat;
        if estava_vazia || self.get_highest_threat_target() == Some(player_id) {
            self.renovar_odio = true;
        }
    }

    /// `RefreshAggroTimer` (`npc.h:125-128`, `ainpc.h:147-153`): o golpe do monstro no
    /// primeiro da lista renova o `_cur_time` (`session_npc_attack`, `range_attack` e
    /// `skill`, `npcsession.cpp:73`, `284`, `690`, `741`).
    fn renovar_odio_se_primeiro(&mut self, alvo: i64) {
        if self.get_highest_threat_target() == Some(alvo) {
            self.renovar_odio = true;
        }
    }

    /// Batimentos que o ódio do primeiro da lista ainda dura (para os testes e o log).
    pub fn odio_restante(&self) -> i32 {
        self.odio_restante
    }

    /// Está voltando para casa (`ai_returnhome_task`).
    pub fn esta_voltando(&self) -> bool {
        matches!(self.sessao, Sessao::Voltando)
    }

    /// O batimento do ódio e da vida, uma vez por segundo.
    ///
    /// * `aggro_policy::OnHeartbeat` (`ainpc.h:259-275`): o `_cur_time` desce; ao zerar, o
    ///   primeiro sai da lista (`RemoveFirst`) e, sobrando alguém, conta de novo. É isto que
    ///   faz o monstro desistir de quem só foge ou voa: só bater, apanhar do primeiro da
    ///   lista ou o `RefreshAggroTimer` dos golpes renovam.
    /// * `gnpc_imp::OnHeartbeat` (`npc.cpp:1946-1958`): em combate (ou com `_fast_regen`
    ///   desligado) regenera `hp_gen`; fora de combate, com `hp_gen` diferente de zero, enche
    ///   a vida inteira.
    fn batimento_de_odio_e_vida(&mut self, monster: &mut MonsterEntity) {
        if std::mem::take(&mut self.renovar_odio) && !self.aggro_table.is_empty() {
            self.odio_restante = monster.tempo_de_odio_s.max(1);
        } else if self.odio_restante > 0 {
            self.odio_restante -= 1;
            if self.odio_restante == 0 {
                if let Some(primeiro) = self.get_highest_threat_target() {
                    self.aggro_table.remove(&primeiro);
                }
                if !self.aggro_table.is_empty() {
                    self.odio_restante = monster.tempo_de_odio_s.max(1);
                }
            }
        }
        if self.aggro_table.is_empty() {
            self.odio_restante = 0;
        }

        if monster.hp > 0 && monster.hp < monster.max_hp {
            let regen = monster.regeneracao_de_vida as i64;
            if self.em_combate || regen == 0 {
                monster.hp = (monster.hp + regen).clamp(0, monster.max_hp);
            } else {
                monster.hp = monster.max_hp;
            }
        }
    }

    /// `ai_policy::RollBack` (`aipolicy.cpp:204-231`), no que toca ao lugar: longe de casa
    /// mais de 10 m, a tarefa de volta (`ai_returnhome_task::StartTask`,
    /// `aipolicy.cpp:1291-1304`) põe o monstro invencível por 22 batimentos
    /// (`SetInvincibleFilter(true, 22)`; no 1.5.5 o filtro liga o estado visível 49, o
    /// efeito sobre o monstro) e o leva de volta correndo.
    fn voltar_para_casa(&mut self, monster: &mut MonsterEntity) {
        self.seguir = None;
        self.no_espaco = None;
        self.perseguindo = None;
        self.passeio = None;
        self.volta = None;
        self.patrulha = None;
        self.state = MonsterState::Idle;
        // Subordinado com o líder vivo: `GetReturnHomeRange` = 1e20 (`aipolicy.cpp:1360-1369`),
        // não volta — o `ai_follow_master` o leva de novo ao líder.
        if self.lider_em.is_some() {
            self.sessao = Sessao::Nenhuma;
            return;
        }
        // Com rota, a volta é sempre, ao ponto atual dela (`aipolicy.cpp:214-220`).
        self.volta_para = self.rota.as_ref().map(|r| r.atual());
        let casa = self.casa(monster);
        let longe = self.volta_para.is_some()
            || monster.position.distance(&casa).powi(2) > Self::DISTANCIA_PARA_VOLTAR_AO_QUADRADO;
        if longe {
            self.sessao = Sessao::Voltando;
            self.espera_ms = 0;
            monster.efeitos.invencivel_s =
                monster.efeitos.invencivel_s.max(Self::INVENCIVEL_NA_VOLTA_S);
        } else {
            self.sessao = Sessao::Nenhuma;
        }
    }

    /// `ai_returnhome_task::EndTask` (`aipolicy.cpp:1307-1320`): tira o invencível.
    fn fim_da_volta(&mut self, monster: &mut MonsterEntity) {
        self.sessao = Sessao::Nenhuma;
        self.volta_para = None;
        monster.efeitos.invencivel_s = 0;
    }

    /// Para onde a volta vai: o ponto da rota, ou onde nasceu.
    fn casa(&self, monster: &MonsterEntity) -> Vector3 {
        self.volta_para.unwrap_or(monster.spawn_center)
    }

    /// `GM_MSG_TRANSFER_AGGRO` no subordinado do chefe (`aggro_minor_policy::AggroTransfer`,
    /// `aiaggro.h:11-27`): a lista vira só o inimigo do chefe, com a mesma raiva, e o
    /// `_cur_time` volta a `aggro_time`.
    pub fn receber_odio_do_chefe(&mut self, alvo: i64, raiva: i64) {
        self.aggro_table.clear();
        self.aggro_table.insert(alvo, raiva.max(1));
        self.renovar_odio = true;
    }

    /// Retorna o alvo com maior ameaça
    pub fn get_highest_threat_target(&self) -> Option<i64> {
        self.aggro_table
            .iter()
            .max_by_key(|(_, &threat)| threat)
            .map(|(&id, _)| id)
    }

    /// Está passeando (para os testes e para o log).
    pub fn esta_passeando(&self) -> bool {
        matches!(self.sessao, Sessao::Passeando { .. })
    }

    /// Atualiza o ciclo de IA do monstro a cada tick (50ms).
    ///
    /// `chao` dá a altura do terreno num ponto; `None` fora do mapa de alturas.
    pub fn tick(
        &mut self,
        monster: &mut MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        delta_ms: u32,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        // Sem mapa de movimento: tudo alcançável, e o monstro de chão anda em linha reta.
        let vazio = pw_data_loader::MapaDeMovimento::vazio();
        self.tick_no_mapa(
            monster,
            players,
            delta_ms,
            &Mapa {
                terreno: chao,
                movimento: &vazio,
                espaco: None,
                agua: None,
            },
        )
    }

    /// O ciclo da IA com o mapa inteiro: terreno **e** mapa de movimento. É por ele que o
    /// monstro de chão contorna obstáculo ([`crate::navegacao`]).
    pub fn tick_no_mapa(
        &mut self,
        monster: &mut MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        delta_ms: u32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        self.tick_com_mascotes(monster, players, &HashMap::new(), delta_ms, mapa)
    }

    /// O ciclo da IA com os mascotes de combate também como alvo: o monstro que apanha de
    /// um mascote o odeia (`AddAggroEntry(msg.source, ...)`, `npc.cpp:1781`), e o
    /// agressivo o vê como vê o jogador — o mascote difunde o `GM_MSG_WATCHING_YOU` como
    /// ele (`gpet_imp::PeepEnemy`, `petnpc.cpp:1359-1374`). `mascotes`: o corpo de cada um.
    pub fn tick_com_mascotes(
        &mut self,
        monster: &mut MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        mascotes: &HashMap<i64, MonsterEntity>,
        delta_ms: u32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        let acao = self.tick_interno(monster, players, mascotes, delta_ms, mapa);
        match acao {
            Some(AcaoDoMonstro::Atacou { alvo, .. })
            | Some(AcaoDoMonstro::Conjurou { alvo, .. })
            | Some(AcaoDoMonstro::UsouHabilidade { alvo, .. }) => {
                self.renovar_odio_se_primeiro(alvo)
            }
            _ => {}
        }
        acao
    }

    fn tick_interno(
        &mut self,
        monster: &mut MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        mascotes: &HashMap<i64, MonsterEntity>,
        delta_ms: u32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        // O piso para o monstro de água e de ar: terreno + estrutura.
        let piso = |x: f32, z: f32| {
            (mapa.terreno)(x, z).map(|h| h + mapa.movimento.acima_do_terreno(x, z).unwrap_or(0.0))
        };
        let chao: Chao = &piso;
        if monster.is_dead {
            self.state = MonsterState::Dead;
            self.sessao = Sessao::Nenhuma;
            self.parado = true;
            return None;
        }

        self.attack_cooldown_ms = self.attack_cooldown_ms.saturating_sub(delta_ms);
        self.espera_ms = self.espera_ms.saturating_sub(delta_ms);

        // `RollBack` → `EnableCombat(false, true)`: `Reset` + `EndCombat` da política.
        if std::mem::take(&mut self.fim_de_combate_pendente) {
            self.rodar_politica(monster, players, mascotes, 0, None, |p, e, c| p.fim_de_combate(e, c));
        }

        // `IncIdleSealMode(MODE_INDEX_STUN/SLEEP)` (`filter_Dizzy`, `filter_Sleep`): parado,
        // sem golpe nem passo, até o filtro sair. `gnpc_imp::SetIdleMode` (`npc.cpp:2129-2138`)
        // faz `ClearSession`: o canto em curso acaba, não fica pausado (B140).
        if monster.efeitos.sem_acao() {
            if matches!(self.combate, Combate::Conjurando { .. } | Combate::Serie(_) | Combate::Afastando { .. }) {
                self.combate = Combate::Nenhum;
            }
            self.seguir = None;
            if !self.parado {
                return Some(self.parar(monster, monster.corrida(), MODO_CORRER));
            }
            return None;
        }

        self.batimento_ms += delta_ms;
        while self.batimento_ms >= Self::BATIMENTO_MS {
            self.batimento_ms -= Self::BATIMENTO_MS;
            self.batimento_de_odio_e_vida(monster);
            self.batimento(monster, players);
            self.conferir_evento_de_vida(monster);
            // `ai_policy::OnHeartbeat`: parado sem ninguém por perto (`_idle_mode`), nada.
            if self.politica.is_some() && (self.em_combate || self.idle_timer > 0) {
                let em_combate = self.em_combate;
                self.rodar_politica(monster, players, mascotes, 0, None, |p, e, c| p.batimento(e, c, em_combate));
            }
        }

        // A sessão de habilidade ocupa o monstro até o fim, com ou sem alvo.
        if let Some(acao) = self.andamento_da_conjuracao(delta_ms) {
            return acao;
        }

        // 0. Monstro agressivo procura briga: sem alvo, ele pega o jogador vivo mais perto
        // dentro do alcance de visão. No original é o **jogador** que avisa ao andar —
        // `GM_MSG_WATCHING_YOU` difundido a quem tem `MSG_MASK_PLAYER_MOVE`, a marca que só
        // o monstro com `aggressive_mode` recebe (`npcgenerator.cpp:2534-2537`,
        // `playerctrl.cpp:265-276`) —, num raio de `GetMaxMobSightRange`, 15 m
        // (`worldmanager.cpp:48`). Quem decide odiar é a política do `aipolicy.data`, que
        // ainda não interpretamos: aqui o agressivo odeia sempre (B76).
        // Voltando para casa o `invincible_filter` tira o `MSG_MASK_PLAYER_MOVE` do monstro
        // (`invincible_filter.cpp:13-22`): não nota ninguém.
        let raio = Self::raio_de_deteccao(monster);
        if monster.agressivo
            && self.get_highest_threat_target().is_none()
            && !matches!(self.sessao, Sessao::Voltando)
        {
            let mais_perto = players
                .iter()
                .filter(|(_, p)| p.hp > 0)
                .map(|(id, p)| (*id, monster.position.distance(&p.position)))
                .chain(
                    mascotes
                        .iter()
                        .filter(|(_, m)| !m.is_dead && m.hp > 0)
                        .map(|(id, m)| (*id, monster.position.distance(&m.position))),
                )
                .filter(|(_, d)| *d < raio)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((id, _)) = mais_perto {
                self.add_threat(id, 1);
            }
        }

        // 1. Com alvo: a tarefa da estratégia (`ai_policy::DeterminePolicy` → `AddPrimaryTask`,
        // `aipolicy.cpp:179-200`, `aipolicy.h:1214-1277`).
        if let Some(target_id) = self.get_highest_threat_target() {
            let jogador = players.get(&target_id).filter(|p| p.hp > 0);
            let mascote = mascotes.get(&target_id).filter(|m| !m.is_dead && m.hp > 0);
            let posicao_do_alvo = match (jogador, mascote) {
                (Some(p), _) => p.position,
                (None, Some(m)) => m.position,
                (None, None) => {
                    self.aggro_table.remove(&target_id);
                    return self.sem_alvo(monster, mapa);
                }
            };
            if !self.em_combate {
                // `ai_policy::OnAggro` → `EnableCombat(true)` → `StartCombat`.
                self.em_combate = true;
                self.rodar_politica(monster, players, mascotes, 0, None, |p, e, c| p.comeco_de_combate(e, c));
            }
            if self.tarefa_alvo != Some(target_id) {
                self.nova_tarefa(target_id);
            }
            if let Some(a) = self.tarefa_da_politica(monster, players, mascotes, mapa, chao) {
                return a;
            }
            let distancia = monster.position.distance(&posicao_do_alvo);
            let alvo = AlvoDeCombate {
                id: target_id,
                jogador,
                mascote,
                posicao: posicao_do_alvo,
                distancia,
            };
            return self.combater(monster, &alvo, mapa, chao);
        }

        self.sem_alvo(monster, mapa)
    }

    // ------------------------------------------------------------------
    // Combate por estratégia — `aipolicy.cpp:515-1110`, `npcsession.cpp`.
    // ------------------------------------------------------------------

    /// Outra tarefa, para outro alvo: o `_state`, o `_ko_count` e a habilidade escolhida
    /// recomeçam (cada `AddTargetTask` constrói a tarefa do zero).
    fn nova_tarefa(&mut self, alvo: i64) {
        self.tarefa_alvo = Some(alvo);
        self.golpeando_perto = false;
        self.habilidade_da_tarefa = None;
        self.vezes_de_afastar = VEZES_DE_AFASTAR;
        self.estado_da_tarefa = match self.perfil.as_ref().map(|p| p.estrategia) {
            Some(Estrategia::Distancia) => DISTANCIA_AFASTOU,
            _ => ESTADO_INICIO,
        };
        if matches!(self.combate, Combate::Serie(_) | Combate::Afastando { .. }) {
            self.combate = Combate::Nenhum;
        }
    }

    /// `ai_policy::RollBack` (`aipolicy.cpp:204-231`): sem ninguém na lista de ódio, o
    /// `_cur_event_hp` passa a ser a vida atual e o `_policy_flag` volta a zero.
    fn sair_de_combate(&mut self, monster: &MonsterEntity) {
        if !self.em_combate {
            return;
        }
        self.em_combate = false;
        self.fim_de_combate_pendente = self.politica.is_some();
        self.tarefas_da_politica.clear();
        self.tarefa_de_habilidade = None;
        self.estrategia_forcada = None;
        self.vida_do_evento = Some(monster.hp);
        self.ordem_das_habilidades = 0;
        self.tarefa_alvo = None;
        self.habilidade_da_tarefa = None;
        self.evento_pendente = None;
        if matches!(self.combate, Combate::Serie(_) | Combate::Afastando { .. }) {
            self.combate = Combate::Nenhum;
        }
    }

    /// O fim do `ai_policy::OnHeartbeat` (`aipolicy.cpp:344-360`) com o `TriggerEvent`
    /// (`aipolicy.h:1343-1370`): só em combate e só sem política do `aipolicy.data`. O
    /// índice do evento é `_cur_event_hp / _quarter_hp − 1` (2 = 75 %, 1 = 50 %, 0 = 25 %), e o
    /// `_cur_event_hp` desce de quarto em quarto até ficar abaixo da vida — então uma queda
    /// grande dispara **um** evento, o do primeiro limiar cruzado.
    fn conferir_evento_de_vida(&mut self, monster: &MonsterEntity) {
        let Some(perfil) = self.perfil.as_ref() else {
            return;
        };
        if perfil.tem_politica || !self.em_combate || monster.max_hp <= 0 {
            return;
        }
        let quarto = (monster.max_hp >> 2).max(1);
        let atual = self.vida_do_evento.get_or_insert(quarto * 3);
        let hp = monster.hp;
        if hp < *atual {
            let evento = *atual / quarto - 1;
            while hp < *atual {
                *atual -= quarto;
            }
            if (0..3).contains(&evento) {
                if let Some(e) = perfil.eventos[evento as usize] {
                    self.evento_pendente = Some(e);
                }
            }
        } else {
            let h = hp - quarto;
            while h > *atual {
                *atual += quarto;
            }
        }
    }

    /// `GetPrimarySkill` (`aipolicy.h:1013-1057`): a primeira vez uma bênção, a segunda uma
    /// maldição, depois 80 % ataque, 10 % bênção (`r == 8`) e 10 % maldição (`r == 9`, ou
    /// sempre, sem habilidade de ataque). O segundo valor é o `_skill_type`: 0 bênção,
    /// 1 maldição, 2 ataque.
    fn habilidade_principal(&mut self) -> Option<(HabilidadeDeCriatura, u8)> {
        let p = self.perfil.as_ref()?;
        let mut rng = rand::thread_rng();
        let uma = |v: &Vec<HabilidadeDeCriatura>, rng: &mut rand::rngs::ThreadRng| {
            v[rng.gen_range(0..v.len())]
        };
        if self.ordem_das_habilidades == 0 && !p.bencao.is_empty() {
            self.ordem_das_habilidades = 1;
            return Some((uma(&p.bencao, &mut rng), 0));
        }
        if self.ordem_das_habilidades <= 1 && !p.maldicao.is_empty() {
            self.ordem_das_habilidades = 2;
            return Some((uma(&p.maldicao, &mut rng), 1));
        }
        self.ordem_das_habilidades = 2;
        let mut r = rng.gen_range(0..=9);
        if r == 8 && !p.bencao.is_empty() {
            return Some((uma(&p.bencao, &mut rng), 0));
        }
        if p.ataque.is_empty() {
            r = 9;
        }
        if r == 9 && !p.maldicao.is_empty() {
            return Some((uma(&p.maldicao, &mut rng), 1));
        }
        (!p.ataque.is_empty()).then(|| (uma(&p.ataque, &mut rng), 2))
    }

    /// O andamento da `session_npc_skill`: o timer dispara depois do canto (o efeito,
    /// `RepeatSession` → `NPCEndSkill`) e de novo depois da execução (fim). A execução em
    /// tiques de 50 ms, e zero vira 20 tiques (`npcsession.cpp:716-721`).
    fn andamento_da_conjuracao(&mut self, delta_ms: u32) -> Option<Option<AcaoDoMonstro>> {
        let Combate::Conjurando {
            habilidade,
            alvo,
            falta_ms,
            aplicado,
        } = &mut self.combate
        else {
            return None;
        };
        *falta_ms = falta_ms.saturating_sub(delta_ms);
        if *falta_ms > 0 {
            return Some(None);
        }
        if !*aplicado {
            *aplicado = true;
            let execucao = habilidade.execucao_ms / 50 * 50;
            *falta_ms = if execucao == 0 { 1000 } else { execucao };
            return Some(Some(AcaoDoMonstro::UsouHabilidade {
                habilidade: *habilidade,
                alvo: *alvo,
            }));
        }
        self.combate = Combate::Nenhum;
        None
    }

    /// Abre a `session_npc_skill`. O canto em tiques de 50 ms; zero é instantânea: o efeito
    /// na hora e nada de sessão (`npcsession.cpp:708-713`).
    fn conjurar(&mut self, h: HabilidadeDeCriatura, alvo: i64) -> AcaoDoMonstro {
        let canto = h.canto_ms / 50 * 50;
        self.state = MonsterState::Attacking;
        self.seguir = None;
        self.combate = if canto > 0 {
            Combate::Conjurando {
                habilidade: h,
                alvo,
                falta_ms: canto,
                aplicado: false,
            }
        } else {
            Combate::Nenhum
        };
        AcaoDoMonstro::Conjurou {
            habilidade: h,
            alvo,
            instantanea: canto == 0,
        }
    }

    /// Antes de conjurar ou golpear, o monstro que andava para (o fim da sessão de seguir
    /// manda o `stop_move`).
    fn parar_se_andando(&mut self, monster: &MonsterEntity) -> Option<AcaoDoMonstro> {
        (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER))
    }

    fn combater(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: &AlvoDeCombate,
        mapa: &Mapa,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        // Correndo para longe (`keep_out`/`flee`): a sessão vai até o fim.
        if matches!(self.combate, Combate::Afastando { .. }) {
            if let Some(a) = self.afastar(monster, alvo, chao) {
                return Some(a);
            }
            if matches!(self.combate, Combate::Afastando { .. }) {
                return None;
            }
        }
        // O evento de vida entra na frente: a tarefa nova começa quando a sessão atual acaba
        // (`ai_target_task::OnHeartbeat`, `aipolicy.cpp:515-527`).
        if let Some(e) = self.evento_pendente {
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(a);
            }
            self.evento_pendente = None;
            match e {
                EventoDeVida::Fugir => {
                    let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
                    // `ai_runaway_task::StartTask`: `SetTarget(_target, FLEE_RANGE + body, 8)`,
                    // e o `keep_out` guarda 90 % do alcance (`npcsession.h:152-157`).
                    self.espera_ms = 0; // `StartSession` já dá o primeiro passo (`npcsession.cpp:325`)
                    self.combate = Combate::Afastando {
                        alcance: (ALCANCE_DA_FUGA + corpo) * 0.9,
                        restantes: 8,
                    };
                    return self.afastar(monster, alvo, chao);
                }
                EventoDeVida::Habilidade(h) => {
                    let quem = if crate::mascote::area_sem_alvo(h.area) {
                        monster.id
                    } else {
                        alvo.id
                    };
                    return Some(self.conjurar(h, quem));
                }
            }
        }
        let estrategia = self
            .estrategia_forcada
            .or(self.perfil.as_ref().map(|p| p.estrategia))
            .unwrap_or(Estrategia::CorpoACorpo);
        match estrategia {
            Estrategia::CorpoACorpo => self.corpo_a_corpo(monster, alvo, mapa, chao),
            Estrategia::Inerte => self.parar_se_andando(monster),
            Estrategia::Fugitivo => {
                let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
                self.espera_ms = 0; // `StartSession` já dá o primeiro passo (`npcsession.cpp:325`)
                self.combate = Combate::Afastando {
                    alcance: (ALCANCE_DA_FUGA + corpo) * 0.9,
                    restantes: 8,
                };
                self.afastar(monster, alvo, chao)
            }
            Estrategia::Fixo => self.fixo(monster, alvo),
            Estrategia::Distancia => self.a_distancia(monster, alvo, mapa, chao),
            Estrategia::Magia | Estrategia::CorpoACorpoEMagia | Estrategia::FixoMagico => {
                self.magico(monster, alvo, estrategia, mapa, chao)
            }
        }
    }

    /// Um golpe no alvo, com o intervalo do `attack_speed` do monstro.
    fn golpear(&mut self, monster: &MonsterEntity, alvo: &AlvoDeCombate) -> Option<AcaoDoMonstro> {
        if self.attack_cooldown_ms > 0 {
            return None;
        }
        // O intervalo entre golpes é o `attack_speed` do próprio monstro, em tiques de 50 ms
        // (`ChangeInterval(_cur_prop.attack_speed)`, `gs/npcsession.cpp:60-70`). Era 1,5 s
        // escrito aqui para todos (B62).
        self.attack_cooldown_ms = (monster.ataque_em_ticks.max(4) as u32) * 50;
        // Golpe que erra é resultado legítimo, e o `dano()` devolve zero nele.
        let mut fisico = 0;
        let dano = match (alvo.jogador, alvo.mascote) {
            (Some(p), _) => {
                let golpe = crate::combat::CombatEngine::golpe_de_monstro(monster);
                let r = crate::combat::resolver(
                    &golpe,
                    &crate::combat::CombatEngine::defesa_do_jogador(p),
                    alvo.distancia,
                    false,
                    crate::combat::Rolagens::sortear(),
                );
                // `AdjustDamage` só roda no golpe que acertou, e os espinhos saem com
                // `short_range > 0` — o monstro de longe, `attack_range > 6`
                // (`short_range_mode`, `npcgenerator.cpp:1972`; `monstros.rs`).
                if matches!(r, crate::combat::Resultado::Acertou { .. })
                    && monster.attack_range <= 6.0
                {
                    fisico = golpe.dano_fisico;
                }
                r.dano()
            }
            (None, Some(m)) => crate::combat::resolver(
                &crate::combat::CombatEngine::golpe_de_monstro(monster),
                &crate::combat::CombatEngine::defesa_do_monstro(m),
                alvo.distancia,
                false,
                crate::combat::Rolagens::sortear(),
            )
            .dano(),
            (None, None) => 0,
        };
        Some(AcaoDoMonstro::Atacou {
            alvo: alvo.id,
            dano,
            fisico,
        })
    }

    /// `ai_melee_task` — o corpo a corpo de sempre (B52–B99): no alcance bate, longe demais
    /// perde o alvo, senão persegue.
    fn corpo_a_corpo(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: &AlvoDeCombate,
        mapa: &Mapa,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        // `ai_melee_task::Execute` (`aipolicy.cpp:597-609`): começa a bater a `(attack_range −
        // corpo) × 0,8 + corpo + corpo do alvo` e persegue até `× 0,6`; a `session_npc_attack`
        // continua até `attack_range + corpo do alvo` (`CheckAttack`, `actobject.cpp:1280-1287`).
        // Tudo em 3D, e a meta do agente de chão no plano: antes o monstro parava a 0,9 ×
        // `attack_range` no plano, sem o corpo do alvo, e quem estava um pouco acima (voando, o
        // mascote de ar) ficava fora do alcance 3D — ele chegava, não batia e desistia (B139).
        let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
        let corpo_alvo = corpo_do_alvo(alvo);
        let puro = monster.attack_range - corpo;
        let limite = if self.golpeando_perto {
            monster.attack_range + corpo_alvo
        } else {
            puro * 0.8 + corpo + corpo_alvo
        };
        if alvo.distancia <= limite {
            self.golpeando_perto = true;
            // `range < _range_min`: a sessão de perseguição acaba (`npcsession.cpp:185-189`).
            self.seguir = None;
            self.state = MonsterState::Attacking;
            self.sessao = Sessao::Perseguindo;
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(a);
            }
            return self.golpear(monster, alvo);
        }
        self.golpeando_perto = false;
        let parar_a = (puro * 0.6 + corpo + corpo_alvo).max(0.5);
        self.aproximar(monster, alvo, parar_a, mapa, chao)
    }

    /// A perseguição (`session_npc_follow_target`) até `parar_a` do alvo; longe demais perde
    /// o alvo e, sem alvo nenhum, volta para casa.
    fn aproximar(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: &AlvoDeCombate,
        parar_a: f32,
        mapa: &Mapa,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        if alvo.distancia >= monster.aggro_range.max(Self::PERSEGUICAO_MINIMA) {
            // Longe demais: perde o alvo, e sem alvo nenhum volta para casa.
            self.aggro_table.remove(&alvo.id);
            return self.sem_alvo(monster, mapa);
        }
        // `MODE_INDEX_ROOT` (`filter_Fix`): não anda, mas bate se o alvo vier.
        if monster.efeitos.preso() {
            return self.parar_se_andando(monster);
        }
        self.state = MonsterState::Chasing;
        if !matches!(self.sessao, Sessao::Perseguindo) {
            self.passeio = None;
            self.volta = None;
            self.sessao = Sessao::Perseguindo;
            self.espera_ms = 0; // o original dá o primeiro passo ao abrir a sessão
        }
        if self.espera_ms > 0 {
            return None;
        }
        self.espera_ms = Self::PASSO_DE_PERSEGUICAO_MS;
        let passo = monster.corrida() * Self::PASSO_DE_PERSEGUICAO_MS as f32 / 1000.0;
        let acao = if monster.habitat == Habitat::Chao {
            self.perseguir(monster, alvo.id, alvo.posicao, passo, parar_a, mapa)
        } else {
            let _ = chao;
            self.perseguir_no_espaco(monster, alvo.id, alvo.posicao, passo, parar_a, mapa)
        };
        // `ai_target_task::OnSessionEnd` (`aipolicy.cpp:443-480`): a perseguição a um
        // **jogador** que acaba em `NSRC_ERR_PATHFINDING` — o agente desistiu, ou "chegou" três
        // vezes sem alcançar (`TEST_GETTOGOAL`, `npcsession.cpp:166-177`), como embaixo de quem
        // bate voando — faz o monstro esquecer tudo: `ClearAggro` + `ClearDamageList` e fim da
        // tarefa (o comentário do original: "igual ao WoW, todo mundo é esquivado"). Contra
        // mascote, a tarefa só recomeça (B134).
        if std::mem::take(&mut self.caminho_falhou) && alvo.jogador.is_some() {
            self.aggro_table.clear();
            monster.danos.clear();
            monster.primeiro_atacante = None;
            return self.sem_alvo(monster, mapa).or(acao);
        }
        acao
    }

    /// `session_npc_keep_out` / `session_npc_flee` (`npcsession.cpp:316-477`): um passo de
    /// `run_speed × NPC_FLEE_TIME` a cada 0,5 s para longe do alvo, até ficar a `alcance`.
    ///
    /// **Não é igual:** o original anda pelo agente `path_finding::keep_out`, que não está
    /// portado; aqui é a reta que se afasta do alvo, no chão do mapa. O `keep_out` do original
    /// não conta o `_timeout` (acaba ao chegar ou quando o agente chega à meta); sem o agente,
    /// o número de passos fica limitado pelo `timeout` que a tarefa passa (4 ou 5). A fuga
    /// conta os 8 dela (`session_npc_flee::RepeatSession`).
    fn afastar(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: &AlvoDeCombate,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        let Combate::Afastando { alcance, restantes } = &mut self.combate else {
            return None;
        };
        if alvo.distancia >= *alcance || *restantes <= 0 || monster.efeitos.preso() {
            self.combate = Combate::Nenhum;
            return self.parar_se_andando(monster);
        }
        if self.espera_ms > 0 {
            return None;
        }
        *restantes -= 1;
        self.espera_ms = PASSO_DE_FUGA_MS;
        self.seguir = None;
        self.passeio = None;
        self.volta = None;
        self.sessao = Sessao::Perseguindo;
        self.state = MonsterState::Chasing;
        let (dx, dz) = (
            monster.position.x - alvo.posicao.x,
            monster.position.z - alvo.posicao.z,
        );
        let d = (dx * dx + dz * dz).sqrt();
        let (ux, uz) = if d > 1e-3 {
            (dx / d, dz / d)
        } else {
            let a = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
            (a.cos(), a.sin())
        };
        let longe = Vector3::new(
            monster.position.x + ux * 100.0,
            monster.position.y,
            monster.position.z + uz * 100.0,
        );
        let passo = monster.corrida() * PASSO_DE_FUGA_MS as f32 / 1000.0;
        self.passo(
            monster,
            longe,
            passo,
            0.0,
            PASSO_DE_FUGA_MS,
            monster.corrida(),
            MODO_CORRER,
            chao,
        )
    }

    /// Continua a série de golpes em curso. `None` quando ela acabou (a tarefa decide de
    /// novo); `Some(ação ou nada)` enquanto dura. `session_npc_attack::RepeatSession`
    /// (`npcsession.cpp:44-77`) e `session_npc_range_attack::RepeatSession` (`:273-308`).
    fn continuar_serie(
        &mut self,
        monster: &MonsterEntity,
        alvo: &AlvoDeCombate,
    ) -> Option<Option<AcaoDoMonstro>> {
        let Combate::Serie(s) = self.combate else {
            return None;
        };
        let d2 = alvo.distancia * alvo.distancia;
        let acabou = alvo.distancia > s.alcance
            || d2 < s.curto
            || (s.interrompe && d2 < s.alcance * s.alcance * 0.36)
            || s.restantes == Some(0);
        if acabou {
            self.combate = Combate::Nenhum;
            return None;
        }
        let golpe = self.golpear(monster, alvo);
        if golpe.is_some() {
            if let Combate::Serie(Serie {
                restantes: Some(n), ..
            }) = &mut self.combate
            {
                *n -= 1;
            }
        }
        Some(golpe)
    }

    fn abrir_serie(
        &mut self,
        monster: &MonsterEntity,
        alvo: &AlvoDeCombate,
        serie: Serie,
    ) -> Option<AcaoDoMonstro> {
        self.seguir = None;
        self.state = MonsterState::Attacking;
        self.sessao = Sessao::Perseguindo;
        self.combate = Combate::Serie(serie);
        if let Some(a) = self.parar_se_andando(monster) {
            return Some(a);
        }
        self.continuar_serie(monster, alvo).flatten()
    }

    /// `ai_fix_melee_task::Execute` (`aipolicy.cpp:628-678`): não se move; no alcance
    /// (`(attack_range − corpo) × 0,8 + corpo + corpo do alvo`) bate, fora dele o alvo vai
    /// para o fim da lista (`FadeTarget`) e o monstro espera.
    fn fixo(&mut self, monster: &mut MonsterEntity, alvo: &AlvoDeCombate) -> Option<AcaoDoMonstro> {
        if let Some(a) = self.continuar_serie(monster, alvo) {
            return a;
        }
        let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
        let corpo_alvo = corpo_do_alvo(alvo);
        let alcance = (monster.attack_range - corpo) * 0.8 + corpo + corpo_alvo;
        if alvo.distancia < alcance {
            let serie = Serie {
                restantes: None,
                alcance: monster.attack_range + corpo_alvo,
                curto: 0.0,
                interrompe: false,
            };
            return self.abrir_serie(monster, alvo, serie);
        }
        self.parar_se_andando(monster)
    }

    /// `ai_range_task::Execute` (`aipolicy.cpp:744-812`).
    fn a_distancia(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: &AlvoDeCombate,
        mapa: &Mapa,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        if let Some(a) = self.continuar_serie(monster, alvo) {
            return a;
        }
        let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
        let corpo_alvo = corpo_do_alvo(alvo);
        let alcance = monster.attack_range + corpo_alvo;
        let sa = alcance * alcance;
        let d2 = alvo.distancia * alvo.distancia;
        if d2 > sa {
            return self.aproximar(monster, alvo, alcance * 0.8, mapa, chao);
        }
        let curto = (sa * 0.3 * 0.3).max(4.0);
        let colado = corpo_alvo + corpo + 0.3;
        if (d2 < colado * colado || (d2 < sa * 0.36 && self.vezes_de_afastar > 0))
            && self.estado_da_tarefa != DISTANCIA_AFASTOU
        {
            self.estado_da_tarefa = DISTANCIA_AFASTOU;
            self.vezes_de_afastar -= 1;
            // `SetTarget(_target, attack_range, 5)` → 90 % do alcance.
            self.espera_ms = 0; // `StartSession` já dá o primeiro passo (`npcsession.cpp:325`)
            self.combate = Combate::Afastando {
                alcance: alcance * 0.9,
                restantes: 5,
            };
            return self.afastar(monster, alvo, chao);
        }
        self.estado_da_tarefa = DISTANCIA_GOLPEANDO;
        let serie = Serie {
            restantes: None,
            alcance,
            curto: colado * colado,
            interrompe: d2 > curto,
        };
        self.abrir_serie(monster, alvo, serie)
    }

    /// `ai_magic_task` (2), `ai_magic_melee_task` (3) e `ai_fix_magic_task` (7)
    /// (`aipolicy.cpp:680-742`, `853-1107`).
    fn magico(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: &AlvoDeCombate,
        estrategia: Estrategia,
        mapa: &Mapa,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        if let Some(a) = self.continuar_serie(monster, alvo) {
            return a;
        }
        let corpo = self.perfil.as_ref().map_or(0.0, |p| p.corpo);
        let corpo_alvo = corpo_do_alvo(alvo);
        if self.habilidade_da_tarefa.is_none() {
            self.habilidade_da_tarefa = self.habilidade_principal();
        }
        let Some((h, tipo)) = self.habilidade_da_tarefa else {
            // Sem habilidade: o `StartTask` falha e nenhuma tarefa começa (`aipolicy.cpp:853-859`,
            // `970-976`); a fixa tira o alvo da lista (`:716-722`). O original avisa quem monta
            // um monstro assim (`npcgenerator.cpp:288-291`).
            if estrategia == Estrategia::FixoMagico {
                self.aggro_table.remove(&alvo.id);
            }
            return self.parar_se_andando(monster);
        };
        let alvo_da_habilidade = |h: &HabilidadeDeCriatura| {
            if crate::mascote::area_sem_alvo(h.area) {
                monster.id
            } else {
                alvo.id
            }
        };
        let d2 = alvo.distancia * alvo.distancia;

        if estrategia == Estrategia::FixoMagico {
            // `GetMagicRange + body_size`, sem o corpo do alvo.
            let alcance = h.alcance + corpo;
            if d2 < alcance * alcance {
                if let Some(a) = self.parar_se_andando(monster) {
                    return Some(a);
                }
                self.habilidade_da_tarefa = None;
                let quem = alvo_da_habilidade(&h);
                return Some(self.conjurar(h, quem));
            }
            return self.parar_se_andando(monster);
        }

        // Bênção (`_skill_type` 0): conjura já, onde estiver.
        if tipo == 0 {
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(a);
            }
            self.estado_da_tarefa = ESTADO_MAGIA;
            self.habilidade_da_tarefa = self.habilidade_principal();
            let quem = alvo_da_habilidade(&h);
            return Some(self.conjurar(h, quem));
        }
        let alcance_magico = h.alcance + corpo + corpo_alvo;
        let sa = alcance_magico * alcance_magico;
        if d2 > sa * 0.81 {
            self.estado_da_tarefa = ESTADO_PERSEGUIR;
            return self.aproximar(monster, alvo, alcance_magico * 0.9, mapa, chao);
        }
        if estrategia == Estrategia::Magia {
            // `KeepMagicCastRange` é sempre verdade (`aipolicy.h:1133-1136`).
            if d2 < sa * 0.25
                && self.vezes_de_afastar > 0
                && self.estado_da_tarefa != ESTADO_AFASTAR
            {
                self.estado_da_tarefa = ESTADO_AFASTAR;
                self.vezes_de_afastar -= 1;
                self.espera_ms = 0; // `StartSession` já dá o primeiro passo (`npcsession.cpp:325`)
                self.combate = Combate::Afastando {
                    alcance: alcance_magico * 0.9,
                    restantes: 4,
                };
                return self.afastar(monster, alvo, chao);
            }
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(a);
            }
            self.estado_da_tarefa = ESTADO_MAGIA;
            self.habilidade_da_tarefa = self.habilidade_principal();
            let quem = alvo_da_habilidade(&h);
            return Some(self.conjurar(h, quem));
        }

        // Corpo a corpo e magia.
        let puro = monster.attack_range - corpo;
        let alcance = puro * 0.8 + corpo + corpo_alvo;
        let curto = corpo + corpo_alvo;
        if d2 > alcance * alcance || self.estado_da_tarefa == ESTADO_GOLPES {
            if let Some(a) = self.parar_se_andando(monster) {
                return Some(a);
            }
            self.estado_da_tarefa = ESTADO_MAGIA;
            self.habilidade_da_tarefa = self.habilidade_principal();
            let quem = alvo_da_habilidade(&h);
            return Some(self.conjurar(h, quem));
        }
        if d2 < curto * curto && self.estado_da_tarefa != ESTADO_AFASTAR {
            self.estado_da_tarefa = ESTADO_AFASTAR;
            self.espera_ms = 0; // `StartSession` já dá o primeiro passo (`npcsession.cpp:325`)
            self.combate = Combate::Afastando {
                alcance: alcance * 0.9,
                restantes: 4,
            };
            return self.afastar(monster, alvo, chao);
        }
        self.estado_da_tarefa = ESTADO_GOLPES;
        // `SetAttackTimes((195 + Rand(10, 20)) / (attack_speed + 1))`; zero é sem limite
        // (`if(_attack_times)`, `npcsession.cpp:46`).
        let vezes =
            (195 + rand::thread_rng().gen_range(10..=20)) / (monster.ataque_em_ticks.max(0) + 1);
        let serie = Serie {
            restantes: (vezes > 0).then_some(vezes),
            alcance: monster.attack_range + corpo_alvo,
            curto,
            interrompe: false,
        };
        self.abrir_serie(monster, alvo, serie)
    }

    /// `gnpc_imp::OnHeartbeat` + `ai_policy::HaveRest`.
    fn batimento(&mut self, monster: &MonsterEntity, players: &HashMap<i64, PlayerEntity>) {
        let alguem_perto = players
            .values()
            .any(|p| p.position.distance(&monster.position) <= Self::RAIO_DE_ATIVIDADE);
        if alguem_perto {
            self.idle_timer = Self::BATIMENTOS_OCIOSO;
        } else if self.idle_timer > 0 {
            self.idle_timer -= 1;
        }

        // `group_boss_policy::OnHeartbeat` → `TryForwardAggro` (`aipolicy.h:1470-1497`): o
        // primeiro da lista, quando muda, vai aos subordinados.
        if self.chefe {
            match self.get_highest_threat_target() {
                Some(t) if self.inimigo_repassado != Some(t) => {
                    self.inimigo_repassado = Some(t);
                    self.odio_a_repassar = Some((t, self.aggro_table.get(&t).copied().unwrap_or(1)));
                }
                Some(_) => {}
                None => self.inimigo_repassado = None,
            }
        }

        let livre = matches!(self.sessao, Sessao::Nenhuma) && self.aggro_table.is_empty();
        // `ai_policy::OnHeartbeat` sem tarefa e fora de combate (`aipolicy.cpp:320-340`): com
        // rota, o próximo ponto vira um `ai_patrol_task`; senão o `HaveRest` — no subordinado com
        // líder, o `ai_follow_master`. Parado sem ninguém por perto (`_idle_mode`), nada.
        if livre && self.idle_timer > 0 {
            if let Some(p) = self.rota.as_mut().and_then(|r| r.proximo()) {
                self.sessao = Sessao::Patrulhando { alvo: p, restantes: Self::PASSOS_DA_PATRULHA, inicio: monster.position };
                self.patrulha = None;
                self.espera_ms = 0;
                self.state = MonsterState::Patrol;
                return;
            }
            if self.rota.is_some() {
                return;
            }
            if let Some(l) = self.lider_em {
                self.seguir_o_lider(monster, l);
                return;
            }
        }
        if livre && monster.patrulha && self.idle_timer > 0 {
            self.cruise_timer = self.cruise_timer.wrapping_sub(1) & 31;
            if self.cruise_timer == 0 {
                // O passeio do `ai_rest_task`, em volta de casa (o do líder já não vale).
                self.centro_do_passeio = None;
                self.comecar_passeio(monster);
            }
        }
    }

    /// `ai_follow_master::OnHeartbeat` (`aipolicy.cpp:1519-1583`), a distância **horizontal**
    /// ao líder: de 20 m para cima (`MAX_MASTER_MINOR_RANGE` 400, ao quadrado, `config.h:105`)
    /// vai de uma vez para até 7 m dele (`ReturnHome(info.pos, 7)`); de 8 m para cima corre
    /// atrás dele; perto, passeia em volta dele.
    fn seguir_o_lider(&mut self, monster: &MonsterEntity, lider: Vector3) {
        let d2 = (lider.x - monster.position.x).powi(2) + (lider.z - monster.position.z).powi(2);
        self.centro_do_passeio = None;
        if d2 >= Self::LONGE_DO_LIDER_AO_QUADRADO {
            let mut rng = rand::thread_rng();
            let r = Self::PERTO_DO_LIDER;
            self.teleporte = Some(Vector3::new(
                lider.x + rng.gen_range(-r..=r),
                lider.y,
                lider.z + rng.gen_range(-r..=r),
            ));
        } else if d2 >= 64.0 {
            self.sessao = Sessao::SeguindoLider;
            self.patrulha = None;
            self.espera_ms = 0;
            self.state = MonsterState::Patrol;
        } else {
            self.centro_do_passeio = Some((lider, Self::PERTO_DO_LIDER, 6));
            self.passeio = None;
            self.comecar_passeio(monster);
        }
    }

    fn comecar_passeio(&mut self, monster: &MonsterEntity) {
        let mut rng = rand::thread_rng();
        let (centro, r, passos) =
            self.centro_do_passeio.unwrap_or((monster.spawn_center, Self::RAIO_DO_PASSEIO, Self::PASSOS_DO_PASSEIO));
        let destino = Vector3::new(
            centro.x + rng.gen_range(-r..=r),
            centro.y,
            centro.z + rng.gen_range(-r..=r),
        );
        self.sessao = Sessao::Passeando {
            destino,
            passos_restantes: passos,
        };
        // A espera **não** é zerada (B104): na emenda (`ai_rest_task::OnSessionEnd`, 10 %) o
        // último passo do passeio anterior acabou de sair com `use_time` de 1 s, e o primeiro
        // do novo saía no tique seguinte — dois `OBJECT_MOVE` a ~50 ms, e o cliente corria ou
        // pulava o monstro para alcançar o segundo destino. No começo normal ela já é zero.
        self.state = MonsterState::Patrol;
    }

    /// O que fazer sem alvo: terminar a volta para casa ou o passeio em curso.
    fn sem_alvo(&mut self, monster: &mut MonsterEntity, mapa: &Mapa) -> Option<AcaoDoMonstro> {
        if self.aggro_table.is_empty() {
            let estava_em_combate = self.em_combate;
            self.sair_de_combate(monster);
            if estava_em_combate {
                // `RollBack`: a lista esvaziou.
                self.voltar_para_casa(monster);
            }
        }
        let piso = |x: f32, z: f32| {
            (mapa.terreno)(x, z).map(|h| h + mapa.movimento.acima_do_terreno(x, z).unwrap_or(0.0))
        };
        let chao: Chao = &piso;
        self.seguir = None;
        self.perseguindo = None;
        if let Some(p) = self.teleporte.take() {
            // `ai_npcobject::ReturnHome` (`ainpc.cpp:98-107`): `stop_move(pos, 0x500, 1,
            // MOVE_MODE_RETURN)` e o passo de uma vez.
            monster.position = p;
            self.parado = true;
            return Some(AcaoDoMonstro::Parou {
                posicao: p,
                velocidade: 0x500 as f32 / 256.0,
                direcao: self.direcao,
                modo: MODO_VOLTAR,
            });
        }
        match self.sessao {
            Sessao::Patrulhando { .. } => self.passo_de_patrulha(monster, mapa, chao),
            Sessao::SeguindoLider => self.passo_atras_do_lider(monster, mapa, chao),
            Sessao::Perseguindo => {
                // Perdeu o alvo sem a lista esvaziar do lado de cá (alvo que sumiu):
                // o mesmo `RollBack`.
                self.voltar_para_casa(monster);
                None
            }
            Sessao::Voltando => {
                if self.espera_ms > 0 {
                    return None;
                }
                self.espera_ms = Self::PASSO_DE_PATRULHA_MS;
                let passo = monster.corrida() * Self::PASSO_DE_PATRULHA_MS as f32 / 1000.0;
                if monster.habitat == Habitat::Chao {
                    return self.voltar(monster, passo, mapa);
                }
                let casa = self.casa(monster);
                let acao = self.passo(
                    monster,
                    casa,
                    passo,
                    0.0,
                    Self::PASSO_DE_PATRULHA_MS,
                    monster.corrida(),
                    MODO_CORRER,
                    chao,
                );
                if matches!(acao, Some(AcaoDoMonstro::Parou { .. }) | None) {
                    self.fim_da_volta(monster);
                }
                acao
            }
            Sessao::Passeando {
                destino,
                passos_restantes,
            } => {
                if self.espera_ms > 0 {
                    return None;
                }
                self.espera_ms = Self::PASSO_DE_PATRULHA_MS;
                if passos_restantes <= 0 {
                    self.passeio = None;
                    self.sessao = Sessao::Nenhuma;
                    self.state = MonsterState::Idle;
                    return (!self.parado)
                        .then(|| self.parar(monster, monster.andar(), MODO_ANDAR));
                }
                self.sessao = Sessao::Passeando {
                    destino,
                    passos_restantes: passos_restantes - 1,
                };
                let passo = monster.andar() * Self::PASSO_DE_PATRULHA_MS as f32 / 1000.0;
                if monster.habitat == Habitat::Chao {
                    return self.passear(monster, passo, mapa);
                }
                let mut acao = self.passo(
                    monster,
                    destino,
                    passo,
                    0.0,
                    Self::PASSO_DE_PATRULHA_MS,
                    monster.andar(),
                    MODO_ANDAR,
                    chao,
                );
                // Chegou neste passo: ele vai como parada, como o de chão acima (B106).
                let (dx, dz) = (
                    destino.x - monster.position.x,
                    destino.z - monster.position.z,
                );
                if matches!(acao, Some(AcaoDoMonstro::Andou { .. }))
                    && dx * dx + dz * dz <= 0.05 * 0.05
                {
                    acao = Some(self.parar(monster, monster.andar(), MODO_ANDAR));
                }
                if matches!(acao, Some(AcaoDoMonstro::Parou { .. }) | None) {
                    self.sessao = Sessao::Nenhuma;
                    self.state = MonsterState::Idle;
                    if rand::thread_rng().gen_bool(Self::CHANCE_DE_EMENDAR_PASSEIO) {
                        self.comecar_passeio(monster);
                    }
                }
                acao
            }
            Sessao::Nenhuma => {
                self.state = MonsterState::Idle;
                None
            }
        }
    }

    /// Um passo de até `passo` metros em direção a `alvo`, parando a `parar_a` metros dele.
    /// Devolve o aviso de movimento, ou o de parada quando chegou.
    #[allow(clippy::too_many_arguments)]
    fn passo(
        &mut self,
        monster: &mut MonsterEntity,
        alvo: Vector3,
        passo: f32,
        parar_a: f32,
        tempo_ms: u32,
        velocidade: f32,
        modo: u8,
        chao: Chao,
    ) -> Option<AcaoDoMonstro> {
        let dx = alvo.x - monster.position.x;
        let dz = alvo.z - monster.position.z;
        let distancia = (dx * dx + dz * dz).sqrt();
        let falta = distancia - parar_a;
        if falta <= 0.05 {
            return (!self.parado).then(|| self.parar(monster, velocidade, modo));
        }
        let andar = passo.min(falta);
        let (ux, uz) = (dx / distancia, dz / distancia);
        let x = monster.position.x + ux * andar;
        let z = monster.position.z + uz * andar;
        let y_pretendido = monster.position.y + (alvo.y - monster.position.y) * (andar / distancia);
        let y = Self::altura(monster.habitat, x, z, y_pretendido, chao);

        monster.position = Vector3::new(x, y, z);
        self.direcao = direcao_do_vetor(ux, uz);
        self.parado = false;
        Some(AcaoDoMonstro::Andou {
            destino: monster.position,
            tempo_ms: tempo_ms as u16,
            velocidade,
            modo: modo | monster.habitat.mascara_de_movimento(),
        })
    }

    /// Anda até `p` (o que o agente devolveu), ou para se não saiu do lugar.
    fn ir_para(
        &mut self,
        monster: &mut MonsterEntity,
        p: V3,
        tempo_ms: u32,
        velocidade: f32,
        modo: u8,
    ) -> Option<AcaoDoMonstro> {
        let (dx, dy, dz) = (
            p.x - monster.position.x,
            p.y - monster.position.y,
            p.z - monster.position.z,
        );
        // `offset.squared_magnitude() < 1e-3` → `TrySendStop`.
        if dx * dx + dy * dy + dz * dz < 1e-3 {
            return (!self.parado).then(|| self.parar(monster, velocidade, modo));
        }
        monster.position = Vector3::new(p.x, p.y, p.z);
        let m = (dx * dx + dz * dz).sqrt();
        if m > 0.0 {
            self.direcao = direcao_do_vetor(dx / m, dz / m);
        }
        self.parado = false;
        Some(AcaoDoMonstro::Andou {
            destino: monster.position,
            tempo_ms: tempo_ms as u16,
            velocidade,
            modo: modo | monster.habitat.mascara_de_movimento(),
        })
    }

    /// `session_npc_follow_target::Run` (`gs/npcsession.cpp:164-273`) para o monstro de chão:
    /// o `follow_target` recomeça quando chega (com 60% do alcance) ou quando o alvo se afasta
    /// mais de 7 m da meta antiga (4 m, se não estiver bloqueado); três chegadas sem encostar
    /// (`_reachable_count`) ou o agente desistindo encerram a sessão, e a próxima começa do
    /// zero no passo seguinte — no original é a tarefa de IA que abre outra.
    /// A perseguição do monstro de ar ou de água: `follow_target` com o `CNPCChaseOnAirPFAgent`
    /// / `CNPCChaseInWaterPFAgent` (`NPCMoveAgent.cpp:68-90`), sem a dispersão (que é só do
    /// agente de chão) — reta quando o `airmap/` deixa, senão a busca na octree (B133).
    fn perseguir_no_espaco(
        &mut self,
        monster: &mut MonsterEntity,
        alvo_id: i64,
        alvo: Vector3,
        passo: f32,
        alcance: f32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        use crate::navegacao::Ambiente;
        let amb = if monster.habitat == Habitat::Agua { Ambiente::Agua } else { Ambiente::Ar };
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let meta = V3::new(alvo.x, alvo.y, alvo.z);
        let d2 = monster.position.distance(&alvo).powi(2);
        if self.perseguindo != Some(alvo_id) {
            self.perseguindo = Some(alvo_id);
            self.no_espaco = None;
            self.chegadas = 0;
        }
        let primeira = self.no_espaco.is_none();
        let mut s = self.no_espaco.take().unwrap_or_default();
        let mut recomecou = false;
        if primeira {
            s.comecar(amb, de, meta, passo, alcance, d2, mapa);
            recomecou = true;
        } else if s.chegou() {
            s.comecar(amb, de, meta, passo, alcance * 0.6, d2, mapa);
            recomecou = true;
        } else {
            let a = s.alvo();
            let dis = (a.x - meta.x).powi(2) + (a.z - meta.z).powi(2);
            if dis > 49.0 || (dis > 16.0 && !s.bloqueado()) {
                s.comecar(amb, de, meta, passo, alcance, d2, mapa);
                recomecou = true;
            }
        }
        if recomecou && s.chegou() {
            self.chegadas += 1;
            if self.chegadas >= 3 {
                self.chegadas = 0;
                self.caminho_falhou = true;
            } else {
                self.no_espaco = Some(s);
            }
            return (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER));
        }
        if !s.andar(passo, mapa) {
            if primeira {
                self.no_espaco = Some(s);
                return None;
            }
            self.chegadas = 0;
            self.caminho_falhou = true;
            return (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER));
        }
        let p = s.posicao();
        self.no_espaco = Some(s);
        self.ir_para(monster, p, Self::PASSO_DE_PERSEGUICAO_MS, monster.corrida(), MODO_CORRER)
    }

    fn perseguir(
        &mut self,
        monster: &mut MonsterEntity,
        alvo_id: i64,
        alvo: Vector3,
        passo: f32,
        alcance: f32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let meta = V3::new(alvo.x, alvo.y, alvo.z);
        // O `range` do `Start` é a distância **ao quadrado** (`squared_distance`).
        let d2 = monster.position.distance(&alvo).powi(2);
        if self.perseguindo != Some(alvo_id) {
            // Outro alvo: a direção de dispersão é por perseguição.
            self.perseguindo = Some(alvo_id);
            self.info = InfoDePerseguicao::default();
            self.seguir = None;
            self.chegadas = 0;
        }
        let primeira = self.seguir.is_none();
        let mut s = self.seguir.take().unwrap_or_default();
        let mut recomecou = false;
        if primeira {
            s.comecar(de, meta, passo, alcance, d2, Some(&mut self.info), mapa);
            recomecou = true;
        } else if s.chegou() {
            s.comecar(
                de,
                meta,
                passo,
                alcance * 0.6,
                d2,
                Some(&mut self.info),
                mapa,
            );
            recomecou = true;
        } else {
            let a = s.alvo();
            let dis = (a.x - meta.x).powi(2) + (a.z - meta.z).powi(2);
            if dis > 49.0 || (dis > 16.0 && !s.bloqueado()) {
                s.comecar(de, meta, passo, alcance, d2, Some(&mut self.info), mapa);
                recomecou = true;
            }
        }
        // `TEST_GETTOGOAL`.
        if recomecou && s.chegou() {
            self.chegadas += 1;
            if self.chegadas >= 3 {
                // `NSRC_ERR_PATHFINDING`: a sessão acaba.
                self.chegadas = 0;
                self.caminho_falhou = true;
            } else {
                self.seguir = Some(s);
            }
            return (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER));
        }
        if !s.andar(passo, mapa) {
            if primeira {
                self.seguir = Some(s);
                return None;
            }
            // `NSRC_ERR_PATHFINDING`: a sessão acaba.
            self.chegadas = 0;
            self.caminho_falhou = true;
            return (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER));
        }
        let p = s.posicao();
        self.seguir = Some(s);
        self.ir_para(
            monster,
            p,
            Self::PASSO_DE_PERSEGUICAO_MS,
            monster.corrida(),
            MODO_CORRER,
        )
    }

    /// `ai_returnhome_task` → `session_npc_patrol::Run` (`gs/aipolicy.cpp:1291-1322`,
    /// `gs/npcsession.cpp:883-960`): `follow_target` até casa com alcance de 0,8 m; acaba a
    /// 1,2 passo de casa, ao chegar ou quando o agente desiste. Se ao fim ainda estiver a mais
    /// de 10 m (`GetReturnHomeRange` = 10², `aipolicy.h:1393`), o `ReturnHome` o põe em casa
    /// de uma vez (`gs/ainpc.cpp:98-106`: `stop_move` com `MOVE_MODE_RETURN` e 0x500).
    fn voltar(
        &mut self,
        monster: &mut MonsterEntity,
        passo: f32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        let casa = self.casa(monster);
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let meta = V3::new(casa.x, casa.y, casa.z);
        let mut s = match self.volta.take() {
            Some(s) => s,
            None => {
                let mut s = SeguirAlvo::default();
                s.comecar(de, meta, passo, 0.8, 15.0, None, mapa);
                s
            }
        };
        let perto = monster.position.distance(&casa).powi(2) <= 1.44 * passo * passo;
        let acabou = perto || s.chegou() || !s.andar(passo, mapa);
        if !acabou {
            let p = s.posicao();
            self.volta = Some(s);
            return self.ir_para(
                monster,
                p,
                Self::PASSO_DE_PATRULHA_MS,
                monster.corrida(),
                MODO_CORRER,
            );
        }
        self.fim_da_volta(monster);
        if monster.position.distance(&casa).powi(2) > 100.0 {
            monster.position = casa;
            self.parado = true;
            return Some(AcaoDoMonstro::Parou {
                posicao: casa,
                velocidade: 0x500 as f32 / 256.0,
                direcao: self.direcao,
                modo: MODO_VOLTAR,
            });
        }
        (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER))
    }

    /// `session_npc_patrol::Run` (`npcsession.cpp:883-994`) da patrulha: um passo por segundo
    /// (`NPC_PATROL_TIME`), andando, ou correndo com o `iSpeedFlag`. Perto do ponto
    /// (`1,44 × passo²`) ou com o agente na meta, pega o próximo da rota (meta de 1,8 m ou
    /// 0,8 m); sem próximo, ou com o agente sem caminho, a sessão acaba. O de ar e o de água
    /// vão em reta.
    fn passo_de_patrulha(&mut self, monster: &mut MonsterEntity, mapa: &Mapa, chao: Chao) -> Option<AcaoDoMonstro> {
        let Sessao::Patrulhando { mut alvo, restantes, inicio } = self.sessao else {
            return None;
        };
        if self.espera_ms > 0 {
            return None;
        }
        self.espera_ms = Self::PASSO_DE_PATRULHA_MS;
        let corre = self.rota.as_ref().is_some_and(|r| r.corre);
        let (velocidade, modo) = if corre { (monster.corrida(), MODO_CORRER) } else { (monster.andar(), MODO_ANDAR) };
        if restantes <= 0 {
            return self.fim_da_patrulha(monster, alvo, inicio, velocidade, modo);
        }
        let passo = velocidade * Self::PASSO_DE_PATRULHA_MS as f32 / 1000.0;
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let perto = monster.position.distance(&alvo).powi(2) <= 1.44 * passo * passo;
        if monster.habitat != Habitat::Chao {
            if perto {
                match self.rota.as_mut().and_then(|r| r.proximo()) {
                    Some(n) => alvo = n,
                    None => return self.fim_da_patrulha(monster, alvo, inicio, velocidade, modo),
                }
            }
            self.sessao = Sessao::Patrulhando { alvo, restantes: restantes - 1, inicio };
            return self.passo(monster, alvo, passo, 0.0, Self::PASSO_DE_PATRULHA_MS, velocidade, modo, chao);
        }
        let mut s = self.patrulha.take().unwrap_or_else(|| {
            let mut s = SeguirAlvo::default();
            s.comecar(de, V3::new(alvo.x, alvo.y, alvo.z), passo, 0.8, 15.0, None, mapa);
            s
        });
        let reinicio = if perto { Some(1.8) } else if s.chegou() { Some(0.8) } else { None };
        if let Some(meta) = reinicio {
            match self.rota.as_mut().and_then(|r| r.proximo()) {
                Some(n) => {
                    alvo = n;
                    s.comecar(de, V3::new(n.x, n.y, n.z), passo, meta, 15.0, None, mapa);
                }
                None => return self.fim_da_patrulha(monster, alvo, inicio, velocidade, modo),
            }
        }
        if !s.andar(passo, mapa) {
            return self.fim_da_patrulha(monster, alvo, inicio, velocidade, modo);
        }
        let p = s.posicao();
        self.patrulha = Some(s);
        self.sessao = Sessao::Patrulhando { alvo, restantes: restantes - 1, inicio };
        self.ir_para(monster, p, Self::PASSO_DE_PATRULHA_MS, velocidade, modo)
    }

    /// `session_npc_patrol::EndSession` (a parada, se ainda andava) e `ai_patrol_task::EndTask`
    /// (`aipolicy.cpp:1661-1673`): quem não saiu do lugar desde o começo da tarefa vai de uma
    /// vez ao ponto (`ReturnHome(_target, 0)`).
    fn fim_da_patrulha(&mut self, monster: &mut MonsterEntity, alvo: Vector3, inicio: Vector3, velocidade: f32, modo: u8) -> Option<AcaoDoMonstro> {
        self.sessao = Sessao::Nenhuma;
        self.patrulha = None;
        self.state = MonsterState::Idle;
        if monster.position.distance(&inicio).powi(2) < 1e-3 && monster.position.distance(&alvo) > 1e-3 {
            monster.position = alvo;
            self.parado = true;
            return Some(AcaoDoMonstro::Parou {
                posicao: alvo,
                velocidade: 0x500 as f32 / 256.0,
                direcao: self.direcao,
                modo: MODO_VOLTAR,
            });
        }
        (!self.parado).then(|| self.parar(monster, velocidade, modo))
    }

    /// `session_npc_follow_target` do `ai_follow_master`: correndo, a cada 500 ms, até ficar a
    /// menos de 7 m do líder (`_range_min`), desistindo além de 20 m (`_range_max`) ou sem
    /// caminho. As regras de recomeço do agente são as do `follow_target` (`npcsession.cpp:
    /// 199-233`).
    fn passo_atras_do_lider(&mut self, monster: &mut MonsterEntity, mapa: &Mapa, chao: Chao) -> Option<AcaoDoMonstro> {
        let fim = |ia: &mut Self, monster: &MonsterEntity| {
            ia.sessao = Sessao::Nenhuma;
            ia.patrulha = None;
            ia.state = MonsterState::Idle;
            (!ia.parado).then(|| ia.parar(monster, monster.corrida(), MODO_CORRER))
        };
        let Some(lider) = self.lider_em else {
            return fim(self, monster);
        };
        if self.espera_ms > 0 {
            return None;
        }
        self.espera_ms = Self::PASSO_DE_PERSEGUICAO_MS;
        let d = monster.position.distance(&lider);
        if !(Self::PERTO_DO_LIDER..=20.0).contains(&d) {
            return fim(self, monster);
        }
        let passo = monster.corrida() * Self::PASSO_DE_PERSEGUICAO_MS as f32 / 1000.0;
        if monster.habitat != Habitat::Chao {
            return self.passo(monster, lider, passo, Self::PERTO_DO_LIDER, Self::PASSO_DE_PERSEGUICAO_MS, monster.corrida(), MODO_CORRER, chao);
        }
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let meta = V3::new(lider.x, lider.y, lider.z);
        let (recomecar, alcance) = match &self.patrulha {
            None => (true, Self::PERTO_DO_LIDER),
            Some(s) if s.chegou() => (true, Self::PERTO_DO_LIDER * 0.6),
            Some(s) => {
                let a = s.alvo();
                let dis = (a.x - meta.x).powi(2) + (a.z - meta.z).powi(2);
                (dis > 49.0 || (dis > 16.0 && !s.bloqueado()), Self::PERTO_DO_LIDER)
            }
        };
        let mut s = self.patrulha.take().unwrap_or_default();
        if recomecar {
            s.comecar(de, meta, passo, alcance, d * d, None, mapa);
        }
        if !s.andar(passo, mapa) {
            return fim(self, monster);
        }
        let p = s.posicao();
        self.patrulha = Some(s);
        self.ir_para(monster, p, Self::PASSO_DE_PERSEGUICAO_MS, monster.corrida(), MODO_CORRER)
    }

    /// `session_npc_cruise::Run` (`gs/npcsession.cpp:590-650`) com o `cruise`: a meta é
    /// sorteada no disco de 10 m em volta de onde nasceu, alcançável e de preferência em linha
    /// reta, e o caminho até ela desvia de obstáculo.
    fn passear(
        &mut self,
        monster: &mut MonsterEntity,
        passo: f32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMonstro> {
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let (casa, raio, _) =
            self.centro_do_passeio.unwrap_or((monster.spawn_center, Self::RAIO_DO_PASSEIO, 0));
        let mut p = match self.passeio.take() {
            Some(p) => p,
            None => {
                let mut p = Passeio::default();
                p.comecar(
                    de,
                    V3::new(casa.x, casa.y, casa.z),
                    passo,
                    raio,
                    mapa,
                );
                p
            }
        };
        if p.parou() {
            return self.fim_do_passeio(monster);
        }
        p.andar(passo, mapa);
        let alvo = p.posicao();
        if p.parou() {
            // O último passo vai **só** como `stop_move` até o ponto final (`npcsession.cpp:
            // 626-633`: `GetToGoal` depois do `StepMove` → `stop_move(targetpos)`, sem `move`).
            // Antes ia como `OBJECT_MOVE` e a parada era descartada: o cliente segue andando
            // na mesma direção enquanto não chega comando novo (`CECNPC::MovingTo`, "just move
            // on", `EC_NPC.cpp:1225-1240`) e só puxa o monstro de volta quando passa de 25 m
            // do destino (`MAX_LAGDIST`, `EC_NPC.cpp:79`) — o monstro "disparando" e
            // voltando de uma vez que o Murillo via (B106).
            let (dx, dz) = (alvo.x - monster.position.x, alvo.z - monster.position.z);
            let m = (dx * dx + dz * dz).sqrt();
            if m > 0.0 {
                self.direcao = direcao_do_vetor(dx / m, dz / m);
            }
            monster.position = Vector3::new(alvo.x, alvo.y, alvo.z);
            self.parado = false;
            return self.fim_do_passeio(monster);
        }
        let acao = self.ir_para(
            monster,
            alvo,
            Self::PASSO_DE_PATRULHA_MS,
            monster.andar(),
            MODO_ANDAR,
        );
        self.passeio = Some(p);
        acao
    }

    fn fim_do_passeio(&mut self, monster: &MonsterEntity) -> Option<AcaoDoMonstro> {
        self.sessao = Sessao::Nenhuma;
        self.state = MonsterState::Idle;
        let parada = (!self.parado).then(|| self.parar(monster, monster.andar(), MODO_ANDAR));
        if rand::thread_rng().gen_bool(Self::CHANCE_DE_EMENDAR_PASSEIO) {
            self.comecar_passeio(monster);
        }
        parada
    }

    fn parar(&mut self, monster: &MonsterEntity, velocidade: f32, modo: u8) -> AcaoDoMonstro {
        self.parado = true;
        AcaoDoMonstro::Parou {
            posicao: monster.position,
            velocidade,
            direcao: self.direcao,
            modo: modo | monster.habitat.mascara_de_movimento(),
        }
    }

    /// Onde o passo assenta. Monstro de chão fica **no** chão; o de água e o de ar seguem
    /// a altura pretendida, sem descer abaixo do terreno.
    pub fn altura(habitat: Habitat, x: f32, z: f32, y_pretendido: f32, chao: Chao) -> f32 {
        match (habitat, chao(x, z)) {
            (Habitat::Chao, Some(h)) => h,
            (_, Some(h)) => y_pretendido.max(h),
            (_, None) => y_pretendido,
        }
    }
}

/// `a3dvector_to_dir` (`common/types.h:99`): o ângulo no plano XZ em 256 passos.
pub fn direcao_do_vetor(x: f32, z: f32) -> u8 {
    ((z.atan2(x) as f64 * (128.0 / std::f64::consts::PI)) as i32 as u32 & 0xFF) as u8
}
