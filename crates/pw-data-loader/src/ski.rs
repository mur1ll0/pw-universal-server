//! Leitor do `.ski` — a pele (malhas, texturas, materiais) de um modelo do Angelica (B272, Estúdio S5).
//!
//! Porte de `A3DSkin::Load` (`Angelica3D/Source/A3DSkin.cpp:201-500`, cliente 1.5.5): cabeçalho
//! `A3DSKINFILEHEADER` (`:66-82`, 104 bytes: identidade `'ASKI'`, versão ≤ 9, as contagens, `fMinWeight` e 52
//! reservados); os nomes dos ossos da pele (só versão ≥ 9); os nomes das texturas; os materiais; e as malhas na ordem
//! pele, rígida, morph de pele, morph rígida, "supple", "muscle".
//!
//! - String: `i32` tamanho + bytes (`AFile::ReadString(AString&)`, `AngelicaFile/Source/AFile.cpp:194-220`).
//! - Material (`A3DMaterial::Load`, `A3DMaterial.cpp:181-200`, binário): string terminada em NUL
//!   (`"MATERIAL: nome"`, `AFile::ReadString(char*)` `:151-172`), Ambient, Diffuse, Emissive, Specular (RGBA `f32`),
//!   Power `f32`, 2Sided `bool` (1 byte).
//! - Malha de pele (`A3DSkinMesh::LoadData`, `A3DSkinMesh.cpp:162-225`): nome, `SKINMESHDATA` (`A3DSkinMesh.h:83-89`:
//!   textura, material, vértices, índices), `A3DBVERTEX3` × n (`A3DVertex.h:184-191`, 48 bytes: posição, 3 pesos,
//!   índices de matriz, normal, uv) e `u16` × n.
//! - Malha rígida (`A3DRigidMesh::Load`, `A3DRigidMesh.cpp:115-170`): nome, `RIGIDMESHDATA` (`A3DRigidMesh.h:54-61`:
//!   osso, textura, material, vértices, índices), `A3DVERTEX` × n (`A3DVertex.h:57-61`, 32 bytes: posição, normal, uv)
//!   e `u16` × n.
//!
//! Morph, supple e muscle ainda não são lidos: o arquivo que os tem é recusado com o motivo (medido no teste quantos
//! são). O carregador do cliente é o juiz e o arquivo tem que fechar no último byte.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum SkiError {
    #[error("arquivo curto: precisava de {0} bytes em {1}")]
    Curto(usize, usize),
    #[error("não é .ski (identidade {0:#x})")]
    Identidade(u32),
    #[error("versão {0} maior que 9 (SKINFILE_VERSION)")]
    Versao(u32),
    #[error("contagem absurda: {0} = {1}")]
    Contagem(&'static str, i32),
    #[error("malhas ainda não lidas: {0}")]
    NaoSuportado(String),
    #[error("sobraram {0} bytes depois da última malha")]
    Sobra(usize),
}

pub type Result<T> = std::result::Result<T, SkiError>;

const IDENTIDADE: u32 = (b'A' as u32) << 24 | (b'S' as u32) << 16 | (b'K' as u32) << 8 | b'I' as u32;
const VERSAO_MAXIMA: u32 = 9;

#[derive(Debug, Clone, Serialize)]
pub struct Material {
    pub nome: String,
    pub ambiente: [f32; 4],
    pub difusa: [f32; 4],
    pub emissiva: [f32; 4],
    pub especular: [f32; 4],
    pub brilho: f32,
    pub dois_lados: bool,
    /// B286: o byte do `2Sided` como está (para reescrever igual) e a linha inteira (`"MATERIAL: nome"`).
    #[serde(skip)]
    pub dois_lados_byte: u8,
    #[serde(skip)]
    pub linha: String,
}

