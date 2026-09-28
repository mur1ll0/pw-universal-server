//! A IA de combate do monstro por estratégia, eventos de vida e habilidades
//! (`aipolicy.cpp:515-1110`, `aipolicy.h:1013-1057`, `1214-1370`, `npcsession.cpp`).
//!
//! Cada teste monta o perfil à mão — estratégia, habilidades e eventos — e confere a decisão
//! que o original toma na mesma situação.

use pw_core::Vector3;
use pw_gs::ai::{
    AcaoDoMonstro, Estrategia, EventoDeVida, HabilidadeDeCriatura, MonsterAi, PerfilDeCombate,
};
use pw_gs::entity::{MonsterEntity, PlayerEntity};
use std::collections::HashMap;

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
        montaria: None,
        forma_enviada: None,
        passivas_de_forma: Default::default(),
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
        meditacao_s: 0,
        chi_ao_meditar: 15,
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
    Some(0.0)
}

fn monstro(pos: Vector3) -> MonsterEntity {
    let mut m = MonsterEntity::placeholder(900_001, 1001, pos, 30_000);
    m.attack_range = 2.0;
    m.aggro_range = 30.0;
    m.move_speed = 4.0;
    m.ataque_em_ticks = 20;
    m
}

fn habilidade(id: i32, tipo: i32, alcance: f32, canto_ms: u32) -> HabilidadeDeCriatura {
    HabilidadeDeCriatura {
        id,
        nivel: 1,
        tipo,
        area: 0,
        alcance,
        canto_ms,
        execucao_ms: 500,
        recarga_ms: 0,
        mana: 0,
    }
}

fn perfil(estrategia: Estrategia) -> PerfilDeCombate {
    PerfilDeCombate {
        estrategia,
        corpo: 0.5,
        ..Default::default()
    }
}

fn com_alvo(d: f32) -> HashMap<i64, PlayerEntity> {
    HashMap::from([(1i64, jogador(Vector3::new(d, 0.0, 0.0)))])
}

/// Roda a IA até a primeira ação que satisfaça `achou`, no máximo `ms`.
fn ate(
    ai: &mut MonsterAi,
    m: &mut MonsterEntity,
    p: &HashMap<i64, PlayerEntity>,
    ms: u32,
    achou: impl Fn(&AcaoDoMonstro) -> bool,
) -> Option<AcaoDoMonstro> {
    let mut t = 0;
    while t < ms {
        if let Some(a) = ai.tick(m, p, 50, &sem_mapa) {
            if achou(&a) {
                return Some(a);
            }
        }
        t += 50;
    }
    None
}

/// Corpo a corpo e magia (3), o alvo longe do golpe mas dentro do alcance mágico: o monstro
/// **conjura** de onde está em vez de correr (`ai_magic_melee_task::Execute`,
/// `aipolicy.cpp:1042-1066`: `range > sa` → `session_npc_skill`). É o "ataca de longe quando
/// atacado de longe" do relato do Murillo.
#[test]
fn corpo_a_corpo_e_magia_conjura_no_alvo_que_esta_longe() {
    let mut p = perfil(Estrategia::CorpoACorpoEMagia);
    p.ataque = vec![habilidade(2001, 1, 20.0, 1000)];
    let mut ai = MonsterAi::com_perfil(Some(p));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    ai.add_threat(1, 10);
    let jogadores = com_alvo(12.0);

    let a = ai.tick(&mut m, &jogadores, 50, &sem_mapa);
    match a {
        Some(AcaoDoMonstro::Conjurou {
            habilidade,
            alvo,
            instantanea,
        }) => {
            assert_eq!((habilidade.id, alvo, instantanea), (2001, 1, false));
        }
        outra => panic!("devia conjurar, fez {outra:?}"),
    }
    assert_eq!(m.position.x, 0.0, "não saiu do lugar");
    // O efeito sai no fim do canto (1 s), não antes.
    let efeito = ate(&mut ai, &mut m, &jogadores, 1000, |a| {
        matches!(a, AcaoDoMonstro::UsouHabilidade { .. })
    });
    assert!(
        matches!(efeito, Some(AcaoDoMonstro::UsouHabilidade { alvo: 1, .. })),
        "efeito ao fim do canto: {efeito:?}"
    );
}

