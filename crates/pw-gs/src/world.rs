use crate::ai::MonsterAi;
use crate::entity::{ItemDropEntity, MatterEntity, MonsterEntity, NpcEntity, PlayerEntity};
use crate::grid::SpatialGrid;
use pw_core::{RoleId, WorldId};
use pw_data_loader::GameDataManager;
use pw_protocol::MembroDoGrupo;
use pw_storage::CharacterRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Algo que aconteceu no mundo e que um jogador precisa saber.
///
/// # Por que um evento, e não um pacote
///
/// A simulação não conhece formato de fio. Ela diz *o que aconteceu*; quem traduz para os
/// subcomandos do cliente é o [`crate::BusServer`], que já é o lugar onde os dois formatos
/// se encontram. Manter essa divisão é o que permite testar a simulação sem montar rede,
/// e trocar o protocolo sem tocar no mundo.
#[derive(Debug, Clone, PartialEq)]
pub enum EventoDoMundo {
    /// O jogador levou dano de alguém.
    DanoRecebido {
        roleid: RoleId,
        /// Quem bateu. `0` quando a origem não é conhecida.
        atacante: i64,
        dano: i32,
        hp: i32,
        max_hp: i32,
    },
    /// O jogador chegou a zero de vida.
    JogadorMorreu {
        roleid: RoleId,
        matador: i64,
        pos: pw_core::Vector3,
    },
    /// Um monstro andou. O cliente interpola até `destino`; sem este aviso o monstro
    /// perseguia em silêncio, e na tela ficava parado.
    MonstroAndou {
        id: i64,
        destino: pw_core::Vector3,
        /// Quanto o cliente leva para percorrer o trecho, em milissegundos.
        tempo_ms: u16,
        velocidade: f32,
        /// `move_mode`: andar/correr e o bit do habitat.
        modo: u8,
    },
    /// Um monstro parou (`OBJECT_STOP_MOVE`).
    MonstroParou {
        id: i64,
        posicao: pw_core::Vector3,
        velocidade: f32,
        direcao: u8,
        modo: u8,
    },
    /// O jogador voltou a viver, e onde.
    JogadorReviveu {
        roleid: RoleId,
        pos: pw_core::Vector3,
        hp: i32,
        max_hp: i32,
    },
    /// Vida ou mana do jogador mudaram sozinhas (regeneração): o `SELF_INFO_00` é o que o
    /// original manda quando o `_refresh_state` liga (`GenHPandMP`, `actobject.h:2167`).
    EstadoMudou { roleid: RoleId },
    /// O corpo do monstro some (`GM_MSG_OBJ_ZOMBIE_END`, `_corpse_delay`, `npc.cpp:1446-1459`).
    MonstroSumiu { id: i64 },
    /// O monstro renasceu no ponto de origem.
    MonstroRenasceu { id: i64 },
    /// Um item ou monte de moedas no chão acabou a vida (`gmatter_item_base_imp`,
    /// `matter.cpp:133-137`).
    DropSumiu { id: i64 },
}

/// `_corpse_delay` do monstro: 20 s (`npc.cpp:803`), vezes 20 ticks no `PostLazyMessage`.
pub const CORPO_MS: u32 = 20_000;
/// Primeiro id de item no chão. Os dois bits altos marcam matéria (`ISMATTERID`,
/// `EC_GPDataType.h:27`); o `npcgen.data` usa os ids baixos para minério e erva.
const PRIMEIRO_ID_DE_DROP: u32 = 0xC800_0000;

/// Um grupo de jogadores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grupo {
    pub lider: RoleId,
    pub membros: Vec<RoleId>,
}

pub struct WorldInstance {
    pub world_id: WorldId,
    pub grid: SpatialGrid,
    pub players: HashMap<i64, PlayerEntity>,
    pub monsters: HashMap<i64, (MonsterEntity, MonsterAi)>,
    pub npcs: HashMap<i64, NpcEntity>,
    /// A altura do chão deste mapa, dos `.hmap`. Ver [`pw_data_loader::terreno`].
    ///
    /// Carregada só para **este** mapa, em [`Self::init_spawns`]: o mundo principal são 92
    /// MB de vértices, e o realm tem 68 pastas de mapa.
    pub terreno: pw_data_loader::Terreno,
    /// Os recursos do mapa — minério, erva, tronco. Ver [`MatterEntity`].
    ///
    /// Ficam separados dos NPCs porque o comando de entrada é outro
    /// (`MATTER_ENTER_WORLD`, 18) e o de saída também (`OUT_OF_SIGHT_LIST`, 34, e não o
    /// `OBJECT_LEAVE_SLICE` que serve a jogador e NPC).
    pub matters: HashMap<i64, MatterEntity>,
    pub drops: HashMap<i64, ItemDropEntity>,
    pub data_manager: Arc<GameDataManager>,
    pub char_repo: CharacterRepository,
    /// Grupos ativos, por id.
    grupos: HashMap<u32, Grupo>,
    /// Em que grupo cada jogador está — o índice inverso, para não varrer todos.
    grupo_de: HashMap<RoleId, u32>,
    /// Convites pendentes: convidado → quem convidou.
    convites: HashMap<RoleId, RoleId>,
    proximo_grupo: u32,
    /// Por onde a simulação avisa o mundo lá fora. `None` num mundo sem rede — que é o
    /// caso em teste de unidade, e era o caso do `pw-gs` inteiro antes do barramento.
    eventos: Option<mpsc::Sender<EventoDoMundo>>,
    _entity_counter: i64,
    autosave_timer_ms: u32,
    /// O batimento de 1 s dos jogadores (regeneração e combate).
    batimento_ms: u32,
    proximo_drop: u32,
    /// Corpos de monstro ainda na tela: id → quanto falta para sumir.
    corpos: HashMap<i64, u32>,
}

