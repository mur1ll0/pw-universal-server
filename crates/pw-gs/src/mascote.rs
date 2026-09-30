//! Mascote de combate — porte do lado do dono (`gs/petman.cpp`, `combat_petdata_imp` e
//! `pet_manager`) e da criatura no mundo (`gs/petnpc.cpp`, `gpet_imp` e `gpet_policy`).
//!
//! # Como o original funciona
//!
//! Invocar põe no mundo um NPC (`npc_spawner::CreatePetBase`, `npcgenerator.cpp:1989-2139`)
//! com id `MERGE_PET_ID` — o bit 29 (`PET_MASK`, `common/types.h:214`) sobre o de NPC —, os
//! atributos do `PET_ESSENCE` no nível do mascote (`pet_dataman::GenerateBaseProp`) e a vida
//! em `hp_factor` do máximo. A criatura segue o dono (`ai_pet_follow_master`), ataca quem o
//! dono mandar (`DispatchPlayerCommand`), e sozinha conforme a agressividade:
//!
//! | agressividade | o que dispara o ataque |
//! | :--- | :--- |
//! | 0 defesa | apanhar, ou o dono apanhar (`GM_MSG_MASTER_ASK_HELP`, `petnpc.cpp:664-682`) |
//! | 1 automático | o dono começar a atacar alguém (`GM_MSG_PET_AUTO_ATTACK`, `:683-702`) |
//! | 2 passivo | só a ordem do dono |
//!
//! O golpe do mascote leva o **dono** como atacante (`gpet_imp::FillAttackMsg`,
//! `petnpc.cpp:1208-1243`): o crédito da morte, a experiência e a contagem de missão são do
//! jogador. O monstro que apanha põe ódio no mascote e, em menor grau, no dono
//! (`AddAggroEntry` 3 e 1, `npc.cpp:1781-1785`). O mascote avisa o dono da vida a cada 5
//! batimentos ou quando algo muda (`NotifyMasterInHeartbeat`, `:1318-1357`), e morre
//! avisando (`NotifyDeathToMaster`).

use crate::ai::{AcaoDoMonstro, Habitat, MODO_CORRER, MODO_VOLTAR};
use crate::entity::MonsterEntity;
use crate::navegacao::{Mapa, SeguirAlvo, V3};
use pw_core::{InfoPet, Vector3};
use pw_data_loader::pet::ModeloDeMascote;
use std::collections::HashMap;

/// `PET_MASK` (`common/types.h:214`).
pub const PET_MASK: u32 = 0x2000_0000;

/// O id de NPC de um mascote: `MERGE_PET_ID(índice)` (`common/types.h:330`).
pub fn id_do_mascote(indice: u32) -> i64 {
    (0x8000_0000u32 | PET_MASK | (indice & 0x1FFF_FFFF)) as i32 as i64
}

/// É id de mascote (`IS_PET`, `common/types.h:254`).
pub fn e_mascote(id: i64) -> bool {
    let u = id as i32 as u32;
    u & 0x8000_0000 != 0 && u & PET_MASK != 0
}

/// `gpet_imp::PET_AGGRO_*` (`petnpc.h:125-127`).
pub const AGRESSIVIDADE_DEFESA: u8 = 0;
pub const AGRESSIVIDADE_AUTOMATICA: u8 = 1;
pub const AGRESSIVIDADE_PASSIVA: u8 = 2;
/// `PET_MOVE_FOLLOW` / `PET_STAY_STAY` (`petnpc.h:132-133`).
pub const MOVIMENTO_SEGUIR: u8 = 0;
pub const MOVIMENTO_FICAR: u8 = 1;

/// `pet_manager::PET_DEATH` (`petman.h:120-126`), motivo do `RECALL_PET`.
pub const RECOLHIDO_POR_MORTE: u8 = 1;

/// `honor_level_list` (`petman.cpp:292-300`): até 50, 150, 500 e 999 pontos.
pub fn nivel_de_lealdade(pontos: i32) -> usize {
    [50, 150, 500, 999].iter().position(|&l| pontos <= l).unwrap_or(3)
}

/// `combat_petdata_imp::GetExpAdjust` (`petman.cpp:568-574`).
pub fn ajuste_de_exp(nivel_de_lealdade: usize) -> f32 {
    [0.1, 0.5, 1.0, 1.5][nivel_de_lealdade.min(3)]
}

/// `__pet_damage_adjust` do `pet_damage_filter` (`pet_filter.cpp:8`), em pontos percentuais
/// do dano.
pub fn ajuste_de_dano(nivel_de_lealdade: usize) -> i32 {
    [-40, -20, 0, 20][nivel_de_lealdade.min(3)]
}

/// `pet_data::HONOR_POINT_MAX` (`petman.h:102`).
pub const LEALDADE_MAXIMA: i32 = 999;

/// `pet_data::FEED_TIME_UNIT` (`petman.h:91`): a cada 300 batimentos de 1 s a fome sobe.
pub const PERIODO_DE_COMIDA_S: i32 = 300;

/// `__pet_feed_param_list` (`petman.cpp:1645-1666`), por nível de fome (0..=11):
/// `(fator da lealdade ao comer, quanto a comida tira da fome, lealdade perdida no período,
/// fome ganha no período)`.
pub const TABELA_DE_COMIDA: [(f32, i32, i32, i32); 12] = [
    (1.0, 0, 0, 1),
    (0.8, 1, -1, 1),
    (0.6, 1, -5, 1),
    (0.6, 1, -5, 1),
    (0.8, 2, -15, 1),
    (0.8, 2, -15, 1),
    (0.8, 2, -15, 1),
    (0.6, 3, -50, 1),
    (0.6, 3, -50, 1),
    (0.6, 3, -50, 1),
    (0.6, 3, -50, 1),
    (0.3, 4, -100, 1),
];

/// `COOLDOWN_INDEX_FEED_PET` (18, `cooldowncfg.h:45-63`) e `PET_FOOD_COOLDOWN_TIME`
/// (60 s, `:18`).
pub const RECARGA_DA_COMIDA: i32 = 18;
pub const RECARGA_DA_COMIDA_MS: i32 = 60_000;

/// `ERR_PET_IS_NOT_ACTIVE` 73 e `ERR_PET_FOOD_TYPE_NOT_MATCH` 74 (`common/protocol.h`).
pub const ERRO_SEM_MASCOTE: i32 = 73;
pub const ERRO_COMIDA_ERRADA: i32 = 74;

fn mudar_lealdade(info: &mut InfoPet, delta: i32) {
    info.honor_point = (info.honor_point + delta).clamp(0, LEALDADE_MAXIMA);
}

fn mudar_fome(info: &mut InfoPet, delta: i32) {
    info.hunger = (info.hunger + delta).clamp(0, TABELA_DE_COMIDA.len() as i32 - 1);
}

