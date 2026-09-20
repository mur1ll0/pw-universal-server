//! O cálculo de dano do servidor original, portado.
//!
//! # Origem
//!
//! `gactive_imp::AttackJudgement` e o trecho de `gactive_imp::HandleAttackMsg` que vem
//! depois dele (`cgame/gs/actobject.cpp`, fonte 1.5.5), mais os auxiliares estáticos de
//! `player_template` (`cgame/gs/playertemplate.h`). Cada passo abaixo cita a linha que o
//! origina.
//!
//! # O que havia antes
//!
//! Duas funções de 20 linhas com fórmula inventada:
//!
//! ```text
//! def_factor = 1 / (1 + def / (100 * nível_do_atacante))
//! dano = uniforme(attack_min, attack_max) * def_factor * (crítico ? 2 : 1)
//! ```
//!
//! Não havia rolagem de acerto, nem classe mágica, nem imunidade, nem grau de ataque e
//! defesa. E a redução por defesa não tem relação nenhuma com a do original — que é
//! `def / (def + 40*nível - 25)`, com teto de 0,95.
//!
//! # A ordem exata
//!
//! 1. **Acerto** (só ataque físico): `taxa / (taxa + armadura/2)`, com piso de 0,05.
//!    Errou aqui, o golpe acaba — sem dano, sem crítico.
//! 2. **Curta distância**: golpe de habilidade multiplica pelo fator próprio; golpe
//!    normal tem o dano físico **dividido por 2**.
//! 3. **Atenuação por distância**, só quando quem ataca é jogador ou pet.
//! 4. **Defesa, por classe de dano**: físico contra `defesa`, e cada uma das cinco
//!    classes mágicas contra a sua `resistência`. Imunidade zera a classe inteira.
//!    A redução usa o **nível do atacante**, não o do alvo.
//! 5. **Crítico**: multiplica o total por `2,0 + bônus% − redução%`.
//! 6. **Grau de ataque menos grau de defesa**, com curvas diferentes para positivo e
//!    negativo.
//! 7. **Piso de 1**: golpe que acertou nunca faz zero.
//!
//! # O que ainda não entra, e por quê
//!
//! O original passa por `AdjustDamage` (virtual, por tipo de objeto), pelos *filters*
//! (`EF_AdjustDamage`/`EF_DoDamage`), pelo vigor (`GetVigourEnhance`), pela esquiva de
//! dano e de *debuff* (`_damage_dodge_rate`, `_debuff_dodge_rate`) e pelo roubo de vida.
//! Nenhum desses sistemas existe do nosso lado — não há *filter*, não há vigor, não há
//! buff que altere dano. Entram quando existirem; deixar um lugar reservado para eles
//! seria fingir que já fazem alguma coisa.
//!
//! # Sobre o acaso
//!
//! As funções deste módulo **não sorteiam**: recebem as rolagens prontas
//! ([`Rolagens`]). É o que torna o cálculo inteiro testável contra valores fechados, em
//! vez de "roda mil vezes e vê se a média parece certa". Quem sorteia é
//! [`Rolagens::sortear`], num lugar só.

use crate::entity::{MonsterEntity, PlayerEntity};
use rand::Rng;

/// Quantas classes de dano mágico existem — `MAGIC_CLASS` em `cgame/gs/config.h`. A
/// ordem é metal, madeira, água, fogo, terra.
pub const CLASSES_MAGICAS: usize = 5;

/// `CRIT_DAMAGE_BONUS` (`cgame/gs/config.h`): o crítico dobra, e o bônus de equipamento
/// soma em cima disso.
const BONUS_BASE_DE_CRITICO: f32 = 2.0;

/// Máscara de imunidade (`_immune_state`): bit 0 é físico, e os bits 1 a 5 são as cinco
/// classes mágicas — `IMMUNE_MASK_PHYSIC` e `IMMUNE_GOLD = 0x02` em `actobject.h`, com o
/// teste `imask & (1 << (i + 1))` no laço das classes.
pub const IMUNE_A_FISICO: i32 = 0x01;

