//! Um subcomando sai do link, atravessa o barramento e muda o mundo.
//!
//! Os outros testes cobrem pedaços: `comandos_contra_o_ir` prova que os deslocamentos
//! estão certos, `pw-bus` prova que o quadro atravessa o TCP, `pw-link/uplink_*` prova o
//! roteamento por jogador. Falta o que só aparece quando tudo está ligado: **o comando
//! chega e o mundo muda**.
//!
//! Este teste monta o `BusServer` de verdade sobre um `BusListener` de verdade, com um
//! `WorldInstance` de verdade, e fala com ele como o `pw-link` fala. Nenhum dublê.
//!
//! # Por que precisa de banco
//!
//! O `WorldInstance` carrega um `CharacterRepository`, que exige um pool de conexões — e
//! não porque o movimento grave alguma coisa (ele não grava; quem grava é o autosave de
//! 60s, e essa é justamente a mudança em relação ao `gateway.rs`, que fazia um `UPDATE`
//! por pacote). É só a construção que pede.
//!
//! Sem `TEST_DATABASE_URL` o teste passa sem verificar nada e diz isso na saída. Como
//! rodar está no cabeçalho de `pw-storage/tests/autorizacao_de_personagem.rs`.

use pw_protocol::GameVersion;
use pw_bus::{BusClient, BusListener, BusMessage};
use pw_core::{CharacterClass, Gender, Race, Vector3};
use pw_data_loader::GameDataManager;
use pw_gs::comandos::ids;
use pw_gs::ai::MonsterAi;
use pw_gs::entity::{MonsterEntity, PlayerEntity};
use pw_gs::{BusServer, WorldInstance};
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const LOCALSID: u32 = 0xC0FF_EE01;
/// A missão ativa do personagem de teste, para conferir a notificação de abate.
const MISSAO: u32 = 4242;
const MONSTRO: i64 = 900_001;
/// HP deliberadamente diferente de 1000: era o valor fixo que o `gateway.rs` mandava, e
/// um teste com 1000 passaria mesmo se nada tivesse mudado de lado.
const MONSTRO_HP: i64 = 137;
const MONSTRO_HP_MAX: i64 = 480;

/// Um monstro com HP conhecido, para conferir que o cliente recebe o valor real.
fn monstro() -> MonsterEntity {
    MonsterEntity {
        id: MONSTRO,
        template_id: 1001,
        name: "Alvo".to_string(),
        level: 1,
        hp: MONSTRO_HP,
        max_hp: MONSTRO_HP_MAX,
        mp: 0,
        max_mp: 0,
        def_phys: 0,
        def_magic: 0,
        attack_min: 1,
        attack_max: 2,
        attack_range: 2.0,
        exp: 1,
        sp: 1,
        aipolicy_id: 0,
        drop_table_id: 0,
        position: Vector3::new(5.0, 0.0, 5.0),
        spawn_center: Vector3::new(5.0, 0.0, 5.0),
        move_speed: 1.0,
        is_dead: false,
        respawn_timer_ms: 0,
        respawn_delay_ms: 1000,
        target_id: None,
        buffs: Vec::new(),
    }
}

/// Monta um jogador no mundo, com o mínimo que o teste precisa.
fn jogador(role_id: i32, pos: Vector3) -> PlayerEntity {
    PlayerEntity {
        role_id,
        name: "Testador".to_string(),
        race: Race::Human,
        cls: CharacterClass::Blademaster,
        gender: Gender::Male,
        level: 1,
        cultivation: 0,
        hp: 100,
        max_hp: 100,
        mp: 50,
        max_mp: 50,
        exp: 0,
        sp: 0,
        money: 0,
        strength: 5,
        agility: 5,
        vitality: 5,
        energy: 5,
        def_phys: 10,
        def_metal: 10,
        def_wood: 10,
        def_water: 10,
        def_fire: 10,
        def_earth: 10,
        attack_min: 10,
        attack_max: 15,
        magic_attack_min: 0,
        magic_attack_max: 0,
        attack_speed: 1.0,
        move_speed: 4.8,
        crit_rate: 0.0,
        position: pos,
        target_id: None,
        buffs: Vec::new(),
    }
}

/// Abre um pool **por teste**, pequeno.
///
/// A tentação é compartilhar um `static` entre todos os testes do arquivo, e ela é uma
/// armadilha: cada `#[tokio::test]` cria o **próprio runtime**, e uma conexão sqlx só vive
/// enquanto o runtime que a abriu existir. Um pool `static` guarda conexões do runtime do
/// primeiro teste, entrega-as ao segundo, e o segundo trava até o tempo esgotar — com um
/// `PoolTimedOut` que parece problema de servidor e não é.
///
/// Pequeno porque são muitos testes em paralelo, e cada um só faz algumas consultas.
async fn pool_do_teste(url: String) -> PostgresPool {
    let cfg = StorageConfig {
        database_url: url,
        max_connections: 3,
        min_connections: 1,
        ..Default::default()
    };
    PostgresPool::new(&cfg).await.expect("conexão com o banco")
}

/// Um personagem de verdade no banco, com uma missão ativa.
///
/// O `role_id` não é inventado: a notificação de abate consulta `character_quests`, que
/// tem chave estrangeira para `characters`. Com um id fictício a consulta voltaria vazia
/// e o teste do abate passaria sem testar nada — que era o caso antes.
async fn personagem_com_missao(pool: &PostgresPool) -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let m = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;

    let realm = format!("t_gs_{m}");
    sqlx::query(
        "INSERT INTO realms (id, name, version, host, port, max_players, config)
         VALUES ($1, 'Teste GS', '1.2.6', '127.0.0.1', 29000, 10, '{}'::jsonb)",
    )
    .bind(&realm)
    .execute(pool.get_ref())
    .await
    .expect("criar realm");

    let conta: i32 = sqlx::query_scalar(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id",
    )
    .bind(format!("gs_{m}"))
    .fetch_one(pool.get_ref())
    .await
    .expect("criar conta");

    let repo = CharacterRepository::new(pool.clone());
    let role_id = repo
        .create_character(
            conta,
            &realm,
            &format!("Caca{m}"),
            Race::Human,
            CharacterClass::Blademaster,
            Gender::Male,
            Vec::new(),
        )
        .await
        .expect("criar personagem");

    repo.quest_repo()
        .save_quest(role_id, MISSAO, pw_core::QuestStatus::Active, &[0], None)
        .await
        .expect("criar missão ativa");

    role_id
}

/// Monta mundo + servidor de barramento, ou `None` sem banco configurado.
async fn montar() -> Option<(Arc<RwLock<WorldInstance>>, std::net::SocketAddr, i32)> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!(
                "AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada."
            );
            return None;
        }
    };
    // Pool pequeno de propósito: cada teste abre o seu, e o padrão (50) multiplicado
    // pelos testes em paralelo estoura o `max_connections` do servidor.
    let pool = pool_do_teste(url).await;
    let roleid = personagem_com_missao(&pool).await;

    let mut mundo = WorldInstance::new(
        1,
        Arc::new(GameDataManager::new()),
        CharacterRepository::new(pool),
    );
    mundo.add_player(jogador(roleid, Vector3::new(0.0, 0.0, 0.0)));
    mundo
        .monsters
        .insert(MONSTRO, (monstro(), MonsterAi::new()));
    let mundo = Arc::new(RwLock::new(mundo));

    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();
    // O realm de teste é 1.2.6 — é o que o `personagem_com_missao` cria — e desde o
    // item 56 isso muda bytes: 32 comandos têm layout próprio naquela versão.
    let servidor = Arc::new(BusServer::new(Arc::clone(&mundo), GameVersion::V1_2_6));
    // Sem isto, o que o tick decide não chega ao cliente — que era o estado anterior.
    servidor.ligar_eventos_do_mundo().await;
    tokio::spawn(Arc::clone(&servidor).executar(escuta));

    Some((mundo, addr, roleid))
}

