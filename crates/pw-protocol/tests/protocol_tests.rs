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
        last_login_at: None,
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
    // 2 (cabeçalho) + 4 (tamanho) + 3 (base) + 8 (tempo, capitão) + 3 (sub_tags vazio).
    let p4 = S2CGamedataSend::task_notify_new(1, 1600000000, 0, &[0, 0, 0]);
    assert_eq!(u16::from_le_bytes([p4.data[0], p4.data[1]]), 106);
    assert_eq!(p4.data.len(), 2 + 4 + 3 + 8 + 3);

    // 5. TASK_NOTIFY_MONSTER_KILLED (CMD 106 / Reason 4)
    let p5 = S2CGamedataSend::task_notify_monster_killed(1, 13641, 5, 0, 0);
    assert_eq!(u16::from_le_bytes([p5.data[0], p5.data[1]]), 106);
    // sizeof(svr_monster_killed) com pack(1) = 3 + 4 + 2 + 4 + 4 = 17; o cliente recusa outro.
    assert_eq!(p5.data.len(), 2 + 4 + 17);

    // 6. SERVER_CONFIG_DATA / INST_DATA_CHECKOUT (CMD 206)
    let p6 = S2CGamedataSend::inst_data_checkout(1, 1156141381, 1156141381, 1206433535, 1206433535);
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

    // O bloco de dados da arma sai da ficha que vem do `elements.data`. A durabilidade
    // passa como chegou: quem converte para a escala do cliente é o chamador.
    let ficha = pw_core::FichaDaArma {
        tipo_de_arma: 0, // corpo a corpo
        classes_permitidas: 767,
        nivel_exigido: 1,
        forca_exigida: 5,
        vitalidade_exigida: 0,
        agilidade_exigida: 0,
        energia_exigida: 3,
        municao_exigida: 0,
        tipo_maior: 292,
        dano_minimo: 3,
        dano_maximo: 3,
        dano_magico_minimo: 5,
        dano_magico_maximo: 5,
        velocidade_de_ataque: 0,
        alcance: 3.0,
    };
    let arma = S2CGamedataSend::item_info(
        1, 0, 2251, 2800, 2800, 1, &[],
        Some(pw_core::FichaDoEquipamento::Arma(ficha)),
    );
    assert_eq!(u16::from_le_bytes([arma.data[0], arma.data[1]]), 40);

    // Cabeçalho do comando: 2 + 1 + 1 + 4 + 4 + 4 + 4 + 2 = 22 bytes, depois o tamanho do
    // bloco (2) e o bloco. Os requisitos são os seis primeiros `short` do bloco.
    const BLOCO: usize = 22 + 2;
    let s16 = |off: usize| i16::from_le_bytes([arma.data[off], arma.data[off + 1]]);
    let s32 = |off: usize| {
        i32::from_le_bytes([
            arma.data[off], arma.data[off + 1], arma.data[off + 2], arma.data[off + 3],
        ])
    };
    assert_eq!(s16(BLOCO), 1, "nível exigido");
    assert_eq!(s16(BLOCO + 2), 767, "máscara de classes — zero aqui recusa todo mundo");
    assert_eq!(s16(BLOCO + 4), 5, "força exigida");
    assert_eq!(s16(BLOCO + 10), 3, "energia exigida");
    assert_eq!(s32(BLOCO + 12), 2800, "durabilidade passa como chegou");
    assert_eq!(s32(BLOCO + 16), 2800);

    // O `weapon_type` fica logo depois do cabeçalho da ficha (2 do tamanho + 1 + 1).
    assert_eq!(
        s16(BLOCO + 20 + 4),
        0,
        "a Varinha não pode viajar como arma de munição: era o que a deixava vermelha"
    );

    // Sem ficha, o comando vai sem bloco nenhum — melhor do que inventar requisito.
    let sem = S2CGamedataSend::item_info(0, 3, 1796, 0, 0, 10, &[], None);
    assert_eq!(sem.data.len(), 24, "cabeçalho de 22 mais o tamanho do bloco em zero");
}