/// Perto, o mesmo monstro bate uma série de `(195 + Rand(10, 20)) / (attack_speed + 1)` golpes
/// e depois conjura (`SetAttackTimes`, `aipolicy.cpp:1096-1105`; `STATE_PHYSC` →
/// magia, `:1057`).
#[test]
fn corpo_a_corpo_e_magia_bate_uma_serie_e_depois_conjura() {
    let mut p = perfil(Estrategia::CorpoACorpoEMagia);
    p.ataque = vec![habilidade(2001, 1, 20.0, 1000)];
    let mut ai = MonsterAi::com_perfil(Some(p));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    ai.add_threat(1, 10);
    let jogadores = com_alvo(1.5);

    let mut golpes = 0;
    let mut t = 0;
    let conjurou = loop {
        assert!(t < 60_000, "não conjurou em 60 s");
        match ai.tick(&mut m, &jogadores, 50, &sem_mapa) {
            Some(AcaoDoMonstro::Atacou { .. }) => golpes += 1,
            Some(AcaoDoMonstro::Conjurou { .. }) => break true,
            _ => {}
        }
        t += 50;
    };
    assert!(conjurou);
    // `attack_speed` 20 tiques: (195 + 10..=20) / 21 = 9 ou 10 golpes.
    assert!((9..=10).contains(&golpes), "{golpes} golpes antes da magia");
}

/// Magia (2): só conjura; com o alvo muito perto (menos da metade do alcance) afasta-se até
/// duas vezes (`ST_KO_COUNT`, `aipolicy.cpp:903-913`).
#[test]
fn magia_se_afasta_de_quem_chega_perto_e_conjura() {
    let mut p = perfil(Estrategia::Magia);
    p.ataque = vec![habilidade(2002, 1, 20.0, 500)];
    let mut ai = MonsterAi::com_perfil(Some(p));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    ai.add_threat(1, 10);
    let jogadores = com_alvo(3.0);

    let a = ai.tick(&mut m, &jogadores, 50, &sem_mapa);
    assert!(
        matches!(a, Some(AcaoDoMonstro::Andou { .. })),
        "devia se afastar: {a:?}"
    );
    assert!(
        m.position.x < 0.0,
        "afastou para o lado oposto ao do alvo: x = {}",
        m.position.x
    );
    let c = ate(&mut ai, &mut m, &jogadores, 10_000, |a| {
        matches!(a, AcaoDoMonstro::Conjurou { .. })
    });
    assert!(c.is_some(), "depois de se afastar, conjura");
    let atacou = ate(&mut ai, &mut m, &jogadores, 5_000, |a| {
        matches!(a, AcaoDoMonstro::Atacou { .. })
    });
    assert!(atacou.is_none(), "o mágico não dá golpe normal");
}

/// Fixo (4): não sai do lugar; fora do alcance não persegue.
#[test]
fn fixo_nao_persegue() {
    let mut ai = MonsterAi::com_perfil(Some(perfil(Estrategia::Fixo)));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    ai.add_threat(1, 10);
    let longe = com_alvo(8.0);
    assert!(
        ate(&mut ai, &mut m, &longe, 3_000, |_| true).is_none(),
        "longe: parado e quieto"
    );
    assert_eq!(m.position.x, 0.0);
    let perto = com_alvo(1.5);
    let a = ate(&mut ai, &mut m, &perto, 3_000, |a| {
        matches!(a, AcaoDoMonstro::Atacou { .. })
    });
    assert!(a.is_some(), "perto: bate");
    assert_eq!(m.position.x, 0.0);
}

/// Distância (1): bate de longe, sem se aproximar, com o alcance do `attack_range`.
#[test]
fn a_distancia_bate_de_longe() {
    let mut ai = MonsterAi::com_perfil(Some(perfil(Estrategia::Distancia)));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.attack_range = 15.0;
    ai.add_threat(1, 10);
    let jogadores = com_alvo(12.0);
    let a = ai.tick(&mut m, &jogadores, 50, &sem_mapa);
    assert!(
        matches!(a, Some(AcaoDoMonstro::Atacou { alvo: 1, .. })),
        "bate de onde está: {a:?}"
    );
    assert_eq!(m.position.x, 0.0);
}

