//! As habilidades de um monstro (pelo id) e o roteiro de cada uma no catálogo da versão.
//!
//! Uso: `cargo run -p pw-data-loader --example habilidades_do_monstro -- <pasta config> <126|155> <tid>`
use pw_data_loader::{generic_elements::load_elements_data_auto, habilidades::TabelaDeHabilidades, monstros};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let el = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let mobs = monstros::carregar(&el, None);
    let t = if a[2] == "126" { TabelaDeHabilidades::do_126() } else { TabelaDeHabilidades::do_155() };
    let m = mobs.get(a[3].parse().unwrap()).expect("monstro");
    println!("{} ({}) nível {} dano mágico {:?}", m.nome, m.id, m.nivel, m.dano_magico);
    for s in &m.skills {
        println!("  skill {:?}", s);
        if let Some(h) = t.por_id.get(&(s.id as u32)) {
            println!("    tipo {:?} doenchant {} no_alvo {:?}", h.tipo, h.doenchant, h.no_alvo);
        }
    }
}
