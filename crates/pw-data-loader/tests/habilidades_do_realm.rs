//! Os números das habilidades do servidor 1.5.5, contra os stubs do `cskill`.


/// B67 — a fase de execução da habilidade, que é o que o cliente anima depois da conjuração.
///
/// A sessão do original percorre os estados e só manda `stop_skill` no fim de todos
/// (`gs/actsession.cpp:466-600`). A Flecha Fulgurante (244) tem 3.000 ms de conjuração e
/// 800 ms de execução (`cskill/skills/skill244.h:20-80`, `GetExecutetime`).
#[test]
fn a_flecha_fulgurante_tem_conjuracao_e_execucao() {
    let t = pw_data_loader::habilidades::TabelaDeHabilidades::do_155();
    let h = t.get(244).expect("a 244 está na tabela do servidor");
    assert_eq!(h.conjuracao_ms(1), Some(3000), "State1::GetTime");
    assert_eq!(h.fase_de_execucao_ms(1), Some(800), "GetExecutetime / State2::GetTime");

    // Uma habilidade sem segunda fase devolve `None`, e aí o `stop_skill` sai na hora.
    let sem = t
        .por_id
        .values()
        .find(|x| x.conjuracao_ms(1).is_some() && x.fase_de_execucao_ms(1).is_none());
    assert!(sem.is_some(), "deveria haver habilidade sem fase de execução");
}

/// B68 — o chi: quanto cada golpe dá (`angro_increase` da classe) e quais missões concedem
/// o teto (`m_ulFuryULimit` do prêmio).
#[test]
fn o_chi_vem_da_classe_e_o_teto_vem_da_missao() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = pw_data_loader::GameDataManager::new();
    d.load_from_directory(&dir);

    // Arqueiro (classe 6): 5 de chi por golpe normal.
    let arqueiro = d.classes.get(6).expect("classe 6 no CHARRACTER_CLASS_CONFIG");
    assert_eq!(arqueiro.chi_por_golpe, 5, "angro_increase do Arqueiro");

    // A missão 32394 "Só um Pouco de Progresso" (nível 9) abre a barra com teto 99.
    let t = &d.tasks;
    let m = t.get_task(32394).expect("32394 no tasks.data");
    assert_eq!(m.rewards.teto_de_chi, 99, "m_ulFuryULimit da missão de nível 9");
    // E a progressão continua nas missões de cultivo.
    assert_eq!(t.get_task(922).map(|x| x.rewards.teto_de_chi), Some(199));
    assert_eq!(t.get_task(2804).map(|x| x.rewards.teto_de_chi), Some(399));
}
