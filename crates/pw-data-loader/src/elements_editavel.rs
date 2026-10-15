//! `elements.data` editável, com ida e volta byte a byte (P4/S1 do Estúdio, B237).
//!
//! Percorre o arquivo pelo mesmo caminho do leitor genérico ([`crate::generic_elements`],
//! a ordem do `elementdataman::load_data` do cliente), mas guarda **bytes**: o cabeçalho, os
//! dois blocos de etiqueta do v156 e o `TALK_PROC` (laço próprio) como pedaços crus, e cada
//! registro de tabela fixa com o seu `record_size`. Gravar sem editar devolve o arquivo
//! idêntico; editar um campo troca só os bytes daquele campo (o layout cobre o registro
//! inteiro — soma dos campos = `record_size` nas 119 tabelas do v7 e nas 231 do v156).
//!
//! Ids: o cliente indexa os registros em espaços (`add_id_index`, `elementdataman.cpp`) e um id
//! repetido no mesmo espaço **sobrescreve em silêncio** o anterior. O mapa tabela → espaço está
//! em `specs/elements_layouts/espacos_de_id.json`; tabela fora dele (as 17 do Lar e afins do
//! v156, sem fonte que as registre) não aceita registro novo.

use crate::generic_elements::{
    avancar, decode_record, detect_header, load_layout, read_talk_proc_table, u32_em, FieldDef,
    FieldValue, GenericElementsError, LayoutCatalog, Record, Result, TableDef, TAG_DEPOIS_DOS_TANQUES,
    TAG_DO_EXPORTADOR,
};
use std::collections::{BTreeMap, HashMap};

const ESPACOS: &str = include_str!("../../../specs/elements_layouts/espacos_de_id.json");

/// Espaço de id de uma tabela (`ESSENCE`, `CONFIG`, `FACE`, `RECIPE`, `ADDON`, `TALK`).
pub fn espaco_de_id(tabela: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Arquivo {
        tabelas: BTreeMap<String, String>,
    }
    thread_local! {
        static MAPA: BTreeMap<String, String> =
            serde_json::from_str::<Arquivo>(ESPACOS).expect("espacos_de_id.json").tabelas;
    }
    MAPA.with(|m| m.get(tabela).cloned())
}

#[derive(Debug, Clone)]
enum Pedaco {
    /// Cabeçalho, blocos de etiqueta, `TALK_PROC`: copiados como estão.
    Cru(Vec<u8>),
    /// Uma tabela de registros de tamanho fixo (índice em `ElementsEditavel::tabelas`).
    Tabela(usize),
    /// O `TALK_PROC`, estruturado em `ElementsEditavel::conversas` (B240).
    Conversas,
}

#[derive(Debug, Clone)]
pub struct TabelaEditavel {
    pub nome: String,
    pub tamanho: usize,
    pub registros: Vec<Vec<u8>>,
    def: TableDef,
}

impl TabelaEditavel {
    /// Deslocamento, tamanho e definição de cada campo.
    pub fn campos(&self) -> Vec<(usize, usize, &FieldDef)> {
        let mut off = 0;
        self.def
            .fields
            .iter()
            .map(|f| {
                let t = tamanho_do_campo(f);
                let r = (off, t, f);
                off += t;
                r
            })
            .collect()
    }

    /// O registro decodificado (os mesmos valores do leitor genérico).
    pub fn ler(&self, indice: usize) -> Result<Record> {
        decode_record(&self.registros[indice], 0, &self.def)
    }

    /// `ID` do registro (primeiro campo em todas as tabelas fixas dos layouts).
    pub fn id(&self, indice: usize) -> Option<i32> {
        let r = self.registros.get(indice)?;
        (self.def.fields.first()?.name == "ID").then(|| i32::from_le_bytes(r[0..4].try_into().unwrap()))
    }

    pub fn indice_do_id(&self, id: i32) -> Option<usize> {
        (0..self.registros.len()).find(|&i| self.id(i) == Some(id))
    }
}

fn tamanho_do_campo(f: &FieldDef) -> usize {
    match f.field_type.as_str() {
        "int32" | "float" => 4,
        _ => f.size.unwrap_or(0),
    }
}

