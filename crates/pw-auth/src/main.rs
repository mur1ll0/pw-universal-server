mod server;
mod service;

use pw_storage::{AccountRepository, CacheManager, PostgresPool, StorageConfig};
use server::AuthServer;
use service::AuthService;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("Iniciando microsserviço pw-auth (Autenticação Global)...");

    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;
    let cache_manager = CacheManager::new(&storage_config).await?;

    let account_repo = AccountRepository::new(pg_pool);
    let auth_service = AuthService::new(account_repo, cache_manager);

    let listen_port = std::env::var("LISTEN_PORT")
        .unwrap_or_else(|_| "29200".to_string())
        .parse::<u16>()
        .unwrap_or(29200);

    let server = AuthServer::new(auth_service, listen_port);
    server.run().await?;

    Ok(())
}
