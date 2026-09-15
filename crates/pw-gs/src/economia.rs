//! Drop de monstro e a bolsa do jogador.
//!
//! # Drop
//!
//! `gnpc_imp::DropItemFromData` (`cgame/gs/npc.cpp:2649-2717`) com
//! `itemdataman::generate_item_from_monster` (`template/itemdataman.cpp:1191-1216`):
//!
//! * itens: com chance `drop_adj` (ajuste pela diferença de nível), `drop_times` rodadas; em
//!   cada uma sorteia quantos caem (`probability_drop_num0..3`) e, para cada um, qual dos 32
//!   `drop_matters` — a partir da 2ª rodada, índice ≥ 16 é descartado;
//! * dinheiro: `drop_times` vezes, `Rand(médio − variação, médio + variação)`; com chance
//!   `MONEY_DROP_RATE` (0,7, `config.h:82`) cai `money × money_adj`, arredondado.
//!
//! Cada monte cai a ±2 m do monstro no plano, no chão (`world_manager`, `GM_MSG_PRODUCE_*`,
//! `worldmanager.cpp:512-555`), e fica do dono por 30 s e no chão por 300 s
//! (`gmatter_item_base_imp(life = 300, belong_time = 30)`, `matter.h:62`).
//!
//! # Bolsa
//!
//! [`Bolsa`] reproduz o empilhamento de `item_list::Push` do servidor e de
//! `CECInventory::MergeItem` do cliente (`EC_Inventory.cpp:179-215`): primeiro completa as
//! pilhas do mesmo item, na ordem dos slots; o que sobrar vai inteiro para o primeiro slot
//! vazio. O cliente confere o último slot e a quantidade final que o servidor manda — se a
//! regra divergir, ele descarta o comando.

use pw_core::{ContainerType, ItemRecord, RoleId};
use pw_data_loader::{GameDataManager, TemplateDeMonstro};
use rand::Rng;

/// `MONEY_DROP_RATE` (`gs/config.h:82`).
pub const CHANCE_DE_DINHEIRO: f32 = 0.7;
/// `MONEY_MATTER_ID` (`gs/config.h:83`) — o `tid` do monte de moedas no chão.
pub const TID_DO_DINHEIRO: u32 = 3044;
/// `PICKUP_DISTANCE` (`gs/config.h:22`).
pub const DISTANCIA_PARA_PEGAR: f32 = 10.0;
/// Vida e posse de um item no chão, em segundos (`matter.h:62`).
pub const VIDA_NO_CHAO_S: u32 = 300;
pub const POSSE_S: u32 = 30;
/// `ITEM_LIST_BASE_SIZE` e `TASKITEM_LIST_SIZE` (`gs/config.h:12-15`).
pub const TAMANHO_DA_BOLSA: usize = 32;
pub const TAMANHO_DA_BOLSA_DE_MISSAO: usize = 32;

/// O que um monstro deixou cair.
#[derive(Debug, Clone, PartialEq)]
pub struct Queda {
    pub itens: Vec<u32>,
    pub montes_de_dinheiro: Vec<u32>,
}

/// `abase::RandSelect` (`template/itemdataman.h:33-47`): percorre subtraindo; se nada for
/// escolhido, fica com o índice 0.
fn sortear_indice(r: f32, probs: impl Iterator<Item = f32>) -> usize {
    let mut op = r;
    for (i, p) in probs.enumerate() {
        if op < p {
            return i;
        }
        op -= p;
    }
    0
}

/// Gera o drop de um monstro morto por um jogador de `nivel_do_dono`.
pub fn gerar_queda<R: Rng>(m: &TemplateDeMonstro, nivel_do_dono: i32, dados: &GameDataManager, rng: &mut R) -> Queda {
    let ajuste = dados.progressao.ajuste(nivel_do_dono - m.nivel);
    let mut q = Queda { itens: Vec::new(), montes_de_dinheiro: Vec::new() };
    if rng.gen::<f32>() <= ajuste.item {
        for rodada in 0..m.rodadas_de_drop.max(0) {
            let quantos = sortear_indice(rng.gen(), m.chance_de_quantos.iter().copied());
            for _ in 0..quantos {
                let idx = sortear_indice(rng.gen(), m.itens_de_drop.iter().map(|d| d.1));
                if rodada > 0 && idx >= 16 {
                    continue;
                }
                let id = m.itens_de_drop.get(idx).map(|d| d.0).unwrap_or(0);
                if id != 0 {
                    q.itens.push(id);
                }
            }
        }
    }
    let (baixo, alto) = (m.dinheiro_medio - m.dinheiro_variacao, m.dinheiro_medio + m.dinheiro_variacao);
    if alto > 0 {
        for _ in 0..m.rodadas_de_drop.max(0) {
            let valor = if baixo >= alto { baixo } else { rng.gen_range(baixo..=alto) };
            if valor > 0 && rng.gen::<f32>() < CHANCE_DE_DINHEIRO {
                let d = (valor as f32 * ajuste.dinheiro + 0.5) as i32;
                if d > 0 {
                    q.montes_de_dinheiro.push(d as u32);
                }
            }
        }
    }
    q
}

/// Uma bolsa carregada do banco, com o que mudou marcado para gravar.
#[derive(Debug, Clone)]
pub struct Bolsa {
    pub dono: RoleId,
    pub tipo: ContainerType,
    pub slots: Vec<Option<ItemRecord>>,
    alterados: std::collections::BTreeSet<usize>,
}

/// O resultado de empilhar: quanto entrou, o último slot tocado e quanto ficou nele.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Empilhado {
    pub entrou: u32,
    pub slot: usize,
    pub no_slot: u32,
}

