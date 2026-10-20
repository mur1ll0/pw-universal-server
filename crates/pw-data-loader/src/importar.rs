//! Importar um glTF para a pele do Angelica (`.ski`) — B289, Estúdio S6.
//!
//! O caminho inverso de `glb::do_modelo` (B274):
//! - **Eixos:** o glTF é de mão direita e o Direct3D de mão esquerda. Nega-se o `z` de posições e normais e inverte-se
//!   a ordem dos triângulos (`glb::espelhar`, `Montagem::indices`).
//! - **Pele:** cada primitiva com `JOINTS_0`/`WEIGHTS_0` vira uma malha de pele (`A3DBVERTEX3`, `A3DVertex.h:184-191`:
//!   3 pesos e 4 índices de osso; o 4º peso é `1 − soma`). Os pesos são normalizados. Influências além das 4 do
//!   `JOINTS_0` não existem no formato e são relatadas.
//! - **Ossos:** pelo nome do nó de cada junta no esqueleto (`.bon`) do modelo, gravados como `.ski` v9 (nomes,
//!   `A3DSkin::BindSkeleton`, `A3DSkin.cpp:830-865`). O nó "esqueleto" do exportador é a vaga extra que
//!   `BindSkeleton` reserva ao próprio esqueleto (índice = número de nomes).
//! - **Material:** o glTF só tem a cor. Com `extras.angelica` (gravado pelo exportador) os valores voltam exatos; sem
//!   ele, valem os mais comuns no cliente, medidos em `ski::testes::medir_padroes`: em 83% dos 28.421 materiais do
//!   1.5.5, ambiente = difusa, emissiva e especular (0, 0, 0, 1) e brilho 10. O `fMinWeight` é 0 em todos os 24.165
//!   `.ski` (e o motor só o lê e regrava, `A3DSkin.cpp:220`, `:538`).
//! - **Texturas:** o nome da imagem do glTF (o exportador grava o nome original); a imagem embutida em PNG vira DDS de
//!   DXT1/DXT5 com mipmaps (B292/B293, `imagem::dds_dxt`), com o nome em `.dds` — em `texturas_novas`.
//!
//! Primitiva sem pele, índice que não cabe em `u16` e modo diferente de triângulos são recusados ou relatados — nada
//! é inventado.

use crate::bon::Bon;
use crate::ski::{MalhaDePele, MalhaRigida, Material, Ski, Vertice, VerticeComPeso};
use serde_json::Value;

pub struct Importacao {
    pub ski: Ski,
    /// B292: as texturas que o glTF traz embutidas, já como o cliente lê. PNG vira DDS com mipmaps — B293: DXT1 sem
    /// alfa, DXT5 com alfa (`imagem::dds_dxt`); DDS/TGA embutidos vão como estão.
    pub texturas_novas: Vec<TexturaNova>,
    /// O que foi ajustado ou deixado de fora, para o relatório.
    pub relatorio: Vec<String>,
}

pub struct TexturaNova {
    /// O nome no `.ski` (e do arquivo).
    pub nome: String,
    /// O arquivo que o cliente lê.
    pub bytes: Vec<u8>,
    /// Os pixels de origem (o PNG antes da compressão), para comparar com a textura que o cliente já tem — o DXT tem
    /// perda e nunca bateria pixel a pixel.
    pub pixels: Option<crate::imagem::Imagem>,
}

struct Glb<'a> {
    json: Value,
    bin: &'a [u8],
}

fn abrir(b: &[u8]) -> Result<Glb<'_>, String> {
    if b.len() < 20 || &b[0..4] != b"glTF" {
        return Err("não é GLB (falta a marca glTF)".into());
    }
    let u = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
    if u(4) != 2 {
        return Err(format!("GLB versão {} (só a 2)", u(4)));
    }
    let n_json = u(12);
    if &b[16..20] != b"JSON" || 20 + n_json > b.len() {
        return Err("bloco JSON do GLB ilegível".into());
    }
    let json: Value = serde_json::from_slice(&b[20..20 + n_json]).map_err(|e| format!("JSON do GLB: {e}"))?;
    let o = 20 + n_json;
    let bin = if o + 8 <= b.len() && &b[o + 4..o + 8] == b"BIN\0" { b.get(o + 8..o + 8 + u(o)).ok_or("bloco BIN curto")? } else { &[] };
    Ok(Glb { json, bin })
}

impl Glb<'_> {
    /// Os valores de um acessor, cada elemento como `f32` (inteiros normalizados viram 0..1 se `normalized`).
    fn acessor(&self, i: usize) -> Result<Vec<Vec<f32>>, String> {
        let a = &self.json["accessors"][i];
        let n = a["count"].as_u64().ok_or("acessor sem count")? as usize;
        let comps = match a["type"].as_str() {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            outro => return Err(format!("tipo de acessor {outro:?}")),
        };
        let tipo = a["componentType"].as_u64().ok_or("acessor sem componentType")?;
        let tam = match tipo {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            t => return Err(format!("componentType {t}")),
        };
        let normalizado = a["normalized"].as_bool().unwrap_or(false);
        let v = &self.json["bufferViews"][a["bufferView"].as_u64().ok_or("acessor sem bufferView (esparso não é aceito)")? as usize];
        if v["buffer"].as_u64().unwrap_or(0) != 0 {
            return Err("só o buffer 0 (o BIN do GLB)".into());
        }
        let base = v["byteOffset"].as_u64().unwrap_or(0) as usize + a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let passo = v["byteStride"].as_u64().map_or(comps * tam, |x| x as usize);
        let mut r = Vec::with_capacity(n);
        for k in 0..n {
            let mut e = Vec::with_capacity(comps);
            for c in 0..comps {
                let o = base + k * passo + c * tam;
                let x = self.bin.get(o..o + tam).ok_or("acessor além do BIN")?;
                e.push(match tipo {
                    5126 => f32::from_le_bytes(x.try_into().unwrap()),
                    5121 => x[0] as f32 / if normalizado { 255.0 } else { 1.0 },
                    5123 => u16::from_le_bytes(x.try_into().unwrap()) as f32 / if normalizado { 65535.0 } else { 1.0 },
                    5125 => u32::from_le_bytes(x.try_into().unwrap()) as f32,
                    5120 => x[0] as i8 as f32,
                    _ => i16::from_le_bytes(x.try_into().unwrap()) as f32,
                });
            }
            r.push(e);
        }
        Ok(r)
    }
}

