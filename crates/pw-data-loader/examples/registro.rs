//! Todos os campos de um registro do `elements.data`: `cargo run -p pw-data-loader --example registro -- <config> <TABELA> <id>`.
use pw_data_loader::generic_elements::FieldValue;
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("config");
    let tabela = a.next().expect("tabela");
    let id: i32 = a.next().expect("id").parse().unwrap();
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&dir));
    let e = d.elements_generic.as_ref().expect("elements");
    for r in e.get(&tabela) {
        if r.get("ID").and_then(|v| v.as_i32()) == Some(id) {
            let mut k: Vec<_> = r.iter().collect();
            k.sort_by_key(|x| x.0.clone());
            for (n, v) in k {
                if !matches!(v, FieldValue::Raw(_)) {
                    println!("{n} = {v:?}");
                }
            }
        }
    }
}
