//! Os destinos das transportadoras do `realm_155`: `NPC_TRANSMIT_SERVICE` + `world_targets.sev`.

use std::path::PathBuf;

fn realm() -> Option<pw_data_loader::GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(&dir);
    Some(d)
}

/// B68 — o `world_targets.sev` fecha no último byte e tem os pontos que as transportadoras
/// citam. O arquivo do `realm_155`: 92 pontos, 2.212 bytes (`4 + 92 × 24`).
#[test]
fn o_world_targets_fecha_e_tem_os_destinos_citados() {
    let Some(d) = realm() else { return };
    assert_eq!(d.pontos_do_mundo.len(), 92, "pontos lidos do world_targets.sev");

    // O primeiro ponto do arquivo, conferido byte a byte.
    let p = d.pontos_do_mundo.get(1001).expect("ponto 1001");
    assert_eq!(p.mundo, 1);
    assert!((p.pos[0] - (-2545.362)).abs() < 0.01, "x {}", p.pos[0]);
    assert!((p.pos[2] - 4116.355).abs() < 0.01, "z {}", p.pos[2]);

    // **Todo** destino citado por transportadora tem coordenada — se faltasse uma, o
    // teleporte mandaria o jogador para lugar nenhum.
    let mut destinos = 0;
    let mut npcs = 0;
    for s in d.servicos_de_npc.values() {
        if s.destinos.is_empty() {
            continue;
        }
        npcs += 1;
        for t in &s.destinos {
            destinos += 1;
            assert!(
                d.pontos_do_mundo.get(t.id_ponto).is_some(),
                "o destino {} não está no world_targets.sev",
                t.id_ponto
            );
        }
    }
    eprintln!("{npcs} transportadoras, {destinos} destinos");
    assert!(npcs > 10, "poucas transportadoras: {npcs}");
    assert!(destinos > 50, "poucos destinos: {destinos}");
}
