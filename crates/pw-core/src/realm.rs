use crate::types::RealmId;
use serde::{Deserialize, Serialize};

/// Configuração e estado dinâmico de um Realm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealmInfo {
    pub id: RealmId,
    pub name: String,
    pub version: String, // "1.2.6", "1.5.3"
    pub host: String,
    pub port: u16,
    pub is_online: bool,
    pub max_players: i32,
    
    // Multiplicadores ao Vivo (Double Events)
    pub double_exp_multiplier: f32,
    pub double_sp_multiplier: f32,
    pub double_drop_multiplier: f32,
    pub double_gold_multiplier: f32,
    
    pub config: RealmConfig,
}

/// Parâmetros e Feature Flags do Realm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RealmConfig {
    pub enabled_classes: Vec<u8>,
    pub max_level: i32,
    pub meridians_enabled: bool,
    pub reincarnation_enabled: bool,
    pub astrolabe_enabled: bool,
    pub homestead_enabled: bool,
}
