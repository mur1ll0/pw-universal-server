//! O conteúdo da roupa (`generate_fashion_item`) com o `elements.data` de cada realm: a roupa
//! feminina leva `gender` 1 (B129, a Tsuko via "Masculino" no maiô comprado na Loja Gold).
use std::path::PathBuf;

fn dados(realm: &str) -> Option<pw_data_loader::GameDataManager> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
    if !p.join("elements.data").exists() {
        eprintln!("sem {realm}: teste pulado");
        return None;
    }
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(&p);
    Some(d)
}

#[test]
fn o_maio_feminino_leva_gender_1_nos_dois_realms() {
    for realm in ["realm_126", "realm_155"] {
        let Some(d) = dados(realm) else { continue };
        let c = d.conteudo_da_roupa(15770, 0x1234).expect("15770 é FASHION_ESSENCE");
        assert_eq!(c.len(), 10, "{realm}: int + u16 + u16 + etiqueta de 2 B");
        assert_eq!(i32::from_le_bytes([c[0], c[1], c[2], c[3]]), 5, "{realm}: require_level");
        assert_eq!(u16::from_le_bytes([c[4], c[5]]), 0x1234, "{realm}: cor");
        assert_eq!(u16::from_le_bytes([c[6], c[7]]), 1, "{realm}: gender feminino");
        assert!(d.conteudo_da_roupa(12503, 0).is_none(), "{realm}: arma não é roupa");
    }
}

/// A captura (B130): o Gato de Presas Afiadas (3316) dá o ovo 10765 nos dois realms, e o ovo
/// tem conteúdo de mascote (`get_item_for_sell` do `GM_MSG_MOB_BE_TRAINED`).
#[test]
fn o_gato_de_presas_afiadas_da_o_ovo_10765_com_conteudo() {
    for realm in ["realm_126", "realm_155"] {
        let Some(d) = dados(realm) else { continue };
        let gato = d.monstros.get(3316).expect("3316 é monstro");
        assert_eq!(gato.ovo_de_captura, 10765, "{realm}");
        assert!(d.eh_ovo_de_pet(10765), "{realm}: 10765 é PET_EGG_ESSENCE");
        assert!(d.gerar_octetos_do_ovo(10765).is_some_and(|o| !o.is_empty()), "{realm}");
    }
}
