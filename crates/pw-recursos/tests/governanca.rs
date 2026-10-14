//! P3 de ponta a ponta (B235): importar uma instalação de mentira → API administrativa (segredo,
//! rascunho, trava por caminho, publicar) → o launcher atualiza uma pasta e instala outra do zero
//! na versão 1 (pacotes da 0 + objetos) → reverter → versão 2 volta ao conteúdo original.

use anyhow::Result;
use pw_atualizador::cliente::{self as launcher, Origem, Servidor};
use pw_recursos::assinatura::Chave;
use pw_recursos::fonte::{FonteDePacote, ResultadoDoPacote};
use pw_recursos::governanca::Governanca;
use pw_recursos::importar::{importar, Importacao};
use pw_recursos::nomes;
use pw_storage::{PostgresPool, RecursosRepository, StorageConfig};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const LINHA: &str = "t_p3";
const SEGREDO: &str = "segredo-de-teste-0123456789";

struct Pacotes;

impl FonteDePacote for Pacotes {
    fn ler(&self, _pck: &Path, receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>) -> Result<ResultadoDoPacote> {
        let mut r = ResultadoDoPacote::default();
        for (n, d) in [
            (b"configs\\a.txt".to_vec(), b"original".to_vec()),
            (b"configs\\b.txt".to_vec(), b"fica".to_vec()),
            (nomes::gbk_de_unicode("cursors\\七夕鼠标.cur").unwrap(), b"cursor".to_vec()),
        ] {
            r.entradas += 1;
            receber(n, d)?;
        }
        Ok(r)
    }
}

fn pasta_limpa(nome: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("pw_recursos_p3_{nome}_{}", std::process::id()));
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

