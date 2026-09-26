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
    Firearrow,
    /// `filter_Wingshield` (`cskill/skill/skillfilter.h:4136-4232`): um escudo que **absorve**
    /// dano até acabar e, de quebra, injeta mana a cada 3 s. A Barreira de Asa (249) o aplica
    /// com `SetAmount(60 + 75 × nível)`, `SetValue(4 + 6 × nível)` e `SetTime(20000)`
    /// (`cskill/skills/skill249.h:257-262`).
    Wingshield,
    /// `filter_Retort` (`cskill/skill/skillfilter.h:1450-1505`), a **Muralha de Espinhos**
    /// (306) do 1.2.6: todo golpe **físico corpo a corpo** que acerta devolve ao atacante
    /// `(int)(physic_damage × ratio)` — o dano bruto do golpe, antes da defesa — como um
    /// golpe mágico que sempre acerta e passa pela defesa física dele (`attack_attr =
    /// MAGIC_ATTACK`, `attack_rate` 1000). Não devolve golpe de longe (`short_range > 0`),
    /// nem golpe não físico, nem ≤ 1. O `gs` 1.2.6 faz o mesmo (`filter_Retort::AdjustDamage`,
    /// VA 0x8310e96). A 306 do 1.2.6 dá `SetRatio(0,05·L + 0,1)` por 600 s (B120).
    Retort,
    /// `filter_Retort2` (`skillfilter.h:14617-14672`), a 306 do 1.5.5: igual ao `Retort`, mas
    /// o golpe de **habilidade** usa o `value` em vez do `ratio`. Aqui só o golpe normal do
    /// monstro dispara, então o `ratio` basta.
    Retort2,
    /// `filter_Fairyform` (`cskill/skill/skillfilter.h:16819-16875`), a **Forma Sombria**
    /// (2570) do Tormentador: enquanto dura, o jogador muda de forma (`ChangeShape(1 |
    /// FORM_CLASS << 6)` → `PLAYER_CHGSHAPE`), tem o equipamento trancado
    /// (`LockEquipment`), e ganha `_speed`% de velocidade e `_defense`% de defesa
    /// (`EnhanceSpeed`, `EnhanceScaleDefense`). O roteiro dá `SetTime(16000 + 3000 × nível)`,
    /// `SetRatio(0,04 × nível)` e `SetValue(0,6 × nível)` (`cskill/skills/skill2570.h:240-243`),
    /// e `SetFairyform` os converte em `(int)(100 × ratio)` e `(int)(100 × value)`
    /// (`cskill/skill/playerwrapper.cpp:5247-5257`).
    Fairyform,
    /// `filter_Foxform` (`cskill/skill/skillfilter.h:4606-4648`, `skillfilter.cpp:389-413`), o
    /// **Chamado da Raposa** (312) da Feiticeira: sem tempo — fica até a 312 ser lançada de
    /// novo (`SetFoxform` o tira quando `GetForm() == FORM_CLASS`, `playerwrapper.cpp:2539-2551`).
    /// Na forma: `EventChange` para `FORM_CLASS`, equipamento trancado, `ImpairScaleMaxMP`
    /// (`_decmp` = 100 × ratio), `EnhanceScaleDefense` (`_incdefence` = 100 × amount),
    /// `EnhanceScaleAttack` — a **precisão** (`_incaccuracy` = 100 × probability) — e
    /// `ChangeShape(_shape | FORM_CLASS << 6)`, `_shape` = `GetValueInt()`. O roteiro da 312 dá
    /// ratio 0,35 − 0,05 × L, amount 0,3 + 0,3 × L, probability 0,5 + 0,5 × L, value 1
    /// (`cskill/skills/skill312.h:163-167`; o `gs` 1.2.6 faz o mesmo, `filter_Foxform::OnAttach`
    /// em VA 0x830b0f2). No [`Filtro`]: `razao` = mana, `escala_defesa` = defesa,
    /// `por_segundo` = precisão, `contador` = `_shape`.
    Foxform,
    /// `healing_potion_filter` / `mana_potion_filter` (`gs/potion_filter.h:6-130`): a poção
    /// não cura de uma vez — ela reparte o total pelo tempo e entrega **um pedaço por
    /// batimento de 1 s**. Não vem de roteiro de habilidade; quem cria é o uso do item.
    PocaoDeVida,
    PocaoDeMana,
    /// `filter_Rebirth` (`cskill/skill/skillfilter.h:9027-9070`), de `SetRebirth(probability,
    /// ratio)` (`playerwrapper.cpp:3589-3593`, sem dado): antes de morrer, com `probability`%
    /// de chance, cura `ratio` (0,01..1) da vida máxima, avisa `ENCHANT_RESULT` da 1085 e se
    /// desfaz (`BeforeDeath`). `UNIQUE|REMOVE_ON_DEATH|BEFORE_DEATH|TRANSFERABLE_BUFF`, ícone
    /// `HSTATE_REBIRTH` 155. No 1.5.5 o põem a 330 (Curar Mascote), 1096, 1280, 1281 e 2411; no
    /// `gs` 1.2.6 o filtro não existe.
    Rebirth,
    /// `filter_Decregiondmg` (`skillfilter.h:19899-19950`), de `SetDecregiondmg`
    /// (`playerwrapper.cpp:6044-6052`): multiplica por `1 − ratio` o golpe recebido de quem não é
    /// jogador **quando `attack_attr < 0`** (`TranslateRecvAttack`). No fonte do 1.5.5 o
    /// `attack_attr` só recebe `PHYSIC_ATTACK`, `PHYSIC_ATTACK_HIT_DEFINITE`, `MAGIC_ATTACK` ou o
    /// `attr` do stub (0..7 em todas as 3.316) — nunca negativo —, então o original nunca reduz:
    /// fica o ícone `HSTATE_DECREGIONDMG` 328 pelo tempo. `UNIQUE|REMOVE_ON_DEATH`; a máscara de
    /// bênção/maldição vem do `amount` (> 1: nenhuma — o caso da 330). Não existe no `gs` 1.2.6.
    Decregiondmg,
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
            "Firearrow" => Firearrow,
            "Wingshield" => Wingshield,
            "Retort" => Retort,
            "Retort2" => Retort2,
            "Fairyform" => Fairyform,
            "Foxform" => Foxform,
            "Rebirth" => Rebirth,
            "Decregiondmg" => Decregiondmg,
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
            // `filter_Firearrow` (`cskill/skill/skillfilter.h:4233-4290`):
            // `WEAK|BUFF|HEARTBEAT|REMOVE_ON_DEATH|TRANSLATE_SEND_MSG|TRANSFERABLE_BUFF`,
            // com `HSTATE_FIREARROW` na lista visível à equipe e `VSTATE_FIREARROW` no
            // estado. **Fraco**, não único: com um já ativo, o novo é descartado.
            Firearrow => f(Fraco, true, 70, 30),
            // `filter_Wingshield`: `UNIQUE|BUFF|HEARTBEAT|REMOVE_ON_DEATH|ADJUST_DAMAGE|
            // TRANSFERABLE_BUFF`, `HSTATE_WINGSHIELD` 69 e `VSTATE_WINGSHIELD` 29
            // (`cskill/skill/statedef.h:40,261`).
            Wingshield => f(Unico, true, 69, 29),
            // `FILTER_MASK_UNIQUE | BUFF | HEARTBEAT | REMOVE_ON_DEATH | ADJUST_DAMAGE |
            // TRANSFERABLE_BUFF`; `VSTATE_RETORT` 3 e `HSTATE_RETORT` 4 / `HSTATE_RETORT2` 253
            // (`statedef.h:11,183,451`; o `gs` 1.2.6 empurra 3 e 4, VA 0x8310f8e).
            Retort => f(Unico, true, 4, 3),
            Retort2 => f(Unico, true, 253, 3),
            // `filter_Fairyform`: `FILTER_MASK_WEAK | FILTER_MASK_HEARTBEAT` — nem bênção nem
            // maldição (o Dispersar não o tira) e **sem** `REMOVE_ON_DEATH`. O ícone é o
            // `HSTATE_FAIRYFORM` 279 (`statedef.h:477`, `InsertTeamVisibleState`); não há
            // `VSTATE`: o que o cliente desenha é a forma, pelo `PLAYER_CHGSHAPE`.
            Fairyform => Ficha { convivencia: Fraco, bencao: false, maldicao: false, icone: 279, visivel: 0 },
            // `filter_Foxform`: só `FILTER_MASK_WEAK` (`skillfilter.h:4611`) — sem batimento,
            // sem `REMOVE_ON_DEATH`. Ícone `HSTATE_FOXFORM` 75 (`statedef.h:268`; o `gs`
            // 1.2.6 empurra 0x4b), **sem parâmetro**: `InsertTeamVisibleState(state)`.
            Foxform => Ficha { convivencia: Fraco, bencao: false, maldicao: false, icone: 75, visivel: 0 },
            // Sem ícone e sem estado visual: o original não acende nenhum (`potion_filter.h`).
            PocaoDeVida | PocaoDeMana => f(Fundir, true, 0, 0),
            // Nem `BUFF` nem `DEBUFF`: o Dispersar não os tira.
            Rebirth => Ficha { convivencia: Unico, bencao: false, maldicao: false, icone: 155, visivel: 0 },
            Decregiondmg => Ficha { convivencia: Unico, bencao: false, maldicao: false, icone: 328, visivel: 0 },
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

