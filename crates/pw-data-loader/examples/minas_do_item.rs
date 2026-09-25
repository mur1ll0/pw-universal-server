//! Minas (`MINE_ESSENCE`) que dão um item ou cujo nome contém um texto, com o que exigem:
//! `cargo run -p pw-data-loader --example minas_do_item -- <config> <item> [trecho do nome]`.
use pw_data_loader::generic_elements::FieldValue;
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("config");
    let item: i32 = a.next().expect("item").parse().unwrap();
    let trecho = a.next().unwrap_or_default().to_lowercase();
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&dir));
    let e = d.elements_generic.as_ref().expect("elements");
    let txt = |v: Option<&FieldValue>| match v { Some(FieldValue::Text(s)) => s.clone(), Some(x) => format!("{x:?}"), None => String::new() };
    for tabela in ["MINE_ESSENCE"] {
        for r in e.get(tabela) {
            let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            let nome = txt(r.get("Name"));
            let mats: Vec<i32> = (1..=16).map(|k| i(&format!("materials_{k}_id"))).filter(|x| *x != 0).collect();
            if mats.contains(&item) || [3427, 3428].contains(&i("task_in")) || [3427, 3428].contains(&i("task_out")) || (!trecho.is_empty() && nome.to_lowercase().contains(&trecho)) {
                if std::env::var("CRU").is_ok() { let mut k: Vec<_> = r.iter().collect(); k.sort_by_key(|x| x.0.clone()); for (n, v) in k { if !matches!(v, FieldValue::Int(0)) && !matches!(v, FieldValue::Raw(_)) { println!("   {n} = {v:?}"); } } }
                let carregada = d.minas.contains_key(&(i("ID") as u32));
                println!("mina {} {:?}: materiais {:?} num1 {} task_in {} task_out {} nível {} ferramenta {} tempo {}..{} tipo {} carregada {}",
                    i("ID"), nome, mats, i("num1"), i("task_in"), i("task_out"), i("level_required"), i("id_equipment_required"), i("time_min"), i("time_max"), i("mine_type"), carregada);
            }
        }
    }
    for (mapa, s) in &d.map_spawns {
        for x in s.instances.iter().filter(|x| x.template_id as i32 == item || [11117, 12858].contains(&(x.template_id as i32))) {
            let w = std::path::Path::new(&dir).join("world");
            let (ter, mov) = (pw_data_loader::Terreno::ler(*mapa as i32, &w), pw_data_loader::MapaDeMovimento::ler(*mapa as i32, &w));
            println!("npcgen mapa {mapa}: modelo {} fHeiOff {} acima pelo mapa de movimento {:?} tipo {:?} área {:?} em {:?} → no mundo {:?} (terreno {:?})", x.template_id, x.acima_do_chao, mov.acima_do_terreno(x.pos.x, x.pos.z), x.spawn_type, x.tipo_de_area, x.pos, x.posicao_no_mapa(&ter, &mov), ter.altura_em(x.pos.x, x.pos.z));
        }
    }
    for tabela in ["TASKNORMALMATTER_ESSENCE", "TASKMATTER_ESSENCE", "MATERIAL_ESSENCE"] {
        for r in e.get(tabela) {
            if r.get("ID").and_then(|v| v.as_i32()) == Some(item) {
                println!("item {item} em {tabela}: {:?}", txt(r.get("Name")));
            }
        }
    }
}
