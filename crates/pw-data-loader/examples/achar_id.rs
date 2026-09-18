//! Em que tabela do `elements.data` está um id, e com que nome.
//!
//! Uso: `cargo run -p pw-data-loader --example achar_id -- <pasta config> <id>...`
use pw_data_loader::generic_elements::load_elements_data_auto;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    for id in a[2..].iter().filter_map(|v| v.parse::<i32>().ok()) {
        for (tabela, regs) in &e.tables {
            for r in regs {
                if r.get("ID").and_then(|v| v.as_i32()) == Some(id) {
                    let nome = r.get("Name").and_then(|v| v.as_text()).unwrap_or("?");
                    println!("{id}: {tabela} {nome:?}");
                }
            }
        }
    }
}
