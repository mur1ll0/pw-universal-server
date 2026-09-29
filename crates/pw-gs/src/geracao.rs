//! O equipamento gerado como o original (`generate_weapon/armor/decoration`,
//! `gs/template/generate_item_temp.h:156-480`, `740-830`), nas três variantes que o jogo usa
//! ([`Geracao`]): o drop de monstro, a fabricação e a venda na loja.
//!
//! Sai um [`pw_core::ConteudoDeEquipamento`] com a essência sorteada, os furos vazios e as
//! propriedades adicionais — gravado nos octetos do item, mandado cru ao cliente e lido de
//! volta para os atributos ([`crate::entity::Equipamento::dos_itens`]).

use pw_core::{AddonDoItem, ConteudoDeEquipamento, FichaDoEquipamento};
use pw_data_loader::addons::{Familia, ModeloDeGeracao, Sorteio};
use pw_data_loader::GameDataManager;
use rand::Rng;

/// `abase::RandNormal(int, int)`: média de dois uniformes (ver
/// [`crate::combat::sortear_dano_elemental`]).
pub(crate) fn rand_normal(a: i32, b: i32) -> i32 {
    crate::combat::sortear_dano_elemental(a.min(b), a.max(b))
}

/// `abase::RandSelect(option, stride, num)` (`itemdataman.h:33-47`).
fn rand_select(probs: &[f32]) -> usize {
    let mut op: f32 = rand::thread_rng().gen();
    for (i, p) in probs.iter().enumerate() {
        if op < *p {
            return i;
        }
        op -= p;
    }
    0
}

/// Os bits de um `float` guardados num `int` (parâmetro de porcentagem do `EQUIPMENT_ADDON`).
fn como_float(v: i32) -> f32 {
    f32::from_bits(v as u32)
}

/// `itemdataman::generate_addon` + `GenerateParam` do tratador. `None` para tratador sem
/// porte — o addon não é gerado.
fn gerar_addon(dados: &GameDataManager, id: u32) -> Option<AddonDoItem> {
    let a = dados.addons.por_id.get(&id)?;
    let p = a.params;
    let args = match a.sorteio()? {
        Sorteio::Ponto => vec![p[0]],
        Sorteio::EntreDois => vec![rand_normal(p[0], p[1])],
        Sorteio::Porcento => vec![(como_float(p[0]) * 100.0 + 0.1) as i32],
        Sorteio::EntreDoisPorcento => vec![rand_normal((como_float(p[0]) * 100.0 + 0.1) as i32, (como_float(p[1]) * 100.0 + 0.1) as i32)],
        Sorteio::Refino => vec![p[0], 0],
        Sorteio::DoisComoEstao => vec![p[0], p[1]],
    };
    Some(AddonDoItem::novo(id, args))
}

/// `generate_equipment_addon_buffer`: `n` sorteios na lista (id ≤ 0 não conta).
fn sortear_da_lista(dados: &GameDataManager, lista: &[(u32, f32)], n: usize) -> Vec<AddonDoItem> {
    let probs: Vec<f32> = lista.iter().map(|x| x.1).collect();
    (0..n)
        .filter_map(|_| {
            let id = lista.get(rand_select(&probs))?.0;
            (id > 0).then_some(id).and_then(|id| gerar_addon(dados, id))
        })
        .collect()
}

/// Qual das três chamadas do original gera o item.
///
/// | variante | quem chama | sorteio | lista de addons | `item_tag_t` |
/// | :--- | :--- | :--- | :--- | :--- |
/// | [`Geracao::Drop`] | `generate_item_for_drop` (monstro, mina, missão) | `NORMAL` | `ADDON_LIST_DROP` (`addons`) | `IMT_DROP` |
/// | [`Geracao::Producao`] | `generate_item_from_player` (`ProduceItem`, `itemdataman.cpp:1239-1246`) | `NORMAL(0)` | `ADDON_LIST_PRODUCE` (`rands`) | `IMT_PRODUCE` + nome |
/// | [`Geracao::Loja`] | `get_item_for_sell` (montado no carregamento, `itemdataman.cpp:1352-1379`) | `SPECIFIC(0)` | `ADDON_LIST_SHOP` (nenhuma) | `IMT_SHOP` |
///
/// O `SPECIFIC` não sorteia (`itemdataman.h:196-247`): `RandSelect` com tendência `LOWER` dá
/// o índice 0 (sem furo, sem addon, nenhuma resistência zerada), com `MIDDLE` o do meio, e
/// `RandNormal` com `LOWER` dá o mínimo da faixa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Geracao {
    Drop,
    /// Com o nome de quem fabricou (os bytes de [`pw_core::nome_do_fabricante`]).
    Producao { fabricante: Vec<u8> },
    Loja,
}

impl Geracao {
    /// `RandNormal(lower, upper, cls, LOWER_TREND)`.
    fn faixa(&self, a: i32, b: i32) -> i32 {
        match self {
            Geracao::Loja => a,
            _ => rand_normal(a, b),
        }
    }

