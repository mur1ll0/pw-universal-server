//! `CHARRACTER_CLASS_CONFIG` — os atributos por classe que alimentam a precisão e a
//! evasão base do jogador.

use pw_data_loader::classes::{self, TOTAL_DE_CLASSES};
use std::path::PathBuf;

fn carregar() -> Option<classes::TabelaDeClasses> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/realm_155/config/elements.data");
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes)
        .expect("elements.data do realm_155 deveria ser legível");
    Some(classes::carregar(&e))
}

#[test]
fn as_doze_classes_do_155_sao_lidas() {
    let Some(t) = carregar() else { return };
    assert_eq!(t.len(), TOTAL_DE_CLASSES, "o 1.5.5 tem 12 classes");
    for cls in 0..TOTAL_DE_CLASSES as i32 {
        assert!(t.get(cls).is_some(), "faltou a classe {cls}");
    }
}

#[test]
fn a_precisao_e_a_evasao_base_sao_agilidade_vezes_a_constante_da_classe() {
    // `GetBasicAttackRate(cls, agi) = agi_attack[cls] * agi`, idem para a armadura.
    let Some(t) = carregar() else { return };
    for cls in 0..TOTAL_DE_CLASSES as i32 {
        let c = t.get(cls).unwrap();
        assert_eq!(c.precisao_base(30), c.ataque_por_agilidade * 30);
        assert_eq!(c.evasao_base(30), c.armadura_por_agilidade * 30);
        // Zero de agilidade dá zero: não há parcela fixa nesta base.
        assert_eq!(c.precisao_base(0), 0);
        assert_eq!(c.evasao_base(0), 0);
    }
}

#[test]
fn as_constantes_por_classe_sao_diferentes_entre_si() {
    // Se o layout estivesse deslocado, os doze registros sairiam iguais ou absurdos.
    // O que se afirma é que existe variação real e que os valores são plausíveis.
    let Some(t) = carregar() else { return };

    let ataques: Vec<i32> = (0..TOTAL_DE_CLASSES as i32)
        .map(|c| t.get(c).unwrap().ataque_por_agilidade)
        .collect();
    let armaduras: Vec<i32> = (0..TOTAL_DE_CLASSES as i32)
        .map(|c| t.get(c).unwrap().armadura_por_agilidade)
        .collect();

    assert!(ataques.iter().all(|v| (1..=50).contains(v)), "ataque por agilidade: {ataques:?}");
    assert!(armaduras.iter().all(|v| (1..=50).contains(v)), "armadura por agilidade: {armaduras:?}");

    let distintos: std::collections::BTreeSet<_> = ataques.iter().collect();
    assert!(distintos.len() > 1, "todas as classes com o mesmo agi_attack: {ataques:?}");

    // O Guerreiro (classe 0) é a referência corpo a corpo e tem de estar entre os
    // maiores; o Mago (classe 1) entre os menores. É a checagem que pega troca de coluna
    // sem depender de nenhum valor exato.
    let guerreiro = t.get(0).unwrap().ataque_por_agilidade;
    let mago = t.get(1).unwrap().ataque_por_agilidade;
    assert!(guerreiro > mago, "guerreiro {guerreiro} deveria ter mais precisão que mago {mago}");
    assert!(
        t.get(0).unwrap().armadura_por_agilidade > t.get(1).unwrap().armadura_por_agilidade
    );
}

#[test]
fn a_velocidade_de_ataque_vira_ticks_com_piso_de_30() {
    // `(int)(attack_speed * 20)`, e o original força 30 quando dá zero ou menos.
    let Some(t) = carregar() else { return };
    for cls in 0..TOTAL_DE_CLASSES as i32 {
        let c = t.get(cls).unwrap();
        let ticks = c.ataque_em_ticks();
        assert!(ticks > 0, "classe {cls} com ataque em {ticks} ticks");
        assert!(ticks <= 256, "classe {cls} com ataque lento demais: {ticks}");
    }
}

#[test]
fn sem_a_tabela_no_arquivo_a_carga_e_vazia() {
    // 1.2.6/v7: o leitor genérico não cobre, e quem consulta tem de saber lidar com isso.
    let vazio = pw_data_loader::generic_elements::GenericElementsData {
        version: 7,
        tables: std::collections::HashMap::new(),
    };
    assert!(classes::carregar(&vazio).is_empty());
}

/// A velocidade de corrida sai daqui, e não do `ptemplate.conf`.
///
/// O original lê os dois arquivos e o `elements.data` **sobrescreve** o `.conf`:
/// `player_template::__LoadDataFromDataMan` (`gs/playertemplate.cpp:250-301`) grava
/// `walk_speed`, `run_speed`, `swim_speed`, `flight_speed`, `attack_speed`, `attack_range`,
/// `hp_gen` e `mp_gen` do `CHARRACTER_CLASS_CONFIG` por cima do que o `.conf` pôs.
///
/// Até 2026-09-12 o servidor lia o `.conf`, e o Bárbaro corria a 2,8 m/s. Estes números são
/// os mesmos que a captura do servidor 1.2.6 funcional traz no `OWN_EXT_PROP`
/// (`_sync/capturas/full_interno.pcap`): andar 2,0, correr 4,9, nadar 3,0, voar 5,0.
#[test]
fn as_velocidades_do_barbaro_sao_as_do_elements_e_nao_as_do_ptemplate() {
    for realm in ["realm_155BR", "realm_155"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(format!("data/{realm}/config/elements.data"));
        let Ok(bytes) = std::fs::read(&p) else {
            eprintln!("pulado: {} não existe", p.display());
            continue;
        };
        let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes)
            .expect("elements.data legível");
        let t = classes::carregar(&e);
        let barbaro = t.get(4).expect("o Bárbaro (4) tem de estar no CHARRACTER_CLASS_CONFIG");

        assert_eq!(barbaro.velocidade_correndo, 4.9, "{realm}: run_speed do Bárbaro");
        assert_eq!(barbaro.velocidade_andando, 2.0, "{realm}: walk_speed");
        assert_eq!(barbaro.velocidade_nadando, 3.0, "{realm}: swim_speed");
        assert_eq!(barbaro.velocidade_voando, 5.0, "{realm}: fly_speed");
        // `attack_speed` 0,8 s vira 16 ticks — e **não** os 30 do `ptemplate.conf`.
        assert_eq!(barbaro.ataque_em_ticks(), 16, "{realm}: attack_speed em ticks");
    }
}
