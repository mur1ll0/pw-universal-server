//! Distribuição de `m_ulType` (`TaskTemplate::tipo`) no `tasks.data` de um realm.
//!
//! Uso: `cargo run -p pw-data-loader --example tipos_de_missao -- <tasks.data>`
fn main() {
    let caminho = std::env::args().nth(1).expect("caminho do tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&std::fs::read(caminho).unwrap()).unwrap();
    let mut n = std::collections::BTreeMap::new();
    for x in t.tasks.values() {
        *n.entry(x.tipo).or_insert(0) += 1;
    }
    println!("{n:?}");
    for id in [1177u32, 1173, 9376] {
        if let Some(x) = t.tasks.get(&id) {
            println!("{id} {} tipo={}", x.name, x.tipo);
        }
    }
}
