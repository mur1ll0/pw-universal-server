//! B363 — a lista de amigos de ponta a ponta no link, com o banco: o pedido por nome (202) vira o
//! `AddFriendRqst` (204) ao destino; o aceite põe o destino na lista da origem (203 com 0) e avisa o
//! destino (214); repetido dá 7; grupo (208/210), lista (206 → status), saída (214 com 0), remoção
//! (212) e recusa (203 com 2).

use pw_core::{CharacterClass, Gender, Race};
use pw_link::amizade::{self, Presenca};
use pw_link::gateway::LinkGateway;
use pw_protocol::{OctetsStream, OutboundPacket};
use pw_storage::{AccountRepository, CacheManager, CharacterRepository, PostgresPool, StorageConfig};
use tokio::sync::mpsc;

fn utf16(t: &str) -> Vec<u8> {
    t.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

async fn proximo(rx: &mut mpsc::Receiver<OutboundPacket>, opcode: u32) -> Vec<u8> {
    loop {
        let p = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("o opcode {opcode} não chegou"))
            .expect("canal fechado");
        if let OutboundPacket::Raw { opcode: o, payload } = p {
            if o == opcode {
                return payload;
            }
        }
    }
}

#[tokio::test]
async fn amizade_pedido_aceite_grupo_lista_saida_e_remocao() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("AVISO: sem TEST_DATABASE_URL — este teste NÃO verificou nada.");
        return;
    };
    let cfg = StorageConfig { database_url: url, max_connections: 3, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("banco");
    let m = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;
    let realm = format!("t_amz_{m}");
    sqlx::query("INSERT INTO realms (id, name, version, host, port, max_players, config) VALUES ($1, 'Teste', '1.5.5', '127.0.0.1', 1, 10, '{}'::jsonb)")
        .bind(&realm)
        .execute(pool.get_ref())
        .await
        .unwrap();
    let conta: i32 = sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
        .bind(format!("amz_{m}"))
        .fetch_one(pool.get_ref())
        .await
        .unwrap();
    let repo = CharacterRepository::new(pool.clone());
    let (na, nb) = (format!("Ana{m}"), format!("Bia{m}"));
    let criar = |nome: String, cls| {
        let repo = repo.clone();
        let realm = realm.clone();
        async move {
            repo.create_character(conta, &realm, &nome, Race::Human, cls, Gender::Female, Vec::new(), None).await.unwrap()
        }
    };
    let a = criar(na.clone(), CharacterClass::Blademaster).await;
    let b = criar(nb.clone(), CharacterClass::Wizard).await;
    let cache = CacheManager::new(&StorageConfig::default()).await.expect("cache");
    let gw = LinkGateway::new(format!("sem_dados_{m}"), "1.5.5", 0, AccountRepository::new(pool.clone()), repo.clone(), cache);
    let (txa, mut rxa) = mpsc::channel(64);
    let (txb, mut rxb) = mpsc::channel(64);
    gw.amizade_ao_entrar(a, Presenca { envio: txa.clone(), localsid: 11, nome: na.clone(), classe: 0 }).await;
    gw.amizade_ao_entrar(b, Presenca { envio: txb.clone(), localsid: 22, nome: nb.clone(), classe: CharacterClass::Wizard as u8 as i8 }).await;

    // AddFriend por nome (dstroleid 0).
    let pedir = || {
        let mut s = OctetsStream::new();
        s.write_i32(a);
        s.write_i32(0);
        s.write_octets(&utf16(&nb));
        s.write_u32(11);
        s.as_slice().to_vec()
    };
    assert!(gw.amizade_pacote(a, &txa, amizade::OP_ADD_FRIEND, &pedir()).await);
    let rqst = proximo(&mut rxb, amizade::OP_ADD_FRIEND_RQST).await;
    let xid = u32::from_be_bytes(rqst[0..4].try_into().unwrap());
    assert_ne!(xid & amizade::XID_PEDIDO, 0, "vai como pedido");
    assert_eq!(i32::from_be_bytes(rqst[4..8].try_into().unwrap()), a, "srcroleid");
    let responder = |ret: i8| {
        let mut r = (xid & !amizade::XID_PEDIDO).to_be_bytes().to_vec();
        r.push(ret as u8);
        r
    };
    assert!(gw.amizade_pacote(b, &txb, amizade::OP_ADD_FRIEND_RQST, &responder(0)).await);
    let re = proximo(&mut rxa, amizade::OP_ADD_FRIEND_RE).await;
    assert_eq!((re[0], i32::from_be_bytes(re[1..5].try_into().unwrap())), (0, b), "aceito");
    let st = proximo(&mut rxb, amizade::OP_FRIEND_STATUS).await;
    assert_eq!((i32::from_be_bytes(st[0..4].try_into().unwrap()), st[4]), (a, 1), "o destino sabe que a origem está online");
    let lista = repo.lista_de_amigos(a).await.unwrap();
    assert_eq!(lista.amigos.len(), 1);
    assert_eq!((lista.amigos[0].id, lista.amigos[0].nome.as_str(), lista.amigos[0].classe), (b, nb.as_str(), CharacterClass::Wizard as u8 as i8));
    assert!(repo.lista_de_amigos(b).await.unwrap().amigos.is_empty(), "mão única");

    // Repetido.
    gw.amizade_pacote(a, &txa, amizade::OP_ADD_FRIEND, &pedir()).await;
    assert_eq!(proximo(&mut rxa, amizade::OP_ADD_FRIEND_RE).await[0], 7);

    // Grupo: cria "G" (1) e põe a Bia nele.
    let mut s = OctetsStream::new();
    s.write_i32(a);
    s.write_i8(1);
    s.write_octets(&utf16("G"));
    s.write_u32(11);
    gw.amizade_pacote(a, &txa, amizade::OP_SET_GROUP_NAME, s.as_slice()).await;
    assert_eq!(proximo(&mut rxa, amizade::OP_SET_GROUP_NAME_RE).await[0], 0);
    let mut s = OctetsStream::new();
    s.write_i8(1);
    s.write_i32(a);
    s.write_i32(b);
    s.write_u32(11);
    gw.amizade_pacote(a, &txa, amizade::OP_SET_FRIEND_GROUP, s.as_slice()).await;
    assert_eq!(proximo(&mut rxa, amizade::OP_SET_FRIEND_GROUP_RE).await[0], 0);
    let re = gw.amizade_lista(a).await;
    assert_eq!((re.groups.len(), re.friends[0].gid), (1, 1));
    assert!(re.status.is_empty(), "a Bia não tem a Ana na lista: sem status");

    // Saída da Ana: a Bia, que está na lista dela, recebe o 214 com 0.
    gw.amizade_ao_sair(a).await;
    let st = proximo(&mut rxb, amizade::OP_FRIEND_STATUS).await;
    assert_eq!((i32::from_be_bytes(st[0..4].try_into().unwrap()), st[4]), (a, 0));
    gw.amizade_ao_entrar(a, Presenca { envio: txa.clone(), localsid: 11, nome: na.clone(), classe: 0 }).await;

    // Remoção e, de novo, o pedido recusado.
    let mut s = OctetsStream::new();
    s.write_i32(a);
    s.write_i32(b);
    s.write_u32(11);
    gw.amizade_pacote(a, &txa, amizade::OP_DEL_FRIEND, s.as_slice()).await;
    assert_eq!(proximo(&mut rxa, amizade::OP_DEL_FRIEND_RE).await[0], 0);
    assert!(repo.lista_de_amigos(a).await.unwrap().amigos.is_empty());
    gw.amizade_pacote(a, &txa, amizade::OP_ADD_FRIEND, &pedir()).await;
    let rqst = proximo(&mut rxb, amizade::OP_ADD_FRIEND_RQST).await;
    let xid2 = u32::from_be_bytes(rqst[0..4].try_into().unwrap()) & !amizade::XID_PEDIDO;
    let mut r = xid2.to_be_bytes().to_vec();
    r.push(1);
    gw.amizade_pacote(b, &txb, amizade::OP_ADD_FRIEND_RQST, &r).await;
    assert_eq!(proximo(&mut rxa, amizade::OP_ADD_FRIEND_RE).await[0], 2, "recusado");
}