/// O corpo de combate do mascote: um `MonsterEntity` com os atributos de
/// `GenerateBaseProp` no nível dele e a vida em `hp_factor` do máximo
/// (`CreatePetBase`, `npcgenerator.cpp:2021-2040`). O alcance soma o raio do corpo
/// (`_cur_prop.attack_range += body_size`), e o dano leva o ajuste de lealdade.
pub fn corpo_do_mascote(id: i64, modelo: &ModeloDeMascote, info: &InfoPet, pos: Vector3) -> MonsterEntity {
    let nivel = info.level.max(1) as i32;
    let a = modelo.atributos(nivel);
    let mut m = MonsterEntity::placeholder(id, modelo.tid, pos, 0);
    let vida = a.vida.max(1) as i64;
    let dano = (a.dano as i64 * (100 + ajuste_de_dano(nivel_de_lealdade(info.honor_point)) as i64) / 100).max(0) as i32;
    m.name = String::new();
    m.level = nivel;
    m.max_hp = vida;
    m.hp = ((vida as f32 * info.hp_factor) as i64).clamp(1, vida);
    m.mp = 0;
    m.max_mp = 0;
    m.def_phys = a.defesa;
    m.armor = a.esquiva;
    m.attack_rate = a.acerto;
    m.resistances = [a.resistencia; 5];
    m.attack_degree = 0;
    m.defend_degree = 0;
    m.attack_min = dano;
    m.attack_max = dano;
    m.magic_attack = [(0, 0); 5];
    m.attack_range = a.alcance + modelo.corpo;
    // `body_size` do mascote: o `info.body_size` que o monstro soma ao alcance (B139).
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

/// Um mascote de combate no mundo: o corpo, a IA, o dono e o registro da jaula que ele
/// representa (`pet_data`), que volta ao banco com nível, experiência e vida.
#[derive(Debug, Clone)]
pub struct Mascote {
    /// `_inhabit_type` do `PET_ESSENCE` (0..=6); o modo em uso fica em `corpo.habitat`.
    pub tipo_de_habitat: i32,
    /// A camada do dono da última vez que o mascote a viu (`GM_MSG_MASTER_NOTIFY_LAYER`).
    pub camada_do_dono: crate::combat::Camada,
    pub corpo: MonsterEntity,
    pub ai: MascoteAi,
    pub dono: i64,
    /// O índice na jaula (`_cur_active_pet`).
    pub slot: u16,
    pub info: InfoPet,
    /// O modelo visual (`pet_vis_tid`, ou o próprio `pet_tid`).
    pub vis_tid: u32,
    /// `pet_data::name`, até 16 bytes.
    pub nome: Vec<u8>,
    /// O que já foi dito ao dono no último `PET_HP_NOTIFY` e o contador dos 5 batimentos
    /// (`NotifyMasterInHeartbeat`).
    pub vida_avisada: i64,
    /// Mascote **ornamental** ("Ver Mascote", `PET_ESSENCE.id_type` 8783 →
    /// `PET_CLASS_FOLLOW`, `petdataman.cpp:32-33`; igual no `gs` 1.2.6, VA 0x814364b). No
    /// original é o de combate com quase tudo desligado (B154):
    /// - dono: `follow_petdata_imp` (`petman.cpp:1006-1077`) — sem nível, lealdade, comandos
    ///   (`OnPetCtrl` false), experiência por abate, ajuda ao dono, habilidades nem tinta; morto
    ///   é só recolhido, com a vida cheia;
    /// - no mundo: `gpet_imp_2` (`petnpc.cpp:1819-1856`, `petnpc.h:230-236`) — ignora golpe,
    ///   bênção/maldição, comando, ódio, ataque automático e pedido de ajuda, e não se anuncia
    ///   aos monstros (`PeepEnemy` vazio). Só segue o dono.
    pub ornamental: bool,
    pub combate_avisado: bool,
    pub batimentos_sem_aviso: u32,
    batimento_ms: u32,
    /// Quantos batimentos de 1 s já passaram (para o relógio da comida).
    batimentos_dados: u64,
}

impl Mascote {
    pub fn novo(id: i64, dono: i64, slot: u16, info: InfoPet, modelo: &ModeloDeMascote, pos: Vector3, agressividade: u8, movimento: u8) -> Self {
        let corpo = corpo_do_mascote(id, modelo, &info, pos);
        let nome = info.name[..(info.name_len as usize).min(16)].to_vec();
        let vis_tid = if info.pet_vis_tid > 0 { info.pet_vis_tid } else { info.pet_tid } as u32;
        let mut ai = MascoteAi::new(agressividade, movimento, pos);
        ai.raio_do_corpo = modelo.corpo;
        ai.altura_ao_seguir = if modelo.habitat != 0 { 1.5 } else { 0.0 };
        Self {
            tipo_de_habitat: modelo.habitat,
            camada_do_dono: crate::combat::Camada::Chao,
            ai,
            corpo,
            dono,
            slot,
            info,
            vis_tid,
            nome,
            vida_avisada: -1,
            ornamental: modelo.classe == pw_core::PET_CLASS_FOLLOW,
            combate_avisado: false,
            batimentos_sem_aviso: 0,
            batimento_ms: 0,
            batimentos_dados: 0,
        }
    }

    /// `gactive_imp::GetSkillLevel`: o nível da habilidade no `pet_data::skills`, que o
    /// `CreatePetBase` põe no corpo (`npcgenerator.cpp:2098-2106`, parando no primeiro vazio).
    pub fn nivel_da_habilidade(&self, id: i32) -> i32 {
        if id <= 0 {
            return 0;
        }
        self.info.skills.iter().take_while(|(s, _)| *s > 0).find(|(s, _)| *s == id).map(|(_, l)| *l).unwrap_or(0)
    }

    /// Batimentos de 1 s já dados.
    pub fn batimentos(&self) -> u64 {
        self.batimentos_dados
    }

    /// `hp_factor` como o original guarda: vida / máximo.
    pub fn fator_de_vida(&self) -> f32 {
        if self.corpo.max_hp <= 0 {
            0.0
        } else {
            self.corpo.hp as f32 / self.corpo.max_hp as f32
        }
    }

    /// O batimento de 1 s: regeneração e, a cada 5 batimentos ou quando a vida ou o combate
    /// mudam, o aviso ao dono (`NotifyMasterInHeartbeat`). Devolve se o aviso deve sair.
    pub fn batimento(&mut self, modelo: &ModeloDeMascote, delta_ms: u32) -> bool {
        self.batimento_ms += delta_ms;
        if self.batimento_ms < 1000 || self.corpo.is_dead {
            return false;
        }
        self.batimento_ms -= 1000;
        self.batimentos_dados += 1;
        if self.corpo.hp < self.corpo.max_hp {
            self.corpo.hp = (self.corpo.hp + regeneracao(modelo, self.corpo.level)).min(self.corpo.max_hp);
        }
        self.batimentos_sem_aviso += 1;
        let combate = self.ai.em_combate();
        let avisar = self.batimentos_sem_aviso >= 5 || self.vida_avisada != self.corpo.hp || combate != self.combate_avisado;
        if avisar {
            self.batimentos_sem_aviso = 0;
            self.vida_avisada = self.corpo.hp;
            self.combate_avisado = combate;
        }
        avisar
    }

    /// Um segundo do relógio da comida (`DoHeartbeat` com `_need_feed`, `petman.cpp:1616-1636`;
    /// `HandleFeedTimeTick`, `:1697-1711`). Devolve `(lealdade, fome)` quando o período
    /// fechou e o dono deve ser avisado.
    pub fn passar_tempo_de_comida(&mut self, modelo: &ModeloDeMascote) -> Option<(i32, i32)> {
        self.info.feed_time += 1;
        if self.info.feed_time < PERIODO_DE_COMIDA_S {
            return None;
        }
        self.info.feed_time = 0;
        let h = self.info.hunger.clamp(0, TABELA_DE_COMIDA.len() as i32 - 1) as usize;
        let (_, _, perde, fome) = TABELA_DE_COMIDA[h];
        let antes = nivel_de_lealdade(self.info.honor_point);
        mudar_fome(&mut self.info, fome);
        mudar_lealdade(&mut self.info, perde);
        if nivel_de_lealdade(self.info.honor_point) != antes {
            self.refazer_dano(modelo);
        }
        Some((self.info.honor_point, self.info.hunger))
    }

    /// `FeedCurPet` (`petman.cpp:1713-1750`): a comida tem de estar no `food_mask` do modelo;
    /// a lealdade sobe `fator × honra da comida` e a fome desce. `Err` com o `ERR_*`.
    pub fn alimentar(&mut self, modelo: &ModeloDeMascote, tipo: i32, honra: i32) -> Result<(i32, i32), i32> {
        if modelo.comida & tipo == 0 {
            return Err(ERRO_COMIDA_ERRADA);
        }
        let h = self.info.hunger.clamp(0, TABELA_DE_COMIDA.len() as i32 - 1) as usize;
        let (fator, tira, _, _) = TABELA_DE_COMIDA[h];
        let antes = nivel_de_lealdade(self.info.honor_point);
        mudar_lealdade(&mut self.info, (fator * honra as f32 + 0.5) as i32);
        mudar_fome(&mut self.info, -tira);
        self.info.feed_time = 0;
        if nivel_de_lealdade(self.info.honor_point) != antes {
            self.refazer_dano(modelo);
        }
        Ok((self.info.honor_point, self.info.hunger))
    }

    /// `GM_MSG_PET_HONOR_MODIFY` → `SetHonorLevel` → o `pet_damage_filter` com o ajuste novo.
    fn refazer_dano(&mut self, modelo: &ModeloDeMascote) {
        let novo = corpo_do_mascote(self.corpo.id, modelo, &self.info, self.corpo.position);
        self.corpo.attack_min = novo.attack_min;
        self.corpo.attack_max = novo.attack_max;
    }

    /// Guarda no registro da jaula o que a criatura tem agora (a vida em fator).
    pub fn para_a_jaula(&self) -> InfoPet {
        let mut info = self.info.clone();
        info.hp_factor = if self.corpo.is_dead { 0.0 } else { self.fator_de_vida().max(0.0) };
        info
    }

    /// Subiu de nível (`GM_MSG_PET_LEVEL_UP`, `petnpc.cpp:606-622`): atributos do nível novo e
    /// vida cheia.
    pub fn subir_de_nivel(&mut self, modelo: &ModeloDeMascote) {
        let pos = self.corpo.position;
        let mut corpo = corpo_do_mascote(self.corpo.id, modelo, &self.info, pos);
        corpo.hp = corpo.max_hp;
        corpo.efeitos = std::mem::take(&mut self.corpo.efeitos);
        self.corpo = corpo;
    }
}

/// `pet_gen_pos::FindGroundPos` (`petman.cpp:1-31`; o mesmo no `gs` 1.2.6,
/// `combat_petdata_imp::FindGroundPos` VA 0x8147752, 10 voltas e o 6,8 em 0x8504678): até
/// 10 sorteios a `±Rand(0,8..1,2)` m do dono em `x` e em `z`, cada um no terreno + piso do
/// mapa de movimento (`path_finding::GetValidPos`), recusando o inalcançável e o que fica a
/// 6,8 m ou mais da altura do dono. Mapa sem terreno fica na altura do dono.
pub fn posicao_no_chao(terreno: &pw_data_loader::Terreno, movimento: &pw_data_loader::MapaDeMovimento, dono: Vector3) -> Option<Vector3> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..10 {
        let (ox, oz) = (rng.gen_range(0.8f32..1.2), rng.gen_range(0.8f32..1.2));
        let x = dono.x + if rng.gen_bool(0.5) { ox } else { -ox };
        let z = dono.z + if rng.gen_bool(0.5) { oz } else { -oz };
        let Some(acima) = movimento.acima_do_terreno(x, z) else { continue };
        let Some(chao) = terreno.altura_em(x, z) else {
            return Some(Vector3::new(x, dono.y, z));
        };
        let y = chao + acima;
        if (y - dono.y).abs() >= 6.8 {
            continue;
        }
        return Some(Vector3::new(x, y, z));
    }
    None
}

/// Os três `Find*Pos` do `pet_gen_pos` (`petman.cpp:10-87`) num só: até 10 sorteios a
/// `dis ± Rand(0,8..1,2)` m de `base` em `x` e em `z`, somando `offset` a cada volta.
/// * chão: terreno + piso do mapa de movimento, recusando 6,8 m ou mais da altura de `base`;
/// * ar: `base.y + 1`, nunca abaixo de terreno + 1,5 nem de água + 1,5;
/// * água: `base.y + 1`, entre terreno + 1 e água − 1 (sem água suficiente, recusa).
///
/// No ar e na água o sorteio ainda passa pelo `IsValidSPPos`: folha livre do `airmap/` (B133).
#[allow(clippy::too_many_arguments)]
fn procurar_posicao(
    ambiente: Habitat,
    terreno: &pw_data_loader::Terreno,
    movimento: &pw_data_loader::MapaDeMovimento,
    agua: &pw_data_loader::MapaDeAgua,
    espaco: Option<&pw_data_loader::MapaDoEspaco>,
    base: Vector3,
    dis: f32,
    offset: Vector3,
) -> Option<Vector3> {
    // `path_finding::IsValidSPPos` (`pathfinding.cpp:46-51`) no ar e na água: folha livre do
    // `airmap/`. Sem o mapa do espaço, vale (B133).
    let livre = |p: Vector3| match espaco {
        Some(e) if e.tem_dados() => e.livre(e.centro_do_voxel([p.x, p.y, p.z])).is_some(),
        _ => true,
    };
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut off = Vector3::new(0.0, 0.0, 0.0);
    for _ in 0..10 {
        off = Vector3::new(off.x + offset.x, off.y + offset.y, off.z + offset.z);
        let (ox, oz) = (dis + rng.gen_range(0.8f32..1.2), dis + rng.gen_range(0.8f32..1.2));
        let x = base.x + off.x + if rng.gen_bool(0.5) { ox } else { -ox };
        let z = base.z + off.z + if rng.gen_bool(0.5) { oz } else { -oz };
        let y0 = base.y + off.y;
        let chao = terreno.altura_em(x, z);
        match ambiente {
            Habitat::Chao => {
                let Some(acima) = movimento.acima_do_terreno(x, z) else { continue };
                let Some(chao) = chao else {
                    return Some(Vector3::new(x, y0, z));
                };
                let y = chao + acima;
                if (y - y0).abs() >= 6.8 {
                    continue;
                }
                return Some(Vector3::new(x, y, z));
            }
            Habitat::Ar => {
                let mut y = y0 + 1.0;
                if let Some(h) = chao {
                    y = y.max(h + 1.5);
                }
                let superficie = agua.altura_em(x, z);
                if superficie != pw_data_loader::watermap::SEM_AGUA {
                    y = y.max(superficie + 1.5);
                }
                let p = Vector3::new(x, y, z);
                if !livre(p) {
                    continue;
                }
                return Some(p);
            }
            Habitat::Agua => {
                let superficie = agua.altura_em(x, z);
                if superficie == pw_data_loader::watermap::SEM_AGUA {
                    continue;
                }
                let piso = chao.map(|h| h + 1.0);
                let mut y = y0 + 1.0;
                if let Some(p) = piso {
                    y = y.max(p);
                }
                if y > superficie - 1.0 {
                    y = superficie - 1.0;
                    if piso.is_some_and(|p| y < p) {
                        continue;
                    }
                }
                let p = Vector3::new(x, y, z);
                if !livre(p) {
                    continue;
                }
                return Some(p);
            }
        }
    }
    None
}

