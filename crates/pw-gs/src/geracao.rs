//! O equipamento que cai de monstro, sorteado como o original
//! (`generate_weapon/armor/decoration` com `ADDON_LIST_DROP`,
//! `gs/template/generate_item_temp.h:156-480`, `740-830`).
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
fn rand_normal(a: i32, b: i32) -> i32 {
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

/// `generate_magic_defense` (`generate_item_temp.h:360-392`).
fn resistencias(faixas: &[(i32, i32); 5], fixo: bool) -> [i32; 5] {
    const QUANTOS_ZERADOS: [f32; 6] = [0.35, 0.25, 0.20, 0.15, 0.05, 0.051];
    const AJUSTE: [f32; 5] = [1.0, 1.1, 1.3, 1.6, 2.0];
    let mut res = [0; 5];
    let zerados = if fixo { 0 } else { rand_select(&QUANTOS_ZERADOS) };
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
        res[idx] = (rand_normal(faixas[idx].0, faixas[idx].1) as f32 * AJUSTE[zerados]) as i32;
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
    let m: &ModeloDeGeracao = dados.geracao.get(&tid)?;
    let mut ficha = dados.equipamentos.ficha(tid)?;
    if m.familia == Familia::Arma && [300, 293, 76, 291].contains(&m.id_sub_type) {
        return None;
    }
    let furos = if m.furos_no_drop.is_empty() { 0 } else { rand_select(&m.furos_no_drop) };
    let mut quantos = rand_select(&m.quantos_addons);
    let mut addons = Vec::new();
    if m.fixed_props {
        // `generate_equipment_addon_buffer_2`: todos os da lista, se sorteou algum.
        if quantos > 0 {
            addons = m.addons.iter().filter(|x| x.0 > 0).filter_map(|x| gerar_addon(dados, x.0)).collect();
        }
    } else if quantos > 0 {
        // `generate_template_addon` (`:156-170`): o único só existe na arma.
        if !m.unicos.is_empty() && rand::thread_rng().gen::<f32>() < m.chance_de_unico {
            addons.extend(sortear_da_lista(dados, &m.unicos, 1));
            quantos -= 1;
        }
        addons.extend(sortear_da_lista(dados, &m.addons, quantos));
    }

    // Durabilidade (`:292-310`): no drop, `min(RandNormal(drop), máxima)`. Os dois números
    // saem do `elements.data` na escala do arquivo e o original os multiplica pela escala
    // interna no fim da geração (`update_require_data`, `gs/item/item_addon.h:454-458`,
    // chamado em `generate_item_temp.h:367`) — sem isso o cliente divide por 100 e mostra
    // **1/1** em qualquer equipamento gerado (relato do arco, 2026-09-18).
    let escala = pw_core::ESCALA_DA_DURABILIDADE;
    let dur_max = rand_normal(m.durabilidade.0, m.durabilidade.1) * escala;
    let dur = if m.proc_type & 0x1000 != 0 { dur_max } else { (rand_normal(m.durabilidade_no_drop.0, m.durabilidade_no_drop.1) * escala).min(dur_max) };

    match &mut ficha {
        FichaDoEquipamento::Arma(w) => {
            w.dano_maximo = rand_normal(m.dano_maximo.0, m.dano_maximo.1);
            w.dano_magico_maximo = rand_normal(m.dano_magico_maximo.0, m.dano_magico_maximo.1);
        }
        FichaDoEquipamento::Armadura(r) => {
            r.defesa = rand_normal(m.defesa.0, m.defesa.1);
            r.evasao = rand_normal(m.evasao.0, m.evasao.1);
            r.mp_extra = rand_normal(m.mana.0, m.mana.1);
            r.hp_extra = rand_normal(m.vida.0, m.vida.1);
            r.resistencias = resistencias(&m.resistencias, m.todas_as_resistencias || m.fixed_props);
        }
        FichaDoEquipamento::Decoracao(d) => {
            d.dano = rand_normal(m.dano.0, m.dano.1);
            d.dano_magico = rand_normal(m.dano_magico.0, m.dano_magico.1);
            d.defesa = rand_normal(m.defesa.0, m.defesa.1);
            d.evasao = rand_normal(m.evasao.0, m.evasao.1);
            d.resistencias = resistencias(&m.resistencias, m.fixed_props);
        }
        FichaDoEquipamento::Municao(_) => return None,
    }
    for a in &addons {
        aplicar_na_essencia(dados, &mut ficha, a);
    }
    let mut c = ConteudoDeEquipamento::novo(ficha, dur, dur_max);
    c.furos = vec![0; furos];
    c.addons = addons;
    Some(c)
}
