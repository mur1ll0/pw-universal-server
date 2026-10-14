//! Escritor do `elements.data` (B237) com os arquivos reais dos dois realms: ida e volta byte a
//! byte, edição de campo relida pelo leitor genérico, clone com id do espaço e recusas.

use pw_data_loader::elements_editavel::{espaco_de_id, ElementsEditavel, ErroDeEdicao};
use pw_data_loader::generic_elements::{load_elements_data, FieldValue};
use std::path::PathBuf;

fn arquivo(realm: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/elements.data"));
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e} (os dados do realm são obrigatórios)", p.display()))
}

#[test]
fn ida_e_volta_byte_a_byte_nos_dois_realms() {
    for (realm, versao, tabelas) in [("realm_126", 7, 118), ("realm_155", 156, 230)] {
        let buf = arquivo(realm);
        let e = ElementsEditavel::ler(&buf, None).unwrap();
        assert_eq!(e.versao, versao);
        assert_eq!(e.tabelas.len(), tabelas, "{realm}: tabelas fixas (o TALK_PROC vai cru)");
        assert!(e.gravar() == buf, "{realm}: gravar sem editar tem de devolver o arquivo idêntico");
    }
}

#[test]
fn editar_campo_clonar_e_remover_no_126() {
    let buf = arquivo("realm_126");
    let mut e = ElementsEditavel::ler(&buf, None).unwrap();
    let armas = e.tabela("WEAPON_ESSENCE").unwrap();
    let (n, id0) = (armas.registros.len(), armas.id(0).unwrap());

    // Campo de texto e numérico.
    e.definir_campo("WEAPON_ESSENCE", 0, "Name", &FieldValue::Text("Espada de Teste".into())).unwrap();
    e.definir_campo("WEAPON_ESSENCE", 0, "price", &FieldValue::Int(12345)).unwrap();
    // Tipo errado e texto que não cabe são recusados sem mexer no registro.
    assert!(e.definir_campo("WEAPON_ESSENCE", 0, "price", &FieldValue::Text("x".into())).is_err());
    assert!(e.definir_campo("WEAPON_ESSENCE", 0, "Name", &FieldValue::Text("x".repeat(500))).is_err());

    // Id: o próximo do espaço ESSENCE; repetido é recusado (o cliente sobrescreveria o outro).
    assert_eq!(espaco_de_id("WEAPON_ESSENCE").as_deref(), Some("ESSENCE"));
    let novo = e.proximo_id("WEAPON_ESSENCE").unwrap();
    let clone = e.clonar("WEAPON_ESSENCE", 0, novo).unwrap();
    assert_eq!(clone, n);
    let monstro = e.tabela("MONSTER_ESSENCE").unwrap().id(0).unwrap();
    assert!(matches!(e.clonar("WEAPON_ESSENCE", 0, monstro), Err(ErroDeEdicao::IdRepetido { .. })),
        "o id de um monstro é do mesmo espaço");
    assert!(matches!(e.definir_campo("WEAPON_ESSENCE", 1, "ID", &FieldValue::Int(id0)), Err(ErroDeEdicao::IdRepetido { .. })));

    // Relido pelo leitor genérico: o arquivo fecha no último byte e mostra as mudanças.
    let gravado = e.gravar();
    assert_eq!(gravado.len(), buf.len() + e.tabela("WEAPON_ESSENCE").unwrap().tamanho);
    let lido = load_elements_data(&gravado).unwrap();
    let armas = lido.get("WEAPON_ESSENCE");
    assert_eq!(armas.len(), n + 1);
    assert_eq!(armas[0]["Name"], FieldValue::Text("Espada de Teste".into()));
    assert_eq!(armas[0]["price"], FieldValue::Int(12345));
    assert_eq!(armas[n]["ID"], FieldValue::Int(novo));
    assert_eq!(armas[n]["Name"], FieldValue::Text("Espada de Teste".into()));
    // O resto do arquivo não mudou: mesma contagem em todas as outras tabelas.
    let original = load_elements_data(&buf).unwrap();
    for (nome, regs) in &original.tables {
        if nome != "WEAPON_ESSENCE" {
            assert_eq!(lido.get(nome).len(), regs.len(), "{nome}");
        }
    }

    // Remover o clone volta ao tamanho do arquivo original.
    e.remover("WEAPON_ESSENCE", clone).unwrap();
    assert_eq!(e.gravar().len(), buf.len());
}

#[test]
fn tabelas_sem_espaco_conhecido_nao_aceitam_registro_novo() {
    let buf = arquivo("realm_155");
    let mut e = ElementsEditavel::ler(&buf, None).unwrap();
    assert!(espaco_de_id("HOME_CONFIG").is_none());
    if !e.tabela("HOME_CONFIG").unwrap().registros.is_empty() {
        assert!(matches!(e.clonar("HOME_CONFIG", 0, 999_999), Err(ErroDeEdicao::EspacoDesconhecido(_))));
    }
    assert!(matches!(e.proximo_id("HOME_CONFIG"), Err(ErroDeEdicao::EspacoDesconhecido(_))));
}
