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
    let mapa = Mapa { terreno: &chao, movimento: &mov };
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
