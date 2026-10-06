//! Rates do realm (E7, B176): EXP, SP, drop e dinheiro, lidas de
//! `realms.double_{exp,sp,drop,gold}_multiplier` e trocadas pelo painel em tempo real.
//!
//! O original tem `world_param` (`cgame/gs/worldmanager.h:95`):
//! * `double_exp` liga um fator (`GetDoubleExpFactor`) aplicado à EXP **e** ao SP do
//!   abate em `IncExp` (`player.cpp:2906-2922`, `2831-2840`). A EXP de missão
//!   (`ReceiveTaskExp`) não passa por ele. Aqui EXP e SP têm fatores **independentes**
//!   (decisão do Murillo, 2026-10-05); a truncagem é a do original, `(int)(exp*fator)`.
//! * `double_drop` repete os **sorteios** de item do monstro (`item_more_times`,
//!   `npc.cpp:2663-2685`) e `double_money` multiplica os sorteios de dinheiro
//!   (`money_more_times`, `npc.cpp:2691-2696`). O original só conhece ×2; um fator com
//!   fração vira sorteios inteiros mais a chance da fração para um a mais (1,5 = 1
//!   garantido + 50% de outro — decisão do Murillo, 2026-10-05).
use rand::Rng;
use serde::Serialize;

/// Limites: as colunas são `NUMERIC(3,1)` (`specs/01_DATABASE_SCHEMA_POSTGRES.sql:73`),
/// então de 0,1 a 99,9, em passos de 0,1. O mínimo acima de zero é política do painel.
pub const TAXA_MINIMA: f64 = 0.1;
pub const TAXA_MAXIMA: f64 = 99.9;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Taxas {
    pub exp: f64,
    pub sp: f64,
    pub drop: f64,
    pub moedas: f64,
}

impl Default for Taxas {
    fn default() -> Self {
        Self { exp: 1.0, sp: 1.0, drop: 1.0, moedas: 1.0 }
    }
}

impl Taxas {
    /// Arredonda para uma casa, como o banco guarda: o que vale em memória é o que fica salvo.
    pub fn arredondadas(self) -> Self {
        let r = |x: f64| (x * 10.0).round() / 10.0;
        Self { exp: r(self.exp), sp: r(self.sp), drop: r(self.drop), moedas: r(self.moedas) }
    }

    pub fn validas(&self) -> bool {
        [self.exp, self.sp, self.drop, self.moedas]
            .iter()
            .all(|t| t.is_finite() && (TAXA_MINIMA..=TAXA_MAXIMA).contains(t))
    }

    /// EXP e SP do abate depois das rates, truncados como `IncExp` (`player.cpp:2835-2836`).
    pub fn aplicar_exp(&self, exp: i64, sp: i64) -> (i64, i64) {
        let escala = |v: i64, f: f64| ((v as f64) * f).clamp(0.0, i32::MAX as f64) as i64;
        (escala(exp, self.exp), escala(sp, self.sp))
    }

    /// Quantos sorteios fazer para um fator: a parte inteira, mais um com a chance da fração.
    pub fn sorteios<R: Rng>(fator: f64, rng: &mut R) -> u32 {
        let inteiro = fator.floor().max(0.0);
        let fracao = fator - inteiro;
        inteiro as u32 + u32::from(fracao > 0.0 && rng.gen::<f64>() < fracao)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn exp_e_sp_independentes_e_truncados() {
        let t = Taxas { exp: 2.5, sp: 1.0, ..Taxas::default() };
        assert_eq!(t.aplicar_exp(101, 33), (252, 33));
        assert_eq!(Taxas::default().aplicar_exp(7, 3), (7, 3));
    }

    #[test]
    fn fator_inteiro_e_exato_e_fracao_vira_chance() {
        let mut rng = StdRng::seed_from_u64(7);
        assert!((0..100).all(|_| Taxas::sorteios(2.0, &mut rng) == 2));
        assert!((0..100).all(|_| Taxas::sorteios(1.0, &mut rng) == 1));
        let total: u32 = (0..10_000).map(|_| Taxas::sorteios(1.5, &mut rng)).sum();
        // 1 garantido + 50% de outro: média 1,5 (margem larga para não ser frágil).
        assert!((14_500..=15_500).contains(&total), "{total}");
        assert!((0..1000).all(|_| (1..=2).contains(&Taxas::sorteios(1.5, &mut rng))));
    }

    #[test]
    fn limites_do_painel() {
        assert!(Taxas::default().validas());
        assert!(!Taxas { exp: 0.0, ..Taxas::default() }.validas());
        assert!(!Taxas { drop: f64::NAN, ..Taxas::default() }.validas());
        assert!(!Taxas { moedas: 100.0, ..Taxas::default() }.validas());
        assert!(Taxas { moedas: 99.9, ..Taxas::default() }.validas());
        assert_eq!(Taxas { exp: 1.25, sp: 2.04, ..Taxas::default() }.arredondadas().exp, 1.3);
    }
}
