//! As rotas de patrulha do mapa: `path.sev`, lido como o `path_manager` do original
//! (`gs/template/pathman.cpp`, `gs/template/sevbezier.cpp`).
//!
//! O arquivo guarda curvas Bézier; o servidor não anda na curva, anda numa lista de pontos que
//! o `path_manager::Init` tira dela **uma vez, na carga**: um `CSevBezierWalker` a 8 m/s,
//! amostrado a cada 1000 ms, empurrando a posição antes de cada tique e parando quando o
//! caminhante acaba ou fica parado (`|p2 − p| < 1e-3`) — o ponto final da curva não entra.
//! Curvas encadeadas pelo `iNextGlobalID` viram uma rota só.
//!
//! | parte | formato |
//! | :--- | :--- |
//! | cabeçalho | `int iVersion` (recusa `> 1`), `int iNumBezier` (`SEVBEZIERFILEHEADER`) |
//! | curva | `u32 dwVersion`, `int id`; com `dwVersion ≥ 2`, `int global`, `int próximo global`; `int n` pontos × (`pos` 12 + `dir` 12); `int m` segmentos × (`âncora início` 12, `âncora fim` 12, `int início`, `int fim`, `float comprimento`) |
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

/// Uma curva do `path.sev` (`CSevBezier`).
#[derive(Debug, Clone)]
struct Curva {
    id: i32,
    global: i32,
    proximo_global: i32,
    pontos: Vec<[f32; 3]>,
    segmentos: Vec<Segmento>,
}

#[derive(Debug, Clone, Copy)]
struct Segmento {
    ancora_inicio: [f32; 3],
    ancora_fim: [f32; 3],
    inicio: usize,
    fim: usize,
    comprimento: f32,
}

impl Segmento {
    /// `CSevBezierSeg::Bezier(u, true)` (`sevbezier.cpp:97-133`).
    fn bezier(&self, pontos: &[[f32; 3]], u: f32) -> [f32; 3] {
        let (p1, c1, c2, p2) = (pontos[self.inicio], self.ancora_inicio, self.ancora_fim, pontos[self.fim]);
        let mut r = [0.0f32; 3];
        for i in 0..3 {
            let c = 3.0 * (c1[i] - p1[i]);
            let b = 3.0 * (c2[i] - c1[i]) - c;
            let a = p2[i] - p1[i] - c - b;
            r[i] = a * u * u * u + b * u * u + c * u + p1[i];
        }
        r
    }
}

