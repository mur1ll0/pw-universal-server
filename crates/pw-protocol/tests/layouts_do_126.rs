//! Os codificadores por versão, contra os tamanhos **medidos** num servidor 1.2.6 real.
//!
//! # De onde vem o gabarito
//!
//! Não do nosso código, e não do IR: de uma captura de rede do elo interno
//! (`gs -> glinkd`) de um servidor 1.2.6 em funcionamento — 67.482 pacotes, 0 descartados,
//! 22.217 quadros GNET, sessão de 22 minutos com um roteiro de 45 passos.
//! `docs/MEDIDAS_DO_126.md` traz a saída completa da ferramenta que a leu.
//!
//! A tabela [`MEDIDO`] abaixo é **transcrita daquele relatório**, com a contagem de
//! ocorrências junto — não para enfeitar, mas porque um comando visto 80 vezes com o
//! mesmo tamanho vale mais que um visto uma vez.
//!
//! # Por que este teste é diferente dos outros
//!
//! Os outros testes de layout comparam o código com o **IR**, que é um documento. Este
//! compara com o **fio**, que é o que o cliente realmente recebeu. Onde os dois
//! discordarem, é o fio que ganha — e o item 46 explica por quê: o cliente confere o
//! tamanho e **descarta em silêncio** o que não bate, então um comando com o tamanho
//! errado não dá erro nenhum, só deixa de funcionar.
//!
//! # O que este teste não faz
//!
//! Não confere os **valores**, só os tamanhos. Saber que o `NPC_INFO_00` do 1.2.6 tem 12
//! bytes não prova que os três `int` estão na ordem certa — isso veio de olhar os bytes
//! capturados (o `iHP` caindo enquanto o `iMaxHP` fica parado) e está registrado na
//! documentação de cada função.

use pw_protocol::{versions::create_world_protocol, GameVersion, WorldProtocol};

/// `(nome, id, bytes de payload no 1.2.6, bytes no 1.5.3, ocorrências na captura)`.
///
/// Transcrito de `docs/MEDIDAS_DO_126.md`. **Se alguém mudar um número aqui, tem que ser
/// porque uma captura nova mediu diferente** — não porque o código mudou.
const MEDIDO: &[(&str, u16, usize, usize, usize)] = &[
    ("HOST_ATTACKRESULT", 24, 10, 13, 52),
    ("HOST_ATTACKED", 26, 11, 14, 25),
    ("PLAYER_INFO_00", 32, 24, 28, 73),
    ("NPC_INFO_00", 33, 12, 16, 80),
    ("RECEIVE_EXP", 36, 4, 8, 36),
    ("EQUIP_ITEM", 48, 6, 10, 9),
    ("HOST_SKILL_ATTACK_RESULT", 142, 14, 18, 18),
    ("ENTER_SANCTUARY", 164, 0, 4, 11),
    ("LEAVE_SANCTUARY", 165, 0, 4, 9),
    ("INST_DATA_CHECKOUT", 206, 16, 20, 3),
];

/// Chama cada codificador com argumentos quaisquer e devolve `(id, payload)`.
///
/// Os valores não importam para esta conferência — o que se mede é o **tamanho**. Valores
/// distintos entre si, mesmo assim, para que uma troca de campos apareça noutro teste.
fn escrever(p: &std::sync::Arc<dyn WorldProtocol>, id: u16) -> Vec<u8> {
    let d = match id {
        24 => p.host_attack_result(101, 7, 0, 0x10).data,
        26 => p.host_attacked(102, 1, 0x7f, 0, 0x1b).data,
        32 => p.player_info_00(48, 3, 0, 153, 154, 83, 84, 0).data,
        33 => p.npc_info_00(900_001, 29, 30, 0).data,
        36 => p.receive_exp(15, 36).data,
        48 => p.equip_item(7, 0, 1, 0).data,
        142 => p.self_skill_attack_result(103, 102, 17, 0, 6, 0).data,
        164 => p.enter_sanctuary(48).data,
        165 => p.leave_sanctuary(48).data,
        206 => p.inst_data_checkout(1, 0x46b1_a9ac, 0x46b1_a9ac, 0x47e8_b6ff, 0x47e8_b6ff, None).data,
        outro => panic!("o comando {outro} está na tabela e não tem chamada aqui"),
    };
    d
}

