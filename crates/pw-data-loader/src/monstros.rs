//! O template de monstro de verdade, montado a partir da tabela `MONSTER_ESSENCE` do
//! `elements.data`.
//!
//! # Origem
//!
//! Porte de `npcgenerator.cpp::npc_generator::Init` (`cgame/gs/`, fonte 1.5.5), o laço
//! que percorre `DT_MONSTER_ESSENCE` e monta um `npc_template` por monstro. Cada campo
//! aqui nomeia, no comentário, a linha do original que o preenche — inclusive as
//! conversões de unidade (segundos → *ticks* de 50 ms) e os valores que o original fixa em
//! vez de ler do arquivo.
//!
//! # Por que este módulo existe, e o que ele substitui
//!
//! [`crate::elements::MonsterTemplate`] (o leitor tipado antigo) declara `level`, `hp`,
//! `def_phys`, `exp`, `aggro_range`, `aipolicy_id`… e **preenche todos com constantes**:
//! só `id` e `name` saem do arquivo (`elements.rs`, no ramo `DT_MONSTER_ESSENCE`). O
//! `pw-gs` nunca chegou a consultá-lo — `world.rs::init_spawns` tinha os números escritos
//! no código (`hp: 500`, `attack_min: 20`, `level: 1`) para todo monstro do mundo.
//!
//! Este módulo lê os campos reais. Ele depende do leitor genérico
//! ([`crate::generic_elements`]), que é o único que cobre as 231 tabelas do 1.5.5 — o
//! leitor tipado antigo continua sendo o caminho do 1.2.6/v7, e para aquele realm este
//! módulo simplesmente não produz templates (ver [`carregar`]).
//!
//! # O que ainda não vem
//!
//! - **Drops.** `MONSTER_ESSENCE` não tem um "id de tabela de drop": tem 20 pares
//!   `drop_matters_N_id`/`_probability`, mais `probability_drop_num0..3` e `drop_times`.
//!   É um sistema, não um campo, e entra quando drop for a prioridade.
//! - **`MONSTER_ADDON`/`MONSTER_TYPE`.** O original sorteia um "addon" por monstro
//!   (`mt.addons[j].probability_addon`) antes de instanciar. Sem isso o monstro sai sem a
//!   variação de atributos, não sem atributos.
//! - **Ajuste por `role_in_war`**, que reescreve `faction` para cenários de cerco.

use crate::generic_elements::{GenericElementsData, Record};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Quantas classes de dano/resistência mágica existem — `MAGIC_CLASS` em
/// `cgame/gs/config.h`. A ordem é a de `attack.h::ATTACK_ATTR`: metal, madeira, água,
/// fogo, terra (`MAGIC_ATTACK_GOLD = 2` … `MAGIC_ATTACK_EARTH = 6`, deslocados de 2), e é
/// a mesma de `ep.resistance[0..4]` e de `magic_defences_1..5` no arquivo.
pub const CLASSES_MAGICAS: usize = 5;

/// Uma habilidade que o monstro sabe usar, com o nível em que a usa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDeMonstro {
    pub id: i32,
    pub nivel: i32,
}

/// Uma habilidade disparada por limiar de vida (`skill_hp75/50/25` no original), com a
/// probabilidade de sair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkillPorVida {
    pub id: i32,
    pub nivel: i32,
    pub probabilidade: f32,
}

/// Uma das quatro estratégias de ódio sorteadas por probabilidade
/// (`mob.aggro_strategy[j]`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EstrategiaDeOdio {
    pub id: i32,
    pub probabilidade: f32,
}

/// Faixa de dano `[minimo, maximo]` — o original guarda como `damage_low`/`damage_high`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaixaDeDano {
    pub minimo: i32,
    pub maximo: i32,
}

