use pw_data_loader::GameDataManager;
use pw_gs::{GameServer, WorldInstance};
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let realm_id = std::env::var("REALM_ID").unwrap_or_else(|_| "realm_126".to_string());
    let game_version = std::env::var("GAME_VERSION").unwrap_or_else(|_| "1.2.6".to_string());
    let world_tag = std::env::var("WORLD_TAG")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<i32>()
        .unwrap_or(1);
    let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| "./data/config".to_string());

    info!(
        "Iniciando pw-gs (World Server) para o Realm '{}' (v{}), World #{}...",
        realm_id, game_version, world_tag
    );

    // 1. Carrega dados e templates de jogo
    let mut data_manager = GameDataManager::new();
    let _ = data_manager.load_from_directory(&config_dir);
    let data_manager = Arc::new(data_manager);

    // 2. Conecta ao banco de dados PostgreSQL
    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;
    let char_repo = CharacterRepository::new(pg_pool);

    // 3. Inicializa o Mundo de Jogo e os spawns
    let mut world = WorldInstance::new(world_tag, data_manager, char_repo);
    world.init_spawns();

    let server = Arc::new(GameServer::new(world));

    // 4. Executa o loop de simulação em tempo real
    server.run_tick_loop().await;

    Ok(())
}
