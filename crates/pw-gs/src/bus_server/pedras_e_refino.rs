//! Incrustar pedra (serviço 10), remover as pedras (11), refinar (35) e furar (47) — B163; transferir refino (45) e
//! furar acessório (96) — B363 (`refine_transmit_service_executor` e `make_slot_for_decoration_executor`,
//! `serviceprovider.cpp:4849-4995`; nenhum dos dois existe no `gs` 1.2.6, como o 47).
//!
//! Porte dos executores do `cgame/gs/serviceprovider.cpp`: `install_executor` (`:1316-1391`),
//! `uninstall_executor` (`:1393-1450`), `refine_service_executor` (`:3748-3800`) e
//! `make_slot_executor` (`:4912-4950`), com `gplayer_imp::EmbedChipToEquipment`
//! (`player.cpp:10577-10596`), `item_list::ClearEmbed` (`item_list.cpp:188-213`),
//! `gplayer_imp::RefineItemAddon` (`player.cpp:11677-11810`) e `gplayer_imp::ItemMakeSlot`
//! (`player.cpp:20660-20693`). A regra pura está em [`crate::refino`].
//!
//! Cada pedido passa pelas três etapas do original:
//!
//! 1. `SendRequest` do jogador — tamanho, item no slot com o tipo que o cliente disse (senão
//!    recusa **calada**) e, no refino, a recarga de 1 s (`ERR_OBJECT_IS_COOLING`);
//! 2. o NPC (`service_npc`, `servicenpc.cpp:56-75`) — sem o serviço, `ERR_SERVICE_UNAVILABLE`;
//! 3. `OnServe` do jogador — a operação.
//!
//! **1.2.6:** os executores 10, 11 e 35 do `gs` 1.2.6 têm os mesmos tamanhos (12, 8 e 12 B:
//! `cmp [ebp+0x18], 0xc` em VA 0x810b132, `cmp [ebp+0x1c], 8` em 0x810b4ca, `cmp ..., 0xc` em
//! 0x8110158), os mesmos erros e os mesmos comandos de resposta; não há trava de segurança, o
//! talismã não confere vínculo nem grau ([`WorldProtocol::talisma_confere_vinculo_e_grau`]) e
//! não existe furar ([`WorldProtocol::furar_existe`]).

use super::*;
use crate::refino::{self, erro, Ajustes, ResultadoDoRefino};
use pw_core::{AddonDoItem, ConteudoDeEquipamento, ItemRecord};
use pw_data_loader::refino::{Familia, Pedra};
use pw_data_loader::GameDataManager;
use rand::Rng;

/// `DROP_TYPE_TASK` 3 (o `TakeAwayCommonItem` do furo) e `DROP_TYPE_USE` 11 (o `RemoveItems`
/// do refino, `push 0xb` no 1.2.6) — `common/protocol.h:928-942`.
const DESCARTE_DE_MISSAO: u8 = 3;
const DESCARTE_POR_USO: u8 = 11;

/// O conteúdo de equipamento de um item da bolsa, com a família e o grau. Item sem octetos
/// gravados (anterior ao B151) sai do molde, como o `OWN_ITEM_INFO` o mostra.
pub(super) fn bloco_do_equipamento(item: &ItemRecord, dados: &GameDataManager) -> Option<(Vec<u8>, pw_core::FichaDoEquipamento)> {
    let ficha = dados.equipamentos.ficha(item.item_id)?;
    let octetos = if item.octets.is_empty() {
        ConteudoDeEquipamento::novo(ficha.clone(), item.durability as i32, item.max_durability as i32).escrever()
    } else {
        // A durabilidade da coluna: quem regrava o bloco não devolve a antiga aos octetos.
        BusServer::octetos_atuais(item)
    };
    Some((octetos, ficha))
}

/// Os addons que a pedra põe na família: os gravados no item (`stone_item::Load`, duas listas —
/// arma e armadura —, `item/item_stone.cpp:26-45`) ou, sem octetos, os que `generate_stone`
/// sortearia (`generate_item_temp.h:839-910`: um addon de `id_addon_damage` e um de
/// `id_addon_defence`).
pub(super) fn addons_da_pedra(item: &ItemRecord, pedra: &Pedra, familia: Familia, dados: &GameDataManager) -> Vec<AddonDoItem> {
    let lista = match familia {
        Familia::Arma => 0,
        Familia::Armadura => 1,
        Familia::Acessorio => return Vec::new(),
    };
    if let Some(listas) = listas_gravadas(&item.octets) {
        return listas[lista].clone();
    }
    let id = if lista == 0 { pedra.addon_na_arma } else { pedra.addon_na_armadura };
    (id > 0).then(|| crate::geracao::gerar_addon(dados, id)).flatten().into_iter().collect()
}

