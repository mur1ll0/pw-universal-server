use crate::error::{Result, StorageError};
use crate::postgres::PostgresPool;
use crate::repositories::item::ItemRepository;
use crate::repositories::quest::QuestRepository;
use crate::repositories::skill::SkillRepository;
use crate::repositories::template::TemplateRepository;
use chrono::{DateTime, Utc};
use pw_core::{
    AccountId, CharacterClass, CharacterDetails, CharacterSummary, ContainerType, Gender, Race,
    RealmId, RoleId, Vector3, WorldId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CharacterRecord {
    pub id: RoleId,
    pub account_id: AccountId,
    pub realm_id: RealmId,
    pub name: String,
    pub race: i32,
    pub cls: i32,
    pub gender: i16,
    pub level: i32,
    pub cultivation: i32,
    pub exp: i64,
    pub sp: i64,
    pub hp: i32,
    pub mp: i32,
    pub money: i64,
    pub world_id: WorldId,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub custom_data: Option<Vec<u8>>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct CharacterRepository {
    pool: PostgresPool,
    item_repo: ItemRepository,
    skill_repo: SkillRepository,
    quest_repo: QuestRepository,
    template_repo: TemplateRepository,
}

impl CharacterRepository {
    pub fn new(pool: PostgresPool) -> Self {
        let item_repo = ItemRepository::new(pool.clone());
        let skill_repo = SkillRepository::new(pool.clone());
        let quest_repo = QuestRepository::new(pool.clone());
        let template_repo = TemplateRepository::new(pool.clone());
        Self {
            pool,
            item_repo,
            skill_repo,
            quest_repo,
            template_repo,
        }
    }

    pub fn item_repo(&self) -> &ItemRepository {
        &self.item_repo
    }

    pub fn skill_repo(&self) -> &SkillRepository {
        &self.skill_repo
    }

    pub fn quest_repo(&self) -> &QuestRepository {
        &self.quest_repo
    }

    pub fn template_repo(&self) -> &TemplateRepository {
        &self.template_repo
    }

    /// Cria um novo personagem buscando dados da tabela class_templates e insere itens e habilidades
    pub async fn create_character(
        &self,
        account_id: AccountId,
        realm_id: &str,
        name: &str,
        race: Race,
        cls: CharacterClass,
        gender: Gender,
        custom_data: Vec<u8>,
    ) -> Result<RoleId> {
        // Busca template da classe no banco de dados
        let tpl_opt = self.template_repo.get_template_for_class(realm_id, cls as i32).await.unwrap_or(None);

        let (spawn_x, spawn_y, spawn_z, init_lvl, init_cult, init_money, init_sp, world_id) = if let Some(ref tpl) = tpl_opt {
            (
                tpl.template.spawn_x,
                tpl.template.spawn_y,
                tpl.template.spawn_z,
                tpl.template.initial_level,
                tpl.template.initial_cultivation,
                tpl.template.initial_money,
                tpl.template.initial_sp,
                tpl.template.spawn_world_id,
            )
        } else {
            let (sx, sy, sz) = cls.default_spawn_position();
            (sx, sy, sz, 1, 0, 0, 0, 1)
        };

        let (init_hp, init_mp) = cls.default_hp_mp();

        let role_id = sqlx::query_scalar::<_, RoleId>(
            r#"
            INSERT INTO characters (
                account_id, realm_id, name, race, cls, gender, custom_data,
                level, cultivation, money, sp, world_id,
                pos_x, pos_y, pos_z, hp, mp
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(realm_id)
        .bind(name)
        .bind(race as i32)
        .bind(cls as i32)
        .bind(gender as i16)
        .bind(custom_data)
        .bind(init_lvl)
        .bind(init_cult)
        .bind(init_money)
        .bind(init_sp)
        .bind(world_id)
        .bind(spawn_x)
        .bind(spawn_y)
        .bind(spawn_z)
        .bind(init_hp)
        .bind(init_mp)
        .fetch_one(self.pool.get_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
                StorageError::Duplicate(format!("Nome '{}' já está em uso neste Realm", name))
            }
            _ => StorageError::Database(e),
        })?;

        if let Some(tpl) = tpl_opt {
            // 1. Grava itens configurados no template (Equipamentos container_type=1 e Inventário container_type=0)
            for item in tpl.items {
                let ctype = ContainerType::from_i16(item.container_type);
                let record = pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ctype,
                    slot: item.slot as u16,
                    item_id: item.item_id as u32,
                    count: item.count as u32,
                    max_count: 100,
                    refine_level: item.refine_level as u8,
                    sockets_count: item.sockets_count as u8,
                    sockets: item.socket_stones.into_iter().map(|s| s as u32).collect(),
                    durability: item.durability as u32,
                    max_durability: item.max_durability as u32,
                    bind_status: 0,
                    custom_attributes: serde_json::json!({}),
                };
                let _ = self.item_repo.upsert_item(&record).await;
            }

            // 2. Grava habilidades configuradas no template
            for sk in tpl.skills {
                let _ = self.skill_repo.learn_or_upgrade(role_id, sk.skill_id as u32, sk.level as u8).await;
            }
        } else {
            // Fallback: arma inicial equipada no slot 0
            let weapon_id = cls.default_weapon_id() as u32;
            let equip_weapon = pw_core::ItemRecord {
                id: None,
                character_id: role_id,
                container_type: ContainerType::Equipment,
                slot: 0,
                item_id: weapon_id,
                count: 1,
                max_count: 1,
                refine_level: 0,
                sockets_count: 0,
                sockets: vec![],
                durability: 10000,
                max_durability: 10000,
                bind_status: 0,
                custom_attributes: serde_json::json!({}),
            };
            let _ = self.item_repo.upsert_item(&equip_weapon).await;

            for (skill_id, level, _) in cls.default_skills() {
                let _ = self.skill_repo.learn_or_upgrade(role_id, skill_id as u32, level).await;
            }
        }

        Ok(role_id)
    }

    /// Lista os resumos dos personagens para a tela de seleção de personagens do cliente
    pub async fn list_by_account_and_realm(
        &self,
        account_id: AccountId,
        realm_id: &str,
    ) -> Result<Vec<CharacterSummary>> {
        let recs = sqlx::query_as::<_, CharacterRecord>(
            r#"
            SELECT * FROM characters 
            WHERE account_id = $1 AND realm_id = $2 AND is_deleted = FALSE 
            ORDER BY id ASC
            "#,
        )
        .bind(account_id)
        .bind(realm_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        let mut summaries = Vec::with_capacity(recs.len());

        for r in recs {
            let equipment = self
                .item_repo
                .list_by_container(r.id, ContainerType::Equipment)
                .await?;

            let appearance_val = match &r.custom_data {
                Some(bytes) => serde_json::json!({ "raw": hex::encode(bytes) }),
                None => serde_json::json!({}),
            };

            summaries.push(CharacterSummary {
                id: r.id,
                account_id: r.account_id,
                realm_id: r.realm_id,
                name: r.name,
                race: Race::from_u8(r.race as u8).unwrap_or(Race::Human),
                cls: CharacterClass::from_u8(r.cls as u8).unwrap_or(CharacterClass::Blademaster),
                gender: Gender::from_u8(r.gender as u8),
                level: r.level,
                cultivation: r.cultivation,
                world_id: r.world_id,
                position: Vector3::new(r.pos_x, r.pos_y, r.pos_z),
                equipment,
                custom_appearance: appearance_val,
                is_deleted: r.is_deleted,
                delete_time: r.deleted_at,
            });
        }

        Ok(summaries)
    }

    /// Carrega todos os detalhes do personagem para o World Server a partir das tabelas normalizadas
    pub async fn get_details(&self, role_id: RoleId) -> Result<Option<CharacterDetails>> {
        let rec = sqlx::query_as::<_, CharacterRecord>(
            r#"
            SELECT * FROM characters WHERE id = $1
            "#,
        )
        .bind(role_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        let r = match rec {
            Some(row) => row,
            None => return Ok(None),
        };

        let mut inventory = self.item_repo.list_by_container(role_id, ContainerType::Inventory).await?;
        let equipment = self.item_repo.list_by_container(role_id, ContainerType::Equipment).await?;
        let storehouse = self.item_repo.list_by_container(role_id, ContainerType::Storehouse).await?;
        let mut skills = self.skill_repo.list_skills(role_id).await?;
        let quests = self.quest_repo.list_quests(role_id).await?;

        let cls_enum = CharacterClass::from_u8(r.cls as u8).unwrap_or(CharacterClass::Blademaster);

        // Se o personagem já existia no banco sem skills salvas, popula na tabela character_skills
        if skills.is_empty() {
            for (skill_id, level, _) in cls_enum.default_skills() {
                let _ = self.skill_repo.learn_or_upgrade(role_id, skill_id as u32, level).await;
            }
            skills = self.skill_repo.list_skills(role_id).await?;
        }

        // Se o personagem já existia no banco sem itens salvos, popula na tabela character_items
        if inventory.is_empty() && equipment.is_empty() {
            let weapon_id = cls_enum.default_weapon_id() as u32;
            let starter_items = vec![
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 0,
                    item_id: weapon_id,
                    count: 1,
                    max_count: 1,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    custom_attributes: serde_json::json!({}),
                },
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 1,
                    item_id: 2100,
                    count: 5,
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    custom_attributes: serde_json::json!({}),
                },
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 2,
                    item_id: 1796,
                    count: 10,
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    custom_attributes: serde_json::json!({}),
                },
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 3,
                    item_id: 1801,
                    count: 10,
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    custom_attributes: serde_json::json!({}),
                },
            ];
            for item in starter_items {
                let _ = self.item_repo.upsert_item(&item).await;
            }
            inventory = self.item_repo.list_by_container(role_id, ContainerType::Inventory).await?;
        }

        let details = CharacterDetails {
            id: r.id,
            account_id: r.account_id,
            realm_id: r.realm_id,
            name: r.name,
            race: Race::from_u8(r.race as u8).unwrap_or(Race::Human),
            cls: cls_enum,
            gender: Gender::from_u8(r.gender as u8),
            level: r.level,
            cultivation: r.cultivation,
            exp: r.exp,
            sp: r.sp,
            hp: r.hp,
            mp: r.mp,
            money: r.money,
            reputation: 0,
            world_id: r.world_id,
            position: Vector3::new(r.pos_x, r.pos_y, r.pos_z),
            inventory_size: 64,
            storehouse_size: 32,
            inventory,
            equipment,
            storehouse,
            skills,
            quests,
            custom_appearance: match &r.custom_data {
                Some(bytes) => serde_json::json!({ "raw": hex::encode(bytes) }),
                None => serde_json::json!({}),
            },
            version_data: serde_json::json!({}),
            created_at: r.created_at,
            last_login_at: Some(r.updated_at),
        };

        Ok(Some(details))
    }

    /// Salva o estado básico do personagem
    pub async fn save_status(
        &self,
        role_id: RoleId,
        level: i32,
        cultivation: i32,
        exp: i64,
        sp: i64,
        hp: i32,
        mp: i32,
        money: i64,
        world_id: WorldId,
        pos: &Vector3,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET level = $1, cultivation = $2, exp = $3, sp = $4,
                hp = $5, mp = $6, money = $7, world_id = $8,
                pos_x = $9, pos_y = $10, pos_z = $11,
                last_login_at = CURRENT_TIMESTAMP
            WHERE id = $12
            "#,
        )
        .bind(level)
        .bind(cultivation)
        .bind(exp)
        .bind(sp)
        .bind(hp)
        .bind(mp)
        .bind(money)
        .bind(world_id)
        .bind(pos.x)
        .bind(pos.y)
        .bind(pos.z)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Atualiza as coordenadas do personagem no banco de dados
    pub async fn update_position(&self, role_id: RoleId, pos: &Vector3) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET pos_x = $1, pos_y = $2, pos_z = $3,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $4
            "#,
        )
        .bind(pos.x)
        .bind(pos.y)
        .bind(pos.z)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Teletransporte de emergência para a Cidade do Dragão (CDD)
    pub async fn teleport_to_dragon_city(&self, role_id: RoleId) -> Result<()> {
        let cdd = Vector3::dragon_city();
        sqlx::query(
            r#"
            UPDATE characters 
            SET world_id = 1, pos_x = $1, pos_y = $2, pos_z = $3 
            WHERE id = $4
            "#,
        )
        .bind(cdd.x)
        .bind(cdd.y)
        .bind(cdd.z)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Marca o personagem como excluído (soft delete)
    pub async fn delete_character(&self, role_id: RoleId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET is_deleted = TRUE, deleted_at = CURRENT_TIMESTAMP 
            WHERE id = $1
            "#,
        )
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Restaura um personagem marcado para exclusão (undo delete)
    pub async fn restore_character(&self, role_id: RoleId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET is_deleted = FALSE, deleted_at = NULL 
            WHERE id = $1
            "#,
        )
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }
}
