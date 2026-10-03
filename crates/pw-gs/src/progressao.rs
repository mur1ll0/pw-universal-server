//! O laço de progressão do jogador: experiência do abate, subida de nível, regeneração e
//! renascimento.
//!
//! Funções puras sobre [`PlayerEntity`] e as tabelas do realm — quem manda os comandos ao
//! cliente é o `BusServer`. As regras vêm do `gplayer_imp` do servidor 1.5.5
//! (`cgame/gs/player.cpp`) e as tabelas de [`pw_data_loader::progressao`].

use crate::entity::{MonsterEntity, PlayerEntity};
use pw_data_loader::progressao::NIVEL_MAXIMO_DO_JOGO;
use pw_data_loader::{GameDataManager, TabelaDeProgressao};

/// `player_template::GetStatusPointPerLevel` (`playertemplate.h:329`).
pub const PONTOS_POR_NIVEL: i32 = 5;
/// `MAX_COMBAT_TIME` / `NORMAL_COMBAT_TIME` (`gs/config.h:31-32`).
pub const COMBATE_AO_ATACAR_S: i32 = 15;
pub const COMBATE_AO_APANHAR_S: i32 = 5;
/// `DEFAULT_RESURRECT_HP_FACTOR` / `_MP_FACTOR` (`gs/config.h:168-169`).
pub const FATOR_DE_RENASCIMENTO: f32 = 0.1;
/// Teto de SP (`IncExp`, `player.cpp:2842`).
const TETO_DE_SP: i64 = 2_000_000_000;

/// `player_template::GetMaxLevel`: o `logic_level_limit` do `ptemplate.conf`.
pub fn nivel_maximo(dados: &GameDataManager) -> i32 {
    dados.base_das_classes.nivel_maximo.unwrap_or(NIVEL_MAXIMO_DO_JOGO).min(NIVEL_MAXIMO_DO_JOGO)
}

/// `gplayer_imp::IncExp` + `LevelUp` (`player.cpp:2831-2896`, `2627-2711`). Soma SP e
/// experiência e sobe quantos níveis couberem. Devolve quantos níveis subiu.
///
/// Na subida: `exp -= GetLvlupExp(nível)`, nível + 1, cinco pontos de atributo, atributos
/// refeitos (`property_policy::UpdatePlayer`) e vida e mana cheias. No teto de nível a
/// experiência zera e não acumula.
pub fn receber_exp(p: &mut PlayerEntity, exp: i64, sp: i64, dados: &GameDataManager) -> i32 {
    let sp = sp.max(0).min(TETO_DE_SP - p.sp);
    p.sp += sp.max(0);
    let maximo = nivel_maximo(dados);
    let mut exp = exp.max(0);
    if p.level >= maximo {
        exp = 0;
    }
    if exp == 0 {
        return 0;
    }
    p.exp += exp;
    subir_de_nivel(p, &dados.progressao, maximo, dados)
}

fn subir_de_nivel(p: &mut PlayerEntity, tabela: &TabelaDeProgressao, maximo: i32, dados: &GameDataManager) -> i32 {
    let mut subiu = 0;
    loop {
        let precisa = tabela.exp_para_subir(p.level);
        if precisa > p.exp {
            break;
        }
        p.exp -= precisa;
        p.level += 1;
        p.pontos_de_atributo += PONTOS_POR_NIVEL;
        subiu += 1;
        if p.level >= maximo {
            p.exp = 0;
            break;
        }
    }
    if subiu > 0 {
        let base = Some(&dados.base_das_classes).filter(|b| !b.is_empty());
        p.recalcular_por_nivel(&dados.classes, base);
        p.hp = p.max_hp;
        p.mp = p.max_mp;
    }
    subiu
}

