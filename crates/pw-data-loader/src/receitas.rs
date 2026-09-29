//! As receitas de produção (`RECIPE_ESSENCE`) e o que cada NPC produz (`NPC_MAKE_SERVICE`).
//!
//! # Origem
//!
//! `recipe_manager::LoadTemplate` monta um `recipe_template` por `RECIPE_ESSENCE`
//! (`cgame/gs/npcgenerator.cpp:1387-1470`, 1.5.5): `price` vira a taxa (`fee`), `id_skill` e
//! `skill_level` a habilidade de produção e o nível exigido, `num_to_make` a quantidade,
//! `fail_probability` a chance de não sair nada (`null_prob`), `duration` o tempo em
//! **tiques de 50 ms** (`(int)(20 × duration)`, preso a 1..10000), `exp`/`skillpoint` o que se
//! ganha, e os 32 materiais `{id, num}` (os de `num` zero ficam de fora quando `bind_type` é 0).
//! O `level` é sempre `MAX_PLAYER_LEVEL` (150).
//!
//! O `gs` 1.2.6 faz a mesma conta (`recipe_manager::LoadTemplate`, VA 0x80f0a12), com o
//! `RECIPE_ESSENCE` v7 **sem `bind_type`** (os alvos começam em +0x58) e sem o rabo de
//! melhoria de equipamento do v156 — ver `specs/elements_layouts/generate_v7.py`.
//!
//! O NPC produz as receitas das oito páginas de 32 do `NPC_MAKE_SERVICE`, com a habilidade
//! `id_make_skill` (`npcgenerator.cpp:628-665`; `produce_provider::TryServe`,
//! `serviceprovider.cpp:1499-1518`, recusa receita fora da lista ou de outra habilidade).

use crate::generic_elements::{FieldValue, GenericElementsData, Record};
use std::collections::HashMap;

/// `MAX_PLAYER_LEVEL` — o `level` de toda receita.
pub const NIVEL_DA_RECEITA: i32 = 150;

/// Um `recipe_template`.
#[derive(Debug, Clone, PartialEq)]
pub struct Receita {
    pub id: u32,
    /// `produce_skill` (`id_skill`); 0 = sem habilidade.
    pub habilidade: i32,
    /// `require_level` (`skill_level`): o nível mínimo da habilidade.
    pub nivel_exigido: i32,
    /// `recipe_level`: define quanto a proficiência sobe (`IncSkillAbility`).
    pub nivel_da_receita: i32,
    pub exp: i32,
    pub sp: i32,
    /// `null_prob` (`fail_probability`).
    pub chance_de_nada: f32,
    /// `use_time`, em tiques de 50 ms.
    pub tempo_em_tiques: u32,
    /// `count` (`num_to_make`).
    pub quantidade: u32,
    /// `fee` (`price`).
    pub taxa: i64,
    /// `bind_type` (0 no 1.2.6, que não tem o campo).
    pub vinculo: i32,
    /// `proc_type` que o item produzido herda quando `bind_type` é 0.
    pub proc_type: i32,
    /// `equipment_need_upgrade` (`id_upgrade_equip`): receita de melhoria, que o serviço comum
    /// recusa (`produce_executor::SendRequest`, `serviceprovider.cpp:1571`).
    pub melhoria: i32,
    /// `targets[4]`: `(id, probabilidade)`.
    pub alvos: [(i32, f32); 4],
    /// `material_list`: `(id, quantidade)`, os de quantidade zero fora.
    pub materiais: Vec<(u32, u32)>,
}

/// O serviço de produção de um NPC.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServicoDeProducao {
    /// `id_make_skill` — a habilidade que o pedido tem de trazer.
    pub habilidade: i32,
    /// As receitas das páginas, na ordem do arquivo.
    pub receitas: Vec<u32>,
    /// `produce_type` (0 no 1.2.6, que não tem o campo).
    pub tipo: i32,
}

