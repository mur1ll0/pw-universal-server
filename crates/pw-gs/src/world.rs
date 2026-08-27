use crate::ai::MonsterAi;
use crate::entity::{ItemDropEntity, MonsterEntity, NpcEntity, PlayerEntity};
use crate::grid::SpatialGrid;
use pw_core::{RoleId, Vector3, WorldId};
use pw_data_loader::GameDataManager;
use pw_storage::CharacterRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

pub struct WorldInstance {
    pub world_id: WorldId,
    pub grid: SpatialGrid,
    pub players: HashMap<i64, PlayerEntity>,
    pub monsters: HashMap<i64, (MonsterEntity, MonsterAi)>,
    pub npcs: HashMap<i64, NpcEntity>,
    pub drops: HashMap<i64, ItemDropEntity>,
    pub data_manager: Arc<GameDataManager>,
    pub char_repo: CharacterRepository,
    entity_counter: i64,
    autosave_timer_ms: u32,
}

impl WorldInstance {
    pub fn new(
        world_id: WorldId,
        data_manager: Arc<GameDataManager>,
        char_repo: CharacterRepository,
    ) -> Self {
        Self {
            world_id,
            grid: SpatialGrid::new(),
            players: HashMap::new(),
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            drops: HashMap::new(),
            data_manager,
            char_repo,
            entity_counter: 100000,
            autosave_timer_ms: 0,
        }
    }

    /// Inicializa os Spawns de monstros e NPCs a partir do `npcgen.data` do mapa específico
    pub fn init_spawns(&mut self) {
        info!("Inicializando monstros e NPCs do World #{} a partir do seu npcgen.data dedicado...", self.world_id);

        if let Some(spawns) = self.data_manager.map_spawns.get(&self.world_id) {
            for area in &spawns.areas {
                for _ in 0..area.count {
                    self.entity_counter += 1;
                    let monster_id = self.entity_counter;

                    let monster = MonsterEntity {
                        id: monster_id,
                        template_id: area.template_id,
                        name: "Monstro".to_string(),
                        level: 1,
                        hp: 500,
                        max_hp: 500,
                        mp: 100,
                        max_mp: 100,
                        def_phys: 50,
                        def_magic: 50,
                        attack_min: 20,
                        attack_max: 35,
                        attack_range: 2.5,
                        exp: 100,
                        sp: 20,
                        aipolicy_id: 0,
                        drop_table_id: 0,
                        position: area.center,
                        spawn_center: area.center,
                        move_speed: 3.5,
                        is_dead: false,
                        respawn_timer_ms: 0,
                        respawn_delay_ms: area.respawn_sec * 1000,
                        target_id: None,
                        buffs: Vec::new(),
                    };

                    self.grid.add_entity(monster_id, monster.position, false);
                    self.monsters.insert(monster_id, (monster, MonsterAi::new()));
                }
            }
        }

        info!(
            "World #{} inicializado com {} monstros ativos a partir do seu npcgen.data!",
            self.world_id,
            self.monsters.len()
        );
    }

    /// Adiciona um jogador que entrou neste mapa
    pub fn add_player(&mut self, player: PlayerEntity) {
        let role_id = player.role_id as i64;
        self.grid.add_entity(role_id, player.position, true);
        self.players.insert(role_id, player);
        info!("Jogador #{} entrou no World #{}", role_id, self.world_id);
    }

    /// Remove um jogador ao deslogar ou mudar de mapa
    pub fn remove_player(&mut self, role_id: RoleId) -> Option<PlayerEntity> {
        let id = role_id as i64;
        self.grid.remove_entity(id);
        self.players.remove(&id)
    }

    /// Ciclo de Simulação em Tempo Real (Loop de 50ms / 20 TPS)
    pub async fn tick(&mut self, delta_ms: u32) {
        // 1. Atualização da Inteligência Artificial dos Monstros
        let mut attacks_to_process = Vec::new();

        for (monster, ai) in self.monsters.values_mut() {
            if monster.is_dead {
                if monster.respawn_timer_ms > 0 {
                    monster.respawn_timer_ms = monster.respawn_timer_ms.saturating_sub(delta_ms);
                    if monster.respawn_timer_ms == 0 {
                        // Renascimento do Monstro
                        monster.is_dead = false;
                        monster.hp = monster.max_hp;
                        monster.position = monster.spawn_center;
                    }
                }
                continue;
            }

            if let Some(attack) = ai.tick(monster, &self.players, delta_ms) {
                attacks_to_process.push(attack);
            }
        }

        // 2. Aplica danos causados pelos monstros nos jogadores
        for (player_id, damage) in attacks_to_process {
            if let Some(player) = self.players.get_mut(&player_id) {
                player.hp = (player.hp - damage).max(0);
                debug!("Monstro causou {} de dano no Jogador #{} (HP restante: {})", damage, player_id, player.hp);
            }
        }

        // 3. Autosave Periódico de Personagens para o PostgreSQL (a cada 60s)
        self.autosave_timer_ms += delta_ms;
        if self.autosave_timer_ms >= 60_000 {
            self.autosave_timer_ms = 0;
            for player in self.players.values() {
                let _ = self
                    .char_repo
                    .save_status(
                        player.role_id,
                        player.level,
                        player.cultivation,
                        player.exp,
                        player.sp,
                        player.hp,
                        player.mp,
                        player.money,
                        self.world_id,
                        &player.position,
                    )
                    .await;
            }
            debug!("Autosave periódico de {} jogadores executado com sucesso no World #{}", self.players.len(), self.world_id);
        }
    }
}