/// O template de um monstro, com os campos que o `gs` original usa para instanciá-lo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDeMonstro {
    // ---- identidade ----
    /// `mob.id` → `nt.tid`.
    pub id: u32,
    /// `mob.Name`. Vem em UTF-16 no arquivo.
    pub nome: String,

    // ---- atributos básicos (`nt.bp`) ----
    /// `mob.level`.
    pub nivel: i32,
    /// `mob.exp`.
    pub exp: i32,
    /// `mob.skillpoint` → `nt.bp.skill_point`.
    pub pontos_de_skill: i32,
    /// `mob.life` → `nt.bp.hp` **e** `nt.ep.max_hp`. O campo não se chama `hp` no arquivo.
    pub vida: i32,
    /// `mob.hp_regenerate` → `nt.ep.hp_gen`.
    pub regeneracao_de_vida: i32,

    // ---- defesa (`nt.ep`) ----
    /// `mob.defence` → `nt.ep.defense`. É o que entra em
    /// `player_template::GetDamageReduce(def, nível_do_atacante)`.
    pub defesa: i32,
    /// `mob.armor` → `nt.ep.armor`. É o que entra na chance de acerto:
    /// `attack_rate / (attack_rate + armor/2)`, com piso de 0,05
    /// (`actobject.cpp::AttackJudgement`).
    pub armadura: i32,
    /// `mob.magic_defences_1..5` → `nt.ep.resistance[0..4]`.
    pub resistencias: [i32; CLASSES_MAGICAS],
    /// `mob.immune_type` → `nt.immune_type`. Máscara: bit 0 = imune a físico, bits 1 a 5 =
    /// imune a cada classe mágica (`IMMUNE_MASK_PHYSIC`, `IMMUNE_GOLD = 0x02`, …).
    pub imunidades: i32,

    // ---- ataque (`nt.ep`) ----
    /// `mob.attack` → `nt.ep.attack`. É o `attack_rate` da fórmula de acerto, **não** o
    /// dano.
    pub taxa_de_ataque: i32,
    /// `mob.damage_min`/`damage_max` → `nt.ep.damage_low`/`damage_high`.
    pub dano_fisico: FaixaDeDano,
    /// `mob.magic_damage_min`/`max` → `nt.ep.damage_magic_low`/`high`.
    pub dano_magico: FaixaDeDano,
    /// `mob.magic_damages_ext[0..4]` → `nt.ep.addon_damage[0..4]`, na mesma ordem de
    /// [`Self::resistencias`].
    pub dano_magico_por_classe: [FaixaDeDano; CLASSES_MAGICAS],
    /// `mob.attack_range` → `nt.ep.attack_range`, em metros.
    pub alcance_de_ataque: f32,
    /// `(int)(mob.attack_speed * 20 + 0.5)` → `nt.ep.attack_speed`, em *ticks* de 50 ms.
    /// O original recusa o monstro se passar de 256.
    pub ataque_em_ticks: i32,
    /// `(int)(mob.damage_delay * 20)` → `nt.damage_delay`, em *ticks* de 50 ms: o atraso
    /// entre a animação e o dano. O original recusa o monstro se passar de 256.
    pub atraso_do_dano_em_ticks: i32,
    /// `mob.attack_degree` / `mob.defend_degree` → `nt.attack_degree`/`nt.defend_degree`.
    pub grau_de_ataque: i32,
    pub grau_de_defesa: i32,
    /// `nt.short_range_mode = 1` quando `mob.attack_range > 6.0` — **derivado**, não lido.
    /// O arquivo tem um campo `short_range_mode` próprio que o servidor ignora.
    pub ataque_a_distancia: bool,

    // ---- movimento (`nt.ep`) ----
    pub velocidade_andando: f32,
    pub velocidade_correndo: f32,
    pub velocidade_nadando: f32,
    pub velocidade_voando: f32,
    /// `mob.size` → `nt.body_size`.
    pub tamanho: f32,
    /// `mob.inhabit_type` → `nt.inhabit_type`, e daí sai o `inhabit_mode`
    /// (0/4/6 = chão, 1/3 = água, 2/5 = ar).
    pub tipo_de_habitat: i32,
    /// `mob.patroll_mode ? 1 : 0` → `nt.patrol_mode`.
    pub patrulha: bool,

    // ---- comportamento ----
    /// `mob.id_strategy` → `nt.id_strategy`. 2 e 3 são as estratégias que usam habilidade;
    /// o original avisa quando um monstro com essas não tem skill nenhuma configurada.
    pub estrategia: i32,
    /// `mob.common_strategy` → `nt.trigger_policy`: **o id da política em `aipolicy.data`**
    /// (ver [`crate::aipolicy`]). O original zera este campo, com aviso, quando a política
    /// não existe no arquivo — ver [`carregar`].
    pub politica_de_ia: u32,
    /// `mob.aggressive_mode` → `nt.aggressive_mode`: se ataca sozinho.
    pub agressivo: i32,
    /// `mob.aggro_range` → `nt.aggro_range`, em metros.
    pub raio_de_odio: f32,
    /// `(int)mob.aggro_time` → `nt.aggro_time`, em segundos, **com piso de 1** (o original
    /// força `if (nt.aggro_time <= 0) nt.aggro_time = 1`).
    pub tempo_de_odio: i32,
    /// `mob.sight_range` → `nt.sight_range`.
    pub raio_de_visao: i32,
    /// `mob.faction` / `mob.monster_faction` → `nt.faction` / `nt.monster_faction`.
    pub faccao: i32,
    pub faccao_de_monstro: i32,
    /// `mob.after_death` → `nt.after_death`.
    pub depois_da_morte: i32,

    // ---- habilidades ----
    /// `mob.skills[0..32]`, só as com `id_skill > 0`. O original as separa em ataque,
    /// bênção e maldição por `SkillWrapper::GetType`, que depende do `cskill/` — como esse
    /// sistema ainda não existe do nosso lado, aqui elas ficam numa lista só.
    pub skills: Vec<SkillDeMonstro>,
    /// `mob.skill_hp75` / `skill_hp50` / `skill_hp25` → `nt.skill_hp75/50/25`: até cinco
    /// habilidades por limiar de vida, cada uma com probabilidade.
    pub skills_com_75_de_vida: Vec<SkillPorVida>,
    pub skills_com_50_de_vida: Vec<SkillPorVida>,
    pub skills_com_25_de_vida: Vec<SkillPorVida>,
    /// `mob.aggro_strategy[0..4]`, só as com probabilidade não nula.
    pub estrategias_de_odio: Vec<EstrategiaDeOdio>,

    // ---- dinheiro ----
    /// `mob.money_average` / `mob.money_var`.
    pub dinheiro_medio: i32,
    pub dinheiro_variacao: i32,

    // ---- drop ----
    /// `probability_drop_num0..3` — chance de cair 0, 1, 2 ou 3 itens por rodada.
    pub chance_de_quantos: [f32; 4],
    /// `drop_times` — quantas rodadas de drop o monstro faz.
    pub rodadas_de_drop: i32,
    /// `drop_matters[32]` — `(id, probabilidade)`, na ordem do arquivo (a posição importa:
    /// a partir da 2ª rodada só os 16 primeiros valem, `itemdataman.cpp:1210`).
    pub itens_de_drop: Vec<(u32, f32)>,
}

