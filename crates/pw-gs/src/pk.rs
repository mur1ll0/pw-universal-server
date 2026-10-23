//! B363 — agressor (nome rosa) e pária (nome vermelho): `gs/playerinvade.h`, `gs/playerinvade.cpp`.
//!
//! Três estados (`INVADER_LVL_0/1/2`, `actobject.h:429-431`), cada um com o seu controle:
//! - **branco** (0): ferir em PvP quem não é vermelho deixa **rosa** por 60 s
//!   (`GM_MSG_PLAYER_BECOME_INVADER` com 60, `actobject.cpp:669/713/787`); matar um branco deixa
//!   **vermelho** (`GM_MSG_PLAYER_BECOME_PARIAH`, `player.cpp:7209-7213`);
//! - **rosa** (1): cada golpe renova os 60 s; um batimento de 1 s desconta e no fim volta a branco
//!   (`INVADER_FADE`);
//! - **vermelho** (2): 7.200 s por morte (`PARIAH_TIME_PER_KILL`, `config.h:71`), até 100 mortes;
//!   1 s a menos por batimento e 72 ao matar monstro de nível ≥ o seu (`PARIAH_TIME_REDUCE`,
//!   `player.cpp:1359-1364`); o grau (0, 1, 2) sobe aos 14.400 e 72.000 s (`IncPariahState`,
//!   `playertemplate.h:732-741`).
//!
//! E a perda na morte (`player_ctrl_*::OnDeath`): quantos itens da bolsa e do corpo caem, sorteados
//! pelas tabelas de cada estado (`RandSelect(prop, 10)`), que mudam por versão
//! ([`pw_protocol::TabelasDePerda`]).

use pw_protocol::TabelasDePerda;

pub const BRANCO: u8 = 0;
pub const ROSA: u8 = 1;
pub const VERMELHO: u8 = 2;
/// `SendTo<0>(GM_MSG_PLAYER_BECOME_INVADER, …, 60)`; `if(time <= 0) time = 60`.
pub const TEMPO_DE_AGRESSOR: i32 = 60;
/// `PARIAH_TIME_PER_KILL`, `PARIAH_TIME_REDUCE` e `MAX_PARIAH_TIME` (`gs/config.h:71-73`).
pub const TEMPO_POR_MORTE: i32 = 7200;
pub const REDUCAO_POR_MONSTRO: i32 = 72;
pub const TEMPO_MAXIMO: i32 = TEMPO_POR_MORTE * 100;
/// `MAX_INVADER_ENTRY_COUNT` (a lista dos que o agressor feriu): o tamanho não muda nada visível.
pub const DEFENSORES_MAXIMOS: usize = 32;

/// O que avisar a quem vê (e ao próprio).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvisoPk {
    /// `INVADER_RISE` (117).
    Rosa,
    /// `PARIAH_RISE` (118) com o grau.
    Vermelho(u8),
    /// `INVADER_FADE` (119).
    Branco,
    /// `PARIAH_DURATION` (230) só ao próprio.
    Duracao(i32),
}

/// O `player_invade` (`playerinvade.h:62-74`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EstadoPk {
    pub estado: u8,
    pub tempo_de_agressor: i32,
    pub tempo_de_paria: i32,
    pub grau: u8,
    pub mortes: i32,
    pub defensores: Vec<i32>,
}

/// `IncPariahState(cur, t)` (`playertemplate.h:732-741`): o primeiro limiar acima de `t` a partir do
/// grau atual; acima de todos, 0 (como o original).
pub fn grau_do_tempo(atual: u8, t: i32) -> u8 {
    let limiares = [TEMPO_POR_MORTE * 2, TEMPO_POR_MORTE * 10, TEMPO_POR_MORTE * 100];
    (atual as usize..limiares.len()).find(|&i| t < limiares[i]).unwrap_or(0) as u8
}

impl EstadoPk {
    pub fn branco(&self) -> bool {
        self.estado == BRANCO
    }

