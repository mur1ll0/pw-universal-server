use serde::{Deserialize, Serialize};

/// Identificador único de Conta (Global)
pub type AccountId = i32;

/// Identificador único de Personagem (RoleId)
pub type RoleId = i32;

/// Identificador do Realm / Servidor (ex: "realm_126", "realm_153")
pub type RealmId = String;

/// Identificador do Mapa / World / Dungeon (1 = World)
pub type WorldId = i32;

/// Identificador de Facção / Clã
pub type FactionId = i32;

/// Raças do Perfect World
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Race {
    Human = 0,      // Humanos
    WingedElf = 1,  // Alados
    Untamed = 2,    // Selvagens
    Tideborn = 3,   // Abissais (v1.4.2+)
    Earthguard = 4, // Guardiões (v1.4.4+)
    Nightshade = 5, // Sombrios (v1.5.3+)
}

impl Race {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Race::Human),
            1 => Some(Race::WingedElf),
            2 => Some(Race::Untamed),
            3 => Some(Race::Tideborn),
            4 => Some(Race::Earthguard),
            5 => Some(Race::Nightshade),
            _ => None,
        }
    }
}

/// Classes de Personagem do Perfect World
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CharacterClass {
    Blademaster = 0,  // Guerreiro
    Wizard = 1,       // Mago
    Psychomancer = 2, // Espiritualista / Monge legado
    Venomancer = 3,   // Feiticeira
    Barbarian = 4,    // Bárbaro
    Assassin = 5,     // Mercenário / Genie legado
    Archer = 6,       // Arqueiro
    Cleric = 7,       // Sacerdote
    Seeker = 8,       // Arcano (v1.4.4+)
    Mystic = 9,       // Místico (v1.4.4+)
    Duskblade = 10,   // Retalhador (v1.5.3+)
    Stormbringer = 11,// Tormentador (v1.5.3+)
}

impl CharacterClass {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CharacterClass::Blademaster),
            1 => Some(CharacterClass::Wizard),
            2 => Some(CharacterClass::Psychomancer),
            3 => Some(CharacterClass::Venomancer),
            4 => Some(CharacterClass::Barbarian),
            5 => Some(CharacterClass::Assassin),
            6 => Some(CharacterClass::Archer),
            7 => Some(CharacterClass::Cleric),
            8 => Some(CharacterClass::Seeker),
            9 => Some(CharacterClass::Mystic),
            10 => Some(CharacterClass::Duskblade),
            11 => Some(CharacterClass::Stormbringer),
            _ => None,
        }
    }

    /// Retorna true se a classe pertence à versão clássica 1.2.6
    pub fn is_classic_126(&self) -> bool {
        matches!(
            self,
            CharacterClass::Blademaster
                | CharacterClass::Wizard
                | CharacterClass::Venomancer
                | CharacterClass::Barbarian
                | CharacterClass::Archer
                | CharacterClass::Cleric
        )
    }
}

/// Gênero do Personagem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Gender {
    Male = 0,
    Female = 1,
}

impl Gender {
    pub fn from_u8(val: u8) -> Self {
        if val == 1 {
            Gender::Female
        } else {
            Gender::Male
        }
    }
}