/// O golpe, do lado de quem bate — `attack_msg` do original, reduzido ao que o nosso
/// mundo sabe preencher de verdade.
#[derive(Debug, Clone, PartialEq)]
pub struct Golpe {
    /// `ainfo.level`. É **este** nível que entra na redução por defesa, não o do alvo.
    pub nivel_do_atacante: i32,
    /// `attack_rate` — a precisão, que decide o acerto. **Não** é o dano.
    pub taxa_de_ataque: i32,
    /// `physic_damage`, já sorteado pelo atacante.
    pub dano_fisico: i32,
    /// `magic_damage[0..4]`, já sorteados.
    pub dano_magico: [i32; CLASSES_MAGICAS],
    /// `attack_attr == attack_msg::PHYSIC_ATTACK`. Só ataque físico rola acerto; o
    /// mágico sempre acerta (`AttackJudgement` só testa nesse caso).
    pub e_fisico: bool,
    /// `crit_rate`, em pontos percentuais.
    pub chance_de_critico: i32,
    /// `crit_damage_bonus`, em pontos percentuais, somado ao dobro base.
    pub bonus_de_dano_critico: i32,
    /// `attack_degree`.
    pub grau_de_ataque: i32,
    /// `skill_id != 0`. Muda o que a curta distância faz com o dano.
    pub de_habilidade: bool,
    /// `short_range_adjust_factor` — só usado em golpe de habilidade a curta distância.
    pub fator_de_curta_distancia: f32,
    /// `anti_defense_degree` / `anti_resistance_degree`: 0 significa "sem penetração", e
    /// aí a defesa do alvo entra inteira.
    pub anti_defesa: i32,
    pub anti_resistencia: i32,
    /// `attack->ainfo.attacker.IsPlayer() || IsPet()` — só nesse caso a distância atenua.
    pub atacante_e_jogador_ou_pet: bool,
}

/// O que o alvo opõe — os campos de `_cur_prop` que o cálculo consulta.
#[derive(Debug, Clone, PartialEq)]
pub struct Defesa {
    /// `_cur_prop.armor`: a evasão, que decide o acerto junto com a taxa de ataque.
    pub armadura: i32,
    /// `_cur_prop.defense`: reduz o dano físico.
    pub defesa: i32,
    /// `_cur_prop.resistance[0..4]`: reduzem o dano mágico, uma por classe.
    pub resistencias: [i32; CLASSES_MAGICAS],
    /// `_defend_degree`.
    pub grau_de_defesa: i32,
    /// `_immune_state | _immune_state_adj` — ver [`IMUNE_A_FISICO`].
    pub imunidades: i32,
    /// `_crit_resistance`, subtraída da chance de crítico do atacante.
    pub resistencia_a_critico: i32,
    /// `_crit_damage_reduce`, em pontos percentuais.
    pub reducao_de_dano_critico: i32,
    /// `_near_skill_dmg_reduce` / `_far_skill_dmg_reduce` e os pares para golpe normal.
    /// Vêm de equipamento e habilidade passiva no original; sem esse sistema do nosso
    /// lado eles ficam em zero, e a atenuação por distância não faz nada — que é o
    /// comportamento certo para "não tenho o dado", não um atalho.
    pub reducao_perto_habilidade: f32,
    pub reducao_longe_habilidade: f32,
    pub reducao_perto_normal: f32,
    pub reducao_longe_normal: f32,
}

