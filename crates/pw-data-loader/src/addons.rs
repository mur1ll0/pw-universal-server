//! Propriedades adicionais (addons) e o que a geração de equipamento precisa do
//! `elements.data`.
//!
//! - Quem trata cada id: `specs/addons_155/addons.json`, extraído de
//!   `gs/item/item_addon.cpp` (`INSERT_ADDON(id, tratador)`).
//! - Como cada tratador gera os argumentos e de que família é: `specs/addons_155/classificacao.json`,
//!   de `classificar_addons.py` (B209), que resolve macros, `typedef`s e heranças até
//!   `arg_addon<T>` (`item_addon.cpp:50-235`), uma classe de essência (`essence_addon`, com a
//!   família no `if(datatype != DT_xxx_ESSENCE) return -1`) ou uma classe com `GenerateParam` próprio.
//! - Os parâmetros brutos: `EQUIPMENT_ADDON` (`num_params`, `param1..3`,
//!   `gs/template/exptypes.h:122-134`). Parâmetro de porcentagem vem como os bits de um
//!   `float` guardados num `int` (`arg_addon<PERCENT>::GenerateParam`, `item_addon.cpp:103-110`).
//! - O que o drop sorteia: `WEAPON/ARMOR/DECORATION_ESSENCE` (`generate_item_temp.h:200-480`,
//!   `740-830`).

use crate::generic_elements::{FieldValue, GenericElementsData, Record};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const ADDONS_155_JSON: &str = include_str!("../../../specs/addons_155/addons.json");
const CLASSIFICACAO_JSON: &str = include_str!("../../../specs/addons_155/classificacao.json");

#[derive(Deserialize)]
struct Arquivo {
    tratadores: HashMap<String, String>,
}

/// Um tratador resolvido no fonte (`classificacao.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct ClasseDoTratador {
    /// `POINT`, `DOUBLE_POINT`, `PERCENT`, `DOUBLE_PERCENT`, `SECOND`, `DOUBLE_SECOND`,
    /// `DOUBLE_FIX_POINT`, `TRIPLE_POINT`, `ESSENCIA` ou `PROPRIO`.
    pub tipo: String,
    /// A classe com `GenerateParam` próprio (essência ou própria); `None` em `arg_addon<T>`.
    pub classe: Option<String>,
    /// `arma`, `armadura` ou `acessorio`, do `DT_xxx_ESSENCE` da classe de essência.
    pub familia_no_fonte: Option<String>,
    /// `conjunto` (`set_equip_addon`), `refino`, `pedra`, `temporario`, `float` (o `Activate` lê o
    /// argumento como `float`: `enhance_speed_addon_point`).
    pub marcas: Vec<String>,
}

#[derive(Deserialize)]
struct Classificacao {
    tratadores: HashMap<String, ClasseDoTratador>,
}

fn classificacao() -> &'static HashMap<String, ClasseDoTratador> {
    static C: OnceLock<HashMap<String, ClasseDoTratador>> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str::<Classificacao>(CLASSIFICACAO_JSON).map(|c| c.tratadores).unwrap_or_default())
}

/// Como `GenerateParam` trata os parâmetros de um addon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorteio {
    /// `arg_addon<POINT>`: 1 parâmetro, como está.
    Ponto,
    /// `arg_addon<DOUBLE_POINT>` e as essências de inteiro: `RandNormal(arg0, arg1)`, 1 parâmetro.
    EntreDois,
    /// `arg_addon<PERCENT>`: `(int)(float × 100 + 0,1)`, 1 parâmetro.
    Porcento,
    /// `arg_addon<DOUBLE_PERCENT>`: `RandNormal(p0×100, p1×100)`, 1 parâmetro.
    EntreDoisPorcento,
    /// `arg_addon<SECOND>`: `(int)(float × 20 + 0,1)`, 1 parâmetro.
    Segundo,
    /// `arg_addon<DOUBLE_SECOND>`: `RandNormal(p0×20, p1×20)`, 1 parâmetro.
    EntreDoisSegundos,
    /// `enhance_weapon_speed_addon`/`enhance_attack_speed_addon`: `(int)(float × 20)`
    /// (`item_addon_weapon.cpp:50-60`, `item_addon.cpp:768-780`), 1 parâmetro.
    VinteAvos,
    /// Um `float` sorteado entre os dois, gravado como `float` (`IA_EA_ESS_SCALE`,
    /// `item_armor_scale_enhance_resistance`: `abase::Rand`; `reduce_require_addon`,
    /// `enhance_attack_range_addon_2arg`: `RandNormal`), 1 parâmetro.
    FloatEntreDois,
    /// `arg_addon<TRIPLE_POINT>`/`item_armor_enhance_resistance_addon_2`:
    /// `RandNormal(a0 ± a2)`, `RandNormal(a1 ± a2)`, 2 parâmetros.
    TresPontos,
    /// `refine_addon_template`: o parâmetro do tratador base e o nível (0), 2 parâmetros.
    Refino,
    /// Os `n` primeiros parâmetros como estão (`return n` sem mexer: `item_skill_addon`,
    /// `IDMRA`, `IAERA3`, `item_decoration_specific_*`, `DOUBLE_FIX_POINT`…).
    ComoEstao(usize),
}

