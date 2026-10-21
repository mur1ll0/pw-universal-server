//! Avaliador das expressões que `extrair_habilidades.py::expressao` deixa: números,
//! `+ - * /`, comparações, `&& ||`, `?:`, parênteses, `INT(...)` e variáveis. Igual ao C++
//! nos pontos que importam: comparação vale 1 ou 0, `INT` trunca.
//!
//! B308: movido de `pw-gs/src/efeitos.rs` para que o Estúdio (`pw-recursos`) valide com o mesmo código que o GS
//! executa; o GS reexporta (`pw_gs::efeitos::expr`).

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Id(String),
    Op(&'static str),
}

fn tokens(s: &str) -> Option<Vec<Tok>> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    const OPS: [&str; 17] = [
        "&&", "||", "==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "(", ")", "?", ":",
        "!",
    ];
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit()
            || (c == '.' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit())
        {
            let ini = i;
            while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                i += 1;
                if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
                    i += 1;
                }
                while i < b.len() && (b[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            let v: f64 = s[ini..i].parse().ok()?;
            if i < b.len() && (b[i] == b'f' || b[i] == b'F') {
                i += 1;
            }
            out.push(Tok::Num(v));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let ini = i;
            while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push(Tok::Id(s[ini..i].to_string()));
            continue;
        }
        let op = OPS.iter().find(|o| s[i..].starts_with(**o))?;
        out.push(Tok::Op(op));
        i += op.len();
    }
    Some(out)
}

struct P<'a> {
    t: Vec<Tok>,
    i: usize,
    vars: &'a dyn Fn(&str) -> Option<f64>,
}

impl P<'_> {
    fn op(&mut self, o: &str) -> bool {
        if matches!(self.t.get(self.i), Some(Tok::Op(x)) if *x == o) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn ternario(&mut self) -> Option<f64> {
        let c = self.ou()?;
        if self.op("?") {
            let a = self.ternario()?;
            if !self.op(":") {
                return None;
            }
            let b = self.ternario()?;
            return Some(if c != 0.0 { a } else { b });
        }
        Some(c)
    }
    fn ou(&mut self) -> Option<f64> {
        let mut a = self.e()?;
        while self.op("||") {
            let b = self.e()?;
            a = ((a != 0.0) || (b != 0.0)) as i32 as f64;
        }
        Some(a)
    }
    fn e(&mut self) -> Option<f64> {
        let mut a = self.igual()?;
        while self.op("&&") {
            let b = self.igual()?;
            a = ((a != 0.0) && (b != 0.0)) as i32 as f64;
        }
        Some(a)
    }
    fn igual(&mut self) -> Option<f64> {
        let mut a = self.rel()?;
        loop {
            if self.op("==") {
                a = (a == self.rel()?) as i32 as f64;
            } else if self.op("!=") {
                a = (a != self.rel()?) as i32 as f64;
            } else {
                return Some(a);
            }
        }
    }
    fn rel(&mut self) -> Option<f64> {
        let mut a = self.soma()?;
        loop {
            if self.op("<=") {
                a = (a <= self.soma()?) as i32 as f64;
            } else if self.op(">=") {
                a = (a >= self.soma()?) as i32 as f64;
            } else if self.op("<") {
                a = (a < self.soma()?) as i32 as f64;
            } else if self.op(">") {
                a = (a > self.soma()?) as i32 as f64;
            } else {
                return Some(a);
            }
        }
    }
    fn soma(&mut self) -> Option<f64> {
        let mut a = self.produto()?;
        loop {
            if self.op("+") {
                a += self.produto()?;
            } else if self.op("-") {
                a -= self.produto()?;
            } else {
                return Some(a);
            }
        }
    }
    fn produto(&mut self) -> Option<f64> {
        let mut a = self.unario()?;
        loop {
            if self.op("*") {
                a *= self.unario()?;
            } else if self.op("/") {
                let b = self.unario()?;
                a = if b == 0.0 { 0.0 } else { a / b };
            } else {
                return Some(a);
            }
        }
    }
    fn unario(&mut self) -> Option<f64> {
        if self.op("-") {
            return Some(-self.unario()?);
        }
        if self.op("+") {
            return self.unario();
        }
        if self.op("!") {
            return Some((self.unario()? == 0.0) as i32 as f64);
        }
        self.primario()
    }
    fn primario(&mut self) -> Option<f64> {
        match self.t.get(self.i).cloned()? {
            Tok::Num(v) => {
                self.i += 1;
                Some(v)
            }
            Tok::Id(nome) => {
                self.i += 1;
                if nome == "INT" {
                    if !self.op("(") {
                        return None;
                    }
                    let v = self.ternario()?;
                    if !self.op(")") {
                        return None;
                    }
                    return Some(v.trunc());
                }
                (self.vars)(&nome)
            }
            Tok::Op("(") => {
                self.i += 1;
                let v = self.ternario()?;
                if !self.op(")") {
                    return None;
                }
                Some(v)
            }
            _ => None,
        }
    }
}

/// `None` quando a expressão não se lê ou usa uma variável que `vars` não conhece.
pub fn avaliar(texto: &str, vars: &dyn Fn(&str) -> Option<f64>) -> Option<f64> {
    let t = tokens(texto)?;
    let mut p = P { t, i: 0, vars };
    let v = p.ternario()?;
    (p.i == p.t.len()).then_some(v)
}

/// B308: as variáveis que a expressão usa (sem `INT`), ou `None` se ela não se lê — a leitura é a do próprio
/// [`avaliar`], com toda variável valendo 1.
pub fn variaveis(texto: &str) -> Option<Vec<String>> {
    avaliar(texto, &|_| Some(1.0))?;
    let mut v: Vec<String> = tokens(texto)?
        .into_iter()
        .filter_map(|t| match t {
            Tok::Id(n) if n != "INT" => Some(n),
            _ => None,
        })
        .collect();
    v.sort();
    v.dedup();
    Some(v)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_as_expressoes_do_catalogo_e_lista_as_variaveis() {
        assert_eq!(avaliar("10 * P_Level * (3 + L)", &|n| Some(if n == "L" { 2.0 } else { 5.0 })), Some(250.0));
        assert_eq!(avaliar("INT(L / 2) > 1 ? 7 : 3", &|_| Some(5.0)), Some(7.0));
        assert_eq!(variaveis("V_Hp < 0.5 * V_Maxhp ? L * 10 : INT(S_Rand)"), Some(vec!["L".into(), "S_Rand".into(), "V_Hp".into(), "V_Maxhp".into()]));
        assert_eq!(variaveis("1 +"), None);
        assert_eq!(variaveis("(L"), None);
    }

    /// Toda expressão de roteiro dos dois catálogos se lê.
    #[test]
    fn todo_roteiro_dos_catalogos_se_le() {
        for v in ["1.5.5", "1.2.6"] {
            let j: serde_json::Value = serde_json::from_str(crate::habilidades::json_do_servidor(v).unwrap()).unwrap();
            let mut n = 0;
            for h in j["habilidades"].as_object().unwrap().values() {
                for r in ["no_alvo", "em_si"] {
                    for p in h[r].as_array().into_iter().flatten() {
                        let e = p[2].as_str().unwrap();
                        assert!(variaveis(e).is_some(), "{v} {} {r}: {e}", h["id"]);
                        n += 1;
                    }
                }
            }
            assert!(n > 1000, "{v}: {n} passos");
        }
    }
}
