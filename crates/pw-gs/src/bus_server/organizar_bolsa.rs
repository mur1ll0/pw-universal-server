//! B320: organizar a bolsa.
//!
//! - **Jogo (1.5.5):** o cliente ordena e manda `C2S::MULTI_EXCHANGE_ITEM` (126) com a lista de trocas. O servidor
//!   (`playercmd.cpp:3655-3700`) confere o tamanho exato (`2 + 2·count`, `count > 0`, senão `ERR_FATAL_ERR`), a recarga
//!   `COOLDOWN_INDEX_MULTI_EXCHANGE_ITEM` (47) — em recarga o comando é ignorado, sem aviso — e a põe em 10.000 ms
//!   (`cooldowncfg.h:30`; `SetCoolDown` sempre manda `set_cooldown`, `player.cpp:12701-12709`). Na bolsa
//!   (`IL_INVENTORY` = 0) cada par é um `PlayerExchangeInvItem` (`player.cpp:7514-7527`): índice inválido dá
//!   `ERR_FATAL_ERR`, e cada troca feita manda `exchange_inventory_item` (o `EXG_IVTR_ITEM`). O 1.2.6 não tem o comando
//!   (o `gs` dele só trata ids até 0x6c).
//! - **Painel (1.2.6 e 1.5.5):** o servidor faz a mesma conta do cliente (`crate::organizar`) e aplica as trocas pelo
//!   mesmo caminho; com o jogador no mundo, cada troca vai ao cliente como `EXG_IVTR_ITEM`, que os dois clientes
//!   entendem.
//!
//! Falta: os armazéns (`IL_TRASH_BOX*`, que o original também aceita) e a trava `_lock_inventory`
//! (`ERR_INVENTORY_IS_LOCKED`), que o `pw-gs` não modela.

use pw_core::ContainerType;
use pw_protocol::packets::S2CGamedataSend;
use tracing::{debug, info, warn};

use super::jogo::erro_s2c;
use super::{BusServer, EnvioAoCliente};
use crate::economia::TAMANHO_DA_BOLSA;
use crate::organizar::{trocas, ItemParaOrdenar};

/// `COOLDOWN_INDEX_MULTI_EXCHANGE_ITEM` — 47º do enum de `gs/cooldowncfg.h` (e `GP_CT_MULTI_EXCHANGE_ITEM` no cliente).
pub(crate) const RECARGA_DA_TROCA_EM_LOTE: i32 = 47;
/// `MULTI_EXCHANGE_ITEM_COOLDOWN_TIME` (`gs/cooldowncfg.h:30`).
pub(crate) const RECARGA_DA_TROCA_EM_LOTE_MS: i32 = 10_000;
/// `gplayer_imp::IL_INVENTORY` (`player_imp.h:1830`).
const IL_INVENTORY: u8 = 0;

/// `C2S::CMD::multi_exchange_item`: `location`, `count` e `count` pares `(index1, index2)`; `None` se o tamanho não
/// fecha (`playercmd.cpp:3658`).
pub(crate) fn ler_trocas_em_lote(payload: &[u8]) -> Option<(u8, Vec<(u8, u8)>)> {
    let (&local, &n) = (payload.first()?, payload.get(1)?);
    if n == 0 || payload.len() != 2 + 2 * n as usize {
        return None;
    }
    Some((local, payload[2..].chunks(2).map(|p| (p[0], p[1])).collect()))
}

