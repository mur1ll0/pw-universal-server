//! Duelo entre dois jogadores — porte de `player_duel` (`gs/playerduel.cpp`, `playerduel.h`) e do
//! `pvp_duel_filter` (`gs/duel_filter.cpp`). Igual no `gs` 1.2.6 (mesmos ids S2C 214–220/229, erros 64/66
//! e o convite de 30 s: `player_duel::PlayerDuelRequest`, VA 0x81414c0).
//!
//! Cada jogador tem o seu [`Duelo`]. As mensagens entre os dois (`SendTo<0>(GM_MSG_PLAYER_DUEL_*)`)
//! saem como [`Saida::Mensagem`] e o mundo as entrega ao outro; o que vai ao cliente sai como
//! [`Saida::Aviso`]. O batimento é o de 1 s do jogador (`gplayer_imp::OnHeartbeat` →
//! `_duel.Heartbeat`, `player.cpp:9314`).

/// `ERR_INVALID_TARGET` (`common/protocol.h:681`).
pub const ERRO_ALVO_INVALIDO: i32 = 1;
/// `ERR_CANNOT_DUEL_TWICE`.
pub const ERRO_DUELO_DUPLICADO: i32 = 64;
/// `ERR_CREATE_DUEL_FAILED`.
pub const ERRO_DUELO_FALHOU: i32 = 65;
/// `ERR_INVALID_OPERATION_IN_COMBAT`.
pub const ERRO_EM_COMBATE: i32 = 66;
/// `ERR_HERE_CAN_NOT_DUEL` (o `noduel` do mapa).
pub const ERRO_AQUI_NAO_DUELA: i32 = 84;

/// `DUEL_INVITE_TIMEOUT`, em batimentos de 1 s.
pub const PRAZO_DO_CONVITE: i32 = 30;
/// `DUEL_TIME_LIMIT`: 10 min de duelo.
pub const LIMITE_DO_DUELO: i32 = 600;
/// A distância do pedido e da resposta: `squared_distance(msg.pos) <= 400` (`player.cpp:1946`, `:1960`).
pub const ALCANCE_DO_PEDIDO_2: f32 = 400.0;
/// Longe assim, o duelo empata (`>= 100.f*100.f`, `playerduel.cpp` `Heartbeat`).
pub const DISTANCIA_DE_EMPATE_2: f32 = 100.0 * 100.0;
/// O `_mode = 5` do `FMID_DUEL_END`: o filtro fica mais 5 batimentos tirando o efeito do ex-oponente.
pub const TREGUA: i32 = 5;

/// `DUEL_REPLY_*`.
pub mod resposta {
    pub const SUCESSO: i32 = 0;
    pub const PRAZO: i32 = 1;
    pub const RECUSA: i32 = 2;
    pub const LONGE: i32 = 3;
    pub const ESTADO: i32 = 4;
}

/// `DUEL_RESULT_*` (o `param` do `GM_MSG_PLAYER_DUEL_STOP`).
pub mod resultado {
    pub const PRAZO: i32 = 0;
    pub const VITORIA: i32 = 1;
    pub const EMPATE: i32 = 2;
}

/// As mensagens entre os dois (`GM_MSG_PLAYER_DUEL_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mensagem {
    Pedido,
    Resposta(i32),
    Preparar,
    Cancelar,
    Comecar,
    Parar(i32),
}

/// O que vai ao cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aviso {
    /// `duel_recv_request(de)` → `DUEL_RECV_REQUEST` (214) só a `para`.
    PedidoRecebido { para: i64, de: i64 },
    /// `duel_reject_request(quem, motivo)` → 215 só a `para`.
    PedidoRecusado { para: i64, quem: i64, motivo: i32 },
    /// `duel_prepare(oponente, 3)` → 216 só a `para`.
    Preparar { para: i64, oponente: i64, atraso: i32 },
    /// `duel_cancel(oponente)` → 217 só a `para`.
    Cancelado { para: i64, oponente: i64 },
    /// `duel_start(oponente)` (`player.cpp:5387-5400`): `HOST_DUEL_START` (218) a `quem` e
    /// `PLAYER_DUEL_START(quem)` (229) a quem está em volta.
    Comecou { quem: i64, oponente: i64 },
    /// `duel_stop()` → `DUEL_STOP(quem)` (219) a todos em volta, ele incluído.
    Parou { quem: i64 },
    /// `duel_result(oponente, perdeu)` → `DUEL_RESULT(oponente, quem, perdeu)` (220) a todos em volta.
    Resultado { quem: i64, oponente: i64, perdeu: bool },
    /// `error_message(codigo)` só a `para`.
    Erro { para: i64, codigo: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Saida {
    Mensagem { de: i64, para: i64, msg: Mensagem },
    Aviso(Aviso),
}

/// `_invite`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Convite {
    alvo: i64,
    prazo: i32,
    convidou: bool,
    aceitou: bool,
}

