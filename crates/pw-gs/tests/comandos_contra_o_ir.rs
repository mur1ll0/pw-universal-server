//! Os decodificadores de subcomando leem nos deslocamentos que o IR anuncia.
//!
//! # A técnica
//!
//! Em vez de repetir os deslocamentos no teste (o que só provaria que dois arquivos
//! escritos pela mesma pessoa concordam), o teste **monta o payload a partir do IR**:
//! para cada campo, escreve um valor distinto no deslocamento que o IR diz, e depois
//! cobra que a decodificação devolva aquele valor naquele campo.
//!
//! Um `r.u16()` fora de lugar desloca tudo que vem depois e derruba o teste. Um campo com
//! o tipo errado devolve outro número. Nenhum dos dois é visível lendo o código.
//!
//! Os deslocamentos do IR incluem o cabeçalho de 2 bytes; o payload que chega ao
//! decodificador já vem sem ele. A conta é feita aqui, uma vez, e não espalhada.

use pw_gs::comandos::{
    CastSkill, EmoteAction, GetIvtrDetail, Logout, MoveIvtrItem, NormalAttack, ParDeSlots,
    PlayerMove, SelectTarget, StopMove, UseItem, BYTES_DO_CABECALHO,
};
use pw_wire::gamedata::Vec3;
use serde_json::Value;

fn ir() -> Value {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gamedata_153.json"
    );
    let texto = std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    serde_json::from_str(&texto).expect("o IR não é JSON válido")
}

/// A struct do IR, pelo nome com escopo.
fn struct_do_ir<'a>(ir: &'a Value, nome: &str) -> &'a Value {
    ir["structs"]
        .get(nome)
        .unwrap_or_else(|| panic!("{nome} não existe no IR"))
}

