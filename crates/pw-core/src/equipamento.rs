//! O conteúdo de um equipamento — o bloco de dados que o item carrega.
//!
//! É o `_item_content` que `generate_weapon/armor/decoration/projectile` escrevem
//! (`gs/template/generate_item_temp.h:280-480`, `490-560`, `620-650`, `740-830`), o mesmo que
//! o banco guarda em `character_items.extra_data` e que o cliente lê em
//! `CECIvtrEquip::SetItemInfo` (`EC_IvtrEquip.cpp:176-262`):
//!
//! ```text
//! i16 nível, i16 classes, i16 força, i16 vitalidade, i16 agilidade, i16 energia
//! i32 durabilidade, i32 durabilidade máxima
//! i16 tamanho da essência, u8 feito-de, u8 tamanho do nome do fabricante (+ nome)
//! essência (da família)
//! i16 furos, u16 máscara das pedras, i32 × furos (id da pedra, 0 = vazio)
//! i32 addons, e cada addon: i32 tipo (id | nº de parâmetros << 13 | 0x8000 embutido), i32 × nº
//! ```
//!
//! **Um caminho de escrita**: o `OWN_ITEM_INFO` de item sem octetos gravados, os octetos
//! gerados no drop e a leitura para os atributos passam todos por aqui.

use crate::character::{FichaDaArma, FichaDaArmadura, FichaDaMunicao, FichaDeDecoracao, FichaDoEquipamento, ESCOLAS_MAGICAS};

/// Uma propriedade adicional (`addon_data` gravado, `EC_IvtrEquip.cpp:236-251`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonDoItem {
    /// O `iType` inteiro: id nos 13 bits baixos, nº de parâmetros nos bits 13-14, `0x8000`
    /// quando veio de pedra, `0x10000` de conjunto, `0x20000` gravado.
    pub tipo: u32,
    pub args: Vec<i32>,
}

impl AddonDoItem {
    /// `data.id & ADDON_PURE_TYPE_MASK`, o id do `EQUIPMENT_ADDON` (e do tratador).
    pub fn id(&self) -> u32 {
        self.tipo & 0x1FFF
    }

    pub fn novo(id: u32, args: Vec<i32>) -> Self {
        let n = (args.len() as u32).min(3);
        Self { tipo: (id & !(0x3 << 13)) | (n << 13), args }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConteudoDeEquipamento {
    pub ficha: FichaDoEquipamento,
    pub durabilidade: i32,
    pub durabilidade_maxima: i32,
    /// `attack_short_range` da arma (float, do `WEAPON_SUB_TYPE`).
    pub alcance_curto: f32,
    /// As pedras de cada furo; 0 é furo vazio.
    pub furos: Vec<i32>,
    pub mascara_das_pedras: u16,
    pub addons: Vec<AddonDoItem>,
    /// O `item_tag_t` gravado logo depois do tamanho da essência (`generate_item_temp.h:312-314`,
    /// `memcpy(buf, tag, tag_size)`): `type` é o `m_byMadeFrom` do cliente
    /// (`EC_IvtrEquip.cpp:199-206`) e o nome é o de quem fabricou, que o tooltip mostra.
    pub origem: OrigemDoItem,
    /// Nome do fabricante nos bytes que o original copia (`GetPlayerName`, UTF-16LE), no máximo
    /// `MAX_USERNAME_LENGTH` (40, `gs/config.h:139`). Vazio fora da produção.
    pub fabricante: Vec<u8>,
}

/// `element_data::ITEM_MAKE_TAG` (`gs/template/itemdataman.h:309-317`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OrigemDoItem {
    #[default]
    Nenhuma = 0,
    Criado = 1,
    /// `IMT_DROP`: o drop de monstro, a mina e o prêmio de missão (`generate_item_for_drop`).
    /// A captura do `gs` 1.2.6 original (`_sync/capturas/full_interno.pcap`, detalhe da bolsa)
    /// traz `02 00` depois do tamanho da essência.
    Drop = 2,
    /// `IMT_SHOP`: a variante de venda (`get_item_for_sell`, `itemdataman.cpp:1352`).
    Loja = 3,
    /// `IMT_PRODUCE`: `gplayer_imp::ProduceItem` (`player.cpp:16499-16515`).
    Producao = 4,
}

impl OrigemDoItem {
    fn de(b: u8) -> Self {
        match b {
            1 => Self::Criado,
            2 => Self::Drop,
            3 => Self::Loja,
            4 => Self::Producao,
            _ => Self::Nenhuma,
        }
    }
}

/// `MAX_USERNAME_LENGTH` (`gs/config.h:139`): o `ProduceItem` corta o nome aqui.
pub const TAMANHO_MAXIMO_DO_NOME: usize = 40;

/// O nome do personagem como o original o grava no item: os bytes UTF-16LE do nome, cortados
/// em `MAX_USERNAME_LENGTH` (`player.cpp:16509-16513`).
pub fn nome_do_fabricante(nome: &str) -> Vec<u8> {
    let mut b: Vec<u8> = nome.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    b.truncate(TAMANHO_MAXIMO_DO_NOME);
    b
}

struct Leitor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Leitor<'_> {
    fn pegar<const N: usize>(&mut self) -> Option<[u8; N]> {
        let v = self.b.get(self.i..self.i + N)?.try_into().ok()?;
        self.i += N;
        Some(v)
    }
    fn i16(&mut self) -> Option<i16> {
        self.pegar().map(i16::from_le_bytes)
    }
    fn u16(&mut self) -> Option<u16> {
        self.pegar().map(u16::from_le_bytes)
    }
    fn i32(&mut self) -> Option<i32> {
        self.pegar().map(i32::from_le_bytes)
    }
    fn f32(&mut self) -> Option<f32> {
        self.pegar().map(f32::from_le_bytes)
    }
    fn u8(&mut self) -> Option<u8> {
        self.pegar::<1>().map(|b| b[0])
    }
}

