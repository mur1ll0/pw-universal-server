//! A string `edition` montada de ponta a ponta, a partir dos arquivos de uma pasta de realm.
//!
//! # O incidente que este teste fecha
//!
//! O cliente 1.5.3 (build 2552) recusou o login e escreveu no `EC.log`:
//!
//! ```text
//! local ver: 300000917c571db3f456986c25
//! server ver: 3000007f7900
//! ```
//!
//! Três defeitos independentes numa string só:
//!
//! 1. os dois timestamps saíam **zero** porque o `elements.data` de 51 MB falhava no parser
//!    e o `?` daquela leitura abortava toda a carga seguinte, inclusive a dos `gshop`;
//! 2. o `ELEMENTDATA_VERSION` era uma constante de compilação nossa (`0x3000007f`), tirada
//!    de uma árvore de fontes que é de **outro build** do cliente;
//! 3. o `_task_templ_cur_version` idem (121 em vez de 124).
//!
//! Cada um deles tem teste de unidade no seu módulo. Este aqui é o que amarra os três: uma
//! pasta com os mesmos cabeçalhos dos arquivos reais do realm tem que produzir exatamente a
//! string que o cliente disse esperar.
//!
//! Os quatro valores são os medidos nos arquivos do realm do Murillo:
//!
//! ```text
//! elements.data   30000091
//! tasks.data      93858361 0000007c        (0x7c = 124)
//! gshopsev.data   571db3f4
//! gshopsev1.data  56986c25
//! ```

use pw_data_loader::GameDataManager;
use pw_protocol::{Edition, GameVersion, VersaoDoCliente};

/// Cabeçalho de `elements.data`: `[u32 versão][u32 time_t]` (`elementdataman.cpp:3611`).
fn escrever_elements(dir: &std::path::Path, versao: u32, quebrado: bool) {
    let mut bytes = versao.to_le_bytes().to_vec();
    bytes.extend_from_slice(&0x5697_3c4Eu32.to_le_bytes());
    if !quebrado {
        // Um corpo qualquer; o parser das 118 tabelas não é o assunto aqui.
        bytes.extend_from_slice(&[0u8; 512]);
    }
    std::fs::write(dir.join("elements.data"), bytes).unwrap();
}

/// Cabeçalho de `tasks.data`: `[magic][versão][item_count]` (`Task/TaskTempl.h:224`).
fn escrever_tasks(dir: &std::path::Path, versao: u32) {
    let mut bytes = 0x9385_8361u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&versao.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    std::fs::write(dir.join("tasks.data"), bytes).unwrap();
}

/// `gshop`: `[u32 timestamp][u32 contagem]`.
fn escrever_gshop(dir: &std::path::Path, nome: &str, timestamp: u32) {
    let mut bytes = timestamp.to_le_bytes().to_vec();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    std::fs::write(dir.join(nome), bytes).unwrap();
}

