//! Leitor do `tasks.data` — as missões.
//!
//! # Formato (autoridade: o arquivo, e não o fonte)
//!
//! O arquivo é `TASK_PACK_HEADER` (`magic`, `version`, `item_count`), uma tabela de
//! `item_count` deslocamentos `u32` e, em cada deslocamento, uma missão de topo gravada por
//! `ATaskTempl::SaveBinary` (`ElementClient/Task/TaskTempl.cpp`): o bloco fixo
//! `ATaskTemplFixedData` despejado com `fwrite(this)` sob `#pragma pack(1)`, os vetores
//! variáveis na ordem de `LoadFixedDataFromBinFile`, os dois prêmios, as quatro escalas de
//! prêmio, os textos, os cinco diálogos e, recursivamente, as submissões.
//!
//! **A tabela de deslocamentos é o gabarito.** Cada missão de topo tem que terminar
//! exatamente onde a seguinte começa; o leitor recusa o arquivo inteiro na primeira que
//! não termina. Isso não deixa espaço para layout "quase certo".
//!
//! # Por que os tamanhos não são os do fonte
//!
//! O fonte do 1.5.5 que temos (`EvolvedPWClient` e `EvolvedPWServer`, idênticos aqui) é da
//! versão **125** (`_task_templ_cur_version = 125`, `TaskTempl.cpp:5`). Os `tasks.data` dos
//! clientes 1.5.5 BR e EN são da versão **129**. O `ATaskTemplFixedData`
//! medido com o MSVC x86 contra o `TaskTempl.h` real (macros do `ElementClient.vcxproj`)
//! tem 1.087 bytes e o `AWARD_DATA`, 269 — e com isso nenhuma missão fechava.
//!
//! A diferença é o sistema de **Lar** (casa do jogador), acrescentado entre a 125 e a 129.
//! Os campos foram localizados nos próprios dados (histograma de bytes das 14.885 missões:
//! os `m_bShowBy*`, que nascem `true`, e os ponteiros gravados pelo editor denunciam cada
//! deslocamento), e os nomes vêm do fonte 1.7.2 (`172Source/cgame/gs/task/TaskTempl.h`),
//! que é da versão 187 e já tem esses membros na mesma ordem:
//!
//! | onde | bytes | membros |
//! | :--- | ---: | :--- |
//! | depois de `m_bTowerTask` | 3 | `m_bHomeTask`, `m_bDeliverInHostHome`, `m_bFinishInHostHome` |
//! | depois de `m_bShowByVIPLevel` | 47 | `m_bPremNoHome` e as faixas de nível/recurso/fábrica/prosperidade do Lar |
//! | depois de `m_ulTMIconStateID` | 20 | `m_ulTMHomeLevelType`, `m_ulTMReachHomeLevel`, `m_ulTMReachHomeFlourish`, `m_ulHomeItemsWanted`, `m_HomeItemsWanted` |
//! | fim do `AWARD_DATA` | 21 | `m_iHomeResource[5]`, `m_bCreateHome` |
//!
//! E um vetor variável novo: `m_ulHomeItemsWanted × HOME_ITEM_WANTED` (8 bytes), lido
//! depois dos `m_pLeaveSite` — a mesma posição do 1.7.2.
//!
//! Resultado: 1.157 bytes de bloco fixo, 290 de prêmio, e **14.885 de 14.885** missões
//! (cliente BR, `realm_155`) e **14.978 de 14.978** (cliente EN, medido antes de sair do
//! projeto em 2026-09-17) terminando no deslocamento certo.
//!
//! A v55 do 1.2.6 é percorrida pelo layout medido no `elementclient.exe` v126
//! (`docs/RESULTADO_TASKS_V55.md`): 534 bytes fixos, prêmio de 75 bytes e
//! 2.819 raízes/7.994 tarefas até o último byte. Os campos ainda não
//! identificados do bloco fixo ficam zerados no `TaskTemplate`; só os
//! campos com offset confirmado são projetados. A v124 continua só no cabeçalho.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum TasksError {
    #[error("Erro de I/O na leitura do tasks.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato de tasks.data inválido")]
    InvalidFormat,

    #[error("tasks.data truncado: faltam bytes na posição {0}")]
    Truncado(usize),

    #[error(
        "tasks.data desalinhado: a missão de topo #{indice} (id {id}) começa em {inicio} e \
         terminou em {fim}, mas a próxima começa em {esperado}"
    )]
    Desalinhado { indice: usize, id: u32, inicio: usize, fim: usize, esperado: usize },
}

pub type Result<T> = std::result::Result<T, TasksError>;

/// Assinatura do `tasks.data` (`TASK_PACK_MAGIC`, `Task/TaskTempl.h:223`).
pub const MAGICO_DO_TASKS: u32 = 0x9385_8361;

/// A única versão cujo layout foi medido e validado contra a tabela de deslocamentos.
pub const VERSAO_SUPORTADA: u32 = 129;

/// Um item pedido, entregue ou dado como prêmio (`ITEM_WANTED`, 17 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ItemDeMissao {
    pub id: u32,
    /// `m_bCommonItem` — `false` é item de missão (vai para o inventário de missão).
    pub comum: bool,
    pub quantidade: u32,
    /// `m_fProb` — chance de cair, no pedido; peso, no prêmio.
    pub probabilidade: f32,
    /// `m_lPeriod` — validade em segundos (0 = permanente).
    pub validade: i32,
}

/// Um monstro a matar (`MONSTER_WANTED`, 30 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonstroPedido {
    pub monstro: u32,
    pub quantidade: u32,
    /// Item que o monstro solta para a missão (0 = a missão conta mortes, não itens).
    pub item_que_cai: u32,
    pub quantidade_do_item: u32,
    pub item_comum: bool,
    pub chance_do_item: f32,
    /// `m_bKillerLev` — só conta se o matador tiver nível compatível com o monstro.
    pub nivel_do_matador: bool,
    /// `m_iDPS` / `m_iDPH` — metas do boneco de treino; zero nas missões comuns.
    pub dps: i32,
    pub dph: i32,
}

/// Um grupo de itens de prêmio (`AWARD_ITEMS_CAND`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrupoDeItens {
    /// `m_bRandChoose` — sorteia um item do grupo pelos pesos; senão dá todos.
    pub sorteia_um: bool,
    pub itens: Vec<ItemDeMissao>,
}

impl GrupoDeItens {
    /// `m_ulAwardCmnItems` / `m_ulAwardTskItems`, que o original conta ao carregar.
    pub fn contagens(&self) -> (u32, u32) {
        let comuns = self.itens.iter().filter(|i| i.comum).count() as u32;
        (comuns, self.itens.len() as u32 - comuns)
    }
}

/// Um monstro que a missão evoca/invoca (`MONSTERS_SUMMONED`, 16 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonstroInvocado {
    pub monstro: u32,
    pub quantidade: u32,
    pub probabilidade: f32,
    pub periodo: i32,
}

/// A convocação de monstros no prêmio da missão (`AWARD_MONSTERS_SUMMONED`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InvocacaoDeMonstros {
    pub sorteia_um: bool,
    pub raio: u32,
    pub some_ao_morrer: bool,
    pub monstros: Vec<MonstroInvocado>,
}

/// O que a missão paga (`AWARD_DATA`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskReward {
    pub exp: i64,
    pub sp: i64,
    /// `m_ulGoldNum`.
    pub money: i64,
    pub reputation: i32,
    pub realm_exp: u32,
    /// `m_ulNewTask` — missão entregue ao terminar esta (0 = nenhuma).
    pub nova_missao: u32,
    /// `m_ulTransWldId` + `m_TransPt` — teleporte no fim (mundo 0 = sem teleporte).
    pub teleporte: Option<(u32, [f32; 3])>,
    /// `m_ulNewPeriod` — o novo nível de cultivo (0 = a missão não mexe nele).
    pub novo_cultivo: u32,
    /// `m_ulFuryULimit` — o novo teto da barra de chi (0 = a missão não mexe nele).
    pub teto_de_chi: u32,
    pub grupos_de_itens: Vec<GrupoDeItens>,
    /// `m_SummonedMonsters` — monstros invocados ao premiar/concluir a missão.
    pub monstros_invocados: Option<InvocacaoDeMonstros>,
    /// `m_bUseLevCo` — exp e SP multiplicados por `_lev_co[nível-1]` (`TaskProcess.cpp:1260`).
    pub usa_coeficiente_de_nivel: bool,
    /// `m_bMulti`, `m_nNumType`, `m_lNum` — multiplicador por variável global.
    pub multiplica: bool,
    pub tipo_do_multiplicador: i32,
    pub multiplicador: i32,
}

