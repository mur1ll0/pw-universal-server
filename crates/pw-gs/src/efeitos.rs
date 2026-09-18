//! Efeitos de estado — os *filtros* das habilidades.
//!
//! No original, o que uma habilidade faz além do dano mora em `StateAttack` (na vítima) e
//! `BlessMe` (em quem conjura), em cada stub `cskill/skills/skillNNN.h`. São sequências
//! de `SetProbability/SetTime/SetRatio/SetValue/SetAmount` seguidas de um `SetX(1)` que
//! cria o efeito `X` (`PlayerWrapper::SetX`, `cskill/skill/playerwrapper.cpp`). O efeito é
//! um `filter` (`cskill/skill/skillfilter.h`) guardado no `filter_man` do objeto
//! (`cgame/gs/filter_man.h:164-222`), que a cada segundo desconta o tempo
//! (`timeout_filter::Heartbeat`, `gs/filter.h:199-203`) e, ao entrar e sair, mexe nos
//! realces percentuais (`_en_percent`, `obj_interface.cpp:200-561`), nos estados visíveis
//! (`IncVisibleState` → `UPDATE_EXT_STATE`) e nos ícones (`InsertTeamVisibleState` →
//! `ICON_STATE_NOTIFY`, `actobject.h:1760-1860`).
//!
//! Aqui ficam as partes puras: o avaliador das expressões extraídas dos stubs
//! ([`expr`]), a tabela dos efeitos portados ([`Efeito`]), a coleção de filtros de um
//! objeto ([`Efeitos`]) e o roteiro ([`executar_roteiro`]). Quem aplica no mundo e avisa o
//! cliente é `bus_server/habilidades.rs`.

use std::collections::HashMap;

// =============================================================================
// Expressões
// =============================================================================

/// Avaliador das expressões que `extrair_habilidades.py::expressao` deixa: números,
/// `+ - * /`, comparações, `&& ||`, `?:`, parênteses, `INT(...)` e variáveis. Igual ao C++
/// nos pontos que importam: comparação vale 1 ou 0, `INT` trunca.
pub mod expr {
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
        const OPS: [&str; 17] = ["&&", "||", "==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "(", ")", "?", ":", "!"];
        while i < b.len() {
            let c = b[i] as char;
            if c.is_whitespace() {
                i += 1;
                continue;
            }
            if c.is_ascii_digit() || (c == '.' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit()) {
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
}

// =============================================================================
// Os efeitos portados
// =============================================================================

/// Como um filtro novo convive com um do mesmo tipo (`filter_man::AddFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Convivencia {
    /// `FILTER_MASK_UNIQUE`: o novo substitui o antigo.
    Unico,
    /// `FILTER_MASK_WEAK`: com um antigo presente, o novo é descartado.
    Fraco,
    /// `FILTER_MASK_MERGE`: o antigo absorve o novo (`Merge`).
    Fundir,
    /// Nenhuma das três: os dois ficam.
    Livre,
}

/// Os efeitos com porte. O nome é o do setter do stub (`SetSlow` → `Slow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Efeito {
    Slow,
    Speedup,
    Fix,
    Dizzy,
    Sleep,
    Sealed,
    Toxic,
    Bleeding,
    Burning,
    Flood,
    Thunder,
    Fallen,
    Incattack,
    Decattack,
    Incmagic,
    Decmagic,
    Incdefence,
    Decdefence,
    Incresist,
    Decresist,
    Incdodge,
    Decdodge,
    Incaccuracy,
    Decaccuracy,
    Fastattack,
    Slowattack,
    Fastpray,
    Slowpray,
    Inchurt,
    Dechurt,
    Incsmite,
    Hpgen,
    Mpgen,
    Inchp,
    Dechp,
    Powerup,
    Invincible,
}

/// O que se sabe de cada efeito: convivência, se é bênção (`FILTER_MASK_BUFF`) ou
/// maldição (`FILTER_MASK_DEBUFF`), o ícone (`HSTATE_*`) e o estado visível (`VSTATE_*`),
/// com os números de `cskill/skill/statedef.h`.
pub struct Ficha {
    pub convivencia: Convivencia,
    pub bencao: bool,
    pub maldicao: bool,
    pub icone: u16,
    pub visivel: u16,
}

const VS_BLESSED: u16 = 1;
const VS_CURSED: u16 = 2;
const VS_INFAUST: u16 = 9;

impl Efeito {
    pub fn do_setter(nome: &str) -> Option<Self> {
        use Efeito::*;
        Some(match nome {
            "Slow" => Slow,
            "Speedup" => Speedup,
            "Fix" => Fix,
            "Dizzy" => Dizzy,
            "Sleep" => Sleep,
            "Sealed" => Sealed,
            "Toxic" => Toxic,
            "Bleeding" => Bleeding,
            "Burning" => Burning,
            "Flood" => Flood,
            "Thunder" => Thunder,
            "Fallen" => Fallen,
            "Incattack" => Incattack,
            "Decattack" => Decattack,
            "Incmagic" => Incmagic,
            "Decmagic" => Decmagic,
            "Incdefence" => Incdefence,
            "Decdefence" => Decdefence,
            "Incresist" => Incresist,
            "Decresist" => Decresist,
            "Incdodge" => Incdodge,
            "Decdodge" => Decdodge,
            "Incaccuracy" => Incaccuracy,
            "Decaccuracy" => Decaccuracy,
            "Fastattack" => Fastattack,
            "Slowattack" => Slowattack,
            "Fastpray" => Fastpray,
            "Slowpray" => Slowpray,
            "Inchurt" => Inchurt,
            "Dechurt" => Dechurt,
            "Incsmite" => Incsmite,
            "Hpgen" => Hpgen,
            "Mpgen" => Mpgen,
            "Inchp" => Inchp,
            "Dechp" => Dechp,
            "Powerup" => Powerup,
            "Invincible" => Invincible,
            _ => return None,
        })
    }

