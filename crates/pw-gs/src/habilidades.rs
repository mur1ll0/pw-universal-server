//! O efeito das habilidades: quanto uma delas cura ou machuca.
//!
//! # De onde vêm os números
//!
//! Do próprio cliente. Cada habilidade tem um stub gerado em
//! `F:\PW\1.5.5\EvolvedPWClient\ElementSkill\skillNNN.h`, e é lá que o motor original
//! escreve a conta, num `Calculate` por fase da conjuração:
//!
//! ```cpp
//! // skill125.h, a Pluma Espiritual do Sacerdote
//! skill->SetPlus  (4.5 * L * L + 90 * L + 29.6);
//! skill->SetRatio (0.5 + 0.05 * L);
//! skill->SetDamage(skill->GetMagicattack ());
//! ```
//!
//! O que dá `dano = base × ratio + plus`. A `base` é o ataque físico quando o stub usa
//! `SetDamage(GetAttack())`, e o ataque mágico quando usa um dos `Set<elemento>damage`
//! (`SetFiredamage`, `SetWooddamage`, `SetGolddamage`…, todos alimentados por
//! `GetMagicattack()`); o elemento em si vem do campo `attr` do stub.
//!
//! Cura é outro caminho, no `StateAttack`:
//!
//! ```cpp
//! // skill113.h, a Prece da Clareza
//! skill->GetVictim ()->SetValue (skill->GetMagicdamage () * 4 * L / 100 - 35 + 70 * L);
//! skill->GetVictim ()->SetHeal (1);
//! ```
//!
//! # O que esta tabela **não** é
//!
//! Não é o motor de habilidades. É a conta de **dezesseis** habilidades — as iniciais das
//! doze classes, mais as duas curas e o que elas precisam — portadas uma a uma, lendo cada
//! stub. As outras 3.301 do catálogo não estão aqui, e a resposta para elas é
//! [`Habilidade::conhecida`] devolver `None`, não um número inventado.
//!
//! Também não há estado: nada de veneno, lentidão, atordoamento ou buff com duração. As
//! habilidades desta tabela que têm efeito de estado no original (a 1126 tem lentidão, a
//! 1374 tem duas) aplicam aqui só a parte de dano.

/// De onde sai a base do dano.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseDeDano {
    /// `SetDamage(GetAttack())` — o ataque físico da arma.
    Fisico,
    /// `Set<elemento>damage(GetMagicattack())` — o ataque mágico.
    Magico,
}

/// O que a habilidade faz quando termina de conjurar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Efeito {
    /// `base × fator × (ratio.0 + ratio.1 × nível) + (plus.0 × nível² + plus.1 × nível + plus.2)`
    Dano {
        base: BaseDeDano,
        /// Multiplicador aplicado à base antes do `ratio`. É 1.0 quase sempre; a 1350 usa
        /// `1.1 * GetAttack()` e a 2571 depende da forma do jogador.
        fator: f32,
        ratio: (f32, f32),
        plus: (f32, f32, f32),
    },
    /// `ataque_mágico × k × nível / 100 + a × nível + b`
    Cura { k: f32, a: f32, b: f32 },
}

/// Uma habilidade cuja conta foi portada do stub do cliente.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Habilidade {
    pub id: i32,
    /// `GetMpcost` do stub: `custo_de_mp.0 + custo_de_mp.1 × nível`.
    pub custo_de_mp: (f32, f32),
    pub efeito: Efeito,
}

