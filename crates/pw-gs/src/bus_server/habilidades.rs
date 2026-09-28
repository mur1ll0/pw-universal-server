//! Habilidades pelos stubs do servidor: área, flechas, precisão e efeitos de estado (B53).
//!
//! `PlayerWrapper::SetPerform` (`cskill/skill/playerwrapper.cpp:170-420`) é o desenho:
//!
//! 1. `CanAttack(cost)` (`skill.cpp:213-231`): arma exigida e flechas (`UseArrow(arrowcost)`);
//!    faltando, a habilidade não sai.
//! 2. `dobless` → `BlessMe` em quem conjura.
//! 3. `TYPE_ATTACK` → um `attack_msg` com o dano calculado **uma vez**, a precisão vezes
//!    `GetHitrate`, e, se `doenchant`, a habilidade presa (`attached_skill`) para o alvo
//!    rodar o `StateAttack` ao ser atingido (`SkillWrapper::Attack`,
//!    `skillwrapper.cpp:449-484`). O alcance do golpe segue `range.type`: ponto
//!    (`Attack`), linha (`RegionAttack2`, cilindro), bola em si ou no alvo (`RegionAttack1`,
//!    esfera) e setor (`RegionAttack3`, cone) — `obj_interface.cpp:1066-1130`.
//! 4. Bênção/maldição (`DoEnchant`) → `enchant_msg`; cada alvo roda o `StateAttack` e manda
//!    `ENCHANT_RESULT` (`skillwrapper.cpp:516-552`).
//!
//! **Sem trava de PvP**: golpe em área e maldição só alcançam monstros; bênção em área, só
//! jogadores. O original decide pelo modo de PK, que não existe aqui.

use super::*;
use crate::combat::Resultado;
use crate::efeitos::{self, Aplicacao, Efeito, Filtro};
use crate::entity::{MonsterEntity, CORPO_DO_JOGADOR};
use std::collections::HashMap;

/// `TYPE_ATTACK`/`TYPE_BLESS`/`TYPE_CURSE` (`cskill/skill/skill.h:41-54`).
const TIPO_ATAQUE: i32 = 1;
const TIPO_BENCAO: i32 = 2;
const TIPO_MALDICAO: i32 = 3;
/// `TYPE_BLESSPET` (`cskill/skill/skill.h:53`).
const TIPO_BENCAO_DE_MASCOTE: i32 = 10;
/// `Range::Type` (`cskill/skill/range.h:18-25`).
const AREA_PONTO: i32 = 0;
const AREA_LINHA: i32 = 1;
const AREA_BOLA_EM_SI: i32 = 2;
const AREA_BOLA_NO_ALVO: i32 = 3;
const AREA_SETOR: i32 = 4;
const AREA_EM_SI: i32 = 5;

/// Um objeto que recebe efeito.
#[derive(Clone, Copy, PartialEq)]
enum Alvo {
    Jogador(i64),
    Monstro(i64),
    /// O corpo de um mascote de combate (a bênção dele em si, `TYPE_BLESSPET`).
    Mascote(i64),
}

impl Alvo {
    fn id(self) -> i64 {
        match self {
            Alvo::Jogador(i) | Alvo::Monstro(i) | Alvo::Mascote(i) => i,
        }
    }
}

/// Quem conjura, do ponto de vista do roteiro: o `PlayerWrapper` do original embrulha o
/// `object_interface` de qualquer um — jogador ou NPC (`SkillWrapper::NpcStart`,
/// `skillwrapper.cpp:974-1004`).
struct Conjurador {
    /// A origem dos efeitos (ódio e crédito do dano no tempo).
    id: i64,
    nivel: i32,
    /// `skill->GetPlayer()->Get*`.
    vars: HashMap<&'static str, f64>,
    /// Faixa do ataque mágico, para o `GetMagicdamage` do roteiro.
    magico: (i32, i32),
    /// `skill->GetPerformerid().IsPlayerClass()`: o dano no tempo de jogador em jogador é ¼
    /// (`playerwrapper.cpp:1279-1287`); o de monstro em jogador, não.
    e_jogador: bool,
}

impl Conjurador {
    fn do_jogador(p: &PlayerEntity) -> Self {
        Self {
            id: p.role_id as i64,
            nivel: p.level,
            vars: vars_do_jogador(p),
            magico: (p.magic_attack_min, p.magic_attack_max),
            e_jogador: true,
        }
    }

    /// O mascote: o corpo de NPC dele, com o ataque do `GenerateBaseProp` e sem ataque
    /// mágico.
    fn do_mascote(id: i64, corpo: &MonsterEntity) -> Self {
        let mut vars = vars_do_monstro(corpo);
        vars.insert("Attack", ((corpo.attack_min + corpo.attack_max) / 2) as f64);
        vars.insert("Magicattack", 0.0);
        vars.insert("Ap", 0.0);
        vars.insert("Form", 0.0);
        Self {
            id,
            nivel: corpo.level,
            vars,
            magico: (0, 0),
            e_jogador: false,
        }
    }

    /// O monstro: o ataque dele e o mágico do `MONSTER_ESSENCE`.
    fn do_monstro(id: i64, corpo: &MonsterEntity, magico: (i32, i32)) -> Self {
        let mut vars = vars_do_monstro(corpo);
        vars.insert("Attack", ((corpo.attack_min + corpo.attack_max) / 2) as f64);
        vars.insert("Magicattack", ((magico.0 + magico.1) / 2) as f64);
        vars.insert("Ap", 0.0);
        vars.insert("Form", 0.0);
        Self {
            id,
            nivel: corpo.level,
            vars,
            magico,
            e_jogador: false,
        }
    }
}

/// O que mudou num alvo depois de um roteiro.
#[derive(Default)]
struct Mudanca {
    efeitos: bool,
    atributos: bool,
    morreu: bool,
    /// O `immune` do `PlayerWrapper` que vai no `SendClientEnchantResult`
    /// (`skillwrapper.cpp:552-556`): `MOD_IMMUNE` 0x80, `MOD_ENCHANT_FAILED` 0x100,
    /// `MOD_SUCCESS` 0x200 (`EC_ManAttacks.h:36-38`) — o "Imune/Falhou/Sucesso" sobre o alvo.
    imune: i32,
}

/// `MOD_IMMUNE` (`EC_ManAttacks.h:36`).
const MOD_IMUNE: i32 = 0x80;
/// `MOD_ENCHANT_FAILED` (`EC_ManAttacks.h:37`).
const MOD_FALHOU: i32 = 0x100;
/// `MOD_SUCCESS` (`EC_ManAttacks.h:38`).
const MOD_SUCESSO: i32 = 0x200;

/// A chance da captura (`PlayerWrapper::SetEntrap`, `playerwrapper.cpp:2460-2479`), em
/// porcentagem: `((max − hp)/max)² × 100 × (1,35 − nível/100 + nível_da_habilidade × 0,05)`,
/// com `GetLevel() / 100` em **divisão inteira** (zero abaixo do nível 100).
pub(crate) fn chance_de_captura(hp: i64, max_hp: i64, nivel_do_monstro: i32, nivel_da_habilidade: i32) -> f32 {
    if max_hp <= 0 {
        return 0.0;
    }
    let falta = (max_hp - hp.clamp(0, max_hp)) as f32 / max_hp as f32;
    falta * falta * 100.0 * (1.35 - (nivel_do_monstro / 100) as f32 + nivel_da_habilidade as f32 * 0.05)
}

/// `SkillStub::GetEnmity` (`skill.cpp:648-653`): o ódio que a habilidade põe em cada vítima,
/// pelo nível dela e, em 18 habilidades, pelo nível de quem lança (`10 * P_Level * (3 + L)`).
/// Talentos (`S_T0`) valem 0, como no resto dos roteiros.
fn odio_da_habilidade(h: &pw_data_loader::habilidades::HabilidadeDoServidor, nivel: i32, nivel_de_quem_lanca: i32) -> i64 {
    if let Some(e) = &h.odio_expr {
        let vars = |v: &str| match v {
            "L" => Some(nivel as f64),
            "P_Level" => Some(nivel_de_quem_lanca as f64),
            "S_T0" | "S_T1" | "S_T2" => Some(0.0),
            _ => None,
        };
        return efeitos::expr::avaliar(e, &vars).unwrap_or(0.0) as i64;
    }
    h.odio
        .as_ref()
        .and_then(|v| v.get(usize::try_from(nivel - 1).ok()?).copied())
        .unwrap_or(0) as i64
}

fn vars_do_jogador(p: &PlayerEntity) -> HashMap<&'static str, f64> {
    HashMap::from([
        ("Maxhp", p.max_hp as f64),
        ("Maxmp", p.max_mp as f64),
        ("Hp", p.hp as f64),
        ("Mp", p.mp as f64),
        ("Level", p.level as f64),
        ("Cls", p.cls as i32 as f64),
        ("Attack", ((p.attack_min + p.attack_max) / 2) as f64),
        (
            "Magicattack",
            ((p.magic_attack_min + p.magic_attack_max) / 2) as f64,
        ),
        ("Defense", p.def_phys as f64),
        ("Goldresist", p.def_metal as f64),
        ("Woodresist", p.def_wood as f64),
        ("Waterresist", p.def_water as f64),
        ("Fireresist", p.def_fire as f64),
        ("Earthresist", p.def_earth as f64),
        ("Ap", 0.0),
        ("Form", 0.0),
    ])
}

