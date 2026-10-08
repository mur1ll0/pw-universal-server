//! Painel (E6, B198): missões — ver, dar, concluir, cancelar (apagar) e esquecer a conclusão.
//!
//! O cliente refaz cada operação na cópia dele das listas a partir dos avisos de missão, todos no
//! `TASK_VAR_DATA` (106) → `OnServerNotify` (`EC_HostMsg.cpp:3958-3963`,
//! `Task/TaskProcess.cpp:2705-2815` do cliente 1.5.5). Não há pacote que reenvie a lista em jogo
//! (o `TASK_DATA` só vai na entrada), então cada ação usa o aviso que o original manda para ela:
//!
//! | ação | o motor | aviso |
//! | :--- | :--- | :--- |
//! | dar | [`Motor::dar_pelo_painel`] (`DeliverTask` sem pré-requisitos) | `NEW` (1) |
//! | concluir | [`Motor::concluir_pelo_painel`] (o `OnTaskForceSucc`) | `FINISHED` (5), e `COMPLETE` (2) com o prêmio se a conclusão é direta |
//! | cancelar | [`Motor::apagar_pelo_painel`] (`ClearTask`) | `GIVE_UP` (3) |
//! | esquecer | [`ListasDeMissao::esquecer_conclusao`] | nenhum — só offline |
//!
//! 1.2.6: `NEW` (14 bytes) e `COMPLETE` (10) batem com a captura do original (B101–B102);
//! `GIVE_UP` não foi capturado nem conferido no cliente 1.2.6, e apagar desfaz estado — online
//! fica `precisa_estar_offline` (decisão de 2026-10-07).
//!
//! **Offline** o motor roda sobre as listas e as bolsas lidas do banco ([`JogadorOffline`]):
//! itens de missão entram e saem como em jogo; o que precisa da entidade (dinheiro, EXP, SP,
//! reputação, cultivo, teto de chi, jaula, teleporte, monstros) faz a operação inteira ser
//! recusada com `precisa_estar_online`, sem gravar nada.

use super::*;
use crate::economia::{Bolsa, TAMANHO_DA_BOLSA, TAMANHO_DA_BOLSA_DE_MISSAO};
use crate::missoes::{Jogador, ListasDeMissao, Motor, SEM};
use pw_data_loader::{tasks::TasksData, GameDataManager};

/// Concluídas por página da consulta (o quadro do canal tem teto).
pub const CONCLUIDAS_POR_PAGINA: usize = 200;
/// Missões por busca.
const RESULTADOS_DA_BUSCA: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcaoDeMissao {
    Dar,
    Concluir,
    Cancelar,
    Esquecer,
}

/// `{acao, id, sub?}`; `sub` é a submissão escolhida de uma missão `m_bChooseOne` (só em `dar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EdicaoDeMissao {
    pub acao: AcaoDeMissao,
    pub id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<u32>,
}

impl EdicaoDeMissao {
    pub fn valida(&self) -> bool {
        // O id da missão vai num `u16` em todos os avisos e nas listas.
        self.id > 0 && self.id <= u16::MAX as u32 && self.sub.map_or(true, |s| s > 0 && s <= u16::MAX as u32)
    }
}

