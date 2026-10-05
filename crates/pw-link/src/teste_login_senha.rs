//! Exercita o dispatcher real com conta global no schema test, antes/depois da E4.
use super::*;
use hmac::{Hmac, Mac};
use md5::Md5;
use pw_protocol::C2SChallengeResponse;
use pw_storage::{PostgresPool, StorageConfig};
#[path = "../../pw-storage/tests/comum/mod.rs"]
mod comum;

fn resposta(usuario: &str, senha: &str, desafio: &[u8]) -> InboundPacket {
    // Algoritmo cliente original Network/gameclient.cpp:131-139.
    let chave = hex::decode(pw_crypto::hash_legacy_pw_md5(usuario, senha)).unwrap();
    let mut mac = Hmac::<Md5>::new_from_slice(&chave).unwrap();
    mac.update(desafio);
    InboundPacket::Response(C2SChallengeResponse {
        username: usuario.into(),
        password_response: mac.finalize().into_bytes().to_vec(),
        use_token: false,
        cli_fingerprint: vec![],
    })
}

#[tokio::test]
async fn senha_administrativa_global_e_consumida_pelos_links_126_e_155() {
    verificar_fluxo(false).await;
}

#[tokio::test]
async fn conta_criada_recuperavel_autentica_no_dispatcher_real_126_e_155_sem_gm() {
    verificar_fluxo(true).await;
}

async fn verificar_fluxo(criar: bool) {
    let config = StorageConfig {
        database_url: std::env::var("TEST_DATABASE_URL").expect("banco obrigatório"),
        max_connections: 4,
        min_connections: 1,
        ..Default::default()
    };
    let pool = PostgresPool::new(&config).await.unwrap();
    comum::limpar_sobras_de_teste(&pool).await;
    let contas = AccountRepository::new(pool.clone());
    let mut nome = format!("at_{}", rand_sufixo());
    let conta = contas
        .create_account(
            &nome,
            &pw_crypto::hash_legacy_pw_md5(&nome, "Anterior!"),
            None,
        )
        .await
        .unwrap();
    contas.set_gm_privileges(conta.id, 1).await.unwrap();
    let administrador = conta.id;
    let conta = if criar {
        nome = format!("at_{}_2", rand_sufixo());
        let resultado = contas
            .comandos_administrativos()
            .criar_conta(
                &format!("criar-login-{administrador}"),
                administrador,
                "realm_126",
                &nome,
                &[3; 32],
                &pw_crypto::hash_legacy_pw_md5(&nome, "Anterior!"),
            )
            .await
            .unwrap();
        contas
            .find_by_id(resultado["conta_id"].as_i64().unwrap() as i32)
            .await
            .unwrap()
            .unwrap()
    } else {
        conta
    };
    let id = format!("login-{}", conta.id);
    let cache = CacheManager::new(&config).await.unwrap();
    for (versao, realm) in [
        (GameVersion::V1_2_6, "realm_126"),
        (GameVersion::V1_5_5, "realm_155"),
    ] {
        let gateway = LinkGateway {
            coordenacao_gm: RwLock::new(()),
            sessoes_gm: RwLock::new(HashMap::new()),
            realm_id: realm.into(),
            game_version: versao,
            adapter: create_protocol_adapter(versao),
            listen_port: 0,
            account_repo: contas.clone(),
            char_repo: CharacterRepository::new(pool.clone()),
            cache_manager: cache.clone(),
            data_manager: Arc::new(GameDataManager::new()),
            versao_do_cliente: VersaoDoCliente::padrao(versao),
            uplink: None,
            uplinks_por_mundo: HashMap::new(),
            jogadores_visiveis: RwLock::new(HashMap::new()),
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let mut sessao =
            ClientSession::new(1, "127.0.0.1".into(), realm.into(), versao.to_string());
        sessao.desafio_login = vec![1; 16];
        gateway
            .dispatch_packet(&tx, &mut sessao, resposta(&nome, "Errada!", &[1; 16]))
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(OutboundPacket::ErrorInfo(_))
        ));
        assert!(sessao.account_id.is_none());
        if criar {
            let mut nova_sessao =
                ClientSession::new(2, "127.0.0.1".into(), realm.into(), versao.to_string());
            nova_sessao.desafio_login = vec![1; 16];
            gateway
                .dispatch_packet(
                    &tx,
                    &mut nova_sessao,
                    resposta(&nome, "Anterior!", &[1; 16]),
                )
                .await
                .unwrap();
            assert!(matches!(
                rx.recv().await,
                Some(OutboundPacket::OnlineAnnounce(_))
            ));
            assert_eq!(nova_sessao.account_id, Some(conta.id));
            assert_eq!(nova_sessao.sec_level, 0);
            continue;
        }
        let resultado = contas
            .comandos_administrativos()
            .trocar_senha(
                &id,
                administrador,
                realm,
                conta.id,
                &[1; 32],
                &pw_crypto::hash_legacy_pw_md5(&nome, "Nova!"),
            )
            .await
            .unwrap();
        assert_eq!(resultado["estado"], "salvo");
        gateway
            .dispatch_packet(&tx, &mut sessao, resposta(&nome, "Anterior!", &[1; 16]))
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(OutboundPacket::ErrorInfo(_))
        ));
        assert!(sessao.account_id.is_none());
        // Prova copiada de outra conexão (desafio diferente) não autentica.
        gateway
            .dispatch_packet(&tx, &mut sessao, resposta(&nome, "Nova!", &[2; 16]))
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(OutboundPacket::ErrorInfo(_))
        ));
        let mut token = resposta(&nome, "Nova!", &[1; 16]);
        if let InboundPacket::Response(ref mut login) = token {
            login.use_token = true;
        }
        gateway
            .dispatch_packet(&tx, &mut sessao, token)
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(OutboundPacket::ErrorInfo(_))
        ));
        assert!(sessao.account_id.is_none());
        gateway
            .dispatch_packet(&tx, &mut sessao, resposta(&nome, "Nova!", &[1; 16]))
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(OutboundPacket::OnlineAnnounce(_))
        ));
        assert_eq!(sessao.account_id, Some(conta.id));
        assert!(sessao.desafio_login.is_empty());
        gateway
            .dispatch_packet(&tx, &mut sessao, resposta(&nome, "Nova!", &[1; 16]))
            .await
            .unwrap();
        assert!(
            rx.try_recv().is_err(),
            "Response repetido não troca a identidade"
        );
    }
    sqlx::query("DELETE FROM comandos_administrativos WHERE administrador_id=$1")
        .bind(administrador)
        .execute(pool.get_ref())
        .await
        .unwrap();
    sqlx::query("DELETE FROM accounts WHERE id IN ($1,$2)")
        .bind(conta.id)
        .bind(administrador)
        .execute(pool.get_ref())
        .await
        .unwrap();
}

fn rand_sufixo() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
