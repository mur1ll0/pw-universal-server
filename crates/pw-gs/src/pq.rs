//! B348 — a missão pública (PQ): `public_quest` e `public_quest_manager` (`gs/public_quest.cpp`/`.h`) e o batimento do
//! motor de missões (`InitPublicQuestSystem`/`OnTaskPublicQuestHeartbeat`, `gs/task/TaskServer.cpp:1242-1354`), que
//! roda a cada 5 s (`PublicQuestRunTick`, `gs/task/taskman.cpp:31-39`).
//!
//! Cada PQ (`m_bPQTask`) anda pelas etapas (os filhos, `m_bPQSubTask`): a etapa acaba quando a condição global dela
//! (`m_bFinNeedComp`, `CheckGlobalPQKeyValue`) vale sobre as variáveis globais do mapa; então todos recebem a
//! contribuição fixa + sorteada da etapa (quem tem a mínima), o controlador do prêmio é mexido e a PQ vai à próxima
//! etapa (zerando as variáveis dela) — ou, na última, termina e recomeça do zero. Os jogadores entram ao aceitar a PQ
//! e ganham contribuição matando os monstros da etapa; a cada 60 s as colocações (por classe e geral) são refeitas.
//!
//! **Não é igual:** o empate na colocação é decidido pelo id (no original, pela ordem de inserção no `multimap`).

use pw_data_loader::tasks::{CondicaoGlobal, TasksData};
use std::collections::{BTreeMap, HashMap};

/// `USER_CLASS_COUNT` do 1.5.5: as colocações são `[0, 12)` por classe e a 12ª geral (`ranks_size[13]`).
pub const CLASSES: usize = 12;
/// `PQ_RANKS_NOTIFY_CLIENT`: as 20 primeiras de cada colocação vão ao cliente.
pub const NO_AVISO: usize = 20;
/// `PQ_COMMONVALUE_NOTIFY_INTERVAL` / `PQ_RANKS_SORT_INTERVAL`, em segundos.
const INTERVALO_DAS_GLOBAIS: u32 = 5;
const INTERVALO_DA_COLOCACAO: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoDaPq {
    Iniciada,
    Rodando,
    Terminada,
}

/// `player_pq_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JogadorNaPq {
    pub roleid: i64,
    pub ativo: bool,
    pub classe: i32,
    pub pontos: i32,
    pub colocacao_na_classe: i32,
    pub colocacao_geral: i32,
    /// `rand`: −1 sem sorteio.
    pub sorteio: i32,
}

/// O que vai ao cliente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvisoDaPq {
    /// `PUBLIC_QUEST_INFO` (284) `{task_id, child_task_id, score, cls_place, all_place}`.
    Info { para: i64, missao: u32, etapa: u32, pontos: i32, na_classe: i32, geral: i32 },
    /// `PUBLIC_QUEST_RANKS` (285): o total de jogadores, as 13 contagens e as entradas `{roleid, race, score, rand, place}`.
    Colocacao { para: Vec<i64>, missao: u32, jogadores: u32, contagens: [u32; CLASSES + 1], entradas: Vec<[i32; 5]> },
    /// `COMMON_DATA_NOTIFY` (266) `{key, value}[]`.
    Globais { para: Vec<i64>, pares: Vec<(i32, i32)> },
}

/// `public_quest`.
#[derive(Debug, Clone)]
pub struct Pq {
    pub missao: u32,
    primeira_etapa: u32,
    pub etapa: u32,
    pub estampa: u32,
    pub estado: EstadoDaPq,
    contador: u32,
    /// `_common_value`: as variáveis globais da PQ e o último valor avisado.
    globais: BTreeMap<i32, i32>,
    pontos_mudaram: bool,
    sem_mudar_colocacao: bool,
    pub jogadores: HashMap<i64, JogadorNaPq>,
}

impl Pq {
    /// `Init`.
    pub fn nova(missao: u32, primeira_etapa: u32, chaves: &[i32]) -> Self {
        Self {
            missao,
            primeira_etapa,
            etapa: 0,
            estampa: 0,
            estado: EstadoDaPq::Iniciada,
            contador: 0,
            globais: chaves.iter().map(|k| (*k, 0)).collect(),
            pontos_mudaram: false,
            sem_mudar_colocacao: false,
            jogadores: HashMap::new(),
        }
    }

