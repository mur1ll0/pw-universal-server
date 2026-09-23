use crate::packets::s2c::S2CGamedataSend;
use crate::version::GameVersion;
use pw_core::Vector3;

/// Trait que abstrai a geração de subcomandos do mundo 3D (GamedataSend)
/// cujos layouts variam entre as versões do jogo.
pub trait WorldProtocol: Send + Sync {
    /// Versão do jogo implementada por este protocolo
    fn version(&self) -> GameVersion;

    /// Avisos neutros de status enviados pelo link ao entrar no mundo.
    fn initial_status_notifications(&self, reputation: i32, now: i32) -> Vec<S2CGamedataSend> {
        vec![
            S2CGamedataSend::host_reputation(reputation),
            S2CGamedataSend::pvp_mode(0),
            S2CGamedataSend::self_country_notify(0),
            S2CGamedataSend::server_time(now, 0, 102),
            S2CGamedataSend::trashbox_pwd_state(false),
            S2CGamedataSend::pet_room_capacity(0),
            S2CGamedataSend::self_king_notify(false, 0),
            S2CGamedataSend::faction_contrib_notify(0, 0, 0),
            S2CGamedataSend::player_leadership(0, 0),
            S2CGamedataSend::player_world_contribution(0, 0, 0),
            S2CGamedataSend::player_dividend(0),
            S2CGamedataSend::available_double_exp_time(0),
            S2CGamedataSend::double_exp_time(0, 0),
            S2CGamedataSend::pariah_time(0),
        ]
    }

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

    /// HOST_SKILL_ATTACKED (144): o padrão preserva o layout do 155.
    fn host_skill_attacked(
        &self, attacker_id: i32, skill_id: i32, damage: i32,
        attack_flag: i32, speed: u8, section: u8,
    ) -> S2CGamedataSend {
        S2CGamedataSend::host_skill_attacked(attacker_id, skill_id, damage, attack_flag, speed, section)
    }

    /// NPC_INFO_00 (33)
    fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, alvo: i32) -> S2CGamedataSend;

    /// PLAYER_INFO_00 (32)
    #[allow(clippy::too_many_arguments)]
    fn player_info_00(
        &self,
        player_id: i32,
        level: i16,
        level2: u8,
        // `State`: 1 em combate (`IsCombatState() ? 1 : 0`, `gs/player.cpp:3554`).
        em_combate: bool,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        alvo: i32,
    ) -> S2CGamedataSend;

    /// ELF_EXP (283): clientes anteriores ao comando podem omitir a notificação.
    fn elf_exp(&self, exp: i32) -> Option<S2CGamedataSend> {
        Some(S2CGamedataSend::elf_exp(exp))
    }

    /// RECEIVE_EXP (36)
    fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend;

    /// Layout padrão 155; contadores menores no 126 (B90).
    fn pickup_item(&self, tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> S2CGamedataSend {
        S2CGamedataSend::pickup_item(tid, expire_date, amount, slot_amount, package, slot)
    }

    /// Layout padrão 155; contadores menores no 126 (B90).
    fn obtain_item(&self, tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> S2CGamedataSend {
        S2CGamedataSend::obtain_item(tid, expire_date, amount, slot_amount, package, slot)
    }

    /// Layout padrão 155; contadores menores no 126 (B90).
    fn task_deliver_item(&self, tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> S2CGamedataSend {
        S2CGamedataSend::task_deliver_item(tid, expire_date, amount, slot_amount, package, slot)
    }

    /// Layout padrão 155; contadores menores no 126 (B90).
    fn player_drop_item(&self, package: u8, slot: u8, count: u32, tid: i32, drop_type: u8) -> S2CGamedataSend {
        S2CGamedataSend::player_drop_item(package, slot, count, tid, drop_type)
    }

    /// Layout padrão 155; contadores menores no 126 (B90).
    fn purchase_item(&self, cost: u32, itens: &[(i32, i32, u32, u16)]) -> S2CGamedataSend {
        S2CGamedataSend::purchase_item(cost, itens)
    }

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
    /// `SELF_INFO_1` (8) — a entidade do próprio jogador.
    ///
    /// `modo_roupa` acende `GP_STATE_FASHION` (0x2000) no `state`, e é **daqui** que o
    /// cliente sabe que o dono da tela está de roupa: `CECHostPlayer` lê o próprio estado
    /// deste pacote (`EC_HostPlayer.cpp:819-822`). O `info_player_1` só resolve para quem
    /// **vê** o jogador, não para ele mesmo (B86).
    fn self_info_1(
        &self,
        exp: i32,
        sp: i32,
        world_id: i32,
        pos: Vector3,
        sec_level: u8,
        modo_roupa: bool,
    ) -> S2CGamedataSend;

    /// GET_OWN_MONEY (82)
    fn get_own_money(&self, amount: u32, capacity: u32) -> S2CGamedataSend {
        S2CGamedataSend::get_own_money(amount, capacity)
    }

    /// PLAYER_ENTER_WORLD (17)
    fn player_enter_world(&self, role_id: i32, vista: pw_core::VistaDoJogador) -> S2CGamedataSend;

    /// PLAYER_ENTER_SLICE (12)
    fn player_enter_slice(&self, role_id: i32, vista: pw_core::VistaDoJogador) -> S2CGamedataSend;

    /// SELF_SKILL_INTERRUPTED (87)
    fn self_skill_interrupted(&self, reason: u8) -> S2CGamedataSend {
        S2CGamedataSend::self_skill_interrupted(reason)
    }

    /// SKILL_INTERRUPTED (86)
    fn skill_interrupted(&self, caster: i32) -> S2CGamedataSend {
        S2CGamedataSend::skill_interrupted(caster)
    }

    /// SCENE_SERVICE_NPC_LIST (390)
    fn scene_service_npc_list(&self, npcs: &[(i32, i32)]) -> Option<S2CGamedataSend> {
        Some(S2CGamedataSend::scene_service_npc_list(npcs))
    }
}

// ProtocolAdapter é exportado canonicamente de crate::adapter
pub use crate::adapter::ProtocolAdapter;
