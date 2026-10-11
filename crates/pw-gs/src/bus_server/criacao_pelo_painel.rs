//! Painel (B216): criar item **no slot escolhido** e dar mascote, e o catálogo de mascotes.
//!
//! **Item no slot.** O prêmio de missão (`TASK_DELIVER_ITEM`) não serve: o cliente refaz a
//! entrega com o próprio `MergeItem` (empilha ou acha o primeiro vazio) e, se o slot dele não
//! for o do servidor, descarta o comando (`EC_HostMsg.cpp:1181-1216`). O que o cliente aceita sem
//! ter pedido é o recipiente inteiro: `OWN_IVTR_DATA`/`OWN_IVTR_DETAIL_DATA` → `ResetItems` em
//! qualquer pacote, armazém incluído (`OnMsgHstIvtrInfo`, `EC_HostMsg.cpp:2471-2520`) — o mesmo
//! que o login manda ([`S2CGamedataSend::own_ivtr_from_items`]). Então: o item é **gerado** como
//! o drop (`empilhar_gerado`: equipamento sorteado, voo, roupa, ovo), gravado no slot e, com o
//! personagem em jogo, o recipiente vai de novo ao cliente. O banco é a fonte que o
//! `com_contexto` relê a cada operação, e quem chama segura a trava de gravação.
//!
//! Recipientes e tamanhos de `gs/config.h:12-17`: bolsa e bolsa de missão 32, armazém 16. O item
//! de missão só vai à bolsa de missão, e o resto só às outras duas (é onde o jogo os guarda:
//! `DeliverCommonItem`/`DeliverTaskItem`, `task/taskman.cpp:281-330`). Uma pilha por vez.
//!
//! **Dar mascote.** A incubação do jogo (`incubar_mascote`, `hatch_pet_service_executor`,
//! `serviceprovider.cpp:3160-3240`) sem o ovo nem as moedas: o primeiro slot livre abaixo das
//! vagas da jaula (`pet_manager::AddPetData`, `petman.cpp:1487-1503`), a mesma restrição de
//! classe, o `info_pet` da essência do ovo, e em jogo o `GAIN_PET` (231).

use super::*;
use crate::economia::{Bolsa, TAMANHO_DA_BOLSA, TAMANHO_DA_BOLSA_DE_MISSAO};
use pw_core::{InfoPet, ItemRecord};

/// O pacote do cliente e o tamanho de cada recipiente onde o painel cria item.
fn pacote_e_tamanho(recipiente: ContainerType) -> Option<(u8, usize)> {
    match recipiente {
        ContainerType::Inventory => Some((0, TAMANHO_DA_BOLSA)),
        ContainerType::TaskInventory => Some((2, TAMANHO_DA_BOLSA_DE_MISSAO)),
        ContainerType::Storehouse => Some((crate::comandos::IL_TRASH_BOX, super::armazem::TAMANHO_DO_ARMAZEM)),
        _ => None,
    }
}

