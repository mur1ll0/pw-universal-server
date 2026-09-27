//! `gshop*.data` — a Loja Gold (mall, "Boutique").
//!
//! # Formato (juiz: o carregador do cliente)
//!
//! `GlobalData_Load` (`EvolvedPWClient/ElementClient/CCommon/globaldataman.cpp:614-660`):
//!
//! ```text
//! u32 timestamp
//! i32 n
//! n × GSHOP_ITEM            (registro de tamanho fixo, depende da build do cliente)
//! 8 × { wchar szName[64]; i32 nsub; nsub × wchar[64] }   (categorias)
//! ```
//!
//! O registro muda de tamanho entre builds, e o arquivo só fecha no último byte com o
//! tamanho certo — medido nos arquivos dos realms (`docs/evidencias/LOJA_GOLD_DIAGNOSTICO.md`):
//!
//! | layout | bytes | origem |
//! | :--- | ---: | :--- |
//! | 1.5.5 com `VIP` | 1436 | `globaldataman.h:101-138` (1.5.5): `buy[4]` de 36 B, brinde, donos, limite |
//! | 1.5.x sem `VIP` | 1412 | `globaldataman.h:101-131` (1.5.3): `buy[4]` de 32 B, brinde, donos |
//! | 1.2.6 | 1288 | `load_malldata` do `gs` 1.2.6 (VA 0x81e8644): `buy[4]` de 12 B `{price, data, time}` |
//!
//! O leitor tenta os três e fica com o que fecha o arquivo **no último byte**, contando as 8
//! categorias do fim. Nenhum fechando (o `gshopsev*.data` do servidor, arquivos de teste),
//! fica só o carimbo, e a loja fica vazia — o carimbo é o que o `edition` do login usa
//! (spec 02 §3.3).
//!
//! # Por que o do cliente, e não o `gshopsev.data`
//!
//! O cliente manda o **índice** da oferta na lista dele (`C2S::MALL_SHOPPING`), e o servidor
//! confere o `goods_id` naquele índice (`gs/player.cpp:15740-15745`). O `gshopsev.data` do
//! `realm_155` tem 2.262 ofertas (2023) contra as 1.726 do `gshop.data` do cliente BR —
//! outra lista. O `gs` 1.2.6 original lê o próprio `gshop.data` do cliente (o pacote do
//! servidor traz o mesmo arquivo, byte a byte).
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum GShopError {
    #[error("Erro de I/O na leitura do gshop.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato de gshop.data inválido")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, GShopError>;

/// Qual `GSHOP_ITEM` fechou o arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatoDoGshop {
    /// 1436 B — cliente compilado com `VIP` (1.5.5 BR).
    V155Vip,
    /// 1412 B — 1.5.x sem `VIP`.
    V15xSemVip,
    /// 1288 B — 1.2.6.
    V126,
}

impl FormatoDoGshop {
    pub const TODOS: [FormatoDoGshop; 3] = [Self::V155Vip, Self::V15xSemVip, Self::V126];

    pub fn bytes(self) -> usize {
        match self {
            Self::V155Vip => 1436,
            Self::V15xSemVip => 1412,
            Self::V126 => 1288,
        }
    }
}

/// Uma das quatro formas de comprar a oferta (`GSHOP_ITEM::buy[4]`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcaoDeCompra {
    /// `price` → `cash_need`. Zero = opção vazia (`cash_need <= 0` é pedido inválido).
    pub preco: u32,
    /// `time`: validade em segundos a partir da compra (0 = permanente).
    pub validade_s: u32,
    /// Data absoluta de validade. Só o 1.2.6 a tem: o 2º campo do `buy`, que o
    /// `load_malldata` 1.2.6 (VA 0x81e87dc) usa no lugar do `time` quando não é zero. No
    /// 1.5.x o `expire_date_valid` é sempre 0 (`template/globaldataman.cpp:175`).
    pub validade_absoluta: u32,
    /// `type` do período de venda: −1 = sem período (`load_malldata`, `st_type = 0`). O
    /// 1.2.6 não tem período: −1.
    pub tipo_de_periodo: i32,
    /// `flag` → `group_id`.
    pub grupo: u32,
    /// `min_vip_level` (só no layout VIP).
    pub vip_minimo: u32,
}

/// Uma oferta da loja, na posição em que o cliente a vê.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfertaDaLoja {
    /// `id` — o item entregue.
    pub item_id: u32,
    /// `num`.
    pub quantidade: u32,
    pub opcoes: [OpcaoDeCompra; 4],
    /// `idGift`, `iGiftNum`, `iGiftTime`, `iLogPrice` (1.5.x; zero no 1.2.6).
    pub brinde_id: u32,
    pub brinde_quantidade: u32,
    pub brinde_validade_s: u32,
    pub brinde_preco_de_registro: u32,
    /// `owner_npcs[8]` — oferta restrita a NPC (`node.check_owner`).
    pub donos: [u32; 8],
    /// `buy_times_limit` / `buy_times_limit_mode` (layout VIP; 0 = sem limite).
    pub limite_de_compras: i32,
    pub modo_do_limite: i32,
    /// `szName`, para o log.
    pub nome: String,
}