/// Deslocamento de um campo **dentro do payload** (isto é, já sem o cabeçalho).
fn deslocamento(s: &Value, campo: &str) -> usize {
    let f = s["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == campo)
        .unwrap_or_else(|| panic!("campo `{campo}` não existe nesta struct do IR"));
    let no_ir = f["offset"].as_u64().expect("offset") as usize;
    assert!(
        no_ir >= BYTES_DO_CABECALHO,
        "o campo `{campo}` está no deslocamento {no_ir}, dentro do cabeçalho — o IR mudou \
         de forma e esta conta precisa ser revista"
    );
    no_ir - BYTES_DO_CABECALHO
}

/// Tamanho total do payload declarado pelo IR.
fn bytes_do_payload(s: &Value) -> usize {
    s["bytes"].as_u64().expect("bytes") as usize - BYTES_DO_CABECALHO
}

/// Escreve `bytes` a partir de `pos`, crescendo o buffer se preciso.
fn por(buf: &mut Vec<u8>, pos: usize, bytes: &[u8]) {
    if buf.len() < pos + bytes.len() {
        buf.resize(pos + bytes.len(), 0);
    }
    buf[pos..pos + bytes.len()].copy_from_slice(bytes);
}

#[test]
fn o_cabecalho_do_ir_confirma_que_sao_dois_bytes() {
    // Toda a aritmética deste arquivo depende disto. Se o cabeçalho deixar de ter 2
    // bytes, é melhor falhar aqui do que produzir deslocamentos silenciosamente errados.
    let ir = ir();
    let h = struct_do_ir(&ir, "SRV::C2S::cmd_header");
    assert_eq!(
        h["bytes"].as_u64(),
        Some(BYTES_DO_CABECALHO as u64),
        "o cabeçalho de subcomando mudou de tamanho no IR"
    );
}

#[test]
fn player_move_le_cada_campo_no_deslocamento_do_ir() {
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::player_move");
    assert_eq!(bytes_do_payload(s), PlayerMove::BYTES);

    // Valores distintos por campo: se dois campos trocarem de lugar, os números não
    // batem. Valores iguais deixariam a troca passar.
    let cur = Vec3::new(11.0, 12.0, 13.0);
    let prox = Vec3::new(21.0, 22.0, 23.0);

    let mut p = vec![0u8; PlayerMove::BYTES];
    let d = |campo| deslocamento(s, campo);

    let vec_bytes = |v: Vec3| {
        let mut b = Vec::with_capacity(12);
        b.extend_from_slice(&v.x.to_le_bytes());
        b.extend_from_slice(&v.y.to_le_bytes());
        b.extend_from_slice(&v.z.to_le_bytes());
        b
    };
    // `cur_pos` e `next_pos` moram dentro do `move_info`, cujos deslocamentos são
    // relativos ao pai — daí a soma com o deslocamento do próprio `info`.
    let info = struct_do_ir(&ir, "SRV::C2S::INFO::move_info");
    let base = d("info");
    let dentro = |campo: &str| -> usize {
        base + info["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == campo)
            .unwrap_or_else(|| panic!("`{campo}` não existe em move_info"))["offset"]
            .as_u64()
            .unwrap() as usize
    };

    por(&mut p, dentro("cur_pos"), &vec_bytes(cur));
    por(&mut p, dentro("next_pos"), &vec_bytes(prox));
    por(&mut p, dentro("use_time"), &1234u16.to_le_bytes());
    por(&mut p, dentro("speed"), &4321u16.to_le_bytes());
    por(&mut p, dentro("move_mode"), &[7u8]);
    por(&mut p, d("cmd_seq"), &999u16.to_le_bytes());

    let m = PlayerMove::ler(&p).expect("payload completo tem que decodificar");
    assert!(PlayerMove::completo(&p));

    assert_eq!(m.cur_pos, cur, "cur_pos saiu do deslocamento errado");
    assert_eq!(m.next_pos, prox, "next_pos saiu do deslocamento errado");
    assert_eq!(m.use_time, 1234, "use_time saiu do deslocamento errado");
    assert_eq!(m.speed, 4321, "speed saiu do deslocamento errado");
    assert_eq!(m.move_mode, 7, "move_mode saiu do deslocamento errado");
    assert_eq!(m.cmd_seq, 999, "cmd_seq saiu do deslocamento errado");
}

#[test]
fn logout_le_o_tipo_no_deslocamento_do_ir() {
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::logout");
    assert_eq!(bytes_do_payload(s), Logout::BYTES);

    let mut p = vec![0u8; Logout::BYTES];
    por(&mut p, deslocamento(s, "logout_type"), &1i32.to_le_bytes());

    assert_eq!(Logout::ler(&p).unwrap().logout_type, 1);
}

#[test]
fn select_target_le_o_id_no_deslocamento_do_ir() {
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::select_target");
    assert_eq!(bytes_do_payload(s), SelectTarget::BYTES);

    let mut p = vec![0u8; SelectTarget::BYTES];
    por(
        &mut p,
        deslocamento(s, "id"),
        &0x0BAD_F00Du32.to_le_bytes(),
    );

    assert_eq!(SelectTarget::ler(&p).unwrap().id, 0x0BAD_F00Du32 as i32);
}

#[test]
fn normal_attack_le_o_force_attack_no_deslocamento_do_ir() {
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::normal_attack");
    assert_eq!(bytes_do_payload(s), NormalAttack::BYTES);

    let mut p = vec![0u8; NormalAttack::BYTES];
    por(&mut p, deslocamento(s, "force_attack"), &[1u8]);
    assert_eq!(NormalAttack::ler(&p).unwrap().force_attack, 1);
}

#[test]
fn o_normal_attack_nao_tem_campo_de_alvo() {
    // Esta é a razão de o `SELECT_TARGET` ter migrado antes: o alvo não vem no pacote de
    // ataque, é o servidor que o guarda. Se algum dia o IR passar a trazer um id aqui, é
    // melhor descobrir por este teste do que por um jogador atacando o alvo errado.
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::normal_attack");
    let campos: Vec<&str> = s["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(campos, vec!["header", "force_attack"]);
}

#[test]
fn stop_move_le_cada_campo_no_deslocamento_do_ir() {
    // A ordem aqui difere do `PLAYER_MOVE` — `use_time` é o último, não o primeiro
    // depois das posições. Copiar um decodificador no outro é o erro que isto pega.
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::player_stop_move");
    assert_eq!(bytes_do_payload(s), StopMove::BYTES);

    let mut p = vec![0u8; StopMove::BYTES];
    let d = |campo| deslocamento(s, campo);
    for (i, v) in [7.0f32, 8.0, 9.0].iter().enumerate() {
        por(&mut p, d("pos") + i * 4, &v.to_le_bytes());
    }
    por(&mut p, d("speed"), &555u16.to_le_bytes());
    por(&mut p, d("dir"), &[3u8]);
    por(&mut p, d("move_mode"), &[4u8]);
    por(&mut p, d("cmd_seq"), &666u16.to_le_bytes());
    por(&mut p, d("use_time"), &777u16.to_le_bytes());

    let m = StopMove::ler(&p).unwrap();
    assert_eq!(m.pos, Vec3::new(7.0, 8.0, 9.0));
    assert_eq!(m.speed, 555, "speed saiu do deslocamento errado");
    assert_eq!(m.dir, 3, "dir saiu do deslocamento errado");
    assert_eq!(m.move_mode, 4, "move_mode saiu do deslocamento errado");
    assert_eq!(m.cmd_seq, 666, "cmd_seq saiu do deslocamento errado");
    assert_eq!(m.use_time, 777, "use_time saiu do deslocamento errado");
}

#[test]
fn os_comandos_de_par_de_slots_tem_todos_o_mesmo_layout() {
    // Cinco comandos de item compartilham o `ParDeSlots`. Se algum deixar de ter os dois
    // `unsigned char` nos deslocamentos 2 e 3, decodificá-los com a mesma struct passa a
    // ser errado — e é isto que avisa.
    let ir = ir();
    for nome in [
        "SRV::C2S::CMD::get_item_info",
        "SRV::C2S::CMD::exchange_inventory_item",
        "SRV::C2S::CMD::exchange_equip_item",
        "SRV::C2S::CMD::equip_item",
        "SRV::C2S::CMD::move_item_to_equipment",
    ] {
        let s = struct_do_ir(&ir, nome);
        assert_eq!(bytes_do_payload(s), ParDeSlots::BYTES, "{nome}: tamanho");

        let campos = s["fields"].as_array().unwrap();
        assert_eq!(campos.len(), 3, "{nome}: cabeçalho + dois campos");
        for (i, f) in campos.iter().skip(1).enumerate() {
            assert_eq!(
                f["offset"].as_u64(),
                Some(BYTES_DO_CABECALHO as u64 + i as u64),
                "{nome}: campo `{}` fora do lugar",
                f["name"]
            );
            assert_eq!(f["bytes"].as_u64(), Some(1), "{nome}: campo de 1 byte");
        }
    }

    // E a decodificação devolve os dois na ordem em que vieram.
    let p = ParDeSlots::ler(&[3, 9]).unwrap();
    assert_eq!((p.a, p.b), (3, 9));
}

#[test]
fn get_ivtr_detail_e_move_ivtr_item_batem_com_o_ir() {
    let ir = ir();

    let s = struct_do_ir(&ir, "SRV::C2S::CMD::get_inventory_detail");
    assert_eq!(bytes_do_payload(s), GetIvtrDetail::BYTES);
    let mut p = vec![0u8; GetIvtrDetail::BYTES];
    por(&mut p, deslocamento(s, "where"), &[1u8]);
    assert_eq!(GetIvtrDetail::ler(&p).unwrap().onde, 1);

    let s = struct_do_ir(&ir, "SRV::C2S::CMD::move_inventory_item");
    assert_eq!(bytes_do_payload(s), MoveIvtrItem::BYTES);
    let mut p = vec![0u8; MoveIvtrItem::BYTES];
    por(&mut p, deslocamento(s, "src"), &[2u8]);
    por(&mut p, deslocamento(s, "dest"), &[5u8]);
    // `amount` é `size_t`, que no i386 tem 4 bytes — não 8.
    assert_eq!(
        s["fields"].as_array().unwrap()[3]["bytes"].as_u64(),
        Some(4),
        "`amount` deixou de ter 4 bytes; `size_t` no alvo i386 é 32 bits"
    );
    por(&mut p, deslocamento(s, "amount"), &777u32.to_le_bytes());

    let m = MoveIvtrItem::ler(&p).unwrap();
    assert_eq!(m.src, 2, "src saiu do deslocamento errado");
    assert_eq!(m.dest, 5, "dest saiu do deslocamento errado");
    assert_eq!(m.amount, 777, "amount saiu do deslocamento errado");
}

#[test]
fn emote_action_le_a_acao_no_deslocamento_do_ir() {
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::emote_action");
    assert_eq!(bytes_do_payload(s), EmoteAction::BYTES);

    let mut p = vec![0u8; EmoteAction::BYTES];
    por(&mut p, deslocamento(s, "action"), &4242u16.to_le_bytes());
    assert_eq!(EmoteAction::ler(&p).unwrap().action, 4242);
}

#[test]
fn use_item_le_cada_campo_no_deslocamento_do_ir() {
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::use_item");
    assert_eq!(bytes_do_payload(s), UseItem::BYTES);

    // O `index` tem dois bytes. Um valor acima de 255 é o que pega quem o trunca para
    // `u8` — como o `gateway.rs` fazia, deixando os slots do fundo da bolsa inacessíveis.
    let mut p = vec![0u8; UseItem::BYTES];
    let d = |campo| deslocamento(s, campo);
    por(&mut p, d("where"), &[1u8]);
    por(&mut p, d("count"), &[3u8]);
    por(&mut p, d("index"), &300u16.to_le_bytes());
    por(&mut p, d("item_id"), &1796i32.to_le_bytes());

    let u = UseItem::ler(&p).unwrap();
    assert_eq!(u.onde, 1, "where saiu do deslocamento errado");
    assert_eq!(u.quantos, 3, "count saiu do deslocamento errado");
    assert_eq!(u.slot, 300, "o slot foi truncado para um byte");
    assert_eq!(u.item_id, 1796, "item_id saiu do deslocamento errado");
}

#[test]
fn cast_skill_le_os_alvos_depois_da_contagem() {
    // O `gateway.rs` lia o alvo em `data[7..11]`, que começa no `target_count` e engole
    // três bytes do primeiro alvo. A lista começa no deslocamento 8 do struct.
    let ir = ir();
    let s = struct_do_ir(&ir, "SRV::C2S::CMD::cast_skill");
    let d = |campo| deslocamento(s, campo);

    let mut p = vec![0u8; d("targets") + 8];
    por(&mut p, d("skill_id"), &4321i32.to_le_bytes());
    por(&mut p, d("force_attack"), &[1u8]);
    por(&mut p, d("target_count"), &[2u8]);
    por(&mut p, d("targets"), &777i32.to_le_bytes());
    por(&mut p, d("targets") + 4, &888i32.to_le_bytes());

    let c = CastSkill::ler(&p).unwrap();
    assert_eq!(c.skill_id, 4321, "skill_id saiu do deslocamento errado");
    assert_eq!(c.force_attack, 1);
    assert_eq!(
        c.alvos,
        vec![777, 888],
        "os alvos saíram do deslocamento errado — leu o `target_count` junto?"
    );
}

#[test]
fn uma_contagem_de_alvos_absurda_nao_derruba_o_servidor() {
    // `target_count` vem do cliente; sem teto, um valor grande faria o servidor montar
    // uma lista enorme antes de descobrir que o pacote acabou.
    let mut p = 1i32.to_le_bytes().to_vec();
    p.push(0);
    p.push(255); // target_count
    assert!(CastSkill::ler(&p).unwrap().alvos.is_empty());
}

#[test]
fn os_dois_comandos_de_habilidade_tem_o_mesmo_layout() {
    // O 41 e o 80 são tratados pelo mesmo decodificador. Se um deles mudar de forma, a
    // partilha deixa de ser correta.
    let ir = ir();
    let a = struct_do_ir(&ir, "SRV::C2S::CMD::cast_skill");
    let b = struct_do_ir(&ir, "SRV::C2S::CMD::cast_instant_skill");
    let campos = |s: &Value| -> Vec<(String, u64)> {
        s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["name"].as_str().unwrap().to_string(),
                    f["offset"].as_u64().unwrap(),
                )
            })
            .collect()
    };
    assert_eq!(campos(a), campos(b));
}

