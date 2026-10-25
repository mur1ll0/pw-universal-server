//! `gshop*.data` editável (B254, Estúdio S7): a Loja Gold registro a registro, com ida e volta idêntica.
//!
//! O formato é o do cliente (`GlobalData_Load`, `CCommon/globaldataman.cpp:614-660`; ver `crate::gshop`): `u32
//! carimbo`, `i32 n`, `n × GSHOP_ITEM` de tamanho fixo e 8 categorias `{wchar nome[64]; i32 nsub; nsub × wchar[64]}`.
//! Os registros ficam crus (bytes): só o campo editado muda, e o lixo depois do NUL dos textos fica onde estava.
//!
//! Campos de `GSHOP_ITEM` (`globaldataman.h:97-138`, `pack(1)`), pelos nomes do fonte:
//!
//! | layout | `buy[k]` em `148 + passo·k` | depois |
//! | :--- | :--- | :--- |
//! | 1.5.5 VIP (1436) | passo 36: `price end_time time start_time type day status flag min_vip_level` | `desc`@292, `szName`@1316, `idGift iGiftNum iGiftTime iLogPrice`@1380, `owner_npcs[8]`@1396, `buy_times_limit(_mode)`@1428 |
//! | 1.5.x (1412) | passo 32 (sem `min_vip_level`) | `desc`@276, `szName`@1300, brinde@1364, `owner_npcs`@1380 |
//! | 1.2.6 (1288) | passo 12: `price end_time time` | `status`@196, `desc`@200, `szName`@1224 (`load_malldata` do `gs` 1.2.6) |

use crate::gshop::FormatoDoGshop;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroDoGshop {
    #[error("gshop: {0}")]
    Formato(String),
}

type R<T> = Result<T, ErroDoGshop>;

/// Tipo de um campo de `GSHOP_ITEM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDoCampo {
    I32,
    U32,
    /// `char[n]` (GBK) — o ícone.
    Gbk(usize),
    /// `wchar_t[n]` (UTF-16LE) — descrição e nome.
    Utf16(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Categoria {
    /// `wchar nome[64]`, cru.
    pub nome: Vec<u8>,
    /// Cada subcategoria, `wchar[64]` cru.
    pub subcategorias: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GshopEditavel {
    pub formato: FormatoDoGshop,
    pub carimbo: u32,
    pub ofertas: Vec<Vec<u8>>,
    pub categorias: Vec<Categoria>,
}

const BLOCO: usize = 128; // wchar[64]

impl GshopEditavel {
    /// Tenta os três layouts e fica com o que fecha o arquivo no último byte (como `crate::gshop`).
    pub fn ler(b: &[u8]) -> R<Self> {
        let erro = |m: &str| ErroDoGshop::Formato(m.to_string());
        if b.len() < 8 {
            return Err(erro("menor que o cabeçalho"));
        }
        let carimbo = u32::from_le_bytes(b[0..4].try_into().unwrap());
        let n = i32::from_le_bytes(b[4..8].try_into().unwrap());
        if n < 0 {
            return Err(erro("quantidade negativa"));
        }
        for formato in [FormatoDoGshop::V155Vip, FormatoDoGshop::V15xSemVip, FormatoDoGshop::V126] {
            let tam = formato.bytes();
            let Some(fim) = 8usize.checked_add(n as usize * tam).filter(|&f| f <= b.len()) else { continue };
            if let Some(categorias) = ler_categorias(&b[fim..]) {
                let ofertas = (0..n as usize).map(|i| b[8 + i * tam..8 + (i + 1) * tam].to_vec()).collect();
                return Ok(Self { formato, carimbo, ofertas, categorias });
            }
        }
        Err(erro("nenhum layout conhecido fecha o arquivo no último byte"))
    }

    pub fn gravar(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&self.carimbo.to_le_bytes());
        b.extend_from_slice(&(self.ofertas.len() as i32).to_le_bytes());
        for o in &self.ofertas {
            b.extend_from_slice(o);
        }
        for c in &self.categorias {
            b.extend_from_slice(&c.nome);
            b.extend_from_slice(&(c.subcategorias.len() as i32).to_le_bytes());
            for s in &c.subcategorias {
                b.extend_from_slice(s);
            }
        }
        b
    }

    /// Os campos do layout, `(nome, tipo, deslocamento)`.
    pub fn campos(&self) -> Vec<(String, TipoDoCampo, usize)> {
        campos_do_formato(self.formato)
    }
}

fn ler_categorias(b: &[u8]) -> Option<Vec<Categoria>> {
    let mut o = 0;
    let mut cats = Vec::with_capacity(8);
    for _ in 0..8 {
        let nome = b.get(o..o + BLOCO)?.to_vec();
        o += BLOCO;
        let nsub = i32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?);
        o += 4;
        if !(0..=1000).contains(&nsub) {
            return None;
        }
        let mut subs = Vec::with_capacity(nsub as usize);
        for _ in 0..nsub {
            subs.push(b.get(o..o + BLOCO)?.to_vec());
            o += BLOCO;
        }
        cats.push(Categoria { nome, subcategorias: subs });
    }
    (o == b.len()).then_some(cats)
}

