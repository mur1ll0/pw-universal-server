//! Mostra os serviços (missões e habilidades) de NPCs do `elements.data`.
//!
//! Uso: `cargo run -p pw-data-loader --example servicos_do_npc -- <pasta config> <id>...`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&std::fs::read(format!("{}/elements.data", args[1])).unwrap()).unwrap();
    let s = pw_data_loader::servicos::carregar(&e);
    for id in &args[2..] {
        println!("{id}: {:?}", s.get(&id.parse::<u32>().unwrap()));
    }
}
