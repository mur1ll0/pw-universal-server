//! B335: conta as missões com falha por região de entrar/sair e por data no tasks.data do realm.
//! `cargo run -p pw-data-loader --example falhas_de_missao -- <tasks.data>`
fn main() {
    let caminho = std::env::args().nth(1).expect("caminho do tasks.data");
    let t = pw_data_loader::tasks::TasksData::load_from_bytes(&std::fs::read(&caminho).expect("ler")).expect("ler");
    let (mut e, mut s, mut d) = (0, 0, 0);
    let mut mundos = std::collections::BTreeMap::new();
    for m in t.tasks.values() {
        if let Some((w, rs)) = &m.falha_ao_entrar { e += 1; *mundos.entry(("entrar", *w)).or_insert(0) += 1; if e <= 2 { println!("entrar {} mundo {w} {:?}", m.id, rs.first()); } }
        if let Some((w, rs)) = &m.falha_ao_sair { s += 1; *mundos.entry(("sair", *w)).or_insert(0) += 1; if s <= 2 { println!("sair {} mundo {w} {} caixas {:?}", m.id, rs.len(), rs.first()); } }
        if let Some(x) = &m.falha_na_data { d += 1; if d <= 3 { println!("data {} {:?}", m.id, x); } }
    }
    println!("entrar {e}, sair {s}, data {d}; mundos {mundos:?}");
    escalas(&t);
}

/// B340: as escalas de prêmio (por tempo e por itens) no mesmo arquivo.
#[allow(dead_code)]
fn escalas(t: &pw_data_loader::tasks::TasksData) {
    let mut por = [0; 4];
    for m in t.tasks.values() {
        for (i, e) in [&m.escala_de_tempo_sucesso, &m.escala_de_tempo_falha, &m.escala_de_itens_sucesso, &m.escala_de_itens_falha].iter().enumerate() {
            if !e.premios.is_empty() {
                por[i] += 1;
                if por[i] == 1 { println!("escala {i} na {}: item {} limiares {:?}, {} prêmios", m.id, e.item, e.limiares, e.premios.len()); }
            }
        }
    }
    println!("escalas (tempo S/F, itens S/F): {por:?}");
}
