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
//! O original caminha num mapa de movimento (`GetMoveMap`, desvio de obstáculo). Aqui o
//! monstro anda em linha reta e **assenta no chão do `.hmap`** a cada passo — é o que
//! resolve "anda embaixo da terra e no ar". Obstáculo (casa, pedra) ainda não é desviado.
//!
//! # A unidade do `OBJECT_MOVE`
//!
//! `use_time` em **milissegundos** e `speed` em **1/256 de m/s** (`gs/npcsession.cpp:258`,
//! `(unsigned short)(GetSpeed()* 256.0f + 0.5f)`). Até 2026-09-12 ia centésimo de segundo e
//! centésimo de m/s: o cliente recebia um trecho de 2 m "para fazer em 50 ms" e o monstro
//! disparava — o "persegue muito rápido" do teste do POTATO.

use crate::entity::{MonsterEntity, PlayerEntity};
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
                .filter(|(_, d)| *d <= Self::ALCANCE_DE_VISAO)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((id, _)) = mais_perto {
                self.add_threat(id, 1);
            }
        }

        // 1. Com alvo: perseguir e bater.
        if let Some(target_id) = self.get_highest_threat_target() {
            let alvo = match players.get(&target_id) {
                Some(p) if p.hp > 0 => p,
                _ => {
                    self.aggro_table.remove(&target_id);
                    return self.sem_alvo(monster, chao);
                }
            };
            let distancia = monster.position.distance(&alvo.position);

            if distancia <= monster.attack_range {
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
                    let dano = crate::combat::CombatEngine::monstro_ataca_jogador(
                        monster, alvo, distancia,
                    )
                    .dano();
                    return Some(AcaoDoMonstro::Atacou { alvo: target_id, dano });
                }
                return None;
            }

            if distancia >= monster.aggro_range.max(Self::PERSEGUICAO_MINIMA) {
                // Longe demais: perde o alvo, e sem alvo nenhum volta para casa.
                self.aggro_table.remove(&target_id);
                return self.sem_alvo(monster, chao);
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
            return self.passo(
                monster,
                alvo.position,
                passo,
                parar_a,
                Self::PASSO_DE_PERSEGUICAO_MS,
                monster.corrida(),
                MODO_CORRER,
                chao,
            );
        }

        self.sem_alvo(monster, chao)
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
        self.espera_ms = 0;
        self.state = MonsterState::Patrol;
    }

    /// O que fazer sem alvo: terminar a volta para casa ou o passeio em curso.
    fn sem_alvo(&mut self, monster: &mut MonsterEntity, chao: Chao) -> Option<AcaoDoMonstro> {
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
                    self.sessao = Sessao::Nenhuma;
                    self.state = MonsterState::Idle;
                    return (!self.parado).then(|| self.parar(monster, monster.andar(), MODO_ANDAR));
                }
                self.sessao = Sessao::Passeando { destino, passos_restantes: passos_restantes - 1 };
                let passo = monster.andar() * Self::PASSO_DE_PATRULHA_MS as f32 / 1000.0;
                let acao = self.passo(
                    monster,
                    destino,
                    passo,
                    0.0,
                    Self::PASSO_DE_PATRULHA_MS,
                    monster.andar(),
                    MODO_ANDAR,
                    chao,
                );
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
