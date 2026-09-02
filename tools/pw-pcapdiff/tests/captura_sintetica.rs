//! A ferramenta inteira, contra uma captura montada aqui — antes de existir uma de verdade.
//!
//! # Por que provar assim
//!
//! O `pw-pcapdiff` vai ser usado para **decidir** o que é verdade sobre o protocolo do
//! 1.2.6. Se ele medir errado, a conclusão sai errada e entra no IR como fato, que é o
//! pior desfecho possível para este projeto.
//!
//! Só que a captura de verdade ainda não existe, e quando existir não haverá gabarito: é
//! justamente o que não sabemos. Então o gabarito tem que vir de onde já sabemos a
//! resposta — **os nossos próprios codificadores**. `npc_info_00` escreve um número
//! conhecido de bytes; se a ferramenta ler outro, é ela que está errada.
//!
//! Os casos difíceis são os da rede, não os do formato: um quadro GNET partido entre dois
//! segmentos TCP, segmentos fora de ordem, retransmissão. São eles que separam "leu o
//! arquivo" de "remontou a conversa".

use pw_protocol::{OctetsStream, S2CGamedataSend};
use std::process::Command;

const PORTA: u16 = 29000;
const SERVIDOR: [u8; 4] = [10, 0, 0, 5];
const CLIENTE: [u8; 4] = [10, 0, 0, 9];

/// Envelope GNET: `CompactUINT opcode` + `CompactUINT tamanho` + corpo.
fn envelope(opcode: u32, corpo: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    compact(&mut v, opcode);
    compact(&mut v, corpo.len() as u32);
    v.extend_from_slice(corpo);
    v
}

/// O corpo de um `GamedataSend` (34): o subcomando dentro de um `Octets`.
///
/// **O `write_octets` do `pw-protocol` é usado de propósito**, e não um comprimento
/// montado à mão. A primeira versão deste teste escrevia o subcomando cru, igual ao que a
/// ferramenta então lia — gabarito e código concordando no mesmo erro. Usar o mesmo
/// caminho que a produção usa é o que impede o teste de herdar a suposição que ele deveria
/// estar checando.
fn corpo_do_cliente(subcomando: &[u8]) -> Vec<u8> {
    let mut os = OctetsStream::new();
    os.write_octets(subcomando);
    os.into_bytes().to_vec()
}

/// O corpo de um `S2CGamedataSend` (74): `int roleid` + `unsigned int localsid` + `Octets`.
fn corpo_interno_unicast(roleid: i32, localsid: u32, subcomando: &[u8]) -> Vec<u8> {
    let mut os = OctetsStream::new();
    os.write_i32(roleid);
    os.write_u32(localsid);
    os.write_octets(subcomando);
    os.into_bytes().to_vec()
}

/// O corpo de um `S2CMulticast` (77): `Octets data` e depois a lista de jogadores.
fn corpo_interno_multicast(subcomando: &[u8], jogadores: &[i32]) -> Vec<u8> {
    let mut os = OctetsStream::new();
    os.write_octets(subcomando);
    os.write_compact_uint(jogadores.len() as u32);
    for j in jogadores {
        os.write_i32(*j);
    }
    os.into_bytes().to_vec()
}

fn compact(v: &mut Vec<u8>, n: u32) {
    if n < 0x40 {
        v.push(n as u8);
    } else if n < 0x4000 {
        v.push(0x80 | ((n >> 8) as u8 & 0x3F));
        v.push((n & 0xFF) as u8);
    } else {
        v.push(0xC0 | ((n >> 24) as u8 & 0x1F));
        v.push(((n >> 16) & 0xFF) as u8);
        v.push(((n >> 8) & 0xFF) as u8);
        v.push((n & 0xFF) as u8);
    }
}

