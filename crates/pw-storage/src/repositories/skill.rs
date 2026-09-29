use crate::error::Result;
use crate::postgres::PostgresPool;
use pw_core::{LearnedSkill, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SkillRow {
    pub character_id: i32,
    pub skill_id: i32,
    pub level: i16,
    pub ability: i32,
}

#[derive(Clone)]
pub struct SkillRepository {
    pool: PostgresPool,
}

impl SkillRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Lista todas as habilidades aprendidas pelo personagem
    pub async fn list_skills(&self, character_id: RoleId) -> Result<Vec<LearnedSkill>> {
        let rows = sqlx::query_as::<_, SkillRow>(
            r#"
            SELECT character_id, skill_id, level, ability
            FROM character_skills 
            WHERE character_id = $1 
            ORDER BY skill_id ASC
            "#,
        )
        .bind(character_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        let skills = rows
            .into_iter()
            .map(|r| LearnedSkill {
                character_id: r.character_id,
                skill_id: r.skill_id as u32,
                level: r.level as u8,
                ability: r.ability,
            })
            .collect();

        Ok(skills)
    }

    /// Grava a proficiência de uma habilidade já aprendida.
    pub async fn gravar_proficiencia(&self, character_id: RoleId, skill_id: u32, ability: i32) -> Result<()> {
        sqlx::query("UPDATE character_skills SET ability = $3 WHERE character_id = $1 AND skill_id = $2")
            .bind(character_id)
            .bind(skill_id as i32)
            .bind(ability)
            .execute(self.pool.get_ref())
            .await?;
        Ok(())
    }

    /// Aprende ou sobe o nível de uma habilidade (UPSERT)
    pub async fn learn_or_upgrade(&self, character_id: RoleId, skill_id: u32, level: u8) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO character_skills (character_id, skill_id, level)
            VALUES ($1, $2, $3)
            ON CONFLICT (character_id, skill_id) DO UPDATE 
            SET level = EXCLUDED.level
            "#,
        )
        .bind(character_id)
        .bind(skill_id as i32)
        .bind(level as i16)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }
}
