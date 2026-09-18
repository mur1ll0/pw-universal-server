use crate::config::StorageConfig;
use crate::error::Result;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

#[derive(Clone)]
pub struct PostgresPool {
    pool: PgPool,
}

impl PostgresPool {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        info!("Connecting to PostgreSQL database...");

        let mut connect_options = PgConnectOptions::from_str(&config.database_url)?;

        // Quando executando sob o ambiente de testes (TEST_DATABASE_URL),
        // isolamos todas as tabelas no schema `test` antes do `public`.
        if std::env::var("TEST_DATABASE_URL").is_ok() {
            connect_options = connect_options.options([("search_path", "test,public")]);
        }

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout_sec))
            .connect_with(connect_options)
            .await?;

        info!("PostgreSQL connection pool initialized successfully.");
        Ok(Self { pool })
    }

    pub fn get_ref(&self) -> &PgPool {
        &self.pool
    }

    pub fn inner(&self) -> PgPool {
        self.pool.clone()
    }
}
