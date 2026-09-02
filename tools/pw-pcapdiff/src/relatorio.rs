//! Cruza o que a captura mediu com o que o IR do 1.5.3 declara.
//!
//! # A pergunta que este relatório responde
//!
//! "Este comando tem um tamanho diferente no 1.2.6, ou o nosso codificador está errado?"
//! Era a pergunta que o `LAYOUT_DIVERGE` não conseguia responder, e que fez treze
//! codificadores ficarem anos numa lista de "não dá para julgar".
//!
//! Com uma captura de um servidor 1.2.6 **de verdade**, ela vira aritmética.

use crate::gnet::Medidas;
use std::collections::BTreeMap;

/// O `cmd_header { unsigned short cmd; }` que as structs do servidor incluem e as do
/// cliente não.
const BYTES_DO_CABECALHO: usize = 2;

/// O que o IR diz sobre um comando.
pub struct DoIr {
    pub nome: String,
    /// Tamanho do payload, quando é fixo. `None` para os de tamanho variável.
    pub bytes: Option<usize>,
}

/// Veredito de uma linha da tabela.
#[derive(PartialEq, Eq, Debug)]
pub enum Veredito {
    /// A captura concorda com o IR do 1.5.3: mesmo layout nas duas versões.
    Igual,
    /// Tamanho diferente e **constante**: diferença real de versão, e agora medida.
    DifereSempre(usize),
    /// Vários tamanhos: comando de tamanho variável. Nada a concluir sobre o total.
    Variavel,
    /// Tamanho variável **em progressão aritmética**: `cabeçalho + n × elemento`.
    ///
    /// É o caso de toda lista de tamanho fixo — membros de grupo, itens de bolsa. Quando
    /// os tamanhos observados diferem sempre pelo mesmo passo, esse passo **é** o tamanho
    /// do elemento, e o resto é o cabeçalho.
    Progressao { cabecalho: usize, elemento: usize },
    /// O IR não declara tamanho fixo, então não há o que comparar.
    SemReferencia,
    /// O comando apareceu na captura e não existe na tabela do IR.
    ForaDoIr,
}

pub struct Linha {
    pub id: u16,
    pub nome: String,
    pub observado: BTreeMap<usize, usize>,
    pub ir: Option<usize>,
    pub veredito: Veredito,
}

/// Monta a tabela.
pub fn montar(medidas: &Medidas, ir: &BTreeMap<u16, DoIr>) -> Vec<Linha> {
    let mut linhas = Vec::new();

    for (id, tamanhos) in medidas {
        let entrada = ir.get(id);
        let nome = entrada
            .map(|e| e.nome.clone())
            .unwrap_or_else(|| "(fora do IR)".to_string());
        let bytes_ir = entrada.and_then(|e| e.bytes);

        let veredito = if entrada.is_none() {
            Veredito::ForaDoIr
        } else if tamanhos.len() > 1 {
            progressao(tamanhos).unwrap_or(Veredito::Variavel)
        } else {
            let observado = *tamanhos.keys().next().unwrap();
            match bytes_ir {
                None => Veredito::SemReferencia,
                Some(b) if b == observado => Veredito::Igual,
                Some(_) => Veredito::DifereSempre(observado),
            }
        };

        linhas.push(Linha {
            id: *id,
            nome,
            observado: tamanhos.clone(),
            ir: bytes_ir,
            veredito,
        });
    }

    linhas
}