fn vars_do_monstro(m: &MonsterEntity) -> HashMap<&'static str, f64> {
    HashMap::from([
        ("Maxhp", m.max_hp as f64),
        ("Hp", m.hp as f64),
        ("Maxmp", m.max_mp as f64),
        ("Mp", m.mp as f64),
        ("Level", m.level as f64),
        ("Cls", -1.0),
        ("Defense", m.def_phys as f64),
        ("Goldresist", m.resistances[0] as f64),
        ("Woodresist", m.resistances[1] as f64),
        ("Waterresist", m.resistances[2] as f64),
        ("Fireresist", m.resistances[3] as f64),
        ("Earthresist", m.resistances[4] as f64),
    ])
}

/// `skill->GetDamage()/GetMagicdamage()/Get<escola>damage()`: o dano do golpe já sorteado,
/// e o mágico de quem conjura (`SetMagicDamage(GetMagicattack())`, `playerwrapper.cpp:249`).
fn vars_da_habilidade(
    golpe: &crate::combat::Golpe,
    conjurador: &Conjurador,
) -> HashMap<&'static str, f64> {
    let magico =
        crate::combat::sortear_dano_fisico(conjurador.magico.0, conjurador.magico.1) as f64;
    HashMap::from([
        ("Damage", golpe.dano_fisico as f64),
        ("Attack", golpe.dano_fisico as f64),
        ("Magicdamage", magico),
        ("Magicattack", magico),
        ("Golddamage", golpe.dano_magico[0] as f64),
        ("Wooddamage", golpe.dano_magico[1] as f64),
        ("Waterdamage", golpe.dano_magico[2] as f64),
        ("Firedamage", golpe.dano_magico[3] as f64),
        ("Earthdamage", golpe.dano_magico[4] as f64),
        ("Section", 1.0),
    ])
}

/// Distância de um ponto a um segmento, e a projeção ao longo dele.
fn no_cilindro(inicio: Vector3, fim: Vector3, p: Vector3, raio: f32) -> bool {
    let (dx, dy, dz) = (fim.x - inicio.x, fim.y - inicio.y, fim.z - inicio.z);
    let len2 = dx * dx + dy * dy + dz * dz;
    if len2 < 1e-6 {
        return p.distance(&inicio) <= raio;
    }
    let t = ((p.x - inicio.x) * dx + (p.y - inicio.y) * dy + (p.z - inicio.z) * dz) / len2;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let q = Vector3::new(inicio.x + dx * t, inicio.y + dy * t, inicio.z + dz * t);
    p.distance(&q) <= raio
}

/// `BroadcastTaperMessage(pos, range, cos_half_angle)`: dentro do alcance e do cone.
fn no_setor(origem: Vector3, rumo: Vector3, p: Vector3, alcance: f32, cos_meio: f32) -> bool {
    let d = p.distance(&origem);
    if d > alcance {
        return false;
    }
    if d < 1e-3 {
        return true;
    }
    let (ax, ay, az) = (rumo.x - origem.x, rumo.y - origem.y, rumo.z - origem.z);
    let la = (ax * ax + ay * ay + az * az).sqrt();
    if la < 1e-3 {
        return true;
    }
    let cos = ((p.x - origem.x) * ax + (p.y - origem.y) * ay + (p.z - origem.z) * az) / (la * d);
    cos >= cos_meio
}