/// `[i32 n, addons...]` duas vezes, fechando no último byte.
fn listas_gravadas(b: &[u8]) -> Option<[Vec<AddonDoItem>; 2]> {
    if b.is_empty() {
        return None;
    }
    let mut i = 0usize;
    let le = |i: &mut usize| -> Option<i32> {
        let v = i32::from_le_bytes(b.get(*i..*i + 4)?.try_into().ok()?);
        *i += 4;
        Some(v)
    };
    let mut listas: [Vec<AddonDoItem>; 2] = [Vec::new(), Vec::new()];
    for l in &mut listas {
        let n = le(&mut i)?;
        if !(0..=128).contains(&n) {
            return None;
        }
        for _ in 0..n {
            let tipo = le(&mut i)? as u32;
            let args = (0..(tipo & 0x6000) >> 13).map(|_| le(&mut i)).collect::<Option<Vec<_>>>()?;
            l.push(AddonDoItem { tipo, args });
        }
    }
    (i == b.len()).then_some(listas)
}

impl BusServer {
    /// Os serviços do NPC com quem o jogador conversa.
    async fn servicos_em_conversa(&self, roleid: i32) -> Option<pw_data_loader::servicos::ServicosDoNpc> {
        let mundo = self.world.read().await;
        let p = mundo.players.get(&(roleid as i64))?;
        let npc = p.npc_em_conversa.and_then(|id| mundo.npcs.get(&id))?.template_id;
        mundo.data_manager.servicos_de_npc.get(&npc).cloned()
    }

    /// `GP_NPCSEV_EMBED` (10): `{ u16 chip_idx; u16 equip_idx; int chip_type; int equip_type; }`,
    /// 12 bytes (`install_executor::player_request`, `#pragma pack(1)`; o cliente manda o mesmo,
    /// `c2s_SendCmdNPCSevEmbed`, `EC_SendC2SCmds.cpp:3307-3343`).
    pub(super) async fn incrustar_pedra(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let (Ok(chip), Ok(equip), Ok(tipo_chip), Ok(tipo_equip)) = (r.u16(), r.u16(), r.i32(), r.i32()) else { return };
        if conteudo.len() != 12 {
            warn!("mundo: pedido de incrustar de {roleid} com {} B (esperados 12)", conteudo.len());
            return;
        }
        let tem = self.servicos_em_conversa(roleid).await.is_some_and(|s| s.incrustar);
        let (chip, equip) = (chip as usize, equip as usize);
        let feito = self
            .com_contexto(roleid, |ctx| {
                // `SendRequest`: os dois slots com os tipos ditos.
                let confere = |s: usize, t: i32| ctx.bolsa.item_no_slot(s).is_some_and(|i| i.item_id as i32 == t);
                if !confere(chip, tipo_chip) || !confere(equip, tipo_equip) {
                    return Err(None);
                }
                if !tem {
                    return Err(Some(erro::SERVICO_INDISPONIVEL));
                }
                // `OnServe`: a pedra e o preço dela.
                let Some(pedra) = ctx.dados.refino.pedras.get(&(tipo_chip as u32)).copied() else {
                    return Err(Some(erro::NAO_INCRUSTA));
                };
                if ctx.p.money < pedra.preco_de_incrustar as i64 {
                    return Err(Some(erro::SEM_DINHEIRO));
                }
                // `EmbedItem`: pedra cabe, e `InsertTo` acha um furo vazio.
                let Some(alvo) = ctx.dados.refino.equipamentos.get(&(tipo_equip as u32)).copied() else {
                    return Err(Some(erro::NAO_INCRUSTA));
                };
                if chip == equip || !refino::pedra_cabe(&pedra, alvo.familia, alvo.grau) {
                    return Err(Some(erro::NAO_INCRUSTA));
                }
                let item_pedra = ctx.bolsa.item_no_slot(chip).cloned().ok_or(None)?;
                let addons = addons_da_pedra(&item_pedra, &pedra, alvo.familia, ctx.dados);
                let mut item = ctx.bolsa.item_no_slot(equip).cloned().ok_or(None)?;
                let Some((bloco, ficha)) = bloco_do_equipamento(&item, ctx.dados) else {
                    return Err(Some(erro::NAO_INCRUSTA));
                };
                let pedras = &ctx.dados.refino.pedras;
                let cor_e_grau = |id: i32| pedras.get(&(id as u32)).map(|p| (p.cor, p.grau)).unwrap_or((0, 0));
                let Some((novo, true)) = ConteudoDeEquipamento::alterar_rabo(&bloco, &ficha, |c| {
                    let ok = refino::incrustar(&mut c.furos, &mut c.addons, tipo_chip as u32, &addons);
                    if ok {
                        c.mascara_das_pedras = refino::mascara_das_pedras(alvo.familia, &c.furos, c.mascara_das_pedras, cor_e_grau);
                    }
                    ok
                }) else {
                    return Err(Some(erro::NAO_INCRUSTA));
                };
                item.octets = novo;
                item.sockets = furos_de(&item.octets, &ficha);
                ctx.bolsa.por(equip, Some(item));
                // `DecAmount(source, 1)`: o cliente tira a pedra sozinho ao ler o `EMBED_ITEM`.
                ctx.bolsa.tirar_do_slot(chip, 1);
                ctx.para_mim.push(S2CGamedataSend::embed_item(chip as u8, equip as u8).data);
                ctx.gastar_dinheiro(pedra.preco_de_incrustar as i64);
                ctx.para_mim.push(S2CGamedataSend::spend_money(pedra.preco_de_incrustar.max(0) as u32).data);
                ctx.mudou = true;
                Ok(pedra.preco_de_incrustar)
            })
            .await;
        self.relatar(roleid, "incrustou", tipo_chip, feito, envio).await;
    }

