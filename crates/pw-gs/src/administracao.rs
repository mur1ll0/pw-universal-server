//! Canal administrativo separado do barramento de jogo e sem porta pública.
//! HMAC de desafio por conexão protege pedido/resposta de falsificação e replay.
use crate::{entity::PlayerEntity, RoteadorDeMapas};
use hmac::{Hmac, Mac};
use pw_storage::AccountRepository;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
};

pub const LIMITE_QUADRO: usize = 8192;
type Assinatura = Hmac<Sha256>;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PedidoAdministrativo {
    pub operacao_id: String,
    pub realm_id: String,
    pub administrador_id: i32,
    pub consulta: Consulta,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case", deny_unknown_fields)]
pub enum Consulta {
    Mundos,
    Personagem { personagem_id: i32 },
    Resultado { comando_id: String },
    TrocarSenha { conta_id: i32, senha: String },
    CriarConta { usuario: String, senha: String },
    DefinirGm { conta_id: i32, habilitado: bool },
}

#[derive(Debug, Serialize)]
pub struct FichaViva {
    pub id: i32,
    pub nome: String,
    pub classe: i32,
    pub nivel: i32,
    pub cultivo: i32,
    // Inteiros de 64 bits como texto: o navegador não pode perder precisão.
    pub exp: String,
    pub alma: String,
    pub dinheiro: String,
    pub vida: i32,
    pub vida_maxima: i32,
    pub mana: i32,
    pub mana_maxima: i32,
    pub forca: i32,
    pub agilidade: i32,
    pub vitalidade: i32,
    pub energia: i32,
    pub pontos: i32,
    pub mapa: i32,
    pub posicao: pw_core::Vector3,
    pub gm: u8,
    pub gm_invencivel: bool,
    pub gm_invisivel: bool,
}

impl FichaViva {
    pub fn do_jogador(jogador: &PlayerEntity, mapa: i32) -> Self {
        // Fotografia direta de PlayerEntity; nenhuma fórmula ou layout de cliente novo.
        Self {
            id: jogador.role_id,
            nome: jogador.name.clone(),
            classe: jogador.cls as i32,
            nivel: jogador.level,
            cultivo: jogador.cultivation,
            exp: jogador.exp.to_string(),
            alma: jogador.sp.to_string(),
            dinheiro: jogador.money.to_string(),
            vida: jogador.hp,
            vida_maxima: jogador.max_hp,
            mana: jogador.mp,
            mana_maxima: jogador.max_mp,
            forca: jogador.strength,
            agilidade: jogador.agility,
            vitalidade: jogador.vitality,
            energia: jogador.energy,
            pontos: jogador.pontos_de_atributo,
            mapa,
            posicao: jogador.position,
            gm: jogador.sec_level,
            gm_invencivel: jogador.efeitos.gm_invencivel,
            gm_invisivel: jogador.efeitos.gm_invisivel,
        }
    }
}

pub struct ServidorAdministrativo {
    realm: String,
    segredo: Vec<u8>,
    roteador: Arc<RoteadorDeMapas>,
    contas: AccountRepository,
}

impl ServidorAdministrativo {
    pub fn new(
        realm: String,
        segredo: Vec<u8>,
        roteador: Arc<RoteadorDeMapas>,
        contas: AccountRepository,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            segredo.len() == 32,
            "ADMIN_SECRET exige 32 bytes em hexadecimal"
        );
        Ok(Self {
            realm,
            segredo,
            roteador,
            contas,
        })
    }

    pub async fn executar(self: Arc<Self>, escuta: TcpListener) -> anyhow::Result<()> {
        let vagas = Arc::new(Semaphore::new(32));
        loop {
            let (socket, _) = escuta.accept().await?;
            let Ok(vaga) = Arc::clone(&vagas).try_acquire_owned() else {
                continue;
            };
            let servidor = Arc::clone(&self);
            tokio::spawn(async move {
                let _vaga = vaga;
                // Limite por conexão, incluindo banco; nunca dentro do world.tick.
                let resultado =
                    tokio::time::timeout(Duration::from_secs(3), servidor.atender(socket)).await;
                if !matches!(resultado, Ok(Ok(()))) {
                    tracing::debug!("admin: consulta recusada ou conexão interrompida");
                }
            });
        }
    }

    async fn atender(&self, mut socket: TcpStream) -> anyhow::Result<()> {
        let mut desafio = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut desafio);
        escrever_quadro(
            &mut socket,
            &serde_json::to_vec(&json!({"protocolo":1,"desafio":hex::encode(desafio)}))?,
        )
        .await?;
        let corpo = ler_quadro(&mut socket).await?;
        let mut assinatura = [0u8; 32];
        socket.read_exact(&mut assinatura).await?;
        conferir_assinatura(&self.segredo, &desafio, b"pedido", &corpo, &assinatura)?;
        let pedido: PedidoAdministrativo = serde_json::from_slice(&corpo)?;
        anyhow::ensure!(
            !pedido.operacao_id.is_empty()
                && pedido.operacao_id.len() <= 64
                && pedido
                    .operacao_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "id de operação inválido"
        );

        let dados = if pedido.realm_id != self.realm {
            json!({"codigo":"realm_incorreto"})
        } else {
            let conta = self.contas.find_by_id(pedido.administrador_id).await?;
            if !conta.is_some_and(|conta| conta.gm_privileges > 0 && !conta.is_banned) {
                json!({"codigo":"administrador_recusado"})
            } else {
                match pedido.consulta {
                    Consulta::DefinirGm {
                        conta_id,
                        habilitado,
                    } if conta_id > 0 => {
                        let processos = pw_storage::alvos_coordenacao_gm();
                        let impressao = Sha256::digest(serde_json::to_vec(&json!({
                            "tipo":"definir_gm","conta_id":conta_id,"habilitado":habilitado,
                            "administrador_id":pedido.administrador_id,"processos":processos
                        }))?);
                        let resultado = self
                            .contas
                            .comandos_administrativos()
                            .definir_gm(
                                &pedido.operacao_id,
                                pedido.administrador_id,
                                &self.realm,
                                conta_id,
                                habilitado,
                                &impressao,
                                &processos,
                            )
                            .await?;
                        tracing::info!(operacao=%pedido.operacao_id,conta=conta_id,
                            estado=%resultado["estado"],"admin: definir_gm global");
                        resultado
                    }
                    Consulta::Mundos => self.roteador.resumo_administrativo().await,
                    Consulta::Personagem { personagem_id } if personagem_id > 0 => {
                        self.roteador
                            .consultar_administrativamente(personagem_id)
                            .await
                    }
                    Consulta::Resultado { comando_id } if id_valido(&comando_id) => {
                        self.contas
                            .comandos_administrativos()
                            .consultar(&comando_id, pedido.administrador_id)
                            .await?
                    }
                    Consulta::CriarConta { usuario, senha }
                        if usuario_valido(&usuario) && senha_valida(&senha) =>
                    {
                        // Cliente ElementClient/EC_LoginSwitch.cpp:329: MakeLower.
                        // Network/gameclient.cpp:131-139: chave MD5(nome+senha), algo=0.
                        let usuario = usuario.to_ascii_lowercase();
                        let hash = pw_crypto::hash_legacy_pw_md5(&usuario, &senha);
                        let impressao = Sha256::digest(serde_json::to_vec(&json!({
                            "tipo":"criar_conta", "usuario":usuario,
                            "administrador_id":pedido.administrador_id,
                            "senha_resumida":pw_crypto::hash_raw_md5(&senha)
                        }))?);
                        let resultado = self
                            .contas
                            .comandos_administrativos()
                            .criar_conta(
                                &pedido.operacao_id,
                                pedido.administrador_id,
                                &self.realm,
                                &usuario,
                                &impressao,
                                &hash,
                            )
                            .await?;
                        tracing::info!(operacao=%pedido.operacao_id,
                            administrador=pedido.administrador_id, conta=?resultado["conta_id"],
                            estado=%resultado["estado"], "admin: criar_conta global");
                        resultado
                    }
                    Consulta::TrocarSenha { conta_id, senha }
                        if conta_id > 0 && senha_valida(&senha) =>
                    {
                        let alvo = self.contas.find_by_id(conta_id).await?;
                        // Contrato legado consumido pelo Challenge algo=0 do link:
                        // pw-crypto/password.rs: hash_legacy_pw_md5; cliente
                        // Network/gameclient.cpp:131-139 (MD5(nome+senha) como chave HMAC).
                        let hash = pw_crypto::hash_legacy_pw_md5(
                            alvo.as_ref().map_or("", |c| c.username.as_str()),
                            &senha,
                        );
                        // Realm é rota, não escopo: o mesmo ID deduplica em outro GS/realm.
                        let impressao = Sha256::digest(serde_json::to_vec(&json!({
                            "tipo":"trocar_senha", "conta_id":conta_id,
                            "administrador_id":pedido.administrador_id,
                            "senha_resumida":pw_crypto::hash_raw_md5(&senha)
                        }))?);
                        let resultado = self
                            .contas
                            .comandos_administrativos()
                            .trocar_senha(
                                &pedido.operacao_id,
                                pedido.administrador_id,
                                &self.realm,
                                conta_id,
                                &impressao,
                                &hash,
                            )
                            .await?;
                        tracing::info!(operacao=%pedido.operacao_id, conta=conta_id,
                            administrador=pedido.administrador_id,
                            estado=%resultado["estado"], "admin: trocar_senha global");
                        resultado
                    }
                    _ => json!({"codigo":"alvo_invalido"}),
                }
            }
        };
        let resposta = serde_json::to_vec(&json!({
            "operacao_id": pedido.operacao_id, "realm_id": self.realm,
            "estado": dados.get("estado").and_then(|v| v.as_str()).unwrap_or(
                if dados.get("codigo").is_some() {"falha"} else {"consultado"}), "dados":dados,
        }))?;
        escrever_quadro(&mut socket, &resposta).await?;
        socket
            .write_all(&assinar(&self.segredo, &desafio, b"resposta", &resposta))
            .await?;
        Ok(())
    }
}

fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn senha_valida(senha: &str) -> bool {
    // Política administrativa; AString do cliente usa bytes de página de código.
    // ASCII evita prometer compatibilidade de codificação ainda não comprovada.
    !senha.is_empty() && senha.len() <= 64 && senha.bytes().all(|b| (32..=126).contains(&b))
}

fn usuario_valido(usuario: &str) -> bool {
    // Política do painel (1–64 bytes, capacidade de accounts.username).
    // ASCII evita divergência AString/página de código: gameclient.cpp:134,150.
    !usuario.is_empty()
        && usuario.len() <= 64
        && usuario
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn assinar(segredo: &[u8], desafio: &[u8], sentido: &[u8], corpo: &[u8]) -> Vec<u8> {
    let mut mac =
        Assinatura::new_from_slice(segredo).expect("HMAC aceita qualquer tamanho de chave");
    mac.update(desafio);
    mac.update(sentido);
    mac.update(corpo);
    mac.finalize().into_bytes().to_vec()
}

pub fn conferir_assinatura(
    segredo: &[u8],
    desafio: &[u8],
    sentido: &[u8],
    corpo: &[u8],
    assinatura: &[u8],
) -> anyhow::Result<()> {
    let mut mac = Assinatura::new_from_slice(segredo)?;
    mac.update(desafio);
    mac.update(sentido);
    mac.update(corpo);
    mac.verify_slice(assinatura)
        .map_err(|_| anyhow::anyhow!("assinatura recusada"))
}

pub async fn ler_quadro(socket: &mut TcpStream) -> anyhow::Result<Vec<u8>> {
    let tamanho = socket.read_u32().await? as usize;
    anyhow::ensure!(
        tamanho > 0 && tamanho <= LIMITE_QUADRO,
        "quadro administrativo fora do limite"
    );
    let mut corpo = vec![0; tamanho];
    socket.read_exact(&mut corpo).await?;
    Ok(corpo)
}

pub async fn escrever_quadro(socket: &mut TcpStream, corpo: &[u8]) -> anyhow::Result<()> {
    anyhow::ensure!(
        !corpo.is_empty() && corpo.len() <= LIMITE_QUADRO,
        "resposta administrativa fora do limite"
    );
    socket.write_u32(corpo.len() as u32).await?;
    socket.write_all(corpo).await?;
    Ok(())
}