/// Um vértice com pesos (`A3DBVERTEX3`). `ossos` = os 4 bytes de `dwMatIndices`; o 4º peso é `1 - soma dos 3`.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct VerticeComPeso {
    pub posicao: [f32; 3],
    pub pesos: [f32; 3],
    pub ossos: [u8; 4],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Vertice {
    pub posicao: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct MalhaDePele {
    pub nome: String,
    pub textura: i32,
    pub material: i32,
    pub vertices: Vec<VerticeComPeso>,
    pub indices: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MalhaRigida {
    pub nome: String,
    pub osso: i32,
    pub textura: i32,
    pub material: i32,
    pub vertices: Vec<Vertice>,
    pub indices: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ski {
    pub versao: u32,
    pub peso_minimo: f32,
    pub ossos_do_esqueleto: i32,
    pub ossos: Vec<String>,
    pub texturas: Vec<String>,
    pub materiais: Vec<Material>,
    pub malhas_de_pele: Vec<MalhaDePele>,
    pub malhas_rigidas: Vec<MalhaRigida>,
    /// B286 (escrita): a marca `MOXB` do `AFile`, a contagem de ossos do cabeçalho (gravada mesmo na versão < 9, que
    /// não traz os nomes) e os 52 bytes reservados de `A3DSKINFILEHEADER`.
    #[serde(skip)]
    pub marca: bool,
    #[serde(skip)]
    pub ossos_no_cabecalho: i32,
    #[serde(skip)]
    pub reservado: Vec<u8>,
}

struct Leitor<'a> {
    b: &'a [u8],
    o: usize,
}

impl<'a> Leitor<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let f = self.b.get(self.o..self.o + n).ok_or(SkiError::Curto(n, self.o))?;
        self.o += n;
        Ok(f)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn f32s<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut r = [0.0; N];
        for x in &mut r {
            *x = self.f32()?;
        }
        Ok(r)
    }
    fn contagem(&mut self, campo: &'static str, limite: i32) -> Result<usize> {
        let n = self.i32()?;
        if !(0..=limite).contains(&n) {
            return Err(SkiError::Contagem(campo, n));
        }
        Ok(n as usize)
    }
    /// `AFile::ReadString(AString&)`: `i32` + bytes (GBK; aqui com perda para exibição).
    fn texto(&mut self) -> Result<String> {
        let n = self.contagem("tamanho de texto", 1 << 16)?;
        Ok(crate::ski::gbk(self.bytes(n)?))
    }
    /// `AFile::ReadString(char*)`: até o NUL.
    fn texto_nul(&mut self) -> Result<String> {
        let resto = &self.b[self.o..];
        let n = resto.iter().position(|&c| c == 0).ok_or(SkiError::Curto(1, self.b.len()))?;
        let s = gbk(&resto[..n]);
        self.o += n + 1;
        Ok(s)
    }
}

pub(crate) fn gbk(b: &[u8]) -> String {
    encoding_rs::GBK.decode(b).0.into_owned()
}

