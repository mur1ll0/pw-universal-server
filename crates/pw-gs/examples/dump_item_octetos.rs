//! Lê o bloco de octetos de um item do banco e mostra as propriedades adicionais dele.
//!
//! `cargo run -p pw-gs --example dump_item_octetos -- data/realm_155/config 267 <hex>`
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: dump_item_octetos <config> <item_id> <hex>");
    let item_id: u32 = a.next().expect("item_id").parse().expect("item_id");
    let hex = a.next().expect("hex");
    let octetos: Vec<u8> = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect();

    let bytes = std::fs::read(format!("{dir}/elements.data")).expect("elements.data");
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes).expect("elements.data");
    let tabelas = pw_data_loader::armaduras::TabelasDeEquipamento::carregar(&e);
    let addons = pw_data_loader::addons::TabelaDeAddons::carregar(&e);

    let Some(ficha) = tabelas.ficha(item_id) else {
        println!("{item_id}: sem ficha no elements.data");
        return;
    };
    match pw_core::ConteudoDeEquipamento::ler(&octetos, &ficha) {
        None => println!("{item_id}: os {} octetos não fecham no leitor", octetos.len()),
        Some(c) => {
            println!("{item_id}: durabilidade {}/{}, furos {:?}, {} addons", c.durabilidade, c.durabilidade_maxima, c.furos, c.addons.len());
            for x in &c.addons {
                let d = addons.por_id.get(&x.id());
                println!("   addon {} args {:?} → {}", x.id(), x.args, d.map(|d| d.tratador.as_str()).unwrap_or("SEM TRATADOR"));
            }
        }
    }
}
