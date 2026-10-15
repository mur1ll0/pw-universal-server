//! Mapas do Estúdio, fase S4 (B242): a lista de mapas, as peças do minimapa e as criaturas do `npcgen`.
//!
//! - **Lista:** `Configs\instance.txt` do cliente (`EC_GameRun.cpp:336`, `CECInstance::Load`,
//!   `EC_Instance.cpp:60-110`): `"nome" { id caminho linhas, colunas … }` — o 1.5.5 traz campos a mais
//!   (limite de pulo, rotas, `{}` da M-imagem), pulados até o `}` que fecha a entrada.
//! - **Peças:** `Surfaces\MiniMaps\<caminho>\LLCC.dds` (`DlgMiniMap.cpp:1155-1168`, `"%02d%02d"` = linha e
//!   coluna), `linhas×2` por `colunas×2` peças de 256 px; cada célula de 1024 m vira 512 px (2 m por pixel) e
//!   a origem fica no centro: `px = (x + colunas·512)/2`, `py = (linhas·512 − z)/2` (`DlgMiniMap.cpp:1045-1061`).
//! - **Criaturas:** o `npcgen.data` do realm (`config/<caminho>/npcgen.data`; sem `world/npcgen.data`, o da raiz — a ordem do GS, `manager.rs:616-621`).

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Mapa {
    pub nome: String,
    pub id: i32,
    pub caminho: String,
    pub linhas: u32,
    pub colunas: u32,
}

/// Fichas do `AWScriptFile`: palavras separadas por espaço/vírgula, `"…"` como uma ficha, `{`/`}` soltas,
/// `//` até o fim da linha.
fn fichas(texto: &str) -> Vec<String> {
    // O texto inteiro de uma vez: o `instance.txt` do BR tem nome com quebra de linha dentro das aspas
    // ("Terra dos Imortais\r\n"), e o cliente o aceita. `//` só vale fora de aspas.
    let mut saida = Vec::new();
    let mut chars = texto.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
        } else if c == '/' {
            chars.next();
            if chars.peek() == Some(&'/') {
                for x in chars.by_ref() {
                    if x == '\n' {
                        break;
                    }
                }
            }
        } else if c == '"' {
            chars.next();
            let s: String = chars.by_ref().take_while(|&x| x != '"').collect();
            saida.push(s.trim().to_string());
        } else if c == '{' || c == '}' {
            chars.next();
            saida.push(c.to_string());
        } else {
            let mut s = String::new();
            while let Some(&x) = chars.peek() {
                if x.is_whitespace() || x == ',' || x == '{' || x == '}' || x == '"' || x == '/' {
                    break;
                }
                s.push(x);
                chars.next();
            }
            saida.push(s);
        }
    }
    saida
}

/// Texto do `instance.txt`: UTF-16LE com BOM (o 1.5.5) ou GBK.
pub fn texto_do_arquivo(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[..2] == [0xFF, 0xFE] {
        let u: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else {
        crate::nomes::legivel(bytes)
    }
}

pub fn ler_instancias(texto: &str) -> Result<Vec<Mapa>> {
    let f = fichas(texto);
    let mut mapas = Vec::new();
    let mut i = 0;
    while i < f.len() {
        let nome = f[i].clone();
        if f.get(i + 1).map(String::as_str) != Some("{") {
            bail!("instance.txt: esperava '{{' depois de {nome:?}");
        }
        let num = |k: usize| -> Result<i64> {
            f.get(k).and_then(|s| s.parse::<i64>().ok()).with_context(|| format!("instance.txt: número em {nome:?}"))
        };
        let id = num(i + 2)? as i32;
        let caminho = f.get(i + 3).cloned().context("caminho")?;
        let linhas = num(i + 4)? as u32;
        let colunas = num(i + 5)? as u32;
        // pula o resto até o `}` que fecha esta entrada (há `{}` aninhados no 1.5.5)
        let mut j = i + 6;
        let mut prof = 1;
        while j < f.len() && prof > 0 {
            match f[j].as_str() {
                "{" => prof += 1,
                "}" => prof -= 1,
                _ => {}
            }
            j += 1;
        }
        if prof != 0 {
            bail!("instance.txt: entrada {nome:?} sem '}}'");
        }
        mapas.push(Mapa { nome, id, caminho, linhas, colunas });
        i = j;
    }
    Ok(mapas)
}

// ------------------------------------------------------------------ precinct.clt do cliente (B244)

/// Um distrito do `Maps\<caminho>\precinct.clt` do cliente: só o que o servidor não tem (nome, id, marcas).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DistritoDoCliente {
    pub nome: String,
    pub id: u32,
    pub vertices: usize,
    pub marcas: Vec<(String, [f32; 3])>,
}

