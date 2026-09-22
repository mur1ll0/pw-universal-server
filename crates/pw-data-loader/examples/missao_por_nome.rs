//! Procura missões pelo nome (trecho, sem diferenciar maiúsculas).
//!
//! Uso: `cargo run -p pw-data-loader --example missao_por_nome -- <pasta config> <trecho>`
use pw_data_loader::tasks::TasksData;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let alvo = args[2].to_lowercase();
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", args[1])).unwrap()).unwrap();
    let mut ids: Vec<_> = t.tasks.keys().copied().collect();
    ids.sort();
    for id in ids {
        let m = &t.tasks[&id];
        if !m.name.to_lowercase().contains(&alvo) {
            continue;
        }
        println!(
            "{id} {:?} nivel {}-{} metodo={} monstros={:?} invocados={:?}",
            m.name,
            m.min_level,
            m.max_level,
            m.metodo,
            m.monster_kills.iter().map(|k| (k.monstro, k.quantidade, k.item_que_cai)).collect::<Vec<_>>(),
            m.rewards.monstros_invocados
        );
    }
}
