//! B391: o servidor de atualização anuncia a URL do painel (`GET /painel.json` + `.sig`) e o launcher
//! a confere e troca `{host}` pelo host com que alcança o servidor. Sem banco: só HTTP e assinatura.

use pw_atualizador::{painel, Origem, Servidor};
use pw_recursos::assinatura::Chave;
use pw_recursos::servir;

fn chave_temporaria(nome: &str) -> Chave {
    let pasta = std::env::temp_dir().join(format!("pw_painel_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&pasta);
    Chave::carregar_ou_criar(&pasta.join("k")).unwrap().0
}

async fn subir(rotas: axum::Router) -> String {
    let escuta = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", escuta.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(escuta, rotas).await });
    base
}

#[tokio::test]
async fn launcher_recebe_o_painel_com_o_host_do_servidor() {
    let chave = chave_temporaria("ok");
    let publica = chave.publica_hex();
    let assinado = servir::painel_assinado("http://{host}:8000/", &chave).unwrap();
    let base = subir(servir::rotas_com_painel(std::env::temp_dir(), Some(assinado))).await;
    let base_localhost = base.replace("127.0.0.1", "localhost");

    tokio::task::spawn_blocking(move || {
        let modelo = Servidor::new(Origem::Http(base.clone()), publica.clone()).painel().unwrap().unwrap();
        assert_eq!(modelo, "http://{host}:8000/");
        // O launcher usa o host do endereço configurado: local, rede, DNS.
        assert_eq!(painel::montar_url(&modelo, "127.0.0.1").unwrap(), "http://127.0.0.1:8000/");
        assert_eq!(painel::montar_url(&modelo, "pw.exemplo.com.br").unwrap(), "http://pw.exemplo.com.br:8000/");
        // Pelo nome também responde (o modelo não depende do Host da requisição).
        assert!(Servidor::new(Origem::Http(base_localhost), publica.clone()).painel().unwrap().is_some());

        // Outra chave (servidor falso ou resposta trocada no caminho): recusado.
        let outra = chave_temporaria("outra").publica_hex();
        assert!(Servidor::new(Origem::Http(base), outra).painel().is_err());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn sem_painel_configurado_o_launcher_recebe_none() {
    let chave = chave_temporaria("sem");
    let publica = chave.publica_hex();
    let base = subir(servir::rotas(std::env::temp_dir())).await;
    tokio::task::spawn_blocking(move || {
        assert_eq!(Servidor::new(Origem::Http(base), publica).painel().unwrap(), None);
    })
    .await
    .unwrap();
    assert!(servir::painel_assinado("file:///c:/x.exe", &chave).is_err());
    assert!(servir::painel_assinado("http://{host}.mal.com/", &chave).is_err());
}
