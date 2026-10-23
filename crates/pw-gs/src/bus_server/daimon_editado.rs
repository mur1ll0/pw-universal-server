//! Painel (E6, B215): o Daimon (o "pequeno elfo" do original, `elf_item`) do personagem.
//!
//! - **Ver:** a ficha do bloco do item no slot 23 (`EQUIP_INDEX_ELF`, `gs/item.h:219`) — em jogo a
//!   da memória ([`crate::entity::DaimonVestido`]), fora dele a do banco (`Daimon::ler`).
//! - **Dar EXP:** pela regra do `elf_item::InsertExp` (`gs/item/item_elf.cpp:692-750`,
//!   [`crate::entity::DaimonVestido::receber_exp`]), com o nível do dono como "nível de quem deu" —
//!   o mesmo que o jogo usa para a parte que o Daimon recebe do jogador. Sobe quantos níveis
//!   couberem e **para a um ponto** de passar o dono. Em jogo vai `ELF_EXP` (283) ou, subindo, a
//!   ficha inteira do item (`daimon_recebe`); fora do jogo grava o bloco.

use super::*;
use crate::entity::{Daimon, DaimonVestido};
use pw_core::{ContainerType, ItemRecord};
use pw_data_loader::GameDataManager;

/// A ficha que o painel mostra.
pub(crate) fn ficha_do_daimon(d: &DaimonVestido, nome: Option<&String>, exp_para_subir: u32, origem: &str) -> serde_json::Value {
    let e = &d.estado;
    serde_json::json!({
        "item_id": d.item_id, "nome": nome, "origem": origem,
        "nivel": e.nivel, "exp": e.exp, "exp_para_subir": exp_para_subir,
        "pontos_livres": e.total_de_atributos,
        "forca": e.forca, "agilidade": e.agilidade, "vitalidade": e.vitalidade, "energia": e.energia,
        "pontos_de_genio": e.total_de_genios, "genios": e.genios,
        "refino": e.refino, "vigor": e.vigor,
        "habilidades": e.habilidades.iter().map(|(id, n)| [*id as i32, *n as i32]).collect::<Vec<_>>(),
        "equipamento": e.equipamento,
    })
}

/// A experiência que o Daimon precisa para subir do nível atual: a do jogador × `exp_factor`
/// (`item_elf.cpp:696-710`).
fn exp_para_subir(d: &DaimonVestido, dados: &GameDataManager) -> u32 {
    (dados.progressao.exp_para_subir(d.estado.nivel as i32).clamp(0, u32::MAX as i64) as f64 * d.fator_de_exp as f64) as u32
}

/// O Daimon de um item do slot 23 lido do banco.
fn do_item(item: &ItemRecord, dados: &GameDataManager) -> Option<DaimonVestido> {
    let (fator, iniciais) = dados.dados_do_daimon(item.item_id)?;
    Some(DaimonVestido {
        slot: item.slot,
        item_id: item.item_id,
        fator_de_exp: fator,
        estado: Daimon::ler(&item.octets).unwrap_or_else(|| Daimon::novo(&iniciais)),
        sujo: false,
        sorteio: dados.sorteio_do_daimon(item.item_id),
    })
}

impl BusServer {
    /// A ficha do Daimon: a da memória quando o dono está neste mapa, senão a do banco.
    pub(crate) async fn daimon_do_painel(&self, roleid: i32) -> serde_json::Value {
        let (dados, vivo) = {
            let mundo = self.world.read().await;
            let vivo = mundo.players.get(&(roleid as i64)).map(|p| p.daimon.clone());
            (mundo.data_manager.clone(), vivo)
        };
        if !dados.versao_tem_daimon() {
            // 1.2.6: o `elements.data` v7 não tem `GOBLIN_ESSENCE` — a versão não tem Daimon (B218).
            return serde_json::json!({"daimon": null, "existe_na_versao": false});
        }
        let (d, origem) = match vivo {
            Some(d) => (d, "em_jogo"),
            None => {
                let itens = match self.itens().await.list_by_container(roleid, ContainerType::Equipment).await {
                    Ok(v) => v,
                    Err(_) => return serde_json::json!({"codigo":"banco_indisponivel"}),
                };
                (itens.iter().find(|i| i.slot == jogo::SLOT_DO_DAIMON).and_then(|i| do_item(i, &dados)), "persistida")
            }
        };
        match d {
            None => serde_json::json!({"daimon": null, "origem": origem, "existe_na_versao": true}),
            Some(d) => {
                let mut ficha = ficha_do_daimon(&d, dados.nomes_de_itens.get(&d.item_id), exp_para_subir(&d, &dados), origem);
                if let (Some(f), Some(x)) = (ficha.as_object_mut(), extras_do_painel(&d, &dados).as_object()) {
                    f.extend(x.clone());
                }
                serde_json::json!({"existe_na_versao": true, "daimon": ficha})
            }
        }
    }

