use pw_core::Vector3;
use pw_data_loader::{ElementsData, GShopData, GameDataManager, NpcGenData, TasksData};
use std::path::Path;

#[test]
fn test_elements_data_real_file_if_present() {
    let path = Path::new("F:/Python_C_Projects/PWSource1.5.3/files1.2.6/pwserver/gamed/config/elements.data");
    if path.exists() {
        let bytes = std::fs::read(path).expect("Falha ao ler elements.data 1.2.6");
        let elements = ElementsData::load_from_bytes(&bytes).expect("Falha ao parsear elements.data v55");
        
        assert_eq!(elements.version, 7);
        assert_eq!(elements.signature, 12288);
        assert_eq!(elements.table_counts.len(), 118);
        assert!(!elements.weapons.is_empty(), "Deveria ter armas carregadas");
        assert!(!elements.monsters.is_empty(), "Deveria ter monstros carregados");
        assert!(!elements.npcs.is_empty(), "Deveria ter NPCs carregados");
    }
}

#[test]
fn test_tasks_data_real_file_if_present() {
    let path = Path::new("F:/Python_C_Projects/PWSource1.5.3/files1.2.6/pwserver/gamed/config/tasks.data");
    if path.exists() {
        let bytes = std::fs::read(path).expect("Falha ao ler tasks.data 1.2.6");
        let tasks = TasksData::load_from_bytes(&bytes).expect("Falha ao parsear tasks.data v55");
        
        assert!(tasks.version > 0);
    }
}

#[test]
fn test_npcgen_data_real_file_if_present() {
    let path = Path::new("F:/Python_C_Projects/PWSource1.5.3/files1.2.6/pwserver/gamed/config/npcgen.data");
    if path.exists() {
        let bytes = std::fs::read(path).expect("Falha ao ler npcgen.data 1.2.6");
        let npcgen = NpcGenData::load_from_bytes(&bytes).expect("Falha ao parsear npcgen.data v10");
        
        assert_eq!(npcgen.version, 10);
        assert!(!npcgen.instances.is_empty());
        
        // Testa consulta espacial no grid ao redor do Vale das Plumas (-741.5, 219.1, -1234.8)
        let nearby = npcgen.grid.query_radius(Vector3::new(-741.5, 219.1, -1234.8), 200.0);
        assert!(!nearby.is_empty(), "Deveria encontrar spawns ao redor do Vale das Plumas");
    }
}

#[test]
fn test_gshop_data_real_file_if_present() {
    let path = Path::new("F:/Python_C_Projects/PWSource1.5.3/files1.2.6/pwserver/gamed/config/gshop.data");
    if path.exists() {
        let bytes = std::fs::read(path).expect("Falha ao ler gshop.data 1.2.6");
        let gshop = GShopData::load_from_bytes(&bytes).expect("Falha ao parsear gshop.data");
        
        assert!(!gshop.items.is_empty() || gshop.timestamp > 0);
    }
}

#[test]
fn test_game_data_manager_directory_load() {
    let dir = Path::new("F:/Python_C_Projects/PWSource1.5.3/files1.2.6/pwserver/gamed/config");
    if dir.exists() {
        let mut manager = GameDataManager::new();
        let res = manager.load_from_directory(dir);
        assert!(res.is_ok(), "GameDataManager deve carregar a pasta 1.2.6 sem erros");
        assert!(manager.map_spawns.contains_key(&1), "World 1 deve ter spawns carregados");
    }
}
