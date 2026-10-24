//! Vários mapas num processo só: o roteador do barramento.
//!
//! # Por que existe
//!
//! Até 2026-09-14 cada mapa era um contêiner (`pw-world-155` para o mundo 1,
//! `pw-world-155-161` para o 161), e cada um carregava sozinho o `GameDataManager`
//! inteiro: **1,1 GB** o do 161, com 1.269 monstros, quase o mesmo que o do mundo 1 com
//! 29.620. O `gs.conf` do 1.5.5 lista ~80 mapas; um contêiner por mapa não escala.
//!
//! O original sobe vários mapas com **um comando** (`./gs gs01 gs.conf gmserver.conf
//! gsalias.conf is61`, no `start` do `pwserver_155v156`): carrega os dados comuns uma vez
//! (`FirstStepInit`) e faz `fork()` de um processo por mapa, que compartilham a memória já
//! carregada (`cgame/gs/start.cpp:185-234`). Aqui o equivalente é um processo com um
//! [`WorldInstance`] por mapa — cada um com seu próprio laço de tick — sobre o **mesmo**
//! `Arc<GameDataManager>`.
//!
//! # Como o jogador chega ao mapa certo
//!
//! O `pw-link` manda tudo para um endereço só. No `EnterWorld`, o roteador pergunta ao
//! banco em que mapa o personagem está gravado e passa a entregar a esse mapa tudo o que
//! vier daquele `roleid`: subcomandos e a saída. Cada mapa continua sendo um [`BusServer`]
//! completo — sessões, eventos do tick, visibilidade —, só que atrás do roteador em vez de
//! escutar o barramento sozinho.
//!
//! Separar um mapa pesado noutro processo continua possível: o link aceita
//! `GS_BUS=1=a:29100,161=b:29100`, e o roteador de cada processo só conhece os seus mapas.

use crate::bus_server::{BusServer, EnvioAoCliente};
use crate::world::WorldInstance;
use pw_bus::{BusListener, BusMessage};
use pw_protocol::packets::s2c::S2CGamedataSend;
use pw_storage::CharacterRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

mod templates;
pub use templates::{EdicaoDeTemplate, EstatisticasPedidas, HabilidadeDoTemplate, ItemDoTemplate, MoverNoTemplate, SlotDoTemplate};

/// Os mapas deste processo, e de qual mapa é cada jogador.
pub struct RoteadorDeMapas {
    /// Os mapas carregados. Muda pelo painel (B183): ligar carrega, desligar descarrega.
    /// Trava síncrona: nunca segurada durante `await` — quem precisa clona o `Arc`.
    mapas: std::sync::RwLock<HashMap<i32, Arc<BusServer>>>,
    /// O mapa que recebe um personagem cujo mapa gravado não é servido aqui.
    padrao: i32,
    donos: RwLock<HashMap<i32, i32>>,
    repo: CharacterRepository,
    /// Só transições de presença. Consulta administrativa nunca segura isto durante I/O.
    presenca: RwLock<()>,
    /// Rates do realm (E7): um processo atende um realm, todos os mapas leem daqui.
    taxas: std::sync::RwLock<crate::taxas::Taxas>,
    /// Mapas desligados pelo painel (E7): ninguém entra nem chega por troca.
    desligados: std::sync::RwLock<std::collections::HashSet<i32>>,
    /// O que é preciso para carregar um mapa novo em execução (B183). Sem isto (testes que
    /// montam o roteador à mão), ligar um mapa não carregado é recusado.
    carga: std::sync::OnceLock<CargaDeMapas>,
    /// B331: ver [`EstadoDaRecarga`]. Trava síncrona, nunca segurada durante `await`.
    recarga: std::sync::Mutex<EstadoDaRecarga>,
    /// Mapas sendo montados agora (ligados pelo painel, ainda fora de `mapas`).
    carregando: std::sync::Mutex<std::collections::HashSet<i32>>,
    /// Fila de trocas e o próprio roteador, para ligar os mapas carregados depois.
    trocas: std::sync::OnceLock<mpsc::UnboundedSender<crate::bus_server::PedidoDeTroca>>,
    eu: std::sync::OnceLock<std::sync::Weak<RoteadorDeMapas>>,
    /// B350 — os mapas servidos por **outro** servidor de mundo do realm (`MAPAS_REMOTOS`): ir a um deles troca a sessão
    /// de servidor (`SwitchServerStart`). Mapa que ninguém serve continua recusado.
    remotos: std::sync::RwLock<std::collections::HashSet<i32>>,
    /// B349 — as cópias de masmorra (uma por chave), com ids internos a partir de [`PRIMEIRA_COPIA`].
    copias: std::sync::Mutex<Copias>,
    /// B359 — o registro de grupos de todos os mapas deste roteador.
    grupos: Arc<std::sync::Mutex<crate::world::RegistroDeGrupos>>,
    /// Quando o processo montou o roteador (B249): o painel compara com a última publicação de arquivo do servidor
    /// para dizer "aguarda reinício" — os dados do `config` são lidos uma vez, na partida.
    iniciado_em: std::time::SystemTime,
}

/// B349 — o primeiro id interno de cópia de masmorra (nunca é `tag` de mapa).
pub const PRIMEIRA_COPIA: i32 = 1_000_000;
/// `_idle_time = 60*20` e `_life_time = 3600*4` (`gs/instance/instance_manager.h:164-165`), e o batimento de 10 s
/// (`HEARTBEAT_CHECK_INTERVAL`, `instance_manager.cpp:648-690`).
const OCIOSA_S: u64 = 20 * 60;
const VIDA_S: u64 = 4 * 3600;
const BATIMENTO_DAS_COPIAS_S: u64 = 10;
/// B353 — `aei_filter::Heartbeat` (`gs/aei_filter.cpp:32-106`): confere a cada 3 s; quem não confere tem 60 s.
const CONFERE_A_CADA_S: u64 = 3;
const PRAZO_DE_SAIDA_S: i32 = 60;
/// `KIR_NULL`, `KIR_KEY_MISMATCH` (`gs/aei_filter.h:12-13`).
const KIR_NULL: u8 = 0;
const KIR_KEY_MISMATCH: u8 = 1;

/// B353 — a chave de cópia `(roleid, contador)` do original (`_cur_ins_key_list`, `instance_hash_key`) num `i64`:
/// o contador nos 32 bits altos. Contador 0 dá o próprio `roleid`.
pub fn chave_de(roleid: i32, contador: u32) -> i64 {
    ((contador as i64) << 32) | (roleid as u32 as i64)
}

/// B349 — uma cópia de masmorra (`instance_manager::AllocWorld` com a `instance_hash_key`).
struct CopiaDeMasmorra {
    mapa: i32,
    chave: i64,
    criada: std::time::Instant,
    /// `w_obsolete`: desde quando está sem ninguém.
    vazia_desde: Option<std::time::Instant>,
}

#[derive(Default)]
struct Copias {
    /// `_team_ins_key_list`: a chave do grupo, guardada para cada membro quando um deles entra (desde o B359 o grupo
    /// também não se desfaz na troca de mapa, como o `_team` do original).
    chave_do_grupo: HashMap<i32, i64>,
    por_chave: HashMap<(i32, i64), i32>,
    copias: HashMap<i32, CopiaDeMasmorra>,
    proximo: i32,
    /// B353 — `_cur_tag_counter`: o contador de cada `(mapa, roleid)`, que o `ResetInstance` sobe.
    contadores: HashMap<(i32, i32), u32>,
    /// B353 — `aei_filter` em `WAIT_ESCAPE`: quem foi avisado, em que cópia e até quando fica.
    saindo: HashMap<i32, (i32, std::time::Instant)>,
}

impl Copias {
    /// A chave própria (`_cur_ins_key_list[tag]`).
    fn propria(&self, mapa: i32, roleid: i32) -> i64 {
        chave_de(roleid, self.contadores.get(&(mapa, roleid)).copied().unwrap_or(0))
    }

    /// `GetInstanceKey` (`player_imp.h:2577-2604`): a do grupo, se tem; senão a própria.
    fn atual(&self, mapa: i32, roleid: i32) -> i64 {
        self.chave_do_grupo.get(&roleid).copied().unwrap_or_else(|| self.propria(mapa, roleid))
    }

    /// `ResetInstance` (`gs/player.cpp:12925-12965`) da chave `velha`: o dono sobe o contador (`p.second++`) e a chave
    /// do grupo de quem tinha a velha passa à nova (`GM_MSG_REBUILD_TEAM_INSTANCE_KEY`). Devolve a nova.
    fn renovar(&mut self, mapa: i32, velha: i64) -> i64 {
        let dono = velha as u32 as i32;
        let n = self.contadores.entry((mapa, dono)).or_insert(0);
        *n = n.wrapping_add(1);
        let nova = self.propria(mapa, dono);
        for k in self.chave_do_grupo.values_mut() {
            if *k == velha {
                *k = nova;
            }
        }
        nova
    }
}

/// B353 — o que fazer com quem está numa cópia (`aei_filter::Heartbeat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conferencia {
    Fica,
    /// Avisar a saída em 60 s (`kickout_instance(KIR_KEY_MISMATCH, 60)`).
    Avisar,
    /// Voltou a conferir (ou morreu): cancelar (`kickout_instance(KIR_NULL, -1)`).
    Cancelar,
    /// O prazo acabou: `ReturnToTown`.
    Expulsar,
}

/// B353 — a decisão do `aei_filter` para um jogador: `confere` é a chave dele igual à da cópia; `morto` dispensa;
/// `prazo` é o fim dos 60 s, se já foi avisado.
pub fn conferir(confere: bool, morto: bool, prazo: Option<std::time::Instant>, agora: std::time::Instant) -> Conferencia {
    match prazo {
        None if !confere && !morto => Conferencia::Avisar,
        None => Conferencia::Fica,
        Some(p) if agora >= p => Conferencia::Expulsar,
        Some(_) if confere || morto => Conferencia::Cancelar,
        Some(_) => Conferencia::Fica,
    }
}

/// Para onde vai quem entra num mapa.
enum DestinoDeEntrada {
    /// Mapa comum (ou mundo paralelo): o próprio mapa.
    Mapa(i32),
    /// A cópia da masmorra (id interno).
    Copia(i32),
    /// A cópia da chave está cheia (`player_per_instance`) ou não deu para montar.
    Recusada,
}

/// Dados comuns e versão: o que `preparar_mapa` precisa para montar um mapa (B183).
pub struct CargaDeMapas {
    pub dados: Arc<pw_data_loader::GameDataManager>,
    pub versao: pw_protocol::GameVersion,
    /// B331: a pasta do `config` do realm, para a recarga sem reiniciar (`None` nos testes que montam à mão:
    /// a recarga é recusada).
    pub pasta: Option<std::path::PathBuf>,
    /// B331: os arquivos que já falhavam na partida — a recarga só é recusada por falha **nova**.
    pub falhas_na_partida: Vec<String>,
}