    /// `skillfilter.h`, classe `filter_<Nome>` de cada um (os números em `statedef.h`).
    pub fn ficha(self) -> Ficha {
        use Convivencia::*;
        use Efeito::*;
        let f = |convivencia, bencao, icone, visivel| Ficha { convivencia, bencao, maldicao: !bencao, icone, visivel };
        match self {
            // `filter_Slow`: UNIQUE|DEBUFF, VSTATE_SLOW 4, HSTATE_SLOW 3.
            Slow => f(Unico, false, 3, 4),
            // `filter_Speedup`: MERGE|BUFF, VSTATE_BLESSED, HSTATE_SPEEDUP 44.
            Speedup => f(Fundir, true, 44, VS_BLESSED),
            // `filter_Fix`: WEAK|DEBUFF, VSTATE_FIX 7, HSTATE_FIX 8.
            Fix => f(Fraco, false, 8, 7),
            // `filter_Dizzy`: WEAK|DEBUFF, VSTATE_DIZZY 5, HSTATE_DIZZY 1.
            Dizzy => f(Fraco, false, 1, 5),
            // `filter_Sleep`: WEAK|DEBUFF, VSTATE_SLEEP 6, HSTATE_SLEEP 2.
            Sleep => f(Fraco, false, 2, 6),
            // `filter_Sealed`: WEAK|DEBUFF, VSTATE_SEALED 8, HSTATE_SEALED 9.
            Sealed => f(Fraco, false, 9, 8),
            // `filter_Wounded` e filhos: sem UNIQUE/WEAK/MERGE (contra monstro).
            Toxic => f(Livre, false, 13, 11),
            Bleeding => f(Livre, false, 17, 14),
            Burning => f(Livre, false, 14, 12),
            // `filter_Frozen`: só o ícone HSTATE_FROZEN 73.
            Flood => f(Livre, false, 73, 0),
            Thunder => f(Livre, false, 12, 10),
            Fallen => f(Livre, false, 15, 13),
            Incattack => f(Fundir, true, 32, VS_BLESSED),
            Decattack => f(Unico, false, 20, VS_CURSED),
            Incmagic => f(Fundir, true, 46, VS_BLESSED),
            Decmagic => f(Unico, false, 43, VS_CURSED),
            Incdefence => f(Fundir, true, 30, VS_BLESSED),
            Decdefence => f(Unico, false, 18, VS_CURSED),
            Incresist => f(Unico, true, 31, VS_BLESSED),
            Decresist => f(Unico, false, 19, VS_CURSED),
            Incdodge => f(Fundir, true, 34, VS_BLESSED),
            Decdodge => f(Unico, false, 25, VS_CURSED),
            Incaccuracy => f(Unico, true, 36, VS_BLESSED),
            Decaccuracy => f(Unico, false, 24, VS_INFAUST),
            // `filter_Crazy`: MERGE|BUFF.
            Fastattack => f(Fundir, true, 33, VS_BLESSED),
            // `filter_Tardy`: UNIQUE|DEBUFF.
            Slowattack => f(Unico, false, 22, VS_INFAUST),
            Fastpray => f(Unico, true, 37, 0),
            Slowpray => f(Unico, false, 23, VS_INFAUST),
            Inchurt => f(Unico, false, 21, VS_INFAUST),
            Dechurt => f(Unico, true, 35, 0),
            // `filter_Incsmite`: WEAK|BUFF.
            Incsmite => f(Fraco, true, 74, VS_BLESSED),
            Hpgen => f(Livre, true, 41, 0),
            Mpgen => f(Livre, true, 42, 0),
            Inchp => f(Fundir, true, 28, VS_BLESSED),
            // `filter_Dechp`: UNIQUE, DEBUFF pelo construtor.
            Dechp => f(Unico, false, 45, VS_CURSED),
            // `filter_Powerup`: UNIQUE|BUFF, VSTATE_POWERUP 17, HSTATE_POWERUP 60.
            Powerup => f(Unico, true, 60, 17),
            // `SetInvincibleFilter` + `filter_Icon(HSTATE_INVINCIBLE 76)` quando `showicon`.
            Invincible => f(Unico, true, 76, 0),
        }
    }

