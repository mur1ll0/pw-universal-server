//! Que missões de **entrega automática** o cliente pediria, e por que as outras não.
//!
//! O cliente varre `m_AutoDelvMap` a cada tique (`ATaskTemplMan::CheckAutoDelv`,
//! `Task/TaskTemplMan.cpp:106-131`) e só pede as que passam no `CheckPrerequisite`; a zona
//! de entrega é conferida **ali**, com a posição dele (`CheckInZone`, `TaskTempl.inl:368-393`).
//!
//! `cargo run -p pw-gs --example missoes_automaticas -- data/realm_155/config 5 6 161 -773.9 34.8 -160.6 18918,26830,...`
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <nivel> <classe> <mundo> <x> <y> <z> [concluidas]");
    let nivel: u32 = a.next().expect("nivel").parse().unwrap();
    let classe: u32 = a.next().expect("classe").parse().unwrap();
    let mundo: u32 = a.next().expect("mundo").parse().unwrap();
    let pos = [
        a.next().and_then(|v| v.parse().ok()).unwrap_or(0.0f32),
        a.next().and_then(|v| v.parse().ok()).unwrap_or(0.0f32),
        a.next().and_then(|v| v.parse().ok()).unwrap_or(0.0f32),
    ];
    let concluidas: Vec<u32> = a.next().map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();

    let bytes = std::fs::read(format!("{dir}/tasks.data")).expect("tasks.data");
    let t = pw_data_loader::TasksData::load_from_bytes(&bytes).expect("tasks.data");

    let (mut auto, mut prontas) = (0, 0);
    for id in &t.de_topo {
        let Some(m) = t.get_task(*id) else { continue };
        if !m.entrega_automatica {
            continue;
        }
        auto += 1;
        let mut por_que = Vec::new();
        if m.min_level > nivel || (m.max_level > 0 && m.max_level < nivel) {
            por_que.push(format!("nível {}..{}", m.min_level, m.max_level));
        }
        if !m.req_classes.is_empty() && !m.req_classes.contains(&classe) {
            por_que.push(format!("classes {:?}", m.req_classes));
        }
        if concluidas.contains(id) && !m.pode_repetir {
            por_que.push("já concluída".into());
        }
        if !m.pre_tasks.is_empty() && !m.pre_tasks.iter().all(|p| concluidas.contains(p)) {
            por_que.push(format!("falta {:?}", m.pre_tasks));
        }
        if m.entrega_em_zona {
            let dentro = mundo == m.mundo_de_entrega && m.regioes_de_entrega.iter().any(|r| r.contem(pos));
            if !dentro {
                por_que.push(format!(
                    "fora da zona (mundo {}, {} região(ões))",
                    m.mundo_de_entrega,
                    m.regioes_de_entrega.len()
                ));
                for r in &m.regioes_de_entrega {
                    por_que.push(format!("   x {}..{} z {}..{}", r.min[0], r.max[0], r.min[2], r.max[2]));
                }
            }
        }
        if por_que.is_empty() {
            prontas += 1;
            println!("PEDE  {id} {:?}", m.name);
        } else if por_que.len() <= 3 && !por_que[0].starts_with("nível") && !por_que[0].starts_with("classes") {
            println!("não   {id} {:?} — {}", m.name, por_que.join("; "));
        }
    }
    println!("\n{auto} missões de entrega automática no tasks.data; {prontas} passariam nos critérios daqui");
    println!("(aqui só conferimos nível, classe, pré-requisito e zona; o cliente aplica mais:");
    println!(" frequência, itens, reputação, gênero, exclusivas. O total acima é um teto.)");
}
