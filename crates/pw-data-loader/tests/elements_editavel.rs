//! Escritor do `elements.data` (B237) com os arquivos reais dos dois realms: ida e volta byte a
//! byte, edição de campo relida pelo leitor genérico, clone com id do espaço e recusas.

use pw_data_loader::elements_editavel::{espaco_de_id, ElementsEditavel, ErroDeEdicao};
use pw_data_loader::generic_elements::{load_elements_data, FieldValue};
use std::path::PathBuf;

fn arquivo(realm: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/elements.data"));
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e} (os dados do realm são obrigatórios)", p.display()))
}

#[test]
fn ida_e_volta_byte_a_byte_nos_dois_realms() {
    for (realm, versao, tabelas) in [("realm_126", 7, 118), ("realm_155", 156, 230)] {
        let buf = arquivo(realm);
        let e = ElementsEditavel::ler(&buf, None).unwrap();
        assert_eq!(e.versao, versao);
        assert_eq!(e.tabelas.len(), tabelas, "{realm}: tabelas fixas (o TALK_PROC vai cru)");
        assert!(e.gravar() == buf, "{realm}: gravar sem editar tem de devolver o arquivo idêntico");
    }
}

#[test]
fn editar_campo_clonar_e_remover_no_126() {
    let buf = arquivo("realm_126");
    let mut e = ElementsEditavel::ler(&buf, None).unwrap();
    let armas = e.tabela("WEAPON_ESSENCE").unwrap();
    let (n, id0) = (armas.registros.len(), armas.id(0).unwrap());

    // Campo de texto e numérico.
    e.definir_campo("WEAPON_ESSENCE", 0, "Name", &FieldValue::Text("Espada de Teste".into())).unwrap();
    e.definir_campo("WEAPON_ESSENCE", 0, "price", &FieldValue::Int(12345)).unwrap();
    // Tipo errado e texto que não cabe são recusados sem mexer no registro.
    assert!(e.definir_campo("WEAPON_ESSENCE", 0, "price", &FieldValue::Text("x".into())).is_err());
    assert!(e.definir_campo("WEAPON_ESSENCE", 0, "Name", &FieldValue::Text("x".repeat(500))).is_err());

    // Id: o próximo do espaço ESSENCE; repetido é recusado (o cliente sobrescreveria o outro).
    assert_eq!(espaco_de_id("WEAPON_ESSENCE").as_deref(), Some("ESSENCE"));
    let novo = e.proximo_id("WEAPON_ESSENCE").unwrap();
    let clone = e.clonar("WEAPON_ESSENCE", 0, novo).unwrap();
    assert_eq!(clone, n);
    let monstro = e.tabela("MONSTER_ESSENCE").unwrap().id(0).unwrap();
    assert!(matches!(e.clonar("WEAPON_ESSENCE", 0, monstro), Err(ErroDeEdicao::IdRepetido { .. })),
        "o id de um monstro é do mesmo espaço");
    assert!(matches!(e.definir_campo("WEAPON_ESSENCE", 1, "ID", &FieldValue::Int(id0)), Err(ErroDeEdicao::IdRepetido { .. })));

    // Relido pelo leitor genérico: o arquivo fecha no último byte e mostra as mudanças.
    let gravado = e.gravar();
    assert_eq!(gravado.len(), buf.len() + e.tabela("WEAPON_ESSENCE").unwrap().tamanho);
    let lido = load_elements_data(&gravado).unwrap();
    let armas = lido.get("WEAPON_ESSENCE");
    assert_eq!(armas.len(), n + 1);
    assert_eq!(armas[0]["Name"], FieldValue::Text("Espada de Teste".into()));
    assert_eq!(armas[0]["price"], FieldValue::Int(12345));
    assert_eq!(armas[n]["ID"], FieldValue::Int(novo));
    assert_eq!(armas[n]["Name"], FieldValue::Text("Espada de Teste".into()));
    // O resto do arquivo não mudou: mesma contagem em todas as outras tabelas.
    let original = load_elements_data(&buf).unwrap();
    for (nome, regs) in &original.tables {
        if nome != "WEAPON_ESSENCE" {
            assert_eq!(lido.get(nome).len(), regs.len(), "{nome}");
        }
    }

    // Remover o clone volta ao tamanho do arquivo original.
    e.remover("WEAPON_ESSENCE", clone).unwrap();
    assert_eq!(e.gravar().len(), buf.len());
}

