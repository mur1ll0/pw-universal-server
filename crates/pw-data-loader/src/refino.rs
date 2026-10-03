//! O que o refino, as pedras e os furos leem do `elements.data` (B163).
//!
//! | dado | tabela | quem lê no original |
//! | :--- | :--- | :--- |
//! | addon de refino e quantas Pedras Celestiais | `levelup_addon`, `material_need` de `WEAPON/ARMOR/DECORATION_ESSENCE` | `itemdataman::get_item_refine_addon` (`gs/template/itemdataman.cpp:206-228`) |
//! | grau do equipamento | `level` das mesmas tabelas | `item_list::EmbedItem`, `weapon/armor_item::MakeSlot` |
//! | pedra | `STONE_ESSENCE`: `level`, `color`, `install_price`, `uninstall_price`, `id_addon_damage`, `id_addon_defence` | `install_executor::OnServe`, `item_list::ClearEmbed`, `GetStoneColorLevel`, `generate_stone` |
//! | talismã de refino | `REFINE_TICKET_ESSENCE` | `gplayer_imp::RefineItemAddon` (`gs/player.cpp:11677-11810`) |
//! | furo de acessório | `EQUIP_MAKE_HOLE_CONFIG` 2013 + `DECORATION_SUB_TYPE.equip_mask` | `decoration_equip_item::MakeSlot` (`gs/item/equip_item.cpp:1474-1530`) |
//!
//! O `v7` (1.2.6) tem as mesmas colunas, menos o `id_addon_decoration` da pedra e a
//! `EQUIP_MAKE_HOLE_CONFIG`; o `gs` 1.2.6 lê `install_price` em `STONE_ESSENCE+0x158`
//! (`install_executor::OnServe`, VA 0x810b2cb) e `uninstall_price` em `+0x15c`
//! (`item_list::ClearEmbed`, VA 0x80b72a4).

use crate::generic_elements::{FieldValue, GenericElementsData, Record};
use std::collections::HashMap;

/// A família do equipamento, pela tabela em que o id está (`DT_WEAPON/ARMOR/DECORATION_ESSENCE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Familia {
    Arma,
    Armadura,
    Acessorio,
}

/// O que o refino e os furos usam de um equipamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipamentoRefinavel {
    pub familia: Familia,
    /// `level` — o grau (1–20) que limita a pedra e indexa as tabelas de furo.
    pub grau: i32,
    /// `levelup_addon` — o addon de refino (0 = não refina).
    pub addon_de_refino: u32,
    /// `material_need` — quantas Pedras Celestiais cada tentativa gasta.
    pub material: i32,
    /// `id_sub_type` (para o acessório: o `equip_mask` do subtipo diz se é colar/cinto).
    pub subtipo: i32,
}

/// Uma pedra (`STONE_ESSENCE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pedra {
    pub grau: i32,
    pub cor: i32,
    pub preco_de_incrustar: i32,
    pub preco_de_remover: i32,
    pub addon_na_arma: u32,
    pub addon_na_armadura: u32,
}

/// Um talismã de refino (`REFINE_TICKET_ESSENCE`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Talisma {
    pub chance_extra: f32,
    pub chance_de_cair_um: f32,
    pub mantem_o_nivel: bool,
    /// `fail_ext_succeed_prob[12]` — a chance de sucesso por nível quando o talismã mantém o nível.
    pub chance_por_nivel: [f32; 12],
    pub so_vinculado: bool,
    pub grau_maximo: i32,
}

/// Uma casa da `EQUIP_MAKE_HOLE_CONFIG`: `hole_list[n] { require_item_id, require_item_count, fee }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FuroDeAcessorio {
    pub item: i32,
    pub quantidade: i32,
    pub taxa: i32,
}

#[derive(Debug, Clone, Default)]
pub struct DadosDeRefino {
    pub equipamentos: HashMap<u32, EquipamentoRefinavel>,
    pub pedras: HashMap<u32, Pedra>,
    pub talismas: HashMap<u32, Talisma>,
    /// `DECORATION_SUB_TYPE.equip_mask` por id.
    pub mascara_do_subtipo: HashMap<i32, i32>,
    /// `EQUIP_MAKE_HOLE_CONFIG_ID` (2013, `gs/config.h:246`): `[grau-1][furo]`. Vazio sem a tabela (v7).
    pub furos_de_acessorio: Vec<[FuroDeAcessorio; 4]>,
}

