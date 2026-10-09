//! B205: quantas criaturas de cada mapa têm `bAutoRevive` falso (não renascem), e quais.
//!
//! Uso: `cargo run -p pw-data-loader --example sem_renascer -- <pasta config do realm> [tid...]`
use pw_data_loader::npcgen::{NpcGenData, SpawnType};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let filtro: Vec<u32> = a[2..].iter().filter_map(|v| v.parse().ok()).collect();
    let mut pastas: Vec<_> = std::fs::read_dir(&a[1]).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.join("npcgen.data").exists()).collect();
    pastas.sort();
    for p in pastas {
        let Ok(d) = NpcGenData::load_from_bytes(&std::fs::read(p.join("npcgen.data")).unwrap()) else {
            println!("{}: npcgen ilegível", p.file_name().unwrap().to_string_lossy());
            continue;
        };
        let todas: Vec<_> = d.instances.iter().chain(d.controladores.values().flat_map(|c| c.pendentes.iter())).collect();
        let mons: Vec<_> = todas.iter().copied().filter(|i| matches!(i.spawn_type, SpawnType::Monster | SpawnType::Npc)).collect();
        let sem = mons.iter().filter(|i| !i.renasce).count();
        let minas = todas.iter().copied().filter(|i| i.spawn_type == SpawnType::ResourceMine).collect::<Vec<_>>();
        let minas_sem = minas.iter().filter(|i| !i.renasce).count();
        println!("{}: criaturas {} (sem renascer {}), minas {} (sem renascer {})", p.file_name().unwrap().to_string_lossy(), mons.len(), sem, minas.len(), minas_sem);
        for i in mons.iter().filter(|i| filtro.contains(&i.template_id)) {
            println!("   tid {} renasce {} pos ({:.0},{:.0})", i.template_id, i.renasce, i.pos.x, i.pos.z);
        }
    }
}
