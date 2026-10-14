//! Estúdio, fase S1 (B237): editar o `elements.data` de uma versão de cliente pelo painel.
//!
//! Cada edição vai para um **rascunho** do P3 como o item `data\elements.data` (destino `ambos`:
//! o launcher entrega ao cliente e a publicação grava no `config` dos realms da versão), com o
//! resumo **campo a campo** (`{tabela, id, campo, antes, depois}`) e os registros criados ou
//! removidos. O arquivo é lido e gravado pelo `pw_data_loader::elements_editavel` (ida e volta
//! byte a byte; ids por espaço do `add_id_index`).

use crate::governanca::Governanca;
use crate::nomes;
use crate::objetos::{sha256, Objetos, Sha256Bytes};
use anyhow::{bail, Context, Result};
use pw_data_loader::elements_editavel::{espaco_de_id, ElementsEditavel};
use pw_data_loader::generic_elements::{FieldDef, FieldValue};
use pw_storage::ItemDeRecurso;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const CAMINHO: &str = "data\\elements.data";

/// Nível zstd dos objetos de rascunho: cada edição grava o arquivo inteiro (até 55 MB no 1.5.5);
/// os intermediários viram lixo que o `limpar` recolhe.
const NIVEL_DO_RASCUNHO: i32 = 3;

#[derive(Default)]
pub struct Estudio {
    /// Arquivos já lidos, pelo SHA-256 (o publicado e o de cada rascunho em uso).
    cache: Mutex<HashMap<Sha256Bytes, Arc<ElementsEditavel>>>,
    /// Uma edição por rascunho de cada vez.
    travas: Mutex<HashMap<i64, Arc<tokio::sync::Mutex<()>>>>,
}

fn caminho_gbk() -> Vec<u8> {
    nomes::gbk_de_unicode(CAMINHO).expect("ASCII")
}

/// Valor do campo para o painel. `string` (bytes GBK, caminhos de arquivo) vira texto legível.
pub fn valor_json(f: &FieldDef, v: &FieldValue) -> Value {
    match v {
        FieldValue::Int(i) => json!(i),
        FieldValue::Float(x) => json!(x),
        FieldValue::Text(t) => json!(t),
        FieldValue::Raw(b) if f.field_type == "string" => json!(nomes::legivel(b)),
        FieldValue::Raw(b) => json!(hex::encode(b)),
    }
}

/// Valor que veio do painel, no tipo do campo.
pub fn valor_do_json(f: &FieldDef, v: &Value) -> Result<FieldValue> {
    Ok(match f.field_type.as_str() {
        "int32" => {
            let n = v.as_i64().with_context(|| format!("{}: número inteiro", f.name))?;
            FieldValue::Int(i32::try_from(n).map_err(|_| anyhow::anyhow!("{}: fora do int32", f.name))?)
        }
        "float" => FieldValue::Float(v.as_f64().with_context(|| format!("{}: número", f.name))? as f32),
        "wstring" => FieldValue::Text(v.as_str().with_context(|| format!("{}: texto", f.name))?.to_string()),
        "string" => {
            let t = v.as_str().with_context(|| format!("{}: texto", f.name))?;
            FieldValue::Raw(nomes::gbk_de_unicode(t).with_context(|| format!("{}: caractere sem GBK", f.name))?)
        }
        outro => bail!("tipo de campo desconhecido: {outro}"),
    })
}

impl Estudio {
    fn trava(&self, rascunho: i64) -> Arc<tokio::sync::Mutex<()>> {
        self.travas.lock().unwrap().entry(rascunho).or_default().clone()
    }

    async fn ler(&self, objetos: &Objetos, sha: Sha256Bytes, tamanho: usize) -> Result<Arc<ElementsEditavel>> {
        if let Some(e) = self.cache.lock().unwrap().get(&sha) {
            return Ok(e.clone());
        }
        let objetos = objetos.clone();
        let e = tokio::task::spawn_blocking(move || -> Result<ElementsEditavel> {
            let dados = objetos.ler(&sha, tamanho)?;
            Ok(ElementsEditavel::ler(&dados, None)?)
        })
        .await??;
        let e = Arc::new(e);
        let mut c = self.cache.lock().unwrap();
        if c.len() >= 4 {
            c.clear(); // simples: cada arquivo do 1.5.5 ocupa ~55 MB em memória
        }
        c.insert(sha, e.clone());
        Ok(e)
    }