/// `TEAM_EXP_DISTANCE` (100 m, `gs/config.h:47`; `10000.0` comparado ao quadrado da distância
/// no `ReceiveGroupExp` do `gs` 1.2.6, VA 0x8069e56).
pub const DISTANCIA_DA_EXP_DE_EQUIPE: f32 = 100.0;
/// `MIN_TEAM_DISEXP_LEVEL` (`gs/config.h:143`; `cmp ..., 0x13` no 1.2.6): o piso de nível na
/// partilha.
pub const NIVEL_MINIMO_NA_PARTILHA: i32 = 20;

/// O que a repartição usa do monstro morto.
#[derive(Debug, Clone)]
pub struct Abatido<'a> {
    pub danos: &'a [(i64, i64)],
    pub primeiro_atacante: Option<i64>,
    pub max_hp: i64,
    pub exp: i64,
    pub sp: i64,
    pub level: i32,
    pub template_id: u32,
    pub position: pw_core::Vector3,
}

impl<'a> Abatido<'a> {
    pub fn de(m: &'a MonsterEntity) -> Self {
        Self {
            danos: &m.danos,
            primeiro_atacante: m.primeiro_atacante,
            max_hp: m.max_hp,
            exp: m.exp,
            sp: m.sp,
            level: m.level,
            template_id: m.template_id,
            position: m.position,
        }
    }
}

/// Quem toma parte no abate, como o mundo o vê na hora da morte.
#[derive(Debug, Clone, Copy)]
pub struct Participante {
    pub nivel: i32,
    pub classe: i32,
    pub pos: pw_core::Vector3,
    pub grupo: Option<u32>,
}

/// O que cada jogador recebe de um abate.
#[derive(Debug, Clone, PartialEq)]
pub enum ParteDoAbate {
    /// `GM_MSG_EXPERIENCE` → `ReceiveExp(msg_exp_t)`: já com o ajuste de nível.
    Sozinho { id: i64, exp: i64, sp: i64 },
    /// `GM_MSG_TEAM_EXPERIENCE` → `ReceiveExp(exp, sp)` sem ajuste e
    /// `OnTaskTeamKillMonster(monstro, nivel, sorteio)` (`gs/player.cpp:1138-1170`). `monstro` é o
    /// modelo só para a equipe de maior dano; para as outras, 0 (`npc.cpp:1645-1656`).
    EmEquipe { id: i64, exp: i64, sp: i64, monstro: u32, nivel: i32, sorteio: f32 },
}

