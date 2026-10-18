//! `Maps\<mapa>\precinct.clt` e `region.clt` do **cliente** (B251): os distritos e as regiões/portais em texto.
//!
//! O cliente os lê com `AWScriptFile` (`EC_World.cpp:630-640`): distrito em `CELPrecinct::Load(AWScriptFile*)`
//! (`CCommon/EL_Precinct.cpp:96-213`), região e portal em `CELRegion::Load`/`CELTransportBox::Load`
//! (`CCommon/EL_Region.cpp:70-100`, `:254-290`). O servidor tem os mesmos dados em binário (`precinct.sev`,
//! `region.sev`, ver `crate::precinct` e `crate::regioes`): editar um sem o outro desalinha nome, música e — no portal —
//! o **índice** que o cliente manda no `ENTER_INSTANCE` (`EC_World.cpp:2360-2373`).
//!
//! # Formato (medido: os 104 pares dos clientes 1.2.6 e 1.5.5 BR voltam idênticos)
//!
//! UTF-16LE com BOM, quebra de linha LF. Cabeçalho até `version  N` e o carimbo na linha seguinte, uma linha em
//! branco, e os registros, cada um seguido de uma linha em branco:
//!
//! ```text
//! "nome"
//! id  pontos  marcas  prioridade  mapa_do_ponto  músicas  intervalo  laço  [mapa v≥4]  [domínio v≥6]  [pk v≥7]
//! cx, cy, cz
//! x  y  z                          (× pontos)
//!                                  (linha em branco)
//! "marca"  x  y  z                 (× marcas)
//!                                  (linha em branco)
//! "som"  "música"… (uma por linha)  "som da noite" (v≥3)
//! ```
//!
//! `region.clt`: `[trans]` + `destino  origem  [nível v≥5]` + `pos`, `exts`, `alvo` (`%f, %f, %f`), e
//! `[region]` + `"nome"` + `tipo  pontos` + vértices (`%f  %f  %f`).
//!
//! Números em ponto flutuante: `printf("%f")` do MSVC — expansão decimal exata do `float` e empate arredondado para
//! longe do zero (`287.8203125` → `287.820313`; o arredondamento para o par daria `287.820312` e o arquivo mudaria).

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroDoClt {
    #[error("{0}: {1}")]
    Formato(&'static str, String),
}

type R<T> = Result<T, ErroDoClt>;

/// `printf("%f")` do MSVC para um `float`.
pub fn formatar(x: f32) -> String {
    formatar_com(x, 6)
}

/// `printf("%.Nf")` do MSVC para um `float`: expansão decimal exata, empate para longe do zero.
pub fn formatar_com(x: f32, casas: usize) -> String {
    let negativo = x.is_sign_negative();
    let exato = format!("{:.60}", (x as f64).abs());
    let (inteira, frac) = exato.split_once('.').unwrap_or((&exato, ""));
    let mut digitos: Vec<u8> = inteira.bytes().chain(frac.bytes().take(casas)).map(|c| c - b'0').collect();
    if frac.as_bytes().get(casas).is_some_and(|&c| c >= b'5') {
        let mut i = digitos.len();
        loop {
            if i == 0 {
                digitos.insert(0, 1);
                break;
            }
            i -= 1;
            if digitos[i] == 9 {
                digitos[i] = 0;
            } else {
                digitos[i] += 1;
                break;
            }
        }
    }
    let n = digitos.len() - casas;
    let s: String = digitos.iter().map(|d| (d + b'0') as char).collect();
    if casas == 0 {
        return format!("{}{}", if negativo { "-" } else { "" }, s);
    }
    format!("{}{}.{}", if negativo { "-" } else { "" }, &s[..n], &s[n..])
}

pub(crate) fn texto(b: &[u8], onde: &'static str) -> R<String> {
    let corpo = b.strip_prefix(&[0xFF, 0xFE]).ok_or_else(|| ErroDoClt::Formato(onde, "sem BOM UTF-16LE".into()))?;
    if corpo.len() % 2 != 0 {
        return Err(ErroDoClt::Formato(onde, "UTF-16 com byte sobrando".into()));
    }
    let u: Vec<u16> = corpo.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16(&u).map_err(|_| ErroDoClt::Formato(onde, "UTF-16 inválido".into()))
}

pub(crate) fn bytes(t: &str) -> Vec<u8> {
    let mut b = vec![0xFF, 0xFE];
    for u in t.encode_utf16() {
        b.extend_from_slice(&u.to_le_bytes());
    }
    b
}

/// Fichas de uma linha: `"…"` inteira (com as aspas) ou palavras separadas por espaço/vírgula.
pub(crate) fn fichas(l: &str) -> Vec<&str> {
    let mut saida = Vec::new();
    let b = l.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b',' | b'\r' => i += 1,
            b'"' => {
                let fim = l[i + 1..].find('"').map(|k| i + 1 + k + 1).unwrap_or(l.len());
                saida.push(&l[i..fim]);
                i = fim;
            }
            _ => {
                let fim = l[i..].find([' ', '\t', ',']).map(|k| i + k).unwrap_or(l.len());
                saida.push(&l[i..fim]);
                i = fim;
            }
        }
    }
    saida
}

