//! O motor de missões contra o `tasks.data` real do `realm_155`.
//!
//! O cenário é o do teste em jogo de 2026-09-14: um Arqueiro novo fala com o guia dos Alados
//! (NPC 44698), aceita "Escolhido do Chi: Elfo Alado" (32201, falar com NPC, conclui no NPC
//! 44390) e entrega. Sem a pasta do realm o teste avisa e não verifica nada.

use pw_data_loader::tasks::TasksData;
use pw_gs::missoes::{self, Jogador, ListasDeMissao, Motor};
use std::collections::HashMap;

#[derive(Default)]
struct Arqueiro {
    itens: HashMap<(u32, bool), u32>,
    dinheiro: u32,
    exp: u32,
    sp: u32,
    avisos: Vec<Vec<u8>>,
}

impl Jogador for Arqueiro {
    fn agora(&self) -> u32 { 1_757_900_000 }
    fn nivel(&self) -> u32 { 1 }
    fn classe(&self) -> u32 { 6 }
    fn masculino(&self) -> bool { true }
    fn cultivo(&self) -> u32 { 0 }
    fn reputacao(&self) -> i32 { 0 }
    fn dinheiro(&self) -> u32 { self.dinheiro }
    fn e_gm(&self) -> bool { false }
    fn contar(&self, tid: u32, comum: bool) -> u32 { self.itens.get(&(tid, comum)).copied().unwrap_or(0) }
    fn slots_livres(&self, _: bool) -> u32 { 32 }
    fn dar_item(&mut self, tid: u32, q: u32, comum: bool, _: i32) { *self.itens.entry((tid, comum)).or_default() += q; }
    fn tirar_item(&mut self, tid: u32, q: u32, comum: bool) { let e = self.itens.entry((tid, comum)).or_default(); *e = e.saturating_sub(q); }
    fn dar_dinheiro(&mut self, n: u32) { self.dinheiro += n; }
    fn tirar_dinheiro(&mut self, n: u32) { self.dinheiro = self.dinheiro.saturating_sub(n); }
    fn dar_exp(&mut self, exp: u32, sp: u32) { self.exp += exp; self.sp += sp; }
    fn dar_reputacao(&mut self, _: i32) {}
    fn avisar(&mut self, c: Vec<u8>) { self.avisos.push(c); }
    fn sortear(&mut self) -> f32 { 0.5 }
}

fn tarefas() -> Option<TasksData> {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/realm_155/config/tasks.data");
    match std::fs::read(caminho) {
        Ok(b) => Some(TasksData::load_from_bytes(&b).expect("tasks.data do realm")),
        Err(_) => {
            eprintln!("AVISO: {caminho} ausente — este teste NÃO verificou nada.");
            None
        }
    }
}

#[test]
fn o_arqueiro_novo_aceita_e_entrega_a_missao_do_guia_alado() {
    let Some(t) = tarefas() else { return };
    let mut l = ListasDeMissao::default();
    let mut j = Arqueiro::default();

    assert_eq!(Motor { tarefas: &t, listas: &mut l, j: &mut j, eu: 1 }.aceitar(32201, 0, true), 0);
    assert_eq!(l.ativa.indice(32201), Some(0));
    let [ativa, ..] = l.blocos();
    assert_eq!(ativa.len(), missoes::TAM_CABECALHO + missoes::TAM_ENTRADA);

    assert!(Motor { tarefas: &t, listas: &mut l, j: &mut j, eu: 1 }.entregar_no_npc(32201, 0));
    assert_eq!(l.ativa.quantidade, 0);
    assert_eq!(l.procurar_concluida(32201), 0);
    // O prêmio do arquivo: 25 de experiência, 10 de SP e 8 moedas.
    assert_eq!((j.exp, j.sp, j.dinheiro), (25, 10, 8));
}