/// Um quadro Ethernet/IPv4/TCP com o payload dado.
fn quadro(origem: [u8; 4], po: u16, destino: [u8; 4], pd: u16, seq: u32, dados: &[u8]) -> Vec<u8> {
    let mut tcp = Vec::new();
    tcp.extend_from_slice(&po.to_be_bytes());
    tcp.extend_from_slice(&pd.to_be_bytes());
    tcp.extend_from_slice(&seq.to_be_bytes());
    tcp.extend_from_slice(&0u32.to_be_bytes()); // ack
    tcp.push(5 << 4); // data offset = 20 bytes
    tcp.push(0x18); // PSH+ACK
    tcp.extend_from_slice(&[0xFF, 0xFF]); // janela
    tcp.extend_from_slice(&[0, 0]); // checksum: a ferramenta não confere, e não deve
    tcp.extend_from_slice(&[0, 0]); // urgent
    tcp.extend_from_slice(dados);

    let total = 20 + tcp.len();
    let mut ip = Vec::new();
    ip.push(0x45);
    ip.push(0);
    ip.extend_from_slice(&(total as u16).to_be_bytes());
    ip.extend_from_slice(&[0, 0, 0x40, 0, 64, 6, 0, 0]);
    ip.extend_from_slice(&origem);
    ip.extend_from_slice(&destino);
    ip.extend_from_slice(&tcp);

    let mut eth = vec![0u8; 12];
    eth.extend_from_slice(&[0x08, 0x00]);
    eth.extend_from_slice(&ip);
    eth
}

/// Um pcap clássico com os quadros dados.
fn pcap(quadros: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&0xa1b2c3d4u32.to_be_bytes());
    v.extend_from_slice(&2u16.to_be_bytes());
    v.extend_from_slice(&4u16.to_be_bytes());
    v.extend_from_slice(&0u32.to_be_bytes());
    v.extend_from_slice(&0u32.to_be_bytes());
    v.extend_from_slice(&65535u32.to_be_bytes());
    v.extend_from_slice(&1u32.to_be_bytes()); // LinkType 1 = Ethernet
    for (i, q) in quadros.iter().enumerate() {
        v.extend_from_slice(&(i as u32).to_be_bytes());
        v.extend_from_slice(&0u32.to_be_bytes());
        v.extend_from_slice(&(q.len() as u32).to_be_bytes());
        v.extend_from_slice(&(q.len() as u32).to_be_bytes());
        v.extend_from_slice(q);
    }
    v
}

/// Um pcapng com os mesmos quadros — o formato que o Wireshark salva por padrão.
fn pcapng(quadros: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    // Section Header Block
    v.extend_from_slice(&0x0A0D0D0Au32.to_le_bytes());
    v.extend_from_slice(&28u32.to_le_bytes());
    v.extend_from_slice(&0x1A2B3C4Du32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&(-1i64).to_le_bytes());
    v.extend_from_slice(&28u32.to_le_bytes());
    // Interface Description Block
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&20u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // LinkType Ethernet
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&65535u32.to_le_bytes());
    v.extend_from_slice(&20u32.to_le_bytes());
    // Enhanced Packet Blocks
    for q in quadros {
        let preenchimento = (4 - (q.len() % 4)) % 4;
        let tamanho = 32 + q.len() + preenchimento;
        v.extend_from_slice(&6u32.to_le_bytes());
        v.extend_from_slice(&(tamanho as u32).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes()); // interface
        v.extend_from_slice(&0u32.to_le_bytes()); // timestamp alto
        v.extend_from_slice(&0u32.to_le_bytes()); // timestamp baixo
        v.extend_from_slice(&(q.len() as u32).to_le_bytes());
        v.extend_from_slice(&(q.len() as u32).to_le_bytes());
        v.extend_from_slice(q);
        v.extend(std::iter::repeat_n(0u8, preenchimento));
        v.extend_from_slice(&(tamanho as u32).to_le_bytes());
    }
    v
}

fn rodar(arquivo: &str) -> String {
    let raiz = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let exe = env!("CARGO_BIN_EXE_pw-pcapdiff");
    let saida = Command::new(exe)
        .arg(arquivo)
        .arg("--ir")
        .arg(format!("{raiz}/specs/protocol/gamedata_153.json"))
        .output()
        .expect("não consegui rodar o pw-pcapdiff");
    assert!(
        saida.status.success(),
        "o pw-pcapdiff falhou: {}",
        String::from_utf8_lossy(&saida.stderr)
    );
    String::from_utf8_lossy(&saida.stdout).to_string()
}

