//! O motor de missões: as listas binárias do jogador e as regras de aceitar, contar abates e
//! premiar.
//!
//! # Por que as listas são binárias
//!
//! O cliente **não recebe** o estado das missões depois do login: ele recebe as listas no
//! `TASK_DATA` (105) e, a cada aviso do servidor (`TASK_SVR_NOTIFY_*`), refaz sozinho a
//! mesma operação sobre a cópia dele — entrega, prêmio, desistência
//! (`ATaskTempl::OnServerNotify`, `task/TaskProcess.cpp:2643-2815`). Se a lista do servidor
//! divergir da do cliente em um índice, o próximo aviso cai na entrada errada. Por isso este
//! módulo porta as estruturas (`task/TaskProcess.h:103-392`, `pack(1)`) e as funções que as
//! mexem **literalmente**: `DeliverTask`, `RealignTask`, `RecursiveClearTask`,
//! `RecursiveAward`, `FinishedTaskList::AddOneTask`.
//!
//! O defeito de 2026-09-14 ("consigo dialogar, mas ao aceitar não aparece nada") era
//! exatamente isto: o `TASK_DATA` ia com os blocos vazios, a lista do cliente ficava com
//! `m_Version = 0`, e `OnServerNotify` (`TaskClient.cpp:262`) descarta **todo** aviso quando
//! a versão não é `TASK_ENTRY_DATA_CUR_VER` (1).
//!
//! # O que não está portado (e o que acontece nesses casos)
//!
//! Portados em 2026-09-16: janelas de horário (`CheckTimetable`/`judge_time_date`, na hora
//! local do servidor), região de entrega (`CheckInZone`), facção (`CheckFaction` — sem sistema
//! de facção ninguém está em uma, e o original recusa igual), equipe recebida pelo capitão
//! (`CheckTeamTask`/`HasAllTeamMemsWanted` e `OnDeliverTeamMemTask`), chegar e sair de lugar
//! (`OnTaskReachSite`/`OnTaskLeaveSite`).
//!
//! Sistemas que o servidor ainda não tem: casamento, PQ, torre, variáveis globais (lidas como
//! zero, que é o valor de variável não definida no original), prêmios por escala de
//! tempo/itens (`enumTATRatio`/`ItemCount`, prêmio vazio), teleporte de prêmio, invocação de
//! monstros, depósito de missões, falha/sucesso compartilhados pela equipe
//! (`AwardNotifyTeamMem`) e abate contado para a equipe. Ver
//! `specs/05_SIMULACAO_DO_MUNDO.md`.

use pw_data_loader::tasks::{ItemDeMissao, TaskReward, TaskTemplate, TasksData};
use pw_protocol::S2CGamedataSend;

/// `TASK_ACTIVE_LIST_MAX_LEN` (`TaskInterface.h:101`).
pub const MAX_ATIVAS: usize = 175;
/// `TASK_DATA_BUF_MAX_LEN` — o tamanho de uma entrada.
pub const TAM_ENTRADA: usize = 32;
/// `TASK_ACTIVE_LIST_HEADER_LEN`.
pub const TAM_CABECALHO: usize = 8;
/// `TASK_ENTRY_DATA_CUR_VER` (`TaskProcess.cpp:21`).
pub const VERSAO_DA_LISTA: u16 = 1;
/// `TASK_FINISHED_LIST_MAX_LEN`, `TASK_FINISH_TIME_MAX_LEN`, `TASK_FINISH_COUNT_MAX_LEN`.
pub const MAX_CONCLUIDAS: usize = 4080;
pub const MAX_TEMPOS: usize = 2500;
pub const MAX_CONTAGENS: usize = 730;
/// `sizeof(StorageTaskList)`: 32×10×2 + 32×2 + 32×4 + 32 (`TaskProcess.h:379-391`).
pub const TAM_DEPOSITO: usize = 864;
/// `TASK_DEFAULT_MAX_SIMULTANEOUS_COUT`, `TASK_MAX_SIMULTANEOUS_COUT`, `TASK_HIDDEN_COUNT`,
/// `TASK_TITLE_TASK_COUNT` (`TaskProcess.h:92-95`).
const MAX_SIMULTANEAS: u8 = 60;
const MAX_SIMULTANEAS_EXPANDIDO: u8 = 80;
const MAX_OCULTAS: u8 = 30;
const MAX_DE_TITULO: u8 = 10;
/// `MAX_SUB_TAGS` (`TaskTempl.h:1735`).
const MAX_SUB_TAGS: usize = 32;
/// `MONSTER_PLAYER_LEVEL_MAX_DIFF` (`TaskTempl.inl:6`).
const DIFERENCA_MAXIMA_DE_NIVEL: u32 = 8;
/// `MAX_PLAYER_LEV` e `_lev_co` (`TaskProcess.cpp:22-175`).
const MAX_NIVEL_DO_COEFICIENTE: u32 = 150;
const SEM: u8 = 0xff;

/// `TLIST_STATE_UPDATE_TIME_MARK` — os tempos da lista estão em hora absoluta.
const LISTA_COM_TEMPO_ABSOLUTO: u8 = 1;

#[rustfmt::skip]
const COEFICIENTE_DE_NIVEL: [u32; 150] = [
    1, 1, 2, 4, 7, 9, 12, 16, 20, 25, 30, 35, 40, 46, 52, 59, 65, 72, 79, 87,
    95, 102, 110, 118, 127, 135, 143, 152, 157, 158, 168, 179, 190, 202, 214, 227, 239, 253, 267, 281,
    296, 311, 327, 343, 360, 377, 390, 402, 414, 426, 438, 449, 460, 471, 480, 489, 497, 505, 510, 515,
    535, 553, 588, 624, 660, 698, 736, 775, 814, 853, 893, 978, 1066, 1157, 1250, 1344, 1438, 1530, 1620, 1705,
    1784, 1853, 1910, 1953, 1976, 1983, 2116, 2213, 2304, 2455, 2679, 2837, 2990, 3136, 3272, 3282, 3388, 3477, 3543, 4000,
    4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000,
    4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000,
    4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000,
];

/// `TASK_STATE_*` (`TaskProcess.h:105-110`).
pub mod estado {
    pub const FINALIZADA: u8 = 0x01;
    pub const SUCESSO: u8 = 0x02;
    pub const DESISTIU: u8 = 0x04;
    pub const ERRO_AVISADO: u8 = 0x08;
}

/// `enumTM*` (`TaskTempl.h:173-191`).
pub mod metodo {
    pub const NENHUM: u32 = 0;
    pub const MATAR_MONSTROS: u32 = 1;
    pub const COLETAR_ITENS: u32 = 2;
    pub const FALAR_COM_NPC: u32 = 3;
    pub const ALCANCAR_LUGAR: u32 = 4;
    pub const ESPERAR: u32 = 5;
    pub const SAIR_DE_LUGAR: u32 = 11;
    pub const ALCANCAR_NIVEL: u32 = 15;
    pub const SIMPLES_DO_CLIENTE: u32 = 16;
    pub const SIMPLES_DO_CLIENTE_NAVEGACAO: u32 = 17;
}

/// `enumTFT*` (`TaskTempl.h:197-199`).
pub mod conclusao {
    pub const DIRETA: u32 = 0;
    pub const NO_NPC: u32 = 1;
    pub const CONFIRMAR: u32 = 2;
}

/// `TASK_PREREQU_FAIL_*` e `TASK_AWARD_FAIL_*` (`TaskInterface.h:12-92`).
pub mod erro {
    pub const INDETERMINADO: u32 = 1;
    pub const NAO_E_RAIZ: u32 = 2;
    pub const MESMA_MISSAO: u32 = 3;
    pub const SEM_ESPACO: u32 = 4;
    pub const CHEIA: u32 = 5;
    pub const NAO_REPETE: u32 = 6;
    pub const NIVEL_BAIXO: u32 = 7;
    pub const NIVEL_ALTO: u32 = 8;
    pub const SEM_ITEM: u32 = 9;
    pub const REPUTACAO: u32 = 10;
    pub const FACCAO: u32 = 11;
    pub const GENERO: u32 = 12;
    pub const CLASSE: u32 = 13;
    pub const PERIODO: u32 = 14;
    pub const MISSAO_ANTERIOR: u32 = 15;
    pub const DEPOSITO: u32 = 17;
    pub const NAO_E_CAPITAO: u32 = 19;
    pub const MEMBRO_INVALIDO: u32 = 20;
    pub const HORARIO: u32 = 21;
    pub const EXCLUSIVA: u32 = 23;
    pub const FORA_DA_ZONA: u32 = 24;
    pub const SUBMISSAO_ERRADA: u32 = 25;
    pub const LONGE_DA_EQUIPE: u32 = 26;
    pub const ITEM_ENTREGUE: u32 = 27;
    pub const GM: u32 = 30;
    pub const JA_TEM_PQ: u32 = 33;
    pub const NAO_E_CASAL: u32 = 36;
    pub const CONTA_NO_LIMITE: u32 = 34;
    pub const SLOTS_DE_BOLSA: u32 = 38;
    pub const PERSONAGEM_NO_LIMITE: u32 = 63;
    pub const PREMIO_ITEM: u32 = 150;
    pub const PREMIO_REPUTACAO: u32 = 152;
    pub const PREMIO_NIVEL: u32 = 160;
}

/// `TASK_SVR_NOTIFY_*` (`TaskTempl.h`).
pub mod aviso {
    pub const NOVA: u8 = 1;
    pub const CONCLUIDA: u8 = 2;
    pub const DESISTENCIA: u8 = 3;
    pub const FINALIZADA: u8 = 5;
}

// =============================================================================== listas

/// `ActiveTaskEntry` (`TaskProcess.h:113-197`), 32 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entrada {
    pub id: u16,
    pub pai: u8,
    pub anterior: u8,
    pub proximo: u8,
    pub filho: u8,
    pub estado: u8,
    pub tempo: u32,
    pub capitao: u16,
    /// `m_ulTemplAddr != 0`. No original é um ponteiro; aqui, "o modelo foi resolvido".
    /// No fio vai o id no lugar do ponteiro — o cliente recalcula o dele ao receber a lista
    /// (`InitActiveTaskList`, `TaskProcess.cpp:2231-2241`).
    pub valida: bool,
    /// `m_BufData[11]`: os três `m_wMonsterNum`, e o resto.
    pub buf: [u8; 11],
}

impl Entrada {
    pub fn finalizada(&self) -> bool {
        self.estado & estado::FINALIZADA != 0
    }
    pub fn sucesso(&self) -> bool {
        self.estado & estado::SUCESSO != 0
    }
    pub fn desistiu(&self) -> bool {
        self.estado & estado::DESISTIU != 0
    }
    pub fn monstros(&self, i: usize) -> u16 {
        u16::from_le_bytes([self.buf[2 * i], self.buf[2 * i + 1]])
    }
    fn definir_monstros(&mut self, i: usize, n: u16) {
        self.buf[2 * i..2 * i + 2].copy_from_slice(&n.to_le_bytes());
    }
}

/// `ActiveTaskList` (`TaskProcess.h:201-248`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListaAtiva {
    pub quantidade: u8,
    pub usados: u8,
    pub versao: u16,
    pub topo_visiveis: u8,
    pub estado: u8,
    pub topo_ocultas: u8,
    pub limite_expandido: bool,
    pub de_titulo: u8,
    pub e: Vec<Entrada>,
}

impl Default for ListaAtiva {
    fn default() -> Self {
        Self {
            quantidade: 0,
            usados: 0,
            versao: VERSAO_DA_LISTA,
            topo_visiveis: 0,
            estado: LISTA_COM_TEMPO_ABSOLUTO,
            topo_ocultas: 0,
            limite_expandido: false,
            de_titulo: 0,
            e: vec![Entrada::default(); MAX_ATIVAS],
        }
    }
}

impl ListaAtiva {
    pub fn indice(&self, id: u32) -> Option<usize> {
        (0..self.quantidade as usize).find(|&i| self.e[i].id as u32 == id)
    }

    fn max_simultaneas(&self) -> u8 {
        if self.limite_expandido { MAX_SIMULTANEAS_EXPANDIDO } else { MAX_SIMULTANEAS }
    }

    /// `ActiveTaskList::RealignTask` (`TaskProcess.cpp:3788-3866`).
    fn realinhar(&mut self, idx: usize, reserva: u8) {
        let atual = idx;
        let restantes = (self.quantidade as usize).saturating_sub(atual);
        if restantes == 0 {
            return;
        }
        let mut vazios = 0usize;
        for k in atual..MAX_ATIVAS {
            if self.e[k].id == 0 {
                vazios += 1;
            } else {
                break;
            }
        }
        if reserva as usize == vazios {
            return;
        }
        let origem = atual + vazios;
        let destino = atual + reserva as usize;
        let copia: Vec<Entrada> = (0..restantes).map(|k| self.e.get(origem + k).copied().unwrap_or_default()).collect();
        for (k, en) in copia.into_iter().enumerate() {
            if destino + k < MAX_ATIVAS {
                self.e[destino + k] = en;
            }
        }
        let (limpa_ini, limpa_fim) = if destino > origem {
            (origem, destino)
        } else {
            (destino + restantes, origem + restantes)
        };
        for k in limpa_ini..limpa_fim.min(MAX_ATIVAS) {
            self.e[k].valida = false;
            self.e[k].id = 0;
        }
        // `unsigned char uGap = pInsert - pSrc`: a diferença, dando a volta em 256.
        let vao = (destino as i64 - origem as i64) as u8;
        for k in 0..atual {
            let en = &mut self.e[k];
            if en.id == 0 {
                continue;
            }
            if en.filho != SEM && en.filho as usize >= atual {
                en.filho = en.filho.wrapping_add(vao);
            }
            if en.proximo != SEM && en.proximo as usize >= atual {
                en.proximo = en.proximo.wrapping_add(vao);
            }
        }
        for k in 0..restantes {
            let Some(en) = self.e.get_mut(destino + k) else { break };
            if en.id == 0 {
                continue;
            }
            if en.pai != SEM && en.pai as usize >= atual {
                en.pai = en.pai.wrapping_add(vao);
            }
            if en.anterior != SEM && en.anterior as usize >= atual {
                en.anterior = en.anterior.wrapping_add(vao);
            }
            if en.filho != SEM {
                en.filho = en.filho.wrapping_add(vao);
            }
            if en.proximo != SEM {
                en.proximo = en.proximo.wrapping_add(vao);
            }
        }
    }