macro_rules! cenario {
    () => {
        match montar().await {
            Some(c) => c,
            None => return,
        }
    };
}

/// Monta o payload de um subcomando: cabeçalho little-endian + corpo.
fn subcomando(id: u16, corpo: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(corpo);
    v
}

fn vec3(x: f32, y: f32, z: f32) -> Vec<u8> {
    let mut b = Vec::with_capacity(12);
    for c in [x, y, z] {
        b.extend_from_slice(&c.to_le_bytes());
    }
    b
}

/// Espera até `cond` valer, ou desiste. O mundo é atualizado por outra tarefa.
async fn ate<F>(mut cond: F) -> bool
where
    F: FnMut() -> bool,
{
    for _ in 0..200 {
        if cond() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test]
async fn um_player_move_do_cliente_move_o_jogador_no_mundo() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = BusClient::conectar(addr).await.unwrap();

    link.enviar(BusMessage::EnterWorld {
        roleid,
        provider_link_id: 1,
        locktime: 0,
        timeout: 60,
        settime: 0,
        localsid: LOCALSID,
    })
    .await
    .unwrap();

    // Um `PLAYER_MOVE` completo, com `cur_pos` e `next_pos` diferentes — é o que pega
    // quem grava a posição errada das duas.
    let mut corpo = vec3(10.0, 20.0, 30.0);
    corpo.extend_from_slice(&vec3(99.0, 99.0, 99.0)); // next_pos: para onde vai
    corpo.extend_from_slice(&100u16.to_le_bytes()); // use_time
    corpo.extend_from_slice(&48u16.to_le_bytes()); // speed
    corpo.push(0); // move_mode
    corpo.extend_from_slice(&7u16.to_le_bytes()); // cmd_seq

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::PLAYER_MOVE, &corpo),
    })
    .await
    .unwrap();

    let chegou = ate(|| {
        mundo
            .try_read()
            .map(|m| {
                m.players
                    .get(&(roleid as i64))
                    .map(|p| p.position.x == 10.0)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    })
    .await;
    assert!(chegou, "o mundo não moveu o jogador");

    let m = mundo.read().await;
    let p = m.players.get(&(roleid as i64)).unwrap();
    assert_eq!(
        (p.position.x, p.position.y, p.position.z),
        (10.0, 20.0, 30.0),
        "a posição gravada é a `next_pos` — o personagem ficaria um passo à frente"
    );

    // E a grade espacial acompanhou. Se só a entidade tivesse mudado, o jogador andaria
    // na tela e continuaria sendo visto — e agredido — no lugar antigo.
    let perto = m.grid.get_players_in_range(&Vector3::new(10.0, 20.0, 30.0), 1.0);
    assert!(
        perto.contains(&(roleid as i64)),
        "a grade continuou com a posição velha"
    );
}

#[tokio::test]
async fn um_logout_tira_o_jogador_do_mundo_e_avisa_o_link() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = BusClient::conectar(addr).await.unwrap();

    link.enviar(BusMessage::EnterWorld {
        roleid,
        provider_link_id: 1,
        locktime: 0,
        timeout: 60,
        settime: 0,
        localsid: LOCALSID,
    })
    .await
    .unwrap();

    // `logout_type = 1` é `_PLAYER_LOGOUT_HALF`: voltar à seleção de personagens.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::LOGOUT, &1i32.to_le_bytes()),
    })
    .await
    .unwrap();

    // O mundo responde pelo barramento, e não ao cliente: quem fala com o cliente é o
    // link. É esta mensagem que ele traduz no pacote GNET de saída.
    let resposta = tokio::time::timeout(Duration::from_secs(5), link.receber())
        .await
        .expect("o mundo não avisou a saída em 5s")
        .unwrap()
        .expect("conexão fechou sem resposta");

    match resposta {
        BusMessage::PlayerLogout {
            roleid: quem,
            localsid,
            result,
            ..
        } => {
            assert_eq!(quem, roleid);
            assert_eq!(
                localsid, LOCALSID,
                "o `localsid` veio do EnterWorld, não do pacote de saída"
            );
            assert_eq!(result, 1, "seleção de personagem devia dar result 1");
        }
        outra => panic!("o mundo respondeu {outra:?} em vez de PlayerLogout"),
    }

    // E o jogador saiu da simulação — senão o personagem fica "preso" no mundo.
    let saiu = ate(|| {
        mundo
            .try_read()
            .map(|m| !m.players.contains_key(&(roleid as i64)))
            .unwrap_or(false)
    })
    .await;
    assert!(saiu, "o jogador continuou no mundo depois do logout");
}

#[tokio::test]
async fn selecionar_alvo_devolve_o_hp_de_verdade_do_monstro() {
    // No `gateway.rs` este comando respondia HP **1000/1000 fixo**, porque o daemon de
    // link não sabe o estado das criaturas. É a razão de o tratamento pertencer ao mundo,
    // e é o que este teste cobra.
    let (_mundo, addr, roleid) = cenario!();
    let mut link = BusClient::conectar(addr).await.unwrap();

    link.enviar(BusMessage::EnterWorld {
        roleid,
        provider_link_id: 1,
        locktime: 0,
        timeout: 60,
        settime: 0,
        localsid: LOCALSID,
    })
    .await
    .unwrap();

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();

    // Duas respostas: a confirmação da seleção e a barra de vida.
    let mut vistos: Vec<Vec<u8>> = Vec::new();
    for _ in 0..2 {
        let m = tokio::time::timeout(Duration::from_secs(5), link.receber())
            .await
            .expect("o mundo não respondeu à seleção")
            .unwrap()
            .expect("conexão fechou");
        match m {
            BusMessage::GameToClient { data, .. } => vistos.push(data),
            outra => panic!("chegou {outra:?}"),
        }
    }

    let cmd = |v: &Vec<u8>| u16::from_le_bytes([v[0], v[1]]);
    let sel = vistos.iter().find(|v| cmd(v) == 52).expect("sem SELECT_TARGET (52)");
    assert_eq!(
        i32::from_le_bytes([sel[2], sel[3], sel[4], sel[5]]),
        MONSTRO as i32
    );

    let info = vistos.iter().find(|v| cmd(v) == 33).expect("sem NPC_INFO_00 (33)");
    let campo = |i: usize| i32::from_le_bytes([info[i], info[i + 1], info[i + 2], info[i + 3]]);
    assert_eq!(campo(2), MONSTRO as i32, "idNPC");
    assert_eq!(
        campo(6),
        MONSTRO_HP as i32,
        "o HP mandado não é o do monstro — voltou a ser valor fixo?"
    );
    assert_eq!(campo(10), MONSTRO_HP_MAX as i32, "iMaxHP");
}

#[tokio::test]
async fn desmarcar_o_alvo_manda_unselect() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = BusClient::conectar(addr).await.unwrap();

    link.enviar(BusMessage::EnterWorld {
        roleid,
        provider_link_id: 1,
        locktime: 0,
        timeout: 60,
        settime: 0,
        localsid: LOCALSID,
    })
    .await
    .unwrap();

    // `0` é como o cliente desmarca.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &0i32.to_le_bytes()),
    })
    .await
    .unwrap();

    let m = tokio::time::timeout(Duration::from_secs(5), link.receber())
        .await
        .expect("o mundo não respondeu")
        .unwrap()
        .unwrap();
    match m {
        BusMessage::GameToClient { data, .. } => {
            assert_eq!(u16::from_le_bytes([data[0], data[1]]), 39, "devia ser UNSELECT (39)");
            assert_eq!(data.len(), 2, "UNSELECT não tem payload");
        }
        outra => panic!("chegou {outra:?}"),
    }

    let m = mundo.read().await;
    assert_eq!(
        m.players.get(&(roleid as i64)).unwrap().target_id,
        None,
        "o mundo continuou com o alvo antigo"
    );
}

