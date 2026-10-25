//! Texturas do cliente para RGBA (B273, Estúdio S5): DDS (DXT1, DXT3, DXT5 e RGB(A) sem compressão por máscaras) e
//! TGA sem compressão — os formatos medidos nas texturas dos `.ski` do 1.5.5 BR (`ski::testes::medir_texturas`:
//! DXT1, DXT3, DXT5, 32 bits e um TGA). Só o primeiro nível de mip, que é o que se mostra.
//!
//! Formatos públicos: DDS (`DDS_HEADER` de 124 bytes depois de `"DDS "`, `DDS_PIXELFORMAT` em +76: tamanho, flags,
//! FourCC, bits por pixel e as máscaras R, G, B, A) e S3TC/BCn — bloco 4×4 com duas cores RGB565 e índices de 2 bits
//! (DXT1; com `c0 <= c1` o índice 3 é transparente), alfa explícito de 4 bits (DXT3) ou interpolado de 8 valores
//! (DXT5). TGA tipo 2 (RGB sem compressão), 24 ou 32 bits, origem embaixo salvo o bit 5 do descritor.

#[derive(Clone)]
pub struct Imagem {
    pub largura: u32,
    pub altura: u32,
    /// RGBA, linha a linha de cima para baixo.
    pub rgba: Vec<u8>,
}

impl Imagem {
    /// Se algum pixel tem alfa abaixo de 255.
    pub fn tem_transparencia(&self) -> bool {
        self.rgba.chunks_exact(4).any(|p| p[3] < 255)
    }
}

fn u32_em(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 31) as u32;
    let g = ((c >> 5) & 63) as u32;
    let b = (c & 31) as u32;
    [(r * 255 / 31) as u8, (g * 255 / 63) as u8, (b * 255 / 31) as u8]
}

/// As 4 cores do bloco de cor (DXT1 com `dxt1`: `c0 <= c1` vira 3 cores + transparente).
fn cores(bloco: &[u8], dxt1: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([bloco[0], bloco[1]]);
    let c1 = u16::from_le_bytes([bloco[2], bloco[3]]);
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mistura = |p: u32, q: u32, d: u32| -> [u8; 4] {
        [((a[0] as u32 * p + b[0] as u32 * q) / d) as u8, ((a[1] as u32 * p + b[1] as u32 * q) / d) as u8, ((a[2] as u32 * p + b[2] as u32 * q) / d) as u8, 255]
    };
    if !dxt1 || c0 > c1 {
        [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], mistura(2, 1, 3), mistura(1, 2, 3)]
    } else {
        [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], mistura(1, 1, 2), [0, 0, 0, 0]]
    }
}

fn dxt(dados: &[u8], largura: u32, altura: u32, tipo: u8) -> Option<Vec<u8>> {
    let (w, h) = (largura as usize, altura as usize);
    let tam_bloco = if tipo == 1 { 8 } else { 16 };
    let bw = w.div_ceil(4);
    let bh = h.div_ceil(4);
    if dados.len() < bw * bh * tam_bloco {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let b = &dados[(by * bw + bx) * tam_bloco..][..tam_bloco];
            let (alfa, cor) = if tipo == 1 { (None, b) } else { (Some(&b[0..8]), &b[8..16]) };
            let paleta = cores(cor, tipo == 1);
            let indices = u32::from_le_bytes(cor[4..8].try_into().unwrap());
            // Alfa do DXT5: dois extremos e 16 índices de 3 bits (48 bits).
            let alfas5 = (tipo == 5).then(|| {
                let (a0, a1) = (alfa.unwrap()[0] as u32, alfa.unwrap()[1] as u32);
                let mut t = [0u8; 8];
                t[0] = a0 as u8;
                t[1] = a1 as u8;
                for k in 2..8 {
                    t[k] = if a0 > a1 { ((a0 * (8 - k as u32) + a1 * (k as u32 - 1)) / 7) as u8 } else if k < 6 { ((a0 * (6 - k as u32) + a1 * (k as u32 - 1)) / 5) as u8 } else if k == 6 { 0 } else { 255 };
                }
                let mut bits = 0u64;
                for (i, x) in alfa.unwrap()[2..8].iter().enumerate() {
                    bits |= (*x as u64) << (8 * i);
                }
                (t, bits)
            });
            for py in 0..4 {
                for px in 0..4 {
                    let (x, y) = (bx * 4 + px, by * 4 + py);
                    if x >= w || y >= h {
                        continue;
                    }
                    let k = py * 4 + px;
                    let mut c = paleta[((indices >> (2 * k)) & 3) as usize];
                    match tipo {
                        3 => {
                            let a = alfa.unwrap();
                            let v = (u16::from_le_bytes([a[py * 2], a[py * 2 + 1]]) >> (4 * px)) & 15;
                            c[3] = (v * 17) as u8;
                        }
                        5 => {
                            let (t, bits) = alfas5.unwrap();
                            c[3] = t[((bits >> (3 * k)) & 7) as usize];
                        }
                        _ => {}
                    }
                    rgba[(y * w + x) * 4..][..4].copy_from_slice(&c);
                }
            }
        }
    }
    Some(rgba)
}

