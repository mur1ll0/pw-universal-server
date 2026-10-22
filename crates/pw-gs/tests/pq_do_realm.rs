//! B348 — as PQ do `realm_155` iniciam e andam com as variáveis globais.

use pw_data_loader::GameDataManager;
use pw_gs::pq::GerenciadorDePq;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn as_pq_do_realm_iniciam_e_andam() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let mut globais = HashMap::new();
    let (mut g, controles) = GerenciadorDePq::iniciar(&d.tasks, 1000, &mut globais);
    eprintln!("realm_155: {} PQ iniciadas, {} controladores", g.pqs.len(), controles.len());
    assert_eq!(g.pqs.len(), 25);
    // A Trincheira Apodrecida (20759): a etapa 20760 acaba com a global 100202 > 199.
    assert_eq!(g.pqs[&20759].etapa, 20760);
    g.pqs.get_mut(&20759).unwrap().entrar(1, 0, &globais);
    globais.insert(100202, 200);
    let s = g.batimento(&d.tasks, 1005, &mut globais, |_| 0);
    assert_eq!(g.pqs[&20759].etapa, 20761, "andou para a próxima etapa: {s:?}");
}
