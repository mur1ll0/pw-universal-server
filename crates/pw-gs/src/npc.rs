//! `SEVNPC_SERVE` (37): tudo que se pede a um NPC, num comando só.
//!
//! Missão, loja, reparo, cura, teleporte, forja, aprender habilidade — todos chegam por
//! este comando, separados por um `service_type`. O corpo depois dele muda de forma
//! conforme o serviço, e é por isso que ele merece um módulo.
//!
//! # O layout
//!
//! Payload (já sem o cabeçalho de 2 bytes), de `SRV::C2S::CMD::service_serve`:
//!
//! | Deslocamento | Campo |
//! | ---: | :--- |
//! | 0 | `service_type` (`int`) |
//! | 4 | `len` (`size_t`, 4 bytes no i386) |
//! | 8 | `content`, com forma própria por serviço |
//!
//! # A inversão que estava aqui
//!
//! Os nomes do enum são do ponto de vista **do NPC**, e o `gateway.rs` os lia do ponto de
//! vista do jogador. `EC_GPDataType.h` é explícito:
//!
//! ```text
//! GP_NPCSEV_SELL = 1,   //  1, NPC sell to player
//! GP_NPCSEV_BUY,        //  NPC buy from player
//! ```
//!
//! E `EC_SendC2SCmds.cpp` confirma pelo lado de quem envia: a função que o cliente chama
//! quando o **jogador compra** manda `GP_NPCSEV_SELL`, e a que ele chama quando o
//! **jogador vende** manda `GP_NPCSEV_BUY`.
//!
//! O `gateway.rs` fazia o contrário nas duas: no serviço 1 apagava um item do jogador e
//! lhe dava dinheiro, e no serviço 2 cobrava dinheiro e lhe dava um item. Ou seja, comprar
//! tirava o item e pagava; vender cobrava e entregava mercadoria.

use pw_wire::gamedata::Reader;

/// Deslocamento do `content` dentro do payload: `service_type` (4) + `len` (4).
pub const INICIO_DO_CONTEUDO: usize = 8;

/// Os tipos de serviço, de `EC_GPDataType.h`.
///
/// Os nomes são **do NPC**: `VENDE` é o NPC vendendo, isto é, o jogador comprando.
pub mod servico {
    /// `GP_NPCSEV_SELL` — o NPC vende; o **jogador compra**.
    pub const NPC_VENDE: i32 = 1;
    /// `GP_NPCSEV_BUY` — o NPC compra; o **jogador vende**.
    pub const NPC_COMPRA: i32 = 2;
    pub const REPARAR: i32 = 3;
    pub const CURAR: i32 = 4;
    pub const TELEPORTAR: i32 = 5;
    pub const ENTREGAR_MISSAO: i32 = 6;
    pub const ACEITAR_MISSAO: i32 = 7;
    pub const ITEM_DE_MISSAO: i32 = 8;
    pub const APRENDER_HABILIDADE: i32 = 9;
    pub const INCRUSTAR_PEDRA: i32 = 10;
    pub const LIMPAR_PEDRAS: i32 = 11;
    pub const FORJAR: i32 = 12;
    pub const DECOMPOR: i32 = 13;
    pub const SENHA_DO_ARMAZEM: i32 = 14;
    pub const ABRIR_ARMAZEM: i32 = 15;
}

/// O envelope do comando, com o corpo ainda por interpretar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedidoAoNpc<'a> {
    pub service_type: i32,
    /// Tamanho que o cliente declara para o corpo.
    pub len: u32,
    /// O corpo, cuja forma depende do serviço.
    pub conteudo: &'a [u8],
}

impl<'a> PedidoAoNpc<'a> {
    pub fn ler(payload: &'a [u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let service_type = r.i32().ok()?;
        let len = r.u32().ok()?;
        Some(Self {
            service_type,
            len,
            conteudo: payload.get(INICIO_DO_CONTEUDO..).unwrap_or(&[]),
        })
    }