impl WorldInstance {
    pub fn new(
        world_id: WorldId,
        data_manager: Arc<GameDataManager>,
        char_repo: CharacterRepository,
    ) -> Self {
        Self {
            world_id,
            grid: SpatialGrid::new(),
            players: HashMap::new(),
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            matters: HashMap::new(),
            terreno: pw_data_loader::Terreno::vazio(),
            drops: HashMap::new(),
            data_manager,
            char_repo,
            grupos: HashMap::new(),
            grupo_de: HashMap::new(),
            convites: HashMap::new(),
            proximo_grupo: 0,
            eventos: None,
            _entity_counter: 100000,
            autosave_timer_ms: 0,
            batimento_ms: 0,
            proximo_drop: PRIMEIRO_ID_DE_DROP,
            corpos: HashMap::new(),
        }
    }

    /// Liga a saída de eventos da simulação.
    pub fn definir_canal_de_eventos(&mut self, envio: mpsc::Sender<EventoDoMundo>) {
        self.eventos = Some(envio);
    }

    /// Publica um evento, se houver quem escute.
    ///
    /// Nunca bloqueia o tick: a fila cheia significa que a ponta de rede não está dando
    /// conta, e parar a simulação inteira por isso seria trocar um problema de um jogador
    /// por um problema de todos.
    fn emitir(&self, evento: EventoDoMundo) {
        let Some(canal) = self.eventos.as_ref() else {
            return;
        };
        if canal.try_send(evento).is_err() {
            warn!("mundo: fila de eventos cheia; um aviso ao cliente foi descartado");
        }
    }

