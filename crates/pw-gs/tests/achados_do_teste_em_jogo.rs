//! Os defeitos que o teste em jogo de 2026-09-07 revelou, cada um com o seu teste.
//!
//! São problemas de causas independentes que apareceram na mesma sessão, com dois clients
//! 1.5.5 reais no realm 155BR. O que os une é só a origem.

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
        crit_rate: 0.0,
        position: pos,
        target_id: None,
        buffs: Vec::new(),
        voando: false,
        modo_roupa: false,
    };
    p.hp = p.max_hp;
    p
}

fn monstro(pos: Vector3) -> MonsterEntity {
    let mut m = MonsterEntity::placeholder(900_001, 1001, pos, 30_000);
    m.attack_range = 2.0;
    m.aggro_range = 30.0;
    m.move_speed = 4.0;
    m
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
    // 4 m/s × 50 ms = 0,2 m por tique; o aviso sai ao passar de 2 m, ou seja no 10º.
    for _ in 0..40 {
        if let Some(AcaoDoMonstro::Andou { destino, velocidade }) = ai.tick(&mut m, &players, 50) {
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
        if matches!(ai.tick(&mut m, &players, 50), Some(AcaoDoMonstro::Andou { .. })) {
            avisos += 1;
        }
    }
    // 200 tiques × 0,2 m = 40 m percorridos, com aviso a cada 2 m: cerca de 20.
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

    ai.tick(&mut m, &players, 50);
    assert_eq!(m.position, Vector3::new(0.0, 0.0, 0.0), "perseguiu além do próprio raio de ódio");
    assert!(ai.aggro_table.is_empty(), "devia ter perdido o alvo");
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
