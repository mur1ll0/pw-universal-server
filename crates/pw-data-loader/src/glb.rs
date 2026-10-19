//! GLB (glTF 2.0 binário) de um `.ski` (B272, Estúdio S5) — o formato que o visualizador do painel lê e que o
//! Blender (S6) importa.
//!
//! Uma malha glTF por malha do `.ski` (pele e rígida), com `POSITION` (com `min`/`max`, obrigatórios na
//! especificação), `NORMAL`, `TEXCOORD_0` e índices `u16`; um material glTF por par (material, textura) usado pelas malhas (cor difusa como
//! `baseColorFactor`, `doubleSided` do `2Sided` e, desde o B273, a textura em PNG embutida, que quem chama entrega
//! já convertida; com alfa abaixo de 255 em algum pixel, `alphaMode` MASK — escolha de exibição). Sem esqueleto ainda.
//!
//! Eixos: o Angelica é Direct3D, mão esquerda, Y para cima; o glTF é mão direita, Y para cima. Converte-se negando
//! `z` (posição e normal) e invertendo a ordem dos triângulos. A origem de UV dos dois é o canto superior esquerdo.

use crate::bon::{Bon, Matriz};
use crate::ski::Ski;
use serde_json::{json, Value};

struct Montagem {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Montagem {
    fn alinhar(&mut self) {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
    }

    fn view(&mut self, bytes: &[u8], alvo: u32) -> usize {
        self.alinhar();
        let o = self.bin.len();
        self.bin.extend_from_slice(bytes);
        self.views.push(json!({ "buffer": 0, "byteOffset": o, "byteLength": bytes.len(), "target": alvo }));
        if alvo == 0 {
            self.views.last_mut().unwrap().as_object_mut().unwrap().remove("target");
        }
        self.views.len() - 1
    }

    fn floats(&mut self, v: &[[f32; 3]]) -> usize {
        let bytes: Vec<u8> = v.iter().flat_map(|p| p.iter().flat_map(|x| x.to_le_bytes())).collect();
        let view = self.view(&bytes, 34962);
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for p in v {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        if v.is_empty() {
            min = [0.0; 3];
            max = [0.0; 3];
        }
        self.accessors.push(json!({ "bufferView": view, "componentType": 5126, "count": v.len(), "type": "VEC3", "min": min, "max": max }));
        self.accessors.len() - 1
    }

    fn floats2(&mut self, v: &[[f32; 2]]) -> usize {
        let bytes: Vec<u8> = v.iter().flat_map(|p| p.iter().flat_map(|x| x.to_le_bytes())).collect();
        let view = self.view(&bytes, 34962);
        self.accessors.push(json!({ "bufferView": view, "componentType": 5126, "count": v.len(), "type": "VEC2" }));
        self.accessors.len() - 1
    }

    fn indices(&mut self, i: &[u16]) -> usize {
        // Triângulos com a ordem invertida (mão esquerda → direita).
        let mut t = Vec::with_capacity(i.len());
        for c in i.chunks_exact(3) {
            t.extend_from_slice(&[c[0], c[2], c[1]]);
        }
        let bytes: Vec<u8> = t.iter().flat_map(|x| x.to_le_bytes()).collect();
        let view = self.view(&bytes, 34963);
        self.accessors.push(json!({ "bufferView": view, "componentType": 5123, "count": t.len(), "type": "SCALAR" }));
        self.accessors.len() - 1
    }
}

fn espelhar(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[1], -p[2]]
}

/// Uma textura já convertida para o navegador: PNG e se tem transparência.
pub struct Textura {
    pub png: Vec<u8>,
    pub transparente: bool,
}

/// Uma pele do modelo e o fornecedor das texturas dela (`textura(i)`: a i-ésima do `.ski`; `None` = não achada).
pub struct PeleDoModelo<'a> {
    pub ski: &'a Ski,
    pub textura: &'a dyn Fn(usize) -> Option<Textura>,
}

/// O GLB do `.ski` sem texturas. `nome` vai em `asset.extras` e no nó raiz.
pub fn do_ski(s: &Ski, nome: &str) -> Vec<u8> {
    do_ski_com_texturas(s, nome, &|_| None)
}

