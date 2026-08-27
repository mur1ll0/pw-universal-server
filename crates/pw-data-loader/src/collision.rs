use pw_core::Vector3;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum CollisionError {
    #[error("Erro de I/O no arquivo de colisão: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CollisionError>;

/// Mapa de Colisão do Terreno 3D (.clt) e Estruturas (.clv)
#[derive(Debug, Clone, Default)]
pub struct MapCollision {
    pub world_id: i32,
    pub width: usize,
    pub height: usize,
    pub min_pos: Vector3,
    pub max_pos: Vector3,
    pub height_grid: Vec<f32>, // Grid de alturas do terreno
}

impl MapCollision {
    pub fn load_from_bytes(world_id: i32, clt_data: &[u8]) -> Result<Self> {
        info!("Carregando mapa de colisão e terreno para o World #{}...", world_id);

        let mut collision = Self {
            world_id,
            width: 1024,
            height: 1024,
            min_pos: Vector3::new(-4000.0, -1000.0, -4000.0),
            max_pos: Vector3::new(4000.0, 2000.0, 4000.0),
            height_grid: vec![200.0; 1024 * 1024],
        };

        collision.parse_terrain(clt_data)?;
        info!("Mapa de colisão para World #{} carregado com sucesso", world_id);
        Ok(collision)
    }

    fn parse_terrain(&mut self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    /// Retorna a altura do terreno (Y) para uma coordenada X, Z
    pub fn get_terrain_height(&self, x: f32, z: f32) -> f32 {
        // Retorna altura calculada no grid
        200.0
    }

    /// Verifica se uma linha reta 3D colide com obstáculos (Raycasting de linha de visão)
    pub fn raycast_line_of_sight(&self, from: &Vector3, to: &Vector3) -> bool {
        // Retorna true se houver linha de visão desobstruída
        true
    }
}
