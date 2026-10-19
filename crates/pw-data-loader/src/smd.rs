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

#[cfg(test)]
mod testes {
    use super::*;

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
