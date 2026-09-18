use crate::packets::s2c::S2CGamedataSend;
use crate::version::GameVersion;
use pw_core::Vector3;

/// Trait que abstrai a geração de subcomandos do mundo 3D (GamedataSend)
/// cujos layouts variam entre as versões do jogo.
pub trait WorldProtocol: Send + Sync {
    /// Versão do jogo implementada por este protocolo
    fn version(&self) -> GameVersion;

    /// TASK_DATA (105) vazio
    fn task_data(&self) -> S2CGamedataSend;

    /// TASK_DATA (105) com as listas reais
    fn task_data_com_listas(&self, blocos: [&[u8]; 5]) -> S2CGamedataSend;

    /// NPC_ENTER_WORLD (16)
    fn npc_enter_world(&self, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend;

    /// NPC_ENTER_SLICE (11)
    fn npc_enter_slice(&self, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend;

    /// HOST_ATTACKRESULT (24)
    fn host_attack_result(&self, target_id: i32, damage: i32, attack_flag: i32, speed: u8) -> S2CGamedataSend;

    /// HOST_ATTACKED (26)
    fn host_attacked(&self, atacante: i32, dano: i32, equipamento: u8, attack_flag: i32, speed: u8) -> S2CGamedataSend;

    /// HOST_SKILL_ATTACK_RESULT (142)
    fn self_skill_attack_result(
        &self,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend;

    /// OBJECT_SKILL_ATTACK_RESULT (143)
    fn object_skill_attack_result(
        &self,
        attacker_id: i32,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend;

    /// NPC_INFO_00 (33)
    fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, alvo: i32) -> S2CGamedataSend;

    /// PLAYER_INFO_00 (32)
    #[allow(clippy::too_many_arguments)]
    fn player_info_00(
        &self,
        player_id: i32,
        level: i16,
        level2: u8,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        alvo: i32,
    ) -> S2CGamedataSend;

    /// RECEIVE_EXP (36)
    fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend;

    /// EQUIP_ITEM (48)
    fn equip_item(&self, idx_ivtr: u8, idx_equip: u8, count_ivtr: u32, count_equip: u32) -> S2CGamedataSend;

    /// EQUIP_DATA (66)
    fn equip_data(&self, player_id: i32, crc: u16, mask: u64, items: &[i32]) -> S2CGamedataSend {
        S2CGamedataSend::equip_data(player_id, crc, mask, items)
    }

    /// OWN_EXT_PROP (50)
    #[allow(clippy::too_many_arguments)]
    fn own_ext_prop(
        &self,
        status_point: u32,
        atributos: (i32, i32, i32, i32),
        max_hp: i32,
        max_mp: i32,
        regen: (i32, i32),
        velocidades: (f32, f32, f32, f32),
        ataque: (i32, i32, i32, i32, f32),
        defesa: (i32, i32),
    ) -> S2CGamedataSend;

    /// OBJECT_MOVE (15)
    fn object_move(&self, id: i32, dest: Vector3, use_time: u16, speed: i16, move_mode: u8) -> S2CGamedataSend {
        S2CGamedataSend::object_move(id, dest, use_time, speed, move_mode)
    }

    /// OBJECT_STOP_MOVE (35)
    fn object_stop_move(&self, id: i32, dest: Vector3, speed: i16, dir: u8, move_mode: u8) -> S2CGamedataSend {
        S2CGamedataSend::object_stop_move(id, dest, speed, dir, move_mode)
    }

    /// ENTER_SANCTUARY (164)
    fn enter_sanctuary(&self, id: i32) -> S2CGamedataSend;

    /// LEAVE_SANCTUARY (165)
    fn leave_sanctuary(&self, id: i32) -> S2CGamedataSend;

    /// INST_DATA_CHECKOUT (206)
    fn inst_data_checkout(
        &self,
        id_inst: i32,
        region: u32,
        precinct: u32,
        gshop: u32,
        gshop2: u32,
        gshop3: Option<u32>,
    ) -> S2CGamedataSend;

    /// SELF_INFO_1 (8)
    fn self_info_1(
        &self,
        exp: i32,
        sp: i32,
        world_id: i32,
        pos: Vector3,
        sec_level: u8,
    ) -> S2CGamedataSend;

    /// GET_OWN_MONEY (82)
    fn get_own_money(&self, amount: u32, capacity: u32) -> S2CGamedataSend {
        S2CGamedataSend::get_own_money(amount, capacity)
    }

    /// PLAYER_ENTER_WORLD (17)
    fn player_enter_world(&self, role_id: i32, vista: pw_core::VistaDoJogador) -> S2CGamedataSend;

    /// PLAYER_ENTER_SLICE (12)
    fn player_enter_slice(&self, role_id: i32, vista: pw_core::VistaDoJogador) -> S2CGamedataSend;
}

// ProtocolAdapter é exportado canonicamente de crate::adapter
pub use crate::adapter::ProtocolAdapter;