    /// `GP_NPCSEV_CLEAR_TESSERA` (11): `{ size_t equip_idx; int equip_type; }`, 8 bytes
    /// (`uninstall_executor::player_request`; `c2s_SendCmdNPCSevClearEmbeddedChip`).
    pub(super) async fn remover_pedras(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let (Ok(equip), Ok(tipo)) = (r.u32(), r.i32()) else { return };
        if conteudo.len() != 8 {
            warn!("mundo: pedido de remover pedras de {roleid} com {} B (esperados 8)", conteudo.len());
            return;
        }
        let tem = self.servicos_em_conversa(roleid).await.is_some_and(|s| s.remover_pedras);
        let equip = equip as usize;
        let feito = self
            .com_contexto(roleid, |ctx| {
                let mut item = ctx.bolsa.item_no_slot(equip).filter(|i| i.item_id as i32 == tipo).cloned().ok_or(None)?;
                if !tem {
                    return Err(Some(erro::SERVICO_INDISPONIVEL));
                }
                let (bloco, ficha) = bloco_do_equipamento(&item, ctx.dados).ok_or(None)?;
                let familia = ctx.dados.refino.equipamentos.get(&item.item_id).map(|e| e.familia).ok_or(None)?;
                let pedras = &ctx.dados.refino.pedras;
                // `ClearEmbed`: soma o `uninstall_price` de cada pedra; sem pedra ou sem dinheiro,
                // recusa calada.
                let dinheiro = ctx.p.money;
                let (novo, custo) = ConteudoDeEquipamento::alterar_rabo(&bloco, &ficha, |c| {
                    let gravadas: Vec<i32> = c.furos.iter().copied().filter(|&f| f > 0 && pedras.contains_key(&(f as u32))).collect();
                    let custo: i64 = gravadas.iter().map(|f| pedras[&(*f as u32)].preco_de_remover as i64).sum();
                    if gravadas.is_empty() || dinheiro < custo {
                        return None;
                    }
                    refino::limpar_pedras(&mut c.furos, &mut c.addons);
                    c.mascara_das_pedras = refino::mascara_das_pedras(familia, &c.furos, c.mascara_das_pedras, |_| (0, 0));
                    Some(custo)
                })
                .ok_or(None)?;
                let custo = custo.ok_or(None)?;
                item.octets = novo;
                item.sockets = furos_de(&item.octets, &ficha);
                ctx.bolsa.por(equip, Some(item));
                ctx.gastar_dinheiro(custo);
                // Só o `CLEAR_TESSERA`: o cliente desconta o `cost` sozinho.
                ctx.para_mim.push(S2CGamedataSend::clear_tessera(equip as u16, custo as u32).data);
                ctx.mudou = true;
                Ok(custo as i32)
            })
            .await;
        self.relatar(roleid, "removeu as pedras de", tipo, feito, envio).await;
    }

