//! O leitor genérico contra os `elements.data` reais dos dois realms 1.5.5.
//!
//! O leitor já recusa um arquivo que não termine exatamente no último byte, então
//! "carregou" quer dizer "o layout de cada uma das tabelas está certo". O que estes testes
//! acrescentam: as contagens de tabelas conferidas por conteúdo, e o conteúdo de algumas.
//!
//! História que vale guardar: até 2026-09-12 o leitor não conhecia os dois blocos de `tag`
//! que o `elementdataman::load_data` do cliente pula (depois de `ARMORRUNE_ESSENCE` e de
//! `WAR_TANKCALLIN_ESSENCE`), e dez remendos de `skip`/`count`/posição absoluta por arquivo
//! compensavam. O v156 do `realm_155BR`, que é o que o docker serve, lia só 99 das 231
//! tabelas por causa disso — incluindo `MINE_ESSENCE` (o que cada recurso dá) e
//! `PLAYER_ACTION_INFO_CONFIG`.

use pw_data_loader::generic_elements::load_elements_data;
use std::path::PathBuf;

fn ler(realm: &str) -> Option<Vec<u8>> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(realm)
        .join("config/elements.data");
    match std::fs::read(&p) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("pulado: {} não existe", p.display());
            None
        }
    }
}

fn nome(r: &pw_data_loader::generic_elements::Record) -> String {
    r.get("Name").and_then(|v| v.as_text()).unwrap_or_default().to_string()
}

/// O v156 do `realm_155BR`: as 231 tabelas, com as que antes vinham vazias.
#[test]
fn o_v156_do_155br_le_as_231_tabelas() {
    let Some(bytes) = ler("realm_155BR") else { return };
    let d = load_elements_data(&bytes).expect("v156 do 155BR deve fechar no último byte");

    assert_eq!(d.version, 156);
    assert_eq!(d.tables.len(), 231);
    // O mesmo total que `specs/elements_layouts/pw_elements_reader.py` dá neste arquivo.
    assert_eq!(d.tables.values().map(|v| v.len()).sum::<usize>(), 69_640);
    let vazias = d.tables.values().filter(|v| v.is_empty()).count();
    assert!(vazias <= 5, "{vazias} tabelas vazias (eram 132)");

    for (tabela, n) in [
        ("EQUIPMENT_ADDON", 2977),
        ("WEAPON_ESSENCE", 2741),
        ("SKILLTOME_SUB_TYPE", 22),
        ("MONSTER_ESSENCE", 8054),
        ("NPC_ESSENCE", 4761),
        ("TALK_PROC", 3391),
        ("FACE_HAIR_ESSENCE", 430),
        ("CHARRACTER_CLASS_CONFIG", 12),
        ("PLAYER_ACTION_INFO_CONFIG", 1354),
        ("MINE_ESSENCE", 1557),
        ("FASHION_ESSENCE", 3016),
        ("PET_TYPE", 6),
        ("RED_PACKET_PAPER_ESSENCE", 5),
    ] {
        assert_eq!(d.get(tabela).len(), n, "{tabela}");
    }

    // O primeiro registro depois de cada bloco de tag, lido pelo nome.
    assert_eq!(nome(&d.get("SKILLTOME_SUB_TYPE")[0]), "Guerreiro");
    assert!(nome(&d.get("NPC_WAR_TOWERBUILD_SERVICE")[0]).contains("Construir torre"));
    assert_eq!(nome(&d.get("MINE_TYPE")[0]), "Tronco");
}

/// O v159 do `realm_155` (cópia do `elements.data` do cliente EN): as 234 tabelas.
#[test]
fn o_v159_do_155_le_as_234_tabelas() {
    let Some(bytes) = ler("realm_155") else { return };
    let d = load_elements_data(&bytes).expect("v159 do 155 deve fechar no último byte");

    assert_eq!(d.version, 159);
    assert_eq!(d.tables.len(), 234);
    assert_eq!(d.tables.values().map(|v| v.len()).sum::<usize>(), 70_067);
    for (tabela, n) in [
        ("EQUIPMENT_ADDON", 2992),
        ("CHARRACTER_CLASS_CONFIG", 12),
        ("TALK_PROC", 3391),
        ("PET_TYPE", 6),
        ("ASTROLABE_APPEARANCE_CONFIG", 1),
        ("EQUIP_MAKE_HOLE_CONFIG", 1),
    ] {
        assert_eq!(d.get(tabela).len(), n, "{tabela}");
    }
    assert_eq!(nome(&d.get("SKILLTOME_SUB_TYPE")[0]), "Blade.");
}

/// Um byte a mais no fim é erro, não uma tabela a mais lida torta.
#[test]
fn arquivo_que_nao_fecha_no_ultimo_byte_e_recusado() {
    let Some(mut bytes) = ler("realm_155BR") else { return };
    bytes.push(0);
    let e = load_elements_data(&bytes).unwrap_err().to_string();
    assert!(e.contains("terminaram no offset"), "{e}");
}
