//! Gera o bloco de dados de um equipamento, como o drop/prêmio de missão do original.
//!
//! `cargo run -p pw-gs --example gerar_octetos -- data/realm_155/config 28790 28794`
//! Imprime `item_id<TAB>hex` e o que as propriedades somam, para conferir antes de gravar.
fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("uso: <config> <ids...>");
    let ids: Vec<u32> = a.filter_map(|s| s.parse().ok()).collect();
    let mut dm = pw_data_loader::GameDataManager::new();
    let rel = dm.load_from_directory(&dir);
    eprintln!("{} arquivo(s) lidos de {dir}", rel.lidos.len());
    for id in ids {
        match pw_gs::geracao::gerar_equipamento(&dm, id) {
            None => println!("{id}\t(sem modelo de geração)"),
            Some(c) => {
                let hex: String = c.escrever().iter().map(|b| format!("{b:02x}")).collect();
                let addons: Vec<String> = c
                    .addons
                    .iter()
                    .map(|x| format!("{}{:?}", x.id(), x.args))
                    .collect();
                println!("{id}\t{hex}");
                println!("   durabilidade {}/{} | addons: {}", c.durabilidade, c.durabilidade_maxima, if addons.is_empty() { "nenhum".into() } else { addons.join(", ") });
            }
        }
    }
}
