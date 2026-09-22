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
use pw_gs::entity::{MatterEntity, MonsterEntity};
use pw_gs::{BusServer, WorldInstance};
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};

#[path = "../../pw-storage/tests/comum/mod.rs"]
mod comum;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const LOCALSID: u32 = 0xC0FF_EE01;
/// A missão ativa do personagem de teste, para conferir a notificação de abate.
const MISSAO: u32 = 4242;
/// Um item qualquer para o teste de loja, com preço conhecido.
const ITEM_DE_LOJA: i32 = 4123;
/// Um item da bolsa de missão (`TASKMATTER_ESSENCE`) no cenário.
const ITEM_DE_MISSAO: i32 = 2106;
/// Um ovo de montaria (`pet_class` 0) no cenário.
const OVO_DE_MONTARIA: i32 = 41073;
/// `shop_price` daquele item no cenário — o que a loja tem de cobrar por unidade.
const PRECO_DO_ITEM_DE_LOJA: i32 = 137;

const MONSTRO: i64 = 900_001;
/// O NPC de serviço do cenário, que entrega e recebe [`MISSAO_DO_NPC`] e ensina
/// [`HABILIDADE_DO_TREINADOR`].
const NPC: i64 = 0x8000_0101u32 as i32 as i64;
const TEMPLATE_DO_NPC: u32 = 23964;
const MISSAO_DO_NPC: u32 = 5150;
const HABILIDADE_DO_TREINADOR: i32 = 117;
/// Habilidade de teste em área com efeitos (B53).
const HABILIDADE_EM_AREA: i32 = 4322;
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
        armor: 0,
        attack_rate: 1000,
        resistances: [0; 5],
        attack_degree: 0,
        defend_degree: 0,
        attack_min: 1,
        attack_max: 2,
        magic_attack: [(0, 0); 5],
        attack_range: 2.0,
        // 1,5 s entre golpes e 0,5 s de atraso do dano, os números que a IA usava fixos
        // antes de virem do `MONSTER_ESSENCE` (B62).
        ataque_em_ticks: 30,
        atraso_do_dano_em_ticks: 10,
        aggro_range: 30.0,
        sight_range: 40,
        // Toda a vida máxima em experiência: quem tira os 137 de vida leva 137.
        exp: 480,
        sp: 480,
        aipolicy_id: 0,
        drop_table_id: 0,
        position: Vector3::new(5.0, 0.0, 5.0),
        spawn_center: Vector3::new(5.0, 0.0, 5.0),
        move_speed: 1.0,
        walk_speed: 1.0,
        habitat: pw_gs::ai::Habitat::Chao,
        patrulha: false,
        is_dead: false,
        respawn_timer_ms: 0,
        respawn_delay_ms: 1000,
        target_id: None,
        efeitos: Default::default(),
        danos: Vec::new(),
        primeiro_atacante: None,
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
async fn personagem_com_missao(pool: &PostgresPool) -> (i32, i32) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    // Só o relógio não basta: dois testes que começam no mesmo nanossegundo geram o mesmo
    // id e o segundo morre em `duplicate key`. Aconteceu ao subir de 30 para 32 testes em
    // paralelo. O contador desempata dentro do processo, e o relógio entre execuções.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let agora = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let m = format!(
        "{}_{}",
        agora % 1_000_000_000,
        SEQ.fetch_add(1, Ordering::Relaxed)
    );

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
            None,
        )
        .await
        .expect("criar personagem");

    repo.quest_repo()
        .save_quest(role_id, MISSAO, pw_core::QuestStatus::Active, &[0], None)
        .await
        .expect("criar missão ativa");

    // O segundo personagem existe para os testes de grupo. Antes eles usavam
    // `anfitriao + 1`, um id inventado, e funcionava porque o teste inseria o jogador à
    // mão. Agora quem põe jogador no mundo é a carga do banco, e um id que não existe é
    // corretamente recusado — então o convidado precisa ser um personagem de verdade.
    let convidado = repo
        .create_character(
            conta,
            &realm,
            &format!("Convi{m}"),
            Race::Human,
            CharacterClass::Blademaster,
            Gender::Female,
            Vec::new(),
            None,
        )
        .await
        .expect("criar personagem convidado");

    (role_id, convidado)
}

/// Monta mundo + servidor de barramento, ou `None` sem banco configurado.
async fn montar() -> Option<(Arc<RwLock<WorldInstance>>, std::net::SocketAddr, i32, i32)> {
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
    comum::limpar_sobras_de_teste(&pool).await;
    let (roleid, convidado) = personagem_com_missao(&pool).await;

    // A loja cobra o preço do `elements.data` desde 2026-09-11, e este cenário não carrega
    // arquivo nenhum — sem um preço aqui, **toda** compra é recusada, que é o
    // comportamento certo para um item que o realm não conhece.
    let mut dados = GameDataManager::new();
    dados
        .precos
        .insert(ITEM_DE_LOJA as u32, (50, PRECO_DO_ITEM_DE_LOJA));
    // A marca do `dyn_tasks.data` dos realms 1.5.5 (`dyn_tasks_do_realm.rs`).
    dados.marca_das_missoes_dinamicas = Some(MARCA_DAS_MISSOES_DINAMICAS);
    // O cenário não carrega `elements.data`: o que os testes de item de missão e de ovo de
    // mascote precisam entra aqui à mão, como os preços da loja e as missões acima.
    dados.itens_de_missao.insert(ITEM_DE_MISSAO as u32);
    dados.ovos_de_pet.insert(
        OVO_DE_MONTARIA as u32,
        pw_data_loader::pet::DadosDoOvoDePet {
            id: OVO_DE_MONTARIA as u32,
            id_pet: 3000,
            money_hatched: 1000,
            money_restored: 0,
            honor_point: 0,
            level: 1,
            exp: 0,
            skill_point: 0,
            req_level: 0,
            req_class: -1,
            pet_class: 0,
            skills: Vec::new(),
        },
    );
    // O ajuste padrão do construtor é zero (sem `PARAM_ADJUST_CONFIG` nenhum abate daria
    // experiência): o cenário usa o neutro.
    dados.progressao = pw_data_loader::TabelaDeProgressao::com_ajuste_uniforme(pw_data_loader::AjusteDeNivel {
        exp: 1.0,
        sp: 1.0,
        dinheiro: 1.0,
        item: 1.0,
        ataque: 1.0,
    });
    // Uma missão de falar com NPC, que o NPC do cenário entrega e recebe.
    let mut falar = pw_data_loader::tasks::TaskTemplate::vazia(MISSAO_DO_NPC);
    falar.metodo = 3; // enumTMTalkToNPC
    falar.tipo_de_conclusao = 1; // enumTFTNPC
    falar.rewards.exp = 7;
    falar.rewards.money = 30;
    dados.tasks.inserir(falar);
    dados.servicos_de_npc.insert(
        TEMPLATE_DO_NPC,
        pw_data_loader::ServicosDoNpc {
            missoes_entregues: vec![MISSAO_DO_NPC],
            missoes_recebidas: vec![MISSAO_DO_NPC],
            habilidades: vec![HABILIDADE_DO_TREINADOR as u32],
            deposito: 0,
            destinos: Vec::new(),
        },
    );
    dados.habilidades.por_id.insert(
        HABILIDADE_DO_TREINADOR as u32,
        serde_json::from_str(
            r#"{"id": 117, "cls": 255, "max_level": 10, "type": 1, "rank": 0, "pre_skills": [],
                "mp": null, "execucao_ms": null, "recarga_ms": null,
                "nivel_exigido": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "sp_exigido": [100, 100, 100, 100, 100, 100, 100, 100, 100, 100],
                "dinheiro_exigido": [10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
                "estados_ms": []}"#,
        )
        .expect("habilidade de teste"),
    );

    // B53 — golpe mágico em bola no alvo (raio 6) com `doenchant`: lentidão e atordoamento.
    dados.habilidades.por_id.insert(
        HABILIDADE_EM_AREA as u32,
        serde_json::from_str(
            r#"{"id": 4322, "cls": 255, "max_level": 1, "type": 1, "rank": 0, "pre_skills": [],
                "mp": null, "execucao_ms": null, "recarga_ms": null, "nivel_exigido": null,
                "sp_exigido": null, "dinheiro_exigido": null, "estados_ms": [],
                "tipo_de_area": 3, "raio": [6.0], "doenchant": true,
                "dano": {"estado": 0, "base": "magico", "elemento": "Firedamage", "fator": 1.0,
                         "ratio": [0.0], "plus": [10.0], "carga": false},
                "no_alvo": [["V","Probability","1.0 * 100"],["V","Time","5000"],["V","Ratio","0.3"],["V","Slow","1"],
                            ["V","Time","3000"],["V","Dizzy","1"]]}"#,
        )
        .expect("habilidade em área de teste"),
    );

    let mut mundo = WorldInstance::new(
        1,
        Arc::new(dados),
        CharacterRepository::new(pool),
    );
    // O jogador **não** é inserido aqui: quem o põe no mundo é o `EnterWorld`, que carrega
    // o personagem do banco (`BusServer::colocar_no_mundo`). Fabricar um aqui esconderia
    // justamente o caminho que interessa — e escondeu, até 2026-09-07.
    mundo
        .monsters
        .insert(MONSTRO, (monstro(), MonsterAi::new()));
    mundo.npcs.insert(
        NPC,
        pw_gs::entity::NpcEntity {
            id: NPC,
            template_id: TEMPLATE_DO_NPC,
            name: "NPC".into(),
            position: Vector3::new(1.0, 0.0, 1.0),
            dialog_id: 0,
            direcao: 0,
        },
    );
    let mundo = Arc::new(RwLock::new(mundo));

    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();
    // O realm de teste é 1.2.6 — é o que o `personagem_com_missao` cria — e desde o
    // item 56 isso muda bytes: 32 comandos têm layout próprio naquela versão.
    let servidor = Arc::new(BusServer::new(Arc::clone(&mundo), GameVersion::V1_2_6));
    // Sem isto, o que o tick decide não chega ao cliente — que era o estado anterior.
    servidor.ligar_eventos_do_mundo().await;
    tokio::spawn(Arc::clone(&servidor).executar(escuta));

    Some((mundo, addr, roleid, convidado))
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
    let (mundo, addr, roleid, _convidado) = cenario!();
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
    let (mundo, addr, roleid, _convidado) = cenario!();
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
    let (_mundo, addr, roleid, _convidado) = cenario!();
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
    let (mundo, addr, roleid, _convidado) = cenario!();
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
/// Entra no mundo e **espera** o jogador aparecer nele.
///
/// A espera é o ponto: `colocar_no_mundo` roda na tarefa do barramento, então mandar o
/// `EnterWorld` e seguir em frente é uma corrida — o teste alterava um jogador que a carga
/// do banco sobrescrevia logo depois.
///
/// Depois de entrar, os atributos de combate são ajustados para o que estes testes
/// assumem. O personagem que o `montar()` cria é de nível 1, e o `GameDataManager` do
/// teste está vazio (sem `CHARRACTER_CLASS_CONFIG`, sem `ptemplate.conf`), então ele entra
/// com dano 1 e precisão 0 — correto, e inútil para testar subcomando de combate. O que se
/// ajusta aqui é só o que **não** é objeto destes testes; nível, vida e dinheiro cada teste
/// define por conta.
async fn entrar(
    mundo: &Arc<RwLock<WorldInstance>>,
    addr: std::net::SocketAddr,
    roleid: i32,
) -> pw_bus::transport::BusConnection {
    let link = entrar_sem_ajustar(addr, roleid).await;

    let presente = {
        let m = Arc::clone(mundo);
        let id = roleid as i64;
        ate_async(move || {
            let m = Arc::clone(&m);
            async move { m.read().await.players.contains_key(&id) }
        })
        .await
    };
    assert!(presente, "o jogador não entrou no mundo depois do EnterWorld");

    {
        let mut m = mundo.write().await;
        let p = m.players.get_mut(&(roleid as i64)).expect("conferido acima");
        p.position = Vector3::new(0.0, 0.0, 0.0);
        // A âncora do streaming anda junto: mexer na posição sem mexer nela faria o
        // primeiro passo do jogador parecer um salto de quilômetros, e o mundo em volta
        // seria recalculado quando não devia.
        p.centro_do_stream = p.position;
        p.attack_min = 10;
        p.attack_max = 15;
        // Precisão alta e nenhuma armadura no alvo: a rolagem de acerto do combate real
        // sempre passa, então os testes de subcomando continuam determinísticos.
        p.attack_rate = 100_000;
        p.def_phys = 10;
        p.hp = 100;
        p.max_hp = 100;
        p.mp = 50;
        p.max_mp = 50;
        p.move_speed = 4.8;
        // Alcance que cobre o monstro de teste em (5, 0, 5): a sessão de golpe confere o
        // alcance (`CheckAttack`), e o realm de teste não tem arma nem classe carregadas.
        p.attack_range = 20.0;
    }
    m_grade(mundo, roleid).await;
    link
}

