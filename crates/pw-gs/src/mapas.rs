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
use pw_storage::CharacterRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

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
    /// Mapas sendo montados agora (ligados pelo painel, ainda fora de `mapas`).
    carregando: std::sync::Mutex<std::collections::HashSet<i32>>,
    /// Fila de trocas e o próprio roteador, para ligar os mapas carregados depois.
    trocas: std::sync::OnceLock<mpsc::UnboundedSender<crate::bus_server::PedidoDeTroca>>,
    eu: std::sync::OnceLock<std::sync::Weak<RoteadorDeMapas>>,
}

/// Dados comuns e versão: o que `preparar_mapa` precisa para montar um mapa (B183).
pub struct CargaDeMapas {
    pub dados: Arc<pw_data_loader::GameDataManager>,
    pub versao: pw_protocol::GameVersion,
}

/// `_world_limit.nofly` do `gs.conf` da versão (B133).
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
        Self {
            mapas: std::sync::RwLock::new(mapas.into_iter().collect()),
            padrao,
            donos: RwLock::new(HashMap::new()),
            repo,
            presenca: RwLock::new(()),
            taxas: std::sync::RwLock::new(crate::taxas::Taxas::default()),
            desligados: std::sync::RwLock::new(std::collections::HashSet::new()),
            carga: std::sync::OnceLock::new(),
            carregando: std::sync::Mutex::new(std::collections::HashSet::new()),
            trocas: std::sync::OnceLock::new(),
            eu: std::sync::OnceLock::new(),
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
        self.mapa_ligado(mapa) && self.serve(mapa)
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
                if !carga.dados.pastas_de_mapa.contains_key(&mapa) {
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
        let (dados, versao, repo) = (Arc::clone(&carga.dados), carga.versao, self.repo.clone());
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
    pub fn ligar_trocas(self: &Arc<Self>) {
        let (envio, mut fila) = mpsc::unbounded_channel::<crate::bus_server::PedidoDeTroca>();
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
        let _transicao = self.presenca.write().await;
        let controle=self.repo.controle_de_gravacao(p.roleid);
        let _guarda=controle.alterar().await;
        if !self.mapa_ligado(p.mundo) {
            warn!("mundo: {} pediu o mapa {}, desligado pelo painel — troca recusada", p.roleid, p.mundo);
            return;
        }
        let Some(destino) = self.mapa(p.mundo) else {
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
                self.donos.write().await.insert(p.roleid,p.mundo);
                if saiu {destino.encerrar_apos_transferencia(p.roleid,&envio,resultado).await;}
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
                            este.donos.write().await.insert(p.roleid,p.mundo);
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
            .map(|c| c.dados.pastas_de_mapa.keys().copied().collect()).unwrap_or_default();
        carregaveis.sort_unstable();
        serde_json::json!({"presenca":"observada","mapas":mapas,"taxas":self.taxas(),"carregaveis":carregaveis})
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
            E::MoverItem { de, slot_de, tid, para, slot_para } => match bus.mover_item_offline(roleid, de, slot_de, tid, para, slot_para).await {
                Ok(()) => serde_json::json!({"estado":"salvo","presenca":"offline","item":tid,"de":slot_de,"para":slot_para}),
                Err(codigo) => serde_json::json!({"estado":"falha","codigo":codigo}),
            },
            E::Item { tid, quantidade } => match bus.dar_item_offline(roleid, tid, quantidade).await {
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

    /// Painel (B209): como os efeitos pedidos entram na peça `item_id` (dados do realm).
    pub async fn efeitos_para_item(&self, item_id: u32, ids: &[u32]) -> serde_json::Value {
        match self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m)) {
            Some(bus) => bus.efeitos_para_item(item_id, ids).await,
            None => serde_json::json!({"codigo":"sem_mapa"}),
        }
    }

    /// Painel (E6, B186): busca de itens por nome ou id no `elements.data` do realm.
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
        let transicao = matches!(&msg, BusMessage::EnterWorld { .. } | BusMessage::PlayerLogout { .. })
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
            | BusMessage::SetCustomDataRe { roleid, .. } => *roleid,
            BusMessage::ChatSingleCast { dstroleid, .. } => *dstroleid,
        };

        let mapa = if matches!(msg, BusMessage::EnterWorld { .. }) {
            if let Some(m)=self.mapa_de(roleid).await {
                if match self.mapa(m) { Some(s) => s.tem_sessao(roleid).await, None => false } {
                    warn!("entrada: {roleid} já possui sessão ou saída pendente; rejeitada duplicata");
                    return;
                }
            }
            let mapa = self.mapa_para_entrar(roleid).await;
            // Desligado pelo painel, ou o padrão descarregado (B183): mesma recusa.
            if !self.mapa_ligado(mapa) || !self.serve(mapa) {
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
        mundo: WorldInstance,
        versao: pw_protocol::GameVersion,
    ) -> (i32, Arc<BusServer>) {
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