/// `player_duel` de um jogador.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Duelo {
    /// `_duel_target` (0 = `XID(-1,-1)`).
    alvo: i64,
    prazo: i32,
    /// `_duel_mode`: 0 nada, 1 preparando, 2 lutando.
    modo: u8,
    convite: Convite,
    /// O `pvp_duel_filter` depois do fim: quem foi o oponente e quantos batimentos faltam.
    tregua: (i64, i32),
}

impl Duelo {
    pub fn em_duelo(&self) -> bool {
        self.modo != 0
    }
    pub fn lutando(&self) -> bool {
        self.modo == 2
    }
    /// O oponente, durante a luta (`IsDuelStarted` com o `_duel_target`).
    pub fn oponente(&self) -> Option<i64> {
        (self.modo == 2).then_some(self.alvo)
    }
    /// `pvp_duel_filter::TranslateRecvAttack` com `_mode > 0`: o golpe do ex-oponente perde o
    /// `force_attack` e não fere nos 5 s seguintes ao fim.
    pub fn em_tregua_com(&self, quem: i64) -> bool {
        self.tregua.1 > 0 && self.tregua.0 == quem
    }
    /// Nada a guardar: o mundo pode largar este estado.
    pub fn ocioso(&self) -> bool {
        *self == Duelo::default() || (self.modo == 0 && !self.convite.convidou && !self.convite.aceitou && self.tregua.1 <= 0)
    }

    /// `gplayer_imp::PlayerDuelRequest` (`player.cpp:13999-14013`) + `player_duel::PlayerDuelRequest`.
    /// `alvo_e_jogador` é o `target.IsPlayer()`.
    pub fn pedir(&mut self, eu: i64, alvo: i64, alvo_e_jogador: bool, em_combate: bool) -> Vec<Saida> {
        let erro = |c| vec![Saida::Aviso(Aviso::Erro { para: eu, codigo: c })];
        if em_combate {
            return erro(ERRO_EM_COMBATE);
        }
        if self.convite.convidou || self.convite.aceitou {
            return erro(ERRO_DUELO_DUPLICADO);
        }
        if !alvo_e_jogador {
            return erro(ERRO_ALVO_INVALIDO);
        }
        if self.em_duelo() {
            return erro(ERRO_DUELO_DUPLICADO);
        }
        self.convite = Convite { alvo, prazo: PRAZO_DO_CONVITE, convidou: true, aceitou: false };
        vec![Saida::Mensagem { de: eu, para: alvo, msg: Mensagem::Pedido }]
    }

    /// `gplayer_imp::PlayerDuelReply` + `player_duel::PlayerDuelReply` (`playerduel.cpp:40-70`).
    pub fn responder(&mut self, eu: i64, alvo: i64, alvo_e_jogador: bool, param: i32, em_combate: bool) -> Vec<Saida> {
        let erro = |c| vec![Saida::Aviso(Aviso::Erro { para: eu, codigo: c })];
        if em_combate {
            return erro(ERRO_EM_COMBATE);
        }
        if !alvo_e_jogador {
            return erro(ERRO_ALVO_INVALIDO);
        }
        let msg = Saida::Mensagem { de: eu, para: alvo, msg: Mensagem::Resposta(param) };
        if self.convite.aceitou || param != 0 {
            return vec![msg];
        }
        if self.em_duelo() {
            return erro(ERRO_DUELO_DUPLICADO);
        }
        // "3 s para a resposta chegar".
        self.convite = Convite { alvo, prazo: 3, convidou: false, aceitou: true };
        vec![msg]
    }

    /// `MsgDuelRequest` (`playerduel.cpp:72-83`), depois da conferência de distância do mundo.
    pub fn ao_receber_pedido(&mut self, eu: i64, de: i64, em_combate: bool) -> Vec<Saida> {
        if self.convite.aceitou || self.em_duelo() || em_combate {
            return vec![Saida::Mensagem { de: eu, para: de, msg: Mensagem::Resposta(resposta::ESTADO) }];
        }
        vec![Saida::Aviso(Aviso::PedidoRecebido { para: eu, de })]
    }

