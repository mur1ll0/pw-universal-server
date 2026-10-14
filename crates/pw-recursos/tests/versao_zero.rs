//! P1 de ponta a ponta (B233): instalação de mentira → importar → banco (schema `test`) →
//! conferir por pasta e por HTTP com Range → manifesto adulterado recusado. O `.pck` vem de uma
//! fonte em memória: o extrator real (Python) é provado pela importação dos dois clientes.

use anyhow::Result;
use pw_recursos::assinatura::Chave;
use pw_atualizador::cliente::{self as launcher, Origem, Servidor};
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

    // Limpeza: o .bat e a serverlist originais foram gravados como objetos e depois trocados
    // pelos "nossos" — são os 2 objetos sem uso. O resto continua legível.
    let em_uso: std::collections::HashSet<[u8; 32]> = repo
        .objetos_em_uso(LINHA)
        .await
        .unwrap()
        .into_iter()
        .map(|v| v.try_into().unwrap())
        .collect();
    let distintos: std::collections::HashSet<_> = res.registros.iter().map(|r| r.sha256).collect();
    assert_eq!(em_uso, distintos);
    let objetos = pw_recursos::objetos::Objetos::new(dados.join(LINHA).join("objetos"), 3);
    assert_eq!(objetos.limpar(&em_uso).unwrap().0, 2);
    assert_eq!(objetos.limpar(&em_uso).unwrap().0, 0);
    for r in &res.registros {
        objetos.ler(&r.sha256, r.tamanho as usize).unwrap();
    }

    // Instalação (o núcleo do launcher) pela pasta.
    let destino = pasta_limpa("destino");
    let pela_pasta = Servidor::new(Origem::Pasta(dados.clone()), publica.clone());
    let c = launcher::instalar(&pela_pasta, LINHA, &destino, &mut |_| {}).unwrap();
    assert_eq!((c.arquivos, c.pacotes), (8, 5));
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
    assert!(launcher::instalar(&pela_pasta, LINHA, &destino, &mut |_| {}).is_err(), "já instalado");

    // Por HTTP: lista de versões, instalação e Range.
    let escuta = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = escuta.local_addr().unwrap();
    let rotas = pw_recursos::servir::rotas(dados.clone());
    tokio::spawn(async move { axum::serve(escuta, rotas).await });
    let base = format!("http://{endereco}");
    let destino_http = pasta_limpa("destino_http");
    let (d2, p2, b2) = (destino_http.clone(), publica.clone(), base.clone());
    tokio::task::spawn_blocking(move || {
        let serv = Servidor::new(Origem::Http(b2), p2);
        assert_eq!(serv.versoes().unwrap(), vec![LINHA.to_string()]);
        let c = launcher::instalar(&serv, LINHA, &d2, &mut |_| {}).unwrap();
        assert_eq!(c.arquivos, 8);
        launcher::conferir_range(&serv, LINHA).unwrap();

        // Verificar e reparar: um arquivo com o mesmo tamanho e outro conteúdo, outro apagado.
        let a = d2.join(nomes::nome_no_disco(b"configs\\a.txt").unwrap());
        fs::write(&a, b"do configX").unwrap();
        let ecm = d2.join(nomes::nome_no_disco(&gbk("models\\players\\形象\\白虎\\白虎.ecm")).unwrap());
        fs::remove_file(&ecm).unwrap();
        assert_eq!(launcher::verificar(&d2, false, &mut |_| {}).unwrap().len(), 1, "rápido: só tamanho");
        assert_eq!(launcher::verificar(&d2, true, &mut |_| {}).unwrap().len(), 2);
        assert_eq!(launcher::reparar(&serv, &d2, &mut |_| {}).unwrap(), 2);
        assert!(launcher::verificar(&d2, true, &mut |_| {}).unwrap().is_empty());
        assert_eq!(fs::read(&a).unwrap(), b"do configs");
        assert!(launcher::atualizar(&serv, &d2, &mut |_| {}).unwrap().is_none(), "já na atual");
    })
    .await
    .unwrap();

    // Versão 1 montada à mão (o P3 publicará neste formato): troca um arquivo, tira o cursor e
    // acrescenta um. O launcher baixa só os objetos novos e apaga o que saiu.
    let atualizacao = dados.join(LINHA).join("atualizacao");
    let mut m: pw_recursos::manifesto::Manifesto =
        serde_json::from_slice(&fs::read(atualizacao.join("0").join("manifesto.json")).unwrap()).unwrap();
    let objetos = pw_recursos::objetos::Objetos::new(dados.join(LINHA).join("objetos"), 3);
    let novo_arquivo = |caminho: &[u8], conteudo: &[u8]| {
        let sha = pw_recursos::objetos::sha256(conteudo);
        objetos.gravar(&sha, conteudo).unwrap();
        pw_recursos::manifesto::Arquivo {
            caminho_gbk: hex::encode(caminho),
            sha256: hex::encode(sha),
            tamanho: conteudo.len() as u64,
            pacote: "configs".into(),
        }
    };
    let trocado = novo_arquivo(b"configs\\a.txt", b"versao 1 do a");
    let acrescentado = novo_arquivo(b"configs\\novo.txt", b"novo");
    let cursor = hex::encode(gbk("cursors\\七夕鼠标.cur"));
    m.arquivos.retain(|a| a.caminho_gbk != cursor && a.caminho_gbk != trocado.caminho_gbk);
    m.arquivos.push(trocado);
    m.arquivos.push(acrescentado);
    m.numero = 1;
    let m_bytes = serde_json::to_vec(&m).unwrap();
    fs::create_dir_all(atualizacao.join("1")).unwrap();
    fs::write(atualizacao.join("1").join("manifesto.json"), &m_bytes).unwrap();
    fs::write(atualizacao.join("1").join("manifesto.json.sig"), hex::encode(chave.assinar(&m_bytes))).unwrap();
    let v = pw_recursos::manifesto::Versao {
        formato: 1,
        versao_cliente: LINHA.into(),
        atual: 1,
        minima: 0,
        manifesto: "1/manifesto.json".into(),
        manifesto_sha256: hex::encode(pw_recursos::objetos::sha256(&m_bytes)),
    };
    let v_bytes = serde_json::to_vec(&v).unwrap();
    fs::write(atualizacao.join("versao.json"), &v_bytes).unwrap();
    fs::write(atualizacao.join("versao.json.sig"), hex::encode(chave.assinar(&v_bytes))).unwrap();

    let (d3, p3, b3) = (destino_http.clone(), publica.clone(), base.clone());
    tokio::task::spawn_blocking(move || {
        let serv = Servidor::new(Origem::Http(b3), p3);
        let r = launcher::atualizar(&serv, &d3, &mut |_| {}).unwrap().expect("havia atualização");
        assert_eq!((r.numero, r.arquivos, r.removidos), (1, 2, 1));
        assert_eq!(fs::read(d3.join(nomes::nome_no_disco(b"configs\\a.txt").unwrap())).unwrap(), b"versao 1 do a");
        assert_eq!(fs::read(d3.join(nomes::nome_no_disco(b"configs\\novo.txt").unwrap())).unwrap(), b"novo");
        assert!(!d3.join(nomes::nome_no_disco(&gbk("cursors\\七夕鼠标.cur")).unwrap()).exists());
        assert!(launcher::verificar(&d3, true, &mut |_| {}).unwrap().is_empty());
        assert!(launcher::atualizar(&serv, &d3, &mut |_| {}).unwrap().is_none());
        assert_eq!(launcher::ler_estado(&d3).unwrap().unwrap().numero, Some(1));
    })
    .await
    .unwrap();

    // Manifesto adulterado: a instalação recusa antes de baixar qualquer pacote.
    let manifesto = atualizacao.join("1").join("manifesto.json");
    let mut m = fs::read(&manifesto).unwrap();
    let i = m.iter().position(|&b| b == b'"').unwrap();
    m.insert(i, b' ');
    fs::write(&manifesto, m).unwrap();
    let destino3 = pasta_limpa("destino3");
    let erro = launcher::instalar(&pela_pasta, LINHA, &destino3, &mut |_| {}).unwrap_err();
    assert!(format!("{erro:#}").contains("SHA-256 do manifesto"), "{erro:#}");

    assert!(repo.apagar_versao_zero(LINHA).await.unwrap());
    for p in [cliente, dados, destino, destino_http, destino3] {
        let _ = fs::remove_dir_all(p);
    }
}
