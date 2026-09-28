//! A perseguição do monstro de chão no mapa de movimento **real** do mapa 161 (Ilhas
//! Ascendentes): onde a reta até o alvo passa por pixel inalcançável (pedra, estrutura), o
//! agente do original contorna; a reta antiga atravessava.
use pw_data_loader::{MapaDeMovimento, Terreno};
use pw_gs::navegacao::{Mapa, SeguirAlvo, V3};
use rand::{Rng, SeedableRng};
use std::path::PathBuf;

fn a61() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config/a61")
}

#[test]
fn no_mapa_161_o_agente_contorna_o_que_a_reta_atravessa() {
    if !a61().join("movemap").exists() {
        eprintln!("pulado: sem a61/movemap");
        return;
    }
    let mov = MapaDeMovimento::ler(161, &a61());
    let ter = Terreno::ler(161, &a61());
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov, espaco: None, agua: None };
    let mut rng = rand::rngs::StdRng::seed_from_u64(161);

    let dentro = |p: V3| {
        let (u, v) = mov.pixel_de(p.x, p.z);
        !mov.alcancavel(u, v)
    };
    let (mut pares, mut chegou, mut passos_agente, mut dentro_agente, mut dentro_reta, mut passos_reta) = (0, 0, 0, 0, 0, 0);
    // A região das Gárgulas, em volta de (860, −250).
    while pares < 60 {
        let a = V3::new(rng.gen_range(780.0..940.0), 0.0, rng.gen_range(-340.0..-190.0));
        let ang: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let dist: f32 = rng.gen_range(8.0..30.0);
        let b = V3::new(a.x + dist * ang.cos(), 0.0, a.z + dist * ang.sin());
        let (pa, pb) = (mov.pixel_de(a.x, a.z), mov.pixel_de(b.x, b.z));
        if !mov.alcancavel(pa.0, pa.1) || !mov.alcancavel(pb.0, pb.1) || mov.reta_livre(pa, pb).0 {
            continue;
        }
        pares += 1;

        // A reta de antes: 2 m por passo, direto ao alvo.
        let mut p = a;
        while (p.x - b.x).hypot(p.z - b.z) > 2.0 {
            let d = (b.x - p.x).hypot(b.z - p.z);
            p = V3::new(p.x + (b.x - p.x) / d * 2.0, 0.0, p.z + (b.z - p.z) / d * 2.0);
            passos_reta += 1;
            dentro_reta += dentro(p) as i32;
        }

        // O agente do original, com o condutor de `session_npc_follow_target`.
        let mut s = SeguirAlvo::default();
        let d2 = (a.x - b.x).powi(2) + (a.z - b.z).powi(2);
        s.comecar(a, b, 2.0, 1.0, d2, None, &mapa);
        for _ in 0..120 {
            if s.chegou() {
                break;
            }
            if !s.andar(2.0, &mapa) {
                let q = s.posicao();
                let d2 = (q.x - b.x).powi(2) + (q.z - b.z).powi(2);
                s.comecar(q, b, 2.0, 1.0, d2, None, &mapa);
            }
            passos_agente += 1;
            dentro_agente += dentro(s.posicao()) as i32;
        }
        let q = s.posicao();
        chegou += ((q.x - b.x).hypot(q.z - b.z) <= 3.0) as i32;
    }
    let pct = |a: i32, b: i32| 100.0 * a as f32 / b.max(1) as f32;
    eprintln!(
        "{pares} pares com a reta bloqueada: reta {:.1}% dos passos dentro de obstáculo; agente {:.1}%, chegou em {chegou}",
        pct(dentro_reta, passos_reta),
        pct(dentro_agente, passos_agente)
    );
    assert!(pct(dentro_reta, passos_reta) > 5.0, "a amostra não tem obstáculo de verdade");
    assert!(pct(dentro_agente, passos_agente) < pct(dentro_reta, passos_reta) / 4.0, "o agente não desvia");
    assert!(chegou >= pares * 2 / 3, "o agente chegou em só {chegou} de {pares}");
}

