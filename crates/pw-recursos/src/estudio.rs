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
    /// Índice dos arquivos do cliente publicado por versão: caminho em minúsculas → (sha, tamanho),
    /// com o SHA do manifesto de onde saiu (B242).
    arquivos_do_cliente: Mutex<HashMap<String, (String, Arc<HashMap<String, (Sha256Bytes, u64)>>)>>,
    /// Peças do minimapa já convertidas em PNG.
    pecas: Mutex<HashMap<(String, String, String), Arc<Vec<u8>>>>,
    /// Terreno (`.hmap`) de cada mapa já lido, por (versão, caminho) — B248.
    terrenos: Mutex<HashMap<(String, String), Arc<pw_data_loader::Terreno>>>,
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
            })).chain(std::iter::once(json!({
                "nome": "TALK_PROC", "registros": e.conversas.len(), "espaco": "TALK", "campos": 0, "conversas": true,
            }))).collect::<Vec<_>>(),
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
                achados.push(json!({ "indice": i, "id": id, "nome": nome, "icone": icone_do_registro(t, i) }));
            }
        }
        let total = achados.len();
        let por_pagina = por_pagina.clamp(1, 200);
        let pagina_lista: Vec<Value> = achados.into_iter().skip(pagina * por_pagina).take(por_pagina).collect();
        Ok(json!({ "total": total, "pagina": pagina, "por_pagina": por_pagina, "registros": pagina_lista }))
    }

    /// B283: os modelos de voo do elements e o modo de cada um (`ecm::encaixe_de_voo`): `FLYSWORD_ESSENCE.file_model`
    /// (e `file_model2`, o pé direito das rodas) com o `fly_mode`; `WINGMANWING_ESSENCE.file_model` é asa (modo 1).
    pub async fn modelos_de_voo(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<Vec<(String, u32)>> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let mut r: Vec<(String, u32)> = Vec::new();
        for tabela in ["FLYSWORD_ESSENCE", "WINGMANWING_ESSENCE"] {
            let Ok(t) = e.tabela(tabela) else { continue };
            for i in 0..t.registros.len() {
                let reg = t.ler(i)?;
                let modo = match tabela {
                    "WINGMANWING_ESSENCE" => 1,
                    _ => reg.get("fly_mode").and_then(FieldValue::as_i32).unwrap_or(0).max(0) as u32,
                };
                for campo in ["file_model", "file_model2"] {
                    // O `string` do layout vem em bytes GBK (`FieldValue::Raw`), como na ficha (`valor_json`).
                    let c = match reg.get(campo) {
                        Some(FieldValue::Raw(b)) => nomes::legivel(b),
                        Some(FieldValue::Text(t)) => t.clone(),
                        _ => continue,
                    };
                    let c = c.trim().to_string();
                    if !c.is_empty() && !r.iter().any(|(x, _)| x.eq_ignore_ascii_case(&c)) {
                        r.push((c, modo));
                    }
                }
            }
        }
        Ok(r)
    }

    pub async fn ficha(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>, tabela: &str, indice: usize) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let t = e.tabela(tabela)?;
        if indice >= t.registros.len() {
            bail!("registro {indice} inexistente em {tabela}");
        }
        let r = t.ler(indice)?;
        let ids = mapa_de_ids(&e);
        let cat = catalogo();
        let campos: Vec<Value> = t
            .campos()
            .into_iter()
            .map(|(off, tam, f)| {
                let valor = r.get(&f.name);
                let mut c = json!({
                    "nome": f.name, "tipo": f.field_type, "tamanho": tam, "deslocamento": off,
                    "valor": valor.map(|v| valor_json(f, v)),
                });
                let (rotulo, grupo) = rotulo_e_grupo(&f.name);
                c["rotulo"] = json!(rotulo);
                c["grupo"] = json!(grupo);
                if let Some(esp) = espaco_citado(tabela, &f.name) {
                    c["cita"] = json!(esp);
                    if let Some(FieldValue::Int(v)) = valor {
                        if *v != 0 && *v != -1 {
                            c["referencia"] = match ids.get(&(esp.to_string(), *v)) {
                                Some(&(CONVERSAS, ri)) => json!({
                                    "tabela": "TALK_PROC", "indice": ri, "id": v,
                                    "nome": pw_data_loader::conversas::texto(&e.conversas[ri].texto),
                                }),
                                Some(&(ti, ri)) => {
                                    let alvo = &e.tabelas[ti];
                                    json!({ "tabela": alvo.nome, "indice": ri, "id": v, "nome": nome_do_registro(alvo, ri), "icone": icone_do_registro(alvo, ri) })
                                }
                                None => json!({ "ausente": true, "id": v }),
                            };
                        }
                    }
                } else if cat.duvidosos.contains(&(tabela.to_string(), f.name.clone())) {
                    c["duvidoso"] = json!(true);
                }
                c
            })
            .collect();
        let usado_por = match (espaco_de_id(tabela), t.id(indice)) {
            (Some(esp), Some(id)) => citantes(&e, &esp, id, (tabela, indice), false, 100),
            _ => vec![],
        };
        Ok(json!({
            "tabela": tabela, "indice": indice, "id": t.id(indice), "espaco": espaco_de_id(tabela),
            "nome": nome_do_registro(t, indice), "icone": icone_do_registro(t, indice), "campos": campos, "usado_por": usado_por,
        }))
    }

    /// Registros de um espaço com o id exato ou o nome contendo `busca` (o seletor de referência).
    pub async fn buscar_no_espaco(
        &self,
        gov: &Governanca,
        versao: &str,
        rascunho: Option<i64>,
        espaco: &str,
        busca: &str,
        limite: usize,
    ) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let busca = busca.trim().to_lowercase();
        let mut achados = Vec::new();
        if espaco == "TALK" {
            for (i, c) in e.conversas.iter().enumerate() {
                let texto = pw_data_loader::conversas::texto(&c.texto);
                if busca.is_empty() || c.id.to_string() == busca || texto.to_lowercase().contains(&busca) {
                    achados.push(json!({ "tabela": "TALK_PROC", "indice": i, "id": c.id, "nome": texto }));
                    if achados.len() >= limite.clamp(1, 200) {
                        break;
                    }
                }
            }
            return Ok(json!(achados));
        }
        'tabelas: for t in &e.tabelas {
            if espaco_de_id(&t.nome).as_deref() != Some(espaco) {
                continue;
            }
            for i in 0..t.registros.len() {
                let id = t.id(i);
                let nome = nome_do_registro(t, i);
                if busca.is_empty() || id.is_some_and(|x| x.to_string() == busca) || nome.to_lowercase().contains(&busca) {
                    achados.push(json!({ "tabela": t.nome, "indice": i, "id": id, "nome": nome, "icone": icone_do_registro(t, i) }));
                    if achados.len() >= limite.clamp(1, 200) {
                        break 'tabelas;
                    }
                }
            }
        }
        Ok(json!(achados))
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
            // Sem espaço conhecido nada no catálogo aponta para cá; os duvidosos ainda bloqueiam.
            let espaco = espaco_de_id(&tabela).unwrap_or_default();
            let refs = citantes(e, &espaco, id, (&tabela, indice), true, 50);
            if !refs.is_empty() {
                let ex: Vec<String> = refs
                    .iter()
                    .take(5)
                    .map(|r| format!("{}#{}.{}{}", r["tabela"].as_str().unwrap_or(""), r["id"], r["campo"].as_str().unwrap_or(""),
                        if r["duvidoso"] == true { " (duvidoso)" } else { "" }))
                    .collect();
                bail!("o id {id} é citado por {} campo(s), ex.: {}", refs.len(), ex.join(", "));
            }
            e.remover(&tabela, indice)?;
            resumo.registro(&tabela, id, "removido", None);
            Ok(json!({ "removido": id }))
        })
        .await
    }
}

// ------------------------------------------------------------------ índice de referências (S2, B238)

/// `specs/estudio/referencias.json` (gerado por `gerar_referencias.py`): campo → espaço que ele cita,
/// com duas evidências (comentário/nome do `ExpTypes.h` e ≥ 95% dos valores existindo no espaço). Os
/// `duvidosos` (parecem id, falta evidência) não viram link, mas também bloqueiam remoção.
struct Catalogo {
    refs: HashMap<String, HashMap<String, String>>,
    duvidosos: std::collections::HashSet<(String, String)>,
}

fn catalogo() -> &'static Catalogo {
    #[derive(serde::Deserialize)]
    struct Entrada {
        espaco: String,
    }
    #[derive(serde::Deserialize)]
    struct Duvidoso {
        tabela: String,
        campo: String,
    }
    #[derive(serde::Deserialize)]
    struct Arquivo {
        tabelas: HashMap<String, HashMap<String, Entrada>>,
        duvidosos: Vec<Duvidoso>,
    }
    static C: std::sync::OnceLock<Catalogo> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let a: Arquivo = serde_json::from_str(include_str!("../../../specs/estudio/referencias.json")).expect("referencias.json");
        Catalogo {
            refs: a.tabelas.into_iter().map(|(t, cs)| (t, cs.into_iter().map(|(c, e)| (c, e.espaco)).collect())).collect(),
            duvidosos: a.duvidosos.into_iter().map(|d| (d.tabela, d.campo)).collect(),
        }
    })
}

/// O espaço que o campo cita, se o catálogo sabe.
pub fn espaco_citado(tabela: &str, campo: &str) -> Option<&'static str> {
    catalogo().refs.get(tabela)?.get(campo).map(String::as_str)
}

/// Nome do arquivo do `file_icon` (bytes GBK, em hex) — o que a rota de ícones do painel
/// (`/api/icones/<m|f>/<hex>.png`, B188) procura no atlas: a célula é só o nome do arquivo, como o
/// cliente faz (`SetCover`, `EC_GameUIMan.cpp:6204-6208`). O corte é no texto decodificado: um 2º
/// byte GBK pode ser 0x5C (`\`).
fn icone_do_registro(t: &pw_data_loader::elements_editavel::TabelaEditavel, i: usize) -> Option<String> {
    match t.ler(i).ok()?.get("file_icon")? {
        FieldValue::Raw(b) if !b.is_empty() => {
            let texto = nomes::legivel(b);
            let arquivo = texto.rsplit(['\\', '/']).next().unwrap_or(&texto);
            nomes::gbk_de_unicode(arquivo).map(hex::encode)
        }
        _ => None,
    }
}

