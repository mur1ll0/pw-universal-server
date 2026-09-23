//! Gabaritos full_interno.pcap, extraídos em docs/evidencias/126/s2c-{id}.txt.
use pw_protocol::{versions::create_world_protocol, GameVersion, S2CGamedataSend};

#[test]
fn habilidade_recebida_126_reproduz_os_15_bytes_do_original() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    // Amostra única S2C 144: atacante 34, habilidade 250, dano 2404, sem desgaste.
    assert_eq!(p.host_skill_attacked(34, 250, 2404, 0, 9, 0).data,
        [144, 0, 34, 0, 0, 0, 250, 0, 0, 0, 0x64, 9, 0, 0, 0x7f, 0, 9]);
}

#[test]
fn habilidade_recebida_155_preserva_flag_i32_e_section() {
    let p = create_world_protocol(GameVersion::V1_5_5);
    assert_eq!(p.host_skill_attacked(34, 250, 2404, 0x12345678, 9, 3).data,
        [144, 0, 34, 0, 0, 0, 250, 0, 0, 0, 0x64, 9, 0, 0, 0x7f,
         0x78, 0x56, 0x34, 0x12, 9, 3]);
}

#[test]
fn golpe_normal_126_reproduz_os_pacotes_originais() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    let alvo = 0x80103503u32 as i32;
    assert_eq!(S2CGamedataSend::host_start_attack(alvo, 0, 22).data,
        [84, 0, 3, 0x35, 0x10, 0x80, 0, 0, 22]);
    assert_eq!(S2CGamedataSend::attack_once(0).data, [83, 0, 0]);
    assert_eq!(p.host_attack_result(alvo, 7, 0, 16).data,
        [24, 0, 3, 0x35, 0x10, 0x80, 7, 0, 0, 0, 0, 16]);
    assert_eq!(p.host_attacked(alvo, 1, 0x7f, 0, 27).data,
        [26, 0, 3, 0x35, 0x10, 0x80, 1, 0, 0, 0, 0x7f, 0, 27]);
    assert_eq!(p.npc_info_00(alvo, 29, 29, 0).data,
        [33, 0, 3, 0x35, 0x10, 0x80, 29, 0, 0, 0, 29, 0, 0, 0]);
}
