use pw_core::RoleId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendEntry {
    pub role_id: RoleId,
    pub name: String,
    pub level: i32,
    pub cls: u8,
    pub group_id: u8, // 0: Padrão, 1..7: Grupos personalizados
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendList {
    pub character_id: RoleId,
    pub friends: Vec<FriendEntry>,
}