/// Um canal pela máscara (0 se a máscara é 0, salvo o alfa, que vira 255).
fn canal(p: u32, mascara: u32, alfa: bool) -> u8 {
    if mascara == 0 {
        return if alfa { 255 } else { 0 };
    }
    let desloc = mascara.trailing_zeros();
    let max = mascara >> desloc;
    (((p & mascara) >> desloc) * 255 / max) as u8
}

pub fn dds(b: &[u8]) -> Option<Imagem> {
    if b.len() < 128 || &b[0..4] != b"DDS " {
        return None;
    }
    let altura = u32_em(b, 12);
    let largura = u32_em(b, 16);
    let flags_pf = u32_em(b, 80);
    let fourcc = &b[84..88];
    let dados = &b[128..];
    let rgba = if flags_pf & 4 != 0 {
        match fourcc {
            b"DXT1" => dxt(dados, largura, altura, 1)?,
            b"DXT3" => dxt(dados, largura, altura, 3)?,
            b"DXT5" => dxt(dados, largura, altura, 5)?,
            _ => return None,
        }
    } else {
        let bits = u32_em(b, 88);
        let (mr, mg, mb, ma) = (u32_em(b, 92), u32_em(b, 96), u32_em(b, 100), if flags_pf & 1 != 0 { u32_em(b, 104) } else { 0 });
        let bpp = (bits / 8) as usize;
        if !(2..=4).contains(&bpp) || dados.len() < largura as usize * altura as usize * bpp {
            return None;
        }
        let mut v = Vec::with_capacity(largura as usize * altura as usize * 4);
        for p in dados.chunks_exact(bpp).take(largura as usize * altura as usize) {
            let mut x = [0u8; 4];
            x[..bpp].copy_from_slice(p);
            let p = u32::from_le_bytes(x);
            v.extend_from_slice(&[canal(p, mr, false), canal(p, mg, false), canal(p, mb, false), canal(p, ma, true)]);
        }
        v
    };
    Some(Imagem { largura, altura, rgba })
}

pub fn tga(b: &[u8]) -> Option<Imagem> {
    if b.len() < 18 || b[2] != 2 {
        return None;
    }
    let id = b[0] as usize;
    let largura = u16::from_le_bytes([b[12], b[13]]) as u32;
    let altura = u16::from_le_bytes([b[14], b[15]]) as u32;
    let bpp = (b[16] / 8) as usize;
    let de_cima = b[17] & 0x20 != 0;
    if !(3..=4).contains(&bpp) {
        return None;
    }
    let inicio = 18 + id + if b[1] == 1 { u16::from_le_bytes([b[5], b[6]]) as usize * (b[7] as usize / 8) } else { 0 };
    let (w, h) = (largura as usize, altura as usize);
    let dados = b.get(inicio..inicio + w * h * bpp)?;
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let origem = if de_cima { y } else { h - 1 - y };
        for x in 0..w {
            let p = &dados[(origem * w + x) * bpp..][..bpp];
            rgba[(y * w + x) * 4..][..4].copy_from_slice(&[p[2], p[1], p[0], if bpp == 4 { p[3] } else { 255 }]);
        }
    }
    Some(Imagem { largura, altura, rgba })
}

