use crate::items::ItemRecord;
use crate::math::Vector3;
use crate::types::{AccountId, CharacterClass, Gender, Race, RealmId, RoleId, WorldId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sumário de personagem para a tela de seleção de personagens do cliente
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub id: RoleId,
    pub account_id: AccountId,
    pub realm_id: RealmId,
    pub name: String,
    pub race: Race,
    pub cls: CharacterClass,
    pub gender: Gender,
    pub level: i32,
    pub cultivation: i32,
    pub world_id: WorldId,
    pub position: Vector3,
    pub equipment: Vec<ItemRecord>,
    pub custom_appearance: serde_json::Value,
    pub is_deleted: bool,
    pub delete_time: Option<DateTime<Utc>>,
}

impl CharacterSummary {
    /// Um `CharacterSummary` zerado, para quando o protocolo exige um `RoleInfo` mas
    /// não há personagem a informar — o `CreateRole_Re` de uma criação que falhou, por
    /// exemplo. O protocolo não tem campo opcional: o erro vai no `result`, e a
    /// estrutura vai vazia.
    pub fn vazio() -> Self {
        Self {
            id: 0,
            account_id: 0,
            realm_id: String::new(),
            name: String::new(),
            race: Race::Human,
            cls: CharacterClass::Blademaster,
            gender: Gender::Male,
            level: 0,
            cultivation: 0,
            world_id: 0,
            position: Vector3::default(),
            equipment: Vec::new(),
            custom_appearance: serde_json::Value::Null,
            is_deleted: false,
            delete_time: None,
        }
    }
}

/// Dados completos do Personagem para o Game Engine (`pw-gs`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterDetails {
    pub id: RoleId,
    pub account_id: AccountId,
    pub realm_id: RealmId,
    pub name: String,
    pub race: Race,
    pub cls: CharacterClass,
    pub gender: Gender,
    pub level: i32,
    pub cultivation: i32,
    pub exp: i64,
    pub sp: i64,
    pub hp: i32,
    pub mp: i32,
    pub money: i64,
    pub reputation: i32,
    pub world_id: WorldId,
    pub position: Vector3,
    
    pub inventory_size: u16,
    pub storehouse_size: u16,
    
    // Coleções normalizadas (carregadas sob demanda ou no login)
    pub inventory: Vec<ItemRecord>,
    pub equipment: Vec<ItemRecord>,
    pub storehouse: Vec<ItemRecord>,
    pub skills: Vec<LearnedSkill>,
    pub quests: Vec<CharacterQuest>,
    
    pub custom_appearance: serde_json::Value,
    pub version_data: serde_json::Value,
    
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Habilidade aprendida pelo personagem
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnedSkill {
    pub character_id: RoleId,
    pub skill_id: u32,
    pub level: u8,
}

/// Missão do personagem (ativa ou concluída)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterQuest {
    pub character_id: RoleId,
    pub quest_id: u32,
    pub status: QuestStatus,
    pub progress: Vec<i32>,
    pub expire_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    Active,
    Completed,
}

impl QuestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestStatus::Active => "ACTIVE",
            QuestStatus::Completed => "COMPLETED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "COMPLETED" => QuestStatus::Completed,
            _ => QuestStatus::Active,
        }
    }
}
