//! `movemap/` — o mapa de movimento dos monstros de chão: onde se pode pisar e **a que altura
//! acima do terreno** fica o piso ali (ponte, plataforma, estrutura de pedra).
//!
//! Porta de `gs/pathfinding/NPCMoveMap.{h,cpp}`, `BitImage.h` e `BlockImage.h`. No original é o
//! `NPCMoveMap::CMap` do plano (`world::GetMoveMap`), carregado de `gs.conf` → `[MoveMap] Path`
//! (`gs/mapresman.cpp:87-97`), e é por ele que o gerador de posição da área "no chão"
//! (`terrain_gen_pos::Generate`, `gs/npcgenerator.cpp:4299-4318`) põe o monstro **em cima**
//! da estrutura e não dentro dela:
//!
//! ```cpp
//! pos.y = offset;
//! if (path_finding::GetValidPos(plane, pos)) {   // pos.y += delta, se alcançável
//!     pos.y += plane->GetHeightAt(pos.x, pos.z); // + terreno
//!     return;
//! }
//! ```
//!
//! # Os arquivos
//!
//! - **`movemap.conf`**, texto — `Map Width`, `Map Length`, `Submap Width`, `Submap Length`,
//!   `Pixel Size` (`CNPCMoveMap::Load`, `NPCMoveMap.cpp:362-372`). No `realm_155`, o mundo 1
//!   tem 8×11 submapas de 1024×1024 pixels de 1 m.
//! - **`N.rmap`** — `CBitImage` (`BitImage.h:228-282`): `u32 versão` (1), `u32 tamanho`, e no
//!   corpo `i32 largura_em_bytes`, `i32 comprimento`, `i32 largura_da_imagem`,
//!   `i32 comprimento_da_imagem`, `f32 pixel` e os bits, um por pixel (bit `u & 7` do byte
//!   `v × largura_em_bytes + u >> 3`). Bit ligado = alcançável.
//! - **`N.dhmap`** — `CBlockImage<FIX16>` (`BlockImage.h:309-383`): `u32 versão` (1),
//!   `u32 tamanho`, e no corpo `i32 largura` e `i32 comprimento` **em blocos**, `i32 expoente`
//!   do lado do bloco, os dois tamanhos da imagem, `f32 pixel`, os ids de bloco
//!   (`largura × comprimento` `i32`, −1 = bloco zerado), `i32 n` e `n` blocos de
//!   `lado² × u16`. O valor é a altura acima do terreno em **1/64 m**
//!   (`FIX16TOFLOAT(x) = x / 64.0`, `BlockImage.h:30`).
//!
//! O nome do arquivo do submapa `(u, v)` é `(comprimento − v − 1) × largura + u + 1`
//! (`NPCMoveMap.cpp:380`), e submapa sem arquivo fica **todo alcançável com altura zero**
//! (`Init` → `InitNoneZero`/`InitZero`, `NPCMoveMap.cpp:115-123`).
//!
//! # A conta
//!
//! O centro do mapa é a origem: `u = (x + largura_total/2) / pixel`, idem `v` com `z`
//! (`GetMapPos`, `NPCMoveMap.h:93-100`; origem em `NPCMoveMap.h:84-88`). Fora da grade não é
//! alcançável (`IsPosValid`).
use std::path::Path;
use tracing::{debug, warn};

/// `BITIMAGE_VER` e `BLOCKIMAGE_VER` (`BitImage.h:24`, `BlockImage.h:37`).
const VERSAO: u32 = 1;
/// `NULL_ID` (`BlockImage.h:32`): bloco sem dados, altura zero.
const BLOCO_NULO: i32 = -1;

#[derive(Debug, Clone, Default)]
struct Bits {
    largura_em_bytes: usize,
    comprimento: usize,
    dados: Vec<u8>,
}

