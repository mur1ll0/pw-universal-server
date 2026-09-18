//! O que cada NPC entrega, com nível e pré-requisito.
//!
//! `cargo run -p pw-gs --example missoes_do_npc -- data/realm_155/config 44396 44390`
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <npc_tids...>");
    let npcs: Vec<u32> = a.filter_map(|s| s.parse().ok()).collect();
    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");
    for npc in &npcs {
        println!("== NPC {npc}");
        for id in &t.de_topo {
            let Some(m) = t.get_task(*id) else { continue };
            if m.npc_que_entrega == *npc {
                println!("   {id} {:?} nível {}..{} classes {:?} pré {:?} repetível {}",
                    m.name, m.min_level, m.max_level, m.req_classes, m.pre_tasks, m.pode_repetir);
            }
        }
    }
}
