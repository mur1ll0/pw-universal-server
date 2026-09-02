//! Confere que o `OctetsStream` deste crate e o `pw_wire::gnet` produzem **os mesmos
//! bytes**.
//!
//! Hoje existem duas implementações do formato GNET no projeto: a antiga, aqui em
//! `octets.rs`, e a do `pw-wire`, que é a que fica. Enquanto as duas coexistirem, este
//! teste é o que garante que elas não divergem — e é o que torna a migração de
//! `octets.rs` para o `pw-wire` uma troca segura em vez de um salto no escuro.
//!
//! O `CompactUINT` é o ponto sensível: são quatro formas escolhidas por faixa de valor,
//! e um `<` no lugar de um `<=` só aparece em seis valores do universo inteiro.

use pw_protocol::octets::OctetsStream;
use pw_wire::gnet;

/// Valores que exercitam as quatro formas do `CompactUINT` e as fronteiras entre elas.
const FRONTEIRAS: &[u32] = &[
    0,
    1,
    0x7F,
    0x80,
    0x3FFF,
    0x4000,
    0x1FFF_FFFF,
    0x2000_0000,
    u32::MAX,
];

#[test]
fn compact_uint_produz_os_mesmos_bytes_nas_duas_implementacoes() {
    for &valor in FRONTEIRAS {
        let mut antigo = OctetsStream::new();
        antigo.write_compact_uint(valor);

        let mut novo = gnet::Writer::new();
        novo.compact_uint(valor);

        assert_eq!(
            antigo.as_slice(),
            novo.as_slice(),
            "CompactUINT({valor:#x}) diverge entre octets.rs e pw_wire::gnet"
        );
    }
}

#[test]
fn cada_implementacao_le_o_que_a_outra_escreveu() {
    // Ida e volta cruzada: o que importa não é cada uma ser consistente consigo mesma,
    // e sim as duas falarem a mesma língua enquanto convivem.
    for &valor in FRONTEIRAS {
        let mut novo = gnet::Writer::new();
        novo.compact_uint(valor);
        let bytes = novo.into_vec();

        let lido_pelo_antigo = OctetsStream::from_bytes(&bytes)
            .read_compact_uint()
            .expect("octets.rs não leu o que o pw-wire escreveu");
        assert_eq!(lido_pelo_antigo, valor);

        let mut antigo = OctetsStream::new();
        antigo.write_compact_uint(valor);
        let bytes = antigo.as_slice().to_vec();

        let lido_pelo_novo = gnet::Reader::new(&bytes)
            .compact_uint()
            .expect("pw-wire não leu o que o octets.rs escreveu");
        assert_eq!(lido_pelo_novo, valor);
    }
}

#[test]
fn escalares_e_octets_produzem_os_mesmos_bytes() {
    let mut antigo = OctetsStream::new();
    antigo.write_u8(0x12);
    antigo.write_i16(-2);
    antigo.write_u32(0x1122_3344);
    antigo.write_i64(-1);
    antigo.write_f32(1.5);
    antigo.write_octets(b"abc");
    antigo.write_octets(b"");

    let mut novo = gnet::Writer::new();
    novo.u8(0x12);
    novo.i16(-2);
    novo.u32(0x1122_3344);
    novo.i64(-1);
    novo.f32(1.5);
    novo.octets(b"abc");
    novo.octets(b"");

    assert_eq!(antigo.as_slice(), novo.as_slice());
}
