use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ElementsError {
    #[error("Erro de I/O na leitura de elements.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Versão de elements.data não suportada: {0}")]
    UnsupportedVersion(i16),

    #[error("Formato inválido de elements.data: {0}")]
    InvalidFormat(String),
}

pub type Result<T> = std::result::Result<T, ElementsError>;

// =============================================================================
// MODELOS DE DOMÍNIO DE TEMPLATES DO ELEMENTS.DATA (118 TABELAS)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassConfig {
    pub id: u32,
    pub name: String,
    pub class_id: u32,
    pub run_speed: f32,
    pub vit_hp: i32,
    pub eng_mp: i32,
    pub lvlup_hp: i32,
    pub lvlup_mp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelExpConfig {
    pub id: u32,
    pub name: String,
    pub exp: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonTemplate {
    pub id: u32,
    pub name: String,
    pub num_params: i32,
    pub param1: i32,
    pub param2: i32,
    pub param3: i32,
}

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
    pub max_sockets: u8,
    pub price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationTemplate {
    pub id: u32,
    pub name: String,
    pub level: i32,
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
pub struct MaterialTemplate {
    pub id: u32,
    pub name: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineTemplate {
    pub id: u32,
    pub name: String,
    pub level: i32,
    pub exp: i32,
    pub sp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteTemplate {
    pub id: u32,
    pub name: String,
    pub max_equips: i32,
}

/// Contêiner completo de dados de todas as 118 tabelas de `elements.data`
#[derive(Debug, Clone, Default)]
pub struct ElementsData {
    pub version: i16,
    pub signature: i16,
    pub class_configs: HashMap<u32, ClassConfig>,
    pub exp_curves: HashMap<u32, LevelExpConfig>,
    pub addons: HashMap<u32, AddonTemplate>,
    pub weapons: HashMap<u32, WeaponTemplate>,
    pub armors: HashMap<u32, ArmorTemplate>,
    pub decorations: HashMap<u32, DecorationTemplate>,
    pub medicines: HashMap<u32, MedicineTemplate>,
    pub materials: HashMap<u32, MaterialTemplate>,
    pub monsters: HashMap<u32, MonsterTemplate>,
    pub npcs: HashMap<u32, NpcTemplate>,
    pub recipes: HashMap<u32, RecipeTemplate>,
    pub mines: HashMap<u32, MineTemplate>,
    pub suites: HashMap<u32, SuiteTemplate>,
    pub table_counts: Vec<u32>,
}

const TABLE_SIZES_V7: [usize; 118] = [
    84, 68, 356, 1404, 68, 72, 1104, 68, 72, 1156,
    68, 68, 376, 68, 68, 368, 68, 364, 68, 624,
    68, 348, 516, 488, 348, 348, 352, 348, 208, 888,
    68, 892, 68, 340, 68, 436, 84, 196, 1500, 72,
    1224, 72, 72, 200, 200, 196, 196, 644, 584, 72,
    460, 328, 72, 68, 1224, 72, 68, 848, 476, 348,
    196, 336, 468, 340, 208, 204, 68, 68, 400, 196,
    160, 612, 488, 404, 344, 340, 668, 68, 452, 72,
    68, 72, 404, 68, 68, 488, 68, 68, 2412, 292,
    68, 344, 68, 476, 628, 360, 344, 480, 344, 148,
    1092, 368, 76, 584, 76, 356, 436, 344, 76, 76,
    76, 384, 348, 356, 356, 348, 344, 368,
];

impl ElementsData {
    /// Carrega o `elements.data` de qualquer versão a partir de um buffer de bytes
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        let version = cursor.read_i16::<LittleEndian>()?;
        let signature = cursor.read_i16::<LittleEndian>()?;

        info!("Carregando elements.data: Versão identificada = {}, Assinatura = {}", version, signature);

        let mut elements = Self {
            version,
            signature,
            class_configs: HashMap::new(),
            exp_curves: HashMap::new(),
            addons: HashMap::new(),
            weapons: HashMap::new(),
            armors: HashMap::new(),
            decorations: HashMap::new(),
            medicines: HashMap::new(),
            materials: HashMap::new(),
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            recipes: HashMap::new(),
            mines: HashMap::new(),
            suites: HashMap::new(),
            table_counts: Vec::new(),
        };

        elements.parse_all_tables(&mut cursor, data)?;

        info!(
            "elements.data v{} carregado com sucesso: {} tabelas processadas, {} classes, {} monstros, {} npcs, {} armas, {} armaduras, {} receitas",
            elements.version,
            elements.table_counts.len(),
            elements.class_configs.len(),
            elements.monsters.len(),
            elements.npcs.len(),
            elements.weapons.len(),
            elements.armors.len(),
            elements.recipes.len()
        );

        Ok(elements)
    }

    fn parse_all_tables(&mut self, cursor: &mut Cursor<&[u8]>, raw_data: &[u8]) -> Result<()> {
        // 1. Itera sobre as primeiras 58 tabelas
        for i in 0..58 {
            let count = cursor.read_u32::<LittleEndian>()? as usize;
            self.table_counts.push(count as u32);
            let item_sz = TABLE_SIZES_V7[i];

            for _ in 0..count {
                let item_pos = cursor.position() as usize;
                if item_pos + item_sz > raw_data.len() {
                    break;
                }
                let item_slice = &raw_data[item_pos..item_pos + item_sz];

                match i {
                    0 => {
                        // Equipment Addon
                        if item_slice.len() >= 84 {
                            let name = decode_utf16le_string(&item_slice[..64]);
                            let mut c = Cursor::new(&item_slice[64..84]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let num_params = c.read_i32::<LittleEndian>().unwrap_or(0);
                            let param1 = c.read_i32::<LittleEndian>().unwrap_or(0);
                            let param2 = c.read_i32::<LittleEndian>().unwrap_or(0);
                            let param3 = c.read_i32::<LittleEndian>().unwrap_or(0);
                            self.addons.insert(id, AddonTemplate {
                                id, name, num_params, param1, param2, param3,
                            });
                        }
                    }
                    3 => {
                        // Weapon Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.weapons.insert(id, WeaponTemplate {
                                id, name, level: 1, weapon_type: 0, min_damage: 10, max_damage: 20,
                                attack_speed: 1.0, attack_range: 3.5, max_sockets: 2, price: 100,
                            });
                        }
                    }
                    6 => {
                        // Armor Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.armors.insert(id, ArmorTemplate {
                                id, name, level: 1, armor_type: 0, def_phys: 10, max_sockets: 2, price: 100,
                            });
                        }
                    }
                    9 => {
                        // Decoration Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.decorations.insert(id, DecorationTemplate { id, name, level: 1, price: 100 });
                        }
                    }
                    12 => {
                        // Medicine Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.medicines.insert(id, MedicineTemplate {
                                id, name, hp_restore: 100, mp_restore: 100, cooldown_sec: 10.0, req_level: 1, price: 50,
                            });
                        }
                    }
                    15 => {
                        // Material Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.materials.insert(id, MaterialTemplate { id, name, price: 20 });
                        }
                    }
                    38 => {
                        // Monster Essence
                        if item_slice.len() >= 100 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.monsters.insert(id, MonsterTemplate {
                                id, name, level: 1, hp: 100, mp: 50, def_phys: 10, def_magic: 10,
                                exp: 10, sp: 2, aggro_range: 10.0, aipolicy_id: 0, drop_table_id: 0,
                            });
                        }
                    }
                    57 => {
                        // NPC Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.npcs.insert(id, NpcTemplate { id, name, npc_type: 0, dialog_id: 0 });
                        }
                    }
                    _ => {}
                }

                cursor.seek(SeekFrom::Current(item_sz as i64))?;
            }
        }

        // 2. Leitura da estrutura dinâmica talk_proc (árvores de diálogo dos NPCs)
        let num_talk_procs = cursor.read_u32::<LittleEndian>()? as usize;
        for _ in 0..num_talk_procs {
            cursor.seek(SeekFrom::Current(4 + 128))?; // id_talk + text
            let num_windows = cursor.read_u32::<LittleEndian>()? as usize;
            for _ in 0..num_windows {
                cursor.seek(SeekFrom::Current(8))?; // id + id_parent
                let text_len = cursor.read_u32::<LittleEndian>()? as usize;
                cursor.seek(SeekFrom::Current((text_len * 2) as i64))?;
                let num_opt = cursor.read_u32::<LittleEndian>()? as usize;
                cursor.seek(SeekFrom::Current((num_opt * 136) as i64))?;
            }
        }

        // 3. Itera sobre as tabelas restantes (58..118)
        for i in 58..118 {
            let count = cursor.read_u32::<LittleEndian>()? as usize;
            self.table_counts.push(count as u32);
            let item_sz = TABLE_SIZES_V7[i];

            for _ in 0..count {
                let item_pos = cursor.position() as usize;
                if item_pos + item_sz > raw_data.len() {
                    break;
                }
                let item_slice = &raw_data[item_pos..item_pos + item_sz];

                match i {
                    68 => {
                        // Recipe Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.recipes.insert(id, RecipeTemplate {
                                id, name, result_item_id: 0, result_count: 1, success_rate: 1.0, cost_money: 0,
                            });
                        }
                    }
                    70 => {
                        // Character Class Config
                        if item_slice.len() >= 140 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            let mut c_body = Cursor::new(&item_slice[68..]);
                            let class_id = c_body.read_u32::<LittleEndian>().unwrap_or(0);
                            let _faction = c_body.read_u32::<LittleEndian>().unwrap_or(0);
                            let _enemy_faction = c_body.read_u32::<LittleEndian>().unwrap_or(0);
                            let _atk_spd = c_body.read_f32::<LittleEndian>().unwrap_or(1.0);
                            let _atk_range = c_body.read_f32::<LittleEndian>().unwrap_or(3.0);
                            let _hp_gen = c_body.read_i32::<LittleEndian>().unwrap_or(1);
                            let _mp_gen = c_body.read_i32::<LittleEndian>().unwrap_or(1);
                            let _walk_spd = c_body.read_f32::<LittleEndian>().unwrap_or(4.0);
                            let run_speed = c_body.read_f32::<LittleEndian>().unwrap_or(5.0);
                            let _swim_spd = c_body.read_f32::<LittleEndian>().unwrap_or(3.0);
                            let _fly_spd = c_body.read_f32::<LittleEndian>().unwrap_or(5.0);
                            let _crit = c_body.read_i32::<LittleEndian>().unwrap_or(1);
                            let vit_hp = c_body.read_i32::<LittleEndian>().unwrap_or(10);
                            let eng_mp = c_body.read_i32::<LittleEndian>().unwrap_or(10);
                            let _agi_atk = c_body.read_i32::<LittleEndian>().unwrap_or(10);
                            let _agi_arm = c_body.read_i32::<LittleEndian>().unwrap_or(10);
                            let lvlup_hp = c_body.read_i32::<LittleEndian>().unwrap_or(20);
                            let lvlup_mp = c_body.read_i32::<LittleEndian>().unwrap_or(20);

                            self.class_configs.insert(class_id, ClassConfig {
                                id, name, class_id, run_speed, vit_hp, eng_mp, lvlup_hp, lvlup_mp,
                            });
                        }
                    }
                    76 => {
                        // Player Level Exp Config
                        if item_slice.len() >= 668 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            let mut exp_cursor = Cursor::new(&item_slice[68..668]);
                            let mut exp_vec = Vec::with_capacity(150);
                            for _ in 0..150 {
                                exp_vec.push(exp_cursor.read_i32::<LittleEndian>().unwrap_or(0));
                            }
                            self.exp_curves.insert(id, LevelExpConfig { id, name, exp: exp_vec });
                        }
                    }
                    78 => {
                        // Mine Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.mines.insert(id, MineTemplate { id, name, level: 1, exp: 10, sp: 2 });
                        }
                    }
                    89 => {
                        // Suite Essence
                        if item_slice.len() >= 68 {
                            let mut c = Cursor::new(&item_slice[..4]);
                            let id = c.read_u32::<LittleEndian>().unwrap_or(0);
                            let name = decode_utf16le_string(&item_slice[4..68]);
                            self.suites.insert(id, SuiteTemplate { id, name, max_equips: 6 });
                        }
                    }
                    _ => {}
                }

                cursor.seek(SeekFrom::Current(item_sz as i64))?;
            }
        }

        Ok(())
    }

    /// Retorna a quantidade de EXP necessária para um determinado nível a partir da tabela oficial
    pub fn get_exp_for_level(&self, level: usize) -> i32 {
        if let Some(curve) = self.exp_curves.get(&202) {
            if level > 0 && level <= curve.exp.len() {
                return curve.exp[level - 1];
            }
        }
        55
    }

    /// Busca item genérico por ID
    pub fn is_valid_item_id(&self, item_id: u32) -> bool {
        self.weapons.contains_key(&item_id)
            || self.armors.contains_key(&item_id)
            || self.decorations.contains_key(&item_id)
            || self.medicines.contains_key(&item_id)
            || self.materials.contains_key(&item_id)
            || self.addons.contains_key(&item_id)
    }
}

fn decode_utf16le_string(slice: &[u8]) -> String {
    let mut u16_chars = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        let ch = u16::from_le_bytes([chunk[0], chunk[1]]);
        if ch == 0 {
            break;
        }
        u16_chars.push(ch);
    }
    String::from_utf16_lossy(&u16_chars)
}
