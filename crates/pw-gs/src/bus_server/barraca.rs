//! B363 — a barraca pessoal no barramento: C2S 76/77/78/84, a barraca como "NPC" para quem a abre
//! (`SEVNPC_HELLO`, `SEVNPC_GET_CONTENT`, `SEVNPC_SERVE` 19/21) e o estado de barraca.
//! As regras puras estão em [`crate::barraca`].

use super::jogo::erro_s2c;
use super::*;
use crate::barraca::{self as b, erro, Lado, RecusaDeAbrir, RecusaDoNegocio};
use crate::economia::{Bolsa, TAMANHO_DA_BOLSA};

/// `DROP_TYPE_TRADEAWAY` (`common/protocol.h`, enum dos descartes).
const DESCARTE_NA_TROCA: u8 = 9;

impl BusServer {
    async fn bolsa_de(&self, roleid: i32) -> Bolsa {
        let itens = self
            .itens()
            .await
            .list_by_container(roleid, ContainerType::Inventory)
            .await
            .unwrap_or_default();
        Bolsa::nova(roleid, ContainerType::Inventory, TAMANHO_DA_BOLSA, itens)
    }

    /// O bloco do item como o `OWN_ITEM_INFO` o manda (o mesmo `item_content` que a vitrine leva).
    fn conteudo_do_item(item: &pw_core::ItemRecord, dados: &pw_data_loader::GameDataManager) -> Vec<u8> {
        // `OWN_ITEM_INFO`: cabeçalho 2, `where` 1, `index` 1, `type` 4, `expire` 4, `proc` 4,
        // `count` 4, `crc` 2 e o `u16` do tamanho — o conteúdo começa no byte 24.
        Self::info_de(0, item, dados).get(24..).map(<[u8]>::to_vec).unwrap_or_default()
    }

    /// Quem tem barraca aberta.
    pub(super) async fn tem_barraca(&self, roleid: i32) -> bool {
        self.world.read().await.players.get(&(roleid as i64)).is_some_and(|p| p.barraca.is_some())
    }

