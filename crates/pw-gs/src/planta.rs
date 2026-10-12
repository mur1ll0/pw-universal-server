//! Planta invocada (`PET_CLASS_PLANT`, B222) — as plantas do Místico, da Feiticeira de seita e das
//! classes novas (`SetSummonplantpet`, `playerwrapper.cpp:3991-3996`).
//!
//! Porte de três peças do original:
//!
//! 1. **O dono** — `gplayer_imp::SummonPlantPet` (`gs/player.cpp:13450-13486`) e o
//!    `plant_pet_manager` (`gs/petman.cpp:2257-2560`, `petman.h:354-392`): a planta nasce junto do
//!    alvo (ou do dono), com o nível do dono e as habilidades do ovo no nível da habilidade que a
//!    invocou; a primeira habilidade decide se ela é **automática** (ataque/maldição: luta) ou
//!    **passiva** (bênção: cura o grupo); passou do `group_limit` do grupo dela, a mais antiga
//!    some. A vida conta em segundos (`ppd.life`, 0 sem fim).
//! 2. **O corpo** — `npc_spawner::CreatePetBase2` (`npcgenerator.cpp:2141-2250`) com o
//!    `pet_dataman::GenerateBaseProp2` (`petdataman.cpp`) e o `gplayer_imp::CalcPetEnhance`
//!    (`player.cpp:13488-13513`): `5 × nível da habilidade`% das parcelas do dono
//!    ([`crate::entity::BaseParaMascote`]); nasce com a vida cheia e metade da mana.
//! 3. **A planta no mundo** — `gpet_plant_imp`/`gpet_plant_policy` (`petnpc.cpp:1871-2080`):
//!    não segue o dono, some longe dele, a automática junta ódio +1 em todo monstro vivo dentro
//!    da visão quando está fora de combate (`gpet_policy::GatherTarget` + `search_target`,
//!    `petnpc.cpp:1614-1627`, `guardnpc.h:15-41`) e a passiva lança a habilidade automática no
//!    aliado mais próximo (`PetGetNearestTeammate`, `petnpc.cpp:361-392`). O aviso de vida e
//!    mana ao dono sai quando muda (`NotifyMasterInHeartbeat` + `NotifyPlantHP`).
//!
//! Os avisos ao cliente: `SUMMON_PLANT_PET` (293), `PLANT_PET_DISAPPEAR` (294) com o motivo e
//! `PLANT_PET_HP_NOTIFY` (295).

use crate::entity::{BaseParaMascote, MonsterEntity};
use crate::world::{EventoDoMundo, WorldInstance};
use pw_core::Vector3;
use pw_data_loader::pet::ModeloDeMascote;

/// `plant_pet_manager::PLANT_*` (`gs/petman.h:363-370`): o motivo do `PLANT_PET_DISAPPEAR`.
pub const MORREU: u8 = 0;
pub const VIDA_ACABOU: u8 = 1;
pub const FORA_DO_ALCANCE: u8 = 2;
pub const EXPLODIU: u8 = 3;
pub const LIMITE_DO_GRUPO: u8 = 4;

/// `ERR_SUMMON_PET_INVALID_POS` (85, `common/protocol.h:765`).
pub const ERRO_POSICAO: i32 = 85;

/// O id do tipo de planta no `PET_ESSENCE` (`SummonPlantPet` recusa o resto, `player.cpp:13459`).
pub const TIPO_DE_PLANTA: i32 = 28913;

/// O que a planta tem além do mascote — o `plant_pet_data` do dono (`petman.h`).
#[derive(Debug, Clone, PartialEq)]
pub struct Planta {
    pub tid: i32,
    /// `ppd.life`: segundos que faltam; 0 = sem fim.
    pub vida_s: i32,
    pub grupo: i32,
    /// `ppd.is_suicide`: a segunda habilidade do ovo, a da explosão (`InitSkill`, `petnpc.cpp:1884-1889`).
    pub suicida: Option<(i32, i32)>,
    /// A ordem de invocação: o `group_limit` tira a mais antiga.
    pub ordem: u64,
    /// Explodiu: o dono já a esqueceu e ela some 0,5 s depois (`LazySendTo<0>(GM_MSG_PET_DISAPPEAR,
    /// ..., TICK_PER_SEC/2)`, `petnpc.cpp:1983`).
    pub sumir_em_ms: Option<u32>,
    /// `hp`/`mp` do último `PLANT_PET_HP_NOTIFY` (`NotifyPlantHP` só manda o que mudou).
    pub avisado: Option<(i64, i32)>,
    pub batimento_ms: u32,
}