impl Ski {
    pub fn ler(b: &[u8]) -> Result<Self> {
        // `AFile::Open` (`AngelicaFile/Source/AFile.cpp:86-98`) consome a marca do arquivo: `MOXB` (0x42584f4d) =
        // binário, `MOXT` = texto; sem marca, volta ao começo.
        let inicio = if b.starts_with(b"MOXB") { 4 } else { 0 };
        let mut l = Leitor { b, o: inicio };
        let identidade = l.u32()?;
        if identidade != IDENTIDADE {
            return Err(SkiError::Identidade(identidade));
        }
        let versao = l.u32()?;
        if versao > VERSAO_MAXIMA {
            return Err(SkiError::Versao(versao));
        }
        let n_pele = l.contagem("malhas de pele", 4096)?;
        let n_rigida = l.contagem("malhas rígidas", 4096)?;
        let n_morph_pele = l.contagem("malhas morph de pele", 4096)?;
        let n_morph_rigida = l.contagem("malhas morph rígidas", 4096)?;
        let n_texturas = l.contagem("texturas", 4096)?;
        let n_materiais = l.contagem("materiais", 4096)?;
        let n_ossos = l.contagem("ossos da pele", 4096)?;
        let peso_minimo = l.f32()?;
        let ossos_do_esqueleto = l.i32()?;
        let n_supple = l.contagem("malhas supple", 4096)?;
        let n_muscle = l.contagem("malhas muscle", 4096)?;
        let reservado = l.bytes(52)?.to_vec();
        let mut faltam = Vec::new();
        for (n, nome) in [(n_morph_pele, "morph de pele"), (n_morph_rigida, "morph rígida"), (n_supple, "supple"), (n_muscle, "muscle")] {
            if n > 0 {
                faltam.push(format!("{n} {nome}"));
            }
        }
        if !faltam.is_empty() {
            return Err(SkiError::NaoSuportado(faltam.join(", ")));
        }
        let ossos = if versao >= 9 { (0..n_ossos).map(|_| l.texto()).collect::<Result<_>>()? } else { Vec::new() };
        let texturas = (0..n_texturas).map(|_| l.texto()).collect::<Result<_>>()?;
        let mut materiais = Vec::with_capacity(n_materiais);
        for _ in 0..n_materiais {
            let linha = l.texto_nul()?;
            let nome = linha.strip_prefix("MATERIAL: ").unwrap_or(&linha).to_string();
            let (ambiente, difusa, emissiva, especular, brilho) = (l.f32s()?, l.f32s()?, l.f32s()?, l.f32s()?, l.f32()?);
            let byte = l.bytes(1)?[0];
            materiais.push(Material { nome, ambiente, difusa, emissiva, especular, brilho, dois_lados: byte != 0, dois_lados_byte: byte, linha });
        }
        let mut malhas_de_pele = Vec::with_capacity(n_pele);
        for _ in 0..n_pele {
            let nome = l.texto()?;
            let textura = l.i32()?;
            let material = l.i32()?;
            let nv = l.contagem("vértices", 1 << 20)?;
            let ni = l.contagem("índices", 1 << 22)?;
            let mut vertices = Vec::with_capacity(nv);
            for _ in 0..nv {
                let posicao = l.f32s()?;
                let pesos = l.f32s()?;
                let ossos: [u8; 4] = l.bytes(4)?.try_into().unwrap();
                vertices.push(VerticeComPeso { posicao, pesos, ossos, normal: l.f32s()?, uv: l.f32s()? });
            }
            let indices = l.bytes(2 * ni)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            malhas_de_pele.push(MalhaDePele { nome, textura, material, vertices, indices });
        }
        let mut malhas_rigidas = Vec::with_capacity(n_rigida);
        for _ in 0..n_rigida {
            let nome = l.texto()?;
            let osso = l.i32()?;
            let textura = l.i32()?;
            let material = l.i32()?;
            let nv = l.contagem("vértices", 1 << 20)?;
            let ni = l.contagem("índices", 1 << 22)?;
            let mut vertices = Vec::with_capacity(nv);
            for _ in 0..nv {
                vertices.push(Vertice { posicao: l.f32s()?, normal: l.f32s()?, uv: l.f32s()? });
            }
            let indices = l.bytes(2 * ni)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            malhas_rigidas.push(MalhaRigida { nome, osso, textura, material, vertices, indices });
        }
        if l.o != b.len() {
            return Err(SkiError::Sobra(b.len() - l.o));
        }
        Ok(Self {
            versao, peso_minimo, ossos_do_esqueleto, ossos, texturas, materiais, malhas_de_pele, malhas_rigidas,
            marca: inicio == 4, ossos_no_cabecalho: n_ossos as i32, reservado,
        })
    }

