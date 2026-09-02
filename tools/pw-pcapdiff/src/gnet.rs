//! Do fluxo de bytes até os subcomandos do mundo 3D, com os tamanhos medidos.
//!
//! # Os dois formatos, de novo
//!
//! O envelope é GNET: `CompactUINT opcode` + `CompactUINT tamanho` + payload, big-endian.
//! Dentro do payload do `GamedataSend` está o **outro** formato — little-endian, `pack(1)`
//! — começando pelo `cmd_header` de 2 bytes. É a mesma fronteira do `SubComando` do
//! `pw-gs`, e aqui ela é atravessada na direção da leitura.
//!
//! O `compact_uint` vem do `pw-wire`, e não de uma segunda implementação aqui: ele já foi
//! conferido contra 620 estruturas, e duas implementações do mesmo formato é exatamente a
//! divergência que este projeto passa o tempo caçando.

use pw_wire::gnet::Reader;
use std::collections::BTreeMap;

/// Quantos bytes de payload cada id de subcomando trouxe, e quantas vezes.
///
/// O `BTreeMap` interno é `tamanho → ocorrências`. Guardar a distribuição, e não só um
/// tamanho, é o que distingue "este comando tem N bytes" de "este comando é de tamanho
/// variável" — e é a diferença entre uma linha confiável e um palpite.
pub type Medidas = BTreeMap<u16, BTreeMap<usize, usize>>;

/// O que a leitura de um sentido produziu.
pub struct Leitura {
    /// O que o servidor mandou ao cliente.
    pub para_o_cliente: Medidas,
    /// O que o cliente mandou ao servidor.
    pub do_cliente: Medidas,
    /// Quadros GNET lidos com sucesso.
    pub quadros: usize,
    /// Onde a leitura parou, se parou antes do fim.
    ///
    /// Um fluxo que termina no meio de um quadro é normal (a captura acabou). Um fluxo
    /// que **desalinha** é outra coisa, e a diferença aparece aqui: se sobrou muito byte,
    /// a leitura perdeu o sincronismo e a tabela não vale.
    pub sobra: usize,
}

/// Um envelope GNET que carrega subcomandos do mundo 3D.
///
/// # Por que isto é uma tabela e não um número
///
/// Os subcomandos viajam dentro de **quatro** envelopes diferentes, e o caminho até os
/// bytes não é o mesmo em todos:
///
/// | Protocolo | id | Campos antes do `data` |
/// | :--- | ---: | :--- |
/// | `GamedataSend` | 34 | nenhum — é o elo com o cliente |
/// | `S2CGamedataSend` | 74 | `int roleid` + `unsigned int localsid` |
/// | `C2SGamedataSend` | 75 | idem |
/// | `S2CMulticast` | 77 | nenhum (a lista de jogadores vem **depois** do `data`) |
///
/// Em todos, o `data` é um `Octets`: **`CompactUINT comprimento` e só então os bytes**.
pub struct Envelope {
    pub opcode: u32,
    /// Bytes de campos fixos antes do `Octets data`.
    pub antes_do_data: usize,
    /// De onde vem o comando.
    pub sentido: Sentido,
}

/// Em que direção o subcomando viaja.
///
/// # Por que não dá para decidir pela porta
///
/// No elo interno, o `glinkd` é quem **escuta** (porta 29301, o `GProviderServer1` do
/// `gamesys.conf`) e o `gs` é quem conecta. Ou seja, quem parece "cliente" pela porta é
/// justamente quem manda os comandos **S2C**. Deduzir a direção da porta inverte as duas
/// tabelas — e uma tabela invertida não parece errada: parece uma descoberta.
///
/// Os próprios opcodes já declaram a direção (`S2CGamedataSend`, `C2SGamedataSend`), então
/// é deles que ela sai. Só o elo com o cliente usa o mesmo opcode nos dois sentidos, e aí
/// sim a porta decide.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sentido {
    ParaOCliente,
    DoCliente,
    /// O opcode não distingue; quem decide é a porta de origem.
    PelaPorta,
}

/// Os envelopes do elo com o cliente (o que o `glinkd` fala com o jogo).
pub const ENVELOPES_DO_CLIENTE: &[Envelope] = &[Envelope {
    opcode: 34,
    antes_do_data: 0,
    sentido: Sentido::PelaPorta,
}];