    /// `TASK_DATA`: cabeçalho e as `quantidade` primeiras entradas
    /// (`GetActLstDataSize`, `TaskProcess.cpp:2315-2319`).
    pub fn para_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(TAM_CABECALHO + TAM_ENTRADA * self.quantidade as usize);
        b.push(self.quantidade);
        b.push(self.usados);
        b.extend_from_slice(&self.versao.to_le_bytes());
        b.push(self.topo_visiveis);
        b.push(self.estado);
        b.push(self.topo_ocultas);
        // `m_uMaxSimultaneousCount:1` no bit baixo, `m_uTitleTaskCount:7` acima (MSVC).
        b.push((self.limite_expandido as u8) | (self.de_titulo << 1));
        for en in &self.e[..self.quantidade as usize] {
            b.extend_from_slice(&en.id.to_le_bytes());
            b.extend_from_slice(&[en.pai, en.anterior, en.proximo, en.filho, en.estado]);
            b.extend_from_slice(&en.tempo.to_le_bytes());
            b.extend_from_slice(&en.capitao.to_le_bytes());
            let templ = if en.valida { en.id as u32 } else { 0 };
            b.extend_from_slice(&templ.to_le_bytes());
            let cap = if en.capitao != 0 { en.capitao as u32 } else { 0 };
            b.extend_from_slice(&cap.to_le_bytes());
            b.extend_from_slice(&en.buf);
        }
        b
    }

    pub fn de_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < TAM_CABECALHO {
            return None;
        }
        let quantidade = b[0];
        if quantidade as usize > MAX_ATIVAS || b.len() < TAM_CABECALHO + TAM_ENTRADA * quantidade as usize {
            return None;
        }
        let mut l = Self {
            quantidade,
            usados: b[1],
            versao: u16::from_le_bytes([b[2], b[3]]),
            topo_visiveis: b[4],
            estado: b[5],
            topo_ocultas: b[6],
            limite_expandido: b[7] & 1 != 0,
            de_titulo: b[7] >> 1,
            e: vec![Entrada::default(); MAX_ATIVAS],
        };
        for i in 0..quantidade as usize {
            let o = TAM_CABECALHO + TAM_ENTRADA * i;
            let c = &b[o..o + TAM_ENTRADA];
            let mut buf = [0u8; 11];
            buf.copy_from_slice(&c[21..32]);
            l.e[i] = Entrada {
                id: u16::from_le_bytes([c[0], c[1]]),
                pai: c[2],
                anterior: c[3],
                proximo: c[4],
                filho: c[5],
                estado: c[6],
                tempo: u32::from_le_bytes([c[7], c[8], c[9], c[10]]),
                capitao: u16::from_le_bytes([c[11], c[12]]),
                valida: u32::from_le_bytes([c[13], c[14], c[15], c[16]]) != 0,
                buf,
            };
        }
        Some(l)
    }
}

/// `FnshedTaskEntry` — id, sucesso/falha, quantas vezes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Concluida {
    pub id: u16,
    pub falhou: bool,
    pub vezes: u8,
}

/// As listas que acompanham a ativa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListasDeMissao {
    pub ativa: ListaAtiva,
    /// `FinishedTaskList`, **ordenada por id** — `AddOneTask` insere por busca binária.
    pub concluidas: Vec<Concluida>,
    /// `TaskFinishTimeList` — `(id, hora)`, na ordem de inserção.
    pub tempos: Vec<(u16, u32)>,
    /// `TaskFinishCountList` — `(id, vezes, hora)`.
    pub contagens: Vec<(u16, u32, u32)>,
    pub deposito: Vec<u8>,
}

impl Default for ListasDeMissao {
    fn default() -> Self {
        Self {
            ativa: ListaAtiva::default(),
            concluidas: Vec::new(),
            tempos: Vec::new(),
            contagens: Vec::new(),
            deposito: vec![0; TAM_DEPOSITO],
        }
    }
}

impl ListasDeMissao {
    /// `FinishedTaskList::SearchTask`: -1 sem registro, 0 sucesso, 1 falha.
    pub fn procurar_concluida(&self, id: u32) -> i32 {
        match self.concluidas.binary_search_by_key(&(id as u16), |c| c.id) {
            Ok(i) => self.concluidas[i].falhou as i32,
            Err(_) => -1,
        }
    }

    /// `FinishedTaskList::AddOneTask` (`TaskProcess.cpp:3635-3720`): o resultado da busca
    /// binária do original é a inserção ordenada; id repetido só troca o sucesso.
    pub fn marcar_concluida(&mut self, id: u32, sucesso: bool) {
        match self.concluidas.binary_search_by_key(&(id as u16), |c| c.id) {
            Ok(i) => self.concluidas[i].falhou = !sucesso,
            Err(i) => {
                if self.concluidas.len() >= MAX_CONCLUIDAS {
                    return;
                }
                self.concluidas.insert(i, Concluida { id: id as u16, falhou: !sucesso, vezes: 0 });
            }
        }
    }

    fn somar_vez(&mut self, id: u32, sucesso: bool) {
        match self.concluidas.binary_search_by_key(&(id as u16), |c| c.id) {
            Ok(i) => self.concluidas[i].vezes = self.concluidas[i].vezes.wrapping_add(1),
            Err(_) => {
                self.marcar_concluida(id, sucesso);
                if let Ok(i) = self.concluidas.binary_search_by_key(&(id as u16), |c| c.id) {
                    self.concluidas[i].vezes = 1;
                }
            }
        }
    }

    fn vezes_concluida(&self, id: u32) -> u8 {
        self.concluidas
            .binary_search_by_key(&(id as u16), |c| c.id)
            .map(|i| self.concluidas[i].vezes)
            .unwrap_or(0)
    }

    fn hora_de(&self, id: u32) -> u32 {
        self.tempos.iter().find(|t| t.0 == id as u16).map(|t| t.1).unwrap_or(0)
    }

    fn registrar_hora(&mut self, id: u32, hora: u32) {
        if let Some(t) = self.tempos.iter_mut().find(|t| t.0 == id as u16) {
            t.1 = hora;
        } else if self.tempos.len() < MAX_TEMPOS {
            self.tempos.push((id as u16, hora));
        }
    }

    fn registrar_contagem(&mut self, id: u32, hora: u32) {
        if let Some(t) = self.contagens.iter_mut().find(|t| t.0 == id as u16) {
            t.1 += 1;
            t.2 = hora;
        } else if self.contagens.len() < MAX_CONTAGENS {
            self.contagens.push((id as u16, 1, hora));
        }
    }

    /// Os cinco blocos do `TASK_DATA`, na ordem do fio.
    pub fn blocos(&self) -> [Vec<u8>; 5] {
        let mut fin = Vec::with_capacity(4 + 4 * self.concluidas.len());
        fin.extend_from_slice(&(self.concluidas.len() as u16).to_le_bytes());
        fin.push(1); // `m_Version`
        fin.push(0);
        for c in &self.concluidas {
            fin.extend_from_slice(&c.id.to_le_bytes());
            fin.push(c.falhou as u8);
            fin.push(c.vezes);
        }
        let mut tempos = Vec::with_capacity(2 + 6 * self.tempos.len());
        tempos.extend_from_slice(&(self.tempos.len() as u16).to_le_bytes());
        for (id, h) in &self.tempos {
            tempos.extend_from_slice(&id.to_le_bytes());
            tempos.extend_from_slice(&h.to_le_bytes());
        }
        let mut cont = Vec::with_capacity(2 + 14 * self.contagens.len());
        cont.extend_from_slice(&(self.contagens.len() as u16).to_le_bytes());
        for (id, n, h) in &self.contagens {
            cont.extend_from_slice(&id.to_le_bytes());
            cont.extend_from_slice(&n.to_le_bytes());
            cont.extend_from_slice(&h.to_le_bytes());
            cont.extend_from_slice(&0u32.to_le_bytes());
        }
        let mut dep = self.deposito.clone();
        dep.resize(TAM_DEPOSITO, 0);
        [self.ativa.para_bytes(), fin, tempos, cont, dep]
    }

    /// Lê as listas gravadas e as prepara como `TaskInterface::InitActiveTaskList` do lado do
    /// servidor (`TaskProcess.cpp:2100-2313`): lista inválida zera tudo, entrada sem modelo
    /// sai, contadores de topo e espaço usado são recalculados.
    pub fn de_blocos(blocos: [&[u8]; 5], tarefas: &TasksData) -> Self {
        let mut l = Self::default();
        let Some(ativa) = ListaAtiva::de_bytes(blocos[0]) else { return l };
        if ativa.versao != VERSAO_DA_LISTA {
            return l;
        }
        l.ativa = ativa;
        let f = blocos[1];
        if f.len() >= 4 {
            let n = u16::from_le_bytes([f[0], f[1]]) as usize;
            for i in 0..n.min(MAX_CONCLUIDAS) {
                let o = 4 + 4 * i;
                let Some(c) = f.get(o..o + 4) else { break };
                l.concluidas.push(Concluida { id: u16::from_le_bytes([c[0], c[1]]), falhou: c[2] & 1 != 0, vezes: c[3] });
            }
        }
        let t = blocos[2];
        if t.len() >= 2 {
            let n = u16::from_le_bytes([t[0], t[1]]) as usize;
            for i in 0..n.min(MAX_TEMPOS) {
                let o = 2 + 6 * i;
                let Some(c) = t.get(o..o + 6) else { break };
                l.tempos.push((u16::from_le_bytes([c[0], c[1]]), u32::from_le_bytes([c[2], c[3], c[4], c[5]])));
            }
        }
        let c = blocos[3];
        if c.len() >= 2 {
            let n = u16::from_le_bytes([c[0], c[1]]) as usize;
            for i in 0..n.min(MAX_CONTAGENS) {
                let o = 2 + 14 * i;
                let Some(e) = c.get(o..o + 14) else { break };
                l.contagens.push((
                    u16::from_le_bytes([e[0], e[1]]),
                    u32::from_le_bytes([e[2], e[3], e[4], e[5]]),
                    u32::from_le_bytes([e[6], e[7], e[8], e[9]]),
                ));
            }
        }
        if blocos[4].len() == TAM_DEPOSITO {
            l.deposito = blocos[4].to_vec();
        }

        let a = &mut l.ativa;
        a.estado |= LISTA_COM_TEMPO_ABSOLUTO;
        a.topo_visiveis = 0;
        a.topo_ocultas = 0;
        a.de_titulo = 0;
        let mut i = 0usize;
        while i < a.quantidade as usize {
            let en = a.e[i];
            let modelo = tarefas.get_task(en.id as u32).filter(|t| {
                if en.pai == SEM {
                    t.parent.is_none()
                } else {
                    a.e[en.pai as usize].valida && t.parent == Some(a.e[en.pai as usize].id as u32)
                }
            });
            match modelo {
                Some(t) => {
                    a.e[i].valida = true;
                    if en.pai == SEM {
                        if t.oculta {
                            a.topo_ocultas += 1;
                        } else if t.na_janela_de_titulo {
                            a.de_titulo += 1;
                        } else {
                            a.topo_visiveis += 1;
                        }
                    }
                    i += 1;
                }
                None => {
                    // `ClearTask(this, &entry, false)` e segue sem avançar.
                    limpar_sem_itens(a, tarefas, i);
                }
            }
        }
        a.usados = 0;
        for k in 0..a.quantidade as usize {
            if let Some(t) = tarefas.get_task(a.e[k].id as u32) {
                if t.parent.is_none() {
                    a.usados = a.usados.wrapping_add(t.profundidade);
                }
            }
        }
        l
    }
}

/// `ClearTask(pTask, pEntry, false)` sem jogador: só a estrutura da lista.
fn limpar_sem_itens(a: &mut ListaAtiva, tarefas: &TasksData, idx: usize) {
    limpar_recursivo(a, tarefas, idx, &mut |_, _| {});
    a.realinhar(idx, 0);
}

/// `ActiveTaskList::RecursiveClearTask` (`TaskProcess.cpp:3868-3943`). `ao_limpar` recebe cada
/// modelo removido, para quem precisa tirar itens.
fn limpar_recursivo(a: &mut ListaAtiva, tarefas: &TasksData, idx: usize, ao_limpar: &mut dyn FnMut(&TaskTemplate, &Entrada)) {
    while a.e[idx].filho != SEM {
        let f = a.e[idx].filho as usize;
        limpar_recursivo(a, tarefas, f, ao_limpar);
    }
    let en = a.e[idx];
    let modelo = if en.valida { tarefas.get_task(en.id as u32) } else { None };
    if let Some(t) = modelo {
        ao_limpar(t, &en);
    }
    a.e[idx].valida = false;
    a.e[idx].id = 0;
    a.quantidade = a.quantidade.saturating_sub(1);
    if en.pai != SEM {
        if en.anterior != SEM {
            a.e[en.anterior as usize].proximo = en.proximo;
        } else {
            a.e[en.pai as usize].filho = en.proximo;
        }
        if en.proximo != SEM {
            a.e[en.proximo as usize].anterior = en.anterior;
        }
    } else if let Some(t) = modelo {
        if t.oculta {
            a.topo_ocultas = a.topo_ocultas.saturating_sub(1);
        } else if t.na_janela_de_titulo {
            a.de_titulo = a.de_titulo.saturating_sub(1);
        } else {
            a.topo_visiveis = a.topo_visiveis.saturating_sub(1);
        }
        a.usados = a.usados.checked_sub(t.profundidade).unwrap_or(0);
    }
}

// =============================================================================== jogador

