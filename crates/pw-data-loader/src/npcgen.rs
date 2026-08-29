use byteorder::{LittleEndian, ReadBytesExt};
use pw_core::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum NpcGenError {
    #[error("Erro de I/O na leitura do npcgen.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato ou versão do npcgen.data inválido: versão={0}")]
    InvalidVersion(u32),
}

pub type Result<T> = std::result::Result<T, NpcGenError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnType {
    Monster,
    Npc,
    ResourceMine,
    DynamicObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnInstance {
    pub instance_id: i32,      // ID único de instância no mundo em execução (ex: 20001, 20002...)
    pub template_id: u32,      // Template ID do elements.data
    pub spawn_type: SpawnType,
    pub pos: Vector3,
    pub dir: Vector3,
    pub respawn_sec: u32,
    pub aggressive: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SpatialGrid {
    pub cell_size: f32, // padrão 64.0m
    pub cells: HashMap<(i32, i32), Vec<SpawnInstance>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, spawn: SpawnInstance) {
        let gx = (spawn.pos.x / self.cell_size).floor() as i32;
        let gz = (spawn.pos.z / self.cell_size).floor() as i32;
        self.cells.entry((gx, gz)).or_default().push(spawn);
    }

    /// Retorna todas as instâncias de NPCs/monstros no raio de visão especificado
    pub fn query_radius(&self, pos: Vector3, radius: f32) -> Vec<&SpawnInstance> {
        let mut results = Vec::new();
        let min_gx = ((pos.x - radius) / self.cell_size).floor() as i32;
        let max_gx = ((pos.x + radius) / self.cell_size).floor() as i32;
        let min_gz = ((pos.z - radius) / self.cell_size).floor() as i32;
        let max_gz = ((pos.z + radius) / self.cell_size).floor() as i32;

        let r_sq = radius * radius;
        for gx in min_gx..=max_gx {
            for gz in min_gz..=max_gz {
                if let Some(cell_spawns) = self.cells.get(&(gx, gz)) {
                    for spawn in cell_spawns {
                        let dx = spawn.pos.x - pos.x;
                        let dz = spawn.pos.z - pos.z;
                        if (dx * dx + dz * dz) <= r_sq {
                            results.push(spawn);
                        }
                    }
                }
            }
        }
        results
    }
}

#[derive(Debug, Clone, Default)]
pub struct NpcGenData {
    pub version: u32,
    pub instances: Vec<SpawnInstance>,
    pub grid: SpatialGrid,
}

impl NpcGenData {
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        info!("Carregando npcgen.data (Spawns oficiais de Monstros e NPCs do Perfect World)...");

        let version = cursor.read_u32::<LittleEndian>()?;
        let num_ai_gen = cursor.read_i32::<LittleEndian>()? as usize;
        let num_res_area = cursor.read_i32::<LittleEndian>()? as usize;
        let num_dyn_obj = cursor.read_i32::<LittleEndian>()? as usize;
        let num_npc_ctrl = cursor.read_i32::<LittleEndian>()? as usize;

        info!(
            "npcgen.data v{}: {} áreas de IA, {} áreas de recursos, {} objetos dinâmicos, {} controladores",
            version, num_ai_gen, num_res_area, num_dyn_obj, num_npc_ctrl
        );

        let mut instances = Vec::with_capacity(36000);
        let mut grid = SpatialGrid::new(64.0);
        let mut instance_counter: u32 = 1000;

        // 1. Áreas de IA (Monstros e NPCs de Cidade) - Estrutura NPCGENFILEAREA7 (71 bytes)
        for _ in 0..num_ai_gen {
            let _area_type = cursor.read_i32::<LittleEndian>()?;
            let num_gen = cursor.read_i32::<LittleEndian>()? as usize;
            let pos_x = cursor.read_f32::<LittleEndian>()?;
            let pos_y = cursor.read_f32::<LittleEndian>()?;
            let pos_z = cursor.read_f32::<LittleEndian>()?;
            let dir_x = cursor.read_f32::<LittleEndian>()?;
            let dir_y = cursor.read_f32::<LittleEndian>()?;
            let dir_z = cursor.read_f32::<LittleEndian>()?;
            let _ext_x = cursor.read_f32::<LittleEndian>()?;
            let _ext_y = cursor.read_f32::<LittleEndian>()?;
            let _ext_z = cursor.read_f32::<LittleEndian>()?;
            let _npc_type = cursor.read_i32::<LittleEndian>()?;
            let _grp_type = cursor.read_i32::<LittleEndian>()?;
            let b_init_gen = cursor.read_u8()? != 0;
            let _b_auto_revive = cursor.read_u8()? != 0;
            let _b_valid_once = cursor.read_u8()? != 0;
            let _dw_gen_id = cursor.read_u32::<LittleEndian>()?;
            let id_ctrl = cursor.read_i32::<LittleEndian>()?;
            let _life_time = cursor.read_i32::<LittleEndian>()?;
            let _max_num = cursor.read_i32::<LittleEndian>()?;

            let is_active_at_boot = b_init_gen && id_ctrl == 0;
            let area_pos = Vector3::new(pos_x, pos_y, pos_z);
            let area_dir = Vector3::new(dir_x, dir_y, dir_z);

            for _ in 0..num_gen {
                let tid = cursor.read_u32::<LittleEndian>()?;
                let count = cursor.read_u32::<LittleEndian>()?;
                let refresh = cursor.read_i32::<LittleEndian>()?;
                let _died_times = cursor.read_u32::<LittleEndian>()?;
                let aggressive = cursor.read_u32::<LittleEndian>()?;

                // Pula os 40 bytes restantes do registro NPCGENFILEAIGEN10 (60 bytes total - 20 lidos)
                cursor.seek(SeekFrom::Current(40))?;

                if is_active_at_boot && tid > 0 {
                    for c in 0..count.min(10) {
                        instance_counter += 1;
                        // No Perfect World oficial, IDs de NPCs/Monstros possuem o bit 31 ativo (ISNPCID: (id & 0x80000000) && !(id & 0x40000000))
                        let npc_nid = (0x80000000u32 | (instance_counter & 0x3FFFFFFF)) as i32;
                        let offset_x = if c == 0 { 0.0 } else { ((c as f32) * 1.5) - 3.0 };
                        let offset_z = if c == 0 { 0.0 } else { ((c as f32) * 1.2) - 2.5 };
                        let spawn = SpawnInstance {
                            instance_id: npc_nid,
                            template_id: tid,
                            spawn_type: if tid >= 10000 { SpawnType::Npc } else { SpawnType::Monster },
                            pos: Vector3::new(area_pos.x + offset_x, area_pos.y, area_pos.z + offset_z),
                            dir: area_dir,
                            respawn_sec: refresh.max(1) as u32,
                            aggressive,
                        };
                        grid.insert(spawn.clone());
                        instances.push(spawn);
                    }
                }
            }
        }

        // 2. Áreas de Recursos / Minérios - Estrutura NPCGENFILERESAREA7 (42 bytes)
        for _ in 0..num_res_area {
            let pos_x = cursor.read_f32::<LittleEndian>()?;
            let pos_y = cursor.read_f32::<LittleEndian>()?;
            let pos_z = cursor.read_f32::<LittleEndian>()?;
            let _ext_x = cursor.read_f32::<LittleEndian>()?;
            let _ext_z = cursor.read_f32::<LittleEndian>()?;
            let num_res = cursor.read_i32::<LittleEndian>()? as usize;
            let b_init_gen = cursor.read_u8()? != 0;
            let _b_auto_revive = cursor.read_u8()? != 0;
            let _b_valid_once = cursor.read_u8()? != 0;
            let _dw_gen_id = cursor.read_u32::<LittleEndian>()?;
            let _dir = cursor.read_u16::<LittleEndian>()?;
            let _rad = cursor.read_u8()?;
            let id_ctrl = cursor.read_i32::<LittleEndian>()?;
            let _max_num = cursor.read_i32::<LittleEndian>()?;

            let is_active_at_boot = b_init_gen && id_ctrl == 0;
            let res_pos = Vector3::new(pos_x, pos_y, pos_z);
            for _ in 0..num_res {
                let _res_type = cursor.read_i32::<LittleEndian>()?;
                let template_id = cursor.read_u32::<LittleEndian>()?;
                let refresh = cursor.read_u32::<LittleEndian>()?;
                let count = cursor.read_u32::<LittleEndian>()?;
                let _hei_off = cursor.read_f32::<LittleEndian>()?;

                if is_active_at_boot && template_id > 0 {
                    for _ in 0..count.min(5) {
                        instance_counter += 1;
                        let matter_id = (0xC0000000u32 | (instance_counter & 0x3FFFFFFF)) as i32;
                        let spawn = SpawnInstance {
                            instance_id: matter_id,
                            template_id,
                            spawn_type: SpawnType::ResourceMine,
                            pos: res_pos,
                            dir: Vector3::new(0.0, 0.0, 1.0),
                            respawn_sec: refresh.max(5),
                            aggressive: 0,
                        };
                        grid.insert(spawn.clone());
                        instances.push(spawn);
                    }
                }
            }
        }

        info!(
            "npcgen.data carregado com sucesso: {} entidades indexadas no Grid Espacial de 64m",
            instances.len()
        );

        Ok(Self {
            version,
            instances,
            grid,
        })
    }

    pub fn query_nearby(&self, pos: Vector3, radius: f32) -> Vec<&SpawnInstance> {
        self.grid.query_radius(pos, radius)
    }
}

/// Compacta a direção horizontal em um byte (0..255), idêntico a glb_CompressDirH da engine oficial
pub fn compress_dir_h(x: f32, z: f32) -> u8 {
    const INV_INTER: f32 = 256.0 / 360.0;
    if x.abs() < 0.00001 {
        if z > 0.0 {
            64
        } else {
            192
        }
    } else {
        let deg = z.atan2(x).to_degrees();
        let deg_norm = if deg < 0.0 { deg + 360.0 } else { deg };
        (deg_norm * INV_INTER) as u8
    }
}
