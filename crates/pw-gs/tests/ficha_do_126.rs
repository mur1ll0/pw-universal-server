//! B101 — a ficha do personagem no realm 1.2.6 com os dados reais do `realm_126`.
//!
//! Relato do Murillo (Tsuko, Feiticeira nível 1): dano físico e mágico 1-1 na ficha. Causa:
//! o `ptemplate.conf` do 1.2.6 era recusado pelo leitor (seções do 1.5.5), e sem a base da
//! classe `recalcular_por_nivel` não fazia nada. Sem a pasta do realm o teste avisa e sai.

use pw_core::{ContainerType, ItemRecord, Vector3};
use pw_data_loader::GameDataManager;
use pw_gs::entity::{Equipamento, PlayerEntity};
use std::path::PathBuf;

fn realm_126() -> Option<GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    Some(d)
}

fn feiticeira_nivel_1() -> PlayerEntity {
    let pos = Vector3::new(0.0, 0.0, 0.0);
    let mut p = PlayerEntity {
        role_id: 1,
        name: "Alvo".into(),
        race: pw_core::Race::Human,
        cls: pw_core::CharacterClass::Venomancer,
        gender: pw_core::Gender::Male,
        level: 1,
        cultivation: 0,
        hp: 1000,
        max_hp: 1000,
        mp: 0,
        max_mp: 0,
        exp: 0,
        sp: 0,
        money: 0,
        strength: 5,
        agility: 5,
        vitality: 5,
        energy: 5,
        def_phys: 0,
        def_metal: 0,
        def_wood: 0,
        def_water: 0,
        def_fire: 0,
        def_earth: 0,
        attack_min: 1,
        attack_max: 1,
        magic_attack_min: 0,
        magic_attack_max: 0,
        armor: 0,
        attack_rate: 0,
        attack_degree: 0,
        defend_degree: 0,
        crit_damage_bonus: 0,
        attack_speed: 1.0,
        move_speed: 4.0,
        walk_speed: 2.0,
        swim_speed: 3.0,
        fly_speed: 5.0,
        attack_range: 2.5,
        hp_gen: 1,
        mp_gen: 1,
        crit_rate: 0.0,
        position: pos,
        target_id: None,
        efeitos: Default::default(),
        visiveis: std::collections::HashSet::new(),
        centro_do_stream: Vector3::new(0.0, 0.0, 0.0),
        voando: false,
        montaria: None,
        forma_enviada: None,
        operacao_de_pet: 0,
        modo_roupa: false,
        sec_level: 0,
        habilidades: Default::default(),
        crc_aparencia: 0,
        pontos_de_atributo: 0,
        reputacao: 0,
        combate_s: 0,
        contador_hp: 0,
        contador_mp: 0,
        recargas: std::collections::HashMap::new(),
        pecas: [None; pw_gs::entity::PECAS_VESTIDAS],
        auto_hp: None,
        daimon: None,
        auto_mp: None,
        recarga_do_auto_hp_s: 0,
        recarga_do_auto_mp_s: 0,
        npc_em_conversa: None,
        waypoints: Vec::new(),
        ap: 0,
        max_ap: 0,
        ap_por_golpe: 0,
        sentado: false,
        missoes: Default::default(),
        coleta: None,
        equipamento: Default::default(),
        ataque: None,
        conjuracao: None,
        dano_bruto: (1, 1),
        dano_magico_bruto: (1, 1),
        bonus_de_dano_pct: 0,
        bonus_magico_pct: 0,
    };
    p.hp = p.max_hp;
    p
}

#[test]
fn a_feiticeira_de_nivel_1_do_126_tem_dano_da_varinha_e_vida_da_classe() {
    let Some(d) = realm_126() else { return };
    assert!(!d.base_das_classes.is_empty(), "o ptemplate.conf do 1.2.6 não carregou");
    let mut p = feiticeira_nivel_1();
    // A Varinha Mágica (2251) do molde: dano 3-3 e mágico 5-5 no `WEAPON_ESSENCE` v7.
    let varinha = ItemRecord {
        id: None,
        character_id: 1,
        container_type: ContainerType::Equipment,
        slot: 0,
        item_id: 2251,
        count: 1,
        max_count: 1,
        refine_level: 0,
        sockets_count: 0,
        sockets: Vec::new(),
        durability: 2800,
        max_durability: 2800,
        bind_status: 0,
        octets: Vec::new(),
        custom_attributes: serde_json::Value::Null,
    };
    p.equipamento = Equipamento::dos_itens_com_addons(&[varinha], &d.equipamentos, Some(&d.addons));
    p.recalcular_por_nivel(&d.classes, Some(&d.base_das_classes));
    // Vida e mana do molde do `clsconfig` 1.2.6 (Feiticeira: 60/60 = 12 × 5 da
    // `CHARRACTER_CLASS_CONFIG`) — a mesma conta que o mundo faz.
    assert_eq!((p.max_hp, p.max_mp), (60, 60));
    // `UpdateAttack`/`UpdateMagic`: base da classe (1) + arma, com o bônus do atributo.
    assert!(p.attack_min > 1 && p.attack_max > 1, "dano físico {}-{}", p.attack_min, p.attack_max);
    assert!(p.magic_attack_min > 1 && p.magic_attack_max > 1, "dano mágico {}-{}", p.magic_attack_min, p.magic_attack_max);
    eprintln!("ficha: físico {}-{}, mágico {}-{}, vida {}, mana {}", p.attack_min, p.attack_max, p.magic_attack_min, p.magic_attack_max, p.max_hp, p.max_mp);

    // Enxame de Ferroadas (299) nível 1 com a tabela do `gs` 1.2.6: ataque mágico bruto 6
    // (1 da classe + 5 da varinha) × (100 + 5 da energia + 55 do ratio)% + 23,7 = 9 + 23 = 32
    // de madeira antes da resistência. A captura original mostra 20, 23, 23 e 24 no alvo
    // (`142`, B101); com a tabela do 1.5.5 (plus 124,5) saíam 75 e 106 no relato em jogo.
    let dano = d.habilidades.get(299).and_then(|h| h.dano.clone()).expect("dano da 299");
    let g = pw_gs::combat::CombatEngine::golpe_de_habilidade(&p, &dano, 1, 1.0).expect("golpe");
    assert_eq!(g.dano_magico[1], 32);
}
