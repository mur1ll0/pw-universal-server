//! Confere a tabela de monstros montada do `MONSTER_ESSENCE` contra o arquivo real.
//!
//! `cargo run -p pw-data-loader --example dump_monstros -- data/realm_155/config`
use pw_data_loader::aipolicy::AiPolicyData;
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::monstros::{self, TemplateDeMonstro};

fn main() {
    let dir = std::env::args().nth(1).expect("uso: dump_monstros <pasta config>");
    let elements = load_elements_data_auto(&std::fs::read(format!("{dir}/elements.data")).unwrap()).unwrap();
    let politicas = AiPolicyData::load_from_bytes(&std::fs::read(format!("{dir}/aipolicy.data")).unwrap()).unwrap();

    let t = monstros::carregar(&elements, Some(&politicas));
    println!("templates: {}", t.len());
    println!("recusas:   {:?} (total {})", t.recusas, t.recusas.total());
    println!("órfãos:    {:?}", t.politicas_orfas);

    let com_politica = t.templates.values().filter(|m| m.politica_de_ia != 0).count();
    let com_skill = t.templates.values().filter(|m| !m.skills.is_empty()).count();
    let com_hp75 = t.templates.values().filter(|m| !m.skills_com_75_de_vida.is_empty()).count();
    println!("com política de IA: {com_politica} | com skills: {com_skill} | com skill de 75% vida: {com_hp75}");

    let mut niveis: Vec<i32> = t.templates.values().map(|m| m.nivel).collect();
    niveis.sort_unstable();
    println!("níveis: min={} mediana={} max={}", niveis[0], niveis[niveis.len()/2], niveis[niveis.len()-1]);

    let mut vidas: Vec<i32> = t.templates.values().map(|m| m.vida).collect();
    vidas.sort_unstable();
    println!("vida:   min={} mediana={} max={}", vidas[0], vidas[vidas.len()/2], vidas[vidas.len()-1]);

    // Sanidade: campos que, se o layout estivesse deslocado, sairiam absurdos em massa.
    let dano_invertido = t.templates.values().filter(|m| m.dano_fisico.maximo < m.dano_fisico.minimo).count();
    let vida_zero = t.templates.values().filter(|m| m.vida <= 0).count();
    let nivel_zero = t.templates.values().filter(|m| m.nivel <= 0).count();
    let nivel_alto = t.templates.values().filter(|m| m.nivel > 150).count();
    println!("dano max<min: {dano_invertido} | vida<=0: {vida_zero} | nivel<=0: {nivel_zero} | nivel>150: {nivel_alto}");
    let mut hist = std::collections::BTreeMap::new();
    for m in t.templates.values() { *hist.entry(m.nivel / 10 * 10).or_insert(0usize) += 1; }
    println!("niveis por dezena: {hist:?}");

    let sem_nome: Vec<&TemplateDeMonstro> = t.templates.values().filter(|m| m.nome.is_empty()).collect();
    println!("sem nome: {} de {}", sem_nome.len(), t.len());
    let mut ids: Vec<u32> = sem_nome.iter().map(|m| m.id).collect();
    ids.sort_unstable();
    println!("  faixa de ids sem nome: {:?} .. {:?}", &ids[..ids.len().min(8)], &ids[ids.len().saturating_sub(4)..]);
    let sem_nome_com_ia = sem_nome.iter().filter(|m| m.politica_de_ia != 0).count();
    let sem_nome_dano_baixo = sem_nome.iter().filter(|m| m.dano_fisico.maximo <= 5).count();
    let com_nome_com_ia = t.templates.values().filter(|m| !m.nome.is_empty() && m.politica_de_ia != 0).count();
    println!("  dos sem nome: {} com IA, {} com dano <= 5", sem_nome_com_ia, sem_nome_dano_baixo);
    println!("  dos com nome: {} com IA de {}", com_nome_com_ia, t.len() - sem_nome.len());
    for m in sem_nome.iter().take(5) {
        println!("  id {:6} nv {:3} vida {:9} ataque {:6} dano {}..{} IA {} faccao {}", m.id, m.nivel, m.vida, m.taxa_de_ataque, m.dano_fisico.minimo, m.dano_fisico.maximo, m.politica_de_ia, m.faccao);
    }

    println!("\nalguns exemplos:");
    let mut ids: Vec<&u32> = t.templates.keys().collect();
    ids.sort();
    for id in ids.iter().take(6) {
        let m = &t.templates[id];
        println!(
            "  {:6} {:24} nv {:3} vida {:7} def {:5} arm {:5} ataque {:5} dano {}..{} alcance {:.1} vel {:.1} IA {} skills {}",
            m.id, m.nome, m.nivel, m.vida, m.defesa, m.armadura, m.taxa_de_ataque,
            m.dano_fisico.minimo, m.dano_fisico.maximo, m.alcance_de_ataque,
            m.velocidade_correndo, m.politica_de_ia, m.skills.len()
        );
    }
}

