//! B363 — a barraca pessoal (`gs/playerstall.cpp`, `gs/playerstall.h`).
//!
//! O jogador abre uma barraca (C2S `OPEN_BOOTH` 76) com até 40 entradas (24 no 1.2.6): itens da
//! bolsa à venda e itens que ele **compra** (`index` 0xFFFF). Fica parado no estado de barraca
//! (`PLAYER_STATE_MARKET`) até fechar (77) ou sair. Os outros a abrem como um NPC (`SEVNPC_HELLO`
//! no id dele, a até 6 m), recebem a vitrine (`SEVNPC_GET_CONTENT` → `PLAYER_BOOTH_INFO` 169) e
//! compram pelo serviço 19 (`SERVICE_ID_PLAYER_MARKET`) ou vendem pelo 21 (`…MARKET2`).
//!
//! Aqui ficam as partes sem rede: ler os pedidos, conferir e fazer a troca entre as duas bolsas.
//! Sem cartão de barraca (`_stall_info.stallcard_id == -1`, o padrão de `stall_info`,
//! `player_imp.h:273-280`): o `yinpiao` (Nota de Prata) não entra — o original só o aceita com
//! cartão (`playerstall.cpp:584-596`).

use pw_core::ItemRecord;
use pw_data_loader::GameDataManager;
use pw_protocol::RegrasDaBarraca;

use crate::economia::Bolsa;

/// `index` de uma entrada de **compra** (`playerstall.cpp:56`).
pub const SEM_INDICE: u16 = 0xFFFF;
/// `PLAYER_MARKET_SELL_SLOT`/`PLAYER_MARKET_BUY_SLOT` (`gs/config.h:118-119`): vagas sem cartão.
pub const VAGAS_DE_VENDA: usize = 12;
pub const VAGAS_DE_COMPRA: usize = 12;
/// `PLAYER_MARKET_NAME_LEN` (`gs/config.h:120`): o `max_name_length` sem cartão.
pub const NOME_MAXIMO: usize = 28;
/// `MONEY_CAPACITY_BASE` (`gs/config.h:79`): o teto da carteira.
pub const TETO_DE_DINHEIRO: u32 = 2_000_000_000;
/// Distância de serviço: `msg.pos.squared_distance(_parent->pos) < 36.f` (`playerstall.cpp:395`).
pub const DISTANCIA_DE_SERVICO: f32 = 6.0;
/// `SERVICE_ID_PLAYER_MARKET` (19: a barraca **vende**) e `…MARKET2` (21: a barraca **compra**),
/// `gs/serviceprovider.h:284-286`; no cliente `GP_NPCSEV_BOOTHSELL`/`BOOTHBUY`.
pub const SERVICO_BARRACA_VENDE: i32 = 19;
pub const SERVICO_BARRACA_COMPRA: i32 = 21;

/// Erros do `S2C::error_message` que a barraca usa (`common/protocol.h`, enum de erros).
pub mod erro {
    pub const SERVICO_INDISPONIVEL: i32 = 14;
    pub const NAO_PODE_ABRIR: i32 = 41;
    pub const ITEM_INVALIDO: i32 = 42;
    pub const VENDEDOR_NO_TETO: i32 = 133;
    pub const VENDEDOR_SEM_DINHEIRO: i32 = 134;
    pub const COMPRADOR_NO_TETO: i32 = 135;
}

/// Uma entrada do C2S 76 (`open_personal_market::entry_t`, `common/protocol.h:5490-5496`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaPedida {
    pub tipo: i32,
    pub indice: u32,
    pub quantidade: u32,
    pub preco: u32,
}

/// O C2S 76 inteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedidoDeBarraca {
    /// O campo do nome inteiro (62 ou 28 bytes, UTF-16 terminado em `00 00`).
    pub nome: Vec<u8>,
    pub itens: Vec<EntradaPedida>,
}

fn u32_em(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn u16_em(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap())
}

