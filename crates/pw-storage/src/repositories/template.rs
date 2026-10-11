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
    /// E8 (B217): EXP e pontos livres iniciais (`GRoleStatus.exp`/`.pp` do molde).
    pub initial_exp: i64,
    pub initial_points: i32,
    pub spawn_world_id: i32,
    pub spawn_x: f32,
    pub spawn_y: f32,
    pub spawn_z: f32,
}

/// E8 (B217): os valores iniciais editáveis de um template — tudo o que o molde do `clsconfig`
/// dá ao personagem novo fora itens e habilidades (`GRoleStatus`: nível, cultivo, exp, sp, pp,
/// atributos, mapa e posição; `GRolePocket.money`). Vida e mana não entram: são a conta da
/// classe sobre nível, vitalidade e energia.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EstatisticasDoTemplate {
    pub nivel: i32,
    pub cultivo: i32,
    pub exp: i64,
    pub sp: i64,
    pub dinheiro: i64,
    pub pontos: i32,
    pub forca: i32,
    pub agilidade: i32,
    pub vitalidade: i32,
    pub energia: i32,
    pub mapa: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ClassTemplateRecord {
    pub fn estatisticas(&self) -> EstatisticasDoTemplate {
        EstatisticasDoTemplate {
            nivel: self.initial_level,
            cultivo: self.initial_cultivation,
            exp: self.initial_exp,
            sp: self.initial_sp,
            dinheiro: self.initial_money,
            pontos: self.initial_points,
            forca: self.strength,
            agilidade: self.agility,
            vitalidade: self.vitality,
            energia: self.energy,
            mapa: self.spawn_world_id,
            x: self.spawn_x,
            y: self.spawn_y,
            z: self.spawn_z,
        }
    }
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

    /// Busca o template completo de uma classe em um Realm específico (semeia o realm vazio).
    pub async fn get_template_for_class(
        &self,
        realm_id: &str,
        cls: i32,
    ) -> Result<Option<FullClassTemplate>> {
        self.ensure_default_templates(realm_id).await?;
        self.template_completo(realm_id, cls).await
    }

    /// O template completo **sem** semear o realm: o que está no banco.
    pub async fn template_completo(
        &self,
        realm_id: &str,
        cls: i32,
    ) -> Result<Option<FullClassTemplate>> {
        let template_opt = sqlx::query_as::<_, ClassTemplateRecord>(
            r#"
            SELECT id, realm_id, cls, name, initial_level, initial_cultivation,
                   initial_money, initial_sp, strength, agility, vitality, energy,
                   initial_exp, initial_points, spawn_world_id, spawn_x, spawn_y, spawn_z
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
                   initial_exp, initial_points, spawn_world_id, spawn_x, spawn_y, spawn_z
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
    ///
    /// Habilidades: as do `GRoleStatus.skills` de cada molde do `clsconfig` 1.2.6 (B108) — a
    /// de ataque da classe e o Portal da Cidade (167). As listas antigas (2, 7, 255-257,
    /// 352-354, 437-439, 1840, 11, 117-119) não estão em molde nenhum.
    pub async fn seed_defaults(&self, realm_id: &str) -> Result<()> {
        // Atributos: 5/5/5/5 em toda classe, como os moldes do `clsconfig` (`pw_core::ATRIBUTO_INICIAL`).
        // Até o B217 o semeio gravava 15/10/20/5 etc., que a criação não lia.
        let a = pw_core::ATRIBUTO_INICIAL;
        let classes = [
            (0, "Guerreiro", 976.0, 219.2, 4187.3, 2097, vec![1, 167]),
            (1, "Mago", 976.0, 219.2, 4187.3, 2867, vec![81, 167]),
            (3, "Feiticeira", -1445.6, 219.3, 2642.0, 2258, vec![299, 167]),
            (4, "Bárbaro", -1445.6, 219.3, 2642.0, 2867, vec![102, 167]),
            (6, "Arqueiro", -696.3, 219.0, -1178.8, 2250, vec![234, 235, 167]),
            (7, "Sacerdote", -696.3, 219.0, -1178.8, 2867, vec![113, 125, 167]),
        ];

        for (cls, name, sx, sy, sz, weapon_id, skills) in classes {
            let (str_pt, agi, vit, eng) = (a, a, a, a);
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

            // Equipamento inicial que depende da classe, por slot de `EQUIPIVTR_*`
            // (`EC_IvtrTypes.h:56-96` do client 1.5.5):
            //
            //   * 11 `EQUIPIVTR_PROJECTILE` — a munição do Arqueiro. **Estava no 12**, que
            //     é o slot de voo: as flechas ocupavam o lugar das asas.
            //   * 12 `EQUIPIVTR_FLYSWORD` — o item de voo. Para Arqueiro (6) e Sacerdote
            //     (7), que são Alados, é a "Asa" (2096) — a única linha da tabela
            //     `WINGMANWING_ESSENCE` do `elements.data`, nível 1. O cliente confere a
            //     classe por conta própria: `CanUseEquipment` recusa `ICID_WING` para
            //     quem não for `PROF_ARCHOR` nem `PROF_ANGEL` (`EC_HostPlayer.cpp:4927`).
            //
            // As outras raças voam com item de `FLYSWORD_ESSENCE`, que tem 1.098 linhas e
            // nenhum campo de classe — sem uma forma medida de escolher, elas ficam sem
            // item de voo inicial em vez de ganhar um chute.
            let mut equipamento_da_classe: Vec<(i16, i32, i32)> = Vec::new();
            if cls == 6 {
                equipamento_da_classe.push((11, 43283, 1000)); // Flecha de Novato (PROJECTILE_ESSENCE, type 8546 = require_projectile do Arco de Madeira, arma de nível 0-17 — o arco é nível 0)
            }
            if cls == 6 || cls == 7 {
                equipamento_da_classe.push((12, 2096, 1)); // Asa
            }

            for (slot, item_id, count) in equipamento_da_classe {
                sqlx::query(
                    r#"
                    INSERT INTO class_template_items (
                        template_id, container_type, slot, item_id, count,
                        durability, max_durability, refine_level, sockets_count, socket_stones
                    )
                    VALUES ($1, 1, $2, $3, $4, 0, 0, 0, 0, '{}')
                    ON CONFLICT (template_id, container_type, slot) DO UPDATE SET
                        item_id = EXCLUDED.item_id,
                        count = EXCLUDED.count,
                        durability = EXCLUDED.durability,
                        max_durability = EXCLUDED.max_durability
                    "#,
                )
                .bind(id)
                .bind(slot)
                .bind(item_id)
                .bind(count)
                .execute(self.pool.get_ref())
                .await?;
            }

            // 2. Itens Iniciais na Bolsa / Inventário (container_type = 0)
            let bag_items = vec![
                (0, 2100, 5),
                (1, 1796, 10),
                (2, 1801, 10),
            ];

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

    /// E8 (B217): os templates de um realm como estão no banco, sem semear.
    pub async fn listar_completos(&self, realm_id: &str) -> Result<Vec<FullClassTemplate>> {
        let classes: Vec<(i32,)> = sqlx::query_as("SELECT cls FROM class_templates WHERE realm_id = $1 ORDER BY cls")
            .bind(realm_id)
            .fetch_all(self.pool.get_ref())
            .await?;
        let mut todos = Vec::with_capacity(classes.len());
        for (cls,) in classes {
            if let Some(t) = self.template_completo(realm_id, cls).await? {
                todos.push(t);
            }
        }
        Ok(todos)
    }

    /// Se o realm tem template para a classe (sem semear). Decide o caminho de reparo da carga:
    /// com template, o personagem recebe só o que o template manda, mesmo que nada.
    pub async fn existe(&self, realm_id: &str, cls: i32) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM class_templates WHERE realm_id = $1 AND cls = $2)",
        )
        .bind(realm_id)
        .bind(cls)
        .fetch_one(self.pool.get_ref())
        .await?)
    }

    async fn id_do_template(&self, realm_id: &str, cls: i32) -> Result<Option<i32>> {
        Ok(sqlx::query_scalar("SELECT id FROM class_templates WHERE realm_id = $1 AND cls = $2")
            .bind(realm_id)
            .bind(cls)
            .fetch_optional(self.pool.get_ref())
            .await?)
    }

    /// E8 (B217): grava os valores iniciais. `false` = o realm não tem template para a classe.
    pub async fn definir_estatisticas(&self, realm_id: &str, cls: i32, e: &EstatisticasDoTemplate) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE class_templates SET initial_level=$3, initial_cultivation=$4, initial_exp=$5, initial_sp=$6,
                 initial_money=$7, initial_points=$8, strength=$9, agility=$10, vitality=$11, energy=$12,
                 spawn_world_id=$13, spawn_x=$14, spawn_y=$15, spawn_z=$16, updated_at=CURRENT_TIMESTAMP
             WHERE realm_id=$1 AND cls=$2",
        )
        .bind(realm_id).bind(cls)
        .bind(e.nivel).bind(e.cultivo).bind(e.exp).bind(e.sp).bind(e.dinheiro).bind(e.pontos)
        .bind(e.forca).bind(e.agilidade).bind(e.vitalidade).bind(e.energia)
        .bind(e.mapa).bind(e.x).bind(e.y).bind(e.z)
        .execute(self.pool.get_ref())
        .await?;
        Ok(r.rows_affected() == 1)
    }

    /// E8: põe (ou troca) o item de um slot. `container_type` 0 bolsa, 1 corpo.
    #[allow(clippy::too_many_arguments)]
    pub async fn definir_item(&self, realm_id: &str, cls: i32, container_type: i16, slot: i16,
                              item_id: i32, count: i32, durabilidade: i32) -> Result<bool> {
        let Some(id) = self.id_do_template(realm_id, cls).await? else { return Ok(false) };
        sqlx::query(
            "INSERT INTO class_template_items (template_id, container_type, slot, item_id, count,
                 durability, max_durability, refine_level, sockets_count, socket_stones)
             VALUES ($1, $2, $3, $4, $5, $6, $6, 0, 0, '{}')
             ON CONFLICT (template_id, container_type, slot) DO UPDATE SET
                 item_id = EXCLUDED.item_id, count = EXCLUDED.count, durability = EXCLUDED.durability,
                 max_durability = EXCLUDED.max_durability, refine_level = 0, sockets_count = 0, socket_stones = '{}'",
        )
        .bind(id).bind(container_type).bind(slot).bind(item_id).bind(count).bind(durabilidade)
        .execute(self.pool.get_ref())
        .await?;
        self.tocar(id).await?;
        Ok(true)
    }

    /// E8: tira o item do slot. `false` = sem template ou slot vazio.
    pub async fn remover_item(&self, realm_id: &str, cls: i32, container_type: i16, slot: i16) -> Result<bool> {
        let Some(id) = self.id_do_template(realm_id, cls).await? else { return Ok(false) };
        let r = sqlx::query("DELETE FROM class_template_items WHERE template_id=$1 AND container_type=$2 AND slot=$3")
            .bind(id).bind(container_type).bind(slot)
            .execute(self.pool.get_ref())
            .await?;
        self.tocar(id).await?;
        Ok(r.rows_affected() == 1)
    }

    /// E8: leva o item de `de` para `para`, trocando com o que estiver lá. `false` = sem template
    /// ou `de` vazio. Numa transação, passando pela vaga `(-1, -1)` por causa da chave única do slot.
    pub async fn mover_item(&self, realm_id: &str, cls: i32, de: (i16, i16), para: (i16, i16)) -> Result<bool> {
        let Some(id) = self.id_do_template(realm_id, cls).await? else { return Ok(false) };
        let mut tx = self.pool.get_ref().begin().await?;
        let mover = |(c1, s1): (i16, i16), (c2, s2): (i16, i16)| {
            sqlx::query("UPDATE class_template_items SET container_type=$4, slot=$5
                         WHERE template_id=$1 AND container_type=$2 AND slot=$3")
                .bind(id).bind(c1).bind(s1).bind(c2).bind(s2)
        };
        if mover(de, (-1, -1)).execute(&mut *tx).await?.rows_affected() != 1 {
            return Ok(false);
        }
        mover(para, de).execute(&mut *tx).await?;
        mover((-1, -1), para).execute(&mut *tx).await?;
        tx.commit().await?;
        self.tocar(id).await?;
        Ok(true)
    }

    /// E8: habilidade inicial no `nivel` (0 remove). `false` = sem template, ou remover a ausente.
    pub async fn definir_habilidade(&self, realm_id: &str, cls: i32, skill_id: i32, nivel: i16) -> Result<bool> {
        let Some(id) = self.id_do_template(realm_id, cls).await? else { return Ok(false) };
        let r = if nivel == 0 {
            sqlx::query("DELETE FROM class_template_skills WHERE template_id=$1 AND skill_id=$2")
                .bind(id).bind(skill_id)
                .execute(self.pool.get_ref()).await?
        } else {
            sqlx::query("INSERT INTO class_template_skills (template_id, skill_id, level) VALUES ($1, $2, $3)
                         ON CONFLICT (template_id, skill_id) DO UPDATE SET level = EXCLUDED.level")
                .bind(id).bind(skill_id).bind(nivel)
                .execute(self.pool.get_ref()).await?
        };
        self.tocar(id).await?;
        Ok(r.rows_affected() >= 1)
    }

    async fn tocar(&self, id: i32) -> Result<()> {
        sqlx::query("UPDATE class_templates SET updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(id).execute(self.pool.get_ref()).await?;
        Ok(())
    }

}
