//! O que uma peça soma ao jogador pelo cálculo do GS (`Equipamento::dos_itens_com_addons`).
//!
//! `cargo run -p pw-gs --example bonus_do_equipamento -- <config> <item_id> <slot> <hex>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (dir, item_id, slot, hex) = (&a[1], a[2].parse::<u32>().unwrap(), a[3].parse::<u16>().unwrap(), &a[4]);
    let octetos: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap()).collect();
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&std::fs::read(format!("{dir}/elements.data")).unwrap()).unwrap();
    let tabelas = pw_data_loader::armaduras::TabelasDeEquipamento::carregar(&e);
    let addons = pw_data_loader::addons::TabelaDeAddons::carregar(&e);
    let item = pw_core::ItemRecord {
        id: None, character_id: 1, container_type: pw_core::ContainerType::Equipment, slot, item_id,
        count: 1, max_count: 1, refine_level: 0, sockets_count: 0, sockets: vec![], durability: 1, max_durability: 1,
        bind_status: 0, octets: octetos, custom_attributes: serde_json::json!({}),
    };
    let eq = pw_gs::entity::Equipamento::dos_itens_com_addons(&[item], &tabelas, Some(&addons));
    println!("{:#?}", eq.addons);
    println!("arma: {:?}", eq.arma);
}