impl Defesa {
    /// Uma defesa sem nenhum dos sistemas que ainda não existem — imunidade, crítico e
    /// atenuação por distância zerados. Serve para montar um alvo a partir de armadura,
    /// defesa, resistências e graus, que é tudo que as nossas entidades têm.
    pub fn simples(
        armadura: i32,
        defesa: i32,
        resistencias: [i32; CLASSES_MAGICAS],
        grau_de_defesa: i32,
    ) -> Self {
        Self {
            armadura,
            defesa,
            resistencias,
            grau_de_defesa,
            imunidades: 0,
            resistencia_a_critico: 0,
            reducao_de_dano_critico: 0,
            reducao_perto_habilidade: 0.0,
            reducao_longe_habilidade: 0.0,
            reducao_perto_normal: 0.0,
            reducao_longe_normal: 0.0,
        }
    }
}

/// As rolagens que o golpe precisa, separadas do cálculo para o resultado ser
/// reproduzível em teste.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rolagens {
    /// `abase::RandUniform()` da checagem de acerto: erra quando é **maior** que a
    /// chance.
    pub acerto: f64,
    /// `abase::Rand(0, 99)` da checagem de crítico: é crítico quando é **menor** que a
    /// chance efetiva.
    pub critico: i32,
}

impl Rolagens {
    pub fn sortear() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            acerto: rng.gen::<f64>(),
            critico: rng.gen_range(0..100),
        }
    }
}

/// Como o golpe terminou.
///
/// O original manda o **mesmo** pacote ao cliente para [`Self::Errou`] e
/// [`Self::SemEfeito`] (`_runner->dodge_attack`); a distinção existe aqui porque as
/// causas são diferentes e confundi-las esconde bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resultado {
    /// A rolagem de acerto falhou. Só acontece em ataque físico.
    Errou,
    /// Acertou, mas não sobrou dano nenhum: toda classe de dano estava zerada ou imune.
    /// É o `attacked == false` do `AttackJudgement`.
    SemEfeito,
    Acertou {
        dano: i32,
        critico: bool,
        /// Alguma classe de dano foi barrada por imunidade (`AT_STATE_IMMUNE`).
        alguma_imunidade: bool,
    },
}

impl Resultado {
    /// O dano a debitar — zero quando o golpe não chegou.
    pub fn dano(&self) -> i32 {
        match self {
            Resultado::Acertou { dano, .. } => *dano,
            _ => 0,
        }
    }

    pub fn foi_critico(&self) -> bool {
        matches!(self, Resultado::Acertou { critico: true, .. })
    }
}

/// A chance de o golpe físico acertar — `AttackJudgement`, `actobject.cpp`:
///
/// ```text
/// attack_pb = rate / (rate + (armor >> 1));
/// if (attack_pb < 0.05f) attack_pb = 0.05f;
/// ```
///
/// Note o `>> 1` inteiro: a armadura entra pela metade, arredondada para baixo. Não há
/// teto — o original tem a linha do teto de 0,95 comentada.
pub fn chance_de_acerto(taxa_de_ataque: i32, armadura: i32) -> f32 {
    let meia_armadura = armadura >> 1;
    let denominador = taxa_de_ataque + meia_armadura;
    if denominador == 0 {
        // Não acontece com dado real (a taxa de ataque é sempre positiva), mas dividir
        // por zero aqui daria NaN e o NaN passaria por qualquer comparação como "errou".
        return 0.05;
    }
    let pb = taxa_de_ataque as f32 / denominador as f32;
    pb.max(0.05)
}

/// Quanto da defesa é descontado do dano — `player_template::GetDamageReduce`:
///
/// ```text
/// def = def / (def + 40*attacker_level - 25);
/// if (def > 0.95f) def = 0.95f;
/// ```
///
/// O nível é o de **quem ataca**: a mesma defesa segura proporcionalmente menos contra um
/// atacante de nível alto.
pub fn reducao_por_defesa(defesa: i32, nivel_do_atacante: i32) -> f32 {
    let def = defesa as f32;
    let denominador = def + 40.0 * nivel_do_atacante as f32 - 25.0;
    if denominador == 0.0 {
        return 0.0;
    }
    (def / denominador).min(0.95)
}

