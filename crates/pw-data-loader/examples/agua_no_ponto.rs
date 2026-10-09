//! Altura da água (`watermap/`), do terreno e o piso do `movemap` num ponto do mapa.
//! Uso: `--example agua_no_ponto -- <pasta do mapa> <world_id> <x> <z>...` (pares x z)
use pw_data_loader::watermap::MapaDeAgua;
use pw_data_loader::{MapaDeMovimento, Terreno};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&a[1]);
    let mundo: i32 = a[2].parse().unwrap();
    let agua = MapaDeAgua::ler(mundo, &dir);
    let terreno = Terreno::ler(mundo, &dir);
    let mov = MapaDeMovimento::ler(mundo, &dir);
    for par in a[3..].chunks(2) {
        let (x, z): (f32, f32) = (par[0].parse().unwrap(), par[1].parse().unwrap());
        println!("({x},{z}) água {:.1} terreno {:?} piso acima do terreno {:?}", agua.altura_em(x, z), terreno.altura_em(x, z), mov.acima_do_terreno(x, z));
    }
}
