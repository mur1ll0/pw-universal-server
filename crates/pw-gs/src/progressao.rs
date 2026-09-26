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

/// A parte de um jogador na experiência de um monstro morto.
///
/// `gnpc_imp::DispatchExp` (`npc.cpp:1515`): cada um recebe `exp × dano / total`, com o total
/// nunca menor que a vida máxima do monstro; depois `gplayer_imp::ReceiveExp`
/// (`player.cpp:2813-2829`) aplica o ajuste da diferença de nível e arredonda (`+ 0.5`).
/// Devolve `(exp, sp)`.
pub fn parte_do_abate(m: &MonsterEntity, quem: i64, nivel_do_jogador: i32, tabela: &TabelaDeProgressao) -> (i64, i64) {
    let total: i64 = m.danos.iter().map(|d| d.1).sum();
    let Some(dano) = m.danos.iter().find(|d| d.0 == quem).map(|d| d.1) else {
        return (0, 0);
    };
    let base = total.max(m.max_hp).max(1) as f32;
    let fator = dano as f32 / base;
    let exp = (m.exp as f32 * fator + 0.5) as i64;
    let sp = (m.sp as f32 * fator + 0.5) as i64;
    let ajuste = tabela.ajuste(nivel_do_jogador - m.level);
    ((exp as f32 * ajuste.exp + 0.5) as i64, (sp as f32 * ajuste.sp + 0.5) as i64)
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

/// `gplayer_imp::Resurrect` (`player.cpp:8716-8768`): vida e mana a 10 %, e a perda de
/// experiência — `GetLvlupExp(nível) × GetResurrectExpReduce(cultivo)`, sem ficar negativa
/// e zero no teto de nível. Devolve quanto perdeu.
pub fn renascer(p: &mut PlayerEntity, dados: &GameDataManager, morto_por_jogador: bool) -> i64 {
    p.hp = (p.max_hp as f32 * FATOR_DE_RENASCIMENTO + 0.5) as i32;
    p.mp = (p.max_mp as f32 * FATOR_DE_RENASCIMENTO + 0.5) as i32;
    p.combate_s = 0;
    let reducao = if morto_por_jogador { 0.0 } else { dados.progressao.perda_na_morte(p.cultivation) };
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