    /// Offline (com a guarda de presença e a trava de gravação de quem chama): `InsertExp` no bloco
    /// do banco. Devolve `(nível antes, nível depois)`.
    pub(crate) async fn daimon_exp_offline(&self, roleid: i32, exp: i64) -> Result<(i16, i16), &'static str> {
        let dados = self.world.read().await.data_manager.clone();
        let repo = self.itens().await;
        let mut item = repo.list_by_container(roleid, ContainerType::Equipment).await.map_err(|_| "banco_indisponivel")?
            .into_iter().find(|i| i.slot == jogo::SLOT_DO_DAIMON).ok_or("sem_daimon")?;
        let mut d = do_item(&item, &dados).ok_or("sem_daimon")?;
        let nivel_do_dono = self.repo().await.get_details_por_role(roleid).await.map_err(|_| "banco_indisponivel")?
            .ok_or("personagem_inexistente")?.level.clamp(0, i16::MAX as i32) as i16;
        let antes = d.estado.nivel;
        let tabela = &dados.progressao;
        let (ganhou, _) = d.receber_exp(exp.clamp(0, u32::MAX as i64) as u32, nivel_do_dono, nivel_do_dono, |n| {
            tabela.exp_para_subir(n as i32).clamp(0, u32::MAX as i64) as u32
        });
        if !ganhou {
            return Err("sem_mudanca");
        }
        item.octets = d.estado.bloco();
        repo.upsert_item(&item).await.map_err(|_| "banco_indisponivel")?;
        info!(roleid, exp, antes, depois = d.estado.nivel, "painel: EXP do Daimon (offline)");
        Ok((antes, d.estado.nivel))
    }
}

/// Painel (B361): a edição do Daimon vestido — os atributos e gênios distribuídos, ou uma habilidade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "acao", rename_all = "snake_case", deny_unknown_fields)]
pub enum EdicaoDeDaimon {
    /// Os pontos distribuídos: `[força, agilidade, vitalidade, energia]` e os cinco gênios (metal, madeira, água, fogo,
    /// terra), cada um no valor final.
    Atributos { atributos: [i16; 4], genios: [i16; 5] },
    /// A habilidade `id` no `nivel` (0 = esquecer).
    Habilidade { id: u16, nivel: i16 },
}

