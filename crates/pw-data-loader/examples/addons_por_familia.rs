//! Para cada id de `EQUIPMENT_ADDON`: os parâmetros brutos e as famílias cujas essências o
//! sorteiam (`WEAPON/ARMOR/DECORATION_ESSENCE`: `addons_N_id_addon`, `rands_N_id_rand`,
//! `uniques_N_id_unique`). Alimenta `specs/addons_155/classificar_addons.py`.
//! Uso: `--example addons_por_familia -- <pasta config>` → linhas
//! `id<TAB>num_params<TAB>p1<TAB>p2<TAB>p3<TAB>famílias(arma,armadura,acessorio)<TAB>nome`.
use pw_data_loader::generic_elements::load_elements_data_auto;
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let mut familias: BTreeMap<i32, BTreeSet<&str>> = BTreeMap::new();
    for (tabela, familia) in [("WEAPON_ESSENCE", "arma"), ("ARMOR_ESSENCE", "armadura"), ("DECORATION_ESSENCE", "acessorio")] {
        for r in e.get(tabela) {
            for (campo, v) in r.iter() {
                let lista = (campo.starts_with("addons_") && campo.ends_with("_id_addon"))
                    || (campo.starts_with("rands_") && campo.ends_with("_id_rand"))
                    || (campo.starts_with("uniques_") && campo.ends_with("_id_unique"));
                if let (true, Some(id)) = (lista, v.as_i32()) {
                    if id > 0 {
                        familias.entry(id).or_default().insert(familia);
                    }
                }
            }
        }
    }
    for r in e.get("EQUIPMENT_ADDON") {
        let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        let id = i("ID");
        let fam: Vec<&str> = familias.get(&id).map(|s| s.iter().copied().collect()).unwrap_or_default();
        let nome = r.get("Name").and_then(|v| v.as_text()).unwrap_or_default();
        println!("{id}\t{}\t{}\t{}\t{}\t{}\t{nome}", i("num_params"), i("param1"), i("param2"), i("param3"), fam.join(","));
    }
}
