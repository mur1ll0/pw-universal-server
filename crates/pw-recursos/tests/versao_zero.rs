//! P1 de ponta a ponta (B233): instalação de mentira → importar → banco (schema `test`) →
//! conferir por pasta e por HTTP com Range → manifesto adulterado recusado. O `.pck` vem de uma
//! fonte em memória: o extrator real (Python) é provado pela importação dos dois clientes.

use anyhow::Result;
use pw_recursos::assinatura::Chave;
use pw_recursos::conferir::{conferir, Origem};
use pw_recursos::fonte::{FonteDePacote, ResultadoDoPacote};
use pw_recursos::importar::{importar, Importacao};
use pw_recursos::nomes;
use pw_storage::{PostgresPool, RecursosRepository, StorageConfig};
use std::fs;
use std::path::{Path, PathBuf};

const LINHA: &str = "t_p1";

struct PacotesDeTeste;

fn gbk(t: &str) -> Vec<u8> {
    nomes::gbk_de_unicode(t).unwrap()
}

fn nome_longo() -> Vec<u8> {
    gbk(&format!("configs\\{}.txt", "形象".repeat(30))) // > 100 bytes: entrada LongLink
}

impl FonteDePacote for PacotesDeTeste {
    fn ler(&self, pck: &Path, receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>) -> Result<ResultadoDoPacote> {
        let mut r = ResultadoDoPacote::default();
        let entradas: Vec<(Vec<u8>, Vec<u8>)> = match pck.file_name().unwrap().to_str().unwrap() {
            "configs.pck" => vec![
                (b"configs/a.txt".to_vec(), b"do configs".to_vec()),
                (b"Configs\\Repetido.txt".to_vec(), b"vale o pacote".to_vec()),
                (nome_longo(), vec![42u8; 3000]),
            ],
            "models.pck" => {
                r.falhas.push((b"models\\quebrado.ecm".to_vec(), "zlib".into()));
                vec![
                    (b"configs\\a.txt".to_vec(), b"repetido entre pacotes".to_vec()),
                    (gbk("models\\players\\形象\\白虎\\白虎.ecm"), b"ecm".to_vec()),
                ]
            }
            outro => panic!("pacote inesperado {outro}"),
        };
        for (n, d) in entradas {
            r.entradas += 1;
            receber(n, d)?;
        }
        Ok(r)
    }
}

fn utf16(t: &str) -> Vec<u8> {
    [0xFF, 0xFE].into_iter().chain(t.encode_utf16().flat_map(|u| u.to_le_bytes())).collect()
}

fn escrever(raiz: &Path, rel: &str, dados: &[u8]) {
    let p = rel.split('\\').fold(raiz.to_path_buf(), |p, x| p.join(x));
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, dados).unwrap();
}

