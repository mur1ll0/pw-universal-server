//! O `ptemplate.conf` — os atributos base por classe.
//!
//! Metade sintética (o formato e as recusas) e metade contra o arquivo real do realm 155,
//! que é o que pega seção renomeada ou campo que sumiu entre versões.

use pw_data_loader::ptemplate::{self, SECOES_DE_CLASSE};
use std::path::PathBuf;

/// Um arquivo mínimo, com as doze seções e só os campos que o leitor exige.
fn conf_minimo() -> String {
    let mut s = String::from("[GENERAL]\nlogic_level_limit = 105\n\n");
    for (i, secao) in SECOES_DE_CLASSE.iter().enumerate() {
        s.push_str(&format!(
            "[{secao}]\n\
             hp = {}\nmp = {}\nvitality = 20\nenergy = 5\nstrength = 15\nagility = 10\n\
             attack_speed = 30\nattack_range = 1.4\nhp_gen = 3\nmp_gen = 1\n\
             walk_speed = 1.5\nrun_speed = 3.0\nswim_speed = 2.2\nfly_speed = 3.0\n\n",
            60 + i as i32,
            20 + i as i32
        ));
    }
    s
}

#[test]
fn le_as_doze_secoes_na_ordem_do_enum() {
    // A ordem é a lista literal de `player_template::__LoadData`: SWORDSMAN é a classe 0,
    // FAIRY é a 11. Trocar duas aqui daria atributo de mago para guerreiro sem erro
    // nenhum, então o teste amarra o índice ao valor.
    let t = ptemplate::ler(&conf_minimo()).expect("o arquivo mínimo deveria ser válido");
    assert_eq!(t.len(), 12);
    assert_eq!(t.nivel_maximo, Some(105));
    for i in 0..12 {
        let c = t.get(i).unwrap_or_else(|| panic!("faltou a classe {i}"));
        assert_eq!(c.classe, i);
        assert_eq!(c.vida, 60 + i, "a classe {i} pegou a vida de outra seção");
        assert_eq!(c.mana, 20 + i);
    }
}

#[test]
fn aceita_tabulacao_e_comentario() {
    // O arquivo original é alinhado com tabulações e tem comentários com `#`.
    let texto = conf_minimo().replace(" = ", "\t=\t");
    let texto = format!("# comentário\n; outro\n{texto}");
    let t = ptemplate::ler(&texto).expect("tabulação e comentário não deveriam atrapalhar");
    assert_eq!(t.len(), 12);
}

#[test]
fn secao_ausente_e_erro() {
    // Um realm com onze classes carregaria em silêncio e a décima segunda ficaria sem
    // atributo nenhum — daí ser erro, não aviso.
    let texto = conf_minimo().replace("[FAIRY]", "[OUTRACOISA]");
    let e = ptemplate::ler(&texto).unwrap_err();
    assert!(format!("{e}").contains("FAIRY"), "erro pouco específico: {e}");
}

#[test]
fn campo_ausente_e_erro() {
    // O original lança exceção em `ReadInt` quando o campo não existe.
    let texto = conf_minimo().replacen("hp = 60\n", "", 1);
    let e = ptemplate::ler(&texto).unwrap_err();
    let msg = format!("{e}");
    assert!(msg.contains("SWORDSMAN") && msg.contains("hp"), "erro pouco específico: {msg}");
}

#[test]
fn valor_nao_numerico_e_erro() {
    let texto = conf_minimo().replacen("hp = 60", "hp = muito", 1);
    let e = ptemplate::ler(&texto).unwrap_err();
    assert!(format!("{e}").contains("muito"), "o erro devia mostrar o valor: {e}");
}

