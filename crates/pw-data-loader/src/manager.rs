use crate::aipolicy::AiPolicyData;
use crate::collision::MapCollision;
use crate::elements::ElementsData;
use crate::gshop::GShopData;
use crate::npcgen::NpcGenData;
use crate::tasks::TasksData;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use tracing::{info, warn};

/// Um arquivo de dados que existia na pasta e não pôde ser carregado.
#[derive(Debug, Clone)]
pub struct FalhaDeCarga {
    /// Nome do arquivo relativo à pasta de configuração (ex.: `"elements.data"`).
    pub arquivo: String,
    /// O erro, já formatado — o tipo de erro varia por parser e não interessa a quem lê.
    pub motivo: String,
}

impl fmt::Display for FalhaDeCarga {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.arquivo, self.motivo)
    }
}

/// O que a carga conseguiu ler e o que ela não conseguiu.
///
/// # Por que isto não é um `Result`
///
/// Era, e o preço foi caro. O `load_from_directory` lia `elements.data` primeiro e
/// propagava o erro com `?`; um `elements.data` que o nosso parser não entende (o do 1.5.3
/// falha com *failed to fill whole buffer*) **abortava toda a carga seguinte**, inclusive a
/// dos dois `gshop`. Com os dois timestamps em zero, o `edition` do handshake saía
/// `3000007f7900` em vez de `300000917c571db3f456986c25` e o cliente 1.5.3 recusava o
/// login — sintoma a três camadas de distância da causa, e sem nenhuma menção a
/// `elements.data` em lugar nenhum.
///
/// A carga agora é **independente por arquivo**: cada um que falha entra em [`Self::falhas`]
/// e os outros continuam. Quem chama decide o que fazer, mas tem que *ver* — daí este tipo
/// existir em vez de um `Ok(())` que não diz nada.
#[derive(Debug, Clone, Default)]
pub struct RelatorioDeCarga {
    /// Arquivos encontrados e carregados, na ordem em que foram lidos.
    pub lidos: Vec<String>,
    /// Arquivos que existiam e falharam. Arquivo ausente **não** é falha: cada realm traz
    /// o subconjunto de dados que tem.
    pub falhas: Vec<FalhaDeCarga>,
}

impl RelatorioDeCarga {
    /// `true` quando nenhum arquivo presente falhou (uma pasta vazia também é "sem falha").
    pub fn sem_falhas(&self) -> bool {
        self.falhas.is_empty()
    }

    /// Registra a falha de um arquivo e devolve o próprio relatório encadeável.
    fn falhou(&mut self, arquivo: &str, motivo: impl fmt::Display) {
        self.falhas.push(FalhaDeCarga {
            arquivo: arquivo.to_string(),
            motivo: motivo.to_string(),
        });
    }

    /// Lê os bytes de `dir/nome`, se o arquivo existir.
    ///
    /// - arquivo ausente → `None`, sem registro (é o caso normal);
    /// - erro de leitura → `None`, com registro em [`Self::falhas`].
    fn ler(&mut self, dir: &Path, nome: &str) -> Option<Vec<u8>> {
        self.ler_como(dir, nome, nome)
    }

    /// Como [`Self::ler`], mas registrando a falha sob `rotulo` em vez do nome do arquivo.
    ///
    /// Serve às pastas de mapa, onde `npcgen.data` sozinho não diria de qual mapa.
    fn ler_como(&mut self, dir: &Path, nome: &str, rotulo: &str) -> Option<Vec<u8>> {
        let caminho = dir.join(nome);
        if !caminho.exists() {
            return None;
        }
        match std::fs::read(&caminho) {
            Ok(b) => Some(b),
            Err(e) => {
                self.falhou(rotulo, e);
                None
            }
        }
    }
}

