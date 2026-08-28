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

    /// Retorna a coordenada 3D oficial de spawn inicial da vila de nascimento por raça/classe (precinct.sev)
    pub fn default_spawn_position(&self) -> (f32, f32, f32) {
        match self {
            CharacterClass::Blademaster | CharacterClass::Wizard => (976.0, 219.2, 4187.3),   // Vale das Espadas / 剑仙城 (Humanos) -> Mapa (498, 819)
            CharacterClass::Archer | CharacterClass::Cleric => (-741.5, 219.1, -1234.8),      // Vale das Plumas / Vila dos Alados -> Mapa (326, 276)
            CharacterClass::Barbarian | CharacterClass::Venomancer => (-1445.6, 219.3, 2642.0), // Vale das Feras / 万化城 (Selvagens) -> Mapa (255, 664)
            CharacterClass::Assassin | CharacterClass::Psychomancer => (650.0, 130.0, 130.0), // Cidade das Tormentas (Abissais)
            CharacterClass::Seeker | CharacterClass::Mystic => (380.0, 230.0, 230.0),      // Cidade da Névoa (Guardiões)
            CharacterClass::Duskblade | CharacterClass::Stormbringer => (150.0, 250.0, 250.0), // Cidade do Crepúsculo (Sombrios)
        }
    }

    /// Retorna as habilidades iniciais oficiais de nível 1 por classe (CElementSkill v1.2.6)
    pub fn default_skills(&self) -> Vec<(i16, u8, i16)> {
        match self {
            CharacterClass::Cleric => vec![
                (125, 1, 0), // 羽箭 (Pluma Espiritual / Feather Arrow - dano mágico de metal de Sacerdote)
                (113, 1, 0), // 清心咒 (Prece da Clareza / Heal básico de Sacerdote)
                (190, 1, 0), // 飞行精通 (Maestria em Voo dos Alados)
                (167, 1, 0), // 回城术 (Portal da Cidade)
            ],
            CharacterClass::Archer => vec![
                (234, 1, 0), // 引而不发 (Tiro Certeiro / Aimed Shot - ataque básico de Arqueiro)
                (235, 1, 0), // 连射 (Tiro Duplo / Quick Shot)
                (274, 1, 0), // 飞行精通 (Maestria em Voo dos Alados)
                (167, 1, 0), // 回城术 (Portal da Cidade)
            ],
            CharacterClass::Blademaster => vec![
                (1, 1, 0),   // 流水诀 (Golpe de Onda - cls 0)
                (167, 1, 0), // 回城术 (Portal da Cidade)
            ],
            CharacterClass::Wizard => vec![
                (27, 1, 0),  // 烈火符 (Piromancia - cls 1)
                (167, 1, 0), // 回城术 (Portal da Cidade)
            ],
            CharacterClass::Barbarian => vec![
                (90, 1, 0),  // 重击 (Golpe Violento - cls 4)
                (167, 1, 0), // 回城术 (Portal da Cidade)
            ],
            CharacterClass::Venomancer => vec![
                (60, 1, 0),  // 剧毒蛊 (Enxame de Ferroadas - cls 3)
                (61, 1, 0),  // 驯服宠物 (Adestrar Criatura - cls 3)
                (167, 1, 0), // 回城术 (Portal da Cidade)
            ],
            _ => vec![(167, 1, 0)],
        }
    }

    /// Retorna o ID da arma inicial no elements.data
    pub fn default_weapon_id(&self) -> i32 {
        match self {
            CharacterClass::Archer => 2250,     // Arco de Madeira (TID 2250 do elements.data v1.2.6)
            CharacterClass::Barbarian => 2258,  // Porrete de Madeira (TID 2258 do elements.data v1.2.6)
            _ => 2097,                          // Espada de Madeira (TID 2097 do elements.data v1.2.6)
        }
    }

    /// Retorna os valores iniciais oficiais de HP e MP de nível 1 por classe
    pub fn default_hp_mp(&self) -> (i32, i32) {
        match self {
            CharacterClass::Blademaster => (225, 80),
            CharacterClass::Wizard => (100, 280),
            CharacterClass::Archer => (130, 100),
            CharacterClass::Cleric => (120, 280),
            CharacterClass::Barbarian => (260, 70),
            CharacterClass::Venomancer => (120, 240),
            CharacterClass::Assassin => (150, 120),
            CharacterClass::Psychomancer => (110, 260),
            CharacterClass::Seeker => (200, 100),
            CharacterClass::Mystic => (120, 260),
            CharacterClass::Duskblade => (160, 140),
            CharacterClass::Stormbringer => (110, 270),
        }
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
