//! Refino, furos e pedras do equipamento — a regra, sem bolsa nem rede (B163).
//!
//! Porte de `equip_item::RefineAddon` (`gs/item/equip_item.cpp:235-345`), `weapon_item::MakeSlot`
//! e `armor_item::MakeSlot` (`:1385-1470`), `socket_item::OnInsertChip`/`OnClearChips`
//! (`:762-810`) e `weapon/armor_item::AfterChipChanged` (`:991-1045`). O `gs` 1.2.6 tem as
//! mesmas tabelas (`refine_table` em 0x86eebc0, 240 B; `refine_factor` em 0x86eecc0, 52 B —
//! iguais valor a valor) e não tem `MakeSlot`.
//!
//! Quem chama (`bus_server/pedras_e_refino.rs`) cuida da bolsa, do dinheiro e dos comandos.

use pw_core::AddonDoItem;
use pw_data_loader::refino::{Familia, Pedra};

/// `get_refine_meterial_id` — a Pedra Celestial (`gs/template/itemdataman.cpp:201-204`;
/// `mov eax, 0x2bc8` no `gs` 1.2.6, VA 0x81e9af3).
pub const PEDRA_CELESTIAL: u32 = 11208;
/// `MAKE_SLOT_ITEM_ID` e `MAKE_SLOT_ITEM_ID2` (`gs/config.h:101-102`), gastos nesta ordem.
pub const MATERIAIS_DE_FURO: [u32; 2] = [21043, 34232];
/// `COOLDOWN_INDEX_REFINE` (22º do enum, `gs/cooldowncfg.h:86`; `push 0x16` no 1.2.6) e
/// `REFINE_COOLDOWN_TIME` (1000 ms, `:21`; `push 0x3e8`).
pub const RECARGA_DO_REFINO: i32 = 22;
pub const RECARGA_DO_REFINO_MS: i32 = 1000;
/// `ADDON_EMBEDDED` (`gs/item/item_addon.h:309`).
pub const ADDON_EMBUTIDO: u32 = 0x8000;
/// `EQUIP_MASK64_NECK | EQUIP_MASK64_WAIST` (`gs/item.h:146-149`): só colar e cinto furam.
pub const MASCARA_COLAR_OU_CINTO: i32 = 0x4 | 0x20;
/// `MAX_DECORATION_SOCKET_NUM` e `MAX_EQUIP_SOCKET_NUM` (`gs/config.h:247-248`).
pub const FUROS_NO_ACESSORIO: usize = 4;

/// `refine_param_t refine_table[]` (`equip_item.cpp:192-206`): chances de sucesso, de nada
/// mudar, de cair um nível e de voltar a zero, por nível atual.
pub const TABELA_DE_REFINO: [[f32; 4]; 12] = [
    [0.50, 0.7, 0.0, 0.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.30, 0.0, 0.0, 1.0],
    [0.25, 0.0, 0.0, 1.0],
    [0.20, 0.0, 0.0, 1.0],
    [0.12, 0.0, 0.0, 1.0],
    [0.05, 0.0, 0.0, 1.0],
];

/// `refine_factor[]` (`equip_item.cpp:208-223`): o multiplicador do parâmetro por nível.
pub const FATOR_DE_REFINO: [f32; 13] = [0.0, 1.0, 2.0, 3.05, 4.3, 5.75, 7.55, 9.95, 13.0, 17.05, 22.3, 29.0, 37.5];

/// `weapon_slot_material_count[2][20]` (`equip_item.cpp:1385-1386`).
const MATERIAL_DO_FURO_NA_ARMA: [[i32; 20]; 2] = [
    [5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 500, 1000, 4000, 6000, 8000, 10000, 12000, 14000, 16000, 18000],
    [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 1000, 2000, 8000, 12000, 16000, 20000, 24000, 28000, 32000, 36000],
];