/// A defesa que sobra depois da penetração — `player_template::CalcAntiDef`:
///
/// ```text
/// anti_ratio = anti_degree / (anti_degree + 10000);  // no máximo 0,35
/// return def * (1 - anti_ratio);
/// ```
pub fn defesa_apos_penetracao(grau_de_penetracao: i32, defesa: i32) -> i32 {
    let grau = grau_de_penetracao as f32;
    let mut razao = grau / (grau + 10_000.0);
    if razao < 0.0 {
        razao = 0.0;
    }
    if razao > 0.35 {
        razao = 0.35;
    }
    (defesa as f32 * (1.0 - razao)) as i32
}

/// `player_template::GetFarDamageReduceFactor` — o alvo longe demais recebe menos.
pub fn fator_de_reducao_longe(distancia: f32, razao: f32) -> f32 {
    if distancia <= 8.0 {
        return 0.0;
    }
    let d = distancia.min(40.0);
    (razao * (d - 8.0) / (d + 2.0)).min(1.0)
}

/// `player_template::GetNearDamageReduceFactor` — o alvo perto demais recebe menos.
pub fn fator_de_reducao_perto(distancia: f32, razao: f32) -> f32 {
    if distancia >= 8.0 {
        return 0.0;
    }
    razao.min(1.0)
}

/// Resolve um golpe inteiro: acerto, defesa por classe, crítico e graus.
///
/// `distancia` é a que separa os dois no momento do golpe, e `a_curta_distancia` é o
/// `fTmp < short_range²` do original — o alvo estar **dentro** da distância mínima da
/// arma, que penaliza em vez de ajudar.
pub fn resolver(
    golpe: &Golpe,
    defesa: &Defesa,
    distancia: f32,
    a_curta_distancia: bool,
    rolagens: Rolagens,
) -> Resultado {
    // ---- 1. acerto (`AttackJudgement`, só para ataque físico) ----
    if golpe.e_fisico {
        let chance = chance_de_acerto(golpe.taxa_de_ataque, defesa.armadura);
        if rolagens.acerto > chance as f64 {
            return Resultado::Errou;
        }
    }

    let mut dano_fisico = golpe.dano_fisico as f32;
    let mut dano_magico = golpe.dano_magico.map(|d| d as f32);

    // ---- 2. curta distância ----
    if a_curta_distancia {
        if golpe.de_habilidade {
            dano_fisico *= golpe.fator_de_curta_distancia;
            for d in &mut dano_magico {
                *d *= golpe.fator_de_curta_distancia;
            }
        } else {
            // Golpe normal dentro da distância mínima da arma perde metade do dano
            // físico — e só do físico.
            dano_fisico = (golpe.dano_fisico / 2) as f32;
        }
    }

    // ---- 3. atenuação por distância (só jogador ou pet atacando) ----
    if golpe.atacante_e_jogador_ou_pet {
        if golpe.de_habilidade {
            let razao = 1.0
                - fator_de_reducao_perto(distancia, defesa.reducao_perto_habilidade)
                - fator_de_reducao_longe(distancia, defesa.reducao_longe_habilidade);
            let razao = razao.max(0.0);
            dano_fisico *= razao;
            for d in &mut dano_magico {
                *d *= razao;
            }
        } else {
            // No golpe normal o original atenua **só** o dano físico.
            let razao = 1.0
                - fator_de_reducao_perto(distancia, defesa.reducao_perto_normal)
                - fator_de_reducao_longe(distancia, defesa.reducao_longe_normal);
            dano_fisico *= razao.max(0.0);
        }
    }

    // ---- 4. defesa, por classe de dano ----
    let nivel = golpe.nivel_do_atacante;
    let mut acertou_alguma_coisa = false;
    let mut alguma_imunidade = false;
    let mut total = 0.0f32;

    if golpe.dano_fisico != 0 {
        if defesa.imunidades & IMUNE_A_FISICO != 0 {
            alguma_imunidade = true;
        } else {
            let def = if golpe.anti_defesa == 0 {
                defesa.defesa
            } else {
                defesa_apos_penetracao(golpe.anti_defesa, defesa.defesa)
            };
            let passou = dano_fisico * (1.0 - reducao_por_defesa(def, nivel));
            total += passou.max(0.0);
            acertou_alguma_coisa = true;
        }
    }

    for i in 0..CLASSES_MAGICAS {
        if golpe.dano_magico[i] == 0 {
            continue;
        }
        // `imask & (1 << (i + 1))`: o bit 0 é o físico, e as classes começam no bit 1.
        if defesa.imunidades & (1 << (i + 1)) != 0 {
            alguma_imunidade = true;
            continue;
        }
        let res = if golpe.anti_resistencia == 0 {
            defesa.resistencias[i]
        } else {
            defesa_apos_penetracao(golpe.anti_resistencia, defesa.resistencias[i])
        };
        let passou = dano_magico[i] * (1.0 - reducao_por_defesa(res, nivel));
        total += passou.max(0.0);
        acertou_alguma_coisa = true;
    }

    if !acertou_alguma_coisa {
        // `AttackJudgement` devolveu false: o original trata como esquiva.
        return Resultado::SemEfeito;
    }

    // ---- 5. crítico ----
    let mut ajuste = 1.0f32;
    let chance_efetiva = golpe.chance_de_critico - defesa.resistencia_a_critico;
    let critico = rolagens.critico < chance_efetiva;
    if critico {
        ajuste *= BONUS_BASE_DE_CRITICO + golpe.bonus_de_dano_critico as f32 * 0.01
            - defesa.reducao_de_dano_critico as f32 * 0.01;
    }

    // ---- 6. grau de ataque contra grau de defesa ----
    // As duas curvas são do original, e não são simétricas: vantagem soma 1% por ponto,
    // desvantagem divide por `1 - grau*0,012`.
    let final_adjust = golpe.grau_de_ataque - defesa.grau_de_defesa;
    let bruto = total * ajuste + 0.5;
    let dano = if final_adjust >= 0 {
        bruto * (1.0 + final_adjust as f32 * 0.01)
    } else {
        bruto / (1.0 - final_adjust as f32 * 0.012)
    };

    // ---- 7. piso de 1 ----
    let dano = (dano as i32).max(1);

    Resultado::Acertou { dano, critico, alguma_imunidade }
}