/// `size < sizeof(pmo) || !pmo.count || size != sizeof(pmo) + count·16` → `ERR_FATAL_ERR`
/// (`playercmd.cpp:2862-2867`). `None` é esse erro.
pub fn ler_pedido(corpo: &[u8], r: &RegrasDaBarraca) -> Option<PedidoDeBarraca> {
    let cab = 2 + r.nome_no_pedido;
    if corpo.len() < cab {
        return None;
    }
    let n = u16_em(corpo, 0) as usize;
    if n == 0 || corpo.len() != cab + n * 16 {
        return None;
    }
    let itens = (0..n)
        .map(|i| {
            let o = cab + i * 16;
            EntradaPedida {
                tipo: u32_em(corpo, o) as i32,
                indice: u32_em(corpo, o + 4),
                quantidade: u32_em(corpo, o + 8),
                preco: u32_em(corpo, o + 12),
            }
        })
        .collect();
    Some(PedidoDeBarraca { nome: corpo[2..cab].to_vec(), itens })
}

/// O tamanho do nome com o terminador, como o laço de `PlayerOpenPersonalMarket`
/// (`playerstall.cpp:29-37`) e o `SetMarketName` (`playerstall.h:56-68`): de 2 em 2 bytes até o
/// primeiro `00 00`, contando-o; sem terminador, o campo inteiro.
pub fn tamanho_do_nome(nome: &[u8]) -> usize {
    let mut i = 0;
    while i + 1 < nome.len() {
        if nome[i] == 0 && nome[i + 1] == 0 {
            return i + 2;
        }
        i += 2;
    }
    nome.len()
}

/// Um item da barraca (`player_stall::entry_t`, `playerstall.h:9-17`).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemDaBarraca {
    pub tipo: i32,
    /// O slot da bolsa à venda, ou [`SEM_INDICE`] numa compra.
    pub indice: u16,
    /// O que ainda resta vender (ou comprar).
    pub quantidade: u32,
    pub preco: u32,
    /// O conteúdo do item na hora de abrir: o `crc` e o `expire_date` do original
    /// (`AddTradeGoods`, `playerstall.h:84-94`) — se o slot mudar, a venda é recusada.
    pub retrato: Vec<u8>,
}

/// A barraca aberta (`player_stall`).
#[derive(Debug, Clone, PartialEq)]
pub struct Barraca {
    /// `_stall_trade_id` na hora de abrir: o `market_id` dos pacotes e o `trade_id` que o pedido
    /// de compra tem de trazer (`CheckMarketTradeRequest`, `playerstall.cpp:542`).
    pub trade_id: i32,
    /// O nome até o terminador, inclusive (`SetMarketName`).
    pub nome: Vec<u8>,
    pub itens: Vec<ItemDaBarraca>,
}

impl Barraca {
    /// `pPlayer->market_id = _stall_trade_id & 0xFF` (`playerstall.cpp:146`).
    pub fn market_id(&self) -> u8 {
        (self.trade_id & 0xFF) as u8
    }

    /// As entradas como o `SELF_OPEN_BOOTH` as manda.
    pub fn para_o_dono(&self) -> Vec<(i32, u16, u32, u32)> {
        self.itens.iter().map(|i| (i.tipo, i.indice, i.quantidade, i.preco)).collect()
    }
}

/// O próximo `_stall_trade_id`: `++`, pulando os que terminam em `0x00` (`playerstall.cpp:142-144`).
pub fn proximo_trade_id(atual: i32) -> i32 {
    let mut n = atual.wrapping_add(1);
    if n & 0xFF == 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// Por que a barraca não abriu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecusaDeAbrir {
    /// `ERR_INVALID_ITEM` (42) e, como o `PlayerOpenPersonalMarket` devolve falso,
    /// `ERR_CANNOT_OPEN_PLAYER_MARKET` (41) em seguida.
    ItemInvalido,
    /// Só o 41.
    NaoPode,
}

/// `item_list::IsItemExist(index, type, count)`: o slot tem o item e pelo menos `count`.
fn item_existe(bolsa: &Bolsa, indice: usize, tipo: i32, n: u32) -> bool {
    bolsa.item_no_slot(indice).is_some_and(|i| i.item_id as i32 == tipo && i.count >= n)
}

fn nao_negocia(dados: &GameDataManager, i: &ItemRecord) -> bool {
    dados.proc_type_da_instancia(i.item_id, i.bind_status != 0) & pw_data_loader::manager::PROC_SEM_TROCAR != 0
}

/// `price × count` com o `size_t` de 32 bits do original e a conferência `p / count != price`.
fn total(preco: u32, n: u32) -> Option<u32> {
    let p = preco.wrapping_mul(n);
    (n != 0 && p / n == preco).then_some(p)
}

