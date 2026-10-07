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

/// A vez de cada teste no banco (B194). `entrada_em_andamento_nao_parece_offline` trava a tabela
/// `characters` inteira; com o arquivo em paralelo isso estoura o prazo das consultas dos outros
/// testes ("unexpected end of file" em testes variados). O exclusivo só entra com **nenhum**
/// cenário ativo e, enquanto roda, os novos esperam; um teste que monta dois cenários nunca fica
/// preso no meio (o exclusivo não entra enquanto houver um ativo).
static VEZ: std::sync::Mutex<(usize, bool)> = std::sync::Mutex::new((0, false));

struct Vez {
    exclusiva: bool,
}

impl Vez {
    async fn pegar(exclusiva: bool) -> Self {
        loop {
            {
                let mut v = VEZ.lock().unwrap();
                if exclusiva && v.0 == 0 && !v.1 {
                    v.1 = true;
                    return Self { exclusiva };
                }
                if !exclusiva && !v.1 {
                    v.0 += 1;
                    return Self { exclusiva };
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

impl Drop for Vez {
    fn drop(&mut self) {
        let mut v = VEZ.lock().unwrap();
        if self.exclusiva { v.1 = false } else { v.0 -= 1 }
    }
}

struct Cenario {
    _vez: Vez,
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
        Self::montar_com(versao, false).await
    }

    /// Para o teste que trava a tabela inteira: roda sozinho no arquivo.
    async fn montar_exclusivo() -> Self {
        Self::montar_com(GameVersion::V1_2_6, true).await
    }

    async fn montar_com(versao: GameVersion, exclusiva: bool) -> Self {
        let vez = Vez::pegar(exclusiva).await;
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
        // Dois itens conhecidos (B186): 3001 comum, pilha 100; 3002 de missão, pilha 10.
        let mut dados = GameDataManager::new();
        dados.nomes_de_itens.insert(3001, "Poção de teste".into());
        dados.nomes_de_itens.insert(3002, "Carta de missão".into());
        dados.pilhas.insert(3001, 100);
        dados.pilhas.insert(3002, 10);
        dados.itens_de_missao.insert(3002);
        // B191: 3003 é uma arma (máscara `0x1`, só o slot 0 do corpo); o resto não entra no corpo.
        dados.nomes_de_itens.insert(3003, "Espada de teste".into());
        dados.pilhas.insert(3003, 1);
        dados.posicoes.insert(3003, 0x1);
        let itens_de_teste = Arc::new(dados);
        let mut mapas = Vec::new();
        for tag in [1, 161] {
            let mundo = Arc::new(RwLock::new(WorldInstance::new(
                tag,
                Arc::clone(&itens_de_teste),
                repo.clone(),
            )));
            mapas.push((tag, Arc::new(BusServer::new(mundo, versao))));
        }
        let mundo = Arc::clone(mapas[0].1.mundo());
        let roteador = Arc::new(RoteadorDeMapas::new(mapas, repo));
        // Carga em execução (B183): 1, 161 e 105 "têm dados" (pasta inexistente: os
        // leitores de terreno toleram a ausência e o mapa sobe vazio).
        let mut dados = GameDataManager::new();
        for t in [1, 161, 105] {
            dados.pastas_de_mapa.insert(t, std::env::temp_dir().join("pw_teste_mapa_sem_pasta"));
        }
        roteador.permitir_carga(pw_gs::mapas::CargaDeMapas { dados: Arc::new(dados), versao });
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
            _vez: vez,
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
        _vez: Vez::pegar(false).await,
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
        json!([{"mapa":1,"jogadores_online":1,"ligado":true},{"mapa":161,"jogadores_online":0,"ligado":true}])
    );
    c.encerrar().await;
}

#[tokio::test]
async fn entrada_em_andamento_nao_parece_offline() {
    let c = Cenario::montar_exclusivo().await;
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
    for _ in 0..500 {
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
        for _ in 0..500 { if c.mundo.read().await.players.contains_key(&(c.personagem as i64)) {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
        assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].sec_level,1);
    }
    for (c,b) in [(&c1,&mut b1),(&c2,&mut b2)] {
        b.enviar(BusMessage::ClientToGame{roleid:c.personagem,localsid:77,data:pw_gs::comandos::ids::GM_INVINCIBLE.to_le_bytes().to_vec()}).await.unwrap();
        for _ in 0..500 { if c.mundo.read().await.players[&(c.personagem as i64)].efeitos.gm_invencivel {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
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
    for _ in 0..500 { if repo.consultar(&id,admin).await.unwrap()["estado"]=="aplicado" {break;} tokio::time::sleep(Duration::from_millis(10)).await; }
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
    for _ in 0..500 {if repo.consultar(&id2,admin).await.unwrap()["estado"]=="aplicado"{break;}tokio::time::sleep(Duration::from_millis(10)).await;}
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
    for _ in 0..500 {
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

/// Lê do link até chegar a mensagem pedida (o mundo manda muita coisa ao entrar).
async fn esperar_do_mundo(bus: &mut pw_bus::transport::BusConnection, quer: impl Fn(&BusMessage) -> bool) -> BusMessage {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let m = bus.receber().await.unwrap().expect("o link caiu");
            if quer(&m) { return m; }
        }
    }).await.expect("o mundo não mandou a mensagem esperada")
}

/// E4 (B175): desconectar pelo painel salva e tira a entidade como o logout, e o link
/// recebe `PlayerLogout` com result 2 — o cliente volta à tela de login
/// (`EC_GameSession.cpp:5420-5426`). Nas duas versões; conta sem ninguém online = 0.
#[tokio::test]
async fn desconectar_pelo_painel_salva_e_devolve_ao_login_126_e_155() {
    for versao in [GameVersion::V1_2_6, GameVersion::V1_5_5] {
        let c = Cenario::montar_versao(versao).await;
        let mut bus = c.entrar().await;
        c.esperar_presenca("online").await;
        let r = c.pedir(&c.realm, json!({"tipo":"desconectar","conta_id":c.conta})).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["desconectados"].as_i64()), (Some("aplicado"), Some(1)), "{r}");
        let saida = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::PlayerLogout { .. })).await;
        assert!(matches!(saida, BusMessage::PlayerLogout { result: 2, roleid, .. } if roleid == c.personagem), "{saida:?}");
        c.esperar_presenca("ausente").await;
        assert!(!c.mundo.read().await.players.contains_key(&(c.personagem as i64)));
        let de_novo = c.pedir(&c.realm, json!({"tipo":"desconectar","conta_id":c.conta})).await;
        assert_eq!(de_novo["dados"]["desconectados"], 0);
        c.encerrar().await;
    }
}

/// E4 (B175): depois de o gold mudar, o mundo reenvia o saldo a quem está online.
#[tokio::test]
async fn atualizar_cash_reenvia_o_saldo_a_quem_esta_online() {
    let c = Cenario::montar().await;
    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let r = c.pedir(&c.realm, json!({"tipo":"atualizar_cash","conta_id":c.conta})).await;
    assert_eq!(r["dados"]["atualizados"], 1, "{r}");
    // `PLAYER_CASH` (253) com um `int` (`s2c.rs::player_cash`): 2 B de comando + 4 B.
    let cash = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { roleid, data, .. }
        if *roleid == c.personagem && data.len() == 6 && data[..2] == 253u16.to_le_bytes())).await;
    let saldo: i64 = sqlx::query_scalar("SELECT gold_balance FROM accounts WHERE id=$1")
        .bind(c.conta).fetch_one(c.pool.get_ref()).await.unwrap();
    let BusMessage::GameToClient { data, .. } = cash else { unreachable!() };
    assert_eq!(i32::from_le_bytes(data[2..6].try_into().unwrap()) as i64, saldo.clamp(0, i32::MAX as i64));
    c.encerrar().await;
}

