//! Personagem novo nasce com os atributos da classe, não com 10/10/10/10.
//!
//! # A falha que este arquivo tranca
//!
//! O `INSERT` do `create_character` **não mencionava** as colunas `strength`, `agility`,
//! `vitality` e `energy`. O banco usava o `DEFAULT 10` do esquema
//! (`specs/01_DATABASE_SCHEMA_POSTGRES.sql:103-106`) e todo personagem, de toda classe,
//! nascia 10/10/10/10 — enquanto o `ptemplate.conf` do realm dá 15/10/20/5 ao Guerreiro e
//! 10/10/10/20 ao Sacerdote.
//!
//! Não era só cosmético: os quatro atributos entram na vida máxima, na mana máxima, na
//! precisão e na evasão calculadas em `PlayerEntity::do_personagem`.
//!
//! # Por que contra um banco de verdade
//!
//! A falha mora na lista de colunas de um `INSERT`. Nenhum dublê de repositório a
//! mostraria: o código Rust nunca dizia "10" em lugar nenhum — o valor vinha do `DEFAULT`
//! da tabela. Só o banco sabe o que foi gravado.
//!
//! Sem `TEST_DATABASE_URL` o teste passa sem verificar nada e diz isso na saída.

use pw_core::{AtributosIniciais, CharacterClass, Gender, Race};
mod comum;

use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};

struct Cenario {
    repo: CharacterRepository,
    pool: PostgresPool,
    conta: i32,
    realm: String,
    marca: String,
}

async fn montar() -> Option<Cenario> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return None;
        }
    };
    let cfg = StorageConfig {
        database_url: url,
        max_connections: 2,
        min_connections: 1,
        ..Default::default()
    };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");
    comum::limpar_sobras_de_teste(&pool).await;

    // Só o relógio não basta: os três testes deste arquivo rodam em paralelo, e dois que
    // comecem no mesmo nanossegundo geram o mesmo nome de realm e o segundo morre em
    // `duplicate key`. O contador desempata dentro do processo, o relógio entre execuções
    // — o mesmo cuidado que `subcomandos_no_mundo.rs` já tinha.
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let marca = format!(
        "{}_{}",
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000,
        SEQ.fetch_add(1, Ordering::Relaxed)
    );

    let realm = format!("t_at_{marca}");
    sqlx::query(
        "INSERT INTO realms (id, name, version, host, port, max_players, config)
         VALUES ($1, 'Teste Atributos', '1.2.6', '127.0.0.1', 29000, 10, '{}'::jsonb)",
    )
    .bind(&realm)
    .execute(pool.get_ref())
    .await
    .expect("criar realm");

    let conta: i32 = sqlx::query_scalar(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id",
    )
    .bind(format!("at_{marca}"))
    .fetch_one(pool.get_ref())
    .await
    .expect("criar conta");

    let repo = CharacterRepository::new(pool.clone());
    Some(Cenario { repo, pool, conta, realm, marca })
}

/// Lê as quatro colunas direto do banco: é o único jeito de ver o que o `INSERT` gravou.
async fn atributos_no_banco(pool: &PostgresPool, role_id: i32) -> (i32, i32, i32, i32) {
    sqlx::query_as::<_, (i32, i32, i32, i32)>(
        "SELECT strength, agility, vitality, energy FROM characters WHERE id = $1",
    )
    .bind(role_id)
    .fetch_one(pool.get_ref())
    .await
    .expect("ler os atributos do personagem")
}

#[tokio::test]
async fn o_guerreiro_nasce_com_os_atributos_da_classe() {
    let Some(c) = montar().await else { return };

    // Os valores do `[SWORDSMAN]` do `ptemplate.conf` do realm 155BR.
    let do_guerreiro = AtributosIniciais {
        forca: 15,
        agilidade: 10,
        vitalidade: 20,
        energia: 5,
        // 65 de vida base + 4 por vitalidade (o `vit_hp` do [SWORDSMAN]) — os valores
        // exatos não importam aqui; o que o teste cobra é que o que entra é o que é
        // gravado.
        vida: 65 + 4 * 20,
        mana: 20 + 2 * 5,
    };

    let role_id = c
        .repo
        .create_character(
            c.conta,
            &c.realm,
            &format!("Guer{}", c.marca),
            Race::Human,
            CharacterClass::Blademaster,
            Gender::Male,
            Vec::new(),
            Some(do_guerreiro),
        )
        .await
        .expect("criar personagem");

    assert_eq!(
        atributos_no_banco(&c.pool, role_id).await,
        (15, 10, 20, 5),
        "o personagem nasceu com o DEFAULT 10 da coluna em vez do que a classe manda"
    );
}

/// Sem o `ptemplate.conf` — realm que não trouxe o arquivo — continua valendo o padrão da
/// coluna. É menos errado do que inventar um número por classe no repositório.
#[tokio::test]
async fn sem_a_tabela_da_classe_vale_o_padrao_da_coluna() {
    let Some(c) = montar().await else { return };

    let role_id = c
        .repo
        .create_character(
            c.conta,
            &c.realm,
            &format!("Sem{}", c.marca),
            Race::Human,
            CharacterClass::Wizard,
            Gender::Female,
            Vec::new(),
            None,
        )
        .await
        .expect("criar personagem");

    assert_eq!(atributos_no_banco(&c.pool, role_id).await, (10, 10, 10, 10));
}

/// E o que foi gravado é o que volta em `get_details_por_role` — o caminho que o mundo
/// usa para montar o `PlayerEntity`, e portanto a vida máxima e a evasão.
#[tokio::test]
async fn o_que_foi_gravado_e_o_que_o_mundo_le() {
    let Some(c) = montar().await else { return };

    let do_sacerdote = AtributosIniciais {
        forca: 10,
        agilidade: 10,
        vitalidade: 10,
        energia: 20,
        vida: 30 + 2 * 10,
        mana: 50 + 4 * 20,
    };

    let role_id = c
        .repo
        .create_character(
            c.conta,
            &c.realm,
            &format!("Sace{}", c.marca),
            Race::WingedElf,
            CharacterClass::Cleric,
            Gender::Female,
            Vec::new(),
            Some(do_sacerdote),
        )
        .await
        .expect("criar personagem");

    let d = c
        .repo
        .get_details_por_role(role_id)
        .await
        .expect("ler o personagem")
        .expect("o personagem recém-criado tem de existir");

    assert_eq!(
        (d.strength, d.agility, d.vitality, d.energy),
        (10, 10, 10, 20),
        "o mundo leria outros atributos dos que foram gravados"
    );
}