/// `CELPrecinctSet::Load` / `CELPrecinct::Load(AWScriptFile*)` do cliente (`EL_Precinct.cpp:96-213`, `:397-421`):
/// `version N [carimbo v≥5]` e, por distrito: nome, id (v≥2), pontos, marcas, prioridade, mapa do ponto, músicas,
/// intervalo, laço, mapa (v≥4), domínio (v≥6), PK (v≥7), ponto de cidade, vértices, marcas (nome + xyz), som,
/// músicas, som da noite (v≥3).
pub fn ler_precinct_clt(texto: &str) -> Result<Vec<DistritoDoCliente>> {
    let f = fichas(texto);
    if f.first().map(String::as_str) != Some("version") {
        bail!("precinct.clt sem 'version'");
    }
    let num = |k: usize| -> Result<f64> { f.get(k).and_then(|s| s.parse::<f64>().ok()).with_context(|| format!("precinct.clt: número na ficha {k}")) };
    let v = num(1)? as u32;
    let mut i = if v >= 5 { 3 } else { 2 };
    let mut saida = Vec::new();
    while i < f.len() {
        let nome = f[i].clone();
        i += 1;
        let id = if v >= 2 { let x = num(i)? as u32; i += 1; x } else { 0 };
        let (pontos, marcas, musicas) = (num(i)? as usize, num(i + 1)? as usize, num(i + 4)? as usize);
        i += 7 + (v >= 4) as usize + (v >= 6) as usize + (v >= 7) as usize;
        i += 3 + 3 * pontos;
        let mut m = Vec::with_capacity(marcas);
        for _ in 0..marcas {
            let n = f.get(i).cloned().context("precinct.clt: marca")?;
            m.push((n, [num(i + 1)? as f32, num(i + 2)? as f32, num(i + 3)? as f32]));
            i += 4;
        }
        i += 1 + musicas + (v >= 3) as usize;
        if i > f.len() {
            bail!("precinct.clt: distrito {nome:?} truncado");
        }
        saida.push(DistritoDoCliente { nome, id, vertices: pontos, marcas: m });
    }
    Ok(saida)
}

// ------------------------------------------------------------------ DDS (DXT1/3/5) → PNG

fn rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 31) as u32;
    let g = ((c >> 5) & 63) as u32;
    let b = (c & 31) as u32;
    [(r * 255 / 31) as u8, (g * 255 / 63) as u8, (b * 255 / 31) as u8]
}

fn cores_do_bloco(b: &[u8], sempre_quatro: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let (a, z) = (rgb565(c0), rgb565(c1));
    let mistura = |p: u32, q: u32, d: u32| -> [u8; 4] {
        [
            ((a[0] as u32 * p + z[0] as u32 * q) / d) as u8,
            ((a[1] as u32 * p + z[1] as u32 * q) / d) as u8,
            ((a[2] as u32 * p + z[2] as u32 * q) / d) as u8,
            255,
        ]
    };
    if c0 > c1 || sempre_quatro {
        [[a[0], a[1], a[2], 255], [z[0], z[1], z[2], 255], mistura(2, 1, 3), mistura(1, 2, 3)]
    } else {
        [[a[0], a[1], a[2], 255], [z[0], z[1], z[2], 255], mistura(1, 1, 2), [0, 0, 0, 0]]
    }
}

/// Decodifica um `.dds` DXT1/DXT3/DXT5 (só o primeiro nível) em RGBA.
pub fn dds_para_rgba(dds: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    if dds.len() < 128 || &dds[..4] != b"DDS " {
        bail!("não é DDS");
    }
    let altura = u32::from_le_bytes(dds[12..16].try_into().unwrap());
    let largura = u32::from_le_bytes(dds[16..20].try_into().unwrap());
    let formato = &dds[84..88];
    let tam_bloco = match formato {
        b"DXT1" => 8,
        b"DXT3" | b"DXT5" => 16,
        _ => bail!("formato DDS não suportado: {:?}", String::from_utf8_lossy(formato)),
    };
    let (bx, by) = (largura.div_ceil(4) as usize, altura.div_ceil(4) as usize);
    if dds.len() < 128 + bx * by * tam_bloco || largura == 0 || altura == 0 || largura > 4096 || altura > 4096 {
        bail!("DDS truncado ou grande demais");
    }
    let mut px = vec![0u8; (largura * altura * 4) as usize];
    for y in 0..by {
        for x in 0..bx {
            let o = 128 + (y * bx + x) * tam_bloco;
            let bloco = &dds[o..o + tam_bloco];
            let (alfa, cor) = if tam_bloco == 16 { (Some(&bloco[..8]), &bloco[8..]) } else { (None, bloco) };
            let cores = cores_do_bloco(cor, tam_bloco == 16);
            let indices = u32::from_le_bytes(cor[4..8].try_into().unwrap());
            for k in 0..16 {
                let (cx, cy) = (x * 4 + k % 4, y * 4 + k / 4);
                if cx >= largura as usize || cy >= altura as usize {
                    continue;
                }
                let mut c = cores[((indices >> (2 * k)) & 3) as usize];
                if let Some(a) = alfa {
                    if formato == b"DXT3" {
                        let nib = (u64::from_le_bytes(a.try_into().unwrap()) >> (4 * k)) & 15;
                        c[3] = (nib * 17) as u8;
                    } else {
                        let (a0, a1) = (a[0] as u32, a[1] as u32);
                        let bits = u64::from_le_bytes([a[2], a[3], a[4], a[5], a[6], a[7], 0, 0]);
                        let ia = ((bits >> (3 * k)) & 7) as u32;
                        c[3] = match ia {
                            0 => a0,
                            1 => a1,
                            i if a0 > a1 => ((8 - i) * a0 + (i - 1) * a1) / 7,
                            6 => 0,
                            7 => 255,
                            i => ((6 - i) * a0 + (i - 1) * a1) / 5,
                        } as u8;
                    }
                }
                let p = (cy * largura as usize + cx) * 4;
                px[p..p + 4].copy_from_slice(&c);
            }
        }
    }
    Ok((largura, altura, px))
}

