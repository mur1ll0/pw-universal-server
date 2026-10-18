//! Registro dos recursos do cliente (B233): versão 0 importada e versões publicadas de cada
//! versão de cliente. Tabelas em `scripts/2026_10_14_recursos_do_cliente.sql`; desenho no §3 do
//! `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`.

use crate::error::{Result, StorageError};
use crate::postgres::PostgresPool;

/// Uma alteração (rascunho, publicada ou descartada) e a versão em que foi publicada.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AlteracaoDeRecurso {
    pub id: i64,
    pub linha: String,
    pub autor: Option<i32>,
    pub autor_nome: Option<String>,
    pub criada_em: chrono::DateTime<chrono::Utc>,
    pub descricao: String,
    pub estado: String,
    pub reverte: Option<i64>,
    pub versao: Option<i32>,
    pub itens: i64,
}

/// A última versão publicada que levou arquivo ao `config` do realm (item `servidor`/`ambos`, B249): número e
/// `publicada_em` em segundos Unix. O painel compara com o `iniciado_em` do GS para dizer "aguarda reinício".
pub const SELECT_ULTIMA_DO_SERVIDOR: &str = "SELECT v.numero, EXTRACT(EPOCH FROM v.publicada_em)::BIGINT AS publicada_em
     FROM recursos_versoes v
     WHERE v.linha = $1 AND EXISTS (SELECT 1 FROM recursos_alteracao_itens i
                                     WHERE i.alteracao = ANY(v.alteracoes) AND i.destino <> 'cliente')
     ORDER BY v.numero DESC LIMIT 1";

const SELECT_ALTERACAO: &str = "SELECT a.id, a.linha, a.autor, c.username AS autor_nome, a.criada_em,
        a.descricao, a.estado, a.reverte,
        (SELECT v.numero FROM recursos_versoes v WHERE v.linha = a.linha AND a.id = ANY(v.alteracoes)) AS versao,
        (SELECT count(*) FROM recursos_alteracao_itens i WHERE i.alteracao = a.id) AS itens
     FROM recursos_alteracoes a LEFT JOIN accounts c ON c.id = a.autor";