/// B133 — o NPC de ar no espaço **real** do mapa 1 do 1.5.5 (`world/airmap/`): onde a reta entre
/// dois pontos livres atravessa folha bloqueada da octree, o `CNPCChaseOnAirPFAgent` busca o
/// caminho e chega sem entrar em folha bloqueada.
///
/// Alcance 0: a meta da busca é o próprio destino. Com alcance maior, o original recua a meta e,
/// se ela e os 11 sorteios na esfera em volta caem em espaço bloqueado, **desiste da busca e vai
/// em reta** (`NPCChaseSpatiallyPFAgent.cpp:103-117`) — a reta só respeita terreno e água, e
/// atravessa a octree. É o original; este teste mede o desvio, não esse caso.
#[test]
fn no_mapa_1_o_agente_de_ar_desvia_do_espaco_bloqueado() {
    use pw_data_loader::{MapaDeAgua, MapaDoEspaco};
    use pw_gs::navegacao::{Ambiente, SeguirNoEspaco};
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config/world");
    if !dir.join("airmap/spmap.conf").exists() {
        eprintln!("pulado: sem world/airmap");
        return;
    }
    let esp = MapaDoEspaco::ler(1, &dir);
    let ter = Terreno::ler(1, &dir);
    let agua = MapaDeAgua::ler(1, &dir);
    let mov = MapaDeMovimento::vazio();
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov, espaco: Some(&esp), agua: Some(&agua) };
    let bloqueado = |p: V3| esp.no(esp.centro_do_voxel([p.x, p.y, p.z])).is_some_and(|n| n.estado != 0);
    let no_ar = |p: V3| chao(p.x, p.z).is_some_and(|h| p.y > h + 2.0) && !bloqueado(p);

    let (mut casos, mut chegaram, mut atravessaram) = (0, 0, 0);
    'busca: for x in (-4000..4000).step_by(50) {
        for z in (-5500..5500).step_by(50) {
            let Some(h) = chao(x as f32, z as f32) else { continue };
            for dy in [3.0f32, 6.0, 10.0] {
                let b = V3::new(x as f32, h + dy, z as f32);
                if !bloqueado(b) {
                    continue;
                }
                // Origem e destino nos centros de voxel: um ponto na borda cai de um lado ou do
                // outro por arredondamento.
                let centro = |v: V3| {
                    let c = esp.centro_do_voxel([v.x, v.y, v.z]);
                    V3::new(c[0] as f32, c[1] as f32, c[2] as f32)
                };
                let (de, ate) = (centro(V3::new(b.x - 12.0, b.y, b.z)), centro(V3::new(b.x + 12.0, b.y, b.z)));
                if !no_ar(de) || !no_ar(ate) {
                    continue;
                }
                casos += 1;
                let mut s = SeguirNoEspaco::default();
                s.comecar(Ambiente::Ar, de, ate, 2.0, 0.0, 576.0, &mapa);
                let mut ok = true;
                for _ in 0..400 {
                    if s.chegou() {
                        break;
                    }
                    if !s.andar(2.0, &mapa) {
                        break;
                    }
                    if bloqueado(s.posicao()) {
                        let q = s.posicao();
                        // Distância até a borda do voxel (voxel de 2 m, bordas nos pares).
                        let borda = |v: f32| (v - (v / 2.0).round() * 2.0).abs();
                        eprintln!("  bloqueado em {q:?} (borda x {:.2} y {:.2} z {:.2}), de {de:?} a {ate:?}", borda(q.x), borda(q.y), borda(q.z));
                        ok = false;
                    }
                }
                if !ok {
                    atravessaram += 1;
                }
                if s.chegou() {
                    chegaram += 1;
                }
                if casos >= 20 {
                    break 'busca;
                }
            }
        }
    }
    eprintln!("ar: {casos} casos, {chegaram} chegaram, {atravessaram} atravessaram bloqueio");
    assert!(casos > 0, "nenhum trecho bloqueado entre dois pontos livres");
    assert_eq!(atravessaram, 0, "o agente entrou em folha bloqueada");
    assert!(chegaram * 2 >= casos, "chegaram só {chegaram} de {casos}");
}