    fn ativos(&self) -> Vec<i64> {
        let mut v: Vec<i64> = self.jogadores.values().filter(|j| j.ativo).map(|j| j.roleid).collect();
        v.sort_unstable();
        v
    }

    fn info_de(&self, j: &JogadorNaPq) -> AvisoDaPq {
        AvisoDaPq::Info { para: j.roleid, missao: self.missao, etapa: self.etapa, pontos: j.pontos, na_classe: j.colocacao_na_classe, geral: j.colocacao_geral }
    }

    /// `Heartbeat(tick)`.
    pub fn batimento(&mut self, segundos: u32, globais_do_mapa: &HashMap<i32, i32>) -> Vec<AvisoDaPq> {
        let mut s = Vec::new();
        if self.estado != EstadoDaPq::Rodando || self.jogadores.is_empty() {
            return s;
        }
        self.contador += segundos;
        if self.contador % INTERVALO_DAS_GLOBAIS == 0 {
            s.extend(self.avisar_globais(globais_do_mapa));
        }
        if self.contador % INTERVALO_DA_COLOCACAO == 0 && self.pontos_mudaram && !self.sem_mudar_colocacao {
            self.ordenar();
            s.push(self.colocacao(self.ativos()));
            self.pontos_mudaram = false;
            for j in self.jogadores.values().filter(|j| j.ativo) {
                s.push(self.info_de(j));
            }
        }
        s
    }

    /// `SetStart`: limpa tudo (`RestartClear`), a primeira etapa, a estampa e as variáveis dela zeradas.
    pub fn comecar(&mut self, chaves: &[i32], sem_mudar_colocacao: bool, agora: u32, globais_do_mapa: &mut HashMap<i32, i32>) {
        self.contador = 0;
        self.pontos_mudaram = false;
        self.jogadores.clear();
        self.etapa = self.primeira_etapa;
        self.estampa = agora;
        for k in chaves {
            if let Some(v) = self.globais.get_mut(k) {
                *v = 0;
                globais_do_mapa.insert(*k, 0);
            }
        }
        self.sem_mudar_colocacao = sem_mudar_colocacao;
        self.estado = EstadoDaPq::Rodando;
    }

    /// `SetFinish`.
    pub fn terminar(&mut self) -> Vec<AvisoDaPq> {
        let mut s = Vec::new();
        if self.estado != EstadoDaPq::Rodando {
            return s;
        }
        if self.pontos_mudaram {
            self.ordenar();
            s.push(self.colocacao(self.ativos()));
            self.pontos_mudaram = false;
        }
        self.etapa = 0;
        self.estado = EstadoDaPq::Terminada;
        for j in self.jogadores.values().filter(|j| j.ativo) {
            s.push(self.info_de(j));
        }
        s
    }

    /// `SetNextChildTask`: as variáveis da etapa nova zeram no mapa (o "último avisado" fica, e o aviso sai se mudou).
    pub fn proxima_etapa(&mut self, etapa: u32, chaves: &[i32], sem_mudar_colocacao: bool, globais_do_mapa: &mut HashMap<i32, i32>) -> Vec<AvisoDaPq> {
        let mut s = Vec::new();
        if self.estado != EstadoDaPq::Rodando {
            return s;
        }
        if self.pontos_mudaram {
            self.ordenar();
            s.push(self.colocacao(self.ativos()));
            self.pontos_mudaram = false;
        }
        self.etapa = etapa;
        for k in chaves {
            if self.globais.contains_key(k) {
                globais_do_mapa.insert(*k, 0);
            }
            s.extend(self.avisar_globais(globais_do_mapa));
        }
        for j in self.jogadores.values().filter(|j| j.ativo) {
            s.push(self.info_de(j));
        }
        self.sem_mudar_colocacao = sem_mudar_colocacao;
        s
    }

