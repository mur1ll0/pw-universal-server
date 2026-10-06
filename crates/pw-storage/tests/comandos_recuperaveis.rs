//! Atomicidade real no PostgreSQL; nenhum teste se omite sem banco.
use pw_storage::{AccountRepository, PostgresPool, StorageConfig};
use serde_json::json;
#[path = "comum/mod.rs"]
mod comum;

#[tokio::test]
async fn criacao_concorrente_com_mesmo_id_retorna_a_mesma_conta_e_defaults() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let usuario = format!("at_{admin}_31");
    let (a, b) = tokio::join!(
        repo.criar_conta(&id, admin, "realm_126", &usuario, &[7; 32], "hash"),
        repo.criar_conta(&id, admin, "realm_155", &usuario, &[7; 32], "hash")
    );
    let resultado = a.unwrap();
    assert_eq!(resultado, b.unwrap());
    assert_eq!(resultado["estado"], "salvo");
    let nova = resultado["conta_id"].as_i64().unwrap() as i32;
    let conta = AccountRepository::new(pool.clone())
        .find_by_id(nova)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            conta.gold_balance,
            conta.silver_balance,
            conta.gm_privileges,
            conta.is_banned
        ),
        (0, 0, 0, false)
    );
    assert_eq!(repo.consultar(&id, admin).await.unwrap(), resultado);
    assert_eq!(
        repo.criar_conta(&id, admin, "realm_155", &usuario, &[8; 32], "outro")
            .await
            .unwrap()["codigo"],
        "operacao_em_conflito"
    );
    assert_eq!(
        repo.criar_conta(&id, alvo, "realm_155", &usuario, &[7; 32], "hash")
            .await
            .unwrap()["codigo"],
        "administrador_recusado"
    );
    assert_eq!(
        repo.trocar_senha(&id, admin, "realm_126", nova, &[9; 32], "outro")
            .await
            .unwrap()["codigo"],
        "operacao_em_conflito"
    );
    assert_eq!(
        AccountRepository::new(pool.clone())
            .find_by_id(nova)
            .await
            .unwrap()
            .unwrap()
            .password_hash,
        "hash"
    );
    limpar(&pool, admin, nova).await;
    limpar(&pool, admin, alvo).await;
}