/// As listas em JSON: as ativas (com o pai e o estado) e uma página das concluídas, filtradas por
/// `busca` (nome ou id).
pub fn listas_em_json(l: &ListasDeMissao, tarefas: &TasksData, pagina: usize, busca: Option<&str>) -> serde_json::Value {
    let nome = |id: u32| tarefas.get_task(id).map(|t| t.name.clone()).unwrap_or_default();
    let a = &l.ativa;
    let ativas: Vec<serde_json::Value> = (0..a.quantidade as usize)
        .filter(|&i| a.e[i].valida)
        .map(|i| {
            let en = a.e[i];
            let t = tarefas.get_task(en.id as u32);
            let pedidos: Vec<u32> = t.map(|t| t.monster_kills.iter().take(3).map(|m| m.quantidade).collect()).unwrap_or_default();
            serde_json::json!({
                "id": en.id, "nome": nome(en.id as u32),
                "pai": (en.pai != SEM).then(|| a.e[en.pai as usize].id),
                "finalizada": en.finalizada(), "sucesso": en.sucesso(), "desistiu": en.desistiu(),
                "tempo": en.tempo,
                "abates": (0..pedidos.len()).map(|k| [en.monstros(k) as u32, pedidos[k]]).collect::<Vec<_>>(),
                "conclusao_direta": t.is_some_and(|t| t.tipo_de_conclusao == crate::missoes::conclusao::DIRETA),
            })
        })
        .collect();
    let filtro = busca.map(|b| b.trim().to_lowercase()).filter(|b| !b.is_empty());
    let filtradas: Vec<&crate::missoes::Concluida> = l
        .concluidas
        .iter()
        .filter(|c| match &filtro {
            None => true,
            Some(b) => b.parse::<u32>().ok() == Some(c.id as u32) || nome(c.id as u32).to_lowercase().contains(b.as_str()),
        })
        .collect();
    let concluidas: Vec<serde_json::Value> = filtradas
        .iter()
        .skip(pagina * CONCLUIDAS_POR_PAGINA)
        .take(CONCLUIDAS_POR_PAGINA)
        .map(|c| serde_json::json!({"id": c.id, "nome": nome(c.id as u32), "falhou": c.falhou, "vezes": c.vezes}))
        .collect();
    serde_json::json!({
        "ativas": ativas, "concluidas": concluidas, "total_concluidas": filtradas.len(),
        "pagina": pagina, "por_pagina": CONCLUIDAS_POR_PAGINA,
    })
}

/// O jogador do motor fora do jogo: bolsas e ficha lidas do banco.
pub(crate) struct JogadorOffline<'d> {
    pub dados: &'d GameDataManager,
    pub nivel: u32,
    pub classe: u32,
    pub masculino: bool,
    pub cultivo: u32,
    pub reputacao: i32,
    pub dinheiro: u32,
    pub bolsa: Bolsa,
    pub bolsa_de_missao: Bolsa,
    /// A operação pediu algo que só a entidade em jogo faz (o primeiro pedido).
    pub exige_entidade: Option<&'static str>,
    pub bolsa_cheia: bool,
}

impl JogadorOffline<'_> {
    fn exigir(&mut self, o_que: &'static str) {
        self.exige_entidade.get_or_insert(o_que);
    }
}

impl Jogador for JogadorOffline<'_> {
    fn agora(&self) -> u32 { super::jogo::agora() }
    fn nivel(&self) -> u32 { self.nivel }
    fn classe(&self) -> u32 { self.classe }
    fn masculino(&self) -> bool { self.masculino }
    fn cultivo(&self) -> u32 { self.cultivo }
    fn reputacao(&self) -> i32 { self.reputacao }
    fn dinheiro(&self) -> u32 { self.dinheiro }
    fn e_gm(&self) -> bool { false }
    fn contar(&self, tid: u32, comum: bool) -> u32 {
        if comum { self.bolsa.contar(tid) } else { self.bolsa_de_missao.contar(tid) }
    }
    fn slots_livres(&self, comum: bool) -> u32 {
        if comum { self.bolsa.livres() } else { self.bolsa_de_missao.livres() }
    }
    fn dar_item(&mut self, tid: u32, quantidade: u32, comum: bool, _validade: i32) {
        if quantidade == 0 {
            return;
        }
        let dados = self.dados;
        let bolsa = if comum { &mut self.bolsa } else { &mut self.bolsa_de_missao };
        if bolsa.empilhar_gerado(tid, quantidade, dados).is_none() {
            self.bolsa_cheia = true;
        }
    }
    fn tirar_item(&mut self, tid: u32, quantidade: u32, comum: bool) {
        let bolsa = if comum { &mut self.bolsa } else { &mut self.bolsa_de_missao };
        bolsa.tirar(tid, quantidade);
    }
    fn dar_dinheiro(&mut self, n: u32) { if n > 0 { self.exigir("dinheiro") } }
    fn tirar_dinheiro(&mut self, n: u32) { if n > 0 { self.exigir("dinheiro") } }
    fn dar_exp(&mut self, exp: u32, sp: u32) { if exp > 0 || sp > 0 { self.exigir("exp") } }
    fn dar_reputacao(&mut self, r: i32) { if r != 0 { self.exigir("reputacao") } }
    fn definir_cultivo(&mut self, _: u32) { self.exigir("cultivo") }
    fn definir_teto_de_chi(&mut self, _: u32) { self.exigir("chi") }
    fn ampliar_jaula(&mut self, _: u32) { self.exigir("jaula") }
    fn avisar(&mut self, _: Vec<u8>) {}
    fn sortear(&mut self) -> f32 { rand::Rng::gen(&mut rand::thread_rng()) }
    fn teleportar(&mut self, _: u32, _: [f32; 3]) { self.exigir("teleporte") }
    fn invocar_monstro(&mut self, _: u32, _: u32, _: u32, _: i32, _: bool) { self.exigir("monstros") }
}

