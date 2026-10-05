//! Registro técnico durável: comando e efeito da conta na mesma transação.
//! Ausência do registro é desconhecida: pode haver transação concorrente em andamento.
use crate::{PostgresPool, Result};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct ComandoAdministrativoRepository {
    pool: PostgresPool,
}

impl ComandoAdministrativoRepository {
    pub fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    pub async fn consultar(&self, id: &str, administrador: i32) -> Result<Value> {
        let registro: Option<Value> = sqlx::query_scalar(
            "SELECT resultado FROM comandos_administrativos WHERE operacao_id=$1 AND administrador_id=$2",
        ).bind(id).bind(administrador).fetch_optional(self.pool.get_ref()).await?;
        let mut resultado =
            registro.unwrap_or_else(|| json!({"estado":"desconhecido", "codigo":"nao_registrado"}));
        if resultado["tipo"] == "definir_gm" && resultado["estado"] == "pendente" {
            let revisao = resultado["revisao"].as_i64().unwrap_or(-1);
            let conta = resultado["conta_id"].as_i64().unwrap_or(0) as i32;
            let alvos: Vec<String> =
                serde_json::from_value(resultado["processos"].clone()).unwrap_or_default();
            // Estado da conta e recibos no MESMO snapshot SQL. Uma revisão posterior
            // pode ter pulado esta operação; recibos novos sozinhos não provam efeito antigo.
            let fotografia: Option<(i64,Vec<String>)> = sqlx::query_as(
                "SELECT revisao_gm, ARRAY(SELECT processo FROM coordenacao_gm_processos WHERE processo=ANY($2) AND revisao >= $3 AND atualizado_em > NOW()-INTERVAL '5 seconds') FROM accounts WHERE id=$1")
                .bind(conta).bind(&alvos).bind(revisao).fetch_optional(self.pool.get_ref()).await?;
            if fotografia.as_ref().map(|(r, _)| *r) != Some(revisao) {
                resultado["estado"] = json!("substituido");
                resultado["sessoes"] = json!("revisao_posterior");
            } else {
                let recebidos = fotografia.unwrap().1;
                let faltam: Vec<_> = alvos
                    .iter()
                    .filter(|a| !recebidos.contains(a))
                    .cloned()
                    .collect();
                resultado["processos_pendentes"] = json!(faltam);
                if !alvos.is_empty() && faltam.is_empty() {
                    resultado["estado"] = json!("aplicado");
                    resultado["sessoes"] = json!("reconciliadas");
                }
            }
            // Confirmação terminal durável. Consultas concorrentes nunca regressam o resultado.
            if resultado["estado"] != "pendente" {
                let gravado: Option<Value> = sqlx::query_scalar("UPDATE comandos_administrativos SET resultado=$2 WHERE operacao_id=$1 AND resultado->>'estado'='pendente' RETURNING resultado")
                    .bind(id).bind(&resultado).fetch_optional(self.pool.get_ref()).await?;
                // Outra consulta pode ter confirmado aplicação antes da revisão nova.
                // Devolver o vencedor durável, nunca a conclusão local que perdeu o CAS.
                resultado = match gravado {
                    Some(valor) => valor,
                    None => sqlx::query_scalar("SELECT resultado FROM comandos_administrativos WHERE operacao_id=$1 AND administrador_id=$2")
                        .bind(id).bind(administrador).fetch_one(self.pool.get_ref()).await?,
                };
            }
        }
        Ok(resultado)
    }

