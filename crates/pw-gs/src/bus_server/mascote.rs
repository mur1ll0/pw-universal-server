//! O mascote de combate no barramento: o que o mundo decide vira os S2C do dono e de quem vê,
//! e o registro do mascote volta à jaula (`PetCorral`). A simulação está em
//! [`crate::mascote`] e em `WorldInstance::invocar_mascote` e vizinhos.

use super::*;
use pw_core::RoleId;

impl BusServer {
    /// Os eventos de mascote de [`EventoDoMundo`].
    pub(super) async fn evento_de_mascote(&self, ev: EventoDoMundo) {
        match ev {
            EventoDoMundo::MascoteApareceu { id, dono } => self.anunciar_mascote(id, dono).await,
            EventoDoMundo::MascoteRecolhido { dono, slot, pet_tid, motivo, info, .. } => {
                // `RecallPetWithoutFree` → `recall_pet(índice, pet_tid, motivo)`
                // (`petman.cpp:1375`).
                self.enviar_ao_jogador(dono, self.sub.recall_pet(slot as i32, pet_tid, motivo).data).await;
                self.gravar_mascote(dono, slot, &info).await;
            }
            EventoDoMundo::MascoteMorreu { id, dono, slot, pet_tid, matador, info } => {
                // A criatura morre à vista de todos e some (`_corpse_delay` 0).
                self.transmitir_a_outros(0, S2CGamedataSend::npc_died(id as i32, matador as i32).data).await;
                self.transmitir_a_outros(0, S2CGamedataSend::object_disappear(id as i32).data).await;
                // `OnPetDeath`: `recall_pet(..., PET_DEATH)` e então `pet_dead(índice)`; o
                // `PetDeath` avisa a lealdade nova (`petman.cpp:1777-1797`).
                self.enviar_ao_jogador(dono, self.sub.recall_pet(slot as i32, pet_tid, crate::mascote::RECOLHIDO_POR_MORTE).data).await;
                self.enviar_ao_jogador(dono, self.sub.pet_dead(slot as i32).data).await;
                self.enviar_ao_jogador(dono, self.sub.pet_honor_point(slot as i32, info.honor_point).data).await;
                info!("mundo: o mascote {pet_tid} de {dono} morreu (lealdade {})", info.honor_point);
                self.gravar_mascote(dono, slot, &info).await;
            }
            EventoDoMundo::GolpeEntreCriaturas { atacante, alvo, dano, velocidade } => {
                let pacote = self.sub.object_attack_result(atacante as i32, alvo as i32, dano, 0, velocidade).data;
                self.transmitir_a_quem_ve(atacante, pacote).await;
            }
            EventoDoMundo::VidaDoMascote { dono, slot, fator, hp } => {
                self.enviar_ao_jogador(dono, self.sub.pet_hp_notify(slot as i32, fator, hp, 0.0, 0).data).await;
            }
            EventoDoMundo::ExpDoMascote { dono, slot, pet_tid, ganho, subiu, info } => {
                let pacote = if subiu {
                    info!("mundo: o mascote {pet_tid} de {dono} subiu para o nível {}", info.level);
                    self.sub.pet_levelup(slot as i32, pet_tid, info.level as i32, info.exp).data
                } else {
                    self.sub.pet_receive_exp(slot as i32, pet_tid, ganho).data
                };
                self.enviar_ao_jogador(dono, pacote).await;
                self.gravar_mascote(dono, slot, &info).await;
            }
            EventoDoMundo::FomeDoMascote { dono, slot, lealdade, fome, info } => {
                // `notify_pet_honor` e `notify_pet_hunger` (`petman.cpp:1709-1710`, `:1745-1746`).
                self.enviar_ao_jogador(dono, self.sub.pet_honor_point(slot as i32, lealdade).data).await;
                self.enviar_ao_jogador(dono, self.sub.pet_hunger_gauge(slot as i32, fome).data).await;
                self.gravar_mascote(dono, slot, &info).await;
            }
            EventoDoMundo::ReviverMascote { dono } => self.reviver_mascote(dono).await,
            EventoDoMundo::IaDoMascote { dono, agressividade, movimento } => {
                self.enviar_ao_jogador(dono, self.sub.pet_ai_state(agressividade, movimento).data).await;
            }
            _ => {}
        }
    }

