//! B354 — a cerimônia de casamento: a agenda do `wedding_manager` (`gs/weddingmanager.cpp`), que só existe no mapa
//! com `wedding_server = 1` do `gs.conf` (o 134 do 1.5.5; o 1.2.6 não tem).
//!
//! A agenda tem, para cada dia de 3 semanas a partir do domingo da semana anterior (`AddWedding(t_base - 604800,
//! 21)`), uma entrada por sessão do `WEDDING_CONFIG` e por cena (`WEDDING_SCENE_COUNT` = 2, `gs/config.h:166`). A
//! cada 10 s (`RunTick`, 200 ticks) a entrada cujo horário chegou passa a "em andamento" (a reservada vira
//! `ST_ONGOING`), e a que passou do fim vira `ST_FINISH`; sem nada em andamento, depois de uma semana a janela anda
//! uma semana (`DelWedding`/`AddWedding(t_base + 604800, 7)`). Horas pelo fuso do servidor (`localtime`).
//!
//! Gravação: o original grava as entradas reservadas pelo `DBSave` dos dados do servidor; aqui vão ao `config` do
//! realm (`realms.config->'casamento'`), `[início, fim, noivo, noiva, cena, estado]`, e voltam no `DBLoad`
//! ([`AgendaDeCasamento::carregar`]).

use chrono::{Datelike, FixedOffset, TimeZone, Timelike};
use pw_data_loader::casamento::ConfigDeCasamento;
use std::sync::{Mutex, OnceLock};

/// `WEDDING_SCENE_COUNT`.
pub const CENAS: i32 = 2;
/// `WEDDING_BOOK_TICKET_ID`, `WEDDING_APPROVE_TICKET_ID`, `WEDDING_INVITE_TICKET_ID`, `WEDDING_INVITECARD_ID1/2`
/// (`gs/item.h:34-38`).
pub const BILHETE_DE_RESERVA: u32 = 28452;
pub const AUTORIZACAO: u32 = 28454;
pub const BILHETE_DE_CONVITE: u32 = 28400;
pub const CONVITE_DOS_NOIVOS: u32 = 28444;
pub const CONVITE_DO_CONVIDADO: u32 = 28445;
/// `WEDDING_CANCELBOOK_FEE` (`gs/config.h:165`).
pub const TAXA_DE_CANCELAMENTO: i64 = 3_000_000;
const SEMANA: i64 = 604_800;
const DIA: i64 = 86_400;

/// `ST_*` (`weddingmanager.h`).
pub mod estado {
    pub const INVALIDO: u8 = 0;
    pub const LIVRE: u8 = 1;
    pub const RESERVADO: u8 = 2;
    pub const EM_ANDAMENTO: u8 = 3;
    pub const ENCERRADO: u8 = 4;
}

/// `wedding`: uma entrada da agenda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cerimonia {
    pub inicio: i64,
    pub fim: i64,
    pub noivo: i32,
    pub noiva: i32,
    pub cena: i32,
    pub estado: u8,
    /// Dia reservado: só reserva com o cartão daquele dia.
    pub especial: bool,
}

#[derive(Debug, Clone)]
pub struct AgendaDeCasamento {
    sessoes: Vec<[i32; 4]>,
    dias_reservados: Vec<[i32; 3]>,
    fuso: FixedOffset,
    /// `t_base`: o domingo 00:00 desta semana.
    base: i64,
    em_andamento: [Option<usize>; CENAS as usize],
    pub lista: Vec<Cerimonia>,
    /// Mudou desde a última gravação.
    pub suja: bool,
}

impl AgendaDeCasamento {
    /// `Initialize` (sem o `DBLoad`, que é [`Self::carregar`]).
    pub fn nova(cfg: &ConfigDeCasamento, agora: i64, fuso: FixedOffset) -> Self {
        let t = fuso.timestamp_opt(agora, 0).single().expect("instante válido");
        let base = agora
            - DIA * t.weekday().num_days_from_sunday() as i64
            - 3600 * t.hour() as i64
            - 60 * t.minute() as i64
            - t.second() as i64;
        let mut a = Self {
            sessoes: cfg.sessoes.clone(),
            dias_reservados: cfg.dias_reservados.clone(),
            fuso,
            base,
            em_andamento: [None; CENAS as usize],
            lista: Vec::new(),
            suja: false,
        };
        a.acrescentar(base - SEMANA, 21);
        a
    }

