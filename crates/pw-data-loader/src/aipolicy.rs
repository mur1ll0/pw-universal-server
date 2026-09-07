//! Leitor do `aipolicy.data` — as árvores de decisão que governam a IA de monstros e
//! chefes.
//!
//! # Origem do formato
//!
//! Portado de `cgame/gs/ai/policy.cpp`, que é quem lê este arquivo em produção:
//! `CPolicyDataManager::Load` → `CPolicyData::Load` → `CTriggerData::Load` →
//! `ReadConditonTree` / `ReadOperationParam` / `ReadOperationTarget`. Os tipos dos
//! parâmetros vêm de `cgame/gs/ai/policytype.h`, e os enums de condição/operação/alvo de
//! `cgame/gs/ai/policy.h`.
//!
//! **A autoridade é o fonte 1.7.2** (`F:\PW\1.7.2\172Source`), não o 1.5.5
//! (`F:\PW\1.5.5\EvolvedPWServer`) — ver "Por que 1.7.2" abaixo. Os dois têm o mesmo
//! leitor; o 1.7.2 só conhece mais operações.
//!
//! **Não há nada suposto aqui**: cada campo abaixo corresponde a um `fread` do original,
//! na mesma ordem. Onde o original tem ramo por versão, este leitor tem o mesmo ramo.
//!
//! # Tamanhos
//!
//! Os parâmetros de *operação* **não** são autodescritivos no arquivo — o original
//! descobre quantos bytes ler chamando `sizeof(...)` na struct C++. Por isso os tamanhos
//! estão fixados em [`tamanho_do_parametro`], calculados com o alinhamento natural do
//! alvo original (32 bits, `int`/`float`/ponteiro de 4 bytes, `bool` de 1 byte, sem
//! `#pragma pack` em `policytype.h`). Um `bool` no fim de struct de campos de 4 bytes
//! custa 4 (1 + 3 de enchimento) — é o caso de `O_ACTIVE_CONTROLLER`, `O_SET_GLOBAL`,
//! `O_SET_HISTORY` e `O_ACTIVE_CONTROLLER_2`.
//!
//! Os parâmetros de *condição*, ao contrário, trazem o tamanho no fio
//! (`ReadConditonTree` grava `iType` e depois `dat_size`), então tipo desconhecido ali é
//! seguro — vira [`ParametroDeCondicao::Bruto`] em vez de dessincronizar a leitura.
//!
//! # Versões
//!
//! O cabeçalho do arquivo carrega `F_POLICY_EXP_VERSION`. O 1.5.5 grava `1`; o
//! `aipolicy.data` do 1.2.6 que temos grava `0`. O original recusa o que não for igual à
//! sua própria constante — aqui os dois são aceitos, porque o projeto serve os dois
//! realms com o mesmo carregador e a estrutura por baixo é a mesma (conferido lendo os
//! dois arquivos: 1.5.5 = versão 1 com 3144 políticas, 1.2.6 = versão 0 com 293).
//!
//! Cada *trigger* traz a própria versão (`dwVersion`), e é ela que decide o formato dos
//! parâmetros. Os arquivos que temos usam 24 (1.5.5) e 1 (1.2.6), então os dois ramos
//! extremos do original são exercitados de verdade.
//!
//! # Por que 1.7.2, e não 1.5.5
//!
//! O `aipolicy.data` do realm 1.5.5 **não é legível pelo fonte 1.5.5**. Descoberto lendo
//! o arquivo: a política de id 1858 usa a operação **36**, que no
//! `EvolvedPWServer/cgame/gs/ai/policy.h` é `o_num` — o terminador do enum, não uma
//! operação. No `172Source` ela existe e se chama `o_skill_with_talk` (usar habilidade e
//! falar), com um parâmetro que combina `O_USE_SKILL_2` e um texto de tamanho variável —
//! e é exatamente isso que os bytes contêm.
//!
//! Há uma segunda prova independente na mesma direção: os triggers deste arquivo são
//! gravados na versão **24**, e o 1.5.5 declara `F_TRIGGER_VERSION 23`. Na versão 24 o
//! `O_SUMMON_MINE` ganhou um campo (28 → 32 bytes), o que o 1.5.5 não sabe.
//!
//! Ou seja: o pacote de dados do realm é mais novo que o fonte do servidor que temos. O
//! enum do 1.7.2 é **superconjunto exato** do 1.5.5 (os 36 primeiros valores são
//! idênticos, na mesma ordem), então adotá-lo não muda nada do que já funcionava — só
//! acrescenta o que faltava. O `aipolicy.data` do 1.2.6 continua sendo lido pelo mesmo
//! caminho, pelos ramos de versão antiga.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum AiPolicyError {
    #[error("Erro de I/O na leitura do aipolicy.data: {0}")]
    Io(#[from] std::io::Error),
    #[error("aipolicy.data truncado: precisava de {precisava} bytes em {posicao}, só há {restam}")]
    Truncado {
        posicao: usize,
        precisava: usize,
        restam: usize,
    },
    #[error("aipolicy.data com versão de cabeçalho {0} — só 0 (1.2.6) e 1 (1.5.5) são conhecidas")]
    VersaoDeCabecalho(u32),
    #[error("política {id} com versão {versao} — o original só aceita <= 1 (F_POLICY_VERSION)")]
    VersaoDePolitica { id: u32, versao: u32 },
    #[error("contagem absurda em {campo}: {valor}")]
    ContagemAbsurda { campo: &'static str, valor: i32 },
    #[error("condição de tipo {tipo} em {posicao} — o 1.7.2 só declara 0..=46")]
    CondicaoDesconhecida { tipo: i32, posicao: usize },
    #[error("operação de tipo {tipo} em {posicao} — o 1.7.2 só declara 0..=102")]
    OperacaoDesconhecida { tipo: i32, posicao: usize },
    #[error("alvo de tipo {tipo} em {posicao} — o 1.7.2 só declara 0..=20")]
    AlvoDesconhecido { tipo: i32, posicao: usize },
    #[error(
        "parâmetro de {operacao:?} leu {lidos} bytes, mas o sizeof do original é {esperado} — \
         a decodificação campo a campo não bate com policytype.h"
    )]
    TamanhoDeParametro {
        operacao: TipoDeOperacao,
        esperado: usize,
        lidos: usize,
    },
}

pub type Result<T> = std::result::Result<T, AiPolicyError>;

/// `F_POLICY_VERSION` (`ai/policy.h`) — teto da versão de cada `CPolicyData`.
const F_POLICY_VERSION: u32 = 1;

/// Marcadores da árvore de condições (`ai/policy.cpp`, topo do arquivo).
const CONDITION_LEFT_SUB: i32 = 1;
const CONDITION_RIGHT_SUB: i32 = 2;
const CONDITION_LEAF: i32 = 3;
const CONDITION_NODE_END: i32 = 4;

/// Maior valor de condição que o 1.7.2 declara (`c_num = 47`). De 35 a 46 o fonte só tem
/// nomes genéricos (`c_35`…`c_46`), sem struct — ficam sem variante em
/// [`TipoDeCondicao`], mas são aceitos na leitura.
const MAIOR_CONDICAO: i32 = 46;

/// Maior valor de operação que o 1.7.2 declara (`o_num = 103`).
const MAIOR_OPERACAO: i32 = 102;

/// Maior valor de alvo que o 1.7.2 declara (`t_num = 21`).
const MAIOR_ALVO: i32 = 20;

/// Profundidade máxima da árvore de condições. O original não tem limite — recursão em
/// arquivo de dados. Aqui existe para que um arquivo corrompido erre em vez de estourar a
/// pilha.
const PROFUNDIDADE_MAXIMA: usize = 64;