    /// Entrou no mundo: `SUMMON_PET` com o id da criatura e o `PET_AI_STATE` ao dono
    /// (`ActivePet`, `petman.cpp:1333-1337`; `DoActivePet`, `:602`), e o `NPC_ENTER_WORLD`
    /// de mascote a quem está perto.
    async fn anunciar_mascote(&self, id: i64, dono: RoleId) {
        let (perto, pacote, dono_pacotes) = {
            let mut mundo = self.world.write().await;
            let Some(m) = mundo.mascotes.get(&id) else { return };
            let (pos, tid, vis, dir, nome) = (m.corpo.position, m.info.pet_tid, m.vis_tid, m.ai.direcao, m.nome.clone());
            let dono_pacotes = vec![
                self.sub.summon_pet(m.slot as i32, m.info.pet_tid, id as i32, 0).data,
                self.sub.pet_ai_state(m.ai.agressividade, m.ai.movimento).data,
                self.sub.pet_hp_notify(m.slot as i32, m.fator_de_vida(), m.corpo.hp as i32, 0.0, 0).data,
            ];
            let ids: Vec<i64> = mundo
                .players
                .iter()
                .filter(|(_, p)| p.position.distance(&pos) <= RAIO_DE_VISAO)
                .map(|(pid, _)| *pid)
                .collect();
            for pid in &ids {
                if let Some(p) = mundo.players.get_mut(pid) {
                    p.visiveis.insert(id);
                }
            }
            let pacote = self.sub.mascote_entra(16, id as i32, tid, vis as i32, pos, dir, dono, &nome).data;
            (ids, pacote, dono_pacotes)
        };
        for pid in perto {
            self.enviar_ao_jogador(pid as i32, pacote.clone()).await;
        }
        for p in dono_pacotes {
            self.enviar_ao_jogador(dono, p).await;
        }
    }

    /// O registro do mascote de volta ao slot da jaula (`pet_data` no `PetCorral`).
    pub(super) async fn gravar_mascote(&self, dono: RoleId, slot: u16, info: &pw_core::InfoPet) {
        let itens = self.itens().await;
        let Ok(Some(mut item)) = itens.get_item_by_slot(dono, ContainerType::PetCorral, slot).await else {
            warn!("mundo: o mascote do slot {slot} de {dono} sumiu da jaula antes de ser gravado");
            return;
        };
        item.octets = info.para_bytes();
        if let Err(e) = itens.upsert_item(&item).await {
            warn!("mundo: mascote do slot {slot} de {dono} não gravado: {e}");
        }
    }