    /// Nível 0 remove GM, 1 concede todos os privilégios já implementados (B159).
    /// O painel não inventa hierarquia: os bits individuais do original seguem pendentes.
    pub async fn definir_gm(
        &self,
        id: &str,
        administrador: i32,
        realm: &str,
        conta: i32,
        habilitado: bool,
        impressao: &[u8],
        processos: &[String],
    ) -> Result<Value> {
        if processos.is_empty() {
            return Ok(json!({"estado":"falha","codigo":"coordenacao_nao_configurada"}));
        }
        let mut tx = self.pool.get_ref().begin().await?;
        sqlx::query("SET LOCAL lock_timeout='1500ms'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='2000ms'")
            .execute(&mut *tx)
            .await?;
        let inserido = sqlx::query("INSERT INTO comandos_administrativos(operacao_id,administrador_id,realm_origem,conta_id,impressao,resultado) VALUES($1,$2,$3,$4,$5,'{}') ON CONFLICT DO NOTHING")
            .bind(id).bind(administrador).bind(realm).bind(conta).bind(impressao).execute(&mut *tx).await?.rows_affected()==1;
        // Locks de contas ordenados, também em concorrência com senha/revogação.
        let contas: Vec<(i32,i32,bool)> = sqlx::query_as("SELECT id,gm_privileges,is_banned FROM accounts WHERE id IN($1,$2) ORDER BY id FOR UPDATE")
            .bind(administrador).bind(conta).fetch_all(&mut *tx).await?;
        if !contas
            .iter()
            .any(|(id, g, b)| *id == administrador && *g > 0 && !b)
        {
            tx.rollback().await?;
            return Ok(json!({"estado":"falha","codigo":"administrador_recusado"}));
        }
        if !inserido {
            let (dono, alvo_anterior, anterior, resultado): (i32,Option<i32>,Vec<u8>,Value) = sqlx::query_as("SELECT administrador_id,conta_id,impressao,resultado FROM comandos_administrativos WHERE operacao_id=$1")
                .bind(id).fetch_one(&mut *tx).await?;
            tx.rollback().await?;
            if dono != administrador || alvo_anterior != Some(conta) || anterior != impressao {
                return Ok(json!({"estado":"falha","codigo":"operacao_em_conflito"}));
            }
            return if resultado["tipo"] == "definir_gm" {
                self.consultar(id, administrador).await
            } else {
                Ok(resultado)
            };
        }
        let resultado = if contas.iter().any(|(id, _, _)| *id == conta) {
            // Singleton transacional ordena os commits; SERIAL deixaria buracos/inversões.
            let revisao: i64 = sqlx::query_scalar(
                "UPDATE coordenacao_gm_revisao SET revisao=revisao+1 WHERE unico RETURNING revisao",
            )
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query("UPDATE accounts SET gm_privileges=$2,revisao_gm=$3 WHERE id=$1")
                .bind(conta)
                .bind(if habilitado { 1 } else { 0 })
                .bind(revisao)
                .execute(&mut *tx)
                .await?;
            json!({"estado":"pendente","tipo":"definir_gm","conta_id":conta,"habilitado":habilitado,
                "revisao":revisao,"alcance":"global","persistencia":"salva","sessoes":"pendentes",
                "cliente":"reconexao_necessaria","processos":processos})
        } else {
            json!({"estado":"falha","codigo":"conta_inexistente","conta_id":conta})
        };
        sqlx::query("UPDATE comandos_administrativos SET resultado=$2 WHERE operacao_id=$1")
            .bind(id)
            .bind(&resultado)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(resultado)
    }

    pub async fn trocar_senha(
        &self,
        id: &str,
        administrador: i32,
        realm: &str,
        conta: i32,
        impressao: &[u8],
        hash_senha: &str,
    ) -> Result<Value> {
        let mut transacao = self.pool.get_ref().begin().await?;
        // Limites internos, inferiores ao timeout do canal; nada toca lock do mundo.
        sqlx::query("SET LOCAL lock_timeout='1500ms'")
            .execute(&mut *transacao)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='2000ms'")
            .execute(&mut *transacao)
            .await?;
        // ON CONFLICT espera quem reservou o mesmo ID; rollback não deixa tombstone.
        let inserido = sqlx::query(
            "INSERT INTO comandos_administrativos(operacao_id,administrador_id,realm_origem,conta_id,impressao,resultado) \
             VALUES($1,$2,$3,$4,$5,'{}') ON CONFLICT DO NOTHING",
        ).bind(id).bind(administrador).bind(realm).bind(conta).bind(impressao)
         .execute(&mut *transacao).await?.rows_affected() == 1;
        // Ordem única inclusive quando duas contas GM alteram uma à outra.
        let contas: Vec<(i32, i32, bool)> = sqlx::query_as(
            "SELECT id,gm_privileges,is_banned FROM accounts WHERE id IN ($1,$2) ORDER BY id FOR UPDATE",
        ).bind(administrador).bind(conta).fetch_all(&mut *transacao).await?;
        if !contas
            .iter()
            .any(|(id, gm, ban)| *id == administrador && *gm > 0 && !ban)
        {
            transacao.rollback().await?;
            return Ok(json!({"estado":"falha", "codigo":"administrador_recusado"}));
        }
        if !inserido {
            let (dono, alvo_anterior, anterior, resultado): (i32, Option<i32>, Vec<u8>, Value) = sqlx::query_as(
                "SELECT administrador_id,conta_id,impressao,resultado FROM comandos_administrativos WHERE operacao_id=$1",
            ).bind(id).fetch_one(&mut *transacao).await?;
            transacao.rollback().await?;
            return Ok(
                if dono == administrador && alvo_anterior == Some(conta) && anterior == impressao {
                    resultado
                } else {
                    json!({"estado":"falha", "codigo":"operacao_em_conflito"})
                },
            );
        }
        let resultado = if contas.iter().any(|(id, _, _)| *id == conta) {
            sqlx::query("UPDATE accounts SET password_hash=$1 WHERE id=$2")
                .bind(hash_senha)
                .bind(conta)
                .execute(&mut *transacao)
                .await?;
            json!({"estado":"salvo", "tipo":"trocar_senha", "conta_id":conta,
                   "alcance":"global", "efeito":"novos_logins", "sessoes_de_jogo":"mantidas"})
        } else {
            json!({"estado":"falha", "codigo":"conta_inexistente", "conta_id":conta})
        };
        sqlx::query("UPDATE comandos_administrativos SET resultado=$2 WHERE operacao_id=$1")
            .bind(id)
            .bind(&resultado)
            .execute(&mut *transacao)
            .await?;
        transacao.commit().await?;
        Ok(resultado)
    }