/// O corpo de `PlayerOpenPersonalMarket` (`playerstall.cpp:18-163`) depois das conferências de
/// estado: confere cada entrada e monta a barraca. `dinheiro` é o da bolsa.
pub fn abrir(
    pedido: &PedidoDeBarraca,
    r: &RegrasDaBarraca,
    bolsa: &Bolsa,
    dados: &GameDataManager,
    dinheiro: u32,
    trade_id: i32,
) -> Result<Barraca, RecusaDeAbrir> {
    // `count > SELL_MAX + BUY_MAX || count == 0` (40 no 1.5.5, `config.h:121-122`; 0x18 no 1.2.6,
    // VA 0x813057a).
    let maximo = if r.confere_nome_vagas_e_dinheiro { 40 } else { 24 };
    if pedido.itens.is_empty() || pedido.itens.len() > maximo {
        return Err(RecusaDeAbrir::NaoPode);
    }
    let nome_len = tamanho_do_nome(&pedido.nome);
    if r.confere_nome_vagas_e_dinheiro && nome_len > NOME_MAXIMO {
        return Err(RecusaDeAbrir::NaoPode);
    }
    let mut usados = vec![false; bolsa.slots.len()];
    let (mut compras, mut compra_total, mut venda_total) = (0usize, 0u32, 0u32);
    for e in &pedido.itens {
        let venda = e.indice != SEM_INDICE as u32;
        if e.preco == 0
            || e.preco > r.preco_maximo
            || e.quantidade == 0
            || (venda && !item_existe(bolsa, e.indice as usize, e.tipo, e.quantidade))
        {
            return Err(RecusaDeAbrir::ItemInvalido);
        }
        if !venda {
            // `get_item_for_sell(type)` e `count > pile_limit` (`playerstall.cpp:64-72`).
            let tid = e.tipo as u32;
            if e.tipo <= 0 || !dados.pilhas.contains_key(&tid) || e.quantidade > dados.limite_de_pilha(tid) {
                return Err(RecusaDeAbrir::ItemInvalido);
            }
            compras += 1;
            if r.confere_nome_vagas_e_dinheiro {
                compra_total = total(e.preco, e.quantidade)
                    .and_then(|p| compra_total.checked_add(p))
                    .ok_or(RecusaDeAbrir::NaoPode)?;
            }
        } else {
            let i = bolsa.item_no_slot(e.indice as usize).expect("conferido acima");
            if nao_negocia(dados, i) {
                return Err(RecusaDeAbrir::ItemInvalido);
            }
            let slot = &mut usados[e.indice as usize];
            if *slot {
                return Err(RecusaDeAbrir::ItemInvalido);
            }
            *slot = true;
            if r.confere_nome_vagas_e_dinheiro {
                venda_total = total(e.preco, e.quantidade)
                    .and_then(|p| venda_total.checked_add(p))
                    .ok_or(RecusaDeAbrir::NaoPode)?;
            }
        }
    }
    if r.confere_nome_vagas_e_dinheiro {
        // Vagas e dinheiro sem cartão (`playerstall.cpp:104-110`).
        if compras > VAGAS_DE_COMPRA || pedido.itens.len() - compras > VAGAS_DE_VENDA {
            return Err(RecusaDeAbrir::NaoPode);
        }
        if compra_total > dinheiro || venda_total > TETO_DE_DINHEIRO.saturating_sub(dinheiro) {
            return Err(RecusaDeAbrir::NaoPode);
        }
    }
    let itens = pedido
        .itens
        .iter()
        .map(|e| ItemDaBarraca {
            tipo: e.tipo,
            indice: if e.indice == SEM_INDICE as u32 { SEM_INDICE } else { e.indice as u16 },
            quantidade: e.quantidade,
            preco: e.preco,
            retrato: if e.indice == SEM_INDICE as u32 {
                Vec::new()
            } else {
                bolsa.item_no_slot(e.indice as usize).map(|i| i.octets.clone()).unwrap_or_default()
            },
        })
        .collect();
    Ok(Barraca { trade_id, nome: pedido.nome[..nome_len].to_vec(), itens })
}

/// Uma linha do pedido de compra/venda (`trade_request::entry_t`, `playerstall.h:33-39`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinhaDoPedido {
    pub tipo: i32,
    /// O índice na barraca.
    pub indice: u16,
    /// O slot da bolsa de quem vende à barraca (só no serviço 21).
    pub slot: u16,
    pub quantidade: u32,
}

