//! Produção no NPC (serviço 12, `GP_NPCSEV_MAKE`) — B145.
//!
//! Porte de três peças do original, iguais no 1.5.5 e no `gs` 1.2.6:
//!
//! 1. **O pedido** — `produce_executor::SendRequest` + `produce_provider::TryServe`
//!    (`cgame/gs/serviceprovider.cpp:1451-1624`): `{ int skill; int id; size_t count }` (12 B).
//!    A receita tem de existir, não ser de melhoria (`equipment_need_upgrade`), ser da
//!    habilidade pedida, e essa habilidade no nível `require_level`; sem a taxa,
//!    `ERR_OUT_OF_FUND` (16). O NPC confere a habilidade dele e a receita na lista, senão
//!    `ERR_SKILL_NOT_AVAILABLE` (20).
//! 2. **A sessão** — `session_produce` (`actsession.cpp:657-700`): `PRODUCE_START(use_time,
//!    count, receita)`, um `ProduceItem` a cada `use_time` tiques de 50 ms até acabar a
//!    quantidade ou um falhar, e `PRODUCE_END`. `CANCEL_ACTION` a encerra.
//! 3. **Um item** — `gplayer_imp::ProduceItem` (`gs/player.cpp:16404-16641`): taxa, slot livre
//!    (`ERR_INVENTORY_IS_FULL` 7), sorteio (`RandUniform() > null_prob` → um dos quatro alvos pela
//!    probabilidade), materiais pela ordem dos slots (`ERR_NOT_ENOUGH_MATERIAL` 25); saindo
//!    item, a experiência como a da mina (nível 150 na punição) e a proficiência (+2 se a
//!    habilidade está abaixo do nível da receita, +1 se igual); taxa com `SPEND_MONEY`,
//!    materiais com `PLAYER_DROP_ITEM` tipo `DROP_TYPE_PRODUCE` (7), o item gerado e
//!    `PRODUCE_ONCE`; sem item, `PRODUCE_NULL` (210).

use super::jogo::erro_s2c;
use super::*;
use crate::bus_server::jogo::Contexto;
use pw_data_loader::receitas::{self, Receita};

/// `DROP_TYPE_PRODUCE` (`common/protocol.h:936`).
const DESCARTE_DA_PRODUCAO: u8 = 7;
/// `ERR_SKILL_NOT_AVAILABLE`, `ERR_NOT_ENOUGH_MATERIAL` (`common/protocol.h:700`, `:705`).
const HABILIDADE_INDISPONIVEL: i32 = 20;
const MATERIAL_INSUFICIENTE: i32 = 25;