impl Bits {
    fn ler(b: &[u8]) -> Option<Self> {
        let i = |o: usize| b.get(o..o + 4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]));
        if u32::from_le_bytes(b.get(0..4)?.try_into().ok()?) != VERSAO {
            return None;
        }
        let tamanho = i(4)? as usize;
        if b.len() != 8 + tamanho {
            return None; // o arquivo fecha no último byte
        }
        let (lb, c) = (i(8)?, i(12)?);
        if lb <= 0 || c <= 0 {
            return None;
        }
        let (lb, c) = (lb as usize, c as usize);
        let inicio = 8 + 20;
        if tamanho != 20 + lb * c {
            return None;
        }
        Some(Self { largura_em_bytes: lb, comprimento: c, dados: b[inicio..inicio + lb * c].to_vec() })
    }

    /// `CBitImage::GetPixel` (`BitImage.h:90-99`).
    fn pixel(&self, u: usize, v: usize) -> bool {
        let byte = u >> 3;
        if byte >= self.largura_em_bytes || v >= self.comprimento {
            return false;
        }
        self.dados[v * self.largura_em_bytes + byte] & (1 << (u & 7)) != 0
    }
}

#[derive(Debug, Clone, Default)]
struct Blocos {
    largura: usize,
    expoente: u32,
    ids: Vec<i32>,
    blocos: Vec<Vec<u16>>,
}

impl Blocos {
    fn ler(b: &[u8]) -> Option<Self> {
        let i = |o: usize| b.get(o..o + 4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]));
        if u32::from_le_bytes(b.get(0..4)?.try_into().ok()?) != VERSAO {
            return None;
        }
        let tamanho = i(4)? as usize;
        if b.len() != 8 + tamanho {
            return None;
        }
        let (w, l, e) = (i(8)?, i(12)?, i(16)?);
        if w <= 0 || l <= 0 || !(0..16).contains(&e) {
            return None;
        }
        let (w, l, e) = (w as usize, l as usize, e as u32);
        let mut o = 8 + 24;
        let ids: Vec<i32> = (0..w * l).map(|k| i(o + 4 * k)).collect::<Option<_>>()?;
        o += 4 * w * l;
        let n = i(o)?;
        if n < 0 {
            return None;
        }
        o += 4;
        let lado = 1usize << e;
        let por_bloco = lado * lado * 2;
        if b.len() != o + n as usize * por_bloco {
            return None;
        }
        let blocos = (0..n as usize)
            .map(|k| {
                let ini = o + k * por_bloco;
                b[ini..ini + por_bloco].chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect()
            })
            .collect();
        if ids.iter().any(|&id| id != BLOCO_NULO && (id < 0 || id >= n)) {
            return None;
        }
        Some(Self { largura: w, expoente: e, ids, blocos })
    }

    /// `CBlockImage::GetPixel` (`BlockImage.h:78-96`), em metros.
    fn altura(&self, u: usize, v: usize) -> f32 {
        let (bu, bv) = (u >> self.expoente, v >> self.expoente);
        let Some(&id) = self.ids.get(bv * self.largura + bu) else { return 0.0 };
        if id == BLOCO_NULO {
            return 0.0;
        }
        let mascara = (1usize << self.expoente) - 1;
        let k = ((v & mascara) << self.expoente) + (u & mascara);
        self.blocos[id as usize][k] as f32 / 64.0
    }
}

/// Um submapa lido, ou `None` — "todo alcançável, altura zero", como o original.
#[derive(Debug, Clone, Default)]
struct Submapa {
    alcance: Bits,
    altura: Blocos,
}

/// O mapa de movimento de um mapa do jogo.
#[derive(Debug, Clone, Default)]
pub struct MapaDeMovimento {
    largura: i32,
    comprimento: i32,
    submapa_largura: i32,
    submapa_comprimento: i32,
    pixel: f32,
    submapas: Vec<Option<Submapa>>,
}

impl MapaDeMovimento {
    pub fn vazio() -> Self {
        Self::default()
    }

