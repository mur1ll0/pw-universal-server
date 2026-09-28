//! Todo `airmap/N.octr` dos realms carrega e fecha no último byte (B133), e o mapa 1 tem
//! espaço bloqueado e livre.
use pw_data_loader::espaco::{MapaDoEspaco, Octree};
use std::path::PathBuf;

fn config(realm: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
    p.join("world/airmap/spmap.conf").exists().then_some(p)
}

#[test]
fn todos_os_octr_dos_realms_fecham_no_ultimo_byte() {
    for realm in ["realm_126", "realm_155"] {
        let Some(cfg) = config(realm) else {
            eprintln!("sem {realm}: pulado");
            continue;
        };
        let (mut lidos, mut recusados) = (0, Vec::new());
        for pasta in std::fs::read_dir(&cfg).unwrap().flatten() {
            let air = pasta.path().join("airmap");
            let Ok(arquivos) = std::fs::read_dir(&air) else { continue };
            for a in arquivos.flatten() {
                if a.path().extension().is_some_and(|e| e == "octr") {
                    match Octree::ler(&std::fs::read(a.path()).unwrap()) {
                        Some(_) => lidos += 1,
                        None => recusados.push(a.path()),
                    }
                }
            }
        }
        assert!(lidos > 0, "{realm}: nenhuma octree");
        assert!(recusados.is_empty(), "{realm}: {recusados:?}");
        eprintln!("{realm}: {lidos} octrees");
    }
}

#[test]
fn o_mapa_1_tem_espaco_livre_no_alto_e_bloqueado_em_algum_lugar() {
    for realm in ["realm_126", "realm_155"] {
        let Some(cfg) = config(realm) else { continue };
        let m = MapaDoEspaco::ler(1, &cfg.join("world"));
        assert!(m.tem_dados());
        assert_eq!(m.tamanho_do_voxel(), 2);
        // 900 m de altura: acima de tudo.
        assert!(m.livre(m.centro_do_voxel([0.0, 900.0, 0.0])).is_some(), "{realm}");
        // Alguma folha bloqueada perto do chão da região de começo (o mapa não é todo livre).
        let mut bloqueados = 0;
        for x in (-4000..4000).step_by(100) {
            for z in (-5500..5500).step_by(100) {
                for y in [220.0f32, 240.0, 260.0] {
                    if m.no(m.centro_do_voxel([x as f32, y, z as f32])).is_some_and(|n| n.estado != 0) {
                        bloqueados += 1;
                    }
                }
            }
        }
        assert!(bloqueados > 0, "{realm}: nenhuma folha bloqueada");
        eprintln!("{realm}: {bloqueados} amostras bloqueadas");
    }
}
