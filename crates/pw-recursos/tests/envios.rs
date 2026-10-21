//! B332 — envio do cliente original pelo painel de ponta a ponta: criar o envio, mandar a instalação arquivo a arquivo
//! em blocos (com retomada), importar como versão 0 em segundo plano e apagar. Com o banco (`TEST_DATABASE_URL`).
use pw_recursos::assinatura::Chave;
use pw_recursos::fonte::{FonteDePacote, ResultadoDoPacote};
use pw_recursos::governanca::Governanca;
use pw_storage::{PostgresPool, RecursosRepository, StorageConfig};
use serde_json::{json, Value};
use std::sync::Arc;

const LINHA: &str = "t_envio";
const SEGREDO: &str = "segredo-de-teste-0123456789";

/// O `configs.pck` de mentira (como no `governanca.rs`): dois arquivos.
struct Pacotes;

impl FonteDePacote for Pacotes {
    fn ler(&self, _pck: &std::path::Path, receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> anyhow::Result<()>) -> anyhow::Result<ResultadoDoPacote> {
        let mut r = ResultadoDoPacote::default();
        for (n, d) in [(b"configs\\b.txt".to_vec(), b"do pacote".to_vec()), (b"configs\\c.txt".to_vec(), b"outro".to_vec())] {
            r.entradas += 1;
            receber(n, d)?;
        }
        Ok(r)
    }
}

