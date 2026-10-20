//! Leitor do `.bon` — o esqueleto de um modelo do Angelica (B274, Estúdio S5).
//!
//! Porte de `A3DSkeleton::Load` (`Angelica3D/Source/A3DSkeleton.cpp:409-560`, cliente 1.5.5): marca `MOXB` do `AFile`
//! (como no `.ski`), `A3DSKELETONFILEHEADER` (`:53-64`, 96 bytes: `'ASKE'`, versão ≤ 6, ossos, juntas, ganchos, quadros
//! inicial/final, FPS, 64 reservados); cada osso (`A3DBone::Load`, `A3DBone.cpp`): nome, `BONEDATA` (`A3DBone.h:96-104`:
//! flags `u8`, mãe, primeira junta, filhos, `matRelative` e `matBoneInit` 4×4 `f32`; sem `#pragma pack`, então 3 bytes
//! de enchimento depois das flags — 144 bytes) e os índices das filhas (`i32`); os tipos das juntas (`u32` × n) e cada
//! junta (`A3DJoint::Load`, `A3DJoint.cpp:160`: nome e `JOINTDATA` de 12 bytes, `A3DJoint.h:52-57` — os quatro tipos
//! leem só isso); os ganchos (`A3DSkeletonHook::Load`, `:114`: nome e `HOOKDATA` de 72 bytes, `A3DSkeleton.h:67-72`).
//! Antes da versão 6 cada junta traz, logo depois dos seus dados, a trilha de animação do osso (`LoadBoneTrack`,
//! `A3DSkeleton.cpp:495-508`; formato em `crate::trilha`) — B276: lida em `Bon::trilhas`.
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
    #[error("trilha embutida da junta {0} ilegível")]
    Trilha(usize),
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
    /// B287 (escrita): os 3 bytes de enchimento depois das flags em `BONEDATA` (o compilador não os zera).
    #[serde(skip)]
    pub enchimento: [u8; 3],
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
    /// Versão < 6: a trilha de cada junta, na ordem das juntas (B276). Versão 6: vazio (as trilhas ficam nos `.stck`).
    #[serde(skip)]
    pub trilhas: Vec<crate::trilha::TrilhaDeOsso>,
    pub ganchos: Vec<Gancho>,
    /// B287 (escrita): a marca `MOXB` do `AFile` e os 64 bytes reservados de `A3DSKELETONFILEHEADER`.
    #[serde(skip)]
    pub marca: bool,
    #[serde(skip)]
    pub reservado: Vec<u8>,
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
        let reservado = l.bytes(64)?.to_vec();
        let mut ossos = Vec::with_capacity(n_ossos);
        for _ in 0..n_ossos {
            let nome = l.texto()?;
            let f4 = l.bytes(4)?; // u8 + 3 de enchimento
            let (flags, enchimento) = (f4[0], [f4[1], f4[2], f4[3]]);
            let mae = l.i32()?;
            let primeira_junta = l.i32()?;
            let n_filhas = l.contagem("filhas", 4096)?;
            let relativa = l.matriz()?;
            let inicial = l.matriz()?;
            let filhas = (0..n_filhas).map(|_| l.i32()).collect::<Result<_>>()?;
            ossos.push(Osso { nome, flags, mae, primeira_junta, relativa, inicial, filhas, enchimento });
        }
        let mut juntas = Vec::with_capacity(n_juntas);
        let mut trilhas = Vec::new();
        if n_juntas > 0 {
            let tipos: Vec<u32> = (0..n_juntas).map(|_| l.u32()).collect::<Result<_>>()?;
            for (i, tipo) in tipos.into_iter().enumerate() {
                let nome = l.texto()?;
                juntas.push(Junta { tipo, nome, osso_mae: l.i32()?, osso_filho: l.i32()?, irma: l.i32()? });
                if versao < 6 {
                    trilhas.push(crate::trilha::ler_osso(l.b, &mut l.o).ok_or(BonError::Trilha(i))?);
                }
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
        Ok(Self { versao, quadro_inicial, quadro_final, fps, ossos, juntas, trilhas, ganchos, marca: inicio == 4, reservado })
    }

    /// B287 (S6): o inverso de `ler` — `A3DSkeleton::Save` (`A3DSkeleton.cpp:585-677`): cabeçalho de 96 bytes, ossos
    /// (`A3DBone::Save`, `A3DBone.cpp:585-…`: nome, `BONEDATA` de 144 bytes, filhas), tipos das juntas, juntas (nome e 12
    /// bytes) e ganchos (nome e 72 bytes). O `Save` do cliente sempre grava a versão 6; para reescrever um `.bon` v5
    /// (todo o 1.2.6) igual, a trilha de cada junta vai logo depois dela, como o `Load` lê (`:495-508`).
    pub fn escrever(&self) -> Vec<u8> {
        let mut o = Vec::new();
        let i = |o: &mut Vec<u8>, x: i32| o.extend_from_slice(&x.to_le_bytes());
        let m = |o: &mut Vec<u8>, x: &Matriz| x.iter().for_each(|v| o.extend_from_slice(&v.to_le_bytes()));
        let texto = |o: &mut Vec<u8>, t: &str| {
            let b = encoding_rs::GBK.encode(t).0;
            o.extend_from_slice(&(b.len() as i32).to_le_bytes());
            o.extend_from_slice(&b);
        };
        if self.marca {
            o.extend_from_slice(b"MOXB");
        }
        i(&mut o, IDENTIDADE as i32);
        i(&mut o, self.versao as i32);
        for x in [self.ossos.len() as i32, self.juntas.len() as i32, self.ganchos.len() as i32, self.quadro_inicial, self.quadro_final, self.fps] {
            i(&mut o, x);
        }
        let mut res = self.reservado.clone();
        res.resize(64, 0);
        o.extend_from_slice(&res);
        for b in &self.ossos {
            texto(&mut o, &b.nome);
            o.push(b.flags);
            o.extend_from_slice(&b.enchimento);
            for x in [b.mae, b.primeira_junta, b.filhas.len() as i32] {
                i(&mut o, x);
            }
            m(&mut o, &b.relativa);
            m(&mut o, &b.inicial);
            b.filhas.iter().for_each(|f| i(&mut o, *f));
        }
        if !self.juntas.is_empty() {
            self.juntas.iter().for_each(|j| i(&mut o, j.tipo as i32));
            for (k, j) in self.juntas.iter().enumerate() {
                texto(&mut o, &j.nome);
                for x in [j.osso_mae, j.osso_filho, j.irma] {
                    i(&mut o, x);
                }
                if self.versao < 6 {
                    if let Some(t) = self.trilhas.get(k) {
                        crate::trilha::escrever_osso(t, &mut o);
                    }
                }
            }
        }
        for g in &self.ganchos {
            texto(&mut o, &g.nome);
            i(&mut o, g.tipo as i32);
            i(&mut o, g.osso);
            m(&mut o, &g.matriz);
        }
        o
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

    /// B287: todo `.bon` que o leitor aceita (v5 com trilhas embutidas e v6), reescrito, sai byte a byte igual.
    #[test]
    fn escrever_reproduz_o_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut lidos, mut iguais, mut recusados) = (0usize, 0usize, 0usize);
        let mut por_versao = std::collections::BTreeMap::<u32, usize>::new();
        let mut diferentes: Vec<String> = Vec::new();
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bon")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let Ok(s) = Bon::ler(&b) else { recusados += 1; continue };
                lidos += 1;
                *por_versao.entry(s.versao).or_default() += 1;
                let w = s.escrever();
                if w == b {
                    iguais += 1;
                } else if diferentes.len() < 5 {
                    let o = w.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(w.len().min(b.len()));
                    diferentes.push(format!("{} (tam {} × {}, 1º byte diferente em {o})", p.display(), w.len(), b.len()));
                }
            }
        }
        eprintln!("bon: lidos {lidos} {por_versao:?}, reescritos iguais {iguais}, recusados {recusados}; diferentes: {diferentes:?}");
        assert!(lidos > 1000, "{lidos}");
        assert_eq!(iguais, lidos, "{diferentes:?}");
    }

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
        assert!(motivos.is_empty(), "B276: v5 com trilhas também fecha: {motivos:#?}");
        eprintln!("bon: {ok} fecham; versões {versoes:?}; ossos {ossos}, com inicial × mundo ≈ I: {inversa}; variantes [i·w, w·i, i·w2, w2·i] {variantes:?}; alguma {alguma}; falhas {motivos:#?}");
        assert!(ok > 0);
    }
}
