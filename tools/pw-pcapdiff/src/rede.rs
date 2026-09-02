//! Do quadro de enlace até o payload TCP, e daí até um fluxo de bytes por sentido.
//!
//! # O que esta camada precisa acertar, e por quê
//!
//! O framing do GNET é `CompactUINT opcode` + `CompactUINT tamanho` + payload, sem
//! delimitador nem sincronismo. Isso quer dizer que **um byte perdido desalinha tudo o que
//! vem depois** — e desalinhado ele continua "lendo" comandos, com ids e tamanhos que não
//! existem. Um relatório assim é pior que nenhum: parece resposta.
//!
//! Por isso a remontagem é por número de sequência, e um buraco no fluxo é **reportado**,
//! nunca costurado. Se a captura perdeu pacote, o operador precisa saber disso antes de
//! olhar a tabela.

/// Endereço de uma ponta da conversa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ponta {
    pub ip: [u8; 4],
    pub porta: u16,
}

impl std::fmt::Display for Ponta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}:{}",
            self.ip[0], self.ip[1], self.ip[2], self.ip[3], self.porta
        )
    }
}

/// Um segmento TCP com dados.
pub struct Segmento {
    pub origem: Ponta,
    pub destino: Ponta,
    pub seq: u32,
    pub syn: bool,
    pub dados: Vec<u8>,
}

/// Extrai o segmento TCP de um quadro, se houver um.
///
/// Devolve `None` para tudo que não for IPv4/TCP com dados — ARP, DNS, IPv6, ACK puro.
pub fn segmento(tipo_de_enlace: u16, quadro: &[u8]) -> Option<Segmento> {
    let ip = match tipo_de_enlace {
        // Ethernet
        1 => {
            if quadro.len() < 14 {
                return None;
            }
            let mut i = 12;
            let mut ethertype = u16::from_be_bytes([quadro[i], quadro[i + 1]]);
            // VLAN 802.1Q: pula a etiqueta e lê o ethertype de verdade.
            while ethertype == 0x8100 || ethertype == 0x88A8 {
                i += 4;
                if i + 2 > quadro.len() {
                    return None;
                }
                ethertype = u16::from_be_bytes([quadro[i], quadro[i + 1]]);
            }
            if ethertype != 0x0800 {
                return None;
            }
            &quadro[i + 2..]
        }
        // Raw IP
        101 => quadro,
        // Linux cooked capture v1 — o que sai de `tcpdump -i any` até o libpcap 1.10.
        // O tipo de protocolo fica no fim do cabeçalho de 16 bytes.
        113 => {
            if quadro.len() < 16 || u16::from_be_bytes([quadro[14], quadro[15]]) != 0x0800 {
                return None;
            }
            &quadro[16..]
        }
        // Linux cooked capture v2 — o que o libpcap 1.11+ passou a produzir para
        // `tcpdump -i any`. Cabeçalho de 20 bytes e, ao contrário do v1, o tipo de
        // protocolo vem **primeiro**.
        //
        // Está aqui por precaução, e não por necessidade: a captura combinada usa uma
        // interface nomeada, que dá Ethernet. Mas `-i any` é o primeiro fallback de quem
        // estiver com pressa, e um formato não reconhecido só apareceria **depois** da
        // sessão gravada — quando refazer custa uma tarde do Murillo em vez de dez linhas
        // minhas.
        276 => {
            if quadro.len() < 20 || u16::from_be_bytes([quadro[0], quadro[1]]) != 0x0800 {
                return None;
            }
            &quadro[20..]
        }
        // Loopback: 4 bytes de família de protocolo, em ordem do host.
        0 => {
            if quadro.len() < 4 {
                return None;
            }
            &quadro[4..]
        }
        _ => return None,
    };

    if ip.len() < 20 || (ip[0] >> 4) != 4 {
        return None;
    }
    // Protocolo 6 = TCP.
    if ip[9] != 6 {
        return None;
    }
    let ihl = ((ip[0] & 0x0F) as usize) * 4;
    // O tamanho total do datagrama manda, e não o tamanho do quadro: uma captura com
    // padding de Ethernet traria bytes de enchimento como se fossem dados.
    let total = u16::from_be_bytes([ip[2], ip[3]]) as usize;
    if ihl < 20 || total < ihl || total > ip.len() {
        return None;
    }

    let origem_ip = [ip[12], ip[13], ip[14], ip[15]];
    let destino_ip = [ip[16], ip[17], ip[18], ip[19]];
    let tcp = &ip[ihl..total];
    if tcp.len() < 20 {
        return None;
    }

    let porta_origem = u16::from_be_bytes([tcp[0], tcp[1]]);
    let porta_destino = u16::from_be_bytes([tcp[2], tcp[3]]);
    let seq = u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]);
    let offset = ((tcp[12] >> 4) as usize) * 4;
    let syn = (tcp[13] & 0x02) != 0;
    if offset < 20 || offset > tcp.len() {
        return None;
    }

    Some(Segmento {
        origem: Ponta {
            ip: origem_ip,
            porta: porta_origem,
        },
        destino: Ponta {
            ip: destino_ip,
            porta: porta_destino,
        },
        seq,
        syn,
        dados: tcp[offset..].to_vec(),
    })
}