/// Inerte (6): nada, nem com ódio.
#[test]
fn inerte_nao_faz_nada() {
    let mut ai = MonsterAi::com_perfil(Some(perfil(Estrategia::Inerte)));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    ai.add_threat(1, 10);
    assert!(ate(&mut ai, &mut m, &com_alvo(1.0), 5_000, |_| true).is_none());
}

/// Fugitivo (5): corre para longe.
#[test]
fn fugitivo_foge() {
    let mut ai = MonsterAi::com_perfil(Some(perfil(Estrategia::Fugitivo)));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    ai.add_threat(1, 10);
    let jogadores = com_alvo(2.0);
    ate(&mut ai, &mut m, &jogadores, 3_000, |_| false);
    assert!(m.position.x < -3.0, "fugiu: x = {}", m.position.x);
}

/// Eventos de vida (`TriggerEvent`, `aipolicy.h:1343-1370`): abaixo de ¾ da vida sai o evento
/// de 75 %; uma queda direta para menos de ¼ dispara só **um** (o de 75 %), porque o
/// `_cur_event_hp` desce de uma vez até abaixo da vida.
#[test]
fn evento_de_vida_dispara_a_habilidade_do_limiar() {
    let mut p = perfil(Estrategia::CorpoACorpo);
    p.eventos = [
        Some(EventoDeVida::Habilidade(habilidade(25, 1, 10.0, 0))),
        Some(EventoDeVida::Habilidade(habilidade(50, 1, 10.0, 0))),
        Some(EventoDeVida::Habilidade(habilidade(75, 1, 10.0, 0))),
    ];
    let conjurados = |hp: i64, ms: u32| {
        let mut ai = MonsterAi::com_perfil(Some(p.clone()));
        let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
        m.max_hp = 1000;
        m.hp = hp;
        ai.add_threat(1, 10);
        let jogadores = com_alvo(1.5);
        let mut ids = Vec::new();
        let mut t = 0;
        while t < ms {
            if let Some(AcaoDoMonstro::Conjurou { habilidade, .. }) =
                ai.tick(&mut m, &jogadores, 50, &sem_mapa)
            {
                ids.push(habilidade.id);
            }
            t += 50;
        }
        ids
    };
    assert_eq!(
        conjurados(900, 3_000),
        Vec::<i32>::new(),
        "acima de ¾: nada"
    );
    assert_eq!(conjurados(700, 3_000), vec![75], "abaixo de ¾: o de 75 %");
    assert_eq!(
        conjurados(100, 3_000),
        vec![75],
        "queda direta: um evento só"
    );
}

/// Com política no `aipolicy.data` os eventos de vida não valem (`if(!_at_policy)`,
/// `aipolicy.cpp:344`).
#[test]
fn evento_de_vida_nao_vale_para_quem_tem_politica() {
    let mut p = perfil(Estrategia::CorpoACorpo);
    p.tem_politica = true;
    p.eventos[2] = Some(EventoDeVida::Habilidade(habilidade(75, 1, 10.0, 0)));
    let mut ai = MonsterAi::com_perfil(Some(p));
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.max_hp = 1000;
    m.hp = 500;
    ai.add_threat(1, 10);
    let c = ate(&mut ai, &mut m, &com_alvo(1.5), 3_000, |a| {
        matches!(a, AcaoDoMonstro::Conjurou { .. })
    });
    assert!(c.is_none());
}

/// O sorteio do evento (`abase::RandSelect`, `arandomgen.h:140-153`): cumulativo nas cinco
/// entradas; se a soma não fecha, o índice 0.
#[test]
fn sorteio_do_evento_de_vida_e_o_rand_select() {
    use pw_data_loader::monstros::{sortear_evento_de_vida, SkillPorVida};
    let e = |id, p| SkillPorVida {
        id,
        nivel: 1,
        probabilidade: p,
    };
    let lista = [e(0, 0.5), e(7, 0.3), e(9, 0.1), e(0, 0.0), e(0, 0.0)];
    assert_eq!(sortear_evento_de_vida(&lista, 0.2).0, 0);
    assert_eq!(sortear_evento_de_vida(&lista, 0.6).0, 7);
    assert_eq!(sortear_evento_de_vida(&lista, 0.85).0, 9);
    assert_eq!(
        sortear_evento_de_vida(&lista, 0.95).0,
        0,
        "soma 0,9: cai no índice 0"
    );
}

