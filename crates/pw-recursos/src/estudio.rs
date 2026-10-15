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
    self as ng, NpcgenEditavel, CAMPOS_DA_AREA, CAMPOS_DA_AREA_DE_RECURSO, CAMPOS_DO_GERADOR, CAMPOS_DO_RECURSO,
};

/// Áreas de criatura (`AREA` + `NPCGEN`) ou de recurso (`RESAREA` + `RES`), `npcgendata.h:57-102`.
#[derive(Clone, Copy, PartialEq)]
pub enum TipoDeArea {
    Criatura,
    Recurso,
}

impl TipoDeArea {
    pub fn de(s: &str) -> Result<Self> {
        match s {
            "areas" => Ok(TipoDeArea::Criatura),
            "recursos" => Ok(TipoDeArea::Recurso),
            _ => bail!("tipo de área inválido: {s}"),
        }
    }
    fn campos(self) -> (&'static [ng::Campo], &'static [ng::Campo], &'static str, &'static str) {
        match self {
            TipoDeArea::Criatura => (CAMPOS_DA_AREA, CAMPOS_DO_GERADOR, "área", "gerador"),
            TipoDeArea::Recurso => (CAMPOS_DA_AREA_DE_RECURSO, CAMPOS_DO_RECURSO, "área de recurso", "recurso"),
        }
    }
    fn nome(self) -> &'static str {
        match self {
            TipoDeArea::Criatura => "área",
            TipoDeArea::Recurso => "área de recurso",
        }
    }
}

