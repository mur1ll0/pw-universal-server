//! O `tasks.data` do realm 1.5.5, lido inteiro.
//!
//! O leitor já recusa o arquivo se qualquer missão de topo não terminar no deslocamento
//! que a tabela do cabeçalho promete — então "carregou" aqui quer dizer "o layout está
//! certo byte a byte". O que este arquivo acrescenta é a prova de que os **valores** saem
//! certos, conferidos com o que se vê em jogo.

use pw_data_loader::TasksData;
use std::path::PathBuf;

fn ler(realm: &str) -> Option<TasksData> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(realm)
        .join("config/tasks.data");
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    Some(TasksData::load_from_bytes(&bytes).expect("tasks.data v129 devia fechar inteiro"))
}

/// Todas as missões de topo, e as submissões delas, do realm 1.5.5 (cliente BR).
#[test]
fn a_versao_129_fecha_inteira() {
    for (realm, topo, total) in [("realm_155", 14_885, 31_837)] {
        let Some(t) = ler(realm) else { continue };
        assert_eq!(t.version, 129, "{realm}");
        assert_eq!(t.de_topo.len(), topo, "{realm}: missões de topo");
        assert_eq!(t.tasks.len(), total, "{realm}: com as submissões");
    }
}

/// `elementclient.exe` v126, LoadBinary 0x62f6c0: 534 B fixos e seções variáveis;
/// `docs/RESULTADO_TASKS_V55.md` confirma 2.819 raízes e 7.994 tarefas.
#[test]
fn a_versao_55_fecha_inteira_e_expoe_a_missao_inicial() {
    let t = ler("realm_126").expect("tasks.data v55 do realm 126 precisa estar presente");
    assert_eq!(t.version, 55);
    assert_eq!(t.de_topo.len(), 2_819);
    assert_eq!(t.tasks.len(), 7_994);
    let inicial = t.get_task(1173).expect("Primeiro Teste, missão inicial");
    assert!(inicial.name.contains("Primeiro Teste"), "{}", inicial.name);
    assert_eq!(inicial.npc_que_entrega, 3517);
    assert_eq!(inicial.req_classes, vec![0]);
    assert_eq!((inicial.rewards.money, inicial.rewards.exp, inicial.rewards.sp), (45, 75, 20));
    assert_eq!(inicial.rewards.nova_missao, 1174);
    assert_eq!(inicial.metodo, 0);
    assert_eq!(t.get_task(1174).expect("A Cidade das Espadas").metodo, 1);
}

#[test]
fn a_versao_55_recusa_corrupcao_no_fim_e_na_tabela() {
    let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/realm_126/config/tasks.data");
    let mut bytes = std::fs::read(caminho).expect("tasks.data v55 do realm 126");
    bytes.push(0);
    assert!(TasksData::load_from_bytes(&bytes).is_err(), "byte extra no fim");
    bytes.pop();
    bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    assert!(TasksData::load_from_bytes(&bytes).is_err(), "offset fora dos dados");
}

/// A primeira missão do Guerreiro: "Exposição de Talento" (1173), do NPC 3517, com duas
/// submissões de caça. O nome e a descrição saem legíveis — o XOR pelo id está certo — e
/// o monstro, a quantidade e o prêmio são os de jogo: 10 Insetos de Jade (o monstro 16,
/// o mesmo "Inseto Esmeralda" do teste de combate de 2026-09-12).
#[test]
fn a_missao_inicial_do_guerreiro() {
    let Some(t) = ler("realm_155") else { return };
    let m = t.get_task(1173).expect("missão 1173");
    assert!(m.name.contains("Exposição de Talento"), "{:?}", m.name);
    assert!(m.descricao.starts_with("Você será designado guarda"), "{:?}", m.descricao);
    assert_eq!((m.min_level, m.max_level), (1, 20));
    assert_eq!(m.req_classes, vec![0]);
    assert_eq!(m.npc_que_entrega, 3517);
    assert_eq!(m.sub_tasks, vec![1175, 1176]);
    assert_eq!((m.rewards.exp, m.rewards.sp, m.rewards.money), (75, 20, 90));

    let caca = t.get_task(1175).expect("submissão 1175");
    assert_eq!(caca.parent, Some(1173));
    assert!(caca.name.contains("Matar Insetos de Jade"), "{:?}", caca.name);
    assert_eq!(caca.monster_kills.len(), 1);
    assert_eq!((caca.monster_kills[0].monstro, caca.monster_kills[0].quantidade), (16, 10));
    assert_eq!((caca.npc_que_entrega, caca.npc_que_premia), (3517, 3517));
    let itens: Vec<_> = caca.rewards.grupos_de_itens.iter().flat_map(|g| &g.itens).map(|i| (i.id, i.quantidade)).collect();
    assert_eq!(itens, vec![(8617, 5)]);
}

/// B102 — as flags de filhas e de desistência do bloco fixo v55 (0x6a..0x74, `libtask.so`
/// 1.2.6). A 1177 (Guia Selvagem) corre as filhas em ordem: primeiro a 1178 (caça), depois a
/// 1179 — como o servidor original entrega na captura.
#[test]
fn a_versao_55_le_as_flags_de_filhas() {
    let t = ler("realm_126").expect("tasks.data v55 do realm 126");
    let m = t.get_task(1177).expect("1177");
    assert!(m.filhos_em_ordem, "1177 devia correr as filhas em ordem");
    assert!(!m.escolhe_um_filho && !m.sorteia_um_filho);
    assert_eq!(m.sub_tasks, vec![1178, 1179]);
    // A 1173 ("Primeiro Teste") deixa escolher uma das filhas: 1175 ou 1176.
    let p = t.get_task(1173).expect("1173");
    assert!(p.escolhe_um_filho && !p.filhos_em_ordem);
    assert_eq!(p.sub_tasks, vec![1175, 1176]);
    let todas: Vec<_> = t.tasks.values().collect();
    let conta = |f: fn(&pw_data_loader::tasks::TaskTemplate) -> bool| todas.iter().filter(|x| f(x)).count();
    eprintln!(
        "v55: {} tarefas; em ordem {}, escolhe {}, sorteia {}, pai falha {}, pai sucesso {}, desiste {}, repete {}, refaz {}, limpa {}, registro {}, morre {}",
        todas.len(),
        conta(|x| x.filhos_em_ordem), conta(|x| x.escolhe_um_filho), conta(|x| x.sorteia_um_filho),
        conta(|x| x.pai_tambem_falha), conta(|x| x.pai_tambem_sucesso), conta(|x| x.pode_desistir),
        conta(|x| x.pode_repetir), conta(|x| x.refazer_apos_falha), conta(|x| x.limpa_ao_desistir),
        conta(|x| x.precisa_registro), conta(|x| x.falha_ao_morrer),
    );
    // Os padrões do editor (`vazia()`) aparecem ligados na quase totalidade: pai também falha
    // 7.993 de 7.994, pode desistir 7.254, refazer após falha 7.769.
    assert!(conta(|x| x.pai_tambem_falha) > 7900 && conta(|x| x.refazer_apos_falha) > 7700);
}