/// E7 (B176): rates do realm pelo canal — gravadas em `realms`, valendo na hora no
/// processo (o resumo dos mundos mostra as rates em memória) e recusadas fora dos limites.
#[tokio::test]
async fn rates_do_realm_gravam_no_banco_e_valem_na_hora() {
    let c = Cenario::montar().await;
    let r = c.pedir(&c.realm, json!({"tipo":"definir_taxas","exp":2.0,"sp":1.5,"drop":1.5,"moedas":3.0})).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    let banco: (f32, f32, f32, f32) = sqlx::query_as("SELECT double_exp_multiplier::float4,double_sp_multiplier::float4,double_drop_multiplier::float4,double_gold_multiplier::float4 FROM realms WHERE id=$1")
        .bind(&c.realm).fetch_one(c.pool.get_ref()).await.unwrap();
    assert_eq!(banco, (2.0, 1.5, 1.5, 3.0));
    assert_eq!(c.roteador.taxas(), pw_gs::taxas::Taxas { exp: 2.0, sp: 1.5, drop: 1.5, moedas: 3.0 });
    let resumo = c.pedir(&c.realm, json!({"tipo":"mundos"})).await;
    assert_eq!(resumo["dados"]["taxas"]["moedas"], 3.0, "{resumo}");
    let invalida = c.pedir(&c.realm, json!({"tipo":"definir_taxas","exp":0.0,"sp":1.0,"drop":1.0,"moedas":1.0})).await;
    assert_eq!(invalida["dados"]["codigo"], "taxas_invalidas");
    assert_eq!(c.roteador.taxas().exp, 2.0, "valor recusado não muda o que vale");
    // Reinício: um processo novo lê as rates do banco.
    c.roteador.definir_taxas(pw_gs::taxas::Taxas::default());
    c.roteador.carregar_taxas(&AccountRepository::new(c.pool.clone()).realms(), &c.realm).await;
    assert_eq!(c.roteador.taxas().drop, 1.5);
    c.encerrar().await;
}

/// E7 (B177, B183): desligar um mapa tira (salvando) quem está nele, recusa novas entradas
/// com `PlayerLogout` result 2 e **descarrega** o mapa; o estado vai para
/// `realms.config.mapas_desligados`/`mapas_ligados` sem apagar as outras chaves. Religar um
/// mapa descarregado só com `carregar` (o painel manda a um processo só): monta em segundo
/// plano e devolve o acesso. Mapa sem dados é recusado; a partida relê o desligado.
#[tokio::test]
async fn desligar_descarrega_e_ligar_carrega_o_mapa_na_hora() {
    let c = Cenario::montar().await;
    let mapa = c.roteador.mapas()[0];
    sqlx::query("UPDATE realms SET config='{\"max_level\":105}'::jsonb WHERE id=$1").bind(&c.realm).execute(c.pool.get_ref()).await.unwrap();
    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let config = || async {
        sqlx::query_scalar::<_, Value>("SELECT config FROM realms WHERE id=$1").bind(&c.realm).fetch_one(c.pool.get_ref()).await.unwrap()
    };
    let carregados = || async {
        let r = c.pedir(&c.realm, json!({"tipo":"mundos"})).await;
        r["dados"]["mapas"].as_array().unwrap().iter()
            .map(|m| (m["mapa"].as_i64().unwrap() as i32, m["carregando"].as_bool().unwrap_or(false)))
            .collect::<Vec<_>>()
    };
    let esperar_carregado = |alvo: i32| async move {
        for _ in 0..100 {
            if carregados().await.contains(&(alvo, false)) { return; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("o mapa {alvo} não terminou de carregar");
    };

    let r = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":mapa,"ligado":false})).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["desconectados"].as_i64(), r["dados"]["descarregado"].as_bool()),
        (Some("aplicado"), Some(1), Some(true)), "{r}");
    let saida = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::PlayerLogout { .. })).await;
    assert!(matches!(saida, BusMessage::PlayerLogout { result: 2, .. }), "{saida:?}");
    c.esperar_presenca("ausente").await;
    assert_eq!(config().await, json!({"max_level":105,"mapas_desligados":[mapa],"mapas_ligados":[]}), "outras chaves preservadas");
    assert!(!carregados().await.iter().any(|(m, _)| *m == mapa), "descarregado");
    assert!(!c.roteador.mapas().contains(&mapa));

    // Entrada no padrão desligado: recusada na hora.
    let mut de_novo = c.entrar().await;
    let recusa = esperar_do_mundo(&mut de_novo, |m| matches!(m, BusMessage::PlayerLogout { .. })).await;
    assert!(matches!(recusa, BusMessage::PlayerLogout { result: 2, .. }), "{recusa:?}");
    drop(de_novo);
    drop(bus);

    // Descarregado, só aceita ligar com `carregar`; então monta e devolve o acesso.
    let r = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":mapa,"ligado":true})).await;
    assert_eq!(r["dados"]["codigo"], "mapa_nao_servido", "{r}");
    let r = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":mapa,"ligado":true,"carregar":true})).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["carregando"].as_bool()), (Some("aplicado"), Some(true)), "{r}");
    esperar_carregado(mapa).await;
    assert_eq!(config().await["mapas_desligados"], json!([]));
    assert_eq!(config().await["mapas_ligados"], json!([mapa]));
    let _bus = c.entrar().await;
    c.esperar_presenca("online").await;

    // Um mapa fora do `WORLD_TAGS`, com dados: carrega e descarrega sem ninguém nele.
    let r = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":105,"ligado":true,"carregar":true})).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    esperar_carregado(105).await;
    let r = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":105,"ligado":false})).await;
    assert_eq!((r["dados"]["desconectados"].as_i64(), r["dados"]["descarregado"].as_bool()), (Some(0), Some(true)), "{r}");
    assert!(!c.roteador.mapas().contains(&105));
    let resumo = c.pedir(&c.realm, json!({"tipo":"mundos"})).await;
    assert_eq!(resumo["dados"]["carregaveis"], json!([1, 105, 161]), "{resumo}");

    let sem_dados = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":999,"ligado":true,"carregar":true})).await;
    assert_eq!(sem_dados["dados"]["codigo"], "mapa_sem_dados");
    let fora = c.pedir(&c.realm, json!({"tipo":"definir_mapa","mapa":999_999,"ligado":false})).await;
    assert_eq!(fora["dados"]["codigo"], "mapa_nao_servido");

    // A partida relê o desligado do banco.
    sqlx::query("UPDATE realms SET config=jsonb_set(config,'{mapas_desligados}',to_jsonb(ARRAY[$2::int])) WHERE id=$1").bind(&c.realm).bind(161).execute(c.pool.get_ref()).await.unwrap();
    c.roteador.carregar_mapas_desligados(&AccountRepository::new(c.pool.clone()).realms(), &c.realm).await;
    assert!(!c.roteador.mapa_ligado(161));
    c.encerrar().await;
}

