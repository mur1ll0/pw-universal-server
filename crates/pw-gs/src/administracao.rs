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
    /// Gold em unidades do cash (100 = 1 gold na Loja Gold). Só positivo: o painel dá,
    /// não tira (decisão do Murillo, B180 — tirar arrisca saldo negativo).
    AjustarGold { conta_id: i32, delta: i64 },
    DefinirBan { conta_id: i32, banida: bool, #[serde(default)] motivo: Option<String> },
    Desconectar { conta_id: i32 },
    AtualizarCash { conta_id: i32 },
    /// Rates do realm (E7): grava em `realms` e passa a valer na hora neste processo.
    DefinirTaxas { exp: f64, sp: f64, drop: f64, moedas: f64 },
    /// Liga/desliga um mapa deste realm (E7). Só o processo que serve o mapa aplica; com
    /// `carregar`, este processo também aceita um mapa que não carrega (B183: ligar monta).
    DefinirMapa { mapa: i32, ligado: bool, #[serde(default)] carregar: bool },
    /// Edição de personagem (E5): um só de `dinheiro` (dar), `exp`/`sp` (somar, só online),
    /// `pontos` (dar pontos livres), `nivel` (alvo, só sobe) ou `cultivo` (B182).
    EditarPersonagem {
        personagem_id: i32,
        #[serde(default)] dinheiro: Option<i64>,
        #[serde(default)] exp: Option<i64>,
        #[serde(default)] sp: Option<i64>,
        #[serde(default)] pontos: Option<i64>,
        #[serde(default)] nivel: Option<i64>,
        #[serde(default)] cultivo: Option<i64>,
        /// Força, agilidade, vitalidade e energia novas (B184).
        #[serde(default)] atributos: Option<Vec<i64>>,
        /// `true`: devolve todos os atributos aos pontos livres (B184).
        #[serde(default)] redistribuir: Option<bool>,
        /// Mapa e posição (E6, B185).
        #[serde(default)] posicao: Option<PosicaoPedida>,
        /// Dar item (E6, B186): `{id, quantidade}`.
        #[serde(default)] item: Option<ItemPedido>,
        /// Tirar item de um slot (E6, B187).
        #[serde(default)] remover_item: Option<RemocaoPedida>,
    },
    /// Inventário de um personagem, com nomes (E6, B186). Só leitura.
    Inventario { personagem_id: i32 },
    /// Itens do `elements.data` por nome ou id (E6, B186). Só leitura.
    BuscarItens { texto: String },
    /// O item de um slot com os dados da dica (E6, B189). Só leitura.
    DetalheItem { personagem_id: i32, recipiente: String, slot: u16 },
}

/// Teto de uma edição de dinheiro/EXP pelo painel: cabe no `u32`/`i32` dos pacotes
/// (`task_deliver_money`, `task_deliver_exp`) e no teto de dinheiro do mundo.
const TETO_DA_EDICAO: i64 = 2_000_000_000;
/// Teto de pontos livres por operação: política do painel, não regra do jogo (B182).
const TETO_DE_PONTOS_POR_EDICAO: i64 = 10_000;
/// Destino pedido pelo painel (B185); `y` ausente = o chão.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PosicaoPedida {
    pub mapa: i32,
    pub x: f32,
    #[serde(default)]
    pub y: Option<f32>,
    pub z: f32,
}

/// Item pedido pelo painel (B186).
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemPedido {
    pub id: u32,
    pub quantidade: u32,
}

/// Remoção pedida pelo painel (B187): `recipiente` é `bolsa`, `missao`, `equipamento` ou
/// `armazem`; `quantidade` ausente = a pilha inteira; `id` confere o item do slot.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemocaoPedida {
    pub recipiente: String,
    pub slot: u16,
    pub id: u32,
    #[serde(default)]
    pub quantidade: Option<u32>,
}

/// Nome do recipiente no painel → tipo no banco (B187).
fn recipiente_do_painel(nome: &str) -> Option<pw_core::ContainerType> {
    match nome {
        "bolsa" => Some(pw_core::ContainerType::Inventory),
        "missao" => Some(pw_core::ContainerType::TaskInventory),
        "equipamento" => Some(pw_core::ContainerType::Equipment),
        "armazem" => Some(pw_core::ContainerType::Storehouse),
        _ => None,
    }
}

/// Teto de quantidade de um item por operação: política do painel (B186).
const TETO_DE_QUANTIDADE: u32 = 100_000;

/// Teto de coordenada pelo painel: política (os mapas do PW cabem em ±50 000), não regra.
const TETO_DE_COORDENADA: f32 = 100_000.0;

/// Teto de um atributo pelo painel: política, não regra do jogo (B184).
const TETO_DE_ATRIBUTO: i64 = 100_000;