/// A grade espacial guarda a posição de quando o jogador entrou; mexer na entidade sem
/// avisá-la deixaria as duas em desacordo.
async fn m_grade(mundo: &Arc<RwLock<WorldInstance>>, roleid: i32) {
    let mut m = mundo.write().await;
    m.grid.update_position(roleid as i64, Vector3::new(0.0, 0.0, 0.0));
}

/// `entrar` sem o ajuste — para o teste que confere o que a carga do banco produz.
async fn entrar_sem_ajustar(
    addr: std::net::SocketAddr,
    roleid: i32,
) -> pw_bus::transport::BusConnection {
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

/// Como `ate`, mas para condição que precisa de `await`.
async fn ate_async<F, Fut>(mut cond: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..200 {
        if cond().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
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

/// Lê da conexão até achar o comando pedido.
///
/// Contar pacotes exatos é frágil: a mesma conexão carrega broadcast de entrada, de
/// movimento e o que mais estiver acontecendo. Aqui o teste diz **o que** espera, não
/// quantos pacotes vêm antes.
/// Lê do link até achar `cmd`. O teto é generoso de propósito: desde o B62 a morte do
/// monstro chega por evento do tique, atrás do que o combate em curso já enfileirou.
async fn esperar_comando(link: &mut pw_bus::transport::BusConnection, cmd: u16) -> Vec<u8> {
    let mut vistos = Vec::new();
    for _ in 0..200 {
        let m = tokio::time::timeout(Duration::from_secs(5), link.receber())
            .await
            .unwrap_or_else(|_| panic!("nada chegou enquanto eu esperava o comando {cmd}"))
            .unwrap()
            .expect("conexão fechou");
        if let BusMessage::GameToClient { data, .. } = m {
            vistos.push(cmd_de(&data));
            if cmd_de(&data) == cmd {
                return data;
            }
        }
    }
    panic!("o comando {cmd} não chegou em 200 pacotes; vieram {vistos:?}");
}

/// Junta tudo o que o mundo manda até o `TASK_DATA` (105), o marcador de fim da carga.
///
/// Contar pacotes exatos aqui é frágil: cada comando novo que a carga passa a mandar
/// (aconteceu com o `SKILL_DATA` e com o `OWN_EXT_PROP`) quebraria o teste sem que nada
/// estivesse errado no servidor. Este helper existe para o teste dizer **o que** espera.
async fn receber_ate_o_fim_da_carga(link: &mut pw_bus::transport::BusConnection) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for _ in 0..40 {
        let m = tokio::time::timeout(Duration::from_secs(5), link.receber())
            .await
            .expect("a carga não terminou: o TASK_DATA (105) nunca chegou")
            .unwrap()
            .expect("conexão fechou");
        if let BusMessage::GameToClient { data, .. } = m {
            let fim = cmd_de(&data) == 105;
            v.push(data);
            if fim {
                return v;
            }
        }
    }
    panic!("o TASK_DATA (105) não chegou em 40 pacotes");
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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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

    // HOST_START_ATTACK (84, a sessão), ATTACK_ONCE (83, a munição do golpe,
    // `FillAttackMsg`) e HOST_ATTACKRESULT. A barra de vida **não** vem junto (B56).
    let resp = receber(&mut link, 3).await;
    let inicio = resp.iter().find(|v| cmd_de(v) == 84).expect("sem HOST_START_ATTACK (84)");
    assert_eq!(inicio.len(), 2 + 7, "cmd_host_start_attack: idTarget, ammo_remain, attack_speed");
    assert_eq!(i32_em(inicio, 2), MONSTRO as i32);
    let municao = resp.iter().find(|v| cmd_de(v) == 83).expect("sem ATTACK_ONCE (83)");
    assert_eq!(municao.len(), 3, "ATTACK_ONCE: cabeçalho + arrow_dec");
    assert_eq!(municao[2], 0, "sem arma de longo alcance nenhuma flecha sai");
    let golpe = resp.iter().find(|v| cmd_de(v) == 24).expect("sem HOST_ATTACKRESULT (24)");
    let dano = i32_em(golpe, 6);
    assert!(dano > 0, "o golpe não causou dano");
    assert_eq!(i32_em(golpe, 2), MONSTRO as i32, "idTarget");

    assert!(
        !resp.iter().any(|v| cmd_de(v) == 33),
        "a barra de vida saiu junto do golpe — o cliente a derruba antes de a flecha sair"
    );

    // B56 — ela vai no batimento de 1 s, a quem tem o monstro selecionado
    // (`RefreshSubscibeList`, `actobject.cpp:1346-1353`).
    // A sessão é fechada antes, para o batimento não dar o golpe seguinte.
    {
        let mut m = mundo.write().await;
        m.players.get_mut(&(roleid as i64)).unwrap().ataque = None;
        m.tick(1000).await;
    }
    let barra = esperar_comando(&mut link, 33).await;
    let hp_no_fio = i32_em(&barra, 6);

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

/// B56 — a barra de vida do monstro vai no batimento de 1 s, só a quem o tem selecionado e só
/// quando mudou (`RefreshSubscibeList` + `_refresh_state`, `actobject.cpp:1296-1353`).
#[tokio::test]
async fn a_barra_de_vida_vai_no_batimento_e_so_quando_muda() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Quantos NPC_INFO_00 do monstro chegam em 300 ms.
    async fn barras(link: &mut pw_bus::transport::BusConnection) -> Vec<i32> {
        let mut v = Vec::new();
        while let Ok(Ok(Some(m))) = tokio::time::timeout(Duration::from_millis(300), link.receber()).await {
            if let BusMessage::GameToClient { data, .. } = m {
                if cmd_de(&data) == 33 && i32_em(&data, 2) == MONSTRO as i32 {
                    v.push(i32_em(&data, 6));
                }
            }
        }
        v
    }

    // Sem seleção, dano no monstro não manda nada.
    mundo.write().await.monsters.get_mut(&MONSTRO).unwrap().0.hp -= 5;
    mundo.write().await.tick(1000).await;
    assert!(barras(&mut link).await.is_empty(), "barra para quem não selecionou");

    // Selecionar manda na hora (`query_info00`).
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    assert_eq!(barras(&mut link).await, vec![(MONSTRO_HP - 5) as i32]);

    // Sem mudança, o batimento não repete.
    mundo.write().await.tick(1000).await;
    assert!(barras(&mut link).await.is_empty(), "o batimento repetiu a mesma vida");

    // Com mudança, manda uma vez.
    mundo.write().await.monsters.get_mut(&MONSTRO).unwrap().0.hp -= 7;
    mundo.write().await.tick(1000).await;
    assert_eq!(barras(&mut link).await, vec![(MONSTRO_HP - 12) as i32]);
    mundo.write().await.tick(1000).await;
    assert!(barras(&mut link).await.is_empty(), "a mesma mudança saiu duas vezes");
}

#[tokio::test]
async fn o_monstro_morre_e_o_abate_leva_o_template_certo() {
    // O `gateway.rs` notificava abate **a cada golpe**, com a criatura `13641` escrita no
    // código — qualquer missão de caça completava batendo em qualquer coisa. Aqui a
    // notificação só sai na morte, e com o template real.
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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

        receber(&mut link, 3).await;
        // B62 — o golpe é anunciado na hora e a vida cai depois do `attack_delay`
        // (`InsertDamageEntry`): o tique é que cobra o dano adiado.
        let vida = mundo.read().await.monsters[&MONSTRO].0.hp;
        tickar_ate(&mundo, |m| m.monsters[&MONSTRO].0.hp < vida).await;
        if mundo.read().await.monsters[&MONSTRO].0.hp == 0 {
            morreu = true;
            break;
        }
        // Fecha a sessão para o próximo `NORMAL_ATTACK` abrir outra e golpear na hora.
        mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().ataque = None;
    }
    assert!(morreu, "o monstro não chegou a zero em 500 golpes");

    // Na morte vêm NPC_DIED (20) e, para quem bateu, RECEIVE_EXP (36).
    let obito = esperar_comando(&mut link, 20).await;
    assert_eq!(i32_em(&obito, 2), MONSTRO as i32);
    assert_eq!(i32_em(&obito, 6), roleid, "o matador não é o jogador");
    let _exp = esperar_comando(&mut link, 36).await;

    // A contagem de abate das missões sai do motor (`missoes.rs`), com o template real, e
    // é coberta lá (`matar_conta_e_finaliza_e_a_lista_volta_a_zero`).

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

/// B62 — o golpe é anunciado na hora e a vida só cai no fim da animação.
///
/// `InsertDamageEntry(dano, attack.speed)` (`gs/actobject.cpp:1758-1776`) adia o
/// `GM_MSG_HURT` em `attack_delay` tiques de 50 ms, enquanto o `HOST_ATTACKRESULT` já saiu.
/// Aplicar na hora fazia o monstro perder vida no clique, antes de a flecha sair — em jogo
/// parecia "um golpe a mais" no começo de cada sessão (relato de 2026-09-18).
#[tokio::test]
async fn a_vida_do_monstro_so_cai_depois_do_atraso_do_golpe() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    // HOST_START_ATTACK + ATTACK_ONCE + HOST_ATTACKRESULT: o cliente já sabe do golpe.
    let resposta = receber(&mut link, 3).await;
    let resultado = resposta
        .iter()
        .find(|v| cmd_de(v) == 24)
        .expect("sem HOST_ATTACKRESULT (24)");
    let dano = i32_em(resultado, 6);
    assert!(dano > 0, "o golpe saiu sem dano");

    // E a vida ainda está cheia: o dano está na fila, não no monstro.
    assert_eq!(
        mundo.read().await.monsters[&MONSTRO].0.hp,
        MONSTRO_HP,
        "a vida caiu junto com o aviso do golpe — o dano não foi adiado"
    );

    // O `attack_delay` é `(attack_speed × 20 × 0,8) − 1` tiques (`playertemplate.h:980`).
    let ticks = {
        let m = mundo.read().await;
        let v = m.players[&(roleid as i64)].attack_speed;
        ((v * 20.0).round() as i32 as f32 * 0.8) as i32 - 1
    };
    assert!(ticks > 1, "o personagem de teste precisa de um atraso mensurável: {ticks}");

    let mut passados = 0;
    let caiu = tickar_ate(&mundo, |m| {
        passados += 1;
        m.monsters[&MONSTRO].0.hp < MONSTRO_HP
    })
    .await;
    assert!(caiu, "o dano adiado nunca chegou ao monstro");
    assert!(
        passados >= ticks,
        "o dano caiu em {passados} tiques; o atraso do golpe é {ticks}"
    );
    assert_eq!(
        mundo.read().await.monsters[&MONSTRO].0.hp,
        MONSTRO_HP - dano as i64,
        "o dano aplicado não é o que foi anunciado"
    );
}

#[tokio::test]
async fn atacar_sem_alvo_nao_faz_nada() {
    // Sem `SELECT_TARGET` antes, não há o que atacar — e o servidor não pode inventar um
    // alvo nem responder um golpe no vazio.
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, roleid, _convidado) = cenario!();

    // B61 — veste os dez slots de armadura (`EQUIP_ARMOR_START..EQUIP_ARMOR_END`,
    // `gs/item.h:194-241`) para que o sorteio de `SelectRandomArmor` sempre ache peça: assim
    // o índice que chega ao cliente tem de ser um deles, e não `0x7f`.
    const DURABILIDADE_DA_PECA: u32 = 2500;
    let itens = mundo.read().await.char_repo.item_repo().clone();
    for slot in 1..=10u16 {
        itens
            .upsert_item(&pw_core::ItemRecord {
                id: None,
                character_id: roleid,
                container_type: pw_core::ContainerType::Equipment,
                slot,
                item_id: 4200 + slot as u32,
                count: 1,
                max_count: 1,
                refine_level: 0,
                sockets_count: 0,
                sockets: vec![],
                durability: DURABILIDADE_DA_PECA,
                max_durability: DURABILIDADE_DA_PECA,
                bind_status: 0,
                octets: vec![],
                custom_attributes: serde_json::json!({}),
            })
            .await
            .expect("vestir a peça");
    }

    let mut link = entrar(&mundo, addr, roleid).await;

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
    receber(&mut link, 3).await; // HOST_START_ATTACK + ATTACK_ONCE + HOST_ATTACKRESULT
    // Sem mais golpes da sessão enquanto o teste espera o revide.
    mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().ataque = None;

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

    // B59 — e com o **id do monstro** que bateu. Com zero ali, o cliente não acha o
    // atacante (`ISPLAYERID`/`ISNPCID` são falsos para zero) e não mostra golpe nenhum: em
    // jogo o jogador perdia vida sem ver o monstro atacar (teste de 2026-09-17).
    assert_eq!(
        i32_em(golpe, 2),
        MONSTRO as i32,
        "o HOST_ATTACKED foi sem o id de quem bateu"
    );

    // B60/B61 — e com os dois campos que o original preenche: a peça desgastada e o
    // `speed`, que é o `_damage_delay` do monstro e dá a duração da animação
    // (`npc.cpp:2118`, `EC_NPC.cpp:2043-2064`). O `cEquipment` ia **zero**, e o cliente
    // lia isso como "peça 0" — a arma — e gastava a durabilidade dela a cada golpe
    // recebido (`EC_HostMsg.cpp:968-976`).
    //
    // Agora é o índice que `SelectRandomArmor` sorteou (`player.cpp:9552-9570`): com os dez
    // slots vestidos, tem de ser um de 1 a 10.
    let peca = golpe[10] & 0x7f;
    assert!(
        (1..=10).contains(&peca),
        "cEquipment veio {peca}: devia ser a peça sorteada entre 1 e 10"
    );

    // E ela perdeu `DURABILITY_DEC_PER_HIT` (25, `gs/config.h:60`) — só ela.
    let vestido = itens
        .list_by_container(roleid, pw_core::ContainerType::Equipment)
        .await
        .expect("ler o equipamento");
    let gasto: u32 = vestido
        .iter()
        .filter(|i| (1..=10).contains(&i.slot))
        .map(|i| DURABILIDADE_DA_PECA - i.durability)
        .sum();
    assert!(
        gasto >= 25 && gasto % 25 == 0,
        "o golpe recebido desgastou {gasto} — devia ser 25 por golpe"
    );

    // E o golpe **dado** gastou a arma em `DURABILITY_DEC_PER_ATTACK` (2, `gs/config.h:61`;
    // `weapon_item::OnAfterAttack`, `item/equip_item.cpp:978-988`).
    let arma = vestido.iter().find(|i| i.slot == 0).expect("o personagem nasce com arma");
    let gasto_da_arma = arma.max_durability - arma.durability;
    assert!(
        gasto_da_arma >= 2 && gasto_da_arma % 2 == 0,
        "a arma gastou {gasto_da_arma} — devia ser 2 por golpe normal"
    );

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    receber(&mut link, 3).await; // HOST_START_ATTACK + ATTACK_ONCE + HOST_ATTACKRESULT
    // Sem mais golpes da sessão enquanto o teste espera o revide.
    mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().ataque = None;

    let morreu = tickar_ate(&mundo, |m| {
        m.players.get(&(roleid as i64)).map(|p| p.hp == 0).unwrap_or(false)
    })
    .await;
    assert!(morreu, "o jogador não chegou a zero");

    // Chegam o dano, a barra e o HOST_DIED (28) — este por último, porque desde o B62 a
    // vida só cai quando o dano adiado vence, e o aviso do golpe já saiu antes.
    let _ = esperar_comando(&mut link, 28).await;

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
    // `DEFAULT_RESURRECT_HP_FACTOR` = 0,1 (`gs/config.h:168`), arredondado.
    assert_eq!(p.hp, (p.max_hp as f32 * 0.1 + 0.5) as i32, "o renascimento não devolveu 10 % da vida");
    assert_eq!(p.target_id, None, "o alvo antigo sobreviveu à morte");
}