/// Manda `EnterWorld` e devolve o link pronto para os comandos seguintes.
async fn entrar(addr: std::net::SocketAddr, roleid: i32) -> pw_bus::transport::BusConnection {
    let mut link = BusClient::conectar(addr).await.unwrap();
    link.enviar(BusMessage::EnterWorld {
        roleid,
        provider_link_id: 1,
        locktime: 0,
        timeout: 60,
        settime: 0,
        localsid: LOCALSID,
    })
    .await
    .unwrap();
    link
}

/// Recebe `n` subcomandos do mundo, ou falha com prazo.
async fn receber(link: &mut pw_bus::transport::BusConnection, n: usize) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for _ in 0..n {
        let m = tokio::time::timeout(Duration::from_secs(5), link.receber())
            .await
            .unwrap_or_else(|_| panic!("o mundo mandou {} de {n} respostas", v.len()))
            .unwrap()
            .expect("conexão fechou");
        match m {
            BusMessage::GameToClient { data, .. } => v.push(data),
            outra => panic!("chegou {outra:?}"),
        }
    }
    v
}

fn cmd_de(v: &[u8]) -> u16 {
    u16::from_le_bytes([v[0], v[1]])
}

fn i32_em(v: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([v[off], v[off + 1], v[off + 2], v[off + 3]])
}

#[tokio::test]
async fn atacar_debita_o_hp_de_verdade_do_monstro() {
    // No `gateway.rs` o dano era 35 fixo e o HP respondido era 965/1000 fixo — o monstro
    // nunca perdia vida de verdade e nunca morria. Aqui o HP tem que cair.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await; // SELECT_TARGET + NPC_INFO_00

    // O ataque **não carrega alvo**: só o `force_attack`. Quem sabe o alvo é o mundo.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::NORMAL_ATTACK, &[0u8]),
    })
    .await
    .unwrap();

    let resp = receber(&mut link, 2).await; // HOST_ATTACKRESULT + NPC_INFO_00
    let golpe = resp.iter().find(|v| cmd_de(v) == 24).expect("sem HOST_ATTACKRESULT (24)");
    let dano = i32_em(golpe, 6);
    assert!(dano > 0, "o golpe não causou dano");
    assert_eq!(i32_em(golpe, 2), MONSTRO as i32, "idTarget");

    let barra = resp.iter().find(|v| cmd_de(v) == 33).expect("sem NPC_INFO_00 (33)");
    let hp_no_fio = i32_em(barra, 6);

    let hp_no_mundo = mundo.read().await.monsters[&MONSTRO].0.hp;
    assert_eq!(
        hp_no_fio as i64, hp_no_mundo,
        "o HP mandado ao cliente não é o do mundo"
    );
    assert_eq!(
        hp_no_mundo,
        MONSTRO_HP - dano as i64,
        "o HP do mundo não caiu exatamente o dano do golpe"
    );
    assert!(
        hp_no_mundo < MONSTRO_HP,
        "o monstro não perdeu vida — voltou a ser resposta fictícia?"
    );
}

#[tokio::test]
async fn o_monstro_morre_e_o_abate_leva_o_template_certo() {
    // O `gateway.rs` notificava abate **a cada golpe**, com a criatura `13641` escrita no
    // código — qualquer missão de caça completava batendo em qualquer coisa. Aqui a
    // notificação só sai na morte, e com o template real.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;

    // Bate até morrer. O dano é aleatório, então o laço tem um teto para não travar.
    let mut morreu = false;
    for _ in 0..500 {
        link.enviar(BusMessage::ClientToGame {
            roleid,
            localsid: LOCALSID,
            data: subcomando(ids::NORMAL_ATTACK, &[0u8]),
        })
        .await
        .unwrap();

        let r = receber(&mut link, 2).await;
        let barra = r.iter().find(|v| cmd_de(v) == 33).expect("sem NPC_INFO_00");
        if i32_em(barra, 6) == 0 {
            morreu = true;
            break;
        }
    }
    assert!(morreu, "o monstro não chegou a zero em 500 golpes");

    // Na morte vêm NPC_DIED (20) e RECEIVE_EXP (36).
    let apos = receber(&mut link, 2).await;
    let obito = apos.iter().find(|v| cmd_de(v) == 20).expect("sem NPC_DIED (20)");
    assert_eq!(i32_em(obito, 2), MONSTRO as i32);
    assert_eq!(i32_em(obito, 6), roleid, "o matador não é o jogador");
    assert!(
        apos.iter().any(|v| cmd_de(v) == 36),
        "sem RECEIVE_EXP (36) na morte"
    );

    // A notificação de abate: `TASK_VAR_DATA` (106) com `reason = 4`, o id da missão e o
    // **template real** da criatura. O `gateway.rs` mandava `13641` fixo, a cada golpe.
    let aviso = receber(&mut link, 1).await;
    let v = &aviso[0];
    assert_eq!(cmd_de(v), 106, "a notificação de abate vai dentro do TASK_VAR_DATA");
    // Corpo do task_var_data: [len:u32][reason:u8][task:u16][monster_id:u32][num:u16]
    let corpo = &v[2..];
    let reason = corpo[4];
    assert_eq!(reason, 4, "reason devia ser TASK_SVR_NOTIFY_MONSTER_KILLED (4)");
    let missao = u16::from_le_bytes([corpo[5], corpo[6]]);
    assert_eq!(missao as u32, MISSAO, "a missão notificada não é a do personagem");
    let template = u32::from_le_bytes([corpo[7], corpo[8], corpo[9], corpo[10]]);
    assert_eq!(
        template, 1001,
        "o abate foi notificado com o template errado — voltou a ser o 13641 fixo?"
    );

    let m = mundo.read().await;
    assert!(m.monsters[&MONSTRO].0.is_dead, "o monstro não ficou morto");
    assert_eq!(m.monsters[&MONSTRO].0.hp, 0);
    // E saiu da grade: continuar lá o deixaria sendo alvo de quem estivesse perto.
    assert!(
        !m.grid
            .get_entities_in_range(&Vector3::new(5.0, 0.0, 5.0), 3.0)
            .contains(&MONSTRO),
        "o monstro morto continuou na grade espacial"
    );
}

#[tokio::test]
async fn atacar_sem_alvo_nao_faz_nada() {
    // Sem `SELECT_TARGET` antes, não há o que atacar — e o servidor não pode inventar um
    // alvo nem responder um golpe no vazio.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::NORMAL_ATTACK, &[0u8]),
    })
    .await
    .unwrap();

    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "o mundo respondeu a um ataque sem alvo");
    assert_eq!(
        mundo.read().await.monsters[&MONSTRO].0.hp,
        MONSTRO_HP,
        "o monstro levou dano sem ter sido selecionado"
    );
}

