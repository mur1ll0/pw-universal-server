//! `extra_drops.sev` — o **drop global** do GS: listas de item que valem para muitos monstros de uma vez
//! (as "世界掉落" por faixa de nível, livro de habilidade de chefe…), além do `drop_matters` de cada um.
//!
//! # Quem lê
//!
//! O GS, ao subir: `[Template] DropData = extra_drops.sev` (`gs.conf:59`) → `load_extra_drop_tables`
//! (`cgame/gs/worldmanager.cpp:884-888`) → `drop_template::LoadDropList` (`global_drop.cpp:8-61`). Se uma das
//! duas falhar, o GS **não sobe** (`return -5`/`-6`) — aqui isso vira `Err`.
//!
//! # Formato (`template/extra_drop_table.cpp:7-40`)
//!
//! ```text
//! u32 versão                         // EXTRA_DROP_TABLE_VERSION = 1 (extra_drop_table.h:7); outra → recusa
//! i32 quantidade
//! quantidade × {
//!     i32 n;  n × u32 id_monstro
//!     256 × { u32 id_item; f32 chance }
//!     char[128] nome                 // GBK
//!     i32 tipo                       // EDT_TYPE: 1 substitui o drop do monstro, 2 soma (extra_drop_table.h:8-12)
//!     8 × f32 chance de cair 0..7 itens
//! }
//! ```
//!
//! O original lê com `fread` sem conferir o tamanho; aqui o arquivo tem de **fechar no último byte**.
//! `realm_126`: 39.352 B, 15 tabelas, 1.526 monstros, todas do tipo 2. `realm_155`: 2.228 B, 1 tabela vazia.
//!
//! # O que `LoadDropList` aceita
//!
//! * tipo fora de 1/2 → o GS não sobe;
//! * a lista de itens vai até o primeiro `id == 0` (`global_drop.cpp:26`);
//! * lista vazia → a tabela é ignorada (`:36-40`);
//! * soma das chances (somadas em `f32`) com `|1 − soma| > 1e-6` → ignorada (`:42-46`);
//! * o mesmo monstro em duas tabelas aceitas → o GS não sobe (`:48-55`).

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DropGlobalError {
    #[error("extra_drops.sev: {0}")]
    Formato(String),
}

pub type Result<T> = std::result::Result<T, DropGlobalError>;

/// `EXTRA_DROP_TABLE_VERSION` (`extra_drop_table.h:7`).
pub const VERSAO: u32 = 1;
/// Tamanho fixo da lista de itens de uma tabela (`drop_items[256]`, `extra_drop_table.h:27`).
pub const ITENS_POR_TABELA: usize = 256;

/// `EDT_TYPE` (`extra_drop_table.h:8-12`). O que `gnpc_imp::DropItem` faz depois do drop global
/// (`npc.cpp:2721-2725`, `:2646`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoDeDropGlobal {
    /// `EDT_TYPE_REPLACE` = 1: o drop do `drop_matters` **não** acontece; o dinheiro sai daqui, um sorteio só.
    Substitui,
    /// `EDT_TYPE_ADDON` = 2: depois deste, o drop normal do monstro acontece também.
    Soma,
}

/// Uma tabela aceita pelo `LoadDropList`.
#[derive(Debug, Clone, PartialEq)]
pub struct TabelaDeDropGlobal {
    /// A posição dela no arquivo.
    pub indice: usize,
    pub nome: String,
    pub tipo: TipoDeDropGlobal,
    pub monstros: Vec<u32>,
    /// `(id, chance)` até o primeiro id 0; as chances somam 1.
    pub itens: Vec<(u32, f32)>,
    /// `drop_num_probability`: chance de caírem 0, 1, … 7 itens.
    pub chance_de_quantos: [f32; 8],
}

impl TabelaDeDropGlobal {
    /// Chance de o item cair **pelo menos uma vez** numa morte que passou no teste de `drop_adj`:
    /// `Σ P(k) · (1 − (1 − p)^k)` — cada um dos `k` itens é um sorteio independente na lista
    /// (`drop_template::GenerateItem`, `global_drop.h:41-58`).
    pub fn chance_por_morte(&self, id_item: u32) -> f64 {
        let p: f64 = self.itens.iter().filter(|(i, _)| *i == id_item).map(|(_, c)| *c as f64).sum();
        self.chance_de_quantos.iter().enumerate().map(|(k, pk)| *pk as f64 * (1.0 - (1.0 - p).powi(k as i32))).sum()
    }
}

