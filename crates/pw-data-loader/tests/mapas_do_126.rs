//! Os arquivos de mapa do `realm_126` fechando no último byte (B143): cada leitor recusa sobra
//! (`world_targets`, `precinct`, `path.sev`, `npcgen`, bloco de terreno) ou conta os submapas
//! que fecharam (`movemap`, `watermap`). O espaço aéreo (`airmap`) está em `espaco_do_realm`.

use pw_data_loader::{Distritos, MapaDeAgua, MapaDeMovimento, NpcGenData, Terreno};
use std::path::{Path, PathBuf};

fn raiz() -> Option<PathBuf> {
    let r = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if r.exists() {
        Some(r)
    } else {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", r.display());
        None
    }
}

/// Todos os arquivos com este nome, na raiz e nas pastas de mapa.
fn todos(raiz: &Path, nome: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if raiz.join(nome).exists() {
        v.push(raiz.join(nome));
    }
    for e in std::fs::read_dir(raiz).unwrap().flatten() {
        let p = e.path().join(nome);
        if p.exists() {
            v.push(p);
        }
    }
    v
}

#[test]
fn os_arquivos_de_mapa_do_126_fecham_no_ultimo_byte() {
    let Some(r) = raiz() else { return };

    let pontos = pw_data_loader::world_targets::PontosDoMundo::load_from_bytes(
        &std::fs::read(r.join("world_targets.sev")).unwrap(),
    )
    .expect("world_targets.sev");
    assert!(!pontos.is_empty());

    let mut falhas = Vec::new();
    let precintos = todos(&r, "precinct.sev");
    for p in &precintos {
        if let Err(e) = Distritos::ler(&std::fs::read(p).unwrap()) {
            falhas.push(format!("{}: {e}", p.display()));
        }
    }
    let rotas = todos(&r, "path.sev");
    for p in &rotas {
        if let Err(e) = pw_data_loader::rotas::Rotas::ler(&std::fs::read(p).unwrap()) {
            falhas.push(format!("{}: {e}", p.display()));
        }
    }
    let geradores = todos(&r, "npcgen.data");
    for p in &geradores {
        if let Err(e) = NpcGenData::load_from_bytes(&std::fs::read(p).unwrap()) {
            falhas.push(format!("{}: {e}", p.display()));
        }
    }
    assert!(falhas.is_empty(), "arquivos que não fecham: {falhas:#?}");
    eprintln!(
        "126: {} pontos, {} precinct.sev, {} path.sev, {} npcgen.data",
        pontos.len(),
        precintos.len(),
        rotas.len(),
        geradores.len()
    );
    assert_eq!((precintos.len(), rotas.len(), geradores.len()), (43, 41, 41));
}

#[test]
fn terreno_movimento_e_agua_do_126() {
    let Some(r) = raiz() else { return };
    let mundo = r.join("world");
    assert!(Terreno::ler(1, &mundo).tem_dados(), "o .hmap do mundo 1 não abriu");
    let rmaps = std::fs::read_dir(mundo.join("movemap"))
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".rmap"))
        .count();
    let m = MapaDeMovimento::ler(1, &mundo);
    assert_eq!(m.submapas_lidos(), rmaps, "algum .rmap/.dhmap do mundo 1 não fechou");
    assert_eq!(rmaps, 46);

    let mut pastas = 0;
    let mut com_agua = 0;
    for e in std::fs::read_dir(&r).unwrap().flatten() {
        let dir = e.path();
        if !dir.join("watermap/watermap.conf").exists() {
            continue;
        }
        pastas += 1;
        com_agua += usize::from(MapaDeAgua::ler(0, &dir).tem_agua());
    }
    eprintln!("126: {pastas} watermap, {com_agua} com água");
    assert_eq!(pastas, 40);
    assert!(com_agua > 0);
}
