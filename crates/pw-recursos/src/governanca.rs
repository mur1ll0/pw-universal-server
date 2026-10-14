//! Rascunhos, publicação e reversão das versões do cliente (P3, B235) — §3 do
//! `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`.
//!
//! - Um **rascunho** junta itens por caminho: `adicionar`/`substituir` (com o conteúdo novo
//!   guardado como objeto) ou `remover`. Um caminho só pode estar em um rascunho por vez.
//! - **Publicar** gera a versão N+1 = manifesto N + itens dos rascunhos, assinada; os pacotes
//!   continuam os da versão em que nasceram (`Pacote::numero`) e o que mudou chega ao launcher
//!   por objeto. Versão publicada nunca muda.
//! - **Reverter** cria um rascunho com o inverso de uma alteração publicada (os objetos antigos
//!   continuam guardados: `objetos_em_uso` conta o "antes" de cada item).

use crate::assinatura::Chave;
use crate::importar::{gravar_assinado, pacote_de, pasta_da_versao};
use crate::manifesto::{self, Arquivo, Manifesto, Versao};
use crate::nomes;
use crate::objetos::{sha256, Objetos, Sha256Bytes};
use anyhow::{bail, Context, Result};
use pw_storage::{ItemDeRecurso, RecursosRepository};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Resultado de uma publicação: o número e os arquivos gravados nos realms.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Publicacao {
    pub numero: u32,
    pub aplicados: Vec<String>,
}

pub struct Governanca {
    pub dados: PathBuf,
    pub repo: RecursosRepository,
    pub chave: Arc<Chave>,
    /// Pastas `config` dos realms de cada versão de cliente (`PW_RECURSOS_REALMS`): publicar um
    /// item de destino `servidor`/`ambos` grava o arquivo nelas (B237).
    pub realms: BTreeMap<String, Vec<PathBuf>>,
}

/// Caminho resolvido contra o manifesto publicado: os bytes exatos (os do manifesto, se o
/// arquivo existe — sem diferenciar maiúsculas, como o Windows) e o arquivo atual.
pub struct Caminho {
    pub gbk: Vec<u8>,
    pub atual: Option<Arquivo>,
}

impl Governanca {
    pub(crate) fn objetos(&self, versao: &str) -> Objetos {
        Objetos::new(pasta_da_versao(&self.dados, versao).join("objetos"), 12)
    }

    /// `versao.json` e manifesto publicados, lidos do volume.
    pub fn publicado(&self, versao: &str) -> Result<(Versao, Manifesto)> {
        let base = pasta_da_versao(&self.dados, versao).join("atualizacao");
        let v: Versao = serde_json::from_slice(
            &std::fs::read(base.join("versao.json")).with_context(|| format!("a versão {versao} não foi importada"))?,
        )?;
        let m: Manifesto = serde_json::from_slice(&std::fs::read(base.join(&v.manifesto))?)?;
        Ok((v, m))
    }

    /// Texto do caminho (como o GM digita: `configs\x.txt` ou `configs/x.txt`) → bytes GBK.
    pub fn resolver(m: &Manifesto, texto: &str) -> Result<Caminho> {
        let texto = texto.trim().replace('/', "\\");
        let texto = texto.trim_start_matches('\\');
        let gbk = nomes::gbk_de_unicode(texto).context("o caminho tem caracteres que não existem em GBK")?;
        nomes::nome_no_disco(&gbk).context("caminho inválido")?;
        if gbk.first().is_some_and(|b| *b == b'.') {
            bail!("caminho inválido");
        }
        let chave = nomes::chave(&gbk);
        let atual = m
            .arquivos
            .iter()
            .find(|a| hex::decode(&a.caminho_gbk).is_ok_and(|c| nomes::chave(&c) == chave))
            .cloned();
        let gbk = match &atual {
            Some(a) => hex::decode(&a.caminho_gbk)?,
            None => gbk,
        };
        Ok(Caminho { gbk, atual })
    }

    pub(crate) async fn rascunho_da_versao(&self, versao: &str, alteracao: i64) -> Result<()> {
        let a = self.repo.alteracao(alteracao).await?.context("alteração inexistente")?;
        if a.linha != versao {
            bail!("a alteração {alteracao} é da versão {}", a.linha);
        }
        if a.estado != "rascunho" {
            bail!("a alteração {alteracao} já está {}", a.estado);
        }
        Ok(())
    }