// ---------------------------------------------------------------------------------
// Enums do original (`ai/policy.h`, `CTriggerData`)
// ---------------------------------------------------------------------------------

/// `CTriggerData::_e_condition`. Os operadores (`c_not`/`c_or`/`c_and`, aritmética e
/// comparação) são nós internos da árvore; o resto são folhas com parâmetro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TipoDeCondicao {
    TimeCome = 0,
    HpLess = 1,
    StartAttack = 2,
    Random = 3,
    KillPlayer = 4,
    Not = 5,
    Or = 6,
    And = 7,
    Died = 8,
    Plus = 9,
    Minus = 10,
    Multiply = 11,
    Divide = 12,
    Great = 13,
    Less = 14,
    Equ = 15,
    Var = 16,
    Constant = 17,
    BeHurt = 18,
    ReachEnd = 19,
    AtHistoryStage = 20,
    HistoryValue = 21,
    StopFight = 22,
    LocalVar = 23,
    ReachEnd2 = 24,
    HasFilter = 25,
    RoomIndex = 26,
    PlayerCountInRadius = 27,
    PlayerCountInRegion = 28,
    // Daqui para baixo só existe no 1.7.2.
    GetServertimeDay = 29,
    GetServertimeWeek = 30,
    NotBindCarrier = 31,
    ReachStart = 32,
    CarrierFull = 33,
    TimeCome2 = 34,
}

impl TipoDeCondicao {
    pub fn de_i32(v: i32) -> Option<Self> {
        use TipoDeCondicao::*;
        Some(match v {
            0 => TimeCome, 1 => HpLess, 2 => StartAttack, 3 => Random, 4 => KillPlayer,
            5 => Not, 6 => Or, 7 => And, 8 => Died, 9 => Plus, 10 => Minus,
            11 => Multiply, 12 => Divide, 13 => Great, 14 => Less, 15 => Equ,
            16 => Var, 17 => Constant, 18 => BeHurt, 19 => ReachEnd,
            20 => AtHistoryStage, 21 => HistoryValue, 22 => StopFight, 23 => LocalVar,
            24 => ReachEnd2, 25 => HasFilter, 26 => RoomIndex,
            27 => PlayerCountInRadius, 28 => PlayerCountInRegion,
            29 => GetServertimeDay, 30 => GetServertimeWeek, 31 => NotBindCarrier,
            32 => ReachStart, 33 => CarrierFull, 34 => TimeCome2,
            _ => return None,
        })
    }
}

/// `CTriggerData::_e_operation`.
///
/// Só as operações que este leitor decodifica em campos têm variante. As demais de
/// `0..=102` são válidas no arquivo, lidas pelo tamanho e guardadas cruas — ver
/// [`ParametroDeOperacao::Bruto`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TipoDeOperacao {
    Atacar = 0,
    UsarSkill = 1,
    Falar = 2,
    LimparListaDeOdio = 3,
    RodarTrigger = 4,
    PararTrigger = 5,
    AtivarTrigger = 6,
    CriarTimer = 7,
    MatarTimer = 8,
    Fugir = 9,
    OdioParaPrimeiro = 10,
    OdioParaUltimo = 11,
    OdioCinquentaPorCento = 12,
    PularOperacao = 13,
    AtivarControlador = 14,
    DefinirGlobal = 15,
    RevisarGlobal = 16,
    InvocarMonstro = 17,
    AndarPorCaminho = 18,
    TocarAcao = 19,
    RevisarHistorico = 20,
    DefinirHistorico = 21,
    EntregarPontosPvpDeFaccao = 22,
    CalcularVariavel = 23,
    InvocarMonstro2 = 24,
    AndarPorCaminho2 = 25,
    UsarSkill2 = 26,
    AtivarControlador2 = 27,
    EntregarMissao = 28,
    InvocarMina = 29,
    InvocarNpc = 30,
    EntregarMissaoAleatoriaNaRegiao = 31,
    EntregarMissaoNaListaDeOdio = 32,
    LimparMissaoDeTorreNaRegiao = 33,
    SalvarContagemDeJogadoresNoRaio = 34,
    SalvarContagemDeJogadoresNaRegiao = 35,
    /// Só no 1.7.2 — `o_skill_with_talk`.
    SkillComFala = 36,
    /// Só no 1.7.2 — `o_talk_2`.
    Falar2 = 53,
}

impl TipoDeOperacao {
    pub fn de_i32(v: i32) -> Option<Self> {
        use TipoDeOperacao::*;
        Some(match v {
            0 => Atacar, 1 => UsarSkill, 2 => Falar, 3 => LimparListaDeOdio,
            4 => RodarTrigger, 5 => PararTrigger, 6 => AtivarTrigger, 7 => CriarTimer,
            8 => MatarTimer, 9 => Fugir, 10 => OdioParaPrimeiro, 11 => OdioParaUltimo,
            12 => OdioCinquentaPorCento, 13 => PularOperacao, 14 => AtivarControlador,
            15 => DefinirGlobal, 16 => RevisarGlobal, 17 => InvocarMonstro,
            18 => AndarPorCaminho, 19 => TocarAcao, 20 => RevisarHistorico,
            21 => DefinirHistorico, 22 => EntregarPontosPvpDeFaccao, 23 => CalcularVariavel,
            24 => InvocarMonstro2, 25 => AndarPorCaminho2, 26 => UsarSkill2,
            27 => AtivarControlador2, 28 => EntregarMissao, 29 => InvocarMina,
            30 => InvocarNpc, 31 => EntregarMissaoAleatoriaNaRegiao,
            32 => EntregarMissaoNaListaDeOdio, 33 => LimparMissaoDeTorreNaRegiao,
            34 => SalvarContagemDeJogadoresNoRaio, 35 => SalvarContagemDeJogadoresNaRegiao,
            36 => SkillComFala, 53 => Falar2,
            _ => return None,
        })
    }
}