/// A matriz local de um nó glTF (coluna a coluna): `matrix`, ou `T × R × S` (`translation`, `rotation` x,y,z,w,
/// `scale`).
fn matriz_local(no: &Value) -> [f32; 16] {
    if let Some(m) = no["matrix"].as_array().filter(|m| m.len() == 16) {
        return std::array::from_fn(|i| m[i].as_f64().unwrap_or(0.0) as f32);
    }
    let v = |k: &str, n: usize, p: f32| -> Vec<f32> { no[k].as_array().filter(|a| a.len() == n).map_or(vec![p; n], |a| a.iter().map(|x| x.as_f64().unwrap_or(p as f64) as f32).collect()) };
    let t = v("translation", 3, 0.0);
    let r = no["rotation"].as_array().filter(|a| a.len() == 4).map_or(vec![0.0, 0.0, 0.0, 1.0], |a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect());
    let s = v("scale", 3, 1.0);
    let (x, y, z, w) = (r[0], r[1], r[2], r[3]);
    let rot = [
        1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w),
        2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w),
        2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y),
    ];
    let mut m = [0f32; 16];
    for c in 0..3 {
        for l in 0..3 {
            m[c * 4 + l] = rot[c * 3 + l] * s[c];
        }
    }
    m[12] = t[0];
    m[13] = t[1];
    m[14] = t[2];
    m[15] = 1.0;
    m
}

/// `a × b` com matrizes coluna a coluna (glTF).
fn multiplicar_cc(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut r = [0f32; 16];
    for c in 0..4 {
        for l in 0..4 {
            r[c * 4 + l] = (0..4).map(|k| a[k * 4 + l] * b[c * 4 + k]).sum();
        }
    }
    r
}

/// A matriz do nó `i` no mundo: a das mães × a local.
fn mundo(i: usize, nos: &[Value], pais: &[Option<usize>]) -> [f32; 16] {
    let mut m = matriz_local(&nos[i]);
    let mut p = pais[i];
    let mut passos = 0;
    while let (Some(k), true) = (p, passos < 256) {
        m = multiplicar_cc(&matriz_local(&nos[k]), &m);
        p = pais[k];
        passos += 1;
    }
    m
}

/// A matriz (coluna a coluna) aplicada a um ponto (`w` = 1) ou direção (`w` = 0).
fn aplicar(m: &[f32; 16], v: [f32; 3], w: f32) -> [f32; 3] {
    std::array::from_fn(|l| m[l] * v[0] + m[4 + l] * v[1] + m[8 + l] * v[2] + m[12 + l] * w)
}

fn normalizar(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n > 0.0 { v.map(|x| x / n) } else { v }
}

/// B295: translação e quatérnio (x, y, z, w) de uma matriz glTF sem escala (coluna a coluna).
fn decompor(m: &[f32; 16]) -> ([f32; 3], [f32; 4]) {
    let t = [m[12], m[13], m[14]];
    let col = |c: usize| normalizar([m[c * 4], m[c * 4 + 1], m[c * 4 + 2]]);
    let (a, b, c) = (col(0), col(1), col(2));
    // R[linha][coluna]: a coluna c é o eixo c.
    let (r00, r11, r22) = (a[0], b[1], c[2]);
    let traco = r00 + r11 + r22;
    let q = if traco > 0.0 {
        let s = (traco + 1.0).sqrt() * 2.0;
        [(b[2] - c[1]) / s, (c[0] - a[2]) / s, (a[1] - b[0]) / s, 0.25 * s]
    } else if r00 > r11 && r00 > r22 {
        let s = (1.0 + r00 - r11 - r22).sqrt() * 2.0;
        [0.25 * s, (b[0] + a[1]) / s, (c[0] + a[2]) / s, (b[2] - c[1]) / s]
    } else if r11 > r22 {
        let s = (1.0 + r11 - r00 - r22).sqrt() * 2.0;
        [(b[0] + a[1]) / s, 0.25 * s, (c[1] + b[2]) / s, (c[0] - a[2]) / s]
    } else {
        let s = (1.0 + r22 - r00 - r11).sqrt() * 2.0;
        [(c[0] + a[2]) / s, (c[1] + b[2]) / s, 0.25 * s, (a[1] - b[0]) / s]
    };
    (t, q)
}

/// Uma ação importada do glTF: a `.stck` e o último quadro (a ação vai do quadro 0 a ele, no FPS do esqueleto).
pub struct AcaoImportada {
    pub nome: String,
    pub stck: crate::stck::Stck,
    pub quadro_final: f32,
}

