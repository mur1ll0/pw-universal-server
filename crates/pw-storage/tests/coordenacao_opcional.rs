//! Banco sem a migração `scripts/2026_10_05_coordenacao_gm.sql`: a conta ainda carrega.
//! Sem isto, publicar o código antes da migração derruba o login de todos (`SELECT *`).
use pw_storage::{AccountRecord, PostgresPool, StorageConfig};

#[tokio::test]
async fn conta_sem_coluna_revisao_gm_carrega_com_zero() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
        return;
    };
    let pool = PostgresPool::new(&StorageConfig {
        database_url: url,
        max_connections: 1,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .unwrap();
    // Mesmas colunas de `accounts` de antes da migração, sem `revisao_gm`.
    let conta: AccountRecord = sqlx::query_as(
        "SELECT 7::int AS id, 'antiga'::varchar AS username, md5('x')::varchar AS password_hash,
                NULL::varchar AS email, 0::bigint AS gold_balance, 0::bigint AS silver_balance,
                1::int AS gm_privileges, false AS is_banned, NULL::text AS ban_reason,
                NULL::timestamptz AS ban_expires_at, NOW() AS created_at,
                NULL::timestamptz AS last_login_at, NULL::varchar AS last_login_ip",
    )
    .fetch_one(pool.get_ref())
    .await
    .expect("conta sem revisao_gm deve carregar");
    assert_eq!(conta.revisao_gm, 0);
    assert_eq!(conta.gm_privileges, 1);
}