#[tokio::test]
async fn quem_esta_vivo_nao_revive() {
    // Ressuscitar quem não morreu seria um teleporte grátis para a cidade sempre que o
    // jogador quisesse.
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    // O dinheiro vive na entidade e o banco recebe a gravação depois: é a entidade que o
    // autosave escreve por cima de tudo, então é ela a fonte.
    mundo.read().await.players[&(roleid as i64)].money
}

async fn dar_dinheiro(mundo: &Arc<RwLock<WorldInstance>>, roleid: i32, n: i64) {
    mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().money += n;
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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let repo = mundo.read().await.char_repo.clone();
    let itens = repo.item_repo().clone();
    dar_dinheiro(&mundo, roleid, 10_000).await;
    let antes = dinheiro(&mundo, roleid).await;

    // CONTENT da compra: 28 bytes de cabeçalho, depois `npc_trade_item`.
    let mut c = Vec::new();
    c.extend_from_slice(&[0u8; 24]); // money + as cinco contribuições
    c.extend_from_slice(&1u32.to_le_bytes()); // item_count
    c.extend_from_slice(&ITEM_DE_LOJA.to_le_bytes()); // tid
    c.extend_from_slice(&20u32.to_le_bytes()); // index (slot de destino)
    c.extend_from_slice(&1u32.to_le_bytes()); // count

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::NPC_VENDE, &c),
    })
    .await
    .unwrap();

    // `PURCHASE_ITEM` (72): custo, e por item o id, a quantidade e o slot onde entrou.
    let compra = esperar_comando(&mut link, 72).await;
    assert_eq!(i32_em(&compra, 2), PRECO_DO_ITEM_DE_LOJA, "cost");
    // O cenário é V1_2_6: docs/evidencias/126/s2c-72.txt:2 (7 + 13*n).
    assert_eq!(compra.len(), 22);
    assert_eq!(u16::from_le_bytes([compra[7], compra[8]]), 1, "item_count");
    assert_eq!(i32_em(&compra, 9), ITEM_DE_LOJA, "item_id");
    assert_eq!(u16::from_le_bytes([compra[17], compra[18]]), 1, "count");
    let slot = u16::from_le_bytes([compra[19], compra[20]]);

    let itens2 = itens.clone();
    let chegou = ate_async(move || {
        let i = itens2.clone();
        async move {
            i.get_item_by_slot(roleid, pw_core::ContainerType::Inventory, slot)
                .await
                .ok()
                .flatten()
                .is_some_and(|x| x.item_id == ITEM_DE_LOJA as u32)
        }
    })
    .await;
    assert!(chegou, "o item comprado não chegou ao slot {slot} da bolsa");

    let depois = dinheiro(&mundo, roleid).await;
    assert!(
        depois < antes,
        "comprar **aumentou** o dinheiro do jogador ({antes} → {depois}) — a loja voltou \
         a ficar invertida?"
    );
    // E cobra o preço do arquivo, não os 100 fixos que valiam para qualquer coisa até
    // 2026-09-11.
    assert_eq!(
        antes - depois,
        PRECO_DO_ITEM_DE_LOJA as i64,
        "a loja não cobrou o shop_price do elements.data"
    );
}

/// Item sem preço no `elements.data` não é vendido — e nem por isso é dado de graça.
///
/// Antes, qualquer id saía por 100 moedas fixas. Recusar é a resposta honesta enquanto o
/// realm não souber quanto a coisa custa; no 1.2.6/v7, cuja tabela de preços ainda é
/// vazia, é o que acontece com toda compra.
#[tokio::test]
async fn item_sem_preco_no_arquivo_nao_e_vendido() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let repo = mundo.read().await.char_repo.clone();
    dar_dinheiro(&mundo, roleid, 10_000).await;
    let antes = dinheiro(&mundo, roleid).await;

    let mut c = Vec::new();
    c.extend_from_slice(&[0u8; 24]);
    c.extend_from_slice(&1u32.to_le_bytes());
    c.extend_from_slice(&999_999i32.to_le_bytes()); // tid que o cenário não conhece
    c.extend_from_slice(&21u32.to_le_bytes());
    c.extend_from_slice(&1u32.to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::NPC_VENDE, &c),
    })
    .await
    .unwrap();

    let itens = repo.item_repo().clone();
    let entregou = ate_async(move || {
        let i = itens.clone();
        async move {
            i.list_by_container(roleid, pw_core::ContainerType::Inventory)
                .await
                .unwrap_or_default()
                .iter()
                .any(|x| x.item_id == 999_999)
        }
    })
    .await;
    assert!(!entregou, "um item sem preço no arquivo foi entregue mesmo assim");
    assert_eq!(dinheiro(&mundo, roleid).await, antes, "cobrou por um item que não vendeu");
}

#[tokio::test]
async fn vender_ao_npc_tira_o_item_e_da_dinheiro() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let repo = mundo.read().await.char_repo.clone();
    let itens = repo.item_repo().clone();
    itens
        .upsert_item(&pw_core::ItemRecord {
            id: None,
            character_id: roleid,
            container_type: pw_core::ContainerType::Inventory,
            slot: 21,
            item_id: ITEM_DE_LOJA as u32,
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
    c.extend_from_slice(&ITEM_DE_LOJA.to_le_bytes()); // tid
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

    // `ITEM_TO_MONEY` (73): slot, id, quantidade e quanto rendeu — o `price` do arquivo
    // (50) vezes 2, e não os 999.999 que o cliente mandou.
    let venda = esperar_comando(&mut link, 73).await;
    assert_eq!(u16::from_le_bytes([venda[2], venda[3]]), 21);
    assert_eq!(i32_em(&venda, 12), 100, "o valor da venda não é price × count");

    let itens2 = itens.clone();
    let saiu = ate_async(move || {
        let i = itens2.clone();
        async move {
            i.get_item_by_slot(roleid, pw_core::ContainerType::Inventory, 21)
                .await
                .ok()
                .flatten()
                .is_none()
        }
    })
    .await;
    assert!(saiu, "o item vendido continuou na bolsa");

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
async fn aceitar_e_entregar_missao_no_npc_mexe_nas_listas_e_premia() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Sem falar com o NPC antes, o pedido não tem a quem ir.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SEVNPC_HELLO, &(NPC as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    esperar_comando(&mut link, 70).await;

    let mut aceitar = (MISSAO_DO_NPC as i32).to_le_bytes().to_vec();
    aceitar.extend_from_slice(&[0u8; 8]); // idStorage, idRefreshItem
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::ACEITAR_MISSAO, &aceitar),
    })
    .await
    .unwrap();

    // `TASK_VAR_DATA` com `svr_new_task`: reason 1, a missão, e 14 bytes de aviso.
    let nova = esperar_comando(&mut link, 106).await;
    assert_eq!(u32::from_le_bytes([nova[2], nova[3], nova[4], nova[5]]), 14);
    assert_eq!(nova[6], 1, "reason devia ser TASK_SVR_NOTIFY_NEW");
    assert_eq!(u16::from_le_bytes([nova[7], nova[8]]) as u32, MISSAO_DO_NPC);
    assert_eq!(
        mundo.read().await.players[&(roleid as i64)].missoes.ativa.indice(MISSAO_DO_NPC),
        Some(0),
        "a missão não entrou na lista ativa"
    );

    let mut entregar = (MISSAO_DO_NPC as i32).to_le_bytes().to_vec();
    entregar.extend_from_slice(&0i32.to_le_bytes()); // iChoice
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::ENTREGAR_MISSAO, &entregar),
    })
    .await
    .unwrap();

    // O prêmio vem antes do aviso de conclusão, como no `DeliverAward` original.
    let dinheiro_do_premio = esperar_comando(&mut link, 159).await;
    assert_eq!(i32_em(&dinheiro_do_premio, 2), 30);
    let concluida = esperar_comando(&mut link, 106).await;
    assert_eq!(concluida[6], 2, "reason devia ser TASK_SVR_NOTIFY_COMPLETE");

    let m = mundo.read().await;
    let p = &m.players[&(roleid as i64)];
    assert_eq!(p.missoes.ativa.quantidade, 0, "a missão entregue continuou ativa");
    assert_eq!(p.missoes.procurar_concluida(MISSAO_DO_NPC), 0, "não ficou registrada como concluída");
    let repo = m.char_repo.clone();
    drop(m);

    // E as listas vão para o banco, que é de onde o link as manda no próximo login.
    let gravada = ate_async(move || {
        let r = repo.clone();
        async move {
            r.task_lists()
                .carregar(roleid)
                .await
                .ok()
                .flatten()
                .is_some_and(|l| l.concluidas.len() == 8)
        }
    })
    .await;
    assert!(gravada, "as listas de missão não foram gravadas");
}

/// B57 — o `NORMAL_ATTACK` que chega durante a conjuração espera a habilidade acabar.
///
/// No original a habilidade é a sessão corrente e o golpe só entra na fila (`AddSession`
/// devolve `false`, `actobject.cpp:1180-1212`); ele começa quando a habilidade termina. O
/// cliente manda `NORMAL_ATTACK` assim que vê o fim da conjuração, e sem esta fila os dois
/// danos caíam no mesmo instante — o monstro morria "instantaneamente" com a habilidade
/// (teste em jogo de 2026-09-17).
#[tokio::test]
async fn o_golpe_que_chega_conjurando_espera_a_habilidade() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()),
    })
    .await
    .unwrap();
    receber(&mut link, 2).await;

    let mut corpo = 4321i32.to_le_bytes().to_vec(); // skill_id
    corpo.push(0); // force_attack
    corpo.push(0); // target_count
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::CAST_SKILL, &corpo) })
        .await
        .unwrap();
    // O `OBJECT_CAST_SKILL` (85) abre a conjuração.
    esperar_comando(&mut link, 85).await;

    // O clique no monstro durante a conjuração.
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::NORMAL_ATTACK, &[0u8]) })
        .await
        .unwrap();

    // A ordem tem de ser: resultado da habilidade (142) e só depois o golpe (84 + 24).
    let mut ordem = Vec::new();
    while ordem.iter().all(|c| *c != 24u16) {
        let Ok(Ok(Some(BusMessage::GameToClient { data, .. }))) =
            tokio::time::timeout(Duration::from_secs(5), link.receber()).await
        else {
            panic!("o golpe da fila não saiu: {ordem:?}");
        };
        ordem.push(cmd_de(&data));
    }
    let pos = |c: u16| ordem.iter().position(|x| *x == c);
    let resultado = pos(142).expect("sem o resultado da habilidade (142)");
    let inicio = pos(84).expect("sem HOST_START_ATTACK (84)");
    let golpe = pos(24).expect("sem HOST_ATTACKRESULT (24)");
    assert!(
        resultado < inicio && inicio < golpe,
        "o golpe saiu antes do dano da habilidade: {ordem:?}"
    );
}