/// B361 — aplica a edição com as regras que o original confere ao carregar o item (`elf_item::Load`, a validação de
/// `item_elf.cpp:117`) e ao distribuir/aprender: o custo dos atributos (acima de 40 cada ponto vale dois) até o
/// `total_attribute`; cada gênio até 8 **contando as peças** (`GENIUS_UPPER_LIMIT`) e a soma até o `total_genius`;
/// habilidade só de Daimon (`cls` 258), até o nível máximo dela, e nova só com vaga (`cur_skill_slot`, B345). Devolve o
/// que havia antes, para o registro.
pub(crate) fn editar_daimon(d: &mut DaimonVestido, e: EdicaoDeDaimon, dados: &GameDataManager) -> Result<serde_json::Value, &'static str> {
    let iniciais = dados.atributos_iniciais_do_daimon(d.item_id).unwrap_or([0; 4]);
    let atuais = d.estado.atuais(iniciais, |id| dados.equipamento_de_daimon(id));
    match e {
        EdicaoDeDaimon::Atributos { atributos, genios } => {
            if atributos.iter().chain(genios.iter()).any(|v| *v < 0) {
                return Err("valor_negativo");
            }
            let custo: i32 = atributos
                .iter()
                .map(|v| {
                    let t = *v as i32;
                    if t > Daimon::LIMITE_DE_ATRIBUTO {
                        t + t - Daimon::LIMITE_DE_ATRIBUTO
                    } else {
                        t
                    }
                })
                .sum();
            if custo > d.estado.total_de_atributos as i32 {
                return Err("atributos_acima_do_total");
            }
            // O que as peças somam aos gênios: o atual menos o distribuído.
            let das_pecas: [i16; 5] = std::array::from_fn(|k| atuais.genios[k] - d.estado.genios[k]);
            if (0..5).any(|k| genios[k] + das_pecas[k] > Daimon::LIMITE_DE_GENIO) {
                return Err("genio_acima_do_limite");
            }
            if genios.iter().map(|v| *v as i32).sum::<i32>() > d.estado.total_de_genios as i32 {
                return Err("genios_acima_do_total");
            }
            let e = &mut d.estado;
            if [e.forca, e.agilidade, e.vitalidade, e.energia] == atributos && e.genios == genios {
                return Err("sem_mudanca");
            }
            let antes = serde_json::json!({"atributos": [e.forca, e.agilidade, e.vitalidade, e.energia], "genios": e.genios});
            [e.forca, e.agilidade, e.vitalidade, e.energia] = atributos;
            e.genios = genios;
            d.sujo = true;
            Ok(antes)
        }
        EdicaoDeDaimon::Habilidade { id, nivel } => {
            let atual = d.estado.habilidades.iter().find(|(h, _)| *h == id).map(|(_, n)| *n).unwrap_or(0);
            if nivel == atual {
                return Err("sem_mudanca");
            }
            if nivel == 0 {
                d.estado.habilidades.retain(|(h, _)| *h != id);
            } else {
                let h = dados.habilidades.get(id as u32).ok_or("habilidade_inexistente")?;
                if h.cls != Some(258) {
                    return Err("nao_e_de_daimon");
                }
                if nivel < 0 || nivel as i32 > h.max_level {
                    return Err("nivel_invalido");
                }
                if atual == 0 && d.estado.habilidades.len() >= atuais.vagas {
                    return Err("sem_vaga");
                }
                d.estado.aprender(id, nivel);
            }
            d.sujo = true;
            Ok(serde_json::json!({"habilidade": id, "nivel_antes": atual}))
        }
    }
}

/// B361 — o que a janela precisa além do bloco: os atributos de agora (com as peças), as vagas de habilidade e as
/// habilidades de Daimon que existem (para ensinar).
pub(crate) fn extras_do_painel(d: &DaimonVestido, dados: &GameDataManager) -> serde_json::Value {
    let iniciais = dados.atributos_iniciais_do_daimon(d.item_id).unwrap_or([0; 4]);
    let a = d.estado.atuais(iniciais, |id| dados.equipamento_de_daimon(id));
    let mut disponiveis: Vec<(u32, i32)> =
        dados.habilidades.por_id.values().filter(|h| h.cls == Some(258)).map(|h| (h.id, h.max_level)).collect();
    disponiveis.sort_unstable();
    serde_json::json!({
        "atributos_atuais": a.atributos, "genios_atuais": a.genios, "vagas": a.vagas,
        "iniciais": iniciais, "limite_de_atributo": Daimon::LIMITE_DE_ATRIBUTO, "limite_de_genio": Daimon::LIMITE_DE_GENIO,
        "habilidades_de_daimon": disponiveis.iter().map(|(id, m)| [*id as i32, *m]).collect::<Vec<_>>(),
    })
}

impl BusServer {
    /// B361 — online: no Daimon vestido em memória, a ficha do item vai ao cliente (`PlayerGetItemInfo`) e o bloco é
    /// gravado com o resto (`sujo`).
    pub(crate) async fn daimon_pelo_painel(&self, roleid: i32, e: EdicaoDeDaimon) -> Option<serde_json::Value> {
        let feito = self
            .com_contexto(roleid, |ctx| -> Result<serde_json::Value, &'static str> {
                let dados = ctx.dados;
                let d = ctx.p.daimon.as_mut().ok_or("sem_daimon")?;
                let antes = editar_daimon(d, e, dados)?;
                super::daimon::ficha_do_vestido(ctx);
                Ok(antes)
            })
            .await?;
        Some(match feito {
            Ok(antes) => {
                info!(roleid, ?e, "painel: Daimon editado (online)");
                serde_json::json!({"erro": null, "daimon": e, "antes": antes})
            }
            Err(codigo) => serde_json::json!({"erro": codigo}),
        })
    }

    /// B361 — offline (com a guarda de presença e a trava de gravação de quem chama): o bloco do banco.
    pub(crate) async fn daimon_offline(&self, roleid: i32, e: EdicaoDeDaimon) -> Result<serde_json::Value, &'static str> {
        let dados = self.world.read().await.data_manager.clone();
        let repo = self.itens().await;
        let mut item = repo
            .list_by_container(roleid, ContainerType::Equipment)
            .await
            .map_err(|_| "banco_indisponivel")?
            .into_iter()
            .find(|i| i.slot == jogo::SLOT_DO_DAIMON)
            .ok_or("sem_daimon")?;
        let mut d = do_item(&item, &dados).ok_or("sem_daimon")?;
        let antes = editar_daimon(&mut d, e, &dados)?;
        item.octets = d.estado.bloco();
        repo.upsert_item(&item).await.map_err(|_| "banco_indisponivel")?;
        info!(roleid, ?e, "painel: Daimon editado (offline)");
        Ok(antes)
    }
}