/// `gnpc_imp::DispatchExp` (`npc.cpp:1515-1716`) + `gplayer_imp::ReceiveGroupExp`
/// (`player.cpp:2713-2811`) + `player_team::DispatchExp` (`playerteam.cpp:1530-1570`).
///
/// Quem está em equipe soma o dano à equipe; a equipe de maior dano (comparada ao dano
/// equivalente de quem está só) leva o modelo do monstro para as missões de equipe. A parte de
/// cada equipe vai a quem está a até 100 m do monstro — membros que bateram e os que não —,
/// encolhida se algum dos que bateram está longe; multiplicada pelo ajuste da diferença entre o
/// maior nível e o do monstro e, com menos de 20 níveis entre o maior e o menor, pelo bônus
/// `SetTeamBonus(membros, classes)`; e dividida pelo nível (piso 20).
///
/// **Diferença conhecida:** o original grava a equipe no golpe (`damage_entry.team_id`); aqui
/// vale a equipe na hora da morte. As classes da equipe (`CalcRaceCount`) são as dos membros
/// neste mundo.
pub fn repartir_abate(
    m: &Abatido,
    tabela: &TabelaDeProgressao,
    quem: &std::collections::HashMap<i64, Participante>,
    membros: impl Fn(u32) -> Vec<i64>,
    sorteio: f32,
) -> Vec<ParteDoAbate> {
    if m.danos.is_empty() {
        return Vec::new();
    }
    // Entradas na ordem do primeiro aparecimento: `Err(id)` sozinho, `Ok(grupo)` equipe.
    let mut ordem: Vec<Result<u32, i64>> = Vec::new();
    let mut dano_de: std::collections::HashMap<Result<u32, i64>, i64> = std::collections::HashMap::new();
    let mut lista_de: std::collections::HashMap<u32, Vec<(i64, i64)>> = std::collections::HashMap::new();
    let (mut total, mut maior, mut equipe_maior) = (0i64, -1i64, None::<u32>);
    for &(id, dano) in m.danos {
        let equivalente = if Some(id) == m.primeiro_atacante { dano + (m.max_hp >> 2) } else { dano };
        total += dano;
        let chave = match quem.get(&id).and_then(|p| p.grupo) {
            Some(g) => Ok(g),
            None => Err(id),
        };
        if !dano_de.contains_key(&chave) {
            ordem.push(chave);
        }
        let d = dano_de.entry(chave).or_insert(0);
        *d += dano;
        match chave {
            Ok(g) => {
                lista_de.entry(g).or_default().push((id, dano));
                if maior < *d {
                    maior = *d;
                    equipe_maior = Some(g);
                }
            }
            Err(_) => {
                if maior < equivalente {
                    maior = equivalente;
                    equipe_maior = None;
                }
            }
        }
    }
    let fator = 1.0 / total.max(m.max_hp).max(1) as f32;
    let perto = |p: &Participante| p.pos.distance_squared(&m.position) <= DISTANCIA_DA_EXP_DE_EQUIPE * DISTANCIA_DA_EXP_DE_EQUIPE;
    let mut partes = Vec::new();
    for chave in ordem {
        let dano = dano_de[&chave];
        let exp = (m.exp as f32 * fator * dano as f32 + 0.5) as i64;
        let sp = (m.sp as f32 * fator * dano as f32 + 0.5) as i64;
        if exp <= 0 {
            continue;
        }
        match chave {
            Err(id) => {
                let Some(p) = quem.get(&id) else { continue };
                let a = tabela.ajuste(p.nivel - m.level);
                partes.push(ParteDoAbate::Sozinho { id, exp: (exp as f32 * a.exp + 0.5) as i64, sp: (sp as f32 * a.sp + 0.5) as i64 });
            }
            Ok(g) => {
                let (monstro, s) = if equipe_maior == Some(g) { (m.template_id, sorteio) } else { (0, 0.0) };
                partes.extend(partilhar_na_equipe(m, tabela, quem, &membros(g), &lista_de[&g], dano, exp, sp, monstro, s, &perto));
            }
        }
    }
    partes
}

/// `ReceiveGroupExp` + `player_team::DispatchExp`.
#[allow(clippy::too_many_arguments)]
fn partilhar_na_equipe(
    m: &Abatido,
    tabela: &TabelaDeProgressao,
    quem: &std::collections::HashMap<i64, Participante>,
    equipe: &[i64],
    lista: &[(i64, i64)],
    dano_da_equipe: i64,
    mut exp: i64,
    mut sp: i64,
    monstro: u32,
    sorteio: f32,
    perto: &impl Fn(&Participante) -> bool,
) -> Vec<ParteDoAbate> {
    let fator = 1.0 / dano_da_equipe.max(1) as f32;
    let mut recebem: Vec<(i64, i32)> = Vec::new();
    let mut dano_perto = 0i64;
    for &(id, dano) in lista {
        if let Some(p) = quem.get(&id).filter(|p| perto(p)) {
            dano_perto += dano;
            if !recebem.iter().any(|r| r.0 == id) {
                recebem.push((id, p.nivel));
            }
        }
    }
    if dano_perto == 0 || recebem.is_empty() {
        return Vec::new();
    }
    if dano_perto < dano_da_equipe {
        let f = fator * dano_perto as f32;
        exp = (f * exp as f32 + 0.5) as i64;
        sp = (f * sp as f32 + 0.5) as i64;
    }
    for &id in equipe {
        if recebem.iter().any(|r| r.0 == id) {
            continue;
        }
        if let Some(p) = quem.get(&id).filter(|p| perto(p)) {
            recebem.push((id, p.nivel));
        }
    }
    let soma: i32 = recebem.iter().map(|r| r.1.max(NIVEL_MINIMO_NA_PARTILHA)).sum();
    let maior = recebem.iter().map(|r| r.1).max().unwrap_or(0);
    let menor = recebem.iter().map(|r| r.1).min().unwrap_or(0);
    if soma <= 0 {
        return Vec::new();
    }
    let a = tabela.ajuste(maior - m.level);
    let (mut ae, mut asp) = (a.exp, a.sp);
    if maior - menor < 20 {
        let mut classes: Vec<i32> = equipe.iter().filter_map(|id| quem.get(id)).map(|p| p.classe & 0x1F).collect();
        classes.sort_unstable();
        classes.dedup();
        let (be, bs) = tabela.bonus_de_equipe(recebem.len(), classes.len());
        ae *= be;
        asp *= bs;
    }
    exp = (exp as f32 * ae + 0.5) as i64;
    sp = (sp as f32 * asp + 0.5) as i64;
    let fator = 1.0 / soma as f32;
    recebem
        .into_iter()
        .map(|(id, nivel)| {
            let f = nivel.max(NIVEL_MINIMO_NA_PARTILHA) as f32 * fator;
            ParteDoAbate::EmEquipe {
                id,
                exp: (exp as f32 * f + 0.5) as i64,
                sp: (sp as f32 * f + 0.5) as i64,
                monstro,
                nivel: m.level,
                sorteio,
            }
        })
        .collect()
}