pub fn png(largura: u32, altura: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let mut saida = Vec::new();
    {
        let mut e = ::png::Encoder::new(&mut saida, largura, altura);
        e.set_color(::png::ColorType::Rgba);
        e.set_depth(::png::BitDepth::Eight);
        let mut w = e.write_header()?;
        w.write_image_data(rgba)?;
    }
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn instance_txt_do_126_e_do_155() {
        let t126 = "\"world\"\n{\n1\t\t//  ID\n\"world\"\t\t//  path\n11, 8\t\t//  row, column\n}\n\"鬼城\"\n{\n101\n\"a01\"\n1, 1\n}\n";
        let m = ler_instancias(t126).unwrap();
        assert_eq!(m, vec![
            Mapa { nome: "world".into(), id: 1, caminho: "world".into(), linhas: 11, colunas: 8 },
            Mapa { nome: "鬼城".into(), id: 101, caminho: "a01".into(), linhas: 1, colunas: 1 },
        ]);
        let t155 = "\"Perfect World\"\n{\n1\n\"world\"\n11, 8\n0\n\"r1.cfg\",\"r2.cfg\"\t// rotas\n{}\t\t// M图\n}\n\"x\"\n{\n2\n\"a02\"\n2, 3\n}\n";
        let m = ler_instancias(t155).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].id, m[0].linhas, m[0].colunas, m[1].caminho.as_str()), (1, 11, 8, "a02"));
        // Nome com quebra de linha dentro das aspas (instance.txt do 1.5.5 BR) e rota vazia "".
        let br = "\"Terra dos Imortais\r\n\"\r\n{\r\n116\t\t//  ID\r\n\"a16\"\r\n1, 1\r\n1\r\n\"\"\r\n{}\r\n}\r\n";
        let m = ler_instancias(br).unwrap();
        assert_eq!((m[0].nome.as_str(), m[0].id, m[0].caminho.as_str()), ("Terra dos Imortais", 116, "a16"));
    }

    #[test]
    fn precinct_clt_do_155_br() {
        // Trecho do `maps\world\precinct.clt` do cliente 1.5.5 BR (versão 7), com uma marca inventada para o teste.
        let t = "//  Element pricinct file (client version)

version  7
1442234257

\"Fechado\"
1561  5  1  0  1  1  60  1  1  0  0
-696.291992, 218.964996, -1178.810059
-4099.773438  0.100000  5636.250977
-4099.694824  0.100000  4604.409180
-3068.147705  0.100000  4604.448242
-3066.735352  0.100000  5637.446777
-4099.891113  0.100000  5636.623535
\"Porto\" 1.0 2.0 3.0
\"\"
\"Music\\通用场景2.mp3\"
\"\"

\"Cidade\"
7  3  0  2  1  0  60  1  1  0  1
1, 2, 3
0 0 0
1 0 0
1 0 1
\"\"
\"\"
";
        let d = ler_precinct_clt(t).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!((d[0].nome.as_str(), d[0].id, d[0].vertices), ("Fechado", 1561, 5));
        assert_eq!(d[0].marcas, vec![("Porto".to_string(), [1.0, 2.0, 3.0])]);
        assert_eq!((d[1].nome.as_str(), d[1].id, d[1].vertices), ("Cidade", 7, 3));
    }

    #[test]
    fn dxt1_de_um_bloco() {
        let mut dds = vec![0u8; 128];
        dds[..4].copy_from_slice(b"DDS ");
        dds[12..16].copy_from_slice(&4u32.to_le_bytes());
        dds[16..20].copy_from_slice(&4u32.to_le_bytes());
        dds[84..88].copy_from_slice(b"DXT1");
        // c0 = vermelho puro (0xF800), c1 = azul (0x001F), índices 0 em tudo
        dds.extend([0x00, 0xF8, 0x1F, 0x00, 0, 0, 0, 0]);
        let (l, a, px) = dds_para_rgba(&dds).unwrap();
        assert_eq!((l, a, &px[..4]), (4, 4, &[255u8, 0, 0, 255][..]));
        assert!(png(l, a, &px).unwrap().starts_with(b"\x89PNG"));
    }
}
