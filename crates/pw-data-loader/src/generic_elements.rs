//! Leitor genérico de `elements.data`, dirigido pelo catálogo de layouts em
//! `specs/elements_layouts/`. É a contraparte em Rust de
//! `specs/elements_layouts/pw_elements_reader.py` — os dois devem seguir o mesmo
//! algoritmo (ver o README daquela pasta para a arquitetura completa: detecção de versão
//! pelo cabeçalho, catálogo de JSON por build, overrides por realm).
//!
//! Este módulo é **aditivo**: não substitui [`crate::elements::ElementsData`] (usado hoje
//! por `pw-gs`), que continua funcionando como está. `GenericElementsData` existe pra
//! consumidores que precisam de TODAS as 231 tabelas (não só as poucas que `ElementsData`
//! tipa manualmente) sem reescrever a tabela de tamanhos uma terceira vez — o candidato
//! natural é uma futura API administrativa em Rust, ou uma futura migração do `pw-gs` pra
//! este leitor, que ainda não foi feita (ver `specs/elements_155/README.md`, seção
//! "Próximo passo", item sobre decidir se os overrides são do formato ou do arquivo antes
//! de generalizar mais).
//!
//! O layout de v156 e os overrides do realm 155 são embutidos no binário em tempo de
//! compilação (`include_str!`), então carregar um `elements.data` não depende de
//! `specs/` estar disponível em tempo de execução (diferente do lado Python, que hoje
//! precisa localizar a pasta -- ver a nota de empacotamento em
//! `web-admin/backend/elements_decoder.py`).

use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

const V156_LAYOUT_JSON: &str = include_str!("../../../specs/elements_layouts/v156.json");
const REALM_155_OVERRIDES_JSON: &str =
    include_str!("../../../specs/elements_155/realm_155_overrides.json");

#[derive(Error, Debug)]
pub enum GenericElementsError {
    #[error("cabeçalho de elements.data não reconhecido (esperava 0x3000<<16 | build): {0:#x}")]
    UnrecognizedHeader(u32),

    #[error("arquivo pequeno demais pra ter cabeçalho ({0} bytes)")]
    FileTooSmall(usize),

    #[error("versão {0} não tem layout no catálogo -- gere specs/elements_layouts/v{0}.json (ver o README daquela pasta)")]
    UnsupportedVersion(u32),

    #[error("tabela '{0}' (índice {1}) é de tamanho variável mas não tem leitor implementado")]
    UnhandledVariableTable(String, usize),

    #[error("não achei alinhamento plausível pra tabela '{0}' (índice {1}) perto do offset {2}")]
    NoPlausibleAlignment(String, usize, usize),

    #[error("offset {0} fora dos limites do arquivo ({1} bytes)")]
    OutOfBounds(usize, usize),

