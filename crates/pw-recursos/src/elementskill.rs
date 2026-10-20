//! B305 (Estúdio S8): os números do cliente — editar o `elementskill.dll` trocando só os bytes das tabelas.
//!
//! Recompilar o DLL do fonte desfaria o que o BR mudou (B303: o dinheiro do DLL não é o do fonte), então a edição é
//! no binário, em posições **medidas**: `tools/elementskill/enderecos.cpp` acha cada tabela por nível na imagem
//! carregada e a prova pondo uma sentinela em cada nível e chamando a função do próprio DLL; só vale tabela em seção
//! gravável (`.data`) e igual ao arquivo. Prova de ponta a ponta: com +1 em todas as 1.795 tabelas, a medição do DLL
//! alterado muda exatamente os 5.981 valores esperados de 227.436 (`specs/03_DATA_LOADER_SPEC.md` §3.10b).
//!
//! O mapa vale para um DLL só. A cópia editada num rascunho tem outro SHA-256, então o reconhecimento é pelo "resto":
//! o SHA-256 do arquivo com as tabelas zeradas, que não muda com a edição.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// O `elementskill.dll` do cliente 1.5.5 BR v156 (`specs/habilidades_155/elementskill_v156_tabelas.json`).
const MAPA_V156: &str = include_str!("../../../specs/habilidades_155/elementskill_v156_tabelas.json");
/// B306: o do cliente 1.2.6 (`perfectworld_126 launcher/element`; `specs/habilidades_126/elementskill_126_tabelas.json`).
const MAPA_126: &str = include_str!("../../../specs/habilidades_126/elementskill_126_tabelas.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Tabela {
    /// Posição do nível 1 no arquivo; o nível `k` está em `arquivo + 4 × (k − 1)`.
    pub arquivo: usize,
    /// `i` = int32, `f` = float32 (little-endian).
    pub tipo: String,
    /// Níveis editáveis.
    pub n: usize,
    /// Níveis além da tabela (o DLL lê a tabela seguinte: lixo do original, não se edita).
    #[serde(default)]
    pub le_alem: usize,
}

#[derive(Debug, Deserialize)]
pub struct Mapa {
    pub dll_sha256: String,
    pub dll_tamanho: usize,
    pub sha256_do_resto: String,
    pub habilidades: HashMap<String, HashMap<String, Tabela>>,
}

/// Os campos por nível que o mapa pode conter, e quem os confere no cliente.
pub const CAMPOS: [(&str, &str); 4] = [
    ("nivel_exigido", "aprender: LearnCondition (EC_HostPlayer.cpp:6067-6093)"),
    ("sp_exigido", "aprender: LearnCondition"),
    ("dinheiro_exigido", "aprender: LearnCondition"),
    ("item_exigido", "aprender: o livro exigido"),
];

fn mapas() -> &'static [Mapa] {
    static M: OnceLock<Vec<Mapa>> = OnceLock::new();
    M.get_or_init(|| {
        vec![
            serde_json::from_str(MAPA_V156).expect("elementskill_v156_tabelas.json"),
            serde_json::from_str(MAPA_126).expect("elementskill_126_tabelas.json"),
        ]
    })
}

impl Mapa {
    fn zerar_tabelas(&self, dll: &[u8]) -> Vec<u8> {
        let mut resto = dll.to_vec();
        for t in self.habilidades.values().flat_map(|c| c.values()) {
            resto[t.arquivo..t.arquivo + 4 * t.n].fill(0);
        }
        resto
    }

    /// O mapa vale para este arquivo? (o original, ou uma cópia editada só dentro das tabelas)
    pub fn reconhece(&self, dll: &[u8]) -> bool {
        dll.len() == self.dll_tamanho && hex::encode(Sha256::digest(self.zerar_tabelas(dll))) == self.sha256_do_resto
    }

    pub fn tabela(&self, id: i64, campo: &str) -> Option<&Tabela> {
        self.habilidades.get(&id.to_string())?.get(campo)
    }

    /// Os valores por nível de uma tabela.
    pub fn ler(&self, dll: &[u8], t: &Tabela) -> Vec<f64> {
        (0..t.n)
            .map(|k| {
                let b: [u8; 4] = dll[t.arquivo + 4 * k..t.arquivo + 4 * k + 4].try_into().unwrap();
                if t.tipo == "f" { f32::from_le_bytes(b) as f64 } else { i32::from_le_bytes(b) as f64 }
            })
            .collect()
    }

    /// Troca os valores por nível de uma tabela (um por nível editável).
    pub fn escrever(&self, dll: &mut [u8], id: i64, campo: &str, valores: &[f64]) -> Result<(), String> {
        let t = self.tabela(id, campo).ok_or_else(|| format!("a habilidade {id} não tem {campo} numa tabela do elementskill.dll (vem do código: constante ou fórmula)"))?;
        if valores.len() != t.n {
            return Err(format!("{campo}: {} números, um por nível editável", t.n));
        }
        for (k, v) in valores.iter().enumerate() {
            if !v.is_finite() {
                return Err(format!("{campo} nível {}: número inválido", k + 1));
            }
            let b = if t.tipo == "f" {
                (*v as f32).to_le_bytes()
            } else {
                if v.fract() != 0.0 || *v < i32::MIN as f64 || *v > i32::MAX as f64 {
                    return Err(format!("{campo} nível {}: inteiro de 32 bits", k + 1));
                }
                (*v as i32).to_le_bytes()
            };
            dll[t.arquivo + 4 * k..t.arquivo + 4 * k + 4].copy_from_slice(&b);
        }
        Ok(())
    }
}

