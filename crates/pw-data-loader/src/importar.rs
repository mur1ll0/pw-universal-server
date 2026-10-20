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
//!   32 bits com mipmaps (B292, `imagem::dds_argb32`), com o nome em `.dds` — em `texturas_novas`.
//!
//! Primitiva sem pele, índice que não cabe em `u16` e modo diferente de triângulos são recusados ou relatados — nada
//! é inventado.

use crate::bon::Bon;
use crate::ski::{Material, MalhaDePele, Ski, VerticeComPeso};
use serde_json::Value;

pub struct Importacao {
    pub ski: Ski,
    /// B292: as texturas que o glTF traz embutidas, já como o cliente lê: (nome no `.ski`, bytes). PNG vira DDS de 32
    /// bits com mipmaps (`imagem::dds_argb32`); DDS/TGA embutidos vão como estão.
    pub texturas_novas: Vec<(String, Vec<u8>)>,
    /// O que foi ajustado ou deixado de fora, para o relatório.
    pub relatorio: Vec<String>,
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
    let mut texturas_novas: Vec<(String, Vec<u8>)> = Vec::new();
    let mut malhas = Vec::new();
    for no in &nos {
        let Some(mi) = no["mesh"].as_u64() else { continue };
        let malha = &j["meshes"][mi as usize];
        let nome_malha = malha["name"].as_str().or(no["name"].as_str()).unwrap_or("malha").to_string();
        let Some(si) = no["skin"].as_u64() else {
            relatorio.push(format!("{nome_malha}: sem pele (skin) — deixada de fora (malha rígida ainda não é importada)"));
            continue;
        };
        // Juntas da pele → índice no .ski (nomes, na ordem de 1º uso) ou a vaga do esqueleto.
        let juntas: Vec<Option<String>> = j["skins"][si as usize]["joints"]
            .as_array()
            .ok_or("skin sem joints")?
            .iter()
            .map(|x| {
                let n = nome_do_no(x.as_u64().unwrap_or(u64::MAX));
                if n == "esqueleto" { None } else { Some(n) }
            })
            .collect();
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
            let (Some(js), Some(ws)) = (pega("JOINTS_0")?, pega("WEIGHTS_0")?) else {
                relatorio.push(format!("{nome}: sem JOINTS_0/WEIGHTS_0 — deixada de fora"));
                continue;
            };
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
            let mut vertices = Vec::with_capacity(pos.len());
            for k in 0..pos.len() {
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
                                if !texturas_novas.iter().any(|x| x.0 == destino) { texturas_novas.push((destino.clone(), b)); }
                            }
                            Some(b) => match crate::imagem::png(&b) {
                                Some(im) => { if !texturas_novas.iter().any(|x| x.0 == destino) { texturas_novas.push((destino.clone(), crate::imagem::dds_argb32(&im))); } }
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
            malhas.push(MalhaDePele { nome, textura, material, vertices, indices });
        }
    }
    if malhas.is_empty() {
        return Err(format!("nenhuma malha com pele no glTF ({})", relatorio.join("; ")));
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
        malhas_rigidas: Vec::new(),
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
        let (mut n_smd, mut testados, mut sem_pele_ligada, mut malhas, mut vertices) = (0usize, 0usize, 0usize, 0usize, 0usize);
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
                if imp.ski.malhas_de_pele.len() != esperadas { sem_pele_ligada += 1; continue; }
                testados += 1;
                let novo = &imp.ski;
                let nome_novo = |i: u8| novo.ossos.get(i as usize).cloned().unwrap_or_else(|| "esqueleto".into());
                let mut k = 0;
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
                    for x in &s.malhas_rigidas {
                        let infl = vec![vec![(nome_orig(x.osso as u8), 1.0)]; x.vertices.len()];
                        comparar(&x.nome, x.vertices.iter().map(|v| v.posicao).collect(), x.vertices.iter().map(|v| v.normal).collect(), x.vertices.iter().map(|v| v.uv).collect(), &x.indices, infl);
                    }
                }
                let w = novo.escrever();
                assert!(Ski::ler(&w).is_ok_and(|r| r.escrever() == w), "{}: o .ski importado não relê", p.display());
            }
        }
        eprintln!("importar: smd {n_smd} (amostra 1/5), testados {testados}, sem pele ligada {sem_pele_ligada}, malhas {malhas}, vértices {vertices}; erros {erros:?}");
        assert!(testados > 300, "{testados}");
        assert!(erros.is_empty(), "{erros:?}");
    }
}
