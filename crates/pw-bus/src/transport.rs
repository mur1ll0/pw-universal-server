//! Transporte TCP do barramento.
//!
//! Duas pontas simétricas: o `pw-gs` escuta ([`BusListener`]), o `pw-link` conecta
//! ([`BusClient`]). O que trafega é sempre [`BusMessage`]; o enquadramento fica com o
//! [`BusCodec`].
//!
//! O barramento é entre daemons, dentro da infraestrutura do servidor — não é uma porta
//! exposta ao jogador. Quem cuidar do `docker-compose` precisa mantê-la assim.

use crate::codec::{BusCodec, BusError};
use crate::message::BusMessage;
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_util::codec::Framed;
use tracing::{debug, info};

/// Uma conexão de barramento já enquadrada, de qualquer uma das pontas.
pub struct BusConnection {
    framed: Framed<TcpStream, BusCodec>,
    par: String,
}

impl BusConnection {
    pub fn new(stream: TcpStream) -> Self {
        let par = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "?".to_string());
        Self {
            framed: Framed::new(stream, BusCodec),
            par,
        }
    }

    /// Endereço da outra ponta, para log.
    pub fn par(&self) -> &str {
        &self.par
    }

    pub async fn enviar(&mut self, msg: BusMessage) -> Result<(), BusError> {
        debug!(
            "barramento → {}: opcode {} (roleid {})",
            self.par,
            msg.opcode(),
            msg.roleid()
        );
        self.framed.send(msg).await
    }

    /// Espera a próxima mensagem.
    ///
    /// `Ok(None)` é fim de conexão. Um quadro cujo opcode não é do barramento é
    /// descartado pelo codec e a espera continua — não encerra a conexão, porque o
    /// quadro foi consumido por inteiro e o fluxo segue alinhado.
    pub async fn receber(&mut self) -> Result<Option<BusMessage>, BusError> {
        match self.framed.next().await {
            Some(Ok(m)) => Ok(Some(m)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

/// A ponta que escuta — o `pw-gs`.
pub struct BusListener {
    listener: TcpListener,
}

impl BusListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, BusError> {
        let listener = TcpListener::bind(addr).await?;
        if let Ok(local) = listener.local_addr() {
            info!("barramento escutando em {local}");
        }
        Ok(Self { listener })
    }

    /// Endereço efetivo. Útil quando se pede a porta 0 e o sistema escolhe — é o que os
    /// testes usam para não depender de uma porta fixa estar livre.
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, BusError> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn aceitar(&self) -> Result<BusConnection, BusError> {
        let (stream, addr) = self.listener.accept().await?;
        info!("barramento: daemon conectou de {addr}");
        Ok(BusConnection::new(stream))
    }
}

/// A ponta que conecta — o `pw-link`.
pub struct BusClient;

impl BusClient {
    pub async fn conectar<A: ToSocketAddrs>(addr: A) -> Result<BusConnection, BusError> {
        let stream = TcpStream::connect(addr).await?;
        // O barramento carrega comandos de movimento e combate: agrupar pacotes
        // pequenos acrescentaria latência exatamente onde ela é percebida.
        let _ = stream.set_nodelay(true);
        Ok(BusConnection::new(stream))
    }
}
