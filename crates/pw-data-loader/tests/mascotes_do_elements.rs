//! B197 — nome e ícone dos modelos de mascote (`PET_ESSENCE`) contra o atlas `IconList_Pet` do
//! cliente 1.5.5 (`data/icones/iconlist_pet.txt`). Sem os arquivos o teste avisa e não verifica.

use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::servicos::nomes_e_icones_da_tabela;
use std::path::PathBuf;

fn conferir(realm: &str) {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (Ok(bytes), Ok(atlas)) = (std::fs::read(raiz.join(format!("data/{realm}/config/elements.data"))),
                                   std::fs::read(raiz.join("data/icones/iconlist_pet.txt"))) else {
        eprintln!("AVISO: sem elements.data do {realm} ou sem o atlas — este teste NÃO verificou nada.");
        return;
    };
    let g = load_elements_data_auto(&bytes).expect("elements.data legível");
    let mascotes = nomes_e_icones_da_tabela(&g, "PET_ESSENCE");
    let celulas: std::collections::HashSet<Vec<u8>> = atlas.split(|b| *b == b'\n').skip(4)
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l).to_ascii_lowercase()).collect();
    let com_nome = mascotes.values().filter(|(n, _)| !n.is_empty()).count();
    let no_atlas = mascotes.values().filter(|(_, i)| celulas.contains(&i.to_ascii_lowercase())).count();
    println!("{realm}: {} modelos; {com_nome} com nome; {no_atlas} com ícone no atlas", mascotes.len());
    assert!(!mascotes.is_empty(), "{realm}: PET_ESSENCE vazia");
    assert!(no_atlas * 10 >= mascotes.len() * 8, "{realm}: menos de 80% dos ícones no atlas");
}

#[test]
fn mascotes_do_155() {
    conferir("realm_155");
}

#[test]
fn mascotes_do_126() {
    conferir("realm_126");
}
