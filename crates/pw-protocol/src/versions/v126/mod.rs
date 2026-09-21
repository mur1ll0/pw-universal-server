pub mod role;

use crate::octets::OctetsStream;
use crate::packets::s2c::S2CGamedataSend;
use crate::traits::{ProtocolAdapter, WorldProtocol};
use crate::version::GameVersion;
use crate::versions::common::{estreitar, estreitar_u16, so_o_cabecalho};
use pw_core::{CharacterSummary, Vector3, VistaDoJogador};

pub struct V126Protocol;

impl WorldProtocol for V126Protocol {
    fn version(&self) -> GameVersion {
        GameVersion::V1_2_6
    }

    fn scene_service_npc_list(&self, _npcs: &[(i32, i32)]) -> Option<S2CGamedataSend> {
        // elementclient.exe 126, validador 0x584610: ids acima de 260
        // retornam inválido (docs/evidencias/126/cliente-validacao-entrada.txt:11).
        None
    }

    fn initial_status_notifications(&self, reputation: i32, now: i32) -> Vec<S2CGamedataSend> {
        // O validador 0x584610 do cliente 126 aceita somente ids até 260.
        // SERVER_TIME=102 também consta em s2c-114.txt:2 (full_interno.pcap).
        vec![
            S2CGamedataSend::host_reputation(reputation),
            S2CGamedataSend::pvp_mode(0),
            S2CGamedataSend::server_time(now, 0, 102),
            S2CGamedataSend::trashbox_pwd_state(false),
            S2CGamedataSend::pet_room_capacity(0),
            S2CGamedataSend::available_double_exp_time(0),
            S2CGamedataSend::double_exp_time(0, 0),
            S2CGamedataSend::pariah_time(0),
        ]
    }