/// `FORM_CLASS` (`cskill/skill/skill.h:84`): a forma de classe (raposa, Forma Sombria).
pub const FORMA_DE_CLASSE: u8 = 1;

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
    /// `_amount` do `filter_Wingshield`: quanto de dano o escudo ainda aguenta. Cai a cada
    /// golpe e, abaixo de 6, o filtro se apaga (`skillfilter.h:4168-4195`).
    pub absorve: f32,
    /// `_defense` do `filter_Fairyform`: porcentagem somada à defesa (`EnhanceScaleDefense`).
    /// A velocidade vai na `razao`, como nos outros realces de velocidade.
    pub escala_defesa: i32,
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
    /// `_en_percent.max_mp` (`Enhance/ImpairScaleMaxMP`), aplicado no `UpdateMana`.
    pub mana: i32,
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
                    } else if matches!(novo.efeito, Efeito::PocaoDeVida | Efeito::PocaoDeMana) {
                        // `healing_potion_filter::Merge` (`potion_filter.h:30-42`): soma o
                        // tempo e o total das duas, e reparte de novo.
                        let total = velho.por_segundo * velho.restante_s + novo.por_segundo * novo.restante_s;
                        velho.restante_s += novo.restante_s;
                        velho.por_segundo = (total / velho.restante_s.max(1)).max(1);
                        return true;
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
            // `filter_Foxform` não tem `FILTER_MASK_HEARTBEAT` nem tempo: nada a contar.
            if f.efeito == Efeito::Foxform {
                continue;
            }
            // `filter_Wounded::Heartbeat`, `filter_Hpgen/Mpgen::Heartbeat`: a cada 3 s, ou no
            // último, o acumulado.
            // A poção entrega todo segundo (`healing_potion_filter::Heartbeat`), sem o
            // acúmulo de 3 s dos filtros de habilidade.
            if matches!(f.efeito, Efeito::PocaoDeVida | Efeito::PocaoDeMana) {
                let v = f.por_segundo.min(if f.restante_s <= 1 { i32::MAX } else { f.por_segundo });
                tiques.push(if f.efeito == Efeito::PocaoDeVida { Tique::Cura(v) } else { Tique::Mana(v) });
                f.restante_s -= 1;
                continue;
            }
            // `filter_Wingshield::Heartbeat`: a cada 3 s injeta `_mpgen` **inteiro**, não o
            // acumulado (`skillfilter.h:4207-4218`).
            if f.efeito == Efeito::Wingshield {
                f.contador += 1;
                if f.contador >= 3 || 1 >= f.restante_s {
                    if f.por_segundo > 0 {
                        tiques.push(Tique::Mana(f.por_segundo));
                    }
                    f.contador -= 3;
                }
                f.restante_s -= 1;
                continue;
            }
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

    /// `filter_Wingshield::AdjustDamage` (`cskill/skill/skillfilter.h:4168-4195`).
    ///
    /// O original compara **um quinto** do golpe com o escudo. Enquanto couber ali, o dano
    /// passa a 20% e o escudo perde quatro vezes o que absorveu; quando não cabe mais, o
    /// dano é reduzido na proporção do que sobrou e o escudo zera. Abaixo de 6 o filtro se
    /// apaga.
    /// `filter_Rebirth::BeforeDeath`: com o filtro e o dado a favor (`rand() % 100 <
    /// chance`), ele se desfaz e devolve a fração da vida máxima a curar. Dado contra: nada, e o
    /// filtro sai com a morte (`REMOVE_ON_DEATH`).
    pub fn renascer(&mut self, dado: i32) -> Option<f32> {
        let i = self.filtros.iter().position(|f| f.efeito == Efeito::Rebirth)?;
        if dado >= self.filtros[i].razao {
            return None;
        }
        Some(self.filtros.remove(i).fator.clamp(0.01, 1.0))
    }

    pub fn escudo_absorve(&mut self, dano: i32) -> i32 {
        let Some(f) = self.filtros.iter_mut().find(|f| f.efeito == Efeito::Wingshield) else {
            return dano;
        };
        let quinto = dano as f32 * 0.2;
        let saida = if quinto < f.absorve {
            f.absorve -= quinto * 4.0;
            quinto
        } else if quinto > 1.0 {
            let r = 1.0 - f.absorve / quinto;
            f.absorve = 0.0;
            dano as f32 * r
        } else {
            dano as f32
        };
        if f.absorve < 6.0 {
            f.restante_s = 0;
        }
        saida.max(0.0) as i32
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

    /// `FILTER_MASK_REMOVE_ON_DEATH` — todos os portados têm, **menos as formas**: o
    /// `Fairyform` é só `WEAK | HEARTBEAT` (`skillfilter.h:16822-16825`) e acaba pelo tempo; o
    /// `Foxform` é só `WEAK` (`:4611`) e fica até a 312 ser lançada de novo.
    pub fn ao_morrer(&mut self) {
        self.filtros.retain(|f| matches!(f.efeito, Efeito::Fairyform | Efeito::Foxform));
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

    /// A forma do objeto: `(_shape, FORM_CLASS)` na Forma Sombria (`_shape` 1,
    /// `skillfilter.h:16853`) e na da raposa (`_shape` = o `value` da 312, 1), nenhuma fora
    /// delas. `FORM_CLASS` = 1 (`cskill/skill/skill.h:84`). O byte que vai ao cliente é da
    /// versão (`WorldProtocol::byte_de_forma`): 65 no 1.5.5, 1 no 1.2.6.
    pub fn forma(&self) -> Option<(u8, u8)> {
        self.filtros.iter().find_map(|f| match f.efeito {
            Efeito::Fairyform => Some((1, FORMA_DE_CLASSE)),
            Efeito::Foxform => Some((f.contador as u8, FORMA_DE_CLASSE)),
            _ => None,
        })
    }

    /// `GetForm()` (`gs/actobject.h:1058`): 0 fora de forma, `FORM_CLASS` numa forma de classe.
    /// É o que o `allow_forms` das habilidades testa (`cskill/skill/skill.cpp:128`).
    pub fn forma_atual(&self) -> u8 {
        self.forma().map_or(0, |(_, forma)| forma)
    }

    /// `SetFoxform` com a raposa já ativa: `RemoveFilter(FILTER_FOXFORM)` e volta à forma
    /// humana (`playerwrapper.cpp:2541-2546`). `true` quando havia o que tirar.
    pub fn desfazer_raposa(&mut self) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.efeito != Efeito::Foxform);
        antes != self.filtros.len()
    }

    /// `_lock_equipment` (`LockEquipment(true)` no `filter_Fairyform::OnAttach`): vestir,
    /// trocar, mover para o corpo e descartar peça são recusados com
    /// `ERR_EQUIPMENT_IS_LOCKED` (`gs/player.cpp:7874, 7991, 8077, 8258`).
    pub fn equipamento_travado(&self) -> bool {
        self.tem(Efeito::Fairyform) || self.tem(Efeito::Foxform)
    }

    /// `filter_Retort(2)::AdjustDamage`: quanto do golpe **físico corpo a corpo, normal**
    /// (dano bruto `fisico`, antes da defesa) volta ao atacante. `None` sem espinhos ou com
    /// o resultado ≤ 1 (`skillfilter.h:1480-1484`, `:14646-14650`). O teto de 1.000.000 do
    /// 1.5.5 não existe no 1.2.6 e não muda nada abaixo dele.
    pub fn espinhos(&self, fisico: i32) -> Option<i32> {
        if fisico >= 1_000_000 {
            return None;
        }
        let f = self.filtros.iter().find(|f| matches!(f.efeito, Efeito::Retort | Efeito::Retort2))?;
        let dano = (fisico as f32 * f.fator) as i32;
        (dano > 1).then_some(dano)
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
                Fairyform => {
                    r.velocidade += k;
                    r.defesa += f.escala_defesa;
                }
                // `ImpairScaleMaxMP(_decmp)`, `EnhanceScaleDefense(_incdefence)`,
                // `EnhanceScaleAttack(_incaccuracy)` (`skillfilter.cpp:393-395`).
                Foxform => {
                    r.mana -= k;
                    r.defesa += f.escala_defesa;
                    r.precisao += f.por_segundo;
                }
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
                // A raposa entra sem parâmetro (`InsertTeamVisibleState(HSTATE_FOXFORM)`):
                // [`SEM_PARAMETRO`] faz o `ICON_STATE_NOTIFY` mandar o ícone sem tempo.
                let param = if f.efeito == Efeito::Foxform { pw_protocol::packets::s2c::SEM_PARAMETRO } else { f.restante_s };
                out.push((h, param));
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
    let dano = efeitos.escudo_absorve(dano);
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
    /// O `probability` do `PlayerWrapper` na hora do setter — o `SetRebirth` o usa como chance
    /// (`(int)(probability)`).
    pub probabilidade: f32,
}

/// `PlayerWrapper` do lado da vítima: os parâmetros que os `SetX` guardam
/// (`playerwrapper.h:160-205`) e o dado (`ThrowDice`, `:170-178`), que **fixa** a
/// probabilidade em 100 ou 0 depois de rolar — o efeito seguinte sem `SetProbability`
/// herda o resultado.
/// Os efeitos que **não** consultam o dado no original.
///
/// No `cskill`, quem decide se a probabilidade entra é **cada setter**: dos 486
/// `PlayerWrapper::Set*` de `cskill/skill/playerwrapper.cpp`, 316 abrem com
/// `if (ThrowDice())` e estes 170 aplicam o filtro direto. `SetFirearrow`
/// (`playerwrapper.cpp:2333-2337`) é um deles — daí a Flecha Fulgurante (244) ser garantida,
/// embora o roteiro dela não tenha `SetProbability` (`cskill/skills/skill244.h:234-240`).
///
/// A lista foi **extraída do fonte**, não escrita à mão. `probability` nasce em zero
/// (`playerwrapper.h:61`) e `ThrowDice()` com zero devolve falso (`:169-178`): quem consulta o
/// dado sem `SetProbability` antes não aplica nada, e é assim no original.
const GARANTIDOS_SEM_DADO: &[&str] = &[
    "Absorbdamageincdefense", "Addball", "Adddefence", "Additionalattack", "Additionalheal",
    "Addmaxhp", "Addresistance", "Addskilldamage", "Airstreamlock", "Antiwater", "Apgen", "Apgen2",
    "Appendenchant", "Attachstatetoself", "Attachstatetotarget", "Attackattachstate1",
    "Attackattachstate2", "Attackattachstate3", "Attackattachstate4", "Aurabless2", "Aurabless3",
    "Auracurse2", "Auracurse4asn", "Beastieform", "Beattackattachstate1", "Beattackattachstate2",
    "Beattackattachstate3", "Beattackattachstate4", "Blessmagic", "Burningfeet", "Callupteammember",
    "Chanceofrebirth", "Charred", "Clearinvisible", "Clearinvisible2", "Comboid", "CommonCoolDown",
    "Debithurt", "Decdamagefromcrits", "Delaytransmit", "Denyattackcmd", "Devilstate", "Disappear",
    "Disturbrecover", "Dropmoneyondeath", "Earthguard", "Earthhurt", "Enmity", "Enternonpenaltypvp",
    "Entrap", "Entrap2", "Fairyform", "Fastprayincmagic", "Feathershield", "Filpball", "Firearrow",
    "Firehurt", "Fishform", "Flower1", "Flower2", "Flower3", "Flower4", "Foxform", "Freemove",
    "Freemoveapgen", "Frenetic", "Frighten", "Giant", "GiantForm", "Goldhurt", "Hardenskin",
    "Healsteal", "Homefeeling", "Immunedrop", "Incantiinvisiblepassive", "Incatkdefhp",
    "Incatkdefhp2", "Incattackondamage", "Incbow", "Incboxing", "Inccrit", "Incdagger",
    "Incdefencesmite", "Incdefensedegree", "Incearth", "Incfarnormaldmgreduce",
    "Incfarskilldmgreduce", "Incfeather", "Incfight", "Incfightproperty", "Incfire", "Incgold",
    "Inchammer", "Inchitrate", "Inchpgen", "Inchurt3", "Incinvisiblepassive", "Incmaxhpatkdfdlevel",
    "Incmpgen", "Incnearnormaldmgreduce", "Incnearskilldmgreduce", "Incpenres",
    "Incpetattackdegree", "Incpetdamage", "Incpetdefenddegree", "Incpetdefense", "Incpethp",
    "Incpetmagicdamage", "Incpetmagicdefense", "Incpetmp", "Incrange", "Incrementalhpgen",
    "Incresistmagic", "Incscimitar", "Incspear", "Incswim", "Incswimspeed", "Incsword",
    "Inctalisman", "Incwater", "Incwood", "Incwoodwaterdefense", "Insertvstate", "Ironshield",
    "Jingji", "Leavenonpenaltypvp", "Longjumptospouse", "Magicfrenetic", "MnfactionDecresist",
    "Moongod", "Panruo", "Perform", "Petsacrifice", "Physichurt", "Plantsuicide", "Powerup",
    "Queryotherinventory", "Rebirth", "Rebirth2", "Reduceresurrectexplost", "Repelonnormalattack",
    "Resurrect", "Retortmagic", "Returntown", "Sandstorm", "Shadowform", "Soulbeatback",
    "Soulretort", "Soulretort2", "Soulsealed", "Soulstun", "Specialphysichurt", "Specialslow",
    "Startcallup", "Stoneskin", "Summonpet2", "Summonplantpet", "Swiftform", "TalentData",
    "Thunderform", "Tigerform", "Transportdamagetopet", "Transportmptopet", "Vacuum", "Waterhurt",
    "Windshield", "Wingshield", "Woodhurt", "Xisui", "Yijin",
];

/// O efeito é aplicado sem passar pelo dado?
fn garantido_sem_dado(setter: &str) -> bool {
    GARANTIDOS_SEM_DADO.binary_search(&setter).is_ok()
}

pub fn executar_roteiro(
    passos: &[(String, String, String)],
    vars: &dyn Fn(&str) -> Option<f64>,
    dado: &mut dyn FnMut() -> i32,
) -> (Vec<Aplicacao>, Vec<String>) {
    // `probability` nasce zerado no `PlayerWrapper` (`playerwrapper.h:61`).
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
                // Quem não consulta o dado entra sempre — ver [`GARANTIDOS_SEM_DADO`].
                let passou = if garantido_sem_dado(nome) {
                    true
                } else if prob > 99.0 {
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
                        probabilidade: prob as f32,
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

    /// B67 — a lista de efeitos garantidos saiu do fonte e está ordenada (a busca é binária).
    #[test]
    fn a_lista_de_garantidos_esta_ordenada_e_tem_o_firearrow() {
        let mut ordenada = GARANTIDOS_SEM_DADO.to_vec();
        ordenada.sort_unstable();
        assert_eq!(ordenada, GARANTIDOS_SEM_DADO, "a lista precisa estar ordenada");
        assert!(garantido_sem_dado("Firearrow"), "SetFirearrow não consulta o dado no original");
        assert!(garantido_sem_dado("Returntown"));
        // E quem consulta continua dependendo da probabilidade.
        assert!(!garantido_sem_dado("Speedup"), "SetSpeedup abre com if (ThrowDice())");
        assert!(!garantido_sem_dado("Slow"));
    }

    /// B67 — sem `Probability`, o efeito garantido entra e o probabilístico não.
    ///
    /// É o comportamento do original: `probability` nasce em zero (`playerwrapper.h:61`) e
    /// `ThrowDice()` com zero é falso; `SetFirearrow` nem pergunta.
    #[test]
    fn sem_probability_o_garantido_entra_e_o_probabilistico_nao() {
        let passos: Vec<(String, String, String)> = vec![
            ("V".into(), "Time".into(), "600000".into()),
            ("V".into(), "Ratio".into(), "0.4".into()),
            ("V".into(), "Firearrow".into(), "1".into()),
            ("V".into(), "Slow".into(), "1".into()),
        ];
        let mut dado = || 50;
        let (aplicadas, _) = executar_roteiro(&passos, &sem_vars, &mut dado);
        let nomes: Vec<&str> = aplicadas.iter().map(|a| a.nome.as_str()).collect();
        assert_eq!(nomes, vec!["Firearrow"], "só o garantido devia entrar");
        assert_eq!(aplicadas[0].tempo_s, 600, "SetTime divide por 1000");
    }

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
        Filtro { efeito: e, restante_s: s, razao, fator: razao as f32 / 100.0, por_segundo: 0, contador: 0, origem: 0, icone: true, absorve: 0.0, escala_defesa: 0 }
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
        e.adicionar(Filtro { efeito: Efeito::Toxic, restante_s: 5, razao: 0, fator: 0.0, por_segundo: 20, contador: 0, origem: 7, icone: true, absorve: 0.0, escala_defesa: 0 });
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
