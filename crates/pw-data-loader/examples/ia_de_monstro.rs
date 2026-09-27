//! Quantos monstros usam cada camada da IA de combate do original: a estratégia
//! (`id_strategy` → `ai_policy::AddPrimaryTask`, `aipolicy.h:1214-1277`), os eventos de vida
//! (`skill_hp75/50/25` → `TriggerEvent`, `aipolicy.h:1343-1370`) e as condições/operações do
//! `aipolicy.data` usadas por monstros de verdade (`common_strategy` → `SetAITrigger`).
//!
//! `cargo run -p pw-data-loader --example ia_de_monstro -- data/realm_155/config`
use pw_data_loader::aipolicy::{AiPolicyData, NoDeCondicao};
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::monstros;
use std::collections::BTreeMap;

fn condicoes(n: &NoDeCondicao, c: &mut BTreeMap<String, usize>) {
    *c.entry(format!("{:?}", n.tipo.map(|t| format!("{t:?}")).unwrap_or(n.tipo_cru.to_string()))).or_default() += 1;
    if let Some(e) = &n.esquerda {
        condicoes(e, c);
    }
    if let Some(d) = &n.direita {
        condicoes(d, c);
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("uso: ia_de_monstro <pasta config>");
    let elements = load_elements_data_auto(&std::fs::read(format!("{dir}/elements.data")).unwrap()).unwrap();
    let politicas = AiPolicyData::load_from_bytes(&std::fs::read(format!("{dir}/aipolicy.data")).unwrap()).unwrap();
    let t = monstros::carregar(&elements, Some(&politicas));

    let mut por_estrategia: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
    for m in t.templates.values() {
        let e = por_estrategia.entry(m.estrategia).or_default();
        e.0 += 1;
        if !m.skills.is_empty() {
            e.1 += 1;
        }
    }
    println!("monstros: {}", t.templates.len());
    for (e, (n, s)) in &por_estrategia {
        println!("  estratégia {e}: {n} monstros, {s} com habilidade");
    }
    let ev = |f: fn(&monstros::TemplateDeMonstro) -> bool| t.templates.values().filter(|m| f(m)).count();
    println!(
        "eventos de vida: 75% {}, 50% {}, 25% {}",
        ev(|m| m.skills_com_75_de_vida.iter().any(|s| s.id > 0)),
        ev(|m| m.skills_com_50_de_vida.iter().any(|s| s.id > 0)),
        ev(|m| m.skills_com_25_de_vida.iter().any(|s| s.id > 0)),
    );
    let fuga = ev(|m| {
        m.skills_com_75_de_vida.iter().chain(&m.skills_com_50_de_vida).chain(&m.skills_com_25_de_vida).any(|s| s.id == 40)
    });
    println!("com evento de fuga (FLEE_SKILL_ID 40, `config.h:86`): {fuga}");

    let mut cond = BTreeMap::new();
    let mut ops: BTreeMap<String, usize> = BTreeMap::new();
    let mut com_politica = 0;
    for m in t.templates.values().filter(|m| m.politica_de_ia != 0) {
        let Some(p) = politicas.get_policy(m.politica_de_ia) else { continue };
        com_politica += 1;
        for tr in &p.triggers {
            if let Some(c) = &tr.condicao {
                condicoes(c, &mut cond);
            }
            for o in &tr.operacoes {
                *ops.entry(o.tipo.map(|t| format!("{t:?}")).unwrap_or(o.tipo_cru.to_string())).or_default() += 1;
            }
        }
    }
    println!("monstros com política: {com_politica}");
    let mut ids: std::collections::BTreeSet<(i32, i32)> = t.templates.values().flat_map(|m| m.skills.iter().map(|s| (s.id, s.nivel))).collect();
    ids.extend(t.templates.values().flat_map(|m| m.skills_com_75_de_vida.iter().chain(&m.skills_com_50_de_vida).chain(&m.skills_com_25_de_vida).map(|s| (s.id, s.nivel))).filter(|i| i.0 > 0));
    let lista: Vec<String> = ids.iter().map(|(i, n)| format!("{i} {n}")).collect();
    std::fs::write(std::env::var("SAIDA_IDS").unwrap_or("ids_de_monstro.txt".into()), lista.join("
")).unwrap();
    println!("pares (habilidade, nível) de monstro: {}", ids.len());
    println!("condições (por monstro): {cond:?}");
    println!("operações (por monstro): {ops:?}");
}
