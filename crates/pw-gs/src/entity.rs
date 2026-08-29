use pw_core::{CharacterClass, Gender, Race, RoleId, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveBuff {
    pub buff_id: u32,
    pub level: u8,
    pub duration_ms: u32,
    pub elapsed_ms: u32,
    pub tick_interval_ms: u32,
    pub tick_elapsed_ms: u32,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerEntity {
    pub role_id: RoleId,
    pub name: String,
    pub race: Race,
    pub cls: CharacterClass,
    pub gender: Gender,
    pub level: i32,
    pub cultivation: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub sp: i64,
    pub money: i64,
    
    // Atributos de Combate
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub energy: i32,
    pub def_phys: i32,
    pub def_metal: i32,
    pub def_wood: i32,
    pub def_water: i32,
    pub def_fire: i32,
    pub def_earth: i32,
    pub attack_min: i32,
    pub attack_max: i32,
    pub magic_attack_min: i32,
    pub magic_attack_max: i32,
    pub attack_speed: f32,
    pub move_speed: f32,
    pub crit_rate: f32,
    
    pub position: Vector3,
    pub target_id: Option<i64>,
    pub buffs: Vec<ActiveBuff>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonsterEntity {
    pub id: i64,
    pub template_id: u32,
    pub name: String,
    pub level: i32,
    pub hp: i64,
    pub max_hp: i64,
    pub mp: i32,
    pub max_mp: i32,
    pub def_phys: i32,
    pub def_magic: i32,
    pub attack_min: i32,
    pub attack_max: i32,
    pub attack_range: f32,
    pub exp: i64,
    pub sp: i64,
    pub aipolicy_id: u32,
    pub drop_table_id: u32,
    
    pub position: Vector3,
    pub spawn_center: Vector3,
    pub move_speed: f32,
    pub is_dead: bool,
    pub respawn_timer_ms: u32,
    pub respawn_delay_ms: u32,
    
    pub target_id: Option<i64>,
    pub buffs: Vec<ActiveBuff>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcEntity {
    pub id: i64,
    pub template_id: u32,
    pub name: String,
    pub position: Vector3,
    pub dialog_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDropEntity {
    pub id: i64,
    pub item_id: u32,
    pub count: u32,
    pub position: Vector3,
    pub owner_role_id: Option<RoleId>,
    pub protect_timer_ms: u32,
    pub despawn_timer_ms: u32,
}