    /// Inicializa os Spawns de monstros e NPCs a partir do `npcgen.data` do mapa específico
    ///
    /// # A altura de cada entidade
    ///
    /// É a regra do original, por **tipo de área** — ver
    /// `pw_data_loader::npcgen::SpawnInstance::altura_resolvida`: área de chão nasce no
    /// terreno, área em caixa nasce onde a caixa manda com o terreno como piso, e o
    /// `fOffsetTrn`/`fHeiOff` do gerador soma por cima.
    ///
    /// A história, que vale guardar: em 2026-09-11 os monstros apareciam no ar porque a
    /// dispersão sorteava `x`/`z` e copiava o `y` do centro da área (item 42d). A primeira
    /// correção deduziu um deslocamento por área, `centro.y − chão(centro)`, porque o
    /// leitor ainda descartava o tipo da área e o `fOffsetTrn`. Em 2026-09-12 os dois
    /// campos passaram a ser lidos, e a medida derrubou a dedução: o `fOffsetTrn` é zero
    /// em 18.902 dos 18.903 geradores, e as 3.517 áreas em caixa é que são as que ficam
    /// fora do chão de propósito. A dedução deixava flutuando qualquer NPC de área de chão
    /// que o editor tivesse posto um pouco acima do terreno.
    ///
    /// Sem mapa de alturas (mapa fora do catálogo, pasta sem `map/`) vale o `y` que o
    /// arquivo e a dispersão deram — o comportamento antigo, que ao menos não piora.
    pub fn init_spawns(&mut self) {
        info!("Inicializando monstros e NPCs do World #{} a partir do seu npcgen.data dedicado...", self.world_id);

        // O mapa de alturas deste mapa, se o realm o trouxer.
        if let Some(dir) = self.data_manager.pastas_de_mapa.get(&self.world_id).cloned() {
            self.terreno = pw_data_loader::Terreno::ler(self.world_id, &dir);
        }
        let com_terreno = self.terreno.tem_dados();

        let mut sem_template = 0usize;
        let mut fora_do_mapa = 0usize;

        if let Some(spawns) = self.data_manager.map_spawns.get(&self.world_id) {
            for inst in &spawns.instances {
                // A altura, pela regra do original — ver a nota da função.
                let pos = if com_terreno {
                    let chao = self.terreno.altura_em(inst.pos.x, inst.pos.z);
                    if chao.is_none() {
                        fora_do_mapa += 1;
                    }
                    pw_core::Vector3::new(inst.pos.x, inst.altura_resolvida(chao), inst.pos.z)
                } else {
                    inst.pos
                };

                // Monstro ou NPC, pelo tipo do registro no `elements.data` — ver
                // `GameDataManager::ids_de_npc`. O `npcgen.rs` chuta pelo número
                // (`tid >= 10000` é NPC), o que no 1.5.5 transformava em NPC todo monstro
                // novo: no mapa 161 eram 5 monstros e 1.472 "NPCs", com coelhos, cervos e
                // esquilos entre eles. O chute só fica para id que nenhuma tabela conhece.
                let tipo = match inst.spawn_type {
                    pw_data_loader::SpawnType::Monster | pw_data_loader::SpawnType::Npc
                        if self.data_manager.monstros.get(inst.template_id).is_some() =>
                    {
                        pw_data_loader::SpawnType::Monster
                    }
                    pw_data_loader::SpawnType::Monster | pw_data_loader::SpawnType::Npc
                        if self.data_manager.ids_de_npc.contains(&inst.template_id) =>
                    {
                        pw_data_loader::SpawnType::Npc
                    }
                    outro => outro,
                };

                if tipo == pw_data_loader::SpawnType::Monster {
                    let monster_id = inst.instance_id as i64;

                    // Os atributos vêm do `MONSTER_ESSENCE` do `elements.data`
                    // (`pw_data_loader::monstros`), que é o que o `npcgenerator.cpp` do
                    // servidor original usa. Antes eram todos escritos aqui — nível 1,
                    // 500 de vida, dano 20 a 35 — para todo monstro de todo mapa.
                    let monster = match self.data_manager.monstros.get(inst.template_id) {
                        Some(modelo) => MonsterEntity::do_template(
                            monster_id,
                            modelo,
                            pos,
                            inst.respawn_sec * 1000,
                        ),
                        None => {
                            sem_template += 1;
                            MonsterEntity::placeholder(
                                monster_id,
                                inst.template_id,
                                pos,
                                inst.respawn_sec * 1000,
                            )
                        }
                    };

                    self.grid.add_entity(monster_id, monster.position, false);
                    self.monsters.insert(monster_id, (monster, MonsterAi::new()));
                } else if tipo == pw_data_loader::SpawnType::Npc {
                    // NPCs de serviço (treinador, vendedor, dador de missão, guarda) não
                    // existiam como entidade nenhuma no mundo simulado — só monstros eram
                    // spawnados. Sem isso, `SELECT_TARGET` e `SEVNPC_HELLO` não encontram o
                    // id que o cliente manda, mesmo com o NPC visível na tela (o
                    // `gateway.rs` mostra a lista de entidades ao entrar no mundo por outro
                    // caminho, que não depende disto).
                    let npc_id = inst.instance_id as i64;

                    let npc = NpcEntity {
                        id: npc_id,
                        template_id: inst.template_id,
                        name: "NPC".to_string(),
                        position: pos,
                        dialog_id: 0,
                    };

                    self.grid.add_entity(npc_id, npc.position, false);
                    self.npcs.insert(npc_id, npc);
                } else if inst.spawn_type == pw_data_loader::SpawnType::ResourceMine {
                    // Minério, erva, tronco. Ninguém os mandava ao cliente: não havia
                    // entidade nenhuma para eles no mundo, e `MATTER_ENTER_WORLD` (18) não
                    // saía de lugar nenhum do servidor — o mapa vinha sem recurso algum
                    // (2026-09-09).
                    let mid = inst.instance_id as i64;
                    let matter = MatterEntity {
                        id: mid,
                        template_id: inst.template_id,
                        position: pos,
                    };
                    self.grid.add_entity(mid, matter.position, false);
                    self.matters.insert(mid, matter);
                }
            }
        }

        info!(
            "World #{} inicializado com {} monstros, {} NPCs e {} recursos de mapa ativos              a partir do seu npcgen.data!",
            self.world_id,
            self.monsters.len(),
            self.npcs.len(),
            self.matters.len()
        );
        if fora_do_mapa > 0 {
            warn!(
                "World #{}: {fora_do_mapa} spawn(s) fora da área coberta pelos .hmap —                  entraram com o y do npcgen.data como altura absoluta.",
                self.world_id
            );
        }
        if sem_template > 0 {
            warn!(
                "World #{}: {} monstro(s) sem template no elements.data — entraram com                  atributos de placeholder. No realm 1.2.6 isso é esperado (o leitor                  genérico ainda não cobre a v7); no 1.5.5 significa npcgen.data citando                  monstro que o elements.data não tem, ou que o original recusaria.",
                self.world_id, sem_template
            );
        }
    }

    /// Adiciona um jogador que entrou neste mapa
    pub fn add_player(&mut self, player: PlayerEntity) {
        let role_id = player.role_id as i64;
        self.grid.add_entity(role_id, player.position, true);
        self.players.insert(role_id, player);
        info!("Jogador #{} entrou no World #{}", role_id, self.world_id);
    }

