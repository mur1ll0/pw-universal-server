//! O espaço passável do ar e da água: `airmap/spmap.conf` + `N.octr` de cada mapa.
//!
//! Porte de `cgame/gs/pathfinding/GlobalSPMap.{h,cpp}` e `CompactSpacePassableOctree.{h,cpp}`
//! (`CGlobalSPMap`, `CCompactSpacePassableOctree`). O mapa é uma grade `largura × comprimento`
//! de submapas cúbicos de `tamanho` metros; cada submapa é uma octree compacta de nós de 4 bytes
//! (2 bits de estado, 26 bits do endereço dos filhos). Submapa sem arquivo — ou que não carrega —
//! é **livre** (`SetFreeSPMap`, `GlobalSPMap.cpp:45-58`, `:119-140`).
//!
//! Formato do `.octr` (`CCompactSpacePassableOctree::Load`, `CompactSpacePassableOctree.cpp:
//! 245-288`): `u32 0xcc000001`, `u8 id`, `CubeInt` (`int x, y, z, halfsize`), `int` tamanho da
//! folha, `u32 n`, `n × u32` nós — 29 + 4n bytes, e o arquivo fecha no último.
//!
//! Quem usa: a perseguição do NPC de ar e de água (`CNPCChaseSpatiallyPFAgent`, em
//! `pw-gs::navegacao`) e o `IsValidSPPos` do lugar do mascote (`pathfinding.cpp:46-51`).
use std::path::Path;
use tracing::{debug, info, warn};

/// `uiCSPOctreeFileVer` (`CompactSpacePassableOctree.cpp:47`).
pub const VERSAO_DO_OCTR: u32 = 0xcc00_0001;

/// Estado de um nó (`CCompactSpacePassableOctree::Free/Blocked/CHBorder`).
pub const LIVRE: u8 = 0;

/// `CubeInt`: centro e meia aresta, em inteiros. `Inside` é **estrito** nas seis faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cubo {
    pub centro: [i32; 3],
    pub meio: i32,
}

impl Cubo {
    pub fn dentro(&self, p: [i32; 3]) -> bool {
        (0..3).all(|i| p[i] < self.centro[i] + self.meio && p[i] > self.centro[i] - self.meio)
    }
}

/// `CSPOctreeTravNode`: a folha onde um ponto cai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct No {
    pub pai: Cubo,
    pub cubo: Cubo,
    pub estado: u8,
    pub nivel: u8,
    pub primeiro_irmao: u32,
    pub posicao_entre_irmaos: u8,
    pub octree: u8,
}

impl No {
    pub fn dentro(&self, p: [i32; 3]) -> bool {
        self.cubo.dentro(p)
    }

    /// `IsNodeNeighborSibling`: mesma octree, mesmo pai, e vizinhos por uma face.
    pub fn vizinho_irmao(&self, o: &No) -> bool {
        if self.octree != o.octree || self.primeiro_irmao != o.primeiro_irmao {
            return false;
        }
        matches!(self.posicao_entre_irmaos ^ o.posicao_entre_irmaos, 1 | 2 | 4)
    }

    /// `operator ==`: octree, primeiro irmão e posição.
    pub fn mesmo(&self, o: &No) -> bool {
        self.octree == o.octree
            && self.primeiro_irmao == o.primeiro_irmao
            && self.posicao_entre_irmaos == o.posicao_entre_irmaos
    }
}

/// `CCompactSpacePassableOctree`. Sem nós, o submapa é todo livre.
#[derive(Debug, Clone, Default)]
pub struct Octree {
    pub id: u8,
    pub cubo: Cubo,
    pub folha: i32,
    pub nos: Vec<u32>,
}

impl Octree {
    /// Lê um `.octr` inteiro; `None` se o formato não bate ou sobra byte.
    pub fn ler(bytes: &[u8]) -> Option<Self> {
        let u32_em = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        if u32_em(0)? != VERSAO_DO_OCTR {
            return None;
        }
        let id = *bytes.get(4)?;
        let i = |o: usize| u32_em(o).map(|v| v as i32);
        let cubo = Cubo { centro: [i(5)?, i(9)?, i(13)?], meio: i(17)? };
        let folha = i(21)?;
        let n = u32_em(25)? as usize;
        if bytes.len() != 29 + 4 * n {
            return None;
        }
        let nos = (0..n).map(|k| u32_em(29 + 4 * k).unwrap_or(0)).collect();
        Some(Self { id, cubo, folha, nos })
    }

    fn filhos(no: u32) -> u32 {
        no & 0x03ff_ffff
    }
    fn estado(no: u32) -> u8 {
        (no >> 30) as u8
    }