/// E5 (B179): edição de personagem pelo painel. Online: o mapa aplica na memória, avisa o
/// cliente pelos pacotes da recompensa de missão (159 dinheiro, 158 EXP) e grava; o mesmo
/// ID não aplica de novo. Offline: só dinheiro, no banco; EXP exige o personagem online.
#[tokio::test]
async fn editar_personagem_online_e_offline_pelo_canal() {
    let c = Cenario::montar().await;
    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let antes = c.mundo.read().await.players[&(c.personagem as i64)].money;

    let pedido = json!({"tipo":"editar_personagem","personagem_id":c.personagem,"dinheiro":500});
    let r = c.pedir_com_id(&c.realm, &format!("e5-din-1-{}", c.personagem), c.conta, pedido.clone()).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("aplicado"), Some("online")), "{r}");
    assert_eq!(r["dados"]["dinheiro"], (antes + 500).to_string());
    esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 159u16.to_le_bytes())).await;
    assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].money, antes + 500);
    // Mesmo ID: devolve o resultado gravado, não dá o dinheiro de novo.
    let repetido = c.pedir_com_id(&c.realm, &format!("e5-din-1-{}", c.personagem), c.conta, pedido).await;
    assert_eq!(repetido["dados"]["dinheiro"], (antes + 500).to_string());
    assert_eq!(c.mundo.read().await.players[&(c.personagem as i64)].money, antes + 500);
    let conflito = c.pedir_com_id(&c.realm, &format!("e5-din-1-{}", c.personagem), c.conta,
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"dinheiro":7})).await;
    assert_eq!(conflito["dados"]["codigo"], "operacao_em_conflito");
    // Gravado pelo contexto, como a recompensa de missão.
    let gravado: i64 = sqlx::query_scalar("SELECT money FROM characters WHERE id=$1").bind(c.personagem)
        .fetch_one(c.pool.get_ref()).await.unwrap();
    assert_eq!(gravado, antes + 500);

    let exp_antes = c.mundo.read().await.players[&(c.personagem as i64)].exp;
    let r = c.pedir_com_id(&c.realm, &format!("e5-exp-1-{}", c.personagem), c.conta,
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"exp":10,"sp":3})).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 158u16.to_le_bytes())).await;
    assert!(c.mundo.read().await.players[&(c.personagem as i64)].exp >= exp_antes + 10
        || c.mundo.read().await.players[&(c.personagem as i64)].level > 1, "a EXP não entrou");

    drop(bus);
    c.esperar_presenca("ausente").await;
    let r = c.pedir_com_id(&c.realm, &format!("e5-din-off-{}", c.personagem), c.conta,
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"dinheiro":100})).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
    assert_eq!(r["dados"]["dinheiro"], (antes + 600).to_string());
    let r = c.pedir_com_id(&c.realm, &format!("e5-exp-off-{}", c.personagem), c.conta,
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"exp":10})).await;
    assert_eq!(r["dados"]["codigo"], "precisa_estar_online");
    // Só dar (B180): tirar dinheiro é edição inválida, online ou offline.
    let r = c.pedir_com_id(&c.realm, &format!("e5-din-neg-{}", c.personagem), c.conta,
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"dinheiro":-100})).await;
    assert_eq!(r["dados"]["codigo"], "edicao_invalida");
    let invalida = c.pedir_com_id(&c.realm, &format!("e5-inv-{}", c.personagem), c.conta,
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"dinheiro":5,"exp":1})).await;
    assert_eq!(invalida["dados"]["codigo"], "edicao_invalida");
    sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e5-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// E5 (B182): pontos livres, nível direto e cultivo. Online o mapa aplica e avisa o