impl Bolsa {
    pub fn nova(dono: RoleId, tipo: ContainerType, tamanho: usize, itens: Vec<ItemRecord>) -> Self {
        let mut slots = vec![None; tamanho];
        for i in itens {
            if (i.slot as usize) < tamanho {
                let s = i.slot as usize;
                slots[s] = Some(i);
            }
        }
        Self { dono, tipo, slots, alterados: Default::default() }
    }

    pub fn contar(&self, tid: u32) -> u32 {
        self.slots.iter().flatten().filter(|i| i.item_id == tid).map(|i| i.count).sum()
    }

    pub fn livres(&self) -> u32 {
        self.slots.iter().filter(|s| s.is_none()).count() as u32
    }

    /// `CECInventory::MergeItem` / `item_list::Push`. `None` quando não coube nada.
    pub fn empilhar(&mut self, tid: u32, mut quantidade: u32, dados: &GameDataManager) -> Option<Empilhado> {
        let pilha = dados.limite_de_pilha(tid);
        let total = quantidade;
        let mut primeiro_vazio = None;
        for s in 0..self.slots.len() {
            match &mut self.slots[s] {
                Some(i) if i.item_id == tid && i.count < pilha => {
                    let cabe = (pilha - i.count).min(quantidade);
                    i.count += cabe;
                    quantidade -= cabe;
                    self.alterados.insert(s);
                    if quantidade == 0 {
                        let no_slot = i.count;
                        return Some(Empilhado { entrou: total, slot: s, no_slot });
                    }
                }
                None if primeiro_vazio.is_none() => primeiro_vazio = Some(s),
                _ => {}
            }
        }
        let s = primeiro_vazio?;
        let n = quantidade.min(pilha);
        let durabilidade = dados.durabilidade_de_fabrica(tid).unwrap_or(0);
        self.slots[s] = Some(ItemRecord {
            id: None,
            character_id: self.dono,
            container_type: self.tipo,
            slot: s as u16,
            item_id: tid,
            count: n,
            max_count: pilha,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: durabilidade,
            max_durability: durabilidade,
            bind_status: 0,
            octets: vec![],
            custom_attributes: serde_json::json!({}),
        });
        self.alterados.insert(s);
        Some(Empilhado { entrou: total - quantidade + n, slot: s, no_slot: n })
    }

    /// Tira `quantidade` do item, slot a slot na ordem (`TakeAwayCommonItem`,
    /// `taskman.cpp:200-218`). Devolve `(slot, quanto saiu)` de cada slot tocado.
    pub fn tirar(&mut self, tid: u32, mut quantidade: u32) -> Vec<(usize, u32)> {
        let mut saiu = Vec::new();
        for s in 0..self.slots.len() {
            if quantidade == 0 {
                break;
            }
            let Some(i) = &mut self.slots[s] else { continue };
            if i.item_id != tid {
                continue;
            }
            let n = i.count.min(quantidade);
            i.count -= n;
            quantidade -= n;
            if i.count == 0 {
                self.slots[s] = None;
            }
            self.alterados.insert(s);
            saiu.push((s, n));
        }
        saiu
    }

    /// `item_list::DecAmount` num slot só. Devolve quanto saiu.
    pub fn tirar_do_slot(&mut self, slot: usize, n: u32) -> u32 {
        let Some(Some(i)) = self.slots.get_mut(slot) else { return 0 };
        let saiu = i.count.min(n);
        i.count -= saiu;
        if i.count == 0 {
            self.slots[slot] = None;
        }
        self.alterados.insert(slot);
        saiu
    }

    pub fn itens(&self) -> Vec<ItemRecord> {
        self.slots.iter().flatten().cloned().collect()
    }

    /// Grava no banco só os slots que mudaram.
    pub async fn gravar(&mut self, repo: &pw_storage::ItemRepository) -> Result<(), String> {
        for s in std::mem::take(&mut self.alterados) {
            let r = match &self.slots[s] {
                Some(i) => repo.upsert_item(i).await.map(|_| ()),
                None => repo.delete_item_by_slot(self.dono, self.tipo, s as u16).await.map(|_| ()),
            };
            r.map_err(|e| format!("slot {s} de {:?}: {e}", self.tipo))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(slot: u16, tid: u32, count: u32) -> ItemRecord {
        ItemRecord {
            id: None,
            character_id: 1,
            container_type: ContainerType::Inventory,
            slot,
            item_id: tid,
            count,
            max_count: 0,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 0,
            max_durability: 0,
            bind_status: 0,
            octets: vec![],
            custom_attributes: serde_json::json!({}),
        }
    }

    #[test]
    fn empilha_como_o_cliente() {
        let mut dados = GameDataManager::default();
        dados.pilhas.insert(10, 20);
        let mut b = Bolsa::nova(1, ContainerType::Inventory, 4, vec![item(1, 10, 15), item(3, 10, 18)]);
        // Completa o slot 1 (5), o slot 3 (2) e sobra 3 para o primeiro vazio, o 0.
        let e = b.empilhar(10, 10, &dados).unwrap();
        assert_eq!(e, Empilhado { entrou: 10, slot: 0, no_slot: 3 });
        assert_eq!(b.contar(10), 43);
        let e = b.empilhar(10, 1, &dados).unwrap();
        assert_eq!((e.slot, e.no_slot), (0, 4));
        assert_eq!(b.tirar(10, 21), vec![(0, 4), (1, 17)]);
        assert_eq!(b.contar(10), 23);
        assert_eq!(b.livres(), 2);
    }

    #[test]
    fn o_sorteio_de_indice_cai_no_primeiro_quando_nada_bate() {
        assert_eq!(sortear_indice(0.5, [0.2, 0.2].into_iter()), 0);
        assert_eq!(sortear_indice(0.3, [0.2, 0.2].into_iter()), 1);
    }
}