#[derive(Debug, Clone)]
pub struct ElementsEditavel {
    pub versao: u32,
    pedacos: Vec<Pedaco>,
    pub tabelas: Vec<TabelaEditavel>,
    /// O `TALK_PROC` (`crate::conversas`), editável e gravado byte a byte.
    pub conversas: Vec<crate::conversas::Conversa>,
    por_nome: HashMap<String, usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErroDeEdicao {
    #[error("tabela inexistente: {0}")]
    Tabela(String),
    #[error("registro {1} inexistente na tabela {0}")]
    Registro(String, usize),
    #[error("campo inexistente: {0}")]
    Campo(String),
    #[error("valor inválido para {campo} ({tipo}): {motivo}")]
    Valor { campo: String, tipo: String, motivo: String },
    #[error("o id {id} já existe no espaço {espaco} (tabela {tabela}); o cliente sobrescreveria o outro em silêncio")]
    IdRepetido { id: i32, espaco: String, tabela: String },
    #[error("a tabela {0} não tem espaço de id conhecido (sem fonte que a registre): não aceita registro novo")]
    EspacoDesconhecido(String),
    #[error("{0}")]
    Leitura(#[from] GenericElementsError),
}

impl ElementsEditavel {
    /// Lê com o layout dado (o do catálogo do realm) ou o embutido da versão do cabeçalho.
    pub fn ler(buf: &[u8], layout: Option<&LayoutCatalog>) -> Result<Self> {
        let header = detect_header(buf)?;
        let embutido;
        let layout = match layout {
            Some(l) if l.version != header.version => {
                return Err(GenericElementsError::LayoutDeOutraVersao { layout: l.version, arquivo: header.version })
            }
            Some(l) => l,
            None => {
                embutido = load_layout(header.version)?;
                &embutido
            }
        };
        let mut pedacos = vec![Pedaco::Cru(buf[..header.header_size].to_vec())];
        let mut tabelas = Vec::new();
        let mut por_nome = HashMap::new();
        let mut conversas = Vec::new();
        let mut off = header.header_size;
        for t in &layout.tables {
            if t.variable_size {
                if t.name != "TALK_PROC" {
                    return Err(GenericElementsError::UnhandledVariableTable(t.name.clone(), t.index));
                }
                let (cs, fim) = crate::conversas::ler(buf, off)?;
                // O leitor genérico tem de concordar onde a tabela acaba.
                let (_, fim_generico) = read_talk_proc_table(buf, off)?;
                if fim != fim_generico {
                    return Err(GenericElementsError::OutOfBounds(fim, fim_generico));
                }
                conversas = cs;
                pedacos.push(Pedaco::Conversas);
                off = fim;
                continue;
            }
            let tamanho = t
                .record_size
                .ok_or_else(|| GenericElementsError::UnhandledVariableTable(t.name.clone(), t.index))?;
            let n = u32_em(buf, off)?;
            let inicio = off + 4;
            let fim = avancar(buf, inicio, n, tamanho)?;
            let registros = buf[inicio..fim].chunks_exact(tamanho.max(1)).map(<[u8]>::to_vec).collect();
            por_nome.insert(t.name.clone(), tabelas.len());
            pedacos.push(Pedaco::Tabela(tabelas.len()));
            tabelas.push(TabelaEditavel { nome: t.name.clone(), tamanho, registros, def: t.clone() });
            off = fim;
            let bloco = match (header.version, t.name.as_str()) {
                (7, _) => None,
                (_, "ARMORRUNE_ESSENCE") => Some((TAG_DO_EXPORTADOR, true)),
                (_, "WAR_TANKCALLIN_ESSENCE") => Some((TAG_DEPOIS_DOS_TANQUES, false)),
                _ => None,
            };
            if let Some((esperado, com_time_t)) = bloco {
                let achado = u32_em(buf, off)?;
                if achado != esperado {
                    return Err(GenericElementsError::TagInesperado { tabela: t.name.clone(), offset: off, esperado, achado });
                }
                let len = u32_em(buf, off + 4)?;
                let mut fim = avancar(buf, off + 8, len, 1)?;
                if com_time_t {
                    fim = avancar(buf, fim, 1, 4)?;
                }
                pedacos.push(Pedaco::Cru(buf[off..fim].to_vec()));
                off = fim;
            }
        }
        if off != buf.len() {
            return Err(GenericElementsError::NaoTerminaNoFim { fim: off, tamanho: buf.len() });
        }
        Ok(Self { versao: header.version, pedacos, tabelas, conversas, por_nome })
    }

    /// O arquivo inteiro de volta.
    pub fn gravar(&self) -> Vec<u8> {
        let mut saida = Vec::new();
        for p in &self.pedacos {
            match p {
                Pedaco::Cru(b) => saida.extend_from_slice(b),
                Pedaco::Conversas => saida.extend_from_slice(&crate::conversas::gravar(&self.conversas)),
                Pedaco::Tabela(i) => {
                    let t = &self.tabelas[*i];
                    saida.extend_from_slice(&(t.registros.len() as u32).to_le_bytes());
                    for r in &t.registros {
                        saida.extend_from_slice(r);
                    }
                }
            }
        }
        saida
    }

    pub fn tabela(&self, nome: &str) -> std::result::Result<&TabelaEditavel, ErroDeEdicao> {
        self.por_nome.get(nome).map(|&i| &self.tabelas[i]).ok_or_else(|| ErroDeEdicao::Tabela(nome.into()))
    }

    fn tabela_mut(&mut self, nome: &str) -> std::result::Result<&mut TabelaEditavel, ErroDeEdicao> {
        let i = *self.por_nome.get(nome).ok_or_else(|| ErroDeEdicao::Tabela(nome.into()))?;
        Ok(&mut self.tabelas[i])
    }

    /// Troca os bytes de um campo. `Text` em `wstring` (UTF-16LE, completa com zeros), `Raw` em
    /// `string` (bytes GBK, completa com zeros), `Int`/`Float` nos numéricos.
    pub fn definir_campo(
        &mut self,
        tabela: &str,
        indice: usize,
        campo: &str,
        valor: &FieldValue,
    ) -> std::result::Result<(), ErroDeEdicao> {
        let t = self.tabela(tabela)?;
        if indice >= t.registros.len() {
            return Err(ErroDeEdicao::Registro(tabela.into(), indice));
        }
        let (off, tam, def) = t
            .campos()
            .into_iter()
            .find(|(_, _, f)| f.name == campo)
            .map(|(o, s, f)| (o, s, f.clone()))
            .ok_or_else(|| ErroDeEdicao::Campo(campo.into()))?;
        let id_atual = t.id(indice);
        if off == 0 && def.name == "ID" {
            if let FieldValue::Int(novo) = valor {
                if Some(*novo) != id_atual {
                    let espaco = espaco_de_id(tabela).ok_or_else(|| ErroDeEdicao::EspacoDesconhecido(tabela.into()))?;
                    if self.id_em_uso(&espaco, *novo) {
                        return Err(ErroDeEdicao::IdRepetido { id: *novo, espaco, tabela: tabela.into() });
                    }
                }
            }
        }
        let bytes = codificar(&def, tam, valor)?;
        let t = self.tabela_mut(tabela)?;
        t.registros[indice][off..off + tam].copy_from_slice(&bytes);
        Ok(())
    }

    /// Algum registro de alguma tabela do espaço já usa `id`?
    pub fn id_em_uso(&self, espaco: &str, id: i32) -> bool {
        self.tabelas.iter().filter(|t| espaco_de_id(&t.nome).as_deref() == Some(espaco)).any(|t| t.indice_do_id(id).is_some())
    }

    /// Maior id + 1 no espaço da tabela (Estúdio §3.1).
    pub fn proximo_id(&self, tabela: &str) -> std::result::Result<i32, ErroDeEdicao> {
        let espaco = espaco_de_id(tabela).ok_or_else(|| ErroDeEdicao::EspacoDesconhecido(tabela.into()))?;
        let maior = self
            .tabelas
            .iter()
            .filter(|t| espaco_de_id(&t.nome).as_deref() == Some(espaco.as_str()))
            .flat_map(|t| (0..t.registros.len()).filter_map(|i| t.id(i)))
            .max()
            .unwrap_or(0);
        Ok(maior + 1)
    }

    /// Clona o registro `indice` com o id novo no fim da tabela. Devolve o índice do clone.
    pub fn clonar(&mut self, tabela: &str, indice: usize, novo_id: i32) -> std::result::Result<usize, ErroDeEdicao> {
        let espaco = espaco_de_id(tabela).ok_or_else(|| ErroDeEdicao::EspacoDesconhecido(tabela.into()))?;
        if self.id_em_uso(&espaco, novo_id) {
            return Err(ErroDeEdicao::IdRepetido { id: novo_id, espaco, tabela: tabela.into() });
        }
        let t = self.tabela_mut(tabela)?;
        let mut copia = t.registros.get(indice).cloned().ok_or_else(|| ErroDeEdicao::Registro(tabela.into(), indice))?;
        copia[0..4].copy_from_slice(&novo_id.to_le_bytes());
        t.registros.push(copia);
        Ok(t.registros.len() - 1)
    }

    pub fn remover(&mut self, tabela: &str, indice: usize) -> std::result::Result<Vec<u8>, ErroDeEdicao> {
        let t = self.tabela_mut(tabela)?;
        if indice >= t.registros.len() {
            return Err(ErroDeEdicao::Registro(tabela.into(), indice));
        }
        Ok(t.registros.remove(indice))
    }
}

fn codificar(def: &FieldDef, tam: usize, valor: &FieldValue) -> std::result::Result<Vec<u8>, ErroDeEdicao> {
    let erro = |motivo: &str| ErroDeEdicao::Valor { campo: def.name.clone(), tipo: def.field_type.clone(), motivo: motivo.into() };
    match (def.field_type.as_str(), valor) {
        ("int32", FieldValue::Int(v)) => Ok(v.to_le_bytes().to_vec()),
        ("float", FieldValue::Float(v)) if v.is_finite() => Ok(v.to_le_bytes().to_vec()),
        ("float", FieldValue::Float(_)) => Err(erro("número não finito")),
        ("wstring", FieldValue::Text(s)) => {
            let mut b: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            if b.len() > tam {
                return Err(erro(&format!("cabe {} caracteres", tam / 2)));
            }
            b.resize(tam, 0);
            Ok(b)
        }
        ("string", FieldValue::Raw(r)) => {
            if r.len() > tam || r.contains(&0) {
                return Err(erro(&format!("cabe {tam} bytes, sem NUL no meio")));
            }
            let mut b = r.clone();
            b.resize(tam, 0);
            Ok(b)
        }
        _ => Err(erro("tipo do valor não combina com o do campo")),
    }
}
