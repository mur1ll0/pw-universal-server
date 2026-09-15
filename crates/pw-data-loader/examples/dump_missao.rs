//! Imprime uma missão do `tasks.data` e, recursivamente, as submissões.
//!
//! Uso: `cargo run -p pw-data-loader --example dump_missao -- <pasta config> <id>`
use pw_data_loader::tasks::TasksData;

fn imprimir(t: &TasksData, id: u32, recuo: usize) {
    let Some(m) = t.tasks.get(&id) else {
        println!("{:recuo$}{id}: não existe", "");
        return;
    };
    let mut c = m.clone();
    c.descricao = String::new();
    c.sub_tasks.clear();
    println!("{:recuo$}{c:?}", "");
    for f in &m.sub_tasks {
        imprimir(t, *f, recuo + 4);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = TasksData::load_from_bytes(&std::fs::read(format!("{}/tasks.data", args[1])).unwrap()).unwrap();
    for id in &args[2..] {
        imprimir(&t, id.parse().unwrap(), 0);
    }
}