/// `armor_slot_material_count[4][20]` (`equip_item.cpp:1428-1432`).
const MATERIAL_DO_FURO_NA_ARMADURA: [[i32; 20]; 4] = [
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100, 200, 800, 1200, 1600, 2000, 2400, 2800, 3200, 3600],
    [2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 200, 400, 1600, 2400, 3200, 4000, 4800, 5600, 6400, 7200],
    [3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 300, 600, 2400, 3600, 4800, 6000, 7200, 8400, 9600, 10800],
    [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 1000, 2000, 8000, 12000, 16000, 20000, 24000, 28000, 32000, 36000],
];

/// O resultado de `RefineAddon` (`item::REFINE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoDoRefino {
    Sucesso,
    NaoRefina,
    /// `REFINE_FAILED_LEVEL_0`: nada muda, o material se perde.
    NadaMudou,
    /// `REFINE_FAILED_LEVEL_1`: cai um nível.
    CaiuUm,
    /// `REFINE_FAILED_LEVEL_2`: o refino some.
    Zerou,
}

impl ResultadoDoRefino {
    /// O `refine_result(n)` de cada caso (`RefineItemAddon`, `player.cpp:11762-11797`) e se o
    /// item vai de novo ao cliente (`PlayerGetItemInfo`). `NaoRefina` não tem: é erro 92.
    pub fn para_o_cliente(self) -> Option<(i32, bool)> {
        match self {
            Self::Sucesso => Some((0, true)),
            Self::NadaMudou => Some((1, false)),
            Self::CaiuUm => Some((2, true)),
            Self::Zerou => Some((3, true)),
            Self::NaoRefina => None,
        }
    }
}

/// Os ajustes do talismã (`adjust[4]` e `adjust2[12]` de `RefineItemAddon`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ajustes {
    pub ajuste: [f32; 4],
    pub ajuste2: [f32; 12],
}

/// `abase::RandSelect(const float*, int)` (`cgame/include/arandomgen.h:129-139`) com o sorteio
/// `p` já tirado de `[0, 1)`.
pub fn sortear(probs: &[f32], mut p: f32) -> usize {
    for (i, x) in probs.iter().enumerate() {
        if p <= *x {
            return i;
        }
        p -= x;
    }
    0
}

/// O nível de refino que o item tem: `arg[1]` do addon de refino (`equip_item.cpp:241-251`).
pub fn nivel_de_refino(addons: &[AddonDoItem], addon_de_refino: u32) -> i32 {
    addons
        .iter()
        .find(|a| a.id() == addon_de_refino)
        .and_then(|a| a.args.get(1).copied())
        .unwrap_or(0)
}

/// `equip_item::RefineAddon`. `gerado` é o `generate_addon(addon_id)` (`None` → `NaoRefina`).
/// Devolve o resultado e o nível de antes (`level_result`).
pub fn refinar(
    addons: &mut Vec<AddonDoItem>,
    addon_id: u32,
    gerado: Option<AddonDoItem>,
    ajustes: &Ajustes,
    p: f32,
) -> (ResultadoDoRefino, i32) {
    use ResultadoDoRefino::*;
    let indice = addons.iter().position(|a| a.id() == addon_id);
    let nivel = indice.and_then(|i| addons[i].args.get(1).copied()).unwrap_or(0).max(0) as usize;
    if nivel >= TABELA_DE_REFINO.len() {
        return (NaoRefina, nivel as i32);
    }
    let mut prob = TABELA_DE_REFINO[nivel];
    for (x, a) in prob.iter_mut().zip(ajustes.ajuste) {
        *x += a;
    }
    if ajustes.ajuste[1] > 0.0 {
        prob[0] = ajustes.ajuste2[nivel];
    }
    let resultado = [Sucesso, NadaMudou, CaiuUm, Zerou][sortear(&prob, p)];
    let mut novo_nivel = nivel;
    match resultado {
        NadaMudou => return (NadaMudou, nivel as i32),
        CaiuUm => {
            let Some(i) = indice.filter(|_| nivel > 0) else { return (NadaMudou, nivel as i32) };
            if nivel == 1 {
                addons.remove(i);
                return (CaiuUm, nivel as i32);
            }
            novo_nivel -= 1;
        }
        Zerou => {
            if let Some(i) = indice {
                addons.remove(i);
            }
            return (Zerou, nivel as i32);
        }
        _ => novo_nivel += 1,
    }
    let Some(g) = gerado else { return (NaoRefina, nivel as i32) };
    let base = g.args.first().copied().unwrap_or(0);
    let valor = (base as f32 * FATOR_DE_REFINO[novo_nivel] + 0.1) as i32;
    match indice {
        None => addons.push(AddonDoItem { tipo: g.tipo, args: vec![valor, 1] }),
        Some(i) => {
            let a = &mut addons[i];
            if a.args.len() < 2 {
                a.args.resize(2, 0);
            }
            a.args[0] = valor;
            a.args[1] = novo_nivel as i32;
        }
    }
    (resultado, nivel as i32)
}

