use pw_data_loader::generic_elements::load_elements_data_auto;
fn main() {
    let d = std::env::args().nth(1).unwrap();
    let e = load_elements_data_auto(&std::fs::read(format!("{d}/elements.data")).unwrap()).unwrap();
    let t = e.get("CHARRACTER_CLASS_CONFIG");
    println!("registros: {}", t.len());
    for r in t {
        let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(-1);
        let f = |n: &str| match r.get(n) {
            Some(pw_data_loader::generic_elements::FieldValue::Float(v)) => *v,
            Some(pw_data_loader::generic_elements::FieldValue::Int(v)) => *v as f32,
            _ => -1.0,
        };
        println!(
            "cls {:2} agi_attack {:3} agi_armor {:3} crit {:2} vit_hp {:2} eng_mp {:2} lvl_dmg {:2} lvl_def {:2} atk_speed {:.2} atk_range {:.2}",
            i("character_class_id"), i("agi_attack"), i("agi_armor"), i("crit_rate"),
            i("vit_hp"), i("eng_mp"), i("lvlup_dmg"), i("lvlup_defense"),
            f("attack_speed"), f("attack_range")
        );
    }
}
