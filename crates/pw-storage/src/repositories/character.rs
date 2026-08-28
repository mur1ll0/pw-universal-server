use crate::error::{Result, StorageError};
use crate::postgres::PostgresPool;
use crate::repositories::item::ItemRepository;
use crate::repositories::quest::QuestRepository;
use crate::repositories::skill::SkillRepository;
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
}

impl CharacterRepository {
    pub fn new(pool: PostgresPool) -> Self {
        let item_repo = ItemRepository::new(pool.clone());
        let skill_repo = SkillRepository::new(pool.clone());
        let quest_repo = QuestRepository::new(pool.clone());
        Self {
            pool,
            item_repo,
            skill_repo,
            quest_repo,
        }
    }

    /// Cria um novo personagem
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
        let (spawn_x, spawn_y, spawn_z) = cls.default_spawn_position();
        let (init_hp, init_mp) = cls.default_hp_mp();

        let role_id = sqlx::query_scalar::<_, RoleId>(
            r#"
            INSERT INTO characters (
                account_id, realm_id, name, race, cls, gender, custom_data,
                pos_x, pos_y, pos_z, hp, mp
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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

    /// Carrega todos os detalhes do personagem para o World Server
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

        let inventory = self.item_repo.list_by_container(role_id, ContainerType::Inventory).await?;
        let equipment = self.item_repo.list_by_container(role_id, ContainerType::Equipment).await?;
        let storehouse = self.item_repo.list_by_container(role_id, ContainerType::Storehouse).await?;
        let skills = self.skill_repo.list_skills(role_id).await?;
        let quests = self.quest_repo.list_quests(role_id).await?;

        let details = CharacterDetails {
            id: r.id,
            account_id: r.account_id,
            realm_id: r.realm_id,
            name: r.name,
            race: Race::from_u8(r.race as u8).unwrap_or(Race::Human),
            cls: CharacterClass::from_u8(r.cls as u8).unwrap_or(CharacterClass::Blademaster),
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