    /// `RandSelect(..., LOWER_TREND)`.
    fn escolher(&self, probs: &[f32]) -> usize {
        match self {
            Geracao::Loja => 0,
            _ => rand_select(probs),
        }
    }
}

/// `generate_magic_defense` (`generate_item_temp.h:374-402`).
fn resistencias_de(faixas: &[(i32, i32); 5], fixo: bool, g: &Geracao) -> [i32; 5] {
    const QUANTOS_ZERADOS: [f32; 6] = [0.35, 0.25, 0.20, 0.15, 0.05, 0.051];
    const AJUSTE: [f32; 5] = [1.0, 1.1, 1.3, 1.6, 2.0];
    let mut res = [0; 5];
    let zerados = if fixo { 0 } else { g.escolher(&QUANTOS_ZERADOS) };
    if zerados == 5 {
        return res;
    }
    let mut ordem = [0usize, 1, 2, 3, 4];
    let mut rng = rand::thread_rng();
    for i in 0..zerados {
        let r = rng.gen_range(i..=4);
        ordem.swap(i, r);
    }
    for &idx in ordem.iter().take(5 - zerados) {
        res[idx] = (g.faixa(faixas[idx].0, faixas[idx].1) as f32 * AJUSTE[zerados]) as i32;
    }
    res
}

/// `addon_update_ess_data` → `ApplyAtGeneration` dos tratadores de essência.
fn aplicar_na_essencia(dados: &GameDataManager, ficha: &mut FichaDoEquipamento, a: &AddonDoItem) {
    let Some(t) = dados.addons.por_id.get(&a.id()).map(|x| x.tratador.as_str()) else { return };
    let v = a.args.first().copied().unwrap_or(0);
    let v2 = a.args.get(1).copied().unwrap_or(0);
    let escola = |t: &str| t.split('<').nth(1).and_then(|s| s.trim_end_matches('>').parse::<usize>().ok()).filter(|i| *i < 5);
    match ficha {
        FichaDoEquipamento::Arma(w) => match t {
            "enhance_weapon_damage_addon" => {
                w.dano_minimo += v;
                w.dano_maximo += v;
            }
            "enhance_weapon_max_damage_addon" => w.dano_maximo += v,
            "enhance_weapon_magic_addon" => {
                w.dano_magico_minimo += v;
                w.dano_magico_maximo += v;
            }
            "enhance_weapon_max_magic_addon" => w.dano_magico_maximo += v,
            _ => {}
        },
        FichaDoEquipamento::Armadura(r) => {
            // `armor_essence { defense; armor; mp_enhance; hp_enhance; resistance[5] }`.
            if let Some(campo) = t.strip_prefix("IA_EA_ESS<offsetof(armor_essence,") {
                match campo.trim_end_matches(")>") {
                    "defense" => r.defesa += v,
                    "armor" => r.evasao += v,
                    "mp_enhance" => r.mp_extra += v,
                    "hp_enhance" => r.hp_extra += v,
                    _ => {}
                }
            } else if t.starts_with("item_armor_enhance_resistance<") {
                if let Some(i) = escola(t) {
                    r.resistencias[i] += v;
                }
            }
        }
        FichaDoEquipamento::Decoracao(d) => {
            // `decoration_essence { damage; magic_damage; defense; armor; resistance[5] }`.
            if let Some(campo) = t.strip_prefix("IA_ED_ESS<offsetof(decoration_essence,") {
                match campo.trim_end_matches(")>") {
                    "damage" => d.dano += v,
                    "magic_damage" => d.dano_magico += v,
                    "defense" => d.defesa += v,
                    "armor" => d.evasao += v,
                    _ => {}
                }
            } else if t.starts_with("item_decoration_enchance_resistance<") {
                if let Some(i) = escola(t) {
                    d.resistencias[i] += v;
                }
            } else if t == "item_decoration_specific_damage_addon" {
                d.dano += v;
                d.defesa -= v2;
            } else if t == "item_decoration_specific_magic_damage_addon" {
                d.dano_magico += v;
                for x in &mut d.resistencias {
                    *x -= v2;
                }
            }
        }
        FichaDoEquipamento::Municao(_) => {}
    }
}

/// Um equipamento de drop. `None` quando o item não é arma/armadura/acessório com modelo, ou
/// é um dos subtipos que o original não gera (`generate_item_temp.h:208-211`).
pub fn gerar_equipamento(dados: &GameDataManager, tid: u32) -> Option<ConteudoDeEquipamento> {
    gerar_equipamento_de(dados, tid, Geracao::Drop)
}