/// Por que um monstro do arquivo não virou template.
///
/// São exatamente as três recusas do original (`npcgenerator.cpp`), que fazem `continue` e
/// deixam o monstro de fora da tabela — um `npcgen.data` que o cite depois não acha nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Recusas {
    /// `attack_speed <= 0 || damage_low <= 0 || attack <= 0` — "modelo de ataque errado".
    pub modelo_de_ataque_invalido: usize,
    /// `attack_speed > 256`.
    pub ataque_lento_demais: usize,
    /// `damage_delay > 256`.
    pub atraso_de_dano_grande_demais: usize,
}

impl Recusas {
    pub fn total(&self) -> usize {
        self.modelo_de_ataque_invalido + self.ataque_lento_demais + self.atraso_de_dano_grande_demais
    }
}

/// O resultado de [`carregar`].
#[derive(Debug, Clone, Default)]
pub struct TabelaDeMonstros {
    pub templates: HashMap<u32, TemplateDeMonstro>,
    pub recusas: Recusas,
    /// Monstros cujo `common_strategy` não existe no `aipolicy.data` — o original imprime
    /// "a política %d do monstro %d não foi achada no arquivo de políticas" e zera o
    /// campo. Guardamos os pares `(monstro, política)` para o relatório de carga.
    pub politicas_orfas: Vec<(u32, u32)>,
}

