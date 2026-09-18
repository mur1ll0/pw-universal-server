//! As minas do 155 montadas com as regras de `npcgenerator.cpp:1280-1365`.
use pw_data_loader::generic_elements::load_elements_data_auto;
use std::path::PathBuf;

#[test]
fn a_raiz_seca_sai_com_os_limites_do_original() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config/elements.data");
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return;
    };
    let e = load_elements_data_auto(&bytes).unwrap();
    let minas = pw_data_loader::minas::carregar(&e);
    assert!(minas.len() > 100, "só {} minas válidas", minas.len());

    let m = &minas[&3074];
    assert_eq!((m.quantidade, m.quantidade_bonus), (1, 2));
    assert!((m.chance_bonus - 0.1).abs() < 1e-6);
    assert_eq!((m.tempo_minimo, m.tempo_maximo, m.nivel), (5, 10, 1));
    assert_eq!((m.exp, m.sp), (10, 5));
    assert_eq!(m.distancia, 4.0, "gather_dist 0 sobe para o piso de 4 m");
    assert_eq!(m.coletores, 1);
    assert_eq!(m.materiais.len(), 1);
    assert_eq!(m.materiais[0].item, 795);
    // Toda mina válida tem probabilidades somando 1 e distância no intervalo do original.
    for m in minas.values() {
        let soma: f32 = m.materiais.iter().map(|x| x.probabilidade).sum();
        assert!((soma - 1.0).abs() < 1e-5, "mina {}", m.id);
        assert!((4.0..=20.0).contains(&m.distancia));
    }
}