#[tokio::test]
async fn criacoes_de_administradores_distintos_disputam_nome_sem_distinguir_caixa() {
    let (_serie,pool, admin, outro, id) = montar().await;
    let contas = AccountRepository::new(pool.clone());
    contas.set_gm_privileges(outro, 1).await.unwrap();
    let repo = contas.comandos_administrativos();
    let nome = format!("at_{admin}_32");
    let maiusculo = nome.to_uppercase();
    let id_outro = format!("outro-{id}");
    let (a, b) = tokio::join!(
        repo.criar_conta(&id, admin, "realm_126", &nome, &[1; 32], "um"),
        repo.criar_conta(&id_outro, outro, "realm_155", &maiusculo, &[2; 32], "dois")
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert!(matches!(
        (a["estado"].as_str(), b["estado"].as_str()),
        (Some("salvo"), Some("falha")) | (Some("falha"), Some("salvo"))
    ));
    let falha = if a["estado"] == "falha" { &a } else { &b };
    assert_eq!(falha["codigo"], "usuario_existente");
    assert_eq!(repo.consultar(&id, admin).await.unwrap(), a);
    assert_eq!(repo.consultar(&id_outro, outro).await.unwrap(), b);
    let conta = contas.find_by_username(&nome).await.unwrap().unwrap();
    assert_eq!(
        contas
            .find_by_username(&maiusculo)
            .await
            .unwrap()
            .unwrap()
            .id,
        conta.id
    );
    assert!(contas
        .create_account(&maiusculo, "outro", None)
        .await
        .is_err());
    // UPDATE também não pode introduzir uma identidade ambígua.
    assert!(sqlx::query("UPDATE accounts SET username=$1 WHERE id=$2")
        .bind(&maiusculo)
        .bind(admin)
        .execute(pool.get_ref())
        .await
        .is_err());
    limpar(&pool, admin, conta.id).await;
    limpar(&pool, outro, admin).await;
}

#[tokio::test]
async fn criacao_recusa_outro_dono_do_id_e_preserva_resultado_original() {
    let (_serie,pool, admin, outro, id) = montar().await;
    let contas = AccountRepository::new(pool.clone());
    contas.set_gm_privileges(outro, 1).await.unwrap();
    let repo = contas.comandos_administrativos();
    let nome = format!("at_{admin}_33");
    let original = repo
        .criar_conta(&id, admin, "realm_126", &nome, &[1; 32], "hash")
        .await
        .unwrap();
    assert_eq!(
        repo.criar_conta(&id, outro, "realm_155", &nome, &[1; 32], "hash")
            .await
            .unwrap()["codigo"],
        "operacao_em_conflito"
    );
    assert_eq!(
        repo.consultar(&id, outro).await.unwrap()["estado"],
        "desconhecido"
    );
    assert_eq!(repo.consultar(&id, admin).await.unwrap(), original);
    limpar(&pool, admin, original["conta_id"].as_i64().unwrap() as i32).await;
    limpar(&pool, admin, outro).await;
}

#[tokio::test]
async fn falha_do_resultado_reverte_criacao_id_e_reserva_sem_exigir_sequencia_contigua() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let nome = format!("at_{admin}_34");
    let funcao = format!("teste_criacao_{admin}");
    sqlx::query(&format!("CREATE FUNCTION test.{funcao}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operacao_id = '{id}' THEN RAISE EXCEPTION 'falha de teste'; END IF; RETURN NEW; END $$")).execute(pool.get_ref()).await.unwrap();
    sqlx::query(&format!("CREATE TRIGGER {funcao} BEFORE UPDATE ON test.comandos_administrativos FOR EACH ROW EXECUTE FUNCTION test.{funcao}()")).execute(pool.get_ref()).await.unwrap();
    let resposta = repo
        .criar_conta(&id, admin, "realm_126", &nome, &[1; 32], "hash")
        .await;
    sqlx::query(&format!(
        "DROP TRIGGER {funcao} ON test.comandos_administrativos"
    ))
    .execute(pool.get_ref())
    .await
    .unwrap();
    sqlx::query(&format!("DROP FUNCTION test.{funcao}()"))
        .execute(pool.get_ref())
        .await
        .unwrap();
    assert!(resposta.is_err());
    assert!(AccountRepository::new(pool.clone())
        .find_by_username(&nome)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        repo.consultar(&id, admin).await.unwrap()["estado"],
        "desconhecido"
    );
    let resultado = repo
        .criar_conta(&id, admin, "realm_155", &nome, &[1; 32], "hash")
        .await
        .unwrap();
    assert_eq!(resultado["estado"], "salvo");
    limpar(&pool, admin, resultado["conta_id"].as_i64().unwrap() as i32).await;
    limpar(&pool, admin, alvo).await;
}

#[tokio::test]
async fn criacao_aguarda_revogacao_e_nao_cria_conta_sem_autorizacao() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let nome = format!("at_{admin}_35");
    let mut revogacao = pool.get_ref().begin().await.unwrap();
    sqlx::query("UPDATE accounts SET gm_privileges=0 WHERE id=$1")
        .bind(admin)
        .execute(&mut *revogacao)
        .await
        .unwrap();
    let tarefa = tokio::spawn({
        let repo = repo.clone();
        let id = id.clone();
        let nome = nome.clone();
        async move {
            repo.criar_conta(&id, admin, "realm_126", &nome, &[1; 32], "hash")
                .await
                .unwrap()
        }
    });
    assert_eq!(
        repo.consultar(&id, admin).await.unwrap()["estado"],
        "desconhecido"
    );
    revogacao.commit().await.unwrap();
    assert_eq!(tarefa.await.unwrap()["codigo"], "administrador_recusado");
    assert!(AccountRepository::new(pool.clone())
        .find_by_username(&nome)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        repo.consultar(&id, admin).await.unwrap()["estado"],
        "desconhecido"
    );
    limpar(&pool, admin, alvo).await;
}