/// Decompõe uma distribuição de tamanhos em `cabeçalho + n × elemento`.
///
/// # O que autoriza esta conta
///
/// Se um comando carrega uma lista de elementos de tamanho fixo, dois pacotes que diferem
/// por um elemento diferem por exatamente o tamanho dele. Com três ou mais tamanhos
/// observados, o passo constante deixa de ser coincidência: `31, 56, 81` só sai de um
/// elemento de 25 bytes.
///
/// O cabeçalho vem de `menor % passo`, e essa parte tem uma **suposição**: que o menor
/// pacote observado é o de **um** elemento, e que o cabeçalho é menor que um elemento. Nas
/// duas, se a suposição falhar, o cabeçalho sai maior por um múltiplo do elemento — nunca
/// por um valor arbitrário. Por isso o veredito diz a decomposição em vez de afirmar um
/// layout.
///
/// Exige **três** tamanhos distintos de propósito: com dois, qualquer par tem um "passo",
/// e a conta viraria numerologia.
fn progressao(tamanhos: &BTreeMap<usize, usize>) -> Option<Veredito> {
    if tamanhos.len() < 3 {
        return None;
    }
    let chaves: Vec<usize> = tamanhos.keys().copied().collect();
    let passo = chaves[1].checked_sub(chaves[0])?;
    if passo == 0 {
        return None;
    }
    if !chaves.windows(2).all(|j| j[1] - j[0] == passo) {
        return None;
    }
    Some(Veredito::Progressao {
        cabecalho: chaves[0] % passo,
        elemento: passo,
    })
}

/// Extrai do IR o que a comparação precisa: nome e tamanho fixo do payload por id.
///
/// Lê o JSON com um varredor mínimo em vez de um parser completo, pela mesma razão que o
/// `pw-rpcgen` não tem dependências: esta ferramenta decide o que é verdade sobre o
/// protocolo, e o caminho até o dado precisa caber na cabeça de quem for auditar.
pub fn do_ir(json: &str, lado: &str) -> BTreeMap<u16, DoIr> {
    let mut fora = BTreeMap::new();

    // Recorta o array daquele lado dentro de "commands".
    let Some(ini) = json.find(&format!("\"{lado}\"")) else {
        return fora;
    };
    let Some(abre) = json[ini..].find('[') else {
        return fora;
    };
    let corpo = &json[ini + abre..];

    // Os tamanhos das structs ficam noutra parte do arquivo; monta o índice primeiro.
    let tamanhos = tamanhos_das_structs(json);

    for objeto in objetos(corpo) {
        let (Some(nome), Some(id)) = (texto(objeto, "name"), inteiro(objeto, "id")) else {
            continue;
        };
        if id < 0 || id > u16::MAX as i64 {
            continue;
        }

        // A struct do **cliente** já vem sem o cabeçalho de 2 bytes; a do **servidor** o
        // inclui. É a assimetria registrada no item 9 do `ESTADO_E_RETOMADA.md`, e ignorá-la
        // faria todo comando C2S parecer 2 bytes menor do que é.
        //
        // Os comandos C2S quase nunca têm struct de cliente no IR — o que existe é a do
        // servidor —, então sem esta conta o lado C2S sairia inteiro como "sem referência".
        // O recuo só vale quando **não há** struct de cliente. Se ela existe e mesmo assim
        // não declara tamanho, o comando é de tamanho variável, e cair na do servidor
        // devolveria o tamanho do caso de um elemento como se fosse fixo — que é pior que
        // não ter referência: viraria uma "divergência" inventada.
        let bytes = match texto(objeto, "struct") {
            Some(cliente) => tamanhos.get(&cliente).copied(),
            None => texto(objeto, "server_struct")
                .and_then(|s| tamanhos.get(&s).copied())
                .and_then(|b| b.checked_sub(BYTES_DO_CABECALHO)),
        };
        fora.insert(id as u16, DoIr { nome, bytes });
    }

    fora
}

/// `"NOME": { ... "bytes": N ... }` para cada struct que declara tamanho.
fn tamanhos_das_structs(json: &str) -> BTreeMap<String, usize> {
    let mut fora = BTreeMap::new();
    let Some(ini) = json.find("\"structs\"") else {
        return fora;
    };
    let corpo = &json[ini..];

    // Cada struct é `"S2C::cmd_x": {`, e o `"bytes"` dela é o primeiro que aparece antes
    // do próximo nome de struct.
    let mut resto = corpo;
    while let Some(p) = resto.find("\": {") {
        let inicio_nome = resto[..p].rfind('"').unwrap_or(0) + 1;
        let nome = resto[inicio_nome..p].to_string();
        let depois = &resto[p + 4..];
        let ate = depois.find("\": {").unwrap_or(depois.len());
        if let Some(b) = inteiro_em(&depois[..ate], "bytes") {
            if b >= 0 {
                fora.insert(nome, b as usize);
            }
        }
        resto = depois;
    }

    fora
}

