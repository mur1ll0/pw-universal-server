//! Confere as mensagens do barramento contra o IR, e o transporte contra TCP de verdade.
//!
//! As mensagens daqui não são um formato inventado para este projeto: são protocolos
//! GNET reais. Então valem as mesmas duas perguntas que valem para os pacotes do
//! cliente — o opcode está certo, e os campos estão na ordem certa — respondidas contra
//! `specs/protocol/gnet_153.json`.
//!
//! O IR responde ainda uma terceira, que é própria do barramento: **quem fala com
//! quem**. O campo `daemons` de cada protocolo diz em que pernas ele trafega, e é o que
//! sustenta a afirmação de que o `GamedataSend` (34) é do cliente enquanto o par 74/75 é
//! entre daemons.

use pw_bus::{opcode, BusClient, BusListener, BusMessage};
use serde_json::Value;

fn ir() -> Value {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/protocol/gnet_153.json"
    );
    let texto = std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {caminho}: {e}"));
    serde_json::from_str(&texto).expect("o IR não é JSON válido")
}

fn protocolo<'a>(ir: &'a Value, nome: &str) -> &'a Value {
    ir["protocols"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == nome)
        .unwrap_or_else(|| panic!("{nome} não existe no IR"))
}

#[test]
fn os_opcodes_do_barramento_batem_com_o_ir() {
    let ir = ir();
    for (nome, valor) in [
        ("PlayerLogout", opcode::PLAYER_LOGOUT),
        ("EnterWorld", opcode::ENTER_WORLD),
        ("S2CGamedataSend", opcode::S2C_GAMEDATA_SEND),
        ("C2SGamedataSend", opcode::C2S_GAMEDATA_SEND),
    ] {
        let p = protocolo(&ir, nome);
        assert_eq!(
            p["id"].as_u64(),
            Some(u64::from(valor)),
            "{nome}: o IR discorda do opcode"
        );
    }
}

#[test]
fn os_campos_de_cada_mensagem_seguem_a_ordem_do_ir() {
    // Os tipos que cada mensagem escreve, na ordem em que escreve.
    let esperado: &[(&str, &[&str])] = &[
        ("C2SGamedataSend", &["i32", "u32", "octets"]),
        ("S2CGamedataSend", &["i32", "u32", "octets"]),
        ("EnterWorld", &["i32", "i32", "i32", "i32", "i32", "u32"]),
        ("PlayerLogout", &["i32", "i32", "i32", "u32"]),
    ];

    let ir = ir();
    for (nome, tipos) in esperado {
        let p = protocolo(&ir, nome);
        let campos = p["fields"].as_array().unwrap();
        assert_eq!(
            campos.len(),
            tipos.len(),
            "{nome}: o IR tem {} campos, o barramento escreve {}",
            campos.len(),
            tipos.len()
        );
        for (i, (campo, t)) in campos.iter().zip(tipos.iter()).enumerate() {
            let no_ir = match campo["type"]["kind"].as_str().unwrap() {
                "prim" => campo["type"]["prim"].as_str().unwrap(),
                "octets" | "string" => "octets",
                outro => panic!("{nome}: tipo inesperado {outro}"),
            };
            assert_eq!(
                no_ir, *t,
                "{nome}, campo {i} (`{}`): IR diz {no_ir}, o barramento escreve {t}",
                campo["name"]
            );
        }
    }
}