impl BusServer {
    /// A habilidade pelo stub. `false` quando o stub não tem nada a fazer (sem dano, sem
    /// roteiro) — aí o chamador segue pelo caminho antigo.
    pub(super) async fn aplicar_habilidade(
        &self,
        roleid: i32,
        skill_id: i32,
        alvo: i64,
        nivel: i32,
        carga: f32,
        envio: &EnvioAoCliente,
    ) -> bool {
        let (h, conjurador, dados) = {
            let mundo = self.world.read().await;
            let Some(h) = mundo
                .data_manager
                .habilidades
                .get(skill_id.max(0) as u32)
                .cloned()
            else {
                return false;
            };
            let Some(p) = mundo.players.get(&(roleid as i64)).cloned() else {
                return false;
            };
            (h, p, Arc::clone(&mundo.data_manager))
        };
        let tipo = h.tipo.unwrap_or(0);
        let area = h.tipo_de_area.unwrap_or(AREA_PONTO);
        let tem_dano = tipo == TIPO_ATAQUE && h.dano.is_some();
        let roteiro_no_alvo = h
            .no_alvo
            .clone()
            .filter(|_| h.doenchant || tipo == TIPO_BENCAO || tipo == TIPO_MALDICAO);
        let roteiro_em_si = h.em_si.clone().filter(|_| h.dobless);
        if !tem_dano && roteiro_no_alvo.is_none() && roteiro_em_si.is_none() {
            return false;
        }
        // Golpe de ponto em jogador segue o caminho antigo (dano entre jogadores sem trava).
        let alvo_e_monstro = self.world.read().await.monsters.contains_key(&alvo);
        if tipo == TIPO_ATAQUE && area == AREA_PONTO && !alvo_e_monstro && alvo != roleid as i64 {
            return false;
        }

        // `CanAttack(cost)`: flechas.
        let flechas = h.arrowcost.unwrap_or(0).max(0) as u32;
        if tipo == TIPO_ATAQUE && flechas > 0 {
            let repo = self.itens().await;
            let tem = repo
                .get_item_by_slot(roleid, ContainerType::Equipment, 11)
                .await
                .ok()
                .flatten()
                .map(|i| i.count)
                .unwrap_or(0);
            if tem < flechas
                || repo
                    .consume_item(roleid, ContainerType::Equipment, 11, flechas)
                    .await
                    .is_err()
            {
                debug!("mundo: {roleid} conjurou {skill_id} sem {flechas} flecha(s) — não sai (`UseArrow`)");
                return true;
            }
            // `FillAttackMsg(target, msg, arrowcost)` → `attack_once(arrowcost)`.
            self.responder(
                roleid,
                S2CGamedataSend::attack_once(flechas.min(255) as u8).data,
                envio,
            )
            .await;
        }

        // O golpe, calculado uma vez para todos os alvos.
        let mut golpe = match h
            .dano
            .as_ref()
            .and_then(|d| CombatEngine::golpe_de_habilidade(&conjurador, d, nivel, carga))
        {
            Some(g) => g,
            None => {
                let mut g = CombatEngine::golpe_de_jogador(&conjurador);
                g.de_habilidade = true;
                g
            }
        };
        golpe.taxa_de_ataque = (golpe.taxa_de_ataque as f32 * h.precisao(nivel)) as i32;

        let quem = Conjurador::do_jogador(&conjurador);
        let odio = odio_da_habilidade(&h, nivel, conjurador.level);
        let mut nao_portados: Vec<String> = Vec::new();
        let mut avisar: Vec<(Alvo, Mudanca)> = Vec::new();
        let mut mortos: Vec<i64> = Vec::new();

        // 2. `BlessMe`.
        if let Some(passos) = &roteiro_em_si {
            let m = self
                .rodar_roteiro(
                    Alvo::Jogador(roleid as i64),
                    passos,
                    nivel,
                    &quem,
                    &golpe,
                    &mut nao_portados,
                )
                .await;
            avisar.push((Alvo::Jogador(roleid as i64), m));
        }

        // 3/4. Os alvos.
        let alvos = self
            .alvos_da_habilidade(roleid, &conjurador, alvo, tipo, area, &h, nivel)
            .await;
        if tipo == TIPO_ATAQUE {
            let mut resultados = Vec::new();
            {
                let mut mundo = self.world.write().await;
                let progressao = std::sync::Arc::clone(&mundo.data_manager);
                let progressao = &progressao.progressao;
                let mut apanharam = Vec::new();
                for a in &alvos {
                    let Alvo::Monstro(id) = *a else { continue };
                    let Some((m, ai)) = mundo.monsters.get_mut(&id) else {
                        continue;
                    };
                    if m.is_dead {
                        continue;
                    }
                    let distancia = conjurador.position.distance(&m.position);
                    let r = combat::resolver(
                        &golpe,
                        &CombatEngine::defesa_do_monstro_contra(m, conjurador.level, &progressao),
                        distancia,
                        false,
                        combat::Rolagens::sortear(),
                    );
                    let acertou = matches!(r, Resultado::Acertou { .. });
                    let marca = r.marca();
                    let dano = efeitos::dano_recebido(&mut m.efeitos, r.dano()) as i64;
                    ai.add_threat(roleid as i64, dano.max(1));
                    let real = dano.min(m.hp);
                    m.hp = (m.hp - dano).max(0);
                    m.registrar_dano(roleid as i64, real);
                    apanharam.push((id, real as i32));
                    let morreu = m.hp == 0;
                    if morreu {
                        m.is_dead = true;
                        m.efeitos.ao_morrer();
                    }
                    resultados.push((
                        id,
                        dano,
                        acertou,
                        morreu,
                        m.hp,
                        m.max_hp,
                        m.target_id.unwrap_or(0),
                        marca,
                    ));
                    if morreu {
                        mundo.grid.remove_entity(id);
                    }
                }
                if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
                    p.combate_s = crate::progressao::COMBATE_AO_ATACAR_S;
                }
                // `gnpc_imp::OnDamage` → `_at_policy->OnDamage()` (`npc.cpp:1878`).
                for (id, dano) in apanharam {
                    mundo.politica_ao_apanhar(id, dano);
                }
            }
            for (id, dano, acertou, morreu, hp, max_hp, alvo_do_alvo, flag) in resultados {
                self.responder(
                    roleid,
                    self.sub
                        .self_skill_attack_result(
                            id as i32,
                            skill_id,
                            saturar(dano),
                            flag,
                            VELOCIDADE_PADRAO,
                            SECAO_UNICA,
                        )
                        .data,
                    envio,
                )
                .await;
                self.transmitir_a_outros(
                    roleid,
                    self.sub
                        .object_skill_attack_result(
                            roleid,
                            id as i32,
                            skill_id,
                            saturar(dano),
                            flag,
                            VELOCIDADE_PADRAO,
                            SECAO_UNICA,
                        )
                        .data,
                )
                .await;
                // A barra de vida vai no batimento de 1 s (`EventoDoMundo::VidaDoMonstro`, B56).
                let _ = alvo_do_alvo;
                debug!("mundo: habilidade {skill_id} de {roleid} em {id}: dano {dano}, vida {hp}/{max_hp}");
                if morreu {
                    mortos.push(id);
                    continue;
                }
                // `attached_skill` só vale se o golpe acertou (`HandleAttackMsg`).
                if acertou {
                    // `SkillWrapper::Attack` → `SetEnmity(GetEnmity())` (`skillwrapper.cpp:475-477`).
                    self.world.write().await.odio_de_habilidade(id, roleid as i64, odio);
                    if let Some(passos) = &roteiro_no_alvo {
                        let m = self
                            .rodar_roteiro(
                                Alvo::Monstro(id),
                                passos,
                                nivel,
                                &quem,
                                &golpe,
                                &mut nao_portados,
                            )
                            .await;
                        if m.morreu {
                            mortos.push(id);
                        }
                        avisar.push((Alvo::Monstro(id), m));
                    }
                }
            }
        } else if let Some(passos) = &roteiro_no_alvo {
            for a in &alvos {
                let m = self
                    .rodar_roteiro(*a, passos, nivel, &quem, &golpe, &mut nao_portados)
                    .await;
                // `SkillWrapper::Enchant` → `SetEnmity` na vítima monstro (maldição).
                if let Alvo::Monstro(id) = *a {
                    self.world.write().await.odio_de_habilidade(id, roleid as i64, odio);
                }
                // `SendClientEnchantResult` (`skillwrapper.cpp:546-551`).
                let pacote = self
                    .sub
                    .enchant_result(
                        roleid,
                        a.id() as i32,
                        skill_id,
                        nivel.clamp(0, 255) as u8,
                        false,
                        m.imune,
                        1,
                    )
                    .data;
                self.responder(roleid, pacote.clone(), envio).await;
                self.transmitir_a_outros(roleid, pacote).await;
                if m.morreu {
                    mortos.push(a.id());
                }
                avisar.push((*a, m));
            }
            if tipo == TIPO_MALDICAO {
                if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
                    p.combate_s = crate::progressao::COMBATE_AO_ATACAR_S;
                }
            }
        }

        if !nao_portados.is_empty() {
            nao_portados.sort();
            nao_portados.dedup();
            debug!(
                "mundo: habilidade {skill_id} — sem porte: {}",
                nao_portados.join(", ")
            );
        }
        debug!(
            "mundo: {roleid} usou {skill_id} (nível {nivel}, tipo {tipo}, área {area}) em {} alvo(s)",
            alvos.len()
        );
        let _ = dados;
        for (a, m) in avisar {
            if m.efeitos {
                self.avisar_efeitos(a.id(), m.atributos).await;
            }
        }
        for id in mortos {
            info!("mundo: {roleid} matou {id} com a habilidade {skill_id}");
            let morte = S2CGamedataSend::npc_died(id as i32, roleid).data;
            self.responder(roleid, morte.clone(), envio).await;
            self.transmitir_a_outros(roleid, morte).await;
            self.monstro_morreu(id).await;
        }
        true
    }

    /// O efeito de uma habilidade do mascote, ao fim do canto (`SkillWrapper::NpcEnd` →
    /// `NpcRun` → `PlayerWrapper::SetPerform`, `skillwrapper.cpp:1006-1024`,
    /// `playerwrapper.cpp:170-420`), com o mascote como conjurador:
    ///
    /// - `dobless` → o `BlessMe` no próprio mascote (a 761 acelera o golpe dele);
    /// - ataque → o golpe, com o dano do mascote, a precisão × `GetHitrate`, e o crédito do
    ///   **dono** (`gpet_imp::FillAttackMsg`) pelo mesmo caminho do golpe comum; quem vê
    ///   recebe o `OBJECT_SKILL_ATTACK_RESULT` com o mascote como atacante (o monstro
    ///   atingido por quem não é jogador manda a todos, `gnpc_dispatcher::be_damaged`,
    ///   `npc.cpp:219-223`); acertou, o alvo roda o `StateAttack` (o sangramento da 747);
    /// - maldição → o `StateAttack` no monstro; bênção de mascote (`TYPE_BLESSPET`, 10, área
    ///   5) → no próprio mascote; os dois com `ENCHANT_RESULT` a quem vê.
    pub(super) async fn aplicar_habilidade_do_mascote(
        &self,
        pet: i64,
        skill_id: i32,
        nivel: i32,
        alvo: Option<i64>,
    ) {
        let (h, corpo, bruto, lealdade) = {
            let mundo = self.world.read().await;
            let Some(h) = mundo
                .data_manager
                .habilidades
                .get(skill_id.max(0) as u32)
                .cloned()
            else {
                return;
            };
            // Recolhido ou morto entre o canto e o efeito: nada sai.
            let Some(m) = mundo.mascotes.get(&pet).filter(|m| !m.corpo.is_dead) else {
                return;
            };
            let Some(modelo) = mundo
                .data_manager
                .modelos_de_mascote
                .get(&(m.info.pet_tid as u32))
            else {
                return;
            };
            let bruto = modelo.atributos(m.corpo.level.max(1)).dano;
            let lealdade = crate::mascote::ajuste_de_dano(crate::mascote::nivel_de_lealdade(
                m.info.honor_point,
            ));
            (h, m.corpo.clone(), bruto, lealdade)
        };
        let tipo = h.tipo.unwrap_or(0);
        let quem = Conjurador::do_mascote(pet, &corpo);
        let odio = odio_da_habilidade(&h, nivel, corpo.level);
        let mut golpe = match h.dano.as_ref().and_then(|d| {
            CombatEngine::golpe_de_habilidade_de_mascote(&corpo, bruto, lealdade, d, nivel)
        }) {
            Some(g) => g,
            None => {
                let mut g = CombatEngine::golpe_de_monstro(&corpo);
                g.atacante_e_jogador_ou_pet = true;
                g.de_habilidade = true;
                g
            }
        };
        golpe.taxa_de_ataque = (golpe.taxa_de_ataque as f32 * h.precisao(nivel)) as i32;
        let mut nao_portados = Vec::new();
        let mut avisar: Vec<(Alvo, Mudanca)> = Vec::new();
        let mut mortos = Vec::new();

        if let Some(passos) = h.em_si.clone().filter(|_| h.dobless) {
            let m = self
                .rodar_roteiro(
                    Alvo::Mascote(pet),
                    &passos,
                    nivel,
                    &quem,
                    &golpe,
                    &mut nao_portados,
                )
                .await;
            avisar.push((Alvo::Mascote(pet), m));
        }
        let roteiro_no_alvo = h
            .no_alvo
            .clone()
            .filter(|_| h.doenchant || tipo != TIPO_ATAQUE);
        let alvo_monstro = alvo.filter(|a| !crate::mascote::e_mascote(*a));
        if tipo == TIPO_ATAQUE {
            let Some(alvo) = alvo_monstro else { return };
            let resolvido = {
                let mut mundo = self.world.write().await;
                let r = mundo
                    .monsters
                    .get(&alvo)
                    .filter(|(m, _)| !m.is_dead)
                    .map(|(m, _)| {
                        let d = corpo.position.distance(&m.position);
                        combat::resolver(
                            &golpe,
                            &CombatEngine::defesa_do_monstro(m),
                            d,
                            false,
                            combat::Rolagens::sortear(),
                        )
                    });
                if let Some(r) = &r {
                    // O golpe do mascote: crédito do dono, ódio no mascote e 1 no dono.
                    mundo.adiar_dano(alvo, pet, r.dano() as i64, 0, false);
                }
                r
            };
            let Some(r) = resolvido else { return };
            let acertou = matches!(r, Resultado::Acertou { .. });
            let pacote = self
                .sub
                .object_skill_attack_result(
                    pet as i32,
                    alvo as i32,
                    skill_id,
                    saturar(r.dano() as i64),
                    r.marca(),
                    VELOCIDADE_PADRAO,
                    SECAO_UNICA,
                )
                .data;
            self.transmitir_a_quem_ve(pet, pacote).await;
            debug!(
                "mundo: o mascote {pet} usou {skill_id} (nível {nivel}) em {alvo}: dano {}",
                r.dano()
            );
            if acertou {
                // O ódio da habilidade vai a quem lança: o mascote (`GetPerformerid`).
                self.world.write().await.odio_de_habilidade(alvo, pet, odio);
                if let Some(passos) = &roteiro_no_alvo {
                    let m = self
                        .rodar_roteiro(
                            Alvo::Monstro(alvo),
                            passos,
                            nivel,
                            &quem,
                            &golpe,
                            &mut nao_portados,
                        )
                        .await;
                    if m.morreu {
                        mortos.push(alvo);
                    }
                    avisar.push((Alvo::Monstro(alvo), m));
                }
            }
        } else if let Some(passos) = &roteiro_no_alvo {
            let a = if crate::mascote::area_sem_alvo(h.tipo_de_area.unwrap_or(0)) {
                Alvo::Mascote(pet)
            } else {
                match alvo_monstro {
                    Some(id) => Alvo::Monstro(id),
                    None => return,
                }
            };
            let m = self
                .rodar_roteiro(a, passos, nivel, &quem, &golpe, &mut nao_portados)
                .await;
            if let Alvo::Monstro(id) = a {
                self.world.write().await.odio_de_habilidade(id, pet, odio);
            }
            let pacote = self
                .sub
                .enchant_result(
                    pet as i32,
                    a.id() as i32,
                    skill_id,
                    nivel.clamp(0, 255) as u8,
                    false,
                    0,
                    1,
                )
                .data;
            self.transmitir_a_quem_ve(pet, pacote).await;
            debug!(
                "mundo: o mascote {pet} usou {skill_id} (nível {nivel}, tipo {tipo}) em {}",
                a.id()
            );
            if m.morreu {
                mortos.push(a.id());
            }
            avisar.push((a, m));
        }
        if !nao_portados.is_empty() {
            nao_portados.sort();
            nao_portados.dedup();
            debug!(
                "mundo: habilidade {skill_id} do mascote — sem porte: {}",
                nao_portados.join(", ")
            );
        }
        for (a, m) in avisar {
            if m.efeitos {
                self.avisar_efeitos(a.id(), m.atributos).await;
            }
        }
        // Morte pelo roteiro (dano direto do `StateAttack`): o crédito é do dono.
        let dono = self
            .world
            .read()
            .await
            .mascotes
            .get(&pet)
            .map(|m| m.dono as i32);
        for id in mortos {
            if let Some(dono) = dono {
                self.transmitir_a_outros(0, S2CGamedataSend::npc_died(id as i32, dono).data)
                    .await;
            }
            self.monstro_morreu(id).await;
        }
    }

    /// O efeito de uma habilidade de monstro, ao fim do canto (`SkillWrapper::NpcEnd` →
    /// `NpcRun` → `PlayerWrapper::SetPerform`, `skillwrapper.cpp:1006-1024`), com o monstro como
    /// conjurador:
    ///
    /// - `dobless` → o `BlessMe` no próprio monstro;
    /// - ataque → em cada alvo da área, o golpe com o dano do monstro
    ///   ([`CombatEngine::golpe_de_habilidade_de_monstro`]) e a precisão × `GetHitrate`; o
    ///   jogador atingido recebe o `HOST_SKILL_ATTACKED` (144) e quem o vê o
    ///   `OBJECT_SKILL_ATTACK_RESULT` (143) (`gplayer_dispatcher::be_damaged`,
    ///   `player.cpp:3345-3354`); acertou, roda o `StateAttack` nele (sangramento, lentidão…);
    /// - maldição → o `StateAttack` em cada alvo; bênção → no próprio monstro (os amigos do
    ///   monstro são ele mesmo); as duas com `ENCHANT_RESULT` a quem vê.
    pub(super) async fn aplicar_habilidade_do_monstro(
        &self,
        id: i64,
        skill_id: i32,
        nivel: i32,
        alvo: i64,
    ) {
        let (h, corpo, magico) = {
            let mundo = self.world.read().await;
            let Some(h) = mundo
                .data_manager
                .habilidades
                .get(skill_id.max(0) as u32)
                .cloned()
            else {
                return;
            };
            // Morreu entre o canto e o efeito: nada sai.
            let Some((m, _)) = mundo.monsters.get(&id).filter(|(m, _)| !m.is_dead) else {
                return;
            };
            let magico = mundo
                .data_manager
                .monstros
                .get(m.template_id)
                .map(|t| (t.dano_magico.minimo, t.dano_magico.maximo))
                .unwrap_or((0, 0));
            (h, m.clone(), magico)
        };
        let tipo = h.tipo.unwrap_or(0);
        let area = h.tipo_de_area.unwrap_or(0);
        let na_tabela = nivel.clamp(1, h.max_level.max(1));
        let quem = Conjurador::do_monstro(id, &corpo, magico);
        let mut golpe = match h.dano.as_ref().and_then(|d| {
            CombatEngine::golpe_de_habilidade_de_monstro(&corpo, magico, d, na_tabela)
        }) {
            Some(g) => g,
            None => {
                let mut g = CombatEngine::golpe_de_monstro(&corpo);
                g.de_habilidade = true;
                g
            }
        };
        golpe.taxa_de_ataque = (golpe.taxa_de_ataque as f32 * h.precisao(na_tabela)) as i32;
        let mut nao_portados = Vec::new();
        let mut avisar: Vec<(Alvo, Mudanca)> = Vec::new();

        if let Some(passos) = h.em_si.clone().filter(|_| h.dobless) {
            let m = self
                .rodar_roteiro(
                    Alvo::Monstro(id),
                    &passos,
                    nivel,
                    &quem,
                    &golpe,
                    &mut nao_portados,
                )
                .await;
            avisar.push((Alvo::Monstro(id), m));
        }
        let roteiro_no_alvo = h
            .no_alvo
            .clone()
            .filter(|_| h.doenchant || tipo != TIPO_ATAQUE);
        let alvos = if matches!(tipo, TIPO_BENCAO | TIPO_BENCAO_DE_MASCOTE) {
            vec![Alvo::Monstro(id)]
        } else {
            self.alvos_do_monstro(&corpo, alvo, area, &h, na_tabela)
                .await
        };

        for a in alvos {
            if tipo == TIPO_ATAQUE {
                let resolvido = {
                    let mut mundo = self.world.write().await;
                    let defesa = match a {
                        Alvo::Jogador(j) => mundo
                            .players
                            .get(&j)
                            .filter(|p| p.hp > 0)
                            .map(|p| (CombatEngine::defesa_do_jogador(p), p.position)),
                        Alvo::Mascote(m) => mundo
                            .mascotes
                            .get(&m)
                            .filter(|m| !m.corpo.is_dead)
                            .map(|m| (CombatEngine::defesa_do_monstro(&m.corpo), m.corpo.position)),
                        Alvo::Monstro(_) => None,
                    };
                    defesa.map(|(d, pos)| {
                        let r = combat::resolver(
                            &golpe,
                            &d,
                            corpo.position.distance(&pos),
                            false,
                            combat::Rolagens::sortear(),
                        );
                        match a {
                            Alvo::Jogador(j) => {
                                // `msg.short_range` é 0 no golpe de habilidade sem
                                // `RangeAdjust` (`playerwrapper.cpp:269-278`): os espinhos
                                // valem se a habilidade é física e acertou.
                                let fisico = if golpe.e_fisico && matches!(r, Resultado::Acertou { .. }) {
                                    golpe.dano_fisico
                                } else {
                                    0
                                };
                                mundo.habilidade_de_monstro_no_jogador(j, id, r.dano() as i64, fisico)
                            }
                            _ => mundo.adiar_dano(a.id(), id, r.dano() as i64, 0, false),
                        }
                        r
                    })
                };
                let Some(r) = resolvido else { continue };
                let dano = saturar(r.dano() as i64);
                if let Alvo::Jogador(j) = a {
                    let pacote = self
                        .sub
                        .host_skill_attacked(
                            id as i32,
                            skill_id,
                            dano,
                            r.marca(),
                            VELOCIDADE_PADRAO,
                            SECAO_UNICA,
                        )
                        .data;
                    self.enviar_ao_jogador(j as i32, pacote).await;
                }
                let pacote = self
                    .sub
                    .object_skill_attack_result(
                        id as i32,
                        a.id() as i32,
                        skill_id,
                        dano,
                        r.marca(),
                        VELOCIDADE_PADRAO,
                        SECAO_UNICA,
                    )
                    .data;
                self.transmitir_a_quem_ve(a.id(), pacote).await;
                debug!(
                    "mundo: o monstro {id} usou {skill_id} (nível {nivel}) em {}: dano {dano}",
                    a.id()
                );
                if matches!(r, Resultado::Acertou { .. }) {
                    if let Some(passos) = &roteiro_no_alvo {
                        let m = self
                            .rodar_roteiro(a, passos, nivel, &quem, &golpe, &mut nao_portados)
                            .await;
                        avisar.push((a, m));
                    }
                }
            } else if let Some(passos) = &roteiro_no_alvo {
                // Maldição em jogador sentado levanta (`GM_MSG_ENCHANT` não amigável,
                // `gs/player.cpp:776-781`).
                if let Alvo::Jogador(j) = a {
                    if !matches!(tipo, TIPO_BENCAO | TIPO_BENCAO_DE_MASCOTE) {
                        self.world.write().await.maldicao_levanta(j);
                    }
                }
                let m = self
                    .rodar_roteiro(a, passos, nivel, &quem, &golpe, &mut nao_portados)
                    .await;
                let pacote = self
                    .sub
                    .enchant_result(
                        id as i32,
                        a.id() as i32,
                        skill_id,
                        nivel.clamp(0, 255) as u8,
                        false,
                        0,
                        1,
                    )
                    .data;
                if let Alvo::Jogador(j) = a {
                    self.enviar_ao_jogador(j as i32, pacote.clone()).await;
                }
                self.transmitir_a_quem_ve(id, pacote).await;
                debug!(
                    "mundo: o monstro {id} usou {skill_id} (nível {nivel}, tipo {tipo}) em {}",
                    a.id()
                );
                avisar.push((a, m));
            }
        }
        if !nao_portados.is_empty() {
            nao_portados.sort();
            nao_portados.dedup();
            debug!(
                "mundo: habilidade {skill_id} do monstro — sem porte: {}",
                nao_portados.join(", ")
            );
        }
        for (a, m) in avisar {
            if m.efeitos {
                self.avisar_efeitos(a.id(), m.atributos).await;
            }
        }
    }

    /// Os alvos de uma habilidade de monstro pela área (`range.type`): os inimigos do monstro
    /// são os jogadores e os mascotes vivos. Ponto (0): o alvo, se estiver a `corpo + distância
    /// de efeito + corpo do alvo + 1`; bola em si (2) e no alvo (3) pelo `raio`; linha (1) e
    /// setor (4) como os do jogador; em si (5): ninguém além do próprio.
    async fn alvos_do_monstro(
        &self,
        corpo: &MonsterEntity,
        alvo: i64,
        area: i32,
        h: &pw_data_loader::habilidades::HabilidadeDoServidor,
        nivel: i32,
    ) -> Vec<Alvo> {
        let mundo = self.world.read().await;
        let corpo_do_monstro = mundo
            .data_manager
            .monstros
            .get(corpo.template_id)
            .map(|t| t.tamanho)
            .unwrap_or(0.0);
        let inimigos: Vec<(Alvo, Vector3)> = mundo
            .players
            .iter()
            .filter(|(_, p)| p.hp > 0)
            .map(|(id, p)| (Alvo::Jogador(*id), p.position))
            .chain(
                mundo
                    .mascotes
                    .iter()
                    .filter(|(_, m)| !m.corpo.is_dead)
                    .map(|(id, m)| (Alvo::Mascote(*id), m.corpo.position)),
            )
            .collect();
        let pos_do_alvo = inimigos
            .iter()
            .find(|(a, _)| a.id() == alvo)
            .map(|(_, p)| *p);
        let o = corpo.position;
        let escolher = |dentro: &dyn Fn(Vector3) -> bool| -> Vec<Alvo> {
            inimigos
                .iter()
                .filter(|(_, p)| dentro(*p))
                .map(|(a, _)| *a)
                .collect()
        };
        match area {
            AREA_PONTO => {
                let Some(pos) = pos_do_alvo else {
                    return Vec::new();
                };
                let efeito = h
                    .distancia_de_efeito(nivel, corpo.attack_range)
                    .filter(|d| *d > 0.0)
                    .unwrap_or(corpo.attack_range);
                if o.distance(&pos) > corpo_do_monstro + efeito + CORPO_DO_JOGADOR + 1.0 {
                    return Vec::new();
                }
                inimigos
                    .iter()
                    .filter(|(a, _)| a.id() == alvo)
                    .map(|(a, _)| *a)
                    .collect()
            }
            AREA_BOLA_EM_SI => {
                let r = Some(h.raio(nivel))
                    .filter(|r| *r > 0.0)
                    .unwrap_or(corpo.attack_range);
                escolher(&|p| p.distance(&o) <= r)
            }
            AREA_BOLA_NO_ALVO => {
                let Some(c) = pos_do_alvo else {
                    return Vec::new();
                };
                let r = h.raio(nivel);
                escolher(&|p| p.distance(&c) <= r)
            }
            AREA_LINHA => {
                let Some(t) = pos_do_alvo else {
                    return Vec::new();
                };
                let comprimento = Some(h.distancia_de_ataque(nivel))
                    .filter(|d| *d > 0.0)
                    .unwrap_or(corpo.attack_range)
                    + corpo_do_monstro
                    + CORPO_DO_JOGADOR;
                let d = t.distance(&o).max(1e-3);
                let fim = Vector3::new(
                    o.x + (t.x - o.x) / d * comprimento,
                    o.y + (t.y - o.y) / d * comprimento,
                    o.z + (t.z - o.z) / d * comprimento,
                );
                let raio = h.raio(nivel);
                escolher(&|p| no_cilindro(o, fim, p, raio))
            }
            AREA_SETOR => {
                let Some(t) = pos_do_alvo else {
                    return Vec::new();
                };
                let alcance = Some(h.raio(nivel))
                    .filter(|r| *r > 0.0)
                    .unwrap_or(corpo.attack_range)
                    + corpo_do_monstro
                    + CORPO_DO_JOGADOR;
                let cos = h.angulo(nivel);
                escolher(&|p| no_setor(o, t, p, alcance, cos))
            }
            _ => Vec::new(),
        }
    }

    /// Quem a habilidade alcança, pela área.
    #[allow(clippy::too_many_arguments)]
    async fn alvos_da_habilidade(
        &self,
        roleid: i32,
        conjurador: &PlayerEntity,
        alvo: i64,
        tipo: i32,
        area: i32,
        h: &pw_data_loader::habilidades::HabilidadeDoServidor,
        nivel: i32,
    ) -> Vec<Alvo> {
        let mundo = self.world.read().await;
        let eu = roleid as i64;
        let amigavel = tipo == TIPO_BENCAO;
        let pos_do_alvo = mundo
            .monsters
            .get(&alvo)
            .map(|(m, _)| {
                (
                    m.position,
                    mundo
                        .data_manager
                        .monstros
                        .get(m.template_id)
                        .map(|t| t.tamanho)
                        .unwrap_or(0.0),
                )
            })
            .or_else(|| {
                mundo
                    .players
                    .get(&alvo)
                    .map(|p| (p.position, CORPO_DO_JOGADOR))
            })
            .or_else(|| {
                mundo
                    .mascotes
                    .get(&alvo)
                    .filter(|m| !m.corpo.is_dead)
                    .map(|m| (m.corpo.position, m.ai.raio_do_corpo))
            });
        let monstros_vivos = || {
            mundo
                .monsters
                .iter()
                .filter(|(_, (m, _))| !m.is_dead)
                .map(|(id, (m, _))| (*id, m.position))
        };
        let jogadores_vivos = || {
            mundo
                .players
                .iter()
                .filter(|(_, p)| p.hp > 0)
                .map(|(id, p)| (*id, p.position))
        };
        let alcance_de_ataque = conjurador.attack_range;

        let escolher = |dentro: &dyn Fn(Vector3) -> bool| -> Vec<Alvo> {
            if amigavel {
                jogadores_vivos()
                    .filter(|(_, p)| dentro(*p))
                    .map(|(id, _)| Alvo::Jogador(id))
                    .collect()
            } else {
                monstros_vivos()
                    .filter(|(_, p)| dentro(*p))
                    .map(|(id, _)| Alvo::Monstro(id))
                    .collect()
            }
        };

        match area {
            AREA_EM_SI => vec![Alvo::Jogador(eu)],
            AREA_PONTO => {
                if alvo == eu {
                    return vec![Alvo::Jogador(eu)];
                }
                let Some((pos, corpo)) = pos_do_alvo else {
                    return Vec::new();
                };
                // `GetInrange(GetEffectdistance)`: corpo + distância de efeito + corpo do alvo.
                let efeito = h
                    .distancia_de_efeito(nivel, alcance_de_ataque)
                    .filter(|d| *d > 0.0)
                    .unwrap_or(alcance_de_ataque);
                if conjurador.position.distance(&pos) > CORPO_DO_JOGADOR + efeito + corpo + 1.0 {
                    debug!(
                        "mundo: {roleid} — alvo {alvo} saiu da distância de efeito ({efeito:.1})"
                    );
                    return Vec::new();
                }
                if mundo.monsters.contains_key(&alvo) {
                    if amigavel {
                        Vec::new()
                    } else {
                        vec![Alvo::Monstro(alvo)]
                    }
                } else if mundo.mascotes.contains_key(&alvo) {
                    // Bênção num mascote — a Curar Mascote (330) é `TYPE_BLESSPET` (10) de ponto
                    // (`playerwrapper.cpp:398`: `TYPE_BLESS || TYPE_BLESSPET` → `enchant`). Antes
                    // o mascote não era alvo possível e a conjuração acabava sem efeito (B114).
                    if matches!(tipo, TIPO_BENCAO | TIPO_BENCAO_DE_MASCOTE | 11 | 12) {
                        vec![Alvo::Mascote(alvo)]
                    } else {
                        Vec::new()
                    }
                } else {
                    vec![Alvo::Jogador(alvo)]
                }
            }
            AREA_BOLA_EM_SI => {
                let r = Some(h.raio(nivel))
                    .filter(|r| *r > 0.0)
                    .unwrap_or(alcance_de_ataque);
                let c = conjurador.position;
                escolher(&|p| p.distance(&c) <= r)
            }
            AREA_BOLA_NO_ALVO => {
                let Some((c, _)) = pos_do_alvo else {
                    return Vec::new();
                };
                let r = h.raio(nivel);
                escolher(&|p| p.distance(&c) <= r)
            }
            AREA_LINHA => {
                let Some((t, corpo)) = pos_do_alvo else {
                    return Vec::new();
                };
                let comprimento = Some(h.distancia_de_ataque(nivel))
                    .filter(|d| *d > 0.0)
                    .unwrap_or(alcance_de_ataque)
                    + corpo
                    + CORPO_DO_JOGADOR;
                let o = conjurador.position;
                let d = t.distance(&o).max(1e-3);
                let fim = Vector3::new(
                    o.x + (t.x - o.x) / d * comprimento,
                    o.y + (t.y - o.y) / d * comprimento,
                    o.z + (t.z - o.z) / d * comprimento,
                );
                let raio = h.raio(nivel);
                escolher(&|p| no_cilindro(o, fim, p, raio))
            }
            AREA_SETOR => {
                let Some((t, corpo)) = pos_do_alvo else {
                    return Vec::new();
                };
                let alcance = Some(h.raio(nivel))
                    .filter(|r| *r > 0.0)
                    .unwrap_or(alcance_de_ataque)
                    + corpo
                    + CORPO_DO_JOGADOR;
                let o = conjurador.position;
                let cos = h.angulo(nivel);
                escolher(&|p| no_setor(o, t, p, alcance, cos))
            }
            _ => Vec::new(),
        }
    }

    /// Roda um `StateAttack`/`BlessMe` num alvo e aplica o que passou no dado.
    async fn rodar_roteiro(
        &self,
        alvo: Alvo,
        passos: &[(String, String, String)],
        nivel: i32,
        conjurador: &Conjurador,
        golpe: &crate::combat::Golpe,
        nao_portados: &mut Vec<String>,
    ) -> Mudanca {
        let mut mundo = self.world.write().await;
        let dados = Arc::clone(&mundo.data_manager);
        let vars_p = conjurador.vars.clone();
        let vars_s = vars_da_habilidade(golpe, conjurador);
        let vars_v = match alvo {
            Alvo::Jogador(id) => match mundo.players.get(&id) {
                Some(p) if p.hp > 0 => vars_do_jogador(p),
                _ => return Mudanca::default(),
            },
            Alvo::Monstro(id) => match mundo.monsters.get(&id) {
                Some((m, _)) if !m.is_dead => vars_do_monstro(m),
                _ => return Mudanca::default(),
            },
            Alvo::Mascote(id) => match mundo.mascotes.get(&id) {
                Some(m) if !m.corpo.is_dead => vars_do_monstro(&m.corpo),
                _ => return Mudanca::default(),
            },
        };
        let vars = efeitos::variaveis(nivel, &vars_p, &vars_v, &vars_s);
        let (aplicacoes, nao_lidos) =
            efeitos::executar_roteiro(passos, &vars, &mut || (rand::random::<u32>() % 100) as i32);
        nao_portados.extend(nao_lidos.into_iter().map(|n| format!("expressão {n}")));

        let mut mud = Mudanca::default();
        let origem = conjurador.id;
        for ap in aplicacoes {
            self.aplicar_um(
                &mut mundo,
                &dados,
                alvo,
                nivel,
                &ap,
                conjurador,
                origem,
                &mut mud,
                nao_portados,
            );
        }
        if mud.atributos {
            if let Alvo::Jogador(id) = alvo {
                mundo.refazer_atributos(id);
            }
        }
        mud
    }

    /// Um `PlayerWrapper::SetX` que passou no dado.
    #[allow(clippy::too_many_arguments)]
    fn aplicar_um(
        &self,
        mundo: &mut crate::world::WorldInstance,
        dados: &pw_data_loader::GameDataManager,
        alvo: Alvo,
        nivel_da_habilidade: i32,
        ap: &Aplicacao,
        conjurador: &Conjurador,
        origem: i64,
        mud: &mut Mudanca,
        nao_portados: &mut Vec<String>,
    ) {
        // `SetEntrap`/`SetEntrap2` (`playerwrapper.cpp:2460-2479`, `3744-3761`): sem ovo
        // (`GetPetEggID`, só o NPC tem) ou com o conjurador de nível **abaixo** do alvo, imune;
        // senão rola a chance — acertou, o ovo vai ao conjurador e o monstro some.
        if ap.nome == "Entrap" || ap.nome == "Entrap2" {
            let alvo_monstro = match alvo {
                Alvo::Monstro(id) => mundo
                    .monsters
                    .get(&id)
                    .filter(|(m, _)| !m.is_dead)
                    .map(|(m, _)| (id, m.template_id, m.level, m.hp, m.max_hp)),
                _ => None,
            };
            let ovo = alvo_monstro
                .and_then(|(_, t, ..)| dados.monstros.get(t))
                .map(|t| t.ovo_de_captura)
                .unwrap_or(0);
            let Some((id, _, nivel_alvo, hp, max_hp)) = alvo_monstro.filter(|_| ovo > 0) else {
                mud.imune |= MOD_IMUNE;
                return;
            };
            if conjurador.nivel < nivel_alvo {
                mud.imune |= MOD_IMUNE;
                return;
            }
            let chance = if ap.nome == "Entrap" {
                chance_de_captura(hp, max_hp, nivel_alvo, nivel_da_habilidade)
            } else {
                ap.probabilidade
            };
            if ((rand::random::<u32>() % 100) as f32) < chance && conjurador.e_jogador {
                mundo.capturar_monstro(id, conjurador.id as i32, ovo);
                mud.imune |= MOD_SUCESSO;
                mud.morreu = false;
                debug!("mundo: {} capturou {id} (chance {chance:.1} %), ovo {ovo}", conjurador.id);
            } else {
                mud.imune |= MOD_FALHOU;
                debug!("mundo: captura de {id} por {} falhou (chance {chance:.1} %)", conjurador.id);
            }
            return;
        }
        // `SetSummon` (`playerwrapper.cpp:2478-2482`) → `OI_ResurrectPet`: revive o mascote
        // morto de quem recebeu o efeito — a habilidade 329, de área 5, no próprio conjurador.
        if ap.nome == "Summon" {
            if let Alvo::Jogador(id) = alvo {
                mundo.pedir_reviver_mascote(id as i32);
            }
            return;
        }
        // `SetAp` (`playerwrapper.cpp:2351-2357`): passou no dado → `ModifyAP((int)value)`. A
        // barra nova vai com o estado próprio do `avisar_efeitos` (o `SetRefreshState`). Os
        // `BlessMe` de chi do 1.2.6 (404, 406, 420…) — B122.
        if ap.nome == "Ap" {
            if let Alvo::Jogador(id) = alvo {
                if mundo
                    .players
                    .get_mut(&id)
                    .is_some_and(|p| p.mexer_no_chi(ap.valor as i32))
                {
                    mud.efeitos = true;
                }
            }
            return;
        }
        // `SetReturntown` (`playerwrapper.cpp:1916-1939`): `CanReturnToTown` é sempre verdade
        // (`obj_interface.cpp:2380-2383`); o salto fica com o barramento.
        if ap.nome == "Returntown" {
            if let Alvo::Jogador(id) = alvo {
                mundo.pedir_volta_a_cidade(id as i32);
            }
            return;
        }
        // O objeto: efeitos, vida, máximos, defesa e resistências (para o dano no tempo).
        // `GetCls()` da vítima (o NPC não tem classe: -1).
        let cls_do_alvo = match alvo {
            Alvo::Jogador(id) => mundo.players.get(&id).map(|p| p.cls as i32).unwrap_or(-1),
            _ => -1,
        };
        let (efs, hp, max_hp, mp_max, defesa, resist, nivel_alvo, e_jogador): (
            &mut efeitos::Efeitos,
            i64,
            i64,
            i32,
            i32,
            [i32; 5],
            i32,
            bool,
        ) = match alvo {
            Alvo::Jogador(id) => {
                let Some(p) = mundo.players.get_mut(&id) else {
                    return;
                };
                let res = [
                    p.def_metal,
                    p.def_wood,
                    p.def_water,
                    p.def_fire,
                    p.def_earth,
                ];
                (
                    &mut p.efeitos,
                    p.hp as i64,
                    p.max_hp as i64,
                    p.max_mp,
                    p.def_phys,
                    res,
                    p.level,
                    true,
                )
            }
            Alvo::Monstro(id) => {
                let Some((m, _)) = mundo.monsters.get_mut(&id) else {
                    return;
                };
                let r = m.efeitos.realce();
                let res = m
                    .resistances
                    .map(|x| crate::entity::com_realce(x, r.resistencia));
                let def = crate::entity::com_realce(m.def_phys, r.defesa);
                (
                    &mut m.efeitos,
                    m.hp,
                    m.max_hp,
                    m.max_mp,
                    def,
                    res,
                    m.level,
                    false,
                )
            }
            Alvo::Mascote(id) => {
                let Some(m) = mundo.mascotes.get_mut(&id) else {
                    return;
                };
                let c = &mut m.corpo;
                let r = c.efeitos.realce();
                let res = c
                    .resistances
                    .map(|x| crate::entity::com_realce(x, r.resistencia));
                let def = crate::entity::com_realce(c.def_phys, r.defesa);
                (
                    &mut c.efeitos,
                    c.hp,
                    c.max_hp,
                    c.max_mp,
                    def,
                    res,
                    c.level,
                    false,
                )
            }
        };
        let _ = mp_max;
        let razao = (ap.razao * 100.0) as i32;
        let novo = |efeito: Efeito| Filtro {
            efeito,
            restante_s: ap.tempo_s,
            razao,
            fator: ap.razao,
            por_segundo: 0,
            contador: 0,
            origem,
            icone: true,
            absorve: 0.0,
            escala_defesa: 0,
            fator_de_habilidade: ap.valor,
        };

        // Instantâneos: mexem na vida/mana, não criam filtro.
        let mut delta_hp: i64 = 0;
        let mut delta_mp: i32 = 0;
        match ap.nome.as_str() {
            // `HealBySkill(value)`.
            "Heal" => delta_hp = ap.valor as i64,
            // `Heal(GetMaxhp() * ratio)`.
            "Scaleinchp" => delta_hp = (max_hp as f32 * ap.razao) as i64,
            "Scaleincmp" => delta_mp = (mp_max as f32 * ap.razao) as i32,
            // `BeHurt(value)`.
            "Directhurt" => delta_hp = -(efeitos::dano_recebido(efs, ap.valor as i32) as i64),
            "Clearbuff" => mud.efeitos |= efs.limpar(true),
            "Cleardebuff" => mud.efeitos |= efs.limpar(false),
            // `PlayerWrapper::SetFoxform` (`cskill/skill/playerwrapper.cpp:2539-2551`): já na
            // forma de classe, a 312 **desfaz** a raposa; senão monta o `filter_Foxform(object,
            // (int)(100 × ratio), (int)(100 × amount), (int)(100 × probability),
            // GetValueInt())` — sem tempo, até ser desfeito.
            "Foxform" => {
                if efs.desfazer_raposa() {
                    mud.efeitos = true;
                    mud.atributos |= e_jogador;
                } else if e_jogador && efs.forma_atual() == 0 {
                    let mut f = novo(Efeito::Foxform);
                    f.restante_s = i32::MAX;
                    f.razao = (100.0 * ap.razao) as i32;
                    f.escala_defesa = (100.0 * ap.quantia) as i32;
                    f.por_segundo = (100.0 * ap.probabilidade) as i32;
                    // O `_shape`: o `value` do roteiro no 1.5.5 (`GetValueInt()`, 1 na 312); o
                    // `filter_Foxform::OnAttach` do 1.2.6 faz `ChangeShape(1)` fixo (VA
                    // 0x830b1a0) e o `StateAttack` dele não tem `SetValue` (B122).
                    f.contador = if ap.valor != 0.0 { ap.valor as i32 } else { 1 };
                    if efs.adicionar(f) {
                        mud.efeitos = true;
                        mud.atributos = true;
                    }
                }
            }
            _ => {
                let Some(efeito) = ap.efeito else {
                    nao_portados.push(ap.nome.clone());
                    return;
                };
                let mut f = novo(efeito);
                if efeito == Efeito::Inchpsteal {
                    // `PlayerWrapper::SetInchpsteal` (`playerwrapper.cpp:3549-3559`): as classes
                    // da máscara 0xACE não recebem; a porcentagem com `+ 0.00001`.
                    if cls_do_alvo >= 0 && (1 << cls_do_alvo) & 0xACE != 0 {
                        return;
                    }
                    f.razao = (ap.razao * 100.0 + 0.00001) as i32;
                }
                if ap.tempo_s <= 0 {
                    return;
                }
                if let Some(classe) = efeito.dano_no_tempo() {
                    // `CalcMagicDamage/CalcPhysicDamage` com a defesa do alvo, a punição de
                    // nível contra monstro e ¼ entre jogadores (`playerwrapper.cpp` Set*).
                    let def = match classe {
                        None => defesa,
                        Some(i) => resist[i],
                    };
                    let mut dano = (ap.quantia
                        * (1.0 - combat::reducao_por_defesa(def, conjurador.nivel)))
                        as i32;
                    if e_jogador {
                        if conjurador.e_jogador {
                            dano = (0.25 * dano as f32) as i32;
                        }
                    } else {
                        dano = (dados
                            .progressao
                            .ajuste(conjurador.nivel - nivel_alvo)
                            .ataque
                            * dano as f32) as i32;
                    }
                    if dano <= 3 {
                        return;
                    }
                    // `filter_Wounded`: `_damage = damage / period` (mínimo 1), `_timeout =
                    // damage / _damage`.
                    f.por_segundo = (dano / ap.tempo_s).max(1);
                    f.restante_s = dano / f.por_segundo;
                } else {
                    match efeito {
                        Efeito::Hpgen | Efeito::Mpgen => {
                            f.por_segundo = ap.valor as i32 / ap.tempo_s
                        }
                        Efeito::Incsmite => f.por_segundo = ap.valor as i32,
                        // `SetWingshield` monta `filter_Wingshield(object, amount, value,
                        // time)` (`cskill/skill/playerwrapper.cpp:2327-2330`): o `SetAmount`
                        // é o escudo e o `SetValue` a mana por 3 s.
                        Efeito::Wingshield => {
                            f.absorve = ap.quantia;
                            f.por_segundo = ap.valor as i32;
                            if f.absorve < 6.0 {
                                return;
                            }
                        }
                        Efeito::Inchp => f.razao = (ap.razao * 100.0 + 0.00001) as i32,
                        // `SetFairyform`: `filter_Fairyform(object, time, (int)(100 * ratio +
                        // 0.00001f), (int)(100 * value + 0.00001f))`
                        // (`cskill/skill/playerwrapper.cpp:5247-5257`) — velocidade e defesa.
                        Efeito::Fairyform => {
                            f.razao = (ap.razao * 100.0 + 0.00001) as i32;
                            f.escala_defesa = (ap.valor * 100.0 + 0.00001) as i32;
                        }
                        Efeito::Inchurt if ap.razao <= 0.0 => return,
                        // `filter_Rebirth(object, time, (int)probability, ratio)`: a chance em
                        // `razao`, a vida em `fator` (0,01..1).
                        Efeito::Rebirth => {
                            f.razao = ap.probabilidade as i32;
                            f.fator = ap.razao.clamp(0.01, 1.0);
                        }
                        // `_ratio = ratio <= 1 ? 1 − ratio : 0,1` — guardado, sem uso: o
                        // `attack_attr < 0` que o dispara não acontece (ver o `Efeito`).
                        Efeito::Decregiondmg => {
                            f.fator = if ap.razao <= 1.0 { 1.0 - ap.razao } else { 0.1 }
                        }
                        Efeito::Dechurt if !(ap.razao > 0.001 && ap.razao < 0.99) => return,
                        Efeito::Invincible => {
                            // `SetInvincibleFilter(true, time)` + ícone só com `showicon`.
                            efs.invencivel_s = efs.invencivel_s.max(ap.tempo_s);
                            if !ap.mostra_icone {
                                return;
                            }
                        }
                        _ => {}
                    }
                }
                if efs.adicionar(f) {
                    mud.efeitos = true;
                    mud.atributos |= e_jogador;
                }
            }
        }
        if delta_hp != 0 || delta_mp != 0 {
            mud.efeitos = true;
            match alvo {
                Alvo::Jogador(id) => {
                    if let Some(p) = mundo.players.get_mut(&id) {
                        p.hp = (p.hp as i64 + delta_hp).clamp(0, p.max_hp as i64) as i32;
                        p.mp = (p.mp + delta_mp).clamp(0, p.max_mp);
                    }
                }
                Alvo::Monstro(id) => {
                    if let Some((m, ai)) = mundo.monsters.get_mut(&id) {
                        if delta_hp < 0 {
                            // `BeHurt` → `OnHurt` só registra o dano (`npc.cpp:1829-1845`): o
                            // ódio da habilidade é o `GetEnmity` (B137).
                            let _ = &ai;
                            m.registrar_dano(origem, (-delta_hp).min(m.hp));
                        }
                        m.hp = (m.hp + delta_hp).clamp(0, m.max_hp);
                        if m.hp == 0 && !m.is_dead {
                            m.is_dead = true;
                            m.efeitos.ao_morrer();
                            mud.morreu = true;
                            mundo.grid.remove_entity(id);
                        }
                    }
                }
                Alvo::Mascote(id) => {
                    if delta_hp < 0 {
                        mundo.dano_no_mascote(id, origem, -delta_hp);
                    } else if let Some(m) = mundo.mascotes.get_mut(&id) {
                        m.corpo.hp = (m.corpo.hp + delta_hp).min(m.corpo.max_hp);
                    }
                }
            }
        }
        let _ = hp;
    }

    /// Avisa os filtros de um objeto: vida (`SELF_INFO_00` do jogador), ficha e
    /// velocidade quando os realces mudaram, estado visível (124) e ícones (125) — e a
    /// forma (`PLAYER_CHGSHAPE`, 163) quando ela mudou.
    /// `gplayer_imp::ReturnToTown` (`gs/player.cpp:10949-10957`): sem ponto de cidade no
    /// distrito, `GetTownPosition` falha e o jogador fica onde está.
    pub(super) async fn voltar_para_a_cidade(&self, roleid: i32) {
        let destino = {
            let mundo = self.world.read().await;
            mundo.players.get(&(roleid as i64)).and_then(|p| {
                crate::progressao::ponto_de_renascimento(
                    &mundo.data_manager,
                    mundo.world_id,
                    p.position.x,
                    p.position.z,
                )
            })
        };
        match destino {
            Some((ponto, mapa)) => {
                info!("mundo: {roleid} volta para a cidade (mapa {mapa}, {ponto:?})");
                self.transportar(roleid, mapa, Vector3::new(ponto[0], ponto[1], ponto[2]))
                    .await;
            }
            None => debug!(
                "mundo: {roleid} pediu a volta para a cidade fora de distrito com ponto — fica"
            ),
        }
    }

    pub(super) async fn avisar_efeitos(&self, objeto: i64, atributos: bool) {
        let pacotes = {
            let mut mundo = self.world.write().await;
            if let Some(p) = mundo.players.get_mut(&objeto) {
                // `filter_Fairyform::OnAttach/OnRelease` chamam `ChangeShape`, que manda o
                // comando ao dono e a quem está em volta; a troca vem **antes** do ícone e da
                // velocidade (`skillfilter.h:16850-16873`). Só quando a forma de fato mudou:
                // repetir o comando faria o cliente recarregar o modelo à toa.
                let forma = p
                    .efeitos
                    .forma()
                    .map(|(shape, classe)| self.sub.byte_de_forma(shape | (classe << 6)));
                let troca_de_forma = (forma != p.forma_enviada).then(|| {
                    p.forma_enviada = forma;
                    S2CGamedataSend::player_change_shape(p.role_id, forma.unwrap_or(0)).data
                });
                let mut proprios = vec![Self::estado_proprio_de(p)];
                if atributos {
                    proprios.push(self.ficha_propria(p));
                    proprios.push(
                        S2CGamedataSend::ext_prop_move(
                            p.role_id,
                            p.walk_speed,
                            p.move_speed,
                            p.swim_speed,
                            p.fly_speed,
                        )
                        .data,
                    );
                }
                let mut todos: Vec<Vec<u8>> = troca_de_forma.into_iter().collect();
                todos.push(
                    self.sub
                        .update_ext_state(p.role_id, p.efeitos.estados_visiveis())
                        .data,
                );
                todos.push(
                    self.sub
                        .icon_state_notify(p.role_id, &p.efeitos.icones())
                        .data,
                );
                Some((Some(p.role_id), proprios, todos))
            } else if let Some((m, _)) = mundo.monsters.get(&objeto) {
                let todos = vec![
                    self.sub
                        .update_ext_state(objeto as i32, m.efeitos.estados_visiveis())
                        .data,
                    self.sub
                        .icon_state_notify(objeto as i32, &m.efeitos.icones())
                        .data,
                ];
                // A vida do monstro sob dano no tempo vai aos inscritos no batimento de 1 s
                // (`EventoDoMundo::VidaDoMonstro`), e não a todos em volta (B56).
                Some((None, Vec::new(), todos))
            } else if let Some(m) = mundo.mascotes.get(&objeto) {
                // O mascote é um NPC para quem vê (`gnpc_dispatcher`): o mesmo `UPDATE_EXT_STATE`
                // e os ícones — sem isto o atordoado do mascote não aparecia a ninguém (B140).
                let todos = vec![
                    self.sub
                        .update_ext_state(objeto as i32, m.corpo.efeitos.estados_visiveis())
                        .data,
                    self.sub
                        .icon_state_notify(objeto as i32, &m.corpo.efeitos.icones())
                        .data,
                ];
                Some((None, Vec::new(), todos))
            } else {
                None
            }
        };
        let Some((dono, proprios, todos)) = pacotes else {
            return;
        };
        if let Some(roleid) = dono {
            for p in proprios {
                self.enviar_ao_jogador(roleid, p).await;
            }
            for p in &todos {
                self.enviar_ao_jogador(roleid, p.clone()).await;
            }
        }
        for p in todos {
            self.transmitir_a_outros(dono.unwrap_or(0), p).await;
        }
    }

    /// Um monstro morreu de dano no tempo.
    /// `npc_died` a todos e o espólio: experiência, drop e fim das sessões de golpe. É o
    /// caminho único de morte de monstro — golpe normal (adiado), habilidade e dano no tempo.
    pub(super) async fn anunciar_morte_do_monstro(&self, id: i64, matador: i64) {
        info!("mundo: {matador} matou {id}");
        let morte = S2CGamedataSend::npc_died(id as i32, matador as i32).data;
        self.transmitir_a_outros(0, morte).await;
        self.monstro_morreu(id).await;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_cilindro_e_o_setor_pegam_o_que_esta_no_caminho() {
        let o = Vector3::new(0.0, 0.0, 0.0);
        let fim = Vector3::new(10.0, 0.0, 0.0);
        assert!(no_cilindro(o, fim, Vector3::new(5.0, 0.0, 1.5), 2.0));
        assert!(!no_cilindro(o, fim, Vector3::new(5.0, 0.0, 2.5), 2.0));
        assert!(!no_cilindro(o, fim, Vector3::new(11.0, 0.0, 0.0), 2.0));
        // Cone de 60° (cos 30° ≈ 0,866) para +x, alcance 10.
        assert!(no_setor(o, fim, Vector3::new(8.0, 0.0, 3.0), 10.0, 0.866));
        assert!(!no_setor(o, fim, Vector3::new(3.0, 0.0, 5.0), 10.0, 0.866));
        assert!(!no_setor(o, fim, Vector3::new(-5.0, 0.0, 0.0), 10.0, 0.866));
    }

    /// `SetEntrap` (`playerwrapper.cpp:2466-2468`; a mesma conta no `gs` 1.2.6, VA 0x8305982):
    /// vida cheia não captura; o Gato de Presas Afiadas (3316, nível 5 no 1.2.6) a 10 % da vida
    /// com a 328 no nível 1: 0,81 × 100 × 1,40 = 113,4 % — captura certa.
    #[test]
    fn a_chance_de_captura_e_a_do_original() {
        assert_eq!(chance_de_captura(100, 100, 5, 1), 0.0);
        let meio = chance_de_captura(50, 100, 5, 1);
        assert!((meio - 25.0 * 1.40).abs() < 1e-3, "{meio}");
        let quase = chance_de_captura(10, 100, 5, 1);
        assert!((quase - 81.0 * 1.40).abs() < 1e-2, "{quase}");
        // `GetLevel() / 100` é divisão inteira: nível 150 tira 1 inteiro.
        let alto = chance_de_captura(50, 100, 150, 1);
        assert!((alto - 25.0 * 0.40).abs() < 1e-3, "{alto}");
    }

    /// B137 — o `GetEnmity` de cada versão: por nível (a 150 do 1.2.6, 200 × L, contra
    /// 300 × L no 1.5.5) e por expressão com o nível de quem lança (a 747, `10 × P_Level ×
    /// (3 + L)`, nas duas).
    #[test]
    fn o_odio_da_habilidade_sai_do_stub_de_cada_versao() {
        use pw_data_loader::habilidades::TabelaDeHabilidades;
        let t126 = TabelaDeHabilidades::do_126();
        let t155 = TabelaDeHabilidades::do_155();
        assert_eq!(odio_da_habilidade(&t126.por_id[&150], 2, 50), 400);
        assert_eq!(odio_da_habilidade(&t155.por_id[&150], 2, 50), 600);
        assert_eq!(odio_da_habilidade(&t126.por_id[&747], 1, 50), 2000);
        assert_eq!(odio_da_habilidade(&t155.por_id[&747], 3, 20), 1200);
        let sem = t155.por_id.values().filter(|h| h.odio.is_none() && h.odio_expr.is_none()).count();
        assert_eq!(sem, 0, "habilidade sem ódio lido");
    }
}
