use crate::octets::OctetsStream;
use crate::packets::s2c::S2CGamedataSend;

/// Um comando sem payload — só o cabeçalho de 2 bytes.
pub fn so_o_cabecalho(id: u16) -> S2CGamedataSend {
    let mut s = OctetsStream::new();
    s.write_u16_le(id);
    S2CGamedataSend { data: s.into_bytes().to_vec() }
}

/// Encolhe um `attack_flag` de 32 bits para os 8 do 1.2.6, **saturando**.
pub fn estreitar(v: i32) -> i8 {
    v.clamp(i8::MIN as i32, i8::MAX as i32) as i8
}

/// Encolhe exp/sp de 32 para 16 bits, com teto em vez de estouro.
pub fn estreitar_u16(v: i32) -> u16 {
    v.clamp(0, u16::MAX as i32) as u16
}
