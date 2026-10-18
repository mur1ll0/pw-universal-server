//! `world_targets.sev` — onde fica cada **ponto de destino** do mundo.
//!
//! É a tabela que o `NPC_TRANSMIT_SERVICE` do `elements.data` referencia: lá cada
//! transportadora lista até 32 destinos por `idTarget`, com preço e nível exigido, mas **sem
//! coordenada**. A coordenada está aqui — no original ela entra no `transmit_entry`
//! (`{ int world_tag; A3DVECTOR target; size_t fee; int require_level; int target_waypoint; }`,
//! `gs/serviceprovider.cpp:683-700`) que o `transmit_provider` monta ao ser inicializado.
//!
//! # Formato (medido no arquivo do `realm_155`, que fecha no último byte)
//!
//! ```text
//! u32 quantidade
//! quantidade × { i32 id; i32 world_id; A3DVECTOR3 pos; i32 domain_id }   // 24 bytes
//! ```
//!
//! É o que o exportador do cliente grava a partir do `maps\world\world_targets.txt` com `globalsave`
//! (`CCommon/globaldataman.cpp:793-803`; a leitura do texto, com o nome de cada ponto, em `:450-466`).
//! 92 registros no `realm_155`, 2.212 bytes: `4 + 92 × 24`. O último campo é o **domínio** do ponto
//! (`domain_id`), não a ordem: o B244 achou valores repetidos (1, 2, 3, 4, 6, 7, 7, 7, 7…); nada no
//! servidor o usa.

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PontosError {
    #[error("world_targets.sev: {0}")]
    Formato(String),
}

pub type Result<T> = std::result::Result<T, PontosError>;

/// Um ponto de destino do mundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PontoDoMundo {
    pub id: i32,
    /// O mapa (`world_tag`) onde o ponto fica.
    pub mundo: i32,
    pub pos: [f32; 3],
    /// `domain_id` (`globaldataman.cpp:803`).
    pub dominio: i32,
}

/// Os pontos de destino, por id.
#[derive(Debug, Clone, Default)]
pub struct PontosDoMundo {
    pub por_id: HashMap<i32, PontoDoMundo>,
    /// Na ordem do arquivo (para gravar de volta, B245).
    pub lista: Vec<PontoDoMundo>,
}

impl PontosDoMundo {
    pub fn get(&self, id: i32) -> Option<&PontoDoMundo> {
        self.por_id.get(&id)
    }

    pub fn len(&self) -> usize {
        self.por_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.por_id.is_empty()
    }

    /// Lê o arquivo inteiro. Falha quando o arquivo não fecha no último byte — um ponto a
    /// menos aqui é um teleporte que manda o jogador para o lugar errado.
    pub fn load_from_bytes(b: &[u8]) -> Result<Self> {
        const REGISTRO: usize = 24;
        let erro = |m: &str| PontosError::Formato(m.to_string());
        let n = b
            .get(0..4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()) as usize)
            .ok_or_else(|| erro("cabeçalho curto"))?;
        let esperado = 4 + n * REGISTRO;
        if b.len() != esperado {
            return Err(erro(&format!("{} bytes para {n} pontos (esperados {esperado})", b.len())));
        }
        let mut por_id = HashMap::with_capacity(n);
        let mut lista = Vec::with_capacity(n);
        for k in 0..n {
            let o = 4 + k * REGISTRO;
            let i32_em = |d: usize| i32::from_le_bytes(b[o + d..o + d + 4].try_into().unwrap());
            let f32_em = |d: usize| f32::from_le_bytes(b[o + d..o + d + 4].try_into().unwrap());
            let p = PontoDoMundo {
                id: i32_em(0),
                mundo: i32_em(4),
                pos: [f32_em(8), f32_em(12), f32_em(16)],
                dominio: i32_em(20),
            };
            por_id.insert(p.id, p);
            lista.push(p);
        }
        Ok(Self { por_id, lista })
    }

    /// O inverso de `load_from_bytes`, na ordem do arquivo.
    pub fn gravar(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(4 + self.lista.len() * 24);
        b.extend_from_slice(&(self.lista.len() as u32).to_le_bytes());
        for p in &self.lista {
            for v in [p.id, p.mundo] {
                b.extend_from_slice(&v.to_le_bytes());
            }
            for c in p.pos {
                b.extend_from_slice(&c.to_bits().to_le_bytes());
            }
            b.extend_from_slice(&p.dominio.to_le_bytes());
        }
        b
    }

    /// Troca a posição do ponto `id` (o mapa e o domínio ficam).
    pub fn definir_posicao(&mut self, id: i32, pos: [f32; 3]) -> bool {
        let Some(p) = self.lista.iter_mut().find(|p| p.id == id) else { return false };
        p.pos = pos;
        self.por_id.insert(id, *p);
        true
    }
}

