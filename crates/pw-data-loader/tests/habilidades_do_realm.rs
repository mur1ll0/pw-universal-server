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
