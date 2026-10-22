//! B343 — NPC e mina invocados pela política (`CreateNPC`/`CreateMine`, `obj_interface.cpp:2095-2260`):
//! nascem perto do alvo, só com modelo do tipo certo, e somem no fim do prazo.

use pw_core::Vector3;
use pw_data_loader::GameDataManager;
use pw_gs::WorldInstance;
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;

#[tokio::test]
async fn npc_e_mina_invocados_nascem_perto_e_somem_no_prazo() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else { return };
    let pasta = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !pasta.join("elements.data").exists() {
        return;
    }
    let mut dados = GameDataManager::new();
    dados.load_from_directory(&pasta);
    let npc = *dados.ids_de_npc.iter().min().expect("um NPC");
    let mina = *dados.minas.keys().min().expect("uma mina");
    let monstro = dados.monstros.templates.keys().copied().min().expect("um monstro");
    let cfg = StorageConfig { database_url: url, max_connections: 2, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("banco");
    let mut mundo = WorldInstance::new(1, Arc::new(dados), CharacterRepository::new(pool));

    let centro = Vector3::new(100.0, 0.0, 100.0);
    let n = mundo.invocar_npc(npc, centro, 3.0, 2).expect("NPC invocado");
    let m = mundo.invocar_mina(mina, centro, 3.0, 0).expect("mina invocada");
    let p = mundo.npcs[&n].position;
    assert!((p.x - 100.0).abs() <= 3.0 && (p.z - 100.0).abs() <= 3.0, "x/z ± raio: {p:?}");
    assert!(mundo.matters.contains_key(&m));
    assert!(mundo.invocar_npc(monstro, centro, 0.0, 0).is_none(), "monstro não é NPC");
    assert!(mundo.invocar_mina(npc, centro, 0.0, 0).is_none(), "NPC não é mina");

    mundo.descontar_prazos_de_invocados(1_999);
    assert!(mundo.npcs.contains_key(&n), "ainda no prazo");
    mundo.descontar_prazos_de_invocados(1);
    assert!(!mundo.npcs.contains_key(&n), "2 s depois, sumiu");
    assert!(mundo.matters.contains_key(&m), "vida 0 é para sempre");
}