fn pasta_limpa(nome: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("pw_recursos_{nome}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    p
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn importa_registra_serve_e_confere_a_versao_zero() {
    let pool = PostgresPool::new(&StorageConfig {
        database_url: std::env::var("TEST_DATABASE_URL").expect("banco obrigatório"),
        max_connections: 2,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .unwrap();
    let esquema: String = sqlx::query_scalar("SELECT current_schema()").fetch_one(pool.get_ref()).await.unwrap();
    assert_eq!(esquema, "test");
    let repo = RecursosRepository::new(pool);
    assert!(repo.apagar_versao_zero(LINHA).await.unwrap());

    // Instalação original de mentira.
    let cliente = pasta_limpa("cliente");
    for (rel, d) in [
        ("configs.pck", &b""[..]),
        ("models.pck", b""),
        ("elementclient.exe", b"MZ"),
        ("elementclient.bat", b"elementclient.exe game:cpw console=1"),
        ("configs\\repetido.txt", b"solto perde"),
        ("logs\\ec.txt", b"x"),
        ("models.pck.log", b"x"),
        ("sPCK.exe", b"x"),
        ("gfx.pck.files\\a.gfx", b"x"),
    ] {
        escrever(&cliente, rel, d);
    }
    escrever(&cliente, "userdata\\server\\serverlist.txt", &utf16("Universal 1.5.5\t29004:127.0.0.1\t2\r\n"));
    escrever(&cliente, "userdata\\Padrão.ini", b"fora do gbk");
    escrever(&cliente, "cursors\\七夕鼠标.cur", b"cursor");

    let dados = pasta_limpa("dados");
    let (chave, _) = Chave::carregar_ou_criar(&dados.join("_chave").join("k")).unwrap();
    let publica = chave.publica_hex();
    let imp = Importacao {
        versao: LINHA,
        cliente: &cliente,
        dados: &dados,
        nivel: 3,
        linha_de_grupo: true,
        fonte: &PacotesDeTeste,
        chave: &chave,
    };
    let res = importar(&imp).unwrap();
    let r = &res.relatorio;
    assert_eq!(r.repetidos_entre_pacotes.len(), 1);
    assert_eq!(r.falhas_de_pacote.len(), 1);
    assert_eq!(r.soltos_repetidos_do_pacote, 1, "AFileImage.cpp:94-104: o pacote vale primeiro");
    assert_eq!(r.fora_do_gbk, vec!["userdata\\Padrão.ini".to_string()]);
    assert_eq!(r.excluidos.values().sum::<u64>(), 6); // 2 .pck, log, .pck.log, sPCK, .pck.files
    assert_eq!(r.nossos.len(), 2);
    // configs ×3, ecm, exe, bat, serverlist, cursor
    assert_eq!(res.registros.len(), 8);
    assert!(importar(&imp).is_err(), "sem --refazer não sobrescreve a versão 0");

    repo.gravar_versao_zero(LINHA, "teste", &res.manifesto_sha256, &res.assinatura, &pw_recursos::para_o_banco(&res.registros))
        .await
        .unwrap();
    assert_eq!(repo.versoes(LINHA).await.unwrap(), vec![0]);
    let resumo = repo.resumo_da_base(LINHA).await.unwrap();
    let contagem: Vec<(String, i64)> = resumo.iter().map(|(o, n, _)| (o.clone(), *n)).collect();
    assert_eq!(contagem, vec![("nosso".into(), 2), ("pacote".into(), 4), ("solto".into(), 2)]);

    // Conferência pela pasta.
    let destino = pasta_limpa("destino");
    let c = conferir(&Origem::Pasta(dados.clone()), LINHA, &destino, &publica).unwrap();
    assert_eq!((c.arquivos, c.range_conferido), (8, false));
    let no_disco = |gbk: &[u8]| destino.join(nomes::nome_no_disco(gbk).unwrap());
    assert_eq!(fs::read(no_disco(b"Configs\\Repetido.txt")).unwrap(), b"vale o pacote");
    assert_eq!(fs::read(no_disco(&nome_longo())).unwrap(), vec![42u8; 3000]);
    assert_eq!(fs::read(no_disco(&gbk("cursors\\七夕鼠标.cur"))).unwrap(), b"cursor");
    assert_eq!(
        fs::read(no_disco(b"elementclient.bat")).unwrap(),
        b"elementclient.exe game:cpw console=1 sepfile"
    );
    let lista = fs::read(no_disco(b"userdata\\server\\serverlist.txt")).unwrap();
    assert!(lista.starts_with(&utf16("pw-universal\r\nUniversal 1.5.5\t")));

    // Conferência por HTTP, com Range.
    let escuta = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = escuta.local_addr().unwrap();
    let rotas = pw_recursos::servir::rotas(dados.clone());
    tokio::spawn(async move { axum::serve(escuta, rotas).await });
    let destino_http = pasta_limpa("destino_http");
    let (d2, p2) = (destino_http.clone(), publica.clone());
    let c = tokio::task::spawn_blocking(move || {
        conferir(&Origem::Http(format!("http://{endereco}")), LINHA, &d2, &p2)
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!((c.arquivos, c.range_conferido), (8, true));

    // Manifesto adulterado: a assinatura recusa.
    let manifesto = dados.join(LINHA).join("atualizacao").join("0").join("manifesto.json");
    let mut m = fs::read(&manifesto).unwrap();
    let i = m.iter().position(|&b| b == b'"').unwrap();
    m.insert(i, b' ');
    fs::write(&manifesto, m).unwrap();
    let erro = conferir(&Origem::Pasta(dados.clone()), LINHA, &pasta_limpa("destino3"), &publica).unwrap_err();
    assert!(format!("{erro:#}").contains("SHA-256 do manifesto"), "{erro:#}");

    assert!(repo.apagar_versao_zero(LINHA).await.unwrap());
    for p in [cliente, dados, destino, destino_http] {
        let _ = fs::remove_dir_all(p);
    }
}
