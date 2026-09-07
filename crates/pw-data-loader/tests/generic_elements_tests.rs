use pw_data_loader::generic_elements::{load_elements_data, load_overrides_for_version};
use std::path::Path;

/// Confere o leitor genérico contra o `elements.data` real do realm 155.
///
/// **Este arquivo mudou de build durante o desenvolvimento** (decisão do Murillo,
/// 2026-09-02): era v156 (do pacote `pwserver_155v156` da comunidade), mas o client 1.5.5
/// exige que o `elements.data`/`tasks.data` do servidor bata *exatamente* com os dele
/// (comparação de string, não numérica -- `EC_GameSession.cpp::OnPrtcChallenge`), então
/// `data/realm_155/config/elements.data` agora é uma cópia do `elements.data` do client
/// original (build **v159**). Ver `docs/ESTADO_E_RETOMADA.md`, seção "decodificar a build
/// v159", e `specs/elements_155/realm_155_v159_overrides.json` para a arqueologia completa.
///
/// **Cobertura conhecida, não 100%**: com os overrides atuais o leitor consome até o
/// offset 54.397.023 de 55.170.911 bytes (~98,6%) -- as tabelas 216
/// (`HOME_RESOURCE_PRODUCE_CONFIG`) em diante, até ~230 (sistema de "lar/mansão" e bilhetes
/// de loteria, features tardias e obscuras), ainda não foram resolvidas. Por isso este
/// teste confere só as tabelas já verificadas por conteúdo real, não um total agregado de
/// registros (que incluiria contagens erradas dessa cauda ainda não resolvida).
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
    let overrides = load_overrides_for_version(159).expect("overrides do v159 devem existir");
    let data = load_elements_data(&bytes, Some(&overrides))
        .expect("elements.data do realm 155 deveria carregar com os overrides conhecidos");

    assert_eq!(data.version, 159);
    assert_eq!(data.tables.len(), 234, "as 234 tabelas do catálogo v159 devem estar presentes");

    let class_configs = data.get("CHARRACTER_CLASS_CONFIG");
    assert_eq!(class_configs.len(), 12, "as 12 classes do jogo");

    let equipment_addon = data.get("EQUIPMENT_ADDON");
    assert_eq!(equipment_addon.len(), 2992);

    let talk_proc = data.get("TALK_PROC");
    assert_eq!(talk_proc.len(), 3391, "TALK_PROC tem tamanho variável, confirmado à parte");

    let pet_type = data.get("PET_TYPE");
    assert_eq!(pet_type.len(), 6, "os 6 tipos de pet, achados por sequência exata de IDs");

    let astrolabe_appearance = data.get("ASTROLABE_APPEARANCE_CONFIG");
    assert_eq!(astrolabe_appearance.len(), 1);

    let equip_make_hole = data.get("EQUIP_MAKE_HOLE_CONFIG");
    assert_eq!(equip_make_hole.len(), 1, "emenda cabeça-a-cauda com ASTROLABE_APPEARANCE_CONFIG");
}

/// Documenta uma limitação real, não um comportamento desejável: sem os overrides do
/// realm 155, o leitor **não** dá erro nas tabelas com quirks conhecidos -- a busca em
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
    let com_overrides = load_elements_data(&bytes, load_overrides_for_version(159).as_ref())
        .expect("com overrides, o leitor deve carregar");
    let sem_overrides = load_elements_data(&bytes, None)
        .expect("mesmo sem overrides, o leitor deve conseguir terminar (ainda que com dado errado nalgumas tabelas)");

    let total_com: usize = com_overrides.tables.values().map(|v| v.len()).sum();
    let total_sem: usize = sem_overrides.tables.values().map(|v| v.len()).sum();
    assert_ne!(
        total_com, total_sem,
        "sem overrides o total NÃO deveria bater com o resultado corrigido por acidente -- se bateu, os overrides pararam de ser necessários (bom sinal, mas confira antes de remover algum)"
    );
}