    /// O `len` que o cliente declarou bate com o que de fato veio?
    ///
    /// Um `len` maior que o corpo é pacote truncado ou forjado; ler adiante dele seria ler
    /// lixo. Quem usa o conteúdo deve conferir isto antes.
    pub fn tamanho_confere(&self) -> bool {
        self.conteudo.len() >= self.len as usize
    }
}

/// Um item numa compra (`GP_NPCSEV_SELL`), de `C2S::npc_trade_item` — 12 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemComprado {
    pub tid: i32,
    pub index: u32,
    pub count: u32,
}

/// Um item numa venda (`GP_NPCSEV_BUY`), de `C2S::npc_sell_item` — 16 bytes.
///
/// Tem um campo a mais que o de compra: o `price`. **Não é fonte de verdade** — é o preço
/// que o cliente acha que vale, e aceitá-lo seria deixar o jogador escolher quanto ganha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemVendido {
    pub tid: i32,
    pub index: u32,
    pub count: u32,
    pub price: i32,
}

/// Cabeçalho do conteúdo de uma **compra** (o NPC vendendo).
///
/// `EC_SendC2SCmds.cpp`, `c2s_SendCmdNPCSevBuy`: `money`, `consume_contrib`,
/// `cumulate_contrib`, `force_id`, `force_repu`, `force_contrib`, `item_count` — sete
/// `int`/`size_t`, 28 bytes, e só então a lista de itens.
///
/// O `gateway.rs` lia o id do item no deslocamento 0 do conteúdo, que é o `money`.
pub const CABECALHO_DE_COMPRA: usize = 28;

/// Cabeçalho do conteúdo de uma **venda** (o NPC comprando): só `item_count`.
pub const CABECALHO_DE_VENDA: usize = 4;

/// Lê a lista de itens de uma compra.
pub fn itens_comprados(conteudo: &[u8]) -> Vec<ItemComprado> {
    let Some(resto) = conteudo.get(CABECALHO_DE_COMPRA..) else {
        return Vec::new();
    };
    let quantos = contagem(conteudo, CABECALHO_DE_COMPRA - 4);
    let mut r = Reader::new(resto);
    (0..quantos)
        .map_while(|_| {
            Some(ItemComprado {
                tid: r.i32().ok()?,
                index: r.u32().ok()?,
                count: r.u32().ok()?,
            })
        })
        .collect()
}

/// Lê a lista de itens de uma venda.
pub fn itens_vendidos(conteudo: &[u8]) -> Vec<ItemVendido> {
    let Some(resto) = conteudo.get(CABECALHO_DE_VENDA..) else {
        return Vec::new();
    };
    let quantos = contagem(conteudo, 0);
    let mut r = Reader::new(resto);
    (0..quantos)
        .map_while(|_| {
            Some(ItemVendido {
                tid: r.i32().ok()?,
                index: r.u32().ok()?,
                count: r.u32().ok()?,
                price: r.i32().ok()?,
            })
        })
        .collect()
}

/// Lê um `item_count` num deslocamento, com teto.
///
/// O teto não é decoração: o `item_count` vem do cliente, e um número absurdo faria o
/// servidor tentar reservar memória para ele antes de descobrir que o pacote é curto.
fn contagem(conteudo: &[u8], em: usize) -> usize {
    const TETO: u32 = 64;
    let Some(bytes) = conteudo.get(em..em + 4) else {
        return 0;
    };
    let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    n.min(TETO) as usize
}

