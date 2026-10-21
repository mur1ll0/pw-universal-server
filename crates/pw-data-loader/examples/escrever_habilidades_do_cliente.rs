//! B315: modelo do cliente (JSON de `tools/elementskill/gerar_modelo.py`) → `habilidades.pwsk`, com ida e volta.
//!
//!     cargo run -p pw-data-loader --example escrever_habilidades_do_cliente -- <modelo.json> <habilidades.pwsk>
use pw_data_loader::habilidades_do_cliente::{escrever, ler, Modelo};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let modelo: Modelo = serde_json::from_slice(&std::fs::read(&a[1]).expect("modelo")).expect("modelo JSON");
    let b = escrever(&modelo).expect("escrever");
    let volta = ler(&b).expect("ler");
    assert!(volta == modelo, "a ida e volta mudou o modelo");
    std::fs::write(&a[2], &b).expect("gravar");
    println!("{} habilidades, {} bytes -> {} (ida e volta igual)", modelo.habilidades.len(), b.len(), a[2]);
}
