//! Estúdio S1 de ponta a ponta (B237), com o `elements.data` real do 1.2.6: listar, ficha,
//! editar (o resumo guarda o "antes" do publicado), clonar com id do espaço, remover recusado
//! por referência, desfazer tudo volta ao publicado, publicar grava no realm; o npcgen do mapa
//! editado num rascunho só do servidor, publicado e revertido (B243); distritos e portais (B244); teleportes (B245); rastreador de NPCs do cliente (B246); controladores (B247); altura do chão (B248); distritos e portais em par com o .clt do cliente (B251); destinos em par com o world_targets.txt (B252); alinhar o servidor com o cliente (B253); Loja Gold (B254); missões (B255) e editar missão no lugar (B256); comportamento (B258) e editá-lo no lugar (B259); textos do cliente (B260).

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

/// `precinct.clt`/`region.clt` alinhados com os `.sev` da a01 do 1.2.6 (B251): mesmo carimbo, mesma contagem e
/// sequência, gerados com o gravador do cliente (os reais estão fora do repositório).
fn clt_alinhados() -> (Vec<u8>, Vec<u8>) {
    use pw_data_loader::clt::{DistritoDoCliente, PrecinctDoCliente, RegionDoCliente, RegistroDoCliente};
    use pw_data_loader::regioes::{RegioesEditavel, RegistroDeRegiao};
    let a01 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/a01");
    let d = pw_data_loader::precinct::Distritos::ler(&fs::read(a01.join("precinct.sev")).unwrap()).unwrap();
    let p = PrecinctDoCliente {
        cabecalho: vec!["//  Element pricinct file (client version)".into(), "".into(), format!("version  {}", d.versao)],
        versao: d.versao,
        carimbo: d.carimbo,
        distritos: d.lista.iter().enumerate().map(|(i, x)| DistritoDoCliente {
            nome: format!("D{i}"),
            numeros: vec![100 + i as i64, x.vertices.len() as i64, 0, x.prioridade as i64, x.mapa_do_ponto as i64, 0, 60, 1, x.mapa as i64, x.dominio as i64],
            ponto_de_cidade: x.ponto_de_cidade,
            vertices: x.vertices.clone(),
            marcas: vec![],
            som: String::new(),
            musicas: vec![],
            som_da_noite: Some(String::new()),
        }).collect(),
    };
    let r = RegioesEditavel::ler(&fs::read(a01.join("region.sev")).unwrap()).unwrap();
    let c = RegionDoCliente {
        cabecalho: vec!["//  Element region file (client version)".into(), "".into(), format!("version  {}", r.versao)],
        versao: r.versao,
        carimbo: r.carimbo,
        registros: r.registros.iter().map(|x| match x {
            RegistroDeRegiao::Regiao(v) => RegistroDoCliente::Regiao { nome: "安全区".into(), tipo: 1, vertices: v.clone() },
            RegistroDeRegiao::Caixa(k) => RegistroDoCliente::Portal {
                numeros: vec![k.destino as i64, k.origem as i64, k.nivel_minimo as i64], pos: k.pos, exts: k.exts, alvo: k.alvo,
            },
        }).collect(),
    };
    (p.gravar(), c.gravar())
}

struct Pacotes(Vec<u8>);