    /// O `elements.data` publicado da versão: SHA-256 e tamanho.
    fn publicado(gov: &Governanca, versao: &str) -> Result<(Sha256Bytes, usize)> {
        let (_, m) = gov.publicado(versao)?;
        let alvo = hex::encode(caminho_gbk());
        let a = m.arquivos.iter().find(|a| a.caminho_gbk == alvo).context("a versão não tem data\\elements.data")?;
        Ok((hex::decode(&a.sha256)?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?, a.tamanho as usize))
    }

    /// O arquivo como está no rascunho (ou o publicado, se o rascunho não o tem).
    async fn atual(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<(Arc<ElementsEditavel>, Option<ItemDeRecurso>)> {
        let objetos = gov.objetos(versao);
        if let Some(r) = rascunho {
            gov.rascunho_da_versao(versao, r).await?;
            let item = gov.repo.itens(r).await?.into_iter().find(|i| i.caminho == caminho_gbk());
            if let Some(i) = item {
                let sha: Sha256Bytes = i.sha256_depois.clone().context("elements removido no rascunho")?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?;
                let e = self.ler(&objetos, sha, i.tamanho.unwrap_or(0) as usize).await?;
                return Ok((e, Some(i)));
            }
        }
        let (sha, tamanho) = Self::publicado(gov, versao)?;
        Ok((self.ler(&objetos, sha, tamanho).await?, None))
    }

    pub async fn tabelas(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        Ok(json!({
            "versao_do_arquivo": e.versao,
            "tabelas": e.tabelas.iter().map(|t| json!({
                "nome": t.nome,
                "registros": t.registros.len(),
                "espaco": espaco_de_id(&t.nome),
                "campos": t.campos().len(),
            })).collect::<Vec<_>>(),
        }))
    }

    pub async fn listar(
        &self,
        gov: &Governanca,
        versao: &str,
        rascunho: Option<i64>,
        tabela: &str,
        busca: &str,
        pagina: usize,
        por_pagina: usize,
    ) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let t = e.tabela(tabela)?;
        let campo_nome = t.campos().into_iter().find(|(_, _, f)| f.name == "Name").map(|(_, _, f)| f.clone());
        let busca = busca.trim().to_lowercase();
        let mut achados = Vec::new();
        for i in 0..t.registros.len() {
            let id = t.id(i);
            let nome = match &campo_nome {
                Some(_) => match t.ler(i)?.get("Name") {
                    Some(FieldValue::Text(s)) => s.clone(),
                    _ => String::new(),
                },
                None => String::new(),
            };
            if busca.is_empty() || id.is_some_and(|x| x.to_string() == busca) || nome.to_lowercase().contains(&busca) {
                achados.push(json!({ "indice": i, "id": id, "nome": nome }));
            }
        }
        let total = achados.len();
        let por_pagina = por_pagina.clamp(1, 200);
        let pagina_lista: Vec<Value> = achados.into_iter().skip(pagina * por_pagina).take(por_pagina).collect();
        Ok(json!({ "total": total, "pagina": pagina, "por_pagina": por_pagina, "registros": pagina_lista }))
    }

