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

    /// Habilidades que a classe pode usar **no nível 1**, com o nível de cada uma.
    ///
    /// Fonte: os stubs gerados do `ElementSkill` do client 1.5.5
    /// (`EvolvedPWClient/ElementSkill/skillNNN.h`). Cada stub declara `cls` (a classe
    /// dona), `type` (1 ataque, 2 bênção, 3 maldição, 5 passiva…), `rank`, `max_level` e
    /// uma tabela `GetRequiredLevel` — o primeiro valor dela é o nível exigido para
    /// aprender a habilidade no nível 1. O critério usado aqui, aplicado às 3.317
    /// habilidades do catálogo, é o que separa a árvore da classe do resto:
    /// `cls` igual à classe, `rank == 0`, `max_level == 10`, `GetRequiredLevel[0] == 0`
    /// e uma tabela de SP em que o nível 2 já custa pontos (as inerentes e as de
    /// transformação custam 0 SP em todos os níveis).
    ///
    /// **Achado em 2026-09-07, em jogo**: a lista anterior era um chute e cobrava caro.
    /// Os sacerdotes de teste tinham 11 (passiva, nível 29), 117 (nível 29), 118 (39) e
    /// 119 (49) — o cliente monta a barra com elas e recusa todas, que foi exatamente o
    /// "não é possível usar nenhum skill" relatado. Da lista de antes, só `125`, `113`,
    /// `234`, `235`, `1` e `167` sobreviveram à conferência; `27`, `60`, `61`, `90`,
    /// `190` e `274` eram de outra classe, passivas, ou de nível 9 a 39.
    ///
    /// A 167 (回城术, Portal da Cidade) é `cls = 255` — vale para todas as classes — e
    /// exige nível 1.
    pub fn default_skills(&self) -> Vec<(i16, u8, i16)> {
        let mut skills: Vec<(i16, u8, i16)> = match self {
            // 流水诀 — golpe básico; aceita espada, acha, machado, punhos, magia e mão vazia
            CharacterClass::Blademaster => vec![(1, 1, 0)],
            // 烈火符 — piromancia
            CharacterClass::Wizard => vec![(81, 1, 0)],
            // dupla de ataque de orbe do Espiritualista
            CharacterClass::Psychomancer => vec![(1125, 1, 0), (1126, 1, 0)],
            // 剧毒蛊 — veneno da Feiticeira
            CharacterClass::Venomancer => vec![(299, 1, 0)],
            // 重击 — golpe de martelo do Bárbaro
            CharacterClass::Barbarian => vec![(102, 1, 0)],
            // ataque de adaga do Mercenário
            CharacterClass::Assassin => vec![(1111, 1, 0)],
            // 引而不发 / 连射 — os dois tiros iniciais do Arqueiro
            CharacterClass::Archer => vec![(234, 1, 0), (235, 1, 0)],
            // 羽箭 (ataque) e 清心咒 (cura) — a dupla inicial do Sacerdote
            CharacterClass::Cleric => vec![(125, 1, 0), (113, 1, 0)],
            CharacterClass::Seeker => vec![(1350, 1, 0)],
            CharacterClass::Mystic => vec![(1374, 1, 0), (1381, 1, 0)],
            CharacterClass::Duskblade => vec![(2547, 1, 0)],
            CharacterClass::Stormbringer => vec![(2571, 1, 0)],
        };
        skills.push((167, 1, 0)); // 回城术 (Portal da Cidade) — cls 255, todas as classes
        skills
    }

    /// Tipo maior de arma (`WEAPON_MAJOR_TYPE` do `elements.data`) que as habilidades
    /// iniciais da classe aceitam.
    ///
    /// Não é decoração: `ElementSkill::Condition` (`ElementSkill.cpp:195`) recusa a
    /// conjuração com `if (!ValidWeapon(info.weapon)) return 1;`, e `ValidWeapon` é uma
    /// **lista branca** — o `restrict_weapons` do stub. O cliente passa em `info.weapon`
    /// o `GetDBMajorType()->id` da arma equipada (`EC_HostPlayer.cpp:6146-6153`), ou 0
    /// quando não há arma. Equipar a arma do tipo errado bloqueia **todas** as
    /// habilidades da classe, e foi o que aconteceu em jogo: os sacerdotes estavam com o
    /// "Graveto de Madeira" (2867), tipo maior 5 (Acha), enquanto as habilidades 113 e
    /// 125 só aceitam 292 (Magia) ou 0 (desarmado).
    pub fn weapon_major_type(&self) -> i32 {
        match self {
            CharacterClass::Blademaster | CharacterClass::Seeker => 1, // Espada
            CharacterClass::Barbarian => 9,                            // Machado/Martelo
            CharacterClass::Archer => 13,                              // Longo Alcance
            CharacterClass::Psychomancer => 25333,                     // Orbe
            CharacterClass::Assassin => 23749,                         // Adagas
            CharacterClass::Duskblade => 44878,                        // Sabre
            CharacterClass::Stormbringer => 44879,                     // Foice
            // Magia: Mago, Feiticeira, Sacerdote e Místico
            CharacterClass::Wizard
            | CharacterClass::Venomancer
            | CharacterClass::Cleric
            | CharacterClass::Mystic => 292,
        }
    }

    /// Arma inicial da classe (`WEAPON_ESSENCE.ID` do `elements.data`).
    ///
    /// Cada uma é a arma de `require_level = 1` do [`Self::weapon_major_type`] da classe,
    /// conferida no `elements.data` do realm 155BR — ver o teste
    /// `armas_iniciais_batem_com_o_elements` no `pw-data-loader`.
    pub fn default_weapon_id(&self) -> i32 {
        match self {
            CharacterClass::Blademaster | CharacterClass::Seeker => 2097, // Espada de Madeira
            CharacterClass::Barbarian => 2258,                            // Porrete com Espinhos
            CharacterClass::Archer => 2250,                               // Arco de Madeira
            CharacterClass::Psychomancer => 26332,                        // Pequena Esfera
            CharacterClass::Assassin => 26331,                            // Faca de Limpar Osso
            CharacterClass::Duskblade => 44937,                           // Sabre de Bronze
            CharacterClass::Stormbringer => 45020,                        // Foice de Ferro
            CharacterClass::Wizard
            | CharacterClass::Venomancer
            | CharacterClass::Cleric
            | CharacterClass::Mystic => 2251, // Varinha
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
