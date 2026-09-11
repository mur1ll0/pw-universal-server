//! A altura do chão, lida dos `.hmap` do mapa.
//!
//! # Por que isto existe
//!
//! Duas coisas em jogo dependiam de saber onde é o chão, e as duas estavam erradas até
//! 2026-09-11:
//!
//! - **O teleporte de GM enterrava o jogador.** O `y` que o cliente manda no `GOTO` é um
//!   marcador (`c2s_CmdGoto(fX, 1.0f, fZ)`, `DlgWorldMap.cpp:1174`), e o original
//!   **substitui** o campo pela altura do terreno:
//!   `pos.y = pImp->_plane->GetHeightAt(pos.x, pos.z)`
//!   (`EvolvedPWServer/cgame/gs/playercmd.cpp:4926`). Sem o mapa, o item 37 tinha
//!   escolhido manter a altura atual do jogador — remendo que funciona em terreno plano e
//!   falha em qualquer encosta.
//! - **Monstros terrestres flutuavam.** O `y` do `npcgen.data` **não é altura absoluta**:
//!   é um deslocamento acima do chão. O gerador do original não deixa dúvida
//!   (`npcgenerator.cpp:4319-4322`):
//!
//!   ```cpp
//!   virtual float GenerateY(float x, float y, float z, float offset, world * plane)
//!   {
//!       y = plane->GetHeightAt(x,z);
//!       return y + offset;
//!   }
//!   ```
//!
//!   Usar o `y` do arquivo como altura absoluta põe cada monstro na altura em que o
//!   editor de mapas estava, não na do chão sob ele.
//!
//! # O formato
//!
//! Cada mapa é uma grade de blocos, um arquivo `map/<n>.hmap` por bloco, `n` de 1 a
//! `colunas*linhas`. Cada arquivo são `(513)²` floats little-endian **no intervalo 0..1**,
//! sem cabeçalho — 1.052.676 bytes, que é exatamente o tamanho dos arquivos do realm. A
//! altura real sai de `h * (maxima - minima) + minima` (`terrain.cpp:88-92`).
//!
//! A configuração de cada mapa (quantos blocos, em que arranjo, com que escala) **não dá
//! para deduzir da pasta**: 88 arquivos tanto podem ser 8×11 quanto 11×8. Ela vem do
//! `gs.conf` do servidor original, extraída para `specs/mapas/terreno_155.json`.
//!
//! # A geometria, igual à do original
//!
//! O canto de origem é o alto-esquerda, e **o eixo `z` é invertido** em relação ao índice
//! da linha (`terrain.cpp:181-182`):
//!
//! ```text
//! ox = -(vertices_por_bloco * colunas * celula) / 2
//! oz = +(vertices_por_bloco * linhas  * celula) / 2
//! h  = (x - ox) / celula        v = (oz - z) / celula
//! ```
//!
//! Para o mundo principal do 1.5.5 isso dá `x ∈ [-4096, 4096]`, `z ∈ [-5632, 5632]` — os
//! mesmos limites que o `base_region` do `gs.conf` declara, o que é a conferência de que a
//! fórmula está certa.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum TerrenoError {
    #[error("erro de I/O lendo o terreno: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0} tem {1} bytes; um bloco de {2}x{2} vértices tem {3}")]
    TamanhoDoBloco(String, usize, usize, usize),
}

pub type Result<T> = std::result::Result<T, TerrenoError>;

/// A configuração de terreno de um mapa, como o `gs.conf` do original a declara.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfigDeTerreno {
    pub blocos_colunas: usize,
    pub blocos_linhas: usize,
    /// `nAreaWidth` — o bloco tem **este valor + 1** vértices de lado, porque a última
    /// coluna/linha é compartilhada com o bloco vizinho.
    pub vertices_por_bloco: usize,
    /// `vGridSize`, em metros entre vértices vizinhos.
    pub tamanho_da_celula: f32,
    pub altura_minima: f32,
    pub altura_maxima: f32,
}

impl ConfigDeTerreno {
    /// Quantos vértices um bloco tem de lado no arquivo: `nAreaWidth + 1`.
    fn lado_do_bloco(&self) -> usize {
        self.vertices_por_bloco + 1
    }

