//! Em que slot do corpo cada item pode entrar — o `equip_mask` do item (B191).
//!
//! O servidor original grava no `item_data` de cada item gerado uma máscara de 32 bits
//! (`generate_*`, `gs/template/generate_item_temp.h`) e confere a posição com
//! `CheckEquipPostion(equip_mask, índice) = Equipmask32To64(mask) & (1 << índice)`
//! (`gs/item.h:294-297`) ao equipar e ao trocar dentro do corpo (`gs/player.cpp:8008-8041`,
//! `:8109`, `:8150`). O bit `i` é o slot `i` do corpo (`EQUIPIVTR_*`, `EC_IvtrTypes.h:56-85`).
//!
//! A máscara de cada família, pelo `itemdataman.cpp:1355-1605` (essência → `generate_*`):
//!
//! | essência | máscara | origem |
//! | :--- | :--- | :--- |
//! | `WEAPON_ESSENCE` | `0x1` | `generate_item_temp.h:267` |
//! | `ARMOR_ESSENCE` | `ARMOR_SUB_TYPE.equip_mask` | `:470` |
//! | `PROJECTILE_ESSENCE` | `0x800` | `:585` |
//! | `DECORATION_ESSENCE` | `DECORATION_SUB_TYPE.equip_mask` | `:752` |
//! | `FLYSWORD_ESSENCE`, `WINGMANWING_ESSENCE` | `0x1000` | `:1146`, `:1209` |
//! | `FASHION_ESSENCE` | `FASHION_SUB_TYPE.equip_fashion_mask` | `:1663` |
//! | `DAMAGERUNE_ESSENCE` | `0x20000` | `:1304` |
//! | `BIBLE_ESSENCE` / `SPEAKER_ESSENCE` | `0x40000` / `0x80000` | `:2191`, `:2242` |
//! | `AUTOHP_ESSENCE` / `AUTOMP_ESSENCE` | `0x100000` / `0x200000` | `:2285`, `:2329` |
//! | `GOBLIN_ESSENCE` | `0x800000` | `:2462` |
//! | `SELL_CERTIFICATE_ESSENCE` | `0x1000000` | `:2634` |
//! | `FORCE_TOKEN_ESSENCE` | `0x4000000` | `:3031` |
//! | `DYNSKILLEQUIP_ESSENCE` | `0x18000000` | `:3077` |
//! | `POKER_ESSENCE` | `equip_mask_64_to_32(POKER_SUB_TYPE.equip_mask)` | `:3160` |
//! | `ASTROLABE_ESSENCE` | `equip_mask_64_to_32(1 << 38)` | `:3287` |
//!
//! Os valores das constantes estão em `itemdataman.h:340-366`. Qualquer outra família grava
//! máscara 0 (não entra no corpo). O 1.2.6 (`elements` v7) tem as listas até `AUTOMP`; as
//! que não existem no arquivo simplesmente não aparecem.

use crate::generic_elements::{GenericElementsData, Record};
use std::collections::HashMap;

/// `ELEMENTDATAMAN_EQUIP_MASK_EXTEND64` e `_HIGH` (`itemdataman.h:363-365`).
const EXTEND64: u32 = 0x8000_0000;
const ALTOS: u32 = 0xC000_0000;

/// `equip_mask_64_to_32` (`itemdataman.h:369-381`).
fn de_64_para_32(mascara: u64) -> u32 {
    if mascara >> 32 != 0 {
        ((mascara >> 32) as u32 & !ALTOS) | EXTEND64
    } else {
        mascara as u32 & !ALTOS
    }
}

/// `equip_mask_32_to_64` / `Equipmask32To64` (`itemdataman.h:383-395`).
pub fn de_32_para_64(mascara: u32) -> u64 {
    let m = (mascara & !ALTOS) as u64;
    if mascara & EXTEND64 != 0 { m << 32 } else { m }
}

/// `CheckEquipPostion` (`gs/item.h:294-297`): o item de máscara `mascara` pode ir ao slot?
pub fn cabe_no_slot(mascara: u32, slot: usize) -> bool {
    slot < 64 && de_32_para_64(mascara) & (1u64 << slot) != 0
}

