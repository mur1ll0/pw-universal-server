//! GLB (glTF 2.0 binário) de um `.ski` (B272, Estúdio S5) — o formato que o visualizador do painel lê e que o
//! Blender (S6) importa.
//!
//! Uma malha glTF por malha do `.ski` (pele e rígida), com `POSITION` (com `min`/`max`, obrigatórios na
//! especificação), `NORMAL`, `TEXCOORD_0` e índices `u16`; um material por material do `.ski` (cor difusa como
//! `baseColorFactor`, `doubleSided` do `2Sided`). Sem esqueleto nem textura ainda (S5 seguinte).
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

/// O GLB do `.ski`. `nome` vai em `asset.extras` e no nó raiz.
pub fn do_ski(s: &Ski, nome: &str) -> Vec<u8> {
    let mut m = Montagem { bin: Vec::new(), views: Vec::new(), accessors: Vec::new() };
    let materiais: Vec<Value> = s
        .materiais
        .iter()
        .map(|x| {
            let c = x.difusa;
            json!({ "name": x.nome, "doubleSided": x.dois_lados,
                "pbrMetallicRoughness": { "baseColorFactor": [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.9 } })
        })
        .collect();
    let mut meshes = Vec::new();
    let mut primitiva = |m: &mut Montagem, nome: &str, pos: Vec<[f32; 3]>, nor: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, idx: &[u16], material: i32| {
        let p = m.floats(&pos);
        let n = m.floats(&nor);
        let t = m.floats2(&uv);
        let i = m.indices(idx);
        let mut prim = json!({ "attributes": { "POSITION": p, "NORMAL": n, "TEXCOORD_0": t }, "indices": i, "mode": 4 });
        if material >= 0 && (material as usize) < materiais.len() {
            prim["material"] = json!(material);
        }
        meshes.push(json!({ "name": nome, "primitives": [prim] }));
    };
    for x in &s.malhas_de_pele {
        let pos = x.vertices.iter().map(|v| espelhar(v.posicao)).collect();
        let nor = x.vertices.iter().map(|v| espelhar(v.normal)).collect();
        let uv = x.vertices.iter().map(|v| v.uv).collect();
        primitiva(&mut m, &x.nome, pos, nor, uv, &x.indices, x.material);
    }
    for x in &s.malhas_rigidas {
        let pos = x.vertices.iter().map(|v| espelhar(v.posicao)).collect();
        let nor = x.vertices.iter().map(|v| espelhar(v.normal)).collect();
        let uv = x.vertices.iter().map(|v| v.uv).collect();
        primitiva(&mut m, &x.nome, pos, nor, uv, &x.indices, x.material);
    }
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
