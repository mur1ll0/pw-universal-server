//! B354 — as missões cujo prêmio divorcia (`m_bDivorce`) no `tasks.data` de um realm.
use pw_data_loader::GameDataManager;
fn main() {
    let pasta = std::env::args().nth(1).unwrap_or_else(|| "data/realm_155/config".into());
    let mut d = GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&pasta));
    let mut v: Vec<_> = d.tasks.tasks.values().filter(|t| t.rewards.divorcio).map(|t| (t.id, t.name.clone(), t.so_casal, t.conjuge)).collect();
    v.sort();
    println!("{} missões divorciam", v.len());
    for (id, nome, casal, conjuge) in v.iter().take(10) {
        println!("  {id} '{nome}' so_casal {casal} conjuge {conjuge}");
    }
}
