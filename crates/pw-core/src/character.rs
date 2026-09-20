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
    /// Os pontos de teleporte já descobertos (`_waypoint_list`, `gs/player_imp.h:2520-2550`).
    /// Vão ao cliente no `WAYPOINT_LIST` (180) da carga inicial.
    pub waypoints: Vec<u16>,
    
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

/// `GP_STATE2_GENDER` (`EC_GPDataType.h:260`) — no servidor original,
/// `STATE_PLAYER_GENDER` (`gs/object.h:202`). **Ligado quer dizer mulher.**
///
/// É por este bit, e só por ele, que o cliente sabe o sexo de outro jogador a partir de um
/// pacote de visão:
///
/// ```cpp
/// unsigned char GetGender() const {
///     return (state2 & GP_STATE2_GENDER) ? GENDER_FEMALE : GENDER_MALE;
/// }
/// ```
///
/// O original o liga em `SetPlayerClass(cls, gender)` (`gs/player_imp.h:1886-1891`).
///
/// Mandar `state2 = 0` para todo mundo, como se fazia até 2026-09-11, é afirmar que todo
/// jogador é homem — e o cliente acredita: duas sacerdotisas voltaram ao campo de visão
/// como modelo masculino, com cabelo e barba de padrão.
pub const ESTADO2_MULHER: i32 = 0x0000_0040;

/// O carimbo da aparência de um personagem — o `custom_crc` do original.
///
/// Viaja em dois lugares que **precisam concordar**: o `crc_c` da `info_player_1` (o
/// pacote que apresenta um jogador) e o `custom_stamp` do `PlayerBaseInfo_Re` (a resposta
/// com a aparência). O cliente compara os dois para decidir se o que ele tem em cache
/// ainda vale (`m_bCustomReady = (m_PlayerInfo.crc_c == Info.crc_c)`,
/// `EC_ElsePlayer.cpp:176`); se discordarem para sempre, ele pede a aparência a cada
/// reaparição, e se forem sempre iguais ele nunca percebe uma troca de visual.
///
/// **Não é o CRC do original** — é um carimbo nosso, e só precisa de duas propriedades:
/// ser estável para os mesmos bytes e mudar quando eles mudam. FNV-1a truncado a 16 bits
/// dá as duas. Aparência vazia carimba zero, que é o que o cliente já espera de quem não
/// tem aparência gravada.
pub fn stamp_de_aparencia(bytes: &[u8]) -> u16 {
    if bytes.is_empty() {
        return 0;
    }
    let mut h: u32 = 0x811c_9dc5;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    // Dobra os 32 bits em 16 para não jogar fora metade da dispersão.
    let dobrado = ((h >> 16) ^ h) as u16;
    // Zero é reservado para "sem aparência": um carimbo real nunca deve colidir com ele.
    if dobrado == 0 { 1 } else { dobrado }
}

/// Os bytes crus da aparência, do `custom_appearance` gravado no banco.
///
/// A coluna guarda ou `{"raw": "<hex>"}` — o caminho normal, os octetos que o cliente
/// mandou na criação — ou um JSON qualquer, de personagem antigo. A mesma extração que o
/// `ProtocolAdapter` já fazia para montar o `RoleInfo`; mora aqui porque o mundo também
/// precisa dela, para carimbar a aparência sem conhecer o formato de rede.
pub fn bytes_da_aparencia(custom_appearance: &serde_json::Value) -> Vec<u8> {
    if let Some(hex_cru) = custom_appearance.get("raw").and_then(|v| v.as_str()) {
        return hex::decode(hex_cru).unwrap_or_default();
    }
    // `null` e `{}` são as duas formas de "este personagem não tem aparência gravada" que
    // o repositório produz: `get_details` escreve `{}` quando a coluna é NULL, e o
    // `CharacterDetails::default` escreve `null`. As duas têm de carimbar **vazio**, o
    // mesmo que o `pw-link` carimba ao responder `custom_data` vazio — senão os dois lados
    // discordam e o cliente pede a aparência a cada reaparição.
    if custom_appearance.is_null() {
        return Vec::new();
    }
    if let Some(obj) = custom_appearance.as_object() {
        if obj.is_empty() {
            return Vec::new();
        }
    }
    serde_json::to_vec(custom_appearance).unwrap_or_default()
}