    /// Dano ao longo do tempo (`filter_Wounded`), com a classe de dano do
    /// `CalcMagicDamage` de cada `Set` (`playerwrapper.cpp`): `None` = físico.
    pub fn dano_no_tempo(self) -> Option<Option<usize>> {
        use Efeito::*;
        match self {
            Bleeding => Some(None),
            Thunder => Some(Some(0)),
            Toxic => Some(Some(1)),
            Flood => Some(Some(2)),
            Burning => Some(Some(3)),
            Fallen => Some(Some(4)),
            _ => None,
        }
    }
}

/// Um filtro vivo num objeto.
#[derive(Debug, Clone, PartialEq)]
pub struct Filtro {
    pub efeito: Efeito,
    /// `_timeout`, em segundos.
    pub restante_s: i32,
    /// `(int)(ratio * 100)` — a porcentagem dos realces.
    pub razao: i32,
    /// `ratio` sem arredondar, para `Inchurt`/`Dechurt` (`1 ± ratio`).
    pub fator: f32,
    /// `_damage` / `_health` por segundo, e `_point` do `Incsmite`.
    pub por_segundo: i32,
    pub contador: i32,
    /// Quem causou — o dono do dano do `filter_Wounded`.
    pub origem: i64,
    /// Se mostra ícone (`Invincible` só com `showicon`).
    pub icone: bool,
}

/// O que um segundo de filtros faz ([`Efeitos::batida`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Tique {
    Dano { origem: i64, valor: i32 },
    Cura(i32),
    Mana(i32),
}

/// Soma dos realces vivos — o `_en_percent` (e o `_crit_rate`) que os filtros mexem.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Realce {
    pub dano: i32,
    pub magia: i32,
    pub defesa: i32,
    pub evasao: i32,
    pub precisao: i32,
    /// `_en_percent.attack_speed` — **negativo** acelera (`EnhanceScaleAttackSpeed` subtrai).
    pub velocidade_de_ataque: i32,
    pub velocidade: i32,
    pub vida: i32,
    pub resistencia: i32,
    pub critico: i32,
    /// `DecPrayTime`/`IncPrayTime` — porcentagem a menos no tempo de conjuração.
    pub conjuracao: i32,
    /// Multiplica o dano recebido (`AdjustDamage` de `Inchurt`/`Dechurt`).
    pub dano_recebido: f32,
}