impl TaskReward {
    pub fn tem_algo(&self) -> bool {
        self.exp != 0
            || self.sp != 0
            || self.money != 0
            || self.reputation != 0
            || self.realm_exp != 0
            || self.nova_missao != 0
            || self.novo_cultivo != 0
            || self.teto_de_chi != 0
            || self.teleporte.is_some()
            || self.grupos_de_itens.iter().any(|g| !g.itens.is_empty())
            || self.monstros_invocados.is_some()
    }
}

/// `Task_Region` (`TaskTempl.h:1891`): uma caixa `zvMin`..`zvMax`, 24 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct RegiaoDeMissao {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl RegiaoDeMissao {
    /// `is_in_zone` (`TaskTempl.h:255-261`): bordas incluídas nos três eixos.
    pub fn contem(&self, p: [f32; 3]) -> bool {
        (0..3).all(|i| self.min[i] <= p[i] && self.max[i] >= p[i])
    }
}

/// `task_tm` (`TaskTempl.h:1594-1601`): seis `long` de 4 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MomentoDeMissao {
    pub ano: i32,
    pub mes: i32,
    pub dia: i32,
    pub hora: i32,
    pub minuto: i32,
    /// 1 = segunda … 7 = domingo (`task_week_map`, `TaskTempl.h:1583-1592`).
    pub dia_da_semana: i32,
}

/// Uma janela de `m_tmStart[i]`..`m_tmEnd[i]` com o tipo `m_tmType[i]`
/// (`enumTaskTimeDate`, `Month`, `Week`, `Day`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct JanelaDeHorario {
    pub tipo: u8,
    pub inicio: MomentoDeMissao,
    pub fim: MomentoDeMissao,
}

/// `TEAM_MEM_WANTED` (`TaskTempl.h:369-379`), 36 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MembroPedido {
    pub nivel_minimo: u32,
    pub nivel_maximo: u32,
    pub raca: u32,
    /// `INVALID_VAL` (`0xFFFFFFFF`) = qualquer classe.
    pub classe: u32,
    pub genero: u32,
    pub minimo: u32,
    pub maximo: u32,
    /// Missão que o membro recebe no lugar da do capitão (0 = a mesma).
    pub missao: u32,
    pub forca: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: u32,
    pub name: String,
    /// Missão-mãe, para submissões (`m_ulParent`; `None` na de topo).
    pub parent: Option<u32>,
    pub sub_tasks: Vec<u32>,
    /// `m_ulType` — a categoria que o cliente mostra (diária, principal, ...).
    pub tipo: u32,
    /// `m_ulTimeLimit`, em segundos (0 = sem limite).
    pub limite_de_tempo: u32,
    /// `m_ulPremise_Lev_Min` / `m_ulPremise_Lev_Max` (0 = sem limite).
    pub min_level: u32,
    pub max_level: u32,
    /// `m_Occupations[m_ulOccupations]` — vazio = qualquer classe.
    pub req_classes: Vec<u32>,
    /// `m_ulGender` — 0 qualquer, 1 masculino, 2 feminino.
    pub genero: u32,
    /// `m_ulPremise_Tasks[m_ulPremise_Task_Count]`.
    pub pre_tasks: Vec<u32>,
    /// `m_ulMutexTasks[m_ulMutexTaskCount]` — não pode estar com estas ativas.
    pub missoes_exclusivas: Vec<u32>,
    /// Itens que o jogador precisa ter para receber (`m_PremItems`).
    pub itens_exigidos: Vec<ItemDeMissao>,
    /// Itens que a missão entrega ao ser aceita (`m_GivenItems`).
    pub itens_entregues: Vec<ItemDeMissao>,
    /// `m_ulDelvNPC` / `m_ulAwardNPC` (0 = sem NPC).
    pub npc_que_entrega: u32,
    pub npc_que_premia: u32,
    /// `m_enumMethod` — como se cumpre (matar, coletar, falar com NPC, chegar a um lugar...).
    pub metodo: u32,
    /// `m_enumFinishType` — como se conclui (automático, no NPC, ...).
    pub tipo_de_conclusao: u32,
    pub monster_kills: Vec<MonstroPedido>,
    pub item_collections: Vec<ItemDeMissao>,
    /// `m_ulGoldWanted`.
    pub dinheiro_pedido: u32,
    /// `m_ulReachLevel` — para missões "chegue ao nível N".
    pub nivel_a_alcancar: u32,
    /// `m_ulReachSiteId` — mundo do lugar a alcançar.
    pub mundo_a_alcancar: u32,
    /// `m_ulWaitTime`, em segundos, para missões de espera.
    pub espera: u32,
    pub entrega_automatica: bool,
    pub pode_desistir: bool,
    pub pode_repetir: bool,
    /// `m_bNeedRecord` — fica registrada como concluída (e por isso não se repete).
    pub precisa_registro: bool,
    pub escolhe_um_filho: bool,
    pub sorteia_um_filho: bool,
    pub filhos_em_ordem: bool,
    pub oculta: bool,
    pub missao_chave: bool,
    pub descricao: String,
    pub rewards: TaskReward,
    pub premio_de_falha: TaskReward,

    // Campos que o motor de missões do `pw-gs` consulta. Deslocamentos em
    // `specs/tasks_155/layout129.tsv` (sonda do MSVC, B45).
    pub item_nao_retirado: bool,
    pub tempo_absoluto: bool,
    /// `m_ulTimetable` — quantas janelas de horário a missão tem (0 = sempre).
    pub janelas_de_horario: u32,
    /// `m_lAvailFrequency` (`enumTAF*`): 0 normal, 1 dia, 2 semana, 3 mês, 4 ano.
    pub frequencia: i32,
    pub limite_por_periodo: i32,
    pub pai_tambem_falha: bool,
    pub pai_tambem_sucesso: bool,
    pub refazer_apos_falha: bool,
    pub limpa_ao_desistir: bool,
    pub falha_ao_morrer: bool,
    pub max_receptores: u32,
    pub limpa_adquiridos: bool,
    pub mostra_aviso: bool,
    pub casamento: bool,
    pub compara_bolsa: bool,
    pub slots_de_bolsa: u32,
    pub torre: bool,
    pub pq: bool,
    pub pq_sub: bool,
    pub limite_de_conta: bool,
    pub limite_de_personagem: bool,
    pub nao_conta_falha: bool,
    pub nao_limpa_item_na_falha: bool,
    pub na_janela_de_titulo: bool,
    pub nivel_maximo_historico: bool,
    pub itens_exigidos_qualquer_um: bool,
    pub entregues_comuns: u32,
    pub entregues_de_missao: u32,
    pub deposito: u32,
    pub reputacao_minima: i32,
    pub pre_missoes_minimo: u32,
    /// `m_ulPremise_Period` — o nível de cultivo exigido.
    pub periodo: u32,
    pub so_gm: bool,
    pub cotask: u32,
    pub em_equipe: bool,
    /// `m_bRcvByTeam` — entregue à equipe inteira pelo capitão.
    pub recebida_pela_equipe: bool,
    /// `m_bDelvInZone` — só se aceita dentro de uma região.
    pub entrega_em_zona: bool,
    /// `m_ulPremise_Faction` — exige facção.
    pub faccao: u32,
    /// `m_bTransTo`, `m_ulTransWldId`, `m_TransPt` — teleporte ao **receber** a missão.
    pub teleporte_ao_receber: Option<(u32, [f32; 3])>,
    /// `m_iPremise_FactionRole` — cargo máximo na facção.
    pub papel_na_faccao: i32,
    /// `m_tmStart`/`m_tmEnd`/`m_tmType` — lidos do trecho variável.
    pub janelas: Vec<JanelaDeHorario>,
    /// `m_ulDelvWorld` e `m_pDelvRegion`.
    pub mundo_de_entrega: u32,
    pub regioes_de_entrega: Vec<RegiaoDeMissao>,
    /// `m_pReachSite` (o mundo é `mundo_a_alcancar`).
    pub lugares_a_alcancar: Vec<RegiaoDeMissao>,
    /// `m_ulLeaveSiteId` e `m_pLeaveSite`.
    pub mundo_a_sair: u32,
    pub lugares_a_sair: Vec<RegiaoDeMissao>,
    /// `m_TeamMemsWanted`.
    pub membros_pedidos: Vec<MembroPedido>,
    /// `m_bRcvChckMem` e `m_fRcvMemDist` (já ao quadrado: comparado com a distância ao
    /// quadrado em `HasAllTeamMemsWanted`, `TaskTempl.inl:182-188`).
    pub confere_membros: bool,
    pub distancia_dos_membros: f32,
    /// `m_bDistinguishedOcc` — sem classe repetida na equipe.
    pub classes_distintas: bool,
    /// `m_bCoupleOnly`.
    pub so_casal: bool,
    /// `m_bPremise_Spouse` — exige casamento.
    pub conjuge: bool,
    /// `m_ulAwardType_S` / `_F` (`enumTAT*`): 0 normal, 1 por unidade, 2 por tempo, 3 por itens.
    pub tipo_de_premio_sucesso: u32,
    pub tipo_de_premio_falha: u32,
    /// `m_uDepth` — quantas entradas da lista ativa a missão ocupa
    /// (`ATaskTempl::CheckDepth`, `TaskTempl.h:2748-2771`). Calculado depois da leitura.
    pub profundidade: u8,
}

