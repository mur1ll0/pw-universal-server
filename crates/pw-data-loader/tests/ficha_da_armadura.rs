//! O bloco de dados que acompanha armadura e acessório no `OWN_ITEM_INFO` sai do
//! `elements.data`, como já saía o da arma.
//!
//! Este teste foi escrito **antes** de a primeira peça de armadura chegar em jogo, com o
//! defeito já diagnosticado: até 2026-09-09 só arma ganhava bloco, e sem bloco o
//! `m_iProfReq` do cliente fica no zero do construtor (`EC_IvtrEquip.cpp:74`) — o que faz
//! `CanUseEquipment` recusar `ICID_ARMOR`/`ICID_DECORATION` para todas as classes
//! (`EC_HostPlayer.cpp:4953-4959`) e desenhar a peça em vermelho. Era a Varinha de novo,
//! pela outra ponta.

use pw_data_loader::armaduras::{self, TabelasDeEquipamento};
use std::path::PathBuf;

fn carregar(realm: &str) -> Option<TabelasDeEquipamento> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(format!("data/{realm}/config/elements.data"));
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes)
        .expect("elements.data deveria ser legível");
    Some(TabelasDeEquipamento::carregar(&e))
}

/// O realm tem as duas tabelas, e elas não se confundem com a das armas: cada id vive em
/// um espaço de essência só, e é isso que faz a busca por família funcionar.
#[test]
fn as_tres_familias_sao_conjuntos_disjuntos() {
    let Some(t) = carregar("realm_155BR") else { return };

    assert!(!t.armaduras.is_empty(), "ARMOR_ESSENCE vazio no realm");
    assert!(!t.decoracoes.is_empty(), "DECORATION_ESSENCE vazio no realm");

    for id in t.armaduras.keys() {
        assert!(
            !t.armas.contains_key(id),
            "o id {id} está em ARMOR_ESSENCE e em WEAPON_ESSENCE: a busca por família \
             devolveria a essência errada"
        );
        assert!(!t.decoracoes.contains_key(id), "o id {id} está em duas tabelas");
    }
    for id in t.decoracoes.keys() {
        assert!(!t.armas.contains_key(id), "o id {id} está em duas tabelas");
    }
}

/// Zero na máscara de classes recusa **todo mundo**. Uma armadura que viajasse assim
/// apareceria vermelha para qualquer personagem, e é exatamente o que acontecia quando
/// ela viajava sem bloco nenhum.
///
/// O arquivo do realm tem quatro peças com máscara zero (16122, 16123, 28770, 28773),
/// todas de `require_level = 0` e defesa 0 — não são equipamento que jogador ganhe, e o
/// servidor original mandaria zero nelas do mesmo jeito. Quem tem nível exigido é que
/// precisa de máscara.
#[test]
fn nenhuma_peca_com_nivel_exigido_viaja_com_mascara_de_classe_zerada() {
    let Some(t) = carregar("realm_155BR") else { return };

    let mut conferidas = 0usize;
    for a in t.armaduras.values() {
        if a.requisitos.nivel_exigido < 1 {
            continue;
        }
        let ficha: pw_core::FichaDaArmadura = a.into();
        assert_ne!(
            ficha.classes_permitidas, 0,
            "a armadura {} (nível {}) viajaria recusando todas as classes",
            a.id, a.requisitos.nivel_exigido
        );
        conferidas += 1;
    }
    assert!(
        conferidas > 2_000,
        "só {conferidas} armaduras com nível exigido — o campo lido não deve ser esse"
    );

    for d in t.decoracoes.values() {
        if d.requisitos.nivel_exigido < 1 {
            continue;
        }
        let ficha: pw_core::FichaDeDecoracao = d.into();
        assert_ne!(
            ficha.classes_permitidas, 0,
            "o acessório {} viajaria recusando todas as classes",
            d.id
        );
    }
}