/// `DURABILITY_UNIT_COUNT` (`gs/config.h:59`) / `ENDURANCE_SCALE` (`EC_IvtrTypes.h:26`): uma
/// unidade de durabilidade na tela são **100** pontos internos.
///
/// Quem gera um item multiplica por isto no fim (`update_require_data`,
/// `gs/item/item_addon.h:454-458`, chamado em `generate_item_temp.h:367, 552, 644, 831`), e é
/// nessa escala que a durabilidade viaja no bloco de dados e fica no banco. O cliente divide
/// de volta, arredondando para cima (`CECIvtrEquip::GetEnduranceDisplay`,
/// `EC_IvtrEquip.cpp:281`) — por isso um arco gerado com 50 (a escala do `elements.data`)
/// aparecia como **1/1** em jogo (relato de 2026-09-18).
pub const ESCALA_DA_DURABILIDADE: i32 = 100;

/// O `durability` e o `max_durability` estão em posição fixa no bloco: logo depois dos seis
/// `short` de requisito (`generate_item_temp.h:288-310`). São 12 e 16 bytes do começo.
pub const OFFSET_DA_DURABILIDADE: usize = 12;

/// Regrava a durabilidade dentro de um bloco já montado, sem reler o resto.
///
/// O bloco é a fonte que o cliente lê; a coluna do banco é a que o servidor desgasta. Esta
/// função é o que mantém as duas iguais na hora de mandar o item. Devolve `false` quando o
/// bloco é curto demais para ter durabilidade (item que não é equipamento).
pub fn escrever_durabilidade(bloco: &mut [u8], durabilidade: i32, maxima: i32) -> bool {
    if bloco.len() < OFFSET_DA_DURABILIDADE + 8 {
        return false;
    }
    bloco[OFFSET_DA_DURABILIDADE..OFFSET_DA_DURABILIDADE + 4].copy_from_slice(&durabilidade.to_le_bytes());
    bloco[OFFSET_DA_DURABILIDADE + 4..OFFSET_DA_DURABILIDADE + 8].copy_from_slice(&maxima.to_le_bytes());
    true
}