pub fn campos_do_formato(f: FormatoDoGshop) -> Vec<(String, TipoDoCampo, usize)> {
    use TipoDoCampo::*;
    let mut c: Vec<(String, TipoDoCampo, usize)> = vec![
        ("local_id".into(), I32, 0),
        ("main_type".into(), I32, 4),
        ("sub_type".into(), I32, 8),
        ("icon".into(), Gbk(128), 12),
        ("id".into(), U32, 140),
        ("num".into(), U32, 144),
    ];
    let (passo, nomes): (usize, &[&str]) = match f {
        FormatoDoGshop::V155Vip => (36, &["price", "end_time", "time", "start_time", "type", "day", "status", "flag", "min_vip_level"]),
        FormatoDoGshop::V15xSemVip => (32, &["price", "end_time", "time", "start_time", "type", "day", "status", "flag"]),
        FormatoDoGshop::V126 => (12, &["price", "end_time", "time"]),
    };
    for k in 0..4 {
        for (i, n) in nomes.iter().enumerate() {
            let t = if *n == "type" { I32 } else { U32 };
            c.push((format!("buy_{k}_{n}"), t, 148 + passo * k + 4 * i));
        }
    }
    let depois = 148 + passo * 4;
    match f {
        FormatoDoGshop::V126 => {
            c.push(("status".into(), U32, depois));
            c.push(("desc".into(), Utf16(512), depois + 4));
            c.push(("szName".into(), Utf16(32), depois + 4 + 1024));
        }
        _ => {
            c.push(("desc".into(), Utf16(512), depois));
            let nome = depois + 1024;
            c.push(("szName".into(), Utf16(32), nome));
            let brinde = nome + 64;
            for (i, n) in ["idGift", "iGiftNum", "iGiftTime", "iLogPrice"].iter().enumerate() {
                c.push((n.to_string(), U32, brinde + 4 * i));
            }
            for k in 0..8 {
                c.push((format!("owner_npcs_{k}"), U32, brinde + 16 + 4 * k));
            }
            if f == FormatoDoGshop::V155Vip {
                c.push(("buy_times_limit".into(), I32, brinde + 48));
                c.push(("buy_times_limit_mode".into(), I32, brinde + 52));
            }
        }
    }
    c
}

/// Texto de um `wchar[n]`/`char[n]` até o NUL.
pub fn ler_texto(b: &[u8], tipo: TipoDoCampo) -> String {
    match tipo {
        TipoDoCampo::Utf16(_) => {
            let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
            String::from_utf16_lossy(&u)
        }
        TipoDoCampo::Gbk(_) => {
            let fim = b.iter().position(|&c| c == 0).unwrap_or(b.len());
            encoding_rs::GBK.decode(&b[..fim]).0.into_owned()
        }
        _ => String::new(),
    }
}

/// Os campos de uma oferta como JSON `{nome: valor}`.
pub fn ler_oferta(g: &GshopEditavel, o: &[u8]) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    for (nome, tipo, d) in g.campos() {
        let v = match tipo {
            TipoDoCampo::I32 => serde_json::json!(i32::from_le_bytes(o[d..d + 4].try_into().unwrap())),
            TipoDoCampo::U32 => serde_json::json!(u32::from_le_bytes(o[d..d + 4].try_into().unwrap())),
            TipoDoCampo::Gbk(n) | TipoDoCampo::Utf16(n) => {
                let largura = if matches!(tipo, TipoDoCampo::Utf16(_)) { 2 * n } else { n };
                serde_json::json!(ler_texto(&o[d..d + largura], tipo))
            }
        };
        m.insert(nome, v);
    }
    m
}

