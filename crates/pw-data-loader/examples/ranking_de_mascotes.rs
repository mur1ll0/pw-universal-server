//! Mascotes de combate (`PET_ESSENCE.id_type` 8782) de ataque à distância, com os atributos num
//! nível pelo `GenerateBaseProp`, o ovo, o preço e os NPCs que vendem o ovo.
//! Uso: `--example ranking_de_mascotes -- <pasta config> <nível> <alcance mínimo>`
use pw_data_loader::generic_elements::Record;
use std::collections::HashMap;

fn i(r: &Record, c: &str) -> i32 {
    r.get(c).and_then(|v| v.as_i32()).unwrap_or(0)
}
fn nome(r: &Record) -> String {
    r.get("Name").map(|v| format!("{v:?}")).unwrap_or_default()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&a[1]));
    let nivel: i32 = a[2].parse().unwrap();
    let alcance_minimo: f32 = a[3].parse().unwrap();
    let g = d.elements_generic.as_ref().expect("elements genérico");
    let nomes: HashMap<i32, String> = g
        .get("PET_ESSENCE")
        .iter()
        .chain(g.get("PET_EGG_ESSENCE").iter())
        .chain(g.get("NPC_ESSENCE").iter())
        .map(|r| (i(r, "ID"), nome(r)))
        .collect();
    // Quem vende cada item: NPC → serviço de venda → páginas.
    let mut vende: HashMap<i32, Vec<i32>> = HashMap::new();
    let servicos: HashMap<i32, &Record> = g.get("NPC_SELL_SERVICE").iter().map(|r| (i(r, "ID"), r)).collect();
    for npc in g.get("NPC_ESSENCE").iter() {
        if let Some(s) = servicos.get(&i(npc, "id_sell_service")) {
            for p in 1..=8 {
                for k in 1..=32 {
                    let item = i(s, &format!("pages_{p}_goods_{k}_id"));
                    if item > 0 {
                        vende.entry(item).or_default().push(i(npc, "ID"));
                    }
                }
            }
        }
    }
    let mut linhas = Vec::new();
    for (tid, m) in &d.modelos_de_mascote {
        if m.classe != 1 || m.alcance < alcance_minimo {
            continue;
        }
        let at = m.atributos(nivel.min(m.nivel_maximo.max(1)));
        let dps = at.dano as f32 * 20.0 / at.intervalo_do_golpe.max(1) as f32;
        let ovos: Vec<_> = d.ovos_de_pet.values().filter(|o| o.id_pet == *tid).collect();
        let ovo = ovos.first().map(|o| (o.id, o.money_hatched, o.req_level)).unwrap_or((0, 0, 0));
        let mut npcs: Vec<i32> = ovos.iter().flat_map(|o| vende.get(&(o.id as i32)).cloned().unwrap_or_default()).collect();
        npcs.sort();
        npcs.dedup();
        linhas.push((dps, *tid, m.alcance, m.nivel_exigido, m.nivel_maximo, at, ovo, npcs));
    }
    linhas.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (dps, tid, alc, req, max, at, ovo, npcs) in linhas.iter().take(1000) {
        let vendedores: Vec<String> = npcs.iter().take(3).map(|n| format!("{n} {}", nomes.get(n).cloned().unwrap_or_default())).collect();
        println!(
            "{tid} {} | alcance {alc} | nível exigido {req} máx {max} | vida {} dano {} a cada {:.2} s = {dps:.0}/s | acerto {} defesa {} esquiva {} resist {} | ovo {} {} (chocar {}, nível {}) | vende: {:?}",
            nomes.get(&(*tid as i32)).cloned().unwrap_or_default(),
            at.vida, at.dano, at.intervalo_do_golpe as f32 / 20.0, at.acerto, at.defesa, at.esquiva, at.resistencia,
            ovo.0, nomes.get(&(ovo.0 as i32)).cloned().unwrap_or_default(), ovo.1, ovo.2, vendedores
        );
    }
}
