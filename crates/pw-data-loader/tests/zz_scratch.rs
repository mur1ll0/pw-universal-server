use pw_data_loader::terreno::Terreno;
use std::path::PathBuf;
#[test]
fn scratch() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155BR/config/world");
    let t = Terreno::ler(1, &base);
    let pontos = [
        ("Blademaster/Wizard", 976.0f32, 219.2f32, 4187.3f32),
        ("Archer/Cleric", -741.5, 219.1, -1234.8),
        ("Barbarian/Venomancer", -1445.6, 219.3, 2642.0),
        ("Assassin/Psychomancer", 650.0, 130.0, 130.0),
        ("Seeker/Mystic", 380.0, 230.0, 230.0),
        ("Duskblade/Stormbringer", 150.0, 250.0, 250.0),
        ("HEal (banco)", -716.1949, 219.06999, -1210.188),
        ("testesacer (banco)", 363.41852, 227.97995, 2175.4927),
    ];
    for (nome, x, y, z) in pontos {
        match t.altura_em(x, z) {
            Some(h) => eprintln!("{nome:24} y={y:8.1} chao={h:8.1} dif={:+8.1}", y - h),
            None => eprintln!("{nome:24} y={y:8.1} FORA DO MAPA"),
        }
    }
}