#[test]
fn os_ids_dos_comandos_sao_os_do_ir() {
    // Os números 0, 1 e 2 aparecem no `match` do servidor de mundo. Se o IR discordar,
    // o mundo estaria tratando o comando errado com a struct certa — que é pior do que
    // não tratar nada.
    let ir = ir();
    let c2s = ir["commands"]["c2s"].as_array().unwrap();
    let id_de = |nome: &str| -> i64 {
        c2s.iter()
            .find(|c| c["name"] == nome)
            .unwrap_or_else(|| panic!("{nome} não existe no IR"))["id"]
            .as_i64()
            .unwrap()
    };

    assert_eq!(id_de("PLAYER_MOVE"), pw_gs::comandos::ids::PLAYER_MOVE as i64);
    assert_eq!(id_de("LOGOUT"), pw_gs::comandos::ids::LOGOUT as i64);
    assert_eq!(
        id_de("SELECT_TARGET"),
        pw_gs::comandos::ids::SELECT_TARGET as i64
    );
    assert_eq!(
        id_de("NORMAL_ATTACK"),
        pw_gs::comandos::ids::NORMAL_ATTACK as i64
    );
    assert_eq!(id_de("STOP_MOVE"), pw_gs::comandos::ids::STOP_MOVE as i64);
    assert_eq!(id_de("UNSELECT"), pw_gs::comandos::ids::UNSELECT as i64);
    assert_eq!(
        id_de("REVIVE_VILLAGE"),
        pw_gs::comandos::ids::REVIVE_VILLAGE as i64
    );
    for (nome, id) in [
        ("GET_ITEM_INFO", pw_gs::comandos::ids::GET_ITEM_INFO),
        ("GET_IVTR_DETAIL", pw_gs::comandos::ids::GET_IVTR_DETAIL),
        ("EXG_IVTR_ITEM", pw_gs::comandos::ids::EXG_IVTR_ITEM),
        ("MOVE_IVTR_ITEM", pw_gs::comandos::ids::MOVE_IVTR_ITEM),
        ("EXG_EQUIP_ITEM", pw_gs::comandos::ids::EXG_EQUIP_ITEM),
        ("EQUIP_ITEM", pw_gs::comandos::ids::EQUIP_ITEM),
        ("MOVE_ITEM_TO_EQUIP", pw_gs::comandos::ids::MOVE_ITEM_TO_EQUIP),
        ("CANCEL_ACTION", pw_gs::comandos::ids::CANCEL_ACTION),
        ("SIT_DOWN", pw_gs::comandos::ids::SIT_DOWN),
        ("STAND_UP", pw_gs::comandos::ids::STAND_UP),
        ("EMOTE_ACTION", pw_gs::comandos::ids::EMOTE_ACTION),
        ("ENTER_SANCTUARY", pw_gs::comandos::ids::ENTER_SANCTUARY),
        ("SEVNPC_SERVE", pw_gs::comandos::ids::SEVNPC_SERVE),
        ("USE_ITEM", pw_gs::comandos::ids::USE_ITEM),
        ("CAST_SKILL", pw_gs::comandos::ids::CAST_SKILL),
        ("CAST_INSTANT_SKILL", pw_gs::comandos::ids::CAST_INSTANT_SKILL),
        ("TEAM_INVITE", pw_gs::comandos::ids::TEAM_INVITE),
        ("TEAM_AGREE_INVITE", pw_gs::comandos::ids::TEAM_AGREE_INVITE),
        ("TEAM_REJECT_INVITE", pw_gs::comandos::ids::TEAM_REJECT_INVITE),
        ("TEAM_LEAVE_PARTY", pw_gs::comandos::ids::TEAM_LEAVE_PARTY),
        ("SEVNPC_HELLO", pw_gs::comandos::ids::SEVNPC_HELLO),
        ("TASK_NOTIFY", pw_gs::comandos::ids::TASK_NOTIFY),
    ] {
        assert_eq!(id_de(nome), id as i64, "{nome}");
    }
}

