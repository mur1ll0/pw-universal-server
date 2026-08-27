use crate::config::StorageConfig;
use crate::error::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

#[derive(Clone)]
pub struct PostgresPool {
    pool: PgPool,
}

impl PostgresPool {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        info!("Connecting to PostgreSQL database...");
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout_sec))
            .connect(&config.database_url)
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