#[tokio::test]
async fn stop_move_tambem_move_o_jogador_no_mundo() {
    // A ordem dos campos do STOP_MOVE difere da do PLAYER_MOVE (`use_time` é o último).
    // Aqui interessa que a posição chegue ao mundo; o teste de ordem está em
    // `comandos_contra_o_ir.rs`.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let mut corpo = vec3(-3.0, 4.0, -5.0);
    corpo.extend_from_slice(&48u16.to_le_bytes()); // speed
    corpo.push(2); // dir
    corpo.push(0); // move_mode
    corpo.extend_from_slice(&9u16.to_le_bytes()); // cmd_seq
    corpo.extend_from_slice(&30u16.to_le_bytes()); // use_time

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::STOP_MOVE, &corpo),
    })
    .await
    .unwrap();

    let chegou = ate(|| {
        mundo
            .try_read()
            .map(|m| {
                m.players
                    .get(&(roleid as i64))
                    .map(|p| p.position.x == -3.0)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    })
    .await;
    assert!(chegou, "o mundo não registrou a parada");
}

/// Roda o tick do mundo até `cond` valer, ou desiste.
///
/// O tick de produção roda sozinho a 20 TPS; aqui ele é chamado à mão, para o teste não
/// depender de tempo de relógio.
async fn tickar_ate<F>(mundo: &Arc<RwLock<WorldInstance>>, mut cond: F) -> bool
where
    F: FnMut(&WorldInstance) -> bool,
{
    for _ in 0..400 {
        {
            let mut m = mundo.write().await;
            m.tick(50).await;
            if cond(&m) {
                return true;
            }
        }
        tokio::task::yield_now().await;
    }
    false
}

#[tokio::test]
async fn o_monstro_revida_e_o_cliente_fica_sabendo() {
    // Duas coisas que não existiam. Primeira: **nada em produção alimentava a tabela de
    // ameaça** — só um teste de unidade —, então o `MonsterAi` e o
    // `calculate_monster_to_player_damage` eram código morto e o monstro nunca revidava.
    // Segunda: mesmo que revidasse, o dano era aplicado **em silêncio**; o cliente via a
    // vida cheia até morrer do nada.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    // Encosta no monstro: a IA só ataca dentro do alcance.
    {
        let mut m = mundo.write().await;
        m.mover_jogador(roleid, Vector3::new(5.0, 0.0, 5.0));
    }

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;

    // Um golpe do jogador gera ameaça — é o que acorda a IA.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::NORMAL_ATTACK, &[0u8]),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;

    let hp_inicial = mundo.read().await.players[&(roleid as i64)].hp;

    let apanhou = tickar_ate(&mundo, |m| {
        m.players.get(&(roleid as i64)).map(|p| p.hp < hp_inicial).unwrap_or(false)
    })
    .await;
    assert!(
        apanhou,
        "o monstro nunca revidou — a ameaça não está sendo registrada?"
    );

    // E o cliente foi avisado: HOST_ATTACKED (26) mais a barra de vida.
    let avisos = receber(&mut link, 2).await;
    let golpe = avisos
        .iter()
        .find(|v| cmd_de(v) == 26)
        .expect("sem HOST_ATTACKED (26) — o dano chegou em silêncio");
    let dano = i32_em(golpe, 6);
    assert!(dano > 0, "o aviso de dano veio zerado");

    // A vida **do próprio jogador** vai no `SELF_INFO_00` (38), e não no `NPC_INFO_00`
    // (33): o cliente entrega o 33 ao gerenciador de NPCs, que não conhece jogador nenhum
    // (`EC_GameDataPrtc.cpp`). Era o comando errado, e o aviso morria lá.
    assert!(
        !avisos.iter().any(|v| cmd_de(v) == 33),
        "a vida do jogador saiu como NPC_INFO_00 (33) — vai para o gerenciador de NPCs"
    );
    let barra = avisos
        .iter()
        .find(|v| cmd_de(v) == 38)
        .expect("sem SELF_INFO_00 (38) — o jogador não soube quanta vida lhe restou");
    // `cmd_self_info_00`: sLevel(2) State(1) Level2(1) iHP(4) ... depois do cabeçalho.
    assert_eq!(
        i32_em(barra, 2 + 4),
        mundo.read().await.players[&(roleid as i64)].hp,
        "o HP avisado ao cliente não é o do mundo"
    );
}

#[tokio::test]
async fn morrer_avisa_o_cliente_e_reviver_devolve_a_vida() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    // Deixa o jogador a um golpe da morte, encostado no monstro.
    {
        let mut m = mundo.write().await;
        m.mover_jogador(roleid, Vector3::new(5.0, 0.0, 5.0));
        m.players.get_mut(&(roleid as i64)).unwrap().hp = 1;
    }

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::NORMAL_ATTACK, &[0u8]),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;

    let morreu = tickar_ate(&mundo, |m| {
        m.players.get(&(roleid as i64)).map(|p| p.hp == 0).unwrap_or(false)
    })
    .await;
    assert!(morreu, "o jogador não chegou a zero");

    // Chegam o dano, a barra e o HOST_DIED (28).
    let mut viu_morte = false;
    for _ in 0..3 {
        let v = receber(&mut link, 1).await;
        if cmd_de(&v[0]) == 28 {
            viu_morte = true;
            break;
        }
    }
    assert!(viu_morte, "sem HOST_DIED (28) — o jogador morreu em silêncio");

    // Agora o renascimento, que antes não tinha tratamento nenhum: quem zerava a vida
    // ficava preso até reconectar.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::REVIVE_VILLAGE, &0i32.to_le_bytes()),
    })
    .await
    .unwrap();

    let resp = receber(&mut link, 2).await;
    let revive = resp
        .iter()
        .find(|v| cmd_de(v) == 29)
        .expect("sem PLAYER_REVIVE (29)");
    assert_eq!(i32_em(revive, 2), roleid, "idPlayer");

    let m = mundo.read().await;
    let p = &m.players[&(roleid as i64)];
    assert_eq!(p.hp, p.max_hp, "o jogador reviveu sem a vida cheia");
    assert_eq!(p.target_id, None, "o alvo antigo sobreviveu à morte");
}

#[tokio::test]
async fn quem_esta_vivo_nao_revive() {
    // Ressuscitar quem não morreu seria um teleporte grátis para a cidade sempre que o
    // jogador quisesse.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let antes = mundo.read().await.players[&(roleid as i64)].position;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::REVIVE_VILLAGE, &0i32.to_le_bytes()),
    })
    .await
    .unwrap();

    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "o mundo respondeu a um revive de quem está vivo");

    let depois = mundo.read().await.players[&(roleid as i64)].position;
    assert_eq!(
        (antes.x, antes.y, antes.z),
        (depois.x, depois.y, depois.z),
        "o jogador vivo foi teleportado"
    );
}

#[tokio::test]
async fn equipar_pelo_barramento_move_o_item_e_avisa_o_cliente() {
    // O caminho inteiro de um comando de item: chega pelo barramento, mexe no banco, e o
    // cliente recebe o estado novo. E o item continua com os octetos — que é a falha que
    // `pw-storage/tests/itens_sobrevivem.rs` tranca do lado do repositório.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    const OCTETOS: &[u8] = &[0x11, 0x22, 0x33, 0x44];
    let itens = mundo.read().await.char_repo.item_repo().clone();
    itens
        .upsert_item(&pw_core::ItemRecord {
            id: None,
            character_id: roleid,
            container_type: pw_core::ContainerType::Inventory,
            // Slot alto de propósito: o personagem nasce com itens nos primeiros.
            slot: 9,
            item_id: 4123,
            count: 1,
            max_count: 1,
            refine_level: 5,
            sockets_count: 0,
            sockets: vec![],
            durability: 900,
            max_durability: 1000,
            bind_status: 0,
            octets: OCTETOS.to_vec(),
            custom_attributes: serde_json::json!({}),
        })
        .await
        .expect("guardar o item");

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::EQUIP_ITEM, &[9u8, 0u8]),
    })
    .await
    .unwrap();

    // O personagem nasce com uma arma equipada, então a operação é uma **troca**: o item
    // da bolsa vai para o corpo e o que estava no corpo vem para a bolsa. Saem então
    // EQUIP_ITEM (48), dois `item_info` (40) — um por lado — e dois unfreeze (181).
    let resp = receber(&mut link, 5).await;
    assert!(
        resp.iter().any(|v| cmd_de(v) == 48),
        "sem a confirmação EQUIP_ITEM (48)"
    );

    let infos: Vec<&Vec<u8>> = resp.iter().filter(|v| cmd_de(v) == 40).collect();
    assert!(
        infos.iter().any(|v| v[2] == 1),
        "sem `item_info` do corpo — o cliente não saberia o que está equipado"
    );
    assert!(
        infos.iter().any(|v| v[2] == 0),
        "sem `item_info` da bolsa — o item desequipado sumiria da interface"
    );

    // E no banco: saiu da bolsa, entrou no corpo, com os octetos intactos.
    let no_corpo = itens
        .get_item_by_slot(roleid, pw_core::ContainerType::Equipment, 0)
        .await
        .unwrap()
        .expect("o item não foi equipado");
    assert_eq!(no_corpo.item_id, 4123);
    assert_eq!(
        no_corpo.octets, OCTETOS,
        "equipar apagou os octetos do item"
    );
    assert_eq!(no_corpo.refine_level, 5, "o refino se perdeu ao equipar");

    // E a bolsa recebeu o que estava equipado — e não uma cópia do que foi equipado.
    let na_bolsa = itens
        .get_item_by_slot(roleid, pw_core::ContainerType::Inventory, 9)
        .await
        .unwrap();
    assert!(
        na_bolsa.map(|i| i.item_id) != Some(4123),
        "o item foi duplicado: ficou na bolsa e no corpo"
    );
}