/// As dezesseis habilidades com conta portada.
///
/// Cada linha traz, no comentário, o stub de onde ela saiu — para conferir sem sair do
/// arquivo.
const TABELA: &[Habilidade] = &[
    // ---- Guerreiro ----------------------------------------------------------------
    Habilidade {
        id: 1, // 流水诀
        custo_de_mp: (-5.0, 7.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.0,
            ratio: (0.0, 0.0),
            plus: (1.9, 64.0, 36.7),
        },
    },
    // ---- Mago ---------------------------------------------------------------------
    Habilidade {
        id: 81, // 烈火符 — attr 5 (fogo), SetFiredamage
        custo_de_mp: (-9.0, 15.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            fator: 1.0,
            ratio: (0.5, 0.05),
            plus: (4.1, 93.8, 31.6),
        },
    },
    // ---- Bárbaro ------------------------------------------------------------------
    Habilidade {
        id: 102, // 重击
        custo_de_mp: (-3.0, 5.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.0,
            ratio: (0.0, 0.0),
            plus: (1.4, 52.3, 64.2),
        },
    },
    // ---- Sacerdote ----------------------------------------------------------------
    Habilidade {
        id: 113, // 清心咒, a Prece da Clareza
        custo_de_mp: (-20.0, 30.0),
        efeito: Efeito::Cura { k: 4.0, a: 70.0, b: -35.0 },
    },
    Habilidade {
        id: 125, // 羽箭 — attr 2 (metal)
        custo_de_mp: (-10.0, 15.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            fator: 1.0,
            ratio: (0.5, 0.05),
            plus: (4.5, 90.0, 29.6),
        },
    },
    // ---- Arqueiro -----------------------------------------------------------------
    Habilidade {
        id: 234, // 引而不发
        custo_de_mp: (0.0, 10.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.0,
            // O stub diz `(1.5 + 0.15 * L) * GetCharging() / (5200 - 200 * L)`: o dano
            // sobe com o tempo de carga. **Assumimos carga cheia**, quando a fração vale
            // 1 — o servidor não recebe o tempo de carga do cliente. Está no limite
            // superior de propósito: é o valor que o jogador vê na descrição.
            ratio: (1.5, 0.15),
            plus: (0.0, 0.0, 0.0),
        },
    },
    Habilidade {
        id: 235, // 连射
        custo_de_mp: (-5.5, 7.5),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.0,
            ratio: (0.0, 0.0),
            plus: (2.3, 63.2, 46.4),
        },
    },
    // ---- Feiticeira ---------------------------------------------------------------
    Habilidade {
        id: 299, // 剧毒蛊 — attr 3 (madeira)
        custo_de_mp: (-3.2, 7.2),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            fator: 1.0,
            ratio: (0.5, 0.05),
            plus: (2.3, 68.2, 54.0),
        },
    },
    // ---- Mercenário ---------------------------------------------------------------
    Habilidade {
        id: 1111,
        custo_de_mp: (-5.0, 7.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.0,
            ratio: (0.0, 0.0),
            plus: (1.7, 55.5, 58.8),
        },
    },
    // ---- Espiritualista -----------------------------------------------------------
    Habilidade {
        id: 1125, // attr 6 (terra)
        custo_de_mp: (-6.0, 15.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            fator: 1.0,
            ratio: (0.5, 0.05),
            plus: (6.5, 138.2, 11.6),
        },
    },
    Habilidade {
        id: 1126, // attr 4 (água) — no original também deixa lentidão, que não portamos
        custo_de_mp: (-9.0, 15.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            fator: 1.0,
            ratio: (0.5, 0.05),
            plus: (4.1, 92.6, 36.6),
        },
    },
    // ---- Arcano -------------------------------------------------------------------
    Habilidade {
        id: 1350,
        custo_de_mp: (-5.0, 7.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.1, // `SetDamage(1.1 * GetAttack())`
            ratio: (0.0, 0.09),
            plus: (1.9, 53.3, 60.8),
        },
    },
    // ---- Místico ------------------------------------------------------------------
    Habilidade {
        id: 1374, // attr 3 — no original também deixa lentidão, que não portamos
        custo_de_mp: (-8.0, 14.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            fator: 1.0,
            ratio: (0.5, 0.05),
            plus: (1.5, 51.1, 62.0),
        },
    },
    Habilidade {
        id: 1381,
        custo_de_mp: (-10.0, 25.0),
        efeito: Efeito::Cura { k: 3.0, a: 30.0, b: 30.0 },
    },
    // ---- Retalhador ---------------------------------------------------------------
    Habilidade {
        id: 2547,
        custo_de_mp: (-5.4, 9.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Fisico,
            fator: 1.0,
            // O stub não chama `SetRatio`: só o `plus` conta.
            ratio: (0.0, 0.0),
            plus: (3.7, 49.8, 61.0),
        },
    },
    // ---- Tormentador --------------------------------------------------------------
    Habilidade {
        id: 2571, // attr 2
        custo_de_mp: (-9.0, 15.0),
        efeito: Efeito::Dano {
            base: BaseDeDano::Magico,
            // `(GetForm() == 1 ? 1.2 : 0.9) * GetMagicattack()`. O servidor não tem a
            // forma do jogador, então fica a de fora de forma — o valor menor.
            fator: 0.9,
            ratio: (0.6, 0.06),
            plus: (5.3, 71.1, 38.0),
        },
    },
];