    /// `SetRandContrib(fixed, max_rand, lowest)`: abaixo da mínima ninguém ganha; acima, fixa + `Rand(0, max)`.
    pub fn sortear_contribuicao(&mut self, fixa: i32, sorteada_max: i32, minima: i32, mut sortear: impl FnMut(i32) -> i32) {
        if self.estado != EstadoDaPq::Rodando {
            return;
        }
        for j in self.jogadores.values_mut() {
            if j.pontos < minima {
                j.sorteio = if sorteada_max <= 0 { -1 } else { 0 };
                continue;
            }
            j.pontos += fixa;
            if sorteada_max <= 0 {
                j.sorteio = -1;
            } else {
                j.sorteio = sortear(sorteada_max);
                j.pontos += j.sorteio;
            }
        }
        if fixa != 0 || sorteada_max > 0 {
            self.pontos_mudaram = true;
        }
    }

    /// `AddPlayer`: quem já está volta a ficar ativo; quem é novo recebe as globais, a info zerada e a colocação.
    pub fn entrar(&mut self, roleid: i64, classe: i32, globais_do_mapa: &HashMap<i32, i32>) -> Vec<AvisoDaPq> {
        let mut s = Vec::new();
        if self.estado != EstadoDaPq::Rodando {
            return s;
        }
        if let Some(j) = self.jogadores.get_mut(&roleid) {
            j.ativo = true;
            return s;
        }
        self.jogadores.insert(roleid, JogadorNaPq { roleid, ativo: true, classe, pontos: 0, colocacao_na_classe: -1, colocacao_geral: -1, sorteio: -1 });
        s.push(AvisoDaPq::Globais { para: vec![roleid], pares: self.todas_as_globais(globais_do_mapa) });
        s.push(AvisoDaPq::Info { para: roleid, missao: self.missao, etapa: self.etapa, pontos: 0, na_classe: -1, geral: -1 });
        s.push(self.colocacao(vec![roleid]));
        s
    }

    /// `RemovePlayer`: a info vazia ao cliente e sai da PQ.
    pub fn sair(&mut self, roleid: i64) -> Vec<AvisoDaPq> {
        if self.estado != EstadoDaPq::Rodando || self.jogadores.remove(&roleid).is_none() {
            return Vec::new();
        }
        vec![AvisoDaPq::Info { para: roleid, missao: self.missao, etapa: 0, pontos: 0, na_classe: -1, geral: -1 }]
    }

    /// `UpdatePlayerContrib`.
    pub fn somar(&mut self, roleid: i64, mais: i32) -> Vec<AvisoDaPq> {
        if self.estado != EstadoDaPq::Rodando {
            return Vec::new();
        }
        let Some(j) = self.jogadores.get_mut(&roleid) else { return Vec::new() };
        j.pontos += mais;
        let j = *j;
        self.pontos_mudaram = true;
        vec![self.info_de(&j)]
    }

    /// `EnterWorldInit`: quem estava volta ativo e recebe tudo de novo.
    pub fn voltar(&mut self, roleid: i64, globais_do_mapa: &HashMap<i32, i32>) -> Vec<AvisoDaPq> {
        if self.estado != EstadoDaPq::Rodando {
            return Vec::new();
        }
        let Some(j) = self.jogadores.get_mut(&roleid) else { return Vec::new() };
        j.ativo = true;
        let j = *j;
        let pares: Vec<(i32, i32)> = self.globais.iter().map(|(k, v)| (*k, *v)).collect();
        let _ = globais_do_mapa;
        vec![AvisoDaPq::Globais { para: vec![roleid], pares }, self.info_de(&j), self.colocacao(vec![roleid])]
    }

    /// `LeaveWorld`.
    pub fn ausentar(&mut self, roleid: i64) {
        if let Some(j) = self.jogadores.get_mut(&roleid) {
            j.ativo = false;
        }
    }

    pub fn pontos(&self, roleid: i64) -> i32 {
        self.jogadores.get(&roleid).map(|j| j.pontos).unwrap_or(0)
    }

    pub fn colocacao_de(&self, roleid: i64, por_classe: bool) -> i32 {
        self.jogadores
            .get(&roleid)
            .map(|j| if por_classe { j.colocacao_na_classe } else { j.colocacao_geral })
            .unwrap_or(-1)
    }