/// Quantas pedras de furo o próximo furo pede, ou o erro de `MakeSlot`: `ERR_MAKE_SLOT_FAILURE`
/// (106) para grau fora de 1–20 ou furos no limite (2 na arma, 4 na armadura).
pub fn material_do_furo(familia: Familia, grau: i32, furos: usize) -> Result<i32, i32> {
    if !(1..=20).contains(&grau) {
        return Err(erro::FURO_FALHOU);
    }
    let g = (grau - 1) as usize;
    match familia {
        Familia::Arma => MATERIAL_DO_FURO_NA_ARMA.get(furos).map(|t| t[g]).ok_or(erro::FURO_FALHOU),
        Familia::Armadura => MATERIAL_DO_FURO_NA_ARMADURA.get(furos).map(|t| t[g]).ok_or(erro::FURO_FALHOU),
        Familia::Acessorio => Err(erro::FURO_FALHOU),
    }
}

/// `item_list::EmbedItem` (`gs/item_list.cpp:242-275`): a pedra cabe no equipamento? Grau da
/// pedra até o do equipamento e o `IsStoneFit` (`gs/item.cpp:333-355`) com `combined_switch` 0 —
/// a coluna não existe no `STONE_ESSENCE` do v156 nem do v7 —, que deixa arma e armadura e recusa
/// acessório. O `EmbedItem` do `gs` 1.2.6 (VA 0x80b74ea-0x80b7521) só aceita arma (4) e armadura (7).
pub fn pedra_cabe(pedra: &Pedra, familia: Familia, grau_do_equipamento: i32) -> bool {
    pedra.grau <= grau_do_equipamento && matches!(familia, Familia::Arma | Familia::Armadura)
}

/// `socket_item::OnInsertChip`: a pedra no primeiro furo vazio e os addons dela marcados como
/// embutidos. `false` sem furo vazio ou sem addon para a família.
pub fn incrustar(furos: &mut [i32], addons: &mut Vec<AddonDoItem>, pedra_id: u32, addons_da_pedra: &[AddonDoItem]) -> bool {
    if addons_da_pedra.is_empty() {
        return false;
    }
    let Some(furo) = furos.iter_mut().find(|f| **f == 0) else { return false };
    *furo = pedra_id as i32;
    addons.extend(addons_da_pedra.iter().map(|a| AddonDoItem { tipo: a.tipo | ADDON_EMBUTIDO, args: a.args.clone() }));
    true
}

/// `socket_item::OnClearChips`: zera os furos e tira os addons embutidos.
pub fn limpar_pedras(furos: &mut [i32], addons: &mut Vec<AddonDoItem>) {
    furos.iter_mut().for_each(|f| *f = 0);
    addons.retain(|a| a.tipo & ADDON_EMBUTIDO == 0);
}