fn temporario(nome: &str, dados: &[u8]) -> String {
    let caminho = std::env::temp_dir().join(nome);
    std::fs::write(&caminho, dados).expect("escrever a captura de teste");
    caminho.to_string_lossy().to_string()
}

/// Os comandos que a captura sintética carrega, com o tamanho que os **nossos**
/// codificadores produzem — que é o gabarito.
fn comandos() -> Vec<(u16, Vec<u8>, usize)> {
    let npc = S2CGamedataSend::npc_info_00(900_001, 55, 480, 0).data;
    let cash = S2CGamedataSend::player_cash(4242).data;
    let ataque = S2CGamedataSend::host_attack_result(900_001, 37, 0, 0).data;
    // Tamanho variável: dois grupos de tamanhos diferentes.
    let grupo1 = S2CGamedataSend::team_member_data(7, &[Default::default(); 2]).data;
    let grupo2 = S2CGamedataSend::team_member_data(7, &[Default::default(); 3]).data;

    vec![
        (33, npc.clone(), npc.len() - 2),
        (253, cash.clone(), cash.len() - 2),
        (24, ataque.clone(), ataque.len() - 2),
        (64, grupo1.clone(), grupo1.len() - 2),
        (64, grupo2.clone(), grupo2.len() - 2),
    ]
}

/// Os quadros da conversa, com o fluxo do servidor partido em segmentos de `mtu` bytes.
///
/// Partir em pedaços pequenos é de propósito: um quadro GNET **tem** que atravessar a
/// fronteira entre dois segmentos TCP, porque é isso que acontece na rede de verdade e é
/// onde uma remontagem ingênua quebra.
fn conversa(mtu: usize) -> Vec<Vec<u8>> {
    let mut fluxo = Vec::new();
    for (_, dados, _) in comandos() {
        fluxo.extend(envelope(34, &corpo_do_cliente(&dados)));
    }

    let mut quadros = Vec::new();
    // O SYN, para que a remontagem tenha o ponto de partida.
    quadros.push(quadro(SERVIDOR, PORTA, CLIENTE, 50000, 999, &[]));
    let mut seq = 1000u32;
    for pedaco in fluxo.chunks(mtu) {
        quadros.push(quadro(SERVIDOR, PORTA, CLIENTE, 50000, seq, pedaco));
        seq = seq.wrapping_add(pedaco.len() as u32);
    }
    quadros
}

#[test]
fn mede_cada_comando_com_o_tamanho_que_o_codificador_escreveu() {
    let arquivo = temporario("pwus_sintetica.pcap", &pcap(&conversa(40)));
    let saida = rodar(&arquivo);

    for (id, _, bytes) in comandos() {
        if id == 64 {
            continue; // tamanho variável, conferido no seu próprio teste
        }
        let linha = saida
            .lines()
            .find(|l| l.starts_with(&format!("| {id} |")))
            .unwrap_or_else(|| panic!("o comando {id} não apareceu no relatório:\n{saida}"));
        assert!(
            linha.contains(&format!("{bytes}×1")),
            "o comando {id} devia ter {bytes} bytes; a linha saiu: {linha}"
        );
        assert!(
            linha.contains("igual ao 1.5.3"),
            "o comando {id} devia bater com o IR (os codificadores foram corrigidos \
             para o layout do 1.5.3); a linha saiu: {linha}"
        );
    }
}

#[test]
fn um_quadro_partido_entre_segmentos_e_remontado() {
    // MTU de 7 bytes garante que praticamente todo quadro GNET seja partido.
    let miudo = rodar(&temporario("pwus_miudo.pcap", &pcap(&conversa(7))));
    let graudo = rodar(&temporario("pwus_graudo.pcap", &pcap(&conversa(4096))));

    let extrair = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| l.starts_with("| ") && !l.contains("---"))
            .map(|l| l.to_string())
            .collect()
    };
    assert_eq!(
        extrair(&miudo),
        extrair(&graudo),
        "a segmentação TCP mudou o resultado — a remontagem não está funcionando"
    );
}