    /// `SortRanks`: maior pontuação primeiro, 1, 2, 3…; cada classe e a geral.
    fn ordenar(&mut self) {
        let mut todos: Vec<(i32, i64)> = self.jogadores.values().map(|j| (j.pontos, j.roleid)).collect();
        todos.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut por_classe = [1i32; CLASSES];
        for (lugar, (_, id)) in todos.iter().enumerate() {
            let Some(j) = self.jogadores.get_mut(id) else { continue };
            j.colocacao_geral = lugar as i32 + 1;
            let c = (j.classe & 0x7FFF_FFFF) as usize;
            if c < CLASSES {
                j.colocacao_na_classe = por_classe[c];
                por_classe[c] += 1;
            } else {
                j.colocacao_na_classe = -1;
            }
        }
    }

    /// `SendRanks`/`BroadcastRanks`.
    fn colocacao(&self, para: Vec<i64>) -> AvisoDaPq {
        let mut todos: Vec<&JogadorNaPq> = self.jogadores.values().filter(|j| j.colocacao_geral > 0).collect();
        todos.sort_by(|a, b| b.pontos.cmp(&a.pontos).then(a.roleid.cmp(&b.roleid)));
        let mut contagens = [0u32; CLASSES + 1];
        let mut entradas = Vec::new();
        let mut blocos: Vec<Vec<[i32; 5]>> = vec![Vec::new(); CLASSES + 1];
        for j in &todos {
            let c = (j.classe & 0x7FFF_FFFF) as usize;
            if c < CLASSES && blocos[c].len() < NO_AVISO {
                blocos[c].push([j.roleid as i32, j.classe, j.pontos, j.sorteio, j.colocacao_na_classe]);
            }
            if blocos[CLASSES].len() < NO_AVISO {
                blocos[CLASSES].push([j.roleid as i32, j.classe, j.pontos, j.sorteio, j.colocacao_geral]);
            }
        }
        for (k, b) in blocos.into_iter().enumerate() {
            contagens[k] = b.len() as u32;
            entradas.extend(b);
        }
        AvisoDaPq::Colocacao { para, missao: self.missao, jogadores: self.jogadores.len() as u32, contagens, entradas }
    }

    fn todas_as_globais(&self, globais_do_mapa: &HashMap<i32, i32>) -> Vec<(i32, i32)> {
        let _ = globais_do_mapa;
        self.globais.iter().map(|(k, v)| (*k, *v)).collect()
    }

    /// `BroadcastCommonvalue`: só as que mudaram desde o último aviso.
    fn avisar_globais(&mut self, globais_do_mapa: &HashMap<i32, i32>) -> Vec<AvisoDaPq> {
        let mut pares = Vec::new();
        for (k, v) in self.globais.iter_mut() {
            let agora = globais_do_mapa.get(k).copied().unwrap_or(0);
            if agora != *v {
                *v = agora;
                pares.push((*k, agora));
            }
        }
        if pares.is_empty() {
            return Vec::new();
        }
        vec![AvisoDaPq::Globais { para: self.ativos(), pares }]
    }
}

/// `_compare_pq_key_value` / `CheckGlobalPQKeyValue(bFinCheck)` (`TaskTempl.inl:924-997`).
pub fn condicao_vale(c: &CondicaoGlobal, globais: &HashMap<i32, i32>) -> bool {
    condicao_vale_com(c, |k| globais.get(&k).copied().unwrap_or(0))
}

/// O mesmo, com a leitura da variável global por função (`_compare_key_value`, `pTask->GetGlobalValue`).
pub fn condicao_vale_com(c: &CondicaoGlobal, global: impl Fn(i32) -> i32) -> bool {
    let lado = |tipo: i32, n: i32| if tipo == 0 { global(n) } else { n };
    let cmp = |x: &pw_data_loader::tasks::ComparacaoGlobal| {
        let (e, d) = (lado(x.tipo_esquerda, x.esquerda), lado(x.tipo_direita, x.direita));
        match x.operador {
            0 => e > d,
            1 => e == d,
            2 => e < d,
            _ => false,
        }
    };
    let (a, b) = (cmp(&c.a), cmp(&c.b));
    if c.e {
        a && b
    } else {
        a || b
    }
}