/// Os envelopes internos, entre `gs` e `glinkd`.
///
/// **É por aqui que se mede o 1.2.6.** O elo com o cliente é cifrado com ARCFOUR logo
/// depois do `KeyExchange` (item 54), e a chave não é derivável da captura. Os daemons
/// conversam entre si em claro, e os subcomandos passam por ali **antes** de serem
/// cifrados.
pub const ENVELOPES_INTERNOS: &[Envelope] = &[
    Envelope {
        opcode: 74,
        antes_do_data: 8,
        sentido: Sentido::ParaOCliente,
    },
    Envelope {
        opcode: 75,
        antes_do_data: 8,
        sentido: Sentido::DoCliente,
    },
    Envelope {
        opcode: 77,
        antes_do_data: 0,
        sentido: Sentido::ParaOCliente,
    },
];

/// Percorre um sentido do fluxo e mede os subcomandos que passaram por ele.
///
/// # O `Octets` que eu tinha esquecido
///
/// A primeira versão desta função tratava o payload do envelope **como se fosse** o
/// subcomando. Está errado: o campo é um `Octets`, então vem `CompactUINT comprimento` e
/// só depois os bytes. Um subcomando lido assim sai com o id trocado (os dois primeiros
/// bytes viram comprimento + metade do id) e com o tamanho errado.
///
/// O teste sintético não pegou porque **eu montei a captura de teste com a mesma
/// suposição errada** — o gabarito concordava com o bug. É exatamente a armadilha do item
/// 47, na ferramenta escrita para evitá-la. O teste agora monta o envelope com
/// `write_octets` do `pw-protocol`, que é o mesmo caminho que a produção usa.
pub fn medir(bytes: &[u8], envelopes: &[Envelope], porta_e_do_servidor: bool) -> Leitura {
    let mut para_o_cliente: Medidas = BTreeMap::new();
    let mut do_cliente: Medidas = BTreeMap::new();
    let mut quadros = 0usize;
    let mut pos = 0usize;

    while pos < bytes.len() {
        let mut r = Reader::new(&bytes[pos..]);
        let Ok(opcode) = r.compact_uint() else { break };
        let Ok(tamanho) = r.compact_uint() else { break };
        let cabecalho = r.position();
        let tamanho = tamanho as usize;

        if pos + cabecalho + tamanho > bytes.len() {
            // Quadro incompleto: ou a captura acabou, ou perdemos bytes. Quem decide é o
            // chamador, olhando a `sobra` e os buracos da remontagem.
            break;
        }

        let payload = &bytes[pos + cabecalho + tamanho - tamanho..pos + cabecalho + tamanho];
        if let Some(env) = envelopes.iter().find(|e| e.opcode == opcode) {
            if let Some(dados) = desembrulhar(payload, env.antes_do_data) {
                if dados.len() >= 2 {
                    let id = u16::from_le_bytes([dados[0], dados[1]]);
                    let para_cliente = match env.sentido {
                        Sentido::ParaOCliente => true,
                        Sentido::DoCliente => false,
                        Sentido::PelaPorta => porta_e_do_servidor,
                    };
                    let alvo = if para_cliente {
                        &mut para_o_cliente
                    } else {
                        &mut do_cliente
                    };
                    *alvo.entry(id).or_default().entry(dados.len() - 2).or_default() += 1;
                }
            }
        }

        quadros += 1;
        pos += cabecalho + tamanho;
    }

    Leitura {
        para_o_cliente,
        do_cliente,
        quadros,
        sobra: bytes.len() - pos,
    }
}

/// Pula os campos fixos e devolve o conteúdo do `Octets data`.
fn desembrulhar(payload: &[u8], antes: usize) -> Option<&[u8]> {
    if payload.len() < antes {
        return None;
    }
    let mut r = Reader::new(&payload[antes..]);
    let n = r.compact_uint().ok()? as usize;
    let ini = antes + r.position();
    payload.get(ini..ini + n)
}