/// `pet_gen_pos::FindValidPos` (`petman.cpp:127-286`): o `inhabit_type` do mascote (0 chão,
/// 1 água, 2 ar, 3 chão+água, 4 chão+ar, 5 água+ar, 6 todos) contra a camada do dono decide se
/// ele pode aparecer (`Err(-3)`) e em que ordem se procura o lugar — o de mais de um ambiente
/// tenta primeiro o do dono. Devolve o ponto e o modo (`inhabit_mode`); sem lugar, `Err(-4)`.
/// O 1.2.6 só tem os tipos 0..=2 (o leitor zera o resto, VA 0x8143893).
#[allow(clippy::too_many_arguments)]
pub fn posicao_para_mascote(
    terreno: &pw_data_loader::Terreno,
    movimento: &pw_data_loader::MapaDeMovimento,
    agua: &pw_data_loader::MapaDeAgua,
    espaco: Option<&pw_data_loader::MapaDoEspaco>,
    sem_voo: bool,
    base: Vector3,
    camada_do_dono: crate::combat::Camada,
    tipo_de_habitat: i32,
    dis: f32,
    offset: Vector3,
) -> Result<(Vector3, Habitat), i32> {
    use crate::combat::Camada as C;
    // `GetWorldLimit().nofly`: o de ar puro não aparece (`petman.cpp:140`, -2); o de mais de
    // um ambiente pula o ar (`:270`).
    if sem_voo && tipo_de_habitat == 2 {
        return Err(-2);
    }
    let pode = match tipo_de_habitat {
        0 => camada_do_dono == C::Chao,
        1 => camada_do_dono == C::Agua,
        2 | 4 => camada_do_dono != C::Agua,
        3 => camada_do_dono != C::Ar,
        5 | 6 => true,
        _ => return Err(-2),
    };
    if !pode {
        return Err(-3);
    }
    let ordem: &[Habitat] = match (tipo_de_habitat, camada_do_dono) {
        (0, _) => &[Habitat::Chao],
        (1, _) => &[Habitat::Agua],
        (2, _) => &[Habitat::Ar],
        (3, C::Agua) => &[Habitat::Agua, Habitat::Chao],
        (3, _) => &[Habitat::Chao, Habitat::Agua],
        (4, C::Ar) => &[Habitat::Ar, Habitat::Chao],
        (4, _) => &[Habitat::Chao, Habitat::Ar],
        (5, C::Agua) => &[Habitat::Agua, Habitat::Ar],
        (5, _) => &[Habitat::Ar, Habitat::Agua],
        (6, C::Agua) => &[Habitat::Agua, Habitat::Chao, Habitat::Ar],
        (6, C::Ar) => &[Habitat::Ar, Habitat::Chao, Habitat::Agua],
        _ => &[Habitat::Chao, Habitat::Ar, Habitat::Agua],
    };
    for amb in ordem {
        if sem_voo && *amb == Habitat::Ar {
            continue;
        }
        if let Some(p) = procurar_posicao(*amb, terreno, movimento, agua, espaco, base, dis, offset) {
            return Ok((p, *amb));
        }
    }
    Err(-4)
}

/// `gpet_imp::TryChangeInhabitMode` (`petnpc.cpp:313-355`), no `GM_MSG_MASTER_NOTIFY_LAYER`
/// (`:791-802`): o dono mudou de camada. Só o mascote de mais de um ambiente tenta o outro
/// modo (0, 1 e 2 ficam com o dono, que o recolhe); o de chão+água não vai ao ar, o de
/// chão+ar não vai à água, o de água+ar no ar fica no ar com o dono no chão.
pub fn tenta_trocar_de_modo(tipo_de_habitat: i32, camada_do_dono: crate::combat::Camada, modo_atual: Habitat) -> bool {
    use crate::combat::Camada as C;
    match tipo_de_habitat {
        3 => camada_do_dono != C::Ar,
        4 => camada_do_dono != C::Agua,
        5 => !(camada_do_dono == C::Chao && modo_atual == Habitat::Ar),
        6 => true,
        _ => false,
    }
}

/// A regeneração do batimento de 1 s: `GenHPandMP(hp_gen)` — o mascote tem `SetFastRegen(0)`,
/// então é sempre a lenta (`gnpc_imp::OnHeartbeat`, `npc.cpp:1948-1957`).
pub fn regeneracao(modelo: &ModeloDeMascote, nivel: i32) -> i64 {
    modelo.atributos(nivel.max(1)).regeneracao.max(0) as i64
}

/// `COOLINGID_BEGIN` (`cskill/skill/playerwrapper.h:22`): a recarga de uma habilidade é
/// guardada em `id + 1024` (`SkillWrapper::GetCooldownID`, `skillwrapper.cpp:1495-1498`).
pub const COOLINGID_BEGIN: i32 = 1024;

/// `ERR_PET_SKILL_IN_COOLDOWN` (93 no 1.5.5, `common/protocol.h:773`; o mesmo `push 0x5d` no
/// `gpet_imp::NotifySkillStillCoolDown` do `gs` 1.2.6, VA 0x813a1c1).
pub const ERRO_HABILIDADE_EM_RECARGA: i32 = 93;

/// `range.type` das habilidades que não pedem alvo: 2 bola em si e 5 em si
/// (`cskill/skill/range.h:18-25`; `DispatchPlayerCommand`, `petnpc.cpp:1071-1072`).
pub fn area_sem_alvo(area: i32) -> bool {
    area == 2 || area == 5
}

/// Uma habilidade do mascote com o que o mundo precisa para conjurá-la, lida do catálogo do
/// servidor no nível que o mascote tem (`pet_data::skills`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HabilidadeDoMascote {
    pub id: i32,
    pub nivel: i32,
    /// `GetType`: 1 ataque, 2 bênção, 3 maldição…
    pub tipo: i32,
    /// `RangeType` (`range.type`).
    pub area: i32,
    /// `GetMagicRange` = `GetPraydistance` (sem o raio do corpo).
    pub alcance: f32,
    /// `State1::GetTime` — o canto: é o `time` do `OBJECT_CAST_SKILL` e o que falta até o
    /// efeito (`SkillWrapper::NpcStart`, `skillwrapper.cpp:974-1004`).
    pub canto_ms: u32,
    /// `State2::GetTime` — a execução depois do efeito; 0 vira 20 tiques
    /// (`session_npc_skill::StartSession`, `npcsession.cpp:718-721`).
    pub execucao_ms: u32,
    /// `GetCoolingtime` armado como o original: segundos truncados × 1000
    /// (`PlayerWrapper::SetPerform`, `playerwrapper.cpp:170`).
    pub recarga_ms: i32,
    /// `GetMpCost`. É 0 em todas as habilidades de mascote dos dois catálogos — e o de combate
    /// tem `max_mp` 0 (`GenerateBaseProp`, `petdataman.cpp:180`), então o `CheckMp` passa.
    pub mana: i32,
}

impl HabilidadeDoMascote {
    pub fn recarga(&self) -> i32 {
        self.id + COOLINGID_BEGIN
    }
}

/// `ai_pet_skill_task` (`aipolicy.h:733-748`): a habilidade e o alvo, com as duas perseguições
/// que o `ai_skill_task_2::Execute` permite antes de conjurar de onde está (`_trace_count`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TarefaDeHabilidade {
    pub habilidade: HabilidadeDoMascote,
    pub alvo: Option<i64>,
    rastros: u8,
}

/// `session_npc_skill`: o canto até o efeito e a execução depois dele.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Conjuracao {
    Canto { habilidade: HabilidadeDoMascote, alvo: Option<i64>, falta_ms: u32 },
    Execucao { falta_ms: u32 },
}

/// O que o mascote decidiu no tique.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AcaoDoMascote {
    Moveu(AcaoDoMonstro),
    /// Golpe no alvo, com o dano já resolvido (acerto, defesa, lealdade).
    Atacou { alvo: i64, dano: i32 },
    /// `RelocatePetPos(false)`: o dono o põe ao lado dele (`OnPetRelocate` →
    /// `GM_MSG_PET_CHANGE_POS` → `stop_move(pos, 0x500, 1, MOVE_MODE_RETURN)`).
    Reposicionar,
    /// `RelocatePetPos(true)` ou o dono sumiu: recolher (`GM_MSG_PET_DISAPPEAR`).
    Sumir,
    /// Começou a conjurar (`NpcStart` → `SendClientMsgSkillCasting`): `OBJECT_CAST_SKILL` a
    /// quem vê. `alvo` `None` é a habilidade em si.
    Conjurou { habilidade: HabilidadeDoMascote, alvo: Option<i64> },
    /// O canto acabou (`RepeatSession` → `NpcEnd` → `SetPerform`): o efeito sai e a recarga
    /// fica armada (`gpet_imp::SetCoolDown`, que avisa o dono).
    UsouHabilidade { habilidade: HabilidadeDoMascote, alvo: Option<i64> },
    /// A sessão abriu com a recarga armada (`NotifySkillStillCoolDown`, `petnpc.cpp:277-285`):
    /// `ERR_PET_SKILL_IN_COOLDOWN` ao dono, e nada sai.
    HabilidadeEmRecarga,
}