/// Rótulo em português e grupo da ficha (`specs/estudio/rotulos.json`, B239), pelo nome do campo
/// com os índices de vetor trocados por `N`. Sem rótulo: `None` e grupo "Outros".
pub fn rotulo_e_grupo(campo: &str) -> (Option<&'static str>, &'static str) {
    #[derive(serde::Deserialize)]
    struct Grupo {
        nome: String,
        padroes: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct Arquivo {
        campos: HashMap<String, String>,
        grupos: Vec<Grupo>,
    }
    struct Rotulos {
        campos: HashMap<String, String>,
        grupos: Vec<(String, Vec<regex_lite::Regex>)>,
        vetor: regex_lite::Regex,
    }
    static R: std::sync::OnceLock<Rotulos> = std::sync::OnceLock::new();
    let r = R.get_or_init(|| {
        let a: Arquivo = serde_json::from_str(include_str!("../../../specs/estudio/rotulos.json")).expect("rotulos.json");
        Rotulos {
            campos: a.campos,
            grupos: a
                .grupos
                .into_iter()
                .map(|g| (g.nome, g.padroes.iter().map(|p| regex_lite::Regex::new(p).expect("padrão de grupo")).collect()))
                .collect(),
            vetor: regex_lite::Regex::new(r"_\d+(_|$)").unwrap(),
        }
    });
    // `drop_matters_12_id` → `drop_matters_N_id` (o mesmo que o gerador faz em Python).
    let mut base = campo.to_string();
    loop {
        let novo = r.vetor.replace(&base, "_N$1").into_owned();
        if novo == base {
            break;
        }
        base = novo;
    }
    let rotulo = r.campos.get(&base).map(String::as_str);
    let grupo = r.grupos.iter().find(|(_, ps)| ps.iter().any(|p| p.is_match(&base))).map_or("Outros", |(n, _)| n.as_str());
    (rotulo, grupo)
}

fn nome_do_registro(t: &pw_data_loader::elements_editavel::TabelaEditavel, i: usize) -> String {
    match t.ler(i).ok().and_then(|r| r.get("Name").cloned()) {
        Some(FieldValue::Text(s)) => s,
        _ => String::new(),
    }
}

/// `(espaço, id)` → `(tabela, registro)` de todo o arquivo.
fn mapa_de_ids(e: &ElementsEditavel) -> HashMap<(String, i32), (usize, usize)> {
    let mut m = HashMap::new();
    for (ti, t) in e.tabelas.iter().enumerate() {
        let Some(espaco) = espaco_de_id(&t.nome) else { continue };
        for i in 0..t.registros.len() {
            if let Some(id) = t.id(i) {
                m.entry((espaco.clone(), id)).or_insert((ti, i));
            }
        }
    }
    // As conversas (`TALK_PROC`) são o espaço TALK (`add_id_index(ID_SPACE_TALK…)`).
    for (i, c) in e.conversas.iter().enumerate() {
        m.entry(("TALK".to_string(), c.id as i32)).or_insert((CONVERSAS, i));
    }
    m
}

/// Marca, no índice de ids, um registro que é uma conversa (`e.conversas[i]`).
const CONVERSAS: usize = usize::MAX;

/// Registros cujos campos de referência (e, com `com_duvidosos`, os duvidosos) citam `id` no
/// `espaco`. `excluir` = o próprio registro.
fn citantes(e: &ElementsEditavel, espaco: &str, id: i32, excluir: (&str, usize), com_duvidosos: bool, limite: usize) -> Vec<Value> {
    let mut achados = Vec::new();
    if id == 0 || id == -1 {
        return achados;
    }
    let alvo = id.to_le_bytes();
    let cat = catalogo();
    for t in &e.tabelas {
        let refs = cat.refs.get(&t.nome);
        let campos: Vec<(usize, String, bool)> = t
            .campos()
            .into_iter()
            .filter(|(_, _, f)| f.field_type == "int32" && f.name != "ID")
            .filter_map(|(off, _, f)| {
                if refs.and_then(|r| r.get(&f.name)).is_some_and(|esp| esp == espaco) {
                    Some((off, f.name.clone(), false))
                } else if com_duvidosos && cat.duvidosos.contains(&(t.nome.clone(), f.name.clone())) {
                    Some((off, f.name.clone(), true))
                } else {
                    None
                }
            })
            .collect();
        if campos.is_empty() {
            continue;
        }
        for (i, r) in t.registros.iter().enumerate() {
            if t.nome == excluir.0 && i == excluir.1 {
                continue;
            }
            for (off, nome, duvidoso) in &campos {
                if r[*off..*off + 4] == alvo {
                    achados.push(json!({
                        "tabela": t.nome, "indice": i, "id": t.id(i), "nome": nome_do_registro(t, i),
                        "campo": nome, "duvidoso": duvidoso,
                    }));
                    if achados.len() >= limite {
                        return achados;
                    }
                }
            }
        }
    }
    achados
}

/// Campos do `elements` que guardam id de missão (B268): `int32` com "task" no nome, menos os que apontam para registro
/// de serviço ou são quantidade, probabilidade, custo, item ou contagem. Medido no layout v156: `id_tasks_N`
/// (`NPC_TASK_OUT/IN_SERVICE`), `tasks_N_id_task` (`NPC_TASK_MATTER_SERVICE`), `task_lists_N_id` (`TASKDICE_ESSENCE`),
/// `list_N_task_id` (`AUTOTASK_DISPLAY_CONFIG`), `task_N` (`WIKI_TABOO_CONFIG`). Larga de propósito: um falso positivo
/// só recusa a remoção (e diz quem cita); um falso negativo deixaria referência quebrada.
fn campo_de_missao(nome: &str) -> bool {
    let n = nome.to_lowercase();
    n.contains("task") && !["service", "num", "prob", "matter", "contrib", "cost", "max", "total", "count"].iter().any(|x| n.contains(x))
}

/// Registros do `elements` com algum campo de missão igual a `id`.
fn citantes_de_missao(e: &ElementsEditavel, id: u32, limite: usize) -> Vec<Value> {
    let alvo = (id as i32).to_le_bytes();
    let mut achados = Vec::new();
    for t in &e.tabelas {
        let campos: Vec<(usize, String)> = t.campos().into_iter().filter(|(_, _, f)| f.field_type == "int32" && campo_de_missao(&f.name)).map(|(o, _, f)| (o, f.name.clone())).collect();
        for (i, r) in t.registros.iter().enumerate() {
            for (o, nome) in &campos {
                if r[*o..*o + 4] == alvo {
                    achados.push(json!({ "tabela": t.nome, "indice": i, "id": t.id(i), "nome": nome_do_registro(t, i), "campo": nome }));
                    if achados.len() >= limite {
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

#[cfg(test)]
mod testes {
    use super::rotulo_e_grupo;

    #[test]
    fn rotulos_e_grupos_pelo_nome_com_n() {
        assert_eq!(rotulo_e_grupo("drop_matters_12_id"), (Some("Item"), "Drops"));
        assert_eq!(rotulo_e_grupo("magic_defences_3_high"), (Some("Defesa elemental máxima"), "Combate"));
        assert_eq!(rotulo_e_grupo("skill_hp75_2_probability"), (Some("Chance"), "Habilidades"));
        assert_eq!(rotulo_e_grupo("price"), (Some("Preço de venda ao NPC"), "Comércio"));
        assert_eq!(rotulo_e_grupo("Name"), (Some("Nome"), "Identificação"));
        assert_eq!(rotulo_e_grupo("unk1"), (None, "Outros"));
        assert_eq!(rotulo_e_grupo("attack_speed").1, "Combate");
        assert_eq!(rotulo_e_grupo("run_speed").1, "Comportamento");
        // NPCs, serviços e receitas (B241).
        assert_eq!(rotulo_e_grupo("id_sell_service"), (Some("Serviço de loja"), "Serviços do NPC"));
        assert_eq!(rotulo_e_grupo("id_skill_service").1, "Serviços do NPC", "antes de Habilidades");
        assert_eq!(rotulo_e_grupo("pages_3_goods_12_id"), (Some("Item"), "Páginas"));
        assert_eq!(rotulo_e_grupo("materials_4_num"), (Some("Quantidade"), "Fabricação"));
        assert_eq!(rotulo_e_grupo("targets_2_idTarget"), (Some("Estação de destino"), "Teleporte"));
        assert_eq!(rotulo_e_grupo("id_tasks_17"), (Some("Missão"), "Missões"));
        assert_eq!(rotulo_e_grupo("hello_msg"), (Some("Saudação"), "Identificação"));
    }
}

// ------------------------------------------------------------------ conversas (S3, B240)

/// Funções pré-definidas das opções (`specs/estudio/funcoes_de_conversa.json`, do `enum SERVICE_TYPE`).
fn funcoes() -> &'static Vec<Value> {
    static F: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    F.get_or_init(|| {
        let v: Value =
            serde_json::from_str(include_str!("../../../specs/estudio/funcoes_de_conversa.json")).expect("funcoes_de_conversa.json");
        v["funcoes"].as_array().cloned().unwrap_or_default()
    })
}

fn funcao(codigo: u32) -> Option<&'static Value> {
    funcoes().iter().find(|f| f["codigo"].as_u64() == Some(codigo as u64))
}

fn conversa_json(c: &pw_data_loader::conversas::Conversa, indice: usize) -> Value {
    use pw_data_loader::conversas::{texto, BIT_DE_FUNCAO};
    json!({
        "indice": indice,
        "id": c.id,
        "texto": texto(&c.texto),
        "janelas": c.janelas.iter().map(|j| json!({
            "id": j.id,
            "id_pai": if j.id_pai == u32::MAX { json!(-1) } else { json!(j.id_pai) },
            "texto": texto(&j.texto),
            "opcoes": j.opcoes.iter().map(|o| {
                let mut v = json!({ "id": o.id, "texto": texto(&o.texto), "param": o.param });
                if o.id & BIT_DE_FUNCAO != 0 {
                    v["funcao"] = funcao(o.id).map(|f| f["rotulo"].clone()).unwrap_or(json!("função desconhecida"));
                }
                v
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// A conversa que veio do painel, validada. Texto igual ao antigo mantém as unidades originais, para
/// uma conversa salva sem mudança gravar os mesmos bytes.
fn conversa_do_json(
    v: &Value,
    antiga: Option<&pw_data_loader::conversas::Conversa>,
    id: u32,
) -> Result<pw_data_loader::conversas::Conversa> {
    use pw_data_loader::conversas::{texto, texto_fixo, texto_variavel, Conversa, Janela, Opcao, BIT_DE_FUNCAO};
    let fixo = |novo: &str, velho: Option<&Vec<u16>>| -> Result<Vec<u16>> {
        match velho {
            Some(u) if texto(u) == novo => Ok(u.clone()),
            _ => texto_fixo(novo).map_err(|e| anyhow::anyhow!("texto {novo:?}: {e}")),
        }
    };
    let variavel = |novo: &str, velho: Option<&Vec<u16>>| -> Result<Vec<u16>> {
        match velho {
            Some(u) if texto(u) == novo => Ok(u.clone()),
            _ => texto_variavel(novo).map_err(|e| anyhow::anyhow!(e)),
        }
    };
    let janelas_v = v["janelas"].as_array().context("janelas")?;
    if janelas_v.is_empty() {
        bail!("a conversa precisa de ao menos uma janela");
    }
    let mut ids = std::collections::HashSet::new();
    for j in janelas_v {
        let jid = j["id"].as_u64().context("id da janela")? as u32;
        if jid & BIT_DE_FUNCAO != 0 {
            bail!("id de janela {jid} com o bit alto (reservado às funções)");
        }
        if !ids.insert(jid) {
            bail!("id de janela repetido: {jid}");
        }
    }
    let mut janelas = Vec::new();
    for j in janelas_v {
        let jid = j["id"].as_u64().unwrap() as u32;
        let pai = match j["id_pai"].as_i64().context("id_pai")? {
            -1 => u32::MAX,
            p if p >= 0 && ids.contains(&(p as u32)) => p as u32,
            p => bail!("janela {jid}: a janela pai {p} não existe"),
        };
        let velha = antiga.and_then(|a| a.janelas.iter().find(|x| x.id == jid));
        let mut opcoes = Vec::new();
        for (k, o) in j["opcoes"].as_array().context("opcoes")?.iter().enumerate() {
            let oid = o["id"].as_u64().context("id da opção")? as u32;
            if oid & BIT_DE_FUNCAO != 0 {
                if funcao(oid).is_none() {
                    bail!("janela {jid}: função {oid:#x} desconhecida");
                }
            } else if !ids.contains(&oid) {
                bail!("janela {jid}: a opção aponta para a janela {oid}, que não existe");
            }
            let velha_o = velha.and_then(|x| x.opcoes.get(k));
            opcoes.push(Opcao {
                id: oid,
                texto: fixo(o["texto"].as_str().unwrap_or(""), velha_o.map(|x| &x.texto))?,
                param: o["param"].as_u64().unwrap_or(0) as u32,
            });
        }
        janelas.push(Janela {
            id: jid,
            id_pai: pai,
            texto: variavel(j["texto"].as_str().unwrap_or(""), velha.map(|x| &x.texto))?,
            opcoes,
        });
    }
    Ok(Conversa { id, texto: fixo(v["texto"].as_str().unwrap_or(""), antiga.map(|a| &a.texto))?, janelas })
}

impl Estudio {
    pub fn funcoes_de_conversa(&self) -> Value {
        json!(funcoes())
    }

    pub async fn listar_conversas(
        &self,
        gov: &Governanca,
        versao: &str,
        rascunho: Option<i64>,
        busca: &str,
        pagina: usize,
        por_pagina: usize,
    ) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let busca = busca.trim().to_lowercase();
        let achados: Vec<Value> = e
            .conversas
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let t = pw_data_loader::conversas::texto(&c.texto).to_lowercase();
                busca.is_empty() || c.id.to_string() == busca || t.contains(&busca)
            })
            .map(|(i, c)| json!({
                "indice": i, "id": c.id, "nome": pw_data_loader::conversas::texto(&c.texto), "janelas": c.janelas.len(),
            }))
            .collect();
        let por_pagina = por_pagina.clamp(1, 200);
        let total = achados.len();
        Ok(json!({
            "total": total, "pagina": pagina, "por_pagina": por_pagina,
            "registros": achados.into_iter().skip(pagina * por_pagina).take(por_pagina).collect::<Vec<_>>(),
        }))
    }

    pub async fn conversa(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>, indice: usize) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let c = e.conversas.get(indice).with_context(|| format!("conversa {indice} inexistente"))?;
        let mut v = conversa_json(c, indice);
        v["usado_por"] = json!(citantes(&e, "TALK", c.id as i32, ("TALK_PROC", indice), false, 100));
        Ok(v)
    }

    pub async fn salvar_conversa(&self, gov: &Governanca, versao: &str, rascunho: i64, indice: usize, conversa: Value) -> Result<Value> {
        self.alterar(gov, versao, rascunho, move |e, resumo| {
            let antiga = e.conversas.get(indice).cloned().with_context(|| format!("conversa {indice} inexistente"))?;
            let nova = conversa_do_json(&conversa, Some(&antiga), antiga.id)?;
            let mudou = nova != antiga;
            if mudou {
                resumo.registro("TALK_PROC", antiga.id as i32, "editada", None);
                e.conversas[indice] = nova;
            }
            Ok(json!({ "id": antiga.id, "mudou": mudou }))
        })
        .await
    }

    /// Conversa nova (cópia de `base`, ou uma janela com "Até logo") com o próximo id do espaço TALK.
    pub async fn nova_conversa(&self, gov: &Governanca, versao: &str, rascunho: i64, base: Option<usize>) -> Result<Value> {
        use pw_data_loader::conversas::{texto_fixo, texto_variavel, Conversa, Janela, Opcao};
        self.alterar(gov, versao, rascunho, move |e, resumo| {
            let id = e.conversas.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            let base_id = base.and_then(|b| e.conversas.get(b)).map(|c| c.id as i32);
            let mut c = match base {
                Some(b) => e.conversas.get(b).cloned().with_context(|| format!("conversa {b} inexistente"))?,
                None => Conversa {
                    id,
                    texto: texto_fixo("Conversa nova").unwrap(),
                    janelas: vec![Janela {
                        id: 1,
                        id_pai: u32::MAX,
                        texto: texto_variavel("Olá!").unwrap(),
                        // TALK_EXIT = 0x80000012 (`enum SERVICE_TYPE`, ExpTypes.h:36-102)
                        opcoes: vec![Opcao { id: 0x8000_0012, texto: texto_fixo("Até logo").unwrap(), param: 0 }],
                    }],
                },
            };
            c.id = id;
            e.conversas.push(c);
            resumo.registro("TALK_PROC", id as i32, "novo", base_id);
            Ok(json!({ "indice": e.conversas.len() - 1, "id": id }))
        })
        .await
    }

    pub async fn remover_conversa(&self, gov: &Governanca, versao: &str, rascunho: i64, indice: usize) -> Result<Value> {
        self.alterar(gov, versao, rascunho, move |e, resumo| {
            let id = e.conversas.get(indice).with_context(|| format!("conversa {indice} inexistente"))?.id;
            let refs = citantes(e, "TALK", id as i32, ("TALK_PROC", indice), true, 50);
            if !refs.is_empty() {
                bail!(
                    "a conversa {id} é usada por {} registro(s), ex.: {}#{}",
                    refs.len(),
                    refs[0]["tabela"].as_str().unwrap_or(""),
                    refs[0]["id"]
                );
            }
            e.conversas.remove(indice);
            resumo.registro("TALK_PROC", id as i32, "removido", None);
            Ok(json!({ "removido": id }))
        })
        .await
    }
}

// ------------------------------------------------------------------ mapas (S4, B242)

/// A pasta que o GS carrega para o mapa (`manager.rs:616-621`): o mundo é `world/` se lá houver
/// `npcgen.data`, senão a raiz do config; os outros, `config/<caminho>/`.
fn pasta_do_mapa(pasta: &std::path::Path, caminho: &str) -> std::path::PathBuf {
    if caminho == "world" && !pasta.join("world").join("npcgen.data").exists() {
        return pasta.to_path_buf();
    }
    pasta.join(caminho)
}

fn caminho_no_mapa(pasta: &std::path::Path, caminho: &str, arquivo: &str) -> Option<std::path::PathBuf> {
    let p = pasta_do_mapa(pasta, caminho).join(arquivo);
    p.exists().then_some(p)
}

fn caminho_de_mapa_valido(c: &str) -> bool {
    !c.is_empty() && c.len() <= 32 && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Estudio {
    /// Um arquivo do cliente publicado, pelo caminho (sem diferenciar maiúsculas).
    async fn arquivo_do_cliente(&self, gov: &Governanca, versao: &str, caminho: &str) -> Result<Vec<u8>> {
        let v = gov.publicado(versao)?.0;
        let indice = {
            let atual = self.arquivos_do_cliente.lock().unwrap().get(versao).cloned();
            match atual {
                Some((sha, i)) if sha == v.manifesto_sha256 => i,
                _ => {
                    let (_, m) = gov.publicado(versao)?;
                    let mut i = HashMap::with_capacity(m.arquivos.len());
                    for a in &m.arquivos {
                        let nome = nomes::legivel(&hex::decode(&a.caminho_gbk)?).to_lowercase();
                        let sha: Sha256Bytes = hex::decode(&a.sha256)?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?;
                        i.insert(nome, (sha, a.tamanho));
                    }
                    let i = Arc::new(i);
                    self.arquivos_do_cliente.lock().unwrap().insert(versao.to_string(), (v.manifesto_sha256.clone(), i.clone()));
                    i
                }
            }
        };
        let (sha, tamanho) = *indice.get(&caminho.to_lowercase()).with_context(|| format!("{caminho} não está no cliente {versao}"))?;
        let objetos = gov.objetos(versao);
        Ok(tokio::task::spawn_blocking(move || objetos.ler(&sha, tamanho as usize)).await??)
    }

    /// Mapas do `Configs\instance.txt` do cliente, com o que o realm tem de cada um.
    pub async fn mapas(&self, gov: &Governanca, versao: &str) -> Result<Value> {
        let bytes = self.arquivo_do_cliente(gov, versao, "configs\\instance.txt").await?;
        let mapas = crate::mapas::ler_instancias(&crate::mapas::texto_do_arquivo(&bytes))?;
        let pasta = gov.realms.get(versao).and_then(|p| p.first()).cloned();
        Ok(json!(mapas
            .iter()
            .map(|m| {
                let npcgen = pasta.as_ref().is_some_and(|p| self.caminho_do_npcgen(p, &m.caminho).is_some());
                json!({ "nome": m.nome, "id": m.id, "caminho": m.caminho, "linhas": m.linhas, "colunas": m.colunas, "npcgen": npcgen })
            })
            .collect::<Vec<_>>()))
    }

    fn caminho_do_npcgen(&self, pasta: &std::path::Path, caminho: &str) -> Option<std::path::PathBuf> {
        caminho_no_mapa(pasta, caminho, "npcgen.data")
    }

    /// Uma peça do minimapa em PNG (`Surfaces\MiniMaps\<caminho>\LLCC.dds`).
    pub async fn peca(&self, gov: &Governanca, versao: &str, caminho: &str, llcc: &str) -> Result<Arc<Vec<u8>>> {
        if !caminho_de_mapa_valido(caminho) || llcc.len() != 4 || !llcc.bytes().all(|b| b.is_ascii_digit()) {
            bail!("peça inválida");
        }
        let chave = (versao.to_string(), caminho.to_string(), llcc.to_string());
        if let Some(p) = self.pecas.lock().unwrap().get(&chave) {
            return Ok(p.clone());
        }
        let dds = self.arquivo_do_cliente(gov, versao, &format!("surfaces\\minimaps\\{caminho}\\{llcc}.dds")).await?;
        let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let (l, a, px) = crate::mapas::dds_para_rgba(&dds)?;
            crate::mapas::png(l, a, &px)
        })
        .await??;
        let png = Arc::new(png);
        let mut c = self.pecas.lock().unwrap();
        if c.len() > 3000 {
            c.clear();
        }
        c.insert(chave, png.clone());
        Ok(png)
    }

    /// Criaturas do `npcgen.data` do realm para o mapa: tipo, modelo, posição, área e renascer, com o nome
    /// do modelo no `elements` publicado.
    pub async fn criaturas(&self, gov: &Governanca, versao: &str, caminho: &str) -> Result<Value> {
        if !caminho_de_mapa_valido(caminho) {
            bail!("mapa inválido");
        }
        let pasta = gov.realms.get(versao).and_then(|p| p.first()).context("a versão não tem realm configurado (PW_RECURSOS_REALMS)")?;
        let arquivo = self.caminho_do_npcgen(pasta, caminho).with_context(|| format!("o realm não tem npcgen para {caminho}"))?;
        let bytes = std::fs::read(&arquivo)?;
        let gen = tokio::task::spawn_blocking(move || pw_data_loader::npcgen::NpcGenData::load_from_bytes(&bytes))
            .await?
            .map_err(|e| anyhow::anyhow!("npcgen: {e}"))?;
        let (e, _) = self.atual(gov, versao, None).await?;
        let ids = mapa_de_ids(&e);
        let mut nomes_por_modelo: HashMap<u32, Value> = HashMap::new();
        let mut pontos = Vec::with_capacity(gen.instances.len());
        for s in &gen.instances {
            let tipo = match s.spawn_type {
                pw_data_loader::npcgen::SpawnType::Monster => "monstro",
                pw_data_loader::npcgen::SpawnType::Npc => "npc",
                pw_data_loader::npcgen::SpawnType::ResourceMine => "recurso",
                pw_data_loader::npcgen::SpawnType::DynamicObject => "objeto",
            };
            nomes_por_modelo.entry(s.template_id).or_insert_with(|| match ids.get(&("ESSENCE".to_string(), s.template_id as i32)) {
                Some(&(ti, ri)) if ti != CONVERSAS => json!({
                    "nome": nome_do_registro(&e.tabelas[ti], ri), "tabela": e.tabelas[ti].nome, "indice": ri,
                }),
                _ => json!({ "nome": null }),
            });
            pontos.push(json!([tipo, s.template_id, (s.pos.x * 10.0).round() / 10.0, (s.pos.z * 10.0).round() / 10.0,
                s.renascer_min_s, (s.extensao_da_area.x * 10.0).round() / 10.0, (s.extensao_da_area.z * 10.0).round() / 10.0]));
        }
        Ok(json!({
            "arquivo": arquivo.file_name().and_then(|n| n.to_str()),
            "colunas_dos_pontos": ["tipo", "modelo", "x", "z", "renascer_s", "area_x", "area_z"],
            "pontos": pontos,
            "modelos": nomes_por_modelo,
        }))
    }
}

// ------------------------------------------------------------------ edição do npcgen (S4, B243)

use pw_data_loader::npcgen_editavel::{
    self as ng, NpcgenEditavel, CAMPOS_DA_AREA, CAMPOS_DA_AREA_DE_RECURSO, CAMPOS_DO_CONTROLADOR, CAMPOS_DO_GERADOR,
    CAMPOS_DO_OBJETO, CAMPOS_DO_RECURSO,
};

/// Áreas de criatura (`AREA` + `NPCGEN`) ou de recurso (`RESAREA` + `RES`), `npcgendata.h:57-102`.
#[derive(Clone, Copy, PartialEq)]
pub enum TipoDeArea {
    Criatura,
    Recurso,
    /// Objeto dinâmico (`NPCGENFILEDYNOBJ*`), sem filhos (B247).
    Objeto,
    /// Controlador (`NPCGENFILECTRL*`), sem filhos (B247).
    Controlador,
}

impl TipoDeArea {
    pub fn de(s: &str) -> Result<Self> {
        match s {
            "areas" => Ok(TipoDeArea::Criatura),
            "recursos" => Ok(TipoDeArea::Recurso),
            "objetos" => Ok(TipoDeArea::Objeto),
            "controladores" => Ok(TipoDeArea::Controlador),
            _ => bail!("tipo de área inválido: {s}"),
        }
    }
    fn campos(self) -> (&'static [ng::Campo], &'static [ng::Campo], &'static str, &'static str) {
        match self {
            TipoDeArea::Criatura => (CAMPOS_DA_AREA, CAMPOS_DO_GERADOR, "área", "gerador"),
            TipoDeArea::Recurso => (CAMPOS_DA_AREA_DE_RECURSO, CAMPOS_DO_RECURSO, "área de recurso", "recurso"),
            TipoDeArea::Objeto => (CAMPOS_DO_OBJETO, &[], "objeto dinâmico", ""),
            TipoDeArea::Controlador => (CAMPOS_DO_CONTROLADOR, &[], "controlador", ""),
        }
    }
    fn nome(self) -> &'static str {
        match self {
            TipoDeArea::Criatura => "área",
            TipoDeArea::Recurso => "área de recurso",
            TipoDeArea::Objeto => "objeto dinâmico",
            TipoDeArea::Controlador => "controlador",
        }
    }
}

fn numero(m: &Map<String, Value>, k: &str) -> f64 {
    m.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

fn um_decimal(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Quem cita o controlador `id` (o `id` interno, não o `iControllerID`): áreas de criatura e de recurso pelo `idCtrl`,
/// objetos pelo `idController` (`npcgendata.h`; o leitor do GS faz o mesmo em `npcgen.rs`).
fn usos_do_controlador(n: &NpcgenEditavel, id: u32) -> Vec<Value> {
    let v = n.versao;
    let mut usos = Vec::new();
    for (i, a) in n.areas.iter().enumerate() {
        if ng::ler_campos(&a.bruto, CAMPOS_DA_AREA, v).get("idCtrl").and_then(Value::as_i64) == Some(id as i64) {
            usos.push(json!(["areas", i]));
        }
    }
    for (i, a) in n.areas_de_recurso.iter().enumerate() {
        if ng::ler_campos(&a.bruto, CAMPOS_DA_AREA_DE_RECURSO, v).get("idCtrl").and_then(Value::as_i64) == Some(id as i64) {
            usos.push(json!(["recursos", i]));
        }
    }
    for (i, o) in n.objetos.iter().enumerate() {
        if ng::ler_campos(o, CAMPOS_DO_OBJETO, v).get("idController").and_then(Value::as_u64) == Some(id as u64) {
            usos.push(json!(["objetos", i]));
        }
    }
    usos
}

/// Os campos que a versão do arquivo tem, `[[nome, tipo]]`, na ordem da struct.
fn campos_da_versao(campos: &[ng::Campo], versao: u32) -> Value {
    json!(campos.iter().filter(|c| versao >= c.3).map(|c| json!([c.0, c.1])).collect::<Vec<_>>())
}

impl Estudio {
    /// O `npcgen.data` do mapa: nome do item (`servidor\<pasta>\npcgen.data`), bytes do realm, e os do
    /// rascunho quando ele já tem o item.
    async fn npcgen_atual(
        &self,
        gov: &Governanca,
        versao: &str,
        caminho: &str,
        rascunho: Option<i64>,
    ) -> Result<(String, Vec<u8>, Option<(ItemDeRecurso, Vec<u8>)>)> {
        self.arquivo_do_mapa(gov, versao, caminho, "npcgen.data", rascunho).await
    }

    /// Um arquivo do servidor do mapa (`npcgen.data`, `precinct.sev`, `region.sev`): nome do item
    /// (`servidor\<pasta>\<arquivo>`), bytes do realm, e os do rascunho quando ele já tem o item.
    async fn arquivo_do_mapa(
        &self,
        gov: &Governanca,
        versao: &str,
        caminho: &str,
        nome_do_arquivo: &str,
        rascunho: Option<i64>,
    ) -> Result<(String, Vec<u8>, Option<(ItemDeRecurso, Vec<u8>)>)> {
        if !caminho_de_mapa_valido(caminho) {
            bail!("mapa inválido");
        }
        let pasta = gov.realms.get(versao).and_then(|p| p.first()).context("a versão não tem realm configurado (PW_RECURSOS_REALMS)")?;
        let arquivo = caminho_no_mapa(pasta, caminho, nome_do_arquivo)
            .with_context(|| format!("o realm não tem {nome_do_arquivo} para {caminho}"))?;
        let nome = if arquivo.parent() == Some(pasta.as_path()) {
            format!("servidor\\{nome_do_arquivo}") // o mundo sem `world/`, na raiz do config
        } else {
            format!("servidor\\{caminho}\\{nome_do_arquivo}")
        };
        self.item_do_realm(gov, versao, nome, rascunho).await
    }

    /// Um arquivo do `config` do realm pelo nome do item (`servidor\…`): bytes do realm e, se o rascunho já tem o
    /// item, os dele.
    async fn item_do_realm(
        &self,
        gov: &Governanca,
        versao: &str,
        nome: String,
        rascunho: Option<i64>,
    ) -> Result<(String, Vec<u8>, Option<(ItemDeRecurso, Vec<u8>)>)> {
        let do_realm = gov.ler_do_realm(versao, &nome)?.with_context(|| format!("{nome} não existe no realm"))?;
        let mut do_rascunho = None;
        if let Some(r) = rascunho {
            gov.rascunho_da_versao(versao, r).await?;
            if let Some(i) = gov.repo.itens(r).await?.into_iter().find(|i| i.caminho == nome.as_bytes()) {
                let sha: Sha256Bytes = i.sha256_depois.clone().context("arquivo removido no rascunho")?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?;
                let objetos = gov.objetos(versao);
                let tamanho = i.tamanho.unwrap_or(0) as usize;
                let b = tokio::task::spawn_blocking(move || objetos.ler(&sha, tamanho)).await??;
                do_rascunho = Some((i, b));
            }
        }
        Ok((nome, do_realm, do_rascunho))
    }

    /// B284: onde um id (monstro, NPC ou mina) nasce — em todos os mapas com npcgen, as áreas de criatura (geradores:
    /// id em `[0..4]`, quantidade em `[4..8]`) e de recurso (id em `[4..8]`, quantidade em `[12..16]`) que o citam, com
    /// os mesmos campos que o mapa desenha.
    pub async fn onde_nasce(&self, gov: &Governanca, versao: &str, id: i64, rascunho: Option<i64>) -> Result<Value> {
        let bytes = self.arquivo_do_cliente(gov, versao, "configs\\instance.txt").await?;
        let mapas = crate::mapas::ler_instancias(&crate::mapas::texto_do_arquivo(&bytes))?;
        let mut r = Vec::new();
        let mut lidos = 0usize;
        for m in &mapas {
            let Ok((_, do_realm, do_rascunho)) = self.npcgen_atual(gov, versao, &m.caminho, rascunho).await else { continue };
            let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
            let Ok(n) = NpcgenEditavel::ler(&bytes) else { continue };
            lidos += 1;
            let v = n.versao;
            for (i, a) in n.areas.iter().enumerate() {
                let qtd: i64 = a.geradores.iter().filter(|g| u32::from_le_bytes(g[0..4].try_into().unwrap()) as i64 == id).map(|g| u32::from_le_bytes(g[4..8].try_into().unwrap()) as i64).sum();
                if a.geradores.iter().any(|g| u32::from_le_bytes(g[0..4].try_into().unwrap()) as i64 == id) {
                    let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA, v);
                    let tipo = if numero(&c, "iNPCType") as i64 == 1 { "npc" } else { "monstro" };
                    r.push(json!({ "mapa": m.nome, "caminho": m.caminho, "tipo": tipo, "area": i, "x": um_decimal(numero(&c, "vPos_x")),
                        "z": um_decimal(numero(&c, "vPos_z")), "quantidade": qtd }));
                }
            }
            for (i, a) in n.areas_de_recurso.iter().enumerate() {
                let deste = |x: &&Vec<u8>| i32::from_le_bytes(x[4..8].try_into().unwrap()) as i64 == id;
                if a.recursos.iter().any(|x| deste(&x)) {
                    let qtd: i64 = a.recursos.iter().filter(deste).map(|x| u32::from_le_bytes(x[12..16].try_into().unwrap()) as i64).sum();
                    let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA_DE_RECURSO, v);
                    r.push(json!({ "mapa": m.nome, "caminho": m.caminho, "tipo": "recurso", "area": i, "x": um_decimal(numero(&c, "vPos_x")),
                        "z": um_decimal(numero(&c, "vPos_z")), "quantidade": qtd }));
                }
            }
        }
        Ok(json!({ "id": id, "mapas_lidos": lidos, "locais": r }))
    }

    /// Resumo do npcgen para o mapa: uma linha por área, com o que a tela desenha.
    pub async fn npcgen(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let (nome, do_realm, do_rascunho) = self.npcgen_atual(gov, versao, caminho, rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let n = NpcgenEditavel::ler(&bytes).map_err(|e| anyhow::anyhow!("npcgen: {e}"))?;
        let v = n.versao;
        let (e, _) = self.atual(gov, versao, None).await?;
        let ids = mapa_de_ids(&e);
        let mut modelos: HashMap<i64, Value> = HashMap::new();
        let mut nomear = |id: i64| {
            modelos.entry(id).or_insert_with(|| match ids.get(&("ESSENCE".to_string(), id as i32)) {
                Some(&(ti, ri)) if ti != CONVERSAS => json!({ "nome": nome_do_registro(&e.tabelas[ti], ri), "tabela": e.tabelas[ti].nome, "indice": ri }),
                _ => json!({ "nome": null }),
            });
        };
        let mut areas = Vec::with_capacity(n.areas.len());
        for (i, a) in n.areas.iter().enumerate() {
            let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA, v);
            let primeiro = a.geradores.first().map(|g| u32::from_le_bytes(g[0..4].try_into().unwrap()) as i64).unwrap_or(0);
            let total: i64 = a.geradores.iter().map(|g| u32::from_le_bytes(g[4..8].try_into().unwrap()) as i64).sum();
            for g in &a.geradores {
                nomear(u32::from_le_bytes(g[0..4].try_into().unwrap()) as i64);
            }
            // NPC_MONSTER = 0, NPC_SERVER = 1 (`npcgendata.h:313-317`)
            let tipo = if numero(&c, "iNPCType") as i64 == 1 { "npc" } else { "monstro" };
            areas.push(json!([i, tipo, um_decimal(numero(&c, "vPos_x")), um_decimal(numero(&c, "vPos_z")),
                um_decimal(numero(&c, "vExts_x")), um_decimal(numero(&c, "vExts_z")), primeiro, a.geradores.len(), total,
                c.get("bInitGen") == Some(&json!(true))]));
        }
        let mut recursos = Vec::with_capacity(n.areas_de_recurso.len());
        for (i, a) in n.areas_de_recurso.iter().enumerate() {
            let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA_DE_RECURSO, v);
            let primeiro = a.recursos.first().map(|r| i32::from_le_bytes(r[4..8].try_into().unwrap()) as i64).unwrap_or(0);
            let total: i64 = a.recursos.iter().map(|r| u32::from_le_bytes(r[12..16].try_into().unwrap()) as i64).sum();
            for r in &a.recursos {
                nomear(i32::from_le_bytes(r[4..8].try_into().unwrap()) as i64);
            }
            recursos.push(json!([i, "recurso", um_decimal(numero(&c, "vPos_x")), um_decimal(numero(&c, "vPos_z")),
                um_decimal(numero(&c, "fExtX")), um_decimal(numero(&c, "fExtZ")), primeiro, a.recursos.len(), total,
                c.get("bInitGen") == Some(&json!(true))]));
        }
        // Modelos dos objetos dinâmicos (B250), do `configs\DynamicObjects.data` do cliente.
        let modelos_dos_objetos = self.modelos_dos_objetos(gov, versao).await;
        // Objetos dinâmicos e controladores (B247). Os controladores levam quantas áreas/objetos os citam.
        let objetos: Vec<Value> = n
            .objetos
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let c = ng::ler_campos(o, CAMPOS_DO_OBJETO, v);
                let modelo = c["dwDynObjID"].as_u64().and_then(|id| modelos_dos_objetos.get(&(id as u32))).cloned();
                json!([i, "objeto", um_decimal(numero(&c, "vPos_x")), um_decimal(numero(&c, "vPos_z")), c["dwDynObjID"], c.get("idController"), modelo])
            })
            .collect();
        let mut citacoes: HashMap<i64, usize> = HashMap::new();
        for a in &n.areas {
            if let Some(c) = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA, v).get("idCtrl").and_then(Value::as_i64).filter(|&c| c != 0) {
                *citacoes.entry(c).or_default() += 1;
            }
        }
        for a in &n.areas_de_recurso {
            if let Some(c) = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA_DE_RECURSO, v).get("idCtrl").and_then(Value::as_i64).filter(|&c| c != 0) {
                *citacoes.entry(c).or_default() += 1;
            }
        }
        for o in &n.objetos {
            if let Some(c) = ng::ler_campos(o, CAMPOS_DO_OBJETO, v).get("idController").and_then(Value::as_i64).filter(|&c| c != 0) {
                *citacoes.entry(c).or_default() += 1;
            }
        }
        let controladores: Vec<Value> = n
            .controladores
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let c = ng::ler_campos(b, CAMPOS_DO_CONTROLADOR, v);
                let id = c["id"].as_i64().unwrap_or(0);
                json!([i, id, c["iControllerID"], nomes::legivel(ng::nome_do_controlador(b)), c["bActived"], citacoes.get(&id).copied().unwrap_or(0)])
            })
            .collect();
        let ids: std::collections::HashSet<i64> = controladores.iter().filter_map(|c| c[1].as_i64()).collect();
        let pendurados: Vec<i64> = citacoes.keys().filter(|c| !ids.contains(c)).copied().collect();
        Ok(json!({
            "item": nome,
            "versao_do_arquivo": v,
            "objetos": objetos,
            "colunas_dos_objetos": ["indice", "tipo", "x", "z", "dwDynObjID", "idController", "modelo"],
            "controladores": controladores,
            "colunas_dos_controladores": ["indice", "id", "iControllerID", "nome", "bActived", "citado_por"],
            "controladores_inexistentes_citados": pendurados,
            "no_rascunho": no_rascunho,
            "colunas": ["indice", "tipo", "x", "z", "ext_x", "ext_z", "primeiro_modelo", "geradores", "quantidade", "nasce_ao_subir"],
            "areas": areas,
            "recursos": recursos,
            "modelos": modelos,
            "campos": {
                "area": campos_da_versao(CAMPOS_DA_AREA, v),
                "gerador": campos_da_versao(CAMPOS_DO_GERADOR, v),
                "area_de_recurso": campos_da_versao(CAMPOS_DA_AREA_DE_RECURSO, v),
                "recurso": campos_da_versao(CAMPOS_DO_RECURSO, v),
                "objeto": campos_da_versao(CAMPOS_DO_OBJETO, v),
                "controlador": campos_da_versao(CAMPOS_DO_CONTROLADOR, v),
            },
        }))
    }

    /// Uma área com seus geradores (ou recursos), campo a campo.
    pub async fn area_do_npcgen(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>, tipo: TipoDeArea, indice: usize) -> Result<Value> {
        let (_, do_realm, do_rascunho) = self.npcgen_atual(gov, versao, caminho, rascunho).await?;
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let n = NpcgenEditavel::ler(&bytes).map_err(|e| anyhow::anyhow!("npcgen: {e}"))?;
        let (ca, cf, _, _) = tipo.campos();
        let vazio = Vec::new();
        let (bruto, filhos) = match tipo {
            TipoDeArea::Criatura => n.areas.get(indice).map(|a| (&a.bruto, &a.geradores)),
            TipoDeArea::Recurso => n.areas_de_recurso.get(indice).map(|a| (&a.bruto, &a.recursos)),
            TipoDeArea::Objeto => n.objetos.get(indice).map(|b| (b, &vazio)),
            TipoDeArea::Controlador => n.controladores.get(indice).map(|b| (b, &vazio)),
        }
        .with_context(|| format!("{} {indice} não existe", tipo.nome()))?;
        let mut r = json!({
            "indice": indice,
            "campos": ng::ler_campos(bruto, ca, n.versao),
            "filhos": filhos.iter().map(|f| ng::ler_campos(f, cf, n.versao)).collect::<Vec<_>>(),
        });
        if tipo == TipoDeArea::Objeto {
            let id = u32::from_le_bytes(bruto[0..4].try_into().unwrap());
            r["nome"] = json!(self.modelos_dos_objetos(gov, versao).await.get(&id));
        }
        if tipo == TipoDeArea::Controlador {
            let id = u32::from_le_bytes(bruto[0..4].try_into().unwrap());
            r["nome"] = json!(nomes::legivel(ng::nome_do_controlador(bruto)));
            r["usado_por"] = json!(usos_do_controlador(&n, id));
        }
        Ok(r)
    }

    /// Aplica uma mudança ao npcgen do mapa no rascunho; se o resultado volta a ser o do realm, o item sai.
    async fn alterar_npcgen<F>(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, mudar: F) -> Result<Value>
    where
        F: FnOnce(&mut NpcgenEditavel) -> Result<(Value, Value)>,
    {
        self.alterar_arquivo_do_mapa(gov, versao, caminho, "npcgen.data", rascunho, move |base| {
            let mut n = NpcgenEditavel::ler(base).map_err(|e| anyhow::anyhow!("npcgen: {e}"))?;
            let (resposta, mudanca) = mudar(&mut n)?;
            Ok((n.gravar(), resposta, mudanca))
        })
        .await
    }

    /// Aplica uma mudança a um arquivo do servidor do mapa no rascunho (item destino `servidor`); se o
    /// resultado volta a ser o do realm, o item sai. `mudar` devolve (bytes novos, resposta, mudança p/ o resumo).
    async fn alterar_arquivo_do_mapa<F>(
        &self,
        gov: &Governanca,
        versao: &str,
        caminho: &str,
        nome_do_arquivo: &str,
        rascunho: i64,
        mudar: F,
    ) -> Result<Value>
    where
        F: FnOnce(&[u8]) -> Result<(Vec<u8>, Value, Value)>,
    {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let (nome, do_realm, do_rascunho) = self.arquivo_do_mapa(gov, versao, caminho, nome_do_arquivo, Some(rascunho)).await?;
        self.gravar_item_do_realm(gov, versao, rascunho, nome, do_realm, do_rascunho, mudar).await
    }

    /// O miolo de toda edição de arquivo do realm: confere que o realm não mudou, aplica `mudar`, guarda o
    /// original no volume (para reverter) e grava o item `servidor` no rascunho — ou o tira, se voltou ao igual.
    /// Quem chama segura a trava do rascunho.
    #[allow(clippy::too_many_arguments)]
    async fn gravar_item_do_realm<F>(
        &self,
        gov: &Governanca,
        versao: &str,
        rascunho: i64,
        nome: String,
        do_realm: Vec<u8>,
        do_rascunho: Option<(ItemDeRecurso, Vec<u8>)>,
        mudar: F,
    ) -> Result<Value>
    where
        F: FnOnce(&[u8]) -> Result<(Vec<u8>, Value, Value)>,
    {
        let sha_do_realm = sha256(&do_realm);
        let mut mudancas = Vec::new();
        if let Some((i, _)) = &do_rascunho {
            if i.sha256_antes.as_deref() != Some(sha_do_realm.as_slice()) {
                bail!("o {nome} do realm mudou depois que entrou no rascunho; tire o item do rascunho e refaça");
            }
            mudancas = i.resumo.get("mudancas").and_then(Value::as_array).cloned().unwrap_or_default();
        }
        let base = do_rascunho.map(|(_, b)| b).unwrap_or_else(|| do_realm.clone());
        let (bytes, resposta, mudanca) = mudar(&base)?;
        let sha = sha256(&bytes);
        if sha == sha_do_realm {
            gov.repo.tirar_item(rascunho, nome.as_bytes()).await?;
            return Ok(json!({ "resultado": resposta, "igual_ao_publicado": true }));
        }
        if !mudanca.is_null() {
            mudancas.push(mudanca);
            if mudancas.len() > 300 {
                mudancas.drain(..mudancas.len() - 300);
            }
        }
        // O arquivo do realm também entra no volume: é o que "reverter" devolve.
        let objetos = Objetos::new(gov.objetos(versao).raiz().to_path_buf(), NIVEL_DO_RASCUNHO);
        let (tam_antes, tam_depois) = (do_realm.len(), bytes.len());
        tokio::task::spawn_blocking(move || -> Result<()> {
            if !objetos.existe(&sha_do_realm) {
                objetos.gravar(&sha_do_realm, &do_realm)?;
            }
            Ok(objetos.gravar(&sha, &bytes)?)
        })
        .await??;
        let item = ItemDeRecurso {
            alteracao: rascunho,
            caminho: nome.as_bytes().to_vec(),
            destino: "servidor".into(),
            acao: "substituir".into(),
            sha256_antes: Some(sha_do_realm.to_vec()),
            sha256_depois: Some(sha.to_vec()),
            tamanho: Some(tam_depois as i64),
            resumo: json!({
                "caminho": nome, "tamanho_antes": tam_antes, "tamanho_depois": tam_depois,
                "mudancas": mudancas,
            }),
        };
        gov.repo.gravar_item(&item).await?;
        Ok(json!({ "resultado": resposta, "igual_ao_publicado": false }))
    }

    /// Troca campos da área e dos filhos (geradores/recursos). `filhos` substitui a lista inteira: os que
    /// sobram ganham os campos dados; os novos partem de uma cópia do último; os que faltam saem.
    #[allow(clippy::too_many_arguments)]
    pub async fn editar_area_do_npcgen(
        &self,
        gov: &Governanca,
        versao: &str,
        caminho: &str,
        rascunho: i64,
        tipo: TipoDeArea,
        indice: usize,
        campos: Map<String, Value>,
        filhos: Option<Vec<Map<String, Value>>>,
    ) -> Result<Value> {
        self.alterar_npcgen(gov, versao, caminho, rascunho, move |n| {
            let v = n.versao;
            let (ca, cf, onde_a, onde_f) = tipo.campos();
            if matches!(tipo, TipoDeArea::Objeto | TipoDeArea::Controlador) {
                if filhos.is_some() {
                    bail!("{} não tem filhos", tipo.nome());
                }
                let bruto = match tipo {
                    TipoDeArea::Objeto => n.objetos.get_mut(indice),
                    _ => n.controladores.get_mut(indice),
                }
                .with_context(|| format!("{} {indice} não existe", tipo.nome()))?;
                if tipo == TipoDeArea::Controlador
                    && campos.get("id").is_some_and(|x| x.as_u64() != Some(u32::from_le_bytes(bruto[0..4].try_into().unwrap()) as u64))
                {
                    bail!("o id do controlador é o que as áreas citam (idCtrl): não se edita; clone para ter outro");
                }
                let antes = ng::ler_campos(bruto, ca, v);
                for (k, valor) in &campos {
                    ng::definir_campo(bruto, ca, v, k, valor, onde_a).map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                let depois = ng::ler_campos(bruto, ca, v);
                let mudados: Map<String, Value> =
                    depois.iter().filter(|(k, x)| antes.get(*k) != Some(*x)).map(|(k, x)| (k.clone(), json!([antes[k], x]))).collect();
                let mudanca = json!({ "acao": "editado", "tipo": tipo.nome(), "indice": indice, "campos": mudados });
                return Ok((json!({ "indice": indice }), mudanca));
            }
            let (bruto, lista, tam) = match tipo {
                TipoDeArea::Criatura => n.areas.get_mut(indice).map(|a| (&mut a.bruto, &mut a.geradores, ng::tamanho_do_gerador(v))),
                TipoDeArea::Recurso => n.areas_de_recurso.get_mut(indice).map(|a| (&mut a.bruto, &mut a.recursos, ng::TAMANHO_DO_RECURSO)),
                _ => unreachable!(),
            }
            .with_context(|| format!("{} {indice} não existe", tipo.nome()))?;
            let antes = ng::ler_campos(bruto, ca, v);
            for (k, valor) in &campos {
                ng::definir_campo(bruto, ca, v, k, valor, onde_a).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            let depois = ng::ler_campos(bruto, ca, v);
            let mudados: Map<String, Value> =
                depois.iter().filter(|(k, x)| antes.get(*k) != Some(*x)).map(|(k, x)| (k.clone(), json!([antes[k], x]))).collect();
            let (n_antes, mut n_depois) = (lista.len(), lista.len());
            if let Some(filhos) = filhos {
                if filhos.is_empty() {
                    bail!("a {} precisa de ao menos um {onde_f}; para tirá-la, remova a área", tipo.nome());
                }
                if filhos.len() > 64 {
                    bail!("no máximo 64 {onde_f}s por área");
                }
                let modelo = lista.last().cloned().unwrap_or_else(|| vec![0u8; tam]);
                lista.resize(filhos.len(), modelo);
                for (f, mapa) in lista.iter_mut().zip(&filhos) {
                    for (k, valor) in mapa {
                        ng::definir_campo(f, cf, v, k, valor, onde_f).map_err(|e| anyhow::anyhow!("{e}"))?;
                    }
                }
                n_depois = lista.len();
            }
            let mudanca = json!({ "acao": "editada", "tipo": tipo.nome(), "indice": indice, "campos": mudados,
                "filhos_antes": n_antes, "filhos_depois": n_depois });
            Ok((json!({ "indice": indice }), mudanca))
        })
        .await
    }

    /// Nova área no fim da lista, cópia de `indice` (o `dwGenID` não é usado pelo GS: só o editor o lê).
    pub async fn clonar_area_do_npcgen(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, tipo: TipoDeArea, indice: usize) -> Result<Value> {
        self.alterar_npcgen(gov, versao, caminho, rascunho, move |n| {
            let novo = match tipo {
                TipoDeArea::Criatura => {
                    let a = n.areas.get(indice).cloned().with_context(|| format!("área {indice} não existe"))?;
                    n.areas.push(a);
                    n.areas.len() - 1
                }
                TipoDeArea::Recurso => {
                    let a = n.areas_de_recurso.get(indice).cloned().with_context(|| format!("área de recurso {indice} não existe"))?;
                    n.areas_de_recurso.push(a);
                    n.areas_de_recurso.len() - 1
                }
                TipoDeArea::Objeto => {
                    let a = n.objetos.get(indice).cloned().with_context(|| format!("objeto {indice} não existe"))?;
                    n.objetos.push(a);
                    n.objetos.len() - 1
                }
                TipoDeArea::Controlador => {
                    // O `id` é a chave que as áreas citam: o clone ganha o maior + 1 (o `iControllerID`, que o
                    // `aipolicy` usa, fica igual — mude-o se o clone for ligado por outro gatilho).
                    let mut a = n.controladores.get(indice).cloned().with_context(|| format!("controlador {indice} não existe"))?;
                    let maior = n.controladores.iter().map(|c| u32::from_le_bytes(c[0..4].try_into().unwrap())).max().unwrap_or(0);
                    a[0..4].copy_from_slice(&(maior + 1).to_le_bytes());
                    n.controladores.push(a);
                    n.controladores.len() - 1
                }
            };
            Ok((json!({ "indice": novo }), json!({ "acao": "clonada", "tipo": tipo.nome(), "indice": novo, "base": indice })))
        })
        .await
    }

    /// Tira a área (as seguintes sobem um índice).
    pub async fn remover_area_do_npcgen(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, tipo: TipoDeArea, indice: usize) -> Result<Value> {
        self.alterar_npcgen(gov, versao, caminho, rascunho, move |n| {
            let (x, z) = match tipo {
                TipoDeArea::Criatura => {
                    if indice >= n.areas.len() {
                        bail!("área {indice} não existe");
                    }
                    let a = n.areas.remove(indice);
                    let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA, n.versao);
                    (numero(&c, "vPos_x"), numero(&c, "vPos_z"))
                }
                TipoDeArea::Recurso => {
                    if indice >= n.areas_de_recurso.len() {
                        bail!("área de recurso {indice} não existe");
                    }
                    let a = n.areas_de_recurso.remove(indice);
                    let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA_DE_RECURSO, n.versao);
                    (numero(&c, "vPos_x"), numero(&c, "vPos_z"))
                }
                TipoDeArea::Objeto => {
                    if indice >= n.objetos.len() {
                        bail!("objeto {indice} não existe");
                    }
                    let o = n.objetos.remove(indice);
                    let c = ng::ler_campos(&o, CAMPOS_DO_OBJETO, n.versao);
                    (numero(&c, "vPos_x"), numero(&c, "vPos_z"))
                }
                TipoDeArea::Controlador => {
                    let c = n.controladores.get(indice).with_context(|| format!("controlador {indice} não existe"))?;
                    let id = u32::from_le_bytes(c[0..4].try_into().unwrap());
                    let usos = usos_do_controlador(n, id);
                    if !usos.is_empty() {
                        bail!("o controlador {id} é citado por {} área(s)/objeto(s) (ex.: {}); mude o idCtrl delas antes",
                            usos.len(), usos.iter().take(3).map(|u| u.to_string()).collect::<Vec<_>>().join(", "));
                    }
                    n.controladores.remove(indice);
                    (0.0, 0.0)
                }
            };
            Ok((json!({ "removida": indice }),
                json!({ "acao": "removida", "tipo": tipo.nome(), "indice": indice, "x": um_decimal(x), "z": um_decimal(z) })))
        })
        .await
    }
}

