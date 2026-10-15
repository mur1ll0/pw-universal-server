//! Estúdio S1 de ponta a ponta (B237), com o `elements.data` real do 1.2.6: listar, ficha,
//! editar (o resumo guarda o "antes" do publicado), clonar com id do espaço, remover recusado
//! por referência, desfazer tudo volta ao publicado, publicar grava no realm.

use anyhow::Result;
use pw_data_loader::generic_elements::{load_elements_data, FieldValue};
use pw_recursos::assinatura::Chave;
use pw_recursos::fonte::{FonteDePacote, ResultadoDoPacote};
use pw_recursos::governanca::Governanca;
use pw_recursos::importar::{importar, Importacao};
use pw_storage::{PostgresPool, RecursosRepository, StorageConfig};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const LINHA: &str = "t_p4";
const SEGREDO: &str = "segredo-de-teste-estudio-0123";

fn elements_126() -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/elements.data");
    fs::read(&p).unwrap_or_else(|e| panic!("{}: {e} (os dados do realm são obrigatórios)", p.display()))
}

struct Pacotes(Vec<u8>);

impl FonteDePacote for Pacotes {
    fn ler(&self, _pck: &Path, receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>) -> Result<ResultadoDoPacote> {
        receber(b"data\\elements.data".to_vec(), self.0.clone())?;
        Ok(ResultadoDoPacote { entradas: 1, falhas: vec![] })
    }
}

