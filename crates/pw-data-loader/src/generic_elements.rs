//! Leitor genérico de `elements.data`, dirigido pelo catálogo de layouts em
//! `specs/elements_layouts/`. É a contraparte em Rust de
//! `specs/elements_layouts/pw_elements_reader.py` — os dois devem seguir o mesmo
//! algoritmo (ver o README daquela pasta para a arquitetura completa: detecção de versão
//! pelo cabeçalho, catálogo de JSON por build, overrides por realm).
//!
//! `GameDataManager::load_from_directory` (`crates/pw-data-loader/src/manager.rs`) usa este
//! leitor para qualquer `elements.data` cuja versão o catálogo cobre (hoje, só v156) — é o
//! caminho real que o `pw-gs` percorre ao subir um realm. [`crate::elements::ElementsData`]
//! (o leitor tipado antigo, `TABLE_SIZES_V7`, 118 tabelas) continua existindo só como
//! **fallback para versões que o catálogo ainda não cobre** (1.2.6, v7) — não porque seja
//! preferido. Ver `docs/ESTADO_E_RETOMADA.md`, seção "Prioridade atual", para o porquê da
//! migração (o leitor tipado nunca tinha terminado de carregar o `elements.data` real do
//! 1.5.5; este aqui já foi validado byte a byte contra as 231 tabelas).
//!
//! # O algoritmo é o do cliente, sem busca
//!
//! `elementdataman::load_data` (`EvolvedPWClient/ElementClient/CCommon/elementdataman.cpp:3879`)
//! lê, em ordem: versão, `time_t`, e cada tabela como `count` + `count × sizeof(T)`. Há só
//! três exceções, e as três estão aqui:
//!
//! 1. Depois de `ARMORRUNE_ESSENCE`: `tag` (`0xab7689dd`), `len`, `len` bytes com o nome da
//!    máquina que exportou, e outro `time_t` (`elementdataman.cpp:4009-4016`).
//! 2. Depois de `WAR_TANKCALLIN_ESSENCE`: `tag` (`0xee35679f`), `len` e `len` bytes
//!    (`elementdataman.cpp:4122-4124`).
//! 3. `TALK_PROC`, com laço próprio.
//!
//! **Até 2026-09-12 as duas primeiras não eram lidas.** O leitor tentava a posição
//! ingênua, pontuava o primeiro registro por "plausibilidade" e, se não gostasse, buscava
//! numa janela de bytes — e dez `skip`/`count`/`abs_count_off` por arquivo remendavam o
//! resto (`specs/elements_155/realm_155*_overrides.json`). O "skip de 19 bytes sem
//! explicação" antes de `SKILLTOME_SUB_TYPE` era o bloco 1 (`4 + 4 + 7 + 4`); as âncoras
//! absolutas compensavam o bloco 2. Como as âncoras eram posições de **um** arquivo, o
//! `elements.data` v156 do `realm_155BR` (outro arquivo) lia só 99 das 231 tabelas.
//!
//! Com os dois blocos lidos, o `realm_155BR` (v156, 55.442.775 bytes) e o `realm_155`
//! (v159, 55.170.911 bytes) fecham **exatamente** no último byte, sem override nenhum —
//! e cada posição que as âncoras antigas diziam cai no mesmo lugar. Arquivo que não fecha
//! no último byte é erro ([`GenericElementsError::NaoTerminaNoFim`]), não dado torto.
//!
//! O layout é embutido no binário em tempo de compilação (`include_str!`), então carregar
//! um `elements.data` não depende de `specs/` estar disponível em tempo de execução.

use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

const V156_LAYOUT_JSON: &str = include_str!("../../../specs/elements_layouts/v156.json");
const V159_LAYOUT_JSON: &str = include_str!("../../../specs/elements_layouts/v159.json");

/// `tag` gravado por `elementdataman::save_data` antes do nome da máquina exportadora
/// (`gs/template/elementdataman.cpp:3608`).
const TAG_DO_EXPORTADOR: u32 = 0xab76_89dd;
/// `tag2` gravado depois de `WAR_TANKCALLIN_ESSENCE` (`gs/template/elementdataman.cpp:3725`).
const TAG_DEPOIS_DOS_TANQUES: u32 = 0xee35_679f;

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

    #[error("depois da tabela '{tabela}' devia vir o tag {esperado:#x} no offset {offset}, veio {achado:#x}")]
    TagInesperado { tabela: String, offset: usize, esperado: u32, achado: u32 },

    #[error("as tabelas terminaram no offset {fim}, mas o arquivo tem {tamanho} bytes -- o layout não é o deste arquivo")]
    NaoTerminaNoFim { fim: usize, tamanho: usize },

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
        159 => Ok(serde_json::from_str(V159_LAYOUT_JSON)?),
        v => Err(GenericElementsError::UnsupportedVersion(v)),
    }
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