/// O dono do abate: maior dano, com o primeiro atacante valendo `max_hp/4` a mais
/// (`DispatchExp`, `npc.cpp:1533-1555`).
pub fn dono_do_abate(m: &MonsterEntity) -> Option<i64> {
    let mut dono = None;
    let mut maior = -1i64;
    for &(quem, dano) in &m.danos {
        let equivalente = if Some(quem) == m.primeiro_atacante { dano + (m.max_hp >> 2) } else { dano };
        if maior < equivalente {
            maior = equivalente;
            dono = Some(quem);
        }
    }
    dono
}

/// `func::Update` (`actobject.h:2143-2154`): o gerador soma no contador, e só os oitavos
/// inteiros viram ponto.
fn gerar(base: &mut i32, contador: &mut i32, gen: i32, maximo: i32) {
    *contador += gen;
    let resto = *contador & 0x07;
    *base += *contador >> 3;
    *contador = resto;
    *base = (*base).clamp(0, maximo.max(0));
}

/// Um batimento de 1 s do jogador vivo (`gplayer_imp::OnHeartbeat`, `player.cpp:9068-9139`):
/// desconta o combate e regenera — `hp_gen`/`mp_gen` em combate, quatro vezes isso fora.
/// Devolve `true` se vida ou mana mudaram.
pub fn batimento(p: &mut PlayerEntity) -> bool {
    // Saiu do combate: o `SELF_INFO_00` vai mesmo sem vida nem mana mudarem — é o único
    // lugar em que o cliente apaga o estado de luta (`m_bFight = pCmd->State`,
    // `EC_HostMsg.cpp:1335`). A captura do 1.2.6 original tem o aviso em que só o estado
    // passa de 1 a 0 (`full_interno.pcap`, t = 2409,9 s e 2441,9 s). Sem ele, de vida e
    // mana cheias, o personagem ficava em combate para sempre (B106).
    let saiu_do_combate = p.combate_s == 1;
    if p.combate_s > 0 {
        p.combate_s -= 1;
    }
    if p.hp <= 0 {
        return false;
    }
    let (hp, mp) = (p.hp, p.mp);
    let fator = if p.combate_s > 0 { 1 } else { 4 };
    // `sit_down_filter::Heartbeat` (`gs/sitdown_filter.cpp:19-34`; no `gs` 1.2.6, VA 0x812ff22,
    // os mesmos `push 0x64`): nasce com `_timeout` 1 e no segundo batimento sentado soma
    // `STAYIN_BONUS` (100, `gs/config.h:103`) à escala da regeneração — `Result2(gen, 100, 0)`
    // dobra o `hp_gen`/`mp_gen` (`playertemplate.h:880-894`) até se levantar (`OnRelease`).
    if p.sentado {
        p.meditacao_s = p.meditacao_s.saturating_add(1);
    } else {
        p.meditacao_s = 0;
    }
    let meditacao = if p.meditacao_s >= 2 { 2 } else { 1 };
    gerar(&mut p.hp, &mut p.contador_hp, p.hp_gen * meditacao * fator, p.max_hp);
    gerar(&mut p.mp, &mut p.contador_mp, p.mp_gen * meditacao * fator, p.max_mp);
    // Meditando, o chi da versão por batimento (`ModifyAP(15)` no mesmo `Heartbeat` do 1.5.5;
    // o do 1.2.6 não tem a chamada — `chi_ao_meditar` 0, B119).
    let chi = p.sentado && p.chi_ao_meditar != 0 && p.mexer_no_chi(p.chi_ao_meditar);
    hp != p.hp || mp != p.mp || chi || saiu_do_combate
}