/// Tamanho do parâmetro de uma operação, em bytes — `CTriggerData::GetOperationParamSize`
/// do 1.7.2, com as structs de `policytype.h` medidas no alvo de 32 bits.
///
/// `None` = tipo fora de `0..=102`, que o original também não saberia ler.
///
/// Os ramos por versão são os mesmos do `ReadOperationParam`: antes da versão 1 o
/// `o_active_controller` não tinha `bStop`; antes da 4 o `o_set_global` não tinha
/// `bIsValue`; o `o_play_action` cresceu duas vezes (136 → 140 na versão 9, → 144 na 38);
/// e o `o_summon_mine` cresceu de 28 para 32 na versão 24. O `o_summon_monster` também
/// mudou na versão 7, mas de layout, não de tamanho (24 nos dois).
///
/// Os três tipos de tamanho variável (2 `o_talk`, 36 `o_skill_with_talk`, 53 `o_talk_2`)
/// devolvem o `sizeof` da struct C++, que **não** é quanto se lê do arquivo — eles têm
/// leitor próprio e nunca chegam ao caminho genérico. O que importa deles aqui é só não
/// ser zero, que é o que faz o original entrar no `switch` em vez de pular o parâmetro.
fn tamanho_do_parametro(tipo: i32, versao: u32) -> Option<usize> {
    Some(match tipo {
        14 if versao < 1 => 4,    // O_ACTIVE_CONTROLLER sem bStop
        15 if versao < 4 => 8,    // O_SET_GLOBAL_VERSION3
        19 if versao < 9 => 136,  // O_PLAY_ACTION_VERSION8
        19 if versao < 38 => 140, // O_PLAY_ACTION_VERSION37
        29 if versao < 24 => 28,  // O_SUMMON_MINE_VERSION_23
        0 => 4,     // O_ATTACK_TYPE
        1 => 8,     // O_USE_SKILL
        2 => 12,    // O_TALK_TEXT (variável no arquivo)
        4 => 4,     // O_RUN_TRIGGER
        5 => 4,     // O_STOP_TRIGGER
        6 => 4,     // O_ACTIVE_TRIGGER
        7 => 12,    // O_CREATE_TIMER
        8 => 4,     // O_KILL_TIMER
        14 => 8,    // O_ACTIVE_CONTROLLER
        15 => 12,   // O_SET_GLOBAL
        16 => 8,    // O_REVISE_GLOBAL
        17 => 24,   // O_SUMMON_MONSTER
        18 => 16,   // O_WALK_ALONG
        19 => 144,  // O_PLAY_ACTION
        20 => 8,    // O_REVISE_HISTORY
        21 => 12,   // O_SET_HISTORY
        22 => 4,    // O_DELIVER_FACTION_PVP_POINTS
        23 => 28,   // O_CALC_VAR
        24 => 36,   // O_SUMMON_MONSTER_2
        25 => 20,   // O_WALK_ALONG_2
        26 => 16,   // O_USE_SKILL_2
        27 => 12,   // O_ACTIVE_CONTROLLER_2
        28 => 8,    // O_DELIVER_TASK
        29 => 32,   // O_SUMMON_MINE
        30 => 36,   // O_SUMMON_NPC
        31 => 28,   // O_DELIVER_RANDOM_TASK_IN_REGION
        32 => 16,   // O_DELIVER_TASK_IN_HATE_LIST
        33 => 24,   // O_CLEAR_TOWER_TASK_IN_REGION
        34 => 16,   // O_SAVE_PLAYER_COUNT_IN_RADIUS_TO_PARAM
        35 => 32,   // O_SAVE_PLAYER_COUNT_IN_REGION_TO_PARAM
        36 => 28,   // O_SKILL_WITH_TALK (variável no arquivo)
        37 => 24,   // O_USE_SKILL_3
        38 => 16,   // O_SORT_NUM
        39 => 24,   // O_GET_POS_NUM
        40 => 4,    // O_AUTO_BIND_CARRIER
        41 => 8,    // O_ADD_RANGE_TO_HATE_LIST
        42 => 16,   // O_SAVE_ALIVE_PLAYER_COUNT_IN_RADIUS_TO_PARAM
        43 => 32,   // O_SAVE_ALIVE_PLAYER_COUNT_IN_REGION_TO_PARAM
        44 => 28,   // O_WALK_ALONG_3
        45 => 48,   // O_WALK_ALONG_4
        46 => 12,   // O_SAVE_TIME
        47 => 16,   // O_RANDOM_ASSIGNMENT
        48 => 8,    // O_CARRIER_VOTING
        49 => 24,   // O_VOTING_RESULT
        50 => 68,   // O_VOTING_SHOW
        51 => 8,    // O_CARRIER_DELIVERY_TASK
        52 => 4,    // O_CARRIER_NOENTRY
        53 => 28,   // O_TALK_TEXT_2 (variável no arquivo)
        54 => 24,   // O_CREATE_TIMER_2
        55 => 8,    // O_KILL_TIMER_2
        56 => 4,    // O_CHANGE_MONSTER_FIGHTING_STATE
        57 => 4,    // O_CHANGE_MONSTER_ACTIVE_PASSIVE
        58 => 8,    // O_CHILD_CARRIER_PARENT_MONSTER
        59 => 4,    // O_CLOSE_CHILD_MONSTER
        60 => 4,    // O_DELIVER_HATE_TARGETS
        61 => 4,    // O_CHANGE_MONSTER_ATTACK_POLICY
        62 => 8,    // O_SPECIFY_FAILED_TASK_ID
        63 => 32,   // O_SPECIFY_FAILED_TASK_ID_REGIONAL
        // De 64 a 102 o 1.7.2 só nomeia por número (`o_64`…`o_102`), sem comentário nem
        // uso no gs — mas o tamanho existe, e é ele que mantém o arquivo em sincronia.
        64 => 84, 65 => 44, 66 => 32, 67 => 28, 68 => 12, 69 => 8, 70 => 4, 71 => 8,
        73 => 8, 74 => 68, 75 => 4, 76 => 16, 77 => 24, 78 => 40, 79 => 8, 80 => 24,
        81 => 24, 82 => 28, 83 => 44, 85 => 28, 86 => 12, 87 => 44, 88 => 12, 89 => 40,
        90 => 16, 91 => 24, 92 => 36, 93 => 40, 94 => 1, 95 => 1, 96 => 8, 98 => 4,
        99 => 80, 100 => 8, 101 => 8, 102 => 8,
        // Sem parâmetro nenhum: 3, 9..13 (as ações imediatas), 72, 84 e 97.
        3 | 9..=13 | 72 | 84 | 97 => 0,
        _ => return None,
    })
}

/// `CTriggerData::_e_target` — quem a operação atinge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TipoDeAlvo {
    OdioPrimeiro = 0,
    OdioSegundo = 1,
    OdioOutros = 2,
    MaiorHp = 3,
    MaiorMp = 4,
    MenorHp = 5,
    ListaDeProfissoes = 6,
    EuMesmo = 7,
    QuemMatouOMonstro = 8,
    FaccaoDoLocalDeNascimento = 9,
    OdioAleatorio = 10,
    OdioMaisProximo = 11,
    OdioMaisDistante = 12,
    OdioPrimeiroRedirecionado = 13,
    // Só no 1.7.2.
    AleatorioNoAlcance = 14,
    MaisProximoNoAlcance = 15,
    MaisDistanteNoAlcance = 16,
}

impl TipoDeAlvo {
    pub fn de_i32(v: i32) -> Option<Self> {
        use TipoDeAlvo::*;
        Some(match v {
            0 => OdioPrimeiro, 1 => OdioSegundo, 2 => OdioOutros, 3 => MaiorHp,
            4 => MaiorMp, 5 => MenorHp, 6 => ListaDeProfissoes, 7 => EuMesmo,
            8 => QuemMatouOMonstro, 9 => FaccaoDoLocalDeNascimento, 10 => OdioAleatorio,
            11 => OdioMaisProximo, 12 => OdioMaisDistante, 13 => OdioPrimeiroRedirecionado,
            14 => AleatorioNoAlcance, 15 => MaisProximoNoAlcance, 16 => MaisDistanteNoAlcance,
            _ => return None,
        })
    }
}

