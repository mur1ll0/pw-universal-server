//! As missões de uma Carta da Sorte. Uso: `--example carta -- <pasta config> <id>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let e = pw_data_loader::generic_elements::load_elements_data_auto(&std::fs::read(format!("{}/elements.data", a[1])).unwrap()).unwrap();
    let t = pw_data_loader::cartas::carregar(&e);
    println!("{} cartas", t.len());
    println!("{:?}", t.get(&a[2].parse().unwrap()));
}