// ------------------------------------------------------------------ distritos e portais (S4, B244)

use pw_data_loader::precinct::Distritos;
use pw_data_loader::regioes::{RegioesEditavel, RegistroDeRegiao};
use pw_data_loader::clt::{PrecinctDoCliente, RegionDoCliente, RegistroDoCliente};

fn vetor_do_json(v: &Value, onde: &str) -> Result<[f32; 3]> {
    let a = v.as_array().filter(|a| a.len() == 3).with_context(|| format!("{onde}: esperava [x, y, z]"))?;
    let mut s = [0f32; 3];
    for (k, x) in a.iter().enumerate() {
        let f = x.as_f64().filter(|f| f.is_finite() && f.abs() < 1.0e6).with_context(|| format!("{onde}: número inválido"))?;
        s[k] = f as f32;
    }
    Ok(s)
}

fn vetor_json(v: &[f32; 3]) -> Value {
    json!([v[0] as f64, v[1] as f64, v[2] as f64])
}

impl Estudio {
    /// O `.clt` do cliente publicado para o mapa (`Maps\<caminho>\<arquivo>`), com o rascunho; `None` se a versão
    /// não o tem.
    async fn clt_do_mapa(&self, gov: &Governanca, versao: &str, caminho: &str, arquivo: &str, rascunho: Option<i64>) -> Option<Vec<u8>> {
        self.item_do_cliente(gov, versao, &format!("maps\\{caminho}\\{arquivo}"), rascunho).await.ok().map(|t| t.3)
    }

