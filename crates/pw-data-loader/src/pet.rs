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

/// `pet_data_temp` (`gs/petdataman.h:18-150`): o modelo de um mascote no `PET_ESSENCE`, com
/// as fórmulas de atributo por nível. Montado como `pet_dataman::LoadTemplate`
/// (`gs/petdataman.cpp:10-150`; no `gs` 1.2.6, VA 0x8143580).
#[derive(Debug, Clone, PartialEq)]
pub struct ModeloDeMascote {
    pub tid: u32,
    /// `pet_data::PET_CLASS_*`: 0 montaria, 1 combate, 2 companhia, 3 invocação, 4 planta,
    /// 5 evolução.
    pub classe: i32,
    pub hp: [f32; 3],
    pub hp_gen: [f32; 3],
    pub dano: [f32; 4],
    pub velocidade: [f32; 2],
    pub ataque: [f32; 3],
    pub esquiva: [f32; 3],
    pub defesa: [f32; 4],
    pub resistencia: [f32; 4],
    /// `size`: o raio do corpo, somado ao alcance do golpe (`CreatePetBase`,
    /// `npcgenerator.cpp:2033`).
    pub corpo: f32,
    pub alcance: f32,
    /// Em tiques de 50 ms: `(int)(segundos × 20 + 0,1)`.
    pub atraso_do_dano: i32,
    pub intervalo_do_golpe: i32,
    pub visao: f32,
    pub comida: i32,
    pub habitat: i32,
    pub imunidade: i32,
    pub nivel_maximo: i32,
    pub nivel_exigido: i32,
}

/// Os atributos de base de um mascote num nível: `pet_dataman::GenerateBaseProp`
/// (`gs/petdataman.cpp:152-186`). O de combate não tem mana (`max_mp = 0`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AtributosDeMascote {
    pub vida: i32,
    pub regeneracao: i32,
    pub andar: f32,
    pub correr: f32,
    pub dano: i32,
    /// `attack` (acerto).
    pub acerto: i32,
    pub intervalo_do_golpe: i32,
    pub alcance: f32,
    pub resistencia: i32,
    pub defesa: i32,
    /// `armor` (esquiva).
    pub esquiva: i32,
}

impl ModeloDeMascote {
    // As fórmulas de `gs/petdataman.h:23-76`, com o truncamento `(int)` do original.
    pub fn vida(&self, n: i32) -> i32 {
        (self.hp[0] * (n as f32 - self.hp[1] * self.nivel_exigido as f32 + self.hp[2])) as i32
    }
    pub fn regeneracao(&self, n: i32) -> i32 {
        (self.hp_gen[0] * (n as f32 - self.hp_gen[1] * self.nivel_exigido as f32 + self.hp_gen[2])) as i32
    }
    pub fn dano(&self, n: i32) -> i32 {
        let n = n as f32;
        (self.dano[0] * (self.dano[1] * n * n + self.dano[2] * n + self.dano[3])) as i32
    }
    pub fn defesa(&self, n: i32) -> i32 {
        (self.defesa[0] * (self.defesa[1] * (n as f32 - self.defesa[2] * self.nivel_exigido as f32) + self.defesa[3])) as i32
    }
    pub fn acerto(&self, n: i32) -> i32 {
        (self.ataque[0] * (n as f32 - self.ataque[1] * self.nivel_exigido as f32 + self.ataque[2])) as i32
    }
    pub fn esquiva(&self, n: i32) -> i32 {
        (self.esquiva[0] * (n as f32 - self.esquiva[1] * self.nivel_exigido as f32 + self.esquiva[2])) as i32
    }
    pub fn resistencia(&self, n: i32) -> i32 {
        (self.resistencia[0] * (self.resistencia[1] * (n as f32 - self.resistencia[2] * self.nivel_exigido as f32) + self.resistencia[3]))
            as i32
    }

    /// `GenerateBaseProp`: velocidade `speed_a + speed_b × (nível − 1)`, andar a metade.
    pub fn atributos(&self, nivel: i32) -> AtributosDeMascote {
        let velocidade = self.velocidade[0] + self.velocidade[1] * (nivel - 1) as f32;
        AtributosDeMascote {
            vida: self.vida(nivel),
            regeneracao: self.regeneracao(nivel),
            andar: velocidade * 0.5,
            correr: velocidade,
            dano: self.dano(nivel),
            acerto: self.acerto(nivel),
            intervalo_do_golpe: self.intervalo_do_golpe,
            alcance: self.alcance,
            resistencia: self.resistencia(nivel),
            defesa: self.defesa(nivel),
            esquiva: self.esquiva(nivel),
        }
    }
}