async fn apagar_linha(pool: &PostgresPool) {
    for sql in [
        "DELETE FROM recursos_alteracao_itens WHERE alteracao IN (SELECT id FROM recursos_alteracoes WHERE linha = $1)",
        "DELETE FROM recursos_alteracoes WHERE linha = $1",
        "DELETE FROM recursos_linhas WHERE id = $1",
    ] {
        sqlx::query(sql).bind(LINHA).execute(pool.get_ref()).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enviar_a_instalacao_em_blocos_e_importar() {
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
    apagar_linha(&pool).await;
    let dados = std::env::temp_dir().join(format!("pw_recursos_envio_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dados);
    let (chave, _) = Chave::carregar_ou_criar(&dados.join("_chave").join("k")).unwrap();
    let gov = Governanca { dados: dados.clone(), repo: RecursosRepository::new(pool.clone()), chave: Arc::new(chave), realms: Default::default() };
    let admin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", admin.local_addr().unwrap());
    let rotas = pw_recursos::admin::rotas_com_fonte(gov, SEGREDO.into(), Arc::new(Pacotes));
    tokio::spawn(async move { axum::serve(admin, rotas).await });

    let (b2, d2) = (base.clone(), dados.clone());
    tokio::task::spawn_blocking(move || {
        let pedir = |metodo: &str, rel: &str| ureq::request(metodo, &format!("{b2}/admin/{rel}")).set("x-recursos-segredo", SEGREDO);
        let json_de = |r: ureq::Response| -> Value { serde_json::from_str(&r.into_string().unwrap()).unwrap() };
        let erro = |r: Result<ureq::Response, ureq::Error>| -> (u16, Value) {
            match r {
                Err(ureq::Error::Status(s, resp)) => (s, serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap_or(Value::Null)),
                Ok(r) => panic!("esperava erro, veio {}", r.status()),
                Err(e) => panic!("{e}"),
            }
        };
        let id = json_de(pedir("POST", "envios").send_json(json!({"descricao": "instalação de teste"})).unwrap())["id"]
            .as_str()
            .unwrap()
            .to_string();
        // A instalação (pasta "element" escolhida no navegador): um .bat e um texto em dois blocos.
        let bloco = |caminho: &str, desl: u64, b: &[u8]| {
            pedir("PUT", &format!("envios/{id}/arquivo")).query("caminho", caminho).query("deslocamento", &desl.to_string()).send_bytes(b)
        };
        assert_eq!(json_de(bloco("element/elementclient.bat", 0, b"elementclient.exe game:cpw").unwrap())["tamanho"], json!(26));
        assert_eq!(json_de(bloco("element/configs/a.txt", 0, b"meio ").unwrap())["tamanho"], json!(5));
        // Retomar: o bloco repetido (deslocamento velho) é recusado com o tamanho que já chegou.
        let (s, v) = erro(bloco("element/configs/a.txt", 0, b"meio "));
        assert_eq!((s, v["tamanho"].clone()), (409, json!(5)));
        assert_eq!(json_de(bloco("element/configs/a.txt", 5, b"arquivo").unwrap())["tamanho"], json!(12));
        assert_eq!(erro(bloco("../fora.txt", 0, b"x")).0, 400, "caminho para fora do envio");
        // O pacote vazio: quem o lê é a fonte (a de mentira aqui; no serviço, o extrator).
        assert_eq!(json_de(bloco("element/configs.pck", 0, b"").unwrap())["tamanho"], json!(0));
        let ver = json_de(pedir("GET", &format!("envios/{id}")).call().unwrap());
        assert_eq!((ver["quantos"].clone(), ver["total"].clone()), (json!(3), json!(38)), "{ver}");
        assert_eq!(ver["arquivos"][1], json!(["element/configs/a.txt", 12]), "{ver}");
        assert_eq!(std::fs::read(d2.join(".envios").join(&id).join("element").join("configs").join("a.txt")).unwrap(), b"meio arquivo");
        assert_eq!(erro(pedir("POST", &format!("envios/{id}/importar")).send_json(json!({"versao": LINHA, "subpasta": "nada"}))).0, 400);
        // Importar a subpasta "element" como a versão 0 da linha, em segundo plano.
        let r = pedir("POST", &format!("envios/{id}/importar")).send_json(json!({"versao": LINHA, "subpasta": "element", "nivel": 3})).unwrap();
        assert_eq!(r.status(), 202);
        let limite = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let fim = loop {
            let v = json_de(pedir("GET", &format!("envios/{id}")).call().unwrap());
            if v["importacao"]["estado"] != json!("em_andamento") {
                break v;
            }
            assert!(std::time::Instant::now() < limite, "a importação não terminou: {v}");
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        assert_eq!(fim["importacao"]["estado"], json!("concluida"), "{fim}");
        assert!(fim["importacao"]["mensagem"].as_str().unwrap().contains("versão 0 de t_envio: 4 arquivos, 0.0 MB; pacotes .pck lidos 1"), "{fim}");
        // A versão aparece no serviço; a segunda importação sem "refazer" é recusada (falha registrada).
        let versoes = json_de(pedir("GET", "versoes").call().unwrap());
        assert!(versoes.as_array().unwrap().iter().any(|x| x["versao"] == json!(LINHA)), "{versoes}");
        assert_eq!(pedir("POST", &format!("envios/{id}/importar")).send_json(json!({"versao": LINHA, "subpasta": "element"})).unwrap().status(), 202);
        let limite = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let de_novo = loop {
            let v = json_de(pedir("GET", &format!("envios/{id}")).call().unwrap());
            if v["importacao"]["estado"] != json!("em_andamento") {
                break v;
            }
            assert!(std::time::Instant::now() < limite);
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        assert_eq!(de_novo["importacao"]["estado"], json!("falha"), "{de_novo}");
        assert!(json_de(pedir("GET", "envios").call().unwrap())["envios"].as_array().unwrap().iter().any(|e| e["id"] == json!(id)));
        assert_eq!(json_de(pedir("DELETE", &format!("envios/{id}")).call().unwrap())["apagado"], json!(id));
        assert!(!d2.join(".envios").join(&id).exists());
        assert_eq!(erro(pedir("GET", &format!("envios/{id}")).call()).0, 404);
    })
    .await
    .unwrap();
    apagar_linha(&pool).await;
    let _ = std::fs::remove_dir_all(&dados);
}
