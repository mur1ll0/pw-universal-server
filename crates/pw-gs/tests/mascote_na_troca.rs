//! B358 — o mascote de combate volta depois da troca de mapa (`pet_manager::PostSwitchServer`, `gs/petman.cpp:1857-1866`;
//! `combat_petdata_imp::PostSwitchServer`, `:844-870`): sai do mapa de origem com o dono e é invocado de novo no destino.
//!
//! Sem `TEST_DATABASE_URL` (ou sem `data/realm_155/config`) o teste passa sem verificar nada e diz isso na saída.

use pw_bus::{BusClient, BusListener, BusMessage};
use pw_core::{CharacterClass, Gender, Race};
use pw_data_loader::GameDataManager;
use pw_gs::{RoteadorDeMapas, WorldInstance};
use pw_protocol::GameVersion;
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use std::time::Duration;

#[path = "../../pw-storage/tests/comum/mod.rs"]
mod comum;

async fn ate<F, Fut>(mut cond: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..500 {
        if cond().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test]
async fn o_mascote_de_combate_volta_no_mapa_novo() {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return;
        }
    };
    let pasta = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !pasta.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", pasta.display());
        return;
    }
    let cfg = StorageConfig { database_url: url, max_connections: 3, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");
    comum::limpar_sobras_de_teste(&pool).await;
    let m = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;
    let realm = format!("t_gs_{m}");
    sqlx::query("INSERT INTO realms (id, name, version, host, port, max_players, config) VALUES ($1, 'Teste Mascote', '1.5.5', '127.0.0.1', 29000, 10, '{}'::jsonb)")
        .bind(&realm)
        .execute(pool.get_ref())
        .await
        .expect("criar realm");
    let conta: i32 = sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
        .bind(format!("gs_{m}"))
        .fetch_one(pool.get_ref())
        .await
        .expect("criar conta");
    let repo = CharacterRepository::new(pool.clone());
    let roleid = repo
        .create_character(conta, &realm, &format!("Dono{m}"), Race::Human, CharacterClass::Venomancer, Gender::Female, Vec::new(), None)
        .await
        .expect("criar personagem");
    sqlx::query("UPDATE characters SET world_id = 1, level = 40 WHERE id = $1").bind(roleid).execute(pool.get_ref()).await.unwrap();
    let mut info = pw_core::InfoPet::default();
    info.pet_tid = 10386;
    info.pet_class = pw_core::PET_CLASS_COMBAT;
    info.level = 30;
    info.hp_factor = 1.0;
    info.honor_point = 200;
    repo.item_repo()
        .upsert_item(&pw_core::ItemRecord {
            id: None,
            character_id: roleid,
            container_type: pw_core::ContainerType::PetCorral,
            slot: 0,
            item_id: 10386,
            count: 1,
            max_count: 1,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 0,
            max_durability: 0,
            bind_status: 0,
            octets: info.para_bytes(),
            custom_attributes: serde_json::json!({}),
        })
        .await
        .expect("guardar o mascote");

    let mut dados = GameDataManager::new();
    dados.load_from_directory(&pasta);
    let dados = Arc::new(dados);
    let mut servidos = Vec::new();
    for mapa in [1, 161] {
        let mundo = WorldInstance::new(mapa, Arc::clone(&dados), repo.clone());
        servidos.push(RoteadorDeMapas::preparar_mapa(mundo, GameVersion::V1_5_5).await);
    }
    let (mundo_1, mundo_161) = (Arc::clone(servidos[0].1.mundo()), Arc::clone(servidos[1].1.mundo()));
    let roteador = Arc::new(RoteadorDeMapas::new(servidos, repo));
    roteador.permitir_carga(pw_gs::mapas::CargaDeMapas { dados, versao: GameVersion::V1_5_5, pasta: None, falhas_na_partida: vec![] });
    roteador.ligar_trocas();
    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();
    tokio::spawn(Arc::clone(&roteador).executar(escuta));
    let mut link = BusClient::conectar(addr).await.unwrap();
    link.enviar(BusMessage::EnterWorld { roleid, provider_link_id: 1, locktime: 0, timeout: 60, settime: 0, localsid: 7 }).await.unwrap();
    let m1 = Arc::clone(&mundo_1);
    assert!(ate(|| { let m1 = Arc::clone(&m1); async move { m1.read().await.players.contains_key(&(roleid as i64)) } }).await);
    let id = mundo_1.write().await.invocar_mascote(roleid, 0, info).expect("o mascote sai no mapa 1");
    assert!(mundo_1.read().await.mascotes.contains_key(&id));

    roteador.transportar(roleid, 161, pw_core::Vector3::new(10.0, 0.0, 10.0)).await;
    let m161 = Arc::clone(&mundo_161);
    assert!(
        ate(|| {
            let m = Arc::clone(&m161);
            async move { m.read().await.mascotes.values().any(|p| p.dono == roleid as i64) }
        })
        .await,
        "o mascote não voltou no mapa 161"
    );
    assert!(!mundo_1.read().await.mascotes.values().any(|p| p.dono == roleid as i64), "e saiu do mapa 1");
}
