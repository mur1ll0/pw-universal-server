use crate::service::AuthService;
use pw_core::AccountId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum AuthRequest {
    Login {
        username: String,
        password: String,
        client_ip: String,
        realm_id: String,
    },
    Register {
        username: String,
        password: String,
        email: Option<String>,
    },
    AddGold {
        account_id: AccountId,
        amount: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", content = "result")]
pub enum AuthResponse {
    Ok(serde_json::Value),
    Error(String),
}

pub struct AuthServer {
    service: Arc<AuthService>,
    listen_port: u16,
}

impl AuthServer {
    pub fn new(service: AuthService, listen_port: u16) -> Self {
        Self {
            service: Arc::new(service),
            listen_port,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        info!("pw-auth (Serviço de Autenticação) escutando em {}", addr);

        loop {
            let (socket, remote_addr) = listener.accept().await?;
            let service = self.service.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, service).await {
                    warn!("Erro na conexão de {}: {:?}", remote_addr, e);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, service: Arc<AuthService>) -> anyhow::Result<()> {
    let mut buffer = vec![0u8; 4096];
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let req_str = std::str::from_utf8(&buffer[..n])?;
    let req: AuthRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(e) => {
            let res = AuthResponse::Error(format!("Requisição inválida: {}", e));
            let bytes = serde_json::to_vec(&res)?;
            socket.write_all(&bytes).await?;
            return Ok(());
        }
    };

    let response = match req {
        AuthRequest::Login {
            username,
            password,
            client_ip,
            realm_id,
        } => match service.authenticate(&username, &password, &client_ip, &realm_id).await {
            Ok(login_res) => {
                AuthResponse::Ok(serde_json::to_value(login_res)?)
            }
            Err(e) => AuthResponse::Error(e.to_string()),
        },
        AuthRequest::Register {
            username,
            password,
            email,
        } => match service.register(&username, &password, email).await {
            Ok(acc_id) => {
                AuthResponse::Ok(serde_json::json!({ "account_id": acc_id }))
            }
            Err(e) => AuthResponse::Error(e.to_string()),
        },
        AuthRequest::AddGold { account_id, amount } => {
            match service.add_gold(account_id, amount).await {
                Ok(new_balance) => {
                    AuthResponse::Ok(serde_json::json!({ "new_balance": new_balance }))
                }
                Err(e) => AuthResponse::Error(e.to_string()),
            }
        }
    };

    let bytes = serde_json::to_vec(&response)?;
    socket.write_all(&bytes).await?;
    Ok(())
}