    /// `AddWedding(tbase, day_num)`.
    fn acrescentar(&mut self, base: i64, dias: i64) {
        let mut t1 = base;
        for _ in 0..dias {
            let d = self.fuso.timestamp_opt(t1, 0).single().expect("instante válido");
            let especial = self.dias_reservados.iter().any(|x| x[0] == d.year() && x[1] == d.month() as i32 && x[2] == d.day() as i32);
            for p in &self.sessoes {
                for cena in 0..CENAS {
                    self.lista.push(Cerimonia {
                        inicio: t1 + p[0] as i64 * 3600 + p[1] as i64 * 60,
                        fim: t1 + p[2] as i64 * 3600 + p[3] as i64 * 60,
                        noivo: 0,
                        noiva: 0,
                        cena,
                        estado: estado::LIVRE,
                        especial,
                    });
                }
            }
            t1 += DIA;
        }
    }

    /// `DBLoad`: devolve quantas não acharam lugar (gravadas de uma semana que já saiu da agenda).
    pub fn carregar(&mut self, gravadas: &[[i64; 6]]) -> usize {
        let mut perdidas = 0;
        for g in gravadas {
            let (inicio, fim, cena) = (g[0], g[1], g[4] as i32);
            match self.lista.iter_mut().find(|w| w.inicio == inicio && w.fim == fim && w.cena == cena) {
                Some(w) => {
                    w.noivo = g[2] as i32;
                    w.noiva = g[3] as i32;
                    w.estado = g[5] as u8;
                }
                None => perdidas += 1,
            }
        }
        perdidas
    }

    /// `DBSave`: as que não estão livres.
    pub fn gravar(&self) -> Vec<[i64; 6]> {
        self.lista
            .iter()
            .filter(|w| w.estado != estado::LIVRE)
            .map(|w| [w.inicio, w.fim, w.noivo as i64, w.noiva as i64, w.cena as i64, w.estado as i64])
            .collect()
    }

    /// `RunTick` (a cada 10 s).
    pub fn batimento(&mut self, agora: i64) {
        if self.em_andamento.iter().all(|x| x.is_none()) {
            let mut alguma = false;
            for (i, w) in self.lista.iter_mut().enumerate() {
                if agora >= w.inicio && agora < w.fim {
                    alguma = true;
                    self.em_andamento[w.cena as usize] = Some(i);
                    if w.estado == estado::RESERVADO {
                        w.estado = estado::EM_ANDAMENTO;
                        self.suja = true;
                    }
                }
            }
            if !alguma && agora - self.base > SEMANA {
                self.base += SEMANA;
                self.em_andamento = [None; CENAS as usize];
                let corte = self.base - SEMANA;
                self.lista.retain(|w| w.inicio >= corte);
                self.acrescentar(self.base + SEMANA, 7);
                self.suja = true;
            }
        } else {
            for c in 0..CENAS as usize {
                let Some(i) = self.em_andamento[c] else { continue };
                let w = &mut self.lista[i];
                if agora >= w.fim {
                    if w.estado == estado::EM_ANDAMENTO {
                        w.estado = estado::ENCERRADO;
                        self.suja = true;
                    }
                    self.em_andamento[c] = None;
                }
            }
        }
    }

    /// `CheckOngoing(groom, bride, scene)`.
    pub fn em_andamento(&self, noivo: i32, noiva: i32, cena: i32) -> bool {
        if !(0..CENAS).contains(&cena) {
            return false;
        }
        self.em_andamento[cena as usize]
            .map(|i| self.lista[i])
            .is_some_and(|w| w.estado == estado::EM_ANDAMENTO && w.noivo == noivo && w.noiva == noiva)
    }

    /// `CheckOngoing(id)`.
    pub fn em_andamento_de(&self, id: i32) -> bool {
        self.em_andamento
            .iter()
            .flatten()
            .map(|i| self.lista[*i])
            .any(|w| w.estado == estado::EM_ANDAMENTO && (w.noivo == id || w.noiva == id))
    }