#[test]
fn tabelas_sem_espaco_conhecido_nao_aceitam_registro_novo() {
    let buf = arquivo("realm_155");
    let mut e = ElementsEditavel::ler(&buf, None).unwrap();
    assert!(espaco_de_id("HOME_CONFIG").is_none());
    if !e.tabela("HOME_CONFIG").unwrap().registros.is_empty() {
        assert!(matches!(e.clonar("HOME_CONFIG", 0, 999_999), Err(ErroDeEdicao::EspacoDesconhecido(_))));
    }
    assert!(matches!(e.proximo_id("HOME_CONFIG"), Err(ErroDeEdicao::EspacoDesconhecido(_))));
}

#[test]
fn conversas_ida_e_volta_e_edicao() {
    use pw_data_loader::conversas::{self, BIT_DE_FUNCAO};
    for realm in ["realm_126", "realm_155"] {
        let buf = arquivo(realm);
        let mut e = ElementsEditavel::ler(&buf, None).unwrap();
        let n = e.conversas.len();
        assert!(n > 100, "{realm}: {n} conversas");
        let janelas: Vec<_> = e.conversas.iter().flat_map(|c| &c.janelas).collect();
        let com_nul = janelas.iter().filter(|j| j.texto.last() == Some(&0)).count();
        let vazias = janelas.iter().filter(|j| j.texto.is_empty()).count();
        // Evidência do formato: o talk_text_len conta o NUL final.
        assert_eq!(com_nul + vazias, janelas.len(), "{realm}: janelas sem NUL final");
        let funcoes = janelas.iter().flat_map(|j| &j.opcoes).filter(|o| o.id & BIT_DE_FUNCAO != 0).count();
        assert!(funcoes > 0);
        assert!(e.gravar() == buf, "{realm}: conversas regravadas idênticas");

        // Editar o texto de uma janela muda o tamanho e o arquivo continua fechando.
        e.conversas[0].janelas[0].texto = conversas::texto_variavel("Olá, viajante!").unwrap();
        e.conversas[0].texto = conversas::texto_fixo("Conversa de teste").unwrap();
        let gravado = e.gravar();
        let relido = ElementsEditavel::ler(&gravado, None).unwrap();
        assert_eq!(conversas::texto(&relido.conversas[0].janelas[0].texto), "Olá, viajante!");
        assert_eq!(conversas::texto(&relido.conversas[0].texto), "Conversa de teste");
        assert_eq!(relido.conversas.len(), n);
        assert!(load_elements_data(&gravado).is_ok(), "o leitor genérico também fecha no último byte");
        assert!(conversas::texto_fixo(&"x".repeat(64)).is_err());
    }
}

