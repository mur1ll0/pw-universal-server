//! Registro dos recursos do cliente (B233): versão 0 importada e versões publicadas de cada
//! versão de cliente. Tabelas em `scripts/2026_10_14_recursos_do_cliente.sql`; desenho no §3 do
//! `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`.

use crate::error::Result;
use crate::postgres::PostgresPool;

/// Um arquivo da versão 0. `caminho` em bytes GBK (é com eles que o cliente abre o arquivo).
#[derive(Debug, Clone)]
pub struct RegistroDeRecurso {
    pub caminho: Vec<u8>,
    pub origem: &'static str,
    pub pacote_original: Option<String>,
    pub pacote: String,
    pub tamanho: i64,
    pub sha256: Vec<u8>,
    pub md5: Vec<u8>,
}

#[derive(Clone)]
pub struct RecursosRepository {
    pool: PostgresPool,
}

const LOTE: usize = 5_000;

impl RecursosRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Versões registradas da linha (0 = só a importação).
    pub async fn versoes(&self, linha: &str) -> Result<Vec<i32>> {
        Ok(sqlx::query_scalar("SELECT numero FROM recursos_versoes WHERE linha = $1 ORDER BY numero")
            .bind(linha)
            .fetch_all(self.pool.get_ref())
            .await?)
    }

    /// Apaga a linha para refazer a versão 0. Só quando não há versão publicada além da 0 nem
    /// alteração registrada (a FK de `recursos_alteracoes` impede o resto).
    pub async fn apagar_versao_zero(&self, linha: &str) -> Result<bool> {
        let alem: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM recursos_versoes WHERE linha = $1 AND numero > 0)
                  + (SELECT count(*) FROM recursos_alteracoes WHERE linha = $1)",
        )
        .bind(linha)
        .fetch_one(self.pool.get_ref())
        .await?;
        if alem > 0 {
            return Ok(false);
        }
        sqlx::query("DELETE FROM recursos_linhas WHERE id = $1")
            .bind(linha)
            .execute(self.pool.get_ref())
            .await?;
        Ok(true)
    }

    /// Grava a linha, os arquivos da base e a versão 0, numa transação só.
    pub async fn gravar_versao_zero(
        &self,
        linha: &str,
        origem: &str,
        manifesto_sha256: &[u8],
        assinatura: &[u8],
        registros: &[RegistroDeRecurso],
    ) -> Result<()> {
        let mut tx = self.pool.get_ref().begin().await?;
        sqlx::query("INSERT INTO recursos_linhas (id, origem, base_sha256) VALUES ($1, $2, $3)")
            .bind(linha)
            .bind(origem)
            .bind(manifesto_sha256)
            .execute(&mut *tx)
            .await?;
        for lote in registros.chunks(LOTE) {
            let caminhos: Vec<Vec<u8>> = lote.iter().map(|r| r.caminho.clone()).collect();
            let origens: Vec<&str> = lote.iter().map(|r| r.origem).collect();
            let originais: Vec<Option<String>> = lote.iter().map(|r| r.pacote_original.clone()).collect();
            let pacotes: Vec<String> = lote.iter().map(|r| r.pacote.clone()).collect();
            let tamanhos: Vec<i64> = lote.iter().map(|r| r.tamanho).collect();
            let shas: Vec<Vec<u8>> = lote.iter().map(|r| r.sha256.clone()).collect();
            let md5s: Vec<Vec<u8>> = lote.iter().map(|r| r.md5.clone()).collect();
            sqlx::query(
                "INSERT INTO recursos_base (linha, caminho, origem, pacote_original, pacote, tamanho, sha256, md5)
                 SELECT $1, * FROM UNNEST($2::bytea[], $3::varchar[], $4::varchar[], $5::varchar[],
                                          $6::bigint[], $7::bytea[], $8::bytea[])",
            )
            .bind(linha)
            .bind(caminhos)
            .bind(origens)
            .bind(originais)
            .bind(pacotes)
            .bind(tamanhos)
            .bind(shas)
            .bind(md5s)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO recursos_versoes (linha, numero, manifesto_sha256, assinatura, minima)
             VALUES ($1, 0, $2, $3, 0)",
        )
        .bind(linha)
        .bind(manifesto_sha256)
        .bind(assinatura)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// `(arquivos, bytes)` da versão 0 por origem — para conferir uma importação.
    pub async fn resumo_da_base(&self, linha: &str) -> Result<Vec<(String, i64, i64)>> {
        Ok(sqlx::query_as(
            "SELECT origem::text, count(*), COALESCE(sum(tamanho), 0)::bigint
             FROM recursos_base WHERE linha = $1 GROUP BY origem ORDER BY origem",
        )
        .bind(linha)
        .fetch_all(self.pool.get_ref())
        .await?)
    }
}
