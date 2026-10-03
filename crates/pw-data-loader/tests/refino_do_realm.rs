//! Refino, pedras e furos lidos do `elements.data` real dos dois realms (B163).
use pw_data_loader::addons::TabelaDeAddons;
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::refino::{self, Familia};

fn carregar(realm: &str) -> Option<pw_data_loader::generic_elements::GenericElementsData> {
    let caminho = format!("{}/../../data/{realm}/config/elements.data", env!("CARGO_MANIFEST_DIR"));
    let Ok(bytes) = std::fs::read(&caminho) else {
        eprintln!("sem {caminho}: teste pulado");
        return None;
    };
    Some(load_elements_data_auto(&bytes).expect("elements.data"))
}

fn conferir(realm: &str, com_tabela_de_furos: bool) {
    let Some(e) = carregar(realm) else { return };
    let d = refino::carregar(&e);
    let addons = TabelaDeAddons::carregar(&e);
    assert!(!d.pedras.is_empty(), "{realm}: nenhuma pedra");
    assert!(!d.talismas.is_empty(), "{realm}: nenhum talismã de refino");
    // Toda pedra tem preço de incrustar e põe addon em arma ou armadura.
    // As que não põem addon em arma nem armadura são só de acessório (`id_addon_decoration`, v156)
    // — e `IsStoneFit` com `combined_switch` 0 não as deixa entrar em nada.
    let so_de_acessorio: Vec<u32> = d.pedras.iter().filter(|(_, p)| p.addon_na_arma == 0 && p.addon_na_armadura == 0).map(|(id, _)| *id).collect();
    for id in &so_de_acessorio {
        let r = e.get("STONE_ESSENCE").iter().find(|r| r.get("ID").and_then(|v| v.as_i32()) == Some(*id as i32)).unwrap();
        let deco = r.get("id_addon_decoration").and_then(|v| v.as_i32()).unwrap_or(0);
        assert!(deco > 0, "{realm}: a pedra {id} não põe addon em nada");
    }
    // O addon de refino de todo equipamento refinável tem tratador `refine_*` — senão o porte
    // do `generate_addon` não o gera e o refino responderia sempre 92.
    let mut refinaveis = 0;
    for (id, eq) in &d.equipamentos {
        if eq.addon_de_refino == 0 {
            continue;
        }
        refinaveis += 1;
        let t = addons.por_id.get(&eq.addon_de_refino).map(|a| a.tratador.as_str());
        assert!(
            t.is_some_and(|t| t.starts_with("refine_")),
            "{realm}: o equipamento {id} refina com o addon {} de tratador {t:?}",
            eq.addon_de_refino
        );
    }
    assert!(refinaveis > 100, "{realm}: só {refinaveis} equipamentos refináveis");
    // Os materiais fixos do original existem como item.
    let existe = |id: i32| e.tables.values().flatten().any(|r| r.get("ID").and_then(|v| v.as_i32()) == Some(id));
    assert!(existe(11208), "{realm}: sem a Pedra Celestial 11208");
    // O material de furo só existe onde existe furar (o 1.2.6 não tem `make_slot_executor`).
    assert_eq!(existe(21043), com_tabela_de_furos, "{realm}: pedra de furo 21043");
    assert_eq!(!d.furos_de_acessorio.is_empty(), com_tabela_de_furos, "{realm}: tabela de furos de acessório");
    let armas = d.equipamentos.values().filter(|e| e.familia == Familia::Arma).count();
    eprintln!(
        "{realm}: {} pedras ({} só de acessório), {} talismãs, {refinaveis} refináveis ({armas} armas), {} graus de furo de acessório",
        d.pedras.len(),
        so_de_acessorio.len(),
        d.talismas.len(),
        d.furos_de_acessorio.len()
    );
}

#[test]
fn o_155_tem_pedras_talismas_e_refino_com_tratador() {
    conferir("realm_155", true);
}

#[test]
fn o_126_tem_pedras_talismas_e_refino_com_tratador() {
    conferir("realm_126", false);
}