    /// Edição em par (B251): o `.sev` do realm (item `servidor`) e o `.clt` do cliente (item `cliente`) no mesmo
    /// rascunho, sob a mesma trava. `mudar` recebe os bytes atuais dos dois (o `.clt` pode faltar) e devolve os novos
    /// (`None` = o `.clt` não muda).
    #[allow(clippy::too_many_arguments)]
    async fn alterar_sev_e_clt<F>(
        &self,
        gov: &Governanca,
        versao: &str,
        caminho: &str,
        rascunho: i64,
        sev: &str,
        clt: &str,
        mudar: F,
    ) -> Result<Value>
    where
        F: FnOnce(&[u8], Option<&[u8]>) -> Result<(Vec<u8>, Option<Vec<u8>>, Value, Value)>,
    {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let (nome, do_realm, do_rascunho) = self.arquivo_do_mapa(gov, versao, caminho, sev, Some(rascunho)).await?;
        let base = do_rascunho.as_ref().map(|(_, b)| b.clone()).unwrap_or_else(|| do_realm.clone());
        let clt_atual = self.clt_do_mapa(gov, versao, caminho, clt, Some(rascunho)).await;
        let (sev_novo, clt_novo, resposta, mudanca) = mudar(&base, clt_atual.as_deref())?;
        let m2 = mudanca.clone();
        let r1 = self.gravar_item_do_realm(gov, versao, rascunho, nome, do_realm, do_rascunho, move |_| Ok((sev_novo, resposta, mudanca))).await?;
        let mut saida = r1.clone();
        if let Some(c) = clt_novo {
            let caminho_clt = format!("maps\\{caminho}\\{clt}");
            let r2 = self.gravar_item_do_cliente(gov, versao, &caminho_clt, rascunho, move |_| Ok((c, Value::Null, m2))).await?;
            saida["cliente_igual_ao_publicado"] = r2["igual_ao_publicado"].clone();
        }
        Ok(saida)
    }
}

/// O `precinct.clt` casa com o `precinct.sev` (mesmo carimbo, mesma contagem, mesmo número de vértices por distrito)?
fn precinct_alinhado(sev: &Distritos, clt: &PrecinctDoCliente) -> std::result::Result<(), String> {
    if sev.versao >= 5 && sev.carimbo != clt.carimbo {
        return Err(format!("carimbo do precinct.clt ({}) diferente do precinct.sev ({}): o cliente já recusa este mapa", clt.carimbo, sev.carimbo));
    }
    if sev.lista.len() != clt.distritos.len() {
        return Err(format!("{} distritos no .clt e {} no .sev", clt.distritos.len(), sev.lista.len()));
    }
    if let Some(i) = sev.lista.iter().zip(&clt.distritos).position(|(a, b)| a.vertices.len() != b.vertices.len()) {
        return Err(format!("o distrito {i} tem vértices diferentes no .clt e no .sev"));
    }
    Ok(())
}

/// O `region.clt` casa com o `region.sev` (mesmo carimbo e a mesma sequência de regiões e portais — é a ordem que
/// dá o índice do portal no `ENTER_INSTANCE`)?
fn region_alinhado(sev: &RegioesEditavel, clt: &RegionDoCliente) -> std::result::Result<(), String> {
    if sev.carimbo != clt.carimbo {
        return Err(format!("carimbo do region.clt ({}) diferente do region.sev ({}): o cliente já recusa este mapa", clt.carimbo, sev.carimbo));
    }
    let s: Vec<bool> = sev.registros.iter().map(|r| matches!(r, RegistroDeRegiao::Caixa(_))).collect();
    if s != clt.sequencia() {
        return Err(format!("a sequência de regiões/portais do .clt ({}) não é a do .sev ({})", clt.registros.len(), s.len()));
    }
    Ok(())
}

/// `[[x, z], …]` → vértices com a altura `y` (o polígono do distrito usa só x e z: `CELPrecinct::IsPointIn`).
fn vertices_do_json(v: &Value, y: f32) -> Result<Vec<[f32; 3]>> {
    let a = v.as_array().context("vertices: lista de [x, z]")?;
    if a.len() < 3 || a.len() > 512 {
        bail!("vertices: de 3 a 512 pontos");
    }
    a.iter()
        .map(|p| {
            let p = p.as_array().filter(|p| p.len() == 2).context("vertices: cada ponto é [x, z]")?;
            let n = |k: usize| p[k].as_f64().filter(|x| x.is_finite() && x.abs() < 1.0e6).map(|x| x as f32).context("vertices: número inválido");
            Ok([n(0)?, y, n(1)?])
        })
        .collect()
}

fn nome_do_json(v: &Value) -> Result<String> {
    let s = v.as_str().context("nome: texto")?.trim().to_string();
    if s.is_empty() || s.chars().count() > 60 || s.contains(['"', '\n', '\r']) {
        bail!("nome: de 1 a 60 caracteres, sem aspas nem quebra de linha");
    }
    Ok(s)
}

impl Estudio {
    /// Distritos do `precinct.sev` do realm (ponto de renascer: `city_region::GetCityPos`), com nome, id e marcas do
    /// `Maps\<caminho>\precinct.clt` do cliente quando os dois estão alinhados (B251: só então nome, polígono, criar e
    /// remover são permitidos — `alinhado_com_o_cliente` / `motivo`).
    pub async fn distritos(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let (nome, do_realm, do_rascunho) = self.arquivo_do_mapa(gov, versao, caminho, "precinct.sev", rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let d = Distritos::ler(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
        let clt = match self.clt_do_mapa(gov, versao, caminho, "precinct.clt", rascunho).await {
            Some(b) => PrecinctDoCliente::ler(&b).map_err(|e| e.to_string()),
            None => Err("o cliente não tem precinct.clt para este mapa".into()),
        };
        let alinhado = clt.as_ref().map_err(|e| e.clone()).and_then(|c| precinct_alinhado(&d, c).map(|_| c));
        let lista: Vec<Value> = d
            .lista
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let c = alinhado.as_ref().ok().map(|c| &c.distritos[i]);
                json!({
                    "indice": i,
                    "nome": c.map(|c| c.nome.clone()),
                    "id": c.map(|c| c.id()),
                    "mapa": x.mapa,
                    "mapa_do_ponto": x.mapa_do_ponto,
                    "prioridade": x.prioridade,
                    "dominio": x.dominio,
                    "protege_pk": x.protege_pk,
                    "ponto": vetor_json(&x.ponto_de_cidade),
                    "vertices": x.vertices.iter().map(|v| json!([v[0], v[2]])).collect::<Vec<_>>(),
                    "marcas": c.map(|c| c.marcas.iter().map(|(n, p)| json!({ "nome": n, "x": p[0], "z": p[2] })).collect::<Vec<_>>()),
                })
            })
            .collect();
        Ok(json!({
            "item": nome, "versao_do_arquivo": d.versao, "no_rascunho": no_rascunho,
            "nomes_do_cliente": alinhado.is_ok(), "alinhado_com_o_cliente": alinhado.is_ok(), "motivo": alinhado.err(),
            "distritos": lista,
        }))
    }

    /// Distritos em par: aplica `mudar` no `.sev` e, se alinhado, no `.clt`. `exige_cliente` = a operação mexe no que
    /// só existe no cliente (nome, polígono, criar, remover) e é recusada sem o `.clt` alinhado.
    async fn alterar_distritos<F>(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, exige_cliente: bool, mudar: F) -> Result<Value>
    where
        F: FnOnce(&mut Distritos, Option<&mut PrecinctDoCliente>) -> Result<(Value, Value)>,
    {
        self.alterar_sev_e_clt(gov, versao, caminho, rascunho, "precinct.sev", "precinct.clt", move |sev, clt| {
            let mut d = Distritos::ler(sev).map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut c = match clt.map(PrecinctDoCliente::ler) {
                Some(Ok(c)) => match precinct_alinhado(&d, &c) {
                    Ok(()) => Some(c),
                    Err(m) if exige_cliente => bail!("precinct: {m}"),
                    Err(_) => None,
                },
                Some(Err(e)) if exige_cliente => bail!("{e}"),
                _ if exige_cliente => bail!("o cliente não tem precinct.clt para este mapa: nome, polígono, criar e remover exigem o arquivo do cliente"),
                _ => None,
            };
            let (resposta, mudanca) = mudar(&mut d, c.as_mut())?;
            Ok((d.gravar(), c.map(|c| c.gravar()), resposta, mudanca))
        })
        .await
    }

    /// Edita um distrito: ponto de renascer, mapa dele, prioridade, PK (no `.sev`, e no `.clt` se alinhado) e, só com o
    /// `.clt` alinhado, `nome` e `vertices` (`[[x, z]…]`). O carimbo dos dois arquivos não muda.
    pub async fn editar_distrito(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, indice: usize, campos: Map<String, Value>) -> Result<Value> {
        let exige = campos.contains_key("nome") || campos.contains_key("vertices");
        self.alterar_distritos(gov, versao, caminho, rascunho, exige, move |d, mut c| {
            let v = d.versao;
            let x = d.lista.get_mut(indice).with_context(|| format!("distrito {indice} não existe"))?;
            let mut cd = c.as_mut().map(|c| &mut c.distritos[indice]);
            let mut mudou = Map::new();
            for (k, valor) in &campos {
                match k.as_str() {
                    "ponto" => {
                        let novo = vetor_do_json(valor, "ponto")?;
                        if novo != x.ponto_de_cidade {
                            mudou.insert(k.clone(), json!([vetor_json(&x.ponto_de_cidade), vetor_json(&novo)]));
                            x.ponto_de_cidade = novo;
                        }
                        if let Some(cd) = cd.as_mut() {
                            cd.ponto_de_cidade = novo;
                        }
                    }
                    "mapa_do_ponto" | "prioridade" => {
                        let n = valor.as_i64().and_then(|n| i32::try_from(n).ok()).with_context(|| format!("{k}: inteiro"))?;
                        if k == "mapa_do_ponto" && n <= 0 {
                            bail!("mapa_do_ponto: id de mapa inválido");
                        }
                        let alvo = if k == "mapa_do_ponto" { &mut x.mapa_do_ponto } else { &mut x.prioridade };
                        if *alvo != n {
                            mudou.insert(k.clone(), json!([*alvo, n]));
                            *alvo = n;
                        }
                        // Na linha do .clt: prioridade no índice 3, mapa do ponto no 4 (`EL_Precinct.cpp:121-124`).
                        if let Some(cd) = cd.as_mut() {
                            cd.numeros[if k == "mapa_do_ponto" { 4 } else { 3 }] = n as i64;
                        }
                    }
                    "protege_pk" => {
                        if v < 7 {
                            bail!("protege_pk só existe no precinct.sev versão 7 (este é {v})");
                        }
                        let b = valor.as_bool().context("protege_pk: verdadeiro ou falso")?;
                        if x.protege_pk != b {
                            mudou.insert(k.clone(), json!([x.protege_pk, b]));
                            x.protege_pk = b;
                        }
                        if let Some(cd) = cd.as_mut() {
                            if let Some(pk) = cd.numeros.get_mut(10) {
                                *pk = b as i64;
                            }
                        }
                    }
                    "nome" => {
                        let cd = cd.as_mut().context("nome exige o .clt")?;
                        let novo = nome_do_json(valor)?;
                        if novo != cd.nome {
                            mudou.insert(k.clone(), json!([cd.nome, novo]));
                            cd.nome = novo;
                        }
                    }
                    "vertices" => {
                        let cd = cd.as_mut().context("vertices exige o .clt")?;
                        let y = x.vertices.first().map(|p| p[1]).unwrap_or(0.0);
                        let novos = vertices_do_json(valor, y)?;
                        if novos != x.vertices {
                            mudou.insert(k.clone(), json!([x.vertices.len(), novos.len()]));
                            x.definir_vertices(novos.clone());
                            cd.vertices = novos;
                        }
                    }
                    _ => bail!("campo do distrito não editável: {k}"),
                }
            }
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editado", "tipo": "distrito", "indice": indice, "campos": mudou }) };
            Ok((json!({ "indice": indice }), mudanca))
        })
        .await
    }

    /// Distrito novo, cópia do `base` (música, prioridade, mapas, PK) com `nome`, `vertices` e o ponto de renascer
    /// (`ponto`, ou o do base). No `.clt` o id é o maior + 1 e as marcas do base não vêm. Exige o `.clt` alinhado.
    pub async fn novo_distrito(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, base: usize, campos: Map<String, Value>) -> Result<Value> {
        self.alterar_distritos(gov, versao, caminho, rascunho, true, move |d, c| {
            let c = c.context("exige o .clt")?;
            let mut x = d.lista.get(base).cloned().with_context(|| format!("distrito {base} não existe"))?;
            let mut cd = c.distritos[base].clone();
            let y = x.vertices.first().map(|p| p[1]).unwrap_or(0.0);
            let vertices = vertices_do_json(campos.get("vertices").context("vertices")?, y)?;
            x.definir_vertices(vertices.clone());
            cd.vertices = vertices;
            cd.nome = nome_do_json(campos.get("nome").context("nome")?)?;
            cd.numeros[0] = c.distritos.iter().map(|d| d.id()).max().unwrap_or(0) + 1;
            cd.marcas.clear();
            if let Some(p) = campos.get("ponto") {
                x.ponto_de_cidade = vetor_do_json(p, "ponto")?;
            }
            cd.ponto_de_cidade = x.ponto_de_cidade;
            let (indice, id) = (d.lista.len(), cd.id());
            d.lista.push(x);
            c.distritos.push(cd);
            Ok((json!({ "indice": indice, "id": id }), json!({ "acao": "novo", "tipo": "distrito", "indice": indice, "id": id, "base": base })))
        })
        .await
    }