    /// `GP_NPCSEV_REFINE` (35): `{ int inv_index; int item_type; int rt_index; }`, 12 bytes
    /// (`refine_service_executor::player_request`; `c2s_SendCmdNPCSevRefine`). `rt_index` < 0 é
    /// sem talismã.
    pub(super) async fn refinar(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let (Ok(indice), Ok(tipo), Ok(talisma)) = (r.i32(), r.i32(), r.i32()) else { return };
        if conteudo.len() != 12 {
            warn!("mundo: pedido de refino de {roleid} com {} B (esperados 12)", conteudo.len());
            return;
        }
        let tem = self.servicos_em_conversa(roleid).await.is_some_and(|s| s.refinar);
        let confere_talisma = self.sub.talisma_confere_vinculo_e_grau();
        let sorteio: f32 = rand::thread_rng().gen();
        let feito = self
            .com_contexto(roleid, |ctx| {
                // `SendRequest`: recarga, depois o item (`IsItemExist`, calado).
                let agora = std::time::Instant::now();
                if ctx.p.recargas.get(&refino::RECARGA_DO_REFINO).is_some_and(|&fim| fim > agora) {
                    return Err(Some(erro::EM_RECARGA));
                }
                let presente = |s: i32| (s >= 0).then(|| ctx.bolsa.item_no_slot(s as usize)).flatten();
                if !presente(indice).is_some_and(|i| i.item_id as i32 == tipo && i.count >= 1) {
                    return Err(None);
                }
                if !tem {
                    return Err(Some(erro::SERVICO_INDISPONIVEL));
                }
                // `OnServe`: arma a recarga e tenta.
                let ms = refino::RECARGA_DO_REFINO_MS;
                ctx.p.recargas.insert(refino::RECARGA_DO_REFINO, agora + std::time::Duration::from_millis(ms as u64));
                ctx.para_mim.push(S2CGamedataSend::set_cooldown(refino::RECARGA_DO_REFINO, ms).data);
                let resultado = refinar_item(ctx, indice as usize, tipo as u32, talisma, confere_talisma, sorteio);
                resultado.ok_or(Some(erro::NAO_REFINA))
            })
            .await;
        match feito {
            Some(Ok((r, antes))) => info!("mundo: {roleid} refinou o item {tipo} (nível {antes}): {r:?}"),
            Some(Err(Some(e))) => {
                debug!("mundo: {roleid} não refinou o item {tipo}: erro {e}");
                self.responder(roleid, S2CGamedataSend::error_message(e).data, envio).await;
            }
            _ => {}
        }
    }

    /// `GP_NPCSEV_MAKE_SLOT` (47): `{ int src_index; int src_id; }`, 8 bytes
    /// (`make_slot_executor::player_request`; `c2s_SendCmdNPCSevMakeSlot`).
    pub(super) async fn furar(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let (Ok(indice), Ok(tipo)) = (r.i32(), r.i32()) else { return };
        if conteudo.len() != 8 {
            warn!("mundo: pedido de furar de {roleid} com {} B (esperados 8)", conteudo.len());
            return;
        }
        // Todo NPC do 1.5.5 fura (`if(1 || ...)`); no 1.2.6 nenhum.
        let tem = self.sub.furar_existe() && self.servicos_em_conversa(roleid).await.is_some();
        let feito = self
            .com_contexto(roleid, |ctx| {
                let item = (indice >= 0)
                    .then(|| ctx.bolsa.item_no_slot(indice as usize))
                    .flatten()
                    .filter(|i| tipo > 0 && i.item_id as i32 == tipo && i.count >= 1)
                    .cloned()
                    .ok_or(None)?;
                if !tem {
                    return Err(Some(erro::SERVICO_INDISPONIVEL));
                }
                furar_item(ctx, indice as usize, item)
            })
            .await;
        self.relatar(roleid, "furou", tipo, feito, envio).await;
    }

    /// B363 — `GP_NPCSEV_REFINE_TRANSMIT` (45): `{int src_index; int src_id; int dest_index; int dest_id}`, 16 B.
    /// `SendRequest`: recarga 29 (`ERR_OBJECT_IS_COOLING`), mesmo índice ou item que não está no slot → calado;
    /// `OnServe` arma 1 s de recarga e chama `RefineTransmit`, cujo erro vai ao cliente.
    pub(super) async fn transferir_refino(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        if conteudo.len() != 16 {
            return;
        }
        let campo = |i: usize| i32::from_le_bytes(conteudo[i * 4..i * 4 + 4].try_into().unwrap());
        let (origem, tipo_origem, destino, tipo_destino) = (campo(0), campo(1), campo(2), campo(3));
        let tem = self.sub.furar_existe() && self.servicos_em_conversa(roleid).await.is_some_and(|s| s.transferir_refino);
        let feito = self
            .com_contexto(roleid, |ctx| {
                const RECARGA: i32 = 29;
                let agora = std::time::Instant::now();
                if ctx.p.recargas.get(&RECARGA).is_some_and(|fim| *fim > agora) {
                    return Err(Some(erro::EM_RECARGA));
                }
                let existe = |i: i32, t: i32| i >= 0 && ctx.bolsa.item_no_slot(i as usize).is_some_and(|it| it.item_id as i32 == t);
                if origem == destino || !existe(origem, tipo_origem) || !existe(destino, tipo_destino) {
                    return Err(None);
                }
                if !tem {
                    return Err(Some(erro::SERVICO_INDISPONIVEL));
                }
                ctx.p.recargas.insert(RECARGA, agora + std::time::Duration::from_millis(1000));
                transferir_refino_no_contexto(ctx, origem as usize, destino as usize).map_err(Some)
            })
            .await;
        self.relatar(roleid, "transferiu o refino de", tipo_origem, feito, envio).await;
    }

