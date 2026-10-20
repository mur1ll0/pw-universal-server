//! O começo do `.smd` — o modelo de pele que liga um esqueleto às peles (B274, Estúdio S5).
//!
//! `A3DSkinModelMan::LoadModelFromFile` (`Angelica3D/Source/A3DSkinModelMan.cpp`): marca `MOXB` do `AFile`,
//! `A3DSKINMODELFILEHEADER` (`A3DSkinModel.h:48-57`, 84 bytes: `'ASMD'`, versão ≤ 8, peles, ações, cabides,
//! propriedades, 60 reservados), o nome do esqueleto e o de cada pele (strings `i32` + bytes), relativos à pasta do
//! `.smd` (`strPath + "\\" + strFile`). B276: depois das peles, o arquivo de física (string), a pasta das trilhas
//! (string a partir da v8; antes, `tcks_<título>`) e as ações (`A3DSkinModelActionCore::Load`,
//! `A3DSkinModelAct.cpp:140-215`): nome; v < 6 — `FILEACTIONDATA` (12 bytes: grupo, juntas, flags) + juntas ×
//! `FILEJOINTACTION` (20 bytes: junta, quadro inicial e final `f32`, laços, velocidade), os quadros da 1ª; v ≥ 6 —
//! `FILEACTIONDATA6` (quadro inicial e final `f32`); v ≥ 7 — o `.stck` (relativo à pasta das trilhas). Tempo =
//! `quadro × 1000 / fps` do esqueleto (`FrameToTime`, `:301-307`). Com `.smd` v < 7 e esqueleto v ≥ 6 o motor descarta
//! as ações (`LoadModelFromFile`). Cabides e propriedades não são lidos.

#[derive(Debug, thiserror::Error)]
pub enum SmdError {
    #[error("arquivo curto")]
    Curto,
    #[error("não é .smd (identidade {0:#x})")]
    Identidade(u32),
    #[error("versão {0} maior que 8 (SKMDFILE_VERSION)")]
    Versao(u32),
    #[error("contagem absurda: {0}")]
    Contagem(i32),
    #[error("sobraram {0} bytes depois das propriedades")]
    Sobra(usize),
}

const IDENTIDADE: u32 = (b'A' as u32) << 24 | (b'S' as u32) << 16 | (b'M' as u32) << 8 | b'D' as u32;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CabecaDoSmd {
    pub versao: u32,
    pub acoes: i32,
    /// O nome do `.bon` como está no arquivo (relativo à pasta do `.smd`); vazio = sem esqueleto.
    pub esqueleto: String,
    pub peles: Vec<String>,
    /// B276: a pasta das trilhas (relativa à do `.smd`) e as ações (se o arquivo as tem inteiras).
    pub pasta_das_trilhas: String,
    pub lista_de_acoes: Vec<Acao>,
}

/// Uma ação do modelo: quadros (a converter em ms com o FPS do esqueleto) e, v ≥ 7, o `.stck` dela.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Acao {
    pub nome: String,
    pub quadro_inicial: f32,
    pub quadro_final: f32,
    pub stck: Option<String>,
}

