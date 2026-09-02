//! O handshake do 1.2.6, contra os bytes que um servidor 1.2.6 de verdade trocou.
//!
//! # De onde vêm estes bytes
//!
//! De `capturas/t2_externo.pcap`: o elo **cliente ↔ glinkd** (porta 29000) de um servidor
//! 1.2.6 em funcionamento, na parte que ainda está em claro — do `Challenge` até a troca
//! de chaves, que é onde a cifra começa e a leitura acaba.
//!
//! A sequência observada, com os tamanhos de payload:
//!
//! ```text
//! servidor → cliente:  1(22)   3(18)   [daqui em diante, cifrado]
//! cliente  → servidor:         2(23)   [idem]
//! ```
//!
//! # A descoberta
//!
//! O opcode **2 do 1.2.6 é o `Response`** — o pacote de login — e não o `KeyExchange`. Os
//! bytes não deixam dúvida: o payload do 2 traz `Octets("teste")`, o nome de usuário que o
//! jogador digitou, seguido de `Octets` de 16 bytes, que é o resumo da senha. Uma troca de
//! chaves não carrega o nome do usuário em claro.
//!
//! E o **3 é o `KeyExchange`**: `Octets(16) + i8(0)`, exatamente os campos
//! `{nonce, blkickuser}` que o IR do 1.5.3 dá para o `KeyExchange` — só que com o número
//! trocado.
//!
//! No 1.5.3 é o contrário (2 = `KeyExchange`, 3 = `Response`), e é isso que o IR diz.
//!
//! # O que isso causava
//!
//! Com a numeração do 1.5.3 valendo para todos os realms, o `Response` do cliente 1.2.6
//! caía no ramo do `KeyExchange`, que só escreve uma linha de log e não responde nada. O
//! login **nunca acontecia**: o cliente ficava em "Conectando ao jogo" até a conexão morrer
//! por inatividade, sem mensagem de erro em nenhum dos dois lados.

use bytes::BytesMut;
use pw_protocol::{
    create_protocol_adapter, Edition, GameVersion, InboundPacket, OctetsStream, OutboundPacket,
    PwPacketCodec, S2CChallenge,
};
use tokio_util::codec::{Decoder, Encoder};

/// O quadro GNET inteiro do `Challenge` que o servidor 1.2.6 real mandou (22 bytes de
/// payload), sem o cabeçalho de opcode/tamanho.
const CHALLENGE_REAL: &[u8] = &[
    0x10, // Octets de 16 bytes
    0x00, 0x00, 0x00, 0xd0, 0x00, 0x00, 0x00, 0x00, 0x1b, 0x01, 0xaf, 0xe0, 0x6a, 0xda, 0x46, 0x4c,
    0x00, 0x01, 0x02, 0x06, // version = 0x00010206
    0x00, // algo = 0
];

/// O payload do opcode 2 que o cliente 1.2.6 real mandou: `Octets("teste") + Octets(16)`.
const RESPONSE_REAL: &[u8] = &[
    0x05, b't', b'e', b's', b't', b'e', // Octets("teste")
    0x10, // Octets de 16 bytes
    0x75, 0xca, 0x09, 0x65, 0x35, 0x96, 0x76, 0x97, 0xbf, 0xa0, 0x72, 0x0a, 0x16, 0xfd, 0x65, 0xa5,
];

/// O payload do opcode 3 que o servidor real mandou: `Octets(16) + i8(0)`.
const KEYEXCHANGE_REAL: &[u8] = &[
    0x10, // Octets de 16 bytes
    0x1c, 0x49, 0x52, 0x52, 0x52, 0x7f, 0x4a, 0xea, 0x67, 0xc0, 0xaa, 0x5c, 0xc0, 0x9c, 0x9b, 0x90,
    0x00, // blkickuser = 0
];

/// Monta um quadro GNET: `CompactUINT opcode`, `CompactUINT tamanho`, payload.
///
/// Só cobre valores pequenos, que é o que estes payloads exigem.
fn quadro(opcode: u8, payload: &[u8]) -> BytesMut {
    let mut v = BytesMut::new();
    assert!(opcode < 0x40 && payload.len() < 0x40, "use a forma longa do CompactUINT");
    v.extend_from_slice(&[opcode, payload.len() as u8]);
    v.extend_from_slice(payload);
    v
}

fn codec(v: GameVersion) -> PwPacketCodec {
    PwPacketCodec::from_adapter(create_protocol_adapter(v))
}

