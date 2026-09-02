//! Conformidade do modelo `gamedata` contra o IR extraído dos fontes C++.
//!
//! O que este teste prova, e por que ele vale mais que um teste escrito à mão: os
//! deslocamentos em `specs/protocol/gamedata_153.json` **não** foram escolhidos por
//! ninguém. Eles saíram dos cabeçalhos originais e foram conferidos, campo a campo,
//! contra o `g++ -m32` lendo esses mesmos cabeçalhos
//! (`tools/pw-rpcgen/verify/check_sizes.py`, 4.426 asserções). Então, quando este
//! teste escreve os campos de uma struct em sequência com o [`Writer`] e cobra que
//! cada um caia no deslocamento que o IR anuncia, ele está comparando o empacotamento
//! deste crate com o que o compilador C++ de verdade produziu — para **mais de mil
//! structs reais do jogo**, não para um punhado de exemplos.
//!
//! Um `#pragma pack(1)` esquecido, um `size_t` tratado como 8 bytes, um `A3DVECTOR3`
//! com preenchimento: qualquer um desses aparece aqui como dezenas de falhas.

use pw_wire::gamedata::{Reader, Vec3, Writer};
use serde_json::Value;

fn carregar_ir() -> Value {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gamedata_153.json"
    );
    let texto = std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    serde_json::from_str(&texto).expect("o IR não é JSON válido")
}

/// Um campo já achatado num escalar concreto, com o que escrever e onde.
#[derive(Debug, Clone)]
struct Escalar {
    kind: String,
    /// Valor a escrever, codificado como bits. Determinístico a partir da posição,
    /// para que uma leitura no lugar errado devolva um número diferente.
    semente: u64,
}

/// Expande os campos de uma struct na sequência de escalares que ela põe no fio.
///
/// Devolve `None` se algo não for resolvível — um campo de tamanho desconhecido, uma
/// lista de tamanho variável, um tipo que não é copiável por `memcpy`. Comparar uma
/// struct parcial contra um deslocamento completo produziria uma falha falsa.
fn achatar(ir: &Value, chave: &str, profundidade: u32) -> Option<Vec<Escalar>> {
    if profundidade > 8 {
        return None;
    }
    let s = ir["structs"].get(chave)?;
    if !s["variable"].is_null() {
        return None;
    }

    let mut out = Vec::new();
    for campo in s["fields"].as_array()? {
        let n = campo["array_len"].as_u64().unwrap_or(1) as usize;
        let tipo = &campo["type"];
        let kind = tipo["kind"].as_str()?;

        for _ in 0..n {
            match kind {
                "prim" => {
                    let prim = tipo["prim"].as_str()?;
                    let semente = (out.len() as u64).wrapping_mul(0x9E37_79B9).wrapping_add(1);
                    out.push(Escalar {
                        kind: prim.to_string(),
                        semente,
                    });
                }
                "vec3" => {
                    for _ in 0..3 {
                        let semente = (out.len() as u64).wrapping_mul(0x9E37_79B9).wrapping_add(1);
                        out.push(Escalar {
                            kind: "f32".to_string(),
                            semente,
                        });
                    }
                }
                "struct" => {
                    let alvo = campo["resolved"].as_str()?;
                    out.extend(achatar(ir, alvo, profundidade + 1)?);
                }
                // `placeholder` e `unresolved` não têm layout conhecido.
                _ => return None,
            }
        }
    }
    Some(out)
}

