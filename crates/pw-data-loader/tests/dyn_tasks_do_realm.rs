//! O cabeçalho do `dyn_tasks.data` dos realms 1.5.5, lido como o `UnmarshalDynTasks` do
//! original lê (`EvolvedPWServer/cgame/gs/task/TaskTemplMan.cpp:1915-1939`).
//!
//! Os números abaixo foram conferidos no arquivo: `pack_size` igual ao tamanho do arquivo
//! (12.979 bytes), `version` igual a `DYN_TASK_CUR_VERSION` (10), marca `0x52776c0d`
//! (2013-11-04) e 28 missões. Os dois realms 1.5.5 têm o mesmo pacote.

use pw_data_loader::{CabecalhoDasMissoesDinamicas, GameDataManager};
use std::path::PathBuf;

fn pasta(realm: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(realm)
        .join("config");
    if p.join("dyn_tasks.data").exists() {
        Some(p)
    } else {
        eprintln!("AVISO: {} não tem dyn_tasks.data — este teste NÃO verificou nada.", p.display());
        None
    }
}

#[test]
fn o_cabecalho_dos_dois_realms_155_fecha_com_o_arquivo() {
    for realm in ["realm_155BR", "realm_155"] {
        let Some(dir) = pasta(realm) else { continue };
        let dados = std::fs::read(dir.join("dyn_tasks.data")).unwrap();
        let c = CabecalhoDasMissoesDinamicas::ler(&dados)
            .unwrap_or_else(|e| panic!("{realm}: {e}"));
        assert_eq!(dados.len(), 12_979, "{realm}");
        assert_eq!(c.versao, 10, "{realm}");
        assert_eq!(c.marca, 0x5277_6c0d, "{realm}");
        assert_eq!(c.missoes, 28, "{realm}");
    }
}

#[test]
fn o_game_data_manager_guarda_a_marca_que_o_mundo_responde() {
    let Some(dir) = pasta("realm_155BR") else { return };
    let mut dm = GameDataManager::new();
    // Só a raiz interessa aqui; o relatório inteiro é conferido em outros testes.
    let _ = dm.load_from_directory(&dir);
    assert_eq!(dm.marca_das_missoes_dinamicas, Some(0x5277_6c0d));
}

#[test]
fn um_byte_a_mais_e_recusado_como_no_original() {
    let Some(dir) = pasta("realm_155BR") else { return };
    let mut dados = std::fs::read(dir.join("dyn_tasks.data")).unwrap();
    dados.push(0);
    assert!(CabecalhoDasMissoesDinamicas::ler(&dados).is_err());
}
