//! Carregamento de dados de ovos e mascotes a partir de `PET_EGG_ESSENCE` e `PET_ESSENCE`.

use crate::generic_elements::GenericElementsData;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DadosDoOvoDePet {
    pub id: u32,
    pub id_pet: u32,
    pub money_hatched: i32,
    pub money_restored: i32,
    pub honor_point: i32,
    pub level: i16,
    pub exp: i32,
    pub skill_point: i32,
    pub req_level: i32,
    pub req_class: i32,
    pub pet_class: i32,
    pub skills: Vec<(i32, i32)>,
}

fn i(r: &crate::generic_elements::Record, campo: &str) -> i32 {
    r.get(campo).and_then(|v| v.as_i32()).unwrap_or(0)
}

/// Carrega todos os templates de ovos de mascote do `elements.data`.
pub fn carregar_ovos(g: &GenericElementsData) -> HashMap<u32, DadosDoOvoDePet> {
    let mut pets_info: HashMap<u32, (i32, i32, i32)> = HashMap::new();
    for r in g.get("PET_ESSENCE") {
        let id = i(r, "ID");
        if id > 0 {
            let req_level = i(r, "level_require").max(1);
            let req_class = i(r, "character_combo_id");
            let id_type = i(r, "id_type");
            pets_info.insert(id as u32, (req_level, req_class, id_type));
        }
    }

    let mut ovos = HashMap::new();
    for r in g.get("PET_EGG_ESSENCE") {
        let id = i(r, "ID");
        if id <= 0 {
            continue;
        }
        let id_pet = i(r, "id_pet").max(0) as u32;
        let money_hatched = i(r, "money_hatched");
        let money_restored = i(r, "money_restored");
        let honor_point = i(r, "honor_point");
        let level = i(r, "level").max(1) as i16;
        let exp = i(r, "exp");
        let skill_point = i(r, "skill_point");

        let (req_level, mut req_class, id_type) = pets_info
            .get(&id_pet)
            .copied()
            .unwrap_or((1, 0xFFFF, 8781));

        let pet_class = match id_type {
            8781 => 0, // PET_CLASS_MOUNT
            8782 => 1, // PET_CLASS_COMBAT
            8783 => 2, // PET_CLASS_FOLLOW
            28752 => 3, // PET_CLASS_SUMMON
            28913 => 4, // PET_CLASS_PLANT
            37698 => 5, // PET_CLASS_EVOLUTION
            _ => 0,
        };

        // Montarias e ovos genéricos podem ser montados por todas as classes (0xFFFF)
        if pet_class == 0 || req_class == 0 {
            req_class = 0xFFFF;
        }

        let mut skills = Vec::new();
        for k in 1..=32 {
            let s = i(r, &format!("skills_{k}_id_skill"));
            let l = i(r, &format!("skills_{k}_level"));
            if s > 0 && l > 0 {
                skills.push((s, l));
            }
        }

        ovos.insert(
            id as u32,
            DadosDoOvoDePet {
                id: id as u32,
                id_pet,
                money_hatched,
                money_restored,
                honor_point,
                level,
                exp,
                skill_point,
                req_level,
                req_class,
                pet_class,
                skills,
            },
        );
    }
    ovos
}

/// Identifica todos os IDs de itens de missão (`TASKMATTER_ESSENCE` e itens com `proc_type & 0x0020`).
pub fn carregar_itens_de_missao(g: &GenericElementsData) -> HashSet<u32> {
    let mut set = HashSet::new();
    for r in g.get("TASKMATTER_ESSENCE") {
        let id = i(r, "ID");
        if id > 0 {
            set.insert(id as u32);
        }
    }
    for registros in g.tables.values() {
        let Some(primeiro) = registros.first() else { continue };
        if !primeiro.contains_key("proc_type") || !primeiro.contains_key("ID") {
            continue;
        }
        for r in registros {
            let proc = i(r, "proc_type");
            if (proc & 0x0020) != 0 {
                let id = i(r, "ID");
                if id > 0 {
                    set.insert(id as u32);
                }
            }
        }
    }
    set
}

/// Lista de NPCs da cena com serviços ilimitados (`distance_service & 1 != 0`).
pub fn carregar_scene_service_npcs(g: &GenericElementsData) -> Vec<(i32, i32)> {
    let mut lista = Vec::new();
    for r in g.get("NPC_ESSENCE") {
        let id = i(r, "ID");
        let dist = i(r, "distance_service");
        if id > 0 && (dist & 1 != 0) {
            let nid = ((id as u32) | 0x8000_0000) as i32;
            lista.push((id, nid));
        }
    }
    lista
}