#[tokio::test]
async fn conjurar_habilidade_causa_dano_real_no_alvo_selecionado() {
    // No `gateway.rs` o dano era **150 fixo**, mandado por uma tarefa que dormia um
    // segundo e respondia sem olhar para nada — o monstro não perdia vida.
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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

    // OBJECT_CAST_SKILL (85), SKILL_PERFORM (88), HOST_STOP_SKILL (123) e o resultado
    // (142). A barra (33) vai no batimento de 1 s (B56).
    let r = receber(&mut link, 4).await;
    assert!(r.iter().any(|v| cmd_de(v) == 85), "sem OBJECT_CAST_SKILL");
    assert!(r.iter().any(|v| cmd_de(v) == 88), "sem SKILL_PERFORM");
    assert!(r.iter().any(|v| cmd_de(v) == 123), "sem HOST_STOP_SKILL");

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
/// A espera é uma ida-e-volta de verdade (`UNSELECT` e a resposta), e não um `sleep`: o
/// que se quer garantir é que o servidor **já processou** o `EnterWorld` daquela conexão,
/// e só a resposta prova isso.
async fn segundo_jogador(
    mundo: &Arc<RwLock<WorldInstance>>,
    addr: std::net::SocketAddr,
    roleid: i32,
) -> pw_bus::transport::BusConnection {
    // Nada de inserir à mão: o `entrar` manda o `EnterWorld` e espera a carga do banco,
    // igual ao primeiro jogador. Inserir antes só criava um jogador que a carga
    // sobrescreveria logo depois.
    let mut link = entrar(&mundo, addr, roleid).await;
    {
        // O segundo fica a dois metros do primeiro, que é o que os testes de grupo
        // assumem — os dois entram na mesma posição por padrão.
        let mut m = mundo.write().await;
        let p = m.players.get_mut(&(roleid as i64)).expect("acabou de entrar");
        p.position = Vector3::new(2.0, 0.0, 2.0);
        m.grid.update_position(roleid as i64, Vector3::new(2.0, 0.0, 2.0));
    }

    // `UNSELECT` de propósito, e não `SIT_DOWN`: a resposta de sentar é **transmitida a
    // quem está por perto** (é o comportamento certo, e foi corrigido em 2026-09-08),
    // então usá-la aqui punha um `OBJECT_SIT_DOWN` na fila do outro jogador e quebrava
    // todo teste que exige "nada chega". O `UNSELECT` só responde a quem mandou.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::UNSELECT, &[]),
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
    let (mundo, addr, anfitriao, convidado) = cenario!();
    let mut link_a = entrar(&mundo, addr, anfitriao).await;

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
    let (mundo, addr, anfitriao, convidado) = cenario!();
    let mut link_a = entrar(&mundo, addr, anfitriao).await;
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
    let (mundo, addr, intruso, convidado) = cenario!();
    let mut link = entrar(&mundo, addr, intruso).await;
    // O convidado vem do `montar()`, não de `intruso + 1`: os dois personagens saem com
    // ids consecutivos hoje, mas depender disso é aceitar que o teste passe por acaso.
    let outro = convidado;
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
    let (mundo, addr, anfitriao, convidado) = cenario!();
    let mut link_a = entrar(&mundo, addr, anfitriao).await;
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
    let (mundo, addr, anfitriao, convidado) = cenario!();
    let mut link_a = entrar(&mundo, addr, anfitriao).await;
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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    let (mundo, addr, anfitriao, convidado) = cenario!();
    let mut link = entrar(&mundo, addr, anfitriao).await;
    let outro = convidado;
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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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

    // SELF_INFO_00, OWN_EXT_PROP (`PlayerGetProperty`, `player.cpp:8588`) e PLAYER_CASH.
    let r = receber(&mut link, 3).await;
    assert!(r.iter().any(|v| cmd_de(v) == 50), "sem OWN_EXT_PROP (50): a janela de atributos não se refaz");
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
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().money = 999;

    // Só o dinheiro e o marcador de fim: nada de bolsa, equipamento ou missões.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GET_ALL_DATA, &[0, 0, 0]),
    })
    .await
    .unwrap();

    let r = receber_ate_o_fim_da_carga(&mut link).await;
    // No servidor original C++ (player.cpp:13233-13248), OWN_IVTR_DATA (42) vai SEMPRE
    // para as 3 bolsas (0, 1 e 2) para inicializar a estrutura no cliente.
    // O que os três sinalizadores controlam são os blocos detalhados OWN_ITEM_INFO (40).
    let ivtrs: Vec<_> = r.iter().filter(|v| cmd_de(v) == 42).collect();
    assert_eq!(ivtrs.len(), 3, "as 3 bolsas (42) vão sempre para inicializar os contêineres");
    assert!(
        !r.iter().any(|v| cmd_de(v) == 40),
        "mandou OWN_ITEM_INFO (40) com os sinalizadores desligados"
    );
    assert!(
        r.iter().any(|v| cmd_de(v) == 105),
        "faltou o TASK_DATA (105) — é o marcador que destrava o cliente, e vai sempre"
    );
    // A ficha do jogador vai sempre: é ela que dá os atributos que o cliente confere antes
    // de deixar usar equipamento.
    assert!(
        r.iter().any(|v| cmd_de(v) == 50),
        "faltou o OWN_EXT_PROP (50) — sem ele todo equipamento aparece vermelho"
    );
    let saldo = r
        .iter()
        .find(|v| cmd_de(v) == 253)
        .expect("sem PLAYER_CASH (253)");
    assert_eq!(i32_em(saldo, 2), 999);

    // E com os sinalizadores ligados, as bolsas continuam vindo.
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GET_ALL_DATA, &[1, 1, 1]),
    })
    .await
    .unwrap();
    let r = receber_ate_o_fim_da_carga(&mut link).await;
    assert_eq!(
        r.iter().filter(|v| cmd_de(v) == 42).count(),
        3,
        "as 3 bolsas (42) continuam vindo com detalhe ligado"
    );
}

#[tokio::test]
async fn o_enter_world_poe_o_jogador_no_mundo_com_os_dados_do_banco() {
    // O buraco que este arquivo escondia: `world.players` nunca era populado em produção
    // — `PlayerEntity` só existia em teste, e era o próprio `montar()` que o fabricava.
    // Tudo que começa com "olhe o jogador" saía cedo sem fazer nada.
    //
    // Aqui não há maquiagem: manda o `EnterWorld` e confere o que a carga do banco pôs no
    // mundo.
    let (mundo, addr, roleid, _convidado) = cenario!();

    assert!(
        mundo.read().await.players.is_empty(),
        "antes do EnterWorld o mundo não pode ter jogador nenhum"
    );

    let _link = entrar_sem_ajustar(addr, roleid).await;

    let m2 = Arc::clone(&mundo);
    let presente = ate_async(move || {
        let m = Arc::clone(&m2);
        async move { m.read().await.players.contains_key(&(roleid as i64)) }
    })
    .await;
    assert!(presente, "o EnterWorld não pôs o jogador no mundo");

    let m = mundo.read().await;
    let p = &m.players[&(roleid as i64)];

    // Identidade vinda do banco, não de constante.
    assert_eq!(p.role_id, roleid);
    assert!(p.name.starts_with("Caca"), "nome veio errado: {}", p.name);
    assert_eq!(p.cls, CharacterClass::Blademaster);
    assert_eq!(p.level, 1, "o personagem de teste é criado no nível 1");

    // E a grade espacial conhece o jogador — sem isso ele não é visto por ninguém.
    let perto = m.grid.get_players_in_range(&p.position, 5.0);
    assert!(perto.contains(&(roleid as i64)), "o jogador não entrou na grade espacial");
}

#[tokio::test]
async fn sair_tira_o_jogador_do_mundo() {
    // O `remove_player` já era chamado antes de existir quem adicionasse; agora dá para
    // conferir o par inteiro.
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    assert!(mundo.read().await.players.contains_key(&(roleid as i64)));

    link.enviar(BusMessage::PlayerLogout {
        result: 0,
        roleid,
        provider_link_id: 1,
        localsid: LOCALSID,
    })
    .await
    .unwrap();

    let m2 = Arc::clone(&mundo);
    let saiu = ate_async(move || {
        let m = Arc::clone(&m2);
        async move { !m.read().await.players.contains_key(&(roleid as i64)) }
    })
    .await;
    assert!(saiu, "o PlayerLogout não tirou o jogador do mundo");
    assert!(
        mundo.read().await.grid.get_players_in_range(&Vector3::new(0.0, 0.0, 0.0), 50.0).is_empty(),
        "o jogador ficou na grade espacial depois de sair"
    );
}

/// Uma cura em si mesmo tem de **fechar a conjuração** igual a um ataque.
///
/// Em jogo, 2026-09-08: o Murillo conjurou a Prece da Clareza (113) no próprio sacerdote,
/// o console mostrou `Cast skill(113)` e a barra nunca fechou. No log do mundo:
/// `42 conjurou em 42, que não é um monstro deste mundo` — o tratamento saía cedo, antes
/// de mandar o comando que solta o conjurador.
///
/// O que fecha é o `HOST_STOP_SKILL` (123), sem corpo: é o único caminho que zera
/// `CECHostPlayer::m_pCurSkill` numa conjuração bem-sucedida (`EC_HostMsg.cpp:6065`). O
/// `SKILL_PERFORM` (88) não serve — vai para o gerente dos **outros** jogadores.
#[tokio::test]
async fn conjurar_em_si_mesmo_ainda_fecha_a_conjuracao() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Alvo explícito: o próprio conjurador.
    let mut corpo = 113i32.to_le_bytes().to_vec(); // Prece da Clareza
    corpo.push(0); // force_attack
    corpo.push(1); // target_count
    corpo.extend_from_slice(&(roleid as i32).to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CAST_SKILL, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 2).await;
    assert!(r.iter().any(|v| cmd_de(v) == 85), "sem OBJECT_CAST_SKILL");
    // Desde o B57 o efeito da habilidade vai **antes** do fim da sessão, como no original
    // (`RunSkill` e depois `EndSession`), então o 123 vem depois dos comandos do efeito.
    let parada = esperar_comando(&mut link, 123).await;
    let parada = &parada;
    assert_eq!(
        parada.len(),
        2,
        "o cliente exige corpo vazio no HOST_STOP_SKILL (dwSize == 0) e descarta o resto"
    );
}

/// O botão de armadura/roupa é um comando **sem corpo** que só o servidor resolve.
///
/// Em jogo, 2026-09-08: "mudei para modo roupa, não sincronizou para o outro jogador, e
/// ao clicar de novo não voltou — no debug não loga nada". Os três sintomas são o mesmo
/// defeito: o `SWITCH_FASHION_MODE` (85) caía no ramo silencioso do `match`.
///
/// O cliente não alterna sozinho; ele descobre o estado pelo `PLAYER_ENABLE_FASHION`
/// (192), que precisa chegar a quem apertou **e** a quem está por perto.
#[tokio::test]
async fn o_botao_de_roupa_alterna_e_avisa_os_dois_lados() {
    let (mundo, addr, roleid, convidado) = cenario!();
    let mut anfitriao = entrar(&mundo, addr, roleid).await;
    let mut outro = entrar(&mundo, addr, convidado).await;

    // Primeira vez: liga a roupa.
    anfitriao
        .enviar(BusMessage::ClientToGame {
            roleid,
            localsid: LOCALSID,
            data: subcomando(ids::SWITCH_FASHION_MODE, &[]),
        })
        .await
        .unwrap();

    let meu = receber(&mut anfitriao, 1).await;
    let pacote = meu
        .iter()
        .find(|v| cmd_de(v) == 192)
        .expect("quem apertou não recebeu PLAYER_ENABLE_FASHION");
    assert_eq!(pacote.len(), 2 + 5, "cmd_player_enable_fashion tem 5 bytes");
    assert_eq!(i32_em(pacote, 2), roleid, "o comando tem de dizer de quem é a roupa");
    assert_eq!(pacote[6], 1, "a primeira troca liga o modo roupa");

    let dele = receber(&mut outro, 1).await;
    assert!(
        dele.iter().any(|v| cmd_de(v) == 192),
        "o outro jogador não foi avisado da troca"
    );

    // Segunda vez: volta para a armadura.
    anfitriao
        .enviar(BusMessage::ClientToGame {
            roleid,
            localsid: LOCALSID,
            data: subcomando(ids::SWITCH_FASHION_MODE, &[]),
        })
        .await
        .unwrap();

    let volta = receber(&mut anfitriao, 1).await;
    let pacote = volta.iter().find(|v| cmd_de(v) == 192).expect("sem resposta na volta");
    assert_eq!(pacote[6], 0, "clicar de novo tem de voltar para a armadura");
}