    /// `MsgDuelReply` (`playerduel.cpp:85-124`).
    pub fn ao_receber_resposta(&mut self, eu: i64, de: i64, param: i32, em_combate: bool) -> Vec<Saida> {
        if self.em_duelo() {
            return Vec::new();
        }
        if param != 0 {
            if self.convite.convidou && self.convite.alvo == de {
                self.convite.convidou = false;
                return vec![Saida::Aviso(Aviso::PedidoRecusado { para: eu, quem: de, motivo: param })];
            }
            if self.convite.aceitou && self.convite.alvo == de {
                self.convite.aceitou = false;
                return vec![Saida::Aviso(Aviso::Erro { para: eu, codigo: ERRO_DUELO_FALHOU })];
            }
            return Vec::new();
        }
        if !self.convite.convidou || self.convite.alvo != de || em_combate {
            // `SendTo<0>(GM_MSG_ERROR_MESSAGE, who, ERR_CREATE_DUEL_FAILED)`.
            return vec![Saida::Aviso(Aviso::Erro { para: de, codigo: ERRO_DUELO_FALHOU })];
        }
        self.convite.aceitou = false;
        self.convite.convidou = false;
        self.alvo = de;
        self.modo = 1;
        self.prazo = 3;
        vec![
            Saida::Mensagem { de: eu, para: de, msg: Mensagem::Preparar },
            Saida::Aviso(Aviso::Preparar { para: eu, oponente: de, atraso: 3 }),
        ]
    }

    /// `MsgDuelPrepare` (`playerduel.cpp:126-142`).
    pub fn ao_receber_preparo(&mut self, eu: i64, de: i64) -> Vec<Saida> {
        if self.convite.aceitou && de == self.convite.alvo {
            self.convite.aceitou = false;
            self.convite.convidou = false;
            self.alvo = de;
            self.modo = 1;
            // "10 s: espera o outro mandar o começo".
            self.prazo = 10;
            return vec![Saida::Aviso(Aviso::Preparar { para: eu, oponente: de, atraso: 3 })];
        }
        vec![Saida::Mensagem { de: eu, para: de, msg: Mensagem::Cancelar }]
    }

    /// `MsgDuelCancel` (`playerduel.cpp:144-151`).
    pub fn ao_receber_cancelamento(&mut self, eu: i64, de: i64) -> Vec<Saida> {
        if !self.em_duelo() || self.alvo != de {
            return Vec::new();
        }
        let mut s = self.limpar(eu);
        self.alvo = 0;
        s.push(Saida::Aviso(Aviso::Cancelado { para: eu, oponente: de }));
        s
    }

    /// `MsgDuelStart` (`playerduel.cpp:153-162`) → `SetDuelStartMode` + `pvp_duel_filter::OnAttach`.
    pub fn ao_receber_comeco(&mut self, eu: i64, de: i64) -> Vec<Saida> {
        if !(self.modo == 1 && self.alvo == de) {
            return vec![Saida::Mensagem { de: eu, para: de, msg: Mensagem::Cancelar }];
        }
        self.modo = 2;
        self.prazo = LIMITE_DO_DUELO;
        self.tregua = (0, 0);
        vec![Saida::Aviso(Aviso::Comecou { quem: eu, oponente: self.alvo })]
    }

    /// `MsgDuelStop` (`playerduel.cpp:164-170`).
    pub fn ao_receber_fim(&mut self, eu: i64, de: i64) -> Vec<Saida> {
        if !self.em_duelo() || self.alvo != de {
            return Vec::new();
        }
        let s = self.limpar(eu);
        self.alvo = 0;
        s
    }

