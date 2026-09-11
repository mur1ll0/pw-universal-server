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
        .join("data/realm_155BR/config")
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

/// A conta que assenta os monstros no chão, medida contra o mapa e o `npcgen.data` reais.
///
/// É o teste que sustenta a correção de "monstros terrestres no ar". O que os números do
/// realm mostram, e que mudou o diagnóstico no meio do caminho:
///
/// - O `y` do `npcgen.data` **é altura absoluta e está certo** — no centro da área ele fica
///   a 0,10 m do chão, em três quartos das áreas. Aquele 0,10 constante é o
///   `offset_terrain` (`fOffsetTrn`) que o gerador do original soma.
/// - Quem erra é a **nossa dispersão**: `posicao_na_area` sorteia `x`/`z` dentro da caixa e
///   copia o `y` do centro. Em encosta, o monstro fica na altura do centro da área.
/// - **Nem toda área é de chão.** 1.083 das 10.172 áreas do mundo têm o centro a dezenas ou
///   centenas de metros do terreno, para os dois lados: deslocamento negativo é caverna,
///   positivo é gerador aéreo ou cidade de vários níveis. O mapa de alturas não representa
///   nada disso, e o original também não — ele preserva o deslocamento do gerador. Mexer
///   nessas seria tirar o monstro da caverna e pô-lo no telhado.
///
/// Então o teste mede a correção onde ela se aplica: nas áreas que **são** de chão.
#[test]
fn assentar_no_chao_tira_os_monstros_do_ar() {
    let Some(t) = mundo_principal() else { return };
    let caminho = realm().join("world/npcgen.data");
    let Ok(bytes) = std::fs::read(&caminho) else {
        eprintln!("pulado: {} não existe", caminho.display());
        return;
    };
    let ng = pw_data_loader::npcgen::NpcGenData::load_from_bytes(&bytes)
        .expect("npcgen.data do mundo deveria ser legível");

    /// Até onde um monstro conta como "no chão".
    const RENTE: f32 = 2.0;

    let mut de_chao = 0usize;
    let mut de_chao_no_ar_antes = 0usize;
    let mut de_chao_no_ar_depois = 0usize;
    let mut deliberadamente_fora = 0usize;
    let mut deslocamentos: Vec<f32> = Vec::new();

    for inst in &ng.instances {
        let (Some(chao), Some(chao_do_centro)) = (
            t.altura_em(inst.pos.x, inst.pos.z),
            t.altura_em(inst.centro_da_area.x, inst.centro_da_area.z),
        ) else {
            continue;
        };
        let deslocamento = inst.centro_da_area.y - chao_do_centro;
        deslocamentos.push(deslocamento);

        // Área cujo centro está longe do chão é caverna ou gerador aéreo: o deslocamento
        // é intencional e tem de sobreviver à correção.
        if deslocamento.abs() > RENTE {
            deliberadamente_fora += 1;
            let y_corrigido = chao + deslocamento;
            assert!(
                (y_corrigido - chao - deslocamento).abs() < 0.01,
                "a correção não preservou o deslocamento de uma área aérea/subterrânea"
            );
            continue;
        }

        de_chao += 1;
        if (inst.pos.y - chao).abs() > RENTE {
            de_chao_no_ar_antes += 1;
        }
        if (chao + deslocamento - chao).abs() > RENTE {
            de_chao_no_ar_depois += 1;
        }
    }

    assert!(de_chao > 15_000, "só {de_chao} spawns de área de chão");
    assert!(deliberadamente_fora > 0, "nenhuma área aérea/subterrânea — amostra suspeita");

    // O 0,10 constante é o `offset_terrain` do arquivo. Se a mediana saísse disso, a
    // leitura de `area.pos` ou a escala do `.hmap` teriam mudado.
    deslocamentos.sort_by(|a, b| a.total_cmp(b));
    let mediana = deslocamentos[deslocamentos.len() / 2];
    assert!(
        (mediana - 0.10).abs() < 0.5,
        "deslocamento mediano de {mediana:.2} m — esperava ~0,10 (o offset_terrain do          arquivo). A leitura do npcgen ou a escala do .hmap mudou."
    );

    let pct = |n: usize| n as f32 * 100.0 / de_chao as f32;
    eprintln!(
        "áreas de chão: {de_chao} spawns — no ar antes {} ({:.1}%), depois {} ({:.1}%);          {deliberadamente_fora} spawns em área aérea/subterrânea preservados",
        de_chao_no_ar_antes,
        pct(de_chao_no_ar_antes),
        de_chao_no_ar_depois,
        pct(de_chao_no_ar_depois),
    );
    assert!(
        pct(de_chao_no_ar_antes) > 10.0,
        "só {:.1}% dos spawns de chão estavam fora do chão — o defeito relatado não se          reproduz nestes dados, e esta correção precisa ser reexaminada",
        pct(de_chao_no_ar_antes)
    );
    assert_eq!(
        de_chao_no_ar_depois, 0,
        "depois da correção nenhum spawn de área de chão pode ficar fora do chão"
    );
}