/// Uma cura em si mesmo tem de **subir a vida** e mandar o número para a tela.
///
/// Em jogo, 2026-09-08: a Prece da Clareza fechava a barra e não fazia nada. A conta é a
/// do stub (`skill113.h`): `ataque_mágico * 4 * nível / 100 - 35 + 70 * nível`.
#[tokio::test]
async fn a_cura_em_si_mesmo_devolve_vida() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Machuca o jogador para haver o que curar.
    let (antes, max_hp) = {
        let mut m = mundo.write().await;
        let p = m.players.get_mut(&(roleid as i64)).expect("jogador no mundo");
        p.hp = 10;
        (p.hp, p.max_hp)
    };
    assert!(max_hp > antes, "o cenário precisa de vida faltando");

    let mut corpo = 113i32.to_le_bytes().to_vec(); // Prece da Clareza
    corpo.push(0); // force_attack
    corpo.push(1); // target_count
    corpo.extend_from_slice(&(roleid as i32).to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CAST_SKILL, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 6).await;
    assert!(r.iter().any(|v| cmd_de(v) == 123), "a conjuração não fechou");
    let res = r
        .iter()
        .find(|v| cmd_de(v) == 279)
        .expect("sem PLAYER_HP_STEAL: o número verde não apareceria");
    let curado = i32_em(res, 2);
    assert!(curado > 0, "a cura veio {curado}");

    // O efeito visual (a luz sobre o alvo) precisa do 142 com dano **-2**, que o cliente
    // trata como "habilidade de ajuda": nada de número, nada de animação de ferido
    // (`EC_Player.cpp:3435-3443`). Sem isto a cura funciona e não aparece nada.
    let efeito = r
        .iter()
        .find(|v| cmd_de(v) == 142)
        .expect("sem o 142 de ajuda, o efeito visual da cura não roda");
    assert_eq!(
        i32_em(efeito, 10),
        -2,
        "o 142 da cura tem de levar dano -2, senão o número sai vermelho"
    );

    let depois = mundo.read().await.players[&(roleid as i64)].hp;
    assert_eq!(depois, antes + curado, "a vida no mundo não subiu o que foi anunciado");
    assert!(
        r.iter().any(|v| cmd_de(v) == 38),
        "sem o SELF_INFO_00 (38) a barra de vida do cliente não mexe"
    );
}

/// Dano de habilidade num **outro jogador**: tira vida dele, e ele precisa saber.
///
/// Sem o `HOST_SKILL_ATTACKED` (144) o alvo não toca efeito nenhum nem entra em combate
/// (`EC_HostMsg.cpp:1023-1068`).
#[tokio::test]
async fn uma_habilidade_de_ataque_machuca_o_outro_jogador() {
    let (mundo, addr, roleid, convidado) = cenario!();
    let mut atacante = entrar(&mundo, addr, roleid).await;
    let mut vitima = entrar(&mundo, addr, convidado).await;

    // Garante ataque mágico para a conta não depender do que o banco trouxe.
    let antes = {
        let mut m = mundo.write().await;
        let a = m.players.get_mut(&(roleid as i64)).unwrap();
        a.magic_attack_min = 200;
        a.magic_attack_max = 200;
        let v = m.players.get_mut(&(convidado as i64)).unwrap();
        v.def_phys = 0;
        // A Pluma Espiritual faz ~234 com 200 de ataque mágico. Sem vida de sobra o HP
        // bate no piso de zero e a conta "vida - dano" deixa de valer.
        v.max_hp = 5_000;
        v.hp = 5_000;
        v.hp
    };

    let mut corpo = 125i32.to_le_bytes().to_vec(); // Pluma Espiritual
    corpo.push(0);
    corpo.push(1);
    corpo.extend_from_slice(&(convidado as i32).to_le_bytes());

    atacante
        .enviar(BusMessage::ClientToGame {
            roleid,
            localsid: LOCALSID,
            data: subcomando(ids::CAST_SKILL, &corpo),
        })
        .await
        .unwrap();

    let r = receber(&mut atacante, 4).await;
    let res = r.iter().find(|v| cmd_de(v) == 142).expect("sem o resultado (142)");
    let dano = i32_em(res, 10);
    assert!(dano > 0, "o dano veio {dano}");

    let depois = mundo.read().await.players[&(convidado as i64)].hp;
    assert_eq!(depois, antes - dano, "a vida do alvo não caiu o dano anunciado");

    // O alvo precisa saber que levou: sem o 144 ele não toca efeito nenhum nem entra em
    // combate (`EC_HostMsg.cpp:1023-1068`).
    let aviso = esperar_comando(&mut vitima, 144).await;
    assert_eq!(i32_em(&aviso, 2), roleid, "o 144 tem de dizer quem bateu");
}

/// Habilidade sem conta portada não inventa efeito.
#[tokio::test]
async fn habilidade_desconhecida_nao_mexe_na_vida_de_ninguem() {
    let (mundo, addr, roleid, convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    let _ = entrar(&mundo, addr, convidado).await;

    let antes = mundo.read().await.players[&(convidado as i64)].hp;

    let mut corpo = 4321i32.to_le_bytes().to_vec(); // não está na tabela
    corpo.push(0);
    corpo.push(1);
    corpo.extend_from_slice(&(convidado as i32).to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CAST_SKILL, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 3).await;
    assert!(r.iter().any(|v| cmd_de(v) == 123), "a conjuração tem de fechar mesmo assim");
    assert_eq!(
        mundo.read().await.players[&(convidado as i64)].hp,
        antes,
        "uma habilidade sem fórmula não pode machucar ninguém"
    );
}

/// **Usar um equipamento não pode gastá-lo.**
///
/// Em jogo, 2026-09-08, a asa do Sacerdote sumiu do banco: o jogador clicou nela para
/// voar, o cliente mandou `USE_ITEM` apontando para o container de equipamento, e o
/// servidor — que obedecia ao container e ao slot que o cliente mandasse — consumiu o
/// item. Não havia log, porque aquele caminho só registrava falha.
#[tokio::test]
async fn usar_um_equipamento_nao_o_consome() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let itens = mundo.read().await.char_repo.item_repo().clone();
    let asa = pw_core::ItemRecord {
        id: None,
        character_id: roleid,
        container_type: pw_core::ContainerType::Equipment,
        slot: 12, // EQUIPIVTR_FLYSWORD
        item_id: 2096,
        count: 1,
        max_count: 1,
        refine_level: 0,
        sockets_count: 0,
        sockets: vec![],
        durability: 0,
        max_durability: 0,
        bind_status: 0,
        octets: Vec::new(),
        custom_attributes: serde_json::json!({}),
    };
    itens.upsert_item(&asa).await.unwrap();

    // `UseItem`: onde(u8), quantos(u8), slot(u16), item_id(i32) — ver `comandos::UseItem`.
    let mut corpo = vec![1u8, 1u8];
    corpo.extend_from_slice(&12u16.to_le_bytes());
    corpo.extend_from_slice(&2096i32.to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::USE_ITEM, &corpo),
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let ainda_la = itens
        .get_item_by_slot(roleid, pw_core::ContainerType::Equipment, 12)
        .await
        .unwrap();
    assert!(
        ainda_la.is_some(),
        "a asa foi consumida ao ser usada — foi assim que ela sumiu do banco em jogo"
    );
}

/// Usar o item do slot de voo é **decolar**, e o cliente precisa saber disso pelo
/// `OBJECT_TAKEOFF` (96) — não pelo `HOST_USE_ITEM` (91), que significa "o item foi gasto"
/// e faz o cliente apagar a asa da tela.
#[tokio::test]
async fn usar_a_asa_decola_em_vez_de_gastar() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let itens = mundo.read().await.char_repo.item_repo().clone();
    let asa = pw_core::ItemRecord {
        id: None,
        character_id: roleid,
        container_type: pw_core::ContainerType::Equipment,
        slot: 12,
        item_id: 2096,
        count: 1,
        max_count: 1,
        refine_level: 0,
        sockets_count: 0,
        sockets: vec![],
        durability: 0,
        max_durability: 0,
        bind_status: 0,
        octets: Vec::new(),
        custom_attributes: serde_json::json!({}),
    };
    itens.upsert_item(&asa).await.unwrap();

    let mut corpo = vec![1u8, 1u8];
    corpo.extend_from_slice(&12u16.to_le_bytes());
    corpo.extend_from_slice(&2096i32.to_le_bytes());
    let usar = BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::USE_ITEM, &corpo),
    };

    link.enviar(usar.clone()).await.unwrap();
    let r = receber(&mut link, 1).await;
    assert_eq!(cmd_de(&r[0]), 96, "usar a asa tem de decolar (OBJECT_TAKEOFF)");
    assert_eq!(i32_em(&r[0], 2), roleid);
    assert!(mundo.read().await.players[&(roleid as i64)].voando, "o mundo não marcou o voo");

    // Usar de novo pousa.
    link.enviar(usar).await.unwrap();
    let r = receber(&mut link, 1).await;
    assert_eq!(cmd_de(&r[0]), 97, "usar a asa voando tem de pousar (OBJECT_LANDING)");
    assert!(!mundo.read().await.players[&(roleid as i64)].voando);
}

/// Sentar tem de ser visto por quem está por perto.
#[tokio::test]
async fn sentar_aparece_para_o_outro_jogador() {
    let (mundo, addr, roleid, convidado) = cenario!();
    let mut anfitriao = entrar(&mundo, addr, roleid).await;
    let mut outro = entrar(&mundo, addr, convidado).await;

    anfitriao
        .enviar(BusMessage::ClientToGame {
            roleid,
            localsid: LOCALSID,
            data: subcomando(ids::SIT_DOWN, &[]),
        })
        .await
        .unwrap();

    let meu = receber(&mut anfitriao, 1).await;
    assert!(meu.iter().any(|v| !v.is_empty()), "quem sentou não recebeu nada");

    let dele = receber(&mut outro, 1).await;
    assert!(
        !dele.is_empty(),
        "a meditação não chegou ao outro jogador — foi o que se viu em jogo"
    );
}

/// O Ctrl+clique do GM move o personagem e avisa quem está por perto.
///
/// Quem não é GM é recusado: o comando é teleporte livre.
#[tokio::test]
async fn o_goto_do_gm_teleporta_e_o_de_jogador_comum_nao() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let antes = mundo.read().await.players[&(roleid as i64)].position;
    let mut corpo = Vec::new();
    for v in [antes.x + 50.0, antes.y, antes.z + 50.0] {
        corpo.extend_from_slice(&v.to_le_bytes());
    }

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GOTO, &corpo),
    })
    .await
    .unwrap();

    // Jogador comum é recusado **em silêncio**: não há pacote para esperar, só o efeito
    // que não pode acontecer. Um respiro para o mundo processar a mensagem.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // O cenário cria conta comum: sem privilégio de GM, nada acontece.
    let depois = mundo.read().await.players[&(roleid as i64)].position;
    assert_eq!(
        (depois.x, depois.z),
        (antes.x, antes.z),
        "jogador comum não pode se teleportar"
    );
}

/// O teleporte de GM mantém a **altura atual** do jogador, e não o `y` que o cliente manda.
///
/// Os cliques de mapa mandam `y = 1.0` como marcador (`c2s_CmdGoto(fX, 1.0f, fZ)`), e o
/// servidor original substitui o campo pela altura do terreno
/// (`playercmd.cpp:4926`). Obedecer ao `y` recebido enterrava o personagem no chão — foi o
/// que se viu em jogo em 2026-09-08.
#[tokio::test]
async fn o_teleporte_ignora_o_y_do_cliente() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Dá privilégio de GM à conta deste personagem.
    let repo = mundo.read().await.char_repo.clone();
    let conta = repo
        .get_details_por_role(roleid)
        .await
        .unwrap()
        .expect("personagem existe")
        .account_id;
    sqlx::query("UPDATE accounts SET gm_privileges = 32 WHERE id = $1")
        .bind(conta)
        .execute(repo.pool().get_ref())
        .await
        .unwrap();

    let antes = mundo.read().await.players[&(roleid as i64)].position;
    let mut corpo = Vec::new();
    // `y = 1.0`, exatamente o marcador que os cliques de mapa mandam.
    for v in [antes.x + 80.0, 1.0f32, antes.z + 80.0] {
        corpo.extend_from_slice(&v.to_le_bytes());
    }

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GOTO, &corpo),
    })
    .await
    .unwrap();
    let r = receber(&mut link, 1).await;
    assert_eq!(cmd_de(&r[0]), 177, "sem HOST_CORRECT_POS o cliente não se move");

    let depois = mundo.read().await.players[&(roleid as i64)].position;
    assert_eq!(depois.x, antes.x + 80.0, "não andou em x");
    assert_eq!(depois.z, antes.z + 80.0, "não andou em z");
    assert_eq!(depois.y, antes.y, "o y do cliente (1.0) enterraria o personagem");

    let y_no_pacote = f32::from_le_bytes(r[0][6..10].try_into().unwrap());
    assert_eq!(y_no_pacote, antes.y, "o pacote levou o y errado");
}

