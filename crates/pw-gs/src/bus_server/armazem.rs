//! Armazém do personagem (B147) — `session_use_trashbox`, o serviço de NPC 15 e os C2S 55–61.
//!
//! Porte de `gs/serviceprovider.cpp:2013-2070` (abrir), `gs/actsession.cpp:1212-1246` (a sessão
//! que fecha com qualquer outra ação), `gs/playercmd.cpp:2389-2600` (os pedidos) e
//! `gs/player.cpp:7379-7779` (as operações). Só o armazém do personagem (`IL_TRASH_BOX`, 16
//! slots, `TRASHBOX_BASE_SIZE` de `gs/config.h:17`); o de materiais (`IL_TRASH_BOX2`) começa com
//! 0 slots (`playertrashbox.h`: `_box2(item::BACKPACK, 0)`) e vai vazio; o da conta, a senha e a
//! expansão ficam para depois.

use super::jogo::erro_s2c;
use super::*;
use crate::comandos::{PedidoDoArmazem, IL_TRASH_BOX};
use crate::economia::{mover_entre, trocar_entre, Bolsa, TAMANHO_DA_BOLSA};

/// `TRASHBOX_BASE_SIZE` (`gs/config.h:17`).
pub const TAMANHO_DO_ARMAZEM: usize = 16;
/// `TRASHBOX_MONEY_CAPACITY` (`gs/config.h:78`).
pub const TETO_DO_ARMAZEM: i64 = 2_000_000_000;
/// `MONEY_CAPACITY_BASE` (`gs/config.h:79`) — o teto do dinheiro na bolsa.
const TETO_DA_BOLSA: i64 = 2_000_000_000;
/// `IL_TRASH_BOX2` (`gs/player_imp.h:1834`).
const IL_TRASH_BOX2: u8 = 4;

/// Erros de `common/protocol.h:679-740` usados aqui (contados a partir de `// 30`, `:710`).
mod erro {
    pub const OUTRA_SESSAO: i32 = 33;
    pub const SENHA_NAO_CONFERE: i32 = 35;
    pub const ARMAZEM_FECHADO: i32 = 36;
}

/// O que uma operação do armazém devolve: o pacote de resposta (antes do `armazem` da versão)
/// ou um erro para `ERROR_MESSAGE`; `None` quando o original só retorna calado.
enum Resposta {
    Pacote(S2CGamedataSend),
    Erro(i32),
    Nada,
}