/// Um sentido de uma conexão, remontado.
pub struct Fluxo {
    pub origem: Ponta,
    pub destino: Ponta,
    pub bytes: Vec<u8>,
    /// Quantas vezes faltou um pedaço. Qualquer valor acima de zero invalida a leitura
    /// dos comandos a partir dali — o relatório precisa dizer isso em voz alta.
    pub buracos: usize,
}

/// Remonta os segmentos em fluxos, um por sentido de cada conexão.
///
/// # Como a ordem é resolvida
///
/// Guardamos os segmentos por número de sequência e emitimos em ordem, o que resolve
/// reordenação e retransmissão de uma vez. O ponto de partida é o `SYN` quando ele
/// aparece na captura; quando não aparece — captura começada com a sessão já aberta —
/// usamos o menor `seq` visto, o que é o começo do que **temos**, e não do que houve.
pub fn remontar(segmentos: Vec<Segmento>) -> Vec<Fluxo> {
    use std::collections::BTreeMap;

    // (origem, destino) → (seq inicial, seq → dados)
    type Chave = (Ponta, Ponta);
    type Pedacos = BTreeMap<u32, Vec<u8>>;
    let mut por_sentido: BTreeMap<Chave, (Option<u32>, Pedacos)> = BTreeMap::new();

    for s in segmentos {
        let e = por_sentido
            .entry((s.origem, s.destino))
            .or_insert((None, BTreeMap::new()));
        if s.syn {
            // Depois do SYN os dados começam em seq+1.
            e.0 = Some(s.seq.wrapping_add(1));
        }
        if s.dados.is_empty() {
            continue;
        }
        // Retransmissão: fica com o que já tínhamos. Segmentos iguais são iguais; se
        // divergirem, o primeiro é o que o outro lado provavelmente aceitou.
        e.1.entry(s.seq).or_insert(s.dados);
    }

    let mut fluxos = Vec::new();
    for ((origem, destino), (inicio, pedacos)) in por_sentido {
        if pedacos.is_empty() {
            continue;
        }
        let mut esperado = inicio.unwrap_or_else(|| *pedacos.keys().next().unwrap());
        let mut bytes = Vec::new();
        let mut buracos = 0usize;

        for (seq, dados) in pedacos {
            if seq == esperado {
                bytes.extend_from_slice(&dados);
                esperado = esperado.wrapping_add(dados.len() as u32);
            } else if seq.wrapping_sub(esperado) > (u32::MAX / 2) {
                // Sequência anterior ao que já temos: sobreposição de retransmissão. A
                // parte nova, se houver, é a cauda.
                let ja_temos = esperado.wrapping_sub(seq) as usize;
                if ja_temos < dados.len() {
                    bytes.extend_from_slice(&dados[ja_temos..]);
                    esperado = seq.wrapping_add(dados.len() as u32);
                }
            } else {
                // Buraco de verdade: perdemos bytes que o outro lado recebeu.
                buracos += 1;
                bytes.extend_from_slice(&dados);
                esperado = seq.wrapping_add(dados.len() as u32);
            }
        }

        fluxos.push(Fluxo {
            origem,
            destino,
            bytes,
            buracos,
        });
    }

    fluxos
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Um IPv4/TCP mínimo com `carga` bytes de dados, para pendurar em qualquer enlace.
    fn ip_tcp(carga: &[u8]) -> Vec<u8> {
        let mut tcp = vec![
            0x71, 0x48, // porta origem 29000
            0xC0, 0x00, // porta destino 49152
            0, 0, 0x03, 0xE8, // seq 1000
            0, 0, 0, 0, // ack
            5 << 4, 0x18, // offset 20 bytes, PSH+ACK
            0xFF, 0xFF, 0, 0, 0, 0,
        ];
        tcp.extend_from_slice(carga);

        let total = 20 + tcp.len();
        let mut ip = vec![0x45, 0];
        ip.extend_from_slice(&(total as u16).to_be_bytes());
        ip.extend_from_slice(&[0, 0, 0x40, 0, 64, 6, 0, 0]);
        ip.extend_from_slice(&[192, 168, 1, 200]);
        ip.extend_from_slice(&[192, 168, 1, 10]);
        ip.extend_from_slice(&tcp);
        ip
    }

    const CARGA: &[u8] = b"pwus";

    fn confere(tipo: u16, quadro: &[u8]) {
        let s = segmento(tipo, quadro)
            .unwrap_or_else(|| panic!("enlace {tipo} não foi reconhecido"));
        assert_eq!(s.origem.porta, 29000, "enlace {tipo}: porta errada");
        assert_eq!(s.origem.ip, [192, 168, 1, 200], "enlace {tipo}: IP errado");
        assert_eq!(s.dados, CARGA, "enlace {tipo}: payload errado");
    }

    #[test]
    fn ethernet() {
        let mut q = vec![0u8; 12];
        q.extend_from_slice(&[0x08, 0x00]);
        q.extend_from_slice(&ip_tcp(CARGA));
        confere(1, &q);
    }

    #[test]
    fn linux_cooked_v1_do_tcpdump_i_any() {
        // 16 bytes de cabeçalho, tipo de protocolo no fim.
        let mut q = vec![0u8; 14];
        q.extend_from_slice(&[0x08, 0x00]);
        q.extend_from_slice(&ip_tcp(CARGA));
        confere(113, &q);
    }

    #[test]
    fn linux_cooked_v2_do_libpcap_novo() {
        // 20 bytes de cabeçalho, tipo de protocolo no **começo** — é a diferença que faz
        // o v1 e o v2 não serem intercambiáveis.
        let mut q = vec![0x08, 0x00];
        q.extend_from_slice(&[0u8; 18]);
        q.extend_from_slice(&ip_tcp(CARGA));
        confere(276, &q);
    }

    #[test]
    fn a_etiqueta_de_vlan_nao_esconde_o_pacote() {
        let mut q = vec![0u8; 12];
        q.extend_from_slice(&[0x81, 0x00, 0x00, 0x64]); // VLAN 100
        q.extend_from_slice(&[0x08, 0x00]);
        q.extend_from_slice(&ip_tcp(CARGA));
        confere(1, &q);
    }

    #[test]
    fn o_enchimento_de_ethernet_nao_entra_como_dado() {
        // Quadro menor que 60 bytes é preenchido com zeros pela placa. Se a leitura
        // usasse o tamanho do quadro em vez do `total` do IP, esses zeros virariam bytes
        // de comando — e o framing GNET desalinharia logo depois.
        let mut q = vec![0u8; 12];
        q.extend_from_slice(&[0x08, 0x00]);
        q.extend_from_slice(&ip_tcp(CARGA));
        q.extend_from_slice(&[0u8; 20]); // enchimento
        let s = segmento(1, &q).expect("não leu");
        assert_eq!(s.dados, CARGA, "o enchimento entrou como dado");
    }

    #[test]
    fn ipv6_e_arp_sao_ignorados_em_vez_de_lidos_torto() {
        let mut arp = vec![0u8; 12];
        arp.extend_from_slice(&[0x08, 0x06]);
        arp.extend_from_slice(&[0u8; 28]);
        assert!(segmento(1, &arp).is_none());

        let mut v6 = vec![0u8; 12];
        v6.extend_from_slice(&[0x86, 0xDD]);
        v6.extend_from_slice(&[0u8; 40]);
        assert!(segmento(1, &v6).is_none());
    }
}