/// A armadura e o acessório têm o mesmo cabeçalho da arma e essências próprias — e sem
/// bloco nenhum o cliente recusa a peça para todas as classes.
///
/// Referências: `generate_armor` e `generate_decoration`
/// (`EvolvedPWServer/cgame/gs/template/generate_item_temp.h:490-556` e `772-830`) do lado
/// do servidor original; `IVTR_ESSENCE_ARMOR` e `IVTR_ESSENCE_DECORATION`
/// (`EC_IvtrTypes.h:244-260`) do lado do cliente.
#[test]
fn o_bloco_da_armadura_e_o_do_acessorio_saem_com_a_essencia_da_familia_certa() {
    // Cabeçalho do comando (22) + o tamanho do bloco (2).
    const BLOCO: usize = 22 + 2;
    // Dentro do bloco: 6 shorts de requisito (12) + 2 ints de durabilidade (8) + o
    // tamanho da essência (2) + os 2 bytes da marca do fabricante = 24.
    const ESSENCIA: usize = BLOCO + 24;

    let ficha = pw_core::FichaDaArmadura {
        classes_permitidas: 767,
        nivel_exigido: 3,
        forca_exigida: 7,
        vitalidade_exigida: 11,
        agilidade_exigida: 13,
        energia_exigida: 17,
        defesa: 41,
        evasao: 5,
        mp_extra: 60,
        hp_extra: 90,
        resistencias: [1, 2, 3, 4, 5],
    };
    let p = S2CGamedataSend::item_info(
        1, 4, 10001, 500, 700, 1, &[],
        Some(pw_core::FichaDoEquipamento::Armadura(ficha)),
    );
    let s16 = |d: &[u8], off: usize| i16::from_le_bytes([d[off], d[off + 1]]);
    let s32 = |d: &[u8], off: usize| {
        i32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
    };

    assert_eq!(u16::from_le_bytes([p.data[0], p.data[1]]), 40);
    assert_eq!(s16(&p.data, BLOCO), 3, "nível exigido");
    assert_eq!(
        s16(&p.data, BLOCO + 2),
        767,
        "máscara de classes — é o campo que decide se a peça aparece vermelha"
    );
    assert_eq!(s16(&p.data, BLOCO + 4), 7, "força");
    // A ordem do original: vitalidade **antes** de agilidade.
    assert_eq!(s16(&p.data, BLOCO + 6), 11, "vitalidade");
    assert_eq!(s16(&p.data, BLOCO + 8), 13, "agilidade");
    assert_eq!(s16(&p.data, BLOCO + 10), 17, "energia");
    assert_eq!(s32(&p.data, BLOCO + 12), 500, "durabilidade atual");
    assert_eq!(s32(&p.data, BLOCO + 16), 700, "durabilidade máxima");
    assert_eq!(
        s16(&p.data, BLOCO + 20),
        36,
        "sizeof(IVTR_ESSENCE_ARMOR): o cliente tem um ASSERT em cima disto"
    );

    assert_eq!(s32(&p.data, ESSENCIA), 41, "defense");
    assert_eq!(s32(&p.data, ESSENCIA + 4), 5, "armor (evasão)");
    assert_eq!(s32(&p.data, ESSENCIA + 8), 60, "mp_enhance");
    assert_eq!(s32(&p.data, ESSENCIA + 12), 90, "hp_enhance");
    for n in 0..5 {
        assert_eq!(
            s32(&p.data, ESSENCIA + 16 + n * 4),
            n as i32 + 1,
            "resistance[{n}]"
        );
    }
    // Depois da essência: buracos (2), máscara de cravos (2), propriedades (4).
    assert_eq!(p.data.len(), ESSENCIA + 36 + 8);

    // O acessório: mesmos 36 bytes, ordem diferente. Trocar as duas essências passaria
    // por qualquer teste de tamanho.
    let dec = pw_core::FichaDeDecoracao {
        classes_permitidas: 1023,
        nivel_exigido: 20,
        forca_exigida: 0,
        vitalidade_exigida: 0,
        agilidade_exigida: 0,
        energia_exigida: 0,
        dano: 7,
        dano_magico: 9,
        defesa: 12,
        evasao: 2,
        resistencias: [10, 20, 30, 40, 50],
    };
    let p = S2CGamedataSend::item_info(
        1, 6, 20001, 100, 100, 1, &[],
        Some(pw_core::FichaDoEquipamento::Decoracao(dec)),
    );
    assert_eq!(s16(&p.data, BLOCO + 20), 36, "sizeof(IVTR_ESSENCE_DECORATION)");
    assert_eq!(s32(&p.data, ESSENCIA), 7, "damage vem primeiro no acessório");
    assert_eq!(s32(&p.data, ESSENCIA + 4), 9, "magic_damage");
    assert_eq!(s32(&p.data, ESSENCIA + 8), 12, "defense");
    assert_eq!(s32(&p.data, ESSENCIA + 12), 2, "armor");
    assert_eq!(s32(&p.data, ESSENCIA + 16), 10, "resistance[0]");
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
    let com_terceiro = sub_155.inst_data_checkout(1, 10, 20, 30, 35, Some(40));
    // 2 (cabeçalho) + 4 (idInst) + 4 + 4 + 4 (gshop) + 4 (gshop2) + 4 (gshop3) = 26 bytes.
    assert_eq!(com_terceiro.data.len(), 26);
    let gshop3_no_fio = u32::from_le_bytes(com_terceiro.data[22..26].try_into().unwrap());
    assert_eq!(gshop3_no_fio, 40);

    // Sem `Some`, o 1.5.5 continua no layout de cinco campos — ninguém é forçado a
    // fornecer um terceiro timestamp que não tem.
    let sem_terceiro = sub_155.inst_data_checkout(1, 10, 20, 30, 35, None);
    assert_eq!(sem_terceiro.data.len(), 22);

    // E o 1.5.3 ignora `Some` — o sexto campo é só para quem mediu precisar dele.
    let sub_153 = PorVersao::new(GameVersion::V1_5_3);
    let v153_com_some = sub_153.inst_data_checkout(1, 10, 20, 30, 35, Some(40));
    assert_eq!(v153_com_some.data.len(), 22, "1.5.3 não ganha o sexto campo só por receber Some");
}