/// Um equipamento na variante `g` (ver [`Geracao`]).
pub fn gerar_equipamento_de(dados: &GameDataManager, tid: u32, g: Geracao) -> Option<ConteudoDeEquipamento> {
    let m: &ModeloDeGeracao = dados.geracao.get(&tid)?;
    let mut ficha = dados.equipamentos.ficha(tid)?;
    if m.familia == Familia::Arma && [300, 293, 76, 291].contains(&m.id_sub_type) {
        return None;
    }
    // Furos (`generate_item_temp.h:215-222`): a tabela do drop no drop, a da fabricação nos
    // outros dois (na loja, índice 0 — nenhum).
    let tabela_de_furos = if g == Geracao::Drop { &m.furos_no_drop } else { &m.furos_na_producao };
    let furos = if tabela_de_furos.is_empty() { 0 } else { g.escolher(tabela_de_furos) };
    let mut quantos = g.escolher(&m.quantos_addons);
    let mut addons = Vec::new();
    if m.fixed_props {
        // `generate_equipment_addon_buffer_2`: todos os da lista, se sorteou algum.
        if quantos > 0 {
            addons = m.addons.iter().filter(|x| x.0 > 0).filter_map(|x| gerar_addon(dados, x.0)).collect();
        }
    } else if quantos > 0 {
        // `generate_template_addon` (`:156-190`): o único só existe na arma; a lista é a do
        // drop (`addons`) ou a da fabricação (`rands`). A loja (`ADDON_LIST_SHOP`) cai no
        // `else` que zera tudo — e o `SPECIFIC` já sorteia zero addons.
        let lista = match &g {
            Geracao::Drop => &m.addons,
            Geracao::Producao { .. } => &m.addons_da_producao,
            Geracao::Loja => &Vec::new(),
        };
        if !lista.is_empty() {
            if !m.unicos.is_empty() && rand::thread_rng().gen::<f32>() < m.chance_de_unico {
                addons.extend(sortear_da_lista(dados, &m.unicos, 1));
                quantos -= 1;
            }
            addons.extend(sortear_da_lista(dados, lista, quantos));
        }
    }

    // Durabilidade (`:292-310`): a máxima é `RandNormal(durability_min, durability_max)`; só o
    // drop (sem o bit 0x1000 do `proc_type`) sai gasto, com `min(RandNormal(drop), máxima)` —
    // a fabricação e a loja saem **cheias** (a foice 15964 fabricada saía 245/300, B151). Os
    // números saem do `elements.data` na escala do arquivo e o original os multiplica pela
    // escala interna no fim da geração (`update_require_data`, `gs/item/item_addon.h:454-458`,
    // chamado em `generate_item_temp.h:367`) — sem isso o cliente divide por 100 e mostra
    // **1/1** em qualquer equipamento gerado (relato do arco, 2026-09-18).
    let escala = pw_core::ESCALA_DA_DURABILIDADE;
    let dur_max = g.faixa(m.durabilidade.0, m.durabilidade.1) * escala;
    let dur = if g != Geracao::Drop || m.proc_type & 0x1000 != 0 {
        dur_max
    } else {
        (rand_normal(m.durabilidade_no_drop.0, m.durabilidade_no_drop.1) * escala).min(dur_max)
    };

    match &mut ficha {
        FichaDoEquipamento::Arma(w) => {
            w.dano_maximo = g.faixa(m.dano_maximo.0, m.dano_maximo.1);
            w.dano_magico_maximo = g.faixa(m.dano_magico_maximo.0, m.dano_magico_maximo.1);
        }
        FichaDoEquipamento::Armadura(r) => {
            r.defesa = g.faixa(m.defesa.0, m.defesa.1);
            r.evasao = g.faixa(m.evasao.0, m.evasao.1);
            r.mp_extra = g.faixa(m.mana.0, m.mana.1);
            r.hp_extra = g.faixa(m.vida.0, m.vida.1);
            r.resistencias = resistencias_de(&m.resistencias, m.todas_as_resistencias || m.fixed_props, &g);
        }
        FichaDoEquipamento::Decoracao(d) => {
            d.dano = g.faixa(m.dano.0, m.dano.1);
            d.dano_magico = g.faixa(m.dano_magico.0, m.dano_magico.1);
            d.defesa = g.faixa(m.defesa.0, m.defesa.1);
            d.evasao = g.faixa(m.evasao.0, m.evasao.1);
            d.resistencias = resistencias_de(&m.resistencias, m.fixed_props, &g);
        }
        FichaDoEquipamento::Municao(_) => return None,
    }
    for a in &addons {
        aplicar_na_essencia(dados, &mut ficha, a);
    }
    let mut c = ConteudoDeEquipamento::novo(ficha, dur, dur_max);
    c.furos = vec![0; furos];
    c.addons = addons;
    (c.origem, c.fabricante) = match g {
        Geracao::Drop => (pw_core::OrigemDoItem::Drop, Vec::new()),
        Geracao::Loja => (pw_core::OrigemDoItem::Loja, Vec::new()),
        Geracao::Producao { fabricante } => (pw_core::OrigemDoItem::Producao, fabricante),
    };
    Some(c)
}