#[derive(Debug, Clone, Default)]
pub struct TasksData {
    /// O `_task_templ_cur_version` deste arquivo — a **segunda** palavra do cabeçalho.
    pub version: u32,
    /// Quantas missões de topo o arquivo declara (`item_count` do cabeçalho).
    pub quantidade_declarada: u32,
    /// Todas as missões, de topo e submissões, por id.
    pub tasks: HashMap<u32, TaskTemplate>,
    /// Os ids das missões de topo, na ordem do arquivo.
    pub de_topo: Vec<u32>,
}

// ============================================================================ layout v129

/// Tamanhos da v129 — ver a nota do módulo.
mod v129 {
    pub const FIXO: usize = 1157;
    pub const PREMIO: usize = 290;

    pub const TASK_CHAR: usize = 2;
    pub const MAX_TASK_NAME_LEN: usize = 30;
    pub const TASK_TM: usize = 24;
    pub const TASK_REGION: usize = 24;
    pub const ITEM_WANTED: usize = 17;
    pub const MONSTER_WANTED: usize = 30;
    pub const PLAYER_WANTED: usize = 37;
    pub const TEAM_MEM_WANTED: usize = 36;
    pub const MONSTERS_CONTRIB: usize = 16;
    pub const HOME_ITEM_WANTED: usize = 8;
    pub const CHANGE_KEY: usize = 9; // long + long + bool
    /// `TASK_AWARD_MAX_DISPLAY_CHAR_LEN`, `sizeof(TASK_EXPRESSION)`.
    pub const EXP_LEN: usize = 64;
    pub const EXPRESSION: usize = 8;
    pub const EXPRESSAO: usize = EXP_LEN + EXPRESSION * EXP_LEN;
    pub const TITLE_AWARD: usize = 8;
    pub const MONSTER_SUMMONED: usize = 16;
    pub const RANKING_AWARD: usize = 21;
    pub const TALK_OPTION: usize = 136;
    pub const MAX_AWARD_SCALES: usize = 5;

    /// Deslocamentos dentro do bloco fixo (medidos no MSVC e corrigidos pelos campos do Lar).
    pub mod f {
        pub const ID: usize = 0;
        pub const NOME: usize = 4;
        pub const TEM_ASSINATURA: usize = 64;
        pub const TIPO: usize = 69;
        pub const LIMITE_DE_TEMPO: usize = 73;
        pub const ITEM_NAO_RETIRADO: usize = 103;
        pub const TEMPO_ABSOLUTO: usize = 104;
        pub const TIMETABLE: usize = 105;
        pub const FREQUENCIA: usize = 141;
        pub const LIMITE_POR_PERIODO: usize = 145;
        pub const PAI_TAMBEM_FALHA: usize = 152;
        pub const PAI_TAMBEM_SUCESSO: usize = 153;
        pub const REFAZER_APOS_FALHA: usize = 156;
        pub const LIMPA_AO_DESISTIR: usize = 157;
        pub const FALHA_AO_MORRER: usize = 159;
        pub const MAX_RECEPTORES: usize = 160;
        pub const LIMPA_ADQUIRIDOS: usize = 231;
        pub const MOSTRA_AVISO: usize = 236;
        pub const CASAMENTO: usize = 249;
        pub const COMPARA_BOLSA: usize = 279;
        pub const SLOTS_DE_BOLSA: usize = 280;
        pub const TORRE: usize = 284;
        pub const PQ: usize = 288;
        pub const PQ_SUB: usize = 301;
        pub const LIMITE_DE_CONTA: usize = 321;
        pub const LIMITE_DE_PERSONAGEM: usize = 322;
        pub const NAO_CONTA_FALHA: usize = 328;
        pub const NAO_LIMPA_ITEM_NA_FALHA: usize = 329;
        pub const NA_JANELA_DE_TITULO: usize = 330;
        pub const NIVEL_MAXIMO_HISTORICO: usize = 341;
        pub const ITENS_QUALQUER_UM: usize = 376;
        pub const ENTREGUES_COMUNS: usize = 381;
        pub const ENTREGUES_DE_MISSAO: usize = 385;
        pub const DEPOSITO: usize = 393;
        pub const REPUTACAO_MIN: usize = 398;
        pub const PRE_MISSOES_MINIMO: usize = 492;
        pub const PERIODO: usize = 496;
        pub const GM: usize = 572;
        pub const ENTREGA_EM_ZONA: usize = 164;
        pub const FACCAO: usize = 501;
        pub const PAPEL_NA_FACCAO: usize = 505;
        pub const TRANS_TO: usize = 204;
        pub const TRANS_WLD: usize = 205;
        pub const TRANS_PT: usize = 209;
        pub const TM_TYPE: usize = 109; // char × 24
        pub const MUNDO_DE_ENTREGA: usize = 165;
        pub const MUNDO_A_SAIR: usize = 989;
        pub const CONFERE_MEMBROS: usize = 691;
        pub const DISTANCIA_DOS_MEMBROS: usize = 692;
        pub const SO_CASAL: usize = 702;
        pub const CLASSES_DISTINTAS: usize = 703;
        pub const CONJUGE: usize = 568;
        pub const RECEBIDA_PELA_EQUIPE: usize = 675;
        pub const COTASK: usize = 621;
        pub const TIPO_DE_PREMIO_SUCESSO: usize = 1099;
        pub const TIPO_DE_PREMIO_FALHA: usize = 1103;
        pub const ESCOLHE_UM: usize = 149;
        pub const SORTEIA_UM: usize = 150;
        pub const FILHOS_EM_ORDEM: usize = 151;
        pub const PODE_DESISTIR: usize = 154;
        pub const PODE_REPETIR: usize = 155;
        pub const PRECISA_REGISTRO: usize = 158;
        pub const DELV_REGION_CNT: usize = 169;
        pub const ENTER_REGION_CNT: usize = 182;
        pub const LEAVE_REGION_CNT: usize = 195;
        pub const ENTREGA_AUTOMATICA: usize = 226;
        pub const MISSAO_CHAVE: usize = 237;
        pub const NPC_QUE_ENTREGA: usize = 238;
        pub const NPC_QUE_PREMIA: usize = 242;
        pub const CHANGE_KEY_CNT: usize = 250;
        pub const OCULTA: usize = 267;
        pub const PQ_EXP_CNT: usize = 289;
        pub const MONSTER_CONTRIB_CNT: usize = 303;
        pub const NIVEL_MIN: usize = 333;
        pub const NIVEL_MAX: usize = 337;
        pub const PREM_ITEMS: usize = 367;
        pub const GIVEN_ITEMS: usize = 377;
        pub const PRE_TASK_CNT: usize = 407;
        pub const PRE_TASKS: usize = 411; // u32 × 20
        pub const GENERO: usize = 510;
        pub const OCCUPATIONS_CNT: usize = 515;
        pub const OCCUPATIONS: usize = 519; // u32 × 12
        pub const MUTEX_CNT: usize = 629;
        pub const MUTEX: usize = 633; // u32 × 5
        pub const TEAMWORK: usize = 674;
        pub const TEAM_MEMS_WANTED: usize = 704;
        pub const PREM_TITLE_TOTAL: usize = 793;
        pub const METODO: usize = 890;
        pub const TIPO_DE_CONCLUSAO: usize = 894;
        pub const PLAYER_WANTED: usize = 898;
        pub const MONSTER_WANTED: usize = 906;
        pub const ITEMS_WANTED: usize = 914;
        pub const DINHEIRO_PEDIDO: usize = 922;
        pub const REACH_SITE_CNT: usize = 954;
        pub const MUNDO_A_ALCANCAR: usize = 958;
        pub const ESPERA: usize = 962;
        pub const LEAVE_SITE_CNT: usize = 985;
        pub const EXP_CNT: usize = 1038;
        pub const TASK_CHAR_CNT: usize = 1050;
        pub const NIVEL_A_ALCANCAR: usize = 1059;
        pub const HOME_ITEMS_WANTED: usize = 1091;
        pub const PAI: usize = 1131;
    }

