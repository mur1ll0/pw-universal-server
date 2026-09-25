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
    /// Bateu em alguém.
    Atacou { alvo: i64, dano: i32 },
    /// Deu um passo até `destino`, que o cliente percorre em `tempo_ms`.
    Andou { destino: Vector3, tempo_ms: u16, velocidade: f32, modo: u8 },
    /// Parou em `posicao` (`OBJECT_STOP_MOVE`).
    Parou { posicao: Vector3, velocidade: f32, direcao: u8, modo: u8 },
}

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
    Passeando { destino: Vector3, passos_restantes: i32 },
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
    /// O `cruise` do passeio.
    passeio: Option<Passeio>,
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
    /// Até onde um jogador conta como "por perto" para o monstro não ficar ocioso. É o raio
    /// de visão do `bus_server`: quem está vendo o monstro.
    pub const RAIO_DE_ATIVIDADE: f32 = 120.0;
    /// Piso da distância de perseguição, para monstro cujo `aggro_range` é pequeno demais
    /// para ele sair do lugar.
    pub const PERSEGUICAO_MINIMA: f32 = 15.0;

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
            passeio: None,
        }
    }

    /// Adiciona ameaça a um jogador
    pub fn add_threat(&mut self, player_id: i64, threat: i64) {
        let entry = self.aggro_table.entry(player_id).or_insert(0);
        *entry += threat;
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
        self.tick_no_mapa(monster, players, delta_ms, &Mapa { terreno: chao, movimento: &vazio })
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
        // O piso para o monstro de água e de ar: terreno + estrutura.
        let piso = |x: f32, z: f32| (mapa.terreno)(x, z).map(|h| h + mapa.movimento.acima_do_terreno(x, z).unwrap_or(0.0));
        let chao: Chao = &piso;
        if monster.is_dead {
            self.state = MonsterState::Dead;
            self.sessao = Sessao::Nenhuma;
            self.parado = true;
            return None;
        }

        self.attack_cooldown_ms = self.attack_cooldown_ms.saturating_sub(delta_ms);
        self.espera_ms = self.espera_ms.saturating_sub(delta_ms);

        // `IncIdleSealMode(MODE_INDEX_STUN/SLEEP)` (`filter_Dizzy`, `filter_Sleep`): parado,
        // sem golpe nem passo, até o filtro sair.
        if monster.efeitos.sem_acao() {
            if !self.parado {
                return Some(self.parar(monster, monster.corrida(), MODO_CORRER));
            }
            return None;
        }

        self.batimento_ms += delta_ms;
        while self.batimento_ms >= Self::BATIMENTO_MS {
            self.batimento_ms -= Self::BATIMENTO_MS;
            self.batimento(monster, players);
        }

        // 0. Monstro agressivo procura briga: sem alvo, ele pega o jogador vivo mais perto
        // dentro do alcance de visão. No original é o **jogador** que avisa ao andar —
        // `GM_MSG_WATCHING_YOU` difundido a quem tem `MSG_MASK_PLAYER_MOVE`, a marca que só
        // o monstro com `aggressive_mode` recebe (`npcgenerator.cpp:2534-2537`,
        // `playerctrl.cpp:265-276`) —, num raio de `GetMaxMobSightRange`, 15 m
        // (`worldmanager.cpp:48`). Quem decide odiar é a política do `aipolicy.data`, que
        // ainda não interpretamos: aqui o agressivo odeia sempre (B76).
        if monster.agressivo && self.get_highest_threat_target().is_none() {
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
                .filter(|(_, d)| *d <= Self::ALCANCE_DE_VISAO)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((id, _)) = mais_perto {
                self.add_threat(id, 1);
            }
        }

        // 1. Com alvo: perseguir e bater.
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
            let distancia = monster.position.distance(&posicao_do_alvo);

            if distancia <= monster.attack_range {
                // `range < _range_min`: a sessão de perseguição acaba (`npcsession.cpp:185-189`).
                self.seguir = None;
                self.state = MonsterState::Attacking;
                self.sessao = Sessao::Perseguindo;
                if !self.parado {
                    return Some(self.parar(monster, monster.corrida(), MODO_CORRER));
                }
                if self.attack_cooldown_ms == 0 {
                    // O intervalo entre golpes é o `attack_speed` do próprio monstro, em
                    // tiques de 50 ms (`ChangeInterval(_cur_prop.attack_speed)`,
                    // `gs/npcsession.cpp:60-70`). Era 1,5 s escrito aqui para todos (B62).
                    self.attack_cooldown_ms = (monster.ataque_em_ticks.max(4) as u32) * 50;
                    // Golpe que erra é resultado legítimo, e o `dano()` devolve zero nele.
                    let dano = match (jogador, mascote) {
                        (Some(p), _) => crate::combat::CombatEngine::monstro_ataca_jogador(monster, p, distancia).dano(),
                        (None, Some(m)) => crate::combat::resolver(
                            &crate::combat::CombatEngine::golpe_de_monstro(monster),
                            &crate::combat::CombatEngine::defesa_do_monstro(m),
                            distancia,
                            false,
                            crate::combat::Rolagens::sortear(),
                        )
                        .dano(),
                        (None, None) => 0,
                    };
                    return Some(AcaoDoMonstro::Atacou { alvo: target_id, dano });
                }
                return None;
            }

            if distancia >= monster.aggro_range.max(Self::PERSEGUICAO_MINIMA) {
                // Longe demais: perde o alvo, e sem alvo nenhum volta para casa.
                self.aggro_table.remove(&target_id);
                return self.sem_alvo(monster, mapa);
            }

            // `MODE_INDEX_ROOT` (`filter_Fix`): não anda, mas bate se o alvo vier.
            if monster.efeitos.preso() {
                if !self.parado {
                    return Some(self.parar(monster, monster.corrida(), MODO_CORRER));
                }
                return None;
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
            // Para um pouco antes do alcance, como o `_range_target` do original.
            let parar_a = (monster.attack_range * 0.9).max(0.5);
            let passo = monster.corrida() * Self::PASSO_DE_PERSEGUICAO_MS as f32 / 1000.0;
            if monster.habitat == Habitat::Chao {
                return self.perseguir(monster, target_id, posicao_do_alvo, passo, parar_a, mapa);
            }
            return self.passo(
                monster,
                posicao_do_alvo,
                passo,
                parar_a,
                Self::PASSO_DE_PERSEGUICAO_MS,
                monster.corrida(),
                MODO_CORRER,
                chao,
            );
        }

        self.sem_alvo(monster, mapa)
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

        let livre = matches!(self.sessao, Sessao::Nenhuma) && self.aggro_table.is_empty();
        if livre && monster.patrulha && self.idle_timer > 0 {
            self.cruise_timer = self.cruise_timer.wrapping_sub(1) & 31;
            if self.cruise_timer == 0 {
                self.comecar_passeio(monster);
            }
        }
    }

    fn comecar_passeio(&mut self, monster: &MonsterEntity) {
        let mut rng = rand::thread_rng();
        let r = Self::RAIO_DO_PASSEIO;
        let destino = Vector3::new(
            monster.spawn_center.x + rng.gen_range(-r..=r),
            monster.spawn_center.y,
            monster.spawn_center.z + rng.gen_range(-r..=r),
        );
        self.sessao = Sessao::Passeando { destino, passos_restantes: Self::PASSOS_DO_PASSEIO };
        // A espera **não** é zerada (B104): na emenda (`ai_rest_task::OnSessionEnd`, 10 %) o
        // último passo do passeio anterior acabou de sair com `use_time` de 1 s, e o primeiro
        // do novo saía no tique seguinte — dois `OBJECT_MOVE` a ~50 ms, e o cliente corria ou
        // pulava o monstro para alcançar o segundo destino. No começo normal ela já é zero.
        self.state = MonsterState::Patrol;
    }

    /// O que fazer sem alvo: terminar a volta para casa ou o passeio em curso.
    fn sem_alvo(&mut self, monster: &mut MonsterEntity, mapa: &Mapa) -> Option<AcaoDoMonstro> {
        let piso = |x: f32, z: f32| (mapa.terreno)(x, z).map(|h| h + mapa.movimento.acima_do_terreno(x, z).unwrap_or(0.0));
        let chao: Chao = &piso;
        self.seguir = None;
        self.perseguindo = None;
        match self.sessao {
            Sessao::Perseguindo => {
                // Acabou o combate: volta para onde nasceu (`ai_policy::RollBack`).
                self.sessao = Sessao::Voltando;
                self.espera_ms = 0;
                self.state = MonsterState::Idle;
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
                let acao = self.passo(
                    monster,
                    monster.spawn_center,
                    passo,
                    0.0,
                    Self::PASSO_DE_PATRULHA_MS,
                    monster.corrida(),
                    MODO_CORRER,
                    chao,
                );
                if matches!(acao, Some(AcaoDoMonstro::Parou { .. }) | None) {
                    self.sessao = Sessao::Nenhuma;
                }
                acao
            }
            Sessao::Passeando { destino, passos_restantes } => {
                if self.espera_ms > 0 {
                    return None;
                }
                self.espera_ms = Self::PASSO_DE_PATRULHA_MS;
                if passos_restantes <= 0 {
                    self.passeio = None;
                    self.sessao = Sessao::Nenhuma;
                    self.state = MonsterState::Idle;
                    return (!self.parado).then(|| self.parar(monster, monster.andar(), MODO_ANDAR));
                }
                self.sessao = Sessao::Passeando { destino, passos_restantes: passos_restantes - 1 };
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
                let (dx, dz) = (destino.x - monster.position.x, destino.z - monster.position.z);
                if matches!(acao, Some(AcaoDoMonstro::Andou { .. })) && dx * dx + dz * dz <= 0.05 * 0.05 {
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
    fn ir_para(&mut self, monster: &mut MonsterEntity, p: V3, tempo_ms: u32, velocidade: f32, modo: u8) -> Option<AcaoDoMonstro> {
        let (dx, dy, dz) = (p.x - monster.position.x, p.y - monster.position.y, p.z - monster.position.z);
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
            s.comecar(de, meta, passo, alcance * 0.6, d2, Some(&mut self.info), mapa);
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
            return (!self.parado).then(|| self.parar(monster, monster.corrida(), MODO_CORRER));
        }
        let p = s.posicao();
        self.seguir = Some(s);
        self.ir_para(monster, p, Self::PASSO_DE_PERSEGUICAO_MS, monster.corrida(), MODO_CORRER)
    }

    /// `ai_returnhome_task` → `session_npc_patrol::Run` (`gs/aipolicy.cpp:1291-1322`,
    /// `gs/npcsession.cpp:883-960`): `follow_target` até casa com alcance de 0,8 m; acaba a
    /// 1,2 passo de casa, ao chegar ou quando o agente desiste. Se ao fim ainda estiver a mais
    /// de 10 m (`GetReturnHomeRange` = 10², `aipolicy.h:1393`), o `ReturnHome` o põe em casa
    /// de uma vez (`gs/ainpc.cpp:98-106`: `stop_move` com `MOVE_MODE_RETURN` e 0x500).
    fn voltar(&mut self, monster: &mut MonsterEntity, passo: f32, mapa: &Mapa) -> Option<AcaoDoMonstro> {
        let casa = monster.spawn_center;
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
            return self.ir_para(monster, p, Self::PASSO_DE_PATRULHA_MS, monster.corrida(), MODO_CORRER);
        }
        self.sessao = Sessao::Nenhuma;
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

    /// `session_npc_cruise::Run` (`gs/npcsession.cpp:590-650`) com o `cruise`: a meta é
    /// sorteada no disco de 10 m em volta de onde nasceu, alcançável e de preferência em linha
    /// reta, e o caminho até ela desvia de obstáculo.
    fn passear(&mut self, monster: &mut MonsterEntity, passo: f32, mapa: &Mapa) -> Option<AcaoDoMonstro> {
        let de = V3::new(monster.position.x, monster.position.y, monster.position.z);
        let casa = monster.spawn_center;
        let mut p = match self.passeio.take() {
            Some(p) => p,
            None => {
                let mut p = Passeio::default();
                p.comecar(de, V3::new(casa.x, casa.y, casa.z), passo, Self::RAIO_DO_PASSEIO, mapa);
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
        let acao = self.ir_para(monster, alvo, Self::PASSO_DE_PATRULHA_MS, monster.andar(), MODO_ANDAR);
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