/// O conteúdo do serviço 19/21 (`trade_request`; no cliente o `CONTENT` de
/// `c2s_SendCmdNPCSevBoothBuy`, `EC_SendC2SCmds.cpp:3785-3831`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedidoDeCompra {
    pub trade_id: i32,
    pub yinpiao: u32,
    pub linhas: Vec<LinhaDoPedido>,
}

/// `length < sizeof(req) || length != sizeof(req) + count·12` (`playerstall.cpp:213-217`): `None`
/// é o `-2`, que o comprador vê como `ERR_SERVICE_UNAVILABLE`.
pub fn ler_pedido_de_compra(c: &[u8], r: &RegrasDaBarraca) -> Option<PedidoDeCompra> {
    let cab = if r.pedido_com_yinpiao { 16 } else { 12 };
    if c.len() < cab {
        return None;
    }
    let n = u32_em(c, cab - 4) as usize;
    if c.len() != cab + n.checked_mul(12)? {
        return None;
    }
    let yinpiao = if r.pedido_com_yinpiao { u32_em(c, 8) } else { 0 };
    let linhas = (0..n)
        .map(|i| {
            let o = cab + i * 12;
            LinhaDoPedido {
                tipo: u32_em(c, o) as i32,
                indice: u16_em(c, o + 4),
                slot: u16_em(c, o + 6),
                quantidade: u32_em(c, o + 8),
            }
        })
        .collect();
    Some(PedidoDeCompra { trade_id: u32_em(c, 0) as i32, yinpiao, linhas })
}

/// Um lado do negócio: a bolsa e o dinheiro.
pub struct Lado<'a> {
    pub id: i32,
    pub bolsa: &'a mut Bolsa,
    pub dinheiro: &'a mut u32,
}

/// Por que o negócio não saiu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecusaDoNegocio {
    /// O `< 0` do `DoPlayerMarketTrade`/`Purchase`: `ERR_SERVICE_UNAVILABLE` ao freguês. O `Option`
    /// é o erro que o original manda **antes** (o teto, o dinheiro).
    Recusa(Option<i32>),
    /// 1.2.6: o vendedor passaria do teto — a barraca fecha (`CheckMarketTradeRequest`, VA 0x8131cae).
    FechaABarraca,
}

/// O que o negócio deixou para avisar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Negocio {
    /// O total pago.
    pub total: u32,
    /// `(slot da bolsa da barraca, tipo, quantidade)` — o `BOOTH_SELL_ITEM` (191) ao dono, ou o
    /// `PLAYER_DROP_ITEM` ao freguês na compra da barraca.
    pub sairam: Vec<(u16, i32, u32)>,
    /// `(tipo, validade, quantidade, slot onde entrou, índice na barraca)` — o `PURCHASE_ITEM`.
    pub entraram: Vec<(i32, i32, u32, u16, u8)>,
}

/// Tira `n` do slot (`Exchange`/`Remove` se for tudo, `DecAmount` senão) e devolve o pedaço.
fn tirar(bolsa: &mut Bolsa, slot: usize, n: u32) -> Option<ItemRecord> {
    let i = bolsa.item_no_slot(slot)?.clone();
    if i.count == n {
        bolsa.pegar(slot)
    } else {
        bolsa.tirar_do_slot(slot, n);
        Some(ItemRecord { id: None, count: n, ..i })
    }
}

