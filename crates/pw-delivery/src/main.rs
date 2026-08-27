mod chat;
mod friends;
mod mail;
mod party;
mod server;
mod service;

use pw_storage::{MailRepository, PostgresPool, StorageConfig, CacheManager};
use server::DeliveryServer;
use service::DeliveryService;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let realm_id = std::env::var("REALM_ID").unwrap_or_else(|_| "realm_126".to_string());
    let listen_port = std::env::var("DELIVERY_PORT")
        .unwrap_or_else(|_| "29100".to_string())
        .parse::<u16>()
        .unwrap_or(29100);

    info!(
        "Iniciando pw-delivery para o Realm '{}' na porta {}...",
        realm_id, listen_port
    );

    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;
    let cache_manager = CacheManager::new(&storage_config).await?;

    let mail_repo = MailRepository::new(pg_pool);
    let delivery_service = DeliveryService::new(realm_id, mail_repo, cache_manager);

    let server = DeliveryServer::new(delivery_service, listen_port);
    server.run().await?;

    Ok(())
}