    pub async fn ficha(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>, tabela: &str, indice: usize) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let t = e.tabela(tabela)?;
        if indice >= t.registros.len() {
            bail!("registro {indice} inexistente em {tabela}");
        }
        let r = t.ler(indice)?;
        let campos: Vec<Value> = t
            .campos()
            .into_iter()
            .map(|(off, tam, f)| json!({
                "nome": f.name, "tipo": f.field_type, "tamanho": tam, "deslocamento": off,
                "valor": r.get(&f.name).map(|v| valor_json(f, v)),
            }))
            .collect();
        Ok(json!({ "tabela": tabela, "indice": indice, "id": t.id(indice), "espaco": espaco_de_id(tabela), "campos": campos }))
    }

    /// Aplica `mudar` sobre o arquivo do rascunho e grava o resultado como o item do rascunho.
    async fn alterar<F>(&self, gov: &Governanca, versao: &str, rascunho: i64, mudar: F) -> Result<Value>
    where
        F: FnOnce(&mut ElementsEditavel, &mut Resumo) -> Result<Value>,
    {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let (atual, item) = self.atual(gov, versao, Some(rascunho)).await?;
        let mut e = (*atual).clone();
        let mut resumo = Resumo::de(item.as_ref().map(|i| &i.resumo));
        let resposta = mudar(&mut e, &mut resumo)?;
        let bytes = e.gravar();
        let sha = sha256(&bytes);
        let (sha_publicado, tamanho_publicado) = Self::publicado(gov, versao)?;
        if sha == sha_publicado {
            gov.repo.tirar_item(rascunho, &caminho_gbk()).await?;
            return Ok(json!({ "resultado": resposta, "igual_ao_publicado": true }));
        }
        let objetos = Objetos::new(gov.objetos(versao).raiz().to_path_buf(), NIVEL_DO_RASCUNHO);
        let tamanho = bytes.len();
        tokio::task::spawn_blocking(move || objetos.gravar(&sha, &bytes)).await??;
        self.cache.lock().unwrap().insert(sha, Arc::new(e));
        let item = ItemDeRecurso {
            alteracao: rascunho,
            caminho: caminho_gbk(),
            destino: "ambos".into(),
            acao: "substituir".into(),
            sha256_antes: Some(sha_publicado.to_vec()),
            sha256_depois: Some(sha.to_vec()),
            tamanho: Some(tamanho as i64),
            resumo: resumo.json(tamanho_publicado, tamanho),
        };
        gov.repo.gravar_item(&item).await?;
        Ok(json!({ "resultado": resposta, "igual_ao_publicado": false }))
    }

    /// Edita campos de um registro. `campos` = `{nome: valor}`.
    pub async fn editar(
        &self,
        gov: &Governanca,
        versao: &str,
        rascunho: i64,
        tabela: &str,
        indice: usize,
        campos: Map<String, Value>,
    ) -> Result<Value> {
        let tabela = tabela.to_string();
        self.alterar(gov, versao, rascunho, move |e, resumo| {
            let t = e.tabela(&tabela)?;
            let id = t.id(indice);
            let antes = t.ler(indice).with_context(|| format!("registro {indice}"))?;
            let defs: HashMap<String, FieldDef> = t.campos().into_iter().map(|(_, _, f)| (f.name.clone(), f.clone())).collect();
            let mut mudados = 0;
            for (nome, v) in campos {
                let f = defs.get(&nome).with_context(|| format!("campo inexistente: {nome}"))?;
                let novo = valor_do_json(f, &v)?;
                let velho = antes.get(&nome).cloned().context("campo")?;
                if novo == velho {
                    continue;
                }
                e.definir_campo(&tabela, indice, &nome, &novo)?;
                // O que fica no registro (o leitor relê: um texto com NUL no meio sai cortado).
                let gravado = e.tabela(&tabela)?.ler(indice)?.get(&nome).cloned().context("campo")?;
                resumo.campo(&tabela, id, &nome, valor_json(f, &velho), valor_json(f, &gravado));
                mudados += 1;
            }
            Ok(json!({ "campos_mudados": mudados }))
        })
        .await
    }

    /// Novo registro clonado de `indice`, com o próximo id do espaço.
    pub async fn clonar(&self, gov: &Governanca, versao: &str, rascunho: i64, tabela: &str, indice: usize) -> Result<Value> {
        let tabela = tabela.to_string();
        self.alterar(gov, versao, rascunho, move |e, resumo| {
            let base = e.tabela(&tabela)?.id(indice);
            let novo = e.proximo_id(&tabela)?;
            let i = e.clonar(&tabela, indice, novo)?;
            resumo.registro(&tabela, novo, "novo", base);
            Ok(json!({ "indice": i, "id": novo }))
        })
        .await
    }

    /// Remove um registro — recusado se outro registro do arquivo cita o id (Estúdio §3.2).
    pub async fn remover(&self, gov: &Governanca, versao: &str, rascunho: i64, tabela: &str, indice: usize) -> Result<Value> {
        let tabela = tabela.to_string();
        self.alterar(gov, versao, rascunho, move |e, resumo| {
            let id = e.tabela(&tabela)?.id(indice).context("registro sem ID")?;
            let refs = referencias(e, &tabela, indice, id);
            if !refs.is_empty() {
                bail!("o id {id} é citado por {} campo(s), ex.: {}", refs.len(), refs.iter().take(5).cloned().collect::<Vec<_>>().join(", "));
            }
            e.remover(&tabela, indice)?;
            resumo.registro(&tabela, id, "removido", None);
            Ok(json!({ "removido": id }))
        })
        .await
    }
}

