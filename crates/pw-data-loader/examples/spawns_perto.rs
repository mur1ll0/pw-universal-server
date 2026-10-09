//! Spawns de um `npcgen.data` num raio de um ponto (x, z), com altura e controlador.
//!
//! Uso: `cargo run -p pw-data-loader --example spawns_perto -- <npcgen.data> <x> <z> <raio> [tid...]`
use pw_data_loader::npcgen::NpcGenData;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let d = NpcGenData::load_from_bytes(&std::fs::read(&a[1]).unwrap()).unwrap();
    let (x, z, r): (f32, f32, f32) = (a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
    let tids: Vec<u32> = a[5..].iter().filter_map(|t| t.parse().ok()).collect();
    let mostra = |orig: &str, s: &pw_data_loader::npcgen::SpawnInstance| {
        let dist = ((s.pos.x - x).powi(2) + (s.pos.z - z).powi(2)).sqrt();
        if (dist <= r || tids.contains(&s.template_id)) && (tids.is_empty() || tids.contains(&s.template_id)) {
            println!(
                "{orig} tid {} {:?} pos ({:.1},{:.1},{:.1}) dist {:.0} area {:?} ext ({:.1},{:.1},{:.1}) chao {:.1} agua {:.1} gerador {}",
                s.template_id, s.spawn_type, s.pos.x, s.pos.y, s.pos.z, dist, s.tipo_de_area,
                s.extensao_da_area.x, s.extensao_da_area.y, s.extensao_da_area.z, s.acima_do_chao, s.acima_da_agua, s.gerador_na_area
            );
        }
    };
    for s in &d.instances {
        mostra("ativo", s);
    }
    for (id, c) in &d.controladores {
        for s in &c.pendentes {
            mostra(&format!("ctrl {id} (ativo_de_inicio {})", c.ativo_de_inicio), s);
        }
    }
}
