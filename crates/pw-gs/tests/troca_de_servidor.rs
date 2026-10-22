//! B350 — troca de mapa entre servidores de mundo (contêineres): o de origem grava e avisa o link com `SwitchServerStart`;
//! o de destino recebe o mesmo pedido, põe o jogador no mapa novo e responde `SwitchServerSuccess`.
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
async fn proxima(link: &mut pw_bus::transport::BusConnection, quer: impl Fn(&BusMessage) -> bool) -> BusMessage {
    for i in 0..500 {
        let m = match tokio::time::timeout(Duration::from_secs(5), link.receber()).await {
            Ok(m) => m.unwrap().expect("conexão fechou"),
            Err(_) => panic!("nada chegou depois de {i} mensagens"),
        };
        if quer(&m) {
            return m;
        }
    }
    panic!("a mensagem esperada não veio");
}

#[tokio::test]
async fn o_jogador_troca_de_servidor_de_mundo() {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return;
        }
    };
    let cfg = StorageConfig { database_url: url, max_connections: 3, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");
    comum::limpar_sobras_de_teste(&pool).await;
    let m = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;
    let realm = format!("t_gs_{m}");
    sqlx::query("INSERT INTO realms (id, name, version, host, port, max_players, config) VALUES ($1, 'Teste Troca', '1.5.5', '127.0.0.1', 29000, 10, '{}'::jsonb)")
        .bind(&realm).execute(pool.get_ref()).await.expect("criar realm");
    let conta: i32 = sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
        .bind(format!("gs_{m}")).fetch_one(pool.get_ref()).await.expect("criar conta");
    let repo = CharacterRepository::new(pool.clone());
    let id = repo
        .create_character(conta, &realm, &format!("Viajante{m}"), Race::Human, CharacterClass::Blademaster, Gender::Male, Vec::new(), None)
        .await
        .expect("criar personagem");
    sqlx::query("UPDATE characters SET world_id = 1 WHERE id = $1").bind(id).execute(pool.get_ref()).await.unwrap();

    // Dois servidores de mundo: A com o mapa 1 (o 161 é remoto), B com o 161.
    let dados = Arc::new(GameDataManager::new());
    let mut servidores = Vec::new();
    for (mapa, remotos) in [(1, vec![161]), (161, vec![1])] {
        let mundo = WorldInstance::new(mapa, Arc::clone(&dados), repo.clone());
        let servido = RoteadorDeMapas::preparar_mapa(mundo, GameVersion::V1_5_5).await;
        let w = Arc::clone(servido.1.mundo());
        let roteador = Arc::new(RoteadorDeMapas::new(vec![servido], repo.clone()));
        roteador.definir_mapas_remotos(remotos);
        roteador.ligar_trocas();
        let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
        let addr = escuta.local_addr().unwrap();
        tokio::spawn(Arc::clone(&roteador).executar(escuta));
        servidores.push((roteador, w, addr));
    }
    let (ra, wa, addr_a) = (Arc::clone(&servidores[0].0), Arc::clone(&servidores[0].1), servidores[0].2);
    let (wb, addr_b) = (Arc::clone(&servidores[1].1), servidores[1].2);

    // O "link" entra no A.
    let mut link_a = BusClient::conectar(addr_a).await.unwrap();
    link_a.enviar(BusMessage::EnterWorld { roleid: id, provider_link_id: 1, locktime: 0, timeout: 60, settime: 0, localsid: LOCALSID }).await.unwrap();
    for _ in 0..300 {
        if wa.read().await.players.contains_key(&(id as i64)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(wa.read().await.players.contains_key(&(id as i64)));

    // Vai ao 161: o A grava, larga e avisa o link.
    let destino = pw_core::Vector3::new(10.0, 5.0, 20.0);
    ra.transportar(id, 161, destino).await;
    let pedido = proxima(&mut link_a, |m| matches!(m, BusMessage::SwitchServerStart { .. })).await;
    let BusMessage::SwitchServerStart { roleid, src_gsid, dst_gsid, .. } = &pedido else { unreachable!() };
    assert_eq!((*roleid, *src_gsid, *dst_gsid), (id, 1, 161));
    assert!(!wa.read().await.players.contains_key(&(id as i64)), "saiu do A");
    assert_eq!(ra.mapa_de(id).await, None);
    assert_eq!(repo.mundo_do_personagem(id).await.unwrap(), Some(161), "gravado no mapa novo");

    // O link repassa ao B, que põe o jogador no 161 e confirma.
    let mut link_b = BusClient::conectar(addr_b).await.unwrap();
    link_b.enviar(pedido).await.unwrap();
    let ok = proxima(&mut link_b, |m| matches!(m, BusMessage::SwitchServerSuccess { .. })).await;
    assert!(matches!(ok, BusMessage::SwitchServerSuccess { dst_gsid: 161, .. }));
    let p = wb.read().await.players.get(&(id as i64)).map(|p| p.position).expect("chegou ao B");
    assert_eq!((p.x, p.z), (10.0, 20.0));
}