/// cliente — `ADD_STATUS_POINT` (51) com os quatro em zero, um `LEVEL_UP` (37) por nível,
/// `TASK_DELIVER_LEVEL2` (160) — e grava; offline grava no banco. Nível só sobe; cultivo
/// conforme a versão (20–22/30–32 só no 1.5.5).
#[tokio::test]
async fn editar_pontos_nivel_e_cultivo_online_e_offline_126_e_155() {
    for versao in [GameVersion::V1_2_6, GameVersion::V1_5_5] {
        let c = Cenario::montar_versao(versao).await;
        let id = |s: &str| format!("e5b-{s}-{}", c.personagem);
        let editar = |campos: Value| {
            let mut p = json!({"tipo":"editar_personagem","personagem_id":c.personagem});
            for (k, v) in campos.as_object().unwrap() { p[k] = v.clone(); }
            p
        };
        let jogador = |m: &WorldInstance| {
            let p = &m.players[&(c.personagem as i64)];
            (p.level, p.pontos_de_atributo, p.cultivation)
        };
        let mut bus = c.entrar().await;
        c.esperar_presenca("online").await;
        let (nivel, pontos, _) = jogador(&*c.mundo.read().await);

        let r = c.pedir_com_id(&c.realm, &id("pts"), c.conta, editar(json!({"pontos":7}))).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("aplicado"), Some("online")), "{r}");
        let m = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 51u16.to_le_bytes())).await;
        let BusMessage::GameToClient { data, .. } = m else { unreachable!() };
        assert_eq!(data.len(), 22, "ADD_STATUS_POINT tem 22 bytes");
        assert_eq!(&data[2..18], &[0u8; 16], "os quatro atributos em zero");
        assert_eq!(u32::from_le_bytes(data[18..22].try_into().unwrap()) as i32, pontos + 7);

        let r = c.pedir_com_id(&c.realm, &id("niv"), c.conta, editar(json!({"nivel": nivel + 3}))).await;
        assert_eq!(r["estado"], "aplicado", "{r}");
        for _ in 0..3 {
            esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 37u16.to_le_bytes())).await;
        }
        assert_eq!(jogador(&*c.mundo.read().await).0, nivel + 3);
        assert_eq!(jogador(&*c.mundo.read().await).1, pontos + 7 + 15, "cinco pontos por nível");
        let r = c.pedir_com_id(&c.realm, &id("niv-baixo"), c.conta, editar(json!({"nivel": nivel + 1}))).await;
        assert_eq!(r["dados"]["codigo"], "nivel_invalido", "nível só sobe: {r}");

        let r = c.pedir_com_id(&c.realm, &id("cul"), c.conta, editar(json!({"cultivo": 5}))).await;
        assert_eq!(r["estado"], "aplicado", "{r}");
        esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 160u16.to_le_bytes())).await;
        assert_eq!(jogador(&*c.mundo.read().await).2, 5);
        let r = c.pedir_com_id(&c.realm, &id("cul-deus"), c.conta, editar(json!({"cultivo": 22}))).await;
        if versao == GameVersion::V1_2_6 {
            assert_eq!(r["dados"]["codigo"], "cultivo_invalido", "{r}");
        } else {
            assert_eq!(r["estado"], "aplicado", "{r}");
        }
        let r = c.pedir_com_id(&c.realm, &id("cul-9"), c.conta, editar(json!({"cultivo": 9}))).await;
        assert_eq!(r["dados"]["codigo"], "cultivo_invalido");
        let r = c.pedir_com_id(&c.realm, &id("dois"), c.conta, editar(json!({"pontos": 1, "nivel": 90}))).await;
        assert_eq!(r["dados"]["codigo"], "edicao_invalida");

        drop(bus);
        c.esperar_presenca("ausente").await;
        let gravado: (i32, i32, i32) = sqlx::query_as("SELECT level, potential_points, cultivation FROM characters WHERE id=$1")
            .bind(c.personagem).fetch_one(c.pool.get_ref()).await.unwrap();
        let cultivo_online = if versao == GameVersion::V1_2_6 { 5 } else { 22 };
        assert_eq!(gravado, (nivel + 3, pontos + 22, cultivo_online), "o contexto gravou");

        let r = c.pedir_com_id(&c.realm, &id("pts-off"), c.conta, editar(json!({"pontos":4}))).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
        assert_eq!(r["dados"]["pontos_livres"], pontos + 26);
        let r = c.pedir_com_id(&c.realm, &id("niv-off"), c.conta, editar(json!({"nivel": nivel + 5}))).await;
        assert_eq!(r["estado"], "salvo", "{r}");
        assert_eq!((r["dados"]["nivel"].as_i64(), r["dados"]["pontos_livres"].as_i64()),
            (Some((nivel + 5) as i64), Some((pontos + 36) as i64)));
        let r = c.pedir_com_id(&c.realm, &id("niv-off-baixo"), c.conta, editar(json!({"nivel": nivel + 5}))).await;
        assert_eq!(r["dados"]["codigo"], "nivel_invalido_ou_personagem_inexistente");
        let r = c.pedir_com_id(&c.realm, &id("niv-teto"), c.conta, editar(json!({"nivel": 100_000}))).await;
        assert_eq!(r["dados"]["codigo"], "nivel_invalido");
        let r = c.pedir_com_id(&c.realm, &id("cul-off"), c.conta, editar(json!({"cultivo": 8}))).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["cultivo"].as_i64()), (Some("salvo"), Some(8)), "{r}");
        sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e5b-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
        c.encerrar().await;
    }
}

/// E5 (B184): modificar os atributos já distribuídos e redistribuir, com o total (atributos +
/// pontos livres) conservado e o piso da versão (`piso_da_restauracao`: 1.2.6 3 em vitalidade e
/// energia, 1.5.5 5 nos quatro). Online o mapa aplica e manda a ficha (`OWN_EXT_PROP` 50);
/// offline grava no banco com as mesmas condições.
#[tokio::test]
async fn modificar_e_redistribuir_atributos_online_e_offline_126_e_155() {
    for versao in [GameVersion::V1_2_6, GameVersion::V1_5_5] {
        let c = Cenario::montar_versao(versao).await;
        let id = |s: &str| format!("e5c-{s}-{}", c.personagem);
        let editar = |campos: Value| {
            let mut p = json!({"tipo":"editar_personagem","personagem_id":c.personagem});
            for (k, v) in campos.as_object().unwrap() { p[k] = v.clone(); }
            p
        };
        // Partida conhecida: 10 em cada e 20 livres (total 60).
        sqlx::query("UPDATE characters SET strength=10, agility=10, vitality=10, energy=10, potential_points=20 WHERE id=$1")
            .bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
        let piso = if versao == GameVersion::V1_2_6 { [5, 5, 3, 3] } else { [5, 5, 5, 5] };
        let mut bus = c.entrar().await;
        c.esperar_presenca("online").await;
        let atributos = || async {
            let m = c.roteador.consultar_administrativamente(c.personagem).await;
            assert_eq!(m["presenca"], "online", "{m}");
            let mundo = c.mundo.read().await;
            let p = &mundo.players[&(c.personagem as i64)];
            [p.strength, p.agility, p.vitality, p.energy, p.pontos_de_atributo]
        };
        assert_eq!(atributos().await, [10, 10, 10, 10, 20]);

        // Tira da força, põe na vitalidade e gasta livres: 6+12+30+10 = 58, sobram 2.
        let r = c.pedir_com_id(&c.realm, &id("mod"), c.conta, editar(json!({"atributos":[6, 12, 30, 10]}))).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["pontos_livres"].as_i64()), (Some("aplicado"), Some(2)), "{r}");
        let m = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 50u16.to_le_bytes())).await;
        assert!(matches!(m, BusMessage::GameToClient { .. }));
        assert_eq!(atributos().await, [6, 12, 30, 10, 2]);
        // Soma acima do total, abaixo do piso, ou igual ao atual: recusados sem mudar nada.
        for (n, alvo, codigo) in [(1, json!([6, 12, 30, 13]), "atributos_invalidos"),
                                  (2, json!([4, 12, 30, 10]), "atributos_invalidos"),
                                  (3, json!([6, 12, 30, 10]), "sem_mudanca")] {
            let r = c.pedir_com_id(&c.realm, &id(&format!("rec{n}")), c.conta, editar(json!({"atributos": alvo}))).await;
            assert_eq!(r["dados"]["codigo"], codigo, "{r}");
        }
        assert_eq!(atributos().await, [6, 12, 30, 10, 2]);

        // Redistribuir: todos ao piso, o resto aos livres.
        let r = c.pedir_com_id(&c.realm, &id("red"), c.conta, editar(json!({"redistribuir": true}))).await;
        assert_eq!(r["estado"], "aplicado", "{r}");
        let livres = 60 - piso.iter().sum::<i32>();
        assert_eq!(atributos().await, [piso[0], piso[1], piso[2], piso[3], livres]);
        let r = c.pedir_com_id(&c.realm, &id("dois"), c.conta, editar(json!({"redistribuir": true, "pontos": 1}))).await;
        assert_eq!(r["dados"]["codigo"], "edicao_invalida");
        let r = c.pedir_com_id(&c.realm, &id("tres"), c.conta, editar(json!({"atributos": [1, 2, 3]}))).await;
        assert_eq!(r["dados"]["codigo"], "edicao_invalida");

        drop(bus);
        c.esperar_presenca("ausente").await;
        let gravado: (i32, i32, i32, i32, i32) = sqlx::query_as("SELECT strength, agility, vitality, energy, potential_points FROM characters WHERE id=$1")
            .bind(c.personagem).fetch_one(c.pool.get_ref()).await.unwrap();
        assert_eq!(gravado, (piso[0], piso[1], piso[2], piso[3], livres), "o contexto gravou");

        let r = c.pedir_com_id(&c.realm, &id("mod-off"), c.conta, editar(json!({"atributos":[20, 10, 10, 10]}))).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
        assert_eq!((r["dados"]["forca"].as_i64(), r["dados"]["pontos_livres"].as_i64()), (Some(20), Some(10)));
        let r = c.pedir_com_id(&c.realm, &id("rec-off"), c.conta, editar(json!({"atributos":[20, 10, 10, 21]}))).await;
        assert_eq!(r["dados"]["codigo"], "atributos_invalidos_ou_personagem_inexistente", "{r}");
        let r = c.pedir_com_id(&c.realm, &id("red-off"), c.conta, editar(json!({"redistribuir": true}))).await;
        assert_eq!(r["estado"], "salvo", "{r}");
        assert_eq!(r["dados"]["pontos_livres"].as_i64(), Some(livres as i64));
        sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e5c-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
        c.encerrar().await;
    }
}

