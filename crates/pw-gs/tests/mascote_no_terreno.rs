//! O Vespão Pequeno (10521, de ar) do `realm_126` perseguindo um monstro que foge pelo terreno
//! **real** do mapa 1: não pode travar nem passar por dentro do chão.
//!
//! O cliente desenha o NPC de ar em linha reta entre dois `OBJECT_MOVE` (sem acompanhar o chão,
//! `CECNPC::MoveTo` com `bTraceGround` falso no ar, `EC_NPC.cpp:1000-1020`): por isso o teste
//! mede também o **meio** de cada passo contra o terreno.
use pw_core::Vector3;
use pw_data_loader::{MapaDeAgua, MapaDeMovimento, MapaDoEspaco, Terreno};
use pw_gs::entity::MonsterEntity;
use pw_gs::mascote::{id_do_mascote, AcaoDoMascote, Mascote};
use pw_gs::navegacao::Mapa;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn o_vespao_persegue_quem_foge_no_terreno_real_sem_travar_nem_afundar() {
    let config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    let dir = config.join("world");
    let Ok(b) = std::fs::read(config.join("elements.data")) else {
        eprintln!("pulado: sem realm_126");
        return;
    };
    let el = pw_data_loader::generic_elements::load_elements_data_auto(&b).unwrap();
    let modelos = pw_data_loader::pet::carregar_modelos(&el);
    let modelo = modelos.get(&10521).expect("Vespão Pequeno");
    let ter = Terreno::ler(1, &dir);
    let mov = MapaDeMovimento::ler(1, &dir);
    let esp = MapaDoEspaco::ler(1, &dir);
    let agua = MapaDeAgua::ler(1, &dir);
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov, espaco: Some(&esp), agua: Some(&agua) };

    let mut rng = rand::rngs::StdRng::seed_from_u64(7);
    let (mut casos, mut travados, mut afundou, mut meio_dentro, mut golpes) = (0, 0, 0, 0, 0);
    let mut pior_meio = 0.0f32;
    while casos < 40 {
        let (x, z) = (rng.gen_range(-3500.0..3500.0f32), rng.gen_range(-5000.0..5000.0f32));
        let Some(h) = chao(x, z) else { continue };
        // Só onde há desnível (a queixa): 3 m ou mais em 20 m à volta.
        let h2 = chao(x + 20.0, z).unwrap_or(h);
        let h3 = chao(x, z + 20.0).unwrap_or(h);
        if (h2 - h).abs().max((h3 - h).abs()) < 3.0 || agua.altura_em(x, z) > h - 1.0 && agua.altura_em(x, z) != pw_data_loader::watermap::SEM_AGUA {
            continue;
        }
        casos += 1;
        let info = pw_core::pet::InfoPet { pet_tid: 10521, level: 30, hp_factor: 1.0, ..Default::default() };
        let mut m = Mascote::novo(id_do_mascote(1), 7, 0, info, modelo, Vector3::new(x, h + 1.5, z), 1, 1);
        m.corpo.habitat = pw_gs::ai::Habitat::Ar;
        let mut dono = Vector3::new(x - 3.0, chao(x - 3.0, z).unwrap_or(h), z);
        // O monstro foge em linha reta, a 4 m/s, sempre no chão.
        let ang = rng.gen_range(0.0..std::f32::consts::TAU);
        let (dx, dz) = (ang.cos(), ang.sin());
        let mut alvo = MonsterEntity::placeholder(900_001, 1001, Vector3::new(x + 4.0 * dx, 0.0, z + 4.0 * dz), 0);
        alvo.position.y = chao(alvo.position.x, alvo.position.z).unwrap_or(h);
        m.ai.atacar_por_ordem(900_001, m.corpo.max_hp);
        let mut parado_ms = 0u32;
        let mut ultima = m.corpo.position;
        let mut travou = false;
        for t in 0..(30_000 / 50) {
            if t < 400 {
                let nx = alvo.position.x + dx * 4.0 * 0.05;
                let nz = alvo.position.z + dz * 4.0 * 0.05;
                if let Some(nh) = chao(nx, nz) {
                    alvo.position = Vector3::new(nx, nh, nz);
                }
            }
            // O dono vai atrás, a 8 m do monstro (a Tsuko acompanha a briga).
            let (ddx, ddz) = (alvo.position.x - dono.x, alvo.position.z - dono.z);
            let dd = (ddx * ddx + ddz * ddz).sqrt();
            if dd > 8.0 {
                let (nx, nz) = (dono.x + ddx / dd * 0.25, dono.z + ddz / dd * 0.25);
                dono = Vector3::new(nx, chao(nx, nz).unwrap_or(dono.y), nz);
            }
            let alvos = HashMap::from([(900_001i64, (alvo.position, true, alvo.clone()))]);
            let antes = m.corpo.position;
            let acao = m.ai.tick(&mut m.corpo, Some(dono), &alvos, 50, &mapa);
            if matches!(acao, Some(AcaoDoMascote::Atacou { .. })) {
                golpes += 1;
            }
            let p = m.corpo.position;
            if let Some(hp) = chao(p.x, p.z) {
                if p.y < hp - 0.01 {
                    afundou += 1;
                }
            }
            if p != antes {
                // O meio do trecho que o cliente desenha.
                let meio = Vector3::new((p.x + antes.x) / 2.0, (p.y + antes.y) / 2.0, (p.z + antes.z) / 2.0);
                if let Some(hm) = chao(meio.x, meio.z) {
                    if meio.y < hm - 0.3 {
                        meio_dentro += 1;
                        pior_meio = pior_meio.max(hm - meio.y);
                    }
                }
            }
            // Travado: longe do alvo e sem sair do lugar por 5 s.
            let longe = p.distance(&alvo.position) > m.corpo.attack_range + 1.0;
            if longe && p.distance(&ultima) < 0.05 {
                parado_ms += 50;
                if parado_ms >= 5_000 {
                    travou = true;
                }
            } else {
                parado_ms = 0;
                ultima = p;
            }
        }
        if travou {
            travados += 1;
            eprintln!("  travou em ({x:.0}, {z:.0}), alvo em {:?}, mascote em {:?}", alvo.position, m.corpo.position);
        }
    }
    eprintln!("vespão: {casos} perseguições em encosta, {travados} travadas, {afundou} tiques abaixo do chão, {meio_dentro} trechos pelo chão (pior {pior_meio:.2} m), {golpes} golpes");
    assert_eq!(afundou, 0, "o mascote ficou abaixo do chão");
    assert_eq!(travados, 0, "o mascote travou");
    assert_eq!(meio_dentro, 0, "o trecho desenhado passa por dentro do chão");
}