/// DDS, TGA ou (B292) PNG, pelo conteúdo.
pub fn decodificar(b: &[u8]) -> Option<Imagem> {
    dds(b).or_else(|| tga(b)).or_else(|| png(b))
}

/// B292: PNG (o que o glTF embute) → RGBA de 8 bits.
pub fn png(b: &[u8]) -> Option<Imagem> {
    if !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut d = ::png::Decoder::new(b);
    d.set_transformations(::png::Transformations::EXPAND | ::png::Transformations::STRIP_16);
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        ::png::ColorType::Rgba => px.to_vec(),
        ::png::ColorType::Rgb => px.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        ::png::ColorType::GrayscaleAlpha => px.chunks_exact(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        ::png::ColorType::Grayscale => px.iter().flat_map(|&c| [c, c, c, 255]).collect(),
        ::png::ColorType::Indexed => return None, // EXPAND já converte paleta em RGB(A)
    };
    (rgba.len() == (w * h * 4) as usize).then_some(Imagem { largura: w, altura: h, rgba })
}

/// Os níveis de mipmap (B292): o próprio, e cada um a média 2×2 do anterior (`D3DX_FILTER_BOX`), até 1×1.
fn mipmaps(img: &Imagem) -> Vec<(u32, u32, Vec<u8>)> {
    let (mut lw, mut lh) = (img.largura.max(1), img.altura.max(1));
    let niveis = 32 - lw.max(lh).leading_zeros();
    let mut r = vec![(lw, lh, img.rgba.clone())];
    for _ in 1..niveis {
        let atual = &r.last().unwrap().2;
        let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
        let mut prox = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut soma = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (sx, sy) = ((2 * x + dx).min(lw - 1), (2 * y + dy).min(lh - 1));
                        soma += atual[((sy * lw + sx) * 4 + c) as usize] as u32;
                    }
                    prox[((y * nw + x) * 4 + c) as usize] = ((soma + 2) / 4) as u8;
                }
            }
        }
        r.push((nw, nh, prox));
        lw = nw;
        lh = nh;
    }
    r
}

fn para_565(c: [f32; 3]) -> u16 {
    let q = |v: f32, m: f32| (v.clamp(0.0, 255.0) * m / 255.0).round() as u16;
    q(c[0], 31.0) << 11 | q(c[1], 63.0) << 5 | q(c[2], 31.0)
}

/// A mesma expansão do decodificador (`rgb565`), para escolher os índices contra as cores que ele vai ver.
fn de_565(c: u16) -> [f32; 3] {
    rgb565(c).map(|x| x as f32)
}