/// Os filtros de um objeto.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Efeitos {
    pub filtros: Vec<Filtro>,
    /// `_invincible_timeout` do `SetInvincibleFilter` (sem ícone).
    pub invencivel_s: i32,
}

impl Efeitos {
    /// `filter_man::AddFilter` com a convivência de cada efeito. `true` quando algo mudou.
    pub fn adicionar(&mut self, novo: Filtro) -> bool {
        let ficha = novo.efeito.ficha();
        if let Some(i) = self.filtros.iter().position(|f| f.efeito == novo.efeito) {
            match ficha.convivencia {
                Convivencia::Unico => {
                    self.filtros.retain(|f| f.efeito != novo.efeito);
                }
                Convivencia::Fraco => return false,
                Convivencia::Fundir => {
                    let velho = &mut self.filtros[i];
                    if novo.efeito.dano_no_tempo().is_some() {
                        // `filter_Wounded::Merge`.
                        let resto = velho.por_segundo * velho.restante_s;
                        velho.restante_s = novo.restante_s.max(1);
                        let r = resto / velho.restante_s;
                        velho.por_segundo = (novo.por_segundo + if r != 0 { r } else { 1 }).max(1);
                    } else {
                        // `Merge` dos realces: o tempo e a razão do novo.
                        velho.restante_s = novo.restante_s;
                        velho.razao = novo.razao;
                        velho.fator = novo.fator;
                    }
                    return true;
                }
                Convivencia::Livre => {}
            }
        }
        self.filtros.push(novo);
        true
    }