/// `task_team_member_info` (`TaskInterface.h:167-177`). O capitão é o índice 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MembroDaEquipe {
    pub id: u32,
    pub nivel: u32,
    pub classe: u32,
    pub masculino: bool,
    /// Mundo em que o membro está; 0 quando não está neste servidor de mundo.
    pub mundo: u32,
    pub pos: [f32; 3],
}

/// `_race_occ_map` (`TaskTempl.cpp:377-391`): a raça de cada classe.
const RACA_DA_CLASSE: [u32; 12] = [1, 1, 3, 2, 2, 3, 4, 4, 5, 5, 6, 6];
/// `INVALID_VAL`.
const QUALQUER: u32 = 0xFFFF_FFFF;

/// O que o motor precisa saber do jogador e pedir a ele — o `TaskInterface` do original
/// (`task/TaskInterface.h`, implementado por `PlayerTaskInterface` em `taskman.cpp`).
pub trait Jogador {
    fn agora(&self) -> u32;
    fn nivel(&self) -> u32;
    fn classe(&self) -> u32;
    fn masculino(&self) -> bool;
    fn cultivo(&self) -> u32;
    fn reputacao(&self) -> i32;
    fn dinheiro(&self) -> u32;
    fn e_gm(&self) -> bool;
    /// `GetCommonItemCount` (soma da bolsa) / `GetTaskItemCount` (bolsa de missão).
    fn contar(&self, tid: u32, comum: bool) -> u32;
    /// `GetEmptySlotCount` da bolsa ou da bolsa de missão.
    fn slots_livres(&self, comum: bool) -> u32;
    fn dar_item(&mut self, tid: u32, quantidade: u32, comum: bool, validade: i32);
    fn tirar_item(&mut self, tid: u32, quantidade: u32, comum: bool);
    fn dar_dinheiro(&mut self, n: u32);
    fn tirar_dinheiro(&mut self, n: u32);
    /// `ReceiveTaskExp` — um de cada vez, como `DeliverExperience`/`DeliverSP`.
    fn dar_exp(&mut self, exp: u32, sp: u32);
    fn dar_reputacao(&mut self, r: i32);
    /// `SetCurPeriod` (`task/taskman.cpp:251-254`) → `gplayer_imp::SetSecLevel`: o **nível de
    /// cultivo** novo, que o prêmio `m_ulNewPeriod` da missão define
    /// (`Task/TaskProcess.cpp:1284`). Quem implementa avisa o cliente com
    /// `TASK_DELIVER_LEVEL2` (160).
    fn definir_cultivo(&mut self, nivel: u32);
    /// `SetFuryUpperLimit` → `gplayer_imp::SetMaxAP` (`gs/task/taskman.cpp:498-501`): o teto
    /// da barra de chi que o prêmio `m_ulFuryULimit` concede.
    fn definir_teto_de_chi(&mut self, teto: u32);
    /// Um comando pronto para o cliente (o `TASK_VAR_DATA` com o aviso).
    fn avisar(&mut self, comando: Vec<u8>);
    /// `UnitRand` — `[0, 1)`.
    fn sortear(&mut self) -> f32;
    /// `GetPos`: o mundo e a posição.
    fn posicao(&self) -> (u32, [f32; 3]) {
        (0, [0.0; 3])
    }
    /// `IsInFaction`/`GetFactionRole`: o id da facção (0 = nenhuma) e o cargo.
    fn faccao(&self) -> (u32, i32) {
        (0, 0)
    }
    /// A equipe, com o capitão no índice 0; vazio quando não está em equipe.
    fn equipe(&self) -> Vec<MembroDaEquipe> {
        Vec::new()
    }
    /// `TransportTo` (`taskman.cpp:504-507` → `LongJump`): leva o jogador a `pos` do mapa
    /// `mundo`, depois que a operação terminar.
    fn teleportar(&mut self, _mundo: u32, _pos: [f32; 3]) {}
    /// `SummonMonster` (`TaskProcess.cpp:1189`): evoca monstro na cena perto do jogador.
    fn invocar_monstro(&mut self, _monstro_tid: u32, _quantidade: u32, _raio: u32, _periodo_s: i32, _some_ao_morrer: bool) {}
}

/// Jogador vazio, só para limpar estrutura.
pub struct SemJogador;
impl Jogador for SemJogador {
    fn agora(&self) -> u32 { 0 }
    fn nivel(&self) -> u32 { 0 }
    fn classe(&self) -> u32 { 0 }
    fn masculino(&self) -> bool { true }
    fn cultivo(&self) -> u32 { 0 }
    fn reputacao(&self) -> i32 { 0 }
    fn dinheiro(&self) -> u32 { 0 }
    fn e_gm(&self) -> bool { false }
    fn contar(&self, _: u32, _: bool) -> u32 { 0 }
    fn slots_livres(&self, _: bool) -> u32 { 0 }
    fn dar_item(&mut self, _: u32, _: u32, _: bool, _: i32) {}
    fn tirar_item(&mut self, _: u32, _: u32, _: bool) {}
    fn dar_dinheiro(&mut self, _: u32) {}
    fn tirar_dinheiro(&mut self, _: u32) {}
    fn dar_exp(&mut self, _: u32, _: u32) {}
    fn dar_reputacao(&mut self, _: i32) {}
    fn definir_cultivo(&mut self, _: u32) {}
    fn definir_teto_de_chi(&mut self, _: u32) {}
    fn avisar(&mut self, _: Vec<u8>) {}
    fn sortear(&mut self) -> f32 { 0.0 }
}

// =============================================================================== motor

/// `task_sub_tags` (`TaskTempl.h:1737-1755`).
#[derive(Debug, Clone, Default)]
struct Etiquetas {
    /// `union { unsigned short sub_task; unsigned char state; }`.
    uniao: u16,
    tags: Vec<u8>,
}

impl Etiquetas {
    fn bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(3 + self.tags.len());
        b.extend_from_slice(&self.uniao.to_le_bytes());
        b.push(self.tags.len() as u8);
        b.extend_from_slice(&self.tags);
        b
    }
}

/// O motor, sobre as listas de um jogador.
pub struct Motor<'a, J: Jogador> {
    pub tarefas: &'a TasksData,
    pub listas: &'a mut ListasDeMissao,
    pub j: &'a mut J,
    /// O `roleid` do jogador — para saber se ele é o capitão da equipe.
    pub eu: u32,
}

/// `TEAM_MEM_WANTED::IsMeetBaseInfo` (`TaskTempl.h:386-417`). Força (`m_iForce`) não existe
/// no servidor: membro de força pedida não serve.
fn membro_serve(w: &pw_data_loader::tasks::MembroPedido, m: &MembroDaEquipe) -> bool {
    if w.nivel_minimo != 0 && m.nivel < w.nivel_minimo || w.nivel_maximo != 0 && m.nivel > w.nivel_maximo {
        return false;
    }
    if w.raca != 0 {
        if RACA_DA_CLASSE.get(m.classe as usize) != Some(&w.raca) {
            return false;
        }
    } else if w.classe != QUALQUER && m.classe != w.classe {
        return false;
    }
    if w.forca != 0 {
        return false;
    }
    !(w.genero == 1 && !m.masculino || w.genero == 2 && m.masculino)
}

/// `judge_time_date` (`TaskTempl.h:1697-1733`) na hora local do servidor (`localtime`).
fn janela_vale(j: &pw_data_loader::tasks::JanelaDeHorario, agora: u32) -> bool {
    use chrono::{Datelike, TimeZone, Timelike};
    let local = |t: u32| chrono::Local.timestamp_opt(t as i64, 0).single();
    let (Some(h), Some(amanha)) = (local(agora), local(agora.saturating_add(24 * 3600))) else {
        return false;
    };
    let ultimo_dia = h.month() != amanha.month();
    let (ano, mes, dia, hora, min) = (h.year(), h.month() as i32, h.day() as i32, h.hour() as i32, h.minute() as i32);
    // `task_week_map`: domingo é 7.
    let semana = match h.weekday().num_days_from_sunday() { 0 => 7, d => d as i32 };
    let (s, e) = (&j.inicio, &j.fim);
    // `before`/`after` comparam de ano até minuto; as variantes por mês, semana e dia
    // cortam o começo da comparação.
    let hm_antes = |t: &pw_data_loader::tasks::MomentoDeMissao| t.hora < hora || t.hora == hora && t.minuto <= min;
    let hm_depois = |t: &pw_data_loader::tasks::MomentoDeMissao| t.hora > hora || t.hora == hora && t.minuto > min;
    match j.tipo {
        0 => {
            let chave = |t: &pw_data_loader::tasks::MomentoDeMissao| (t.ano, t.mes, t.dia);
            let agora_ = (ano, mes, dia);
            let ini = chave(s) < agora_ || chave(s) == agora_ && hm_antes(s);
            let fim = chave(e) > agora_ || chave(e) == agora_ && hm_depois(e);
            ini && fim
        }
        1 => {
            // `before_per_month`/`after_per_month`.
            let ini = if s.dia < dia { true } else if !ultimo_dia && s.dia > dia { false } else { hm_antes(s) };
            let fim = if e.dia < dia { false } else if !ultimo_dia && e.dia > dia { true } else { hm_depois(e) };
            ini && fim
        }
        2 => {
            let ini = s.dia_da_semana < semana || s.dia_da_semana == semana && hm_antes(s);
            let fim = e.dia_da_semana > semana || e.dia_da_semana == semana && hm_depois(e);
            ini && fim
        }
        3 => hm_antes(s) && hm_depois(e),
        _ => false,
    }
}

fn pai_de<'t>(tarefas: &'t TasksData, t: &TaskTemplate) -> Option<&'t TaskTemplate> {
    t.parent.and_then(|p| tarefas.get_task(p))
}

fn topo_de<'t>(tarefas: &'t TasksData, t: &'t TaskTemplate) -> &'t TaskTemplate {
    let mut atual = t;
    while let Some(p) = pai_de(tarefas, atual) {
        atual = p;
    }
    atual
}

fn proximo_irmao<'t>(tarefas: &'t TasksData, t: &TaskTemplate) -> Option<&'t TaskTemplate> {
    let pai = pai_de(tarefas, t)?;
    let pos = pai.sub_tasks.iter().position(|&s| s == t.id)?;
    pai.sub_tasks.get(pos + 1).and_then(|&s| tarefas.get_task(s))
}

fn itens_do_premio(g: &pw_data_loader::tasks::GrupoDeItens) -> (u32, u32) {
    g.contagens()
}

