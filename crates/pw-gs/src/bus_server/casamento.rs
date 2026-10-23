//! B354 — a cerimônia de casamento: o conteúdo do serviço 62 (a agenda, C2S 36), a reserva e o cancelamento (serviço
//! 62, `WeddingBookJob`/`WeddingCancelBookJob`, `gs/teamrelationjob.cpp:133-470`), o convite (serviço 63,
//! `gplayer_imp::WeddingInvite`, `player.cpp:20868-20915`) e o uso do convite (`wedding_invitecard_item::OnUse`,
//! `item/item_wedding.cpp:12-40`). A agenda é a de [`crate::casamento::agenda`], que só existe no mapa da cerimônia.

use super::*;
use crate::casamento::{self as cas, agenda, Convite};
use crate::comandos::UseItem;

/// Códigos `ERR_*` contados a partir do `ERR_SUCCESS` (`common/protocol.h:680`).
mod erro {
    pub const ITEM_FORA_DA_BOLSA: i32 = 5;
    pub const BOLSA_CHEIA: i32 = 7;
    pub const SEM_DINHEIRO: i32 = 16;
    pub const NAO_PODE_USAR: i32 = 18;
    pub const JOGADOR_NAO_EXISTE: i32 = 46;
    pub const CERIMONIA_NAO_ESTA_ACONTECENDO: i32 = 140;
    pub const NAO_PODE_RESERVAR: i32 = 141;
    pub const NAO_PODE_CANCELAR: i32 = 142;
    pub const EQUIPE_LONGE: i32 = 243;
}

/// `GP_NPCSEV_WEDDING_BOOK`: `TYPE_BOOK`/`TYPE_CANCELBOOK` (`serviceprovider.cpp:6357-6372`).
const RESERVAR: i32 = 1;
const CANCELAR: i32 = 2;
/// `COOLDOWN_INDEX_TEAM_RELATION` (28º de `gs/cooldowncfg.h`), 1 s.
const RECARGA_DA_RELACAO: i32 = 28;
/// `DROP_TYPE_USE` (`common/protocol.h:927-941`).
const DROP_USO: u8 = 11;
/// `service_type` do serviço de reserva.
const SERVICO_DE_RESERVA: i32 = 62;

fn i32_em(b: &[u8], k: usize) -> i32 {
    i32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap())
}

fn agora_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// O convite na bolsa que é deste jogador para a cerimônia (`inv.Find(…, WEDDING_INVITECARD_ID1)` com o conteúdo
/// conferido): o slot.
fn slot_do_convite(bolsa: &crate::economia::Bolsa, eu: i32, inicio: i32, fim: i32, cena: i32) -> Option<usize> {
    bolsa.itens().iter().find_map(|it| {
        let c = Convite::ler(&it.octets)?;
        (it.item_id == cas::CONVITE_DOS_NOIVOS
            && c.inicio == inicio
            && c.fim == fim
            && c.cena == cena
            && (c.noivo == eu || c.noiva == eu)
            && c.convidado == eu)
            .then_some(it.slot as usize)
    })
}

