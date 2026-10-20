//! Tabela de textos UTF-16 do cliente com fichas de várias linhas (B301, Estúdio S8) — o `Configs\skillstr.txt` (nome e
//! descrição de cada habilidade: `N×10` e `N×10+1`) e as outras tabelas de `CECStringTab::LoadWideStrings`
//! (`ElementClient/EC_StringTab.cpp:202-262`).
//!
//! O leitor do cliente é o juiz: `AWScriptFile` (UTF-16LE com BOM) lê fichas por `GetNextToken`
//! (`AngelicaFile/Source/AWScriptFile.cpp`): pula tudo ≤ 32, `;` e `,`; `//` comenta até o fim da linha e `/* */` em
//! bloco; uma ficha entre aspas vai **até a próxima aspa**, atravessando linhas e **sem escape**; o resto vai até o
//! próximo separador. Tudo até `#_begin` é cabeçalho (`#_index` antes dele = indexada). Indexada: `id "texto"` — o id
//! por `GetNextTokenAsInt(true)` e o texto por `GetNextToken(false)`, que **não atravessa linha antes de começar** (a
//! aspa de abertura fica na linha do id); id repetido não entra (vale o 1º, `m_WStrTab.put`). Uma ficha tem no máximo
//! `MAX_LINELEN − 1` = 2.047 unidades (`AWScriptFile.h:52`).
//!
//! Trabalha nas unidades UTF-16 do arquivo: o que não é editado fica byte a byte igual (o `skillstr.txt` do 1.5.5 BR
//! mistura CRLF e LF).

#[derive(Debug, Clone)]
pub struct Ficha {
    /// O id (indexada) ou a posição (sequencial).
    pub chave: i64,
    /// Onde está o texto (sem as aspas) e se estava entre aspas.
    pub inicio: usize,
    pub fim: usize,
    pub entre_aspas: bool,
    /// Id repetido: o cliente fica com o 1º (esta não vale).
    pub repetida: bool,
}

#[derive(Debug, Clone)]
pub struct TabelaDeTextos {
    texto: Vec<u16>,
    pub indexada: bool,
    pub fichas: Vec<Ficha>,
}

pub const MAX_FICHA: usize = 2047;

struct Leitor<'a> {
    t: &'a [u16],
    p: usize,
}

impl Leitor<'_> {
    /// `GetNextToken(cruza)`: devolve (início, fim, entre aspas) do conteúdo da ficha.
    fn ficha(&mut self, cruza: bool) -> Option<(usize, usize, bool)> {
        let t = self.t;
        loop {
            while self.p < t.len() {
                let c = t[self.p];
                if c > 32 && c != b';' as u16 && c != b',' as u16 {
                    break;
                }
                if c == b'\n' as u16 && !cruza {
                    return None;
                }
                self.p += 1;
            }
            if self.p >= t.len() {
                return None;
            }
            if t[self.p] == b'/' as u16 && t.get(self.p + 1) == Some(&(b'/' as u16)) {
                while self.p < t.len() && t[self.p] != b'\n' as u16 {
                    self.p += 1;
                }
                if self.p >= t.len() || !cruza {
                    return None;
                }
                self.p += 1;
                continue;
            }
            if t[self.p] == b'/' as u16 && t.get(self.p + 1) == Some(&(b'*' as u16)) {
                self.p += 2;
                let mut erro = false;
                while !(t.get(self.p) == Some(&(b'*' as u16)) && t.get(self.p + 1) == Some(&(b'/' as u16))) {
                    if self.p >= t.len() {
                        return None;
                    }
                    if t[self.p] == b'\n' as u16 && !cruza {
                        erro = true;
                    }
                    self.p += 1;
                }
                self.p += 2;
                if erro {
                    return None;
                }
                continue;
            }
            break;
        }
        let c = t[self.p];
        if c == b'"' as u16 || c == b'(' as u16 {
            let fecho = if c == b'"' as u16 { b'"' as u16 } else { b')' as u16 };
            self.p += 1;
            let ini = self.p;
            while self.p < t.len() && t[self.p] != fecho {
                self.p += 1;
            }
            let fim = self.p;
            self.p += 1;
            Some((ini, fim, true))
        } else {
            let ini = self.p;
            while self.p < t.len() && t[self.p] > 32 && t[self.p] != b';' as u16 && t[self.p] != b',' as u16 {
                self.p += 1;
            }
            Some((ini, self.p, false))
        }
    }
}