    /// `GetTraversalNode` a partir da raiz (`CompactSpacePassableOctree.cpp:127-230`).
    pub fn no(&self, p: [i32; 3]) -> No {
        if self.nos.is_empty() {
            return No { pai: self.cubo, cubo: self.cubo, estado: LIVRE, nivel: 0, primeiro_irmao: 0, posicao_entre_irmaos: 0, octree: self.id };
        }
        let mut no = No { pai: self.cubo, nivel: 1, octree: self.id, ..Default::default() };
        let mut endereco: u32 = 0;
        let mut atual = self.cubo;
        loop {
            no.pai = atual;
            let meio = atual.meio >> 1;
            let mut filho = 0u8;
            let mut centro = atual.centro;
            for eixo in 0..3 {
                let mais = p[eixo] > atual.centro[eixo];
                centro[eixo] += if mais { meio } else { -meio };
                if mais {
                    filho |= 4 >> eixo; // x é o bit alto, z o baixo (`Convert3To8`)
                }
            }
            atual = Cubo { centro, meio };
            no.cubo = atual;
            let indice = ((endereco as usize) << 3) + filho as usize;
            let Some(&bruto) = self.nos.get(indice) else {
                // Índice fora do arquivo: trata como bloqueado, que é o que não deixa passar.
                no.estado = 1;
                return no;
            };
            no.estado = Self::estado(bruto);
            no.primeiro_irmao = endereco;
            no.posicao_entre_irmaos = filho;
            endereco = Self::filhos(bruto);
            if endereco == 0 {
                return no;
            }
            no.nivel += 1;
        }
    }
}

/// `CGlobalSPMap`: a grade de octrees de um mapa.
#[derive(Debug, Clone, Default)]
pub struct MapaDoEspaco {
    largura: i32,
    comprimento: i32,
    tamanho: i32,
    voxel: i32,
    submapas: Vec<Octree>,
}

impl MapaDoEspaco {
    pub fn vazio() -> Self {
        Self::default()
    }

    pub fn tem_dados(&self) -> bool {
        !self.submapas.is_empty()
    }

    pub fn tamanho_do_voxel(&self) -> i32 {
        self.voxel
    }

    /// `spmap.conf`: `Map Width`, `Map Length`, `Submap Size`, `Voxel Size`.
    fn ler_conf(texto: &str) -> Option<(i32, i32, i32, i32)> {
        let mut v = [None; 4];
        for linha in texto.lines() {
            let Some((k, valor)) = linha.split_once('=') else { continue };
            let n: Option<i32> = valor.trim().parse().ok();
            match k.trim() {
                "Map Width" => v[0] = n,
                "Map Length" => v[1] = n,
                "Submap Size" => v[2] = n,
                "Voxel Size" => v[3] = n,
                _ => {}
            }
        }
        Some((v[0]?, v[1]?, v[2]?, v[3]?))
    }

    /// Lê `dir/airmap/` (`CGlobalSPMap::Load`, `GlobalSPMap.cpp:59-143`). Pasta ausente: sem
    /// espaço, e quem consulta cai no comportamento sem mapa.
    pub fn ler(world_id: i32, dir: &Path) -> Self {
        let pasta = dir.join("airmap");
        // A última linha do `spmap.conf` é um comentário em GBK: lido como bytes.
        let Ok(texto) = std::fs::read(pasta.join("spmap.conf")).map(|b| String::from_utf8_lossy(&b).into_owned()) else {
            debug!("airmap: mapa {world_id} sem pasta airmap — segue sem espaço aéreo");
            return Self::vazio();
        };
        let Some((largura, comprimento, tamanho, voxel)) = Self::ler_conf(&texto) else {
            warn!("airmap: spmap.conf do mapa {world_id} sem as quatro medidas");
            return Self::vazio();
        };
        if largura <= 0 || comprimento <= 0 || tamanho <= 0 || voxel <= 0 {
            warn!("airmap: medidas inválidas no mapa {world_id}");
            return Self::vazio();
        }
        // A origem global (o canto de baixo à esquerda) e o centro de cada submapa livre.
        let origem = [(tamanho * largura) >> 1, 0, (tamanho * comprimento) >> 1];
        let meio = tamanho >> 1;
        let mut submapas = Vec::with_capacity((largura * comprimento) as usize);
        let (mut lidos, mut recusados) = (0usize, 0usize);
        for linha in 0..comprimento {
            for coluna in 0..largura {
                let id = (comprimento - linha - 1) * largura + coluna + 1;
                let livre = Octree {
                    id: id as u8,
                    cubo: Cubo {
                        centro: [coluna * tamanho + meio - origem[0], meio, linha * tamanho + meio - origem[2]],
                        meio,
                    },
                    folha: voxel,
                    nos: Vec::new(),
                };
                let o = match std::fs::read(pasta.join(format!("{id}.octr"))) {
                    Ok(bytes) => match Octree::ler(&bytes) {
                        Some(o) if o.cubo.meio * 2 == tamanho => {
                            lidos += 1;
                            o
                        }
                        _ => {
                            recusados += 1;
                            livre
                        }
                    },
                    Err(_) => livre,
                };
                submapas.push(o);
            }
        }
        info!("airmap: mapa {world_id} com {lidos} octrees ({recusados} recusadas) de {}", largura * comprimento);
        Self { largura, comprimento, tamanho, voxel, submapas }
    }

