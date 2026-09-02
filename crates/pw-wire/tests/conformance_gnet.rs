//! Conformidade do modelo `gnet` contra o IR extraído dos fontes do servidor.
//!
//! Ao contrário do `gamedata`, o GNET não tem deslocamentos fixos: os campos são
//! escritos um após o outro, e `Octets`, strings e contêineres têm tamanho variável.
//! Então o que se prova aqui é outra coisa — que **a ordem e os tipos** de
//! `specs/protocol/gnet_153.json` bastam para escrever e reler cada estrutura, e que a
//! ida e volta devolve exatamente os mesmos valores.
//!
//! Isso exercita, sobre as **620 estruturas reais** do protocolo, o que testes escritos
//! à mão quase nunca cobrem: `Octets` vazio, contêineres com zero elementos, aninhamento
//! de estrutura dentro de contêiner, e a ausência de prefixo no `pair`. Um campo lido
//! fora de ordem, ou um `CompactUINT` a mais ou a menos, desalinha todo o resto da
//! estrutura e aparece aqui.

use pw_wire::gnet::{Reader, Writer};
use serde_json::Value;

fn carregar_ir() -> Value {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gnet_153.json"
    );
    let texto = std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    serde_json::from_str(&texto).expect("o IR não é JSON válido")
}

/// Índice nome → estrutura, para resolver campos que referenciam outras estruturas.
fn indexar(ir: &Value) -> std::collections::BTreeMap<String, &Value> {
    ir["structs"]
        .as_array()
        .expect("`structs` deveria ser lista")
        .iter()
        .map(|s| (s["name"].as_str().unwrap().to_string(), s))
        .collect()
}

/// Um passo de escrita/leitura já resolvido, na ordem exata em que vai para o fio.
#[derive(Debug, Clone)]
enum Passo {
    Prim { kind: String, semente: u64 },
    Octets { dados: Vec<u8> },
    /// Contagem de contêiner, seguida dos passos de cada elemento.
    SeqLen(usize),
}

/// Profundidade a partir da qual os contêineres passam a ser gerados vazios.
///
/// É o que torna uma estrutura **recursiva** testável: `OctetsTree` contém um
/// `vector<OctetsTree>`, e sem um ponto de parada a expansão não termina. Gerar zero
/// elementos ali fecha a recursão sem tirar a estrutura do teste.
const PROFUNDIDADE_MAXIMA: u32 = 4;

