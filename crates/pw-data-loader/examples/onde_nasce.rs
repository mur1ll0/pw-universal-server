//! Em que pastas de mapa um template nasce, e onde.
//!
//! Uso: `cargo run -p pw-data-loader --example onde_nasce -- <pasta config> <tid>...`
//! Com `MUNDO=<id>`, também a altura resolvida naquele mapa (terreno, piso, `fHeiOff`).
use pw_data_loader::NpcGenData;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tids: Vec<u32> = args[2..].iter().filter_map(|v| v.parse().ok()).collect();
    for e in std::fs::read_dir(&args[1]).unwrap().flatten() {
        let arq = e.path().join("npcgen.data");
        let Ok(b) = std::fs::read(&arq) else { continue };
        let Ok(n) = NpcGenData::load_from_bytes(&b) else { continue };
        for s in n.instances.iter().filter(|s| tids.contains(&s.template_id)) {
            println!("{} tid {} em {:?}", e.path().file_name().unwrap().to_string_lossy(), s.template_id, s.pos);
            if let Some(w) = std::env::var("MUNDO").ok().and_then(|v| v.parse::<i32>().ok()) {
                let (t, m) = (pw_data_loader::Terreno::ler(w, &e.path()), pw_data_loader::MapaDeMovimento::ler(w, &e.path()));
                println!(
                    "   terreno {:?}, piso acima {:?}, fHeiOff {}, tipo {:?} → {:?}",
                    t.altura_em(s.pos.x, s.pos.z),
                    m.acima_do_terreno(s.pos.x, s.pos.z),
                    s.acima_do_chao,
                    s.tipo_de_area,
                    s.posicao_no_mapa(&t, &m)
                );
            }
        }
    }
}