/// O GLB do `.ski` sozinho (sem esqueleto).
pub fn do_ski_com_texturas(s: &Ski, nome: &str, textura: &dyn Fn(usize) -> Option<Textura>) -> Vec<u8> {
    do_modelo(&[PeleDoModelo { ski: s, textura }], None, nome)
}

/// Matriz do Direct3D (linha × matriz, em ordem de linha) → glTF (coluna, em ordem de coluna) com a troca de mão: o
/// arranjo de memória do D3D já é o do glTF transposto; a troca de mão é `S·M·S`, `S = diag(1, 1, −1, 1)`.
fn matriz_gltf(m: &Matriz) -> [f32; 16] {
    let s = [1.0f32, 1.0, -1.0, 1.0];
    let mut r = [0.0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            r[i * 4 + j] = m[i * 4 + j] * s[i] * s[j];
        }
    }
    r
}

/// Inversa de uma 4×4 (cofatores).
pub fn inversa(m: &Matriz) -> Option<Matriz> {
    let a = |r: usize, c: usize| m[r * 4 + c] as f64;
    let mut inv = [0.0f64; 16];
    for r in 0..4 {
        for c in 0..4 {
            let (rs, cs): (Vec<usize>, Vec<usize>) = ((0..4).filter(|&x| x != c).collect(), (0..4).filter(|&x| x != r).collect());
            let d = a(rs[0], cs[0]) * (a(rs[1], cs[1]) * a(rs[2], cs[2]) - a(rs[1], cs[2]) * a(rs[2], cs[1]))
                - a(rs[0], cs[1]) * (a(rs[1], cs[0]) * a(rs[2], cs[2]) - a(rs[1], cs[2]) * a(rs[2], cs[0]))
                + a(rs[0], cs[2]) * (a(rs[1], cs[0]) * a(rs[2], cs[1]) - a(rs[1], cs[1]) * a(rs[2], cs[0]));
            inv[r * 4 + c] = if (r + c) % 2 == 0 { d } else { -d };
        }
    }
    let det: f64 = (0..4).map(|c| a(0, c) * inv[c * 4]).sum();
    if det.abs() < 1e-12 {
        return None;
    }
    let mut r = [0.0f32; 16];
    for k in 0..16 {
        r[k] = (inv[k] / det) as f32;
    }
    Some(r)
}

