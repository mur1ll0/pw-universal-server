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
