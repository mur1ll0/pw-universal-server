use pw_data_loader::generic_elements::load_elements_data_auto;
fn main() {
    let d = std::env::args().nth(1).unwrap();
    let e = load_elements_data_auto(&std::fs::read(format!("{d}/elements.data")).unwrap()).unwrap();
    for t in ["SKILL_TYPE", "SKILL_ESSENCE", "SKILLMATTER_ESSENCE"] {
        println!("{t}: {} registros", e.get(t).len());
    }
    for r in e.get("SKILLMATTER_ESSENCE").iter().take(3) {
        let mut k: Vec<&String> = r.keys().collect(); k.sort();
        println!("campos: {:?}", &k[..k.len().min(12)]);
        break;
    }
}
