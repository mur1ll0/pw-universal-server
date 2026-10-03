//! Serviços de NPC lidos do `elements.data` real (B164: item de missão pelo NPC, serviço 8).
use pw_data_loader::generic_elements::load_elements_data_auto;

#[test]
fn npcs_com_item_de_missao_nos_dois_realms() {
    for realm in ["realm_155", "realm_126"] {
        let caminho = format!("{}/../../data/{realm}/config/elements.data", env!("CARGO_MANIFEST_DIR"));
        let Ok(bytes) = std::fs::read(&caminho) else {
            eprintln!("sem {caminho}: pulado");
            continue;
        };
        let e = load_elements_data_auto(&bytes).expect("elements");
        let s = pw_data_loader::servicos::carregar(&e);
        let com: Vec<_> = s.values().filter(|v| !v.missoes_com_item.is_empty()).collect();
        let entradas: usize = com.iter().map(|v| v.missoes_com_item.len()).sum();
        // Missão listada sem item existe no dado; o original não entrega nada (`actual_num == 0`).
        let sem_item = com.iter().flat_map(|v| v.itens_de_missao.iter()).filter(|x| x.0 != 0 && x.1[0].0 == 0).count();
        eprintln!("{realm}: {} NPCs com o serviço 8, {entradas} missões ({sem_item} sem item)", com.len());
        assert!(!com.is_empty(), "{realm}: nenhum NPC com o serviço 8");
    }
}