#[test]
fn test_self_info_1_155_ganha_o_state2() {
    // Achado em 2026-09-03 lendo `cmd_self_info_1::CheckValid` em EC_GPDataType.h (source
    // do client 1.5.5, F:\PW\1.5.5\EvolvedPWClient — sem captura disponível para essa
    // versão): faltando o `state2` (int) depois do `state`, `CalcS2CCmdDataSize` recusa o
    // pacote como "unknown command" (CHECK_VALID falha), o `case SELF_INFO_1` que cancela
    // o timeout `OT_ENTERGAME` nunca roda, e o cliente trava 30s na tela de login e
    // desconecta com "EnterWorld Overtime".
    let pos = Vector3::new(10.0, 20.0, 30.0);

    let sub_126 = PorVersao::new(GameVersion::V1_2_6);
    let pacote_126 = sub_126.self_info_1(1000, 500, 1024, pos, 32);
    // 2 (cabeçalho) + 34 (cmd_self_info_1 do 1.2.6, sem state2) = 36 bytes.
    assert_eq!(pacote_126.data.len(), 36, "1.2.6 continua nos 34 bytes de sempre");

    let sub_155 = PorVersao::new(GameVersion::V1_5_5);
    let pacote_155 = sub_155.self_info_1(1000, 500, 1024, pos, 32);
    // 2 (cabeçalho) + 34 + 4 (state2) = 40 bytes.
    assert_eq!(pacote_155.data.len(), 40, "1.5.5 precisa do state2 de 4 bytes no fim");
}