    /// Remove um jogador ao deslogar ou mudar de mapa
    pub fn remove_player(&mut self, role_id: RoleId) -> Option<PlayerEntity> {
        let id = role_id as i64;
        self.grid.remove_entity(id);
        // Sair do mundo é sair do grupo. Sem isto o grupo guardaria um membro que não
        // existe mais, e a lista de membros mostraria um fantasma.
        self.sair_do_grupo(role_id);
        self.convites.remove(&role_id);
        self.players.remove(&id)
    }

    /// Move um jogador para a posição que ele reportou.
    ///
    /// Atualiza a entidade **e** a grade espacial — as duas, sempre. Mexer só na entidade
    /// deixaria a grade com a posição velha, e a grade é o que responde "quem está perto
    /// de quem": o jogador andaria na tela e continuaria sendo visto no lugar antigo por
    /// todo mundo, inclusive pelos monstros que decidem agressão por distância.
    ///
    /// Não persiste nada. A gravação é do autosave periódico, e é isso que distingue esta
    /// implementação da anterior: o `gateway.rs` fazia um `UPDATE` no PostgreSQL **a cada
    /// pacote de movimento**, de cada jogador. Com o mundo em memória, o banco vê uma
    /// gravação por minuto por jogador em vez de dezenas por segundo.
    ///
    /// Devolve `false` quando o jogador não está neste mundo — o que é o caso normal
    /// enquanto o `EnterWorld` ainda não trouxe a entidade para cá.
    pub fn mover_jogador(&mut self, role_id: RoleId, pos: pw_core::Vector3) -> bool {
        let id = role_id as i64;
        let Some(p) = self.players.get_mut(&id) else {
            return false;
        };
        p.position = pos;
        self.grid.update_position(id, pos);
        true
    }

    /// Renasce o jogador na cidade (`gplayer_controller::ResurrectInTown`,
    /// `playercmd.cpp:112-129`).
    ///
    /// O ponto é o de cidade do distrito do `precinct.sev` que contém a posição, quando é
    /// deste mapa; sem distrito o original renasce no lugar. Vida e mana voltam a 10 % e a
    /// experiência perde a fração do cultivo — ver [`crate::progressao::renascer`].
    ///
    /// Devolve a posição, ou `None` se o jogador não estiver neste mundo ou não estiver
    /// morto — ressuscitar quem está vivo é o caminho para se teleportar de graça.
    pub fn reviver_jogador(&mut self, role_id: RoleId) -> Option<pw_core::Vector3> {
        let id = role_id as i64;
        let dados = Arc::clone(&self.data_manager);
        let mapa = self.world_id;
        let p = self.players.get_mut(&id)?;
        if p.hp > 0 {
            return None;
        }
        let mut pos = p.position;
        if let Some((ponto, mapa_do_ponto)) = crate::progressao::ponto_de_renascimento(&dados, mapa, p.position.x, p.position.z) {
            if mapa_do_ponto == mapa {
                pos = pw_core::Vector3::new(ponto[0], ponto[1], ponto[2]);
            } else {
                warn!("renascer: o distrito de #{role_id} manda para o mapa {mapa_do_ponto}, e trocar de mapa não existe — renasce no lugar");
            }
        }
        let perdeu = crate::progressao::renascer(p, &dados, false);
        p.position = pos;
        p.target_id = None;
        let (hp, max_hp) = (p.hp, p.max_hp);

        self.grid.update_position(id, pos);
        self.emitir(EventoDoMundo::JogadorReviveu {
            roleid: role_id,
            pos,
            hp,
            max_hp,
        });
        info!("Jogador #{} renasceu em ({:.0}, {:.0}), perdeu {perdeu} de experiência", role_id, pos.x, pos.z);
        Some(pos)
    }

    /// Põe um item (ou um monte de moedas, `tid` 3044) no chão, a ±2 m do ponto e no
    /// terreno (`GM_MSG_PRODUCE_MONEY`/`_MONSTER_DROP`, `worldmanager.cpp:512-555`).
    pub fn criar_drop(&mut self, tid: u32, quantidade: u32, perto_de: pw_core::Vector3, dono: Option<RoleId>) -> ItemDropEntity {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut pos = perto_de;
        pos.x += rng.gen::<f32>() * 4.0 - 2.0;
        pos.z += rng.gen::<f32>() * 4.0 - 2.0;
        if let Some(chao) = self.terreno.altura_em(pos.x, pos.z) {
            if pos.y < chao {
                pos.y = chao;
            }
        }
        // Id de entidade é `i32` no fio, e o mundo o guarda com sinal: `0xC8...` é negativo,
        // como os ids de NPC e matéria do `npcgen.data`.
        let id = self.proximo_drop as i32 as i64;
        self.proximo_drop = self.proximo_drop.wrapping_add(1) | PRIMEIRO_ID_DE_DROP;
        let d = ItemDropEntity {
            id,
            item_id: tid,
            count: quantidade,
            position: pos,
            owner_role_id: dono,
            protect_timer_ms: crate::economia::POSSE_S * 1000,
            despawn_timer_ms: crate::economia::VIDA_NO_CHAO_S * 1000,
        };
        self.grid.add_entity(id, pos, false);
        self.drops.insert(id, d.clone());
        d
    }