impl TabelaDeMonstros {
    pub fn get(&self, id: u32) -> Option<&TemplateDeMonstro> {
        self.templates.get(&id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

/// Lê os campos de um registro sem repetir `and_then(|v| v.as_i32()).unwrap_or(0)` em cada
/// linha. Campo ausente vira zero: o catálogo de layout pode não ter todos os nomes em
/// toda versão, e um campo a menos não deve derrubar a carga inteira.
struct Campos<'a>(&'a Record);

impl Campos<'_> {
    fn i32(&self, nome: &str) -> i32 {
        self.0.get(nome).and_then(|v| v.as_i32()).unwrap_or(0)
    }

    fn f32(&self, nome: &str) -> f32 {
        // O leitor genérico devolve `Float` para campos `float` e `Int` para `int32`; um
        // layout que classifique diferente não deve virar zero em silêncio.
        match self.0.get(nome) {
            Some(crate::generic_elements::FieldValue::Float(v)) => *v,
            Some(crate::generic_elements::FieldValue::Int(v)) => *v as f32,
            _ => 0.0,
        }
    }

    fn texto(&self, nome: &str) -> String {
        self.0.get(nome).and_then(|v| v.as_text()).unwrap_or_default().to_string()
    }

    /// Um campo de vetor achatado pelo catálogo, que numera a partir de 1
    /// (`magic_defences_1`, `skills_3_level`, …).
    fn i32_indexado(&self, prefixo: &str, indice: usize, sufixo: &str) -> i32 {
        self.i32(&format!("{prefixo}{}{sufixo}", indice + 1))
    }

    fn f32_indexado(&self, prefixo: &str, indice: usize, sufixo: &str) -> f32 {
        self.f32(&format!("{prefixo}{}{sufixo}", indice + 1))
    }
}

/// Monta a tabela de monstros a partir do `elements.data` já decodificado.
///
/// `politicas` é o `aipolicy.data` do mesmo realm, quando houver: serve só para reproduzir
/// a checagem do original — política inexistente vira `0` e entra em
/// [`TabelaDeMonstros::politicas_orfas`]. Passar `None` pula a checagem e mantém o id como
/// veio.
///
/// Devolve uma tabela vazia quando o `elements.data` não tem `MONSTER_ESSENCE` — é o caso
/// do 1.2.6/v7, que ainda usa o leitor tipado antigo.
pub fn carregar(
    elements: &GenericElementsData,
    politicas: Option<&crate::aipolicy::AiPolicyData>,
) -> TabelaDeMonstros {
    let registros = elements.get("MONSTER_ESSENCE");
    if registros.is_empty() {
        return TabelaDeMonstros::default();
    }

    let mut tabela = TabelaDeMonstros::default();

    for reg in registros {
        let c = Campos(reg);
        let id = c.i32("ID");
        if id <= 0 {
            // Registro de preenchimento — o `elements.data` tem alguns com id zero.
            continue;
        }
        let id = id as u32;

        // As três recusas do original, na mesma ordem. Elas vêm antes de qualquer outra
        // coisa porque o monstro simplesmente não entra na tabela.
        let taxa_de_ataque = c.i32("attack");
        let dano_minimo = c.i32("damage_min");
        let ataque_em_ticks = (c.f32("attack_speed") * 20.0 + 0.5) as i32;
        let atraso_do_dano_em_ticks = (c.f32("damage_delay") * 20.0) as i32;

        if atraso_do_dano_em_ticks > 256 {
            tabela.recusas.atraso_de_dano_grande_demais += 1;
            continue;
        }
        if ataque_em_ticks <= 0 || dano_minimo <= 0 || taxa_de_ataque <= 0 {
            tabela.recusas.modelo_de_ataque_invalido += 1;
            continue;
        }
        if ataque_em_ticks > 256 {
            tabela.recusas.ataque_lento_demais += 1;
            continue;
        }

        let mut resistencias = [0i32; CLASSES_MAGICAS];
        let mut dano_magico_por_classe = [FaixaDeDano::default(); CLASSES_MAGICAS];
        for i in 0..CLASSES_MAGICAS {
            resistencias[i] = c.i32_indexado("magic_defences_", i, "");
            dano_magico_por_classe[i] = FaixaDeDano {
                minimo: c.i32_indexado("magic_damages_ext_", i, "_damage_min"),
                maximo: c.i32_indexado("magic_damages_ext_", i, "_damage_max"),
            };
        }

        // 32 habilidades no arquivo, só as configuradas entram.
        let skills = (0..32)
            .filter_map(|i| {
                let id = c.i32_indexado("skills_", i, "_id_skill");
                (id > 0).then(|| SkillDeMonstro { id, nivel: c.i32_indexado("skills_", i, "_level") })
            })
            .collect();

        let por_vida = |prefixo: &str| -> Vec<SkillPorVida> {
            (0..5)
                .filter_map(|i| {
                    let id = c.i32_indexado(prefixo, i, "_id_skill");
                    (id > 0).then(|| SkillPorVida {
                        id,
                        nivel: c.i32_indexado(prefixo, i, "_level"),
                        probabilidade: c.f32_indexado(prefixo, i, "_probability"),
                    })
                })
                .collect()
        };

        let estrategias_de_odio = (0..4)
            .filter_map(|i| {
                // O original descarta probabilidade < 1e-7.
                let p = c.f32_indexado("aggro_strategy_", i, "_probability");
                (p >= 1e-7).then(|| EstrategiaDeOdio {
                    id: c.i32_indexado("aggro_strategy_", i, "_id"),
                    probabilidade: p,
                })
            })
            .collect();

        // `nt.aggro_time` tem piso de 1 no original.
        let tempo_de_odio = c.f32("aggro_time") as i32;
        let alcance_de_ataque = c.f32("attack_range");

        let mut politica_de_ia = c.i32("common_strategy").max(0) as u32;
        if politica_de_ia != 0 {
            if let Some(p) = politicas {
                if p.get_policy(politica_de_ia).is_none() {
                    tabela.politicas_orfas.push((id, politica_de_ia));
                    politica_de_ia = 0;
                }
            }
        }

        let t = TemplateDeMonstro {
            id,
            nome: c.texto("Name"),
            nivel: c.i32("level"),
            exp: c.i32("exp"),
            pontos_de_skill: c.i32("skillpoint"),
            vida: c.i32("life"),
            regeneracao_de_vida: c.i32("hp_regenerate"),
            defesa: c.i32("defence"),
            armadura: c.i32("armor"),
            resistencias,
            imunidades: c.i32("immune_type"),
            taxa_de_ataque,
            dano_fisico: FaixaDeDano { minimo: dano_minimo, maximo: c.i32("damage_max") },
            dano_magico: FaixaDeDano {
                minimo: c.i32("magic_damage_min"),
                maximo: c.i32("magic_damage_max"),
            },
            dano_magico_por_classe,
            alcance_de_ataque,
            ataque_em_ticks,
            atraso_do_dano_em_ticks,
            grau_de_ataque: c.i32("attack_degree"),
            grau_de_defesa: c.i32("defend_degree"),
            // Derivado, não lido — ver o doc do campo.
            ataque_a_distancia: alcance_de_ataque > 6.0,
            velocidade_andando: c.f32("walk_speed"),
            velocidade_correndo: c.f32("run_speed"),
            velocidade_nadando: c.f32("swim_speed"),
            velocidade_voando: c.f32("fly_speed"),
            tamanho: c.f32("size"),
            tipo_de_habitat: c.i32("inhabit_type"),
            patrulha: c.i32("patroll_mode") != 0,
            estrategia: c.i32("id_strategy"),
            politica_de_ia,
            agressivo: c.i32("aggressive_mode"),
            raio_de_odio: c.f32("aggro_range"),
            tempo_de_odio: tempo_de_odio.max(1),
            raio_de_visao: c.i32("sight_range"),
            faccao: c.i32("faction"),
            faccao_de_monstro: c.i32("monster_faction"),
            depois_da_morte: c.i32("after_death"),
            skills,
            skills_com_75_de_vida: por_vida("skill_hp75_"),
            skills_com_50_de_vida: por_vida("skill_hp50_"),
            skills_com_25_de_vida: por_vida("skill_hp25_"),
            estrategias_de_odio,
            dinheiro_medio: c.i32("money_average"),
            dinheiro_variacao: c.i32("money_var"),
            chance_de_quantos: [
                c.f32("probability_drop_num0"),
                c.f32("probability_drop_num1"),
                c.f32("probability_drop_num2"),
                c.f32("probability_drop_num3"),
            ],
            rodadas_de_drop: c.i32("drop_times"),
            itens_de_drop: (0..32)
                .map(|i| {
                    (
                        c.i32_indexado("drop_matters_", i, "_id").max(0) as u32,
                        c.f32_indexado("drop_matters_", i, "_probability"),
                    )
                })
                .collect(),
        };

        tabela.templates.insert(id, t);
    }

    if tabela.recusas.total() > 0 {
        // Não é aviso de erro nosso: o original recusa os mesmos monstros, imprimindo
        // "modelo de ataque errado". Fica visível porque um monstro recusado some do mundo.
        warn!(
            "MONSTER_ESSENCE: {} monstro(s) recusados como o original recusa — \
             {} com modelo de ataque inválido, {} com ataque acima de 256 ticks, \
             {} com atraso de dano acima de 256 ticks",
            tabela.recusas.total(),
            tabela.recusas.modelo_de_ataque_invalido,
            tabela.recusas.ataque_lento_demais,
            tabela.recusas.atraso_de_dano_grande_demais
        );
    }
    if !tabela.politicas_orfas.is_empty() {
        warn!(
            "{} monstro(s) apontam para política de IA que não existe no aipolicy.data \
             (zerados, como o original faz): {:?}",
            tabela.politicas_orfas.len(),
            &tabela.politicas_orfas[..tabela.politicas_orfas.len().min(5)]
        );
    }
    info!("MONSTER_ESSENCE: {} templates de monstro carregados", tabela.templates.len());

    tabela
}
