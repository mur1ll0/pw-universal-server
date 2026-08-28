use serde::{Deserialize, Serialize};

/// Tipo de container onde o item está guardado
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerType {
    Inventory,   // Bolsa / Inventário principal
    Equipment,   // Itens equipados no corpo
    Storehouse,  // Armazém / Banco
    Fashion,     // Roupas e cosméticos equipados
    PetCorral,   // Bolsa de mascotes
}

impl ContainerType {
    pub fn to_i16(&self) -> i16 {
        match self {
            ContainerType::Inventory => 0,
            ContainerType::Equipment => 1,
            ContainerType::Storehouse => 2,
            ContainerType::Fashion => 3,
            ContainerType::PetCorral => 4,
        }
    }

    pub fn from_i16(val: i16) -> Self {
        match val {
            0 => ContainerType::Inventory,
            1 => ContainerType::Equipment,
            2 => ContainerType::Storehouse,
            3 => ContainerType::Fashion,
            4 => ContainerType::PetCorral,
            _ => ContainerType::Inventory,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerType::Inventory => "INVENTORY",
            ContainerType::Equipment => "EQUIPMENT",
            ContainerType::Storehouse => "STOREHOUSE",
            ContainerType::Fashion => "FASHION",
            ContainerType::PetCorral => "PET_CORRAL",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "INVENTORY" | "0" => Some(ContainerType::Inventory),
            "EQUIPMENT" | "1" => Some(ContainerType::Equipment),
            "STOREHOUSE" | "2" => Some(ContainerType::Storehouse),
            "FASHION" | "3" => Some(ContainerType::Fashion),
            "PET_CORRAL" | "4" => Some(ContainerType::PetCorral),
            _ => None,
        }
    }
}

/// Estrutura de item normalizada com ID de instância e atributos de forja
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemRecord {
    pub id: Option<i64>,           // ID único de instância no banco (BIGSERIAL)
    pub character_id: i32,
    pub container_type: ContainerType,
    pub slot: u16,                 // Índice do slot no container (0..63)
    pub item_id: u32,              // ID do template no elements.data
    pub count: u32,
    pub max_count: u32,
    pub refine_level: u8,          // Refino (+0 a +12)
    pub sockets_count: u8,         // Quantidade de furos para pedras (0..4)
    pub sockets: Vec<u32>,         // IDs das pedras espirituais incrustadas
    pub durability: u32,           // Durabilidade atual
    pub max_durability: u32,       // Durabilidade máxima
    pub bind_status: u8,           // 0: Livre, 1: Preso à alma
    pub custom_attributes: serde_json::Value, // Atributos adicionais e nome do criador
}

pub type InventoryItem = ItemRecord;

impl ItemRecord {
    pub fn new(character_id: i32, container_type: ContainerType, slot: u16, item_id: u32, count: u32) -> Self {
        Self {
            id: None,
            character_id,
            container_type,
            slot,
            item_id,
            count,
            max_count: 100,
            refine_level: 0,
            sockets_count: 0,
            sockets: Vec::new(),
            durability: 1000,
            max_durability: 1000,
            bind_status: 0,
            custom_attributes: serde_json::json!({}),
        }
    }
}

/// Slots de Equipamento do Personagem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EquipmentSlot {
    Weapon = 0,        // Arma
    Helmet = 1,        // Elmo / Capacete
    Necklace = 2,      // Colar
    Cape = 3,          // Capa / Manto
    ChestArmor = 4,    // Armadura Peitoral
    Belt = 5,          // Ornamento / Cinto
    LegArmor = 6,      // Calça / Perneira
    Boots = 7,         // Botas
    Bracers = 8,       // Braçadeiras
    RingLeft = 9,      // Anel Esquerdo
    RingRight = 10,    // Anel Direito
    Ammunition = 11,   // Flechas / Projéteis
    FlyMount = 12,     // Asa / Voo
    FashionTop = 13,   // Roupa Superior
    FashionBottom = 14,// Roupa Inferior
    FashionBoots = 15, // Sapato Fashion
    FashionBracers = 16,// Luva Fashion
    Tome = 17,         // Livro Sagrado
    Smiley = 18,       // Emoticons
    GuardianAngel = 19,// Amuleto / Hierograma
    GoldCharm = 20,    // Amuleto de Vida
    SilverCharm = 21,  // Amuleto de Mana
}
