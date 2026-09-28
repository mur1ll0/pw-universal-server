//! Todo `path.sev` dos dois realms fecha no último byte pelo leitor do `path_manager`, e a rota
//! do Carniçal Sanguinário (o chefe andarilho do mapa 1 do 1.2.6) existe.
use pw_data_loader::rotas::Rotas;
use std::path::PathBuf;

fn config(realm: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config"))
}

#[test]
fn todo_path_sev_fecha_no_ultimo_byte() {
    let mut total = (0usize, 0usize);
    for realm in ["realm_126", "realm_155"] {
        let dir = config(realm);
        let Ok(pastas) = std::fs::read_dir(&dir) else {
            eprintln!("pulado: sem {realm}");
            continue;
        };
        for e in pastas.flatten() {
            let arq = e.path().join("path.sev");
            let Ok(b) = std::fs::read(&arq) else { continue };
            let r = Rotas::ler(&b).unwrap_or_else(|e| panic!("{}: {e}", arq.display()));
            // Rota de menos de dois pontos o `base_patrol_agent::Init` recusa; aqui só conta.
            total.0 += 1;
            total.1 += r.len();
        }
    }
    eprintln!("rotas: {} arquivos, {} rotas", total.0, total.1);
    assert!(total.0 > 0);
}

#[test]
fn a_rota_do_carnical_sanguinario_existe() {
    let arq = config("realm_126").join("world/path.sev");
    let Ok(b) = std::fs::read(&arq) else {
        eprintln!("pulado: sem {}", arq.display());
        return;
    };
    let r = Rotas::ler(&b).unwrap();
    let p = r.rota(486539715).expect("rota 486539715");
    eprintln!("rota 486539715: {} pontos, de {:?} a {:?}", p.len(), p[0], p[p.len() - 1]);
    assert!(p.len() >= 2);
    // Perto da área do chefe (1506, 316, 2302 no npcgen).
    let perto = p.iter().any(|v| (v[0] - 1506.0).abs() < 300.0 && (v[2] - 2302.0).abs() < 300.0);
    assert!(perto, "a rota não passa perto da área do chefe");
}
