//! B344 — o duelo no mundo (`player_duel`, `gs/playerduel.cpp`): convite, aceite, os 3 s de preparo, a luta e a
//! "morte" pelo oponente, que deixa 1 de vida e encerra o duelo com a derrota.

use pw_core::Vector3;
use pw_data_loader::GameDataManager;
use pw_gs::duelo::Aviso;
use pw_gs::entity::PlayerEntity;
use pw_gs::world::EventoDoMundo;
use pw_gs::WorldInstance;
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;

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
        passivas_comuns: Default::default(),
        base_para_mascote: Default::default(),
        amuleto_de_dano: None,
        operacao_de_pet: 0,
        modo_roupa: false,
        sec_level: 0,
        conta_id: 0,
        revisao_gm: 0,
        habilidades: Default::default(),
        proficiencias: Default::default(),
        crc_aparencia: 0,
        pontos_de_atributo: 0,
        vagas_na_jaula: 1,
        lideranca: 0,
        conjuge: 0,
        reputacao: 0,
        combate_s: 0,
        contador_hp: 0,
        contador_mp: 0,
        recargas: std::collections::HashMap::new(),
        bilhete_de_rosto: None,
        pecas: [None; pw_gs::entity::PECAS_VESTIDAS],
        equip_visivel: None,
        voo_gasta_mana: None,
        direcao: 0,
        crc_equipamento: 0,
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
        ultima_coleta: None,
        producao: 0,
        armazem_aberto: false,
        pvp_ligado: false,
        pvp_espera_s: 0,
        forcar_ataque: false,
        produzindo: None,
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

#[tokio::test]
async fn convite_aceite_luta_e_derrota_com_1_de_vida() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else { return };
    let cfg = StorageConfig { database_url: url, max_connections: 2, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("banco");
    let mut mundo = WorldInstance::new(1, Arc::new(GameDataManager::new()), CharacterRepository::new(pool));
    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    mundo.definir_canal_de_eventos(tx);
    let mut a = jogador(Vector3::new(10.0, 0.0, 10.0));
    a.role_id = 1;
    let mut b = jogador(Vector3::new(14.0, 0.0, 10.0));
    b.role_id = 2;
    mundo.players.insert(1, a);
    mundo.players.insert(2, b);

    mundo.pedir_duelo(1, 2);
    mundo.responder_duelo(2, 1, 0);
    assert!(!mundo.em_duelo_entre(1, 2), "ainda preparando");
    for _ in 0..3 {
        mundo.tick(1000).await;
    }
    assert!(mundo.em_duelo_entre(1, 2) && mundo.em_duelo_entre(2, 1), "3 s depois, lutando");

    // O golpe do oponente que zeraria a vida: fica com 1, e o duelo acaba com a derrota.
    mundo.aplicar_dano_no_jogador(2, 1, 5_000);
    assert_eq!(mundo.players[&2].hp, 1);
    assert!(!mundo.em_duelo_entre(1, 2) && !mundo.em_duelo_entre(2, 1));
    assert!(mundo.em_tregua_de_duelo(2, 1), "5 s sem dano do ex-oponente");

    let mut avisos = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let EventoDoMundo::Duelo(a) = ev {
            avisos.push(a);
        }
        assert!(!matches!(ev, EventoDoMundo::JogadorMorreu { .. }), "no duelo ninguém morre");
    }
    for esperado in [
        Aviso::PedidoRecebido { para: 2, de: 1 },
        Aviso::Preparar { para: 1, oponente: 2, atraso: 3 },
        Aviso::Comecou { quem: 1, oponente: 2 },
        Aviso::Comecou { quem: 2, oponente: 1 },
        Aviso::Resultado { quem: 2, oponente: 1, perdeu: true },
        Aviso::Parou { quem: 1 },
        Aviso::Parou { quem: 2 },
    ] {
        assert!(avisos.contains(&esperado), "faltou {esperado:?} em {avisos:?}");
    }

    // Fora do duelo, um golpe mortal mata de verdade.
    for _ in 0..6 {
        mundo.tick(1000).await;
    }
    mundo.aplicar_dano_no_jogador(2, 1, 5_000);
    assert_eq!(mundo.players[&2].hp, 0);
}

#[tokio::test]
async fn longe_demais_o_pedido_cai_pelo_prazo() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else { return };
    let cfg = StorageConfig { database_url: url, max_connections: 2, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("banco");
    let mut mundo = WorldInstance::new(1, Arc::new(GameDataManager::new()), CharacterRepository::new(pool));
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    mundo.definir_canal_de_eventos(tx);
    let mut a = jogador(Vector3::new(0.0, 0.0, 0.0));
    a.role_id = 1;
    let mut b = jogador(Vector3::new(21.0, 0.0, 0.0));
    b.role_id = 2;
    mundo.players.insert(1, a);
    mundo.players.insert(2, b);
    mundo.pedir_duelo(1, 2);
    // `GM_MSG_PLAYER_DUEL_REQUEST` a mais de 20 m volta como `DUEL_REPLY_OUT_OF_RANGE`, mas essa resposta também
    // passa pela conferência de 20 m e vira `GM_MSG_PLAYER_DUEL_CANCEL` (`player.cpp:1943-1968`): quem pediu só
    // fica sabendo pelo prazo do convite (30 s, `DUEL_REPLY_TIMEOUT`).
    let mut avisos = Vec::new();
    for _ in 0..30 {
        mundo.tick(1000).await;
    }
    while let Ok(ev) = rx.try_recv() {
        if let EventoDoMundo::Duelo(a) = ev {
            avisos.push(a);
        }
    }
    assert_eq!(avisos, vec![Aviso::PedidoRecusado { para: 1, quem: 2, motivo: 1 }]);
}