pub fn cabeca(b: &[u8]) -> Result<CabecaDoSmd, SmdError> {
    let mut o = if b.starts_with(b"MOXB") { 4 } else { 0 };
    let u32_ = |o: &mut usize| -> Result<u32, SmdError> {
        let v = b.get(*o..*o + 4).ok_or(SmdError::Curto)?;
        *o += 4;
        Ok(u32::from_le_bytes(v.try_into().unwrap()))
    };
    let id = u32_(&mut o)?;
    if id != IDENTIDADE {
        return Err(SmdError::Identidade(id));
    }
    let versao = u32_(&mut o)?;
    if versao > 8 {
        return Err(SmdError::Versao(versao));
    }
    let n_peles = u32_(&mut o)? as i32;
    let acoes = u32_(&mut o)? as i32;
    o += 8 + 60; // cabides, propriedades, reservado
    if !(0..=256).contains(&n_peles) {
        return Err(SmdError::Contagem(n_peles));
    }
    let texto = |o: &mut usize| -> Result<String, SmdError> {
        let n = u32_(o)? as usize;
        if n > 1 << 12 {
            return Err(SmdError::Contagem(n as i32));
        }
        let s = b.get(*o..*o + n).ok_or(SmdError::Curto)?;
        *o += n;
        Ok(crate::ski::gbk(s))
    };
    let esqueleto = texto(&mut o)?;
    let peles = (0..n_peles).map(|_| texto(&mut o)).collect::<Result<_, _>>()?;
    let _fisica = texto(&mut o)?;
    let pasta_das_trilhas = if versao >= 8 { texto(&mut o)? } else { String::new() };
    let f32_ = |o: &mut usize| -> Result<f32, SmdError> { Ok(f32::from_bits(u32_(o)?)) };
    let mut lista_de_acoes = Vec::with_capacity(acoes.max(0) as usize);
    for _ in 0..acoes.max(0) {
        let nome = texto(&mut o)?;
        let (qi, qf) = if versao < 6 {
            let _grupo = u32_(&mut o)?;
            let juntas = u32_(&mut o)? as usize;
            let _flags = u32_(&mut o)?;
            if juntas > 4096 {
                return Err(SmdError::Contagem(juntas as i32));
            }
            let mut q = (0.0, 0.0);
            for j in 0..juntas {
                let _junta = u32_(&mut o)?;
                let (a, b) = (f32_(&mut o)?, f32_(&mut o)?);
                let _lacos = u32_(&mut o)?;
                let _velocidade = u32_(&mut o)?;
                if j == 0 {
                    q = (a, b);
                }
            }
            q
        } else {
            (f32_(&mut o)?, f32_(&mut o)?)
        };
        let stck = if versao >= 7 { Some(texto(&mut o)?) } else { None };
        lista_de_acoes.push(Acao { nome, quadro_inicial: qi, quadro_final: qf, stck });
    }
    Ok(CabecaDoSmd { versao, acoes, esqueleto, peles, pasta_das_trilhas, lista_de_acoes })
}