pub(crate) fn sem_aspas(s: &str) -> String {
    s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s).to_string()
}

struct Linhas<'a> {
    l: Vec<&'a str>,
    i: usize,
    onde: &'static str,
}

impl<'a> Linhas<'a> {
    fn proxima(&mut self) -> R<&'a str> {
        let l = *self.l.get(self.i).ok_or_else(|| ErroDoClt::Formato(self.onde, format!("acabou na linha {}", self.i + 1)))?;
        self.i += 1;
        Ok(l)
    }
    fn branca(&mut self) -> R<()> {
        let n = self.i + 1;
        let l = self.proxima()?;
        if !l.is_empty() {
            return Err(ErroDoClt::Formato(self.onde, format!("esperava linha em branco na {n}: {l:?}")));
        }
        Ok(())
    }
    fn inteiros(&mut self) -> R<Vec<i64>> {
        let n = self.i + 1;
        let onde = self.onde;
        self.proxima()?.split_whitespace().map(|x| x.parse().map_err(|_| ErroDoClt::Formato(onde, format!("número na linha {n}")))).collect()
    }
    fn vetor(&mut self) -> R<[f32; 3]> {
        let n = self.i + 1;
        let onde = self.onde;
        let f = fichas(self.proxima()?);
        if f.len() != 3 {
            return Err(ErroDoClt::Formato(onde, format!("esperava 3 números na linha {n}")));
        }
        let mut v = [0f32; 3];
        for (k, x) in f.iter().enumerate() {
            v[k] = x.parse().map_err(|_| ErroDoClt::Formato(onde, format!("número na linha {n}")))?;
        }
        Ok(v)
    }
}

/// Lê o cabeçalho até `version  N` e o carimbo. Devolve (linhas antes do carimbo, versão, carimbo).
fn cabecalho(l: &mut Linhas) -> R<(Vec<String>, u32, u32)> {
    let mut cab = Vec::new();
    loop {
        let x = l.proxima()?;
        cab.push(x.to_string());
        if let Some(v) = x.strip_prefix("version") {
            let versao = v.trim().parse().map_err(|_| ErroDoClt::Formato(l.onde, "versão".into()))?;
            let carimbo = l.proxima()?.trim().parse().map_err(|_| ErroDoClt::Formato(l.onde, "carimbo".into()))?;
            return Ok((cab, versao, carimbo));
        }
    }
}

fn vetor_txt(v: &[f32; 3], separador: &str) -> String {
    v.iter().map(|x| formatar(*x)).collect::<Vec<_>>().join(separador)
}

// ------------------------------------------------------------------ precinct.clt

#[derive(Debug, Clone, PartialEq)]
pub struct DistritoDoCliente {
    pub nome: String,
    /// A linha de inteiros como veio; `pontos`, `marcas` e `músicas` (índices 1, 2 e 5) são recalculados ao gravar.
    pub numeros: Vec<i64>,
    pub ponto_de_cidade: [f32; 3],
    pub vertices: Vec<[f32; 3]>,
    pub marcas: Vec<(String, [f32; 3])>,
    pub som: String,
    pub musicas: Vec<String>,
    pub som_da_noite: Option<String>,
}

