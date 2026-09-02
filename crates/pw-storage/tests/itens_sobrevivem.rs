//! Um item não pode perder atributos nem sumir por mudar de lugar.
//!
//! # As duas falhas que este arquivo tranca
//!
//! **1. Os octetos eram apagados a cada troca de slot.** A coluna `extra_data` guarda a
//! essência da arma, os atributos da armadura — tudo que o cliente recebe no `item_info`.
//! O `upsert_item` não escrevia essa coluna. Como `swap_slots` e
//! `move_between_containers` fazem o item dar a volta por `get` → `delete` → `upsert`,
//! **arrastar um item de um slot para outro devolvia ele sem os atributos**. Em silêncio,
//! sem erro nenhum no log. O mesmo valia para o `creator_name`.
//!
//! **2. A troca apagava os dois itens antes de reinserir, sem transação.** A restrição
//! `uq_item_slot_per_container` obriga a apagar antes de escrever, o que abre uma janela
//! em que nenhum dos dois existe. Uma falha ali — rede, processo, banco — e o jogador
//! perde os dois para sempre. Numa operação que ele faz dezenas de vezes por sessão.
//!
//! # Por que contra um banco de verdade
//!
//! As duas falhas moram no SQL: uma coluna ausente do `INSERT` e a falta de um `BEGIN`.
//! Nenhum dublê de repositório mostraria qualquer uma das duas.
//!
//! Sem `TEST_DATABASE_URL` o teste passa sem verificar nada e diz isso na saída. Como
//! rodar está no cabeçalho de `autorizacao_de_personagem.rs`.

use pw_core::{CharacterClass, ContainerType, Gender, ItemRecord, Race};
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};

/// Os octetos de um item, com bytes distintos para que um truncamento apareça.
const OCTETOS: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03];

struct Cenario {
    repo: CharacterRepository,
    role_id: i32,
}

async fn montar() -> Option<Cenario> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return None;
        }
    };
    // Pool pequeno de propósito: cada teste abre o seu, e o padrão (50) multiplicado
    // pelos testes em paralelo estoura o `max_connections` do servidor.
    let cfg = StorageConfig {
        database_url: url,
        max_connections: 2,
        min_connections: 1,
        ..Default::default()
    };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");

    use std::time::{SystemTime, UNIX_EPOCH};
    let m = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;

    let realm = format!("t_it_{m}");
    sqlx::query(
        "INSERT INTO realms (id, name, version, host, port, max_players, config)
         VALUES ($1, 'Teste Itens', '1.2.6', '127.0.0.1', 29000, 10, '{}'::jsonb)",
    )
    .bind(&realm)
    .execute(pool.get_ref())
    .await
    .expect("criar realm");

    let conta: i32 = sqlx::query_scalar(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id",
    )
    .bind(format!("it_{m}"))
    .fetch_one(pool.get_ref())
    .await
    .expect("criar conta");

    let repo = CharacterRepository::new(pool);
    let role_id = repo
        .create_character(
            conta,
            &realm,
            &format!("Mula{m}"),
            Race::Human,
            CharacterClass::Blademaster,
            Gender::Male,
            Vec::new(),
        )
        .await
        .expect("criar personagem");

    Some(Cenario { repo, role_id })
}

macro_rules! cenario {
    () => {
        match montar().await {
            Some(c) => c,
            None => return,
        }
    };
}

/// Uma espada com tudo que pode ser perdido: octetos, refino, pedras, criador.
fn espada(role_id: i32, slot: u16, container: ContainerType) -> ItemRecord {
    ItemRecord {
        id: None,
        character_id: role_id,
        container_type: container,
        slot,
        item_id: 4123,
        count: 1,
        max_count: 1,
        refine_level: 9,
        sockets_count: 2,
        sockets: vec![7001, 7002],
        durability: 4500,
        max_durability: 5000,
        bind_status: 1,
        octets: OCTETOS.to_vec(),
        custom_attributes: serde_json::json!({ "creator_name": "Ferreiro" }),
    }
}