/// `GetMaxAbility` dos stubs das habilidades de produção (`cskill/skills/skill158.h:88-92`,
/// `159`, `160`, `161`, `1402`; `164` e `165` têm tabelas próprias). 0 = sem teto de
/// proficiência (a habilidade não a usa).
pub fn teto_de_proficiencia(habilidade: u32, nivel: u8) -> i32 {
    const ARTESAO: [i32; 10] = [10, 20, 30, 45, 60, 80, 100, 120, 150, 200];
    const B164: [i32; 6] = [10, 20, 30, 40, 50, 60];
    const B165: [i32; 5] = [10, 20, 30, 40, 50];
    let i = (nivel.max(1) - 1) as usize;
    match habilidade {
        158..=161 | 1402 => ARTESAO.get(i).copied().unwrap_or(0),
        164 => B164.get(i).copied().unwrap_or(0),
        165 => B165.get(i).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `SkillWrapper::IncAbility` (`skillwrapper.cpp:1325-1350`): soma até o teto do nível;
/// `None` quando já estava no teto (nada muda, nada vai ao cliente).
pub fn subir_proficiencia(atual: i32, teto: i32, quanto: i32) -> Option<i32> {
    if teto <= 0 || atual >= teto {
        return None;
    }
    Some((atual + quanto).min(teto))
}

/// O que um `ProduceItem` decidiu, para o que acontece fora do contexto.
struct Produzido {
    continuar: bool,
    proficiencia: Option<(u32, i32)>,
}

impl BusServer {
    /// `GP_NPCSEV_MAKE` (12): confere o pedido e abre a sessão de produção.
    pub(super) async fn produzir(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let (Ok(habilidade), Ok(receita_id), Ok(quantidade)) = (r.i32(), r.i32(), r.u32()) else {
            warn!("mundo: pedido de produção de {roleid} curto ({} B)", conteudo.len());
            return;
        };
        let checado = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            let dados = &mundo.data_manager;
            let Some(rt) = dados.receitas.get(&(receita_id as u32)).cloned() else {
                debug!("mundo: {roleid} pediu a receita {receita_id}, que não existe");
                return;
            };
            // `produce_executor::SendRequest`: as recusas caladas (`return false`) e a do dinheiro.
            let nivel = p.habilidades.get(&(rt.habilidade.max(0) as u32)).copied().unwrap_or(0) as i32;
            if rt.melhoria > 0 || rt.habilidade != habilidade || (rt.habilidade > 0 && nivel < rt.nivel_exigido) {
                debug!("mundo: {roleid} não pode produzir {receita_id} (habilidade {habilidade}, nível {nivel})");
                return;
            }
            if p.money < rt.taxa {
                Err(erro_s2c::SEM_DINHEIRO)
            } else {
                // `produce_provider::TryServe`: a habilidade do NPC e a receita na lista dele.
                let npc = p.npc_em_conversa.and_then(|id| mundo.npcs.get(&id)).map(|n| n.template_id);
                let servico = npc.and_then(|t| dados.producao_do_npc.get(&t));
                match servico {
                    Some(s) if s.habilidade == habilidade && s.receitas.contains(&(receita_id as u32)) => Ok(rt),
                    _ => Err(HABILIDADE_INDISPONIVEL),
                }
            }
        };
        let rt = match checado {
            Ok(rt) => rt,
            Err(e) => {
                self.responder(roleid, S2CGamedataSend::error_message(e).data, envio).await;
                return;
            }
        };
        let quantidade = quantidade.clamp(1, u16::MAX as u32);
        let marcador = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            p.producao += 1;
            p.produzindo = Some(p.producao);
            p.producao
        };
        info!("mundo: {roleid} começou a produzir {} × {quantidade} ({} tiques)", rt.id, rt.tempo_em_tiques);
        let inicio = S2CGamedataSend::produce_start(
            rt.tempo_em_tiques.min(u16::MAX as u32) as u16,
            quantidade as u16,
            rt.id as i32,
        );
        self.responder(roleid, inicio.data, envio).await;
        let Some(este) = self.clone_arc() else { return };
        tokio::spawn(async move {
            let mut faltam = quantidade;
            while faltam > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(rt.tempo_em_tiques as u64 * 50)).await;
                if !este.ainda_produzindo(roleid, marcador).await {
                    return;
                }
                faltam -= 1;
                if !este.produzir_um(roleid, &rt).await {
                    break;
                }
            }
            este.encerrar_producao(roleid, marcador).await;
        });
    }

    async fn ainda_produzindo(&self, roleid: i32, marcador: u64) -> bool {
        self.world.read().await.players.get(&(roleid as i64)).is_some_and(|p| p.produzindo == Some(marcador))
    }

    /// `session_produce::EndSession`: `PRODUCE_END`. Com `marcador` 0, encerra a que houver
    /// (o `CANCEL_ACTION`).
    pub(super) async fn encerrar_producao(&self, roleid: i32, marcador: u64) {
        let encerrou = {
            let mut mundo = self.world.write().await;
            match mundo.players.get_mut(&(roleid as i64)) {
                Some(p) if p.produzindo.is_some() && (marcador == 0 || p.produzindo == Some(marcador)) => {
                    p.produzindo = None;
                    true
                }
                _ => false,
            }
        };
        if encerrou {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::produce_end().data).await;
        }
    }

    /// Um `gplayer_imp::ProduceItem`. `false` encerra a sessão.
    async fn produzir_um(&self, roleid: i32, rt: &Receita) -> bool {
        use rand::Rng;
        let (sorteio_nada, sorteio_alvo) = {
            let mut g = rand::thread_rng();
            (g.gen::<f32>(), g.gen::<f32>())
        };
        let item_id = if sorteio_nada > rt.chance_de_nada {
            receitas::escolher_alvo(&rt.alvos, sorteio_alvo)
        } else {
            0
        };
        let feito = self
            .com_contexto(roleid, |ctx| Self::produzir_no_contexto(ctx, rt, item_id))
            .await;
        let Some(feito) = feito else { return false };
        if let Some((habilidade, proficiencia)) = feito.proficiencia {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::skill_ability(habilidade as i32, proficiencia).data)
                .await;
            let personagens = self.repo().await;
            if let Err(e) = personagens.skill_repo().gravar_proficiencia(roleid, habilidade, proficiencia).await {
                warn!("mundo: não gravei a proficiência {habilidade} de {roleid}: {e}");
            }
        }
        feito.continuar
    }

    fn produzir_no_contexto(ctx: &mut Contexto, rt: &Receita, item_id: i32) -> Produzido {
        let falhou = Produzido { continuar: false, proficiencia: None };
        let erro = |ctx: &mut Contexto, e: i32| ctx.para_mim.push(S2CGamedataSend::error_message(e).data);
        if ctx.p.money < rt.taxa {
            erro(ctx, erro_s2c::SEM_DINHEIRO);
            return falhou;
        }
        if ctx.bolsa.livres() == 0 {
            erro(ctx, erro_s2c::BOLSA_CHEIA);
            return falhou;
        }
        if rt.materiais.iter().any(|&(m, n)| ctx.bolsa.contar(m) < n) {
            erro(ctx, MATERIAL_INSUFICIENTE);
            return falhou;
        }
        let mut proficiencia = None;
        if item_id > 0 {
            // `GM_MSG_EXPERIENCE {level 150, exp, sp}` → `ReceiveExp` com a punição de nível.
            if rt.exp != 0 || rt.sp != 0 {
                let a = ctx.dados.progressao.ajuste(ctx.p.level - receitas::NIVEL_DA_RECEITA);
                let exp = (rt.exp.max(0) as f32 * a.exp + 0.5) as i64;
                let sp = (rt.sp.max(0) as f32 * a.sp + 0.5) as i64;
                if exp + sp > 0 {
                    ctx.ganhar_exp(exp, sp);
                    ctx.para_mim.push(ctx.sub.receive_exp(exp as i32, sp as i32).data);
                }
            }
            if rt.habilidade > 0 {
                let h = rt.habilidade as u32;
                let nivel = ctx.p.habilidades.get(&h).copied().unwrap_or(0);
                let quanto = match (nivel as i32).cmp(&rt.nivel_da_receita) {
                    std::cmp::Ordering::Less => 2,
                    std::cmp::Ordering::Equal => 1,
                    std::cmp::Ordering::Greater => 0,
                };
                let atual = ctx.p.proficiencias.get(&h).copied().unwrap_or(0);
                if quanto > 0 {
                    if let Some(nova) = subir_proficiencia(atual, teto_de_proficiencia(h, nivel), quanto) {
                        ctx.p.proficiencias.insert(h, nova);
                        proficiencia = Some((h, nova));
                    }
                }
            }
        }
        if rt.taxa > 0 {
            ctx.gastar_dinheiro(rt.taxa);
            ctx.para_mim.push(S2CGamedataSend::spend_money(rt.taxa as u32).data);
        }
        for &(m, n) in &rt.materiais {
            for (slot, saiu) in ctx.bolsa.tirar(m, n) {
                ctx.para_mim.push(
                    S2CGamedataSend::player_drop_item(0, slot as u8, saiu, m as i32, DESCARTE_DA_PRODUCAO).data,
                );
            }
        }
        if item_id <= 0 {
            ctx.para_mim.push(S2CGamedataSend::produce_null(rt.id as i32).data);
            return Produzido { continuar: true, proficiencia };
        }
        let dados = ctx.dados;
        match ctx.bolsa.empilhar_gerado(item_id as u32, rt.quantidade.max(1), dados) {
            Some(e) => {
                let pacote = ctx.sub.produce_once(item_id, e.entrou, e.no_slot, 0, e.slot as u8);
                ctx.para_mim.push(pacote.data);
                Produzido { continuar: true, proficiencia }
            }
            None => {
                erro(ctx, erro_s2c::BOLSA_CHEIA);
                Produzido { continuar: false, proficiencia }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proficiencia_para_no_teto_do_nivel() {
        assert_eq!(teto_de_proficiencia(158, 1), 10);
        assert_eq!(teto_de_proficiencia(158, 10), 200);
        assert_eq!(teto_de_proficiencia(165, 5), 50);
        assert_eq!(subir_proficiencia(9, 10, 2), Some(10));
        assert_eq!(subir_proficiencia(10, 10, 1), None, "no teto não muda nem avisa");
        assert_eq!(subir_proficiencia(0, 0, 1), None, "habilidade sem proficiência");
    }
}
