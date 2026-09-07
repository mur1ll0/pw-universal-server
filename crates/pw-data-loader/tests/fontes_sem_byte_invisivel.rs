//! Nenhum fonte do workspace pode ter byte NUL cru.
//!
//! # Por que este teste existe
//!
//! Um `'\0'` escrito por script de edição pode virar o **byte** 0x00 no arquivo em vez da
//! sequência de escape de dois caracteres. O Rust aceita: `contains('\u{0}')` compila e
//! roda igual. O problema é o resto — `grep` passa a tratar o arquivo como binário e para
//! de mostrar as linhas, `git diff` também, e a revisão fica cega justamente no trecho
//! alterado.
//!
//! Aconteceu duas vezes na mesma sessão (2026-09-07), as duas em
//! `aipolicy_tests.rs`. É invisível em revisão e custa nada de checar.

use std::path::{Path, PathBuf};

fn raiz_do_workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn varrer(dir: &Path, achados: &mut Vec<(PathBuf, usize)>) {
    let Ok(entradas) = std::fs::read_dir(dir) else { return };
    for e in entradas.flatten() {
        let caminho = e.path();
        if caminho.is_dir() {
            // `target/` tem binários de verdade; não é fonte.
            if caminho.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            varrer(&caminho, achados);
        } else if caminho.extension().is_some_and(|x| x == "rs") {
            if let Ok(bytes) = std::fs::read(&caminho) {
                let n = bytes.iter().filter(|b| **b == 0).count();
                if n > 0 {
                    achados.push((caminho, n));
                }
            }
        }
    }
}

#[test]
fn nenhum_fonte_rust_tem_byte_nul() {
    let mut achados = Vec::new();
    varrer(&raiz_do_workspace().join("crates"), &mut achados);
    assert!(
        achados.is_empty(),
        "byte NUL cru em fonte Rust (troque por '\\0'): {achados:?}"
    );
}