    /// Deslocamentos dentro do `AWARD_DATA`.
    pub mod p {
        pub const DINHEIRO: usize = 0;
        pub const EXP: usize = 4;
        pub const REALM_EXP: usize = 8;
        pub const NOVA_MISSAO: usize = 13;
        pub const SP: usize = 17;
        pub const REPUTACAO: usize = 21;
        /// `m_ulNewPeriod` — o **nível de cultivo** que a missão concede
        /// (`Task/TaskTempl.h:1136-1144`: `GoldNum, Exp, RealmExp, ExpandRealmLevelMax(1),
        /// NewTask, SP, Reputation, NewPeriod`). Entregue por `SetCurPeriod`
        /// (`TaskProcess.cpp:1284`).
        pub const NOVO_CULTIVO: usize = 25;
        /// `m_ulFuryULimit` — o **teto da barra de chi** que a missão concede
        /// (`Task/TaskTempl.h:1136-1152`: … `NewPeriod, NewRelayStation, StorehouseSize×4,
        /// InventorySize, PetInventorySize, FuryULimit, TransWldId`). Entregue por
        /// `SetFuryUpperLimit` → `gplayer_imp::SetMaxAP` (`gs/task/taskman.cpp:498-501`).
        /// O deslocamento seguinte, 61, é o `MUNDO_DO_TELEPORTE` já validado.
        pub const TETO_DE_CHI: usize = 57;
        pub const MUNDO_DO_TELEPORTE: usize = 61;
        pub const PONTO_DO_TELEPORTE: usize = 65;
        pub const USA_COEF_DE_NIVEL: usize = 82;
        pub const CAND_ITEMS: usize = 89;
        pub const MULTIPLICA: usize = 196;
        pub const TIPO_DO_MULTIPLICADOR: usize = 197;
        pub const MULTIPLICADOR: usize = 201;
        pub const SUMMONED_MONSTERS: usize = 97;
        pub const PQ_RANKING_CNT: usize = 156;
        pub const CHANGE_KEY_CNT: usize = 164;
        pub const HISTORY_CHANGE_CNT: usize = 180;
        pub const DISPLAY_KEY_CNT: usize = 205;
        pub const EXP_CNT: usize = 213;
        pub const TASK_CHAR_CNT: usize = 225;
        pub const TITLE_NUM: usize = 253;
    }
}

/// Um trecho de bytes com posição, que só falha com `Truncado`.
struct Leitor<'a> {
    d: &'a [u8],
    o: usize,
}

impl<'a> Leitor<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let fim = self.o.checked_add(n).ok_or(TasksError::Truncado(self.o))?;
        let s = self.d.get(self.o..fim).ok_or(TasksError::Truncado(self.o))?;
        self.o = fim;
        Ok(s)
    }
    fn pular(&mut self, n: usize) -> Result<()> {
        self.bytes(n).map(|_| ())
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32_em(self.bytes(4)?, 0))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    /// Um contador que multiplica tamanho: absurdo é arquivo corrompido, não alocação.
    fn contador(&mut self, n: u32, tamanho: usize) -> Result<usize> {
        let total = (n as usize).checked_mul(tamanho).ok_or(TasksError::Truncado(self.o))?;
        if self.o + total > self.d.len() {
            return Err(TasksError::Truncado(self.o));
        }
        Ok(n as usize)
    }
}

fn u32_em(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn f32_em(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_em(b, o))
}

/// `convert_txt` (`TaskTempl.cpp:629`): cada `wchar_t` vem XOR com o id da missão truncado
/// para `namechar`. Para no primeiro NUL.
fn texto(bytes: &[u8], id: u32) -> String {
    let chave = id as u16;
    let unidades: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]) ^ chave)
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&unidades)
}

fn item(b: &[u8]) -> ItemDeMissao {
    ItemDeMissao {
        id: u32_em(b, 0),
        comum: b[4] != 0,
        quantidade: u32_em(b, 5),
        probabilidade: f32_em(b, 9),
        validade: u32_em(b, 13) as i32,
    }
}

fn itens(l: &mut Leitor, n: u32) -> Result<Vec<ItemDeMissao>> {
    let n = l.contador(n, v129::ITEM_WANTED)?;
    (0..n).map(|_| l.bytes(v129::ITEM_WANTED).map(item)).collect()
}

fn lista_u32(b: &[u8], o: usize, n: u32, maximo: usize) -> Vec<u32> {
    (0..(n as usize).min(maximo)).map(|i| u32_em(b, o + 4 * i)).collect()
}

/// `LoadAwardDataBin` (`TaskTempl.cpp:1359`).
fn premio(l: &mut Leitor) -> Result<TaskReward> {
    use v129::p;
    let a = l.bytes(v129::PREMIO)?;

    let mut grupos = Vec::new();
    for _ in 0..l.contador(u32_em(a, p::CAND_ITEMS), 5)? {
        let sorteia_um = l.u8()? != 0;
        let n = l.u32()?;
        grupos.push(GrupoDeItens { sorteia_um, itens: itens(l, n)? });
    }
    let invocados = u32_em(a, p::SUMMONED_MONSTERS);
    let mut monstros_invocados = None;
    if invocados != 0 {
        let sorteia_um = l.u8()? != 0;
        let raio = l.u32()?;
        let some_ao_morrer = l.u8()? != 0;
        let n = l.contador(invocados, v129::MONSTER_SUMMONED)?;
        let mut monstros = Vec::with_capacity(n);
        for _ in 0..n {
            let m = l.bytes(v129::MONSTER_SUMMONED)?;
            monstros.push(MonstroInvocado {
                monstro: u32_em(m, 0),
                quantidade: u32_em(m, 4),
                probabilidade: f32_em(m, 8),
                periodo: u32_em(m, 12) as i32,
            });
        }
        monstros_invocados = Some(InvocacaoDeMonstros {
            sorteia_um,
            raio,
            some_ao_morrer,
            monstros,
        });
    }
    let ranking = u32_em(a, p::PQ_RANKING_CNT);
    if ranking != 0 {
        l.pular(1)?;
        let n = l.contador(ranking, v129::RANKING_AWARD)?;
        l.pular(n * v129::RANKING_AWARD)?;
    }
    for (campo, tamanho) in [
        (p::TITLE_NUM, v129::TITLE_AWARD),
        (p::CHANGE_KEY_CNT, v129::CHANGE_KEY),
        (p::HISTORY_CHANGE_CNT, v129::CHANGE_KEY),
        (p::DISPLAY_KEY_CNT, 4),
        (p::EXP_CNT, v129::EXPRESSAO),
        (p::TASK_CHAR_CNT, v129::TASK_CHAR * v129::EXP_LEN),
    ] {
        let n = l.contador(u32_em(a, campo), tamanho)?;
        l.pular(n * tamanho)?;
    }

    let mundo = u32_em(a, p::MUNDO_DO_TELEPORTE);
    Ok(TaskReward {
        exp: u32_em(a, p::EXP) as i64,
        sp: u32_em(a, p::SP) as i64,
        money: u32_em(a, p::DINHEIRO) as i64,
        reputation: u32_em(a, p::REPUTACAO) as i32,
        novo_cultivo: u32_em(a, p::NOVO_CULTIVO),
        teto_de_chi: u32_em(a, p::TETO_DE_CHI),
        realm_exp: u32_em(a, p::REALM_EXP),
        nova_missao: u32_em(a, p::NOVA_MISSAO),
        teleporte: (mundo != 0).then(|| {
            let o = p::PONTO_DO_TELEPORTE;
            (mundo, [f32_em(a, o), f32_em(a, o + 4), f32_em(a, o + 8)])
        }),
        grupos_de_itens: grupos,
        monstros_invocados,
        usa_coeficiente_de_nivel: a[p::USA_COEF_DE_NIVEL] != 0,
        multiplica: a[p::MULTIPLICA] != 0,
        tipo_do_multiplicador: u32_em(a, p::TIPO_DO_MULTIPLICADOR) as i32,
        multiplicador: u32_em(a, p::MULTIPLICADOR) as i32,
    })
}

/// `LoadAwardDataRatioScale` / `LoadAwardDataItemsScale`: o cabeçalho da escala e um
/// prêmio por degrau. Os degraus não são usados ainda; são lidos para manter o passo.
fn escala(l: &mut Leitor, cabecalho_extra: usize) -> Result<()> {
    let degraus = l.u32()?;
    l.pular(cabecalho_extra)?;
    for _ in 0..l.contador(degraus, v129::PREMIO)? {
        premio(l)?;
    }
    Ok(())
}

/// `task_tm`: seis `long` (4 bytes cada no binário de 32 bits).
fn momento(m: &[u8]) -> MomentoDeMissao {
    let i = |o: usize| u32_em(m, o) as i32;
    MomentoDeMissao { ano: i(0), mes: i(4), dia: i(8), hora: i(12), minuto: i(16), dia_da_semana: i(20) }
}

