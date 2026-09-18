//! Todos os campos que o leitor traz de uma missão — para achar o gatilho dela.
//!
//! `cargo run -p pw-gs --example ficha_da_missao -- data/realm_155/config 31690`
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <ids...>");
    let ids: Vec<u32> = a.filter_map(|s| s.parse().ok()).collect();
    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");
    for id in ids {
        match t.get_task(id) {
            None => println!("{id}: não existe"),
            Some(m) => println!("{id} {:?}\n   {:#?}", m.name, m),
        }
    }
}