/// Contêiner de um `gshop*.data`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GShopData {
    pub timestamp: u32,
    /// `None` quando nenhum layout conhecido fechou o arquivo (a loja fica vazia).
    pub formato: Option<FormatoDoGshop>,
    /// Ofertas **na ordem do arquivo**: o índice é o `goods_index` do `MALL_SHOPPING`.
    pub ofertas: Vec<OfertaDaLoja>,
}

fn u32_em(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn i32_em(b: &[u8], o: usize) -> i32 {
    u32_em(b, o) as i32
}

fn texto(b: &[u8]) -> String {
    let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
    String::from_utf16_lossy(&u)
}

/// As 8 categorias do fim (`globaldataman.cpp:641-670`) a partir de `off`; devolve onde
/// terminam.
fn fim_das_categorias(d: &[u8], mut off: usize) -> Option<usize> {
    for _ in 0..8 {
        off = off.checked_add(128)?;
        if off + 4 > d.len() {
            return None;
        }
        let nsub = i32_em(d, off);
        if !(0..=1024).contains(&nsub) {
            return None;
        }
        off += 4 + nsub as usize * 128;
    }
    Some(off)
}

fn ler_oferta(formato: FormatoDoGshop, b: &[u8]) -> OfertaDaLoja {
    // Cabeçalho comum: local_id@0 main_type@4 sub_type@8 icon[128]@12 id@140 num@144.
    let mut o = OfertaDaLoja { item_id: u32_em(b, 140), quantidade: u32_em(b, 144), ..Default::default() };
    match formato {
        FormatoDoGshop::V126 => {
            // buy[4] de 12 B em 148: {price, data absoluta, time}; status@196; desc@200;
            // szName@1224.
            for j in 0..4 {
                let e = 148 + 12 * j;
                o.opcoes[j] = OpcaoDeCompra {
                    preco: u32_em(b, e),
                    validade_absoluta: u32_em(b, e + 4),
                    validade_s: u32_em(b, e + 8),
                    tipo_de_periodo: -1,
                    ..Default::default()
                };
            }
            o.nome = texto(&b[1224..1288]);
        }
        FormatoDoGshop::V155Vip | FormatoDoGshop::V15xSemVip => {
            // buy[4] em 148: {price, end_time, time, start_time, type, day, status, flag
            // [, min_vip_level]}.
            let vip = formato == FormatoDoGshop::V155Vip;
            let passo = if vip { 36 } else { 32 };
            for j in 0..4 {
                let e = 148 + passo * j;
                o.opcoes[j] = OpcaoDeCompra {
                    preco: u32_em(b, e),
                    validade_s: u32_em(b, e + 8),
                    validade_absoluta: 0,
                    tipo_de_periodo: i32_em(b, e + 16),
                    grupo: u32_em(b, e + 28),
                    vip_minimo: if vip { u32_em(b, e + 32) } else { 0 },
                };
            }
            let desc = 148 + 4 * passo; // wchar desc[512]
            let nome = desc + 1024; // wchar szName[32]
            let brinde = nome + 64;
            o.nome = texto(&b[nome..brinde]);
            o.brinde_id = u32_em(b, brinde);
            o.brinde_quantidade = u32_em(b, brinde + 4);
            o.brinde_validade_s = u32_em(b, brinde + 8);
            o.brinde_preco_de_registro = u32_em(b, brinde + 12);
            for k in 0..8 {
                o.donos[k] = u32_em(b, brinde + 16 + 4 * k);
            }
            if vip {
                o.limite_de_compras = i32_em(b, brinde + 48);
                o.modo_do_limite = i32_em(b, brinde + 52);
            }
        }
    }
    o
}

impl GShopData {
    /// Lê o carimbo e, se um dos layouts conhecidos fechar o arquivo, as ofertas.
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let timestamp = cursor.read_u32::<LittleEndian>()?;
        let n = cursor.read_i32::<LittleEndian>().unwrap_or(0);

        let mut gshop = Self { timestamp, formato: None, ofertas: Vec::new() };
        if !(0..=65535).contains(&n) {
            warn!("gshop.data: {n} ofertas no cabeçalho — só o carimbo foi lido");
            return Ok(gshop);
        }
        let n = n as usize;
        let formato = FormatoDoGshop::TODOS
            .into_iter()
            .find(|f| fim_das_categorias(data, 8 + n * f.bytes()) == Some(data.len()));
        let Some(formato) = formato else {
            info!("gshop: carimbo {timestamp}; nenhum layout de cliente fecha o arquivo ({} B, {n} ofertas) — loja vazia", data.len());
            return Ok(gshop);
        };
        let tam = formato.bytes();
        gshop.ofertas = (0..n).map(|i| ler_oferta(formato, &data[8 + i * tam..8 + (i + 1) * tam])).collect();
        gshop.formato = Some(formato);
        info!("gshop: carimbo {timestamp}, {n} ofertas, layout {formato:?} ({tam} B)");
        Ok(gshop)
    }

    /// A oferta no índice que o cliente mandou.
    pub fn oferta(&self, indice: usize) -> Option<&OfertaDaLoja> {
        self.ofertas.get(indice)
    }
}
