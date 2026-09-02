use pw_core::{CharacterClass, CharacterSummary, Gender, ItemRecord, Race, Vector3};
use pw_protocol::{
    create_protocol_adapter, Edition, GameVersion, InboundPacket, OctetsStream, OutboundPacket,
    PorVersao, PwPacketCodec, S2CChallenge, S2CGamedataSend, S2CRoleListResponse,
    S2CSelectRoleResponse,
};
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

#[test]
fn test_challenge_and_response_126() {
    let adapter_126 = create_protocol_adapter(GameVersion::V1_2_6);
    let mut codec_126 = PwPacketCodec::from_adapter(adapter_126);

    let nonce = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    // Construído com a versão do realm, e não com uma fixa: o código de versão agora
    // sai do pacote, e não mais do adapter. Este teste montava um Challenge de 1.5.3 e
    // o mandava por um codec de 1.2.6 — passava porque o adapter ignorava o campo.
    let challenge = OutboundPacket::Challenge(S2CChallenge::new(
        nonce.clone(),
        GameVersion::V1_2_6,
        Edition::new(GameVersion::V1_2_6, 0x4A2B_1C3D, 0x4A2B_1C40, None),
    ));

    let mut encoded = BytesMut::new();
    codec_126.encode(challenge, &mut encoded).expect("Falha ao encodificar Challenge 1.2.6");

    // Decodifica o frame e valida opcode 1
    let mut stream = OctetsStream::from_bytes(&encoded);
    let opcode = stream.read_compact_uint().unwrap();
    let length = stream.read_compact_uint().unwrap();
    assert_eq!(opcode, 1);
    assert!(length > 0);

    let read_nonce = stream.read_octets().unwrap();
    assert_eq!(read_nonce, nonce);
    let version = stream.read_u32().unwrap();
    assert_eq!(version, 0x00010206);
    let algo = stream.read_i8().unwrap();
    assert_eq!(algo, 0);
    // 1.2.6 não deve ter campos edition e exp_rate
    assert_eq!(stream.len(), 0);
}

#[test]
fn test_challenge_153_has_edition_and_exp_rate() {
    let adapter_153 = create_protocol_adapter(GameVersion::V1_5_3);
    let mut codec_153 = PwPacketCodec::from_adapter(adapter_153);

    let nonce = vec![0xAA; 16];
    let challenge = OutboundPacket::Challenge(S2CChallenge::new(
        nonce.clone(),
        GameVersion::V1_5_3,
        Edition::new(GameVersion::V1_5_3, 0x4A2B_1C3D, 0x4A2B_1C40, None),
    ));

    let mut encoded = BytesMut::new();
    codec_153.encode(challenge, &mut encoded).expect("Falha ao encodificar Challenge 1.5.3");

    let mut stream = OctetsStream::from_bytes(&encoded);
    let opcode = stream.read_compact_uint().unwrap();
    let _length = stream.read_compact_uint().unwrap();
    assert_eq!(opcode, 1);

    let read_nonce = stream.read_octets().unwrap();
    assert_eq!(read_nonce, nonce);
    let version = stream.read_u32().unwrap();
    // `0x00010502`, e não `0x00010503`: o valor vem de `EC_Game.cpp:115` dos fontes do
    // cliente (`GAME_VERSION = (0<<24)|(1<<16)|(5<<8)|2`). Este teste afirmava 0x...503,
    // deduzido do nome "1.5.3" — e travava o bug em vez de pegá-lo. O cliente compara
    // este campo e derruba a conexão antes de olhar a senha.
    assert_eq!(version, 0x0001_0502);
    let _algo = stream.read_i8().unwrap();
    let edition = stream.read_octets().unwrap();
    // Era `assert!(edition.is_empty())` — e o `edition` vazio é justamente a segunda
    // causa de o cliente recusar o login. Agora vai a string do `%x%x%x%x`.
    // `30000091` + `7c` (124) são as constantes medidas no cliente 1.5.3 do realm (build
    // 2552, via `EC.log`), seguidas dos dois timestamps de gshop passados acima.
    assert_eq!(edition, b"300000917c4a2b1c3d4a2b1c40");
    let exp_rate = stream.read_u8().unwrap();
    assert_eq!(exp_rate, 1);
}

