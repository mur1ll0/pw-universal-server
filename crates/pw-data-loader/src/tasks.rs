use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
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

/// Assinatura do `tasks.data` (`TASK_PACK_MAGIC`, `Task/TaskTempl.h:222`).
pub const MAGICO_DO_TASKS: u32 = 0x9385_8361;

#[derive(Debug, Clone, Default)]
pub struct TasksData {
    /// O `_task_templ_cur_version` deste arquivo — a **segunda** palavra do cabeçalho.
    pub version: u32,
    /// Quantas missões o arquivo declara (`item_count` do cabeçalho).
    pub quantidade_declarada: u32,
    pub tasks: HashMap<u32, TaskTemplate>,
}

impl TasksData {
    /// Lê o cabeçalho: `(version, item_count)`.
    ///
    /// # Formato (autoridade)
    ///
    /// `CElementClient/Task/TaskTempl.h:224`:
    ///
    /// ```cpp
    /// #define TASK_PACK_MAGIC 0x93858361
    /// struct TASK_PACK_HEADER { unsigned long magic; unsigned long version; unsigned long item_count; };
    /// ```
    ///
    /// E `TaskTemplMan.cpp:1599`: o cliente recusa o arquivo se
    /// `tph.version != _task_templ_cur_version`. Ou seja — como no `elements.data` — a
    /// `version` deste arquivo **é** a constante do cliente que consegue abri-lo, e é ela
    /// que tem que ir para a string `edition` do handshake.
    ///
    /// Isto era lido como "a primeira palavra é a versão", o que devolvia o mágico
    /// `0x93858361` como se fosse número de versão.
    pub fn ler_cabecalho(data: &[u8]) -> Result<(u32, u32)> {
        let mut cursor = Cursor::new(data);
        let magico = cursor.read_u32::<LittleEndian>()?;
        if magico != MAGICO_DO_TASKS {
            return Err(TasksError::InvalidFormat);
        }
        let versao = cursor.read_u32::<LittleEndian>()?;
        let quantidade = cursor.read_u32::<LittleEndian>()?;
        Ok((versao, quantidade))
    }

    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let (version, quantidade_declarada) = Self::ler_cabecalho(data)?;
        let mut cursor = Cursor::new(data);
        cursor.set_position(12);

        info!(
            "Carregando tasks.data: _task_templ_cur_version = {}, {} missões declaradas",
            version, quantidade_declarada
        );

        let mut tasks_data = Self {
            version,
            quantidade_declarada,
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
