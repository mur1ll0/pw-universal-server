//! `region.sev` por mapa: as **caixas de transporte** (B208) — os portais de região que levam a
//! outro mapa (a saída e a entrada das masmorras).
//!
//! Formato binário do lado servidor (`cgame/gs/template/el_region.cpp`, `CELRegionSet::Load` e
//! `CELTransportBox::Load(FILE*)`, `:293-330`; o mesmo do cliente, `CCommon/EL_Region.cpp`):
//! `REGIONFILEHEADER4 { u32 dwVersion; i32 iNumRegion; i32 iNumTrans; u32 dwTimeStamp }` e, até
//! completar as duas contagens, registros `i32 tipo` seguidos de
//! - tipo 0, região: `i32 iNumPoint` + `iNumPoint × f32[3]` (`CELRegion::Load`, `:111-125`);
//! - tipo 1, caixa: `i32 idInst` (destino), `i32 idSrcInst` (v ≥ 3, senão 1), `i32 iLevelLmt`
//!   (v ≥ 5, senão 1), `f32[3] pos`, `f32[3] exts`, `f32[3] alvo`.
//!
//! O índice da caixa é a ordem de leitura (`SetIndex(m_aTransBoxes.size())`) — é ele que o cliente
//! manda no `ENTER_INSTANCE` (`EC_World.cpp:2360-2373`). O arquivo tem de fechar no último byte.

use std::io::{Cursor, Read};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaixaDeTransporte {
    /// `m_idInst`: o mapa de destino.
    pub destino: i32,
    /// `m_idSrcInst`: o mapa onde a caixa vale (`GetRegionTransport` confere com o tag do mundo).
    pub origem: i32,
    /// `m_iLevelLmt` (o servidor original não confere).
    pub nivel_minimo: i32,
    pub pos: [f32; 3],
    /// Meia-medida: `IsPointIn` aceita `pos ± exts` em cada eixo (`el_region.h:184-192`).
    pub exts: [f32; 3],
    /// `m_vTarget`: onde o jogador chega no destino.
    pub alvo: [f32; 3],
}

impl CaixaDeTransporte {
    /// `CELTransportBox::IsPointIn` (`el_region.h:184-192`).
    pub fn contem(&self, x: f32, y: f32, z: f32) -> bool {
        let dentro = |v: f32, c: f32, e: f32| v >= c - e && v <= c + e;
        dentro(x, self.pos[0], self.exts[0]) && dentro(y, self.pos[1], self.exts[1]) && dentro(z, self.pos[2], self.exts[2])
    }
}

fn erro(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg.to_string())
}

/// As caixas de transporte de um `region.sev`, na ordem do índice. Recusa versão < 4 (sem o
/// arquivo real para conferir) e byte que sobre ou falte.
pub fn ler_caixas_de_transporte(data: &[u8]) -> std::io::Result<Vec<CaixaDeTransporte>> {
    let mut c = Cursor::new(data);
    let u32_ = |c: &mut Cursor<&[u8]>| -> std::io::Result<u32> {
        let mut b = [0u8; 4];
        c.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    };
    let versao = u32_(&mut c)?;
    if versao < 4 {
        return Err(erro("region.sev versão < 4 não implementada"));
    }
    let n_regioes = u32_(&mut c)? as i32;
    let n_caixas = u32_(&mut c)? as i32;
    let _carimbo = u32_(&mut c)?;
    let f = |c: &mut Cursor<&[u8]>| -> std::io::Result<f32> {
        let mut b = [0u8; 4];
        c.read_exact(&mut b)?;
        Ok(f32::from_le_bytes(b))
    };
    let v3 = |c: &mut Cursor<&[u8]>| -> std::io::Result<[f32; 3]> { Ok([f(c)?, f(c)?, f(c)?]) };
    let (mut regioes, mut caixas) = (0, Vec::new());
    while regioes < n_regioes || (caixas.len() as i32) < n_caixas {
        match u32_(&mut c)? {
            0 => {
                let pontos = u32_(&mut c)? as usize;
                if pontos > 100_000 {
                    return Err(erro("região com pontos demais"));
                }
                for _ in 0..pontos {
                    v3(&mut c)?;
                }
                regioes += 1;
            }
            1 => {
                let destino = u32_(&mut c)? as i32;
                let origem = if versao >= 3 { u32_(&mut c)? as i32 } else { 1 };
                let nivel_minimo = if versao >= 5 { u32_(&mut c)? as i32 } else { 1 };
                let (pos, exts, alvo) = (v3(&mut c)?, v3(&mut c)?, v3(&mut c)?);
                caixas.push(CaixaDeTransporte { destino, origem, nivel_minimo, pos, exts, alvo });
            }
            _ => return Err(erro("region.sev com registro de tipo desconhecido")),
        }
    }
    if c.position() as usize != data.len() {
        return Err(erro("region.sev não fecha no último byte"));
    }
    Ok(caixas)
}

/// Um registro do `region.sev` na ordem do arquivo (B244, Estúdio): região (`i32 0`) ou caixa (`i32 1`).
#[derive(Debug, Clone, PartialEq)]
pub enum RegistroDeRegiao {
    Regiao(Vec<[f32; 3]>),
    Caixa(CaixaDeTransporte),
}