/// O GLB de um modelo: as peles e, se houver, o esqueleto (B274). Com esqueleto, cada osso vira um nó (mais um nó
/// raiz "esqueleto", o índice `n` que `A3DSkin::BindSkeleton` reserva para o próprio esqueleto, com matriz
/// identidade), a `skin` usa `matBoneInit` como inversa de ligação (`A3DSkinModel.cpp:1341-1342`: a matriz da pele é
/// `BoneInitTM × UpToRoot`), e cada nó tem como transformação local a pose de ligação (inversa do `matBoneInit`)
/// relativa à da mãe — assim o modelo em repouso fica exatamente na posição dos vértices. Os vértices levam
/// `JOINTS_0`/`WEIGHTS_0`: ossos da pele mapeados como `BindSkeleton` (`A3DSkin.cpp:830-865`: v < 9, o índice é o do
/// esqueleto; v ≥ 9, pelo nome) e o 4º peso `1 − soma`; a malha rígida fica presa ao osso dela com peso 1.
/// Pele que não casa com o esqueleto (osso inexistente ou nome sem par) vai sem `skin`.
pub fn do_modelo(peles: &[PeleDoModelo], esqueleto: Option<&Bon>, nome: &str) -> Vec<u8> {
    let mut m = Montagem { bin: Vec::new(), views: Vec::new(), accessors: Vec::new() };
    let mut images = Vec::new();
    let mut materiais: Vec<Value> = Vec::new();
    let mut meshes: Vec<Value> = Vec::new();
    let mut nos: Vec<Value> = Vec::new();
    let mut raizes: Vec<usize> = Vec::new();
    // Esqueleto: nós de osso (0..n) e o nó do esqueleto (n).
    let n_ossos = esqueleto.map_or(0, |b| b.ossos.len());
    let mut skin = None;
    if let Some(b) = esqueleto {
        let mundo: Vec<Matriz> = b.ossos.iter().map(|o| inversa(&o.inicial).unwrap_or(o.relativa)).collect();
        let base = nos.len();
        for (i, o) in b.ossos.iter().enumerate() {
            let local = if o.mae >= 0 && (o.mae as usize) < n_ossos {
                inversa(&mundo[o.mae as usize]).map_or(mundo[i], |pi| crate::bon::multiplicar(&mundo[i], &pi))
            } else {
                mundo[i]
            };
            let filhas: Vec<usize> = b.ossos.iter().enumerate().filter(|(_, x)| x.mae == i as i32).map(|(k, _)| base + k).collect();
            let mut no = json!({ "name": o.nome, "matrix": matriz_gltf(&local) });
            if !filhas.is_empty() {
                no["children"] = json!(filhas);
            }
            nos.push(no);
        }
        let raiz_esq = nos.len();
        let filhas_raiz: Vec<usize> = b.ossos.iter().enumerate().filter(|(_, x)| x.mae < 0 || x.mae as usize >= n_ossos).map(|(k, _)| base + k).collect();
        nos.push(json!({ "name": "esqueleto", "children": filhas_raiz }));
        raizes.push(raiz_esq);
        let mut ibm: Vec<u8> = Vec::new();
        for o in &b.ossos {
            ibm.extend(matriz_gltf(&o.inicial).iter().flat_map(|x| x.to_le_bytes()));
        }
        ibm.extend([1.0f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.].iter().flat_map(|x| x.to_le_bytes()));
        let v = m.view(&ibm, 0);
        m.accessors.push(json!({ "bufferView": v, "componentType": 5126, "count": n_ossos + 1, "type": "MAT4" }));
        let mut joints: Vec<usize> = (base..base + n_ossos).collect();
        joints.push(raiz_esq);
        skin = Some(json!({ "inverseBindMatrices": m.accessors.len() - 1, "joints": joints, "skeleton": raiz_esq }));
    }
    for pele in peles {
        let s = pele.ski;
        // Mapa osso da pele → junta (índice na lista `joints`: osso do esqueleto, ou `n_ossos` = o esqueleto).
        let mapa: Option<Vec<u16>> = esqueleto.and_then(|b| {
            if s.versao >= 9 {
                let mut v: Vec<u16> = Vec::with_capacity(s.ossos.len() + 1);
                for n in &s.ossos {
                    v.push(b.ossos.iter().position(|o| &o.nome == n)? as u16);
                }
                v.push(n_ossos as u16);
                Some(v)
            } else {
                let n = s.ossos_do_esqueleto.max(0) as usize;
                if n > n_ossos {
                    return None;
                }
                let mut v: Vec<u16> = (0..n as u16).collect();
                v.push(n_ossos as u16);
                Some(v)
            }
        });
        let junta = |i: u8| -> Option<u16> { mapa.as_ref().and_then(|v| v.get(i as usize).copied()) };
        // Imagens desta pele.
        let mut indice_da_imagem = vec![None; s.texturas.len()];
        let mut transparente = vec![false; s.texturas.len()];
        for (i, nome_t) in s.texturas.iter().enumerate() {
            if let Some(t) = (pele.textura)(i) {
                let v = m.view(&t.png, 0);
                images.push(json!({ "bufferView": v, "mimeType": "image/png", "name": nome_t }));
                indice_da_imagem[i] = Some(images.len() - 1);
                transparente[i] = t.transparente;
            }
        }
        let mut pares: Vec<((i32, i32), usize)> = Vec::new();
        let mut material_de = |materiais: &mut Vec<Value>, mat: i32, tex: i32| -> Option<usize> {
            if let Some(k) = pares.iter().find(|x| x.0 == (mat, tex)) {
                return Some(k.1);
            }
            let base = s.materiais.get(mat.max(0) as usize).filter(|_| mat >= 0);
            let img = (tex >= 0).then(|| indice_da_imagem.get(tex as usize).copied().flatten()).flatten();
            if base.is_none() && img.is_none() {
                return None;
            }
            let c = base.map_or([1.0; 4], |x| x.difusa);
            let mut pbr = json!({ "baseColorFactor": [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.9 });
            let mut mat_json = json!({ "name": base.map_or(String::new(), |x| x.nome.clone()), "doubleSided": base.is_some_and(|x| x.dois_lados) });
            if let Some(im) = img {
                pbr["baseColorTexture"] = json!({ "index": im });
                // Com textura, a cor vem dela (o difuso do material do Angelica costuma ser branco ou cinza).
                pbr["baseColorFactor"] = json!([1.0, 1.0, 1.0, 1.0]);
                if transparente[tex as usize] {
                    mat_json["alphaMode"] = json!("MASK");
                    mat_json["alphaCutoff"] = json!(0.5);
                }
            }
            mat_json["pbrMetallicRoughness"] = pbr;
            materiais.push(mat_json);
            pares.push(((mat, tex), materiais.len() - 1));
            Some(materiais.len() - 1)
        };
        let mut primitiva = |m: &mut Montagem, nome: &str, pos: Vec<[f32; 3]>, nor: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, idx: &[u16], material: i32, tex: i32, pesos: Option<(Vec<[u16; 4]>, Vec<[f32; 4]>)>| {
            let p = m.floats(&pos);
            let n = m.floats(&nor);
            let t = m.floats2(&uv);
            let i = m.indices(idx);
            let mut prim = json!({ "attributes": { "POSITION": p, "NORMAL": n, "TEXCOORD_0": t }, "indices": i, "mode": 4 });
            let tem_pesos = pesos.is_some();
            if let Some((j, w)) = pesos {
                let bj: Vec<u8> = j.iter().flat_map(|x| x.iter().flat_map(|v| v.to_le_bytes())).collect();
                let vj = m.view(&bj, 34962);
                m.accessors.push(json!({ "bufferView": vj, "componentType": 5123, "count": j.len(), "type": "VEC4" }));
                prim["attributes"]["JOINTS_0"] = json!(m.accessors.len() - 1);
                let bw: Vec<u8> = w.iter().flat_map(|x| x.iter().flat_map(|v| v.to_le_bytes())).collect();
                let vw = m.view(&bw, 34962);
                m.accessors.push(json!({ "bufferView": vw, "componentType": 5126, "count": w.len(), "type": "VEC4" }));
                prim["attributes"]["WEIGHTS_0"] = json!(m.accessors.len() - 1);
            }
            if let Some(k) = material_de(&mut materiais, material, tex) {
                prim["material"] = json!(k);
            }
            meshes.push(json!({ "name": nome, "primitives": [prim] }));
            let mut no = json!({ "mesh": meshes.len() - 1, "name": nome });
            if tem_pesos {
                no["skin"] = json!(0);
            }
            nos.push(no);
            raizes.push(nos.len() - 1);
        };
        for x in &s.malhas_de_pele {
            let pos = x.vertices.iter().map(|v| espelhar(v.posicao)).collect();
            let nor = x.vertices.iter().map(|v| espelhar(v.normal)).collect();
            let uv = x.vertices.iter().map(|v| v.uv).collect();
            let pesos = mapa.as_ref().and_then(|_| {
                let mut j = Vec::with_capacity(x.vertices.len());
                let mut w = Vec::with_capacity(x.vertices.len());
                for v in &x.vertices {
                    let w4 = [v.pesos[0], v.pesos[1], v.pesos[2], (1.0 - v.pesos[0] - v.pesos[1] - v.pesos[2]).max(0.0)];
                    let mut jj = [0u16; 4];
                    for k in 0..4 {
                        jj[k] = if w4[k] > 0.0 { junta(v.ossos[k])? } else { 0 };
                    }
                    let soma: f32 = w4.iter().sum();
                    j.push(jj);
                    w.push(if soma > 0.0 { w4.map(|x| x / soma) } else { [1.0, 0.0, 0.0, 0.0] });
                }
                Some((j, w))
            });
            primitiva(&mut m, &x.nome, pos, nor, uv, &x.indices, x.material, x.textura, pesos);
        }
        for x in &s.malhas_rigidas {
            let pos = x.vertices.iter().map(|v| espelhar(v.posicao)).collect();
            let nor = x.vertices.iter().map(|v| espelhar(v.normal)).collect();
            let uv = x.vertices.iter().map(|v| v.uv).collect();
            let pesos = (x.osso >= 0).then(|| junta(x.osso as u8)).flatten().map(|j| (vec![[j, 0, 0, 0]; x.vertices.len()], vec![[1.0, 0.0, 0.0, 0.0]; x.vertices.len()]));
            primitiva(&mut m, &x.nome, pos, nor, uv, &x.indices, x.material, x.textura, pesos);
        }
    }
    m.alinhar();
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "pw-universal-server (Estúdio S5)", "extras": { "origem": nome } },
        "scene": 0,
        "scenes": [{ "name": nome, "nodes": raizes }],
        "nodes": nos,
        "meshes": meshes,
        "buffers": [{ "byteLength": m.bin.len() }],
        "bufferViews": m.views,
        "accessors": m.accessors,
    });
    if let Some(s) = skin {
        doc["skins"] = json!([s]);
    }
    if !materiais.is_empty() {
        doc["materials"] = json!(materiais);
    }
    if !images.is_empty() {
        doc["textures"] = json!((0..images.len()).map(|i| json!({ "source": i, "sampler": 0 })).collect::<Vec<_>>());
        // Repetir (10497) nos dois eixos, filtro linear — o padrão do D3D9 para as peles.
        doc["samplers"] = json!([{ "wrapS": 10497, "wrapT": 10497, "magFilter": 9729, "minFilter": 9729 }]);
        doc["images"] = json!(images);
    }
    let mut texto = serde_json::to_vec(&doc).unwrap();
    while texto.len() % 4 != 0 {
        texto.push(b' ');
    }
    let total = 12 + 8 + texto.len() + 8 + m.bin.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(texto.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"JSON");
    glb.extend_from_slice(&texto);
    glb.extend_from_slice(&(m.bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"BIN\0");
    glb.extend_from_slice(&m.bin);
    glb
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Modelos reais (amostra de 1/10 dos `.smd` do 1.5.5 BR com `.bon` v6): o GLB com esqueleto tem juntas válidas,
    /// pesos que somam 1 e, em toda junta, (mundo pelos nós) × (inversa de ligação) ≈ identidade — o modelo em repouso
    /// fica na pose de ligação. Mede quantas peles casam com o esqueleto.
    #[test]
    fn modelos_com_esqueleto() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let disco = |s: &str| encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(s).0).0.into_owned();
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut n, mut modelos, mut peles, mut casadas, mut juntas, mut ruins) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("smd")) { continue; }
                n += 1;
                if n % 10 != 0 { continue; }
                let Ok(c) = crate::smd::cabeca(&std::fs::read(&p).unwrap()) else { continue };
                let pasta = p.parent().unwrap();
                let Ok(b) = std::fs::read(pasta.join(disco(&c.esqueleto))).map_err(|_| ()).and_then(|x| crate::bon::Bon::ler(&x).map_err(|_| ())) else { continue };
                let skis: Vec<Ski> = c.peles.iter().filter_map(|s| Ski::ler(&std::fs::read(pasta.join(disco(s))).ok()?).ok()).collect();
                if skis.is_empty() { continue; }
                modelos += 1;
                let sem = |_: usize| None;
                let lista: Vec<PeleDoModelo> = skis.iter().map(|s| PeleDoModelo { ski: s, textura: &sem }).collect();
                let g = do_modelo(&lista, Some(&b), "t");
                let nj = u32::from_le_bytes(g[12..16].try_into().unwrap()) as usize;
                let doc: Value = serde_json::from_slice(&g[20..20 + nj]).unwrap();
                let bin = &g[28 + nj..];
                let f32s = |acc: usize| -> Vec<f32> {
                    let a = &doc["accessors"][acc];
                    let v = &doc["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
                    let o = v["byteOffset"].as_u64().unwrap() as usize;
                    let k = a["count"].as_u64().unwrap() as usize * match a["type"].as_str().unwrap() { "MAT4" => 16, "VEC4" => 4, _ => 1 };
                    (0..k).map(|i| f32::from_le_bytes(bin[o + 4 * i..o + 4 * i + 4].try_into().unwrap())).collect()
                };
                // Mundo de cada nó pelos `matrix` (coluna; mundo = mãe · local).
                let nos = doc["nodes"].as_array().unwrap();
                let mut mae = vec![None; nos.len()];
                for (i, x) in nos.iter().enumerate() {
                    for c in x["children"].as_array().into_iter().flatten() { mae[c.as_u64().unwrap() as usize] = Some(i); }
                }
                let local = |i: usize| -> [f32; 16] { nos[i]["matrix"].as_array().map_or([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.], |a| core::array::from_fn(|k| a[k].as_f64().unwrap() as f32)) };
                // Coluna em ordem de coluna: (A·B)[c*4+r] = Σ A[k*4+r]·B[c*4+k].
                let mul = |a: &[f32; 16], b: &[f32; 16]| -> [f32; 16] { core::array::from_fn(|i| { let (c, r) = (i / 4, i % 4); (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum() }) };
                let mundo = |mut i: usize| -> [f32; 16] { let mut w = local(i); while let Some(p) = mae[i] { w = mul(&local(p), &w); i = p; } w };
                if let Some(sk) = doc["skins"].as_array().and_then(|x| x.first()) {
                    let ibm = f32s(sk["inverseBindMatrices"].as_u64().unwrap() as usize);
                    for (k, j) in sk["joints"].as_array().unwrap().iter().enumerate() {
                        juntas += 1;
                        let w = mundo(j.as_u64().unwrap() as usize);
                        let ib: [f32; 16] = core::array::from_fn(|q| ibm[16 * k + q]);
                        let id = mul(&w, &ib);
                        let escala = w.iter().map(|x| x.abs()).fold(1.0f32, f32::max);
                        if (0..16).any(|q| (id[q] - if q % 5 == 0 { 1.0 } else { 0.0 }).abs() > 1e-3 * escala) { ruins += 1; }
                    }
                }
                for me in doc["meshes"].as_array().unwrap() {
                    peles += 1;
                    let at = &me["primitives"][0]["attributes"];
                    if let Some(wacc) = at["WEIGHTS_0"].as_u64() {
                        casadas += 1;
                        let w = f32s(wacc as usize);
                        assert!(w.chunks(4).all(|x| (x.iter().sum::<f32>() - 1.0).abs() < 1e-3), "pesos");
                    }
                }
            }
        }
        eprintln!("modelos {modelos}; malhas {peles}, com pesos {casadas}; juntas {juntas}, fora da pose de ligação {ruins}");
        assert!(modelos > 0 && ruins == 0, "{ruins}");
    }

    /// Um `.ski` real vira um GLB coerente: cabeçalho, tamanho total, os dois blocos, e cada accessor dentro do buffer.
    #[test]
    fn glb_de_um_ski_real() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models\npcs");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let mut achado = None;
        while let (None, Some(d)) = (&achado, pilha.pop()) {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    pilha.push(p);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ski")) {
                    if let Ok(s) = Ski::ler(&std::fs::read(&p).unwrap()) {
                        if !s.malhas_de_pele.is_empty() {
                            achado = Some(s);
                            break;
                        }
                    }
                }
            }
        }
        let s = achado.expect("algum .ski com malha de pele");
        let g = do_ski(&s, "teste");
        assert_eq!(&g[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes(g[8..12].try_into().unwrap()) as usize, g.len());
        let nj = u32::from_le_bytes(g[12..16].try_into().unwrap()) as usize;
        let doc: Value = serde_json::from_slice(&g[20..20 + nj]).unwrap();
        let nb = u32::from_le_bytes(g[20 + nj..24 + nj].try_into().unwrap()) as usize;
        assert_eq!(&g[24 + nj..28 + nj], b"BIN\0");
        assert_eq!(doc["buffers"][0]["byteLength"].as_u64().unwrap() as usize, nb);
        for v in doc["bufferViews"].as_array().unwrap() {
            assert!(v["byteOffset"].as_u64().unwrap() + v["byteLength"].as_u64().unwrap() <= nb as u64);
        }
        assert_eq!(doc["meshes"].as_array().unwrap().len(), s.malhas_de_pele.len() + s.malhas_rigidas.len());
    }
}