#[test]
fn le_o_arquivo_real_do_realm_155() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/realm_155/config/ptemplate.conf");
    let Ok(texto) = std::fs::read_to_string(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return;
    };
    let t = ptemplate::ler(&texto).expect("o ptemplate.conf do realm 155 deveria ser legível");
    assert_eq!(t.len(), 12);

    // O guerreiro do pacote 1.5.5: 60 de vida, 20 de mana, 20 de vitalidade.
    let guerreiro = t.get(0).unwrap();
    assert_eq!((guerreiro.vida, guerreiro.mana), (60, 20));
    assert_eq!(guerreiro.vitalidade, 20);

    // As doze classes têm valores plausíveis, e há variação real entre elas — arquivo mal
    // lido daria tudo igual ou tudo absurdo.
    for i in 0..12 {
        let c = t.get(i).unwrap();
        assert!((10..=200).contains(&c.vida), "classe {i} com {} de vida", c.vida);
        assert!((10..=200).contains(&c.mana), "classe {i} com {} de mana", c.mana);
        assert!(c.velocidade_correndo > 1.0 && c.velocidade_correndo < 10.0);
        // `attack_speed` aqui está em ticks de 50 ms, não em segundos — 30 = 1,5 s.
        assert!((10..=200).contains(&c.ataque_em_ticks), "classe {i}: {}", c.ataque_em_ticks);
    }
    let vidas: std::collections::BTreeSet<i32> = (0..12).map(|i| t.get(i).unwrap().vida).collect();
    assert!(vidas.len() > 1, "todas as classes com a mesma vida base");

    // O mago tem menos vida e mais mana que o guerreiro — é o par mais distante, e a
    // checagem pega troca de seção sem depender de valor exato.
    let mago = t.get(1).unwrap();
    assert!(mago.vida < guerreiro.vida, "mago {} contra guerreiro {}", mago.vida, guerreiro.vida);
    assert!(mago.mana > guerreiro.mana);
}

#[test]
fn le_o_arquivo_do_realm_mesmo_nao_sendo_utf8() {
    // O arquivo do pacote original tem comentários em chinês em GBK. Um `read_to_string`
    // falha nele com "stream did not contain valid UTF-8" — e foi exatamente isso que
    // aconteceu no primeiro teste em jogo (2026-09-07), deixando o realm sem vida máxima
    // de personagem. Os bytes altos ficam todos em comentário, que o leitor descarta.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/realm_155/config");
    if !dir.join("ptemplate.conf").exists() {
        eprintln!("pulado: {} não tem ptemplate.conf", dir.display());
        return;
    }
    let bytes = std::fs::read(dir.join("ptemplate.conf")).unwrap();
    assert!(
        String::from_utf8(bytes.clone()).is_err(),
        "o arquivo virou UTF-8 puro; este teste perdeu o sentido — confira se não foi          reconvertido por engano"
    );
    let t = ptemplate::ler_da_pasta(&dir).expect("devia ler apesar do GBK nos comentários");
    assert_eq!(t.len(), 12);
    assert_eq!(t.get(0).unwrap().vida, 60);
}

/// Todo personagem nasce com 5/5/5/5, e a vida e a mana de nível 1 são as dos moldes do
/// `clsconfig` original: `vit_hp × 5` e `eng_mp × 5`, sem o `hp`/`mp` do `.conf`.
///
/// Até 2026-09-16 os atributos saíam do `ptemplate.conf` e o Arqueiro nascia com 20 de
/// energia e 10 de agilidade (teste em jogo do Murillo). Os valores esperados abaixo são os
/// do `gamedbd/clsconfig` do `pwserver_155v156` (`GRoleStatus.hp/mp` dos roles 16..31).
#[test]
fn todo_personagem_nasce_cinco_cinco_cinco_cinco_com_a_vida_do_clsconfig() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join("data/realm_155/config");
    let Some(t) = ptemplate::ler_da_pasta(&dir) else {
        eprintln!("pulado: {} sem ptemplate.conf", dir.display());
        return;
    };
    let Ok(bytes) = std::fs::read(dir.join("elements.data")) else { return };
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&bytes).unwrap();
    let classes = pw_data_loader::classes::carregar(&e);

    // (classe, vida, mana) de nível 1 nos moldes do clsconfig.
    let moldes = [
        (0, 75, 45), (1, 50, 70), (2, 50, 70), (3, 60, 60), (4, 85, 35), (5, 65, 50),
        (6, 65, 55), (7, 50, 70), (8, 75, 45), (9, 50, 70), (10, 65, 55), (11, 50, 70),
    ];
    for (cls, vida, mana) in moldes {
        let a = t.get(cls).unwrap().ficha_inicial(classes.get(cls));
        assert_eq!((a.forca, a.agilidade, a.vitalidade, a.energia), (5, 5, 5, 5), "classe {cls}");
        assert_eq!((a.vida, a.mana), (vida, mana), "classe {cls}");
    }
}
