//! Ajuda comum aos testes de integração que escrevem no banco.
//!
//! Incluído por `mod comum;` nos testes do `pw-storage` e por
//! `#[path = "../../pw-storage/tests/comum/mod.rs"] mod comum;` no `pw-gs`, para que a regra
//! de limpeza exista num lugar só.

use pw_storage::PostgresPool;
use std::sync::atomic::{AtomicBool, Ordering};

/// Apaga realms, contas e personagens que execuções **anteriores** dos testes deixaram.
///
/// A regra (o que é de teste, a margem de 15 minutos, a ordem) está no próprio SQL,
/// `tests/sql/limpar_sobras_de_teste.sql`. Roda uma vez por processo de teste: os testes de
/// um arquivo rodam em paralelo, e o primeiro a montar o cenário limpa por todos.
pub async fn limpar_sobras_de_teste(pool: &PostgresPool) {
    static FEITO: AtomicBool = AtomicBool::new(false);
    if FEITO.swap(true, Ordering::SeqCst) {
        return;
    }

    let sem_comentarios: String = include_str!("../sql/limpar_sobras_de_teste.sql")
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");

    for comando in sem_comentarios.split(';').map(str::trim).filter(|c| !c.is_empty()) {
        sqlx::query(comando)
            .execute(pool.get_ref())
            .await
            .expect("limpar as sobras de testes anteriores");
    }
}