impl BusServer {
    /// `GP_NPCSEV_TRASHBOX_OPEN` (15) — `trashbox_open_executor` (`serviceprovider.cpp:2026-2069`):
    /// `{ u32 passwd_size; char passwd[] }`; tamanho errado é ignorado; com sessão em curso
    /// `ERR_OTHER_SESSION_IN_EXECUTE`; senha que não confere `ERR_PASSWD_NOT_MATCH`. Sem senha
    /// guardada, `CheckPassword` só aceita a vazia (`playertrashbox.cpp:33-40`).
    pub(super) async fn abrir_armazem(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let Ok(tamanho) = r.u32() else { return };
        if conteudo.len() != 4 + tamanho as usize {
            debug!("mundo: {roleid} pediu o armazém com senha de tamanho incoerente");
            return;
        }
        let erro = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            if p.ataque.is_some() || p.produzindo.is_some() || p.coleta.is_some() || p.armazem_aberto {
                Some(erro::OUTRA_SESSAO)
            } else if tamanho != 0 {
                Some(erro::SENHA_NAO_CONFERE)
            } else {
                p.armazem_aberto = true;
                None
            }
        };
        let pacote = match erro {
            Some(e) => S2CGamedataSend::error_message(e),
            None => {
                info!("mundo: {roleid} abriu o armazém");
                self.sub.trashbox_open(TAMANHO_DO_ARMAZEM as u16)
            }
        };
        self.responder(roleid, pacote.data, envio).await;
    }

    /// `session_use_trashbox::EndSession` → `TrashBoxClose` (`actsession.cpp:1232-1241`,
    /// `player.cpp:7394-7406`): qualquer ação nova encerra a sessão e manda `TRASHBOX_CLOSE`.
    pub(super) async fn fechar_armazem(&self, roleid: i32) {
        let estava = {
            let mut mundo = self.world.write().await;
            match mundo.players.get_mut(&(roleid as i64)) {
                Some(p) => std::mem::replace(&mut p.armazem_aberto, false),
                None => false,
            }
        };
        if estava {
            debug!("mundo: {roleid} fechou o armazém");
            self.enviar_ao_jogador(roleid, self.sub.armazem(S2CGamedataSend::trashbox_close()).data)
                .await;
        }
    }

    /// Os C2S 55–61 (`playercmd.cpp:2389-2600`).
    pub(super) async fn pedido_do_armazem(&self, roleid: i32, id: u16, payload: &[u8], envio: &EnvioAoCliente) {
        let Some(pedido) = PedidoDoArmazem::ler(id, payload, self.sub.armazem_do_126()) else {
            // `sizeof(cmd) != size` → `ERR_FATAL_ERR`.
            self.responder(roleid, S2CGamedataSend::error_message(erro_s2c::ERRO_FATAL).data, envio)
                .await;
            return;
        };
        // O cliente congelou os slots ao mandar o pedido: o original os destrava antes de tudo
        // (`unlock_inventory_slot`, `playercmd.cpp:2428-2429`, `:2464-2465`, `:2500-2501`…).
        let destravar: Vec<(u8, u8)> = match pedido {
            PedidoDoArmazem::Trocar { onde, a, b } => vec![(onde, a), (onde, b)],
            PedidoDoArmazem::Mover { onde, src, dest, .. } => vec![(onde, src), (onde, dest)],
            PedidoDoArmazem::TrocarComBolsa { onde, armazem, bolsa }
            | PedidoDoArmazem::ParaBolsa { onde, armazem, bolsa, .. }
            | PedidoDoArmazem::ParaArmazem { onde, armazem, bolsa, .. } => vec![(onde, armazem), (0, bolsa)],
            PedidoDoArmazem::Info { .. } | PedidoDoArmazem::Dinheiro { .. } => vec![],
        };
        for (onde, slot) in destravar {
            self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(onde, slot as u16).data, envio)
                .await;
        }
        let aberto = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .is_some_and(|p| p.armazem_aberto);
        // Só o armazém do personagem: `where` fora de `IL_TRASH_BOX`/`IL_TRASH_BOX2`, o da conta
        // (sem `_user_trash_box_open_flag`) ou o armazém fechado → `ERR_TRASH_BOX_NOT_OPEN`.
        let onde_ok = match pedido {
            PedidoDoArmazem::Info { conta, .. } | PedidoDoArmazem::Dinheiro { conta, .. } => !conta,
            PedidoDoArmazem::Trocar { onde, .. }
            | PedidoDoArmazem::Mover { onde, .. }
            | PedidoDoArmazem::TrocarComBolsa { onde, .. }
            | PedidoDoArmazem::ParaBolsa { onde, .. }
            | PedidoDoArmazem::ParaArmazem { onde, .. } => onde == IL_TRASH_BOX || onde == IL_TRASH_BOX2,
        };
        if !aberto || !onde_ok {
            self.responder(roleid, S2CGamedataSend::error_message(erro::ARMAZEM_FECHADO).data, envio)
                .await;
            return;
        }
        match pedido {
            PedidoDoArmazem::Info { detalhe, .. } => self.info_do_armazem(roleid, detalhe, envio).await,
            PedidoDoArmazem::Dinheiro { da_bolsa, do_armazem, .. } => {
                self.dinheiro_do_armazem(roleid, da_bolsa, do_armazem, envio).await
            }
            outro => self.item_do_armazem(roleid, outro, envio).await,
        }
    }

    /// `PlayerGetTrashBoxInfo` (`player.cpp:7423-7446`): a lista do armazém, a do armazém de
    /// materiais (1.5.5, 0 slots) e `TRASHBOX_WEALTH`. O `detail` pede o `item_info` de cada um.
    async fn info_do_armazem(&self, roleid: i32, detalhe: bool, envio: &EnvioAoCliente) {
        let itens = self.itens().await;
        let lista = itens
            .list_by_container(roleid, ContainerType::Storehouse)
            .await
            .unwrap_or_default();
        let dinheiro = self.repo().await.dinheiro_do_armazem(roleid).await.unwrap_or(0);
        let mut saida = vec![S2CGamedataSend::own_ivtr_from_items(IL_TRASH_BOX, TAMANHO_DO_ARMAZEM as u8, &lista).data];
        if !self.sub.armazem_do_126() {
            // O `gs` 1.2.6 só manda o `IL_TRASH_BOX` (`PlayerGetTrashBoxInfo`, `push 3`, VA 0x8071c1b).
            saida.push(S2CGamedataSend::own_ivtr_from_items(IL_TRASH_BOX2, 0, &[]).data);
        }
        if detalhe {
            let dados = self.world.read().await.data_manager.clone();
            for i in &lista {
                saida.push(Self::info_de(IL_TRASH_BOX, i, &dados));
            }
        }
        saida.push(
            self.sub
                .armazem(S2CGamedataSend::trashbox_wealth(dinheiro.clamp(0, u32::MAX as i64) as u32))
                .data,
        );
        for d in saida {
            self.responder(roleid, d, envio).await;
        }
    }

    /// As operações de item (56–60), sobre a bolsa e o armazém carregados do banco.
    async fn item_do_armazem(&self, roleid: i32, pedido: PedidoDoArmazem, envio: &EnvioAoCliente) {
        let itens = self.itens().await;
        let (Ok(bolsa), Ok(guardado)) = (
            itens.list_by_container(roleid, ContainerType::Inventory).await,
            itens.list_by_container(roleid, ContainerType::Storehouse).await,
        ) else {
            warn!("mundo: não li a bolsa/armazém de {roleid}");
            return;
        };
        let mut bolsa = Bolsa::nova(roleid, ContainerType::Inventory, TAMANHO_DA_BOLSA, bolsa);
        // O armazém de materiais tem 0 slots: todo índice nele está fora (`ERR_FATAL_ERR`).
        let tamanho = |onde: u8| if onde == IL_TRASH_BOX { TAMANHO_DO_ARMAZEM } else { 0 };
        let dados = self.world.read().await.data_manager.clone();
        let pilha = |b: &Bolsa, s: usize| {
            b.item_no_slot(s).map(|i| dados.limite_de_pilha(i.item_id)).unwrap_or(1)
        };
        let fatal = Resposta::Erro(erro_s2c::ERRO_FATAL);
        let onde_de = |p: &PedidoDoArmazem| match *p {
            PedidoDoArmazem::Trocar { onde, .. }
            | PedidoDoArmazem::Mover { onde, .. }
            | PedidoDoArmazem::TrocarComBolsa { onde, .. }
            | PedidoDoArmazem::ParaBolsa { onde, .. }
            | PedidoDoArmazem::ParaArmazem { onde, .. } => onde,
            _ => IL_TRASH_BOX,
        };
        let mut armazem = Bolsa::nova(roleid, ContainerType::Storehouse, tamanho(onde_de(&pedido)), guardado);

        let resposta = match pedido {
            // `PlayerExchangeTrashItem` (`player.cpp:7544-7556`).
            PedidoDoArmazem::Trocar { a, b, .. } => {
                if armazem.trocar(a as usize, b as usize) {
                    Resposta::Pacote(S2CGamedataSend::exg_trashbox_item(a, b))
                } else {
                    fatal
                }
            }
            // `PlayerMoveTrashItem` (`player.cpp:7558-7572`): `MoveItem` falso → `ERR_FATAL_ERR`.
            PedidoDoArmazem::Mover { src, dest, quantidade, .. } => {
                let p = pilha(&armazem, src as usize);
                match armazem.mover(src as usize, dest as usize, quantidade, p) {
                    Some(n) => Resposta::Pacote(S2CGamedataSend::move_trashbox_item(src, dest, n)),
                    None => fatal,
                }
            }
            // `PlayerExchangeTrashInv` (`player.cpp:7603-7650`).
            PedidoDoArmazem::TrocarComBolsa { armazem: t, bolsa: i, .. } => {
                if trocar_entre(&mut armazem, t as usize, &mut bolsa, i as usize) {
                    Resposta::Pacote(S2CGamedataSend::exg_trashbox_ivtr(t, i))
                } else {
                    fatal
                }
            }
            // `PlayerTrashItemToInv` (`player.cpp:7652-7683`).
            PedidoDoArmazem::ParaBolsa { armazem: t, bolsa: i, quantidade, .. } => {
                let p = pilha(&armazem, t as usize);
                match mover_entre(&mut armazem, t as usize, &mut bolsa, i as usize, quantidade, p) {
                    Some(n) => Resposta::Pacote(S2CGamedataSend::trash_item_to_ivtr(t, i, n)),
                    None => fatal,
                }
            }
            // `PlayerInvItemToTrash` (`player.cpp:7685-7725`): índice fora do armazém retorna calado.
            PedidoDoArmazem::ParaArmazem { bolsa: i, armazem: t, quantidade, .. } => {
                if t as usize >= armazem.slots.len() {
                    Resposta::Nada
                } else {
                    let p = pilha(&bolsa, i as usize);
                    match mover_entre(&mut bolsa, i as usize, &mut armazem, t as usize, quantidade, p) {
                        Some(n) => Resposta::Pacote(S2CGamedataSend::ivtr_item_to_trash(i, t, n)),
                        None => fatal,
                    }
                }
            }
            _ => Resposta::Nada,
        };

        if let Resposta::Pacote(_) = resposta {
            for b in [&mut bolsa, &mut armazem] {
                if let Err(e) = b.gravar(&itens).await {
                    warn!("mundo: não gravei o armazém de {roleid}: {e}");
                }
            }
        }
        self.enviar_resposta(roleid, resposta, envio).await;
    }

    /// `PlayerExchangeTrashMoney` (`player.cpp:7727-7779`): um dos dois valores, nunca os dois
    /// nem nenhum, e nunca mais do que há (`ERR_FATAL_ERR`). Guardar corta no teto do armazém;
    /// tirar exige caber na bolsa (`CheckIncMoney`, senão `ERR_INVENTORY_IS_FULL`).
    async fn dinheiro_do_armazem(&self, roleid: i32, da_bolsa: u32, do_armazem: u32, envio: &EnvioAoCliente) {
        let repo = self.repo().await;
        let Ok(guardado) = repo.dinheiro_do_armazem(roleid).await else {
            warn!("mundo: não li o dinheiro do armazém de {roleid}");
            return;
        };
        let (resposta, novo) = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            let (bolsa_v, arm_v) = (da_bolsa as i64, do_armazem as i64);
            match calcular_dinheiro(p.money, guardado, bolsa_v, arm_v) {
                Ok((delta_bolsa, delta_armazem)) => {
                    p.money += delta_bolsa;
                    (
                        Resposta::Pacote(S2CGamedataSend::exg_trash_money(delta_bolsa as i32, delta_armazem as i32)),
                        Some((guardado + delta_armazem, delta_bolsa)),
                    )
                }
                Err(e) => (Resposta::Erro(e), None),
            }
        };
        if let Some((valor, delta_bolsa)) = novo {
            if let Err(e) = repo.gravar_dinheiro_do_armazem(roleid, valor).await {
                warn!("mundo: não gravei o dinheiro do armazém de {roleid}: {e}");
            }
            if let Err(e) = repo.add_money(roleid, delta_bolsa).await {
                warn!("mundo: não gravei o dinheiro de {roleid}: {e}");
            }
        }
        self.enviar_resposta(roleid, resposta, envio).await;
    }

    async fn enviar_resposta(&self, roleid: i32, resposta: Resposta, envio: &EnvioAoCliente) {
        let dados = match resposta {
            Resposta::Pacote(p) => self.sub.armazem(p).data,
            Resposta::Erro(e) => S2CGamedataSend::error_message(e).data,
            Resposta::Nada => return,
        };
        self.responder(roleid, dados, envio).await;
    }
}

