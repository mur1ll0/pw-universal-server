//! Missões entregues pelos NPCs de uma pasta de mapa que teleportam para outro mundo.
//!
//! Uso: `cargo run -p pw-data-loader --example saidas_do_mapa -- <pasta config> <pasta do mapa>`
use pw_data_loader::{generic_elements::load_elements_data_auto, NpcGenData, TasksData};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let serv = pw_data_loader::servicos::carregar(&e);
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", a[1])).unwrap()).unwrap();
    let n = NpcGenData::load_from_bytes(&std::fs::read(format!("{}/{}/npcgen.data", a[1], a[2])).unwrap()).unwrap();
    let mut vistos = std::collections::BTreeSet::new();
    for s in &n.instances {
        let Some(sv) = serv.get(&s.template_id) else { continue };
        for &id in &sv.missoes_entregues {
            let Some(m) = t.get_task(id) else { continue };
            let destinos: Vec<_> = std::iter::once(m).chain(m.sub_tasks.iter().filter_map(|x| t.get_task(*x)))
                .flat_map(|x| [x.rewards.teleporte, x.teleporte_ao_receber]).flatten().collect();
            if !destinos.is_empty() && vistos.insert(id) {
                println!("NPC {} em {:?}: missão {} {:?} classes {:?} nível {}-{} → {:?}", s.template_id, s.pos, id, m.name, m.req_classes, m.min_level, m.max_level, destinos);
            }
        }
    }
}
