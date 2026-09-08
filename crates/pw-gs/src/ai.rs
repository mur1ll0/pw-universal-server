use crate::entity::{MonsterEntity, PlayerEntity};
use pw_core::Vector3;
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

/// O que o monstro decidiu fazer neste tique.
///
/// Antes o `tick` devolvia só `Option<(alvo, dano)>`, e o **movimento não saía daqui**:
/// a IA mexia em `monster.position` e nada mais acontecia — nem a grade espacial sabia,
/// nem o cliente. Em jogo, 2026-09-07: "os monstros não estão se movendo". Ele andava;
/// ninguém era avisado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AcaoDoMonstro {
    /// Bateu em alguém.
    Atacou { alvo: i64, dano: i32 },
    /// Andou até `destino`. Só é devolvido quando vale a pena avisar — ver
    /// [`MonsterAi::PASSO_MINIMO_PARA_AVISAR`].
    Andou { destino: Vector3, velocidade: f32 },
}

#[derive(Debug, Clone, Default)]
pub struct MonsterAi {
    pub state: MonsterState,
    pub aggro_table: HashMap<i64, i64>, // (Target EntityId -> Threat Value)
    pub attack_cooldown_ms: u32,
    /// Onde o monstro estava da última vez que um movimento foi anunciado. O cliente
    /// interpola entre um `OBJECT_MOVE` e o próximo, então mandar um por tique de 50 ms
    /// seria desperdício de rede sem ganho nenhum na tela.
    ultima_posicao_anunciada: Option<Vector3>,
}

impl MonsterAi {
    /// Quanto o monstro precisa andar para valer um `OBJECT_MOVE`. Dois metros a 4 m/s
    /// dão um pacote a cada meio segundo por monstro em perseguição, contra vinte por
    /// segundo se fosse um por tique.
    pub const PASSO_MINIMO_PARA_AVISAR: f32 = 2.0;

    /// Piso da distância de perseguição, para monstro cujo `aggro_range` é pequeno demais
    /// para ele sair do lugar.
    pub const PERSEGUICAO_MINIMA: f32 = 15.0;

    pub fn new() -> Self {
        Self {
            state: MonsterState::Idle,
            aggro_table: HashMap::new(),
            attack_cooldown_ms: 0,
            ultima_posicao_anunciada: None,
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

    /// Atualiza o ciclo de IA do monstro a cada tick (50ms)
    pub fn tick(
        &mut self,
        monster: &mut MonsterEntity,
        players: &HashMap<i64, PlayerEntity>,
        delta_ms: u32,
    ) -> Option<AcaoDoMonstro> {
        if monster.is_dead {
            self.state = MonsterState::Dead;
            return None;
        }

        if self.attack_cooldown_ms > 0 {
            self.attack_cooldown_ms = self.attack_cooldown_ms.saturating_sub(delta_ms);
        }

        // 1. Localiza o alvo prioritário
        let target_id = match self.get_highest_threat_target() {
            Some(id) => id,
            None => {
                self.state = MonsterState::Idle;
                return None;
            }
        };

        // 2. Verifica se o jogador alvo ainda está vivo e no alcance
        let target_player = match players.get(&target_id) {
            Some(p) if p.hp > 0 => p,
            _ => {
                self.aggro_table.remove(&target_id);
                return None;
            }
        };

        let distance = monster.position.distance(&target_player.position);

        // 3. Máquina de Estados (Chasing vs Attacking)
        if distance <= monster.attack_range {
            self.state = MonsterState::Attacking;
            if self.attack_cooldown_ms == 0 {
                self.attack_cooldown_ms = 1500; // Cooldown de 1.5s entre ataques básicos
                // Golpe que erra é resultado legítimo, e o `dano()` devolve zero nele —
                // quem recebe decide o que mostrar. Antes o dano nunca podia ser zero
                // porque não havia rolagem de acerto nenhuma.
                let damage = crate::combat::CombatEngine::monstro_ataca_jogador(
                    monster,
                    target_player,
                    distance,
                )
                .dano();
                return Some(AcaoDoMonstro::Atacou { alvo: target_id, dano: damage });
            }
        } else if distance < monster.aggro_range.max(Self::PERSEGUICAO_MINIMA) {
            // Persegue o jogador em direção à sua coordenada.
            //
            // O limite era `35.0` escrito no código; agora é o `aggro_range` do
            // `elements.data`, com um piso para que monstro de raio minúsculo ainda dê
            // um passo em vez de ficar parado a dois metros do alvo.
            self.state = MonsterState::Chasing;
            let dir_x = target_player.position.x - monster.position.x;
            let dir_z = target_player.position.z - monster.position.z;
            let len = (dir_x * dir_x + dir_z * dir_z).sqrt().max(0.001);

            let move_dist = monster.move_speed * (delta_ms as f32 / 1000.0);
            monster.position.x += (dir_x / len) * move_dist;
            monster.position.z += (dir_z / len) * move_dist;

            // Só avisa quando andou o bastante para valer um pacote: o cliente interpola
            // entre um `OBJECT_MOVE` e o próximo.
            let anterior = self.ultima_posicao_anunciada.unwrap_or(monster.spawn_center);
            if monster.position.distance(&anterior) >= Self::PASSO_MINIMO_PARA_AVISAR {
                self.ultima_posicao_anunciada = Some(monster.position);
                return Some(AcaoDoMonstro::Andou {
                    destino: monster.position,
                    velocidade: monster.move_speed,
                });
            }
        } else {
            // Alvo muito longe -> perde o aggro e retorna à base
            self.aggro_table.remove(&target_id);
            self.state = MonsterState::Idle;
            self.ultima_posicao_anunciada = None;
        }

        None
    }
}