    /// Tira um item do chão.
    pub fn remover_drop(&mut self, id: i64) -> Option<ItemDropEntity> {
        let d = self.drops.remove(&id)?;
        self.grid.remove_entity(id);
        for p in self.players.values_mut() {
            p.visiveis.remove(&id);
        }
        Some(d)
    }

    /// Marca o monstro como morto: corpo por [`CORPO_MS`] e renascimento pelo tempo do
    /// gerador do `npcgen.data`.
    pub fn matar_monstro(&mut self, id: i64) {
        if let Some((m, _)) = self.monsters.get_mut(&id) {
            m.is_dead = true;
            m.respawn_timer_ms = m.respawn_delay_ms.max(1);
            m.target_id = None;
        }
        self.corpos.insert(id, CORPO_MS);
    }

    // ------------------------------------------------------------------
    // Grupo
    //
    // Nada disto existia. O `gateway.rs` respondia os pacotes de grupo e não guardava
    // grupo nenhum: o convite era mandado **de volta a quem convidou**, a lista de
    // membros vinha com vida e posição escritas no código, e sair do grupo era só um eco
    // para o próprio jogador. Ninguém mais no jogo ficava sabendo de nada.
    // ------------------------------------------------------------------

    /// Registra um convite pendente. `false` se o convidado já está num grupo.
    ///
    /// Um convite só substitui outro convite; não atropela alguém que já tem grupo, o que
    /// seria uma forma de arrastar jogador para fora do time dele à revelia.
    pub fn convidar_para_grupo(&mut self, quem_convida: RoleId, convidado: RoleId) -> bool {
        if quem_convida == convidado || self.grupo_de.contains_key(&convidado) {
            return false;
        }
        self.convites.insert(convidado, quem_convida);
        true
    }

    /// Aceita o convite pendente e devolve os membros do grupo resultante.
    ///
    /// `None` quando não havia convite de quem o cliente diz — é o que impede um jogador
    /// de entrar em qualquer grupo mandando o comando com o id de um estranho.
    pub fn aceitar_convite(&mut self, quem: RoleId, de_quem: RoleId) -> Option<Vec<RoleId>> {
        match self.convites.get(&quem) {
            Some(convidou) if *convidou == de_quem => {}
            _ => return None,
        }
        self.convites.remove(&quem);

        let id = match self.grupo_de.get(&de_quem) {
            Some(id) => *id,
            None => {
                // Quem convidou ainda não tinha grupo: cria um, com ele de líder.
                self.proximo_grupo += 1;
                let id = self.proximo_grupo;
                self.grupos.insert(
                    id,
                    Grupo {
                        lider: de_quem,
                        membros: vec![de_quem],
                    },
                );
                self.grupo_de.insert(de_quem, id);
                id
            }
        };

        let g = self.grupos.get_mut(&id)?;
        if !g.membros.contains(&quem) {
            g.membros.push(quem);
        }
        self.grupo_de.insert(quem, id);
        Some(g.membros.clone())
    }

    /// Recusa o convite pendente.
    pub fn recusar_convite(&mut self, quem: RoleId) -> Option<RoleId> {
        self.convites.remove(&quem)
    }

    /// Tira o jogador do grupo. Devolve `(líder, quem ficou)`, para que sejam avisados.
    ///
    /// O líder faz parte da resposta porque os dois comandos que anunciam a saída —
    /// `TEAM_MEMBER_LEAVE` (60) e `TEAM_LEAVE_PARTY` (61) — começam pelo `idLeader`, e
    /// quem chama não teria como saber quem é depois que o grupo já mudou.
    ///
    /// Um grupo com um membro só deixa de ser grupo: mantê-lo faria o jogador continuar
    /// "em grupo" sozinho, sem nunca conseguir aceitar outro convite.
    pub fn sair_do_grupo(&mut self, quem: RoleId) -> Option<(RoleId, Vec<RoleId>)> {
        let id = self.grupo_de.remove(&quem)?;
        let g = self.grupos.get_mut(&id)?;
        g.membros.retain(|m| *m != quem);

        if g.lider == quem {
            // O líder saiu: o primeiro que ficou assume.
            if let Some(novo) = g.membros.first().copied() {
                g.lider = novo;
            }
        }

        let lider = g.lider;
        let restantes = g.membros.clone();
        if restantes.len() < 2 {
            for m in &restantes {
                self.grupo_de.remove(m);
            }
            self.grupos.remove(&id);
        }
        Some((lider, restantes))
    }

    /// Os membros do grupo do jogador, ou vazio se ele não tem grupo.
    pub fn membros_do_grupo(&self, quem: RoleId) -> Vec<RoleId> {
        self.grupo_de
            .get(&quem)
            .and_then(|id| self.grupos.get(id))
            .map(|g| g.membros.clone())
            .unwrap_or_default()
    }

