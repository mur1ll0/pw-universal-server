//! B357 — o serviço 89, renascer carta (`generalcard_rebirth_service_executor`).

use super::*;

impl BusServer {
    /// Serviço 89 `{u8 major_inv_idx; u8 minor_inv_idx}`: NPC com `combined_services2 & 0x2000`; o erro vai ao cliente
    /// (`error_message(ret)`).
    pub(super) async fn renascer_carta(&self, roleid: i32, c: &[u8]) {
        if c.len() != 2 {
            return;
        }
        if !self.npc_em_conversa(roleid).await.is_some_and(|(_, s)| s.renascer_carta) {
            return;
        }
        let (a, b) = (c[0] as usize, c[1] as usize);
        match self.com_contexto(roleid, |ctx| super::jogo::renascer_carta(ctx, a, b)).await {
            Some(Ok(())) => info!("mundo: {roleid} renasceu a carta do slot {a}"),
            Some(Err(codigo)) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(codigo).data).await;
            }
            None => {}
        }
    }
}
