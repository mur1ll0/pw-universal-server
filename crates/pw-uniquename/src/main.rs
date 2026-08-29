use pw_storage::{PostgresPool, StorageConfig};
use pw_uniquename::{UniqueNameServer, UniqueNameService};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let listen_port = std::env::var("UNIQUENAME_PORT")
        .unwrap_or_else(|_| "29500".to_string())
        .parse::<u16>()
        .unwrap_or(29500);

    info!("Iniciando pw-uniquename na porta {}...", listen_port);

    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;
    let service = UniqueNameService::new(pg_pool);

    let server = UniqueNameServer::new(service, listen_port);
    server.run().await?;

    Ok(())
}