    /// `Heartbeat` (`playerduel.cpp:172-258`). `oponente_ok`: o `QueryObject(_duel_target)` achou o
    /// oponente vivo a menos de 100 m.
    pub fn batimento(&mut self, eu: i64, oponente_ok: bool) -> Vec<Saida> {
        if self.tregua.1 > 0 {
            self.tregua.1 -= 1;
        }
        let mut s = Vec::new();
        if self.em_duelo() {
            self.prazo -= 1;
            if self.modo == 1 {
                if self.prazo <= 0 {
                    self.prazo = 3;
                    // `SendMessage(list[2])`: a si (com o alvo no conteúdo) e ao alvo.
                    s.push(Saida::Mensagem { de: self.alvo, para: eu, msg: Mensagem::Comecar });
                    s.push(Saida::Mensagem { de: eu, para: self.alvo, msg: Mensagem::Comecar });
                }
            } else {
                let fim = if self.prazo <= 0 {
                    Some(resultado::PRAZO)
                } else if !oponente_ok {
                    Some(resultado::EMPATE)
                } else {
                    None
                };
                if let Some(v) = fim {
                    self.prazo = 3;
                    let alvo = self.alvo;
                    s.push(Saida::Mensagem { de: alvo, para: eu, msg: Mensagem::Parar(v) });
                    s.push(Saida::Mensagem { de: eu, para: alvo, msg: Mensagem::Parar(v) });
                    s.push(Saida::Aviso(Aviso::Resultado { quem: eu, oponente: alvo, perdeu: false }));
                }
            }
        } else if self.convite.convidou {
            self.convite.prazo -= 1;
            if self.convite.prazo <= 0 {
                s.push(Saida::Aviso(Aviso::PedidoRecusado { para: eu, quem: self.convite.alvo, motivo: resposta::PRAZO }));
                self.convite.convidou = false;
            }
        } else if self.convite.aceitou {
            self.convite.prazo -= 1;
            if self.convite.prazo <= 0 {
                s.push(Saida::Aviso(Aviso::Erro { para: eu, codigo: ERRO_DUELO_FALHOU }));
                self.convite.aceitou = false;
            }
        }
        s
    }

    /// `OnDeath(pImp, duel_failed)` (`playerduel.cpp:260-302`). `perdeu_o_duelo`: a "morte" veio do
    /// oponente (`attacker_mode & PVP_DUEL`; o jogador fica com 1 de vida). Senão, morte de verdade.
    pub fn ao_morrer(&mut self, eu: i64, perdeu_o_duelo: bool) -> Vec<Saida> {
        let mut s = Vec::new();
        let alvo = self.alvo;
        if perdeu_o_duelo {
            if self.lutando() {
                s.push(Saida::Aviso(Aviso::Resultado { quem: eu, oponente: alvo, perdeu: true }));
                s.push(Saida::Mensagem { de: eu, para: alvo, msg: Mensagem::Parar(resultado::VITORIA) });
            }
            s.extend(self.limpar(eu));
            self.alvo = 0;
        } else if self.lutando() {
            s.push(Saida::Aviso(Aviso::Resultado { quem: eu, oponente: alvo, perdeu: false }));
            s.push(Saida::Mensagem { de: eu, para: alvo, msg: Mensagem::Parar(resultado::EMPATE) });
            s.extend(self.limpar(eu));
            self.alvo = 0;
        }
        s
    }