/// `n` × `Task_Region`.
fn regioes(l: &mut Leitor, n: u32) -> Result<Vec<RegiaoDeMissao>> {
    let n = l.contador(n, v129::TASK_REGION)?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let r = l.bytes(v129::TASK_REGION)?;
        let f = |o: usize| f32_em(r, o);
        v.push(RegiaoDeMissao { min: [f(0), f(4), f(8)], max: [f(12), f(16), f(20)] });
    }
    Ok(v)
}

/// `LoadDescriptionBin` / `LoadTributeBin`: `size_t` de caracteres e o texto.
fn texto_longo(l: &mut Leitor, id: u32) -> Result<String> {
    let n = l.u32()?;
    let n = l.contador(n, v129::TASK_CHAR)?;
    Ok(texto(l.bytes(n * v129::TASK_CHAR)?, id))
}

/// Um `talk_proc`: `id_talk`, `text[64]`, as janelas e suas opções. Só é atravessado.
fn dialogo(l: &mut Leitor) -> Result<()> {
    l.pular(4 + 64 * v129::TASK_CHAR)?;
    let janelas = l.i32()?.max(0) as u32;
    for _ in 0..l.contador(janelas, 16)? {
        l.pular(8)?;
        let n = l.i32()?.max(0) as u32;
        let n = l.contador(n, v129::TASK_CHAR)?;
        l.pular(n * v129::TASK_CHAR)?;
        let opcoes = l.i32()?.max(0) as u32;
        let opcoes = l.contador(opcoes, v129::TALK_OPTION)?;
        l.pular(opcoes * v129::TALK_OPTION)?;
    }
    Ok(())
}

/// Layout medido no `elementclient.exe` v126: `LoadBinary` VA 0x62f6c0,
/// leitura fixa em 0x62d04d. Travessia completa documentada em
/// `docs/RESULTADO_TASKS_V55.md` e `docs/evidencias/126/validar_tasks_v55.py:112-148`.
mod v55 {
    pub const FIXO: usize = 534;
    pub const ITEM: usize = 13;
    pub const MONSTRO: usize = 22;
    pub const MEMBRO: usize = 32;
    pub const PREMIO: usize = 75;
}

fn item_v55(b: &[u8]) -> ItemDeMissao {
    ItemDeMissao {
        id: u32_em(b, 0), comum: b[4] != 0, quantidade: u32_em(b, 5),
        probabilidade: f32_em(b, 9), validade: 0,
    }
}

fn itens_v55(l: &mut Leitor, n: u32) -> Result<Vec<ItemDeMissao>> {
    let n = l.contador(n, v55::ITEM)?;
    (0..n).map(|_| l.bytes(v55::ITEM).map(item_v55)).collect()
}

fn premio_v55(l: &mut Leitor) -> Result<TaskReward> {
    let a = l.bytes(v55::PREMIO)?;
    let mut grupos = Vec::new();
    for _ in 0..l.contador(u32_em(a, 67), 5)? {
        let sorteia_um = l.u8()? != 0;
        let n = l.u32()?;
        grupos.push(GrupoDeItens { sorteia_um, itens: itens_v55(l, n)? });
    }
    // `AWARD_DATA` v55: `m_ulGoldNum`, `m_ulExp`, `m_ulNewTask`,
    // `m_ulSP`, `m_lReputation`, `m_ulNewPeriod` (cliente 1.5.3,
    // CElementClient/Task/TaskTempl.h:1134-1141; posições v55
    // conferidas no prêmio da tarefa 1173 do arquivo real).
    Ok(TaskReward {
        money: u32_em(a, 0) as i64, exp: u32_em(a, 4) as i64,
        nova_missao: u32_em(a, 8), sp: u32_em(a, 12) as i64,
        reputation: u32_em(a, 16) as i32, novo_cultivo: u32_em(a, 20),
        grupos_de_itens: grupos, ..Default::default()
    })
}

fn escala_v55(l: &mut Leitor, bytes_do_cabecalho: usize) -> Result<()> {
    let n = l.i32()?;
    if n < 0 { return Err(TasksError::InvalidFormat); }
    l.pular(bytes_do_cabecalho)?;
    for _ in 0..l.contador(n as u32, v55::PREMIO)? { premio_v55(l)?; }
    Ok(())
}

fn dialogo_v55(l: &mut Leitor) -> Result<()> {
    l.pular(4 + 128)?;
    let janelas = l.i32()?;
    if janelas < 0 { return Err(TasksError::InvalidFormat); }
    for _ in 0..l.contador(janelas as u32, 16)? {
        l.pular(8)?;
        texto_longo(l, 0)?;
        let opcoes = l.i32()?;
        if opcoes < 0 { return Err(TasksError::InvalidFormat); }
        let n = l.contador(opcoes as u32, 136)?;
        l.pular(n * 136)?;
    }
    Ok(())
}

