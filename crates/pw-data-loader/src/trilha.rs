//! Trilhas de animação do Angelica (B276, Estúdio S5): `A3DTrackData<T>` (`Angelica3D/Header/A3DTrackData.h`).
//!
//! Cada trilha no arquivo: `A3DTRACKSAVEDATA` (`:36-41`, 12 bytes: chaves, segmentos, quadros por segundo —
//! `a3d_LoadTrackInfo`, `:49`), as chaves (`T` × n: `A3DVECTOR3` de 12 bytes para posição, `A3DQUATERNION` `x, y, z, w`
//! de 16 bytes para rotação) e os segmentos (`SEGMENT`, `:113-119`, 16 bytes: tempo inicial e final em ms, chave inicial
//! e final) — `A3DTrackData<T>::Load`, `:574`. Um osso tem posição e depois rotação (`A3DSklTrackSet::LoadBoneTrack`,
//! `A3DTrackMan.cpp:203`). Tempo → chave (`GetFloorKeyIndex`, `:347-380`): antes do 1º segmento, chave 0; depois do
//! último, a última; dentro, `chave_inicial + (t − inicio) × fps / 1000` (ou a chave inicial se o segmento tem uma só).

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Segmento {
    pub inicio_ms: i32,
    pub fim_ms: i32,
    pub chave_inicial: i32,
    pub chave_final: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Trilha<const N: usize> {
    pub fps: i32,
    #[serde(skip)]
    pub chaves: Vec<[f32; N]>,
    pub segmentos: Vec<Segmento>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrilhaDeOsso {
    pub posicao: Trilha<3>,
    /// Quatérnio `x, y, z, w` (a ordem da `union` de `A3DQUATERNION`, `A3DQuaternion.h:38-45`).
    pub rotacao: Trilha<4>,
}

/// Lê uma trilha a partir de `b[*o..]`; `None` se faltam bytes ou as contagens são absurdas.
pub fn ler<const N: usize>(b: &[u8], o: &mut usize) -> Option<Trilha<N>> {
    let u = |o: &mut usize| -> Option<i32> {
        let v = b.get(*o..*o + 4)?;
        *o += 4;
        Some(i32::from_le_bytes(v.try_into().unwrap()))
    };
    let n_chaves = u(o)?;
    let n_segmentos = u(o)?;
    let fps = u(o)?;
    if !(0..=1 << 20).contains(&n_chaves) || !(0..=1 << 16).contains(&n_segmentos) {
        return None;
    }
    let mut chaves = Vec::with_capacity(n_chaves as usize);
    for _ in 0..n_chaves {
        let mut k = [0.0f32; N];
        for x in &mut k {
            *x = f32::from_bits(u(o)? as u32);
        }
        chaves.push(k);
    }
    let mut segmentos = Vec::with_capacity(n_segmentos as usize);
    for _ in 0..n_segmentos {
        segmentos.push(Segmento { inicio_ms: u(o)?, fim_ms: u(o)?, chave_inicial: u(o)?, chave_final: u(o)? });
    }
    Some(Trilha { fps, chaves, segmentos })
}

/// Trilha de um osso: posição e depois rotação.
pub fn ler_osso(b: &[u8], o: &mut usize) -> Option<TrilhaDeOsso> {
    Some(TrilhaDeOsso { posicao: ler::<3>(b, o)?, rotacao: ler::<4>(b, o)? })
}

impl<const N: usize> Trilha<N> {
    /// Os pares (tempo em ms, chave) que a trilha define: cada chave dos segmentos no seu tempo
    /// (`inicio + (k − chave_inicial) × 1000 / fps`); segmento de uma chave só vale do início ao fim.
    pub fn amostras(&self) -> Vec<(f32, [f32; N])> {
        let mut r: Vec<(f32, [f32; N])> = Vec::new();
        for s in &self.segmentos {
            let ki = s.chave_inicial.max(0) as usize;
            let kf = s.chave_final.max(0) as usize;
            if ki >= self.chaves.len() {
                continue;
            }
            if ki == kf || self.fps <= 0 {
                r.push((s.inicio_ms as f32, self.chaves[ki]));
                r.push((s.fim_ms as f32, self.chaves[ki]));
                continue;
            }
            for k in ki..=kf.min(self.chaves.len() - 1) {
                let t = s.inicio_ms as f32 + (k - ki) as f32 * 1000.0 / self.fps as f32;
                r.push((t.min(s.fim_ms as f32), self.chaves[k]));
            }
        }
        // Tempos crescentes estritos (o glTF exige): a amostra repetida no mesmo tempo fica a última.
        let mut saida: Vec<(f32, [f32; N])> = Vec::with_capacity(r.len());
        for (t, k) in r {
            match saida.last_mut() {
                Some(u) if t <= u.0 => {
                    if t == u.0 {
                        *u = (t, k);
                    }
                }
                _ => saida.push((t, k)),
            }
        }
        saida
    }
}

/// B287 (S6): o inverso de `ler` — `A3DTRACKSAVEDATA` (chaves, segmentos, fps), as chaves e os segmentos
/// (`A3DTrackData<T>::Save`, `A3DTrackData.h:601-…`, mesma ordem do `Load`).
pub fn escrever<const N: usize>(t: &Trilha<N>, o: &mut Vec<u8>) {
    for x in [t.chaves.len() as i32, t.segmentos.len() as i32, t.fps] {
        o.extend_from_slice(&x.to_le_bytes());
    }
    for k in &t.chaves {
        k.iter().for_each(|v| o.extend_from_slice(&v.to_le_bytes()));
    }
    for s in &t.segmentos {
        for x in [s.inicio_ms, s.fim_ms, s.chave_inicial, s.chave_final] {
            o.extend_from_slice(&x.to_le_bytes());
        }
    }
}

/// Trilha de um osso: posição e depois rotação.
pub fn escrever_osso(t: &TrilhaDeOsso, o: &mut Vec<u8>) {
    escrever(&t.posicao, o);
    escrever(&t.rotacao, o);
}
