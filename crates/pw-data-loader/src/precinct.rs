//! `precinct.sev` — os distritos de cada mapa e o ponto de cidade de cada um.
//!
//! # Para que serve
//!
//! Renascer na cidade leva ao **ponto de cidade do distrito** onde o jogador morreu:
//! `gplayer_controller::ResurrectInTown` (`cgame/gs/playercmd.cpp:112-129`) chama
//! `world_manager::GetTownPosition`, que no 1.5.5 vai a
//! `city_region::GetCityPos` (`playertemplate.h:358-362` — a versão que lia o
//! `[TOWN_REGION]` do `ptemplate.conf` está comentada) e daí a
//! `CELPrecinctSet::IsPointIn(x, z, tag do mapa)` (`template/city_region.cpp:21-37`).
//! Sem distrito, o jogador renasce onde está.
//!
//! # Formato (`template/el_precinct.h/.cpp`, ramo servidor)
//!
//! `PRECINCTFILEHEADER5` (`dwVersion`, `iNumPrecinct`, `dwTimeStamp`; versão < 5 sem o
//! carimbo) e, por distrito (`CELPrecinct::Load(FILE*)`, `el_precinct.cpp:219-266`):
//!
//! | campo | tipo | versão |
//! | :--- | :--- | :--- |
//! | `iNumPoint` | `int` | todas |
//! | `m_iPriority` | `int` | todas |
//! | `m_idDstInst` (mapa do ponto de cidade) | `int` | todas |
//! | `m_idSrcInst` (mapa do distrito) | `int` | ≥ 4 (antes, 1) |
//! | `m_idDomain` | `int` | ≥ 6 |
//! | `m_bPKProtect` | `bool` | ≥ 7 |
//! | ponto de cidade | `float[3]` | todas |
//! | vértices | `iNumPoint × float[3]` | todas |
//!
//! O leitor recusa arquivo que não termina no último byte.

use thiserror::Error;

