//! As tabelas de progressão e os distritos, lidos dos arquivos do realm 155.
//!
//! Os números foram conferidos no `elements.data` pelo leitor Python
//! (`specs/elements_layouts/pw_elements_reader.py`) e no `a61/precinct.sev` por leitura
//! direta, antes de o leitor Rust existir.

use pw_data_loader::{Distritos, GameDataManager, TabelaDeProgressao};
use std::path::PathBuf;

fn pasta() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if p.join("elements.data").exists() {
        Some(p)
    } else {
        eprintln!("AVISO: {} sem elements.data — este teste NÃO verificou nada.", p.display());
        None
    }
}

fn dados() -> Option<GameDataManager> {
    let p = pasta()?;
    let mut dm = GameDataManager::new();
    let _ = dm.load_from_directory(&p);
    Some(dm)
}

#[test]
fn a_curva_de_experiencia_e_a_do_id_202_e_nao_a_do_mascote() {
    let Some(dm) = dados() else { return };
    let t: &TabelaDeProgressao = &dm.progressao;
    assert!(t.do_arquivo);
    // `PLAYER_LEVELEXP_CONFIG` id 202: 55, 220, 495…; o 592 (mascote) começa em 50.
    assert_eq!(t.exp_para_subir(1), 55);
    assert_eq!(t.exp_para_subir(2), 220);
    assert_eq!(t.exp_para_subir(3), 495);
}

#[test]
fn o_ajuste_por_diferenca_de_nivel_segue_os_degraus_do_param_adjust() {
    let Some(dm) = dados() else { return };
    let t = &dm.progressao;
    // Primeiro degrau do arquivo: diferença >= 40 → exp 0,05, dinheiro 1, item 0,2.
    let a = t.ajuste(40);
    assert!((a.exp - 0.05).abs() < 1e-6, "{a:?}");
    assert!((a.item - 0.2).abs() < 1e-6, "{a:?}");
    // Segundo degrau: diferença de 30 a 39 → exp 0,1.
    assert!((t.ajuste(35).exp - 0.1).abs() < 1e-6);
    // Mesmo nível tem de pagar a experiência inteira.
    assert!((t.ajuste(0).exp - 1.0).abs() < 1e-6, "{:?}", t.ajuste(0));
}

#[test]
fn a_perda_na_morte_vem_do_cultivo() {
    let Some(dm) = dados() else { return };
    // `PLAYER_SECONDLEVEL_CONFIG`: 0,05, 0,05, 0,045, 0,04…
    assert!((dm.progressao.perda_na_morte(0) - 0.05).abs() < 1e-6);
    assert!((dm.progressao.perda_na_morte(2) - 0.045).abs() < 1e-6);
}

#[test]
fn o_distrito_do_nascimento_no_161_tem_ponto_de_cidade_no_proprio_161() {
    let Some(p) = pasta() else { return };
    let d = Distritos::ler(&std::fs::read(p.join("a61/precinct.sev")).unwrap()).unwrap();
    assert_eq!(d.lista.len(), 25);
    // Onde o Bárbaro nasce (molde do clsconfig, B47).
    let e = d.distrito_em(-712.9, -364.4, 161).expect("o nascimento está num distrito");
    assert_eq!(e.mapa_do_ponto, 161);
}
