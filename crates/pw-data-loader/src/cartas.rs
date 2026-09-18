//! `TASKDICE_ESSENCE` — a "Carta da Sorte": usar entrega uma missão sorteada da lista.
//!
//! Servidor: `item_taskdice::OnUse` (`gs/item/item_taskdice.cpp:12-42`) e
//! `generate_taskdice` (`gs/template/generate_item_temp.h:1471-1500`). O sorteio é
//! `RandSelect` sobre as probabilidades (`itemdataman.h:33-47`): um número uniforme em
//! `[0, 1)` e a primeira entrada cuja probabilidade acumulada passa dele.
use crate::generic_elements::{FieldValue, GenericElementsData};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct CartaDeMissao {
    /// `task_lists[20]`: (id da missão, probabilidade).
    pub missoes: Vec<(u32, f32)>,
    /// `no_use_in_combat`.
    pub nao_usa_em_combate: bool,
}

impl CartaDeMissao {
    /// `RandSelect(&task_lists[0].probability, sizeof(int)+sizeof(float), 20)` com o número
    /// `sorteio` em `[0, 1)`. Como o original, sem acerto: devolve a entrada 0 se a soma
    /// não passar do número (`ASSERT(false); return 0`).
    pub fn sortear(&self, sorteio: f32) -> Option<u32> {
        let mut op = sorteio;
        for (id, p) in &self.missoes {
            if op < *p {
                return Some(*id).filter(|i| *i != 0);
            }
            op -= p;
        }
        self.missoes.first().map(|m| m.0).filter(|i| *i != 0)
    }
}

pub type TabelaDeCartas = HashMap<u32, CartaDeMissao>;

pub fn carregar(elements: &GenericElementsData) -> TabelaDeCartas {
    elements
        .get("TASKDICE_ESSENCE")
        .iter()
        .filter_map(|r| {
            let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            let f = |n: &str| match r.get(n) {
                Some(FieldValue::Float(v)) => *v,
                _ => 0.0,
            };
            let id = i("ID");
            (id > 0).then(|| {
                let missoes = (1..=20)
                    .map(|k| (i(&format!("task_lists_{k}_id")).max(0) as u32, f(&format!("task_lists_{k}_probability"))))
                    .collect();
                (id as u32, CartaDeMissao { missoes, nao_usa_em_combate: i("no_use_in_combat") != 0 })
            })
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_sorteio_anda_pelas_probabilidades_acumuladas() {
        let c = CartaDeMissao { missoes: vec![(10, 0.25), (20, 0.5), (30, 0.25), (0, 0.0)], nao_usa_em_combate: false };
        assert_eq!(c.sortear(0.0), Some(10));
        assert_eq!(c.sortear(0.24), Some(10));
        assert_eq!(c.sortear(0.25), Some(20));
        assert_eq!(c.sortear(0.74), Some(20));
        assert_eq!(c.sortear(0.99), Some(30));
    }
}