#[tokio::test]
async fn trocar_slots_da_bolsa_pelo_barramento_preserva_o_item() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    const OCTETOS: &[u8] = &[0xA1, 0xA2];
    let itens = mundo.read().await.char_repo.item_repo().clone();
    itens
        .upsert_item(&pw_core::ItemRecord {
            id: None,
            character_id: roleid,
            container_type: pw_core::ContainerType::Inventory,
            slot: 11,
            item_id: 999,
            count: 3,
            max_count: 99,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 100,
            max_durability: 100,
            bind_status: 0,
            octets: OCTETOS.to_vec(),
            custom_attributes: serde_json::json!({}),
        })
        .await
        .unwrap();

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::EXG_IVTR_ITEM, &[11u8, 16u8]),
    })
    .await
    .unwrap();

    let resp = receber(&mut link, 3).await; // EXG (44) + dois unfreeze (181)
    assert!(
        resp.iter().any(|v| cmd_de(v) == 44),
        "sem a confirmação EXG_IVTR_ITEM (44)"
    );

    let movido = itens
        .get_item_by_slot(roleid, pw_core::ContainerType::Inventory, 16)
        .await
        .unwrap()
        .expect("o item não foi para o slot 16");
    assert_eq!(movido.count, 3, "a quantidade se perdeu");
    assert_eq!(movido.octets, OCTETOS, "os octetos se perderam na troca");
}

/// O dinheiro do personagem, lido direto do banco.
///
/// Consultar a coluna evita acrescentar um método só-para-teste ao repositório — a API de
/// produção não deve crescer por causa de asserção.
async fn dinheiro(mundo: &Arc<RwLock<WorldInstance>>, roleid: i32) -> i64 {
    let repo = mundo.read().await.char_repo.clone();
    sqlx::query_scalar::<_, i64>("SELECT money FROM characters WHERE id = $1")
        .bind(roleid)
        .fetch_one(repo.pool().get_ref())
        .await
        .expect("ler o dinheiro do personagem")
}

/// Monta o envelope do `SEVNPC_SERVE`: serviço, tamanho e conteúdo.
fn pedido_ao_npc(servico: i32, conteudo: &[u8]) -> Vec<u8> {
    let mut v = servico.to_le_bytes().to_vec();
    v.extend_from_slice(&(conteudo.len() as u32).to_le_bytes());
    v.extend_from_slice(conteudo);
    subcomando(ids::SEVNPC_SERVE, &v)
}

#[tokio::test]
async fn comprar_do_npc_tira_dinheiro_e_da_o_item() {
    // `GP_NPCSEV_SELL` é o **NPC vendendo**, ou seja, o jogador comprando. O `gateway.rs`
    // lia o nome do enum do ponto de vista do jogador e fazia o contrário: apagava um item
    // e pagava por ele.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let repo = mundo.read().await.char_repo.clone();
    let itens = repo.item_repo().clone();
    let _ = repo.add_money(roleid, 10_000).await;
    let antes = dinheiro(&mundo, roleid).await;

    // CONTENT da compra: 28 bytes de cabeçalho, depois `npc_trade_item`.
    let mut c = Vec::new();
    c.extend_from_slice(&[0u8; 24]); // money + as cinco contribuições
    c.extend_from_slice(&1u32.to_le_bytes()); // item_count
    c.extend_from_slice(&4123i32.to_le_bytes()); // tid
    c.extend_from_slice(&20u32.to_le_bytes()); // index (slot de destino)
    c.extend_from_slice(&1u32.to_le_bytes()); // count

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::NPC_VENDE, &c),
    })
    .await
    .unwrap();

    receber(&mut link, 2).await; // item_info + unfreeze

    let comprado = itens
        .get_item_by_slot(roleid, pw_core::ContainerType::Inventory, 20)
        .await
        .unwrap()
        .expect("o item comprado não chegou à bolsa");
    assert_eq!(comprado.item_id, 4123);

    let depois = dinheiro(&mundo, roleid).await;
    assert!(
        depois < antes,
        "comprar **aumentou** o dinheiro do jogador ({antes} → {depois}) — a loja voltou \
         a ficar invertida?"
    );
}

#[tokio::test]
async fn vender_ao_npc_tira_o_item_e_da_dinheiro() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let repo = mundo.read().await.char_repo.clone();
    let itens = repo.item_repo().clone();
    itens
        .upsert_item(&pw_core::ItemRecord {
            id: None,
            character_id: roleid,
            container_type: pw_core::ContainerType::Inventory,
            slot: 21,
            item_id: 555,
            count: 2,
            max_count: 99,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 100,
            max_durability: 100,
            bind_status: 0,
            octets: vec![],
            custom_attributes: serde_json::json!({}),
        })
        .await
        .unwrap();

    let antes = dinheiro(&mundo, roleid).await;

    // CONTENT da venda: 4 bytes de contagem, depois `npc_sell_item` (com `price`).
    let mut c = 1u32.to_le_bytes().to_vec();
    c.extend_from_slice(&555i32.to_le_bytes()); // tid
    c.extend_from_slice(&21u32.to_le_bytes()); // index
    c.extend_from_slice(&2u32.to_le_bytes()); // count
    c.extend_from_slice(&999_999i32.to_le_bytes()); // price que o cliente inventou

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::NPC_COMPRA, &c),
    })
    .await
    .unwrap();

    receber(&mut link, 1).await; // unfreeze

    assert!(
        itens
            .get_item_by_slot(roleid, pw_core::ContainerType::Inventory, 21)
            .await
            .unwrap()
            .is_none(),
        "o item vendido continuou na bolsa"
    );

    let depois = dinheiro(&mundo, roleid).await;
    assert!(depois > antes, "vender não pagou nada ({antes} → {depois})");
    assert!(
        depois - antes < 999_999,
        "o servidor obedeceu ao `price` que o cliente mandou — o jogador escolheria \
         quanto ganha"
    );
}

#[tokio::test]
async fn nao_da_para_vender_um_slot_vazio() {
    // Sem conferir o slot, o jogador ganha dinheiro por vender nada.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let antes = dinheiro(&mundo, roleid).await;

    let mut c = 1u32.to_le_bytes().to_vec();
    c.extend_from_slice(&555i32.to_le_bytes());
    c.extend_from_slice(&40u32.to_le_bytes()); // slot vazio
    c.extend_from_slice(&1u32.to_le_bytes());
    c.extend_from_slice(&100i32.to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::NPC_COMPRA, &c),
    })
    .await
    .unwrap();

    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "o mundo respondeu à venda de um slot vazio");
    assert_eq!(
        dinheiro(&mundo, roleid).await,
        antes,
        "o jogador foi pago por vender nada"
    );
}

#[tokio::test]
async fn aceitar_missao_pelo_npc_grava_no_banco() {
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    const NOVA: u32 = 5150;
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(
            pw_gs::npc::servico::ACEITAR_MISSAO,
            &(NOVA as i32).to_le_bytes(),
        ),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 1).await;
    assert_eq!(cmd_de(&r[0]), 106, "a notificação vai no TASK_VAR_DATA");

    let repo = mundo.read().await.char_repo.clone();
    let missoes = repo.quest_repo().list_quests(roleid).await.unwrap();
    assert!(
        missoes.iter().any(|q| q.quest_id == NOVA),
        "a missão aceita não foi gravada"
    );
}