/// A contribuição de um abate para a PQ de um jogador (`msg_contribution_t`, `npc.cpp:1666-1720`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContribuicaoDoAbate {
    pub jogador: i64,
    /// `is_owner`.
    pub dono: bool,
    /// `team_contribution`: a fração do dano da equipe (ou a própria, sozinho).
    pub equipe: f32,
    /// `team_member_count`.
    pub membros: i32,
    /// `personal_contribution`.
    pub pessoal: f32,
}

/// `gnpc_imp::DispatchExp` (parte da contribuição, `npc.cpp:1605`, `:1666-1720`) + `GM_MSG_GROUP_CONTRIBUTION_TO_KILL_NPC`
/// (`player.cpp:2324-…`): fator `1 / max(dano total, vida máxima)`; sozinho, a fração dele; em equipe, cada membro que
/// bateu recebe a soma da equipe, a contagem dos que bateram e a própria fração. `dono` é o grupo (ou o jogador
/// sozinho) do dono do abate. **Não é igual:** o raio de equipe (`TEAM_EXP_DISTANCE`) não é conferido.
pub fn contribuicoes_do_abate(danos: &[(i64, i64)], max_hp: i64, dono: Option<i64>, grupo_de: impl Fn(i64) -> Option<u32>) -> Vec<ContribuicaoDoAbate> {
    let total: i64 = danos.iter().map(|d| d.1).sum();
    let fator = 1.0 / total.max(max_hp).max(1) as f32;
    let grupo_do_dono = dono.and_then(&grupo_de);
    let mut por_grupo: BTreeMap<u32, (f32, i32)> = BTreeMap::new();
    for (id, dano) in danos {
        if let Some(g) = grupo_de(*id) {
            let e = por_grupo.entry(g).or_insert((0.0, 0));
            e.0 += *dano as f32 * fator;
            e.1 += 1;
        }
    }
    danos
        .iter()
        .map(|(id, dano)| {
            let pessoal = *dano as f32 * fator;
            match grupo_de(*id) {
                Some(g) => {
                    let (soma, n) = por_grupo[&g];
                    ContribuicaoDoAbate { jogador: *id, dono: grupo_do_dono == Some(g), equipe: soma, membros: n, pessoal }
                }
                None => ContribuicaoDoAbate { jogador: *id, dono: dono == Some(*id) && grupo_do_dono.is_none(), equipe: pessoal, membros: 1, pessoal },
            }
        })
        .collect()
}

/// O que o batimento das PQ pede ao mundo além dos avisos: mexer controladores (`SetMonsterController`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SaidaDasPq {
    pub avisos: Vec<AvisoDaPq>,
    pub controladores: Vec<(i32, bool)>,
}

/// `public_quest_manager` + o lado do motor (`InitPublicQuestSystem`, `OnTaskPublicQuestHeartbeat`).
#[derive(Debug, Clone, Default)]
pub struct GerenciadorDePq {
    pub pqs: BTreeMap<u32, Pq>,
}

impl GerenciadorDePq {
    /// `InitPublicQuestSystem` (`TaskServer.cpp:1243-1287`): cada PQ com filhos começa na primeira etapa.
    pub fn iniciar(tarefas: &TasksData, agora: u32, globais: &mut HashMap<i32, i32>) -> (Self, Vec<(i32, bool)>) {
        let mut g = GerenciadorDePq::default();
        let mut controles = Vec::new();
        let mut ids: Vec<u32> = tarefas.tasks.values().filter(|t| t.pq && t.parent.is_none()).map(|t| t.id).collect();
        ids.sort_unstable();
        for id in ids {
            let t = &tarefas.tasks[&id];
            let Some(primeira) = t.sub_tasks.first().and_then(|f| tarefas.tasks.get(f)) else { continue };
            let mut pq = Pq::nova(id, primeira.id, &t.pq_chaves);
            pq.comecar(&primeira.pq_chaves, primeira.nao_muda_colocacao, agora, globais);
            if primeira.controle_de_monstros != 0 {
                controles.push((primeira.controle_de_monstros, primeira.dispara_controle));
            }
            g.pqs.insert(id, pq);
        }
        (g, controles)
    }

