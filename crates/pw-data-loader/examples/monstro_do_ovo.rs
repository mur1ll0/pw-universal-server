//! Os monstros que dão cada ovo na captura (`MONSTER_ESSENCE.id_pet_egg_captured`), com nível.
//! Uso: `--example monstro_do_ovo -- <pasta config> <ovo>...`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&a[1]));
    let g = d.elements_generic.as_ref().expect("elements");
    let i = |r: &pw_data_loader::generic_elements::Record, c: &str| r.get(c).and_then(|v| v.as_i32()).unwrap_or(0);
    for ovo in a[2..].iter().map(|v| v.parse::<i32>().unwrap()) {
        let m: Vec<String> = g
            .get("MONSTER_ESSENCE")
            .iter()
            .filter(|r| i(r, "id_pet_egg_captured") == ovo)
            .map(|r| format!("{} {:?} nível {}", i(r, "ID"), r.get("Name").map(|v| format!("{v:?}")).unwrap_or_default(), i(r, "level")))
            .collect();
        println!("ovo {ovo}: {m:?}");
    }
}