    /// `CheckBook`: horário futuro, ninguém reservado naquele horário (uma cerimônia por horário, em qualquer cena),
    /// e no dia reservado só com o cartão daquele dia.
    pub fn pode_reservar(&self, inicio: i64, fim: i64, cena: i32, cartao: [i32; 3], agora: i64) -> bool {
        if inicio <= agora {
            return false;
        }
        let mut achou = false;
        for w in self.lista.iter().filter(|w| w.inicio == inicio && w.fim == fim) {
            if w.estado != estado::LIVRE {
                return false;
            }
            if w.cena == cena {
                if w.especial {
                    let d = self.fuso.timestamp_opt(w.inicio, 0).single().expect("instante válido");
                    if [d.year(), d.month() as i32, d.day() as i32] != cartao {
                        return false;
                    }
                }
                achou = true;
            }
        }
        achou
    }

    /// `TryBook`.
    pub fn reservar(&mut self, inicio: i64, fim: i64, noivo: i32, noiva: i32, cena: i32, agora: i64) -> bool {
        if inicio <= agora {
            return false;
        }
        let Some(w) = self.lista.iter_mut().find(|w| w.inicio == inicio && w.fim == fim && w.cena == cena) else {
            return false;
        };
        if w.estado != estado::LIVRE {
            return false;
        }
        (w.estado, w.noivo, w.noiva) = (estado::RESERVADO, noivo, noiva);
        self.suja = true;
        true
    }

    /// `CheckCancelBook`.
    pub fn pode_cancelar(&self, inicio: i64, fim: i64, noivo: i32, noiva: i32, cena: i32) -> bool {
        self.lista
            .iter()
            .find(|w| w.inicio == inicio && w.fim == fim && w.noivo == noivo && w.noiva == noiva && w.cena == cena)
            .is_some_and(|w| w.estado == estado::RESERVADO)
    }

    /// `TryCancelBook`.
    pub fn cancelar(&mut self, inicio: i64, fim: i64, noivo: i32, noiva: i32, cena: i32) -> bool {
        let Some(w) = self
            .lista
            .iter_mut()
            .find(|w| w.inicio == inicio && w.fim == fim && w.noivo == noivo && w.noiva == noiva && w.cena == cena)
        else {
            return false;
        };
        if w.estado != estado::RESERVADO {
            return false;
        }
        (w.estado, w.noivo, w.noiva) = (estado::LIVRE, 0, 0);
        self.suja = true;
        true
    }
}

/// A agenda do processo (o `_wedding_man` do `global_world_manager` do mapa da cerimônia); `None` fora dele.
pub fn agenda() -> &'static Mutex<Option<AgendaDeCasamento>> {
    static A: OnceLock<Mutex<Option<AgendaDeCasamento>>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(None))
}

/// `wedding_filter`: quem entrou na cena pelo convite, com `(noivo, noiva, cena)`. Sai quando a cerimônia acaba
/// (o batimento do roteador), e não é gravado (`FILTER_MASK_NOSAVE`).
pub fn na_cena() -> &'static Mutex<std::collections::HashMap<i32, (i32, i32, i32)>> {
    static N: OnceLock<Mutex<std::collections::HashMap<i32, (i32, i32, i32)>>> = OnceLock::new();
    N.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// O conteúdo do convite (`wedding_invitecard_essence`, 6 `int`: início, fim, noivo, noiva, cena, convidado).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Convite {
    pub inicio: i32,
    pub fim: i32,
    pub noivo: i32,
    pub noiva: i32,
    pub cena: i32,
    pub convidado: i32,
}