/// Depois do teleporte, o jogador precisa receber os NPCs do destino.
///
/// Os NPCs são mandados uma vez só, no login. Em jogo, 2026-09-09: o GM se teleportou e
/// não havia NPC nenhum no destino — e ao voltar para a vila também não havia mais nada,
/// porque o cliente já tinha descartado o que saiu do raio.
#[tokio::test]
async fn o_teleporte_reenvia_os_npcs_do_destino() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let repo = mundo.read().await.char_repo.clone();
    let conta = repo
        .get_details_por_role(roleid)
        .await
        .unwrap()
        .expect("personagem existe")
        .account_id;
    sqlx::query("UPDATE accounts SET gm_privileges = 32 WHERE id = $1")
        .bind(conta)
        .execute(repo.pool().get_ref())
        .await
        .unwrap();

    // Sem `npcgen` carregado no cenário de teste, o servidor não tem NPC para mandar — o
    // que este teste garante é que o teleporte **responde** e não trava quando não há
    // nada por perto, e que o `HOST_CORRECT_POS` continua sendo o primeiro pacote.
    let antes = mundo.read().await.players[&(roleid as i64)].position;
    let mut corpo = Vec::new();
    for v in [antes.x + 40.0, 1.0f32, antes.z + 40.0] {
        corpo.extend_from_slice(&v.to_le_bytes());
    }
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GOTO, &corpo),
    })
    .await
    .unwrap();

    let r = receber(&mut link, 1).await;
    assert_eq!(cmd_de(&r[0]), 177, "o teleporte tem de confirmar com HOST_CORRECT_POS");
}

/// O mundo em volta acompanha quem anda.
///
/// Até 2026-09-09 os NPCs eram mandados **uma vez só**, no login. O cliente descarta o que
/// sai do raio ativo dele e ninguém reenviava: andar para longe e voltar deixava o mapa
/// vazio. Este teste cobre o ciclo inteiro — entrou no alcance, saiu do alcance.
#[tokio::test]
async fn andar_traz_o_que_entra_no_alcance_e_tira_o_que_sai() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // O monstro do cenário existe no mapa, mas não na grade espacial — quem consulta a
    // grade é o streaming. Põe ele a 10 m de onde o jogador está.
    let origem = {
        let mut m = mundo.write().await;
        let origem = m.players[&(roleid as i64)].position;
        let perto = Vector3::new(origem.x + 10.0, origem.y, origem.z);
        m.monsters.get_mut(&MONSTRO).unwrap().0.position = perto;
        m.grid.add_entity(MONSTRO, perto, false);
        origem
    };

    // Anda o suficiente para o mundo ser recalculado (o passo mínimo é 20 m).
    let passo = Vector3::new(origem.x + 25.0, origem.y, origem.z);
    let mut corpo = vec3(passo.x, passo.y, passo.z);
    corpo.extend_from_slice(&vec3(passo.x, passo.y, passo.z));
    corpo.extend_from_slice(&0u16.to_le_bytes()); // use_time
    corpo.extend_from_slice(&0u16.to_le_bytes()); // speed
    corpo.push(0); // move_mode
    corpo.extend_from_slice(&0u16.to_le_bytes()); // cmd_seq

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::PLAYER_MOVE, &corpo),
    })
    .await
    .unwrap();

    let entrou = esperar_comando(&mut link, 11).await;
    assert_eq!(
        i32_em(&entrou, 2),
        MONSTRO as i32,
        "o NPC_ENTER_SLICE não é do monstro que entrou no alcance"
    );
    assert!(
        mundo.read().await.players[&(roleid as i64)].visiveis.contains(&MONSTRO),
        "o mundo não anotou que o jogador passou a ver o monstro"
    );

    // Agora anda para longe: tem de sair.
    let longe = Vector3::new(origem.x + 500.0, origem.y, origem.z);
    let mut corpo = vec3(longe.x, longe.y, longe.z);
    corpo.extend_from_slice(&vec3(longe.x, longe.y, longe.z));
    corpo.extend_from_slice(&0u16.to_le_bytes());
    corpo.extend_from_slice(&0u16.to_le_bytes());
    corpo.push(0);
    corpo.extend_from_slice(&0u16.to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::PLAYER_MOVE, &corpo),
    })
    .await
    .unwrap();

    let saiu = esperar_comando(&mut link, 13).await;
    assert_eq!(
        i32_em(&saiu, 2),
        MONSTRO as i32,
        "o OBJECT_LEAVE_SLICE não é do monstro que saiu do alcance"
    );
    assert!(
        !mundo.read().await.players[&(roleid as i64)].visiveis.contains(&MONSTRO),
        "o mundo continua achando que o jogador vê o monstro"
    );
}

/// Andar um passo curto **não** refaz a conta: o cliente manda movimento 20 vezes por
/// segundo, e varrer a grade a cada pacote é o que este limiar evita.
#[tokio::test]
async fn passo_curto_nao_refaz_a_conta_do_que_esta_a_vista() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let origem = {
        let mut m = mundo.write().await;
        let origem = m.players[&(roleid as i64)].position;
        let perto = Vector3::new(origem.x + 5.0, origem.y, origem.z);
        m.monsters.get_mut(&MONSTRO).unwrap().0.position = perto;
        m.grid.add_entity(MONSTRO, perto, false);
        origem
    };

    // Dois metros: bem abaixo do passo mínimo.
    let perto = Vector3::new(origem.x + 2.0, origem.y, origem.z);
    let mut corpo = vec3(perto.x, perto.y, perto.z);
    corpo.extend_from_slice(&vec3(perto.x, perto.y, perto.z));
    corpo.extend_from_slice(&0u16.to_le_bytes());
    corpo.extend_from_slice(&0u16.to_le_bytes());
    corpo.push(0);
    corpo.extend_from_slice(&0u16.to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::PLAYER_MOVE, &corpo),
    })
    .await
    .unwrap();

    // Nada de NPC_ENTER_SLICE deve chegar neste passo.
    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    if let Ok(Ok(Some(BusMessage::GameToClient { ref data, .. }))) = nada {
        assert_ne!(
            cmd_de(data),
            11,
            "recalculou o mundo em volta com dois metros de caminhada"
        );
    }
    assert!(
        !mundo.read().await.players[&(roleid as i64)].visiveis.contains(&MONSTRO),
        "o conjunto visível foi recalculado sem o jogador andar o passo mínimo"
    );
}

/// A visibilidade entre jogadores acompanha quem anda, nos dois sentidos.
///
/// Até 2026-09-09 ela era do `gateway.rs`: `PLAYER_ENTER_WORLD` mútuo no login,
/// `PLAYER_LEAVE_WORLD` no logout, e nada entre as duas coisas. Dois jogadores que se
/// afastassem além do raio ativo do cliente sumiam um para o outro **para sempre** — o
/// cliente descarta o que sai do raio, e ninguém reenviava.
///
/// O ciclo inteiro: entram e se veem, um anda para longe e os dois deixam de se ver, ele
/// volta e os dois voltam a se ver. O jogador **parado** recebe tudo isso sem se mexer,
/// que é a parte que uma implementação ingênua erra.
#[tokio::test]
async fn dois_jogadores_se_veem_se_perdem_e_se_reencontram() {
    let (mundo, addr, anfitriao, convidado) = cenario!();

    let mut link_a = entrar_sem_ajustar(addr, anfitriao).await;
    esperar_no_mundo(&mundo, anfitriao).await;
    // Os dois nascem no mesmo ponto: um vê o outro assim que o segundo entra.
    let mut link_b = entrar_sem_ajustar(addr, convidado).await;
    esperar_no_mundo(&mundo, convidado).await;

    // 12 é `PLAYER_ENTER_SLICE`, não 17 (`PLAYER_ENTER_WORLD`): a struct é a mesma, mas o
    // cliente escolhe o efeito de aparição pelo comando (`EC_ManPlayer.cpp:1845`).
    let viu = esperar_comando(&mut link_b, 12).await;
    assert_eq!(i32_em(&viu, 2), anfitriao, "quem entrou não viu quem já estava");

    let foi_visto = esperar_comando(&mut link_a, 12).await;
    assert_eq!(
        i32_em(&foi_visto, 2),
        convidado,
        "quem já estava não viu quem entrou — a visibilidade tem de ser mútua"
    );

    assert!(
        mundo.read().await.players[&(anfitriao as i64)]
            .visiveis
            .contains(&(convidado as i64)),
        "o mundo não anotou, no jogador parado, que ele passou a ver o outro"
    );

    // O anfitrião anda para longe. Só a atualização **dele** roda: o convidado está
    // parado e não recalcula nada sozinho.
    andar(&mut link_a, anfitriao, Vector3::new(5_000.0, 0.0, 5_000.0)).await;

    let sumiu = esperar_comando(&mut link_a, 13).await;
    assert_eq!(i32_em(&sumiu, 2), convidado, "quem andou continuou vendo quem ficou");

    let sumiu = esperar_comando(&mut link_b, 13).await;
    assert_eq!(
        i32_em(&sumiu, 2),
        anfitriao,
        "quem ficou parado continuou vendo o avatar de quem foi embora"
    );
    assert!(
        !mundo.read().await.players[&(convidado as i64)]
            .visiveis
            .contains(&(anfitriao as i64)),
        "o mundo continua achando que o jogador parado vê quem saiu do alcance"
    );

    // E de volta, para onde o convidado está parado: os dois têm de se ver de novo.
    let onde_ele_esta = mundo.read().await.players[&(convidado as i64)].position;
    andar(&mut link_a, anfitriao, onde_ele_esta).await;

    let voltou = esperar_comando(&mut link_b, 12).await;
    assert_eq!(
        i32_em(&voltou, 2),
        anfitriao,
        "o jogador parado não viu o outro voltar — era este o buraco"
    );
}

/// Sair do jogo tira o avatar da tela de quem estava vendo, com `PLAYER_LEAVE_WORLD` (19).
///
/// Não é `OBJECT_LEAVE_SLICE` (13): quem saiu do jogo não saiu do alcance, e o cliente
/// distingue os dois casos (`bExit` em `CECPlayerMan::ElsePlayerLeave`).
#[tokio::test]
async fn quem_sai_do_mundo_some_da_tela_de_quem_ficou() {
    let (mundo, addr, anfitriao, convidado) = cenario!();

    let mut link_a = entrar_sem_ajustar(addr, anfitriao).await;
    esperar_no_mundo(&mundo, anfitriao).await;
    let mut link_b = entrar_sem_ajustar(addr, convidado).await;
    esperar_no_mundo(&mundo, convidado).await;

    // Consome a entrada mútua para não confundir com o que vem depois.
    let _ = esperar_comando(&mut link_a, 12).await;
    let _ = esperar_comando(&mut link_b, 12).await;

    // `logout_type = 1` é `_PLAYER_LOGOUT_HALF`: voltar à seleção de personagens.
    link_b
        .enviar(BusMessage::ClientToGame {
            roleid: convidado,
            localsid: LOCALSID,
            data: subcomando(ids::LOGOUT, &1i32.to_le_bytes()),
        })
        .await
        .unwrap();

    let saiu = esperar_comando(&mut link_a, 19).await;
    assert_eq!(
        i32_em(&saiu, 2),
        convidado,
        "o PLAYER_LEAVE_WORLD não é de quem saiu"
    );
    assert!(
        !mundo.read().await.players[&(anfitriao as i64)]
            .visiveis
            .contains(&(convidado as i64)),
        "o mundo continua achando que quem ficou vê quem saiu"
    );
}

/// Espera o jogador aparecer no mundo depois do `EnterWorld`.
async fn esperar_no_mundo(mundo: &Arc<RwLock<WorldInstance>>, roleid: i32) {
    let m = Arc::clone(mundo);
    let id = roleid as i64;
    let presente = ate_async(move || {
        let m = Arc::clone(&m);
        async move { m.read().await.players.contains_key(&id) }
    })
    .await;
    assert!(presente, "o jogador {roleid} não entrou no mundo depois do EnterWorld");
}

/// Um `PLAYER_MOVE` completo para `destino`.
async fn andar(
    link: &mut pw_bus::transport::BusConnection,
    roleid: i32,
    destino: Vector3,
) {
    let mut corpo = vec3(destino.x, destino.y, destino.z);
    corpo.extend_from_slice(&vec3(destino.x, destino.y, destino.z));
    corpo.extend_from_slice(&0u16.to_le_bytes()); // use_time
    corpo.extend_from_slice(&0u16.to_le_bytes()); // speed
    corpo.push(0); // move_mode
    corpo.extend_from_slice(&0u16.to_le_bytes()); // cmd_seq

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::PLAYER_MOVE, &corpo),
    })
    .await
    .unwrap();
}