/// Os objetos de primeiro nível de um array JSON.
fn objetos(corpo: &str) -> Vec<&str> {
    let mut fora = Vec::new();
    let bytes = corpo.as_bytes();
    let mut profundidade = 0i32;
    let mut inicio = 0usize;
    let mut em_texto = false;
    let mut escapado = false;

    for (i, b) in bytes.iter().enumerate() {
        if em_texto {
            if escapado {
                escapado = false;
            } else if *b == b'\\' {
                escapado = true;
            } else if *b == b'"' {
                em_texto = false;
            }
            continue;
        }
        match b {
            b'"' => em_texto = true,
            b'{' => {
                if profundidade == 0 {
                    inicio = i;
                }
                profundidade += 1;
            }
            b'}' => {
                profundidade -= 1;
                if profundidade == 0 {
                    fora.push(&corpo[inicio..=i]);
                }
            }
            b']' if profundidade == 0 => break,
            _ => {}
        }
    }

    fora
}

fn texto(objeto: &str, campo: &str) -> Option<String> {
    let marca = format!("\"{campo}\":");
    let p = objeto.find(&marca)? + marca.len();
    let resto = objeto[p..].trim_start();
    let resto = resto.strip_prefix('"')?;
    let fim = resto.find('"')?;
    Some(resto[..fim].to_string())
}

fn inteiro(objeto: &str, campo: &str) -> Option<i64> {
    inteiro_em(objeto, campo)
}

