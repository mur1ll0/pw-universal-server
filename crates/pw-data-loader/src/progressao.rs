//! As tabelas de progressão do jogador: experiência por nível, ajuste por diferença de
//! nível e perda de experiência na morte.
//!
//! # Origem
//!
//! `player_template` do servidor original (`EvolvedPWServer/cgame/gs/playertemplate.cpp`
//! e `.h`). As três tabelas nascem com um valor padrão no construtor e são sobrescritas
//! pelo `elements.data` em `__LoadDataFromDataMan`:
//!
//! | tabela | padrão (`playertemplate.cpp:24-35`) | do `elements.data` |
//! | :--- | :--- | :--- |
//! | `_exp_list[nível]` | `nível² × 500` | `PLAYER_LEVELEXP_CONFIG` **id 202**, `exp[nível-1]` quando não zero (`:365-385`) |
//! | `_level_adjust_table[201]` | `MESMD_ADJUST` do `ptemplate.conf` | `PARAM_ADJUST_CONFIG`, `level_diff_adjust[16]` (`:311-337`) |
//! | `_death_exp_punish[256]` | `0.05` | `PLAYER_SECONDLEVEL_CONFIG`, `exp_lost[i]` quando não zero (`:403-419`) |
//!
//! O id 592 da `PLAYER_LEVELEXP_CONFIG` é a curva do mascote, não a do jogador.

use crate::generic_elements::{FieldValue, GenericElementsData, Record};

/// `MAX_PLAYER_LEVEL` (`cgame/gs/config.h:127`).
pub const NIVEL_MAXIMO_DO_JOGO: i32 = 150;
/// `player_template::MAX_LEVEL_DIFF` e `BASE_LEVEL_DIFF` (`playertemplate.h:58-59`).
pub const MAX_LEVEL_DIFF: usize = 200;
pub const BASE_LEVEL_DIFF: i32 = -100;
/// `GetStatusPointPerLevel` (`playertemplate.h:329`).
pub const PONTOS_POR_NIVEL: i32 = 5;

/// Um degrau da tabela de ajuste por diferença de nível (`level_adjust`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AjusteDeNivel {
    pub exp: f32,
    pub sp: f32,
    pub dinheiro: f32,
    pub item: f32,
    pub ataque: f32,
}

/// As três tabelas, prontas para consulta.
#[derive(Debug, Clone)]
pub struct TabelaDeProgressao {
    /// Índice = nível atual; valor = experiência para passar ao seguinte.
    exp_por_nivel: Vec<i64>,
    ajuste: Vec<AjusteDeNivel>,
    perda_na_morte: Vec<f32>,
    /// Veio do `elements.data` (e não do padrão do construtor).
    pub do_arquivo: bool,
}

impl Default for TabelaDeProgressao {
    fn default() -> Self {
        Self {
            exp_por_nivel: (0..=NIVEL_MAXIMO_DO_JOGO as i64).map(|i| i * i * 500).collect(),
            ajuste: vec![AjusteDeNivel::default(); MAX_LEVEL_DIFF + 1],
            perda_na_morte: vec![0.05; 256],
            do_arquivo: false,
        }
    }
}

fn i(reg: &Record, campo: &str) -> i32 {
    match reg.get(campo) {
        Some(FieldValue::Int(v)) => *v,
        Some(FieldValue::Float(v)) => *v as i32,
        _ => 0,
    }
}

fn f(reg: &Record, campo: &str) -> f32 {
    match reg.get(campo) {
        Some(FieldValue::Float(v)) => *v,
        Some(FieldValue::Int(v)) => *v as f32,
        _ => 0.0,
    }
}

