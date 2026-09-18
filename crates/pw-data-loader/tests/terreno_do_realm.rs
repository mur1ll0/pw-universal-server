//! O mapa de alturas do realm, lido dos `.hmap` de verdade.
//!
//! Duas coisas em jogo dependiam disto e estavam erradas até 2026-09-11: o teleporte de GM
//! enterrava o jogador, e monstros terrestres flutuavam. As duas pelo mesmo motivo — o
//! servidor não sabia onde era o chão.
//!
//! O que este arquivo cobra do arquivo real, e que nenhum teste sintético cobriria: que os
//! blocos existam com o tamanho que a configuração promete, que as alturas caiam na faixa
//! declarada (`vHeightMin`..`vHeightMax`), e que a altura sob os spawns do `npcgen.data`
//! seja plausível — é ela que decide se um monstro fica no chão ou no ar.

use pw_data_loader::terreno::{self, Terreno};
use std::path::PathBuf;

fn realm() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/realm_155/config")
}

fn mundo_principal() -> Option<Terreno> {
    let dir = realm().join("world");
    if !dir.join("map").exists() {
        eprintln!("pulado: {} não existe", dir.join("map").display());
        return None;
    }
    Some(Terreno::ler(1, &dir))
}

/// Os 88 blocos do mundo principal existem e têm o tamanho que o `gs.conf` promete.
///
/// 1.052.676 bytes por arquivo = 513² floats. Se um dia o tamanho mudar, é aqui que se
/// descobre — e não com o personagem dentro do chão.
#[test]
fn os_oitenta_e_oito_blocos_do_mundo_estao_la() {
    let Some(t) = mundo_principal() else { return };
    assert!(t.tem_dados(), "nenhum bloco lido do mundo principal");

    let (c, _) = terreno::config_do_mapa(1).unwrap();
    let esperado = (c.vertices_por_bloco + 1) * (c.vertices_por_bloco + 1) * 4;
    let pasta = realm().join("world/map");
    for n in 1..=(c.blocos_colunas * c.blocos_linhas) {
        let p = pasta.join(format!("{n}.hmap"));
        let tam = std::fs::metadata(&p).map(|m| m.len() as usize);
        assert_eq!(
            tam.ok(),
            Some(esperado),
            "{} deveria ter {esperado} bytes",
            p.display()
        );
    }
}

/// As alturas caem dentro de `vHeightMin..vHeightMax`.
///
/// É a conferência de que a conversão `h * (max - min) + min` foi aplicada: os floats do
/// arquivo são 0..1, e esquecer a escala daria um mundo inteiro com 1 metro de relevo.
#[test]
fn as_alturas_ficam_na_faixa_declarada_e_variam() {
    let Some(t) = mundo_principal() else { return };
    let (c, _) = terreno::config_do_mapa(1).unwrap();

    // Uma varredura grosseira do mapa inteiro, de 128 em 128 metros.
    let mut vistas = Vec::new();
    let mut x = -4000.0f32;
    while x < 4000.0 {
        let mut z = -5500.0f32;
        while z < 5500.0 {
            if let Some(h) = t.altura_em(x, z) {
                assert!(
                    h >= c.altura_minima - 0.01 && h <= c.altura_maxima + 0.01,
                    "altura {h} em ({x}, {z}) fora de {}..{}",
                    c.altura_minima,
                    c.altura_maxima
                );
                vistas.push(h);
            }
            z += 128.0;
        }
        x += 128.0;
    }

    assert!(vistas.len() > 1_000, "só {} amostras dentro do mapa", vistas.len());
    let min = vistas.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = vistas.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max - min > 20.0,
        "o mapa inteiro entre {min} e {max} — relevo plano demais para ser verdade; \
         a escala de altura não deve ter sido aplicada"
    );
    eprintln!("mundo 1: altura de {min:.1} a {max:.1} em {} amostras", vistas.len());
}

/// Fora dos limites o terreno diz que não sabe, em vez de inventar.
///
/// Importa porque quem chama decide o que fazer com o `None`: o teleporte mantém a altura
/// do jogador, o spawn mantém o `y` do arquivo. Um zero disfarçado de resposta poria os
/// dois no fundo do mapa.
#[test]
fn fora_dos_limites_do_gs_conf_nao_ha_resposta() {
    let Some(t) = mundo_principal() else { return };
    assert_eq!(t.altura_em(-4097.0, 0.0), None, "x abaixo do limite oeste");
    assert_eq!(t.altura_em(5000.0, 0.0), None, "x além do limite leste");
    assert_eq!(t.altura_em(0.0, 5633.0), None, "z além do limite norte");
    assert!(t.altura_em(0.0, 0.0).is_some(), "o centro do mapa tem de ter chão");
}