fn pasta(marca: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pw_edition_{marca}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Monta o `edition` como o `LinkGateway` monta: carrega a pasta, resolve as constantes do
/// cliente, escreve a string.
fn edition_da_pasta(dir: &std::path::Path, version: GameVersion) -> String {
    let mut m = GameDataManager::new();
    let _rel = m.load_from_directory(dir);
    let cliente =
        VersaoDoCliente::resolver(version, m.versao_do_elements, m.versao_das_tasks, |_| None)
            .unwrap();
    let gshop3 = version
        .challenge_edition_tem_terceiro_gshop()
        .then_some(m.gshop3.timestamp);
    let e = Edition::com_versao_do_cliente(cliente, m.gshop.timestamp, m.gshop2.timestamp, gshop3);
    String::from_utf8(e.to_wire()).unwrap()
}

#[test]
fn a_pasta_do_realm_153_reproduz_a_string_do_cliente() {
    let dir = pasta("realm153");
    escrever_elements(&dir, 0x3000_0091, false);
    escrever_tasks(&dir, 124);
    escrever_gshop(&dir, "gshopsev.data", 0x571d_b3f4);
    escrever_gshop(&dir, "gshopsev1.data", 0x5698_6c25);

    assert_eq!(
        edition_da_pasta(&dir, GameVersion::V1_5_3),
        "300000917c571db3f456986c25"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn um_elements_data_ilegivel_nao_muda_a_string() {
    // O defeito nº 1, encenado: o `elements.data` do 1.5.3 real tem 51 MB e o nosso parser
    // não o percorre até o fim. Isso não pode ter efeito nenhum sobre o `edition` — o
    // cabeçalho de 8 bytes é tudo de que ele precisa, e os outros arquivos não têm nada com
    // isso.
    let dir = pasta("elements_ilegivel");
    escrever_elements(&dir, 0x3000_0091, true); // só o cabeçalho, sem corpo
    escrever_tasks(&dir, 124);
    escrever_gshop(&dir, "gshopsev.data", 0x571d_b3f4);
    escrever_gshop(&dir, "gshopsev1.data", 0x5698_6c25);

    let mut m = GameDataManager::new();
    let rel = m.load_from_directory(&dir);
    assert!(!rel.sem_falhas(), "o `elements.data` truncado devia ter sido relatado");

    assert_eq!(
        edition_da_pasta(&dir, GameVersion::V1_5_3),
        "300000917c571db3f456986c25"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_string_errada_do_incidente_nao_volta() {
    // `3000007f7900` era o que o servidor mandava: as duas constantes da árvore de fontes e
    // os dois timestamps zerados. Uma asserção negativa não prova grande coisa sozinha —
    // ela está aqui porque é a string exata que o cliente registrou recusando, e serve de
    // marco: se alguém reintroduzir qualquer um dos três defeitos, é para cá que volta.
    let dir = pasta("regressao");
    escrever_elements(&dir, 0x3000_0091, false);
    escrever_tasks(&dir, 124);
    escrever_gshop(&dir, "gshopsev.data", 0x571d_b3f4);
    escrever_gshop(&dir, "gshopsev1.data", 0x5698_6c25);

    assert_ne!(edition_da_pasta(&dir, GameVersion::V1_5_3), "3000007f7900");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pasta_do_realm_155_ganha_o_quinto_valor_do_terceiro_gshop() {
    // Achado do pivô 1.5.3→1.5.5 (2026-09-02): as constantes de compilação do
    // EvolvedPWClient (`ELEMENTDATA_VERSION=0x30000091`, `_task_templ_cur_version=125`)
    // e um terceiro `gshopsev2.data`, que só entra na string porque a versão é 1.5.5
    // (`GameVersion::challenge_edition_tem_terceiro_gshop`) — ver
    // docs/ESTADO_E_RETOMADA.md.
    let dir = pasta("realm155");
    escrever_elements(&dir, 0x3000_0091, false);
    escrever_tasks(&dir, 125);
    escrever_gshop(&dir, "gshopsev.data", 0x1111_1111);
    escrever_gshop(&dir, "gshopsev1.data", 0x2222_2222);
    escrever_gshop(&dir, "gshopsev2.data", 0x3333_3333);

    assert_eq!(
        edition_da_pasta(&dir, GameVersion::V1_5_5),
        "300000917d111111112222222233333333"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pasta_do_realm_126_tem_os_seus_proprios_numeros() {
    // O 1.2.6 não manda `edition` no `Challenge`, mas os cabeçalhos dele existem e são
    // outros — e é o que garante que os números vêm mesmo dos arquivos, e não de uma
    // constante que por acaso bate com o 1.5.3.
    let dir = pasta("realm126");
    escrever_elements(&dir, 0x3000_0007, false);
    escrever_tasks(&dir, 55);
    escrever_gshop(&dir, "gshop.data", 0x47e8_b6ff);

    let mut m = GameDataManager::new();
    let _ = m.load_from_directory(&dir);
    assert_eq!(m.versao_do_elements, Some(0x3000_0007));
    assert_eq!(m.versao_das_tasks, Some(55));

    let cliente = VersaoDoCliente::resolver(
        GameVersion::V1_2_6,
        m.versao_do_elements,
        m.versao_das_tasks,
        |_| None,
    )
    .unwrap();
    assert_eq!(cliente.elements_data, 0x3000_0007);
    assert_eq!(cliente.task_templ, 55);
    let _ = std::fs::remove_dir_all(&dir);
}