    /// Quantos submapas foram lidos (os que faltam valem "alcançável, altura zero").
    pub fn submapas_lidos(&self) -> usize {
        self.submapas.iter().filter(|s| s.is_some()).count()
    }

    pub fn tem_dados(&self) -> bool {
        !self.submapas.is_empty()
    }

    /// Lê `<dir>/movemap/`. Sem a pasta, o mapa fica vazio e quem pergunta usa só o terreno.
    pub fn ler(world_id: i32, dir: &Path) -> Self {
        let pasta = dir.join("movemap");
        let conf = pasta.join("movemap.conf");
        // O `movemap.conf` termina num comentário em GBK: lido como bytes, não como UTF-8.
        let Ok(bruto) = std::fs::read(&conf) else {
            debug!("movemap: mapa {world_id} sem pasta movemap — segue só com o terreno");
            return Self::vazio();
        };
        let texto = String::from_utf8_lossy(&bruto);
        let Some((largura, comprimento, sw, sl, pixel)) = Self::ler_conf(&texto) else {
            warn!("movemap: {} não tem as cinco medidas — mapa {world_id} só com o terreno", conf.display());
            return Self::vazio();
        };
        if largura <= 0 || comprimento <= 0 || sw <= 0 || sl <= 0 || pixel <= 0.0 {
            warn!("movemap: medidas inválidas em {} — mapa {world_id} só com o terreno", conf.display());
            return Self::vazio();
        }
        let mut submapas = vec![None; (largura * comprimento) as usize];
        let (mut lidos, mut recusados) = (0usize, 0usize);
        for v in 0..comprimento {
            for u in 0..largura {
                // `iFileID = (iLength - i - 1) * iWidth + j + 1`, com `i = v`, `j = u`.
                let id = (comprimento - v - 1) * largura + u + 1;
                let (Ok(r), Ok(d)) = (
                    std::fs::read(pasta.join(format!("{id}.rmap"))),
                    std::fs::read(pasta.join(format!("{id}.dhmap"))),
                ) else {
                    continue;
                };
                match (Bits::ler(&r), Blocos::ler(&d)) {
                    (Some(alcance), Some(altura)) => {
                        lidos += 1;
                        submapas[(v * largura + u) as usize] = Some(Submapa { alcance, altura });
                    }
                    // O original também volta ao estado padrão quando a carga falha
                    // (`NPCMoveMap.cpp:393-399`).
                    _ => {
                        recusados += 1;
                        warn!("movemap: {id}.rmap/.dhmap do mapa {world_id} não fecham com o formato; ignorados");
                    }
                }
            }
        }
        debug!("movemap: mapa {world_id} — {lidos}/{} submapas ({recusados} recusados)", largura * comprimento);
        Self { largura, comprimento, submapa_largura: sw, submapa_comprimento: sl, pixel, submapas }
    }

    /// As cinco medidas do `movemap.conf` (`fscanf("Map Width =%d\n")` etc.).
    fn ler_conf(texto: &str) -> Option<(i32, i32, i32, i32, f32)> {
        let valor = |chave: &str| {
            texto
                .lines()
                .find(|l| l.trim_start().starts_with(chave))
                .and_then(|l| l.split('=').nth(1))
                .map(|v| v.trim().to_string())
        };
        Some((
            valor("Map Width")?.parse().ok()?,
            valor("Map Length")?.parse().ok()?,
            valor("Submap Width")?.parse().ok()?,
            valor("Submap Length")?.parse().ok()?,
            valor("Pixel Size")?.parse().ok()?,
        ))
    }

    fn tamanho(&self) -> f32 {
        if self.tem_dados() { self.pixel } else { 1.0 }
    }

    fn origem(&self) -> (f32, f32) {
        if !self.tem_dados() {
            return (0.0, 0.0);
        }
        (
            (self.submapa_largura * self.largura) as f32 * self.pixel * 0.5,
            (self.submapa_comprimento * self.comprimento) as f32 * self.pixel * 0.5,
        )
    }

