//! B199: troca de rosto em jogo — o bilhete, a sessão cosmética e a gravação da aparência.
//!
//! O original, em quatro passos:
//! 1. Serviço de NPC 24 (`GP_NPCSEV_FACECHANGE`, `cosmetic_executor`,
//!    `serviceprovider.cpp:2880-2958`), pedido `{u32 inv_index, i32 item_type}`: o item tem de
//!    estar no slot, ser `FACETICKET_ESSENCE` e o nível chegar ao `require_level`; abre a sessão
//!    cosmética (`EnterCosmeticMode` → `player_cosmetic_begin`, `player.cpp:12615-12622`), que o
//!    cliente recebe como `CHANGE_FACE_START` (201) e abre a tela de rosto
//!    (`EC_HostMsg.cpp:4962-4983`).
//! 2. O cliente manda `SetCustomData` ao `gdeliveryd`, que só aceita com o bilhete
//!    (`setcustomdata.hpp`; senão `ERR_NOFACETICKET` 201). Aqui o link repassa ao GS dono, que
//!    tem o bilhete.
//! 3. `CosmeticSuccess` (`player.cpp:12969-12990`): gasta um bilhete (`DROP_TYPE_USE`), arma a
//!    recarga `COOLDOWN_INDEX_FACETICKET`, troca o `custom_crc` e difunde `cosmetic_success` =
//!    `PLAYER_CHG_FACE` (203) a quem vê, ele incluído (`AutoBroadcastCSMsg`); quem vê pede os
//!    dados novos (`EC_ElsePlayer.cpp:2418-2424`). Fecha a sessão → `CHANGE_FACE_END` (202).
//! 4. Cancelar a tela manda `CANCEL_ACTION` (`CECCustomizeHostPolicy::OnCancel`,
//!    `EC_CustomizePolicy.cpp:86-93`): a sessão termina e o cliente recebe `CHANGE_FACE_END`.
//!
//! Diferença: o original prende o jogador em `PLAYER_STATE_COSMETIC` (nada de andar, lutar ou
//! usar item); aqui só o bilhete fica guardado na entidade.

use super::*;
use crate::economia::TAMANHO_DA_BOLSA;

/// `COOLDOWN_INDEX_FACETICKET` — 8º do enum de `gs/cooldowncfg.h:62-73` (depois de `FACEPILL`).
pub(crate) const RECARGA_DO_BILHETE: i32 = 8;
/// `FACETICKET_COOLDOWN_TIME` (`gs/cooldowncfg.h:10`), em ms.
pub(crate) const RECARGA_DO_BILHETE_MS: i32 = 1000;
/// `ERR_NOFACETICKET` (`share/rpc/errcode.h:161`).
pub(crate) const ERRO_SEM_BILHETE: i32 = 201;
/// `DROP_TYPE_USE` (`common/protocol.h:927-943`, 12º do enum).
pub(crate) const DESCARTE_POR_USO: u8 = 11;

impl BusServer {
    /// Serviço 24: abre a troca de rosto se o bilhete e o nível conferem.
    pub(super) async fn iniciar_troca_de_rosto(&self, roleid: i32, c: &[u8], envio: &EnvioAoCliente) {
        if c.len() != 8 {
            warn!("mundo: troca de rosto de {roleid} com {} bytes (esperava 8)", c.len());
            return;
        }
        let slot = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        let tid = i32::from_le_bytes([c[4], c[5], c[6], c[7]]);
        let Some(nivel_exigido) = self.world.read().await.data_manager.nivel_do_bilhete_de_rosto(tid as u32) else {
            debug!("mundo: {roleid} pediu troca de rosto com {tid}, que não é FACETICKET_ESSENCE");
            return;
        };
        let no_slot = self.itens().await.get_item_by_slot(roleid, ContainerType::Inventory, slot as u16).await.ok().flatten();
        if slot >= TAMANHO_DA_BOLSA as u32 || !no_slot.is_some_and(|i| i.item_id == tid as u32 && i.count >= 1) {
            return;
        }
        {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            if p.bilhete_de_rosto.is_some() || p.hp <= 0 || p.level < nivel_exigido {
                return;
            }
            p.bilhete_de_rosto = Some((slot as u16, tid as u32));
        }
        info!(roleid, slot, tid, "mundo: troca de rosto aberta");
        self.responder(roleid, S2CGamedataSend::change_face_start(slot as u16).data, envio).await;
    }

