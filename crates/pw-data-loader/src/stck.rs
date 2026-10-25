//! `.stck` — as trilhas de uma ação (B276, Estúdio S5), para esqueleto v6.
//!
//! `A3DSklTrackSet::Load` (`Angelica3D/Source/A3DTrackMan.cpp:150-200`): marca `MOXB` do `AFile`, `'STCK'` versão 1,
//! `A3DSKLTRACKFILEHEADER` (`:46-54`, 16 bytes: trilhas, quadro inicial, final, FPS) e, por trilha, o `id` (`i32`) e a
//! trilha do osso (`crate::trilha::ler_osso`). O `id` é o índice da junta: cada osso usa a trilha da sua primeira junta
//! (`A3DSkinModelAct.cpp:612`, `pBone->GetFirstJoint()`).

#[derive(Debug, thiserror::Error)]
pub enum StckError {
    #[error("não é .stck (identidade {0:#x})")]
    Identidade(u32),
    #[error("versão {0} (só a 1)")]
    Versao(u32),
    #[error("arquivo curto ou trilha {0} ilegível")]
    Trilha(usize),
    #[error("sobraram {0} bytes")]
    Sobra(usize),
}

const IDENTIDADE: u32 = (b'S' as u32) << 24 | (b'T' as u32) << 16 | (b'C' as u32) << 8 | b'K' as u32;

#[derive(Debug, Clone)]
pub struct Stck {
    pub quadro_inicial: i32,
    pub quadro_final: i32,
    pub fps: i32,
    /// `(id da junta, trilha)`.
    pub trilhas: Vec<(i32, crate::trilha::TrilhaDeOsso)>,
    /// B287 (escrita): a marca `MOXB` do `AFile`.
    pub marca: bool,
}

/// B287 (S6): o inverso de `ler` — `A3DSklTrackSet::Save` (`A3DTrackMan.cpp:304-…`): identidade, versão 1, cabeçalho
/// (trilhas, quadros, FPS) e, por trilha, o id da junta, a posição e a rotação.
pub fn escrever(s: &Stck) -> Vec<u8> {
    let mut o = Vec::new();
    if s.marca {
        o.extend_from_slice(b"MOXB");
    }
    for x in [IDENTIDADE as i32, 1, s.trilhas.len() as i32, s.quadro_inicial, s.quadro_final, s.fps] {
        o.extend_from_slice(&x.to_le_bytes());
    }
    for (id, t) in &s.trilhas {
        o.extend_from_slice(&id.to_le_bytes());
        crate::trilha::escrever_osso(t, &mut o);
    }
    o
}

pub fn ler(b: &[u8]) -> Result<Stck, StckError> {
    let mut o = if b.starts_with(b"MOXB") { 4 } else { 0 };
    let u = |o: &mut usize| -> Result<i32, StckError> {
        let v = b.get(*o..*o + 4).ok_or(StckError::Trilha(0))?;
        *o += 4;
        Ok(i32::from_le_bytes(v.try_into().unwrap()))
    };
    let id = u(&mut o)? as u32;
    if id != IDENTIDADE {
        return Err(StckError::Identidade(id));
    }
    let versao = u(&mut o)? as u32;
    if versao != 1 {
        return Err(StckError::Versao(versao));
    }
    let n = u(&mut o)?;
    let quadro_inicial = u(&mut o)?;
    let quadro_final = u(&mut o)?;
    let fps = u(&mut o)?;
    let mut trilhas = Vec::with_capacity(n.max(0) as usize);
    for i in 0..n.max(0) as usize {
        let junta = u(&mut o)?;
        trilhas.push((junta, crate::trilha::ler_osso(b, &mut o).ok_or(StckError::Trilha(i))?));
    }
    if o != b.len() {
        return Err(StckError::Sobra(b.len() - o));
    }
    Ok(Stck { quadro_inicial, quadro_final, fps, trilhas, marca: b.starts_with(b"MOXB") })
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Todos os `.stck` do cliente 1.5.5 BR fecham no último byte e (B287) se reescrevem iguais.
    #[test]
    fn todos_os_stck_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut ok, mut falhas, mut trilhas) = (0usize, std::collections::BTreeMap::<String, usize>::new(), 0usize);
        let mut iguais = 0usize;
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    pilha.push(p);
                    continue;
                }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("stck")) {
                    continue;
                }
                let b = std::fs::read(&p).unwrap();
                match ler(&b) {
                    Ok(s) => {
                        ok += 1;
                        trilhas += s.trilhas.len();
                        // B287: reescrito, sai byte a byte igual.
                        iguais += (escrever(&s) == b) as usize;
                    }
                    Err(e) => *falhas.entry(format!("{e}").split(' ').take(3).collect::<Vec<_>>().join(" ")).or_default() += 1,
                }
            }
        }
        eprintln!("stck: {ok} fecham ({trilhas} trilhas), {iguais} reescritos iguais; falhas {falhas:?}");
        assert!(ok > 0 && falhas.is_empty());
        assert_eq!(iguais, ok);
    }
}
