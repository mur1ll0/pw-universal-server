use crate::aipolicy::AiPolicyData;
use crate::armaduras::TabelasDeEquipamento;
use crate::classes::TabelaDeClasses;
use crate::collision::MapCollision;
use crate::elements::ElementsData;
use crate::generic_elements::{self, GenericElementsData};
use crate::gshop::GShopData;
use crate::monstros::TabelaDeMonstros;
use crate::npcgen::NpcGenData;
use crate::ptemplate::TabelaDeBase;
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
    /// Leitor **tipado** antigo (`TABLE_SIZES_V7`, 118 tabelas) — só populado quando
    /// [`Self::elements_generic`] não cobre a versão do arquivo (hoje, 1.2.6/v7). Ver
    /// `crates/pw-data-loader/src/generic_elements.rs` para o porquê da migração.
    pub elements: ElementsData,
    /// Leitor **genérico**, dirigido pelo catálogo de `specs/elements_layouts/` — 231
    /// tabelas, validado byte a byte. Populado quando a versão do `elements.data` deste
    /// realm está no catálogo (hoje, v156/1.5.5). `None` quando a pasta não tem
    /// `elements.data`, ou quando a versão só o leitor tipado acima cobre.
    pub elements_generic: Option<GenericElementsData>,
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
    /// Os templates de monstro de verdade, montados da tabela `MONSTER_ESSENCE` — ver
    /// [`crate::monstros`]. Só é populada quando [`Self::elements_generic`] cobre a versão
    /// do `elements.data` (hoje, v156/1.5.5); no 1.2.6/v7 fica vazia, e quem consulta
    /// precisa saber lidar com a ausência do template.
    ///
    /// É montada **depois** do `aipolicy.data`, porque reproduz a checagem do original:
    /// monstro que aponta para política inexistente tem o campo zerado, com aviso.
    pub monstros: TabelaDeMonstros,
    /// Os atributos por classe de personagem (`CHARRACTER_CLASS_CONFIG`) — ver
    /// [`crate::classes`]. É de onde saem a precisão e a evasão base do jogador. Vazia
    /// no 1.2.6/v7, pelo mesmo motivo de [`Self::monstros`].
    pub classes: TabelaDeClasses,

    /// As três tabelas de equipamento do realm — armas (`WEAPON_ESSENCE`), armaduras
    /// (`ARMOR_ESSENCE`) e acessórios (`DECORATION_ESSENCE`) — e a busca que decide qual
    /// delas responde por um id de item. Ver [`crate::armaduras::TabelasDeEquipamento`].
    ///
    /// É daqui que sai o bloco de dados que acompanha cada peça equipável no
    /// `OWN_ITEM_INFO`, e é esse bloco que decide se o cliente aceita o que está
    /// equipado: sem ele, a máscara de classes do item fica zerada no cliente e **toda**
    /// classe é recusada. Vazias no 1.2.6/v7.
    pub equipamentos: TabelasDeEquipamento,
    /// `(price, shop_price)` de cada item que declara os dois campos, por id.
    ///
    /// Varre **todas** as tabelas do `elements.data` em vez de conhecer uma por uma:
    /// arma, armadura, remédio, material e mais uma dúzia de famílias têm os mesmos dois
    /// campos, e a loja precisa do preço de qualquer uma delas. Vazio no 1.2.6/v7.
    pub precos: HashMap<u32, (i32, i32)>,
    /// Os atributos **base** por classe, do `ptemplate.conf` — ver [`crate::ptemplate`].
    /// Fonte diferente da de [`Self::classes`]: aquela traz o que escala por nível e por
    /// ponto de atributo, esta traz o ponto de partida do nível 1. Vazia quando o pacote
    /// do realm não trouxe o arquivo.
    pub base_das_classes: TabelaDeBase,
    
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
    /// A pasta de cada mapa, pelo mesmo id de [`Self::map_spawns`].
    ///
    /// Guardar o caminho, e não o conteúdo, é de propósito: o mapa de alturas do mundo
    /// principal são 88 blocos de 1 MB, e há 68 pastas de mapa no realm. Carregar todos
    /// aqui custaria centenas de MB em **todo** daemon que monta um `GameDataManager` —
    /// inclusive o `pw-link`, que não precisa de altura nenhuma. Quem precisa é o
    /// servidor de mundo, e só do mapa que ele serve: ver `WorldInstance::init_spawns`.
    pub pastas_de_mapa: HashMap<i32, std::path::PathBuf>,
    pub collisions: HashMap<i32, MapCollision>,

    /// `dwTimeStamp` de `<mapa>/region.sev`, indexado por ID de mapa/instância (mesma
    /// chave de [`Self::map_spawns`]). É um dos valores que `INST_DATA_CHECKOUT` (comando
    /// 206) manda ao cliente ao entrar no mundo — se não bater com o que o `region.clt`
    /// local do cliente tem, ele rejeita a instância e o mundo nunca termina de carregar
    /// (achado em 2026-09-03, cliente 1.5.5 real: "regionset timestamp error").
    pub region_timestamps: HashMap<i32, u32>,
    /// `dwTimeStamp` de `<mapa>/precinct.sev`, mesma história do campo acima.
    pub precinct_timestamps: HashMap<i32, u32>,
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
            // para o `edition` do handshake. Ler as tabelas é outra história, e o login não
            // pode depender dela.
            match ElementsData::ler_cabecalho(&data) {
                Ok((versao, _t)) => self.versao_do_elements = Some(versao),
                Err(e) => rel.falhou("elements.data (cabeçalho)", e),
            }

            // Duas leituras possíveis, escolhidas pela versão do próprio arquivo (nunca
            // pelo nome/pasta do realm — ver `pw_universal_overview` na memória, princípio
            // central do projeto). O catálogo genérico (`specs/elements_layouts/vNNN.json`)
            // é a fonte de verdade quando cobre a versão — validado byte a byte contra as
            // tabelas reais de v156 e v159, diferente do leitor tipado abaixo, que nunca
            // terminou de carregar nenhum dos dois. `load_elements_data_auto` detecta a
            // versão sozinho e escolhe os overrides certos pra ela (nunca aplica o override
            // de uma build a um arquivo de outra — ver `load_overrides_for_version`). O
            // leitor tipado só entra como fallback para versões que o catálogo ainda não
            // tem (1.2.6/v7 hoje).
            match generic_elements::load_elements_data_auto(&data) {
                Ok(d) => {
                    self.elements_generic = Some(d);
                    rel.lidos.push("elements.data".into());
                }
                Err(generic_elements::GenericElementsError::UnsupportedVersion(_)) => {
                    match ElementsData::load_from_bytes(&data) {
                        Ok(d) => {
                            self.elements = d;
                            rel.lidos.push("elements.data".into());
                        }
                        Err(e) => rel.falhou("elements.data", e),
                    }
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

        // A tabela de monstros vem dos dois arquivos acima: os atributos do `elements.data`
        // e a validação da política do `aipolicy.data`. Por isso é montada aqui, e não
        // dentro da carga de nenhum dos dois.
        if let Some(g) = &self.elements_generic {
            self.monstros = crate::monstros::carregar(g, Some(&self.aipolicy));
            self.classes = crate::classes::carregar(g);
            self.equipamentos = TabelasDeEquipamento::carregar(g);
            self.precos = crate::precos::carregar(g);
        }

        // O `ptemplate.conf` não é um `.data`: é um arquivo de configuração do `gamed`, e
        // no pacote original mora fora da pasta de `config`. Aqui ele é procurado junto
        // com os outros; ausência não é falha de carga (ver `ptemplate::ler_da_pasta`).
        if let Some(t) = crate::ptemplate::ler_da_pasta(dir) {
            self.base_das_classes = t;
            rel.lidos.push("ptemplate.conf".into());
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

    /// Quanto custa comprar este item de um NPC, em moedas.
    ///
    /// O original monta o preço da loja assim (`gs/serviceprovider.cpp:241-252`):
    ///
    /// ```cpp
    /// int shop_price = GetDataMan().get_item_shop_price(tid);
    /// if (shop_price < (int)data->price) shop_price = data->price;
    /// float fp = shop_price * _tax_rate * tax_rate + 0.5f;
    /// int price = (int)fp;  if (price <= 0) price = 1;
    /// ```
    ///
    /// Sem sistema de impostos (`_tax_rate`, que varia por NPC e por facção dona do
    /// território), o preço base é o próprio `shop_price`, com o piso no `price` — que é
    /// exatamente o que sobra da fórmula com as taxas em 1.
    ///
    /// `None` quando o realm não tem o item nas tabelas: quem chama decide, e recusar a
    /// venda é melhor do que cobrar um número inventado.
    pub fn preco_de_compra(&self, item_id: u32) -> Option<i32> {
        let (price, shop_price) = self.precos.get(&item_id).copied()?;
        Some(shop_price.max(price).max(1))
    }

    /// A durabilidade de fábrica de um equipamento, já na escala do cliente.
    ///
    /// O `elements.data` guarda `durability_min` na escala dele; o cliente multiplica por
    /// `ENDURANCE_SCALE` (100) ao montar o item, e é nessa escala que o `OWN_ITEM_INFO`
    /// viaja. Quem grava o item no banco guarda a escala do arquivo, e o codificador
    /// multiplica — então o que sai daqui é o número do arquivo.
    ///
    /// `None` para item que não é equipamento: poção não tem durabilidade.
    pub fn durabilidade_de_fabrica(&self, item_id: u32) -> Option<u32> {
        let e = &self.equipamentos;
        let d = if let Some(a) = e.armas.get(&item_id) {
            a.durabilidade
        } else if let Some(a) = e.armaduras.get(&item_id) {
            a.requisitos.durabilidade
        } else if let Some(d) = e.decoracoes.get(&item_id) {
            d.requisitos.durabilidade
        } else {
            return None;
        };
        (d > 0).then_some(d as u32)
    }

    /// Quanto um item de cura restaura de HP/MP, segundo o `elements.data` deste realm —
    /// `None` quando o item não é remédio (é a resposta certa para uma arma, não um zero
    /// disfarçado de cura).
    ///
    /// Funciona nos dois formatos: quando [`Self::elements_generic`] está populado (builds
    /// cobertas pelo catálogo, ex. v156/1.5.5), consulta a tabela `MEDICINE_ESSENCE` por lá;
    /// senão cai para o leitor tipado (`Self::elements.medicines`, 1.2.6/v7).
    pub fn quanto_o_remedio_restaura(&self, item_id: u32) -> Option<(i32, i32)> {
        if let Some(g) = &self.elements_generic {
            let rec = g
                .get("MEDICINE_ESSENCE")
                .iter()
                .find(|r| r.get("ID").and_then(|v| v.as_i32()) == Some(item_id as i32))?;
            let hp = rec.get("hp_add_total").and_then(|v| v.as_i32()).unwrap_or(0);
            let mp = rec.get("mp_add_total").and_then(|v| v.as_i32()).unwrap_or(0);
            return Some((hp, mp));
        }
        let m = self.elements.medicines.get(&item_id)?;
        Some((m.hp_restore, m.mp_restore))
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
        // O caminho da pasta, para quem precisar ler algo dela depois — hoje o mapa de
        // alturas, que o servidor de mundo carrega só do mapa que serve.
        self.pastas_de_mapa.insert(world_id, map_dir.to_path_buf());

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

        let region_nome = format!("{rotulo}/region.sev");
        if let Some(data) = rel.ler_como(map_dir, "region.sev", &region_nome) {
            match ler_timestamp_region_sev(&data) {
                Ok(ts) => {
                    self.region_timestamps.insert(world_id, ts);
                    rel.lidos.push(region_nome);
                }
                Err(e) => rel.falhou(&region_nome, e),
            }
        }

        let precinct_nome = format!("{rotulo}/precinct.sev");
        if let Some(data) = rel.ler_como(map_dir, "precinct.sev", &precinct_nome) {
            match ler_timestamp_precinct_sev(&data) {
                Ok(ts) => {
                    self.precinct_timestamps.insert(world_id, ts);
                    rel.lidos.push(precinct_nome);
                }
                Err(e) => rel.falhou(&precinct_nome, e),
            }
        }
    }
}

/// Lê `dwTimeStamp` de um `region.sev` — formato confirmado em
/// `cgame/gs/template/el_region.h`/`.cpp` (EvolvedPWServer), lado servidor (`#else
/// _ELEMENTCLIENT`) de `CELRegionSet::Load(const char*)`: `REGIONFILEHEADER4`
/// (`dwVersion:u32, iNumRegion:i32, iNumTrans:i32, dwTimeStamp:u32`), válido pra
/// `dwVersion >= 4` (`ELRGNFILE_VERSION = 5` na árvore que temos — é a única versão real
/// medida, `data/realm_155/config/world/region.sev`). Versões mais antigas (< 4) não têm
/// este campo (o carregador original usa `dwTimeStamp = 0`) — não implementadas aqui por
/// falta de um arquivo real pra confirmar o layout, mesmo princípio do resto do projeto:
/// recusar em vez de adivinhar.
fn ler_timestamp_region_sev(data: &[u8]) -> std::io::Result<u32> {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;
    let mut c = Cursor::new(data);
    let versao = c.read_u32::<LittleEndian>()?;
    if versao < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("region.sev versão {versao} < 4 não tem dwTimeStamp — formato não implementado, sem arquivo real pra confirmar"),
        ));
    }
    c.set_position(12); // dwVersion(4) + iNumRegion(4) + iNumTrans(4)
    Ok(c.read_u32::<LittleEndian>()?)
}

/// Lê `dwTimeStamp` de um `precinct.sev` — formato confirmado em
/// `cgame/gs/template/el_precinct.h` (EvolvedPWServer): `PRECINCTFILEHEADER5`
/// (`dwVersion:u32, iNumPrecinct:i32, dwTimeStamp:u32`), válido pra `dwVersion >= 5`
/// (única versão real medida: 7, `data/realm_155/config/world/precinct.sev`). Mesma
/// ressalva do `region.sev` acima para versões mais antigas.
fn ler_timestamp_precinct_sev(data: &[u8]) -> std::io::Result<u32> {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;
    let mut c = Cursor::new(data);
    let versao = c.read_u32::<LittleEndian>()?;
    if versao < 5 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("precinct.sev versão {versao} < 5 não tem dwTimeStamp — formato não implementado, sem arquivo real pra confirmar"),
        ));
    }
    c.set_position(8); // dwVersion(4) + iNumPrecinct(4)
    Ok(c.read_u32::<LittleEndian>()?)
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