/// B295 (S6): as animações do glTF como ações do modelo, para o esqueleto `esqueleto`.
/// - Cada osso usa a trilha da sua primeira junta (`A3DSkinModelAct.cpp:612`), e **todo** osso precisa de uma: sem
///   ela, `GetBoneStateOnTrack` falha (`:379-384`) e o chamador usa posição e rotação sem valor. Osso sem canal no glTF
///   fica constante na pose de repouso do seu nó (`translation`/`rotation` ou a `matrix` decomposta).
/// - Os canais são os valores locais do osso. Volta-se à mão do Direct3D como o exportador (B276) foi: posição
///   (x, y, −z) e quatérnio (−x, −y, z, w).
/// - Amostragem: uma chave por quadro no FPS do esqueleto (o `FrameToTime` da ação usa esse FPS); no cliente as `.stck`
///   têm 15 (83%) ou 30 quadros por segundo, iguais ao FPS de cada trilha. Um segmento de 0 ao fim. Interpolação do glTF
///   `LINEAR` (rotação por *nlerp*) e `STEP`; `CUBICSPLINE` usa o valor do meio e é relatada. Canal de escala é relatado
///   e ignorado (a trilha só tem posição e rotação).
pub fn acoes_do_glb(glb: &[u8], esqueleto: &Bon) -> Result<(Vec<AcaoImportada>, Vec<String>), String> {
    let g = abrir(glb)?;
    let j = &g.json;
    let mut relatorio = Vec::new();
    let nos = j["nodes"].as_array().cloned().unwrap_or_default();
    let fps = if esqueleto.fps > 0 { esqueleto.fps } else {
        relatorio.push("o esqueleto não tem FPS: usado 15 (o mais comum nas .stck do cliente)".into());
        15
    };
    // O nó de cada osso, pelo nome.
    let no_do_osso: Vec<Option<usize>> = esqueleto.ossos.iter().map(|o| nos.iter().position(|n| n["name"].as_str() == Some(o.nome.as_str()))).collect();
    let faltam = no_do_osso.iter().filter(|x| x.is_none()).count();
    if faltam > 0 {
        relatorio.push(format!("{faltam} osso(s) do esqueleto sem nó de mesmo nome no glTF: ficam na pose de ligação do .bon"));
    }
    let mut r = Vec::new();
    for (ai, anim) in j["animations"].as_array().cloned().unwrap_or_default().iter().enumerate() {
        let nome = anim["name"].as_str().map(str::to_string).unwrap_or_else(|| format!("acao{ai}"));
        // (nó, caminho) → (tempos, valores, interpolação)
        let mut canais: std::collections::HashMap<(usize, String), (Vec<f32>, Vec<Vec<f32>>, String)> = std::collections::HashMap::new();
        let mut duracao = 0f32;
        for c in anim["channels"].as_array().cloned().unwrap_or_default() {
            let (Some(no), Some(caminho)) = (c["target"]["node"].as_u64(), c["target"]["path"].as_str()) else { continue };
            if caminho == "scale" || caminho == "weights" {
                relatorio.push(format!("{nome}: canal \"{caminho}\" ignorado (a trilha do Angelica só tem posição e rotação)"));
                continue;
            }
            let s = &anim["samplers"][c["sampler"].as_u64().unwrap_or(u64::MAX) as usize];
            let tempos: Vec<f32> = g.acessor(s["input"].as_u64().ok_or("sampler sem input")? as usize)?.iter().map(|x| x[0]).collect();
            let mut valores = g.acessor(s["output"].as_u64().ok_or("sampler sem output")? as usize)?;
            let interp = s["interpolation"].as_str().unwrap_or("LINEAR").to_string();
            if interp == "CUBICSPLINE" {
                relatorio.push(format!("{nome}: interpolação CUBICSPLINE — usado o valor de cada chave, sem as tangentes"));
                valores = valores.chunks(3).filter_map(|x| x.get(1).cloned()).collect();
            }
            duracao = duracao.max(tempos.last().copied().unwrap_or(0.0));
            canais.insert((no as usize, caminho.to_string()), (tempos, valores, interp));
        }
        if canais.is_empty() {
            relatorio.push(format!("{nome}: animação sem canais de posição/rotação — deixada de fora"));
            continue;
        }
        let n_chaves = (duracao * fps as f32).round() as usize + 1;
        let fim_ms = ((n_chaves - 1) as f32 * 1000.0 / fps as f32).round() as i32;
        let amostra = |no: usize, caminho: &str, t: f32| -> Option<Vec<f32>> {
            let (ts, vs, interp) = canais.get(&(no, caminho.to_string()))?;
            let i = ts.iter().position(|&x| x > t).unwrap_or(ts.len());
            if i == 0 { return vs.first().cloned(); }
            if i == ts.len() || interp == "STEP" { return vs.get(i - 1).cloned(); }
            let (a, b) = (&vs[i - 1], &vs[i]);
            let f = if ts[i] > ts[i - 1] { (t - ts[i - 1]) / (ts[i] - ts[i - 1]) } else { 0.0 };
            if caminho == "rotation" {
                let sinal = if a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() < 0.0 { -1.0 } else { 1.0 };
                let q: Vec<f32> = a.iter().zip(b).map(|(x, y)| x + (sinal * y - x) * f).collect();
                let n = q.iter().map(|x| x * x).sum::<f32>().sqrt();
                Some(q.iter().map(|x| x / n).collect())
            } else {
                Some(a.iter().zip(b).map(|(x, y)| x + (y - x) * f).collect())
            }
        };
        let mut trilhas: Vec<(i32, crate::trilha::TrilhaDeOsso)> = Vec::new();
        for (k, o) in esqueleto.ossos.iter().enumerate() {
            if o.primeira_junta < 0 || trilhas.iter().any(|x| x.0 == o.primeira_junta) {
                continue;
            }
            // Repouso: o nó (TRS ou matriz); sem nó, a pose de ligação local do .bon (inversa do matBoneInit relativa à mãe).
            let repouso: ([f32; 3], [f32; 4]) = match no_do_osso[k] {
                Some(n) => {
                    let no = &nos[n];
                    if no["matrix"].is_array() {
                        decompor(&matriz_local(no))
                    } else {
                        let v = |c: &str, p: Vec<f32>| no[c].as_array().map_or(p.clone(), |a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect());
                        let t = v("translation", vec![0.0; 3]);
                        let q = v("rotation", vec![0.0, 0.0, 0.0, 1.0]);
                        ([t[0], t[1], t[2]], [q[0], q[1], q[2], q[3]])
                    }
                }
                None => {
                    let mundo = |i: usize| crate::glb::inversa(&esqueleto.ossos[i].inicial).unwrap_or(esqueleto.ossos[i].relativa);
                    let local = if o.mae >= 0 && (o.mae as usize) < esqueleto.ossos.len() {
                        crate::glb::inversa(&mundo(o.mae as usize)).map_or(mundo(k), |pi| crate::bon::multiplicar(&mundo(k), &pi))
                    } else {
                        mundo(k)
                    };
                    decompor(&crate::glb::matriz_gltf(&local))
                }
            };
            let mut pos = Vec::with_capacity(n_chaves);
            let mut rot = Vec::with_capacity(n_chaves);
            for c in 0..n_chaves {
                let t = c as f32 / fps as f32;
                let p = no_do_osso[k].and_then(|n| amostra(n, "translation", t)).map_or(repouso.0, |v| [v[0], v[1], v[2]]);
                let q = no_do_osso[k].and_then(|n| amostra(n, "rotation", t)).map_or(repouso.1, |v| [v[0], v[1], v[2], v[3]]);
                pos.push([p[0], p[1], -p[2]]);
                rot.push([-q[0], -q[1], q[2], q[3]]);
            }
            let seg = vec![crate::trilha::Segmento { inicio_ms: 0, fim_ms, chave_inicial: 0, chave_final: n_chaves as i32 - 1 }];
            trilhas.push((o.primeira_junta, crate::trilha::TrilhaDeOsso {
                posicao: crate::trilha::Trilha { fps, chaves: pos, segmentos: seg.clone() },
                rotacao: crate::trilha::Trilha { fps, chaves: rot, segmentos: seg },
            }));
        }
        let stck = crate::stck::Stck { quadro_inicial: 0, quadro_final: n_chaves as i32 - 1, fps, trilhas, marca: true };
        r.push(AcaoImportada { nome, stck, quadro_final: (n_chaves - 1) as f32 });
    }
    Ok((r, relatorio))
}