    /// Tira o distrito dos dois arquivos (os seguintes sobem um índice). Exige o `.clt` alinhado.
    pub async fn remover_distrito(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, indice: usize) -> Result<Value> {
        self.alterar_distritos(gov, versao, caminho, rascunho, true, move |d, c| {
            let c = c.context("exige o .clt")?;
            if indice >= d.lista.len() {
                bail!("distrito {indice} não existe");
            }
            d.lista.remove(indice);
            let nome = c.distritos.remove(indice).nome;
            Ok((json!({ "removido": indice }), json!({ "acao": "removido", "tipo": "distrito", "indice": indice, "nome": nome })))
        })
        .await
    }

    /// Portais (caixas de transporte) e regiões do `region.sev` do realm, com o nome das regiões do `region.clt`.
    /// Sem o `.clt` alinhado (`alinhado_com_o_cliente`), só o ponto de chegada é editável: o cliente acha a caixa pela
    /// própria `region.clt` e manda só o índice (`EC_World.cpp:2360-2373`).
    pub async fn portais(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let (nome, do_realm, do_rascunho) = self.arquivo_do_mapa(gov, versao, caminho, "region.sev", rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let r = RegioesEditavel::ler(&bytes)?;
        let clt = match self.clt_do_mapa(gov, versao, caminho, "region.clt", rascunho).await {
            Some(b) => RegionDoCliente::ler(&b).map_err(|e| e.to_string()),
            None => Err("o cliente não tem region.clt para este mapa".into()),
        };
        let alinhado = clt.as_ref().map_err(|e| e.clone()).and_then(|c| region_alinhado(&r, c).map(|_| c));
        let (mut caixas, mut regioes) = (Vec::new(), Vec::new());
        for (n, reg) in r.registros.iter().enumerate() {
            match reg {
                RegistroDeRegiao::Regiao(p) => {
                    let nome = alinhado.as_ref().ok().and_then(|c| match &c.registros[n] {
                        RegistroDoCliente::Regiao { nome, .. } => Some(nome.clone()),
                        _ => None,
                    });
                    regioes.push(json!({ "nome": nome, "vertices": p.iter().map(|v| json!([v[0], v[2]])).collect::<Vec<_>>() }));
                }
                RegistroDeRegiao::Caixa(k) => caixas.push(json!({
                    "indice": caixas.len(), "destino": k.destino, "origem": k.origem, "nivel_minimo": k.nivel_minimo,
                    "pos": vetor_json(&k.pos), "exts": vetor_json(&k.exts), "alvo": vetor_json(&k.alvo),
                })),
            }
        }
        Ok(json!({
            "item": nome, "versao_do_arquivo": r.versao, "no_rascunho": no_rascunho,
            "alinhado_com_o_cliente": alinhado.is_ok(), "motivo": alinhado.err(), "caixas": caixas, "regioes": regioes,
        }))
    }

    async fn alterar_portais<F>(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, exige_cliente: bool, mudar: F) -> Result<Value>
    where
        F: FnOnce(&mut RegioesEditavel, Option<&mut RegionDoCliente>) -> Result<(Value, Value)>,
    {
        self.alterar_sev_e_clt(gov, versao, caminho, rascunho, "region.sev", "region.clt", move |sev, clt| {
            let mut r = RegioesEditavel::ler(sev)?;
            let mut c = match clt.map(RegionDoCliente::ler) {
                Some(Ok(c)) => match region_alinhado(&r, &c) {
                    Ok(()) => Some(c),
                    Err(m) if exige_cliente => bail!("region: {m}"),
                    Err(_) => None,
                },
                Some(Err(e)) if exige_cliente => bail!("{e}"),
                _ if exige_cliente => bail!("o cliente não tem region.clt para este mapa: destino, posição, tamanho, criar e remover exigem o arquivo do cliente"),
                _ => None,
            };
            let (resposta, mudanca) = mudar(&mut r, c.as_mut())?;
            Ok((r.gravar(), c.map(|c| c.gravar()), resposta, mudanca))
        })
        .await
    }

    /// Edita um portal: `alvo` (só no `.sev` se o `.clt` não está alinhado) e, com o `.clt` alinhado, `destino`,
    /// `nivel_minimo` (v≥5), `pos` e `exts` (meia-medida) nos dois arquivos.
    pub async fn editar_portal(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, indice: usize, campos: Map<String, Value>) -> Result<Value> {
        let exige = campos.keys().any(|k| k != "alvo");
        self.alterar_portais(gov, versao, caminho, rascunho, exige, move |r, c| {
            let v = r.versao;
            let mut mudou = Map::new();
            let mut caixas = r.caixas_mut();
            let k = caixas.get_mut(indice).with_context(|| format!("portal {indice} não existe"))?;
            let mut cp = c.and_then(|c| {
                c.registros.iter_mut().filter_map(|x| match x {
                    RegistroDoCliente::Portal { numeros, pos, exts, alvo } => Some((numeros, pos, exts, alvo)),
                    _ => None,
                }).nth(indice)
            });
            for (nome, valor) in &campos {
                match nome.as_str() {
                    "alvo" | "pos" | "exts" => {
                        let novo = vetor_do_json(valor, "vetor")?;
                        if nome == "exts" && novo.iter().any(|x| *x <= 0.0) {
                            bail!("exts: meia-medida positiva");
                        }
                        let atual = match nome.as_str() { "alvo" => &mut k.alvo, "pos" => &mut k.pos, _ => &mut k.exts };
                        if novo != *atual {
                            mudou.insert(nome.clone(), json!([vetor_json(atual), vetor_json(&novo)]));
                            *atual = novo;
                        }
                        if let Some((_, pos, exts, alvo)) = cp.as_mut() {
                            *match nome.as_str() { "alvo" => &mut **alvo, "pos" => &mut **pos, _ => &mut **exts } = novo;
                        }
                    }
                    "destino" | "nivel_minimo" => {
                        let n = valor.as_i64().and_then(|n| i32::try_from(n).ok()).with_context(|| format!("{nome}: inteiro"))?;
                        if nome == "destino" && n <= 0 {
                            bail!("destino: id de mapa inválido");
                        }
                        if nome == "nivel_minimo" && v < 5 {
                            bail!("nivel_minimo só existe no region versão 5");
                        }
                        let atual = if nome == "destino" { &mut k.destino } else { &mut k.nivel_minimo };
                        if *atual != n {
                            mudou.insert(nome.clone(), json!([*atual, n]));
                            *atual = n;
                        }
                        // `[trans]`: destino no índice 0, nível no 2 (`EL_Region.cpp:254-270`).
                        if let Some((numeros, ..)) = cp.as_mut() {
                            numeros[if nome == "destino" { 0 } else { 2 }] = n as i64;
                        }
                    }
                    _ => bail!("campo do portal não editável: {nome}"),
                }
            }
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editado", "tipo": "portal", "indice": indice, "campos": mudou }) };
            Ok((json!({ "indice": indice }), mudanca))
        })
        .await
    }

    /// Portal novo no fim dos dois arquivos (o índice novo é o mesmo nos dois), cópia do `base`. Exige o `.clt`.
    pub async fn novo_portal(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, base: usize) -> Result<Value> {
        self.alterar_portais(gov, versao, caminho, rascunho, true, move |r, c| {
            let c = c.context("exige o .clt")?;
            let k = **r.caixas_mut().get(base).with_context(|| format!("portal {base} não existe"))?;
            let kc = c.registros.iter().filter(|x| matches!(x, RegistroDoCliente::Portal { .. })).nth(base).cloned().context("portal no .clt")?;
            let indice = r.caixas_mut().len();
            r.registros.push(RegistroDeRegiao::Caixa(k));
            c.registros.push(kc);
            Ok((json!({ "indice": indice }), json!({ "acao": "novo", "tipo": "portal", "indice": indice, "base": base })))
        })
        .await
    }

    /// Tira o portal dos dois arquivos (os seguintes sobem um índice nos dois). Exige o `.clt`.
    pub async fn remover_portal(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, indice: usize) -> Result<Value> {
        self.alterar_portais(gov, versao, caminho, rascunho, true, move |r, c| {
            let c = c.context("exige o .clt")?;
            let pos_sev = r.registros.iter().enumerate().filter(|(_, x)| matches!(x, RegistroDeRegiao::Caixa(_))).nth(indice).map(|(i, _)| i)
                .with_context(|| format!("portal {indice} não existe"))?;
            let pos_clt = c.registros.iter().enumerate().filter(|(_, x)| matches!(x, RegistroDoCliente::Portal { .. })).nth(indice).map(|(i, _)| i)
                .context("portal no .clt")?;
            r.registros.remove(pos_sev);
            c.registros.remove(pos_clt);
            Ok((json!({ "removido": indice }), json!({ "acao": "removido", "tipo": "portal", "indice": indice })))
        })
        .await
    }
}

// ------------------------------------------------------------------ teleportes (S4, B245)

use pw_data_loader::world_targets::{PontosDoMundo, TxtDosDestinos};

/// Item do `world_targets.sev`: fica na raiz do `config` (`manager.rs:489`).
const ITEM_DOS_TELEPORTES: &str = "servidor\\world_targets.sev";
/// O mesmo, no cliente (B252).
const DESTINOS_NO_CLIENTE: &str = "maps\\world\\world_targets.txt";

impl Estudio {
    /// Id do mapa (world tag) pelo caminho, do `instance.txt` do cliente.
    async fn id_do_mapa(&self, gov: &Governanca, versao: &str, caminho: &str) -> Result<i32> {
        let bytes = self.arquivo_do_cliente(gov, versao, "configs\\instance.txt").await?;
        let mapas = crate::mapas::ler_instancias(&crate::mapas::texto_do_arquivo(&bytes))?;
        mapas.iter().find(|m| m.caminho == caminho).map(|m| m.id).with_context(|| format!("mapa {caminho} não está no instance.txt"))
    }

