use pw_core::CharacterClass;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameVersion {
    V1_2_6,
    V1_4_8,
    V1_5_3,
    /// Fontes EvolvedPW (`F:\PW\1.5.5`), a versão base do projeto a partir de 2026-09-02.
    /// "1.5.5" é o nome do pacote da comunidade — o `GAME_VERSION` que o cliente de fato
    /// carrega é `0x00010503` (ver [`Self::server_version_code`]), não deduzido do nome.
    V1_5_5,
}

impl GameVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameVersion::V1_2_6 => "1.2.6",
            GameVersion::V1_4_8 => "1.4.8",
            GameVersion::V1_5_3 => "1.5.3",
            GameVersion::V1_5_5 => "1.5.5",
        }
    }

    /// Código de versão enviado no campo `version` do `Challenge`.
    ///
    /// O cliente compara este número com o `GAME_VERSION` compilado dentro dele e
    /// derruba a conexão **antes de qualquer verificação de senha** se não bater. O
    /// empacotamento é `(major << 24) | (minor << 16) | (release << 8) | patch`.
    ///
    /// Para o 1.5.3 o valor é **`0x00010502`**, não `0x00010503`. Isso não é dedução:
    /// está escrito em `CElementClient/EC_Game.cpp:115` dos fontes do cliente,
    ///
    /// ```text
    /// DWORD GAME_VERSION = ((0 << 24) | (1 << 16) | (5 << 8) | 2);
    /// ```
    ///
    /// ou seja, o cliente que todo mundo chama de "1.5.3" carrega 1.5.**2** no campo de
    /// versão. Deduzir o número a partir do nome da versão é exatamente o erro que
    /// estava aqui.
    ///
    /// O do **1.2.6 deixou de ser palpite**: numa captura de um servidor 1.2.6 real
    /// (item 54), o `Challenge` que o `glinkd` manda traz exatamente `00 01 02 06` neste
    /// campo, nas cinco conexões observadas — e o `gamesys.conf` daquele servidor declara
    /// `version = 10206`. O valor que estava aqui por dedução está certo, agora medido.
    ///
    /// O de **1.4.8 continua não conferido**: não temos cliente nem servidor dessa versão.
    /// Ao obtê-lo, conferir `GAME_VERSION` em `EC_Game.cpp` antes de confiar.
    ///
    /// O do **1.5.5** está em `EvolvedPWClient/ElementClient/EC_Game.cpp:116`:
    /// `((0 << 24) | (1 << 16) | (5 << 8) | 3)` = `0x00010503` — que é, sem ironia
    /// nenhuma, o número que o item acima já tinha descartado como errado para "1.5.3".
    /// A lição de não deduzir da string do nome se prova de novo, numa versão diferente.
    pub fn server_version_code(&self) -> u32 {
        match self {
            GameVersion::V1_2_6 => 0x0001_0206, // medido na captura de 2026-09-01
            GameVersion::V1_4_8 => 0x0001_0408, // não conferido
            GameVersion::V1_5_3 => 0x0001_0502, // EC_Game.cpp:115
            GameVersion::V1_5_5 => 0x0001_0503, // EvolvedPWClient/EC_Game.cpp:116
        }
    }

    /// O opcode do `Response` — o pacote em que o cliente manda usuário e senha.
    ///
    /// **Muda de número entre as versões**, e é o defeito que deixava o cliente 1.2.6
    /// preso em "Conectando ao jogo": ele manda o `Response` como opcode **2**, que na
    /// numeração do 1.5.3 é o `KeyExchange`. O servidor lia uma troca de chaves, não
    /// autenticava ninguém e não respondia nada — sem erro em lugar nenhum, dos dois lados.
    ///
    /// Medido em `docs/HANDSHAKE_DO_126.md`; o do 1.5.3 vem do IR.
    pub fn opcode_response(&self) -> u32 {
        match self {
            GameVersion::V1_2_6 => crate::opcodes::OP_RESPONSE_126,
            // Não medido. Herda o do 1.5.3 porque é o vizinho mais próximo com IR — a
            // mesma política dos layouts de gamedata. Uma captura de um 1.4.8 real decide.
            GameVersion::V1_4_8 => crate::opcodes::OP_RESPONSE_153,
            GameVersion::V1_5_3 => crate::opcodes::OP_RESPONSE_153,
            // Conferido, não herdado: o IR do 1.5.5 tem os 698 protocolos GNET do 1.5.3
            // com o mesmo id, `Response` incluído (2026-09-02, comparação campo a campo
            // dos dois IRs).
            GameVersion::V1_5_5 => crate::opcodes::OP_RESPONSE_153,
        }
    }

    /// O opcode do `KeyExchange`. É o par trocado do [`Self::opcode_response`].
    pub fn opcode_key_exchange(&self) -> u32 {
        match self {
            GameVersion::V1_2_6 => crate::opcodes::OP_KEYEXCHANGE_126,
            GameVersion::V1_4_8 => crate::opcodes::OP_KEYEXCHANGE_153,
            GameVersion::V1_5_3 => crate::opcodes::OP_KEYEXCHANGE_153,
            GameVersion::V1_5_5 => crate::opcodes::OP_KEYEXCHANGE_153,
        }
    }

    /// Quantidade de campos serializados na struct RoleInfo
    pub fn role_info_fields_count(&self) -> usize {
        match self {
            GameVersion::V1_2_6 => 19,
            GameVersion::V1_4_8 => 23,
            GameVersion::V1_5_3 => 23,
            // Não conferido campo a campo contra o RoleInfo do 1.5.5 ainda — herda o do
            // 1.5.3 porque é o mais próximo com IR. Este método hoje não tem chamador
            // (item 53 do docs/ESTADO_E_RETOMADA.md: é código morto), então o valor não
            // muda byte nenhum enquanto continuar assim.
            GameVersion::V1_5_5 => 23,
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
            // Não conferido contra os fontes do 1.5.5 ainda; herda do 1.5.3 (superset
            // conhecido) em vez de restringir por palpite.
            GameVersion::V1_5_5 => true,
        }
    }

    pub fn has_reincarnation(&self) -> bool {
        matches!(self, GameVersion::V1_4_8 | GameVersion::V1_5_3 | GameVersion::V1_5_5)
    }

    pub fn has_meridians(&self) -> bool {
        matches!(self, GameVersion::V1_4_8 | GameVersion::V1_5_3 | GameVersion::V1_5_5)
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
            "1.5.5" | "v1.5.5" | "155" | "realm_155" => Ok(GameVersion::V1_5_5),
            other => Err(format!("Versão do jogo desconhecida: '{}'", other)),
        }
    }
}