/// Troca um campo da oferta. Texto: grava e zera o resto do campo **só se mudou** (o lixo depois do NUL dos textos que
/// não mudam fica); recusa texto que não cabe (com o NUL) ou que o GBK não representa.
pub fn definir_campo(g: &GshopEditavel, o: &mut [u8], nome: &str, valor: &serde_json::Value) -> R<()> {
    let erro = |m: String| ErroDoGshop::Formato(m);
    let (_, tipo, d) = g.campos().into_iter().find(|c| c.0 == nome).ok_or_else(|| erro(format!("campo inexistente: {nome}")))?;
    match tipo {
        TipoDoCampo::I32 => {
            let v = valor.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or_else(|| erro(format!("{nome}: inteiro")))?;
            o[d..d + 4].copy_from_slice(&v.to_le_bytes());
        }
        TipoDoCampo::U32 => {
            let v = valor.as_i64().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| erro(format!("{nome}: inteiro sem sinal")))?;
            o[d..d + 4].copy_from_slice(&v.to_le_bytes());
        }
        TipoDoCampo::Gbk(n) | TipoDoCampo::Utf16(n) => {
            let s = valor.as_str().ok_or_else(|| erro(format!("{nome}: texto")))?;
            let largura = if matches!(tipo, TipoDoCampo::Utf16(_)) { 2 * n } else { n };
            if ler_texto(&o[d..d + largura], tipo) == s {
                return Ok(());
            }
            let bytes: Vec<u8> = match tipo {
                TipoDoCampo::Utf16(_) => s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect(),
                _ => {
                    let (b, _, ruim) = encoding_rs::GBK.encode(s);
                    if ruim {
                        return Err(erro(format!("{nome}: caractere fora do GBK")));
                    }
                    b.into_owned()
                }
            };
            let nul = if matches!(tipo, TipoDoCampo::Utf16(_)) { 2 } else { 1 };
            if bytes.len() + nul > largura {
                return Err(erro(format!("{nome}: no máximo {} caracteres", (largura - nul) / nul)));
            }
            o[d..d + largura].fill(0);
            o[d..d + bytes.len()].copy_from_slice(&bytes);
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn as_lojas_dos_dois_realms_voltam_identicas() {
        let raiz = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut vistos = 0;
        for (realm, arquivo, formato) in [
            ("realm_126", "gshop.data", FormatoDoGshop::V126),
            ("realm_155", "gshop.data", FormatoDoGshop::V155Vip),
            ("realm_155", "gshop1.data", FormatoDoGshop::V155Vip),
            ("realm_155", "gshop2.data", FormatoDoGshop::V155Vip),
        ] {
            let p = raiz.join(realm).join("config").join(arquivo);
            let b = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let g = GshopEditavel::ler(&b).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert_eq!(g.formato, formato, "{}", p.display());
            assert!(g.gravar() == b, "{} não volta idêntico", p.display());
            // Editar preço e nome e voltar ao original devolve o arquivo idêntico.
            if !g.ofertas.is_empty() {
                let mut h = g.clone();
                let antes = ler_oferta(&g, &g.ofertas[0]);
                let mut o = h.ofertas[0].clone();
                definir_campo(&h, &mut o, "buy_0_price", &serde_json::json!(12345)).unwrap();
                definir_campo(&h, &mut o, "szName", &serde_json::json!("Poção do Estúdio")).unwrap();
                h.ofertas[0] = o.clone();
                let lido = ler_oferta(&h, &h.ofertas[0]);
                assert_eq!((lido["buy_0_price"].clone(), lido["szName"].clone()), (serde_json::json!(12345), serde_json::json!("Poção do Estúdio")));
                definir_campo(&h, &mut o, "buy_0_price", &antes["buy_0_price"]).unwrap();
                assert!(definir_campo(&h, &mut o, "szName", &serde_json::json!("x".repeat(40))).is_err(), "não cabe em wchar[32]");
                assert!(definir_campo(&h, &mut o, "nao_existe", &serde_json::json!(1)).is_err());
            }
            vistos += 1;
        }
        assert_eq!(vistos, 4);
    }
}

// ------------------------------------------------------------------ categorias (B257)

/// `wchar[64]` cru com o texto `s` (63 caracteres no máximo, mais o NUL; o resto zerado).
pub fn bloco_de_texto(s: &str) -> R<Vec<u8>> {
    let u: Vec<u16> = s.encode_utf16().collect();
    if u.len() + 1 > BLOCO / 2 || u.contains(&0) {
        return Err(ErroDoGshop::Formato(format!("nome de categoria: no máximo {} caracteres", BLOCO / 2 - 1)));
    }
    let mut b = vec![0u8; BLOCO];
    for (k, c) in u.iter().enumerate() {
        b[2 * k..2 * k + 2].copy_from_slice(&c.to_le_bytes());
    }
    Ok(b)
}

impl GshopEditavel {
    fn usa(&self, principal: i32, sub: Option<i32>) -> usize {
        let (_, _, d_main) = self.campos().into_iter().find(|c| c.0 == "main_type").unwrap();
        let (_, _, d_sub) = self.campos().into_iter().find(|c| c.0 == "sub_type").unwrap();
        self.ofertas
            .iter()
            .filter(|o| {
                let m = i32::from_le_bytes(o[d_main..d_main + 4].try_into().unwrap());
                let s = i32::from_le_bytes(o[d_sub..d_sub + 4].try_into().unwrap());
                m == principal && sub.is_none_or(|x| x == s)
            })
            .count()
    }

    /// Renomeia a categoria `c` (ou a subcategoria `sub` dela). Só reescreve o bloco se o texto muda.
    pub fn renomear_categoria(&mut self, c: usize, sub: Option<usize>, nome: &str) -> R<()> {
        let cat = self.categorias.get_mut(c).ok_or_else(|| ErroDoGshop::Formato(format!("categoria {c} não existe")))?;
        let alvo = match sub {
            None => &mut cat.nome,
            Some(k) => cat.subcategorias.get_mut(k).ok_or_else(|| ErroDoGshop::Formato(format!("subcategoria {k} não existe")))?,
        };
        if ler_texto(alvo, TipoDoCampo::Utf16(64)) != nome {
            *alvo = bloco_de_texto(nome)?;
        }
        Ok(())
    }

    /// Subcategoria nova no fim da categoria `c` (nenhuma oferta muda de índice). Devolve o índice dela.
    pub fn nova_subcategoria(&mut self, c: usize, nome: &str) -> R<usize> {
        let b = bloco_de_texto(nome)?;
        let cat = self.categorias.get_mut(c).ok_or_else(|| ErroDoGshop::Formato(format!("categoria {c} não existe")))?;
        cat.subcategorias.push(b);
        Ok(cat.subcategorias.len() - 1)
    }

    /// Tira a **última** subcategoria da categoria `c`, se nenhuma oferta a usa (`main_type`/`sub_type` são índices:
    /// tirar uma do meio mudaria as seguintes).
    pub fn remover_ultima_subcategoria(&mut self, c: usize) -> R<()> {
        let n = self.categorias.get(c).map(|x| x.subcategorias.len()).ok_or_else(|| ErroDoGshop::Formato(format!("categoria {c} não existe")))?;
        if n == 0 {
            return Err(ErroDoGshop::Formato("a categoria não tem subcategoria".into()));
        }
        let usos = self.usa(c as i32, Some(n as i32 - 1));
        if usos > 0 {
            return Err(ErroDoGshop::Formato(format!("{usos} oferta(s) usam a subcategoria {}", n - 1)));
        }
        self.categorias[c].subcategorias.pop();
        Ok(())
    }
}

#[cfg(test)]
mod testes_das_categorias {
    use super::*;

    #[test]
    fn renomear_acrescentar_e_tirar_subcategoria() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/gshop.data");
        let b = std::fs::read(p).unwrap();
        let mut g = GshopEditavel::ler(&b).unwrap();
        let nome0 = ler_texto(&g.categorias[0].nome, TipoDoCampo::Utf16(64));
        g.renomear_categoria(0, None, &nome0).unwrap();
        assert!(g.gravar() == b, "renomear com o mesmo nome não muda nada");
        g.renomear_categoria(0, Some(0), "Sub do Estúdio").unwrap();
        let k = g.nova_subcategoria(0, "Nova").unwrap();
        let relido = GshopEditavel::ler(&g.gravar()).unwrap();
        assert_eq!(ler_texto(&relido.categorias[0].subcategorias[0], TipoDoCampo::Utf16(64)), "Sub do Estúdio");
        assert_eq!(ler_texto(&relido.categorias[0].subcategorias[k], TipoDoCampo::Utf16(64)), "Nova");
        assert_eq!(relido.ofertas, g.ofertas, "as ofertas não mudam");
        g.remover_ultima_subcategoria(0).unwrap();
        assert!(g.renomear_categoria(0, None, &"x".repeat(64)).is_err());
        // A última subcategoria que tem oferta não sai.
        let com_oferta = (0..g.categorias.len()).find(|&c| {
            let n = g.categorias[c].subcategorias.len();
            n > 0 && g.usa(c as i32, Some(n as i32 - 1)) > 0
        });
        if let Some(c) = com_oferta {
            assert!(g.remover_ultima_subcategoria(c).is_err());
        }
    }
}

