//! Os modelos (tid) de um `npcgen.data`, com quantidade e uma posição.
//!
//! Uso: `cargo run -p pw-data-loader --example tids_do_npcgen -- <npcgen.data>`
use pw_data_loader::npcgen::NpcGenData;
fn main() {
    let p = std::env::args().nth(1).expect("uso: tids_do_npcgen <npcgen.data>");
    let d = NpcGenData::load_from_bytes(&std::fs::read(&p).unwrap()).unwrap();
    let mut m: std::collections::BTreeMap<u32, (usize, String)> = Default::default();
    for i in d.instances.iter().chain(d.controladores.values().flat_map(|c| c.pendentes.iter())) {
        let e = m.entry(i.template_id).or_insert((0, format!("{:?} ({:.0},{:.0},{:.0})", i.spawn_type, i.pos.x, i.pos.y, i.pos.z)));
        e.0 += 1;
    }
    for (t, (n, s)) in m { println!("{t} x{n} {s}"); }
}