    /// O lado do pixel, em metros (`GetGroundPixelSize`).
    pub fn tamanho_do_pixel(&self) -> f32 {
        self.tamanho()
    }

    /// `GetMapPos` (`NPCMoveMap.h:93-100`): o pixel de um ponto do mundo. O `(int)` do C++
    /// trunca em direção ao zero.
    pub fn pixel_de(&self, x: f32, z: f32) -> (i32, i32) {
        let recip = if self.tamanho() == 1.0 { 1.0 } else { 1.0 / self.tamanho() };
        let (ox, oz) = self.origem();
        (((x + ox) * recip) as i32, ((z + oz) * recip) as i32)
    }

    /// `GetPixelCenter` (`NPCMoveMap.h:104-110`): o centro do pixel, em `x`/`z`.
    pub fn centro_do_pixel(&self, u: i32, v: i32) -> (f32, f32) {
        let (ox, oz) = self.origem();
        ((u as f32 + 0.5) * self.tamanho() - ox, (v as f32 + 0.5) * self.tamanho() - oz)
    }

    fn submapa_e_pixel(&self, u: i32, v: i32) -> Option<(Option<&Submapa>, usize, usize)> {
        // `IsPosValid`.
        if u < 0 || v < 0 || u >= self.largura * self.submapa_largura || v >= self.comprimento * self.submapa_comprimento {
            return None;
        }
        let (su, sv) = (u / self.submapa_largura, v / self.submapa_comprimento);
        let (pu, pv) = ((u % self.submapa_largura) as usize, (v % self.submapa_comprimento) as usize);
        Some((self.submapas[(sv * self.largura + su) as usize].as_ref(), pu, pv))
    }

    /// `IsPosReachable` (`NPCMoveMap.h:134-157`). Mapa sem dados: tudo alcançável.
    pub fn alcancavel(&self, u: i32, v: i32) -> bool {
        if !self.tem_dados() {
            return true;
        }
        match self.submapa_e_pixel(u, v) {
            None => false,
            Some((None, _, _)) => true,
            Some((Some(s), pu, pv)) => s.alcance.pixel(pu, pv),
        }
    }

    /// `GetPosDeltaHeight` (`NPCMoveMap.h:174-197`): a altura do piso acima do terreno no
    /// pixel, **alcançável ou não**; zero fora da grade.
    pub fn acima_no_pixel(&self, u: i32, v: i32) -> f32 {
        if !self.tem_dados() {
            return 0.0;
        }
        match self.submapa_e_pixel(u, v) {
            Some((Some(s), pu, pv)) => s.altura.altura(pu, pv),
            _ => 0.0,
        }
    }

    /// `IsPosNeighborsReachable(pos, adj)` (`NPCMoveMap.h:159-172`).
    pub fn vizinhos_alcancaveis(&self, (u, v): (i32, i32), adj: i32) -> bool {
        let r = 2 * adj + 1;
        (0..=r).any(|i| (0..=r).any(|j| self.alcancavel(u - adj + i, v - adj + j)))
    }

    /// `CNPCMoveMap::CanGoStraightForward(from, to, posStop)` (`NPCMoveMap.cpp:588-645`):
    /// Bresenham de `de` até `ate` (sem conferir o último pixel), e o último pixel livre antes
    /// do bloqueio — que, como no original, é o **anterior** ao pixel testado.
    pub fn reta_livre(&self, de: (i32, i32), ate: (i32, i32)) -> (bool, (i32, i32)) {
        let (mut x, mut y) = de;
        let (mut dx, mut dy) = (ate.0 - de.0, ate.1 - de.1);
        let (s1, s2) = (dx.signum(), dy.signum());
        dx = dx.abs();
        dy = dy.abs();
        let troca = dy > dx;
        if troca {
            std::mem::swap(&mut dx, &mut dy);
        }
        let mut e = 2 * dy - dx;
        let mut pos = de;
        let mut parada = de;
        for _ in 0..dx {
            parada = pos;
            pos = (x, y);
            if !self.alcancavel(x, y) {
                return (false, parada);
            }
            while e > 0 {
                if troca { x += s1 } else { y += s2 }
                e -= 2 * dx;
            }
            if troca { y += s2 } else { x += s1 }
            e += 2 * dy;
        }
        (true, parada)
    }