#[test]
fn test_role_list_multi_realm_encoding() {
    let summary = CharacterSummary {
        id: 1024,
        account_id: 1,
        realm_id: "realm_126".to_string(),
        name: "GuerreiroPW".to_string(),
        race: Race::Human,
        cls: CharacterClass::Blademaster,
        gender: Gender::Male,
        level: 80,
        cultivation: 20,
        world_id: 1,
        position: Vector3::new(438.0, 21.0, 676.0),
        equipment: vec![
            ItemRecord {
                id: None,
                character_id: 1024,
                container_type: pw_core::ContainerType::Equipment,
                slot: 0,
                item_id: 2097,
                count: 1,
                max_count: 1,
                refine_level: 5,
                sockets_count: 2,
                sockets: vec![1234, 1234],
                durability: 5000,
                max_durability: 5000,
                bind_status: 0,
                octets: Vec::new(),
                custom_attributes: serde_json::json!({}),
            }
        ],
        custom_appearance: serde_json::json!({ "raw": "01020304" }),
        is_deleted: false,
        delete_time: None,
    };

    // 1. Testa Realm 1.2.6 (19 campos por RoleInfo)
    let adapter_126 = create_protocol_adapter(GameVersion::V1_2_6);
    let mut codec_126 = PwPacketCodec::from_adapter(adapter_126);
    let role_list_126 = OutboundPacket::RoleListResponse(S2CRoleListResponse::new(
        1, 100, vec![summary.clone()]
    ));
    let mut buf_126 = BytesMut::new();
    codec_126.encode(role_list_126, &mut buf_126).expect("Encode 1.2.6 RoleList falhou");
    assert!(!buf_126.is_empty());

    // 2. Testa Realm 1.5.3 (23 campos por RoleInfo)
    let adapter_153 = create_protocol_adapter(GameVersion::V1_5_3);
    let mut codec_153 = PwPacketCodec::from_adapter(adapter_153);
    let role_list_153 = OutboundPacket::RoleListResponse(S2CRoleListResponse::new(
        1, 100, vec![summary]
    ));
    let mut buf_153 = BytesMut::new();
    codec_153.encode(role_list_153, &mut buf_153).expect("Encode 1.5.3 RoleList falhou");
    assert!(!buf_153.is_empty());

    // O buffer da 1.5.3 deve ser maior devido aos campos adicionais de reencarnação e realm
    assert!(buf_153.len() > buf_126.len());
}

#[test]
fn test_select_role_response_codec() {
    let adapter_126 = create_protocol_adapter(GameVersion::V1_2_6);
    let mut codec = PwPacketCodec::from_adapter(adapter_126);

    let auth_token = vec![10, 20, 30, 40];
    let packet = OutboundPacket::SelectRoleResponse(S2CSelectRoleResponse {
        result: 0,
        auth: auth_token.clone(),
    });

    let mut buf = BytesMut::new();
    codec.encode(packet, &mut buf).expect("Falha ao encodificar SelectRoleResponse");

    let mut stream = OctetsStream::from_bytes(&buf);
    let opcode = stream.read_compact_uint().unwrap();
    let length = stream.read_compact_uint().unwrap();
    assert_eq!(opcode, 0x47); // OP_S2C_SELECT_ROLE_RE = 71
    assert_eq!(length as usize, 4 + 1 + 4); // result(4) + compact_uint(1) + auth(4)

    let result = stream.read_i32().unwrap();
    assert_eq!(result, 0);
    let read_auth = stream.read_octets().unwrap();
    assert_eq!(read_auth, auth_token);
}

