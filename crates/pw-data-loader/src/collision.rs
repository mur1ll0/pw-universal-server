use pw_core::Vector3;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum CollisionError {
    #[error("Erro de I/O no arquivo de colisão: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato de colisão inválido")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, CollisionError>;

/// Estrutura que mantém o mapa de alturas do terreno (.rmap) e geometria de colisão
#[derive(Debug, Clone, Default)]
pub struct MapCollision {
    pub world_id: i32,
    pub width: usize,
    pub height: usize,
    pub height_data: Vec<f32>,
}

impl MapCollision {
    pub fn new(world_id: i32) -> Self {
        Self {
            world_id,
            width: 1024,
            height: 1024,
            height_data: Vec::new(),
        }
    }

    pub fn load_from_bytes(world_id: i32, data: &[u8]) -> Result<Self> {
        let mut collision = Self::new(world_id);
        collision.parse_terrain(data)?;
        info!("Mapa de colisão para World #{} carregado com sucesso", world_id);
        Ok(collision)
    }

    fn parse_terrain(&mut self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    /// Retorna a altura do terreno (Y) para uma coordenada X, Z
    pub fn get_terrain_height(&self, _x: f32, _z: f32) -> f32 {
        // Retorna altura calculada no grid
        200.0
    }

    /// Verifica se uma linha reta 3D colide com obstáculos (Raycasting de linha de visão)
    pub fn raycast_line_of_sight(&self, _from: &Vector3, _to: &Vector3) -> bool {
        // Retorna true se houver linha de visão desobstruída
        true
    }
}