    /// Acrescenta ao rascunho o conteúdo novo de um arquivo (substitui o publicado ou acrescenta).
    pub async fn gravar_arquivo(
        &self,
        versao: &str,
        alteracao: i64,
        caminho: &str,
        conteudo: Vec<u8>,
        destino: &str,
    ) -> Result<ItemDeRecurso> {
        if !matches!(destino, "cliente" | "servidor" | "ambos") {
            bail!("destino inválido: {destino}");
        }
        self.rascunho_da_versao(versao, alteracao).await?;
        let (_, m) = self.publicado(versao)?;
        let c = Self::resolver(&m, caminho)?;
        let sha = sha256(&conteudo);
        if c.atual.as_ref().is_some_and(|a| a.sha256 == hex::encode(sha)) {
            bail!("o arquivo enviado é igual ao publicado");
        }
        let objetos = self.objetos(versao);
        let tamanho = conteudo.len() as i64;
        tokio::task::spawn_blocking(move || objetos.gravar(&sha, &conteudo)).await??;
        let item = ItemDeRecurso {
            alteracao,
            caminho: c.gbk.clone(),
            destino: destino.into(),
            acao: if c.atual.is_some() { "substituir" } else { "adicionar" }.into(),
            sha256_antes: c.atual.as_ref().map(|a| hex::decode(&a.sha256)).transpose()?,
            sha256_depois: Some(sha.to_vec()),
            tamanho: Some(tamanho),
            resumo: serde_json::json!({
                "caminho": nomes::legivel(&c.gbk),
                "tamanho_antes": c.atual.as_ref().map(|a| a.tamanho),
                "tamanho_depois": tamanho,
            }),
        };
        self.repo.gravar_item(&item).await?;
        Ok(item)
    }

    /// Marca no rascunho a remoção de um arquivo publicado.
    pub async fn remover_arquivo(&self, versao: &str, alteracao: i64, caminho: &str) -> Result<ItemDeRecurso> {
        self.rascunho_da_versao(versao, alteracao).await?;
        let (_, m) = self.publicado(versao)?;
        let c = Self::resolver(&m, caminho)?;
        let atual = c.atual.context("o arquivo não existe na versão publicada")?;
        let item = ItemDeRecurso {
            alteracao,
            caminho: c.gbk.clone(),
            destino: "cliente".into(),
            acao: "remover".into(),
            sha256_antes: Some(hex::decode(&atual.sha256)?),
            sha256_depois: None,
            tamanho: None,
            resumo: serde_json::json!({ "caminho": nomes::legivel(&c.gbk), "tamanho_antes": atual.tamanho }),
        };
        self.repo.gravar_item(&item).await?;
        Ok(item)
    }

    /// Tira um caminho do rascunho.
    pub async fn tirar(&self, versao: &str, alteracao: i64, caminho: &str) -> Result<bool> {
        self.rascunho_da_versao(versao, alteracao).await?;
        let (_, m) = self.publicado(versao)?;
        let c = Self::resolver(&m, caminho)?;
        Ok(self.repo.tirar_item(alteracao, &c.gbk).await?)
    }

