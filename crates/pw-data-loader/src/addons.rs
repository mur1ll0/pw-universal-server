//! Propriedades adicionais (addons) e o que a geração de equipamento precisa do
//! `elements.data`.
//!
//! - Quem trata cada id: `specs/addons_155/addons.json`, extraído de
//!   `gs/item/item_addon.cpp` (`INSERT_ADDON(id, tratador)`).
//! - Os parâmetros brutos: `EQUIPMENT_ADDON` (`num_params`, `param1..3`,
//!   `gs/template/exptypes.h:122-134`). Parâmetro de porcentagem vem como os bits de um
//!   `float` guardados num `int` (`arg_addon<PERCENT>::GenerateParam`, `item_addon.cpp:103-110`).
//! - O que o drop sorteia: `WEAPON/ARMOR/DECORATION_ESSENCE` (`generate_item_temp.h:200-480`,
//!   `740-830`).

use crate::generic_elements::{FieldValue, GenericElementsData, Record};
use serde::Deserialize;
use std::collections::HashMap;

const ADDONS_155_JSON: &str = include_str!("../../../specs/addons_155/addons.json");

#[derive(Deserialize)]
struct Arquivo {
    tratadores: HashMap<String, String>,
}

/// Como `GenerateParam` trata os parâmetros de um addon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorteio {
    /// `arg_addon<POINT>`: 1 parâmetro, como está.
    Ponto,
    /// `arg_addon<DOUBLE_POINT>` e as essências: `RandNormal(arg0, arg1)`, 1 parâmetro.
    EntreDois,
    /// `arg_addon<PERCENT>`: `(int)(float × 100 + 0,1)`, 1 parâmetro.
    Porcento,
    /// `arg_addon<DOUBLE_PERCENT>`: `RandNormal(p0×100, p1×100)`, 1 parâmetro.
    EntreDoisPorcento,
    /// `refine_addon_template`: o parâmetro do tratador base e o nível (0), 2 parâmetros.
    Refino,
    /// 2 parâmetros como estão (`item_decoration_specific_*`).
    DoisComoEstao,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DadosDoAddon {
    pub tratador: String,
    pub num_params: i32,
    pub params: [i32; 3],
}