impl DistritoDoCliente {
    pub fn id(&self) -> i64 {
        self.numeros[0]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrecinctDoCliente {
    pub cabecalho: Vec<String>,
    pub versao: u32,
    pub carimbo: u32,
    pub distritos: Vec<DistritoDoCliente>,
}

/// Quantos inteiros a linha do distrito tem por versão (`EL_Precinct.cpp:113-148`).
pub fn inteiros_do_distrito(versao: u32) -> usize {
    (if versao >= 2 { 8 } else { 7 }) + (versao >= 4) as usize + (versao >= 6) as usize + (versao >= 7) as usize
}

impl PrecinctDoCliente {
    pub fn ler(b: &[u8]) -> R<Self> {
        const ONDE: &str = "precinct.clt";
        let t = texto(b, ONDE)?;
        let mut l = Linhas { l: t.split('\n').collect(), i: 0, onde: ONDE };
        let (cabecalho, versao, carimbo) = cabecalho(&mut l)?;
        if versao < 2 {
            return Err(ErroDoClt::Formato(ONDE, format!("versão {versao} sem id de distrito (não há arquivo para conferir)")));
        }
        l.branca()?;
        let mut distritos = Vec::new();
        while l.i < l.l.len() {
            let nome = l.proxima()?;
            if nome.is_empty() {
                if l.i == l.l.len() {
                    break; // o arquivo termina com uma linha em branco a mais
                }
                return Err(ErroDoClt::Formato(ONDE, format!("linha em branco sobrando na {}", l.i)));
            }
            let numeros = l.inteiros()?;
            if numeros.len() != inteiros_do_distrito(versao) {
                return Err(ErroDoClt::Formato(ONDE, format!("{} inteiros no distrito {nome}", numeros.len())));
            }
            let (pontos, marcas, musicas) = (numeros[1] as usize, numeros[2] as usize, numeros[5] as usize);
            let ponto_de_cidade = l.vetor()?;
            let vertices = (0..pontos).map(|_| l.vetor()).collect::<R<Vec<_>>>()?;
            l.branca()?;
            let mut m = Vec::with_capacity(marcas);
            for _ in 0..marcas {
                let linha = l.proxima()?;
                let f = fichas(linha);
                if f.len() != 4 {
                    return Err(ErroDoClt::Formato(ONDE, format!("marca: {linha:?}")));
                }
                let n = |k: usize| f[k].parse::<f32>().map_err(|_| ErroDoClt::Formato(ONDE, format!("marca: {linha:?}")));
                m.push((sem_aspas(f[0]), [n(1)?, n(2)?, n(3)?]));
            }
            l.branca()?;
            let som = sem_aspas(l.proxima()?);
            let musicas = (0..musicas).map(|_| l.proxima().map(sem_aspas)).collect::<R<Vec<_>>>()?;
            let som_da_noite = if versao >= 3 { Some(sem_aspas(l.proxima()?)) } else { None };
            l.branca()?;
            distritos.push(DistritoDoCliente {
                nome: sem_aspas(nome),
                numeros,
                ponto_de_cidade,
                vertices,
                marcas: m,
                som,
                musicas,
                som_da_noite,
            });
        }
        Ok(Self { cabecalho, versao, carimbo, distritos })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let mut s = self.cabecalho.join("\n");
        s.push('\n');
        s.push_str(&self.carimbo.to_string());
        s.push_str("\n\n");
        for d in &self.distritos {
            let mut n = d.numeros.clone();
            n[1] = d.vertices.len() as i64;
            n[2] = d.marcas.len() as i64;
            n[5] = d.musicas.len() as i64;
            s.push_str(&format!("\"{}\"\n", d.nome));
            s.push_str(&n.iter().map(i64::to_string).collect::<Vec<_>>().join("  "));
            s.push('\n');
            s.push_str(&vetor_txt(&d.ponto_de_cidade, ", "));
            s.push('\n');
            for v in &d.vertices {
                s.push_str(&vetor_txt(v, "  "));
                s.push('\n');
            }
            s.push('\n');
            for (nome, p) in &d.marcas {
                s.push_str(&format!("\"{nome}\"  {}\n", vetor_txt(p, "  ")));
            }
            s.push('\n');
            s.push_str(&format!("\"{}\"\n", d.som));
            for m in &d.musicas {
                s.push_str(&format!("\"{m}\"\n"));
            }
            if let Some(noite) = &d.som_da_noite {
                s.push_str(&format!("\"{noite}\"\n"));
            }
            s.push('\n');
        }
        bytes(&s)
    }
}

// ------------------------------------------------------------------ region.clt

#[derive(Debug, Clone, PartialEq)]
pub enum RegistroDoCliente {
    Portal { numeros: Vec<i64>, pos: [f32; 3], exts: [f32; 3], alvo: [f32; 3] },
    Regiao { nome: String, tipo: i64, vertices: Vec<[f32; 3]> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegionDoCliente {
    pub cabecalho: Vec<String>,
    pub versao: u32,
    pub carimbo: u32,
    pub registros: Vec<RegistroDoCliente>,
}

impl RegionDoCliente {
    pub fn ler(b: &[u8]) -> R<Self> {
        const ONDE: &str = "region.clt";
        let t = texto(b, ONDE)?;
        let mut l = Linhas { l: t.split('\n').collect(), i: 0, onde: ONDE };
        let (cabecalho, versao, carimbo) = cabecalho(&mut l)?;
        if versao < 4 {
            return Err(ErroDoClt::Formato(ONDE, format!("versão {versao} < 4 não implementada")));
        }
        l.branca()?;
        let mut registros = Vec::new();
        while l.i < l.l.len() {
            let marca = l.proxima()?;
            match marca {
                "" if l.i == l.l.len() => break,
                "[trans]" => {
                    let numeros = l.inteiros()?;
                    if numeros.len() != 2 + (versao >= 5) as usize {
                        return Err(ErroDoClt::Formato(ONDE, format!("{} inteiros no portal", numeros.len())));
                    }
                    let (pos, exts, alvo) = (l.vetor()?, l.vetor()?, l.vetor()?);
                    registros.push(RegistroDoCliente::Portal { numeros, pos, exts, alvo });
                }
                "[region]" => {
                    let nome = sem_aspas(l.proxima()?);
                    let n = l.inteiros()?;
                    if n.len() != 2 {
                        return Err(ErroDoClt::Formato(ONDE, format!("região {nome}: tipo e pontos")));
                    }
                    let vertices = (0..n[1] as usize).map(|_| l.vetor()).collect::<R<Vec<_>>>()?;
                    registros.push(RegistroDoCliente::Regiao { nome, tipo: n[0], vertices });
                }
                outra => return Err(ErroDoClt::Formato(ONDE, format!("linha inesperada {}: {outra:?}", l.i))),
            }
            l.branca()?;
        }
        Ok(Self { cabecalho, versao, carimbo, registros })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let mut s = self.cabecalho.join("\n");
        s.push('\n');
        s.push_str(&self.carimbo.to_string());
        s.push_str("\n\n");
        for r in &self.registros {
            match r {
                RegistroDoCliente::Portal { numeros, pos, exts, alvo } => {
                    s.push_str("[trans]\n");
                    s.push_str(&numeros.iter().map(i64::to_string).collect::<Vec<_>>().join("  "));
                    s.push('\n');
                    for v in [pos, exts, alvo] {
                        s.push_str(&vetor_txt(v, ", "));
                        s.push('\n');
                    }
                }
                RegistroDoCliente::Regiao { nome, tipo, vertices } => {
                    s.push_str(&format!("[region]\n\"{nome}\"\n{tipo}  {}\n", vertices.len()));
                    for v in vertices {
                        s.push_str(&vetor_txt(v, "  "));
                        s.push('\n');
                    }
                }
            }
            s.push('\n');
        }
        bytes(&s)
    }

    /// A sequência de tipos (`true` = portal), para conferir com a do `region.sev`.
    pub fn sequencia(&self) -> Vec<bool> {
        self.registros.iter().map(|r| matches!(r, RegistroDoCliente::Portal { .. })).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn formatar_como_o_msvc() {
        assert_eq!(formatar(287.8203125), "287.820313", "empate para longe do zero");
        assert_eq!(formatar(-287.8203125), "-287.820313");
        assert_eq!(formatar(0.1), "0.100000");
        assert_eq!(formatar(-4099.773438), "-4099.773438");
        assert_eq!(formatar(0.0), "0.000000");
        assert_eq!(formatar(-0.0), "-0.000000");
        assert_eq!(formatar(9.9999999), "10.000000");
        assert_eq!(formatar(999999.9999999), "1000000.000000");
    }

    /// Os `.clt` reais dos dois clientes (fora do repositório). Sem eles, o teste avisa e não verifica nada.
    #[test]
    fn os_clt_dos_dois_clientes_voltam_identicos() {
        let raizes = [r"F:\Games\perfectworld_126\element\maps", r"F:\PW\1.5.5\1.5.5 BR launcher\element\maps"];
        let (mut p, mut r) = (0, 0);
        for raiz in raizes {
            let Ok(pastas) = std::fs::read_dir(raiz) else {
                eprintln!("AVISO: sem {raiz} — este teste NÃO verificou esse cliente.");
                continue;
            };
            for d in pastas.filter_map(|e| e.ok()).map(|e| e.path()) {
                if let Ok(b) = std::fs::read(d.join("precinct.clt")) {
                    let x = PrecinctDoCliente::ler(&b).unwrap_or_else(|e| panic!("{}: {e}", d.display()));
                    assert!(x.gravar() == b, "{}/precinct.clt não volta idêntico", d.display());
                    p += 1;
                }
                if let Ok(b) = std::fs::read(d.join("region.clt")) {
                    let x = RegionDoCliente::ler(&b).unwrap_or_else(|e| panic!("{}: {e}", d.display()));
                    assert!(x.gravar() == b, "{}/region.clt não volta idêntico", d.display());
                    r += 1;
                }
            }
        }
        println!("precinct.clt {p}, region.clt {r}");
    }
}
