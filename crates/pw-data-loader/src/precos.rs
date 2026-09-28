//! O preço de cada item, varrido de todas as tabelas do `elements.data`.
//!
//! # Por que uma varredura, e não uma tabela por família
//!
//! A loja de NPC precisa do preço de qualquer coisa que ela venda: arma, armadura,
//! remédio, material, flecha, livro de habilidade. Cada família mora numa tabela própria,
//! mas **os dois campos de preço têm o mesmo nome em todas** — `price` e `shop_price`.
//! Varrer por nome de campo cobre as dezenas de famílias de uma vez, e passa a cobrir
//! sozinha qualquer família nova que o catálogo venha a decodificar.
//!
//! # Os dois campos
//!
//! `price` é quanto o NPC paga ao jogador; `shop_price` é quanto ele cobra. O original usa
//! o segundo e trata o primeiro como piso (`gs/serviceprovider.cpp:241-246`) — se um item
//! valesse mais na venda do que na compra, o jogador comprava e revendia em laço.
//!
//! Até 2026-09-11 a loja cobrava **100 moedas fixas por unidade**, de qualquer coisa.

use crate::generic_elements::GenericElementsData;
use std::collections::HashMap;
use tracing::info;

/// `(price, shop_price)` por id de item.
pub type TabelaDePrecos = HashMap<u32, (i32, i32)>;

/// Varre todas as tabelas atrás de registros que tenham `ID` e `shop_price`.
///
/// Um id repetido entre tabelas mantém a **primeira** ocorrência: os espaços de essência
/// do `elements.data` não se sobrepõem (ver o teste `as_tres_familias_sao_conjuntos_disjuntos`
/// no `pw-data-loader`), então a repetição só aconteceria num arquivo corrompido — e aí
/// vale o que foi lido primeiro, que ao menos é determinístico.
pub fn carregar(elements: &GenericElementsData) -> TabelaDePrecos {
    let mut tabela = TabelaDePrecos::default();
    let mut tabelas_com_preco = 0usize;

    for (nome, registros) in &elements.tables {
        let Some(primeiro) = registros.first() else { continue };
        if !primeiro.contains_key("shop_price") || !primeiro.contains_key("ID") {
            continue;
        }
        let _ = nome;
        tabelas_com_preco += 1;

        for reg in registros {
            let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            let id = i("ID");
            if id <= 0 {
                continue;
            }
            tabela
                .entry(id as u32)
                .or_insert((i("price"), i("shop_price")));
        }
    }

    if !tabela.is_empty() {
        info!(
            "preços: {} itens em {tabelas_com_preco} tabelas do elements.data",
            tabela.len()
        );
    }
    tabela
}

/// `ITEM_PROC_TYPE_UNREPAIRABLE` (`gs/item.h:321`): a peça não entra no reparo.
pub const PROC_IRREPARAVEL: i32 = 0x1000;

/// O que o reparo precisa de um equipamento: `repairfee` e se o `proc_type` o proíbe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReparoDoItem {
    /// `repairfee` do `WEAPON_ESSENCE`/`ARMOR_ESSENCE`/`DECORATION_ESSENCE` — o preço de
    /// consertar a peça inteira (`itemdataman::get_item_repair_fee`,
    /// `gs/template/itemdataman.cpp:1016-1034`; outras famílias devolvem 0).
    pub taxa: i32,
    /// `proc_type & ITEM_PROC_TYPE_UNREPAIRABLE` (`gs/item_list.cpp:226`).
    pub irreparavel: bool,
}

/// As três famílias que têm `repairfee` (`itemdataman.cpp:1022-1030`).
const FAMILIAS_COM_REPARO: [&str; 3] = ["WEAPON_ESSENCE", "ARMOR_ESSENCE", "DECORATION_ESSENCE"];

/// `repairfee` e `proc_type` por id de equipamento. Lê as três tabelas pelo nome do campo, o
/// mesmo nas versões v7, v156 e v159 do catálogo.
pub fn carregar_reparo(elements: &GenericElementsData) -> HashMap<u32, ReparoDoItem> {
    let mut tabela = HashMap::new();
    for familia in FAMILIAS_COM_REPARO {
        for reg in elements.get(familia) {
            let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            let id = i("ID");
            if id <= 0 {
                continue;
            }
            tabela.entry(id as u32).or_insert(ReparoDoItem {
                taxa: i("repairfee").max(0),
                irreparavel: i("proc_type") & PROC_IRREPARAVEL != 0,
            });
        }
    }
    tabela
}