/// `EQUIP_MAKE_HOLE_CONFIG_ID` (`gs/config.h:246`).
pub const ID_DA_TABELA_DE_FUROS: i32 = 2013;

fn i(r: &Record, n: &str) -> i32 {
    match r.get(n) {
        Some(FieldValue::Int(v)) => *v,
        Some(FieldValue::Float(v)) => *v as i32,
        _ => 0,
    }
}

fn f(r: &Record, n: &str) -> f32 {
    match r.get(n) {
        Some(FieldValue::Float(v)) => *v,
        Some(FieldValue::Int(v)) => f32::from_bits(*v as u32),
        _ => 0.0,
    }
}

pub fn carregar(g: &GenericElementsData) -> DadosDeRefino {
    let mut d = DadosDeRefino::default();
    for (tabela, familia) in [
        ("WEAPON_ESSENCE", Familia::Arma),
        ("ARMOR_ESSENCE", Familia::Armadura),
        ("DECORATION_ESSENCE", Familia::Acessorio),
    ] {
        for r in g.get(tabela) {
            let id = i(r, "ID");
            if id <= 0 {
                continue;
            }
            d.equipamentos.insert(
                id as u32,
                EquipamentoRefinavel {
                    familia,
                    grau: i(r, "level"),
                    addon_de_refino: i(r, "levelup_addon").max(0) as u32,
                    material: i(r, "material_need"),
                    subtipo: i(r, "id_sub_type"),
                },
            );
        }
    }
    for r in g.get("STONE_ESSENCE") {
        let id = i(r, "ID");
        if id <= 0 {
            continue;
        }
        d.pedras.insert(
            id as u32,
            Pedra {
                grau: i(r, "level"),
                cor: i(r, "color"),
                preco_de_incrustar: i(r, "install_price"),
                preco_de_remover: i(r, "uninstall_price"),
                addon_na_arma: i(r, "id_addon_damage").max(0) as u32,
                addon_na_armadura: i(r, "id_addon_defence").max(0) as u32,
            },
        );
    }
    for r in g.get("REFINE_TICKET_ESSENCE") {
        let id = i(r, "ID");
        if id <= 0 {
            continue;
        }
        let mut chance_por_nivel = [0.0; 12];
        for (k, c) in chance_por_nivel.iter_mut().enumerate() {
            *c = f(r, &format!("fail_ext_succeed_prob_{}", k + 1));
        }
        d.talismas.insert(
            id as u32,
            Talisma {
                chance_extra: f(r, "ext_succeed_prob"),
                chance_de_cair_um: f(r, "ext_reserved_prob"),
                mantem_o_nivel: i(r, "fail_reserve_level") != 0,
                chance_por_nivel,
                so_vinculado: i(r, "binding_only") != 0,
                grau_maximo: i(r, "require_level_max"),
            },
        );
    }
    for r in g.get("DECORATION_SUB_TYPE") {
        d.mascara_do_subtipo.insert(i(r, "ID"), i(r, "equip_mask"));
    }
    if let Some(r) = g.get("EQUIP_MAKE_HOLE_CONFIG").iter().find(|r| i(r, "ID") == ID_DA_TABELA_DE_FUROS) {
        d.furos_de_acessorio = (1..=20)
            .map(|nivel| {
                let mut furos = [FuroDeAcessorio::default(); 4];
                for (k, furo) in furos.iter_mut().enumerate() {
                    let p = format!("level_{nivel}_hole_{}_", k + 1);
                    *furo = FuroDeAcessorio {
                        item: i(r, &format!("{p}id")),
                        quantidade: i(r, &format!("{p}num")),
                        taxa: i(r, &format!("{p}price")),
                    };
                }
                furos
            })
            .collect();
    }
    d
}
