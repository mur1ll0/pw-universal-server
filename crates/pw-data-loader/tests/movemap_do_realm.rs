//! O mapa de movimento (`movemap/`) do realm, lido dos arquivos de verdade.
use pw_data_loader::npcgen::{NpcGenData, SpawnType};
use pw_data_loader::{MapaDeMovimento, Terreno};
use std::path::PathBuf;

fn mundo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config/world")
}

/// Os 88 submapas (8×11) do mundo fecham no último byte.
#[test]
fn os_submapas_do_mundo_fecham_no_ultimo_byte() {
    if !mundo().join("movemap").exists() {
        eprintln!("pulado: sem movemap");
        return;
    }
    let m = MapaDeMovimento::ler(1, &mundo());
    // Só 55 dos 88 submapas têm arquivo; os outros valem "alcançável, altura zero", como no
    // original (`Init` antes da carga). Todo par que existe tem de fechar.
    let pares = std::fs::read_dir(mundo().join("movemap"))
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".rmap"))
        .count();
    assert_eq!(pares, 55);
    assert_eq!(m.submapas_lidos(), pares, "algum .rmap/.dhmap do mundo não fechou com o formato");
}

/// Relato de 2026-09-24: Gárgulas Ancestrais (44606) nascendo **dentro** da estrutura de pedra
/// nas Ilhas Ascendentes (mapa 161, tela 486/525 → mundo 860/−250). O piso ali fica até 2 m
/// acima do terreno, e o servidor as punha no terreno.
#[test]
fn as_gargulas_do_mapa_161_nascem_em_cima_da_pedra() {
    let a61 = mundo().parent().unwrap().join("a61");
    if !a61.join("movemap").exists() {
        return;
    }
    let m = MapaDeMovimento::ler(161, &a61);
    assert_eq!(m.submapas_lidos(), 2, "os dois submapas do a61");
    let t = Terreno::ler(161, &a61);
    let ng = NpcGenData::load_from_bytes(&std::fs::read(a61.join("npcgen.data")).unwrap()).unwrap();
    let gargulas: Vec<_> = ng.instances.iter().filter(|i| i.template_id == 44606).collect();
    assert_eq!(gargulas.len(), 52);
    let mut em_cima = 0;
    for g in &gargulas {
        let (p, no_piso) = g.posicao_no_mapa(&t, &m);
        em_cima += no_piso as usize;
        let chao = t.altura_em(p.x, p.z).unwrap();
        if let Some(acima) = m.acima_do_terreno(p.x, p.z) {
            assert!(p.y >= chao + acima - 0.001, "gárgula em ({:.1},{:.1}) {:.2} m dentro da pedra", p.x, p.z, chao + acima - p.y);
        }
    }

    assert_eq!(em_cima, 22, "as que o piso de pedra levanta (das 52)");
    // Uma medida fixa: o gerador em (848,7; −240,2) fica 2 m acima do terreno.
    let g = gargulas.iter().find(|g| (g.pos.x - 848.7).abs() < 0.1 && (g.pos.z + 240.2).abs() < 0.1).unwrap();
    let (p, _) = g.posicao_no_mapa(&t, &m);
    assert!((p.y - (t.altura_em(p.x, p.z).unwrap() + 2.0)).abs() < 0.001);
}

/// Nenhum nascimento de área no chão, de nenhum mapa do realm, fica dentro de estrutura.
#[test]
fn nenhum_nascimento_no_chao_fica_dentro_de_estrutura() {
    let raiz = mundo().parent().unwrap().to_path_buf();
    let catalogo = pw_data_loader::terreno::config_do_mapa;
    let (mut conferidos, mut levantados) = (0usize, 0usize);
    for e in std::fs::read_dir(&raiz).unwrap().flatten() {
        let dir = e.path();
        let nome = e.file_name().to_string_lossy().to_string();
        let id = if nome == "world" { 1 } else if let Some(n) = nome.strip_prefix('a').or(nome.strip_prefix('b')).and_then(|n| n.parse::<i32>().ok()) { if nome.starts_with('a') { 100 + n } else { 200 + n } } else { continue };
        if catalogo(id).is_none() || !dir.join("movemap").exists() {
            continue;
        }
        let Ok(b) = std::fs::read(dir.join("npcgen.data")) else { continue };
        let Ok(ng) = NpcGenData::load_from_bytes(&b) else { continue };
        let (t, m) = (Terreno::ler(id, &dir), MapaDeMovimento::ler(id, &dir));
        if !t.tem_dados() {
            continue;
        }
        for i in ng.instances.iter().filter(|i| i.tipo_de_area == pw_data_loader::npcgen::TipoDeArea::NoChao) {
            let (p, no_piso) = i.posicao_no_mapa(&t, &m);
            let (Some(chao), Some(acima)) = (t.altura_em(p.x, p.z), m.acima_do_terreno(p.x, p.z)) else { continue };
            conferidos += 1;
            levantados += no_piso as usize;
            assert!(p.y >= chao + acima - 0.001, "{nome}: tid {} em ({:.1},{:.1}) dentro da estrutura", i.template_id, p.x, p.z);
        }
    }
    eprintln!("{conferidos} nascimentos no chão conferidos, {levantados} em cima de estrutura");
    assert!(conferidos > 10_000 && levantados > 0);
}
