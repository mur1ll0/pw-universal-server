//! Missões cujo prêmio teleporta (`m_ulTransWldId` do `AWARD_DATA`), com nível e classes.
//!
//! Uso: `cargo run -p pw-data-loader --example missoes_com_teleporte -- <pasta config> [nivel_max]`
use pw_data_loader::TasksData;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let nivel_max: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(10);
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", args[1])).unwrap()).unwrap();
    let faixa: Option<(u32, u32)> = args.get(3).zip(args.get(4)).and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)));
    let mut v: Vec<_> = t.tasks.values().filter(|x| (x.rewards.teleporte.is_some() || x.teleporte_ao_receber.is_some()) && x.min_level <= nivel_max && faixa.is_none_or(|(a, b)| (a..=b).contains(&x.id))).collect();
    v.sort_by_key(|x| x.id);
    for x in v {
        println!("{} {:?} nivel {}-{} classes {:?} pai {:?} teleporte {:?} ao_receber {:?} npc_entrega {} npc_premia {} auto {}",
            x.id, x.name, x.min_level, x.max_level, x.req_classes, x.parent, x.rewards.teleporte, x.teleporte_ao_receber, x.npc_que_entrega, x.npc_que_premia, x.entrega_automatica);
    }
}
