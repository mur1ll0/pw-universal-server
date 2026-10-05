//! TCP real, conta/GM reais no schema test e o mesmo roteador do jogo.
use pw_bus::{BusClient, BusListener, BusMessage};
use pw_data_loader::GameDataManager;
use pw_gs::{
    administracao::{
        assinar, conferir_assinatura, escrever_quadro, ler_quadro, ServidorAdministrativo,
    },
    BusServer, RoteadorDeMapas, WorldInstance,
};
use pw_protocol::GameVersion;
use pw_storage::{AccountRepository, CharacterRepository, PostgresPool, StorageConfig};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
    task::JoinHandle,
};

#[path = "../../pw-storage/tests/comum/mod.rs"]
mod comum;
const CHAVE: [u8; 32] = [83; 32];

#[tokio::test]
async fn criacao_valida_nome_no_daemon_e_deduplica_variacao_de_caixa() {
    let c = Cenario::montar().await;
    for nome in ["", "é", "nome com espaço", "a\n", "a-"] {
        let resposta = c
            .pedir(
                &c.realm,
                json!({"tipo":"criar_conta", "usuario":nome, "senha":"Nova!"}),
            )
            .await;
        assert_eq!(resposta["dados"]["codigo"], "alvo_invalido");
    }
    let nome = format!("at_{}_2", c.conta);
    let consulta = json!({"tipo":"criar_conta", "usuario":nome.to_uppercase(), "senha":"Nova!"});
    let recusado = c.pedir("outro_realm", consulta.clone()).await;
    assert_eq!(recusado["dados"]["codigo"], "realm_incorreto");
    let original = c.pedir_com_id(&c.realm, "criar", c.conta, consulta).await;
    assert_eq!(original["estado"], "salvo");
    assert_eq!(original["dados"]["usuario"], nome);
    let repetido = c
        .pedir_com_id(
            &c.realm,
            "criar",
            c.conta,
            json!({"tipo":"criar_conta", "usuario":nome, "senha":"Nova!"}),
        )
        .await;
    assert_eq!(original, repetido);
    let outro_nome = c
        .pedir_com_id(
            &c.realm,
            "criar",
            c.conta,
            json!({"tipo":"criar_conta", "usuario":"outro", "senha":"Nova!"}),
        )
        .await;
    assert_eq!(outro_nome["dados"]["codigo"], "operacao_em_conflito");
    sqlx::query("UPDATE accounts SET is_banned=true WHERE id=$1")
        .bind(c.conta)
        .execute(c.pool.get_ref())
        .await
        .unwrap();
    let banido = c
        .pedir_com_id(
            &c.realm,
            "outro",
            c.conta,
            json!({"tipo":"criar_conta", "usuario":"outro", "senha":"Nova!"}),
        )
        .await;
    assert_eq!(banido["dados"]["codigo"], "administrador_recusado");
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(original["dados"]["conta_id"].as_i64().unwrap() as i32)
        .execute(c.pool.get_ref())
        .await
        .unwrap();
    c.encerrar().await;
}

struct Cenario {
    pool: PostgresPool,
    realm: String,
    conta: i32,
    personagem: i32,
    admin: SocketAddr,
    bus: SocketAddr,
    mundo: Arc<RwLock<WorldInstance>>,
    roteador: Arc<RoteadorDeMapas>,
    tarefas: Vec<JoinHandle<()>>,
}

impl Cenario {
    async fn montar() -> Self { Self::montar_versao(GameVersion::V1_2_6).await }

