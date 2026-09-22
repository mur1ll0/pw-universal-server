//! Para que bolsa vai cada item que os monstros soltam para missão — e de que tabela do
//! `elements.data` ele é.
//!
//! Quem escolhe a bolsa é o `m_bDropCmnItem` de cada `MONSTER_WANTED`
//! (`gs/task/TaskTempl.inl:2028-2037`): verdadeiro manda para a bolsa comum, falso para a de
//! missão. Este exemplo mostra que o bit acompanha o tipo do item — `TASKMATTER_ESSENCE` vai
//! para a bolsa de missão, `TASKNORMALMATTER_ESSENCE` e os demais para a normal (B74).
//!
//! Uso: `cargo run -p pw-data-loader --example bolsa_do_item_de_missao -- <pasta config>`
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::tasks::TasksData;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pasta = &args[1];
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{pasta}/tasks.data")).unwrap()).unwrap();
    let e = load_elements_data_auto(&std::fs::read(format!("{pasta}/elements.data")).unwrap()).unwrap();

    let mut tabela_do_id: HashMap<i32, &str> = HashMap::new();
    for (tabela, regs) in &e.tables {
        for r in regs {
            if let Some(id) = r.get("ID").and_then(|v| v.as_i32()) {
                tabela_do_id.entry(id).or_insert(tabela.as_str());
            }
        }
    }

    // (tabela, vai para a bolsa comum) -> quantos ids distintos
    let mut contagem: BTreeMap<(&str, bool), Vec<u32>> = BTreeMap::new();
    for m in t.tasks.values() {
        for k in &m.monster_kills {
            if k.item_que_cai == 0 {
                continue;
            }
            let tabela = tabela_do_id.get(&(k.item_que_cai as i32)).copied().unwrap_or("(fora do elements)");
            let v = contagem.entry((tabela, k.item_comum)).or_default();
            if !v.contains(&k.item_que_cai) {
                v.push(k.item_que_cai);
            }
        }
    }
    for ((tabela, comum), ids) in &contagem {
        if *tabela == "TASKNORMALMATTER_ESSENCE" && !*comum {
            println!("EXCEÇÃO — TASKNORMALMATTER na bolsa de missão: {ids:?}");
        }
        println!(
            "{tabela:<28} bolsa {:<7} {:>4} itens  ex.: {:?}",
            if *comum { "comum" } else { "missão" },
            ids.len(),
            &ids[..ids.len().min(4)]
        );
    }
}