/// O freguês **compra** da barraca (serviço 19): `CheckMarketTradeRequest`,
/// `CheckMarketTradeRequire` e `DoPlayerMarketTrade` (`playerstall.cpp:198-237`, `:530-626`,
/// `:755-874`).
pub fn comprar_da_barraca(
    barraca: &mut Barraca,
    pedido: &PedidoDeCompra,
    r: &RegrasDaBarraca,
    dados: &GameDataManager,
    vendedor: Lado,
    comprador: Lado,
) -> Result<Negocio, RecusaDoNegocio> {
    use RecusaDoNegocio::Recusa;
    if barraca.trade_id != pedido.trade_id {
        return Err(Recusa(None));
    }
    let mut pedidos = vec![0u32; barraca.itens.len()];
    let mut m = 0u32;
    for l in &pedido.linhas {
        let idx = l.indice as usize;
        let e = barraca.itens.get(idx).ok_or(Recusa(None))?;
        if l.quantidade == 0 || e.indice == SEM_INDICE || e.tipo != l.tipo {
            return Err(Recusa(None));
        }
        if e.quantidade < l.quantidade || e.quantidade < pedidos[idx] + l.quantidade {
            return Err(Recusa(None));
        }
        pedidos[idx] += l.quantidade;
        let i = vendedor.bolsa.item_no_slot(e.indice as usize).ok_or(Recusa(None))?;
        if i.item_id as i32 != e.tipo || i.count < e.quantidade || nao_negocia(dados, i) || i.octets != e.retrato {
            return Err(Recusa(None));
        }
        m = total(e.preco, l.quantidade).and_then(|p| m.checked_add(p)).ok_or(Recusa(None))?;
    }
    // Sem cartão de barraca, Nota de Prata não entra (`if (_stall_info.stallcard_id == -1) return false`).
    if pedido.yinpiao != 0 {
        return Err(Recusa(None));
    }
    let teto = vendedor.dinheiro.checked_add(m).is_none_or(|t| t > TETO_DE_DINHEIRO);
    if teto {
        return Err(if r.teto_do_vendedor_fecha_a_barraca {
            RecusaDoNegocio::FechaABarraca
        } else {
            Recusa(Some(erro::VENDEDOR_NO_TETO))
        });
    }
    // `CheckMarketTradeRequire` (`playerstall.cpp:755-774`).
    if *comprador.dinheiro < m || (comprador.bolsa.livres() as usize) < pedido.linhas.len() {
        return Err(Recusa(None));
    }
    let mut negocio = Negocio { total: m, ..Default::default() };
    for l in &pedido.linhas {
        let e = &mut barraca.itens[l.indice as usize];
        let Some(pedaco) = tirar(vendedor.bolsa, e.indice as usize, l.quantidade) else { continue };
        e.quantidade -= l.quantidade;
        negocio.sairam.push((e.indice, e.tipo, l.quantidade));
        let Some(onde) = comprador.bolsa.empurrar(pedaco, dados) else { continue };
        negocio.entraram.push((e.tipo, 0, l.quantidade, onde as u16, 0));
    }
    // `pImp->SpendMoney(need_money); GainMoney(need_money)`.
    *comprador.dinheiro -= m;
    *vendedor.dinheiro += m;
    Ok(negocio)
}

/// O freguês **vende** à barraca (serviço 21): `CheckMarketPurchaseRequest`,
/// `CheckMarketPurchaseRequire` e `DoPlayerMarketPurchase` (`playerstall.cpp:239-278`, `:628-753`,
/// `:876-975`).
pub fn vender_a_barraca(
    barraca: &mut Barraca,
    pedido: &PedidoDeCompra,
    dados: &GameDataManager,
    dono: Lado,
    fregues: Lado,
) -> Result<Negocio, RecusaDoNegocio> {
    use RecusaDoNegocio::Recusa;
    if barraca.trade_id != pedido.trade_id {
        return Err(Recusa(None));
    }
    let mut pedidos = vec![0u32; barraca.itens.len()];
    let mut m = 0u32;
    for l in &pedido.linhas {
        let idx = l.indice as usize;
        let e = barraca.itens.get(idx).ok_or(Recusa(None))?;
        if l.quantidade == 0 || e.indice != SEM_INDICE || e.tipo != l.tipo {
            return Err(Recusa(None));
        }
        if e.quantidade < l.quantidade || e.quantidade < pedidos[idx] + l.quantidade {
            return Err(Recusa(None));
        }
        pedidos[idx] += l.quantidade;
        m = total(e.preco, l.quantidade).and_then(|p| m.checked_add(p)).ok_or(Recusa(None))?;
    }
    if *dono.dinheiro < m {
        return Err(Recusa(Some(erro::VENDEDOR_SEM_DINHEIRO)));
    }
    if (dono.bolsa.livres() as usize) < pedido.linhas.len() {
        return Err(Recusa(None));
    }
    // `CheckMarketPurchaseRequire` (`playerstall.cpp:699-753`).
    if fregues.dinheiro.checked_add(m).is_none_or(|t| t > TETO_DE_DINHEIRO) {
        return Err(Recusa(Some(erro::COMPRADOR_NO_TETO)));
    }
    let mut slots = vec![false; fregues.bolsa.slots.len()];
    for l in &pedido.linhas {
        let s = l.slot as usize;
        let ok = fregues
            .bolsa
            .item_no_slot(s)
            .is_some_and(|i| i.item_id as i32 == l.tipo && i.count >= l.quantidade && !nao_negocia(dados, i));
        if !ok || slots[s] {
            return Err(Recusa(None));
        }
        slots[s] = true;
    }
    let mut negocio = Negocio { total: m, ..Default::default() };
    for l in &pedido.linhas {
        let Some(pedaco) = tirar(fregues.bolsa, l.slot as usize, l.quantidade) else { continue };
        let e = &mut barraca.itens[l.indice as usize];
        e.quantidade -= l.quantidade;
        negocio.sairam.push((l.slot, l.tipo, l.quantidade));
        let Some(onde) = dono.bolsa.empurrar(pedaco, dados) else { continue };
        negocio.entraram.push((l.tipo, 0, l.quantidade, onde as u16, (l.indice & 0xFF) as u8));
    }
    *dono.dinheiro -= m;
    *fregues.dinheiro += m;
    Ok(negocio)
}