fn u32_em(buf: &[u8], off: usize) -> Result<u32> {
    buf.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(GenericElementsError::OutOfBounds(off + 4, buf.len()))
}

/// Avança `n × tamanho` bytes, recusando o que passaria do fim do arquivo.
fn avancar(buf: &[u8], off: usize, n: u32, tamanho: usize) -> Result<usize> {
    (n as usize)
        .checked_mul(tamanho)
        .and_then(|b| off.checked_add(b))
        .filter(|&fim| fim <= buf.len())
        .ok_or(GenericElementsError::OutOfBounds(usize::MAX, buf.len()))
}

/// `talk_proc::load` (`CCommon/ExpTypes.h:3872`): `id_talk`, `text[64]`, as janelas e, em
/// cada uma, `id`, `id_parent`, o texto e as opções (`id` + `text[64]` + `param`).
fn read_talk_proc_table(buf: &[u8], off: usize) -> Result<(Vec<Record>, usize)> {
    let count = u32_em(buf, off)?;
    let mut cur = off + 4;
    let mut talk_procs = Vec::with_capacity((count as usize).min(buf.len() / 136));
    for _ in 0..count {
        let id_talk = u32_em(buf, cur)? as i32;
        cur += 4;
        let (text, next) = read_wstr(buf, cur, 64)?;
        cur = next;
        let num_window = u32_em(buf, cur)?;
        cur += 4;
        for _ in 0..num_window {
            cur = avancar(buf, cur, 1, 8)?; // id + id_parent
            let talk_text_len = u32_em(buf, cur)?;
            cur = avancar(buf, cur + 4, talk_text_len, 2)?;
            let num_option = u32_em(buf, cur)?;
            cur = avancar(buf, cur + 4, num_option, 4 + 64 * 2 + 4)?;
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

#[derive(Debug, Clone)]
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

/// Carrega um `elements.data` inteiro, exatamente como `elementdataman::load_data` — ver a
/// nota do módulo.
pub fn load_elements_data(buf: &[u8]) -> Result<GenericElementsData> {
    let header = detect_header(buf)?;
    let layout = load_layout(header.version)?;

    let mut result = HashMap::with_capacity(layout.tables.len());
    let mut off = header.header_size;

    for table in &layout.tables {
        if table.variable_size {
            if table.name != "TALK_PROC" {
                return Err(GenericElementsError::UnhandledVariableTable(
                    table.name.clone(),
                    table.index,
                ));
            }
            let (records, next_off) = read_talk_proc_table(buf, off)?;
            result.insert(table.name.clone(), records);
            off = next_off;
            continue;
        }

        let size = table.record_size.ok_or_else(|| {
            GenericElementsError::UnhandledVariableTable(table.name.clone(), table.index)
        })?;
        let count = u32_em(buf, off)?;
        let inicio = off + 4;
        let fim = avancar(buf, inicio, count, size)?;
        let mut records = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            records.push(decode_record(buf, inicio + i * size, table)?);
        }
        result.insert(table.name.clone(), records);
        off = fim;

        // Os dois blocos que não são tabela (elementdataman.cpp:4009-4016 e 4122-4124).
        let bloco = match table.name.as_str() {
            "ARMORRUNE_ESSENCE" => Some((TAG_DO_EXPORTADOR, true)),
            "WAR_TANKCALLIN_ESSENCE" => Some((TAG_DEPOIS_DOS_TANQUES, false)),
            _ => None,
        };
        if let Some((esperado, com_time_t)) = bloco {
            let achado = u32_em(buf, off)?;
            if achado != esperado {
                return Err(GenericElementsError::TagInesperado {
                    tabela: table.name.clone(),
                    offset: off,
                    esperado,
                    achado,
                });
            }
            let len = u32_em(buf, off + 4)?;
            off = avancar(buf, off + 8, len, 1)?;
            if com_time_t {
                off = avancar(buf, off, 1, 4)?;
            }
        }
    }

    if off != buf.len() {
        return Err(GenericElementsError::NaoTerminaNoFim { fim: off, tamanho: buf.len() });
    }

    Ok(GenericElementsData {
        version: header.version,
        tables: result,
    })
}

/// O mesmo que [`load_elements_data`]. Mantido pelo nome: era a variante que escolhia os
/// overrides pela versão, e overrides não existem mais.
pub fn load_elements_data_auto(buf: &[u8]) -> Result<GenericElementsData> {
    load_elements_data(buf)
}