#[test]
fn segmentos_fora_de_ordem_e_retransmitidos_dao_o_mesmo_resultado() {
    let certos = conversa(40);
    let mut bagunçados = certos.clone();
    // Inverte a ordem de chegada dos dados, mantendo o SYN na frente, e repete um
    // segmento — os dois casos que a rede de verdade produz sozinha.
    let cauda = bagunçados.split_off(1);
    let repetido = cauda[1].clone();
    bagunçados.extend(cauda.into_iter().rev());
    bagunçados.push(repetido);

    let a = rodar(&temporario("pwus_ordem_ok.pcap", &pcap(&certos)));
    let b = rodar(&temporario("pwus_ordem_ruim.pcap", &pcap(&bagunçados)));

    let extrair = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| l.starts_with("| ") && !l.contains("---"))
            .map(|l| l.to_string())
            .collect()
    };
    assert_eq!(
        extrair(&a),
        extrair(&b),
        "reordenação ou retransmissão mudou o resultado"
    );
}

#[test]
fn o_pcapng_do_wireshark_da_o_mesmo_que_o_pcap() {
    // O Wireshark salva em pcapng por padrão. Se só o pcap funcionasse, o erro só
    // apareceria com a captura de verdade já na mão.
    let a = rodar(&temporario("pwus_fmt.pcap", &pcap(&conversa(40))));
    let b = rodar(&temporario("pwus_fmt.pcapng", &pcapng(&conversa(40))));

    let extrair = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| l.starts_with("| ") && !l.contains("---"))
            .map(|l| l.to_string())
            .collect()
    };
    assert_eq!(extrair(&a), extrair(&b), "pcapng e pcap divergiram");
}

#[test]
fn um_comando_com_tamanho_menor_e_reportado_como_diferenca_medida() {
    // **É este o caso que a captura do 1.2.6 vai produzir**, se a hipótese estiver certa:
    // o mesmo comando, com menos bytes. Aqui ele é fabricado de propósito — um
    // `NPC_INFO_00` de 12 bytes, que é o layout sem o `iTargetID`.
    let mut fluxo = Vec::new();
    let mut corpo = 33u16.to_le_bytes().to_vec();
    corpo.extend_from_slice(&[0u8; 12]);
    fluxo.extend(envelope(34, &corpo_do_cliente(&corpo)));

    let quadros = vec![
        quadro(SERVIDOR, PORTA, CLIENTE, 50000, 999, &[]),
        quadro(SERVIDOR, PORTA, CLIENTE, 50000, 1000, &fluxo),
    ];
    let saida = rodar(&temporario("pwus_menor.pcap", &pcap(&quadros)));

    let linha = saida
        .lines()
        .find(|l| l.starts_with("| 33 |"))
        .unwrap_or_else(|| panic!("o comando 33 não apareceu:\n{saida}"));
    assert!(
        linha.contains("difere: 12 bytes (-4)"),
        "a diferença devia ser reportada com o tamanho e o delta; saiu: {linha}"
    );
}

#[test]
fn perda_de_pacote_vira_aviso_e_nao_tabela_silenciosa() {
    // Sem sincronismo no framing, um buraco desalinha tudo o que vem depois. O perigo
    // não é a ferramenta falhar: é ela **continuar** e produzir uma tabela plausível.
    let certos = conversa(40);
    let mut furados = certos.clone();
    furados.remove(2); // some com um segmento do meio

    let saida = rodar(&temporario("pwus_furado.pcap", &pcap(&furados)));
    assert!(
        saida.contains("## Avisos") && saida.contains("buraco"),
        "a captura tinha perda de pacote e o relatório não avisou:\n{saida}"
    );
}