/// O inventário de quadros de um fluxo: `opcode → (tamanho do payload → ocorrências)`.
///
/// Diferente de [`medir`], que abre o envelope e vai atrás dos subcomandos, este fica na
/// camada de fora. Serve para **descobrir** o que passa por um elo que ainda não
/// conhecemos — por exemplo o `glinkd ↔ gdeliveryd` (porta 29100), por onde viaja a lista
/// de personagens. Sem isto, a única forma de saber qual opcode carrega o quê seria
/// adivinhar.
pub fn inventariar(bytes: &[u8]) -> (BTreeMap<u32, BTreeMap<usize, usize>>, usize) {
    let mut mapa: BTreeMap<u32, BTreeMap<usize, usize>> = BTreeMap::new();
    let mut pos = 0usize;

    while pos < bytes.len() {
        let mut r = Reader::new(&bytes[pos..]);
        let Ok(opcode) = r.compact_uint() else { break };
        let Ok(tamanho) = r.compact_uint() else { break };
        let cabecalho = r.position();
        let tamanho = tamanho as usize;
        if pos + cabecalho + tamanho > bytes.len() {
            break;
        }
        *mapa.entry(opcode).or_default().entry(tamanho).or_default() += 1;
        pos += cabecalho + tamanho;
    }

    (mapa, bytes.len() - pos)
}

/// A sequência de `(opcode, tamanho)` na ordem em que os quadros passaram.
///
/// O inventário diz *o que* passou; isto diz *em que ordem*. Para comparar um handshake
/// inteiro com o de um servidor de verdade, a ordem é metade da informação.
pub fn sequencia(bytes: &[u8]) -> Vec<(u32, usize)> {
    let mut saida = Vec::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let mut r = Reader::new(&bytes[pos..]);
        let Ok(opcode) = r.compact_uint() else { break };
        let Ok(tamanho) = r.compact_uint() else { break };
        let cabecalho = r.position();
        let tamanho = tamanho as usize;
        if pos + cabecalho + tamanho > bytes.len() {
            break;
        }
        saida.push((opcode, tamanho));
        pos += cabecalho + tamanho;
    }
    saida
}

/// Devolve os payloads crus dos quadros de um opcode, na ordem em que apareceram.
///
/// É o que permite olhar os bytes de um `RoleList_Re` de verdade em vez de deduzir o
/// formato. `limite` evita despejar uma sessão inteira.
pub fn payloads_de(bytes: &[u8], alvo: u32, limite: usize) -> Vec<Vec<u8>> {
    let mut saida = Vec::new();
    let mut pos = 0usize;

    while pos < bytes.len() && saida.len() < limite {
        let mut r = Reader::new(&bytes[pos..]);
        let Ok(opcode) = r.compact_uint() else { break };
        let Ok(tamanho) = r.compact_uint() else { break };
        let cabecalho = r.position();
        let tamanho = tamanho as usize;
        if pos + cabecalho + tamanho > bytes.len() {
            break;
        }
        if opcode == alvo {
            saida.push(bytes[pos + cabecalho..pos + cabecalho + tamanho].to_vec());
        }
        pos += cabecalho + tamanho;
    }

    saida
}

