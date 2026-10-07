//! B186: o índice de nomes de itens (`servicos::nomes_de_itens`) lê o campo `Name` dos
//! `elements.data` reais do 1.2.6 (v7) e do 1.5.5 (v156). Sem o arquivo, o teste não roda.
//! Medido em 2026-10-06: 126 7.896 de 7.896 com nome; 155 24.806 de 26.188 (o próprio arquivo
//! traz 1.382 itens com `Name` vazio — o painel mostra "Item <id>").
use std::path::PathBuf;

fn conferir(realm: &str) {
    let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/elements.data"));
    let Ok(buf) = std::fs::read(&caminho) else { return };
    let g = pw_data_loader::generic_elements::load_elements_data_auto(&buf).expect("elements.data legível");
    let nomes = pw_data_loader::servicos::nomes_de_itens(&g);
    let pilhas = pw_data_loader::servicos::pilhas(&g);
    assert_eq!(nomes.len(), pilhas.len(), "{realm}: um nome por item");
    let com_nome = nomes.values().filter(|n| !n.is_empty()).count();
    assert!(nomes.len() > 1000 && com_nome * 100 >= nomes.len() * 90, "{realm}: {com_nome} de {} com nome", nomes.len());
    assert!(nomes.values().all(|n| !n.contains('\0')), "{realm}: nome com NUL");
    println!("{realm}: {} itens, {com_nome} com nome", nomes.len());
}

#[test]
fn os_itens_do_126_e_do_155_tem_nome() {
    conferir("realm_126");
    conferir("realm_155");
}