#[cfg(test)]
mod testes_da_edicao {
    use super::*;

    fn vestido() -> DaimonVestido {
        let mut estado = Daimon::novo(&[]);
        estado.nivel = 50;
        estado.total_de_atributos = 49;
        estado.total_de_genios = 11;
        DaimonVestido { slot: 23, item_id: 1, fator_de_exp: 1.0, estado, sujo: false, sorteio: [(0, 0.0); 10] }
    }

    /// B361 — atributos pelo custo do original (acima de 40 cada ponto vale dois) e gênios pelo total e pelo limite 8.
    #[test]
    fn atributos_e_genios_pelas_regras_do_original() {
        let dados = GameDataManager::new();
        let mut d = vestido();
        let pedir = |a: [i16; 4], g: [i16; 5]| EdicaoDeDaimon::Atributos { atributos: a, genios: g };
        // 41 custa 42; 42 + 7 = 49 cabe.
        assert!(editar_daimon(&mut d, pedir([41, 7, 0, 0], [0; 5]), &dados).is_ok());
        assert_eq!((d.estado.forca, d.estado.agilidade, d.sujo), (41, 7, true));
        assert_eq!(editar_daimon(&mut d, pedir([41, 8, 0, 0], [0; 5]), &dados), Err("atributos_acima_do_total"));
        assert_eq!(editar_daimon(&mut d, pedir([41, 7, 0, 0], [9, 0, 0, 0, 0]), &dados), Err("genio_acima_do_limite"));
        assert_eq!(editar_daimon(&mut d, pedir([41, 7, 0, 0], [8, 4, 0, 0, 0]), &dados), Err("genios_acima_do_total"));
        assert!(editar_daimon(&mut d, pedir([41, 7, 0, 0], [8, 3, 0, 0, 0]), &dados).is_ok());
        assert_eq!(editar_daimon(&mut d, pedir([41, 7, 0, 0], [8, 3, 0, 0, 0]), &dados), Err("sem_mudanca"));
        assert_eq!(editar_daimon(&mut d, pedir([-1, 0, 0, 0], [0; 5]), &dados), Err("valor_negativo"));
    }

    /// B361 — habilidade: só de Daimon (`cls` 258), até o nível máximo, nova só com vaga; nível 0 esquece.
    #[test]
    fn habilidade_de_daimon_pelas_regras() {
        let mut dados = GameDataManager::new();
        dados.habilidades =
            pw_data_loader::habilidades::TabelaDeHabilidades::de_json(pw_data_loader::habilidades::json_do_servidor("1.5.5").unwrap())
                .unwrap();
        let mut d = vestido();
        let pedir = |id: u16, nivel: i16| EdicaoDeDaimon::Habilidade { id, nivel };
        assert!(editar_daimon(&mut d, pedir(1004, 3), &dados).is_ok());
        assert_eq!(d.estado.habilidades, vec![(1004, 3)]);
        assert_eq!(editar_daimon(&mut d, pedir(1004, 11), &dados), Err("nivel_invalido"), "a 1004 vai até o 10");
        assert_eq!(editar_daimon(&mut d, pedir(1, 1), &dados), Err("nao_e_de_daimon"));
        assert!(editar_daimon(&mut d, pedir(1004, 0), &dados).is_ok());
        assert!(d.estado.habilidades.is_empty());
    }
}