    /// Um segundo (`FILTER_MASK_HEARTBEAT`, `Heartbeat(1)`). Devolve os tiques e se algum
    /// filtro acabou.
    pub fn batida(&mut self) -> (Vec<Tique>, bool) {
        let mut tiques = Vec::new();
        for f in &mut self.filtros {
            // `filter_Wounded::Heartbeat`, `filter_Hpgen/Mpgen::Heartbeat`: a cada 3 s, ou no
            // último, o acumulado.
            let acumula = f.efeito.dano_no_tempo().is_some() || matches!(f.efeito, Efeito::Hpgen | Efeito::Mpgen);
            if acumula {
                f.contador += 1;
                if f.contador >= 3 || 1 >= f.restante_s {
                    let v = f.por_segundo * f.contador;
                    f.contador -= 3;
                    match f.efeito {
                        Efeito::Hpgen => tiques.push(Tique::Cura(v)),
                        Efeito::Mpgen => tiques.push(Tique::Mana(v)),
                        _ => tiques.push(Tique::Dano { origem: f.origem, valor: v }),
                    }
                }
            }
            f.restante_s -= 1;
        }
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.restante_s > 0);
        if self.invencivel_s > 0 {
            self.invencivel_s -= 1;
        }
        (tiques, self.filtros.len() != antes)
    }

    /// `ClearSpecFilter(FILTER_MASK_BUFF/DEBUFF)`.
    pub fn limpar(&mut self, bencaos: bool) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| {
            let fi = f.efeito.ficha();
            if bencaos { !fi.bencao } else { !fi.maldicao }
        });
        antes != self.filtros.len()
    }

    /// `FILTER_MASK_REMOVE_ON_DEATH` — todos os portados têm.
    pub fn ao_morrer(&mut self) {
        self.filtros.clear();
        self.invencivel_s = 0;
    }

    /// `Sleep::DoDamage`: dano acorda.
    pub fn ao_receber_dano(&mut self) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.efeito != Efeito::Sleep);
        antes != self.filtros.len()
    }

    fn tem(&self, e: Efeito) -> bool {
        self.filtros.iter().any(|f| f.efeito == e)
    }

    /// `MODE_INDEX_STUN`/`SLEEP`: não age.
    pub fn sem_acao(&self) -> bool {
        self.tem(Efeito::Dizzy) || self.tem(Efeito::Sleep)
    }
    /// `MODE_INDEX_ROOT`: não anda.
    pub fn preso(&self) -> bool {
        self.sem_acao() || self.tem(Efeito::Fix)
    }
    /// `MODE_INDEX_SILENT`: não conjura.
    pub fn selado(&self) -> bool {
        self.sem_acao() || self.tem(Efeito::Sealed)
    }
    pub fn invencivel(&self) -> bool {
        self.invencivel_s > 0 || self.tem(Efeito::Invincible)
    }

    pub fn realce(&self) -> Realce {
        use Efeito::*;
        let mut r = Realce { dano_recebido: 1.0, ..Default::default() };
        for f in &self.filtros {
            let k = f.razao;
            match f.efeito {
                Slow => r.velocidade -= k,
                Speedup => r.velocidade += k,
                Incattack => r.dano += k,
                Decattack => r.dano -= k,
                Incmagic => r.magia += k,
                Decmagic => r.magia -= k,
                Incdefence => r.defesa += k,
                Decdefence => r.defesa -= k,
                Incresist => r.resistencia += k,
                Decresist => r.resistencia -= k,
                Incdodge => r.evasao += k,
                Decdodge => r.evasao -= k,
                Incaccuracy => r.precisao += k,
                Decaccuracy => r.precisao -= k,
                Fastattack => r.velocidade_de_ataque -= k,
                Slowattack => r.velocidade_de_ataque += k,
                Fastpray => r.conjuracao += k,
                Slowpray => r.conjuracao -= k,
                Inchp => r.vida += k,
                Dechp => r.vida -= k,
                Incsmite => r.critico += f.por_segundo,
                Inchurt => r.dano_recebido *= 1.0 + f.fator,
                Dechurt => r.dano_recebido *= 1.0 - f.fator,
                _ => {}
            }
        }
        r
    }

    /// Os seis `DWORD` de estado visível (`gactive_imp::UpdateVisibleState`,
    /// `actobject.cpp:1531-1590`).
    pub fn estados_visiveis(&self) -> [u32; 6] {
        let mut s = [0u32; 6];
        for f in &self.filtros {
            let v = f.efeito.ficha().visivel as usize;
            if v > 0 && v < 192 {
                s[v / 32] |= 1 << (v % 32);
            }
        }
        s
    }

    /// Os ícones com o tempo restante (`_visible_team_state`, um parâmetro cada:
    /// `InsertTeamVisibleState(HSTATE, _timeout)`), sem repetir ícone.
    pub fn icones(&self) -> Vec<(u16, i32)> {
        let mut out: Vec<(u16, i32)> = Vec::new();
        for f in &self.filtros {
            if !f.icone {
                continue;
            }
            let h = f.efeito.ficha().icone;
            if h != 0 && !out.iter().any(|(x, _)| *x == h) {
                out.push((h, f.restante_s));
            }
        }
        out
    }
}

/// O dano que chega a um objeto, depois dos filtros dele: invencível não leva nada
/// (`SetInvincibleFilter`), `Inchurt`/`Dechurt` multiplicam (`AdjustDamage`,
/// `skillfilter.h:1838-1860`, `3239-3260`) e o sono acaba (`filter_Sleep::DoDamage`).
pub fn dano_recebido(efeitos: &mut Efeitos, dano: i32) -> i32 {
    if dano <= 0 {
        return dano;
    }
    if efeitos.invencivel() {
        return 0;
    }
    efeitos.ao_receber_dano();
    ((dano as f32) * efeitos.realce().dano_recebido) as i32
}

// =============================================================================
// O roteiro de StateAttack/BlessMe
// =============================================================================

