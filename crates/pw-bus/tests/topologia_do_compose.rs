//! Confere a topologia declarada no `docker/docker-compose.yml`.
//!
//! O `pw-bus` traz uma regra de segurança que não é expressável em Rust: **a porta do
//! barramento não pode ser publicada**. Ela não tem autenticação nenhuma — quem alcança
//! o barramento manda `EnterWorld` por qualquer `roleid` e recebe o que é dos outros
//! jogadores. É infraestrutura interna, e só.
//!
//! Uma regra que só existe num comentário é uma regra que volta a ser quebrada na
//! próxima vez que alguém precisar "só depurar rapidinho". Então ela é verificada aqui,
//! contra o arquivo de verdade.
//!
//! O segundo pedaço é a ligação: cada daemon de link precisa saber onde fica o servidor
//! de mundo do seu realm, e o endereço que ele procura tem que ser o que o servidor de
//! mundo de fato escuta. Um `GS_BUS` apontando para uma porta errada não dá erro de
//! compilação nem de subida — dá um jogador que entra no mundo e não vê nada, que é
//! exatamente o sintoma que esta fase existe para resolver.
//!
//! # Sobre a leitura do YAML
//!
//! O arquivo é lido por um varredor de indentação, e não por um parser de YAML — o
//! `pw-bus` não vai ganhar uma dependência de YAML só por causa deste teste. Isso só é
//! honesto porque o arquivo é nosso e tem forma uniforme, então
//! [`servicos`] falha alto se a forma mudar, em vez de devolver um mapa vazio e deixar
//! os testes passarem por vacuidade.

use std::collections::BTreeMap;

const CAMINHO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docker/docker-compose.yml"
);

/// Um serviço do compose, com só o que interessa a este teste.
#[derive(Debug, Default)]
struct Servico {
    /// Qual binário do workspace ele roda (`build.args.CRATE_NAME`).
    crate_name: Option<String>,
    /// As variáveis de ambiente declaradas.
    ambiente: BTreeMap<String, String>,
    /// Os mapeamentos de porta publicados — o que fica alcançável de fora.
    portas: Vec<String>,
}