/// Os quatro atributos com que um personagem nasce.
///
/// No original saem do molde do `clsconfig` do `gamedbd`: **5/5/5/5 para toda classe**, e
/// a vida e a mana de nível 1 = `vit_hp × 5` e `eng_mp × 5`
/// (`pw_data_loader::ptemplate::ATRIBUTO_INICIAL`). Entre 2026-09-09 e 2026-09-16 o projeto
/// os tirava do `ptemplate.conf` (Arqueiro com 20 de energia), que o `gamed` lê mas não
/// aplica a jogador.
///
/// `None` no ponto de criação significa "o realm não trouxe o `ptemplate.conf`"; aí vale
/// o padrão da coluna, que ao menos não é inventado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtributosIniciais {
    pub forca: i32,
    pub agilidade: i32,
    pub vitalidade: i32,
    pub energia: i32,
    /// A vida **máxima** da classe no nível 1, com estes atributos.
    ///
    /// Vem da mesma conta que o mundo usa ao carregar o personagem
    /// (`BaseDaClasse::vida_e_mana_maximas`). Antes a criação gravava uma tabela escrita
    /// no código, e as duas divergiam: o Bárbaro nascia com 260 de vida contra 490 de
    /// máximo — **metade**, e foi o que apareceu em jogo.
    pub vida: i32,
    pub mana: i32,
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
    /// `level` do `WEAPON_ESSENCE` — o **nível da arma** (`weapon_level`), que o original
    /// copia direto (`generate_item_temp.h:329`). É contra ele que o cliente confere a faixa
    /// da munição (`CanUseProjectile`, `EC_HostPlayer.cpp:5014-5016`). O Arco de Madeira
    /// (2250) é nível **0**.
    pub nivel_da_arma: i32,
    pub dano_minimo: i32,
    pub dano_maximo: i32,
    pub dano_magico_minimo: i32,
    pub dano_magico_maximo: i32,
    /// Em *ticks* de 50 ms, como o cliente conta.
    pub velocidade_de_ataque: i32,
    pub alcance: f32,
}

/// O número de escolas mágicas (`NUM_MAGICCLASS`, `EC_RoleTypes.h:38`; `MAGIC_CLASS` no
/// servidor original). Metal, Madeira, Água, Fogo e Terra.
pub const ESCOLAS_MAGICAS: usize = 5;

/// A ficha de uma **armadura**, como o cliente precisa recebê-la no `OWN_ITEM_INFO`.
///
/// Irmã de [`FichaDaArma`], e pelo mesmo motivo mora aqui. Todos os campos saem do
/// `ARMOR_ESSENCE` — ver `pw_data_loader::armaduras`.
///
/// A parte que viaja depois do cabeçalho é `IVTR_ESSENCE_ARMOR`
/// (`EC_IvtrTypes.h:253-260`), 36 bytes, idêntica ao `armor_essence` do servidor original
/// (`gs/item/equip_item.h:150-157`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FichaDaArmadura {
    /// Máscara de classes que podem equipar (`character_combo_id`), um bit por classe.
    ///
    /// **É o campo que importa.** `CanUseEquipment` (`EC_HostPlayer.cpp:4953-4959`) faz,
    /// para `ICID_ARMOR` e `ICID_DECORATION`,
    /// `!(GetProfessionRequirement() & (1 << profissão))` — e sem bloco de dados o
    /// `m_iProfReq` do cliente fica no zero do construtor (`EC_IvtrEquip.cpp:74`), que
    /// recusa **todas** as classes.
    pub classes_permitidas: i32,
    pub nivel_exigido: i16,
    pub forca_exigida: i16,
    pub vitalidade_exigida: i16,
    pub agilidade_exigida: i16,
    pub energia_exigida: i16,
    /// `defense` — a defesa física.
    pub defesa: i32,
    /// `armor` — a evasão que a peça acrescenta (o "grau de armadura" do tooltip).
    pub evasao: i32,
    pub mp_extra: i32,
    pub hp_extra: i32,
    /// `resistance[5]`, na ordem Metal, Madeira, Água, Fogo, Terra.
    pub resistencias: [i32; ESCOLAS_MAGICAS],
}