#[test]
fn test_player_waypoint_list_devolve_os_ids_recebidos() {
    // Achado em 2026-09-03: sem essa resposta, o client (EC_World.cpp, checagem que roda a
    // cada quadro) nunca considera um waypoint "conhecido" e reenvia ACTIVATE_REGION_WAYPOINTS
    // (C2S 178) pra sempre — media de ~166 vezes por segundo num teste real, com a tela de
    // entrada no mundo travada. `count` é `size_t` no engine (32 bits) = 4 bytes, não 2.
    let pacote = S2CGamedataSend::player_waypoint_list(&[5201, 100]);
    assert_eq!(u16::from_le_bytes([pacote.data[0], pacote.data[1]]), 180);
    let count = u32::from_le_bytes(pacote.data[2..6].try_into().unwrap());
    assert_eq!(count, 2);
    assert_eq!(u16::from_le_bytes([pacote.data[6], pacote.data[7]]), 5201);
    assert_eq!(u16::from_le_bytes([pacote.data[8], pacote.data[9]]), 100);
    assert_eq!(pacote.data.len(), 10);
}

/// O `color_name` do fonte do 1.5.5 **não existe** no binário que serve este projeto.
///
/// Medido em jogo com o overlay do `d_rtdebug` em 2026-09-08:
/// `SERVER - Invalid GAMEDATA_82 size(Network:12, Client:8)` — doze bytes saindo do
/// servidor, oito esperados pelo `elementclient.exe`. A diferença é exatamente o campo.
/// O teste anterior fixava os 12 bytes, com um comentário que já avisava que a evidência
/// era o fonte e não uma captura.
#[test]
fn test_get_own_money_tem_oito_bytes_nas_duas_versoes() {
    for versao in [GameVersion::V1_5_3, GameVersion::V1_5_5] {
        let pacote = PorVersao::new(versao).get_own_money(1000, 2_000_000_000);
        assert_eq!(
            pacote.data.len(),
            10,
            "{versao:?}: 2 (cabeçalho) + 8 (amount, max_amount), sem color_name"
        );
    }
}

/// Mesma história do `get_own_money`, no comando que trava o modelo 3D do outro jogador.
///
/// `cmd_equip_data` do binário: `crc(2) + idPlayer(4) + mask(8)` e um `int` por bit ligado
/// na máscara. Com o `color_name` do fonte na frente, o corpo saía 4 bytes maior e o
/// cliente descartava o comando inteiro — `SERVER - Unknown GAMEDATA_66` no overlay.
#[test]
fn test_equip_data_nao_leva_color_name() {
    for versao in [GameVersion::V1_5_3, GameVersion::V1_5_5] {
        let sub = PorVersao::new(versao);

        let vazio = sub.equip_data(40, 0, 0, &[]);
        assert_eq!(vazio.data.len(), 16, "{versao:?}: 2 + 14 de prefixo, sem item");

        let com_arma = sub.equip_data(40, 0, 1, &[2251]);
        assert_eq!(com_arma.data.len(), 20, "{versao:?}: 2 + 14 + 4 do único item");

        // O cliente confere `buf_size == 14 + 4 * bits_ligados`; um item a menos do que a
        // máscara promete faz o comando ser descartado por tamanho.
        let tres = sub.equip_data(40, 0, 0b1011, &[2251, 1234, 999]);
        assert_eq!(tres.data.len(), 2 + 14 + 3 * 4, "{versao:?}");
    }
}

#[test]
fn test_server_time_leva_o_lua_version_certo() {
    // Regressão do bug achado em 2026-09-03: `lua_version = 0` faz o client comparar contra
    // a primeira linha do seu `global_api.lua` local (`--102`), não bater, setar
    // `FATAL_ERROR_WRONG_CONFIGDATA` e fechar o processo — "exit process because wrong
    // config data" nos dois clients de teste, ~1.3s depois do SetServerTime.
    let pacote = S2CGamedataSend::server_time(1_700_000_000, 0, 102);
    assert_eq!(u16::from_le_bytes([pacote.data[0], pacote.data[1]]), 114);
    let lua_version = i32::from_le_bytes(pacote.data[10..14].try_into().unwrap());
    assert_eq!(lua_version, 102, "lua_version errado derruba o client, não é cosmético");
}

