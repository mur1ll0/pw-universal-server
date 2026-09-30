//! Renascer — na cidade (`C2S::RESURRECT_IN_TOWN`, 4) e com o pergaminho no lugar
//! (`C2S::RESURRECT_BY_ITEM`, 5). B155.
//!
//! Porte de `gplayer_controller::ZombieCommandHandler` (`gs/playercmd.cpp:683-720`), das sessões
//! (`session_resurrect_in_town`/`_by_item`, `actsession.cpp:1300-1350`), de
//! `ResurrectInTown`/`ResurrectByItem` (`playercmd.cpp:64-129`) e de `gplayer_imp::Resurrect`
//! (`player.cpp:8716-8768`). O `gs` 1.2.6 é igual (`ZombieCommandHandler` VA 0x80cd1ee: sessões
//! de 0x27 e 0x63 tiques e o nível protegido 9; `ResurrectByItem` 0x80cc81c: só o pergaminho
//! 0xbe3 = 3043, erros 5 e 0x36, recarga 10, descarte tipo 10; `session_resurrect_protect`
//! 0x80bcaca: 0x64 tiques):
//!
//! 1. o pedido abre uma sessão — **39** tiques (≈ 2 s) na cidade, **99** (≈ 5 s) com o
//!    pergaminho — e só no fim, se o jogador continuar morto, renasce;
//! 2. **cidade**: `ResurrectInTown` acha o ponto do distrito (`GetTownPosition`) e
//!    `Resurrect(pos, nomove = false)`: `PLAYER_REVIVAL` tipo **0** a quem vê (ele incluído) e o
//!    `LongJump` — `NOTIFY_POS` e `OBJECT_STOP_MOVE` no mesmo mapa, troca de mapa noutro. **Era
//!    o que faltava:** o servidor mudava a posição e não mandava o `NOTIFY_POS`, e o cliente
//!    ficava de pé onde morreu;
//! 3. **pergaminho**: `ResurrectByItem` procura o 32021 e depois o 3043 (`REVIVE_SCROLL_ID2`/
//!    `REVIVE_SCROLL_ID`, `gs/config.h:84-85`; no 1.2.6 só o 3043) — senão
//!    `ERR_ITEM_NOT_IN_INVENTORY` (5) —, a recarga `COOLDOWN_INDEX_SOUL_STONE` (10) — senão
//!    `ERR_OBJECT_IS_COOLING` (54) —, arma a recarga com o `cool_time` do item, gasta um
//!    (`PLAYER_DROP_ITEM` tipo `DROP_TYPE_RESURRECT` 10) e `Resurrect(pos atual, nomove =
//!    true)`: `session_resurrect_protect` — `PLAYER_REVIVAL` tipo **1** (a animação de
//!    reviver no lugar), o `invincible_banish_filter` de `PLAYER_REBORN_PROTECT` = 5 s
//!    (estado visível 49) e, ao fim, `PLAYER_REVIVAL` tipo **2**.
//!
//! Vida e mana voltam a 10 % e a experiência perde `GetResurrectExpReduce(cultivo)` do nível,
//! salvo até o nível 9 (`LOW_PROTECT_LEVEL`) — [`crate::progressao::renascer`].

use super::jogo::erro_s2c;
use super::*;

/// `session_resurrect_in_town(pImp, param, 39)` (`playercmd.cpp:696`; 0x27 no 1.2.6).
const TIQUES_NA_CIDADE: u64 = 39;
/// `session_resurrect_by_item(pImp, param, 99)` (`playercmd.cpp:713`; 0x63 no 1.2.6).
const TIQUES_DO_PERGAMINHO: u64 = 99;
/// `PLAYER_REBORN_PROTECT` (`gs/config.h:134`), em segundos; `SetTimer(..., 5 * 20)`.
const PROTECAO_S: i32 = 5;
const MS_POR_TIQUE: u64 = 50;
/// `REVIVE_SCROLL_ID2` e `REVIVE_SCROLL_ID` (`gs/config.h:84-85`), na ordem em que o
/// `ResurrectByItem` procura. O 32021 não existe no 1.2.6 (nem nos dados dos dois realms).
const PERGAMINHOS: [u32; 2] = [32021, 3043];
/// `COOLDOWN_INDEX_SOUL_STONE` (`gs/cooldowncfg.h:74`, 10º do enum; `push 0xa` no 1.2.6).
const RECARGA_DO_PERGAMINHO: i32 = 10;
/// `DROP_TYPE_RESURRECT` (`common/protocol.h:939`).
const DESCARTE_AO_RENASCER: u8 = 10;
/// `ERR_OBJECT_IS_COOLING` (`common/protocol.h:734`; 0x36 no 1.2.6).
const OBJETO_EM_RECARGA: i32 = 54;
/// Tipos do `PLAYER_REVIVAL` (`gplayer_dispatcher::resurrect(level)`, `player.cpp:3485-3498`).
const REVIVEU_NA_CIDADE: i16 = 0;
const REVIVEU_NO_LUGAR: i16 = 1;
const FIM_DA_PROTECAO: i16 = 2;