/// O que o painel pode fazer com o valor do efeito (B209).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edicao {
    /// Um inteiro somado direto (`POINT`, `DOUBLE_POINT`, `PERCENT`, `DOUBLE_PERCENT` e as
    /// essências de inteiro): qualquer valor; o padrão vem do id.
    Editavel,
    /// Codificação própria (float, argumentos de significados diferentes, habilidade,
    /// conjunto): só o que o `GenerateParam` daria.
    Fixo,
    /// Refino (vem do nível), pedra (vem do furo), temporário (precisa da data): fora da busca.
    ForaDaBusca,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DadosDoAddon {
    pub tratador: String,
    pub num_params: i32,
    pub params: [i32; 3],
    /// Arma, armadura, acessório em que o efeito pode entrar (B209, [`TabelaDeAddons::ligar_familias`]);
    /// `None` = todas.
    pub familias: Option<[bool; 3]>,
}

fn como_float(v: i32) -> f32 {
    f32::from_bits(v as u32)
}

impl DadosDoAddon {
    pub fn classe(&self) -> Option<&'static ClasseDoTratador> {
        classificacao().get(&self.tratador)
    }

    fn marca(&self, m: &str) -> bool {
        self.classe().is_some_and(|c| c.marcas.iter().any(|x| x == m))
    }

    /// O sorteio que o tratador faz; `None` para tratador desconhecido (o addon não é gerado).
    pub fn sorteio(&self) -> Option<Sorteio> {
        use Sorteio::*;
        if self.tratador.starts_with("refine_") {
            return Some(Refino);
        }
        let c = self.classe()?;
        Some(match c.tipo.as_str() {
            "POINT" => Ponto,
            "DOUBLE_POINT" => EntreDois,
            "PERCENT" => Porcento,
            "DOUBLE_PERCENT" => EntreDoisPorcento,
            "SECOND" => Segundo,
            "DOUBLE_SECOND" => EntreDoisSegundos,
            "DOUBLE_FIX_POINT" => ComoEstao(2),
            "TRIPLE_POINT" => TresPontos,
            _ => match c.classe.as_deref()? {
                "IA_EA_ESS" | "IA_ED_ESS" | "item_armor_enhance_all_resistance" | "item_armor_enhance_resistance"
                | "item_decoration_enchance_resistance" | "item_decoration_enhance_all_resistance"
                | "enhance_weapon_damage_addon" | "enhance_weapon_max_damage_addon" | "enhance_weapon_magic_addon"
                | "enhance_weapon_max_magic_addon" => EntreDois,
                "IA_EA_ESS_SCALE" | "item_armor_scale_enhance_resistance" | "reduce_require_addon"
                | "enhance_attack_range_addon_2arg" => FloatEntreDois,
                "enhance_weapon_speed_addon" | "enhance_attack_speed_addon" => VinteAvos,
                "item_armor_enhance_resistance_addon_2" => TresPontos,
                "item_armor_enhance_resistance_addon" | "item_armor_enhance_resistance_addon_3" | "item_armor_specific_addon"
                | "item_decoration_specific_damage_addon" | "item_decoration_specific_magic_damage_addon"
                | "item_decoration_magic_resistance_addon" | "item_skill_addon" | "item_skill_addon_2" => ComoEstao(2),
                "item_rebound_skill_addon" | "item_rebound_skill_addon2" => ComoEstao(3),
                "enhance_durability_addon" | "enhance_durability_addon_point" | "enhance_weapon_attack_range"
                | "item_decoration_scale_enhance_damage" | "item_decoration_scale_enhance_magic_damage"
                | "query_other_property_addon" | "enhance_mount_speed_addon" => ComoEstao(1),
                "empty_addon" => ComoEstao(self.num_params.clamp(0, 3) as usize),
                "item_addon_random" => ComoEstao(0),
                _ => return None,
            },
        })
    }

    /// `GenerateParam` com as escolhas dadas (`int` entre dois e `float` entre dois). O GS passa
    /// o sorteio; o painel, o máximo (o "valor do id").
    pub fn gerar_com(&self, int: &mut dyn FnMut(i32, i32) -> i32, flt: &mut dyn FnMut(f32, f32) -> f32) -> Option<Vec<i32>> {
        use Sorteio::*;
        let p = self.params;
        let pc = |v: i32| (como_float(v) * 100.0 + 0.1) as i32;
        let sg = |v: i32| (como_float(v) * 20.0 + 0.1) as i32;
        let mut args = match self.sorteio()? {
            Ponto => vec![p[0]],
            EntreDois => vec![int(p[0], p[1])],
            Porcento => vec![pc(p[0])],
            EntreDoisPorcento => vec![int(pc(p[0]), pc(p[1]))],
            Segundo => vec![sg(p[0])],
            EntreDoisSegundos => vec![int((como_float(p[0]) * 20.0) as i32, (como_float(p[1]) * 20.0) as i32)],
            VinteAvos => {
                let v = como_float(p[0]);
                // `enhance_attack_speed_addon`: `if(speed <= -1 || speed > 1) return -1;`
                if v <= -1.0 || v > 1.0 {
                    return None;
                }
                vec![(v * 20.0) as i32]
            }
            FloatEntreDois => vec![flt(como_float(p[0]), como_float(p[1])).to_bits() as i32],
            TresPontos => vec![int(p[0] - p[2], p[0] + p[2]), int(p[1] - p[2], p[1] + p[2])],
            Refino => vec![p[0], 0],
            ComoEstao(n) => p[..n.min(3)].to_vec(),
        };
        // `temporary_addon_template`: `data.arg[1] = 0xFFFF` (a data de expirar), 2 parâmetros
        // (`item_addon.cpp:1318-1329`).
        if self.marca("temporario") {
            args.truncate(1);
            args.push(0xFFFF);
        }
        Some(args)
    }

    /// O valor do id: o `GenerateParam` com o máximo de cada faixa.
    pub fn valor_do_id(&self) -> Option<Vec<i32>> {
        self.gerar_com(&mut |a, b| a.max(b), &mut |a: f32, b: f32| a.max(b))
    }

    fn valor_minimo(&self) -> Option<Vec<i32>> {
        self.gerar_com(&mut |a, b| a.min(b), &mut |a: f32, b: f32| a.min(b))
    }

    /// A faixa do primeiro argumento (`min`, `max`) que o `GenerateParam` sortearia.
    pub fn faixa(&self) -> Option<(i32, i32)> {
        Some((*self.valor_minimo()?.first()?, *self.valor_do_id()?.first()?))
    }

    pub fn edicao(&self) -> Edicao {
        if self.tratador.starts_with("refine_") || self.marca("refino") || self.marca("pedra") || self.marca("temporario") {
            return Edicao::ForaDaBusca;
        }
        if self.marca("conjunto") || self.marca("float") {
            return Edicao::Fixo;
        }
        let tipo = self.classe().map(|c| c.tipo.as_str()).unwrap_or("");
        match (tipo, self.sorteio()) {
            ("POINT" | "DOUBLE_POINT" | "PERCENT" | "DOUBLE_PERCENT", _) | (_, Some(Sorteio::EntreDois)) => Edicao::Editavel,
            _ => Edicao::Fixo,
        }
    }

    /// `true` quando o valor é uma porcentagem inteira (`PERCENT`, `DOUBLE_PERCENT`).
    pub fn em_porcento(&self) -> bool {
        matches!(self.sorteio(), Some(Sorteio::Porcento | Sorteio::EntreDoisPorcento))
    }

    /// Os argumentos que o painel pode gravar: editável = o número de argumentos do id, com
    /// qualquer valor; fixo = cada argumento dentro do que o `GenerateParam` daria.
    pub fn aceita(&self, args: &[i32]) -> bool {
        let (Some(min), Some(max)) = (self.valor_minimo(), self.valor_do_id()) else { return false };
        if args.len() != max.len() {
            return false;
        }
        if self.edicao() == Edicao::Editavel {
            return true;
        }
        let float = matches!(self.sorteio(), Some(Sorteio::FloatEntreDois));
        args.iter().zip(min.iter().zip(&max)).all(|(&v, (&a, &b))| {
            if float {
                let (v, a, b) = (como_float(v), como_float(a), como_float(b));
                v >= a.min(b) && v <= a.max(b)
            } else {
                v >= a.min(b) && v <= a.max(b)
            }
        })
    }

    /// O efeito pode entrar numa peça desta família.
    pub fn serve_em(&self, familia: Familia) -> bool {
        self.familias.map_or(true, |f| f[familia as usize])
    }
}