    /// `CANCEL_ACTION` com a tela de rosto aberta: fecha a sessão sem gastar o bilhete.
    pub(super) async fn cancelar_troca_de_rosto(&self, roleid: i32, envio: &EnvioAoCliente) {
        let bilhete = self.world.write().await.players.get_mut(&(roleid as i64)).and_then(|p| p.bilhete_de_rosto.take());
        if let Some((slot, _)) = bilhete {
            info!(roleid, "mundo: troca de rosto cancelada");
            self.responder(roleid, S2CGamedataSend::change_face_end(slot).data, envio).await;
        }
    }

    /// `SetCustomData` repassado pelo link: confere o bilhete e o formato, grava, responde ao link
    /// (`SetCustomData_Re`) e conclui a troca como o `CosmeticSuccess`.
    pub(super) async fn aparencia_nova(&self, roleid: i32, localsid: u32, dados: Vec<u8>, envio: &EnvioAoCliente) {
        let responder = |result: i32, crc: u32| {
            let _ = envio.try_send(BusMessage::SetCustomDataRe { result, crc, roleid, localsid });
        };
        let bilhete = self.world.write().await.players.get_mut(&(roleid as i64)).and_then(|p| p.bilhete_de_rosto.take());
        let Some((slot, tid)) = bilhete else {
            info!(roleid, "mundo: aparência sem bilhete de rosto, recusada");
            responder(ERRO_SEM_BILHETE, 0);
            return;
        };
        let gravou = pw_core::formato_de_aparencia_valido(&dados)
            && self.repo().await.gravar_aparencia(roleid, &dados).await.unwrap_or(false);
        if !gravou {
            warn!(roleid, bytes = dados.len(), "mundo: aparência recusada (formato ou banco)");
            responder(ERRO_SEM_BILHETE, 0);
            self.responder(roleid, S2CGamedataSend::change_face_end(slot).data, envio).await;
            return;
        }
        let crc = pw_core::stamp_de_aparencia(&dados);
        responder(0, crc as u32);
        self.com_contexto(roleid, |ctx| {
            ctx.p.crc_aparencia = crc;
            // `IsItemExist(ticket_inv_idx, ticket_id, 1)` ou o primeiro com o mesmo id.
            let onde = match ctx.bolsa.item_no_slot(slot as usize) {
                Some(i) if i.item_id == tid => Some(slot as usize),
                _ => ctx.bolsa.primeiro_slot_com(tid),
            };
            if let Some(s) = onde {
                ctx.bolsa.tirar_do_slot(s, 1);
                ctx.para_mim.push(ctx.sub.player_drop_item(0, s as u8, 1, tid as i32, DESCARTE_POR_USO).data);
            }
            ctx.p.recargas.insert(RECARGA_DO_BILHETE, std::time::Instant::now() + std::time::Duration::from_millis(RECARGA_DO_BILHETE_MS as u64));
            ctx.para_mim.push(S2CGamedataSend::set_cooldown(RECARGA_DO_BILHETE, RECARGA_DO_BILHETE_MS).data);
            let pacote = S2CGamedataSend::player_chg_face(crc, roleid).data;
            ctx.para_mim.push(pacote.clone());
            ctx.para_todos.push(pacote);
            ctx.para_mim.push(S2CGamedataSend::change_face_end(slot).data);
            ctx.mudou = true;
        })
        .await;
        info!(roleid, crc, "mundo: aparência nova gravada (troca de rosto)");
    }
}
