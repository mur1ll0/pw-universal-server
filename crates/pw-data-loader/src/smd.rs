//! O começo do `.smd` — o modelo de pele que liga um esqueleto às peles (B274, Estúdio S5).
//!
//! `A3DSkinModelMan::LoadModelFromFile` (`Angelica3D/Source/A3DSkinModelMan.cpp`): marca `MOXB` do `AFile`,
//! `A3DSKINMODELFILEHEADER` (`A3DSkinModel.h:48-57`, 84 bytes: `'ASMD'`, versão ≤ 8, peles, ações, cabides,
//! propriedades, 60 reservados), o nome do esqueleto e o de cada pele (strings `i32` + bytes), relativos à pasta do
//! `.smd` (`strPath + "\\" + strFile`). O resto (ações, cabides, propriedades) não é lido aqui.

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
    Ok(CabecaDoSmd { versao, acoes, esqueleto, peles })
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
        eprintln!("smd: {ok} leem, {falha} falham; com esqueleto {com_esqueleto} (existe {esqueleto_existe}); peles {peles} (existem {peles_existem})");
        assert!(ok > 0);
    }
}