    fn origem_x(&self) -> f32 {
        -((self.vertices_por_bloco * self.blocos_colunas) as f32) * self.tamanho_da_celula / 2.0
    }

    fn origem_z(&self) -> f32 {
        (self.vertices_por_bloco * self.blocos_linhas) as f32 * self.tamanho_da_celula / 2.0
    }
}

/// O catálogo de `specs/mapas/terreno_155.json`, por `world_id`.
#[derive(Debug, Clone, Deserialize)]
struct EntradaDoCatalogo {
    world_id: i32,
    sub_pasta: String,
    blocos_colunas: usize,
    blocos_linhas: usize,
    vertices_por_bloco: usize,
    tamanho_da_celula: f32,
    altura_minima: f32,
    altura_maxima: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalogo {
    mapas: Vec<EntradaDoCatalogo>,
}

const CATALOGO_JSON: &str = include_str!("../../../specs/mapas/terreno_155.json");

/// A configuração e a sub-pasta de um mapa, ou `None` se o catálogo não o conhece.
pub fn config_do_mapa(world_id: i32) -> Option<(ConfigDeTerreno, String)> {
    let c: Catalogo = serde_json::from_str(CATALOGO_JSON).ok()?;
    c.mapas.into_iter().find(|m| m.world_id == world_id).map(|m| {
        (
            ConfigDeTerreno {
                blocos_colunas: m.blocos_colunas,
                blocos_linhas: m.blocos_linhas,
                vertices_por_bloco: m.vertices_por_bloco,
                tamanho_da_celula: m.tamanho_da_celula,
                altura_minima: m.altura_minima,
                altura_maxima: m.altura_maxima,
            },
            m.sub_pasta,
        )
    })
}

/// O mapa de alturas de um mapa, com os blocos que existirem em disco.
///
/// Os blocos ficam **separados**, um `Vec<f32>` por arquivo, e não numa matriz única. Dá
/// no mesmo em memória (as bordas são vértices repetidos entre vizinhos, que o original
/// também duplica ao montar a matriz) e evita uma alocação contígua de dezenas de MB —
/// o mundo principal tem 4097×5633 vértices.
#[derive(Debug, Clone, Default)]
pub struct Terreno {
    pub world_id: i32,
    config: Option<ConfigDeTerreno>,
    /// Índice do bloco → alturas já convertidas para metros. `None` = arquivo ausente.
    blocos: Vec<Option<Vec<f32>>>,
}

impl Terreno {
    /// Lê os `.hmap` da pasta do mapa. Um bloco ausente não é erro — vira `None`, e
    /// [`Self::altura_em`] devolve `None` sobre ele.
    ///
    /// `dir` é a pasta do mapa (a que contém `map/`), não a pasta `map/` em si.
    pub fn ler(world_id: i32, dir: &Path) -> Self {
        let Some((config, sub_pasta)) = config_do_mapa(world_id) else {
            warn!(
                "terreno: o mapa {world_id} não está em specs/mapas/terreno_155.json — sem \
                 altura de chão; teleporte e spawn caem no comportamento antigo"
            );
            return Self::default();
        };

        let pasta = dir.join(&sub_pasta);
        let total = config.blocos_colunas * config.blocos_linhas;
        let lado = config.lado_do_bloco();
        let escala = config.altura_maxima - config.altura_minima;

        let mut blocos = Vec::with_capacity(total);
        let mut lidos = 0usize;
        for n in 1..=total {
            let caminho = pasta.join(format!("{n}.hmap"));
            match ler_bloco(&caminho, lado, escala, config.altura_minima) {
                Ok(b) => {
                    blocos.push(Some(b));
                    lidos += 1;
                }
                Err(e) => {
                    // Só vale avisar uma vez por mapa: um mapa sem `map/` nenhum geraria
                    // uma linha por bloco.
                    if lidos == 0 && n == 1 {
                        warn!("terreno: mapa {world_id} sem {}: {e}", caminho.display());
                    }
                    blocos.push(None);
                }
            }
        }

        if lidos == 0 {
            return Self { world_id, config: None, blocos: Vec::new() };
        }

        info!(
            "terreno: mapa {world_id} com {lidos}/{total} blocos de {lado}x{lado} \
             ({:.0} MB), x de {:.0} a {:.0}, z de {:.0} a {:.0}",
            (lidos * lado * lado * 4) as f32 / 1_048_576.0,
            config.origem_x(),
            -config.origem_x(),
            -config.origem_z(),
            config.origem_z(),
        );

        Self { world_id, config: Some(config), blocos }
    }