impl DadosDoAddon {
    /// O sorteio que o tratador faz; `None` para tratador sem porte (o addon não é gerado).
    pub fn sorteio(&self) -> Option<Sorteio> {
        use Sorteio::*;
        let t = self.tratador.as_str();
        Some(match t {
            "enhance_hp_addon" | "enhance_mp_addon" | "enhance_attack_degree" | "enhance_defend_degree"
            | "enhance_all_resistance_addon" | "enhance_attack_addon" | "enhance_defense_addon_1arg"
            | "enhance_defense_addon_2" | "enhance_damage_addon_2" | "enhance_magic_damage_addon_2"
            | "enhance_armor_addon" | "enhance_penetration" | "enhance_resilience" | "enhance_damage_addon"
            | "enhance_magic_damage_addon" => Ponto,
            "enhance_str_addon" | "enhance_agi_addon" | "enhance_vit_addon" | "enhance_eng_addon"
            | "enhance_hp_addon_2" | "enhance_mp_addon_2" | "enhance_attack_addon_2" | "enhance_defense_addon"
            | "enhance_armor_range_addon" | "enhance_weapon_damage_addon" | "enhance_weapon_max_damage_addon"
            | "enhance_weapon_magic_addon" | "enhance_weapon_max_magic_addon" => EntreDois,
            "enhance_crit_rate" | "enhance_damage_reduce_addon" | "reduce_cast_time_addon"
            | "enhance_all_resistance_scale_addon" | "enhance_damage_scale_addon_2" | "enhance_magic_damage_scale_addon" => Porcento,
            "item_decoration_specific_damage_addon" | "item_decoration_specific_magic_damage_addon" => DoisComoEstao,
            _ if t.starts_with("refine_") => Refino,
            _ if t.starts_with("IA_EA_ESS<") || t.starts_with("IA_ED_ESS<") => EntreDois,
            _ if t.starts_with("item_armor_enhance_resistance<") || t.starts_with("item_decoration_enchance_resistance<") => EntreDois,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct TabelaDeAddons {
    pub por_id: HashMap<u32, DadosDoAddon>,
}

impl TabelaDeAddons {
    pub fn carregar(elements: &GenericElementsData) -> Self {
        let tratadores: HashMap<u32, String> = serde_json::from_str::<Arquivo>(ADDONS_155_JSON)
            .map(|a| a.tratadores.into_iter().filter_map(|(k, v)| Some((k.parse().ok()?, v))).collect())
            .unwrap_or_default();
        let por_id = elements
            .get("EQUIPMENT_ADDON")
            .iter()
            .filter_map(|r| {
                let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
                let id = i("ID") as u32;
                let tratador = tratadores.get(&id)?.clone();
                Some((id, DadosDoAddon { tratador, num_params: i("num_params"), params: [i("param1"), i("param2"), i("param3")] }))
            })
            .collect();
        Self { por_id }
    }
}

/// A família do equipamento gerado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Familia {
    Arma,
    Armadura,
    Decoracao,
}

/// O que `generate_weapon/armor/decoration` leem do modelo.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeloDeGeracao {
    pub familia: Familia,
    pub id_sub_type: i32,
    pub fixed_props: bool,
    pub proc_type: i32,
    pub furos_no_drop: Vec<f32>,
    pub quantos_addons: Vec<f32>,
    pub chance_de_unico: f32,
    pub addons: Vec<(u32, f32)>,
    pub unicos: Vec<(u32, f32)>,
    pub durabilidade: (i32, i32),
    pub durabilidade_no_drop: (i32, i32),
    /// Arma: `damage_high_min/max`, `magic_damage_high_min/max`.
    pub dano_maximo: (i32, i32),
    pub dano_magico_maximo: (i32, i32),
    /// Armadura/acessório.
    pub defesa: (i32, i32),
    pub evasao: (i32, i32),
    pub mana: (i32, i32),
    pub vida: (i32, i32),
    pub dano: (i32, i32),
    pub dano_magico: (i32, i32),
    pub resistencias: [(i32, i32); 5],
    pub todas_as_resistencias: bool,
}

fn f(r: &Record, n: &str) -> f32 {
    match r.get(n) {
        Some(FieldValue::Float(v)) => *v,
        Some(FieldValue::Int(v)) => *v as f32,
        _ => 0.0,
    }
}

fn i(r: &Record, n: &str) -> i32 {
    r.get(n).and_then(|v| v.as_i32()).unwrap_or(0)
}

fn lista(r: &Record, prefixo: &str, sufixo_id: &str, sufixo_p: &str, n: usize) -> Vec<(u32, f32)> {
    (1..=n).map(|k| (i(r, &format!("{prefixo}_{k}_{sufixo_id}")).max(0) as u32, f(r, &format!("{prefixo}_{k}_{sufixo_p}")))).collect()
}

pub type TabelaDeGeracao = HashMap<u32, ModeloDeGeracao>;

pub fn carregar_geracao(elements: &GenericElementsData) -> TabelaDeGeracao {
    let mut t = TabelaDeGeracao::new();
    for (tabela, familia) in [("WEAPON_ESSENCE", Familia::Arma), ("ARMOR_ESSENCE", Familia::Armadura), ("DECORATION_ESSENCE", Familia::Decoracao)] {
        for r in elements.get(tabela) {
            let id = i(r, "ID");
            if id <= 0 {
                continue;
            }
            let probs = |base: &str, n: usize| (0..n).map(|k| f(r, &format!("{base}{k}"))).collect::<Vec<_>>();
            let (furos, n_addons) = match familia {
                Familia::Arma => (probs("drop_probability_socket", 3), probs("probability_addon_num", 6)),
                Familia::Armadura => (probs("drop_probability_socket", 5), probs("probability_addon_num", 5)),
                Familia::Decoracao => (Vec::new(), probs("probability_addon_num", 5)),
            };
            let par = |a: &str, b: &str| (i(r, a), i(r, b));
            let mut res = [(0, 0); 5];
            for (k, x) in res.iter_mut().enumerate() {
                *x = par(&format!("magic_defences_{}_low", k + 1), &format!("magic_defences_{}_high", k + 1));
            }
            t.insert(
                id as u32,
                ModeloDeGeracao {
                    familia,
                    id_sub_type: i(r, "id_sub_type"),
                    fixed_props: i(r, "fixed_props") != 0,
                    proc_type: i(r, "proc_type"),
                    furos_no_drop: furos,
                    quantos_addons: n_addons,
                    chance_de_unico: f(r, "probability_unique"),
                    addons: lista(r, "addons", "id_addon", "probability_addon", 32),
                    unicos: if familia == Familia::Arma { lista(r, "uniques", "id_unique", "probability_unique", 16) } else { Vec::new() },
                    durabilidade: par("durability_min", "durability_max"),
                    durabilidade_no_drop: par("durability_drop_min", "durability_drop_max"),
                    dano_maximo: par("damage_high_min", "damage_high_max"),
                    dano_magico_maximo: par("magic_damage_high_min", "magic_damage_high_max"),
                    defesa: par("defence_low", "defence_high"),
                    evasao: par("armor_enhance_low", "armor_enhance_high"),
                    mana: par("mp_enhance_low", "mp_enhance_high"),
                    vida: par("hp_enhance_low", "hp_enhance_high"),
                    dano: par("damage_low", "damage_high"),
                    dano_magico: par("magic_damage_low", "magic_damage_high"),
                    resistencias: res,
                    todas_as_resistencias: i(r, "force_all_magic_defences") != 0,
                },
            );
        }
    }
    t
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_arquivo_de_tratadores_se_le() {
        let a: Arquivo = serde_json::from_str(ADDONS_155_JSON).unwrap();
        assert!(a.tratadores.len() > 2900);
        assert_eq!(a.tratadores["1497"], "refine_damage");
    }
}