    /// Reserva sem alvo numérico; conta/ID/resultado só ficam visíveis no mesmo commit.
    pub async fn criar_conta(
        &self,
        id: &str,
        administrador: i32,
        realm: &str,
        usuario: &str,
        impressao: &[u8],
        hash_senha: &str,
    ) -> Result<Value> {
        let mut transacao = self.pool.get_ref().begin().await?;
        sqlx::query("SET LOCAL lock_timeout='1500ms'")
            .execute(&mut *transacao)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='2000ms'")
            .execute(&mut *transacao)
            .await?;
        let inserido = sqlx::query(
            "INSERT INTO comandos_administrativos(operacao_id,administrador_id,realm_origem,conta_id,impressao,resultado) \
             VALUES($1,$2,$3,NULL,$4,'{}') ON CONFLICT DO NOTHING",
        ).bind(id).bind(administrador).bind(realm).bind(impressao)
         .execute(&mut *transacao).await?.rows_affected() == 1;
        let autorizado: Option<(i32, bool)> =
            sqlx::query_as("SELECT gm_privileges,is_banned FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(administrador)
                .fetch_optional(&mut *transacao)
                .await?;
        if !autorizado.is_some_and(|(gm, ban)| gm > 0 && !ban) {
            transacao.rollback().await?;
            return Ok(json!({"estado":"falha", "codigo":"administrador_recusado"}));
        }
        if !inserido {
            let (dono, anterior, resultado): (i32, Vec<u8>, Value) = sqlx::query_as(
                "SELECT administrador_id,impressao,resultado FROM comandos_administrativos WHERE operacao_id=$1",
            ).bind(id).fetch_one(&mut *transacao).await?;
            transacao.rollback().await?;
            return Ok(if dono == administrador && anterior == impressao {
                resultado
            } else {
                json!({"estado":"falha", "codigo":"operacao_em_conflito"})
            });
        }
        // Mesma chave LOWER(username) de AccountRepository::find_by_username.
        // Índice UNIQUE arbitra concorrência em qualquer produtor, sem SELECT+INSERT.
        // Defaults: specs/01_DATABASE_SCHEMA_POSTGRES.sql:19-22 (saldo/GM/ban).
        let conta: Option<i32> = sqlx::query_scalar(
            "INSERT INTO accounts(username,password_hash) VALUES($1,$2) ON CONFLICT DO NOTHING RETURNING id",
        ).bind(usuario).bind(hash_senha).fetch_optional(&mut *transacao).await?;
        let resultado = if let Some(conta) = conta {
            json!({"estado":"salvo", "tipo":"criar_conta", "conta_id":conta,
                   "usuario":usuario, "alcance":"global"})
        } else {
            json!({"estado":"falha", "codigo":"usuario_existente", "usuario":usuario})
        };
        sqlx::query(
            "UPDATE comandos_administrativos SET conta_id=$2,resultado=$3 WHERE operacao_id=$1",
        )
        .bind(id)
        .bind(conta)
        .bind(&resultado)
        .execute(&mut *transacao)
        .await?;
        transacao.commit().await?;
        Ok(resultado)
    }
}
