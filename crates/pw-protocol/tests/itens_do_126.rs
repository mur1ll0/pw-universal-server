//! Amostras do original: docs/evidencias/126/s2c-{id}.txt, primeira amostra.
use pw_protocol::{versions::create_world_protocol, GameVersion, S2CGamedataSend};

#[test]
fn pickup_126() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(p.pickup_item(825, 0, 1, 1, 0, 4).data,
        [31, 0, 0x39, 3, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 4]);
}
#[test]
fn obtain_126() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(p.obtain_item(11208, 0, 8, 58, 0, 10).data,
        [99, 0, 0xc8, 0x2b, 0, 0, 0, 0, 0, 0, 8, 0, 58, 0, 0, 10]);
}
#[test]
fn premio_item_126() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(p.task_deliver_item(12498, 0, 1, 1, 0, 4).data,
        [156, 0, 0xd2, 0x30, 0, 0, 1, 0, 1, 0, 0, 4]);
}
#[test]
fn retirada_126() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(p.player_drop_item(0, 4, 1, 825, 1).data,
        [46, 0, 0, 4, 1, 0, 0x39, 3, 0, 0, 1]);
}
#[test]
fn compra_126() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(p.purchase_item(24000, &[(1947, 0, 1000, 21)]).data,
        [72, 0, 0xc0, 0x5d, 0, 0, 0, 1, 0, 0x9b, 7, 0, 0, 0, 0, 0, 0, 0xe8, 3, 21, 0, 0]);
}
#[test]
fn experiencia_126_reproduz_36_e_158() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(p.receive_exp(15, 36).data, [36, 0, 15, 0, 36, 0]);
    assert_eq!(S2CGamedataSend::task_deliver_exp(80, 0).data,
        [158, 0, 80, 0, 0, 0, 0, 0, 0, 0]);
}
#[test]
fn itens_155_preservam_layout_e_contadores_u32() {
    let p = create_world_protocol(GameVersion::V1_5_5);
    // Validade e quantidades acima de u16 não podem sumir/estreitar no padrão.
    for (id, actual) in [(31, p.pickup_item(825, 123, 70000, 80000, 2, 4)),
        (99, p.obtain_item(825, 123, 70000, 80000, 2, 4)),
        (156, p.task_deliver_item(825, 123, 70000, 80000, 2, 4))] {
        assert_eq!(actual.data, [id, 0, 0x39, 3, 0, 0, 123, 0, 0, 0,
            0x70, 0x11, 1, 0, 0x80, 0x38, 1, 0, 2, 4]);
    }
    assert_eq!(p.player_drop_item(2, 4, 70000, 825, 3).data,
        [46, 0, 2, 4, 0x70, 0x11, 1, 0, 0x39, 3, 0, 0, 3]);
    assert_eq!(p.purchase_item(24000, &[(1947, 123, 70000, 21)]).data,
        [72, 0, 0xc0, 0x5d, 0, 0, 0, 0, 0, 0, 0, 1, 0,
         0x9b, 7, 0, 0, 123, 0, 0, 0, 0x70, 0x11, 1, 0, 21, 0, 0]);
    assert_eq!(p.receive_exp(70000, 80000).data,
        [36, 0, 0x70, 0x11, 1, 0, 0x80, 0x38, 1, 0]);
}

#[test]
fn daimon_omite_283_no_126_e_preserva_os_bytes_155() {
    // cliente-validacao-entrada.txt: switch em 0x584618 limita ids a 260.
    assert!(create_world_protocol(GameVersion::V1_2_6).elf_exp(70000).is_none());
    assert_eq!(create_world_protocol(GameVersion::V1_5_5).elf_exp(70000).unwrap().data,
        [0x1b, 1, 0x70, 0x11, 1, 0]);
}