    #[error("erro ao decodificar o catálogo de layout: {0}")]
    LayoutParse(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GenericElementsError>;

// =============================================================================
// Catálogo de layout (specs/elements_layouts/vNNN.json)
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TableDef {
    pub index: usize,
    pub name: String,
    #[serde(default)]
    pub variable_size: bool,
    pub record_size: Option<usize>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutCatalog {
    pub version: u32,
    pub table_count: usize,
    pub tables: Vec<TableDef>,
}

// =============================================================================
// Overrides por realm (specs/elements_155/realm_155_overrides.json)
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct TableOverride {
    pub skip: Option<i64>,
    pub abs_count_off: Option<usize>,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RealmOverrides {
    pub version: u32,
    pub overrides: HashMap<String, TableOverride>,
}

// =============================================================================
// Valor de campo decodificado
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Int(i32),
    Float(f32),
    Text(String),
    Raw(Vec<u8>),
}

impl FieldValue {
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            FieldValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            FieldValue::Text(v) => Some(v),
            _ => None,
        }
    }
}

pub type Record = HashMap<String, FieldValue>;

// =============================================================================
// Detecção de cabeçalho -- ver docstring de detect_header() em pw_elements_reader.py
// =============================================================================

#[derive(Debug, Clone, Copy)]
pub struct HeaderInfo {
    pub version: u32,
    pub raw_version: u32,
    pub header_size: usize,
    pub build_timestamp: Option<u32>,
}

pub fn detect_header(buf: &[u8]) -> Result<HeaderInfo> {
    if buf.len() < 8 {
        return Err(GenericElementsError::FileTooSmall(buf.len()));
    }
    let raw_version = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if (raw_version >> 16) != 0x3000 {
        return Err(GenericElementsError::UnrecognizedHeader(raw_version));
    }
    let next_u32 = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if next_u32 > 900_000_000 {
        Ok(HeaderInfo {
            version: raw_version & 0xFFFF,
            raw_version,
            header_size: 8,
            build_timestamp: Some(next_u32),
        })
    } else {
        Ok(HeaderInfo {
            version: raw_version & 0xFFFF,
            raw_version,
            header_size: 4,
            build_timestamp: None,
        })
    }
}

// =============================================================================
// Catálogo embutido -- adicionar aqui quando um v<N>.json novo for gerado
// =============================================================================

pub fn load_layout(version: u32) -> Result<LayoutCatalog> {
    match version {
        156 => Ok(serde_json::from_str(V156_LAYOUT_JSON)?),
        v => Err(GenericElementsError::UnsupportedVersion(v)),
    }
}

pub fn load_realm_155_overrides() -> RealmOverrides {
    serde_json::from_str(REALM_155_OVERRIDES_JSON)
        .expect("realm_155_overrides.json embutido no binário deve ser válido")
}

// =============================================================================
// Decodificação de um registro, dirigida pelos campos do layout
// =============================================================================

pub fn decode_record(buf: &[u8], off: usize, table: &TableDef) -> Result<Record> {
    let mut record = Record::new();
    let mut cur = off;
    for field in &table.fields {
        let (value, next) = decode_one_field(buf, cur, field)?;
        record.insert(field.name.clone(), value);
        cur = next;
    }
    Ok(record)
}

fn decode_one_field(buf: &[u8], off: usize, field: &FieldDef) -> Result<(FieldValue, usize)> {
    let check = |end: usize| -> Result<()> {
        if end > buf.len() {
            Err(GenericElementsError::OutOfBounds(end, buf.len()))
        } else {
            Ok(())
        }
    };
    match field.field_type.as_str() {
        "int32" => {
            check(off + 4)?;
            let v = i32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
            Ok((FieldValue::Int(v), off + 4))
        }
        "float" => {
            check(off + 4)?;
            let v = f32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
            Ok((FieldValue::Float(v), off + 4))
        }
        "wstring" => {
            let size = field.size.unwrap_or(0);
            check(off + size)?;
            let raw = &buf[off..off + size];
            let u16s: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            Ok((FieldValue::Text(String::from_utf16_lossy(&u16s)), off + size))
        }
        "string" => {
            let size = field.size.unwrap_or(0);
            check(off + size)?;
            let raw = &buf[off..off + size];
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            Ok((FieldValue::Raw(raw[..end].to_vec()), off + size))
        }
        other => panic!("tipo de campo desconhecido no layout: {other}"),
    }
}

fn plausibility_score(record: &Record, table: &TableDef) -> f64 {
    let mut score = 0.0;
    for field in &table.fields {
        let Some(v) = record.get(&field.name) else { continue };
        match v {
            FieldValue::Int(n) => {
                if (-1_000_000..=100_000_000).contains(n) {
                    score += 1.0;
                } else if *n == 0 {
                    score += 0.5;
                } else {
                    score -= 2.0;
                }
            }
            FieldValue::Float(f) => {
                if *f == 0.0 {
                    score += 0.5;
                } else if f.is_finite() && f.abs() < 1e7 {
                    score += 1.0;
                } else {
                    score -= 2.0;
                }
            }
            FieldValue::Text(t) => {
                // Equivalente a `str.isprintable()` do Python: qualquer caractere que não
                // seja de controle conta como "legível" -- cobre cirílico, CJK, latim
                // acentuado, etc., não só ASCII. Um `is_ascii_graphic()` aqui (bug já
                // corrigido) rejeitava cirílico/CJK como "ilegível" e derrubava o score de
                // toda tabela com texto não-ASCII abaixo da barra de aceite.
                if t.is_empty() {
                    score += 0.2;
                } else if t.chars().all(|c| !c.is_control()) {
                    score += 1.5;
                } else {
                    score -= 1.5;
                }
            }
            FieldValue::Raw(_) => {}
        }
    }
    score
}

/// Acha `(offset_do_campo_count, count, bytes_consumidos)` pra uma tabela de tamanho fixo,
/// tentando a posição ingênua primeiro (ver `_try_table` em `pw_elements_reader.py` para o
/// histórico completo do porque este é o algoritmo, não uma busca gulosa simples).
fn try_table(buf: &[u8], off: usize, table: &TableDef) -> Option<(usize, u32, usize)> {
    let size = table.record_size?;
    let filesize = buf.len();

    let eval_at = |c_off: usize| -> Option<f64> {
        if c_off + 4 > filesize {
            return None;
        }
        let count = u32::from_le_bytes(buf[c_off..c_off + 4].try_into().unwrap());
        if count > 200_000 {
            return None;
        }
        let rec_start = c_off + 4;
        if rec_start + size * count as usize > filesize {
            return None;
        }
        if count == 0 {
            return Some(0.1);
        }
        let rec = decode_record(buf, rec_start, table).ok()?;
        Some(plausibility_score(&rec, table))
    };

    let good_bar = table.fields.len() as f64 * 0.6;
    if let Some(sc) = eval_at(off) {
        let count_at_off = u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
        if count_at_off == 0 || sc > good_bar {
            return Some((off, count_at_off, 4 + count_at_off as usize * size));
        }
    }

    let mut best: Option<(f64, usize)> = None;
    let neg_window = 64usize;
    let window = 1024usize;
    let start = off.saturating_sub(neg_window);
    let end = off + window;
    for c_off in start..end {
        if let Some(sc) = eval_at(c_off) {
            if best.map(|(b, _)| sc > b).unwrap_or(true) {
                best = Some((sc, c_off));
            }
        }
    }
    let (_, c_off) = best?;
    let count = u32::from_le_bytes(buf[c_off..c_off + 4].try_into().unwrap());
    Some((c_off, count, 4 + count as usize * size))
}

// =============================================================================
// TALK_PROC -- a única tabela de tamanho variável em elements.data (v156)
// =============================================================================

fn read_wstr(buf: &[u8], off: usize, nchars: usize) -> Result<(String, usize)> {
    let size = nchars * 2;
    if off + size > buf.len() {
        return Err(GenericElementsError::OutOfBounds(off + size, buf.len()));
    }
    let raw = &buf[off..off + size];
    let u16s: Vec<u16> = raw
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    Ok((String::from_utf16_lossy(&u16s), off + size))
}

fn read_talk_proc_table(buf: &[u8], off: usize) -> Result<(Vec<Record>, usize)> {
    let count = u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
    let mut cur = off + 4;
    let mut talk_procs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id_talk = i32::from_le_bytes(buf[cur..cur + 4].try_into().unwrap());
        cur += 4;
        let (text, next) = read_wstr(buf, cur, 64)?;
        cur = next;
        let num_window = i32::from_le_bytes(buf[cur..cur + 4].try_into().unwrap());
        cur += 4;
        for _ in 0..num_window {
            cur += 8; // id + id_parent
            let talk_text_len =
                i32::from_le_bytes(buf[cur..cur + 4].try_into().unwrap()) as usize;
            cur += 4;
            cur += talk_text_len * 2;
            let num_option = i32::from_le_bytes(buf[cur..cur + 4].try_into().unwrap());
            cur += 4;
            cur += num_option as usize * (4 + 64 * 2 + 4);
        }
        let mut rec = Record::new();
        rec.insert("id_talk".into(), FieldValue::Int(id_talk));
        rec.insert("text".into(), FieldValue::Text(text));
        talk_procs.push(rec);
    }
    Ok((talk_procs, cur))
}