#[tokio::test]
async fn conjurar_habilidade_causa_dano_real_no_alvo_selecionado() {
    // No `gateway.rs` o dano era **150 fixo**, mandado por uma tarefa que dormia um
    // segundo e respondia sem olhar para nada — o monstro não perdia vida.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;

    // Sem lista de alvos: o servidor usa a seleção corrente.
    let mut corpo = 4321i32.to_le_bytes().to_vec(); // skill_id
    corpo.push(0); // force_attack
    corpo.push(0); // target_count

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CAST_SKILL, &corpo),
    })
    .await
    .unwrap();

    // OBJECT_CAST_SKILL (85), SKILL_PERFORM (88), o resultado (142) e a barra (33).
    let r = receber(&mut link, 4).await;
    assert!(r.iter().any(|v| cmd_de(v) == 85), "sem OBJECT_CAST_SKILL");
    assert!(r.iter().any(|v| cmd_de(v) == 88), "sem SKILL_PERFORM");

    let res = r.iter().find(|v| cmd_de(v) == 142).expect("sem o resultado (142)");
    let dano = i32_em(res, 10);
    assert!(dano > 0, "a habilidade não causou dano");
    assert_ne!(dano, 150, "o dano voltou a ser o valor fixo de antes");

    let hp = mundo.read().await.monsters[&MONSTRO].0.hp;
    assert_eq!(
        hp,
        MONSTRO_HP - dano as i64,
        "o HP do mundo não caiu exatamente o dano da habilidade"
    );
}

#[tokio::test]
async fn conjurar_no_alvo_da_lista_e_nao_no_selecionado() {
    // Quando o cliente manda a lista, ela manda. Este teste pega quem lê o alvo do
    // deslocamento errado: com `target_count` no meio, ler `data[7..11]` daria outro id.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let mut corpo = 4321i32.to_le_bytes().to_vec();
    corpo.push(0); // force_attack
    corpo.push(1); // target_count
    corpo.extend_from_slice(&(MONSTRO as i32).to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CAST_INSTANT_SKILL, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 4).await;
    let res = r.iter().find(|v| cmd_de(v) == 142).expect("sem o resultado");
    assert_eq!(
        i32_em(res, 2),
        MONSTRO as i32,
        "o alvo lido não é o que veio na lista"
    );
    assert!(
        mundo.read().await.monsters[&MONSTRO].0.hp < MONSTRO_HP,
        "o monstro da lista não levou dano"
    );
}

#[tokio::test]
async fn usar_pocao_cura_pelo_valor_do_elements_data() {
    // O `gateway.rs` reconhecia poção por dois ids escritos no código e respondia
    // HP/MP 120/280 fixos, sem curar nada. Aqui o quanto vem do `elements.data`.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    const POCAO: u32 = 7777;
    const CURA_HP: i32 = 37;
    {
        // Um remédio conhecido, posto direto no `elements` deste mundo de teste.
        let mut m = mundo.write().await;
        let dm = Arc::get_mut(&mut m.data_manager).expect("único dono do data_manager");
        dm.elements.medicines.insert(
            POCAO,
            pw_data_loader::MedicineTemplate {
                id: POCAO,
                name: "Poção de Teste".into(),
                hp_restore: CURA_HP,
                mp_restore: 0,
                cooldown_sec: 0.0,
                req_level: 1,
                price: 10,
            },
        );
        // E o jogador machucado, para que a cura tenha para onde ir.
        m.players.get_mut(&(roleid as i64)).unwrap().hp = 10;
    }

    let itens = mundo.read().await.char_repo.item_repo().clone();
    itens
        .upsert_item(&pw_core::ItemRecord {
            id: None,
            character_id: roleid,
            container_type: pw_core::ContainerType::Inventory,
            slot: 30,
            item_id: POCAO,
            count: 5,
            max_count: 99,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 1,
            max_durability: 1,
            bind_status: 0,
            octets: vec![],
            custom_attributes: serde_json::json!({}),
        })
        .await
        .unwrap();

    let mut corpo = vec![0u8, 1u8]; // where = bolsa, count = 1
    corpo.extend_from_slice(&30u16.to_le_bytes()); // index
    corpo.extend_from_slice(&(POCAO as i32).to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::USE_ITEM, &corpo),
    })
    .await
    .unwrap();

    // HOST_USE_ITEM (91), unfreeze (181) e os status (38).
    let r = receber(&mut link, 3).await;
    assert!(r.iter().any(|v| cmd_de(v) == 91), "sem HOST_USE_ITEM (91)");
    assert!(
        r.iter().any(|v| cmd_de(v) == 38),
        "sem SELF_INFO_00 (38) — a poção não curou"
    );

    let hp = mundo.read().await.players[&(roleid as i64)].hp;
    assert_eq!(
        hp,
        10 + CURA_HP,
        "a cura não foi a do `elements.data` (voltou aos 120 fixos?)"
    );

    // E a poção foi consumida.
    let sobrou = itens
        .get_item_by_slot(roleid, pw_core::ContainerType::Inventory, 30)
        .await
        .unwrap()
        .expect("a pilha inteira sumiu");
    assert_eq!(sobrou.count, 4, "usou uma e devia sobrar quatro");
}

#[tokio::test]
async fn nao_da_para_usar_item_que_nao_esta_no_slot() {
    // Sem conferir, o cliente escolhe o que usar — inclusive o que não tem.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    let hp_antes = mundo.read().await.players[&(roleid as i64)].hp;

    let mut corpo = vec![0u8, 1u8];
    corpo.extend_from_slice(&50u16.to_le_bytes()); // slot vazio
    corpo.extend_from_slice(&7777i32.to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::USE_ITEM, &corpo),
    })
    .await
    .unwrap();

    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "o mundo respondeu ao uso de um slot vazio");
    assert_eq!(
        mundo.read().await.players[&(roleid as i64)].hp,
        hp_antes,
        "o jogador foi curado por usar nada"
    );
}

/// Põe um segundo jogador no mundo e devolve o link dele, **já registrado**.
///
/// Grupo é a primeira coisa neste arquivo que precisa de **dois** clientes, e com dois
/// clientes aparece uma corrida que com um só não existia: cada um tem a sua conexão, e
/// nada ordena o `EnterWorld` de um contra o comando do outro. Se o convite chegar antes
/// do registro, o mundo não sabe para onde mandá-lo — comportamento correto, mas o teste
/// falharia por motivo errado.
///
/// A espera é uma ida-e-volta de verdade (`SIT_DOWN` e a resposta), e não um `sleep`: o
/// que se quer garantir é que o servidor **já processou** o `EnterWorld` daquela conexão,
/// e só a resposta prova isso.
async fn segundo_jogador(
    mundo: &Arc<RwLock<WorldInstance>>,
    addr: std::net::SocketAddr,
    roleid: i32,
) -> pw_bus::transport::BusConnection {
    mundo
        .write()
        .await
        .add_player(jogador(roleid, Vector3::new(2.0, 0.0, 2.0)));
    let mut link = entrar(addr, roleid).await;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SIT_DOWN, &[]),
    })
    .await
    .unwrap();
    receber(&mut link, 1).await;

    link
}

#[tokio::test]
async fn o_convite_de_grupo_chega_a_quem_foi_convidado() {
    // No `gateway.rs` o convite era mandado **de volta a quem convidou**: o convidado
    // nunca ficava sabendo, e o grupo — que não existia em lugar nenhum — jamais se
    // formava.
    let (mundo, addr, anfitriao) = cenario!();
    let mut link_a = entrar(addr, anfitriao).await;

    let convidado = anfitriao + 1;
    let mut link_b = segundo_jogador(&mundo, addr, convidado).await;

    link_a
        .enviar(BusMessage::ClientToGame {
            roleid: anfitriao,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_INVITE, &convidado.to_le_bytes()),
        })
        .await
        .unwrap();

    // Quem convidou não recebe nada; quem foi convidado recebe o convite (57).
    let nada = tokio::time::timeout(Duration::from_millis(300), link_a.receber()).await;
    assert!(
        nada.is_err(),
        "o convite voltou para quem convidou — é o bug de origem"
    );

    let v = receber(&mut link_b, 1).await;
    assert_eq!(cmd_de(&v[0]), 57, "o convidado não recebeu TEAM_LEADER_INVITE");
    assert_eq!(
        i32_em(&v[0], 2),
        anfitriao,
        "o convite não diz quem convidou"
    );
}