/// Um efeito que passou no dado, com os parâmetros daquele momento do roteiro.
#[derive(Debug, Clone, PartialEq)]
pub struct Aplicacao {
    pub nome: String,
    pub efeito: Option<Efeito>,
    pub tempo_s: i32,
    pub razao: f32,
    pub valor: f32,
    pub quantia: f32,
    pub mostra_icone: bool,
}

/// `PlayerWrapper` do lado da vítima: os parâmetros que os `SetX` guardam
/// (`playerwrapper.h:160-205`) e o dado (`ThrowDice`, `:170-178`), que **fixa** a
/// probabilidade em 100 ou 0 depois de rolar — o efeito seguinte sem `SetProbability`
/// herda o resultado.
pub fn executar_roteiro(
    passos: &[(String, String, String)],
    vars: &dyn Fn(&str) -> Option<f64>,
    dado: &mut dyn FnMut() -> i32,
) -> (Vec<Aplicacao>, Vec<String>) {
    let mut prob = 0.0f64;
    let mut tempo_s = 0i32;
    let (mut razao, mut valor, mut quantia) = (0.0f32, 0.0f32, 0.0f32);
    let mut icone = false;
    let mut aplicar = Vec::new();
    let mut nao_lidos = Vec::new();
    for (quem, setter, e) in passos {
        if quem != "V" {
            continue;
        }
        let Some(v) = expr::avaliar(e, vars) else {
            nao_lidos.push(format!("{setter}({e})"));
            continue;
        };
        match setter.as_str() {
            "Probability" => {
                if v >= 0.0 {
                    prob = v;
                }
            }
            // `SetTime`: `(int)((t + 0.00001) / 1000)`.
            "Time" => tempo_s = ((v as f32 + 0.00001) / 1000.0) as i32,
            "Ratio" => razao = v as f32,
            "Value" => valor = v as f32,
            "Amount" => quantia = v as f32,
            "Showicon" => icone = v != 0.0,
            nome => {
                let passou = if prob > 99.0 {
                    true
                } else if prob < 0.001 {
                    false
                } else {
                    let ok = (dado() as f64) < prob;
                    prob = if ok { 100.0 } else { 0.0 };
                    ok
                };
                if passou {
                    aplicar.push(Aplicacao {
                        nome: nome.to_string(),
                        efeito: Efeito::do_setter(nome),
                        tempo_s,
                        razao,
                        valor,
                        quantia,
                        mostra_icone: icone,
                    });
                }
            }
        }
    }
    (aplicar, nao_lidos)
}