impl fmt::Display for RelatorioDeCarga {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} arquivo(s) carregado(s)", self.lidos.len())?;
        if self.falhas.is_empty() {
            return Ok(());
        }
        write!(f, ", {} com falha:", self.falhas.len())?;
        for falha in &self.falhas {
            write!(f, "\n  - {falha}")?;
        }
        Ok(())
    }
}

/// Gerenciador Central de Dados de Jogo (Carregado na inicialização do World Server)
#[derive(Debug, Clone, Default)]
pub struct GameDataManager {
    pub elements: ElementsData,
    pub gshop: GShopData,
    /// O **segundo** shop. Existe separado porque o `edition` do handshake carrega os
    /// dois timestamps, e eles vêm de arquivos diferentes.
    ///
    /// Os nomes dependem de quem empacotou os dados — ver [`Self::load_from_directory`].
    pub gshop2: GShopData,
    /// O **terceiro** shop. Só existe (`timestamp != 0`) em realms cujo cliente foi
    /// compilado com `VIP` definido — achado no 1.5.5 (`EvolvedPWClient`,
    /// `EC_Game.cpp:655-659`): o `edition` desses clientes tem **cinco** valores em vez
    /// de quatro, com este timestamp no final. Ver `GameVersion::challenge_edition_tem_terceiro_gshop`
    /// em `pw-protocol`.
    pub gshop3: GShopData,
    pub tasks: TasksData,
    pub aipolicy: AiPolicyData,
    
    /// `ELEMENTDATA_VERSION` lido do cabeçalho do `elements.data` **deste realm**, quando
    /// o arquivo existe.
    ///
    /// Separado de `elements.version` de propósito: este sobrevive a um `elements.data` que
    /// o parser não consegue percorrer até o fim, e é ele que alimenta o `edition` do
    /// handshake. `None` = a pasta do realm não tem o arquivo.
    pub versao_do_elements: Option<u32>,
    /// `_task_templ_cur_version` lido do cabeçalho do `tasks.data` deste realm.
    pub versao_das_tasks: Option<u32>,

    // Spawns indexados por ID do Mapa/Instância (ex: 1 -> world/npcgen.data, 101 -> a01/npcgen.data)
    pub map_spawns: HashMap<i32, NpcGenData>,
    pub collisions: HashMap<i32, MapCollision>,
}