impl TabelaDeProgressao {
    /// Monta as tabelas como `__LoadDataFromDataMan`. Tabela ausente fica no padrão.
    pub fn carregar(elements: &GenericElementsData) -> Self {
        let mut t = Self::default();

        if let Some(curva) = elements.get("PLAYER_LEVELEXP_CONFIG").iter().find(|r| i(r, "ID") == 202) {
            for nivel in 1..=NIVEL_MAXIMO_DO_JOGO as usize {
                let v = i(curva, &format!("exp_{nivel}"));
                if v != 0 {
                    t.exp_por_nivel[nivel] = v as i64;
                }
            }
            t.do_arquivo = true;
        }

        // `playertemplate.cpp:315-337`, fielmente: `j` desce de `MAX_LEVEL_DIFF`, e cada
        // degrau vale para as diferenças `>= level_diff`.
        for adj in elements.get("PARAM_ADJUST_CONFIG") {
            let mut nivel = 100;
            let mut j: i64 = MAX_LEVEL_DIFF as i64;
            let mut lad = AjusteDeNivel::default();
            for k in 1..=16 {
                let p = format!("level_diff_adjust_{k}_");
                let diff = i(adj, &format!("{p}level_diff"));
                if diff == 0 {
                    break;
                }
                if diff >= nivel {
                    // O original recusa a tabela inteira ("ordem errada").
                    break;
                }
                nivel = diff;
                lad = AjusteDeNivel {
                    exp: f(adj, &format!("{p}adjust_exp")),
                    sp: f(adj, &format!("{p}adjust_sp")),
                    dinheiro: f(adj, &format!("{p}adjust_money")),
                    item: f(adj, &format!("{p}adjust_matter")),
                    ataque: {
                        let a = f(adj, &format!("{p}adjust_attack"));
                        if !(0.0..=1.0).contains(&a) { 0.0 } else { a }
                    },
                };
                while j >= (nivel - BASE_LEVEL_DIFF) as i64 && j >= 0 {
                    t.ajuste[j as usize] = lad;
                    j -= 1;
                }
            }
            while j >= 0 {
                t.ajuste[j as usize] = lad;
                j -= 1;
            }
            t.do_arquivo = true;
        }

        if let Some(cfg) = elements.get("PLAYER_SECONDLEVEL_CONFIG").first() {
            for k in 0..256 {
                let v = f(cfg, &format!("exp_lost_{}", k + 1));
                if v != 0.0 {
                    t.perda_na_morte[k] = v;
                }
            }
        }

        t
    }

    /// A tabela padrão com o mesmo ajuste para qualquer diferença de nível. Para testes: o
    /// padrão do construtor zera o ajuste, e sem ele nenhum abate dá experiência.
    pub fn com_ajuste_uniforme(a: AjusteDeNivel) -> Self {
        let mut t = Self::default();
        t.ajuste = vec![a; MAX_LEVEL_DIFF + 1];
        t
    }

    /// `player_template::GetLvlupExp` (`playertemplate.cpp:644-651`).
    pub fn exp_para_subir(&self, nivel: i32) -> i64 {
        if (0..=NIVEL_MAXIMO_DO_JOGO).contains(&nivel) {
            self.exp_por_nivel[nivel as usize]
        } else {
            nivel as i64 * nivel as i64 * 500
        }
    }

    /// O degrau para `nível do jogador − nível do monstro`, com os mesmos cortes de
    /// `GetExpPunishment` / `GetDropPunishment` (`playertemplate.h:495-535`).
    pub fn ajuste(&self, jogador_menos_monstro: i32) -> AjusteDeNivel {
        if jogador_menos_monstro < BASE_LEVEL_DIFF {
            return self.ajuste[0];
        }
        let idx = (jogador_menos_monstro - BASE_LEVEL_DIFF) as usize;
        if idx >= MAX_LEVEL_DIFF {
            return self.ajuste[MAX_LEVEL_DIFF - 1];
        }
        self.ajuste[idx]
    }

    /// `GetResurrectExpReduce(sec_level)` — fração da experiência do nível perdida ao
    /// renascer (`playertemplate.cpp:714-718`). O índice é o nível de cultivo.
    pub fn perda_na_morte(&self, cultivo: i32) -> f32 {
        self.perda_na_morte[(cultivo & 0xFF) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curva_padrao_e_nivel_ao_quadrado_vezes_500() {
        let t = TabelaDeProgressao::default();
        assert_eq!(t.exp_para_subir(1), 500);
        assert_eq!(t.exp_para_subir(10), 50_000);
    }

    #[test]
    fn o_ajuste_corta_nas_pontas_como_o_original() {
        let mut t = TabelaDeProgressao::default();
        t.ajuste[0].exp = 0.01;
        t.ajuste[MAX_LEVEL_DIFF - 1].exp = 0.99;
        t.ajuste[100].exp = 1.0;
        assert_eq!(t.ajuste(-500).exp, 0.01);
        assert_eq!(t.ajuste(0).exp, 1.0);
        assert_eq!(t.ajuste(500).exp, 0.99);
    }
}