/// A conta de `PlayerExchangeTrashMoney`: devolve `(delta da bolsa, delta do armazém)` ou o erro.
pub(crate) fn calcular_dinheiro(na_bolsa: i64, guardado: i64, da_bolsa: i64, do_armazem: i64) -> Result<(i64, i64), i32> {
    if (da_bolsa != 0) == (do_armazem != 0) || da_bolsa > na_bolsa || do_armazem > guardado {
        return Err(erro_s2c::ERRO_FATAL);
    }
    if da_bolsa != 0 {
        let delta = da_bolsa.min(TETO_DO_ARMAZEM - guardado).max(0);
        Ok((-delta, delta))
    } else {
        // `CheckIncMoney` (`player.cpp:13870-13877`): a bolsa tem de comportar tudo (`_money_capacity`,
        // `MONEY_CAPACITY_BASE` sem expansão).
        if na_bolsa + do_armazem > TETO_DA_BOLSA {
            return Err(erro_s2c::BOLSA_CHEIA);
        }
        Ok((do_armazem, -do_armazem))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dinheiro_do_armazem_segue_o_original() {
        // Os dois ou nenhum, e mais do que há: `ERR_FATAL_ERR`.
        assert_eq!(calcular_dinheiro(100, 0, 10, 10), Err(erro_s2c::ERRO_FATAL));
        assert_eq!(calcular_dinheiro(100, 0, 0, 0), Err(erro_s2c::ERRO_FATAL));
        assert_eq!(calcular_dinheiro(100, 0, 101, 0), Err(erro_s2c::ERRO_FATAL));
        assert_eq!(calcular_dinheiro(100, 5, 0, 6), Err(erro_s2c::ERRO_FATAL));
        // Guardar e tirar.
        assert_eq!(calcular_dinheiro(100, 5, 40, 0), Ok((-40, 40)));
        assert_eq!(calcular_dinheiro(100, 5, 0, 5), Ok((5, -5)));
        // Guardar corta no teto do armazém.
        assert_eq!(calcular_dinheiro(100, TETO_DO_ARMAZEM - 30, 100, 0), Ok((-30, 30)));
        // Tirar além do teto da bolsa: `ERR_INVENTORY_IS_FULL`.
        assert_eq!(calcular_dinheiro(TETO_DA_BOLSA - 1, 5, 0, 5), Err(erro_s2c::BOLSA_CHEIA));
    }
}