// ---------------------------------------------------------------------------------
// Perseguição, desistência e volta para casa (teste da Tsuko, 2026-09-26)
// ---------------------------------------------------------------------------------

/// Quem só foge (ou voa fora do alcance) nunca é golpeado: o `_cur_time` do
/// `aggro_policy` (= `aggro_time`) zera, o monstro esquece o alvo, volta para casa
/// invencível — longe mais de 10 m — e enche a vida fora de combate (`ainpc.h:259-275`,
/// `aipolicy.cpp:204-231`, `aipolicy.cpp:1291-1304`, `npc.cpp:1946-1958`).
#[test]
fn quem_so_foge_faz_o_monstro_desistir_voltar_invencivel_e_recuperar_a_vida() {
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.tempo_de_odio_s = 5;
    m.aggro_range = 200.0;
    m.regeneracao_de_vida = 3;
    m.hp = m.max_hp / 2;
    ai.add_threat(1, 10);
    let mut jogadores = com_alvo(8.0);

    let mut t = 0;
    while t < 20_000 && !ai.aggro_table.is_empty() {
        // O jogador corre sempre 8 m à frente: o monstro persegue e nunca alcança.
        let x = m.position.x + 8.0;
        jogadores.get_mut(&1).unwrap().position = Vector3::new(x, 0.0, 0.0);
        ai.tick(&mut m, &jogadores, 50, &sem_mapa);
        t += 50;
    }
    assert!(ai.aggro_table.is_empty(), "o ódio não acabou em 20 s");
    assert!((5_000..=7_000).contains(&t), "desistiu em {t} ms; o aggro_time era 5 s");
    assert!(m.position.x > 10.0, "devia ter se afastado de casa ({:?})", m.position);

    // Um tique depois: `RollBack` → volta para casa, invencível.
    jogadores.get_mut(&1).unwrap().position = Vector3::new(500.0, 0.0, 0.0);
    ai.tick(&mut m, &jogadores, 50, &sem_mapa);
    assert!(ai.esta_voltando());
    assert!(m.efeitos.invencivel(), "a volta põe o invincible_filter");
    assert!(m.efeitos.estados_visiveis()[1] & (1 << (49 - 32)) != 0, "estado visível 49 (1.5.5)");
    assert_eq!(pw_gs::efeitos::dano_recebido(&mut m.efeitos, 500), 0);
    // Apanhar voltando não reacende o ódio (`ai_returnhome_task::OnAggro`).
    ai.add_threat(1, 999);
    assert!(ai.aggro_table.is_empty());

    let mut t = 0;
    while t < 30_000 && ai.esta_voltando() {
        ai.tick(&mut m, &jogadores, 50, &sem_mapa);
        t += 50;
    }
    assert!(!ai.esta_voltando(), "não chegou em casa em 30 s");
    // A volta acaba a 1,2 passo de casa (passo = corrida × 1 s = 4 m, `npcsession.cpp:883-960`).
    assert!(m.position.distance(&m.spawn_center) < 4.8, "parou em {:?}", m.position);
    assert!(!m.efeitos.invencivel(), "o EndTask tira o invencível");
    assert_eq!(m.hp, m.max_hp, "fora de combate a vida enche");
}

/// Enquanto o monstro bate no primeiro da lista, o `RefreshAggroTimer` renova o ódio: ele não
/// desiste de quem fica ao alcance.
#[test]
fn quem_fica_ao_alcance_continua_odiado() {
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.tempo_de_odio_s = 3;
    ai.add_threat(1, 10);
    let jogadores = com_alvo(1.5);
    let mut golpes = 0;
    for _ in 0..(12_000 / 50) {
        if let Some(AcaoDoMonstro::Atacou { .. }) = ai.tick(&mut m, &jogadores, 50, &sem_mapa) {
            golpes += 1;
        }
    }
    assert!(golpes >= 5, "bateu só {golpes} vezes");
    assert_eq!(ai.get_highest_threat_target(), Some(1), "desistiu de quem estava ao alcance");
}

