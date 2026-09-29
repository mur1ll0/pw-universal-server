//! Missões cujo modelo lido cita um número (em qualquer campo), pelo `Debug` do `TaskTemplate`.
//!
//! Uso: `cargo run -p pw-data-loader --example missoes_que_citam -- <tasks.data> <número>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let t = pw_data_loader::TasksData::load_from_bytes(&std::fs::read(&a[1]).unwrap()).unwrap();
    let alvo = &a[2];
    for (id, x) in &t.tasks {
        let d = format!("{x:?}");
        if d.contains(alvo.as_str()) {
            let i = d.find(alvo.as_str()).unwrap();
            let ini = d[..i].rfind(|c: char| c == '{' || c == ',').unwrap_or(0);
            println!("{id} {:?}: …{}…", x.name, &d[ini..(i + 40).min(d.len())]);
        }
    }
}