#[test]
fn o_126_escreve_o_tamanho_que_o_servidor_de_verdade_escreveu() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    let mut erros = Vec::new();

    for (nome, id, bytes_126, _, vezes) in MEDIDO {
        let d = escrever(&p, *id);
        let cabecalho = u16::from_le_bytes([d[0], d[1]]);
        if cabecalho != *id {
            erros.push(format!("{nome} ({id}): escreveu o cabeçalho {cabecalho}"));
            continue;
        }
        let payload = d.len() - 2;
        if payload != *bytes_126 {
            erros.push(format!(
                "{nome} ({id}): escreveu {payload} bytes; o servidor 1.2.6 escreveu \
                 {bytes_126}, medido em {vezes} ocorrências"
            ));
        }
    }

    assert!(
        erros.is_empty(),
        "layout do 1.2.6 diferente do medido:\n  {}",
        erros.join("\n  ")
    );
}

#[test]
fn o_153_continua_com_o_layout_do_ir() {
    // A ramificação por versão não pode ter mexido no que já estava certo para 1.5.3.
    let p = create_world_protocol(GameVersion::V1_5_3);
    let mut erros = Vec::new();

    for (nome, id, _, bytes_153, _) in MEDIDO {
        let payload = escrever(&p, *id).len() - 2;
        if payload != *bytes_153 {
            erros.push(format!("{nome} ({id}): {payload} bytes, o IR do 1.5.3 diz {bytes_153}"));
        }
    }

    assert!(erros.is_empty(), "o 1.5.3 regrediu:\n  {}", erros.join("\n  "));
}

#[test]
fn as_duas_versoes_diferem_em_todos_os_comandos_da_tabela() {
    // Um comando que sai igual nas duas versões não deveria estar neste módulo: ou a
    // ramificação não foi escrita, ou o comando não pertence aqui. Os dois casos são
    // erro, e este teste é o que os separa de "está tudo bem".
    let a = create_world_protocol(GameVersion::V1_2_6);
    let b = create_world_protocol(GameVersion::V1_5_3);

    for (nome, id, _, _, _) in MEDIDO {
        assert_ne!(
            escrever(&a, *id),
            escrever(&b, *id),
            "{nome} ({id}) saiu idêntico nas duas versões — a ramificação não está lá"
        );
    }
}

#[test]
fn o_148_usa_o_layout_do_153_por_falta_de_medicao() {
    // Não é uma afirmação sobre o 1.4.8: é o registro de que não temos captura dele. O
    // dia em que houver, este teste muda junto com a tabela — e é bom que ele exista para
    // que a mudança seja consciente em vez de silenciosa.
    let quatro_oito = create_world_protocol(GameVersion::V1_4_8);
    let cinco_tres = create_world_protocol(GameVersion::V1_5_3);

    for (_, id, _, _, _) in MEDIDO {
        assert_eq!(escrever(&quatro_oito, *id), escrever(&cinco_tres, *id));
    }
}

#[test]
fn o_receive_exp_do_126_cabe_em_16_bits_sem_estourar() {
    // Um abate que desse mais de 65.535 de experiência truncaria para um número pequeno e
    // aleatório: o jogador veria "ganhou 3 de exp" ao matar um chefe. O teto é errado por
    // menos.
    let p = create_world_protocol(GameVersion::V1_2_6);
    let d = p.receive_exp(70_000, -5).data;
    assert_eq!(d.len(), 2 + 4);
    assert_eq!(u16::from_le_bytes([d[2], d[3]]), u16::MAX, "não saturou o exp");
    assert_eq!(u16::from_le_bytes([d[4], d[5]]), 0, "exp negativa devia virar zero");
}

