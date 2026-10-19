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

/// O GLB do `.ski` sem texturas. `nome` vai em `asset.extras` e no nó raiz.
pub fn do_ski(s: &Ski, nome: &str) -> Vec<u8> {
    do_ski_com_texturas(s, nome, &|_| None)
}

/// O GLB do `.ski`; `textura(i)` entrega a i-ésima textura do `.ski` (`None` = não achada: fica só a cor).
pub fn do_ski_com_texturas(s: &Ski, nome: &str, textura: &dyn Fn(usize) -> Option<Textura>) -> Vec<u8> {
    let mut m = Montagem { bin: Vec::new(), views: Vec::new(), accessors: Vec::new() };
    // Imagens: uma por textura do `.ski` que veio.
    let mut images = Vec::new();
    let mut indice_da_imagem = vec![None; s.texturas.len()];
    let mut transparente = vec![false; s.texturas.len()];
    for (i, nome_t) in s.texturas.iter().enumerate() {
        if let Some(t) = textura(i) {
            let v = m.view(&t.png, 0);
            images.push(json!({ "bufferView": v, "mimeType": "image/png", "name": nome_t }));
            indice_da_imagem[i] = Some(images.len() - 1);
            transparente[i] = t.transparente;
        }
    }
    // Materiais glTF: um por par (material, textura) usado.
    let mut pares: Vec<(i32, i32)> = Vec::new();
    let mut materiais: Vec<Value> = Vec::new();
    let mut material_de = |mat: i32, tex: i32| -> Option<usize> {
        if let Some(k) = pares.iter().position(|&x| x == (mat, tex)) {
            return Some(k);
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
        pares.push((mat, tex));
        materiais.push(mat_json);
        Some(materiais.len() - 1)
    };
    let mut meshes = Vec::new();
    let mut primitiva = |m: &mut Montagem, nome: &str, pos: Vec<[f32; 3]>, nor: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, idx: &[u16], material: i32, tex: i32| {
        let p = m.floats(&pos);
        let n = m.floats(&nor);
        let t = m.floats2(&uv);
        let i = m.indices(idx);
        let mut prim = json!({ "attributes": { "POSITION": p, "NORMAL": n, "TEXCOORD_0": t }, "indices": i, "mode": 4 });
        if let Some(k) = material_de(material, tex) {
            prim["material"] = json!(k);
        }
        meshes.push(json!({ "name": nome, "primitives": [prim] }));
    };
    for x in &s.malhas_de_pele {
        let pos = x.vertices.iter().map(|v| espelhar(v.posicao)).collect();
        let nor = x.vertices.iter().map(|v| espelhar(v.normal)).collect();
        let uv = x.vertices.iter().map(|v| v.uv).collect();
        primitiva(&mut m, &x.nome, pos, nor, uv, &x.indices, x.material, x.textura);
    }
    for x in &s.malhas_rigidas {
        let pos = x.vertices.iter().map(|v| espelhar(v.posicao)).collect();
        let nor = x.vertices.iter().map(|v| espelhar(v.normal)).collect();
        let uv = x.vertices.iter().map(|v| v.uv).collect();
        primitiva(&mut m, &x.nome, pos, nor, uv, &x.indices, x.material, x.textura);
    }
    drop(primitiva);
    let nos: Vec<Value> = (0..meshes.len()).map(|i| json!({ "mesh": i, "name": meshes[i]["name"] })).collect();
    m.alinhar();
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "pw-universal-server (Estúdio S5)", "extras": { "origem": nome } },
        "scene": 0,
        "scenes": [{ "name": nome, "nodes": (0..nos.len()).collect::<Vec<_>>() }],
        "nodes": nos,
        "meshes": meshes,
        "buffers": [{ "byteLength": m.bin.len() }],
        "bufferViews": m.views,
        "accessors": m.accessors,
    });
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