/// Teto de um ajuste de gold: o `PLAYER_CASH` leva `i32` (`bus_server.rs::saldo`).
const TETO_DO_AJUSTE_DE_GOLD: i64 = i32::MAX as i64;
/// Motivo do banimento: texto curto para `accounts.ban_reason`.
const TETO_DO_MOTIVO: usize = 120;

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
                match resultado {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => tracing::debug!(erro = %e, "admin: consulta recusada ou conexão interrompida"),
                    Err(_) => tracing::debug!("admin: consulta sem resposta em 3 s"),
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
                    Consulta::AjustarGold { conta_id, delta }
                        if conta_id > 0 && delta > 0 && delta <= TETO_DO_AJUSTE_DE_GOLD =>
                    {
                        let impressao = Sha256::digest(serde_json::to_vec(&json!({
                            "tipo":"ajustar_gold","conta_id":conta_id,"delta":delta,
                            "administrador_id":pedido.administrador_id
                        }))?);
                        let resultado = self.contas.comandos_administrativos()
                            .ajustar_gold(&pedido.operacao_id, pedido.administrador_id, &self.realm,
                                conta_id, delta, &impressao).await?;
                        tracing::info!(operacao=%pedido.operacao_id, conta=conta_id, delta,
                            estado=%resultado["estado"], "admin: ajustar_gold global");
                        resultado
                    }
                    Consulta::DefinirBan { conta_id, banida, motivo }
                        if conta_id > 0 && motivo.as_ref().is_none_or(|m| {
                            m.chars().count() <= TETO_DO_MOTIVO && !m.chars().any(char::is_control)
                        }) =>
                    {
                        let impressao = Sha256::digest(serde_json::to_vec(&json!({
                            "tipo":"definir_ban","conta_id":conta_id,"banida":banida,"motivo":motivo,
                            "administrador_id":pedido.administrador_id
                        }))?);
                        let resultado = self.contas.comandos_administrativos()
                            .definir_ban(&pedido.operacao_id, pedido.administrador_id, &self.realm,
                                conta_id, banida, motivo.as_deref(), &impressao).await?;
                        tracing::info!(operacao=%pedido.operacao_id, conta=conta_id, banida,
                            estado=%resultado["estado"], "admin: definir_ban global");
                        resultado
                    }
                    Consulta::Desconectar { conta_id } if conta_id > 0 => {
                        let resultado = self.roteador.desconectar_conta(conta_id).await;
                        tracing::info!(conta=conta_id, desconectados=%resultado["desconectados"],
                            "admin: desconectar conta");
                        resultado
                    }
                    Consulta::AtualizarCash { conta_id } if conta_id > 0 => {
                        self.roteador.atualizar_cash_da_conta(conta_id).await
                    }
                    Consulta::DefinirTaxas { exp, sp, drop, moedas } => {
                        let taxas = crate::taxas::Taxas { exp, sp, drop, moedas }.arredondadas();
                        if !taxas.validas() {
                            json!({"codigo":"taxas_invalidas"})
                        } else {
                            self.contas.realms()
                                .update_multipliers(&self.realm, taxas.exp as f32, taxas.sp as f32, taxas.drop as f32, taxas.moedas as f32)
                                .await?;
                            self.roteador.definir_taxas(taxas);
                            tracing::info!(?taxas, administrador = pedido.administrador_id,
                                "admin: rates do realm {} trocadas", self.realm);
                            json!({"estado":"aplicado","tipo":"definir_taxas","taxas":taxas})
                        }
                    }
                    Consulta::Inventario { personagem_id } if personagem_id > 0 => {
                        self.roteador.inventario_do_painel(personagem_id).await
                    }
                    Consulta::DetalheItem { personagem_id, recipiente, slot } if personagem_id > 0 => {
                        match recipiente_do_painel(&recipiente) {
                            Some(r) => self.roteador.detalhe_do_item(personagem_id, r, slot).await,
                            None => json!({"codigo":"recipiente_invalido"}),
                        }
                    }
                    Consulta::BuscarItens { texto } if texto.chars().count() <= 64 => {
                        self.roteador.buscar_itens(&texto).await
                    }
                    Consulta::EditarPersonagem { personagem_id, dinheiro, exp, sp, pontos, nivel, cultivo, atributos, redistribuir, posicao, item, remover_item } if personagem_id > 0 => {
                        use crate::bus_server::EdicaoDePersonagem as E;
                        let dentro = |v: i64| v.abs() <= TETO_DA_EDICAO;
                        let tipos = [dinheiro.is_some(), exp.or(sp).is_some(), pontos.is_some(), nivel.is_some(), cultivo.is_some(),
                            atributos.is_some(), redistribuir.is_some(), posicao.is_some(), item.is_some(), remover_item.is_some()]
                            .iter().filter(|t| **t).count();
                        // Atributos: quatro valores entre 0 e o teto da política (B184).
                        let quatro = atributos.as_deref().and_then(|v| {
                            (v.len() == 4 && v.iter().all(|x| (0..=TETO_DE_ATRIBUTO).contains(x)))
                                .then(|| [v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32])
                        });
                        let coordenada = |v: f32| v.is_finite() && v.abs() <= TETO_DE_COORDENADA;
                        let edicao = if tipos != 1 { None }
                            else if let Some(r) = &remover_item {
                                let recipiente = recipiente_do_painel(&r.recipiente);
                                recipiente.filter(|_| r.id > 0 && r.slot < 256 && r.quantidade.map_or(true, |q| (1..=TETO_DE_QUANTIDADE).contains(&q)))
                                    .map(|recipiente| E::RemoverItem { recipiente, slot: r.slot, tid: r.id, quantidade: r.quantidade })
                            }
                            else if let Some(i) = item {
                                (i.id > 0 && (1..=TETO_DE_QUANTIDADE).contains(&i.quantidade))
                                    .then_some(E::Item { tid: i.id, quantidade: i.quantidade })
                            }
                            else if let Some(p) = posicao {
                                (p.mapa > 0 && coordenada(p.x) && coordenada(p.z) && p.y.map_or(true, coordenada))
                                    .then_some(E::Posicao { mapa: p.mapa, x: p.x, y: p.y, z: p.z })
                            }
                            else if atributos.is_some() { quatro.map(|q| E::Atributos(Some(q))) }
                            else if redistribuir.is_some() { (redistribuir == Some(true)).then_some(E::Atributos(None)) }
                            else { match (dinheiro, exp, sp, pontos, nivel, cultivo) {
                            // Só dar (B180): dinheiro negativo é edição inválida.
                            (Some(d), ..) if d > 0 && dentro(d) => Some(E::Dinheiro(d)),
                            (None, e, s, None, None, None) if e.or(s).is_some() => {
                                let (e, s) = (e.unwrap_or(0), s.unwrap_or(0));
                                (e >= 0 && s >= 0 && e + s > 0 && dentro(e) && dentro(s)).then_some(E::Experiencia { exp: e, sp: s })
                            }
                            (.., Some(p), None, None) if (1..=TETO_DE_PONTOS_POR_EDICAO).contains(&p) => Some(E::PontosLivres(p)),
                            // Teto de nível e cultivo da versão: o roteador confere (dependem do realm).
                            (.., Some(n), None) if (2..=i32::MAX as i64).contains(&n) => Some(E::Nivel(n as i32)),
                            (.., Some(c)) if (0..=255).contains(&c) => Some(E::Cultivo(c as i32)),
                            _ => None,
                        }};
                        match edicao {
                            None => json!({"codigo":"edicao_invalida"}),
                            Some(edicao) => {
                                let mut parametros = json!({
                                    "tipo":"editar_personagem","personagem_id":personagem_id,
                                    "dinheiro":dinheiro,"exp":exp,"sp":sp,
                                    "administrador_id":pedido.administrador_id
                                });
                                // Chaves do B182 só quando presentes: a impressão de uma
                                // operação B179 pendente continua a mesma.
                                for (chave, v) in [("pontos", pontos), ("nivel", nivel), ("cultivo", cultivo)] {
                                    if let Some(v) = v { parametros[chave] = json!(v); }
                                }
                                if let Some(a) = &atributos { parametros["atributos"] = json!(a); }
                                if let Some(r) = redistribuir { parametros["redistribuir"] = json!(r); }
                                if let Some(p) = posicao { parametros["posicao"] = json!(p); }
                                if let Some(i) = item { parametros["item"] = json!(i); }
                                if let Some(r) = &remover_item { parametros["remover_item"] = json!(r); }
                                let impressao = Sha256::digest(serde_json::to_vec(&parametros)?);
                                let repo = self.contas.comandos_administrativos();
                                match repo.reservar_operacao_de_personagem(&pedido.operacao_id, pedido.administrador_id, &self.realm, &impressao).await? {
                                    Some(anterior) => anterior,
                                    None => {
                                        let mut r = self.roteador.editar_personagem(personagem_id, edicao).await;
                                        r["tipo"] = json!("editar_personagem");
                                        r["personagem_id"] = json!(personagem_id);
                                        repo.gravar_resultado_de_personagem(&pedido.operacao_id, &r).await?;
                                        tracing::info!(operacao=%pedido.operacao_id, personagem=personagem_id,
                                            ?edicao, estado=%r["estado"], "admin: editar personagem");
                                        r
                                    }
                                }
                            }
                        }
                    }
                    Consulta::DefinirMapa { mapa, ligado, carregar } => {
                        self.roteador.definir_mapa(&self.contas.realms(), &self.realm, mapa, ligado, carregar).await
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

#[cfg(test)]
mod testes_de_formato {
    use super::*;

    /// B185: o pedido de posição passa pela leitura do canal (enum com `tag` interno).
    #[test]
    fn pedido_de_posicao_e_lido() {
        let r: Result<PedidoAdministrativo, _> = serde_json::from_str(r#"{"operacao_id":"x","realm_id":"r","administrador_id":1,"consulta":{"tipo":"editar_personagem","personagem_id":1,"posicao":{"mapa":1,"x":120.5,"y":30.0,"z":-80.0}}}"#);
        assert!(r.is_ok(), "{:?}", r.err());
    }
}
