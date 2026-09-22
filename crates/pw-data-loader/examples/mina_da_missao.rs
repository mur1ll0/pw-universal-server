//! As minas (matérias colhíveis) ligadas a uma missão: o que produzem e com que chance.
//!
//! Uso: `cargo run -p pw-data-loader --example mina_da_missao -- <pasta config> [id da missão]`
use pw_data_loader::GameDataManager;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let missao: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut d = GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&args[1]));
    for (id, m) in &d.minas {
        if missao != 0 && m.missao_de_entrada != missao && m.missao_de_saida != missao {
            continue;
        }
        if missao == 0 && m.missao_de_saida == 0 {
            continue;
        }
        println!(
            "mina {id}: entrada={} saída={} sucesso={:.2} tempo={}..{} exp={} materiais={:?}",
            m.missao_de_entrada,
            m.missao_de_saida,
            m.chance_de_sucesso,
            m.tempo_minimo,
            m.tempo_maximo,
            m.exp,
            m.materiais.iter().map(|x| (x.item, x.probabilidade)).collect::<Vec<_>>()
        );
    }
}
