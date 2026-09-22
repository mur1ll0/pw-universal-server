//! Lista as missões que exigem, entregam ou premiam um item.
//!
//! Uso: `cargo run -p pw-data-loader --example missoes_com_item -- <pasta config> <id do item>`
use pw_data_loader::tasks::TasksData;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let item: u32 = args[2].parse().unwrap();
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", args[1])).unwrap()).unwrap();
    let mut ids: Vec<_> = t.tasks.keys().copied().collect();
    ids.sort();
    for id in ids {
        let m = &t.tasks[&id];
        let entrega = m.itens_entregues.iter().any(|i| i.id == item);
        let premia = m.rewards.grupos_de_itens.iter().any(|g| g.itens.iter().any(|i| i.id == item));
        let exige = m.itens_exigidos.iter().any(|i| i.id == item);
        // O item que o monstro solta para a missão (`MONSTER_WANTED::m_ulItemDropped`) é um
        // quarto caminho, e é por ele que chega a maioria dos materiais de missão.
        let cai_de: Vec<String> = m
            .monster_kills
            .iter()
            .filter(|mo| mo.item_que_cai == item)
            .map(|mo| format!("monstro {} ×{} comum={}", mo.monstro, mo.quantidade_do_item, mo.item_comum))
            .collect();
        if entrega || premia || exige || !cai_de.is_empty() {
            println!(
                "{id} {:?} pai={:?} nivel {}-{} classes {:?} npc_entrega {} npc_premia {} entrega={entrega} premia={premia} exige={exige} cai_de={cai_de:?}",
                m.name, m.parent, m.min_level, m.max_level, m.req_classes, m.npc_que_entrega, m.npc_que_premia
            );
        }
    }
}