/// Os testes deste arquivo disputam a linha única `coordenacao_gm_revisao` com
/// `lock_timeout` de 1,5 s e contam esperas no `pg_stat_activity` do banco inteiro:
/// em paralelo falhavam de forma intermitente (B172). Rodam em série entre si.
static SERIE: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn montar() -> (std::sync::MutexGuard<'static, ()>, PostgresPool, i32, i32, String) {
    let serie = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let pool = PostgresPool::new(&StorageConfig {
        database_url: std::env::var("TEST_DATABASE_URL").expect("banco obrigatório"),
        max_connections: 4,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .unwrap();
    comum::limpar_sobras_de_teste(&pool).await;
    let esquema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(pool.get_ref())
        .await
        .unwrap();
    assert_eq!(esquema, "test");
    let sufixo = rand_sem_dependencia();
    let admin: i32 = sqlx::query_scalar("INSERT INTO accounts(username,password_hash,gm_privileges) VALUES($1,'anterior',1) RETURNING id")
        .bind(format!("at_{sufixo}_1")).fetch_one(pool.get_ref()).await.unwrap();
    let alvo: i32 = sqlx::query_scalar(
        "INSERT INTO accounts(username,password_hash) VALUES($1,'anterior') RETURNING id",
    )
    .bind(format!("at_{sufixo}_2"))
    .fetch_one(pool.get_ref())
    .await
    .unwrap();
    (serie, pool, admin, alvo, format!("teste-{admin}"))
}

/// Relógio sozinho repete no Windows entre testes paralelos (resolução de ~100 ns):
/// soma um contador do processo para o sufixo nunca colidir.
fn rand_sem_dependencia() -> u128 {
    static CONTADOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CONTADOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u128;
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        * 65_536 + n % 65_536
}

async fn limpar(pool: &PostgresPool, admin: i32, alvo: i32) {
    sqlx::query("DELETE FROM comandos_administrativos WHERE administrador_id=$1")
        .bind(admin)
        .execute(pool.get_ref())
        .await
        .unwrap();
    sqlx::query("DELETE FROM accounts WHERE id IN ($1,$2)")
        .bind(admin)
        .bind(alvo)
        .execute(pool.get_ref())
        .await
        .unwrap();
}

#[tokio::test]
async fn falha_ao_gravar_resultado_reverte_a_senha_e_libera_o_identificador() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    // Trigger só no ID desta conta de teste, removido mesmo após uma falha prevista.
    let funcao = format!("teste_falhar_{admin}");
    sqlx::query(&format!("CREATE FUNCTION test.{funcao}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operacao_id = '{id}' THEN RAISE EXCEPTION 'falha de teste'; END IF; RETURN NEW; END $$"))
        .execute(pool.get_ref()).await.unwrap();
    sqlx::query(&format!("CREATE TRIGGER {funcao} BEFORE UPDATE ON test.comandos_administrativos FOR EACH ROW EXECUTE FUNCTION test.{funcao}()"))
        .execute(pool.get_ref()).await.unwrap();
    let resultado = repo
        .trocar_senha(&id, admin, "realm_126", alvo, &[1; 32], "novo")
        .await;
    sqlx::query(&format!(
        "DROP TRIGGER {funcao} ON test.comandos_administrativos"
    ))
    .execute(pool.get_ref())
    .await
    .unwrap();
    sqlx::query(&format!("DROP FUNCTION test.{funcao}()"))
        .execute(pool.get_ref())
        .await
        .unwrap();
    assert!(resultado.is_err());
    assert_eq!(
        AccountRepository::new(pool.clone())
            .find_by_id(alvo)
            .await
            .unwrap()
            .unwrap()
            .password_hash,
        "anterior"
    );
    assert_eq!(
        repo.consultar(&id, admin).await.unwrap()["estado"],
        "desconhecido"
    );
    assert_eq!(
        repo.trocar_senha(&id, admin, "realm_155", alvo, &[1; 32], "novo")
            .await
            .unwrap()["estado"],
        "salvo"
    );
    limpar(&pool, admin, alvo).await;
}

#[tokio::test]
async fn revogacao_concorrente_e_checada_sob_o_mesmo_lock_da_escrita() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let mut revogacao = pool.get_ref().begin().await.unwrap();
    sqlx::query("UPDATE accounts SET is_banned=true WHERE id=$1")
        .bind(admin)
        .execute(&mut *revogacao)
        .await
        .unwrap();
    let tarefa = tokio::spawn({
        let repo = repo.clone();
        let id = id.clone();
        async move {
            repo.trocar_senha(&id, admin, "realm_126", alvo, &[1; 32], "novo")
                .await
                .unwrap()
        }
    });
    // Confirmar que o comando alcançou a reserva sem conseguir ultrapassar o lock.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    revogacao.commit().await.unwrap();
    assert_eq!(tarefa.await.unwrap()["codigo"], "administrador_recusado");
    assert_eq!(
        repo.consultar(&id, admin).await.unwrap()["estado"],
        "desconhecido"
    );
    assert_eq!(
        AccountRepository::new(pool.clone())
            .find_by_id(alvo)
            .await
            .unwrap()
            .unwrap()
            .password_hash,
        "anterior"
    );
    limpar(&pool, admin, alvo).await;
}

