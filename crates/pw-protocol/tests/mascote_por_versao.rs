//! Os comandos de mascote, por versão, contra o que o cliente de cada uma aceita.
//!
//! 1.2.6: a tabela do validador de tamanho do `elementclient.exe` 1.2.6 (VA 0x584610, saltos
//! em 0x584e90; SHA256 em `docs/evidencias/126/cliente-validacao-entrada.txt`), que a captura
//! `full_interno.pcap` confirma nos que aparecem nela (233 = 12, 234 = 8). 1.5.5: as structs
//! de `Network/EC_GPDataType.h` do cliente 1.5.5. O cliente descarta em silêncio o que não bate.

use pw_core::Vector3;
use pw_protocol::{versions::create_world_protocol, GameVersion, WorldProtocol};

fn corpo(p: &pw_protocol::packets::s2c::S2CGamedataSend) -> (u16, usize) {
    (u16::from_le_bytes([p.data[0], p.data[1]]), p.data.len() - 2)
}

fn medir(w: &dyn WorldProtocol) -> Vec<(u16, usize)> {
    vec![
        corpo(&w.summon_pet(0, 10386, -2147000000, 0)),
        corpo(&w.recall_pet(0, 10386, 0)),
        corpo(&w.pet_hp_notify(0, 1.0, 100, 0.0, 0)),
        corpo(&w.pet_ai_state(0, 0)),
        corpo(&w.pet_dead(0)),
        corpo(&w.pet_revive(0, 0.1)),
        corpo(&w.pet_receive_exp(0, 10386, 5)),
        corpo(&w.pet_levelup(0, 10386, 3, 0)),
        corpo(&w.pet_honor_point(0, 50)),
        corpo(&w.pet_hunger_gauge(0, 1)),
        corpo(&w.object_attack_result(-5, -6, 10, 0, 20)),
    ]
}

#[test]
fn os_comandos_de_mascote_do_126_tem_o_tamanho_do_validador() {
    let w = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(
        medir(w.as_ref()),
        vec![(233, 12), (234, 8), (249, 12), (250, 2), (247, 4), (248, 8), (237, 12), (238, 16), (241, 8), (242, 8), (120, 14)]
    );
    // `info_npc` 27 B + dono (bit 0x1000) e + 1 + tamanho com nome (bit 0x2000), VA 0x58486a.
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let sem_nome = w.mascote_entra(16, -5, 10386, 10386, pos, 7, 11455, &[]);
    assert_eq!(corpo(&sem_nome), (16, 27 + 4));
    assert_eq!(i32::from_le_bytes(sem_nome.data[2 + 23..2 + 27].try_into().unwrap()), 0x1000);
    assert_eq!(i32::from_le_bytes(sem_nome.data[2 + 27..2 + 31].try_into().unwrap()), 11455);
    let com_nome = w.mascote_entra(11, -5, 10386, 10386, pos, 7, 11455, b"Lobo");
    assert_eq!(corpo(&com_nome), (11, 27 + 4 + 1 + 4));
}

#[test]
fn os_comandos_de_mascote_do_155_tem_o_tamanho_das_structs() {
    let w = create_world_protocol(GameVersion::V1_5_5);
    assert_eq!(
        medir(w.as_ref()),
        vec![(233, 16), (234, 9), (249, 20), (250, 2), (247, 4), (248, 8), (237, 12), (238, 16), (241, 8), (242, 8), (120, 17)]
    );
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let p = w.mascote_entra(16, -5, 10386, 10386, pos, 7, 11455, &[]);
    assert_eq!(corpo(&p), (16, 35 + 4));
    assert_eq!(i32::from_le_bytes(p.data[2 + 27..2 + 31].try_into().unwrap()), 0x1000);
}
