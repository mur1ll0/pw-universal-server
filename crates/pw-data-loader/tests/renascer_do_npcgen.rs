//! B205: o `bAutoRevive` das áreas do `npcgen.data` (`SetRespawn`, `gs/npcgenerator.cpp:3734`):
//! na Caverna das Sombras (a69 do 1.5.5) nenhuma das 38 criaturas renasce — entre elas os 6
//! Cavaleiros Negros (45818) —, e o mundo aberto (a01) renasce todo.
use pw_data_loader::npcgen::{NpcGenData, SpawnInstance};

fn criaturas(pasta: &str) -> Option<Vec<SpawnInstance>> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/realm_155/config/{pasta}/npcgen.data"));
    let Ok(b) = std::fs::read(&p) else {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", p.display());
        return None;
    };
    let d = NpcGenData::load_from_bytes(&b).unwrap();
    Some(d.instances.iter().chain(d.controladores.values().flat_map(|c| c.pendentes.iter()))
        .filter(|i| matches!(i.spawn_type, pw_data_loader::SpawnType::Monster | pw_data_loader::SpawnType::Npc))
        .cloned().collect())
}

#[test]
fn a_caverna_das_sombras_nao_renasce_e_o_mundo_aberto_renasce() {
    let Some(caverna) = criaturas("a69") else { return };
    assert_eq!(caverna.len(), 38);
    assert!(caverna.iter().all(|i| !i.renasce), "a69: todas sem bAutoRevive");
    assert_eq!(caverna.iter().filter(|i| i.template_id == 45818).count(), 6, "os Cavaleiros Negros");
    let Some(mundo) = criaturas("a01") else { return };
    assert!(!mundo.is_empty() && mundo.iter().all(|i| i.renasce), "a01 renasce todo");
}
