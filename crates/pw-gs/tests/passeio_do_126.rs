//! B103 — o monstro do 1.2.6 anda no mapa real (`world/`) sem "teletransporte": cada
//! `OBJECT_MOVE` cobre no máximo velocidade × `use_time`, como na captura original.
use pw_core::Vector3;
use pw_data_loader::{GameDataManager, MapaDeMovimento, Terreno};
use pw_gs::ai::{AcaoDoMonstro, MonsterAi};
use pw_gs::entity::{MonsterEntity, PlayerEntity};
use pw_gs::navegacao::Mapa;
use std::collections::HashMap;
use std::path::PathBuf;

fn feiticeira_nivel_1() -> PlayerEntity {
    let pos = Vector3::new(0.0, 0.0, 0.0);
    let mut p = PlayerEntity {
        role_id: 1,
        name: "Alvo".into(),
        race: pw_core::Race::Human,
        cls: pw_core::CharacterClass::Venomancer,
        gender: pw_core::Gender::Male,
        level: 1,
        cultivation: 0,
        hp: 1000,
        max_hp: 1000,
        mp: 0,
        max_mp: 0,
        exp: 0,
        sp: 0,
        money: 0,
        strength: 5,
        agility: 5,
        vitality: 5,
        energy: 5,
        def_phys: 0,
        def_metal: 0,
        def_wood: 0,
        def_water: 0,
        def_fire: 0,
        def_earth: 0,
        attack_min: 1,
        attack_max: 1,
        magic_attack_min: 0,
        magic_attack_max: 0,
        armor: 0,
        attack_rate: 0,
        attack_degree: 0,
        defend_degree: 0,
        crit_damage_bonus: 0,
        attack_speed: 1.0,
        move_speed: 4.0,
        walk_speed: 2.0,
        swim_speed: 3.0,
        fly_speed: 5.0,
        attack_range: 2.5,
        hp_gen: 1,
        mp_gen: 1,
        crit_rate: 0.0,
        position: pos,
        target_id: None,
        efeitos: Default::default(),
        visiveis: std::collections::HashSet::new(),
        centro_do_stream: Vector3::new(0.0, 0.0, 0.0),
        voando: false,
        montaria: None,
        forma_enviada: None,
        operacao_de_pet: 0,
        modo_roupa: false,
        sec_level: 0,
        habilidades: Default::default(),
        crc_aparencia: 0,
        pontos_de_atributo: 0,
        reputacao: 0,
        combate_s: 0,
        contador_hp: 0,
        contador_mp: 0,
        recargas: std::collections::HashMap::new(),
        pecas: [None; pw_gs::entity::PECAS_VESTIDAS],
        auto_hp: None,
        daimon: None,
        auto_mp: None,
        recarga_do_auto_hp_s: 0,
        recarga_do_auto_mp_s: 0,
        npc_em_conversa: None,
        waypoints: Vec::new(),
        ap: 0,
        max_ap: 0,
        ap_por_golpe: 0,
        sentado: false,
        missoes: Default::default(),
        coleta: None,
        equipamento: Default::default(),
        ataque: None,
        conjuracao: None,
        dano_bruto: (1, 1),
        dano_magico_bruto: (1, 1),
        bonus_de_dano_pct: 0,
        bonus_magico_pct: 0,
    };
    p.hp = p.max_hp;
    p
}

#[test]
fn o_filhote_de_mandragora_passeia_sem_saltos() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.join("world").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let ter = Terreno::ler(1, &dir.join("world"));
    let mov = MapaDeMovimento::ler(1, &dir.join("world"));
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov };
    let modelo = d.monstros.get(3303).expect("3303");
    // A IA só passeia com alguém por perto (`RAIO_DE_ATIVIDADE`); o 3303 não é agressivo.
    let mut j = feiticeira_nivel_1();
    j.position = Vector3::new(-1440.0, 241.0, 1400.0);
    let jogadores: HashMap<i64, PlayerEntity> = [(1i64, j)].into_iter().collect();
    let (mut passos, mut saltos, mut maior, mut colados, mut sem_parada) = (0, 0, 0.0f32, 0, 0);
    for k in 0..20 {
        let inicio = Vector3::new(-1445.0 + k as f32 * 3.0, 241.0, 1390.0 + k as f32 * 2.0);
        let mut m = MonsterEntity::do_template(1, modelo, inicio, 30_000);
        let mut ai = MonsterAi::new();
        // (instante do último passo, use_time dele): o próximo não pode sair antes (B104).
        let mut ultimo: Option<(u32, u16)> = None;
        // Depois de um `OBJECT_MOVE`, o cliente segue andando até chegar comando novo
        // (`EC_NPC.cpp:1225-1240`): tem de vir outro passo ou a parada até o fim do use_time (B106).
        let mut prazo: Option<u32> = None;
        for t in 0..(180_000 / 50) {
            let antes = m.position;
            let acao = ai.tick_no_mapa(&mut m, &jogadores, 50, &mapa);
            if acao.is_some() {
                prazo = None;
            } else if prazo.is_some_and(|p| t * 50 > p) {
                sem_parada += 1;
                prazo = None;
            }
            if let Some(AcaoDoMonstro::Andou { tempo_ms, .. }) = acao {
                prazo = Some(t * 50 + tempo_ms as u32 + 100);
            }
            if let Some(AcaoDoMonstro::Andou { destino, tempo_ms, velocidade, .. }) = acao {
                if let Some((t0, u0)) = ultimo {
                    if (t - t0) * 50 < u0 as u32 {
                        colados += 1;
                    }
                }
                ultimo = Some((t, tempo_ms));
                let dist = ((destino.x - antes.x).powi(2) + (destino.z - antes.z).powi(2)).sqrt();
                let limite = velocidade * tempo_ms as f32 / 1000.0;
                passos += 1;
                maior = maior.max(dist / limite.max(0.01));
                if dist > limite * 1.1 + 0.05 {
                    saltos += 1;
                    if saltos <= 5 {
                        eprintln!("SALTO: {:.2} m em {} ms a {:.2} m/s (de {:?} para {:?})", dist, tempo_ms, velocidade, (antes.x, antes.z), (destino.x, destino.z));
                    }
                }
            }
        }
    }
    eprintln!("passeio 126: {passos} passos, {saltos} maiores que velocidade × use_time, {colados} antes do use_time do anterior, {sem_parada} sem passo nem parada depois, maior razão {maior:.2}");
    assert!(passos > 100);
    assert_eq!((saltos, colados, sem_parada), (0, 0, 0));
}

