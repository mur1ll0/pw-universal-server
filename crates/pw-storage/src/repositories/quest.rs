use crate::error::Result;
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::{CharacterQuest, QuestStatus, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QuestRow {
    pub character_id: i32,
    pub quest_id: i32,
    pub status: String,
    pub progress: Vec<i32>,
    pub expire_time: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct QuestRepository {
    pool: PostgresPool,
}

impl QuestRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Lista todas as missões do personagem
    pub async fn list_quests(&self, character_id: RoleId) -> Result<Vec<CharacterQuest>> {
        let rows = sqlx::query_as::<_, QuestRow>(
            r#"
            SELECT character_id, quest_id, status, progress, expire_time 
            FROM character_quests 
            WHERE character_id = $1 
            ORDER BY quest_id ASC
            "#,
        )
        .bind(character_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        let quests = rows
            .into_iter()
            .map(|r| CharacterQuest {
                character_id: r.character_id,
                quest_id: r.quest_id as u32,
                status: QuestStatus::from_str(&r.status),
                progress: r.progress,
                expire_time: r.expire_time,
            })
            .collect();

        Ok(quests)
    }

    /// Salva ou atualiza o progresso de uma missão
    pub async fn save_quest(
        &self,
        character_id: RoleId,
        quest_id: u32,
        status: QuestStatus,
        progress: &[i32],
        expire_time: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO character_quests (character_id, quest_id, status, progress, expire_time, updated_at)
            VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
            ON CONFLICT (character_id, quest_id) DO UPDATE SET
                status = EXCLUDED.status,
                progress = EXCLUDED.progress,
                expire_time = EXCLUDED.expire_time,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(character_id)
        .bind(quest_id as i32)
        .bind(status.as_str())
        .bind(progress)
        .bind(expire_time)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }
}