#[test]
fn o_hp_do_npc_info_00_do_126_fica_onde_a_captura_mostrou() {
    // A captura mostra o `iHP` caindo (29 → 22 → 17 → 11 → 2) enquanto o `iMaxHP` fica em
    // 29. É o que fixa a **ordem** dos dois campos, que o tamanho sozinho não fixaria.
    let p = create_world_protocol(GameVersion::V1_2_6);
    let d = p.npc_info_00(900_001, 11, 29, 0).data;
    assert_eq!(i32::from_le_bytes([d[2], d[3], d[4], d[5]]), 900_001, "idNPC");
    assert_eq!(i32::from_le_bytes([d[6], d[7], d[8], d[9]]), 11, "iHP fora do lugar");
    assert_eq!(i32::from_le_bytes([d[10], d[11], d[12], d[13]]), 29, "iMaxHP fora do lugar");
}

/// `TASK_DATA` (105): três blocos de tamanho no 1.2.6, cinco do 1.5.3 em diante.
///
/// Medido por desmontagem de `CECHostPlayer::OnMsgHstTaskData` nos dois clients reais
/// (ver `PorVersao::task_data` para os endereços e o raciocínio). Mandar três blocos pro
/// 1.5.5 fazia o cliente ler 8 bytes depois do fim do buffer e deixava
/// `CECHostPlayer[+0x1508]` nulo — o mesmo campo que aparece nulo no minidump do crash.
#[test]
fn task_data_tem_tres_blocos_no_126_e_cinco_do_153_em_diante() {
    let cabecalho = 2; // u16 com o id do subcomando (105)

    let p126 = create_world_protocol(GameVersion::V1_2_6);
    let b126 = p126.task_data().data;
    assert_eq!(&b126[..2], &105u16.to_le_bytes());
    assert_eq!(b126.len(), cabecalho + 3 * 4, "1.2.6 espera exatamente 3 blocos");
    assert!(b126[2..].iter().all(|&b| b == 0), "todos os tamanhos são zero");

    for versao in [GameVersion::V1_5_3, GameVersion::V1_5_5] {
        let b = create_world_protocol(versao).task_data().data;
        assert_eq!(&b[..2], &105u16.to_le_bytes());
        assert_eq!(b.len(), cabecalho + 5 * 4, "{versao:?} espera exatamente 5 blocos");
        assert!(b[2..].iter().all(|&x| x == 0), "todos os tamanhos são zero");
    }
}

/// `NPC_ENTER_WORLD`/`NPC_ENTER_SLICE`: 27 bytes de payload no 1.2.6, 35 do 1.5.3 em diante.
///
/// A struct `S2C::info_npc` do IR (idêntica em `gamedata_153.json` e `gamedata_155.json`)
/// tem `vis_tid` no meio e `state2` no fim; o layout que este projeto sempre escreveu era o
/// do 1.2.6, sem os dois. Com 27 bytes o client 1.5.5 recebia os NPCs e não desenhava
/// nenhum — ver `PorVersao::npc_enter_world`.
#[test]
fn info_npc_ganha_vis_tid_e_state2_do_153_em_diante() {
    use pw_core::Vector3;
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let cabecalho = 2;

    let b126 = create_world_protocol(GameVersion::V1_2_6).npc_enter_world(7, 2191, pos, 64).data;
    assert_eq!(&b126[..2], &16u16.to_le_bytes());
    assert_eq!(b126.len(), cabecalho + 27, "1.2.6: nid+tid+pos+seed+dir+state");

    for versao in [GameVersion::V1_5_3, GameVersion::V1_5_5] {
        let b = create_world_protocol(versao).npc_enter_world(7, 2191, pos, 64).data;
        assert_eq!(b.len(), cabecalho + 35, "{versao:?}: com vis_tid e state2");
        // vis_tid vem logo depois do tid e vai igual a ele
        let tid = i32::from_le_bytes(b[6..10].try_into().unwrap());
        let vis_tid = i32::from_le_bytes(b[10..14].try_into().unwrap());
        assert_eq!(tid, 2191);
        assert_eq!(vis_tid, tid, "vis_tid espelha o tid enquanto nada troca a aparência");
        // state2, os 4 últimos bytes, zerado
        assert_eq!(&b[b.len() - 4..], &0i32.to_le_bytes());
    }

    // O ENTER_SLICE carrega a mesma struct, só muda o id do comando.
    let slice = create_world_protocol(GameVersion::V1_5_5).npc_enter_slice(7, 2191, pos, 64).data;
    assert_eq!(&slice[..2], &11u16.to_le_bytes());
    assert_eq!(slice.len(), cabecalho + 35);
}