    /// Quem lidera o grupo do jogador.
    pub fn lider_do_grupo(&self, quem: RoleId) -> Option<RoleId> {
        self.grupo_de
            .get(&quem)
            .and_then(|id| self.grupos.get(id))
            .map(|g| g.lider)
    }

    /// Os dados que o cliente espera na lista de membros, com os valores **reais**.
    ///
    /// Membro que não está neste mundo é omitido, e não preenchido com zeros: um zero
    /// desenharia barra de vida vazia num companheiro vivo.
    ///
    /// Os campos que a simulação ainda não modela — reencarnações, `wallow_level`,
    /// facção, `profit_level` — saem em zero **por não existirem ainda**, e não por
    /// palpite: zero é o valor neutro de cada um deles no cliente. Quando a simulação
    /// passar a tê-los, é aqui que entram.
    pub fn dados_dos_membros(&self, membros: &[RoleId]) -> Vec<MembroDoGrupo> {
        membros
            .iter()
            .filter_map(|m| {
                let p = self.players.get(&(*m as i64))?;
                Some(MembroDoGrupo {
                    role_id: p.role_id,
                    level: p.level as i16,
                    state: 0,
                    level2: p.cultivation.clamp(0, u8::MAX as i32) as u8,
                    reencarnacoes: 0,
                    wallow_level: 0,
                    hp: p.hp,
                    mp: p.mp,
                    max_hp: p.max_hp,
                    max_mp: p.max_mp,
                    force_id: 0,
                    profit_level: 0,
                })
            })
            .collect()
    }

    /// Vida, vida máxima e alvo atual de um monstro — o que o `NPC_INFO_00` (33) leva.
    ///
    /// É a consulta periódica de barra de vida (`QUERY_NPC_INFO_1`, 68). O `gateway.rs`
    /// respondia **`1000/1000` fixo** para qualquer criatura, porque o daemon de link não
    /// tem simulação: era a mesma razão que já tinha feito o `SELECT_TARGET` mudar de
    /// lado. Enquanto isso o combate daqui debitava a vida de verdade, e a consulta
    /// seguinte desenhava a barra cheia de novo.
    pub fn dados_do_monstro(&self, id: i64) -> Option<(i32, i32, i32)> {
        let (m, _) = self.monsters.get(&id)?;
        let alvo = m.target_id.unwrap_or(0);
        Some((
            m.hp.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            m.max_hp.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            alvo.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        ))
    }

    /// O que o `NPC_INFO_00` (33) leva sobre um NPC de serviço (não-monstro).
    ///
    /// NPCs de serviço não têm HP na nossa entidade — não são atacáveis. `1/1` é o valor
    /// que diz "vivo, cheio", sem inventar um número de combate que não existe para eles;
    /// é diferente do bug antigo (item 37/45 do `docs/ESTADO_E_RETOMADA.md`) porque não
    /// varia por personagem nem finge ser dano real, só marca presença.
    pub fn dados_do_npc(&self, id: i64) -> Option<(i32, i32, i32)> {
        self.npcs.get(&id)?;
        Some((1, 1, 0))
    }

    /// O que o `PLAYER_INFO_00` (32) leva sobre outro jogador.
    ///
    /// Devolve `(level, level2, hp, max_hp, mp, max_mp, alvo)`. O `gateway.rs` **não
    /// respondia nada** ao `QUERY_PLAYER_INFO_1` (67): lia a contagem, escrevia uma linha
    /// de log e devolvia. Nenhum outro jogador tinha barra de vida.
    #[allow(clippy::type_complexity)]
    pub fn dados_do_jogador(&self, role_id: RoleId) -> Option<(i16, u8, i32, i32, i32, i32, i32)> {
        let p = self.players.get(&(role_id as i64))?;
        Some((
            p.level as i16,
            p.cultivation.clamp(0, u8::MAX as i32) as u8,
            p.hp,
            p.max_hp,
            p.mp,
            p.max_mp,
            p.target_id.unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        ))
    }

    /// O bloco que o `SELF_INFO_00` (38) leva sobre o próprio jogador.
    ///
    /// Devolve `(level, level2, hp, max_hp, mp, max_mp, exp, sp)`. O `gateway.rs`
    /// respondia `120/120/280/280` para qualquer personagem, com exp e sp zerados —
    /// a **terceira** aparição do mesmo `120/280` escrito no código (itens 37 e 45).
    #[allow(clippy::type_complexity)]
    pub fn dados_do_proprio(&self, role_id: RoleId) -> Option<(i16, u8, i32, i32, i32, i32, i32, i32)> {
        let p = self.players.get(&(role_id as i64))?;
        Some((
            p.level as i16,
            p.cultivation.clamp(0, u8::MAX as i32) as u8,
            p.hp,
            p.max_hp,
            p.mp,
            p.max_mp,
            p.exp.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            p.sp.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        ))
    }