/// A `prerequisition` do começo do bloco de equipamento (`gs/item/item_addon.h:30-40`):
/// `short level, race, strength, vitality, agility, energy; int durability, max_durability`.
/// `race` é a máscara de classes (`character_combo_id & 0xFFFF`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requisitos {
    pub nivel: i16,
    pub classes: u16,
    pub forca: i16,
    pub vitalidade: i16,
    pub agilidade: i16,
    pub energia: i16,
}

impl Requisitos {
    /// Lê os requisitos do bloco; `None` se ele é curto demais para ter a `prerequisition`.
    pub fn do_bloco(bloco: &[u8]) -> Option<Self> {
        if bloco.len() < 12 {
            return None;
        }
        let h = |i: usize| i16::from_le_bytes([bloco[2 * i], bloco[2 * i + 1]]);
        Some(Self {
            nivel: h(0),
            classes: h(1) as u16,
            forca: h(2),
            vitalidade: h(3),
            agilidade: h(4),
            energia: h(5),
        })
    }

    /// `equip_item::VerifyRequirement` (`gs/item/equip_item.cpp:60-80`), sem a reputação: nível
    /// histórico, o bit da classe (`1 << (classe & 0x0F)`) e os quatro atributos de `_cur_prop`.
    /// `atributos` na ordem `(vitalidade, energia, força, agilidade)`.
    pub fn atende(&self, nivel: i32, classe: i32, atributos: (i32, i32, i32, i32)) -> bool {
        let (vit, eng, forca, agi) = atributos;
        nivel >= self.nivel as i32
            && (1u32 << (classe & 0x0F)) & self.classes as u32 != 0
            && forca >= self.forca as i32
            && vit >= self.vitalidade as i32
            && agi >= self.agilidade as i32
            && eng >= self.energia as i32
    }
}

impl ConteudoDeEquipamento {
    /// Um item sem furos nem addons, com a ficha do modelo.
    pub fn novo(ficha: FichaDoEquipamento, durabilidade: i32, durabilidade_maxima: i32) -> Self {
        Self {
            ficha,
            durabilidade,
            durabilidade_maxima,
            alcance_curto: 0.0,
            furos: Vec::new(),
            mascara_das_pedras: 0,
            addons: Vec::new(),
            origem: OrigemDoItem::Nenhuma,
            fabricante: Vec::new(),
        }
    }

    fn requisitos(&self) -> (i16, i32, i16, i16, i16, i16) {
        use FichaDoEquipamento as F;
        match &self.ficha {
            F::Arma(a) => (a.nivel_exigido, a.classes_permitidas, a.forca_exigida, a.vitalidade_exigida, a.agilidade_exigida, a.energia_exigida),
            F::Armadura(a) => (a.nivel_exigido, a.classes_permitidas, a.forca_exigida, a.vitalidade_exigida, a.agilidade_exigida, a.energia_exigida),
            F::Decoracao(d) => (d.nivel_exigido, d.classes_permitidas, d.forca_exigida, d.vitalidade_exigida, d.agilidade_exigida, d.energia_exigida),
            // `generate_projectile` (`generate_item_temp.h:607-615`): sem requisito, máscara 0xFFFF.
            F::Municao(_) => (0, 0xFFFF, 0, 0, 0, 0),
        }
    }

