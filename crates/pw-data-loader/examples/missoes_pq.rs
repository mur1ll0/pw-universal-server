//! B348 — as PQ (`m_bPQTask`) do `tasks.data` de um realm: quantas, com os filhos (`m_bPQSubTask`).
use pw_data_loader::GameDataManager;
fn main() {
    let pasta = std::env::args().nth(1).unwrap_or_else(|| "data/realm_155/config".into());
    let mut d = GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&pasta));
    let mut pq: Vec<_> = d.tasks.tasks.values().filter(|t| t.pq).collect();
    pq.sort_by_key(|t| t.id);
    let subs = d.tasks.tasks.values().filter(|t| t.pq_sub).count();
    println!("{} PQ, {subs} etapas", pq.len());
    for t in pq.iter().filter(|t| t.id == 20759 || t.id == 21299) {
        println!("PQ {} '{}' chaves {:?} filhos {:?}", t.id, t.name, t.pq_chaves, t.sub_tasks);
        for f in &t.sub_tasks {
            let Some(e) = d.tasks.tasks.get(f) else { continue };
            println!("  etapa {} metodo {} conclusao {} chaves {:?} ctrl ({}, {}) fim {:?} contrib {:?} premio contrib {}+{} min {} ctrl ({}, {}) colocacao {:?}",
                e.id, e.metodo, e.tipo_de_conclusao, e.pq_chaves, e.controle_de_monstros, e.dispara_controle, e.global_fim, e.contribuicao_por_monstro.iter().take(2).collect::<Vec<_>>(),
                e.rewards.contribuicao, e.rewards.contribuicao_sorteada, e.rewards.contribuicao_minima, e.rewards.controle_de_monstros, e.rewards.dispara_controle,
                e.rewards.premio_da_pq.as_ref().map(|p| p.faixas.len()));
        }
        println!("  topo metodo {} conclusao {}: premio min {} colocacao {:?}", t.metodo, t.tipo_de_conclusao, t.rewards.contribuicao_minima, t.rewards.premio_da_pq);
    }
}