impl Convite {
    pub fn ler(b: &[u8]) -> Option<Self> {
        if b.len() < 24 {
            return None;
        }
        let i = |k: usize| i32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap());
        Some(Self { inicio: i(0), fim: i(1), noivo: i(2), noiva: i(3), cena: i(4), convidado: i(5) })
    }

    pub fn octetos(&self) -> Vec<u8> {
        [self.inicio, self.fim, self.noivo, self.noiva, self.cena, self.convidado].iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigDeCasamento {
        ConfigDeCasamento {
            sessoes: vec![[14, 0, 15, 0], [19, 30, 20, 30]],
            dias_reservados: vec![[2026, 10, 27]],
            ..Default::default()
        }
    }

    /// Quinta-feira 2026-10-22 12:00 UTC; o domingo da semana é 2026-10-18.
    const AGORA: i64 = 1_792_670_400;

    #[test]
    fn tres_semanas_de_agenda_a_partir_do_domingo_anterior() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let a = AgendaDeCasamento::nova(&cfg(), AGORA, utc);
        assert_eq!(a.lista.len(), 21 * 2 * 2, "21 dias × 2 sessões × 2 cenas");
        let domingo = utc.timestamp_opt(a.base, 0).unwrap();
        assert_eq!((domingo.weekday().num_days_from_sunday(), domingo.day(), domingo.hour()), (0, 18, 0));
        assert_eq!(a.lista[0].inicio, a.base - SEMANA + 14 * 3600);
        assert_eq!(a.lista.iter().filter(|w| w.especial).count(), 4, "o 27/10 reservado");
    }

    #[test]
    fn reservar_andar_e_encerrar() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let mut a = AgendaDeCasamento::nova(&cfg(), AGORA, utc);
        // Hoje às 14:00 (já passou de 12:00? não — 14 > 12): livre.
        let inicio = a.base + 4 * DIA + 14 * 3600;
        let fim = inicio + 3600;
        assert!(a.pode_reservar(inicio, fim, 1, [0, 0, 0], AGORA));
        assert!(!a.pode_reservar(inicio - DIA, fim - DIA, 1, [0, 0, 0], AGORA), "ontem já passou");
        assert!(a.reservar(inicio, fim, 10, 11, 1, AGORA));
        assert!(!a.pode_reservar(inicio, fim, 0, [0, 0, 0], AGORA), "um só por horário, mesmo noutra cena");
        // Dia reservado: só com o cartão do dia.
        let especial = a.base + 9 * DIA + 14 * 3600;
        assert!(!a.pode_reservar(especial, especial + 3600, 0, [0, 0, 0], AGORA));
        assert!(a.pode_reservar(especial, especial + 3600, 0, [2026, 10, 27], AGORA));
        // A hora chega: em andamento; passa: encerrada.
        a.batimento(inicio + 10);
        assert!(a.em_andamento(10, 11, 1) && a.em_andamento_de(11) && !a.em_andamento(10, 11, 0));
        a.batimento(fim);
        assert!(!a.em_andamento_de(10));
        assert_eq!(a.gravar(), vec![[inicio, fim, 10, 11, 1, estado::ENCERRADO as i64]]);
        // Gravar e carregar numa agenda nova.
        let mut b = AgendaDeCasamento::nova(&cfg(), AGORA, utc);
        assert_eq!(b.carregar(&a.gravar()), 0);
        assert_eq!(b.gravar(), a.gravar());
    }

    #[test]
    fn cancelar_so_o_reservado_e_a_semana_anda() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let mut a = AgendaDeCasamento::nova(&cfg(), AGORA, utc);
        let inicio = a.base + 5 * DIA + 19 * 3600 + 1800;
        assert!(a.reservar(inicio, inicio + 3600, 1, 2, 0, AGORA));
        assert!(!a.pode_cancelar(inicio, inicio + 3600, 2, 1, 0), "noivo e noiva na ordem");
        assert!(a.cancelar(inicio, inicio + 3600, 1, 2, 0));
        assert!(a.gravar().is_empty());
        let base = a.base;
        a.batimento(base + SEMANA + 3600);
        assert_eq!(a.base, base + SEMANA);
        assert_eq!(a.lista.len(), 21 * 2 * 2, "tira a semana mais antiga e põe uma nova");
        assert_eq!(a.lista.first().unwrap().inicio, base + 14 * 3600);
    }

    #[test]
    fn conteudo_do_convite() {
        let c = Convite { inicio: 1, fim: 2, noivo: 3, noiva: 4, cena: 1, convidado: 3 };
        assert_eq!(c.octetos().len(), 24);
        assert_eq!(Convite::ler(&c.octetos()), Some(c));
    }
}