/// Variáveis de um roteiro. `L` é o nível; `P_X` quem conjura, `V_X` a vítima, `S_X` a
/// habilidade. Talentos (`S_T0..T2`) valem 0 — o servidor não tem talentos.
pub fn variaveis<'a>(nivel: i32, jogador: &'a HashMap<&'static str, f64>, vitima: &'a HashMap<&'static str, f64>, habilidade: &'a HashMap<&'static str, f64>) -> impl Fn(&str) -> Option<f64> + 'a {
    move |n: &str| {
        if n == "L" {
            return Some(nivel as f64);
        }
        if let Some(x) = n.strip_prefix("P_") {
            return jogador.get(x).copied();
        }
        if let Some(x) = n.strip_prefix("V_") {
            return vitima.get(x).copied();
        }
        if let Some(x) = n.strip_prefix("S_") {
            return match x {
                "T0" | "T1" | "T2" => Some(0.0),
                "Rand" => Some(rand::random::<u32>() as f64 % 100.0),
                _ => habilidade.get(x).copied(),
            };
        }
        None
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn sem_vars(n: &str) -> Option<f64> {
        (n == "L").then_some(3.0)
    }

    #[test]
    fn a_expressao_do_stub_se_avalia_como_em_cpp() {
        assert_eq!(expr::avaliar("8 * L * L + 214.1 * L + 659.8", &sem_vars).map(|v| (v * 10.0).round() / 10.0), Some(1374.1));
        assert_eq!(expr::avaliar("1.0 * 5 + L", &sem_vars), Some(8.0));
        assert_eq!(expr::avaliar("L == 3 ? 100 : 0", &sem_vars), Some(100.0));
        assert_eq!(expr::avaliar("-5.5 + 7.5 * L", &sem_vars), Some(17.0));
        assert_eq!(expr::avaliar("INT(7.9) + 0.5f", &sem_vars), Some(7.5));
        assert_eq!(expr::avaliar("P_Maxhp * 2", &sem_vars), None);
    }

    fn passo(s: &str, e: &str) -> (String, String, String) {
        ("V".into(), s.into(), e.into())
    }

    /// A 10 (nível 3): Fallen 100%, Slow 90% com razão 0,15, Fix a 5 + L %. O dado que
    /// falha no Slow **zera** a probabilidade só até o próximo `SetProbability`.
    #[test]
    fn o_roteiro_rola_o_dado_e_herda_o_resultado() {
        let passos = vec![
            passo("Probability", "1.0 * 100"),
            passo("Time", "15000"),
            passo("Amount", "8 * L * L + 214.1 * L + 659.8"),
            passo("Fallen", "1"),
            passo("Probability", "1.0 * 90"),
            passo("Time", "15000"),
            passo("Ratio", "0.15"),
            passo("Slow", "1"),
            passo("Fix", "1"),
            passo("Probability", "1.0 * 5 + L"),
            passo("Time", "4000"),
            passo("Fix", "1"),
        ];
        let mut rolagens = vec![95, 0].into_iter();
        let (ap, nao) = executar_roteiro(&passos, &sem_vars, &mut || rolagens.next().unwrap());
        assert!(nao.is_empty());
        let nomes: Vec<_> = ap.iter().map(|a| (a.nome.as_str(), a.tempo_s)).collect();
        // Slow falhou (95 ≥ 90), e o Fix logo depois herdou o 0; o último Fix rolou 0 < 8.
        assert_eq!(nomes, vec![("Fallen", 15), ("Fix", 4)]);
    }

    fn filtro(e: Efeito, s: i32, razao: i32) -> Filtro {
        Filtro { efeito: e, restante_s: s, razao, fator: razao as f32 / 100.0, por_segundo: 0, contador: 0, origem: 0, icone: true }
    }

    #[test]
    fn unico_substitui_fraco_descarta_fundir_absorve() {
        let mut e = Efeitos::default();
        assert!(e.adicionar(filtro(Efeito::Slow, 10, 20)));
        assert!(e.adicionar(filtro(Efeito::Slow, 5, 50)));
        assert_eq!(e.filtros.len(), 1);
        assert_eq!(e.realce().velocidade, -50);
        assert!(e.adicionar(filtro(Efeito::Dizzy, 3, 0)));
        assert!(!e.adicionar(filtro(Efeito::Dizzy, 9, 0)));
        assert!(e.sem_acao() && e.preso() && e.selado());
        assert!(e.adicionar(filtro(Efeito::Incattack, 30, 10)));
        assert!(e.adicionar(filtro(Efeito::Incattack, 60, 25)));
        assert_eq!(e.filtros.iter().filter(|f| f.efeito == Efeito::Incattack).count(), 1);
        assert_eq!(e.realce().dano, 25);
        assert_eq!(e.icones(), vec![(3, 5), (1, 3), (32, 60)]);
        let v = e.estados_visiveis();
        assert_eq!(v[0], (1 << 4) | (1 << 5) | (1 << 1));
    }

    /// `filter_Wounded`: dano total dividido pelo tempo, tique a cada 3 s com o acumulado.
    #[test]
    fn dano_no_tempo_tica_de_tres_em_tres() {
        let mut e = Efeitos::default();
        e.adicionar(Filtro { efeito: Efeito::Toxic, restante_s: 5, razao: 0, fator: 0.0, por_segundo: 20, contador: 0, origem: 7, icone: true });
        let mut total = 0;
        for _ in 0..5 {
            for t in e.batida().0 {
                if let Tique::Dano { valor, origem } = t {
                    assert_eq!(origem, 7);
                    total += valor;
                }
            }
        }
        assert_eq!(total, 100);
        assert!(e.filtros.is_empty());
    }
}
