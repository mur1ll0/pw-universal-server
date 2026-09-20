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
//! quantidade × { i32 id; i32 world_tag; f32 x; f32 y; f32 z; i32 ordem }   // 24 bytes
//! ```
//!
//! 92 registros, 2.212 bytes: `4 + 92 × 24`. O último campo é a ordem no arquivo (1, 2, 3…);
//! guardamos por fidelidade, mas nada o usa.

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
    /// A ordem em que o ponto aparece no arquivo.
    pub ordem: i32,
}

/// Os pontos de destino, por id.
#[derive(Debug, Clone, Default)]
pub struct PontosDoMundo {
    pub por_id: HashMap<i32, PontoDoMundo>,
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
        for k in 0..n {
            let o = 4 + k * REGISTRO;
            let i32_em = |d: usize| i32::from_le_bytes(b[o + d..o + d + 4].try_into().unwrap());
            let f32_em = |d: usize| f32::from_le_bytes(b[o + d..o + d + 4].try_into().unwrap());
            let p = PontoDoMundo {
                id: i32_em(0),
                mundo: i32_em(4),
                pos: [f32_em(8), f32_em(12), f32_em(16)],
                ordem: i32_em(20),
            };
            por_id.insert(p.id, p);
        }
        Ok(Self { por_id })
    }
}