    /// B363 — `GP_NPCSEV_MAKE_SLOT_FOR_DECORATION` (96): `{int src_index; int src_id; u32 material_id; int
    /// material_count}`, 16 B. Índice negativo, ids/quantidade ≤ 0, item fora do slot ou material a menos → calado.
    pub(super) async fn furar_acessorio(&self, roleid: i32, conteudo: &[u8], envio: &EnvioAoCliente) {
        if conteudo.len() != 16 {
            return;
        }
        let campo = |i: usize| i32::from_le_bytes(conteudo[i * 4..i * 4 + 4].try_into().unwrap());
        let (indice, tipo, material, quantidade) = (campo(0), campo(1), campo(2) as u32, campo(3));
        let tem = self.sub.furar_existe() && self.servicos_em_conversa(roleid).await.is_some_and(|s| s.furar_acessorio);
        let sorteio: i32 = rand::thread_rng().gen_range(1..=i32::MAX);
        let feito = self
            .com_contexto(roleid, |ctx| {
                if indice < 0 || tipo <= 0 || material == 0 || (material as i32) <= 0 || quantidade <= 0 {
                    return Err(None);
                }
                let Some(item) = ctx.bolsa.item_no_slot(indice as usize).filter(|i| i.item_id as i32 == tipo).cloned() else {
                    return Err(None);
                };
                if (ctx.bolsa.contar(material) as i64) < quantidade as i64 {
                    return Err(None);
                }
                if !tem {
                    return Err(Some(erro::SERVICO_INDISPONIVEL));
                }
                furar_acessorio_no_contexto(ctx, indice as usize, item, material, quantidade, sorteio)
            })
            .await;
        self.relatar(roleid, "furou o acessório", tipo, feito, envio).await;
    }


    async fn relatar(&self, roleid: i32, verbo: &str, tipo: i32, feito: Option<Result<i32, Option<i32>>>, envio: &EnvioAoCliente) {
        match feito {
            Some(Ok(n)) => info!("mundo: {roleid} {verbo} o item {tipo} ({n})"),
            Some(Err(Some(e))) => {
                debug!("mundo: {roleid} não {verbo} o item {tipo}: erro {e}");
                self.responder(roleid, S2CGamedataSend::error_message(e).data, envio).await;
            }
            Some(Err(None)) => debug!("mundo: pedido de {roleid} sobre o item {tipo} recusado calado"),
            None => {}
        }
    }
}

/// As pedras gravadas no bloco, para a coluna `sockets` do banco.
pub(super) fn furos_de(bloco: &[u8], ficha: &pw_core::FichaDoEquipamento) -> Vec<u32> {
    ConteudoDeEquipamento::ler(bloco, ficha).map(|c| c.furos.iter().map(|&f| f.max(0) as u32).collect()).unwrap_or_default()
}

/// `gplayer_imp::RefineItemAddon`. `None` é o `return false` (o executor manda o erro 92); o
/// `REFINE_CAN_NOT_REFINE` de dentro também vira `None` — o original manda o mesmo erro 92 e
/// não gasta nada.
fn refinar_item(
    ctx: &mut jogo::Contexto,
    indice: usize,
    tipo: u32,
    talisma: i32,
    confere_talisma: bool,
    sorteio: f32,
) -> Option<(ResultadoDoRefino, i32)> {
    let mut item = ctx.bolsa.item_no_slot(indice).cloned()?;
    let alvo = ctx.dados.refino.equipamentos.get(&tipo).copied()?;
    if alvo.addon_de_refino == 0 || alvo.material <= 0 {
        return None;
    }
    if ctx.bolsa.contar(refino::PEDRA_CELESTIAL) < alvo.material as u32 {
        return None;
    }
    let mut ajustes = Ajustes::default();
    let mut talisma_id = None;
    if talisma >= 0 {
        let rt = ctx.bolsa.item_no_slot(talisma as usize)?.item_id;
        let t = ctx.dados.refino.talismas.get(&rt).copied()?;
        if confere_talisma {
            if t.so_vinculado && item.bind_status == 0 {
                return None;
            }
            if t.grau_maximo != 0 && alvo.grau > t.grau_maximo {
                return None;
            }
        }
        ajustes.ajuste[0] = t.chance_extra.clamp(0.0, 1.0);
        ajustes.ajuste[2] = t.chance_de_cair_um.clamp(0.0, 1.0);
        if t.mantem_o_nivel {
            ajustes.ajuste[1] = 2.0;
        }
        ajustes.ajuste2 = t.chance_por_nivel;
        talisma_id = Some(rt);
    }
    let (bloco, ficha) = bloco_do_equipamento(&item, ctx.dados)?;
    let gerado = crate::geracao::gerar_addon(ctx.dados, alvo.addon_de_refino);
    let (novo, (resultado, antes)) = ConteudoDeEquipamento::alterar_rabo(&bloco, &ficha, |c| {
        refino::refinar(&mut c.addons, alvo.addon_de_refino, gerado, &ajustes, sorteio)
    })?;
    let (n, mandar_item) = resultado.para_o_cliente()?;
    ctx.para_mim.push(S2CGamedataSend::refine_result(n).data);
    if resultado != ResultadoDoRefino::NadaMudou {
        item.refine_level = ConteudoDeEquipamento::ler(&novo, &ficha)
            .map(|c| refino::nivel_de_refino(&c.addons, alvo.addon_de_refino).clamp(0, 255) as u8)
            .unwrap_or(0);
        item.octets = novo;
        ctx.bolsa.por(indice, Some(item.clone()));
    }
    if mandar_item {
        ctx.para_mim.push(BusServer::info_de(0, &item, ctx.dados));
    }
    // `RemoveItems(material_id, material_need, DROP_TYPE_USE, true)`.
    for (slot, n) in ctx.bolsa.tirar(refino::PEDRA_CELESTIAL, alvo.material as u32) {
        ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, n, refino::PEDRA_CELESTIAL as i32, DESCARTE_POR_USO).data);
    }
    if let Some(rt) = talisma_id {
        ctx.bolsa.tirar_do_slot(talisma as usize, 1);
        ctx.para_mim.push(ctx.sub.player_drop_item(0, talisma as u8, 1, rt as i32, DESCARTE_POR_USO).data);
    }
    ctx.mudou = true;
    Some((resultado, antes))
}

