//! Que missões um personagem poderia pegar, pelos dados do realm.
//!
//! `cargo run -p pw-gs --example missoes_disponiveis -- data/realm_155/config 5 6 18918,31676,...`
//! (config, nível, classe, concluídas separadas por vírgula)
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <nivel> <classe> [concluidas]");
    let nivel: u32 = a.next().expect("nivel").parse().unwrap();
    let classe: u32 = a.next().expect("classe").parse().unwrap();
    let concluidas: Vec<u32> = a
        .next()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_default();

    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");

    let mut aptas = Vec::new();
    let mut por_nivel = 0usize;
    let mut por_classe = 0usize;
    let mut por_pre = 0usize;
    for id in &t.de_topo {
        let Some(m) = t.get_task(*id) else { continue };
        if m.min_level > nivel || (m.max_level > 0 && m.max_level < nivel) { por_nivel += 1; continue }
        if !m.req_classes.is_empty() && !m.req_classes.contains(&classe) { por_classe += 1; continue }
        if concluidas.contains(id) && !m.pode_repetir { continue }
        if !m.pre_tasks.is_empty() && !m.pre_tasks.iter().all(|p| concluidas.contains(p)) { por_pre += 1; continue }
        aptas.push((*id, m.name.clone(), m.npc_que_entrega, m.min_level, m.max_level, m.pre_tasks.clone()));
    }
    println!("missões de topo: {} | recusadas por nível {por_nivel}, por classe {por_classe}, por missão anterior {por_pre}", t.de_topo.len());
    println!("aptas para nível {nivel}, classe {classe}: {}", aptas.len());
    for (id, nome, npc, mn, mx, pre) in aptas.iter().take(25) {
        println!("  {id} {nome:?} npc {npc} nível {mn}..{mx} pré {pre:?}");
    }
}
