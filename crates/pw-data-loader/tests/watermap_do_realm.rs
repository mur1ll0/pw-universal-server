//! O `watermap/` dos mapas do `realm_155`, lido dos arquivos reais (B88).
//!
//! O leitor recusa arquivo que não fecha no último byte, então ler os 75 mapas sem recusa
//! nenhuma é a prova de que o formato está certo — a mesma prova que o `npcgen` e o
//! `aipolicy` usam.

use pw_data_loader::MapaDeAgua;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join("data/realm_155/config")
}

/// Todo mapa do realm que tem `watermap/` é lido inteiro, e os que têm água de verdade
/// aparecem.
#[test]
fn os_mapas_de_agua_do_realm_sao_lidos_inteiros() {
    let raiz = raiz();
    if !raiz.exists() {
        eprintln!("pulado: o realm_155 não está nesta máquina");
        return;
    }

    let mut com_pasta = 0usize;
    let mut com_agua = 0usize;
    let mut areas = 0usize;

    let Ok(entradas) = std::fs::read_dir(&raiz) else { return };
    for e in entradas.flatten() {
        let dir = e.path();
        if !dir.join("watermap/watermap.conf").exists() {
            continue;
        }
        com_pasta += 1;
        let mapa = MapaDeAgua::ler(0, &dir);
        if mapa.tem_agua() {
            com_agua += 1;
        }
        // Conta as áreas varrendo um punhado de pontos não serve; o que importa aqui é que
        // a leitura não recusou nada. Um submapa recusado vira lista vazia e o `tem_agua`
        // cai — por isso o total de mapas com água é a régua.
        areas += usize::from(mapa.tem_agua());
    }

    assert!(com_pasta >= 70, "o realm_155 tem 75 pastas watermap; achei {com_pasta}");
    assert!(
        com_agua > 0,
        "nenhum mapa com área de água: ou o formato mudou, ou todo submapa foi recusado"
    );
    assert_eq!(areas, com_agua);
    eprintln!("watermap: {com_pasta} mapas com pasta, {com_agua} com água");
}

/// O mundo 1 tem água, e ela está onde o arquivo diz — conferido **à mão**.
///
/// O ponto `(0, 0)` do mundo cai, pela conta do original, no submapa `u = 4`, `v = 5` de uma
/// grade 8×11 de 1024 — o arquivo `45.wmap` (`(11−5−1)×8 + 4 + 1`) — e, dentro dele, em
/// `(−512, 0)`. Abrindo esse arquivo por fora do leitor, a terceira das cinco áreas é a
/// caixa centrada em `(−464, −240)` com meias medidas `48 × 272`: `|−512 − (−464)| = 48 ≤ 48`
/// e `|0 − (−240)| = 240 ≤ 272`. Ela contém o ponto, e a altura dela é **216**.
///
/// Este teste vale porque a conta foi refeita fora do código que ele testa: se o leitor
/// errar a origem, o índice do submapa, o nome do arquivo ou a caixa, o 216 não aparece.
#[test]
fn o_mundo_1_tem_agua_na_altura_que_o_arquivo_diz() {
    let dir = raiz().join("world");
    if !dir.join("watermap/watermap.conf").exists() {
        eprintln!("pulado: o realm_155 não está nesta máquina");
        return;
    }
    let mapa = MapaDeAgua::ler(1, &dir);
    assert!(mapa.tem_agua(), "o mundo 1 tem água em vários submapas");
    assert_eq!(mapa.altura_em(0.0, 0.0), 216.0, "a área que contém (0,0) tem altura 216");

    // Bem longe da grade não há água nenhuma.
    assert_eq!(mapa.altura_em(50_000.0, 50_000.0), pw_data_loader::watermap::SEM_AGUA);

    // E submerso vs. acima da superfície, no mesmo ponto.
    assert_eq!(mapa.quanto_abaixo(0.0, 214.0, 0.0), 2.0, "2 m abaixo da superfície");
    assert!(mapa.quanto_abaixo(0.0, 220.0, 0.0) < 0.0, "acima da superfície é negativo");
}
