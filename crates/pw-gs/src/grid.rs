use pw_core::Vector3;
use std::collections::{HashMap, HashSet};

/// Identificador de entidade no mundo (jogadores usam IDs positivos, monstros/npcs usam IDs negativos ou sequenciais)
pub type EntityId = i64;

/// Tamanho de cada célula do Grid espacial (50 metros)
pub const CELL_SIZE: f32 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i32,
    pub z: i32,
}

impl GridCoord {
    pub fn from_vector3(pos: &Vector3) -> Self {
        Self {
            x: (pos.x / CELL_SIZE).floor() as i32,
            z: (pos.z / CELL_SIZE).floor() as i32,
        }
    }
}

/// Grid Espacial 2D/3D para particionamento de visibilidade e cálculo de AOI (Area of Interest)
#[derive(Debug, Clone, Default)]
pub struct SpatialGrid {
    cells: HashMap<GridCoord, HashSet<EntityId>>,
    entity_positions: HashMap<EntityId, (Vector3, bool)>, // (Posição, is_player)
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona uma entidade ao grid espacial
    pub fn add_entity(&mut self, id: EntityId, pos: Vector3, is_player: bool) {
        let coord = GridCoord::from_vector3(&pos);
        self.cells.entry(coord).or_default().insert(id);
        self.entity_positions.insert(id, (pos, is_player));
    }

    /// Atualiza a posição de uma entidade, movendo entre células do grid se necessário
    pub fn update_position(&mut self, id: EntityId, new_pos: Vector3) -> bool {
        if let Some((old_pos, _)) = self.entity_positions.get_mut(&id) {
            let old_coord = GridCoord::from_vector3(old_pos);
            let new_coord = GridCoord::from_vector3(&new_pos);

            *old_pos = new_pos;

            if old_coord != new_coord {
                if let Some(cell) = self.cells.get_mut(&old_coord) {
                    cell.remove(&id);
                }
                self.cells.entry(new_coord).or_default().insert(id);
                return true; // Mudou de célula
            }
        }
        false
    }

    /// Remove uma entidade do grid
    pub fn remove_entity(&mut self, id: EntityId) {
        if let Some((pos, _)) = self.entity_positions.remove(&id) {
            let coord = GridCoord::from_vector3(&pos);
            if let Some(cell) = self.cells.get_mut(&coord) {
                cell.remove(&id);
            }
        }
    }

    /// Retorna todas as entidades dentro do raio de visão a partir de um ponto central
    pub fn get_entities_in_range(&self, center: &Vector3, radius: f32) -> Vec<EntityId> {
        let mut results = Vec::new();
        let radius_sq = radius * radius;

        let min_x = ((center.x - radius) / CELL_SIZE).floor() as i32;
        let max_x = ((center.x + radius) / CELL_SIZE).floor() as i32;
        let min_z = ((center.z - radius) / CELL_SIZE).floor() as i32;
        let max_z = ((center.z + radius) / CELL_SIZE).floor() as i32;

        for x in min_x..=max_x {
            for z in min_z..=max_z {
                let coord = GridCoord { x, z };
                if let Some(cell) = self.cells.get(&coord) {
                    for &id in cell {
                        if let Some((pos, _)) = self.entity_positions.get(&id) {
                            if center.distance_squared(pos) <= radius_sq {
                                results.push(id);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Retorna apenas os jogadores dentro do raio de visão (para transmissões de pacotes)
    pub fn get_players_in_range(&self, center: &Vector3, radius: f32) -> Vec<EntityId> {
        let entities = self.get_entities_in_range(center, radius);
        entities
            .into_iter()
            .filter(|id| {
                if let Some((_, is_player)) = self.entity_positions.get(id) {
                    *is_player
                } else {
                    false
                }
            })
            .collect()
    }
}