    fn task_data(&self) -> S2CGamedataSend {
        // 1.2.6: 3 blocos (medido na desmontagem de elementclient.exe do 1.2.6)
        let mut s = OctetsStream::new();
        s.write_u16_le(105);
        for _ in 0..3 {
            s.write_u32_le(0);
        }
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn task_data_com_listas(&self, blocos: [&[u8]; 5]) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(105);
        for b in blocos.iter().take(3) {
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
        // 10 bytes: idTarget(4), iDamage(4), attack_flag(1), speed(1)
        let mut s = OctetsStream::new();
        s.write_u16_le(24);
        s.write_i32_le(target_id);
        s.write_i32_le(damage);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn host_attacked(&self, atacante: i32, dano: i32, equipamento: u8, attack_flag: i32, speed: u8) -> S2CGamedataSend {
        // 11 bytes: idAttacker(4), iDamage(4), cEquipment(1), attack_flag(1), speed(1)
        let mut s = OctetsStream::new();
        s.write_u16_le(26);
        s.write_i32_le(atacante);
        s.write_i32_le(dano);
        s.write_u8(equipamento);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn self_skill_attack_result(
        &self,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        _section: u8,
    ) -> S2CGamedataSend {
        // 14 bytes: target(4), skill(4), damage(4), attack_flag(1), speed(1) - sem section
        let mut s = OctetsStream::new();
        s.write_u16_le(142);
        s.write_i32_le(target_id);
        s.write_i32_le(skill_id);
        s.write_i32_le(damage);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn object_skill_attack_result(
        &self,
        attacker_id: i32,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        _section: u8,
    ) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(143);
        s.write_i32_le(attacker_id);
        s.write_i32_le(target_id);
        s.write_i32_le(skill_id);
        s.write_i32_le(damage);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, _alvo: i32) -> S2CGamedataSend {
        // 12 bytes no 1.2.6: sem iTargetID
        let mut s = OctetsStream::new();
        s.write_u16_le(33);
        s.write_i32_le(nid);
        s.write_i32_le(hp);
        s.write_i32_le(max_hp);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn player_info_00(
        &self,
        player_id: i32,
        level: i16,
        level2: u8,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        _alvo: i32,
    ) -> S2CGamedataSend {
        // 24 bytes no 1.2.6: sem iTargetID
        let mut s = OctetsStream::new();
        s.write_u16_le(32);
        s.write_i32_le(player_id);
        s.write_i16_le(level);
        s.write_u8(0);
        s.write_u8(level2);
        s.write_i32_le(hp);
        s.write_i32_le(max_hp);
        s.write_i32_le(mp);
        s.write_i32_le(max_mp);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend {
        // 4 bytes: u16 exp, u16 sp
        let mut s = OctetsStream::new();
        s.write_u16_le(36);
        s.write_u16_le(estreitar_u16(exp));
        s.write_u16_le(estreitar_u16(sp));
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn equip_item(&self, idx_ivtr: u8, idx_equip: u8, count_ivtr: u32, count_equip: u32) -> S2CGamedataSend {
        // 6 bytes: u8 idx_ivtr, u8 idx_equip, u16 count_ivtr, u16 count_equip
        let mut s = OctetsStream::new();
        s.write_u16_le(48);
        s.write_u8(idx_ivtr);
        s.write_u8(idx_equip);
        s.write_u16_le(count_ivtr.min(u16::MAX as u32) as u16);
        s.write_u16_le(count_equip.min(u16::MAX as u32) as u16);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn equip_data(&self, player_id: i32, crc: u16, mask: u64, items: &[i32]) -> S2CGamedataSend {
        // full_interno.pcap, S2C 66 #1: CRC 0x1c54, jogador 48,
        // máscara 0x11, itens 2258 e 154: payload de 18 bytes.
        // docs/evidencias/126/s2c-66.txt:8-10. A máscara tem 32 bits.
        let mask = mask as u32;
        let mut s = OctetsStream::new();
        s.write_u16_le(66);
        s.write_u16_le(crc);
        s.write_i32_le(player_id);
        s.write_u32_le(mask);
        // O mundo fornece os itens em ordem crescente de slot; os slots
        // acima de 31 não cabem no layout e não podem sobrar após a lista.
        for item in items.iter().take(mask.count_ones() as usize) {
            s.write_i32_le(*item);
        }
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

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
    ) -> S2CGamedataSend {
        let (vitality, energy, strength, agility) = atributos;
        let (hp_gen, mp_gen) = regen;
        let (walk, run, swim, fly) = velocidades;
        let (attack_rate, damage_low, damage_high, attack_speed, attack_range) = ataque;
        let (defense, armor) = defesa;

        let mut s = OctetsStream::new();
        s.write_u16_le(50);
        s.write_u32_le(status_point);

        // ROLEEXTPROP_BASE (32B)
        s.write_i32_le(vitality);
        s.write_i32_le(energy);
        s.write_i32_le(strength);
        s.write_i32_le(agility);
        s.write_i32_le(max_hp);
        s.write_i32_le(max_mp);
        s.write_i32_le(hp_gen);
        s.write_i32_le(mp_gen);

        // ROLEEXTPROP_MOVE (16B)
        s.write_f32_le(walk);
        s.write_f32_le(run);
        s.write_f32_le(swim);
        s.write_f32_le(fly);

        // ROLEEXTPROP_ATK (68B)
        s.write_i32_le(attack_rate);
        s.write_i32_le(damage_low);
        s.write_i32_le(damage_high);
        s.write_i32_le(attack_speed);
        s.write_f32_le(attack_range);
        for _ in 0..5 {
            s.write_i32_le(0);
            s.write_i32_le(0);
        }
        s.write_i32_le(0);
        s.write_i32_le(0);

        // ROLEEXTPROP_DEF (28B)
        for _ in 0..5 {
            s.write_i32_le(0);
        }
        s.write_i32_le(defense);
        s.write_i32_le(armor);

        s.write_i32_le(0); // max_ap -> 152 bytes exatos

        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn enter_sanctuary(&self, _id: i32) -> S2CGamedataSend {
        so_o_cabecalho(164)
    }

    fn leave_sanctuary(&self, _id: i32) -> S2CGamedataSend {
        so_o_cabecalho(165)
    }

    fn inst_data_checkout(
        &self,
        id_inst: i32,
        region: u32,
        precinct: u32,
        gshop: u32,
        _gshop2: u32,
        _gshop3: Option<u32>,
    ) -> S2CGamedataSend {
        // 16 bytes: id_inst(4), region(4), precinct(4), gshop(4)
        let mut s = OctetsStream::new();
        s.write_u16_le(206);
        s.write_i32_le(id_inst);
        s.write_u32_le(region);
        s.write_u32_le(precinct);
        s.write_u32_le(gshop);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn self_info_1(
        &self,
        exp: i32,
        sp: i32,
        world_id: i32,
        pos: Vector3,
        sec_level: u8,
    ) -> S2CGamedataSend {
        // 34 bytes (sem state2)
        S2CGamedataSend::self_info_1(exp, sp, world_id, pos, sec_level)
    }

    fn player_enter_world(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        self.info_player_1_126(17, role_id, vista)
    }

    fn player_enter_slice(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        self.info_player_1_126(12, role_id, vista)
    }
}

impl V126Protocol {
    fn info_npc(&self, comando: u16, nid: i32, tid: i32, pos: Vector3, dir: u8) -> S2CGamedataSend {
        // 27 bytes no 1.2.6 (sem vis_tid e sem state2)
        let mut s = OctetsStream::new();
        s.write_u16_le(comando);
        s.write_i32_le(nid);
        s.write_i32_le(tid);
        s.write_f32_le(pos.x);
        s.write_f32_le(pos.y);
        s.write_f32_le(pos.z);
        s.write_u16_le(0);
        s.write_u8(dir);
        s.write_i32_le(0);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn info_player_1_126(&self, cmd: u16, role_id: i32, v: VistaDoJogador) -> S2CGamedataSend {
        // 28 bytes de payload (sem state2)
        let mut s = OctetsStream::new();
        s.write_u16_le(cmd);
        s.write_i32_le(role_id);
        s.write_f32_le(v.pos.x);
        s.write_f32_le(v.pos.y);
        s.write_f32_le(v.pos.z);
        s.write_u16_le(v.crc_equipamento);
        s.write_u16_le(v.crc_aparencia);
        s.write_u8(v.dir);
        s.write_u8(v.cultivo);
        let state = if v.sec_level > 0 { 0x0000_4000 } else { 0 };
        s.write_i32_le(state);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }
}

pub struct V126Adapter;

impl ProtocolAdapter for V126Adapter {
    fn version(&self) -> GameVersion {
        GameVersion::V1_2_6
    }

    fn encode_online_announce(&self, stream: &mut OctetsStream, userid: i32, localsid: u32) {
        stream.write_i32(userid);
        stream.write_u32(localsid);
        stream.write_i32(0);
        stream.write_i8(1);
        stream.write_i32(0);
        stream.write_i32(0);
        stream.write_i32(0);
        // Sem flags de 1.4+
    }

    fn encode_role_info(&self, stream: &mut OctetsStream, c: &CharacterSummary) {
        role::encode_role_info_126(stream, c);
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
        // Sem refretcode no 1.2.6
    }
}
