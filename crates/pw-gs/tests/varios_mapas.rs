//! Vários mapas num processo: o roteador entrega cada jogador ao mapa gravado dele.
//!
//! O cenário é o do realm 155 depois do B48 — um personagem antigo no mundo 1 e um novo
//! no 161 —, só que os dois mapas no mesmo processo e atrás de uma única escuta de
//! barramento, como o `./gs gs01 ... is61` do original (`pw_gs::mapas`).
//!
//! Sem `TEST_DATABASE_URL` os testes passam sem verificar nada e dizem isso na saída.

use pw_bus::{BusClient, BusListener, BusMessage};
use pw_core::{CharacterClass, Gender, Race};
use pw_data_loader::GameDataManager;
use pw_gs::comandos::ids;
use pw_gs::{RoteadorDeMapas, WorldInstance};
use pw_protocol::GameVersion;
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use std::time::Duration;

#[path = "../../pw-storage/tests/comum/mod.rs"]
mod comum;

const LOCALSID: u32 = 77;

struct Cenario {
    roteador: Arc<RoteadorDeMapas>,
    mundo_1: Arc<tokio::sync::RwLock<WorldInstance>>,
    mundo_161: Arc<tokio::sync::RwLock<WorldInstance>>,
    addr: std::net::SocketAddr,
    no_1: i32,
    no_161: i32,
}

