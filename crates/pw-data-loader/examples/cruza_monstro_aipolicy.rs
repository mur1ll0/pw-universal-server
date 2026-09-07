//! Cruza `MONSTER_ESSENCE.common_strategy` do `elements.data` com os ids do
//! `aipolicy.data` — é o vínculo que `npcgenerator.cpp:218` faz no servidor original
//! (`nt.trigger_policy = mob.common_strategy`).
//!
//! `cargo run -p pw-data-loader --example cruza_monstro_aipolicy -- data/realm_155/config`
use pw_data_loader::aipolicy::AiPolicyData;
use pw_data_loader::generic_elements::load_elements_data_auto;

fn main() {
    let dir = std::env::args().nth(1).expect("uso: cruza_monstro_aipolicy <pasta config>");
    let elements = std::fs::read(format!("{dir}/elements.data")).expect("elements.data");
    let policies = std::fs::read(format!("{dir}/aipolicy.data")).expect("aipolicy.data");

    let elements = load_elements_data_auto(&elements).expect("elements.data ilegível");
    let policies = AiPolicyData::load_from_bytes(&policies).expect("aipolicy.data ilegível");

    let monstros = elements.get("MONSTER_ESSENCE");
    let mut com_politica = 0usize;
    let mut resolvidos = 0usize;
    let mut orfaos: Vec<(i32, i32)> = Vec::new();

    for m in monstros {
        let id = m.get("ID").and_then(|v| v.as_i32()).unwrap_or(0);
        let estrategia = m.get("common_strategy").and_then(|v| v.as_i32()).unwrap_or(0);
        if estrategia == 0 {
            continue;
        }
        com_politica += 1;
        if policies.get_policy(estrategia as u32).is_some() {
            resolvidos += 1;
        } else if orfaos.len() < 20 {
            orfaos.push((id, estrategia));
        }
    }

    println!("monstros em MONSTER_ESSENCE: {}", monstros.len());
    println!("com common_strategy != 0:   {com_politica}");
    println!("com política existente:     {resolvidos}");
    println!("órfãos (amostra):           {orfaos:?}");
}
