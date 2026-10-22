//! Cartas de General: a caixa que sorteia a carta (`POKER_DICE_ESSENCE`) e a carta
//! (`POKER_ESSENCE`, com o tipo do `POKER_SUB_TYPE`).
//!
//! Servidor original:
//! - abrir a caixa: `generalcard_dice_item::OnUse` (`gs/item/item_generalcard_dice.cpp:10-54`)
//!   — bolsa cheia recusa; `RandSelect` sobre as 256 entradas `list[].probability`; a carta
//!   sorteada tem de ser um `POKER_ESSENCE`; gera a carta e a põe no primeiro slot vazio;
//! - a carta gerada: `generate_poker` (`gs/template/generate_item_temp.h:3144-3191`), cujo
//!   conteúdo é o `generalcard_essence` (`gs/item/item_generalcard.h:9-19`) — oito `int`.
//!
//! B346: os atributos da carta vestida (`generalcard_item::UpdateEssence`/`OnActivate`,
//! `gs/item/item_generalcard.cpp`), a curva de nível (`POKER_LEVELEXP_CONFIG`,
//! `player_template::__GetGeneralCardLvlupExp`, `playertemplate.cpp:823-829`), o ajuste por classe
//! (`CHARRACTER_CLASS_CONFIG.poker_adjust[6]`, `:303-307`) e o devorar (`InsertExp`, `GetSwallowExp`).
//! **Não** portados: conjuntos de cartas (`generalcard_set_man`), os efeitos extras (`set_addon_manager`),
//! o `IsSecActive`, o renascimento e a coleção.
use crate::generic_elements::{FieldValue, GenericElementsData};
use std::collections::HashMap;

/// Entradas de `POKER_DICE_ESSENCE::list` (`gs/template/exptypes.h:629-633`).
const ENTRADAS_DA_CAIXA: usize = 256;

/// Uma caixa: (id da carta, probabilidade), na ordem do arquivo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CaixaDeCartas {
    pub cartas: Vec<(u32, f32)>,
}

impl CaixaDeCartas {
    /// `abase::RandSelect(&list[0].probability, sizeof(list[0]), 256)` com o número `sorteio`
    /// em `[0, 1)` (`itemdataman.h:33-47`): a primeira entrada cuja probabilidade acumulada
    /// passa do número; sem nenhuma, a entrada 0 (o `ASSERT(false); return 0` do original).
    pub fn sortear(&self, sorteio: f32) -> Option<u32> {
        let mut op = sorteio;
        for (id, p) in &self.cartas {
            if op < *p {
                return (*id > 0).then_some(*id);
            }
            op -= p;
        }
        self.cartas.first().map(|c| c.0).filter(|id| *id > 0)
    }
}

/// O que o `generate_poker` tira do `elements.data` para escrever a carta.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CartaDeGeneral {
    /// `POKER_SUB_TYPE::type` do `id_sub_type` — o slot da carta.
    pub tipo: i32,
    /// `rank` — qualidade (C, B, A, S, S+).
    pub qualidade: i32,
    pub nivel_exigido: i32,
    /// `require_control_point[0..2]` — a faixa da liderança exigida, sorteada na geração.
    pub lideranca: (i32, i32),
    pub nivel_maximo: i32,
    /// B346 — `hp`, `damage`, `magic_damage`, `defence`, `magic_defence[5]`, `vigour` e os `inc_*` por nível.
    pub base: AtributosDaCarta,
    pub por_nivel: AtributosDaCarta,
    /// `swallow_exp` — a experiência que a carta dá ao ser devorada.
    pub exp_ao_devorar: i32,
}

/// Os atributos de uma carta (`generalcard_extend`, `gs/item/item_generalcard.h:29-39`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AtributosDaCarta {
    pub vida: i32,
    pub dano: i32,
    pub dano_magico: i32,
    pub defesa: i32,
    pub resistencias: [i32; 5],
    pub vigor: i32,
}

/// O `generalcard_essence` (8 `int`) gravado no item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EssenciaDaCarta {
    pub tipo: i32,
    pub qualidade: i32,
    pub nivel_exigido: i32,
    pub lideranca_exigida: i32,
    pub nivel_maximo: i32,
    pub nivel: i32,
    pub exp: i32,
    pub renascimentos: i32,
}