#[test]
fn npcgen_de_todos_os_mapas_volta_identico() {
    use pw_data_loader::npcgen_editavel::{definir_campo, ler_campos, NpcgenEditavel, CAMPOS_DA_AREA, CAMPOS_DO_GERADOR};
    let mut por_versao = std::collections::BTreeMap::new();
    let mut total = 0;
    for realm in ["realm_126", "realm_155"] {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config"));
        let mut arquivos: Vec<PathBuf> = std::fs::read_dir(&raiz)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().join("npcgen.data"))
            .filter(|p| p.exists())
            .collect();
        if raiz.join("npcgen.data").exists() {
            arquivos.push(raiz.join("npcgen.data"));
        }
        assert!(arquivos.len() > 30, "{realm}: {} npcgen", arquivos.len());
        for p in arquivos {
            let buf = std::fs::read(&p).unwrap();
            let n = NpcgenEditavel::ler(&buf).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert!(n.gravar() == buf, "{} não volta idêntico", p.display());
            *por_versao.entry(n.versao).or_insert(0) += 1;
            total += 1;
        }
    }
    println!("npcgen por versão: {por_versao:?}, total {total}");
    // Editar uma área do mundo do 1.5.5: posição, quantidade; acrescentar e tirar gerador mantém o arquivo coerente.
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config/world/npcgen.data");
    let buf = std::fs::read(&p).unwrap();
    let mut n = NpcgenEditavel::ler(&buf).unwrap();
    let v = n.versao;
    let a = &mut n.areas[0];
    definir_campo(&mut a.bruto, CAMPOS_DA_AREA, v, "vPos_x", &serde_json::json!(123.5), "area").unwrap();
    definir_campo(&mut a.geradores[0], CAMPOS_DO_GERADOR, v, "dwNum", &serde_json::json!(7), "gerador").unwrap();
    assert!(definir_campo(&mut a.bruto, CAMPOS_DA_AREA, v, "inexistente", &serde_json::json!(1), "area").is_err());
    assert!(definir_campo(&mut a.geradores[0], CAMPOS_DO_GERADOR, v, "dwNum", &serde_json::json!(-1), "gerador").is_err());
    let copia = a.geradores[0].clone();
    a.geradores.push(copia);
    let relido = NpcgenEditavel::ler(&n.gravar()).unwrap();
    assert_eq!(ler_campos(&relido.areas[0].bruto, CAMPOS_DA_AREA, v)["vPos_x"], 123.5);
    assert_eq!(ler_campos(&relido.areas[0].geradores[0], CAMPOS_DO_GERADOR, v)["dwNum"], 7);
    assert_eq!(relido.areas[0].geradores.len(), n.areas[0].geradores.len());
    assert!(pw_data_loader::npcgen::NpcGenData::load_from_bytes(&n.gravar()).is_ok(), "o leitor do GS também lê");
}

/// B244: todo `precinct.sev` e `region.sev` dos dois realms relido e regravado byte a byte.
#[test]
fn precinct_e_region_de_todos_os_mapas_voltam_identicos() {
    use pw_data_loader::precinct::Distritos;
    use pw_data_loader::regioes::RegioesEditavel;
    let (mut np, mut nr, mut caixas) = (0, 0, 0);
    for realm in ["realm_126", "realm_155"] {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config"));
        let mut pastas: Vec<PathBuf> = std::fs::read_dir(&raiz).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| {
            // Só as pastas que o GS carrega (`manager.rs`: `world`, `aNN`, `bNN`); a `empty` do 1.5.5 tem arquivos de 8 bytes.
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            p.is_dir() && (n == "world" || (n.len() == 3 && (n.starts_with('a') || n.starts_with('b')) && n[1..].bytes().all(|c| c.is_ascii_digit())))
        }).collect();
        pastas.push(raiz.clone());
        for d in pastas {
            if let Ok(buf) = std::fs::read(d.join("precinct.sev")) {
                let p = Distritos::ler(&buf).unwrap_or_else(|e| panic!("{}: {e}", d.display()));
                assert!(p.gravar() == buf, "{}/precinct.sev não volta idêntico", d.display());
                np += 1;
            }
            if let Ok(buf) = std::fs::read(d.join("region.sev")) {
                let mut r = RegioesEditavel::ler(&buf).unwrap_or_else(|e| panic!("{}: {e}", d.display()));
                assert!(r.gravar() == buf, "{}/region.sev não volta idêntico", d.display());
                let k = r.caixas_mut().len();
                assert_eq!(k, pw_data_loader::regioes::ler_caixas_de_transporte(&buf).unwrap().len());
                caixas += k;
                nr += 1;
            }
        }
    }
    println!("precinct.sev: {np}, region.sev: {nr}, caixas: {caixas}");
    assert!(np > 50 && nr > 50, "{np} {nr}");
}
