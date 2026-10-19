//! Leitor do `.bon` — o esqueleto de um modelo do Angelica (B274, Estúdio S5).
//!
//! Porte de `A3DSkeleton::Load` (`Angelica3D/Source/A3DSkeleton.cpp:409-560`, cliente 1.5.5): marca `MOXB` do `AFile`
//! (como no `.ski`), `A3DSKELETONFILEHEADER` (`:53-64`, 96 bytes: `'ASKE'`, versão ≤ 6, ossos, juntas, ganchos, quadros
//! inicial/final, FPS, 64 reservados); cada osso (`A3DBone::Load`, `A3DBone.cpp`): nome, `BONEDATA` (`A3DBone.h:96-104`:
//! flags `u8`, mãe, primeira junta, filhos, `matRelative` e `matBoneInit` 4×4 `f32`; sem `#pragma pack`, então 3 bytes
//! de enchimento depois das flags — 144 bytes) e os índices das filhas (`i32`); os tipos das juntas (`u32` × n) e cada
//! junta (`A3DJoint::Load`, `A3DJoint.cpp:160`: nome e `JOINTDATA` de 12 bytes, `A3DJoint.h:52-57` — os quatro tipos
//! leem só isso); os ganchos (`A3DSkeletonHook::Load`, `:114`: nome e `HOOKDATA` de 72 bytes, `A3DSkeleton.h:67-72`).
//! Antes da versão 6 cada junta traz uma trilha de animação embutida (`LoadBoneTrack`): ainda não lida (recusa com o
//! motivo; o teste mede quantos são).
//!
//! As matrizes são do Direct3D: linha × matriz, translação na 4ª linha.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum BonError {
    #[error("arquivo curto: precisava de {0} bytes em {1}")]
    Curto(usize, usize),
    #[error("não é .bon (identidade {0:#x})")]
    Identidade(u32),
    #[error("versão {0} maior que 6 (SKEFILE_VERSION)")]
    Versao(u32),
    #[error("versão {0} < 6 traz trilhas embutidas nas juntas (ainda não lidas)")]
    TrilhasEmbutidas(u32),
    #[error("contagem absurda: {0} = {1}")]
    Contagem(&'static str, i32),
    #[error("sobraram {0} bytes depois do último gancho")]
    Sobra(usize),
}

pub type Result<T> = std::result::Result<T, BonError>;

const IDENTIDADE: u32 = (b'A' as u32) << 24 | (b'S' as u32) << 16 | (b'K' as u32) << 8 | b'E' as u32;
pub type Matriz = [f32; 16];

#[derive(Debug, Clone, Serialize)]
pub struct Osso {
    pub nome: String,
    pub flags: u8,
    pub mae: i32,
    pub primeira_junta: i32,
    /// `matRelative`: leva do espaço do osso ao da mãe (a pose de ligação local).
    pub relativa: Matriz,
    /// `matBoneInit`: o deslocamento inicial (medido em `bon::testes`: é a inversa da pose de ligação no mundo).
    pub inicial: Matriz,
    pub filhas: Vec<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Junta {
    pub tipo: u32,
    pub nome: String,
    pub osso_mae: i32,
    pub osso_filho: i32,
    pub irma: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Gancho {
    pub nome: String,
    pub tipo: u32,
    pub osso: i32,
    pub matriz: Matriz,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bon {
    pub versao: u32,
    pub quadro_inicial: i32,
    pub quadro_final: i32,
    pub fps: i32,
    pub ossos: Vec<Osso>,
    pub juntas: Vec<Junta>,
    pub ganchos: Vec<Gancho>,
}

struct Leitor<'a> {
    b: &'a [u8],
    o: usize,
}

impl<'a> Leitor<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let f = self.b.get(self.o..self.o + n).ok_or(BonError::Curto(n, self.o))?;
        self.o += n;
        Ok(f)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn matriz(&mut self) -> Result<Matriz> {
        let mut m = [0.0; 16];
        for x in &mut m {
            *x = f32::from_bits(self.u32()?);
        }
        Ok(m)
    }
    fn contagem(&mut self, campo: &'static str, limite: i32) -> Result<usize> {
        let n = self.i32()?;
        if !(0..=limite).contains(&n) {
            return Err(BonError::Contagem(campo, n));
        }
        Ok(n as usize)
    }
    fn texto(&mut self) -> Result<String> {
        let n = self.contagem("tamanho de texto", 1 << 16)?;
        Ok(crate::ski::gbk(self.bytes(n)?))
    }
}

impl Bon {
    pub fn ler(b: &[u8]) -> Result<Self> {
        let inicio = if b.starts_with(b"MOXB") { 4 } else { 0 };
        let mut l = Leitor { b, o: inicio };
        let identidade = l.u32()?;
        if identidade != IDENTIDADE {
            return Err(BonError::Identidade(identidade));
        }
        let versao = l.u32()?;
        if versao > 6 {
            return Err(BonError::Versao(versao));
        }
        let n_ossos = l.contagem("ossos", 4096)?;
        let n_juntas = l.contagem("juntas", 4096)?;
        let n_ganchos = l.contagem("ganchos", 4096)?;
        let quadro_inicial = l.i32()?;
        let quadro_final = l.i32()?;
        let fps = l.i32()?;
        l.bytes(64)?;
        let mut ossos = Vec::with_capacity(n_ossos);
        for _ in 0..n_ossos {
            let nome = l.texto()?;
            let flags = l.bytes(4)?[0]; // u8 + 3 de enchimento
            let mae = l.i32()?;
            let primeira_junta = l.i32()?;
            let n_filhas = l.contagem("filhas", 4096)?;
            let relativa = l.matriz()?;
            let inicial = l.matriz()?;
            let filhas = (0..n_filhas).map(|_| l.i32()).collect::<Result<_>>()?;
            ossos.push(Osso { nome, flags, mae, primeira_junta, relativa, inicial, filhas });
        }
        let mut juntas = Vec::with_capacity(n_juntas);
        if n_juntas > 0 {
            if versao < 6 {
                return Err(BonError::TrilhasEmbutidas(versao));
            }
            let tipos: Vec<u32> = (0..n_juntas).map(|_| l.u32()).collect::<Result<_>>()?;
            for tipo in tipos {
                let nome = l.texto()?;
                juntas.push(Junta { tipo, nome, osso_mae: l.i32()?, osso_filho: l.i32()?, irma: l.i32()? });
            }
        }
        let mut ganchos = Vec::with_capacity(n_ganchos);
        for _ in 0..n_ganchos {
            let nome = l.texto()?;
            ganchos.push(Gancho { nome, tipo: l.u32()?, osso: l.i32()?, matriz: l.matriz()? });
        }
        if l.o != b.len() {
            return Err(BonError::Sobra(b.len() - l.o));
        }
        Ok(Self { versao, quadro_inicial, quadro_final, fps, ossos, juntas, ganchos })
    }