impl BusServer {
    /// Cria `quantidade` de `tid` no `slot` de `recipiente` (vazio). `online`: reenvia o recipiente.
    pub(crate) async fn criar_item_no_slot(&self, roleid: i32, tid: u32, quantidade: u32,
        recipiente: ContainerType, slot: u16, online: bool) -> Result<serde_json::Value, &'static str> {
        let (pacote, tamanho) = pacote_e_tamanho(recipiente).ok_or("edicao_invalida")?;
        if slot as usize >= tamanho {
            return Err("slot_invalido");
        }
        let dados = self.world.read().await.data_manager.clone();
        if dados.e_item_de_missao(tid) != (recipiente == ContainerType::TaskInventory) {
            return Err("recipiente_errado");
        }
        if quantidade == 0 || quantidade > dados.limite_de_pilha(tid) {
            return Err("quantidade_invalida");
        }
        let repo = self.itens().await;
        let mut itens = repo.list_by_container(roleid, recipiente).await.map_err(|_| "banco_indisponivel")?;
        if itens.iter().any(|i| i.slot == slot) {
            return Err("slot_ocupado");
        }
        // A geração do drop num recipiente de um slot, depois posto no slot pedido.
        let mut avulsa = Bolsa::nova(roleid, recipiente, 1, vec![]);
        let entrou = avulsa.empilhar_gerado(tid, quantidade, &dados).map(|e| e.entrou).ok_or("item_inexistente")?;
        let mut item: ItemRecord = avulsa.slots[0].take().ok_or("item_inexistente")?;
        item.slot = slot;
        repo.upsert_item(&item).await.map_err(|_| "banco_indisponivel")?;
        if online {
            itens.push(item);
            self.enviar_ao_jogador(roleid, S2CGamedataSend::own_ivtr_from_items(pacote, tamanho as u8, &itens).data).await;
        }
        info!(roleid, tid, entrou, ?recipiente, slot, online, "painel: item criado no slot");
        Ok(serde_json::json!({"item": tid, "entrou": entrou, "slot": slot}))
    }

    /// Dá o mascote do ovo `ovo`: a geração da incubação, sem custo. `online`: `GAIN_PET`.
    pub(crate) async fn dar_mascote(&self, roleid: i32, ovo: u32, online: bool) -> Result<serde_json::Value, &'static str> {
        let dados = self.world.read().await.data_manager.clone();
        let ovo_info = dados.dados_do_ovo(ovo).ok_or("ovo_inexistente")?.clone();
        let vivo = self.world.read().await.players.get(&(roleid as i64)).map(|p| (p.cls as i32, p.vagas_na_jaula));
        let (classe, vagas) = match vivo.filter(|_| online) {
            Some(v) => v,
            None => {
                let repo = self.repo().await;
                let d = repo.get_details_por_role(roleid).await.map_err(|_| "banco_indisponivel")?
                    .ok_or("personagem_inexistente")?;
                (d.cls as i32, repo.vagas_da_jaula(roleid).await.unwrap_or(1))
            }
        };
        let ess = super::jogo::essencia_do_ovo(&ovo_info);
        // A mesma recusa da incubação (`incubar_mascote`).
        if ((1 << (classe & 0x1F)) & ess.require_class) == 0 {
            return Err("classe_nao_permitida");
        }
        let repo = self.itens().await;
        let ocupados: std::collections::HashSet<u16> = repo.list_by_container(roleid, ContainerType::PetCorral).await
            .map_err(|_| "banco_indisponivel")?.iter().map(|p| p.slot).collect();
        let slot = (0..vagas as u16).find(|s| !ocupados.contains(s)).ok_or("jaula_cheia")?;
        let bytes = InfoPet::de_essencia(&ess).para_bytes();
        let mut registro = ItemRecord::new(roleid, ContainerType::PetCorral, slot, ovo_info.id_pet, 1);
        registro.octets = bytes.clone();
        repo.upsert_item(&registro).await.map_err(|_| "banco_indisponivel")?;
        if online {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::gain_pet(slot as i32, &bytes).data).await;
        }
        info!(roleid, ovo, mascote = ovo_info.id_pet, slot, online, "painel: mascote dado");
        Ok(serde_json::json!({"ovo": ovo, "mascote": ovo_info.id_pet, "slot": slot}))
    }

    /// Até 60 ovos de mascote por nome (do ovo ou do mascote) ou id, com tipo e ambiente.
    /// `tipo`: o `pet_class` (0 montaria, 1 combate, 2 seguidor, 3 invocação, 4 planta,
    /// 5 evolução — `pw_data_loader::pet::carregar_ovos`); `ambiente`: `terra`, `agua` ou `ar` (o mascote
    /// que vive nele, `inhabit_type`).
    pub(crate) async fn buscar_mascotes(&self, texto: &str, tipo: Option<i32>, ambiente: Option<&str>) -> serde_json::Value {
        let dados = self.world.read().await.data_manager.clone();
        let alvo = texto.trim().to_lowercase();
        let id: Option<u32> = alvo.parse().ok();
        let vive = |habitat: i32, ambiente: &str| match ambiente {
            "terra" => matches!(habitat, 0 | 3 | 4 | 6),
            "agua" => matches!(habitat, 1 | 3 | 5 | 6),
            "ar" => matches!(habitat, 2 | 4 | 5 | 6),
            _ => true,
        };
        let mut achados: Vec<serde_json::Value> = dados.ovos_de_pet.values()
            .filter(|o| tipo.map_or(true, |t| o.pet_class == t))
            .filter(|o| ambiente.map_or(true, |a| vive(dados.ambientes_dos_mascotes.get(&o.id_pet).copied().unwrap_or(0), a)))
            .filter_map(|o| {
                let (modelo, icone) = dados.mascotes_do_elements.get(&o.id_pet).cloned().unwrap_or_default();
                let nome_do_ovo = dados.nomes_de_itens.get(&o.id).cloned().unwrap_or_default();
                let bate = alvo.is_empty() || id == Some(o.id) || id == Some(o.id_pet)
                    || modelo.to_lowercase().contains(&alvo) || nome_do_ovo.to_lowercase().contains(&alvo);
                bate.then(|| serde_json::json!({
                    "ovo": o.id, "nome_do_ovo": nome_do_ovo, "mascote": o.id_pet, "nome": modelo,
                    "icone": icone.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                    "tipo": o.pet_class, "ambiente": dados.ambientes_dos_mascotes.get(&o.id_pet),
                    "nivel": o.level, "nivel_exigido": o.req_level, "classes": o.req_class,
                    "habilidades": o.skills.iter().map(|(s, n)| [*s, *n]).collect::<Vec<_>>(),
                }))
            })
            .collect();
        achados.sort_by_key(|m| (id != m["ovo"].as_u64().map(|v| v as u32), m["ovo"].as_u64()));
        achados.truncate(60);
        serde_json::json!({"estado":"consultado","mascotes":achados})
    }
}
