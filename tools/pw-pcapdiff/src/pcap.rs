//! Leitura de arquivos de captura, nos dois formatos que o Wireshark produz.
//!
//! # Por que ler os dois
//!
//! O Wireshark salva em **pcapng** por padrão desde a versão 1.8. Pedir "salve como pcap
//! clássico" é um passo a mais no roteiro de captura, e um passo a mais é um passo que se
//! esquece — com a captura já feita, o erro só aparece do lado de cá. Ler os dois custa
//! umas cinquenta linhas e elimina a ida e volta.
//!
//! Só o que interessa é extraído: o tipo de enlace e os bytes de cada quadro.

/// Um quadro de enlace, como saiu da captura.
pub struct Quadro {
    /// O `LinkType` do bloco/cabeçalho que descreve este quadro.
    pub tipo_de_enlace: u16,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum ErroDePcap {
    NaoReconhecido,
    Truncado,
}

impl std::fmt::Display for ErroDePcap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NaoReconhecido => write!(
                f,
                "não é um pcap nem um pcapng (número mágico desconhecido). \
                 No Wireshark: Arquivo → Salvar como, e escolha pcapng ou pcap."
            ),
            Self::Truncado => write!(f, "o arquivo termina no meio de um bloco"),
        }
    }
}

/// Lê a captura inteira e devolve os quadros na ordem em que foram gravados.
pub fn ler(dados: &[u8]) -> Result<Vec<Quadro>, ErroDePcap> {
    if dados.len() < 4 {
        return Err(ErroDePcap::Truncado);
    }
    match [dados[0], dados[1], dados[2], dados[3]] {
        // pcap clássico, nas quatro combinações de ordem de bytes e resolução.
        [0xd4, 0xc3, 0xb2, 0xa1] | [0x4d, 0x3c, 0xb2, 0xa1] => classico(dados, false),
        [0xa1, 0xb2, 0xc3, 0xd4] | [0xa1, 0xb2, 0x3c, 0x4d] => classico(dados, true),
        // pcapng: o primeiro bloco é sempre um Section Header Block.
        [0x0a, 0x0d, 0x0d, 0x0a] => png(dados),
        _ => Err(ErroDePcap::NaoReconhecido),
    }
}

fn u16_em(d: &[u8], i: usize, be: bool) -> u16 {
    let b = [d[i], d[i + 1]];
    if be {
        u16::from_be_bytes(b)
    } else {
        u16::from_le_bytes(b)
    }
}

fn u32_em(d: &[u8], i: usize, be: bool) -> u32 {
    let b = [d[i], d[i + 1], d[i + 2], d[i + 3]];
    if be {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    }
}

/// pcap clássico: cabeçalho de 24 bytes, depois registros de 16 bytes + dados.
fn classico(d: &[u8], be: bool) -> Result<Vec<Quadro>, ErroDePcap> {
    if d.len() < 24 {
        return Err(ErroDePcap::Truncado);
    }
    let tipo_de_enlace = u32_em(d, 20, be) as u16;

    let mut quadros = Vec::new();
    let mut i = 24;
    while i + 16 <= d.len() {
        let capturados = u32_em(d, i + 8, be) as usize;
        let ini = i + 16;
        let fim = ini.checked_add(capturados).ok_or(ErroDePcap::Truncado)?;
        if fim > d.len() {
            // Captura interrompida no meio de um registro — comum quando se para o
            // Wireshark com o botão. O que veio antes continua valendo.
            break;
        }
        quadros.push(Quadro {
            tipo_de_enlace,
            bytes: d[ini..fim].to_vec(),
        });
        i = fim;
    }
    Ok(quadros)
}

/// pcapng: blocos com tipo e tamanho. Só dois interessam.
///
/// O `LinkType` mora no Interface Description Block, e cada Enhanced Packet Block aponta
/// para a interface pelo índice — por isso a lista.
fn png(d: &[u8]) -> Result<Vec<Quadro>, ErroDePcap> {
    let mut quadros = Vec::new();
    let mut enlaces: Vec<u16> = Vec::new();
    // O SHB traz a ordem de bytes da seção no campo Byte-Order Magic.
    let mut be = false;
    let mut i = 0usize;

    while i + 12 <= d.len() {
        let tipo = u32_em(d, i, be);
        // O SHB é o único bloco cujo tipo é o mesmo nas duas ordens (0x0A0D0D0A), o que é
        // justamente como ele anuncia a ordem da seção que começa ali.
        if tipo == 0x0A0D_0D0A {
            if i + 12 > d.len() {
                break;
            }
            be = d[i + 8..i + 12] == [0x1A, 0x2B, 0x3C, 0x4D];
            enlaces.clear();
        }

        let tamanho = u32_em(d, i + 4, be) as usize;
        // Um bloco tem no mínimo tipo + tamanho + tamanho final = 12 bytes.
        if tamanho < 12 || i + tamanho > d.len() {
            break;
        }
        let bloco = &d[i..i + tamanho];

        match tipo {
            // Interface Description Block: LinkType nos 2 bytes após tipo+tamanho.
            0x0000_0001 => enlaces.push(u16_em(bloco, 8, be)),
            // Enhanced Packet Block.
            0x0000_0006 if bloco.len() >= 32 => {
                let iface = u32_em(bloco, 8, be) as usize;
                let capturados = u32_em(bloco, 20, be) as usize;
                let fim = 28usize.saturating_add(capturados);
                if fim <= bloco.len() {
                    quadros.push(Quadro {
                        tipo_de_enlace: enlaces.get(iface).copied().unwrap_or(1),
                        bytes: bloco[28..fim].to_vec(),
                    });
                }
            }
            // Simple Packet Block: sem índice de interface nem tamanho capturado próprio.
            0x0000_0003 if bloco.len() > 12 => {
                quadros.push(Quadro {
                    tipo_de_enlace: enlaces.first().copied().unwrap_or(1),
                    bytes: bloco[12..bloco.len() - 4].to_vec(),
                });
            }
            _ => {}
        }

        i += tamanho;
    }

    Ok(quadros)
}