impl GameDataManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Carrega todos os arquivos de dados a partir de uma pasta de configuração (ex: `config/`).
    ///
    /// **Nunca aborta na primeira falha.** Cada arquivo é independente dos outros; o que
    /// falhar entra no [`RelatorioDeCarga`] devolvido e a carga segue. Ver a documentação
    /// daquele tipo para o incidente que motivou isto.
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, config_dir: P) -> RelatorioDeCarga {
        let dir = config_dir.as_ref();
        let mut rel = RelatorioDeCarga::default();
        info!("Carregando templates de jogo a partir de: {:?}", dir);

        // 1. Arquivos Globais de Configuração
        if let Some(data) = rel.ler(dir, "elements.data") {
            // O cabeçalho primeiro, e **em separado**: são 8 bytes exatos, documentados em
            // `ElementsData::ler_cabecalho`, e deles sai o `ELEMENTDATA_VERSION` que vai
            // para o `edition` do handshake. Ler as 118 tabelas é outra história — a do
            // 1.5.3 o nosso parser ainda não termina — e o login não pode depender dela.
            match ElementsData::ler_cabecalho(&data) {
                Ok((versao, _t)) => self.versao_do_elements = Some(versao),
                Err(e) => rel.falhou("elements.data (cabeçalho)", e),
            }

            match ElementsData::load_from_bytes(&data) {
                Ok(d) => {
                    self.elements = d;
                    rel.lidos.push("elements.data".into());
                }
                Err(e) => rel.falhou("elements.data", e),
            }
        }

        // Os dois shops, que alimentam os dois timestamps do `edition` no handshake.
        //
        // O mesmo par de valores vem em arquivos de **nomes diferentes** conforme quem
        // empacotou os dados. Em `CCommon/globaldataman.cpp` do cliente 1.5.3 há dois
        // caminhos de carga que preenchem os mesmos globais:
        //
        // | Empacotamento | `timestamp` | `timestamp2` | Onde |
        // | :--- | :--- | :--- | :--- |
        // | cliente | `Data\gshop.data` | `Data\gshop1.data` | linhas 597 e 652 |
        // | servidor (`_sev`) | `gshopsev.data` | `gshopsev1.data` | linhas 1009 e 1038 |
        //
        // Aceitar só os nomes do cliente foi um erro caro: as pastas de realm que temos
        // para o 1.5.3 trazem o par `gshopsev*`, então os dois timestamps ficavam zero, o
        // `edition` saía errado e o cliente recusava o login — com o usuário procurando
        // arquivos que já estavam ali, sob outro nome.
        const NOMES_GSHOP: [&str; 2] = ["gshop.data", "gshopsev.data"];
        const NOMES_GSHOP2: [&str; 2] = ["gshop1.data", "gshopsev1.data"];

        for nome in NOMES_GSHOP {
            if let Some(data) = rel.ler(dir, nome) {
                match GShopData::load_from_bytes(&data) {
                    Ok(d) => {
                        self.gshop = d;
                        rel.lidos.push(nome.into());
                    }
                    Err(e) => rel.falhou(nome, e),
                }
                break;
            }
        }

        for nome in NOMES_GSHOP2 {
            if let Some(data) = rel.ler(dir, nome) {
                match GShopData::load_from_bytes(&data) {
                    Ok(d) => {
                        self.gshop2 = d;
                        rel.lidos.push(nome.into());
                    }
                    Err(e) => rel.falhou(nome, e),
                }
                break;
            }
        }

        // O terceiro, opcional: só clientes compilados com `VIP` o leem
        // (`EC_Game.cpp` do 1.5.5, `#ifdef VIP`). Ausente numa pasta de realm 1.2.6/1.5.3
        // é normal — não gera falha no relatório, só fica com `timestamp = 0`.
        const NOMES_GSHOP3: [&str; 2] = ["gshop2.data", "gshopsev2.data"];
        for nome in NOMES_GSHOP3 {
            if let Some(data) = rel.ler(dir, nome) {
                match GShopData::load_from_bytes(&data) {
                    Ok(d) => {
                        self.gshop3 = d;
                        rel.lidos.push(nome.into());
                    }
                    Err(e) => rel.falhou(nome, e),
                }
                break;
            }
        }

        if let Some(data) = rel.ler(dir, "tasks.data") {
            // Mesma história do `elements.data`: o cabeçalho é curto e exato, e é dele que
            // sai o `_task_templ_cur_version` do `edition`.
            match TasksData::ler_cabecalho(&data) {
                Ok((versao, _n)) => self.versao_das_tasks = Some(versao),
                Err(e) => rel.falhou("tasks.data (cabeçalho)", e),
            }

            match TasksData::load_from_bytes(&data) {
                Ok(d) => {
                    self.tasks = d;
                    rel.lidos.push("tasks.data".into());
                }
                Err(e) => rel.falhou("tasks.data", e),
            }
        }

        if let Some(data) = rel.ler(dir, "aipolicy.data") {
            match AiPolicyData::load_from_bytes(&data) {
                Ok(d) => {
                    self.aipolicy = d;
                    rel.lidos.push("aipolicy.data".into());
                }
                Err(e) => rel.falhou("aipolicy.data", e),
            }
        }

        // 2. Carrega o npcgen.data do mundo principal (world/npcgen.data ou npcgen.data na raiz)
        let world_dir = dir.join("world");
        if world_dir.join("npcgen.data").exists() {
            self.load_map_folder(1, &world_dir, "world", &mut rel);
        } else {
            self.load_map_folder(1, dir, ".", &mut rel);
        }

        // Mapeamento das dungeons clássicas a01..a33 e b01..b35
        for i in 1..=33 {
            let nome = format!("a{i:02}");
            let map_path = dir.join(&nome);
            if map_path.exists() {
                self.load_map_folder(100 + i, &map_path, &nome, &mut rel);
            }
        }
        for i in 1..=35 {
            let nome = format!("b{i:02}");
            let map_path = dir.join(&nome);
            if map_path.exists() {
                self.load_map_folder(200 + i, &map_path, &nome, &mut rel);
            }
        }

        if rel.sem_falhas() {
            info!("Templates de dados e mapas carregados: {rel}");
        } else {
            warn!("Carga de templates incompleta — {rel}");
        }
        rel
    }

    /// Carrega os dados específicos de uma pasta de mapa (`npcgen.data` e colisão).
    ///
    /// `rotulo` é o nome da pasta como ela aparece no relatório; sem ele, uma falha em
    /// `a07/npcgen.data` seria indistinguível de uma em `b12/npcgen.data`.
    fn load_map_folder(
        &mut self,
        world_id: i32,
        map_dir: &Path,
        rotulo: &str,
        rel: &mut RelatorioDeCarga,
    ) {
        let npcgen = format!("{rotulo}/npcgen.data");
        if let Some(data) = rel.ler_como(map_dir, "npcgen.data", &npcgen) {
            let nome = npcgen;
            match NpcGenData::load_from_bytes(&data) {
                Ok(d) => {
                    self.map_spawns.insert(world_id, d);
                    rel.lidos.push(nome);
                }
                Err(e) => rel.falhou(&nome, e),
            }
        }

        let colisao = format!("{rotulo}/collision.clt");
        if let Some(data) = rel.ler_como(map_dir, "collision.clt", &colisao) {
            let nome = colisao;
            match MapCollision::load_from_bytes(world_id, &data) {
                Ok(d) => {
                    self.collisions.insert(world_id, d);
                    rel.lidos.push(nome);
                }
                Err(e) => rel.falhou(&nome, e),
            }
        }
    }
}

