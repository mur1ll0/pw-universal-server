use pw_core::{CharacterClass, ContainerType, Race, Vector3};

#[test]
fn test_character_class_and_race_conversion() {
    assert_eq!(CharacterClass::from_u8(0), Some(CharacterClass::Blademaster));
    assert_eq!(CharacterClass::from_u8(1), Some(CharacterClass::Wizard));
    assert_eq!(CharacterClass::from_u8(2), Some(CharacterClass::Psychomancer));
    assert_eq!(CharacterClass::from_u8(3), Some(CharacterClass::Venomancer));
    assert_eq!(CharacterClass::from_u8(4), Some(CharacterClass::Barbarian));
    assert_eq!(CharacterClass::from_u8(5), Some(CharacterClass::Assassin));
    assert_eq!(CharacterClass::from_u8(6), Some(CharacterClass::Archer));
    assert_eq!(CharacterClass::from_u8(7), Some(CharacterClass::Cleric));
    assert_eq!(CharacterClass::from_u8(12), None);

    assert_eq!(Race::from_u8(0), Some(Race::Human));
    assert_eq!(Race::from_u8(1), Some(Race::WingedElf));
    assert_eq!(Race::from_u8(2), Some(Race::Untamed));
    assert_eq!(Race::from_u8(3), Some(Race::Tideborn));
    assert_eq!(Race::from_u8(4), Some(Race::Earthguard));
    assert_eq!(Race::from_u8(5), Some(Race::Nightshade));
    assert_eq!(Race::from_u8(6), None);
}

#[test]
fn test_vector3_math() {
    let v1 = Vector3::new(10.0, 20.0, 30.0);
    let v2 = Vector3::new(13.0, 24.0, 30.0); // dist = sqrt(3^2 + 4^2) = 5.0
    
    assert_eq!(v1.distance(&v2), 5.0);
    assert_eq!(v1.distance_squared(&v2), 25.0);
    
    let sum = v1 + v2;
    assert_eq!(sum.x, 23.0);
    assert_eq!(sum.y, 44.0);
    assert_eq!(sum.z, 60.0);

    let diff = v2 - v1;
    assert_eq!(diff.x, 3.0);
    assert_eq!(diff.y, 4.0);
    assert_eq!(diff.z, 0.0);
}

#[test]
fn test_default_spawn_positions_and_skills() {
    let cleric = CharacterClass::Cleric;
    let (sx, sy, sz) = cleric.default_spawn_position();
    assert_eq!(sx, -741.5);
    assert_eq!(sy, 219.1);
    assert_eq!(sz, -1234.8);

    let skills = cleric.default_skills();
    assert!(!skills.is_empty());
    assert_eq!(skills[0].0, 125); // 羽箭 — o ataque inicial do Sacerdote

    let (hp, mp) = cleric.default_hp_mp();
    assert!(hp > 0);
    assert!(mp > 0);
}

#[test]
fn test_container_type_conversions() {
    assert_eq!(ContainerType::from_i16(0), ContainerType::Inventory);
    assert_eq!(ContainerType::from_i16(1), ContainerType::Equipment);
    assert_eq!(ContainerType::from_i16(2), ContainerType::Storehouse);
    assert_eq!(ContainerType::from_i16(3), ContainerType::Fashion);
    assert_eq!(ContainerType::from_i16(4), ContainerType::PetCorral);
}

/// A lista inicial de cada classe veio dos stubs do `ElementSkill` do client 1.5.5:
/// `cls` igual à classe, `rank == 0`, `max_level == 10`, `GetRequiredLevel[0] == 0` e SP
/// cobrado a partir do nível 2. Fixar os ids aqui é o que impede a lista de voltar a ser
/// palpite — foi um palpite que pôs habilidades de nível 29 a 49 nos sacerdotes de teste
/// e deixou o jogador sem nenhuma habilidade utilizável em jogo (2026-09-07).
#[test]
fn as_habilidades_iniciais_sao_as_da_arvore_da_classe() {
    use CharacterClass::*;

    let esperado: [(CharacterClass, &[i16]); 12] = [
        (Blademaster, &[1]),
        (Wizard, &[81]),
        (Psychomancer, &[1125, 1126]),
        (Venomancer, &[299]),
        (Barbarian, &[102]),
        (Assassin, &[1111]),
        (Archer, &[234, 235]),
        (Cleric, &[125, 113]),
        (Seeker, &[1350]),
        (Mystic, &[1374, 1381]),
        (Duskblade, &[2547]),
        (Stormbringer, &[2571]),
    ];

    for (cls, da_classe) in esperado {
        let skills = cls.default_skills();
        let ids: Vec<i16> = skills.iter().map(|s| s.0).collect();

        let mut completo = da_classe.to_vec();
        completo.push(167); // 回城术 — cls 255, entra em todas
        assert_eq!(ids, completo, "{cls:?}");

        // Nenhuma repetida, e todas no nível 1: `character_skills` tem chave
        // (character_id, skill_id), e um id repetido viraria conflito na inserção.
        let mut unicos = ids.clone();
        unicos.sort_unstable();
        unicos.dedup();
        assert_eq!(unicos.len(), ids.len(), "{cls:?} tem habilidade repetida");
        assert!(skills.iter().all(|s| s.1 == 1), "{cls:?} não começa tudo no nível 1");
    }
}

/// O par arma inicial / tipo maior é o que o cliente confere em `ValidWeapon` antes de
/// deixar conjurar. Cada classe precisa dos dois, e classes diferentes que usam o mesmo
/// tipo maior têm que usar a mesma arma.
#[test]
fn cada_classe_tem_arma_inicial_do_seu_tipo_maior() {
    use CharacterClass::*;

    let todas = [
        Blademaster, Wizard, Psychomancer, Venomancer, Barbarian, Assassin,
        Archer, Cleric, Seeker, Mystic, Duskblade, Stormbringer,
    ];

    for cls in todas {
        assert!(cls.default_weapon_id() > 0, "{cls:?} sem arma inicial");
        assert!(cls.weapon_major_type() > 0, "{cls:?} sem tipo maior de arma");
    }

    // Mago, Feiticeira, Sacerdote e Místico conjuram com arma de Magia (292).
    for cls in [Wizard, Venomancer, Cleric, Mystic] {
        assert_eq!(cls.weapon_major_type(), 292, "{cls:?}");
        assert_eq!(cls.default_weapon_id(), 2251, "{cls:?}"); // Varinha
    }

    // O Graveto de Madeira (2867, tipo maior 5) não pode voltar para classe nenhuma:
    // foi ele que travou as habilidades dos sacerdotes em jogo.
    for cls in todas {
        assert_ne!(cls.default_weapon_id(), 2867, "{cls:?}");
    }
}
