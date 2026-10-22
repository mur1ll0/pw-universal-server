//! B346 — as Cartas de General vestidas (slots 32–37, `EQUIP_INDEX_GENERALCARD1..6`, `gs/item.h:228-233`).
//!
//! - **Ativa** só a carta que passa no `generalcard_item::VerifyRequirement` (`gs/item/item_generalcard.cpp:68-74`):
//!   nível ≥ `require_level` e liderança livre ≥ `require_leadership`; ativando, ela ocupa a liderança
//!   (`OccupyLeadership`). As cartas são conferidas na ordem dos slots, como o `Activate` do equipamento.
//! - **Soma** o `GeneralCardEnhance` (`gs/actobject.h:1331-1345`): vida, dano, dano mágico, defesa, resistências e
//!   vigor, pelo [`pw_data_loader::cartas_de_general::CartaDeGeneral::reforco`].
//! - **Devorar** (C2S 170 `SWALLOW_GENERALCARD`): `gplayer_imp::PlayerSwallowGeneralCard` (`gs/player.cpp:24684-24745`).
//!
//! **Não é igual:** o nível é o atual, não o histórico (`GetHistoricalMaxLevel`); conjuntos de cartas, efeitos extras
//! (`set_addon_manager`) e o `IsSecActive` não estão portados; devorar só da bolsa (o armazém de cartas,
//! `IL_TRASH_BOX4`, não existe aqui).

use crate::entity::Equipamento;
use pw_data_loader::cartas_de_general::EssenciaDaCarta;
use pw_data_loader::GameDataManager;

/// `EQUIP_INDEX_GENERALCARD1`.
pub const PRIMEIRO_SLOT: u16 = 32;
pub const ULTIMO_SLOT: u16 = 37;

/// Uma carta vestida e ativa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartaAtiva {
    pub slot: u16,
    pub item_id: u32,
    pub essencia: EssenciaDaCarta,
}

/// As cartas vestidas que ativam, na ordem dos slots, e a liderança que ocupam.
pub fn cartas_ativas(itens: &[pw_core::ItemRecord], dados: &GameDataManager, nivel: i32, lideranca: i32) -> (Vec<CartaAtiva>, i32) {
    let mut vestidas: Vec<&pw_core::ItemRecord> =
        itens.iter().filter(|i| (PRIMEIRO_SLOT..=ULTIMO_SLOT).contains(&i.slot) && i.count > 0).collect();
    vestidas.sort_by_key(|i| i.slot);
    let mut ocupada = 0i32;
    let mut ativas = Vec::new();
    for i in vestidas {
        let Some(essencia) = EssenciaDaCarta::ler(&i.octets) else { continue };
        if !dados.cartas_de_general.cartas.contains_key(&i.item_id) {
            continue;
        }
        if nivel < essencia.nivel_exigido || lideranca - ocupada < essencia.lideranca_exigida {
            continue;
        }
        ocupada += essencia.lideranca_exigida;
        ativas.push(CartaAtiva { slot: i.slot, item_id: i.item_id, essencia });
    }
    (ativas, ocupada)
}

/// Soma as cartas ativas ao equipamento (`_en_point.max_hp/damage/magic_dmg/defense/resistance`, `_vigour_en`).
pub fn somar_cartas(e: &mut Equipamento, ativas: &[CartaAtiva], dados: &GameDataManager, classe: i32) {
    let c = &dados.cartas_de_general;
    for a in ativas {
        let Some(carta) = c.cartas.get(&a.item_id) else { continue };
        let coluna = (a.slot - PRIMEIRO_SLOT) as usize;
        let r = carta.reforco(&a.essencia, coluna, c.ajuste(classe, coluna));
        e.vida += r.vida;
        // O `Equipamento` guarda um dano só para o baixo e o alto (o dos acessórios); na carta os dois são iguais.
        e.dano += r.dano_baixo;
        e.dano_magico += r.magico_baixo;
        e.defesa += r.defesa;
        for k in 0..5 {
            e.resistencias[k] += r.resistencias[k];
        }
        e.addons.vigor += r.vigor;
    }
}