/// `(mp_launch, mp_per_second)` das asas de Arqueiro/Anjo (`WINGMANWING_ESSENCE`), por id.
/// São o único item de voo que gasta mana (`angel_wing_item::OnUse`,
/// `gs/item/item_flysword.cpp:118-148`; `angel_wing_fly_filter::Heartbeat`,
/// `gs/fly_filter.cpp:42-48`); a espada voadora gasta o próprio tempo de voo (`cur_time`).
pub fn carregar_asas(elements: &GenericElementsData) -> HashMap<u32, (i32, i32)> {
    elements
        .get("WINGMANWING_ESSENCE")
        .iter()
        .filter_map(|r| {
            let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            (i("ID") > 0).then(|| (i("ID") as u32, (i("mp_launch").max(0), i("mp_per_second").max(0))))
        })
        .collect()
}

/// `_tax_rate` do `vendor_provider` (`gs/serviceprovider.cpp:183`; 1,05 também no `gs` 1.2.6,
/// VA 0x8107987).
pub const TAXA_DO_VENDEDOR: f32 = 1.05;

/// `shop_price × 1,05 × (tax_rate + 1) + 0,5` em `float`, teto 2·10⁸, piso 1, e o
/// `AdjustVendorFee` (`gs/serviceprovider.cpp:185-198`, 220-252).
pub fn preco_do_vendedor(shop_price: i32, taxa_do_npc: f32) -> i32 {
    let taxa = taxa_do_npc + 1.0;
    let mut fp = shop_price as f32 * TAXA_DO_VENDEDOR * taxa + 0.5;
    if fp > 2e8 {
        fp = 2e8;
    }
    let mut preco = (fp as i32).max(1);
    if preco >= 100 {
        let passo = if preco < 1000 { 10 } else { 100 };
        let r = preco % passo;
        if r != 0 {
            preco += passo - r;
        }
    }
    preco
}

/// `player_template::GetRepairCost` (`gs/playertemplate.h:535-546`): `base × (falta/máxima)`
/// em `float`, zero quando nada falta. Quem soma várias peças trunca a soma, não cada parcela
/// (`item_list::GetRepairCost`, `gs/item_list.cpp:217-239`).
pub fn custo_do_reparo(falta: i32, maxima: i32, taxa: i32) -> f32 {
    if maxima > 0 && falta > 0 {
        taxa as f32 * (falta as f32 / maxima as f32)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_elements::{FieldValue, Record};

    fn registro(id: i32, price: i32, shop: i32) -> Record {
        let mut r = Record::new();
        r.insert("ID".into(), FieldValue::Int(id));
        r.insert("price".into(), FieldValue::Int(price));
        r.insert("shop_price".into(), FieldValue::Int(shop));
        r
    }

    #[test]
    fn o_vendedor_cobra_a_taxa_e_arredonda_para_cima() {
        // 9600 × 1,05 + 0,5 = 10080,5 → 10080 → múltiplo de 100 acima: 10100.
        assert_eq!(preco_do_vendedor(9600, 0.0), 10100);
        // 50 × 1,05f + 0,5 = 52,99999 em `float` (1,05f é 1,0499999…): trunca em 52, como o
        // original, que faz a mesma conta em `float`. Abaixo de 100 não arredonda.
        assert_eq!(preco_do_vendedor(50, 0.0), 52);
        // 137 × 1,05 + 0,5 = 144,35 → 144 → 150.
        assert_eq!(preco_do_vendedor(137, 0.0), 150);
        // Com a taxa do NPC (0,05): 1000 × 1,05 × 1,05 + 0,5 = 1103 → 1200.
        assert_eq!(preco_do_vendedor(1000, 0.05), 1200);
        assert_eq!(preco_do_vendedor(0, 0.0), 1);
    }

    #[test]
    fn custo_do_reparo_e_proporcional_ao_desgaste() {
        // taxa 1000, metade gasta → 500; nada gasto → 0; máxima zero → 0.
        assert_eq!(custo_do_reparo(1400, 2800, 1000), 500.0);
        assert_eq!(custo_do_reparo(0, 2800, 1000), 0.0);
        assert_eq!(custo_do_reparo(10, 0, 1000), 0.0);
    }

    #[test]
    fn so_entra_tabela_que_tem_os_dois_campos() {
        let mut tables = HashMap::new();
        tables.insert("COM_PRECO".to_string(), vec![registro(10, 5, 50)]);

        let mut sem = Record::new();
        sem.insert("ID".into(), FieldValue::Int(20));
        tables.insert("SEM_PRECO".to_string(), vec![sem]);

        let t = carregar(&GenericElementsData { tables, version: 156 });
        assert_eq!(t.get(&10), Some(&(5, 50)));
        assert_eq!(t.get(&20), None, "tabela sem shop_price não entra");
    }

    #[test]
    fn id_invalido_fica_de_fora() {
        let mut tables = HashMap::new();
        tables.insert("T".to_string(), vec![registro(0, 1, 2), registro(-3, 1, 2)]);
        assert!(carregar(&GenericElementsData { tables, version: 156 }).is_empty());
    }
}