fn missao_v55(l: &mut Leitor, pai: Option<u32>, saida: &mut HashMap<u32, TaskTemplate>) -> Result<u32> {
    let b = l.bytes(v55::FIXO)?;
    let id = u32_em(b, 0);
    if b[0x40] != 0 { l.pular(60)?; }
    let horarios = u32_em(b, 0x4e);
    let n = l.contador(horarios, 48)?;
    let mut janelas = Vec::with_capacity(n);
    for i in 0..n {
        let inicio = momento(l.bytes(24)?);
        let fim = momento(l.bytes(24)?);
        // `m_tmType` segue o contador no fixo: cliente 1.5.5,
        // ElementClient/Task/TaskTempl.cpp:3885-3897; no v55,
        // 0x4e + 4 = 0x52 (confirmado pelos 281 registros com horário).
        let tipo = b.get(0x52 + i).copied().unwrap_or(0);
        janelas.push(JanelaDeHorario { tipo, inicio, fim });
    }
    let itens_exigidos = itens_v55(l, u32_em(b, 0xca))?;
    let itens_entregues = itens_v55(l, u32_em(b, 0xd3))?;
    if b[0x176] != 0 {
        let n = l.contador(u32_em(b, 0x191), v55::MEMBRO)?;
        l.pular(n * v55::MEMBRO)?;
    }
    let n = l.contador(u32_em(b, 0x1a2), v55::MONSTRO)?;
    let mut monstros = Vec::with_capacity(n);
    for _ in 0..n {
        let m = l.bytes(v55::MONSTRO)?;
        monstros.push(MonstroPedido {
            monstro: u32_em(m, 0), quantidade: u32_em(m, 4),
            item_que_cai: u32_em(m, 8), quantidade_do_item: u32_em(m, 12),
            item_comum: m[16] != 0, chance_do_item: f32_em(m, 17),
            nivel_do_matador: m[21] != 0, dps: 0, dph: 0,
        });
    }
    let coleta = itens_v55(l, u32_em(b, 0x1aa))?;
    let rewards = premio_v55(l)?;
    let premio_de_falha = premio_v55(l)?;
    escala_v55(l, 20)?;
    escala_v55(l, 20)?;
    escala_v55(l, 24)?;
    escala_v55(l, 24)?;
    let descricao = texto_longo(l, id)?;
    for _ in 0..3 { texto_longo(l, id)?; }
    for _ in 0..5 { dialogo_v55(l)?; }

    let mut tarefa = TaskTemplate {
        id, name: texto(&b[4..64], id), parent: pai,
        itens_exigidos, itens_entregues, monster_kills: monstros,
        item_collections: coleta, rewards, premio_de_falha, descricao,
        janelas_de_horario: horarios, janelas,
        // `m_ulDelvNPC` em 0xb5: tarefa 1173 aponta ao NPC 3517
        // (TaskTempl.h:2112; conferido com o registro do realm v55).
        npc_que_entrega: u32_em(b, 0xb5),
        npc_que_premia: u32_em(b, 0xb9),
        // `m_ulOccupations`/`m_Occupations` e `m_enumMethod`/`m_enumFinishType`:
        // TaskTempl.h:2243-2244,2346-2347; no v55 as contagens e os
        // métodos são confirmados pelo bloco fixo e pelos vetores lidos em
        // `validar_tasks_v55.py:119-131` (0x19a=1 nas tarefas de caça).
        req_classes: lista_u32(b, 0x11d, u32_em(b, 0x119), 8),
        metodo: u32_em(b, 0x19a),
        tipo_de_conclusao: u32_em(b, 0x19e),
        // B102 — as flags de filhas e de desistência, `pack(1)` logo após `m_lPeriodLimit`
        // (`TaskTempl.h:2037-2057` do 1.5.3: `m_ulTimetable` 0x4e, `m_tmType[8]` 0x52, dois
        // ponteiros e dois `long` → `m_bChooseOne` em 0x6a). No binário do servidor 1.2.6
        // (`files1.2.6/pwserver/gamed/libtask.so`): `ATaskTempl::CheckDepth` (0x197b6) testa
        // +0x6c/+0x6a/+0x6b na ordem de `m_bExeChildInOrder || m_bChooseOne || m_bRandOne`,
        // e `ATaskTemplMan::CanGiveUpTask`/`GiveUpOneTask` leem +0x6f (`m_bCanGiveUp`).
        // Sem elas a 1177 ativava as duas filhas juntas e o cliente 1.2.6 perdia a lista.
        escolhe_um_filho: b[0x6a] != 0,
        sorteia_um_filho: b[0x6b] != 0,
        filhos_em_ordem: b[0x6c] != 0,
        pai_tambem_falha: b[0x6d] != 0,
        pai_tambem_sucesso: b[0x6e] != 0,
        pode_desistir: b[0x6f] != 0,
        pode_repetir: b[0x70] != 0,
        refazer_apos_falha: b[0x71] != 0,
        limpa_ao_desistir: b[0x72] != 0,
        precisa_registro: b[0x73] != 0,
        falha_ao_morrer: b[0x74] != 0,
        // B107 — medidos no `libtask.so` 1.2.6, todos com base `this + 4` (o bloco fixo):
        // `AddOneTaskTempl` (0x1cc8e/0x1ccbf) testa +0xad (`m_bDeathTrig`) e +0xac
        // (`m_bAutoDeliver`), como `TaskTemplMan.cpp:1735-1736`; `CheckLevel` (0xfafd/0xfb26)
        // lê +0xc1/+0xc5; `CheckPreTask` (0xff79/0xff8f) conta em +0xf1 e o vetor em +0xf5
        // (5 posições: o que cabe até o gênero em +0x114, `CheckGender` 0xfd36, pela ordem do
        // `TaskTempl.h:2227-2237` do 1.5.3); `CheckInZone` (0xf70f-0xf752) lê +0x79, +0x7a e a
        // caixa em +0x7e/+0x8a. Sem eles, o 1.2.6 não tinha missão automática nenhuma.
        entrega_automatica: b[0xac] != 0,
        min_level: u32_em(b, 0xc1),
        max_level: u32_em(b, 0xc5),
        pre_tasks: lista_u32(b, 0xf5, u32_em(b, 0xf1), 5),
        genero: u32_em(b, 0x114),
        entrega_em_zona: b[0x79] != 0,
        mundo_de_entrega: u32_em(b, 0x7a),
        regioes_de_entrega: if b[0x79] != 0 {
            vec![RegiaoDeMissao {
                min: [f32_em(b, 0x7e), f32_em(b, 0x82), f32_em(b, 0x86)],
                max: [f32_em(b, 0x8a), f32_em(b, 0x8e), f32_em(b, 0x92)],
            }]
        } else {
            Vec::new()
        },
        // B110 — o lugar a alcançar (`enumTMReachSite`, método 4): `OnTaskReachSite` do
        // `libtask.so` 1.2.6 (0x1fa00-0x1fa6e) testa +0x19a == 4, compara o mundo com +0x1de
        // (base `this + 4`) e chama `is_in_zone(this + 0x1ca, this + 0x1d6, pos)` — no bloco
        // fixo, mínimo +0x1c6 e máximo +0x1d2 — e então `OnSetFinished`. Sem isto a 5911
        // "Instruções" (filha da automática 5909) nunca se cumpria.
        mundo_a_alcancar: if u32_em(b, 0x19a) == 4 { u32_em(b, 0x1de) } else { 0 },
        lugares_a_alcancar: if u32_em(b, 0x19a) == 4 {
            vec![RegiaoDeMissao {
                min: [f32_em(b, 0x1c6), f32_em(b, 0x1ca), f32_em(b, 0x1ce)],
                max: [f32_em(b, 0x1d2), f32_em(b, 0x1d6), f32_em(b, 0x1da)],
            }]
        } else {
            Vec::new()
        },
        profundidade: 1, ..Default::default()
    };
    let filhos = l.i32()?;
    if filhos < 0 { return Err(TasksError::InvalidFormat); }
    let n = l.contador(filhos as u32, v55::FIXO)?;
    for _ in 0..n { tarefa.sub_tasks.push(missao_v55(l, Some(id), saida)?); }
    if saida.insert(id, tarefa).is_some() { return Err(TasksError::InvalidFormat); }
    Ok(id)
}