/// O id da missão, primeiro campo do conteúdo em `TASK_ACCEPT`, `TASK_RETURN` e
/// `TASK_MATTER`.
///
/// Confirmado em `c2s_SendCmdNPCSevAcceptTask` e `c2s_SendCmdNPCSevReturnTask`: `idTask` é
/// o primeiro `int` do `CONTENT`. Esta era a parte que o `gateway.rs` já lia certo.
pub fn id_da_missao(conteudo: &[u8]) -> Option<i32> {
    Reader::new(conteudo).i32().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(servico: i32, conteudo: &[u8]) -> Vec<u8> {
        let mut v = servico.to_le_bytes().to_vec();
        v.extend_from_slice(&(conteudo.len() as u32).to_le_bytes());
        v.extend_from_slice(conteudo);
        v
    }

    #[test]
    fn o_envelope_separa_servico_tamanho_e_conteudo() {
        let p = envelope(servico::ACEITAR_MISSAO, &7788i32.to_le_bytes());
        let pedido = PedidoAoNpc::ler(&p).unwrap();
        assert_eq!(pedido.service_type, servico::ACEITAR_MISSAO);
        assert_eq!(pedido.len, 4);
        assert!(pedido.tamanho_confere());
        assert_eq!(id_da_missao(pedido.conteudo), Some(7788));
    }

    #[test]
    fn um_len_maior_que_o_corpo_e_detectado() {
        // Pacote truncado ou forjado: o cliente diz 40 bytes e manda 4.
        let mut p = servico::NPC_VENDE.to_le_bytes().to_vec();
        p.extend_from_slice(&40u32.to_le_bytes());
        p.extend_from_slice(&[0u8; 4]);
        assert!(!PedidoAoNpc::ler(&p).unwrap().tamanho_confere());
    }

    #[test]
    fn a_compra_le_os_itens_depois_dos_28_bytes_de_cabecalho() {
        // O `gateway.rs` lia o id do item no deslocamento 0 do conteúdo, que é o `money`.
        // Aqui o `money` vai com um valor bem visível, para que lê-lo por engano apareça.
        let mut c = Vec::new();
        c.extend_from_slice(&999_999u32.to_le_bytes()); // money
        c.extend_from_slice(&[0u8; 20]); // os cinco campos de contribuição/facção
        c.extend_from_slice(&1u32.to_le_bytes()); // item_count
        c.extend_from_slice(&4123i32.to_le_bytes()); // tid
        c.extend_from_slice(&3u32.to_le_bytes()); // index
        c.extend_from_slice(&2u32.to_le_bytes()); // count

        let itens = itens_comprados(&c);
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].tid, 4123, "leu o `money` no lugar do `tid`?");
        assert_eq!(itens[0].index, 3);
        assert_eq!(itens[0].count, 2);
    }

    #[test]
    fn a_venda_le_os_itens_depois_dos_4_bytes_de_cabecalho() {
        let mut c = 2u32.to_le_bytes().to_vec(); // item_count
        for (tid, idx, qtd, preco) in [(801i32, 5u32, 10u32, 77i32), (802, 6, 1, 88)] {
            c.extend_from_slice(&tid.to_le_bytes());
            c.extend_from_slice(&idx.to_le_bytes());
            c.extend_from_slice(&qtd.to_le_bytes());
            c.extend_from_slice(&preco.to_le_bytes());
        }
        let itens = itens_vendidos(&c);
        assert_eq!(itens.len(), 2);
        assert_eq!((itens[0].tid, itens[0].index, itens[0].count), (801, 5, 10));
        assert_eq!(itens[1].tid, 802);
        assert_eq!(itens[1].price, 88, "o preço do cliente é lido, não obedecido");
    }

    #[test]
    fn uma_contagem_absurda_nao_derruba_o_servidor() {
        // `item_count` vem do cliente. Sem teto, um número grande faria o servidor tentar
        // montar uma lista enorme antes de descobrir que o pacote acabou.
        let mut c = Vec::new();
        c.extend_from_slice(&[0u8; 24]);
        c.extend_from_slice(&u32::MAX.to_le_bytes()); // item_count
        let itens = itens_comprados(&c);
        assert!(itens.is_empty(), "não havia item nenhum depois da contagem");
    }
}