#[tokio::test]
async fn ausencia_de_registro_nao_afirma_falha_enquanto_a_transacao_esta_em_andamento() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let mut bloqueio = pool.get_ref().begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(alvo)
        .execute(&mut *bloqueio)
        .await
        .unwrap();
    let tarefa = tokio::spawn({
        let repo = repo.clone();
        let id = id.clone();
        async move {
            repo.trocar_senha(&id, admin, "realm_126", alvo, &[1; 32], "novo")
                .await
                .unwrap()
        }
    });
    assert_eq!(
        repo.consultar(&id, admin).await.unwrap()["estado"],
        "desconhecido"
    );
    bloqueio.rollback().await.unwrap();
    let resultado = tarefa.await.unwrap();
    assert_eq!(repo.consultar(&id, admin).await.unwrap(), resultado);
    assert_eq!(resultado["estado"], "salvo");
    limpar(&pool, admin, alvo).await;
}

#[tokio::test]
async fn falha_de_alvo_e_duravel_entre_realms_e_id_nao_aceita_outro_alvo() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let inexistente = i32::MAX;
    let resposta = repo
        .trocar_senha(&id, admin, "realm_126", inexistente, &[1; 32], "novo")
        .await
        .unwrap();
    assert_eq!(resposta["codigo"], "conta_inexistente");
    assert_eq!(repo.consultar(&id, admin).await.unwrap(), resposta);
    assert_eq!(
        repo.trocar_senha(&id, admin, "realm_155", inexistente, &[1; 32], "novo")
            .await
            .unwrap(),
        resposta
    );
    assert_eq!(
        repo.trocar_senha(&id, admin, "realm_155", alvo, &[2; 32], "novo")
            .await
            .unwrap(),
        json!({"estado":"falha", "codigo":"operacao_em_conflito"})
    );
    limpar(&pool, admin, alvo).await;
}

