//! Missões com `m_bDeathTrig` (oferecidas quando o jogador morre).
//!
//! Uso: `cargo run -p pw-data-loader --example missoes_ao_morrer -- <pasta config>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let t = pw_data_loader::TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", a[1])).unwrap()).unwrap();
    let mut v: Vec<_> = t.tasks.values().filter(|x| x.entrega_ao_morrer).collect();
    v.sort_by_key(|x| x.id);
    println!("{} missões", v.len());
    for x in v {
        println!("{} {:?} pai={:?} nivel {}-{} pre={:?} auto={}", x.id, x.name, x.parent, x.min_level, x.max_level, x.pre_tasks, x.entrega_automatica);
    }
}