#[test]
fn own_ext_prop_tem_152_bytes_no_126_e_196_do_153_em_diante() {
    let p126 = create_world_protocol(GameVersion::V1_2_6);
    let p155 = create_world_protocol(GameVersion::V1_5_5);

    let d126 = p126.own_ext_prop(
        0, (10, 10, 10, 10), 100, 100, 99, (2, 2),
        (2.0, 4.9, 3.0, 5.0),
        (40, 5, 10, 22, 3.8),
        (2, 3)
    ).data;

    let d155 = p155.own_ext_prop(
        0, (10, 10, 10, 10), 100, 100, 99, (2, 2),
        (2.0, 4.9, 3.0, 5.0),
        (40, 5, 10, 22, 3.8),
        (2, 3)
    ).data;

    assert_eq!(d126.len() - 2, 152, "payload do OWN_EXT_PROP no 1.2.6 deve ser 152 bytes exatos");
    assert_eq!(d155.len() - 2, 196, "payload do OWN_EXT_PROP no 1.5.5 deve ser 196 bytes exatos");
    assert_eq!(i32::from_le_bytes(d126[d126.len() - 4..].try_into().unwrap()), 99, "o max_ap fecha o OWN_EXT_PROP do 1.2.6");
    assert_eq!(i32::from_le_bytes(d155[d155.len() - 4..].try_into().unwrap()), 99, "o max_ap fecha o OWN_EXT_PROP do 1.5.5");
}

// full_interno.pcap, S2C 66 amostra #1:
// docs/evidencias/126/s2c-66.txt:8-10. CRC=0x1c54, role=48, mask=0x11,
// itens 2258 e 154: 10 bytes de cabeçalho de payload + 2*4 de itens.
#[test]
fn equip_data_126_reproduz_a_mascara_de_32_bits_da_captura() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    let esperado = [
        66, 0, 0x54, 0x1c, 48, 0, 0, 0, 0x11, 0, 0, 0,
        0xd2, 8, 0, 0, 0x9a, 0, 0, 0,
    ];
    assert_eq!(p.equip_data(48, 0x1c54, 0x11, &[2258, 154]).data, esperado);
}

#[test]
fn equip_data_155_preserva_a_mascara_de_64_bits() {
    let p = create_world_protocol(GameVersion::V1_5_5);
    let esperado = [
        66, 0, 0x54, 0x1c, 48, 0, 0, 0, 0x11, 0, 0, 0, 1, 0, 0, 0,
        0xd2, 8, 0, 0, 0x9a, 0, 0, 0,
    ];
    assert_eq!(p.equip_data(48, 0x1c54, 0x1_0000_0011, &[2258, 154]).data, esperado);
}

#[test]
fn equip_data_126_nao_anexa_slots_que_sua_mascara_nao_representa() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert_eq!(
        p.equip_data(48, 0, 0x1_0000_0011, &[2258, 154, 999]).data,
        p.equip_data(48, 0, 0x11, &[2258, 154]).data,
    );
}

#[test]
fn o_126_nao_anuncia_comando_390_que_o_binario_recusa() {
    // elementclient.exe SHA256 em cliente-validacao-entrada.txt:2;
    // 0x584618 compara cmd com 0x104 (260), acima retorna inválido.
    let p = create_world_protocol(GameVersion::V1_2_6);
    assert!(p.scene_service_npc_list(&[(1, 123)]).is_none());
}

#[test]
fn o_155_continua_anunciando_os_npcs_de_servico() {
    let p = create_world_protocol(GameVersion::V1_5_5);
    assert_eq!(
        p.scene_service_npc_list(&[(1, 123)]).unwrap().data,
        pw_protocol::S2CGamedataSend::scene_service_npc_list(&[(1, 123)]).data,
    );
}

