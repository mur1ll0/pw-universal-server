//! A arma inicial de cada classe tem que ser do tipo maior que as habilidades iniciais
//! daquela classe aceitam — senão o jogador não consegue conjurar nenhuma delas.
//!
//! `ElementSkill::Condition` (`EvolvedPWClient/ElementSkill/ElementSkill.cpp:195`) abre
//! com `if (!ValidWeapon(info.weapon)) return 1;`, e `ValidWeapon` (`skill.h:179`) é uma
//! **lista branca**: o `restrict_weapons` do stub da habilidade. O cliente preenche
//! `info.weapon` com o `GetDBMajorType()->id` da arma equipada
//! (`EC_HostPlayer.cpp:6146-6153`), ou 0 quando não há arma nenhuma.
//!
//! Em jogo, 2026-09-07: os sacerdotes nasciam com o "Graveto de Madeira" (2867), tipo
//! maior 5 (Acha), e as habilidades 113/125 do Sacerdote só aceitam 292 (Magia) ou 0.
//! Resultado: nenhuma habilidade conjurava, e o relato foi "não está sendo possível usar
//! nenhum skill". Este teste amarra `default_weapon_id()` ao `elements.data` de verdade
//! para o erro não voltar em silêncio.

use pw_core::CharacterClass;
use pw_data_loader::generic_elements::{load_elements_data_auto, GenericElementsData};
use std::path::PathBuf;

const CLASSES: [CharacterClass; 12] = [
    CharacterClass::Blademaster,
    CharacterClass::Wizard,
    CharacterClass::Psychomancer,
    CharacterClass::Venomancer,
    CharacterClass::Barbarian,
    CharacterClass::Assassin,
    CharacterClass::Archer,
    CharacterClass::Cleric,
    CharacterClass::Seeker,
    CharacterClass::Mystic,
    CharacterClass::Duskblade,
    CharacterClass::Stormbringer,
];

fn carregar(realm: &str) -> Option<GenericElementsData> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(format!("data/{realm}/config/elements.data"));
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    Some(load_elements_data_auto(&bytes).expect("elements.data deveria ser legível"))
}

fn campo(e: &GenericElementsData, tabela: &str, id: i32, nome: &str) -> Option<i32> {
    e.get(tabela)
        .iter()
        .find(|r| r.get("ID").and_then(|v| v.as_i32()) == Some(id))
        .and_then(|r| r.get(nome))
        .and_then(|v| v.as_i32())
}

#[test]
fn armas_iniciais_batem_com_o_elements() {
    let Some(e) = carregar("realm_155") else { return };

    let tipos_conhecidos: Vec<i32> = e
        .get("WEAPON_MAJOR_TYPE")
        .iter()
        .filter_map(|r| r.get("ID").and_then(|v| v.as_i32()))
        .collect();
    assert!(
        tipos_conhecidos.len() >= 11,
        "WEAPON_MAJOR_TYPE veio com {} tipos, esperava os 11+ do 1.5.5",
        tipos_conhecidos.len()
    );

    for cls in CLASSES {
        let arma = cls.default_weapon_id();
        let esperado = cls.weapon_major_type();

        assert!(
            tipos_conhecidos.contains(&esperado),
            "{cls:?}: tipo maior {esperado} não existe no WEAPON_MAJOR_TYPE"
        );

        let tipo = campo(&e, "WEAPON_ESSENCE", arma, "id_major_type")
            .unwrap_or_else(|| panic!("{cls:?}: arma inicial {arma} não existe no WEAPON_ESSENCE"));
        assert_eq!(
            tipo, esperado,
            "{cls:?}: a arma inicial {arma} é do tipo maior {tipo}, mas as habilidades \
             iniciais da classe só aceitam {esperado} — nenhuma habilidade conjuraria"
        );

        // Um personagem recém-criado é nível 1 e tem 10 em cada atributo (o padrão do
        // schema). Uma arma acima disso ficaria equipada e inútil.
        let nivel = campo(&e, "WEAPON_ESSENCE", arma, "require_level").unwrap_or(0);
        assert!(nivel <= 1, "{cls:?}: arma inicial {arma} exige nível {nivel}");
        for atributo in ["require_strength", "require_agility", "require_energy"] {
            let v = campo(&e, "WEAPON_ESSENCE", arma, atributo).unwrap_or(0);
            assert!(v <= 10, "{cls:?}: arma inicial {arma} exige {atributo} = {v}");
        }
    }
}

#[test]
fn a_arma_de_antes_do_sacerdote_era_mesmo_do_tipo_errado() {
    // Guarda o achado: se algum dia alguém "voltar" o 2867 para o Sacerdote, este teste
    // explica por que aquilo quebra em jogo.
    let Some(e) = carregar("realm_155") else { return };

    let graveto = campo(&e, "WEAPON_ESSENCE", 2867, "id_major_type")
        .expect("o Graveto de Madeira (2867) deveria existir");
    assert_eq!(graveto, 5, "o Graveto de Madeira é do tipo maior 5 (Acha)");
    assert_ne!(
        graveto,
        CharacterClass::Cleric.weapon_major_type(),
        "era isso que travava as habilidades do Sacerdote"
    );

    let varinha = campo(&e, "WEAPON_ESSENCE", 2251, "id_major_type")
        .expect("a Varinha (2251) deveria existir");
    assert_eq!(varinha, 292, "a Varinha é do tipo maior 292 (Magia)");
}