    /// O bloco inteiro, na ordem de `CECIvtrEquip::SetItemInfo`.
    pub fn escrever(&self) -> Vec<u8> {
        use FichaDoEquipamento as F;
        let mut o = Vec::with_capacity(96);
        let (nivel, classes, forca, vitalidade, agilidade, energia) = self.requisitos();
        // **Vitalidade antes de agilidade**, e a máscara truncada a 16 bits pelo próprio
        // original (`character_combo_id & 0xFFFF`).
        for v in [nivel, classes as i16, forca, vitalidade, agilidade, energia] {
            o.extend_from_slice(&v.to_le_bytes());
        }
        let (dur, dur_max) = match self.ficha {
            // `generate_projectile` grava 1 e 1 (`generate_item_temp.h:615-616`), e o
            // `update_require_data` do fim multiplica os dois pela escala.
            F::Municao(_) => (ESCALA_DA_DURABILIDADE, ESCALA_DA_DURABILIDADE),
            _ => (self.durabilidade, self.durabilidade_maxima),
        };
        o.extend_from_slice(&dur.to_le_bytes());
        o.extend_from_slice(&dur_max.to_le_bytes());
        let tamanho: i16 = match self.ficha {
            F::Arma(_) => 44,
            F::Armadura(_) | F::Decoracao(_) => 36,
            F::Municao(_) => 20,
        };
        o.extend_from_slice(&tamanho.to_le_bytes());
        // `item_tag_t { char type; char size; char name[size]; }`.
        o.push(self.origem as u8);
        let nome = &self.fabricante[..self.fabricante.len().min(TAMANHO_MAXIMO_DO_NOME)];
        o.push(nome.len() as u8);
        o.extend_from_slice(nome);
        let i32s = |o: &mut Vec<u8>, vs: &[i32]| {
            for v in vs {
                o.extend_from_slice(&v.to_le_bytes());
            }
        };
        match &self.ficha {
            // `IVTR_ESSENCE_WEAPON` (`EC_IvtrTypes.h`), 44 bytes, na ordem de
            // `generate_weapon` (`generate_item_temp.h:318-352`).
            F::Arma(a) => {
                o.extend_from_slice(&a.tipo_de_arma.to_le_bytes());
                o.extend_from_slice(&0i16.to_le_bytes()); // weapon_delay
                i32s(&mut o, &[a.tipo_maior, a.nivel_da_arma, a.municao_exigida, a.dano_minimo, a.dano_maximo, a.dano_magico_minimo, a.dano_magico_maximo, a.velocidade_de_ataque]);
                o.extend_from_slice(&a.alcance.to_le_bytes());
                o.extend_from_slice(&self.alcance_curto.to_le_bytes());
            }
            // `IVTR_ESSENCE_ARMOR` (`EC_IvtrTypes.h:253-260`).
            F::Armadura(a) => {
                i32s(&mut o, &[a.defesa, a.evasao, a.mp_extra, a.hp_extra]);
                i32s(&mut o, &a.resistencias);
            }
            // `IVTR_ESSENCE_DECORATION` (`EC_IvtrTypes.h:244-251`): dano e dano mágico
            // **antes** da defesa.
            F::Decoracao(d) => {
                i32s(&mut o, &[d.dano, d.dano_magico, d.defesa, d.evasao]);
                i32s(&mut o, &d.resistencias);
            }
            // `IVTR_ESSENCE_ARROW` (`EC_IvtrTypes.h:235-242`).
            F::Municao(m) => i32s(&mut o, &[m.tipo, m.dano_extra, m.dano_extra_percentual, m.nivel_minimo_da_arma, m.nivel_maximo_da_arma]),
        }
        self.escrever_rabo(&mut o);
        o
    }

    /// `SaveSocketData` + `SaveAddOn` (`gs/item/equip_item.h:407-415`): furos, máscara das
    /// pedras, as pedras, e os addons.
    fn escrever_rabo(&self, o: &mut Vec<u8>) {
        o.extend_from_slice(&(self.furos.len() as i16).to_le_bytes());
        o.extend_from_slice(&self.mascara_das_pedras.to_le_bytes());
        for v in &self.furos {
            o.extend_from_slice(&v.to_le_bytes());
        }
        o.extend_from_slice(&(self.addons.len() as i32).to_le_bytes());
        for a in &self.addons {
            o.extend_from_slice(&a.tipo.to_le_bytes());
            for v in &a.args {
                o.extend_from_slice(&v.to_le_bytes());
            }
        }
    }

    fn tamanho_do_rabo(&self) -> usize {
        4 + 4 * self.furos.len() + 4 + self.addons.iter().map(|a| 4 + 4 * a.args.len()).sum::<usize>()
    }

