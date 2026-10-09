//! B199: a aparência mandada fora do jogo (tela de seleção) só grava para personagem criado há
//! menos de dois dias, da conta e do realm da sessão (`cnet/gdeliveryd/setcustomdata.hpp`).
mod comum;

use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};

#[tokio::test]
async fn so_personagem_novo_da_conta_grava_a_aparencia_fora_do_jogo() {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return;
        }
    };
    let cfg = StorageConfig { database_url: url, max_connections: 2, min_connections: 1, ..Default::default() };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");
    comum::limpar_sobras_de_teste(&pool).await;
    let m = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000;
    let realm = format!("t_ap_{m}");
    sqlx::query("INSERT INTO realms (id, name, version, host, port, max_players, config) VALUES ($1, 'Teste Aparência', '1.5.5', '127.0.0.1', 29000, 10, '{}'::jsonb)")
        .bind(&realm).execute(pool.get_ref()).await.expect("criar realm");
    let conta: i32 = sqlx::query_scalar("INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id")
        .bind(format!("ap_{m}")).fetch_one(pool.get_ref()).await.expect("criar conta");
    let novo: i32 = sqlx::query_scalar("INSERT INTO characters(account_id,realm_id,name,race,cls,gender) VALUES($1,$2,$3,0,0,0) RETURNING id")
        .bind(conta).bind(&realm).bind(format!("Novo{m}")).fetch_one(pool.get_ref()).await.unwrap();
    let velho: i32 = sqlx::query_scalar("INSERT INTO characters(account_id,realm_id,name,race,cls,gender,created_at) VALUES($1,$2,$3,0,0,0, CURRENT_TIMESTAMP - INTERVAL '3 days') RETURNING id")
        .bind(conta).bind(&realm).bind(format!("Velho{m}")).fetch_one(pool.get_ref()).await.unwrap();
    let repo = CharacterRepository::new(pool.clone());
    let dados = vec![9u8; 176];
    assert!(repo.gravar_aparencia_de_personagem_novo(novo, conta, &realm, &dados).await.unwrap());
    assert!(!repo.gravar_aparencia_de_personagem_novo(velho, conta, &realm, &dados).await.unwrap(), "criado há 3 dias");
    assert!(!repo.gravar_aparencia_de_personagem_novo(novo, conta + 1, &realm, &dados).await.unwrap(), "outra conta");
    assert!(!repo.gravar_aparencia_de_personagem_novo(novo, conta, "outro_realm", &dados).await.unwrap(), "outro realm");
    let gravado: Option<Vec<u8>> = sqlx::query_scalar("SELECT custom_data FROM characters WHERE id=$1").bind(novo).fetch_one(pool.get_ref()).await.unwrap();
    assert_eq!(gravado.as_deref(), Some(&dados[..]));
    assert!(repo.gravar_aparencia(velho, &dados).await.unwrap(), "em jogo, com bilhete, grava sem a regra de idade");
    sqlx::query("DELETE FROM characters WHERE realm_id=$1").bind(&realm).execute(pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE id=$1").bind(conta).execute(pool.get_ref()).await.unwrap();
    sqlx::query("DELETE FROM realms WHERE id=$1").bind(&realm).execute(pool.get_ref()).await.unwrap();
}