    /// `CNPCMoveMap::GetValid3DPos` (`NPCMoveMap.h:199-210`): a altura do piso **acima do
    /// terreno** em `(x, z)`, ou `None` quando o ponto não é alcançável (ou está fora do
    /// mapa). Mapa sem dados responde `Some(0.0)` — o terreno manda.
    pub fn acima_do_terreno(&self, x: f32, z: f32) -> Option<f32> {
        let (u, v) = self.pixel_de(x, z);
        self.alcancavel(u, v).then(|| self.acima_no_pixel(u, v))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn rmap(bits: &[u8], lb: i32, c: i32) -> Vec<u8> {
        let mut corpo = Vec::new();
        for v in [lb, c, lb * 8, c] {
            corpo.extend_from_slice(&v.to_le_bytes());
        }
        corpo.extend_from_slice(&1.0f32.to_le_bytes());
        corpo.extend_from_slice(bits);
        let mut b = VERSAO.to_le_bytes().to_vec();
        b.extend_from_slice(&(corpo.len() as u32).to_le_bytes());
        b.extend_from_slice(&corpo);
        b
    }

    #[test]
    fn o_bit_do_pixel_e_o_bit_u_e_7_do_byte() {
        // 16×2 pixels: linha 0 com o pixel 9 ligado; linha 1 toda desligada.
        let b = rmap(&[0x00, 0x02, 0x00, 0x00], 2, 2);
        let bits = Bits::ler(&b).expect("formato");
        assert!(bits.pixel(9, 0));
        assert!(!bits.pixel(8, 0));
        assert!(!bits.pixel(9, 1));
        // Um byte a mais e o arquivo não fecha.
        let mut sobra = b.clone();
        sobra.push(0);
        assert!(Bits::ler(&sobra).is_none());
    }

    #[test]
    fn a_altura_vem_em_sessenta_e_quatro_avos_e_bloco_nulo_e_zero() {
        // 2×1 blocos de 2×2 (expoente 1): o primeiro nulo, o segundo com 64, 128, 0, 32.
        let mut corpo = Vec::new();
        for v in [2i32, 1, 1, 4, 2] {
            corpo.extend_from_slice(&v.to_le_bytes());
        }
        corpo.extend_from_slice(&1.0f32.to_le_bytes());
        for id in [BLOCO_NULO, 0] {
            corpo.extend_from_slice(&id.to_le_bytes());
        }
        corpo.extend_from_slice(&1i32.to_le_bytes());
        for h in [64u16, 128, 0, 32] {
            corpo.extend_from_slice(&h.to_le_bytes());
        }
        let mut b = VERSAO.to_le_bytes().to_vec();
        b.extend_from_slice(&(corpo.len() as u32).to_le_bytes());
        b.extend_from_slice(&corpo);
        let blocos = Blocos::ler(&b).expect("formato");
        assert_eq!(blocos.altura(0, 0), 0.0, "bloco nulo");
        assert_eq!(blocos.altura(2, 0), 1.0);
        assert_eq!(blocos.altura(3, 0), 2.0);
        assert_eq!(blocos.altura(3, 1), 0.5);
    }

    #[test]
    fn mapa_vazio_deixa_o_terreno_mandar() {
        assert_eq!(MapaDeMovimento::vazio().acima_do_terreno(10.0, 10.0), Some(0.0));
    }
}
