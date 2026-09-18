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