impl EssenciaDaCarta {
    pub fn ler(b: &[u8]) -> Option<Self> {
        if b.len() < 32 {
            return None;
        }
        let v = |k: usize| i32::from_le_bytes([b[4 * k], b[4 * k + 1], b[4 * k + 2], b[4 * k + 3]]);
        Some(Self {
            tipo: v(0),
            qualidade: v(1),
            nivel_exigido: v(2),
            lideranca_exigida: v(3),
            nivel_maximo: v(4),
            nivel: v(5),
            exp: v(6),
            renascimentos: v(7),
        })
    }

    /// Os 32 bytes de volta (o resto do bloco, se houver, fica com quem chama).
    pub fn octetos(&self) -> Vec<u8> {
        [self.tipo, self.qualidade, self.nivel_exigido, self.lideranca_exigida, self.nivel_maximo, self.nivel, self.exp, self.renascimentos]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

/// `GENERALCARD_MAX_REBIRTH_TIMES` = 2 e o `__rebirth_enhance` (`item_generalcard.cpp:21-26`).
const REFORCO_DO_RENASCIMENTO: [f32; 3] = [0.0, 0.3, 0.6];
/// `rank_ratio[5]` (`item_generalcard.cpp:19`): a carta fora do lugar do tipo dela vale isto do extra da coluna.
const RAZAO_DA_QUALIDADE: [f32; 5] = [0.0, 0.65, 1.0, 1.35, 1.7];
/// `genaralcard_column_extend[6][12]` (`item_generalcard.cpp:11-18`): resistências[5], vida, dano baixo/alto,
/// mágico baixo/alto, defesa, vigor — o que uma carta dá no lugar errado.
const EXTRA_DA_COLUNA: [[i32; 12]; 6] = [
    [0, 0, 0, 0, 0, 0, 200, 200, 200, 200, 0, 0],
    [0, 0, 0, 0, 0, 0, 200, 200, 200, 200, 0, 0],
    [0, 0, 0, 0, 0, 330, 0, 0, 0, 0, 700, 0],
    [700, 700, 700, 700, 700, 330, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50],
];

/// O que uma carta vestida soma ao jogador (`GeneralCardEnhance`, `gs/actobject.h:1331-1345`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReforcoDaCarta {
    pub vida: i32,
    pub dano_baixo: i32,
    pub dano_alto: i32,
    pub magico_baixo: i32,
    pub magico_alto: i32,
    pub defesa: i32,
    pub resistencias: [i32; 5],
    pub vigor: i32,
}

impl CartaDeGeneral {
    /// `UpdateEssence` (`item_generalcard.cpp:50-66`): `base + inc × (nível − 1)`.
    pub fn atributos_no_nivel(&self, nivel: i32) -> AtributosDaCarta {
        let n = nivel - 1;
        let b = &self.base;
        let i = &self.por_nivel;
        let mut r = AtributosDaCarta {
            vida: b.vida + i.vida * n,
            dano: b.dano + i.dano * n,
            dano_magico: b.dano_magico + i.dano_magico * n,
            defesa: b.defesa + i.defesa * n,
            resistencias: [0; 5],
            vigor: b.vigor + i.vigor * n,
        };
        for k in 0..5 {
            r.resistencias[k] = b.resistencias[k] + i.resistencias[k] * n;
        }
        r
    }

