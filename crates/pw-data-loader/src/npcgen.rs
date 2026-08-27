use pw_core::Vector3;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum NpcGenError {
    #[error("Erro de I/O na leitura do npcgen.data: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NpcGenError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnType {
    Monster,
    Npc,
    ResourceMine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnArea {
    pub world_id: i32,
    pub area_name: String,
    pub spawn_type: SpawnType,
    pub template_id: u32,  // ID do Monstro, NPC ou Mina do elements.data
    pub count: u32,
    pub center: Vector3,
    pub radius: f32,
    pub respawn_sec: u32,
    pub patrol_path_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct NpcGenData {
    pub areas: Vec<SpawnArea>,
}

impl NpcGenData {
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        info!("Carregando npcgen.data (Pontos de Spawn de Monstros e NPCs)...");

        let mut npcgen_data = Self {
            areas: Vec::new(),
        };

        npcgen_data.parse_areas(&mut cursor)?;
        info!("npcgen.data carregado: {} zonas de spawn registradas", npcgen_data.areas.len());

        Ok(npcgen_data)
    }

    fn parse_areas(&mut self, _cursor: &mut Cursor<&[u8]>) -> Result<()> {
        Ok(())
    }
}
