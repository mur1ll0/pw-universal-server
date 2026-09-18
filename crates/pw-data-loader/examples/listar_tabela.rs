//! Lista id, nome e alguns campos de uma tabela do `elements.data`.
//! Uso: `--example listar_tabela -- <pasta config> <TABELA> [campo...]`
use pw_data_loader::generic_elements::{load_elements_data_auto, FieldValue};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    for r in e.get(&a[2]) {
        let mut l = format!("{:?} {:?}", r.get("ID").and_then(|v| v.as_i32()), r.get("Name").and_then(|v| v.as_text()));
        for c in &a[3..] {
            let v = match r.get(c.as_str()) {
                Some(FieldValue::Int(i)) => i.to_string(),
                Some(FieldValue::Float(f)) => f.to_string(),
                Some(FieldValue::Text(t)) => t.clone(),
                _ => "-".into(),
            };
            l.push_str(&format!(" {c}={v}"));
        }
        println!("{l}");
    }
}