impl Habilidade {
    /// A habilidade, se a conta dela foi portada. `None` é a resposta honesta para as
    /// outras 3.301 do catálogo.
    pub fn conhecida(id: i32) -> Option<&'static Habilidade> {
        TABELA.iter().find(|h| h.id == id)
    }

    /// `GetMpcost` do stub, arredondado para cima — o cliente cobra o inteiro.
    pub fn custo_de_mp(&self, nivel: i32) -> i32 {
        let n = nivel.max(1) as f32;
        (self.custo_de_mp.0 + self.custo_de_mp.1 * n).ceil().max(0.0) as i32
    }

    /// Quanto a habilidade cura, ou `None` se ela não é de cura.
    ///
    /// O stub usa `GetMagicdamage()`, que no motor original é o dano mágico da arma —
    /// distinto de `GetMagicattack()`. **Não distinguimos os dois**: o `PlayerEntity` só
    /// tem `magic_attack_min/max`, e é a média deles que entra aqui. Onde os dois
    /// divergirem, a cura sai proporcionalmente errada; a parte fixa (`a × nível + b`),
    /// que domina nos níveis baixos, está certa.
    pub fn cura(&self, nivel: i32, ataque_magico: i32) -> Option<i32> {
        let Efeito::Cura { k, a, b } = self.efeito else {
            return None;
        };
        let n = nivel.max(1) as f32;
        let valor = ataque_magico as f32 * k * n / 100.0 + a * n + b;
        Some(valor.round().max(0.0) as i32)
    }

    /// Quanto a habilidade tira, antes da defesa do alvo, ou `None` se ela é de cura.
    pub fn dano(&self, nivel: i32, ataque_fisico: i32, ataque_magico: i32) -> Option<i32> {
        let Efeito::Dano { base, fator, ratio, plus } = self.efeito else {
            return None;
        };
        let n = nivel.max(1) as f32;
        let bruto = match base {
            BaseDeDano::Fisico => ataque_fisico,
            BaseDeDano::Magico => ataque_magico,
        } as f32
            * fator;
        let valor = bruto * (ratio.0 + ratio.1 * n) + (plus.0 * n * n + plus.1 * n + plus.2);
        // Piso de 1: o original nunca deixa um golpe que acertou fazer zero.
        Some((valor.round() as i32).max(1))
    }

    /// A habilidade é de cura?
    pub fn e_cura(&self) -> bool {
        matches!(self.efeito, Efeito::Cura { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confere a conta da Prece da Clareza contra o stub, na mão.
    ///
    /// `skill113.h`: `SetValue(GetMagicdamage() * 4 * L / 100 - 35 + 70 * L)`.
    /// Com nível 1 e 200 de ataque mágico: `200 * 4 * 1 / 100 - 35 + 70 = 8 + 35 = 43`.
    #[test]
    fn a_prece_da_clareza_cura_o_que_o_stub_manda() {
        let h = Habilidade::conhecida(113).expect("a 113 tem de estar na tabela");
        assert!(h.e_cura());
        assert_eq!(h.cura(1, 200), Some(43));
        assert_eq!(h.dano(1, 100, 200), None, "cura não faz dano");

        // Nível 2: `200 * 4 * 2 / 100 - 35 + 140 = 16 + 105 = 121`.
        assert_eq!(h.cura(2, 200), Some(121));
    }

    /// `skill125.h`: `plus = 4.5 + 90 + 29.6 = 124.1`, `ratio = 0.55`, base mágica.
    /// Com 200 de ataque mágico: `200 * 0.55 + 124.1 = 110 + 124.1 = 234.1` → 234.
    #[test]
    fn a_pluma_espiritual_bate_o_que_o_stub_manda() {
        let h = Habilidade::conhecida(125).expect("a 125 tem de estar na tabela");
        assert!(!h.e_cura());
        assert_eq!(h.dano(1, 999, 200), Some(234), "usa o ataque mágico, não o físico");
        assert_eq!(h.cura(1, 200), None, "ataque não cura");
    }

    /// O Guerreiro tem `ratio = 0`: só o `plus` conta, e o ataque físico não entra.
    /// `1.9 + 64 + 36.7 = 102.6` → 103.
    #[test]
    fn uma_habilidade_de_ratio_zero_ignora_o_ataque() {
        let h = Habilidade::conhecida(1).unwrap();
        assert_eq!(h.dano(1, 100, 100), Some(103));
        assert_eq!(h.dano(1, 100_000, 100), Some(103), "ratio 0 não olha o ataque");
    }

    /// O custo de MP é o do stub, arredondado para cima.
    /// A 125 custa `-10 + 15 * L`: 5 no nível 1, 20 no nível 2.
    #[test]
    fn o_custo_de_mp_vem_do_stub() {
        let h = Habilidade::conhecida(125).unwrap();
        assert_eq!(h.custo_de_mp(1), 5);
        assert_eq!(h.custo_de_mp(2), 20);

        // A 235 tem coeficientes quebrados (`-5.5 + 7.5 * L`): 2 no nível 1, arredondado
        // para cima como o cliente faz.
        assert_eq!(Habilidade::conhecida(235).unwrap().custo_de_mp(1), 2);
    }

    /// Uma habilidade fora da tabela não vira número inventado.
    #[test]
    fn habilidade_sem_formula_portada_e_desconhecida() {
        assert!(Habilidade::conhecida(4321).is_none());
        assert!(Habilidade::conhecida(167).is_none(), "o Portal da Cidade não tem dano");
    }

    /// As doze classes têm a habilidade inicial na tabela — senão o jogador conjura e
    /// nada acontece, que foi o defeito relatado em jogo.
    #[test]
    fn o_kit_inicial_das_doze_classes_esta_todo_aqui() {
        for id in [1, 81, 1125, 1126, 299, 102, 1111, 234, 235, 125, 113, 1350, 1374, 1381, 2547, 2571] {
            assert!(
                Habilidade::conhecida(id).is_some(),
                "a habilidade inicial {id} não tem conta portada"
            );
        }
    }

    /// Todo dano tem piso 1: o original não deixa um golpe que acertou fazer zero.
    #[test]
    fn dano_nunca_e_zero() {
        for h in TABELA.iter().filter(|h| !h.e_cura()) {
            let d = h.dano(1, 0, 0).unwrap();
            assert!(d >= 1, "a {} fez {d} com ataque zero", h.id);
        }
    }
}
