mod server;
mod service;

use pw_storage::{PostgresPool, StorageConfig};
use server::UniqueNameServer;
use service::UniqueNameService;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("Iniciando microsserviço pw-uniquename (Garantia de Nomes Únicos)...");

    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;

    let unique_service = UniqueNameService::new(pg_pool);

    let listen_port = std::env::var("LISTEN_PORT")
        .unwrap_or_else(|_| "29300".to_string())
        .parse::<u16>()
        .unwrap_or(29300);

    let server = UniqueNameServer::new(unique_service, listen_port);
    server.run().await?;

    Ok(())
}