/// O agressivo nota quem anda a menos de `sight_range + body_size` (`ainpc.h:851-854`), não
/// os 15 m do aviso do jogador.
#[test]
fn o_agressivo_nota_so_dentro_do_sight_range() {
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.agressivo = true;
    m.sight_range = 6;
    m.tamanho = 1.0;
    assert_eq!(MonsterAi::raio_de_deteccao(&m), 7.0);

    let mut ai = MonsterAi::new();
    ai.tick(&mut m, &com_alvo(10.0), 50, &sem_mapa);
    assert!(ai.aggro_table.is_empty(), "notou o jogador a 10 m com raio de 7 m");

    ai.tick(&mut m, &com_alvo(6.5), 50, &sem_mapa);
    assert_eq!(ai.get_highest_threat_target(), Some(1));
}

/// `gnpc_imp::AdjustDamage` (`npc.cpp:1727-1768`): do ar num monstro de chão, metade.
#[test]
fn golpe_do_ar_no_monstro_de_chao_vale_metade() {
    use pw_gs::combat::{self, ajuste_de_camada_no_npc, Camada, CombatEngine, Resultado, Rolagens};
    assert_eq!(ajuste_de_camada_no_npc(Camada::Ar, Camada::Chao), 0.5);
    assert_eq!(ajuste_de_camada_no_npc(Camada::Chao, Camada::Ar), 1.0);
    assert_eq!(ajuste_de_camada_no_npc(Camada::Ar, Camada::Ar), 1.0);
    assert_eq!(ajuste_de_camada_no_npc(Camada::Agua, Camada::Chao), 0.5);

    let m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut p = jogador(Vector3::new(1.0, 0.0, 0.0));
    let certo = Rolagens { acerto: 0.0, critico: 99 };
    let mut g = CombatEngine::golpe_de_jogador(&p);
    g.dano_fisico = 1000;
    let chao = combat::resolver(&g, &CombatEngine::defesa_do_monstro(&m), 1.0, false, certo);
    p.voando = true;
    let mut g = CombatEngine::golpe_de_jogador(&p);
    g.dano_fisico = 1000;
    let ar = combat::resolver(&g, &CombatEngine::defesa_do_monstro(&m), 1.0, false, certo);
    match (chao, ar) {
        (Resultado::Acertou { dano: a, .. }, Resultado::Acertou { dano: b, .. }) => {
            assert!((b - a / 2).abs() <= 1, "do chão {a}, do ar {b}");
        }
        outro => panic!("{outro:?}"),
    }
}

/// O `speed_increase` do item de voo sai do offset 20 do conteúdo gravado — o da Tsuko
/// (15732, "15 m/s") no banco em 2026-09-26.
#[test]
fn a_velocidade_do_item_de_voo_vem_do_conteudo() {
    let hex = "c20100008403000001000400180000000f00000000007041000070410000";
    let b: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
    assert_eq!(pw_gs::entity::velocidade_do_item_de_voo(&b), Some(15.0));
}

/// B134 — `ai_target_task::OnSessionEnd` (`aipolicy.cpp:443-480`): o monstro de chão embaixo de
/// quem bate voando "chega" três vezes sem alcançar (`TEST_GETTOGOAL`) → `NSRC_ERR_PATHFINDING`,
/// e contra **jogador** limpa o ódio e o registro de dano. Antes ele ficava parado embaixo, sem
/// bater em ninguém (teste da Tsuko com o Esqueleto Espectral).
#[test]
fn embaixo_de_quem_voa_o_monstro_de_chao_desiste_do_jogador() {
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.tempo_de_odio_s = 600; // o temporizador não é quem decide aqui
    ai.add_threat(1, 50);
    m.registrar_dano(1, 50);
    let mut jogadores = com_alvo(0.5);
    jogadores.get_mut(&1).unwrap().position = Vector3::new(0.5, 20.0, 0.0);
    let mut t = 0;
    while t < 10_000 && !ai.aggro_table.is_empty() {
        ai.tick(&mut m, &jogadores, 50, &sem_mapa);
        t += 50;
    }
    assert!(ai.aggro_table.is_empty(), "continuou odiando quem não alcança");
    assert!(m.danos.is_empty(), "ClearDamageList");
    assert!(t < 5_000, "levou {t} ms");
}

