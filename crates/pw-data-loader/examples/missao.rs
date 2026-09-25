//! Mostra o que o leitor tirou de uma missão (e das filhas): `cargo run -p pw-data-loader --example missao -- <config> <id>`.
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("config");
    let arg = a.next().expect("id ou trecho do nome");
    let t = pw_data_loader::TasksData::load_from_bytes(&std::fs::read(format!("{dir}/tasks.data")).unwrap()).unwrap();
    let Ok(id) = arg.parse::<u32>() else {
        for id in &t.de_topo {
            if let Some(m) = t.get_task(*id) {
                if m.name.to_lowercase().contains(&arg.to_lowercase()) {
                    println!("{id} {:?} auto {} nível {}..{} classes {:?} pré {:?} repete {} zona {} {:?}", m.name, m.entrega_automatica, m.min_level, m.max_level, m.req_classes, m.pre_tasks, m.pode_repetir, m.entrega_em_zona, m.regioes_de_entrega);
                }
            }
        }
        return;
    };
    let mut fila = vec![(id, 0)];
    while let Some((id, n)) = fila.pop() {
        let Some(m) = t.get_task(id) else { println!("{id} não existe"); continue };
        println!("{}{} {:?} método {} conclusão {} npc entrega {} premia {} itens pedidos {:?} monstros {:?} coleta {:?} lugares {:?} pai {:?}",
            "  ".repeat(n), id, m.name, m.metodo, m.tipo_de_conclusao, m.npc_que_entrega, m.npc_que_premia,
            m.itens_exigidos.iter().map(|i| (i.id, i.quantidade)).collect::<Vec<_>>(),
            m.monster_kills.iter().map(|k| (k.monstro, k.quantidade, k.item_que_cai)).collect::<Vec<_>>(),
            m.item_collections.iter().map(|i| (i.id, i.quantidade)).collect::<Vec<_>>(),
            m.lugares_a_alcancar, m.parent);
        for f in m.sub_tasks.iter().rev() { fila.push((*f, n + 1)); }
    }
}
