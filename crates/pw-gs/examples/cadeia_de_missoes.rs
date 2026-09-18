//! Quem depende de uma missão: a continuação da cadeia, com os requisitos.
//!
//! `cargo run -p pw-gs --example cadeia_de_missoes -- data/realm_155/config 32201 31679`
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <ids...>");
    let alvos: Vec<u32> = a.filter_map(|s| s.parse().ok()).collect();
    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");
    for alvo in &alvos {
        if let Some(m) = t.get_task(*alvo) {
            println!("== {alvo} {:?}: npc {} → premia {} | nível {}..{} | classes {:?} | pré {:?} | filhos {:?}",
                m.name, m.npc_que_entrega, m.npc_que_premia, m.min_level, m.max_level, m.req_classes, m.pre_tasks, m.sub_tasks);
        } else {
            println!("== {alvo}: não existe no tasks.data");
        }
        let mut n = 0;
        for id in &t.de_topo {
            let Some(m) = t.get_task(*id) else { continue };
            if m.pre_tasks.contains(alvo) {
                println!("   continua em {id} {:?} npc {} nível {}..{} classes {:?} pré {:?}",
                    m.name, m.npc_que_entrega, m.min_level, m.max_level, m.req_classes, m.pre_tasks);
                n += 1;
            }
        }
        if n == 0 { println!("   (nenhuma missão de topo tem esta como pré-requisito)"); }
    }
}
