//! Um jogador só mexe no que é dele, no realm em que está.
//!
//! # O que este teste existe para impedir
//!
//! O `role_id` chega dentro de um pacote do cliente (`SelectRole`, `EnterWorld`,
//! `DeleteRole`, `UndoDeleteRole`) e é um inteiro **sequencial** — adivinhar o do vizinho
//! custa nada. Enquanto as consultas eram `WHERE id = $1`, um cliente autenticado entrava
//! no mundo como qualquer personagem do servidor e apagava o personagem de qualquer
//! outro.
//!
//! # Por que contra um banco de verdade
//!
//! A correção mora numa cláusula `WHERE`. Um dublê de repositório testaria o dublê; ler o
//! código-fonte e procurar `account_id` provaria que o texto está lá, não que o banco o
//! respeita. Aqui as consultas rodam no PostgreSQL, no esquema de verdade
//! (`specs/01_DATABASE_SCHEMA_POSTGRES.sql`).
//!
//! # O cenário: dois realms da **mesma versão**
//!
//! É o caso mais exigente, e o que motivou este teste: dois mundos 1.2.6 independentes,
//! um banco só. A mesma conta tem personagem nos dois, e eles não podem se enxergar. Se
//! algum dia o escopo de realm cair, é aqui que aparece.
//!
//! # Como rodar
//!
//! ```bash
//! export TEST_DATABASE_URL='postgresql://pw_admin@localhost:5432/pw_database_test'
//! psql "$TEST_DATABASE_URL" -f specs/01_DATABASE_SCHEMA_POSTGRES.sql
//! cargo test -p pw-storage --test autorizacao_de_personagem
//! ```
//!
//! Sem a variável, o teste **passa sem verificar nada** e diz isso na saída. É um
//! compromisso consciente: manter o `cargo test --workspace` verde em máquina sem banco.
//! Quem mexer em autorização de personagem tem que rodar isto com o banco de pé.

use pw_core::{CharacterClass, Gender, Race};
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};

/// Sufixo único por execução, para que rodar o teste duas vezes não esbarre nas
/// restrições de unicidade (`uq_character_name_per_realm`, `accounts.username`).
fn marca() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000_000
    )
}

struct Cenario {
    repo: CharacterRepository,
    conta_a: i32,
    conta_b: i32,
    realm_a: String,
    realm_b: String,
    /// Personagem da conta A, no realm A.
    dele_a: i32,
    /// Personagem da conta B, no realm A — o alvo que A não pode tocar.
    dele_b: i32,
    /// Personagem da conta A, no **realm B** — mesma conta, outro mundo.
    dele_a_no_outro_realm: i32,
}

/// Monta o cenário, ou devolve `None` quando não há banco configurado.
async fn montar() -> Option<Cenario> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!(
                "AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada. \
                 Veja o cabeçalho do arquivo para rodá-lo de verdade."
            );
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
    let m = marca();

    // Dois realms da MESMA versão. É este par que representa a pergunta "quero subir
    // dois servidores 1.2.6".
    let realm_a = format!("t_a_{m}");
    let realm_b = format!("t_b_{m}");
    for (id, nome) in [(&realm_a, "Teste A"), (&realm_b, "Teste B")] {
        sqlx::query(
            "INSERT INTO realms (id, name, version, host, port, max_players, config)
             VALUES ($1, $2, '1.2.6', '127.0.0.1', 29000, 100, '{}'::jsonb)",
        )
        .bind(id)
        .bind(nome)
        .execute(pool.get_ref())
        .await
        .expect("criar realm de teste");
    }

    let mut contas = Vec::new();
    for quem in ["a", "b"] {
        let id: i32 = sqlx::query_scalar(
            "INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id",
        )
        .bind(format!("teste_{quem}_{m}"))
        .fetch_one(pool.get_ref())
        .await
        .expect("criar conta de teste");
        contas.push(id);
    }
    let (conta_a, conta_b) = (contas[0], contas[1]);

    let repo = CharacterRepository::new(pool);
    let criar = |conta: i32, realm: String, nome: String| {
        let repo = &repo;
        async move {
            repo.create_character(
                conta,
                &realm,
                &nome,
                Race::Human,
                CharacterClass::Blademaster,
                Gender::Male,
                Vec::new(),
            )
            .await
            .expect("criar personagem de teste")
        }
    };

    let dele_a = criar(conta_a, realm_a.clone(), format!("Ana{m}")).await;
    let dele_b = criar(conta_b, realm_a.clone(), format!("Bia{m}")).await;
    let dele_a_no_outro_realm = criar(conta_a, realm_b.clone(), format!("Ana{m}")).await;

    Some(Cenario {
        repo,
        conta_a,
        conta_b,
        realm_a,
        realm_b,
        dele_a,
        dele_b,
        dele_a_no_outro_realm,
    })
}

/// Aborta o teste sem falhar quando não há banco. Uma macro porque `?` num `Option`
/// dentro de `#[tokio::test]` não devolve nada útil.
macro_rules! cenario {
    () => {
        match montar().await {
            Some(c) => c,
            None => return,
        }
    };
}