impl Planta {
    /// O dono a conta (grupo, explosão, avisos)?
    pub fn ativa(&self) -> bool {
        self.sumir_em_ms.is_none()
    }
}

/// `GenerateBaseProp2(tid, nível, nível da habilidade)` + `CalcPetEnhance` (`petdataman.cpp`,
/// `player.cpp:13488-13513`): vida, mana, regeneração, dano (físico e mágico iguais), acerto,
/// defesa, esquiva, resistências e graus no nível do dono, e `5 × nível`% das parcelas dele. O
/// `_pet_enhance` dos addons do dono (`pet_enhance`) ainda não é lido: vale zero.
pub fn corpo_da_planta(
    id: i64,
    modelo: &ModeloDeMascote,
    nivel: i32,
    nivel_da_habilidade: i32,
    dono: &BaseParaMascote,
    pos: Vector3,
) -> MonsterEntity {
    let a = modelo.atributos(nivel);
    let pct = 5 * nivel_da_habilidade;
    // `(int)(base × 0.01f × pct)` no x87 do original: sem arredondar a `float` no meio — em dupla,
    // com o `0.01f` (1000 × 0.01f × 20 dá 199, não os 200 da conta em `f32`).
    let mais = |base: i32| (base as f64 * 0.01f32 as f64 * pct as f64) as i32;
    let max_hp = (a.vida + mais(dono.vida)).max(1) as i64;
    let max_mp = (modelo.mana_maxima(nivel) + mais(dono.mana)).max(0);
    let dano = a.dano + mais(dono.dano_magico);
    let mut m = MonsterEntity::placeholder(id, modelo.tid, pos, 0);
    m.name = String::new();
    m.level = nivel;
    m.max_hp = max_hp;
    m.hp = max_hp;
    m.max_mp = max_mp;
    m.mp = max_mp / 2;
    m.def_phys = a.defesa + mais(dono.defesa);
    m.armor = a.esquiva;
    m.attack_rate = a.acerto;
    m.resistances = std::array::from_fn(|i| a.resistencia + mais(dono.resistencias[i]));
    m.attack_degree = modelo.grau_de_ataque_no(nivel) + mais(dono.grau_de_ataque);
    m.defend_degree = modelo.grau_de_defesa_no(nivel) + mais(dono.grau_de_defesa);
    m.attack_min = dano;
    m.attack_max = dano;
    m.magic_attack = [(dano, dano); 5];
    m.attack_range = a.alcance + modelo.corpo;
    m.tamanho = modelo.corpo;
    m.ataque_em_ticks = a.intervalo_do_golpe;
    m.atraso_do_dano_em_ticks = modelo.atraso_do_dano;
    m.aggro_range = 60.0;
    m.agressivo = false;
    m.sight_range = modelo.visao as i32;
    m.exp = 0;
    m.sp = 0;
    m.move_speed = a.correr;
    m.walk_speed = a.andar;
    m.habitat = crate::ai::Habitat::do_elements(modelo.habitat);
    m.patrulha = false;
    m
}