/// As rotas de um mapa: pelo id da curva (`single_path::id`, o `iPathID` do `npcgen.data`),
/// e a tabela do id global para o id (`IdConvert`, usada pelos gatilhos do `aipolicy`).
#[derive(Debug, Clone, Default)]
pub struct Rotas {
    pub pontos: HashMap<i32, Vec<[f32; 3]>>,
    pub do_global: HashMap<i32, i32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErroDeRota {
    #[error("path.sev: {0}")]
    Io(#[from] std::io::Error),
    #[error("path.sev: versão {0} maior que 1")]
    Versao(i32),
    #[error("path.sev: {0} bytes sobrando depois da última curva")]
    Sobra(usize),
    #[error("path.sev: curva {0} com índice de ponto fora da lista")]
    Indice(i32),
    #[error("path.sev: id de rota {0} repetido")]
    Repetido(i32),
}

fn i32_(c: &mut Cursor<&[u8]>) -> std::io::Result<i32> {
    let mut b = [0u8; 4];
    c.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}
fn f32_(c: &mut Cursor<&[u8]>) -> std::io::Result<f32> {
    let mut b = [0u8; 4];
    c.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}
fn v3(c: &mut Cursor<&[u8]>) -> std::io::Result<[f32; 3]> {
    Ok([f32_(c)?, f32_(c)?, f32_(c)?])
}

impl Rotas {
    /// `path_manager::Init` sobre os bytes de um `path.sev`. Arquivo vazio (0 bytes, como o
    /// `b01` do 1.5.5) é mapa sem rota.
    pub fn ler(bytes: &[u8]) -> Result<Self, ErroDeRota> {
        if bytes.is_empty() {
            return Ok(Self::default());
        }
        let mut c = Cursor::new(bytes);
        let versao = i32_(&mut c)?;
        if versao > 1 {
            return Err(ErroDeRota::Versao(versao));
        }
        let n = i32_(&mut c)?.max(0);
        let mut curvas = Vec::with_capacity(n as usize);
        for _ in 0..n {
            // `CSevBezier::Load` (`sevbezier.cpp:215-285`).
            let v = i32_(&mut c)? as u32;
            let id = i32_(&mut c)?;
            let (global, proximo_global) = if v >= 2 { (i32_(&mut c)?, i32_(&mut c)?) } else { (-1, -1) };
            let np = i32_(&mut c)?.max(0) as usize;
            let mut pontos = Vec::with_capacity(np);
            for _ in 0..np {
                pontos.push(v3(&mut c)?);
                let _dir = v3(&mut c)?;
            }
            let ns = i32_(&mut c)?.max(0) as usize;
            let mut segmentos = Vec::with_capacity(ns);
            for _ in 0..ns {
                let ancora_inicio = v3(&mut c)?;
                let ancora_fim = v3(&mut c)?;
                let (inicio, fim) = (i32_(&mut c)?, i32_(&mut c)?);
                let comprimento = f32_(&mut c)?;
                if inicio < 0 || fim < 0 || inicio as usize >= np || fim as usize >= np {
                    return Err(ErroDeRota::Indice(id));
                }
                segmentos.push(Segmento { ancora_inicio, ancora_fim, inicio: inicio as usize, fim: fim as usize, comprimento });
            }
            curvas.push(Curva { id, global, proximo_global, pontos, segmentos });
        }
        let sobra = bytes.len() - c.position() as usize;
        if sobra != 0 {
            return Err(ErroDeRota::Sobra(sobra));
        }

        let mut rotas = Self::default();
        for curva in &curvas {
            let mut pontos = Vec::new();
            let mut atual = Some(curva);
            let mut vistos = HashSet::new();
            while let Some(cv) = atual {
                // O original não se protege de encadeamento circular (travaria a carga).
                if !vistos.insert(cv.id) {
                    break;
                }
                amostrar(cv, &mut pontos);
                if cv.proximo_global < 0 {
                    break;
                }
                atual = curvas.iter().find(|o| o.global == cv.proximo_global);
            }
            if rotas.pontos.insert(curva.id, pontos).is_some() {
                return Err(ErroDeRota::Repetido(curva.id));
            }
            if curva.global > 0 {
                rotas.do_global.insert(curva.global, curva.id);
            }
        }
        Ok(rotas)
    }

    /// `path_manager::GetPath`: os pontos da rota, ou `None`.
    pub fn rota(&self, id: i32) -> Option<&[[f32; 3]]> {
        self.pontos.get(&id).map(|v| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.pontos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pontos.is_empty()
    }
}

/// O `CSevBezierWalker` do `path_manager::Init`: `SetSpeed(8)`, `StartWalk(false, true)`,
/// empurra `GetPos()`, `Tick(1000)`, e para quando o caminhante acaba ou não saiu do lugar
/// (`pathman.cpp:20-31`; `sevbezier.cpp:374-565`). Os tempos são `int` em ms, como lá.
fn amostrar(cv: &Curva, saida: &mut Vec<[f32; 3]>) {
    const VELOCIDADE: f32 = 8.0;
    let n = cv.segmentos.len();
    if n == 0 || cv.pontos.is_empty() {
        return;
    }
    let inv = 1000.0 / VELOCIDADE;
    let tempo_de = |i: usize| (cv.segmentos[i].comprimento * inv) as i32;
    let total: i32 = (0..n).map(tempo_de).sum();
    let (mut cont, mut seg, mut passado) = (0i32, 0usize, 0i32);
    let mut tempo_seg = tempo_de(0);
    let mut andando = true;
    let pos = |andando: bool, cont: i32, passado: i32, seg: usize, tempo_seg: i32| -> [f32; 3] {
        if andando {
            // `(m_iTimeCnt - m_iPassSegTime) / m_iCurSegTime`; um segmento de tempo 0 daria
            // divisão por zero no original.
            let f = if tempo_seg > 0 { (cont - passado) as f32 / tempo_seg as f32 } else { 0.0 };
            cv.segmentos[seg].bezier(&cv.pontos, f)
        } else {
            cv.segmentos[n - 1].bezier(&cv.pontos, 1.0)
        }
    };
    // Sem tempo nenhum o `Tick` faria `% 0`; a curva fica só com o primeiro ponto.
    if total <= 0 {
        saida.push(pos(true, 0, 0, 0, tempo_seg));
        return;
    }
    // Teto de segurança: uma rota de 8 m/s não passa disto em mapa nenhum.
    for _ in 0..1_000_000 {
        let p = pos(andando, cont, passado, seg, tempo_seg);
        saida.push(p);
        // `Tick(1000)`, sem laço.
        if andando {
            cont += 1000 % total;
            while cont >= passado + tempo_seg {
                if seg + 1 >= n {
                    cont = passado + tempo_seg;
                    andando = false;
                    break;
                }
                seg += 1;
                passado += tempo_seg;
                tempo_seg = tempo_de(seg);
            }
        }
        let p2 = pos(andando, cont, passado, seg, tempo_seg);
        let d = ((p2[0] - p[0]).powi(2) + (p2[1] - p[1]).powi(2) + (p2[2] - p[2]).powi(2)).sqrt();
        if !andando || d < 1e-3 {
            break;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn curva(comprimento: f32) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [1i32, 1] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&1u32.to_le_bytes()); // dwVersion
        b.extend_from_slice(&77i32.to_le_bytes()); // id
        b.extend_from_slice(&2i32.to_le_bytes());
        for p in [[0.0f32, 0.0, 0.0], [0.0, 0.0, 0.0], [comprimento, 0.0, 0.0], [0.0, 0.0, 0.0]] {
            for x in p {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        b.extend_from_slice(&1i32.to_le_bytes());
        // Âncoras a 1/3 e 2/3: a curva é a reta, percorrida em velocidade constante.
        for x in [comprimento / 3.0, 0.0, 0.0, 2.0 * comprimento / 3.0, 0.0, 0.0] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b.extend_from_slice(&0i32.to_le_bytes());
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&comprimento.to_le_bytes());
        b
    }

    /// Uma reta de 40 m a 8 m/s: 5 s, pontos a cada 8 m, sem o ponto final.
    #[test]
    fn a_reta_de_40_m_vira_pontos_a_cada_8_m() {
        let r = Rotas::ler(&curva(40.0)).unwrap();
        let p = r.rota(77).unwrap();
        let xs: Vec<i32> = p.iter().map(|v| v[0].round() as i32).collect();
        assert_eq!(xs, vec![0, 8, 16, 24, 32]);
    }

    #[test]
    fn byte_sobrando_e_recusado() {
        let mut b = curva(40.0);
        b.push(0);
        assert!(matches!(Rotas::ler(&b), Err(ErroDeRota::Sobra(1))));
    }
}
