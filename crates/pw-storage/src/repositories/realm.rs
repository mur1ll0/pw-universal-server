use crate::error::Result;
use crate::postgres::PostgresPool;
use chrono::{DateTime, Utc};
use pw_core::RealmId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
/// As rates são `NUMERIC(3,1)` no schema (`specs/01_DATABASE_SCHEMA_POSTGRES.sql`); as
/// consultas convertem para `float4`, senão a leitura falha (B176).
pub struct RealmRecord {
    pub id: RealmId,
    pub name: String,
    pub version: String,
    pub host: String,
    pub port: i32,
    pub is_online: bool,
    pub max_players: i32,
    pub double_exp_multiplier: f32,
    pub double_sp_multiplier: f32,
    pub double_drop_multiplier: f32,
    pub double_gold_multiplier: f32,
    pub config: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct RealmRepository {
    pool: PostgresPool,
}

impl RealmRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// Lista todos os Realms configurados
    pub async fn list_realms(&self) -> Result<Vec<RealmRecord>> {
        let recs = sqlx::query_as::<_, RealmRecord>(
            r#"
            SELECT id, name, version, host, port, is_online, max_players, double_exp_multiplier::float4 AS double_exp_multiplier, double_sp_multiplier::float4 AS double_sp_multiplier, double_drop_multiplier::float4 AS double_drop_multiplier, double_gold_multiplier::float4 AS double_gold_multiplier, config, created_at FROM realms ORDER BY id ASC
            "#,
        )
        .fetch_all(self.pool.get_ref())
        .await?;

        Ok(recs)
    }

    /// Busca um Realm por ID (ex: "realm_126")
    pub async fn get_realm(&self, realm_id: &str) -> Result<Option<RealmRecord>> {
        let rec = sqlx::query_as::<_, RealmRecord>(
            r#"
            SELECT id, name, version, host, port, is_online, max_players, double_exp_multiplier::float4 AS double_exp_multiplier, double_sp_multiplier::float4 AS double_sp_multiplier, double_drop_multiplier::float4 AS double_drop_multiplier, double_gold_multiplier::float4 AS double_gold_multiplier, config, created_at FROM realms WHERE id = $1
            "#,
        )
        .bind(realm_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(rec)
    }

    /// Atualiza multiplicadores de Double Events em tempo real
    pub async fn update_multipliers(
        &self,
        realm_id: &str,
        exp: f32,
        sp: f32,
        drop: f32,
        gold: f32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE realms 
            SET double_exp_multiplier = $1,
                double_sp_multiplier = $2,
                double_drop_multiplier = $3,
                double_gold_multiplier = $4
            WHERE id = $5
            "#,
        )
        .bind(exp)
        .bind(sp)
        .bind(drop)
        .bind(gold)
        .bind(realm_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Liga ou desliga o Realm
    pub async fn set_online_status(&self, realm_id: &str, is_online: bool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE realms SET is_online = $1 WHERE id = $2
            "#,
        )
        .bind(is_online)
        .bind(realm_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Mapas desligados pelo painel (E7, B177): `config.mapas_desligados` (JSONB), lista de ids.
    pub async fn mapas_desligados(&self, realm_id: &str) -> Result<Vec<i32>> {
        let lista: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT config->'mapas_desligados' FROM realms WHERE id = $1",
        )
        .bind(realm_id)
        .fetch_optional(self.pool.get_ref())
        .await?
        .flatten();
        Ok(lista
            .and_then(|v| serde_json::from_value::<Vec<i32>>(v).ok())
            .unwrap_or_default())
    }

    /// Mapas ligados pelo painel além do `WORLD_TAGS` (`config.mapas_ligados`, B183): a
    /// partida do GS também os carrega.
    pub async fn mapas_ligados(&self, realm_id: &str) -> Result<Vec<i32>> {
        let lista: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT config->'mapas_ligados' FROM realms WHERE id = $1",
        )
        .bind(realm_id)
        .fetch_optional(self.pool.get_ref())
        .await?
        .flatten();
        Ok(lista
            .and_then(|v| serde_json::from_value::<Vec<i32>>(v).ok())
            .unwrap_or_default())
    }

    /// Estado de um mapa pelo painel (B183), numa só instrução: ligar tira de
    /// `mapas_desligados` e põe em `mapas_ligados`; desligar faz o contrário. As outras
    /// chaves do `config` ficam intactas. Devolve se o realm existe.
    pub async fn definir_estado_do_mapa(&self, realm_id: &str, mapa: i32, ligado: bool) -> Result<bool> {
        // `$3` = pôr o mapa em `mapas_ligados`; `NOT $3` = pôr em `mapas_desligados`. O
        // segundo `config->...` lê a linha antiga, que não tem a chave mudada pelo primeiro.
        let lista = |chave: &str, pos: &str| format!(
            "coalesce((SELECT jsonb_agg(DISTINCT x ORDER BY x) FROM (                 SELECT x FROM jsonb_array_elements(coalesce(config->'{chave}', '[]'::jsonb)) x                 UNION SELECT to_jsonb($2::int)) s              WHERE {pos} OR x <> to_jsonb($2::int)), '[]'::jsonb)"
        );
        let sql = format!(
            "UPDATE realms SET config = jsonb_set(jsonb_set(coalesce(config, '{{}}'::jsonb),              '{{mapas_desligados}}', {}), '{{mapas_ligados}}', {}) WHERE id = $1",
            lista("mapas_desligados", "NOT $3"),
            lista("mapas_ligados", "$3"),
        );
        let linhas = sqlx::query(&sql)
            .bind(realm_id)
            .bind(mapa)
            .bind(ligado)
            .execute(self.pool.get_ref())
            .await?
            .rows_affected();
        Ok(linhas == 1)
    }

    /// Liga ou desliga um mapa em `config.mapas_desligados` numa só instrução: as outras
    /// chaves do `config` e os mapas de outros processos do mesmo realm ficam intactos.
    /// Devolve se o realm existe.
    pub async fn definir_mapa_desligado(&self, realm_id: &str, mapa: i32, desligado: bool) -> Result<bool> {
        let linhas = sqlx::query(
            "UPDATE realms SET config = jsonb_set(config, '{mapas_desligados}', coalesce((\
                SELECT jsonb_agg(DISTINCT x ORDER BY x) FROM ( \
                    SELECT x FROM jsonb_array_elements(coalesce(config->'mapas_desligados', '[]'::jsonb)) x \
                    UNION SELECT to_jsonb($2::int)) s \
                WHERE $3 OR x <> to_jsonb($2::int)), '[]'::jsonb)) \
             WHERE id = $1",
        )
        .bind(realm_id)
        .bind(mapa)
        .bind(desligado)
        .execute(self.pool.get_ref())
        .await?
        .rows_affected();
        Ok(linhas == 1)
    }
}