/// A ficha de um **acessório** (anel, colar, cinto, patuá), do `DECORATION_ESSENCE`.
///
/// O cabeçalho é o mesmo da armadura; o que muda é a essência: `IVTR_ESSENCE_DECORATION`
/// (`EC_IvtrTypes.h:244-251`) troca `mp_enhance`/`hp_enhance` por `damage`/`magic_damage`
/// e os põe **antes** de `defense`. Mesmos 36 bytes, ordem diferente — por isso são dois
/// tipos, e não um com campo sobrando.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FichaDeDecoracao {
    pub classes_permitidas: i32,
    pub nivel_exigido: i16,
    pub forca_exigida: i16,
    pub vitalidade_exigida: i16,
    pub agilidade_exigida: i16,
    pub energia_exigida: i16,
    pub dano: i32,
    pub dano_magico: i32,
    pub defesa: i32,
    pub evasao: i32,
    pub resistencias: [i32; ESCOLAS_MAGICAS],
}

/// A ficha de uma **munição** (flecha, dardo, bala), do `PROJECTILE_ESSENCE`.
///
/// A essência é `IVTR_ESSENCE_ARROW` (`EC_IvtrTypes.h:235-242`), 20 bytes, igual ao
/// `projectile_essence` do servidor (`gs/item/equip_item.h:129-136`) e escrita por
/// `generate_projectile` (`generate_item_temp.h:560-650`). Sem ela o cliente mostra "arma de
/// nível 0-0" e o arco fica vermelho: `CanUseProjectile` compara o nível da arma com
/// `iWeaponReqLow`/`iWeaponReqHigh` desta essência.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FichaDaMunicao {
    /// `type` — o tipo de munição (`PROJECTILE_TYPE`), o mesmo número do
    /// `require_projectile` da arma.
    pub tipo: i32,
    pub dano_extra: i32,
    pub dano_extra_percentual: i32,
    pub nivel_minimo_da_arma: i32,
    pub nivel_maximo_da_arma: i32,
}

/// O que acompanha um item equipável no `OWN_ITEM_INFO` (40).
///
/// Cada família de equipamento tem a sua própria essência, com tamanho e ordem próprios,
/// e o cliente escolhe o leitor pelo **tipo do item no `elements.data` dele** — não por
/// nada que venha na rede. Mandar a essência da família errada é o mesmo que mandar lixo.
///
/// `None` (item que não é equipamento, ou realm sem a tabela) faz o comando ir **sem
/// bloco**, que é o certo: requisito inventado tranca o item no cliente.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FichaDoEquipamento {
    Arma(FichaDaArma),
    Armadura(FichaDaArmadura),
    Decoracao(FichaDeDecoracao),
    Municao(FichaDaMunicao),
}

/// O que um jogador parece, visto de fora — o que vai na `info_player_1`.
///
/// Junta os campos que apresentam um jogador a outro: onde está, para onde olha, se é GM,
/// **se é mulher**, e os dois carimbos que dizem ao cliente se o que ele guardou daquele
/// jogador ainda vale.
///
/// Existe como struct, e não como sete parâmetros, porque os dois codificadores que a
/// usam (`PLAYER_ENTER_WORLD` e `PLAYER_ENTER_SLICE`) escrevem exatamente os mesmos
/// campos na mesma ordem, e um parâmetro trocado de lugar entre eles seria invisível.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VistaDoJogador {
    pub pos: Vector3,
    /// Direção horizontal comprimida em 1/256 de volta. Zerada enquanto a grade espacial
    /// guardar posição e não direção.
    pub dir: u8,
    /// O **nível de cultivo** (`_basic.sec_level` do original, o "period" das missões).
    ///
    /// É o `level2` dos pacotes de visão: o cliente guarda em `m_BasicProps.iLevel2`, tira
    /// dele o título taoista (`CECGameRun::GetLevel2Name`, `EC_GameRun.cpp:3477-3499`) e
    /// toca o efeito de avanço quando muda (`CECPlayer::SetLevel2`, `EC_Player.cpp:7447`).
    /// **Não** é nível de GM — até 2026-09-20 mandávamos o privilégio da conta aqui (B67).
    pub cultivo: u8,
    /// Nível de GM da conta. Acende `STATE_GAMEMASTER` no `state` e põe a coroa sobre o
    /// avatar; não viaja em campo próprio.
    pub sec_level: u8,
    /// **O sexo do personagem.** Vira o bit [`ESTADO2_MULHER`] do `state2`, e é de lá que
    /// o cliente o lê (`info_player_1::GetGender()`).
    pub feminino: bool,
    /// `crc_e` — carimbo do equipamento visível.
    pub crc_equipamento: u16,
    /// `crc_c` — carimbo da aparência. Tem de ser **o mesmo** valor que o `custom_stamp`
    /// do `PlayerBaseInfo_Re` daquele personagem. Ver [`stamp_de_aparencia`].
    pub crc_aparencia: u16,
}
