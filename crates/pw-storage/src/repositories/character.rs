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
    pub reputation: i32,
    pub world_id: WorldId,
    pub pos_x: sqlx::types::BigDecimal,
    pub pos_y: sqlx::types::BigDecimal,
    pub pos_z: sqlx::types::BigDecimal,
    pub inventory_size: i16,
    pub storehouse_size: i16,
    pub is_deleted: bool,
    pub delete_time: Option<DateTime<Utc>>,
    pub custom_appearance: sqlx::types::Json<serde_json::Value>,
    pub version_data: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
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
        custom_appearance: serde_json::Value,
    ) -> Result<RoleId> {
        let role_id = sqlx::query_scalar::<_, RoleId>(
            r#"
            INSERT INTO characters (
                account_id, realm_id, name, race, cls, gender, custom_appearance
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(realm_id)
        .bind(name)
        .bind(race as i32)
        .bind(cls as i32)
        .bind(gender as i16)
        .bind(sqlx::types::Json(custom_appearance))
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
            // Carrega os itens equipados para exibir visualmente o personagem com suas armaduras e armas
            let equipment = self
                .item_repo
                .list_by_container(r.id, ContainerType::Equipment)
                .await?;

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
                position: Vector3::new(
                    r.pos_x.to_string().parse().unwrap_or(550.0),
                    r.pos_y.to_string().parse().unwrap_or(200.0),
                    r.pos_z.to_string().parse().unwrap_or(650.0),
                ),
                equipment,
                custom_appearance: r.custom_appearance.0,
                is_deleted: r.is_deleted,
                delete_time: r.delete_time,
            });
        }

        Ok(summaries)
    }

    /// Carrega todos os detalhes do personagem (itens normalizados, skills e quests) para o World Server
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

        // Carrega as coleções normalizadas em paralelo ou sequencialmente
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
            reputation: r.reputation,
            world_id: r.world_id,
            position: Vector3::new(
                r.pos_x.to_string().parse().unwrap_or(550.0),
                r.pos_y.to_string().parse().unwrap_or(200.0),
                r.pos_z.to_string().parse().unwrap_or(650.0),
            ),
            inventory_size: r.inventory_size as u16,
            storehouse_size: r.storehouse_size as u16,
            inventory,
            equipment,
            storehouse,
            skills,
            quests,
            custom_appearance: r.custom_appearance.0,
            version_data: r.version_data.0,
            created_at: r.created_at,
            last_login_at: r.last_login_at,
        };

        Ok(Some(details))
    }

    /// Salva o estado básico do personagem (HP, MP, EXP, Posição)
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
}
