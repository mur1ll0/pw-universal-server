//! O caminho que o `pw-gs` não tinha: um payload do cliente chegar ao mundo e voltar.
//!
//! Este teste não exercita o formato do fio (isso é `barramento_contra_o_ir.rs`), e sim
//! **a costura**: uma ponta de link conecta, anuncia um jogador, manda um subcomando do
//! mundo 3D e recebe a resposta endereçada àquele jogador.
//!
//! Ele monta o servidor com o `BusListener`/`BusConnection` de verdade e um TCP de
//! verdade, e não com dublês. Um `mpsc` interno passaria e não provaria nada sobre a
//! rede — que é justamente o que faltava.

use pw_bus::transport::BusConnection;
use pw_bus::{BusClient, BusListener, BusMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// O mínimo de um servidor de mundo: sabe quem entrou e responde a quem mandou.
///
/// Reproduz o roteamento do `pw_gs::BusServer` sem arrastar o `WorldInstance`, que
/// precisa de banco. O que está sob teste é a costura, não a simulação.
struct MundoDeTeste {
    sessoes: Arc<Mutex<HashMap<i32, u32>>>,
}

impl MundoDeTeste {
    async fn atender(&self, mut conexao: BusConnection) {
        let (envio, mut fila) = mpsc::channel::<BusMessage>(64);

        loop {
            tokio::select! {
                entrada = conexao.receber() => {
                    let Ok(Some(msg)) = entrada else { break };
                    match msg {
                        BusMessage::EnterWorld { roleid, localsid, .. } => {
                            self.sessoes.lock().await.insert(roleid, localsid);
                        }
                        BusMessage::ClientToGame { roleid, data, .. } => {
                            // O cabeçalho do subcomando é little-endian.
                            let id = u16::from_le_bytes([data[0], data[1]]);
                            let localsid = *self.sessoes.lock().await.get(&roleid)
                                .expect("subcomando de jogador que não entrou");
                            // Responde com o comando seguinte, só para provar o trajeto.
                            let mut resposta = (id + 1).to_le_bytes().to_vec();
                            resposta.extend_from_slice(&data[2..]);
                            let _ = envio.send(BusMessage::GameToClient {
                                roleid, localsid, data: resposta,
                            }).await;
                        }
                        BusMessage::PlayerLogout { roleid, .. } => {
                            self.sessoes.lock().await.remove(&roleid);
                        }
                        BusMessage::GameToClient { .. } => panic!("sentido invertido"),
                    }
                }
                saida = fila.recv() => {
                    let Some(msg) = saida else { break };
                    if conexao.enviar(msg).await.is_err() { break }
                }
            }
        }
    }
}

#[tokio::test]
async fn o_subcomando_do_cliente_chega_ao_mundo_e_a_resposta_volta() {
    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();

    let sessoes = Arc::new(Mutex::new(HashMap::new()));
    let mundo = MundoDeTeste {
        sessoes: Arc::clone(&sessoes),
    };

    let servidor = tokio::spawn(async move {
        let conexao = escuta.aceitar().await.unwrap();
        mundo.atender(conexao).await;
    });

    let mut link = BusClient::conectar(addr).await.unwrap();

    // 1. O jogador entrou no mundo: a partir daqui ele é deste servidor.
    link.enviar(BusMessage::EnterWorld {
        roleid: 1024,
        provider_link_id: 1,
        locktime: 0,
        timeout: 60,
        settime: 0,
        localsid: 0xABCD,
    })
    .await
    .unwrap();

    // 2. Um subcomando do mundo 3D. O `0x0F, 0x00` é `OBJECT_MOVE` (15) em
    //    little-endian — se o barramento ou o mundo lessem em big-endian, isso viraria
    //    3840 e nada bateria.
    link.enviar(BusMessage::ClientToGame {
        roleid: 1024,
        localsid: 0xABCD,
        data: vec![0x0F, 0x00, 0xDE, 0xAD],
    })
    .await
    .unwrap();

    // 3. A resposta volta endereçada ao mesmo jogador, com o `localsid` que o mundo
    //    guardou no EnterWorld — e não com o que veio na mensagem.
    let resposta = link.receber().await.unwrap().expect("sem resposta do mundo");
    assert_eq!(
        resposta,
        BusMessage::GameToClient {
            roleid: 1024,
            localsid: 0xABCD,
            data: vec![0x10, 0x00, 0xDE, 0xAD],
        }
    );

    // 4. O jogador sai e o mundo solta o registro.
    link.enviar(BusMessage::PlayerLogout {
        result: 0,
        roleid: 1024,
        provider_link_id: 1,
        localsid: 0xABCD,
    })
    .await
    .unwrap();

    drop(link);
    servidor.await.unwrap();
    assert!(
        sessoes.lock().await.is_empty(),
        "o mundo continuou com o jogador registrado depois do logout"
    );
}

#[tokio::test]
async fn dois_jogadores_no_mesmo_link_nao_se_misturam() {
    // Um daemon de link atende muitos jogadores por uma conexão só de barramento. O
    // `roleid` é o que separa um do outro — se ele se perdesse, um jogador receberia o
    // que é do outro, que é o pior tipo de bug para depurar em produção.
    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();

    let mundo = MundoDeTeste {
        sessoes: Arc::new(Mutex::new(HashMap::new())),
    };
    let servidor = tokio::spawn(async move {
        let conexao = escuta.aceitar().await.unwrap();
        mundo.atender(conexao).await;
    });

    let mut link = BusClient::conectar(addr).await.unwrap();

    for (roleid, localsid) in [(7, 0x1111u32), (99, 0x2222)] {
        link.enviar(BusMessage::EnterWorld {
            roleid,
            provider_link_id: 1,
            locktime: 0,
            timeout: 60,
            settime: 0,
            localsid,
        })
        .await
        .unwrap();
    }

    // Manda na ordem 99, 7 — a resposta tem que sair na mesma ordem, com o `localsid`
    // de cada um.
    for (roleid, cmd) in [(99i32, 0x20u16), (7, 0x30)] {
        link.enviar(BusMessage::ClientToGame {
            roleid,
            localsid: 0,
            data: cmd.to_le_bytes().to_vec(),
        })
        .await
        .unwrap();
    }

    let primeira = link.receber().await.unwrap().unwrap();
    let segunda = link.receber().await.unwrap().unwrap();

    assert_eq!(
        primeira,
        BusMessage::GameToClient {
            roleid: 99,
            localsid: 0x2222,
            data: 0x21u16.to_le_bytes().to_vec(),
        }
    );
    assert_eq!(
        segunda,
        BusMessage::GameToClient {
            roleid: 7,
            localsid: 0x1111,
            data: 0x31u16.to_le_bytes().to_vec(),
        }
    );

    drop(link);
    servidor.await.unwrap();
}