fn i(r: &Record, campo: &str) -> i32 {
    r.get(campo).and_then(|v| v.as_i32()).unwrap_or(0)
}

fn f(r: &Record, campo: &str) -> f32 {
    match r.get(campo) {
        Some(FieldValue::Float(v)) => *v,
        Some(FieldValue::Int(v)) => *v as f32,
        _ => 0.0,
    }
}

/// `recipe_manager::LoadTemplate` sobre todo `RECIPE_ESSENCE`.
pub fn carregar_receitas(g: &GenericElementsData) -> HashMap<u32, Receita> {
    g.get("RECIPE_ESSENCE")
        .iter()
        .filter_map(|r| {
            let id = i(r, "ID");
            if id <= 0 {
                return None;
            }
            let vinculo = i(r, "bind_type");
            let tempo = ((20.0 * f(r, "duration")) as i32).clamp(1, 10_000) as u32;
            let materiais = (1..=32)
                .map(|k| (i(r, &format!("materials_{k}_id")), i(r, &format!("materials_{k}_num"))))
                .filter(|&(m, n)| m > 0 && n > 0)
                .map(|(m, n)| (m as u32, n as u32))
                .collect();
            let alvo = |k: usize| (i(r, &format!("targets_{k}_id_to_make")), f(r, &format!("targets_{k}_probability")));
            Some((
                id as u32,
                Receita {
                    id: id as u32,
                    habilidade: i(r, "id_skill"),
                    nivel_exigido: i(r, "skill_level"),
                    nivel_da_receita: i(r, "recipe_level"),
                    exp: i(r, "exp"),
                    sp: i(r, "skillpoint"),
                    chance_de_nada: f(r, "fail_probability"),
                    tempo_em_tiques: tempo,
                    quantidade: i(r, "num_to_make").max(0) as u32,
                    taxa: i(r, "price").max(0) as i64,
                    vinculo,
                    proc_type: i(r, "proc_type"),
                    melhoria: i(r, "id_upgrade_equip"),
                    alvos: [alvo(1), alvo(2), alvo(3), alvo(4)],
                    materiais,
                },
            ))
        })
        .collect()
}

/// Os `NPC_MAKE_SERVICE` por id.
pub fn carregar_servicos_de_producao(g: &GenericElementsData) -> HashMap<i32, ServicoDeProducao> {
    g.get("NPC_MAKE_SERVICE")
        .iter()
        .map(|r| {
            let receitas = (1..=8)
                .flat_map(|p| (1..=32).map(move |k| (p, k)))
                .map(|(p, k)| i(r, &format!("pages_{p}_id_goods_{k}")))
                .filter(|&id| id > 0)
                .map(|id| id as u32)
                .collect();
            (
                i(r, "ID"),
                ServicoDeProducao { habilidade: i(r, "id_make_skill"), receitas, tipo: i(r, "produce_type") },
            )
        })
        .collect()
}

/// `abase::RandSelect` sobre as probabilidades dos alvos: o índice cuja faixa acumulada
/// contém `sorteio` (em 0..1). Probabilidades que não somam 1 caem no último alvo.
pub fn escolher_alvo(alvos: &[(i32, f32); 4], sorteio: f32) -> i32 {
    let mut acumulado = 0.0;
    for &(id, p) in alvos {
        acumulado += p;
        if sorteio < acumulado {
            return id;
        }
    }
    alvos.iter().rev().find(|(id, _)| *id > 0).map(|(id, _)| *id).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_alvo_sai_pela_faixa_acumulada() {
        let alvos = [(10, 0.5), (20, 0.3), (30, 0.2), (0, 0.0)];
        assert_eq!(escolher_alvo(&alvos, 0.0), 10);
        assert_eq!(escolher_alvo(&alvos, 0.49), 10);
        assert_eq!(escolher_alvo(&alvos, 0.5), 20);
        assert_eq!(escolher_alvo(&alvos, 0.95), 30);
    }
}
