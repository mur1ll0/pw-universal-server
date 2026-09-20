use pw_core::{InfoPet, PeEssence, TAMANHO_INFO_PET, TAMANHO_PE_ESSENCE_BASE};

#[test]
fn pe_essence_tamanho_e_roundtrip() {
    let mut ess = PeEssence::default();
    ess.pet_tid = 1234;
    ess.pet_egg_tid = 5678;
    ess.require_level = 10;
    ess.require_class = 0xFFFF;
    ess.skills.push((100, 1));
    ess.skills.push((101, 2));

    let bytes = ess.para_bytes();
    assert_eq!(bytes.len(), TAMANHO_PE_ESSENCE_BASE + 2 * 8);

    let lido = PeEssence::de_bytes(&bytes).expect("deve reler a essencia");
    assert_eq!(lido.pet_tid, 1234);
    assert_eq!(lido.pet_egg_tid, 5678);
    assert_eq!(lido.require_level, 10);
    assert_eq!(lido.require_class, 0xFFFF);
    assert_eq!(lido.skills, vec![(100, 1), (101, 2)]);
}

#[test]
fn info_pet_tamanho_e_de_essencia() {
    let mut ess = PeEssence::default();
    ess.pet_tid = 999;
    ess.pet_egg_tid = 888;
    ess.pet_class = 0; // mount
    ess.level = 5;

    let pet = InfoPet::de_essencia(&ess);
    assert_eq!(pet.pet_tid, 999);
    assert_eq!(pet.pet_egg_tid, 888);
    assert_eq!(pet.pet_class, 0);
    assert_eq!(pet.level, 5);

    let bytes = pet.para_bytes();
    assert_eq!(bytes.len(), TAMANHO_INFO_PET);
}