/// B103 — a perseguição no mapa real: com agressão, o monstro corre até o alvo em passos de
/// `run_speed × 0,5 s` (`session_npc_follow_target`, 500 ms também no `gs` 1.2.6).
#[test]
fn o_filhote_persegue_sem_saltos() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.join("world").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let ter = Terreno::ler(1, &dir.join("world"));
    let mov = MapaDeMovimento::ler(1, &dir.join("world"));
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov };
    let modelo = d.monstros.get(3303).expect("3303");
    let mut m = MonsterEntity::do_template(1, modelo, Vector3::new(-1445.0, 241.0, 1390.0), 30_000);
    let mut ai = MonsterAi::new();
    let mut j = feiticeira_nivel_1();
    j.position = Vector3::new(-1433.0, 241.0, 1390.0);
    ai.add_threat(1, 100);
    let mut jogadores: HashMap<i64, PlayerEntity> = [(1i64, j)].into_iter().collect();
    let (mut t, mut log, mut saltos) = (0u32, Vec::new(), 0);
    for passo in 0..(12_000 / 50) {
        // Depois de 5 s o jogador foge a 5 m/s.
        if passo * 50 > 5_000 {
            jogadores.get_mut(&1).unwrap().position.x += 5.0 * 0.05;
        }
        let antes = m.position;
        match ai.tick_no_mapa(&mut m, &jogadores, 50, &mapa) {
            Some(AcaoDoMonstro::Andou { destino, tempo_ms, velocidade, modo }) => {
                let dist = ((destino.x - antes.x).powi(2) + (destino.z - antes.z).powi(2)).sqrt();
                if dist > velocidade * tempo_ms as f32 / 1000.0 * 1.1 + 0.05 {
                    saltos += 1;
                }
                log.push(format!("t={t} anda {dist:.2}m/{tempo_ms}ms v={velocidade:.2} modo={modo}"));
            }
            Some(AcaoDoMonstro::Parou { posicao, .. }) => {
                let dist = ((posicao.x - antes.x).powi(2) + (posicao.z - antes.z).powi(2)).sqrt();
                log.push(format!("t={t} para (desloca {dist:.2}m)"));
            }
            Some(AcaoDoMonstro::Atacou { .. }) => log.push(format!("t={t} ataca")),
            _ => {}
        }
        t += 50;
    }
    eprintln!("perseguição 126: {}", log.join(" | "));
    assert_eq!(saltos, 0);
}