#[tokio::test]
async fn aceitar_forma_o_grupo_e_avisa_os_dois_com_dados_reais() {
    let (mundo, addr, anfitriao) = cenario!();
    let mut link_a = entrar(addr, anfitriao).await;
    let convidado = anfitriao + 1;
    let mut link_b = segundo_jogador(&mundo, addr, convidado).await;

    // Quatro valores **distintos entre si**, e diferentes do padrão.
    //
    // Distintos não é capricho: com `hp` e `max_hp` iguais a 100 e `mp` e `max_mp` iguais
    // a 50, trocar `mp` com `max_hp` de lugar produz um pacote diferente que o teste não
    // consegue distinguir. Foi o que aconteceu na primeira versão deste teste — a
    // asserção passava com os campos na ordem errada, que é exatamente o erro que o
    // layout antigo tinha.
    {
        let mut m = mundo.write().await;
        let p = m.players.get_mut(&(convidado as i64)).unwrap();
        p.hp = 42;
        p.mp = 43;
        p.max_hp = 44;
        p.max_mp = 45;
    }

    link_a
        .enviar(BusMessage::ClientToGame {
            roleid: anfitriao,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_INVITE, &convidado.to_le_bytes()),
        })
        .await
        .unwrap();
    receber(&mut link_b, 1).await;

    link_b
        .enviar(BusMessage::ClientToGame {
            roleid: convidado,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_AGREE_INVITE, &anfitriao.to_le_bytes()),
        })
        .await
        .unwrap();

    // Os **dois** recebem entrada no grupo (59) e a lista de membros (64).
    for (quem, link) in [("anfitrião", &mut link_a), ("convidado", &mut link_b)] {
        let r = receber(link, 2).await;
        assert!(
            r.iter().any(|v| cmd_de(v) == 59),
            "{quem} não recebeu TEAM_JOIN_TEAM (59)"
        );
        let lista = r
            .iter()
            .find(|v| cmd_de(v) == 64)
            .unwrap_or_else(|| panic!("{quem} não recebeu a lista de membros (64)"));

        // `cmd_team_member_data` (EC_GPDataType.h), com o cabeçalho de 2 bytes na frente:
        //   2: member_count (1)   3: data_count (1)   4: idLeader (4)   8: data[]
        // e cada MEMBER ocupa 34 bytes:
        //   +0 idMember(4) +4 level(2) +6 state(1) +7 level2(1) +8 reincarnation(1)
        //   +9 wallow(1) +10 hp(4) +14 mp(4) +18 max_hp(4) +22 max_mp(4)
        //   +26 force_id(4) +30 profit_level(4)
        //
        // Os dois primeiros bytes contam separado de propósito: o `CheckValid` do cliente
        // dimensiona o pacote por `data_count`, e era ele que faltava.
        assert_eq!(lista[2], 2, "member_count devia ser 2");
        assert_eq!(lista[3], 2, "data_count devia ser 2 — é por ele que o cliente conta");
        assert_eq!(i32_em(lista, 4), anfitriao, "o idLeader não é o anfitrião");

        const MEMBRO: usize = 34;
        const INICIO: usize = 8;
        let seg = INICIO + MEMBRO;

        // Cada campo no seu deslocamento, com o seu valor. A ordem do cliente é
        // `hp, mp, max_hp, max_mp` — e não `hp, max_hp, mp, max_mp`, que era o que
        // escrevíamos. Com quatro valores distintos, a troca aparece.
        assert_eq!(i32_em(lista, seg), convidado, "idMember errado ({quem})");
        assert_eq!(i32_em(lista, seg + 10), 42, "hp fora do lugar ({quem})");
        assert_eq!(i32_em(lista, seg + 14), 43, "mp fora do lugar ({quem})");
        assert_eq!(i32_em(lista, seg + 18), 44, "max_hp fora do lugar ({quem})");
        assert_eq!(i32_em(lista, seg + 22), 45, "max_mp fora do lugar ({quem})");
    }

    let m = mundo.read().await;
    assert_eq!(m.membros_do_grupo(anfitriao).len(), 2, "o grupo não se formou");
    assert_eq!(m.membros_do_grupo(convidado).len(), 2);
}

#[tokio::test]
async fn nao_da_para_entrar_num_grupo_sem_convite() {
    // Sem conferir o convite pendente, bastaria mandar o comando com o id de um estranho
    // para entrar no grupo dele.
    let (mundo, addr, intruso) = cenario!();
    let mut link = entrar(addr, intruso).await;
    let outro = intruso + 1;
    let _link_b = segundo_jogador(&mundo, addr, outro).await;

    link.enviar(BusMessage::ClientToGame {
        roleid: intruso,
        localsid: LOCALSID,
        data: subcomando(ids::TEAM_AGREE_INVITE, &outro.to_le_bytes()),
    })
    .await
    .unwrap();

    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "o mundo aceitou uma entrada sem convite");
    assert!(
        mundo.read().await.membros_do_grupo(intruso).is_empty(),
        "o intruso entrou no grupo sem ter sido convidado"
    );
}

#[tokio::test]
async fn sair_do_grupo_avisa_quem_ficou() {
    // No `gateway.rs` a saída era um eco para o próprio jogador: os companheiros
    // continuavam vendo alguém que já tinha ido embora.
    let (mundo, addr, anfitriao) = cenario!();
    let mut link_a = entrar(addr, anfitriao).await;
    let convidado = anfitriao + 1;
    let mut link_b = segundo_jogador(&mundo, addr, convidado).await;

    link_a
        .enviar(BusMessage::ClientToGame {
            roleid: anfitriao,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_INVITE, &convidado.to_le_bytes()),
        })
        .await
        .unwrap();
    receber(&mut link_b, 1).await;
    link_b
        .enviar(BusMessage::ClientToGame {
            roleid: convidado,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_AGREE_INVITE, &anfitriao.to_le_bytes()),
        })
        .await
        .unwrap();
    receber(&mut link_a, 2).await;
    receber(&mut link_b, 2).await;

    // O convidado sai.
    link_b
        .enviar(BusMessage::ClientToGame {
            roleid: convidado,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_LEAVE_PARTY, &[]),
        })
        .await
        .unwrap();

    // Quem ficou é avisado — é isto que não acontecia.
    //
    // E o comando é o `TEAM_MEMBER_LEAVE` (60), não o `TEAM_LEAVE_PARTY` (61): o 61 diz
    // "seu grupo acabou" e nem carrega o id de quem saiu, então mandá-lo a quem fica não
    // permite tirar a pessoa certa da lista.
    let r = receber(&mut link_a, 2).await;
    let saida = r
        .iter()
        .find(|v| cmd_de(v) == 60)
        .expect("o anfitrião não recebeu TEAM_MEMBER_LEAVE (60)");
    // `cmd_team_member_leave { int idLeader; int idMember; short reason; }`, depois do
    // cabeçalho de 2 bytes.
    assert_eq!(i32_em(saida, 2), anfitriao, "o idLeader não é o anfitrião");
    assert_eq!(i32_em(saida, 6), convidado, "saiu o membro errado");
    assert!(
        !r.iter().any(|v| cmd_de(v) == 61),
        "quem ficou recebeu 'seu grupo acabou' (61) em vez de 'o fulano saiu' (60)"
    );

    let m = mundo.read().await;
    assert!(
        m.membros_do_grupo(convidado).is_empty(),
        "quem saiu continuou no grupo"
    );
    assert!(
        m.membros_do_grupo(anfitriao).is_empty(),
        "grupo de um membro só devia ter sido desfeito"
    );
}