#[test]
fn o_opcode_2_do_126_e_o_login_e_nao_a_troca_de_chaves() {
    let mut c = codec(GameVersion::V1_2_6);
    let mut buf = quadro(2, RESPONSE_REAL);

    match c.decode(&mut buf).unwrap() {
        Some(InboundPacket::Response(r)) => {
            assert_eq!(r.username, "teste", "o nome de usuário do pacote real");
            assert_eq!(r.password_response.len(), 16, "o resumo da senha tem 16 bytes");
            assert!(!r.use_token, "o Response do 1.2.6 não tem `use_token`");
            assert!(r.cli_fingerprint.is_empty(), "nem `cli_fingerprint`");
        }
        outro => panic!(
            "o opcode 2 de um realm 1.2.6 tem que virar Response; virou {outro:?} — é este o \
             bug que deixava o cliente preso em \"Conectando ao jogo\""
        ),
    }
}

#[test]
fn o_opcode_3_do_126_e_a_troca_de_chaves() {
    let mut c = codec(GameVersion::V1_2_6);
    let mut buf = quadro(3, KEYEXCHANGE_REAL);

    match c.decode(&mut buf).unwrap() {
        Some(InboundPacket::KeyExchange(k)) => {
            assert_eq!(k.nonce.len(), 16);
            assert_eq!(k.blkickuser, 0);
        }
        outro => panic!("o opcode 3 de um realm 1.2.6 tem que virar KeyExchange; virou {outro:?}"),
    }
}

#[test]
fn no_153_os_dois_numeros_sao_os_do_ir_e_nao_os_do_126() {
    // A prova de que a ramificação é por versão e não uma troca global: os **mesmos** dois
    // números, num realm 1.5.3, significam o contrário.
    let mut c = codec(GameVersion::V1_5_3);

    let mut buf = quadro(2, KEYEXCHANGE_REAL);
    assert!(
        matches!(c.decode(&mut buf).unwrap(), Some(InboundPacket::KeyExchange(_))),
        "no 1.5.3 o opcode 2 é o KeyExchange (IR)"
    );

    let mut buf = quadro(3, RESPONSE_REAL);
    assert!(
        matches!(c.decode(&mut buf).unwrap(), Some(InboundPacket::Response(_))),
        "no 1.5.3 o opcode 3 é o Response (IR)"
    );
}

#[test]
fn o_key_exchange_que_mandamos_sai_no_numero_da_versao() {
    use pw_protocol::S2CKeyExchange;

    for (v, esperado) in [(GameVersion::V1_2_6, 3u8), (GameVersion::V1_5_3, 2)] {
        let mut c = codec(v);
        let mut saida = BytesMut::new();
        c.encode(
            OutboundPacket::KeyExchange(S2CKeyExchange {
                nonce: vec![0u8; 16],
                blkickuser: 0,
            }),
            &mut saida,
        )
        .unwrap();
        assert_eq!(
            saida[0], esperado,
            "o KeyExchange de um realm {v} tem que sair no opcode {esperado}"
        );
    }
}

#[test]
fn o_challenge_que_mandamos_bate_byte_a_byte_com_o_do_servidor_real() {
    // O `Challenge` é o primeiro pacote do jogo e o único do handshake que nós **enviamos**
    // em claro. Comparar com o de um servidor de verdade, byte a byte, é a única forma de
    // saber que ele está certo — o cliente, quando não está, apenas fecha a conexão.
    let mut c = codec(GameVersion::V1_2_6);
    let nonce = CHALLENGE_REAL[1..17].to_vec();

    let mut saida = BytesMut::new();
    c.encode(
        OutboundPacket::Challenge(S2CChallenge::new(
            nonce,
            GameVersion::V1_2_6,
            // Os quatro valores do `edition` não vão para o fio no 1.2.6; qualquer um serve.
            Edition::new(GameVersion::V1_2_6, 0x571d_b3f4, 0x5698_6c25, None),
        )),
        &mut saida,
    )
    .unwrap();

    let mut s = OctetsStream::from_bytes(&saida);
    assert_eq!(s.read_compact_uint().unwrap(), 1, "opcode do Challenge");
    assert_eq!(
        s.read_compact_uint().unwrap() as usize,
        CHALLENGE_REAL.len(),
        "o payload tem que ter os mesmos 22 bytes do servidor real"
    );
    assert_eq!(
        &saida[2..],
        CHALLENGE_REAL,
        "o Challenge saiu diferente do que o servidor 1.2.6 real mandou"
    );
}
