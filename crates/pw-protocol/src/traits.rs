use crate::packets::s2c::S2CGamedataSend;
use crate::version::GameVersion;
use pw_core::Vector3;

/// Trait que abstrai a geração de subcomandos do mundo 3D (GamedataSend)
/// cujos layouts variam entre as versões do jogo.
pub trait WorldProtocol: Send + Sync {
    /// Versão do jogo implementada por este protocolo
    fn version(&self) -> GameVersion;

    /// `sizeof(C2S::npc_sell_item)` no pedido de venda ao NPC (`c2s_SendCmdNPCSevSell`): 16 no
    /// 1.5.5 (`tid, index, count, price`, `EC_GPDataType.h:5462-5468`). O 1.2.6 sobrescreve.
    fn bytes_do_item_vendido(&self) -> usize {
        16
    }

    /// Cabeçalho do pedido de **compra** ao NPC (`c2s_SendCmdNPCSevBuy`), antes da lista de
    /// `{tid, index, count}`: 28 B no 1.5.5 (`money`, cinco campos de contribuição/facção,
    /// `item_count`). O `item_count` é sempre o último `int`. O 1.2.6 sobrescreve.
    fn bytes_do_cabecalho_da_compra(&self) -> usize {
        28
    }

    /// A quantidade do `C2S::DROP_IVTR_ITEM` (14), depois do `u8 index`: `u32` no 1.5.5 (o
    /// corpo de 5 B que o descarte lê desde o B84). `None` quando o corpo não a traz. O 1.2.6
    /// sobrescreve.
    fn quantidade_do_descarte(&self, corpo: &[u8]) -> Option<u32> {
        corpo.get(1..5).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    }

    /// `C2S::MALL_SHOPPING` (106): a primeira entrada do pedido de compra na Loja Gold,
    /// `(goods_id, goods_index, goods_slot)`, ou `None` se o corpo não tiver o tamanho que
    /// o original exige.
    ///
    /// 1.5.5: `{ u32 count; count × { int goods_id; int goods_index; int goods_slot } }`
    /// (`common/protocol.h:5654-5665`); o `gs` recusa com `ERR_FATAL_ERR` se `count == 0`,
    /// `count > 65535` ou o tamanho não for `6 + 12·count` com o cabeçalho, e só compra a
    /// `list[0]` (`PlayerDoShopping(1, …)`, `gs/playercmd.cpp:3299-3318`). O 1.2.6
    /// sobrescreve.
    fn pedido_da_loja_gold(&self, corpo: &[u8]) -> Option<(i32, i32, i32)> {
        entrada_da_loja_gold(corpo, 12, |b| i32::from_le_bytes(b.try_into().unwrap()))
    }

    /// Chi por batimento de 1 s meditando: `ModifyAP(15)` no `sit_down_filter::Heartbeat` do
    /// 1.5.5 (`gs/sitdown_filter.cpp:19-34`). O 1.2.6 sobrescreve.
    fn chi_por_meditacao(&self) -> i32 {
        15
    }

    /// O byte de forma do `PLAYER_CHGSHAPE` (163) a partir do `shape | FORM_CLASS << 6` do
    /// 1.5.5: `ChangeShape(_shape | (FORM_CLASS << 6))` nos filtros de forma
    /// (`cskill/skill/skillfilter.cpp:398`). O 1.2.6 sobrescreve.
    fn byte_de_forma(&self, forma_155: u8) -> u8 {
        forma_155
    }

    /// `UPDATE_EXT_STATE` (124): os seis `DWORD` de estado visual. O 1.2.6 sobrescreve.
    fn update_ext_state(&self, id: i32, estados: [u32; 6]) -> S2CGamedataSend {
        S2CGamedataSend::update_ext_state(id, estados)
    }

    /// `ICON_STATE_NOTIFY` (125): os ícones de estado. O 1.2.6 sobrescreve.
    fn icon_state_notify(&self, id: i32, icones: &[(u16, i32)]) -> S2CGamedataSend {
        S2CGamedataSend::icon_state_notify(id, icones)
    }

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
    /// `estado`: o `state` do `info_npc` — `GP_STATE_NPC_FLY` 0x10000 / `SWIM` 0x20000 do NPC de
    /// ar/água (`SetInhabitMode`, `npc.cpp:823-843`; o cliente o põe no ar, `EC_NPC.cpp:411-416`).
    fn npc_enter_world(&self, nid: i32, tid: i32, pos: Vector3, dir: u8, estado: u32) -> S2CGamedataSend;

