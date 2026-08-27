use crate::entity::{MonsterEntity, PlayerEntity};
use pw_core::Vector3;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterState {
    Idle,
    Patrol,
    Chasing,
    Attacking,
    Dead,
}

#[derive(Debug, Clone, Default)]
pub struct MonsterAi {
    pub state: MonsterState,
    pub aggro_table: HashMap<i64, i64>, // (Target EntityId -> Threat Value)
    pub attack_cooldown_ms: u32,
}

impl MonsterAi {
    pub fn new() -> Self {
        Self {
            state: MonsterState::Idle,
            aggro_table: HashMap::new(),
            attack_cooldown_ms: 0,
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
    ) -> Option<(i64, i32)> { // Retorna Some((target_id, dano)) se executou um ataque
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
                let damage = crate::combat::CombatEngine::calculate_monster_to_player_damage(monster, target_player);
                return Some((target_id, damage));
            }
        } else if distance < 35.0 {
            // Persegue o jogador em direção à sua coordenada
            self.state = MonsterState::Chasing;
            let dir_x = target_player.position.x - monster.position.x;
            let dir_z = target_player.position.z - monster.position.z;
            let len = (dir_x * dir_x + dir_z * dir_z).sqrt().max(0.001);

            let move_dist = monster.move_speed * (delta_ms as f32 / 1000.0);
            monster.position.x += (dir_x / len) * move_dist;
            monster.position.z += (dir_z / len) * move_dist;
        } else {
            // Alvo muito longe -> perde o aggro e retorna à base
            self.aggro_table.remove(&target_id);
            self.state = MonsterState::Idle;
        }

        None
    }
}
