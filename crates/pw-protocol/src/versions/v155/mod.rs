pub mod role;

use crate::octets::OctetsStream;
use crate::packets::s2c::S2CGamedataSend;
use crate::traits::{ProtocolAdapter, WorldProtocol};
use crate::version::GameVersion;
use pw_core::{CharacterSummary, Vector3, VistaDoJogador};

pub struct V155Protocol;

impl WorldProtocol for V155Protocol {
    fn version(&self) -> GameVersion {
        GameVersion::V1_5_5
    }

    fn task_data(&self) -> S2CGamedataSend {
        // 1.5.5: 5 blocos (active, finished, time, count, storage)
        let mut s = OctetsStream::new();
        s.write_u16_le(105);
        for _ in 0..5 {
            s.write_u32_le(0);
        }
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn task_data_com_listas(&self, blocos: [&[u8]; 5]) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(105);
        for b in blocos.iter() {
            s.write_u32_le(b.len() as u32);
            s.write_raw_bytes(b);
        }
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn npc_enter_world(&self, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend {
        self.info_npc(16, nid, tid, pos, dir)
    }

    fn npc_enter_slice(&self, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend {
        self.info_npc(11, nid, tid, pos, dir)
    }

    fn host_attack_result(&self, target_id: i32, damage: i32, attack_flag: i32, speed: u8) -> S2CGamedataSend {
        S2CGamedataSend::host_attack_result(target_id, damage, attack_flag, speed)
    }

    fn host_attacked(&self, atacante: i32, dano: i32, equipamento: u8, attack_flag: i32, speed: u8) -> S2CGamedataSend {
        S2CGamedataSend::host_attacked(atacante, dano, equipamento, attack_flag, speed)
    }

    fn self_skill_attack_result(
        &self,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend {
        S2CGamedataSend::self_skill_attack_result(target_id, skill_id, damage, attack_flag, speed, section)
    }

    fn object_skill_attack_result(
        &self,
        attacker_id: i32,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend {
        S2CGamedataSend::object_skill_attack_result(attacker_id, target_id, skill_id, damage, attack_flag, speed, section)
    }

    fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, alvo: i32) -> S2CGamedataSend {
        S2CGamedataSend::npc_info_00(nid, hp, max_hp, alvo)
    }

    fn player_info_00(
        &self,
        player_id: i32,
        level: i16,
        level2: u8,
        em_combate: bool,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        alvo: i32,
    ) -> S2CGamedataSend {
        S2CGamedataSend::player_info_00(player_id, level, level2, em_combate, hp, max_hp, mp, max_mp, alvo)
    }

    fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend {
        S2CGamedataSend::receive_exp(exp, sp)
    }

    fn equip_item(&self, idx_ivtr: u8, idx_equip: u8, count_ivtr: u32, count_equip: u32) -> S2CGamedataSend {
        S2CGamedataSend::equip_item(idx_ivtr, idx_equip, count_ivtr, count_equip)
    }

    fn own_ext_prop(
        &self,
        status_point: u32,
        atributos: (i32, i32, i32, i32),
        max_hp: i32,
        max_mp: i32,
        max_ap: i32,
        regen: (i32, i32),
        velocidades: (f32, f32, f32, f32),
        ataque: (i32, i32, i32, i32, f32),
        // `damage_magic_low/high` do `ROLEEXTPROP_ATK`: é o "Atq. Mágico" da ficha, e ia
        // zero fixo até o B77 (`gs/player.cpp:4358` manda o `_cur_prop` inteiro).
        magico: (i32, i32),
        // `resistance[5]` do `ROLEEXTPROP_DEF`, na ordem metal/madeira/água/fogo/terra.
        resistencias: [i32; 5],
        defesa: (i32, i32),
    ) -> S2CGamedataSend {
        // 196 bytes no 1.5.5
        S2CGamedataSend::own_ext_prop(
            status_point,
            atributos,
            max_hp,
            max_mp,
            max_ap,
            regen,
            velocidades,
            ataque,
            magico,
            resistencias,
            defesa,
        )
    }

    fn enter_sanctuary(&self, id: i32) -> S2CGamedataSend {
        S2CGamedataSend::enter_sanctuary(id)
    }

    fn leave_sanctuary(&self, id: i32) -> S2CGamedataSend {
        S2CGamedataSend::leave_sanctuary(id)
    }

    fn inst_data_checkout(
        &self,
        id_inst: i32,
        region: u32,
        precinct: u32,
        gshop: u32,
        gshop2: u32,
        gshop3: Option<u32>,
    ) -> S2CGamedataSend {
        let base = S2CGamedataSend::inst_data_checkout(id_inst, region, precinct, gshop, gshop2);
        match gshop3 {
            Some(g3) => {
                let mut bytes = base.data;
                bytes.extend_from_slice(&g3.to_le_bytes());
                S2CGamedataSend { data: bytes }
            }
            None => base,
        }
    }

    fn self_info_1(
        &self,
        exp: i32,
        sp: i32,
        world_id: i32,
        pos: Vector3,
        sec_level: u8,
    ) -> S2CGamedataSend {
        // 38 bytes no 1.5.5: ganha state2 (4B)
        let base = S2CGamedataSend::self_info_1(exp, sp, world_id, pos, sec_level);
        let mut bytes = base.data;
        bytes.extend_from_slice(&0i32.to_le_bytes()); // state2
        S2CGamedataSend { data: bytes }
    }

    fn player_enter_world(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        S2CGamedataSend::player_enter_world(role_id, vista)
    }

    fn player_enter_slice(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        S2CGamedataSend::player_enter_slice(role_id, vista)
    }
}

impl V155Protocol {
    fn info_npc(&self, comando: u16, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend {
        // 35 bytes no 1.5.5 (com vis_tid e state2)
        let mut s = OctetsStream::new();
        s.write_u16_le(comando);
        s.write_i32_le(nid);
        s.write_i32_le(tid);
        s.write_i32_le(tid); // vis_tid
        s.write_f32_le(pos.x);
        s.write_f32_le(pos.y);
        s.write_f32_le(pos.z);
        s.write_u16_le(0); // seed
        s.write_u8(dir);
        s.write_i32_le(0); // state
        s.write_i32_le(0); // state2
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }
}

pub struct V155Adapter;

impl ProtocolAdapter for V155Adapter {
    fn version(&self) -> GameVersion {
        GameVersion::V1_5_5
    }

    fn encode_online_announce(&self, stream: &mut OctetsStream, userid: i32, localsid: u32) {
        stream.write_i32(userid);
        stream.write_u32(localsid);
        stream.write_i32(0);
        stream.write_i8(1);
        stream.write_i32(0);
        stream.write_i32(0);
        stream.write_i32(0);
        stream.write_i8(0); // referrer_flag
        stream.write_i8(0); // passwd_flag
        stream.write_i8(0); // usbbind
        stream.write_i8(0); // accountinfo_flag
    }

    fn encode_role_info(&self, stream: &mut OctetsStream, c: &CharacterSummary) {
        role::encode_role_info_155(stream, c);
    }

    fn encode_create_role_response(
        &self,
        stream: &mut OctetsStream,
        result: i32,
        role_id: i32,
        localsid: u32,
        character: Option<&CharacterSummary>,
    ) {
        stream.write_i32(result);
        stream.write_i32(role_id);
        stream.write_u32(localsid);

        let vazio;
        let c = match character {
            Some(c) => c,
            None => {
                vazio = CharacterSummary::vazio();
                &vazio
            }
        };
        self.encode_role_info(stream, c);
        stream.write_i32(0); // refretcode presente no 1.5.5
    }
}