impl FonteDePacote for Pacotes {
    fn ler(&self, _pck: &Path, receber: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>) -> Result<ResultadoDoPacote> {
        receber(b"data\\elements.data".to_vec(), self.0.clone())?;
        // Mapas (B242): o instance.txt (formato do 1.2.6) e uma peça DXT1 4×4 do minimapa.
        receber(b"configs\\instance.txt".to_vec(), "\"world\"\r\n{\r\n1\t\t// ID\r\n\"world\"\r\n1, 1\r\n}\r\n".as_bytes().to_vec())?;
        let mut dds = vec![0u8; 128];
        dds[..4].copy_from_slice(b"DDS ");
        dds[12..16].copy_from_slice(&4u32.to_le_bytes());
        dds[16..20].copy_from_slice(&4u32.to_le_bytes());
        dds[84..88].copy_from_slice(b"DXT1");
        dds.extend([0x00, 0xF8, 0x1F, 0x00, 0, 0, 0, 0]);
        receber(b"surfaces\\minimaps\\world\\0000.dds".to_vec(), dds)?;
        // Rastreador de NPCs do cliente (B246): o task_npc.data real do 1.2.6.
        let tn = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/task_npc.data");
        receber(b"data\\task_npc.data".to_vec(), fs::read(tn).expect("task_npc do realm_126"))?;
        let (precinct, region) = clt_alinhados();
        receber(b"maps\\world\\precinct.clt".to_vec(), precinct)?;
        receber(b"maps\\world\\region.clt".to_vec(), region)?;
        // world_targets.txt alinhado com o .sev do 1.2.6 (B252): os mesmos ids, na mesma ordem.
        let wt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/world_targets.sev");
        let w = pw_data_loader::world_targets::PontosDoMundo::load_from_bytes(&fs::read(wt).unwrap()).unwrap();
        let txt = pw_data_loader::world_targets::TxtDosDestinos {
            destinos: w.lista.iter().map(|p| pw_data_loader::world_targets::DestinoDoCliente {
                id: p.id, nome: format!("T{}", p.id), mundo: p.mundo, pos: p.pos, dominio: p.dominio,
            }).collect(),
        };
        receber(b"maps\\world\\world_targets.txt".to_vec(), txt.gravar())?;
        // A Loja Gold real do 1.2.6 (B254): o cliente e o realm usam o mesmo arquivo.
        receber(b"data\\gshop.data".to_vec(), fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/gshop.data")).unwrap())?;
        // tasks.data (B255/B256): o cliente e o realm usam o mesmo arquivo.
        receber(b"data\\tasks.data".to_vec(), fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/tasks.data")).unwrap())?;
        // Tabelas de texto (B260): uma indexada e uma sequencial, no formato do cliente.
        let utf16 = |t: &str| { let mut b = vec![0xFFu8, 0xFE]; t.encode_utf16().for_each(|c| b.extend_from_slice(&c.to_le_bytes())); b };
        receber(b"configs\\item_ext_desc.txt".to_vec(), utf16("//  Element item extend descriptions.\n\n#_index\n#_begin\n\n10037  \"^ffcb4aUma invenção\"\n10042  \"Outra\"\n"))?;
        receber(b"configs\\fixed_msg.txt".to_vec(), utf16("//  Element fixed notify messages.\n\n#_begin\n\n\"Ganhou %d moedas\"\n\"Ganhou %d de experiência\"\n"))?;
        // Um .ski mínimo (B272): MOXB + cabeçalho v9 (1 malha de pele, 1 textura, 1 material), um triângulo.
        let ski = {
            let mut b = b"MOXB".to_vec();
            let u = |b: &mut Vec<u8>, x: u32| b.extend_from_slice(&x.to_le_bytes());
            let f = |b: &mut Vec<u8>, x: f32| b.extend_from_slice(&x.to_le_bytes());
            u(&mut b, u32::from_be_bytes(*b"ASKI")); u(&mut b, 9);
            for x in [1u32, 0, 0, 0, 1, 1, 0] { u(&mut b, x); }
            f(&mut b, 0.0); u(&mut b, 0); u(&mut b, 0); u(&mut b, 0);
            b.extend_from_slice(&[0u8; 52]);
            u(&mut b, 9); b.extend_from_slice(b"caixa.dds");
            b.extend_from_slice(b"MATERIAL: madeira\0");
            for _ in 0..16 { f(&mut b, 0.5); }
            f(&mut b, 10.0); b.push(1);
            u(&mut b, 5); b.extend_from_slice(b"tampa");
            for x in [0u32, 0, 3, 3] { u(&mut b, x); }
            for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
                for x in p { f(&mut b, x); }
                for _ in 0..3 { f(&mut b, 0.0); }
                u(&mut b, 0);
                for x in [0.0f32, 0.0, -1.0, 0.0, 0.0] { f(&mut b, x); }
            }
            for i in [0u16, 1, 2] { b.extend_from_slice(&i.to_le_bytes()); }
            b
        };
        receber(b"models\\teste\\caixa.ski".to_vec(), ski)?;
        // A textura (B273): DDS DXT1 4×4 vermelho, na pasta `textures` (minúscula, como no cliente).
        let mut dds = vec![0u8; 128];
        dds[0..4].copy_from_slice(b"DDS ");
        dds[12..16].copy_from_slice(&4u32.to_le_bytes());
        dds[16..20].copy_from_slice(&4u32.to_le_bytes());
        dds[80..84].copy_from_slice(&4u32.to_le_bytes());
        dds[84..88].copy_from_slice(b"DXT1");
        dds.extend_from_slice(&[0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0]);
        receber(b"models\\teste\\textures\\caixa.dds".to_vec(), dds)?;
        // Esqueleto e modelo (B274): .bon v6 com um osso (matrizes identidade) e .smd ligando-o ao caixa.ski.
        let u = |b: &mut Vec<u8>, x: u32| b.extend_from_slice(&x.to_le_bytes());
        let ident = |b: &mut Vec<u8>| for k in 0..16 { b.extend_from_slice(&(if k % 5 == 0 { 1.0f32 } else { 0.0 }).to_le_bytes()); };
        let mut bon = b"MOXB".to_vec();
        u(&mut bon, u32::from_be_bytes(*b"ASKE")); u(&mut bon, 6);
        for x in [1u32, 0, 0, 0, 0, 30] { u(&mut bon, x); }
        bon.extend_from_slice(&[0u8; 64]);
        u(&mut bon, 4); bon.extend_from_slice(b"raiz");
        bon.extend_from_slice(&[0u8; 4]); u(&mut bon, u32::MAX); u(&mut bon, u32::MAX); u(&mut bon, 0);
        ident(&mut bon); ident(&mut bon);
        receber(b"models\\teste\\caixa.bon".to_vec(), bon)?;
        let mut smd = b"MOXB".to_vec();
        u(&mut smd, u32::from_be_bytes(*b"ASMD")); u(&mut smd, 8);
        for x in [1u32, 0, 0, 0] { u(&mut smd, x); }
        smd.extend_from_slice(&[0u8; 60]);
        u(&mut smd, 9); smd.extend_from_slice(b"caixa.bon");
        u(&mut smd, 9); smd.extend_from_slice(b"caixa.ski");
        receber(b"models\\teste\\caixa.smd".to_vec(), smd)?;
        Ok(ResultadoDoPacote { entradas: 15, falhas: vec![] })
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
    // npcgen real da instância a01 do 1.2.6, posto como o do "world" deste teste (B242).
    let npcgen = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/a01/npcgen.data");
    fs::create_dir_all(realm.join("world")).unwrap();
    fs::copy(&npcgen, realm.join("world").join("npcgen.data")).expect("npcgen da a01 do realm_126");
    // precinct.sev (v6, 1 distrito) e region.sev (v5, 1 região, 1 portal) da mesma a01 (B244).
    for f in ["precinct.sev", "region.sev"] {
        fs::copy(npcgen.with_file_name(f), realm.join("world").join(f)).expect("sev da a01 do realm_126");
    }
    // aipolicy.data real do 1.2.6 (B258).
    fs::copy(npcgen.parent().unwrap().parent().unwrap().join("aipolicy.data"), realm.join("aipolicy.data")).expect("aipolicy do realm_126");
    // tasks.data real do 1.2.6, na raiz do config (B255).
    fs::copy(npcgen.parent().unwrap().parent().unwrap().join("tasks.data"), realm.join("tasks.data")).expect("tasks do realm_126");
    // world_targets.sev real do 1.2.6, na raiz do config (B245).
    fs::copy(npcgen.parent().unwrap().parent().unwrap().join("world_targets.sev"), realm.join("world_targets.sev")).expect("world_targets do realm_126");
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

        // Mapas (B242): lista do instance.txt, peça em PNG, criaturas do npcgen do realm com nome.
        let mapas = json_de(pedir("GET", "estudio/mapas").call().unwrap());
        assert_eq!(mapas, json!([{"nome": "world", "id": 1, "caminho": "world", "linhas": 1, "colunas": 1, "npcgen": true}]));
        let mut png = Vec::new();
        std::io::Read::read_to_end(&mut pedir("GET", "estudio/mapas/world/pecas/0000.png").call().unwrap().into_reader(), &mut png).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        assert_eq!(erro_de(pedir("GET", "estudio/mapas/world/pecas/9999.png").call()).0, 404);
        assert_eq!(erro_de(pedir("GET", "estudio/mapas/..%2Fx/pecas/0000.png").call()).0, 404);
        let cr = json_de(pedir("GET", "estudio/mapas/world/criaturas").call().unwrap());
        let pontos = cr["pontos"].as_array().unwrap();
        assert!(!pontos.is_empty());
        let com_nome = pontos.iter().filter(|p| cr["modelos"][p[1].to_string()]["nome"].is_string()).count();
        assert!(com_nome * 10 >= pontos.len() * 9, "{com_nome} de {} com nome", pontos.len());

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

        // Conversas (TALK_PROC, B240).
        let t = json_de(pedir("GET", "estudio/tabelas").call().unwrap());
        let talk = t["tabelas"].as_array().unwrap().iter().find(|x| x["nome"] == "TALK_PROC").unwrap().clone();
        assert!(talk["registros"].as_u64().unwrap() > 100 && talk["conversas"] == true);
        // O serviço de conversa cita a conversa pelo id_dialog, com o texto dela.
        let servico = json_de(pedir("GET", "estudio/tabelas/NPC_TALK_SERVICE/0").call().unwrap());
        let dialogo = servico["campos"].as_array().unwrap().iter().find(|c| c["nome"] == "id_dialog").unwrap().clone();
        let r_dialogo = dialogo["referencia"].clone();
        assert_eq!((dialogo["cita"].clone(), r_dialogo["tabela"].clone()), (json!("TALK"), json!("TALK_PROC")), "{dialogo}");
        let ic = r_dialogo["indice"].as_u64().unwrap();
        let conv = json_de(pedir("GET", &format!("estudio/conversas/{ic}")).call().unwrap());
        assert!(!conv["janelas"].as_array().unwrap().is_empty());
        assert!(!conv["usado_por"].as_array().unwrap().is_empty(), "o serviço a usa");
        // Salvar igual: nada muda.
        let igual = json_de(pedir("PUT", &em(&format!("estudio/conversas/{ic}"))).send_json(json!({"conversa": conv})).unwrap());
        assert_eq!((igual["resultado"]["mudou"].clone(), igual["igual_ao_publicado"].clone()), (json!(false), json!(true)));
        // Editar o texto da 1ª janela; opção para janela inexistente é recusada.
        let mut nova = conv.clone();
        nova["janelas"][0]["texto"] = json!("Texto do Estúdio");
        pedir("PUT", &em(&format!("estudio/conversas/{ic}"))).send_json(json!({"conversa": nova})).unwrap();
        let relida = json_de(pedir("GET", &em(&format!("estudio/conversas/{ic}"))).call().unwrap());
        assert_eq!(relida["janelas"][0]["texto"], "Texto do Estúdio");
        let mut ruim = relida.clone();
        ruim["janelas"][0]["opcoes"] = json!([{"id": 99999, "texto": "x", "param": 0}]);
        assert_eq!(erro_de(pedir("PUT", &em(&format!("estudio/conversas/{ic}"))).send_json(json!({"conversa": ruim}))).0, 400);
        // Remover a conversa em uso é recusado; uma nova (id = maior + 1) some ao ser removida.
        assert_eq!(erro_de(pedir("DELETE", &em(&format!("estudio/conversas/{ic}"))).call()).0, 400);
        let criada = json_de(pedir("POST", &em("estudio/conversas")).call().unwrap())["resultado"].clone();
        let lista_c = json_de(pedir("GET", &em("estudio/conversas")).query("busca", &criada["id"].to_string()).call().unwrap());
        assert_eq!(lista_c["total"], 1);
        pedir("DELETE", &em(&format!("estudio/conversas/{}", criada["indice"]))).call().unwrap();
        let funcoes = json_de(pedir("GET", "estudio/funcoes").call().unwrap());
        assert!(funcoes.as_array().unwrap().iter().any(|f| f["nome"] == "TALK_EXIT" && f["codigo"] == 0x8000_0012u32));

        // Editar de novo e publicar: o cliente ganha a versão 1 e o realm o arquivo novo.
        pedir("PUT", &em("estudio/tabelas/WEAPON_ESSENCE/0")).send_json(json!({"campos": {"price": 333}})).unwrap();
        let p = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r]})).unwrap());
        assert_eq!(p["numero"], 1);
        assert_eq!(p["aplicados"].as_array().unwrap().len(), 1, "{p}");
        let no_realm = fs::read(realm2.join("elements.data")).unwrap();
        assert_ne!(no_realm, original2);
        // O tamanho muda com o texto da conversa (B240); o arquivo tem de continuar fechando.
        assert_ne!(no_realm.len(), original2.len());
        let lido = load_elements_data(&no_realm).unwrap();
        assert_eq!(lido.get("WEAPON_ESSENCE")[0]["price"], FieldValue::Int(333));
        let conversas = pw_data_loader::elements_editavel::ElementsEditavel::ler(&no_realm, None).unwrap().conversas;
        assert_eq!(pw_data_loader::conversas::texto(&conversas[ic as usize].janelas[0].texto), "Texto do Estúdio");
        // O publicado agora é o novo: a ficha sem rascunho mostra 333.
        assert_eq!(valor(&json_de(pedir("GET", "estudio/tabelas/WEAPON_ESSENCE/0").call().unwrap()), "price"), json!(333));

        // Edição do npcgen (B243): rascunho só do servidor, publicar grava no realm, reverter devolve.
        let npcgen_original = fs::read(realm2.join("world").join("npcgen.data")).unwrap();
        let r2 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "npcgen", "operacao_id": "t_p4_r2"})).unwrap())["id"].as_i64().unwrap();
        let em2 = |rel: &str| format!("{rel}?rascunho={r2}");
        let ng = json_de(pedir("GET", "estudio/mapas/world/npcgen").call().unwrap());
        assert_eq!((ng["item"].as_str(), ng["no_rascunho"].as_bool()), (Some("servidor\\world\\npcgen.data"), Some(false)));
        let areas = ng["areas"].as_array().unwrap().clone();
        let n_areas = areas.len();
        assert!(n_areas >= 2, "a01 tem áreas");
        let a0 = json_de(pedir("GET", "estudio/mapas/world/npcgen/areas/0").call().unwrap());
        let x0 = a0["campos"]["vPos_x"].as_f64().unwrap();
        let tid0 = a0["filhos"][0]["dwID"].as_u64().unwrap();
        assert_eq!(areas[0][6].as_u64(), Some(tid0));
        // Sem rascunho é recusado; campo inexistente e tipo de área inválido também.
        assert_eq!(erro_de(pedir("PUT", "estudio/mapas/world/npcgen/areas/0").send_json(json!({"campos": {"vPos_x": 1.0}}))).0, 400);
        assert_eq!(erro_de(pedir("PUT", &em2("estudio/mapas/world/npcgen/areas/0")).send_json(json!({"campos": {"nada": 1}}))).0, 400);
        assert_eq!(erro_de(pedir("GET", "estudio/mapas/world/npcgen/xyz/0").call()).0, 404);
        assert_eq!(erro_de(pedir("PUT", &em2("estudio/mapas/world/npcgen/areas/0")).send_json(json!({"filhos": []}))).0, 400);
        // Mover a área 0, pôr 2 geradores (o 2º copia o último e troca a quantidade), clonar e remover.
        pedir("PUT", &em2("estudio/mapas/world/npcgen/areas/0"))
            .send_json(json!({"campos": {"vPos_x": x0 + 10.0}, "filhos": [{}, {"dwNum": 3}]}))
            .unwrap();
        let clone = json_de(pedir("POST", &em2("estudio/mapas/world/npcgen/areas/1/clonar")).call().unwrap())["resultado"]["indice"].as_u64().unwrap();
        assert_eq!(clone as usize, n_areas);
        pedir("DELETE", &em2(&format!("estudio/mapas/world/npcgen/areas/{}", n_areas - 1))).call().unwrap();
        let ng2 = json_de(pedir("GET", &em2("estudio/mapas/world/npcgen")).call().unwrap());
        assert_eq!((ng2["no_rascunho"].as_bool(), ng2["areas"].as_array().unwrap().len()), (Some(true), n_areas));
        let a0b = json_de(pedir("GET", &em2("estudio/mapas/world/npcgen/areas/0")).call().unwrap());
        assert_eq!(a0b["filhos"].as_array().unwrap().len(), 2);
        assert_eq!((a0b["filhos"][1]["dwID"].as_u64(), a0b["filhos"][1]["dwNum"].as_u64()), (Some(tid0), Some(3)));
        assert_eq!(fs::read(realm2.join("world").join("npcgen.data")).unwrap(), npcgen_original, "o realm só muda ao publicar");
        let hist = json_de(pedir("GET", "alteracoes").call().unwrap());
        let item = hist.as_array().unwrap().iter().find(|a| a["id"] == r2).unwrap()["lista"][0].clone();
        assert_eq!((item["caminho"].as_str(), item["destino"].as_str()), (Some("servidor\\world\\npcgen.data"), Some("servidor")));
        assert_eq!(item["resumo"]["mudancas"].as_array().unwrap().len(), 3, "{item}");
        // "Aguarda reinício" (B249): a publicação do elements (v1, destino ambos) já conta como arquivo do servidor.
        let u1 = json_de(pedir("GET", "servidor/ultima").call().unwrap());
        assert_eq!(u1["numero"], 1, "{u1}");
        // Publicar: o realm ganha o arquivo novo, que o carregador do GS lê.
        let p2 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r2]})).unwrap());
        assert_eq!((p2["numero"].as_i64(), p2["aplicados"].as_array().unwrap().len()), (Some(2), 1), "{p2}");
        let u2 = json_de(pedir("GET", "servidor/ultima").call().unwrap());
        assert!(u2["numero"] == 2 && u2["publicada_em"].as_i64().unwrap() >= u1["publicada_em"].as_i64().unwrap(), "{u2}");
        let novo = fs::read(realm2.join("world").join("npcgen.data")).unwrap();
        assert_ne!(novo, npcgen_original);
        let ed = pw_data_loader::npcgen_editavel::NpcgenEditavel::ler(&novo).unwrap();
        assert_eq!((ed.areas.len(), ed.areas[0].geradores.len()), (n_areas, 2));
        pw_data_loader::npcgen::NpcGenData::load_from_bytes(&novo).unwrap();
        // Reverter: o rascunho devolve o arquivo antigo do realm.
        let rv = json_de(pedir("POST", &format!("alteracoes/{r2}/reverter"))
            .send_json(json!({"operacao_id": "t_p4_r3"})).unwrap())["rascunho"].as_i64().unwrap();
        pedir("POST", "publicar").send_json(json!({"rascunhos": [rv]})).unwrap();
        assert_eq!(fs::read(realm2.join("world").join("npcgen.data")).unwrap(), npcgen_original, "reverter devolve o npcgen");

        // Distritos e portais (B244): só ponto de renascer / chegada; carimbo intacto; publicar grava os dois.
        let sev = |f: &str| fs::read(realm2.join("world").join(f)).unwrap();
        let (precinct0, region0) = (sev("precinct.sev"), sev("region.sev"));
        let r4 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "distritos", "operacao_id": "t_p4_r4"})).unwrap())["id"].as_i64().unwrap();
        let em4 = |rel: &str| format!("{rel}?rascunho={r4}");
        let d = json_de(pedir("GET", "estudio/mapas/world/distritos").call().unwrap());
        assert_eq!((d["item"].as_str(), d["versao_do_arquivo"].as_u64(), d["distritos"].as_array().unwrap().len()),
            (Some("servidor\\world\\precinct.sev"), Some(6), 1), "{d}");
        assert!(d["distritos"][0]["vertices"].as_array().unwrap().len() >= 3);
        let pt = json_de(pedir("GET", "estudio/mapas/world/portais").call().unwrap());
        assert_eq!((pt["caixas"].as_array().unwrap().len(), pt["regioes"].as_array().unwrap().len()), (1, 1), "{pt}");
        assert_eq!(erro_de(pedir("PUT", &em4("estudio/mapas/world/distritos/0")).send_json(json!({"campos": {"protege_pk": true}}))).0, 400, "v6 não tem PK");
        // Com o region.clt alinhado (B251) o destino é editável; mapa 0 continua recusado.
        assert_eq!(erro_de(pedir("PUT", &em4("estudio/mapas/world/portais/0")).send_json(json!({"campos": {"destino": 0}}))).0, 400);
        assert_eq!(erro_de(pedir("PUT", &em4("estudio/mapas/world/distritos/0")).send_json(json!({"campos": {"ponto": [1, 2]}}))).0, 400);
        pedir("PUT", &em4("estudio/mapas/world/distritos/0")).send_json(json!({"campos": {"ponto": [100.5, 20.0, -30.25], "prioridade": 3}})).unwrap();
        pedir("PUT", &em4("estudio/mapas/world/portais/0")).send_json(json!({"campos": {"alvo": [1.5, 2.5, 3.5]}})).unwrap();
        let d2 = json_de(pedir("GET", &em4("estudio/mapas/world/distritos")).call().unwrap());
        assert_eq!((d2["distritos"][0]["ponto"].clone(), d2["distritos"][0]["prioridade"].clone()), (json!([100.5, 20.0, -30.25]), json!(3)));
        let p4 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r4]})).unwrap());
        assert_eq!(p4["aplicados"].as_array().unwrap().len(), 2, "{p4}");
        let (precinct1, region1) = (sev("precinct.sev"), sev("region.sev"));
        assert_eq!((precinct1.len(), region1.len()), (precinct0.len(), region0.len()));
        assert_eq!(precinct1[8..12], precinct0[8..12], "carimbo do precinct intacto");
        assert_eq!(region1[12..16], region0[12..16], "carimbo do region intacto");
        let lido = pw_data_loader::precinct::Distritos::ler(&precinct1).unwrap();
        assert_eq!((lido.lista[0].ponto_de_cidade, lido.lista[0].prioridade), ([100.5, 20.0, -30.25], 3));
        assert_eq!(pw_data_loader::regioes::ler_caixas_de_transporte(&region1).unwrap()[0].alvo, [1.5, 2.5, 3.5]);

        // Teleportes (B245): destinos do mapa 1 com as transportadoras do elements; editar a posição de um.
        let tp = json_de(pedir("GET", "estudio/mapas/world/teleportes").call().unwrap());
        let pontos = tp["pontos"].as_array().unwrap().clone();
        assert_eq!((tp["item"].as_str(), tp["mapa"].as_i64()), (Some("servidor\\world_targets.sev"), Some(1)));
        assert!(pontos.len() > 5 && pontos.iter().all(|p| p["id"].as_i64().is_some()), "{}", pontos.len()); // no 1.2.6 os 49 ficam todos no mapa 1
        let com_npc = pontos.iter().filter(|p| p["usado_por"].as_array().unwrap().iter().any(|u| !u["npcs"].as_array().unwrap().is_empty())).count();
        assert!(com_npc * 2 >= pontos.len(), "{com_npc} de {} com NPC que leva", pontos.len());
        let id = pontos[0]["id"].as_i64().unwrap();
        let wt0 = fs::read(realm2.join("world_targets.sev")).unwrap();
        let r5 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "teleporte", "operacao_id": "t_p4_r5"})).unwrap())["id"].as_i64().unwrap();
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/teleportes/{id}?rascunho={r5}")).send_json(json!({"campos": {"mundo": 2}}))).0, 400);
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/teleportes/999999?rascunho={r5}")).send_json(json!({"campos": {"pos": [1, 2, 3]}}))).0, 400);
        pedir("PUT", &format!("estudio/teleportes/{id}?rascunho={r5}")).send_json(json!({"campos": {"pos": [10.0, 20.0, 30.0]}})).unwrap();
        let tp2 = json_de(pedir("GET", &format!("estudio/mapas/world/teleportes?rascunho={r5}")).call().unwrap());
        assert_eq!((tp2["no_rascunho"].as_bool(), tp2["pontos"][0]["pos"].clone()), (Some(true), json!([10.0, 20.0, 30.0])));
        let p5 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r5]})).unwrap());
        assert_eq!(p5["aplicados"].as_array().unwrap().len(), 1, "{p5}");
        let wt1 = fs::read(realm2.join("world_targets.sev")).unwrap();
        assert_eq!(wt1.len(), wt0.len());
        let lido = pw_data_loader::world_targets::PontosDoMundo::load_from_bytes(&wt1).unwrap();
        assert_eq!(lido.get(id as i32).unwrap().pos, [10.0, 20.0, 30.0]);

        // Rastreador (B246): NPCs das áreas de NPC do npcgen com a posição do task_npc.data do cliente; editar
        // gera item do cliente (destino "cliente") que entra no manifesto da versão publicada.
        let rs = json_de(pedir("GET", "estudio/mapas/world/rastreador").call().unwrap());
        assert_eq!((rs["item"].as_str(), rs["total"].as_u64()), (Some("data\\task_npc.data"), Some(765)));
        let npcs = rs["npcs"].as_array().unwrap().clone();
        assert!(!npcs.is_empty(), "a a01 tem NPCs");
        let id_npc = npcs[0]["id"].as_u64().unwrap();
        // NPC que o rastreador não tinha é acrescentado no fim.
        let original_tn = pw_data_loader::task_npc::Rastreador::ler(&fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/realm_126/config/task_npc.data")).unwrap()).unwrap();
        let esperado = 765 + !original_tn.npcs.iter().any(|n| n.id as u64 == id_npc) as usize;
        assert_eq!(rs["usado_pelo_cliente"], true, "o world do teste é o mapa 1");
        let r6 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "rastreador", "operacao_id": "t_p4_r6"})).unwrap())["id"].as_i64().unwrap();
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/rastreador/{id_npc}?rascunho={r6}")).send_json(json!({"campos": {"pos": [40000, 0, 0]}}))).0, 400);
        pedir("PUT", &format!("estudio/rastreador/{id_npc}?rascunho={r6}")).send_json(json!({"campos": {"pos": [100.4, 5.0, -20.6]}})).unwrap();
        let rs2 = json_de(pedir("GET", &format!("estudio/mapas/world/rastreador?rascunho={r6}")).call().unwrap());
        let meu = rs2["npcs"].as_array().unwrap().iter().find(|n| n["id"].as_u64() == Some(id_npc)).unwrap().clone();
        assert_eq!((rs2["no_rascunho"].as_bool(), meu["rastreador"].clone()), (Some(true), json!([100, 5, -21])));
        let hist = json_de(pedir("GET", "alteracoes").call().unwrap());
        let item = hist.as_array().unwrap().iter().find(|a| a["id"] == r6).unwrap()["lista"][0].clone();
        assert_eq!((item["caminho"].as_str(), item["destino"].as_str()), (Some("data\\task_npc.data"), Some("cliente")));
        let p6 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r6]})).unwrap());
        assert_eq!(p6["aplicados"].as_array().unwrap().len(), 0, "item do cliente não vai para o realm: {p6}");
        let novo = pedir("GET", "arquivo").query("caminho", "data\\task_npc.data").call().unwrap();
        let mut b = Vec::new();
        std::io::Read::read_to_end(&mut novo.into_reader(), &mut b).unwrap();
        let lido = pw_data_loader::task_npc::Rastreador::ler(&b).unwrap();
        assert_eq!((lido.posicao(id_npc as u32), lido.npcs.len()), (Some([100, 5, -21]), esperado));

        // Controladores e objetos dinâmicos do npcgen (B247): a a01 do 1.2.6 tem 346 controladores, as áreas citam o 456,
        // e nenhum objeto.
        let ng = json_de(pedir("GET", "estudio/mapas/world/npcgen").call().unwrap());
        let ctrls = ng["controladores"].as_array().unwrap().clone();
        assert_eq!((ctrls.len(), ng["objetos"].as_array().unwrap().len()), (346, 0));
        let citado = ctrls.iter().find(|c| c[1] == 456).expect("controlador 456").clone();
        assert!(citado[5].as_u64().unwrap() > 0, "{citado}");
        let ic = citado[0].as_u64().unwrap();
        let ficha = json_de(pedir("GET", &format!("estudio/mapas/world/npcgen/controladores/{ic}")).call().unwrap());
        assert_eq!(ficha["campos"]["id"], 456);
        assert!(!ficha["usado_por"].as_array().unwrap().is_empty());
        assert_eq!(erro_de(pedir("GET", "estudio/mapas/world/npcgen/objetos/0").call()).0, 400);
        let r7 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "controladores", "operacao_id": "t_p4_r7"})).unwrap())["id"].as_i64().unwrap();
        let em7 = |rel: &str| format!("{rel}?rascunho={r7}");
        let (s, msg) = erro_de(pedir("DELETE", &em7(&format!("estudio/mapas/world/npcgen/controladores/{ic}"))).call());
        assert!(s == 400 && msg.contains("citado"), "{s} {msg}");
        assert_eq!(erro_de(pedir("PUT", &em7(&format!("estudio/mapas/world/npcgen/controladores/{ic}"))).send_json(json!({"campos": {"id": 9}}))).0, 400);
        assert_eq!(erro_de(pedir("PUT", &em7(&format!("estudio/mapas/world/npcgen/controladores/{ic}"))).send_json(json!({"filhos": [{}]}))).0, 400);
        let ativo = ficha["campos"]["bActived"].as_bool().unwrap();
        pedir("PUT", &em7(&format!("estudio/mapas/world/npcgen/controladores/{ic}")))
            .send_json(json!({"campos": {"bActived": !ativo, "ActiveTime_iHours": 5}})).unwrap();
        let f2 = json_de(pedir("GET", &em7(&format!("estudio/mapas/world/npcgen/controladores/{ic}"))).call().unwrap());
        assert_eq!((f2["campos"]["bActived"].as_bool(), f2["campos"]["ActiveTime_iHours"].as_i64()), (Some(!ativo), Some(5)));
        let maior = ctrls.iter().map(|c| c[1].as_i64().unwrap()).max().unwrap();
        let clone = json_de(pedir("POST", &em7(&format!("estudio/mapas/world/npcgen/controladores/{ic}/clonar"))).call().unwrap())["resultado"]["indice"].as_u64().unwrap();
        let fc = json_de(pedir("GET", &em7(&format!("estudio/mapas/world/npcgen/controladores/{clone}"))).call().unwrap());
        assert_eq!(fc["campos"]["id"].as_i64(), Some(maior + 1), "o clone ganha id novo");
        pedir("DELETE", &em7(&format!("estudio/mapas/world/npcgen/controladores/{clone}"))).call().unwrap();
        let p7 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r7]})).unwrap());
        assert_eq!(p7["aplicados"].as_array().unwrap().len(), 1, "{p7}");
        let ed = pw_data_loader::npcgen_editavel::NpcgenEditavel::ler(&fs::read(realm2.join("world").join("npcgen.data")).unwrap()).unwrap();
        let c = pw_data_loader::npcgen_editavel::ler_campos(&ed.controladores[ic as usize], pw_data_loader::npcgen_editavel::CAMPOS_DO_CONTROLADOR, ed.versao);
        assert_eq!((c["bActived"].as_bool(), c["ActiveTime_iHours"].as_i64(), ed.controladores.len()), (Some(!ativo), Some(5), 346));
        pw_data_loader::npcgen::NpcGenData::load_from_bytes(&fs::read(realm2.join("world").join("npcgen.data")).unwrap()).unwrap();

        // Destinos de teleporte em par com o world_targets.txt (B252): renomear, criar, remover recusado se usado.
        let tp0 = json_de(pedir("GET", "estudio/mapas/world/teleportes").call().unwrap());
        assert_eq!(tp0["alinhado_com_o_cliente"], true, "{}", tp0["motivo"]);
        let usado = tp0["pontos"].as_array().unwrap().iter().find(|p| !p["usado_por"].as_array().unwrap().is_empty()).unwrap()["id"].as_i64().unwrap();
        let r9 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "destinos", "operacao_id": "t_p4_r9"})).unwrap())["id"].as_i64().unwrap();
        let (s, msg) = erro_de(pedir("DELETE", &format!("estudio/teleportes/{usado}?rascunho={r9}")).call());
        assert!(s == 400 && msg.contains("transportadora"), "{s} {msg}");
        pedir("PUT", &format!("estudio/teleportes/{usado}?rascunho={r9}")).send_json(json!({"campos": {"nome": "Vila Renomeada"}})).unwrap();
        let nd = json_de(pedir("POST", &format!("estudio/mapas/world/teleportes?rascunho={r9}"))
            .send_json(json!({"base": usado, "campos": {"nome": "Destino Novo", "pos": [100.12345, 200.0, -300.5]}})).unwrap());
        let id_novo = nd["resultado"]["id"].as_i64().unwrap();
        let maior = tp0["pontos"].as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).max().unwrap();
        assert!(id_novo > maior, "{id_novo} > {maior}");
        let p9 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r9]})).unwrap());
        assert_eq!(p9["aplicados"].as_array().unwrap().len(), 1, "{p9}");
        let baixar = |c: &str| { let mut b = Vec::new(); std::io::Read::read_to_end(&mut pedir("GET", "arquivo").query("caminho", c).call().unwrap().into_reader(), &mut b).unwrap(); b };
        let txt9 = pw_data_loader::world_targets::TxtDosDestinos::ler(&baixar("maps\\world\\world_targets.txt")).unwrap();
        let sev9 = pw_data_loader::world_targets::PontosDoMundo::load_from_bytes(&fs::read(realm2.join("world_targets.sev")).unwrap()).unwrap();
        assert_eq!(txt9.destinos.iter().map(|d| d.id).collect::<Vec<_>>(), sev9.lista.iter().map(|p| p.id).collect::<Vec<_>>(), "ids alinhados");
        let novo_txt = txt9.destinos.iter().find(|d| d.id as i64 == id_novo).unwrap();
        assert_eq!((novo_txt.nome.as_str(), novo_txt.pos, sev9.get(id_novo as i32).unwrap().pos), ("Destino Novo", [100.123, 200.0, -300.5], [100.123, 200.0, -300.5]));
        assert_eq!(txt9.destinos.iter().find(|d| d.id as i64 == usado).unwrap().nome, "Vila Renomeada");

        // Distritos e portais em par com o cliente (B251): nome, polígono, criar e remover gravam o .sev (realm) e o
        // .clt (cliente) no mesmo rascunho, com os carimbos intactos.
        let d0 = json_de(pedir("GET", "estudio/mapas/world/distritos").call().unwrap());
        assert_eq!((d0["alinhado_com_o_cliente"].as_bool(), d0["distritos"][0]["nome"].as_str()), (Some(true), Some("D0")), "{d0}");
        let p0 = json_de(pedir("GET", "estudio/mapas/world/portais").call().unwrap());
        assert_eq!(p0["alinhado_com_o_cliente"], true, "{p0}");
        let r8 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "par", "operacao_id": "t_p4_r8"})).unwrap())["id"].as_i64().unwrap();
        let em8 = |rel: &str| format!("{rel}?rascunho={r8}");
        assert_eq!(erro_de(pedir("PUT", &em8("estudio/mapas/world/distritos/0")).send_json(json!({"campos": {"vertices": [[0, 0], [1, 1]]}}))).0, 400, "menos de 3 pontos");
        assert_eq!(erro_de(pedir("PUT", &em8("estudio/mapas/world/distritos/0")).send_json(json!({"campos": {"nome": "a\"b"}}))).0, 400);
        pedir("PUT", &em8("estudio/mapas/world/distritos/0")).send_json(json!({"campos": {"nome": "Cidade Nova", "vertices": [[0, 0], [10, 0], [10, 10]]}})).unwrap();
        let novo = json_de(pedir("POST", &em8("estudio/mapas/world/distritos")).send_json(json!({"base": 0, "campos": {"nome": "Outro", "vertices": [[0, 0], [5, 0], [5, 5], [0, 5]], "ponto": [1, 2, 3]}})).unwrap());
        assert_eq!(novo["resultado"]["indice"], 1, "{novo}");
        let d1 = json_de(pedir("GET", &em8("estudio/mapas/world/distritos")).call().unwrap());
        assert_eq!((d1["alinhado_com_o_cliente"].as_bool(), d1["distritos"].as_array().unwrap().len()), (Some(true), 2), "{d1}");
        assert_eq!((d1["distritos"][0]["nome"].as_str(), d1["distritos"][1]["nome"].as_str()), (Some("Cidade Nova"), Some("Outro")));
        pedir("PUT", &em8("estudio/mapas/world/portais/0")).send_json(json!({"campos": {"destino": 101, "pos": [1.5, 2.0, 3.0]}})).unwrap();
        let np = json_de(pedir("POST", &em8("estudio/mapas/world/portais")).send_json(json!({"base": 0})).unwrap());
        assert_eq!(np["resultado"]["indice"], 1);
        pedir("DELETE", &em8("estudio/mapas/world/portais/1")).call().unwrap();
        let hist8 = json_de(pedir("GET", "alteracoes").call().unwrap());
        let lista8 = hist8.as_array().unwrap().iter().find(|a| a["id"] == r8).unwrap()["lista"].as_array().unwrap().clone();
        let mut caminhos: Vec<String> = lista8.iter().map(|i| format!("{} {}", i["destino"].as_str().unwrap(), i["caminho"].as_str().unwrap())).collect();
        caminhos.sort();
        assert_eq!(caminhos, ["cliente maps\\world\\precinct.clt", "cliente maps\\world\\region.clt",
            "servidor servidor\\world\\precinct.sev", "servidor servidor\\world\\region.sev"]);
        let p8 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r8]})).unwrap());
        assert_eq!(p8["aplicados"].as_array().unwrap().len(), 2, "{p8}");
        let baixar = |c: &str| { let mut b = Vec::new(); std::io::Read::read_to_end(&mut pedir("GET", "arquivo").query("caminho", c).call().unwrap().into_reader(), &mut b).unwrap(); b };
        let pc = pw_data_loader::clt::PrecinctDoCliente::ler(&baixar("maps\\world\\precinct.clt")).unwrap();
        let ps = pw_data_loader::precinct::Distritos::ler(&fs::read(realm2.join("world").join("precinct.sev")).unwrap()).unwrap();
        assert_eq!((pc.distritos.len(), ps.lista.len(), pc.carimbo), (2, 2, ps.carimbo), "carimbo e contagem iguais");
        assert_eq!((pc.distritos[1].nome.as_str(), pc.distritos[1].ponto_de_cidade, ps.lista[1].ponto_de_cidade), ("Outro", [1.0, 2.0, 3.0], [1.0, 2.0, 3.0]));
        assert_eq!(ps.lista[0].vertices.len(), 3);
        assert!(ps.distrito_em(7.0, 2.0, ps.lista[0].mapa).is_some(), "o polígono novo vale no GS");
        let rc = pw_data_loader::clt::RegionDoCliente::ler(&baixar("maps\\world\\region.clt")).unwrap();
        let rs = pw_data_loader::regioes::RegioesEditavel::ler(&fs::read(realm2.join("world").join("region.sev")).unwrap()).unwrap();
        assert_eq!(rc.carimbo, rs.carimbo);
        assert_eq!(rc.sequencia(), rs.registros.iter().map(|x| matches!(x, pw_data_loader::regioes::RegistroDeRegiao::Caixa(_))).collect::<Vec<_>>());
        let caixa = pw_data_loader::regioes::ler_caixas_de_transporte(&fs::read(realm2.join("world").join("region.sev")).unwrap()).unwrap()[0];
        assert_eq!((caixa.destino, caixa.pos), (101, [1.5, 2.0, 3.0]));

        // Alinhar o servidor com o cliente (B253): um precinct.clt com um distrito a menos no rascunho → o alinhamento
        // acusa; "alinhar" refaz o precinct.sev a partir do .clt (item servidor) e o par volta a ser equivalente.
        let al0 = json_de(pedir("GET", "estudio/alinhamento").call().unwrap());
        let w0 = al0.as_array().unwrap().iter().find(|m| m["caminho"] == "world").unwrap().clone();
        assert_eq!(w0["desalinhado"], false, "{w0}");
        let r10 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "alinhar", "operacao_id": "t_p4_r10"})).unwrap())["id"].as_i64().unwrap();
        let mut pc = pw_data_loader::clt::PrecinctDoCliente::ler(&baixar("maps\\world\\precinct.clt")).unwrap();
        let n_antes = pc.distritos.len();
        pc.distritos.pop();
        pedir("PUT", &format!("rascunhos/{r10}/arquivo")).query("caminho", "maps\\world\\precinct.clt").query("destino", "cliente")
            .send_bytes(&pc.gravar()).unwrap();
        let al1 = json_de(pedir("GET", &format!("estudio/alinhamento?rascunho={r10}")).call().unwrap());
        let w1 = al1.as_array().unwrap().iter().find(|m| m["caminho"] == "world").unwrap().clone();
        assert_eq!((w1["desalinhado"].as_bool(), w1["arquivos"]["precinct.sev"]["no_cliente"].as_u64()), (Some(true), Some(n_antes as u64 - 1)), "{w1}");
        let feito = json_de(pedir("POST", &format!("estudio/mapas/world/alinhar?rascunho={r10}")).call().unwrap());
        assert_eq!(feito["alinhados"].as_array().unwrap().len(), 1, "só o precinct: {feito}");
        let al2 = json_de(pedir("GET", &format!("estudio/alinhamento?rascunho={r10}")).call().unwrap());
        assert_eq!(al2.as_array().unwrap().iter().find(|m| m["caminho"] == "world").unwrap()["desalinhado"], false);
        let de_novo = json_de(pedir("POST", &format!("estudio/mapas/world/alinhar?rascunho={r10}")).call().unwrap());
        assert!(de_novo["alinhados"].as_array().unwrap().is_empty(), "já alinhado: nada a fazer");
        pedir("POST", &format!("rascunhos/{r10}/descartar")).send_json(json!({})).unwrap();

        // Missões (B255, só leitura): o tasks.data do realm, com os nomes do elements.
        let ms = json_de(pedir("GET", "estudio/missoes").call().unwrap());
        assert_eq!((ms["missoes"].as_u64(), ms["de_topo"].as_u64(), ms["total"].as_u64()), (Some(7994), Some(2819), Some(2819)), "{}", ms["versao_do_arquivo"]);
        let achou = json_de(pedir("GET", "estudio/missoes").query("busca", "1173").call().unwrap());
        assert_eq!(achou["linhas"][0][0], 1173);
        let mi = json_de(pedir("GET", "estudio/missoes/1173").call().unwrap());
        assert_eq!(mi["ficha"]["npc_que_entrega"], 3517, "{}", mi["ficha"]["name"]);
        assert!(mi["nomes"]["3517"]["nome"].is_string(), "o NPC 3517 tem nome: {}", mi["nomes"]);
        assert_eq!(erro_de(pedir("GET", "estudio/missoes/999999999").call()).0, 400);
        // Editar no lugar (B256): campos de tamanho fixo; publicar grava o mesmo tasks.data no cliente e no realm.
        assert!(mi["editaveis"].as_array().unwrap().iter().any(|c| c[0] == "min_level"));
        let r12 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "missão", "operacao_id": "t_p4_r12"})).unwrap())["id"].as_i64().unwrap();
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/missoes/1173?rascunho={r12}")).send_json(json!({"campos": {"titulo": "x"}}))).0, 400);
        assert!(mi["textos"]["descricao"].is_string(), "B262: a ficha traz os textos longos");
        assert!(mi["listas"].as_array().unwrap().iter().any(|l| l[0] == "pre_tasks"), "B263: listas fixas");
        assert!(mi["vetores"].as_array().unwrap().iter().any(|v| v[0] == "monster_kills" && v[1] == 3), "B264: vetores com o teto");
        let quatro = Value::Array(vec![json!({"monstro": 1, "quantidade": 1, "item_que_cai": 0, "quantidade_do_item": 0, "item_comum": false, "chance_do_item": 0.0, "nivel_do_matador": false, "dps": 0, "dph": 0}); 4]);
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/missoes/1173?rascunho={r12}")).send_json(json!({"campos": {"monster_kills": quatro}}))).0, 400, "teto de 3");
        assert_eq!(mi["grupos_do_premio"]["grupos"], 12, "B265");
        assert_eq!((mi["dialogos"].as_array().unwrap().len(), mi["dialogos"][0]["nome"].as_str()), (5, Some("entrega")), "B266");
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/missoes/1173?rascunho={r12}")).send_json(json!({"campos": {"dialogos": [{"dialogo": 0, "janela": 0, "acao": "nova_opcao", "texto": "x", "destino": {"funcao": 99}}]}}))).0, 400, "B270: função desconhecida");
        assert_eq!(erro_de(pedir("POST", &format!("estudio/missoes/1173/clonar?rascunho={r12}")).send_json(json!({"novo": 70000}))).0, 400, "B267: id > 65535");
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/missoes/1173?rascunho={r12}")).send_json(json!({"campos": {"dialogos": [{"dialogo": 0, "texto": "x".repeat(64)}]}}))).0, 400, "text[64]");
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/missoes/1173?rascunho={r12}")).send_json(json!({"campos": {"pre_tasks": [999999999]}}))).0, 400);
        pedir("PUT", &format!("estudio/missoes/1173?rascunho={r12}")).send_json(json!({"campos": {"min_level": 5, "exp": 999, "name": "Primeiro Teste do Estúdio",
            "pre_tasks": [1174], "dialogos": [{"dialogo": 0, "texto": "Título do Estúdio"},
                {"dialogo": 0, "janela": 0, "acao": "nova_opcao", "texto": "Conte mais", "destino": {"janela": "Janela do Estúdio"}}], "itens_entregues": [{"id": 3044, "comum": true, "quantidade": 3, "probabilidade": 1.0, "validade": 0}],
            "premio_grupos": [{"sorteia_um": false, "itens": [{"id": 3045, "comum": true, "quantidade": 2, "probabilidade": 1.0, "validade": 0}]}], "descricao": "Descrição longa do Estúdio, que muda o tamanho da missão e a tabela de deslocamentos", "tributo": ""}})).unwrap();
        let origem_do_clone = {
            let d = pw_data_loader::tasks::TasksData::load_from_bytes(&fs::read(realm2.join("tasks.data")).unwrap()).unwrap();
            *d.de_topo.iter().find(|x| d.tasks[x].sub_tasks.is_empty()).expect("missão de topo sem submissões")
        };
        // B269: a 1173 tem submissões — clona a árvore e remove de novo.
        let arv = json_de(pedir("POST", &format!("estudio/missoes/1173/clonar?rascunho={r12}")).send_json(json!({})).unwrap());
        let id_arv = arv["resultado"]["id"].as_u64().unwrap();
        let rm_arv = json_de(pedir("DELETE", &format!("estudio/missoes/{id_arv}?rascunho={r12}")).call().unwrap());
        assert!(rm_arv["resultado"]["missoes_removidas"].as_u64().unwrap() > 1, "B269: a árvore inteira: {rm_arv}");
        let cl = json_de(pedir("POST", &format!("estudio/missoes/{origem_do_clone}/clonar?rascunho={r12}")).send_json(json!({})).unwrap());
        let id_clone = cl["resultado"]["id"].as_u64().expect("B267: id do clone") as u32;
        // Remover (B268): citada por outra missão → 400; o clone (não citado) sai e volta a entrar.
        let citada = {
            let d = pw_data_loader::tasks_editavel::TasksEditavel::ler(&fs::read(realm2.join("tasks.data")).unwrap()).unwrap();
            *d.dados.de_topo.iter().find(|&&x| !d.citacoes_da_missao(x).is_empty()).expect("missão citada")
        };
        assert_eq!(erro_de(pedir("DELETE", &format!("estudio/missoes/{citada}?rascunho={r12}")).call()).0, 400, "B268: citada");
        let rm = json_de(pedir("DELETE", &format!("estudio/missoes/{id_clone}?rascunho={r12}")).call().unwrap());
        assert_eq!(rm["resultado"]["missoes_removidas"], 1, "B268");
        let cl = json_de(pedir("POST", &format!("estudio/missoes/{origem_do_clone}/clonar?rascunho={r12}")).send_json(json!({"novo": id_clone})).unwrap());
        assert_eq!(cl["resultado"]["id"].as_u64(), Some(id_clone as u64));
        let mr = json_de(pedir("GET", &format!("estudio/missoes/1173?rascunho={r12}")).call().unwrap());
        assert_eq!((mr["ficha"]["min_level"].as_u64(), mr["ficha"]["rewards"]["exp"].as_i64(), mr["ficha"]["name"].as_str()), (Some(5), Some(999), Some("Primeiro Teste do Estúdio")));
        assert_eq!(json_de(pedir("GET", "estudio/missoes/1173").call().unwrap())["ficha"]["min_level"], mi["ficha"]["min_level"], "o publicado não muda");
        let p12 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r12]})).unwrap());
        assert_eq!(p12["aplicados"].as_array().unwrap().len(), 1, "{p12}");
        let tr = fs::read(realm2.join("tasks.data")).unwrap();
        let lido = pw_data_loader::tasks::TasksData::load_from_bytes(&tr).unwrap();
        assert_eq!((lido.tasks[&1173].min_level, lido.tasks[&1173].rewards.exp, lido.tasks.len()), (5, 999, 7995), "com o clone");
        assert_eq!(lido.tasks[&id_clone].name, lido.tasks[&origem_do_clone].name, "B267: o clone tem o nome da origem");
        assert!(lido.tasks[&1173].descricao.starts_with("Descrição longa do Estúdio"), "B262: texto longo publicado no realm");
        assert_eq!(lido.tasks[&1173].pre_tasks, vec![1174], "B263: lista fixa publicada");
        assert_eq!((lido.tasks[&1173].itens_entregues.len(), lido.tasks[&1173].itens_entregues[0].quantidade), (1, 3), "B264: vetor publicado");
        assert_eq!(lido.tasks[&1173].rewards.grupos_de_itens[0].itens[0].id, 3045, "B265: prêmio em item publicado");
        let ed = pw_data_loader::tasks_editavel::TasksEditavel::ler(&tr).unwrap();
        assert_eq!(ed.dialogos(1173).unwrap()[0].texto, "Título do Estúdio", "B266: título do diálogo publicado");
        assert_eq!(ed.dialogos(1173).unwrap()[0].janelas.last().unwrap().texto, "Janela do Estúdio", "B270: janela nova publicada");

        // Comportamento (B258, só leitura): as políticas do realm e os monstros que as usam.
        let cp = json_de(pedir("GET", "estudio/comportamentos").call().unwrap());
        assert_eq!(cp["politicas"].as_u64(), Some(293), "{}", cp["versao_do_arquivo"]);
        let usada = cp["linhas"].as_array().unwrap().iter().find(|l| l[2].as_u64().unwrap_or(0) > 0).expect("alguma política usada").clone();
        let pol = json_de(pedir("GET", &format!("estudio/comportamentos/{}", usada[0])).call().unwrap());
        assert_eq!(pol["politica"]["triggers"].as_array().unwrap().len() as u64, usada[1].as_u64().unwrap());
        assert!(pol["monstros"][0]["nome"].is_string(), "{}", pol["monstros"]);
        let por_monstro = json_de(pedir("GET", "estudio/comportamentos").query("busca", &pol["monstros"][0]["id"].to_string()).call().unwrap());
        assert!(por_monstro["linhas"].as_array().unwrap().iter().any(|l| l[0] == usada[0]), "busca pelo id do monstro");
        assert_eq!(erro_de(pedir("GET", "estudio/comportamentos/999999999").call()).0, 400);
        // Editar no lugar (B259): uma operação de habilidade de alguma política; item "servidor", publicado no realm.
        let politicas = pw_data_loader::aipolicy::AiPolicyData::load_from_bytes(&fs::read(realm2.join("aipolicy.data")).unwrap()).unwrap();
        let mut ids: Vec<u32> = politicas.policies.keys().copied().collect();
        ids.sort_unstable();
        let (pid, ti, oi) = ids.iter().find_map(|&pid| politicas.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| {
            t.operacoes.iter().position(|o| matches!(o.parametro, pw_data_loader::aipolicy::ParametroDeOperacao::Skill { .. })).map(|oi| (pid, ti, oi))
        })).expect("operação de habilidade");
        let ficha = json_de(pedir("GET", &format!("estudio/comportamentos/{pid}")).call().unwrap());
        assert_eq!(ficha["editaveis"][ti]["operacoes"][oi.to_string()][0][0], "skill", "{}", ficha["editaveis"][ti]);
        let r13 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "aipolicy", "operacao_id": "t_p4_r13"})).unwrap())["id"].as_i64().unwrap();
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/comportamentos/{pid}?rascunho={r13}")).send_json(json!({"gatilho": ti, "operacao": {"indice": oi, "valores": {"voar": 1}}}))).0, 400);
        pedir("PUT", &format!("estudio/comportamentos/{pid}?rascunho={r13}"))
            .send_json(json!({"gatilho": ti, "flags": {"so_em_combate": true}, "operacao": {"indice": oi, "valores": {"skill": 4242, "nivel": 3}}})).unwrap();
        let f2 = json_de(pedir("GET", &format!("estudio/comportamentos/{pid}?rascunho={r13}")).call().unwrap());
        assert_eq!(f2["politica"]["triggers"][ti]["operacoes"][oi]["parametro"], json!({"Skill": {"skill": 4242, "nivel": 3}}));
        let p13 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r13]})).unwrap());
        assert_eq!(p13["aplicados"].as_array().unwrap().len(), 1, "{p13}");
        let lido = pw_data_loader::aipolicy::AiPolicyData::load_from_bytes(&fs::read(realm2.join("aipolicy.data")).unwrap()).unwrap();
        assert_eq!(lido.policies[&pid].triggers[ti].operacoes[oi].parametro, pw_data_loader::aipolicy::ParametroDeOperacao::Skill { skill: 4242, nivel: 3 });
        assert!(lido.policies[&pid].triggers[ti].so_em_combate);

        // Comportamento estrutural (B261): trocar a fala, duplicar e remover gatilho (o leitor do GS relê).
        let pol_fala = {
            let d = pw_data_loader::aipolicy::AiPolicyData::load_from_bytes(&fs::read(realm2.join("aipolicy.data")).unwrap()).unwrap();
            let mut ids: Vec<u32> = d.policies.keys().copied().collect();
            ids.sort_unstable();
            ids.iter().find_map(|&pid| d.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| t.operacoes.iter().position(|o|
                matches!(o.parametro, pw_data_loader::aipolicy::ParametroDeOperacao::Fala { .. })).map(|oi| (pid, ti, oi, d.policies[&pid].triggers.len()))))
        };
        if let Some((pid, ti, oi, n)) = pol_fala {
            let r15 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "falas", "operacao_id": "t_p4_r15"})).unwrap())["id"].as_i64().unwrap();
            let rota = format!("estudio/comportamentos/{pid}?rascunho={r15}");
            pedir("PUT", &rota).send_json(json!({"gatilho": ti, "fala": {"indice": oi, "texto": "Fala do Estúdio"}})).unwrap();
            let dup = json_de(pedir("PUT", &rota).send_json(json!({"gatilho": ti, "duplicar": {}})).unwrap());
            assert!(dup["resultado"]["novo_gatilho"].as_u64().is_some(), "{dup}");
            assert_eq!(erro_de(pedir("PUT", &rota).send_json(json!({"gatilho": ti, "fala": {"indice": 999, "texto": "x"}}))).0, 400);
            pedir("PUT", &rota).send_json(json!({"gatilho": ti, "remover": true})).unwrap();
            // B271: operação nova (atacar) e removida; tipo inexistente no arquivo → 400.
            let ficha = json_de(pedir("GET", &rota).call().unwrap());
            assert!(ficha["tipos"]["operacoes"].as_array().unwrap().iter().any(|t| t[0] == 0), "B271: catálogo");
            pedir("PUT", &rota).send_json(json!({"gatilho": ti, "operacao_nova": {"tipo": 0}})).unwrap();
            let nops = json_de(pedir("GET", &rota).call().unwrap())["politica"]["triggers"][ti]["operacoes"].as_array().unwrap().len();
            pedir("PUT", &rota).send_json(json!({"gatilho": ti, "remover_operacao": nops - 1})).unwrap();
            assert_eq!(erro_de(pedir("PUT", &rota).send_json(json!({"gatilho": ti, "operacao_nova": {"tipo": 9999}}))).0, 400);
            let p15 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r15]})).unwrap());
            assert_eq!(p15["aplicados"].as_array().unwrap().len(), 1, "{p15}");
            let d = pw_data_loader::aipolicy::AiPolicyData::load_from_bytes(&fs::read(realm2.join("aipolicy.data")).unwrap()).unwrap();
            let t = &d.policies[&pid].triggers;
            assert_eq!(t.len(), n, "duplicou um e removeu o original: a mesma quantidade");
            assert!(matches!(&t[ti].operacoes[oi].parametro, pw_data_loader::aipolicy::ParametroDeOperacao::Fala { texto, .. } if texto == "Fala do Estúdio"));
            assert_eq!(t[ti].id, dup["resultado"]["novo_gatilho"].as_u64().unwrap() as u32, "a cópia ficou no lugar do original");
        } else {
            eprintln!("AVISO: o aipolicy do realm de teste não tem fala — a edição estrutural NÃO foi verificada aqui.");
        }

        // Modelos 3D (B272): lista de .ski, ficha e GLB.
        let ml = json_de(pedir("GET", "estudio/modelos").query("busca", "caixa").call().unwrap());
        assert_eq!(ml["total"], 2, "{ml}");
        assert!(ml["linhas"][0]["caminho"].as_str().unwrap().to_lowercase().ends_with(".smd"), "B274: o .smd primeiro");
        // B274: o modelo inteiro — ficha com esqueleto e pele, GLB com skin.
        let smd = ml["linhas"][0]["caminho"].as_str().unwrap().to_string();
        let fm = json_de(pedir("GET", "estudio/modelos/ficha").query("caminho", &smd).call().unwrap());
        assert_eq!((fm["tipo"].as_str(), fm["esqueleto"]["ossos"][0][0].as_str(), fm["peles"][0]["malhas"].as_u64()), (Some("modelo"), Some("raiz"), Some(1)), "{fm}");
        let mut gm = Vec::new();
        std::io::Read::read_to_end(&mut pedir("GET", "estudio/modelos/glb").query("caminho", &smd).call().unwrap().into_reader(), &mut gm).unwrap();
        let nj = u32::from_le_bytes(gm[12..16].try_into().unwrap()) as usize;
        let docm: Value = serde_json::from_slice(&gm[20..20 + nj]).unwrap();
        assert_eq!(docm["skins"][0]["joints"].as_array().map(|x| x.len()), Some(2), "um osso + o esqueleto: {docm}");
        assert!(docm["meshes"][0]["primitives"][0]["attributes"]["WEIGHTS_0"].is_u64());
        let caminho = ml["linhas"][1]["caminho"].as_str().unwrap().to_string();
        let fi = json_de(pedir("GET", "estudio/modelos/ficha").query("caminho", &caminho).call().unwrap());
        assert_eq!((fi["versao"].as_u64(), fi["texturas"][0].as_str(), fi["materiais"][0]["nome"].as_str(), fi["malhas"][0]["triangulos"].as_u64()),
            (Some(9), Some("caixa.dds"), Some("madeira"), Some(1)), "{fi}");
        let mut glb = Vec::new();
        std::io::Read::read_to_end(&mut pedir("GET", "estudio/modelos/glb").query("caminho", &caminho).call().unwrap().into_reader(), &mut glb).unwrap();
        assert_eq!(&glb[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize, glb.len());
        // B273: a textura foi achada (pasta `textures`) e vai no GLB como PNG.
        assert!(fi["texturas_achadas"][0].as_str().is_some_and(|c| c.to_lowercase().ends_with("textures\\caixa.dds")), "{fi}");
        let nj = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc: Value = serde_json::from_slice(&glb[20..20 + nj]).unwrap();
        assert_eq!((doc["images"][0]["mimeType"].as_str(), doc["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64()), (Some("image/png"), Some(0)), "{doc}");
        assert_eq!(erro_de(pedir("GET", "estudio/modelos/ficha").query("caminho", "data\\elements.data").call()).0, 400);

        // Textos do cliente (B260): listar, buscar, editar, acrescentar (só na indexada); item "cliente".
        let tx = json_de(pedir("GET", "estudio/textos/item_ext_desc.txt").call().unwrap());
        assert_eq!((tx["indexada"].as_bool(), tx["entradas"].as_u64()), (Some(true), Some(2)));
        let fm = json_de(pedir("GET", "estudio/textos/fixed_msg.txt").query("busca", "experiência").call().unwrap());
        assert_eq!((fm["indexada"].as_bool(), fm["linhas"][0][0].as_i64()), (Some(false), Some(1)));
        assert_eq!(erro_de(pedir("GET", "estudio/textos/elements.data").call()).0, 400);
        let r14 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "textos", "operacao_id": "t_p4_r14"})).unwrap())["id"].as_i64().unwrap();
        pedir("PUT", &format!("estudio/textos/item_ext_desc.txt/10042?rascunho={r14}")).send_json(json!({"texto": "Descrição do Estúdio"})).unwrap();
        pedir("POST", &format!("estudio/textos/item_ext_desc.txt?rascunho={r14}")).send_json(json!({"id": 50000, "texto": "Item novo"})).unwrap();
        assert_eq!(erro_de(pedir("POST", &format!("estudio/textos/fixed_msg.txt?rascunho={r14}")).send_json(json!({"id": 9, "texto": "x"}))).0, 400, "sequencial não acrescenta");
        assert_eq!(erro_de(pedir("PUT", &format!("estudio/textos/item_ext_desc.txt/10042?rascunho={r14}")).send_json(json!({"texto": "com \"aspas\""}))).0, 400);
        let tx2 = json_de(pedir("GET", &format!("estudio/textos/item_ext_desc.txt?rascunho={r14}")).call().unwrap());
        assert_eq!(tx2["entradas"], 3);
        let p14 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r14]})).unwrap());
        assert_eq!(p14["aplicados"].as_array().unwrap().len(), 0, "texto é só do cliente: {p14}");
        let lido = pw_recursos::textos::TabelaDeTextos::ler(&baixar("configs\\item_ext_desc.txt")).unwrap();
        assert_eq!((lido.entrada(10042).unwrap().texto.as_str(), lido.entrada(50000).unwrap().texto.as_str()), ("Descrição do Estúdio", "Item novo"));

        // Loja Gold (B254): ofertas do gshop.data; editar preço e nome, clonar, remover; publicar grava o cliente e o
        // realm (item "ambos") com o carimbo intacto.
        let lj = json_de(pedir("GET", "estudio/loja/gshop.data").call().unwrap());
        assert_eq!((lj["formato"].as_str(), lj["ofertas"].as_array().unwrap().len(), lj["categorias"].as_array().unwrap().len()), (Some("V126"), 668, 8), "{}", lj["formato"]);
        let comitem = lj["ofertas"].as_array().unwrap().iter().filter(|o| o[7].is_string()).count();
        assert!(comitem * 10 >= 668 * 9, "{comitem} ofertas com o item no elements");
        assert_eq!(erro_de(pedir("GET", "estudio/loja/outra.data").call()).0, 400);
        let of0 = json_de(pedir("GET", "estudio/loja/gshop.data/0").call().unwrap());
        let carimbo0 = lj["carimbo"].as_u64().unwrap();
        let r11 = json_de(pedir("POST", "rascunhos").send_json(json!({"descricao": "loja", "operacao_id": "t_p4_r11"})).unwrap())["id"].as_i64().unwrap();
        let em11 = |rel: &str| format!("{rel}?rascunho={r11}");
        assert_eq!(erro_de(pedir("PUT", &em11("estudio/loja/gshop.data/0")).send_json(json!({"campos": {"szName": "x".repeat(40)}}))).0, 400);
        pedir("PUT", &em11("estudio/loja/gshop.data/0")).send_json(json!({"campos": {"buy_0_price": 777, "szName": "Oferta do Estúdio"}})).unwrap();
        let cl = json_de(pedir("POST", &em11("estudio/loja/gshop.data")).send_json(json!({"base": 0})).unwrap());
        assert_eq!(cl["resultado"]["indice"], 668);
        pedir("DELETE", &em11("estudio/loja/gshop.data/668")).call().unwrap();
        let of1 = json_de(pedir("GET", &em11("estudio/loja/gshop.data/0")).call().unwrap());
        assert_eq!((of1["campos"]["buy_0_price"].as_u64(), of1["campos"]["szName"].as_str()), (Some(777), Some("Oferta do Estúdio")));
        assert_eq!(of1["campos"]["id"], of0["campos"]["id"]);
        // Categorias (B257): renomear a 1ª subcategoria, acrescentar e tirar uma no fim; acao inválida → 400.
        pedir("POST", &em11("estudio/loja/gshop.data/categorias")).send_json(json!({"acao": "renomear", "categoria": 0, "sub": 0, "nome": "Sub do Estúdio"})).unwrap();
        pedir("POST", &em11("estudio/loja/gshop.data/categorias")).send_json(json!({"acao": "nova", "categoria": 0, "nome": "Temporária"})).unwrap();
        pedir("POST", &em11("estudio/loja/gshop.data/categorias")).send_json(json!({"acao": "remover", "categoria": 0})).unwrap();
        assert_eq!(erro_de(pedir("POST", &em11("estudio/loja/gshop.data/categorias")).send_json(json!({"acao": "x", "categoria": 0}))).0, 400);
        let lj2 = json_de(pedir("GET", &em11("estudio/loja/gshop.data")).call().unwrap());
        assert_eq!(lj2["categorias"][0]["subcategorias"][0], "Sub do Estúdio");
        assert_eq!(lj2["categorias"][0]["subcategorias"].as_array().unwrap().len(), lj["categorias"][0]["subcategorias"].as_array().unwrap().len());
        let p11 = json_de(pedir("POST", "publicar").send_json(json!({"rascunhos": [r11]})).unwrap());
        assert_eq!(p11["aplicados"].as_array().unwrap().len(), 1, "o realm recebe o gshop.data: {p11}");
        let no_realm = fs::read(realm2.join("gshop.data")).unwrap();
        assert_eq!(no_realm, baixar("data\\gshop.data"), "cliente e realm com o mesmo arquivo");
        let g = pw_data_loader::gshop_editavel::GshopEditavel::ler(&no_realm).unwrap();
        assert_eq!((g.ofertas.len(), g.carimbo as u64), (668, carimbo0));
        let loja_do_gs = pw_data_loader::gshop::GShopData::load_from_bytes(&no_realm).unwrap();
        assert_eq!(loja_do_gs.ofertas[0].opcoes[0].preco, 777, "o leitor do GS vê o preço novo");

        // Altura do chão (B248): o realm do teste não tem .hmap — a rota responde sem altura; coordenada inválida é 400.
        let al = json_de(pedir("GET", "estudio/mapas/world/altura").query("x", "10").query("z", "-20").call().unwrap());
        assert_eq!((al["y"].clone(), al["tem_terreno"].clone(), al["x"].as_f64()), (json!(null), json!(false), Some(10.0)));
        assert_eq!(erro_de(pedir("GET", "estudio/mapas/world/altura").query("x", "abc").query("z", "1").call()).0, 400);
    })
    .await
    .unwrap();

    apagar_tudo(&pool).await;
    for p in [cliente, dados, realm] {
        let _ = fs::remove_dir_all(p);
    }
}
