//! B191: o `equip_mask` de cada família (`pw_data_loader::posicoes`) contra os `elements.data`
//! reais dos dois realms. Cada família só pode abrir os slots que o original lhe dá.

use pw_data_loader::generic_elements::{load_elements_data_auto, GenericElementsData};
use pw_data_loader::posicoes::{cabe_no_slot, carregar};
use std::path::PathBuf;

fn dados(realm: &str) -> Option<GenericElementsData> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(format!("data/{realm}/config/elements.data"));
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    Some(load_elements_data_auto(&bytes).expect("elements.data legível"))
}

fn slots(mascara: u32) -> Vec<usize> {
    (0..64).filter(|&s| cabe_no_slot(mascara, s)).collect()
}

fn conferir(realm: &str) {
    let Some(g) = dados(realm) else { return };
    let m = carregar(&g);
    let ids = |t: &str| g.get(t).iter().filter_map(|r| r.get("ID").and_then(|v| v.as_i32())).filter(|&i| i > 0)
        .map(|i| i as u32).collect::<Vec<_>>();
    // Arma: só o slot 0; munição: só o 11; voo: só o 12.
    for (tabela, esperado) in [("WEAPON_ESSENCE", vec![0]), ("PROJECTILE_ESSENCE", vec![11]),
                               ("FLYSWORD_ESSENCE", vec![12])] {
        let lista = ids(tabela);
        assert!(!lista.is_empty(), "{realm}: {tabela} vazia");
        assert!(lista.iter().all(|i| slots(m[i]) == esperado), "{realm}: {tabela} fora de {esperado:?}");
    }
    // Armadura: cabeça 1, manto 3, peito 4, cinto 5, pernas 6, pés 7, pulsos 8 — e, no 1.5.5,
    // 156 peças cujo subtipo abre o 22 (`EQUIP_INDEX_TWEAK`, `gs/item.h:218`; medido).
    let corpo = [1, 3, 4, 5, 6, 7, 8, 22];
    let armaduras = ids("ARMOR_ESSENCE");
    let com_slot = armaduras.iter().filter(|i| !slots(m[i]).is_empty()).count();
    assert!(com_slot * 10 >= armaduras.len() * 9, "{realm}: armaduras sem slot demais ({com_slot}/{})", armaduras.len());
    assert!(armaduras.iter().all(|i| slots(m[i]).iter().all(|s| corpo.contains(s))), "{realm}: armadura fora do corpo");
    // Acessório: colar 2, cinto 5, anéis 9–10.
    let acessorios = ids("DECORATION_ESSENCE");
    assert!(acessorios.iter().all(|i| slots(m[i]).iter().all(|s| [2, 5, 9, 10].contains(s))), "{realm}: acessório fora");
    // Roupa: só os slots de moda (13–16, 25, 29).
    let roupas = ids("FASHION_ESSENCE");
    assert!(roupas.iter().all(|i| slots(m[i]).iter().all(|s| [13, 14, 15, 16, 25, 29].contains(s))), "{realm}: roupa fora");
    let abertas = |t: &str| ids(t).iter().filter(|i| !slots(m[i]).is_empty()).count();
    println!("{realm}: {} itens com posição; armaduras {com_slot}/{}, acessórios {}/{}, roupas {}/{}",
             m.values().filter(|&&v| v != 0).count(), armaduras.len(),
             abertas("DECORATION_ESSENCE"), acessorios.len(), abertas("FASHION_ESSENCE"), roupas.len());
}

#[test]
fn as_posicoes_do_155_saem_do_elements() {
    conferir("realm_155");
}

#[test]
fn as_posicoes_do_126_saem_do_elements() {
    conferir("realm_126");
}
