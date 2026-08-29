use crate::error::Result;
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::{ContainerType, ItemRecord, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemRow {
    pub id: i64,
    pub character_id: i32,
    pub container_type: i16,
    pub slot: i16,
    pub item_id: i32,
    pub count: i32,
    pub durability: i32,
    pub max_durability: i32,
    pub refine_level: i16,
    pub sockets_count: i16,
    pub socket_stones: Vec<i32>,
    pub creator_name: Option<String>,
    pub bind_status: i32,
    pub expire_time: Option<DateTime<Utc>>,
    pub extra_data: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ItemRow> for ItemRecord {
    fn from(r: ItemRow) -> Self {
        Self {
            id: Some(r.id),
            character_id: r.character_id,
            container_type: ContainerType::from_i16(r.container_type),
            slot: r.slot as u16,
            item_id: r.item_id as u32,
            count: r.count as u32,
            max_count: 100,
            refine_level: r.refine_level as u8,
            sockets_count: r.sockets_count as u8,
            sockets: r.socket_stones.into_iter().map(|s| s as u32).collect(),
            durability: r.durability as u32,
            max_durability: r.max_durability as u32,
            bind_status: r.bind_status as u8,
            custom_attributes: serde_json::json!({
                "creator_name": r.creator_name,
                "extra_data": r.extra_data.map(|d| hex::encode(d))
            }),
        }
    }
}

#[derive(Clone)]
pub struct ItemRepository {
    pool: PostgresPool,
}

impl ItemRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Busca todos os itens de um container específico (ex: INVENTORY ou EQUIPMENT)
    pub async fn list_by_container(
        &self,
        character_id: RoleId,
        container_type: ContainerType,
    ) -> Result<Vec<ItemRecord>> {
        let rows = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT * FROM character_items 
            WHERE character_id = $1 AND container_type = $2 
            ORDER BY slot ASC
            "#,
        )
        .bind(character_id)
        .bind(container_type.to_i16())
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Busca todos os itens do personagem (todos os containers)
    pub async fn list_all_for_character(&self, character_id: RoleId) -> Result<Vec<ItemRecord>> {
        let rows = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT * FROM character_items 
            WHERE character_id = $1 
            ORDER BY container_type, slot ASC
            "#,
        )
        .bind(character_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Insere ou atualiza um item em um slot específico (UPSERT atômico)
    pub async fn upsert_item(&self, item: &ItemRecord) -> Result<i64> {
        let sockets_i32: Vec<i32> = item.sockets.iter().map(|&s| s as i32).collect();

        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO character_items (
                character_id, container_type, slot, item_id, count,
                durability, max_durability, refine_level, sockets_count,
                socket_stones, bind_status, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP)
            ON CONFLICT (character_id, container_type, slot) DO UPDATE SET
                item_id = EXCLUDED.item_id,
                count = EXCLUDED.count,
                durability = EXCLUDED.durability,
                max_durability = EXCLUDED.max_durability,
                refine_level = EXCLUDED.refine_level,
                sockets_count = EXCLUDED.sockets_count,
                socket_stones = EXCLUDED.socket_stones,
                bind_status = EXCLUDED.bind_status,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id
            "#,
        )
        .bind(item.character_id)
        .bind(item.container_type.to_i16())
        .bind(item.slot as i16)
        .bind(item.item_id as i32)
        .bind(item.count as i32)
        .bind(item.durability as i32)
        .bind(item.max_durability as i32)
        .bind(item.refine_level as i16)
        .bind(item.sockets_count as i16)
        .bind(&sockets_i32)
        .bind(item.bind_status as i32)
        .fetch_one(self.pool.get_ref())
        .await?;

        Ok(id)
    }

    /// Altera o nível de refino de um item específico (ex: via Painel Web)
    pub async fn update_refine(&self, item_instance_id: i64, refine_level: u8) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE character_items 
            SET refine_level = $1, updated_at = CURRENT_TIMESTAMP 
            WHERE id = $2
            "#,
        )
        .bind(refine_level as i16)
        .bind(item_instance_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Remove um item de um slot
    pub async fn delete_item_by_slot(
        &self,
        character_id: RoleId,
        container_type: ContainerType,
        slot: u16,
    ) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM character_items 
            WHERE character_id = $1 AND container_type = $2 AND slot = $3
            "#,
        )
        .bind(character_id)
        .bind(container_type.to_i16())
        .bind(slot as i16)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Remove item por ID de instância
    pub async fn delete_item_by_id(&self, item_instance_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM character_items WHERE id = $1
            "#,
        )
        .bind(item_instance_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Busca um item específico por slot
    pub async fn get_item_by_slot(
        &self,
        character_id: RoleId,
        container_type: ContainerType,
        slot: u16,
    ) -> Result<Option<ItemRecord>> {
        let row = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT * FROM character_items 
            WHERE character_id = $1 AND container_type = $2 AND slot = $3
            "#,
        )
        .bind(character_id)
        .bind(container_type.to_i16())
        .bind(slot as i16)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(row.map(Into::into))
    }

    /// Troca os slots de dois itens dentro do mesmo container
    pub async fn swap_slots(
        &self,
        character_id: RoleId,
        container_type: ContainerType,
        slot1: u16,
        slot2: u16,
    ) -> Result<()> {
        let item1 = self.get_item_by_slot(character_id, container_type, slot1).await?;
        let item2 = self.get_item_by_slot(character_id, container_type, slot2).await?;

        // Remove ambos temporariamente para evitar colisão de chave única (uq_item_slot_per_container)
        self.delete_item_by_slot(character_id, container_type, slot1).await?;
        self.delete_item_by_slot(character_id, container_type, slot2).await?;

        if let Some(mut i1) = item1 {
            i1.slot = slot2;
            self.upsert_item(&i1).await?;
        }

        if let Some(mut i2) = item2 {
            i2.slot = slot1;
            self.upsert_item(&i2).await?;
        }

        Ok(())
    }

    /// Move ou equipa um item entre containers (ex: Inventário -> Equipamento)
    pub async fn move_between_containers(
        &self,
        character_id: RoleId,
        src_container: ContainerType,
        src_slot: u16,
        dest_container: ContainerType,
        dest_slot: u16,
    ) -> Result<()> {
        let src_item = self.get_item_by_slot(character_id, src_container, src_slot).await?;
        let dest_item = self.get_item_by_slot(character_id, dest_container, dest_slot).await?;

        self.delete_item_by_slot(character_id, src_container, src_slot).await?;
        self.delete_item_by_slot(character_id, dest_container, dest_slot).await?;

        if let Some(mut s) = src_item {
            s.container_type = dest_container;
            s.slot = dest_slot;
            self.upsert_item(&s).await?;
        }

        if let Some(mut d) = dest_item {
            d.container_type = src_container;
            d.slot = src_slot;
            self.upsert_item(&d).await?;
        }

        Ok(())
    }

    /// Consome quantidade de um item (ex: poções)
    pub async fn consume_item(
        &self,
        character_id: RoleId,
        container_type: ContainerType,
        slot: u16,
        amount: u32,
    ) -> Result<Option<ItemRecord>> {
        if let Some(mut item) = self.get_item_by_slot(character_id, container_type, slot).await? {
            if item.count <= amount {
                self.delete_item_by_slot(character_id, container_type, slot).await?;
                Ok(None)
            } else {
                item.count -= amount;
                self.upsert_item(&item).await?;
                Ok(Some(item))
            }
        } else {
            Ok(None)
        }
    }
}