#[test]
fn test_inst_data_checkout_gshop_e_gshop2_sao_valores_diferentes() {
    // Regressão do bug achado em 2026-09-03: `S2CGamedataSend::inst_data_checkout`
    // escrevia o mesmo `gshop_ts` duas vezes no fio, então o cliente sempre via
    // `gshop_time_stamp2` errado (igual ao primeiro, nunca o valor real de
    // `gshop2.data`/`gshop1.data`) — um cliente 1.5.5 real recusava a instância com
    // "gshop1 timestamp error" mesmo depois do handshake de login já ter passado.
    let sub = PorVersao::new(GameVersion::V1_5_3);
    let pacote = sub.inst_data_checkout(1, 10, 20, 0x1111_1111, 0x2222_2222, None);
    let gshop_no_fio = u32::from_le_bytes(pacote.data[14..18].try_into().unwrap());
    let gshop2_no_fio = u32::from_le_bytes(pacote.data[18..22].try_into().unwrap());
    assert_eq!(gshop_no_fio, 0x1111_1111);
    assert_eq!(gshop2_no_fio, 0x2222_2222, "gshop_time_stamp2 tem que ser o valor de gshop2, não uma cópia do gshop");
}

/// O `OWN_EXT_PROP` (50) tem **196** bytes de corpo — medido em jogo, não deduzido:
/// `SERVER - Invalid GAMEDATA_50 size(Network:188, Client:196)`.
///
/// São doze inteiros de cabeçalho e o `ROLEEXTPROP` de 148 (`bs` 32 + `mv` 16 + `ak` 68 +
/// `df` 28 + `max_ap` 4). O IR do 1.5.3 diz dez inteiros (188) e o fonte do
/// `EvolvedPWClient` diz vinte (228): o binário fica **entre os dois**, com
/// `anti_defense_degree` e `anti_resistance_degree` mas sem os quatro campos que o fonte
/// marca `// NEW`. Errar isso derruba o comando inteiro, e com ele os atributos que fazem
/// o equipamento sair do vermelho.
#[test]
fn test_own_ext_prop_tem_196_bytes_e_os_atributos_no_lugar() {
    let p = S2CGamedataSend::own_ext_prop(
        3,
        (10, 20, 15, 12), // vitalidade, energia, força, agilidade
        130,
        280,
        (2, 3),
        (1.5, 4.8, 2.2, 5.0),
        (7, 11, 19, 30, 1.4),
        (23, 29),
    );
    assert_eq!(p.data.len(), 2 + 196, "cabeçalho de 2 + os 196 bytes medidos no cliente");
    assert_eq!(u16::from_le_bytes(p.data[0..2].try_into().unwrap()), 50);

    let i32_em = |off: usize| i32::from_le_bytes(p.data[off..off + 4].try_into().unwrap());
    let f32_em = |off: usize| f32::from_le_bytes(p.data[off..off + 4].try_into().unwrap());

    assert_eq!(i32_em(2), 3, "status_point");

    // ROLEEXTPROP começa em 40 (mais os 2 do cabeçalho). É a força, em 40+8, que decide
    // se `CanUseEquipment` aceita a arma.
    const BS: usize = 2 + 48;
    assert_eq!(i32_em(BS), 10, "vitalidade");
    assert_eq!(i32_em(BS + 4), 20, "energia");
    assert_eq!(i32_em(BS + 8), 15, "força — é este campo que solta o equipamento");
    assert_eq!(i32_em(BS + 12), 12, "agilidade");
    assert_eq!(i32_em(BS + 16), 130, "max_hp");
    assert_eq!(i32_em(BS + 20), 280, "max_mp");

    assert_eq!(f32_em(BS + 32), 1.5, "walk_speed");
    assert_eq!(i32_em(BS + 48), 7, "attack rate");
    assert_eq!(f32_em(BS + 64), 1.4, "attack_range");

    // `df` fica em 116 dentro do ROLEEXTPROP, e a defesa em +20 dele.
    assert_eq!(i32_em(BS + 116 + 20), 23, "defense");
    assert_eq!(i32_em(BS + 116 + 24), 29, "armor");
    assert_eq!(i32_em(BS + 144), 0, "max_ap");
}

