use crate::aipolicy::AiPolicyData;
use crate::collision::MapCollision;
use crate::elements::ElementsData;
use crate::gshop::GShopData;
use crate::npcgen::NpcGenData;
use crate::tasks::TasksData;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

/// Gerenciador Central de Dados de Jogo (Carregado na inicialização do World Server)
#[derive(Debug, Clone, Default)]
pub struct GameDataManager {
    pub elements: ElementsData,
    pub gshop: GShopData,
    pub tasks: TasksData,
    pub aipolicy: AiPolicyData,
    
    // Spawns indexados por ID do Mapa/Instância (ex: 1 -> world/npcgen.data, 101 -> a01/npcgen.data)
    pub map_spawns: HashMap<i32, NpcGenData>,
    pub collisions: HashMap<i32, MapCollision>,
}

impl GameDataManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Carrega todos os arquivos de dados a partir de uma pasta de configuração (ex: `config/`)
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, config_dir: P) -> anyhow::Result<()> {
        let dir = config_dir.as_ref();
        info!("Carregando templates de jogo a partir de: {:?}", dir);

        // 1. Arquivos Globais de Configuração
        let elements_path = dir.join("elements.data");
        if elements_path.exists() {
            let data = std::fs::read(&elements_path)?;
            self.elements = ElementsData::load_from_bytes(&data)?;
        }

        let gshop_path = dir.join("gshop.data");
        if gshop_path.exists() {
            let data = std::fs::read(&gshop_path)?;
            self.gshop = GShopData::load_from_bytes(&data)?;
        }

        let tasks_path = dir.join("tasks.data");
        if tasks_path.exists() {
            let data = std::fs::read(&tasks_path)?;
            self.tasks = TasksData::load_from_bytes(&data)?;
        }

        let aipolicy_path = dir.join("aipolicy.data");
        if aipolicy_path.exists() {
            let data = std::fs::read(&aipolicy_path)?;
            self.aipolicy = AiPolicyData::load_from_bytes(&data)?;
        }

        // 2. Carrega o npcgen.data específico de cada pasta de mapa (world, a01, a02, etc.)
        self.load_map_folder(1, &dir.join("world"))?;
        
        // Mapeamento das dungeons clássicas a01..a33 e b01..b35
        for i in 1..=33 {
            let folder_name = format!("a{:02}", i);
            let map_path = dir.join(&folder_name);
            if map_path.exists() {
                let _ = self.load_map_folder(100 + i, &map_path);
            }
        }
        for i in 1..=35 {
            let folder_name = format!("b{:02}", i);
            let map_path = dir.join(&folder_name);
            if map_path.exists() {
                let _ = self.load_map_folder(200 + i, &map_path);
            }
        }

        info!("Todos os templates de dados e mapas foram carregados com sucesso!");
        Ok(())
    }

    /// Carrega os dados específicos de uma pasta de mapa (npcgen.data e colisão)
    fn load_map_folder(&mut self, world_id: i32, map_dir: &Path) -> anyhow::Result<()> {
        let npcgen_path = map_dir.join("npcgen.data");
        if npcgen_path.exists() {
            let data = std::fs::read(&npcgen_path)?;
            let spawns = NpcGenData::load_from_bytes(&data)?;
            self.map_spawns.insert(world_id, spawns);
        }

        let clt_path = map_dir.join("collision.clt");
        if clt_path.exists() {
            let data = std::fs::read(&clt_path)?;
            let collision = MapCollision::load_from_bytes(world_id, &data)?;
            self.collisions.insert(world_id, collision);
        }

        Ok(())
    }
}
