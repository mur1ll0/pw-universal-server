use pw_data_loader::GameDataManager;
use std::path::Path;

#[test]
fn test_carregamento_pets_missoes_e_npcs_servico() {
    let path = Path::new("../../data/realm_155/config");
    if !path.exists() {
        return;
    }
    let mut dm = GameDataManager::new();
    let rel = dm.load_from_directory(path);
    assert!(!rel.lidos.is_empty());

    // 1. Itens de missão devem conter TASKMATTER_ESSENCE
    assert!(!dm.itens_de_missao.is_empty(), "itens_de_missao não deve estar vazio");
    assert!(dm.e_item_de_missao(2106), "item 2106 deve ser item de missão");

    // 2. Ovos de mascote carregados
    assert!(!dm.ovos_de_pet.is_empty(), "ovos_de_pet não deve estar vazio");
    let montaria = dm.ovos_de_pet.values().find(|o| o.pet_class == 0).expect("deve haver ovo de montaria");
    let octetos = dm.gerar_octetos_do_ovo(montaria.id).expect("deve gerar octetos do ovo de montaria");
    assert!(octetos.len() >= 60, "octetos do ovo devem ter pelo menos 60 bytes");
    let ess = pw_core::PeEssence::de_bytes(&octetos).expect("deve reler a essencia gerada");
    assert!((ess.require_class & (1 << 8)) != 0, "arqueiro (classe 8) deve ser permitido em ovo de montaria: {:#X}", ess.require_class);

    if let Some(combate) = dm.ovos_de_pet.values().find(|o| o.pet_class == 1) {
        let octetos = dm.gerar_octetos_do_ovo(combate.id).expect("deve gerar octetos do ovo de combate");
        let ess = pw_core::PeEssence::de_bytes(&octetos).expect("deve reler a essencia gerada");
        assert_eq!(ess.require_class & (1 << 8), 0, "arqueiro NÃO deve poder usar pet de combate: {:#X}", ess.require_class);
    }

    // 3. NPCs de serviço ilimitado de cena
    assert!(!dm.scene_service_npcs.is_empty(), "scene_service_npcs não deve estar vazio");
    assert_eq!(dm.scene_service_npcs.len(), 15, "deve ter os 15 NPCs de serviço ilimitado");
    let tids: Vec<i32> = dm.scene_service_npcs.iter().map(|(t, _)| *t).collect();
    assert!(tids.contains(&44680), "deve conter o mestre de arqueiro 44680");

    // 4. Missão 31711 (Evocação Próxima) deve conter o monstro 44617 (Macaco Bêbado) em monstros_invocados
    let t31711 = dm.tasks.tasks.get(&31711).expect("task 31711 deve existir");
    let inv = t31711.rewards.monstros_invocados.as_ref().expect("task 31711 deve ter monstros_invocados");
    assert!(!inv.monstros.is_empty(), "deve conter monstros para evocar");
    assert_eq!(inv.monstros[0].monstro, 44617, "deve invocar o Macaco Bêbado (44617)");
    assert_eq!(inv.monstros[0].quantidade, 1, "quantidade deve ser 1");
    println!("Task 31711 invocação confirmada com sucesso: {:?}", inv);
}