    /// O dinheiro do jogador. O `gateway.rs` mandava **50000 fixo** para todo mundo.
    pub fn dinheiro(&self, role_id: RoleId) -> Option<i32> {
        let p = self.players.get(&(role_id as i64))?;
        Some(p.money.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    }

    /// Restaura vida e mana do jogador, sem passar do máximo.
    ///
    /// Devolve `(hp, max_hp, mp, max_mp)` depois da cura, ou `None` se o jogador não
    /// estiver neste mundo. Não cura quem está morto: para isso existe o renascimento, e
    /// deixar uma poção reviver seria mudar a regra do jogo por acidente.
    pub fn curar_jogador(
        &mut self,
        role_id: RoleId,
        hp_recupera: i32,
        mp_recupera: i32,
    ) -> Option<(i32, i32, i32, i32)> {
        let p = self.players.get_mut(&(role_id as i64))?;
        if p.hp <= 0 {
            return None;
        }
        p.hp = (p.hp + hp_recupera.max(0)).min(p.max_hp);
        p.mp = (p.mp + mp_recupera.max(0)).min(p.max_mp);
        Some((p.hp, p.max_hp, p.mp, p.max_mp))
    }

    /// Quanto um item de consumo restaura, segundo o `elements.data`.
    ///
    /// `None` quando o item não é remédio — o que é a resposta certa para uma arma, e não
    /// um zero disfarçado de cura.
    ///
    /// Antes disto o `gateway.rs` reconhecia poção comparando o id com **1796 e 1801**,
    /// escritos no código, e respondia HP/MP `120/280` fixos para qualquer personagem. Os
    /// valores de verdade já estavam carregados no `elements.data` e nunca eram
    /// consultados. A busca em si (tipada vs. catálogo genérico, conforme a versão do
    /// realm) vive em `GameDataManager::quanto_o_remedio_restaura` — `pw-gs` não precisa
    /// saber qual dos dois formatos este realm usa.
    pub fn quanto_o_remedio_restaura(&self, item_id: u32) -> Option<(i32, i32)> {
        self.data_manager.quanto_o_remedio_restaura(item_id)
    }

    /// Ciclo de Simulação em Tempo Real (Loop de 50ms / 20 TPS)
    pub async fn tick(&mut self, delta_ms: u32) {
        // 1. Atualização da Inteligência Artificial dos Monstros
        let mut attacks_to_process = Vec::new();

        let mut movimentos = Vec::new();
        let mut renasceram: Vec<(i64, pw_core::Vector3)> = Vec::new();

        for (monster, ai) in self.monsters.values_mut() {
            if monster.is_dead {
                if monster.respawn_timer_ms > 0 {
                    monster.respawn_timer_ms = monster.respawn_timer_ms.saturating_sub(delta_ms);
                    if monster.respawn_timer_ms == 0 {
                        // Renascimento do Monstro
                        monster.is_dead = false;
                        monster.hp = monster.max_hp;
                        monster.position = monster.spawn_center;
                        monster.danos.clear();
                        monster.primeiro_atacante = None;
                        *ai = MonsterAi::new();
                        renasceram.push((monster.id, monster.position));
                    }
                }
                continue;
            }

            let terreno = &self.terreno;
            let chao = |x: f32, z: f32| terreno.altura_em(x, z);
            match ai.tick(monster, &self.players, delta_ms, &chao) {
                Some(crate::ai::AcaoDoMonstro::Atacou { alvo, dano }) => {
                    attacks_to_process.push((alvo, dano));
                }
                Some(crate::ai::AcaoDoMonstro::Andou { destino, tempo_ms, velocidade, modo }) => {
                    // A grade espacial tem de acompanhar: quem consulta vizinhos por
                    // posição usa ela, não o campo da entidade.
                    movimentos.push((
                        monster.id,
                        destino,
                        EventoDoMundo::MonstroAndou { id: monster.id, destino, tempo_ms, velocidade, modo },
                    ));
                }
                Some(crate::ai::AcaoDoMonstro::Parou { posicao, velocidade, direcao, modo }) => {
                    movimentos.push((
                        monster.id,
                        posicao,
                        EventoDoMundo::MonstroParou { id: monster.id, posicao, velocidade, direcao, modo },
                    ));
                }
                None => {}
            }
        }

        // Corpos que somem, e os que renasceram antes de o corpo sumir.
        let mut sumiram = Vec::new();
        for (id, falta) in self.corpos.iter_mut() {
            *falta = falta.saturating_sub(delta_ms);
            if *falta == 0 {
                sumiram.push(*id);
            }
        }
        for (id, _) in &renasceram {
            if self.corpos.contains_key(id) && !sumiram.contains(id) {
                sumiram.push(*id);
            }
        }
        for id in sumiram {
            self.corpos.remove(&id);
            for p in self.players.values_mut() {
                p.visiveis.remove(&id);
            }
            self.emitir(EventoDoMundo::MonstroSumiu { id });
        }
        for (id, pos) in renasceram {
            self.grid.add_entity(id, pos, false);
            self.emitir(EventoDoMundo::MonstroRenasceu { id });
        }

        // Itens no chão: a posse acaba em 30 s e o item some em 300 s.
        let mut drops_sumiram = Vec::new();
        for d in self.drops.values_mut() {
            d.protect_timer_ms = d.protect_timer_ms.saturating_sub(delta_ms);
            if d.protect_timer_ms == 0 {
                d.owner_role_id = None;
            }
            d.despawn_timer_ms = d.despawn_timer_ms.saturating_sub(delta_ms);
            if d.despawn_timer_ms == 0 {
                drops_sumiram.push(d.id);
            }
        }
        for id in drops_sumiram {
            self.remover_drop(id);
            self.emitir(EventoDoMundo::DropSumiu { id });
        }

        // Batimento de 1 s: combate e regeneração (`gplayer_imp::OnHeartbeat`).
        self.batimento_ms += delta_ms;
        if self.batimento_ms >= 1000 {
            self.batimento_ms -= 1000;
            let mudaram: Vec<RoleId> = self
                .players
                .values_mut()
                .filter_map(|p| crate::progressao::batimento(p).then_some(p.role_id))
                .collect();
            for roleid in mudaram {
                self.emitir(EventoDoMundo::EstadoMudou { roleid });
            }
        }

        // Fora do laço porque `self.grid` e `self.monsters` não podem ser emprestados ao
        // mesmo tempo.
        for (id, destino, evento) in movimentos {
            self.grid.update_position(id, destino);
            self.emitir(evento);
        }

        // 2. Aplica danos causados pelos monstros nos jogadores
        //
        // Até aqui isto acontecia **em silêncio**: o HP caía e o cliente nunca era
        // avisado. O jogador via a vida cheia e morria do nada. Agora cada golpe vira um
        // evento, e o `BusServer` o entrega àquele jogador.
        for (player_id, damage) in attacks_to_process {
            let Some(player) = self.players.get_mut(&player_id) else {
                continue;
            };
            if player.hp <= 0 {
                continue; // já caído: não se bate em quem está morto
            }

            player.hp = (player.hp - damage).max(0);
            // Apanhar põe em combate por pelo menos 5 s (`OnAttacked`, `player.cpp:9514`).
            player.combate_s = player.combate_s.max(crate::progressao::COMBATE_AO_APANHAR_S);
            let (hp, max_hp, pos) = (player.hp, player.max_hp, player.position);
            let role_id = player.role_id;
            debug!("Monstro causou {} de dano no Jogador #{} (HP restante: {})", damage, player_id, hp);

            self.emitir(EventoDoMundo::DanoRecebido {
                roleid: role_id,
                // Sem rastrear qual monstro bateu, o cliente não sabe de onde veio. O
                // `MonsterAi::tick` ainda não devolve o atacante; até lá vai zero, que o
                // cliente trata como "origem desconhecida".
                atacante: 0,
                dano: damage,
                hp,
                max_hp,
            });

            if hp == 0 {
                info!("Jogador #{} morreu", player_id);
                self.emitir(EventoDoMundo::JogadorMorreu {
                    roleid: role_id,
                    matador: 0,
                    pos,
                });
            }
        }

        // 3. Autosave Periódico de Personagens para o PostgreSQL (a cada 60s)
        self.autosave_timer_ms += delta_ms;
        if self.autosave_timer_ms >= 60_000 {
            self.autosave_timer_ms = 0;
            let mut falhas = 0usize;
            for player in self.players.values() {
                let r = self
                    .char_repo
                    .save_status(
                        player.role_id,
                        player.level,
                        player.cultivation,
                        player.exp,
                        player.sp,
                        player.hp,
                        player.mp,
                        player.money,
                        self.world_id,
                        &player.position,
                    )
                    .await;
                if let Err(e) = r {
                    falhas += 1;
                    warn!(
                        "autosave: não consegui gravar o personagem {}: {e}",
                        player.role_id
                    );
                }
                let _ = self.char_repo.gravar_pontos_de_atributo(player.role_id, player.pontos_de_atributo).await;
                let [a, b, c, d, e] = player.missoes.blocos();
                let listas = pw_storage::ListasDeMissaoGravadas { ativa: a, concluidas: b, tempos: c, contagens: d, deposito: e };
                if let Err(e) = self.char_repo.task_lists().gravar(player.role_id, &listas).await {
                    warn!("autosave: não consegui gravar as missões de {}: {e}", player.role_id);
                }
            }
            // O `let _ =` que havia aqui engolia o erro, e a linha abaixo dizia "com
            // sucesso" de qualquer jeito. O `UPDATE` vinha falhando havia semanas porque
            // escrevia numa coluna `last_login_at` que a tabela `characters` não tem —
            // ninguém viu, e nada de posição, experiência, dinheiro ou nível era salvo.
            if falhas == 0 {
                debug!(
                    "autosave: {} jogadores gravados no mundo {}",
                    self.players.len(),
                    self.world_id
                );
            } else {
                warn!(
                    "autosave: {falhas} de {} jogadores não foram gravados no mundo {}",
                    self.players.len(),
                    self.world_id
                );
            }
        }
    }
}
