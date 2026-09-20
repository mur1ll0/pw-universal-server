//! Quais missões concedem nível de cultivo (`m_ulNewPeriod` do prêmio).
//!
//! `cargo run -p pw-gs --example missoes_de_cultivo -- data/realm_155/config`
fn main() {
    let dir = std::env::args().nth(1).expect("uso: <config>");
    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");
    let mut n = 0;
    let mut ids: Vec<u32> = t.tasks.keys().copied().collect();
    ids.sort();
    for id in ids {
        let Some(m) = t.get_task(id) else { continue };
        if m.rewards.novo_cultivo == 0 {
            continue;
        }
        n += 1;
        println!(
            "{id} {:?} → cultivo {} | nível {}..{} | npc {} → {} | pré {:?}",
            m.name, m.rewards.novo_cultivo, m.min_level, m.max_level, m.npc_que_entrega, m.npc_que_premia, m.pre_tasks
        );
    }
    println!("\n{n} missões concedem cultivo");
}