// Amostras originais, extraídas por pw-pcapdiff; não geradas pelo codificador.
fn primeira_amostra(texto: &str) -> Vec<u8> {
    texto.lines().skip(1).take_while(|l| !l.starts_with("## "))
        .filter(|l| l.len() >= 6 && l.as_bytes()[..4].iter().all(u8::is_ascii_hexdigit))
        .flat_map(|l| l.split_whitespace().skip(1).take(16))
        .filter_map(|b| if b.len() == 2 { u8::from_str_radix(b, 16).ok() } else { None })
        .collect()
}

#[test]
fn entrada_126_self_info_e_bolsa_vazia_reproduzem_o_original() {
    use pw_protocol::S2CGamedataSend;
    let self_info = primeira_amostra(include_str!("../../../docs/evidencias/126/s2c-38.txt"));
    assert_eq!(S2CGamedataSend::self_info_00(2, 0, 119, 119, 49, 49, 170, 541, 0, 0).data[2..], self_info);
    let bolsa = primeira_amostra(include_str!("../../../docs/evidencias/126/s2c-42.txt"));
    assert_eq!(S2CGamedataSend::own_ivtr_from_items(2, 32, &[]).data[2..], bolsa);
}

#[test]
fn entrada_126_propriedades_conferem_os_campos_representados_no_trait() {
    let original = primeira_amostra(include_str!("../../../docs/evidencias/126/s2c-50.txt"));
    let p = create_world_protocol(GameVersion::V1_2_6);
    let pacote = p.own_ext_prop(5, (5, 5, 5, 5), 119, 49, 0, (5, 1),
        (2.0, 4.9, 3.0, 5.0), (40, 6, 10, 22, 3.8), (3, 40));
    let corpo = &pacote.data[2..];
    assert_eq!(corpo.len(), 152);
    assert_eq!(&corpo[..112], &original[..112]);
    assert_eq!(&corpo[140..], &original[140..]);
    // [112..140] são ataque mágico/resistências que o trait atual não recebe.
    // Não declarar igualdade integral: esses valores ainda são zeros no servidor.
}

#[test]
fn entrada_126_atalhos_preservam_o_blob_original_sem_reescrever_versao() {
    use pw_protocol::packets::s2c::S2CGetUIConfigRe;
    use pw_protocol::octets::OctetsStream;
    let original = primeira_amostra(include_str!("../../../docs/evidencias/126/gnet-ui-105.txt"));
    assert_eq!(original.len(), 323);
    assert_eq!(&original[12..18], &[0x81, 0x35, 3, 0, 0, 0]);
    let mut s = OctetsStream::new();
    S2CGetUIConfigRe::new(48, 22, &original[14..]).encode(&mut s, "1.2.6");
    assert_eq!(s.into_bytes().as_ref(), original.as_slice());
}

#[test]
fn entrada_126_nao_envia_ids_acima_do_limite_do_binario() {
    let p = create_world_protocol(GameVersion::V1_2_6);
    for pacote in p.initial_status_notifications(17, 1788303334) {
        let id = u16::from_le_bytes([pacote.data[0], pacote.data[1]]);
        assert!(id <= 260, "comando {id} excede o limite em 0x584618 do cliente 126");
    }
}

#[test]
fn entrada_155_preserva_a_sequencia_e_os_bytes_anteriores() {
    use pw_protocol::S2CGamedataSend as S;
    let p = create_world_protocol(GameVersion::V1_5_5);
    let esperado = vec![S::host_reputation(17), S::pvp_mode(0), S::self_country_notify(0),
        S::server_time(1234, 0, 102), S::trashbox_pwd_state(false), S::pet_room_capacity(0),
        S::self_king_notify(false, 0), S::faction_contrib_notify(0, 0, 0),
        S::player_leadership(0, 0), S::player_world_contribution(0, 0, 0), S::player_dividend(0),
        S::available_double_exp_time(0), S::double_exp_time(0, 0), S::pariah_time(0)];
    let bytes = |v: Vec<S>| v.into_iter().map(|p| p.data).collect::<Vec<_>>();
    assert_eq!(bytes(p.initial_status_notifications(17, 1234)), bytes(esperado));
}
