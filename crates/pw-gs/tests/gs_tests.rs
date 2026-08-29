use pw_core::{CharacterClass, Gender, Race, Vector3};
use pw_gs::combat::CombatEngine;
use pw_gs::entity::{MonsterEntity, PlayerEntity};
use pw_gs::grid::SpatialGrid;
use pw_gs::ai::{MonsterAi, MonsterState};

#[test]
fn test_spatial_grid_queries() {
    let mut grid = SpatialGrid::new();

    let p1 = Vector3::new(100.0, 20.0, 100.0);
    let p2 = Vector3::new(120.0, 20.0, 100.0);
    let p3 = Vector3::new(500.0, 20.0, 500.0);

    grid.add_entity(1001, p1, true);
    grid.add_entity(1002, p2, false);
    grid.add_entity(1003, p3, false);

    // Consulta em raio de 50m a partir de p1
    let nearby = grid.get_entities_in_range(&p1, 50.0);
    assert_eq!(nearby.len(), 2);
    assert!(nearby.contains(&1001));
    assert!(nearby.contains(&1002));
    assert!(!nearby.contains(&1003));

    // Consulta apenas jogadores
    let players = grid.get_players_in_range(&p1, 50.0);
    assert_eq!(players.len(), 1);
    assert_eq!(players[0], 1001);

    // Move p2 para longe
    grid.update_position(1002, Vector3::new(600.0, 20.0, 600.0));
    let nearby_after = grid.get_entities_in_range(&p1, 50.0);
    assert_eq!(nearby_after.len(), 1);
    assert_eq!(nearby_after[0], 1001);
}

#[test]
fn test_combat_engine_damage_calculation() {
    let player = PlayerEntity {
        role_id: 1001,
        name: "TestHero".to_string(),
        race: Race::Human,
        cls: CharacterClass::Blademaster,
        gender: Gender::Male,
        level: 30,
        cultivation: 9,
        hp: 1500,
        max_hp: 1500,
        mp: 500,
        max_mp: 500,
        exp: 0,
        sp: 0,
        money: 0,
        strength: 50,
        agility: 30,
        vitality: 40,
        energy: 10,
        def_phys: 200,
        def_metal: 50,
        def_wood: 50,
        def_water: 50,
        def_fire: 50,
        def_earth: 50,
        attack_min: 150,
        attack_max: 250,
        magic_attack_min: 0,
        magic_attack_max: 0,
        attack_speed: 1.2,
        move_speed: 4.8,
        crit_rate: 0.1,
        position: Vector3::new(0.0, 0.0, 0.0),
        target_id: None,
        buffs: Vec::new(),
    };

    let monster = MonsterEntity {
        id: -2001,
        template_id: 1001,
        name: "Lobo Selvagem".to_string(),
        level: 25,
        hp: 2000,
        max_hp: 2000,
        mp: 100,
        max_mp: 100,
        def_phys: 120,
        def_magic: 50,
        attack_min: 50,
        attack_max: 90,
        attack_range: 2.5,
        exp: 300,
        sp: 60,
        aipolicy_id: 0,
        drop_table_id: 0,
        position: Vector3::new(2.0, 0.0, 0.0),
        spawn_center: Vector3::new(2.0, 0.0, 0.0),
        move_speed: 3.5,
        is_dead: false,
        respawn_timer_ms: 0,
        respawn_delay_ms: 5000,
        target_id: None,
        buffs: Vec::new(),
    };

    let (dmg, is_crit) = CombatEngine::calculate_player_to_monster_damage(&player, &monster);
    assert!(dmg > 0, "Dano deve ser maior que 0");
    if is_crit {
        assert!(dmg >= 50, "Crítico deve causar dano amplificado");
    }

    let m_dmg = CombatEngine::calculate_monster_to_player_damage(&monster, &player);
    assert!(m_dmg > 0);
}

#[test]
fn test_monster_ai_threat_table() {
    let mut ai = MonsterAi::new();
    assert_eq!(ai.state, MonsterState::Idle);

    ai.add_threat(1001, 50);
    ai.add_threat(1002, 120);
    ai.add_threat(1001, 80); // 1001 total = 130

    let highest = ai.get_highest_threat_target();
    assert_eq!(highest, Some(1001));
}
