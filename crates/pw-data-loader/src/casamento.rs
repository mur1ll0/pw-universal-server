//! B354 — a cerimônia de casamento no `elements.data`: o `WEDDING_CONFIG` de id 801 (`WEDDING_CONFIG_ID`,
//! `gs/config.h:164`) e os cartões (`WEDDING_BOOKCARD_ESSENCE`, `WEDDING_INVITECARD_ESSENCE`).
//!
//! `wedding_manager::Initialize` (`gs/weddingmanager.cpp:8-60`) lê as sessões do dia até a primeira com
//! `start_hour <= 0`, recusa sessão inválida ou que comece antes do fim da anterior, e os dias reservados até o
//! primeiro com `year <= 0`; sem nenhuma sessão, não há cerimônia.

use crate::generic_elements::{FieldValue, GenericElementsData};
use std::collections::HashMap;

/// `WEDDING_CONFIG_ID`.
pub const ID_DA_CONFIGURACAO: i32 = 801;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigDeCasamento {
    /// `schedule_periods`: `[início h, início min, fim h, fim min]`.
    pub sessoes: Vec<[i32; 4]>,
    /// `special_days`: `[ano, mês, dia]`.
    pub dias_reservados: Vec<[i32; 3]>,
    /// `wedding_scene[i].pos`: onde o convite leva (dez no arquivo; o original só usa `WEDDING_SCENE_COUNT`).
    pub cenas: Vec<[f32; 3]>,
    /// `WEDDING_BOOKCARD_ESSENCE`: o dia de cada cartão de reserva (`generate_wedding_bookcard`).
    pub cartoes_de_reserva: HashMap<u32, [i32; 3]>,
    /// Os ids de `WEDDING_INVITECARD_ESSENCE` (o convite que se usa).
    pub convites: Vec<u32>,
}

/// `PERIOD::Valid` (`weddingmanager.h`).
fn sessao_valida(p: &[i32; 4]) -> bool {
    (0..=23).contains(&p[0])
        && (0..=59).contains(&p[1])
        && (0..=23).contains(&p[2])
        && (0..=59).contains(&p[3])
        && (p[0] < p[2] || (p[0] == p[2] && p[1] < p[3]))
}

/// `DAY::Valid`: o fevereiro aceita 29 sempre.
fn dia_valido(d: &[i32; 3]) -> bool {
    const DIAS: [i32; 13] = [0, 31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    d[0] >= 0 && (1..=12).contains(&d[1]) && d[2] >= 1 && d[2] <= DIAS[d[1] as usize]
}

/// `None` quando o registro 801 falta ou o `Initialize` o recusaria.
pub fn carregar(g: &GenericElementsData) -> Option<ConfigDeCasamento> {
    let i = |r: &HashMap<String, FieldValue>, n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
    let f = |r: &HashMap<String, FieldValue>, n: &str| match r.get(n) {
        Some(FieldValue::Float(v)) => *v,
        _ => 0.0,
    };
    let cartoes_de_reserva = g
        .get("WEDDING_BOOKCARD_ESSENCE")
        .iter()
        .map(|r| (i(r, "ID") as u32, [i(r, "year"), i(r, "month"), i(r, "day")]))
        .collect();
    let convites = g.get("WEDDING_INVITECARD_ESSENCE").iter().map(|r| i(r, "ID") as u32).collect();
    let tabela = g.get("WEDDING_CONFIG");
    let r = tabela.iter().find(|r| i(r, "ID") == ID_DA_CONFIGURACAO)?;
    let mut sessoes = Vec::new();
    let mut fim_anterior = 0;
    for k in 1..=10 {
        let p = [
            i(r, &format!("wedding_session_{k}_start_hour")),
            i(r, &format!("wedding_session_{k}_start_min")),
            i(r, &format!("wedding_session_{k}_end_hour")),
            i(r, &format!("wedding_session_{k}_end_min")),
        ];
        if p[0] <= 0 {
            break;
        }
        if !sessao_valida(&p) || p[0] * 3600 + p[1] * 60 < fim_anterior {
            return None;
        }
        fim_anterior = p[2] * 3600 + p[3] * 60;
        sessoes.push(p);
    }
    if sessoes.is_empty() {
        return None;
    }
    let mut dias_reservados = Vec::new();
    for k in 1..=20 {
        let d = [
            i(r, &format!("reserved_day_{k}_year")),
            i(r, &format!("reserved_day_{k}_month")),
            i(r, &format!("reserved_day_{k}_day")),
        ];
        if d[0] <= 0 {
            break;
        }
        if !dia_valido(&d) {
            return None;
        }
        dias_reservados.push(d);
    }
    let cenas = (1..=10)
        .map(|k| [1, 2, 3].map(|c| f(r, &format!("wedding_scene_{k}_pos_{c}"))))
        .collect();
    Some(ConfigDeCasamento { sessoes, dias_reservados, cenas, cartoes_de_reserva, convites })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn validacao_das_sessoes_e_dos_dias() {
        assert!(sessao_valida(&[19, 0, 20, 0]));
        assert!(!sessao_valida(&[20, 0, 20, 0]), "fim igual ao início");
        assert!(!sessao_valida(&[24, 0, 24, 30]));
        assert!(dia_valido(&[2026, 2, 29]), "o fevereiro do original aceita 29");
        assert!(!dia_valido(&[2026, 4, 31]));
    }
}