/// E6 (B185): mover o personagem. Online pelo `transportar` (o do GM): no mesmo mapa a posição
/// muda na hora; noutro, a troca de mapa leva o jogador e grava. Destino só em mapa carregado
/// e ligado; sem terreno a altura é obrigatória. Offline grava mapa e posição.
#[tokio::test]
async fn mover_personagem_online_e_offline_pelo_canal() {
    let c = Cenario::montar().await;
    let id = |s: &str| format!("e6p-{s}-{}", c.personagem);
    let mover = |p: Value| json!({"tipo":"editar_personagem","personagem_id":c.personagem,"posicao":p});
    let _bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let ficha = || async { c.roteador.consultar_administrativamente(c.personagem).await };

    let r = c.pedir_com_id(&c.realm, &id("mesmo"), c.conta, mover(json!({"mapa":1,"x":120.5,"y":30.0,"z":-80.0}))).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["troca"].as_bool()), (Some("aplicado"), Some(false)), "{r}");
    let f = ficha().await;
    assert_eq!((f["ficha"]["mapa"].as_i64(), f["ficha"]["posicao"]["x"].as_f64()), (Some(1), Some(120.5)), "{f}");

    let r = c.pedir_com_id(&c.realm, &id("outro"), c.conta, mover(json!({"mapa":161,"x":10.0,"y":5.0,"z":20.0}))).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["troca"].as_bool()), (Some("aplicado"), Some(true)), "{r}");
    let mut chegou = false;
    for _ in 0..100 {
        if c.roteador.mapa_de(c.personagem).await == Some(161) { chegou = true; break; }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(chegou, "a troca para o 161 não aconteceu");
    let gravado: (i32, f32) = sqlx::query_as("SELECT world_id, pos_x FROM characters WHERE id=$1")
        .bind(c.personagem).fetch_one(c.pool.get_ref()).await.unwrap();
    assert_eq!(gravado, (161, 10.0), "a troca grava na hora");

    for (n, p, codigo) in [(1, json!({"mapa":1,"x":1.0,"z":1.0}), "altura_obrigatoria"),
                           (2, json!({"mapa":999,"x":1.0,"y":1.0,"z":1.0}), "mapa_indisponivel"),
                           (3, json!({"mapa":1,"x":1.0e9,"y":1.0,"z":1.0}), "edicao_invalida")] {
        let r = c.pedir_com_id(&c.realm, &id(&format!("rec{n}")), c.conta, mover(p)).await;
        assert_eq!(r["dados"]["codigo"], codigo, "{n}: {r}");
    }

    drop(_bus);
    c.esperar_presenca("ausente").await;
    let r = c.pedir_com_id(&c.realm, &id("off"), c.conta, mover(json!({"mapa":1,"x":-319.5,"y":220.0,"z":-900.25}))).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
    let gravado: (i32, f32, f32, f32) = sqlx::query_as("SELECT world_id, pos_x, pos_y, pos_z FROM characters WHERE id=$1")
        .bind(c.personagem).fetch_one(c.pool.get_ref()).await.unwrap();
    assert_eq!(gravado, (1, -319.5, 220.0, -900.25));
    sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e6p-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// E6 (B186): ver o inventário, buscar e dar item. Online pelo prêmio de missão
/// (`TASK_DELIVER_ITEM` 156), item de missão na bolsa de missão; se a quantidade inteira não
/// cabe, nada entra. Offline a mesma geração, gravada no banco.
#[tokio::test]
async fn dar_item_e_ver_inventario_online_e_offline() {
    let c = Cenario::montar().await;
    let id = |s: &str| format!("e6i-{s}-{}", c.personagem);
    let dar = |tid: u32, q: u32| json!({"tipo":"editar_personagem","personagem_id":c.personagem,"item":{"id":tid,"quantidade":q}});
    let total = |inv: &Value, onde: &str, tid: u64| inv["dados"]["recipientes"][onde].as_array().unwrap().iter()
        .filter(|i| i["id"].as_u64() == Some(tid)).map(|i| i["quantidade"].as_u64().unwrap()).sum::<u64>();
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();

    let busca = c.pedir(&c.realm, json!({"tipo":"buscar_itens","texto":"poção"})).await;
    assert_eq!(busca["dados"]["itens"], json!([{"id":3001,"nome":"Poção de teste","pilha":100,"missao":false,"icone":""}]), "{busca}");
    let busca = c.pedir(&c.realm, json!({"tipo":"buscar_itens","texto":"3002"})).await;
    assert_eq!(busca["dados"]["itens"][0]["id"], 3002);

    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let r = c.pedir_com_id(&c.realm, &id("on"), c.conta, dar(3001, 150)).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("aplicado"), Some("online")), "{r}");
    esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 156u16.to_le_bytes())).await;
    let r = c.pedir_com_id(&c.realm, &id("missao"), c.conta, dar(3002, 3)).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!((total(&inv, "bolsa", 3001), total(&inv, "missao", 3002)), (150, 3), "{inv}");
    assert_eq!(inv["dados"]["recipientes"]["bolsa"][0]["nome"], "Poção de teste");

    // 33 pilhas de 100 não cabem em 32 slots: nada entra.
    let r = c.pedir_com_id(&c.realm, &id("cheia"), c.conta, dar(3001, 3300)).await;
    assert_eq!(r["dados"]["codigo"], "bolsa_cheia", "{r}");
    let r = c.pedir_com_id(&c.realm, &id("nao-existe"), c.conta, dar(999_999, 1)).await;
    assert_eq!(r["dados"]["codigo"], "item_inexistente", "{r}");
    let r = c.pedir_com_id(&c.realm, &id("zero"), c.conta, dar(3001, 0)).await;
    assert_eq!(r["dados"]["codigo"], "edicao_invalida", "{r}");
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!(total(&inv, "bolsa", 3001), 150, "a recusa não deu nada");

    drop(bus);
    c.esperar_presenca("ausente").await;
    let r = c.pedir_com_id(&c.realm, &id("off"), c.conta, dar(3001, 5)).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
    let r = c.pedir_com_id(&c.realm, &id("off-cheia"), c.conta, dar(3001, 3300)).await;
    assert_eq!(r["dados"]["codigo"], "bolsa_cheia", "{r}");
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!(total(&inv, "bolsa", 3001), 155, "{inv}");
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e6i-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// E6 (B187): tirar item de um slot. Online (bolsa/missão) manda `PLAYER_DROP_ITEM` (46) com
/// `DROP_TYPE_GM` (0); o id confere o slot; equipamento/armazém só offline.
#[tokio::test]
async fn remover_item_online_e_offline() {
    let c = Cenario::montar().await;
    let id = |s: &str| format!("e6r-{s}-{}", c.personagem);
    let tirar = |rec: &str, slot: u16, tid: u32, q: Option<u32>| {
        let mut r = json!({"recipiente":rec,"slot":slot,"id":tid});
        if let Some(q) = q { r["quantidade"] = json!(q); }
        json!({"tipo":"editar_personagem","personagem_id":c.personagem,"remover_item":r})
    };
    let total = |inv: &Value, onde: &str| inv["dados"]["recipientes"][onde].as_array().unwrap().iter()
        .map(|i| i["quantidade"].as_u64().unwrap()).sum::<u64>();
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    // 30 poções na bolsa (slot 0) e uma no armazém (slot 4).
    for (tipo, slot, n) in [(0i32, 0i32, 30i32), (2, 4, 1)] {
        sqlx::query("INSERT INTO character_items(character_id, container_type, slot, item_id, count) VALUES($1,$2::smallint,$3::smallint,3001,$4)")
            .bind(c.personagem).bind(tipo).bind(slot).bind(n).execute(c.pool.get_ref()).await.unwrap();
    }

    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let r = c.pedir_com_id(&c.realm, &id("on"), c.conta, tirar("bolsa", 0, 3001, Some(10))).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    let m = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 46u16.to_le_bytes())).await;
    let BusMessage::GameToClient { data, .. } = m else { unreachable!() };
    // Cenário 1.2.6: `count` em u16 (v126 `player_drop_item`), 2 + 9 bytes.
    assert_eq!(data.len(), 11, "PLAYER_DROP_ITEM do 1.2.6");
    assert_eq!((data[2], data[3], u16::from_le_bytes(data[4..6].try_into().unwrap()), data[10]), (0, 0, 10, 0), "bolsa, slot 0, 10, motivo GM");
    for (n, pedido, codigo) in [(1, tirar("bolsa", 0, 3002, None), "slot_mudou"),
                                (2, tirar("bolsa", 5, 3001, None), "slot_mudou"),
                                (3, tirar("bolsa", 0, 3001, Some(21)), "quantidade_invalida"),
                                (4, tirar("armazem", 4, 3001, None), "precisa_estar_offline"),
                                (5, tirar("bau", 4, 3001, None), "edicao_invalida")] {
        let r = c.pedir_com_id(&c.realm, &id(&format!("rec{n}")), c.conta, pedido).await;
        assert_eq!(r["dados"]["codigo"], codigo, "{n}: {r}");
    }
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!((total(&inv, "bolsa"), total(&inv, "armazem")), (20, 1), "{inv}");

    drop(bus);
    c.esperar_presenca("ausente").await;
    let r = c.pedir_com_id(&c.realm, &id("off-arm"), c.conta, tirar("armazem", 4, 3001, None)).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["removidos"].as_u64()), (Some("salvo"), Some(1)), "{r}");
    let r = c.pedir_com_id(&c.realm, &id("off-bolsa"), c.conta, tirar("bolsa", 0, 3001, None)).await;
    assert_eq!(r["dados"]["removidos"], 20, "{r}");
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!((total(&inv, "bolsa"), total(&inv, "armazem")), (0, 0), "{inv}");
    sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e6r-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// E6 (B189): o detalhe de um slot para a dica — registro com nome, quantidade, pilha;