/// `ELPCTFILE_VERSION`.
pub const VERSAO_MAXIMA: u32 = 7;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrecinctError {
    #[error("precinct.sev truncado no byte {0}")]
    Truncado(usize),
    #[error("precinct.sev versão {0} maior que a suportada ({VERSAO_MAXIMA})")]
    Versao(u32),
    #[error("precinct.sev terminou no byte {lido} e tem {tamanho}")]
    Sobra { lido: usize, tamanho: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Distrito {
    pub prioridade: i32,
    pub mapa_do_ponto: i32,
    pub mapa: i32,
    pub dominio: i32,
    pub protege_pk: bool,
    pub ponto_de_cidade: [f32; 3],
    pub vertices: Vec<[f32; 3]>,
    esquerda: f32,
    topo: f32,
    direita: f32,
    base: f32,
}

impl Distrito {
    /// `CELPrecinct::IsPointIn` (`el_precinct.cpp:290-306`): caixa, depois paridade dos
    /// cruzamentos de um raio para a direita.
    pub fn contem(&self, x: f32, z: f32) -> bool {
        if x < self.esquerda || x > self.direita || z < self.topo || z > self.base {
            return false;
        }
        let cruzamentos = (0..self.vertices.len()).filter(|&i| self.cruza(x, z, i)).count();
        cruzamentos & 1 == 1
    }

    /// `CELPrecinct::IsCrossLine` (`el_precinct.cpp:309-357`), linha a linha.
    fn cruza(&self, x: f32, z: f32, i: usize) -> bool {
        let n = self.vertices.len();
        let v1 = self.vertices[i];
        let v2 = self.vertices[(i + 1) % n];
        if v1[0] < x && v2[0] < x {
            return false;
        }
        if v1[2] < z && v2[2] < z {
            return false;
        }
        if v1[2] > z && v2[2] > z {
            return false;
        }
        if v1[2] == v2[2] {
            return false;
        }
        if z == v1[2] {
            let anterior = (1..n)
                .map(|k| self.vertices[(i + n - k) % n])
                .find(|p| p[2] != z);
            let Some(pre) = anterior else { return false };
            if (pre[2] < z && v2[2] > z) || (pre[2] > z && v2[2] < z) {
                return false;
            }
        }
        let inclinacao = (v2[0] - v1[0]) / (v2[2] - v1[2]);
        let x_cruzamento = (z - v1[2]) * inclinacao + v1[0];
        x_cruzamento > x
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Distritos {
    pub versao: u32,
    pub carimbo: u32,
    pub lista: Vec<Distrito>,
}

impl Distritos {
    pub fn ler(dados: &[u8]) -> Result<Self, PrecinctError> {
        let mut o = 0usize;
        let u32_em = |o: &mut usize| -> Result<u32, PrecinctError> {
            let b = dados.get(*o..*o + 4).ok_or(PrecinctError::Truncado(*o))?;
            *o += 4;
            Ok(u32::from_le_bytes(b.try_into().unwrap()))
        };
        let versao = u32_em(&mut o)?;
        if versao > VERSAO_MAXIMA {
            return Err(PrecinctError::Versao(versao));
        }
        let quantos = u32_em(&mut o)? as i32;
        let carimbo = if versao >= 5 { u32_em(&mut o)? } else { 0 };

        let mut lista = Vec::with_capacity(quantos.max(0) as usize);
        for _ in 0..quantos.max(0) {
            let pontos = u32_em(&mut o)? as i32;
            let prioridade = u32_em(&mut o)? as i32;
            let mapa_do_ponto = u32_em(&mut o)? as i32;
            let mapa = if versao >= 4 { u32_em(&mut o)? as i32 } else { 1 };
            let dominio = if versao >= 6 { u32_em(&mut o)? as i32 } else { 0 };
            let protege_pk = if versao >= 7 {
                let b = *dados.get(o).ok_or(PrecinctError::Truncado(o))?;
                o += 1;
                b != 0
            } else {
                false
            };
            let vetor = |o: &mut usize| -> Result<[f32; 3], PrecinctError> {
                let mut v = [0f32; 3];
                for c in &mut v {
                    *c = f32::from_bits(u32_em(o)?);
                }
                Ok(v)
            };
            let ponto_de_cidade = vetor(&mut o)?;
            let vertices = (0..pontos.max(0)).map(|_| vetor(&mut o)).collect::<Result<Vec<_>, _>>()?;

            let (mut esquerda, mut topo, mut direita, mut base) = (999999.0f32, 999999.0f32, -999999.0f32, -999999.0f32);
            for v in &vertices {
                esquerda = esquerda.min(v[0]);
                direita = direita.max(v[0]);
                topo = topo.min(v[2]);
                base = base.max(v[2]);
            }
            lista.push(Distrito {
                prioridade,
                mapa_do_ponto,
                mapa,
                dominio,
                protege_pk,
                ponto_de_cidade,
                vertices,
                esquerda,
                topo,
                direita,
                base,
            });
        }
        if o != dados.len() {
            return Err(PrecinctError::Sobra { lido: o, tamanho: dados.len() });
        }
        Ok(Self { versao, carimbo, lista })
    }

    /// `CELPrecinctSet::IsPointIn(x, z, idSrcInst)` (`el_precinct.cpp:379-394`): entre os
    /// distritos do mapa que contêm o ponto, o de **menor** prioridade; empate fica com o
    /// último da lista (`<=`).
    pub fn distrito_em(&self, x: f32, z: f32, mapa: i32) -> Option<&Distrito> {
        let mut escolhido: Option<&Distrito> = None;
        for d in &self.lista {
            if d.mapa != mapa || !d.contem(x, z) {
                continue;
            }
            if escolhido.map_or(true, |e| d.prioridade <= e.prioridade) {
                escolhido = Some(d);
            }
        }
        escolhido
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadrado(prioridade: i32, ponto: [f32; 3]) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [4i32, prioridade, 1, 1, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.push(0);
        for c in ponto {
            b.extend_from_slice(&c.to_le_bytes());
        }
        for (x, z) in [(0.0f32, 0.0f32), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
            for c in [x, 0.0, z] {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b
    }

    fn arquivo(distritos: &[Vec<u8>]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&7u32.to_le_bytes());
        b.extend_from_slice(&(distritos.len() as i32).to_le_bytes());
        b.extend_from_slice(&123u32.to_le_bytes());
        for d in distritos {
            b.extend_from_slice(d);
        }
        b
    }

    #[test]
    fn o_ponto_dentro_do_poligono_escolhe_a_menor_prioridade() {
        let d = Distritos::ler(&arquivo(&[quadrado(4, [1.0, 2.0, 3.0]), quadrado(0, [7.0, 8.0, 9.0])])).unwrap();
        assert_eq!(d.carimbo, 123);
        let e = d.distrito_em(5.0, 5.0, 1).unwrap();
        assert_eq!(e.ponto_de_cidade, [7.0, 8.0, 9.0]);
        assert!(d.distrito_em(15.0, 5.0, 1).is_none());
        assert!(d.distrito_em(5.0, 5.0, 161).is_none(), "distrito de outro mapa");
    }

    #[test]
    fn recusa_byte_a_mais() {
        let mut b = arquivo(&[quadrado(0, [0.0; 3])]);
        b.push(0);
        assert!(matches!(Distritos::ler(&b), Err(PrecinctError::Sobra { .. })));
    }
}
