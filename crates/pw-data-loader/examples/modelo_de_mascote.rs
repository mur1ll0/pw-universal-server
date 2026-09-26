//! Atributos de um mascote pelo `GenerateBaseProp` em alguns níveis.
//! Uso: `--example modelo_de_mascote -- <pasta config> <pet_tid> [nível...]`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&a[1]));
    let tid: u32 = a[2].parse().unwrap();
    let m = d.modelos_de_mascote.get(&tid).expect("sem modelo");
    println!("corpo {} alcance {} defesa {:?} esquiva {:?} dano {:?} vida {:?}", m.corpo, m.alcance, m.defesa, m.esquiva, m.dano, m.hp);
    for n in a[3..].iter().map(|v| v.parse::<i32>().unwrap()) {
        println!("nível {n}: {:?}", m.atributos(n));
    }
}
