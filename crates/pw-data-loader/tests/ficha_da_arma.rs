//! O bloco de dados que acompanha uma arma no `OWN_ITEM_INFO` sai do `elements.data`.
//!
//! Em jogo, de 2026-09-06 a 2026-09-09, a Varinha do Sacerdote aparecia vermelha e
//! inutilizável em toda sessão. A causa: o bloco era montado por uma tabela chumbada de
//! quatro ids no codificador, e a Varinha (2251) caía num genérico que declarava
//! `weapon_type = 1` (`WEAPONTYPE_RANGE`). O cliente concluía que era arma de munição,
//! não achava flecha e recusava (`CanUseEquipment`, razão 5).

use pw_data_loader::armas::{self, CORPO_A_CORPO, LONGO_ALCANCE};
use std::path::PathBuf;

fn carregar(realm: &str) -> Option<armas::TabelaDeArmas> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(format!("data/{realm}/config/elements.data"));
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes)
        .expect("elements.data deveria ser legível");
    Some(armas::carregar(&e))
}

#[test]
fn a_varinha_do_sacerdote_nao_e_arma_de_municao() {
    let Some(t) = carregar("realm_155") else { return };

    let varinha = t.get(&2251).expect("a Varinha (2251) tem de estar na tabela");
    assert_eq!(
        varinha.tipo_de_arma(),
        CORPO_A_CORPO,
        "declarar a Varinha como longo alcance faz o cliente cobrar munição e recusar a arma"
    );
    assert_eq!(varinha.municao_exigida, 0, "a Varinha não usa munição");
    assert_eq!(varinha.tipo_maior, 292, "tipo maior Magia — é o que as skills do Sacerdote exigem");

    // Bit 7 é o Sacerdote. Zero aqui recusaria todas as classes.
    assert_ne!(varinha.classes_permitidas & (1 << 7), 0, "o Sacerdote tem de poder equipar");
    assert_eq!(varinha.forca_exigida, 5);
    assert_eq!(varinha.energia_exigida, 3);
    assert_eq!(varinha.nivel_exigido, 1);
}

#[test]
fn o_arco_continua_sendo_arma_de_municao() {
    let Some(t) = carregar("realm_155") else { return };

    let arco = t.get(&2250).expect("o Arco de Madeira (2250) tem de estar na tabela");
    assert_eq!(
        arco.tipo_de_arma(),
        LONGO_ALCANCE,
        "o arco é o caso em que o cliente **deve** cobrar munição"
    );
    assert_ne!(arco.municao_exigida, 0, "o arco exige flecha");
}

/// A ficha que viaja não pode ter zero na máscara de classes: zero recusa todo mundo.
#[test]
fn nenhuma_arma_inicial_viaja_com_mascara_de_classe_zerada() {
    let Some(t) = carregar("realm_155") else { return };

    for id in [2097u32, 2250, 2251, 2258, 26331, 26332, 44937, 45020] {
        let arma = t.get(&id).unwrap_or_else(|| panic!("arma inicial {id} não existe"));
        let ficha: pw_core::FichaDaArma = arma.into();
        assert_ne!(
            ficha.classes_permitidas, 0,
            "a arma {id} viajaria recusando todas as classes"
        );
        assert_eq!(ficha.nivel_exigido, arma.nivel_exigido);
        assert_eq!(ficha.tipo_de_arma, arma.tipo_de_arma());
    }
}

/// A correlação que sustenta [`TemplateDeArma::tipo_de_arma`]: arma de munição é arma de
/// `short_range_mode == 0`. Se um realm futuro quebrar isso, é aqui que se descobre.
#[test]
fn arma_de_municao_e_arma_de_modo_de_alcance_zero() {
    let Some(t) = carregar("realm_155") else { return };

    let com_municao = t.values().filter(|a| a.municao_exigida != 0).count();
    let concordam = t
        .values()
        .filter(|a| a.municao_exigida != 0 && a.modo_de_alcance == 0)
        .count();

    assert!(com_municao > 100, "poucas armas de munição no arquivo: {com_municao}");
    let taxa = concordam as f32 / com_municao as f32;
    assert!(
        taxa > 0.99,
        "só {concordam} de {com_municao} armas de munição têm modo de alcance 0 ({taxa:.3})"
    );
}