/// `SEVNPC_HELLO` (35) e `TASK_NOTIFY` (49): o IR marca os dois como só cabeçalho
/// (`payload: null`), e está errado — confirmado no servidor 1.5.3
/// (`cgame/common/protocol.h`) e numa captura real de um servidor 1.2.6. Sem struct no
/// IR, a técnica "monta o payload a partir do IR" não se aplica aqui; estes testes
/// decodificam os bytes **exatamente como uma sessão real os mandou**, então uma correção
/// futura do extrator (`pw-rpcgen`) que passe a descrever um layout diferente derruba
/// este teste — o que é o comportamento certo, e não um teste que só concorda com o
/// código que testa.
#[test]
fn o_ir_marca_sevnpc_hello_e_task_notify_como_so_cabecalho_e_esta_errado() {
    let ir = ir();
    let c2s = ir["commands"]["c2s"].as_array().unwrap();
    let entrada = |nome: &str| -> &Value {
        c2s.iter()
            .find(|c| c["name"] == nome)
            .unwrap_or_else(|| panic!("{nome} não existe no IR"))
    };

    for nome in ["SEVNPC_HELLO", "TASK_NOTIFY"] {
        let e = entrada(nome);
        assert!(
            e["payload"].is_null() && e["struct"].is_null(),
            "{nome} passou a ter struct no IR — revisar `SevnpcHello`/`TaskNotify` em \
             `comandos.rs` contra o novo layout, e trocar este teste pela técnica normal \
             de montar o payload a partir do IR"
        );
    }
}