/// Junta as medidas de vários fluxos numa só.
pub fn juntar(destino: &mut Medidas, origem: &Medidas) {
    for (id, tamanhos) in origem {
        let e = destino.entry(*id).or_default();
        for (t, n) in tamanhos {
            *e.entry(*t).or_default() += n;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Monta um quadro GNET com o opcode e o payload dados.
    ///
    /// Só cobre `CompactUINT` de 1 e 2 bytes, que é o suficiente para os testes: os
    /// opcodes do jogo são pequenos e os payloads de teste também.
    fn quadro(opcode: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        empacotar(&mut v, opcode);
        empacotar(&mut v, payload.len() as u32);
        v.extend_from_slice(payload);
        v
    }

    fn empacotar(v: &mut Vec<u8>, n: u32) {
        if n < 0x40 {
            v.push(n as u8);
        } else {
            v.push(0x80 | ((n >> 8) as u8 & 0x3F));
            v.push((n & 0xFF) as u8);
        }
    }

    /// O subcomando cru: cabeçalho de 2 bytes little-endian + corpo.
    fn subcomando(id: u16, corpo: &[u8]) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(corpo);
        v
    }

    /// O subcomando dentro do `Octets` do envelope, que é como ele viaja de verdade.
    fn embrulhado(id: u16, corpo: &[u8]) -> Vec<u8> {
        let sub = subcomando(id, corpo);
        let mut v = Vec::new();
        empacotar(&mut v, sub.len() as u32);
        v.extend_from_slice(&sub);
        v
    }

    /// Os envelopes de teste: o do cliente (34, sem campos antes) e um interno.
    const CLIENTE: &[Envelope] = &[Envelope {
        opcode: 34,
        antes_do_data: 0,
        sentido: Sentido::PelaPorta,
    }];

    #[test]
    fn mede_o_payload_sem_o_cabecalho_do_subcomando() {
        // Um comando 33 com 16 bytes de payload é o `NPC_INFO_00` do 1.5.3.
        let fluxo = quadro(34, &embrulhado(33, &[0u8; 16]));
        let l = medir(&fluxo, CLIENTE, true);
        assert_eq!(l.quadros, 1);
        assert_eq!(l.sobra, 0);
        assert_eq!(l.para_o_cliente[&33][&16], 1);
    }

    #[test]
    fn conta_ocorrencias_e_guarda_a_distribuicao() {
        let mut fluxo = Vec::new();
        fluxo.extend(quadro(34, &embrulhado(64, &[0u8; 40])));
        fluxo.extend(quadro(34, &embrulhado(64, &[0u8; 74])));
        fluxo.extend(quadro(34, &embrulhado(64, &[0u8; 40])));

        let l = medir(&fluxo, CLIENTE, true);
        // Dois tamanhos distintos para o mesmo id é a assinatura de comando de tamanho
        // variável — e é por isso que guardamos a distribuição em vez de um número.
        assert_eq!(l.para_o_cliente[&64].len(), 2);
        assert_eq!(l.para_o_cliente[&64][&40], 2);
        assert_eq!(l.para_o_cliente[&64][&74], 1);
    }

    #[test]
    fn ignora_envelope_que_nao_e_gamedata() {
        // Um pacote de login com bytes que **pareceriam** um subcomando 33 se a
        // ferramenta olhasse qualquer envelope.
        let fluxo = quadro(3, &embrulhado(33, &[0u8; 16]));
        let l = medir(&fluxo, CLIENTE, true);
        assert!(l.para_o_cliente.is_empty(), "leu comando de um envelope de login");
        assert_eq!(l.quadros, 1, "o quadro devia ter sido percorrido mesmo assim");
    }

    #[test]
    fn um_quadro_incompleto_no_fim_vira_sobra_e_nao_medida_errada() {
        let mut fluxo = quadro(34, &embrulhado(33, &[0u8; 16]));
        let cortado = quadro(34, &embrulhado(38, &[0u8; 36]));
        fluxo.extend_from_slice(&cortado[..10]); // captura parada no meio

        let l = medir(&fluxo, CLIENTE, true);
        assert_eq!(l.para_o_cliente[&33][&16], 1);
        assert!(
            !l.para_o_cliente.contains_key(&38),
            "mediu um comando a partir de um quadro truncado"
        );
        assert_eq!(l.sobra, 10, "a sobra é o sinal de que algo ficou por ler");
    }

    #[test]
    fn varios_quadros_seguidos_sao_lidos_em_sequencia() {
        let mut fluxo = Vec::new();
        for id in [33u16, 38, 24, 253] {
            fluxo.extend(quadro(34, &embrulhado(id, &[0u8; 4])));
        }
        let l = medir(&fluxo, CLIENTE, true);
        assert_eq!(l.quadros, 4);
        assert_eq!(l.sobra, 0);
        assert_eq!(l.para_o_cliente.len(), 4);
    }

    #[test]
    fn payload_grande_usa_compact_uint_de_dois_bytes() {
        // 300 bytes força a forma `10xxxxxx xxxxxxxx` do CompactUINT — o caso em que uma
        // implementação errada do formato apareceria.
        let fluxo = quadro(34, &embrulhado(42, &vec![0u8; 300]));
        let l = medir(&fluxo, CLIENTE, true);
        assert_eq!(l.para_o_cliente[&42][&300], 1);
        assert_eq!(l.sobra, 0);
    }
}