    async fn montar_versao(versao: GameVersion) -> Self {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL obrigatória; sem banco é falha");
        let pool = PostgresPool::new(&StorageConfig {
            database_url: url,
            max_connections: 4,
            min_connections: 1,
            ..Default::default()
        })
        .await
        .unwrap();
        comum::limpar_sobras_de_teste(&pool).await;
        let sufixo = rand::random::<u64>();
        let realm = format!("t_admin_{sufixo}");
        sqlx::query("INSERT INTO realms(id,name,version,host,port) VALUES($1,'Admin teste','1.2.6','127.0.0.1',1)")
            .bind(&realm).execute(pool.get_ref()).await.unwrap();
        let conta: i32 = sqlx::query_scalar("INSERT INTO accounts(username,password_hash,gm_privileges) VALUES($1,'teste',1) RETURNING id")
            .bind(format!("gs_{sufixo}")).fetch_one(pool.get_ref()).await.unwrap();
        let personagem: i32 = sqlx::query_scalar("INSERT INTO characters(account_id,realm_id,name,race,cls,gender) VALUES($1,$2,'AdminTeste',0,0,0) RETURNING id")
            .bind(conta).bind(&realm).fetch_one(pool.get_ref()).await.unwrap();
        let repo = CharacterRepository::new(pool.clone());
        let mut mapas = Vec::new();
        for tag in [1, 161] {
            let mundo = Arc::new(RwLock::new(WorldInstance::new(
                tag,
                Arc::new(GameDataManager::new()),
                repo.clone(),
            )));
            mapas.push((tag, Arc::new(BusServer::new(mundo, versao))));
        }
        let mundo = Arc::clone(mapas[0].1.mundo());
        let roteador = Arc::new(RoteadorDeMapas::new(mapas, repo));
        roteador.ligar_trocas();
        let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
        let bus = escuta.local_addr().unwrap();
        let r = Arc::clone(&roteador);
        let tarefa_bus = tokio::spawn(async move {
            r.executar(escuta).await;
        });
        let escuta = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let admin = escuta.local_addr().unwrap();
        let servidor = Arc::new(
            ServidorAdministrativo::new(
                realm.clone(),
                CHAVE.to_vec(),
                Arc::clone(&roteador),
                AccountRepository::new(pool.clone()),
            )
            .unwrap(),
        );
        let tarefa_admin = tokio::spawn(async move {
            servidor.executar(escuta).await.unwrap();
        });
        Self {
            pool,
            realm,
            conta,
            personagem,
            admin,
            bus,
            mundo,
            roteador,
            tarefas: vec![tarefa_bus, tarefa_admin],
        }
    }

    async fn pedir(&self, realm: &str, consulta: Value) -> Value {
        self.pedir_com_id(realm, "teste-1", self.conta, consulta)
            .await
    }

    async fn pedir_com_id(
        &self,
        realm: &str,
        id: &str,
        administrador: i32,
        consulta: Value,
    ) -> Value {
        let mut socket = TcpStream::connect(self.admin).await.unwrap();
        let saudacao: Value =
            serde_json::from_slice(&ler_quadro(&mut socket).await.unwrap()).unwrap();
        let desafio = hex::decode(saudacao["desafio"].as_str().unwrap()).unwrap();
        let corpo = serde_json::to_vec(&json!({"operacao_id":id,"realm_id":realm,"administrador_id":administrador,"consulta":consulta})).unwrap();
        escrever_quadro(&mut socket, &corpo).await.unwrap();
        socket
            .write_all(&assinar(&CHAVE, &desafio, b"pedido", &corpo))
            .await
            .unwrap();
        let resposta = ler_quadro(&mut socket).await.unwrap();
        let mut assinatura = [0; 32];
        socket.read_exact(&mut assinatura).await.unwrap();
        conferir_assinatura(&CHAVE, &desafio, b"resposta", &resposta, &assinatura).unwrap();
        serde_json::from_slice(&resposta).unwrap()
    }

    async fn consultar(&self) -> Value {
        self.pedir(
            &self.realm,
            json!({"tipo":"personagem","personagem_id":self.personagem}),
        )
        .await
    }

    async fn entrar(&self) -> pw_bus::transport::BusConnection {
        let mut bus = BusClient::conectar(self.bus).await.unwrap();
        bus.enviar(BusMessage::EnterWorld {
            roleid: self.personagem,
            provider_link_id: 1,
            locktime: 0,
            timeout: 60,
            settime: 0,
            localsid: 77,
        })
        .await
        .unwrap();
        bus
    }

