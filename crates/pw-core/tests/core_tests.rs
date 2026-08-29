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
    assert_eq!(skills[0].0, 125); // Pluma Espiritual

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
