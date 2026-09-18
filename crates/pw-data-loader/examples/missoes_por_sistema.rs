//! Conta e mostra exemplos das missões de cada sistema do motor: horário, região de
//! entrega, lugar a alcançar/sair, equipe e facção.
//!
//! Uso: `cargo run -p pw-data-loader --example missoes_por_sistema -- <pasta config>`
use pw_data_loader::TasksData;

fn main() {
    let pasta = std::env::args().nth(1).expect("pasta");
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{pasta}/tasks.data")).unwrap()).unwrap();
    let todas: Vec<_> = t.tasks.values().collect();
    let mostrar = |nome: &str, f: &dyn Fn(&&pw_data_loader::tasks::TaskTemplate) -> bool| {
        let v: Vec<_> = todas.iter().filter(|x| f(x)).collect();
        println!("== {nome}: {}", v.len());
        for x in v.iter().take(3) {
            println!("  {} {:?} janelas={:?} entrega=({}, {:?}) alcancar=({}, {:?}) sair=({}, {:?}) membros={:?} faccao={} metodo={}",
                x.id, x.name, x.janelas.first(), x.mundo_de_entrega, x.regioes_de_entrega.first(),
                x.mundo_a_alcancar, x.lugares_a_alcancar.first(), x.mundo_a_sair, x.lugares_a_sair.first(),
                x.membros_pedidos.first(), x.faccao, x.metodo);
        }
    };
    mostrar("com janela de horário", &|x| !x.janelas.is_empty());
    mostrar("entrega em zona", &|x| x.entrega_em_zona);
    mostrar("alcançar lugar", &|x| !x.lugares_a_alcancar.is_empty());
    mostrar("sair de lugar", &|x| !x.lugares_a_sair.is_empty());
    mostrar("equipe recebida pelo capitão", &|x| x.em_equipe && x.recebida_pela_equipe);
    mostrar("facção", &|x| x.faccao != 0);
}
