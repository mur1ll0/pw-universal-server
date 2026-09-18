//! Lê todo `npcgen.data` abaixo de uma pasta e diz versão, entidades e se fechou no último
//! byte (o leitor recusa sobra).
//!
//! Uso: `cargo run -p pw-data-loader --example conferir_npcgen -- <pasta>`
use pw_data_loader::NpcGenData;

fn visitar(dir: &std::path::Path, ok: &mut usize, falhas: &mut usize) {
    let Ok(entradas) = std::fs::read_dir(dir) else { return };
    for e in entradas.flatten() {
        let p = e.path();
        if p.is_dir() {
            visitar(&p, ok, falhas);
        } else if p.file_name().is_some_and(|n| n == "npcgen.data") {
            let bytes = std::fs::read(&p).unwrap();
            if bytes.len() < 4 {
                println!("vazio ({} bytes)  {}", bytes.len(), p.display());
                continue;
            }
            let v = u32::from_le_bytes(bytes[..4].try_into().unwrap());
            match NpcGenData::load_from_bytes(&bytes) {
                Ok(n) => {
                    *ok += 1;
                    println!("ok    v{v:2} {:6} entidades  {}", n.instances.len(), p.display());
                }
                Err(e) => {
                    *falhas += 1;
                    println!("FALHA v{v:2} {e}  {}", p.display());
                }
            }
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("pasta");
    let (mut ok, mut falhas) = (0, 0);
    visitar(std::path::Path::new(&dir), &mut ok, &mut falhas);
    println!("{ok} lidos, {falhas} com falha");
}
