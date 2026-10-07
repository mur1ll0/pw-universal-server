//! B194 — edição livre do item pelo painel contra o `elements.data` real dos dois realms: o
//! bloco editado se relê inteiro (`ConteudoDeEquipamento::ler`, o `SetItemInfo` do cliente),
//! a essência fica byte a byte, e cada campo sai onde o cliente o lê. Sem a pasta do realm o
//! teste avisa e não verifica nada.

use pw_core::{ContainerType, ConteudoDeEquipamento, ItemRecord, Requisitos};
use pw_data_loader::refino::Familia;
use pw_data_loader::GameDataManager;
use pw_gs::bus_server::item_editado::{aplicar, EdicaoDeItem, EfeitoEditado, RequisitosEditados};
use std::path::PathBuf;

fn realm(nome: &str) -> Option<GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{nome}/config"));
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    Some(d)
}

/// O trecho da essência: depois do cabeçalho (22 bytes + tag) e antes do rabo.
fn essencia(bloco: &[u8]) -> Vec<u8> {
    let nome = bloco[23] as usize;
    let tamanho = i16::from_le_bytes([bloco[20], bloco[21]]) as usize;
    bloco[24 + nome..24 + nome + tamanho].to_vec()
}

fn conferir(nome: &str) {
    let Some(d) = realm(nome) else { return };
    let mut armas: Vec<u32> = d.refino.equipamentos.iter().filter(|(_, e)| e.familia == Familia::Arma).map(|(id, _)| *id).collect();
    armas.sort();
    let arma = *armas.iter().find(|id| d.equipamentos.ficha(**id).is_some()).expect("uma arma refinável com ficha");
    let grau = d.refino.equipamentos[&arma].grau;
    let mut pedras: Vec<u32> = d.refino.pedras.iter().filter(|(_, p)| p.grau <= grau).map(|(id, _)| *id).collect();
    pedras.sort();
    let pedra = *pedras.first().expect("uma pedra que cabe");
    let ficha = d.equipamentos.ficha(arma).unwrap();
    let mut item = ItemRecord::new(1, ContainerType::Inventory, 0, arma, 1);
    item.octets = ConteudoDeEquipamento::novo(ficha.clone(), 5000, 6000).escrever();
    let antes = essencia(&item.octets);

    let e = EdicaoDeItem {
        quantidade: Some(3),
        durabilidade: Some(100),
        durabilidade_maxima: Some(99_900),
        requisitos: Some(RequisitosEditados { nivel: 7, classes: 0x0003, forca: 11, agilidade: 22, vitalidade: 33, energia: 44 }),
        fabricante: Some("Murillo".into()),
        efeitos: Some(vec![EfeitoEditado { id: 999, args: vec![5] }]),
        refino: Some(5),
        pedras: Some(vec![pedra, 0]),
    };
    aplicar(&mut item, &e, &d).unwrap_or_else(|c| panic!("{nome}: {c}"));
    let c = ConteudoDeEquipamento::ler(&item.octets, &ficha).expect("o bloco editado se relê inteiro");
    assert_eq!(essencia(&item.octets), antes, "{nome}: a essência fica byte a byte");
    assert_eq!((item.count, c.durabilidade, c.durabilidade_maxima, item.max_durability), (3, 100, 99_900, 99_900));
    let r = Requisitos::do_bloco(&item.octets).unwrap();
    assert_eq!((r.nivel, r.classes, r.forca, r.agilidade, r.vitalidade, r.energia), (7, 3, 11, 22, 33, 44), "{nome}: requisitos");
    assert_eq!(c.fabricante, pw_core::nome_do_fabricante("Murillo"));
    assert_eq!(c.furos, vec![pedra as i32, 0]);
    assert_eq!((item.refine_level, item.sockets.clone()), (5, vec![pedra, 0]), "{nome}: colunas espelho");
    let addon_de_refino = d.refino.equipamentos[&arma].addon_de_refino;
    assert_eq!(pw_gs::refino::nivel_de_refino(&c.addons, addon_de_refino), 5);
    assert!(c.addons.iter().any(|a| a.id() == 999 && a.args == vec![5]), "{nome}: efeito livre");
    assert!(c.addons.iter().any(|a| a.tipo & 0x8000 != 0), "{nome}: a pedra pôs efeito embutido");

    // Tirar refino e pedras volta ao rabo sem eles; nada muda quando a edição é recusada.
    let mut volta = item.clone();
    aplicar(&mut volta, &EdicaoDeItem { refino: Some(0), pedras: Some(vec![]), ..Default::default() }, &d).unwrap();
    let c = ConteudoDeEquipamento::ler(&volta.octets, &ficha).unwrap();
    assert!(c.furos.is_empty() && c.addons.iter().all(|a| a.id() != addon_de_refino && a.tipo & 0x8000 == 0));
    let copia = item.clone();
    assert_eq!(aplicar(&mut item, &EdicaoDeItem { pedras: Some(vec![999_999]), ..Default::default() }, &d), Err("pedra_inexistente"));
    assert_eq!(item, copia, "{nome}: recusa não muda nada");
}

#[test]
fn editar_item_no_155() {
    conferir("realm_155");
}

#[test]
fn editar_item_no_126() {
    conferir("realm_126");
}
