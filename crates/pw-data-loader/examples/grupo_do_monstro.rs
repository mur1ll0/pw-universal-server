//! Onde nasce um monstro (pelo nome), com o tipo de grupo da área, os outros geradores da
//! mesma área e a rota de patrulha — para conferir líder, subordinados e caminho.
//!
//! Uso: `cargo run -p pw-data-loader --example grupo_do_monstro -- <pasta config> <parte do nome>`
use pw_data_loader::{generic_elements::load_elements_data_auto, monstros, NpcGenData};
use std::collections::BTreeMap;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = &a[1];
    let busca = a[2].to_lowercase();
    let el = load_elements_data_auto(&std::fs::read(format!("{dir}/elements.data")).unwrap()).unwrap();
    let mobs = monstros::carregar(&el, None);
    let nome = |t: u32| mobs.get(t).map(|m| m.nome.clone()).unwrap_or_else(|| format!("#{t}"));
    let alvos: Vec<u32> = mobs.templates.values().filter(|m| m.nome.to_lowercase().contains(&busca)).map(|m| m.id).collect();
    for t in &alvos {
        let m = mobs.get(*t).unwrap();
        println!("{} = {} (patrulha {}, estratégia {})", t, m.nome, m.patrulha, m.estrategia);
    }
    let b = std::fs::read(format!("{dir}/world/npcgen.data")).unwrap();
    let n = NpcGenData::load_from_bytes(&b).unwrap();
    let mut areas: BTreeMap<i32, Vec<&pw_data_loader::SpawnInstance>> = BTreeMap::new();
    for s in &n.instances {
        if s.area_de_ia >= 0 {
            areas.entry(s.area_de_ia).or_default().push(s);
        }
    }
    for (area, ss) in &areas {
        if !ss.iter().any(|s| alvos.contains(&s.template_id)) {
            continue;
        }
        let s0 = ss[0];
        println!(
            "área {area}: grupo {} em ({:.0}, {:.0}, {:.0})",
            s0.tipo_de_grupo, s0.centro_da_area.x, s0.centro_da_area.y, s0.centro_da_area.z
        );
        for s in ss {
            println!(
                "  gerador {} {} ({}) caminho {} laço {} corre {}",
                s.gerador_na_area, s.template_id, nome(s.template_id), s.caminho, s.laco_do_caminho, s.corre_no_caminho
            );
        }
    }
}