/// Sorteia o dano físico de um golpe normal — `GenerateAttackDamage`, que usa
/// `abase::Rand(low, high)`: distribuição **uniforme**.
///
/// O dano elemental é o único que usa `RandNormal` (média de dois uniformes, portanto
/// triangular) — ver [`sortear_dano_elemental`]. Trocar um pelo outro muda a forma da
/// distribuição, não só o valor.
pub fn sortear_dano_fisico(minimo: i32, maximo: i32) -> i32 {
    if minimo >= maximo {
        return minimo;
    }
    rand::thread_rng().gen_range(minimo..=maximo)
}

/// Sorteia uma parcela de dano elemental — `normalrand` em `MakeAttackMsg`, que é
/// `abase::RandNormal(int, int)`:
///
/// ```text
/// p = (RandomUniform() + RandomUniform()) * 0.5;
/// return (int)(p * (upper - lower + 1)) + lower;
/// ```
pub fn sortear_dano_elemental(minimo: i32, maximo: i32) -> i32 {
    if minimo >= maximo {
        return minimo;
    }
    let mut rng = rand::thread_rng();
    let p = (rng.gen::<f64>() + rng.gen::<f64>()) * 0.5;
    (p * (maximo - minimo + 1) as f64) as i32 + minimo
}

/// Ponte para o mundo: monta o golpe e a defesa a partir das entidades.
pub struct CombatEngine;

/// `magic_damage[3]` — a escola do fogo na ordem do original (metal, madeira, água, fogo,
/// terra), que é a mesma do `MAGIC_CLASS` e das resistências.
const ESCOLA_DO_FOGO: usize = 3;