/// Uma ação sobre as listas, a mesma online e offline. `Ok(concluida)` só importa a `concluir`.
fn aplicar<J: Jogador>(m: &mut Motor<J>, e: &EdicaoDeMissao) -> Result<bool, &'static str> {
    match e.acao {
        AcaoDeMissao::Dar => m.dar_pelo_painel(e.id, e.sub.unwrap_or(0)).map(|_| false),
        AcaoDeMissao::Concluir => m.concluir_pelo_painel(e.id),
        AcaoDeMissao::Cancelar => m.apagar_pelo_painel(e.id).map(|_| false),
        AcaoDeMissao::Esquecer => {
            if m.listas.ativa.indice(e.id).is_some() {
                return Err("missao_ativa");
            }
            if m.listas.esquecer_conclusao(e.id) { Ok(false) } else { Err("sem_registro") }
        }
    }
}

impl BusServer {
    /// As listas do personagem: as da memória quando ele está neste mapa (a verdade em jogo),
    /// senão as do banco (`task_lists`, o que a entrada lê).
    pub(crate) async fn missoes_do_painel(&self, roleid: i32, pagina: usize, busca: Option<&str>) -> serde_json::Value {
        let (dados, viva) = {
            let mundo = self.world.read().await;
            (mundo.data_manager.clone(), mundo.players.get(&(roleid as i64)).map(|p| p.missoes.clone()))
        };
        let (listas, origem) = match viva {
            Some(l) => (l, "em_jogo"),
            None => match self.repo().await.task_lists().carregar(roleid).await {
                Ok(Some(l)) => (
                    ListasDeMissao::de_blocos([&l.ativa, &l.concluidas, &l.tempos, &l.contagens, &l.deposito], &dados.tasks),
                    "persistida",
                ),
                Ok(None) => (ListasDeMissao::default(), "persistida"),
                Err(_) => return serde_json::json!({"codigo":"banco_indisponivel"}),
            },
        };
        let mut j = listas_em_json(&listas, &dados.tasks, pagina, busca);
        j["origem"] = serde_json::json!(origem);
        j
    }

    /// Até 30 missões **de topo** do `tasks.data` do realm por nome ou id, com as submissões a
    /// escolher quando a missão é `m_bChooseOne`.
    pub(crate) async fn buscar_missoes(&self, texto: &str) -> serde_json::Value {
        let dados = self.world.read().await.data_manager.clone();
        let texto = texto.trim().to_lowercase();
        let por_id = texto.parse::<u32>().ok();
        let mut achadas: Vec<&pw_data_loader::tasks::TaskTemplate> = dados
            .tasks
            .tasks
            .values()
            .filter(|t| t.parent.is_none())
            .filter(|t| por_id == Some(t.id) || (!texto.is_empty() && t.name.to_lowercase().contains(&texto)))
            .collect();
        achadas.sort_by_key(|t| (por_id != Some(t.id), t.id));
        let lista: Vec<serde_json::Value> = achadas
            .iter()
            .take(RESULTADOS_DA_BUSCA)
            .map(|t| {
                let subs: Vec<serde_json::Value> = if t.escolhe_um_filho {
                    t.sub_tasks.iter().filter_map(|&s| dados.tasks.get_task(s))
                        .map(|s| serde_json::json!({"id": s.id, "nome": s.name})).collect()
                } else {
                    Vec::new()
                };
                serde_json::json!({"id": t.id, "nome": t.name, "nivel": [t.min_level, t.max_level],
                    "filhas": t.sub_tasks.len(), "escolha": subs})
            })
            .collect();
        serde_json::json!({"missoes": lista})
    }