// =============================================================================
// Orquestrador de topo
// =============================================================================

#[derive(Debug)]
pub struct GenericElementsData {
    pub version: u32,
    pub tables: HashMap<String, Vec<Record>>,
}

impl GenericElementsData {
    pub fn get(&self, table_name: &str) -> &[Record] {
        self.tables
            .get(table_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Carrega um `elements.data` inteiro. `overrides` é opcional -- passar `None` funciona
/// pra qualquer arquivo bem formado da versão suportada; passar
/// `Some(load_realm_155_overrides())` aplica as correções conhecidas de
/// `data/realm_155/config/elements.data` especificamente (ver o README de
/// `specs/elements_layouts/` para o porquê disso ser opcional, não parte do layout).
pub fn load_elements_data(buf: &[u8], overrides: Option<&RealmOverrides>) -> Result<GenericElementsData> {
    let header = detect_header(buf)?;
    let layout = load_layout(header.version)?;

    let mut result = HashMap::with_capacity(layout.tables.len());
    let mut off = header.header_size;

    for table in &layout.tables {
        let override_def = overrides.and_then(|o| o.overrides.get(&table.index.to_string()));

        if table.variable_size {
            if table.name == "TALK_PROC" {
                let (records, next_off) = read_talk_proc_table(buf, off)?;
                result.insert(table.name.clone(), records);
                off = next_off;
                continue;
            }
            return Err(GenericElementsError::UnhandledVariableTable(
                table.name.clone(),
                table.index,
            ));
        }

        let size = table.record_size.unwrap();
        let (c_off, count) = if let Some(ov) = override_def {
            let c_off = ov
                .abs_count_off
                .unwrap_or_else(|| (off as i64 + ov.skip.unwrap_or(0)) as usize);
            (c_off, ov.count as u32)
        } else {
            let (c_off, count, _consumed) = try_table(buf, off, table).ok_or_else(|| {
                GenericElementsError::NoPlausibleAlignment(table.name.clone(), table.index, off)
            })?;
            (c_off, count)
        };

        let mut records = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            let rec_off = c_off + 4 + i * size;
            records.push(decode_record(buf, rec_off, table)?);
        }
        result.insert(table.name.clone(), records);
        off = c_off + 4 + count as usize * size;
    }

    Ok(GenericElementsData {
        version: header.version,
        tables: result,
    })
}