#[tokio::test]
async fn gm_global_persistencia_nao_confirma_sessoes_e_reinicio_exige_novo_recibo() {
    let (_serie,pool, admin, alvo, id) = montar().await;
    let contas = AccountRepository::new(pool.clone());
    let repo = contas.comandos_administrativos();
    let coord = contas.coordenacao_gm();
    let processos = vec![format!("teste-link-{admin}"),format!("teste-gs-{admin}")];
    let mut link = coord.registrar(&processos[0],"primeiro").await.unwrap();
    let r = repo.definir_gm(&id,admin,"realm_126",alvo,true,&[169;32],&processos).await.unwrap();
    assert_eq!(r["estado"],"pendente");
    assert_eq!(r["persistencia"],"salva");
    let rev = r["revisao"].as_i64().unwrap();
    assert_eq!(contas.find_by_id(alvo).await.unwrap().unwrap().gm_privileges,1);
    assert_eq!(repo.definir_gm(&id,admin,"realm_155",alvo,true,&[169;32],&processos).await.unwrap()["revisao"],rev);
    assert_eq!(repo.definir_gm(&id,admin,"realm_155",admin,true,&[169;32],&processos).await.unwrap()["codigo"],"operacao_em_conflito");
    assert_eq!(repo.definir_gm(&id,admin,"realm_155",alvo,false,&[170;32],&processos).await.unwrap()["codigo"],"operacao_em_conflito");
    pw_storage::CoordenacaoGmRepository::confirmar(&mut link,&processos[0],"primeiro",rev).await.unwrap();
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"pendente");
    assert!(coord.registrar(&processos[0],"duplicado").await.is_err());
    let mut gs = coord.registrar(&processos[1],"gs").await.unwrap();
    drop(link);
    let mut reiniciado = coord.registrar(&processos[0],"segundo").await.unwrap();
    pw_storage::CoordenacaoGmRepository::confirmar(&mut gs,&processos[1],"gs",rev).await.unwrap();
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"pendente");
    pw_storage::CoordenacaoGmRepository::confirmar(&mut reiniciado,&processos[0],"primeiro",rev).await.unwrap();
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"pendente","encarnação antiga não confirma nova");
    pw_storage::CoordenacaoGmRepository::confirmar(&mut reiniciado,&processos[0],"segundo",rev).await.unwrap();
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"aplicado");
    drop(gs); drop(reiniciado);
    sqlx::query("DELETE FROM coordenacao_gm_processos WHERE processo=ANY($1)").bind(&processos).execute(pool.get_ref()).await.unwrap();
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"aplicado","confirmação passada é durável");
    limpar(&pool,admin,alvo).await;
}

#[tokio::test]
async fn gm_repeticao_concorrente_nao_reaplica_e_comando_posterior_substitui() {
    let (_serie,pool,admin,alvo,id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let alvos = vec![format!("teste-offline-{admin}")];
    let (a,b)=tokio::join!(repo.definir_gm(&id,admin,"126",alvo,true,&[1;32],&alvos),repo.definir_gm(&id,admin,"155",alvo,true,&[1;32],&alvos));
    assert_eq!(a.unwrap()["revisao"],b.unwrap()["revisao"]);
    let id2=format!("{id}-2");
    assert_eq!(repo.definir_gm(&id2,admin,"155",alvo,false,&[2;32],&alvos).await.unwrap()["estado"],"pendente");
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"substituido");
    assert_eq!(repo.definir_gm(&id,admin,"155",alvo,true,&[1;32],&alvos).await.unwrap()["estado"],"substituido");
    assert_eq!(AccountRepository::new(pool.clone()).find_by_id(alvo).await.unwrap().unwrap().gm_privileges,0);
    limpar(&pool,admin,alvo).await;
}

#[tokio::test]
async fn gm_autorizacao_sob_lock_e_rollback_por_timeout() {
    let (_serie,pool,admin,alvo,id)=montar().await;
    let repo=AccountRepository::new(pool.clone()).comandos_administrativos();
    let processos=vec![format!("teste-offline-{admin}")];
    assert_eq!(repo.definir_gm(&id,alvo,"126",admin,false,&[3;32],&processos).await.unwrap()["codigo"],"administrador_recusado");
    assert_eq!(repo.definir_gm(&id,admin,"126",alvo,true,&[3;32],&[]).await.unwrap()["codigo"],"coordenacao_nao_configurada");
    let mut bloqueio=pool.get_ref().begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE").bind(alvo).execute(&mut *bloqueio).await.unwrap();
    assert!(repo.definir_gm(&id,admin,"126",alvo,true,&[3;32],&processos).await.is_err());
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"desconhecido");
    bloqueio.rollback().await.unwrap();
    // Revogação confirmada antes de obter o lock do administrador é revalidada.
    let mut bloqueio=pool.get_ref().begin().await.unwrap();
    sqlx::query("UPDATE accounts SET gm_privileges=0 WHERE id=$1").bind(admin).execute(&mut *bloqueio).await.unwrap();
    let tarefa=tokio::spawn({ let repo=repo.clone(); let id=id.clone(); let processos=processos.clone(); async move {repo.definir_gm(&id,admin,"126",alvo,true,&[3;32],&processos).await.unwrap()} });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    bloqueio.commit().await.unwrap();
    assert_eq!(tarefa.await.unwrap()["codigo"],"administrador_recusado");
    limpar(&pool,admin,alvo).await;
}