    /// C2S `OPEN_BOOTH` (76; `OPEN_PERSONAL_MARKET`, `playercmd.cpp:2855-2889`) →
    /// `PlayerOpenPersonalMarket` (`playerstall.cpp:18-163`).
    pub(super) async fn abrir_barraca(&self, roleid: i32, corpo: &[u8], envio: &EnvioAoCliente) {
        let r = self.sub.regras_da_barraca();
        let Some(pedido) = b::ler_pedido(corpo, &r) else {
            self.responder(roleid, S2CGamedataSend::error_message(erro_s2c::ERRO_FATAL).data, envio).await;
            return;
        };
        let bolsa = self.bolsa_de(roleid).await;
        let agora = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let resultado = {
            let mut guarda = self.world.write().await;
            let mundo = &mut *guarda;
            let dados = Arc::clone(&mundo.data_manager);
            let na_agua = mundo.players.get(&(roleid as i64)).is_some_and(|p| mundo.esta_na_agua(p.position));
            let tem_mascote = mundo.mascote_de(roleid as i64).is_some();
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            if p.efeitos.gm_invisivel {
                // `IsInvisible` → `ERR_OPERTION_DENYED_IN_INVISIBLE` (`playercmd.cpp:2878-2882`).
                Err(Some(138))
            } else if p.hp <= 0
                || p.barraca.is_some()
                || p.sentado
                // `CanSitDown`: só no chão (`move_mode == MODE_GROUND`, `actobject.h:99`).
                || p.voando
                || na_agua
                // `_cur_session || _session_list.size()`: golpe, conjuração ou coleta em curso.
                || p.ataque.is_some()
                || p.conjuracao.is_some()
                || p.coleta.is_some()
            {
                Err(None)
            } else if p.ultima_barraca_s == agora {
                // `if (sys_time == _stall_trade_timer) return false` (`playerstall.cpp:113-117`).
                Err(None)
            } else {
                let proximo = b::proximo_trade_id(p.seq_da_barraca);
                let dinheiro = p.money.clamp(0, u32::MAX as i64) as u32;
                match b::abrir(&pedido, &r, &bolsa, &dados, dinheiro, proximo) {
                    Ok(barraca) => {
                        p.ultima_barraca_s = agora;
                        p.seq_da_barraca = proximo;
                        let montaria = p.montaria.take();
                        let pacote_dono = self.sub.self_open_booth(&barraca.para_o_dono()).data;
                        let anuncio =
                            S2CGamedataSend::player_open_booth(roleid, barraca.market_id(), &barraca.nome).data;
                        info!("mundo: {roleid} abriu barraca ({} itens, id {proximo})", barraca.itens.len());
                        p.barraca = Some(barraca);
                        // `_petman.RecallPet(this)` (`playerstall.cpp:122`).
                        if tem_mascote {
                            mundo.recolher_mascote(roleid as i64, 0);
                        }
                        Ok((pacote_dono, anuncio, montaria))
                    }
                    Err(RecusaDeAbrir::ItemInvalido) => Err(Some(erro::ITEM_INVALIDO)),
                    Err(RecusaDeAbrir::NaoPode) => Err(None),
                }
            }
        };
        match resultado {
            Ok((pacote_dono, anuncio, montaria)) => {
                if let Some(m) = montaria {
                    // A montaria é um mascote no original: o `RecallPet` a recolhe também.
                    if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
                        p.montaria = Some(m);
                    }
                    self.desmontar(roleid, m, envio).await;
                }
                self.responder(roleid, pacote_dono, envio).await;
                // `begin_personal_market`: `AutoBroadcastCSMsg(..., pPlayer->ID.id)` — a quem vê, sem ele.
                self.transmitir_a_quem_ve(roleid as i64, anuncio).await;
            }
            Err(Some(138)) => {
                self.responder(roleid, S2CGamedataSend::error_message(138).data, envio).await;
            }
            Err(antes) => {
                if let Some(e) = antes {
                    self.responder(roleid, S2CGamedataSend::error_message(e).data, envio).await;
                }
                self.responder(roleid, S2CGamedataSend::error_message(erro::NAO_PODE_ABRIR).data, envio).await;
            }
        }
    }

    /// `CancelPersonalMarket` (`playerstall.cpp:177-196`): C2S 77, sair do jogo, e no 1.2.6 o teto
    /// de dinheiro do vendedor. `PLAYER_CLOSE_BOOTH` a quem vê **e** a ele (`AutoBroadcastCSMsg(..., -1)`).
    pub(super) async fn fechar_barraca(&self, roleid: i32) {
        let fechou = self
            .world
            .write()
            .await
            .players
            .get_mut(&(roleid as i64))
            .is_some_and(|p| p.barraca.take().is_some());
        if !fechou {
            return;
        }
        info!("mundo: {roleid} fechou a barraca");
        let pacote = S2CGamedataSend::player_close_booth(roleid).data;
        self.enviar_ao_jogador(roleid, pacote.clone()).await;
        self.transmitir_a_quem_ve(roleid as i64, pacote).await;
    }

    /// C2S `QUERY_BOOTH_NAME` (78; `playercmd.cpp:2891-2915`): `{u16 count; int list[]}`, no máximo
    /// 128; cada jogador da lista com barraca responde o `BOOTH_NAME` (171) (`MarketHandler`,
    /// `GM_MSG_QUERY_PERSONAL_MARKET_NAME`, `playerstall.cpp:284-286`).
    pub(super) async fn nomes_de_barraca(&self, roleid: i32, corpo: &[u8], envio: &EnvioAoCliente) {
        let n = corpo.get(..2).map(|c| u16::from_le_bytes([c[0], c[1]]) as usize).unwrap_or(0);
        if n == 0 || corpo.len() != 2 + n * 4 {
            self.responder(roleid, S2CGamedataSend::error_message(erro_s2c::ERRO_FATAL).data, envio).await;
            return;
        }
        let ids: Vec<i64> = (0..n.min(128)).map(|i| i32::from_le_bytes(corpo[2 + i * 4..6 + i * 4].try_into().unwrap()) as i64).collect();
        let nomes: Vec<Vec<u8>> = {
            let mundo = self.world.read().await;
            ids.iter()
                .filter_map(|id| {
                    let p = mundo.players.get(id)?;
                    let bar = p.barraca.as_ref()?;
                    Some(S2CGamedataSend::booth_name(p.role_id, bar.market_id(), &bar.nome).data)
                })
                .collect()
        };
        for pacote in nomes {
            self.responder(roleid, pacote, envio).await;
        }
    }

    /// C2S `OPEN_BOOTH_TEST` (84; `TEST_PERSONAL_MARKET`, `playercmd.cpp:2947-2970`) →
    /// `PlayerTestPersonalMarket` (`playerstall.cpp:165-175`): `OPEN_BOOTH_TEST` (188) se pode.
    pub(super) async fn testar_barraca(&self, roleid: i32, envio: &EnvioAoCliente) {
        let pode = {
            let mundo = self.world.read().await;
            mundo.players.get(&(roleid as i64)).is_some_and(|p| {
                !p.efeitos.gm_invisivel
                    && p.hp > 0
                    && p.barraca.is_none()
                    && !p.sentado
                    && !p.voando
                    && !mundo.esta_na_agua(p.position)
                    && p.ataque.is_none()
                    && p.conjuracao.is_none()
                    && p.coleta.is_none()
            })
        };
        if pode {
            self.responder(roleid, S2CGamedataSend::open_booth_test().data, envio).await;
        }
    }

    /// A barraca de outro como "NPC": `GM_MSG_SERVICE_HELLO` a menos de 6 m → `SERVICE_GREETING`
    /// (`playerstall.cpp:393-399`). `true` se `alvo` é jogador com barraca (e o olá foi tratado).
    pub(super) async fn ola_a_barraca(&self, roleid: i32, alvo: i64, envio: &EnvioAoCliente) -> bool {
        let perto = {
            let mut mundo = self.world.write().await;
            let Some(dono) = mundo.players.get(&alvo).filter(|p| p.barraca.is_some()).map(|p| p.position) else {
                return false;
            };
            let Some(eu) = mundo.players.get_mut(&(roleid as i64)) else { return true };
            let perto = eu.position.distance(&dono) < b::DISTANCIA_DE_SERVICO;
            if perto {
                eu.npc_em_conversa = Some(alvo);
            }
            perto
        };
        if perto {
            self.responder(roleid, S2CGamedataSend::npc_greeting(alvo as i32).data, envio).await;
        }
        true
    }

    /// A barraca com que `roleid` conversa, se for uma.
    async fn barraca_em_conversa(&self, roleid: i32) -> Option<i32> {
        let mundo = self.world.read().await;
        let alvo = mundo.players.get(&(roleid as i64))?.npc_em_conversa?;
        mundo.players.get(&alvo).filter(|p| p.barraca.is_some()).map(|p| p.role_id)
    }

    /// `GM_MSG_SERVICE_QUIERY_CONTENT` (`playerstall.cpp:463-525`): a vitrine (169). `true` se o
    /// interlocutor é uma barraca.
    pub(super) async fn conteudo_da_barraca(&self, roleid: i32) -> bool {
        let Some(dono) = self.barraca_em_conversa(roleid).await else { return false };
        let bolsa = self.bolsa_de(dono).await;
        let pacote = {
            let mundo = self.world.read().await;
            let dados = Arc::clone(&mundo.data_manager);
            let (Some(eu), Some(d)) = (mundo.players.get(&(roleid as i64)), mundo.players.get(&(dono as i64))) else {
                return true;
            };
            if eu.position.distance(&d.position) > b::DISTANCIA_DE_SERVICO {
                None
            } else {
                d.barraca.as_ref().map(|bar| {
                    let itens = b::vitrine(bar, &bolsa, |i| Self::conteudo_do_item(i, &dados));
                    S2CGamedataSend::player_booth_info(dono, bar.trade_id, &itens).data
                })
            }
        };
        match pacote {
            Some(p) => {
                self.enviar_ao_jogador(roleid, p).await;
            }
            None => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro::SERVICO_INDISPONIVEL).data).await;
            }
        }
        true
    }

    /// `GM_MSG_SERVICE_REQUEST` na barraca (`playerstall.cpp:400-461`): o serviço 19 (comprar dela)
    /// ou 21 (vender a ela); outro serviço, longe demais ou recusa → `ERR_SERVICE_UNAVILABLE`.
    /// `true` se o interlocutor é uma barraca.
    pub(super) async fn servico_da_barraca(&self, roleid: i32, servico: i32, c: &[u8]) -> bool {
        let Some(dono) = self.barraca_em_conversa(roleid).await else { return false };
        let indisponivel = S2CGamedataSend::error_message(erro::SERVICO_INDISPONIVEL).data;
        let r = self.sub.regras_da_barraca();
        let pedido = b::ler_pedido_de_compra(c, &r);
        if (servico != b::SERVICO_BARRACA_VENDE && servico != b::SERVICO_BARRACA_COMPRA) || pedido.is_none() {
            self.enviar_ao_jogador(roleid, indisponivel).await;
            return true;
        }
        let pedido = pedido.expect("conferido");
        let (mut bolsa_dono, mut bolsa_fregues) = (self.bolsa_de(dono).await, self.bolsa_de(roleid).await);
        let resultado = {
            let mut guarda = self.world.write().await;
            let mundo = &mut *guarda;
            let dados = Arc::clone(&mundo.data_manager);
            let (Some(f), Some(d)) = (mundo.players.get(&(roleid as i64)), mundo.players.get(&(dono as i64))) else {
                return true;
            };
            // `pBuyer->IsZombie()`, a distância de 6 m e o `PLAYER_STATE_NORMAL` do freguês
            // (`CheckMarketTradeRequire`/`PurchaseRequire`).
            let fora = f.hp <= 0
                || d.hp <= 0
                || f.position.distance(&d.position) > b::DISTANCIA_DE_SERVICO
                || f.barraca.is_some()
                || f.sentado;
            if fora {
                Err(RecusaDoNegocio::Recusa(None))
            } else {
                let mut barraca = d.barraca.clone().expect("barraca em conversa");
                let mut dinheiro_d = d.money.clamp(0, u32::MAX as i64) as u32;
                let mut dinheiro_f = f.money.clamp(0, u32::MAX as i64) as u32;
                let feito = if servico == b::SERVICO_BARRACA_VENDE {
                    b::comprar_da_barraca(
                        &mut barraca,
                        &pedido,
                        &r,
                        &dados,
                        Lado { id: dono, bolsa: &mut bolsa_dono, dinheiro: &mut dinheiro_d },
                        Lado { id: roleid, bolsa: &mut bolsa_fregues, dinheiro: &mut dinheiro_f },
                    )
                } else {
                    b::vender_a_barraca(
                        &mut barraca,
                        &pedido,
                        &dados,
                        Lado { id: dono, bolsa: &mut bolsa_dono, dinheiro: &mut dinheiro_d },
                        Lado { id: roleid, bolsa: &mut bolsa_fregues, dinheiro: &mut dinheiro_f },
                    )
                };
                if feito.is_ok() {
                    if let Some(d) = mundo.players.get_mut(&(dono as i64)) {
                        d.barraca = Some(barraca);
                        d.money = dinheiro_d as i64;
                    }
                    if let Some(f) = mundo.players.get_mut(&(roleid as i64)) {
                        f.money = dinheiro_f as i64;
                    }
                }
                feito.map(|n| (n, dinheiro_d, dinheiro_f))
            }
        };
        match resultado {
            Err(RecusaDoNegocio::Recusa(antes)) => {
                if let Some(e) = antes {
                    self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(e).data).await;
                }
                self.enviar_ao_jogador(roleid, indisponivel).await;
            }
            Err(RecusaDoNegocio::FechaABarraca) => {
                self.fechar_barraca(dono).await;
                self.enviar_ao_jogador(roleid, indisponivel).await;
            }
            Ok((negocio, dinheiro_d, dinheiro_f)) => {
                let itens = self.itens().await;
                for bolsa in [&mut bolsa_dono, &mut bolsa_fregues] {
                    if let Err(e) = bolsa.gravar(&itens).await {
                        warn!("mundo: barraca de {dono}: não consegui gravar a bolsa: {e}");
                    }
                }
                let repo = self.repo().await;
                let total = negocio.total as i64;
                let (ganha, paga) = if servico == b::SERVICO_BARRACA_VENDE { (dono, roleid) } else { (roleid, dono) };
                if let Err(e) = repo.add_money(ganha, total).await {
                    warn!("mundo: barraca: não consegui gravar o dinheiro de {ganha}: {e}");
                }
                if let Err(e) = repo.deduct_money(paga, total).await {
                    warn!("mundo: barraca: não consegui gravar o dinheiro de {paga}: {e}");
                }
                let teto = b::TETO_DE_DINHEIRO;
                if servico == b::SERVICO_BARRACA_VENDE {
                    // `DoPlayerMarketTrade` (`playerstall.cpp:776-874`): o `trade_away_item` de cada
                    // item ao dono, o dinheiro dele, o `PURCHASE_ITEM` ao comprador e o sucesso aos dois.
                    for &(slot, tipo, n) in &negocio.sairam {
                        self.enviar_ao_jogador(dono, S2CGamedataSend::booth_sell_item(slot as i16, tipo, n, roleid).data).await;
                    }
                    self.enviar_ao_jogador(dono, self.sub.get_own_money(dinheiro_d, teto).data).await;
                    self.enviar_ao_jogador(roleid, self.sub.purchase_item_da_barraca(negocio.total, 0, &negocio.entraram).data)
                        .await;
                } else {
                    // `DoPlayerMarketPurchase` (`playerstall.cpp:876-975`): o freguês vê os itens saírem
                    // (`PLAYER_DROP_ITEM` tipo 9), o dono recebe o `PURCHASE_ITEM` tipo 1 e o freguês o
                    // dinheiro dele.
                    for &(slot, tipo, n) in &negocio.sairam {
                        self.enviar_ao_jogador(roleid, self.sub.player_drop_item(0, slot as u8, n, tipo, DESCARTE_NA_TROCA).data)
                            .await;
                    }
                    self.enviar_ao_jogador(dono, self.sub.purchase_item_da_barraca(negocio.total, 1, &negocio.entraram).data)
                        .await;
                    self.enviar_ao_jogador(roleid, self.sub.get_own_money(dinheiro_f, teto).data).await;
                }
                self.enviar_ao_jogador(dono, S2CGamedataSend::booth_trade_success(roleid).data).await;
                self.enviar_ao_jogador(roleid, S2CGamedataSend::booth_trade_success(dono).data).await;
                info!("mundo: barraca de {dono}: negócio de {} com {roleid} (serviço {servico})", negocio.total);
            }
        }
        true
    }
}