#[test]
fn um_barbaro_nao_pega_a_missao_dos_alados() {
    let Some(t) = tarefas() else { return };
    struct Barbaro(Arqueiro);
    impl Jogador for Barbaro {
        fn agora(&self) -> u32 { self.0.agora() }
        fn nivel(&self) -> u32 { 1 }
        fn classe(&self) -> u32 { 4 }
        fn masculino(&self) -> bool { true }
        fn cultivo(&self) -> u32 { 0 }
        fn reputacao(&self) -> i32 { 0 }
        fn dinheiro(&self) -> u32 { 0 }
        fn e_gm(&self) -> bool { false }
        fn contar(&self, _: u32, _: bool) -> u32 { 0 }
        fn slots_livres(&self, _: bool) -> u32 { 32 }
        fn dar_item(&mut self, _: u32, _: u32, _: bool, _: i32) {}
        fn tirar_item(&mut self, _: u32, _: u32, _: bool) {}
        fn dar_dinheiro(&mut self, _: u32) {}
        fn tirar_dinheiro(&mut self, _: u32) {}
        fn dar_exp(&mut self, _: u32, _: u32) {}
        fn dar_reputacao(&mut self, _: i32) {}
        fn avisar(&mut self, c: Vec<u8>) { self.0.avisos.push(c); }
        fn sortear(&mut self) -> f32 { 0.5 }
    }
    let mut l = ListasDeMissao::default();
    let mut j = Barbaro(Arqueiro::default());
    assert_eq!(Motor { tarefas: &t, listas: &mut l, j: &mut j, eu: 1 }.aceitar(32201, 0, true), missoes::erro::CLASSE);
    // O erro vai ao cliente: `svr_task_err_code`, reason 6, código 13.
    let aviso = j.0.avisos.last().expect("aviso de erro");
    assert_eq!(aviso[6], 6);
    assert_eq!(u32::from_le_bytes([aviso[9], aviso[10], aviso[11], aviso[12]]), missoes::erro::CLASSE);
}

#[test]
fn todas_as_missoes_de_topo_do_realm_se_entregam_e_se_limpam_sem_quebrar_a_lista() {
    // Entrega cada missão de topo numa lista vazia e confere as invariantes que o cliente
    // cobra ao receber a lista (`ActiveTaskEntry::IsValid`, `TaskProcess.h:169-196`).
    let Some(t) = tarefas() else { return };
    let mut aceitas = 0;
    for &id in &t.de_topo {
        let mut l = ListasDeMissao::default();
        let mut j = Arqueiro::default();
        let sub = t.get_task(id).and_then(|m| m.sub_tasks.first().copied()).unwrap_or(0);
        let r = Motor { tarefas: &t, listas: &mut l, j: &mut j, eu: 1 }.aceitar(id, sub, false);
        if r != 0 {
            continue;
        }
        aceitas += 1;
        let a = &l.ativa;
        for i in 0..a.quantidade as usize {
            let e = a.e[i];
            let (idx, max) = (i as u8, a.quantidade);
            assert!(e.id != 0, "missão {id}: entrada {i} vazia dentro da contagem");
            assert!(e.pai == 0xff || (e.pai < idx && e.pai < max), "missão {id}: pai inválido na entrada {i}");
            assert!(e.anterior == 0xff || (e.anterior < idx && e.anterior < max), "missão {id}: anterior inválido");
            assert!(e.proximo == 0xff || (e.proximo > idx && e.proximo < max), "missão {id}: próximo inválido");
            assert!(e.filho == 0xff || (e.filho > idx && e.filho < max), "missão {id}: filho inválido");
        }
        let b = l.blocos();
        let volta = ListasDeMissao::de_blocos([&b[0], &b[1], &b[2], &b[3], &b[4]], &t);
        assert_eq!(volta.ativa.quantidade, l.ativa.quantidade, "missão {id}: a lista não sobreviveu à gravação");
    }
    assert!(aceitas > 1000, "só {aceitas} missões de topo foram aceitas por um Arqueiro nível 1");
}