#[tokio::test]
async fn gm_recibo_expirado_nao_confirma_daemon_indisponivel() {
    let (_serie,pool,admin,alvo,id)=montar().await;
    let contas=AccountRepository::new(pool.clone());
    let repo=contas.comandos_administrativos();
    let processos=vec![format!("teste-gs-{admin}")];
    let mut c=contas.coordenacao_gm().registrar(&processos[0],"a").await.unwrap();
    let r=repo.definir_gm(&id,admin,"126",alvo,true,&[3;32],&processos).await.unwrap();
    pw_storage::CoordenacaoGmRepository::confirmar(&mut c,&processos[0],"a",r["revisao"].as_i64().unwrap()).await.unwrap();
    sqlx::query("UPDATE coordenacao_gm_processos SET atualizado_em=NOW()-INTERVAL '10 seconds' WHERE processo=$1").bind(&processos[0]).execute(pool.get_ref()).await.unwrap();
    assert_eq!(repo.consultar(&id,admin).await.unwrap()["estado"],"pendente");
    drop(c);
    sqlx::query("DELETE FROM coordenacao_gm_processos WHERE processo=$1").bind(&processos[0]).execute(pool.get_ref()).await.unwrap();
    limpar(&pool,admin,alvo).await;
}

#[tokio::test]
async fn gm_consultas_concorrentes_devolvem_o_mesmo_vencedor_duravel() {
    let (_serie,pool,admin,alvo,id)=montar().await;
    let contas=AccountRepository::new(pool.clone());
    let repo=contas.comandos_administrativos();
    let processos=vec![format!("teste-gs-{admin}")];
    let mut c=contas.coordenacao_gm().registrar(&processos[0],"a").await.unwrap();
    let r=repo.definir_gm(&id,admin,"126",alvo,true,&[3;32],&processos).await.unwrap();
    pw_storage::CoordenacaoGmRepository::confirmar(&mut c,&processos[0],"a",r["revisao"].as_i64().unwrap()).await.unwrap();
    let mut bloqueio=pool.get_ref().begin().await.unwrap();
    sqlx::query("SELECT operacao_id FROM comandos_administrativos WHERE operacao_id=$1 FOR UPDATE").bind(&id).execute(&mut *bloqueio).await.unwrap();
    let consultar=|| {let repo=repo.clone();let id=id.clone();tokio::spawn(async move {repo.consultar(&id,admin).await.unwrap()})};
    let primeira=consultar();
    async fn esperar_consultas(pool:&PostgresPool,n:i64) {
        for _ in 0..500 {
            let esperando:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'UPDATE comandos_administrativos SET resultado=%' AND query LIKE '%pendente%'").fetch_one(pool.get_ref()).await.unwrap();
            if esperando>=n{return;} tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("consulta não alcançou lock de confirmação");
    }
    esperar_consultas(&pool,1).await;
    contas.set_gm_privileges(alvo,0).await.unwrap();
    let segunda=consultar();
    esperar_consultas(&pool,2).await;
    bloqueio.rollback().await.unwrap();
    let a=primeira.await.unwrap(); let b=segunda.await.unwrap();
    assert_eq!(a,b,"consulta concorrente devolveu conclusão local em vez do vencedor durável");
    assert_eq!(repo.consultar(&id,admin).await.unwrap(),a);
    drop(c);
    sqlx::query("DELETE FROM coordenacao_gm_processos WHERE processo=$1").bind(&processos[0]).execute(pool.get_ref()).await.unwrap();
    limpar(&pool,admin,alvo).await;
}

/// E4 (B175): gold da conta — soma atômica, nunca negativo, repetição devolve o mesmo
/// resultado sem somar de novo, parâmetro diferente com o mesmo ID é conflito.
#[tokio::test]
async fn gold_soma_atomica_repeticao_idempotente_e_saldo_nunca_negativo() {
    let (_serie, pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let saldo = |pool: PostgresPool| async move {
        sqlx::query_scalar::<_, i64>("SELECT gold_balance FROM accounts WHERE id=$1")
            .bind(alvo).fetch_one(pool.get_ref()).await.unwrap()
    };
    let antes = saldo(pool.clone()).await;
    let r = repo.ajustar_gold(&id, admin, "realm_126", alvo, 1500, &[7; 32]).await.unwrap();
    assert_eq!(r["estado"], "salvo");
    assert_eq!(r["saldo"], (antes + 1500).to_string());
    // Mesmo ID e parâmetros: devolve o resultado gravado, não soma outra vez.
    let repetido = repo.ajustar_gold(&id, admin, "realm_155", alvo, 1500, &[7; 32]).await.unwrap();
    assert_eq!(repetido, r);
    assert_eq!(saldo(pool.clone()).await, antes + 1500);
    // Mesmo ID com outro valor: conflito, saldo intacto.
    let conflito = repo.ajustar_gold(&id, admin, "realm_126", alvo, 9, &[8; 32]).await.unwrap();
    assert_eq!(conflito["codigo"], "operacao_em_conflito");
    // Só dar (B180): valor negativo ou zero é recusado, saldo intacto.
    for (n, delta) in [(9u8, -1i64), (10, 0)] {
        let falha = repo.ajustar_gold(&format!("{id}-neg{n}"), admin, "realm_126", alvo, delta, &[n; 32]).await.unwrap();
        assert_eq!(falha["codigo"], "valor_invalido");
    }
    assert_eq!(saldo(pool.clone()).await, antes + 1500);
    limpar(&pool, admin, alvo).await;
}

/// E4 (B175): banimento global — o administrador não se bane; ban grava motivo,
/// desban limpa; conta inexistente é falha.
#[tokio::test]
async fn ban_e_desban_globais_sem_autobanimento() {
    let (_serie, pool, admin, alvo, id) = montar().await;
    let repo = AccountRepository::new(pool.clone()).comandos_administrativos();
    let proprio = repo.definir_ban(&format!("{id}-p"), admin, "realm_126", admin, true, None, &[1; 32]).await.unwrap();
    assert_eq!(proprio["codigo"], "proprio_administrador");
    let ban = repo.definir_ban(&id, admin, "realm_126", alvo, true, Some("teste"), &[2; 32]).await.unwrap();
    assert_eq!((ban["estado"].as_str(), ban["banida"].as_bool()), (Some("salvo"), Some(true)));
    let (banida, motivo): (bool, Option<String>) = sqlx::query_as("SELECT is_banned, ban_reason FROM accounts WHERE id=$1")
        .bind(alvo).fetch_one(pool.get_ref()).await.unwrap();
    assert_eq!((banida, motivo.as_deref()), (true, Some("teste")));
    let desban = repo.definir_ban(&format!("{id}-d"), admin, "realm_155", alvo, false, None, &[3; 32]).await.unwrap();
    assert_eq!(desban["banida"], false);
    let (banida, motivo): (bool, Option<String>) = sqlx::query_as("SELECT is_banned, ban_reason FROM accounts WHERE id=$1")
        .bind(alvo).fetch_one(pool.get_ref()).await.unwrap();
    assert_eq!((banida, motivo), (false, None));
    let inexistente = repo.definir_ban(&format!("{id}-x"), admin, "realm_126", i32::MAX, true, None, &[4; 32]).await.unwrap();
    assert_eq!(inexistente["codigo"], "conta_inexistente");
    limpar(&pool, admin, alvo).await;
}
