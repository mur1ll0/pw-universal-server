//! O leitor genérico contra o `elements.data` real do realm 1.5.5 (cliente BR, v156).
//! O layout v159 (cliente EN) continua no catálogo, mas o arquivo EN saiu do projeto em
//! 2026-09-17 (B55) e não há mais teste contra ele.
//!
//! O leitor já recusa um arquivo que não termine exatamente no último byte, então
//! "carregou" quer dizer "o layout de cada uma das tabelas está certo". O que estes testes
//! acrescentam: as contagens de tabelas conferidas por conteúdo, e o conteúdo de algumas.
//!
//! História que vale guardar: até 2026-09-12 o leitor não conhecia os dois blocos de `tag`
//! que o `elementdataman::load_data` do cliente pula (depois de `ARMORRUNE_ESSENCE` e de
//! `WAR_TANKCALLIN_ESSENCE`), e dez remendos de `skip`/`count`/posição absoluta por arquivo
//! compensavam. O v156 do realm 1.5.5 (então `realm_155`), que é o que o docker serve, lia só 99 das 231
//! tabelas por causa disso — incluindo `MINE_ESSENCE` (o que cada recurso dá) e
//! `PLAYER_ACTION_INFO_CONFIG`.

use pw_data_loader::generic_elements::load_elements_data;
use std::path::PathBuf;

fn ler(realm: &str) -> Option<Vec<u8>> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(realm)
        .join("config/elements.data");
    match std::fs::read(&p) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("pulado: {} não existe", p.display());
            None
        }
    }
}

fn nome(r: &pw_data_loader::generic_elements::Record) -> String {
    r.get("Name").and_then(|v| v.as_text()).unwrap_or_default().to_string()
}

/// O v156 do `realm_155`: as 231 tabelas, com as que antes vinham vazias.
#[test]
fn o_v156_do_155_le_as_231_tabelas() {
    let Some(bytes) = ler("realm_155") else { return };
    let d = load_elements_data(&bytes).expect("v156 do 155 deve fechar no último byte");

    assert_eq!(d.version, 156);
    assert_eq!(d.tables.len(), 231);
    // O mesmo total que `specs/elements_layouts/pw_elements_reader.py` dá neste arquivo.
    assert_eq!(d.tables.values().map(|v| v.len()).sum::<usize>(), 69_640);
    let vazias = d.tables.values().filter(|v| v.is_empty()).count();
    assert!(vazias <= 5, "{vazias} tabelas vazias (eram 132)");

    for (tabela, n) in [
        ("EQUIPMENT_ADDON", 2977),
        ("WEAPON_ESSENCE", 2741),
        ("SKILLTOME_SUB_TYPE", 22),
        ("MONSTER_ESSENCE", 8054),
        ("NPC_ESSENCE", 4761),
        ("TALK_PROC", 3391),
        ("FACE_HAIR_ESSENCE", 430),
        ("CHARRACTER_CLASS_CONFIG", 12),
        ("PLAYER_ACTION_INFO_CONFIG", 1354),
        ("MINE_ESSENCE", 1557),
        ("FASHION_ESSENCE", 3016),
        ("PET_TYPE", 6),
        ("RED_PACKET_PAPER_ESSENCE", 5),
    ] {
        assert_eq!(d.get(tabela).len(), n, "{tabela}");
    }

    // O primeiro registro depois de cada bloco de tag, lido pelo nome.
    assert_eq!(nome(&d.get("SKILLTOME_SUB_TYPE")[0]), "Guerreiro");
    assert!(nome(&d.get("NPC_WAR_TOWERBUILD_SERVICE")[0]).contains("Construir torre"));
    assert_eq!(nome(&d.get("MINE_TYPE")[0]), "Tronco");
}

/// Um byte a mais no fim é erro, não uma tabela a mais lida torta.
#[test]
fn arquivo_que_nao_fecha_no_ultimo_byte_e_recusado() {
    let Some(mut bytes) = ler("realm_155") else { return };
    bytes.push(0);
    let e = load_elements_data(&bytes).unwrap_err().to_string();
    assert!(e.contains("terminaram no offset"), "{e}");
}