/// `gplayer_imp::ItemMakeSlot` + `weapon/armor/decoration_equip_item::MakeSlot`. `Err(None)` é
/// a saída calada (item que não é equipamento); `Err(Some(e))` o `error_message(e)`.
fn furar_item(ctx: &mut jogo::Contexto, indice: usize, mut item: ItemRecord) -> Result<i32, Option<i32>> {
    let alvo = ctx.dados.refino.equipamentos.get(&item.item_id).copied().ok_or(None)?;
    let (bloco, ficha) = bloco_do_equipamento(&item, ctx.dados).ok_or(None)?;
    let atual = ConteudoDeEquipamento::ler(&bloco, &ficha).ok_or(None)?;
    if alvo.familia == Familia::Acessorio {
        return Err(Some(erro_do_furo_no_acessorio(ctx.dados, alvo.subtipo, alvo.grau, atual.furos.len())));
    }
    let precisa = refino::material_do_furo(alvo.familia, alvo.grau, atual.furos.len()).map_err(Some)?;
    let [m1, m2] = refino::MATERIAIS_DE_FURO;
    let (tem1, tem2) = (ctx.bolsa.contar(m1) as i64, ctx.bolsa.contar(m2) as i64);
    if precisa as i64 > tem1 + tem2 {
        return Err(Some(erro::SEM_MATERIAL));
    }
    let (novo, _) = ConteudoDeEquipamento::alterar_rabo(&bloco, &ficha, |c| c.furos.push(0)).ok_or(None)?;
    item.octets = novo;
    item.sockets_count = item.sockets_count.saturating_add(1);
    item.sockets = furos_de(&item.octets, &ficha);
    ctx.bolsa.por(indice, Some(item.clone()));
    // `TakeAwayCommonItem` do primeiro material, e do segundo o que faltar.
    let do_primeiro = (precisa as i64).min(tem1) as u32;
    for (m, n) in [(m1, do_primeiro), (m2, precisa as u32 - do_primeiro)] {
        if n == 0 {
            continue;
        }
        for (slot, saiu) in ctx.bolsa.tirar(m, n) {
            ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, saiu, m as i32, DESCARTE_DE_MISSAO).data);
        }
    }
    ctx.para_mim.push(S2CGamedataSend::error_message(erro::FURO_FEITO).data);
    ctx.para_mim.push(BusServer::info_de(0, &item, ctx.dados));
    ctx.mudou = true;
    Ok(precisa)
}

/// `decoration_equip_item::MakeSlot` pelo serviço 47, que chama `ItemMakeSlot(index, id)` com
/// `material_id = 0` (`player_imp.h:4091`): só colar e cinto, grau 1–20, até 4 furos e a casa
/// da `EQUIP_MAKE_HOLE_CONFIG` válida — senão 106; com tudo certo, o material 0 nunca é o pedido
/// e sai `ERR_NOT_ENOUGH_MATERIAL` (25). O furo de acessório de verdade é o serviço 96
/// (`make_slot_for_decoration`), que não está portado.
fn erro_do_furo_no_acessorio(dados: &GameDataManager, subtipo: i32, grau: i32, furos: usize) -> i32 {
    let mascara = dados.refino.mascara_do_subtipo.get(&subtipo).copied();
    if mascara.map_or(true, |m| m & refino::MASCARA_COLAR_OU_CINTO == 0) || !(1..=20).contains(&grau) || furos >= refino::FUROS_NO_ACESSORIO {
        return erro::FURO_FALHOU;
    }
    match dados.refino.furos_de_acessorio.get((grau - 1) as usize).map(|n| n[furos]) {
        Some(c) if c.item > 0 && c.quantidade > 0 && c.taxa >= 0 => erro::SEM_MATERIAL,
        _ => erro::FURO_FALHOU,
    }
}