/// Bloco de cor DXT (8 bytes) de 16 pixels RGBA, sempre no modo de 4 cores (`c0 > c1`: com `c0 <= c1` o índice 3 é
/// transparente). Extremos pelo eixo principal das cores do bloco (iteração de potência na covariância).
fn bloco_de_cor(px: &[[u8; 4]; 16]) -> [u8; 8] {
    let cores: Vec<[f32; 3]> = px.iter().map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect();
    let media = [0, 1, 2].map(|k| cores.iter().map(|c| c[k]).sum::<f32>() / 16.0);
    let mut cov = [[0f32; 3]; 3];
    for c in &cores {
        let d = [c[0] - media[0], c[1] - media[1], c[2] - media[2]];
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    let mut eixo = [1.0f32, 1.0, 1.0];
    for _ in 0..8 {
        let n = [0, 1, 2].map(|i| cov[i][0] * eixo[0] + cov[i][1] * eixo[1] + cov[i][2] * eixo[2]);
        let m = n.iter().fold(0f32, |a, x| a.max(x.abs()));
        if m < 1e-6 {
            break;
        }
        eixo = n.map(|x| x / m);
    }
    let proj = |c: &[f32; 3]| (c[0] - media[0]) * eixo[0] + (c[1] - media[1]) * eixo[1] + (c[2] - media[2]) * eixo[2];
    let (mut lo, mut hi) = (cores[0], cores[0]);
    for c in &cores {
        if proj(c) < proj(&lo) {
            lo = *c;
        }
        if proj(c) > proj(&hi) {
            hi = *c;
        }
    }
    let (mut c0, mut c1) = (para_565(hi), para_565(lo));
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    if c0 == c1 {
        // Cor única: o modo de 4 cores exige c0 > c1; um dos extremos repete a cor e todos os pixels apontam para ele.
        let (a, b, idx) = if c0 > 0 { (c0, c0 - 1, 0u32) } else { (1, 0, 1u32) };
        let mut o = [0u8; 8];
        o[0..2].copy_from_slice(&a.to_le_bytes());
        o[2..4].copy_from_slice(&b.to_le_bytes());
        let ind: u32 = (0..16).fold(0, |acc, i| acc | idx << (2 * i));
        o[4..8].copy_from_slice(&ind.to_le_bytes());
        return o;
    }
    let (p0, p1) = (de_565(c0), de_565(c1));
    let paleta = [p0, p1, [0, 1, 2].map(|k| (2.0 * p0[k] + p1[k]) / 3.0), [0, 1, 2].map(|k| (p0[k] + 2.0 * p1[k]) / 3.0)];
    let mut ind = 0u32;
    for (i, c) in cores.iter().enumerate() {
        let d = |p: &[f32; 3]| (0..3).map(|k| (c[k] - p[k]).powi(2)).sum::<f32>();
        let melhor = (0..4).min_by(|&a, &b| d(&paleta[a]).total_cmp(&d(&paleta[b]))).unwrap() as u32;
        ind |= melhor << (2 * i);
    }
    let mut o = [0u8; 8];
    o[0..2].copy_from_slice(&c0.to_le_bytes());
    o[2..4].copy_from_slice(&c1.to_le_bytes());
    o[4..8].copy_from_slice(&ind.to_le_bytes());
    o
}

/// Bloco de alfa DXT5 (8 bytes): `a0 = max > a1 = min` (modo de 8 alfas) e o índice de 3 bits mais perto; alfa único
/// → `a0 = a1` e índice 0.
fn bloco_de_alfa(px: &[[u8; 4]; 16]) -> [u8; 8] {
    let a0 = px.iter().map(|p| p[3]).max().unwrap();
    let a1 = px.iter().map(|p| p[3]).min().unwrap();
    let mut o = [0u8; 8];
    o[0] = a0;
    o[1] = a1;
    if a0 == a1 {
        return o;
    }
    let paleta: Vec<f32> = (0..8u32).map(|i| match i {
        0 => a0 as f32,
        1 => a1 as f32,
        _ => ((8 - i) as f32 * a0 as f32 + (i - 1) as f32 * a1 as f32) / 7.0,
    }).collect();
    let mut bits: u64 = 0;
    for (i, p) in px.iter().enumerate() {
        let melhor = (0..8).min_by(|&a, &b| (p[3] as f32 - paleta[a]).abs().total_cmp(&(p[3] as f32 - paleta[b]).abs())).unwrap() as u64;
        bits |= melhor << (3 * i);
    }
    o[2..8].copy_from_slice(&bits.to_le_bytes()[0..6]);
    o
}

/// B293 (S6): DDS comprimido — DXT1 se a imagem é toda opaca, DXT5 se tem alfa (no cliente 1.5.5: 6.563 DXT1, 6.445
/// DXT3, 2.038 DXT5; o motor trata DXT3 e DXT5 igualmente como textura com alfa, `A3DTexture.cpp:590-591`). Cabeçalho
/// medido nos DXT do cliente: `flags` 0xA1007, tamanho linear = blocos × 8 (DXT1) ou × 16, formato 0x4 (FourCC),
/// `caps` 0x401008, mipmaps até 1×1 (como `dds_argb32`). Bloco de 4×4; nas bordas o pixel repete o último.
pub fn dds_dxt(img: &Imagem) -> Vec<u8> {
    let alfa = img.tem_transparencia();
    let (w, h) = (img.largura.max(1), img.altura.max(1));
    let por_bloco = if alfa { 16 } else { 8 };
    let niveis = mipmaps(img);
    let mut o = Vec::new();
    o.extend_from_slice(b"DDS ");
    let u = |o: &mut Vec<u8>, x: u32| o.extend_from_slice(&x.to_le_bytes());
    let linear = w.div_ceil(4) * h.div_ceil(4) * por_bloco;
    for x in [124, 0xA1007, h, w, linear, 0, niveis.len() as u32] {
        u(&mut o, x);
    }
    o.extend_from_slice(&[0u8; 44]);
    u(&mut o, 32);
    u(&mut o, 0x4);
    o.extend_from_slice(if alfa { b"DXT5" } else { b"DXT1" });
    for x in [0, 0, 0, 0, 0, 0x40_1008, 0, 0, 0, 0] {
        u(&mut o, x);
    }
    for (lw, lh, rgba) in &niveis {
        for by in 0..lh.div_ceil(4) {
            for bx in 0..lw.div_ceil(4) {
                let mut px = [[0u8; 4]; 16];
                for (i, p) in px.iter_mut().enumerate() {
                    let (x, y) = ((bx * 4 + i as u32 % 4).min(lw - 1), (by * 4 + i as u32 / 4).min(lh - 1));
                    let k = ((y * lw + x) * 4) as usize;
                    p.copy_from_slice(&rgba[k..k + 4]);
                }
                if alfa {
                    o.extend_from_slice(&bloco_de_alfa(&px));
                }
                o.extend_from_slice(&bloco_de_cor(&px));
            }
        }
    }
    o
}

/// B292 (S6): DDS sem compressão de 32 bits (A8R8G8B8) com a cadeia inteira de mipmaps — o cabeçalho que 646 DDS de 32
/// bits do cliente 1.5.5 têm (medido): `flags` 0xA1007 (CAPS, HEIGHT, WIDTH, PIXELFORMAT, MIPMAPCOUNT, LINEARSIZE), o
/// tamanho linear `w × h × 4`, formato 0x41 (RGB + alfa) com máscaras R 0xFF0000, G 0xFF00, B 0xFF, A 0xFF000000 e
/// `caps` 0x401008 (TEXTURE, MIPMAP, COMPLEX). O motor lê pelo `D3DXCreateTextureFromFileInMemoryEx` com o número de
/// mipmaps do arquivo (`A3DTexture.cpp:594`); cada nível é a média 2×2 do anterior (`D3DX_FILTER_BOX`, o filtro que o
/// motor usa ao reduzir, `:661-688`), até 1×1. Pixels em B, G, R, A.
pub fn dds_argb32(img: &Imagem) -> Vec<u8> {
    let (w, h) = (img.largura.max(1), img.altura.max(1));
    let niveis = 32 - w.max(h).leading_zeros();
    let mut o = Vec::with_capacity(128 + (w * h * 4 * 4 / 3) as usize + 64);
    o.extend_from_slice(b"DDS ");
    let u = |o: &mut Vec<u8>, x: u32| o.extend_from_slice(&x.to_le_bytes());
    for x in [124, 0xA1007, h, w, w * h * 4, 0, niveis] {
        u(&mut o, x);
    }
    o.extend_from_slice(&[0u8; 44]);
    for x in [32, 0x41, 0, 32, 0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000, 0x40_1008, 0, 0, 0, 0] {
        u(&mut o, x);
    }
    let mut atual = img.rgba.clone();
    let (mut lw, mut lh) = (w, h);
    for nivel in 0..niveis {
        o.extend(atual.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]));
        if nivel + 1 == niveis {
            break;
        }
        let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
        let mut prox = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut soma = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (sx, sy) = ((2 * x + dx).min(lw - 1), (2 * y + dy).min(lh - 1));
                        soma += atual[((sy * lw + sx) * 4 + c) as usize] as u32;
                    }
                    prox[((y * nw + x) * 4 + c) as usize] = ((soma + 2) / 4) as u8;
                }
            }
        }
        atual = prox;
        lw = nw;
        lh = nh;
    }
    o
}

