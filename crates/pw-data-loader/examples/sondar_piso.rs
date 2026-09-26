//! Terreno, piso do `movemap` e alcançabilidade num quadrado em volta de um ponto do mundo 1.
//! Uso: `--example sondar_piso -- <pasta do mapa> <x> <z> [raio]`
use pw_data_loader::{MapaDeMovimento, Terreno};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&a[1]);
    let (x, z): (f32, f32) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let r: i32 = a.get(4).and_then(|v| v.parse().ok()).unwrap_or(12);
    let t = Terreno::ler(1, &dir);
    let m = MapaDeMovimento::ler(1, &dir);
    println!("pixel {} m; linhas z de -{r} a +{r}, colunas x; cada célula: alcançável(A/.) piso-acima(m)", m.tamanho_do_pixel());
    for dz in (-r..=r).step_by(2) {
        let mut l = format!("{:+4} ", dz);
        for dx in (-r..=r).step_by(2) {
            let (px, pz) = (x + dx as f32, z + dz as f32);
            let (u, v) = m.pixel_de(px, pz);
            let ac = m.acima_no_pixel(u, v);
            let al = if m.alcancavel(u, v) { 'A' } else { '.' };
            l.push_str(&format!("{al}{:>3.0}", ac));
        }
        println!("{l}");
    }
    println!("terreno no centro: {:?}", t.altura_em(x, z));
}