/// Campos `int32` de outros registros cujo valor é `id` (conservador: um número igual por
/// acaso também bloqueia; o índice de referências por catálogo é da fase S2).
fn referencias(e: &ElementsEditavel, tabela: &str, indice: usize, id: i32) -> Vec<String> {
    let mut achados = Vec::new();
    if id == 0 {
        return achados;
    }
    let alvo = id.to_le_bytes();
    for t in &e.tabelas {
        let ints: Vec<(usize, String)> = t
            .campos()
            .into_iter()
            .filter(|(off, _, f)| f.field_type == "int32" && !(*off == 0 && f.name == "ID"))
            .map(|(off, _, f)| (off, f.name.clone()))
            .collect();
        for (i, r) in t.registros.iter().enumerate() {
            if t.nome == tabela && i == indice {
                continue;
            }
            for (off, nome) in &ints {
                if r[*off..*off + 4] == alvo {
                    achados.push(format!("{}#{}.{}", t.nome, t.id(i).unwrap_or(-1), nome));
                    if achados.len() >= 50 {
                        return achados;
                    }
                }
            }
        }
    }
    achados
}

/// O resumo do item `data\elements.data` de um rascunho: mudanças por campo e registros.
#[derive(Default)]
struct Resumo {
    campos: Vec<Value>,
    registros: Vec<Value>,
}

impl Resumo {
    fn de(v: Option<&Value>) -> Self {
        let lista = |k: &str| v.and_then(|v| v.get(k)).and_then(Value::as_array).cloned().unwrap_or_default();
        Resumo { campos: lista("campos"), registros: lista("registros") }
    }

    fn campo(&mut self, tabela: &str, id: Option<i32>, campo: &str, antes: Value, depois: Value) {
        let mesmo = |c: &Value| c["tabela"] == tabela && c["id"] == json!(id) && c["campo"] == campo;
        if let Some(c) = self.campos.iter_mut().find(|c| mesmo(c)) {
            c["depois"] = depois; // o "antes" fica o do publicado
        } else {
            self.campos.push(json!({ "tabela": tabela, "id": id, "campo": campo, "antes": antes, "depois": depois }));
        }
        self.campos.retain(|c| c["antes"] != c["depois"]);
    }

    fn registro(&mut self, tabela: &str, id: i32, acao: &str, base: Option<i32>) {
        if acao == "removido" {
            // Remover um registro criado neste mesmo rascunho só desfaz a criação.
            let antes = self.registros.len();
            self.registros.retain(|r| !(r["tabela"] == tabela && r["id"] == id && r["acao"] == "novo"));
            self.campos.retain(|c| !(c["tabela"] == tabela && c["id"] == id));
            if self.registros.len() < antes {
                return;
            }
        }
        self.registros.push(json!({ "tabela": tabela, "id": id, "acao": acao, "base": base }));
    }

    fn json(&self, tamanho_antes: usize, tamanho_depois: usize) -> Value {
        json!({
            "caminho": CAMINHO,
            "tamanho_antes": tamanho_antes,
            "tamanho_depois": tamanho_depois,
            "campos": self.campos,
            "registros": self.registros,
        })
    }
}