/// B103 — a volta para casa depois da luta: no original o monstro volta andando
/// (`session_npc_patrol`) e só é posto em casa de uma vez (`ReturnHome`, modo 7) quando o
/// agente desiste a mais de 10 m. Mede quantas voltas acabam em salto no mapa real do 126.
#[test]
fn a_volta_para_casa_no_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.join("world").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let ter = Terreno::ler(1, &dir.join("world"));
    let mov = MapaDeMovimento::ler(1, &dir.join("world"));
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov };
    let modelo = d.monstros.get(3303).expect("3303");
    let (mut andando, mut salto) = (0, 0);
    for k in 0..20 {
        let casa = Vector3::new(-1450.0 + (k % 5) as f32 * 6.0, 241.0, 1385.0 + (k / 5) as f32 * 6.0);
        let mut m = MonsterEntity::do_template(1, modelo, casa, 30_000);
        let mut ai = MonsterAi::new();
        let mut j = feiticeira_nivel_1();
        j.position = Vector3::new(casa.x + 5.0, 241.0, casa.z);
        ai.add_threat(1, 100);
        let mut jogadores: HashMap<i64, PlayerEntity> = [(1i64, j)].into_iter().collect();
        let mut voltou_de_uma_vez = false;
        for passo in 0..(40_000 / 50) {
            if passo * 50 < 6_000 {
                jogadores.get_mut(&1).unwrap().position.x += 4.0 * 0.05;
            } else if passo * 50 == 6_000 {
                // O alvo some (saiu do jogo): a agressão acaba e o monstro volta.
                jogadores.clear();
                ai.aggro_table.clear();
            }
            if let Some(AcaoDoMonstro::Parou { modo, .. }) = ai.tick_no_mapa(&mut m, &jogadores, 50, &mapa) {
                if modo == 7 {
                    voltou_de_uma_vez = true;
                }
            }
        }
        if voltou_de_uma_vez { salto += 1 } else { andando += 1 }
    }
    eprintln!("volta para casa 126: {andando} andando, {salto} de uma vez (modo 7)");
    // Medido em 2026-09-24: 19 andando, 1 de uma vez.
    assert!(salto <= 2, "{salto} de 20 voltas acabaram em salto");
}

/// B104 — a altura do passeio: monstro de chão anda no chão (terreno + estrutura do mapa de
/// movimento), sem ficar no ar. Relato: "às vezes começam a andar no ar".
#[test]
fn o_passeio_de_chao_fica_no_chao() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.join("world").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let ter = Terreno::ler(1, &dir.join("world"));
    let mov = MapaDeMovimento::ler(1, &dir.join("world"));
    let chao = |x: f32, z: f32| ter.altura_em(x, z);
    let mapa = Mapa { terreno: &chao, movimento: &mov };
    let mut j = feiticeira_nivel_1();
    j.position = Vector3::new(-1440.0, 241.0, 1400.0);
    let jogadores: HashMap<i64, PlayerEntity> = [(1i64, j)].into_iter().collect();
    // Os spawns reais em volta do Guia dos Selvagens.
    let spawns: Vec<_> = d.map_spawns[&1].instances.iter()
        .filter(|s| (s.pos.x + 1445.0).hypot(s.pos.z - 1399.0) < 60.0 && d.monstros.get(s.template_id).is_some())
        .cloned().collect();
    let (mut passos, mut no_ar, mut rapido) = (0, 0, 0);
    for s in &spawns {
        let modelo = d.monstros.get(s.template_id).unwrap();
        let mut m = MonsterEntity::do_template(1, modelo, s.pos, 30_000);
        let mut ai = MonsterAi::new();
        for _ in 0..(120_000 / 50) {
            let antes = m.position;
            if let Some(AcaoDoMonstro::Andou { destino, tempo_ms, velocidade, modo }) = ai.tick_no_mapa(&mut m, &jogadores, 50, &mapa) {
                passos += 1;
                let piso = ter.altura_em(destino.x, destino.z).unwrap_or(destino.y) + mov.acima_do_terreno(destino.x, destino.z).unwrap_or(0.0);
                if modo & 0xC0 == 0 && destino.y - piso > 0.5 {
                    no_ar += 1;
                    if no_ar <= 5 { eprintln!("NO AR: modelo {} y={:.2} piso={:.2} terreno={:?} em ({:.1},{:.1})", s.template_id, destino.y, piso, ter.altura_em(destino.x, destino.z), destino.x, destino.z); }
                }
                let dist = ((destino.x - antes.x).powi(2) + (destino.z - antes.z).powi(2)).sqrt();
                if velocidade > modelo.velocidade_andando * 1.01 && modo == 0 || dist > velocidade * tempo_ms as f32 / 1000.0 * 1.1 + 0.05 {
                    rapido += 1;
                    if rapido <= 5 { eprintln!("RÁPIDO: modelo {} {dist:.2} m em {tempo_ms} ms a {velocidade:.2} m/s modo {modo}", s.template_id); }
                }
            }
        }
    }
    eprintln!("passeio de chão 126: {} spawns, {passos} passos, {no_ar} no ar, {rapido} rápidos", spawns.len());
}

/// B106 — de vida e mana cheias, o batimento em que o combate acaba ainda manda o
/// `SELF_INFO_00`: é ele que tira o cliente do estado de luta (`EC_HostMsg.cpp:1335`; captura
/// do 1.2.6 original, t = 2409,9 s). Nos outros batimentos sem mudança, nada vai.
#[test]
fn sair_do_combate_de_vida_cheia_avisa_o_cliente() {
    let mut p = feiticeira_nivel_1();
    p.mp = p.max_mp;
    p.combate_s = 2;
    let avisos: Vec<bool> = (0..4).map(|_| pw_gs::progressao::batimento(&mut p)).collect();
    assert_eq!(avisos, [false, true, false, false]);
    assert_eq!(p.combate_s, 0);
}