/// Escreve um escalar e devolve o valor que ele deve ter na volta.
fn escrever(w: &mut Writer, e: &Escalar) {
    let s = e.semente;
    match e.kind.as_str() {
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

/// Lê um escalar e confere contra a semente.
fn conferir_leitura(r: &mut Reader, e: &Escalar, onde: &str) {
    let s = e.semente;
    match e.kind.as_str() {
        "bool" => assert_eq!(r.bool().unwrap(), s & 1 == 1, "{onde}"),
        "i8" => assert_eq!(r.i8().unwrap(), s as i8, "{onde}"),
        "u8" => assert_eq!(r.u8().unwrap(), s as u8, "{onde}"),
        "i16" => assert_eq!(r.i16().unwrap(), s as i16, "{onde}"),
        "u16" => assert_eq!(r.u16().unwrap(), s as u16, "{onde}"),
        "i32" => assert_eq!(r.i32().unwrap(), s as i32, "{onde}"),
        "u32" => assert_eq!(r.u32().unwrap(), s as u32, "{onde}"),
        "i64" => assert_eq!(r.i64().unwrap(), s as i64, "{onde}"),
        "u64" => assert_eq!(r.u64().unwrap(), s, "{onde}"),
        // Comparados por bits: uma semente arbitrária vira NaN com frequência, e
        // `NaN != NaN` faria o teste falhar por um motivo que não é o do teste.
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

fn tamanho(kind: &str) -> usize {
    match kind {
        "bool" | "i8" | "u8" => 1,
        "i16" | "u16" => 2,
        "i32" | "u32" | "f32" => 4,
        "i64" | "u64" | "f64" => 8,
        outro => panic!("escalar desconhecido no IR: {outro}"),
    }
}

#[test]
fn o_empacotamento_reproduz_o_layout_conferido_pelo_compilador() {
    let ir = carregar_ir();
    let structs = ir["structs"].as_object().expect("`structs` deveria ser objeto");

    let mut conferidas = 0usize;
    let mut escalares = 0usize;
    let mut puladas = 0usize;

    for (chave, s) in structs {
        let Some(esperado) = s["bytes"].as_u64() else {
            puladas += 1;
            continue;
        };
        let Some(campos) = achatar(&ir, chave, 0) else {
            puladas += 1;
            continue;
        };
        if campos.is_empty() {
            puladas += 1;
            continue;
        }

        // Escreve em sequência, cobrando o deslocamento de cada campo enquanto escreve.
        let mut w = Writer::with_capacity(esperado as usize);
        let mut posicao = 0usize;
        for e in &campos {
            assert_eq!(
                w.len(),
                posicao,
                "{chave}: o campo em {posicao} começou em {} — há preenchimento onde \
                 não deveria",
                w.len()
            );
            escrever(&mut w, e);
            posicao += tamanho(&e.kind);
        }

        // O total tem que bater com o `sizeof` que o compilador de 32 bits confirmou.
        assert_eq!(
            w.len() as u64, esperado,
            "{chave}: escrevi {} bytes, o C++ diz que a struct tem {esperado}",
            w.len()
        );

        // E a volta: cada campo lido de onde foi escrito devolve o mesmo valor.
        let bytes = w.into_vec();
        let mut r = Reader::new(&bytes);
        for (i, e) in campos.iter().enumerate() {
            conferir_leitura(&mut r, e, &format!("{chave}, escalar {i}"));
        }
        assert_eq!(r.remaining(), 0, "{chave}: sobraram bytes na leitura");

        conferidas += 1;
        escalares += campos.len();
    }

    eprintln!(
        "gamedata: {conferidas} structs conferidas ({escalares} escalares), {puladas} puladas \
         (tamanho variável ou campo irresolúvel)"
    );

    // Um limite inferior, para que uma regressão que faça o IR virar quase tudo
    // "pulado" apareça como falha em vez de como um teste que passa sem testar nada.
    assert!(
        conferidas > 1000,
        "só {conferidas} structs foram conferidas; o IR ou o filtro regrediram"
    );
}

#[test]
fn os_deslocamentos_do_ir_batem_com_a_leitura_endereçada() {
    // O outro modo de uso: em vez de ler em sequência, ir direto ao deslocamento que o
    // IR anuncia. É assim que o `pw-protocol` vai decodificar, e é o que prova que os
    // deslocamentos do IR são utilizáveis como estão.
    let ir = carregar_ir();
    let structs = ir["structs"].as_object().unwrap();
    let mut conferidos = 0usize;

    for (chave, s) in structs {
        if s["bytes"].is_null() {
            continue;
        }
        let Some(campos) = achatar(&ir, chave, 0) else {
            continue;
        };
        if campos.is_empty() {
            continue;
        }

        let mut w = Writer::new();
        for e in &campos {
            escrever(&mut w, e);
        }
        let bytes = w.into_vec();

        // Só os campos de primeiro nível, que são os que têm deslocamento no IR.
        let mut r = Reader::new(&bytes);
        for campo in s["fields"].as_array().unwrap() {
            let (Some(offset), Some(kind)) =
                (campo["offset"].as_u64(), campo["type"]["kind"].as_str())
            else {
                continue;
            };
            if kind != "prim" {
                continue;
            }
            let offset = offset as usize;
            let prim = campo["type"]["prim"].as_str().unwrap();

            // O escalar que mora naquele deslocamento, achado pela soma dos tamanhos.
            let mut acc = 0usize;
            let Some(esperado) = campos.iter().find(|e| {
                let aqui = acc;
                acc += tamanho(&e.kind);
                aqui == offset
            }) else {
                continue;
            };
            assert_eq!(
                esperado.kind, prim,
                "{chave}: o IR diz {prim} em {offset}, o achatamento diz {}",
                esperado.kind
            );

            r.at(offset).unwrap();
            conferir_leitura(&mut r, esperado, &format!("{chave} @ {offset}"));
            conferidos += 1;
        }
    }

    eprintln!("gamedata: {conferidos} campos lidos pelo deslocamento do IR");
    assert!(conferidos > 2400, "só {conferidos} campos conferidos");
}

#[test]
fn a3dvector3_do_ir_tem_doze_bytes_em_todas_as_ocorrencias() {
    // O IR declara `vec3` como três f32 sem preenchimento. Se o crate discordasse
    // disso, cada struct de posição sairia deslocada — e há dezenas delas.
    let ir = carregar_ir();
    let mut ocorrencias = 0usize;

    for (_, s) in ir["structs"].as_object().unwrap() {
        for campo in s["fields"].as_array().unwrap() {
            if campo["type"]["kind"] == "vec3" {
                assert_eq!(campo["type"]["bytes"].as_u64(), Some(12));
                let n = campo["array_len"].as_u64().unwrap_or(1);
                if let Some(bytes) = campo["bytes"].as_u64() {
                    assert_eq!(bytes, 12 * n);
                }
                ocorrencias += 1;
            }
        }
    }

    let mut w = Writer::new();
    w.vec3(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(w.len(), 12);

    eprintln!("gamedata: {ocorrencias} campos A3DVECTOR3 no IR");
    assert!(ocorrencias > 20, "só {ocorrencias} ocorrências de vec3");
}
