//! Registros de uma tabela com algum campo inteiro igual a um valor (e qual campo).
//!
//! Uso: `cargo run -p pw-data-loader --example campo_com_valor -- <pasta config> <TABELA|*> <valor>`
use pw_data_loader::generic_elements::load_elements_data_auto;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let valor: i32 = a[3].parse().unwrap();
    for (tabela, regs) in &e.tables {
        if a[2] != "*" && *tabela != a[2] {
            continue;
        }
        for r in regs {
            for (campo, v) in r.iter() {
                if v.as_i32() == Some(valor) && campo != "ID" {
                    let id = r.get("ID").and_then(|v| v.as_i32()).unwrap_or(0);
                    let nome = r.get("Name").and_then(|v| v.as_text()).unwrap_or("");
                    println!("{tabela} {id} {nome:?}: {campo}");
                }
            }
        }
    }
}
