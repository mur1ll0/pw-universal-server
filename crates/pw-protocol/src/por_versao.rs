//! Fachada de compatibilidade para a geração de subcomandos por versão.
//!
//! Internamente, esta struct delega todas as operações para o despacho polimórfico
//! implementado em `crate::versions::create_world_protocol(versao)`.
//! Isso preserva a API existente e todos os testes sem duplicar código.

use crate::packets::s2c::S2CGamedataSend;
use crate::traits::WorldProtocol;
use crate::version::GameVersion;
use crate::versions::create_world_protocol;
use pw_core::{Vector3, VistaDoJogador};
use std::sync::Arc;

#[derive(Clone)]
pub struct PorVersao {
    protocol: Arc<dyn WorldProtocol>,
}

impl std::fmt::Debug for PorVersao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PorVersao")
            .field("versao", &self.protocol.version())
            .finish()
    }
}

impl PartialEq for PorVersao {
    fn eq(&self, other: &Self) -> bool {
        self.protocol.version() == other.protocol.version()
    }
}

impl Eq for PorVersao {}

impl PorVersao {
    pub fn new(versao: GameVersion) -> Self {
        Self {
            protocol: create_world_protocol(versao),
        }
    }

    pub fn versao(&self) -> GameVersion {
        self.protocol.version()
    }

    pub fn task_data(&self) -> S2CGamedataSend {
        self.protocol.task_data()
    }

    pub fn task_data_com_listas(&self, blocos: [&[u8]; 5]) -> S2CGamedataSend {
        self.protocol.task_data_com_listas(blocos)
    }

    pub fn npc_enter_world(&self, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend {
        self.protocol.npc_enter_world(nid, tid, pos, dir)
    }

    pub fn npc_enter_slice(&self, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend {
        self.protocol.npc_enter_slice(nid, tid, pos, dir)
    }

    pub fn host_attack_result(
        &self,
        target_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
    ) -> S2CGamedataSend {
        self.protocol.host_attack_result(target_id, damage, attack_flag, speed)
    }

    pub fn host_attacked(
        &self,
        atacante: i32,
        dano: i32,
        equipamento: u8,
        attack_flag: i32,
        speed: u8,
    ) -> S2CGamedataSend {
        self.protocol.host_attacked(atacante, dano, equipamento, attack_flag, speed)
    }

    pub fn self_skill_attack_result(
        &self,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend {
        self.protocol.self_skill_attack_result(
            target_id,
            skill_id,
            damage,
            attack_flag,
            speed,
            section,
        )
    }

    pub fn object_skill_attack_result(
        &self,
        attacker_id: i32,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend {
        self.protocol.object_skill_attack_result(
            attacker_id,
            target_id,
            skill_id,
            damage,
            attack_flag,
            speed,
            section,
        )
    }

    pub fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, alvo: i32) -> S2CGamedataSend {
        self.protocol.npc_info_00(nid, hp, max_hp, alvo)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn player_info_00(
        &self,
        player_id: i32,
        level: i16,
        level2: u8,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        alvo: i32,
    ) -> S2CGamedataSend {
        self.protocol.player_info_00(
            player_id, level, level2, hp, max_hp, mp, max_mp, alvo,
        )
    }

    pub fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend {
        self.protocol.receive_exp(exp, sp)
    }

    pub fn equip_item(
        &self,
        idx_ivtr: u8,
        idx_equip: u8,
        count_ivtr: u32,
        count_equip: u32,
    ) -> S2CGamedataSend {
        self.protocol.equip_item(idx_ivtr, idx_equip, count_ivtr, count_equip)
    }

    pub fn equip_data(&self, player_id: i32, crc: u16, mask: u64, items: &[i32]) -> S2CGamedataSend {
        self.protocol.equip_data(player_id, crc, mask, items)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn own_ext_prop(
        &self,
        status_point: u32,
        atributos: (i32, i32, i32, i32),
        max_hp: i32,
        max_mp: i32,
        regen: (i32, i32),
        velocidades: (f32, f32, f32, f32),
        ataque: (i32, i32, i32, i32, f32),
        defesa: (i32, i32),
    ) -> S2CGamedataSend {
        self.protocol.own_ext_prop(
            status_point,
            atributos,
            max_hp,
            max_mp,
            regen,
            velocidades,
            ataque,
            defesa,
        )
    }

    pub fn object_move(&self, id: i32, dest: Vector3, use_time: u16, speed: i16, move_mode: u8) -> S2CGamedataSend {
        self.protocol.object_move(id, dest, use_time, speed, move_mode)
    }

    pub fn object_stop_move(&self, id: i32, dest: Vector3, speed: i16, dir: u8, move_mode: u8) -> S2CGamedataSend {
        self.protocol.object_stop_move(id, dest, speed, dir, move_mode)
    }

    pub fn enter_sanctuary(&self, id: i32) -> S2CGamedataSend {
        self.protocol.enter_sanctuary(id)
    }

    pub fn leave_sanctuary(&self, id: i32) -> S2CGamedataSend {
        self.protocol.leave_sanctuary(id)
    }

    pub fn inst_data_checkout(
        &self,
        id_inst: i32,
        region: u32,
        precinct: u32,
        gshop: u32,
        gshop2: u32,
        gshop3: Option<u32>,
    ) -> S2CGamedataSend {
        self.protocol.inst_data_checkout(id_inst, region, precinct, gshop, gshop2, gshop3)
    }

    pub fn self_info_1(
        &self,
        exp: i32,
        sp: i32,
        world_id: i32,
        pos: Vector3,
        sec_level: u8,
    ) -> S2CGamedataSend {
        self.protocol.self_info_1(exp, sp, world_id, pos, sec_level)
    }

    pub fn get_own_money(&self, amount: u32, capacity: u32) -> S2CGamedataSend {
        self.protocol.get_own_money(amount, capacity)
    }

    pub fn player_enter_world(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        self.protocol.player_enter_world(role_id, vista)
    }

    pub fn player_enter_slice(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        self.protocol.player_enter_slice(role_id, vista)
    }
}