fn ler(p: &Path, gbk: &[u8]) -> Option<Vec<u8>> {
    fs::read(p.join(nomes::nome_no_disco(gbk).unwrap())).ok()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rascunho_publicar_atualizar_e_reverter() {
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

    // Versão 0.
    let cliente = pasta_limpa("cliente");
    fs::create_dir_all(&cliente).unwrap();
    fs::write(cliente.join("configs.pck"), b"").unwrap();
    fs::write(cliente.join("elementclient.bat"), b"elementclient.exe game:cpw console=1").unwrap();
    let dados = pasta_limpa("dados");
    let (chave, _) = Chave::carregar_ou_criar(&dados.join("_chave").join("k")).unwrap();
    let publica = chave.publica_hex();
    let res = importar(&Importacao {
        versao: LINHA,
        cliente: &cliente,
        dados: &dados,
        nivel: 3,
        linha_de_grupo: false,
        fonte: &Pacotes,
        chave: &chave,
    })
    .unwrap();
    repo.gravar_versao_zero(LINHA, "teste", &res.manifesto_sha256, &res.assinatura, &pw_recursos::para_o_banco(&res.registros))
        .await
        .unwrap();

    // Servidores: público e administrativo.
    let publico = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", publico.local_addr().unwrap());
    let rotas = pw_recursos::servir::rotas(dados.clone());
    tokio::spawn(async move { axum::serve(publico, rotas).await });
    let admin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_admin = format!("http://{}", admin.local_addr().unwrap());
    let gov = Governanca { dados: dados.clone(), repo: RecursosRepository::new(pool.clone()), chave: Arc::new(chave) };
    let rotas_admin = pw_recursos::admin::rotas(gov, SEGREDO.into());
    tokio::spawn(async move { axum::serve(admin, rotas_admin).await });

    let (pa, pb) = (pasta_limpa("a"), pasta_limpa("b"));
    let (pa2, pb2) = (pa.clone(), pb.clone());
    tokio::task::spawn_blocking(move || {
        let serv = Servidor::new(Origem::Http(base.clone()), publica);
        let cursor = nomes::gbk_de_unicode("cursors\\七夕鼠标.cur").unwrap();
        launcher::instalar(&serv, LINHA, &pa2, &mut |_| {}).unwrap();

        let url = |rel: &str| format!("{base_admin}/admin/{LINHA}/{rel}");
        let pedir = |metodo: &str, rel: &str| ureq::request(metodo, &url(rel)).set("x-recursos-segredo", SEGREDO);
        let json_de = |r: ureq::Response| -> Value { serde_json::from_str(&r.into_string().unwrap()).unwrap() };
        let status_de = |r: Result<ureq::Response, ureq::Error>| match r {
            Ok(r) => r.status(),
            Err(ureq::Error::Status(s, _)) => s,
            Err(e) => panic!("{e}"),
        };

        // Sem segredo: 401.
        assert_eq!(status_de(ureq::get(&format!("{base_admin}/admin/versoes")).call()), 401);
        let v = json_de(ureq::get(&format!("{base_admin}/admin/versoes")).set("x-recursos-segredo", SEGREDO).call().unwrap());
        assert_eq!(v, json!([{ "versao": LINHA, "atual": 0, "minima": 0 }]));

        // Rascunho: troca a.txt (o GM digita em maiúsculas e com /), acrescenta novo.txt, remove o cursor.
        let r1 = json_de(pedir("POST", "rascunhos").send_json(json!({"autor": null, "descricao": "teste", "operacao_id": "t_p3_r1"})).unwrap())["id"]
            .as_i64()
            .unwrap();
        let r1_de_novo = json_de(pedir("POST", "rascunhos").send_json(json!({"autor": null, "descricao": "teste", "operacao_id": "t_p3_r1"})).unwrap())["id"]
            .as_i64()
            .unwrap();
        assert_eq!(r1, r1_de_novo, "operacao_id idempotente");
        let item = json_de(pedir("PUT", &format!("rascunhos/{r1}/arquivo")).query("caminho", "CONFIGS/A.TXT").send_bytes(b"versao 1").unwrap());
        assert_eq!((item["acao"].as_str(), item["caminho"].as_str()), (Some("substituir"), Some("configs\\a.txt")));
        let item = json_de(pedir("PUT", &format!("rascunhos/{r1}/arquivo")).query("caminho", "configs\\novo.txt").send_bytes(b"novo").unwrap());
        assert_eq!(item["acao"], "adicionar");
        pedir("POST", &format!("rascunhos/{r1}/remover")).query("caminho", "cursors\\七夕鼠标.cur").call().unwrap();
        assert_eq!(
            status_de(pedir("PUT", &format!("rascunhos/{r1}/arquivo")).query("caminho", "configs\\b.txt").send_bytes(b"fica")),
            400,
            "igual ao publicado"
        );
        // Trava: o mesmo caminho em outro rascunho.
        let r2 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "outro", "operacao_id": "t_p3_r2"})).unwrap())["id"]
            .as_i64()
            .unwrap();
        assert_eq!(status_de(pedir("PUT", &format!("rascunhos/{r2}/arquivo")).query("caminho", "configs\\a.txt").send_bytes(b"x")), 409);
        pedir("POST", &format!("rascunhos/{r2}/descartar")).call().unwrap();

        let hist = json_de(pedir("GET", "alteracoes").call().unwrap());
        let h1 = hist.as_array().unwrap().iter().find(|a| a["id"] == r1).unwrap();
        assert_eq!((h1["estado"].as_str(), h1["itens"].as_i64()), (Some("rascunho"), Some(3)));
        let busca = json_de(pedir("GET", "arquivos").query("busca", "CONFIGS").call().unwrap());
        assert_eq!(busca.as_array().unwrap().len(), 2);
        let mut atual = Vec::new();
        std::io::Read::read_to_end(&mut pedir("GET", "arquivo").query("caminho", "configs\\a.txt").call().unwrap().into_reader(), &mut atual).unwrap();
        assert_eq!(atual, b"original");

        // Publicar a versão 1 (mínima 1: obriga a atualizar).
        let p = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r1], "minima": 1})).unwrap());
        assert_eq!(p["numero"], 1);
        assert_eq!(status_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r1]}))), 400, "já publicado");
        assert_eq!(serv.versao_anunciada(LINHA).unwrap().minima, 1);

        // Launcher: atualiza a pasta A; instala a B do zero na versão 1.
        let r = launcher::atualizar(&serv, &pa2, &mut |_| {}).unwrap().unwrap();
        assert_eq!((r.numero, r.arquivos, r.removidos), (1, 2, 1));
        assert_eq!(ler(&pa2, b"configs\\a.txt").unwrap(), b"versao 1");
        assert_eq!(ler(&pa2, b"configs\\novo.txt").unwrap(), b"novo");
        assert!(ler(&pa2, &cursor).is_none());
        let r = launcher::instalar(&serv, LINHA, &pb2, &mut |_| {}).unwrap();
        assert_eq!((r.numero, r.por_objeto, r.pulados), (1, 2, 2), "a.txt e novo.txt por objeto; a.txt antigo e o cursor pulados");
        assert!(launcher::verificar(&pb2, true, &mut |_| {}).unwrap().is_empty());
        assert!(ler(&pb2, &cursor).is_none());

        // Reverter a alteração e publicar a versão 2: volta ao conteúdo da 0.
        let rv = json_de(pedir("POST", &format!("alteracoes/{r1}/reverter")).send_json(json!({"operacao_id": "t_p3_rv"})).unwrap())["rascunho"]
            .as_i64()
            .unwrap();
        let p = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [rv]})).unwrap());
        assert_eq!(p["numero"], 2);
        let r = launcher::atualizar(&serv, &pa2, &mut |_| {}).unwrap().unwrap();
        assert_eq!((r.numero, r.arquivos, r.removidos), (2, 2, 1));
        assert_eq!(ler(&pa2, b"configs\\a.txt").unwrap(), b"original");
        assert_eq!(ler(&pa2, &cursor).unwrap(), b"cursor");
        assert!(ler(&pa2, b"configs\\novo.txt").is_none());
        assert!(launcher::verificar(&pa2, true, &mut |_| {}).unwrap().is_empty());
    })
    .await
    .unwrap();

    assert_eq!(repo.versoes(LINHA).await.unwrap(), vec![0, 1, 2]);
    // Os objetos do "antes" continuam em uso (é o que permite reverter).
    assert!(repo.objetos_em_uso(LINHA).await.unwrap().len() >= 5);
    apagar_tudo(&pool).await;
    for p in [cliente, dados, pa, pb] {
        let _ = fs::remove_dir_all(p);
    }
}