#[test]
fn o_gamedata_do_cliente_nao_tem_dono_e_o_do_barramento_tem() {
    // É esta diferença que define o barramento: o cliente manda só `data`, porque a
    // conexão já sabe quem ele é. Entre daemons o mesmo payload precisa de `roleid` e
    // `localsid` — sem isso o servidor de jogo não sabe de quem é, nem por onde
    // responder.
    let ir = ir();

    let do_cliente = protocolo(&ir, "GamedataSend");
    let campos: Vec<&str> = do_cliente["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(campos, vec!["data"]);

    for nome in ["C2SGamedataSend", "S2CGamedataSend"] {
        let campos: Vec<&str> = protocolo(&ir, nome)["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert_eq!(campos, vec!["roleid", "localsid", "data"], "{nome}");
    }
}

#[test]
fn o_ir_confirma_quem_fala_com_quem() {
    // O `daemons` do IR é o que sustenta a separação: o `GamedataSend` do cliente só é
    // falado pelo `glinkd` (a ponta que atende o jogador), enquanto o par 74/75 é
    // falado por `glinkd` **e** `gamed` — as duas pontas do barramento.
    let ir = ir();
    let daemons = |nome: &str| -> Vec<String> {
        protocolo(&ir, nome)["daemons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.as_str().unwrap().to_string())
            .collect()
    };

    assert_eq!(daemons("GamedataSend"), vec!["glinkd"]);
    for nome in ["C2SGamedataSend", "S2CGamedataSend"] {
        let d = daemons(nome);
        assert!(d.contains(&"glinkd".to_string()), "{nome}: {d:?}");
        assert!(d.contains(&"gamed".to_string()), "{nome}: {d:?}");
    }
}

#[tokio::test]
async fn as_duas_pontas_conversam_por_tcp_de_verdade() {
    // Porta 0: o sistema escolhe uma livre, e o teste não depende de nenhuma específica.
    let escuta = BusListener::bind("127.0.0.1:0").await.expect("bind falhou");
    let addr = escuta.local_addr().unwrap();

    let servidor = tokio::spawn(async move {
        let mut conexao = escuta.aceitar().await.expect("accept falhou");
        let recebida = conexao.receber().await.unwrap().expect("nada chegou");

        // O servidor de jogo responde ao mesmo jogador, pelo outro opcode do par.
        match recebida {
            BusMessage::ClientToGame {
                roleid,
                localsid,
                data,
            } => {
                assert_eq!(data, vec![0x0F, 0x00, 0x2A]);
                conexao
                    .enviar(BusMessage::GameToClient {
                        roleid,
                        localsid,
                        data: vec![0x0E, 0x00],
                    })
                    .await
                    .unwrap();
            }
            outra => panic!("chegou {outra:?}"),
        }
    });

    let mut cliente = BusClient::conectar(addr).await.expect("connect falhou");
    cliente
        .enviar(BusMessage::ClientToGame {
            roleid: 1024,
            localsid: 0xDEAD_BEEF,
            data: vec![0x0F, 0x00, 0x2A],
        })
        .await
        .unwrap();

    let resposta = cliente.receber().await.unwrap().expect("sem resposta");
    assert_eq!(
        resposta,
        BusMessage::GameToClient {
            roleid: 1024,
            localsid: 0xDEAD_BEEF,
            data: vec![0x0E, 0x00],
        }
    );

    servidor.await.unwrap();
}

#[tokio::test]
async fn uma_rajada_chega_inteira_e_em_ordem() {
    // TCP é fluxo, não mensagem: várias mensagens seguidas viram um punhado de bytes que
    // o codec precisa reseparar. Um payload grande no meio força o quadro a cruzar mais
    // de um pedaço da rede, que é onde um enquadramento frágil quebra.
    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();

    const QUANTAS: i32 = 50;

    let servidor = tokio::spawn(async move {
        let mut conexao = escuta.aceitar().await.unwrap();
        let mut vistas = Vec::new();
        while let Some(m) = conexao.receber().await.unwrap() {
            vistas.push(m);
            if vistas.len() == QUANTAS as usize {
                break;
            }
        }
        vistas
    });

    let mut cliente = BusClient::conectar(addr).await.unwrap();
    for i in 0..QUANTAS {
        cliente
            .enviar(BusMessage::ClientToGame {
                roleid: i,
                localsid: i as u32,
                // Tamanho variando, inclusive um bem maior que o MTU.
                data: vec![(i % 256) as u8; (i as usize % 7) * 1000],
            })
            .await
            .unwrap();
    }

    let vistas = servidor.await.unwrap();
    assert_eq!(vistas.len(), QUANTAS as usize);
    for (i, m) in vistas.iter().enumerate() {
        assert_eq!(m.roleid(), i as i32, "chegou fora de ordem");
    }
}
