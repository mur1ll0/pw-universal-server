//! As Cartas de General do `realm_155`, lidas do `elements.data` real.
use pw_data_loader::cartas_de_general;
use pw_data_loader::generic_elements::load_elements_data_auto;

#[test]
fn a_caixa_de_tesouro_do_guerreiro_sorteia_cartas_que_existem() {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/realm_155/config/elements.data");
    let Ok(bytes) = std::fs::read(caminho) else {
        eprintln!("sem {caminho}: teste pulado");
        return;
    };
    let e = load_elements_data_auto(&bytes).expect("elements.data");
    let t = cartas_de_general::carregar(&e);
    // Item 41073, "Caixa de Tesouro do Guerreiro" (relato do Murillo, 2026-09-24).
    let caixa = t.caixas.get(&41073).expect("a caixa 41073 é um POKER_DICE_ESSENCE");
    let validas: Vec<&(u32, f32)> = caixa.cartas.iter().filter(|(id, p)| *id > 0 && *p > 0.0).collect();
    assert!(!validas.is_empty(), "a caixa não tem nenhuma carta com probabilidade");
    let soma: f32 = caixa.cartas.iter().map(|c| c.1).sum();
    assert!((soma - 1.0).abs() < 0.01, "as probabilidades somam {soma}");
    // Toda carta da caixa é um POKER_ESSENCE com subtipo — senão o original recusa o uso.
    for (id, _) in &validas {
        assert!(t.cartas.contains_key(id), "a carta {id} da caixa não é um POKER_ESSENCE com subtipo");
    }
    // O sorteio sempre devolve uma carta dessas.
    for k in 0..100 {
        let id = caixa.sortear(k as f32 / 100.0).expect("sorteio vazio");
        assert!(t.cartas.contains_key(&id));
    }
    eprintln!("{} caixas, {} cartas; a 41073 tem {} cartas", t.caixas.len(), t.cartas.len(), validas.len());
}