/// A IA do mascote (`gpet_policy` + as tarefas de alvo). Um passo de perseguição a cada
/// 500 ms (`NPC_FOLLOW_TARGET_TIME`), como o monstro.
#[derive(Debug, Clone)]
pub struct MascoteAi {
    /// `PetRelocatePos(false)` pedido fora do tique (troca de modo sem lugar, B131).
    reposicao_pedida: bool,
    pub agressividade: u8,
    pub movimento: u8,
    /// `aggro_policy::STATE_FREEZE`: não ganha ódio por apanhar.
    pub congelado: bool,
    pub odio: HashMap<i64, i64>,
    pub ponto_de_parada: Vector3,
    espera_ms: u32,
    recarga_ms: u32,
    batimento_ms: u32,
    parado: bool,
    pub direcao: u8,
    seguir: Option<SeguirAlvo>,
    /// O `follow_target` do mascote de ar ou de água (B133).
    no_espaco: Option<crate::navegacao::SeguirNoEspaco>,
    falhas_de_caminho: u32,
    /// A `ai_pet_follow_master` em curso: começa no batimento de 1 s com o dono a mais de
    /// 1,5 m (`petnpc.cpp:1712-1716`) e acaba a menos de 0,8 m (`session_npc_follow_target`).
    seguindo: bool,
    /// `gpet_policy::_auto_skill_*` (`SetPetAutoSkill`, `petnpc.cpp:1812-1818`).
    pub automatica: Option<HabilidadeDoMascote>,
    /// A `ai_pet_skill_task` pendente (ordem do dono ou a automática).
    pub tarefa: Option<TarefaDeHabilidade>,
    conjuracao: Option<Conjuracao>,
    /// `gpet_imp::_cooldown`: o que falta de cada recarga, em ms, pelo id da recarga.
    pub recargas: HashMap<i32, u32>,
    /// O raio do corpo, que o alcance da habilidade soma (`ai_pet_skill_task::StartTask`,
    /// `aipolicy.cpp:1922-1927`).
    pub raio_do_corpo: f32,
    /// O `_height_offset` do seguir o dono: 1,5 m para quem não é só de chão (B135).
    pub altura_ao_seguir: f32,
    /// A `session_npc_attack` em curso: enquanto dura, o golpe vale até `attack_range + corpo
    /// do alvo` (`CheckAttack`, `actobject.cpp:1280-1287`); para começar, o `ai_melee_task`
    /// exige o alcance menor (B136).
    golpeando: bool,
    /// De quem é a sessão de movimento em curso: cada tarefa (golpe, habilidade, seguir o dono,
    /// ficar no ponto) cria a sua `session_npc_follow_target`, com agente novo (B136).
    sessao_de: Option<Meta>,
}

/// O que a sessão de movimento do mascote persegue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Meta {
    Alvo(i64),
    Dono,
    Ponto,
}

impl MascoteAi {
    pub const PASSO_MS: u32 = 500;

    /// `SetAggroState`/`SetStayState` da criação (`CreatePet`, `obj_interface.cpp:2841-2843`):
    /// só o passivo congela.
    pub fn new(agressividade: u8, movimento: u8, pos: Vector3) -> Self {
        Self {
            agressividade,
            movimento,
            congelado: agressividade == AGRESSIVIDADE_PASSIVA,
            odio: HashMap::new(),
            ponto_de_parada: pos,
            espera_ms: 0,
            recarga_ms: 0,
            batimento_ms: 0,
            parado: true,
            direcao: 0,
            seguir: None,
            no_espaco: None,
            falhas_de_caminho: 0,
            reposicao_pedida: false,
            seguindo: false,
            automatica: None,
            tarefa: None,
            conjuracao: None,
            recargas: HashMap::new(),
            raio_do_corpo: 0.0,
            altura_ao_seguir: 0.0,
            golpeando: false,
            sessao_de: None,
        }
    }

    /// A recarga pronta (`gpet_imp::CheckCoolDown`)?
    pub fn recarga_pronta(&self, recarga: i32) -> bool {
        self.recargas.get(&recarga).is_none_or(|ms| *ms == 0)
    }

    /// Está no meio de uma conjuração (`STATE_SESSION_USE_SKILL`)?
    pub fn conjurando(&self) -> bool {
        self.conjuracao.is_some()
    }

    /// Comando 4 (`petnpc.cpp:1065-1112`): `ClearNextTask` + `AddPetSkillTask`. Quem chama já
    /// conferiu o nível, o alvo e, para quem não é bênção, pôs o alvo no ódio com
    /// `max_hp + 10` (o mesmo de [`Self::atacar_por_ordem`]).
    pub fn ordenar_habilidade(&mut self, habilidade: HabilidadeDoMascote, alvo: Option<i64>) {
        self.tarefa = Some(TarefaDeHabilidade { habilidade, alvo, rastros: 2 });
    }

    /// Comando 5 (`petnpc.cpp:1115-1131`): `None` desliga.
    pub fn habilidade_automatica(&mut self, habilidade: Option<HabilidadeDoMascote>) {
        self.automatica = habilidade;
    }

    pub fn alvo(&self) -> Option<i64> {
        self.odio.iter().max_by_key(|(_, v)| **v).map(|(k, _)| *k)
    }

    pub fn em_combate(&self) -> bool {
        !self.odio.is_empty()
    }

    /// Ódio por apanhar (`HandleAttackMsg`), que o congelado recusa.
    pub fn apanhou(&mut self, de: i64, quanto: i64) {
        if !self.congelado {
            *self.odio.entry(de).or_insert(0) += quanto.max(1);
        }
    }

    /// Comando 1 do dono (`DispatchPlayerCommand`, `petnpc.cpp:956-975`): limpa o ódio e põe
    /// o alvo com `max_hp + 10`, mesmo congelado.
    pub fn atacar_por_ordem(&mut self, alvo: i64, max_hp: i64) {
        self.odio.clear();
        self.odio.insert(alvo, max_hp + 10);
        self.seguir = None;
    }

    /// Comando 2: seguir ou ficar (`:976-1011`). Limpa o ódio; ficar guarda o ponto
    /// (`ChangeStayMode`). Devolve se mudou (e então o `PET_AI_STATE` vai ao dono).
    pub fn mudar_movimento(&mut self, estado: u8, pos: Vector3) -> Option<bool> {
        if estado != MOVIMENTO_SEGUIR && estado != MOVIMENTO_FICAR {
            return None;
        }
        self.odio.clear();
        self.seguir = None;
        let mudou = self.movimento != estado;
        self.movimento = estado;
        if estado == MOVIMENTO_FICAR {
            self.ponto_de_parada = pos;
        }
        Some(mudou)
    }

    /// Comando 3: agressividade (`:1012-1045`). Defesa volta ao normal (limpando o ódio só
    /// se mudou); automático e passivo congelam e limpam.
    pub fn mudar_agressividade(&mut self, estado: u8) -> Option<bool> {
        match estado {
            AGRESSIVIDADE_DEFESA => {
                if self.agressividade != estado {
                    self.odio.clear();
                }
                self.congelado = false;
            }
            AGRESSIVIDADE_AUTOMATICA | AGRESSIVIDADE_PASSIVA => {
                self.odio.clear();
                self.congelado = true;
            }
            _ => return None,
        }
        let mudou = self.agressividade != estado;
        self.agressividade = estado;
        Some(mudou)
    }

    /// `GM_MSG_MASTER_ASK_HELP`: só na defesa, ódio 2 em quem bateu no dono.
    pub fn dono_apanhou(&mut self, de: i64) {
        if self.agressividade == AGRESSIVIDADE_DEFESA {
            *self.odio.entry(de).or_insert(0) += 2;
        }
    }

    /// `GM_MSG_PET_AUTO_ATTACK`: só no automático e sem ódio nenhum.
    pub fn dono_atacou(&mut self, alvo: i64, max_hp: i64) {
        if self.agressividade == AGRESSIVIDADE_AUTOMATICA && self.odio.is_empty() {
            self.odio.insert(alvo, max_hp + 10);
        }
    }