/// `ATaskTempl::LoadBinary`: uma missão e, recursivamente, as submissões. Devolve o id.
fn missao(l: &mut Leitor, pai: Option<u32>, saida: &mut HashMap<u32, TaskTemplate>) -> Result<u32> {
    use v129::f;
    let b = l.bytes(v129::FIXO)?;
    let id = u32_em(b, f::ID);
    let flag = |o: usize| b[o] != 0;

    // LoadFixedDataFromBinFile — a ordem é a do fonte (TaskTempl.cpp:3876-4119).
    if flag(f::TEM_ASSINATURA) {
        l.pular(v129::MAX_TASK_NAME_LEN * v129::TASK_CHAR)?;
    }
    // `m_tmStart[i]` e `m_tmEnd[i]` alternados (`TaskTempl.cpp:3885-3897`); o tipo de cada
    // janela está no bloco fixo.
    let n = l.contador(u32_em(b, f::TIMETABLE), 2 * v129::TASK_TM)?;
    let mut janelas = Vec::with_capacity(n);
    for i in 0..n {
        let inicio = momento(l.bytes(v129::TASK_TM)?);
        let fim = momento(l.bytes(v129::TASK_TM)?);
        let tipo = if i < 24 { b[f::TM_TYPE + i] } else { 0 };
        janelas.push(JanelaDeHorario { tipo, inicio, fim });
    }
    for (campo, tamanho) in [
        (f::CHANGE_KEY_CNT, v129::CHANGE_KEY),
        (f::PQ_EXP_CNT, v129::EXPRESSAO),
        (f::MONSTER_CONTRIB_CNT, v129::MONSTERS_CONTRIB),
    ] {
        let n = l.contador(u32_em(b, campo), tamanho)?;
        l.pular(n * tamanho)?;
    }
    let regioes_de_entrega = regioes(l, u32_em(b, f::DELV_REGION_CNT))?;
    for campo in [f::ENTER_REGION_CNT, f::LEAVE_REGION_CNT] {
        let n = l.contador(u32_em(b, campo), v129::TASK_REGION)?;
        l.pular(n * v129::TASK_REGION)?;
    }
    let itens_exigidos = itens(l, u32_em(b, f::PREM_ITEMS))?;
    let itens_entregues = itens(l, u32_em(b, f::GIVEN_ITEMS))?;
    let mut membros_pedidos = Vec::new();
    if flag(f::TEAMWORK) {
        let n = l.contador(u32_em(b, f::TEAM_MEMS_WANTED), v129::TEAM_MEM_WANTED)?;
        for _ in 0..n {
            let m = l.bytes(v129::TEAM_MEM_WANTED)?;
            membros_pedidos.push(MembroPedido {
                nivel_minimo: u32_em(m, 0),
                nivel_maximo: u32_em(m, 4),
                raca: u32_em(m, 8),
                classe: u32_em(m, 12),
                genero: u32_em(m, 16),
                minimo: u32_em(m, 20),
                maximo: u32_em(m, 24),
                missao: u32_em(m, 28),
                forca: u32_em(m, 32) as i32,
            });
        }
    }
    let titulos = u32_em(b, f::PREM_TITLE_TOTAL) as i32;
    if titulos > 0 {
        let n = l.contador(titulos as u32, 4)?;
        l.pular(n * 4)?;
    }
    let n = l.contador(u32_em(b, f::MONSTER_WANTED), v129::MONSTER_WANTED)?;
    let mut monstros = Vec::with_capacity(n);
    for _ in 0..n {
        let m = l.bytes(v129::MONSTER_WANTED)?;
        monstros.push(MonstroPedido {
            monstro: u32_em(m, 0),
            quantidade: u32_em(m, 4),
            item_que_cai: u32_em(m, 8),
            quantidade_do_item: u32_em(m, 12),
            item_comum: m[16] != 0,
            chance_do_item: f32_em(m, 17),
            nivel_do_matador: m[21] != 0,
            dps: u32_em(m, 22) as i32,
            dph: u32_em(m, 26) as i32,
        });
    }
    let n = l.contador(u32_em(b, f::PLAYER_WANTED), v129::PLAYER_WANTED)?;
    l.pular(n * v129::PLAYER_WANTED)?;
    let coleta = itens(l, u32_em(b, f::ITEMS_WANTED))?;
    for (campo, tamanho) in [
        (f::EXP_CNT, v129::EXPRESSAO),
        (f::TASK_CHAR_CNT, v129::TASK_CHAR * v129::EXP_LEN),
    ] {
        let n = l.contador(u32_em(b, campo), tamanho)?;
        l.pular(n * tamanho)?;
    }
    let lugares_a_alcancar = regioes(l, u32_em(b, f::REACH_SITE_CNT))?;
    let lugares_a_sair = regioes(l, u32_em(b, f::LEAVE_SITE_CNT))?;
    let n = l.contador(u32_em(b, f::HOME_ITEMS_WANTED), v129::HOME_ITEM_WANTED)?;
    l.pular(n * v129::HOME_ITEM_WANTED)?;

    let rewards = premio(l)?;
    let premio_de_falha = premio(l)?;
    let degraus_de_razao = 4 * v129::MAX_AWARD_SCALES;
    escala(l, degraus_de_razao)?;
    escala(l, degraus_de_razao)?;
    escala(l, 4 + 4 * v129::MAX_AWARD_SCALES)?;
    escala(l, 4 + 4 * v129::MAX_AWARD_SCALES)?;

    let descricao = texto_longo(l, id)?;
    texto_longo(l, id)?; // ok
    texto_longo(l, id)?; // no
    texto_longo(l, id)?; // tributo
    for _ in 0..5 {
        dialogo(l)?;
    }

    let mut tarefa = TaskTemplate {
        id,
        name: texto(&b[f::NOME..f::NOME + v129::MAX_TASK_NAME_LEN * v129::TASK_CHAR], id),
        parent: pai.or_else(|| Some(u32_em(b, f::PAI)).filter(|&p| p != 0)),
        sub_tasks: Vec::new(),
        tipo: u32_em(b, f::TIPO),
        limite_de_tempo: u32_em(b, f::LIMITE_DE_TEMPO),
        min_level: u32_em(b, f::NIVEL_MIN),
        max_level: u32_em(b, f::NIVEL_MAX),
        req_classes: lista_u32(b, f::OCCUPATIONS, u32_em(b, f::OCCUPATIONS_CNT), 12),
        genero: u32_em(b, f::GENERO),
        pre_tasks: lista_u32(b, f::PRE_TASKS, u32_em(b, f::PRE_TASK_CNT), 20),
        missoes_exclusivas: lista_u32(b, f::MUTEX, u32_em(b, f::MUTEX_CNT), 5),
        itens_exigidos,
        itens_entregues,
        npc_que_entrega: u32_em(b, f::NPC_QUE_ENTREGA),
        npc_que_premia: u32_em(b, f::NPC_QUE_PREMIA),
        metodo: u32_em(b, f::METODO),
        tipo_de_conclusao: u32_em(b, f::TIPO_DE_CONCLUSAO),
        monster_kills: monstros,
        item_collections: coleta,
        dinheiro_pedido: u32_em(b, f::DINHEIRO_PEDIDO),
        nivel_a_alcancar: u32_em(b, f::NIVEL_A_ALCANCAR),
        mundo_a_alcancar: u32_em(b, f::MUNDO_A_ALCANCAR),
        espera: u32_em(b, f::ESPERA),
        entrega_automatica: flag(f::ENTREGA_AUTOMATICA),
        pode_desistir: flag(f::PODE_DESISTIR),
        pode_repetir: flag(f::PODE_REPETIR),
        precisa_registro: flag(f::PRECISA_REGISTRO),
        escolhe_um_filho: flag(f::ESCOLHE_UM),
        sorteia_um_filho: flag(f::SORTEIA_UM),
        filhos_em_ordem: flag(f::FILHOS_EM_ORDEM),
        oculta: flag(f::OCULTA),
        missao_chave: flag(f::MISSAO_CHAVE),
        descricao,
        rewards,
        premio_de_falha,
        item_nao_retirado: flag(f::ITEM_NAO_RETIRADO),
        tempo_absoluto: flag(f::TEMPO_ABSOLUTO),
        janelas_de_horario: u32_em(b, f::TIMETABLE),
        frequencia: u32_em(b, f::FREQUENCIA) as i32,
        limite_por_periodo: u32_em(b, f::LIMITE_POR_PERIODO) as i32,
        pai_tambem_falha: flag(f::PAI_TAMBEM_FALHA),
        pai_tambem_sucesso: flag(f::PAI_TAMBEM_SUCESSO),
        refazer_apos_falha: flag(f::REFAZER_APOS_FALHA),
        limpa_ao_desistir: flag(f::LIMPA_AO_DESISTIR),
        falha_ao_morrer: flag(f::FALHA_AO_MORRER),
        max_receptores: u32_em(b, f::MAX_RECEPTORES),
        limpa_adquiridos: flag(f::LIMPA_ADQUIRIDOS),
        mostra_aviso: flag(f::MOSTRA_AVISO),
        casamento: flag(f::CASAMENTO),
        compara_bolsa: flag(f::COMPARA_BOLSA),
        slots_de_bolsa: u32_em(b, f::SLOTS_DE_BOLSA),
        torre: flag(f::TORRE),
        pq: flag(f::PQ),
        pq_sub: flag(f::PQ_SUB),
        limite_de_conta: flag(f::LIMITE_DE_CONTA),
        limite_de_personagem: flag(f::LIMITE_DE_PERSONAGEM),
        nao_conta_falha: flag(f::NAO_CONTA_FALHA),
        nao_limpa_item_na_falha: flag(f::NAO_LIMPA_ITEM_NA_FALHA),
        na_janela_de_titulo: flag(f::NA_JANELA_DE_TITULO),
        nivel_maximo_historico: u32_em(b, f::NIVEL_MAXIMO_HISTORICO) != 0,
        itens_exigidos_qualquer_um: flag(f::ITENS_QUALQUER_UM),
        entregues_comuns: u32_em(b, f::ENTREGUES_COMUNS),
        entregues_de_missao: u32_em(b, f::ENTREGUES_DE_MISSAO),
        deposito: u32_em(b, f::DEPOSITO),
        reputacao_minima: u32_em(b, f::REPUTACAO_MIN) as i32,
        pre_missoes_minimo: u32_em(b, f::PRE_MISSOES_MINIMO),
        periodo: u32_em(b, f::PERIODO),
        so_gm: flag(f::GM),
        cotask: u32_em(b, f::COTASK),
        em_equipe: flag(f::TEAMWORK),
        recebida_pela_equipe: flag(f::RECEBIDA_PELA_EQUIPE),
        entrega_em_zona: flag(f::ENTREGA_EM_ZONA),
        faccao: u32_em(b, f::FACCAO),
        papel_na_faccao: u32_em(b, f::PAPEL_NA_FACCAO) as i32,
        teleporte_ao_receber: flag(f::TRANS_TO).then(|| {
            (u32_em(b, f::TRANS_WLD), [f32_em(b, f::TRANS_PT), f32_em(b, f::TRANS_PT + 4), f32_em(b, f::TRANS_PT + 8)])
        }),
        janelas,
        mundo_de_entrega: u32_em(b, f::MUNDO_DE_ENTREGA),
        regioes_de_entrega,
        lugares_a_alcancar,
        mundo_a_sair: u32_em(b, f::MUNDO_A_SAIR),
        lugares_a_sair,
        membros_pedidos,
        confere_membros: flag(f::CONFERE_MEMBROS),
        distancia_dos_membros: f32_em(b, f::DISTANCIA_DOS_MEMBROS),
        classes_distintas: flag(f::CLASSES_DISTINTAS),
        so_casal: flag(f::SO_CASAL),
        conjuge: flag(f::CONJUGE),
        tipo_de_premio_sucesso: u32_em(b, f::TIPO_DE_PREMIO_SUCESSO),
        tipo_de_premio_falha: u32_em(b, f::TIPO_DE_PREMIO_FALHA),
        profundidade: 1,
    };

    let filhos = l.i32()?.max(0) as u32;
    let filhos = l.contador(filhos, v129::FIXO)?;
    for _ in 0..filhos {
        tarefa.sub_tasks.push(missao(l, Some(id), saida)?);
    }
    saida.insert(id, tarefa);
    Ok(id)
}

/// `ATaskTempl::CheckDepth` (`TaskTempl.h:2748-2771`): folha vale 1; com filhos, soma a
/// profundidade dos filhos — ou só a maior, quando eles correm em ordem, quando se escolhe um
/// ou quando se sorteia um. É `unsigned char` no original: a soma dá a volta em 256.
fn calcular_profundidade(tarefas: &mut HashMap<u32, TaskTemplate>, id: u32) -> u8 {
    let Some(t) = tarefas.get(&id) else { return 0 };
    let filhos = t.sub_tasks.clone();
    let maior = t.filhos_em_ordem || t.escolhe_um_filho || t.sorteia_um_filho;
    let mut acumulado: u8 = 0;
    for f in filhos {
        let d = calcular_profundidade(tarefas, f);
        if maior {
            acumulado = acumulado.max(d);
        } else {
            acumulado = acumulado.wrapping_add(d);
        }
    }
    let t = tarefas.get_mut(&id).expect("conferido acima");
    t.profundidade = 1u8.wrapping_add(acumulado);
    t.profundidade
}

