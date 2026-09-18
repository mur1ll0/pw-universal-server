//! Alcance de golpe, ódio e visão de alguns monstros do realm.
//!
//! `cargo run -p pw-data-loader --example dump_alcance -- data/realm_155/config 44582 44584`
fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("uso: dump_alcance <config> [ids...]");
    let ids: Vec<u32> = args.filter_map(|s| s.parse().ok()).collect();
    let bytes = std::fs::read(format!("{dir}/elements.data")).expect("elements.data");
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes).expect("elements.data legível");
    let t = pw_data_loader::monstros::carregar(&e, None);
    for id in ids {
        match t.get(id) {
            Some(m) => println!(
                "{id}: {:?} nível {} vida {} alcance {:.2} ódio {:.2} visão {} tamanho {:.2}",
                m.nome, m.nivel, m.vida, m.alcance_de_ataque, m.raio_de_odio, m.raio_de_visao, m.tamanho
            ),
            None => println!("{id}: sem template"),
        }
    }
}
