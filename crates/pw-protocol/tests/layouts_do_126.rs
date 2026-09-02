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

use pw_protocol::{GameVersion, PorVersao};

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
fn escrever(p: &PorVersao, id: u16) -> Vec<u8> {
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
        206 => p.inst_data_checkout(1, 0x46b1_a9ac, 0x46b1_a9ac, 0x47e8_b6ff, None).data,
        outro => panic!("o comando {outro} está na tabela e não tem chamada aqui"),
    };
    d
}

#[test]
fn o_126_escreve_o_tamanho_que_o_servidor_de_verdade_escreveu() {
    let p = PorVersao::new(GameVersion::V1_2_6);
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
    let p = PorVersao::new(GameVersion::V1_5_3);
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
    let a = PorVersao::new(GameVersion::V1_2_6);
    let b = PorVersao::new(GameVersion::V1_5_3);

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
    let quatro_oito = PorVersao::new(GameVersion::V1_4_8);
    let cinco_tres = PorVersao::new(GameVersion::V1_5_3);

    for (_, id, _, _, _) in MEDIDO {
        assert_eq!(escrever(&quatro_oito, *id), escrever(&cinco_tres, *id));
    }
}

#[test]
fn o_receive_exp_do_126_cabe_em_16_bits_sem_estourar() {
    // Um abate que desse mais de 65.535 de experiência truncaria para um número pequeno e
    // aleatório: o jogador veria "ganhou 3 de exp" ao matar um chefe. O teto é errado por
    // menos.
    let p = PorVersao::new(GameVersion::V1_2_6);
    let d = p.receive_exp(70_000, -5).data;
    assert_eq!(d.len(), 2 + 4);
    assert_eq!(u16::from_le_bytes([d[2], d[3]]), u16::MAX, "não saturou o exp");
    assert_eq!(u16::from_le_bytes([d[4], d[5]]), 0, "exp negativa devia virar zero");
}

#[test]
fn o_hp_do_npc_info_00_do_126_fica_onde_a_captura_mostrou() {
    // A captura mostra o `iHP` caindo (29 → 22 → 17 → 11 → 2) enquanto o `iMaxHP` fica em
    // 29. É o que fixa a **ordem** dos dois campos, que o tamanho sozinho não fixaria.
    let p = PorVersao::new(GameVersion::V1_2_6);
    let d = p.npc_info_00(900_001, 11, 29, 0).data;
    assert_eq!(i32::from_le_bytes([d[2], d[3], d[4], d[5]]), 900_001, "idNPC");
    assert_eq!(i32::from_le_bytes([d[6], d[7], d[8], d[9]]), 11, "iHP fora do lugar");
    assert_eq!(i32::from_le_bytes([d[10], d[11], d[12], d[13]]), 29, "iMaxHP fora do lugar");
}