    /// `BecomeInvader(who, time)`: branco vira rosa; rosa renova; vermelho nada.
    pub fn tornar_agressor(&mut self, ferido: i32, tempo: i32) -> Vec<AvisoPk> {
        let tempo = if tempo <= 0 { TEMPO_DE_AGRESSOR } else { tempo };
        match self.estado {
            BRANCO => {
                self.estado = ROSA;
                self.defensores.clear();
                self.inserir_defensor(ferido);
                self.tempo_de_agressor = tempo;
                vec![AvisoPk::Rosa]
            }
            ROSA => {
                self.inserir_defensor(ferido);
                self.tempo_de_agressor = tempo;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// `BecomePariah()`: branco ou rosa viram vermelho com 7.200 s; vermelho soma 7.200 (até o teto)
    /// e sobe de grau.
    pub fn tornar_paria(&mut self) -> Vec<AvisoPk> {
        match self.estado {
            BRANCO | ROSA => {
                self.mortes = if self.estado == BRANCO { 1 } else { self.mortes + 1 };
                self.estado = VERMELHO;
                self.tempo_de_paria = TEMPO_POR_MORTE;
                self.grau = grau_do_tempo(0, self.tempo_de_paria);
                self.defensores.clear();
                vec![AvisoPk::Vermelho(self.grau), AvisoPk::Duracao(self.tempo_de_paria)]
            }
            _ => {
                let t = self.tempo_de_paria + TEMPO_POR_MORTE;
                self.tempo_de_paria = t.min(TEMPO_MAXIMO);
                self.mortes += 1;
                let mut v = Vec::new();
                // `IncPariahState(_pariah_state, t)` com o `t` antes do teto.
                let novo = grau_do_tempo(self.grau, t);
                if novo != self.grau {
                    self.grau = novo;
                    v.push(AvisoPk::Vermelho(novo));
                }
                v.push(AvisoPk::Duracao(self.tempo_de_paria));
                v
            }
        }
    }

    /// O batimento de 1 s (`OnHeartbeat`): o rosa desconta e volta a branco; o vermelho desconta 1.
    pub fn batimento(&mut self) -> Vec<AvisoPk> {
        match self.estado {
            ROSA => {
                self.tempo_de_agressor -= 1;
                if self.tempo_de_agressor <= 0 {
                    self.estado = BRANCO;
                    self.tempo_de_agressor = 0;
                    return vec![AvisoPk::Branco];
                }
                Vec::new()
            }
            VERMELHO => self.reduzir_paria(1),
            _ => Vec::new(),
        }
    }

    /// `ReducePariah(t)` (`playerinvade.cpp`): só o vermelho; no fim volta a branco e zera as mortes;
    /// a duração só é avisada quando a redução é maior que 1 (o monstro).
    pub fn reduzir_paria(&mut self, t: i32) -> Vec<AvisoPk> {
        if self.estado != VERMELHO {
            return Vec::new();
        }
        self.tempo_de_paria -= t;
        let mut v = Vec::new();
        if self.tempo_de_paria <= 0 {
            self.estado = BRANCO;
            self.tempo_de_paria = 0;
            self.defensores.clear();
            self.mortes = 0;
            self.grau = 0;
            v.push(AvisoPk::Branco);
        } else {
            let novo = grau_do_tempo(0, self.tempo_de_paria);
            if novo != self.grau {
                self.grau = novo;
                v.push(AvisoPk::Vermelho(novo));
            }
        }
        if t > 1 {
            v.push(AvisoPk::Duracao(self.tempo_de_paria));
        }
        v
    }

    fn inserir_defensor(&mut self, id: i32) {
        if self.defensores.contains(&id) {
            return;
        }
        if self.defensores.len() >= DEFENSORES_MAXIMOS {
            self.defensores.remove(0);
        }
        self.defensores.push(id);
    }
}

/// `abase::RandSelect(prop, 10)`: o primeiro índice em que a soma das probabilidades passa do
/// sorteio `r` (0..1); é a quantidade de itens.
pub fn sortear(prop: &[f32; 10], r: f32) -> usize {
    let mut acc = 0.0f32;
    for (i, &p) in prop.iter().enumerate() {
        acc += p;
        if r < acc {
            return i;
        }
    }
    prop.len() - 1
}

/// Quantos itens da bolsa e do corpo caem na morte (`player_ctrl_*::OnDeath`). `por_jogador`: o
/// último golpe foi de jogador; `pvp`: a chave de PvP da vítima (`_pvp_enable_flag`). Sem PvP, só
/// a tabela de monstro do jogador branco do 1.2.6 ([`TabelasDePerda::sem_pvp`]).
pub fn perda_na_morte(
    t: &TabelasDePerda,
    pk: &EstadoPk,
    por_jogador: bool,
    pvp: bool,
    r_bolsa: f32,
    r_corpo: f32,
) -> (usize, usize) {
    if !pvp {
        return match (&t.sem_pvp, por_jogador) {
            (Some(p), false) => (sortear(p, r_bolsa), 0),
            _ => (0, 0),
        };
    }
    let g = pk.grau.min(2) as usize;
    match (pk.estado, por_jogador) {
        (BRANCO, true) => (sortear(&t.branco_bolsa, r_bolsa), sortear(&t.branco_corpo, r_corpo)),
        (BRANCO, false) => (sortear(&t.monstro_branco, r_bolsa), 0),
        (ROSA, true) => (sortear(&t.rosa_bolsa, r_bolsa), sortear(&t.rosa_corpo, r_corpo)),
        (ROSA, false) => (sortear(&t.monstro_rosa, r_bolsa), 0),
        (_, true) => (sortear(&t.vermelho_bolsa[g], r_bolsa), sortear(&t.vermelho_corpo[g], r_corpo)),
        (_, false) => (sortear(&t.monstro_vermelho[g], r_bolsa), 0),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn branco_rosa_vermelho_e_de_volta() {
        let mut pk = EstadoPk::default();
        assert_eq!(pk.tornar_agressor(9, 60), vec![AvisoPk::Rosa]);
        assert_eq!((pk.estado, pk.tempo_de_agressor, pk.defensores.clone()), (ROSA, 60, vec![9]));
        for _ in 0..59 {
            assert!(pk.batimento().is_empty());
        }
        assert_eq!(pk.batimento(), vec![AvisoPk::Branco], "60 s depois volta a branco");
        assert_eq!(pk.tornar_paria(), vec![AvisoPk::Vermelho(0), AvisoPk::Duracao(7200)]);
        assert_eq!((pk.estado, pk.mortes), (VERMELHO, 1));
        assert!(pk.tornar_agressor(3, 60).is_empty(), "vermelho não vira rosa");
        assert_eq!(pk.tornar_paria(), vec![AvisoPk::Vermelho(1), AvisoPk::Duracao(14_400)], "aos 14.400 s, grau 1");
        assert_eq!(pk.reduzir_paria(REDUCAO_POR_MONSTRO), vec![AvisoPk::Vermelho(0), AvisoPk::Duracao(14_328)]);
        assert!(pk.batimento().is_empty(), "a redução de 1 s não avisa");
        assert_eq!(pk.reduzir_paria(20_000), vec![AvisoPk::Branco, AvisoPk::Duracao(0)]);
        assert_eq!((pk.estado, pk.mortes, pk.tempo_de_paria), (BRANCO, 0, 0));
    }

    #[test]
    fn o_grau_segue_os_limiares_do_original() {
        assert_eq!(grau_do_tempo(0, 7_200), 0);
        assert_eq!(grau_do_tempo(0, 14_400), 1);
        assert_eq!(grau_do_tempo(0, 72_000), 2);
        assert_eq!(grau_do_tempo(0, 720_000), 0, "acima de todos, 0 — como o IncPariahState");
        assert_eq!(grau_do_tempo(1, 7_000), 1, "não desce na subida");
    }

    #[test]
    fn a_perda_sai_da_tabela_do_estado() {
        let t155 = TabelasDePerda::DO_155;
        let t126 = TabelasDePerda::DO_126;
        let branco = EstadoPk::default();
        assert_eq!(perda_na_morte(&t155, &branco, true, true, 0.5, 0.5), (0, 0), "1.5.5: branco não perde");
        assert_eq!(perda_na_morte(&t126, &branco, true, true, 0.995, 0.5), (1, 0), "1.2.6: branco perde 1 com 1,1 %");
        assert_eq!(perda_na_morte(&t126, &branco, false, false, 0.995, 0.5), (1, 0), "1.2.6: sem PvP, a tabela de monstro");
        assert_eq!(perda_na_morte(&t155, &branco, false, false, 0.995, 0.5), (0, 0), "1.5.5: sem PvP não perde");
        let rosa = EstadoPk { estado: ROSA, ..Default::default() };
        assert_eq!(perda_na_morte(&t155, &rosa, true, true, 0.85, 0.0), (1, 0), "rosa: 0,80 | 0,21");
        let vermelho = EstadoPk { estado: VERMELHO, grau: 2, ..Default::default() };
        assert_eq!(perda_na_morte(&t155, &vermelho, true, true, 0.5, 0.5), (3, 1), "grau 2: 0,6 em 3; corpo 0,6 em 1");
    }
}
