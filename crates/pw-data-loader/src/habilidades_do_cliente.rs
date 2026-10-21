//! B315 (fase 3 do DLL de dados): `habilidades.pwsk`, o arquivo que o nosso `elementskill.dll` lê
//! (`specs/08_ELEMENTSKILL_DE_DADOS.md` §8–§9).
//!
//! O conteúdo vem do **modelo do cliente** (`tools/elementskill/gerar_modelo.py`, JSON): tudo o que o DLL original
//! devolve, medido no binário (o gabarito) e completado pelo fonte. Este módulo é o único escritor do formato (o
//! Estúdio o usará na fase 5) e também o lê, para a ida e volta.
//!
//! Formato (little-endian; `str` = `u16` n + n bytes, `wstr` = `u16` n + n unidades UTF-16):
//! ```text
//! "PWSK" u32 versao_do_formato(1) i32 versao_do_dll u32 n_habilidades u32 n_visiveis u32 n_equipe u32 n_inerentes
//! habilidade × n (na ordem do NextSkill do original):
//!   u32 id u8 max_level i8 tipo u8 flags i32 pre_combo i32 recarga_comum i32 intervalo_combo
//!     flags: 1 duende, 2 móvel, 4 sobrescrita, 8 todas as armas, 16 tem vida, 32 tem combo
//!   str icone wstr nome str nome_nativo str efeito                (as estáticas por id)
//!   wstr texto0 str texto1 str texto3 str texto29 str texto30      (os slots de texto da vtable)
//!   [!todas] u16 n i32 arma×n   [vida] u8 lo u8 hi   [combo] u8 arg u8 n (i32 lo, i32 hi)×n
//!   nível × max_level:
//!     i32 ints×25  i8 bytes×13  f64 a f64 b f64 c
//!     u8 tem_genio [i32×5]  u8 n_pre (u32 id, i32 nivel)×n  u8 n_jun (u32, i32)×n  i32 est×7
//!     u8 n_args  arg×n: u8 tipo (0 palavras, 1 proficiência, 2 proficiência máxima) [0: u8 n u32×n]
//! visível × n: i32 prof i32 consulta i32 id wstr nome str hh str efeito
//! equipe × n: i32 consulta i32 id wstr nome str icone
//! inerentes × n: i32 classe u16 n u32×n
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAGICO: &[u8; 4] = b"PWSK";
pub const VERSAO_DO_FORMATO: u32 = 1;
/// Slots da vtable guardados por nível (spec 08 §3), na ordem do arquivo.
pub const SLOTS_INT: [u8; 25] = [5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 19, 21, 26, 27, 31, 32, 33, 42, 45, 46, 47, 48, 49, 50];
pub const SLOTS_BYTE: [u8; 13] = [2, 22, 23, 24, 25, 35, 36, 37, 38, 39, 40, 41, 52];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modelo {
    pub versao_do_modelo: u32,
    pub versao_do_dll: i32,
    pub habilidades: Vec<Habilidade>,
    pub estados_visiveis: Vec<EstadoVisivel>,
    pub estados_equipe: Vec<EstadoEquipe>,
    /// Classe → habilidades inerentes (`GetInherentSkills`).
    pub inerentes: BTreeMap<String, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estaticas {
    #[serde(rename = "GetType")]
    pub tipo: i8,
    #[serde(rename = "GetIcon")]
    pub icone: String,
    #[serde(rename = "GetName")]
    pub nome: String,
    #[serde(rename = "GetNativeName")]
    pub nome_nativo: String,
    #[serde(rename = "GetEffect")]
    pub efeito: String,
    #[serde(rename = "IsGoblinSkill")]
    pub duende: u8,
    #[serde(rename = "IsMovingSkill")]
    pub movel: u8,
    #[serde(rename = "IsOverridden")]
    pub sobrescrita: u8,
    #[serde(rename = "GetComboSkPreSkill")]
    pub pre_combo: i32,
    #[serde(rename = "GetCommonCoolDown")]
    pub recarga_comum: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Armas {
    /// `"todas"`: `ValidWeapon` verdadeiro para qualquer arma.
    Todas(String),
    Lista(Vec<i32>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Habilidade {
    pub id: u32,
    pub max_level: u8,
    pub estaticas: Estaticas,
    pub armas: Armas,
    /// `CheckHpCondition`: vida em `[lo, hi]` % (`hp ≥ max/100·lo && hp ≤ max/100·hi`, em float).
    pub vida: Option<[u8; 2]>,
    /// `CheckComboSkExtraCondition` (B317): verdadeira se `arg[arg]` cai numa das faixas (fechadas; `i32::MIN`/`MAX`
    /// = sem limite); sem faixas = sempre falsa; `None` = sempre verdadeira. Medida no binário pelas varreduras do
    /// gabarito (os stubs do BR têm faixas que o fonte não tem).
    pub combo: Option<Combo>,
    #[serde(default)]
    pub intervalo_combo: i32,
    pub niveis: Vec<Nivel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Combo {
    pub arg: u8,
    pub faixas: Vec<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nivel {
    pub ints: Vec<i32>,
    pub bytes: Vec<i8>,
    /// Hex: o 0 é UTF-16LE, os outros `char*` (GBK).
    pub textos: Vec<String>,
    pub genio: Option<[i32; 5]>,
    pub pre: Vec<(u32, i32)>,
    pub juniores: Vec<(u32, i32)>,
    pub est: Vec<i32>,
    /// `[a, b, c]`: `d = a·alcance_da_arma + b + c·bônus`.
    pub alcance: [f64; 3],
    pub intro: Vec<Argumento>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argumento {
    Palavras { palavras: Vec<u32> },
    Proficiencia { proficiencia: bool },
    ProficienciaMaxima { proficiencia_max: bool },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstadoVisivel {
    pub prof: i32,
    pub consulta: i32,
    pub id: i32,
    pub nome: String,
    pub hh: String,
    pub efeito: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstadoEquipe {
    pub consulta: i32,
    pub id: i32,
    pub nome: String,
    pub icone: String,
}

fn de_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err(format!("hex ímpar: {s}"));
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("hex {s}: {e}"))).collect()
}

fn para_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

struct Escrita(Vec<u8>);

impl Escrita {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// `char*` em hex → `u16` n + bytes.
    fn str(&mut self, hex: &str) -> Result<(), String> {
        let b = de_hex(hex)?;
        self.u16(u16::try_from(b.len()).map_err(|_| "texto longo demais".to_string())?);
        self.0.extend_from_slice(&b);
        Ok(())
    }
    /// `wchar_t*` em hex UTF-16LE → `u16` unidades + bytes.
    fn wstr(&mut self, hex: &str) -> Result<(), String> {
        let b = de_hex(hex)?;
        if b.len() % 2 != 0 {
            return Err("UTF-16 ímpar".into());
        }
        self.u16(u16::try_from(b.len() / 2).map_err(|_| "texto longo demais".to_string())?);
        self.0.extend_from_slice(&b);
        Ok(())
    }
}

/// O arquivo `habilidades.pwsk` do modelo.
pub fn escrever(m: &Modelo) -> Result<Vec<u8>, String> {
    let mut w = Escrita(Vec::new());
    w.0.extend_from_slice(MAGICO);
    w.u32(VERSAO_DO_FORMATO);
    w.i32(m.versao_do_dll);
    w.u32(m.habilidades.len() as u32);
    w.u32(m.estados_visiveis.len() as u32);
    w.u32(m.estados_equipe.len() as u32);
    w.u32(m.inerentes.len() as u32);
    for h in &m.habilidades {
        let e = &h.estaticas;
        if h.niveis.len() != h.max_level as usize {
            return Err(format!("{}: {} níveis para max_level {}", h.id, h.niveis.len(), h.max_level));
        }
        let todas = matches!(&h.armas, Armas::Todas(t) if t == "todas");
        let flags = e.duende & 1 | (e.movel & 1) << 1 | (e.sobrescrita & 1) << 2 | (todas as u8) << 3
            | (h.vida.is_some() as u8) << 4 | (h.combo.is_some() as u8) << 5;
        w.u32(h.id);
        w.u8(h.max_level);
        w.u8(e.tipo as u8);
        w.u8(flags);
        w.i32(e.pre_combo);
        w.i32(e.recarga_comum);
        w.i32(h.intervalo_combo);
        w.str(&e.icone)?;
        w.wstr(&e.nome)?;
        w.str(&e.nome_nativo)?;
        w.str(&e.efeito)?;
        let t = &h.niveis.first().ok_or_else(|| format!("{}: sem níveis", h.id))?.textos;
        if t.len() != 5 || h.niveis.iter().any(|n| &n.textos != t) {
            return Err(format!("{}: os textos dos slots têm de ser 5 e iguais em todos os níveis", h.id));
        }
        w.wstr(&t[0])?;
        for x in &t[1..] {
            w.str(x)?;
        }
        match &h.armas {
            Armas::Todas(t) if t == "todas" => {}
            Armas::Lista(l) => {
                w.u16(u16::try_from(l.len()).map_err(|_| "armas demais".to_string())?);
                l.iter().for_each(|&x| w.i32(x));
            }
            Armas::Todas(t) => return Err(format!("{}: armas {t:?}", h.id)),
        }
        if let Some([lo, hi]) = h.vida {
            w.u8(lo);
            w.u8(hi);
        }
        if let Some(c) = &h.combo {
            if c.arg > 2 {
                return Err(format!("{}: combo no argumento {}", h.id, c.arg));
            }
            w.u8(c.arg);
            w.u8(u8::try_from(c.faixas.len()).map_err(|_| "faixas demais".to_string())?);
            for &(lo, hi) in &c.faixas {
                w.i32(lo);
                w.i32(hi);
            }
        }
        for (k, n) in h.niveis.iter().enumerate() {
            if n.ints.len() != SLOTS_INT.len() || n.bytes.len() != SLOTS_BYTE.len() || n.est.len() != 7 {
                return Err(format!("{} nível {}: contagem de campos", h.id, k + 1));
            }
            n.ints.iter().for_each(|&x| w.i32(x));
            n.bytes.iter().for_each(|&x| w.u8(x as u8));
            n.alcance.iter().for_each(|&x| w.f64(x));
            match n.genio {
                Some(g) => {
                    w.u8(1);
                    g.iter().for_each(|&x| w.i32(x));
                }
                None => w.u8(0),
            }
            for lista in [&n.pre, &n.juniores] {
                w.u8(u8::try_from(lista.len()).map_err(|_| "lista longa demais".to_string())?);
                for &(id, nv) in lista {
                    w.u32(id);
                    w.i32(nv);
                }
            }
            n.est.iter().for_each(|&x| w.i32(x));
            w.u8(u8::try_from(n.intro.len()).map_err(|_| "argumentos demais".to_string())?);
            for a in &n.intro {
                match a {
                    Argumento::Palavras { palavras } => {
                        w.u8(0);
                        w.u8(palavras.len() as u8);
                        palavras.iter().for_each(|&x| w.u32(x));
                    }
                    Argumento::Proficiencia { .. } => w.u8(1),
                    Argumento::ProficienciaMaxima { .. } => w.u8(2),
                }
            }
        }
    }
    for e in &m.estados_visiveis {
        w.i32(e.prof);
        w.i32(e.consulta);
        w.i32(e.id);
        w.wstr(&e.nome)?;
        w.str(&e.hh)?;
        w.str(&e.efeito)?;
    }
    for e in &m.estados_equipe {
        w.i32(e.consulta);
        w.i32(e.id);
        w.wstr(&e.nome)?;
        w.str(&e.icone)?;
    }
    for (cls, l) in &m.inerentes {
        w.i32(cls.parse().map_err(|_| format!("classe {cls}"))?);
        w.u16(l.len() as u16);
        l.iter().for_each(|&x| w.u32(x));
    }
    Ok(w.0)
}

struct Leitura<'a> {
    b: &'a [u8],
    p: usize,
}

impl Leitura<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], String> {
        let r = self.b.get(self.p..self.p + n).ok_or_else(|| format!("fim do arquivo em {}", self.p))?;
        self.p += n;
        Ok(r)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    fn str(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        Ok(para_hex(self.bytes(n)?))
    }
    fn wstr(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        Ok(para_hex(self.bytes(2 * n)?))
    }
}

/// Lê o arquivo de volta para o modelo (ida e volta; o DLL lê o mesmo formato em C++).
pub fn ler(b: &[u8]) -> Result<Modelo, String> {
    let mut r = Leitura { b, p: 0 };
    if r.bytes(4)? != MAGICO {
        return Err("não é um habilidades.pwsk".into());
    }
    let v = r.u32()?;
    if v != VERSAO_DO_FORMATO {
        return Err(format!("versão do formato {v}"));
    }
    let versao_do_dll = r.i32()?;
    let (nh, nv, ne, ni) = (r.u32()?, r.u32()?, r.u32()?, r.u32()?);
    let mut habilidades = Vec::new();
    for _ in 0..nh {
        let id = r.u32()?;
        let max_level = r.u8()?;
        let tipo = r.u8()? as i8;
        let flags = r.u8()?;
        let (pre_combo, recarga_comum, intervalo_combo) = (r.i32()?, r.i32()?, r.i32()?);
        let estaticas = Estaticas {
            tipo,
            icone: r.str()?,
            nome: r.wstr()?,
            nome_nativo: r.str()?,
            efeito: r.str()?,
            duende: flags & 1,
            movel: flags >> 1 & 1,
            sobrescrita: flags >> 2 & 1,
            pre_combo,
            recarga_comum,
        };
        let textos = vec![r.wstr()?, r.str()?, r.str()?, r.str()?, r.str()?];
        let armas = if flags & 8 != 0 {
            Armas::Todas("todas".into())
        } else {
            let n = r.u16()?;
            Armas::Lista((0..n).map(|_| r.i32()).collect::<Result<_, _>>()?)
        };
        let vida = if flags & 16 != 0 { Some([r.u8()?, r.u8()?]) } else { None };
        let combo = if flags & 32 != 0 {
            let arg = r.u8()?;
            let n = r.u8()?;
            Some(Combo { arg, faixas: (0..n).map(|_| Ok((r.i32()?, r.i32()?))).collect::<Result<_, String>>()? })
        } else {
            None
        };
        let mut niveis = Vec::new();
        for _ in 0..max_level {
            let ints = (0..SLOTS_INT.len()).map(|_| r.i32()).collect::<Result<_, _>>()?;
            let bytes = (0..SLOTS_BYTE.len()).map(|_| r.u8().map(|x| x as i8)).collect::<Result<_, _>>()?;
            let alcance = [r.f64()?, r.f64()?, r.f64()?];
            let genio = if r.u8()? == 1 { Some([r.i32()?, r.i32()?, r.i32()?, r.i32()?, r.i32()?]) } else { None };
            let mut listas = Vec::new();
            for _ in 0..2 {
                let n = r.u8()?;
                listas.push((0..n).map(|_| Ok((r.u32()?, r.i32()?))).collect::<Result<Vec<_>, String>>()?);
            }
            let juniores = listas.pop().unwrap();
            let pre = listas.pop().unwrap();
            let est = (0..7).map(|_| r.i32()).collect::<Result<_, _>>()?;
            let na = r.u8()?;
            let mut intro = Vec::new();
            for _ in 0..na {
                intro.push(match r.u8()? {
                    0 => {
                        let n = r.u8()?;
                        Argumento::Palavras { palavras: (0..n).map(|_| r.u32()).collect::<Result<_, _>>()? }
                    }
                    1 => Argumento::Proficiencia { proficiencia: true },
                    2 => Argumento::ProficienciaMaxima { proficiencia_max: true },
                    t => return Err(format!("argumento {t}")),
                });
            }
            niveis.push(Nivel { ints, bytes, textos: textos.clone(), genio, pre, juniores, est, alcance, intro });
        }
        habilidades.push(Habilidade { id, max_level, estaticas, armas, vida, combo, intervalo_combo, niveis });
    }
    let mut estados_visiveis = Vec::new();
    for _ in 0..nv {
        estados_visiveis.push(EstadoVisivel { prof: r.i32()?, consulta: r.i32()?, id: r.i32()?, nome: r.wstr()?, hh: r.str()?, efeito: r.str()? });
    }
    let mut estados_equipe = Vec::new();
    for _ in 0..ne {
        estados_equipe.push(EstadoEquipe { consulta: r.i32()?, id: r.i32()?, nome: r.wstr()?, icone: r.str()? });
    }
    let mut inerentes = BTreeMap::new();
    for _ in 0..ni {
        let cls = r.i32()?;
        let n = r.u16()?;
        inerentes.insert(cls.to_string(), (0..n).map(|_| r.u32()).collect::<Result<_, _>>()?);
    }
    if r.p != b.len() {
        return Err(format!("{} bytes sobrando", b.len() - r.p));
    }
    Ok(Modelo { versao_do_modelo: 1, versao_do_dll, habilidades, estados_visiveis, estados_equipe, inerentes })
}

// ------------------------------------------------------------------ B325: os números da descrição

/// Uma conversão do formato da descrição (`_snwprintf`), na ordem do texto.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conversao {
    /// O trecho do formato (`%d`, `%.1f`…).
    pub texto: String,
    /// `i` inteiro (1 palavra), `f` real (`double`, 2 palavras), `o` outro (string, `*`… — não se edita).
    pub tipo: char,
    pub palavras: usize,
}

/// As conversões do formato, como o `_snwprintf` as consome (`%%` não consome nada).
pub fn conversoes(formato: &str) -> Vec<Conversao> {
    let c: Vec<char> = formato.chars().collect();
    let mut r = Vec::new();
    let mut i = 0;
    let largura = |c: &[char], i: &mut usize| -> usize {
        if *i < c.len() && c[*i] == '*' {
            *i += 1;
            1
        } else {
            while *i < c.len() && c[*i].is_ascii_digit() {
                *i += 1;
            }
            0
        }
    };
    while i < c.len() {
        if c[i] != '%' {
            i += 1;
            continue;
        }
        let ini = i;
        i += 1;
        while i < c.len() && "-+ #0".contains(c[i]) {
            i += 1;
        }
        let mut estrelas = largura(&c, &mut i);
        if i < c.len() && c[i] == '.' {
            i += 1;
            estrelas += largura(&c, &mut i);
        }
        let mut longo = false;
        for pre in ["I64", "I32", "ll", "hh", "h", "l", "L", "I"] {
            let p: Vec<char> = pre.chars().collect();
            if c[i..].starts_with(&p) {
                longo = matches!(pre, "I64" | "ll" | "L");
                i += p.len();
                break;
            }
        }
        let Some(&conv) = c.get(i) else { break };
        i += 1;
        if conv == '%' {
            continue;
        }
        let texto: String = c[ini..i].iter().collect();
        let (tipo, palavras) = if "eEfgGaA".contains(conv) {
            ('f', 2)
        } else if "diuxXoc".contains(conv) && !longo {
            ('i', 1)
        } else {
            ('o', if longo { 2 } else { 1 })
        };
        if estrelas > 0 {
            r.push(Conversao { texto, tipo: 'o', palavras: palavras + estrelas });
        } else {
            r.push(Conversao { texto, tipo, palavras });
        }
    }
    r
}

/// O valor de uma conversão num nível.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo", content = "valor", rename_all = "snake_case")]
pub enum ValorDaDescricao {
    Inteiro(i32),
    Real(f64),
    /// A proficiência (ou a máxima) que o DLL põe na hora — não é número do arquivo.
    Proficiencia,
    /// O formato pede mais do que a habilidade passa: o original mostra o que estiver na pilha.
    ForaDosArgumentos,
    /// Conversão que não é número simples (string, `*`), ou que mistura proficiência com palavra.
    NaoEditavel,
}

/// Uma palavra do `_snwprintf`: do arquivo, ou a proficiência que o DLL põe na hora (1 = atual, 2 = máxima).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Palavra {
    Arquivo(u32),
    Proficiencia(u8),
}

fn achatar(intro: &[Argumento]) -> Vec<Palavra> {
    let mut v = Vec::new();
    for a in intro {
        match a {
            Argumento::Palavras { palavras } => v.extend(palavras.iter().map(|&p| Palavra::Arquivo(p))),
            Argumento::Proficiencia { .. } => v.push(Palavra::Proficiencia(1)),
            Argumento::ProficienciaMaxima { .. } => v.push(Palavra::Proficiencia(2)),
        }
    }
    v
}

fn remontar(v: &[Palavra]) -> Vec<Argumento> {
    let mut r: Vec<Argumento> = Vec::new();
    for p in v {
        match *p {
            Palavra::Arquivo(w) => match r.last_mut() {
                Some(Argumento::Palavras { palavras }) => palavras.push(w),
                _ => r.push(Argumento::Palavras { palavras: vec![w] }),
            },
            Palavra::Proficiencia(1) => r.push(Argumento::Proficiencia { proficiencia: true }),
            Palavra::Proficiencia(_) => r.push(Argumento::ProficienciaMaxima { proficiencia_max: true }),
        }
    }
    r
}

/// Os números que a descrição mostra num nível, conversão a conversão.
pub fn numeros_da_descricao(nivel: &Nivel, conv: &[Conversao]) -> Vec<ValorDaDescricao> {
    let v = achatar(&nivel.intro);
    let mut k = 0;
    conv.iter()
        .map(|c| {
            let fatia = v.get(k..k + c.palavras);
            k += c.palavras;
            match fatia {
                None => ValorDaDescricao::ForaDosArgumentos,
                Some(f) if f.iter().any(|p| matches!(p, Palavra::Proficiencia(_))) => {
                    if c.palavras == 1 { ValorDaDescricao::Proficiencia } else { ValorDaDescricao::NaoEditavel }
                }
                Some(f) => {
                    let w: Vec<u32> = f.iter().map(|p| match p { Palavra::Arquivo(w) => *w, _ => 0 }).collect();
                    match c.tipo {
                        'i' => ValorDaDescricao::Inteiro(w[0] as i32),
                        'f' => ValorDaDescricao::Real(f64::from_bits(w[0] as u64 | (w[1] as u64) << 32)),
                        _ => ValorDaDescricao::NaoEditavel,
                    }
                }
            }
        })
        .collect()
}

/// Troca os números da descrição de um nível: `valores[i]` para a conversão `i` (`None` = fica). Só número do
/// arquivo se troca; as palavras que o formato não usa ficam como estão.
pub fn definir_numeros_da_descricao(nivel: &mut Nivel, conv: &[Conversao], valores: &[Option<f64>]) -> Result<(), String> {
    if valores.len() != conv.len() {
        return Err(format!("{} valores para {} conversões", valores.len(), conv.len()));
    }
    let mut v = achatar(&nivel.intro);
    let mut k = 0;
    for (c, x) in conv.iter().zip(valores) {
        let ini = k;
        k += c.palavras;
        let Some(x) = x else { continue };
        if !x.is_finite() {
            return Err(format!("{}: número inválido", c.texto));
        }
        let Some(f) = v.get_mut(ini..k) else { return Err(format!("{}: além dos argumentos da habilidade", c.texto)) };
        if f.iter().any(|p| matches!(p, Palavra::Proficiencia(_))) || c.tipo == 'o' {
            return Err(format!("{}: não é número do arquivo", c.texto));
        }
        if c.tipo == 'i' {
            if x.fract() != 0.0 || *x < i32::MIN as f64 || *x > i32::MAX as f64 {
                return Err(format!("{}: inteiro de 32 bits", c.texto));
            }
            f[0] = Palavra::Arquivo(*x as i32 as u32);
        } else {
            let b = x.to_bits();
            f[0] = Palavra::Arquivo(b as u32);
            f[1] = Palavra::Arquivo((b >> 32) as u32);
        }
    }
    nivel.intro = remontar(&v);
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    fn exemplo() -> Modelo {
        let nivel = |k: i32| Nivel {
            ints: (0..25).map(|i| i * k).collect(),
            bytes: vec![1; 13],
            textos: vec!["4100".into(), "61".into(), "".into(), "62".into(), "".into()],
            genio: (k == 2).then_some([1, 2, 3, 4, 5]),
            pre: vec![(7, k)],
            juniores: vec![],
            est: vec![k; 7],
            alcance: [1.0, 19.5 + 0.9 * k as f64, 0.0],
            intro: vec![Argumento::Palavras { palavras: vec![k as u32] }, Argumento::Proficiencia { proficiencia: true }],
        };
        Modelo {
            versao_do_modelo: 1,
            versao_do_dll: 7,
            habilidades: vec![Habilidade {
                id: 8,
                max_level: 2,
                estaticas: Estaticas { tipo: 1, icone: "6162".into(), nome: "4100".into(), nome_nativo: "".into(), efeito: "63".into(), duende: 0, movel: 1, sobrescrita: 0, pre_combo: 0, recarga_comum: 3 },
                armas: Armas::Lista(vec![0, 1, 182]),
                vida: Some([0, 40]),
                combo: Some(Combo { arg: 0, faixas: vec![(34, 66), (100, i32::MAX)] }),
                intervalo_combo: 45000,
                niveis: vec![nivel(1), nivel(2)],
            }],
            estados_visiveis: vec![EstadoVisivel { prof: 0, consulta: 1, id: 1, nome: "3100".into(), hh: "".into(), efeito: "78".into() }],
            estados_equipe: vec![EstadoEquipe { consulta: 2, id: 2, nome: "".into(), icone: "79".into() }],
            inerentes: BTreeMap::from([("0".into(), vec![1, 2])]),
        }
    }

    #[test]
    fn conversoes_como_o_snwprintf() {
        let c = conversoes("Nível %d, dano %.1f (100%%), %s, % d, %*d, %I64d");
        let t: Vec<(String, char, usize)> = c.iter().map(|x| (x.texto.clone(), x.tipo, x.palavras)).collect();
        assert_eq!(t, vec![("%d".into(), 'i', 1), ("%.1f".into(), 'f', 2), ("%s".into(), 'o', 1), ("% d".into(), 'i', 1), ("%*d".into(), 'o', 2), ("%I64d".into(), 'o', 2)]);
    }

    #[test]
    fn le_e_troca_os_numeros_da_descricao() {
        // Habilidade 1 nível 2 do B314: `2 | 9 | 172,3 (double)`, mais a proficiência.
        let d = 172.3f64.to_bits();
        let mut n = exemplo().habilidades[0].niveis[0].clone();
        n.intro = vec![Argumento::Palavras { palavras: vec![2, 9, d as u32, (d >> 32) as u32] }, Argumento::Proficiencia { proficiencia: true }];
        let conv = conversoes("Nível %d: %d de mana, %.1f de dano, %d%% de proficiência, %d");
        assert_eq!(numeros_da_descricao(&n, &conv), vec![
            ValorDaDescricao::Inteiro(2), ValorDaDescricao::Inteiro(9), ValorDaDescricao::Real(172.3),
            ValorDaDescricao::Proficiencia, ValorDaDescricao::ForaDosArgumentos]);
        definir_numeros_da_descricao(&mut n, &conv, &[None, Some(12.0), Some(200.5), None, None]).unwrap();
        assert_eq!(numeros_da_descricao(&n, &conv)[1..3], [ValorDaDescricao::Inteiro(12), ValorDaDescricao::Real(200.5)]);
        assert!(matches!(n.intro[1], Argumento::Proficiencia { .. }), "a proficiência fica no lugar");
        assert!(definir_numeros_da_descricao(&mut n, &conv, &[None, None, None, Some(1.0), None]).is_err(), "proficiência não se edita");
        assert!(definir_numeros_da_descricao(&mut n, &conv, &[None, None, None, None, Some(1.0)]).is_err(), "além dos argumentos");
        assert!(definir_numeros_da_descricao(&mut n, &conv, &[Some(1.5), None, None, None, None]).is_err(), "inteiro");
    }

    #[test]
    fn ida_e_volta() {
        let m = exemplo();
        let b = escrever(&m).unwrap();
        assert_eq!(&b[..4], MAGICO);
        assert_eq!(ler(&b).unwrap(), m);
    }

    #[test]
    fn recusa_textos_que_variam_e_niveis_faltando() {
        let mut m = exemplo();
        m.habilidades[0].niveis[1].textos[1] = "7a".into();
        assert!(escrever(&m).unwrap_err().contains("textos"));
        let mut m = exemplo();
        m.habilidades[0].niveis.pop();
        assert!(escrever(&m).unwrap_err().contains("níveis"));
    }
}