    /// `generalcard_item::OnActivate` (`item_generalcard.cpp:76-140`) na coluna `coluna` (slot − 32), com o ajuste
    /// `poker_adjust[coluna]` da classe: no lugar do tipo da carta, os atributos dela; fora, o extra da coluna × a
    /// razão da qualidade. Cada valor `(int)(x × ajuste + 0,5)`.
    pub fn reforco(&self, ess: &EssenciaDaCarta, coluna: usize, ajuste_da_classe: f32) -> ReforcoDaCarta {
        let mut ajuste = ajuste_da_classe;
        if ess.renascimentos > 0 {
            ajuste *= 1.0 + REFORCO_DO_RENASCIMENTO[(ess.renascimentos as usize).min(2)];
        }
        let arred = |v: i32, a: f32| (v as f32 * a + 0.5) as i32;
        let mut r = ReforcoDaCarta::default();
        if coluna as i32 == ess.tipo {
            if ajuste > 1e-5 {
                let x = self.atributos_no_nivel(ess.nivel);
                r = ReforcoDaCarta {
                    vida: arred(x.vida, ajuste),
                    dano_baixo: arred(x.dano, ajuste),
                    dano_alto: arred(x.dano, ajuste),
                    magico_baixo: arred(x.dano_magico, ajuste),
                    magico_alto: arred(x.dano_magico, ajuste),
                    defesa: arred(x.defesa, ajuste),
                    resistencias: x.resistencias.map(|v| arred(v, ajuste)),
                    vigor: arred(x.vigor, ajuste),
                };
            }
        } else if coluna < 6 {
            ajuste *= RAZAO_DA_QUALIDADE.get(ess.qualidade.max(0) as usize).copied().unwrap_or(0.0);
            if ajuste > 1e-5 {
                let c = &EXTRA_DA_COLUNA[coluna];
                r = ReforcoDaCarta {
                    resistencias: [c[0], c[1], c[2], c[3], c[4]].map(|v| arred(v, ajuste)),
                    vida: arred(c[5], ajuste),
                    dano_baixo: arred(c[6], ajuste),
                    dano_alto: arred(c[7], ajuste),
                    magico_baixo: arred(c[8], ajuste),
                    magico_alto: arred(c[9], ajuste),
                    defesa: arred(c[10], ajuste),
                    vigor: arred(c[11], ajuste),
                };
            }
        }
        r
    }
}

impl CartaDeGeneral {
    /// O `generalcard_essence` de uma carta nova (`generate_item_temp.h:3180-3188`): tipo,
    /// qualidade, nível exigido, a liderança sorteada, nível máximo, **nível 1**, experiência
    /// 0 e renascimentos 0 — 32 bytes.
    pub fn octetos(&self, lideranca: i32) -> Vec<u8> {
        [self.tipo, self.qualidade, self.nivel_exigido, lideranca, self.nivel_maximo, 1, 0, 0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CartasDeGeneral {
    pub caixas: HashMap<u32, CaixaDeCartas>,
    pub cartas: HashMap<u32, CartaDeGeneral>,
    /// B346 — `swallow_exp` de cada caixa (`POKER_DICE_ESSENCE`).
    pub exp_da_caixa: HashMap<u32, i32>,
    /// `_generalcard_exp_list`: o índice 0 é o marcador do original, depois `exp_1..100`.
    pub curva: Vec<i32>,
    /// `_generalcard_exp_rank_adjust_list` (`exp_adjust_1..5`).
    pub ajuste_da_qualidade: Vec<f32>,
    /// `poker_adjust[6]` por classe (`character_class_id` → os seis lugares).
    pub ajuste_da_classe: HashMap<i32, [f32; 6]>,
}

impl CartasDeGeneral {
    /// `__GetGeneralCardLvlupExp(rank, lvl)` (`playertemplate.cpp:823-829`): `(int)(exp[lvl] × adjust[rank] + 0,5)`;
    /// fora das tabelas, `INT_MAX`.
    pub fn exp_para_subir(&self, qualidade: i32, nivel: i32) -> i32 {
        let (Some(a), Some(e)) = (self.ajuste_da_qualidade.get(qualidade.max(0) as usize), self.curva.get(nivel.max(0) as usize)) else {
            return i32::MAX;
        };
        if qualidade < 0 || nivel < 0 {
            return i32::MAX;
        }
        (*e as f32 * a + 0.5) as i32
    }

    /// `__GetGeneralCardClsAdjust(cls, coluna)`: 0 fora da tabela.
    pub fn ajuste(&self, classe: i32, coluna: usize) -> f32 {
        self.ajuste_da_classe.get(&classe).and_then(|a| a.get(coluna)).copied().unwrap_or(0.0)
    }

    /// `generalcard_item::InsertExp` (`item_generalcard.cpp:250-275`): recusa no nível máximo ou se a soma não
    /// cresce; sobe enquanto couber; no máximo, a experiência zera.
    pub fn inserir_exp(&self, ess: &mut EssenciaDaCarta, exp: i32, so_conferir: bool) -> bool {
        if ess.nivel >= ess.nivel_maximo {
            return false;
        }
        let soma = ess.exp.wrapping_add(exp);
        if soma <= ess.exp {
            return false;
        }
        if so_conferir {
            return true;
        }
        ess.exp = soma;
        loop {
            let precisa = self.exp_para_subir(ess.qualidade, ess.nivel);
            if ess.exp < precisa {
                break;
            }
            ess.exp -= precisa;
            ess.nivel += 1;
            if ess.nivel >= ess.nivel_maximo {
                ess.exp = 0;
                break;
            }
        }
        true
    }

    /// `GetSwallowExp` (`item_generalcard.cpp:277-293`): o que a carta acumulou — os níveis já subidos, mais os
    /// renascimentos (cada um vale a curva inteira até o máximo) e a experiência atual.
    pub fn exp_acumulada(&self, ess: &EssenciaDaCarta) -> i32 {
        let mut exp = 0i32;
        if ess.renascimentos > 0 {
            let inteira: i32 = (1..ess.nivel_maximo).map(|n| self.exp_para_subir(ess.qualidade, n)).fold(0i32, |a, b| a.wrapping_add(b));
            exp = exp.wrapping_add(inteira.wrapping_mul(ess.renascimentos));
        }
        for n in 1..ess.nivel {
            exp = exp.wrapping_add(self.exp_para_subir(ess.qualidade, n));
        }
        exp.wrapping_add(ess.exp)
    }
}

pub fn carregar(elements: &GenericElementsData) -> CartasDeGeneral {
    let i = |r: &HashMap<String, FieldValue>, n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
    let f = |r: &HashMap<String, FieldValue>, n: &str| match r.get(n) {
        Some(FieldValue::Float(v)) => *v,
        _ => 0.0,
    };
    let tipos: HashMap<i32, i32> = elements.get("POKER_SUB_TYPE").iter().map(|r| (i(r, "ID"), i(r, "type"))).collect();
    let exp_da_caixa = elements.get("POKER_DICE_ESSENCE").iter().map(|r| (i(r, "ID") as u32, i(r, "swallow_exp"))).collect();
    let mut curva = Vec::new();
    let mut ajuste_da_qualidade = Vec::new();
    // `POKER_LEVELEXP_CONFIG`: o original empilha cada registro (`playertemplate.cpp:439-453`); há um só.
    for r in elements.get("POKER_LEVELEXP_CONFIG") {
        curva.push(0);
        curva.extend((1..=100).map(|k| i(r, &format!("exp_{k}"))));
        ajuste_da_qualidade.extend((1..=5).map(|k| f(r, &format!("exp_adjust_{k}"))));
    }
    let ajuste_da_classe = elements
        .get("CHARRACTER_CLASS_CONFIG")
        .iter()
        .map(|r| (i(r, "character_class_id"), [1, 2, 3, 4, 5, 6].map(|k| f(r, &format!("poker_adjust_{k}")))))
        .collect();
    let caixas = elements
        .get("POKER_DICE_ESSENCE")
        .iter()
        .filter(|r| i(r, "ID") > 0)
        .map(|r| {
            let cartas = (1..=ENTRADAS_DA_CAIXA)
                .map(|k| (i(r, &format!("list_{k}_id")).max(0) as u32, f(r, &format!("list_{k}_probability"))))
                .collect();
            (i(r, "ID") as u32, CaixaDeCartas { cartas })
        })
        .collect();
    let cartas = elements
        .get("POKER_ESSENCE")
        .iter()
        .filter(|r| i(r, "ID") > 0)
        .filter_map(|r| {
            // `generate_poker` recusa a carta cujo subtipo não existe (`:3148-3149`).
            let tipo = *tipos.get(&i(r, "id_sub_type"))?;
            Some((
                i(r, "ID") as u32,
                CartaDeGeneral {
                    tipo,
                    qualidade: i(r, "rank"),
                    nivel_exigido: i(r, "require_level"),
                    lideranca: (i(r, "require_control_point_1"), i(r, "require_control_point_2")),
                    nivel_maximo: i(r, "max_level"),
                    base: AtributosDaCarta {
                        vida: i(r, "hp"),
                        dano: i(r, "damage"),
                        dano_magico: i(r, "magic_damage"),
                        defesa: i(r, "defence"),
                        resistencias: [1, 2, 3, 4, 5].map(|k| i(r, &format!("magic_defence_{k}"))),
                        vigor: i(r, "vigour"),
                    },
                    por_nivel: AtributosDaCarta {
                        vida: i(r, "inc_hp"),
                        dano: i(r, "inc_damage"),
                        dano_magico: i(r, "inc_magic_damage"),
                        defesa: i(r, "inc_defence"),
                        resistencias: [1, 2, 3, 4, 5].map(|k| i(r, &format!("inc_magic_defence_{k}"))),
                        vigor: i(r, "inc_vigour"),
                    },
                    exp_ao_devorar: i(r, "swallow_exp"),
                },
            ))
        })
        .collect();
    CartasDeGeneral { caixas, cartas, exp_da_caixa, curva, ajuste_da_qualidade, ajuste_da_classe }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_sorteio_acumula_as_probabilidades_como_o_rand_select() {
        let c = CaixaDeCartas { cartas: vec![(10, 0.25), (20, 0.5), (30, 0.25)] };
        assert_eq!(c.sortear(0.0), Some(10));
        assert_eq!(c.sortear(0.3), Some(20));
        assert_eq!(c.sortear(0.8), Some(30));
        // Soma que não passa do número: a entrada 0, como o original.
        assert_eq!(c.sortear(1.5), Some(10));
    }

    #[test]
    fn a_carta_nova_tem_oito_int_com_nivel_1() {
        let c = CartaDeGeneral { tipo: 2, qualidade: 1, nivel_exigido: 10, lideranca: (5, 9), nivel_maximo: 50, ..Default::default() };
        let o = c.octetos(7);
        assert_eq!(o.len(), 32);
        let v: Vec<i32> = o.chunks(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
        assert_eq!(v, vec![2, 1, 10, 7, 50, 1, 0, 0]);
    }

    fn carta() -> CartaDeGeneral {
        CartaDeGeneral {
            tipo: 2,
            qualidade: 2,
            nivel_maximo: 10,
            base: AtributosDaCarta { vida: 100, dano: 10, dano_magico: 20, defesa: 30, resistencias: [1, 2, 3, 4, 5], vigor: 7 },
            por_nivel: AtributosDaCarta { vida: 10, dano: 1, dano_magico: 2, defesa: 3, resistencias: [1, 1, 1, 1, 1], vigor: 1 },
            ..Default::default()
        }
    }

    /// B346 — `UpdateEssence` + `OnActivate`: no lugar do tipo, `(int)((base + inc × (nível − 1)) × ajuste + 0,5)`;
    /// renascida, o ajuste × 1,3; fora do lugar, o extra da coluna × a razão da qualidade.
    #[test]
    fn reforco_no_lugar_fora_dele_e_renascida() {
        let c = carta();
        let ess = EssenciaDaCarta { tipo: 2, qualidade: 2, nivel: 3, ..Default::default() };
        // Nível 3: vida 120, dano 12, mágico 24, defesa 36, resistências 3..7, vigor 9; ajuste 0,5.
        let r = c.reforco(&ess, 2, 0.5);
        assert_eq!((r.vida, r.dano_baixo, r.magico_alto, r.defesa, r.vigor), (60, 6, 12, 18, 5));
        assert_eq!(r.resistencias, [2, 2, 3, 3, 4], "(int)(3·0,5+0,5)=2, (int)(4·0,5+0,5)=2, 5→3, 6→3, 7→4");
        let renascida = EssenciaDaCarta { renascimentos: 1, ..ess };
        assert_eq!(c.reforco(&renascida, 2, 1.0).vida, (120.0f32 * 1.3 + 0.5) as i32);
        // Coluna 3 (resistências 700, vida 330), qualidade 2 → razão 1,0, ajuste 1.
        let fora = c.reforco(&ess, 3, 1.0);
        assert_eq!((fora.vida, fora.resistencias[0], fora.dano_baixo), (330, 700, 0));
        assert_eq!(c.reforco(&ess, 2, 0.0), ReforcoDaCarta::default(), "ajuste 0 não soma");
    }

    /// B346 — `InsertExp` sobe enquanto couber e zera no máximo; `GetSwallowExp` soma o caminho feito.
    #[test]
    fn experiencia_e_devorar() {
        let mut t = CartasDeGeneral::default();
        t.curva = vec![0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
        t.ajuste_da_qualidade = vec![1.0, 1.0, 1.5];
        assert_eq!(t.exp_para_subir(2, 1), 150);
        assert_eq!(t.exp_para_subir(9, 1), i32::MAX, "fora da tabela");
        let mut e = EssenciaDaCarta { qualidade: 2, nivel: 1, nivel_maximo: 3, ..Default::default() };
        assert!(t.inserir_exp(&mut e, 160, false));
        assert_eq!((e.nivel, e.exp), (2, 10));
        assert_eq!(t.exp_acumulada(&e), 150 + 10);
        assert!(t.inserir_exp(&mut e, 10_000, false));
        assert_eq!((e.nivel, e.exp), (3, 0), "no máximo a experiência zera");
        assert!(!t.inserir_exp(&mut e, 1, true), "no máximo recusa");
        let renascida = EssenciaDaCarta { qualidade: 2, nivel: 1, nivel_maximo: 3, renascimentos: 2, exp: 5, ..Default::default() };
        assert_eq!(t.exp_acumulada(&renascida), (150 + 300) * 2 + 5);
    }
}

