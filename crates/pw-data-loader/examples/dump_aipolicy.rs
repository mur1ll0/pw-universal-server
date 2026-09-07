//! Estatísticas do `aipolicy.data` lido pelo `pw-data-loader`.
//!
//! `cargo run -p pw-data-loader --example dump_aipolicy -- data/realm_155/config/aipolicy.data`
use pw_data_loader::aipolicy::{AiPolicyData, ParametroDeOperacao};
use std::collections::BTreeMap;

fn main() {
    let caminho = std::env::args().nth(1).expect("uso: dump_aipolicy <aipolicy.data>");
    let bytes = std::fs::read(&caminho).expect("não consegui ler o arquivo");
    let dados = AiPolicyData::load_from_bytes(&bytes).expect("falhou ao ler");

    let mut por_operacao: BTreeMap<i32, usize> = BTreeMap::new();
    let mut por_alvo: BTreeMap<i32, usize> = BTreeMap::new();
    let mut por_condicao: BTreeMap<i32, usize> = BTreeMap::new();
    let mut niveis: BTreeMap<u32, usize> = BTreeMap::new();
    let mut falas = 0usize;
    let mut triggers = 0usize;

    for p in dados.policies.values() {
        triggers += p.triggers.len();
        for t in &p.triggers {
            let mut pilha: Vec<&_> = t.condicao.iter().collect();
            while let Some(no) = pilha.pop() {
                *por_condicao.entry(no.tipo_cru).or_default() += 1;
                pilha.extend(no.esquerda.iter().map(|b| &**b));
                pilha.extend(no.direita.iter().map(|b| &**b));
            }
            for op in &t.operacoes {
                *por_operacao.entry(op.tipo_cru).or_default() += 1;
                *por_alvo.entry(op.alvo.tipo_cru).or_default() += 1;
                match &op.parametro {
                    ParametroDeOperacao::Skill { nivel, .. } => {
                        *niveis.entry(*nivel).or_default() += 1;
                    }
                    ParametroDeOperacao::Fala { .. }
                    | ParametroDeOperacao::Fala2 { .. }
                    | ParametroDeOperacao::SkillComFala { .. } => falas += 1,
                    _ => {}
                }
            }
        }
    }

    println!("versão do cabeçalho: {}", dados.versao);
    println!("políticas: {} (ids repetidos: {})", dados.policies.len(), dados.ids_repetidos.len());
    println!("triggers: {triggers}");
    println!("falas: {falas}");
    println!("\ncondições por tipo: {por_condicao:?}");
    println!("\noperações por tipo: {por_operacao:?}");
    println!("\nalvos por tipo: {por_alvo:?}");
    println!("\nníveis de o_use_skill: {niveis:?}");
}