#[test]
fn test_gamedatasend_s2c_subcommands() {
    // 1. SELF_INFO_00 (CMD 38)
    let p1 = S2CGamedataSend::self_info_00(10, 32, 500, 500, 300, 300, 1000, 500);
    assert!(!p1.data.is_empty());
    assert_eq!(u16::from_le_bytes([p1.data[0], p1.data[1]]), 38);

    // 2. SELF_INFO_1 (CMD 8) com GM flag
    let p2 = S2CGamedataSend::self_info_1(1000, 500, 1024, Vector3::new(10.0, 20.0, 30.0), 32);
    assert_eq!(u16::from_le_bytes([p2.data[0], p2.data[1]]), 8);

    // 3. NPC_ENTER_SLICE (CMD 11)
    let p3 = S2CGamedataSend::npc_enter_slice(20001, 2191, Vector3::new(100.0, 200.0, 300.0), 64);
    assert_eq!(u16::from_le_bytes([p3.data[0], p3.data[1]]), 11);

    // 4. TASK_NOTIFY_NEW (CMD 106 / Reason 1)
    let p4 = S2CGamedataSend::task_notify_new(1, 1600000000);
    assert_eq!(u16::from_le_bytes([p4.data[0], p4.data[1]]), 106);

    // 5. TASK_NOTIFY_MONSTER_KILLED (CMD 106 / Reason 4)
    let p5 = S2CGamedataSend::task_notify_monster_killed(1, 13641, 5);
    assert_eq!(u16::from_le_bytes([p5.data[0], p5.data[1]]), 106);

    // 6. SERVER_CONFIG_DATA / INST_DATA_CHECKOUT (CMD 206)
    let p6 = S2CGamedataSend::inst_data_checkout(1, 1156141381, 1156141381, 1206433535);
    assert_eq!(u16::from_le_bytes([p6.data[0], p6.data[1]]), 206);

    // 7. MALL_ITEM_PRICE (CMD 270)
    //
    // Este teste afirmava 197 e, com isso, **prendia o bug**: 197 é `REVIVAL_INQUIRE`.
    // Um teste escrito a partir do código só confirma o que o código faz; quem decide é
    // o IR, e é o que `subcomandos_s2c_contra_o_ir.rs` passou a cobrar.
    let p_mall = S2CGamedataSend::mall_item_price();
    assert_eq!(u16::from_le_bytes([p_mall.data[0], p_mall.data[1]]), 270);

    // 8. SKILL_PERFORM (CMD 88) e SELF_SKILL_ATTACK_RESULT (CMD 142)
    let p7 = S2CGamedataSend::skill_perform();
    assert_eq!(u16::from_le_bytes([p7.data[0], p7.data[1]]), 88);

    let p8 = S2CGamedataSend::self_skill_attack_result(100, 1, 150, 0, 0, 0);
    assert_eq!(u16::from_le_bytes([p8.data[0], p8.data[1]]), 142);

    let p9 = S2CGamedataSend::self_stop_skill();
    assert_eq!(u16::from_le_bytes([p9.data[0], p9.data[1]]), 123);

    // Valida normalização de durabilidade para armas (ex: 28 -> 1400 = 28*50)
    let item_weapon = S2CGamedataSend::item_info(1, 0, 2097, 28, 28, 1, &[]);
    assert_eq!(u16::from_le_bytes([item_weapon.data[0], item_weapon.data[1]]), 40);
    let cur_dur = i32::from_le_bytes([
        item_weapon.data[36], item_weapon.data[37], item_weapon.data[38], item_weapon.data[39]
    ]);
    let max_dur = i32::from_le_bytes([
        item_weapon.data[40], item_weapon.data[41], item_weapon.data[42], item_weapon.data[43]
    ]);
    assert_eq!(cur_dur, 1400);
    assert_eq!(max_dur, 1400);
}

#[test]
fn test_inbound_c2s_packet_decoding() {
    let mut codec = PwPacketCodec::new("1.2.6");

    // 1. Cria payload simulado de C2SRoleList (Opcode 0x52 / 82)
    let mut payload = OctetsStream::new();
    payload.write_i32(1001); // userid
    payload.write_u32(55);   // localsid
    payload.write_i32(-1);   // handle

    let mut frame = OctetsStream::new();
    frame.write_compact_uint(0x52);
    frame.write_compact_uint(payload.len() as u32);
    frame.write_raw_bytes(payload.as_slice());

    let mut buf = BytesMut::from(frame.as_slice());
    let packet_opt = codec.decode(&mut buf).expect("Falha ao decodificar C2SRoleList");

    match packet_opt {
        Some(InboundPacket::RoleList(req)) => {
            assert_eq!(req.userid, 1001);
            assert_eq!(req.localsid, 55);
            assert_eq!(req.handle, -1);
        }
        other => panic!("Esperava InboundPacket::RoleList, obteve: {:?}", other),
    }
}

#[test]
fn test_inst_data_checkout_155_ganha_o_sexto_campo_gshop3() {
    // Achado comparando o IR do 1.5.5 com o do 1.5.3 (2026-09-02): o 1.5.3 já tem cinco
    // campos (`idInst`, `region`, `precinct`, `gshop`, `gshop2` — ver `layouts_do_126.rs`
    // para a diferença medida contra o 1.2.6, que só tem quatro); o 1.5.5 acrescenta um
    // sexto, `gshop_time_stamp3`.
    let sub_155 = PorVersao::new(GameVersion::V1_5_5);
    let com_terceiro = sub_155.inst_data_checkout(1, 10, 20, 30, Some(40));
    // 2 (cabeçalho) + 4 (idInst) + 4 + 4 + 4 (gshop) + 4 (gshop2, igual ao gshop — questão
    // em aberto, não desta mudança) + 4 (gshop3) = 26 bytes.
    assert_eq!(com_terceiro.data.len(), 26);
    let gshop3_no_fio = u32::from_le_bytes(com_terceiro.data[22..26].try_into().unwrap());
    assert_eq!(gshop3_no_fio, 40);

    // Sem `Some`, o 1.5.5 continua no layout de cinco campos — ninguém é forçado a
    // fornecer um terceiro timestamp que não tem.
    let sem_terceiro = sub_155.inst_data_checkout(1, 10, 20, 30, None);
    assert_eq!(sem_terceiro.data.len(), 22);

    // E o 1.5.3 ignora `Some` — o sexto campo é só para quem mediu precisar dele.
    let sub_153 = PorVersao::new(GameVersion::V1_5_3);
    let v153_com_some = sub_153.inst_data_checkout(1, 10, 20, 30, Some(40));
    assert_eq!(v153_com_some.data.len(), 22, "1.5.3 não ganha o sexto campo só por receber Some");
}
