//! O que um personagem do banco vira no mundo, com as tabelas do realm.
use pw_data_loader::{classes, generic_elements::load_elements_data_auto, ptemplate};
fn main() {
    let dir = std::env::args().nth(1).expect("uso: jogador_esperado <pasta config>");
    let el = load_elements_data_auto(&std::fs::read(format!("{dir}/elements.data")).unwrap()).unwrap();
    let cls = classes::carregar(&el);
    let base = ptemplate::ler_da_pasta(std::path::Path::new(&dir)).expect("ptemplate.conf");
    println!("{:<12} {:>5} {:>5} {:>7} {:>7}", "classe", "vida", "mana", "precisão", "evasão");
    for c in [0i32, 1, 7] {
        let (b, k) = (base.get(c).unwrap(), cls.get(c).unwrap());
        // nível 1, com os atributos iniciais da própria classe
        let hp = b.vida + k.vida_por_vitalidade * b.vitalidade;
        let mp = b.mana + k.mana_por_energia * b.energia;
        println!(
            "{:<12} {:>5} {:>5} {:>8} {:>7}   (base {} + {}×{} de vitalidade)",
            match c { 0 => "Guerreiro", 1 => "Mago", _ => "Sacerdote" },
            hp, mp, k.precisao_base(b.agilidade), k.evasao_base(b.agilidade),
            b.vida, k.vida_por_vitalidade, b.vitalidade
        );
    }
}