/// O `VerifyRequirement` de uma carta que vai ao `slot`: o conjunto como ficaria (a carta que estava lá sai), recontado
/// na ordem dos slots — a carta nova tem de ativar.
pub fn pode_vestir(itens: &[pw_core::ItemRecord], dados: &GameDataManager, octetos: &[u8], slot: u16, nivel: i32, lideranca: i32) -> bool {
    let Some(e) = EssenciaDaCarta::ler(octetos) else { return false };
    let Some(tid) = dados.cartas_de_general.cartas.iter().find(|(_, c)| c.tipo == e.tipo).map(|(id, _)| *id) else {
        return false;
    };
    let mut conjunto: Vec<pw_core::ItemRecord> = itens.iter().filter(|i| i.slot != slot).cloned().collect();
    // Só a essência conta para a ativação; o id serve para a carta existir no catálogo.
    let mut nova = itens.first().cloned().unwrap_or_else(|| pw_core::ItemRecord {
        id: None,
        character_id: 0,
        container_type: pw_core::ContainerType::Equipment,
        slot,
        item_id: tid,
        count: 1,
        max_count: 1,
        refine_level: 0,
        sockets_count: 0,
        sockets: vec![],
        durability: 0,
        max_durability: 0,
        bind_status: 0,
        octets: vec![],
        custom_attributes: serde_json::Value::Null,
    });
    nova.slot = slot;
    nova.item_id = tid;
    nova.count = 1;
    nova.octets = octetos.to_vec();
    conjunto.push(nova);
    let (ativas, _) = cartas_ativas(&conjunto, dados, nivel, lideranca);
    ativas.iter().any(|a| a.slot == slot)
}

/// `ERR_ITEM_NOT_IN_INVENTORY` e `ERR_GENERALCARD_INSERT_EXP` (`common/protocol.h`, contados do `ERR_SUCCESS`).
pub const ERRO_FORA_DA_BOLSA: i32 = 5;
pub const ERRO_EXP_DA_CARTA: i32 = 182;
/// `GENERALCARD_RANK_S`: carta S ou acima não é devorada (`player.cpp:24727-24733`).
const QUALIDADE_S: i32 = 3;

/// `PlayerSwallowGeneralCard` sem a parte de itens: dado o que vai ser devorado (`tid`, quantidade, os octetos se for
/// carta), a experiência que entra na carta vestida, ou o erro.
pub fn exp_a_devorar(dados: &GameDataManager, vestida: &EssenciaDaCarta, tid: u32, octetos: &[u8], quantidade: u32) -> Result<i32, i32> {
    let c = &dados.cartas_de_general;
    let (mut exp, devorada) = if let Some(carta) = c.cartas.get(&tid) {
        (carta.exp_ao_devorar, EssenciaDaCarta::ler(octetos))
    } else if let Some(e) = c.exp_da_caixa.get(&tid) {
        (*e, None)
    } else {
        return Err(ERRO_FORA_DA_BOLSA);
    };
    if let Some(d) = &devorada {
        exp = exp.wrapping_add(c.exp_acumulada(d));
    }
    if exp <= 0 {
        return Err(ERRO_EXP_DA_CARTA);
    }
    if exp as f32 * quantidade as f32 > 2e9 {
        return Err(ERRO_EXP_DA_CARTA);
    }
    let exp = exp * quantidade as i32;
    if let Some(d) = &devorada {
        if d.qualidade > vestida.qualidade {
            return Err(ERRO_EXP_DA_CARTA);
        }
    }
    let mut teste = *vestida;
    if !c.inserir_exp(&mut teste, exp, true) {
        return Err(ERRO_EXP_DA_CARTA);
    }
    if devorada.is_some_and(|d| d.qualidade >= QUALIDADE_S) {
        return Err(ERRO_EXP_DA_CARTA);
    }
    Ok(exp)
}