fn i(r: &Record, n: &str) -> i64 {
    r.get(n).and_then(|v| v.as_i32()).unwrap_or(0) as i64
}

/// Máscara de 32 bits (como o `item_data` guarda) por id de item.
pub fn carregar(g: &GenericElementsData) -> HashMap<u32, u32> {
    let subtipo = |tabela: &str, campo: &str| -> HashMap<i64, u32> {
        g.get(tabela).iter().map(|r| (i(r, "ID"), i(r, campo) as u32)).collect()
    };
    let armadura = subtipo("ARMOR_SUB_TYPE", "equip_mask");
    let acessorio = subtipo("DECORATION_SUB_TYPE", "equip_mask");
    let roupa = subtipo("FASHION_SUB_TYPE", "equip_fashion_mask");
    // `POKER_SUB_TYPE`: união `{equip_mask1, equip_mask2}` / `UINT64 equip_mask`
    // (`exptypes.h:659-662`), little-endian.
    let carta: HashMap<i64, u32> = g
        .get("POKER_SUB_TYPE")
        .iter()
        .map(|r| {
            let m = (i(r, "equip_mask_1") as u32 as u64) | ((i(r, "equip_mask_2") as u32 as u64) << 32);
            (i(r, "ID"), de_64_para_32(m))
        })
        .collect();

    let mut saida = HashMap::new();
    let mut fixa = |tabela: &str, mascara: u32| {
        for r in g.get(tabela) {
            if i(r, "ID") > 0 {
                saida.insert(i(r, "ID") as u32, mascara);
            }
        }
    };
    fixa("WEAPON_ESSENCE", 0x1);
    fixa("PROJECTILE_ESSENCE", 0x800);
    fixa("FLYSWORD_ESSENCE", 0x1000);
    fixa("WINGMANWING_ESSENCE", 0x1000);
    fixa("DAMAGERUNE_ESSENCE", 0x2_0000);
    fixa("BIBLE_ESSENCE", 0x4_0000);
    fixa("SPEAKER_ESSENCE", 0x8_0000);
    fixa("AUTOHP_ESSENCE", 0x10_0000);
    fixa("AUTOMP_ESSENCE", 0x20_0000);
    fixa("GOBLIN_ESSENCE", 0x80_0000);
    fixa("SELL_CERTIFICATE_ESSENCE", 0x100_0000);
    fixa("FORCE_TOKEN_ESSENCE", 0x400_0000);
    fixa("DYNSKILLEQUIP_ESSENCE", 0x1800_0000);
    fixa("ASTROLABE_ESSENCE", de_64_para_32(1u64 << 38));
    for (tabela, mapa) in [("ARMOR_ESSENCE", &armadura), ("DECORATION_ESSENCE", &acessorio),
                           ("FASHION_ESSENCE", &roupa), ("POKER_ESSENCE", &carta)] {
        for r in g.get(tabela) {
            if i(r, "ID") > 0 {
                saida.insert(i(r, "ID") as u32, mapa.get(&i(r, "id_sub_type")).copied().unwrap_or(0));
            }
        }
    }
    saida
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_mascara_de_64_bits_volta_igual() {
        // Astrolábio: bit 38 vai ao alto com EXTEND64 e volta ao mesmo bit.
        let m = de_64_para_32(1u64 << 38);
        assert_eq!(m, EXTEND64 | (1 << 6));
        assert!(cabe_no_slot(m, 38));
        assert!(!cabe_no_slot(m, 6));
        // Arma só no slot 0; o bit de addon (0x40000000) não abre slot nenhum.
        assert!(cabe_no_slot(0x1 | 0x4000_0000, 0));
        assert!(!cabe_no_slot(0x1 | 0x4000_0000, 30));
        // Habilidade dinâmica: os dois slots 27 e 28.
        assert!(cabe_no_slot(0x1800_0000, 27) && cabe_no_slot(0x1800_0000, 28));
    }
}