    /// Invocar um mascote de combate, ao fim da canalização (`PlayerSummonPet` →
    /// `pet_manager::ActivePet`). `Err` com o `ERR_*` do original.
    pub(super) async fn invocar_mascote_de_combate(&self, roleid: RoleId, slot: u16, info: pw_core::InfoPet) {
        let (pet_tid, nivel) = (info.pet_tid, info.level);
        let r = self.world.write().await.invocar_mascote(roleid, slot, info);
        match r {
            Ok(id) => info!("mundo: {roleid} invocou o mascote {pet_tid} (nível {nivel}) como {id}"),
            Err(codigo) => {
                debug!("mundo: {roleid} não invocou o mascote {pet_tid}: erro {codigo}");
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(codigo).data).await;
            }
        }
    }

    /// `item_pet_food::OnUse` (`gs/item/item_petfood.cpp:8-20`): recarga 18 armada recusa com
    /// `ERR_OBJECT_IS_COOLING`; alimentou, gasta uma e arma 60 s; não alimentou (sem mascote,
    /// comida errada), o erro vai e o item fica. O slot congelado sempre destrava.
    pub(super) async fn alimentar_mascote(
        &self,
        roleid: RoleId,
        u: &crate::comandos::UseItem,
        (honra, tipo): (i32, i32),
        envio: &crate::bus_server::EnvioAoCliente,
    ) {
        let agora = std::time::Instant::now();
        let em_recarga = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .and_then(|p| p.recargas.get(&crate::mascote::RECARGA_DA_COMIDA))
            .is_some_and(|ate| *ate > agora);
        if em_recarga {
            self.responder(roleid, S2CGamedataSend::error_message(53).data, envio).await;
        } else {
            let r = self.world.write().await.alimentar_mascote(roleid, honra, tipo);
            match r {
                Ok(()) => {
                    if self.itens().await.consume_item(roleid, ContainerType::Inventory, u.slot, 1).await.is_ok() {
                        self.responder(roleid, S2CGamedataSend::host_use_item(u.onde, u.slot as u8, u.item_id, 1).data, envio).await;
                    }
                    if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
                        p.recargas.insert(
                            crate::mascote::RECARGA_DA_COMIDA,
                            agora + std::time::Duration::from_millis(crate::mascote::RECARGA_DA_COMIDA_MS as u64),
                        );
                    }
                    self.responder(
                        roleid,
                        S2CGamedataSend::set_cooldown(crate::mascote::RECARGA_DA_COMIDA, crate::mascote::RECARGA_DA_COMIDA_MS).data,
                        envio,
                    )
                    .await;
                    info!("mundo: {roleid} alimentou o mascote com {}", u.item_id);
                }
                Err(codigo) => self.responder(roleid, S2CGamedataSend::error_message(codigo).data, envio).await,
            }
        }
        self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(u.onde, u.slot).data, envio).await;
    }

    /// `pet_manager::ResurrectPet(pImp)` (`petman.cpp:1891-1906`): o primeiro mascote morto
    /// da jaula volta com `hp_factor` 0,1 e `PET_REVIVE`; sem nenhum morto,
    /// `ERR_PET_IS_NOT_DEAD` (88) pelo `OI_ResurrectPet` (`player.cpp:15329-15335`).
    pub(super) async fn reviver_mascote(&self, dono: RoleId) {
        let itens = self.itens().await;
        let mut jaula = itens.list_by_container(dono, ContainerType::PetCorral).await.unwrap_or_default();
        jaula.sort_by_key(|i| i.slot);
        for mut item in jaula {
            let Some(mut info) = pw_core::InfoPet::do_bloco(&item.octets) else { continue };
            if info.hp_factor > 0.0 {
                continue;
            }
            info.hp_factor = 0.1;
            item.octets = info.para_bytes();
            if let Err(e) = itens.upsert_item(&item).await {
                warn!("mundo: o mascote do slot {} de {dono} não foi revivido: {e}", item.slot);
                return;
            }
            info!("mundo: {dono} reviveu o mascote {} do slot {}", info.pet_tid, item.slot);
            self.enviar_ao_jogador(dono, self.sub.pet_revive(item.slot as i32, 0.1).data).await;
            return;
        }
        self.enviar_ao_jogador(dono, S2CGamedataSend::error_message(88).data).await;
    }

    /// `PET_CTRL_CMD` (C2S 103): `{int target; int pet_cmd; char buf[]}`
    /// (`common/protocol.h`, `playercmd.cpp:3286-3297`).
    pub(super) async fn ordem_ao_mascote(&self, roleid: RoleId, payload: &[u8]) {
        if payload.len() < 8 {
            return;
        }
        let alvo = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let comando = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
        self.world.write().await.ordem_ao_mascote(roleid, alvo, comando, &payload[8..]);
    }
}