#[test]
fn sevnpc_hello_le_o_alvo_como_uma_sessao_real_mandou() {
    // Payload de uma captura real do 1.2.6 (docs/ESTADO_E_RETOMADA.md, sessão de
    // 2026-09-02): `SEVNPC_HELLO` de 6 bytes, cabeçalho `23 00` (35) seguido do mesmo
    // alvo que o `SELECT_TARGET` anterior mandou (`50 4c 00 80` = -2147464112).
    let payload = [0x50, 0x4c, 0x00, 0x80];
    let cmd = pw_gs::comandos::SevnpcHello::ler(&payload).expect("payload de 4 bytes válido");
    assert_eq!(cmd.target, -2147464112);

    // Curto demais para o `int` — não pode decodificar em vez de ler lixo.
    assert!(pw_gs::comandos::SevnpcHello::ler(&[0x50, 0x4c]).is_none());
}

#[test]
fn task_notify_le_size_e_o_prefixo_de_task_notify_base_como_uma_sessao_real_mandou() {
    // Mesma captura: `TASK_NOTIFY` de 9 bytes, cabeçalho `31 00` (49), `size=3` em
    // little-endian e três bytes de `buf` — `07 00 00`, que é `task_notify_base { reason:
    // 7, task: 0 }`.
    let payload = [0x03, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00];
    let cmd = pw_gs::comandos::TaskNotify::ler(&payload).expect("size e buf consistentes");
    assert_eq!(cmd.buf, vec![0x07, 0x00, 0x00]);
    assert_eq!(cmd.reason, Some(7));
    assert_eq!(cmd.task, Some(0));

    // `size` maior do que o payload realmente trouxe: o servidor original recusa
    // (`error_cmd(S2C::ERR_FATAL_ERR)` em `playercmd.cpp`), e nós também.
    let size_maior_que_o_buf = [0x05, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00];
    assert!(pw_gs::comandos::TaskNotify::ler(&size_maior_que_o_buf).is_none());
}