// ------------------------------------------------------------------ conferência da oferta (Estúdio G-Shop)

/// O que impede o GS de carregar a loja (`erros`) e o que só fica sem efeito ou não foi conferido (`avisos`).
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Conferencia {
    pub erros: Vec<String>,
    pub avisos: Vec<String>,
}

impl Conferencia {
    pub fn vazia(&self) -> bool {
        self.erros.is_empty() && self.avisos.is_empty()
    }
}

/// Aviso da oferta sem nenhum preço (o cliente a esconde; zerar os preços é como "tirar" uma oferta sem mudar o
/// `goods_index` das seguintes).
pub const DESATIVADA: &str = "oferta desativada: nenhuma opção tem preço, o cliente não a mostra e o GS não a vende";

/// Uma opção de compra lida da oferta (`buy[k]`).
#[derive(Debug, Clone, Copy)]
struct Opcao {
    preco: u32,
    fim: u32,
    validade: u32,
    inicio: u32,
    tipo: i32,
    dias: u32,
    status: u32,
    grupo: u32,
}

/// Janela de venda como o GS a guarda (`sale_time`, `playermall.h:24-29`): 0 sem limite, 1 intervalo, 2 semanal,
/// 3 mensal, com os parâmetros que o `load_malldata` monta (`template/globaldataman.cpp:148-170`).
fn janela(o: &Opcao) -> (i32, i32, i32) {
    match o.tipo {
        0 => (1, o.inicio as i32, o.fim as i32),
        1 => (2, (o.dias & 0x7F) as i32, 0),
        2 => (3, ((o.dias & 0x7FFF_FFFF) << 1) as i32, 0),
        _ => (0, 0, 0),
    }
}