/// A altura de cada spawn, pela regra do original, medida contra o mapa real.
///
/// O que os campos lidos do `npcgen.data` mostram, e que derrubou a dedução do item 42d:
///
/// - O `fOffsetTrn` é **zero** em quase todos os geradores. Não há deslocamento a
///   preservar por área.
/// - O que separa caverna e gerador aéreo do monstro comum é o **tipo da área** (`iType`):
///   área de chão nasce no terreno; área em caixa nasce onde a caixa manda, com o terreno
///   como piso (`box_gen_pos`, `npcgenerator.cpp:4340-4345`).
///
/// A dedução antiga (`centro.y − chão(centro)`) deixava no ar toda área de **chão** cujo
/// centro o editor tivesse posto um pouco acima do terreno — é o candidato natural ao NPC
/// "Guia" flutuando do teste de 2026-09-12.
#[test]
fn a_altura_segue_o_tipo_da_area() {
    use pw_data_loader::npcgen::{SpawnType, TipoDeArea};

    let Some(t) = mundo_principal() else { return };
    let caminho = realm().join("world/npcgen.data");
    let Ok(bytes) = std::fs::read(&caminho) else {
        eprintln!("pulado: {} não existe", caminho.display());
        return;
    };
    let ng = pw_data_loader::npcgen::NpcGenData::load_from_bytes(&bytes)
        .expect("npcgen.data do mundo deveria ser legível");

    const RENTE: f32 = 2.0;
    let mut de_chao = 0usize;
    let mut de_chao_fora = 0usize;
    let mut em_caixa = 0usize;
    let mut em_caixa_abaixo_do_chao = 0usize;
    let mut npc_de_chao_que_a_deducao_deixava_no_ar = 0usize;

    for inst in &ng.instances {
        if inst.spawn_type == SpawnType::DynamicObject {
            continue;
        }
        let Some(chao) = t.altura_em(inst.pos.x, inst.pos.z) else {
            continue;
        };
        let y = inst.altura_resolvida(Some(chao));

        match inst.tipo_de_area {
            TipoDeArea::NoChao => {
                de_chao += 1;
                if (y - chao - inst.acima_do_chao).abs() > 0.01 {
                    de_chao_fora += 1;
                }
                // A dedução antiga, para medir o que ela fazia com este mesmo spawn.
                if inst.spawn_type == SpawnType::Npc {
                    if let Some(chao_do_centro) =
                        t.altura_em(inst.centro_da_area.x, inst.centro_da_area.z)
                    {
                        let y_antigo = chao + (inst.centro_da_area.y - chao_do_centro);
                        if y_antigo - chao > RENTE {
                            npc_de_chao_que_a_deducao_deixava_no_ar += 1;
                        }
                    }
                }
            }
            TipoDeArea::NaCaixa => {
                em_caixa += 1;
                if y < chao - 0.01 {
                    em_caixa_abaixo_do_chao += 1;
                }
            }
        }
    }

    eprintln!(
        "spawns de chão: {de_chao}; em caixa: {em_caixa}; NPCs de área de chão que a          dedução antiga deixava a mais de {RENTE} m do chão:          {npc_de_chao_que_a_deducao_deixava_no_ar}"
    );

    assert!(de_chao > 10_000, "só {de_chao} spawns em área de chão");
    assert!(em_caixa > 0, "nenhuma área em caixa — o iType não deve estar sendo lido");
    assert_eq!(de_chao_fora, 0, "spawn de área de chão fora do terreno");
    assert_eq!(
        em_caixa_abaixo_do_chao, 0,
        "spawn de área em caixa abaixo do chão — o terreno tem de ser piso"
    );
}