#[tokio::test]
async fn trocar_de_slot_nao_apaga_os_atributos_do_item() {
    let c = cenario!();
    let itens = c.repo.item_repo();

    itens
        .upsert_item(&espada(c.role_id, 3, ContainerType::Inventory))
        .await
        .expect("guardar a espada");

    // Confere o ponto de partida: se o `upsert` já não gravasse os octetos, o teste
    // abaixo passaria por comparar vazio com vazio.
    let antes = itens
        .get_item_by_slot(c.role_id, ContainerType::Inventory, 3)
        .await
        .unwrap()
        .expect("a espada devia estar no slot 3");
    assert_eq!(
        antes.octets, OCTETOS,
        "os octetos não sobreviveram nem à gravação inicial"
    );

    itens
        .swap_slots(c.role_id, ContainerType::Inventory, 3, 8)
        .await
        .expect("trocar de slot");

    let depois = itens
        .get_item_by_slot(c.role_id, ContainerType::Inventory, 8)
        .await
        .unwrap()
        .expect("a espada devia ter ido para o slot 8");

    assert_eq!(
        depois.octets, OCTETOS,
        "arrastar o item apagou os octetos — a essência da arma foi embora"
    );
    assert_eq!(depois.refine_level, 9, "o refino se perdeu");
    assert_eq!(depois.sockets, vec![7001, 7002], "as pedras se perderam");
    assert_eq!(depois.durability, 4500, "a durabilidade se perdeu");
    assert_eq!(depois.bind_status, 1, "o vínculo se perdeu");
    assert_eq!(
        depois.custom_attributes.get("creator_name").unwrap(),
        "Ferreiro",
        "o nome do criador se perdeu"
    );

    // E o slot de origem ficou vazio, e não com uma cópia.
    assert!(
        itens
            .get_item_by_slot(c.role_id, ContainerType::Inventory, 3)
            .await
            .unwrap()
            .is_none(),
        "o item foi duplicado: ficou nos dois slots"
    );
}

#[tokio::test]
async fn equipar_nao_apaga_os_atributos() {
    // Mesmo caminho, atravessando contêineres — é o que acontece ao equipar uma arma.
    let c = cenario!();
    let itens = c.repo.item_repo();

    itens
        .upsert_item(&espada(c.role_id, 0, ContainerType::Inventory))
        .await
        .unwrap();

    itens
        .move_between_containers(
            c.role_id,
            ContainerType::Inventory,
            0,
            ContainerType::Equipment,
            0,
        )
        .await
        .expect("equipar");

    let equipada = itens
        .get_item_by_slot(c.role_id, ContainerType::Equipment, 0)
        .await
        .unwrap()
        .expect("a espada devia estar equipada");

    assert_eq!(
        equipada.octets, OCTETOS,
        "equipar apagou os octetos da arma"
    );
    assert_eq!(equipada.refine_level, 9);
}

#[tokio::test]
async fn trocar_dois_itens_de_lugar_preserva_os_dois() {
    let c = cenario!();
    let itens = c.repo.item_repo();

    let mut poção = espada(c.role_id, 5, ContainerType::Inventory);
    poção.item_id = 8888;
    poção.refine_level = 0;
    poção.count = 20;
    poção.octets = vec![0xAA, 0xBB];

    itens
        .upsert_item(&espada(c.role_id, 4, ContainerType::Inventory))
        .await
        .unwrap();
    itens.upsert_item(&poção).await.unwrap();

    itens
        .swap_slots(c.role_id, ContainerType::Inventory, 4, 5)
        .await
        .expect("trocar");

    let no_4 = itens
        .get_item_by_slot(c.role_id, ContainerType::Inventory, 4)
        .await
        .unwrap()
        .expect("slot 4 vazio depois da troca");
    let no_5 = itens
        .get_item_by_slot(c.role_id, ContainerType::Inventory, 5)
        .await
        .unwrap()
        .expect("slot 5 vazio depois da troca");

    // Cada um foi para o lugar do outro, com o que era seu.
    assert_eq!(no_4.item_id, 8888, "a poção não foi para o slot 4");
    assert_eq!(no_4.octets, vec![0xAA, 0xBB], "a poção perdeu os octetos");
    assert_eq!(no_4.count, 20, "a poção perdeu a quantidade");

    assert_eq!(no_5.item_id, 4123, "a espada não foi para o slot 5");
    assert_eq!(no_5.octets, OCTETOS, "a espada perdeu os octetos");
    assert_eq!(no_5.refine_level, 9, "a espada perdeu o refino");
}

#[tokio::test]
async fn trocar_um_slot_por_ele_mesmo_nao_faz_nada() {
    // O caminho `apaga os dois, reinsere os dois` com o mesmo slot apagaria o item e o
    // reinseriria — trabalho inútil que, sem transação, era uma chance a mais de perder
    // um item por nada.
    let c = cenario!();
    let itens = c.repo.item_repo();

    itens
        .upsert_item(&espada(c.role_id, 7, ContainerType::Inventory))
        .await
        .unwrap();

    itens
        .swap_slots(c.role_id, ContainerType::Inventory, 7, 7)
        .await
        .expect("trocar consigo mesmo");

    let ainda_la = itens
        .get_item_by_slot(c.role_id, ContainerType::Inventory, 7)
        .await
        .unwrap();
    assert!(ainda_la.is_some(), "o item sumiu ao trocar consigo mesmo");
    assert_eq!(ainda_la.unwrap().octets, OCTETOS);
}