    /// B286 (S6): o inverso de `ler` — `A3DSkin::Save` (`A3DSkin.cpp:523-604`) na mesma ordem: cabeçalho de 104 bytes, nomes dos
    /// ossos (v ≥ 9), texturas, materiais (linha terminada em NUL + 4 RGBA + brilho + 2Sided), malhas de pele e rígidas.
    /// Textos em GBK. Sem morph/supple/muscle (o leitor recusa quem os tem). Para um `.ski` lido, sai byte a byte igual
    /// (`testes::escrever_reproduz_o_cliente`).
    pub fn escrever(&self) -> Vec<u8> {
        let mut o = Vec::new();
        let u = |o: &mut Vec<u8>, x: u32| o.extend_from_slice(&x.to_le_bytes());
        let f = |o: &mut Vec<u8>, x: &[f32]| x.iter().for_each(|v| o.extend_from_slice(&v.to_le_bytes()));
        let texto = |o: &mut Vec<u8>, t: &str| {
            let b = encoding_rs::GBK.encode(t).0;
            o.extend_from_slice(&(b.len() as u32).to_le_bytes());
            o.extend_from_slice(&b);
        };
        if self.marca {
            o.extend_from_slice(b"MOXB");
        }
        u(&mut o, IDENTIDADE);
        u(&mut o, self.versao);
        let n_ossos = if self.versao >= 9 { self.ossos.len() as u32 } else { self.ossos_no_cabecalho as u32 };
        for n in [self.malhas_de_pele.len() as u32, self.malhas_rigidas.len() as u32, 0, 0, self.texturas.len() as u32, self.materiais.len() as u32, n_ossos] {
            u(&mut o, n);
        }
        f(&mut o, &[self.peso_minimo]);
        u(&mut o, self.ossos_do_esqueleto as u32);
        u(&mut o, 0);
        u(&mut o, 0);
        let mut res = self.reservado.clone();
        res.resize(52, 0);
        o.extend_from_slice(&res);
        if self.versao >= 9 {
            self.ossos.iter().for_each(|t| texto(&mut o, t));
        }
        self.texturas.iter().for_each(|t| texto(&mut o, t));
        for m in &self.materiais {
            let linha = if m.linha.is_empty() { format!("MATERIAL: {}", m.nome) } else { m.linha.clone() };
            o.extend_from_slice(&encoding_rs::GBK.encode(&linha).0);
            o.push(0);
            f(&mut o, &m.ambiente);
            f(&mut o, &m.difusa);
            f(&mut o, &m.emissiva);
            f(&mut o, &m.especular);
            f(&mut o, &[m.brilho]);
            o.push(if m.linha.is_empty() { m.dois_lados as u8 } else { m.dois_lados_byte });
        }
        for m in &self.malhas_de_pele {
            texto(&mut o, &m.nome);
            for x in [m.textura as u32, m.material as u32, m.vertices.len() as u32, m.indices.len() as u32] {
                u(&mut o, x);
            }
            for v in &m.vertices {
                f(&mut o, &v.posicao);
                f(&mut o, &v.pesos);
                o.extend_from_slice(&v.ossos);
                f(&mut o, &v.normal);
                f(&mut o, &v.uv);
            }
            m.indices.iter().for_each(|i| o.extend_from_slice(&i.to_le_bytes()));
        }
        for m in &self.malhas_rigidas {
            texto(&mut o, &m.nome);
            for x in [m.osso as u32, m.textura as u32, m.material as u32, m.vertices.len() as u32, m.indices.len() as u32] {
                u(&mut o, x);
            }
            for v in &m.vertices {
                f(&mut o, &v.posicao);
                f(&mut o, &v.normal);
                f(&mut o, &v.uv);
            }
            m.indices.iter().for_each(|i| o.extend_from_slice(&i.to_le_bytes()));
        }
        o
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// B286: todo `.ski` que o leitor aceita, reescrito, sai byte a byte igual ao do cliente.
    #[test]
    fn escrever_reproduz_o_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut lidos, mut iguais, mut recusados) = (0usize, 0usize, 0usize);
        let mut diferentes: Vec<String> = Vec::new();
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ski")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let Ok(s) = Ski::ler(&b) else { recusados += 1; continue };
                lidos += 1;
                let w = s.escrever();
                if w == b {
                    iguais += 1;
                } else if diferentes.len() < 5 {
                    let o = w.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(w.len().min(b.len()));
                    diferentes.push(format!("{} (tam {} × {}, 1º byte diferente em {o})", p.display(), w.len(), b.len()));
                }
            }
        }
        eprintln!("ski: lidos {lidos}, reescritos iguais {iguais}, recusados pelo leitor {recusados}; diferentes: {diferentes:?}");
        assert!(lidos > 20000, "{lidos}");
        assert_eq!(iguais, lidos, "{diferentes:?}");
    }

    /// Medida (B273): as texturas dos `.ski` — extensão, onde o cliente as acha (`A3DSkin.cpp`: primeiro
    /// `<pasta>\Textures\<nome>`, depois `<pasta>\Tex_<título>\<nome>`), e o formato dos DDS (FourCC ou bits).
    #[test]
    fn medir_texturas() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let mut ext = std::collections::BTreeMap::<String, usize>::new();
        let mut onde = std::collections::BTreeMap::<&str, usize>::new();
        let mut formato = std::collections::BTreeMap::<String, usize>::new();
        let mut decodificadas = std::collections::BTreeMap::<bool, usize>::new();
        let mut n = 0;
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ski")) { continue; }
                n += 1;
                if n % 7 != 0 { continue; } // amostra de 1/7
                let Ok(s) = Ski::ler(&std::fs::read(&p).unwrap()) else { continue };
                let pasta = p.parent().unwrap();
                let titulo = p.file_stem().unwrap().to_string_lossy().to_string();
                for t in &s.texturas {
                    // Os nomes desta instalação desempacotada estão em disco como os bytes GBK lidos em Windows-1252.
                    let t = &encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(t).0).0.into_owned();
                    *ext.entry(t.rsplit('.').next().unwrap_or("").to_lowercase()).or_default() += 1;
                    let a = pasta.join("Textures").join(t);
                    let b = pasta.join(format!("Tex_{titulo}")).join(t);
                    let achado = if a.exists() { Some((a, "Textures")) } else if b.exists() { Some((b, "Tex_")) } else { None };
                    *onde.entry(achado.as_ref().map_or("não achada", |x| x.1)).or_default() += 1;
                    if let Some((f, _)) = achado {
                        let h = std::fs::read(&f).unwrap_or_default();
                        let k = if h.len() >= 128 && &h[0..4] == b"DDS " {
                            let fourcc = &h[84..88];
                            if fourcc.iter().all(|c| c.is_ascii_alphanumeric()) { String::from_utf8_lossy(fourcc).to_string() }
                            else { format!("bits{}", u32::from_le_bytes(h[88..92].try_into().unwrap())) }
                        } else { format!("não-DDS {:?}", h.get(0..4).map(|x| String::from_utf8_lossy(x).to_string())) };
                        *formato.entry(k).or_default() += 1;
                        let dec = crate::imagem::decodificar(&h).map(|i| i.rgba.len() == 4 * (i.largura * i.altura) as usize);
                        *decodificadas.entry(dec == Some(true)).or_default() += 1;
                    }
                }
            }
        }
        eprintln!("ski {n} (amostra 1/7); extensões {ext:?}; onde {onde:?}; formatos {formato:?}; decodificadas {decodificadas:?}");
        assert_eq!(decodificadas.get(&false).copied().unwrap_or(0), 0, "toda textura achada decodifica");
    }

    /// Todos os `.ski` do cliente 1.5.5 BR (desempacotado, fora do repositório): quantos fecham no último byte, e o
    /// motivo dos que não fecham. Sem a pasta, avisa.
    #[test]
    fn todos_os_ski_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut ok, mut motivos) = (0usize, std::collections::BTreeMap::<String, (usize, String)>::new());
        let mut versoes = std::collections::BTreeMap::new();
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    pilha.push(p);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ski")) {
                    let b = std::fs::read(&p).unwrap();
                    match Ski::ler(&b) {
                        Ok(s) => {
                            ok += 1;
                            *versoes.entry(s.versao).or_insert(0) += 1;
                        }
                        Err(e) => {
                            let chave = match &e {
                                SkiError::NaoSuportado(_) => "não suportado".to_string(),
                                outro => format!("{outro}").split(':').next().unwrap().to_string(),
                            };
                            let x = motivos.entry(chave).or_insert((0, String::new()));
                            x.0 += 1;
                            if x.1.is_empty() {
                                x.1 = format!("{} — {e}", p.display());
                            }
                        }
                    }
                }
            }
        }
        eprintln!("ski: {ok} fecham; versões {versoes:?}; falhas {motivos:#?}");
        assert!(ok > 0);
        // Medido no 1.5.5 BR: 24.165 de 24.166 fecham (v8: 6.033, v9: 18.132); nenhum tem morph/supple/muscle. O que
        // sobra é `npcs\怪物\灵族\巨牛\牛.ski`, 36 bytes sem `ASKI` — o cliente o recusa na mesma checagem
        // (`A3DSkin.cpp:214`). Só pode falhar o que o cliente também recusa (identidade) ou o que ainda não é lido.
        assert!(motivos.keys().all(|k| k == "não suportado" || k.starts_with("não é .ski")), "{motivos:#?}");
        assert!(ok >= 24_165, "{ok}");
    }
}