    async fn esperar_presenca(&self, presenca: &str) -> Value {
        for _ in 0..500 { // 5 s: sob a suíte paralela 1 s falhava (B172)
            let resposta = self.consultar().await;
            if resposta["dados"]["presenca"] == presenca {
                return resposta;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("presença não chegou a {presenca}");
    }

    async fn encerrar(&self) {
        for tarefa in &self.tarefas {
            tarefa.abort();
        }
        sqlx::query("DELETE FROM comandos_administrativos WHERE administrador_id=$1")
            .bind(self.conta)
            .execute(self.pool.get_ref())
            .await
            .unwrap();
        sqlx::query("DELETE FROM accounts WHERE id=$1")
            .bind(self.conta)
            .execute(self.pool.get_ref())
            .await
            .unwrap();
        sqlx::query("DELETE FROM realms WHERE id=$1")
            .bind(&self.realm)
            .execute(self.pool.get_ref())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn senha_global_deduplica_concorrencia_conflito_e_resultado_apos_reinicio() {
    let c = Cenario::montar().await;
    let id = format!("senha-{}", c.conta);
    let comando = json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":"SenhaNova!"});
    let (a, b) = tokio::join!(
        c.pedir_com_id(&c.realm, &id, c.conta, comando.clone()),
        c.pedir_com_id(&c.realm, &id, c.conta, comando.clone())
    );
    assert_eq!(a["estado"], "salvo");
    assert_eq!(a, b);
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(c.conta)
        .fetch_one(c.pool.get_ref())
        .await
        .unwrap();
    let nome = AccountRepository::new(c.pool.clone())
        .find_by_id(c.conta)
        .await
        .unwrap()
        .unwrap()
        .username;
    assert!(pw_crypto::verify_password(&nome, "SenhaNova!", &hash).is_valid);
    assert_eq!(
        c.pedir_com_id(
            &c.realm,
            &id,
            c.conta,
            json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":"Outra!"})
        )
        .await["dados"]["codigo"],
        "operacao_em_conflito"
    );
    let outro = format!("novo-{}", c.conta);
    assert_eq!(
        c.pedir_com_id(
            &c.realm,
            &outro,
            c.conta,
            json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":"MaisNova!"})
        )
        .await["estado"],
        "salvo"
    );
    // Outro processo e realm recuperam o mesmo registro global.
    c.tarefas[1].abort();
    let escuta = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = escuta.local_addr().unwrap();
    let servidor = Arc::new(
        ServidorAdministrativo::new(
            "outro_realm".into(),
            CHAVE.to_vec(),
            Arc::clone(&c.roteador),
            AccountRepository::new(c.pool.clone()),
        )
        .unwrap(),
    );
    let tarefa = tokio::spawn(async move {
        servidor.executar(escuta).await.unwrap();
    });
    let c2 = Cenario {
        admin: endereco,
        realm: "outro_realm".into(),
        tarefas: vec![],
        pool: c.pool.clone(),
        conta: c.conta,
        personagem: c.personagem,
        bus: c.bus,
        mundo: Arc::clone(&c.mundo),
        roteador: Arc::clone(&c.roteador),
    };
    assert_eq!(
        c2.pedir_com_id(&c2.realm, &id, c.conta, comando).await["estado"],
        "salvo"
    );
    assert_eq!(
        c2.pedir(&c2.realm, json!({"tipo":"resultado", "comando_id":id}))
            .await["dados"],
        a["dados"]
    );
    let hash = AccountRepository::new(c.pool.clone())
        .find_by_id(c.conta)
        .await
        .unwrap()
        .unwrap()
        .password_hash;
    assert!(
        pw_crypto::verify_password(&nome, "MaisNova!", &hash).is_valid,
        "replay antigo não desfaz comando novo"
    );
    tarefa.abort();
    c.encerrar().await;
}

#[tokio::test]
async fn senha_recusa_alvo_invalido_payload_secreto_e_operacao_de_outro_administrador() {
    let c = Cenario::montar().await;
    let id = format!("protegido-{}", c.conta);
    assert_eq!(
        c.pedir_com_id(
            "realm_errado",
            &id,
            c.conta,
            json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":"Nova!"})
        )
        .await["dados"]["codigo"],
        "realm_incorreto"
    );
    for senha in ["", "não-ascii", "\n"] {
        assert_eq!(
            c.pedir_com_id(
                &c.realm,
                &id,
                c.conta,
                json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":senha})
            )
            .await["estado"],
            "falha"
        );
    }
    let resposta = c
        .pedir_com_id(
            &c.realm,
            &id,
            c.conta,
            json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":"SegredoDoTeste!"}),
        )
        .await;
    assert_eq!(resposta["estado"], "salvo");
    let registro: String = sqlx::query_scalar(
        "SELECT row_to_json(c)::text FROM comandos_administrativos c WHERE operacao_id=$1",
    )
    .bind(&id)
    .fetch_one(c.pool.get_ref())
    .await
    .unwrap();
    assert!(!registro.contains("SegredoDoTeste"));
    assert!(!registro.contains("password_hash"));
    let outro: i32 = sqlx::query_scalar("INSERT INTO accounts(username,password_hash,gm_privileges) VALUES($1,'teste',1) RETURNING id")
        .bind(format!("at_{}", rand::random::<u64>())).fetch_one(c.pool.get_ref()).await.unwrap();
    assert_eq!(
        c.pedir_com_id(
            &c.realm,
            "recuperar",
            outro,
            json!({"tipo":"resultado", "comando_id":id})
        )
        .await["estado"],
        "desconhecido"
    );
    assert_eq!(
        c.pedir_com_id(
            &c.realm,
            &id,
            outro,
            json!({"tipo":"trocar_senha", "conta_id":c.conta, "senha":"SegredoDoTeste!"})
        )
        .await["dados"]["codigo"],
        "operacao_em_conflito"
    );
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(outro)
        .execute(c.pool.get_ref())
        .await
        .unwrap();
    c.encerrar().await;
}

#[tokio::test]
async fn canal_confere_realm_e_revogacao_de_gm() {
    let c = Cenario::montar().await;
    let resposta = c.pedir("realm_errado", json!({"tipo":"mundos"})).await;
    assert_eq!(resposta["dados"]["codigo"], "realm_incorreto");
    sqlx::query("UPDATE accounts SET gm_privileges=0 WHERE id=$1")
        .bind(c.conta)
        .execute(c.pool.get_ref())
        .await
        .unwrap();
    assert_eq!(
        c.consultar().await["dados"]["codigo"],
        "administrador_recusado"
    );
    sqlx::query("UPDATE accounts SET gm_privileges=1,is_banned=true WHERE id=$1")
        .bind(c.conta)
        .execute(c.pool.get_ref())
        .await
        .unwrap();
    assert_eq!(
        c.consultar().await["dados"]["codigo"],
        "administrador_recusado"
    );
    c.encerrar().await;
}

#[tokio::test]
async fn canal_recusa_assinatura_incorreta_replay_e_quadro_excessivo() {
    let c = Cenario::montar().await;
    let corpo = serde_json::to_vec(&json!({"operacao_id":"replay","realm_id":c.realm,"administrador_id":c.conta,"consulta":{"tipo":"mundos"}})).unwrap();
    let mut antigo = TcpStream::connect(c.admin).await.unwrap();
    let saudacao: Value = serde_json::from_slice(&ler_quadro(&mut antigo).await.unwrap()).unwrap();
    let desafio = hex::decode(saudacao["desafio"].as_str().unwrap()).unwrap();
    drop(antigo);
    for assinatura in [vec![0; 32], assinar(&CHAVE, &desafio, b"pedido", &corpo)] {
        let mut socket = TcpStream::connect(c.admin).await.unwrap();
        ler_quadro(&mut socket).await.unwrap();
        escrever_quadro(&mut socket, &corpo).await.unwrap();
        socket.write_all(&assinatura).await.unwrap();
        assert!(
            ler_quadro(&mut socket).await.is_err(),
            "consulta sem autenticação devolveu dados"
        );
    }
    let mut socket = TcpStream::connect(c.admin).await.unwrap();
    ler_quadro(&mut socket).await.unwrap();
    socket.write_u32(8193).await.unwrap();
    assert!(ler_quadro(&mut socket).await.is_err());
    c.encerrar().await;
}

#[tokio::test]
async fn consulta_ve_memoria_viva_e_mapas_reais_sem_escrever_banco() {
    let c = Cenario::montar().await;
    assert_eq!(c.consultar().await["dados"]["presenca"], "ausente");
    let _bus = c.entrar().await;
    c.esperar_presenca("online").await;
    c.mundo
        .write()
        .await
        .players
        .get_mut(&(c.personagem as i64))
        .unwrap()
        .exp = 9_007_199_254_740_993;
    assert_eq!(
        c.consultar().await["dados"]["ficha"]["exp"],
        "9007199254740993"
    );
    let exp: i64 = sqlx::query_scalar("SELECT exp FROM characters WHERE id=$1")
        .bind(c.personagem)
        .fetch_one(c.pool.get_ref())
        .await
        .unwrap();
    assert_eq!(exp, 0, "consulta administrativa alterou o banco");
    let resposta = c.pedir(&c.realm, json!({"tipo":"mundos"})).await;
    assert_eq!(
        resposta["dados"]["mapas"],
        json!([{"mapa":1,"jogadores_online":1},{"mapa":161,"jogadores_online":0}])
    );
    c.encerrar().await;
}

#[tokio::test]
async fn entrada_em_andamento_nao_parece_offline() {
    let c = Cenario::montar().await;
    let mut transacao = c.pool.get_ref().begin().await.unwrap();
    sqlx::query("LOCK TABLE characters IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *transacao)
        .await
        .unwrap();
    let _bus = c.entrar().await;
    c.esperar_presenca("em_transicao").await;
    transacao.rollback().await.unwrap();
    c.esperar_presenca("online").await;
    c.encerrar().await;
}

#[tokio::test]
async fn queda_do_link_salva_e_remove_a_entidade() {
    let c = Cenario::montar().await;
    let bus = c.entrar().await;
    c.esperar_presenca("online").await;
    drop(bus);
    c.esperar_presenca("ausente").await;
    assert!(!c.mundo.read().await.players.contains_key(&(c.personagem as i64)));
    c.encerrar().await;
}

#[tokio::test]
async fn consulta_acompanha_troca_de_mapa_e_logout() {
    let c = Cenario::montar().await;
    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    c.roteador
        .transportar(c.personagem, 161, pw_core::Vector3::new(100.0, 0.0, 100.0))
        .await;
    let mut chegou = false;
    for _ in 0..100 {
        if c.consultar().await["dados"]["ficha"]["mapa"] == 161 {
            chegou = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(chegou);
    bus.enviar(BusMessage::PlayerLogout {
        result: 0,
        roleid: c.personagem,
        provider_link_id: 1,
        localsid: 77,
    })
    .await
    .unwrap();
    c.esperar_presenca("ausente").await;
    c.encerrar().await;
}

#[tokio::test]
async fn gm_e_consumido_e_revogado_em_sessoes_vivas_126_e_155() {
    let c1=Cenario::montar_versao(GameVersion::V1_2_6).await;
    let c2=Cenario::montar_versao(GameVersion::V1_5_5).await;
    sqlx::query("UPDATE characters SET account_id=$1 WHERE id=$2").bind(c1.conta).bind(c2.personagem).execute(c2.pool.get_ref()).await.unwrap();
    let admin: i32=sqlx::query_scalar("INSERT INTO accounts(username,password_hash,gm_privileges) VALUES($1,'teste',1) RETURNING id")
        .bind(format!("at_{}",rand::random::<u64>())).fetch_one(c1.pool.get_ref()).await.unwrap();
    let contas=AccountRepository::new(c1.pool.clone());
    let processos=vec![format!("teste-gs-126-{admin}"),format!("teste-gs-155-{admin}")];
    let mut tarefas=Vec::new();
    tarefas.push(c1.roteador.iniciar_coordenacao_gm(contas.clone(),processos[0].clone()).await.unwrap());
    tarefas.push(c2.roteador.iniciar_coordenacao_gm(contas.clone(),processos[1].clone()).await.unwrap());
    let mut b1=c1.entrar().await; let mut b2=c2.entrar().await;
    for c in [&c1,&c2] {
        for _ in 0..150 { if c.mundo.read().await.players.contains_key(&(c.personagem as i64)) {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
        assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].sec_level,1);
    }
    for (c,b) in [(&c1,&mut b1),(&c2,&mut b2)] {
        b.enviar(BusMessage::ClientToGame{roleid:c.personagem,localsid:77,data:pw_gs::comandos::ids::GM_INVINCIBLE.to_le_bytes().to_vec()}).await.unwrap();
        for _ in 0..100 { if c.mundo.read().await.players[&(c.personagem as i64)].efeitos.gm_invencivel {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
        let mut m=c.mundo.write().await;
        let p=m.players.get_mut(&(c.personagem as i64)).unwrap();
        assert!(p.efeitos.gm_invencivel);
        assert_eq!(pw_gs::efeitos::dano_recebido(&mut p.efeitos,500),0);
    }
    let id=format!("gm-{admin}");
    let repo=contas.comandos_administrativos();
    let r=repo.definir_gm(&id,admin,&c1.realm,c1.conta,false,&[7;32],&processos).await.unwrap();
    assert_eq!(r["estado"],"pendente");
    // Revogação já persistida nega consumo mesmo antes do recibo do reconciliador.
    for (c,b) in [(&c1,&mut b1),(&c2,&mut b2)] {
        b.enviar(BusMessage::ClientToGame{roleid:c.personagem,localsid:77,data:pw_gs::comandos::ids::GM_INVISIBLE.to_le_bytes().to_vec()}).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(30)).await;
    for c in [&c1,&c2] { assert!(!c.mundo.read().await.players[&(c.personagem as i64)].efeitos.gm_invisivel); }
    for _ in 0..200 { if repo.consultar(&id,admin).await.unwrap()["estado"]=="aplicado" {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"aplicado");
    for (c,b) in [(&c1,&mut b1),(&c2,&mut b2)] {
        {
            let mut m=c.mundo.write().await; let p=m.players.get_mut(&(c.personagem as i64)).unwrap();
            assert_eq!(p.sec_level,0); assert!(!p.efeitos.gm_invencivel);
            assert_eq!(pw_gs::efeitos::dano_recebido(&mut p.efeitos,500),500);
        }
        b.enviar(BusMessage::ClientToGame{roleid:c.personagem,localsid:77,data:pw_gs::comandos::ids::GM_INVINCIBLE.to_le_bytes().to_vec()}).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    for c in [&c1,&c2] {assert!(!c.mundo.read().await.players[&(c.personagem as i64)].efeitos.gm_invencivel);}
    // Nova concessão é consumida na mesma sessão; pacote de auth do cliente exige reentrada.
    let id2=format!("{id}-2");
    repo.definir_gm(&id2,admin,&c2.realm,c1.conta,true,&[8;32],&processos).await.unwrap();
    for _ in 0..200 {if repo.consultar(&id2,admin).await.unwrap()["estado"]=="aplicado"{break;}tokio::time::sleep(Duration::from_millis(10)).await;}
    assert_eq!(repo.consultar(&id2,admin).await.unwrap()["estado"],"aplicado");
    for c in [&c1,&c2] {assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].sec_level,1);}
    for t in tarefas.drain(..){t.abort(); let _=t.await;}
    sqlx::query("DELETE FROM coordenacao_gm_processos WHERE processo=ANY($1)").bind(&processos).execute(c1.pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM comandos_administrativos WHERE administrador_id=$1").bind(admin).execute(c1.pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE id=$1").bind(admin).execute(c1.pool.get_ref()).await.unwrap();
    sqlx::query("UPDATE characters SET account_id=$1 WHERE id=$2").bind(c2.conta).bind(c2.personagem).execute(c2.pool.get_ref()).await.unwrap();
    c2.encerrar().await; c1.encerrar().await;
}


async fn fotografia(c:&Cenario) -> pw_gs::EstadoParaGravar {
    let m=c.mundo.read().await;
    pw_gs::EstadoParaGravar::do_jogador(&m.players[&(c.personagem as i64)],m.world_id,&m.char_repo)
}
async fn recusar_listas(c:&Cenario,ligar:bool) {
    let nome=format!("b170_{}",c.personagem);
    if ligar {
        sqlx::query(&format!("CREATE FUNCTION {nome}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.character_id={} THEN RAISE EXCEPTION 'falha de teste B170'; END IF; RETURN NEW; END $$",c.personagem)).execute(c.pool.get_ref()).await.unwrap();
        sqlx::query(&format!("CREATE TRIGGER {nome} BEFORE INSERT OR UPDATE ON character_task_lists FOR EACH ROW EXECUTE FUNCTION {nome}()"))
            .execute(c.pool.get_ref()).await.unwrap();
    } else {
        sqlx::query(&format!("DROP TRIGGER {nome} ON character_task_lists")).execute(c.pool.get_ref()).await.unwrap();
        sqlx::query(&format!("DROP FUNCTION {nome}()")).execute(c.pool.get_ref()).await.unwrap();
    }
}

#[tokio::test]
async fn saida_grava_126_e_155_e_autosave_antigo_nao_sobrepoe_reentrada() {
    for versao in [GameVersion::V1_2_6,GameVersion::V1_5_5] {
        let c=Cenario::montar_versao(versao).await;
        let mut bus=c.entrar().await;
        c.esperar_presenca("online").await;
        let opaco=[99i32.to_le_bytes(),42i32.to_le_bytes()].concat();
        sqlx::query("UPDATE characters SET character_mode=$2 WHERE id=$1").bind(c.personagem).bind(&opaco)
            .execute(c.pool.get_ref()).await.unwrap();
        let antiga=fotografia(&c).await;
        {
            let mut m=c.mundo.write().await;
            let j=m.players.get_mut(&(c.personagem as i64)).unwrap();
            j.money=4321; j.exp=987; j.ap=47; j.max_ap=299;
            j.strength=33; j.pontos_de_atributo=8; j.modo_roupa=true; j.waypoints=vec![1,256,513];
            j.position=pw_core::Vector3::new(101.,102.,103.);
        }
        bus.enviar(BusMessage::ClientToGame{roleid:c.personagem,localsid:77,data:vec![1,0,1,0,0,0]}).await.unwrap();
        let resposta=tokio::time::timeout(Duration::from_secs(5),async {
            loop {if let Some(BusMessage::PlayerLogout{result,..})=bus.receber().await.unwrap() {break result;}}
        }).await.unwrap();
        assert_eq!(resposta,1);
        c.esperar_presenca("ausente").await;
        let repo=c.mundo.read().await.char_repo.clone();
        pw_gs::gravar_autosave(repo.clone(),vec![antiga.clone()],1).await;
        let d=repo.get_details_por_role(c.personagem).await.unwrap().unwrap();
        assert_eq!((d.money,d.exp,d.ap,d.max_ap,d.strength),(4321,987,47,299,33));
        assert!(d.modo_roupa); assert_eq!(d.waypoints,vec![1,256,513]);
        let cru:Vec<u8>=sqlx::query_scalar("SELECT character_mode FROM characters WHERE id=$1").bind(c.personagem).fetch_one(c.pool.get_ref()).await.unwrap();
        assert!(cru.starts_with(&opaco),"fotografia conserva pares desconhecidos de charactermode");
        let nova=c.entrar().await;
        c.esperar_presenca("online").await;
        pw_gs::gravar_autosave(repo,vec![antiga],1).await;
        drop(bus); // limpeza atrasada da conexão anterior não encerra a nova sessão.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(c.consultar().await["dados"]["presenca"],"online");
        assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].money,4321);
        drop(nova); c.esperar_presenca("ausente").await;
        c.encerrar().await;
    }
}

#[tokio::test]
async fn conexao_duplicada_nao_toma_nem_encerra_sessao_do_dono() {
    let c=Cenario::montar().await;
    let mut dono=c.entrar().await; c.esperar_presenca("online").await;
    dono.enviar(BusMessage::PlayerLogout{result:0,roleid:c.personagem,provider_link_id:1,localsid:78}).await.unwrap();
    dono.enviar(BusMessage::ClientToGame{roleid:c.personagem,localsid:78,data:pw_gs::comandos::ids::GM_INVINCIBLE.to_le_bytes().to_vec()}).await.unwrap();
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(!c.mundo.read().await.players[&(c.personagem as i64)].efeitos.gm_invencivel);
    c.mundo.write().await.players.get_mut(&(c.personagem as i64)).unwrap().money=654;
    let mut outra=c.entrar().await;
    outra.enviar(BusMessage::PlayerLogout{result:0,roleid:c.personagem,provider_link_id:1,localsid:77}).await.unwrap();
    drop(outra);
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(c.consultar().await["dados"]["presenca"],"online");
    assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].money,654);
    drop(dono); c.esperar_presenca("ausente").await; c.encerrar().await;
}

#[tokio::test]
async fn falha_de_saida_preserva_fotografia_bloqueia_login_e_recupera_sem_tick_trancado() {
    let c=Cenario::montar().await;
    let bus=c.entrar().await; c.esperar_presenca("online").await;
    let repo=c.mundo.read().await.char_repo.clone();
    let antes=repo.get_details_por_role(c.personagem).await.unwrap().unwrap().money;
    c.mundo.write().await.players.get_mut(&(c.personagem as i64)).unwrap().money=741;
    recusar_listas(&c,true).await;
    drop(bus);
    for _ in 0..100 {
        if !c.mundo.read().await.players.contains_key(&(c.personagem as i64)) {break;}
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!c.mundo.read().await.players.contains_key(&(c.personagem as i64)));
    c.esperar_presenca("em_transicao").await;
    assert_eq!(repo.get_details_por_role(c.personagem).await.unwrap().unwrap().money,antes); // rollback inclui status
    let outra=c.entrar().await;
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(!c.mundo.read().await.players.contains_key(&(c.personagem as i64)));
    tokio::time::timeout(Duration::from_millis(300),async {c.mundo.write().await.tick(50).await;}).await.unwrap();
    drop(outra);
    recusar_listas(&c,false).await;
    tokio::time::timeout(Duration::from_secs(5),async {
        loop {if c.consultar().await["dados"]["presenca"]=="ausente" {break;} tokio::time::sleep(Duration::from_millis(30)).await;}
    }).await.unwrap();
    assert_eq!(repo.get_details_por_role(c.personagem).await.unwrap().unwrap().money,741);
    let nova=c.entrar().await; c.esperar_presenca("online").await;
    assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].money,741);
    drop(nova); c.esperar_presenca("ausente").await; c.encerrar().await;
}

#[tokio::test]
async fn transferencia_falha_volta_a_origem_e_fotografia_antiga_nao_desfaz_destino() {
    let c=Cenario::montar().await;
    let bus=c.entrar().await; c.esperar_presenca("online").await;
    let aliado=c.personagem+1_000_000_000;
    {
        let mut m=c.mundo.write().await;
        let mut j=m.players[&(c.personagem as i64)].clone();j.role_id=aliado;m.add_player(j);
        assert!(m.convidar_para_grupo(c.personagem,aliado));
        m.aceitar_convite(aliado,c.personagem).unwrap();
    }
    let antiga=fotografia(&c).await;
    recusar_listas(&c,true).await;
    c.roteador.transportar(c.personagem,161,pw_core::Vector3::new(100.,0.,100.)).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(c.roteador.mapa_de(c.personagem).await,Some(1));
    assert!(c.mundo.read().await.players.contains_key(&(c.personagem as i64)));
    assert_eq!(c.mundo.read().await.membros_do_grupo(c.personagem).len(),2,"falha não efetiva saída de grupo");
    let repo=c.mundo.read().await.char_repo.clone();
    assert_eq!(repo.mundo_do_personagem(c.personagem).await.unwrap(),Some(1));
    recusar_listas(&c,false).await;
    c.roteador.transportar(c.personagem,161,pw_core::Vector3::new(100.,0.,100.)).await;
    tokio::time::timeout(Duration::from_secs(3),async {loop {
        if c.roteador.mapa_de(c.personagem).await==Some(161) {break;} tokio::time::sleep(Duration::from_millis(10)).await;
    }}).await.unwrap();
    pw_gs::gravar_autosave(repo.clone(),vec![antiga],1).await;
    assert_eq!(repo.mundo_do_personagem(c.personagem).await.unwrap(),Some(161));
    drop(bus); c.esperar_presenca("ausente").await; c.encerrar().await;
}


#[tokio::test]
async fn confirmacao_incerta_de_transferencia_nao_libera_login_nem_ressuscita_link_caido() {
    let c=Cenario::montar().await;
    let bus=c.entrar().await; c.esperar_presenca("online").await;
    c.mundo.write().await.players.get_mut(&(c.personagem as i64)).unwrap().money=852;
    let nome=format!("b170_commit_{}",c.personagem);
    sqlx::query(&format!("CREATE FUNCTION {nome}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.character_id={} THEN RAISE EXCEPTION 'falha no commit de teste B170'; END IF; RETURN NEW; END $$",c.personagem)).execute(c.pool.get_ref()).await.unwrap();
    sqlx::query(&format!("CREATE CONSTRAINT TRIGGER {nome} AFTER INSERT OR UPDATE ON character_task_lists DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION {nome}()"))
        .execute(c.pool.get_ref()).await.unwrap();
    c.roteador.transportar(c.personagem,161,pw_core::Vector3::new(100.,0.,100.)).await;
    c.esperar_presenca("em_transicao").await;
    drop(bus);
    let outra=c.entrar().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(c.roteador.mapa_de(c.personagem).await,Some(1));
    assert!(!c.mundo.read().await.players.contains_key(&(c.personagem as i64)));
    drop(outra);
    sqlx::query(&format!("DROP TRIGGER {nome} ON character_task_lists")).execute(c.pool.get_ref()).await.unwrap();
    sqlx::query(&format!("DROP FUNCTION {nome}()")).execute(c.pool.get_ref()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5),async {loop {
        if c.consultar().await["dados"]["presenca"]=="ausente" {break;} tokio::time::sleep(Duration::from_millis(30)).await;
    }}).await.unwrap();
    let repo=c.mundo.read().await.char_repo.clone();
    let d=repo.get_details_por_role(c.personagem).await.unwrap().unwrap();
    assert_eq!((d.world_id,d.money),(161,852));
    c.encerrar().await;
}