/// `HOST_SKILL_ATTACKED` (144) tem 19 bytes de corpo — o IR e o cabeçalho do cliente
/// concordam (`S2C::cmd_host_skill_attacked`, `EC_GPDataType.h:2763`).
///
/// O `cEquipment` vai em `0x7f`, que é o valor que o cliente lê como "nenhuma peça se
/// desgastou": `(pCmd->cEquipment & 0x7f) != 0x7f` é a condição para gastar durabilidade
/// (`EC_HostMsg.cpp:1030`). Qualquer outro valor comeria a durabilidade de uma peça a cada
/// golpe recebido.
#[test]
fn test_host_skill_attacked_tem_19_bytes_e_nao_gasta_equipamento() {
    let p = S2CGamedataSend::host_skill_attacked(40, 125, 234, 0, 30, 0);
    assert_eq!(p.data.len(), 2 + 19);
    assert_eq!(u16::from_le_bytes(p.data[0..2].try_into().unwrap()), 144);

    let i32_em = |off: usize| i32::from_le_bytes(p.data[off..off + 4].try_into().unwrap());
    assert_eq!(i32_em(2), 40, "idAttacker");
    assert_eq!(i32_em(6), 125, "idSkill");
    assert_eq!(i32_em(10), 234, "iDamage");
    assert_eq!(p.data[14], 0x7f, "cEquipment: nenhuma peça desgastada");
    assert_eq!(i32_em(15), 0, "attack_flag");
    assert_eq!(p.data[19], 30, "speed");
    assert_eq!(p.data[20], 0, "section");
}

/// `MATTER_ENTER_WORLD` (18) tem 25 bytes de payload, e o número não é escolha nossa.
///
/// `S2C::cmd_matter_enter_world` é uma `info_matter` (`EC_GPDataType.h:784-794`) dentro de
/// um `#pragma pack(1)` (`:563`), idêntica à `INFO::matter_info_1` do servidor original
/// (`cgame/common/protocol.h:86-96`). Um byte a mais ou a menos e o cliente descarta o
/// comando **em silêncio** — foi o que escondeu quatro defeitos seguidos (item 46).
#[test]
fn matter_enter_world_tem_vinte_e_cinco_bytes_e_nasce_em_pe() {
    let p = S2CGamedataSend::matter_enter_world(
        0xC000_1234u32 as i32,
        8582,
        pw_core::Vector3::new(-2412.94, 246.05, 4347.46),
    );

    assert_eq!(u16::from_le_bytes([p.data[0], p.data[1]]), 18);
    assert_eq!(
        p.data.len(),
        2 + 25,
        "cabeçalho de 2 mais os 25 bytes de info_matter: 4 + 4 + 12 + 5"
    );

    let s32 = |off: usize| i32::from_le_bytes([p.data[off], p.data[off + 1], p.data[off + 2], p.data[off + 3]]);
    assert_eq!(s32(2), 0xC000_1234u32 as i32, "mid — os dois bits altos são ISMATTERID");
    assert_eq!(s32(6), 8582, "tid");

    // dir0, dir1, rad, state, value — os cinco últimos bytes.
    //
    // `a3d_DecompressDir(0, 0)` devolve o eixo Y (`A3DVectorComp.cpp:238-249`), e `rad`
    // zero não gira: a peça nasce em pé, sem rotação. `state` zero é recurso comum — o bit
    // 0 marcaria objeto de modelo dinâmico e o 1, mina de espírito
    // (`EC_Matter.cpp:167-168`), e nenhum dos dois é minério.
    assert_eq!(&p.data[22..27], &[0, 0, 0, 0, 0]);
}