impl CombatEngine {
    /// O golpe normal de um jogador, com o dano já sorteado.
    ///
    /// # O que aqui ainda não é real
    ///
    /// `PlayerEntity` não tem sistema de equipamento nem de pontos de atributo, então
    /// `armor`, `attack_rate`, `attack_degree` e `crit_damage_bonus` vêm do que estiver
    /// na entidade — e hoje, em produção, **nada constrói um `PlayerEntity`**
    /// (`world.players` nunca é populado). Ver `docs/ESTADO_E_RETOMADA.md`.
    ///
    /// `atacante_e_jogador_ou_pet` é `true`, e é o que liga a atenuação por distância —
    /// que não faz nada enquanto as razões do alvo forem zero.
    pub fn golpe_de_jogador(jogador: &PlayerEntity) -> Golpe {
        // `filter_Firearrow::TranslateSendAttack` (`cskill/skill/skillfilter.h:4268-4275`):
        // num golpe **físico**, soma `ratio × 0,5 × (dano_baixo + dano_alto)` **da arma
        // vestida** ao dano de fogo (`magic_damage[3]`). É o dano da arma
        // (`_parent.GetCurWeapon()`), não o dano total do personagem.
        let mut dano_magico = [0; CLASSES_MAGICAS];
        if let Some(f) = jogador.efeitos.filtros.iter().find(|f| f.efeito == crate::efeitos::Efeito::Firearrow) {
            let (baixo, alto) = jogador.equipamento.arma.map(|a| a.dano).unwrap_or((0, 0));
            dano_magico[ESCOLA_DO_FOGO] += (f.fator * 0.5 * (baixo + alto) as f32) as i32;
        }
        Golpe {
            nivel_do_atacante: jogador.level,
            taxa_de_ataque: jogador.attack_rate,
            dano_fisico: sortear_dano_fisico(jogador.attack_min, jogador.attack_max),
            dano_magico,
            e_fisico: true,
            // `crit_rate` está em fração na entidade e em pontos percentuais no original.
            chance_de_critico: (jogador.crit_rate * 100.0).round() as i32,
            bonus_de_dano_critico: jogador.crit_damage_bonus,
            grau_de_ataque: jogador.attack_degree,
            de_habilidade: false,
            fator_de_curta_distancia: 1.0,
            anti_defesa: 0,
            anti_resistencia: 0,
            atacante_e_jogador_ou_pet: true,
        }
    }

    /// O golpe normal de um monstro. `MakeAttackMsg` sorteia o dano físico **e** as cinco
    /// parcelas elementais do `addon_damage`.
    pub fn golpe_de_monstro(monstro: &MonsterEntity) -> Golpe {
        let mut dano_magico = [0i32; CLASSES_MAGICAS];
        for (i, faixa) in monstro.magic_attack.iter().enumerate() {
            dano_magico[i] = sortear_dano_elemental(faixa.0, faixa.1);
        }
        // Realces dos filtros (`Incattack`/`Decattack`, `Incaccuracy`/`Decaccuracy`).
        let r = monstro.efeitos.realce();
        use crate::entity::com_realce;
        Golpe {
            nivel_do_atacante: monstro.level,
            taxa_de_ataque: com_realce(monstro.attack_rate, r.precisao),
            dano_fisico: sortear_dano_fisico(com_realce(monstro.attack_min, r.dano), com_realce(monstro.attack_max, r.dano)),
            dano_magico,
            e_fisico: true,
            // Monstro comum não tem crítico próprio no original: o `crit_rate` do
            // `attack_msg` fica zerado por `memset` e só habilidade o preenche.
            chance_de_critico: 0,
            bonus_de_dano_critico: 0,
            grau_de_ataque: monstro.attack_degree,
            de_habilidade: false,
            fator_de_curta_distancia: 1.0,
            anti_defesa: 0,
            anti_resistencia: 0,
            atacante_e_jogador_ou_pet: false,
        }
    }

