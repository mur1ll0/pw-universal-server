//! O caminho de volta: o mundo manda algo, e aquilo chega ao jogador certo.
//!
//! O `pw-bus` já prova que os dois lados falam o mesmo formato. O que falta provar é a
//! parte que só existe no `pw-link`: **uma conexão de barramento, muitos jogadores**. Se
//! o desmultiplexador errar, um jogador recebe o pacote do outro — e isso não aparece em
//! nenhum teste de formato, só em jogo, como um bug que ninguém consegue reproduzir.
//!
//! O mundo aqui é um `BusListener` de verdade sobre TCP de verdade. O que ele responde é
//! irrelevante; o que importa é para onde a resposta vai.

use pw_bus::{BusListener, BusMessage};
use pw_link::uplink::BusUplink;
use pw_protocol::OutboundPacket;
use std::time::Duration;
use tokio::sync::mpsc;

/// Extrai os bytes de um `GamedataSend`, falhando alto em qualquer outra coisa.
fn bytes_do_gamedata(p: OutboundPacket) -> Vec<u8> {
    match p {
        OutboundPacket::GamedataSend(g) => g.data,
        outro => panic!("o uplink entregou {outro:?} em vez de um GamedataSend"),
    }
}

/// Espera um pacote com prazo, para que uma falha vire erro e não um teste travado.
async fn esperar(fila: &mut mpsc::Receiver<OutboundPacket>) -> OutboundPacket {
    tokio::time::timeout(Duration::from_secs(5), fila.recv())
        .await
        .expect("nada chegou ao jogador em 5s")
        .expect("a fila do jogador fechou")
}

#[tokio::test]
async fn o_que_o_mundo_manda_chega_ao_jogador_certo() {
    let escuta = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = escuta.local_addr().unwrap();

    // O mundo: devolve a cada `ClientToGame` um `GameToClient` para o mesmo `roleid`,
    // com o payload marcado para dar para distinguir um do outro.
    let mundo = tokio::spawn(async move {
        let mut conexao = escuta.aceitar().await.unwrap();
        while let Ok(Some(msg)) = conexao.receber().await {
            if let BusMessage::ClientToGame {
                roleid,
                localsid,
                data,
            } = msg
            {
                let mut resposta = data.clone();
                resposta.push(0xFF);
                if conexao
                    .enviar(BusMessage::GameToClient {
                        roleid,
                        localsid,
                        data: resposta,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let uplink = BusUplink::iniciar(addr.to_string());

    // Dois jogadores no mesmo link, cada um com a sua fila de saída — que é exatamente o
    // `tx` que o gateway já usa para escrever no socket do cliente.
    let (tx_a, mut fila_a) = mpsc::channel::<OutboundPacket>(16);
    let (tx_b, mut fila_b) = mpsc::channel::<OutboundPacket>(16);
    uplink.registrar(7, tx_a).await;
    uplink.registrar(99, tx_b).await;
    assert_eq!(uplink.jogadores_registrados().await, 2);

    // A conexão é feita em segundo plano: dá tempo dela subir antes de mandar.
    for _ in 0..100 {
        if uplink.enviar(BusMessage::ClientToGame {
            roleid: 7,
            localsid: 0x1111,
            data: vec![0x0F, 0x00, 0xAA],
        }) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    uplink.enviar(BusMessage::ClientToGame {
        roleid: 99,
        localsid: 0x2222,
        data: vec![0x20, 0x00, 0xBB],
    });

    // O de 7 recebe o payload de 7; o de 99, o de 99. Cruzar os dois é o bug que este
    // teste existe para pegar.
    assert_eq!(
        bytes_do_gamedata(esperar(&mut fila_a).await),
        vec![0x0F, 0x00, 0xAA, 0xFF]
    );
    assert_eq!(
        bytes_do_gamedata(esperar(&mut fila_b).await),
        vec![0x20, 0x00, 0xBB, 0xFF]
    );

    // Depois de sair, o jogador não recebe mais nada — nem por engano de roteamento.
    uplink.desregistrar(7).await;
    assert_eq!(uplink.jogadores_registrados().await, 1);

    drop(uplink);
    mundo.abort();
}

#[tokio::test]
async fn o_uplink_espera_o_mundo_subir() {
    // No `docker-compose` os dois contêineres sobem juntos. Se o link exigisse que o
    // mundo já estivesse pronto, a ordem de subida viraria uma condição de corrida — e o
    // sintoma seria um realm que funciona ou não conforme o humor da máquina.
    let sonda = BusListener::bind("127.0.0.1:0").await.unwrap();
    let addr = sonda.local_addr().unwrap();
    drop(sonda); // A porta fica livre: ninguém escutando, como um mundo que ainda não subiu.

    let uplink = BusUplink::iniciar(addr.to_string());

    // O mundo demora a subir.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let escuta = BusListener::bind(addr).await.expect("a porta deveria estar livre");

    let mundo = tokio::spawn(async move {
        let mut conexao = escuta.aceitar().await.unwrap();
        let msg = conexao.receber().await.unwrap().expect("nada chegou");
        assert_eq!(msg.roleid(), 42);
    });

    // A espera entre tentativas cresce (500ms, 1s, 2s...), então o prazo aqui é largo de
    // propósito: o que está sob teste é que a ligação **acontece**, não em quanto tempo.
    let mut conseguiu = false;
    for _ in 0..60 {
        if uplink.enviar(BusMessage::EnterWorld {
            roleid: 42,
            provider_link_id: 1,
            locktime: 0,
            timeout: 60,
            settime: 0,
            localsid: 1,
        }) {
            conseguiu = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(conseguiu, "o uplink nunca aceitou uma mensagem para a fila");

    tokio::time::timeout(Duration::from_secs(10), mundo)
        .await
        .expect("o mundo não recebeu nada: o uplink não reconectou")
        .unwrap();
}
