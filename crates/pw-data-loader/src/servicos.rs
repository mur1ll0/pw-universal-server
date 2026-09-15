//! Os serviços de cada NPC — que missões ele entrega e recebe, que habilidades ensina — e o
//! limite de pilha de cada item.
//!
//! # Origem
//!
//! `NPC_ESSENCE` aponta, por id, para um registro de cada serviço (`id_task_out_service`,
//! `id_task_in_service`, `id_skill_service`). O servidor original monta um provedor por
//! serviço e só atende o que está na lista dele: `task_out_provider::TryServe`,
//! `task_in_provider::TryServe` e `skill_provider::TryServe`
//! (`cgame/gs/serviceprovider.cpp:1012-1033`, `:1088-1116`, `:1244-1263`) fazem
//! `binary_search` do id pedido e respondem `ERR_TASK_NOT_AVAILABLE` / `ERR_SKILL_NOT_AVAILABLE`
//! quando ele não está lá.
//!
//! O limite de pilha é o `pile_num_max` que toda família de item declara com o mesmo nome;
//! `item_list::Push` do servidor e `CECInventory::MergeItem` do cliente
//! (`EC_Inventory.cpp:179-215`) empilham até ele.

use crate::generic_elements::{GenericElementsData, Record};
use std::collections::HashMap;

/// O que um NPC oferece, pelo id do `NPC_ESSENCE`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServicosDoNpc {
    /// `NPC_TASK_OUT_SERVICE.id_tasks[256]`, ordenados.
    pub missoes_entregues: Vec<u32>,
    /// `NPC_TASK_OUT_SERVICE.storage_id` — o depósito de missões (0 = lista comum).
    pub deposito: u32,
    /// `NPC_TASK_IN_SERVICE.id_tasks[256]`, ordenados.
    pub missoes_recebidas: Vec<u32>,
    /// `NPC_SKILL_SERVICE.id_skills[256]`, ordenados.
    pub habilidades: Vec<u32>,
}

fn i(r: &Record, campo: &str) -> i32 {
    r.get(campo).and_then(|v| v.as_i32()).unwrap_or(0)
}

fn por_id(g: &GenericElementsData, tabela: &str) -> HashMap<i32, Record> {
    g.get(tabela).iter().map(|r| (i(r, "ID"), r.clone())).collect()
}

fn lista(r: Option<&Record>, prefixo: &str) -> Vec<u32> {
    let Some(r) = r else { return Vec::new() };
    let mut v: Vec<u32> = (1..=256)
        .map(|k| i(r, &format!("{prefixo}{k}")))
        .filter(|&id| id > 0)
        .map(|id| id as u32)
        .collect();
    // `general_id_provider::OnInit` ordena a lista para o `binary_search`.
    v.sort_unstable();
    v.dedup();
    v
}

/// Monta os serviços de todos os NPCs.
pub fn carregar(g: &GenericElementsData) -> HashMap<u32, ServicosDoNpc> {
    let saidas = por_id(g, "NPC_TASK_OUT_SERVICE");
    let entradas = por_id(g, "NPC_TASK_IN_SERVICE");
    let ensinos = por_id(g, "NPC_SKILL_SERVICE");
    g.get("NPC_ESSENCE")
        .iter()
        .filter_map(|npc| {
            let id = i(npc, "ID");
            if id <= 0 {
                return None;
            }
            let saida = saidas.get(&i(npc, "id_task_out_service"));
            let s = ServicosDoNpc {
                missoes_entregues: lista(saida, "id_tasks_"),
                deposito: saida.map(|r| i(r, "storage_id").max(0) as u32).unwrap_or(0),
                missoes_recebidas: lista(entradas.get(&i(npc, "id_task_in_service")), "id_tasks_"),
                habilidades: lista(ensinos.get(&i(npc, "id_skill_service")), "id_skills_"),
            };
            Some((id as u32, s))
        })
        .collect()
}

/// `pile_num_max` de todo item que o declara.
pub fn pilhas(g: &GenericElementsData) -> HashMap<u32, u32> {
    let mut t = HashMap::new();
    for registros in g.tables.values() {
        let Some(primeiro) = registros.first() else { continue };
        if !primeiro.contains_key("pile_num_max") || !primeiro.contains_key("ID") {
            continue;
        }
        for r in registros {
            let id = i(r, "ID");
            if id > 0 {
                t.entry(id as u32).or_insert(i(r, "pile_num_max").max(1) as u32);
            }
        }
    }
    t
}
