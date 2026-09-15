//! As cinco listas binárias de missão de cada personagem — ver
//! `scripts/2026_09_14_listas_de_missao.sql` para o porquê de serem blobs.

use crate::error::Result;
use crate::postgres::PostgresPool;
use pw_core::RoleId;

/// Os cinco blocos, na ordem do `TASK_DATA` (105).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListasDeMissaoGravadas {
    pub ativa: Vec<u8>,
    pub concluidas: Vec<u8>,
    pub tempos: Vec<u8>,
    pub contagens: Vec<u8>,
    pub deposito: Vec<u8>,
}

#[derive(Clone)]
pub struct TaskListRepository {
    pool: PostgresPool,
}

impl TaskListRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// `None` para personagem que nunca teve lista gravada — quem chama começa do zero.
    pub async fn carregar(&self, role_id: RoleId) -> Result<Option<ListasDeMissaoGravadas>> {
        let linha: Option<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> = sqlx::query_as(
            "SELECT active, finished, finish_time, finish_count, storage
               FROM character_task_lists WHERE character_id = $1",
        )
        .bind(role_id)
        .fetch_optional(self.pool.get_ref())
        .await?;
        Ok(linha.map(|(ativa, concluidas, tempos, contagens, deposito)| ListasDeMissaoGravadas {
            ativa,
            concluidas,
            tempos,
            contagens,
            deposito,
        }))
    }

    pub async fn gravar(&self, role_id: RoleId, l: &ListasDeMissaoGravadas) -> Result<()> {
        sqlx::query(
            "INSERT INTO character_task_lists
                 (character_id, active, finished, finish_time, finish_count, storage, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)
             ON CONFLICT (character_id) DO UPDATE SET
                 active = EXCLUDED.active, finished = EXCLUDED.finished,
                 finish_time = EXCLUDED.finish_time, finish_count = EXCLUDED.finish_count,
                 storage = EXCLUDED.storage, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(role_id)
        .bind(&l.ativa)
        .bind(&l.concluidas)
        .bind(&l.tempos)
        .bind(&l.contagens)
        .bind(&l.deposito)
        .execute(self.pool.get_ref())
        .await?;
        Ok(())
    }
}
