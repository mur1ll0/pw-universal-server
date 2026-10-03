//! NPCs do `elements.data` que entregam ou recebem as missões dadas (serviço de missão).
//!
//! Uso: `cargo run -p pw-data-loader --example npcs_da_missao -- <pasta config> <id da missão>...`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&std::fs::read(format!("{}/elements.data", args[1])).unwrap()).unwrap();
    let s = pw_data_loader::servicos::carregar(&e);
    for m in &args[2..] {
        let m: u32 = m.parse().unwrap();
        let mut entrega: Vec<u32> = s.iter().filter(|(_, v)| v.missoes_entregues.contains(&m)).map(|(k, _)| *k).collect();
        let mut recebe: Vec<u32> = s.iter().filter(|(_, v)| v.missoes_recebidas.contains(&m)).map(|(k, _)| *k).collect();
        entrega.sort();
        recebe.sort();
        println!("{m}: entregam {entrega:?} recebem {recebe:?}");
    }
}