    /// Lê o bloco gravado, deixa `f` mexer em furos, máscara e addons, e devolve o bloco com
    /// o rabo novo e **o cabeçalho e a essência byte a byte como estavam** — o
    /// `equip_item::OnRefreshItem` (`gs/item/equip_item.cpp:83-97`) regrava `_ess` intacta e
    /// só muda o que vem depois. `None` se o bloco não fecha no último byte.
    pub fn alterar_rabo<R>(bytes: &[u8], modelo: &FichaDoEquipamento, f: impl FnOnce(&mut Self) -> R) -> Option<(Vec<u8>, R)> {
        let mut c = Self::ler(bytes, modelo)?;
        let corpo = bytes.len() - c.tamanho_do_rabo();
        let r = f(&mut c);
        let mut o = bytes[..corpo].to_vec();
        c.escrever_rabo(&mut o);
        Some((o, r))
    }

    /// Lê um bloco gravado. `modelo` diz a família (a essência não se identifica sozinha:
    /// armadura e acessório têm o mesmo tamanho) e dá os campos que não viajam
    /// (`tipo_de_arma` é lido; `classes` vem do cabeçalho).
    pub fn ler(bytes: &[u8], modelo: &FichaDoEquipamento) -> Option<Self> {
        use FichaDoEquipamento as F;
        let mut r = Leitor { b: bytes, i: 0 };
        let (nivel, classes, forca, vitalidade, agilidade, energia) = (r.i16()?, r.i16()?, r.i16()?, r.i16()?, r.i16()?, r.i16()?);
        let (durabilidade, durabilidade_maxima) = (r.i32()?, r.i32()?);
        let tamanho = r.i16()?;
        let origem = OrigemDoItem::de(r.u8()?);
        let nome = r.u8()? as usize;
        let fabricante = r.b.get(r.i..r.i + nome)?.to_vec();
        r.i += nome;
        let inicio = r.i;
        let classes = classes as u16 as i32;
        let mut alcance_curto = 0.0;
        let ficha = match modelo {
            F::Arma(_) => {
                let tipo_de_arma = r.i16()?;
                let _delay = r.i16()?;
                let (tipo_maior, nivel_da_arma, municao_exigida) = (r.i32()?, r.i32()?, r.i32()?);
                let (dano_minimo, dano_maximo, dano_magico_minimo, dano_magico_maximo) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?);
                let velocidade_de_ataque = r.i32()?;
                let alcance = r.f32()?;
                alcance_curto = r.f32()?;
                F::Arma(FichaDaArma {
                    tipo_de_arma,
                    classes_permitidas: classes,
                    nivel_exigido: nivel,
                    forca_exigida: forca,
                    vitalidade_exigida: vitalidade,
                    agilidade_exigida: agilidade,
                    energia_exigida: energia,
                    municao_exigida,
                    tipo_maior,
                    nivel_da_arma,
                    dano_minimo,
                    dano_maximo,
                    dano_magico_minimo,
                    dano_magico_maximo,
                    velocidade_de_ataque,
                    alcance,
                })
            }
            F::Armadura(_) => {
                let (defesa, evasao, mp_extra, hp_extra) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?);
                let mut resistencias = [0; ESCOLAS_MAGICAS];
                for x in &mut resistencias {
                    *x = r.i32()?;
                }
                F::Armadura(FichaDaArmadura {
                    classes_permitidas: classes,
                    nivel_exigido: nivel,
                    forca_exigida: forca,
                    vitalidade_exigida: vitalidade,
                    agilidade_exigida: agilidade,
                    energia_exigida: energia,
                    defesa,
                    evasao,
                    mp_extra,
                    hp_extra,
                    resistencias,
                })
            }
            F::Decoracao(_) => {
                let (dano, dano_magico, defesa, evasao) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?);
                let mut resistencias = [0; ESCOLAS_MAGICAS];
                for x in &mut resistencias {
                    *x = r.i32()?;
                }
                F::Decoracao(FichaDeDecoracao {
                    classes_permitidas: classes,
                    nivel_exigido: nivel,
                    forca_exigida: forca,
                    vitalidade_exigida: vitalidade,
                    agilidade_exigida: agilidade,
                    energia_exigida: energia,
                    dano,
                    dano_magico,
                    defesa,
                    evasao,
                    resistencias,
                })
            }
            F::Municao(_) => F::Municao(FichaDaMunicao {
                tipo: r.i32()?,
                dano_extra: r.i32()?,
                dano_extra_percentual: r.i32()?,
                nivel_minimo_da_arma: r.i32()?,
                nivel_maximo_da_arma: r.i32()?,
            }),
        };
        // A essência tem o tamanho que o bloco declara (`dr.Offset(iEssenceSize)`).
        if tamanho < 0 || r.i != inicio + tamanho as usize {
            return None;
        }
        let n_furos = r.i16()?;
        let mascara_das_pedras = r.u16()?;
        if n_furos < 0 {
            return None;
        }
        let furos = (0..n_furos).map(|_| r.i32()).collect::<Option<Vec<_>>>()?;
        let n_addons = r.i32()?;
        if n_addons < 0 {
            return None;
        }
        let mut addons = Vec::with_capacity(n_addons as usize);
        for _ in 0..n_addons {
            let tipo = r.i32()? as u32;
            let n = (tipo & 0x6000) >> 13;
            let args = (0..n).map(|_| r.i32()).collect::<Option<Vec<_>>>()?;
            addons.push(AddonDoItem { tipo, args });
        }
        // O bloco fecha no último byte.
        (r.i == bytes.len()).then_some(Self { ficha, durabilidade, durabilidade_maxima, alcance_curto, furos, mascara_das_pedras, addons, origem, fabricante })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn arco() -> FichaDoEquipamento {
        FichaDoEquipamento::Arma(FichaDaArma {
            tipo_de_arma: 1,
            classes_permitidas: 0x20,
            nivel_exigido: 1,
            forca_exigida: 0,
            vitalidade_exigida: 0,
            agilidade_exigida: 5,
            energia_exigida: 0,
            municao_exigida: 1,
            tipo_maior: 7,
            nivel_da_arma: 0,
            dano_minimo: 5,
            dano_maximo: 8,
            dano_magico_minimo: 0,
            dano_magico_maximo: 0,
            velocidade_de_ataque: 30,
            alcance: 20.0,
        })
    }

    #[test]
    fn o_bloco_escrito_se_le_de_volta_e_fecha_no_ultimo_byte() {
        let mut c = ConteudoDeEquipamento::novo(arco(), 2500, 3000);
        c.furos = vec![0, 0];
        c.addons = vec![AddonDoItem::novo(206, vec![3]), AddonDoItem::novo(1497, vec![12, 1])];
        let b = c.escrever();
        // cabeçalho 12 + dur 8 + tamanho 2 + fabricante 2 + essência 44 + furos 4 + 8 +
        // addons 4 + (4+4) + (4+8).
        assert_eq!(b.len(), 12 + 8 + 2 + 2 + 44 + 4 + 8 + 4 + 8 + 12);
        let lido = ConteudoDeEquipamento::ler(&b, &arco()).expect("ler");
        assert_eq!(lido, c);
        assert_eq!(lido.addons[1].id(), 1497);
        assert!(ConteudoDeEquipamento::ler(&b[..b.len() - 1], &arco()).is_none());
    }

    #[test]
    fn alterar_o_rabo_preserva_cabecalho_e_essencia() {
        let mut c = ConteudoDeEquipamento::novo(arco(), 2500, 3000);
        c.furos = vec![0];
        let mut b = c.escrever();
        // Um byte da essência que a reescrita completa não reproduziria (`weapon_delay`).
        b[26] = 7;
        let (novo, _) = ConteudoDeEquipamento::alterar_rabo(&b, &arco(), |c| {
            c.furos.push(0);
            c.addons.push(AddonDoItem::novo(1497, vec![12, 1]));
        })
        .expect("alterar");
        let corpo = b.len() - 12;
        assert_eq!(&novo[..corpo], &b[..corpo]);
        let lido = ConteudoDeEquipamento::ler(&novo, &arco()).expect("ler");
        assert_eq!(lido.furos, vec![0, 0]);
        assert_eq!(lido.addons, vec![AddonDoItem::novo(1497, vec![12, 1])]);
    }

    #[test]
    fn sem_furos_nem_addons_o_rabo_e_de_8_bytes() {
        let b = ConteudoDeEquipamento::novo(arco(), 10, 10).escrever();
        assert_eq!(&b[b.len() - 8..], &[0u8; 8]);
    }
}
