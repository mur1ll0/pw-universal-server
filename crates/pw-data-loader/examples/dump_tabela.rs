//! Imprime registros de uma tabela do `elements.data`, com os campos pedidos.
//!
//! Uso: `cargo run -p pw-data-loader --example dump_tabela -- <pasta config> <TABELA> [campo,campo,...] [id]`
//! Sem lista de campos, imprime todos; com `id`, só aquele registro.
use pw_data_loader::generic_elements::{load_elements_data_auto, FieldValue};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pasta = &args[1];
    let tabela = &args[2];
    let campos: Vec<&str> = args.get(3).map(|c| c.split(',').filter(|s| !s.is_empty()).collect()).unwrap_or_default();
    let id: Option<i32> = args.get(4).and_then(|v| v.parse().ok());
    let e = load_elements_data_auto(&std::fs::read(format!("{pasta}/elements.data")).unwrap()).unwrap();
    let t = e.get(tabela);
    println!("{tabela}: {} registros", t.len());
    for r in t {
        let rid = r.get("ID").and_then(|v| v.as_i32()).unwrap_or(-1);
        if id.is_some_and(|i| i != rid) {
            continue;
        }
        let mut linha = format!("ID {rid}");
        let mut nomes: Vec<&String> = r.keys().collect();
        nomes.sort();
        for n in nomes {
            if n == "ID" || (!campos.is_empty() && !campos.contains(&n.as_str())) {
                continue;
            }
            let v = match &r[n] {
                FieldValue::Int(v) => v.to_string(),
                FieldValue::Float(v) => format!("{v}"),
                outro => outro.as_text().map(|s| s.to_string()).unwrap_or_else(|| "?".into()),
            };
            linha.push_str(&format!(" | {n}={v}"));
        }
        println!("{linha}");
    }
}