fn pasta_limpa(nome: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("pw_recursos_p4_{nome}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    p
}

async fn apagar_tudo(pool: &PostgresPool) {
    for sql in [
        "DELETE FROM recursos_alteracao_itens WHERE alteracao IN (SELECT id FROM recursos_alteracoes WHERE linha = $1)",
        "DELETE FROM recursos_alteracoes WHERE linha = $1",
        "DELETE FROM recursos_linhas WHERE id = $1",
    ] {
        sqlx::query(sql).bind(LINHA).execute(pool.get_ref()).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editar_o_elements_pelo_estudio_e_publicar_no_realm() {
    let pool = PostgresPool::new(&StorageConfig {
        database_url: std::env::var("TEST_DATABASE_URL").expect("banco obrigatório"),
        max_connections: 4,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .unwrap();
    let esquema: String = sqlx::query_scalar("SELECT current_schema()").fetch_one(pool.get_ref()).await.unwrap();
    assert_eq!(esquema, "test");
    apagar_tudo(&pool).await;
    let repo = RecursosRepository::new(pool.clone());

    let original = elements_126();
    let cliente = pasta_limpa("cliente");
    fs::create_dir_all(&cliente).unwrap();
    fs::write(cliente.join("configs.pck"), b"").unwrap();
    fs::write(cliente.join("elementclient.bat"), b"elementclient.exe game:cpw console=1").unwrap();
    let dados = pasta_limpa("dados");
    let realm = pasta_limpa("realm");
    fs::create_dir_all(&realm).unwrap();
    fs::write(realm.join("elements.data"), &original).unwrap();
    let (chave, _) = Chave::carregar_ou_criar(&dados.join("_chave").join("k")).unwrap();
    let res = importar(&Importacao {
        versao: LINHA,
        cliente: &cliente,
        dados: &dados,
        nivel: 3,
        linha_de_grupo: false,
        fonte: &Pacotes(original.clone()),
        chave: &chave,
    })
    .unwrap();
    repo.gravar_versao_zero(LINHA, "teste", &res.manifesto_sha256, &res.assinatura, &pw_recursos::para_o_banco(&res.registros))
        .await
        .unwrap();

    let admin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", admin.local_addr().unwrap());
    let gov = Governanca {
        dados: dados.clone(),
        repo: RecursosRepository::new(pool.clone()),
        chave: Arc::new(chave),
        realms: [(LINHA.to_string(), vec![realm.clone()])].into(),
    };
    let rotas = pw_recursos::admin::rotas(gov, SEGREDO.into());
    tokio::spawn(async move { axum::serve(admin, rotas).await });

    let realm2 = realm.clone();
    let original2 = original.clone();
    tokio::task::spawn_blocking(move || {
        let url = |rel: &str| format!("{base}/admin/{LINHA}/{rel}");
        let pedir = |metodo: &str, rel: &str| ureq::request(metodo, &url(rel)).set("x-recursos-segredo", SEGREDO);
        let json_de = |r: ureq::Response| -> Value { serde_json::from_str(&r.into_string().unwrap()).unwrap() };
        let erro_de = |r: Result<ureq::Response, ureq::Error>| -> (u16, String) {
            match r {
                Err(ureq::Error::Status(s, r)) => (s, r.into_string().unwrap()),
                Ok(r) => (r.status(), String::new()),
                Err(e) => panic!("{e}"),
            }
        };

        // Tabelas e listagem do publicado.
        let t = json_de(pedir("GET", "estudio/tabelas").call().unwrap());
        assert_eq!(t["versao_do_arquivo"], 7);
        let armas = t["tabelas"].as_array().unwrap().iter().find(|x| x["nome"] == "WEAPON_ESSENCE").unwrap().clone();
        assert_eq!(armas["espaco"], "ESSENCE");
        let lista = json_de(pedir("GET", "estudio/tabelas/WEAPON_ESSENCE").query("por_pagina", "5").call().unwrap());
        assert_eq!(lista["registros"].as_array().unwrap().len(), 5);
        let id0 = lista["registros"][0]["id"].as_i64().unwrap();
        let por_id = json_de(pedir("GET", "estudio/tabelas/WEAPON_ESSENCE").query("busca", &id0.to_string()).call().unwrap());
        assert_eq!(por_id["total"], 1);
        let ficha = json_de(pedir("GET", "estudio/tabelas/WEAPON_ESSENCE/0").call().unwrap());
        let valor = |f: &Value, nome: &str| f["campos"].as_array().unwrap().iter().find(|c| c["nome"] == nome).unwrap()["valor"].clone();
        let (nome0, preco0) = (valor(&ficha, "Name"), valor(&ficha, "price"));
        // Rótulos, grupos e ícone (B239).
        let campo = |f: &Value, nome: &str| f["campos"].as_array().unwrap().iter().find(|c| c["nome"] == nome).unwrap().clone();
        assert_eq!((campo(&ficha, "price")["rotulo"].clone(), campo(&ficha, "price")["grupo"].clone()), (json!("Preço de venda ao NPC"), json!("Comércio")));
        assert_eq!(campo(&ficha, "Name")["grupo"], "Identificação");
        assert!(ficha["icone"].as_str().is_some_and(|h| !h.is_empty() && hex::decode(h).is_ok()), "a arma tem file_icon: {}", ficha["icone"]);
        assert!(lista["registros"][0]["icone"].is_string());

        // Índice de referências (B238): drop do monstro com o nome do item; quem usa um tipo de arma;
        // busca no espaço para o seletor.
        let mut drop_com_nome = false;
        for i in 0..40 {
            let m = json_de(pedir("GET", &format!("estudio/tabelas/MONSTER_ESSENCE/{i}")).call().unwrap());
            let campo = m["campos"].as_array().unwrap().iter().find(|c| c["nome"] == "drop_matters_1_id").unwrap().clone();
            assert_eq!(campo["cita"], "ESSENCE");
            if let Some(r) = campo.get("referencia") {
                assert!(r.get("ausente").is_none(), "drop citando id inexistente no monstro {i}: {r}");
                assert!(r["tabela"].as_str().unwrap().ends_with("ESSENCE"));
                drop_com_nome = true;
                break;
            }
        }
        assert!(drop_com_nome, "nenhum dos 40 primeiros monstros tem drop");
        let tipo = json_de(pedir("GET", "estudio/tabelas/WEAPON_MAJOR_TYPE/0").call().unwrap());
        let usado = tipo["usado_por"].as_array().unwrap();
        assert!(!usado.is_empty() && usado.iter().all(|u| u["campo"] == "id_major_type"), "{usado:?}");
        let achados = json_de(pedir("GET", "estudio/espacos/ESSENCE").query("busca", &id0.to_string()).call().unwrap());
        assert!(achados.as_array().unwrap().iter().any(|a| a["id"] == id0 && a["tabela"] == "WEAPON_ESSENCE"));
        assert_eq!(erro_de(pedir("GET", "estudio/espacos/XYZ").call()).0, 404);

        // Sem rascunho a edição é recusada.
        assert_eq!(erro_de(pedir("PUT", "estudio/tabelas/WEAPON_ESSENCE/0").send_json(json!({"campos": {"price": 1}}))).0, 400);
        let r = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "estudio", "operacao_id": "t_p4_r1"})).unwrap())["id"].as_i64().unwrap();
        let em = |rel: &str| format!("{rel}?rascunho={r}");

        // Editar duas vezes o preço: o "antes" continua o do publicado.
        pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"Name": "Espada do Estúdio", "price": 111}})).unwrap();
        pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"price": 222}})).unwrap();
        let f = json_de(pedir("GET", &em("estudio/tabelas/WEAPON_ESSENCE/0")).call().unwrap());
        assert_eq!((valor(&f, "Name"), valor(&f, "price")), (json!("Espada do Estúdio"), json!(222)));
        assert_eq!(valor(&json_de(pedir("GET", "estudio/tabelas/WEAPON_ESSENCE/0").call().unwrap()), "price"), preco0, "o publicado não muda");
        // Tipo errado e campo inexistente.
        assert_eq!(erro_de(pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"price": "x"}}))).0, 400);
        assert_eq!(erro_de(pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"nao_existe": 1}}))).0, 400);

        // Clonar: id novo do espaço ESSENCE; remover o clone desfaz a criação no resumo.
        let c = json_de(pedir("POST", &em("estudio/tabelas/WEAPON_ESSENCE/0/clonar")).call().unwrap())["resultado"].clone();
        let (ic, idc) = (c["indice"].as_u64().unwrap(), c["id"].as_i64().unwrap());
        assert!(idc > id0);
        // Remover um tipo de arma usado pelas armas é recusado.
        let (s, msg) = erro_de(pedir("DELETE", &em("estudio/tabelas/WEAPON_MAJOR_TYPE/0")).call());
        assert_eq!(s, 400, "{msg}");
        assert!(msg.contains("citado"), "{msg}");
        pedir("DELETE", &em(&format!("estudio/tabelas/WEAPON_ESSENCE/{ic}"))).call().unwrap();

        let hist = json_de(pedir("GET", "alteracoes").call().unwrap());
        let item = &hist.as_array().unwrap().iter().find(|a| a["id"] == r).unwrap()["lista"][0];
        assert_eq!((item["caminho"].as_str(), item["destino"].as_str()), (Some("data\\elements.data"), Some("ambos")));
        let campos = item["resumo"]["campos"].as_array().unwrap();
        assert_eq!(campos.len(), 2, "{campos:?}");
        let preco = campos.iter().find(|c| c["campo"] == "price").unwrap();
        assert_eq!((preco["antes"].clone(), preco["depois"].clone()), (preco0.clone(), json!(222)));
        assert!(item["resumo"]["registros"].as_array().unwrap().is_empty(), "clone criado e removido no mesmo rascunho");

        // Voltar os valores ao publicado tira o item do rascunho.
        let v = json_de(pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"Name": nome0, "price": preco0}})).unwrap());
        assert_eq!(v["igual_ao_publicado"], true);

        // Editar de novo e publicar: o cliente ganha a versão 1 e o realm o arquivo novo.
        pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"price": 333}})).unwrap();
        let p = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r]})).unwrap());
        assert_eq!(p["numero"], 1);
        assert_eq!(p["aplicados"].as_array().unwrap().len(), 1, "{p}");
        let no_realm = fs::read(realm2.join("elements.data")).unwrap();
        assert_ne!(no_realm, original2);
        assert_eq!(no_realm.len(), original2.len());
        let lido = load_elements_data(&no_realm).unwrap();
        assert_eq!(lido.get("WEAPON_ESSENCE")[0]["price"], FieldValue::Int(333));
        // O publicado agora é o novo: a ficha sem rascunho mostra 333.
        assert_eq!(valor(&json_de(pedir("GET", "estudio/tabelas/WEAPON_ESSENCE/0").call().unwrap()), "price"), json!(333));
    })
    .await
    .unwrap();

    apagar_tudo(&pool).await;
    for p in [cliente, dados, realm] {
        let _ = fs::remove_dir_all(p);
    }
}