/// slot vazio e recipiente desconhecido recusados.
#[tokio::test]
async fn detalhe_do_item_para_a_dica() {
    let c = Cenario::montar().await;
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    sqlx::query("INSERT INTO character_items(character_id, container_type, slot, item_id, count) VALUES($1,0::smallint,3::smallint,3001,7)")
        .bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    let pedir = |rec: &str, slot: u16| json!({"tipo":"detalhe_item","personagem_id":c.personagem,"recipiente":rec,"slot":slot});
    let r = c.pedir(&c.realm, pedir("bolsa", 3)).await;
    assert_eq!(r["estado"], "consultado", "{r}");
    assert_eq!((r["dados"]["id"].as_u64(), r["dados"]["nome"].as_str(), r["dados"]["quantidade"].as_u64(), r["dados"]["pilha"].as_u64()),
        (Some(3001), Some("Poção de teste"), Some(7), Some(100)), "{r}");
    assert!(r["dados"]["equipamento"].is_null(), "item comum não tem bloco de equipamento");
    assert_eq!(c.pedir(&c.realm, pedir("bolsa", 4)).await["dados"]["codigo"], "slot_vazio");
    assert_eq!(c.pedir(&c.realm, pedir("bau", 3)).await["dados"]["codigo"], "recipiente_invalido");
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// E6 (B191): arrastar. Online pelos tratadores do cliente (troca na bolsa, vestir, tirar),
/// com a posição no corpo (`CheckEquipPostion`) conferida antes; armazém e bolsa de missão só
/// offline. Offline, a mesma troca no banco em transação. Nenhum item some nem duplica.
#[tokio::test]
async fn arrastar_item_online_e_offline() {
    let c = Cenario::montar().await;
    let id = |s: &str| format!("e6m-{s}-{}", c.personagem);
    let mover = |de: &str, sd: u16, tid: u32, para: &str, sp: u16| json!({"tipo":"editar_personagem",
        "personagem_id":c.personagem,"mover_item":{"de":de,"slot_de":sd,"id":tid,"para":para,"slot_para":sp}});
    let onde = |inv: &Value, rec: &str, slot: u64| inv["dados"]["recipientes"][rec].as_array().unwrap().iter()
        .find(|i| i["slot"].as_u64() == Some(slot)).map(|i| i["id"].as_u64().unwrap());
    let soma = |inv: &Value, tid: u64| ["bolsa", "equipamento", "armazem", "missao"].iter()
        .flat_map(|r| inv["dados"]["recipientes"][*r].as_array().unwrap().clone())
        .filter(|i| i["id"].as_u64() == Some(tid)).map(|i| i["quantidade"].as_u64().unwrap()).sum::<u64>();
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    // Bolsa: 30 poções no 0 e a espada no 1; armazém: 1 poção no 2; missão: a carta no 0.
    for (tipo, slot, tid, n) in [(0i32, 0i32, 3001i32, 30i32), (0, 1, 3003, 1), (2, 2, 3001, 1), (5, 0, 3002, 1)] {
        sqlx::query("INSERT INTO character_items(character_id, container_type, slot, item_id, count) VALUES($1,$2::smallint,$3::smallint,$4,$5)")
            .bind(c.personagem).bind(tipo).bind(slot).bind(tid).bind(n).execute(c.pool.get_ref()).await.unwrap();
    }

    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let r = c.pedir_com_id(&c.realm, &id("on-bolsa"), c.conta, mover("bolsa", 0, 3001, "bolsa", 5)).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("aplicado"), Some("online")), "{r}");
    let r = c.pedir_com_id(&c.realm, &id("on-vestir"), c.conta, mover("bolsa", 1, 3003, "equipamento", 0)).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    let r = c.pedir_com_id(&c.realm, &id("on-tirar"), c.conta, mover("equipamento", 0, 3003, "bolsa", 7)).await;
    assert_eq!(r["estado"], "aplicado", "{r}");
    for (n, pedido, codigo) in [
        (1, mover("bolsa", 7, 3003, "equipamento", 3), "posicao_invalida"),
        (2, mover("bolsa", 5, 3001, "equipamento", 0), "posicao_invalida"),
        (3, mover("armazem", 2, 3001, "bolsa", 8), "precisa_estar_offline"),
        (4, mover("bolsa", 0, 3001, "bolsa", 9), "slot_mudou"),
        (5, mover("bolsa", 5, 3001, "bolsa", 5), "sem_mudanca"),
        (6, mover("bolsa", 5, 3001, "bolsa", 40), "slot_invalido"),
        (7, mover("missao", 0, 3002, "bolsa", 9), "movimento_invalido"),
        (8, mover("bau", 0, 3001, "bolsa", 9), "edicao_invalida"),
    ] {
        let r = c.pedir_com_id(&c.realm, &id(&format!("on{n}")), c.conta, pedido).await;
        assert_eq!(r["dados"]["codigo"], codigo, "{n}: {r}");
    }
    // O cliente recebeu o `EXG_IVTR_ITEM` (44, `s2c.rs` `exg_ivtr_item`: u16 id, u8, u8) da primeira troca.
    esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data.len() == 4 && data[..2] == 44u16.to_le_bytes() && data[2..4] == [0, 5])).await;
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!((onde(&inv, "bolsa", 5), onde(&inv, "bolsa", 7), onde(&inv, "equipamento", 0)), (Some(3001), Some(3003), None), "{inv}");

    drop(bus);
    c.esperar_presenca("ausente").await;
    let r = c.pedir_com_id(&c.realm, &id("off-arm"), c.conta, mover("armazem", 2, 3001, "bolsa", 8)).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
    let r = c.pedir_com_id(&c.realm, &id("off-vestir"), c.conta, mover("bolsa", 7, 3003, "equipamento", 0)).await;
    assert_eq!(r["estado"], "salvo", "{r}");
    let r = c.pedir_com_id(&c.realm, &id("off-missao"), c.conta, mover("missao", 0, 3002, "missao", 3)).await;
    assert_eq!(r["estado"], "salvo", "{r}");
    for (n, pedido, codigo) in [
        // Trocaria a espada pela poção: a poção não entra no slot 0.
        (1, mover("bolsa", 8, 3001, "equipamento", 0), "posicao_invalida"),
        (2, mover("equipamento", 0, 3003, "equipamento", 5), "posicao_invalida"),
        (3, mover("equipamento", 0, 3003, "armazem", 1), "movimento_invalido"),
    ] {
        let r = c.pedir_com_id(&c.realm, &id(&format!("off{n}")), c.conta, pedido).await;
        assert_eq!(r["dados"]["codigo"], codigo, "{n}: {r}");
    }
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!((onde(&inv, "bolsa", 8), onde(&inv, "equipamento", 0), onde(&inv, "missao", 3), onde(&inv, "armazem", 2)),
        (Some(3001), Some(3003), Some(3002), None), "{inv}");
    assert_eq!((soma(&inv, 3001), soma(&inv, 3003), soma(&inv, 3002)), (31, 1, 1), "nada some nem duplica: {inv}");
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e6m-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// E6 (B194): editar o item. Online grava e reenvia a ficha (`OWN_ITEM_INFO` 40) com a quantidade
/// nova; armazém só offline; campo de equipamento em item comum = `nao_e_equipamento`; slot
/// conferido pelo id; offline grava.
#[tokio::test]
async fn editar_item_online_e_offline() {
    let c = Cenario::montar().await;
    let id = |s: &str| format!("e6e-{s}-{}", c.personagem);
    let editar = |rec: &str, slot: u16, tid: u32, e: Value| json!({"tipo":"editar_personagem",
        "personagem_id":c.personagem,"editar_item":{"recipiente":rec,"slot":slot,"id":tid,"edicao":e}});
    let qtd = |inv: &Value, rec: &str, slot: u64| inv["dados"]["recipientes"][rec].as_array().unwrap().iter()
        .find(|i| i["slot"].as_u64() == Some(slot)).map(|i| i["quantidade"].as_u64().unwrap());
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    for (tipo, slot, n) in [(0i32, 2i32, 10i32), (2, 1, 4)] {
        sqlx::query("INSERT INTO character_items(character_id, container_type, slot, item_id, count) VALUES($1,$2::smallint,$3::smallint,3001,$4)")
            .bind(c.personagem).bind(tipo).bind(slot).bind(n).execute(c.pool.get_ref()).await.unwrap();
    }

    let mut bus = c.entrar().await;
    c.esperar_presenca("online").await;
    let r = c.pedir_com_id(&c.realm, &id("on"), c.conta, editar("bolsa", 2, 3001, json!({"quantidade": 777}))).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("aplicado"), Some("online")), "{r}");
    let m = esperar_do_mundo(&mut bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 40u16.to_le_bytes())).await;
    let BusMessage::GameToClient { data, .. } = m else { unreachable!() };
    // `OWN_ITEM_INFO`: id 2, pacote 1, slot 1, tipo 4, validade 4, estado 4, quantidade 4.
    assert_eq!((data[2], data[3], u32::from_le_bytes(data[16..20].try_into().unwrap())), (0, 2, 777), "bolsa, slot 2, 777");
    for (n, pedido, codigo) in [
        (1, editar("armazem", 1, 3001, json!({"quantidade": 5})), "precisa_estar_offline"),
        (2, editar("bolsa", 2, 3001, json!({"durabilidade": 5})), "nao_e_equipamento"),
        (3, editar("bolsa", 2, 3002, json!({"quantidade": 5})), "slot_mudou"),
        (4, editar("bolsa", 2, 3001, json!({"refino": 13})), "edicao_invalida"),
        (5, editar("bolsa", 2, 3001, json!({})), "edicao_invalida"),
        // Campo desconhecido fecha a conexão sem resposta (contrato do canal); a API recusa antes.
    ] {
        let r = c.pedir_com_id(&c.realm, &id(&format!("on{n}")), c.conta, pedido).await;
        assert_eq!(r["dados"]["codigo"], codigo, "{n}: {r}");
    }

    drop(bus);
    c.esperar_presenca("ausente").await;
    let r = c.pedir_com_id(&c.realm, &id("off"), c.conta, editar("armazem", 1, 3001, json!({"quantidade": 9}))).await;
    assert_eq!((r["estado"].as_str(), r["dados"]["presenca"].as_str()), (Some("salvo"), Some("offline")), "{r}");
    let inv = c.pedir(&c.realm, json!({"tipo":"inventario","personagem_id":c.personagem})).await;
    assert_eq!((qtd(&inv, "bolsa", 2), qtd(&inv, "armazem", 1)), (Some(777), Some(9)), "{inv}");
    sqlx::query("DELETE FROM character_items WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e6e-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
    c.encerrar().await;
}