/// O `extra_drops.sev` como o GS o deixa depois do `LoadDropList`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DropGlobal {
    pub tabelas: Vec<TabelaDeDropGlobal>,
    /// Tabelas do arquivo que o GS ignora, com o motivo (posição, nome, motivo).
    pub ignoradas: Vec<(usize, String, &'static str)>,
    por_monstro: HashMap<u32, usize>,
}

struct Leitor<'a> {
    b: &'a [u8],
    o: usize,
}

impl Leitor<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        let fim = self.o.checked_add(n).filter(|f| *f <= self.b.len()).ok_or_else(|| {
            DropGlobalError::Formato(format!("acabou no byte {} (pedia {n} a mais, o arquivo tem {})", self.o, self.b.len()))
        })?;
        let s = &self.b[self.o..fim];
        self.o = fim;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
}

impl DropGlobal {
    pub fn ler(b: &[u8]) -> Result<Self> {
        let mut l = Leitor { b, o: 0 };
        let versao = l.u32()?;
        if versao != VERSAO {
            return Err(DropGlobalError::Formato(format!("versão {versao}, o GS só aceita {VERSAO}")));
        }
        let quantidade = l.i32()?;
        let mut d = DropGlobal::default();
        for indice in 0..quantidade.max(0) as usize {
            let n = l.i32()?;
            if n < 0 {
                return Err(DropGlobalError::Formato(format!("tabela {indice}: {n} monstros")));
            }
            let monstros = (0..n).map(|_| l.u32()).collect::<Result<Vec<_>>>()?;
            let mut todos = Vec::with_capacity(ITENS_POR_TABELA);
            for _ in 0..ITENS_POR_TABELA {
                todos.push((l.u32()?, l.f32()?));
            }
            let nome = l.bytes(128)?;
            let nome = encoding_rs::GBK.decode(&nome[..nome.iter().position(|c| *c == 0).unwrap_or(128)]).0.into_owned();
            let tipo = match l.i32()? {
                1 => TipoDeDropGlobal::Substitui,
                2 => TipoDeDropGlobal::Soma,
                t => return Err(DropGlobalError::Formato(format!("tabela {indice} ({nome}): tipo {t}, o GS só aceita 1 ou 2"))),
            };
            let mut chance_de_quantos = [0f32; 8];
            for c in chance_de_quantos.iter_mut() {
                *c = l.f32()?;
            }
            let itens: Vec<(u32, f32)> = todos.into_iter().take_while(|(id, _)| *id != 0).collect();
            if itens.is_empty() {
                d.ignoradas.push((indice, nome, "sem itens"));
                continue;
            }
            let soma = itens.iter().fold(0f32, |s, (_, c)| s + c);
            if (1.0 - soma as f64).abs() > 1e-6 {
                d.ignoradas.push((indice, nome, "as chances não somam 1"));
                continue;
            }
            let pos = d.tabelas.len();
            for m in &monstros {
                if d.por_monstro.insert(*m, pos).is_some() {
                    return Err(DropGlobalError::Formato(format!("monstro {m} em mais de uma tabela (a {indice}, {nome})")));
                }
            }
            d.tabelas.push(TabelaDeDropGlobal { indice, nome, tipo, monstros, itens, chance_de_quantos });
        }
        if l.o != b.len() {
            return Err(DropGlobalError::Formato(format!("sobram {} bytes depois da última tabela", b.len() - l.o)));
        }
        Ok(d)
    }

    /// `drop_template::GetDropList` (`global_drop.h:32-37`).
    pub fn do_monstro(&self, id: u32) -> Option<&TabelaDeDropGlobal> {
        self.por_monstro.get(&id).map(|i| &self.tabelas[*i])
    }

    /// Monta a tabela a partir de tabelas já prontas (testes do mundo, que não carregam o arquivo).
    pub fn de_tabelas(tabelas: Vec<TabelaDeDropGlobal>) -> Self {
        let por_monstro = tabelas.iter().enumerate().flat_map(|(i, t)| t.monstros.iter().map(move |m| (*m, i))).collect();
        DropGlobal { tabelas, ignoradas: Vec::new(), por_monstro }
    }