    /// `ClearDuelMode` (`playerduel.cpp:318-327`): com o filtro, `FMID_DUEL_END` → `_mode = 5` e
    /// `OnRelease` (`duel_filter.cpp`): `DuelStop` e `SendClientDuelStop` (o `DUEL_STOP`). O
    /// `ClearSpecFilter(FILTER_MASK_DEBUFF)` do `OnRelease` fica com o mundo.
    fn limpar(&mut self, eu: i64) -> Vec<Saida> {
        let estava_lutando = self.modo == 2;
        self.modo = 0;
        if estava_lutando {
            self.tregua = (self.alvo, TREGUA);
            return vec![Saida::Aviso(Aviso::Parou { quem: eu })];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Entrega as mensagens entre os dois até acabar, como o mundo faz.
    fn entregar(a: &mut Duelo, b: &mut Duelo, ida: Vec<Saida>, avisos: &mut Vec<Aviso>) {
        let mut fila = ida;
        while let Some(s) = fila.pop() {
            match s {
                Saida::Aviso(x) => avisos.push(x),
                Saida::Mensagem { de, para, msg } => {
                    let d = if para == 1 { &mut *a } else { &mut *b };
                    fila.extend(match msg {
                        Mensagem::Pedido => d.ao_receber_pedido(para, de, false),
                        Mensagem::Resposta(p) => d.ao_receber_resposta(para, de, p, false),
                        Mensagem::Preparar => d.ao_receber_preparo(para, de),
                        Mensagem::Cancelar => d.ao_receber_cancelamento(para, de),
                        Mensagem::Comecar => d.ao_receber_comeco(para, de),
                        Mensagem::Parar(_) => d.ao_receber_fim(para, de),
                    });
                }
            }
        }
    }

    #[test]
    fn pedido_aceite_preparo_luta_e_derrota() {
        let (mut a, mut b, mut av) = (Duelo::default(), Duelo::default(), Vec::new());
        let s = a.pedir(1, 2, true, false);
        entregar(&mut a, &mut b, s, &mut av);
        assert_eq!(av, vec![Aviso::PedidoRecebido { para: 2, de: 1 }]);
        av.clear();
        let s = b.responder(2, 1, true, 0, false);
        entregar(&mut a, &mut b, s, &mut av);
        assert!(av.contains(&Aviso::Preparar { para: 1, oponente: 2, atraso: 3 }));
        assert!(av.contains(&Aviso::Preparar { para: 2, oponente: 1, atraso: 3 }));
        assert!(a.em_duelo() && b.em_duelo() && !a.lutando());
        av.clear();
        // 3 batimentos depois, `a` (o que convidou) manda o começo aos dois.
        for _ in 0..3 {
            let s = a.batimento(1, true);
            entregar(&mut a, &mut b, s, &mut av);
        }
        assert!(a.lutando() && b.lutando());
        assert_eq!(a.oponente(), Some(2));
        assert!(av.contains(&Aviso::Comecou { quem: 1, oponente: 2 }) && av.contains(&Aviso::Comecou { quem: 2, oponente: 1 }));
        av.clear();
        // `b` "morre" pelo golpe de `a`: perde, os dois param, e `b` fica 5 s sem levar dano de `a`.
        let s = b.ao_morrer(2, true);
        entregar(&mut a, &mut b, s, &mut av);
        assert!(av.contains(&Aviso::Resultado { quem: 2, oponente: 1, perdeu: true }));
        assert!(av.contains(&Aviso::Parou { quem: 1 }) && av.contains(&Aviso::Parou { quem: 2 }));
        assert!(!a.em_duelo() && !b.em_duelo());
        assert!(b.em_tregua_com(1) && a.em_tregua_com(2));
        for _ in 0..TREGUA {
            b.batimento(2, true);
        }
        assert!(!b.em_tregua_com(1) && b.ocioso());
    }

    #[test]
    fn recusa_prazo_combate_e_longe() {
        let (mut a, mut b, mut av) = (Duelo::default(), Duelo::default(), Vec::new());
        assert_eq!(a.pedir(1, 2, true, true), vec![Saida::Aviso(Aviso::Erro { para: 1, codigo: ERRO_EM_COMBATE })]);
        assert_eq!(a.pedir(1, 99, false, false), vec![Saida::Aviso(Aviso::Erro { para: 1, codigo: ERRO_ALVO_INVALIDO })]);
        let s = a.pedir(1, 2, true, false);
        entregar(&mut a, &mut b, s, &mut av);
        assert_eq!(a.pedir(1, 2, true, false), vec![Saida::Aviso(Aviso::Erro { para: 1, codigo: ERRO_DUELO_DUPLICADO })]);
        av.clear();
        let s = b.responder(2, 1, true, resposta::RECUSA, false);
        entregar(&mut a, &mut b, s, &mut av);
        assert_eq!(av, vec![Aviso::PedidoRecusado { para: 1, quem: 2, motivo: resposta::RECUSA }]);
        // Sem resposta, 30 batimentos e o pedido cai por prazo.
        let mut a = Duelo::default();
        a.pedir(1, 2, true, false);
        let mut fim = Vec::new();
        for _ in 0..PRAZO_DO_CONVITE {
            fim = a.batimento(1, true);
        }
        assert_eq!(fim, vec![Saida::Aviso(Aviso::PedidoRecusado { para: 1, quem: 2, motivo: resposta::PRAZO })]);
    }

    #[test]
    fn longe_demais_empata() {
        let (mut a, mut b, mut av) = (Duelo::default(), Duelo::default(), Vec::new());
        let s = a.pedir(1, 2, true, false);
        entregar(&mut a, &mut b, s, &mut av);
        let s = b.responder(2, 1, true, 0, false);
        entregar(&mut a, &mut b, s, &mut av);
        for _ in 0..3 {
            let s = a.batimento(1, true);
            entregar(&mut a, &mut b, s, &mut av);
        }
        av.clear();
        let s = b.batimento(2, false);
        entregar(&mut a, &mut b, s, &mut av);
        assert!(av.contains(&Aviso::Resultado { quem: 2, oponente: 1, perdeu: false }));
        assert!(!a.em_duelo() && !b.em_duelo());
    }
}
