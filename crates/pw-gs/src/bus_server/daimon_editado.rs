//! Painel (E6, B215): o Daimon (o "pequeno elfo" do original, `elf_item`) do personagem.
//!
//! - **Ver:** a ficha do bloco do item no slot 23 (`EQUIP_INDEX_ELF`, `gs/item.h:219`) — em jogo a
//!   da memória ([`crate::entity::DaimonVestido`]), fora dele a do banco (`Daimon::ler`).
//! - **Dar EXP:** pela regra do `elf_item::InsertExp` (`gs/item/item_elf.cpp:692-750`,
//!   [`crate::entity::DaimonVestido::receber_exp`]), com o nível do dono como "nível de quem deu" —
//!   o mesmo que o jogo usa para a parte que o Daimon recebe do jogador. Sobe quantos níveis
//!   couberem e **para a um ponto** de passar o dono. Em jogo vai `ELF_EXP` (283) ou, subindo, a
//!   ficha inteira do item (`daimon_recebe`); fora do jogo grava o bloco.

use super::*;
use crate::entity::{Daimon, DaimonVestido};
use pw_core::{ContainerType, ItemRecord};
use pw_data_loader::GameDataManager;

/// A ficha que o painel mostra.
pub(crate) fn ficha_do_daimon(d: &DaimonVestido, nome: Option<&String>, exp_para_subir: u32, origem: &str) -> serde_json::Value {
    let e = &d.estado;
    serde_json::json!({
        "item_id": d.item_id, "nome": nome, "origem": origem,
        "nivel": e.nivel, "exp": e.exp, "exp_para_subir": exp_para_subir,
        "pontos_livres": e.total_de_atributos,
        "forca": e.forca, "agilidade": e.agilidade, "vitalidade": e.vitalidade, "energia": e.energia,
        "pontos_de_genio": e.total_de_genios, "genios": e.genios,
        "refino": e.refino, "vigor": e.vigor,
        "habilidades": e.habilidades.iter().map(|(id, n)| [*id as i32, *n as i32]).collect::<Vec<_>>(),
        "equipamento": e.equipamento,
    })
}

/// A experiência que o Daimon precisa para subir do nível atual: a do jogador × `exp_factor`
/// (`item_elf.cpp:696-710`).
fn exp_para_subir(d: &DaimonVestido, dados: &GameDataManager) -> u32 {
    (dados.progressao.exp_para_subir(d.estado.nivel as i32).clamp(0, u32::MAX as i64) as f64 * d.fator_de_exp as f64) as u32
}

/// O Daimon de um item do slot 23 lido do banco.
fn do_item(item: &ItemRecord, dados: &GameDataManager) -> Option<DaimonVestido> {
    let (fator, iniciais) = dados.dados_do_daimon(item.item_id)?;
    Some(DaimonVestido {
        slot: item.slot,
        item_id: item.item_id,
        fator_de_exp: fator,
        estado: Daimon::ler(&item.octets).unwrap_or_else(|| Daimon::novo(&iniciais)),
        sujo: false,
    })
}

impl BusServer {
    /// A ficha do Daimon: a da memória quando o dono está neste mapa, senão a do banco.
    pub(crate) async fn daimon_do_painel(&self, roleid: i32) -> serde_json::Value {
        let (dados, vivo) = {
            let mundo = self.world.read().await;
            let vivo = mundo.players.get(&(roleid as i64)).map(|p| p.daimon.clone());
            (mundo.data_manager.clone(), vivo)
        };
        let (d, origem) = match vivo {
            Some(d) => (d, "em_jogo"),
            None => {
                let itens = match self.itens().await.list_by_container(roleid, ContainerType::Equipment).await {
                    Ok(v) => v,
                    Err(_) => return serde_json::json!({"codigo":"banco_indisponivel"}),
                };
                (itens.iter().find(|i| i.slot == jogo::SLOT_DO_DAIMON).and_then(|i| do_item(i, &dados)), "persistida")
            }
        };
        match d {
            None => serde_json::json!({"daimon": null, "origem": origem}),
            Some(d) => serde_json::json!({
                "daimon": ficha_do_daimon(&d, dados.nomes_de_itens.get(&d.item_id), exp_para_subir(&d, &dados), origem),
            }),
        }
    }

    /// Offline (com a guarda de presença e a trava de gravação de quem chama): `InsertExp` no bloco
    /// do banco. Devolve `(nível antes, nível depois)`.
    pub(crate) async fn daimon_exp_offline(&self, roleid: i32, exp: i64) -> Result<(i16, i16), &'static str> {
        let dados = self.world.read().await.data_manager.clone();
        let repo = self.itens().await;
        let mut item = repo.list_by_container(roleid, ContainerType::Equipment).await.map_err(|_| "banco_indisponivel")?
            .into_iter().find(|i| i.slot == jogo::SLOT_DO_DAIMON).ok_or("sem_daimon")?;
        let mut d = do_item(&item, &dados).ok_or("sem_daimon")?;
        let nivel_do_dono = self.repo().await.get_details_por_role(roleid).await.map_err(|_| "banco_indisponivel")?
            .ok_or("personagem_inexistente")?.level.clamp(0, i16::MAX as i32) as i16;
        let antes = d.estado.nivel;
        let tabela = &dados.progressao;
        let (ganhou, _) = d.receber_exp(exp.clamp(0, u32::MAX as i64) as u32, nivel_do_dono, nivel_do_dono, |n| {
            tabela.exp_para_subir(n as i32).clamp(0, u32::MAX as i64) as u32
        });
        if !ganhou {
            return Err("sem_mudanca");
        }
        item.octets = d.estado.bloco();
        repo.upsert_item(&item).await.map_err(|_| "banco_indisponivel")?;
        info!(roleid, exp, antes, depois = d.estado.nivel, "painel: EXP do Daimon (offline)");
        Ok((antes, d.estado.nivel))
    }
}