/// O mapa deste `elementskill.dll`, se houver.
pub fn mapa_para(dll: &[u8]) -> Option<&'static Mapa> {
    mapas().iter().find(|m| m.reconhece(dll))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn mapa_de_teste(dll: &[u8]) -> Mapa {
        let mut h = HashMap::new();
        h.insert("8".to_string(), HashMap::from([("dinheiro_exigido".to_string(), Tabela { arquivo: 8, tipo: "i".into(), n: 3, le_alem: 0 })]));
        h.insert("9".to_string(), HashMap::from([("mp".to_string(), Tabela { arquivo: 20, tipo: "f".into(), n: 2, le_alem: 0 })]));
        let mut m = Mapa { dll_sha256: String::new(), dll_tamanho: dll.len(), sha256_do_resto: String::new(), habilidades: h };
        m.sha256_do_resto = hex::encode(Sha256::digest(m.zerar_tabelas(dll)));
        m
    }

    #[test]
    fn escreve_so_dentro_da_tabela_e_reconhece_a_copia() {
        let mut dll: Vec<u8> = (0..40u8).collect();
        let m = mapa_de_teste(&dll);
        assert!(m.reconhece(&dll));
        let antes = dll.clone();
        m.escrever(&mut dll, 8, "dinheiro_exigido", &[0.0, 6980.0, 771980.0]).unwrap();
        m.escrever(&mut dll, 9, "mp", &[1.5, 2.25]).unwrap();
        // Fora das tabelas, nada muda; a cópia editada continua reconhecida.
        assert_eq!((&dll[..8], &dll[28..]), (&antes[..8], &antes[28..]));
        assert_ne!(&dll[8..28], &antes[8..28]);
        assert!(m.reconhece(&dll));
        let t = m.tabela(8, "dinheiro_exigido").unwrap().clone();
        assert_eq!(m.ler(&dll, &t), vec![0.0, 6980.0, 771980.0]);
        let t = m.tabela(9, "mp").unwrap().clone();
        assert_eq!(m.ler(&dll, &t), vec![1.5, 2.25]);
        // Byte fora das tabelas mudado: outro arquivo.
        dll[0] ^= 1;
        assert!(!m.reconhece(&dll));
    }

    #[test]
    fn recusa_campo_sem_tabela_tamanho_errado_e_nao_inteiro() {
        let mut dll = vec![0u8; 40];
        let m = mapa_de_teste(&dll);
        assert!(m.escrever(&mut dll, 8, "sp_exigido", &[1.0, 2.0, 3.0]).unwrap_err().contains("vem do código"));
        assert!(m.escrever(&mut dll, 8, "dinheiro_exigido", &[1.0]).unwrap_err().contains("3 números"));
        assert!(m.escrever(&mut dll, 8, "dinheiro_exigido", &[1.0, 2.5, 3.0]).unwrap_err().contains("inteiro"));
        assert!(m.escrever(&mut dll, 8, "dinheiro_exigido", &[1.0, f64::NAN, 3.0]).is_err());
    }

    #[test]
    fn o_mapa_embutido_le_e_e_coerente() {
        let m = &mapas()[0];
        assert_eq!(m.dll_tamanho, 4_391_000);
        // B303: a habilidade 8 tem nível, SP e dinheiro em tabelas lado a lado (40 bytes cada).
        let (n, s, d) = (m.tabela(8, "nivel_exigido").unwrap(), m.tabela(8, "sp_exigido").unwrap(), m.tabela(8, "dinheiro_exigido").unwrap());
        assert_eq!((s.arquivo - n.arquivo, d.arquivo - s.arquivo, d.n), (40, 40, 10));
        // A 187 tem 12 níveis e tabelas de 10: os 2 últimos do livro leem a tabela seguinte.
        let t = m.tabela(187, "item_exigido").unwrap();
        assert_eq!((t.n, t.le_alem), (10, 2));
        // Nenhuma tabela passa do arquivo nem invade outra.
        let mut pos: Vec<(usize, usize)> = m.habilidades.values().flat_map(|c| c.values()).map(|t| (t.arquivo, t.arquivo + 4 * t.n)).collect();
        pos.sort();
        assert!(pos.windows(2).all(|w| w[0].1 <= w[1].0) && pos.last().unwrap().1 <= m.dll_tamanho);
        assert_eq!(pos.len(), 1795);
    }

    #[test]
    fn o_mapa_do_126_le_e_e_coerente() {
        let m = &mapas()[1];
        assert_eq!(m.dll_tamanho, 577_536);
        // A 187 do 1.2.6 também tem 12 níveis e tabelas de 10.
        assert_eq!(m.tabela(187, "sp_exigido").map(|t| (t.n, t.le_alem)), Some((10, 2)));
        let mut pos: Vec<(usize, usize)> = m.habilidades.values().flat_map(|c| c.values()).map(|t| (t.arquivo, t.arquivo + 4 * t.n)).collect();
        pos.sort();
        assert!(pos.windows(2).all(|w| w[0].1 <= w[1].0) && pos.last().unwrap().1 <= m.dll_tamanho);
        assert_eq!(pos.len(), 1865);
    }
}
