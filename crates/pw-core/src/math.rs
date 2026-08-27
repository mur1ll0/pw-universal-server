use serde::{Deserialize, Serialize};

/// Vetor tridimensional para posições e física no mundo 3D
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Distância euclidiana ao quadrado entre dois pontos
    pub fn distance_squared(&self, other: &Vector3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Distância euclidiana 3D
    pub fn distance(&self, other: &Vector3) -> f32 {
        self.distance_squared(other).sqrt()
    }

    /// Distância 2D no plano horizontal (X-Z)
    pub fn distance_2d(&self, other: &Vector3) -> f32 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }

    /// Ponto padrão na Cidade do Dragão (CDD - 550, 200, 650)
    pub const fn dragon_city() -> Self {
        Self::new(550.0, 200.0, 650.0)
    }
}