// ------------------------------------------------------------------ world_targets.txt do cliente (B252)

/// Um destino do `maps\world\world_targets.txt` do cliente (`globaldata_load`, `globaldataman.cpp:450-466`).
#[derive(Debug, Clone, PartialEq)]
pub struct DestinoDoCliente {
    pub id: i32,
    pub nome: String,
    pub mundo: i32,
    pub pos: [f32; 3],
    pub dominio: i32,
}

/// O `world_targets.txt` inteiro. Formato medido nos clientes 1.2.6 (49 destinos) e 1.5.5 BR (92): UTF-16LE com BOM,
/// CRLF, uma linha por destino, `id ⇥ "nome" ⇥ mundo ⇥ "x,y,z" ⇥ domínio` com a posição em `%.3f` — é dela que o
/// `globalsave` gera o `world_targets.sev` (`:793-803`), por isso o `.sev` tem os mesmos ids, na mesma ordem.
#[derive(Debug, Clone, PartialEq)]
pub struct TxtDosDestinos {
    pub destinos: Vec<DestinoDoCliente>,
}

impl TxtDosDestinos {
    pub fn ler(b: &[u8]) -> Result<Self> {
        let erro = |m: String| PontosError::Formato(format!("world_targets.txt: {m}"));
        let t = crate::clt::texto(b, "world_targets.txt").map_err(|e| erro(e.to_string()))?;
        let corpo = t.strip_suffix("\r\n").ok_or_else(|| erro("não termina em CRLF".into()))?;
        let mut destinos = Vec::new();
        for (n, l) in corpo.split("\r\n").enumerate() {
            let campos: Vec<&str> = l.split('\t').collect();
            if campos.len() != 5 {
                return Err(erro(format!("linha {}: {} campos", n + 1, campos.len())));
            }
            let int = |s: &str| s.parse::<i32>().map_err(|_| erro(format!("linha {}: número {s:?}", n + 1)));
            let pos_txt = crate::clt::sem_aspas(campos[3]);
            let p: Vec<f32> = pos_txt.split(',').map(|x| x.parse::<f32>()).collect::<std::result::Result<_, _>>()
                .map_err(|_| erro(format!("linha {}: posição {pos_txt:?}", n + 1)))?;
            if p.len() != 3 || !campos[1].starts_with('"') || !campos[1].ends_with('"') {
                return Err(erro(format!("linha {}: nome ou posição", n + 1)));
            }
            destinos.push(DestinoDoCliente {
                id: int(campos[0])?,
                nome: crate::clt::sem_aspas(campos[1]),
                mundo: int(campos[2])?,
                pos: [p[0], p[1], p[2]],
                dominio: int(campos[4])?,
            });
        }
        Ok(Self { destinos })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let f = |x: f32| crate::clt::formatar_com(x, 3);
        let mut s = String::new();
        for d in &self.destinos {
            s.push_str(&format!("{}\t\"{}\"\t{}\t\"{},{},{}\"\t{}\r\n", d.id, d.nome, d.mundo, f(d.pos[0]), f(d.pos[1]), f(d.pos[2]), d.dominio));
        }
        crate::clt::bytes(&s)
    }

    /// A posição como o `globalsave` a passa ao `.sev`: o `float` do texto com 3 casas.
    pub fn posicao_do_texto(pos: [f32; 3]) -> [f32; 3] {
        pos.map(|x| crate::clt::formatar_com(x, 3).parse().unwrap_or(x))
    }
}

#[cfg(test)]
mod testes_do_txt {
    use super::*;

    #[test]
    fn world_targets_txt_dos_dois_clientes_volta_identico() {
        for p in [r"F:\Games\perfectworld_126\element\maps\world\world_targets.txt", r"F:\PW\1.5.5\1.5.5 BR launcher\element\maps\world\world_targets.txt"] {
            let Ok(b) = std::fs::read(p) else {
                eprintln!("AVISO: sem {p} — este teste NÃO verificou esse cliente.");
                continue;
            };
            let t = TxtDosDestinos::ler(&b).unwrap();
            assert!(t.gravar() == b, "{p} não volta idêntico");
            assert!(t.destinos.len() > 40);
        }
        // 0,0625 é empate exato em binário: o MSVC vai para longe do zero (0.063), o "para o par" daria 0.062.
        assert_eq!(TxtDosDestinos::posicao_do_texto([1.23456, 0.0625, -0.0625]), [1.235, 0.063, -0.063]);
    }
}