/// `weapon_item::AfterChipChanged` / `armor_item::AfterChipChanged`: o byte baixo do
/// `_modify_mask` (as cores das pedras de grau alto, que o cliente desenha). `cor_e_grau` dá
/// `(cor, grau)` da pedra (`GetStoneColorLevel`, `item_list.cpp`); o byte alto fica.
pub fn mascara_das_pedras(familia: Familia, furos: &[i32], mascara: u16, cor_e_grau: impl Fn(i32) -> (i32, i32)) -> u16 {
    let alto = mascara & !0xFF;
    let baixo: i32 = match familia {
        Familia::Arma => {
            if furos.len() != 2 || furos[0] == 0 || furos[1] == 0 {
                0
            } else {
                let (c1, g1) = cor_e_grau(furos[0]);
                let (c2, g2) = cor_e_grau(furos[1]);
                if g1 < 7 || g2 < 7 {
                    0
                } else {
                    ((c2 & 7) << 3) | (c1 & 7)
                }
            }
        }
        Familia::Armadura => {
            let (mut n5, mut n8, mut cor) = (0, 0, 0);
            for &f in furos.iter().filter(|f| **f != 0) {
                let (c, g) = cor_e_grau(f);
                if c == 0 || g < 5 {
                    continue;
                }
                n5 += 1;
                if g >= 8 {
                    n8 += 1;
                }
                cor = if cor != 0 { if cor != c { 7 } else { cor } } else { c & 7 };
            }
            if n8 >= 4 {
                cor + 8
            } else if n5 < 2 {
                0
            } else {
                cor
            }
        }
        // `decoration_equip_item::AfterChipChanged` não faz nada (`equip_item.h:615`).
        Familia::Acessorio => return mascara,
    };
    alto | (baixo & 0xFF) as u16
}

/// Os erros (`common/protocol.h`, enum `ERR_*`; os mesmos números empurrados no `gs` 1.2.6).
pub mod erro {
    pub const SERVICO_INDISPONIVEL: i32 = 14;
    /// `ERR_OUT_OF_FUND` (0x10 no 1.2.6, `install_executor::OnServe` VA 0x810b2e1).
    pub const SEM_DINHEIRO: i32 = 16;
    /// `ERR_CANNOT_EMBED` (0x15 no 1.2.6, VA 0x810b29d).
    pub const NAO_INCRUSTA: i32 = 21;
    pub const SEM_MATERIAL: i32 = 25;
    /// `ERR_OBJECT_IS_COOLING` (0x36 no 1.2.6, `refine_service_executor::SendRequest`).
    pub const EM_RECARGA: i32 = 54;
    /// `ERR_REFINE_CAN_NOT_REFINE` (0x5c no 1.2.6).
    pub const NAO_REFINA: i32 = 92;
    pub const FURO_FALHOU: i32 = 106;
    pub const FURO_FEITO: i32 = 107;
}

#[cfg(test)]
mod testes {
    use super::*;

    fn refino(nivel: i32) -> AddonDoItem {
        AddonDoItem::novo(1497, vec![12 * FATOR_DE_REFINO[nivel as usize] as i32, nivel])
    }
    fn gerado() -> Option<AddonDoItem> {
        Some(AddonDoItem::novo(1497, vec![12, 0]))
    }

    #[test]
    fn o_primeiro_refino_cria_o_addon_no_nivel_1_e_falhar_nao_muda_nada() {
        let mut a = vec![];
        let (r, antes) = refinar(&mut a, 1497, gerado(), &Ajustes::default(), 0.2);
        assert_eq!((r, antes), (ResultadoDoRefino::Sucesso, 0));
        assert_eq!(a, vec![AddonDoItem::novo(1497, vec![12, 1])]);
        // Nível 0 → 1: 50 % de sucesso, o resto "nada mudou" (0,7).
        let mut b = vec![];
        assert_eq!(refinar(&mut b, 1497, gerado(), &Ajustes::default(), 0.6).0, ResultadoDoRefino::NadaMudou);
        assert!(b.is_empty());
    }

    #[test]
    fn do_nivel_1_em_diante_falhar_zera_e_o_sucesso_usa_o_fator() {
        let mut a = vec![refino(3)];
        assert_eq!(refinar(&mut a, 1497, gerado(), &Ajustes::default(), 0.1).0, ResultadoDoRefino::Sucesso);
        // 12 × 4,3 + 0,1 = 51,7 → 51.
        assert_eq!(a[0].args, vec![51, 4]);
        assert_eq!(refinar(&mut a, 1497, gerado(), &Ajustes::default(), 0.9).0, ResultadoDoRefino::Zerou);
        assert!(a.is_empty());
    }