fn numero(m: &Map<String, Value>, k: &str) -> f64 {
    m.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

fn um_decimal(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
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
        Ok(json!({
            "item": nome,
            "versao_do_arquivo": v,
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
            },
        }))
    }

    /// Uma área com seus geradores (ou recursos), campo a campo.
    pub async fn area_do_npcgen(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>, tipo: TipoDeArea, indice: usize) -> Result<Value> {
        let (_, do_realm, do_rascunho) = self.npcgen_atual(gov, versao, caminho, rascunho).await?;
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let n = NpcgenEditavel::ler(&bytes).map_err(|e| anyhow::anyhow!("npcgen: {e}"))?;
        let (ca, cf, _, _) = tipo.campos();
        let (bruto, filhos) = match tipo {
            TipoDeArea::Criatura => n.areas.get(indice).map(|a| (&a.bruto, &a.geradores)),
            TipoDeArea::Recurso => n.areas_de_recurso.get(indice).map(|a| (&a.bruto, &a.recursos)),
        }
        .with_context(|| format!("{} {indice} não existe", tipo.nome()))?;
        Ok(json!({
            "indice": indice,
            "campos": ng::ler_campos(bruto, ca, n.versao),
            "filhos": filhos.iter().map(|f| ng::ler_campos(f, cf, n.versao)).collect::<Vec<_>>(),
        }))
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
            let (bruto, lista, tam) = match tipo {
                TipoDeArea::Criatura => n.areas.get_mut(indice).map(|a| (&mut a.bruto, &mut a.geradores, ng::tamanho_do_gerador(v))),
                TipoDeArea::Recurso => n.areas_de_recurso.get_mut(indice).map(|a| (&mut a.bruto, &mut a.recursos, ng::TAMANHO_DO_RECURSO)),
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
    /// Distritos do `precinct.sev` do realm (ponto de renascer: `city_region::GetCityPos`), com nome, id e marcas
    /// do `Maps\<caminho>\precinct.clt` do cliente quando os dois casam (mesma contagem e vértices por distrito).
    pub async fn distritos(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let (nome, do_realm, do_rascunho) = self.arquivo_do_mapa(gov, versao, caminho, "precinct.sev", rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let d = Distritos::ler(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
        let clt = self
            .arquivo_do_cliente(gov, versao, &format!("maps\\{caminho}\\precinct.clt"))
            .await
            .ok()
            .and_then(|b| crate::mapas::ler_precinct_clt(&crate::mapas::texto_do_arquivo(&b)).ok())
            .filter(|c| c.len() == d.lista.len() && c.iter().zip(&d.lista).all(|(a, b)| a.vertices == b.vertices.len()));
        let lista: Vec<Value> = d
            .lista
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let c = clt.as_ref().map(|c| &c[i]);
                json!({
                    "indice": i,
                    "nome": c.map(|c| c.nome.clone()),
                    "id": c.map(|c| c.id),
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
        Ok(json!({ "item": nome, "versao_do_arquivo": d.versao, "no_rascunho": no_rascunho, "nomes_do_cliente": clt.is_some(), "distritos": lista }))
    }

    /// Edita o ponto de renascer, o mapa dele, a prioridade e a proteção de PK de um distrito. Os vértices e o
    /// carimbo (`dwTimeStamp`, conferido pelo cliente no `INST_DATA_CHECKOUT`) não mudam.
    pub async fn editar_distrito(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, indice: usize, campos: Map<String, Value>) -> Result<Value> {
        self.alterar_arquivo_do_mapa(gov, versao, caminho, "precinct.sev", rascunho, move |base| {
            let mut d = Distritos::ler(base).map_err(|e| anyhow::anyhow!("{e}"))?;
            let v = d.versao;
            let x = d.lista.get_mut(indice).with_context(|| format!("distrito {indice} não existe"))?;
            let mut mudou = Map::new();
            for (k, valor) in &campos {
                match k.as_str() {
                    "ponto" => {
                        let novo = vetor_do_json(valor, "ponto")?;
                        if novo != x.ponto_de_cidade {
                            mudou.insert(k.clone(), json!([vetor_json(&x.ponto_de_cidade), vetor_json(&novo)]));
                            x.ponto_de_cidade = novo;
                        }
                    }
                    "mapa_do_ponto" | "prioridade" => {
                        let n = valor.as_i64().and_then(|n| i32::try_from(n).ok()).with_context(|| format!("{k}: inteiro"))?;
                        let alvo = if k == "mapa_do_ponto" { &mut x.mapa_do_ponto } else { &mut x.prioridade };
                        if k == "mapa_do_ponto" && n <= 0 {
                            bail!("mapa_do_ponto: id de mapa inválido");
                        }
                        if *alvo != n {
                            mudou.insert(k.clone(), json!([*alvo, n]));
                            *alvo = n;
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
                    }
                    _ => bail!("campo do distrito não editável: {k}"),
                }
            }
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editado", "tipo": "distrito", "indice": indice, "campos": mudou }) };
            Ok((d.gravar(), json!({ "indice": indice }), mudanca))
        })
        .await
    }

    /// Portais (caixas de transporte) e regiões do `region.sev` do realm. Só o ponto de chegada é editável: o
    /// cliente acha a caixa pela própria `region.clt` e manda só o índice (`EC_World.cpp:2360-2373`).
    pub async fn portais(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        let (nome, do_realm, do_rascunho) = self.arquivo_do_mapa(gov, versao, caminho, "region.sev", rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let r = RegioesEditavel::ler(&bytes)?;
        let (mut caixas, mut regioes) = (Vec::new(), Vec::new());
        for reg in &r.registros {
            match reg {
                RegistroDeRegiao::Regiao(p) => regioes.push(json!(p.iter().map(|v| json!([v[0], v[2]])).collect::<Vec<_>>())),
                RegistroDeRegiao::Caixa(k) => caixas.push(json!({
                    "indice": caixas.len(), "destino": k.destino, "origem": k.origem, "nivel_minimo": k.nivel_minimo,
                    "pos": vetor_json(&k.pos), "exts": vetor_json(&k.exts), "alvo": vetor_json(&k.alvo),
                })),
            }
        }
        Ok(json!({ "item": nome, "versao_do_arquivo": r.versao, "no_rascunho": no_rascunho, "caixas": caixas, "regioes": regioes }))
    }

    pub async fn editar_portal(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: i64, indice: usize, campos: Map<String, Value>) -> Result<Value> {
        self.alterar_arquivo_do_mapa(gov, versao, caminho, "region.sev", rascunho, move |base| {
            let mut r = RegioesEditavel::ler(base)?;
            let mut mudou = Map::new();
            {
                let mut caixas = r.caixas_mut();
                let k = caixas.get_mut(indice).with_context(|| format!("portal {indice} não existe"))?;
                for (nome, valor) in &campos {
                    if nome != "alvo" {
                        bail!("só o ponto de chegada (alvo) do portal é editável no servidor: {nome}");
                    }
                    let novo = vetor_do_json(valor, "alvo")?;
                    if novo != k.alvo {
                        mudou.insert(nome.clone(), json!([vetor_json(&k.alvo), vetor_json(&novo)]));
                        k.alvo = novo;
                    }
                }
            }
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editado", "tipo": "portal", "indice": indice, "campos": mudou }) };
            Ok((r.gravar(), json!({ "indice": indice }), mudanca))
        })
        .await
    }
}

// ------------------------------------------------------------------ teleportes (S4, B245)

use pw_data_loader::world_targets::PontosDoMundo;

/// Item do `world_targets.sev`: fica na raiz do `config` (`manager.rs:489`).
const ITEM_DOS_TELEPORTES: &str = "servidor\\world_targets.sev";

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
    /// nível e os NPCs que oferecem o serviço (`NPC_ESSENCE.id_transmit_service`).
    pub async fn teleportes(&self, gov: &Governanca, versao: &str, caminho: &str, rascunho: Option<i64>) -> Result<Value> {
        if !caminho_de_mapa_valido(caminho) {
            bail!("mapa inválido");
        }
        let id_mapa = self.id_do_mapa(gov, versao, caminho).await?;
        let (nome, do_realm, do_rascunho) = self.item_do_realm(gov, versao, ITEM_DOS_TELEPORTES.to_string(), rascunho).await?;
        let no_rascunho = do_rascunho.is_some();
        let bytes = do_rascunho.map(|(_, b)| b).unwrap_or(do_realm);
        let w = PontosDoMundo::load_from_bytes(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
        let nomes: HashMap<i32, String> = match self.arquivo_do_cliente(gov, versao, "maps\\world\\world_targets.txt").await {
            Ok(b) => crate::mapas::ler_world_targets_txt(&crate::mapas::texto_do_arquivo(&b)),
            Err(_) => HashMap::new(),
        };
        // Quem leva a cada destino.
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
        let pontos: Vec<Value> = w
            .lista
            .iter()
            .filter(|p| p.mundo == id_mapa)
            .map(|p| json!({
                "id": p.id, "nome": nomes.get(&p.id), "dominio": p.dominio, "pos": vetor_json(&p.pos),
                "usado_por": usado_por.get(&p.id).cloned().unwrap_or_default(),
            }))
            .collect();
        Ok(json!({ "item": nome, "mapa": id_mapa, "no_rascunho": no_rascunho, "total": w.lista.len(), "pontos": pontos }))
    }

    /// Troca a posição de um destino (o mapa e o domínio ficam; o cliente só usa a posição do `.txt` dele para
    /// saber em que região o destino fica).
    pub async fn editar_teleporte(&self, gov: &Governanca, versao: &str, rascunho: i64, id: i32, campos: Map<String, Value>) -> Result<Value> {
        let trava = self.trava(rascunho);
        let _guarda = trava.lock().await;
        let (nome, do_realm, do_rascunho) = self.item_do_realm(gov, versao, ITEM_DOS_TELEPORTES.to_string(), Some(rascunho)).await?;
        self.gravar_item_do_realm(gov, versao, rascunho, nome, do_realm, do_rascunho, move |base| {
            let mut w = PontosDoMundo::load_from_bytes(base).map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut mudou = Map::new();
            for (k, v) in &campos {
                if k != "pos" {
                    bail!("só a posição do destino é editável: {k}");
                }
                let novo = vetor_do_json(v, "pos")?;
                let antes = w.get(id).with_context(|| format!("destino {id} não existe"))?.pos;
                if novo != antes {
                    w.definir_posicao(id, novo);
                    mudou.insert(k.clone(), json!([vetor_json(&antes), vetor_json(&novo)]));
                }
            }
            let mudanca = if mudou.is_empty() { Value::Null } else { json!({ "acao": "editado", "tipo": "teleporte", "id": id, "campos": mudou }) };
            Ok((w.gravar(), json!({ "id": id }), mudanca))
        })
        .await
    }
}