/// `CTriggerData::ReadOperationTarget` do 1.7.2, que ganhou parâmetro em mais tipos que o
/// 1.5.5 (lá só `t_occupation_list` tinha):
///
/// ```text
/// 6            -> T_OCCUPATION, 4 bytes
/// 14, 15, 16   -> 4 bytes  (os "…_in_range", que carregam o alcance)
/// 18           -> 12 bytes
/// 20           -> 16 bytes
/// resto        -> nenhum
/// ```
///
/// Recebe o tipo cru porque 17 a 20 não têm nome no fonte e mesmo assim ocupam bytes —
/// ignorá-los desalinharia o arquivo inteiro.
fn tamanho_do_parametro_de_alvo(tipo_cru: i32) -> usize {
    match tipo_cru {
        6 | 14 | 15 | 16 => 4,
        18 => 12,
        20 => 16,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------------
// Modelo carregado
// ---------------------------------------------------------------------------------

/// `POLICY_ZONE_VERT` (`policytype.h`) — 3 floats, 12 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct VerticeDeZona {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Parâmetro de uma condição. O tamanho vem no fio, então tipo desconhecido não quebra a
/// leitura — cai em [`ParametroDeCondicao::Bruto`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParametroDeCondicao {
    Nenhum,
    /// `C_TIME_COME { unsigned int uID }`
    IdDeTimer(u32),
    /// `C_HP_LESS { float fPercent }`
    HpAbaixoDe(f32),
    /// `C_RANDOM { float fProbability }`
    Probabilidade(f32),
    /// `C_VAR` / `C_LOCAL_VAR` / `C_HISTORY_STAGE` / `C_HAS_FILTER` — todos `{ int iID }`
    Id(i32),
    /// `C_CONSTANT { int iValue }` / `C_HISTORY_VALUE { int iValue }`
    Valor(i32),
    /// `C_BE_HURT { int iHurtLow; int iHurtHigh }`
    FaixaDeDano { minimo: i32, maximo: i32 },
    /// `C_REACH_END { int iPathID }`
    Caminho(i32),
    /// `C_REACH_END_2 { int iPathID; int iPathIDType }`
    Caminho2 { caminho: i32, tipo_do_caminho: i32 },
    /// `C_PLAYER_COUNT_IN_RADIUS { float fRadius }`
    Raio(f32),
    /// `C_PLAYER_COUNT_IN_REGION { POLICY_ZONE_VERT zvMin, zvMax }`
    Regiao { minimo: VerticeDeZona, maximo: VerticeDeZona },
    /// Tipo sem struct conhecida — os bytes ficam guardados como vieram.
    Bruto(Vec<u8>),
}

/// Parâmetro de uma operação.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParametroDeOperacao {
    Nenhum,
    /// `O_ATTACK_TYPE { uType }` — 0 corpo a corpo, 1 físico à distância, 2 mágico,
    /// 3 corpo a corpo à distância (comentário do `_e_operation`).
    TipoDeAtaque(u32),
    /// `O_USE_SKILL { uSkill, uLevel }`
    Skill { skill: u32, nivel: u32 },
    /// `O_TALK_TEXT` — o texto vem em UTF-16 (`policy_char`), com a máscara de dados
    /// anexos só a partir da versão 17 do trigger.
    Fala { texto: String, mascara_de_anexos: u32 },
    /// `O_RUN_TRIGGER` / `O_STOP_TRIGGER` / `O_ACTIVE_TRIGGER` / `O_KILL_TIMER`
    IdDeTrigger(u32),
    /// `O_CREATE_TIMER { uID, uPeriod, uCounter }`
    Timer { id: u32, periodo: u32, contador: u32 },
    /// `O_ACTIVE_CONTROLLER { uID, bStop }`
    Controlador { id: u32, parar: bool },
    /// `O_SET_GLOBAL { iID, iValue, bIsValue }`
    DefinirGlobal { id: i32, valor: i32, e_valor_direto: bool },
    /// `O_REVISE_GLOBAL` / `O_REVISE_HISTORY` — `{ iID, iValue }`
    AjustarVariavel { id: i32, valor: i32 },
    /// `O_SET_HISTORY { iID, iValue, bIsHistoryValue }`
    DefinirHistorico { id: i32, valor: i32, valor_e_do_historico: bool },
    /// `O_SUMMON_MONSTER { iMonsterID, iRange, iLife, iDispearCondition, iPathID, iMonsterNum }`
    InvocarMonstro {
        monstro: i32,
        alcance: i32,
        vida_em_segundos: i32,
        condicao_de_sumico: i32,
        caminho: i32,
        quantidade: i32,
    },
    /// `O_WALK_ALONG { iWorldID, iPathID, iPatrolType, iSpeedType }`
    AndarPorCaminho { mundo: i32, caminho: i32, tipo_de_patrulha: i32, tipo_de_velocidade: i32 },
    /// `O_PLAY_ACTION { szActionName[128], iLoopCount, iInterval, iPlayTime }`
    TocarAcao { acao: String, repeticoes: i32, intervalo: i32, duracao: i32 },
    /// `O_DELIVER_FACTION_PVP_POINTS { uType }`
    PontosPvpDeFaccao(u32),
    /// `O_CALC_VAR` — 7 inteiros, ver `policytype.h`.
    CalcularVariavel {
        destino: i32,
        tipo_do_destino: i32,
        origem1: i32,
        tipo_da_origem1: i32,
        operador: i32,
        origem2: i32,
        tipo_da_origem2: i32,
    },
    /// `O_SUMMON_MONSTER_2` — 9 inteiros.
    InvocarMonstro2 {
        condicao_de_sumico: i32,
        monstro: i32,
        tipo_do_monstro: i32,
        alcance: i32,
        vida_em_segundos: i32,
        caminho: i32,
        tipo_do_caminho: i32,
        quantidade: i32,
        tipo_da_quantidade: i32,
    },
    /// `O_WALK_ALONG_2` — 5 inteiros.
    AndarPorCaminho2 {
        mundo: i32,
        caminho: i32,
        tipo_do_caminho: i32,
        tipo_de_patrulha: i32,
        tipo_de_velocidade: i32,
    },
    /// `O_USE_SKILL_2 { uSkill, uSkillType, uLevel, uLevelType }`
    Skill2 { skill: u32, tipo_da_skill: u32, nivel: u32, tipo_do_nivel: u32 },
    /// `O_ACTIVE_CONTROLLER_2 { uID, uIDType, bStop }`
    Controlador2 { id: u32, tipo_do_id: u32, parar: bool },
    /// `O_DELIVER_TASK { uID, uIDType }`
    EntregarMissao { id: u32, tipo_do_id: u32 },
    /// `O_SUMMON_MINE` — 7 inteiros até a versão 23, mais `bSetOwnerAsMonsterOwner` a
    /// partir da 24.
    InvocarMina {
        tipo_da_vida: i32,
        mina: i32,
        tipo_da_mina: i32,
        alcance: i32,
        vida_em_segundos: i32,
        quantidade: i32,
        tipo_da_quantidade: i32,
        dono_e_o_dono_do_monstro: Option<bool>,
    },
    /// `O_SUMMON_NPC` — 9 inteiros.
    InvocarNpc {
        tipo_da_vida: i32,
        npc: i32,
        tipo_do_npc: i32,
        alcance: i32,
        vida_em_segundos: i32,
        caminho: i32,
        tipo_do_caminho: i32,
        quantidade: i32,
        tipo_da_quantidade: i32,
    },
    /// `O_DELIVER_RANDOM_TASK_IN_REGION { uID, zvMin, zvMax }`
    MissaoAleatoriaNaRegiao { id: u32, minimo: VerticeDeZona, maximo: VerticeDeZona },
    /// `O_DELIVER_TASK_IN_HATE_LIST { uID, uIDType, iRange, iPlayerNum }`
    MissaoNaListaDeOdio { id: u32, tipo_do_id: u32, alcance: i32, jogadores: i32 },
    /// `O_CLEAR_TOWER_TASK_IN_REGION { zvMin, zvMax }`
    LimparMissaoDeTorre { minimo: VerticeDeZona, maximo: VerticeDeZona },
    /// `O_SAVE_PLAYER_COUNT_IN_RADIUS_TO_PARAM`
    ContarJogadoresNoRaio { raio: f32, tipo_do_raio: u32, destino: i32, tipo_do_destino: u32 },
    /// `O_SAVE_PLAYER_COUNT_IN_REGION_TO_PARAM`
    ContarJogadoresNaRegiao {
        minimo: VerticeDeZona,
        maximo: VerticeDeZona,
        destino: i32,
        tipo_do_destino: u32,
    },
    /// `O_SKILL_WITH_TALK { O_USE_SKILL_2 skill; O_TALK_TEXT talk }` (1.7.2) — usar a
    /// habilidade e falar no mesmo passo. É a operação que provou que o `aipolicy.data`
    /// do realm é mais novo que o fonte 1.5.5.
    SkillComFala {
        skill: u32,
        tipo_da_skill: u32,
        nivel: u32,
        tipo_do_nivel: u32,
        texto: String,
        mascara_de_anexos: u32,
    },
    /// `O_TALK_TEXT_2` (1.7.2) — falar escolhendo canal.
    Fala2 {
        texto: String,
        mascara_de_anexos: u32,
        tipos_de_canal: u32,
        tipo_dos_tipos_de_canal: u32,
        canal: u32,
        tipo_do_canal: u32,
    },
    /// Operação válida no arquivo (`0..=102`) que este leitor ainda não decodifica em
    /// campos. Os bytes vêm do tamanho que o original usaria, então nada desalinha — só
    /// falta dar nome aos campos quando alguma delas virar prioridade.
    Bruto { tipo: i32, bytes: Vec<u8> },
}

