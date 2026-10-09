//! B208: portais de região — a saída e a entrada das masmorras.
//!
//! O cliente, ao pisar numa caixa de transporte do `region.sev`, manda sozinho o C2S 86
//! (`ENTER_INSTANCE` no cliente, `REGION_TRANSPORT` no servidor) com `{int region_index; int
//! target_tag}` (`EC_World.cpp:2360-2373`; `common/protocol.h:5548-5553`). O original
//! (`playercmd.cpp:2984-2998` → `PlayerRegionTransport` → `session_region_transport` →
//! `RegionTransport`, `player.cpp:12633-12660`) recusa com `ERR_CANNOT_ENTER_INSTANCE` se o
//! jogador não está em estado normal ou se `city_region::GetRegionTransport`
//! (`template/city_region.cpp:73-97`) falha: caixa inexistente, caixa de outro mapa
//! (`GetSrcInstanceID`), jogador fora dela (`IsPointIn`) ou destino diferente do pedido. Passando,
//! `LongJump(alvo + 0,05 em y, destino)`.
//!
//! **Limitação:** o original cria uma cópia da masmorra por grupo e a recicla quando ela fecha;
//! aqui cada masmorra é um mapa único e compartilhado (ver spec 05 e `docs/ESTADO_E_RETOMADA.md`).

use super::*;

/// `ERR_CANNOT_ENTER_INSTANCE` (`common/protocol.h:732`, 52º do enum: depois de
/// `ERR_COMMAND_IN_SEALED` 50 e `ERR_LEVEL_NOT_MATCH` 51). Cliente 1.2.6 não conferido.
pub(crate) const ERR_CANNOT_ENTER_INSTANCE: i32 = 52;

impl BusServer {
    /// C2S 86: atravessar a caixa de transporte `indice` para o mapa `destino`.
    pub(super) async fn portal_de_regiao(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        if payload.len() != 8 {
            warn!("mundo: portal de {roleid} com {} bytes (esperava 8)", payload.len());
            return;
        }
        let indice = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let destino = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
        let (este, caixa, pos, morto) = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            let caixa = usize::try_from(indice)
                .ok()
                .and_then(|i| mundo.data_manager.caixas_de_transporte.get(&mundo.world_id)?.get(i).copied());
            (mundo.world_id, caixa, p.position, p.hp <= 0)
        };
        let recusa = match caixa {
            _ if morto => Some("morto"),
            None => Some("caixa_inexistente"),
            Some(c) if c.origem != este => Some("caixa_de_outro_mapa"),
            Some(c) if !c.contem(pos.x, pos.y, pos.z) => Some("fora_da_caixa"),
            Some(c) if c.destino != destino => Some("destino_errado"),
            Some(_) if destino != este && !self.roteador.get().and_then(|r| r.upgrade()).is_some_and(|r| r.aceita_destino(destino)) => {
                Some("destino_indisponivel")
            }
            _ => None,
        };
        if let Some(motivo) = recusa {
            info!(roleid, indice, destino, motivo, x = pos.x, y = pos.y, z = pos.z, "mundo: portal recusado");
            self.responder(roleid, S2CGamedataSend::error_message(ERR_CANNOT_ENTER_INSTANCE).data, envio).await;
            return;
        }
        let c = caixa.expect("conferida acima");
        let alvo = Vector3::new(c.alvo[0], c.alvo[1] + 0.05, c.alvo[2]);
        info!(roleid, indice, de = este, para = destino, "mundo: portal de região");
        self.transportar(roleid, destino, alvo).await;
    }
}