impl TaskTemplate {
    /// Uma missão sem nada: sem requisito, sem objetivo, sem prêmio. Os campos do arquivo
    /// que valem `true` por padrão no editor (`m_bParentAlsoFail`, `m_bCanRedoAfterFailure`,
    /// `m_bClearAcquired`, `m_bShowPrompt`, `m_bCanGiveUp`) vêm ligados. Para testes e para
    /// quem monta missão fora do `tasks.data`.
    pub fn vazia(id: u32) -> Self {
        Self {
            id,
            name: String::new(),
            parent: None,
            sub_tasks: Vec::new(),
            tipo: 0,
            limite_de_tempo: 0,
            min_level: 0,
            max_level: 0,
            req_classes: Vec::new(),
            genero: 0,
            pre_tasks: Vec::new(),
            missoes_exclusivas: Vec::new(),
            itens_exigidos: Vec::new(),
            itens_entregues: Vec::new(),
            npc_que_entrega: 0,
            npc_que_premia: 0,
            metodo: 0,
            tipo_de_conclusao: 0,
            monster_kills: Vec::new(),
            item_collections: Vec::new(),
            dinheiro_pedido: 0,
            nivel_a_alcancar: 0,
            mundo_a_alcancar: 0,
            espera: 0,
            entrega_automatica: false,
            pode_desistir: true,
            pode_repetir: false,
            precisa_registro: true,
            escolhe_um_filho: false,
            sorteia_um_filho: false,
            filhos_em_ordem: false,
            oculta: false,
            missao_chave: false,
            descricao: String::new(),
            rewards: TaskReward::default(),
            premio_de_falha: TaskReward::default(),
            item_nao_retirado: false,
            tempo_absoluto: false,
            janelas_de_horario: 0,
            frequencia: 0,
            limite_por_periodo: 0,
            pai_tambem_falha: true,
            pai_tambem_sucesso: false,
            refazer_apos_falha: true,
            limpa_ao_desistir: false,
            falha_ao_morrer: false,
            max_receptores: 0,
            limpa_adquiridos: true,
            mostra_aviso: true,
            casamento: false,
            compara_bolsa: false,
            slots_de_bolsa: 0,
            torre: false,
            pq: false,
            pq_sub: false,
            limite_de_conta: false,
            limite_de_personagem: false,
            nao_conta_falha: false,
            nao_limpa_item_na_falha: false,
            na_janela_de_titulo: false,
            nivel_maximo_historico: false,
            itens_exigidos_qualquer_um: false,
            entregues_comuns: 0,
            entregues_de_missao: 0,
            deposito: 0,
            reputacao_minima: 0,
            pre_missoes_minimo: 0,
            periodo: 0,
            so_gm: false,
            cotask: 0,
            em_equipe: false,
            recebida_pela_equipe: false,
            entrega_em_zona: false,
            faccao: 0,
            papel_na_faccao: 0,
            teleporte_ao_receber: None,
            janelas: Vec::new(),
            mundo_de_entrega: 0,
            regioes_de_entrega: Vec::new(),
            lugares_a_alcancar: Vec::new(),
            mundo_a_sair: 0,
            lugares_a_sair: Vec::new(),
            membros_pedidos: Vec::new(),
            confere_membros: false,
            distancia_dos_membros: 0.0,
            classes_distintas: false,
            so_casal: false,
            conjuge: false,
            tipo_de_premio_sucesso: 0,
            tipo_de_premio_falha: 0,
            profundidade: 1,
        }
    }
}

impl TasksData {
    /// Acrescenta uma missão já montada (e as submissões, se vierem antes). Para testes.
    pub fn inserir(&mut self, t: TaskTemplate) {
        if t.parent.is_none() {
            self.de_topo.push(t.id);
        }
        self.tasks.insert(t.id, t);
    }

    /// Lê o cabeçalho: `(version, item_count)`.
    ///
    /// # Formato (autoridade)
    ///
    /// `CElementClient/Task/TaskTempl.h:224`:
    ///
    /// ```cpp
    /// #define TASK_PACK_MAGIC 0x93858361
    /// struct TASK_PACK_HEADER { unsigned long magic; unsigned long version; unsigned long item_count; };
    /// ```
    ///
    /// E `TaskTemplMan.cpp:1599`: o cliente recusa o arquivo se
    /// `tph.version != _task_templ_cur_version`. Ou seja — como no `elements.data` — a
    /// `version` deste arquivo **é** a constante do cliente que consegue abri-lo, e é ela
    /// que tem que ir para a string `edition` do handshake.
    ///
    /// Isto era lido como "a primeira palavra é a versão", o que devolvia o mágico
    /// `0x93858361` como se fosse número de versão.
    pub fn ler_cabecalho(data: &[u8]) -> Result<(u32, u32)> {
        if data.len() < 12 {
            return Err(TasksError::InvalidFormat);
        }
        if u32_em(data, 0) != MAGICO_DO_TASKS {
            return Err(TasksError::InvalidFormat);
        }
        Ok((u32_em(data, 4), u32_em(data, 8)))
    }

    /// Lê o arquivo inteiro para v55/v129. Versão sem layout medido devolve só
    /// o cabeçalho (com aviso); versão suportada que não fecha é erro.
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let (version, quantidade_declarada) = Self::ler_cabecalho(data)?;
        info!(
            "Carregando tasks.data: _task_templ_cur_version = {}, {} missões declaradas",
            version, quantidade_declarada
        );
        let mut tasks_data = Self { version, quantidade_declarada, ..Default::default() };

        if version != VERSAO_SUPORTADA && version != 55 {
            warn!(
                "tasks.data v{}: layout não medido (só v55 e v{} são lidas); nenhuma missão carregada",
                version, VERSAO_SUPORTADA
            );
            return Ok(tasks_data);
        }

        let n = quantidade_declarada as usize;
        let fim_tabela = 12usize.checked_add(n.checked_mul(4).ok_or(TasksError::InvalidFormat)?)
            .ok_or(TasksError::InvalidFormat)?;
        let tabela = data.get(12..fim_tabela).ok_or(TasksError::Truncado(12))?;
        let inicios: Vec<usize> = (0..n).map(|i| u32_em(tabela, 4 * i) as usize).collect();

        for (indice, &inicio) in inicios.iter().enumerate() {
            let esperado = inicios.get(indice + 1).copied().unwrap_or(data.len());
            if inicio < fim_tabela || esperado > data.len() || inicio >= esperado {
                return Err(TasksError::InvalidFormat);
            }
            let mut l = Leitor { d: &data[..esperado], o: inicio };
            let id = if version == 55 {
                missao_v55(&mut l, None, &mut tasks_data.tasks)?
            } else {
                missao(&mut l, None, &mut tasks_data.tasks)?
            };
            if l.o != esperado {
                return Err(TasksError::Desalinhado { indice, id, inicio, fim: l.o, esperado });
            }
            tasks_data.de_topo.push(id);
        }
        for id in tasks_data.de_topo.clone() {
            calcular_profundidade(&mut tasks_data.tasks, id);
        }

        info!(
            "tasks.data carregado: {} missões de topo, {} ao todo com as submissões",
            tasks_data.de_topo.len(),
            tasks_data.tasks.len()
        );
        Ok(tasks_data)
    }

    pub fn get_task(&self, task_id: u32) -> Option<&TaskTemplate> {
        self.tasks.get(&task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_texto_desfaz_o_xor_pelo_id() {
        let id = 35860u32;
        let original = "Olá";
        let bytes: Vec<u8> = original
            .encode_utf16()
            .chain([0u16, 0u16])
            .flat_map(|u| (u ^ id as u16).to_le_bytes())
            .collect();
        assert_eq!(texto(&bytes, id), original);
    }

    #[test]
    fn versao_sem_layout_so_le_o_cabecalho() {
        let mut d = Vec::new();
        d.extend(MAGICO_DO_TASKS.to_le_bytes());
        d.extend(124u32.to_le_bytes());
        d.extend(3u32.to_le_bytes());
        let t = TasksData::load_from_bytes(&d).unwrap();
        assert_eq!((t.version, t.quantidade_declarada), (124, 3));
        assert!(t.tasks.is_empty());
    }

    #[test]
    fn arquivo_truncado_e_erro_e_nao_panico() {
        let mut d = Vec::new();
        d.extend(MAGICO_DO_TASKS.to_le_bytes());
        d.extend(VERSAO_SUPORTADA.to_le_bytes());
        d.extend(1u32.to_le_bytes());
        d.extend(16u32.to_le_bytes());
        d.extend([0u8; 100]);
        assert!(matches!(TasksData::load_from_bytes(&d), Err(TasksError::Truncado(_))));
    }
}