    pub fn defesa_do_monstro(monstro: &MonsterEntity) -> Defesa {
        use crate::entity::com_realce;
        let r = monstro.efeitos.realce();
        Defesa::simples(
            com_realce(monstro.armor, r.evasao),
            com_realce(monstro.def_phys, r.defesa),
            monstro.resistances.map(|x| com_realce(x, r.resistencia)),
            monstro.defend_degree,
        )
    }

    pub fn defesa_do_jogador(jogador: &PlayerEntity) -> Defesa {
        Defesa::simples(
            jogador.armor,
            jogador.def_phys,
            [
                jogador.def_metal,
                jogador.def_wood,
                jogador.def_water,
                jogador.def_fire,
                jogador.def_earth,
            ],
            jogador.defend_degree,
        )
    }

    /// Um golpe de jogador em monstro, do sorteio ao dano final.
    pub fn jogador_ataca_monstro(
        jogador: &PlayerEntity,
        monstro: &MonsterEntity,
        distancia: f32,
    ) -> Resultado {
        resolver(
            &Self::golpe_de_jogador(jogador),
            &Self::defesa_do_monstro(monstro),
            distancia,
            false,
            Rolagens::sortear(),
        )
    }

    /// O golpe de uma habilidade de dano (`SetDamage(fator × GetAttack())` e parentes).
    ///
    /// `GeneratePhysicDamage((int)ratio, (int)plus)` (`actobject.h:1422-1444`, `skill.cpp:897`,
    /// `skill.h:634` — `SetRatio` guarda `r × 100`): dano bruto sorteado × (100 + bônus do
    /// atributo + ratio%)/100 + plus; o mágico igual, com a energia (`GenerateMaigicDamage2`).
    /// `carga` escala o `ratio` das habilidades de carga (`GetCharging()` / tempo cheio).
    /// `Damage` vai na parcela física; as escolas, na mágica correspondente.
    pub fn golpe_de_habilidade(
        jogador: &PlayerEntity,
        d: &pw_data_loader::habilidades::DanoDaHabilidade,
        nivel: i32,
        carga: f32,
    ) -> Option<Golpe> {
        let i = usize::try_from(nivel - 1).ok()?;
        let ratio = d.ratio.get(i)? * if d.carga { carga.clamp(0.0, 1.0) } else { 1.0 };
        let plus = *d.plus.get(i)? as i32;
        let (faixa, bonus) = if d.base == "magico" {
            (jogador.dano_magico_bruto, jogador.bonus_magico_pct)
        } else {
            (jogador.dano_bruto, jogador.bonus_de_dano_pct)
        };
        let bruto = sortear_dano_fisico(faixa.0, faixa.1);
        let pct = 100 + bonus + (ratio * 100.0) as i32;
        let valor = (((bruto as f32 * 0.01 * pct as f32) as i32 + plus).max(0) as f32 * d.fator) as i32;
        let mut g = Self::golpe_de_jogador(jogador);
        g.de_habilidade = true;
        g.dano_fisico = 0;
        let escola = match d.elemento.as_str() {
            "Golddamage" => Some(0),
            "Wooddamage" => Some(1),
            "Waterdamage" => Some(2),
            "Firedamage" => Some(3),
            "Earthdamage" => Some(4),
            _ => None,
        };
        match escola {
            Some(e) => {
                g.dano_magico[e] = valor;
                g.e_fisico = false;
            }
            None => g.dano_fisico = valor,
        }
        Some(g)
    }

    /// Um golpe de monstro em jogador.
    pub fn monstro_ataca_jogador(
        monstro: &MonsterEntity,
        jogador: &PlayerEntity,
        distancia: f32,
    ) -> Resultado {
        resolver(
            &Self::golpe_de_monstro(monstro),
            &Self::defesa_do_jogador(jogador),
            distancia,
            false,
            Rolagens::sortear(),
        )
    }
}
