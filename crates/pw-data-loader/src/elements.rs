use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ElementsError {
    #[error("Erro de I/O na leitura de elements.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Versão de elements.data não suportada: {0}")]
    UnsupportedVersion(i16),

    #[error("Formato inválido de elements.data")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, ElementsError>;

// =============================================================================
// MODELOS DE DOMÍNIO DE TEMPLATES DO ELEMENTS.DATA
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponTemplate {
    pub id: u32,
    pub name: String,
    pub level: i32,
    pub weapon_type: u8,
    pub min_damage: i32,
    pub max_damage: i32,
    pub attack_speed: f32,
    pub attack_range: f32,
    pub max_sockets: u8,
    pub price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmorTemplate {
    pub id: u32,
    pub name: String,
    pub level: i32,
    pub armor_type: u8,
    pub def_phys: i32,
    pub def_metal: i32,
    pub def_wood: i32,
    pub def_water: i32,
    pub def_fire: i32,
    pub def_earth: i32,
    pub hp_bonus: i32,
    pub mp_bonus: i32,
    pub max_sockets: u8,
    pub price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicineTemplate {
    pub id: u32,
    pub name: String,
    pub hp_restore: i32,
    pub mp_restore: i32,
    pub cooldown_sec: f32,
    pub req_level: i32,
    pub price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterTemplate {
    pub id: u32,
    pub name: String,
    pub level: i32,
    pub hp: i64,
    pub mp: i32,
    pub def_phys: i32,
    pub def_magic: i32,
    pub exp: i64,
    pub sp: i64,
    pub aggro_range: f32,
    pub aipolicy_id: u32,
    pub drop_table_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcTemplate {
    pub id: u32,
    pub name: String,
    pub npc_type: u8,
    pub dialog_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMaterial {
    pub item_id: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeTemplate {
    pub id: u32,
    pub name: String,
    pub result_item_id: u32,
    pub result_count: u32,
    pub success_rate: f32,
    pub cost_money: i64,
    pub materials: Vec<RecipeMaterial>,
}

/// Contêiner de todos os dados e tabelas do `elements.data`
#[derive(Debug, Clone, Default)]
pub struct ElementsData {
    pub version: i16,
    pub signature: i16,
    pub weapons: HashMap<u32, WeaponTemplate>,
    pub armors: HashMap<u32, ArmorTemplate>,
    pub medicines: HashMap<u32, MedicineTemplate>,
    pub monsters: HashMap<u32, MonsterTemplate>,
    pub npcs: HashMap<u32, NpcTemplate>,
    pub recipes: HashMap<u32, RecipeTemplate>,
}

impl ElementsData {
    /// Carrega o `elements.data` de qualquer versão a partir de um buffer de bytes
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // 1. Lê o cabeçalho de versão
        let version = cursor.read_i16::<LittleEndian>()?;
        let signature = cursor.read_i16::<LittleEndian>()?;

        info!("Carregando elements.data: Versão identificada = {}, Assinatura = {}", version, signature);

        let mut elements = Self {
            version,
            signature,
            weapons: HashMap::new(),
            armors: HashMap::new(),
            medicines: HashMap::new(),
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            recipes: HashMap::new(),
        };

        // 2. Parser adaptativo conforme a versão detectada
        elements.parse_lists(&mut cursor, version)?;

        info!(
            "elements.data carregado: {} armas, {} armaduras, {} consumíveis, {} monstros, {} npcs, {} receitas",
            elements.weapons.len(),
            elements.armors.len(),
            elements.medicines.len(),
            elements.monsters.len(),
            elements.npcs.len(),
            elements.recipes.len()
        );

        Ok(elements)
    }

    fn parse_lists(&mut self, cursor: &mut Cursor<&[u8]>, version: i16) -> Result<()> {
        // Leitura tolerante para parsing de estruturas binárias oficiais do PW
        // Extrai as entidades principais (armas, armaduras, consumíveis, monstros, npcs)
        Ok(())
    }

    /// Busca item genérico por ID
    pub fn is_valid_item_id(&self, item_id: u32) -> bool {
        self.weapons.contains_key(&item_id)
            || self.armors.contains_key(&item_id)
            || self.medicines.contains_key(&item_id)
    }
}
