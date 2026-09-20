//! O bloco de dados de um item de voo, em hexadecimal — para acertar item já entregue.
//!
//! `cargo run -p pw-gs --example octetos_do_voo -- data/realm_155/config 45782`
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <item_id>...");
    let bytes = std::fs::read(format!("{dir}/elements.data")).expect("elements.data");
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes).expect("elements.data");
    let mut d = pw_data_loader::GameDataManager::new();
    d.elements_generic = Some(e);
    for id in a.filter_map(|v| v.parse::<u32>().ok()) {
        match d.conteudo_do_item_de_voo(id) {
            Some(b) => println!("{id}: {} bytes\n{}", b.len(), b.iter().map(|x| format!("{x:02x}")).collect::<String>()),
            None => println!("{id}: não é FLYSWORD_ESSENCE"),
        }
    }
}