impl BusServer {
    /// `C2S::MULTI_EXCHANGE_ITEM` (126, 1.5.5).
    pub(super) async fn trocas_em_lote(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let Some((local, pares)) = ler_trocas_em_lote(payload) else {
            warn!("mundo: multi_exchange_item de {roleid} com {} B que não fecham com a contagem", payload.len());
            self.responder(roleid, S2CGamedataSend::error_message(erro_s2c::ERRO_FATAL).data, envio).await;
            return;
        };
        let agora = std::time::Instant::now();
        let livre = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            if p.recargas.get(&RECARGA_DA_TROCA_EM_LOTE).is_some_and(|ate| *ate > agora) {
                false
            } else {
                p.recargas.insert(RECARGA_DA_TROCA_EM_LOTE, agora + std::time::Duration::from_millis(RECARGA_DA_TROCA_EM_LOTE_MS as u64));
                true
            }
        };
        if !livre {
            debug!("mundo: {roleid} mandou multi_exchange_item em recarga — ignorado (playercmd.cpp:3666)");
            return;
        }
        self.responder(roleid, S2CGamedataSend::set_cooldown(RECARGA_DA_TROCA_EM_LOTE, RECARGA_DA_TROCA_EM_LOTE_MS).data, envio).await;
        if local != IL_INVENTORY {
            debug!("mundo: {roleid} quer organizar o local {local}; só a bolsa está portada");
            return;
        }
        let feitas = self.aplicar_trocas_na_bolsa(roleid, &pares, Some(envio)).await;
        info!("mundo: {roleid} organizou a bolsa ({feitas} de {} trocas)", pares.len());
    }

    /// Cada par como `PlayerExchangeInvItem`: índice fora da bolsa dá `ERR_FATAL_ERR`; a troca feita grava e, com
    /// `envio`, vai ao cliente como `EXG_IVTR_ITEM`. Devolve quantas foram feitas.
    async fn aplicar_trocas_na_bolsa(&self, roleid: i32, pares: &[(u8, u8)], envio: Option<&EnvioAoCliente>) -> usize {
        let itens = self.itens().await;
        let mut feitas = 0;
        for &(a, b) in pares {
            if a as usize >= TAMANHO_DA_BOLSA || b as usize >= TAMANHO_DA_BOLSA {
                if let Some(e) = envio {
                    self.responder(roleid, S2CGamedataSend::error_message(erro_s2c::ERRO_FATAL).data, e).await;
                }
                continue;
            }
            if let Err(e) = itens.swap_slots(roleid, ContainerType::Inventory, a as u16, b as u16).await {
                warn!("mundo: falha ao trocar {a} e {b} na bolsa de {roleid}: {e:?}");
                continue;
            }
            feitas += 1;
            if let Some(e) = envio {
                self.responder(roleid, S2CGamedataSend::exg_ivtr_item(a, b).data, e).await;
            }
        }
        feitas
    }

    /// As trocas que organizam a bolsa do personagem agora (a conta do cliente, `crate::organizar`).
    pub(crate) async fn trocas_para_organizar(&self, roleid: i32) -> Result<Vec<(u8, u8)>, &'static str> {
        let lista = self.itens().await.list_by_container(roleid, ContainerType::Inventory).await.map_err(|_| "banco_indisponivel")?;
        let dados = self.world.read().await.data_manager.clone();
        let mut slots: Vec<Option<ItemParaOrdenar>> = vec![None; TAMANHO_DA_BOLSA];
        for i in &lista {
            let Some(s) = slots.get_mut(i.slot as usize) else { continue };
            let (classe, nivel) = dados.chave_de_ordem(i.item_id);
            // `IVTR_ESSENCE_GENERALCARD`: `type` nos bytes 0–3, `rank` nos 4–7 (`EC_IvtrType.h`).
            let ler = |k: usize| i.octets.get(k..k + 4).map_or(0, |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            *s = Some(ItemParaOrdenar { classe, nivel, tid: i.item_id, carta: (ler(0), ler(4)) });
        }
        Ok(trocas(&slots))
    }

    /// Painel, com o jogador no mundo: organiza e avisa o cliente troca a troca.
    pub(crate) async fn organizar_bolsa_pelo_painel(&self, roleid: i32) -> Option<serde_json::Value> {
        let envio = self.envio_de(roleid).await?;
        Some(match self.trocas_para_organizar(roleid).await {
            Ok(pares) => {
                let feitas = self.aplicar_trocas_na_bolsa(roleid, &pares, Some(&envio)).await;
                info!(roleid, feitas, "painel: bolsa organizada (online)");
                serde_json::json!({"erro": null, "trocas": feitas})
            }
            Err(codigo) => serde_json::json!({"erro": codigo}),
        })
    }

    /// Painel, sem o jogador no mundo: organiza no banco.
    pub(crate) async fn organizar_bolsa_offline(&self, roleid: i32) -> Result<usize, &'static str> {
        let pares = self.trocas_para_organizar(roleid).await?;
        let feitas = self.aplicar_trocas_na_bolsa(roleid, &pares, None).await;
        info!(roleid, feitas, "painel: bolsa organizada (offline)");
        Ok(feitas)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_o_lote_e_recusa_tamanho_que_nao_fecha() {
        assert_eq!(ler_trocas_em_lote(&[0, 2, 1, 0, 5, 3]), Some((0, vec![(1, 0), (5, 3)])));
        assert_eq!(ler_trocas_em_lote(&[0, 2, 1, 0, 5]), None, "falta um byte");
        assert_eq!(ler_trocas_em_lote(&[0, 0]), None, "count 0");
        assert_eq!(ler_trocas_em_lote(&[0]), None);
    }
}