/// A vitrine (`GM_MSG_SERVICE_QUIERY_CONTENT`, `playerstall.cpp:463-525`): venda com o item ainda
/// igual no slot, compra com `-count`, esgotado ou mudado vazio. `conteudo` dá o bloco do item.
pub fn vitrine(
    barraca: &Barraca,
    bolsa: &Bolsa,
    conteudo: impl Fn(&ItemRecord) -> Vec<u8>,
) -> Vec<pw_protocol::packets::s2c::ItemDaVitrine> {
    use pw_protocol::packets::s2c::ItemDaVitrine;
    barraca
        .itens
        .iter()
        .map(|e| {
            if e.quantidade == 0 {
                return ItemDaVitrine::Vazio;
            }
            if e.indice == SEM_INDICE {
                return ItemDaVitrine::Compra { tipo: e.tipo, quantidade: e.quantidade, preco: e.preco };
            }
            match bolsa.item_no_slot(e.indice as usize) {
                Some(i) if i.item_id as i32 == e.tipo && i.octets == e.retrato => ItemDaVitrine::Vende {
                    tipo: e.tipo,
                    quantidade: e.quantidade,
                    preco: e.preco,
                    validade: 0,
                    conteudo: conteudo(i),
                },
                _ => ItemDaVitrine::Vazio,
            }
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use pw_core::ContainerType;

    fn item(slot: u16, tid: u32, n: u32) -> ItemRecord {
        ItemRecord {
            id: None,
            character_id: 1,
            container_type: ContainerType::Inventory,
            slot,
            item_id: tid,
            count: n,
            max_count: 99,
            refine_level: 0,
            sockets_count: 0,
            sockets: vec![],
            durability: 0,
            max_durability: 0,
            bind_status: 0,
            octets: vec![],
            custom_attributes: serde_json::json!({}),
        }
    }

    fn lado<'a>(id: i32, bolsa: &'a mut Bolsa, dinheiro: &'a mut u32) -> Lado<'a> {
        Lado { id, bolsa, dinheiro }
    }

    fn dados() -> GameDataManager {
        let mut d = GameDataManager::new();
        d.pilhas.insert(3043, 99);
        d.pilhas.insert(11208, 1);
        d
    }

    fn pedido(nome: &[u8], itens: &[(i32, u32, u32, u32)], r: &RegrasDaBarraca) -> Vec<u8> {
        let mut c = (itens.len() as u16).to_le_bytes().to_vec();
        let mut campo = nome.to_vec();
        campo.resize(r.nome_no_pedido, 0);
        c.extend(campo);
        for &(t, i, n, p) in itens {
            for v in [t as u32, i, n, p] {
                c.extend(v.to_le_bytes());
            }
        }
        c
    }

    /// `ler_pedido`: o tamanho tem de fechar com o nome da versão; o nome conta o terminador.
    #[test]
    fn o_pedido_fecha_com_o_nome_da_versao() {
        for r in [RegrasDaBarraca::DO_155, RegrasDaBarraca::DO_126] {
            let c = pedido(&[b'A', 0, b'B', 0, 0, 0], &[(3043, 4, 5, 100)], &r);
            let p = ler_pedido(&c, &r).expect("fecha");
            assert_eq!(tamanho_do_nome(&p.nome), 6);
            assert_eq!(p.itens[0], EntradaPedida { tipo: 3043, indice: 4, quantidade: 5, preco: 100 });
            assert!(ler_pedido(&c[..c.len() - 1], &r).is_none(), "um byte a menos não fecha");
        }
        let r = RegrasDaBarraca::DO_155;
        assert!(ler_pedido(&pedido(b"", &[], &r), &r).is_none(), "sem itens é erro fatal");
    }

    /// `PlayerOpenPersonalMarket`: item que não está na bolsa, preço acima do teto da versão, slot
    /// repetido e compra acima da pilha recusam com o item inválido; dinheiro e vagas só no 1.5.5.
    #[test]
    fn abrir_confere_cada_entrada_como_o_original() {
        let d = dados();
        let mut bolsa = Bolsa::nova(1, ContainerType::Inventory, 32, vec![item(4, 3043, 10), item(5, 11208, 1)]);
        let r155 = RegrasDaBarraca::DO_155;
        let r126 = RegrasDaBarraca::DO_126;
        let abre = |itens: &[(i32, u32, u32, u32)], r: &RegrasDaBarraca, din: u32, b: &Bolsa| {
            abrir(&ler_pedido(&pedido(&[b'X', 0, 0, 0], itens, r), r).unwrap(), r, b, &d, din, 7)
        };
        let b = abre(&[(3043, 4, 10, 50), (3043, 0xFFFF, 20, 3)], &r155, 1_000, &bolsa).expect("abre");
        assert_eq!((b.market_id(), b.itens.len(), b.itens[1].indice), (7, 2, SEM_INDICE));
        assert_eq!(abre(&[(3043, 4, 11, 50)], &r155, 0, &bolsa), Err(RecusaDeAbrir::ItemInvalido), "mais do que tem");
        assert_eq!(abre(&[(3043, 4, 1, 300_000_000)], &r126, 0, &bolsa), Err(RecusaDeAbrir::ItemInvalido), "teto do 1.2.6");
        assert!(abre(&[(3043, 4, 1, 300_000_000)], &r155, 0, &bolsa).is_ok(), "o 1.5.5 aceita até 2·10⁹");
        assert_eq!(abre(&[(3043, 4, 1, 5), (3043, 4, 1, 5)], &r155, 0, &bolsa), Err(RecusaDeAbrir::ItemInvalido), "slot repetido");
        assert_eq!(abre(&[(3043, 0xFFFF, 100, 1)], &r155, 1_000, &bolsa), Err(RecusaDeAbrir::ItemInvalido), "acima da pilha");
        assert_eq!(abre(&[(3043, 0xFFFF, 20, 100)], &r155, 1_000, &bolsa), Err(RecusaDeAbrir::NaoPode), "compra sem dinheiro");
        assert!(abre(&[(3043, 0xFFFF, 20, 100)], &r126, 1_000, &bolsa).is_ok(), "o 1.2.6 não confere o dinheiro");
        bolsa.por(5, None);
        let nome_longo: Vec<u8> = std::iter::repeat_n(b'a', 40).collect();
        let longo = ler_pedido(&pedido(&nome_longo, &[(3043, 4, 1, 1)], &r155), &r155).unwrap();
        assert_eq!(abrir(&longo, &r155, &bolsa, &d, 0, 1), Err(RecusaDeAbrir::NaoPode), "nome acima de 28 bytes");
    }

    /// `DoPlayerMarketTrade`: parte da pilha passa ao comprador, o dinheiro troca de mãos e a entrada
    /// diminui; o `trade_id` errado, o item mudado no slot e o comprador sem dinheiro recusam.
    #[test]
    fn comprar_da_barraca_troca_item_e_dinheiro() {
        let d = dados();
        let r = RegrasDaBarraca::DO_155;
        let mut bv = Bolsa::nova(1, ContainerType::Inventory, 32, vec![item(4, 3043, 10)]);
        let mut bc = Bolsa::nova(2, ContainerType::Inventory, 32, vec![]);
        let mut barraca = abrir(&ler_pedido(&pedido(&[0, 0], &[(3043, 4, 10, 50)], &r), &r).unwrap(), &r, &bv, &d, 0, 9).unwrap();
        let (mut dv, mut dc) = (100u32, 1_000u32);
        let pede = |trade_id: i32, n: u32| PedidoDeCompra {
            trade_id,
            yinpiao: 0,
            linhas: vec![LinhaDoPedido { tipo: 3043, indice: 0, slot: 0, quantidade: n }],
        };
        assert_eq!(
            comprar_da_barraca(&mut barraca, &pede(8, 3), &r, &d, lado(1, &mut bv, &mut dv), lado(2, &mut bc, &mut dc)),
            Err(RecusaDoNegocio::Recusa(None))
        );
        let n = comprar_da_barraca(&mut barraca, &pede(9, 3), &r, &d, lado(1, &mut bv, &mut dv), lado(2, &mut bc, &mut dc)).unwrap();
        assert_eq!((n.total, dv, dc), (150, 250, 850));
        assert_eq!(n.sairam, vec![(4, 3043, 3)]);
        assert_eq!(n.entraram, vec![(3043, 0, 3, 0, 0)]);
        assert_eq!(bv.item_no_slot(4).map(|i| i.count), Some(7));
        assert_eq!(bc.item_no_slot(0).map(|i| i.count), Some(3));
        assert_eq!(barraca.itens[0].quantidade, 7);
        dc = 10;
        assert_eq!(
            comprar_da_barraca(&mut barraca, &pede(9, 1), &r, &d, lado(1, &mut bv, &mut dv), lado(2, &mut bc, &mut dc)),
            Err(RecusaDoNegocio::Recusa(None)),
            "comprador sem dinheiro"
        );
        dc = 1_000;
        dv = TETO_DE_DINHEIRO - 10;
        assert_eq!(
            comprar_da_barraca(&mut barraca, &pede(9, 1), &r, &d, lado(1, &mut bv, &mut dv), lado(2, &mut bc, &mut dc)),
            Err(RecusaDoNegocio::Recusa(Some(erro::VENDEDOR_NO_TETO)))
        );
        assert_eq!(
            comprar_da_barraca(&mut barraca, &pede(9, 1), &RegrasDaBarraca::DO_126, &d, lado(1, &mut bv, &mut dv), lado(2, &mut bc, &mut dc)),
            Err(RecusaDoNegocio::FechaABarraca),
            "no 1.2.6 o teto fecha a barraca"
        );
    }

    /// `DoPlayerMarketPurchase`: o freguês entrega do slot dele, recebe o preço, e a barraca paga.
    #[test]
    fn vender_a_barraca_paga_o_fregues() {
        let d = dados();
        let r = RegrasDaBarraca::DO_155;
        let mut bd = Bolsa::nova(1, ContainerType::Inventory, 32, vec![]);
        let mut bf = Bolsa::nova(2, ContainerType::Inventory, 32, vec![item(6, 3043, 5)]);
        let mut barraca = abrir(&ler_pedido(&pedido(&[0, 0], &[(3043, 0xFFFF, 4, 25)], &r), &r).unwrap(), &r, &bd, &d, 1_000, 3).unwrap();
        let (mut dd, mut df) = (1_000u32, 0u32);
        let pedido = PedidoDeCompra {
            trade_id: 3,
            yinpiao: 0,
            linhas: vec![LinhaDoPedido { tipo: 3043, indice: 0, slot: 6, quantidade: 4 }],
        };
        let n = vender_a_barraca(
            &mut barraca,
            &pedido,
            &d,
            Lado { id: 1, bolsa: &mut bd, dinheiro: &mut dd },
            Lado { id: 2, bolsa: &mut bf, dinheiro: &mut df },
        )
        .unwrap();
        assert_eq!((n.total, dd, df), (100, 900, 100));
        assert_eq!(n.sairam, vec![(6, 3043, 4)]);
        assert_eq!(bf.item_no_slot(6).map(|i| i.count), Some(1));
        assert_eq!(bd.item_no_slot(0).map(|i| i.count), Some(4));
        assert_eq!(barraca.itens[0].quantidade, 0);
        // Esgotou: outra venda passa da quantidade.
        assert_eq!(
            vender_a_barraca(&mut barraca, &pedido, &d, Lado { id: 1, bolsa: &mut bd, dinheiro: &mut dd }, Lado { id: 2, bolsa: &mut bf, dinheiro: &mut df }),
            Err(RecusaDoNegocio::Recusa(None))
        );
    }

    #[test]
    fn o_trade_id_pula_o_final_zero() {
        assert_eq!(proximo_trade_id(0), 1);
        assert_eq!(proximo_trade_id(0xFF), 0x101);
    }
}
