//! B199: os bilhetes de troca de rosto (`FACETICKET_ESSENCE`) dos dois realms, com o nível.
use pw_data_loader::GameDataManager;

fn carregar(realm: &str) -> GameDataManager {
    let pasta = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config"));
    let mut d = GameDataManager::new();
    d.load_from_directory(&pasta);
    d
}

#[test]
fn os_dois_realms_tem_bilhetes_de_rosto() {
    for realm in ["realm_126", "realm_155"] {
        let d = carregar(realm);
        let mut ids: Vec<_> = d.bilhetes_de_rosto.iter().map(|(id, n)| (*id, *n)).collect();
        ids.sort();
        println!("{realm}: {} bilhetes, primeiros {:?}", ids.len(), &ids[..ids.len().min(5)]);
        assert!(!ids.is_empty(), "{realm} sem FACETICKET_ESSENCE");
        assert!(ids.iter().all(|(_, n)| (0..=150).contains(n)), "{realm}: require_level fora do esperado");
    }
}

/// B203: o `id_major_type` das armas é o `WeaponClass` que as passivas de arma conferem
/// (`cskill/skill/skillwrapper.h:54-66`): 1 espada, 5 lança, 9 machado, 13 arco, 182 punho, 292
/// varinha, 23749 adaga, 25333 talismã, 44878 cimitarra, 44879 foice. As das passivas de arma dos
/// dois realms (espada, lança, machado, arco, punho) existem como `tipo_maior`; o 291 (magia) não é
/// classe de passiva.
#[test]
fn a_classe_da_arma_e_o_tipo_maior() {
    let classes = [1, 5, 9, 13, 182];
    for realm in ["realm_126", "realm_155"] {
        let d = carregar(realm);
        let mut tipos: Vec<i32> = d.equipamentos.armas.values().map(|a| a.tipo_maior).collect();
        tipos.sort();
        tipos.dedup();
        println!("{realm}: tipos maiores das armas {tipos:?}");
        assert!(classes.iter().all(|c| tipos.contains(c)), "{realm}: falta classe de passiva entre os tipos");
    }
}
