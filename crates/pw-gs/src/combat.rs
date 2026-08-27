use crate::entity::{MonsterEntity, PlayerEntity};
use rand::Rng;

pub struct CombatEngine;

impl CombatEngine {
    /// Calcula o dano físico de um jogador contra um monstro
    pub fn calculate_player_to_monster_damage(
        player: &PlayerEntity,
        monster: &MonsterEntity,
    ) -> (i64, bool) {
        let mut rng = rand::thread_rng();

        // 1. Dano base aleatório entre attack_min e attack_max
        let raw_damage = rng.gen_range(player.attack_min..=player.attack_max) as f32;

        // 2. Fator de redução de defesa física do alvo
        let def = monster.def_phys as f32;
        let def_factor = 1.0 / (1.0 + (def / (100.0 * player.level.max(1) as f32)));

        // 3. Cálculo de Acerto Crítico
        let is_crit = rng.gen::<f32>() < player.crit_rate;
        let crit_multiplier = if is_crit { 2.0 } else { 1.0 };

        let final_damage = (raw_damage * def_factor * crit_multiplier).max(1.0) as i64;
        (final_damage, is_crit)
    }

    /// Calcula o dano físico de um monstro contra um jogador
    pub fn calculate_monster_to_player_damage(
        monster: &MonsterEntity,
        player: &PlayerEntity,
    ) -> i32 {
        let mut rng = rand::thread_rng();
        let raw_damage = rng.gen_range(monster.attack_min..=monster.attack_max) as f32;

        let def = player.def_phys as f32;
        let def_factor = 1.0 / (1.0 + (def / (100.0 * monster.level.max(1) as f32)));

        (raw_damage * def_factor).max(1.0) as i32
    }
}
