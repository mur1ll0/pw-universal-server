//! `tasks.data` editável (B256, Estúdio S7): campos **de tamanho fixo** de uma missão, corrigidos no lugar.
//!
//! O arquivo inteiro fica cru; o leitor do GS (`crate::tasks`) diz onde começam o bloco fixo e o prêmio de cada missão
//! (`TaskTemplate::bloco_fixo_em`/`premio_em`). Trocar um campo de tamanho fixo não muda o tamanho de nada — a tabela de
//! deslocamentos do cabeçalho continua valendo e o resto do arquivo fica byte a byte igual. Campos de tamanho variável
//! (listas de itens, monstros, descrição) ficam para um gravador completo.
//!
//! Deslocamentos: v129 pelas constantes do leitor (`v129::f` e `v129::p`, do `TaskTempl.h` 1.7.2 conferido no arquivo
//! do 1.5.5 BR); v55 pelos do `libtask.so`/`elementclient.exe` 1.2.6 que o leitor usa (spec 03 §3.2). O nome é
//! `wchar[30]` em `+4` nas duas versões, com cada unidade em XOR com `id as u16` (`tasks::texto`).
//!
//! **Textos longos (B262):** descrição, aceitar, recusar e tributo mudam de tamanho. Cada um é `u32` de caracteres
//! (`wcslen`, sem NUL) + as unidades em XOR com `(namechar)m_ID` (`SaveDescriptionBin`/`SaveTributeBin`,
//! `TaskTempl.cpp:4940-5010`; `convert_txt` `:629`). Trocar um reescreve só esse texto e soma a diferença nos
//! deslocamentos da tabela do cabeçalho das missões de topo que começam depois dele; o arquivo é relido.

use crate::tasks::{TaskTemplate, TasksData, TasksError};

/// Em que bloco o campo fica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bloco {
    Fixo,
    Premio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDoCampo {
    U32,
    I32,
    Bool,
    /// O nome: `wchar[30]` em XOR com o id.
    Nome,
}

/// `(nome do campo no TaskTemplate, bloco, tipo, deslocamento)`.
pub type Campo = (&'static str, Bloco, TipoDoCampo, usize);

pub fn campos(versao: u32) -> &'static [Campo] {
    use Bloco::*;
    use TipoDoCampo::*;
    const V129: &[Campo] = &[
        ("name", Fixo, Nome, 4),
        ("limite_de_tempo", Fixo, U32, 73),
        ("frequencia", Fixo, I32, 141),
        ("pode_desistir", Fixo, Bool, 154),
        ("pode_repetir", Fixo, Bool, 155),
        ("entrega_automatica", Fixo, Bool, 226),
        ("npc_que_entrega", Fixo, U32, 238),
        ("npc_que_premia", Fixo, U32, 242),
        ("min_level", Fixo, U32, 333),
        ("max_level", Fixo, U32, 337),
        ("espera", Fixo, U32, 962),
        ("money", Premio, U32, 0),
        ("exp", Premio, U32, 4),
        ("nova_missao", Premio, U32, 13),
        ("sp", Premio, U32, 17),
        ("reputation", Premio, I32, 21),
    ];
    const V55: &[Campo] = &[
        ("name", Fixo, Nome, 4),
        ("limite_de_tempo", Fixo, U32, 0x49),
        ("frequencia", Fixo, I32, 0x66),
        ("pode_desistir", Fixo, Bool, 0x6f),
        ("pode_repetir", Fixo, Bool, 0x70),
        ("entrega_automatica", Fixo, Bool, 0xac),
        ("npc_que_entrega", Fixo, U32, 0xb5),
        ("npc_que_premia", Fixo, U32, 0xb9),
        ("min_level", Fixo, U32, 0xc1),
        ("max_level", Fixo, U32, 0xc5),
        ("espera", Fixo, U32, 0x1e2),
        ("money", Premio, U32, 0),
        ("exp", Premio, U32, 4),
        ("nova_missao", Premio, U32, 8),
        ("sp", Premio, U32, 12),
        ("reputation", Premio, I32, 16),
    ];
    match versao {
        55 => V55,
        129 => V129,
        _ => &[],
    }
}

const BYTES_DO_NOME: usize = 60;

/// Os quatro textos longos, na ordem do arquivo.
pub const TEXTOS: [&str; 4] = ["descricao", "ok", "nao", "tributo"];

/// Posição de cada um dos quatro textos longos: `(onde está o tamanho, quantos caracteres)`.
fn posicoes_dos_textos(bytes: &[u8], t: &TaskTemplate) -> [(usize, usize); 4] {
    let mut o = t.textos_em;
    let mut r = [(0, 0); 4];
    for x in &mut r {
        let n = u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as usize;
        *x = (o, n);
        o += 4 + 2 * n;
    }
    r
}