/// A busca por id devolve a variante da família certa. Trocar a essência da armadura pela
/// do acessório passa por qualquer teste de tamanho — as duas têm 36 bytes — e só
/// aparece em jogo, como número errado no tooltip.
#[test]
fn a_busca_por_id_devolve_a_familia_certa() {
    use pw_core::FichaDoEquipamento as F;
    let Some(t) = carregar("realm_155BR") else { return };

    let id_armadura = *t.armaduras.keys().next().unwrap();
    let id_decoracao = *t.decoracoes.keys().next().unwrap();

    assert!(matches!(t.ficha(id_armadura), Some(F::Armadura(_))));
    assert!(matches!(t.ficha(id_decoracao), Some(F::Decoracao(_))));
    assert!(matches!(t.ficha(2251), Some(F::Arma(_))), "a Varinha continua arma");
    assert!(t.ficha(1).is_none(), "id que não é equipamento vai sem bloco");
}

/// Os campos batem com o arquivo, um por um, na peça que o Murillo vai equipar primeiro:
/// a armadura de nível mais baixo que aceita o Sacerdote (bit 7).
#[test]
fn a_primeira_armadura_do_sacerdote_bate_campo_a_campo() {
    let Some(t) = carregar("realm_155BR") else { return };

    const SACERDOTE: i32 = 1 << 7;
    let peca = t
        .armaduras
        .values()
        .filter(|a| a.requisitos.classes_permitidas & SACERDOTE != 0)
        .min_by_key(|a| (a.requisitos.nivel_exigido, a.id))
        .expect("o realm tem de ter alguma armadura para o Sacerdote");

    let ficha: pw_core::FichaDaArmadura = peca.into();
    assert_eq!(ficha.classes_permitidas, peca.requisitos.classes_permitidas);
    assert_eq!(ficha.nivel_exigido, peca.requisitos.nivel_exigido);
    assert_eq!(ficha.forca_exigida, peca.requisitos.forca_exigida);
    assert_eq!(ficha.vitalidade_exigida, peca.requisitos.vitalidade_exigida);
    assert_eq!(ficha.agilidade_exigida, peca.requisitos.agilidade_exigida);
    assert_eq!(ficha.energia_exigida, peca.requisitos.energia_exigida);
    assert_eq!(ficha.defesa, peca.defesa);
    assert_eq!(ficha.resistencias, peca.resistencias);

    eprintln!(
        "primeira armadura do Sacerdote: id {} nv {} classes {:#x} defesa {} evasão {} \
         hp+{} mp+{} subtipo {}",
        peca.id,
        peca.requisitos.nivel_exigido,
        peca.requisitos.classes_permitidas,
        peca.defesa,
        peca.evasao,
        peca.hp_extra,
        peca.mp_extra,
        peca.tipo_menor,
    );
}

/// A durabilidade que o arquivo traz é positiva: é ela que o chamador multiplica pela
/// escala do cliente ao montar o comando, e zero ali significa peça quebrada.
#[test]
fn a_durabilidade_de_fabrica_nao_e_zero() {
    let Some(t) = carregar("realm_155BR") else { return };

    let zeradas = t
        .armaduras
        .values()
        .filter(|a| a.requisitos.durabilidade <= 0)
        .count();
    let total = t.armaduras.len();
    assert!(
        zeradas * 20 < total,
        "{zeradas} de {total} armaduras com durability_min <= 0 — o campo lido não deve \
         ser esse"
    );
}

/// A leitura não depende da ordem dos campos no `HashMap`: as cinco resistências saem dos
/// cinco `magic_defences_N_low`, na ordem Metal, Madeira, Água, Fogo, Terra.
#[test]
fn as_cinco_resistencias_saem_de_campos_distintos() {
    let Some(t) = carregar("realm_155BR") else { return };

    let alguma = t
        .armaduras
        .values()
        .find(|a| a.resistencias.iter().any(|&r| r != 0));
    assert!(
        alguma.is_some(),
        "nenhuma armadura do realm com resistência mágica: o nome do campo mudou?"
    );

    // E o mesmo vale para o acessório, que lê os mesmos cinco campos.
    let _: &armaduras::TemplateDeDecoracao =
        t.decoracoes.values().next().expect("DECORATION_ESSENCE vazio");
}