/// `GNET::SkillWrapper::GetType` da primeira habilidade (`ActivePlant`, `petman.cpp:2288-2296`):
/// ataque (1) e maldição (3) → automática; bênção (2) → passiva; outro tipo, não nasce.
pub fn agressiva_pelo_tipo(tipo: i32) -> Option<bool> {
    match tipo {
        1 | 3 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

impl WorldInstance {
    /// As plantas vivas de um dono, da mais antiga para a mais nova.
    pub fn plantas_de(&self, dono: i64) -> Vec<i64> {
        let mut v: Vec<(u64, i64)> = self
            .mascotes
            .iter()
            .filter_map(|(id, m)| {
                let p = m.planta.as_ref().filter(|p| p.ativa())?;
                (m.dono == dono).then_some((p.ordem, *id))
            })
            .collect();
        v.sort();
        v.into_iter().map(|(_, id)| id).collect()
    }

    /// `gplayer_imp::SummonPlantPet` + `plant_pet_manager::ActivePlant` (`player.cpp:13450-13486`,
    /// `petman.cpp:2257-2343`). `alvo` é o da habilidade (`target ? *target : XID(-1,-1)`); o
    /// próprio dono vale como sem alvo. `Err` com o erro a mandar ao dono (só a posição tem um).
    pub fn invocar_planta(
        &mut self,
        dono: i64,
        ovo: u32,
        nivel_da_habilidade: i32,
        vida_s: i32,
        alvo: Option<i64>,
    ) -> Result<i64, Option<i32>> {
        let dados = std::sync::Arc::clone(&self.data_manager);
        let Some(o) = dados.ovos_de_pet.get(&ovo) else { return Err(None) };
        let Some(modelo) = dados.modelos_de_mascote.get(&o.id_pet).filter(|m| m.classe == 4) else {
            return Err(None);
        };
        let Some(p) = self.players.get(&dono) else { return Err(None) };
        let nivel = p.level.max(1);
        let base = p.base_para_mascote;
        let camada = self.camada_do_jogador(p);
        // As habilidades do ovo, no nível da que invocou (`data.skills[i].level = skill_level`).
        let habilidades: Vec<i32> = o.skills.iter().map(|(s, _)| *s).take(8).collect();
        let Some(&primeira) = habilidades.first() else { return Err(None) };
        let tipo = dados.habilidades.get(primeira.max(0) as u32).and_then(|h| h.tipo).unwrap_or(0);
        let Some(agressiva) = agressiva_pelo_tipo(tipo) else { return Err(None) };
        // O lugar: junto do alvo (com o corpo dele) ou do dono (`FindValidPos`).
        let alvo = alvo.filter(|a| *a != dono);
        let (centro, corpo_do_alvo) = alvo
            .and_then(|a| {
                self.monsters
                    .get(&a)
                    .map(|(m, _)| (m.position, m.tamanho))
                    .or_else(|| self.players.get(&a).map(|q| (q.position, 0.0)))
            })
            .unwrap_or((p.position, 0.0));
        if modelo.limite_do_grupo <= 0 {
            return Err(Some(ERRO_POSICAO));
        }
        let Ok((pos, modo)) = crate::mascote::posicao_para_mascote(
            &self.terreno,
            &self.movimento,
            &self.agua,
            Some(&self.espaco),
            self.sem_voo,
            centro,
            camada,
            modelo.habitat,
            corpo_do_alvo,
            Vector3::new(0.0, 0.0, 0.0),
        ) else {
            return Err(Some(ERRO_POSICAO));
        };
        self.proximo_mascote = self.proximo_mascote.wrapping_add(1);
        let id = crate::mascote::id_do_mascote(self.proximo_mascote);
        let info = pw_core::InfoPet {
            pet_tid: o.id_pet as i32,
            pet_egg_tid: o.id as i32,
            pet_class: 4,
            level: nivel as i16,
            honor_point: 500,
            hp_factor: 1.0,
            skills: std::array::from_fn(|i| habilidades.get(i).map(|s| (*s, nivel_da_habilidade)).unwrap_or((0, 0))),
            ..Default::default()
        };
        let movimento = crate::mascote::MOVIMENTO_FICAR;
        let agressividade = if agressiva {
            crate::mascote::AGRESSIVIDADE_AUTOMATICA
        } else {
            crate::mascote::AGRESSIVIDADE_PASSIVA
        };
        let mut m = crate::mascote::Mascote::novo(id, dono, 0, info, modelo, pos, agressividade, movimento);
        m.corpo = corpo_da_planta(id, modelo, nivel, nivel_da_habilidade, &base, pos);
        m.corpo.habitat = modo;
        m.camada_do_dono = camada;
        m.ai.e_planta = true;
        m.ai.imovel = m.corpo.move_speed <= 0.0;
        // `InitSkill`: a primeira é a automática; a segunda, a da explosão.
        m.ai.automatica = crate::world::habilidade_do_mascote(&dados, primeira, nivel_da_habilidade, m.corpo.attack_range);
        let suicida = habilidades.get(1).map(|s| (*s, nivel_da_habilidade));
        // O limite do grupo: a mais antiga do mesmo grupo some (`PLANT_GROUP_LIMIT`).
        let do_grupo: Vec<i64> = self
            .plantas_de(dono)
            .into_iter()
            .filter(|i| self.mascotes.get(i).and_then(|x| x.planta.as_ref()).is_some_and(|x| x.grupo == modelo.grupo_de_planta))
            .collect();
        if do_grupo.len() as i32 >= modelo.limite_do_grupo {
            self.remover_planta(do_grupo[0], LIMITE_DO_GRUPO);
        }
        self.ordem_das_plantas = self.ordem_das_plantas.wrapping_add(1);
        m.planta = Some(Planta {
            tid: o.id_pet as i32,
            vida_s: vida_s.max(0),
            grupo: modelo.grupo_de_planta,
            suicida,
            ordem: self.ordem_das_plantas,
            sumir_em_ms: None,
            avisado: None,
            batimento_ms: 0,
        });
        // A automática ataca o alvo da invocação (`GM_MSG_PET_AUTO_ATTACK` → `AddToFirst(alvo, 2)`).
        if agressiva {
            if let Some(a) = alvo.filter(|a| self.monsters.get(a).is_some_and(|(x, _)| !x.is_dead)) {
                m.ai.por_no_topo(a, 2);
            }
        }
        self.grid.add_entity(id, pos, false);
        self.mascotes.insert(id, m);
        self.emitir(EventoDoMundo::PlantaApareceu {
            id,
            dono: dono as pw_core::RoleId,
            tid: o.id_pet as i32,
            vida_s: vida_s.max(0),
        });
        Ok(id)
    }

    /// Tira a planta do mundo e avisa o dono (`plant_pet_disappear(id, motivo)`); a quem vê, o
    /// `OBJECT_DISAPPEAR`. Morrer (motivo 0) é avisado por quem a matou.
    pub fn remover_planta(&mut self, id: i64, motivo: u8) {
        let Some(dono) = self.mascotes.get(&id).filter(|m| m.planta.is_some()).map(|m| m.dono) else {
            return;
        };
        let avisar = self.mascotes.get(&id).and_then(|m| m.planta.as_ref()).is_some_and(|p| p.ativa());
        self.tirar_mascote(id);
        if motivo != MORREU {
            self.emitir(EventoDoMundo::MonstroSumiu { id });
        }
        if avisar {
            self.emitir(EventoDoMundo::PlantaSumiu {
                id,
                dono: dono as pw_core::RoleId,
                motivo,
            });
        }
    }

    /// Todas as plantas do dono somem — sair do jogo ou trocar de servidor (`PreSwitchServer`,
    /// `petman.cpp:2430-2440`: `PLANT_OUT_OF_RANGE`).
    pub fn remover_plantas_de(&mut self, dono: i64) {
        let todas: Vec<i64> = self
            .mascotes
            .iter()
            .filter(|(_, m)| m.dono == dono && m.planta.is_some())
            .map(|(id, _)| *id)
            .collect();
        for id in todas {
            self.remover_planta(id, FORA_DO_ALCANCE);
        }
    }

    /// `SetPlantsuicide` → `plant_pet_manager::PlantSuicide` (`playerwrapper.cpp`,
    /// `petman.cpp:2345-2370`): da mais nova para a mais antiga, a primeira que tem habilidade de
    /// explosão e está a menos de `distancia` do dono lança-a no alvo e some 0,5 s depois; o dono
    /// recebe o `PLANT_PET_DISAPPEAR` com `PLANT_SUICIDE` na hora. Devolve a planta, a habilidade
    /// e o nível.
    pub fn explodir_planta(&mut self, dono: i64, distancia: f32, alvo: Option<i64>) -> Option<(i64, i32, i32)> {
        let pos_dono = self.players.get(&dono)?.position;
        let id = self.plantas_de(dono).into_iter().rev().find(|i| {
            self.mascotes.get(i).is_some_and(|m| {
                m.planta.as_ref().is_some_and(|p| p.suicida.is_some())
                    && m.corpo.position.distance(&pos_dono) <= distancia
            })
        })?;
        let m = self.mascotes.get_mut(&id)?;
        let p = m.planta.as_mut()?;
        let (skill, nivel) = p.suicida?;
        p.sumir_em_ms = Some(500);
        self.emitir(EventoDoMundo::PlantaSumiu {
            id,
            dono: dono as pw_core::RoleId,
            motivo: EXPLODIU,
        });
        let _ = alvo;
        Some((id, skill, nivel))
    }

    /// O batimento de 1 s das plantas, do lado do dono e da `gpet_plant_policy` — o que o tique
    /// comum do mascote não faz: a vida contada (`PLANT_LIFE_EXHAUST`), a explosão que some, o
    /// juntar alvo da automática, a cura da passiva e o aviso de vida/mana ao dono.
    pub(crate) fn batimento_das_plantas(&mut self, delta_ms: u32) {
        let ids: Vec<i64> = self.mascotes.iter().filter(|(_, m)| m.planta.is_some()).map(|(id, _)| *id).collect();
        let mut sumir = Vec::new();
        let mut avisos = Vec::new();
        for id in ids {
            let Some(m) = self.mascotes.get_mut(&id) else { continue };
            let Some(p) = m.planta.as_mut() else { continue };
            if let Some(ms) = p.sumir_em_ms {
                let ms = ms.saturating_sub(delta_ms);
                p.sumir_em_ms = Some(ms);
                if ms == 0 {
                    sumir.push((id, EXPLODIU));
                }
                continue;
            }
            p.batimento_ms += delta_ms;
            if p.batimento_ms < 1000 {
                continue;
            }
            p.batimento_ms -= 1000;
            if p.vida_s > 0 {
                p.vida_s -= 1;
                if p.vida_s == 0 {
                    sumir.push((id, VIDA_ACABOU));
                    continue;
                }
            }
            let agora = (m.corpo.hp, m.corpo.mp);
            if p.avisado != Some(agora) {
                p.avisado = Some(agora);
                let fh = if m.corpo.max_hp > 0 { m.corpo.hp as f32 / m.corpo.max_hp as f32 } else { 0.0 };
                let fm = if m.corpo.max_mp > 0 { m.corpo.mp as f32 / m.corpo.max_mp as f32 } else { 0.0 };
                avisos.push(EventoDoMundo::VidaDaPlanta {
                    id,
                    dono: m.dono as pw_core::RoleId,
                    fator_hp: fh,
                    hp: m.corpo.hp as i32,
                    fator_mp: fm,
                    mp: m.corpo.mp,
                });
            }
            self.juntar_alvo_da_planta(id);
        }
        for ev in avisos {
            self.emitir(ev);
        }
        for (id, motivo) in sumir {
            if motivo == EXPLODIU {
                // O dono já foi avisado na explosão.
                self.tirar_mascote(id);
                self.emitir(EventoDoMundo::MonstroSumiu { id });
            } else {
                self.remover_planta(id, motivo);
            }
        }
    }

    /// `gpet_plant_policy::OnHeartbeat` (`petnpc.cpp:2019-2056`): a automática fora de combate e
    /// sem tarefa junta ódio +1 em cada monstro vivo dentro da visão; a passiva, sem tarefa e com
    /// a automática pronta (recarga e mana), lança-a no aliado mais próximo dentro da visão — do
    /// grupo do dono, ou o próprio dono sem grupo.
    fn juntar_alvo_da_planta(&mut self, id: i64) {
        let Some(m) = self.mascotes.get(&id) else { return };
        if m.corpo.is_dead || m.ai.tarefa.is_some() || m.ai.conjurando() {
            return;
        }
        let pos = m.corpo.position;
        let visao = m.corpo.sight_range as f32;
        let dono = m.dono;
        if m.ai.agressividade == crate::mascote::AGRESSIVIDADE_AUTOMATICA {
            if m.ai.em_combate() {
                return;
            }
            let perto: Vec<i64> = self
                .monsters
                .iter()
                .filter(|(_, (x, _))| !x.is_dead && x.position.distance(&pos) < visao)
                .map(|(i, _)| *i)
                .collect();
            if let Some(m) = self.mascotes.get_mut(&id) {
                for i in perto {
                    *m.ai.odio.entry(i).or_insert(0) += 1;
                }
            }
            return;
        }
        let Some(h) = m.ai.automatica else { return };
        if !m.ai.recarga_pronta(h.recarga()) || m.corpo.mp < h.mana {
            return;
        }
        let grupo = self.membros_do_grupo(dono as pw_core::RoleId);
        let candidatos: Vec<i64> = if grupo.len() > 1 {
            grupo.iter().map(|r| *r as i64).collect()
        } else {
            vec![dono]
        };
        let alvo = candidatos
            .into_iter()
            .filter_map(|c| self.players.get(&c).filter(|q| q.hp > 0).map(|q| (c, q.position.distance(&pos))))
            .filter(|(_, d)| *d < visao)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(c, _)| c);
        if let (Some(alvo), Some(m)) = (alvo, self.mascotes.get_mut(&id)) {
            m.ai.tarefa = Some(crate::mascote::TarefaDeHabilidade::nova(h, Some(alvo)));
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn modelo() -> ModeloDeMascote {
        ModeloDeMascote {
            tid: 28914,
            classe: 4,
            hp: [8.0, 0.0, 5.0],
            hp_gen: [0.1, 0.0, 0.0],
            dano: [8.0, 0.0, 1.0, 5.0],
            velocidade: [0.0, 0.0],
            ataque: [10.0, 0.0, 5.0],
            esquiva: [5.0, 0.0, 5.0],
            defesa: [15.0, 1.0, 0.0, 5.0],
            resistencia: [15.0, 1.0, 0.0, 5.0],
            corpo: 0.6,
            alcance: 3.0,
            atraso_do_dano: 10,
            intervalo_do_golpe: 20,
            visao: 15.0,
            comida: 0,
            habitat: 6,
            imunidade: 0,
            nivel_maximo: 105,
            nivel_exigido: 1,
            mana: 2.0,
            regen_mana: 0.0,
            grau_de_ataque: 1.0,
            grau_de_defesa: 1.0,
            grupo_de_planta: 0,
            limite_do_grupo: 1,
        }
    }

    /// `GenerateBaseProp2` + `CalcPetEnhance`: no nível 10 com a habilidade no nível 4 (20 %), a
    /// planta 28914 do Místico soma 20 % das parcelas do dono.
    #[test]
    fn o_corpo_da_planta_soma_a_parte_do_dono() {
        let dono = BaseParaMascote {
            vida: 1000,
            mana: 500,
            dano_magico: 200,
            defesa: 100,
            resistencias: [50, 60, 70, 80, 90],
            grau_de_ataque: 10,
            grau_de_defesa: 5,
        };
        let m = corpo_da_planta(1, &modelo(), 10, 4, &dono, Vector3::new(0.0, 0.0, 0.0));
        // `CalcHP` = (int)(8 × (10 − 0 + 5)) = 120, + (int)(1000 × 0.01f × 20) = 199 no x87.
        assert_eq!((m.max_hp, m.hp), (319, 319));
        // `CalcMP` = (int)(2 × (10 + 5)) = 30, + 99 = 129; nasce com a metade.
        assert_eq!((m.max_mp, m.mp), (129, 64));
        // `CalcDamage` = (int)(8 × (0 + 10 + 5)) = 120, + 39 do dano mágico do dono.
        assert_eq!((m.attack_min, m.attack_max, m.magic_attack[0]), (159, 159, (159, 159)));
        // `CalcDefense` = (int)(15 × (1 × (10 − 0) + 5)) = 225 + 19; resistência 225 + a parte de cada.
        assert_eq!(m.def_phys, 244);
        assert_eq!(m.resistances, [234, 236, 238, 240, 242]);
        // `CalcAttackDegree` (int)(1 × 15) + 1; o de defesa + 0.
        assert_eq!((m.attack_degree, m.defend_degree), (16, 15));
        assert_eq!(m.attack_range, 3.6, "alcance + corpo");
        assert_eq!(m.move_speed, 0.0);
    }

    #[test]
    fn o_tipo_da_primeira_habilidade_decide() {
        assert_eq!(agressiva_pelo_tipo(1), Some(true));
        assert_eq!(agressiva_pelo_tipo(3), Some(true));
        assert_eq!(agressiva_pelo_tipo(2), Some(false));
        assert_eq!(agressiva_pelo_tipo(10), None);
    }
}
