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

    // Mascote — o validador do cliente 1.2.6 (VA 0x584610, tabela de saltos em 0x584e90)
    // mede 233 = 12 B, 234 = 8 B e 249 = 12 B; a captura `full_interno.pcap` confirma 233 e 234.

    /// OBJECT_ATTACK_RESULT (120) com o `attack_flag` de 1 byte, como o 24 e o 26 do 1.2.6:
    /// 14 B no validador do cliente (caso 120, VA 0x584e7a).
    fn object_attack_result(&self, atacante: i32, alvo: i32, dano: i32, attack_flag: i32, speed: u8) -> S2CGamedataSend {
        crate::traits::mascote_s2c(120, |s| {
            s.write_i32_le(atacante);
            s.write_i32_le(alvo);
            s.write_i32_le(dano);
            s.write_i8(estreitar(attack_flag));
            s.write_u8(speed);
        })
    }

    /// O cliente 1.2.6 manda o item vendido sem o `price`: `{tid, index, count}`, 12 B — medido
    /// no pedido real (`len` 148 = 4 + 12 × 12, B116).
    fn bytes_do_item_vendido(&self) -> usize {
        12
    }

    /// O pedido de compra do cliente 1.2.6 tem cabeçalho de 8 B — `money`, `item_count` —,
    /// sem os campos de contribuição/facção do 1.5.5: capturado na VM original
    /// (`_sync/capturas/full_interno.pcap`, subcomando 37 #0: `service_type 1`, `len 20` =
    /// 8 + 1 × 12). Com 28 B a lista saía vazia e a compra não fazia nada.
    fn bytes_do_cabecalho_da_compra(&self) -> usize {
        8
    }

    /// O `gplayer_controller::CommandHandler` do `gs` 1.2.6 exige 5 B no comando 14 (com o
    /// cabeçalho; VA 0x80ce383) e passa ao `PlayerDropInvItem` o `byte [+2]` e o `word [+3]`
    /// (VA 0x80ce44c-0x80ce45f): `{u8 index; u16 amount}`. Lido como o 1.5.5, o corpo de 3 B
    /// não trazia quantidade e ia a pilha inteira (B122).
    fn quantidade_do_descarte(&self, corpo: &[u8]) -> Option<u32> {
        corpo.get(1..3).map(|b| u16::from_le_bytes(b.try_into().unwrap()) as u32)
    }

    /// O `CommandHandler` do `gs` 1.2.6 aceita o comando 106 com `6 + 6·count` bytes (com o
    /// cabeçalho; VA 0x80d1637-0x80d1666) e passa a lista a `PlayerDoShopping(unsigned,
    /// const short*)` (VA 0x807f8e0), que lê cada entrada como três `short` com sinal
    /// (`movsx`, VA 0x807f9f1-0x807fa19): `{short goods_id; short goods_index; short
    /// goods_slot}`.
    fn pedido_da_loja_gold(&self, corpo: &[u8]) -> Option<(i32, i32, i32)> {
        crate::traits::entrada_da_loja_gold(corpo, 6, |b| i16::from_le_bytes(b.try_into().unwrap()) as i32)
    }

    /// O `sit_down_filter::Heartbeat` do `gs` 1.2.6 (VA 0x812ff22) só chama
    /// `EnhanceScaleHPGen/MPGen` e `UpdateHPMPGen` — meditar não dá chi (B119).
    fn chi_por_meditacao(&self) -> i32 {
        0
    }

    /// O `filter_Foxform::OnAttach` do `gs` 1.2.6 chama `ChangeShape(1)` — o `_shape` sem o
    /// `FORM_CLASS << 6` (VA 0x830b1a0: `push 1` antes do `object_interface::ChangeShape`) —, e
    /// o `gactive_imp::ChangeShape` 1.2.6 (VA 0x811d78a) guarda o valor inteiro como forma:
    /// `shape_form = shape & 0xff`, `_cur_form = shape`. O byte do cliente é só o `_shape`.
    fn byte_de_forma(&self, forma_155: u8) -> u8 {
        forma_155 & 0x3f
    }

    /// `UPDATE_EXT_STATE` (124) do 1.2.6: `{int id; DWORD state}`, 8 bytes — o validador do
    /// `elementclient.exe` 1.2.6 exige 8 (`docs/evidencias/126/tamanhos_s2c_126.txt`) e o `gs`
    /// 1.2.6 monta com `update_visible_state(gobject*, unsigned int)` (VA 0x80a6ed2): um só
    /// `DWORD`, os estados 0..31 (B124).
    fn update_ext_state(&self, id: i32, estados: [u32; 6]) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(124);
        s.write_i32_le(id);
        s.write_u32_le(estados[0]);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `ICON_STATE_NOTIFY` (125) do 1.2.6: `{int id; u16 count; u16 state[count]}`, **sem** a
    /// lista de parâmetros do 1.5.5 — o validador do cliente 1.2.6 (VA 0x584ba3) exige
    /// `tamanho == 6 + 2 × count` (`lea eax, [eax + eax + 6]`) e descartava o formato do 1.5.5 em
    /// silêncio: nenhum ícone de bênção aparecia no 1.2.6 (B124).
    fn icon_state_notify(&self, id: i32, icones: &[(u16, i32)]) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(125);
        s.write_i32_le(id);
        s.write_u16_le(icones.len() as u16);
        for (estado, _) in icones {
            s.write_u16_le(*estado);
        }
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// ENCHANT_RESULT (139) em 16 B: `{caster, target, skill, char level, char orange_name,
    /// char modifier, char modifier2}` — validador do cliente
    /// 1.2.6 (caso 139, VA 0x584e52) e o montador do `gs` 1.2.6
    /// (`S2C::CMD::Make<enchant_result>::From(…, int skill, char, char, char, char)`, VA
    /// 0x80a6f0e: três `int` e quatro `char`). Com os 19 B do 1.5.5 o cliente descartava toda
    /// bênção/maldição em silêncio (B116).
    ///
    /// B118 — os dois últimos bytes **não** são `attack_flag` e `section`: o `gs` 1.2.6 passa
    /// `immune & 0xff` e `(immune & 0xff00) >> 8` (`SkillWrapper::Attack(…enchant_msg…)`, VA
    /// 0x831bb81-0x831bb93), o `modifier`/`modifier2` que o cliente junta em
    /// `(modifier2 << 8) | modifier` (a linha comentada em `EC_Player.cpp:7256`). Mandar o
    /// `section` 1 ali acendia o `0x100` = `MOD_ENCHANT_FAILED`, e toda bênção mostrava "FALHA".
    /// TEAM_MEMBER_DATA (64) em `6 + 25·n`: o `Make<team_member_data>` do `gs` 1.2.6 (VA
    /// 0x808fe34) escreve `u8 member_count, u8 data_count, int idLeader` e por membro `int id,
    /// short level, char state, u8 level2`, um `char` do `member_entry` (+0x20) e `hp, mp,
    /// max_hp, max_mp` — sem `reincarnation_times`, `force_id` e `profit_level` do 1.5.5. Na
    /// captura (`_sync/capturas/full_interno.pcap`, 3 comandos, 6 membros) esse `char` é
    /// sempre `0xff` (B122).
    fn team_member_data(&self, lider: i32, membros: &[crate::packets::s2c::MembroDoGrupo]) -> S2CGamedataSend {
        crate::traits::mascote_s2c(64, |s| {
            s.write_u8(membros.len() as u8);
            s.write_u8(membros.len() as u8);
            s.write_i32_le(lider);
            for m in membros {
                s.write_i32_le(m.role_id);
                s.write_i16_le(m.level);
                s.write_u8(m.state);
                s.write_u8(m.level2);
                s.write_u8(0xff);
                s.write_i32_le(m.hp);
                s.write_i32_le(m.mp);
                s.write_i32_le(m.max_hp);
                s.write_i32_le(m.max_mp);
            }
        })
    }

    /// NOTIFY_HOSTPOS (14) em 16 B: o `Make<notify_pos>::From(…, A3DVECTOR, int)` do `gs` 1.2.6
    /// (VA 0x808ed9d) escreve `pos` e `tag`, sem a linha do mundo paralelo; o validador do cliente
    /// 1.2.6 pede 16 e a captura tem 16 × 7. Os 20 B do 1.5.5 eram descartados — teleporte e
    /// volta para a cidade não reposicionavam o cliente (B122).
    fn notify_hostpos(&self, pos: Vector3, tag: i32, _linha: i32) -> S2CGamedataSend {
        crate::traits::mascote_s2c(14, |s| {
            s.write_f32_le(pos.x);
            s.write_f32_le(pos.y);
            s.write_f32_le(pos.z);
            s.write_i32_le(tag);
        })
    }

    /// PLAYER_MOUNTING (227) em 9 B: `S2C::CMD::Make<player_mounting>::From(…, XID, int, char)`
    /// do `gs` 1.2.6 (VA 0x8092326) escreve `id`, `mount_id` e a cor como `char`; o validador do
    /// cliente 1.2.6 pede 9 (`docs/evidencias/126/tamanhos_s2c_126.txt`). Os 10 B do 1.5.5 eram
    /// descartados: quem estava vendo não via o outro montar (B122).
    fn player_mounting(&self, player_id: i32, mount_id: i32, mount_color: u16) -> S2CGamedataSend {
        crate::traits::mascote_s2c(227, |s| {
            s.write_i32_le(player_id);
            s.write_i32_le(mount_id);
            s.write_u8(mount_color as u8);
        })
    }

    fn enchant_result(&self, caster: i32, alvo: i32, skill: i32, nivel: u8, orange_name: bool, attack_flag: i32, _section: u8) -> S2CGamedataSend {
        crate::traits::mascote_s2c(139, |s| {
            s.write_i32_le(caster);
            s.write_i32_le(alvo);
            s.write_i32_le(skill);
            s.write_u8(nivel);
            s.write_u8(orange_name as u8);
            s.write_u8((attack_flag & 0xff) as u8);
            s.write_u8(((attack_flag & 0xff00) >> 8) as u8);
        })
    }

    /// SUMMON_PET (233) sem o `life_time`: `{slot_index, pet_tid, pet_pid}`.
    fn summon_pet(&self, slot: i32, pet_tid: i32, pet_pid: i32, _life_time: i32) -> S2CGamedataSend {
        crate::traits::mascote_s2c(233, |s| {
            s.write_i32_le(slot);
            s.write_i32_le(pet_tid);
            s.write_i32_le(pet_pid);
        })
    }

    /// RECALL_PET (234) sem o `reason`: `{slot_index, pet_id}`.
    fn recall_pet(&self, slot: i32, pet_tid: i32, _motivo: u8) -> S2CGamedataSend {
        crate::traits::mascote_s2c(234, |s| {
            s.write_i32_le(slot);
            s.write_i32_le(pet_tid);
        })
    }

    /// PET_HP_NOTIFY (249) sem a mana: `{pet_index, hp_factor, cur_hp}`.
    fn pet_hp_notify(&self, slot: i32, hp_factor: f32, hp: i32, _mp_factor: f32, _mp: i32) -> S2CGamedataSend {
        crate::traits::mascote_s2c(249, |s| {
            s.write_i32_le(slot);
            s.write_f32_le(hp_factor);
            s.write_i32_le(hp);
        })
    }

    /// O `info_npc` de 27 B (sem `vis_tid` nem `state2`, estado em +0x17) e a mesma cauda: o
    /// validador do caso 16 soma 4 com o bit 0x1000 e `1 + tamanho` com o 0x2000
    /// (VA 0x58486a-0x5848a2).
    #[allow(clippy::too_many_arguments)]
    fn mascote_entra(&self, comando: u16, nid: i32, tid: i32, _vis_tid: i32, pos: Vector3, dir: u8, dono: i32, nome: &[u8], ambiente: i32) -> S2CGamedataSend {
        crate::traits::mascote_s2c(comando, |s| {
            s.write_i32_le(nid);
            s.write_i32_le(tid);
            s.write_f32_le(pos.x);
            s.write_f32_le(pos.y);
            s.write_f32_le(pos.z);
            s.write_u16_le(0);
            s.write_u8(dir);
            // O bit de ambiente do NPC é o mesmo no fonte 1.5.3 (`GP_STATE_NPC_FLY` 0x10000,
            // `EC_GPDataType.h:237`) e o validador do caso 16 não soma bytes por ele.
            s.write_i32_le(crate::traits::estado_do_mascote(nome) | ambiente);
            crate::traits::cauda_do_mascote(s, dono, nome);
        })
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
        for (i, b) in blocos.iter().take(3).enumerate() {
            let convertido;
            let b: &[u8] = if i == 1 {
                convertido = concluidas_no_formato_antigo(b);
                &convertido
            } else {
                b
            };
            s.write_u32_le(b.len() as u32);
            s.write_raw_bytes(b);
        }
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn npc_enter_world(&self, nid: i32, tid: i32, pos: Vector3, dir: u8, estado: u32) -> S2CGamedataSend {
        self.info_npc(16, nid, tid, pos, dir, estado)
    }

    fn npc_enter_slice(&self, nid: i32, tid: i32, pos: Vector3, dir: u8, estado: u32) -> S2CGamedataSend {
        self.info_npc(11, nid, tid, pos, dir, estado)
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

    fn host_skill_attacked(
        &self, attacker_id: i32, skill_id: i32, damage: i32,
        attack_flag: i32, speed: u8, _section: u8,
    ) -> S2CGamedataSend {
        // full_interno.pcap, S2C 144 #0 (s2c-144.txt:2): 15 bytes.
        // Cliente 126: caso 144 na tabela VA 0x584e90 (cliente-validacao-combate.txt).
        // attack_flag ocupa um byte; não existe section, como nos comandos 142/143.
        let mut s = OctetsStream::new();
        s.write_u16_le(144);
        s.write_i32_le(attacker_id);
        s.write_i32_le(skill_id);
        s.write_i32_le(damage);
        s.write_u8(0x7f); // nenhuma peça desgastada, igual à amostra e ao caminho comum
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
        em_combate: bool,
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
        s.write_u8(u8::from(em_combate)); // State — mesma posição do 1.5.5
        s.write_u8(level2);
        s.write_i32_le(hp);
        s.write_i32_le(max_hp);
        s.write_i32_le(mp);
        s.write_i32_le(max_mp);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    fn elf_exp(&self, _exp: i32) -> Option<S2CGamedataSend> {
        // O validador do cliente rejeita ids > 260 (VA 0x584618).
        None
    }

    // B101 — captura original da missão 1178 (`_sync/capturas/*.pcap`, subcomando 106):
    // `09 00 00 00 | 04 | 9a 04 | e7 0c 00 00 | 0a 00` — reason 4, task, monster, count;
    // **sem** os `dps`/`dph` do 1.5.3. Com os 17 bytes do padrão o cliente descartava o aviso
    // e o contador da missão não andava na tela, embora o servidor contasse.
    fn task_notify_monster_killed(&self, task_id: u16, monster_id: u32, monster_num: u16) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u8(4);
        s.write_u16_le(task_id);
        s.write_u32_le(monster_id);
        s.write_u16_le(monster_num);
        S2CGamedataSend::task_var_data(&s.into_bytes())
    }

    fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend {
        // 4 bytes: u16 exp, u16 sp
        let mut s = OctetsStream::new();
        s.write_u16_le(36);
        s.write_u16_le(estreitar_u16(exp));
        s.write_u16_le(estreitar_u16(sp));
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    // Original: docs/evidencias/126/s2c-31.txt:2; validador em cliente-validacao-itens.txt.
    fn pickup_item(&self, tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(31);
        s.write_i32_le(tid);
        s.write_i32_le(expire_date);
        s.write_u16_le(amount.min(u16::MAX as u32) as u16);
        s.write_u16_le(slot_amount.min(u16::MAX as u32) as u16);
        s.write_u8(package);
        s.write_u8(slot);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    // Original: docs/evidencias/126/s2c-99.txt:2; validador em cliente-validacao-itens.txt.
    fn obtain_item(&self, tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(99);
        s.write_i32_le(tid);
        s.write_i32_le(expire_date);
        s.write_u16_le(amount.min(u16::MAX as u32) as u16);
        s.write_u16_le(slot_amount.min(u16::MAX as u32) as u16);
        s.write_u8(package);
        s.write_u8(slot);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    // Original: docs/evidencias/126/s2c-156.txt:2; validador em cliente-validacao-itens.txt.
    fn task_deliver_item(&self, tid: i32, _expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(156);
        s.write_i32_le(tid);
        s.write_u16_le(amount.min(u16::MAX as u32) as u16);
        s.write_u16_le(slot_amount.min(u16::MAX as u32) as u16);
        s.write_u8(package);
        s.write_u8(slot);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    // S2C 46: contagem u16, payload 9 B (validador VA 0x584bc3); a ordem `where, index, count,
    // tid, type` é a em que o `Make<player_drop_item>::From` do `gs` 1.2.6 escreve
    // (VA 0x80906af-0x80906d3, conferido no B116).
    fn player_drop_item(&self, package: u8, slot: u8, count: u32, tid: i32, drop_type: u8) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(46);
        s.write_u8(package);
        s.write_u8(slot);
        s.write_u16_le(count.min(u16::MAX as u32) as u16);
        s.write_i32_le(tid);
        s.write_u8(drop_type);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    // S2C 72: sem yinpiao; cabeçalho 7 B + 13 B/item (VA 0x584abd).
    fn purchase_item(&self, cost: u32, itens: &[(i32, i32, u32, u16)]) -> S2CGamedataSend {
        let mut s = OctetsStream::new();
        s.write_u16_le(72);
        s.write_u32_le(cost);
        s.write_u8(0);
        s.write_u16_le(itens.len() as u16);
        for (tid, expira, n, slot) in itens {
            s.write_i32_le(*tid);
            s.write_i32_le(*expira);
            s.write_u16_le((*n).min(u16::MAX as u32) as u16);
            s.write_u16_le(*slot);
            s.write_u8(0);
        }
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

    fn equip_data_changed(&self, player_id: i32, crc: u16, mask_add: u64, mask_del: u64, items: &[i32]) -> S2CGamedataSend {
        // `Make<equipment_info_changed>::From` do `gs` 1.2.6 (VA 0x808fc06): cabeçalho 0x43,
        // `(u16) crc`, `int id`, `unsigned int mask_add`, `unsigned int mask_del` e os ids —
        // as máscaras têm 32 bits, como a do 66 (assinatura `gplayer*, jjPKvj`).
        let mask_add = mask_add as u32;
        let mut s = OctetsStream::new();
        s.write_u16_le(67);
        s.write_u16_le(crc);
        s.write_i32_le(player_id);
        s.write_u32_le(mask_add);
        s.write_u32_le(mask_del as u32);
        for item in items.iter().take(mask_add.count_ones() as usize) {
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
            s.write_i32_le(0); // addon_damage[i].low
            s.write_i32_le(0); // addon_damage[i].high
        }
        s.write_i32_le(magico.0); // damage_magic_low
        s.write_i32_le(magico.1); // damage_magic_high

        // ROLEEXTPROP_DEF (28B)
        for r in resistencias {
            s.write_i32_le(r);
        }
        s.write_i32_le(defense);
        s.write_i32_le(armor);

        s.write_i32_le(max_ap); // tem de espelhar SELF_INFO_00; fecha em 152 bytes

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
        modo_roupa: bool,
    ) -> S2CGamedataSend {
        // 34 bytes (sem state2). O `state` é `MakeObjectState<gplayer> | object_state`
        // (`Make<INFO::self_info_1>`, VA 0x808edf8), e o `gplayer_imp::SwitchFashionMode` do
        // `gs` 1.2.6 (VA 0x807bb12) liga e desliga o bit 13 (0x2000) desse `object_state` —
        // o mesmo `GP_STATE_FASHION` do 1.5.5 (B122).
        S2CGamedataSend::self_info_1(exp, sp, world_id, pos, sec_level, modo_roupa)
    }

    fn player_enter_world(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        self.info_player_1_126(17, role_id, vista)
    }

    fn player_enter_slice(&self, role_id: i32, vista: VistaDoJogador) -> S2CGamedataSend {
        self.info_player_1_126(12, role_id, vista)
    }
}

impl V126Protocol {
    fn info_npc(&self, comando: u16, nid: i32, tid: i32, pos: Vector3, dir: u8, estado: u32) -> S2CGamedataSend {
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
        // O `state`: 0x10000/0x20000 são "voa"/"nada" também no fonte 1.5.3
        // (`GP_STATE_NPC_FLY`, `EC_GPDataType.h:237`); o validador do caso 11/16 não soma bytes
        // por eles (B133).
        s.write_u32_le(estado);
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
        // `state`: o validador do cliente 1.2.6 (VA 0x584633, o caso de 12 e 17) parte de 26
        // bytes e soma por bit — `0x1` +1 (o `shape_form`), `0x2` +1, `0x40` +4, `0x400` → 34,
        // `0x800` +5, `0x1000` +1, `0x8` +1, `0x80000` +5, `0x100000` +5, `0x800000` +4, `0x10000`
        // variável. Por ora só a forma (B122): com o bit, quem entra no campo de visão de alguém na
        // raposa já o desenha raposa (o `PLAYER_CHGSHAPE` só alcança quem estava vendo).
        let mut state = if v.sec_level > 0 { 0x0000_4000 } else { 0 };
        if v.forma.is_some() {
            state |= 0x1;
        }
        // `MakeObjectState<gplayer>` (VA 0x8062d08) liga 0x80 no cadáver (`IsZombie`); o
        // `object_state` leva a roupa em 0x2000 (`SwitchFashionMode`, VA 0x807bb12). Nenhum dos
        // dois soma bytes no validador.
        if v.morto {
            state |= 0x80;
        }
        if v.modo_roupa {
            state |= 0x2000;
        }
        // Montado: 0x80000 com `{char mount_color; int mount_id}` (+5 no validador; o
        // `MakePlayerExtendState` do `gs` 1.2.6, VA 0x8062ebc-0x8062eed, escreve o `char` de
        // +0xca e o `int` de +0xd0 que o `ActiveMountState` grava). Quem entra no campo de visão
        // de alguém montado já o vê montado (B122).
        if v.montaria.is_some() {
            state |= 0x80000;
        }
        // Voo: `object_interface::TakeOff` → `gplayer_imp` faz `object_state |= 0x10`
        // (`gs` 1.2.6, VA 0x811d575); sem bytes no validador (B143).
        if v.voando {
            state |= 0x10;
        }
        // `MakeObjectState<gplayer>` liga 0x40 com o `extend_state` (um só `int`, +0x60) não zero
        // (VA 0x8062d31); o validador soma +4. É o mesmo DWORD do `UPDATE_EXT_STATE` do 1.2.6
        // (B124). O 1.2.6 não liga os bits de grupo no `object_state` (nenhum `or` com
        // 0x100/0x200 no `gs`), e os demais (`0x2` emote, `0x8` pária, `0x400` anúncio, `0x800`
        // facção, `0x1000` barraca, `0x10000` efeito, `0x100000` casal, `0x800000` cônjuge) são
        // de sistemas que o `pw-gs` não tem: vão zero (B143).
        let estendido = v.estados_visiveis[0];
        if estendido != 0 {
            state |= 0x40;
        }
        s.write_i32_le(state);
        // Na ordem do `MakePlayerExtendState` 1.2.6: forma (0x1) antes da montaria (0x80000).
        if let Some(forma) = v.forma {
            s.write_u8(self.byte_de_forma(forma));
        }
        if estendido != 0 {
            s.write_u32_le(estendido);
        }
        if let Some((cor, modelo)) = v.montaria {
            s.write_u8(cor as u8);
            s.write_i32_le(modelo);
        }
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

/// B102 — a lista de concluídas no formato do 1.2.6 (`FnshedTaskListOld`): cabeçalho
/// `m_uTaskCount u16, m_Version u8 = 0, reservado u8` e entradas `u16` com a falha no bit 15.
/// É o que o servidor 1.2.6 manda (captura: `01 00 00 00 e8 06`) e o que o cliente 1.5.3
/// converte (`CElementClient/Task/TaskProcess.cpp:2060-2072`: `id & 0x7fff`, `mask = id >> 15`).
/// O mundo guarda o formato novo (versão 1: `id u16, falhou:1, vezes u8`); a contagem de vezes
/// não existe no 1.2.6. Bloco que não é o formato novo segue como está.
pub fn concluidas_no_formato_antigo(b: &[u8]) -> Vec<u8> {
    if b.len() < 4 || b[2] != 1 {
        return b.to_vec();
    }
    let n = u16::from_le_bytes([b[0], b[1]]) as usize;
    if b.len() < 4 + 4 * n {
        return b.to_vec();
    }
    let mut o = Vec::with_capacity(4 + 2 * n);
    o.extend_from_slice(&(n as u16).to_le_bytes());
    o.extend_from_slice(&[0, 0]);
    for k in 0..n {
        let e = &b[4 + 4 * k..8 + 4 * k];
        let id = u16::from_le_bytes([e[0], e[1]]) & 0x7fff;
        let falhou = (e[2] & 1) as u16;
        o.extend_from_slice(&(id | falhou << 15).to_le_bytes());
    }
    o
}
