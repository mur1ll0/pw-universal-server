//! Texturas do cliente para RGBA (B273, Estúdio S5): DDS (DXT1, DXT3, DXT5 e RGB(A) sem compressão por máscaras) e
//! TGA sem compressão — os formatos medidos nas texturas dos `.ski` do 1.5.5 BR (`ski::testes::medir_texturas`:
//! DXT1, DXT3, DXT5, 32 bits e um TGA). Só o primeiro nível de mip, que é o que se mostra.
//!
//! Formatos públicos: DDS (`DDS_HEADER` de 124 bytes depois de `"DDS "`, `DDS_PIXELFORMAT` em +76: tamanho, flags,
//! FourCC, bits por pixel e as máscaras R, G, B, A) e S3TC/BCn — bloco 4×4 com duas cores RGB565 e índices de 2 bits
//! (DXT1; com `c0 <= c1` o índice 3 é transparente), alfa explícito de 4 bits (DXT3) ou interpolado de 8 valores
//! (DXT5). TGA tipo 2 (RGB sem compressão), 24 ou 32 bits, origem embaixo salvo o bit 5 do descritor.

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

/// DDS ou TGA, pelo conteúdo.
pub fn decodificar(b: &[u8]) -> Option<Imagem> {
    dds(b).or_else(|| tga(b))
}

#[cfg(test)]
mod testes {
    use super::*;

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