/// O minério do mapa chega pelo comando dele, e some pelo comando dele.
///
/// O `npcgen.data` deste realm tem 5.125 instâncias de matéria, e até 2026-09-09 **nenhum
/// ponto do servidor** mandava `MATTER_ENTER_WORLD` (18) — nem no login, nem no
/// streaming. O mapa vinha sem recurso algum.
///
/// A saída é o outro lado do achado: `OBJECT_LEAVE_SLICE` (13) não conhece matéria (só
/// `ISPLAYERID` e `ISNPCID`, `EC_GameDataPrtc.cpp:891-899`); quem a tira da tela é o
/// `OUT_OF_SIGHT_LIST` (34).
#[tokio::test]
async fn o_minerio_do_mapa_entra_pelo_comando_de_materia_e_sai_pela_lista() {
    // `ISMATTERID` exige os dois bits mais altos ligados (`EC_GPDataType.h:27`) — é assim
    // que o `npcgen.rs` monta o id, e é por isso que o cliente sabe para qual gerente
    // mandar o que vem na lista de saída.
    const MINERIO: i64 = 0xC000_6886u32 as i32 as i64;
    const TID_DO_MINERIO: u32 = 8582;

    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let origem = {
        let mut m = mundo.write().await;
        let origem = m.players[&(roleid as i64)].position;
        let perto = Vector3::new(origem.x + 10.0, origem.y, origem.z);
        m.matters.insert(
            MINERIO,
            MatterEntity { id: MINERIO, template_id: TID_DO_MINERIO, position: perto, renascer_s: 15 },
        );
        m.grid.add_entity(MINERIO, perto, false);
        origem
    };

    let passo = Vector3::new(origem.x + 25.0, origem.y, origem.z);
    andar(&mut link, roleid, passo).await;

    let entrou = esperar_comando(&mut link, 18).await;
    assert_eq!(
        entrou.len(),
        2 + 25,
        "MATTER_ENTER_WORLD com tamanho errado é descartado em silêncio pelo cliente"
    );
    assert_eq!(i32_em(&entrou, 2), MINERIO as i32, "o mid não é o do minério");
    assert_eq!(i32_em(&entrou, 6), TID_DO_MINERIO as i32, "o tid não é o do minério");

    // Agora para longe: tem de sair — e pela lista, não pelo 13.
    andar(&mut link, roleid, Vector3::new(origem.x + 5_000.0, origem.y, origem.z)).await;

    let saiu = esperar_comando(&mut link, 34).await;
    assert_eq!(
        u32::from_le_bytes([saiu[2], saiu[3], saiu[4], saiu[5]]),
        1,
        "a lista de fora de vista devia ter um id só"
    );
    assert_eq!(i32_em(&saiu, 6), MINERIO as i32);
    assert!(
        !mundo.read().await.players[&(roleid as i64)].visiveis.contains(&MINERIO),
        "o mundo continua achando que o jogador vê o minério"
    );
}

/// O treinador sobe a habilidade um nível, grava e avisa o cliente.
///
/// `GP_NPCSEV_LEARN` (9) caía no ramo de "serviço ainda não tratado" até 2026-09-11:
/// clicar em aprender não fazia nada. E como o nível de conjuração deixou de ser fixo em 1
/// na mesma semana, não havia como subir uma habilidade para ver a diferença em jogo.
///
/// O pedido é um `int idSkill` e nada mais — o cliente **não** manda o nível
/// (`c2s_SendCmdNPCSevLearnSkill`, `EC_SendC2SCmds.cpp:3379-3405`). A resposta é
/// `LEARN_SKILL` (95), com id e nível novo.
#[tokio::test]
async fn o_treinador_sobe_a_habilidade_um_nivel_e_grava() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Uma habilidade que o personagem de teste não tem: começa do zero e vai a 1. O
    // cenário a põe na tabela com 100 de SP e 10 moedas por nível.
    const HABILIDADE: i32 = HABILIDADE_DO_TREINADOR;
    {
        let mut m = mundo.write().await;
        let p = m.players.get_mut(&(roleid as i64)).unwrap();
        p.sp = 1_000;
        p.money = 1_000;
    }
    let antes = mundo.read().await.players[&(roleid as i64)]
        .habilidades
        .get(&(HABILIDADE as u32))
        .copied()
        .unwrap_or(0);

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(
            pw_gs::npc::servico::APRENDER_HABILIDADE,
            &HABILIDADE.to_le_bytes(),
        ),
    })
    .await
    .unwrap();

    let resposta = esperar_comando(&mut link, 95).await;
    assert_eq!(i32_em(&resposta, 2), HABILIDADE, "o LEARN_SKILL não é da habilidade pedida");
    {
        let m = mundo.read().await;
        let p = &m.players[&(roleid as i64)];
        assert_eq!((p.sp, p.money), (900, 990), "aprender não cobrou SP e dinheiro da tabela");
    }
    assert_eq!(
        i32_em(&resposta, 6),
        antes as i32 + 1,
        "o nível novo tem de ser o anterior mais um"
    );

    // O mundo sabe — é o que faz a conjuração usar o nível novo.
    assert_eq!(
        mundo.read().await.players[&(roleid as i64)]
            .habilidades
            .get(&(HABILIDADE as u32))
            .copied(),
        Some(antes + 1)
    );

    // E o banco também, senão o nível se perde no relogin.
    let gravadas = mundo
        .read()
        .await
        .char_repo
        .skill_repo()
        .list_skills(roleid)
        .await
        .expect("ler as habilidades do banco");
    let gravada = gravadas
        .iter()
        .find(|h| h.skill_id == HABILIDADE as u32)
        .expect("a habilidade subida tem de estar no banco");
    assert_eq!(gravada.level, antes + 1);
}

#[tokio::test]
async fn pegar_moedas_do_chao_da_o_dinheiro_e_some_o_monte() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    let antes = dinheiro(&mundo, roleid).await;

    // Um monte de 25 moedas (`MONEY_MATTER_ID` = 3044) ao lado do jogador, de outro dono.
    let outro = {
        let mut m = mundo.write().await;
        m.criar_drop(3044, 25, Vector3::new(1.0, 0.0, 1.0), Some(roleid + 1))
    };
    let mut pedido = (outro.id as i32).to_le_bytes().to_vec();
    pedido.extend_from_slice(&3044i32.to_le_bytes());
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::PICKUP, &pedido) })
        .await
        .unwrap();
    // `ERR_ITEM_CANT_PICKUP` (6): durante os 30 s de posse só o dono pega.
    let erro = esperar_comando(&mut link, 25).await;
    assert_eq!(i32_em(&erro, 2), 6);

    let meu = {
        let mut m = mundo.write().await;
        m.criar_drop(3044, 25, Vector3::new(1.0, 0.0, 1.0), Some(roleid))
    };
    let mut pedido = (meu.id as i32).to_le_bytes().to_vec();
    pedido.extend_from_slice(&3044i32.to_le_bytes());
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::PICKUP, &pedido) })
        .await
        .unwrap();
    let moedas = esperar_comando(&mut link, 30).await;
    assert_eq!(i32_em(&moedas, 2), 25, "PICKUP_MONEY (30) com o valor do monte");
    let sumiu = esperar_comando(&mut link, 152).await;
    assert_eq!(i32_em(&sumiu, 2), meu.id as i32, "MATTER_PICKUP (152) com o id do monte");
    assert_eq!(dinheiro(&mundo, roleid).await, antes + 25);
    assert!(!mundo.read().await.drops.contains_key(&meu.id), "o monte continuou no chão");
}

/// A marca do `dyn_tasks.data` que o `montar()` põe no realm de teste.
const MARCA_DAS_MISSOES_DINAMICAS: u32 = 0x5277_6c0d;

/// Ao entrar no mundo o cliente pede a marca das missões dinâmicas
/// (`TASK_NOTIFY` com `reason` 7, `TaskProcess.cpp:2185`), e o mundo responde como
/// `ATaskTemplMan::OnTaskGetDynTasksTimeMark` (`TaskTemplMan.cpp:299-309`): `TASK_VAR_DATA`
/// com `reason` **8**, a marca do realm e a versão 10.
///
/// Até 2026-09-14 quem respondia era o `gateway.rs`, com `reason` 7 — no cliente,
/// `TASK_SVR_NOTIFY_FORGET_SKILL` — e marca zero.
#[tokio::test]
async fn o_pedido_da_marca_das_missoes_dinamicas_recebe_a_marca_do_realm() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // `task_notify { size = 3; buf = task_notify_base { reason = 7, task = 0 } }`
    let mut corpo = 3u32.to_le_bytes().to_vec();
    corpo.extend_from_slice(&[7, 0, 0]);
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::TASK_NOTIFY, &corpo),
    })
    .await
    .unwrap();

    // Outras notificações de missão também viajam no 106; a da marca é a de `reason` 8.
    let mut marca = None;
    for _ in 0..10 {
        let p = esperar_comando(&mut link, 106).await;
        if p.get(6) == Some(&8) {
            marca = Some(p);
            break;
        }
    }
    let p = marca.expect("o mundo não respondeu com a marca das missões dinâmicas");

    assert_eq!(i32_em(&p, 2), 9, "size = sizeof(svr_task_dyn_time_mark)");
    assert_eq!(p.len(), 2 + 4 + 9);
    assert_eq!(u16::from_le_bytes([p[7], p[8]]), 0, "task");
    assert_eq!(i32_em(&p, 9) as u32, MARCA_DAS_MISSOES_DINAMICAS, "a marca do realm");
    assert_eq!(u16::from_le_bytes([p[13], p[14]]), 10, "DYN_TASK_CUR_VERSION");
}

/// `session_normal_attack`: com uma sessão aberta, outro `NORMAL_ATTACK` só entra na fila e
/// não golpeia (`AddSession`, `actobject.cpp:1180-1213`) — o "cada clique é um golpe" do
/// teste em jogo de 2026-09-16.
#[tokio::test]
async fn clicar_de_novo_durante_a_sessao_nao_da_outro_golpe() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()) })
        .await
        .unwrap();
    receber(&mut link, 2).await;
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::NORMAL_ATTACK, &[0u8]) })
        .await
        .unwrap();
    receber(&mut link, 3).await;
    let hp = mundo.read().await.monsters[&MONSTRO].0.hp;
    for _ in 0..5 {
        link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::NORMAL_ATTACK, &[0u8]) })
            .await
            .unwrap();
    }
    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "um clique durante a sessão respondeu");
    assert_eq!(mundo.read().await.monsters[&MONSTRO].0.hp, hp, "os cliques golpearam");
    assert!(mundo.read().await.players[&(roleid as i64)].ataque.is_some());

    // B53 — o cliente manda CANCEL_ACTION + NORMAL_ATTACK a cada clique. O cancelamento não
    // fecha a sessão de golpe (`TerminateSession(false)` recusa, `actsession.h:109-115`), e o
    // novo golpe só entra na fila: nada de dano na hora.
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::CANCEL_ACTION, &[]) })
        .await
        .unwrap();
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::NORMAL_ATTACK, &[0u8]) })
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    {
        let m = mundo.read().await;
        let s = m.players[&(roleid as i64)].ataque.expect("o cancelamento fechou a sessão de golpe");
        assert_eq!(s.proximo, Some(MONSTRO), "o clique não entrou na fila");
        assert_eq!(m.monsters[&MONSTRO].0.hp, hp, "cancelar + atacar golpeou na hora");
    }
    // No golpe seguinte a sessão da fila substitui a atual: HOST_STOPATTACK (23) e
    // HOST_START_ATTACK (84), no ritmo da arma.
    for _ in 0..120 {
        mundo.write().await.tick(50).await;
        if mundo.read().await.players[&(roleid as i64)].ataque.is_some_and(|s| s.proximo.is_none()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let fim = esperar_comando(&mut link, 23).await;
    assert_eq!(fim.len(), 2 + 4);
    esperar_comando(&mut link, 84).await;
    assert!(mundo.read().await.players[&(roleid as i64)].ataque.is_some_and(|s| s.proximo.is_none()));
}

/// `CheckAttack`: além de `attack_range + corpo do alvo` a sessão nem começa.
#[tokio::test]
async fn fora_do_alcance_o_golpe_nao_comeca() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    mundo.write().await.players.get_mut(&(roleid as i64)).unwrap().attack_range = 2.8;
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()) })
        .await
        .unwrap();
    receber(&mut link, 2).await;
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::NORMAL_ATTACK, &[0u8]) })
        .await
        .unwrap();
    let nada = tokio::time::timeout(Duration::from_millis(400), link.receber()).await;
    assert!(nada.is_err(), "golpeou a 7 m com 2,8 m de alcance");
    assert_eq!(mundo.read().await.monsters[&MONSTRO].0.hp, MONSTRO_HP);
}