    /// Um terreno vazio: nenhum bloco, [`Self::altura_em`] sempre `None`.
    pub fn vazio() -> Self {
        Self::default()
    }

    pub fn tem_dados(&self) -> bool {
        self.config.is_some() && self.blocos.iter().any(|b| b.is_some())
    }

    /// A altura do chão em `(x, z)`, ou `None` fora do mapa ou sobre um bloco ausente.
    ///
    /// Reproduz `CTerrain::GetHeightAt` (`terrain.cpp:179-217`): interpolação linear
    /// dentro do triângulo da célula, escolhido pela diagonal `dx < dz`.
    ///
    /// ```text
    /// 0-----1
    /// | \   |
    /// |  \  |
    /// 2-----3
    /// ```
    pub fn altura_em(&self, x: f32, z: f32) -> Option<f32> {
        let c = self.config?;
        let inv = 1.0 / c.tamanho_da_celula;
        let h = (x - c.origem_x()) * inv;
        let v = (c.origem_z() - z) * inv;
        if h < 0.0 || v < 0.0 || !h.is_finite() || !v.is_finite() {
            return None;
        }

        let nx = h as usize;
        let nz = v as usize;
        let mut dx = h - nx as f32;
        let mut dz = v - nz as f32;

        let max_x = c.vertices_por_bloco * c.blocos_colunas;
        let max_z = c.vertices_por_bloco * c.blocos_linhas;
        // O vértice da borda mais distante não tem célula à frente para interpolar.
        if nx >= max_x || nz >= max_z {
            return None;
        }

        let (h0, h1, h2) = if dx < dz {
            // triângulo de cima à esquerda
            dz = 1.0 - dz;
            (
                self.vertice(nx, nz + 1)?,
                self.vertice(nx + 1, nz + 1)?,
                self.vertice(nx, nz)?,
            )
        } else {
            // triângulo de baixo à direita
            dx = 1.0 - dx;
            (
                self.vertice(nx + 1, nz)?,
                self.vertice(nx, nz)?,
                self.vertice(nx + 1, nz + 1)?,
            )
        };

        Some(h0 + (h1 - h0) * dx + (h2 - h0) * dz)
    }

    /// A altura de um vértice global, achando o bloco que o contém.
    ///
    /// A borda entre dois blocos é o mesmo vértice guardado nos dois arquivos (cada bloco
    /// tem `nAreaWidth + 1` de lado justamente por isso), então dividir por
    /// `vertices_por_bloco` sempre cai num bloco válido.
    fn vertice(&self, gx: usize, gz: usize) -> Option<f32> {
        let c = self.config?;
        let por_bloco = c.vertices_por_bloco;
        let lado = c.lado_do_bloco();

        let bx = (gx / por_bloco).min(c.blocos_colunas - 1);
        let bz = (gz / por_bloco).min(c.blocos_linhas - 1);
        let lx = gx - bx * por_bloco;
        let lz = gz - bz * por_bloco;

        let bloco = self.blocos.get(bz * c.blocos_colunas + bx)?.as_ref()?;
        bloco.get(lz * lado + lx).copied()
    }