/// B363 — `gplayer_imp::RefineTransmit(src, dest)` (`player.cpp`): o vínculo do destino tem de ser ao menos o da
/// origem (`BIND` 1 com `CAN_WEBTRADE`, 2 sem; senão 104); os dois refináveis (103); níveis válidos (101) e a origem
/// acima do destino (102); as Pedras do Universo do nível (105). A origem vai a 0 e o destino ao nível da origem; as
/// pedras saem (`TakeAwayCommonItem`) e os dois itens vão de novo ao cliente. `Ok` com o nível transferido.
fn transferir_refino_no_contexto(ctx: &mut jogo::Contexto, origem: usize, destino: usize) -> Result<i32, i32> {
    let (Some(io), Some(id)) = (ctx.bolsa.item_no_slot(origem).cloned(), ctx.bolsa.item_no_slot(destino).cloned()) else {
        return Err(erro::ITEM_FORA_DA_BOLSA);
    };
    let (Some((bo, fo)), Some((bd, fd))) = (bloco_do_equipamento(&io, ctx.dados), bloco_do_equipamento(&id, ctx.dados)) else {
        return Err(erro::ITEM_FORA_DA_BOLSA);
    };
    let vinculo = |i: &ItemRecord| {
        let p = ctx.dados.proc_type_da_instancia(i.item_id, i.bind_status != 0);
        match (p & 0x8000 != 0, p & 0x10000 != 0) {
            (true, true) => 1,
            (true, false) => 2,
            _ => 0,
        }
    };
    if vinculo(&id) < vinculo(&io) {
        return Err(erro::TRANSFERIR_PEDE_VINCULO);
    }
    let refinavel = |tid: u32| ctx.dados.refino.equipamentos.get(&tid).filter(|a| a.addon_de_refino > 0 && a.material > 0).map(|a| a.addon_de_refino);
    let (Some(addon_d), Some(addon_o)) = (refinavel(id.item_id), refinavel(io.item_id)) else {
        return Err(erro::DESTINO_NAO_TRANSFERE);
    };
    let (Some(co), Some(cd)) = (ConteudoDeEquipamento::ler(&bo, &fo), ConteudoDeEquipamento::ler(&bd, &fd)) else {
        return Err(erro::ITEM_FORA_DA_BOLSA);
    };
    let (nivel_o, nivel_d) = (refino::nivel_de_refino(&co.addons, addon_o), refino::nivel_de_refino(&cd.addons, addon_d));
    if nivel_d < 0 || nivel_o < 0 || nivel_o > 12 {
        return Err(erro::NAO_TRANSFERE);
    }
    if nivel_o == 0 || nivel_d >= nivel_o {
        return Err(erro::NIVEL_BAIXO_PARA_TRANSFERIR);
    }
    let pedras = refino::PEDRAS_PARA_TRANSFERIR[(nivel_o - 1) as usize];
    if ctx.bolsa.contar(refino::PEDRA_DO_UNIVERSO) < pedras {
        return Err(erro::TRANSFERIR_SEM_MATERIAL);
    }
    let gerado = crate::geracao::gerar_addon(ctx.dados, addon_d);
    let (novo_o, _) = ConteudoDeEquipamento::alterar_rabo(&bo, &fo, |c| refino::definir_nivel_de_refino(&mut c.addons, addon_o, None, 0))
        .ok_or(erro::ITEM_FORA_DA_BOLSA)?;
    let (novo_d, ok) = ConteudoDeEquipamento::alterar_rabo(&bd, &fd, |c| {
        refino::definir_nivel_de_refino(&mut c.addons, addon_d, gerado, nivel_o as usize)
    })
    .ok_or(erro::ITEM_FORA_DA_BOLSA)?;
    if !ok {
        return Err(erro::DESTINO_NAO_TRANSFERE);
    }
    let mut io = io;
    io.octets = novo_o;
    io.refine_level = 0;
    let mut id = id;
    id.octets = novo_d;
    id.refine_level = nivel_o.clamp(0, 255) as u8;
    ctx.bolsa.por(origem, Some(io.clone()));
    ctx.bolsa.por(destino, Some(id.clone()));
    for (slot, n) in ctx.bolsa.tirar(refino::PEDRA_DO_UNIVERSO, pedras) {
        ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, n, refino::PEDRA_DO_UNIVERSO as i32, DESCARTE_DE_MISSAO).data);
    }
    ctx.para_mim.push(BusServer::info_de(0, &io, ctx.dados));
    ctx.para_mim.push(BusServer::info_de(0, &id, ctx.dados));
    ctx.mudou = true;
    Ok(nivel_o)
}