/// O recurso do mapa se espalha pela área, como no original.
///
/// O leitor descartava o `fExtX`/`fExtZ` da área de recurso e punha toda instância na mesma
/// coordenada — o "Eufórbio todo junto" do teste de 2026-09-12. O original sorteia dentro
/// da caixa (`mine_spawner` com `SetRegion(0, vPos, {fExtX, 0, fExtZ})`,
/// `npcgenerator.cpp:3900`).
#[test]
fn o_recurso_se_espalha_pela_area() {
    use pw_data_loader::npcgen::SpawnType;
    use std::collections::HashMap;

    let caminho = realm().join("world/npcgen.data");
    let Ok(bytes) = std::fs::read(&caminho) else {
        eprintln!("pulado: {} não existe", caminho.display());
        return;
    };
    let ng = pw_data_loader::npcgen::NpcGenData::load_from_bytes(&bytes)
        .expect("npcgen.data do mundo deveria ser legível");

    // Agrupa por área (o centro identifica a área) e conta posições distintas.
    let mut por_area: HashMap<(i32, i32), (usize, Vec<(i32, i32)>, bool)> = HashMap::new();
    for inst in ng.instances.iter().filter(|i| i.spawn_type == SpawnType::ResourceMine) {
        let chave = (
            (inst.centro_da_area.x * 10.0) as i32,
            (inst.centro_da_area.z * 10.0) as i32,
        );
        let pontual = inst.extensao_da_area.x.abs() < 1e-3 && inst.extensao_da_area.z.abs() < 1e-3;
        let e = por_area.entry(chave).or_insert((0, Vec::new(), pontual));
        e.0 += 1;
        e.1.push(((inst.pos.x * 10.0) as i32, (inst.pos.z * 10.0) as i32));
    }

    let mut com_varios = 0usize;
    let mut empilhadas = 0usize;
    for (_, (n, pontos, pontual)) in &por_area {
        if *n < 2 || *pontual {
            continue;
        }
        com_varios += 1;
        let mut distintos = pontos.clone();
        distintos.sort();
        distintos.dedup();
        if distintos.len() == 1 {
            empilhadas += 1;
        }
    }

    assert!(com_varios > 100, "só {com_varios} áreas de recurso com mais de uma instância");
    assert_eq!(
        empilhadas, 0,
        "{empilhadas} áreas de recurso com todas as instâncias no mesmo ponto"
    );
}

/// O mapa 161 (`a61`), onde todo personagem novo do 155 nasce: os 12 blocos existem, e
/// cada ponto de nascimento do `clsconfig` original fica no chão — a 1 cm, que é o que o
/// molde gravou (`scripts/2026_09_12_nascimento_no_mapa_161_155.sql`).
///
/// Até 2026-09-12 o catálogo de terreno usava o `index` do `gs.conf` como id de mundo, e o
/// 161 não existia nele; o mesmo número era o `a31`.
#[test]
fn o_mapa_161_tem_chao_sob_os_nascimentos() {
    let dir = realm().join("a61");
    if !dir.join("map").exists() {
        eprintln!("pulado: {} não existe", dir.display());
        return;
    }
    let t = Terreno::ler(161, &dir);
    assert!(t.tem_dados());
    let (c, sub) = terreno::config_do_mapa(161).expect("161 no catálogo");
    assert_eq!((c.blocos_colunas, c.blocos_linhas, sub.as_str()), (4, 3, "map"));

    for (cls, x, y, z) in [
        (0, -848.3218f32, 40.5060f32, -181.9892f32),
        (2, -651.0889, 41.0100, -225.2057),
        (4, -712.8714, 35.0153, -364.3938),
        (6, -821.6534, 44.9115, -259.6685),
        (8, -800.5193, 44.9111, -314.2396),
        (10, -760.7382, 44.8697, -218.2820),
    ] {
        let chao = t.altura_em(x, z).unwrap_or_else(|| panic!("cls {cls}: fora do mapa"));
        assert!((y - chao).abs() < 0.05, "cls {cls}: molde em y={y}, chão em {chao}");
    }
}

/// Os ids do catálogo são os `tag` do `gs.conf`: a pasta `aNN` é o mundo `100 + NN`.
#[test]
fn o_catalogo_usa_o_tag_do_mundo() {
    assert_eq!(terreno::config_do_mapa(1).map(|(c, _)| c.blocos_colunas), Some(8));
    assert!(terreno::config_do_mapa(131).is_some(), "a31 é o mundo 131");
    assert!(terreno::config_do_mapa(31).is_none(), "31 era o index do is01, não um tag");
}
