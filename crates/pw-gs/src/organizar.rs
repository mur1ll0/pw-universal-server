//! B320: organizar a bolsa — porte do botão "organizar" do cliente 1.5.5.
//!
//! O cliente ordena sozinho e manda ao servidor só a lista de trocas (`MULTI_EXCHANGE_ITEM`): `CECHostPlayer::SortPack`
//! (`EC_HostPlayer.cpp:1775-1868`) com o `DefaultPackSorter` (`:1700-1770`) e o `BubbleSort` de `EC_Algorithm.h:39-53`.
//! O servidor do painel (botão "Organizar", 1.2.6 e 1.5.5) usa a mesma conta, para dar o mesmo resultado que o jogo.
//!
//! - Ordem: vazio por último; classe pela ordem de `GetClassIDOrder` (maior antes), empate de ordem pelo número da
//!   classe; na mesma classe, arma e armadura por nível decrescente, carta de general por rank decrescente e tipo
//!   crescente (e só isso), e por fim o id do modelo.
//! - As trocas: a permutação ordenada vira pares `(i, j)` pelo laço de ciclos do `SortPack`, com par só quando um dos
//!   dois slots originais tem item; depois a lista inteira é invertida (`:1854-1858`), o que também troca a ordem de
//!   cada par.

use pw_data_loader::ordem_do_inventario::{ordem_da_classe, CLASSE_ARMA, CLASSE_ARMADURA, CLASSE_CARTA};

/// O que o comparador vê de um item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemParaOrdenar {
    pub classe: i32,
    /// `level` do `WEAPON_ESSENCE`/`ARMOR_ESSENCE` (0 nas outras classes).
    pub nivel: i32,
    pub tid: u32,
    /// Carta de general: `(type, rank)` dos octetos da instância (`IVTR_ESSENCE_GENERALCARD`, bytes 0 e 4).
    pub carta: (i32, i32),
}

/// `DefaultPackSorter::operator()(index1, index2)`: `a` vai antes de `b`?
pub fn antes(a: Option<&ItemParaOrdenar>, b: Option<&ItemParaOrdenar>) -> bool {
    let Some(a) = a else { return false };
    let Some(b) = b else { return true };
    if a.classe != b.classe {
        let (oa, ob) = (ordem_da_classe(a.classe), ordem_da_classe(b.classe));
        if oa != ob {
            return oa > ob;
        }
        return a.classe < b.classe;
    }
    if a.classe == CLASSE_ARMA || a.classe == CLASSE_ARMADURA {
        if a.nivel != b.nivel {
            return a.nivel > b.nivel;
        }
    } else if a.classe == CLASSE_CARTA {
        let ((ta, ra), (tb, rb)) = (a.carta, b.carta);
        return if ra > rb { true } else if ra == rb { ta < tb } else { false };
    }
    a.tid < b.tid
}

/// As trocas que o `SortPack` manda para a bolsa `slots` (um por slot, `None` = vazio). Vazia se nada muda.
pub fn trocas(slots: &[Option<ItemParaOrdenar>]) -> Vec<(u8, u8)> {
    let n = slots.len();
    if n == 0 || slots.iter().all(Option::is_none) {
        return Vec::new();
    }
    // `BubbleSort(vec.begin(), vec.end(), pred)`: troca quando `pred(*próximo, *atual)`; o fim encolhe a cada passada.
    let mut v: Vec<usize> = (0..n).collect();
    let mut fim = n;
    while fim > 0 {
        let mut atual = 0;
        let mut proximo = 1;
        while proximo < fim {
            if antes(slots[v[proximo]].as_ref(), slots[v[atual]].as_ref()) {
                v.swap(atual, proximo);
            }
            atual = proximo;
            proximo += 1;
        }
        fim = atual;
    }
    // O laço de ciclos (`:1834-1850`).
    let mut lista: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < n {
        let j = v[i];
        if j == i {
            i += 1;
            continue;
        }
        let k = v[j];
        if slots[j].is_some() || slots[k].is_some() {
            lista.push(i);
            lista.push(j);
        }
        v.swap(i, j);
    }
    lista.reverse();
    lista.chunks(2).map(|p| (p[0] as u8, p[1] as u8)).collect()
}

/// Aplica as trocas numa cópia (para conferir e para testar).
pub fn aplicar<T: Clone>(slots: &[T], pares: &[(u8, u8)]) -> Vec<T> {
    let mut s = slots.to_vec();
    for &(a, b) in pares {
        s.swap(a as usize, b as usize);
    }
    s
}

#[cfg(test)]
mod testes {
    use super::*;

    fn item(classe: i32, nivel: i32, tid: u32) -> Option<ItemParaOrdenar> {
        Some(ItemParaOrdenar { classe, nivel, tid, carta: (0, 0) })
    }

    #[test]
    fn junta_sem_buracos_e_ordena_como_o_cliente() {
        // Remédio (9), arma nível 5, vazio, arma nível 9, armadura (0), material (8, fora da lista: vai por último).
        let bolsa = vec![item(9, 0, 300), item(15, 5, 20), None, item(15, 9, 10), item(0, 3, 40), item(8, 0, 50), None];
        let pares = trocas(&bolsa);
        let depois = aplicar(&bolsa, &pares);
        let tids: Vec<Option<u32>> = depois.iter().map(|x| x.map(|i| i.tid)).collect();
        // Arma (ordem 23) por nível decrescente, armadura (20), remédio (10), material (0); vazios no fim.
        assert_eq!(tids, vec![Some(10), Some(20), Some(40), Some(300), Some(50), None, None]);
        assert!(pares.iter().all(|&(a, b)| a != b));
    }

    #[test]
    fn mesma_classe_pelo_id_e_bolsa_arrumada_nao_troca() {
        let bolsa = vec![item(9, 0, 3), item(9, 0, 1), item(9, 0, 2)];
        assert_eq!(aplicar(&bolsa, &trocas(&bolsa)).iter().map(|x| x.unwrap().tid).collect::<Vec<_>>(), vec![1, 2, 3]);
        let arrumada = vec![item(9, 0, 1), item(9, 0, 2), None];
        assert!(trocas(&arrumada).is_empty());
        assert!(trocas(&[None, None]).is_empty());
    }

    #[test]
    fn carta_por_rank_e_tipo_sem_desempate_pelo_id() {
        let c = |tipo, rank, tid| Some(ItemParaOrdenar { classe: CLASSE_CARTA, nivel: 0, tid, carta: (tipo, rank) });
        let bolsa = vec![c(2, 1, 1), c(1, 3, 2), c(0, 1, 3)];
        let tids: Vec<u32> = aplicar(&bolsa, &trocas(&bolsa)).iter().map(|x| x.unwrap().tid).collect();
        assert_eq!(tids, vec![2, 3, 1], "rank 3 antes; no rank 1, tipo 0 antes do 2");
    }

    #[test]
    fn as_trocas_saem_invertidas_como_no_cliente() {
        // Dois itens trocados de lugar: um par só, e o par vem invertido (`lista.reverse()`).
        let bolsa = vec![item(9, 0, 2), item(9, 0, 1)];
        assert_eq!(trocas(&bolsa), vec![(1, 0)]);
    }
}