    /// Online: a ação pelo `com_contexto` (que manda os avisos, os prêmios e grava). `None`: não
    /// está neste mapa.
    pub(crate) async fn missao_pelo_painel(&self, roleid: i32, e: EdicaoDeMissao) -> Option<serde_json::Value> {
        if e.acao == AcaoDeMissao::Esquecer
            || e.acao == AcaoDeMissao::Cancelar && self.versao() == GameVersion::V1_2_6
        {
            if !self.world.read().await.players.contains_key(&(roleid as i64)) {
                return None;
            }
            return Some(serde_json::json!({"erro": "precisa_estar_offline"}));
        }
        let dados = self.world.read().await.data_manager.clone();
        let r = self
            .com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| aplicar(m, &e)))
            .await?;
        Some(match r {
            Ok(concluida) => {
                info!(roleid, ?e, concluida, "painel: missão (online)");
                serde_json::json!({"erro": null, "concluida": concluida})
            }
            Err(codigo) => serde_json::json!({"erro": codigo}),
        })
    }

    /// Offline: o motor sobre as listas e as bolsas do banco; grava as bolsas e as listas. Quem
    /// chama segura a guarda de presença e a trava de gravação.
    pub(crate) async fn missao_no_banco(&self, roleid: i32, e: EdicaoDeMissao) -> Result<bool, &'static str> {
        let dados = self.world.read().await.data_manager.clone();
        let repo = self.repo().await;
        let ficha = repo.get_details_por_role(roleid).await.map_err(|_| "banco_indisponivel")?.ok_or("personagem_inexistente")?;
        let gravadas = repo.task_lists().carregar(roleid).await.map_err(|_| "banco_indisponivel")?;
        let mut listas = match &gravadas {
            Some(l) => ListasDeMissao::de_blocos([&l.ativa, &l.concluidas, &l.tempos, &l.contagens, &l.deposito], &dados.tasks),
            None => ListasDeMissao::default(),
        };
        let itens = self.itens().await;
        let bolsa = itens.list_by_container(roleid, ContainerType::Inventory).await.map_err(|_| "banco_indisponivel")?;
        let de_missao = itens.list_by_container(roleid, ContainerType::TaskInventory).await.map_err(|_| "banco_indisponivel")?;
        let mut j = JogadorOffline {
            dados: &dados,
            nivel: ficha.level.max(0) as u32,
            classe: ficha.cls as u32,
            masculino: ficha.gender != pw_core::Gender::Female,
            cultivo: ficha.cultivation.max(0) as u32,
            reputacao: ficha.reputation,
            dinheiro: ficha.money.clamp(0, u32::MAX as i64) as u32,
            bolsa: Bolsa::nova(roleid, ContainerType::Inventory, TAMANHO_DA_BOLSA, bolsa),
            bolsa_de_missao: Bolsa::nova(roleid, ContainerType::TaskInventory, TAMANHO_DA_BOLSA_DE_MISSAO, de_missao),
            exige_entidade: None,
            bolsa_cheia: false,
        };
        let r = aplicar(&mut Motor { tarefas: &dados.tasks, listas: &mut listas, j: &mut j, eu: roleid as u32 }, &e)?;
        if let Some(o_que) = j.exige_entidade {
            info!(roleid, ?e, o_que, "painel: missão offline recusada (precisa da entidade)");
            return Err("precisa_estar_online");
        }
        if j.bolsa_cheia {
            return Err("bolsa_cheia");
        }
        j.bolsa.gravar(&itens).await.map_err(|_| "banco_indisponivel")?;
        j.bolsa_de_missao.gravar(&itens).await.map_err(|_| "banco_indisponivel")?;
        let [a, b, c, d, dep] = listas.blocos();
        let novas = pw_storage::ListasDeMissaoGravadas { ativa: a, concluidas: b, tempos: c, contagens: d, deposito: dep };
        repo.task_lists().gravar(roleid, &novas).await.map_err(|_| "banco_indisponivel")?;
        info!(roleid, ?e, concluida = r, "painel: missão (offline)");
        Ok(r)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_edicao_de_missao_e_lida_e_conferida() {
        let e: EdicaoDeMissao = serde_json::from_str(r#"{"acao":"dar","id":1173,"sub":1175}"#).unwrap();
        assert_eq!((e.acao, e.id, e.sub), (AcaoDeMissao::Dar, 1173, Some(1175)));
        assert!(e.valida());
        assert!(!EdicaoDeMissao { acao: AcaoDeMissao::Concluir, id: 70_000, sub: None }.valida());
        assert!(serde_json::from_str::<EdicaoDeMissao>(r#"{"acao":"apagar","id":1}"#).is_err());
        assert!(serde_json::from_str::<EdicaoDeMissao>(r#"{"acao":"dar","id":1,"x":1}"#).is_err());
    }
}