    #[test]
    fn o_talisma_que_mantem_o_nivel_troca_a_chance_e_cai_um_pela_chance_extra() {
        let mut aj = Ajustes::default();
        aj.ajuste[1] = 2.0;
        aj.ajuste2[5] = 0.0;
        let mut a = vec![refino(5)];
        // Chance de sucesso trocada por 0: o sorteio cai em "nada mudou" (2,0).
        assert_eq!(refinar(&mut a, 1497, gerado(), &aj, 0.1).0, ResultadoDoRefino::NadaMudou);
        let mut aj = Ajustes::default();
        aj.ajuste[2] = 1.0;
        aj.ajuste[0] = -0.3;
        let mut a = vec![refino(5)];
        assert_eq!(refinar(&mut a, 1497, gerado(), &aj, 0.5).0, ResultadoDoRefino::CaiuUm);
        assert_eq!(a[0].args[1], 4);
    }

    #[test]
    fn no_nivel_12_nao_refina() {
        let mut a = vec![AddonDoItem::novo(1497, vec![450, 12])];
        assert_eq!(refinar(&mut a, 1497, gerado(), &Ajustes::default(), 0.0).0, ResultadoDoRefino::NaoRefina);
    }

    #[test]
    fn furo_segue_as_tabelas_e_os_limites() {
        assert_eq!(material_do_furo(Familia::Arma, 1, 0), Ok(5));
        assert_eq!(material_do_furo(Familia::Arma, 12, 1), Ok(2000));
        assert_eq!(material_do_furo(Familia::Arma, 5, 2), Err(106));
        assert_eq!(material_do_furo(Familia::Armadura, 3, 3), Ok(30));
        assert_eq!(material_do_furo(Familia::Armadura, 3, 4), Err(106));
        assert_eq!(material_do_furo(Familia::Armadura, 0, 0), Err(106));
    }

    #[test]
    fn incrustar_e_limpar_mexem_so_nos_addons_embutidos() {
        let mut furos = vec![0, 0];
        let mut addons = vec![AddonDoItem::novo(206, vec![3])];
        assert!(incrustar(&mut furos, &mut addons, 777, &[AddonDoItem::novo(300, vec![9])]));
        assert_eq!(furos, vec![777, 0]);
        assert_eq!(addons[1].tipo & ADDON_EMBUTIDO, ADDON_EMBUTIDO);
        assert_eq!(addons[1].id(), 300);
        limpar_pedras(&mut furos, &mut addons);
        assert_eq!(furos, vec![0, 0]);
        assert_eq!(addons, vec![AddonDoItem::novo(206, vec![3])]);
        assert!(!incrustar(&mut [5], &mut addons, 777, &[AddonDoItem::novo(300, vec![9])]));
    }

    #[test]
    fn a_mascara_das_pedras_segue_o_after_chip_changed() {
        let cg = |id: i32| (id / 100, id % 100);
        // Arma: duas pedras de grau 7+ → cor2 << 3 | cor1.
        assert_eq!(mascara_das_pedras(Familia::Arma, &[207, 308], 0x100, cg), 0x100 | (3 << 3) | 2);
        assert_eq!(mascara_das_pedras(Familia::Arma, &[206, 308], 0x1FF, cg), 0x100);
        // Armadura: quatro de grau 8+ da mesma cor → cor + 8.
        assert_eq!(mascara_das_pedras(Familia::Armadura, &[208, 208, 209, 210], 0, cg), 10);
        // Cores diferentes → 7; menos de duas de grau 5+ → 0.
        assert_eq!(mascara_das_pedras(Familia::Armadura, &[205, 305], 0, cg), 7);
        assert_eq!(mascara_das_pedras(Familia::Armadura, &[205, 0], 0, cg), 0);
    }
}