#[test]
fn captura_sem_a_porta_pedida_falha_em_vez_de_dar_tabela_vazia() {
    let arquivo = temporario("pwus_porta.pcap", &pcap(&conversa(40)));
    let exe = env!("CARGO_BIN_EXE_pw-pcapdiff");
    let saida = Command::new(exe)
        .arg(&arquivo)
        .arg("--porta")
        .arg("1234")
        .output()
        .expect("rodar");
    assert!(
        !saida.status.success(),
        "porta errada devia falhar, e não devolver uma tabela vazia como se estivesse tudo bem"
    );
    assert!(String::from_utf8_lossy(&saida.stderr).contains("nenhum segmento TCP"));
}

/// Uma conversa no elo **interno**, com os três envelopes que ele usa.
fn conversa_interna() -> Vec<Vec<u8>> {
    let mut fluxo = Vec::new();
    for (i, (_, dados, _)) in comandos().iter().enumerate() {
        // Alterna unicast e multicast, que é como o servidor de verdade faz: o que é só
        // para um jogador vai no 74, o que a vizinhança inteira vê vai no 77.
        if i % 2 == 0 {
            fluxo.extend(envelope(74, &corpo_interno_unicast(1024, 0xABCD, dados)));
        } else {
            fluxo.extend(envelope(77, &corpo_interno_multicast(dados, &[1024, 1025])));
        }
    }

    let mut quadros = vec![quadro(SERVIDOR, 29301, CLIENTE, 41868, 999, &[])];
    let mut seq = 1000u32;
    for pedaco in fluxo.chunks(37) {
        quadros.push(quadro(SERVIDOR, 29301, CLIENTE, 41868, seq, pedaco));
        seq = seq.wrapping_add(pedaco.len() as u32);
    }
    quadros
}

fn rodar_interno(arquivo: &str) -> String {
    let raiz = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let exe = env!("CARGO_BIN_EXE_pw-pcapdiff");
    let saida = Command::new(exe)
        .arg(arquivo)
        .arg("--ir")
        .arg(format!("{raiz}/specs/protocol/gamedata_153.json"))
        .arg("--interno")
        .output()
        .expect("rodar");
    assert!(
        saida.status.success(),
        "falhou: {}",
        String::from_utf8_lossy(&saida.stderr)
    );
    String::from_utf8_lossy(&saida.stdout).to_string()
}

#[test]
fn o_elo_interno_da_os_mesmos_tamanhos_que_o_elo_com_o_cliente() {
    // O mesmo subcomando, embrulhado de três jeitos diferentes, tem que sair com o mesmo
    // tamanho. É esta igualdade que autoriza medir o 1.2.6 pelo tráfego interno em vez do
    // elo com o cliente, que é cifrado (item 54).
    let cliente = rodar(&temporario("pwus_i_cli.pcap", &pcap(&conversa(40))));
    let interno = rodar_interno(&temporario("pwus_i_int.pcap", &pcap(&conversa_interna())));

    let linhas = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| l.starts_with("| ") && !l.contains("---") && !l.contains("comando"))
            .map(|l| {
                // Só id, nome e tamanho: a contagem de ocorrências difere de propósito.
                let c: Vec<&str> = l.split('|').collect();
                format!("{}|{}|{}", c[1].trim(), c[2].trim(), c[4].trim())
            })
            .collect()
    };
    assert_eq!(
        linhas(&cliente),
        linhas(&interno),
        "o mesmo comando mediu diferente conforme o envelope"
    );
}

#[test]
fn o_octets_do_envelope_e_desembrulhado() {
    // Sem desembrulhar o `Octets`, os dois primeiros bytes lidos são o comprimento e
    // metade do id — o comando sai com id errado **e** tamanho errado. Este teste fixa o
    // valor certo para que a regressão apareça como id trocado, e não como número feio.
    let saida = rodar(&temporario("pwus_oct.pcap", &pcap(&conversa(4096))));
    assert!(
        saida.contains("| 253 | PLAYER_CASH | 4×1"),
        "o PLAYER_CASH devia sair com 4 bytes de payload:\n{saida}"
    );
    assert!(
        !saida.contains("não existe no IR"),
        "apareceu comando fora do IR — sinal clássico de envelope lido torto:\n{saida}"
    );
}
