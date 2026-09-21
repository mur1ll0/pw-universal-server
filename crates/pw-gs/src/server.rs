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
            // O tique tranca o mundo inteiro. Nada que espere o banco pode acontecer aqui
            // dentro: o autosave sai daqui como fotografia e é gravado com o lock já solto,
            // numa tarefa à parte (B72).
            let (lote, repo, mundo) = {
                let mut world = self.world.write().await;
                let lote = world.tick(50).await;
                (lote, world.char_repo.clone(), world.world_id)
            };
            if !lote.is_empty() {
                tokio::spawn(crate::world::gravar_autosave(repo, lote, mundo));
            }
        }
    }
}