    /// A pose de ligação de cada osso no espaço do modelo: `relativa(osso) × … × relativa(raiz)` (linha × matriz).
    pub fn poses_no_mundo(&self) -> Vec<Matriz> {
        let mut r: Vec<Option<Matriz>> = vec![None; self.ossos.len()];
        fn calc(i: usize, ossos: &[Osso], r: &mut Vec<Option<Matriz>>, prof: usize) -> Matriz {
            if let Some(m) = r[i] {
                return m;
            }
            let o = &ossos[i];
            let m = if o.mae >= 0 && (o.mae as usize) < ossos.len() && prof < 512 {
                multiplicar(&o.relativa, &calc(o.mae as usize, ossos, r, prof + 1))
            } else {
                o.relativa
            };
            r[i] = Some(m);
            m
        }
        (0..self.ossos.len()).map(|i| calc(i, &self.ossos, &mut r, 0)).collect()
    }
}

/// `a × b` com matrizes em ordem de linha (D3D).
pub fn multiplicar(a: &Matriz, b: &Matriz) -> Matriz {
    let mut r = [0.0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            r[i * 4 + j] = (0..4).map(|k| a[i * 4 + k] * b[k * 4 + j]).sum();
        }
    }
    r
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Todos os `.bon` do cliente 1.5.5 BR: quantos fecham no último byte, as versões, e se `matBoneInit` é a inversa
    /// da pose de ligação no mundo (`poses_no_mundo × inicial ≈ identidade`).
    #[test]
    fn todos_os_bon_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut ok, mut versoes, mut motivos) = (0usize, std::collections::BTreeMap::new(), std::collections::BTreeMap::<String, (usize, String)>::new());
        let (mut ossos, mut inversa, mut alguma, mut variantes) = (0usize, 0usize, 0usize, [0usize; 4]);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    pilha.push(p);
                    continue;
                }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bon")) {
                    continue;
                }
                match Bon::ler(&std::fs::read(&p).unwrap()) {
                    Ok(s) => {
                        ok += 1;
                        *versoes.entry(s.versao).or_insert(0) += 1;
                        // Variantes: mundo pela ordem filho×mãe (D3D) e mãe×filho; produto nas duas ordens.
                        let mut alt: Vec<Option<Matriz>> = vec![None; s.ossos.len()];
                        fn mundo2(i: usize, s: &Bon, r: &mut Vec<Option<Matriz>>, prof: usize) -> Matriz {
                            if let Some(m) = r[i] { return m; }
                            let o = &s.ossos[i];
                            let m = if o.mae >= 0 && (o.mae as usize) < s.ossos.len() && prof < 512 { multiplicar(&mundo2(o.mae as usize, s, r, prof + 1), &o.relativa) } else { o.relativa };
                            r[i] = Some(m);
                            m
                        }
                        let perto = |m: Matriz| (0..16).map(|k| (m[k] - if k % 5 == 0 { 1.0 } else { 0.0 }).abs()).fold(0.0f32, f32::max) < 1e-2;
                        for (i, (w, o)) in s.poses_no_mundo().iter().zip(&s.ossos).enumerate() {
                            ossos += 1;
                            let w2 = mundo2(i, &s, &mut alt, 0);
                            let casos = [perto(multiplicar(&o.inicial, w)), perto(multiplicar(w, &o.inicial)), perto(multiplicar(&o.inicial, &w2)), perto(multiplicar(&w2, &o.inicial))];
                            if casos[0] { inversa += 1; }
                            for (k, c) in casos.iter().enumerate() { if *c { variantes[k] += 1; } }
                            if casos.iter().any(|c| *c) { alguma += 1; }
                        }
                    }
                    Err(e) => {
                        let chave = format!("{e}").split(':').next().unwrap().to_string();
                        let x = motivos.entry(chave).or_insert((0, String::new()));
                        x.0 += 1;
                        if x.1.is_empty() {
                            x.1 = p.display().to_string();
                        }
                    }
                }
            }
        }
        eprintln!("bon: {ok} fecham; versões {versoes:?}; ossos {ossos}, com inicial × mundo ≈ I: {inversa}; variantes [i·w, w·i, i·w2, w2·i] {variantes:?}; alguma {alguma}; falhas {motivos:#?}");
        assert!(ok > 0);
    }
}