    /// Destinos de teleporte (`world_targets.sev`) que ficam no mapa, com nome e domínio do
    /// `maps\world\world_targets.txt` do cliente (`globaldataman.cpp:450-466`) e as transportadoras
    /// (`NPC_TRANSMIT_SERVICE.targets_N_idTarget`, `serviceprovider.cpp:683-700`) que levam a cada um, com preço,
    /// nível e os NPCs que oferecem o serviço (`NPC_ESSENCE.id_transmit_service`). B252: `alinhado_com_o_cliente` diz
    /// se o `.txt` tem os mesmos ids na mesma ordem (só então nome, criar e remover).
    pub async fn teleportes(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        if !caminho_de_mapa_valido(caminho) {
            bail!("mapa inválido");
        }
        let id_mapa = self.id_do_mapa(gov, versao, caminho).await?;
        let (nome, do_realm, do_rascunho) = self.item_do_realm(gov, versao, ITEM_DOS_TELEPORTES.to_string(), rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let w = PontosDoMundo::load_from_bytes(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
        let txt = match self.item_do_cliente(gov, versao, DESTINOS_NO_CLIENTE, rascunho).await {
            Ok(t) => TxtDosDestinos::ler(&t.3).map_err(|e| e.to_string()),
            Err(_) => Err("o cliente não tem maps\\world\\world_targets.txt".to_string()),
        };
        let alinhado = txt.as_ref().map_err(|e| e.clone()).and_then(|t| destinos_alinhados(&w, t).map(|_| t));
        let nomes: HashMap<i32, String> = match &txt {
            Ok(t) => t.destinos.iter().map(|d| (d.id, d.nome.clone())).collect(),
            Err(_) => HashMap::new(),
        };
        let usado_por = self.transportadoras(gov, versao, rascunho).await?;
        let pontos: Vec<Value> = w
            .lista
            .iter()
            .filter(|p| p.mundo == id_mapa)
            .map(|p| json!({
                "id": p.id, "nome": nomes.get(&p.id), "dominio": p.dominio, "pos": vetor_json(&p.pos),
                "usado_por": usado_por.get(&p.id).cloned().unwrap_or_default(),
            }))
            .collect();
        Ok(json!({
            "item": nome, "mapa": id_mapa, "no_rascunho": no_rascunho, "total": w.lista.len(), "pontos": pontos,
            "alinhado_com_o_cliente": alinhado.is_ok(), "motivo": alinhado.err(),
        }))
    }

    /// Quem leva a cada destino: as transportadoras do `elements` (do rascunho, se houver) e os NPCs delas.
    async fn transportadoras(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<HashMap<i32, Vec<Value>>> {
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let mut npcs_do_servico: HashMap<i32, Vec<Value>> = HashMap::new();
        if let Ok(t) = e.tabela("NPC_ESSENCE") {
            for i in 0..t.registros.len() {
                let Ok(r) = t.ler(i) else { continue };
                if let Some(s) = r.get("id_transmit_service").and_then(|v| v.as_i32()).filter(|&s| s > 0) {
                    npcs_do_servico.entry(s).or_default().push(json!({ "nome": nome_do_registro(t, i), "indice": i }));
                }
            }
        }
        let mut usado_por: HashMap<i32, Vec<Value>> = HashMap::new();
        if let Ok(t) = e.tabela("NPC_TRANSMIT_SERVICE") {
            for i in 0..t.registros.len() {
                let Ok(r) = t.ler(i) else { continue };
                let n = r.get("num_targets").and_then(|v| v.as_i32()).unwrap_or(0).clamp(0, 32);
                let id_servico = t.id(i).unwrap_or(0);
                for k in 0..n {
                    let campo = |c: &str| r.get(&format!("targets_{k}_{c}")).and_then(|v| v.as_i32()).unwrap_or(0);
                    usado_por.entry(campo("idTarget")).or_default().push(json!({
                        "servico": id_servico, "indice": i, "nome": nome_do_registro(t, i),
                        "preco": campo("fee"), "nivel": campo("required_level"),
                        "npcs": npcs_do_servico.get(&id_servico).cloned().unwrap_or_default(),
                    }));
                }
            }
        }
        Ok(usado_por)
    }

    /// Destinos em par (B252): `world_targets.sev` (item `servidor`) e `world_targets.txt` do cliente (item `cliente`)
    /// no mesmo rascunho. Com o `.txt` alinhado a posição vai aos dois, arredondada a 3 casas como o `globalsave` faz;
    /// sem ele, só o `.sev` (com a posição exata) e só a posição.
    async fn alterar_destinos<F>(&self, gov: &Governanca, versao: &str, rascunho: i64, exige_cliente: bool, mudar: F) -> Result<Value>
    where
        F: FnOnce(&mut PontosDoMundo, Option<&mut TxtDosDestinos>) -> Result<(Value, Value)>,
    {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let (nome, do_realm, do_rascunho) = self.item_do_realm(gov, versao, ITEM_DOS_TELEPORTES.to_string(), Some(rascunho)).await?;
        let base = do_rascunho.as_ref().map(|(_, b)| b.clone()).unwrap_or_else(|| do_realm.clone());
        let mut w = PontosDoMundo::load_from_bytes(&base).map_err(|e| anyhow::anyhow!("{e}"))?;
        let txt = self.item_do_cliente(gov, versao, DESTINOS_NO_CLIENTE, Some(rascunho)).await.ok().map(|t| t.3);
        let mut t = match txt.as_deref().map(TxtDosDestinos::ler) {
            Some(Ok(t)) => match destinos_alinhados(&w, &t) {
                Ok(()) => Some(t),
                Err(m) if exige_cliente => bail!("world_targets: {m}"),
                Err(_) => None,
            },
            Some(Err(e)) if exige_cliente => bail!("{e}"),
            _ if exige_cliente => bail!("o cliente não tem maps\\world\\world_targets.txt: nome, criar e remover exigem o arquivo do cliente"),
            _ => None,
        };
        let (resposta, mudanca) = mudar(&mut w, t.as_mut())?;
        let (sev_novo, txt_novo, m2) = (w.gravar(), t.map(|t| t.gravar()), mudanca.clone());
        let mut saida = self.gravar_item_do_realm(gov, versao, rascunho, nome, do_realm, do_rascunho, move |_| Ok((sev_novo, resposta, mudanca))).await?;
        if let Some(b) = txt_novo {
            let r2 = self.gravar_item_do_cliente(gov, versao, DESTINOS_NO_CLIENTE, rascunho, move |_| Ok((b, Value::Null, m2))).await?;
            saida["cliente_igual_ao_publicado"] = r2["igual_ao_publicado"].clone();
        }
        Ok(saida)
    }

    /// Edita um destino: `pos` (nos dois, ou só no `.sev` sem o `.txt` alinhado) e `nome` (só o cliente tem; exige o
    /// `.txt`). O mapa e o domínio ficam.
    pub async fn editar_teleporte(&self, gov: &Governanca, versao: &str, rascunho: i64, id: i32, campos: Map<String, Value>) -> Result<Value> {
        let exige = campos.keys().any(|k| k != "pos");
        self.alterar_destinos(gov, versao, rascunho, exige, move |w, mut t| {
            let mut mudou = Map::new();
            for (k, v) in &campos {
                match k.as_str() {
                    "pos" => {
                        let mut novo = vetor_do_json(v, "pos")?;
                        if t.is_some() {
                            novo = TxtDosDestinos::posicao_do_texto(novo);
                        }
                        let antes = w.get(id).with_context(|| format!("destino {id} não existe"))?.pos;
                        if novo != antes {
                            w.definir_posicao(id, novo);
                            mudou.insert(k.clone(), json!([vetor_json(&antes), vetor_json(&novo)]));
                        }
                        if let Some(d) = t.as_mut().and_then(|t| t.destinos.iter_mut().find(|d| d.id == id)) {
                            d.pos = novo;
                        }
                    }
                    "nome" => {
                        let d = t.as_mut().and_then(|t| t.destinos.iter_mut().find(|d| d.id == id)).context("nome exige o .txt")?;
                        let novo = nome_do_json(v)?;
                        if novo.contains('\t') {
                            bail!("nome: sem tabulação");
                        }
                        if novo != d.nome {
                            mudou.insert(k.clone(), json!([d.nome, novo]));
                            d.nome = novo;
                        }
                    }
                    _ => bail!("campo do destino não editável: {k}"),
                }
            }
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editado", "tipo": "teleporte", "id": id, "campos": mudou }) };
            Ok((json!({ "id": id }), mudanca))
        })
        .await
    }

    /// Destino novo no fim dos dois arquivos: cópia do `base` (domínio) no mapa `mundo`, com `nome` e `pos`; o id é o
    /// maior + 1. Fica sem transportadora até alguma `NPC_TRANSMIT_SERVICE` listá-lo (ficha do serviço).
    pub async fn novo_teleporte(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, base: i32, campos: Map<String, Value>) -> Result<Value> {
        let mundo = self.id_do_mapa(gov, versao, caminho).await?;
        self.alterar_destinos(gov, versao, rascunho, true, move |w, t| {
            let t = t.context("exige o .txt")?;
            let b = *w.get(base).with_context(|| format!("destino {base} não existe"))?;
            let nome = nome_do_json(campos.get("nome").context("nome")?)?;
            if nome.contains('\t') {
                bail!("nome: sem tabulação");
            }
            let pos = TxtDosDestinos::posicao_do_texto(vetor_do_json(campos.get("pos").context("pos")?, "pos")?);
            let id = w.lista.iter().map(|p| p.id).max().unwrap_or(0) + 1;
            let p = pw_data_loader::world_targets::PontoDoMundo { id, mundo, pos, dominio: b.dominio };
            w.lista.push(p);
            w.por_id.insert(id, p);
            t.destinos.push(pw_data_loader::world_targets::DestinoDoCliente { id, nome, mundo, pos, dominio: b.dominio });
            Ok((json!({ "id": id }), json!({ "acao": "novo", "tipo": "teleporte", "id": id, "base": base, "mundo": mundo })))
        })
        .await
    }

    /// Tira o destino dos dois arquivos; recusado se alguma transportadora (no `elements` do rascunho) ainda leva a ele.
    pub async fn remover_teleporte(&self, gov: &Governanca, versao: &str, rascunho: i64, id: i32) -> Result<Value> {
        let usos = self.transportadoras(gov, versao, Some(rascunho)).await?.remove(&id).unwrap_or_default();
        if !usos.is_empty() {
            bail!("o destino {id} é usado por {} transportadora(s) (ex.: {}); tire-o delas antes", usos.len(),
                usos.iter().take(3).map(|u| u["nome"].as_str().unwrap_or("?").to_string()).collect::<Vec<_>>().join(", "));
        }
        self.alterar_destinos(gov, versao, rascunho, true, move |w, t| {
            let t = t.context("exige o .txt")?;
            let antes = w.lista.len();
            w.lista.retain(|p| p.id != id);
            if w.lista.len() == antes {
                bail!("destino {id} não existe");
            }
            w.por_id.remove(&id);
            t.destinos.retain(|d| d.id != id);
            Ok((json!({ "removido": id }), json!({ "acao": "removido", "tipo": "teleporte", "id": id })))
        })
        .await
    }
}

/// O `.txt` do cliente tem os mesmos destinos que o `.sev`, na mesma ordem? (o `globalsave` gera um do outro).
fn destinos_alinhados(w: &PontosDoMundo, t: &TxtDosDestinos) -> std::result::Result<(), String> {
    let a: Vec<i32> = w.lista.iter().map(|p| p.id).collect();
    let b: Vec<i32> = t.destinos.iter().map(|d| d.id).collect();
    if a != b {
        return Err(format!("o world_targets.txt ({} destinos) não tem os mesmos ids, na mesma ordem, que o .sev ({})", b.len(), a.len()));
    }
    Ok(())
}

// ------------------------------------------------------------------ rastreador de NPCs (task_npc.data, S4, B246)

use pw_data_loader::task_npc::Rastreador;

/// O `task_npc.data` é do cliente (lido do disco, `EC_TaskInterface.cpp:166`).
const CAMINHO_DO_RASTREADOR: &str = "data\\task_npc.data";

impl Estudio {
    /// Um arquivo do cliente pelo caminho (sem diferenciar maiúsculas): o caminho GBK exato do manifesto, o SHA e
    /// tamanho publicados, os bytes atuais (do rascunho se ele já tem o item) e o item.
    async fn item_do_cliente(
        &self,
        gov: &Governanca,
        versao: &str,
        caminho: &str,
        rascunho: Option<i64>,
    ) -> Result<(Vec<u8>, Sha256Bytes, usize, Vec<u8>, Option<ItemDeRecurso>)> {
        let (_, m) = gov.publicado(versao)?;
        let alvo = caminho.to_lowercase();
        let a = m
            .arquivos
            .iter()
            .find(|a| hex::decode(&a.caminho_gbk).map(|g| nomes::legivel(&g).to_lowercase() == alvo).unwrap_or(false))
            .with_context(|| format!("{caminho} não está no cliente {versao}"))?;
        let gbk = hex::decode(&a.caminho_gbk)?;
        let sha: Sha256Bytes = hex::decode(&a.sha256)?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?;
        let tamanho = a.tamanho as usize;
        let objetos = gov.objetos(versao);
        if let Some(r) = rascunho {
            gov.rascunho_da_versao(versao, r).await?;
            if let Some(i) = gov.repo.itens(r).await?.into_iter().find(|i| i.caminho == gbk) {
                let s: Sha256Bytes = i.sha256_depois.clone().context("arquivo removido no rascunho")?.try_into().map_err(|_| anyhow::anyhow!("sha256"))?;
                let t = i.tamanho.unwrap_or(0) as usize;
                let b = tokio::task::spawn_blocking(move || objetos.ler(&s, t)).await??;
                return Ok((gbk, sha, tamanho, b, Some(i)));
            }
        }
        let b = tokio::task::spawn_blocking(move || objetos.ler(&sha, tamanho)).await??;
        Ok((gbk, sha, tamanho, b, None))
    }

    /// Edição de um arquivo do cliente no rascunho (item destino `cliente`: entra no manifesto e o launcher baixa);
    /// volta ao publicado → o item sai. `mudar` devolve (bytes novos, resposta, mudança p/ o resumo).
    async fn alterar_item_do_cliente<F>(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, mudar: F) -> Result<Value>
    where
        F: FnOnce(&[u8]) -> Result<(Vec<u8>, Value, Value)>,
    {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        self.gravar_item_do_cliente(gov, versao, caminho, rascunho, mudar).await
    }

    /// O miolo de `alterar_item_do_cliente`, sem a trava (quem chama a segura) — B251, para editar em par com o `.sev`.
    async fn gravar_item_do_cliente<F>(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, mudar: F) -> Result<Value>
    where
        F: FnOnce(&[u8]) -> Result<(Vec<u8>, Value, Value)>,
    {
        self.gravar_item_com_destino(gov, versao, caminho, rascunho, "cliente", mudar).await
    }

    /// O mesmo, com o destino do item: `cliente` (só o manifesto) ou `ambos` (o manifesto e o `config` do realm, como o
    /// `elements.data` e o `gshop.data`, que o realm usa idênticos — B254).
    async fn gravar_item_com_destino<F>(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, destino: &'static str, mudar: F) -> Result<Value>
    where
        F: FnOnce(&[u8]) -> Result<(Vec<u8>, Value, Value)>,
    {
        let (gbk, sha_publicado, tam_publicado, atual, item) = self.item_do_cliente(gov, versao, caminho, Some(rascunho)).await?;
        let mut mudancas = item.as_ref().and_then(|i| i.resumo.get("mudancas")).and_then(Value::as_array).cloned().unwrap_or_default();
        let (bytes, resposta, mudanca) = mudar(&atual)?;
        let sha = sha256(&bytes);
        if sha == sha_publicado {
            gov.repo.tirar_item(rascunho, &gbk).await?;
            return Ok(json!({ "resultado": resposta, "igual_ao_publicado": true }));
        }
        if !mudanca.is_null() {
            mudancas.push(mudanca);
            if mudancas.len() > 300 {
                mudancas.drain(..mudancas.len() - 300);
            }
        }
        let objetos = Objetos::new(gov.objetos(versao).raiz().to_path_buf(), NIVEL_DO_RASCUNHO);
        let tamanho = bytes.len();
        tokio::task::spawn_blocking(move || objetos.gravar(&sha, &bytes)).await??;
        gov.repo
            .gravar_item(&ItemDeRecurso {
                alteracao: rascunho,
                caminho: gbk,
                destino: destino.into(),
                acao: "substituir".into(),
                sha256_antes: Some(sha_publicado.to_vec()),
                sha256_depois: Some(sha.to_vec()),
                tamanho: Some(tamanho as i64),
                resumo: json!({ "caminho": caminho, "tamanho_antes": tam_publicado, "tamanho_depois": tamanho, "mudancas": mudancas }),
            })
            .await?;
        Ok(json!({ "resultado": resposta, "igual_ao_publicado": false }))
    }

    /// NPCs das áreas de NPC do `npcgen` do mapa (`iNPCType` = 1) com a posição que o rastreador de missões do
    /// cliente tem para cada um (`task_npc.data`) e a distância até a área mais perto. O cliente só consulta o
    /// rastreador no mapa principal (`MAJOR_MAP` = 1, `DlgWorldMap.h:13`; `DlgNameLink.cpp:240`): nos outros mapas a
    /// resposta diz `usado_pelo_cliente: false`.
    pub async fn rastreador(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let id_mapa = self.id_do_mapa(gov, versao, caminho).await?;
        let (_, do_realm, do_rascunho) = self.npcgen_atual(gov, versao, caminho, rascunho).await?;
        let ng = NpcgenEditavel::ler(&do_rascunho.map(|(_, b)| b).unwrap_or(do_realm)).map_err(|e| anyhow::anyhow!("npcgen: {e}"))?;
        let (_, _, _, bytes, item) = self.item_do_cliente(gov, versao, CAMINHO_DO_RASTREADOR, rascunho).await?;
        let r = Rastreador::ler(&bytes)?;
        let mut areas_por_npc: std::collections::BTreeMap<u32, Vec<(usize, f64, f64)>> = Default::default();
        for (i, a) in ng.areas.iter().enumerate() {
            let c = ng::ler_campos(&a.bruto, CAMPOS_DA_AREA, ng.versao);
            if numero(&c, "iNPCType") as i64 != 1 {
                continue;
            }
            for g in &a.geradores {
                let id = u32::from_le_bytes(g[0..4].try_into().unwrap());
                areas_por_npc.entry(id).or_default().push((i, numero(&c, "vPos_x"), numero(&c, "vPos_z")));
            }
        }
        let (e, _) = self.atual(gov, versao, None).await?;
        let ids = mapa_de_ids(&e);
        let npcs: Vec<Value> = areas_por_npc
            .iter()
            .map(|(&id, areas)| {
                let pos = r.posicao(id);
                let distancia = pos.map(|p| {
                    areas.iter().map(|&(_, x, z)| ((p[0] as f64 - x).powi(2) + (p[2] as f64 - z).powi(2)).sqrt()).fold(f64::MAX, f64::min)
                });
                let nome = match ids.get(&("ESSENCE".to_string(), id as i32)) {
                    Some(&(ti, ri)) if ti != CONVERSAS => json!(nome_do_registro(&e.tabelas[ti], ri)),
                    _ => Value::Null,
                };
                json!({
                    "id": id, "nome": nome, "rastreador": pos,
                    "areas": areas.iter().map(|&(i, x, z)| json!([i, um_decimal(x), um_decimal(z)])).collect::<Vec<_>>(),
                    "distancia": distancia.map(um_decimal),
                })
            })
            .collect();
        Ok(json!({ "item": CAMINHO_DO_RASTREADOR, "no_rascunho": item.is_some(), "total": r.npcs.len(),
            "usado_pelo_cliente": id_mapa == 1, "npcs": npcs }))
    }

    /// Põe o NPC `id` no rastreador na posição dada (arredondada: o campo é `short`); acrescenta se não houver.
    pub async fn editar_rastreador(&self, gov: &Governanca, versao: &str, rascunho: i64, id: u32, campos: Map<String, Value>) -> Result<Value> {
        self.alterar_item_do_cliente(gov, versao, CAMINHO_DO_RASTREADOR, rascunho, move |base| {
            let mut r = Rastreador::ler(base)?;
            let mut mudanca = Value::Null;
            for (k, v) in &campos {
                if k != "pos" {
                    bail!("só a posição do NPC no rastreador é editável: {k}");
                }
                let novo = vetor_do_json(v, "pos")?;
                let antes = r.posicao(id);
                let existia = r.definir_posicao(id, novo)?;
                if r.posicao(id) != antes {
                    mudanca = json!({ "acao": if existia { "editado" } else { "novo" }, "tipo": "rastreador", "id": id,
                        "campos": { "pos": [antes, r.posicao(id)] } });
                }
            }
            Ok((r.gravar(), json!({ "id": id, "pos": r.posicao(id) }), mudanca))
        })
        .await
    }
}

// ------------------------------------------------------------------ altura do chão (S4, B248)

impl Estudio {
    /// Altura do chão em (x, z) no mapa, pelos `.hmap` da pasta do mapa no realm — o mesmo leitor do GS
    /// (`pw_data_loader::Terreno`, a conta de `GetHeightAt`: `playercmd.cpp:4926`, `npcgenerator.cpp:4319-4322`).
    /// `y` nulo quando o realm não tem o bloco daquele ponto.
    pub async fn altura(&self, gov: &Governanca, versao: &str, caminho: &str, x: f32, z: f32) -> Result<Value> {
        if !caminho_de_mapa_valido(caminho) || !x.is_finite() || !z.is_finite() {
            bail!("mapa ou coordenada inválidos");
        }
        let chave = (versao.to_string(), caminho.to_string());
        let existente = self.terrenos.lock().unwrap().get(&chave).cloned();
        let terreno = match existente {
            Some(t) => t,
            None => {
                let id = self.id_do_mapa(gov, versao, caminho).await?;
                let pasta = gov.realms.get(versao).and_then(|p| p.first()).context("a versão não tem realm configurado (PW_RECURSOS_REALMS)")?;
                let dir = pasta_do_mapa(pasta, caminho);
                let t = Arc::new(tokio::task::spawn_blocking(move || pw_data_loader::Terreno::ler(id, &dir)).await?);
                let mut c = self.terrenos.lock().unwrap();
                if c.len() >= 8 {
                    c.clear(); // o mundo inteiro ocupa bastante: poucos mapas em memória
                }
                c.insert(chave, t.clone());
                t
            }
        };
        let y = terreno.altura_em(x, z);
        Ok(json!({ "x": x, "z": z, "y": y.map(|y| (y * 100.0).round() / 100.0), "tem_terreno": terreno.tem_dados() }))
    }
}

// ------------------------------------------------------------------ modelos dos objetos dinâmicos (S4, B250)

impl Estudio {
    /// `dwDynObjID` → caminho do modelo, do `configs\DynamicObjects.data` do cliente publicado. Vazio (com aviso no
    /// log) se a versão não tem o arquivo ou ele não fecha.
    async fn modelos_dos_objetos(&self, gov: &Governanca, versao: &str) -> HashMap<u32, String> {
        match self.arquivo_do_cliente(gov, versao, "configs\\dynamicobjects.data").await {
            Ok(b) => crate::mapas::ler_objetos_dinamicos(&b).unwrap_or_else(|e| {
                tracing::warn!("{versao}: {e:#}");
                HashMap::new()
            }),
            Err(_) => HashMap::new(),
        }
    }
}

// ------------------------------------------------------------------ alinhar o servidor com o cliente (S4, B253)

impl Estudio {
    /// O par (`.clt` do cliente, `.sev` do realm) de um mapa: estado de `precinct` e `region` — equivalentes (o `.sev`
    /// gerado do `.clt` casa com o do realm, `clt::precinct_equivalente`) ou não, com carimbos e contagens.
    async fn par_do_mapa(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let mut saida = Map::new();
        for (clt, sev) in [("precinct.clt", "precinct.sev"), ("region.clt", "region.sev")] {
            let Ok((_, do_realm, do_rascunho)) = self.arquivo_do_mapa(gov, versao, caminho, sev, rascunho).await else { continue };
            let s = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
            let Some(c) = self.clt_do_mapa(gov, versao, caminho, clt, rascunho).await else {
                saida.insert(sev.into(), json!({ "cliente": false }));
                continue;
            };
            let estado = if clt == "precinct.clt" {
                let real = Distritos::ler(&s).map_err(|e| anyhow::anyhow!("{e}"))?;
                let c = PrecinctDoCliente::ler(&c)?;
                let gerado = c.para_o_servidor(real.versao);
                json!({ "cliente": true, "equivalente": pw_data_loader::clt::precinct_equivalente(&gerado, &real),
                    "carimbo_cliente": c.carimbo, "carimbo_servidor": real.carimbo, "no_cliente": c.distritos.len(), "no_servidor": real.lista.len() })
            } else {
                let real = RegioesEditavel::ler(&s)?;
                let c = RegionDoCliente::ler(&c)?;
                let gerado = c.para_o_servidor(real.versao);
                json!({ "cliente": true, "equivalente": pw_data_loader::clt::region_equivalente(&gerado, &real),
                    "carimbo_cliente": c.carimbo, "carimbo_servidor": real.carimbo, "no_cliente": c.registros.len(), "no_servidor": real.registros.len() })
            };
            saida.insert(sev.into(), estado);
        }
        Ok(Value::Object(saida))
    }

    /// Todos os mapas do `instance.txt` com o estado do par cliente × servidor.
    pub async fn alinhamento(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<Value> {
        let bytes = self.arquivo_do_cliente(gov, versao, "configs\\instance.txt").await?;
        let mapas = crate::mapas::ler_instancias(&crate::mapas::texto_do_arquivo(&bytes))?;
        let mut lista = Vec::new();
        for m in mapas {
            let par = self.par_do_mapa(gov, versao, &m.caminho, rascunho).await?;
            if par.as_object().is_some_and(|o| !o.is_empty()) {
                let desalinhado = par.as_object().unwrap().values().any(|e| e["cliente"] == true && e["equivalente"] == false);
                lista.push(json!({ "caminho": m.caminho, "nome": m.nome, "id": m.id, "desalinhado": desalinhado, "arquivos": par }));
            }
        }
        Ok(json!(lista))
    }

    /// Refaz no rascunho o `precinct.sev`/`region.sev` do mapa a partir do `.clt` do cliente (o cliente é o juiz: é o
    /// `.clt` que ele usa para nomes, música e o índice do portal, e cujos carimbos ele confere no
    /// `INST_DATA_CHECKOUT`, `EC_World.cpp:2389-2409`). Só os arquivos que não são equivalentes; a versão do `.sev`
    /// fica a do realm. Nada vai ao cliente.
    pub async fn alinhar_com_o_cliente(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64) -> Result<Value> {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let mut feitos = Vec::new();
        for (clt, sev) in [("precinct.clt", "precinct.sev"), ("region.clt", "region.sev")] {
            let Ok((nome, do_realm, do_rascunho)) = self.arquivo_do_mapa(gov, versao, caminho, sev, Some(rascunho)).await else { continue };
            let Some(c) = self.clt_do_mapa(gov, versao, caminho, clt, Some(rascunho)).await else { continue };
            let base = do_rascunho.as_ref().map(|(_, b)| b.clone()).unwrap_or_else(|| do_realm.clone());
            let (novo, antes, depois) = if clt == "precinct.clt" {
                let real = Distritos::ler(&base).map_err(|e| anyhow::anyhow!("{e}"))?;
                let g = PrecinctDoCliente::ler(&c)?.para_o_servidor(real.versao);
                if pw_data_loader::clt::precinct_equivalente(&g, &real) {
                    continue;
                }
                (g.gravar(), json!([real.carimbo, real.lista.len()]), json!([g.carimbo, g.lista.len()]))
            } else {
                let real = RegioesEditavel::ler(&base)?;
                let g = RegionDoCliente::ler(&c)?.para_o_servidor(real.versao);
                if pw_data_loader::clt::region_equivalente(&g, &real) {
                    continue;
                }
                (g.gravar(), json!([real.carimbo, real.registros.len()]), json!([g.carimbo, g.registros.len()]))
            };
            let mudanca = json!({ "acao": "alinhado com o cliente", "tipo": sev, "antes_carimbo_e_registros": antes, "depois": depois });
            self.gravar_item_do_realm(gov, versao, rascunho, nome, do_realm, do_rascunho, move |_| Ok((novo, json!({}), mudanca))).await?;
            feitos.push(json!({ "arquivo": sev, "antes": antes, "depois": depois }));
        }
        Ok(json!({ "alinhados": feitos }))
    }
}

// ------------------------------------------------------------------ Loja Gold (gshop*.data, S7, B254)

use pw_data_loader::gshop_editavel::{self as gs, GshopEditavel};

/// `data\gshop.data` (a loja), `gshop1`/`gshop2` (as outras do 1.5.5). O realm usa o mesmo arquivo do cliente (o
/// índice da oferta é o `goods_index` do `MALL_SHOPPING`, spec 03 §3.8): o item é `ambos`.
fn caminho_da_loja(arquivo: &str) -> Result<String> {
    if !matches!(arquivo, "gshop.data" | "gshop1.data" | "gshop2.data") {
        bail!("loja inválida: {arquivo}");
    }
    Ok(format!("data\\{arquivo}"))
}

fn texto_do_bloco(b: &[u8]) -> String {
    gs::ler_texto(b, gs::TipoDoCampo::Utf16(64))
}

impl Estudio {
    /// A loja: categorias (8, com subcategorias) e uma linha por oferta `[indice, main_type, sub_type, id, num, nome,
    /// buy_0_price, nome do item no elements]`.
    pub async fn loja(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: Option<i64>) -> Result<Value> {
        let caminho = caminho_da_loja(arquivo)?;
        let (_, _, _, bytes, item) = self.item_do_cliente(gov, versao, &caminho, rascunho).await?;
        let g = GshopEditavel::ler(&bytes)?;
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let ids = mapa_de_ids(&e);
        let nome_do_item = |id: u32| match ids.get(&("ESSENCE".to_string(), id as i32)) {
            Some(&(ti, ri)) if ti != CONVERSAS => Some(nome_do_registro(&e.tabelas[ti], ri)),
            _ => None,
        };
        let campos = g.campos();
        let pega = |o: &[u8], nome: &str| -> Value {
            let (_, t, d) = campos.iter().find(|c| c.0 == nome).cloned().unwrap();
            match t {
                gs::TipoDoCampo::I32 => json!(i32::from_le_bytes(o[d..d + 4].try_into().unwrap())),
                gs::TipoDoCampo::U32 => json!(u32::from_le_bytes(o[d..d + 4].try_into().unwrap())),
                gs::TipoDoCampo::Utf16(n) => json!(gs::ler_texto(&o[d..d + 2 * n], t)),
                gs::TipoDoCampo::Gbk(n) => json!(gs::ler_texto(&o[d..d + n], t)),
            }
        };
        let ofertas: Vec<Value> = g
            .ofertas
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let id = u32::from_le_bytes(o[140..144].try_into().unwrap());
                json!([i, pega(o, "main_type"), pega(o, "sub_type"), id, pega(o, "num"), pega(o, "szName"), pega(o, "buy_0_price"), nome_do_item(id)])
            })
            .collect();
        let categorias: Vec<Value> = g
            .categorias
            .iter()
            .map(|c| json!({ "nome": texto_do_bloco(&c.nome), "subcategorias": c.subcategorias.iter().map(|s| texto_do_bloco(s)).collect::<Vec<_>>() }))
            .collect();
        Ok(json!({
            "arquivo": arquivo, "formato": format!("{:?}", g.formato), "carimbo": g.carimbo, "no_rascunho": item.is_some(),
            "colunas": ["indice", "main_type", "sub_type", "id", "num", "szName", "buy_0_price", "item"],
            "ofertas": ofertas, "categorias": categorias,
            "campos": campos.iter().map(|(n, t, _)| json!([n, t])).collect::<Vec<_>>(),
        }))
    }

    /// Uma oferta campo a campo, com o item que ela vende (link para a ficha).
    pub async fn oferta(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: Option<i64>, indice: usize) -> Result<Value> {
        let caminho = caminho_da_loja(arquivo)?;
        let (_, _, _, bytes, _) = self.item_do_cliente(gov, versao, &caminho, rascunho).await?;
        let g = GshopEditavel::ler(&bytes)?;
        let o = g.ofertas.get(indice).with_context(|| format!("oferta {indice} não existe"))?;
        let campos = gs::ler_oferta(&g, o);
        let id = campos["id"].as_u64().unwrap_or(0) as i32;
        let (e, _) = self.atual(gov, versao, rascunho).await?;
        let item = match mapa_de_ids(&e).get(&("ESSENCE".to_string(), id)) {
            Some(&(ti, ri)) if ti != CONVERSAS => json!({ "nome": nome_do_registro(&e.tabelas[ti], ri), "tabela": e.tabelas[ti].nome, "indice": ri }),
            _ => json!(null),
        };
        Ok(json!({ "indice": indice, "campos": campos, "item": item }))
    }

    /// Muda a loja no rascunho como item `ambos` (cliente e realm, o mesmo arquivo); o carimbo não muda.
    async fn alterar_loja<F>(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: i64, mudar: F) -> Result<Value>
    where
        F: FnOnce(&mut GshopEditavel) -> Result<(Value, Value)>,
    {
        let caminho = caminho_da_loja(arquivo)?;
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        self.gravar_item_com_destino(gov, versao, &caminho, rascunho, "ambos", move |base| {
            let mut g = GshopEditavel::ler(base)?;
            let (resposta, mudanca) = mudar(&mut g)?;
            Ok((g.gravar(), resposta, mudanca))
        })
        .await
    }

    pub async fn editar_oferta(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: i64, indice: usize, campos: Map<String, Value>) -> Result<Value> {
        self.alterar_loja(gov, versao, arquivo, rascunho, move |g| {
            let mut o = g.ofertas.get(indice).cloned().with_context(|| format!("oferta {indice} não existe"))?;
            let antes = gs::ler_oferta(g, &o);
            for (k, v) in &campos {
                gs::definir_campo(g, &mut o, k, v)?;
            }
            let depois = gs::ler_oferta(g, &o);
            let mudou: Map<String, Value> = depois.iter().filter(|(k, x)| antes.get(*k) != Some(*x)).map(|(k, x)| (k.clone(), json!([antes[k], x]))).collect();
            g.ofertas[indice] = o;
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editada", "tipo": "oferta", "indice": indice, "campos": mudou }) };
            Ok((json!({ "indice": indice }), mudanca))
        })
        .await
    }

    /// Oferta nova no fim da lista (não muda o índice de ninguém), cópia do `base`.
    pub async fn clonar_oferta(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: i64, base: usize) -> Result<Value> {
        self.alterar_loja(gov, versao, arquivo, rascunho, move |g| {
            let o = g.ofertas.get(base).cloned().with_context(|| format!("oferta {base} não existe"))?;
            g.ofertas.push(o);
            let indice = g.ofertas.len() - 1;
            Ok((json!({ "indice": indice }), json!({ "acao": "nova", "tipo": "oferta", "indice": indice, "base": base })))
        })
        .await
    }

    /// Tira a oferta (as seguintes sobem um índice; cliente e realm recebem o mesmo arquivo, então seguem casando).
    pub async fn remover_oferta(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: i64, indice: usize) -> Result<Value> {
        self.alterar_loja(gov, versao, arquivo, rascunho, move |g| {
            if indice >= g.ofertas.len() {
                bail!("oferta {indice} não existe");
            }
            let o = g.ofertas.remove(indice);
            let nome = gs::ler_texto(&o[g.campos().iter().find(|c| c.0 == "szName").map(|c| c.2).unwrap()..][..64], gs::TipoDoCampo::Utf16(32));
            Ok((json!({ "removida": indice }), json!({ "acao": "removida", "tipo": "oferta", "indice": indice, "nome": nome })))
        })
        .await
    }
}

// ------------------------------------------------------------------ missões (tasks.data, S7, B255 — só leitura)

use pw_data_loader::tasks::TasksData;

static MISSOES: std::sync::OnceLock<Mutex<HashMap<(String, Sha256Bytes), Arc<TasksData>>>> = std::sync::OnceLock::new();

impl Estudio {
    /// O `tasks.data` (o do cliente publicado é o mesmo do realm, que o GS executa — `manager.rs:461`; B256: com o
    /// rascunho, o dele), lido uma vez por versão e SHA.
    async fn missoes_do_realm(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<Arc<TasksData>> {
        Ok(self.missoes_e_bytes(gov, versao, rascunho).await?.1)
    }

    /// O `tasks.data` (do rascunho, se houver) e a leitura dele (em cache por sha).
    async fn missoes_e_bytes(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<(Arc<Vec<u8>>, Arc<TasksData>)> {
        let bytes = Arc::new(self.item_do_cliente(gov, versao, "data\\tasks.data", rascunho).await?.3);
        let sha = sha256(&bytes);
        let cache = MISSOES.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(t) = cache.lock().unwrap().get(&(versao.to_string(), sha)) {
            return Ok((bytes, t.clone()));
        }
        let b = bytes.clone();
        let t = Arc::new(tokio::task::spawn_blocking(move || TasksData::load_from_bytes(&b)).await?.map_err(|e| anyhow::anyhow!("tasks.data: {e}"))?);
        let mut c = cache.lock().unwrap();
        c.retain(|(v, _), _| v != versao); // uma por versão: o 1.5.5 tem 104 MB
        c.insert((versao.to_string(), sha), t.clone());
        Ok((bytes, t))
    }

    /// Lista de missões: busca por id ou nome; `topo` = só as de topo (as outras se chegam pela árvore). Linha
    /// `[id, nome, tipo, nível mín., nível máx., mãe, filhas, npc que entrega, npc que premia]`.
    #[allow(clippy::too_many_arguments)]
    pub async fn missoes(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>, busca: &str, topo: bool, pagina: usize, por_pagina: usize) -> Result<Value> {
        let t = self.missoes_do_realm(gov, versao, rascunho).await?;
        let busca = busca.trim().to_lowercase();
        let id_buscado: Option<u32> = busca.parse().ok();
        let mut ids: Vec<u32> = if topo && busca.is_empty() { t.de_topo.clone() } else { t.tasks.keys().copied().collect() };
        if !busca.is_empty() {
            ids.retain(|id| Some(*id) == id_buscado || t.tasks[id].name.to_lowercase().contains(&busca));
        }
        ids.sort_unstable();
        let total = ids.len();
        let linhas: Vec<Value> = ids
            .iter()
            .skip(pagina * por_pagina)
            .take(por_pagina)
            .map(|id| {
                let m = &t.tasks[id];
                json!([m.id, m.name, m.tipo, m.min_level, m.max_level, m.parent, m.sub_tasks.len(), m.npc_que_entrega, m.npc_que_premia])
            })
            .collect();
        Ok(json!({
            "versao_do_arquivo": t.version, "missoes": t.tasks.len(), "de_topo": t.de_topo.len(), "total": total, "pagina": pagina,
            "colunas": ["id", "nome", "tipo", "nivel_min", "nivel_max", "mae", "filhas", "npc_entrega", "npc_premio"], "linhas": linhas,
        }))
    }

    /// Uma missão inteira (os campos do `TaskTemplate` do leitor do GS), com os nomes dos NPCs, itens e monstros que ela
    /// cita (do `elements` publicado) e das missões mãe, filhas, pré-requisitos e exclusivas.
    pub async fn missao(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>, id: u32) -> Result<Value> {
        let (bytes, t) = self.missoes_e_bytes(gov, versao, rascunho).await?;
        let m = t.tasks.get(&id).with_context(|| format!("missão {id} não existe"))?;
        let ficha = serde_json::to_value(m)?;
        // B262: os quatro textos longos (descrição, aceitar, recusar, tributo), editáveis com tamanho variável.
        let textos = pw_data_loader::tasks_editavel::textos_da_missao(&bytes, m);
        // B266: os cinco diálogos (título, janelas e opções).
        let dialogos: Vec<Value> = pw_data_loader::tasks_editavel::DIALOGOS.iter()
            .zip(pw_data_loader::tasks_editavel::dialogos_da_missao(&bytes, m))
            .map(|(nome, d)| { let mut v = serde_json::to_value(d).unwrap_or_default(); v["nome"] = json!(nome); v })
            .collect();
        // Ids do espaço ESSENCE citados em qualquer lugar da ficha (NPCs, itens, monstros): todos os números que
        // estejam em campos com "id", "npc", "item" ou "monstro" no nome.
        let mut citados = std::collections::BTreeSet::new();
        fn varrer(v: &Value, chave: &str, s: &mut std::collections::BTreeSet<i64>) {
            match v {
                Value::Object(o) => o.iter().for_each(|(k, x)| varrer(x, k, s)),
                Value::Array(a) => a.iter().for_each(|x| varrer(x, chave, s)),
                Value::Number(n) => {
                    let k = chave.to_lowercase();
                    if (k == "id" || k.contains("npc") || k.contains("item") || k.contains("monstro") || k.contains("brinde")) && !k.contains("quantidade") {
                        if let Some(x) = n.as_i64().filter(|&x| x > 0) {
                            s.insert(x);
                        }
                    }
                }
                _ => {}
            }
        }
        varrer(&ficha, "", &mut citados);
        let (e, _) = self.atual(gov, versao, None).await?;
        let ids = mapa_de_ids(&e);
        let nomes: Map<String, Value> = citados
            .iter()
            .filter_map(|&x| match ids.get(&("ESSENCE".to_string(), x as i32)) {
                Some(&(ti, ri)) if ti != CONVERSAS => Some((x.to_string(), json!({ "nome": nome_do_registro(&e.tabelas[ti], ri), "tabela": e.tabelas[ti].nome, "indice": ri }))),
                _ => None,
            })
            .collect();
        let nome_da_missao = |i: u32| json!({ "id": i, "nome": t.tasks.get(&i).map(|x| x.name.clone()) });
        Ok(json!({
            "ficha": ficha,
            "nomes": nomes,
            "mae": m.parent.map(nome_da_missao),
            "filhas": m.sub_tasks.iter().map(|&i| nome_da_missao(i)).collect::<Vec<_>>(),
            "pre_missoes": m.pre_tasks.iter().map(|&i| nome_da_missao(i)).collect::<Vec<_>>(),
            "exclusivas": m.missoes_exclusivas.iter().map(|&i| nome_da_missao(i)).collect::<Vec<_>>(),
            "nova_missao": (m.rewards.nova_missao > 0).then(|| nome_da_missao(m.rewards.nova_missao)),
            // B256: os campos de tamanho fixo que se editam no lugar, com o tipo.
            "editaveis": pw_data_loader::tasks_editavel::campos(t.version).iter().map(|c| json!([c.0, c.2])).collect::<Vec<_>>(),
            "textos": textos,
            "dialogos": dialogos,
            // B270: as funções de opção (`SERVICE_TYPE`).
            "funcoes_de_opcao": pw_data_loader::tasks_editavel::FUNCOES_DE_OPCAO.iter().map(|(n, nome)| json!([n, nome])).collect::<Vec<_>>(),
            // B263: listas de capacidade fixa (pré-missões, exclusivas, classes) — `[nome, capacidade]`.
            "listas": pw_data_loader::tasks::listas_fixas(t.version).iter().map(|l| json!([l.nome, l.capacidade])).collect::<Vec<_>>(),
            // B264: vetores variáveis (itens exigidos, entregues, monstros) — `[nome, teto do cliente]`.
            "vetores": pw_data_loader::tasks::vetores(t.version).iter().map(|v| json!([v.nome, v.teto])).collect::<Vec<_>>(),
            // B265: tetos dos grupos de itens de prêmio.
            "grupos_do_premio": json!({ "grupos": pw_data_loader::tasks::MAX_AWARD_CANDIDATES, "itens": pw_data_loader::tasks::MAX_ITEM_AWARD }),
        }))
    }

    /// Edita campos de tamanho fixo da missão (`tasks_editavel`): item `ambos` (`data\\tasks.data`, o mesmo arquivo no
    /// cliente e no realm). Relê o resultado pelo leitor do GS antes de gravar.
    pub async fn editar_missao(&self, gov: &Governanca, versao: &str, rascunho: i64, id: u32, campos: Map<String, Value>) -> Result<Value> {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        self.gravar_item_com_destino(gov, versao, "data\\tasks.data", rascunho, "ambos", move |base| {
            let mut t = pw_data_loader::tasks_editavel::TasksEditavel::ler(base).map_err(|e| anyhow::anyhow!("tasks.data: {e}"))?;
            let com_textos = |t: &pw_data_loader::tasks_editavel::TasksEditavel| -> Option<Map<String, Value>> {
                let mut c = t.ler_campos(id)?;
                c.extend(t.textos(id)?);
                let m = t.dados.tasks.get(&id)?;
                c.insert("pre_tasks".into(), json!(m.pre_tasks));
                c.insert("missoes_exclusivas".into(), json!(m.missoes_exclusivas));
                c.insert("req_classes".into(), json!(m.req_classes));
                c.insert("itens_exigidos".into(), json!(m.itens_exigidos));
                c.insert("itens_entregues".into(), json!(m.itens_entregues));
                c.insert("monster_kills".into(), json!(m.monster_kills));
                c.insert("premio_grupos".into(), json!(m.rewards.grupos_de_itens));
                c.insert("falha_grupos".into(), json!(m.premio_de_falha.grupos_de_itens));
                c.insert("dialogos".into(), json!(t.dialogos(id)?));
                Some(c)
            };
            let antes = com_textos(&t).with_context(|| format!("missão {id} não existe"))?;
            // Textos longos (B262) mudam o tamanho e a tabela de deslocamentos: vão por último, depois dos fixos.
            let (variaveis, fixos): (Vec<_>, Vec<_>) = campos.iter().partition(|(k, _)| {
                pw_data_loader::tasks_editavel::TEXTOS.contains(&k.as_str())
                    || pw_data_loader::tasks::vetores(t.dados.version).iter().any(|v| v.nome == k.as_str())
                    || k.as_str() == "premio_grupos"
                    || k.as_str() == "falha_grupos"
                    || k.as_str() == "dialogos"
            });
            for (k, v) in fixos {
                if pw_data_loader::tasks::listas_fixas(t.dados.version).iter().any(|l| l.nome == k) {
                    let valores: Vec<u32> = v.as_array().with_context(|| format!("{k}: lista"))?.iter()
                        .map(|x| x.as_u64().and_then(|x| u32::try_from(x).ok()).with_context(|| format!("{k}: inteiros sem sinal")))
                        .collect::<Result<_>>()?;
                    t.definir_lista(id, k, &valores).map_err(anyhow::Error::msg)?;
                } else {
                    t.definir_campo(id, k, v).map_err(anyhow::Error::msg)?;
                }
            }
            for (k, v) in variaveis {
                if pw_data_loader::tasks_editavel::TEXTOS.contains(&k.as_str()) {
                    t.definir_texto(id, k, v.as_str().with_context(|| format!("{k}: texto"))?).map_err(anyhow::Error::msg)?;
                } else if k == "dialogos" {
                    // B266: `[{dialogo, janela?, opcao?, texto}]` — título, texto de janela ou de opção. B270: `acao` =
                    // `nova_opcao` (`texto`, `destino: {janela: texto} | {funcao: n}`, `param?`), `remover_opcao`
                    // (`janela`, `opcao`) ou `criar_raiz` (`texto`). Aplicadas na ordem.
                    use pw_data_loader::tasks_editavel::DestinoDaOpcao;
                    for x in v.as_array().context("dialogos: lista")? {
                        let d = x["dialogo"].as_u64().context("dialogos[].dialogo")? as usize;
                        let j = x["janela"].as_u64().map(|x| x as usize);
                        let o = x["opcao"].as_u64().map(|x| x as usize);
                        let texto = || x["texto"].as_str().context("dialogos[].texto");
                        let r = match x["acao"].as_str() {
                            None => t.definir_texto_do_dialogo(id, d, j, o, texto()?),
                            Some("nova_opcao") => {
                                let destino = if let Some(tj) = x["destino"]["janela"].as_str() {
                                    DestinoDaOpcao::NovaJanela(tj.to_string())
                                } else {
                                    DestinoDaOpcao::Funcao(x["destino"]["funcao"].as_u64().context("destino: {janela} ou {funcao}")? as u32)
                                };
                                let param = x["param"].as_u64().map(|p| p as u32);
                                t.acrescentar_opcao(id, d, j.context("janela")?, texto()?, destino, param)
                            }
                            Some("remover_opcao") => t.remover_opcao(id, d, j.context("janela")?, o.context("opcao")?),
                            Some("criar_raiz") => t.criar_janela_raiz(id, d, texto()?),
                            Some(a) => bail!("ação de diálogo desconhecida: {a}"),
                        };
                        r.map_err(anyhow::Error::msg)?;
                    }
                } else if k == "premio_grupos" || k == "falha_grupos" {
                    // B265: grupos de itens de prêmio (sucesso ou falha).
                    t.definir_grupos_do_premio(id, k == "falha_grupos", v).map_err(anyhow::Error::msg)?;
                } else {
                    t.definir_vetor(id, k, v).map_err(anyhow::Error::msg)?;
                }
            }
            let novo = t.gravar();
            let relido = pw_data_loader::tasks_editavel::TasksEditavel::ler(&novo).map_err(|e| anyhow::anyhow!("o tasks.data editado não relê: {e}"))?;
            let depois = com_textos(&relido).context("missão sumiu")?;
            let mudou: Map<String, Value> = depois.iter().filter(|(k, x)| antes.get(*k) != Some(*x)).map(|(k, x)| (k.clone(), json!([antes[k], x]))).collect();
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editada", "tipo": "missao", "id": id, "campos": mudou }) };
            Ok((novo, json!({ "id": id }), mudanca))
        })
        .await
    }
}

impl Estudio {
    /// Clona uma missão de topo com a árvore de submissões (B267/B269, `tasks_editavel::clonar_missao`): ids
    /// consecutivos a partir de `novo` (ou do maior id + 1). Item `ambos`. A missão nova só é oferecida em jogo depois de entrar no serviço de um NPC no `elements`.
    pub async fn clonar_missao(&self, gov: &Governanca, versao: &str, rascunho: i64, id: u32, novo: Option<u32>) -> Result<Value> {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        self.gravar_item_com_destino(gov, versao, "data\\tasks.data", rascunho, "ambos", move |base| {
            let mut t = pw_data_loader::tasks_editavel::TasksEditavel::ler(base).map_err(|e| anyhow::anyhow!("tasks.data: {e}"))?;
            let novo = match novo {
                Some(n) => n,
                None => t.dados.tasks.keys().max().copied().unwrap_or(0) + 1,
            };
            t.clonar_missao(id, novo).map_err(anyhow::Error::msg)?;
            let mudanca = json!({ "acao": "nova", "tipo": "missao", "id": novo, "clone_de": id });
            Ok((t.gravar(), json!({ "id": novo, "clone_de": id }), mudanca))
        })
        .await
    }

    /// Remove uma missão de topo e a árvore dela (B268, `tasks_editavel::remover_missao`). Recusa (400, com a lista) se
    /// o `elements` (com o rascunho) ou outra missão cita a missão ou uma submissão dela. Item `ambos`.
    pub async fn remover_missao(&self, gov: &Governanca, versao: &str, rascunho: i64, id: u32) -> Result<Value> {
        let (e, _) = self.atual(gov, versao, Some(rascunho)).await?;
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        self.gravar_item_com_destino(gov, versao, "data\\tasks.data", rascunho, "ambos", move |base| {
            let mut t = pw_data_loader::tasks_editavel::TasksEditavel::ler(base).map_err(|e| anyhow::anyhow!("tasks.data: {e}"))?;
            let m = t.dados.tasks.get(&id).with_context(|| format!("missão {id} não existe"))?;
            let mut arvore = vec![m.id];
            let mut k = 0;
            while k < arvore.len() {
                arvore.extend(t.dados.tasks.get(&arvore[k]).map(|x| x.sub_tasks.clone()).unwrap_or_default());
                k += 1;
            }
            let no_elements: Vec<Value> = arvore.iter().flat_map(|&x| citantes_de_missao(&e, x, 20)).take(20).collect();
            if !no_elements.is_empty() {
                let lista: Vec<String> = no_elements.iter().map(|c| format!("{}[{}].{}", c["tabela"].as_str().unwrap_or(""), c["id"], c["campo"].as_str().unwrap_or(""))).collect();
                bail!("a missão (ou uma submissão) é citada no elements: {} — tire essas citações antes", lista.join(", "));
            }
            let n = arvore.len();
            t.remover_missao(id).map_err(anyhow::Error::msg)?;
            let mudanca = json!({ "acao": "removida", "tipo": "missao", "id": id, "missoes_removidas": n });
            Ok((t.gravar(), json!({ "id": id, "missoes_removidas": n }), mudanca))
        })
        .await
    }

    /// Categorias da loja (B257): `acao` = `renomear` (`categoria`, `sub?`, `nome`), `nova` (subcategoria no fim de
    /// `categoria`, com `nome`) ou `remover` (a última subcategoria de `categoria`, se nenhuma oferta a usa).
    pub async fn categoria_da_loja(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: i64, pedido: Value) -> Result<Value> {
        self.alterar_loja(gov, versao, arquivo, rascunho, move |g| {
            let c = pedido["categoria"].as_u64().context("categoria")? as usize;
            let texto = |k: &str| pedido[k].as_str().map(str::to_string).with_context(|| format!("{k}: texto"));
            let (resposta, mudanca) = match pedido["acao"].as_str() {
                Some("renomear") => {
                    let sub = pedido["sub"].as_u64().map(|x| x as usize);
                    let nome = texto("nome")?;
                    g.renomear_categoria(c, sub, &nome)?;
                    (json!({ "categoria": c, "sub": sub }), json!({ "acao": "renomeada", "tipo": "categoria", "categoria": c, "sub": sub, "nome": nome }))
                }
                Some("nova") => {
                    let nome = texto("nome")?;
                    let k = g.nova_subcategoria(c, &nome)?;
                    (json!({ "categoria": c, "sub": k }), json!({ "acao": "nova", "tipo": "subcategoria", "categoria": c, "sub": k, "nome": nome }))
                }
                Some("remover") => {
                    g.remover_ultima_subcategoria(c)?;
                    (json!({ "categoria": c }), json!({ "acao": "removida", "tipo": "subcategoria", "categoria": c }))
                }
                _ => bail!("acao: renomear, nova ou remover"),
            };
            Ok((resposta, mudanca))
        })
        .await
    }
}

// ------------------------------------------------------------------ comportamento (aipolicy.data, S7, B258 — só leitura)

use pw_data_loader::aipolicy::AiPolicyData;

static POLITICAS: std::sync::OnceLock<Mutex<HashMap<(String, Sha256Bytes), Arc<AiPolicyData>>>> = std::sync::OnceLock::new();

impl Estudio {
    /// O `aipolicy.data` do realm (só do servidor: o cliente não o tem), pelo leitor do GS, em cache por versão/SHA;
    /// com o rascunho, o dele (B259).
    async fn politicas_do_realm(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>) -> Result<Arc<AiPolicyData>> {
        let (_, do_realm, do_rascunho) = self.item_do_realm(gov, versao, "servidor\\aipolicy.data".into(), rascunho).await?;
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let sha = sha256(&bytes);
        let cache = POLITICAS.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(p) = cache.lock().unwrap().get(&(versao.to_string(), sha)) {
            return Ok(p.clone());
        }
        let p = Arc::new(tokio::task::spawn_blocking(move || AiPolicyData::load_from_bytes(&bytes)).await?.map_err(|e| anyhow::anyhow!("aipolicy.data: {e}"))?);
        let mut c = cache.lock().unwrap();
        c.retain(|(v, _), _| v != versao);
        c.insert((versao.to_string(), sha), p.clone());
        Ok(p)
    }

    /// Monstros por política: `MONSTER_ESSENCE.common_strategy` (`monstros.rs:169`) do `elements` publicado.
    async fn monstros_por_politica(&self, gov: &Governanca, versao: &str) -> Result<HashMap<u32, Vec<Value>>> {
        let (e, _) = self.atual(gov, versao, None).await?;
        let mut por: HashMap<u32, Vec<Value>> = HashMap::new();
        if let Ok(t) = e.tabela("MONSTER_ESSENCE") {
            for i in 0..t.registros.len() {
                let Ok(r) = t.ler(i) else { continue };
                if let Some(p) = r.get("common_strategy").and_then(|v| v.as_i32()).filter(|&p| p > 0) {
                    por.entry(p as u32).or_default().push(json!({ "id": t.id(i), "nome": nome_do_registro(t, i), "indice": i }));
                }
            }
        }
        Ok(por)
    }

    /// Lista de políticas: `[id, gatilhos, monstros que usam]`, com busca por id (da política ou de um monstro que a usa).
    pub async fn comportamentos(&self, gov: &Governanca, versao: &str, busca: &str) -> Result<Value> {
        let p = self.politicas_do_realm(gov, versao, None).await?;
        let usos = self.monstros_por_politica(gov, versao).await?;
        let busca = busca.trim().to_lowercase();
        let mut ids: Vec<u32> = p.policies.keys().copied().collect();
        if !busca.is_empty() {
            let n: Option<i64> = busca.parse().ok();
            ids.retain(|id| {
                Some(*id as i64) == n
                    || usos.get(id).is_some_and(|ms| ms.iter().any(|m| m["id"].as_i64() == n || m["nome"].as_str().is_some_and(|s| s.to_lowercase().contains(&busca))))
            });
        }
        ids.sort_unstable();
        let linhas: Vec<Value> = ids.iter().take(500).map(|id| json!([id, p.policies[id].triggers.len(), usos.get(id).map_or(0, Vec::len)])).collect();
        Ok(json!({
            "versao_do_arquivo": p.versao, "politicas": p.policies.len(), "total": ids.len(), "linhas": linhas,
            "sem_politica": usos.keys().filter(|id| !p.policies.contains_key(id)).count(),
        }))
    }

    /// Uma política: os gatilhos (condição em árvore e operações, como o leitor do GS os vê) e os monstros que a usam.
    pub async fn comportamento(&self, gov: &Governanca, versao: &str, rascunho: Option<i64>, id: u32) -> Result<Value> {
        let p = self.politicas_do_realm(gov, versao, rascunho).await?;
        let pol = p.policies.get(&id).with_context(|| format!("política {id} não existe"))?;
        let usos = self.monstros_por_politica(gov, versao).await?;
        // B259: por gatilho, os campos editáveis de cada operação e de cada condição (caminho na árvore: "", "e", "d"…).
        use pw_data_loader::aipolicy_editavel as ae;
        fn condicoes(no: Option<&pw_data_loader::aipolicy::NoDeCondicao>, caminho: String, saida: &mut Map<String, Value>) {
            let Some(no) = no else { return };
            let c = ae::campos_da_condicao(&no.parametro);
            if !c.is_empty() {
                saida.insert(caminho.clone(), json!(c));
            }
            condicoes(no.esquerda.as_deref(), format!("{caminho}e"), saida);
            condicoes(no.direita.as_deref(), format!("{caminho}d"), saida);
        }
        let editaveis: Vec<Value> = pol
            .triggers
            .iter()
            .map(|t| {
                let mut c = Map::new();
                condicoes(t.condicao.as_ref(), String::new(), &mut c);
                let ops: Map<String, Value> = t
                    .operacoes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, o)| {
                        let campos = ae::campos_da_operacao(&o.parametro);
                        (!campos.is_empty()).then(|| (i.to_string(), json!(campos)))
                    })
                    .collect();
                json!({ "condicoes": c, "operacoes": ops })
            })
            .collect();
        // B271: os tipos que este arquivo usa — folhas de condição por papel (valor = debaixo de aritmética/comparação) e
        // operações —, para trocar tipo e acrescentar operação.
        let mut folhas: std::collections::BTreeMap<i32, bool> = Default::default();
        let mut ops: std::collections::BTreeSet<i32> = Default::default();
        fn andar(n: &pw_data_loader::aipolicy::NoDeCondicao, pai: Option<i32>, f: &mut std::collections::BTreeMap<i32, bool>) {
            if n.esquerda.is_none() && n.direita.is_none() && !matches!(n.tipo_cru, 5..=7 | 9..=15) {
                f.entry(n.tipo_cru).or_insert(pai.is_some_and(|x| (9..=15).contains(&x)));
            }
            for x in [&n.esquerda, &n.direita].into_iter().flatten() {
                andar(x, Some(n.tipo_cru), f);
            }
        }
        for q in p.policies.values() {
            for t in &q.triggers {
                if let Some(c) = &t.condicao {
                    andar(c, None, &mut folhas);
                }
                ops.extend(t.operacoes.iter().map(|o| o.tipo_cru));
            }
        }
        let nome_c = |t: i32| pw_data_loader::aipolicy::TipoDeCondicao::de_i32(t).map(|x| format!("{x:?}")).unwrap_or_else(|| format!("tipo {t}"));
        let nome_o = |t: i32| pw_data_loader::aipolicy::TipoDeOperacao::de_i32(t).map(|x| format!("{x:?}")).unwrap_or_else(|| format!("tipo {t}"));
        let tipos = json!({
            "condicoes": folhas.iter().map(|(t, v)| json!([t, nome_c(*t), v])).collect::<Vec<_>>(),
            "operacoes": ops.iter().map(|t| json!([t, nome_o(*t)])).collect::<Vec<_>>(),
        });
        Ok(json!({ "politica": pol, "monstros": usos.get(&id).cloned().unwrap_or_default(), "editaveis": editaveis, "tipos": tipos }))
    }

    /// Edita um gatilho da política no rascunho (item `servidor\aipolicy.data`; o cliente não tem o arquivo):
    /// `{gatilho, flags?: {ativo, rodando, so_em_combate}, operacao?: {indice, valores}, condicao?: {caminho, valores}}`.
    pub async fn editar_comportamento(&self, gov: &Governanca, versao: &str, rascunho: i64, id: u32, pedido: Value) -> Result<Value> {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let (nome, do_realm, do_rascunho) = self.item_do_realm(gov, versao, "servidor\\aipolicy.data".into(), Some(rascunho)).await?;
        self.gravar_item_do_realm(gov, versao, rascunho, nome, do_realm, do_rascunho, move |base| {
            let mut a = pw_data_loader::aipolicy_editavel::AiPolicyEditavel::ler(base).map_err(anyhow::Error::msg)?;
            let g = pedido["gatilho"].as_u64().context("gatilho")? as usize;
            // B271: trocas de tipo e operações novas/removidas vêm primeiro (as edições de valor abaixo valem para o
            // tipo novo).
            let id_gatilho = |x: &Value| x["id_gatilho"].as_u64().map(|v| v as u32);
            if let Some(c) = pedido["tipo_da_condicao"].as_object() {
                let caminho = c.get("caminho").and_then(Value::as_str).unwrap_or("");
                let tipo = c.get("tipo").and_then(Value::as_i64).context("tipo_da_condicao.tipo")? as i32;
                a.trocar_tipo_da_condicao(id, g, caminho, tipo).map_err(anyhow::Error::msg)?;
            }
            if let Some(o) = pedido.get("tipo_da_operacao").filter(|x| x.is_object()) {
                let i = o["indice"].as_u64().context("tipo_da_operacao.indice")? as usize;
                let tipo = o["tipo"].as_i64().context("tipo_da_operacao.tipo")? as i32;
                a.trocar_tipo_da_operacao(id, g, i, tipo, id_gatilho(o)).map_err(anyhow::Error::msg)?;
            }
            if let Some(o) = pedido.get("operacao_nova").filter(|x| x.is_object()) {
                let tipo = o["tipo"].as_i64().context("operacao_nova.tipo")? as i32;
                a.acrescentar_operacao(id, g, tipo, id_gatilho(o)).map_err(anyhow::Error::msg)?;
            }
            if let Some(i) = pedido["remover_operacao"].as_u64() {
                a.remover_operacao(id, g, i as usize).map_err(anyhow::Error::msg)?;
            }
            if let Some(f) = pedido["flags"].as_object() {
                for (k, v) in f {
                    a.definir_flag(id, g, k, v.as_bool().with_context(|| format!("{k}: verdadeiro ou falso"))?).map_err(anyhow::Error::msg)?;
                }
            }
            if let Some(o) = pedido["operacao"].as_object() {
                let i = o.get("indice").and_then(Value::as_u64).context("operacao.indice")? as usize;
                let v = o.get("valores").and_then(Value::as_object).context("operacao.valores")?;
                a.definir_operacao(id, g, i, v).map_err(anyhow::Error::msg)?;
            }
            if let Some(c) = pedido["condicao"].as_object() {
                let caminho = c.get("caminho").and_then(Value::as_str).unwrap_or("");
                let v = c.get("valores").and_then(Value::as_object).context("condicao.valores")?;
                a.definir_condicao(id, g, caminho, v).map_err(anyhow::Error::msg)?;
            }
            // Edição estrutural (B261): fala, depois duplicar ou remover (que mudam os índices; vêm por último).
            if let Some(f) = pedido["fala"].as_object() {
                let i = f.get("indice").and_then(Value::as_u64).context("fala.indice")? as usize;
                let t = f.get("texto").and_then(Value::as_str).context("fala.texto")?;
                a.definir_fala(id, g, i, t).map_err(anyhow::Error::msg)?;
            }
            let mut resposta = json!({ "id": id });
            if !pedido["duplicar"].is_null() {
                let novo_id = pedido["duplicar"]["novo_id"].as_u64().map(|x| u32::try_from(x)).transpose().context("duplicar.novo_id")?;
                resposta["novo_gatilho"] = json!(a.duplicar_gatilho(id, g, novo_id).map_err(anyhow::Error::msg)?);
            } else if pedido["remover"].as_bool() == Some(true) {
                a.remover_gatilho(id, g).map_err(anyhow::Error::msg)?;
            }
            let novo = a.gravar();
            pw_data_loader::aipolicy::AiPolicyData::load_from_bytes(&novo).map_err(|e| anyhow::anyhow!("o aipolicy.data editado não relê: {e}"))?;
            let mudanca = json!({ "acao": "editado", "tipo": "politica", "id": id, "pedido": pedido });
            Ok((novo, resposta, mudanca))
        })
        .await
    }
}

// ------------------------------------------------------------------ textos do cliente (S7, B260)

fn caminho_do_texto(arquivo: &str) -> Result<String> {
    if !crate::textos::ARQUIVOS.contains(&arquivo) {
        bail!("tabela de texto inválida: {arquivo}");
    }
    Ok(format!("configs\\{arquivo}"))
}

impl Estudio {
    /// Uma tabela de texto do cliente (`crate::textos`): `[chave, texto]` paginado, busca por chave ou texto.
    #[allow(clippy::too_many_arguments)]
    pub async fn textos(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: Option<i64>, busca: &str, pagina: usize, por_pagina: usize) -> Result<Value> {
        let caminho = caminho_do_texto(arquivo)?;
        let (_, _, _, bytes, item) = self.item_do_cliente(gov, versao, &caminho, rascunho).await?;
        let t = crate::textos::TabelaDeTextos::ler(&bytes)?;
        let busca = busca.trim().to_lowercase();
        let n: Option<i64> = busca.parse().ok();
        let achadas: Vec<&crate::textos::Entrada> = t
            .entradas
            .iter()
            .filter(|e| busca.is_empty() || Some(e.chave) == n || e.texto.to_lowercase().contains(&busca))
            .collect();
        let linhas: Vec<Value> = achadas.iter().skip(pagina * por_pagina).take(por_pagina).map(|e| json!([e.chave, e.texto])).collect();
        Ok(json!({ "arquivo": arquivo, "indexada": t.indexada, "entradas": t.entradas.len(), "total": achadas.len(), "pagina": pagina,
            "no_rascunho": item.is_some(), "linhas": linhas }))
    }

    /// Troca o texto de uma entrada (`acao = definir`) ou acrescenta uma (`acrescentar`, só na indexada). Item `cliente`.
    pub async fn editar_texto(&self, gov: &Governanca, versao: &str, arquivo: &str, rascunho: i64, chave: i64, texto: String, novo: bool) -> Result<Value> {
        let caminho = caminho_do_texto(arquivo)?;
        self.alterar_item_do_cliente(gov, versao, &caminho, rascunho, move |base| {
            let mut t = crate::textos::TabelaDeTextos::ler(base)?;
            let antes = t.entrada(chave).map(|e| e.texto.clone());
            if novo {
                t.acrescentar(chave, &texto)?;
            } else {
                t.definir(chave, &texto)?;
            }
            let mudanca = if antes.as_deref() == Some(texto.as_str()) {
                Value::Null
            } else {
                json!({ "acao": if novo { "nova" } else { "editada" }, "tipo": "texto", "chave": chave, "antes": antes, "depois": texto })
            };
            Ok((t.gravar(), json!({ "chave": chave }), mudanca))
        })
        .await
    }
}

// ------------------------------------------------------------------ ações do modelo (S5, B280)

impl Estudio {
    /// Os bytes de um arquivo do cliente — do rascunho, se ele o tem, senão o publicado (para fichas que mostram a edição).
    pub async fn bytes_do_cliente(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Vec<u8>> {
        Ok(self.item_do_cliente(gov, versao, caminho, rascunho).await?.3)
    }

    /// Cria uma ação combinada no `.ecm` clonando outra (`pw_data_loader::ecm::clonar_acao`), opcionalmente apontando
    /// para outra ação-base do `.smd`. Item `cliente` (o `.ecm` só existe no cliente).
    pub async fn acao_nova_no_ecm(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, origem: String, nova: String, base: Option<String>) -> Result<Value> {
        if !caminho.to_lowercase().ends_with(".ecm") {
            bail!("só .ecm");
        }
        self.alterar_item_do_cliente(gov, versao, caminho, rascunho, move |b| {
            let novo = pw_data_loader::ecm::clonar_acao(b, &origem, &nova, base.as_deref()).map_err(anyhow::Error::msg)?;
            pw_data_loader::ecm::ler(&novo).context("o .ecm editado não relê")?;
            let mudanca = json!({ "acao": "nova", "tipo": "acao_combinada", "nome": nova, "clone_de": origem, "base": base });
            Ok((novo, json!({ "nome": nova }), mudanca))
        })
        .await
    }
}