async fn montar() -> Option<Cenario> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return None;
        }
    };
    let cfg = StorageConfig { database_url: url, max_connections: 3, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");
    comum::limpar_sobras_de_teste(&pool).await;

    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let m = format!(
        "{}_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000,
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let realm = format!("t_gs_{m}");
    sqlx::query(
        "INSERT INTO realms (id, name, version, host, port, max_players, config)
         VALUES ($1, 'Teste Mapas', '1.5.5', '127.0.0.1', 29000, 10, '{}'::jsonb)",
    )
    .bind(&realm)
    .execute(pool.get_ref())
    .await
    .expect("criar realm");
    let conta: i32 =
        sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
            .bind(format!("gs_{m}"))
            .fetch_one(pool.get_ref())
            .await
            .expect("criar conta");

    let repo = CharacterRepository::new(pool.clone());
    let mut ids = Vec::new();
    for nome in ["Mapa1", "Mapa161"] {
        let id = repo
            .create_character(conta, &realm, &format!("{nome}{m}"), Race::Human, CharacterClass::Blademaster, Gender::Male, Vec::new(), None)
            .await
            .expect("criar personagem");
        ids.push(id);
    }
    // Todo personagem do realm de teste nasce no mundo 1; o segundo passa para o 161.
    for (id, mapa) in [(ids[0], 1), (ids[1], 161)] {
        sqlx::query("UPDATE characters SET world_id = $2 WHERE id = $1")
            .bind(id)
            .bind(mapa)
            .execute(pool.get_ref())
            .await
            .expect("pôr o personagem no mapa");
    }

    let dados = Arc::new(GameDataManager::new());
    let mut servidos = Vec::new();
    for mapa in [1, 161] {
        let mundo = WorldInstance::new(mapa, Arc::clone(&dados), repo.clone());
        servidos.push(RoteadorDeMapas::preparar_mapa(mundo, GameVersion::V1_5_5).await);
    }
    let mundo_1 = Arc::clone(servidos[0].1.mundo());
    let mundo_161 = Arc::clone(servidos[1].1.mundo());
    let roteador = Arc::new(RoteadorDeMapas::new(servidos, repo));

    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();
    tokio::spawn(Arc::clone(&roteador).executar(escuta));

    Some(Cenario { roteador, mundo_1, mundo_161, addr, no_1: ids[0], no_161: ids[1] })
}

async fn ate<F, Fut>(mut cond: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..300 {
        if cond().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

fn entrar(roleid: i32) -> BusMessage {
    BusMessage::EnterWorld { roleid, provider_link_id: 1, locktime: 0, timeout: 60, settime: 0, localsid: LOCALSID }
}

#[tokio::test]
async fn cada_jogador_entra_no_mapa_gravado_pela_mesma_conexao() {
    let Some(c) = montar().await else { return };
    let mut link = BusClient::conectar(c.addr).await.unwrap();
    link.enviar(entrar(c.no_1)).await.unwrap();
    link.enviar(entrar(c.no_161)).await.unwrap();

    let (m1, m161, a, b) = (Arc::clone(&c.mundo_1), Arc::clone(&c.mundo_161), c.no_1 as i64, c.no_161 as i64);
    assert!(
        ate(|| {
            let (m1, m161) = (Arc::clone(&m1), Arc::clone(&m161));
            async move {
                m1.read().await.players.contains_key(&a) && m161.read().await.players.contains_key(&b)
            }
        })
        .await,
        "os dois jogadores não chegaram cada um ao seu mapa"
    );
    assert!(!c.mundo_1.read().await.players.contains_key(&b), "o do 161 também entrou no mundo 1");
    assert!(!c.mundo_161.read().await.players.contains_key(&a), "o do mundo 1 também entrou no 161");
    assert_eq!(c.roteador.mapa_de(c.no_161).await, Some(161));
}

#[tokio::test]
async fn subcomando_e_saida_vao_ao_mapa_do_jogador() {
    let Some(c) = montar().await else { return };
    let mut link = BusClient::conectar(c.addr).await.unwrap();
    link.enviar(entrar(c.no_161)).await.unwrap();

    let (m161, b) = (Arc::clone(&c.mundo_161), c.no_161 as i64);
    assert!(
        ate(|| {
            let m = Arc::clone(&m161);
            async move { m.read().await.players.contains_key(&b) }
        })
        .await,
        "o jogador não entrou no 161"
    );

    // `PLAYER_MOVE`: cur_pos, next_pos, use_time, speed, move_mode, cmd_seq.
    let mut data = ids::PLAYER_MOVE.to_le_bytes().to_vec();
    for v in [123.0f32, 45.0, -67.0, 124.0, 45.0, -67.0] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(&100u16.to_le_bytes());
    data.extend_from_slice(&48u16.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&1u16.to_le_bytes());
    link.enviar(BusMessage::ClientToGame { roleid: c.no_161, localsid: LOCALSID, data }).await.unwrap();

    let m = Arc::clone(&m161);
    assert!(
        ate(|| {
            let m = Arc::clone(&m);
            async move { m.read().await.players.get(&b).is_some_and(|p| (p.position.x - 123.0).abs() < 0.01) }
        })
        .await,
        "o movimento não chegou ao mapa 161"
    );

    link.enviar(BusMessage::PlayerLogout { result: 0, roleid: c.no_161, provider_link_id: 1, localsid: LOCALSID })
        .await
        .unwrap();
    let m = Arc::clone(&m161);
    assert!(
        ate(|| {
            let m = Arc::clone(&m);
            async move { !m.read().await.players.contains_key(&b) }
        })
        .await,
        "a saída não tirou o jogador do mapa 161"
    );
    assert!(
        ate(|| {
            let r = Arc::clone(&c.roteador);
            let id = c.no_161;
            async move { r.mapa_de(id).await.is_none() }
        })
        .await,
        "o roteador continuou achando que o jogador está num mapa"
    );
}

/// Teleporte para outro mapa do mesmo processo (`LongJump` → `PlaneSwitch`): o jogador sai
/// do 161, entra no mundo 1, o roteador passa a entregá-lo ao novo mapa, o banco grava o
/// mapa, e o cliente recebe `NOTIFY_HOSTPOS` com o `tag` novo — é isso que faz ele carregar o
/// outro mundo (`JumpToInstance`).
#[tokio::test]
async fn teleporte_para_outro_mapa_passa_o_jogador_e_avisa_o_cliente() {
    let Some(c) = montar().await else { return };
    c.roteador.ligar_trocas();
    let mut link = BusClient::conectar(c.addr).await.unwrap();
    link.enviar(entrar(c.no_161)).await.unwrap();
    let (m1, m161, b) = (Arc::clone(&c.mundo_1), Arc::clone(&c.mundo_161), c.no_161 as i64);
    assert!(
        ate(|| {
            let m = Arc::clone(&m161);
            async move { m.read().await.players.contains_key(&b) }
        })
        .await,
        "o jogador não entrou no 161"
    );
    // Descarta o que veio na entrada.
    while tokio::time::timeout(Duration::from_millis(200), link.receber()).await.is_ok() {}

    let destino = pw_core::Vector3::new(-319.667, 220.007, -900.309);
    c.roteador.transportar(c.no_161, 1, destino).await;

    assert!(
        ate(|| {
            let (m1, m161) = (Arc::clone(&m1), Arc::clone(&m161));
            async move { m1.read().await.players.contains_key(&b) && !m161.read().await.players.contains_key(&b) }
        })
        .await,
        "o jogador não passou do 161 para o mundo 1"
    );
    assert_eq!(c.roteador.mapa_de(c.no_161).await, Some(1));
    assert_eq!(c.mundo_1.read().await.players[&b].position.x, destino.x);

    let mut aviso = None;
    for _ in 0..50 {
        match tokio::time::timeout(Duration::from_millis(500), link.receber()).await {
            Ok(Ok(Some(BusMessage::GameToClient { data, .. }))) if u16::from_le_bytes([data[0], data[1]]) == 14 => {
                aviso = Some(data);
                break;
            }
            Ok(Ok(Some(_))) => {}
            _ => break,
        }
    }
    let aviso = aviso.expect("sem NOTIFY_HOSTPOS (14)");
    assert_eq!(aviso.len(), 2 + 20, "cmd_notify_hostpos: vPos, tag, line");
    assert_eq!(i32::from_le_bytes(aviso[14..18].try_into().unwrap()), 1, "tag do mapa novo");

    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PostgresPool::new(&StorageConfig { database_url: url, max_connections: 1, min_connections: 1, ..Default::default() }).await.unwrap();
    let mapa: i32 = sqlx::query_scalar("SELECT world_id FROM characters WHERE id = $1").bind(c.no_161).fetch_one(pool.get_ref()).await.unwrap();
    assert_eq!(mapa, 1, "o mapa novo não foi gravado — relogar voltaria ao 161");
}