/// Um arquivo de uma alteração: o que muda nele (`acao`) e os hashes antes e depois.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ItemDeRecurso {
    pub alteracao: i64,
    /// Bytes GBK do caminho (separador `\`).
    pub caminho: Vec<u8>,
    /// `cliente`, `servidor` ou `ambos`.
    pub destino: String,
    /// `adicionar`, `substituir` ou `remover`.
    pub acao: String,
    pub sha256_antes: Option<Vec<u8>>,
    pub sha256_depois: Option<Vec<u8>>,
    pub tamanho: Option<i64>,
    pub resumo: serde_json::Value,
}

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

    /// Todo SHA-256 que a linha ainda usa: a base (versão 0) e os dois lados de cada item de
    /// alteração (o "antes" serve para reverter). Objeto fora deste conjunto é lixo.
    pub async fn objetos_em_uso(&self, linha: &str) -> Result<Vec<Vec<u8>>> {
        Ok(sqlx::query_scalar(
            "SELECT sha256 FROM recursos_base WHERE linha = $1
             UNION
             SELECT s FROM recursos_alteracao_itens i
               JOIN recursos_alteracoes a ON a.id = i.alteracao,
               LATERAL (VALUES (i.sha256_antes), (i.sha256_depois)) v(s)
             WHERE a.linha = $1 AND s IS NOT NULL",
        )
        .bind(linha)
        .fetch_all(self.pool.get_ref())
        .await?)
    }

    // ---------------------------------------------------------------------------------------
    // Rascunhos, publicação e histórico (P3, B235). Regras do §3 do RECURSOS…: versão publicada
    // nunca muda; desfazer = nova alteração com `reverte`; um caminho só pode estar em um
    // rascunho por vez.

    /// `(numero, minima)` da versão publicada mais nova.
    pub async fn versao_atual(&self, linha: &str) -> Result<Option<(i32, i32)>> {
        Ok(sqlx::query_as(
            "SELECT numero, minima FROM recursos_versoes WHERE linha = $1 ORDER BY numero DESC LIMIT 1",
        )
        .bind(linha)
        .fetch_optional(self.pool.get_ref())
        .await?)
    }

    /// A última versão que levou arquivo ao realm: (número, `publicada_em` em segundos Unix) — B249.
    pub async fn ultima_do_servidor(&self, linha: &str) -> Result<Option<(i32, i64)>> {
        Ok(sqlx::query_as(SELECT_ULTIMA_DO_SERVIDOR).bind(linha).fetch_optional(self.pool.get_ref()).await?)
    }

    /// Cria um rascunho (idempotente por `operacao_id`: repetir o pedido devolve o mesmo id).
    pub async fn criar_rascunho(
        &self,
        linha: &str,
        autor: Option<i32>,
        descricao: &str,
        operacao_id: &str,
        reverte: Option<i64>,
    ) -> Result<i64> {
        let novo: Option<i64> = sqlx::query_scalar(
            "INSERT INTO recursos_alteracoes (linha, autor, descricao, operacao_id, reverte)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (operacao_id) DO NOTHING RETURNING id",
        )
        .bind(linha)
        .bind(autor)
        .bind(descricao)
        .bind(operacao_id)
        .bind(reverte)
        .fetch_optional(self.pool.get_ref())
        .await?;
        match novo {
            Some(id) => Ok(id),
            None => Ok(sqlx::query_scalar(
                "SELECT id FROM recursos_alteracoes WHERE operacao_id = $1 AND linha = $2",
            )
            .bind(operacao_id)
            .bind(linha)
            .fetch_optional(self.pool.get_ref())
            .await?
            .ok_or_else(|| StorageError::Duplicate("operacao_id usado em outra versão de cliente".into()))?),
        }
    }

    pub async fn alteracao(&self, id: i64) -> Result<Option<AlteracaoDeRecurso>> {
        Ok(sqlx::query_as(&format!("{SELECT_ALTERACAO} WHERE a.id = $1"))
            .bind(id)
            .fetch_optional(self.pool.get_ref())
            .await?)
    }

    /// Histórico da versão de cliente, mais novas primeiro (rascunhos inclusive).
    pub async fn alteracoes(&self, linha: &str, limite: i64) -> Result<Vec<AlteracaoDeRecurso>> {
        Ok(sqlx::query_as(&format!("{SELECT_ALTERACAO} WHERE a.linha = $1 ORDER BY a.id DESC LIMIT $2"))
            .bind(linha)
            .bind(limite)
            .fetch_all(self.pool.get_ref())
            .await?)
    }

    pub async fn itens(&self, alteracao: i64) -> Result<Vec<ItemDeRecurso>> {
        Ok(sqlx::query_as(
            "SELECT alteracao, caminho, destino, acao, sha256_antes, sha256_depois, tamanho, resumo
             FROM recursos_alteracao_itens WHERE alteracao = $1 ORDER BY caminho",
        )
        .bind(alteracao)
        .fetch_all(self.pool.get_ref())
        .await?)
    }

    /// Acrescenta (ou substitui) o item de um caminho num rascunho. O caminho chega nos bytes
    /// exatos do manifesto (o motor do `pw-recursos` resolve maiúsculas/minúsculas antes).
    /// Recusa se a alteração não é
    /// rascunho ou se o caminho está em outro rascunho da mesma versão de cliente.
    pub async fn gravar_item(&self, item: &ItemDeRecurso) -> Result<()> {
        let mut tx = self.pool.get_ref().begin().await?;
        let alt: Option<(String, String)> = sqlx::query_as(
            "SELECT linha, estado FROM recursos_alteracoes WHERE id = $1 FOR UPDATE",
        )
        .bind(item.alteracao)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((linha, estado)) = alt else {
            return Err(StorageError::NotFound(format!("alteração {}", item.alteracao)));
        };
        if estado != "rascunho" {
            return Err(StorageError::InvalidData(format!("a alteração {} já está {estado}", item.alteracao)));
        }
        // Trava por caminho: serializa quem grava itens desta versão de cliente.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('recursos:' || $1))")
            .bind(&linha)
            .execute(&mut *tx)
            .await?;
        let outro: Option<i64> = sqlx::query_scalar(
            "SELECT i.alteracao FROM recursos_alteracao_itens i
             JOIN recursos_alteracoes a ON a.id = i.alteracao
             WHERE a.linha = $1 AND a.estado = 'rascunho' AND i.alteracao <> $2
               AND i.caminho = $3
             LIMIT 1",
        )
        .bind(&linha)
        .bind(item.alteracao)
        .bind(&item.caminho)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(o) = outro {
            return Err(StorageError::Duplicate(format!("o arquivo já está no rascunho {o}")));
        }
        sqlx::query(
            "DELETE FROM recursos_alteracao_itens
             WHERE alteracao = $1 AND caminho = $2",
        )
        .bind(item.alteracao)
        .bind(&item.caminho)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO recursos_alteracao_itens
               (alteracao, caminho, destino, acao, sha256_antes, sha256_depois, tamanho, resumo)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(item.alteracao)
        .bind(&item.caminho)
        .bind(&item.destino)
        .bind(&item.acao)
        .bind(&item.sha256_antes)
        .bind(&item.sha256_depois)
        .bind(item.tamanho)
        .bind(&item.resumo)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Tira um caminho de um rascunho. `false` se não estava lá (ou não é rascunho).
    pub async fn tirar_item(&self, alteracao: i64, caminho: &[u8]) -> Result<bool> {
        let n = sqlx::query(
            "DELETE FROM recursos_alteracao_itens i USING recursos_alteracoes a
             WHERE a.id = i.alteracao AND a.estado = 'rascunho' AND i.alteracao = $1
               AND i.caminho = $2",
        )
        .bind(alteracao)
        .bind(caminho)
        .execute(self.pool.get_ref())
        .await?
        .rows_affected();
        Ok(n > 0)
    }

    pub async fn descartar(&self, alteracao: i64) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE recursos_alteracoes SET estado = 'descartada' WHERE id = $1 AND estado = 'rascunho'",
        )
        .bind(alteracao)
        .execute(self.pool.get_ref())
        .await?
        .rows_affected();
        Ok(n > 0)
    }

    /// Registra a versão `numero` com os rascunhos `alteracoes`, numa transação: a versão anterior
    /// tem de ser `numero - 1` (dois GMs publicando ao mesmo tempo: um perde) e todos os
    /// rascunhos têm de continuar rascunhos.
    pub async fn publicar(
        &self,
        linha: &str,
        numero: i32,
        alteracoes: &[i64],
        manifesto_sha256: &[u8],
        assinatura: &[u8],
        publicada_por: Option<i32>,
        minima: i32,
    ) -> Result<()> {
        let mut tx = self.pool.get_ref().begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('recursos:' || $1))")
            .bind(linha)
            .execute(&mut *tx)
            .await?;
        let atual: Option<i32> =
            sqlx::query_scalar("SELECT max(numero) FROM recursos_versoes WHERE linha = $1")
                .bind(linha)
                .fetch_one(&mut *tx)
                .await?;
        if atual != Some(numero - 1) {
            return Err(StorageError::Duplicate(format!(
                "a versão publicada mudou (era {}, esperada {})",
                atual.map_or("nenhuma".into(), |n| n.to_string()),
                numero - 1
            )));
        }
        let n = sqlx::query(
            "UPDATE recursos_alteracoes SET estado = 'publicada'
             WHERE id = ANY($1) AND linha = $2 AND estado = 'rascunho'",
        )
        .bind(alteracoes)
        .bind(linha)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n as usize != alteracoes.len() {
            return Err(StorageError::InvalidData("algum rascunho já foi publicado ou descartado".into()));
        }
        sqlx::query(
            "INSERT INTO recursos_versoes (linha, numero, alteracoes, manifesto_sha256, assinatura, publicada_por, minima)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(linha)
        .bind(numero)
        .bind(alteracoes)
        .bind(manifesto_sha256)
        .bind(assinatura)
        .bind(publicada_por)
        .bind(minima)
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