    /// Publica os rascunhos como a versão seguinte. Devolve o número publicado.
    pub async fn publicar(
        &self,
        versao: &str,
        rascunhos: &[i64],
        autor: Option<i32>,
        minima: Option<i32>,
    ) -> Result<Publicacao> {
        if rascunhos.is_empty() {
            bail!("nenhum rascunho escolhido");
        }
        let mut ids = rascunhos.to_vec();
        ids.sort_unstable();
        ids.dedup();
        let (v, m) = self.publicado(versao)?;
        let (atual_banco, minima_atual) = self.repo.versao_atual(versao).await?.context("versão sem registro no banco")?;
        if atual_banco as u32 != v.atual {
            bail!("o volume (versão {}) e o banco (versão {atual_banco}) discordam", v.atual);
        }
        let mut arquivos: BTreeMap<String, Arquivo> =
            m.arquivos.into_iter().map(|a| (a.caminho_gbk.clone(), a)).collect();
        let objetos = self.objetos(versao);
        let mut total_itens = 0;
        let mut para_o_realm: Vec<ItemDeRecurso> = Vec::new();
        for id in &ids {
            self.rascunho_da_versao(versao, *id).await?;
            let itens = self.repo.itens(*id).await?;
            if itens.is_empty() {
                bail!("o rascunho {id} está vazio");
            }
            for i in itens {
                total_itens += 1;
                if i.destino != "cliente" {
                    para_o_realm.push(i.clone());
                }
                if i.destino == "servidor" {
                    continue; // só do servidor: não entra no manifesto do cliente
                }
                let chave = hex::encode(&i.caminho);
                let atual_sha = arquivos.get(&chave).map(|a| a.sha256.clone());
                let antes = i.sha256_antes.as_ref().map(hex::encode);
                if atual_sha != antes {
                    bail!(
                        "{} mudou depois que o rascunho {id} foi feito; refaça o item",
                        nomes::legivel(&i.caminho)
                    );
                }
                match (i.acao.as_str(), &i.sha256_depois) {
                    ("remover", _) => {
                        arquivos.remove(&chave);
                    }
                    (_, Some(depois)) => {
                        let sha: Sha256Bytes = depois.as_slice().try_into().context("sha256_depois")?;
                        if !objetos.existe(&sha) {
                            bail!("o conteúdo de {} sumiu do volume", nomes::legivel(&i.caminho));
                        }
                        let pacote = arquivos
                            .get(&chave)
                            .map(|a| a.pacote.clone())
                            .unwrap_or_else(|| pacote_de(&i.caminho));
                        arquivos.insert(
                            chave.clone(),
                            Arquivo {
                                caminho_gbk: chave,
                                sha256: hex::encode(sha),
                                tamanho: i.tamanho.context("item sem tamanho")? as u64,
                                pacote,
                            },
                        );
                    }
                    _ => bail!("item inválido em {}", nomes::legivel(&i.caminho)),
                }
            }
        }
        let numero = v.atual + 1;
        let minima = minima.unwrap_or(minima_atual).clamp(0, numero as i32);
        let pacotes = m
            .pacotes
            .into_iter()
            .map(|mut p| {
                p.numero = Some(p.numero.unwrap_or(m.numero));
                p
            })
            .collect();
        let novo = Manifesto {
            formato: manifesto::FORMATO,
            versao_cliente: versao.into(),
            numero,
            gerado_em: chrono::Utc::now().to_rfc3339(),
            pacotes,
            arquivos: arquivos.into_values().collect(),
        };
        let bytes = serde_json::to_vec(&novo)?;
        let atualizacao = pasta_da_versao(&self.dados, versao).join("atualizacao");
        let dir = atualizacao.join(numero.to_string());
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?; // sobra de uma publicação que falhou no banco
        }
        std::fs::create_dir_all(&dir)?;
        let assinatura = gravar_assinado(&dir.join("manifesto.json"), &bytes, &self.chave)?;
        let manifesto_sha256 = sha256(&bytes);
        if let Err(e) = self
            .repo
            .publicar(versao, numero as i32, &ids, &manifesto_sha256, &assinatura, autor, minima)
            .await
        {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e.into());
        }
        let nova = Versao {
            formato: manifesto::FORMATO,
            versao_cliente: versao.into(),
            atual: numero,
            minima: minima as u32,
            manifesto: format!("{numero}/manifesto.json"),
            manifesto_sha256: hex::encode(manifesto_sha256),
        };
        gravar_assinado(&atualizacao.join("versao.json"), &serde_json::to_vec_pretty(&nova)?, &self.chave)?;
        tracing::info!("versão {versao} nº {numero} publicada: {} rascunhos, {total_itens} itens", ids.len());
        let aplicados = self.aplicar_nos_realms(versao, &para_o_realm)?;
        Ok(Publicacao { numero, aplicados })
    }

    /// Grava nos `config` dos realms da versão os itens de destino `servidor`/`ambos` (o "Ligação
    /// com os realms" do §3): só arquivos de `data\` direto (`data\elements.data` → `elements.data`),
    /// por troca atômica. O GS lê esses arquivos ao subir: o realm precisa ser reiniciado.
    /// Remoção não apaga nada do realm (o servidor depende desses arquivos).
    fn aplicar_nos_realms(&self, versao: &str, itens: &[ItemDeRecurso]) -> Result<Vec<String>> {
        let mut aplicados = Vec::new();
        let Some(pastas) = self.realms.get(versao) else {
            if !itens.is_empty() {
                tracing::warn!("versão {versao} sem PW_RECURSOS_REALMS: {} itens do servidor não aplicados", itens.len());
            }
            return Ok(aplicados);
        };
        let objetos = self.objetos(versao);
        for i in itens {
            let Some(depois) = &i.sha256_depois else { continue };
            let nome = nomes::legivel(&i.caminho);
            let arquivo = match nome.get(..5) {
                Some(p) if p.eq_ignore_ascii_case("data\\") => &nome[5..],
                _ => bail!("{nome}: só arquivos de data\\ vão para o realm"),
            };
            if arquivo.is_empty() || arquivo.contains(['\\', '/', ':']) || arquivo.starts_with('.') {
                bail!("{nome}: nome inválido para o realm");
            }
            let sha: Sha256Bytes = depois.as_slice().try_into().context("sha256_depois")?;
            let tamanho = i.tamanho.context("item sem tamanho")? as usize;
            let dados = objetos.ler(&sha, tamanho)?;
            for pasta in pastas {
                let destino = pasta.join(arquivo.to_lowercase());
                let tmp = pasta.join(format!(".{}.pw-tmp", arquivo.to_lowercase()));
                std::fs::write(&tmp, &dados).with_context(|| format!("gravar {}", tmp.display()))?;
                std::fs::rename(&tmp, &destino).with_context(|| format!("trocar {}", destino.display()))?;
                aplicados.push(destino.display().to_string());
            }
        }
        Ok(aplicados)
    }

    /// Cria um rascunho que desfaz uma alteração publicada. Devolve o id do rascunho.
    pub async fn reverter(&self, versao: &str, alteracao: i64, autor: Option<i32>, operacao_id: &str) -> Result<i64> {
        let a = self.repo.alteracao(alteracao).await?.context("alteração inexistente")?;
        if a.linha != versao || a.estado != "publicada" {
            bail!("só se reverte alteração publicada desta versão");
        }
        let (_, m) = self.publicado(versao)?;
        let atuais: BTreeMap<Vec<u8>, &Arquivo> = m
            .arquivos
            .iter()
            .filter_map(|x| hex::decode(&x.caminho_gbk).ok().map(|c| (c, x)))
            .collect();
        let itens = self.repo.itens(alteracao).await?;
        let novo = self
            .repo
            .criar_rascunho(versao, autor, &format!("Reverte a alteração #{alteracao}: {}", a.descricao), operacao_id, Some(alteracao))
            .await?;
        let objetos = self.objetos(versao);
        for i in itens {
            let atual = atuais.get(&i.caminho);
            let sha_antes = atual.map(|x| hex::decode(&x.sha256)).transpose()?;
            let item = match &i.sha256_antes {
                // O arquivo não existia antes: o inverso é removê-lo (se ainda existe).
                None => match atual {
                    Some(x) => ItemDeRecurso {
                        alteracao: novo,
                        caminho: i.caminho.clone(),
                        destino: i.destino.clone(),
                        acao: "remover".into(),
                        sha256_antes: sha_antes,
                        sha256_depois: None,
                        tamanho: None,
                        resumo: serde_json::json!({ "caminho": nomes::legivel(&i.caminho), "tamanho_antes": x.tamanho }),
                    },
                    None => continue,
                },
                Some(antigo) => {
                    let sha: Sha256Bytes = antigo.as_slice().try_into().context("sha256_antes")?;
                    if sha_antes.as_deref() == Some(antigo.as_slice()) {
                        continue; // já está como antes
                    }
                    let tamanho = objetos.tamanho(&sha).context("o conteúdo antigo sumiu do volume")? as i64;
                    ItemDeRecurso {
                        alteracao: novo,
                        caminho: i.caminho.clone(),
                        destino: i.destino.clone(),
                        acao: if atual.is_some() { "substituir" } else { "adicionar" }.into(),
                        sha256_antes: sha_antes,
                        sha256_depois: Some(antigo.clone()),
                        tamanho: Some(tamanho),
                        resumo: serde_json::json!({
                            "caminho": nomes::legivel(&i.caminho),
                            "tamanho_antes": atual.map(|x| x.tamanho),
                            "tamanho_depois": tamanho,
                        }),
                    }
                }
            };
            self.repo.gravar_item(&item).await?;
        }
        Ok(novo)
    }

    /// Conteúdo atual de um arquivo da versão publicada (para o GM baixar e editar).
    pub fn conteudo(&self, versao: &str, caminho: &str) -> Result<(String, Vec<u8>)> {
        let (_, m) = self.publicado(versao)?;
        let c = Self::resolver(&m, caminho)?;
        let a = c.atual.context("o arquivo não existe na versão publicada")?;
        let sha: Sha256Bytes = hex::decode(&a.sha256)?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?;
        Ok((nomes::legivel(&c.gbk), self.objetos(versao).ler(&sha, a.tamanho as usize)?))
    }

    /// Arquivos da versão publicada cujo caminho contém `busca` (sem diferenciar maiúsculas).
    pub fn buscar(&self, versao: &str, busca: &str, limite: usize) -> Result<Vec<serde_json::Value>> {
        let (_, m) = self.publicado(versao)?;
        let busca = busca.to_lowercase().replace('/', "\\");
        Ok(m.arquivos
            .iter()
            .filter_map(|a| {
                let nome = nomes::legivel(&hex::decode(&a.caminho_gbk).ok()?);
                nome.to_lowercase().contains(&busca).then(|| {
                    serde_json::json!({ "caminho": nome, "tamanho": a.tamanho, "sha256": a.sha256, "pacote": a.pacote })
                })
            })
            .take(limite)
            .collect())
    }
}
