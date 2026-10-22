//! B354 — o `WEDDING_CONFIG` (801) e os cartões de casamento do `elements.data` de um realm.
use pw_data_loader::GameDataManager;
fn main() {
    let pasta = std::env::args().nth(1).unwrap_or_else(|| "data/realm_155/config".into());
    let mut d = GameDataManager::new();
    d.load_from_directory(std::path::Path::new(&pasta));
    match &d.casamento {
        Some(c) => println!("sessões {:?}\ndias {:?}\ncenas {:?}\nreserva {:?}\nconvites {:?}", c.sessoes, c.dias_reservados, &c.cenas[..2], c.cartoes_de_reserva, c.convites),
        None => println!("sem WEDDING_CONFIG 801 válido"),
    }
}