/// Os quatro textos longos de uma missão lida de `bytes` (sem reler o arquivo), decifrados.
pub fn textos_da_missao(bytes: &[u8], t: &TaskTemplate) -> serde_json::Map<String, serde_json::Value> {
    let chave = t.id as u16;
    let mut m = serde_json::Map::new();
    for (nome, (o, n)) in TEXTOS.iter().zip(posicoes_dos_textos(bytes, t)) {
        let u: Vec<u16> = bytes[o + 4..o + 4 + 2 * n].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) ^ chave).collect();
        m.insert((*nome).into(), String::from_utf16_lossy(&u).into());
    }
    m
}

pub struct TasksEditavel {
    pub bytes: Vec<u8>,
    pub dados: TasksData,
}

impl TasksEditavel {
    pub fn ler(b: &[u8]) -> Result<Self, TasksError> {
        let dados = TasksData::load_from_bytes(b)?;
        Ok(Self { bytes: b.to_vec(), dados })
    }

    pub fn gravar(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn posicao(&self, t: &TaskTemplate, c: &Campo) -> usize {
        match c.1 {
            Bloco::Fixo => t.bloco_fixo_em + c.3,
            Bloco::Premio => t.premio_em + c.3,
        }
    }

    fn posicoes_dos_textos(&self, t: &TaskTemplate) -> [(usize, usize); 4] {
        posicoes_dos_textos(&self.bytes, t)
    }

    /// Os quatro textos longos da missão (`descricao`, `ok`, `nao`, `tributo`), decifrados.
    pub fn textos(&self, id: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        Some(textos_da_missao(&self.bytes, self.dados.tasks.get(&id)?))
    }

    /// Troca um texto longo e corrige a tabela de deslocamentos.
    pub fn definir_texto(&mut self, id: u32, qual: &str, texto: &str) -> Result<(), String> {
        let k = TEXTOS.iter().position(|x| *x == qual).ok_or_else(|| format!("texto inexistente: {qual}"))?;
        if texto.contains('\0') {
            return Err("o texto não pode ter NUL".into());
        }
        let unidades: Vec<u16> = texto.encode_utf16().collect();
        if unidades.len() > 8000 {
            return Err("texto grande demais (guarda do painel: 8000 unidades UTF-16)".into());
        }
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let (o, n) = self.posicoes_dos_textos(t)[k];
        let chave = id as u16;
        let novo: Vec<u8> = unidades.iter().flat_map(|u| (u ^ chave).to_le_bytes()).collect();
        let delta = novo.len() as i64 - 2 * n as i64;
        self.bytes.splice(o + 4..o + 4 + 2 * n, novo);
        self.bytes[o..o + 4].copy_from_slice(&(unidades.len() as u32).to_le_bytes());
        let quantas = u32::from_le_bytes(self.bytes[8..12].try_into().unwrap()) as usize;
        for i in 0..quantas {
            let e = 12 + 4 * i;
            let inicio = u32::from_le_bytes(self.bytes[e..e + 4].try_into().unwrap()) as usize;
            if inicio > o {
                let novo_inicio = u32::try_from(inicio as i64 + delta).map_err(|_| "deslocamento fora de u32")?;
                self.bytes[e..e + 4].copy_from_slice(&novo_inicio.to_le_bytes());
            }
        }
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Os campos editáveis da missão, lidos do arquivo.
    pub fn ler_campos(&self, id: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        let t = self.dados.tasks.get(&id)?;
        let mut m = serde_json::Map::new();
        for c in campos(self.dados.version) {
            let o = self.posicao(t, c);
            let u = |o: usize| u32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap());
            let v = match c.2 {
                TipoDoCampo::U32 => serde_json::json!(u(o)),
                TipoDoCampo::I32 => serde_json::json!(u(o) as i32),
                TipoDoCampo::Bool => serde_json::json!(self.bytes[o] != 0),
                TipoDoCampo::Nome => serde_json::json!(t.name),
            };
            m.insert(c.0.to_string(), v);
        }
        Some(m)
    }

    /// Troca um campo de tamanho fixo no lugar. Recusa campo fora da tabela, valor fora do tipo e nome que não cabe em
    /// `wchar[30]` (com o NUL). Não relê o arquivo: quem chama relê (`TasksEditavel::ler`) para conferir.
    pub fn definir_campo(&mut self, id: u32, nome: &str, valor: &serde_json::Value) -> Result<(), String> {
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let c = campos(self.dados.version).iter().find(|c| c.0 == nome).ok_or_else(|| format!("campo não editável: {nome}"))?;
        let o = self.posicao(t, c);
        match c.2 {
            TipoDoCampo::U32 => {
                let v = valor.as_i64().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| format!("{nome}: inteiro sem sinal"))?;
                self.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            TipoDoCampo::I32 => {
                let v = valor.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or_else(|| format!("{nome}: inteiro"))?;
                self.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            TipoDoCampo::Bool => self.bytes[o] = valor.as_bool().ok_or_else(|| format!("{nome}: verdadeiro ou falso"))? as u8,
            TipoDoCampo::Nome => {
                let s = valor.as_str().ok_or_else(|| format!("{nome}: texto"))?;
                if s == t.name {
                    return Ok(());
                }
                let u: Vec<u16> = s.encode_utf16().collect();
                if u.len() + 1 > BYTES_DO_NOME / 2 || u.contains(&0) {
                    return Err(format!("{nome}: no máximo {} caracteres", BYTES_DO_NOME / 2 - 1));
                }
                let chave = id as u16;
                for k in 0..BYTES_DO_NOME / 2 {
                    let x = u.get(k).copied().unwrap_or(0) ^ chave;
                    self.bytes[o + 2 * k..o + 2 * k + 2].copy_from_slice(&x.to_le_bytes());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Nos dois realms: ler, trocar campos de uma missão (fixo e prêmio, inteiro, bool e nome), reler pelo leitor do
    /// GS (vê os valores novos e nada mais muda), e voltar os valores devolve o arquivo idêntico.
    #[test]
    fn editar_no_lugar_e_voltar_devolve_o_arquivo() {
        for (realm, id) in [("realm_126", 1173u32), ("realm_155", 1173)] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let mut t = TasksEditavel::ler(&b).unwrap();
            assert!(t.gravar() == b);
            let antes = t.ler_campos(id).unwrap_or_else(|| panic!("{realm}: sem a missão {id}"));
            let outra = *t.dados.de_topo.iter().find(|&&x| x != id).unwrap();
            let outra_antes = serde_json::to_value(&t.dados.tasks[&outra]).unwrap();
            for (k, v) in [("min_level", serde_json::json!(7)), ("exp", serde_json::json!(4321)), ("pode_repetir", serde_json::json!(true)), ("name", serde_json::json!("Missão do Estúdio"))] {
                t.definir_campo(id, k, &v).unwrap();
            }
            assert!(t.definir_campo(id, "name", &serde_json::json!("x".repeat(30))).is_err(), "não cabe");
            assert!(t.definir_campo(id, "descricao", &serde_json::json!("x")).is_err(), "tamanho variável não");
            assert_eq!(t.gravar().len(), b.len(), "o tamanho não muda");
            let relido = TasksData::load_from_bytes(&t.gravar()).unwrap();
            let m = &relido.tasks[&id];
            assert_eq!((m.min_level, m.rewards.exp, m.pode_repetir, m.name.as_str()), (7, 4321, true, "Missão do Estúdio"), "{realm}");
            assert_eq!(serde_json::to_value(&relido.tasks[&outra]).unwrap(), outra_antes, "{realm}: a outra missão não muda");
            let mut volta = TasksEditavel::ler(&t.gravar()).unwrap();
            for (k, v) in &antes {
                volta.definir_campo(id, k, v).unwrap();
            }
            // O nome volta pelo texto; os bytes depois do NUL do nome original podem não ser os zeros em XOR que
            // gravamos, então compara-se o arquivo fora do campo do nome.
            let o = t.dados.tasks[&id].bloco_fixo_em + 4;
            let (x, y) = (volta.gravar(), b.clone());
            assert!(x[..o] == y[..o] && x[o + BYTES_DO_NOME..] == y[o + BYTES_DO_NOME..], "{realm}: voltar devolve o resto idêntico");
            assert_eq!(TasksData::load_from_bytes(&x).unwrap().tasks[&id].name, TasksData::load_from_bytes(&y).unwrap().tasks[&id].name);
        }
    }

    /// Nos dois realms: trocar a descrição (maior) e o tributo (vazio) de uma missão de topo do meio e de uma
    /// submissão; o leitor do GS relê tudo, as missões de topo seguintes continuam iguais; voltar = arquivo idêntico.
    #[test]
    fn textos_longos_mudam_de_tamanho_e_voltam() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let topo = t.dados.de_topo[t.dados.de_topo.len() / 2];
            let sub = *t.dados.tasks.values().find(|x| x.parent.is_some() && x.textos_em > 0).map(|x| &x.id).unwrap();
            let ultima = *t.dados.de_topo.last().unwrap();
            let ultima_antes = serde_json::to_value(&t.dados.tasks[&ultima]).unwrap();
            let antes_topo = t.textos(topo).unwrap();
            let antes_sub = t.textos(sub).unwrap();
            assert_eq!(antes_topo["descricao"].as_str().unwrap(), t.dados.tasks[&topo].descricao, "{realm}: decifra igual ao leitor");
            let longa = "Descrição nova do Estúdio — bem mais comprida do que costuma ser, para mudar o tamanho.".repeat(3);
            t.definir_texto(topo, "descricao", &longa).unwrap();
            t.definir_texto(topo, "tributo", "").unwrap();
            t.definir_texto(sub, "ok", "Aceito!").unwrap();
            assert!(t.definir_texto(topo, "titulo", "x").is_err());
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!(r.tasks[&topo].descricao, longa, "{realm}");
            assert_eq!(t.textos(sub).unwrap()["ok"], "Aceito!");
            assert_eq!(serde_json::to_value(&r.tasks[&ultima]).unwrap(), ultima_antes, "{realm}: a última de topo não muda");
            for (k, v) in &antes_topo { t.definir_texto(topo, k, v.as_str().unwrap()).unwrap(); }
            for (k, v) in &antes_sub { t.definir_texto(sub, k, v.as_str().unwrap()).unwrap(); }
            assert!(t.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }
}
