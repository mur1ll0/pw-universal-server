use pw_core::CharacterClass;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameVersion {
    V1_2_6,
    V1_4_8,
    V1_5_3,
}

impl GameVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameVersion::V1_2_6 => "1.2.6",
            GameVersion::V1_4_8 => "1.4.8",
            GameVersion::V1_5_3 => "1.5.3",
        }
    }

    /// Código numérico da versão retornado no handshake Challenge do Wanmei GNet
    /// Formato oficial: (major << 24) | (minor << 16) | (release << 8) | patch
    pub fn server_version_code(&self) -> u32 {
        match self {
            GameVersion::V1_2_6 => 0x00010206, // 66054: Versão 1.2.6 oficial de ElementClient.exe
            GameVersion::V1_4_8 => 0x00010408, // 66568: Versão 1.4.8 (Tides / Genesis)
            GameVersion::V1_5_3 => 0x00010503, // 66819: Versão 1.5.3 (Eclipse)
        }
    }

    /// Quantidade de campos serializados na struct RoleInfo
    pub fn role_info_fields_count(&self) -> usize {
        match self {
            GameVersion::V1_2_6 => 19,
            GameVersion::V1_4_8 => 23,
            GameVersion::V1_5_3 => 23,
        }
    }

    /// Valida se uma classe de personagem é compatível com esta versão do jogo
    pub fn is_class_supported(&self, cls: CharacterClass) -> bool {
        match self {
            GameVersion::V1_2_6 => matches!(
                cls,
                CharacterClass::Blademaster
                    | CharacterClass::Wizard
                    | CharacterClass::Barbarian
                    | CharacterClass::Venomancer
                    | CharacterClass::Archer
                    | CharacterClass::Cleric
            ),
            GameVersion::V1_4_8 => matches!(
                cls,
                CharacterClass::Blademaster
                    | CharacterClass::Wizard
                    | CharacterClass::Barbarian
                    | CharacterClass::Venomancer
                    | CharacterClass::Archer
                    | CharacterClass::Cleric
                    | CharacterClass::Assassin
                    | CharacterClass::Psychomancer
                    | CharacterClass::Seeker
                    | CharacterClass::Mystic
            ),
            GameVersion::V1_5_3 => true, // Suporta todas as 12 classes (+ Duskblade, Stormbringer)
        }
    }

    pub fn has_reincarnation(&self) -> bool {
        matches!(self, GameVersion::V1_4_8 | GameVersion::V1_5_3)
    }

    pub fn has_meridians(&self) -> bool {
        matches!(self, GameVersion::V1_4_8 | GameVersion::V1_5_3)
    }
}

impl fmt::Display for GameVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for GameVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "1.2.6" | "v1.2.6" | "126" | "realm_126" => Ok(GameVersion::V1_2_6),
            "1.4.8" | "v1.4.8" | "148" | "realm_148" => Ok(GameVersion::V1_4_8),
            "1.5.3" | "v1.5.3" | "153" | "realm_153" => Ok(GameVersion::V1_5_3),
            other => Err(format!("Versão do jogo desconhecida: '{}'", other)),
        }
    }
}
