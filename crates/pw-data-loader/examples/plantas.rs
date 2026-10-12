//! Os ovos de planta (`PET_CLASS_PLANT`, `id_type` 28913) do realm e o que cada um invoca:
//! `cargo run -p pw-data-loader --example plantas -- <config> [ovo...]`. Sem ovos, lista todos.
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("config");
    let pedidos: Vec<u32> = a.map(|x| x.parse().unwrap()).collect();
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&dir));
    let e = d.elements_generic.as_ref().expect("elements");
    let i = |r: &pw_data_loader::generic_elements::Record, c: &str| r.get(c).and_then(|v| v.as_i32()).unwrap_or(0);
    let mut ovos: Vec<_> = d.ovos_de_pet.values().filter(|o| o.pet_class == 4).collect();
    ovos.sort_by_key(|o| o.id);
    for o in ovos {
        if !pedidos.is_empty() && !pedidos.contains(&o.id) {
            continue;
        }
        let pet = e.get("PET_ESSENCE").iter().find(|r| i(r, "ID") as u32 == o.id_pet);
        let (grupo, limite, habitat, vel) = pet
            .map(|r| (i(r, "plant_group"), i(r, "group_limit"), i(r, "inhabit_type"), r.get("speed_a").map(|v| format!("{v:?}")).unwrap_or_default()))
            .unwrap_or_default();
        println!("ovo {} → pet {} grupo {grupo} limite {limite} habitat {habitat} vel {vel} habilidades {:?}", o.id, o.id_pet, o.skills);
    }
}