#[derive(Debug, Clone, Default)]
pub struct TabelaDeAddons {
    pub por_id: HashMap<u32, DadosDoAddon>,
    /// B209 — `set_addon_manager::LoadTemplate` (`gs/item/set_addon.cpp:5-48`): cada peça
    /// (`SUITE_ESSENCE.equipments[12]`) com a lista de efeitos do conjunto (`addons[11]`), que o
    /// `equip_item::OnActivate` ativa junto com os da peça (`_extra_addon`, `equip_item.cpp:703-735`).
    pub conjuntos: HashMap<u32, Vec<u32>>,
}

/// `set_equip_addon<N, BASE>` (`item_addon.cpp:1194-1232`): `(N, tratador base)` de um tratador
/// `SET_ADDON_MACRO(N,BASE)`.
pub fn conjunto_de(tratador: &str) -> Option<(i32, &str)> {
    let resto = tratador.strip_prefix("SET_ADDON_MACRO(")?.strip_suffix(')')?;
    let (n, base) = resto.split_once(',')?;
    Some((n.trim().parse().ok()?, base.trim()))
}

impl TabelaDeAddons {
    pub fn carregar(elements: &GenericElementsData) -> Self {
        let tratadores: HashMap<u32, String> = serde_json::from_str::<Arquivo>(ADDONS_155_JSON)
            .map(|a| a.tratadores.into_iter().filter_map(|(k, v)| Some((k.parse().ok()?, v))).collect())
            .unwrap_or_default();
        let por_id = elements
            .get("EQUIPMENT_ADDON")
            .iter()
            .filter_map(|r| {
                let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
                let id = i("ID") as u32;
                let tratador = tratadores.get(&id)?.clone();
                Some((id, DadosDoAddon { tratador, num_params: i("num_params"), params: [i("param1"), i("param2"), i("param3")], familias: None }))
            })
            .collect();
        let mut conjuntos: HashMap<u32, Vec<u32>> = HashMap::new();
        for r in elements.get("SUITE_ESSENCE") {
            let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            let lista: Vec<u32> = (1..=11).map(|k| i(&format!("addons_{k}_id"))).filter(|id| *id > 0).map(|id| id as u32).collect();
            if lista.is_empty() {
                continue;
            }
            for k in 1..=12 {
                let peca = i(&format!("equipments_{k}_id"));
                if peca > 0 {
                    conjuntos.entry(peca as u32).or_default().extend(lista.iter().copied());
                }
            }
        }
        Self { por_id, conjuntos }
    }