#[cfg(test)]
mod testes_do_gshop {
    use super::*;

    /// Escreve um `gshop` mínimo: `[timestamp: u32][contagem: u32]`, ambos little-endian.
    fn escrever_gshop(dir: &Path, nome: &str, timestamp: u32) {
        let mut bytes = timestamp.to_le_bytes().to_vec();
        bytes.extend_from_slice(&0u32.to_le_bytes()); // zero itens
        std::fs::write(dir.join(nome), bytes).unwrap();
    }

    fn pasta_temporaria(marca: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pw_gshop_{marca}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn os_nomes_do_cliente_sao_aceitos() {
        let dir = pasta_temporaria("cliente");
        escrever_gshop(&dir, "gshop.data", 1206433535);
        escrever_gshop(&dir, "gshop1.data", 1185265628);

        let mut m = GameDataManager::new();
        let rel = m.load_from_directory(&dir);
        assert!(rel.sem_falhas(), "{rel}");

        assert_eq!(m.gshop.timestamp, 1206433535);
        assert_eq!(m.gshop2.timestamp, 1185265628);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn os_nomes_do_servidor_sao_aceitos() {
        // Este é o caso real das pastas de realm do 1.5.3: `gshopsev.data` e
        // `gshopsev1.data`, do caminho `_sev` do `globaldataman.cpp`. Antes desta
        // mudança os dois timestamps ficavam zero e o cliente recusava o login.
        let dir = pasta_temporaria("servidor");
        escrever_gshop(&dir, "gshopsev.data", 1461564404);
        escrever_gshop(&dir, "gshopsev1.data", 1452829733);

        let mut m = GameDataManager::new();
        let rel = m.load_from_directory(&dir);
        assert!(rel.sem_falhas(), "{rel}");

        assert_eq!(
            m.gshop.timestamp, 1461564404,
            "o `gshopsev.data` não foi lido: o `edition` sairia zerado"
        );
        assert_eq!(m.gshop2.timestamp, 1452829733);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn o_nome_do_cliente_tem_precedencia_quando_os_dois_existem() {
        // Uma pasta com os dois pares é ambígua. A ordem é fixa e declarada para que o
        // resultado não dependa da ordem em que o sistema de arquivos lista os nomes.
        let dir = pasta_temporaria("ambos");
        escrever_gshop(&dir, "gshop.data", 111);
        escrever_gshop(&dir, "gshopsev.data", 222);

        let mut m = GameDataManager::new();
        let rel = m.load_from_directory(&dir);
        assert!(rel.sem_falhas(), "{rel}");

        assert_eq!(m.gshop.timestamp, 111);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn um_elements_quebrado_nao_impede_a_leitura_dos_gshop() {
        // Este é o teste do incidente. Antes, o `?` no `elements.data` abortava a função
        // inteira: os dois `gshop` ficavam zerados, o `edition` do handshake saía
        // `3000007f7900` e o cliente 1.5.3 recusava o login sem que nada no log falasse em
        // `elements.data`.
        //
        // O `elements.data` aqui é curto de propósito — é a mesma classe de falha do
        // arquivo real do 1.5.3 ("failed to fill whole buffer"), que o nosso parser ainda
        // não entende.
        let dir = pasta_temporaria("elements_quebrado");
        std::fs::write(dir.join("elements.data"), [0u8; 8]).unwrap();
        escrever_gshop(&dir, "gshopsev.data", 0x571d_b3f4);
        escrever_gshop(&dir, "gshopsev1.data", 0x5698_6c25);

        let mut m = GameDataManager::new();
        let rel = m.load_from_directory(&dir);

        assert_eq!(
            m.gshop.timestamp, 0x571d_b3f4,
            "o `elements.data` quebrado levou o `gshop` junto: {rel}"
        );
        assert_eq!(m.gshop2.timestamp, 0x5698_6c25, "idem para o `gshop2`: {rel}");

        // E a falha não pode ser engolida: quem chama precisa poder gritar no log.
        assert!(!rel.sem_falhas(), "o `elements.data` inválido passou como se estivesse bom");
        assert_eq!(rel.falhas.len(), 1, "só um arquivo devia ter falhado: {rel}");
        assert_eq!(rel.falhas[0].arquivo, "elements.data");
        assert!(
            rel.lidos.iter().any(|n| n == "gshopsev.data"),
            "o relatório não creditou o `gshopsev.data`: {rel}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_falha_de_um_mapa_diz_qual_mapa_foi() {
        // Um `npcgen.data` inválido em `a07` e outro em `b12` produziriam a mesma linha de
        // log se o relatório guardasse só o nome do arquivo — e "npcgen.data falhou" numa
        // pasta com 68 deles não é informação.
        let dir = pasta_temporaria("mapa_quebrado");
        std::fs::create_dir_all(dir.join("a07")).unwrap();
        std::fs::write(dir.join("a07/npcgen.data"), [0xffu8; 3]).unwrap();

        let mut m = GameDataManager::new();
        let rel = m.load_from_directory(&dir);

        assert_eq!(rel.falhas.len(), 1, "{rel}");
        assert_eq!(rel.falhas[0].arquivo, "a07/npcgen.data", "{rel}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sem_nenhum_dos_arquivos_o_timestamp_fica_zero() {
        // O zero é o sinal que o `pw-link` procura para avisar no log — não pode virar
        // erro de carga, senão o realm nem sobe.
        let dir = pasta_temporaria("vazia");
        let mut m = GameDataManager::new();
        let rel = m.load_from_directory(&dir);
        assert!(rel.sem_falhas(), "pasta vazia não é falha: {rel}");
        assert_eq!(m.gshop.timestamp, 0);
        assert_eq!(m.gshop2.timestamp, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