/// A pele (`.ski` v9) do glTF `glb`, com os ossos casados pelo nome no `esqueleto`.
pub fn ski_do_glb(glb: &[u8], esqueleto: &Bon) -> Result<Importacao, String> {
    let g = abrir(glb)?;
    let j = &g.json;
    let mut relatorio = Vec::new();
    let nos = j["nodes"].as_array().cloned().unwrap_or_default();
    let nome_do_no = |i: u64| nos.get(i as usize).and_then(|n| n["name"].as_str()).unwrap_or("").to_string();
    let mut ossos: Vec<String> = Vec::new();
    let mut texturas: Vec<String> = Vec::new();
    let mut materiais: Vec<Material> = Vec::new();
    let mut pares_de_material: Vec<(i64, usize, i32)> = Vec::new();
    let mut texturas_novas: Vec<TexturaNova> = Vec::new();
    let mut malhas = Vec::new();
    let mut rigidas: Vec<MalhaRigida> = Vec::new();
    // A mãe de cada nó (pelos `children`), para a matriz no mundo e para achar o osso acima de uma malha sem pele.
    let mut pais: Vec<Option<usize>> = vec![None; nos.len()];
    for (i, n) in nos.iter().enumerate() {
        for c in n["children"].as_array().into_iter().flatten() {
            if let Some(c) = c.as_u64().filter(|c| (*c as usize) < nos.len()) {
                pais[c as usize] = Some(i);
            }
        }
    }
    for no in &nos {
        let Some(mi) = no["mesh"].as_u64() else { continue };
        let malha = &j["meshes"][mi as usize];
        let nome_malha = malha["name"].as_str().or(no["name"].as_str()).unwrap_or("malha").to_string();
        // B294: malha rígida — a marcada pelo exportador (`extras.angelica.rigida`, com skin de peso 1) e a sem pele
        // filha (direta ou não) de um nó de osso, como o Blender exporta um objeto preso a um osso.
        let marcada_rigida = malha["extras"]["angelica"]["rigida"].as_bool().unwrap_or(false);
        let hierarquia: Option<(i32, [f32; 16])> = if no["skin"].is_u64() {
            None
        } else {
            let i_no = nos.iter().position(|x| std::ptr::eq(x, no)).unwrap();
            let mut p = pais[i_no];
            let mut achado = None;
            while let Some(k) = p {
                if let Some(o) = esqueleto.ossos.iter().position(|o| o.nome == nome_do_no(k as u64)) {
                    achado = Some(o as i32);
                    break;
                }
                p = pais[k];
            }
            match achado {
                Some(o) => Some((o, mundo(i_no, &nos, &pais))),
                None => {
                    relatorio.push(format!("{nome_malha}: sem pele e sem osso acima dela — deixada de fora (prenda-a a um osso ou dê pesos)"));
                    continue;
                }
            }
        };
        // Juntas da pele → índice no .ski (nomes, na ordem de 1º uso) ou a vaga do esqueleto.
        let juntas: Vec<Option<String>> = match no["skin"].as_u64() {
            None => Vec::new(),
            Some(si) => j["skins"][si as usize]["joints"]
                .as_array()
                .ok_or("skin sem joints")?
                .iter()
                .map(|x| {
                    let n = nome_do_no(x.as_u64().unwrap_or(u64::MAX));
                    if n == "esqueleto" { None } else { Some(n) }
                })
                .collect(),
        };
        for (pi, prim) in malha["primitives"].as_array().cloned().unwrap_or_default().iter().enumerate() {
            let nome = if pi == 0 { nome_malha.clone() } else { format!("{nome_malha}_{pi}") };
            if prim["mode"].as_u64().unwrap_or(4) != 4 {
                relatorio.push(format!("{nome}: primitiva que não é de triângulos — deixada de fora"));
                continue;
            }
            let at = &prim["attributes"];
            let pega = |k: &str| at[k].as_u64().map(|i| g.acessor(i as usize)).transpose();
            let pos = pega("POSITION")?.ok_or(format!("{nome}: sem POSITION"))?;
            let nor = pega("NORMAL")?;
            let uv = pega("TEXCOORD_0")?;
            let (js, ws) = (pega("JOINTS_0")?.unwrap_or_default(), pega("WEIGHTS_0")?.unwrap_or_default());
            if hierarquia.is_none() && (js.len() != pos.len() || ws.len() != pos.len()) {
                relatorio.push(format!("{nome}: sem JOINTS_0/WEIGHTS_0 — deixada de fora"));
                continue;
            }
            if at.get("JOINTS_1").is_some() {
                relatorio.push(format!("{nome}: mais de 4 ossos por vértice (JOINTS_1) — o formato tem 4; os extras foram ignorados e os pesos renormalizados"));
            }
            if nor.is_none() {
                relatorio.push(format!("{nome}: sem NORMAL — normais zeradas"));
            }
            if uv.is_none() {
                relatorio.push(format!("{nome}: sem TEXCOORD_0 — UV zerado"));
            }
            if pos.len() > 65536 {
                return Err(format!("{nome}: {} vértices — o índice do .ski é u16 (até 65.536)", pos.len()));
            }
            // B294: malha rígida — o osso no esqueleto (o índice que `A3DRigidMesh` usa direto) e os vértices no espaço do
            // modelo (a matriz da rígida é `BoneInitTM × AbsoluteTM`, como a da pele, `A3DSkinModel.cpp:1341`).
            let rigida: Option<(i32, Vec<Vertice>)> = if let Some((osso, w)) = hierarquia {
                let vr = (0..pos.len())
                    .map(|k| {
                        let p = &pos[k];
                        let n = nor.as_ref().map_or(vec![0.0; 3], |x| x[k].clone());
                        let t = uv.as_ref().map_or(vec![0.0; 2], |x| x[k].clone());
                        let pm = aplicar(&w, [p[0], p[1], p[2]], 1.0);
                        let nm = normalizar(aplicar(&w, [n[0], n[1], n[2]], 0.0));
                        Vertice { posicao: [pm[0], pm[1], -pm[2]], normal: [nm[0], nm[1], -nm[2]], uv: [t[0], t[1]] }
                    })
                    .collect();
                Some((osso, vr))
            } else if marcada_rigida {
                let junta = js.first().and_then(|x| x.first()).copied().unwrap_or(0.0) as usize;
                let osso = match juntas.get(junta) {
                    Some(Some(n)) => esqueleto.ossos.iter().position(|o| &o.nome == n).ok_or(format!("{nome}: o osso \"{n}\" não existe no esqueleto"))? as i32,
                    _ => malha["extras"]["angelica"]["osso"].as_i64().unwrap_or(-1) as i32,
                };
                let vr = (0..pos.len())
                    .map(|k| {
                        let p = &pos[k];
                        let n = nor.as_ref().map_or(vec![0.0; 3], |x| x[k].clone());
                        let t = uv.as_ref().map_or(vec![0.0; 2], |x| x[k].clone());
                        Vertice { posicao: [p[0], p[1], -p[2]], normal: [n[0], n[1], -n[2]], uv: [t[0], t[1]] }
                    })
                    .collect();
                Some((osso, vr))
            } else {
                None
            };
            let mut vertices = Vec::with_capacity(pos.len());
            for k in 0..if rigida.is_some() { 0 } else { pos.len() } {
                let mut ossos_v = [0u8; 4];
                let mut w = [0f32; 4];
                for c in 0..4 {
                    let peso = ws[k].get(c).copied().unwrap_or(0.0);
                    if peso <= 0.0 {
                        continue;
                    }
                    let junta = js[k].get(c).copied().unwrap_or(0.0) as usize;
                    let alvo = match juntas.get(junta) {
                        None => return Err(format!("{nome}: junta {junta} fora da skin")),
                        Some(None) => None,
                        Some(Some(n)) => {
                            if !esqueleto.ossos.iter().any(|o| &o.nome == n) {
                                return Err(format!("{nome}: o osso \"{n}\" não existe no esqueleto"));
                            }
                            Some(n.clone())
                        }
                    };
                    let indice = match alvo {
                        Some(n) => match ossos.iter().position(|x| *x == n) {
                            Some(p) => p,
                            None => {
                                ossos.push(n);
                                ossos.len() - 1
                            }
                        },
                        None => usize::MAX, // a vaga do esqueleto: resolvida no fim, quando o número de nomes é conhecido
                    };
                    if indice != usize::MAX && indice > 254 {
                        return Err(format!("{nome}: mais de 255 ossos na pele"));
                    }
                    ossos_v[c] = if indice == usize::MAX { 255 } else { indice as u8 };
                    w[c] = peso;
                }
                let soma: f32 = w.iter().sum();
                if soma > 0.0 {
                    w.iter_mut().for_each(|x| *x /= soma);
                }
                let p = &pos[k];
                let n = nor.as_ref().map_or(vec![0.0; 3], |x| x[k].clone());
                let t = uv.as_ref().map_or(vec![0.0; 2], |x| x[k].clone());
                vertices.push(VerticeComPeso {
                    posicao: [p[0], p[1], -p[2]],
                    pesos: [w[0], w[1], w[2]],
                    ossos: ossos_v,
                    normal: [n[0], n[1], -n[2]],
                    uv: [t[0], t[1]],
                });
            }
            let idx: Vec<u16> = match prim["indices"].as_u64() {
                Some(i) => g.acessor(i as usize)?.iter().map(|x| x[0] as u16).collect(),
                None => (0..pos.len() as u16).collect(),
            };
            if idx.len() % 3 != 0 {
                return Err(format!("{nome}: {} índices (não é múltiplo de 3)", idx.len()));
            }
            let indices: Vec<u16> = idx.chunks_exact(3).flat_map(|c| [c[0], c[2], c[1]]).collect();
            // Material e textura.
            let mi_gltf = prim["material"].as_i64().unwrap_or(-1);
            let (material, textura) = match pares_de_material.iter().find(|x| x.0 == mi_gltf) {
                Some(x) => (x.1 as i32, x.2),
                None => {
                    let m = &j["materials"][mi_gltf.max(0) as usize];
                    let tex = m["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64().and_then(|t| {
                        let img = &j["images"][j["textures"][t as usize]["source"].as_u64()? as usize];
                        let nome_img = img["name"].as_str().or(img["uri"].as_str()).map(|x| x.rsplit(['/', '\\']).next().unwrap_or(x).to_string()).unwrap_or_else(|| nome.clone());
                        // B292: a imagem embutida (bufferView) vira o arquivo que o cliente lê.
                        let bruto = img["bufferView"].as_u64().and_then(|v| {
                            let bv = &j["bufferViews"][v as usize];
                            let o = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
                            g.bin.get(o..o + bv["byteLength"].as_u64()? as usize).map(<[u8]>::to_vec)
                        });
                        let minusculo = nome_img.to_lowercase();
                        let destino = if minusculo.ends_with(".dds") || minusculo.ends_with(".tga") {
                            nome_img.clone()
                        } else {
                            format!("{}.dds", nome_img.rsplit_once('.').map_or(nome_img.as_str(), |x| x.0))
                        };
                        match bruto {
                            Some(b) if b.starts_with(b"DDS ") || (minusculo.ends_with(".tga") && crate::imagem::tga(&b).is_some()) => {
                                if !texturas_novas.iter().any(|x| x.nome == destino) {
                                    let pixels = crate::imagem::decodificar(&b);
                                    texturas_novas.push(TexturaNova { nome: destino.clone(), bytes: b, pixels });
                                }
                            }
                            Some(b) => match crate::imagem::png(&b) {
                                Some(im) => {
                                    if !texturas_novas.iter().any(|x| x.nome == destino) {
                                        texturas_novas.push(TexturaNova { nome: destino.clone(), bytes: crate::imagem::dds_dxt(&im), pixels: Some(im) });
                                    }
                                }
                                None => relatorio.push(format!("{nome}: a imagem \"{nome_img}\" não é PNG/DDS/TGA (JPEG ainda não) — a textura fica só pelo nome")),
                            },
                            None => relatorio.push(format!("{nome}: a imagem \"{nome_img}\" não vem embutida no GLB — a textura fica só pelo nome")),
                        }
                        Some(destino)
                    });
                    let textura = match tex {
                        Some(t) => {
                            match texturas.iter().position(|x| *x == t) {
                                Some(p) => p as i32,
                                None => {
                                    texturas.push(t);
                                    texturas.len() as i32 - 1
                                }
                            }
                        }
                        None => -1,
                    };
                    let a = &m["extras"]["angelica"];
                    let cor = |v: &Value, padrao: [f32; 4]| -> [f32; 4] {
                        v.as_array().filter(|x| x.len() == 4).map_or(padrao, |x| std::array::from_fn(|i| x[i].as_f64().unwrap_or(0.0) as f32))
                    };
                    let base = cor(&m["pbrMetallicRoughness"]["baseColorFactor"], [1.0; 4]);
                    let difusa = cor(&a["difusa"], base);
                    let material = Material {
                        nome: m["name"].as_str().filter(|x| !x.is_empty()).unwrap_or(&nome).to_string(),
                        ambiente: cor(&a["ambiente"], difusa),
                        difusa,
                        emissiva: cor(&a["emissiva"], [0.0, 0.0, 0.0, 1.0]),
                        especular: cor(&a["especular"], [0.0, 0.0, 0.0, 1.0]),
                        brilho: a["brilho"].as_f64().map_or(10.0, |x| x as f32),
                        dois_lados: m["doubleSided"].as_bool().unwrap_or(false),
                        dois_lados_byte: 0,
                        linha: String::new(),
                    };
                    materiais.push(material);
                    pares_de_material.push((mi_gltf, materiais.len() - 1, textura));
                    (materiais.len() as i32 - 1, textura)
                }
            };
            match rigida {
                Some((osso, vr)) => rigidas.push(MalhaRigida { nome, osso, textura, material, vertices: vr, indices }),
                None => malhas.push(MalhaDePele { nome, textura, material, vertices, indices }),
            }
        }
    }
    if malhas.is_empty() && rigidas.is_empty() {
        return Err(format!("nenhuma malha importável no glTF ({})", relatorio.join("; ")));
    }
    // A vaga do esqueleto é o índice logo depois dos nomes.
    let vaga = ossos.len() as u8;
    for m in &mut malhas {
        for v in &mut m.vertices {
            v.ossos.iter_mut().filter(|o| **o == 255).for_each(|o| *o = vaga);
        }
    }
    let ski = Ski {
        versao: 9,
        peso_minimo: 0.0,
        ossos_do_esqueleto: esqueleto.ossos.len() as i32,
        ossos: ossos.clone(),
        texturas,
        materiais,
        malhas_de_pele: malhas,
        malhas_rigidas: rigidas,
        marca: true,
        ossos_no_cabecalho: ossos.len() as i32,
        reservado: vec![0; 52],
    };
    Ok(Importacao { ski, texturas_novas, relatorio })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::glb::{do_modelo, PeleDoModelo};

    /// As influências de um vértice: (nome do osso, peso normalizado), na ordem dos 4 lugares. O 4º peso é implícito
    /// (`1 − soma`) e, medido, sobra como ruído de arredondamento (~1e-8) num lado e some no outro — o osso desse lugar
    /// é o que estiver no índice. Influências abaixo de 1e-6 não contam.
    fn influencias(pesos: [f32; 3], ossos: [u8; 4], nome: &dyn Fn(u8) -> String) -> Vec<(String, f32)> {
        let w = [pesos[0], pesos[1], pesos[2], (1.0 - pesos[0] - pesos[1] - pesos[2]).max(0.0)];
        let soma: f32 = w.iter().sum();
        (0..4).filter(|&k| w[k] / soma > 1e-6).map(|k| (nome(ossos[k]), w[k] / soma)).collect()
    }

    /// O valor da trilha no tempo `t` pelas amostras (linear; quatérnio renormalizado), como o exportador amostra.
    fn valor<const N: usize>(a: &[(f32, [f32; N])], t: f32) -> [f32; N] {
        match a.iter().position(|x| x.0 >= t) {
            None => a.last().map_or([0.0; N], |x| x.1),
            Some(0) => a[0].1,
            Some(i) => {
                let (p, q) = (a[i - 1], a[i]);
                let f = if q.0 > p.0 { (t - p.0) / (q.0 - p.0) } else { 0.0 };
                // O caminho curto do quatérnio, como o exportador (`glb::valor_em`).
                let sinal = if N == 4 && (0..N).map(|k| p.1[k] * q.1[k]).sum::<f32>() < 0.0 { -1.0 } else { 1.0 };
                let mut v: [f32; N] = std::array::from_fn(|k| p.1[k] * (1.0 - f) + q.1[k] * sinal * f);
                if N == 4 {
                    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                    v.iter_mut().for_each(|x| *x /= n);
                }
                v
            }
        }
    }

    /// B295: ida e volta de ações reais — exportada por `glb::do_modelo_animado` (B276) e importada por `acoes_do_glb`:
    /// para cada osso e quadro, a chave importada é o valor da trilha original naquele instante (posição com tolerância
    /// de 1e-3; quatérnio até o sinal, 1e-3). A `.stck` gerada se reescreve e relê.
    #[test]
    fn acoes_ida_e_volta() {
        use crate::glb::{do_modelo_animado, AnimacaoDoModelo, PeleDoModelo};
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models\npcs");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let disco = |s: &str| encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(s).0).0.into_owned();
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut n, mut v5, mut v6, mut chaves, mut pior_p, mut pior_q) = (0usize, 0usize, 0usize, 0usize, 0f32, 0f32);
        let (mut na_chave, mut pior_entre) = (0usize, 0f32);
        while let Some(d) = pilha.pop() {
            if v5 >= 20 && v6 >= 20 { break; }
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("smd")) { continue; }
                n += 1;
                if n % 3 != 0 { continue; }
                let Ok(c) = crate::smd::cabeca(&std::fs::read(&p).unwrap()) else { continue };
                let pasta = p.parent().unwrap();
                let Ok(b) = std::fs::read(pasta.join(disco(&c.esqueleto))).map_err(|_| ()).and_then(|x| Bon::ler(&x).map_err(|_| ())) else { continue };
                let Some(ac) = c.lista_de_acoes.first() else { continue };
                if (c.versao < 7 && b.versao >= 6) || b.fps <= 0 { continue; }
                let skis: Vec<Ski> = c.peles.iter().filter_map(|s| Ski::ler(&std::fs::read(pasta.join(disco(s))).ok()?).ok()).collect();
                if skis.is_empty() { continue; }
                let stck;
                let trilhas: Vec<(i32, &crate::trilha::TrilhaDeOsso)> = if b.versao < 6 {
                    if v5 >= 20 { continue; }
                    b.trilhas.iter().enumerate().map(|(i, t)| (i as i32, t)).collect()
                } else {
                    if v6 >= 20 { continue; }
                    let dir = if c.versao >= 8 { c.pasta_das_trilhas.clone() } else { format!("tcks_{}", p.file_stem().unwrap().to_string_lossy()) };
                    let Some(nome) = &ac.stck else { continue };
                    let Ok(x) = std::fs::read(pasta.join(disco(&dir)).join(disco(nome))) else { continue };
                    let Ok(st) = crate::stck::ler(&x) else { continue };
                    stck = st;
                    stck.trilhas.iter().map(|(j, t)| (*j, t)).collect()
                };
                let fps = b.fps as f32;
                let acao = AnimacaoDoModelo { nome: ac.nome.clone(), trilhas: trilhas.clone(), inicio_ms: ac.quadro_inicial * 1000.0 / fps, fim_ms: ac.quadro_final * 1000.0 / fps };
                if acao.fim_ms <= acao.inicio_ms { continue; }
                let sem = |_: usize| None;
                let peles: Vec<PeleDoModelo> = skis.iter().map(|s| PeleDoModelo { ski: s, textura: &sem }).collect();
                let glb = do_modelo_animado(&peles, &b, &acao, "t");
                let (acoes, _) = acoes_do_glb(&glb, &b).unwrap();
                let Some(nova) = acoes.first() else { continue };
                if b.versao < 6 { v5 += 1 } else { v6 += 1 }
                assert_eq!(nova.nome, ac.nome);
                let w = crate::stck::escrever(&nova.stck);
                assert!(crate::stck::ler(&w).is_ok_and(|s| crate::stck::escrever(&s) == w));
                for o in &b.ossos {
                    let Some((_, orig)) = trilhas.iter().find(|(j, _)| *j == o.primeira_junta) else { continue };
                    let (pa, ra) = (orig.posicao.amostras(), orig.rotacao.amostras());
                    if pa.is_empty() || ra.is_empty() { continue; }
                    let (_, t) = nova.stck.trilhas.iter().find(|(j, _)| *j == o.primeira_junta).expect("todo osso tem trilha");
                    for (k, (pn, qn)) in t.posicao.chaves.iter().zip(&t.rotacao.chaves).enumerate() {
                        let tempo = (acao.inicio_ms + k as f32 * 1000.0 / fps).min(acao.fim_ms);
                        let (po, qo) = (valor(&pa, tempo), valor(&ra, tempo));
                        let dp = (0..3).map(|i| (po[i] - pn[i]).abs()).fold(0f32, f32::max);
                        let dot: f32 = (0..4).map(|i| qo[i] * qn[i]).sum();
                        let dq = (0..4).map(|i| (qo[i] * dot.signum() - qn[i]).abs()).fold(0f32, f32::max);
                        pior_p = pior_p.max(dp / (1.0 + po.iter().map(|x| x.abs()).fold(0f32, f32::max)));
                        // Na chave original (a menos de 0,5 ms) tem que bater; entre chaves fora da grade de quadros, a
                        // reamostragem sobre as amostras já normalizadas do GLB difere um pouco — medido e relatado.
                        if ra.iter().any(|x| (x.0 - tempo).abs() < 0.5) {
                            pior_q = pior_q.max(dq);
                            na_chave += 1;
                        } else {
                            pior_entre = pior_entre.max(dq);
                        }
                        chaves += 1;
                    }
                }
            }
        }
        eprintln!("ações: v5 {v5}, v6 {v6}, chaves comparadas {chaves} ({na_chave} na chave original); maior diferença relativa de posição {pior_p:.2e}, de quatérnio na chave {pior_q:.2e}, entre chaves {pior_entre:.2e}");
        assert!(v5 + v6 >= 20 && chaves > 1000, "{v5} {v6} {chaves}");
        assert!(pior_p < 1e-3 && pior_q < 1e-3, "{pior_p} {pior_q}");
    }

    /// B294: malha sem pele filha de um osso (o que o Blender exporta para um objeto preso a um osso) vira rígida desse
    /// osso, com os vértices no espaço do modelo: a matriz do nó no mundo (osso transladado (1, 2, 3), malha (0, 0, 1)
    /// dentro dele) e depois o `z` negado; sem osso acima, fica de fora com relatório.
    #[test]
    fn rigida_por_hierarquia() {
        let pos: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bin: Vec<u8> = pos.iter().flat_map(|x| x.to_le_bytes()).collect();
        let montar = |pai_e_osso: bool| -> Vec<u8> {
            let nos = if pai_e_osso {
                serde_json::json!([{ "name": "raiz", "translation": [1.0, 2.0, 3.0], "children": [1] }, { "name": "enfeite", "mesh": 0, "translation": [0.0, 0.0, 1.0] }])
            } else {
                serde_json::json!([{ "name": "enfeite", "mesh": 0 }])
            };
            let j = serde_json::json!({
                "asset": { "version": "2.0" }, "nodes": nos,
                "meshes": [{ "name": "enfeite", "primitives": [{ "attributes": { "POSITION": 0 } }] }],
                "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }],
                "bufferViews": [{ "buffer": 0, "byteLength": 36 }], "buffers": [{ "byteLength": 36 }],
            });
            let mut t = serde_json::to_vec(&j).unwrap();
            while t.len() % 4 != 0 { t.push(b' '); }
            let total = 12 + 8 + t.len() + 8 + bin.len();
            let mut g = b"glTF".to_vec();
            g.extend_from_slice(&2u32.to_le_bytes());
            g.extend_from_slice(&(total as u32).to_le_bytes());
            g.extend_from_slice(&(t.len() as u32).to_le_bytes());
            g.extend_from_slice(b"JSON");
            g.extend_from_slice(&t);
            g.extend_from_slice(&(bin.len() as u32).to_le_bytes());
            g.extend_from_slice(b"BIN\0");
            g.extend_from_slice(&bin);
            g
        };
        let id: crate::bon::Matriz = [1.0, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
        let osso = crate::bon::Osso { nome: "raiz".into(), flags: 0, mae: -1, primeira_junta: 0, relativa: id, inicial: id, filhas: vec![], enchimento: [0; 3] };
        let bon = Bon { versao: 6, quadro_inicial: 0, quadro_final: 0, fps: 30, ossos: vec![osso], juntas: vec![], trilhas: vec![], ganchos: vec![], marca: true, reservado: vec![] };
        let i = ski_do_glb(&montar(true), &bon).unwrap();
        assert_eq!((i.ski.malhas_de_pele.len(), i.ski.malhas_rigidas.len()), (0, 1));
        let r = &i.ski.malhas_rigidas[0];
        assert_eq!(r.osso, 0);
        assert_eq!(r.vertices.iter().map(|v| v.posicao).collect::<Vec<_>>(), vec![[1.0, 2.0, -4.0], [2.0, 2.0, -4.0], [1.0, 3.0, -4.0]]);
        assert_eq!(r.indices, vec![0, 2, 1], "ordem do triângulo invertida");
        let w = i.ski.escrever();
        assert!(Ski::ler(&w).is_ok_and(|x| x.malhas_rigidas.len() == 1));
        let sem_osso = ski_do_glb(&montar(false), &bon).err().unwrap();
        assert!(sem_osso.contains("sem pele e sem osso acima dela"), "{sem_osso}");
    }

    /// B294: o osso da malha rígida é o índice no esqueleto — `A3DRigidMesh` desenha com `GetBlendMatrix(m_iBoneIdx)`
    /// direto (`A3DRigidMesh.cpp:367`), sem o `MapBoneIndex` que a pele usa (`A3DSkinMesh.cpp:856`). Medida: em quantas
    /// malhas rígidas de `.ski` v9 o nome pelo esqueleto difere do nome pela lista da pele (o que o exportador usava).
    #[test]
    #[ignore = "medição: cargo test -p pw-data-loader --lib medir_osso_da_rigida -- --ignored --nocapture"]
    fn medir_osso_da_rigida() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut rigidas, mut v9, mut difere, mut fora) = (0usize, 0usize, 0usize, 0usize);
        let mut ex = Vec::new();
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("smd")) { continue; }
                let Ok(c) = crate::smd::cabeca(&std::fs::read(&p).unwrap()) else { continue };
                let pasta = p.parent().unwrap();
                let no_disco = |t: &str| encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(t).0).0.into_owned();
                let Some(bon) = std::fs::read(pasta.join(no_disco(&c.esqueleto))).ok().and_then(|b| Bon::ler(&b).ok()) else { continue };
                for s in c.peles.iter().filter_map(|s| std::fs::read(pasta.join(no_disco(s))).ok().and_then(|b| Ski::ler(&b).ok())) {
                    for m in &s.malhas_rigidas {
                        rigidas += 1;
                        if s.versao < 9 { continue; }
                        v9 += 1;
                        let pelo_esqueleto = bon.ossos.get(m.osso as usize).map(|o| o.nome.clone());
                        let pela_pele = s.ossos.get(m.osso as usize).cloned();
                        if pelo_esqueleto.is_none() { fora += 1; }
                        if pelo_esqueleto != pela_pele {
                            difere += 1;
                            if ex.len() < 3 { ex.push(format!("{} osso {}: esqueleto {pelo_esqueleto:?} × pele {pela_pele:?}", m.nome, m.osso)); }
                        }
                    }
                }
            }
        }
        eprintln!("rígidas {rigidas}; em .ski v9 {v9}; nome diferente {difere}; índice fora do esqueleto {fora}; ex {ex:?}");
    }

    /// B289: ida e volta em modelos reais — o `.smd` exportado pelo nosso GLB (`glb::do_modelo`) e importado de volta
    /// dá as mesmas malhas: posição, normal, UV e índices exatos, e por vértice as mesmas influências (osso pelo nome,
    /// peso com tolerância de 1e-5). O `.ski` importado se reescreve e relê.
    #[test]
    fn ida_e_volta_em_modelos_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut n_smd, mut testados, mut sem_pele_ligada, mut malhas, mut vertices, mut rigidas) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        let mut erros: Vec<String> = Vec::new();
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("smd")) { continue; }
                n_smd += 1;
                if n_smd % 5 != 0 { continue; } // amostra de 1/5
                let Ok(c) = crate::smd::cabeca(&std::fs::read(&p).unwrap()) else { continue };
                let pasta = p.parent().unwrap();
                // Os nomes desta instalação desempacotada estão em disco como os bytes GBK lidos em Windows-1252 (B273).
                let no_disco = |t: &str| encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(t).0).0.into_owned();
                let Some(bon) = std::fs::read(pasta.join(no_disco(&c.esqueleto))).ok().and_then(|b| Bon::ler(&b).ok()) else { continue };
                let skis: Vec<Ski> = c.peles.iter().filter_map(|s| std::fs::read(pasta.join(no_disco(s))).ok().and_then(|b| Ski::ler(&b).ok())).collect();
                if skis.is_empty() || skis.len() != c.peles.len() { continue; }
                let sem_textura = |_: usize| None;
                let peles: Vec<PeleDoModelo> = skis.iter().map(|s| PeleDoModelo { ski: s, textura: &sem_textura }).collect();
                let glb = do_modelo(&peles, Some(&bon), "teste");
                // Só modelos em que toda pele casou com o esqueleto (senão o exportador as manda sem skin).
                let esperadas: usize = skis.iter().map(|s| s.malhas_de_pele.len() + s.malhas_rigidas.len()).sum();
                let imp = match ski_do_glb(&glb, &bon) {
                    Ok(i) => i,
                    Err(x) => { sem_pele_ligada += 1; if !x.starts_with("nenhuma malha") && erros.len() < 5 { erros.push(format!("{}: {x}", p.display())); } continue }
                };
                if imp.ski.malhas_de_pele.len() + imp.ski.malhas_rigidas.len() != esperadas { sem_pele_ligada += 1; continue; }
                testados += 1;
                let novo = &imp.ski;
                let nome_novo = |i: u8| novo.ossos.get(i as usize).cloned().unwrap_or_else(|| "esqueleto".into());
                let mut k = 0;
                let mut kr = 0;
                for s in &skis {
                    let nome_orig = |i: u8| -> String {
                        if s.versao >= 9 { s.ossos.get(i as usize).cloned().unwrap_or_else(|| "esqueleto".into()) }
                        else if (i as i32) < s.ossos_do_esqueleto { bon.ossos[i as usize].nome.clone() } else { "esqueleto".into() }
                    };
                    let mut comparar = |nome: &str, pos: Vec<[f32; 3]>, nor: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, idx: &[u16], infl: Vec<Vec<(String, f32)>>| {
                        let m = &novo.malhas_de_pele[k];
                        k += 1;
                        malhas += 1;
                        vertices += pos.len();
                        let ok_geo = m.vertices.len() == pos.len()
                            && m.vertices.iter().zip(&pos).all(|(v, p)| v.posicao == *p)
                            && m.vertices.iter().zip(&nor).all(|(v, n)| v.normal == *n)
                            && m.vertices.iter().zip(&uv).all(|(v, t)| v.uv == *t)
                            && m.indices == idx;
                        let ok_pesos = m.vertices.iter().zip(&infl).all(|(v, a)| {
                            let b = influencias(v.pesos, v.ossos, &nome_novo);
                            a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| x.0 == y.0 && (x.1 - y.1).abs() < 1e-5)
                        });
                        if !(ok_geo && ok_pesos) && erros.len() < 5 {
                            let ex = m.vertices.iter().zip(&infl).find(|(v, a)| {
                                let b = influencias(v.pesos, v.ossos, &nome_novo);
                                !(a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| x.0 == y.0 && (x.1 - y.1).abs() < 1e-5))
                            }).map(|(v, a)| format!("orig {a:?} × novo {:?}", influencias(v.pesos, v.ossos, &nome_novo)));
                            erros.push(format!("{} / {nome}: geometria {ok_geo}, pesos {ok_pesos} {ex:?}", p.display()));
                        }
                    };
                    for x in &s.malhas_de_pele {
                        let infl = x.vertices.iter().map(|v| influencias(v.pesos, v.ossos, &nome_orig)).collect();
                        comparar(&x.nome, x.vertices.iter().map(|v| v.posicao).collect(), x.vertices.iter().map(|v| v.normal).collect(), x.vertices.iter().map(|v| v.uv).collect(), &x.indices, infl);
                    }
                    // B294: a rígida volta rígida, presa ao mesmo osso do esqueleto (o índice que o motor usa).
                    for x in &s.malhas_rigidas {
                        let m = &novo.malhas_rigidas[kr];
                        kr += 1;
                        rigidas += 1;
                        vertices += x.vertices.len();
                        let ok = m.osso == x.osso
                            && m.vertices.len() == x.vertices.len()
                            && m.vertices.iter().zip(&x.vertices).all(|(a, b)| a.posicao == b.posicao && a.normal == b.normal && a.uv == b.uv)
                            && m.indices == x.indices;
                        if !ok && erros.len() < 5 {
                            erros.push(format!("{} / rígida {}: osso {} × {}", p.display(), x.nome, m.osso, x.osso));
                        }
                    }
                }
                let w = novo.escrever();
                assert!(Ski::ler(&w).is_ok_and(|r| r.escrever() == w), "{}: o .ski importado não relê", p.display());
            }
        }
        eprintln!("importar: smd {n_smd} (amostra 1/5), testados {testados}, sem pele ligada {sem_pele_ligada}, malhas de pele {malhas}, rígidas {rigidas}, vértices {vertices}; erros {erros:?}");
        assert!(rigidas > 100, "a amostra tem que ter malhas rígidas: {rigidas}");
        assert!(testados > 300, "{testados}");
        assert!(erros.is_empty(), "{erros:?}");
    }
}