    pub fn monstros(&self) -> usize {
        self.por_monstro.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn tabela(monstros: &[u32], itens: &[(u32, f32)], tipo: i32, quantos: [f32; 8], nome: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend((monstros.len() as i32).to_le_bytes());
        for m in monstros {
            b.extend(m.to_le_bytes());
        }
        for k in 0..ITENS_POR_TABELA {
            let (id, c) = itens.get(k).copied().unwrap_or((0, 0.0));
            b.extend(id.to_le_bytes());
            b.extend(c.to_le_bytes());
        }
        let mut n = [0u8; 128];
        n[..nome.len()].copy_from_slice(nome.as_bytes());
        b.extend(n);
        b.extend(tipo.to_le_bytes());
        for q in quantos {
            b.extend(q.to_le_bytes());
        }
        b
    }

    fn arquivo(tabelas: &[Vec<u8>]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(VERSAO.to_le_bytes());
        b.extend((tabelas.len() as i32).to_le_bytes());
        for t in tabelas {
            b.extend(t);
        }
        b
    }

    const UM: [f32; 8] = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    #[test]
    fn le_aceita_e_ignora_como_o_load_drop_list() {
        let b = arquivo(&[
            tabela(&[10, 11], &[(500, 0.25), (501, 0.75)], 2, UM, "a"),
            tabela(&[12], &[], 2, UM, "vazia"),
            tabela(&[13], &[(502, 0.5)], 1, UM, "meia"),
            tabela(&[14], &[(503, 1.0)], 1, [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], "b"),
        ]);
        let d = DropGlobal::ler(&b).unwrap();
        assert_eq!(d.tabelas.len(), 2);
        assert_eq!(d.ignoradas.iter().map(|i| i.2).collect::<Vec<_>>(), ["sem itens", "as chances não somam 1"]);
        assert_eq!(d.do_monstro(11).unwrap().itens, [(500, 0.25), (501, 0.75)]);
        assert_eq!(d.do_monstro(14).unwrap().tipo, TipoDeDropGlobal::Substitui);
        assert!(d.do_monstro(12).is_none() && d.do_monstro(13).is_none());
        assert_eq!(d.monstros(), 3);
        // 1 item por morte, 25% → 25%; metade das mortes com 1 item de chance 1 → 50%.
        assert!((d.do_monstro(10).unwrap().chance_por_morte(500) - 0.25).abs() < 1e-6);
        assert!((d.do_monstro(14).unwrap().chance_por_morte(503) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn recusa_o_que_faz_o_gs_nao_subir() {
        let dup = arquivo(&[tabela(&[10], &[(500, 1.0)], 2, UM, "a"), tabela(&[10], &[(501, 1.0)], 2, UM, "b")]);
        assert!(DropGlobal::ler(&dup).is_err());
        assert!(DropGlobal::ler(&arquivo(&[tabela(&[10], &[(500, 1.0)], 3, UM, "a")])).is_err());
        let mut sobra = arquivo(&[tabela(&[10], &[(500, 1.0)], 2, UM, "a")]);
        sobra.push(0);
        assert!(DropGlobal::ler(&sobra).is_err());
        let mut curto = arquivo(&[tabela(&[10], &[(500, 1.0)], 2, UM, "a")]);
        curto.pop();
        assert!(DropGlobal::ler(&curto).is_err());
        let mut v2 = arquivo(&[]);
        v2[0] = 2;
        assert!(DropGlobal::ler(&v2).is_err());
    }

    /// Os arquivos dos realms, quando estão no disco.
    #[test]
    fn arquivos_dos_realms() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if let Ok(b) = std::fs::read(raiz.join("realm_126/config/extra_drops.sev")) {
            assert_eq!(b.len(), 39_352);
            let d = DropGlobal::ler(&b).unwrap();
            assert_eq!((d.tabelas.len(), d.ignoradas.len(), d.monstros()), (15, 0, 1_526));
            assert!(d.tabelas.iter().all(|t| t.tipo == TipoDeDropGlobal::Soma));
            assert_eq!(d.tabelas[14].itens, [(12962, 1.0)]);
        }
        if let Ok(b) = std::fs::read(raiz.join("realm_155/config/extra_drops.sev")) {
            let d = DropGlobal::ler(&b).unwrap();
            assert_eq!((d.tabelas.len(), d.ignoradas.len()), (0, 1));
        }
    }
}