    /// NPC_ENTER_SLICE (11)
    fn npc_enter_slice(&self, nid: i32, tid: i32, pos: Vector3, dir: u8, estado: u32) -> S2CGamedataSend;

    /// NPC_INFO_LIST (9) com um NPC — a resposta ao `QUERY_NPC_INFO_1`
    /// (`gnpc_dispatcher::query_info_1`, `gs/npc.cpp:331-338`: `multi_data_header` com
    /// `count = 1` e a `INFO::npc_info`). O corpo é a mesma `INFO::npc_info` do
    /// `NPC_ENTER_WORLD`, então sai do codificador de cada versão, trocando o cabeçalho.
    fn npc_info_list_de_um(&self, nid: i32, tid: i32, pos: Vector3, dir: u8, estado: u32) -> S2CGamedataSend {
        let corpo = self.npc_enter_world(nid, tid, pos, dir, estado).data;
        let mut data = Vec::with_capacity(corpo.len() + 2);
        data.extend_from_slice(&9u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&corpo[2..]);
        S2CGamedataSend { data }
    }

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

    /// `svr_monster_killed` no `TASK_VAR_DATA` (106): no 1.5.3/1.5.5 com `dps`/`dph`
    /// (`cgame/gs/task/TaskTempl.h:1773-1779`, 17 bytes); o 1.2.6 sobrescreve (9 bytes).
    fn task_notify_monster_killed(&self, task_id: u16, monster_id: u32, monster_num: u16) -> S2CGamedataSend {
        S2CGamedataSend::task_notify_monster_killed(task_id, monster_id, monster_num, 0, 0)
    }

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

