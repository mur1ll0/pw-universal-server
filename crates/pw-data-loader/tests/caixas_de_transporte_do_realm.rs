//! B208: todo `region.sev` dos dois realms se lê até o último byte, com as caixas de transporte.
use pw_data_loader::regioes::ler_caixas_de_transporte;

#[test]
fn todo_region_sev_dos_realms_se_le_inteiro() {
    for realm in ["realm_126", "realm_155"] {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config"));
        let Ok(dir) = std::fs::read_dir(&base) else {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", base.display());
            continue;
        };
        let (mut lidos, mut caixas, mut falhas) = (0, 0, Vec::new());
        for e in dir.filter_map(|e| e.ok()) {
            // `empty/` é a pasta de mapa vazio, com `region.sev` de versão antiga (< 4), que o
            // leitor do carimbo já recusava.
            if e.file_name() == "empty" {
                continue;
            }
            let p = e.path().join("region.sev");
            let Ok(b) = std::fs::read(&p) else { continue };
            match ler_caixas_de_transporte(&b) {
                Ok(c) => { lidos += 1; caixas += c.len(); }
                Err(err) => falhas.push(format!("{}: {err}", p.display())),
            }
        }
        println!("{realm}: {lidos} region.sev, {caixas} caixas de transporte");
        assert!(falhas.is_empty(), "{realm}: {falhas:?}");
    }
}