impl TabelaDeTextos {
    pub fn ler(b: &[u8]) -> Result<Self, String> {
        if !b.starts_with(&[0xFF, 0xFE]) || b.len() % 2 != 0 {
            return Err("não é UTF-16LE com BOM (o AWScriptFile exige)".into());
        }
        let texto: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let mut l = Leitor { t: &texto, p: 0 };
        let igual = |a: (usize, usize, bool), s: &str| String::from_utf16_lossy(&texto[a.0..a.1]).eq_ignore_ascii_case(s);
        let mut indexada = false;
        let mut comecou = false;
        while let Some(f) = l.ficha(true) {
            if igual(f, "#_index") {
                indexada = true;
            } else if igual(f, "#_begin") {
                comecou = true;
                break;
            }
        }
        let mut fichas = Vec::new();
        if comecou {
            let mut vistos = std::collections::HashSet::new();
            let mut n = 0i64;
            if indexada {
                while let Some(id) = l.ficha(true) {
                    // `GetNextTokenAsInt`: o número no começo da ficha (o resto é ignorado).
                    let s = String::from_utf16_lossy(&texto[id.0..id.1]);
                    let digitos: String = s.trim().chars().enumerate().take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && *c == '-')).map(|x| x.1).collect();
                    let chave = digitos.parse::<i64>().unwrap_or(0);
                    match l.ficha(false) {
                        Some((ini, fim, aspas)) => {
                            let repetida = !vistos.insert(chave);
                            fichas.push(Ficha { chave, inicio: ini, fim, entre_aspas: aspas, repetida });
                        }
                        None => fichas.push(Ficha { chave, inicio: l.p, fim: l.p, entre_aspas: false, repetida: !vistos.insert(chave) }),
                    }
                }
            } else {
                while let Some((ini, fim, aspas)) = l.ficha(true) {
                    fichas.push(Ficha { chave: n, inicio: ini, fim, entre_aspas: aspas, repetida: false });
                    n += 1;
                }
            }
        }
        Ok(Self { texto, indexada, fichas })
    }

    /// O texto que o cliente usa para `chave` (a 1ª ficha com ela).
    pub fn texto(&self, chave: i64) -> Option<String> {
        self.fichas.iter().find(|f| f.chave == chave && !f.repetida).map(|f| String::from_utf16_lossy(&self.texto[f.inicio..f.fim]))
    }

    /// Troca o texto de `chave` (a ficha que vale). Recusa aspa (o leitor não tem escape), texto acima de 2.047 unidades
    /// e chave que não existe; ficha que não estava entre aspas passa a estar.
    pub fn definir(&mut self, chave: i64, novo: &str) -> Result<(), String> {
        if novo.contains('"') {
            return Err("o texto não pode ter aspas (o leitor do cliente não tem escape)".into());
        }
        let u: Vec<u16> = novo.encode_utf16().collect();
        if u.len() > MAX_FICHA {
            return Err(format!("{} unidades: o cliente lê até {MAX_FICHA} por ficha", u.len()));
        }
        let i = self.fichas.iter().position(|f| f.chave == chave && !f.repetida).ok_or_else(|| format!("a ficha {chave} não existe"))?;
        let f = self.fichas[i].clone();
        let (ini, fim, novo_u) = if f.entre_aspas {
            (f.inicio, f.fim, u)
        } else {
            (f.inicio, f.fim, [vec![b'"' as u16], u, vec![b'"' as u16]].concat())
        };
        let delta = novo_u.len() as isize - (fim - ini) as isize;
        self.texto.splice(ini..fim, novo_u.iter().copied());
        let deslocar = |x: usize| if x >= fim { (x as isize + delta) as usize } else { x };
        for g in self.fichas.iter_mut() {
            g.inicio = deslocar(g.inicio);
            g.fim = deslocar(g.fim);
        }
        let g = &mut self.fichas[i];
        if f.entre_aspas {
            g.fim = (fim as isize + delta) as usize;
        } else {
            g.inicio = ini + 1;
            g.fim = ini + novo_u.len() - 1;
            g.entre_aspas = true;
        }
        Ok(())
    }

    pub fn escrever(&self) -> Vec<u8> {
        let mut b = vec![0xFF, 0xFE];
        b.extend(self.texto.iter().flat_map(|u| u.to_le_bytes()));
        b
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn em_utf16(s: &str) -> Vec<u8> {
        [vec![0xFF, 0xFE], s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()].concat()
    }

    #[test]
    fn regras_do_leitor() {
        let s = "// cabeçalho\r\n#_index\r\n#_begin\r\n10  \"Ataque\"\r\n11  \"linha 1\r\nlinha 2\"\n/* bloco */ 12 sem_aspas\n10 \"repetida\"\n13\n\"na outra linha\"\n";
        let mut t = TabelaDeTextos::ler(&em_utf16(s)).unwrap();
        assert!(t.indexada);
        assert_eq!(t.texto(10).as_deref(), Some("Ataque"));
        assert_eq!(t.texto(11).as_deref(), Some("linha 1\r\nlinha 2"));
        assert_eq!(t.texto(12).as_deref(), Some("sem_aspas"));
        assert!(t.fichas.iter().any(|f| f.chave == 10 && f.repetida), "id repetido: vale o 1º");
        // `GetNextToken(false)` não cruza linha antes de começar: a ficha 13 fica vazia.
        assert_eq!(t.texto(13).as_deref(), Some(""));
        assert_eq!(t.escrever(), em_utf16(s), "sem edição, igual");
        t.definir(11, "uma linha só").unwrap();
        t.definir(12, "com aspas agora").unwrap();
        let w = t.escrever();
        let r = TabelaDeTextos::ler(&w).unwrap();
        assert_eq!((r.texto(10).as_deref(), r.texto(11).as_deref(), r.texto(12).as_deref()), (Some("Ataque"), Some("uma linha só"), Some("com aspas agora")));
        assert!(t.definir(10, "a\"b").is_err());
        assert!(t.definir(10, &"x".repeat(2048)).is_err());
        assert!(t.definir(99, "x").is_err());
    }

    /// O `skillstr.txt` real do cliente 1.5.5 BR: relê byte a byte igual; o nome da habilidade 1 ("Ataque do Tigre") em
    /// 10; trocar uma descrição de várias linhas muda só ela.
    #[test]
    fn skillstr_do_cliente() {
        let p = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\configs\skillstr.txt");
        let Ok(b) = std::fs::read(p) else {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", p.display());
            return;
        };
        let mut t = TabelaDeTextos::ler(&b).unwrap();
        assert_eq!(t.escrever(), b);
        let validas = t.fichas.iter().filter(|f| !f.repetida).count();
        eprintln!("skillstr: {} fichas ({validas} valem, {} repetidas), indexada {}", t.fichas.len(), t.fichas.len() - validas, t.indexada);
        assert_eq!(t.texto(10).as_deref(), Some("Ataque do Tigre"));
        let antes: Vec<(i64, Option<String>)> = t.fichas.iter().filter(|f| !f.repetida).map(|f| (f.chave, t.texto(f.chave))).collect();
        t.definir(11, "Nova descrição\r\nem duas linhas (Nível %d)").unwrap();
        let r = TabelaDeTextos::ler(&t.escrever()).unwrap();
        for (k, v) in antes {
            if k == 11 {
                assert_eq!(r.texto(11).as_deref(), Some("Nova descrição\r\nem duas linhas (Nível %d)"));
            } else {
                assert_eq!(r.texto(k), v, "ficha {k}");
            }
        }
    }
}