/// B288 (S6): o `.smd` inteiro, para reescrever. Depois das ações (`cabeca`), os cabides (`A3DSkinModel::LoadHanger`,
/// `A3DSkinModel.cpp:851-…`: tipo `i32` — 0 GFX, 1 modelo, `A3DSkinModelAux.h:56-58` —, nome, osso/gancho, `HANGERDATA`
/// de 8 bytes — `int` + `bool` sem `#pragma pack`, `:62-66` —, e o arquivo: GFX `A3DGFXHanger::Load` `:209`, modelo
/// `A3DSkinModelHanger::Load` `:304` + o nome do CC) e as propriedades (nome e valor, `LoadModelFromFile`,
/// `A3DSkinModelMan.cpp:321-331`). Cada ação guarda os bytes dos dados como estão (`FILEACTIONDATA` + juntas na v < 6,
/// `FILEACTIONDATA6` na v ≥ 6; `SkipFileData`, `A3DSkinModelAct.cpp`) e o `.stck` (v ≥ 7).
#[derive(Debug, Clone)]
pub struct Smd {
    pub marca: bool,
    pub versao: u32,
    pub reservado: Vec<u8>,
    pub esqueleto: String,
    pub peles: Vec<String>,
    pub fisica: String,
    /// v ≥ 8.
    pub pasta_das_trilhas: Option<String>,
    pub acoes: Vec<AcaoBruta>,
    pub cabides: Vec<Cabide>,
    pub propriedades: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct AcaoBruta {
    pub nome: String,
    pub dados: Vec<u8>,
    pub stck: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Cabide {
    pub tipo: i32,
    pub nome: String,
    pub preso_a: String,
    pub dados: [u8; 8],
    pub arquivo: String,
    /// Só no cabide de modelo (tipo 1): o gancho de encaixe do filho.
    pub cc: Option<String>,
}

impl Smd {
    pub fn ler(b: &[u8]) -> Result<Smd, SmdError> {
        let marca = b.starts_with(b"MOXB");
        let mut o = if marca { 4 } else { 0 };
        let bytes = |o: &mut usize, n: usize| -> Result<Vec<u8>, SmdError> {
            let v = b.get(*o..*o + n).ok_or(SmdError::Curto)?.to_vec();
            *o += n;
            Ok(v)
        };
        let i32_ = |o: &mut usize| -> Result<i32, SmdError> { Ok(i32::from_le_bytes(bytes(o, 4)?.try_into().unwrap())) };
        let texto = |o: &mut usize| -> Result<String, SmdError> {
            let n = i32_(o)?;
            if !(0..=1 << 16).contains(&n) {
                return Err(SmdError::Contagem(n));
            }
            Ok(crate::ski::gbk(&bytes(o, n as usize)?))
        };
        let contagem = |o: &mut usize| -> Result<usize, SmdError> {
            let n = i32_(o)?;
            if !(0..=1 << 16).contains(&n) {
                return Err(SmdError::Contagem(n));
            }
            Ok(n as usize)
        };
        if i32_(&mut o)? as u32 != IDENTIDADE {
            return Err(SmdError::Identidade(u32::from_le_bytes(b[if marca { 4 } else { 0 }..][..4].try_into().unwrap())));
        }
        let versao = i32_(&mut o)? as u32;
        if versao > 8 {
            return Err(SmdError::Versao(versao));
        }
        let (n_peles, n_acoes, n_cabides, n_props) = (contagem(&mut o)?, contagem(&mut o)?, contagem(&mut o)?, contagem(&mut o)?);
        let reservado = bytes(&mut o, 60)?;
        let esqueleto = texto(&mut o)?;
        let peles = (0..n_peles).map(|_| texto(&mut o)).collect::<Result<_, _>>()?;
        let fisica = texto(&mut o)?;
        let pasta_das_trilhas = if versao >= 8 { Some(texto(&mut o)?) } else { None };
        let mut acoes = Vec::with_capacity(n_acoes);
        for _ in 0..n_acoes {
            let nome = texto(&mut o)?;
            let dados = if versao < 6 {
                let ad = bytes(&mut o, 12)?;
                let juntas = i32::from_le_bytes(ad[4..8].try_into().unwrap());
                if !(0..=4096).contains(&juntas) {
                    return Err(SmdError::Contagem(juntas));
                }
                [ad, bytes(&mut o, 20 * juntas as usize)?].concat()
            } else {
                bytes(&mut o, 8)?
            };
            let stck = if versao >= 7 { Some(texto(&mut o)?) } else { None };
            acoes.push(AcaoBruta { nome, dados, stck });
        }
        let mut cabides = Vec::with_capacity(n_cabides);
        for _ in 0..n_cabides {
            let tipo = i32_(&mut o)?;
            if !(0..=1).contains(&tipo) {
                return Err(SmdError::Contagem(tipo));
            }
            let nome = texto(&mut o)?;
            let preso_a = texto(&mut o)?;
            let dados: [u8; 8] = bytes(&mut o, 8)?.try_into().unwrap();
            let arquivo = texto(&mut o)?;
            let cc = if tipo == 1 { Some(texto(&mut o)?) } else { None };
            cabides.push(Cabide { tipo, nome, preso_a, dados, arquivo, cc });
        }
        let propriedades = (0..n_props).map(|_| Ok((texto(&mut o)?, texto(&mut o)?))).collect::<Result<_, SmdError>>()?;
        if o != b.len() {
            return Err(SmdError::Sobra(b.len() - o));
        }
        Ok(Smd { marca, versao, reservado, esqueleto, peles, fisica, pasta_das_trilhas, acoes, cabides, propriedades })
    }

    /// O inverso de `ler`, na ordem de `A3DSkinModel::Save` (`A3DSkinModel.cpp:302-416`).
    pub fn escrever(&self) -> Vec<u8> {
        let mut o = Vec::new();
        let i = |o: &mut Vec<u8>, x: i32| o.extend_from_slice(&x.to_le_bytes());
        let texto = |o: &mut Vec<u8>, t: &str| {
            let b = encoding_rs::GBK.encode(t).0;
            o.extend_from_slice(&(b.len() as i32).to_le_bytes());
            o.extend_from_slice(&b);
        };
        if self.marca {
            o.extend_from_slice(b"MOXB");
        }
        for x in [IDENTIDADE as i32, self.versao as i32, self.peles.len() as i32, self.acoes.len() as i32, self.cabides.len() as i32, self.propriedades.len() as i32] {
            i(&mut o, x);
        }
        let mut res = self.reservado.clone();
        res.resize(60, 0);
        o.extend_from_slice(&res);
        texto(&mut o, &self.esqueleto);
        self.peles.iter().for_each(|p| texto(&mut o, p));
        texto(&mut o, &self.fisica);
        if let Some(p) = &self.pasta_das_trilhas {
            texto(&mut o, p);
        }
        for a in &self.acoes {
            texto(&mut o, &a.nome);
            o.extend_from_slice(&a.dados);
            if let Some(s) = &a.stck {
                texto(&mut o, s);
            }
        }
        for c in &self.cabides {
            i(&mut o, c.tipo);
            texto(&mut o, &c.nome);
            texto(&mut o, &c.preso_a);
            o.extend_from_slice(&c.dados);
            texto(&mut o, &c.arquivo);
            if let Some(cc) = &c.cc {
                texto(&mut o, cc);
            }
        }
        for (n, v) in &self.propriedades {
            texto(&mut o, n);
            texto(&mut o, v);
        }
        o
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// B288: todo `.smd` do cliente fecha no último byte com o leitor completo (cabides e propriedades) e, reescrito,
    /// sai byte a byte igual; o que o `cabeca` lê bate com o completo.
    #[test]
    fn ler_inteiro_e_reescrever_o_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut lidos, mut iguais, mut cabides, mut props) = (0usize, 0usize, 0usize, 0usize);
        let mut por_versao = std::collections::BTreeMap::<u32, usize>::new();
        let mut falhas = std::collections::BTreeMap::<String, usize>::new();
        let mut diferentes: Vec<String> = Vec::new();
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("smd")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let s = match Smd::ler(&b) {
                    Ok(s) => s,
                    Err(e) => { *falhas.entry(format!("{e}").split(' ').take(3).collect::<Vec<_>>().join(" ")).or_default() += 1; continue }
                };
                lidos += 1;
                *por_versao.entry(s.versao).or_default() += 1;
                cabides += s.cabides.len();
                props += s.propriedades.len();
                let c = cabeca(&b).unwrap();
                assert_eq!((c.esqueleto.as_str(), c.peles.len(), c.lista_de_acoes.len()), (s.esqueleto.as_str(), s.peles.len(), s.acoes.len()), "{}", p.display());
                let w = s.escrever();
                if w == b {
                    iguais += 1;
                } else if diferentes.len() < 5 {
                    let o = w.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(w.len().min(b.len()));
                    diferentes.push(format!("{} (tam {} × {}, 1º byte diferente em {o})", p.display(), w.len(), b.len()));
                }
            }
        }
        eprintln!("smd: lidos {lidos} {por_versao:?} (cabides {cabides}, propriedades {props}), reescritos iguais {iguais}; falhas {falhas:?}; diferentes {diferentes:?}");
        assert!(lidos > 1000 && falhas.is_empty(), "{falhas:?}");
        assert_eq!(iguais, lidos, "{diferentes:?}");
    }

    /// Todos os `.smd` do cliente 1.5.5 BR: a cabeça lê, e o esqueleto e as peles citados existem na pasta (nomes do
    /// disco da instalação desempacotada: GBK lido em Windows-1252).
    #[test]
    fn todos_os_smd_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let disco = |s: &str| encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(s).0).0.into_owned();
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut ok, mut falha, mut com_esqueleto, mut esqueleto_existe, mut peles, mut peles_existem) = (0, 0, 0, 0, 0, 0);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    pilha.push(p);
                    continue;
                }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("smd")) {
                    continue;
                }
                match cabeca(&std::fs::read(&p).unwrap()) {
                    Ok(c) => {
                        ok += 1;
                        let pasta = p.parent().unwrap();
                        if !c.esqueleto.is_empty() {
                            com_esqueleto += 1;
                            if pasta.join(disco(&c.esqueleto)).exists() {
                                esqueleto_existe += 1;
                            }
                        }
                        for s in &c.peles {
                            peles += 1;
                            if pasta.join(disco(s)).exists() {
                                peles_existem += 1;
                            }
                        }
                    }
                    Err(_) => falha += 1,
                }
            }
        }
        assert_eq!(falha, 0, "B276: com as ações, todo .smd lê");
        eprintln!("smd: {ok} leem, {falha} falham; com esqueleto {com_esqueleto} (existe {esqueleto_existe}); peles {peles} (existem {peles_existem})");
        assert!(ok > 0);
    }
}
