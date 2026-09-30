//! Missões cujo prêmio amplia a jaula de mascotes (`m_ulPetInventorySize`, B153), com a
//! cadeia até a raiz e os NPCs. Uso: `--example missoes_da_jaula -- <pasta config>`.
use pw_data_loader::tasks::TasksData;

fn main() {
    let dir = std::env::args().nth(1).expect("pasta config");
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{dir}/tasks.data")).unwrap()).unwrap();
    let mut ids: Vec<_> = t.tasks.keys().copied().collect();
    ids.sort();
    for id in ids {
        let m = &t.tasks[&id];
        if m.rewards.vagas_na_jaula == 0 {
            continue;
        }
        let mut cadeia = vec![];
        let mut p = m.parent;
        while let Some(pid) = p {
            let Some(pm) = t.tasks.get(&pid) else { break };
            cadeia.push(format!("{pid} {:?} (nível {}, entrega NPC {}, premia NPC {})", pm.name, pm.min_level, pm.npc_que_entrega, pm.npc_que_premia));
            p = pm.parent;
        }
        println!(
            "{id} {:?} → {} vagas | nível {} | entrega NPC {} premia NPC {} | pais: {:?}",
            m.name, m.rewards.vagas_na_jaula, m.min_level, m.npc_que_entrega, m.npc_que_premia, cadeia
        );
    }
}