    /// A altura do chão, ou `padrao` quando o mapa não sabe responder.
    ///
    /// Existe para o chamador não ter de repetir o `unwrap_or` — e para o padrão ficar
    /// escrito no ponto de uso, onde dá para julgar se ele é aceitável.
    pub fn altura_ou(&self, x: f32, z: f32, padrao: f32) -> f32 {
        self.altura_em(x, z).unwrap_or(padrao)
    }
}

fn ler_bloco(caminho: &Path, lado: usize, escala: f32, minima: f32) -> Result<Vec<f32>> {
    let bytes = std::fs::read(caminho)?;
    let esperado = lado * lado * 4;
    if bytes.len() != esperado {
        return Err(TerrenoError::TamanhoDoBloco(
            caminho.display().to_string(),
            bytes.len(),
            lado,
            esperado,
        ));
    }

    let mut alturas = Vec::with_capacity(lado * lado);
    for pedaco in bytes.chunks_exact(4) {
        let bruto = f32::from_le_bytes([pedaco[0], pedaco[1], pedaco[2], pedaco[3]]);
        alturas.push(bruto * escala + minima);
    }
    Ok(alturas)
}

/// Os terrenos carregados, por `world_id`.
pub type TerrenosPorMapa = HashMap<i32, Terreno>;

#[cfg(test)]
mod tests {
    use super::*;

    fn config_de_teste() -> ConfigDeTerreno {
        ConfigDeTerreno {
            blocos_colunas: 1,
            blocos_linhas: 1,
            vertices_por_bloco: 2, // 3x3 vértices
            tamanho_da_celula: 2.0,
            altura_minima: 0.0,
            altura_maxima: 1.0,
        }
    }

    /// Uma rampa que sobe com o `x`: as três colunas valem 0, 1 e 2.
    fn rampa() -> Terreno {
        Terreno {
            world_id: 0,
            config: Some(config_de_teste()),
            blocos: vec![Some(vec![
                0.0, 1.0, 2.0, //
                0.0, 1.0, 2.0, //
                0.0, 1.0, 2.0,
            ])],
        }
    }

    #[test]
    fn a_origem_e_o_canto_alto_esquerda_e_o_z_e_invertido() {
        let c = config_de_teste();
        // 2 vértices por bloco * 1 bloco * 2 m = 4 m de lado, centrado na origem.
        assert_eq!(c.origem_x(), -2.0);
        assert_eq!(c.origem_z(), 2.0);
    }

    /// O mundo principal do 1.5.5 tem de dar exatamente os limites que o `base_region` do
    /// `gs.conf` declara: `{-4096,-5632},{4096,5632}`. É a conferência da fórmula.
    #[test]
    fn o_mundo_principal_bate_com_o_base_region_do_gs_conf() {
        let (c, sub) = config_do_mapa(1).expect("o mapa 1 está no catálogo");
        assert_eq!(sub, "map");
        assert_eq!((c.blocos_colunas, c.blocos_linhas), (8, 11));
        assert_eq!(c.origem_x(), -4096.0);
        assert_eq!(c.origem_z(), 5632.0);
        assert_eq!(c.altura_maxima, 800.0);
    }

    #[test]
    fn interpola_entre_os_vertices_da_rampa() {
        let t = rampa();
        // Canto alto-esquerda: vértice (0,0), altura 0.
        assert_eq!(t.altura_em(-2.0, 2.0), Some(0.0));
        // Duas células à direita: vértice (2,0), altura 2.
        assert_eq!(t.altura_em(2.0 - 1e-3, 2.0).unwrap().round(), 2.0);
        // No meio da primeira célula em x, ainda na primeira linha: metade do caminho.
        let meio = t.altura_em(-1.0, 2.0).unwrap();
        assert!((meio - 0.5).abs() < 1e-5, "esperava 0,5 no meio da rampa, veio {meio}");
    }

    #[test]
    fn fora_do_mapa_nao_inventa_altura() {
        let t = rampa();
        assert_eq!(t.altura_em(-1000.0, 0.0), None);
        assert_eq!(t.altura_em(0.0, 1000.0), None);
        assert_eq!(t.altura_em(f32::NAN, 0.0), None);
    }

    #[test]
    fn sem_blocos_nao_ha_altura() {
        let t = Terreno::vazio();
        assert!(!t.tem_dados());
        assert_eq!(t.altura_em(0.0, 0.0), None);
        assert_eq!(t.altura_ou(0.0, 0.0, 219.0), 219.0);
    }
}
