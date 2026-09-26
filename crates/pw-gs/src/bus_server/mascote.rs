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
            EventoDoMundo::MascoteConjurou { id, alvo, skill, nivel, tempo_ms } => {
                // `gnpc_dispatcher::cast_skill` → `OBJECT_CAST_SKILL` (85) a quem vê; 15 B nas
                // duas versões (validador do cliente 1.2.6: 15).
                let pacote = S2CGamedataSend::object_cast_skill(id as i32, alvo as i32, skill, tempo_ms, nivel.clamp(0, 255) as u8).data;
                self.transmitir_a_quem_ve(id, pacote).await;
            }
            EventoDoMundo::MascoteUsouHabilidade { id, skill, nivel, alvo, .. } => {
                self.aplicar_habilidade_do_mascote(id, skill, nivel, alvo).await;
            }
            EventoDoMundo::RecargaDoMascote { dono, slot, recarga, ms } => {
                self.enviar_ao_jogador(dono, self.sub.pet_set_cooldown(slot as i32, recarga, ms).data).await;
            }
            EventoDoMundo::ErroDoMascote { dono, erro } => {
                self.enviar_ao_jogador(dono, S2CGamedataSend::error_message(erro).data).await;
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

    /// `send_pet_room(&pData, index, index + 1)` (`gs/player.cpp:5620-5627`): o `PET_ROOM`
    /// (239) de um slot só, no formato do `SendAllData` — `count`, e `{slot, pet_data}`.
    async fn avisar_slot_da_jaula(&self, dono: RoleId, slot: u16, info: &pw_core::InfoPet) {
        let mut corpo = (slot as i32).to_le_bytes().to_vec();
        corpo.extend_from_slice(&info.para_bytes());
        self.enviar_ao_jogador(dono, S2CGamedataSend::pet_room(1, &corpo).data).await;
    }

    /// O `inv.Find(0, item)` do original: o primeiro slot da bolsa com o item.
    pub(super) async fn achar_na_bolsa(&self, roleid: RoleId, item_id: i32) -> Option<u16> {
        let mut bolsa = self.itens().await.list_by_container(roleid, ContainerType::Inventory).await.unwrap_or_default();
        bolsa.sort_by_key(|i| i.slot);
        bolsa.into_iter().find(|i| i.item_id as i32 == item_id && i.count > 0).map(|i| i.slot)
    }

    /// O custo dos serviços 36 e 37 depois do sucesso (`OnServe`): o item sai da bolsa
    /// (`use_item`) e o dinheiro (`spend_money`).
    async fn cobrar_servico_de_mascote(&self, roleid: RoleId, slot_do_item: Option<u16>, item_id: i32, preco: i32) {
        if let Some(slot) = slot_do_item {
            if self.itens().await.consume_item(roleid, ContainerType::Inventory, slot, 1).await.is_ok() {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::host_use_item(0, slot as u8, item_id, 1).data).await;
            }
        }
        if preco > 0 {
            let pagou = self.com_contexto(roleid, |ctx| ctx.gastar_dinheiro(preco as i64)).await.unwrap_or(false);
            if pagou {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::spend_money(preco as u32).data).await;
            }
        }
    }

    /// As conferências de dinheiro e item comuns ao 36 e ao 37 (`TryServe` + `OnServe`):
    /// `ERR_OUT_OF_FUND` e `ERR_ITEM_NOT_IN_INVENTORY`. Devolve o slot do item exigido.
    async fn conferir_custo(&self, roleid: RoleId, (preco, item): (i32, i32)) -> Result<Option<u16>, i32> {
        let dinheiro = self.world.read().await.players.get(&(roleid as i64)).map(|p| p.money).unwrap_or(0);
        if dinheiro < preco as i64 {
            return Err(16);
        }
        if item > 0 {
            return match self.achar_na_bolsa(roleid, item).await {
                Some(s) => Ok(Some(s)),
                None => Err(5),
            };
        }
        Ok(None)
    }

    /// Serviço 36 — `change_pet_name_executor` (`serviceprovider.cpp:3896-3984`):
    /// `{u16 pet_index; u16 name_len; char name[]}`, com o nome em UTF-16 de 2 a 16 bytes e
    /// tamanho par, e o corpo fechando no nome. `pet_manager::ChangePetName`
    /// (`petman.cpp:1921-1935`) recusa o mascote que não existe e **o ativo** — então o nome
    /// novo aparece na próxima invocação (`mascote_entra`, bit 0x2000); `OnChangeName` corta
    /// em 16 bytes. Responde com o `PET_ROOM` do slot.
    pub(super) async fn renomear_mascote(&self, roleid: RoleId, c: &[u8]) {
        if c.len() < 4 {
            return;
        }
        let indice = u16::from_le_bytes([c[0], c[1]]);
        let tamanho = u16::from_le_bytes([c[2], c[3]]) as usize;
        if tamanho == 0 || tamanho > 16 || tamanho & 1 != 0 || tamanho + 4 != c.len() {
            debug!("mundo: renomear mascote de {roleid} com nome de {tamanho} bytes em {} — recusado", c.len());
            return;
        }
        let Some((npc, servicos)) = self.npc_em_conversa(roleid).await else { return };
        let Some(custo) = servicos.renomear_mascote else {
            debug!("mundo: o NPC {npc} não renomeia mascote");
            return;
        };
        let item_do_servico = match self.conferir_custo(roleid, custo).await {
            Ok(s) => s,
            Err(e) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(e).data).await;
                return;
            }
        };
        let itens = self.itens().await;
        let Ok(Some(mut item)) = itens.get_item_by_slot(roleid, ContainerType::PetCorral, indice).await else {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(72).data).await;
            return;
        };
        if self.mascote_ativo_no_slot(roleid, indice).await {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(71).data).await;
            return;
        }
        let mut info = pw_core::InfoPet::do_bloco(&item.octets).unwrap_or_default();
        info.name = [0; 16];
        info.name[..tamanho].copy_from_slice(&c[4..4 + tamanho]);
        info.name_len = tamanho as u16;
        item.octets = info.para_bytes();
        if let Err(e) = itens.upsert_item(&item).await {
            warn!("mundo: o nome do mascote do slot {indice} de {roleid} não foi gravado: {e}");
            return;
        }
        info!("mundo: {roleid} renomeou o mascote do slot {indice} ({tamanho} bytes)");
        self.avisar_slot_da_jaula(roleid, indice, &info).await;
        self.cobrar_servico_de_mascote(roleid, item_do_servico, custo.1, custo.0).await;
    }

    /// Serviço 37 — `forget_pet_skill_executor` (`serviceprovider.cpp:4075-4157`): `{int
    /// skill_id}`. `pet_manager::ForgetPetSkill` (`petman.cpp:1937-1955`) exige o mascote
    /// **ativo**; `combat_petdata_imp::OnForgetSkill` (`:890-917`) tira a habilidade e sobe as
    /// seguintes (a lista não tem buraco), e o corpo recebe a lista nova
    /// (`GM_MSG_PET_SKILL_LIST`). Falhou: `ERR_SKILL_NOT_AVAILABLE`.
    pub(super) async fn esquecer_habilidade_de_mascote(&self, roleid: RoleId, c: &[u8]) {
        let Some(skill) = c.get(0..4).filter(|_| c.len() == 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])) else { return };
        let Some((npc, servicos)) = self.npc_em_conversa(roleid).await else { return };
        let Some(custo) = servicos.esquecer_habilidade_de_mascote else {
            debug!("mundo: o NPC {npc} não faz esquecer habilidade de mascote");
            return;
        };
        let item_do_servico = match self.conferir_custo(roleid, custo).await {
            Ok(s) => s,
            Err(e) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(e).data).await;
                return;
            }
        };
        let resultado = {
            let mut mundo = self.world.write().await;
            match mundo.mascote_de(roleid as i64).map(|m| (m.slot, m.info.skills)) {
                None => Err(73),
                Some((slot, mut lista)) => match lista.iter().take_while(|(s, _)| *s > 0).position(|(s, _)| *s == skill) {
                    None => Err(20),
                    Some(i) => {
                        lista.copy_within(i + 1.., i);
                        lista[7] = (0, 0);
                        mundo.trocar_habilidades_do_mascote(roleid, lista);
                        Ok((slot, mundo.mascote_de(roleid as i64).map(|m| m.para_a_jaula())))
                    }
                },
            }
        };
        match resultado {
            Err(e) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(e).data).await;
            }
            Ok((slot, Some(info))) => {
                info!("mundo: {roleid} fez o mascote esquecer a habilidade {skill}");
                self.avisar_slot_da_jaula(roleid, slot, &info).await;
                self.gravar_mascote(roleid, slot, &info).await;
                self.cobrar_servico_de_mascote(roleid, item_do_servico, custo.1, custo.0).await;
            }
            Ok((_, None)) => {}
        }
    }

    /// Serviço 38 — `pet_skill_executor` (`serviceprovider.cpp:4211-4236`): `{int skill_id}`,
    /// da lista do NPC (`pet_skill_provider::TryServe`, `binary_search`; fora dela
    /// `ERR_SKILL_NOT_AVAILABLE`). `pet_manager::LearnSkill` (`petman.cpp:1957-1974`) exige o
    /// mascote ativo (`ERR_PET_IS_NOT_ACTIVE`); `combat_petdata_imp::OnLearnSkill`
    /// (`:919-960`) recusa uma quinta habilidade normal (`GetNormalSkillNum >= 4`), e
    /// `SkillWrapper::PetLearn` (`skillwrapper.cpp:1512-1569`): nível atual + 1 até o
    /// `max_level`, `cls == 127`, pré-requisitos entre as do mascote, nível do **mascote** ≥
    /// `GetRequiredLevel`, SP do dono ≥ `GetRequiredSp`, e o livro (`GetRequiredItem`) sai da
    /// bolsa (`DROP_TYPE_TAKEOUT`). Qualquer recusa desses é `ERR_SERVICE_UNAVILABLE`.
    pub(super) async fn aprender_habilidade_de_mascote(&self, roleid: RoleId, c: &[u8]) {
        let Some(skill) = c.get(0..4).filter(|_| c.len() == 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])) else { return };
        let Some((npc, servicos)) = self.npc_em_conversa(roleid).await else { return };
        if skill <= 0 || servicos.habilidades_de_mascote.binary_search(&(skill as u32)).is_err() {
            debug!("mundo: o NPC {npc} não ensina a habilidade de mascote {skill}");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(20).data).await;
            return;
        }
        const RECUSA: i32 = 14;
        let (dados, ativo, sp) = {
            let mundo = self.world.read().await;
            let ativo = mundo.mascote_de(roleid as i64).map(|m| (m.slot, m.info.clone()));
            let sp = mundo.players.get(&(roleid as i64)).map(|p| p.sp).unwrap_or(0);
            (Arc::clone(&mundo.data_manager), ativo, sp)
        };
        let Some((slot, info)) = ativo else {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(73).data).await;
            return;
        };
        let lista: Vec<(i32, i32)> = info.skills.iter().take_while(|(s, _)| *s > 0).copied().collect();
        let atual = lista.iter().find(|(s, _)| *s == skill).map(|(_, l)| *l);
        // Combate: sem natureza nem habilidade própria, então toda habilidade é "normal".
        if atual.is_none() && lista.len() >= 4 {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(RECUSA).data).await;
            return;
        }
        let proximo = atual.unwrap_or(0) + 1;
        let Some(h) = dados.habilidades.get(skill as u32) else {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(RECUSA).data).await;
            return;
        };
        let pre_ok = h.pre_skills.iter().all(|(pre, nivel)| *pre == 0 || lista.iter().find(|(s, _)| *s == *pre as i32).map(|(_, l)| *l).unwrap_or(0) >= *nivel);
        let (Some(nivel), Some(sp_exigido)) = (h.nivel_exigido(proximo), h.sp_exigido(proximo)) else {
            warn!("mundo: a habilidade de mascote {skill} nível {proximo} tem requisito desconhecido — recusada");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(RECUSA).data).await;
            return;
        };
        let livro = h.item_exigido(proximo).unwrap_or(0);
        if proximo > h.max_level || h.cls != Some(127) || !pre_ok || (info.level as i32) < nivel || sp < sp_exigido as i64 {
            debug!("mundo: o mascote de {roleid} não aprende {skill} nível {proximo} (nível {} / {nivel}, SP {sp} / {sp_exigido})", info.level);
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(RECUSA).data).await;
            return;
        }
        // `TakeOutItem(item) < 0` recusa.
        if livro > 0 {
            let Some(s) = self.achar_na_bolsa(roleid, livro).await else {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(RECUSA).data).await;
                return;
            };
            if self.itens().await.consume_item(roleid, ContainerType::Inventory, s, 1).await.is_err() {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(RECUSA).data).await;
                return;
            }
            self.enviar_ao_jogador(roleid, self.sub.player_drop_item(0, s as u8, 1, livro, 2).data).await;
        }
        if sp_exigido > 0 {
            let _ = self
                .com_contexto(roleid, |ctx| {
                    ctx.p.sp -= sp_exigido as i64;
                    ctx.mudou = true;
                })
                .await;
            self.enviar_ao_jogador(roleid, S2CGamedataSend::cost_skill_point(sp_exigido).data).await;
        }
        let mut nova = info.skills;
        match lista.iter().position(|(s, _)| *s == skill) {
            Some(i) => nova[i].1 = proximo,
            None => nova[lista.len()] = (skill, proximo),
        }
        let info = {
            let mut mundo = self.world.write().await;
            mundo.trocar_habilidades_do_mascote(roleid, nova);
            mundo.mascote_de(roleid as i64).map(|m| m.para_a_jaula())
        };
        let Some(info) = info else { return };
        info!("mundo: o mascote de {roleid} aprendeu a habilidade {skill} no nível {proximo}");
        self.avisar_slot_da_jaula(roleid, slot, &info).await;
        self.gravar_mascote(roleid, slot, &info).await;
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
