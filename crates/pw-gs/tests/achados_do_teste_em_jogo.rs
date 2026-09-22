//! Os defeitos que o teste em jogo de 2026-09-07 revelou, cada um com o seu teste.
//!
//! São problemas de causas independentes que apareceram na mesma sessão, com dois clients
//! 1.5.5 reais no realm 155. O que os une é só a origem.

use pw_core::Vector3;
use pw_gs::ai::{AcaoDoMonstro, MonsterAi};
use pw_gs::entity::{MonsterEntity, PlayerEntity};
use pw_protocol::packets::s2c::S2CGamedataSend;

/// Um jogador simples para a IA ter em quem bater.
fn jogador(pos: Vector3) -> PlayerEntity {
    let mut p = PlayerEntity {
        role_id: 1,
        name: "Alvo".into(),
        race: pw_core::Race::Human,
        cls: pw_core::CharacterClass::Blademaster,
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
        strength: 10,
        agility: 10,
        vitality: 10,
        energy: 10,
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

fn sem_mapa(_x: f32, _z: f32) -> Option<f32> {
    None
}

fn monstro(pos: Vector3) -> MonsterEntity {
    let mut m = MonsterEntity::placeholder(900_001, 1001, pos, 30_000);
    m.attack_range = 2.0;
    m.aggro_range = 30.0;
    m.move_speed = 4.0;
    m
}

// ---------------------------------------------------------------------------------
// "nenhum monstro ataca só de chegar perto" (B76)
// ---------------------------------------------------------------------------------

/// `aggressive_mode` do `MONSTER_ESSENCE`: o monstro marcado assim recebe o aviso de
/// movimento do jogador (`MSG_MASK_PLAYER_MOVE`, `npcgenerator.cpp:2534-2537`) num raio de
/// `GetMaxMobSightRange` — 15 m (`playerctrl.cpp:265-276`, `worldmanager.cpp:48`) — e parte
/// para cima dele. O passivo só reage a quem bate.
#[test]
fn o_monstro_agressivo_ataca_quem_chega_perto() {
    let perto = |d: f32| {
        let mut players = std::collections::HashMap::new();
        players.insert(1i64, jogador(Vector3::new(d, 0.0, 0.0)));
        players
    };

    // Passivo: ninguém o incomoda, ele não sai do lugar por causa do jogador.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.agressivo = false;
    ai.tick(&mut m, &perto(5.0), 50, &sem_mapa);
    assert_eq!(ai.get_highest_threat_target(), None, "monstro passivo não pode odiar sozinho");

    // Agressivo, com o jogador dentro dos 15 m: pega o alvo.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.agressivo = true;
    ai.tick(&mut m, &perto(5.0), 50, &sem_mapa);
    assert_eq!(ai.get_highest_threat_target(), Some(1), "o agressivo não viu o jogador a 5 m");

    // Agressivo, mas longe demais: o aviso do original não chega.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.agressivo = true;
    ai.tick(&mut m, &perto(20.0), 50, &sem_mapa);
    assert_eq!(ai.get_highest_threat_target(), None, "o agressivo enxergou além dos 15 m");

    // E morto não vê ninguém.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.agressivo = true;
    m.is_dead = true;
    ai.tick(&mut m, &perto(5.0), 50, &sem_mapa);
    assert_eq!(ai.get_highest_threat_target(), None);
}

// ---------------------------------------------------------------------------------
// "os monstros não estão se movendo"
// ---------------------------------------------------------------------------------

#[test]
fn o_monstro_que_persegue_anuncia_o_movimento() {
    // A IA mexia em `monster.position` e não devolvia nada: nem a grade espacial sabia,
    // nem o cliente. O monstro andava e na tela ficava parado.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let alvo = jogador(Vector3::new(20.0, 0.0, 0.0));
    let mut players = std::collections::HashMap::new();
    players.insert(1i64, alvo);
    ai.add_threat(1, 10);

    let mut andou = None;
    // Um passo a cada 0,5 s (`NPC_FOLLOW_TARGET_TIME`): 4 m/s dão 2 m por passo.
    for _ in 0..40 {
        if let Some(AcaoDoMonstro::Andou { destino, velocidade, .. }) = ai.tick(&mut m, &players, 50, &sem_mapa) {
            andou = Some((destino, velocidade));
            break;
        }
    }
    let (destino, velocidade) = andou.expect("o monstro perseguiu e nunca avisou");
    assert_eq!(velocidade, 4.0);
    assert!(destino.x > 1.5, "andou pouco demais para valer um aviso: {destino:?}");
    assert!(destino.x < 20.0, "passou do alvo: {destino:?}");
    assert_eq!(m.position, destino, "o aviso tem de ser a posição de verdade");
}

#[test]
fn o_monstro_nao_anuncia_a_cada_tique() {
    // Um `OBJECT_MOVE` por tique de 50 ms seriam 20 pacotes por segundo por monstro em
    // perseguição, sem ganho nenhum na tela: o cliente interpola entre um aviso e o outro.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut players = std::collections::HashMap::new();
    players.insert(1i64, jogador(Vector3::new(200.0, 0.0, 0.0)));
    // Alvo longe: o monstro persegue sem alcançar, então dá para contar os avisos.
    m.aggro_range = 500.0;
    ai.add_threat(1, 10);

    let mut avisos = 0;
    for _ in 0..200 {
        if matches!(ai.tick(&mut m, &players, 50, &sem_mapa), Some(AcaoDoMonstro::Andou { .. })) {
            avisos += 1;
        }
    }
    // 200 tiques = 10 s, um passo a cada 0,5 s: 20 avisos.
    assert!(avisos > 0, "nenhum aviso em 200 tiques");
    assert!(avisos <= 25, "{avisos} avisos em 200 tiques — está mandando quase por tique");
}

#[test]
fn a_perseguicao_usa_o_aggro_range_do_monstro() {
    // Era `35.0` escrito no código para todo monstro do jogo; agora vem do
    // `elements.data`.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.aggro_range = 10.0;
    let mut players = std::collections::HashMap::new();
    // 25 m: dentro dos 35 antigos, fora dos 10 deste monstro.
    players.insert(1i64, jogador(Vector3::new(25.0, 0.0, 0.0)));
    ai.add_threat(1, 10);

    ai.tick(&mut m, &players, 50, &sem_mapa);
    assert_eq!(m.position, Vector3::new(0.0, 0.0, 0.0), "perseguiu além do próprio raio de ódio");
    assert!(ai.aggro_table.is_empty(), "devia ter perdido o alvo");
}

// ---------------------------------------------------------------------------------
// "quando entram em fúria não andam respeitando o terreno, e se movem rápido demais"
// (teste de 2026-09-12)
// ---------------------------------------------------------------------------------

#[test]
fn o_monstro_de_chao_persegue_assentado_no_terreno() {
    // Terreno em rampa: sobe 1 m a cada metro em x. O monstro nasceu em y = 0 e o
    // jogador está no alto; antes o `y` do monstro ficava fixo e ele entrava no morro.
    let rampa = |x: f32, _z: f32| Some(x);
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut players = std::collections::HashMap::new();
    players.insert(1i64, jogador(Vector3::new(20.0, 20.0, 0.0)));
    ai.add_threat(1, 10);

    let mut passos = 0;
    for _ in 0..60 {
        if let Some(AcaoDoMonstro::Andou { destino, .. }) = ai.tick(&mut m, &players, 50, &rampa) {
            assert!((destino.y - destino.x).abs() < 1e-3, "fora do chão: {destino:?}");
            passos += 1;
        }
    }
    assert!(passos >= 3, "{passos} passos");
}

#[test]
fn o_movimento_do_monstro_vai_nas_unidades_do_original() {
    // `gs/npcsession.cpp:258`: `cost_time` em ms e `speed × 256`. Ia centésimo de segundo
    // e ×100, e o cliente fazia o trecho de 2 m em 50 ms.
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut players = std::collections::HashMap::new();
    players.insert(1i64, jogador(Vector3::new(20.0, 0.0, 0.0)));
    ai.add_threat(1, 10);
    let Some(AcaoDoMonstro::Andou { tempo_ms, velocidade, modo, .. }) = ai.tick(&mut m, &players, 50, &sem_mapa) else {
        panic!("o primeiro tique da perseguição dá um passo");
    };
    assert_eq!(tempo_ms, 500);
    assert_eq!(AcaoDoMonstro::velocidade_no_protocolo(velocidade), 1024);
    assert_eq!(modo, pw_gs::ai::MODO_CORRER);
}

#[test]
fn o_monstro_ocioso_passeia_perto_de_onde_nasceu_e_so_com_jogador_perto() {
    let plano = |_x: f32, _z: f32| Some(5.0);
    let mut m = monstro(Vector3::new(100.0, 5.0, 100.0));
    m.patrulha = true;
    m.walk_speed = 1.5;

    // Sem ninguém por perto, fica parado: o `idle_timer` do original nunca liga.
    let mut ai = MonsterAi::new();
    let ninguem = std::collections::HashMap::new();
    for _ in 0..(40 * 20) {
        assert!(ai.tick(&mut m, &ninguem, 50, &plano).is_none());
    }

    // Com jogador a 50 m, o contador de 32 batimentos dá a volta e ele sai andando.
    let mut perto = std::collections::HashMap::new();
    perto.insert(1i64, jogador(Vector3::new(150.0, 5.0, 100.0)));
    let mut andou = 0;
    for _ in 0..(120 * 20) {
        match ai.tick(&mut m, &perto, 50, &plano) {
            Some(AcaoDoMonstro::Andou { destino, tempo_ms, modo, .. }) => {
                andou += 1;
                assert_eq!(tempo_ms, 1000);
                assert_eq!(modo, pw_gs::ai::MODO_ANDAR);
                assert_eq!(destino.y, 5.0);
                let dx = destino.x - 100.0;
                let dz = destino.z - 100.0;
                assert!(dx.abs() <= 10.01 && dz.abs() <= 10.01, "saiu do raio de 10 m: {destino:?}");
            }
            _ => {}
        }
    }
    assert!(andou > 0, "em 2 minutos com jogador perto não passeou nenhuma vez");
}

#[test]
fn sem_alvo_o_monstro_volta_para_onde_nasceu() {
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.aggro_range = 30.0;
    let mut players = std::collections::HashMap::new();
    players.insert(1i64, jogador(Vector3::new(25.0, 0.0, 0.0)));
    ai.add_threat(1, 10);
    for _ in 0..40 {
        ai.tick(&mut m, &players, 50, &sem_mapa);
    }
    assert!(m.position.x > 5.0, "não perseguiu: {:?}", m.position);

    // O jogador foge para longe: perde o alvo e corre de volta.
    players.insert(1i64, jogador(Vector3::new(500.0, 0.0, 0.0)));
    let mut parou = false;
    for _ in 0..(30 * 20) {
        if let Some(AcaoDoMonstro::Parou { posicao, .. }) = ai.tick(&mut m, &players, 50, &sem_mapa) {
            if posicao.x.abs() < 0.1 {
                parou = true;
                break;
            }
        }
    }
    assert!(parou, "não voltou para casa: {:?}", m.position);
}

// ---------------------------------------------------------------------------------
// "não consigo desequipar, está protegido por senha"
// ---------------------------------------------------------------------------------

#[test]
fn a_resposta_de_senha_nao_tem_corpo() {
    // O IR marca `payload: empty` para `SECURITY_PASSWD_CHECKED` (277), e
    // `CECHostPlayer::OnMsgPlayerPasswdChecked` não lê byte nenhum. Um corpo a mais faria
    // o cliente descartar o comando pelo tamanho, e o guarda-roupa continuaria trancado.
    let p = S2CGamedataSend::security_passwd_checked().data;
    assert_eq!(p.len(), 2, "o comando 277 é só o cabeçalho: {p:?}");
    assert_eq!(u16::from_le_bytes([p[0], p[1]]), 277);
}

#[test]
fn o_estado_da_senha_tem_um_byte() {
    // `struct cmd_trashbox_pwd_state { unsigned char has_passwd; }`.
    let p = S2CGamedataSend::trashbox_pwd_state(false).data;
    assert_eq!(p.len(), 3, "cabeçalho de 2 mais um byte: {p:?}");
    assert_eq!(u16::from_le_bytes([p[0], p[1]]), 129);
    assert_eq!(p[2], 0);
    assert_eq!(S2CGamedataSend::trashbox_pwd_state(true).data[2], 1);
}

// ---------------------------------------------------------------------------------
// "meus personagens não têm skills que possam ser usadas"
// ---------------------------------------------------------------------------------

#[test]
fn a_lista_de_habilidades_tem_o_layout_do_cliente() {
    // `struct cmd_skill_data { size_t skill_count; struct { short id; unsigned char level;
    // short ability; } list[]; }` sob `#pragma pack(1)`: 4 bytes de contagem e 5 por
    // habilidade. Tamanho errado faz o cliente descartar o comando inteiro, e a barra de
    // habilidades fica vazia — que é o sintoma relatado.
    let skills = vec![
        pw_core::LearnedSkill { character_id: 42, skill_id: 11, level: 1 },
        pw_core::LearnedSkill { character_id: 42, skill_id: 167, level: 3 },
    ];
    let p = S2CGamedataSend::skill_data_from_records(&skills).data;
    assert_eq!(p.len(), 2 + 4 + 2 * 5, "tamanho fora do layout: {} bytes", p.len());
    assert_eq!(u16::from_le_bytes([p[0], p[1]]), 90);
    assert_eq!(u32::from_le_bytes([p[2], p[3], p[4], p[5]]), 2);
    assert_eq!(i16::from_le_bytes([p[6], p[7]]), 11);
    assert_eq!(p[8], 1);
    assert_eq!(i16::from_le_bytes([p[11], p[12]]), 167);
    assert_eq!(p[13], 3);

    // Lista vazia continua sendo um comando válido: o cliente precisa saber que não há
    // habilidade, e não ficar esperando.
    let vazio = S2CGamedataSend::skill_data_from_records(&[]).data;
    assert_eq!(vazio.len(), 6);
    assert_eq!(u32::from_le_bytes([vazio[2], vazio[3], vazio[4], vazio[5]]), 0);
}

/// B59 — o monstro do teste em jogo (Lobo Sangrento: alcance 3 m, ódio 35 m) tem de
/// perseguir um arqueiro a 20 m e **bater** quando chegar.
///
/// Relato de 2026-09-17: "quando ele se aproxima de mim não está me atacando".
#[test]
fn o_monstro_persegue_o_arqueiro_de_longe_e_bate() {
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.attack_range = 3.0;
    m.aggro_range = 35.0;
    m.move_speed = 4.0;
    m.attack_min = 10;
    m.attack_max = 20;
    let alvo = jogador(Vector3::new(20.0, 0.0, 0.0));
    let mut players = std::collections::HashMap::new();
    players.insert(1i64, alvo);
    ai.add_threat(1, 100);

    let mut bateu = false;
    let mut passos = 0;
    for _ in 0..400 {
        match ai.tick(&mut m, &players, 50, &sem_mapa) {
            Some(AcaoDoMonstro::Atacou { alvo, .. }) => {
                assert_eq!(alvo, 1);
                bateu = true;
                break;
            }
            Some(AcaoDoMonstro::Andou { destino, .. }) => {
                m.position = destino;
                passos += 1;
            }
            _ => {}
        }
    }
    assert!(
        bateu,
        "o monstro não bateu em 20 s: {passos} passos, distância final {:.1} m",
        m.position.distance(&Vector3::new(20.0, 0.0, 0.0))
    );
}

/// B59 — a direção com que a criatura nasce: `GenDir()` do original
/// (`npcgenerator.h:747-757`).
///
/// Relato de 2026-09-17: "os NPCs estão todos virados para a mesma direção". Mandávamos
/// zero para todos.
#[test]
fn a_direcao_do_gerador_e_a_do_original() {
    use pw_gs::entity::direcao_do_gerador;
    let ponto = Vector3::new(0.0, 0.0, 0.0);

    // Área que é um ponto: a direção do gerador, `atan2(z, x) × 128/π`.
    assert_eq!(direcao_do_gerador(Vector3::new(1.0, 0.0, 0.0), ponto), 0, "leste = 0");
    assert_eq!(direcao_do_gerador(Vector3::new(0.0, 0.0, 1.0), ponto), 64, "norte = 1/4 de volta");
    assert_eq!(direcao_do_gerador(Vector3::new(-1.0, 0.0, 0.0), ponto), 128, "oeste = 1/2 volta");
    // `atan2` negativo vira o complemento pelo `& 0xFF` do original.
    assert_eq!(direcao_do_gerador(Vector3::new(0.0, 0.0, -1.0), ponto), 192, "sul = 3/4 de volta");

    // Área com extensão: direção sorteada — o que se garante é que ela varia.
    let caixa = Vector3::new(20.0, 0.0, 20.0);
    let dir = Vector3::new(1.0, 0.0, 0.0);
    let mut vistas = std::collections::HashSet::new();
    for _ in 0..200 {
        vistas.insert(direcao_do_gerador(dir, caixa));
    }
    assert!(vistas.len() > 10, "área com extensão devia sortear a direção: {} valores", vistas.len());
}
