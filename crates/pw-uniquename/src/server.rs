use crate::service::UniqueNameService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum UniqueNameRequest {
    CheckRoleName {
        realm_id: String,
        name: String,
        is_gm: bool,
    },
    CheckFactionName {
        realm_id: String,
        name: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", content = "result")]
pub enum UniqueNameResponse {
    Available,
    Error(String),
}

pub struct UniqueNameServer {
    service: Arc<UniqueNameService>,
    listen_port: u16,
}

impl UniqueNameServer {
    pub fn new(service: UniqueNameService, listen_port: u16) -> Self {
        Self {
            service: Arc::new(service),
            listen_port,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        info!("pw-uniquename daemon escutando em {}", addr);

        loop {
            let (socket, remote_addr) = listener.accept().await?;
            let service = self.service.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, service).await {
                    warn!("Erro na conexão de uniquename com {}: {:?}", remote_addr, e);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, service: Arc<UniqueNameService>) -> anyhow::Result<()> {
    let mut buffer = vec![0u8; 4096];
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let req_str = std::str::from_utf8(&buffer[..n])?;
    let req: UniqueNameRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(e) => {
            let res = UniqueNameResponse::Error(format!("Requisição inválida: {}", e));
            let bytes = serde_json::to_vec(&res)?;
            socket.write_all(&bytes).await?;
            return Ok(());
        }
    };

    let response = match req {
        UniqueNameRequest::CheckRoleName {
            realm_id,
            name,
            is_gm,
        } => match service.check_character_name(&realm_id, &name, is_gm).await {
            Ok(_) => UniqueNameResponse::Available,
            Err(e) => UniqueNameResponse::Error(e.to_string()),
        },
        UniqueNameRequest::CheckFactionName { realm_id, name } => {
            match service.check_faction_name(&realm_id, &name).await {
                Ok(_) => UniqueNameResponse::Available,
                Err(e) => UniqueNameResponse::Error(e.to_string()),
            }
        }
    };

    let bytes = serde_json::to_vec(&response)?;
    socket.write_all(&bytes).await?;
    Ok(())
}
