//! Cruza o `m_bCommonItem` dos itens pedidos pelas missões com a tabela do item no
//! `elements.data` — confere a decodificação do campo (o item de missão "de verdade",
//! `TASKMATTER_ESSENCE`, deveria vir com `comum = false`).
//!
//! Uso: `cargo run -p pw-data-loader --example comum_por_tipo -- <pasta config>`
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::tasks::TasksData;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let mut tabela_do_item: HashMap<u32, String> = HashMap::new();
    for (tabela, regs) in &e.tables {
        for r in regs {
            if let Some(id) = r.get("ID").and_then(|v| v.as_i32()) {
                tabela_do_item.insert(id as u32, tabela.clone());
            }
        }
    }
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", a[1])).unwrap()).unwrap();
    let mut contagem: BTreeMap<(String, bool), usize> = BTreeMap::new();
    for m in t.tasks.values() {
        for i in &m.item_collections {
            let tab = tabela_do_item.get(&i.id).cloned().unwrap_or_else(|| "?".into());
            *contagem.entry((tab, i.comum)).or_default() += 1;
        }
    }
    for ((tab, comum), n) in contagem {
        println!("{tab:32} comum={comum:5} {n}");
    }
}