/// `pet_dataman::LoadTemplate`: `id_type` → classe (8781..8783, e no 1.5.5 também 28752,
/// 28913 e 37698); o resto é recusado. Mascote que luta (combate, invocação, planta,
/// evolução) com atraso de dano fora de 2..=200 tiques, intervalo ≤ 2, visão < 0,1 ou
/// `hp_a` ≤ 0 também (`petdataman.cpp:120-133`; no 1.2.6 só o de combate é conferido).
pub fn carregar_modelos(g: &GenericElementsData) -> HashMap<u32, ModeloDeMascote> {
    let f = |r: &crate::generic_elements::Record, c: &str| match r.get(c) {
        Some(crate::generic_elements::FieldValue::Float(v)) => *v,
        Some(crate::generic_elements::FieldValue::Int(v)) => *v as f32,
        _ => 0.0,
    };
    let mut modelos = HashMap::new();
    for r in g.get("PET_ESSENCE") {
        let id = i(r, "ID");
        if id <= 0 {
            continue;
        }
        let classe = match i(r, "id_type") {
            8781 => 0,
            8782 => 1,
            8783 => 2,
            28752 => 3,
            28913 => 4,
            37698 => 5,
            _ => continue,
        };
        let tiques = |s: f32| (s * 20.0 + 0.1) as i32;
        let m = ModeloDeMascote {
            tid: id as u32,
            classe,
            hp: [f(r, "hp_a"), f(r, "hp_b"), f(r, "hp_c")],
            hp_gen: [f(r, "hp_gen_a"), f(r, "hp_gen_b"), f(r, "hp_gen_c")],
            dano: [f(r, "damage_a"), f(r, "damage_b"), f(r, "damage_c"), f(r, "damage_d")],
            velocidade: [f(r, "speed_a"), f(r, "speed_b")],
            ataque: [f(r, "attack_a"), f(r, "attack_b"), f(r, "attack_c")],
            esquiva: [f(r, "armor_a"), f(r, "armor_b"), f(r, "armor_c")],
            defesa: [f(r, "physic_defence_a"), f(r, "physic_defence_b"), f(r, "physic_defence_c"), f(r, "physic_defence_d")],
            resistencia: [f(r, "magic_defence_a"), f(r, "magic_defence_b"), f(r, "magic_defence_c"), f(r, "magic_defence_d")],
            corpo: f(r, "size"),
            alcance: f(r, "attack_range"),
            atraso_do_dano: tiques(f(r, "damage_delay")),
            intervalo_do_golpe: tiques(f(r, "attack_speed")),
            visao: f(r, "sight_range"),
            comida: i(r, "food_mask"),
            // O 1.2.6 aceita 0..=2 e põe 0 no resto (VA 0x8143893-0x81438a5); o 1.5.5 vai
            // até 6. Fora disso, chão.
            habitat: match i(r, "inhabit_type") {
                h @ 0..=6 => h,
                _ => 0,
            },
            imunidade: i(r, "immune_type"),
            nivel_maximo: i(r, "level_max"),
            nivel_exigido: i(r, "level_require"),
        };
        let luta = matches!(classe, 1 | 3 | 4 | 5);
        if luta
            && (m.atraso_do_dano > 200
                || m.atraso_do_dano <= 1
                || m.intervalo_do_golpe <= 2
                || m.visao < 0.1
                || m.hp[0] <= 0.0)
        {
            continue;
        }
        modelos.insert(id as u32, m);
    }
    modelos
}

/// `PET_FOOD_ESSENCE`: `(hornor, food_type)` de cada comida de mascote — o `_ess.honor` e o
/// `_ess.food_type` do `item_pet_food::OnUse` (`gs/item/item_petfood.cpp:8-20`).
pub fn carregar_comidas(g: &GenericElementsData) -> HashMap<u32, (i32, i32)> {
    g.get("PET_FOOD_ESSENCE")
        .iter()
        .filter_map(|r| {
            let id = i(r, "ID");
            (id > 0).then(|| (id as u32, (i(r, "hornor"), i(r, "food_type"))))
        })
        .collect()
}