/// Contra um **mascote** que não alcança, a tarefa só recomeça: o ódio fica.
#[test]
fn contra_mascote_fora_de_alcance_o_odio_fica() {
    use pw_gs::navegacao::Mapa;
    let mut ai = MonsterAi::new();
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    m.tempo_de_odio_s = 600;
    ai.add_threat(2, 50);
    let mut pet = MonsterEntity::placeholder(2, 10521, Vector3::new(0.5, 20.0, 0.0), 0);
    pet.hp = 100;
    let mascotes = HashMap::from([(2i64, pet)]);
    let vazio = pw_data_loader::MapaDeMovimento::vazio();
    let mapa = Mapa { terreno: &sem_mapa, movimento: &vazio, espaco: None, agua: None };
    for _ in 0..(10_000 / 50) {
        ai.tick_com_mascotes(&mut m, &HashMap::new(), &mascotes, 50, &mapa);
    }
    assert_eq!(ai.get_highest_threat_target(), Some(2));
}

/// B135 — `IS_HUMANSIDE` é só jogador (`common/types.h:321`): o mascote de ar bate **inteiro**
/// no monstro de chão (no B128 levava o corte de 0,5 do ar para o chão).
#[test]
fn o_mascote_de_ar_bate_inteiro_no_monstro_de_chao() {
    use pw_gs::combat::{self, CombatEngine, Rolagens};
    let alvo = monstro(Vector3::new(0.0, 0.0, 0.0));
    let certo = Rolagens { acerto: 0.0, critico: 99 };
    let dano = |habitat| {
        let mut pet = MonsterEntity::placeholder(2, 10521, Vector3::new(0.0, 5.0, 0.0), 0);
        pet.habitat = habitat;
        let mut g = CombatEngine::golpe_de_monstro(&pet);
        g.atacante_e_jogador_ou_pet = true;
        g.dano_fisico = 1000;
        combat::resolver(&g, &CombatEngine::defesa_do_monstro(&alvo), 1.0, false, certo).dano()
    };
    assert_eq!(dano(pw_gs::ai::Habitat::Ar), dano(pw_gs::ai::Habitat::Chao));
}


// ---- B136: rota de patrulha, subordinado que segue o líder, ódio do chefe ----

fn rota_reta(tipo: i32, corre: bool) -> pw_gs::ai::Rota {
    let pontos = (0..4).map(|i| Vector3::new(i as f32 * 20.0, 0.0, 0.0)).collect();
    pw_gs::ai::Rota::nova(std::sync::Arc::new(pontos), tipo, corre).unwrap()
}

/// `base_patrol_agent::GetNextWayPoint`: 0 para no fim, 1 vai e volta, 2 recomeça.
#[test]
fn a_rota_para_vai_e_volta_ou_recomeca() {
    let xs = |tipo: i32| {
        let mut r = rota_reta(tipo, false);
        (0..7).map(|_| r.proximo().map(|p| p.x as i32)).collect::<Vec<_>>()
    };
    assert_eq!(xs(0), vec![Some(0), Some(20), Some(40), Some(60), None, None, None]);
    assert_eq!(xs(1), vec![Some(0), Some(20), Some(40), Some(60), Some(40), Some(20), Some(0)]);
    assert_eq!(xs(2), vec![Some(0), Some(20), Some(40), Some(60), Some(0), Some(20), Some(40)]);
    assert!(pw_gs::ai::Rota::nova(std::sync::Arc::new(vec![Vector3::new(0.0, 0.0, 0.0)]), 2, false).is_none());
}

/// Com rota, o monstro sem combate anda de ponto em ponto, um passo por segundo, **andando**
/// (sem o `iSpeedFlag`) — o Carniçal Sanguinário parado do relato (`ai_patrol_task`).
#[test]
fn o_monstro_com_rota_anda_ponto_a_ponto() {
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut ai = MonsterAi::new();
    ai.rota = Some(rota_reta(2, false));
    // Alguém a 50 m: o monstro não fica no `_idle_mode`, e não é agressivo.
    let p = com_alvo(50.0);
    let (mut max_x, mut andou_a_pe, mut correu) = (0.0f32, 0, 0);
    for _ in 0..(40_000 / 50) {
        if let Some(AcaoDoMonstro::Andou { destino, modo, tempo_ms, .. }) = ai.tick(&mut m, &p, 50, &sem_mapa) {
            assert_eq!(tempo_ms, 1000, "a patrulha anda um passo por segundo");
            max_x = max_x.max(destino.x);
            if modo == pw_gs::ai::MODO_ANDAR {
                andou_a_pe += 1;
            } else {
                correu += 1;
            }
        }
    }
    eprintln!("x máximo {max_x}, {andou_a_pe} passos andando, {correu} correndo");
    assert!(max_x > 45.0, "o monstro não seguiu a rota: x máximo {max_x}");
    assert!(andou_a_pe > 0 && correu == 0);
}

