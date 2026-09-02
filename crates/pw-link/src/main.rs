use pw_link::LinkGateway;
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
    let char_repo = CharacterRepository::new(pg_pool.clone());
    let template_repo = pw_storage::TemplateRepository::new(pg_pool.clone());

    if let Err(e) = template_repo.ensure_default_templates(&realm_id).await {
        tracing::warn!("Aviso ao carregar templates de classes padrão: {:?}", e);
    }

    let mut gateway = LinkGateway::new(
        realm_id,
        &game_version,
        listen_port,
        account_repo,
        char_repo,
        cache_manager,
    );

    // `GS_BUS` é o endereço do servidor de mundo deste realm (`host:porta`). Sem ele o
    // link roda sozinho, como antes — o que é o modo de desenvolvimento, e não o de
    // produção. O aviso existe porque um `GS_BUS` esquecido no `docker-compose` daria um
    // servidor que sobe inteiro e não simula nada, sem um único erro no log.
    match std::env::var("GS_BUS") {
        Ok(endereco) if !endereco.trim().is_empty() => {
            gateway = gateway.com_barramento(endereco.trim());
        }
        _ => {
            tracing::warn!(
                "GS_BUS não definido: este link não está ligado a nenhum servidor de \
                 mundo. O cliente entra, mas o mundo 3D não é simulado."
            );
        }
    }

    Arc::new(gateway).run().await?;

    Ok(())
}