/// Estúdio G-Shop: TGA tipo 2 de 32 bits (BGRA), origem embaixo (descritor 0x08: 8 bits de alfa) e o rodapé TGA 2.0
/// (`TRUEVISION-XFILE.`) — o formato das imagens das abas da Loja Gold no cliente 1.5.5 BR
/// (`surfaces\version02\系统\商城\*.tga`, medido: cabeçalho `00 00 02 … 20 08`, 18 + w·h·4 + 26 bytes).
pub fn tga_32(img: &Imagem) -> Vec<u8> {
    let (w, h) = (img.largura as usize, img.altura as usize);
    let mut o = Vec::with_capacity(18 + w * h * 4 + 26);
    o.extend_from_slice(&[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    o.extend_from_slice(&(w as u16).to_le_bytes());
    o.extend_from_slice(&(h as u16).to_le_bytes());
    o.extend_from_slice(&[32, 0x08]);
    for y in (0..h).rev() {
        for p in img.rgba[y * w * 4..(y + 1) * w * 4].chunks_exact(4) {
            o.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    o.extend_from_slice(&[0u8; 8]);
    o.extend_from_slice(b"TRUEVISION-XFILE.\0");
    o
}

#[cfg(test)]
mod testes_do_tga_32 {
    use super::*;

    /// Decodificar e gravar de novo devolve o arquivo do cliente byte a byte (todas as imagens das abas da loja do
    /// cliente BR que estiverem no disco; sem o cliente, só o caso sintético).
    #[test]
    fn tga_da_aba_volta_identico() {
        let img = Imagem { largura: 2, altura: 1, rgba: vec![1, 2, 3, 4, 5, 6, 7, 8] };
        let t = tga_32(&img);
        assert_eq!(t.len(), 18 + 8 + 26);
        assert_eq!(tga(&t).unwrap().rgba, img.rgba);
        // `系统\商城` em GBK, gravado no disco do cliente BR como Latin-1 (como o extrator deixou os nomes).
        let pasta = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\surfaces\version02\ÏµÍ³\ÉÌ³Ç");
        let mut vistos = 0;
        let mut pilha = vec![pasta.to_path_buf()];
        while let Some(d) = pilha.pop() {
            let Ok(l) = std::fs::read_dir(&d) else { continue };
            for e in l.flatten() {
                let p = e.path();
                if p.is_dir() {
                    pilha.push(p);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("tga")) && vistos < 200 {
                    let b = std::fs::read(&p).unwrap();
                    if b.len() >= 18 && b[2] == 2 && b[16] == 32 && b[17] == 0x08 && b.ends_with(b"TRUEVISION-XFILE.\0") {
                        assert!(tga_32(&tga(&b).unwrap()) == b, "{} não volta idêntico", p.display());
                        vistos += 1;
                    }
                }
            }
        }
        eprintln!("tga_32: {vistos} TGA do cliente BR conferidos");
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// B293: DXT — cor única sai exata, imagem com alfa vira DXT5 e sem alfa DXT1, com o cabeçalho medido e os
    /// mipmaps até 1×1 (6×5 → 6×5, 3×2, 1×1).
    #[test]
    fn dds_dxt_sintetico() {
        let liso = Imagem { largura: 6, altura: 5, rgba: [200u8, 100, 50, 255].repeat(30) };
        let d = dds_dxt(&liso);
        let u = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        assert_eq!((u(&d, 8), u(&d, 20), u(&d, 28), u(&d, 80), &d[84..88], u(&d, 108)), (0xA1007, 2 * 2 * 8, 3, 0x4, b"DXT1".as_slice(), 0x40_1008));
        assert_eq!(d.len(), 128 + (4 + 1 + 1) * 8);
        let v = dds(&d).unwrap();
        let p0 = de_565(para_565([200.0, 100.0, 50.0])).map(|x| x as u8);
        assert!(v.rgba.chunks_exact(4).all(|p| p == [p0[0], p0[1], p0[2], 255]), "cor única exata (no 565)");
        let mut com_alfa = liso.clone();
        com_alfa.rgba[3] = 0;
        let d5 = dds_dxt(&com_alfa);
        assert_eq!((&d5[84..88], u(&d5, 20)), (b"DXT5".as_slice(), 2 * 2 * 16));
        let v5 = dds(&d5).unwrap();
        assert_eq!((v5.rgba[3], v5.rgba[7]), (0, 255), "alfa 0 e 255 exatos");
    }

    /// B293: a qualidade em texturas reais — cada DXT do cliente (amostra) decodificado, recodificado por `dds_dxt` e
    /// decodificado de novo; o erro médio quadrático por canal contra a imagem original decodificada.
    #[test]
    fn dds_dxt_em_texturas_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut n, mut vistos, mut soma_q, mut amostras, mut pior) = (0usize, 0usize, 0f64, 0f64, 0f64);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dds")) { continue; }
                vistos += 1;
                if vistos % 40 != 0 { continue; }
                let b = std::fs::read(&p).unwrap();
                if b.len() < 128 || !b[84..88].starts_with(b"DXT") { continue; }
                let Some(orig) = dds(&b) else { continue };
                let de_novo = dds(&dds_dxt(&orig)).unwrap();
                assert_eq!((de_novo.largura, de_novo.altura), (orig.largura, orig.altura));
                let q: f64 = orig.rgba.iter().zip(&de_novo.rgba).map(|(a, b)| (*a as f64 - *b as f64).powi(2)).sum();
                let rmse = (q / orig.rgba.len() as f64).sqrt();
                pior = pior.max(rmse);
                soma_q += q;
                amostras += orig.rgba.len() as f64;
                n += 1;
            }
        }
        let rmse = (soma_q / amostras).sqrt();
        eprintln!("dxt: {n} texturas reais (1/40 dos DDS), erro médio quadrático por canal {rmse:.2} (pior textura {pior:.2})");
        assert!(n > 100 && rmse < 4.0, "{n} {rmse}");
    }

    /// B292: o DDS de 32 bits gerado relê com os mesmos pixels, tem o cabeçalho medido e a cadeia de mipmaps inteira
    /// (5×3 → 3 níveis: 5×3, 2×1, 1×1); o PNG decodifica.
    #[test]
    fn dds_argb32_e_png() {
        let rgba: Vec<u8> = (0..5 * 3 * 4).map(|i| (i * 7 % 256) as u8).collect();
        let img = Imagem { largura: 5, altura: 3, rgba: rgba.clone() };
        let d = dds_argb32(&img);
        let u = |o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        assert_eq!((u(8), u(20), u(28), u(80), u(108)), (0xA1007, 5 * 3 * 4, 3, 0x41, 0x40_1008));
        assert_eq!(d.len(), 128 + (15 + 2 + 1) * 4);
        let de_volta = dds(&d).unwrap();
        assert_eq!((de_volta.largura, de_volta.altura, de_volta.rgba), (5, 3, rgba.clone()));
        let mut p = Vec::new();
        {
            let mut e = ::png::Encoder::new(&mut p, 5, 3);
            e.set_color(::png::ColorType::Rgba);
            e.set_depth(::png::BitDepth::Eight);
            e.write_header().unwrap().write_image_data(&rgba).unwrap();
        }
        assert_eq!(decodificar(&p).unwrap().rgba, rgba);
    }

    fn cabecalho(w: u32, h: u32, flags: u32, fourcc: &[u8; 4], bits: u32, mascaras: [u32; 4]) -> Vec<u8> {
        let mut b = vec![0u8; 128];
        b[0..4].copy_from_slice(b"DDS ");
        b[12..16].copy_from_slice(&h.to_le_bytes());
        b[16..20].copy_from_slice(&w.to_le_bytes());
        b[80..84].copy_from_slice(&flags.to_le_bytes());
        b[84..88].copy_from_slice(fourcc);
        b[88..92].copy_from_slice(&bits.to_le_bytes());
        for (i, m) in mascaras.iter().enumerate() {
            b[92 + 4 * i..96 + 4 * i].copy_from_slice(&m.to_le_bytes());
        }
        b
    }

    /// Blocos conhecidos: DXT1 com vermelho puro e azul puro nos extremos; DXT1 com o índice 3 transparente; DXT3 com
    /// alfa explícito; DXT5 com alfa interpolado; 32 bits A8R8G8B8 por máscara; TGA 24 bits de baixo para cima.
    #[test]
    fn blocos_conhecidos() {
        // DXT1: c0 = vermelho (F800), c1 = azul (001F), c0 > c1; índices: pixel 0 → 0, pixel 1 → 1, pixel 2 → 2.
        let mut b = cabecalho(4, 4, 4, b"DXT1", 0, [0; 4]);
        b.extend_from_slice(&[0x00, 0xF8, 0x1F, 0x00, 0b10_01_00, 0, 0, 0]);
        let i = dds(&b).unwrap();
        assert_eq!(&i.rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&i.rgba[4..8], &[0, 0, 255, 255]);
        assert_eq!(&i.rgba[8..12], &[170, 0, 85, 255]);
        // DXT1 com c0 <= c1: índice 3 transparente.
        let mut b = cabecalho(4, 4, 4, b"DXT1", 0, [0; 4]);
        b.extend_from_slice(&[0x1F, 0x00, 0x00, 0xF8, 0b11, 0, 0, 0]);
        assert_eq!(dds(&b).unwrap().rgba[3], 0);
        // DXT3: alfa do pixel 0 = 0x8 → 136.
        let mut b = cabecalho(4, 4, 4, b"DXT3", 0, [0; 4]);
        b.extend_from_slice(&[0x08, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0]);
        assert_eq!(dds(&b).unwrap().rgba[3], 136);
        // DXT5: a0 = 255, a1 = 0, índice do pixel 0 = 1 → 0; pixel 1 = 2 → (6·255 + 0)/7 = 218.
        let mut b = cabecalho(4, 4, 4, b"DXT5", 0, [0; 4]);
        b.extend_from_slice(&[255, 0, 0b010_001, 0, 0, 0, 0, 0, 0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0]);
        let i = dds(&b).unwrap();
        assert_eq!((i.rgba[3], i.rgba[7]), (0, 218));
        // 32 bits A8R8G8B8: bytes B, G, R, A.
        let mut b = cabecalho(1, 1, 0x41, b"\0\0\0\0", 32, [0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000]);
        b.extend_from_slice(&[10, 20, 30, 40]);
        assert_eq!(dds(&b).unwrap().rgba, vec![30, 20, 10, 40]);
        // TGA 24 bits, 1×2, de baixo para cima: a primeira linha do arquivo é a de baixo.
        let mut t = vec![0u8; 18];
        t[2] = 2;
        t[12] = 1;
        t[14] = 2;
        t[16] = 24;
        t.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(tga(&t).unwrap().rgba, vec![6, 5, 4, 255, 3, 2, 1, 255]);
    }
}
