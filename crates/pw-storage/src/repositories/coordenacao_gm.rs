//! Coordenação administrativa global. Revisão ordenada por commit, não por sequência.
//! Recibo atesta reconciliação, nunca disponibilidade presumida pela ausência de jogador.
use crate::{PostgresPool, Result};
use sqlx::PgConnection;

#[derive(Clone)]
pub struct CoordenacaoGmRepository {
    pool: PostgresPool,
}

pub struct FotografiaGm {
    pub revisao: i64,
    pub contas: std::collections::HashMap<i32, (i64, i32)>,
}

impl CoordenacaoGmRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Uma fotografia consistente mesmo se houver commit entre as duas consultas.
    pub async fn fotografia(&self) -> Result<FotografiaGm> {
        let mut tx = self.pool.get_ref().begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let revisao = sqlx::query_scalar("SELECT revisao FROM coordenacao_gm_revisao WHERE unico")
            .fetch_one(&mut *tx)
            .await?;
        let contas: Vec<(i32, i64, i32)> = sqlx::query_as(
            "SELECT id,revisao_gm,CASE WHEN is_banned THEN 0 ELSE gm_privileges END FROM accounts",
        )
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(FotografiaGm {
            revisao,
            contas: contas.into_iter().map(|(id, r, g)| (id, (r, g))).collect(),
        })
    }

    /// Conexão dedicada: fechar libera o advisory lock, inclusive após queda.
    /// Duas encarnações com o mesmo ID jamais podem confirmar uma pela outra.
    pub async fn registrar(&self, processo: &str, encarnacao: &str) -> Result<PgConnection> {
        let mut conexao = self.pool.get_ref().acquire().await?.detach();
        sqlx::query("SET statement_timeout='2000ms'")
            .execute(&mut conexao)
            .await?;
        let livre: bool =
            sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1,169))")
                .bind(processo)
                .fetch_one(&mut conexao)
                .await?;
        if !livre {
            return Err(crate::StorageError::Duplicate(format!(
                "processo de coordenação {processo}"
            )));
        }
        sqlx::query("INSERT INTO coordenacao_gm_processos(processo,encarnacao,revisao) VALUES($1,$2,0) ON CONFLICT(processo) DO UPDATE SET encarnacao=$2,revisao=0,atualizado_em=NOW()")
            .bind(processo).bind(encarnacao).execute(&mut conexao).await?;
        Ok(conexao)
    }

    pub async fn confirmar(
        conexao: &mut PgConnection,
        processo: &str,
        encarnacao: &str,
        revisao: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE coordenacao_gm_processos SET revisao=GREATEST(revisao,$3),atualizado_em=NOW() WHERE processo=$1 AND encarnacao=$2")
            .bind(processo).bind(encarnacao).bind(revisao).execute(conexao).await?;
        Ok(())
    }
}

/// Política de implantação: lista completa, inclusive daemons desligados.
pub fn alvos_coordenacao_gm() -> Vec<String> {
    let mut alvos: Vec<_> = std::env::var("ADMIN_COORDENACAO_ALVOS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    alvos.sort();
    alvos.dedup();
    alvos
}