impl BusServer {
    /// `C2S::RESURRECT_IN_TOWN` (4). A sessão roda numa tarefa própria: o tratamento dos
    /// outros comandos não espera por ela.
    pub(super) async fn renascer_na_cidade(&self, roleid: i32) {
        if !self.esta_morto(roleid).await {
            debug!("mundo: {roleid} pediu para renascer na cidade sem estar morto");
            return;
        }
        let este = self.clone();
        tokio::spawn(async move { este.sessao_na_cidade(roleid).await });
    }

    async fn sessao_na_cidade(&self, roleid: i32) {
        tokio::time::sleep(std::time::Duration::from_millis(TIQUES_NA_CIDADE * MS_POR_TIQUE)).await;
        // `EndSession`: `if(!_imp->_parent->IsZombie()) return true;`.
        let Some(r) = self.world.write().await.reviver_jogador(roleid, false) else { return };
        // `_runner->resurrect(0)` com a posição de onde morreu, e depois o `LongJump`.
        self.difundir_renascimento(roleid, REVIVEU_NA_CIDADE, r.pos_da_morte).await;
        self.transportar(roleid, r.mapa, r.destino).await;
    }

    /// `C2S::RESURRECT_BY_ITEM` (5).
    pub(super) async fn renascer_com_pergaminho(&self, roleid: i32) {
        if !self.esta_morto(roleid).await {
            debug!("mundo: {roleid} pediu o pergaminho sem estar morto");
            return;
        }
        let este = self.clone();
        tokio::spawn(async move { este.sessao_do_pergaminho(roleid).await });
    }

    async fn sessao_do_pergaminho(&self, roleid: i32) {
        tokio::time::sleep(std::time::Duration::from_millis(TIQUES_DO_PERGAMINHO * MS_POR_TIQUE)).await;
        if !self.esta_morto(roleid).await {
            return;
        }
        let dados = self.world.read().await.data_manager.clone();
        let gasto = self
            .com_contexto(roleid, |ctx| {
                let Some((id, slot)) = PERGAMINHOS
                    .iter()
                    .find_map(|&id| ctx.bolsa.primeiro_slot_com(id).map(|s| (id, s)))
                else {
                    return Err(erro_s2c::ITEM_NAO_NO_INVENTARIO);
                };
                let agora = std::time::Instant::now();
                if ctx.p.recargas.get(&RECARGA_DO_PERGAMINHO).is_some_and(|&fim| fim > agora) {
                    return Err(OBJETO_EM_RECARGA);
                }
                let ms = dados.recarga_do_pergaminho(id).unwrap_or(0).max(0);
                ctx.p.recargas.insert(RECARGA_DO_PERGAMINHO, agora + std::time::Duration::from_millis(ms as u64));
                ctx.para_mim.push(S2CGamedataSend::set_cooldown(RECARGA_DO_PERGAMINHO, ms).data);
                ctx.bolsa.tirar_do_slot(slot, 1);
                ctx.para_mim.push(
                    S2CGamedataSend::player_drop_item(0, slot as u8, 1, id as i32, DESCARTE_AO_RENASCER).data,
                );
                Ok(id)
            })
            .await;
        match gasto {
            Some(Ok(id)) => {
                let Some(r) = self.world.write().await.reviver_jogador(roleid, true) else { return };
                info!("mundo: {roleid} renasceu no lugar com o pergaminho {id}");
                self.proteger_ao_renascer(roleid, r.destino).await;
            }
            Some(Err(e)) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(e).data).await;
            }
            None => {}
        }
    }

    /// `session_resurrect_protect` (`actsession.cpp:1480-1503`): `PLAYER_REVIVAL` 1, 5 s de
    /// `invincible_banish_filter` (estado visível 49) e `PLAYER_REVIVAL` 2 ao fim.
    async fn proteger_ao_renascer(&self, roleid: i32, pos: pw_core::Vector3) {
        if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
            p.efeitos.invencivel_s = p.efeitos.invencivel_s.max(PROTECAO_S);
        }
        self.difundir_renascimento(roleid, REVIVEU_NO_LUGAR, pos).await;
        self.avisar_efeitos(roleid as i64, false).await;
        let este = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(PROTECAO_S as u64 * 1000)).await;
            let pos = este.world.read().await.players.get(&(roleid as i64)).map(|p| p.position);
            if let Some(pos) = pos {
                este.difundir_renascimento(roleid, FIM_DA_PROTECAO, pos).await;
            }
        });
    }

    /// `AutoBroadcastCSMsg(..., -1)`: a quem vê o jogador **e** a ele mesmo.
    async fn difundir_renascimento(&self, roleid: i32, tipo: i16, pos: pw_core::Vector3) {
        let pacote = S2CGamedataSend::player_revive(roleid, tipo, pos).data;
        self.enviar_ao_jogador(roleid, pacote.clone()).await;
        self.transmitir_a_quem_ve(roleid as i64, pacote).await;
    }

    async fn esta_morto(&self, roleid: i32) -> bool {
        self.world.read().await.players.get(&(roleid as i64)).is_some_and(|p| p.hp <= 0)
    }
}
