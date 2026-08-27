use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub database_url: String,
    pub redis_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_sec: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://pw_admin:pw_secure_password_2026@localhost:5432/pw_database".to_string()
            }),
            redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| {
                "redis://localhost:6379".to_string()
            }),
            max_connections: 50,
            min_connections: 5,
            connection_timeout_sec: 10,
        }
    }
}
