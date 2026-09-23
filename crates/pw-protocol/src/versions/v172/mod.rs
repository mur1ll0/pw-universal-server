use crate::traits::{ProtocolAdapter, WorldProtocol};
use crate::version::GameVersion;
use crate::versions::v155::{V155Adapter, V155Protocol};

/// Stub da versão 1.7.2. Herda o comportamento do 1.5.5 como base e pode
/// sobrescrever os métodos conforme novas especificações forem adicionadas.
pub struct V172Protocol(pub V155Protocol);

impl WorldProtocol for V172Protocol {
    fn version(&self) -> GameVersion {
        GameVersion::V1_7_2
    }

    fn task_data(&self) -> crate::packets::s2c::S2CGamedataSend {
        self.0.task_data()
    }

    fn task_data_com_listas(&self, blocos: [&[u8]; 5]) -> crate::packets::s2c::S2CGamedataSend {
        self.0.task_data_com_listas(blocos)
    }

    fn npc_enter_world(&self, nid: i32, tid: i32, pos: pw_core::Vector3, dir: u8) -> crate::packets::s2c::S2CGamedataSend {
        self.0.npc_enter_world(nid, tid, pos, dir)
    }

    fn npc_enter_slice(&self, nid: i32, tid: i32, pos: pw_core::Vector3, dir: u8) -> crate::packets::s2c::S2CGamedataSend {
        self.0.npc_enter_slice(nid, tid, pos, dir)
    }

    fn host_attack_result(&self, target_id: i32, damage: i32, attack_flag: i32, speed: u8) -> crate::packets::s2c::S2CGamedataSend {
        self.0.host_attack_result(target_id, damage, attack_flag, speed)
    }

    fn host_attacked(&self, atacante: i32, dano: i32, equipamento: u8, attack_flag: i32, speed: u8) -> crate::packets::s2c::S2CGamedataSend {
        self.0.host_attacked(atacante, dano, equipamento, attack_flag, speed)
    }

    fn self_skill_attack_result(
        &self,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> crate::packets::s2c::S2CGamedataSend {
        self.0.self_skill_attack_result(target_id, skill_id, damage, attack_flag, speed, section)
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
    ) -> crate::packets::s2c::S2CGamedataSend {
        self.0.object_skill_attack_result(attacker_id, target_id, skill_id, damage, attack_flag, speed, section)
    }

    fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, alvo: i32) -> crate::packets::s2c::S2CGamedataSend {
        self.0.npc_info_00(nid, hp, max_hp, alvo)
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
    ) -> crate::packets::s2c::S2CGamedataSend {
        self.0.player_info_00(player_id, level, level2, em_combate, hp, max_hp, mp, max_mp, alvo)
    }

    fn receive_exp(&self, exp: i32, sp: i32) -> crate::packets::s2c::S2CGamedataSend {
        self.0.receive_exp(exp, sp)
    }

    fn equip_item(&self, idx_ivtr: u8, idx_equip: u8, count_ivtr: u32, count_equip: u32) -> crate::packets::s2c::S2CGamedataSend {
        self.0.equip_item(idx_ivtr, idx_equip, count_ivtr, count_equip)
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
    ) -> crate::packets::s2c::S2CGamedataSend {
        self.0.own_ext_prop(status_point, atributos, max_hp, max_mp, max_ap, regen, velocidades, ataque, magico, resistencias, defesa)
    }

    fn enter_sanctuary(&self, id: i32) -> crate::packets::s2c::S2CGamedataSend {
        self.0.enter_sanctuary(id)
    }

    fn leave_sanctuary(&self, id: i32) -> crate::packets::s2c::S2CGamedataSend {
        self.0.leave_sanctuary(id)
    }

    fn inst_data_checkout(
        &self,
        id_inst: i32,
        region: u32,
        precinct: u32,
        gshop: u32,
        gshop2: u32,
        gshop3: Option<u32>,
    ) -> crate::packets::s2c::S2CGamedataSend {
        self.0.inst_data_checkout(id_inst, region, precinct, gshop, gshop2, gshop3)
    }

    fn self_info_1(
        &self,
        exp: i32,
        sp: i32,
        world_id: i32,
        pos: pw_core::Vector3,
        sec_level: u8,
        modo_roupa: bool,
    ) -> crate::packets::s2c::S2CGamedataSend {
        self.0.self_info_1(exp, sp, world_id, pos, sec_level, modo_roupa)
    }

    fn player_enter_world(&self, role_id: i32, vista: pw_core::VistaDoJogador) -> crate::packets::s2c::S2CGamedataSend {
        self.0.player_enter_world(role_id, vista)
    }

    fn player_enter_slice(&self, role_id: i32, vista: pw_core::VistaDoJogador) -> crate::packets::s2c::S2CGamedataSend {
        self.0.player_enter_slice(role_id, vista)
    }
}

pub struct V172Adapter;

impl ProtocolAdapter for V172Adapter {
    fn version(&self) -> GameVersion {
        GameVersion::V1_7_2
    }

    fn encode_online_announce(&self, stream: &mut crate::octets::OctetsStream, userid: i32, localsid: u32) {
        V155Adapter.encode_online_announce(stream, userid, localsid);
    }

    fn encode_role_info(&self, stream: &mut crate::octets::OctetsStream, c: &pw_core::CharacterSummary) {
        V155Adapter.encode_role_info(stream, c);
    }

    fn encode_create_role_response(
        &self,
        stream: &mut crate::octets::OctetsStream,
        result: i32,
        role_id: i32,
        localsid: u32,
        character: Option<&pw_core::CharacterSummary>,
    ) {
        V155Adapter.encode_create_role_response(stream, result, role_id, localsid, character);
    }
}