/// `OUT_OF_SIGHT_LIST` (34) é `unsigned int uCount` e os ids.
///
/// É o único caminho de saída da matéria: o `OBJECT_LEAVE_SLICE` (13) só trata
/// `ISPLAYERID` e `ISNPCID` (`EC_GameDataPrtc.cpp:891-899`), e um id de matéria mandado
/// por ele não faz nada — nem erro, nem efeito.
#[test]
fn out_of_sight_list_leva_a_contagem_antes_dos_ids() {
    let ids = [0xC000_0001u32 as i32, 0xC000_0002u32 as i32, 0xC000_0003u32 as i32];
    let p = S2CGamedataSend::out_of_sight_list(&ids);

    assert_eq!(u16::from_le_bytes([p.data[0], p.data[1]]), 34);
    assert_eq!(u32::from_le_bytes([p.data[2], p.data[3], p.data[4], p.data[5]]), 3);
    assert_eq!(p.data.len(), 2 + 4 + 3 * 4);
    for (n, esperado) in ids.iter().enumerate() {
        let off = 6 + n * 4;
        assert_eq!(
            i32::from_le_bytes([p.data[off], p.data[off + 1], p.data[off + 2], p.data[off + 3]]),
            *esperado
        );
    }

    // Lista vazia é comando válido, e o servidor não deve mandá-la — mas se mandar, o
    // cliente lê zero e não faz nada.
    assert_eq!(S2CGamedataSend::out_of_sight_list(&[]).data.len(), 6);
}

/// O sexo de outro jogador viaja num bit do `state2`, e mandar zero afirma que todo mundo
/// é homem.
///
/// Medido em jogo em 2026-09-11: duas sacerdotisas se afastaram além do raio de visão e,
/// ao voltarem, apareceram uma para a outra como **modelo masculino, com barba e cabelo de
/// padrão**. Os logs do realm mostram por quê — na reentrada o cliente **não** pede
/// `PlayerBaseInfo` nem `GetCustomData` de novo (2 pedidos na sessão inteira, os dois do
/// primeiro encontro), porque ele já tem aquele jogador em cache. O único sexo que ele tem
/// à mão é o do pacote:
///
/// ```cpp
/// unsigned char GetGender() const {
///     return (state2 & GP_STATE2_GENDER) ? GENDER_FEMALE : GENDER_MALE;
/// }
/// ```
///
/// (`EC_GPDataType.h:709-711`, usado em `CECElsePlayer::InitFromCache`,
/// `EC_ElsePlayer.cpp:220`.) O original liga o mesmo bit em `SetPlayerClass`
/// (`gs/player_imp.h:1886-1891`, `STATE_PLAYER_GENDER = 0x40`).
#[test]
fn o_sexo_do_jogador_viaja_no_bit_do_state2() {
    let mulher = pw_core::VistaDoJogador {
        pos: pw_core::Vector3::new(1.0, 2.0, 3.0),
        dir: 0,
        sec_level: 0,
        feminino: true,
        crc_equipamento: 0,
        crc_aparencia: 0xBEEF,
    };
    let p = S2CGamedataSend::player_enter_slice(42, mulher);

    assert_eq!(u16::from_le_bytes([p.data[0], p.data[1]]), 12);
    // Do fim do cabeçalho: cid 2..6, pos 6..18, crc_e 18..20, crc_c 20..22, dir 22,
    // level2 23, state 24..28, state2 28..32.
    let s32 = |off: usize| i32::from_le_bytes([p.data[off], p.data[off + 1], p.data[off + 2], p.data[off + 3]]);
    assert_eq!(
        s32(28),
        0x40,
        "sem este bit o cliente desenha a personagem como homem"
    );
    assert_eq!(
        u16::from_le_bytes([p.data[20], p.data[21]]),
        0xBEEF,
        "crc_c — o carimbo da aparência"
    );

    // E o tamanho não muda: nenhum dos bits que este servidor liga acrescenta bytes ao
    // comando (`info_player_1::CheckValid`, `EC_GPDataType.h:625-705`).
    let homem = pw_core::VistaDoJogador { feminino: false, ..mulher };
    let q = S2CGamedataSend::player_enter_slice(42, homem);
    assert_eq!(p.data.len(), q.data.len(), "o bit do sexo não pode mudar o tamanho");
    assert_eq!(s32_de(&q.data, 28), 0, "homem não liga bit nenhum");
}

