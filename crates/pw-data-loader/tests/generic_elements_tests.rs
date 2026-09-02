use pw_data_loader::generic_elements::{load_elements_data, load_realm_155_overrides};
use std::path::Path;

/// Confere o leitor genérico (dirigido por `specs/elements_layouts/v156.json`) contra o
/// `elements.data` real do realm 155 -- deve bater com o resultado independente já obtido
/// em Python (`specs/elements_layouts/pw_elements_reader.py`): 231 tabelas, 69.626
/// registros no total, terminando exatamente no fim do arquivo.
#[test]
fn test_generic_elements_realm_155_if_present() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/realm_155/config/elements.data"
    ));
    if !path.exists() {
        return;
    }
    let bytes = std::fs::read(path).expect("Falha ao ler elements.data do realm 155");
    let overrides = load_realm_155_overrides();
    let data = load_elements_data(&bytes, Some(&overrides))
        .expect("elements.data do realm 155 deveria carregar por completo com os overrides conhecidos");

    assert_eq!(data.version, 156);
    assert_eq!(data.tables.len(), 231, "as 231 tabelas devem estar presentes");

    let total_records: usize = data.tables.values().map(|v| v.len()).sum();
    assert_eq!(
        total_records, 69_626,
        "total de registros deve bater com o já confirmado em Python"
    );

    let equipment_addon = data.get("EQUIPMENT_ADDON");
    assert_eq!(equipment_addon.len(), 2977);

    let talk_proc = data.get("TALK_PROC");
    assert_eq!(talk_proc.len(), 3391, "TALK_PROC tem tamanho variável, confirmado à parte");

    let red_packet = data.get("RED_PACKET_PAPER_ESSENCE");
    assert_eq!(red_packet.len(), 5, "última tabela do arquivo");
}

/// Documenta uma limitação real, não um comportamento desejável: sem os overrides do
/// realm 155, o leitor **não** dá erro nas ~10 tabelas com quirks conhecidos -- a busca em
/// janela às vezes acha um alinhamento *diferente*, plausível o bastante pra passar a
/// pontuação, mas errado (o mesmo risco de falso positivo documentado em
/// `specs/elements_155/README.md`, seção "Achado de metodologia"). Por isso os overrides
/// não são opcionais na prática pra este arquivo específico -- são só opcionais na API
/// (`Option<&RealmOverrides>`) pra realms que ainda não passaram por essa investigação.
#[test]
fn test_generic_elements_sem_overrides_da_resultado_diferente_nao_erro() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/realm_155/config/elements.data"
    ));
    if !path.exists() {
        return;
    }
    let bytes = std::fs::read(path).expect("Falha ao ler elements.data do realm 155");
    let data = load_elements_data(&bytes, None)
        .expect("mesmo sem overrides, o leitor deve conseguir terminar (ainda que com dado errado nalgumas tabelas)");

    let total_records: usize = data.tables.values().map(|v| v.len()).sum();
    assert_ne!(
        total_records, 69_626,
        "sem overrides o total NÃO deveria bater por acidente -- se bateu, os overrides pararam de ser necessários (bom sinal, mas confira antes de remover algum)"
    );
}