#[test]
fn o_v7_do_126_usa_o_catalogo_e_fecha_no_ultimo_byte() {
    let bytes = ler("realm_126").expect("elements.data do realm 126 precisa estar presente");
    assert_eq!(bytes.len(), 16_664_770);
    let dados = load_elements_data(&bytes).expect("v7 deve ser lido pelo catálogo genérico");
    assert_eq!(dados.version, 7);
    assert_eq!(dados.tables.len(), 119); // 118 tabelas fixas e TALK_PROC
    assert_eq!(dados.get("CHARRACTER_CLASS_CONFIG").len(), 8);
    assert!(!dados.get("MINE_ESSENCE").is_empty());
    assert!(!dados.get("PET_ESSENCE").is_empty());
    let item = |tabela: &str, id: i32| {
        dados.get(tabela).iter().find(|r| r.get("ID").and_then(|v| v.as_i32()) == Some(id)).unwrap()
    };
    assert_eq!(item("MEDICINE_ESSENCE", 1796)["id_major_type"].as_i32(), Some(1794));
    assert_eq!(item("MEDICINE_ESSENCE", 1796)["cool_time"].as_i32(), Some(15_000));
    assert_eq!(item("WEAPON_ESSENCE", 6)["price"].as_i32(), Some(120));
    assert_eq!(item("WEAPON_ESSENCE", 6)["shop_price"].as_i32(), Some(240));
    assert_eq!(item("WEAPON_ESSENCE", 6)["pile_num_max"].as_i32(), Some(1));
    assert_eq!(item("MONSTER_ESSENCE", 986)["aggressive_mode"].as_i32(), Some(1));
    assert_eq!(item("MONSTER_ESSENCE", 986)["common_strategy"].as_i32(), Some(60));
    assert_eq!(item("MONSTER_ESSENCE", 986)["drop_times"].as_i32(), Some(1));
    assert_eq!(item("MONSTER_ESSENCE", 986)["drop_matters_1_id"].as_i32(), Some(8612));
    assert_eq!(item("MONSTER_ESSENCE", 1005).get("fly_speed"), Some(&pw_data_loader::generic_elements::FieldValue::Float(3.0)));
    assert_eq!(item("MINE_ESSENCE", 6849)["npcgen_1_id_monster"].as_i32(), Some(3360));
    assert_eq!(item("MINE_ESSENCE", 6849)["npcgen_1_num"].as_i32(), Some(1));
    assert_eq!(item("FLYSWORD_ESSENCE", 2092)["character_combo_id"].as_i32(), Some(192));
    assert_eq!(item("AUTOHP_ESSENCE", 12812)["cool_time"].as_i32(), Some(10_000));
    assert_eq!(item("CHARRACTER_CLASS_CONFIG", 2)["character_class_id"].as_i32(), Some(0));
    assert_eq!(item("PET_ESSENCE", 8784).get("speed_a"), Some(&pw_data_loader::generic_elements::FieldValue::Float(8.0)));
    // Ordem e sizeof(T) do gs 1.2.6 (generate_v7.py, `ORDEM`): 23.447 registros.
    assert_eq!(dados.tables.values().map(Vec::len).sum::<usize>(), 23_447);
    assert_eq!(dados.get("SKILLMATTER_ESSENCE").len(), 78);
    assert_eq!(dados.get("REFINE_TICKET_ESSENCE").len(), 19);
    // NPC_TASK_OUT_SERVICE do v7 = ID + Name + id_tasks[32] (gs 1.2.6, VA 0x80ef014).
    let guia = item("NPC_TASK_OUT_SERVICE", 3531);
    assert_eq!(guia["id_tasks_1"].as_i32(), Some(1177));
    assert_eq!(guia["id_tasks_2"].as_i32(), Some(1178));
    assert!(guia.get("storage_id").is_none());
    // NPC_SKILL_SERVICE = id_skills[128] + id_dialog (VA 0x80ef202).
    assert!(item("NPC_SKILL_SERVICE", dados.get("NPC_SKILL_SERVICE")[0]["ID"].as_i32().unwrap()).get("id_skills_129").is_none());
    // ARMOR_ESSENCE sem `fixed_props`: armadura 139 com defesa e preço coerentes.
    assert_eq!(item("ARMOR_ESSENCE", 139)["defence_low"].as_i32(), Some(552));
    assert_eq!(item("ARMOR_ESSENCE", 139)["price"].as_i32(), Some(4800));
    assert_eq!(item("ARMOR_ESSENCE", 139)["shop_price"].as_i32(), Some(9600));
    assert_eq!(item("ARMOR_ESSENCE", 139)["repairfee"].as_i32(), Some(4800));
    let mut corrompido = bytes;
    corrompido.push(0);
    assert!(load_elements_data(&corrompido).is_err());
}

#[test]
fn o_manager_do_126_recebe_os_campos_do_v7() {
    let pasta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/realm_126/config");
    let mut dados = pw_data_loader::GameDataManager::new();
    let relatorio = dados.load_from_directory(&pasta);
    assert!(dados.elements_generic.is_some(), "{relatorio}");
    assert_eq!(dados.velocidade_da_montaria(8784, 1), Some(8.0));
    assert_eq!(dados.tipo_maior_do_remedio(1796), Some(1794));
    assert_eq!(dados.quanto_o_remedio_restaura_no_tempo(1796).map(|x| x.4), Some(15_000));
    assert!(dados.classes.classes.contains_key(&0));
    assert!(!dados.monstros.is_empty());
    assert!(!dados.precos.is_empty());
    assert_eq!(dados.precos.get(&6), Some(&(120, 240)));
    assert!(!dados.minas.is_empty());
    assert!(dados.minas.get(&8592).is_some_and(|m| m.monstros_ao_colher.iter().any(|x| x.0 == 8226)));
    assert_eq!(dados.dados_do_amuleto(12812).map(|x| x.2), Some(10_000));
    let voo = dados.conteudo_do_item_de_voo(2092).expect("voo 2092");
    assert_eq!(voo.len(), 30);
    assert_eq!(i32::from_le_bytes(voo[12..16].try_into().unwrap()), 192);
    // O Guia Selvagem (NPC 3518) entrega a missão inicial 1177 e a 1178.
    let guia = dados.servicos_de_npc.get(&3518).expect("NPC 3518");
    assert!(guia.missoes_entregues.contains(&1177) && guia.missoes_entregues.contains(&1178));
    assert_eq!(guia.deposito, 0);
    // PLAYER_SECONDLEVEL_CONFIG com 1.092 B: exp_lost[0] = 0,05 (antes 1,7e-41).
    assert_eq!(dados.progressao.perda_na_morte(0), 0.05);
    // O v7 carrega a tabela de habilidades do 1.2.6 (antes ficava vazia: 1.000 ms fixos).
    let enxame = dados.habilidades.get(299).expect("skill 299 no realm 126");
    assert_eq!((enxame.conjuracao_ms(1), enxame.fase_de_execucao_ms(1)), (Some(1_500), Some(1_000)));
}