/// Quantos elementos gerar para cada contêiner encontrado.
///
/// Zero entra de propósito: um contêiner vazio é só o `CompactUINT(0)`, e é o caso que
/// mais some de um teste escrito à mão.
fn elementos(profundidade: u32, indice: usize) -> usize {
    if profundidade >= PROFUNDIDADE_MAXIMA {
        return 0;
    }
    match (profundidade + indice as u32) % 3 {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

/// Expande uma estrutura na sequência de passos que ela escreve no fio.
///
/// Devolve `None` se o IR trouxer algo que o parser não resolveu — comparar uma
/// estrutura parcial não provaria nada.
fn expandir(
    tipo: &Value,
    idx: &std::collections::BTreeMap<String, &Value>,
    profundidade: u32,
    contador: &mut usize,
    saida: &mut Vec<Passo>,
) -> Option<()> {
    // Rede de segurança: os contêineres já param sozinhos em `PROFUNDIDADE_MAXIMA`,
    // então chegar aqui significaria um aninhamento de structs sem contêiner no meio,
    // que não existe neste protocolo.
    if profundidade > 32 {
        return None;
    }
    match tipo["kind"].as_str()? {
        "prim" => {
            let kind = tipo["prim"].as_str()?.to_string();
            *contador += 1;
            saida.push(Passo::Prim {
                kind,
                semente: (*contador as u64).wrapping_mul(0x9E37_79B9).wrapping_add(7),
            });
        }
        "octets" | "string" => {
            *contador += 1;
            // Comprimentos 0, 1 e 5 alternados: o zero é o caso que costuma faltar.
            let n = match *contador % 3 {
                0 => 0,
                1 => 1,
                _ => 5,
            };
            saida.push(Passo::Octets {
                dados: (0..n).map(|i| (i as u8).wrapping_add(*contador as u8)).collect(),
            });
        }
        "seq" => {
            *contador += 1;
            let n = elementos(profundidade, *contador);
            saida.push(Passo::SeqLen(n));
            for _ in 0..n {
                expandir(&tipo["item"], idx, profundidade + 1, contador, saida)?;
            }
        }
        "map" => {
            *contador += 1;
            let n = elementos(profundidade, *contador);
            saida.push(Passo::SeqLen(n));
            for _ in 0..n {
                // Um `map` é um contêiner de pares, e o par **não** leva prefixo: os
                // dois elementos vão em sequência e nada mais.
                expandir(&tipo["key"], idx, profundidade + 1, contador, saida)?;
                expandir(&tipo["value"], idx, profundidade + 1, contador, saida)?;
            }
        }
        "pair" => {
            expandir(&tipo["first"], idx, profundidade + 1, contador, saida)?;
            expandir(&tipo["second"], idx, profundidade + 1, contador, saida)?;
        }
        "struct" => {
            let nome = tipo["name"].as_str()?;
            let alvo = idx.get(nome)?;
            for campo in alvo["fields"].as_array()? {
                expandir(&campo["type"], idx, profundidade + 1, contador, saida)?;
            }
        }
        // `unresolved` — o parser marcou e não há layout a conferir.
        _ => return None,
    }
    Some(())
}

fn escrever(w: &mut Writer, passo: &Passo) {
    match passo {
        Passo::Prim { kind, semente } => {
            let s = *semente;
            match kind.as_str() {
                "bool" => w.bool(s & 1 == 1),
                "i8" => w.i8(s as i8),
                "u8" => w.u8(s as u8),
                "i16" => w.i16(s as i16),
                "u16" => w.u16(s as u16),
                "i32" => w.i32(s as i32),
                "u32" => w.u32(s as u32),
                "i64" => w.i64(s as i64),
                "u64" => w.u64(s),
                "f32" => w.f32(f32::from_bits(s as u32)),
                "f64" => w.f64(f64::from_bits(s)),
                outro => panic!("escalar desconhecido no IR: {outro}"),
            }
        }
        Passo::Octets { dados } => w.octets(dados),
        Passo::SeqLen(n) => w.seq_len(*n),
    }
}

fn conferir(r: &mut Reader, passo: &Passo, onde: &str) {
    match passo {
        Passo::Prim { kind, semente } => {
            let s = *semente;
            match kind.as_str() {
                "bool" => assert_eq!(r.bool().unwrap(), s & 1 == 1, "{onde}"),
                "i8" => assert_eq!(r.i8().unwrap(), s as i8, "{onde}"),
                "u8" => assert_eq!(r.u8().unwrap(), s as u8, "{onde}"),
                "i16" => assert_eq!(r.i16().unwrap(), s as i16, "{onde}"),
                "u16" => assert_eq!(r.u16().unwrap(), s as u16, "{onde}"),
                "i32" => assert_eq!(r.i32().unwrap(), s as i32, "{onde}"),
                "u32" => assert_eq!(r.u32().unwrap(), s as u32, "{onde}"),
                "i64" => assert_eq!(r.i64().unwrap(), s as i64, "{onde}"),
                "u64" => assert_eq!(r.u64().unwrap(), s, "{onde}"),
                // Por bits: sementes arbitrárias viram NaN, e `NaN != NaN` faria o
                // teste falhar por um motivo que não é o do teste.
                "f32" => assert_eq!(
                    r.f32().unwrap().to_bits(),
                    f32::from_bits(s as u32).to_bits(),
                    "{onde}"
                ),
                "f64" => assert_eq!(
                    r.f64().unwrap().to_bits(),
                    f64::from_bits(s).to_bits(),
                    "{onde}"
                ),
                outro => panic!("escalar desconhecido no IR: {outro}"),
            }
        }
        Passo::Octets { dados } => assert_eq!(r.octets().unwrap(), &dados[..], "{onde}"),
        Passo::SeqLen(n) => assert_eq!(r.seq_len().unwrap(), *n, "{onde}"),
    }
}

#[test]
fn toda_estrutura_do_ir_faz_ida_e_volta_pelo_fio() {
    let ir = carregar_ir();
    let idx = indexar(&ir);

    let mut conferidas = 0usize;
    let mut passos_totais = 0usize;
    let mut puladas = 0usize;

    for s in ir["structs"].as_array().unwrap() {
        let nome = s["name"].as_str().unwrap();

        let mut passos = Vec::new();
        let mut contador = 0usize;
        let mut ok = true;
        for campo in s["fields"].as_array().unwrap() {
            if expandir(&campo["type"], &idx, 0, &mut contador, &mut passos).is_none() {
                ok = false;
                break;
            }
        }
        if !ok {
            puladas += 1;
            continue;
        }

        let mut w = Writer::new();
        for p in &passos {
            escrever(&mut w, p);
        }
        let bytes = w.into_vec();

        let mut r = Reader::new(&bytes);
        for (i, p) in passos.iter().enumerate() {
            conferir(&mut r, p, &format!("{nome}, passo {i}"));
        }
        assert_eq!(
            r.remaining(),
            0,
            "{nome}: sobraram {} byte(s) — algum campo consumiu a mais ou a menos",
            r.remaining()
        );

        conferidas += 1;
        passos_totais += passos.len();
    }

    eprintln!(
        "gnet: {conferidas} estruturas conferidas ({passos_totais} passos), {puladas} puladas"
    );
    assert!(
        conferidas >= 620,
        "só {conferidas} estruturas conferidas; o IR ou o filtro regrediram"
    );
}

#[test]
fn contêiner_vazio_ocupa_exatamente_um_byte() {
    // `CompactUINT(0)` é um único zero. É o caso que mais falta em teste escrito à mão,
    // e o que aparece com mais frequência no fio.
    let mut w = Writer::new();
    w.seq_len(0);
    assert_eq!(w.as_slice(), &[0]);
    assert_eq!(Reader::new(w.as_slice()).seq_len().unwrap(), 0);
}

#[test]
fn os_tipos_de_campo_do_ir_sao_todos_conhecidos() {
    // Se o `pw-rpcgen` passar a emitir um tipo novo, este teste falha antes que ele
    // seja silenciosamente ignorado pelos outros.
    let ir = carregar_ir();
    let conhecidos = ["prim", "octets", "string", "seq", "map", "pair", "struct"];
    let mut vistos = std::collections::BTreeSet::new();

    for s in ir["structs"].as_array().unwrap() {
        for campo in s["fields"].as_array().unwrap() {
            let k = campo["type"]["kind"].as_str().unwrap();
            vistos.insert(k.to_string());
            assert!(
                conhecidos.contains(&k) || k == "unresolved",
                "tipo de campo novo no IR: `{k}` em {}",
                s["name"]
            );
        }
    }
    eprintln!("gnet: tipos de campo presentes no IR: {vistos:?}");
}
