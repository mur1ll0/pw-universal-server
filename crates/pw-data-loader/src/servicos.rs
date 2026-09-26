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
    /// `NPC_TRANSMIT_SERVICE.targets[32]` — os destinos da transportadora, **na ordem do
    /// arquivo**: é por índice que o cliente pede o teleporte
    /// (`transmit_provider::TryServe`, `gs/serviceprovider.cpp:771-790`).
    pub destinos: Vec<DestinoDeTeleporte>,
    /// `NPC_PETNAME_SERVICE` `(price, id_object_need)` — renomear mascote, serviço 36
    /// (`npcgenerator.cpp:783-795`, `change_pet_name_provider`).
    pub renomear_mascote: Option<(i32, i32)>,
    /// `NPC_PETFORGETSKILL_SERVICE` `(price, id_object_need)` — esquecer habilidade de
    /// mascote, serviço 37 (`npcgenerator.cpp:815-826`).
    pub esquecer_habilidade_de_mascote: Option<(i32, i32)>,
    /// `NPC_PETLEARNSKILL_SERVICE.id_skills[128]`, ordenados — serviço 38
    /// (`npcgenerator.cpp:797-813`, `pet_skill_provider` com `binary_search`).
    pub habilidades_de_mascote: Vec<u32>,
}

/// Um destino de transportadora (`NPC_TRANSMIT_SERVICE.targets_N_*`).
///
/// A coordenada **não** está aqui: o `id_ponto` remete ao `world_targets.sev`
/// (`crate::world_targets`), que é de onde o original monta o `transmit_entry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestinoDeTeleporte {
    pub id_ponto: i32,
    /// `fee` — o preço em moedas.
    pub preco: i32,
    /// `required_level` — nível mínimo do jogador.
    pub nivel: i32,
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
    let transportes = por_id(g, "NPC_TRANSMIT_SERVICE");
    let nomes_de_mascote = por_id(g, "NPC_PETNAME_SERVICE");
    let esquecimentos = por_id(g, "NPC_PETFORGETSKILL_SERVICE");
    let ensinos_de_mascote = por_id(g, "NPC_PETLEARNSKILL_SERVICE");
    let preco_e_item = |r: Option<&Record>| r.map(|r| (i(r, "price").max(0), i(r, "id_object_need").max(0)));
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
                destinos: destinos(transportes.get(&i(npc, "id_transmit_service"))),
                renomear_mascote: preco_e_item(nomes_de_mascote.get(&i(npc, "id_petname_service"))),
                esquecer_habilidade_de_mascote: preco_e_item(esquecimentos.get(&i(npc, "id_petforgetskill_service"))),
                habilidades_de_mascote: lista(ensinos_de_mascote.get(&i(npc, "id_petlearnskill_service")), "id_skills_"),
            };
            Some((id as u32, s))
        })
        .collect()
}

/// Os destinos de uma transportadora, na ordem do arquivo (1..32), parando no primeiro
/// vazio — `num_targets` diz quantos são, e o original só inicializa os que existem.
fn destinos(r: Option<&Record>) -> Vec<DestinoDeTeleporte> {
    let Some(r) = r else { return Vec::new() };
    let quantos = i(r, "num_targets").clamp(0, 32) as usize;
    (1..=quantos)
        .filter_map(|k| {
            let id_ponto = i(r, &format!("targets_{k}_idTarget"));
            (id_ponto > 0).then_some(DestinoDeTeleporte {
                id_ponto,
                preco: i(r, &format!("targets_{k}_fee")).max(0),
                nivel: i(r, &format!("targets_{k}_required_level")).max(0),
            })
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