    /// B209 — a família de cada efeito, nesta ordem de evidência:
    /// 1. essência: a do `DT_xxx_ESSENCE` da classe (o `GenerateParam` devolve -1 nas outras);
    /// 2. as famílias cujas essências deste realm sorteiam o id (`addons`, `rands`, `uniques`);
    /// 3. a união das famílias dos outros ids do mesmo tratador neste realm;
    /// 4. sem evidência: todas.
    pub fn ligar_familias(&mut self, geracao: &TabelaDeGeracao) {
        let mut por_id: HashMap<u32, [bool; 3]> = HashMap::new();
        for m in geracao.values() {
            for (id, _) in m.addons.iter().chain(&m.addons_da_producao).chain(&m.unicos) {
                if *id > 0 {
                    por_id.entry(*id).or_default()[m.familia as usize] = true;
                }
            }
        }
        let mut por_tratador: HashMap<String, [bool; 3]> = HashMap::new();
        for (id, a) in &self.por_id {
            if let Some(f) = por_id.get(id) {
                let u = por_tratador.entry(a.tratador.clone()).or_default();
                for k in 0..3 {
                    u[k] |= f[k];
                }
            }
        }
        for (id, a) in self.por_id.iter_mut() {
            let fonte = a.classe().and_then(|c| c.familia_no_fonte.as_deref()).and_then(|f| match f {
                "arma" => Some([true, false, false]),
                "armadura" => Some([false, true, false]),
                "acessorio" => Some([false, false, true]),
                _ => None,
            });
            a.familias = fonte.or_else(|| por_id.get(id).copied()).or_else(|| por_tratador.get(&a.tratador).copied());
        }
    }
}

