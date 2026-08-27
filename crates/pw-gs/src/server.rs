use crate::world::WorldInstance;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

pub struct GameServer {
    pub world: Arc<RwLock<WorldInstance>>,
}

impl GameServer {
    pub fn new(world: WorldInstance) -> Self {
        Self {
            world: Arc::new(RwLock::new(world)),
        }
    }

    /// Inicia o Loop em Tempo Real de 50ms (20 Ticks por Segundo / TPS)
    pub async fn run_tick_loop(self: Arc<Self>) {
        info!("Iniciando World Server Tick Loop (50ms / 20 TPS)...");
        let mut interval = tokio::time::interval(Duration::from_millis(50));

        loop {
            interval.tick().await;
            let mut world = self.world.write().await;
            world.tick(50).await;
        }
    }
}
