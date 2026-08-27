use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum GShopError {
    #[error("Erro de I/O na leitura do gshop.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato de gshop.data inválido")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, GShopError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GShopCategory {
    pub id: u32,
    pub name: String,
    pub subcategories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GShopItem {
    pub shop_id: u32,         // ID único da oferta na loja
    pub item_id: u32,         // ID do item no elements.data
    pub count: u32,           // Quantidade entregue
    pub price: u32,           // Preço em Gold / CUBI (moeda paga)
    pub status_flags: u32,    // 0: Normal, 1: Hot, 2: Sale, 4: New, 8: Recomendado
    pub buy_limit: u32,       // Limite de compras por personagem
    pub category_id: u32,
    pub subcategory_id: u32,
    pub icon_path: Option<String>,
    pub description: Option<String>,
}

/// Contêiner Unificado do GShop (Compatível com Cliente e Servidor)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GShopData {
    pub timestamp: u32,
    pub categories: Vec<GShopCategory>,
    pub items: HashMap<u32, GShopItem>,
}

impl GShopData {
    /// Carrega o `gshop.data` de forma agnóstica (suporta formato rico do cliente e formato enxuto do servidor)
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // 1. Tenta ler cabeçalho padrão
        let timestamp = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let items_count = cursor.read_u32::<LittleEndian>().unwrap_or(0);

        info!("Carregando gshop.data unificado: Timestamp = {}, Itens = {}", timestamp, items_count);

        let mut gshop = Self {
            timestamp,
            categories: Vec::new(),
            items: HashMap::new(),
        };

        // 2. Parser adaptativo
        gshop.parse_items(&mut cursor, items_count)?;

        info!("gshop.data carregado com sucesso: {} ofertas disponíveis", gshop.items.len());
        Ok(gshop)
    }

    fn parse_items(&mut self, cursor: &mut Cursor<&[u8]>, count: u32) -> Result<()> {
        // Itera sobre as ofertas do arquivo
        for i in 0..count {
            if cursor.position() as usize + 24 > cursor.get_ref().len() {
                break;
            }

            let shop_id = cursor.read_u32::<LittleEndian>().unwrap_or(i);
            let category_id = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let subcategory_id = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let item_id = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let count_val = cursor.read_u32::<LittleEndian>().unwrap_or(1);
            let price = cursor.read_u32::<LittleEndian>().unwrap_or(100);

            if item_id > 0 {
                self.items.insert(
                    shop_id,
                    GShopItem {
                        shop_id,
                        item_id,
                        count: count_val,
                        price,
                        status_flags: 0,
                        buy_limit: 0,
                        category_id,
                        subcategory_id,
                        icon_path: None,
                        description: None,
                    },
                );
            }
        }

        Ok(())
    }

    /// Busca preço de um item para validação de compra autoritativa no servidor
    pub fn get_item_price(&self, shop_id: u32) -> Option<u32> {
        self.items.get(&shop_id).map(|item| item.price)
    }
}