impl BusServer {
    /// C2S `SEVNPC_GET_CONTENT` (36) `{int service_type}` (`playercmd.cpp:1921-1936`): o 62 manda a agenda
    /// (`wedding_book_provider::GetContent` → `SendBookingList`); fora do mapa da cerimônia não há agenda e nada vai.
    pub(super) async fn conteudo_do_servico(&self, roleid: i32, payload: &[u8]) {
        // B363 — a vitrine da barraca (`SERVICE_QUIERY_CONTENT` no `MarketHandler`).
        if self.conteudo_da_barraca(roleid).await {
            return;
        }
        if payload.len() != 4 {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(3).data).await;
            return;
        }
        let tipo = i32_em(payload, 0);
        if tipo != SERVICO_DE_RESERVA {
            debug!("mundo: {roleid} pediu o conteúdo do serviço {tipo}, sem conteúdo aqui");
            return;
        }
        let lista: Option<Vec<([i32; 5], u8, bool)>> = agenda().lock().ok().and_then(|a| {
            a.as_ref().map(|a| {
                a.lista.iter().map(|w| ([w.inicio as i32, w.fim as i32, w.noivo, w.noiva, w.cena], w.estado, w.especial)).collect()
            })
        });
        match lista {
            Some(l) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::wedding_book_list(&l).data).await;
            }
            None => debug!("mundo: {roleid} pediu a agenda de casamento fora do mapa da cerimônia"),
        }
    }

    /// Serviço 62 `{int type; int start; int end; int scene; int bookcard_index}` (`wedding_book_executor`).
    pub(super) async fn servico_de_reserva(&self, roleid: i32, c: &[u8]) {
        if c.len() != 20 {
            return;
        }
        let (tipo, inicio, fim, cena, cartao) = (i32_em(c, 0), i32_em(c, 1), i32_em(c, 2), i32_em(c, 3), i32_em(c, 4));
        if (tipo != RESERVAR && tipo != CANCELAR) || !(0..cas::CENAS).contains(&cena) {
            return;
        }
        if !self.npc_em_conversa(roleid).await.is_some_and(|(_, s)| s.casamento) {
            return;
        }
        let r = if tipo == RESERVAR {
            self.reservar_cerimonia(roleid, inicio, fim, cena, cartao).await
        } else {
            self.cancelar_cerimonia(roleid, inicio, fim, cena).await
        };
        if let Err(codigo) = r {
            if codigo != 0 {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(codigo).data).await;
            }
        }
    }

    /// `TeamRelationJob` (`teamrelationjob.cpp:20-90`): capitão de uma equipe de dois, recarga de 1 s, os dois aqui,
    /// vivos e a até 30 m; depois o `OnTeamCheck` do casamento (sexos diferentes e casados um com o outro, se
    /// `casados`). Devolve `(capitão, membro, noivo, noiva)`.
    async fn casal_da_equipe(&self, lider: i32, casados: bool, erro_da_recarga: i32) -> Result<(i32, i32, i32, i32), i32> {
        let mut mundo = self.world.write().await;
        let membros = mundo.membros_do_grupo(lider);
        if membros.len() != 2 || membros.first() != Some(&lider) {
            return Err(erro_da_recarga);
        }
        let membro = membros[1];
        let agora = std::time::Instant::now();
        let Some(p) = mundo.players.get_mut(&(lider as i64)) else { return Err(0) };
        if p.recargas.get(&RECARGA_DA_RELACAO).is_some_and(|ate| *ate > agora) {
            return Err(erro_da_recarga);
        }
        p.recargas.insert(RECARGA_DA_RELACAO, agora + std::time::Duration::from_millis(1000));
        let centro = p.position;
        let (Some(a), Some(b)) = (mundo.players.get(&(lider as i64)), mundo.players.get(&(membro as i64))) else {
            return Err(0);
        };
        if a.hp <= 0 || b.hp <= 0 {
            return Err(0);
        }
        if b.position.distance_squared(&centro) > 30.0 * 30.0 {
            return Err(erro::EQUIPE_LONGE);
        }
        let mulher = |g: pw_core::Gender| g == pw_core::Gender::Female;
        if mulher(a.gender) == mulher(b.gender) {
            return Err(0);
        }
        if casados && (a.conjuge != membro || b.conjuge != lider) {
            return Err(0);
        }
        let (noivo, noiva) = if mulher(a.gender) { (membro, lider) } else { (lider, membro) };
        Ok((lider, membro, noivo, noiva))
    }

    /// `gplayer_imp::WeddingBook` + `WeddingBookJob` (`player.cpp:20787-20828`, `teamrelationjob.cpp:133-312`).
    async fn reservar_cerimonia(&self, roleid: i32, inicio: i32, fim: i32, cena: i32, cartao: i32) -> Result<(), i32> {
        let casado = self.world.read().await.players.get(&(roleid as i64)).is_some_and(|p| p.conjuge != 0);
        let (lider, membro, noivo, noiva) = self.casal_da_equipe(roleid, true, erro::NAO_PODE_RESERVAR).await?;
        if !casado {
            return Err(erro::NAO_PODE_RESERVAR);
        }
        let dados = self.world.read().await.data_manager.clone();
        let cartoes = dados.casamento.as_ref().map(|c| c.cartoes_de_reserva.clone()).unwrap_or_default();
        // `OnLeaderCheck`: 2 vagas, o bilhete, o cartão (se veio) e o `CheckBook`.
        let dia = self
            .com_contexto(lider, |ctx| {
                if ctx.bolsa.livres() < 2 {
                    return Err(erro::BOLSA_CHEIA);
                }
                if ctx.bolsa.contar(cas::BILHETE_DE_RESERVA) < 1 {
                    return Err(erro::ITEM_FORA_DA_BOLSA);
                }
                if cartao < 0 {
                    return Ok([0, 0, 0]);
                }
                let it = ctx.bolsa.item_no_slot(cartao as usize).ok_or(erro::ITEM_FORA_DA_BOLSA)?;
                let modelo = cartoes.get(&it.item_id).ok_or(erro::ITEM_FORA_DA_BOLSA)?;
                // O conteúdo do cartão (`generate_wedding_bookcard`: ano, mês, dia); sem ele, o do modelo.
                Ok(if it.octets.len() >= 12 { [i32_em(&it.octets, 0), i32_em(&it.octets, 1), i32_em(&it.octets, 2)] } else { *modelo })
            })
            .await
            .ok_or(0)??;
        let agora = agora_unix();
        let pode = agenda().lock().ok().and_then(|a| a.as_ref().map(|a| a.pode_reservar(inicio as i64, fim as i64, cena, dia, agora)));
        if pode != Some(true) {
            return Err(erro::NAO_PODE_RESERVAR);
        }
        // `OnMemberCheck`: 1 vaga (o erro vai ao membro) e o bilhete.
        let membro_ok = self
            .com_contexto(membro, |ctx| {
                if ctx.bolsa.livres() < 1 {
                    return Err(erro::BOLSA_CHEIA);
                }
                if ctx.bolsa.contar(cas::BILHETE_DE_RESERVA) < 1 {
                    return Err(0);
                }
                Ok(())
            })
            .await
            .ok_or(0)?;
        if let Err(e) = membro_ok {
            if e != 0 {
                self.enviar_ao_jogador(membro, S2CGamedataSend::error_message(e).data).await;
            }
            return Err(0);
        }
        let reservou = agenda()
            .lock()
            .ok()
            .and_then(|mut a| a.as_mut().map(|a| a.reservar(inicio as i64, fim as i64, noivo, noiva, cena, agora)));
        if reservou != Some(true) {
            for quem in [lider, membro] {
                self.enviar_ao_jogador(quem, S2CGamedataSend::error_message(erro::NAO_PODE_RESERVAR).data).await;
            }
            return Ok(());
        }
        let validade = fim.saturating_add(86_400);
        for (quem, e_lider) in [(lider, true), (membro, false)] {
            let convite = Convite { inicio, fim, noivo, noiva, cena, convidado: quem };
            self.com_contexto(quem, |ctx| {
                if let Some(e) = ctx.bolsa.guardar_equipamento(cas::CONVITE_DOS_NOIVOS, &convite.octetos(), ctx.dados) {
                    ctx.para_mim.push(ctx.sub.obtain_item(cas::CONVITE_DOS_NOIVOS as i32, validade, 1, e.no_slot, 0, e.slot as u8).data);
                }
                if e_lider {
                    if let Some(e) = ctx.bolsa.empilhar(cas::AUTORIZACAO, 1, ctx.dados) {
                        ctx.para_mim.push(ctx.sub.obtain_item(cas::AUTORIZACAO as i32, 0, 1, e.no_slot, 0, e.slot as u8).data);
                    }
                }
                for (slot, n) in ctx.bolsa.tirar(cas::BILHETE_DE_RESERVA, 1) {
                    ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, n, cas::BILHETE_DE_RESERVA as i32, DROP_USO).data);
                }
                if e_lider && cartao >= 0 {
                    if let Some(tid) = ctx.bolsa.item_no_slot(cartao as usize).map(|i| i.item_id) {
                        ctx.bolsa.tirar_do_slot(cartao as usize, 1);
                        ctx.para_mim.push(ctx.sub.player_drop_item(0, cartao as u8, 1, tid as i32, DROP_USO).data);
                    }
                }
                ctx.mudou = true;
                ctx.para_mim.push(S2CGamedataSend::wedding_book_success(RESERVAR).data);
            })
            .await;
        }
        info!("mundo: cerimônia de {noivo} e {noiva} reservada ({inicio}–{fim}, cena {cena})");
        Ok(())
    }

    /// `gplayer_imp::WeddingCancelBook` + `WeddingCancelBookJob` (`player.cpp:20830-20866`,
    /// `teamrelationjob.cpp:318-470`).
    async fn cancelar_cerimonia(&self, roleid: i32, inicio: i32, fim: i32, cena: i32) -> Result<(), i32> {
        let (lider, membro, noivo, noiva) = self.casal_da_equipe(roleid, false, erro::NAO_PODE_CANCELAR).await?;
        let pode = |a: &Option<cas::AgendaDeCasamento>| a.as_ref().is_some_and(|a| a.pode_cancelar(inicio as i64, fim as i64, noivo, noiva, cena));
        if !agenda().lock().ok().is_some_and(|a| pode(&a)) {
            return Err(erro::NAO_PODE_CANCELAR);
        }
        let slot_lider = self
            .com_contexto(lider, |ctx| {
                if ctx.bolsa.livres() < 1 {
                    return Err(erro::BOLSA_CHEIA);
                }
                let s = slot_do_convite(&ctx.bolsa, lider, inicio, fim, cena).ok_or(erro::ITEM_FORA_DA_BOLSA)?;
                if ctx.p.money < cas::TAXA_DE_CANCELAMENTO {
                    return Err(erro::SEM_DINHEIRO);
                }
                Ok(s)
            })
            .await
            .ok_or(0)??;
        let slot_membro = self
            .com_contexto(membro, |ctx| {
                if ctx.bolsa.livres() < 1 {
                    return Err(erro::BOLSA_CHEIA);
                }
                slot_do_convite(&ctx.bolsa, membro, inicio, fim, cena).ok_or(0)
            })
            .await
            .ok_or(0)??;
        let cancelou = agenda()
            .lock()
            .ok()
            .and_then(|mut a| a.as_mut().map(|a| a.cancelar(inicio as i64, fim as i64, noivo, noiva, cena)));
        if cancelou != Some(true) {
            for quem in [lider, membro] {
                self.enviar_ao_jogador(quem, S2CGamedataSend::error_message(erro::NAO_PODE_CANCELAR).data).await;
            }
            return Ok(());
        }
        for (quem, slot, e_lider) in [(lider, slot_lider, true), (membro, slot_membro, false)] {
            self.com_contexto(quem, |ctx| {
                // O bilhete de volta; `pile_limit == 1` ganharia validade de 24 h, que a bolsa daqui não guarda.
                if let Some(e) = ctx.bolsa.empilhar(cas::BILHETE_DE_RESERVA, 1, ctx.dados) {
                    ctx.para_mim.push(ctx.sub.obtain_item(cas::BILHETE_DE_RESERVA as i32, 0, 1, e.no_slot, 0, e.slot as u8).data);
                }
                ctx.bolsa.tirar_do_slot(slot, 1);
                ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, 1, cas::CONVITE_DOS_NOIVOS as i32, DROP_USO).data);
                if e_lider && ctx.gastar_dinheiro(cas::TAXA_DE_CANCELAMENTO) {
                    ctx.para_mim.push(S2CGamedataSend::spend_money(cas::TAXA_DE_CANCELAMENTO as u32).data);
                }
                ctx.mudou = true;
                ctx.para_mim.push(S2CGamedataSend::wedding_book_success(CANCELAR).data);
            })
            .await;
        }
        info!("mundo: cerimônia de {noivo} e {noiva} cancelada ({inicio}–{fim}, cena {cena})");
        Ok(())
    }

    /// Serviço 63 `{int invitecard_index; int invitee}` (`wedding_invite_executor`, `WeddingInvite`).
    pub(super) async fn servico_de_convite(&self, roleid: i32, c: &[u8]) {
        if c.len() != 8 {
            return;
        }
        let (indice, convidado) = (i32_em(c, 0), i32_em(c, 1));
        if indice < 0 || convidado < 0 {
            return;
        }
        if !self.npc_em_conversa(roleid).await.is_some_and(|(_, s)| s.casamento) {
            return;
        }
        let r = self
            .com_contexto(roleid, |ctx| {
                let it = ctx.bolsa.item_no_slot(indice as usize).ok_or(erro::ITEM_FORA_DA_BOLSA)?;
                if it.item_id != cas::CONVITE_DOS_NOIVOS {
                    return Err(erro::ITEM_FORA_DA_BOLSA);
                }
                let c = Convite::ler(&it.octets).ok_or(erro::ITEM_FORA_DA_BOLSA)?;
                if c.convidado != roleid {
                    return Err(erro::ITEM_FORA_DA_BOLSA);
                }
                if convidado == c.noivo || convidado == c.noiva {
                    return Err(erro::JOGADOR_NAO_EXISTE);
                }
                if ctx.bolsa.contar(cas::BILHETE_DE_CONVITE) < 1 {
                    return Err(erro::ITEM_FORA_DA_BOLSA);
                }
                if ctx.bolsa.livres() < 1 {
                    return Err(erro::BOLSA_CHEIA);
                }
                let novo = Convite { convidado, ..c };
                let e = ctx.bolsa.guardar_equipamento(cas::CONVITE_DO_CONVIDADO, &novo.octetos(), ctx.dados).ok_or(erro::BOLSA_CHEIA)?;
                ctx.para_mim.push(
                    ctx.sub.obtain_item(cas::CONVITE_DO_CONVIDADO as i32, c.fim.saturating_add(86_400), 1, e.no_slot, 0, e.slot as u8).data,
                );
                for (slot, n) in ctx.bolsa.tirar(cas::BILHETE_DE_CONVITE, 1) {
                    ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, n, cas::BILHETE_DE_CONVITE as i32, DROP_USO).data);
                }
                ctx.mudou = true;
                Ok(())
            })
            .await;
        match r {
            Some(Ok(())) => info!("mundo: {roleid} convidou {convidado} para a cerimônia"),
            Some(Err(codigo)) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(codigo).data).await;
            }
            None => {}
        }
    }

    /// `wedding_invitecard_item::OnUse`: a cerimônia tem de estar acontecendo; o convite com convidado só serve a
    /// ele; leva à posição da cena **neste** mapa (`PosInWorld`: só no mapa da cerimônia) e põe o filtro da cena
    /// (`wedding_filter::OnAttach`: `ENTER_WEDDING_SCENE`). O convite não se gasta. `falta`: o estado visível 22 dos
    /// noivos (`IncVisibleState(22)`).
    pub(super) async fn usar_convite_de_casamento(&self, roleid: i32, u: &UseItem, octetos: &[u8]) {
        let Some(c) = Convite::ler(octetos) else { return };
        let acontecendo = agenda().lock().ok().is_some_and(|a| a.as_ref().is_some_and(|a| a.em_andamento(c.noivo, c.noiva, c.cena)));
        if !acontecendo {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro::CERIMONIA_NAO_ESTA_ACONTECENDO).data).await;
            return;
        }
        if c.convidado != 0 && c.convidado != roleid {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro::NAO_PODE_USAR).data).await;
            return;
        }
        let (mapa, pos) = {
            let mundo = self.world.read().await;
            let pos = mundo.data_manager.casamento.as_ref().and_then(|cfg| cfg.cenas.get(c.cena as usize).copied());
            (mundo.world_id, pos)
        };
        let Some(pos) = pos else { return };
        // Só o 1.5.5 tem mapa de cerimônia (o `gs.conf` 1.2.6 não tem `wedding_server`), e a agenda só existe lá.
        if pw_data_loader::limites::mapa_do_casamento(pw_data_loader::limites::CatalogoDeLimites::V155) != Some(mapa) {
            debug!("mundo: {roleid} usou o convite fora do mapa da cerimônia ({mapa})");
            return;
        }
        let _ = u;
        self.transportar(roleid, mapa, Vector3::new(pos[0], pos[1], pos[2])).await;
        if let Ok(mut n) = cas::na_cena().lock() {
            n.insert(roleid, (c.noivo, c.noiva, c.cena));
        }
        self.enviar_ao_jogador(roleid, S2CGamedataSend::enter_wedding_scene(c.noivo, c.noiva).data).await;
        info!("mundo: {roleid} entrou na cerimônia de {} e {} (cena {})", c.noivo, c.noiva, c.cena);
    }
}
