use crate::ai::MonsterAi;
use crate::entity::{ItemDropEntity, MonsterEntity, NpcEntity, PlayerEntity};
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
    /// O jogador voltou a viver, e onde.
    JogadorReviveu {
        roleid: RoleId,
        pos: pw_core::Vector3,
        hp: i32,
        max_hp: i32,
    },
}

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
    pub fn init_spawns(&mut self) {
        info!("Inicializando monstros e NPCs do World #{} a partir do seu npcgen.data dedicado...", self.world_id);

        if let Some(spawns) = self.data_manager.map_spawns.get(&self.world_id) {
            for inst in &spawns.instances {
                if inst.spawn_type == pw_data_loader::SpawnType::Monster {
                    let monster_id = inst.instance_id as i64;

                    let monster = MonsterEntity {
                        id: monster_id,
                        template_id: inst.template_id,
                        name: "Monstro".to_string(),
                        level: 1,
                        hp: 500,
                        max_hp: 500,
                        mp: 100,
                        max_mp: 100,
                        def_phys: 50,
                        def_magic: 50,
                        attack_min: 20,
                        attack_max: 35,
                        attack_range: 2.5,
                        exp: 100,
                        sp: 20,
                        aipolicy_id: 0,
                        drop_table_id: 0,
                        position: inst.pos,
                        spawn_center: inst.pos,
                        move_speed: 3.5,
                        is_dead: false,
                        respawn_timer_ms: 0,
                        respawn_delay_ms: inst.respawn_sec * 1000,
                        target_id: None,
                        buffs: Vec::new(),
                    };

                    self.grid.add_entity(monster_id, monster.position, false);
                    self.monsters.insert(monster_id, (monster, MonsterAi::new()));
                } else if inst.spawn_type == pw_data_loader::SpawnType::Npc {
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
                        position: inst.pos,
                        dialog_id: 0,
                    };

                    self.grid.add_entity(npc_id, npc.position, false);
                    self.npcs.insert(npc_id, npc);
                }
            }
        }

        info!(
            "World #{} inicializado com {} monstros e {} NPCs ativos a partir do seu npcgen.data!",
            self.world_id,
            self.monsters.len(),
            self.npcs.len()
        );
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

    /// Ressuscita o jogador na cidade, com a vida cheia.
    ///
    /// Devolve a posição de renascimento, ou `None` se o jogador não estiver neste mundo
    /// ou não estiver morto — ressuscitar quem está vivo é o caminho para um jogador se
    /// teleportar de graça sempre que quiser.
    pub fn reviver_jogador(&mut self, role_id: RoleId) -> Option<pw_core::Vector3> {
        let id = role_id as i64;
        let p = self.players.get_mut(&id)?;
        if p.hp > 0 {
            return None;
        }

        // O ponto de nascimento da classe é o mesmo que o `create_character` usa quando
        // não há template no banco — um lugar só define onde cada classe começa.
        let (x, y, z) = p.cls.default_spawn_position();
        let pos = pw_core::Vector3::new(x, y, z);

        p.hp = p.max_hp;
        p.mp = p.max_mp;
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
        info!("Jogador #{} reviveu na cidade", role_id);
        Some(pos)
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
    /// valores de verdade já estavam carregados em `elements.medicines` e nunca eram
    /// consultados.
    pub fn quanto_o_remedio_restaura(&self, item_id: u32) -> Option<(i32, i32)> {
        let m = self.data_manager.elements.medicines.get(&item_id)?;
        Some((m.hp_restore, m.mp_restore))
    }

    /// Ciclo de Simulação em Tempo Real (Loop de 50ms / 20 TPS)
    pub async fn tick(&mut self, delta_ms: u32) {
        // 1. Atualização da Inteligência Artificial dos Monstros
        let mut attacks_to_process = Vec::new();

        for (monster, ai) in self.monsters.values_mut() {
            if monster.is_dead {
                if monster.respawn_timer_ms > 0 {
                    monster.respawn_timer_ms = monster.respawn_timer_ms.saturating_sub(delta_ms);
                    if monster.respawn_timer_ms == 0 {
                        // Renascimento do Monstro
                        monster.is_dead = false;
                        monster.hp = monster.max_hp;
                        monster.position = monster.spawn_center;
                    }
                }
                continue;
            }

            if let Some(attack) = ai.tick(monster, &self.players, delta_ms) {
                attacks_to_process.push(attack);
            }
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
            for player in self.players.values() {
                let _ = self
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
            }
            debug!("Autosave periódico de {} jogadores executado com sucesso no World #{}", self.players.len(), self.world_id);
        }
    }
}