impl<'a, J: Jogador> Motor<'a, J> {
    fn t(&self, id: u32) -> Option<&'a TaskTemplate> {
        self.tarefas.get_task(id)
    }

    fn avisar_erro(&mut self, id: u32, codigo: u32) {
        self.j.avisar(S2CGamedataSend::task_notify_error(id as u16, codigo).data);
    }

    fn contar(&self, i: &ItemDeMissao) -> u32 {
        self.j.contar(i.id, i.comum)
    }

    // ------------------------------------------------------------------ pré-requisitos

    /// `ATaskTempl::CheckBudget` (`TaskTempl.inl:31-52`).
    fn verificar_espaco(&self, t: &TaskTemplate) -> u32 {
        let a = &self.listas.ativa;
        let cheia = if t.oculta {
            a.topo_ocultas >= MAX_OCULTAS
        } else if t.na_janela_de_titulo {
            a.de_titulo >= MAX_DE_TITULO
        } else {
            a.topo_visiveis >= a.max_simultaneas()
        };
        if cheia {
            return erro::CHEIA;
        }
        if a.usados as usize + t.profundidade as usize > MAX_ATIVAS {
            return erro::SEM_ESPACO;
        }
        if a.indice(t.id).is_some() {
            return erro::MESMA_MISSAO;
        }
        0
    }

    /// `CheckRecordListSpace` (`TaskProcess.cpp:818-826`).
    fn verificar_registros(&self, t: &TaskTemplate) -> u32 {
        let precisa_concluidas = t.precisa_registro || (!t.limite_de_conta && t.limite_de_personagem);
        let precisa_tempos = (!t.limite_de_conta && t.limite_de_personagem)
            || (t.frequencia != 0 && !t.limite_de_conta && !t.limite_de_personagem);
        if precisa_concluidas && self.listas.concluidas.len() >= MAX_CONCLUIDAS
            || precisa_tempos && self.listas.tempos.len() >= MAX_TEMPOS
            || t.limite_de_conta && self.listas.contagens.len() >= MAX_CONTAGENS
        {
            return erro::CHEIA;
        }
        0
    }

    /// `CheckDeliverTime` (`TaskTempl.inl:472-611`), com a hora do servidor em UTC.
    fn verificar_frequencia(&mut self, t: &TaskTemplate, agora: u32) -> u32 {
        if t.frequencia == 0 {
            return 0;
        }
        if self.listas.tempos.len() >= MAX_TEMPOS {
            return erro::CHEIA;
        }
        let hora = if t.limite_de_conta {
            match self.listas.contagens.iter().find(|c| c.0 == t.id as u16) {
                Some(c) if c.1 != 0 => c.2,
                _ => return 0,
            }
        } else {
            let h = self.listas.hora_de(t.id);
            if h == 0 {
                return 0;
            }
            h
        };
        use chrono::{Datelike, TimeZone, Utc};
        let (Some(cur), Some(tsk)) = (Utc.timestamp_opt(agora as i64, 0).single(), Utc.timestamp_opt(hora as i64, 0).single()) else {
            return erro::HORARIO;
        };
        let mesmo = match t.frequencia {
            1 => cur.year() == tsk.year() && cur.ordinal() == tsk.ordinal(),
            2 => {
                // `_is_same_week` (`TaskTempl.inl:421-437`) com `task_week_map` (domingo = 7).
                let d = (agora as i64 - hora as i64).abs();
                if d >= 7 * 24 * 3600 {
                    false
                } else {
                    let w1 = cur.weekday().number_from_monday();
                    let w2 = tsk.weekday().number_from_monday();
                    if w1 == w2 {
                        d <= 24 * 3600
                    } else if w1 > w2 {
                        agora > hora
                    } else {
                        agora < hora
                    }
                }
            }
            3 => cur.year() == tsk.year() && cur.month() == tsk.month(),
            4 => cur.year() == tsk.year(),
            _ => return erro::HORARIO,
        };
        if mesmo {
            if !t.limite_de_conta && !t.limite_de_personagem {
                return erro::HORARIO;
            } else if self.verificar_contagem(t) != 0 {
                return erro::HORARIO;
            }
        } else if t.limite_de_conta {
            if let Some(c) = self.listas.contagens.iter_mut().find(|c| c.0 == t.id as u16) {
                c.1 = 0;
            }
        } else if t.limite_de_personagem {
            if let Ok(i) = self.listas.concluidas.binary_search_by_key(&(t.id as u16), |c| c.id) {
                self.listas.concluidas[i].vezes = 0;
            }
        }
        0
    }

    /// `CheckDeliverCount` (`TaskTempl.inl:999-1021`).
    fn verificar_contagem(&self, t: &TaskTemplate) -> u32 {
        if t.limite_de_conta && t.limite_por_periodo != 0 {
            let n = self.listas.contagens.iter().find(|c| c.0 == t.id as u16).map(|c| c.1).unwrap_or(0);
            if n >= t.limite_por_periodo as u32 {
                return erro::CONTA_NO_LIMITE;
            }
        } else if t.limite_de_personagem && t.limite_por_periodo != 0 && self.listas.vezes_concluida(t.id) as i32 >= t.limite_por_periodo {
            return erro::PERSONAGEM_NO_LIMITE;
        }
        0
    }

    /// `ATaskTempl::CheckPrerequisite` (`TaskProcess.cpp:179-446`), na mesma ordem.
    fn verificar_pre_requisitos(&mut self, t: &TaskTemplate, agora: u32, anterior: bool, equipe: bool, espaco: bool) -> u32 {
        if t.parent.is_some() {
            return erro::NAO_E_RAIZ;
        }
        if espaco {
            let r = self.verificar_espaco(t);
            if r != 0 {
                return r;
            }
        } else if self.listas.ativa.indice(t.id).is_some() {
            return erro::MESMA_MISSAO;
        }
        let r = self.verificar_registros(t);
        if r != 0 {
            return r;
        }
        if t.pq {
            let tem = (0..self.listas.ativa.quantidade as usize)
                .filter_map(|i| self.t(self.listas.ativa.e[i].id as u32))
                .any(|o| o.pq);
            if tem {
                return erro::JA_TEM_PQ;
            }
        }
        if !t.itens_entregues.is_empty()
            && (t.entregues_comuns != 0 && self.j.slots_livres(true) < t.entregues_comuns
                || t.entregues_de_missao != 0 && self.j.slots_livres(false) < t.entregues_de_missao)
        {
            return erro::ITEM_ENTREGUE;
        }
        if !t.janelas.is_empty() && !t.janelas.iter().any(|j| janela_vale(j, agora)) {
            return erro::HORARIO;
        }
        let r = self.verificar_frequencia(t, agora);
        if r != 0 {
            return r;
        }
        if (!t.pode_repetir || !t.refazer_apos_falha) && {
            let n = self.listas.procurar_concluida(t.id);
            n == 0 && !t.pode_repetir || n == 1 && !t.refazer_apos_falha
        } {
            return erro::NAO_REPETE;
        }
        let r = self.verificar_contagem(t);
        if r != 0 {
            return r;
        }
        let nivel = self.j.nivel();
        if t.min_level != 0 && nivel < t.min_level {
            return erro::NIVEL_BAIXO;
        }
        if t.max_level != 0 && nivel > t.max_level {
            return erro::NIVEL_ALTO;
        }
        if t.reputacao_minima != 0 && self.j.reputacao() < t.reputacao_minima {
            return erro::REPUTACAO;
        }
        if t.deposito != 0 && self.j.dinheiro() < t.deposito {
            return erro::DEPOSITO;
        }
        // `CheckItems` (`TaskTempl.inl:688-716`).
        let mut r = if t.itens_exigidos_qualquer_um { erro::SEM_ITEM } else { 0 };
        for i in &t.itens_exigidos {
            let n = self.contar(i);
            if t.itens_exigidos_qualquer_um {
                if n >= i.quantidade {
                    r = 0;
                    break;
                }
            } else if n < i.quantidade {
                r = erro::SEM_ITEM;
                break;
            }
        }
        if r != 0 {
            return r;
        }
        // `CheckFaction` (`TaskTempl.inl:718-727`): `IsInFaction` é `id_mafia != 0`
        // (`taskman.cpp:159-162`).
        let (faccao, cargo) = self.j.faccao();
        if t.faccao != 0 && !(faccao != 0 && cargo <= t.papel_na_faccao) {
            return erro::FACCAO;
        }
        let masculino = self.j.masculino();
        if t.genero == 1 && !masculino || t.genero == 2 && masculino {
            return erro::GENERO;
        }
        if !t.req_classes.is_empty() && !t.req_classes.contains(&self.j.classe()) {
            return erro::CLASSE;
        }
        // `CheckPeriod` (`TaskTempl.inl:754-774`).
        let cur = self.j.cultivo();
        if cur < t.periodo {
            return erro::PERIODO;
        }
        if t.periodo >= 20 && (t.periodo < 30 && cur >= 30 || t.periodo >= 30 && t.periodo < 40 && cur >= 40 || t.periodo >= 40) {
            return erro::PERIODO;
        }
        if t.so_gm && !self.j.e_gm() {
            return erro::GM;
        }
        if anterior {
            // `CheckPreTask` (`TaskTempl.inl:810-830`).
            let mut feitas = 0u32;
            for &p in &t.pre_tasks {
                let s = self.listas.procurar_concluida(p);
                if t.pre_missoes_minimo == 0 {
                    if s != 0 {
                        return erro::MISSAO_ANTERIOR;
                    }
                } else if s == 0 {
                    feitas += 1;
                }
            }
            if t.pre_missoes_minimo != 0 && feitas < t.pre_missoes_minimo {
                return erro::MISSAO_ANTERIOR;
            }
        }
        // `CheckMutexTask` (`TaskProcess.cpp:1134-1160`).
        for &m in &t.missoes_exclusivas {
            if self.listas.ativa.indice(m).is_some() {
                return erro::EXCLUSIVA;
            }
            let Some(mt) = self.t(m).filter(|x| x.parent.is_none()) else {
                return erro::EXCLUSIVA;
            };
            if self.verificar_frequencia(mt, agora) != 0 {
                return erro::EXCLUSIVA;
            }
            let n = self.listas.procurar_concluida(mt.id);
            if (!mt.pode_repetir || !mt.refazer_apos_falha) && (n == 0 && !mt.pode_repetir || n == 1 && !mt.refazer_apos_falha) {
                return erro::EXCLUSIVA;
            }
        }
        // `CheckInZone` (`TaskTempl.inl:368-393`).
        if t.entrega_em_zona {
            let (mundo, pos) = self.j.posicao();
            if mundo != t.mundo_de_entrega || !t.regioes_de_entrega.iter().any(|r| r.contem(pos)) {
                return erro::FORA_DA_ZONA;
            }
        }
        // `CheckTeamTask` (`TaskTempl.inl:330-339`), só para quem recebe por conta própria.
        if equipe && t.em_equipe && t.recebida_pela_equipe {
            let membros = self.j.equipe();
            if membros.first().map(|m| m.id) != Some(self.eu) {
                return erro::NAO_E_CAPITAO;
            }
            let r = self.membros_pedidos_ok(t, &membros, true);
            if r != 0 {
                return r;
            }
        }
        if t.conjuge {
            return erro::INDETERMINADO;
        }
        if t.compara_bolsa && self.j.slots_livres(true) < t.slots_de_bolsa {
            return erro::SLOTS_DE_BOLSA;
        }
        0
    }

    /// `HasAllTeamMemsWanted` (`TaskTempl.inl:149-252`), do lado do servidor.
    fn membros_pedidos_ok(&self, t: &TaskTemplate, membros: &[MembroDaEquipe], estrito: bool) -> u32 {
        let (mundo, pos) = self.j.posicao();
        let perto = |m: &MembroDaEquipe| {
            let d = (0..3).map(|i| (pos[i] - m.pos[i]).powi(2)).sum::<f32>();
            m.mundo == mundo && d <= t.distancia_dos_membros
        };
        if t.membros_pedidos.is_empty() {
            if t.confere_membros && membros.iter().skip(1).any(|m| !perto(m)) {
                return erro::LONGE_DA_EQUIPE;
            }
            return 0;
        }
        let mut contagem = vec![0u32; t.membros_pedidos.len()];
        let mut classes = std::collections::BTreeSet::new();
        if let Some(c) = membros.first() {
            classes.insert(c.classe);
        }
        for m in membros.iter().skip(1) {
            classes.insert(m.classe);
            if t.confere_membros && !perto(m) {
                return erro::LONGE_DA_EQUIPE;
            }
            match t.membros_pedidos.iter().position(|w| membro_serve(w, m)) {
                Some(j) => contagem[j] += 1,
                None if estrito => return erro::MEMBRO_INVALIDO,
                None => {}
            }
        }
        if t.classes_distintas && classes.len() != membros.len() {
            return erro::MEMBRO_INVALIDO;
        }
        if t.so_casal {
            // Sem casamento no servidor: ninguém é cônjuge de ninguém.
            return erro::NAO_E_CASAL;
        }
        for (w, &n) in t.membros_pedidos.iter().zip(&contagem) {
            if w.minimo != 0 && n < w.minimo || w.maximo != 0 && n > w.maximo {
                return erro::MEMBRO_INVALIDO;
            }
        }
        0
    }

    /// `OnDeliverTeamMemTask` (`TaskProcess.cpp:1592-1640`): o membro recebe a missão que o
    /// capitão aceitou (ou a que o `TEAM_MEM_WANTED` dele manda), sem conferir equipe.
    pub fn aceitar_como_membro(&mut self, id_do_capitao: u32) -> u32 {
        let Some(t) = self.t(id_do_capitao) else { return erro::INDETERMINADO };
        let membros = self.j.equipe();
        if !t.em_equipe || membros.first().map(|m| m.id) == Some(self.eu) {
            return erro::INDETERMINADO;
        }
        if !t.membros_pedidos.is_empty() {
            let eu = MembroDaEquipe {
                id: self.eu,
                nivel: self.j.nivel(),
                classe: self.j.classe(),
                masculino: self.j.masculino(),
                mundo: 0,
                pos: [0.0; 3],
            };
            let Some(w) = t.membros_pedidos.iter().find(|w| membro_serve(w, &eu)) else {
                return erro::MEMBRO_INVALIDO;
            };
            if w.missao != 0 {
                let topo = self.t(w.missao).map(|x| topo_de(self.tarefas, x).id).unwrap_or(0);
                return self.aceitar_completo(topo, 0, true, true, id_do_capitao);
            }
        }
        self.aceitar_completo(id_do_capitao, 0, true, true, 0)
    }

    /// `OnTaskReachSite` e `OnTaskLeaveSite` (`TaskServer.cpp:466-520`): o cliente avisa, o
    /// servidor confere o lugar e finaliza.
    pub fn conferir_lugar(&mut self, id: u32, saida: bool) -> bool {
        let Some(idx) = (0..self.listas.ativa.quantidade as usize).find(|&i| self.listas.ativa.e[i].valida && self.listas.ativa.e[i].id as u32 == id) else {
            return false;
        };
        if self.listas.ativa.e[idx].finalizada() {
            return false;
        }
        let Some(t) = self.t(id) else { return false };
        let (mundo, pos) = self.j.posicao();
        let chegou = if saida {
            if t.metodo != metodo::SAIR_DE_LUGAR || t.tipo_de_conclusao != conclusao::DIRETA {
                return false;
            }
            !(mundo == t.mundo_a_sair && t.lugares_a_sair.iter().any(|r| r.contem(pos)))
        } else {
            if t.metodo != metodo::ALCANCAR_LUGAR || t.tipo_de_conclusao != conclusao::DIRETA {
                return false;
            }
            mundo == t.mundo_a_alcancar && t.lugares_a_alcancar.iter().any(|r| r.contem(pos))
        };
        if chegou {
            self.ao_finalizar(t, idx);
        }
        chegou
    }

    // ------------------------------------------------------------------ entrega

    /// `ATaskTempl::DeliverTask` (`TaskProcess.cpp:448-670`). Devolve o índice seguinte.
    fn entregar(&mut self, t: &'a TaskTemplate, entrada: Option<usize>, capitao: u32, agora: u32, sub: Option<&'a TaskTemplate>, tags: &mut Etiquetas, pai: u8) -> usize {
        let idx = match entrada {
            None => self.listas.ativa.quantidade as usize,
            Some(i) => {
                if self.listas.ativa.e[i].id != 0 {
                    self.listas.ativa.realinhar(i, 1);
                }
                i
            }
        };
        if idx >= MAX_ATIVAS {
            return idx;
        }
        {
            let en = &mut self.listas.ativa.e[idx];
            en.id = t.id as u16;
            en.valida = true;
            en.pai = pai;
            en.anterior = SEM;
            en.proximo = SEM;
            en.filho = SEM;
            en.estado = estado::SUCESSO;
            en.tempo = agora;
            en.capitao = if capitao != 0 && self.tarefas.get_task(capitao).is_some_and(|c| c.parent.is_none()) { capitao as u16 } else { 0 };
            en.buf = [0; 11];
        }
        let a = &mut self.listas.ativa;
        a.quantidade = a.quantidade.wrapping_add(1);
        if t.parent.is_none() {
            if t.oculta {
                a.topo_ocultas = a.topo_ocultas.wrapping_add(1);
            } else if t.na_janela_de_titulo {
                a.de_titulo = a.de_titulo.wrapping_add(1);
            } else {
                a.topo_visiveis = a.topo_visiveis.wrapping_add(1);
            }
            a.usados = a.usados.wrapping_add(t.profundidade);
        }
        if pai != SEM {
            let p = pai as usize;
            if a.e[p].filho == SEM {
                a.e[p].filho = idx as u8;
            } else {
                let mut f = a.e[p].filho as usize;
                while a.e[f].proximo != SEM {
                    f = a.e[f].proximo as usize;
                }
                a.e[f].proximo = idx as u8;
                a.e[idx].anterior = f as u8;
            }
        }
        if t.parent.is_none() {
            self.entregar_itens_dados(t);
        }
        let seguinte = idx + 1;

        if let Some(s) = sub {
            return self.entregar(s, Some(seguinte), 0, agora, None, tags, idx as u8);
        }
        if t.sorteia_um_filho {
            // `RandOneChild` (`TaskTempl.inl:1695-1703`).
            let n = t.sub_tasks.len();
            if n > 0 {
                let mut sel = (self.j.sortear() * n as f32) as usize;
                if sel >= n {
                    sel = n - 1;
                }
                if let Some(f) = self.t(t.sub_tasks[sel]) {
                    if tags.tags.len() < MAX_SUB_TAGS {
                        tags.tags.push(sel as u8);
                    }
                    return self.entregar(f, Some(seguinte), 0, agora, None, tags, idx as u8);
                }
            }
            return seguinte;
        }
        let mut proxima = seguinte;
        for &f in &t.sub_tasks {
            let Some(ft) = self.t(f) else { continue };
            proxima = self.entregar(ft, Some(proxima), 0, agora, None, tags, idx as u8);
            if t.filhos_em_ordem {
                return proxima;
            }
        }
        proxima
    }

    /// `DeliverGivenItems` (`TaskTempl.inl:1705-1717`).
    fn entregar_itens_dados(&mut self, t: &TaskTemplate) {
        if t.itens_entregues.is_empty() {
            return;
        }
        if t.entregues_comuns != 0 && self.j.slots_livres(true) < t.entregues_comuns
            || t.entregues_de_missao != 0 && self.j.slots_livres(false) < t.entregues_de_missao
        {
            return;
        }
        for i in &t.itens_entregues {
            self.j.dar_item(i.id, i.quantidade, i.comum, 0);
        }
    }

    /// `RemovePrerequisiteItem` (`TaskTempl.inl:1741-1777`).
    fn tirar_itens_exigidos(&mut self, t: &TaskTemplate) {
        if t.deposito != 0 {
            self.j.tirar_dinheiro(t.deposito);
        }
        for i in &t.itens_exigidos {
            if i.id == 0 || i.quantidade == 0 {
                continue;
            }
            if t.itens_exigidos_qualquer_um {
                if self.contar(i) >= i.quantidade {
                    self.j.tirar_item(i.id, i.quantidade, i.comum);
                    break;
                }
            } else {
                self.j.tirar_item(i.id, i.quantidade, i.comum);
            }
        }
    }

    /// `ATaskTempl::CheckDeliverTask` (`TaskProcess.cpp:1735-1880`), sem depósito, PQ, equipe
    /// e teleporte. `sub_id` é a submissão escolhida, para missão `m_bChooseOne`.
    pub fn aceitar(&mut self, id: u32, sub_id: u32, avisar_erro: bool) -> u32 {
        self.aceitar_completo(id, sub_id, avisar_erro, false, 0)
    }

    /// `CheckDeliverTask(sub, global, bNotifyErr, bMemTask, ulCapId)`
    /// (`TaskProcess.cpp:1735-1841`).
    fn aceitar_completo(&mut self, id: u32, sub_id: u32, avisar_erro: bool, de_membro: bool, capitao: u32) -> u32 {
        let Some(t) = self.t(id) else { return erro::INDETERMINADO };
        let mut sub = None;
        if t.escolhe_um_filho {
            match self.t(sub_id).filter(|s| s.parent == Some(t.id)) {
                Some(s) => sub = Some(s),
                None => return erro::SUBMISSAO_ERRADA,
            }
        }
        let agora = self.j.agora();
        let r = self.verificar_pre_requisitos(t, agora, true, !de_membro, true);
        if r != 0 {
            if avisar_erro && !t.entrega_automatica {
                self.avisar_erro(t.id, r);
            }
            return r;
        }
        if !t.item_nao_retirado {
            self.tirar_itens_exigidos(t);
        }
        let mut tags = Etiquetas { uniao: sub.map(|s| s.id as u16).unwrap_or(0), tags: Vec::new() };
        self.entregar(t, None, capitao, agora, sub, &mut tags, SEM);
        if t.frequencia != 0 && !t.limite_de_conta && !t.limite_de_personagem {
            self.listas.registrar_hora(t.id, agora);
        }
        self.j.avisar(S2CGamedataSend::task_notify_new(t.id as u16, agora, capitao, &tags.bytes()).data);
        // `if (m_bTransTo) pTask->TransportTo(...)` (`TaskProcess.cpp:1843-1844`).
        if let Some((mundo, pos)) = t.teleporte_ao_receber {
            self.j.teleportar(mundo, pos);
        }
        0
    }

    // ------------------------------------------------------------------ prêmio

    /// `CalcAwardData` (`TaskTempl.inl:1331-1357`): prêmio de sucesso ou de falha. As escalas
    /// por tempo e por itens não são lidas pelo carregador — ficam sem prêmio.
    fn premio_de(&self, t: &'a TaskTemplate, en: &Entrada) -> Option<&'a TaskReward> {
        let tipo = if en.sucesso() { t.tipo_de_premio_sucesso } else { t.tipo_de_premio_falha };
        match tipo {
            0 | 1 => Some(if en.sucesso() { &t.rewards } else { &t.premio_de_falha }),
            _ => None,
        }
    }

    /// `RecursiveCalcAward` (`TaskProcess.cpp:672-768`).
    #[allow(clippy::too_many_arguments)]
    fn calcular_premio(&mut self, t: &'a TaskTemplate, en: Entrada, escolha: i32, comuns: &mut u32, de_missao: &mut u32, topo: &mut (u32, u32, u32), espaco: &mut u8, reputacao: &mut i64) -> u32 {
        let agora = self.j.agora();
        let vazio = TaskReward::default();
        let p = self.premio_de(t, &en).unwrap_or(&vazio).clone();
        if t.min_level != 0 && !en.desistiu() && !t.nivel_maximo_historico && self.j.nivel() < t.min_level {
            return erro::PREMIO_NIVEL;
        }
        *reputacao += p.reputation as i64;
        if !p.grupos_de_itens.is_empty() {
            let e = if escolha < 0 || escolha as usize >= p.grupos_de_itens.len() { 0 } else { escolha as usize };
            let g = &p.grupos_de_itens[e];
            if g.sorteia_um {
                for i in &g.itens {
                    if i.probabilidade < 1.0 {
                        continue;
                    }
                    if i.comum {
                        *comuns += 1;
                    } else {
                        *de_missao += 1;
                    }
                }
                *comuns += 1;
                *de_missao += 1;
            } else {
                let (c, m) = itens_do_premio(g);
                *comuns += c;
                *de_missao += m;
            }
        }
        if p.nova_missao != 0 {
            if let Some(nova) = self.t(p.nova_missao).filter(|n| n.parent.is_none()) {
                let r = self.verificar_pre_requisitos(nova, agora, false, true, false);
                if r != 0 {
                    return r;
                }
                *comuns += nova.entregues_comuns;
                *de_missao += nova.entregues_de_missao;
                if nova.oculta {
                    topo.2 += 1;
                } else if nova.na_janela_de_titulo {
                    topo.1 += 1;
                } else {
                    topo.0 += 1;
                }
                *espaco = espaco.wrapping_add(nova.profundidade);
            }
        }
        if en.pai != SEM {
            let Some(pai) = pai_de(self.tarefas, t) else { return 0 };
            let mut pe = self.listas.ativa.e[en.pai as usize];
            if !en.sucesso() && t.pai_tambem_falha {
                pe.estado &= !estado::SUCESSO;
                pe.estado |= estado::FINALIZADA;
                return self.calcular_premio(pai, pe, -1, comuns, de_missao, topo, espaco, reputacao);
            } else if en.sucesso() && t.pai_tambem_sucesso {
                pe.estado |= estado::FINALIZADA;
                if pai.tipo_de_conclusao == conclusao::DIRETA || pai.tipo_de_conclusao == conclusao::CONFIRMAR {
                    return self.calcular_premio(pai, pe, -1, comuns, de_missao, topo, espaco, reputacao);
                }
            } else if pai.filhos_em_ordem && proximo_irmao(self.tarefas, t).is_some() {
                return 0;
            } else if en.anterior == SEM && en.proximo == SEM {
                pe.estado |= estado::FINALIZADA;
                if pai.tipo_de_conclusao == conclusao::DIRETA || pai.tipo_de_conclusao == conclusao::CONFIRMAR {
                    return self.calcular_premio(pai, pe, -1, comuns, de_missao, topo, espaco, reputacao);
                }
            }
        }
        0
    }

    /// `RecursiveCheckAward` (`TaskProcess.cpp:828-876`).
    fn verificar_premio(&mut self, t: &'a TaskTemplate, idx: usize, escolha: i32) -> u32 {
        let r = self.verificar_registros(t);
        if r != 0 {
            return r;
        }
        let (mut comuns, mut de_missao, mut topo, mut espaco, mut repu) = (0u32, 0u32, (0u32, 0u32, 0u32), 0u8, 0i64);
        let en = self.listas.ativa.e[idx];
        let r = self.calcular_premio(t, en, escolha, &mut comuns, &mut de_missao, &mut topo, &mut espaco, &mut repu);
        if r != 0 {
            return r;
        }
        let a = &self.listas.ativa;
        if topo.0 != 0 && a.topo_visiveis as u32 + topo.0 > a.max_simultaneas() as u32
            || topo.2 != 0 && a.topo_ocultas as u32 + topo.2 > MAX_OCULTAS as u32
            || topo.1 != 0 && a.de_titulo as u32 + topo.1 > MAX_DE_TITULO as u32
        {
            return erro::CHEIA;
        }
        if espaco != 0 && a.usados as usize + espaco as usize > MAX_ATIVAS {
            return erro::SEM_ESPACO;
        }
        if comuns != 0 && self.j.slots_livres(true) < comuns || de_missao != 0 && self.j.slots_livres(false) < de_missao {
            return erro::PREMIO_ITEM;
        }
        if repu != 0 && self.j.reputacao() as i64 + repu < 0 {
            return erro::PREMIO_REPUTACAO;
        }
        0
    }

    /// `RemoveAcquiredItem` (`TaskTempl.inl:1779-1870`).
    fn tirar_adquiridos(&mut self, t: &TaskTemplate, limpando: bool, sucesso: bool) {
        match t.metodo {
            metodo::COLETAR_ITENS => {
                for i in &t.item_collections {
                    if i.comum {
                        if limpando {
                            continue;
                        }
                        let mut n = self.contar(i);
                        if n == 0 {
                            continue;
                        }
                        if i.quantidade != 0 && n > i.quantidade {
                            n = i.quantidade;
                        }
                        self.j.tirar_item(i.id, n, true);
                    } else {
                        let n = self.contar(i);
                        if n != 0 {
                            self.j.tirar_item(i.id, n, false);
                        }
                    }
                }
                if t.dinheiro_pedido != 0 && !limpando && sucesso {
                    let g = self.j.dinheiro().min(t.dinheiro_pedido);
                    self.j.tirar_dinheiro(g);
                }
            }
            metodo::MATAR_MONSTROS => {
                for m in &t.monster_kills {
                    if m.item_que_cai == 0 {
                        continue;
                    }
                    let mut n = self.j.contar(m.item_que_cai, m.item_comum);
                    if m.item_comum {
                        if m.quantidade_do_item != 0 && n > m.quantidade_do_item {
                            n = m.quantidade_do_item;
                        }
                    }
                    if n != 0 {
                        self.j.tirar_item(m.item_que_cai, n, m.item_comum);
                    }
                }
            }
            _ => {}
        }
    }

    /// `TakeAwayGivenItems` (`TaskTempl.inl:1719-1739`).
    fn tirar_itens_dados(&mut self, t: &TaskTemplate) {
        for i in &t.itens_entregues {
            let n = self.contar(i);
            if i.comum {
                let n = n.min(i.quantidade);
                if n != 0 {
                    self.j.tirar_item(i.id, n, true);
                }
            } else if n != 0 {
                self.j.tirar_item(i.id, n, false);
            }
        }
    }

    /// `ActiveTaskList::ClearChildrenOf` (`TaskProcess.h:400-408`) com a retirada de itens.
    fn limpar_filhos(&mut self, idx: usize, tirar_itens: bool) {
        while self.listas.ativa.e[idx].filho != SEM {
            let f = self.listas.ativa.e[idx].filho as usize;
            self.limpar(f, tirar_itens, true, false);
            self.listas.ativa.realinhar(f, 0);
        }
    }

    /// `RecursiveClearTask` com a retirada de itens do original.
    fn limpar(&mut self, idx: usize, tirar_itens: bool, tirar_adquiridos: bool, limpando_missao: bool) {
        let mut removidos: Vec<u32> = Vec::new();
        let tarefas = self.tarefas;
        limpar_recursivo(&mut self.listas.ativa, tarefas, idx, &mut |t, _| removidos.push(t.id));
        if tirar_itens {
            for id in removidos {
                let Some(t) = self.t(id) else { continue };
                if tirar_adquiridos || t.limpa_adquiridos {
                    self.tirar_adquiridos(t, limpando_missao, false);
                }
                self.tirar_itens_dados(t);
            }
        }
    }

    /// `ActiveTaskList::ClearTask` (`TaskProcess.h:394-398`).
    fn limpar_missao(&mut self, idx: usize, tirar_itens: bool) {
        self.limpar(idx, tirar_itens, true, true);
        self.listas.ativa.realinhar(idx, 0);
    }

    /// `_ConvertPeriod` e `_DeliverItem` (`TaskProcess.cpp:1163-1209`). Validade em data
    /// absoluta (bit 31) não é convertida.
    fn dar_item_de_premio(&mut self, t: &TaskTemplate, i: &ItemDeMissao, multiplo: u32) {
        let _ = t;
        let n = i.quantidade.wrapping_mul(multiplo);
        if (i.validade as u32) > 0x8000_0000 {
            return;
        }
        let validade = if n > 1 { 0 } else { i.validade };
        self.j.dar_item(i.id, n, i.comum, if i.comum { validade } else { 0 });
    }

    /// `CalcAwardMulti` (`TaskTempl.inl:1399-1441`).
    fn multiplicador(&self, t: &TaskTemplate, en: &Entrada) -> u32 {
        let tipo = if en.sucesso() { t.tipo_de_premio_sucesso } else { t.tipo_de_premio_falha };
        if tipo != 1 {
            return 1;
        }
        match t.metodo {
            metodo::COLETAR_ITENS => t.item_collections.iter().map(|i| self.contar(i)).sum(),
            metodo::MATAR_MONSTROS => t
                .monster_kills
                .iter()
                .enumerate()
                .map(|(k, m)| if m.item_que_cai != 0 { self.j.contar(m.item_que_cai, m.item_comum) } else { en.monstros(k) as u32 })
                .sum(),
            _ => 0,
        }
    }

    /// `ATaskTempl::DeliverByAwardData` (`TaskProcess.cpp:1226-1561`), sem os sistemas que o
    /// servidor não tem (ver a nota do módulo).
    fn premiar_com(&mut self, t: &'a TaskTemplate, en: Entrada, p: &TaskReward, escolha: i32) -> u32 {
        let mut multi = self.multiplicador(t, &en);
        if multi == 0 {
            return 1;
        }
        // `CalMultiByGlobalKeyValue`: variável global não definida vale 0.
        if p.multiplica {
            multi = multi.wrapping_mul(if p.tipo_do_multiplicador == 0 { 0 } else { p.multiplicador as u32 });
        }
        let ouro = (p.money as u32).wrapping_mul(multi);
        let mut exp = (p.exp as u32).wrapping_mul(multi);
        let mut sp = (p.sp as u32).wrapping_mul(multi);
        let repu = (p.reputation as i64 * multi as i64) as i32;
        if p.usa_coeficiente_de_nivel {
            let mut nivel = self.j.nivel().clamp(1, MAX_NIVEL_DO_COEFICIENTE);
            let teto = topo_de(self.tarefas, t).max_level;
            if teto != 0 && nivel > teto {
                nivel = teto;
            }
            let co = COEFICIENTE_DE_NIVEL[(nivel - 1) as usize];
            exp = exp.wrapping_mul(co);
            sp = sp.wrapping_mul(co);
        }
        if p.money != 0 {
            self.j.dar_dinheiro(ouro);
        }
        if p.exp != 0 {
            self.j.dar_exp(exp & 0x3FFF_FFFF, 0);
        }
        if p.sp != 0 {
            self.j.dar_exp(0, sp & 0x3FFF_FFFF);
        }
        if p.reputation != 0 {
            self.j.dar_reputacao(repu);
        }
        // `if (pAward->m_ulNewPeriod) pTask->SetCurPeriod(...)` (`TaskProcess.cpp:1284`):
        // é assim que a missão de cultivo sobe o nível de cultivo — sem multiplicador, que o
        // original não aplica aqui (B67).
        if p.novo_cultivo != 0 {
            self.j.definir_cultivo(p.novo_cultivo);
        }
        // `if (pAward->m_ulFuryULimit) pTask->SetFuryUpperLimit(...)` — é assim que a barra
        // de chi aparece e cresce (99 → 199 → 299 → 399 nas missões do realm_155), B68.
        if p.teto_de_chi != 0 {
            self.j.definir_teto_de_chi(p.teto_de_chi);
        }
        let mut ret = 0;
        if !p.grupos_de_itens.is_empty() {
            let e = if escolha < 0 || escolha as usize >= p.grupos_de_itens.len() { 0 } else { escolha as usize };
            let g = p.grupos_de_itens[e].clone();
            // `CanAwardItems` (`TaskTempl.inl:1322-1329`).
            let (mut c, mut m) = (0u32, 0u32);
            if g.sorteia_um {
                for i in &g.itens {
                    if i.probabilidade >= 1.0 {
                        if i.comum { c += 1 } else { m += 1 }
                    }
                }
                c += 1;
                m += 1;
            } else {
                (c, m) = g.contagens();
            }
            if self.j.slots_livres(true) >= c && self.j.slots_livres(false) >= m {
                if g.sorteia_um {
                    let mut prob = self.j.sortear();
                    let mut dado = false;
                    for i in &g.itens {
                        if i.probabilidade > 0.9999 {
                            self.dar_item_de_premio(t, i, multi);
                        } else if !dado {
                            if prob <= i.probabilidade {
                                self.dar_item_de_premio(t, i, multi);
                                dado = true;
                            } else {
                                prob -= i.probabilidade;
                            }
                        }
                    }
                } else {
                    for i in &g.itens {
                        if self.j.sortear() <= i.probabilidade {
                            self.dar_item_de_premio(t, i, multi);
                        }
                    }
                }
            } else {
                ret = 2;
            }
        }
        if p.nova_missao != 0 {
            if self.t(p.nova_missao).is_some_and(|n| n.parent.is_none()) {
                self.aceitar(p.nova_missao, 0, true);
            }
        }
        // `if (pAward->m_ulSummonedMonsters)` (`TaskProcess.cpp:1349-1400`).
        if let Some(ref inv) = p.monstros_invocados {
            if inv.sorteia_um {
                let total_prob: f32 = inv.monstros.iter().map(|m| m.probabilidade).sum();
                if (total_prob - 1.0).abs() < 0.00001 {
                    let mut prob = self.j.sortear();
                    let mut dado = false;
                    for m in &inv.monstros {
                        if !dado {
                            if prob <= m.probabilidade {
                                self.j.invocar_monstro(m.monstro, m.quantidade, inv.raio, m.periodo, inv.some_ao_morrer);
                                dado = true;
                            } else {
                                prob -= m.probabilidade;
                            }
                        }
                    }
                } else {
                    for m in &inv.monstros {
                        if self.j.sortear() <= m.probabilidade {
                            self.j.invocar_monstro(m.monstro, m.quantidade, inv.raio, m.periodo, inv.some_ao_morrer);
                        }
                    }
                }
            } else {
                // Sem `m_bRandChoose` o original invoca **todos**, sem sortear
                // (`TaskProcess.cpp:1427-1434`). Sortear aqui era divergência (B67).
                for m in &inv.monstros {
                    self.j.invocar_monstro(m.monstro, m.quantidade, inv.raio, m.periodo, inv.some_ao_morrer);
                }
            }
        }
        // `if (pAward->m_ulTransWldId) pTask->TransportTo(...)` (`TaskProcess.cpp:1316-1317`),
        // depois do resto do prêmio.
        if let Some((mundo, pos)) = p.teleporte {
            self.j.teleportar(mundo, pos);
        }
        let _ = en;
        ret
    }

    /// `ATaskTempl::RecursiveAward` (`TaskProcess.cpp:921-1120`).
    fn premiar_recursivo(&mut self, t: &'a TaskTemplate, idx: usize, agora: u32, escolha: i32, tags: &mut Etiquetas) {
        let en = self.listas.ativa.e[idx];
        let falha_sem_tirar = !en.sucesso() && t.nao_limpa_item_na_falha && t.limpa_adquiridos;
        self.limpar_filhos(idx, !falha_sem_tirar);
        if !self.listas.ativa.e[idx].valida {
            return;
        }
        let en = self.listas.ativa.e[idx];
        if t.parent.is_none() && t.precisa_registro {
            self.listas.marcar_concluida(t.id, en.sucesso());
        }
        if t.parent.is_none() && t.limite_de_conta {
            self.verificar_frequencia(t, agora);
            if !t.nao_conta_falha || en.sucesso() {
                self.listas.registrar_contagem(t.id, agora);
            }
        } else if t.parent.is_none() && t.limite_de_personagem {
            self.verificar_frequencia(t, agora);
            if !t.nao_conta_falha || en.sucesso() {
                self.listas.somar_vez(t.id, en.sucesso());
                self.listas.registrar_hora(t.id, agora);
            }
        }

        if let Some(p) = self.premio_de(t, &en).cloned() {
            self.premiar_com(t, en, &p, escolha);
        }
        if t.limpa_adquiridos {
            if !falha_sem_tirar {
                self.tirar_adquiridos(t, false, en.sucesso());
            }
        } else if !en.sucesso() {
            self.tirar_itens_dados(t);
        }

        // A entrega de uma missão nova no prêmio pode ter deslocado a lista: o original usa
        // o ponteiro da entrada, que continua apontando para a mesma posição.
        let a = &mut self.listas.ativa;
        let en = a.e[idx];
        a.e[idx].valida = false;
        a.e[idx].id = 0;
        a.quantidade = a.quantidade.saturating_sub(1);

        if en.pai != SEM {
            let pi = en.pai as usize;
            if en.anterior != SEM {
                a.e[en.anterior as usize].proximo = en.proximo;
            } else {
                a.e[pi].filho = en.proximo;
            }
            if en.proximo != SEM {
                a.e[en.proximo as usize].anterior = en.anterior;
            }
            let Some(pai) = pai_de(self.tarefas, t) else {
                a.realinhar(idx, 0);
                return;
            };
            if !en.sucesso() && t.pai_tambem_falha {
                a.realinhar(idx, 0);
                a.e[pi].estado &= !estado::SUCESSO;
                a.e[pi].estado |= estado::FINALIZADA;
                self.premiar_recursivo(pai, pi, agora, -1, tags);
            } else if en.sucesso() && t.pai_tambem_sucesso {
                a.realinhar(idx, 0);
                a.e[pi].estado |= estado::FINALIZADA;
                self.limpar_filhos(pi, true);
                if pai.tipo_de_conclusao == conclusao::DIRETA {
                    self.premiar_recursivo(pai, pi, agora, -1, tags);
                }
            } else if pai.filhos_em_ordem && proximo_irmao(self.tarefas, t).is_some() {
                let irmao = proximo_irmao(self.tarefas, t).expect("conferido");
                if a.e[pi].filho != SEM || a.indice(irmao.id).is_some() {
                    a.realinhar(idx, 0);
                } else {
                    self.entregar(irmao, Some(idx), 0, agora, None, tags, en.pai);
                }
            } else if a.e[pi].filho == SEM {
                a.realinhar(idx, 0);
                a.e[pi].estado |= estado::FINALIZADA;
                if pai.tipo_de_conclusao == conclusao::DIRETA {
                    self.premiar_recursivo(pai, pi, agora, -1, tags);
                }
            } else {
                a.realinhar(idx, 0);
            }
        } else {
            a.realinhar(idx, 0);
            a.usados = a.usados.checked_sub(t.profundidade).unwrap_or(0);
            if t.oculta {
                a.topo_ocultas = a.topo_ocultas.saturating_sub(1);
            } else if t.na_janela_de_titulo {
                a.de_titulo = a.de_titulo.saturating_sub(1);
            } else {
                a.topo_visiveis = a.topo_visiveis.saturating_sub(1);
            }
        }
    }

    /// `RecursiveCheckParent` (`TaskProcess.cpp:906-919`).
    fn pais_com_sucesso(&self, idx: usize) -> bool {
        let mut i = idx;
        loop {
            let en = self.listas.ativa.e[i];
            if en.pai == SEM {
                return true;
            }
            let p = self.listas.ativa.e[en.pai as usize];
            if !p.sucesso() {
                return false;
            }
            i = en.pai as usize;
        }
    }

    /// `ATaskTempl::DeliverAward` (`TaskProcess.cpp:1882-2028`).
    pub fn entregar_premio(&mut self, idx: usize, escolha: i32) -> bool {
        let en = self.listas.ativa.e[idx];
        let Some(t) = self.t(en.id as u32).filter(|_| en.valida) else { return false };
        let agora = self.j.agora();
        // `RecursiveCheckTimeLimit`.
        let mut i = idx;
        loop {
            let e = self.listas.ativa.e[i];
            if let Some(ti) = self.t(e.id as u32) {
                if ti.limite_de_tempo != 0 && e.tempo.wrapping_add(ti.limite_de_tempo) < agora {
                    self.listas.ativa.e[i].estado &= !estado::SUCESSO;
                }
            }
            if e.pai == SEM {
                break;
            }
            i = e.pai as usize;
        }
        if !self.pais_com_sucesso(idx) {
            self.listas.ativa.e[idx].estado &= !estado::SUCESSO;
        }
        let en = self.listas.ativa.e[idx];
        if en.desistiu() && t.limpa_ao_desistir {
            self.limpar_missao(idx, true);
            self.j.avisar(S2CGamedataSend::task_notify_base(aviso::DESISTENCIA, t.id as u16).data);
            return true;
        }
        self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
        let r = self.verificar_premio(t, idx, escolha);
        if r != 0 {
            if t.tipo_de_conclusao == conclusao::NO_NPC || self.listas.ativa.e[idx].estado & estado::ERRO_AVISADO == 0 {
                self.avisar_erro(t.id, r);
                self.listas.ativa.e[idx].estado |= estado::ERRO_AVISADO;
            }
            return false;
        }
        let mut tags = Etiquetas { uniao: self.listas.ativa.e[idx].estado as u16, tags: Vec::new() };
        self.premiar_recursivo(t, idx, agora, escolha, &mut tags);
        self.j.avisar(S2CGamedataSend::task_notify_complete(t.id as u16, agora, &tags.bytes()).data);
        true
    }

    /// `OnSetFinished` (`TaskTempl.inl:2184-2204`).
    fn ao_finalizar(&mut self, t: &'a TaskTemplate, idx: usize) {
        self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
        self.j.avisar(S2CGamedataSend::task_notify_base(aviso::FINALIZADA, t.id as u16).data);
        if t.tipo_de_conclusao == conclusao::DIRETA || !self.listas.ativa.e[idx].sucesso() {
            self.entregar_premio(idx, -1);
        }
    }

    // ------------------------------------------------------------------ eventos

    /// `HasAllMonsterWanted` (`TaskTempl.inl:105-126`).
    fn tem_todos_os_monstros(&self, t: &TaskTemplate, en: &Entrada) -> bool {
        let mut algum = false;
        for (k, m) in t.monster_kills.iter().enumerate().take(3) {
            if m.item_que_cai != 0 {
                let n = self.j.contar(m.item_que_cai, m.item_comum);
                if n < m.quantidade_do_item {
                    return false;
                }
                if n != 0 {
                    algum = true;
                }
            } else if (en.monstros(k) as u32) < m.quantidade {
                return false;
            } else if en.monstros(k) != 0 {
                algum = true;
            }
        }
        algum
    }

    /// `HasAllItemsWanted` (`TaskTempl.inl:78-103`).
    fn tem_todos_os_itens(&self, t: &TaskTemplate) -> bool {
        if t.dinheiro_pedido != 0 && self.j.dinheiro() < t.dinheiro_pedido {
            return false;
        }
        t.item_collections.iter().all(|i| {
            let n = self.contar(i);
            n != 0 && n >= i.quantidade
        })
    }

    /// `ATaskTempl::CheckKillMonster` (`TaskTempl.inl:1972-2070`), fora de equipe.
    fn conferir_abate(&mut self, t: &'a TaskTemplate, idx: usize, monstro: u32, nivel_do_monstro: u32, mut sorteio: f32) -> bool {
        if t.metodo != metodo::MATAR_MONSTROS {
            return false;
        }
        if t.em_equipe || topo_de(self.tarefas, t).em_equipe {
            // `bTeam != ((teamwork) && IsInTeam())` — sem equipe, `IsInTeam` é falso e a
            // conta segue normalmente.
        }
        let mut ret = false;
        let nivel = self.j.nivel();
        for k in 0..t.monster_kills.len().min(3) {
            let m = t.monster_kills[k];
            if m.monstro != 0 && m.monstro != monstro {
                continue;
            }
            if m.nivel_do_matador && nivel > nivel_do_monstro + DIFERENCA_MAXIMA_DE_NIVEL {
                continue;
            }
            let en = self.listas.ativa.e[idx];
            if m.item_que_cai != 0 {
                let n = self.j.contar(m.item_que_cai, m.item_comum);
                if m.quantidade_do_item != 0 && n >= m.quantidade_do_item {
                    if !en.finalizada() && self.tem_todos_os_monstros(t, &en) {
                        self.ao_finalizar(t, idx);
                        return true;
                    }
                    continue;
                }
                ret = true;
                if m.chance_do_item < sorteio {
                    sorteio -= m.chance_do_item;
                    continue;
                }
                if self.j.slots_livres(m.item_comum) >= 1 {
                    self.j.dar_item(m.item_que_cai, 1, m.item_comum, 0);
                }
                let en = self.listas.ativa.e[idx];
                if self.tem_todos_os_monstros(t, &en) {
                    self.ao_finalizar(t, idx);
                }
                return true;
            } else {
                if m.quantidade != 0 && en.monstros(k) as u32 >= m.quantidade {
                    if !en.finalizada() && self.tem_todos_os_monstros(t, &en) {
                        self.ao_finalizar(t, idx);
                        return true;
                    }
                    continue;
                }
                let n = en.monstros(k).wrapping_add(1);
                self.listas.ativa.e[idx].definir_monstros(k, n);
                self.j.avisar(S2CGamedataSend::task_notify_monster_killed(t.id as u16, m.monstro, n, 0, 0).data);
                let en = self.listas.ativa.e[idx];
                if self.tem_todos_os_monstros(t, &en) {
                    self.ao_finalizar(t, idx);
                }
                return true;
            }
        }
        ret
    }

    /// `OnTaskKillMonster` / `_on_kill_monster` (`TaskServer.cpp:1068-1109`).
    pub fn abateu_monstro(&mut self, monstro: u32, nivel_do_monstro: u32) {
        let sorteio = self.j.sortear();
        let mut i: i64 = 0;
        while (i as usize) < self.listas.ativa.quantidade as usize {
            let idx = i as usize;
            let en = self.listas.ativa.e[idx];
            let Some(t) = self.t(en.id as u32).filter(|_| en.valida) else {
                i += 1;
                continue;
            };
            self.conferir_abate(t, idx, monstro, nivel_do_monstro, sorteio);
            // A lista mudou por baixo (uma missão acabou): recomeça.
            if self.listas.ativa.e[idx].id as u32 != t.id {
                i = 0;
                continue;
            }
            i += 1;
        }
    }

    /// `OnTaskMining` (`TaskServer.cpp:1116-1123`) / `ATaskTempl::CheckMining` (`TaskTempl.inl:2105-2148`).
    pub fn colheu_mina(&mut self, task_id: u32) {
        if task_id == 0 {
            return;
        }
        let Some(idx) = (0..self.listas.ativa.quantidade as usize)
            .find(|&i| self.listas.ativa.e[i].id as u32 == task_id && self.listas.ativa.e[i].valida)
        else {
            return;
        };
        let Some(t) = self.t(task_id) else { return };
        if t.metodo != metodo::COLETAR_ITENS || t.item_collections.is_empty() {
            return;
        }
        let item_pedido = &t.item_collections[0];
        let atual = self.j.contar(item_pedido.id, item_pedido.comum);
        if item_pedido.quantidade != 0 && atual >= item_pedido.quantidade {
            return;
        }
        if self.j.slots_livres(item_pedido.comum) >= 1 {
            self.j.dar_item(item_pedido.id, 1, item_pedido.comum, 0);
        }
        let en = self.listas.ativa.e[idx];
        if !en.finalizada() && self.tem_todos_os_itens(t) {
            self.ao_finalizar(t, idx);
        }
    }

    /// `OnTaskCheckAward` (`TaskServer.cpp:767-862`) — entregar no NPC.
    pub fn entregar_no_npc(&mut self, id: u32, escolha: i32) -> bool {
        let Some(idx) = (0..self.listas.ativa.quantidade as usize).find(|&i| self.listas.ativa.e[i].id as u32 == id && self.listas.ativa.e[i].valida) else {
            return false;
        };
        let Some(t) = self.t(id) else { return false };
        if t.tipo_de_conclusao != conclusao::NO_NPC || t.casamento {
            return false;
        }
        let topo = topo_de(self.tarefas, t);
        if self.listas.ativa.indice(topo.id).is_none() {
            return false;
        }
        let en = self.listas.ativa.e[idx];
        match t.metodo {
            metodo::COLETAR_ITENS => {
                if t.sub_tasks.is_empty() && self.tem_todos_os_itens(t) {
                    self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                    return self.entregar_premio(idx, escolha);
                }
                false
            }
            metodo::MATAR_MONSTROS => {
                if en.finalizada() && t.sub_tasks.is_empty() && self.tem_todos_os_monstros(t, &en) {
                    return self.entregar_premio(idx, escolha);
                }
                false
            }
            metodo::ESPERAR => {
                if t.sub_tasks.is_empty() && en.tempo.wrapping_add(t.espera) < self.j.agora() {
                    self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                    return self.entregar_premio(idx, escolha);
                }
                false
            }
            metodo::FALAR_COM_NPC | metodo::SIMPLES_DO_CLIENTE | metodo::SIMPLES_DO_CLIENTE_NAVEGACAO => {
                self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                self.entregar_premio(idx, escolha)
            }
            metodo::ALCANCAR_NIVEL => {
                if self.j.nivel() >= t.nivel_a_alcancar {
                    self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                    return self.entregar_premio(idx, escolha);
                }
                false
            }
            _ => {
                if en.finalizada() {
                    return self.entregar_premio(idx, escolha);
                }
                false
            }
        }
    }

    /// `OnTaskCheckAwardDirect` (`TaskServer.cpp:864-1066`) — o cliente pede para concluir
    /// (`TASK_CLT_NOTIFY_CHECK_FINISH`), sem as falhas por região e facção.
    pub fn conferir_conclusao(&mut self, id: u32) -> bool {
        let Some(idx) = (0..self.listas.ativa.quantidade as usize).find(|&i| self.listas.ativa.e[i].id as u32 == id && self.listas.ativa.e[i].valida) else {
            return false;
        };
        let Some(t) = self.t(id) else { return false };
        let topo = topo_de(self.tarefas, t);
        if self.listas.ativa.indice(topo.id).is_none() {
            return false;
        }
        let en = self.listas.ativa.e[idx];
        let agora = self.j.agora();
        if en.finalizada() && !en.sucesso() {
            return self.entregar_premio(idx, -1);
        }
        if t.limite_de_tempo != 0 && en.tempo.wrapping_add(t.limite_de_tempo) < agora {
            self.listas.ativa.e[idx].estado &= !estado::SUCESSO;
            self.ao_finalizar(t, idx);
            return true;
        }
        if t.tipo_de_conclusao != conclusao::DIRETA && t.tipo_de_conclusao != conclusao::CONFIRMAR || t.casamento {
            return false;
        }
        match t.metodo {
            metodo::COLETAR_ITENS => {
                if t.sub_tasks.is_empty() && self.tem_todos_os_itens(t) {
                    self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                    return self.entregar_premio(idx, -1);
                }
                false
            }
            metodo::MATAR_MONSTROS => {
                if en.finalizada() && t.sub_tasks.is_empty() && self.tem_todos_os_monstros(t, &en) {
                    return self.entregar_premio(idx, -1);
                }
                false
            }
            metodo::ESPERAR => {
                if t.sub_tasks.is_empty() && en.tempo.wrapping_add(t.espera) < agora {
                    self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                    return self.entregar_premio(idx, -1);
                }
                false
            }
            metodo::ALCANCAR_NIVEL => {
                if self.j.nivel() >= t.nivel_a_alcancar {
                    self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                    return self.entregar_premio(idx, -1);
                }
                false
            }
            metodo::SIMPLES_DO_CLIENTE | metodo::SIMPLES_DO_CLIENTE_NAVEGACAO => {
                self.listas.ativa.e[idx].estado |= estado::FINALIZADA;
                self.entregar_premio(idx, -1)
            }
            _ => {
                if en.finalizada() {
                    return self.entregar_premio(idx, -1);
                }
                false
            }
        }
    }

    /// `OnTaskGiveUpOneTask` + `GiveUpOneTask` (`TaskServer.cpp:1156`, `TaskTempl.inl:2170`).
    pub fn desistir(&mut self, id: u32) -> bool {
        let Some(idx) = (0..self.listas.ativa.quantidade as usize).find(|&i| self.listas.ativa.e[i].valida && self.listas.ativa.e[i].id as u32 == id) else {
            return false;
        };
        let Some(t) = self.t(id) else { return false };
        if t.parent.is_some() || !t.pode_desistir {
            return false;
        }
        self.listas.ativa.e[idx].estado &= !estado::SUCESSO;
        self.listas.ativa.e[idx].estado |= estado::DESISTIU;
        self.ao_finalizar(t, idx);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pw_data_loader::tasks::MonstroPedido;
    use std::collections::HashMap;

    #[derive(Default)]
    struct JogadorDeTeste {
        nivel: u32,
        classe: u32,
        itens: HashMap<(u32, bool), u32>,
        dinheiro: u32,
        exp: u32,
        sp: u32,
        avisos: Vec<Vec<u8>>,
        posicao: (u32, [f32; 3]),
        faccao: (u32, i32),
        equipe: Vec<MembroDaEquipe>,
        teleporte: Option<(u32, [f32; 3])>,
    }

    impl Jogador for JogadorDeTeste {
        fn agora(&self) -> u32 { 1_000_000 }
        fn nivel(&self) -> u32 { self.nivel }
        fn classe(&self) -> u32 { self.classe }
        fn masculino(&self) -> bool { true }
        fn cultivo(&self) -> u32 { 0 }
        fn reputacao(&self) -> i32 { 0 }
        fn dinheiro(&self) -> u32 { self.dinheiro }
        fn e_gm(&self) -> bool { false }
        fn contar(&self, tid: u32, comum: bool) -> u32 { self.itens.get(&(tid, comum)).copied().unwrap_or(0) }
        fn slots_livres(&self, _: bool) -> u32 { 32 }
        fn dar_item(&mut self, tid: u32, q: u32, comum: bool, _: i32) { *self.itens.entry((tid, comum)).or_default() += q; }
        fn tirar_item(&mut self, tid: u32, q: u32, comum: bool) { let e = self.itens.entry((tid, comum)).or_default(); *e = e.saturating_sub(q); }
        fn dar_dinheiro(&mut self, n: u32) { self.dinheiro += n; }
        fn tirar_dinheiro(&mut self, n: u32) { self.dinheiro -= n.min(self.dinheiro); }
        fn dar_exp(&mut self, exp: u32, sp: u32) { self.exp += exp; self.sp += sp; }
        fn dar_reputacao(&mut self, _: i32) {}
    fn definir_cultivo(&mut self, _: u32) {}
    fn definir_teto_de_chi(&mut self, _: u32) {}
        fn avisar(&mut self, c: Vec<u8>) { self.avisos.push(c); }
        fn sortear(&mut self) -> f32 { 0.0 }
        fn posicao(&self) -> (u32, [f32; 3]) { self.posicao }
        fn faccao(&self) -> (u32, i32) { self.faccao }
        fn equipe(&self) -> Vec<MembroDaEquipe> { self.equipe.clone() }
        fn teleportar(&mut self, mundo: u32, pos: [f32; 3]) { self.teleporte = Some((mundo, pos)); }
    }

    fn modelo(id: u32) -> TaskTemplate {
        TaskTemplate::vazia(id)
    }

    fn dados(ts: Vec<TaskTemplate>) -> TasksData {
        let mut d = TasksData::default();
        for t in ts {
            if t.parent.is_none() {
                d.de_topo.push(t.id);
            }
            d.tasks.insert(t.id, t);
        }
        d
    }

    #[test]
    fn aceitar_poe_na_lista_e_manda_o_aviso_com_o_tamanho_do_original() {
        let mut t = modelo(32201);
        t.metodo = metodo::FALAR_COM_NPC;
        t.tipo_de_conclusao = conclusao::NO_NPC;
        t.rewards.exp = 25;
        t.rewards.sp = 10;
        t.rewards.money = 8;
        let d = dados(vec![t]);
        let mut l = ListasDeMissao::default();
        let mut j = JogadorDeTeste { nivel: 1, classe: 6, ..Default::default() };
        let mut m = Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 };
        assert_eq!(m.aceitar(32201, 0, true), 0);
        assert_eq!(l.ativa.quantidade, 1);
        assert_eq!(l.ativa.topo_visiveis, 1);
        assert_eq!(l.ativa.usados, 1);
        // TASK_VAR_DATA: 2 + 4 + (3 + 8 + 3).
        assert_eq!(j.avisos[0].len(), 2 + 4 + 14);
        assert_eq!(j.avisos[0][6], aviso::NOVA);

        let mut m = Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 };
        assert!(m.entregar_no_npc(32201, 0));
        assert_eq!(l.ativa.quantidade, 0);
        assert_eq!(l.procurar_concluida(32201), 0);
        assert_eq!((j.exp, j.sp, j.dinheiro), (25, 10, 8));
        let completa = j.avisos.last().unwrap();
        assert_eq!(completa[6], aviso::CONCLUIDA);
        // estado: finalizada + sucesso.
        assert_eq!(completa[6 + 7], estado::FINALIZADA | estado::SUCESSO);

        let mut m = Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 };
        assert_eq!(m.aceitar(32201, 0, false), erro::NAO_REPETE);
    }

    fn aceitar(d: &TasksData, j: &mut JogadorDeTeste, id: u32) -> u32 {
        let mut l = ListasDeMissao::default();
        Motor { tarefas: d, listas: &mut l, j, eu: 1 }.aceitar(id, 0, false)
    }

    fn momento(ano: i32, hora: i32, minuto: i32) -> pw_data_loader::tasks::MomentoDeMissao {
        pw_data_loader::tasks::MomentoDeMissao { ano, mes: 1, dia: 1, hora, minuto, dia_da_semana: 0 }
    }

    /// `CheckTimetable`: basta uma janela valer. Uma janela diária de 00:00 a 24:00 vale a
    /// qualquer hora; uma por data que acabou em 2011 nunca vale.
    #[test]
    fn a_janela_de_horario_e_conferida_e_nao_mais_recusada_sempre() {
        use pw_data_loader::tasks::JanelaDeHorario;
        let mut t = modelo(1);
        t.janelas = vec![JanelaDeHorario { tipo: 3, inicio: momento(0, 0, 0), fim: momento(0, 24, 0) }];
        let mut velha = modelo(2);
        velha.janelas = vec![JanelaDeHorario { tipo: 0, inicio: momento(2010, 0, 0), fim: momento(2011, 0, 0) }];
        let d = dados(vec![t, velha]);
        let agora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;
        assert!(janela_vale(&d.get_task(1).unwrap().janelas[0], agora));
        assert!(!janela_vale(&d.get_task(2).unwrap().janelas[0], agora));
        let mut j = JogadorDeTeste::default();
        // O `agora` do jogador de teste é 1970; a janela por data de 2010 continua fechada.
        assert_eq!(aceitar(&d, &mut j, 2), erro::HORARIO);
    }

    /// `CheckInZone`: mundo **e** caixa.
    #[test]
    fn entrega_em_zona_exige_o_mundo_e_a_caixa() {
        let mut t = modelo(1);
        t.entrega_em_zona = true;
        t.mundo_de_entrega = 161;
        t.regioes_de_entrega = vec![pw_data_loader::tasks::RegiaoDeMissao { min: [-900.0, -9999.0, -300.0], max: [-800.0, 9999.0, -200.0] }];
        let d = dados(vec![t]);
        let mut dentro = JogadorDeTeste { posicao: (161, [-821.0, 44.0, -259.0]), ..Default::default() };
        assert_eq!(aceitar(&d, &mut dentro, 1), 0);
        let mut outro_mapa = JogadorDeTeste { posicao: (1, [-821.0, 44.0, -259.0]), ..Default::default() };
        assert_eq!(aceitar(&d, &mut outro_mapa, 1), erro::FORA_DA_ZONA);
        let mut fora = JogadorDeTeste { posicao: (161, [0.0, 44.0, 0.0]), ..Default::default() };
        assert_eq!(aceitar(&d, &mut fora, 1), erro::FORA_DA_ZONA);
    }

    /// `CheckFaction`: em facção e com cargo até `m_iPremise_FactionRole`.
    #[test]
    fn faccao_exige_estar_em_uma_com_o_cargo_permitido() {
        let mut t = modelo(1);
        t.faccao = 1;
        t.papel_na_faccao = 3;
        let d = dados(vec![t]);
        assert_eq!(aceitar(&d, &mut JogadorDeTeste::default(), 1), erro::FACCAO);
        assert_eq!(aceitar(&d, &mut JogadorDeTeste { faccao: (77, 5), ..Default::default() }, 1), erro::FACCAO);
        assert_eq!(aceitar(&d, &mut JogadorDeTeste { faccao: (77, 2), ..Default::default() }, 1), 0);
    }

    fn membro(id: u32, nivel: u32) -> MembroDaEquipe {
        MembroDaEquipe { id, nivel, classe: 6, masculino: true, mundo: 161, pos: [0.0; 3] }
    }

    /// `CheckTeamTask` + `HasAllTeamMemsWanted`, e o membro recebendo pelo capitão com o id
    /// dele no `cap_task` do aviso.
    #[test]
    fn missao_de_equipe_so_o_capitao_recebe_e_os_membros_ganham_junto() {
        let mut t = modelo(50);
        t.em_equipe = true;
        t.recebida_pela_equipe = true;
        t.membros_pedidos = vec![pw_data_loader::tasks::MembroPedido { nivel_minimo: 10, classe: 0xFFFF_FFFF, minimo: 1, ..Default::default() }];
        let d = dados(vec![t]);

        let mut sozinho = JogadorDeTeste::default();
        assert_eq!(aceitar(&d, &mut sozinho, 50), erro::NAO_E_CAPITAO);

        let mut nao_capitao = JogadorDeTeste { equipe: vec![membro(9, 20), membro(1, 20)], ..Default::default() };
        assert_eq!(aceitar(&d, &mut nao_capitao, 50), erro::NAO_E_CAPITAO);

        let mut fraco = JogadorDeTeste { equipe: vec![membro(1, 20), membro(2, 5)], ..Default::default() };
        assert_eq!(aceitar(&d, &mut fraco, 50), erro::MEMBRO_INVALIDO);

        let mut capitao = JogadorDeTeste { nivel: 20, equipe: vec![membro(1, 20), membro(2, 15)], ..Default::default() };
        assert_eq!(aceitar(&d, &mut capitao, 50), 0);

        let mut l = ListasDeMissao::default();
        let mut m2 = JogadorDeTeste { nivel: 15, classe: 6, equipe: vec![membro(1, 20), membro(2, 15)], ..Default::default() };
        assert_eq!(Motor { tarefas: &d, listas: &mut l, j: &mut m2, eu: 2 }.aceitar_como_membro(50), 0);
        assert_eq!(l.ativa.quantidade, 1);
        // `task_notify_new`: 2 + reason 1 + task 2 + cur_time 4 → cap_task em 9.
        let aviso = m2.avisos.last().unwrap();
        assert_eq!(u32::from_le_bytes(aviso[13..17].try_into().unwrap()), 0, "a missão do capitão vai com cap_task 0");
    }

    /// `OnTaskReachSite`: o cliente avisa, o servidor confere o lugar e finaliza.
    #[test]
    fn chegar_ao_lugar_so_finaliza_dentro_da_caixa_do_mundo_certo() {
        let mut t = modelo(60);
        t.metodo = metodo::ALCANCAR_LUGAR;
        t.tipo_de_conclusao = conclusao::DIRETA;
        t.mundo_a_alcancar = 1;
        t.lugares_a_alcancar = vec![pw_data_loader::tasks::RegiaoDeMissao { min: [0.0, -10.0, 0.0], max: [10.0, 10.0, 10.0] }];
        t.rewards.exp = 7;
        let d = dados(vec![t]);
        let mut l = ListasDeMissao::default();
        let mut j = JogadorDeTeste { posicao: (1, [50.0, 0.0, 50.0]), ..Default::default() };
        assert_eq!(Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.aceitar(60, 0, false), 0);
        assert!(!Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.conferir_lugar(60, false));
        j.posicao = (1, [5.0, 0.0, 5.0]);
        assert!(Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.conferir_lugar(60, false));
        assert_eq!(l.ativa.quantidade, 0, "direta: finalizou e premiou");
        assert_eq!(j.exp, 7);
    }

    /// `if (pAward->m_ulTransWldId) pTask->TransportTo(...)`.
    #[test]
    fn premio_com_teleporte_pede_o_teleporte_ao_jogador() {
        let mut t = modelo(31379);
        t.metodo = metodo::FALAR_COM_NPC;
        t.tipo_de_conclusao = conclusao::NO_NPC;
        t.rewards.teleporte = Some((1, [-319.667, 220.007, -900.309]));
        let d = dados(vec![t]);
        let mut l = ListasDeMissao::default();
        let mut j = JogadorDeTeste::default();
        assert_eq!(Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.aceitar(31379, 0, false), 0);
        assert!(j.teleporte.is_none());
        assert!(Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.entregar_no_npc(31379, 0));
        assert_eq!(j.teleporte, Some((1, [-319.667, 220.007, -900.309])));
    }

    #[test]
    fn matar_conta_e_finaliza_e_a_lista_volta_a_zero() {
        let mut t = modelo(100);
        t.metodo = metodo::MATAR_MONSTROS;
        t.tipo_de_conclusao = conclusao::DIRETA;
        t.monster_kills = vec![MonstroPedido { monstro: 7, quantidade: 2, item_que_cai: 0, quantidade_do_item: 0, item_comum: false, chance_do_item: 0.0, nivel_do_matador: false, dps: 0, dph: 0 }];
        t.rewards.exp = 50;
        let d = dados(vec![t]);
        let mut l = ListasDeMissao::default();
        let mut j = JogadorDeTeste { nivel: 5, ..Default::default() };
        Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.aceitar(100, 0, true);
        Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.abateu_monstro(7, 5);
        assert_eq!(l.ativa.e[0].monstros(0), 1);
        assert_eq!(j.avisos.last().unwrap().len(), 2 + 4 + 17);
        Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.abateu_monstro(7, 5);
        assert_eq!(l.ativa.quantidade, 0, "conclusão direta ao completar");
        assert_eq!(j.exp, 50);
    }

    #[test]
    fn filhos_em_ordem_entregam_o_seguinte_na_mesma_posicao() {
        let mut pai = modelo(10);
        pai.sub_tasks = vec![11, 12];
        pai.filhos_em_ordem = true;
        pai.metodo = metodo::NENHUM;
        pai.profundidade = 2;
        let mut a = modelo(11);
        a.parent = Some(10);
        a.metodo = metodo::FALAR_COM_NPC;
        a.tipo_de_conclusao = conclusao::NO_NPC;
        let mut b = modelo(12);
        b.parent = Some(10);
        b.metodo = metodo::FALAR_COM_NPC;
        b.tipo_de_conclusao = conclusao::NO_NPC;
        let d = dados(vec![pai, a, b]);
        let mut l = ListasDeMissao::default();
        let mut j = JogadorDeTeste::default();
        Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.aceitar(10, 0, true);
        assert_eq!(l.ativa.quantidade, 2);
        assert_eq!((l.ativa.e[1].id, l.ativa.e[1].pai), (11, 0));
        assert_eq!(l.ativa.e[0].filho, 1);
        assert!(Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.entregar_no_npc(11, 0));
        assert_eq!(l.ativa.quantidade, 2);
        assert_eq!((l.ativa.e[1].id, l.ativa.e[1].pai), (12, 0));
        assert!(Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.entregar_no_npc(12, 0));
        assert_eq!(l.ativa.quantidade, 0, "o último filho conclui o pai de conclusão direta");
        assert_eq!(l.procurar_concluida(10), 0);
    }

    #[test]
    fn as_listas_vazias_sao_as_que_o_link_manda() {
        // `listas_de_missao_vazias` em `pw-link/src/gateway.rs` repete estes bytes.
        let b = ListasDeMissao::default().blocos();
        assert_eq!(b[0], vec![0, 0, 1, 0, 0, 1, 0, 0]);
        assert_eq!(b[1], vec![0, 0, 1, 0]);
        assert_eq!((b[2].clone(), b[3].clone()), (vec![0, 0], vec![0, 0]));
        assert_eq!(b[4], vec![0; TAM_DEPOSITO]);
    }

    #[test]
    fn as_listas_vao_e_voltam_pelos_bytes() {
        let mut t = modelo(5);
        t.metodo = metodo::FALAR_COM_NPC;
        let d = dados(vec![t]);
        let mut l = ListasDeMissao::default();
        let mut j = JogadorDeTeste::default();
        Motor { tarefas: &d, listas: &mut l, j: &mut j, eu: 1 }.aceitar(5, 0, true);
        l.marcar_concluida(3, false);
        let b = l.blocos();
        assert_eq!(b[0].len(), TAM_CABECALHO + TAM_ENTRADA);
        assert_eq!(&b[0][2..4], &1u16.to_le_bytes(), "versão 1, ou o cliente descarta tudo");
        assert_eq!(b[4].len(), TAM_DEPOSITO);
        let volta = ListasDeMissao::de_blocos([&b[0], &b[1], &b[2], &b[3], &b[4]], &d);
        assert_eq!(volta, l);
    }
}