/// Os `LEARN_SKILL` (95: id i32, nível i32) que o mundo mandou ao personagem até `fim` chegar.
async fn aprendizados(bus: &mut pw_bus::transport::BusConnection, fim: usize) -> Vec<(i32, i32)> {
    let mut vistos = Vec::new();
    while vistos.len() < fim {
        let m = esperar_do_mundo(bus, |m| matches!(m, BusMessage::GameToClient { data, .. } if data[..2] == 95u16.to_le_bytes())).await;
        let BusMessage::GameToClient { data, .. } = m else { unreachable!() };
        vistos.push((i32::from_le_bytes(data[2..6].try_into().unwrap()), i32::from_le_bytes(data[6..10].try_into().unwrap())));
    }
    vistos
}

/// E6 (B196): ensinar, subir, descer e remover habilidade. Online pelo `LEARN_SKILL` (95): nova
/// num pacote, subir um por nível, remover = nível 0, descer = 0 e o nível novo; no 1.2.6
/// remover/descer só offline. Offline grava no banco.
#[tokio::test]
async fn habilidade_online_e_offline_126_e_155() {
    for versao in [GameVersion::V1_2_6, GameVersion::V1_5_5] {
        let c = Cenario::montar_versao(versao).await;
        let id = |s: &str| format!("e6h-{s}-{}", c.personagem);
        let pedir = |hab: u32, nivel: u8| json!({"tipo":"editar_personagem","personagem_id":c.personagem,"habilidade":{"id":hab,"nivel":nivel}});
        let nivel_no_banco = || async {
            sqlx::query_scalar::<_, i16>("SELECT level FROM character_skills WHERE character_id=$1 AND skill_id=7")
                .bind(c.personagem).fetch_optional(c.pool.get_ref()).await.unwrap()
        };
        sqlx::query("DELETE FROM character_skills WHERE character_id=$1").bind(c.personagem).execute(c.pool.get_ref()).await.unwrap();

        let mut bus = c.entrar().await;
        c.esperar_presenca("online").await;
        let r = c.pedir_com_id(&c.realm, &id("ensina"), c.conta, pedir(7, 3)).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["nivel_antes"].as_u64()), (Some("aplicado"), Some(0)), "{r}");
        assert_eq!(aprendizados(&mut bus, 1).await, vec![(7, 3)], "nova: um pacote no nível");
        let r = c.pedir_com_id(&c.realm, &id("sobe"), c.conta, pedir(7, 5)).await;
        assert_eq!(r["estado"], "aplicado", "{r}");
        assert_eq!(aprendizados(&mut bus, 2).await, vec![(7, 4), (7, 5)], "um pacote por nível");
        assert_eq!(nivel_no_banco().await, Some(5));
        let r = c.pedir_com_id(&c.realm, &id("igual"), c.conta, pedir(7, 5)).await;
        assert_eq!(r["dados"]["codigo"], "sem_mudanca", "{r}");
        let r = c.pedir_com_id(&c.realm, &id("desce"), c.conta, pedir(7, 2)).await;
        if versao == GameVersion::V1_2_6 {
            assert_eq!(r["dados"]["codigo"], "precisa_estar_offline", "{r}");
        } else {
            assert_eq!(r["estado"], "aplicado", "{r}");
            assert_eq!(aprendizados(&mut bus, 2).await, vec![(7, 0), (7, 2)], "descer = remover e criar");
            let r = c.pedir_com_id(&c.realm, &id("remove"), c.conta, pedir(7, 0)).await;
            assert_eq!(r["estado"], "aplicado", "{r}");
            assert_eq!(aprendizados(&mut bus, 1).await, vec![(7, 0)], "remover = nível 0");
            assert_eq!(nivel_no_banco().await, None);
        }

        drop(bus);
        c.esperar_presenca("ausente").await;
        let r = c.pedir_com_id(&c.realm, &id("off-sobe"), c.conta, pedir(7, 4)).await;
        assert_eq!(r["estado"], "salvo", "{r}");
        let r = c.pedir_com_id(&c.realm, &id("off-remove"), c.conta, pedir(7, 0)).await;
        assert_eq!((r["estado"].as_str(), r["dados"]["nivel_antes"].as_u64()), (Some("salvo"), Some(4)), "{r}");
        assert_eq!(nivel_no_banco().await, None);
        let r = c.pedir_com_id(&c.realm, &id("off-nada"), c.conta, pedir(7, 0)).await;
        assert_eq!(r["dados"]["codigo"], "sem_mudanca", "{r}");
        sqlx::query("DELETE FROM comandos_administrativos WHERE operacao_id LIKE 'e6h-%-' || $1").bind(c.personagem.to_string()).execute(c.pool.get_ref()).await.unwrap();
        c.encerrar().await;
    }
}