/// B363 — `ItemMakeSlot(index, id, material, quantidade)` com `decoration_equip_item::MakeSlot`
/// (`item/equip_item.cpp:1474-1557`): só colar e cinto, grau 1–20, até 4 furos e a casa da `EQUIP_MAKE_HOLE_CONFIG`;
/// o material tem de ser o da casa (senão 25); a chance é `quantidade / pedida` (usa no máximo a pedida), a taxa
/// `fee × chance + 0,1` (senão 16). Material e taxa saem mesmo sem furo (`ERR_MAKE_SLOT_FOR_DECOR_PROB`); com o
/// furo, `ERR_MAKE_SLOT_SUCCESS` e o item de novo. Arma e armadura ignoram o material (o furo do serviço 47).
fn furar_acessorio_no_contexto(
    ctx: &mut jogo::Contexto,
    indice: usize,
    item: ItemRecord,
    material: u32,
    quantidade: i32,
    sorteio: i32,
) -> Result<i32, Option<i32>> {
    let alvo = ctx.dados.refino.equipamentos.get(&item.item_id).copied().ok_or(Some(erro::FURO_FALHOU))?;
    if alvo.familia != Familia::Acessorio {
        return furar_item(ctx, indice, item);
    }
    let (bloco, ficha) = bloco_do_equipamento(&item, ctx.dados).ok_or(Some(erro::FURO_FALHOU))?;
    let atual = ConteudoDeEquipamento::ler(&bloco, &ficha).ok_or(Some(erro::FURO_FALHOU))?;
    let mascara = ctx.dados.refino.mascara_do_subtipo.get(&alvo.subtipo).copied();
    let furos = atual.furos.len();
    if mascara.map_or(true, |m| m & refino::MASCARA_COLAR_OU_CINTO == 0)
        || !(1..=20).contains(&alvo.grau)
        || furos >= refino::FUROS_NO_ACESSORIO
    {
        return Err(Some(erro::FURO_FALHOU));
    }
    let casa = ctx
        .dados
        .refino
        .furos_de_acessorio
        .get((alvo.grau - 1) as usize)
        .map(|n| n[furos])
        .filter(|c| c.item > 0 && c.quantidade > 0 && c.taxa >= 0)
        .ok_or(Some(erro::FURO_FALHOU))?;
    if material != casa.item as u32 {
        return Err(Some(erro::SEM_MATERIAL));
    }
    if (ctx.bolsa.contar(material) as i64) < quantidade as i64 {
        return Err(Some(erro::SEM_MATERIAL));
    }
    let usar = quantidade.min(casa.quantidade);
    let chance = usar as f32 / casa.quantidade as f32;
    let taxa = (casa.taxa as f32 * chance + 0.1) as i64;
    if ctx.p.money < taxa {
        return Err(Some(erro::SEM_DINHEIRO));
    }
    // `abase::Rand(1, require_item_count) <= material_count`.
    let acertou = (sorteio % casa.quantidade) + 1 <= usar;
    let mut resultado = Err(Some(erro::FURO_DE_ACESSORIO_FALHOU));
    if acertou {
        let (novo, _) = ConteudoDeEquipamento::alterar_rabo(&bloco, &ficha, |c| c.furos.push(0)).ok_or(Some(erro::FURO_FALHOU))?;
        let mut item = item;
        item.octets = novo;
        item.sockets_count = item.sockets_count.saturating_add(1);
        item.sockets = furos_de(&item.octets, &ficha);
        ctx.bolsa.por(indice, Some(item.clone()));
        ctx.para_mim.push(S2CGamedataSend::error_message(erro::FURO_FEITO).data);
        ctx.para_mim.push(BusServer::info_de(0, &item, ctx.dados));
        resultado = Ok(furos as i32 + 1);
    }
    // `TakeAwayCommonItem(material_id, material_count)` e `SpendMoney(fee_cost)` + `spend_money`.
    for (slot, n) in ctx.bolsa.tirar(material, usar as u32) {
        ctx.para_mim.push(ctx.sub.player_drop_item(0, slot as u8, n, material as i32, DESCARTE_DE_MISSAO).data);
    }
    ctx.p.money -= taxa;
    ctx.para_mim.push(S2CGamedataSend::spend_money(taxa as u32).data);
    ctx.mudou = true;
    resultado
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn as_listas_gravadas_da_pedra_fecham_no_ultimo_byte() {
        let mut b = vec![];
        b.extend(1i32.to_le_bytes());
        b.extend((300u32 | 1 << 13).to_le_bytes());
        b.extend(9i32.to_le_bytes());
        b.extend(0i32.to_le_bytes());
        let l = listas_gravadas(&b).expect("listas");
        assert_eq!(l[0], vec![AddonDoItem::novo(300, vec![9])]);
        assert!(l[1].is_empty());
        assert!(listas_gravadas(&b[..b.len() - 1]).is_none());
        assert!(listas_gravadas(&[]).is_none());
    }
}
