use crate::error::Result;
use crate::postgres::PostgresPool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClassTemplateRecord {
    pub id: i32,
    pub realm_id: String,
    pub cls: i32,
    pub name: String,
    pub initial_level: i32,
    pub initial_cultivation: i32,
    pub initial_money: i64,
    pub initial_sp: i64,
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub energy: i32,
    pub spawn_world_id: i32,
    pub spawn_x: f32,
    pub spawn_y: f32,
    pub spawn_z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TemplateItemRecord {
    pub id: i64,
    pub template_id: i32,
    pub container_type: i16,
    pub slot: i16,
    pub item_id: i32,
    pub count: i32,
    pub durability: i32,
    pub max_durability: i32,
    pub refine_level: i16,
    pub sockets_count: i16,
    pub socket_stones: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TemplateSkillRecord {
    pub template_id: i32,
    pub skill_id: i32,
    pub level: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullClassTemplate {
    pub template: ClassTemplateRecord,
    pub items: Vec<TemplateItemRecord>,
    pub skills: Vec<TemplateSkillRecord>,
}

#[derive(Clone)]
pub struct TemplateRepository {
    pool: PostgresPool,
}

impl TemplateRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Busca o template completo de uma classe em um Realm específico
    pub async fn get_template_for_class(
        &self,
        realm_id: &str,
        cls: i32,
    ) -> Result<Option<FullClassTemplate>> {
        self.ensure_default_templates(realm_id).await?;

        let template_opt = sqlx::query_as::<_, ClassTemplateRecord>(
            r#"
            SELECT id, realm_id, cls, name, initial_level, initial_cultivation,
                   initial_money, initial_sp, strength, agility, vitality, energy,
                   spawn_world_id, spawn_x, spawn_y, spawn_z
            FROM class_templates
            WHERE realm_id = $1 AND cls = $2
            "#,
        )
        .bind(realm_id)
        .bind(cls)
        .fetch_optional(self.pool.get_ref())
        .await?;

        if let Some(template) = template_opt {
            let items = sqlx::query_as::<_, TemplateItemRecord>(
                r#"
                SELECT id, template_id, container_type, slot, item_id, count,
                       durability, max_durability, refine_level, sockets_count, socket_stones
                FROM class_template_items
                WHERE template_id = $1
                ORDER BY container_type DESC, slot ASC
                "#,
            )
            .bind(template.id)
            .fetch_all(self.pool.get_ref())
            .await?;

            let skills = sqlx::query_as::<_, TemplateSkillRecord>(
                r#"
                SELECT template_id, skill_id, level
                FROM class_template_skills
                WHERE template_id = $1
                ORDER BY skill_id ASC
                "#,
            )
            .bind(template.id)
            .fetch_all(self.pool.get_ref())
            .await?;

            Ok(Some(FullClassTemplate {
                template,
                items,
                skills,
            }))
        } else {
            Ok(None)
        }
    }

    /// Lista todos os templates de classes de um Realm
    pub async fn list_templates(&self, realm_id: &str) -> Result<Vec<ClassTemplateRecord>> {
        self.ensure_default_templates(realm_id).await?;

        let rows = sqlx::query_as::<_, ClassTemplateRecord>(
            r#"
            SELECT id, realm_id, cls, name, initial_level, initial_cultivation,
                   initial_money, initial_sp, strength, agility, vitality, energy,
                   spawn_world_id, spawn_x, spawn_y, spawn_z
            FROM class_templates
            WHERE realm_id = $1
            ORDER BY cls ASC
            "#,
        )
        .bind(realm_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(rows)
    }

    /// Garante que os templates padrão existam no banco
    pub async fn ensure_default_templates(&self, realm_id: &str) -> Result<()> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM class_templates WHERE realm_id = $1"
        )
        .bind(realm_id)
        .fetch_one(self.pool.get_ref())
        .await?;

        if count.0 == 0 {
            self.seed_defaults(realm_id).await?;
        }

        Ok(())
    }

    /// Popula as tabelas com os valores oficiais do clsconfig / v1.2.6
    pub async fn seed_defaults(&self, realm_id: &str) -> Result<()> {
        let classes = [
            (0, "Guerreiro", 15, 10, 20, 5, 976.0, 219.2, 4187.3, 2097, vec![1, 2, 7, 167]),
            (1, "Mago", 10, 10, 10, 20, 976.0, 219.2, 4187.3, 2867, vec![255, 256, 257, 167]),
            (3, "Bárbaro", 15, 5, 25, 5, -1445.6, 219.3, 2642.0, 2258, vec![352, 353, 354, 167]),
            (4, "Feiticeira", 15, 5, 15, 15, -1445.6, 219.3, 2642.0, 2867, vec![437, 438, 439, 167]),
            (6, "Arqueiro", 5, 15, 8, 22, -696.3, 219.0, -1178.8, 2250, vec![1840, 234, 235, 167]),
            (7, "Sacerdote", 10, 10, 15, 15, -696.3, 219.0, -1178.8, 2867, vec![11, 117, 118, 119, 167]),
        ];

        for (cls, name, str_pt, agi, vit, eng, sx, sy, sz, weapon_id, skills) in classes {
            let tpl_id: (i32,) = sqlx::query_as(
                r#"
                INSERT INTO class_templates (
                    realm_id, cls, name, initial_level, initial_cultivation,
                    initial_money, initial_sp, strength, agility, vitality, energy,
                    spawn_world_id, spawn_x, spawn_y, spawn_z
                )
                VALUES ($1, $2, $3, 1, 0, 0, 0, $4, $5, $6, $7, 1, $8, $9, $10)
                ON CONFLICT (realm_id, cls) DO UPDATE SET
                    name = EXCLUDED.name,
                    updated_at = CURRENT_TIMESTAMP
                RETURNING id
                "#,
            )
            .bind(realm_id)
            .bind(cls)
            .bind(name)
            .bind(str_pt)
            .bind(agi)
            .bind(vit)
            .bind(eng)
            .bind(sx)
            .bind(sy)
            .bind(sz)
            .fetch_one(self.pool.get_ref())
            .await?;

            let id = tpl_id.0;

            // 1. Arma Inicial Equipada no Corpo (container_type = 1, slot = 0)
            sqlx::query(
                r#"
                INSERT INTO class_template_items (
                    template_id, container_type, slot, item_id, count,
                    durability, max_durability, refine_level, sockets_count, socket_stones
                )
                VALUES ($1, 1, 0, $2, 1, 2800, 2800, 0, 0, '{}')
                ON CONFLICT (template_id, container_type, slot) DO UPDATE SET
                    item_id = EXCLUDED.item_id,
                    count = EXCLUDED.count,
                    durability = EXCLUDED.durability,
                    max_durability = EXCLUDED.max_durability
                "#,
            )
            .bind(id)
            .bind(weapon_id)
            .execute(self.pool.get_ref())
            .await?;

            // Se for Arqueiro (cls == 6), equipa flechas de madeira no slot de munição (slot 12)
            if cls == 6 {
                sqlx::query(
                    r#"
                    INSERT INTO class_template_items (
                        template_id, container_type, slot, item_id, count,
                        durability, max_durability, refine_level, sockets_count, socket_stones
                    )
                    VALUES ($1, 1, 12, 2271, 1000, 0, 0, 0, 0, '{}')
                    ON CONFLICT (template_id, container_type, slot) DO UPDATE SET
                        item_id = EXCLUDED.item_id,
                        count = EXCLUDED.count,
                        durability = EXCLUDED.durability,
                        max_durability = EXCLUDED.max_durability
                    "#,
                )
                .bind(id)
                .execute(self.pool.get_ref())
                .await?;
            }

            // 2. Itens Iniciais na Bolsa / Inventário (container_type = 0)
            let mut bag_items = vec![
                (0, 2100, 5),
                (1, 1796, 10),
                (2, 1801, 10),
            ];
            if cls == 6 {
                bag_items.push((3, 2271, 1000));
            }

            for (slot, item_id, count) in bag_items {
                sqlx::query(
                    r#"
                    INSERT INTO class_template_items (
                        template_id, container_type, slot, item_id, count,
                        durability, max_durability, refine_level, sockets_count, socket_stones
                    )
                    VALUES ($1, 0, $2, $3, $4, 0, 0, 0, 0, '{}')
                    ON CONFLICT (template_id, container_type, slot) DO UPDATE SET
                        item_id = EXCLUDED.item_id,
                        count = EXCLUDED.count,
                        durability = EXCLUDED.durability,
                        max_durability = EXCLUDED.max_durability
                    "#,
                )
                .bind(id)
                .bind(slot as i16)
                .bind(item_id)
                .bind(count)
                .execute(self.pool.get_ref())
                .await?;
            }

            // 3. Habilidades Iniciais
            for sk_id in skills {
                sqlx::query(
                    r#"
                    INSERT INTO class_template_skills (template_id, skill_id, level)
                    VALUES ($1, $2, 1)
                    ON CONFLICT (template_id, skill_id) DO NOTHING
                    "#,
                )
                .bind(id)
                .bind(sk_id)
                .execute(self.pool.get_ref())
                .await?;
            }
        }

        Ok(())
    }
}
