//! Ids cujo `Name` contém um trecho (sem diferenciar maiúsculas), em todas as tabelas.
//!
//! Uso: `cargo run -p pw-data-loader --example achar_nome -- <pasta config> <trecho>`
use pw_data_loader::generic_elements::load_elements_data_auto;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let trecho = a[2].to_lowercase();
    for (tabela, regs) in &e.tables {
        for r in regs {
            let nome = r.get("Name").and_then(|v| v.as_text()).unwrap_or("");
            if nome.to_lowercase().contains(&trecho) {
                let id = r.get("ID").and_then(|v| v.as_i32()).unwrap_or(0);
                println!("{id}: {tabela} {nome:?}");
            }
        }
    }
}
