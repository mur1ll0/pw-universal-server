//! O `aipolicy.data` dos dois realms compilado e rodando (B127).
//!
//! As políticas usadas por monstros viram gatilhos ([`pw_gs::politica`]); no 1.2.6 todas as
//! condições e operações que os monstros usam têm porte. Uma política real do 1.2.6, montada
//! num monstro, faz ele conjurar a habilidade dela no combate. Sem a pasta do realm o teste
//! avisa e sai.

use pw_core::Vector3;
use pw_data_loader::GameDataManager;
use pw_gs::ai::{AcaoDoMonstro, MonsterAi};
use pw_gs::entity::{MonsterEntity, PlayerEntity};
use pw_gs::politica::{Acao, Condicao, Disparo, PoliticaDeIa};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

fn realm(nome: &str) -> Option<GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{nome}/config"));
    if !dir.join("aipolicy.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    Some(d)
}

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
        proficiencias: Default::default(),
        crc_aparencia: 0,
        pontos_de_atributo: 0,
        vagas_na_jaula: 1,
        reputacao: 0,
        combate_s: 0,
        contador_hp: 0,
        contador_mp: 0,
        recargas: std::collections::HashMap::new(),
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

fn cond_sem_porte(c: &Condicao) -> bool {
    match c {
        Condicao::SemPorte(_) => true,
        Condicao::E(a, b) | Condicao::Ou(a, b) => cond_sem_porte(a) || cond_sem_porte(b),
        Condicao::Nao(a) => cond_sem_porte(a),
        _ => false,
    }
}

/// Compila as políticas que algum monstro usa e conta o que ficou sem porte.
fn sem_porte(d: &GameDataManager) -> (usize, BTreeMap<&'static str, usize>, usize) {
    let mut ids: Vec<u32> = d.monstros.templates.values().map(|t| t.politica_de_ia).filter(|p| *p != 0).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut ops = BTreeMap::new();
    let mut conds = 0;
    for id in &ids {
        let p = PoliticaDeIa::compilar(d.aipolicy.get_policy(*id).expect("política citada existe"));
        for g in &p.gatilhos {
            conds += cond_sem_porte(&g.condicao) as usize;
            for o in &g.operacoes {
                if let Acao::SemPorte(n) = o.acao {
                    *ops.entry(n).or_default() += 1;
                }
            }
        }
    }
    (ids.len(), ops, conds)
}

#[test]
fn no_126_toda_condicao_e_operacao_usada_tem_porte() {
    let Some(d) = realm("realm_126") else { return };
    let (politicas, ops, conds) = sem_porte(&d);
    eprintln!("1.2.6: {politicas} políticas; sem porte: {ops:?}, {conds} condições");
    assert!(politicas > 0);
    assert!(ops.is_empty(), "operações sem porte no 1.2.6: {ops:?}");
    assert_eq!(conds, 0, "condições sem porte no 1.2.6");
}

#[test]
fn no_155_as_politicas_compilam_e_o_que_falta_e_conhecido() {
    let Some(d) = realm("realm_155") else { return };
    let (politicas, ops, conds) = sem_porte(&d);
    eprintln!("1.5.5: {politicas} políticas; sem porte: {ops:?}, {conds} condições");
    assert!(politicas > 0);
    // O que o 1.5.5 usa e ainda não tem porte: invocações, caminhos, histórico, ações,
    // missões, PvP de facção e contagem de jogadores salva em variável.
    let conhecidas = [
        "InvocarMonstro",
        "InvocarNpc",
        "InvocarMina",
        "AndarPorCaminho",
        "TocarAcao",
        "Historico",
        "PontosPvpDeFaccao",
        "EntregarMissao",
        "ContarJogadores",
        "LimparMissaoDeTorre",
        "RodarTrigger sem o gatilho",
    ];
    for n in ops.keys() {
        assert!(conhecidas.contains(n), "operação sem porte inesperada no 1.5.5: {n}");
    }
}

/// Uma política real do 1.2.6 que, ao começar o combate, cria um timer e, no vencimento,
/// usa uma habilidade: montada num monstro com a IA inteira, o monstro conjura aquela
/// habilidade no combate.
#[test]
fn uma_politica_do_126_faz_o_monstro_conjurar_no_combate() {
    let Some(d) = realm("realm_126") else { return };
    // Um monstro com política cuja habilidade está no catálogo, disparada por timer.
    let mut templates: Vec<_> = d.monstros.templates.values().filter(|t| t.politica_de_ia != 0).collect();
    templates.sort_by_key(|t| t.id);
    let escolhido = templates.into_iter().find_map(|t| {
        let p = PoliticaDeIa::compilar(d.aipolicy.get_policy(t.politica_de_ia)?);
        // Timers que o começo do combate cria com período curto.
        let curtos: Vec<u32> = p
            .gatilhos
            .iter()
            .filter(|g| g.disparo == Disparo::ComecoDeCombate)
            .flat_map(|g| g.operacoes.iter())
            .filter_map(|o| match o.acao {
                Acao::CriarTimer { id, periodo, .. } if (1..=60).contains(&periodo) => Some(id),
                _ => None,
            })
            .collect();
        let habilidade = p.gatilhos.iter().filter(|g| matches!(g.condicao, Condicao::Timer(id) if curtos.contains(&id))).find_map(|g| {
            g.operacoes.iter().find_map(|o| match o.acao {
                Acao::Habilidade { id, nivel } if d.habilidades.get(id as u32).is_some() => Some((id, nivel)),
                _ => None,
            })
        });
        habilidade.map(|h| (t.id, t.politica_de_ia, p, h))
    });
    let Some((tid, pid, politica, (skill, nivel))) = escolhido else {
        panic!("nenhuma política do 1.2.6 com timer de combate e habilidade");
    };
    eprintln!("monstro {tid}, política {pid}, habilidade {skill} nível {nivel}");

    let modelo = d.monstros.get(tid).unwrap();
    let mut m = MonsterEntity::do_template(-2_000_000_001, modelo, Vector3::new(0.0, 0.0, 0.0), 30_000);
    let mut perfil = pw_gs::world::perfil_de_combate(&d, tid, m.attack_range).unwrap();
    for (id, n) in politica.habilidades_citadas() {
        if let Some(h) = pw_gs::world::habilidade_de_monstro(&d, id, n, m.attack_range) {
            perfil.habilidades_da_politica.insert((id, n), h);
        }
    }
    // Sem os eventos de vida (quem tem política não os usa) e com a estratégia inerte, para a
    // única fonte de conjuração ser a política.
    perfil.estrategia = pw_gs::ai::Estrategia::Inerte;
    let mut ai = MonsterAi::com_perfil(Some(perfil));
    let estado = politica.novo_estado(modelo.variaveis_locais);
    ai.politica = Some((Arc::new(politica), estado));

    let mut alvo = jogador(Vector3::new(1.0, 0.0, 0.0));
    alvo.hp = 100_000;
    alvo.max_hp = 100_000;
    let players: HashMap<i64, PlayerEntity> = HashMap::from([(1, alvo)]);
    ai.add_threat(1, 100);
    let chao = |_x: f32, _z: f32| Some(0.0);
    let mut conjurou = None;
    for _ in 0..(120_000 / 50) {
        if let Some(AcaoDoMonstro::Conjurou { habilidade, .. }) = ai.tick(&mut m, &players, 50, &chao) {
            conjurou = Some(habilidade.id);
            break;
        }
    }
    assert_eq!(conjurou, Some(skill), "a política não fez o monstro conjurar a habilidade do timer em 2 minutos");
    eprintln!("conjurou {:?}", conjurou);
}