#[tokio::test]
async fn sair_do_mundo_tambem_sai_do_grupo() {
    // Sem isto o grupo guardaria um membro que não existe mais, e a lista mostraria um
    // fantasma que ninguém consegue expulsar.
    let (mundo, addr, anfitriao) = cenario!();
    let mut link_a = entrar(addr, anfitriao).await;
    let convidado = anfitriao + 1;
    let mut link_b = segundo_jogador(&mundo, addr, convidado).await;

    link_a
        .enviar(BusMessage::ClientToGame {
            roleid: anfitriao,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_INVITE, &convidado.to_le_bytes()),
        })
        .await
        .unwrap();
    receber(&mut link_b, 1).await;
    link_b
        .enviar(BusMessage::ClientToGame {
            roleid: convidado,
            localsid: LOCALSID,
            data: subcomando(ids::TEAM_AGREE_INVITE, &anfitriao.to_le_bytes()),
        })
        .await
        .unwrap();
    receber(&mut link_a, 2).await;
    receber(&mut link_b, 2).await;

    // O convidado desloga de vez.
    link_b
        .enviar(BusMessage::PlayerLogout {
            result: 0,
            roleid: convidado,
            provider_link_id: 1,
            localsid: LOCALSID,
        })
        .await
        .unwrap();

    let saiu = ate(|| {
        mundo
            .try_read()
            .map(|m| m.membros_do_grupo(anfitriao).is_empty())
            .unwrap_or(false)
    })
    .await;
    assert!(saiu, "o jogador que deslogou continuou no grupo");
}

// ---------------------------------------------------------------------------
// As consultas
//
// Quatro comandos com que o cliente pergunta ao servidor o estado do que está na tela.
// Todos estavam no `gateway.rs`, e todos respondiam número escrito no código — ou não
// respondiam nada — porque o daemon de link não tem simulação de onde tirar a verdade.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_consulta_periodica_devolve_o_hp_real_do_monstro() {
    // O `gateway.rs` respondia `1000/1000` fixo. Como esta consulta é **periódica**, ela
    // desfazia o combate: o golpe tirava vida no mundo e a consulta seguinte redesenhava
    // a barra cheia.
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    // Um dano qualquer, para que o HP consultado seja diferente do inicial.
    mundo.write().await.monsters.get_mut(&MONSTRO).unwrap().0.hp = 55;

    let mut corpo = 1u16.to_le_bytes().to_vec();
    corpo.extend_from_slice(&(MONSTRO as i32).to_le_bytes());
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::QUERY_NPC_INFO_1, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 1).await;
    let info = r
        .iter()
        .find(|v| cmd_de(v) == 33)
        .expect("sem NPC_INFO_00 (33)");

    // **12 bytes no 1.2.6**, medidos em 80 ocorrências de um servidor real: `idNPC`,
    // `iHP`, `iMaxHP` e **sem** o `iTargetID`, que só existe a partir do 1.5.3 (item 56).
    // O mundo deste teste é 1.2.6, então é este o tamanho esperado.
    assert_eq!(info.len(), 2 + 12, "NPC_INFO_00 com tamanho errado: o cliente descarta");
    assert_eq!(i32_em(info, 2), MONSTRO as i32);
    assert_eq!(i32_em(info, 6), 55, "veio HP fixo em vez do HP do mundo");
    assert_eq!(i32_em(info, 10), MONSTRO_HP_MAX as i32);
}

#[tokio::test]
async fn a_consulta_de_jogador_devolve_alguma_coisa() {
    // O `gateway.rs` lia a contagem, escrevia uma linha de log e **devolvia sem
    // responder**. Nenhum outro jogador tinha barra de vida na tela.
    let (mundo, addr, anfitriao) = cenario!();
    let mut link = entrar(addr, anfitriao).await;
    let outro = anfitriao + 1;
    let _link_b = segundo_jogador(&mundo, addr, outro).await;

    mundo.write().await.players.get_mut(&(outro as i64)).unwrap().hp = 77;

    let mut corpo = 1u16.to_le_bytes().to_vec();
    corpo.extend_from_slice(&outro.to_le_bytes());
    link.enviar(BusMessage::ClientToGame {
        roleid: anfitriao,
        localsid: LOCALSID,
        data: subcomando(ids::QUERY_PLAYER_INFO_1, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 1).await;
    let info = r
        .iter()
        .find(|v| cmd_de(v) == 32)
        .expect("sem PLAYER_INFO_00 (32) — a consulta continua muda");

    // `idPlayer(4) sLevel(2) State(1) Level2(1) iHP(4) iMaxHP(4) iMP(4) iMaxMP(4)` = 24
    // no 1.2.6 — **sem** o `iTargetID`, igual ao 33. Medido em 73 ocorrências.
    assert_eq!(info.len(), 2 + 24, "PLAYER_INFO_00 com tamanho errado");
    assert_eq!(i32_em(info, 2), outro);
    assert_eq!(i32_em(info, 10), 77, "o HP do outro jogador não é o do mundo");
}

#[tokio::test]
async fn o_proprio_estado_sai_do_personagem_e_nao_de_120_280() {
    // Terceira aparição do `120/120/280/280` escrito no código (itens 37 e 45).
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;

    {
        let mut m = mundo.write().await;
        let p = m.players.get_mut(&(roleid as i64)).unwrap();
        p.level = 23;
        p.hp = 91;
        p.mp = 17;
        p.money = 4242;
    }

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GET_EXT_PROP, &[]),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 2).await;
    let info = r
        .iter()
        .find(|v| cmd_de(v) == 38)
        .expect("sem SELF_INFO_00 (38)");
    // sLevel(2) State(1) Level2(1) iHP(4) iMaxHP(4) iMP(4) ...
    assert_eq!(i16::from_le_bytes([info[2], info[3]]), 23, "nível fixo");
    assert_eq!(i32_em(info, 6), 91, "veio vida fixa em vez da do personagem");
    assert_eq!(i32_em(info, 14), 17, "veio mana fixa em vez da do personagem");

    let saldo = r
        .iter()
        .find(|v| cmd_de(v) == 253)
        .expect("sem PLAYER_CASH (253)");
    // `struct player_cash { int cash_amount; }` — **um** campo. Escrevíamos dois.
    assert_eq!(saldo.len(), 2 + 4, "PLAYER_CASH com tamanho errado: o cliente descarta");
    assert_eq!(i32_em(saldo, 2), 4242, "o saldo veio de 50000 escrito no código");
}

#[tokio::test]
async fn get_all_data_respeita_os_sinalizadores_do_cliente() {
    // O `gateway.rs` não lia `detail_inv`/`detail_equip`/`detail_task`: mandava sempre
    // tudo. O servidor original passa os três adiante (`playercmd.cpp:1863`).
    let (mundo, addr, roleid) = cenario!();
    let mut link = entrar(addr, roleid).await;
    mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().money = 999;

    // Só o dinheiro e o marcador de fim: nada de bolsa, equipamento ou missões.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GET_ALL_DATA, &[0, 0, 0]),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 2).await;
    assert!(
        !r.iter().any(|v| cmd_de(v) == 42),
        "mandou a bolsa (42) com detail_inv = 0"
    );
    assert!(
        r.iter().any(|v| cmd_de(v) == 105),
        "faltou o TASK_DATA (105) — é o marcador que destrava o cliente, e vai sempre"
    );
    let saldo = r
        .iter()
        .find(|v| cmd_de(v) == 253)
        .expect("sem PLAYER_CASH (253)");
    assert_eq!(i32_em(saldo, 2), 999);

    // E com os sinalizadores ligados, a bolsa vem.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GET_ALL_DATA, &[1, 1, 1]),
    })
    .await
    .unwrap();
    let r = receber(&mut link, 3).await;
    assert!(
        r.iter().any(|v| cmd_de(v) == 42),
        "não mandou a bolsa (42) nem com detail_inv = 1"
    );
}