fn inteiro_em(texto: &str, campo: &str) -> Option<i64> {
    let marca = format!("\"{campo}\":");
    let p = texto.find(&marca)? + marca.len();
    let resto = texto[p..].trim_start();
    let fim = resto
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(resto.len());
    resto[..fim].parse().ok()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ir_de_teste() -> BTreeMap<u16, DoIr> {
        let mut m = BTreeMap::new();
        m.insert(
            33,
            DoIr {
                nome: "NPC_INFO_00".into(),
                bytes: Some(16),
            },
        );
        m.insert(
            64,
            DoIr {
                nome: "TEAM_MEMBER_DATA".into(),
                bytes: None,
            },
        );
        m
    }

    fn medida(id: u16, tamanho: usize, vezes: usize) -> Medidas {
        let mut m: Medidas = BTreeMap::new();
        m.entry(id).or_default().insert(tamanho, vezes);
        m
    }

    #[test]
    fn tamanho_igual_ao_ir_e_mesma_versao() {
        let l = montar(&medida(33, 16, 5), &ir_de_teste());
        assert!(l[0].veredito == Veredito::Igual);
    }

    #[test]
    fn tamanho_constante_e_diferente_e_diferenca_de_versao_medida() {
        // A hipótese do Murillo: mesmo comando, menos bytes no 1.2.6.
        let l = montar(&medida(33, 12, 5), &ir_de_teste());
        assert!(l[0].veredito == Veredito::DifereSempre(12));
    }

    #[test]
    fn dois_tamanhos_no_mesmo_id_nao_viram_conclusao() {
        let mut m = medida(64, 40, 2);
        m.entry(64).or_default().insert(74, 1);
        let l = montar(&m, &ir_de_teste());
        assert!(
            l[0].veredito == Veredito::Variavel,
            "comando de tamanho variável não pode virar 'difere sempre'"
        );
    }

    fn medidas(pares: &[(usize, usize)]) -> Medidas {
        let mut m: Medidas = BTreeMap::new();
        let e = m.entry(64).or_default();
        for (t, n) in pares {
            e.insert(*t, *n);
        }
        m
    }

    #[test]
    fn tres_tamanhos_em_progressao_viram_cabecalho_mais_elemento() {
        // Os números são os do `TEAM_MEMBER_DATA` medido no 1.2.6: 31, 56, 81. Passo 25,
        // e 31 % 25 = 6 — que é exatamente o cabeçalho
        // `member_count(1) + data_count(1) + idLeader(4)` do cabeçalho do cliente.
        let l = montar(&medidas(&[(31, 30), (56, 16), (81, 7)]), &ir_de_teste());
        assert!(
            matches!(
                l[0].veredito,
                Veredito::Progressao {
                    cabecalho: 6,
                    elemento: 25
                }
            ),
            "a decomposição não saiu"
        );
    }

    #[test]
    fn dois_tamanhos_nao_bastam_para_afirmar_progressao() {
        // Com dois pontos qualquer diferença vira "passo". Exigir três é o que separa
        // medição de numerologia.
        let l = montar(&medidas(&[(31, 1), (56, 1)]), &ir_de_teste());
        assert!(l[0].veredito == Veredito::Variavel);
    }

    #[test]
    fn passo_irregular_continua_sendo_so_variavel() {
        // Uma lista de elementos de tamanho **variável** (itens com octetos, por exemplo)
        // não tem passo constante, e afirmar um seria inventar layout.
        let l = montar(&medidas(&[(10, 1), (30, 1), (35, 1)]), &ir_de_teste());
        assert!(l[0].veredito == Veredito::Variavel);
    }

    #[test]
    fn comando_desconhecido_e_reportado_e_nao_engolido() {
        let l = montar(&medida(999, 4, 1), &ir_de_teste());
        assert!(l[0].veredito == Veredito::ForaDoIr);
    }

    #[test]
    fn o_c2s_desconta_o_cabecalho_da_struct_do_servidor() {
        // As structs C2S do IR são as do servidor, que **incluem** os 2 bytes de
        // cabeçalho; o que medimos é o payload sem ele. Sem o desconto, todo comando C2S
        // sairia 2 bytes maior na referência — e a tabela acusaria uma diferença de versão
        // que não existe.
        let caminho = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../specs/protocol/gamedata_153.json"
        );
        let json = std::fs::read_to_string(caminho).expect("o IR sumiu");
        let ir = do_ir(&json, "c2s");

        // `SRV::C2S::CMD::player_move` tem 33 bytes com cabeçalho; o payload são 31.
        let mv = ir.get(&0).expect("PLAYER_MOVE (0) não veio do IR");
        assert_eq!(mv.nome, "PLAYER_MOVE");
        assert_eq!(mv.bytes, Some(31), "o cabeçalho de 2 bytes não foi descontado");

        // `get_all_data`: 5 com cabeçalho, 3 de payload.
        assert_eq!(ir.get(&39).and_then(|e| e.bytes), Some(3));
    }

    #[test]
    fn le_nome_id_e_tamanho_do_ir_de_verdade() {
        // Contra o IR do projeto, não contra um recorte inventado: se o formato do
        // arquivo mudar, este teste é quem avisa.
        let caminho = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../specs/protocol/gamedata_153.json"
        );
        let json = std::fs::read_to_string(caminho).expect("o IR sumiu");
        let ir = do_ir(&json, "s2c");

        let npc = ir.get(&33).expect("NPC_INFO_00 (33) não veio do IR");
        assert_eq!(npc.nome, "NPC_INFO_00");
        assert_eq!(
            npc.bytes,
            Some(16),
            "o tamanho do NPC_INFO_00 mudou no IR, ou a leitura do JSON quebrou"
        );

        let cash = ir.get(&253).expect("PLAYER_CASH (253) não veio do IR");
        assert_eq!(cash.bytes, Some(4));

        // Tamanho variável: sem `bytes` no IR, e por isso sem referência de comparação.
        let membros = ir.get(&64).expect("TEAM_MEMBER_DATA (64) não veio do IR");
        assert_eq!(membros.bytes, None);
    }
}
