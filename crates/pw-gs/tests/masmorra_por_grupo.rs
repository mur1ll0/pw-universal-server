//! B349 — masmorra por grupo: cada chave (o líder do grupo, ou o jogador sozinho) ganha a sua cópia do mapa 169.
//!
//! O cenário é o do realm 155 depois do B48 — um personagem antigo no mundo 1 e um novo
//! no 161 —, só que os dois mapas no mesmo processo e atrás de uma única escuta de
//! barramento, como o `./gs gs01 ... is61` do original (`pw_gs::mapas`).
//!
//! Sem `TEST_DATABASE_URL` os testes passam sem verificar nada e dizem isso na saída.

use pw_bus::{BusClient, BusListener, BusMessage};
use pw_core::{CharacterClass, Gender, Race};
use pw_data_loader::GameDataManager;
use pw_gs::{RoteadorDeMapas, WorldInstance};
use pw_protocol::GameVersion;
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use std::time::Duration;

#[path = "../../pw-storage/tests/comum/mod.rs"]
mod comum;

const LOCALSID: u32 = 77;
async fn ate<F, Fut>(mut cond: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..500 {
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
async fn cada_grupo_ganha_a_sua_copia_da_masmorra() {
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
    sqlx::query("INSERT INTO realms (id, name, version, host, port, max_players, config) VALUES ($1, 'Teste Masmorra', '1.5.5', '127.0.0.1', 29000, 10, '{}'::jsonb)")
        .bind(&realm)
        .execute(pool.get_ref())
        .await
        .expect("criar realm");
    let conta: i32 = sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
        .bind(format!("gs_{m}"))
        .fetch_one(pool.get_ref())
        .await
        .expect("criar conta");
    let repo = CharacterRepository::new(pool.clone());
    let mut ids = Vec::new();
    for nome in ["Lider", "Membro", "Sozinho"] {
        ids.push(
            repo.create_character(conta, &realm, &format!("{nome}{m}"), Race::Human, CharacterClass::Blademaster, Gender::Male, Vec::new(), None)
                .await
                .expect("criar personagem"),
        );
    }
    for id in &ids {
        sqlx::query("UPDATE characters SET world_id = 1 WHERE id = $1").bind(id).execute(pool.get_ref()).await.unwrap();
    }
    let dados = Arc::new(GameDataManager::new());
    let mut servidos = Vec::new();
    for mapa in [1, 169] {
        let mundo = WorldInstance::new(mapa, Arc::clone(&dados), repo.clone());
        servidos.push(RoteadorDeMapas::preparar_mapa(mundo, GameVersion::V1_5_5).await);
    }
    let mundo_1 = Arc::clone(servidos[0].1.mundo());
    let roteador = Arc::new(RoteadorDeMapas::new(servidos, repo));
    roteador.permitir_carga(pw_gs::mapas::CargaDeMapas { dados, versao: GameVersion::V1_5_5, pasta: None, falhas_na_partida: vec![] });
    roteador.ligar_trocas();
    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();
    tokio::spawn(Arc::clone(&roteador).executar(escuta));

    let mut link = BusClient::conectar(addr).await.unwrap();
    for id in &ids {
        link.enviar(entrar(*id)).await.unwrap();
    }
    let m1 = Arc::clone(&mundo_1);
    let todos = ids.clone();
    assert!(ate(|| { let m1 = Arc::clone(&m1); let t = todos.clone(); async move { let w = m1.read().await; t.iter().all(|i| w.players.contains_key(&(*i as i64))) } }).await);
    // Líder e membro em grupo no mundo 1.
    {
        let mut w = mundo_1.write().await;
        assert!(w.convidar_para_grupo(ids[0], ids[1]).is_ok());
        assert!(w.aceitar_convite(ids[1], ids[0]).is_some());
    }
    let pos = pw_core::Vector3::new(0.0, 0.0, 0.0);
    for id in &ids {
        roteador.transportar(*id, 169, pos).await;
        let r = Arc::clone(&roteador);
        let id = *id;
        assert!(ate(|| { let r = Arc::clone(&r); async move { r.mapa_de(id).await.is_some_and(|m| m >= pw_gs::mapas::PRIMEIRA_COPIA) } }).await, "{id} não chegou a uma cópia");
    }
    let (a, b, c) = (roteador.mapa_de(ids[0]).await.unwrap(), roteador.mapa_de(ids[1]).await.unwrap(), roteador.mapa_de(ids[2]).await.unwrap());
    assert_eq!(a, b, "o grupo na mesma cópia");
    assert_ne!(a, c, "quem está sozinho noutra cópia");
    assert!(roteador.mapas().contains(&a) && roteador.mapas().contains(&c));
    // B359 — o grupo não se desfez na troca de mapa: na cópia, o membro continua no grupo do líder.
    assert_eq!(mundo_1.read().await.lider_do_grupo(ids[1]), Some(ids[0]), "o registro é o do roteador");

    // B353 — dentro da cópia, o membro sai do grupo: a chave dele volta a ser a própria e ele é avisado (60 s); o líder
    // e quem está sozinho na própria cópia ficam. Voltar ao grupo do líder cancela o aviso.
    mundo_1.write().await.sair_do_grupo(ids[1]);
    roteador.ao_sair_do_grupo(ids[1], &[ids[0]]);
    roteador.conferir_quem_esta_nas_copias().await;
    assert!(roteador.saindo_da_masmorra(ids[1]), "quem saiu do grupo é avisado");
    assert!(!roteador.saindo_da_masmorra(ids[0]) && !roteador.saindo_da_masmorra(ids[2]));
    roteador.ao_entrar_no_grupo(ids[0], &[ids[0], ids[1]]).await;
    roteador.conferir_quem_esta_nas_copias().await;
    assert!(!roteador.saindo_da_masmorra(ids[1]), "voltar ao grupo cancela");
}