/// `CheckSaleTimeConflictNotExpired` (`playermall.cpp:117-149`), sem o ajuste de fuso (que soma o mesmo valor aos dois).
fn janelas_conflitam(a: (i32, i32, i32), b: (i32, i32, i32)) -> bool {
    if a.0 != b.0 {
        return a.0 != 0 && b.0 != 0;
    }
    match a.0 {
        0 => true,
        1 => {
            let fim_a = if a.2 != 0 { a.2 } else { i32::MAX };
            let fim_b = if b.2 != 0 { b.2 } else { i32::MAX };
            b.1 < fim_a && fim_b > a.1
        }
        2 => (a.1 & 0x7F) & (b.1 & 0x7F) != 0,
        3 => (a.1 & !1) & (b.1 & !1) != 0,
        _ => true,
    }
}

/// Confere uma oferta pelas regras que o GS aplica ao carregar a loja — `load_malldata`
/// (`cgame/gs/template/globaldataman.cpp:135-200`) e `InitMall` (`cgame/gs/playermall.cpp:155-380`), 1.5.5. Qualquer
/// uma delas faz o GS recusar a loja inteira (`throw -3` / `return false`), então são **erros**. `pilha(id)` é o
/// `get_item_pile_limit` do `elements` (`template/itemdataman.cpp:815`): `None` = o item não existe.
///
/// No 1.2.6 (`buy` de 12 B: preço, data, validade) as regras de opção do 1.5.5 não se aplicam; as de item, quantidade
/// e brinde valem como **aviso** (o `InitMall` do `gs` 1.2.6 não foi conferido no binário), salvo o preço da 1ª opção
/// e a existência do item, que o cliente exige para mostrar a oferta.
pub fn conferir_oferta(g: &GshopEditavel, o: &[u8], pilha: &dyn Fn(u32) -> Option<u32>) -> Conferencia {
    let mut c = Conferencia::default();
    let campos = ler_oferta(g, o);
    let n = |k: &str| campos.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    let v126 = g.formato == FormatoDoGshop::V126;
    let regra_do_155 = |c: &mut Conferencia, m: String| if v126 { c.avisos.push(format!("{m} (regra do GS 1.5.5)")) } else { c.erros.push(m) };

    // Aba e subaba: índices na lista de categorias (`globaldataman.h:103-104`).
    let (principal, sub) = (n("main_type"), n("sub_type"));
    match g.categorias.get(principal.max(0) as usize).filter(|_| principal >= 0) {
        None => c.erros.push(format!("aba {principal} não existe (são {} abas)", g.categorias.len())),
        Some(cat) if sub < 0 || sub as usize >= cat.subcategorias.len() => c.erros.push(if cat.subcategorias.is_empty() {
            format!("a aba {principal} não tem subaba: crie uma antes")
        } else {
            format!("subaba {sub} não existe na aba {principal} (são {})", cat.subcategorias.len())
        }),
        _ => {}
    }

    // Item e quantidade (`InitMall`, `playermall.cpp:160-167`).
    let (id, num) = (n("id") as u32, n("num") as u32);
    let pilha_do_item = if id == 0 { None } else { pilha(id) };
    match pilha_do_item {
        None => c.erros.push(format!("o item {id} não existe no elements")),
        Some(p) if num == 0 || num > p.max(1) => regra_do_155(&mut c, format!("quantidade {num} fora da pilha do item (1 a {})", p.max(1))),
        _ => {}
    }

    // Opções de compra.
    if v126 {
        if (0..4).all(|k| n(&format!("buy_{k}_price")) == 0) {
            c.avisos.push(DESATIVADA.into());
        } else if n("buy_0_price") <= 0 {
            c.avisos.push("a 1ª opção não tem preço: o GS para na primeira opção sem preço e não vende nenhuma".into());
        }
        for k in 0..4 {
            if n(&format!("buy_{k}_time")) > i32::MAX as i64 {
                c.avisos.push(format!("opção {}: validade negativa como int (regra do GS 1.5.5)", k + 1));
            }
        }
        return c;
    }
    let opcoes: Vec<Opcao> = (0..4)
        .map(|k| {
            let f = |x: &str| n(&format!("buy_{k}_{x}"));
            Opcao {
                preco: f("price") as u32,
                fim: f("end_time") as u32,
                validade: f("time") as u32,
                inicio: f("start_time") as u32,
                tipo: f("type") as i32,
                dias: f("day") as u32,
                status: f("status") as u32,
                grupo: f("flag") as u32,
            }
        })
        .collect();
    // `load_malldata` lê as 4 opções, com preço ou sem: tipo fora de -1..2 = `throw -3` (`globaldataman.cpp:148-176`).
    for (k, x) in opcoes.iter().enumerate() {
        if !(-1..=2).contains(&x.tipo) {
            c.erros.push(format!("opção {}: tipo de período {} inválido (o GS só aceita -1, 0, 1 e 2)", k + 1, x.tipo));
        }
    }
    // O `InitMall` para na primeira opção sem preço (`playermall.cpp:198`); o nosso GS também só vende a opção se as
    // anteriores têm preço (`pw-gs/src/bus_server/jogo.rs`, `comprar_na_loja_gold`). Oferta sem nenhum preço é a
    // oferta **desativada**: o cliente não a mostra (`CalcBuyType`, `EC_Shop.cpp:172-197`) — 751 das 1726 do
    // `gshop.data` BR estão assim, por isso é aviso e não erro.
    let ativas = opcoes.iter().take_while(|x| x.preco > 0).count();
    if opcoes.iter().all(|x| x.preco == 0) {
        c.avisos.push(DESATIVADA.into());
    } else if ativas == 0 {
        c.avisos.push("a 1ª opção não tem preço: o cliente mostra as outras, mas o GS para na primeira opção sem preço e não vende nenhuma".into());
    }
    for k in ativas.max(1)..4 {
        if opcoes[k].preco > 0 && ativas > 0 {
            c.avisos.push(format!("opção {}: tem preço mas vem depois de uma opção sem preço — o GS a ignora", k + 1));
        }
    }
    let mut com_validade = false;
    let mut com_janela = false;
    for (k, x) in opcoes.iter().take(ativas).enumerate() {
        let r = format!("opção {}", k + 1);
        if (x.validade as i32) < 0 {
            c.erros.push(format!("{r}: validade acima de 2147483647 segundos"));
        }
        if x.validade > 0 {
            com_validade = true;
        }
        let j = janela(x);
        if j.0 != 0 {
            com_janela = true;
        }
        // `sale_time::CheckParam` (`playermall.cpp:97-115`).
        let valida = match j.0 {
            1 => j.1 >= 0 && j.2 >= 0 && (j.1 != 0 || j.2 != 0) && (j.1 == 0 || j.2 == 0 || j.1 < j.2),
            2 => j.1 & 0x7F != 0,
            3 => j.1 & !1 != 0,
            _ => true,
        };
        if !valida {
            c.erros.push(match x.tipo {
                0 => format!("{r}: período por datas precisa de início ou fim, e o início antes do fim"),
                1 => format!("{r}: período semanal sem nenhum dia marcado"),
                _ => format!("{r}: período mensal sem nenhum dia marcado"),
            });
        }
        if (x.grupo as i32) < 0 {
            c.erros.push(format!("{r}: grupo (flag) negativo"));
        }
        if (x.status as i32) < 0 {
            c.erros.push(format!("{r}: selo negativo"));
        }
    }
    if com_validade && pilha_do_item.is_some_and(|p| p != 1) {
        c.erros.push("opção com validade só serve para item que não empilha (pilha 1)".into());
    }
    if com_validade && com_janela {
        c.erros.push("opção com validade não pode ter período de venda (só \"sempre\")".into());
    }
    // Opções do mesmo grupo não podem se sobrepor (`playermall.cpp:266-320`).
    for a in 0..ativas {
        for b in a + 1..ativas {
            let (x, y) = (&opcoes[a], &opcoes[b]);
            if x.grupo != y.grupo {
                continue;
            }
            let conflito = if com_validade { x.validade == y.validade } else { janelas_conflitam(janela(x), janela(y)) };
            if conflito {
                c.erros.push(format!(
                    "opções {} e {} do mesmo grupo {} se sobrepõem ({})",
                    a + 1,
                    b + 1,
                    x.grupo,
                    if com_validade { "mesma validade" } else { "mesmo período de venda" }
                ));
            }
        }
    }

    // Brinde (`playermall.cpp:340-372`).
    let gift = n("idGift") as u32;
    if gift > 0 {
        let (qtd, validade, registro) = (n("iGiftNum"), n("iGiftTime"), n("iLogPrice"));
        match pilha(gift) {
            None => c.erros.push(format!("o brinde {gift} não existe no elements")),
            Some(p) => {
                if qtd <= 0 || qtd > p.max(1) as i64 || qtd > i32::MAX as i64 {
                    c.erros.push(format!("quantidade do brinde {qtd} fora da pilha (1 a {})", p.max(1)));
                }
                if validade > 0 && validade <= i32::MAX as i64 && p != 1 {
                    c.erros.push("brinde com validade só serve para item que não empilha (pilha 1)".into());
                }
            }
        }
        if validade > i32::MAX as i64 {
            c.erros.push("validade do brinde acima de 2147483647 segundos".into());
        }
        if registro > i32::MAX as i64 {
            c.erros.push("preço de registro do brinde negativo".into());
        }
    }
    if g.formato == FormatoDoGshop::V155Vip && n("buy_times_limit_mode") > 0 && n("buy_times_limit") <= 0 {
        c.avisos.push("modo de limite de compras sem quantidade: o cliente não mostra o limite (`DlgQShopItem.cpp:464`)".into());
    }
    c
}