/// A família do equipamento gerado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Familia {
    Arma = 0,
    Armadura,
    Decoracao,
}

/// O que `generate_weapon/armor/decoration` leem do modelo.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeloDeGeracao {
    pub familia: Familia,
    pub id_sub_type: i32,
    pub fixed_props: bool,
    pub proc_type: i32,
    pub furos_no_drop: Vec<f32>,
    /// `make_probability_socket*`: os furos do item fabricado e do vendido na loja
    /// (`generate_weapon/armor`, ramo `else` de `normal_addon == ADDON_LIST_DROP`,
    /// `generate_item_temp.h:217-222`).
    pub furos_na_producao: Vec<f32>,
    pub quantos_addons: Vec<f32>,
    pub chance_de_unico: f32,
    pub addons: Vec<(u32, f32)>,
    /// `rands[32]`: a lista do `ADDON_LIST_PRODUCE` (`generate_template_addon`,
    /// `generate_item_temp.h:175-187`) — o `produce` dos argumentos é o `ess->rands`.
    pub addons_da_producao: Vec<(u32, f32)>,
    pub unicos: Vec<(u32, f32)>,
    pub durabilidade: (i32, i32),
    pub durabilidade_no_drop: (i32, i32),
    /// Arma: `damage_high_min/max`, `magic_damage_high_min/max`.
    pub dano_maximo: (i32, i32),
    pub dano_magico_maximo: (i32, i32),
    /// Armadura/acessório.
    pub defesa: (i32, i32),
    pub evasao: (i32, i32),
    pub mana: (i32, i32),
    pub vida: (i32, i32),
    pub dano: (i32, i32),
    pub dano_magico: (i32, i32),
    pub resistencias: [(i32, i32); 5],
    pub todas_as_resistencias: bool,
}

fn f(r: &Record, n: &str) -> f32 {
    match r.get(n) {
        Some(FieldValue::Float(v)) => *v,
        Some(FieldValue::Int(v)) => *v as f32,
        _ => 0.0,
    }
}

fn i(r: &Record, n: &str) -> i32 {
    r.get(n).and_then(|v| v.as_i32()).unwrap_or(0)
}

fn lista(r: &Record, prefixo: &str, sufixo_id: &str, sufixo_p: &str, n: usize) -> Vec<(u32, f32)> {
    (1..=n).map(|k| (i(r, &format!("{prefixo}_{k}_{sufixo_id}")).max(0) as u32, f(r, &format!("{prefixo}_{k}_{sufixo_p}")))).collect()
}

pub type TabelaDeGeracao = HashMap<u32, ModeloDeGeracao>;

