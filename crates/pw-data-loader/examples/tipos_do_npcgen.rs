//! Quantas criaturas de cada tipo um `npcgen.data` tem, e quantas não renascem.
//!
//! Uso: `cargo run -p pw-data-loader --example tipos_do_npcgen -- <npcgen.data> [tid...]`
use pw_data_loader::npcgen::NpcGenData;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let filtro: Vec<u32> = a[2..].iter().filter_map(|v| v.parse().ok()).collect();
    let d = NpcGenData::load_from_bytes(&std::fs::read(&a[1]).unwrap()).unwrap();
    let todas: Vec<_> = d.instances.iter().map(|i| (i, "início")).chain(d.controladores.iter().flat_map(|(n, c)| c.pendentes.iter().map(move |i| (i, if c.ativo_de_inicio { "controlador ativo" } else { "controlador" })).map(move |x| { let _ = n; x }))).collect();
    let mut c = std::collections::BTreeMap::new();
    for (i, origem) in &todas {
        *c.entry((format!("{:?}", i.spawn_type), *origem, i.renasce)).or_insert(0) += 1;
        if filtro.contains(&i.template_id) {
            println!("tid {} {:?} {origem} renasce {} renascer {}..{} s", i.template_id, i.spawn_type, i.renasce, i.renascer_min_s, i.renascer_max_s);
        }
    }
    for ((t, o, r), n) in c { println!("{t:>14} {o:<18} renasce {r:<5} {n}"); }
}