/// Sem ninguém por perto (`_idle_mode`), a patrulha não anda.
#[test]
fn sem_ninguem_por_perto_a_rota_espera() {
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut ai = MonsterAi::new();
    ai.rota = Some(rota_reta(2, false));
    let ninguem = HashMap::new();
    for _ in 0..(60_000 / 50) {
        ai.tick(&mut m, &ninguem, 50, &sem_mapa);
    }
    assert!(m.position.x < 1.0, "andou sem ninguém por perto: {:?}", m.position);
}

/// `ai_follow_master`: a 15 m do líder (entre 8 e 20), o subordinado corre até ficar a menos
/// de 7 m; perto dele, passeia em volta sem se afastar.
#[test]
fn o_subordinado_segue_o_lider() {
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut ai = MonsterAi::new();
    ai.lider = Some(77);
    let lider = Vector3::new(15.0, 0.0, 0.0);
    let p = com_alvo(60.0);
    let mut correu = false;
    for _ in 0..(8_000 / 50) {
        ai.lider_em = Some(lider);
        if let Some(AcaoDoMonstro::Andou { modo, .. }) = ai.tick(&mut m, &p, 50, &sem_mapa) {
            correu |= modo == pw_gs::ai::MODO_CORRER;
        }
    }
    let d = m.position.distance(&lider);
    assert!(correu, "o subordinado não correu atrás do líder");
    assert!(d < 8.0, "ficou a {d} m do líder");
    let mut mais_longe = 0.0f32;
    for _ in 0..(30_000 / 50) {
        ai.lider_em = Some(lider);
        ai.tick(&mut m, &p, 50, &sem_mapa);
        mais_longe = mais_longe.max(m.position.distance(&lider));
    }
    assert!(mais_longe < 15.0, "passeando, afastou-se {mais_longe} m do líder");
}

/// A 20 m ou mais do líder (`MAX_MASTER_MINOR_RANGE`), o subordinado vai de uma vez para até
/// 7 m dele: `ReturnHome(info.pos, 7)`, uma parada com `MOVE_MODE_RETURN`.
#[test]
fn longe_do_lider_o_subordinado_vai_de_uma_vez() {
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut ai = MonsterAi::new();
    ai.lider = Some(77);
    let lider = Vector3::new(30.0, 0.0, 0.0);
    let p = com_alvo(60.0);
    let mut volta = None;
    for _ in 0..(3_000 / 50) {
        ai.lider_em = Some(lider);
        if let Some(AcaoDoMonstro::Parou { posicao, modo, .. }) = ai.tick(&mut m, &p, 50, &sem_mapa) {
            if modo == pw_gs::ai::MODO_VOLTAR {
                volta = Some(posicao);
                break;
            }
        }
    }
    let v = volta.expect("não voltou para perto do líder");
    assert!((v.x - lider.x).abs() <= 7.0 && (v.z - lider.z).abs() <= 7.0, "{v:?}");
}

/// O chefe (`group_boss_policy`) avisa o primeiro da lista quando ele muda, uma vez; o
/// subordinado (`aggro_minor_policy`) troca a lista por ele.
#[test]
fn o_chefe_repassa_o_odio_ao_subordinado() {
    let mut m = monstro(Vector3::new(0.0, 0.0, 0.0));
    let mut chefe = MonsterAi::new();
    chefe.chefe = true;
    chefe.add_threat(1, 10);
    let p = com_alvo(3.0);
    for _ in 0..(1_100 / 50) {
        chefe.tick(&mut m, &p, 50, &sem_mapa);
    }
    assert_eq!(chefe.odio_a_repassar.take(), Some((1, 10)));
    for _ in 0..(2_000 / 50) {
        chefe.tick(&mut m, &p, 50, &sem_mapa);
    }
    assert_eq!(chefe.odio_a_repassar, None, "o mesmo inimigo não é repassado de novo");

    let mut sub = MonsterAi::new();
    sub.add_threat(9, 50);
    sub.receber_odio_do_chefe(1, 10);
    assert_eq!(sub.get_highest_threat_target(), Some(1));
    assert_eq!(sub.aggro_table.len(), 1);
}
