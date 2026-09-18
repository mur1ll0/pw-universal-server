//! A configuração que o cliente guarda no servidor — barras de atalho, layout, opções — e
//! as marcas de ajuda. Blobs opacos: ver `scripts/2026_09_16_configuracao_do_cliente.sql`.

use crate::error::Result;
use crate::postgres::PostgresPool;
use pw_core::RoleId;

/// Maior bloco aceito. O cliente descomprime num buffer de 8.192 bytes
/// (`LoadConfigsFromServer`, `EC_GameRun.cpp:2166`) e o que chega é o comprimido; o limite
/// só impede que alguém encha o banco por uma sessão.
pub const TAMANHO_MAXIMO_DA_CONFIGURACAO: usize = 65_536;

#[derive(Clone)]
pub struct ClientConfigRepository {
    pool: PostgresPool,
}

impl ClientConfigRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    /// O `ui_config` gravado, ou vazio para quem nunca gravou (o cliente usa o padrão).
    pub async fn ui_config(&self, role_id: RoleId) -> Result<Vec<u8>> {
        let v: Option<(Option<Vec<u8>>,)> =
            sqlx::query_as("SELECT ui_config FROM character_client_config WHERE character_id = $1")
                .bind(role_id)
                .fetch_optional(self.pool.get_ref())
                .await?;
        Ok(v.and_then(|(b,)| b).unwrap_or_default())
    }

    /// O `ui_config` do molde da classe do personagem (`class_templates.ui_config`), para quem
    /// nunca gravou. É o `config_data` do `GRoleBase` do molde do `clsconfig`, que o `gamedbd`
    /// original copia para o personagem novo (`clsconfig.h::ImportClsConfig`): barra com as
    /// habilidades iniciais e o rastreador de missões ligado.
    pub async fn ui_config_do_molde(&self, role_id: RoleId) -> Result<Vec<u8>> {
        let v: Option<(Option<Vec<u8>>,)> = sqlx::query_as(
            "SELECT ct.ui_config FROM characters c
             JOIN class_templates ct ON ct.realm_id = c.realm_id AND ct.cls = c.cls
             WHERE c.id = $1",
        )
        .bind(role_id)
        .fetch_optional(self.pool.get_ref())
        .await?;
        Ok(v.and_then(|(b,)| b).unwrap_or_default())
    }

    pub async fn gravar_ui_config(&self, role_id: RoleId, dados: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT INTO character_client_config (character_id, ui_config, updated_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP)
             ON CONFLICT (character_id) DO UPDATE SET
                 ui_config = EXCLUDED.ui_config, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(role_id)
        .bind(dados)
        .execute(self.pool.get_ref())
        .await?;
        Ok(())
    }

    /// As marcas de ajuda gravadas, ou `None` para quem nunca gravou.
    pub async fn help_states(&self, role_id: RoleId) -> Result<Option<Vec<u8>>> {
        let v: Option<(Option<Vec<u8>>,)> =
            sqlx::query_as("SELECT help_states FROM character_client_config WHERE character_id = $1")
                .bind(role_id)
                .fetch_optional(self.pool.get_ref())
                .await?;
        Ok(v.and_then(|(b,)| b))
    }

    pub async fn gravar_help_states(&self, role_id: RoleId, dados: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT INTO character_client_config (character_id, help_states, updated_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP)
             ON CONFLICT (character_id) DO UPDATE SET
                 help_states = EXCLUDED.help_states, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(role_id)
        .bind(dados)
        .execute(self.pool.get_ref())
        .await?;
        Ok(())
    }
}