    /// EQUIP_DATA_CHANGED (67)
    fn equip_data_changed(&self, player_id: i32, crc: u16, mask_add: u64, mask_del: u64, items: &[i32]) -> S2CGamedataSend {
        S2CGamedataSend::equip_data_changed(player_id, crc, mask_add, mask_del, items)
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

    // ------------------------------------------------------------------ mascote
    //
    // Layouts do 1.5.5 (`Network/EC_GPDataType.h`, `cmd_summon_pet` e vizinhos). O 1.2.6
    // sobrescreve os que o validador do cliente mede diferente (VA 0x584610, tabela em
    // 0x584e90): 233 com 12 B, 234 com 8 B e 249 com 12 B, e o `info_npc` de 27 B.

    /// SUMMON_PET (233) — `{slot_index, pet_tid, pet_pid, life_time}`, 16 B.
    fn summon_pet(&self, slot: i32, pet_tid: i32, pet_pid: i32, life_time: i32) -> S2CGamedataSend {
        S2CGamedataSend::summon_pet(slot, pet_tid, pet_pid, life_time)
    }

    /// RECALL_PET (234) — `{slot_index, pet_id, char reason}`, 9 B.
    fn recall_pet(&self, slot: i32, pet_tid: i32, motivo: u8) -> S2CGamedataSend {
        S2CGamedataSend::recall_pet(slot, pet_tid, motivo)
    }

    /// PET_HP_NOTIFY (249) — `{pet_index, hp_factor, cur_hp, mp_factor, cur_mp}`, 20 B.
    fn pet_hp_notify(&self, slot: i32, hp_factor: f32, hp: i32, mp_factor: f32, mp: i32) -> S2CGamedataSend {
        mascote_s2c(249, |s| {
            s.write_i32_le(slot);
            s.write_f32_le(hp_factor);
            s.write_i32_le(hp);
            s.write_f32_le(mp_factor);
            s.write_i32_le(mp);
        })
    }

    /// FREE_PET (232) — `{slot_index, pet_id}`, 8 B nas duas versões.
    fn free_pet(&self, slot: i32, pet_tid: i32) -> S2CGamedataSend {
        S2CGamedataSend::free_pet(slot, pet_tid)
    }

    /// PET_SET_COOLDOWN (252) — `{pet_index, cooldown_index, cooldown_time}`, 12 B nas duas
    /// versões.
    fn pet_set_cooldown(&self, slot: i32, recarga: i32, ms: i32) -> S2CGamedataSend {
        S2CGamedataSend::pet_set_cooldown(slot, recarga, ms)
    }

    /// PET_AI_STATE (250) — `{u8 attack, u8 move}`, 2 B nas duas versões.
    fn pet_ai_state(&self, agressividade: u8, movimento: u8) -> S2CGamedataSend {
        mascote_s2c(250, |s| {
            s.write_u8(agressividade);
            s.write_u8(movimento);
        })
    }

    /// PET_DEAD (247) — `{pet_index}`, 4 B.
    fn pet_dead(&self, slot: i32) -> S2CGamedataSend {
        mascote_s2c(247, |s| s.write_i32_le(slot))
    }

    /// PET_REVIVE (248) — `{pet_index, float hp_factor}`, 8 B.
    fn pet_revive(&self, slot: i32, hp_factor: f32) -> S2CGamedataSend {
        mascote_s2c(248, |s| {
            s.write_i32_le(slot);
            s.write_f32_le(hp_factor);
        })
    }

    /// PET_RECEIVE_EXP (237) — `{slot_index, pet_id, exp}`, 12 B.
    fn pet_receive_exp(&self, slot: i32, pet_tid: i32, exp: i32) -> S2CGamedataSend {
        mascote_s2c(237, |s| {
            s.write_i32_le(slot);
            s.write_i32_le(pet_tid);
            s.write_i32_le(exp);
        })
    }

    /// PET_LEVELUP (238) — `{slot_index, pet_id, level, exp}`, 16 B.
    fn pet_levelup(&self, slot: i32, pet_tid: i32, nivel: i32, exp: i32) -> S2CGamedataSend {
        mascote_s2c(238, |s| {
            s.write_i32_le(slot);
            s.write_i32_le(pet_tid);
            s.write_i32_le(nivel);
            s.write_i32_le(exp);
        })
    }

    /// PET_HONOR_POINT (241) — `{index, cur_honor_point}`, 8 B.
    fn pet_honor_point(&self, slot: i32, lealdade: i32) -> S2CGamedataSend {
        mascote_s2c(241, |s| {
            s.write_i32_le(slot);
            s.write_i32_le(lealdade);
        })
    }

    /// PET_HUNGER_GAUGE (242) — `{index, cur_hunge_gauge}`, 8 B.
    fn pet_hunger_gauge(&self, slot: i32, fome: i32) -> S2CGamedataSend {
        mascote_s2c(242, |s| {
            s.write_i32_le(slot);
            s.write_i32_le(fome);
        })
    }

    /// TEAM_MEMBER_DATA (64) — cabeçalho de 6 B e 34 B por membro no 1.5.5
    /// ([`S2CGamedataSend::team_member_data`]). O 1.2.6 sobrescreve (25 B por membro).
    fn team_member_data(&self, lider: i32, membros: &[crate::packets::s2c::MembroDoGrupo]) -> S2CGamedataSend {
        S2CGamedataSend::team_member_data(lider, membros)
    }

    /// NOTIFY_HOSTPOS (14) — `{A3DVECTOR pos; int tag; int line}`, 20 B no 1.5.5
    /// (`EC_GPDataType.h:1362-1367`). O 1.2.6 sobrescreve (16 B, sem `line`).
    fn notify_hostpos(&self, pos: Vector3, tag: i32, linha: i32) -> S2CGamedataSend {
        S2CGamedataSend::notify_hostpos(pos, tag, linha)
    }

    /// PLAYER_MOUNTING (227) — `{ int id; int mount_id; u16 mount_color }`, 10 B no 1.5.5
    /// (`gplayer_imp::ActiveMountState`, `gs/player.cpp:14279-14299`; zero nos dois é o
    /// desmontar). O 1.2.6 sobrescreve (9 B).
    fn player_mounting(&self, player_id: i32, mount_id: i32, mount_color: u16) -> S2CGamedataSend {
        S2CGamedataSend::player_mounting(player_id, mount_id, mount_color)
    }

    /// ENCHANT_RESULT (139) — bênção/maldição que pegou: `{caster, target, skill, char level,
    /// char orange_name, int attack_flag, byte section}`, 19 B no 1.5.5 (`cmd_enchant_result`,
    /// `EC_GPDataType.h`). O 1.2.6 sobrescreve (16 B).
    #[allow(clippy::too_many_arguments)]
    fn enchant_result(&self, caster: i32, alvo: i32, skill: i32, nivel: u8, orange_name: bool, attack_flag: i32, section: u8) -> S2CGamedataSend {
        S2CGamedataSend::enchant_result(caster, alvo, skill, nivel, orange_name, attack_flag, section)
    }

    /// OBJECT_ATTACK_RESULT (120) — um golpe entre duas criaturas, para quem vê:
    /// `{attacker_id, target_id, damage, int attack_flag, char speed}`, 17 B no 1.5.5
    /// (`cmd_object_atk_result`, `EC_GPDataType.h`).
    fn object_attack_result(&self, atacante: i32, alvo: i32, dano: i32, attack_flag: i32, speed: u8) -> S2CGamedataSend {
        mascote_s2c(120, |s| {
            s.write_i32_le(atacante);
            s.write_i32_le(alvo);
            s.write_i32_le(dano);
            s.write_i32_le(attack_flag);
            s.write_u8(speed);
        })
    }

    /// NPC_ENTER_WORLD (16) / NPC_ENTER_SLICE (11) de um mascote: o `info_npc` com
    /// `GP_STATE_NPC_PET` (0x1000) e o id do dono logo depois; com nome, também
    /// `GP_STATE_NPC_NAME` (0x2000) + `u8` tamanho + bytes (`EC_GPDataType.h:725-770`;
    /// `CreatePet`, `obj_interface.cpp:2826-2880`). 1.5.5: 35 B de base, com `vis_tid` e
    /// `state2`. `ambiente`: `GP_STATE_NPC_FLY` 0x10000 / `GP_STATE_NPC_SWIM` 0x20000
    /// (`SetInhabitMode`, `npc.cpp:823-843`), que o cliente usa para pôr o NPC no ar
    /// (`CECNPC::Init`, `EC_NPC.cpp:411-416`); sem bytes a mais.
    #[allow(clippy::too_many_arguments)]
    fn mascote_entra(&self, comando: u16, nid: i32, tid: i32, vis_tid: i32, pos: Vector3, dir: u8, dono: i32, nome: &[u8], ambiente: i32) -> S2CGamedataSend {
        mascote_s2c(comando, |s| {
            s.write_i32_le(nid);
            s.write_i32_le(tid);
            s.write_i32_le(vis_tid);
            s.write_f32_le(pos.x);
            s.write_f32_le(pos.y);
            s.write_f32_le(pos.z);
            s.write_u16_le(0);
            s.write_u8(dir);
            s.write_i32_le(estado_do_mascote(nome) | ambiente);
            s.write_i32_le(0);
            cauda_do_mascote(s, dono, nome);
        })
    }
}

/// Um S2C de mascote: o número e o corpo.
pub fn mascote_s2c(comando: u16, corpo: impl FnOnce(&mut crate::octets::OctetsStream)) -> S2CGamedataSend {
    let mut s = crate::octets::OctetsStream::new();
    s.write_u16_le(comando);
    corpo(&mut s);
    S2CGamedataSend { data: s.into_bytes().to_vec() }
}

/// `GP_STATE_NPC_PET`, mais `GP_STATE_NPC_NAME` quando há nome.
pub fn estado_do_mascote(nome: &[u8]) -> i32 {
    0x1000 | if nome.is_empty() { 0 } else { 0x2000 }
}

/// O que vem depois do `info_npc` de um mascote: o dono e, com nome, tamanho + bytes.
pub fn cauda_do_mascote(s: &mut crate::octets::OctetsStream, dono: i32, nome: &[u8]) {
    s.write_i32_le(dono);
    if !nome.is_empty() {
        let n = nome.len().min(u8::MAX as usize);
        s.write_u8(n as u8);
        for b in &nome[..n] {
            s.write_u8(*b);
        }
    }
}

// ProtocolAdapter é exportado canonicamente de crate::adapter
pub use crate::adapter::ProtocolAdapter;

/// Confere `{u32 count; count × entrada}` com `count` em `1..=65535` e o corpo exato, e
/// devolve os três campos da primeira entrada, cada um com `bytes/3` bytes.
pub fn entrada_da_loja_gold(corpo: &[u8], bytes: usize, ler: impl Fn(&[u8]) -> i32) -> Option<(i32, i32, i32)> {
    let count = u32::from_le_bytes(corpo.get(0..4)?.try_into().ok()?) as usize;
    if count == 0 || count > 65535 || corpo.len() != 4 + bytes * count {
        return None;
    }
    let c = bytes / 3;
    let e = &corpo[4..4 + bytes];
    Some((ler(&e[0..c]), ler(&e[c..2 * c]), ler(&e[2 * c..3 * c])))
}