    fn submapa(&self, linha: i32, coluna: i32) -> Option<&Octree> {
        if linha < 0 || linha >= self.comprimento || coluna < 0 || coluna >= self.largura {
            return None;
        }
        self.submapas.get((linha * self.largura + coluna) as usize)
    }

    /// `LocateSPMap(Pos3DInt)`: `y` entre 0 e o tamanho; `row = z / tamanho`.
    fn localizar(&self, p: [i32; 3]) -> Option<&Octree> {
        let base = self.submapas.first()?;
        if p[1] < 0 || p[1] > self.tamanho {
            return None;
        }
        let x = p[0] - base.cubo.centro[0] + (self.tamanho >> 1);
        let z = p[2] - base.cubo.centro[2] + (self.tamanho >> 1);
        self.submapa(z.div_euclid(self.tamanho), x.div_euclid(self.tamanho))
    }

    /// `LocateSPMap(A3DVECTOR3)`, com o arredondamento para inteiro do original.
    fn localizar_f(&self, v: [f32; 3]) -> Option<&Octree> {
        let base = self.submapas.first()?;
        if v[1] < 0.0 || v[1] > self.tamanho as f32 {
            return None;
        }
        let x = v[0] - base.cubo.centro[0] as f32 + (self.tamanho >> 1) as f32;
        let z = v[2] - base.cubo.centro[2] as f32 + (self.tamanho >> 1) as f32;
        let r = 1.0 / self.tamanho as f32;
        self.submapa((z * r) as i32, (x * r) as i32)
    }

    /// `GetVoxelCenter` (`GlobalSPMap.h:118-146`).
    pub fn centro_do_voxel(&self, v: [f32; 3]) -> [i32; 3] {
        let mut p = [v[0] as i32, v[1] as i32, v[2] as i32];
        let Some(o) = self.localizar_f(v) else { return p };
        let r = 1.0 / self.voxel as f32;
        let meio = self.voxel >> 1;
        for i in 0..3 {
            let d = v[i] - o.cubo.centro[i] as f32;
            p[i] = (d * r) as i32 * self.voxel + if d > 0.0 { meio } else { -meio } + o.cubo.centro[i];
        }
        p
    }

    /// `GetTraversalNode(Pos3DInt)`: `None` fora do mapa.
    pub fn no(&self, p: [i32; 3]) -> Option<No> {
        self.localizar(p).map(|o| o.no(p))
    }

    /// `IsPosPassable(Pos3DInt)` sem o teste extra: dentro do mapa e numa folha livre.
    pub fn livre(&self, p: [i32; 3]) -> Option<No> {
        self.no(p).filter(|n| n.estado == LIVRE)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn octr(id: u8, centro: [i32; 3], meio: i32, nos: &[u32]) -> Vec<u8> {
        let mut b = VERSAO_DO_OCTR.to_le_bytes().to_vec();
        b.push(id);
        for v in [centro[0], centro[1], centro[2], meio, 2] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&(nos.len() as u32).to_le_bytes());
        for n in nos {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b
    }

    #[test]
    fn o_octr_fecha_no_ultimo_byte() {
        let b = octr(7, [0, 512, 0], 512, &[0; 8]);
        assert_eq!(b.len(), 29 + 32);
        assert!(Octree::ler(&b).is_some());
        let mut sobra = b.clone();
        sobra.push(0);
        assert!(Octree::ler(&sobra).is_none(), "byte a mais não é o formato");
        assert!(Octree::ler(&b[..b.len() - 1]).is_none());
    }

    /// Raiz com um filho bloqueado (o de x+, y+, z+ = 7) e os outros livres.
    #[test]
    fn a_folha_bloqueada_e_a_livre() {
        let mut nos = [0u32; 8];
        nos[7] = 1 << 30;
        let o = Octree::ler(&octr(1, [0, 512, 0], 512, &nos)).unwrap();
        assert_eq!(o.no([10, 600, 10]).estado, 1);
        let livre = o.no([-10, 600, 10]);
        assert_eq!(livre.estado, LIVRE);
        assert_eq!(livre.posicao_entre_irmaos, 3, "x−, y+, z+ = 0b011");
        assert!(livre.vizinho_irmao(&o.no([-10, 600, -10])));
    }
}