/// O `region.sev` inteiro, para editar e gravar de volta byte a byte.
#[derive(Debug, Clone, PartialEq)]
pub struct RegioesEditavel {
    pub versao: u32,
    pub carimbo: u32,
    pub registros: Vec<RegistroDeRegiao>,
}

impl RegioesEditavel {
    /// Mesmo formato de `ler_caixas_de_transporte`, guardando as regiões e a ordem dos registros.
    pub fn ler(data: &[u8]) -> std::io::Result<Self> {
        let mut c = Cursor::new(data);
        let u32_ = |c: &mut Cursor<&[u8]>| -> std::io::Result<u32> {
            let mut b = [0u8; 4];
            c.read_exact(&mut b)?;
            Ok(u32::from_le_bytes(b))
        };
        let versao = u32_(&mut c)?;
        if versao < 4 {
            return Err(erro("region.sev versão < 4 não implementada"));
        }
        let n_regioes = u32_(&mut c)? as i32;
        let n_caixas = u32_(&mut c)? as i32;
        let carimbo = u32_(&mut c)?;
        let v3 = |c: &mut Cursor<&[u8]>| -> std::io::Result<[f32; 3]> {
            Ok([f32::from_bits(u32_(c)?), f32::from_bits(u32_(c)?), f32::from_bits(u32_(c)?)])
        };
        let (mut regioes, mut caixas, mut registros) = (0, 0, Vec::new());
        while regioes < n_regioes || caixas < n_caixas {
            match u32_(&mut c)? {
                0 => {
                    let pontos = u32_(&mut c)? as usize;
                    if pontos > 100_000 {
                        return Err(erro("região com pontos demais"));
                    }
                    registros.push(RegistroDeRegiao::Regiao((0..pontos).map(|_| v3(&mut c)).collect::<Result<_, _>>()?));
                    regioes += 1;
                }
                1 => {
                    let destino = u32_(&mut c)? as i32;
                    let origem = u32_(&mut c)? as i32; // v ≥ 3 (só se lê v ≥ 4)
                    let nivel_minimo = if versao >= 5 { u32_(&mut c)? as i32 } else { 1 };
                    let (pos, exts, alvo) = (v3(&mut c)?, v3(&mut c)?, v3(&mut c)?);
                    registros.push(RegistroDeRegiao::Caixa(CaixaDeTransporte { destino, origem, nivel_minimo, pos, exts, alvo }));
                    caixas += 1;
                }
                _ => return Err(erro("region.sev com registro de tipo desconhecido")),
            }
        }
        if c.position() as usize != data.len() {
            return Err(erro("region.sev não fecha no último byte"));
        }
        Ok(Self { versao, carimbo, registros })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let mut b = Vec::new();
        let u = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
        let v3 = |b: &mut Vec<u8>, v: &[f32; 3]| v.iter().for_each(|x| b.extend_from_slice(&x.to_bits().to_le_bytes()));
        let n_reg = self.registros.iter().filter(|r| matches!(r, RegistroDeRegiao::Regiao(_))).count();
        u(&mut b, self.versao);
        u(&mut b, n_reg as u32);
        u(&mut b, (self.registros.len() - n_reg) as u32);
        u(&mut b, self.carimbo);
        for r in &self.registros {
            match r {
                RegistroDeRegiao::Regiao(p) => {
                    u(&mut b, 0);
                    u(&mut b, p.len() as u32);
                    p.iter().for_each(|v| v3(&mut b, v));
                }
                RegistroDeRegiao::Caixa(k) => {
                    u(&mut b, 1);
                    u(&mut b, k.destino as u32);
                    u(&mut b, k.origem as u32);
                    if self.versao >= 5 {
                        u(&mut b, k.nivel_minimo as u32);
                    }
                    v3(&mut b, &k.pos);
                    v3(&mut b, &k.exts);
                    v3(&mut b, &k.alvo);
                }
            }
        }
        b
    }

    /// As caixas na ordem do índice que o cliente manda no `ENTER_INSTANCE`.
    pub fn caixas_mut(&mut self) -> Vec<&mut CaixaDeTransporte> {
        self.registros.iter_mut().filter_map(|r| if let RegistroDeRegiao::Caixa(k) = r { Some(k) } else { None }).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// B208: a Caverna das Sombras (a69 do `realm_155`) — a caixa 0 é o altar do começo do mapa e
    /// leva ao mapa 161 em (856, 58, 364); a 1 é um transporte dentro da masmorra.
    #[test]
    fn a_saida_da_caverna_das_sombras_leva_ao_161() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config/a69/region.sev");
        let Ok(b) = std::fs::read(&p) else {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", p.display());
            return;
        };
        let caixas = ler_caixas_de_transporte(&b).expect("o arquivo fecha no último byte");
        assert_eq!(caixas.len(), 2);
        let saida = caixas[0];
        assert_eq!((saida.destino, saida.origem), (161, 169));
        assert_eq!(saida.alvo.map(|v| v.round() as i32), [856, 58, 364]);
        assert!(saida.contem(-388.0, 34.0, 173.0) && !saida.contem(-370.0, 34.0, 173.0));
        assert_eq!((caixas[1].destino, caixas[1].origem), (169, 169));
        let mut sobra = b.clone();
        sobra.push(0);
        assert!(ler_caixas_de_transporte(&sobra).is_err(), "byte a mais é recusado");
    }
}