/// Um nó da árvore de condições. Nós internos (`Not`/`Or`/`And`, aritmética, comparação)
/// usam `esquerda`/`direita`; folhas carregam o parâmetro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoDeCondicao {
    /// `None` para os tipos de 35 a 46, que o 1.7.2 declara mas não nomeia.
    pub tipo: Option<TipoDeCondicao>,
    pub tipo_cru: i32,
    pub parametro: ParametroDeCondicao,
    pub esquerda: Option<Box<NoDeCondicao>>,
    pub direita: Option<Box<NoDeCondicao>>,
}

/// Alvo de uma operação (`_s_target`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlvoDaOperacao {
    /// `None` para os tipos de 17 a 20, que o 1.7.2 declara mas não nomeia.
    pub tipo: Option<TipoDeAlvo>,
    pub tipo_cru: i32,
    /// Bytes do parâmetro do alvo, do tamanho que [`tamanho_do_parametro_de_alvo`] dita —
    /// vazio para a maioria dos tipos. Fica cru porque só um deles tem struct nomeada no
    /// fonte (`T_OCCUPATION`); ver [`Self::mascara_de_profissoes`].
    pub parametro: Vec<u8>,
}

impl AlvoDaOperacao {
    /// `T_OCCUPATION { uBit }` quando o alvo é [`TipoDeAlvo::ListaDeProfissoes`]. Os bits
    /// são `_e_occupation` (`policy.h`): bit 0 = Guerreiro, 1 = Mago, …, 11 = Tormentador.
    pub fn mascara_de_profissoes(&self) -> Option<u32> {
        if self.tipo != Some(TipoDeAlvo::ListaDeProfissoes) || self.parametro.len() != 4 {
            return None;
        }
        Some(u32::from_le_bytes(self.parametro[..4].try_into().unwrap()))
    }
}

/// Uma operação: o que fazer, em quem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operacao {
    /// `None` para as operações de `0..=102` sem variante nomeada — o parâmetro delas
    /// está em [`ParametroDeOperacao::Bruto`].
    pub tipo: Option<TipoDeOperacao>,
    pub tipo_cru: i32,
    pub parametro: ParametroDeOperacao,
    pub alvo: AlvoDaOperacao,
}

/// `CTriggerData` — uma regra: uma árvore de condições e a lista de operações que roda
/// quando a árvore dá verdadeiro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    /// A versão gravada no próprio trigger, que decide o formato dos parâmetros.
    pub versao: u32,
    pub id: u32,
    /// `bActive` — começa habilitado.
    pub ativo: bool,
    /// `bRun` — já em execução ao carregar.
    pub rodando: bool,
    /// `bAttackValid` — só vale enquanto o monstro está em combate.
    pub so_em_combate: bool,
    pub nome: String,
    pub condicao: Option<NoDeCondicao>,
    pub operacoes: Vec<Operacao>,
}

/// `CPolicyData` — a política inteira de um monstro, apontada por
/// `MonsterTemplate::aipolicy_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiPolicy {
    pub id: u32,
    pub versao: u32,
    pub triggers: Vec<Trigger>,
}

/// `CPolicyDataManager` — o arquivo inteiro.
#[derive(Debug, Clone, Default)]
pub struct AiPolicyData {
    /// `F_POLICY_EXP_VERSION` do cabeçalho: 1 no 1.5.5, 0 no 1.2.6.
    pub versao: u32,
    pub policies: HashMap<u32, AiPolicy>,
    /// Políticas cujo id se repetiu no arquivo. O original guarda uma lista, não um mapa,
    /// e nunca checa duplicata; aqui o mapa venceria em silêncio, então a contagem fica
    /// registrada para o carregamento poder dizer que perdeu alguma.
    pub ids_repetidos: Vec<u32>,
}

/// Cursor de leitura com verificação de limite — o original usa `fread` sem checar
/// retorno, e um arquivo truncado vira lixo silencioso lá.
struct Leitor<'a> {
    dados: &'a [u8],
    pos: usize,
}

impl<'a> Leitor<'a> {
    fn novo(dados: &'a [u8]) -> Self {
        Self { dados, pos: 0 }
    }

    fn fatia(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.dados.len() - self.pos < n {
            return Err(AiPolicyError::Truncado {
                posicao: self.pos,
                precisava: n,
                restam: self.dados.len() - self.pos,
            });
        }
        let s = &self.dados[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.fatia(4)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.fatia(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.fatia(4)?.try_into().unwrap()))
    }

    fn bool(&mut self) -> Result<bool> {
        Ok(self.fatia(1)?[0] != 0)
    }

    /// Um `bool` seguido do enchimento que o compilador põe para voltar a 4 bytes — é
    /// assim que `O_ACTIVE_CONTROLLER` e companhia chegam ao disco.
    fn bool_alinhado(&mut self) -> Result<bool> {
        let b = self.bool()?;
        self.fatia(3)?;
        Ok(b)
    }

    fn zona(&mut self) -> Result<VerticeDeZona> {
        Ok(VerticeDeZona { x: self.f32()?, y: self.f32()?, z: self.f32()? })
    }

    /// `char[n]` de tamanho fixo, cortado no primeiro NUL. O conteúdo é GBK no arquivo
    /// original (nomes em chinês), então bytes altos viram `U+FFFD` em vez de erro — o
    /// nome é rótulo de editor, não entra em nenhuma decisão de jogo.
    fn texto_fixo(&mut self, n: usize) -> Result<String> {
        let bruto = self.fatia(n)?;
        let fim = bruto.iter().position(|&b| b == 0).unwrap_or(n);
        Ok(String::from_utf8_lossy(&bruto[..fim]).into_owned())
    }

    /// Um texto no formato de `O_TALK_TEXT`: tamanho em bytes, texto UTF-16 e — só a
    /// partir da versão 17 do trigger — a máscara de dados anexos.
    fn fala(&mut self, versao: u32) -> Result<(String, u32)> {
        let tamanho = self.u32()? as usize;
        let bruto = self.fatia(tamanho)?;
        let unidades: Vec<u16> = bruto
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let texto = String::from_utf16_lossy(&unidades).trim_end_matches('\0').to_string();
        let mascara = if versao < 17 { 0 } else { self.u32()? };
        Ok((texto, mascara))
    }
}

impl AiPolicyData {
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut l = Leitor::novo(data);

        let versao = l.u32()?;
        if versao > 1 {
            return Err(AiPolicyError::VersaoDeCabecalho(versao));
        }
        let quantidade = l.i32()?;
        if !(0..=1_000_000).contains(&quantidade) {
            return Err(AiPolicyError::ContagemAbsurda { campo: "políticas", valor: quantidade });
        }

        let mut policies = HashMap::with_capacity(quantidade as usize);
        let mut ids_repetidos = Vec::new();