/// B331: a recarga dos dados sem reiniciar — os dados da última aplicada (os mapas ligados depois usam estes),
/// quando, se há uma em curso e as falhas da última recusada.
#[derive(Default)]
struct EstadoDaRecarga {
    dados: Option<Arc<pw_data_loader::GameDataManager>>,
    em: Option<std::time::SystemTime>,
    em_andamento: bool,
    falhas: Vec<String>,
}

/// `_world_limit.nofly` do `gs.conf` da versão (B133).
/// B334 — `TEAM_MEMBER_CAPACITY` da versão (ver [`crate::world::WorldInstance::capacidade_do_grupo`]).
pub fn capacidade_do_grupo(versao: pw_protocol::GameVersion) -> usize {
    if versao == pw_protocol::GameVersion::V1_2_6 { 6 } else { 10 }
}

pub fn sem_voo_do_mapa(versao: pw_protocol::GameVersion, mapa: i32) -> bool {
    let catalogo = if versao == pw_protocol::GameVersion::V1_2_6 {
        pw_data_loader::limites::CatalogoDeLimites::V126
    } else {
        pw_data_loader::limites::CatalogoDeLimites::V155
    };
    pw_data_loader::limites::sem_voo(catalogo, mapa)
}

impl RoteadorDeMapas {
    pub async fn iniciar_coordenacao_gm(self: &Arc<Self>, contas: pw_storage::AccountRepository,
        processo: String) -> anyhow::Result<tokio::task::JoinHandle<()>> {
        let repo = contas.coordenacao_gm();
        let encarnacao = hex::encode(pw_crypto::generate_login_challenge());
        let mut conexao = repo.registrar(&processo,&encarnacao).await?;
        let este = Arc::clone(self);
        let tarefa = tokio::spawn(async move {
            let mut intervalo = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                intervalo.tick().await;
                // Exclui entrada, saída, transferência e comandos enquanto reconcilia.
                // I/O só sob guarda de coordenação; nenhum lock do mundo durante banco.
                let _barreira = este.presenca.write().await;
                let fotografia = tokio::time::timeout(std::time::Duration::from_secs(2),repo.fotografia()).await;
                let f = match fotografia { Ok(Ok(f)) => Some(f), _ => None };
                for mapa in este.todos().into_iter().map(|(_, m)| m) { mapa.reconciliar_gm(f.as_ref()).await; }
                if let Some(f) = f {
                    if pw_storage::CoordenacaoGmRepository::confirmar(&mut conexao,&processo,&encarnacao,f.revisao).await.is_err() {
                        // O jogo não cai por isso (MEMORIA_DA_REFORMA §6.2 item 3): efeitos GM
                        // removidos e coordenação parada; comandos GM seguem conferidos no banco.
                        tracing::error!("coordenação GM: conexão de fencing perdida; coordenação desligada até reiniciar o GS");
                        for mapa in este.todos().into_iter().map(|(_, m)| m) { mapa.reconciliar_gm(None).await; }
                        break;
                    }
                } else { tracing::warn!("coordenação GM: banco indisponível, efeitos removidos, sem recibo"); }
            }
        });
        Ok(tarefa)
    }
    /// `mapas` na ordem da configuração; o primeiro é o padrão.
    pub fn new(mapas: Vec<(i32, Arc<BusServer>)>, repo: CharacterRepository) -> Self {
        assert!(!mapas.is_empty(), "um servidor de mundo sem mapa nenhum");
        let padrao = mapas[0].0;
        Self::com_padrao(padrao, mapas, repo)
    }

    /// Como [`Self::new`], com o mapa padrão dado: ele pode estar desligado pelo painel e
    /// fora de `mapas` (B183) — aí a entrada que cairia nele é recusada.
    pub fn com_padrao(padrao: i32, mapas: Vec<(i32, Arc<BusServer>)>, repo: CharacterRepository) -> Self {
        // B359 — um registro de grupos só para todos os mapas: trocar de mapa não desfaz o grupo.
        let grupos: Arc<std::sync::Mutex<crate::world::RegistroDeGrupos>> = Default::default();
        for (_, servidor) in &mapas {
            match servidor.mundo().try_write() {
                Ok(mut m) => m.compartilhar_grupos(Arc::clone(&grupos)),
                Err(_) => warn!("roteador: mapa ocupado ao montar — os grupos dele ficam separados"),
            }
        }
        Self {
            grupos,
            mapas: std::sync::RwLock::new(mapas.into_iter().collect()),
            padrao,
            donos: RwLock::new(HashMap::new()),
            repo,
            presenca: RwLock::new(()),
            taxas: std::sync::RwLock::new(crate::taxas::Taxas::default()),
            desligados: std::sync::RwLock::new(std::collections::HashSet::new()),
            carga: std::sync::OnceLock::new(),
            recarga: Default::default(),
            carregando: std::sync::Mutex::new(std::collections::HashSet::new()),
            trocas: std::sync::OnceLock::new(),
            eu: std::sync::OnceLock::new(),
            iniciado_em: std::time::SystemTime::now(),
            copias: std::sync::Mutex::new(Copias::default()),
            remotos: std::sync::RwLock::new(std::collections::HashSet::new()),
        }
    }

    /// Permite ao painel carregar mapas em execução (B183).
    pub fn permitir_carga(&self, carga: CargaDeMapas) {
        let _ = self.carga.set(carga);
    }

    /// O mapa carregado, se houver.
    fn mapa(&self, id: i32) -> Option<Arc<BusServer>> {
        self.mapas.read().unwrap_or_else(|e| e.into_inner()).get(&id).cloned()
    }

    /// Os mapas carregados, em ordem.
    fn todos(&self) -> Vec<(i32, Arc<BusServer>)> {
        let mut v: Vec<(i32, Arc<BusServer>)> = self.mapas.read().unwrap_or_else(|e| e.into_inner())
            .iter().map(|(k, m)| (*k, Arc::clone(m))).collect();
        v.sort_unstable_by_key(|(k, _)| *k);
        v
    }

    fn serve(&self, id: i32) -> bool {
        self.mapas.read().unwrap_or_else(|e| e.into_inner()).contains_key(&id)
    }

    fn em_carga(&self) -> std::sync::MutexGuard<'_, std::collections::HashSet<i32>> {
        self.carregando.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// B208: um portal pode levar a `mapa` — ligado pelo painel e servido por este processo (o
    /// que o [`Self::trocar`] exige; sem isto a troca seria recusada em silêncio).
    pub fn aceita_destino(&self, mapa: i32) -> bool {
        // B350 — ou de outro servidor de mundo do realm (`MAPAS_REMOTOS`).
        self.mapa_ligado(mapa) && (self.serve(mapa) || self.remoto(mapa))
    }

    pub fn mapa_ligado(&self, mapa: i32) -> bool {
        !self.desligados.read().unwrap_or_else(|e| e.into_inner()).contains(&mapa)
    }

    /// Mapas desligados gravados (`realms.config.mapas_desligados`), lidos na partida.
    pub async fn carregar_mapas_desligados(&self, realms: &pw_storage::RealmRepository, realm: &str) {
        match realms.mapas_desligados(realm).await {
            Ok(lista) => {
                let meus = lista;
                if !meus.is_empty() { tracing::warn!("realm {realm}: mapas desligados pelo painel {meus:?}"); }
                self.desligados.write().unwrap_or_else(|e| e.into_inner()).extend(meus);
            }
            Err(e) => tracing::warn!("realm {realm}: mapas desligados ilegíveis ({e}); todos ligados"),
        }
    }

    /// Painel (E7, B177; B183): liga ou desliga um mapa. Grava primeiro (falha no banco não
    /// muda o mundo). **Ligar** um mapa carregado só libera a entrada; um não carregado é
    /// montado numa tarefa (`carregar_mapa`) e a resposta volta na hora com `carregando`.
    /// **Desligar** expulsa quem está no mapa pelo caminho do logout — salvo e de volta ao
    /// login — e descarrega; se alguém não saiu (saída pendente), o mapa fica carregado e
    /// bloqueado. Mapa que este processo não carrega só é aceito com `carregar` (o painel
    /// manda a um processo só, quando nenhum outro do realm o serve).
    pub async fn definir_mapa(self: &Arc<Self>, realms: &pw_storage::RealmRepository, realm: &str,
        mapa: i32, ligado: bool, carregar: bool) -> serde_json::Value {
        let presente = self.serve(mapa) || self.em_carga().contains(&mapa);
        if !presente {
            if !carregar {
                return serde_json::json!({"codigo":"mapa_nao_servido","mapa":mapa});
            }
            if ligado {
                let Some(carga) = self.carga.get() else {
                    return serde_json::json!({"codigo":"carga_indisponivel","mapa":mapa});
                };
                if !self.dados_vigentes(carga).pastas_de_mapa.contains_key(&mapa) {
                    return serde_json::json!({"codigo":"mapa_sem_dados","mapa":mapa});
                }
            }
        }
        match realms.definir_estado_do_mapa(realm, mapa, ligado).await {
            Ok(true) => {}
            Ok(false) => return serde_json::json!({"codigo":"realm_inexistente","mapa":mapa}),
            Err(e) => {
                tracing::warn!("mapa {mapa}: não gravou o estado ({e}); nada mudou");
                return serde_json::json!({"codigo":"banco_indisponivel","mapa":mapa});
            }
        }
        let _transicao = self.presenca.write().await;
        {
            let mut desligados = self.desligados.write().unwrap_or_else(|e| e.into_inner());
            if ligado { desligados.remove(&mapa); } else { desligados.insert(mapa); }
        }
        let resposta = |extra: serde_json::Value| {
            let mut j = serde_json::json!({"estado":"aplicado","tipo":"definir_mapa","mapa":mapa,"ligado":ligado});
            if let Some(o) = extra.as_object() {
                for (k, v) in o { j[k] = v.clone(); }
            }
            j
        };
        if ligado {
            if self.serve(mapa) {
                tracing::info!(mapa, "mapa ligado pelo painel");
                return resposta(serde_json::json!({"desconectados":0,"carregando":false}));
            }
            if self.em_carga().insert(mapa) {
                let este = Arc::clone(self);
                tokio::spawn(async move { este.carregar_mapa(mapa).await });
                tracing::info!(mapa, "mapa ligado pelo painel — carregando");
            }
            return resposta(serde_json::json!({"desconectados":0,"carregando":true}));
        }
        let Some(servidor) = self.mapa(mapa) else {
            // Em carga ou nunca carregado: a tarefa de carga vê o mapa desligado e o descarta.
            tracing::info!(mapa, "mapa desligado pelo painel (não estava carregado)");
            return resposta(serde_json::json!({"desconectados":0,"descarregado":true}));
        };
        let papeis: Vec<i32> = self.donos.read().await.iter()
            .filter(|(_, m)| **m == mapa).map(|(r, _)| *r).collect();
        let mut desconectados = 0;
        for role in papeis {
            if servidor.expulsar_pelo_painel(role).await { desconectados += 1; }
        }
        let ainda_com_dono = self.donos.read().await.values().any(|m| *m == mapa);
        let descarregado = !ainda_com_dono && servidor.vazio().await;
        if descarregado {
            self.mapas.write().unwrap_or_else(|e| e.into_inner()).remove(&mapa);
            servidor.descarregar().await;
        } else {
            tracing::warn!(mapa, "mapa desligado ainda com jogador (saída pendente?) — fica carregado e bloqueado");
        }
        tracing::info!(mapa, desconectados, descarregado, "mapa desligado pelo painel");
        resposta(serde_json::json!({"desconectados":desconectados,"descarregado":descarregado}))
    }

    /// Monta um mapa ligado pelo painel (B183). O pesado (`WorldInstance::new` e
    /// `init_spawns` — 29 mil monstros no mundo 1 do 1.5.5) roda numa thread de bloqueio,
    /// fora da guarda de presença; só a inserção pega a guarda. Desligado no meio do
    /// caminho, o mapa montado é descartado.
    async fn carregar_mapa(self: Arc<Self>, mapa: i32) {
        let Some(carga) = self.carga.get() else {
            self.em_carga().remove(&mapa);
            return;
        };
        let (dados, versao, repo) = (self.dados_vigentes(carga), carga.versao, self.repo.clone());
        let inicio = std::time::Instant::now();
        let montado = tokio::task::spawn_blocking(move || {
            let mut mundo = WorldInstance::new(mapa, dados, repo);
            mundo.sem_voo = sem_voo_do_mapa(versao, mapa);
            mundo.init_spawns();
            mundo
        }).await;
        let mundo = match montado {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(mapa, "não consegui montar o mapa: {e}");
                self.em_carga().remove(&mapa);
                return;
            }
        };
        let (_, servidor) = Self::preparar_mapa_iniciado(mundo, versao).await;
        if let Some(envio) = self.trocas.get() {
            servidor.ligar_trocas(envio.clone());
        }
        if let Some(eu) = self.eu.get() {
            servidor.ligar_roteador(eu.clone());
        }
        let _transicao = self.presenca.write().await;
        self.em_carga().remove(&mapa);
        if !self.mapa_ligado(mapa) {
            tracing::info!(mapa, "mapa desligado durante a carga — descartado");
            servidor.descarregar().await;
            return;
        }
        servidor.mundo().write().await.compartilhar_grupos(Arc::clone(&self.grupos));
        self.mapas.write().unwrap_or_else(|e| e.into_inner()).insert(mapa, servidor);
        tracing::info!(mapa, ms = inicio.elapsed().as_millis() as u64, "mapa carregado pelo painel");
    }

    pub fn taxas(&self) -> crate::taxas::Taxas {
        *self.taxas.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn definir_taxas(&self, taxas: crate::taxas::Taxas) {
        *self.taxas.write().unwrap_or_else(|e| e.into_inner()) = taxas;
    }

    /// Lê as rates do realm no banco (`realms.double_*_multiplier`). Valor fora dos
    /// limites do painel fica no padrão 1× e vira aviso, em vez de bagunçar a economia.
    /// B354 — a cerimônia de casamento (`global_world_manager` com `wedding_server = 1`, `global_manager.cpp:241-250`):
    /// só no 1.5.5, só se este processo serve o mapa da cerimônia (134) e o `WEDDING_CONFIG` 801 é válido. Monta a
    /// agenda, carrega a gravada (`DBLoad`) e põe o batimento de 10 s (`RunTick`), que grava quando muda e tira da
    /// cena quem está numa cerimônia que acabou (`wedding_filter::Heartbeat`).
    pub async fn ligar_casamento(self: &Arc<Self>, realms: pw_storage::RealmRepository, realm: String) {
        let Some(carga) = self.carga.get() else { return };
        if carga.versao == pw_protocol::GameVersion::V1_2_6 {
            return;
        }
        let Some(mapa) = pw_data_loader::limites::mapa_do_casamento(pw_data_loader::limites::CatalogoDeLimites::V155) else {
            return;
        };
        if !self.serve(mapa) {
            info!("casamento: o mapa da cerimônia ({mapa}) não está neste processo — sem agenda");
            return;
        }
        let Some(cfg) = self.dados_vigentes(carga).casamento.clone() else {
            warn!("casamento: sem WEDDING_CONFIG 801 válido — sem agenda");
            return;
        };
        let agora = chrono::Utc::now().timestamp();
        let fuso = *chrono::Local::now().offset();
        let mut agenda = crate::casamento::AgendaDeCasamento::nova(&cfg, agora, fuso);
        match realms.casamento(&realm).await {
            Ok(gravadas) => {
                let perdidas = agenda.carregar(&gravadas);
                info!("casamento: agenda do mapa {mapa} com {} reservas gravadas ({perdidas} fora da janela)", gravadas.len());
            }
            Err(e) => warn!("casamento: não li a agenda gravada: {e}"),
        }
        if let Ok(mut a) = crate::casamento::agenda().lock() {
            *a = Some(agenda);
        }
        let este = Arc::clone(self);
        tokio::spawn(async move {
            let mut t = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                t.tick().await;
                este.batimento_do_casamento(&realms, &realm, mapa).await;
            }
        });
    }

    /// B354 — `wedding_manager::RunTick` e `wedding_filter::Heartbeat`.
    async fn batimento_do_casamento(&self, realms: &pw_storage::RealmRepository, realm: &str, mapa: i32) {
        let agora = chrono::Utc::now().timestamp();
        let (gravar, acabou): (Option<Vec<[i64; 6]>>, Vec<i32>) = {
            let Ok(mut a) = crate::casamento::agenda().lock() else { return };
            let Some(a) = a.as_mut() else { return };
            a.batimento(agora);
            let gravar = std::mem::take(&mut a.suja).then(|| a.gravar());
            let acabou = match crate::casamento::na_cena().lock() {
                Ok(mut n) => {
                    let fora: Vec<i32> = n.iter().filter(|(_, (g, b, c))| !a.em_andamento(*g, *b, *c)).map(|(r, _)| *r).collect();
                    for r in &fora {
                        n.remove(r);
                    }
                    fora
                }
                Err(_) => Vec::new(),
            };
            (gravar, acabou)
        };
        if let Some(lista) = gravar {
            if let Err(e) = realms.gravar_casamento(realm, &lista).await {
                warn!("casamento: não gravei a agenda: {e}");
            }
        }
        // `WAIT_ESCAPE`: o ponto de salvar do mapa (`save_point = 1,1224,223,1051` no 134), ou (0, 300, 0) do mapa 1.
        let destino = pw_data_loader::limites::do_mapa(pw_data_loader::limites::CatalogoDeLimites::V155, mapa)
            .and_then(|m| m.ponto_de_salvar)
            .filter(|p| p.mapa > 0 && p.mapa != mapa)
            .map(|p| (p.mapa, pw_core::Vector3::new(p.pos[0], p.pos[1], p.pos[2])))
            .unwrap_or((1, pw_core::Vector3::new(0.0, 300.0, 0.0)));
        for roleid in acabou {
            let Some(servidor) = self.mapa_de(roleid).await.filter(|m| *m == mapa).and_then(|m| self.mapa(m)) else { continue };
            servidor.enviar_ao_jogador(roleid, S2CGamedataSend::enter_wedding_scene(0, 0).data).await;
            info!("casamento: a cerimônia acabou — {roleid} sai da cena");
            servidor.transportar(roleid, destino.0, destino.1).await;
        }
    }

    pub async fn carregar_taxas(&self, realms: &pw_storage::RealmRepository, realm: &str) {
        match realms.get_realm(realm).await {
            Ok(Some(r)) => {
                let t = crate::taxas::Taxas {
                    exp: r.double_exp_multiplier as f64,
                    sp: r.double_sp_multiplier as f64,
                    drop: r.double_drop_multiplier as f64,
                    moedas: r.double_gold_multiplier as f64,
                }.arredondadas();
                if t.validas() {
                    tracing::info!(?t, "rates do realm {realm} carregadas");
                    self.definir_taxas(t);
                } else {
                    tracing::warn!(?t, "rates do realm {realm} fora dos limites; usando 1x");
                }
            }
            Ok(None) => tracing::warn!("realm {realm} não está na tabela realms; rates 1x"),
            Err(e) => tracing::warn!("rates do realm {realm} ilegíveis ({e}); rates 1x"),
        }
    }

    /// Começa a atender os pedidos de troca de mapa dos mapas deste processo.
    ///
    /// O original troca o jogador de processo (`SwitchSvr` → `PlaneSwitch`, `player.cpp:3191`);
    /// aqui os mapas são tarefas do mesmo processo, então a troca é tirar de um e pôr no outro,
    /// e passar a entregar aquele `roleid` ao novo. Mapa que este processo não serve não tem
    /// troca (`falta`: seria o link reencaminhar a sessão).
    /// B350 — os mapas de outros servidores de mundo do realm.
    pub fn definir_mapas_remotos(&self, mapas: impl IntoIterator<Item = i32>) {
        *self.remotos.write().unwrap_or_else(|e| e.into_inner()) = mapas.into_iter().collect();
    }

    fn remoto(&self, mapa: i32) -> bool {
        self.remotos.read().unwrap_or_else(|e| e.into_inner()).contains(&mapa)
    }

    /// B349 — a masmorra com cópia por chave (`masmorra_por_chave` do catálogo da versão): o limite por cópia.
    fn masmorra(&self, mapa: i32) -> Option<u32> {
        let versao = self.carga.get()?.versao;
        let c = if versao == pw_protocol::GameVersion::V1_2_6 {
            pw_data_loader::limites::CatalogoDeLimites::V126
        } else {
            pw_data_loader::limites::CatalogoDeLimites::V155
        };
        pw_data_loader::limites::masmorra_por_chave(c, mapa)
    }

    /// B349 — `GetInstanceKey` (`player_imp.h:2577-2610`): em grupo, a do líder; sozinho, a própria.
    async fn chave_de_masmorra(&self, roleid: i32, mapa: i32) -> i64 {
        let membros = match self.mapa_de(roleid).await.and_then(|m| self.mapa(m)) {
            Some(origem) => origem.mundo().read().await.membros_do_grupo(roleid),
            None => Vec::new(),
        };
        let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
        match membros.first() {
            Some(lider) => {
                // B353 — a chave do líder com o contador dele (`_team_ins_key_list = _cur_ins_key_list`).
                let chave = c.propria(mapa, *lider);
                for m in &membros {
                    c.chave_do_grupo.insert(*m, chave);
                }
                chave
            }
            None => c.atual(mapa, roleid),
        }
    }

    /// B353 — a chave com que este jogador entra agora no `mapa` (a do login gravado numa masmorra).
    pub fn chave_atual(&self, mapa: i32, roleid: i32) -> i64 {
        self.copias.lock().unwrap_or_else(|e| e.into_inner()).atual(mapa, roleid)
    }

    /// B353 — foi avisado de que sai da cópia (`aei_filter` em `WAIT_ESCAPE`).
    pub fn saindo_da_masmorra(&self, roleid: i32) -> bool {
        self.copias.lock().unwrap_or_else(|e| e.into_inner()).saindo.contains_key(&roleid)
    }

    /// B353 — saiu do grupo pelo comando (`TEAM_LEAVE_PARTY`): sem grupo, a chave volta a ser a própria
    /// (`GetInstanceKey`); se o grupo acabou, também a de quem ficou. Quem estiver numa cópia de outra chave é avisado
    /// na próxima conferência. Sair do jogo também tira do grupo, sem passar por aqui.
    pub fn ao_sair_do_grupo(&self, quem: i32, restantes: &[i32]) {
        let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
        c.chave_do_grupo.remove(&quem);
        if restantes.len() < 2 {
            for m in restantes {
                c.chave_do_grupo.remove(m);
            }
        }
    }

    /// B353 — alguém entrou num grupo: se o líder está numa cópia, a chave do grupo é a chave atual dele (a da cópia
    /// em que entrou) e vale para todos os membros.
    pub async fn ao_entrar_no_grupo(&self, lider: i32, membros: &[i32]) {
        let Some(id) = self.mapa_de(lider).await else { return };
        let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
        let Some(mapa) = c.copias.get(&id).map(|x| x.mapa) else { return };
        let chave = c.atual(mapa, lider);
        for m in membros {
            c.chave_do_grupo.insert(*m, chave);
        }
    }

    /// B353 — `aei_filter::Heartbeat` (`gs/aei_filter.cpp:32-106`) para todos que estão em cópias, a cada 3 s: a cópia
    /// com mais de 4 h (`w_life_time == 0`) renova a chave de quem ainda confere (`ResetInstance`); quem não confere
    /// (saiu do grupo, ou a chave foi renovada) e não está morto é avisado (`EXIT_INSTANCE` com 60 s) e, no fim do
    /// prazo, volta para a cidade (`ReturnToTown`); se voltar a conferir antes, o aviso é cancelado. Não portado: a
    /// isenção do GM (`gmfree`) e o excesso de membros do grupo no mapa (`KIR_PLAYERCOUNT_EXCEED`).
    pub async fn conferir_quem_esta_nas_copias(self: &Arc<Self>) {
        let donos: Vec<(i32, i32)> = self.donos.read().await.iter().map(|(r, m)| (*r, *m)).collect();
        let agora = std::time::Instant::now();
        let mut na_copia: Vec<(Arc<BusServer>, i32, i32, i32, i64)> = Vec::new();
        {
            let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
            c.saindo.retain(|r, (copia, _)| donos.iter().any(|(d, m)| d == r && m == copia));
            for (roleid, id) in &donos {
                let Some((mapa, chave, velha)) =
                    c.copias.get(id).map(|x| (x.mapa, x.chave, x.criada.elapsed().as_secs() >= VIDA_S))
                else {
                    continue;
                };
                if velha && c.atual(mapa, *roleid) == chave {
                    let nova = c.renovar(mapa, chave);
                    tracing::info!(roleid, mapa, id, nova, "masmorra: cópia vencida (4 h), chave renovada");
                }
                let Some(servidor) = self.mapa(*id) else { continue };
                na_copia.push((servidor, *roleid, mapa, *id, chave));
            }
        }
        let mut acoes: Vec<(Arc<BusServer>, i32, i32, Conferencia)> = Vec::new();
        for (servidor, roleid, mapa, id, chave) in na_copia {
            let morto = servidor.jogador_morto(roleid).await;
            let acao = {
                let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
                let confere = c.atual(mapa, roleid) == chave;
                let acao = conferir(confere, morto, c.saindo.get(&roleid).map(|(_, p)| *p), agora);
                match acao {
                    Conferencia::Avisar => {
                        c.saindo.insert(roleid, (id, agora + std::time::Duration::from_secs(PRAZO_DE_SAIDA_S as u64)));
                    }
                    Conferencia::Cancelar | Conferencia::Expulsar => {
                        c.saindo.remove(&roleid);
                    }
                    Conferencia::Fica => {}
                }
                acao
            };
            acoes.push((servidor, roleid, mapa, acao));
        }
        for (servidor, roleid, mapa, acao) in acoes {
            match acao {
                Conferencia::Avisar => {
                    info!("masmorra: {roleid} não confere com a cópia do {mapa} — sai em {PRAZO_DE_SAIDA_S} s");
                    servidor
                        .enviar_ao_jogador(roleid, S2CGamedataSend::exit_instance(mapa, KIR_KEY_MISMATCH, PRAZO_DE_SAIDA_S).data)
                        .await;
                }
                Conferencia::Cancelar => {
                    servidor.enviar_ao_jogador(roleid, S2CGamedataSend::exit_instance(mapa, KIR_NULL, -1).data).await;
                }
                Conferencia::Expulsar => {
                    info!("masmorra: {roleid} expulso da cópia do {mapa}");
                    servidor.voltar_para_a_cidade(roleid).await;
                }
                Conferencia::Fica => {}
            }
        }
    }

    /// B349 — para onde vai quem entra no `mapa` com a `chave`: a cópia existente (viva há menos de 4 h e com vaga), ou
    /// uma nova (`AllocWorld`); fora de masmorra, o próprio mapa.
    async fn destino_de_entrada(self: &Arc<Self>, mapa: i32, chave: i64) -> DestinoDeEntrada {
        let Some(por_copia) = self.masmorra(mapa) else { return DestinoDeEntrada::Mapa(mapa) };
        let existente = {
            let c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
            c.por_chave
                .get(&(mapa, chave))
                .copied()
                .filter(|id| c.copias.get(id).is_some_and(|x| x.criada.elapsed().as_secs() < VIDA_S))
        };
        if let Some(id) = existente.filter(|id| self.serve(*id)) {
            let n = self.donos.read().await.values().filter(|m| **m == id).count();
            return if n < por_copia as usize { DestinoDeEntrada::Copia(id) } else { DestinoDeEntrada::Recusada };
        }
        match self.montar_copia(mapa, chave).await {
            Some(id) => DestinoDeEntrada::Copia(id),
            None => DestinoDeEntrada::Recusada,
        }
    }

    /// B349 — monta uma cópia nova do mapa, como o painel liga um mapa (B183), com id interno.
    async fn montar_copia(self: &Arc<Self>, mapa: i32, chave: i64) -> Option<i32> {
        let carga = self.carga.get()?;
        let (dados, versao, repo) = (self.dados_vigentes(carga), carga.versao, self.repo.clone());
        let inicio = std::time::Instant::now();
        let mundo = tokio::task::spawn_blocking(move || {
            let mut mundo = WorldInstance::new(mapa, dados, repo);
            mundo.sem_voo = sem_voo_do_mapa(versao, mapa);
            mundo.init_spawns();
            mundo
        })
        .await
        .ok()?;
        let (_, servidor) = Self::preparar_mapa_iniciado(mundo, versao).await;
        if let Some(envio) = self.trocas.get() {
            servidor.ligar_trocas(envio.clone());
        }
        if let Some(eu) = self.eu.get() {
            servidor.ligar_roteador(eu.clone());
        }
        let id = {
            let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
            let id = PRIMEIRA_COPIA + c.proximo;
            c.proximo += 1;
            c.por_chave.insert((mapa, chave), id);
            c.copias.insert(id, CopiaDeMasmorra { mapa, chave, criada: std::time::Instant::now(), vazia_desde: None });
            id
        };
        servidor.definir_id_de_rota(id);
        servidor.mundo().write().await.compartilhar_grupos(Arc::clone(&self.grupos));
        self.mapas.write().unwrap_or_else(|e| e.into_inner()).insert(id, servidor);
        tracing::info!(mapa, chave, id, ms = inicio.elapsed().as_millis() as u64, "masmorra: cópia nova");
        Some(id)
    }

    /// B349 — o batimento das cópias (`instance_manager::Heartbeat`, a cada 10 s): vazia há mais de 20 min, sai da
    /// memória; viva há mais de 4 h, a chave deixa de apontar para ela (quem entrar de novo ganha uma cópia nova).
    async fn batimento_das_copias(self: &Arc<Self>) {
        let ids: Vec<i32> = self.copias.lock().unwrap_or_else(|e| e.into_inner()).copias.keys().copied().collect();
        for id in ids {
            let Some(servidor) = self.mapa(id) else { continue };
            let com_dono = self.donos.read().await.values().any(|m| *m == id);
            let vazia = !com_dono && servidor.vazio().await;
            let liberar = {
                let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
                let Some(x) = c.copias.get_mut(&id) else { continue };
                if vazia {
                    let desde = *x.vazia_desde.get_or_insert_with(std::time::Instant::now);
                    desde.elapsed().as_secs() > OCIOSA_S
                } else {
                    x.vazia_desde = None;
                    false
                }
            };
            let _transicao = self.presenca.write().await;
            {
                let mut c = self.copias.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(x) = c.copias.get(&id) {
                    let (mapa, chave, velha) = (x.mapa, x.chave, x.criada.elapsed().as_secs() >= VIDA_S);
                    if (liberar || velha) && c.por_chave.get(&(mapa, chave)) == Some(&id) {
                        c.por_chave.remove(&(mapa, chave));
                    }
                }
                if liberar {
                    if let Some(x) = c.copias.remove(&id) {
                        let chave = x.chave;
                        c.chave_do_grupo.retain(|_, k| *k != chave);
                    }
                }
            }
            if liberar {
                self.mapas.write().unwrap_or_else(|e| e.into_inner()).remove(&id);
                servidor.descarregar().await;
                tracing::info!(id, "masmorra: cópia liberada (vazia há 20 min)");
            }
        }
    }

    pub fn ligar_trocas(self: &Arc<Self>) {
        let (envio, mut fila) = mpsc::unbounded_channel::<crate::bus_server::PedidoDeTroca>();
        let copias = Arc::clone(self);
        tokio::spawn(async move {
            let mut t = tokio::time::interval(std::time::Duration::from_secs(BATIMENTO_DAS_COPIAS_S));
            loop {
                t.tick().await;
                copias.batimento_das_copias().await;
            }
        });
        let conferir = Arc::clone(self);
        tokio::spawn(async move {
            let mut t = tokio::time::interval(std::time::Duration::from_secs(CONFERE_A_CADA_S));
            loop {
                t.tick().await;
                conferir.conferir_quem_esta_nas_copias().await;
            }
        });
        let _ = self.trocas.set(envio.clone());
        let _ = self.eu.set(Arc::downgrade(self));
        for (_, mapa) in self.todos() {
            mapa.ligar_trocas(envio.clone());
            mapa.ligar_roteador(Arc::downgrade(self));
        }
        let este = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(p) = fila.recv().await {
                este.trocar(p).await;
            }
        });
    }

    /// Leva o jogador a `pos` do mapa `mundo` — o mesmo caminho do teleporte de missão.
    pub async fn transportar(&self, roleid: i32, mundo: i32, pos: pw_core::Vector3) {
        if let Some(atual) = self.mapa_de(roleid).await.and_then(|m| self.mapa(m)) {
            atual.transportar(roleid, mundo, pos).await;
        }
    }

    async fn trocar(self: &Arc<Self>, p: crate::bus_server::PedidoDeTroca) {
        // B349 — a masmorra vai à cópia da chave (montada antes de pegar a guarda: é pesado).
        let destino_id = if self.mapa_ligado(p.mundo) {
            let chave = self.chave_de_masmorra(p.roleid, p.mundo).await;
            match self.destino_de_entrada(p.mundo, chave).await {
                DestinoDeEntrada::Mapa(m) | DestinoDeEntrada::Copia(m) => m,
                DestinoDeEntrada::Recusada => {
                    warn!("mundo: {} não entra na masmorra {} — cópia cheia ou não montada", p.roleid, p.mundo);
                    return;
                }
            }
        } else {
            p.mundo
        };
        let _transicao = self.presenca.write().await;
        let controle=self.repo.controle_de_gravacao(p.roleid);
        let _guarda=controle.alterar().await;
        if !self.mapa_ligado(p.mundo) {
            warn!("mundo: {} pediu o mapa {}, desligado pelo painel — troca recusada", p.roleid, p.mundo);
            return;
        }
        // B350 — mapa de outro servidor de mundo (outro contêiner): a sessão troca de servidor pelo link.
        if self.mapa(destino_id).is_none() && self.remoto(p.mundo) {
            if let Some(origem) = self.mapa_de(p.roleid).await.and_then(|m| self.mapa(m)) {
                if origem.sair_para_outro_servidor(p.roleid, p.mundo, p.pos).await {
                    self.donos.write().await.remove(&p.roleid);
                    return;
                }
            }
        }
        let Some(destino) = self.mapa(destino_id) else {
            warn!(
                "mundo: {} pediu o mapa {}, que este processo não serve ({:?})",
                p.roleid,
                p.mundo,
                self.mapas()
            );
            return;
        };
        let Some(origem) = self
            .mapa_de(p.roleid)
            .await
            .and_then(|m| self.mapa(m))
        else {
            return;
        };
        let Some(vindo) = origem.retirar_para_troca(p.roleid).await else {
            return;
        };
        let envio=vindo.envio();
        let mascote=vindo.mascote.clone();
        match destino.receber_de_outro_mapa(p.roleid,vindo,p.pos).await {
            Ok(())=> {
                let (saiu,resultado)=origem.concluir_troca_na_origem(p.roleid,mascote.clone()).await;
                self.donos.write().await.insert(p.roleid,destino_id);
                if saiu {destino.encerrar_apos_transferencia(p.roleid,&envio,resultado).await;}
                // B358 — `PostSwitchServer`: o mascote de combate volta no mapa novo.
                else if let Some(m)=mascote.as_ref() {destino.reinvocar_depois_da_troca(p.roleid,m.slot,Some(m.para_a_jaula())).await;}
                // B359 — o grupo segue: o cliente que chegou recebe o grupo de novo.
                if !saiu {destino.reenviar_grupo(p.roleid).await;}
            },
            Err((vindo,false))=> {origem.restaurar_troca(p.roleid,vindo).await;},
            Err((vindo,true))=> {
                // Não restaurar por inferência: o commit pode ter ocorrido. Fotografia
                // congelada e mesma sessão ficam reservadas até confirmar o mesmo destino.
                let este=Arc::clone(self);
                let origem=Arc::clone(&origem);
                let destino=Arc::clone(&destino);
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        let _transicao=este.presenca.write().await;
                        let controle=este.repo.controle_de_gravacao(p.roleid);
                        let _guarda=controle.alterar().await;
                        if destino.receber_de_outro_mapa(p.roleid,vindo.clone(),p.pos).await.is_ok() {
                            let (saiu,resultado)=origem.concluir_troca_na_origem(p.roleid,mascote.clone()).await;
                            este.donos.write().await.insert(p.roleid,destino_id);
                            if saiu {destino.encerrar_apos_transferencia(p.roleid,&envio,resultado).await;}
                            break;
                        }
                    }
                });
            }
        }
    }

    /// Os ids dos mapas servidos, em ordem.
    pub fn mapas(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.mapas.read().unwrap_or_else(|e| e.into_inner()).keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// Ausência é observação deste processo, nunca autorização para editar offline.
    pub async fn consultar_administrativamente(&self, roleid: i32) -> serde_json::Value {
        let Ok(_leitura) = self.presenca.try_read() else {
            return serde_json::json!({"presenca":"em_transicao"});
        };
        let Some(mapa) = self.mapa_de(roleid).await else {
            // A queda do link esquece sessões, mas pode deixar uma entidade residual
            // (`atender`, abaixo). Ausência de roteamento não prova ausência em memória.
            for (_, servidor) in self.todos() {
                if servidor.mundo().read().await.players.contains_key(&(roleid as i64)) {
                    return serde_json::json!({"presenca":"inconsistente","motivo":"entidade_sem_roteamento"});
                }
            }
            return serde_json::json!({"presenca":"ausente"});
        };
        let Some(servidor) = self.mapa(mapa) else {
            return serde_json::json!({"presenca":"em_transicao"});
        };
        let ficha = {
            let mundo = servidor.mundo().read().await;
            mundo.players.get(&(roleid as i64)).map(|j| crate::administracao::FichaViva::do_jogador(j, mapa))
        };
        if servidor.tem_sessao(roleid).await && ficha.is_some() {
            serde_json::json!({"presenca":"online","ficha":ficha})
        } else {
            serde_json::json!({"presenca":"em_transicao"})
        }
    }

    pub async fn resumo_administrativo(&self) -> serde_json::Value {
        let Ok(_leitura) = self.presenca.try_read() else {
            return serde_json::json!({"presenca":"em_transicao"});
        };
        let donos = self.donos.read().await.clone();
        let mut mapas = Vec::new();
        for (mapa, servidor) in self.todos() {
            let ids: Vec<i32> = {
                let mundo = servidor.mundo().read().await;
                mundo.players.keys().filter_map(|id| i32::try_from(*id).ok()).collect()
            };
            let mut online = 0;
            for id in ids {
                if donos.get(&id) == Some(&mapa) && servidor.tem_sessao(id).await { online += 1; }
            }
            mapas.push(serde_json::json!({"mapa":mapa,"jogadores_online":online,"ligado":self.mapa_ligado(mapa)}));
        }
        // Ligados pelo painel e ainda em carga (B183): aparecem, sem jogador.
        let mut em_carga: Vec<i32> = self.em_carga().iter().copied().collect();
        em_carga.sort_unstable();
        for mapa in em_carga {
            mapas.push(serde_json::json!({"mapa":mapa,"jogadores_online":0,"ligado":true,"carregando":true}));
        }
        // Os que este processo sabe montar (têm dados): o painel só oferece ligar esses.
        let mut carregaveis: Vec<i32> = self.carga.get()
            .map(|c| self.dados_vigentes(c).pastas_de_mapa.keys().copied().collect()).unwrap_or_default();
        carregaveis.sort_unstable();
        let segundos = |t: std::time::SystemTime| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let iniciado_em = segundos(self.iniciado_em);
        // B331: `dados_em` = quando os arquivos do config foram lidos pela última vez (a partida ou a recarga).
        let (dados_em, recarga) = {
            let r = self.recarga.lock().unwrap_or_else(|e| e.into_inner());
            (r.em.map(segundos).unwrap_or(iniciado_em), serde_json::json!({"em_andamento":r.em_andamento,"falhas":r.falhas}))
        };
        serde_json::json!({"presenca":"observada","mapas":mapas,"taxas":self.taxas(),"carregaveis":carregaveis,
            "iniciado_em":iniciado_em,"dados_em":dados_em,"recarga":recarga})
    }

    /// B331: os dados em uso — os da última recarga aplicada, ou os da partida.
    fn dados_vigentes(&self, carga: &CargaDeMapas) -> Arc<pw_data_loader::GameDataManager> {
        let r = self.recarga.lock().unwrap_or_else(|e| e.into_inner());
        r.dados.clone().unwrap_or_else(|| Arc::clone(&carga.dados))
    }

    /// B331: relê os arquivos do `config` sem reiniciar nem desconectar ninguém — o mesmo
    /// `GameDataManager::load_from_directory` da partida (`main.rs`), em segundo plano (a leitura do 1.5.5 passa do
    /// prazo do canal administrativo). Com falha nova (arquivo que lia na partida e agora não lê), nada muda e as falhas
    /// ficam no resumo. Aplicada: cada mundo passa a usar os dados novos (`WorldInstance::data_manager`) e os jogadores
    /// online têm os atributos refeitos (`refazer_atributos`: passivas, equipamento, nível). O que já nasceu do
    /// `npcgen` continua como estava até reiniciar; tabelas, catálogos, serviços, distritos, portais e destinos valem
    /// na hora. `Err` com o código: `sem_carga`, `sem_pasta` ou `recarga_em_andamento`.
    pub fn recarregar_em_segundo_plano(self: &Arc<Self>) -> Result<(), &'static str> {
        let carga = self.carga.get().ok_or("sem_carga")?;
        let pasta = carga.pasta.clone().ok_or("sem_pasta")?;
        let conhecidas = carga.falhas_na_partida.clone();
        {
            let mut r = self.recarga.lock().unwrap_or_else(|e| e.into_inner());
            if r.em_andamento {
                return Err("recarga_em_andamento");
            }
            r.em_andamento = true;
        }
        let eu = Arc::clone(self);
        tokio::spawn(async move {
            let lido = tokio::task::spawn_blocking(move || {
                let mut d = pw_data_loader::GameDataManager::new();
                let rel = d.load_from_directory(&pasta);
                (d, rel)
            })
            .await;
            let terminar = |falhas: Vec<String>| {
                let mut r = eu.recarga.lock().unwrap_or_else(|e| e.into_inner());
                r.em_andamento = false;
                r.falhas = falhas;
            };
            let (novos, rel) = match lido {
                Ok(x) => x,
                Err(e) => {
                    tracing::warn!("admin: recarga dos dados caiu: {e}");
                    terminar(vec![format!("leitura: {e}")]);
                    return;
                }
            };
            let novas: Vec<String> = rel.falhas.iter().filter(|f| !conhecidas.contains(&f.arquivo))
                .map(|f| format!("{}: {}", f.arquivo, f.motivo)).collect();
            if !novas.is_empty() {
                tracing::warn!(falhas = ?novas, "admin: recarga recusada — os dados em uso continuam");
                terminar(novas);
                return;
            }
            let novos = Arc::new(novos);
            let mut jogadores = 0usize;
            for (_, servidor) in eu.todos() {
                let mut m = servidor.mundo().write().await;
                m.data_manager = Arc::clone(&novos);
                let ids: Vec<i64> = m.players.keys().copied().collect();
                jogadores += ids.len();
                for id in ids {
                    m.refazer_atributos(id);
                }
            }
            {
                let mut r = eu.recarga.lock().unwrap_or_else(|e| e.into_inner());
                r.dados = Some(novos);
                r.em = Some(std::time::SystemTime::now());
                r.em_andamento = false;
                r.falhas.clear();
            }
            tracing::info!(jogadores, lidos = rel.lidos.len(), "admin: recarga dos dados aplicada, sem reiniciar");
        });
        Ok(())
    }

    /// Personagens da conta atendidos por este processo, com o mapa dono de cada um.
    async fn personagens_da_conta(&self, conta: i32) -> Vec<(i32, i32)> {
        let donos = self.donos.read().await.clone();
        let mut achados = Vec::new();
        for (mapa, servidor) in self.todos() {
            let ids: Vec<i32> = {
                let mundo = servidor.mundo().read().await;
                mundo.players.iter().filter(|(_, p)| p.conta_id == conta)
                    .filter_map(|(id, _)| i32::try_from(*id).ok()).collect()
            };
            achados.extend(ids.into_iter().filter(|id| donos.get(id) == Some(&mapa)).map(|id| (mapa, id)));
        }
        achados
    }

    /// Painel (E4, B175): desconecta a conta deste processo. Guarda de presença em
    /// escrita, como o logout em `entregar`, para não cruzar com entrada/troca.
    /// Quantos personagens têm sessão em algum mapa deste processo.
    pub async fn quantos_online(&self) -> usize {
        let mut n = 0;
        for (_, m) in self.todos() {
            n += m.personagens_com_sessao().await.len();
        }
        n
    }

    /// Reiniciar pelo painel (B249): todos saem pelo caminho do logout (`expulsar_pelo_painel`: trava de gravação,
    /// fotografia salva antes de liberar), com a guarda de presença em escrita para ninguém entrar no meio.
    pub async fn desconectar_todos(&self) -> usize {
        let _transicao = self.presenca.write().await;
        let mut desconectados = 0;
        for (_, m) in self.todos() {
            for role in m.personagens_com_sessao().await {
                if m.expulsar_pelo_painel(role).await {
                    desconectados += 1;
                }
            }
        }
        desconectados
    }

    pub async fn desconectar_conta(&self, conta: i32) -> serde_json::Value {
        let _transicao = self.presenca.write().await;
        let mut desconectados = 0;
        for (mapa, role) in self.personagens_da_conta(conta).await {
            if let Some(s) = self.mapa(mapa) { if s.expulsar_pelo_painel(role).await { desconectados += 1; } }
        }
        serde_json::json!({"estado":"aplicado","tipo":"desconectar","conta_id":conta,"desconectados":desconectados})
    }

    /// Painel (E5, B179): edita um personagem deste realm. Modelo da reforma (§6.2 item 2):
    /// online → o mapa dono aplica na memória e avisa o cliente; offline → só dinheiro, no
    /// banco, com a guarda de presença em leitura (entrada pega em escrita, então ninguém
    /// entra no meio) e a trava de gravação do personagem. Saída pendente = em transição.
    pub async fn editar_personagem(&self, roleid: i32, edicao: crate::bus_server::EdicaoDePersonagem) -> serde_json::Value {
        use crate::bus_server::EdicaoDePersonagem as E;
        // Limites que valem online e offline: teto de nível do realm e cultivo da versão.
        let Some(bus) = self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) else {
            return serde_json::json!({"estado":"falha","codigo":"sem_mapa"});
        };
        let maximo = bus.nivel_maximo().await;
        match edicao {
            E::Nivel(alvo) if !(2..=maximo).contains(&alvo) => {
                return serde_json::json!({"estado":"falha","codigo":"nivel_invalido","nivel_maximo":maximo});
            }
            E::Cultivo(v) if !crate::progressao::cultivo_valido(bus.versao(), v) => {
                return serde_json::json!({"estado":"falha","codigo":"cultivo_invalido"});
            }
            E::Posicao { mapa, x, y, z } => return self.mover_pelo_painel(roleid, mapa, x, y, z).await,
            E::Item { tid, .. } if !bus.item_existe(tid).await => {
                return serde_json::json!({"estado":"falha","codigo":"item_inexistente","item":tid});
            }
            _ => {}
        }
        let _comando = self.presenca.read().await;
        let controle = self.repo.controle_de_gravacao(roleid);
        let _guarda = controle.alterar().await;
        if let Some(mapa) = self.mapa_de(roleid).await {
            let editado = match self.mapa(mapa) { Some(s) => s.editar_pelo_painel(roleid, edicao).await, None => None };
            return match editado {
                Some(dados) if !dados["erro"].is_null() => {
                    serde_json::json!({"estado":"falha","codigo":dados["erro"],"nivel":dados["nivel"]})
                }
                Some(mut dados) => {
                    dados.as_object_mut().map(|o| o.remove("erro"));
                    dados["estado"] = serde_json::json!("aplicado");
                    dados["presenca"] = serde_json::json!("online");
                    dados["mapa"] = serde_json::json!(mapa);
                    dados
                }
                None => serde_json::json!({"estado":"falha","codigo":"em_transicao"}),
            };
        }
        match edicao {
            E::Experiencia { .. } => serde_json::json!({"estado":"falha","codigo":"precisa_estar_online"}),
            E::Daimon(e) => match bus.daimon_offline(roleid, e).await {
                Ok(antes) => serde_json::json!({"estado":"salvo","presenca":"offline","daimon":e,"antes":antes}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::DaimonExp(n) => match bus.daimon_exp_offline(roleid, n).await {
                Ok((antes, depois)) => serde_json::json!({"estado":"salvo","presenca":"offline","nivel_antes":antes,"nivel":depois}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            // Já tratada no começo (`mover_pelo_painel`).
            E::Posicao { .. } => serde_json::json!({"estado":"falha","codigo":"edicao_invalida"}),
            E::RemoverItem { recipiente, slot, tid, quantidade } => match bus.remover_item_offline(roleid, recipiente, slot, tid, quantidade).await {
                Ok(n) => serde_json::json!({"estado":"salvo","presenca":"offline","item":tid,"removidos":n,"slot":slot}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::Habilidade { id, nivel } => match bus.habilidade_offline(roleid, id, nivel).await {
                Ok(antes) => serde_json::json!({"estado":"salvo","presenca":"offline","habilidade":id,"nivel":nivel,"nivel_antes":antes}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::OrganizarBolsa => match bus.organizar_bolsa_offline(roleid).await {
                Ok(n) => serde_json::json!({"estado":"salvo","presenca":"offline","trocas":n}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::MoverItem { de, slot_de, tid, para, slot_para } => match bus.mover_item_offline(roleid, de, slot_de, tid, para, slot_para).await {
                Ok(()) => serde_json::json!({"estado":"salvo","presenca":"offline","item":tid,"de":slot_de,"para":slot_para}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::Item { tid, quantidade, destino: Some((recipiente, slot)) } => match bus.criar_item_no_slot(roleid, tid, quantidade, recipiente, slot, false).await {
                Ok(mut v) => { v["estado"] = serde_json::json!("salvo"); v["presenca"] = serde_json::json!("offline"); v }
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::DarMascote(ovo) => match bus.dar_mascote(roleid, ovo, false).await {
                Ok(mut v) => { v["estado"] = serde_json::json!("salvo"); v["presenca"] = serde_json::json!("offline"); v }
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::Item { tid, quantidade, destino: None } => match bus.dar_item_offline(roleid, tid, quantidade).await {
                Ok((entrou, slot)) => serde_json::json!({"estado":"salvo","presenca":"offline","item":tid,"entrou":entrou,"slot":slot}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::PontosLivres(n) => Self::offline(roleid, "pontos", self.repo.dar_pontos_offline(roleid, n).await,
                |p| serde_json::json!({"pontos_livres": p})),
            E::Nivel(alvo) => match self.repo.subir_nivel_offline(roleid, alvo, maximo).await {
                // `None`: inexistente ou o alvo não passa do nível gravado.
                Ok(None) => serde_json::json!({"estado":"falha","codigo":"nivel_invalido_ou_personagem_inexistente"}),
                r => Self::offline(roleid, "nivel", r,
                    |(n, p)| serde_json::json!({"nivel": n, "pontos_livres": p})),
            },
            E::Atributos(alvo) => match self.repo.definir_atributos_offline(roleid, alvo, bus.piso_da_restauracao()).await {
                // `None`: inexistente, abaixo do piso ou soma acima do total.
                Ok(None) => serde_json::json!({"estado":"falha","codigo":"atributos_invalidos_ou_personagem_inexistente"}),
                r => Self::offline(roleid, "atributos", r,
                    |[f, a, v, e, p]| serde_json::json!({"forca":f,"agilidade":a,"vitalidade":v,"energia":e,"pontos_livres":p})),
            },
            E::Cultivo(v) => Self::offline(roleid, "cultivo",
                self.repo.definir_cultivo_offline(roleid, v).await.map(|ok| ok.then_some(v)),
                |c| serde_json::json!({"cultivo": c})),
            E::Dinheiro(delta) => match self.repo.ajustar_dinheiro_offline(roleid, delta).await {
                Ok(Some(dinheiro)) => serde_json::json!({"estado":"salvo","presenca":"offline","dinheiro":dinheiro.to_string()}),
                Ok(None) => serde_json::json!({"estado":"falha","codigo":"personagem_inexistente"}),
                Err(e) => {
                    tracing::warn!("painel: dinheiro offline de {roleid} não gravou: {e}");
                    serde_json::json!({"estado":"falha","codigo":"banco_indisponivel"})
                }
            },
        }
    }

    /// Painel (E6, B194): edita as propriedades de um item, com a guarda de presença e a trava
    /// de gravação como as demais edições. Online pelo mapa dono (grava e reenvia a ficha do
    /// item); offline grava no banco. Regras em [`crate::bus_server::item_editado`].
    pub async fn editar_item(&self, roleid: i32, recipiente: pw_core::ContainerType, slot: u16, tid: u32,
        e: &crate::bus_server::item_editado::EdicaoDeItem) -> serde_json::Value {
        let Some(bus) = self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) else {
            return serde_json::json!({"estado":"falha","codigo":"sem_mapa"});
        };
        let _comando = self.presenca.read().await;
        let controle = self.repo.controle_de_gravacao(roleid);
        let _guarda = controle.alterar().await;
        if let Some(mapa) = self.mapa_de(roleid).await {
            let editado = match self.mapa(mapa) { Some(s) => s.editar_item_pelo_painel(roleid, recipiente, slot, tid, e).await, None => None };
            return match editado {
                Some(d) if !d["erro"].is_null() => serde_json::json!({"estado":"falha","codigo":d["erro"]}),
                Some(_) => serde_json::json!({"estado":"aplicado","presenca":"online","mapa":mapa,"item":tid,"slot":slot}),
                None => serde_json::json!({"estado":"falha","codigo":"em_transicao"}),
            };
        }
        match bus.editar_item_no_banco(roleid, recipiente, slot, tid, e).await {
            Ok(_) => {
                tracing::info!(roleid, ?recipiente, slot, tid, "painel: item editado (offline)");
                serde_json::json!({"estado":"salvo","presenca":"offline","item":tid,"slot":slot})
            }
            Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
        }
    }

    /// Painel (E6, B185): leva o personagem a `(x, y, z)` do `mapa`. Destino: mapa carregado e
    /// ligado neste processo (senão `mapa_indisponivel`), `(x, z)` dentro do terreno dele
    /// (`fora_do_mapa`); `y` ausente = o chão, e abaixo do chão sobe para ele — a regra de quem
    /// chega a um mapa (`if (pos.y < height) pos.y = height`, `global_message.cpp:100-101`).
    /// Online: o mesmo `transportar` do GM e da missão — no mapa `NOTIFY_HOSTPOS`; noutro, a
    /// troca de mapa, que grava na hora (a troca roda depois, na fila: resposta `troca: true`).
    /// Offline: grava mapa e posição sob a guarda de presença e a trava de gravação.
    async fn mover_pelo_painel(&self, roleid: i32, mapa: i32, x: f32, y: Option<f32>, z: f32) -> serde_json::Value {
        let Some(destino) = self.mapa(mapa).filter(|_| self.mapa_ligado(mapa)) else {
            return serde_json::json!({"estado":"falha","codigo":"mapa_indisponivel","mapa":mapa});
        };
        let y = match (destino.chao_em(x, z).await, y) {
            (Err(()), _) => return serde_json::json!({"estado":"falha","codigo":"fora_do_mapa","mapa":mapa}),
            (Ok(Some(chao)), Some(y)) => y.max(chao),
            (Ok(Some(chao)), None) => chao,
            (Ok(None), Some(y)) => y,
            (Ok(None), None) => return serde_json::json!({"estado":"falha","codigo":"altura_obrigatoria","mapa":mapa}),
        };
        let pos = pw_core::Vector3::new(x, y, z);
        let _comando = self.presenca.read().await;
        let controle = self.repo.controle_de_gravacao(roleid);
        let _guarda = controle.alterar().await;
        let posicao = serde_json::json!({"mapa": mapa, "x": x, "y": y, "z": z});
        if let Some(atual) = self.mapa_de(roleid).await {
            let Some(origem) = self.mapa(atual) else {
                return serde_json::json!({"estado":"falha","codigo":"em_transicao"});
            };
            if !origem.tem_sessao(roleid).await {
                return serde_json::json!({"estado":"falha","codigo":"em_transicao"});
            }
            origem.transportar(roleid, mapa, pos).await;
            tracing::info!(roleid, de = atual, mapa, ?pos, "painel: personagem movido (online)");
            return serde_json::json!({"estado":"aplicado","presenca":"online","posicao":posicao,"troca":atual != mapa});
        }
        match self.repo.gravar_posicao_offline(roleid, mapa, pos).await {
            Ok(true) => {
                tracing::info!(roleid, mapa, ?pos, "painel: personagem movido (offline)");
                serde_json::json!({"estado":"salvo","presenca":"offline","posicao":posicao})
            }
            Ok(false) => serde_json::json!({"estado":"falha","codigo":"personagem_inexistente"}),
            Err(e) => {
                tracing::warn!("painel: posição offline de {roleid} não gravou: {e}");
                serde_json::json!({"estado":"falha","codigo":"banco_indisponivel"})
            }
        }
    }

    /// Resposta de uma edição offline (B182): `salvo` com os campos novos, `personagem_inexistente`
    /// ou `banco_indisponivel`.
    fn offline<T, E: std::fmt::Display>(roleid: i32, o_que: &str, r: Result<Option<T>, E>, campos: impl FnOnce(T) -> serde_json::Value) -> serde_json::Value {
        match r {
            Ok(Some(v)) => {
                let mut j = campos(v);
                j["estado"] = serde_json::json!("salvo");
                j["presenca"] = serde_json::json!("offline");
                j
            }
            Ok(None) => serde_json::json!({"estado":"falha","codigo":"personagem_inexistente"}),
            Err(e) => {
                tracing::warn!("painel: {o_que} offline de {roleid} não gravou: {e}");
                serde_json::json!({"estado":"falha","codigo":"banco_indisponivel"})
            }
        }
    }

    /// Painel (E6, B186): inventário de um personagem deste realm (do banco, com nomes).
    pub async fn inventario_do_painel(&self, roleid: i32) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.inventario_do_painel(roleid).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E6, B197): a jaula. Pelo mapa dono quando o personagem está em jogo (é ele que sabe
    /// qual mascote está invocado); senão pelo padrão.
    /// Painel (E6, B215): o Daimon vestido — do mapa onde o dono está, senão do banco.
    pub async fn daimon_do_painel(&self, roleid: i32) -> serde_json::Value {
        let dono = match self.mapa_de(roleid).await { Some(m) => self.mapa(m), None => None };
        match dono.or_else(|| self.mapa(self.padrao)).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.daimon_do_painel(roleid).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    pub async fn mascotes_do_painel(&self, roleid: i32) -> serde_json::Value {
        let dono = match self.mapa_de(roleid).await { Some(m) => self.mapa(m), None => None };
        match dono.or_else(|| self.mapa(self.padrao)).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.mascotes_do_painel(roleid).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E6, B197): editar ou libertar um mascote da jaula, com a guarda de presença e a
    /// trava de gravação. Regras em [`crate::bus_server::mascote_editado`].
    pub async fn editar_mascote(&self, roleid: i32, slot: u16, tid: i32,
        e: &crate::bus_server::mascote_editado::EdicaoDeMascote) -> serde_json::Value {
        let Some(bus) = self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) else {
            return serde_json::json!({"estado":"falha","codigo":"sem_mapa"});
        };
        let _comando = self.presenca.read().await;
        let controle = self.repo.controle_de_gravacao(roleid);
        let _guarda = controle.alterar().await;
        if let Some(mapa) = self.mapa_de(roleid).await {
            let feito = match self.mapa(mapa) { Some(s) => s.mascote_pelo_painel(roleid, slot, tid, e).await, None => None };
            return match feito {
                Some(d) if !d["erro"].is_null() => serde_json::json!({"estado":"falha","codigo":d["erro"]}),
                Some(_) => serde_json::json!({"estado":"aplicado","presenca":"online","mapa":mapa,"slot":slot}),
                None => serde_json::json!({"estado":"falha","codigo":"em_transicao"}),
            };
        }
        match bus.mascote_no_banco(roleid, slot, tid, e).await {
            Ok(r) => {
                tracing::info!(roleid, slot, tid, libertado = r.is_none(), "painel: mascote editado (offline)");
                serde_json::json!({"estado":"salvo","presenca":"offline","slot":slot})
            }
            Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
        }
    }

    /// Painel (E6, B198): as listas de missão. Pelo mapa dono quando o personagem está em jogo (a
    /// memória é a verdade); senão pelo padrão, do banco.
    pub async fn missoes_do_painel(&self, roleid: i32, pagina: usize, busca: Option<&str>) -> serde_json::Value {
        let dono = match self.mapa_de(roleid).await { Some(m) => self.mapa(m), None => None };
        match dono.or_else(|| self.mapa(self.padrao)).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.missoes_do_painel(roleid, pagina, busca).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E6, B198): missões de topo do `tasks.data` do realm por nome ou id.
    pub async fn buscar_missoes(&self, texto: &str) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.buscar_missoes(texto).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E6, B198): dar, concluir, cancelar ou esquecer uma missão, com a guarda de presença
    /// e a trava de gravação. Regras em [`crate::bus_server::missao_editada`].
    pub async fn editar_missao(&self, roleid: i32, e: crate::bus_server::missao_editada::EdicaoDeMissao) -> serde_json::Value {
        let Some(bus) = self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) else {
            return serde_json::json!({"estado":"falha","codigo":"sem_mapa"});
        };
        let _comando = self.presenca.read().await;
        let controle = self.repo.controle_de_gravacao(roleid);
        let _guarda = controle.alterar().await;
        if let Some(mapa) = self.mapa_de(roleid).await {
            let feito = match self.mapa(mapa) { Some(s) => s.missao_pelo_painel(roleid, e).await, None => None };
            return match feito {
                Some(d) if !d["erro"].is_null() => serde_json::json!({"estado":"falha","codigo":d["erro"],"missao":e.id}),
                Some(d) => serde_json::json!({"estado":"aplicado","presenca":"online","mapa":mapa,"missao":e.id,
                    "acao":e.acao,"concluida":d["concluida"]}),
                None => serde_json::json!({"estado":"falha","codigo":"em_transicao"}),
            };
        }
        match bus.missao_no_banco(roleid, e).await {
            Ok(concluida) => serde_json::json!({"estado":"salvo","presenca":"offline","missao":e.id,"acao":e.acao,"concluida":concluida}),
            Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo,"missao":e.id}),
        }
    }

    /// Painel (E6, B189): o item de um slot com os dados da dica.
    pub async fn detalhe_do_item(&self, roleid: i32, recipiente: pw_core::ContainerType, slot: u16) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.detalhe_do_item(roleid, recipiente, slot).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Estúdio (B373): o item `tid` gerado como drop, para a prévia (não grava).
    pub async fn item_de_exemplo(&self, tid: u32) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.item_de_exemplo(tid).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (B209): como os efeitos pedidos entram na peça `item_id` (dados do realm).
    pub async fn efeitos_para_item(&self, item_id: u32, ids: &[u32]) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.efeitos_para_item(item_id, ids).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E6, B186): busca de itens por nome ou id no `elements.data` do realm.
    /// Painel (B216): o catálogo de mascotes (ovos), por nome/id, tipo e ambiente.
    pub async fn buscar_mascotes(&self, texto: &str, tipo: Option<i32>, ambiente: Option<&str>) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.buscar_mascotes(texto, tipo, ambiente).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    pub async fn buscar_itens(&self, texto: &str, categoria: Option<&str>) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.buscar_itens(texto, categoria).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E4, B175): o gold da conta mudou; reenvia o saldo a quem está online aqui.
    pub async fn atualizar_cash_da_conta(&self, conta: i32) -> serde_json::Value {
        let _comando = self.presenca.read().await;
        let mut atualizados = 0;
        for (mapa, role) in self.personagens_da_conta(conta).await {
            if let Some(s) = self.mapa(mapa) { if s.reenviar_cash(role).await { atualizados += 1; } }
        }
        serde_json::json!({"estado":"aplicado","tipo":"atualizar_cash","conta_id":conta,"atualizados":atualizados})
    }

    /// Mapa e posição de um jogador atendido por este processo (comandos de GM).
    pub(crate) async fn posicao_de(&self, roleid: i32) -> Option<(i32, pw_core::Vector3)> {
        let mapa = self.mapa_de(roleid).await?;
        let pos = self.mapa(mapa)?.posicao_do_jogador(roleid).await?;
        Some((mapa, pos))
    }

    /// B359 — manda ao jogador no mapa em que ele está (os avisos de grupo).
    pub async fn enviar_ao_jogador(&self, roleid: i32, data: Vec<u8>) -> bool {
        match self.mapa_de(roleid).await.and_then(|m| self.mapa(m)) {
            Some(s) => s.enviar_ao_jogador(roleid, data).await,
            None => false,
        }
    }

    /// Em que mapa este jogador está sendo atendido agora.
    pub async fn mapa_de(&self, roleid: i32) -> Option<i32> {
        self.donos.read().await.get(&roleid).copied()
    }

    /// Aceita conexões de daemons de link até a escuta cair.
    pub async fn executar(self: Arc<Self>, escuta: BusListener) {
        info!(
            "servidor de mundo escutando o barramento pelos mapas {:?}",
            self.mapas()
        );
        loop {
            match escuta.aceitar().await {
                Ok(conexao) => {
                    let este = Arc::clone(&self);
                    tokio::spawn(async move { este.atender(conexao).await });
                }
                Err(e) => {
                    warn!("barramento: falha ao aceitar conexão: {e}");
                    return;
                }
            }
        }
    }

    /// Como [`BusServer::atender`], mas entregando cada mensagem ao mapa do jogador.
    async fn atender(&self, mut conexao: pw_bus::transport::BusConnection) {
        let par = conexao.par().to_string();
        let (envio, mut fila) = mpsc::channel::<BusMessage>(256);
        let mut desta_conexao: Vec<i32> = Vec::new();

        loop {
            tokio::select! {
                entrada = conexao.receber() => {
                    match entrada {
                        Ok(Some(msg)) => {
                            if let BusMessage::EnterWorld { roleid, .. } = &msg {
                                desta_conexao.push(*roleid);
                            }
                            let saiu = match &msg {
                                BusMessage::PlayerLogout { roleid, .. } => Some(*roleid),
                                _ => None,
                            };
                            self.entregar(msg, &envio).await;
                            if let Some(r) = saiu {
                                let ainda_dona=if let Some(m)=self.mapa_de(r).await {
                                    match self.mapa(m) { Some(s) => s.pertence_a(r,&envio).await, None => false }
                                } else {false};
                                if !ainda_dona {desta_conexao.retain(|x|*x!=r);}
                                if let Some(m)=self.mapa_de(r).await {
                                    let tem = match self.mapa(m) { Some(s) => s.tem_sessao(r).await, None => false };
                                    if !tem {self.donos.write().await.remove(&r);}
                                }
                            }
                        }
                        Ok(None) => {
                            debug!("barramento: {par} desconectou");
                            break;
                        }
                        Err(e) => {
                            warn!("barramento: erro lendo de {par}: {e}");
                            break;
                        }
                    }
                }
                saida = fila.recv() => {
                    match saida {
                        Some(msg) => {
                            if let Err(e) = conexao.enviar(msg).await {
                                warn!("barramento: erro escrevendo para {par}: {e}");
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        // A conexão caiu: cada mapa esquece as sessões que vinham por ela.
        let _transicao = self.presenca.write().await;
        for roleid in desta_conexao {
            if let Some(mapa)=self.mapa_de(roleid).await.and_then(|m|self.mapa(m)) {
                mapa.esquecer_sessoes(&[roleid],&envio).await;
            }
        }
    }

    async fn entregar(&self, msg: BusMessage, envio: &EnvioAoCliente) {
        // Mesmo cabeçalho já decodificado por bus_server.rs::SubComando::ler;
        // apenas reconhece logout para a guarda, sem novo layout/pacote de cliente.
        let transicao = matches!(&msg, BusMessage::EnterWorld { .. } | BusMessage::PlayerLogout { .. } | BusMessage::SwitchServerStart { .. })
            || matches!(&msg, BusMessage::ClientToGame { data, .. }
                if data.get(..2) == Some(&crate::comandos::ids::LOGOUT.to_le_bytes()));
        let _presenca = if transicao { Some(self.presenca.write().await) } else { None };
        let _comando = if !transicao { Some(self.presenca.read().await) } else { None };
        let roleid = match &msg {
            BusMessage::EnterWorld { roleid, .. }
            | BusMessage::PlayerLogout { roleid, .. }
            | BusMessage::ClientToGame { roleid, .. }
            | BusMessage::GameToClient { roleid, .. }
            | BusMessage::SetCustomData { roleid, .. }
            | BusMessage::SetCustomDataRe { roleid, .. }
            | BusMessage::SwitchServerStart { roleid, .. }
            | BusMessage::SwitchServerSuccess { roleid, .. } => *roleid,
            BusMessage::ChatSingleCast { dstroleid, .. } => *dstroleid,
        };

        // B350 — a chegada de outro servidor de mundo entra como uma entrada (o mapa gravado pela origem).
        let mapa = if matches!(msg, BusMessage::EnterWorld { .. } | BusMessage::SwitchServerStart { .. }) {
            if let Some(m)=self.mapa_de(roleid).await {
                if match self.mapa(m) { Some(s) => s.tem_sessao(roleid).await, None => false } {
                    warn!("entrada: {roleid} já possui sessão ou saída pendente; rejeitada duplicata");
                    return;
                }
            }
            let mapa = self.mapa_para_entrar(roleid).await;
            // Desligado pelo painel, ou o padrão descarregado (B183): mesma recusa.
            let mut recusada = !self.mapa_ligado(mapa) || !self.serve(mapa);
            // B349 — quem entra gravado numa masmorra volta à cópia da chave dele, ou a uma nova
            // (`instance_userlogin.cpp:110-227`; sem grupo ainda, a chave é a própria).
            let mapa = if recusada {
                mapa
            } else if let Some(eu) = self.eu.get().and_then(|e| e.upgrade()) {
                match eu.destino_de_entrada(mapa, eu.chave_atual(mapa, roleid)).await {
                    DestinoDeEntrada::Mapa(m) | DestinoDeEntrada::Copia(m) => m,
                    DestinoDeEntrada::Recusada => {
                        recusada = true;
                        mapa
                    }
                }
            } else {
                mapa
            };
            if recusada {
                // Mapa desligado pelo painel: a entrada é recusada e o cliente volta ao
                // login (`PlayerLogout` result 2, ver `SAIDA_PELO_PAINEL`). O link já
                // registrou a sessão antes do `EnterWorld` (`pw-link/src/gateway.rs`).
                warn!("entrada: {roleid} iria para o mapa {mapa}, desligado pelo painel — recusada");
                if let BusMessage::EnterWorld { localsid, .. } = msg {
                    let _ = envio.try_send(BusMessage::PlayerLogout {
                        result: crate::bus_server::SAIDA_PELO_PAINEL, roleid, provider_link_id: 0, localsid,
                    });
                }
                return;
            }
            self.donos.write().await.insert(roleid, mapa);
            mapa
        } else {
            match self.mapa_de(roleid).await {
                Some(m) => m,
                None => {
                    debug!("mundo: mensagem de {roleid}, que não entrou em mapa nenhum — ignorada");
                    return;
                }
            }
        };

        match self.mapa(mapa) {
            Some(s) => s.tratar(msg, envio).await,
            None => debug!("mundo: mensagem de {roleid} para o mapa {mapa}, já descarregado — ignorada"),
        }
    }

    pub(crate) async fn apagar_rota(&self,role:i32,mapa:i32) {
        let mut donos=self.donos.write().await;
        if donos.get(&role)==Some(&mapa) {donos.remove(&role);}
    }
    pub(crate) async fn repetir_saida(&self,mapa:&BusServer,role:i32)->bool {
        let _transicao=self.presenca.write().await;
        let controle=self.repo.controle_de_gravacao(role);
        let _guarda=controle.alterar().await;
        mapa.confirmar_saida(role).await
    }

    /// O mapa gravado do personagem, se este processo o serve; senão o padrão, com aviso.
    async fn mapa_para_entrar(&self, roleid: i32) -> i32 {
        match self.repo.mundo_do_personagem(roleid).await {
            // Desligado pelo painel: fica nele, e a entrada é recusada (não vai ao padrão).
            Ok(Some(m)) if self.serve(m) || !self.mapa_ligado(m) => m,
            Ok(Some(m)) => {
                warn!(
                    "mundo: {roleid} está gravado no mapa {m}, que este processo não serve \
                     ({:?}) — vai para o mapa {}",
                    self.mapas(),
                    self.padrao
                );
                self.padrao
            }
            Ok(None) => self.padrao,
            Err(e) => {
                warn!(
                    "mundo: não consegui ler o mapa de {roleid}: {e} — vai para o {}",
                    self.padrao
                );
                self.padrao
            }
        }
    }

    /// Monta um mapa pronto para o roteador: mundo com spawns, laço de tick e eventos.
    pub async fn preparar_mapa(
        mut mundo: WorldInstance,
        versao: pw_protocol::GameVersion,
    ) -> (i32, Arc<BusServer>) {
        mundo.init_spawns();
        Self::preparar_mapa_iniciado(mundo, versao).await
    }

    /// [`Self::preparar_mapa`] com os spawns já feitos.
    async fn preparar_mapa_iniciado(
        mut mundo: WorldInstance,
        versao: pw_protocol::GameVersion,
    ) -> (i32, Arc<BusServer>) {
        mundo.capacidade_do_grupo = capacidade_do_grupo(versao);
        mundo.santuario_conferido_pelo_servidor = pw_protocol::create_world_protocol(versao).santuario_conferido_pelo_servidor();
        let id = mundo.world_id;
        let servidor = Arc::new(crate::server::GameServer::new(mundo));
        let bus = Arc::new(BusServer::new(Arc::clone(&servidor.world), versao));
        bus.ligar_eventos_do_mundo().await;
        // Um pânico no tick de um mapa derruba só a tarefa daquele mapa — e sem este aviso,
        // em silêncio: os outros continuariam, e aquele ficaria parado.
        let tick = tokio::spawn(servidor.run_tick_loop());
        bus.guardar_tique(tick.abort_handle());
        tokio::spawn(async move {
            match tick.await {
                Err(e) if e.is_cancelled() => tracing::info!("mapa {id}: descarregado, tique parado"),
                Err(e) => tracing::error!("mapa {id}: o laço de tick parou: {e}"),
                Ok(()) => {}
            }
        });
        (id, bus)
    }
}

#[cfg(test)]
mod testes_da_masmorra {
    use super::*;
    use std::time::{Duration, Instant};

    /// `aei_filter::Heartbeat`: avisa quem não confere, cancela quem volta a conferir ou morre, expulsa no prazo.
    #[test]
    fn decisao_do_filtro_da_copia() {
        let t = Instant::now();
        let prazo = Some(t + Duration::from_secs(60));
        assert_eq!(conferir(true, false, None, t), Conferencia::Fica);
        assert_eq!(conferir(false, false, None, t), Conferencia::Avisar);
        assert_eq!(conferir(false, true, None, t), Conferencia::Fica, "morto não é avisado");
        assert_eq!(conferir(false, false, prazo, t), Conferencia::Fica);
        assert_eq!(conferir(true, false, prazo, t), Conferencia::Cancelar);
        assert_eq!(conferir(false, true, prazo, t), Conferencia::Cancelar);
        assert_eq!(conferir(false, false, prazo, t + Duration::from_secs(60)), Conferencia::Expulsar);
    }

    /// `ResetInstance`: o dono sobe o contador e a chave do grupo acompanha.
    #[test]
    fn renovar_a_chave_sobe_o_contador_do_dono() {
        let mut c = Copias::default();
        assert_eq!(c.propria(169, 7), 7, "contador 0 é o próprio roleid");
        c.chave_do_grupo.insert(8, 7);
        c.chave_do_grupo.insert(9, 99);
        let nova = c.renovar(169, 7);
        assert_eq!(nova, chave_de(7, 1));
        assert_eq!((c.atual(169, 7), c.atual(169, 8), c.atual(169, 9)), (nova, nova, 99));
        assert_eq!(c.propria(161, 7), 7, "o contador é por mapa");
    }
}
