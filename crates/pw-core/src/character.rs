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
    /// Quando este personagem entrou no mundo pela última vez.
    ///
    /// Vai no `lastlogin_time` do `RoleInfo`, e é com ele que o **cliente** decide qual
    /// personagem vem selecionado: ele varre a lista e fica com o de maior valor
    /// (`EC_LoginUIMan.cpp:809-818`). Com zero em todos, caía sempre no primeiro.
    pub last_login_at: Option<DateTime<Utc>>,
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
            last_login_at: None,
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

    /// Os quatro atributos distribuíveis, do banco. Ver a nota em
    /// `pw_storage::repositories::character::CharacterRecord`: as colunas existem desde
    /// o começo e nunca eram lidas.
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub energy: i32,
    
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

/// A ficha de uma arma, como o cliente precisa recebê-la no `OWN_ITEM_INFO`.
///
/// Mora aqui, e não no `pw-protocol` nem no `pw-data-loader`, porque os dois precisam
/// dela e nenhum dos dois deve depender do outro: o leitor de arquivos não deve conhecer
/// o formato de rede, e o codificador de rede não deve conhecer o `elements.data`. O
/// `pw-core` é o crate que os dois já enxergam.
///
/// Quem converte é `pw_data_loader::armas::TemplateDeArma`, com um `From`.
///
/// Todos os campos saem do `WEAPON_ESSENCE` — ver `pw_data_loader::armas`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FichaDaArma {
    /// `weapon_type`: 0 corpo a corpo, 1 longo alcance (`EC_IvtrTypes.h:166-167`).
    ///
    /// **É o campo mais perigoso desta struct.** Declarar longo alcance faz o cliente
    /// cobrar munição em `CanUseEquipment`, e sem munição o item vira vermelho e
    /// inutilizável.
    pub tipo_de_arma: i16,
    /// Máscara de classes que podem equipar (`character_combo_id`), um bit por classe.
    /// **Zero recusa todo mundo.**
    pub classes_permitidas: i32,
    pub nivel_exigido: i16,
    pub forca_exigida: i16,
    pub vitalidade_exigida: i16,
    pub agilidade_exigida: i16,
    pub energia_exigida: i16,
    pub municao_exigida: i32,
    pub tipo_maior: i32,
    pub dano_minimo: i32,
    pub dano_maximo: i32,
    pub dano_magico_minimo: i32,
    pub dano_magico_maximo: i32,
    /// Em *ticks* de 50 ms, como o cliente conta.
    pub velocidade_de_ataque: i32,
    pub alcance: f32,
}
