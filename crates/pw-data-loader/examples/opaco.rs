//! Mostra o `_opaco` (e o `ID`) dos primeiros registros de uma tabela, em hexa e como `i32`.
//!
//! Uso: `cargo run -p pw-data-loader --example opaco -- <elements.data> <TABELA> [n]`
use pw_data_loader::generic_elements::{load_elements_data_auto, FieldValue};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let g = load_elements_data_auto(&std::fs::read(&a[1]).unwrap()).unwrap();
    let n: usize = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(10);
    let mut dist = std::collections::BTreeMap::new();
    for (i, r) in g.get(&a[2]).iter().enumerate() {
        let id = r.get("ID").and_then(|v| v.as_i32()).unwrap_or(0);
        let bytes = match r.get("_opaco") {
            Some(FieldValue::Raw(b)) => b.clone(),
            Some(FieldValue::Text(t)) => t.as_bytes().to_vec(),
            _ => vec![],
        };
        let ints: Vec<i32> = bytes.chunks(4).filter(|c| c.len() == 4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        *dist.entry(ints.clone()).or_insert(0) += 1;
        if i < n {
            println!("ID {id}: {:02x?} {ints:?}", bytes);
        }
    }
    println!("distintos: {}", dist.len());
    for (k, v) in dist.iter().take(12) {
        println!("  {k:?} ×{v}");
    }
}
