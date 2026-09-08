//! Distribuição espacial dos spawns do npcgen.data.
use pw_data_loader::npcgen::{NpcGenData, SpawnType};
use std::collections::BTreeMap;
fn main() {
    let p = std::env::args().nth(1).expect("uso: dump_spawns <npcgen.data>");
    let d = NpcGenData::load_from_bytes(&std::fs::read(&p).unwrap()).unwrap();
    let mons: Vec<_> = d.instances.iter().filter(|i| i.spawn_type == SpawnType::Monster).collect();
    println!("instâncias de monstro: {}", mons.len());
    if mons.is_empty() { return; }
    let mut xs: Vec<f32> = mons.iter().map(|i| i.pos.x).collect();
    let mut zs: Vec<f32> = mons.iter().map(|i| i.pos.z).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("x: min {:.1} mediana {:.1} max {:.1}", xs[0], xs[xs.len()/2], xs[xs.len()-1]);
    println!("z: min {:.1} mediana {:.1} max {:.1}", zs[0], zs[zs.len()/2], zs[zs.len()-1]);
    // Quantas posições distintas? Agrupadas = poucas.
    let mut celas: BTreeMap<(i32, i32), usize> = BTreeMap::new();
    for i in &mons { *celas.entry((i.pos.x as i32 / 10, i.pos.z as i32 / 10)).or_default() += 1; }
    println!("células de 10m ocupadas: {}", celas.len());
    let mut top: Vec<_> = celas.iter().collect();
    top.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    println!("as 5 células mais cheias: {:?}", &top[..top.len().min(5)]);
    println!("\nprimeiros 8 spawns:");
    for i in mons.iter().take(8) {
        println!("  tid {:6} pos ({:.1}, {:.1}, {:.1})", i.template_id, i.pos.x, i.pos.y, i.pos.z);
    }
}