#[tokio::test]
async fn o_dono_enxerga_o_proprio_personagem() {
    // A rede de proteção dos outros testes: se este falhar, os "não enxerga" abaixo
    // estariam passando por estarem quebrados, e não por estarem certos.
    let c = cenario!();
    let visto = c
        .repo
        .get_details(c.dele_a, c.conta_a, &c.realm_a)
        .await
        .unwrap();
    assert!(
        visto.is_some(),
        "o dono não conseguiu carregar o próprio personagem"
    );
}

#[tokio::test]
async fn nao_da_para_entrar_no_mundo_como_personagem_alheio() {
    let c = cenario!();
    let roubado = c
        .repo
        .get_details(c.dele_b, c.conta_a, &c.realm_a)
        .await
        .unwrap();
    assert!(
        roubado.is_none(),
        "a conta {} carregou o personagem {} da conta {} — é assim que se entra no \
         mundo como outra pessoa",
        c.conta_a,
        c.dele_b,
        c.conta_b
    );
}

#[tokio::test]
async fn os_dois_realms_da_mesma_versao_nao_se_enxergam() {
    // Mesma conta, dois mundos. O personagem do realm B não pode ser carregado por uma
    // sessão do realm A, senão os dois servidores viram um só.
    let c = cenario!();
    let vazado = c
        .repo
        .get_details(c.dele_a_no_outro_realm, c.conta_a, &c.realm_a)
        .await
        .unwrap();
    assert!(
        vazado.is_none(),
        "o personagem {} do realm '{}' foi carregado por uma sessão do realm '{}'",
        c.dele_a_no_outro_realm,
        c.realm_b,
        c.realm_a
    );

    // E a lista de seleção de personagens mostra um em cada realm, não os dois.
    let no_a = c
        .repo
        .list_by_account_and_realm(c.conta_a, &c.realm_a)
        .await
        .unwrap();
    let no_b = c
        .repo
        .list_by_account_and_realm(c.conta_a, &c.realm_b)
        .await
        .unwrap();
    assert_eq!(no_a.len(), 1, "lista do realm A: {:?}", no_a.len());
    assert_eq!(no_b.len(), 1, "lista do realm B: {:?}", no_b.len());
    assert_eq!(no_a[0].id, c.dele_a);
    assert_eq!(no_b[0].id, c.dele_a_no_outro_realm);
}

#[tokio::test]
async fn nao_da_para_apagar_personagem_alheio() {
    let c = cenario!();

    let apagou = c
        .repo
        .delete_character(c.dele_b, c.conta_a, &c.realm_a)
        .await
        .unwrap();
    assert!(
        !apagou,
        "a conta {} apagou o personagem {} da conta {}",
        c.conta_a, c.dele_b, c.conta_b
    );

    // E de fato continua lá, para o dono. Só o valor de retorno não bastaria: ele
    // poderia estar mentindo enquanto a linha some.
    let ainda_existe = c
        .repo
        .get_details(c.dele_b, c.conta_b, &c.realm_a)
        .await
        .unwrap();
    assert!(
        ainda_existe.is_some(),
        "o personagem {} sumiu apesar da recusa",
        c.dele_b
    );
}

#[tokio::test]
async fn nao_da_para_apagar_personagem_do_outro_realm() {
    // Mesma conta, realm errado: a sessão do realm A não pode apagar o que é do B.
    let c = cenario!();
    let apagou = c
        .repo
        .delete_character(c.dele_a_no_outro_realm, c.conta_a, &c.realm_a)
        .await
        .unwrap();
    assert!(
        !apagou,
        "uma sessão do realm '{}' apagou o personagem {} do realm '{}'",
        c.realm_a, c.dele_a_no_outro_realm, c.realm_b
    );
}

#[tokio::test]
async fn o_dono_apaga_e_restaura_o_proprio_personagem() {
    let c = cenario!();

    assert!(
        c.repo
            .delete_character(c.dele_a, c.conta_a, &c.realm_a)
            .await
            .unwrap(),
        "o dono não conseguiu apagar o próprio personagem"
    );

    // Apagar de novo não faz nada: a linha já está com `is_deleted`.
    assert!(
        !c.repo
            .delete_character(c.dele_a, c.conta_a, &c.realm_a)
            .await
            .unwrap(),
        "apagar duas vezes devolveu sucesso na segunda"
    );

    // E um estranho não restaura o que não é dele.
    assert!(
        !c.repo
            .restore_character(c.dele_a, c.conta_b, &c.realm_a)
            .await
            .unwrap(),
        "a conta {} restaurou o personagem {} da conta {}",
        c.conta_b,
        c.dele_a,
        c.conta_a
    );

    assert!(
        c.repo
            .restore_character(c.dele_a, c.conta_a, &c.realm_a)
            .await
            .unwrap(),
        "o dono não conseguiu restaurar o próprio personagem"
    );
}