    /// Um tique de 50 ms. `dono`: posição do dono, se ele está vivo e no mundo.
    /// `alvos`: posição e vida dos monstros que o mascote pode atacar.
    pub fn tick(
        &mut self,
        corpo: &mut MonsterEntity,
        dono: Option<Vector3>,
        alvos: &HashMap<i64, (Vector3, bool, MonsterEntity)>,
        delta_ms: u32,
        mapa: &Mapa,
    ) -> Option<AcaoDoMascote> {
        if corpo.is_dead {
            return None;
        }
        self.espera_ms = self.espera_ms.saturating_sub(delta_ms);
        self.recarga_ms = self.recarga_ms.saturating_sub(delta_ms);
        for ms in self.recargas.values_mut() {
            *ms = ms.saturating_sub(delta_ms);
        }
        self.recargas.retain(|_, ms| *ms > 0);
        if std::mem::take(&mut self.reposicao_pedida) && dono.is_some() {
            return Some(AcaoDoMascote::Reposicionar);
        }
        let Some(dono) = dono else {
            // `QueryTarget(leader) != TARGET_STATE_NORMAL` → `GM_MSG_PET_DISAPPEAR`.
            return Some(AcaoDoMascote::Sumir);
        };

        // O que o ódio aponta e ainda existe, vivo.
        self.odio.retain(|id, _| alvos.get(id).is_some_and(|a| a.1));

        // As cercas do `gpet_policy::OnHeartbeat` (`petnpc.cpp:1646-1735`); `range` é a
        // distância horizontal **ao quadrado**.
        self.batimento_ms += delta_ms;
        if self.batimento_ms >= 1000 {
            self.batimento_ms -= 1000;
            let d2 = (corpo.position.x - dono.x).powi(2) + (corpo.position.z - dono.z).powi(2);
            let h = (corpo.position.y - dono.y).abs();
            if self.em_combate() {
                if h > 60.0 || d2 >= 3600.0 || (d2 < 36.0 && h > 40.0) {
                    return Some(AcaoDoMascote::Reposicionar);
                }
            } else if self.movimento != MOVIMENTO_SEGUIR {
                if h > 60.0 || d2 >= 3600.0 || (d2 < 36.0 && h > 30.0) {
                    return Some(AcaoDoMascote::Sumir);
                }
            } else {
                if d2 > 150.0 * 150.0 || h > 60.0 || d2 >= 3600.0 || (d2 < 36.0 && h > 30.0) {
                    return Some(AcaoDoMascote::Reposicionar);
                }
                if self.falhas_de_caminho >= 5 {
                    self.falhas_de_caminho = 0;
                    return Some(AcaoDoMascote::Reposicionar);
                }
                // `range > 1.5f*1.5f || h > 10.f` → `AddTargetTask<ai_pet_follow_master>`, só
                // aqui no batimento — não a cada tique.
                if d2 > 1.5 * 1.5 || h > 10.0 {
                    self.seguindo = true;
                }
            }
        }

        // Atordoado ou adormecido (`IncIdleSealMode(MODE_INDEX_STUN/SLEEP)` → `gnpc_imp::
        // SetIdleMode`, `npc.cpp:2129-2138`): a sessão em curso acaba (`ClearSession` — o canto
        // também), a IA limpa a tarefa (`ai_policy::SetIdleMode`, `aipolicy.h:1075-1080`) e não
        // decide nada até o filtro sair (`if(_idle_mode) return`, `aipolicy.cpp:281`); as cercas
        // de distância do dono acima continuam (`gpet_policy::OnHeartbeat` roda depois). B140.
        if corpo.efeitos.sem_acao() {
            self.conjuracao = None;
            self.tarefa = None;
            self.seguir = None;
            self.no_espaco = None;
            self.seguindo = false;
            self.golpeando = false;
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        // Selado (`SetSealMode(silent)`): não começa habilidade (`session_npc_skill` recusa).
        if corpo.efeitos.selado() {
            self.tarefa = None;
        }
        // A sessão de habilidade em curso ocupa o mascote até o fim da execução.
        if let Some(c) = self.conjuracao {
            return self.passar_conjuracao(c, delta_ms);
        }
        // `gpet_policy::OnHeartbeat` e `DeterminePolicy` (`petnpc.cpp:1487-1519`, `:1630-1645`):
        // em combate, com a automática fora da recarga e mana para ela, a próxima tarefa é a
        // habilidade no primeiro do ódio, em vez do golpe comum.
        if self.tarefa.is_none() && !corpo.efeitos.selado() {
            if let (Some(h), Some(alvo)) = (self.automatica, self.alvo()) {
                if self.recarga_pronta(h.recarga()) && corpo.mp >= h.mana {
                    self.tarefa = Some(TarefaDeHabilidade { habilidade: h, alvo: Some(alvo), rastros: 2 });
                }
            }
        }
        if let Some(t) = self.tarefa {
            return self.executar_tarefa(t, corpo, alvos, mapa);
        }

        // Com alvo: a tarefa corpo a corpo — chegar ao alcance e bater no ritmo do
        // `attack_speed` (`ai_melee_task` → `session_npc_attack`).
        if let Some(alvo_id) = self.alvo() {
            // `ai_pet_follow_master::OnHeartbeat`: em combate a tarefa de seguir acaba.
            self.seguindo = false;
            let (pos_alvo, _, alvo_corpo) = &alvos[&alvo_id];
            // `ai_melee_task`: `info.pos.squared_distance(selfpos)`, em **3D**
            // (`aipolicy.cpp:579-591`) — o mascote de ar tem de descer até o alvo (B135).
            let d = corpo.position.distance(pos_alvo);
            // `ai_melee_task::Execute` (`aipolicy.cpp:597-609`): começa a bater a
            // `(attack_range − corpo) × 0,8 + corpo + corpo do alvo` e persegue até
            // `× 0,6` no lugar do 0,8; a `session_npc_attack` continua até `attack_range + corpo
            // do alvo` (`CheckAttack`). Antes o mascote media só o `attack_range` e ia até 0,9
            // dele, sem o corpo do alvo — o de ar descia mais do que o original (B136).
            let corpo_alvo = alvo_corpo.tamanho.max(0.0);
            let puro = corpo.attack_range - self.raio_do_corpo;
            let alcance = if self.golpeando {
                corpo.attack_range + corpo_alvo
            } else {
                puro * 0.8 + self.raio_do_corpo + corpo_alvo
            };
            if d <= alcance {
                self.golpeando = true;
                self.seguir = None;
                if !self.parado {
                    return Some(AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
                }
                if self.recarga_ms == 0 {
                    self.recarga_ms = (corpo.ataque_em_ticks.max(4) as u32) * 50;
                    let mut golpe = crate::combat::CombatEngine::golpe_de_monstro(corpo);
                    golpe.atacante_e_jogador_ou_pet = true;
                    let dano = crate::combat::resolver(
                        &golpe,
                        &crate::combat::CombatEngine::defesa_do_monstro(alvo_corpo),
                        d,
                        false,
                        crate::combat::Rolagens::sortear(),
                    )
                    .dano();
                    return Some(AcaoDoMascote::Atacou { alvo: alvo_id, dano });
                }
                return None;
            }
            self.golpeando = false;
            self.sessao(Meta::Alvo(alvo_id));
            let meta = (puro * 0.6 + self.raio_do_corpo + corpo_alvo).max(0.5);
            return self.mover_ate(corpo, *pos_alvo, meta, mapa);
        }
        self.golpeando = false;

        // Sem alvo: ficar parado no ponto, ou seguir o dono quando passa de 1,5 m ou 10 m de
        // altura (`range > 1.5² || h > 10`), parando a 0,8 m dele (`SetTarget(_target, 0.8f,
        // ...)`, `aipolicy.cpp:1828-1838`).
        if self.movimento == MOVIMENTO_FICAR {
            let p = self.ponto_de_parada;
            if distancia_h(corpo.position, p) > 0.5 {
                self.sessao(Meta::Ponto);
                return self.mover_ate(corpo, p, 0.3, mapa);
            }
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        // `session_npc_follow_target::SetTarget(dono, 0.8, 62, 1.0)` (`aipolicy.cpp:1828-1838`):
        // a sessão acaba com parada quando a distância (3D, `squared_distance`) fica abaixo de
        // 0,8 m; até lá o agente persegue com meta de 1,0 m. Sem sessão, o mascote fica parado.
        if !self.seguindo {
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        let d3 = ((corpo.position.x - dono.x).powi(2) + (corpo.position.y - dono.y).powi(2) + (corpo.position.z - dono.z).powi(2)).sqrt();
        if d3 < 0.8 {
            self.seguindo = false;
            self.seguir = None;
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        // `ai_pet_follow_master::Execute` (`aipolicy.cpp:1827-1835`): mascote que não é só de chão
        // (`GetInhabitType() != 0`) mira 1,5 m acima do dono (`info.pos.y += _height_offset`,
        // `npcsession.cpp:197`). Sem isso o de ar descia até os pés do dono e ficava "enterrado"
        // (B135). O teste de 0,8 m acima continua contra o dono em si, como no original.
        let meta = Vector3::new(dono.x, dono.y + self.altura_ao_seguir, dono.z);
        self.sessao(Meta::Dono);
        self.mover_ate(corpo, meta, 1.0, mapa)
    }

    /// Troca de tarefa, sessão nova: o agente da perseguição anterior não serve para a
    /// seguinte. Sem isso, depois de matar, com o dono a menos de 4 m na horizontal do morto (o
    /// dono voando por cima), o mascote seguia o agente velho até os pés do morto (B136).
    fn sessao(&mut self, de: Meta) {
        if self.sessao_de != Some(de) {
            self.sessao_de = Some(de);
            self.seguir = None;
            self.no_espaco = None;
        }
    }

    /// `ai_skill_task_2::Execute` (`aipolicy.cpp:1963-2050`): em si, conjura já; com alvo,
    /// persegue até `0,9 ×` o alcance (`range > sa × 0,81`) e conjura, ou conjura de onde está
    /// depois de duas perseguições. Alvo que sumiu sai do ódio e a tarefa acaba.
    fn executar_tarefa(
        &mut self,
        t: TarefaDeHabilidade,
        corpo: &mut MonsterEntity,
        alvos: &HashMap<i64, (Vector3, bool, MonsterEntity)>,
        mapa: &Mapa,
    ) -> Option<AcaoDoMascote> {
        if area_sem_alvo(t.habilidade.area) {
            return self.comecar_a_conjurar(t.habilidade, t.alvo, corpo);
        }
        let Some(alvo) = t.alvo else {
            self.tarefa = None;
            return None;
        };
        let Some((pos, true, _)) = alvos.get(&alvo) else {
            self.odio.remove(&alvo);
            self.tarefa = None;
            return None;
        };
        // O raio do alvo não entra: o `MonsterEntity` não o guarda (o golpe comum também
        // mede só até o centro).
        let alcance = t.habilidade.alcance + self.raio_do_corpo;
        // `ai_skill_task_2::Execute`: `squared_distance`, em 3D (`aipolicy.cpp:1999-2027`, B135).
        if t.rastros == 0 || corpo.position.distance(pos) <= alcance * 0.9 {
            return self.comecar_a_conjurar(t.habilidade, t.alvo, corpo);
        }
        self.sessao(Meta::Alvo(alvo));
        if self.seguir.is_none() && self.no_espaco.is_none() {
            // Uma perseguição nova (`_trace_count --`).
            if let Some(t) = self.tarefa.as_mut() {
                t.rastros -= 1;
            }
        }
        self.mover_ate(corpo, *pos, (alcance * 0.9).max(0.5), mapa)
    }

    /// `session_npc_skill::StartSession` (`npcsession.cpp:654-730`): a recarga armada recusa
    /// (e o dono ouve o erro 93), a mana que falta recusa em silêncio; senão o canto começa.
    fn comecar_a_conjurar(&mut self, h: HabilidadeDoMascote, alvo: Option<i64>, corpo: &MonsterEntity) -> Option<AcaoDoMascote> {
        self.tarefa = None;
        if !self.recarga_pronta(h.recarga()) {
            return Some(AcaoDoMascote::HabilidadeEmRecarga);
        }
        if h.mana > 0 && corpo.mp < h.mana {
            return None;
        }
        self.seguir = None;
        self.conjuracao = Some(Conjuracao::Canto { habilidade: h, alvo, falta_ms: h.canto_ms });
        Some(AcaoDoMascote::Conjurou { habilidade: h, alvo })
    }

    /// O relógio da sessão: o canto acaba no efeito (e arma a recarga); a execução acaba
    /// devolvendo o mascote à IA.
    fn passar_conjuracao(&mut self, c: Conjuracao, delta_ms: u32) -> Option<AcaoDoMascote> {
        match c {
            Conjuracao::Canto { habilidade, alvo, falta_ms } => {
                let falta_ms = falta_ms.saturating_sub(delta_ms);
                if falta_ms > 0 {
                    self.conjuracao = Some(Conjuracao::Canto { habilidade, alvo, falta_ms });
                    return None;
                }
                if habilidade.recarga_ms > 0 {
                    self.recargas.insert(habilidade.recarga(), habilidade.recarga_ms as u32);
                }
                // Canto 0 é `interval == 0`: o efeito sai na hora e a sessão não continua.
                self.conjuracao = (habilidade.canto_ms > 0).then(|| Conjuracao::Execucao {
                    falta_ms: if habilidade.execucao_ms > 0 { habilidade.execucao_ms } else { 20 * 50 },
                });
                Some(AcaoDoMascote::UsouHabilidade { habilidade, alvo })
            }
            Conjuracao::Execucao { falta_ms } => {
                let falta_ms = falta_ms.saturating_sub(delta_ms);
                self.conjuracao = (falta_ms > 0).then_some(Conjuracao::Execucao { falta_ms });
                None
            }
        }
    }

    /// `session_npc_follow_target::Run` (`npcsession.cpp:164-280`): um passo de 500 ms
    /// correndo rumo a `meta`, desviando pelo mapa de movimento.
    ///
    /// O que importa para o cliente é **quando sai a parada**: ao chegar à meta antiga
    /// (`GetToGoal`) o agente recomeça rumo à posição nova com `0,6 × alcance` e segue andando;
    /// a parada (`TrySendStop`, uma vez) só sai quando o recomeço já nasce na meta ou o passo não
    /// sai do lugar. O cliente só reinicia a animação de andar — e o som dela — quando o NPC sai
    /// de `WORK_MOVE` (`CECNPC::MoveTo`, `EC_NPC.cpp:1048-1053`); parar a cada passo que
    /// alcançava a meta fazia o som do mascote recomeçar enquanto ele seguia o dono (B114).
    fn mover_ate(&mut self, corpo: &mut MonsterEntity, meta: Vector3, alcance: f32, mapa: &Mapa) -> Option<AcaoDoMascote> {
        if self.espera_ms > 0 {
            return None;
        }
        // Preso (`SetSealMode(root)`): `if(_imp->IsRootMode()) return 0` no `follow_target`
        // (`npcsession.cpp:190`) — não anda, mas bate no que estiver ao alcance.
        if corpo.efeitos.preso() {
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        self.espera_ms = Self::PASSO_MS;
        let passo = corpo.move_speed * Self::PASSO_MS as f32 / 1000.0;
        if corpo.habitat != Habitat::Chao {
            return self.mover_no_espaco(corpo, meta, alcance, passo, mapa);
        }
        let de = V3::new(corpo.position.x, corpo.position.y, corpo.position.z);
        let alvo = V3::new(meta.x, meta.y, meta.z);
        let (recomecar, alcance_da_vez) = match &self.seguir {
            None => (true, alcance),
            Some(s) if s.chegou() => (true, alcance * 0.6),
            Some(s) => {
                // O alvo andou mais de 7 m, ou mais de 4 m sem bloqueio: meta nova.
                let a = s.alvo();
                let dis = (a.x - alvo.x).powi(2) + (a.z - alvo.z).powi(2);
                (dis > 49.0 || (dis > 16.0 && !s.bloqueado()), alcance)
            }
        };
        let mut s = self.seguir.take().unwrap_or_default();
        if recomecar {
            let d2 = (de.x - alvo.x).powi(2) + (de.y - alvo.y).powi(2) + (de.z - alvo.z).powi(2);
            s.comecar(de, alvo, passo, alcance_da_vez, d2, None, mapa);
            if s.chegou() {
                self.seguir = Some(s);
                return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
            }
        }
        if !s.andar(passo, mapa) {
            // `NSRC_ERR_PATHFINDING`: a sessão acaba com parada e conta uma falha
            // (`FollowMasterResult(1)`; 5 seguidas reposicionam).
            self.falhas_de_caminho += 1;
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        self.falhas_de_caminho = 0;
        let p = s.posicao();
        let (dx, dz) = (p.x - corpo.position.x, p.z - corpo.position.z);
        let dy = p.y - corpo.position.y;
        if dx * dx + dy * dy + dz * dz < 1e-3 {
            self.seguir = Some(s);
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        let m = (dx * dx + dz * dz).sqrt();
        if m > 0.0 {
            self.direcao = crate::ai::direcao_do_vetor(dx / m, dz / m);
        }
        corpo.position = Vector3::new(p.x, p.y, p.z);
        self.seguir = Some(s);
        self.parado = false;
        Some(AcaoDoMascote::Moveu(AcaoDoMonstro::Andou {
            destino: corpo.position,
            tempo_ms: Self::PASSO_MS as u16,
            velocidade: corpo.move_speed,
            modo: MODO_CORRER | corpo.habitat.mascara_de_movimento(),
        }))
    }

    /// O mascote de ar ou de água: `follow_target` com o `CNPCChaseOnAirPFAgent` /
    /// `CNPCChaseInWaterPFAgent` (`NPCMoveAgent.cpp:68-90`) — reta quando o espaço deixa, senão a
    /// busca na octree do `airmap/` ([`crate::navegacao::SeguirNoEspaco`], B133). As regras de
    /// recomeço são as do [`Self::mover_ate`] de chão.
    fn mover_no_espaco(&mut self, corpo: &mut MonsterEntity, meta: Vector3, alcance: f32, passo: f32, mapa: &Mapa) -> Option<AcaoDoMascote> {
        use crate::navegacao::Ambiente;
        self.seguir = None;
        let amb = if corpo.habitat == Habitat::Agua { Ambiente::Agua } else { Ambiente::Ar };
        let de = V3::new(corpo.position.x, corpo.position.y, corpo.position.z);
        let alvo = V3::new(meta.x, meta.y, meta.z);
        let (recomecar, alcance_da_vez) = match &self.no_espaco {
            None => (true, alcance),
            Some(s) if s.chegou() => (true, alcance * 0.6),
            Some(s) => {
                let a = s.alvo();
                let dis = (a.x - alvo.x).powi(2) + (a.z - alvo.z).powi(2);
                (dis > 49.0 || (dis > 16.0 && !s.bloqueado()), alcance)
            }
        };
        let mut s = self.no_espaco.take().unwrap_or_default();
        s.sobe_o_chao = true;
        if recomecar {
            let d2 = (de.x - alvo.x).powi(2) + (de.y - alvo.y).powi(2) + (de.z - alvo.z).powi(2);
            s.comecar(amb, de, alvo, passo, alcance_da_vez, d2, mapa);
            if s.chegou() {
                self.no_espaco = Some(s);
                return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
            }
        }
        if !s.andar(passo, mapa) {
            self.falhas_de_caminho += 1;
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        self.falhas_de_caminho = 0;
        let p = s.posicao();
        let (dx, dy, dz) = (p.x - corpo.position.x, p.y - corpo.position.y, p.z - corpo.position.z);
        self.no_espaco = Some(s);
        if dx * dx + dy * dy + dz * dz < 1e-3 {
            return (!self.parado).then(|| AcaoDoMascote::Moveu(self.parar(corpo, MODO_CORRER)));
        }
        let m = (dx * dx + dz * dz).sqrt();
        if m > 0.0 {
            self.direcao = crate::ai::direcao_do_vetor(dx / m, dz / m);
        }
        corpo.position = Vector3::new(p.x, p.y, p.z);
        self.parado = false;
        Some(AcaoDoMascote::Moveu(AcaoDoMonstro::Andou {
            destino: corpo.position,
            tempo_ms: Self::PASSO_MS as u16,
            velocidade: corpo.move_speed,
            modo: MODO_CORRER | corpo.habitat.mascara_de_movimento(),
        }))
    }

    fn parar(&mut self, corpo: &MonsterEntity, modo: u8) -> AcaoDoMonstro {
        self.parado = true;
        AcaoDoMonstro::Parou {
            posicao: corpo.position,
            velocidade: corpo.move_speed,
            direcao: self.direcao,
            modo: modo | corpo.habitat.mascara_de_movimento(),
        }
    }

    /// Depois de reposicionado: sem perseguição em curso, parado no ponto novo.
    /// `gpet_imp::PetRelocatePos(false)` (`petnpc.cpp:302-310`): o próximo tique reposiciona.
    pub fn pedir_reposicao(&mut self) {
        self.reposicao_pedida = true;
    }

    pub fn reposicionado(&mut self) {
        self.seguir = None;
        self.no_espaco = None;
        self.parado = true;
        self.falhas_de_caminho = 0;
    }
}

/// A parada do `GM_MSG_PET_CHANGE_POS`: `stop_move(pos, 0x500, 1, MOVE_MODE_RETURN)`
/// (`petnpc.cpp:430-441`), que o cliente aplica de uma vez.
pub fn parada_de_reposicao(pos: Vector3, habitat: crate::ai::Habitat) -> AcaoDoMonstro {
    AcaoDoMonstro::Parou { posicao: pos, velocidade: 0x500 as f32 / 256.0, direcao: 1, modo: MODO_VOLTAR | habitat.mascara_de_movimento() }
}

fn distancia_h(a: Vector3, b: Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// `combat_petdata_imp::OnKillMob` (`petman.cpp:787-800`): 10 de base, menos a diferença
/// quando o monstro é de nível menor que o mascote, vezes o ajuste da lealdade.
pub fn exp_por_abate(nivel_do_mascote: i32, nivel_do_monstro: i32, lealdade: i32) -> i32 {
    let mut base = 10;
    if nivel_do_monstro < nivel_do_mascote {
        base -= nivel_do_mascote - nivel_do_monstro;
        if base <= 0 {
            return 0;
        }
    }
    (base as f32 * ajuste_de_exp(nivel_de_lealdade(lealdade)) + 0.1) as i32
}

/// O resultado de `pet_manager::RecvExp` (`petman.cpp:1541-1591`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpDoMascote {
    Nada,
    /// Ganhou `exp` sem subir (`pet_recv_exp`).
    Ganhou(i32),
    /// Subiu (`pet_level_up` com o nível e a experiência nova).
    Subiu,
}

/// `RecvExp`: soma, sobe enquanto dá, trava no nível histórico do dono (fica com a
/// experiência cheia) e no teto do modelo (zera). `para_subir(n)` é a curva
/// (`GetPetLvlupExp`).
pub fn receber_exp(info: &mut InfoPet, exp: i32, nivel_maximo: i32, nivel_do_dono: i32, para_subir: impl Fn(i32) -> i64) -> ExpDoMascote {
    if info.level as i32 >= nivel_maximo {
        return ExpDoMascote::Nada;
    }
    let mut atual = info.exp as i64 + exp as i64;
    let mut subiu = false;
    loop {
        let precisa = para_subir(info.level as i32);
        if atual < precisa {
            break;
        }
        if info.level as i32 >= nivel_do_dono {
            atual = precisa;
            break;
        }
        subiu = true;
        atual -= precisa;
        info.level += 1;
        if info.level as i32 >= nivel_maximo {
            atual = 0;
            break;
        }
    }
    let ganho = atual - info.exp as i64;
    info.exp = atual.clamp(0, i32::MAX as i64) as i32;
    if subiu {
        ExpDoMascote::Subiu
    } else if ganho != 0 {
        ExpDoMascote::Ganhou(ganho as i32)
    } else {
        ExpDoMascote::Nada
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_id_do_mascote_tem_o_bit_de_mascote() {
        let id = id_do_mascote(7);
        assert_eq!(id as i32 as u32, 0xA000_0007);
        assert!(e_mascote(id));
        assert!(!e_mascote((0x8000_1B11u32) as i32 as i64));
    }

    #[test]
    fn lealdade_e_ajustes() {
        assert_eq!([0, 50, 51, 150, 151, 500, 501, 999].map(nivel_de_lealdade), [0, 0, 1, 1, 2, 2, 3, 3]);
        assert_eq!(exp_por_abate(5, 5, 200), 10);
        assert_eq!(exp_por_abate(5, 2, 200), 7);
        assert_eq!(exp_por_abate(20, 5, 200), 0);
        assert_eq!(exp_por_abate(5, 5, 10), 1);
    }

    fn modelo() -> ModeloDeMascote {
        ModeloDeMascote {
            tid: 10386,
            classe: 1,
            hp: [27.5, 0.1, 2.0],
            hp_gen: [0.72, 0.1, 2.0],
            dano: [1.625, 0.108, 3.3888, 5.74992],
            velocidade: [6.71, 0.01],
            ataque: [21.6667, 0.1, 2.0],
            esquiva: [13.3333, 0.1, 2.0],
            defesa: [1.66667, 40.0, 0.1, -15.0],
            resistencia: [1.83333, 40.0, 0.1, -15.0],
            corpo: 0.9,
            alcance: 3.0,
            atraso_do_dano: 23,
            intervalo_do_golpe: 25,
            visao: 6.0,
            comida: 26,
            habitat: 0,
            imunidade: 0,
            nivel_maximo: 150,
            nivel_exigido: 2,
        }
    }

    #[test]
    fn a_fome_sobe_a_cada_300_s_e_a_comida_a_baixa() {
        let m0 = modelo();
        let info = InfoPet { pet_tid: 10386, level: 5, hunger: 1, honor_point: 200, hp_factor: 1.0, ..Default::default() };
        let mut m = Mascote::novo(id_do_mascote(1), 7, 0, info, &m0, Vector3::new(0.0, 0.0, 0.0), 1, 0);
        for _ in 0..299 {
            assert_eq!(m.passar_tempo_de_comida(&m0), None);
        }
        // Fome 1: +1 de fome, −1 de lealdade (`__pet_feed_param_list[1]`).
        assert_eq!(m.passar_tempo_de_comida(&m0), Some((199, 2)));
        // Comida do tipo 2 (26 & 2 ≠ 0), 20 de honra: fome 2 → fator 0,6, tira 1.
        assert_eq!(m.alimentar(&m0, 2, 20), Ok((211, 1)));
        // Tipo 1 não está no `food_mask` 26.
        assert_eq!(m.alimentar(&m0, 1, 20), Err(ERRO_COMIDA_ERRADA));
    }

    #[test]
    fn a_lealdade_muda_o_dano() {
        let m0 = modelo();
        let baixa = InfoPet { pet_tid: 10386, level: 5, honor_point: 10, hp_factor: 1.0, ..Default::default() };
        let alta = InfoPet { honor_point: 600, ..baixa.clone() };
        let d_baixa = corpo_do_mascote(1, &m0, &baixa, Vector3::new(0.0, 0.0, 0.0)).attack_min;
        let d_alta = corpo_do_mascote(1, &m0, &alta, Vector3::new(0.0, 0.0, 0.0)).attack_min;
        let base = m0.dano(5);
        assert_eq!((d_baixa, d_alta), (base * 60 / 100, base * 120 / 100));
    }

    #[test]
    fn a_experiencia_sobe_de_nivel_e_trava_no_dono() {
        let curva = |n: i32| (n * n * 500) as i64;
        let mut info = InfoPet { level: 1, exp: 0, ..Default::default() };
        assert_eq!(receber_exp(&mut info, 499, 150, 10, curva), ExpDoMascote::Ganhou(499));
        assert_eq!(receber_exp(&mut info, 1, 150, 10, curva), ExpDoMascote::Subiu);
        assert_eq!((info.level, info.exp), (2, 0));
        // No nível do dono: a experiência enche e para.
        let mut info = InfoPet { level: 3, exp: 0, ..Default::default() };
        assert_eq!(receber_exp(&mut info, 100_000, 150, 3, curva), ExpDoMascote::Ganhou(4500));
        assert_eq!((info.level, info.exp), (3, 4500));
    }

    /// B131 — `FindValidPos` (`petman.cpp:127-286`): o de chão não aparece com o dono no ar; o
    /// de ar aparece 1 m acima do dono; o de chão+ar vai ao ar quando o dono voa.
    #[test]
    fn o_lugar_do_mascote_segue_o_habitat_e_a_camada_do_dono() {
        use crate::combat::Camada;
        let (t, mv, ag) = (
            pw_data_loader::Terreno::vazio(),
            pw_data_loader::MapaDeMovimento::vazio(),
            pw_data_loader::MapaDeAgua::vazio(),
        );
        let dono = Vector3::new(0.0, 20.0, 0.0);
        let zero = Vector3::new(0.0, 0.0, 0.0);
        assert_eq!(posicao_para_mascote(&t, &mv, &ag, None, false, dono, Camada::Ar, 0, 0.0, zero), Err(-3));
        let (p, modo) = posicao_para_mascote(&t, &mv, &ag, None, false, dono, Camada::Ar, 2, 0.0, zero).unwrap();
        assert_eq!(modo, Habitat::Ar);
        assert!((p.y - 21.0).abs() < 1e-3, "{p:?}");
        let d = ((p.x - dono.x).powi(2) + (p.z - dono.z).powi(2)).sqrt();
        assert!(d > 1.0 && d < 1.8, "{d}");
        assert_eq!(posicao_para_mascote(&t, &mv, &ag, None, false, dono, Camada::Agua, 2, 0.0, zero), Err(-3));
        assert_eq!(posicao_para_mascote(&t, &mv, &ag, None, false, dono, Camada::Ar, 4, 0.0, zero).unwrap().1, Habitat::Ar);
        assert!(tenta_trocar_de_modo(4, Camada::Ar, Habitat::Chao));
        assert!(!tenta_trocar_de_modo(2, Camada::Chao, Habitat::Ar), "o de ar só fica com o dono");
    }

    /// B131 — o mascote de ar segue o dono no ar em linha reta (`CNPCChaseOnAirStraightAgent`):
    /// o Vespão Pequeno (`inhabit_type` 2) sobe até o dono a 30 m e manda o passo com a marca de
    /// céu (0x40).
    #[test]
    fn o_mascote_de_ar_sobe_atras_do_dono() {
        let mut m0 = modelo();
        m0.habitat = 2;
        let info = InfoPet { pet_tid: 10386, level: 5, hp_factor: 1.0, ..Default::default() };
        let mut m = Mascote::novo(id_do_mascote(1), 7, 0, info, &m0, Vector3::new(0.0, 5.0, 0.0), 1, MOVIMENTO_SEGUIR);
        m.corpo.habitat = Habitat::Ar;
        let dono = Vector3::new(3.0, 30.0, 0.0);
        let chao = |_x: f32, _z: f32| Some(0.0f32);
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &chao, movimento: &mv, espaco: None, agua: None };
        let mut ceu = false;
        for _ in 0..(20_000 / 50) {
            if let Some(AcaoDoMascote::Moveu(AcaoDoMonstro::Andou { modo, .. })) =
                m.ai.tick(&mut m.corpo, Some(dono), &HashMap::new(), 50, &mapa)
            {
                ceu |= modo & 0x40 != 0;
            }
        }
        assert!(ceu, "o passo não levou a marca de céu");
        assert!((m.corpo.position.y - dono.y).abs() < 1.5, "ficou em {:?}", m.corpo.position);
    }

    /// B133 — mapa `nofly`: o mascote de ar não aparece (`petman.cpp:140`, -2) e o de chão+ar
    /// fica no chão mesmo com o dono voando (`:270`).
    #[test]
    fn no_mapa_sem_voo_o_mascote_de_ar_nao_aparece() {
        use crate::combat::Camada;
        let (t, mv, ag) = (
            pw_data_loader::Terreno::vazio(),
            pw_data_loader::MapaDeMovimento::vazio(),
            pw_data_loader::MapaDeAgua::vazio(),
        );
        let dono = Vector3::new(0.0, 20.0, 0.0);
        let zero = Vector3::new(0.0, 0.0, 0.0);
        assert_eq!(posicao_para_mascote(&t, &mv, &ag, None, true, dono, Camada::Chao, 2, 0.0, zero), Err(-2));
        let r = posicao_para_mascote(&t, &mv, &ag, None, true, dono, Camada::Ar, 4, 0.0, zero);
        assert_eq!(r.map(|x| x.1), Ok(Habitat::Chao));
    }

    fn vespao(pos: Vector3) -> Mascote {
        let mut m0 = modelo();
        m0.habitat = 2;
        let info = InfoPet { pet_tid: 10521, level: 30, hp_factor: 1.0, ..Default::default() };
        let mut m = Mascote::novo(id_do_mascote(1), 7, 0, info, &m0, pos, 1, MOVIMENTO_SEGUIR);
        m.corpo.habitat = Habitat::Ar;
        m
    }

    /// B135 — seguindo o dono no chão, o mascote de ar para 1,5 m acima dele
    /// (`ai_pet_follow_master::Execute`, `aipolicy.cpp:1827-1835`), não nos pés: não afunda.
    #[test]
    fn o_mascote_de_ar_segue_o_dono_no_chao_por_cima() {
        let mut m = vespao(Vector3::new(8.0, 12.0, 0.0));
        let dono = Vector3::new(0.0, 0.0, 0.0);
        let chao = |_x: f32, _z: f32| Some(0.0f32);
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &chao, movimento: &mv, espaco: None, agua: None };
        for _ in 0..(20_000 / 50) {
            m.ai.tick(&mut m.corpo, Some(dono), &HashMap::new(), 50, &mapa);
        }
        assert!(m.corpo.position.y > 0.4, "enterrado: {:?}", m.corpo.position);
        let h = ((m.corpo.position.x).powi(2) + (m.corpo.position.z).powi(2)).sqrt();
        assert!(h < 2.0, "não chegou perto: {:?}", m.corpo.position);
    }

    /// B135 — o alcance do golpe é em 3D (`aipolicy.cpp:579`): 10 m acima do alvo o mascote não
    /// bate, desce.
    #[test]
    fn dez_metros_acima_do_alvo_o_mascote_de_ar_nao_bate() {
        let mut m = vespao(Vector3::new(0.0, 10.0, 0.0));
        let alvo = MonsterEntity::placeholder(900_001, 1001, Vector3::new(0.0, 0.0, 0.0), 0);
        let alvos = HashMap::from([(900_001i64, (alvo.position, true, alvo))]);
        m.ai.atacar_por_ordem(900_001, m.corpo.max_hp);
        let chao = |_x: f32, _z: f32| Some(0.0f32);
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &chao, movimento: &mv, espaco: None, agua: None };
        let primeira = m.ai.tick(&mut m.corpo, Some(Vector3::new(5.0, 0.0, 0.0)), &alvos, 50, &mapa);
        assert!(!matches!(primeira, Some(AcaoDoMascote::Atacou { .. })), "bateu de 10 m de altura");
    }

    /// Um morro suave: o terreno sobe 1 m até `x = 10` e desce. A reta entre dois pontos do
    /// morro passa **por dentro** dele — rente ao chão, o passo reto de ar entrava no terreno.
    fn morro(x: f32, _z: f32) -> Option<f32> {
        Some(1.0 - 0.01 * (x - 10.0).powi(2))
    }

    /// B136 — atacando um monstro que anda por cima do morro, o mascote de ar não afunda
    /// (`AdjustCurPos`: terreno + 0,2, `NPCChaseOnAirAgent.h:27-36`) e não trava: continua
    /// batendo enquanto o alvo anda.
    #[test]
    fn o_mascote_de_ar_persegue_o_monstro_que_anda_no_morro_sem_afundar() {
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &morro, movimento: &mv, espaco: None, agua: None };
        let mut m = vespao(Vector3::new(0.0, 2.0, 0.0));
        let mut alvo = MonsterEntity::placeholder(900_001, 1001, Vector3::new(3.0, morro(3.0, 0.0).unwrap(), 0.0), 0);
        m.ai.atacar_por_ordem(900_001, m.corpo.max_hp);
        let dono = Vector3::new(0.0, morro(0.0, 0.0).unwrap(), 2.0);
        let (mut golpes, mut golpes_depois_de_andar, mut abaixo) = (0, 0, 0);
        for t in 0..(40_000 / 50) {
            // O alvo anda 2 m a cada 2 s, morro acima e depois abaixo.
            if t % 40 == 0 && t > 0 && alvo.position.x < 18.0 {
                let x = alvo.position.x + 2.0;
                alvo.position = Vector3::new(x, morro(x, 0.0).unwrap(), 0.0);
            }
            let alvos = HashMap::from([(900_001i64, (alvo.position, true, alvo.clone()))]);
            if let Some(AcaoDoMascote::Atacou { .. }) = m.ai.tick(&mut m.corpo, Some(dono), &alvos, 50, &mapa) {
                golpes += 1;
                if alvo.position.x >= 17.0 {
                    golpes_depois_de_andar += 1;
                }
            }
            let p = m.corpo.position;
            if p.y < morro(p.x, p.z).unwrap() + 0.2 - 1e-3 {
                abaixo += 1;
            }
        }
        assert_eq!(abaixo, 0, "o mascote ficou abaixo de terreno + 0,2 em {abaixo} tiques");
        assert!(golpes > 5, "só {golpes} golpes");
        assert!(golpes_depois_de_andar > 0, "parou de bater depois que o alvo andou");
    }

    /// B136 — voando, o dono logo acima do monstro morto: o mascote volta para cima dele (a
    /// sessão nova de seguir o dono não herda o agente que ia aos pés do morto).
    #[test]
    fn depois_de_matar_o_mascote_de_ar_volta_ao_dono_que_voa() {
        let chao = |_x: f32, _z: f32| Some(0.0f32);
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &chao, movimento: &mv, espaco: None, agua: None };
        let mut m = vespao(Vector3::new(4.0, 6.0, 0.0));
        let alvo = MonsterEntity::placeholder(900_001, 1001, Vector3::new(0.0, 0.0, 0.0), 0);
        m.ai.atacar_por_ordem(900_001, m.corpo.max_hp);
        let dono = Vector3::new(1.0, 15.0, 1.0);
        let vivo = HashMap::from([(900_001i64, (alvo.position, true, alvo.clone()))]);
        for _ in 0..(3_000 / 50) {
            m.ai.tick(&mut m.corpo, Some(dono), &vivo, 50, &mapa);
        }
        let morto = HashMap::from([(900_001i64, (alvo.position, false, alvo.clone()))]);
        for _ in 0..(15_000 / 50) {
            m.ai.tick(&mut m.corpo, Some(dono), &morto, 50, &mapa);
        }
        let d = m.corpo.position.distance(&Vector3::new(dono.x, dono.y + 1.5, dono.z));
        assert!(d < 2.5, "não voltou ao dono: {:?} (a {d} m)", m.corpo.position);
    }

    fn filtro(efeito: crate::efeitos::Efeito, s: i32) -> crate::efeitos::Filtro {
        crate::efeitos::Filtro {
            efeito,
            restante_s: s,
            razao: 0,
            fator: 0.0,
            por_segundo: 0,
            contador: 0,
            origem: 0,
            icone: true,
            absorve: 0.0,
            escala_defesa: 0,
            fator_de_habilidade: 0.0,
        }
    }

    /// B140 — atordoado (a 37 do Guerreiro Golem, `Dizzy`), o mascote não anda nem bate até o
    /// filtro sair (`gnpc_imp::SetIdleMode` → `ClearSession`; `if(_idle_mode) return`).
    #[test]
    fn o_mascote_atordoado_nao_anda_nem_bate() {
        let chao = |_x: f32, _z: f32| Some(0.0f32);
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &chao, movimento: &mv, espaco: None, agua: None };
        let mut m = vespao(Vector3::new(0.0, 1.0, 0.0));
        let alvo = MonsterEntity::placeholder(900_001, 1001, Vector3::new(1.0, 0.0, 0.0), 0);
        let alvos = HashMap::from([(900_001i64, (alvo.position, true, alvo.clone()))]);
        m.ai.atacar_por_ordem(900_001, m.corpo.max_hp);
        m.corpo.efeitos.adicionar(filtro(crate::efeitos::Efeito::Dizzy, 3));
        let inicio = m.corpo.position;
        for _ in 0..(2_000 / 50) {
            let a = m.ai.tick(&mut m.corpo, Some(Vector3::new(-2.0, 0.0, 0.0)), &alvos, 50, &mapa);
            assert!(!matches!(a, Some(AcaoDoMascote::Atacou { .. })), "bateu atordoado");
        }
        assert_eq!(m.corpo.position, inicio, "andou atordoado");
        // O filtro sai: volta a bater.
        m.corpo.efeitos = Default::default();
        let mut bateu = false;
        for _ in 0..(3_000 / 50) {
            bateu |= matches!(m.ai.tick(&mut m.corpo, Some(Vector3::new(-2.0, 0.0, 0.0)), &alvos, 50, &mapa), Some(AcaoDoMascote::Atacou { .. }));
        }
        assert!(bateu, "não voltou a bater depois do atordoamento");
    }

    /// B140 — preso (`Fix`), não anda (`IsRootMode` no `follow_target`), mas bate no que está ao
    /// alcance.
    #[test]
    fn o_mascote_preso_nao_anda_mas_bate_ao_alcance() {
        let chao = |_x: f32, _z: f32| Some(0.0f32);
        let mv = pw_data_loader::MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &chao, movimento: &mv, espaco: None, agua: None };
        let mut m = vespao(Vector3::new(0.0, 1.0, 0.0));
        m.corpo.efeitos.adicionar(filtro(crate::efeitos::Efeito::Fix, 10));
        let longe = MonsterEntity::placeholder(900_001, 1001, Vector3::new(15.0, 0.0, 0.0), 0);
        let alvos = HashMap::from([(900_001i64, (longe.position, true, longe.clone()))]);
        m.ai.atacar_por_ordem(900_001, m.corpo.max_hp);
        let inicio = m.corpo.position;
        for _ in 0..(2_000 / 50) {
            m.ai.tick(&mut m.corpo, Some(Vector3::new(-2.0, 0.0, 0.0)), &alvos, 50, &mapa);
        }
        assert_eq!(m.corpo.position, inicio, "andou preso");
        let perto = MonsterEntity::placeholder(900_002, 1001, Vector3::new(1.5, 0.5, 0.0), 0);
        let alvos = HashMap::from([(900_002i64, (perto.position, true, perto.clone()))]);
        m.ai.atacar_por_ordem(900_002, m.corpo.max_hp);
        let mut bateu = false;
        for _ in 0..(2_000 / 50) {
            bateu |= matches!(m.ai.tick(&mut m.corpo, Some(Vector3::new(-2.0, 0.0, 0.0)), &alvos, 50, &mapa), Some(AcaoDoMascote::Atacou { .. }));
        }
        assert!(bateu, "preso, não bateu no que estava ao alcance");
    }
}