    /// `PublicQuestRunTick` a cada 5 s: o `Heartbeat(5)` de cada PQ e o `OnTaskPublicQuestHeartbeat`.
    pub fn batimento(&mut self, tarefas: &TasksData, agora: u32, globais: &mut HashMap<i32, i32>, mut sortear: impl FnMut(i32) -> i32) -> SaidaDasPq {
        let mut s = SaidaDasPq::default();
        for pq in self.pqs.values_mut() {
            s.avisos.extend(pq.batimento(5, globais));
        }
        for (id, pq) in self.pqs.iter_mut() {
            let Some(t) = tarefas.tasks.get(id) else { continue };
            let Some(etapa) = tarefas.tasks.get(&pq.etapa) else { continue };
            // Sem condição de fim (`m_bFinNeedComp` falso) o original devolve 0: a etapa acaba na hora.
            if !etapa.global_fim.as_ref().map_or(true, |c| condicao_vale(c, globais)) {
                continue;
            }
            let p = &etapa.rewards;
            pq.sortear_contribuicao(p.contribuicao, p.contribuicao_sorteada, p.contribuicao_minima, &mut sortear);
            if p.controle_de_monstros != 0 {
                s.controladores.push((p.controle_de_monstros, p.dispara_controle));
            }
            let pos = t.sub_tasks.iter().position(|f| *f == etapa.id);
            let proxima = pos.and_then(|i| t.sub_tasks.get(i + 1)).and_then(|f| tarefas.tasks.get(f));
            match proxima {
                Some(n) => {
                    s.avisos.extend(pq.proxima_etapa(n.id, &n.pq_chaves, n.nao_muda_colocacao, globais));
                    if n.controle_de_monstros != 0 {
                        s.controladores.push((n.controle_de_monstros, n.dispara_controle));
                    }
                }
                None => {
                    s.avisos.extend(pq.terminar());
                    let Some(primeira) = t.sub_tasks.first().and_then(|f| tarefas.tasks.get(f)) else { continue };
                    pq.comecar(&primeira.pq_chaves, primeira.nao_muda_colocacao, agora, globais);
                    if primeira.controle_de_monstros != 0 {
                        s.controladores.push((primeira.controle_de_monstros, primeira.dispara_controle));
                    }
                }
            }
        }
        s
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use pw_data_loader::tasks::{ComparacaoGlobal, TaskTemplate};

    fn maior_que(chave: i32, n: i32) -> CondicaoGlobal {
        CondicaoGlobal { e: false, a: ComparacaoGlobal { tipo_esquerda: 0, esquerda: chave, operador: 0, tipo_direita: 1, direita: n }, b: ComparacaoGlobal::default() }
    }

    fn dados() -> TasksData {
        let mut d = TasksData::default();
        let mut topo = TaskTemplate::vazia(100);
        topo.pq = true;
        topo.pq_chaves = vec![7, 8];
        topo.sub_tasks = vec![101, 102];
        let mut e1 = TaskTemplate::vazia(101);
        e1.parent = Some(100);
        e1.pq_sub = true;
        e1.pq_chaves = vec![7];
        e1.global_fim = Some(maior_que(7, 2));
        e1.rewards.contribuicao = 10;
        e1.rewards.contribuicao_sorteada = 5;
        e1.rewards.contribuicao_minima = 1;
        e1.rewards.controle_de_monstros = 55;
        e1.rewards.dispara_controle = true;
        let mut e2 = TaskTemplate::vazia(102);
        e2.parent = Some(100);
        e2.pq_sub = true;
        e2.pq_chaves = vec![8];
        e2.global_fim = Some(maior_que(8, 0));
        for t in [topo, e1, e2] {
            if t.parent.is_none() {
                d.de_topo.push(t.id);
            }
            d.tasks.insert(t.id, t);
        }
        d
    }

    #[test]
    fn etapas_contribuicao_colocacao_e_recomeco() {
        let d = dados();
        let mut globais = HashMap::from([(7, 99), (8, 99)]);
        let (mut g, ctrl) = GerenciadorDePq::iniciar(&d, 1000, &mut globais);
        assert!(ctrl.is_empty());
        assert_eq!(globais[&7], 0, "a variável da primeira etapa zera ao começar");
        let pq = g.pqs.get_mut(&100).unwrap();
        assert_eq!((pq.etapa, pq.estampa, pq.estado), (101, 1000, EstadoDaPq::Rodando));
        // Entram dois jogadores; o 1 ganha 3 por abate, o 2 nada.
        let avisos = pq.entrar(1, 0, &globais);
        assert_eq!(avisos.len(), 3, "globais, info e colocação ao entrar");
        pq.entrar(2, 0, &globais);
        assert_eq!(pq.somar(1, 3), vec![AvisoDaPq::Info { para: 1, missao: 100, etapa: 101, pontos: 3, na_classe: -1, geral: -1 }]);
        // A etapa ainda não acabou (7 = 0, precisa > 2).
        let s = g.batimento(&d, 1005, &mut globais, |_| 4);
        assert!(s.controladores.is_empty());
        assert_eq!(g.pqs[&100].etapa, 101);
        // A variável passa de 2: contribuição fixa 10 + sorteio 4 a quem tem a mínima (1), controlador 55 e a etapa 102.
        globais.insert(7, 3);
        let s = g.batimento(&d, 1010, &mut globais, |_| 4);
        assert_eq!(s.controladores, vec![(55, true)]);
        let pq = &g.pqs[&100];
        assert_eq!(pq.etapa, 102);
        assert_eq!((pq.pontos(1), pq.pontos(2)), (3 + 10 + 4, 0), "o 2 ficou abaixo da mínima");
        assert_eq!((pq.colocacao_de(1, false), pq.colocacao_de(2, false)), (1, 2), "a colocação refeita na troca de etapa");
        // A última etapa acaba: a PQ termina e recomeça do zero, com estampa nova.
        globais.insert(8, 1);
        g.batimento(&d, 1015, &mut globais, |_| 0);
        let pq = &g.pqs[&100];
        assert_eq!((pq.etapa, pq.estampa, pq.jogadores.len()), (101, 1015, 0));
    }

    /// `msg_contribution_t`: fator `1/max(total, vida)`; em equipe, a soma da equipe e quantos bateram.
    #[test]
    fn contribuicao_do_abate_sozinho_e_em_equipe() {
        let danos = vec![(1, 300), (2, 100), (3, 100)];
        let grupo = |id: i64| (id >= 2).then_some(9u32);
        let c = contribuicoes_do_abate(&danos, 1000, Some(1), grupo);
        assert_eq!(c[0], ContribuicaoDoAbate { jogador: 1, dono: true, equipe: 0.3, membros: 1, pessoal: 0.3 });
        assert_eq!((c[1].dono, c[1].membros, c[1].equipe, c[1].pessoal), (false, 2, 0.2, 0.1));
        // Dano total maior que a vida: o fator é sobre o total.
        let c = contribuicoes_do_abate(&[(1, 2000)], 1000, Some(1), |_| None);
        assert_eq!(c[0].pessoal, 1.0);
    }

    #[test]
    fn condicao_e_ou() {
        let g = HashMap::from([(1, 5), (2, 0)]);
        let mut c = CondicaoGlobal {
            e: true,
            a: ComparacaoGlobal { tipo_esquerda: 0, esquerda: 1, operador: 1, tipo_direita: 1, direita: 5 },
            b: ComparacaoGlobal { tipo_esquerda: 0, esquerda: 2, operador: 2, tipo_direita: 0, direita: 1 },
        };
        assert!(condicao_vale(&c, &g), "5 == 5 e 0 < 5");
        c.b.operador = 0;
        assert!(!condicao_vale(&c, &g), "0 > 5 falha o 'e'");
        c.e = false;
        assert!(condicao_vale(&c, &g), "o 'ou' passa");
    }
}
