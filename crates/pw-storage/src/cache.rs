use crate::config::StorageConfig;
use crate::error::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tracing::info;

#[derive(Clone)]
pub struct CacheManager {
    manager: ConnectionManager,
}

impl CacheManager {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        info!("Connecting to DragonflyDB / Redis cache...");
        let client = redis::Client::open(config.redis_url.clone())?;
        let manager = ConnectionManager::new(client).await?;
        info!("DragonflyDB / Redis connection initialized successfully.");
        Ok(Self { manager })
    }

    /// Registra sessão ativa de jogador online
    pub async fn set_player_session(
        &self,
        realm_id: &str,
        role_id: i32,
        account_id: i32,
        ttl_seconds: u64,
    ) -> Result<()> {
        let mut conn = self.manager.clone();
        let key = format!("session:{}:{}", realm_id, role_id);
        let value = format!("{}:{}", account_id, chrono::Utc::now().timestamp());
        conn.set_ex::<_, _, ()>(key, value, ttl_seconds).await?;
        
        // Adiciona ao Set de jogadores online do Realm
        let set_key = format!("online:{}", realm_id);
        conn.sadd::<_, _, ()>(set_key, role_id).await?;
        Ok(())
    }

    /// Remove sessão do jogador ao deslogar
    pub async fn remove_player_session(&self, realm_id: &str, role_id: i32) -> Result<()> {
        let mut conn = self.manager.clone();
        let key = format!("session:{}:{}", realm_id, role_id);
        conn.del::<_, ()>(key).await?;
        
        let set_key = format!("online:{}", realm_id);
        conn.srem::<_, _, ()>(set_key, role_id).await?;
        Ok(())
    }

    /// Retorna contagem de jogadores online em um Realm
    pub async fn get_online_count(&self, realm_id: &str) -> Result<usize> {
        let mut conn = self.manager.clone();
        let set_key = format!("online:{}", realm_id);
        let count: usize = conn.scard(set_key).await?;
        Ok(count)
    }

    /// Publica evento de transmissão global ou de chat (Pub/Sub)
    pub async fn publish_event(&self, channel: &str, payload: &str) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;
        Ok(())
    }

    /// Armazena dados de sessão / ticket com TTL
    pub async fn set_session(&self, key: &str, payload: &str, ttl_seconds: u64) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.set_ex::<_, _, ()>(key, payload, ttl_seconds).await?;
        Ok(())
    }

    /// Busca dados de sessão / ticket
    pub async fn get_session(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.manager.clone();
        let val: Option<String> = conn.get(key).await?;
        Ok(val)
    }
}
