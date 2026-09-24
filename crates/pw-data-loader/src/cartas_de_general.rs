//! Cartas de General: a caixa que sorteia a carta (`POKER_DICE_ESSENCE`) e a carta
//! (`POKER_ESSENCE`, com o tipo do `POKER_SUB_TYPE`).
//!
//! Servidor original:
//! - abrir a caixa: `generalcard_dice_item::OnUse` (`gs/item/item_generalcard_dice.cpp:10-54`)
//!   — bolsa cheia recusa; `RandSelect` sobre as 256 entradas `list[].probability`; a carta
//!   sorteada tem de ser um `POKER_ESSENCE`; gera a carta e a põe no primeiro slot vazio;
//! - a carta gerada: `generate_poker` (`gs/template/generate_item_temp.h:3144-3191`), cujo
//!   conteúdo é o `generalcard_essence` (`gs/item/item_generalcard.h:9-19`) — oito `int`.
//!
//! O resto do sistema (equipar a carta, liderança, atributos, subir de nível, devorar) **não**
//! está portado.
use crate::generic_elements::{FieldValue, GenericElementsData};
use std::collections::HashMap;

/// Entradas de `POKER_DICE_ESSENCE::list` (`gs/template/exptypes.h:629-633`).
const ENTRADAS_DA_CAIXA: usize = 256;

/// Uma caixa: (id da carta, probabilidade), na ordem do arquivo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CaixaDeCartas {
    pub cartas: Vec<(u32, f32)>,
}

impl CaixaDeCartas {
    /// `abase::RandSelect(&list[0].probability, sizeof(list[0]), 256)` com o número `sorteio`
    /// em `[0, 1)` (`itemdataman.h:33-47`): a primeira entrada cuja probabilidade acumulada
    /// passa do número; sem nenhuma, a entrada 0 (o `ASSERT(false); return 0` do original).
    pub fn sortear(&self, sorteio: f32) -> Option<u32> {
        let mut op = sorteio;
        for (id, p) in &self.cartas {
            if op < *p {
                return (*id > 0).then_some(*id);
            }
            op -= p;
        }
        self.cartas.first().map(|c| c.0).filter(|id| *id > 0)
    }
}

/// O que o `generate_poker` tira do `elements.data` para escrever a carta.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CartaDeGeneral {
    /// `POKER_SUB_TYPE::type` do `id_sub_type` — o slot da carta.
    pub tipo: i32,
    /// `rank` — qualidade (C, B, A, S, S+).
    pub qualidade: i32,
    pub nivel_exigido: i32,
    /// `require_control_point[0..2]` — a faixa da liderança exigida, sorteada na geração.
    pub lideranca: (i32, i32),
    pub nivel_maximo: i32,
}

impl CartaDeGeneral {
    /// O `generalcard_essence` de uma carta nova (`generate_item_temp.h:3180-3188`): tipo,
    /// qualidade, nível exigido, a liderança sorteada, nível máximo, **nível 1**, experiência
    /// 0 e renascimentos 0 — 32 bytes.
    pub fn octetos(&self, lideranca: i32) -> Vec<u8> {
        [self.tipo, self.qualidade, self.nivel_exigido, lideranca, self.nivel_maximo, 1, 0, 0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CartasDeGeneral {
    pub caixas: HashMap<u32, CaixaDeCartas>,
    pub cartas: HashMap<u32, CartaDeGeneral>,
}

pub fn carregar(elements: &GenericElementsData) -> CartasDeGeneral {
    let i = |r: &HashMap<String, FieldValue>, n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
    let f = |r: &HashMap<String, FieldValue>, n: &str| match r.get(n) {
        Some(FieldValue::Float(v)) => *v,
        _ => 0.0,
    };
    let tipos: HashMap<i32, i32> = elements.get("POKER_SUB_TYPE").iter().map(|r| (i(r, "ID"), i(r, "type"))).collect();
    let caixas = elements
        .get("POKER_DICE_ESSENCE")
        .iter()
        .filter(|r| i(r, "ID") > 0)
        .map(|r| {
            let cartas = (1..=ENTRADAS_DA_CAIXA)
                .map(|k| (i(r, &format!("list_{k}_id")).max(0) as u32, f(r, &format!("list_{k}_probability"))))
                .collect();
            (i(r, "ID") as u32, CaixaDeCartas { cartas })
        })
        .collect();
    let cartas = elements
        .get("POKER_ESSENCE")
        .iter()
        .filter(|r| i(r, "ID") > 0)
        .filter_map(|r| {
            // `generate_poker` recusa a carta cujo subtipo não existe (`:3148-3149`).
            let tipo = *tipos.get(&i(r, "id_sub_type"))?;
            Some((
                i(r, "ID") as u32,
                CartaDeGeneral {
                    tipo,
                    qualidade: i(r, "rank"),
                    nivel_exigido: i(r, "require_level"),
                    lideranca: (i(r, "require_control_point_1"), i(r, "require_control_point_2")),
                    nivel_maximo: i(r, "max_level"),
                },
            ))
        })
        .collect();
    CartasDeGeneral { caixas, cartas }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_sorteio_acumula_as_probabilidades_como_o_rand_select() {
        let c = CaixaDeCartas { cartas: vec![(10, 0.25), (20, 0.5), (30, 0.25)] };
        assert_eq!(c.sortear(0.0), Some(10));
        assert_eq!(c.sortear(0.3), Some(20));
        assert_eq!(c.sortear(0.8), Some(30));
        // Soma que não passa do número: a entrada 0, como o original.
        assert_eq!(c.sortear(1.5), Some(10));
    }

    #[test]
    fn a_carta_nova_tem_oito_int_com_nivel_1() {
        let c = CartaDeGeneral { tipo: 2, qualidade: 1, nivel_exigido: 10, lideranca: (5, 9), nivel_maximo: 50 };
        let o = c.octetos(7);
        assert_eq!(o.len(), 32);
        let v: Vec<i32> = o.chunks(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
        assert_eq!(v, vec![2, 1, 10, 7, 50, 1, 0, 0]);
    }
}
