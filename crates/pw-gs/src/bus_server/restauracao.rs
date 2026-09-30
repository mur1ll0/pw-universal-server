//! Restauração de atributos no NPC (serviço 33, "Reverter Atributos") — B152.
//!
//! Porte do `resetprop_executor` + `resetprop_provider` (`cgame/gs/serviceprovider.cpp:3527-3690`),
//! iguais no `gs` 1.2.6 (`TryServe` VA 0x810fc0a, `OnServe` 0x810fdde):
//!
//! 1. **Pedido** `{ size_t index; int item_id; }` (8 B). `SendRequest` recusa calado se
//!    `item_id` é 0 ou o item não está na bolsa (`IsItemExist`).
//! 2. **NPC** (`TryServe`): `index` fora da lista do NPC → `ERR_SERVICE_UNAVILABLE` (14);
//!    `item_id` diferente do da opção → `ERR_ITEM_NOT_IN_INVENTORY` (5).
//! 3. **Jogador** (`OnServe`): o item na bolsa (`inv.Find(0, object_need)`, senão 5);
//!    `RegroupPropPoint` — tira os deltas até o piso da versão e devolve aos pontos livres;
//!    nada a tirar → `ERR_CAN_NOT_RESET_PP` (82, também 0x52 no 1.2.6) e o item fica. Com
//!    sucesso, `PlayerGetProperty` (`OWN_EXT_PROP`), um item a menos e `player_use_item`
//!    (`HOST_USE_ITEM` 91, `where` 0).

use super::*;

/// `ERR_ITEM_NOT_IN_INVENTORY` 5, `ERR_SERVICE_UNAVILABLE` 14, `ERR_CAN_NOT_RESET_PP` 82
/// (`common/protocol.h:680-762`; 5, 0xe e 0x52 no `gs` 1.2.6).
const ITEM_FORA_DA_BOLSA: i32 = 5;
const SERVICO_INDISPONIVEL: i32 = 14;
const NAO_PODE_RESTAURAR: i32 = 82;

impl BusServer {
    /// `GP_NPCSEV_RESETPROP` (33).
    pub(super) async fn restaurar_atributos(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let (Ok(indice), Ok(item_id)) = (r.u32(), r.i32()) else {
            warn!("mundo: pedido de restauração de atributos de {roleid} curto ({} B)", conteudo.len());
            return;
        };
        if conteudo.len() != 8 {
            warn!("mundo: pedido de restauração de atributos de {roleid} com {} B (esperados 8)", conteudo.len());
            return;
        }
        // `TryServe`: a opção pelo índice na lista do NPC em conversa.
        let opcao = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            let npc = p.npc_em_conversa.and_then(|id| mundo.npcs.get(&id)).map(|n| n.template_id);
            npc.and_then(|t| mundo.data_manager.servicos_de_npc.get(&t))
                .map(|s| s.restauracao_de_atributos.get(indice as usize).copied())
        };
        let Some(opcao) = opcao else {
            debug!("mundo: {roleid} pediu restauração de atributos sem NPC com o serviço");
            return;
        };
        let piso = self.sub.piso_da_restauracao();
        let feito = self
            .com_contexto(roleid, |ctx| {
                // `SendRequest`: item 0 ou ausente → recusa calada.
                if item_id == 0 || ctx.bolsa.contar(item_id as u32) == 0 {
                    return Err(None);
                }
                let Some(e) = opcao else { return Err(Some(SERVICO_INDISPONIVEL)) };
                if item_id != e.item {
                    return Err(Some(ITEM_FORA_DA_BOLSA));
                }
                // `OnServe`: `inv.Find(0, object_need)` — o primeiro slot com o item.
                let Some(slot) = ctx.bolsa.primeiro_slot_com(e.item as u32) else {
                    return Err(Some(ITEM_FORA_DA_BOLSA));
                };
                let base = Some(&ctx.dados.base_das_classes).filter(|b| !b.is_empty());
                let devolvidos = ctx.p.restaurar_atributos(
                    (e.forca, e.agilidade, e.vitalidade, e.energia),
                    piso,
                    &ctx.dados.classes,
                    base,
                );
                if devolvidos == 0 {
                    return Err(Some(NAO_PODE_RESTAURAR));
                }
                ctx.mudou = true;
                ctx.para_mim.push(BusServer::ficha_de(ctx.sub, ctx.p));
                ctx.bolsa.tirar_do_slot(slot, 1);
                ctx.para_mim.push(S2CGamedataSend::host_use_item(0, slot as u8, e.item, 1).data);
                Ok(devolvidos)
            })
            .await;
        match feito {
            Some(Ok(n)) => info!("mundo: {roleid} restaurou atributos com o item {item_id}: {n} pontos devolvidos"),
            Some(Err(Some(erro))) => {
                debug!("mundo: {roleid} não restaurou atributos (item {item_id}, opção {indice}): erro {erro}");
                self.responder(roleid, S2CGamedataSend::error_message(erro).data, envio).await;
            }
            Some(Err(None)) => debug!("mundo: {roleid} pediu restauração com o item {item_id}, que não tem"),
            None => {}
        }
    }
}