/// `sit_down_filter::Heartbeat`: 15 de chi por segundo meditando.
pub const CHI_POR_MEDITACAO: i32 = 15;
/// O valor com que o jogador nasce, antes de sentar (o barramento o põe pela versão).
pub const CHI_POR_MEDITACAO_155: i32 = CHI_POR_MEDITACAO;

/// Onde renascer: o ponto de cidade do distrito que contém a posição, se ele for deste mapa
/// (`gplayer_controller::ResurrectInTown`, `playercmd.cpp:112-129`; `city_region::GetCityPos`).
/// `None` quando não há distrito — o original renasce no lugar.
pub fn ponto_de_renascimento(dados: &GameDataManager, mapa: i32, x: f32, z: f32) -> Option<([f32; 3], i32)> {
    let d = dados.distritos.get(&mapa)?.distrito_em(x, z, mapa)?;
    Some((d.ponto_de_cidade, d.mapa_do_ponto))
}

/// `LOW_PROTECT_LEVEL` (`gs/config.h:74`): até este nível, renascer não custa experiência.
pub const NIVEL_SEM_PERDA_AO_RENASCER: i32 = 9;

/// `gplayer_imp::Resurrect` (`player.cpp:8716-8768`): vida e mana a 10 %, e a perda de
/// experiência — `GetLvlupExp(nível) × GetResurrectExpReduce(cultivo)`, sem ficar negativa
/// e zero no teto de nível. Devolve quanto perdeu.
pub fn renascer(p: &mut PlayerEntity, dados: &GameDataManager, morto_por_jogador: bool) -> i64 {
    p.hp = (p.max_hp as f32 * FATOR_DE_RENASCIMENTO + 0.5) as i32;
    p.mp = (p.max_mp as f32 * FATOR_DE_RENASCIMENTO + 0.5) as i32;
    p.combate_s = 0;
    // `if (pImp->_basic.level <= LOW_PROTECT_LEVEL) exp_reduce = 0.f` (`playercmd.cpp:697-699`,
    // `LOW_PROTECT_LEVEL` 9 em `gs/config.h:74`; `cmp word [+0xe4], 9` no `gs` 1.2.6, VA
    // 0x80cd2b4) — B155.
    let reducao = if morto_por_jogador || p.level <= NIVEL_SEM_PERDA_AO_RENASCER {
        0.0
    } else {
        dados.progressao.perda_na_morte(p.cultivation)
    };
    let exp_do_nivel = dados.progressao.exp_para_subir(p.level);
    let perda = (exp_do_nivel as f32 * reducao + 0.5) as i64;
    if perda > 0 {
        let mut nova = (p.exp - perda).max(0);
        if p.level >= nivel_maximo(dados) {
            nova = 0;
        }
        let perdeu = p.exp - nova;
        p.exp = nova;
        return perdeu;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participante(nivel: i32, classe: i32, x: f32, grupo: Option<u32>) -> Participante {
        Participante { nivel, classe, pos: pw_core::Vector3 { x, y: 0.0, z: 0.0 }, grupo }
    }

    fn abatido(danos: &[(i64, i64)]) -> Abatido<'_> {
        Abatido {
            danos,
            primeiro_atacante: None,
            max_hp: 1000,
            exp: 1000,
            sp: 100,
            level: 30,
            template_id: 77,
            position: pw_core::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        }
    }

    fn tabela() -> TabelaDeProgressao {
        TabelaDeProgressao::com_ajuste_uniforme(pw_data_loader::progressao::AjusteDeNivel { exp: 1.0, sp: 1.0, ..Default::default() })
    }

    #[test]
    fn sozinho_recebe_pela_fracao_do_dano_com_o_ajuste_de_nivel() {
        let quem = std::collections::HashMap::from([(1, participante(30, 0, 0.0, None)), (2, participante(30, 1, 0.0, None))]);
        let danos = [(1, 600), (2, 400)];
        let p = repartir_abate(&abatido(&danos), &tabela(), &quem, |_| vec![], 0.5);
        assert_eq!(p, vec![ParteDoAbate::Sozinho { id: 1, exp: 600, sp: 60 }, ParteDoAbate::Sozinho { id: 2, exp: 400, sp: 40 }]);
    }

    #[test]
    fn a_equipe_reparte_por_nivel_a_100_m_e_a_de_maior_dano_leva_o_monstro() {
        // Equipe 9: 1 (nível 30) e 2 (nível 10, piso 20) bateram; 3 não bateu mas está a 10 m;
        // 4 está a 150 m. O 5, sozinho, bateu menos que a equipe.
        let quem = std::collections::HashMap::from([
            (1, participante(30, 0, 0.0, Some(9))),
            (2, participante(10, 1, 50.0, Some(9))),
            (3, participante(30, 1, 10.0, Some(9))),
            (4, participante(30, 2, 150.0, Some(9))),
            (5, participante(30, 0, 0.0, None)),
        ]);
        let danos = [(1, 500), (5, 300), (2, 200)];
        let p = repartir_abate(&abatido(&danos), &tabela(), &quem, |_| vec![1, 2, 3, 4], 0.25);
        // 700 de exp e 70 de sp à equipe, divididos por 30/80, 20/80 e 30/80 (20 níveis entre o
        // maior e o menor: sem `SetTeamBonus`).
        let e = |id, exp, sp| ParteDoAbate::EmEquipe { id, exp, sp, monstro: 77, nivel: 30, sorteio: 0.25 };
        assert_eq!(p, vec![e(1, 263, 26), e(2, 175, 18), e(3, 263, 26), ParteDoAbate::Sozinho { id: 5, exp: 300, sp: 30 }]);
    }

    #[test]
    fn a_equipe_sem_o_maior_dano_vai_com_monstro_zero() {
        let quem = std::collections::HashMap::from([(1, participante(30, 0, 0.0, Some(9))), (5, participante(30, 0, 0.0, None))]);
        let danos = [(1, 300), (5, 700)];
        let p = repartir_abate(&abatido(&danos), &tabela(), &quem, |_| vec![1], 0.25);
        assert_eq!(p[0], ParteDoAbate::EmEquipe { id: 1, exp: 300, sp: 30, monstro: 0, nivel: 30, sorteio: 0.0 });
    }

    #[test]
    fn a_regeneracao_acumula_oitavos() {
        let (mut base, mut cont) = (10, 0);
        gerar(&mut base, &mut cont, 3, 100);
        assert_eq!((base, cont), (10, 3));
        gerar(&mut base, &mut cont, 13, 100);
        assert_eq!((base, cont), (12, 0));
        gerar(&mut base, &mut cont, 800, 100);
        assert_eq!(base, 100, "não passa do máximo");
    }
}