pub fn carregar_geracao(elements: &GenericElementsData) -> TabelaDeGeracao {
    let mut t = TabelaDeGeracao::new();
    for (tabela, familia) in [("WEAPON_ESSENCE", Familia::Arma), ("ARMOR_ESSENCE", Familia::Armadura), ("DECORATION_ESSENCE", Familia::Decoracao)] {
        for r in elements.get(tabela) {
            let id = i(r, "ID");
            if id <= 0 {
                continue;
            }
            let probs = |base: &str, n: usize| (0..n).map(|k| f(r, &format!("{base}{k}"))).collect::<Vec<_>>();
            // O número de `probability_addon_num` muda por versão (v156: 6 na arma, 5 na
            // armadura e no acessório; v7: 4 nos três — `gs` 1.2.6, B151); o que falta no
            // registro vira probabilidade 0 e nunca é sorteado.
            let (furos, furos_prod, n_addons) = match familia {
                Familia::Arma => (probs("drop_probability_socket", 3), probs("make_probability_socket", 3), probs("probability_addon_num", 6)),
                Familia::Armadura => (probs("drop_probability_socket", 5), probs("make_probability_socket", 5), probs("probability_addon_num", 5)),
                Familia::Decoracao => (Vec::new(), Vec::new(), probs("probability_addon_num", 5)),
            };
            let par = |a: &str, b: &str| (i(r, a), i(r, b));
            let mut res = [(0, 0); 5];
            for (k, x) in res.iter_mut().enumerate() {
                *x = par(&format!("magic_defences_{}_low", k + 1), &format!("magic_defences_{}_high", k + 1));
            }
            t.insert(
                id as u32,
                ModeloDeGeracao {
                    familia,
                    id_sub_type: i(r, "id_sub_type"),
                    fixed_props: i(r, "fixed_props") != 0,
                    proc_type: i(r, "proc_type"),
                    furos_no_drop: furos,
                    furos_na_producao: furos_prod,
                    quantos_addons: n_addons,
                    chance_de_unico: f(r, "probability_unique"),
                    addons: lista(r, "addons", "id_addon", "probability_addon", 32),
                    addons_da_producao: lista(r, "rands", "id_rand", "probability_rand", 32),
                    unicos: if familia == Familia::Arma { lista(r, "uniques", "id_unique", "probability_unique", 16) } else { Vec::new() },
                    durabilidade: par("durability_min", "durability_max"),
                    durabilidade_no_drop: par("durability_drop_min", "durability_drop_max"),
                    dano_maximo: par("damage_high_min", "damage_high_max"),
                    dano_magico_maximo: par("magic_damage_high_min", "magic_damage_high_max"),
                    defesa: par("defence_low", "defence_high"),
                    evasao: par("armor_enhance_low", "armor_enhance_high"),
                    mana: par("mp_enhance_low", "mp_enhance_high"),
                    vida: par("hp_enhance_low", "hp_enhance_high"),
                    dano: par("damage_low", "damage_high"),
                    dano_magico: par("magic_damage_low", "magic_damage_high"),
                    resistencias: res,
                    todas_as_resistencias: i(r, "force_all_magic_defences") != 0,
                },
            );
        }
    }
    t
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_arquivo_de_tratadores_se_le() {
        let a: Arquivo = serde_json::from_str(ADDONS_155_JSON).unwrap();
        assert!(a.tratadores.len() > 2900);
        assert_eq!(a.tratadores["1497"], "refine_damage");
    }

    fn addon(id: u32, num: i32, params: [i32; 3]) -> DadosDoAddon {
        let a: Arquivo = serde_json::from_str(ADDONS_155_JSON).unwrap();
        DadosDoAddon { tratador: a.tratadores[&id.to_string()].clone(), num_params: num, params, familias: None }
    }

    /// B209: o valor do id e a edição pelo tipo de parâmetro, com os ids do teste da RT.
    #[test]
    fn valor_do_id_e_edicao_pelo_tipo_de_parametro() {
        assert!(classificacao().len() > 240, "todos os tratadores classificados");
        // 1317 `enhance_attack_addon_2` = DOUBLE_POINT 118–118.
        let acerto = addon(1317, 2, [118, 118, 0]);
        assert_eq!((acerto.valor_do_id(), acerto.edicao(), acerto.faixa()), (Some(vec![118]), Edicao::Editavel, Some((118, 118))));
        assert!(acerto.aceita(&[999]) && !acerto.aceita(&[1, 2]));
        // 286 `enhance_speed_addon` = PERCENT 0,05 → 5.
        let corrida = addon(286, 1, [0.05f32.to_bits() as i32, 0, 0]);
        assert_eq!((corrida.valor_do_id(), corrida.edicao(), corrida.em_porcento()), (Some(vec![5]), Edicao::Editavel, true));
        // 332 `reduce_cast_time_addon` = PERCENT 0,03 → 3.
        assert_eq!(addon(332, 1, [0.03f32.to_bits() as i32, 0, 0]).valor_do_id(), Some(vec![3]));
        // 2029 `enhance_attack_degree` = POINT 1.
        assert_eq!(addon(2029, 1, [1, 0, 0]).valor_do_id(), Some(vec![1]));
        // 831: essência de arma, editável, família pelo fonte.
        let a831 = addon(831, 2, [94, 94, 0]);
        assert_eq!(a831.edicao(), Edicao::Editavel);
        assert_eq!(a831.classe().and_then(|c| c.familia_no_fonte.as_deref()), Some("arma"));
        // `item_skill_addon`: dois argumentos como estão, fixo.
        let hab = DadosDoAddon { tratador: "item_skill_addon".into(), num_params: 2, params: [100, 3, 0], familias: None };
        assert_eq!((hab.valor_do_id(), hab.edicao()), (Some(vec![100, 3]), Edicao::Fixo));
        assert!(hab.aceita(&[100, 3]) && !hab.aceita(&[100, 9]));
        // Todo tratador classificado tem sorteio.
        let sem: Vec<&String> = classificacao().keys().filter(|t| DadosDoAddon { tratador: (*t).clone(), ..Default::default() }.sorteio().is_none()).collect();
        assert!(sem.is_empty(), "sem sorteio: {sem:?}");
    }
}