        for _ in 0..quantidade {
            let policy = Self::ler_politica(&mut l)?;
            let id = policy.id;
            if policies.insert(id, policy).is_some() {
                ids_repetidos.push(id);
            }
        }
        if !ids_repetidos.is_empty() {
            warn!(
                "aipolicy.data: {} política(s) com id repetido — a última de cada id venceu: {:?}",
                ids_repetidos.len(),
                &ids_repetidos[..ids_repetidos.len().min(10)]
            );
        }

        let sobra = data.len() - l.pos;
        if sobra != 0 {
            warn!("aipolicy.data: {sobra} byte(s) ignorados depois da última política");
        }

        info!(
            "aipolicy.data carregado (versão {versao}): {} políticas, {} triggers",
            policies.len(),
            policies.values().map(|p| p.triggers.len()).sum::<usize>()
        );

        Ok(Self { versao, policies, ids_repetidos })
    }

    /// `CPolicyData::Load(FILE*)`.
    fn ler_politica(l: &mut Leitor) -> Result<AiPolicy> {
        let versao = l.u32()?;
        let id = l.u32()?;
        if versao > F_POLICY_VERSION {
            return Err(AiPolicyError::VersaoDePolitica { id, versao });
        }
        let quantidade = l.i32()?;
        if !(0..=100_000).contains(&quantidade) {
            return Err(AiPolicyError::ContagemAbsurda { campo: "triggers", valor: quantidade });
        }

        let mut triggers = Vec::with_capacity(quantidade as usize);
        for _ in 0..quantidade {
            triggers.push(Self::ler_trigger(l)?);
        }

        Ok(AiPolicy { id, versao, triggers })
    }

    /// `CTriggerData::Load`. Os cinco ramos do original (`0`, `1`, `2`, `3`, `>= 4`) só
    /// diferem em **quais** operações têm parâmetro lido; o cabeçalho e a árvore são
    /// idênticos. Aqui isso vira uma pergunta só — [`Self::le_parametro_nesta_versao`].
    fn ler_trigger(l: &mut Leitor) -> Result<Trigger> {
        let versao = l.u32()?;
        let id = l.u32()?;
        // Três `fread` de 1 byte, sem enchimento entre eles.
        let ativo = l.bool()?;
        let rodando = l.bool()?;
        let so_em_combate = l.bool()?;
        let nome = l.texto_fixo(128)?;

        let condicao = Self::ler_arvore_de_condicoes(l, 0)?;

        let quantidade = l.i32()?;
        if !(0..=100_000).contains(&quantidade) {
            return Err(AiPolicyError::ContagemAbsurda { campo: "operações", valor: quantidade });
        }

        let mut operacoes = Vec::with_capacity(quantidade as usize);
        for _ in 0..quantidade {
            let posicao_do_tipo = l.pos;
            let tipo_cru = l.i32()?;
            if !(0..=MAIOR_OPERACAO).contains(&tipo_cru) {
                return Err(AiPolicyError::OperacaoDesconhecida {
                    tipo: tipo_cru,
                    posicao: posicao_do_tipo,
                });
            }

            let parametro = if Self::le_parametro_nesta_versao(tipo_cru, versao) {
                Self::ler_parametro_de_operacao(l, tipo_cru, versao)?
            } else {
                // Nesta versão o `switch` do original não chama `ReadOperationParam` para
                // este tipo: ele grava `pParam = 0` e não consome nada.
                ParametroDeOperacao::Nenhum
            };

            let posicao_do_alvo = l.pos;
            let tipo_cru_do_alvo = l.i32()?;
            if !(0..=MAIOR_ALVO).contains(&tipo_cru_do_alvo) {
                return Err(AiPolicyError::AlvoDesconhecido {
                    tipo: tipo_cru_do_alvo,
                    posicao: posicao_do_alvo,
                });
            }
            let n = tamanho_do_parametro_de_alvo(tipo_cru_do_alvo);
            let parametro_do_alvo = l.fatia(n)?.to_vec();

            operacoes.push(Operacao {
                tipo: TipoDeOperacao::de_i32(tipo_cru),
                tipo_cru,
                parametro,
                alvo: AlvoDaOperacao {
                    tipo: TipoDeAlvo::de_i32(tipo_cru_do_alvo),
                    tipo_cru: tipo_cru_do_alvo,
                    parametro: parametro_do_alvo,
                },
            });
        }

        Ok(Trigger { versao, id, ativo, rodando, so_em_combate, nome, condicao, operacoes })
    }

    /// O `switch` que decide se `ReadOperationParam` é chamado, por versão de trigger.
    ///
    /// - versões 0 e 1: só os nove tipos até `o_active_controller`;
    /// - versões 2 e 3: os mesmos nove mais `o_set_global` e `o_revise_global`;
    /// - versão >= 4: todos (o `switch` some, `ReadOperationParam` é chamado direto).
    fn le_parametro_nesta_versao(tipo_cru: i32, versao: u32) -> bool {
        if versao >= 4 {
            return true;
        }
        // 0 o_attact, 1 o_use_skill, 2 o_talk, 4..6 os triggers, 7 o_create_timer,
        // 8 o_kill_timer, 14 o_active_controller.
        matches!(tipo_cru, 0 | 1 | 2 | 4..=8 | 14)
            // 15 o_set_global e 16 o_revise_global entram a partir da versão 2.
            || (versao >= 2 && matches!(tipo_cru, 15 | 16))
    }

    /// `CTriggerData::ReadOperationParam`, com os mesmos ramos por versão.
    fn ler_parametro_de_operacao(
        l: &mut Leitor,
        tipo_cru: i32,
        versao: u32,
    ) -> Result<ParametroDeOperacao> {
        use TipoDeOperacao::*;

        let Some(esperado) = tamanho_do_parametro(tipo_cru, versao) else {
            return Err(AiPolicyError::OperacaoDesconhecida { tipo: tipo_cru, posicao: l.pos });
        };
        if esperado == 0 {
            // `GetOperationParamSize` devolveu 0: o original grava `pParam = 0` e não lê
            // nada. É o caso das ações imediatas (fugir, limpar ódio, …).
            return Ok(ParametroDeOperacao::Nenhum);
        }

        let tipo = TipoDeOperacao::de_i32(tipo_cru);

        // Os três tipos de tamanho variável, que o original lê fora do `fread` genérico.
        match tipo {
            Some(Falar) => {
                let (texto, mascara_de_anexos) = l.fala(versao)?;
                return Ok(ParametroDeOperacao::Fala { texto, mascara_de_anexos });
            }
            Some(SkillComFala) => {
                // `O_SKILL_WITH_TALK { O_USE_SKILL_2 skill; O_TALK_TEXT talk }` — os
                // quatro inteiros da habilidade e depois a fala, de tamanho variável.
                let skill = l.u32()?;
                let tipo_da_skill = l.u32()?;
                let nivel = l.u32()?;
                let tipo_do_nivel = l.u32()?;
                let (texto, mascara_de_anexos) = l.fala(versao)?;
                return Ok(ParametroDeOperacao::SkillComFala {
                    skill,
                    tipo_da_skill,
                    nivel,
                    tipo_do_nivel,
                    texto,
                    mascara_de_anexos,
                });
            }
            Some(Falar2) => {
                // `O_TALK_TEXT_2 { O_TALK_TEXT talk; 4 × unsigned int }` — a fala primeiro,
                // depois o canal.
                let (texto, mascara_de_anexos) = l.fala(versao)?;
                return Ok(ParametroDeOperacao::Fala2 {
                    texto,
                    mascara_de_anexos,
                    tipos_de_canal: l.u32()?,
                    tipo_dos_tipos_de_canal: l.u32()?,
                    canal: l.u32()?,
                    tipo_do_canal: l.u32()?,
                });
            }
            _ => {}
        }

        // Os ramos por versão que trocam o layout (o tamanho já veio certo da tabela).
        match tipo {
            Some(AtivarControlador) if versao < 1 => {
                // Antes da versão 1 só existia o id; `bStop` era assumido falso.
                return Ok(ParametroDeOperacao::Controlador { id: l.u32()?, parar: false });
            }
            Some(DefinirGlobal) if versao < 4 => {
                // `O_SET_GLOBAL_VERSION3 { iID, iValue }` — sem o `bIsValue`, que o
                // original preenche com `true`.
                let id = l.i32()?;
                let valor = l.i32()?;
                return Ok(ParametroDeOperacao::DefinirGlobal { id, valor, e_valor_direto: true });
            }
            Some(InvocarMonstro) if versao < 7 => {
                // `O_SUMMON_MONSTER_VERSION6`: dois `bool` no meio, no lugar do
                // `iDispearCondition` que veio depois. Mesmo tamanho, layout diferente.
                let monstro = l.i32()?;
                let alcance = l.i32()?;
                let vida_em_segundos = l.i32()?;
                let _sumir = l.bool()?;
                let _usa_alvo_da_politica = l.bool()?;
                l.fatia(2)?; // enchimento até o próximo `int`
                let caminho = l.i32()?;
                let quantidade = l.i32()?;
                return Ok(ParametroDeOperacao::InvocarMonstro {
                    monstro,
                    alcance,
                    vida_em_segundos,
                    condicao_de_sumico: 0,
                    caminho,
                    quantidade,
                });
            }
            Some(TocarAcao) if versao < 9 => {
                // `O_PLAY_ACTION_VERSION8 { szActionName[128], bLoop, iInterval }`.
                let acao = l.texto_fixo(128)?;
                let _repetir = l.bool()?;
                l.fatia(3)?;
                let intervalo = l.i32()?;
                return Ok(ParametroDeOperacao::TocarAcao {
                    acao,
                    repeticoes: 1,
                    intervalo,
                    duracao: 5000,
                });
            }
            _ => {}
        }

        // Daqui para baixo o original faz um `fread(p->pParam, GetOperationParamSize(...))`
        // — um bloco só, do tamanho da struct C++.
        //
        // Um tipo sem variante nomeada (de 37 a 102) é guardado cru: o tamanho vem da
        // mesma tabela, então o arquivo continua em sincronia.
        let Some(tipo) = tipo else {
            return Ok(ParametroDeOperacao::Bruto {
                tipo: tipo_cru,
                bytes: l.fatia(esperado)?.to_vec(),
            });
        };

        // Este leitor decodifica campo a campo, então a tabela de tamanhos vira uma
        // conferência: se os campos escritos aqui não somarem exatamente o `sizeof` do
        // original, o erro aparece agora em vez de desalinhar o resto do arquivo.
        let inicio = l.pos;
        let valor = Self::ler_parametro_de_tamanho_fixo(l, tipo, versao)?;
        let lidos = l.pos - inicio;
        if lidos != esperado {
            return Err(AiPolicyError::TamanhoDeParametro { operacao: tipo, esperado, lidos });
        }
        Ok(valor)
    }

    /// O ramo genérico de [`Self::ler_parametro_de_operacao`]: as structs de tamanho fixo
    /// de `policytype.h`, campo a campo.
    fn ler_parametro_de_tamanho_fixo(
        l: &mut Leitor,
        tipo: TipoDeOperacao,
        versao: u32,
    ) -> Result<ParametroDeOperacao> {
        use TipoDeOperacao::*;

        Ok(match tipo {
            Atacar => ParametroDeOperacao::TipoDeAtaque(l.u32()?),
            UsarSkill => ParametroDeOperacao::Skill { skill: l.u32()?, nivel: l.u32()? },
            RodarTrigger | PararTrigger | AtivarTrigger | MatarTimer => {
                ParametroDeOperacao::IdDeTrigger(l.u32()?)
            }
            CriarTimer => ParametroDeOperacao::Timer {
                id: l.u32()?,
                periodo: l.u32()?,
                contador: l.u32()?,
            },
            AtivarControlador => ParametroDeOperacao::Controlador {
                id: l.u32()?,
                parar: l.bool_alinhado()?,
            },
            DefinirGlobal => ParametroDeOperacao::DefinirGlobal {
                id: l.i32()?,
                valor: l.i32()?,
                e_valor_direto: l.bool_alinhado()?,
            },
            RevisarGlobal | RevisarHistorico => {
                ParametroDeOperacao::AjustarVariavel { id: l.i32()?, valor: l.i32()? }
            }
            DefinirHistorico => ParametroDeOperacao::DefinirHistorico {
                id: l.i32()?,
                valor: l.i32()?,
                valor_e_do_historico: l.bool_alinhado()?,
            },
            InvocarMonstro => ParametroDeOperacao::InvocarMonstro {
                monstro: l.i32()?,
                alcance: l.i32()?,
                vida_em_segundos: l.i32()?,
                condicao_de_sumico: l.i32()?,
                caminho: l.i32()?,
                quantidade: l.i32()?,
            },
            AndarPorCaminho => ParametroDeOperacao::AndarPorCaminho {
                mundo: l.i32()?,
                caminho: l.i32()?,
                tipo_de_patrulha: l.i32()?,
                tipo_de_velocidade: l.i32()?,
            },
            TocarAcao => {
                let acao = l.texto_fixo(128)?;
                let repeticoes = l.i32()?;
                let intervalo = l.i32()?;
                let duracao = l.i32()?;
                // A partir da versão 38 a struct ganhou um `iUnknown` no fim, que o
                // próprio 1.7.2 só zera — aqui ele é consumido e descartado, para o
                // tamanho fechar.
                if versao >= 38 {
                    l.fatia(4)?;
                }
                ParametroDeOperacao::TocarAcao { acao, repeticoes, intervalo, duracao }
            }
            EntregarPontosPvpDeFaccao => ParametroDeOperacao::PontosPvpDeFaccao(l.u32()?),
            CalcularVariavel => ParametroDeOperacao::CalcularVariavel {
                destino: l.i32()?,
                tipo_do_destino: l.i32()?,
                origem1: l.i32()?,
                tipo_da_origem1: l.i32()?,
                operador: l.i32()?,
                origem2: l.i32()?,
                tipo_da_origem2: l.i32()?,
            },
            InvocarMonstro2 => ParametroDeOperacao::InvocarMonstro2 {
                condicao_de_sumico: l.i32()?,
                monstro: l.i32()?,
                tipo_do_monstro: l.i32()?,
                alcance: l.i32()?,
                vida_em_segundos: l.i32()?,
                caminho: l.i32()?,
                tipo_do_caminho: l.i32()?,
                quantidade: l.i32()?,
                tipo_da_quantidade: l.i32()?,
            },
            AndarPorCaminho2 => ParametroDeOperacao::AndarPorCaminho2 {
                mundo: l.i32()?,
                caminho: l.i32()?,
                tipo_do_caminho: l.i32()?,
                tipo_de_patrulha: l.i32()?,
                tipo_de_velocidade: l.i32()?,
            },
            UsarSkill2 => ParametroDeOperacao::Skill2 {
                skill: l.u32()?,
                tipo_da_skill: l.u32()?,
                nivel: l.u32()?,
                tipo_do_nivel: l.u32()?,
            },
            AtivarControlador2 => ParametroDeOperacao::Controlador2 {
                id: l.u32()?,
                tipo_do_id: l.u32()?,
                parar: l.bool_alinhado()?,
            },
            EntregarMissao => {
                ParametroDeOperacao::EntregarMissao { id: l.u32()?, tipo_do_id: l.u32()? }
            }
            InvocarMina => {
                let tipo_da_vida = l.i32()?;
                let mina = l.i32()?;
                let tipo_da_mina = l.i32()?;
                let alcance = l.i32()?;
                let vida_em_segundos = l.i32()?;
                let quantidade = l.i32()?;
                let tipo_da_quantidade = l.i32()?;
                // `bSetOwnerAsMonsterOwner` só existe a partir da versão 24.
                let dono_e_o_dono_do_monstro =
                    if versao >= 24 { Some(l.bool_alinhado()?) } else { None };
                ParametroDeOperacao::InvocarMina {
                    tipo_da_vida,
                    mina,
                    tipo_da_mina,
                    alcance,
                    vida_em_segundos,
                    quantidade,
                    tipo_da_quantidade,
                    dono_e_o_dono_do_monstro,
                }
            }
            InvocarNpc => ParametroDeOperacao::InvocarNpc {
                tipo_da_vida: l.i32()?,
                npc: l.i32()?,
                tipo_do_npc: l.i32()?,
                alcance: l.i32()?,
                vida_em_segundos: l.i32()?,
                caminho: l.i32()?,
                tipo_do_caminho: l.i32()?,
                quantidade: l.i32()?,
                tipo_da_quantidade: l.i32()?,
            },
            EntregarMissaoAleatoriaNaRegiao => ParametroDeOperacao::MissaoAleatoriaNaRegiao {
                id: l.u32()?,
                minimo: l.zona()?,
                maximo: l.zona()?,
            },
            EntregarMissaoNaListaDeOdio => ParametroDeOperacao::MissaoNaListaDeOdio {
                id: l.u32()?,
                tipo_do_id: l.u32()?,
                alcance: l.i32()?,
                jogadores: l.i32()?,
            },
            LimparMissaoDeTorreNaRegiao => ParametroDeOperacao::LimparMissaoDeTorre {
                minimo: l.zona()?,
                maximo: l.zona()?,
            },
            SalvarContagemDeJogadoresNoRaio => ParametroDeOperacao::ContarJogadoresNoRaio {
                raio: l.f32()?,
                tipo_do_raio: l.u32()?,
                destino: l.i32()?,
                tipo_do_destino: l.u32()?,
            },
            SalvarContagemDeJogadoresNaRegiao => ParametroDeOperacao::ContarJogadoresNaRegiao {
                minimo: l.zona()?,
                maximo: l.zona()?,
                destino: l.i32()?,
                tipo_do_destino: l.u32()?,
            },
            // Os tipos sem parâmetro, e os três de tamanho variável, que nunca chegam aqui
            // — saem antes, em `ler_parametro_de_operacao`.
            Falar | SkillComFala | Falar2 | LimparListaDeOdio | Fugir | OdioParaPrimeiro
            | OdioParaUltimo | OdioCinquentaPorCento | PularOperacao => {
                ParametroDeOperacao::Nenhum
            }
        })
    }

    /// `CTriggerData::ReadConditonTree`.
    ///
    /// O nó traz `iType` e `dat_size`, depois `dat_size` bytes de parâmetro, e então uma
    /// sequência de marcadores: `LEFT_SUB`/`RIGHT_SUB` abrem um filho (recursivo), `LEAF`
    /// e `NODE_END` fecham o nó.
    fn ler_arvore_de_condicoes(
        l: &mut Leitor,
        profundidade: usize,
    ) -> Result<Option<NoDeCondicao>> {
        if profundidade > PROFUNDIDADE_MAXIMA {
            return Err(AiPolicyError::ContagemAbsurda {
                campo: "profundidade da árvore de condições",
                valor: profundidade as i32,
            });
        }

        let posicao_do_tipo = l.pos;
        let tipo_cru = l.i32()?;
        if !(0..=MAIOR_CONDICAO).contains(&tipo_cru) {
            return Err(AiPolicyError::CondicaoDesconhecida {
                tipo: tipo_cru,
                posicao: posicao_do_tipo,
            });
        }
        let tamanho = l.i32()?;
        if !(0..=65_536).contains(&tamanho) {
            return Err(AiPolicyError::ContagemAbsurda {
                campo: "parâmetro de condição",
                valor: tamanho,
            });
        }

        let tipo = TipoDeCondicao::de_i32(tipo_cru);
        let bruto = l.fatia(tamanho as usize)?;
        let parametro = Self::interpretar_parametro_de_condicao(tipo, bruto);

        let mut esquerda = None;
        let mut direita = None;

        loop {
            let marcador = l.i32()?;
            match marcador {
                CONDITION_LEAF | CONDITION_NODE_END => break,
                CONDITION_LEFT_SUB => {
                    esquerda = Self::ler_arvore_de_condicoes(l, profundidade + 1)?.map(Box::new);
                }
                CONDITION_RIGHT_SUB => {
                    direita = Self::ler_arvore_de_condicoes(l, profundidade + 1)?.map(Box::new);
                }
                // O original ignora marcador desconhecido e roda o laço de novo, o que com
                // arquivo corrompido é laço infinito. Aqui vira erro.
                outro => {
                    return Err(AiPolicyError::ContagemAbsurda {
                        campo: "marcador da árvore de condições",
                        valor: outro,
                    })
                }
            }
        }

        Ok(Some(NoDeCondicao { tipo, tipo_cru, parametro, esquerda, direita }))
    }

    /// Dá forma aos bytes do parâmetro de condição. O tamanho já veio do arquivo, então
    /// divergência entre o tipo e o tamanho gravado não desalinha nada — só cai em
    /// [`ParametroDeCondicao::Bruto`].
    fn interpretar_parametro_de_condicao(
        tipo: Option<TipoDeCondicao>,
        bruto: &[u8],
    ) -> ParametroDeCondicao {
        use TipoDeCondicao::*;

        if bruto.is_empty() {
            return ParametroDeCondicao::Nenhum;
        }

        let i32_em = |o: usize| i32::from_le_bytes(bruto[o..o + 4].try_into().unwrap());
        let u32_em = |o: usize| u32::from_le_bytes(bruto[o..o + 4].try_into().unwrap());
        let f32_em = |o: usize| f32::from_le_bytes(bruto[o..o + 4].try_into().unwrap());
        let zona_em = |o: usize| VerticeDeZona {
            x: f32_em(o),
            y: f32_em(o + 4),
            z: f32_em(o + 8),
        };

        match (tipo, bruto.len()) {
            (Some(TimeCome), 4) => ParametroDeCondicao::IdDeTimer(u32_em(0)),
            (Some(HpLess), 4) => ParametroDeCondicao::HpAbaixoDe(f32_em(0)),
            (Some(Random), 4) => ParametroDeCondicao::Probabilidade(f32_em(0)),
            (Some(Var | LocalVar | AtHistoryStage | HasFilter), 4) => {
                ParametroDeCondicao::Id(i32_em(0))
            }
            (Some(Constant | HistoryValue), 4) => ParametroDeCondicao::Valor(i32_em(0)),
            (Some(BeHurt), 8) => {
                ParametroDeCondicao::FaixaDeDano { minimo: i32_em(0), maximo: i32_em(4) }
            }
            (Some(ReachEnd), 4) => ParametroDeCondicao::Caminho(i32_em(0)),
            (Some(ReachEnd2), 8) => ParametroDeCondicao::Caminho2 {
                caminho: i32_em(0),
                tipo_do_caminho: i32_em(4),
            },
            (Some(PlayerCountInRadius), 4) => ParametroDeCondicao::Raio(f32_em(0)),
            (Some(PlayerCountInRegion), 24) => ParametroDeCondicao::Regiao {
                minimo: zona_em(0),
                maximo: zona_em(12),
            },
            _ => ParametroDeCondicao::Bruto(bruto.to_vec()),
        }
    }

    pub fn get_policy(&self, policy_id: u32) -> Option<&AiPolicy> {
        self.policies.get(&policy_id)
    }
}
