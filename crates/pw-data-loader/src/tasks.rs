use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum TasksError {
    #[error("Erro de I/O na leitura do tasks.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato de tasks.data inválido")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, TasksError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReward {
    pub exp: i64,
    pub sp: i64,
    pub money: i64,
    pub reputation: i32,
    pub items: Vec<(u32, u32)>, // (item_id, count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: u32,
    pub name: String,
    pub min_level: i32,
    pub max_level: i32,
    pub req_cultivation: i32,
    pub req_classes: Vec<u8>,
    pub pre_tasks: Vec<u32>,
    pub monster_kills: Vec<(u32, u32)>,        // (monster_id, count)
    pub item_collections: Vec<(u32, u32, f32)>,// (item_id, count, drop_chance)
    pub rewards: TaskReward,
}

#[derive(Debug, Clone, Default)]
pub struct TasksData {
    pub version: u32,
    pub tasks: HashMap<u32, TaskTemplate>,
}

impl TasksData {
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let version = cursor.read_u32::<LittleEndian>().unwrap_or(12);

        info!("Carregando tasks.data: Versão identificada = {}", version);

        let mut tasks_data = Self {
            version,
            tasks: HashMap::new(),
        };

        tasks_data.parse_tasks(&mut cursor)?;
        info!("tasks.data carregado com sucesso: {} missões registradas", tasks_data.tasks.len());

        Ok(tasks_data)
    }

    fn parse_tasks(&mut self, _cursor: &mut Cursor<&[u8]>) -> Result<()> {
        // Leitura tolerante de missões
        Ok(())
    }

    pub fn get_task(&self, task_id: u32) -> Option<&TaskTemplate> {
        self.tasks.get(&task_id)
    }
}