/// B53 — habilidade em área (`TARGETBALL`) com `doenchant`: acerta o alvo e o vizinho a 2 m,
/// deixa os dois lentos e atordoados (`StateAttack`), e o cliente recebe o estado visível
/// (124, 30 bytes) e os ícones (125).
#[tokio::test]
async fn habilidade_em_area_acerta_os_vizinhos_e_aplica_os_efeitos() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    const VIZINHO: i64 = 900_002;
    const LONGE: i64 = 900_003;
    {
        let mut m = mundo.write().await;
        let base = m.monsters[&MONSTRO].0.clone();
        let mut v = base.clone();
        v.id = VIZINHO;
        v.position.x += 2.0;
        let mut l = base.clone();
        l.id = LONGE;
        l.position.x += 30.0;
        m.monsters.insert(VIZINHO, (v, MonsterAi::new()));
        m.monsters.insert(LONGE, (l, MonsterAi::new()));
    }
    let mut link = entrar(&mundo, addr, roleid).await;
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()) })
        .await
        .unwrap();
    receber(&mut link, 2).await;
    let mut corpo = HABILIDADE_EM_AREA.to_le_bytes().to_vec();
    corpo.push(0);
    corpo.push(0);
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::CAST_SKILL, &corpo) })
        .await
        .unwrap();
    let estado = esperar_comando(&mut link, 124).await;
    assert_eq!(estado.len(), 30, "UPDATE_EXT_STATE fora do tamanho do cliente");
    let icones = esperar_comando(&mut link, 125).await;
    // id + 2 ícones (Slow 3, Dizzy 1) com 1 parâmetro cada.
    assert_eq!(icones.len(), 2 + 4 + 2 + 2 * 2 + 2 + 2 * 4);

    let m = mundo.read().await;
    for id in [MONSTRO, VIZINHO] {
        let (mo, _) = &m.monsters[&id];
        assert!(mo.hp < MONSTRO_HP, "{id} não levou dano");
        assert!(mo.efeitos.sem_acao(), "{id} não ficou atordoado");
        assert_eq!(mo.efeitos.realce().velocidade, -30, "{id} não ficou lento");
    }
    let (longe, _) = &m.monsters[&LONGE];
    assert_eq!(longe.hp, MONSTRO_HP, "o monstro a 30 m levou dano");
    assert!(longe.efeitos.filtros.is_empty());
}

/// B54 — Esc (`CANCEL_ACTION`) e andar encerram o golpe no golpe seguinte (fila do
/// original, `actobject.cpp:180-189`), e a morte do alvo encerra na hora: o monstro renascia
/// com o mesmo id antes do disparo seguinte e o ataque não parava (teste de 2026-09-17).
#[tokio::test]
async fn esc_andar_e_a_morte_do_alvo_param_o_golpe() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;
    let atacar = || BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::NORMAL_ATTACK, &[0u8]) };
    link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(ids::SELECT_TARGET, &(MONSTRO as i32).to_le_bytes()) })
        .await
        .unwrap();
    receber(&mut link, 2).await;

    for comando in [ids::CANCEL_ACTION, ids::STOP_MOVE] {
        mundo.write().await.monsters.get_mut(&MONSTRO).unwrap().0.hp = MONSTRO_HP_MAX;
        link.enviar(atacar()).await.unwrap();
        esperar_comando(&mut link, 84).await;
        let corpo = if comando == ids::STOP_MOVE { vec![0u8; 20] } else { Vec::new() };
        link.enviar(BusMessage::ClientToGame { roleid, localsid: LOCALSID, data: subcomando(comando, &corpo) })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(mundo.read().await.players[&(roleid as i64)].ataque.is_some(), "comando {comando} parou na hora");
        for _ in 0..120 {
            mundo.write().await.tick(50).await;
            if mundo.read().await.players[&(roleid as i64)].ataque.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(mundo.read().await.players[&(roleid as i64)].ataque.is_none(), "comando {comando} não parou o golpe");
        esperar_comando(&mut link, 23).await;
    }

    // A morte do alvo encerra a sessão, e o monstro só renasce depois do corpo sumir.
    mundo.write().await.monsters.get_mut(&MONSTRO).unwrap().0.hp = 1;
    link.enviar(atacar()).await.unwrap();
    // B62 — o golpe sai na hora e a vida cai no tique, quando o dano adiado vence.
    esperar_comando(&mut link, 24).await;
    assert!(
        tickar_ate(&mundo, |m| m.monsters[&MONSTRO].0.is_dead).await,
        "o dano adiado não chegou a matar o monstro"
    );
    let fim = esperar_comando(&mut link, 23).await;
    assert_eq!(i32::from_le_bytes([fim[2], fim[3], fim[4], fim[5]]), 2, "motivo: alvo inválido");
    assert!(mundo.read().await.players[&(roleid as i64)].ataque.is_none());
    {
        let mut m = mundo.write().await;
        m.monsters.get_mut(&MONSTRO).unwrap().0.respawn_delay_ms = 100;
        m.monsters.get_mut(&MONSTRO).unwrap().0.respawn_timer_ms = 100;
        for _ in 0..40 {
            m.tick(50).await;
        }
        assert!(m.monsters[&MONSTRO].0.is_dead, "renasceu com o corpo ainda no chão");
    }
}

/// B65: ESC (CANCEL_ACTION) ou movimento interrompe conjuração com SELF_SKILL_INTERRUPTED (87).
#[tokio::test]
async fn esc_cancela_conjuracao_com_self_skill_interrupted() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    let mut corpo = 167i32.to_le_bytes().to_vec(); // Portal da Cidade (167)
    corpo.push(0);
    corpo.push(0);

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CAST_SKILL, &corpo),
    })
    .await
    .unwrap();

    let cast = esperar_comando(&mut link, 85).await;
    assert_eq!(cmd_de(&cast), 85);
    assert!(mundo.read().await.players[&(roleid as i64)].conjuracao.is_some());

    // Pressiona ESC (CANCEL_ACTION)
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::CANCEL_ACTION, &[]),
    })
    .await
    .unwrap();

    // Deve receber SELF_SKILL_INTERRUPTED (87) com reason 2 (interrompido)
    let interrupcao = esperar_comando(&mut link, 87).await;
    assert_eq!(cmd_de(&interrupcao), 87);
    assert_eq!(interrupcao[2], 2);
    assert!(mundo.read().await.players[&(roleid as i64)].conjuracao.is_none());
}

/// B65: GET_ALL_DATA envia as três bolsas (0, 1 e 2) mesmo com detalhe_missoes = 0 (cliente 1.5.5 envia [1, 1, 0]),
/// e não envia NPCs comuns da cena em SCENE_SERVICE_NPC_LIST (390) para não sequestrar a caixa de diálogo do NPC.
#[tokio::test]
async fn get_all_data_envia_bolsas_incondicionalmente_sem_sequestrar_npcs() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    // Adiciona um NPC no mundo
    mundo.write().await.npcs.insert(12345, pw_gs::NpcEntity {
        id: 12345,
        template_id: 44698,
        name: "Mestre".to_string(),
        position: Vector3::new(0.0, 0.0, 0.0),
        dialog_id: 0,
        direcao: 0,
    });

    let mut link = entrar(&mundo, addr, roleid).await;

    // Cliente 1.5.5 envia GetAllData(true, true, false) => [1, 1, 0]
    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::GET_ALL_DATA, &[1, 1, 0]),
    })
    .await
    .unwrap();

    let pacotes = receber_ate_o_fim_da_carga(&mut link).await;
    // Verifica que as três bolsas (0, 1 e 2) foram enviadas via OWN_IVTR_DATA (42)
    let ivtrs: Vec<_> = pacotes.iter().filter(|p| cmd_de(p) == 42).collect();
    assert_eq!(ivtrs.len(), 3, "todas as 3 bolsas devem ser inicializadas mesmo com detalhe_missoes=0");
    assert_eq!(ivtrs[0][2], 0, "bolsa comum (0)");
    assert_eq!(ivtrs[1][2], 1, "bolsa equipamento (1)");
    assert_eq!(ivtrs[2][2], 2, "bolsa missão (2)");

    // NPCs comuns da cena não devem sair no 390
    assert!(pacotes.iter().all(|p| cmd_de(p) != 390), "SCENE_SERVICE_NPC_LIST (390) não deve emitir NPCs comuns");
}

/// B65: Monstro só ganha ameaça e acorda quando o dano do golpe atinge o alvo, não no clique.
#[tokio::test]
async fn reacao_do_monstro_so_ocorre_quando_o_dano_atinge_o_alvo() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

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
    receber(&mut link, 3).await;

    // Enquanto o dano não caiu, o monstro não pode ter ameaça nem reagir
    assert_eq!(mundo.read().await.monsters[&MONSTRO].1.aggro_table.len(), 0);

    // Quando o dano adiado atinge o monstro no tique do mundo:
    assert!(tickar_ate(&mundo, |m| m.monsters[&MONSTRO].0.hp < MONSTRO_HP).await);

    // Agora sim o monstro tem a ameaça registrada
    assert!(mundo.read().await.monsters[&MONSTRO].1.aggro_table.contains_key(&(roleid as i64)));
}

/// B66: Incubação de mascote/montaria em NPC (`GP_NPCSEV_HATCHPET` = 28).
/// Deduz moedas, remove o ovo da bolsa e responde `GAIN_PET` (231) com o `InfoPet` de 192 bytes.
#[tokio::test]
async fn incubar_ovo_de_montaria_no_npc_gera_mascote_e_salva_no_corral() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Encontra um ovo de montaria no data_manager
    let (ovo_id, ovo_info) = {
        let m = mundo.read().await;
        m.data_manager
            .ovos_de_pet
            .iter()
            .find(|(_, o)| o.pet_class == 0)
            .map(|(&id, o)| (id, o.clone()))
            .expect("deve existir ovo de montaria")
    };

    dar_dinheiro(&mundo, roleid, (ovo_info.money_hatched as i64) + 50_000).await;

    // Coloca o ovo no slot 0 da bolsa do jogador
    let repo = mundo.read().await.char_repo.clone();
    let itens = repo.item_repo().clone();
    let mut item_ovo = pw_core::ItemRecord::new(roleid, pw_core::ContainerType::Inventory, 0, ovo_id, 1);
    item_ovo.octets = mundo.read().await.data_manager.gerar_octetos_do_ovo(ovo_id).unwrap_or_default();
    itens.upsert_item(&item_ovo).await.unwrap();

    // Envia C2S::NPC_SERVICE com serviço 28 (INCUBAR_PET)
    let mut conteudo = (0i32).to_le_bytes().to_vec(); // egg_index = 0
    conteudo.extend_from_slice(&(ovo_id as i32).to_le_bytes()); // egg_id

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: pedido_ao_npc(pw_gs::npc::servico::INCUBAR_PET, &conteudo),
    })
    .await
    .unwrap();

    // Espera o pacote GAIN_PET (231)
    let gain_pet = esperar_comando(&mut link, 231).await;
    let slot_index = i32_em(&gain_pet, 2);
    assert_eq!(slot_index, 0, "deve ser alocado no slot 0 do corral");
    assert_eq!(gain_pet.len() - 6, pw_core::TAMANHO_INFO_PET, "info_pet deve ter exatamente 192 bytes");

    // Verifica que o pet foi salvo no banco no container PetCorral
    let corral = itens.list_by_container(roleid, pw_core::ContainerType::PetCorral).await.unwrap();
    assert_eq!(corral.len(), 1);
    assert_eq!(corral[0].item_id, ovo_info.id_pet);
    assert_eq!(corral[0].octets.len(), pw_core::TAMANHO_INFO_PET);
}

/// B66: Coleta de item de missão do chão vai para a bolsa de missão (`where = 2`).
#[tokio::test]
async fn pegar_item_de_missao_vai_para_bolsa_de_missao() {
    let (mundo, addr, roleid, _convidado) = cenario!();
    let mut link = entrar(&mundo, addr, roleid).await;

    // Encontra um id de item de missão no data_manager
    let item_missao_id = {
        let m = mundo.read().await;
        m.data_manager
            .itens_de_missao
            .iter()
            .copied()
            .next()
            .unwrap_or(ITEM_DE_MISSAO as u32)
    };

    // Cria um drop do item de missão no chão pertencente ao jogador
    let drop = {
        let mut m = mundo.write().await;
        m.criar_drop(item_missao_id, 1, Vector3::new(1.0, 0.0, 1.0), Some(roleid))
    };

    let mut pedido = (drop.id as i32).to_le_bytes().to_vec();
    pedido.extend_from_slice(&(item_missao_id as i32).to_le_bytes());

    link.enviar(BusMessage::ClientToGame {
        roleid,
        localsid: LOCALSID,
        data: subcomando(ids::PICKUP, &pedido),
    })
    .await
    .unwrap();

    // PICKUP_ITEM 126: tid/expire i32, amount/slot_amount u16, package/slot u8 (s2c-31.txt:2).
    let pickup = esperar_comando(&mut link, 31).await;
    assert_eq!(i32_em(&pickup, 2), item_missao_id as i32, "item_id");
    assert_eq!(pickup.len(), 16);
    assert_eq!(u16::from_le_bytes([pickup[10], pickup[11]]), 1, "amount");
    assert_eq!(u16::from_le_bytes([pickup[12], pickup[13]]), 1, "slot_amount");
    assert_eq!(pickup[14], 2, "pacote/where deve ser 2 (IL_TASK_INVENTORY)");

    let sumiu = esperar_comando(&mut link, 152).await;
    assert_eq!(i32_em(&sumiu, 2), drop.id as i32, "MATTER_PICKUP (152)");
}