fn s32_de(d: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// O carimbo da aparência tem de ser o **mesmo** nos dois lugares em que viaja, senão o
/// cliente ou nunca atualiza o visual, ou o repede a cada reaparição.
///
/// `crc_c` da `info_player_1` sai de `PlayerEntity::crc_aparencia`, calculado sobre os
/// bytes que o `custom_appearance` do banco guarda; `custom_stamp` do `PlayerBaseInfo_Re`
/// sai dos mesmos bytes, pela coluna `custom_data`. As duas formas que o repositório
/// produz para "sem aparência" — `null` e `{}` — têm de carimbar zero, que é o que o link
/// carimba para uma lista vazia.
#[test]
fn o_carimbo_da_aparencia_concorda_nos_dois_caminhos() {
    let bytes = [0xDEu8, 0xAD, 0xBE, 0xEF, 0x01, 0x02];
    let do_json = serde_json::json!({ "raw": hex::encode(bytes) });

    assert_eq!(pw_core::bytes_da_aparencia(&do_json), bytes.to_vec());
    assert_eq!(
        pw_core::stamp_de_aparencia(&pw_core::bytes_da_aparencia(&do_json)),
        pw_core::stamp_de_aparencia(&bytes),
        "o mundo e o link carimbariam valores diferentes para a mesma aparência"
    );

    for vazio in [serde_json::Value::Null, serde_json::json!({})] {
        assert!(pw_core::bytes_da_aparencia(&vazio).is_empty(), "{vazio:?}");
        assert_eq!(
            pw_core::stamp_de_aparencia(&pw_core::bytes_da_aparencia(&vazio)),
            0,
            "sem aparência tem de carimbar zero, como o link faz"
        );
    }

    // E o carimbo muda quando a aparência muda — sem isso o cliente nunca refaria o
    // modelo de quem trocou de visual.
    let mut outra = bytes;
    outra[0] ^= 0xFF;
    assert_ne!(
        pw_core::stamp_de_aparencia(&bytes),
        pw_core::stamp_de_aparencia(&outra)
    );
    // Um carimbo real nunca colide com o zero reservado.
    assert_ne!(pw_core::stamp_de_aparencia(&bytes), 0);
}

/// `CALC_NETWORK_DELAY_RE` (291) devolve o `timestamp` do cliente sem tocar nele.
///
/// O cliente descarta a resposta se o valor não bater com o que ele guardou
/// (`EC_GameRun.cpp:3155`), então inventar um relógio nosso aqui seria o mesmo que não
/// responder.
#[test]
fn a_resposta_de_latencia_devolve_o_relogio_do_cliente() {
    let p = S2CGamedataSend::calc_network_delay_re(0x1234_5678);
    assert_eq!(u16::from_le_bytes([p.data[0], p.data[1]]), 291);
    assert_eq!(p.data.len(), 2 + 4);
    assert_eq!(s32_de(&p.data, 2), 0x1234_5678);
}

/// A marca das missões dinâmicas vai no `TASK_VAR_DATA` (106) com o
/// `svr_task_dyn_time_mark` de 9 bytes: `reason` 8, `task` 0, a marca e a versão 10.
///
/// O cliente só aceita esse tamanho exato e essa versão (`TaskClient.cpp:290-296`,
/// `TaskTemplMan.cpp:168`); e o `reason` 7, que o `gateway.rs` mandava, é
/// `TASK_SVR_NOTIFY_FORGET_SKILL` — outra ordem, não a marca.
#[test]
fn a_marca_das_missoes_dinamicas_vai_com_reason_8_e_nove_bytes() {
    let p = S2CGamedataSend::task_dyn_time_mark(0x5277_6c0d);
    let d = &p.data;
    assert_eq!(u16::from_le_bytes([d[0], d[1]]), 106, "TASK_VAR_DATA");
    assert_eq!(s32_de(d, 2), 9, "size = sizeof(svr_task_dyn_time_mark)");
    assert_eq!(d.len(), 2 + 4 + 9);
    assert_eq!(d[6], 8, "reason = TASK_SVR_NOTIFY_DYN_TIME_MARK");
    assert_eq!(u16::from_le_bytes([d[7], d[8]]), 0, "task");
    assert_eq!(s32_de(d, 9) as u32, 0x5277_6c0d, "time_mark");
    assert_eq!(u16::from_le_bytes([d[13], d[14]]), 10, "version = DYN_TASK_CUR_VERSION");
}
