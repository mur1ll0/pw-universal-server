mod gateway;
mod session;

use gateway::LinkGateway;
use pw_storage::{AccountRepository, CacheManager, CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let realm_id = std::env::var("REALM_ID").unwrap_or_else(|_| "realm_126".to_string());
    let game_version = std::env::var("GAME_VERSION").unwrap_or_else(|_| "1.2.6".to_string());
    let listen_port = std::env::var("GATEWAY_PORT")
        .unwrap_or_else(|_| "29000".to_string())
        .parse::<u16>()
        .unwrap_or(29000);

    info!(
        "Iniciando pw-link para o Realm '{}' na versão '{}' (Porta: {})...",
        realm_id, game_version, listen_port
    );

    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;
    let cache_manager = CacheManager::new(&storage_config).await?;

    let account_repo = AccountRepository::new(pg_pool.clone());
    let char_repo = CharacterRepository::new(pg_pool);

    let gateway = Arc::new(LinkGateway::new(
        realm_id,
        &game_version,
        listen_port,
        account_repo,
        char_repo,
        cache_manager,
    ));

    gateway.run().await?;

    Ok(())
}