/// Lê o compose e devolve os serviços indexados pelo nome.
fn servicos() -> BTreeMap<String, Servico> {
    let texto =
        std::fs::read_to_string(CAMINHO).unwrap_or_else(|e| panic!("não consegui ler {CAMINHO}: {e}"));

    let mut mapa: BTreeMap<String, Servico> = BTreeMap::new();
    let mut em_servicos = false;
    let mut atual: Option<String> = None;
    // Em que chave de nível 4 estamos (`environment`, `ports`, `build`...).
    let mut secao = String::new();

    for linha in texto.lines() {
        if linha.trim().is_empty() || linha.trim_start().starts_with('#') {
            continue;
        }
        let indent = linha.len() - linha.trim_start().len();
        let conteudo = linha.trim_end();

        if indent == 0 {
            em_servicos = conteudo.starts_with("services:");
            atual = None;
            continue;
        }
        if !em_servicos {
            continue;
        }

        match indent {
            2 => {
                let nome = conteudo
                    .trim()
                    .trim_end_matches(':')
                    .to_string();
                mapa.insert(nome.clone(), Servico::default());
                atual = Some(nome);
                secao.clear();
            }
            4 => {
                secao = conteudo.trim().split(':').next().unwrap_or("").to_string();
            }
            6 | 8 => {
                let Some(nome) = atual.as_ref() else { continue };
                let s = mapa.get_mut(nome).expect("serviço registrado");
                let t = conteudo.trim();
                match secao.as_str() {
                    "environment" => {
                        if let Some((k, v)) = t.split_once(':') {
                            s.ambiente.insert(
                                k.trim().to_string(),
                                v.trim().trim_matches('"').to_string(),
                            );
                        }
                    }
                    "ports" => {
                        if let Some(p) = t.strip_prefix("- ") {
                            // Descarta o comentário de fim de linha, se houver.
                            let p = p.split('#').next().unwrap_or(p);
                            s.portas.push(p.trim().trim_matches('"').to_string());
                        }
                    }
                    "build" => {
                        if let Some(v) = t.strip_prefix("CRATE_NAME:") {
                            s.crate_name = Some(v.trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // A rede de proteção do varredor: se o arquivo mudar de forma e a leitura devolver
    // lixo, é aqui que se descobre — e não num teste que passa sem verificar nada.
    assert!(
        mapa.len() >= 8,
        "o compose foi lido com só {} serviços — o formato do arquivo mudou e este \
         varredor precisa ser revisto",
        mapa.len()
    );
    assert!(
        mapa.contains_key("pw-postgres"),
        "o compose foi lido sem o `pw-postgres` — leitura quebrada"
    );

    mapa
}

/// Os serviços que rodam um dado binário do workspace.
fn rodando<'a>(mapa: &'a BTreeMap<String, Servico>, bin: &str) -> Vec<(&'a String, &'a Servico)> {
    mapa.iter()
        .filter(|(_, s)| s.crate_name.as_deref() == Some(bin))
        .collect()
}

#[test]
fn a_porta_do_barramento_nunca_e_publicada() {
    // O barramento não autentica nada. Publicá-lo é dar a qualquer um na internet o
    // direito de mandar `EnterWorld` por um `roleid` alheio.
    let mapa = servicos();
    let mundos = rodando(&mapa, "pw-gs");
    assert!(!mundos.is_empty(), "nenhum serviço roda o `pw-gs`");

    for (nome, s) in &mundos {
        assert!(
            s.portas.is_empty(),
            "o serviço de mundo `{nome}` publica portas ({:?}) — o barramento é \
             infraestrutura interna e não pode ser exposto ao jogador",
            s.portas
        );
    }

    // E nenhum outro serviço pode publicar a porta que os mundos escutam, ainda que
    // por engano de cópia.
    let portas_de_barramento: Vec<&str> = mundos
        .iter()
        .filter_map(|(_, s)| s.ambiente.get("BUS_LISTEN"))
        .filter_map(|v| v.rsplit(':').next())
        .collect();

    for (nome, s) in &mapa {
        for p in &s.portas {
            let publicada = p.split(':').next().unwrap_or(p);
            assert!(
                !portas_de_barramento.contains(&publicada),
                "o serviço `{nome}` publica a porta {publicada}, que é a do barramento"
            );
        }
    }
}

#[test]
fn cada_daemon_de_link_aponta_para_um_servidor_de_mundo_que_existe() {
    // Sem isso, o `pw-link` sobe, o cliente entra, e os subcomandos do mundo 3D não vão
    // a lugar nenhum: exatamente o estado anterior a esta fase, só que silencioso.
    let mapa = servicos();
    let links = rodando(&mapa, "pw-link");
    assert!(!links.is_empty(), "nenhum serviço roda o `pw-link`");

    for (nome, s) in &links {
        let gs_bus = s
            .ambiente
            .get("GS_BUS")
            .unwrap_or_else(|| panic!("o link `{nome}` não declara `GS_BUS`"));

        let (host, porta) = gs_bus
            .rsplit_once(':')
            .unwrap_or_else(|| panic!("`GS_BUS` de `{nome}` não tem porta: {gs_bus}"));

        let mundo = mapa
            .get(host)
            .unwrap_or_else(|| panic!("`{nome}` aponta para `{host}`, que não é um serviço"));

        assert_eq!(
            mundo.crate_name.as_deref(),
            Some("pw-gs"),
            "`{nome}` aponta para `{host}`, que não roda o servidor de mundo"
        );

        // O endereço que o link procura tem que ser o que o mundo de fato escuta.
        let escuta = mundo
            .ambiente
            .get("BUS_LISTEN")
            .unwrap_or_else(|| panic!("o mundo `{host}` não declara `BUS_LISTEN`"));
        let porta_escutada = escuta.rsplit(':').next().unwrap();
        assert_eq!(
            porta, porta_escutada,
            "`{nome}` procura `{host}:{porta}`, mas `{host}` escuta em `{escuta}`"
        );

        // E o mundo tem que escutar em todas as interfaces: `127.0.0.1` funcionaria no
        // teste local e falharia dentro do compose, onde a conexão vem de outro
        // contêiner.
        assert!(
            escuta.starts_with("0.0.0.0:") || escuta.starts_with("[::]:"),
            "o mundo `{host}` escuta em `{escuta}` — de dentro do contêiner isso não \
             aceita conexão do daemon de link"
        );
    }
}

#[test]
fn link_e_mundo_do_mesmo_realm_combinam_de_realm_e_versao() {
    // Um link 1.2.6 falando com um mundo 1.5.3 seria um erro de configuração que só
    // aparece como comportamento estranho em jogo, muito depois.
    let mapa = servicos();

    for (nome, s) in rodando(&mapa, "pw-link") {
        let gs_bus = s.ambiente.get("GS_BUS").expect("`GS_BUS` (ver teste acima)");
        let host = gs_bus.rsplit_once(':').unwrap().0;
        let mundo = &mapa[host];

        for chave in ["REALM_ID", "GAME_VERSION"] {
            assert_eq!(
                s.ambiente.get(chave),
                mundo.ambiente.get(chave),
                "`{nome}` e `{host}` discordam em `{chave}`"
            );
        }
    }
}

#[test]
fn todo_realm_tem_exatamente_um_servidor_de_mundo() {
    let mapa = servicos();
    let links = rodando(&mapa, "pw-link").len();
    let mundos = rodando(&mapa, "pw-gs").len();
    assert_eq!(
        links, mundos,
        "há {links} daemons de link e {mundos} servidores de mundo — algum realm ficou \
         sem mundo, ou sobrou um mundo sem link"
    );
}