#[cfg(test)]
mod testes_da_conferencia {
    use super::*;

    fn loja(realm: &str, arquivo: &str) -> GshopEditavel {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config").join(arquivo);
        GshopEditavel::ler(&std::fs::read(p).unwrap()).unwrap()
    }

    #[test]
    fn regras_do_gs_na_oferta() {
        let g = loja("realm_155", "gshop.data");
        let pilha_1 = |_: u32| Some(1u32);
        let mut o = g.ofertas.iter().find(|o| ler_oferta(&g, o)["buy_0_price"].as_u64() > Some(0)).unwrap().clone();
        let d = |o: &mut Vec<u8>, k: &str, v: serde_json::Value| definir_campo(&g, o, k, &v).unwrap();
        d(&mut o, "num", serde_json::json!(1));
        for k in 1..4 {
            d(&mut o, &format!("buy_{k}_price"), serde_json::json!(0));
        }
        let base = conferir_oferta(&g, &o, &pilha_1);
        assert!(base.vazia(), "{base:?}");
        // Sem preço nenhum: desativada (aviso, não erro); preço só depois de um buraco: aviso.
        let mut x = o.clone();
        d(&mut x, "buy_0_price", serde_json::json!(0));
        let r = conferir_oferta(&g, &x, &pilha_1);
        assert!(r.erros.is_empty() && r.avisos == vec![DESATIVADA.to_string()], "{r:?}");
        d(&mut x, "buy_2_price", serde_json::json!(100));
        assert!(conferir_oferta(&g, &x, &pilha_1).avisos.iter().any(|e| e.contains("1ª opção")));
        // Tipo 3 derruba o load_malldata mesmo sem preço.
        let mut x = o.clone();
        d(&mut x, "buy_3_type", serde_json::json!(3));
        assert!(conferir_oferta(&g, &x, &pilha_1).erros.iter().any(|e| e.contains("tipo de período 3")));
        // Semanal sem dia, datas invertidas.
        let mut x = o.clone();
        d(&mut x, "buy_0_type", serde_json::json!(1));
        d(&mut x, "buy_0_day", serde_json::json!(0));
        assert!(conferir_oferta(&g, &x, &pilha_1).erros.iter().any(|e| e.contains("semanal")));
        let mut x = o.clone();
        d(&mut x, "buy_0_type", serde_json::json!(0));
        d(&mut x, "buy_0_start_time", serde_json::json!(2000));
        d(&mut x, "buy_0_end_time", serde_json::json!(1000));
        assert!(conferir_oferta(&g, &x, &pilha_1).erros.iter().any(|e| e.contains("datas")));
        // Item inexistente, quantidade acima da pilha, aba inexistente.
        assert!(conferir_oferta(&g, &o, &|_| None).erros.iter().any(|e| e.contains("não existe no elements")));
        let mut x = o.clone();
        d(&mut x, "num", serde_json::json!(5));
        assert!(conferir_oferta(&g, &x, &pilha_1).erros.iter().any(|e| e.contains("pilha")));
        let mut x = o.clone();
        d(&mut x, "main_type", serde_json::json!(8));
        assert!(conferir_oferta(&g, &x, &pilha_1).erros.iter().any(|e| e.contains("aba 8")));
    }

    /// As lojas reais dos dois realms passam (com pilha folgada): as regras não acusam o que o GS aceita.
    #[test]
    fn lojas_reais_sem_erro_de_opcao() {
        for (realm, arquivo) in [("realm_126", "gshop.data"), ("realm_155", "gshop.data"), ("realm_155", "gshop1.data"), ("realm_155", "gshop2.data")] {
            let g = loja(realm, arquivo);
            let pilha = |_: u32| Some(1u32);
            let erros: usize = g.ofertas.iter().map(|o| conferir_oferta(&g, o, &pilha).erros.iter().filter(|e| !e.contains("pilha")).count()).sum();
            assert_eq!(erros, 0, "{realm}/{arquivo}");
        }
    }
}
