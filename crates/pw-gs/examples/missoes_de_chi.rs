//! Quais missões concedem teto de chi (`m_ulFuryULimit` do prêmio).
//!
//! `cargo run -p pw-gs --example missoes_de_chi -- data/realm_155/config`
fn main() {
    let dir = std::env::args().nth(1).expect("uso: <config>");
    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");
    let mut ids: Vec<u32> = t.tasks.keys().copied().collect();
    ids.sort();
    let mut n = 0;
    for id in ids {
        let Some(m) = t.get_task(id) else { continue };
        if m.rewards.teto_de_chi == 0 {
            continue;
        }
        n += 1;
        println!(
            "{id} {:?} → teto {} | nível {}..{} | npc {} → {}",
            m.name, m.rewards.teto_de_chi, m.min_level, m.max_level, m.npc_que_entrega, m.npc_que_premia
        );
    }
    println!("\n{n} missões concedem teto de chi");
}
