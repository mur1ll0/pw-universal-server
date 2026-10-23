//! B363 — o PK no barramento: os avisos (117/118/119 a quem vê e a ele, 230 a ele) e a perda na morte
//! (`gplayer_imp::OnDeath`, `gs/player.cpp:7133-7213`; `player_ctrl_*::OnDeath`, `gs/playerinvade.cpp`;
//! `DropItemOnDeath`, `player.cpp`). As regras puras estão em [`crate::pk`].

use super::*;
use pw_core::RoleId;
use crate::pk::AvisoPk;

/// `LOW_PROTECT_LEVEL` (`gs/config.h:74`): até este nível a morte não custa nada.
const NIVEL_PROTEGIDO: i32 = 9;
/// `DROP_TYPE_DEATH` e `DROP_TYPE_USE` (`common/protocol.h`, enum dos descartes).
const DESCARTE_NA_MORTE: u8 = 6;
const DESCARTE_POR_USO: u8 = 11;
/// `MAX_ITEM_DROP_COUNT` (`gs/config.h:81`): quantos de uma pilha caem.
const PILHA_QUE_CAI: u32 = 20;
/// `inventory_equipment` (o que conta como bolsa): munição (11) e o slot de runa (17); `equipment_drop`: o
/// que conta como corpo. As mesmas listas no `gs` 1.2.6 (`_ZZN11gplayer_imp15DropItemOnDeath…`).
const CORPO_COMO_BOLSA: [u16; 2] = [11, 17];
const CORPO_QUE_CAI: [u16; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 18, 19];
/// `NODROP | NOTHROW | NOTRADE | BIND` (`DropItemOnDeath`): esses não caem.
const NAO_CAI: i32 = 0x0001 | 0x0002 | 0x0010 | 0x8000;

impl BusServer {
    /// O aviso de uma mudança de PK.
    pub(super) async fn avisar_pk(&self, roleid: RoleId, aviso: AvisoPk) {
        let pacote = match aviso {
            AvisoPk::Rosa => S2CGamedataSend::invader_rise(roleid).data,
            AvisoPk::Branco => S2CGamedataSend::invader_fade(roleid).data,
            AvisoPk::Vermelho(g) => S2CGamedataSend::pariah_rise(roleid, g).data,
            AvisoPk::Duracao(t) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::pariah_time(t).data).await;
                return;
            }
        };
        self.enviar_ao_jogador(roleid, pacote.clone()).await;
        self.transmitir_a_quem_ve(roleid as i64, pacote).await;
    }

    /// A perda na morte. `matador` é o `lastattack`.
    pub(super) async fn perdas_na_morte(&self, roleid: RoleId, matador: i64) {
        let tabelas = self.sub.tabelas_de_perda();
        let (nivel, pk, pvp, por_jogador) = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            (p.level, p.pk.clone(), p.pvp_ligado, mundo.players.contains_key(&matador))
        };
        // `_basic.level > LOW_PROTECT_LEVEL` (`player.cpp:7139`).
        if nivel <= NIVEL_PROTEGIDO {
            return;
        }
        // O boneco: o primeiro da lista na bolsa some (`DROP_TYPE_USE`) e a morte sai de graça — sem perda de
        // item nem de experiência (`free_resurrect` → `_kill_by_player = true`, `player.cpp:7139-7166`, `:7246-7250`).
        let boneco = self
            .com_contexto(roleid, |ctx| {
                let (id, slot) = tabelas.bonecos.iter().find_map(|&id| ctx.bolsa.primeiro_slot_com(id).map(|s| (id, s)))?;
                ctx.bolsa.tirar_do_slot(slot, 1);
                ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, 1, id as i32, DESCARTE_POR_USO).data);
                ctx.p.morto_por_jogador = true;
                Some(id)
            })
            .await
            .flatten();
        if let Some(id) = boneco {
            info!("mundo: {roleid} morreu com o boneco {id} — sem perda");
            return;
        }
        let (r1, r2) = (rand::random::<f32>(), rand::random::<f32>());
        let (n_bolsa, n_corpo) = crate::pk::perda_na_morte(&tabelas, &pk, por_jogador, pvp, r1, r2);
        if n_bolsa + n_corpo == 0 {
            return;
        }
        let (itens, dados) = (self.itens().await, self.world.read().await.data_manager.clone());
        let bolsa = itens.list_by_container(roleid, ContainerType::Inventory).await.unwrap_or_default();
        let corpo = itens.list_by_container(roleid, ContainerType::Equipment).await.unwrap_or_default();
        let proc = |i: &pw_core::ItemRecord| dados.proc_type_da_instancia(i.item_id, i.bind_status != 0);
        // `drop_entry(where, index)`: a bolsa inteira mais munição e runa; o corpo pelas peças da lista.
        let mut lista_bolsa: Vec<(u8, u16, i32)> = bolsa.iter().map(|i| (0u8, i.slot, proc(i))).collect();
        lista_bolsa.extend(corpo.iter().filter(|i| CORPO_COMO_BOLSA.contains(&i.slot)).map(|i| (1u8, i.slot, proc(i))));
        let mut lista_corpo: Vec<(u8, u16, i32)> =
            corpo.iter().filter(|i| CORPO_QUE_CAI.contains(&i.slot)).map(|i| (1u8, i.slot, proc(i))).collect();
        let mut caem = Vec::new();
        for (lista, n) in [(&mut lista_bolsa, n_bolsa), (&mut lista_corpo, n_corpo)] {
            for _ in 0..n {
                if lista.is_empty() {
                    break;
                }
                let i = rand::random::<usize>() % lista.len();
                caem.push(lista.swap_remove(i));
            }
        }
        let dono = por_jogador.then_some(matador as RoleId);
        for (onde, slot, p) in caem {
            if p & NAO_CAI != 0 {
                continue;
            }
            info!("mundo: {roleid} perdeu na morte o slot {slot} do pacote {onde}");
            self.jogar_no_chao(roleid, onde, slot as u8, PILHA_QUE_CAI, dono, DESCARTE_NA_MORTE, None).await;
        }
    }
}
