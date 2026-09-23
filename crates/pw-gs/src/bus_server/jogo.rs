//! O jogo em si, do lado da rede: missões, experiência dos abates, drop e coleta, loja,
//! aprender habilidade e renascer.
//!
//! As regras vivem em módulos puros — [`crate::missoes`], [`crate::progressao`],
//! [`crate::economia`] —; aqui elas encontram o banco (bolsas, listas) e o fio (comandos ao
//! cliente). Todo tratamento segue o mesmo desenho:
//!
//! 1. lê do banco o que a regra precisa e o mundo não guarda (as bolsas);
//! 2. com o mundo travado, roda a regra sobre um [`Contexto`] que junta jogador, bolsas e a
//!    fila de comandos;
//! 3. solta o mundo, manda os comandos e grava o que mudou.

use super::*;
use crate::economia::{self, Bolsa, TAMANHO_DA_BOLSA, TAMANHO_DA_BOLSA_DE_MISSAO, TID_DO_DINHEIRO};
use crate::missoes::{self, Jogador, ListasDeMissao, Motor};
use crate::progressao;
use pw_data_loader::GameDataManager;

/// `money_capacity` padrão (`MONEY_CAPACITY_BASE`, `gs/config.h`).
const TETO_DE_DINHEIRO: i64 = 2_000_000_000;
/// `COOLINGID_BEGIN` (`cskill/skill/playerwrapper.cpp:170`).
const INICIO_DAS_RECARGAS_DE_HABILIDADE: i32 = 1024;
/// `S2C::ERR_*` (`common/protocol.h:679-750`).
#[allow(dead_code)]
mod erro_s2c {
    pub const ITEM_NAO_NO_INVENTARIO: i32 = 5;
    pub const NAO_PODE_PEGAR: i32 = 6;
    pub const BOLSA_CHEIA: i32 = 7;
    pub const SERVICO_INDISPONIVEL: i32 = 14;
    pub const SEM_DINHEIRO: i32 = 16;
    pub const NAO_PODE_USAR_ITEM: i32 = 18;
    pub const MISSAO_INDISPONIVEL: i32 = 19;
    pub const HABILIDADE_INDISPONIVEL: i32 = 20;
    pub const NAO_PODE_APRENDER: i32 = 22;
    pub const MINA_OCUPADA: i32 = 30;
    pub const FERRAMENTA_ERRADA: i32 = 31;
    pub const NIVEL_NAO_BATE: i32 = 51;
    pub const HABILIDADE_EM_RECARGA: i32 = 53;
    pub const OPERACAO_EM_COMBATE: i32 = 66;
    pub const FORA_DE_ALCANCE: i32 = 2;
    pub const PET_NAO_PODE_CHOCAR: i32 = 76;
    /// `ERR_PET_IS_ALEARY_ACTIVE` 71, `ERR_PET_IS_NOT_EXIST` 72, `ERR_PET_IS_NOT_ACTIVE` 73
    /// e `ERR_PET_CAN_NOT_MOUNT` 81 (`common/protocol.h:748-761`).
    pub const PET_JA_ATIVO: i32 = 71;
    pub const PET_NAO_EXISTE: i32 = 72;
    pub const PET_NAO_ATIVO: i32 = 73;
    pub const PET_NAO_MONTA: i32 = 81;
    pub const CLASSE_INVALIDA: i32 = 90;
}
/// `TASK_CLT_NOTIFY_*` (`task/TaskTempl.h:103-108`).
mod aviso_do_cliente {
    pub const CONCLUIR: u8 = 1;
    pub const DESISTIR: u8 = 2;
    pub const CHEGOU_AO_LUGAR: u8 = 3;
    pub const ENTREGA_AUTOMATICA: u8 = 4;
    pub const GATILHO_MANUAL: u8 = 5;
    pub const SAIU_DO_LUGAR: u8 = 10;
}

/// `EQUIP_INDEX_WEAPON` (`EC_IvtrTypes.h:56-67`).
const SLOT_DA_ARMA: u16 = 0;
/// `EQUIP_INDEX_ELF` (`gs/item.h:219`): onde o Daimon é vestido.
pub(super) const SLOT_DO_DAIMON: u16 = 23;
/// `EQUIP_INDEX_HP_ADDON` (20) e `EQUIP_INDEX_MP_ADDON` (21) (`gs/item.h:216-217`).
pub(super) const SLOT_DO_AMULETO_DE_VIDA: u16 = 20;
pub(super) const SLOT_DO_AMULETO_DE_MANA: u16 = 21;
/// `EQUIP_ARMOR_START` (= `EQUIP_INDEX_HEAD`) e `EQUIP_ARMOR_END` (= `EQUIP_INDEX_PROJECTILE`)
/// do `gs/item.h:194-241`: os slots que `SelectRandomArmor` sorteia são de 1 a 10.
const PRIMEIRA_PECA: u16 = 1;
const DEPOIS_DA_ULTIMA_PECA: u16 = 11;
/// `DURABILITY_DEC_PER_ATTACK` (`gs/config.h:61`): o que a arma perde por golpe normal.
const DESGASTE_POR_GOLPE: i32 = 2;
/// `DURABILITY_DEC_PER_HIT` (`gs/config.h:60`): o que a peça sorteada perde por golpe
/// recebido. O cliente desconta o mesmo por conta própria (`ARMOR_RUIN_SPEED = -25`,
/// `EC_IvtrTypes.h:32`).
const DESGASTE_AO_APANHAR: i32 = 25;
/// `eq_index &= 0x7F` do `Make<be_attacked>` (`cgame/common/protocol_imp.h:580-590`): o -1 de
/// `SelectRandomArmor` vira isto, e o cliente lê como "nenhuma peça desgastada"
/// (`EC_HostMsg.cpp:974-981`).
pub(super) const NENHUMA_PECA: u8 = 0x7f;

fn agora() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

/// Jogador, bolsas e a fila de comandos de uma operação — o `PlayerTaskInterface` do
/// original (`task/taskman.cpp`).
pub(crate) struct Contexto<'a> {
    pub p: &'a mut PlayerEntity,
    pub dados: &'a GameDataManager,
    pub bolsa: Bolsa,
    pub bolsa_de_missao: Bolsa,
    pub para_mim: Vec<Vec<u8>>,
    pub para_todos: Vec<Vec<u8>>,
    pub gm: bool,
    pub subiu_de_nivel: bool,
    pub mudou: bool,
    /// O mapa deste servidor de mundo.
    pub mundo: i32,
    /// A equipe do jogador (capitão primeiro), com quem está neste mapa preenchido.
    pub equipe: Vec<missoes::MembroDaEquipe>,
    /// Teleporte pedido durante a operação — feito depois de gravar.
    pub teleporte: Option<(u32, [f32; 3])>,
    /// Monstros que a missão pediu para evocar perto do jogador.
    pub monstros_a_invocar: Vec<(u32, u32, u32, i32, bool)>,
}

impl Contexto<'_> {
    fn bolsa_de(&mut self, comum: bool) -> &mut Bolsa {
        if comum { &mut self.bolsa } else { &mut self.bolsa_de_missao }
    }

    /// `ReceiveExp`/`ReceiveTaskExp` + `IncExp`: aplica e anota a subida de nível.
    pub fn ganhar_exp(&mut self, exp: i64, sp: i64) {
        let niveis = progressao::receber_exp(self.p, exp, sp, self.dados);
        self.mudou = true;
        self.daimon_recebe(exp);
        if niveis > 0 {
            self.subiu_de_nivel = true;
            // `gplayer_dispatcher::level_up` difunde a quem vê o jogador, e o próprio recebe.
            let pacote = S2CGamedataSend::level_up(self.p.role_id).data;
            self.para_mim.push(pacote.clone());
            self.para_todos.push(pacote);
            info!("mundo: {} subiu para o nível {}", self.p.role_id, self.p.level);
        }
    }

    /// `ElfReceiveExp(exp / 10)` — o Daimon fica com **um décimo** da experiência do
    /// jogador, toda vez que ele ganha (`gs/player.cpp:2921-2928`, `player_imp.h:2471`).
    /// Subindo de nível, o cliente recebe a ficha nova do item; senão, só a barra
    /// (`ELF_EXP` 283, `item_elf.cpp:740-748`).
    fn daimon_recebe(&mut self, exp: i64) {
        let parte = (exp.max(0) / 10).clamp(0, u32::MAX as i64) as u32;
        if parte == 0 {
            return;
        }
        let nivel = self.p.level.clamp(0, i16::MAX as i32) as i16;
        let tabela = &self.dados.progressao;
        let Some(d) = self.p.daimon.as_mut() else { return };
        let (ganhou, subiu) = d.receber_exp(parte, nivel, nivel, |n| {
            tabela.exp_para_subir(n as i32).clamp(0, u32::MAX as i64) as u32
        });
        if !ganhou {
            return;
        }
        let (slot, item_id, bloco, exp_do_daimon) =
            (d.slot, d.item_id, d.estado.bloco(), d.estado.exp.min(i32::MAX as u32) as i32);
        if subiu {
            info!("mundo: o Daimon de {} subiu para o nível {}", self.p.role_id, d.estado.nivel);
            self.para_mim.push(
                S2CGamedataSend::item_info(
                    ContainerType::Equipment.pacote_do_cliente().unwrap_or(1),
                    slot as u8,
                    item_id as i32,
                    0,
                    0,
                    1,
                    &bloco,
                    None,
                )
                .data,
            );
        } else {
            self.para_mim.push(S2CGamedataSend::elf_exp(exp_do_daimon).data);
        }
    }

    pub fn ganhar_dinheiro(&mut self, n: i64) -> i64 {
        let antes = self.p.money;
        self.p.money = (self.p.money + n.max(0)).min(TETO_DE_DINHEIRO);
        self.mudou = true;
        self.p.money - antes
    }

    pub fn gastar_dinheiro(&mut self, n: i64) -> bool {
        if n < 0 || self.p.money < n {
            return false;
        }
        self.p.money -= n;
        self.mudou = true;
        true
    }
}

impl Jogador for Contexto<'_> {
    fn agora(&self) -> u32 {
        agora()
    }
    fn nivel(&self) -> u32 {
        self.p.level.max(0) as u32
    }
    fn classe(&self) -> u32 {
        self.p.cls as u32
    }
    fn masculino(&self) -> bool {
        self.p.gender != pw_core::Gender::Female
    }
    fn cultivo(&self) -> u32 {
        self.p.cultivation.max(0) as u32
    }
    fn reputacao(&self) -> i32 {
        self.p.reputacao
    }
    fn dinheiro(&self) -> u32 {
        self.p.money.clamp(0, u32::MAX as i64) as u32
    }
    fn e_gm(&self) -> bool {
        self.gm
    }
    fn contar(&self, tid: u32, comum: bool) -> u32 {
        if comum { self.bolsa.contar(tid) } else { self.bolsa_de_missao.contar(tid) }
    }
    fn slots_livres(&self, comum: bool) -> u32 {
        if comum { self.bolsa.livres() } else { self.bolsa_de_missao.livres() }
    }
    fn dar_item(&mut self, tid: u32, quantidade: u32, comum: bool, validade: i32) {
        if quantidade == 0 {
            return;
        }
        let dados = self.dados;
        let tipo = if comum { ContainerType::Inventory } else { ContainerType::TaskInventory };
        // Prêmio de missão passa pela geração do drop, como o `DeliverCommonItem` do original
        // (`task/taskman.cpp:281-303`): equipamento sai com essência e propriedades sorteadas.
        let Some(e) = self.bolsa_de(comum).empilhar_gerado(tid, quantidade, dados) else {
            warn!("mundo: missão quis dar {quantidade} do item {tid} a {}, e a bolsa está cheia", self.p.role_id);
            return;
        };
        let _ = validade;
        self.para_mim.push(
            S2CGamedataSend::task_deliver_item(tid as i32, 0, e.entrou, e.no_slot, tipo.pacote_do_cliente().unwrap_or(0), e.slot as u8)
                .data,
        );
    }
    fn tirar_item(&mut self, tid: u32, quantidade: u32, comum: bool) {
        let tipo = if comum { ContainerType::Inventory } else { ContainerType::TaskInventory };
        for (slot, n) in self.bolsa_de(comum).tirar(tid, quantidade) {
            // `DROP_TYPE_TASK` = 3 (`common/protocol.h:932`).
            self.para_mim.push(S2CGamedataSend::player_drop_item(tipo.pacote_do_cliente().unwrap_or(0), slot as u8, n, tid as i32, 3).data);
        }
    }
    fn dar_dinheiro(&mut self, n: u32) {
        let ganho = self.ganhar_dinheiro(n as i64);
        self.para_mim.push(S2CGamedataSend::task_deliver_money(ganho as u32, self.p.money as u32).data);
    }
    fn tirar_dinheiro(&mut self, n: u32) {
        let n = (n as i64).min(self.p.money);
        if self.gastar_dinheiro(n) {
            self.para_mim.push(S2CGamedataSend::spend_money(n as u32).data);
        }
    }
    fn dar_exp(&mut self, exp: u32, sp: u32) {
        self.ganhar_exp(exp as i64, sp as i64);
        self.para_mim.push(S2CGamedataSend::task_deliver_exp(exp as i32, sp as i32).data);
    }
    fn dar_reputacao(&mut self, r: i32) {
        self.p.reputacao += r;
        self.mudou = true;
    }
    /// `SetSecLevel` (`gs/player_imp.h:2798-2804`): guarda, marca para gravar e manda o
    /// `TASK_DELIVER_LEVEL2` — é ele que faz o cliente tocar o efeito do avanço de cultivo.
    /// `SetMaxAP` (`gs/actobject.h:1634-1640`): muda o teto e marca o estado para ir ao
    /// cliente. O chi atual continua onde estava, preso ao novo teto.
    fn definir_teto_de_chi(&mut self, teto: u32) {
        self.p.max_ap = teto as i32;
        self.p.ap = self.p.ap.min(self.p.max_ap);
        self.mudou = true;
        self.para_mim.push(crate::BusServer::estado_proprio_de(self.p));
    }

    fn definir_cultivo(&mut self, nivel: u32) {
        self.p.cultivation = nivel as i32;
        self.mudou = true;
        let roleid = self.p.role_id;
        self.para_mim
            .push(S2CGamedataSend::task_deliver_level2(roleid, nivel as i32).data);
    }
    fn avisar(&mut self, comando: Vec<u8>) {
        self.para_mim.push(comando);
    }
    fn posicao(&self) -> (u32, [f32; 3]) {
        let p = self.p.position;
        (self.mundo.max(0) as u32, [p.x, p.y, p.z])
    }
    fn equipe(&self) -> Vec<missoes::MembroDaEquipe> {
        self.equipe.clone()
    }
    fn teleportar(&mut self, mundo: u32, pos: [f32; 3]) {
        self.teleporte = Some((mundo, pos));
    }
    fn sortear(&mut self) -> f32 {
        use rand::Rng;
        rand::thread_rng().gen::<f32>()
    }
    fn invocar_monstro(&mut self, monstro_tid: u32, quantidade: u32, raio: u32, periodo_s: i32, some_ao_morrer: bool) {
        self.monstros_a_invocar.push((monstro_tid, quantidade, raio, periodo_s, some_ao_morrer));
    }
}

/// O que se grava de um jogador depois de uma operação.
struct Gravacao {
    roleid: i32,
    level: i32,
    cultivation: i32,
    exp: i64,
    sp: i64,
    hp: i32,
    mp: i32,
    money: i64,
    world_id: i32,
    pos: pw_core::Vector3,
    pontos: i32,
    atributos: (i32, i32, i32, i32),
    listas: [Vec<u8>; 5],
}

impl BusServer {
    /// Roda `f` sobre o jogador com as bolsas carregadas, manda os comandos e grava.
    ///
    /// `None` quando o jogador não está neste mundo.
    pub(crate) async fn com_contexto<R>(&self, roleid: i32, f: impl FnOnce(&mut Contexto) -> R) -> Option<R> {
        let (itens_repo, repo) = (self.itens().await, self.repo().await);
        let bolsa = itens_repo.list_by_container(roleid, ContainerType::Inventory).await.unwrap_or_default();
        let bolsa_de_missao = itens_repo.list_by_container(roleid, ContainerType::TaskInventory).await.unwrap_or_default();

        let (r, para_mim, para_todos, subiu, mut bolsas, gravacao, ficha, teleporte, daimon) = {
            let mut guarda = self.world.write().await;
            let mundo = &mut *guarda;
            let dados = Arc::clone(&mundo.data_manager);
            let world_id = mundo.world_id;
            // `GetTeamMemberInfo` (`taskman.cpp:372-390`): quem não está neste mapa vai com
            // mundo 0, que nunca é o de ninguém.
            let equipe: Vec<missoes::MembroDaEquipe> = mundo
                .membros_do_grupo(roleid)
                .iter()
                .map(|&m| match mundo.players.get(&(m as i64)) {
                    Some(o) => missoes::MembroDaEquipe {
                        id: m as u32,
                        nivel: o.level.max(0) as u32,
                        classe: o.cls as u32,
                        masculino: o.gender != pw_core::Gender::Female,
                        mundo: world_id.max(0) as u32,
                        pos: [o.position.x, o.position.y, o.position.z],
                    },
                    None => missoes::MembroDaEquipe { id: m as u32, nivel: 0, classe: u32::MAX, masculino: true, mundo: 0, pos: [0.0; 3] },
                })
                .collect();
            let p = mundo.players.get_mut(&(roleid as i64))?;
            let gm = p.sec_level > 0;
            let mut ctx = Contexto {
                p,
                dados: &dados,
                bolsa: Bolsa::nova(roleid, ContainerType::Inventory, TAMANHO_DA_BOLSA, bolsa),
                bolsa_de_missao: Bolsa::nova(roleid, ContainerType::TaskInventory, TAMANHO_DA_BOLSA_DE_MISSAO, bolsa_de_missao),
                para_mim: Vec::new(),
                para_todos: Vec::new(),
                gm,
                subiu_de_nivel: false,
                mudou: false,
                mundo: world_id,
                equipe,
                teleporte: None,
                monstros_a_invocar: Vec::new(),
            };
            let r = f(&mut ctx);
            let Contexto { p, bolsa, bolsa_de_missao, para_mim, para_todos, subiu_de_nivel, mudou, teleporte, monstros_a_invocar, .. } = ctx;
            let p_pos = p.position;
            let gravacao = Gravacao {
                roleid,
                level: p.level,
                cultivation: p.cultivation,
                exp: p.exp,
                sp: p.sp,
                hp: p.hp,
                mp: p.mp,
                money: p.money,
                world_id,
                pos: p_pos,
                pontos: p.pontos_de_atributo,
                atributos: (p.strength, p.agility, p.vitality, p.energy),
                listas: p.missoes.blocos(),
            };
            let ficha = (mudou || subiu_de_nivel).then(|| (self.ficha_propria(p), Self::estado_proprio_de(p)));
            // O bloco do Daimon é o estado dele: se mudou, vai ao banco junto do resto.
            let daimon = p.daimon.as_mut().filter(|d| d.sujo).map(|d| {
                d.sujo = false;
                (d.slot, d.estado.bloco())
            });

            for (monstro_tid, quantidade, raio, periodo_s, some_ao_morrer) in monstros_a_invocar {
                for _ in 0..quantidade {
                    mundo.invocar_monstro(monstro_tid, p_pos, raio, periodo_s, some_ao_morrer, Some(roleid as i64));
                }
            }

            (r, para_mim, para_todos, subiu_de_nivel, [bolsa, bolsa_de_missao], gravacao, ficha, teleporte, daimon)
        };

        for c in para_mim {
            self.enviar_ao_jogador(roleid, c).await;
        }
        for c in para_todos {
            self.transmitir_a_outros(roleid, c).await;
        }
        if let Some((ficha, estado)) = ficha {
            self.enviar_ao_jogador(roleid, estado).await;
            if subiu {
                self.enviar_ao_jogador(roleid, ficha).await;
            }
            let dinheiro = gravacao.money.clamp(0, u32::MAX as i64) as u32;
            self.enviar_ao_jogador(roleid, self.sub.get_own_money(dinheiro, TETO_DE_DINHEIRO as u32).data).await;
        }

        // Grava: bolsas primeiro (são o que o próximo pedido vai ler), o resto numa tarefa.
        for b in &mut bolsas {
            if let Err(e) = b.gravar(&itens_repo).await {
                warn!("mundo: não consegui gravar a bolsa de {roleid}: {e}");
            }
        }
        if let Some((slot, bloco)) = daimon {
            match itens_repo.get_item_by_slot(roleid, ContainerType::Equipment, slot).await {
                Ok(Some(mut i)) => {
                    i.octets = bloco;
                    if let Err(e) = itens_repo.upsert_item(&i).await {
                        warn!("mundo: não consegui gravar o Daimon de {roleid}: {e}");
                    }
                }
                Ok(None) => warn!("mundo: o Daimon de {roleid} sumiu do slot {slot} antes de gravar"),
                Err(e) => warn!("mundo: não consegui ler o Daimon de {roleid}: {e}"),
            }
        }
        let gravar = async move {
            let g = gravacao;
            if let Err(e) = repo
                .save_status(g.roleid, g.level, g.cultivation, g.exp, g.sp, g.hp, g.mp, g.money, g.world_id, &g.pos)
                .await
            {
                warn!("mundo: não consegui gravar o estado de {}: {e}", g.roleid);
            }
            if let Err(e) = repo.gravar_atributos(g.roleid, g.atributos, g.pontos).await {
                warn!("mundo: não consegui gravar os atributos de {}: {e}", g.roleid);
            }
            let [a, b, c, d, e] = g.listas;
            let listas = pw_storage::ListasDeMissaoGravadas { ativa: a, concluidas: b, tempos: c, contagens: d, deposito: e };
            if let Err(e) = repo.task_lists().gravar(g.roleid, &listas).await {
                warn!("mundo: não consegui gravar as missões de {}: {e}", g.roleid);
            }
        };
        // Com teleporte a gravação espera: o mapa de destino grava a posição nova, e uma
        // gravação atrasada daqui a sobrescreveria com a velha.
        match teleporte {
            Some((mundo, pos)) => {
                gravar.await;
                self.transportar(roleid, mundo as i32, pw_core::Vector3::new(pos[0], pos[1], pos[2])).await;
            }
            None => {
                tokio::spawn(gravar);
            }
        }
        Some(r)
    }

    /// `SELF_INFO_00` do jogador.
    pub(super) fn estado_proprio_de(p: &PlayerEntity) -> Vec<u8> {
        S2CGamedataSend::self_info_00(
            p.level as i16,
            p.cultivation.clamp(0, 255) as u8,
            p.combate_s > 0,
            p.hp,
            p.max_hp,
            p.mp,
            p.max_mp,
            p.exp.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            p.sp.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            p.ap,
            p.max_ap,
        )
        .data
    }

    /// `OWN_EXT_PROP` (50) com os números calculados — ver `todos_os_dados`.
    pub(crate) fn ficha_propria(&self, p: &PlayerEntity) -> Vec<u8> {
        self.sub.own_ext_prop(
            p.pontos_de_atributo.max(0) as u32,
            p.atributos_efetivos(),
            p.max_hp,
            p.max_mp,
            p.max_ap,
            (p.hp_gen, p.mp_gen),
            (p.walk_speed, p.move_speed, p.swim_speed, p.fly_speed),
            (p.attack_rate, p.attack_min, p.attack_max, (p.attack_speed * 20.0).round() as i32, p.attack_range),
            // Atq. Mágico da ficha: sem isto, um personagem mágico via o campo vazio mesmo
            // com arma mágica na mão (B77).
            (p.magic_attack_min, p.magic_attack_max),
            p.equipamento.resistencias,
            (p.def_phys, p.armor),
        )
        .data
    }

    // ------------------------------------------------------------------ coleta

    /// `C2S::GATHER_MATERIAL` (54) — `session_gather_prepare` + `GM_MSG_GATHER_REQUEST` da
    /// mina (`playercmd.cpp:2324-2386`, `actsession.cpp:1090-1126`, `matter.cpp:265-382`).
    ///
    /// Confere, na ordem da mina: coletores no limite ou o próprio já colhendo (30),
    /// ferramenta (31), nível (51), missão de entrada (31), distância (2). Aceito, sorteia o
    /// tempo entre `time_min` e `time_max` e difunde `PLAYER_GATHER_START`
    /// (`session_gather::StartSession`, `actsession.cpp:1150-1172`).
    pub(super) async fn coletar(&self, roleid: i32, conteudo: &[u8]) {
        let mut r = Reader::new(conteudo);
        let (Ok(mid), Ok(_onde), Ok(_indice), Ok(ferramenta), Ok(missao)) = (r.i32(), r.i16(), r.i16(), r.i32(), r.i32()) else {
            warn!("mundo: GATHER_MATERIAL de {roleid} com {} bytes (esperados 16)", conteudo.len());
            return;
        };
        let mid = mid as i64;
        let resposta = {
            let mut guarda = self.world.write().await;
            let mundo = &mut *guarda;
            let dados = Arc::clone(&mundo.data_manager);
            let Some(m) = mundo.matters.get(&mid).cloned() else { return };
            let Some(mina) = dados.minas.get(&m.template_id).cloned() else {
                debug!("mundo: {roleid} tentou colher {mid}, template {} sem MINE_ESSENCE válido", m.template_id);
                return;
            };
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            let (nivel, pos, ja) = (p.level, p.position, p.coleta);
            let coletores = mundo.coletores.entry(mid).or_default();
            let erro = if ja.is_some() || coletores.len() as u32 >= mina.coletores || coletores.contains(&roleid) {
                Some(erro_s2c::MINA_OCUPADA)
            } else if ferramenta != mina.ferramenta {
                Some(erro_s2c::FERRAMENTA_ERRADA)
            } else if nivel < mina.nivel {
                Some(erro_s2c::NIVEL_NAO_BATE)
            } else if missao as u32 != mina.missao_de_entrada {
                Some(erro_s2c::FERRAMENTA_ERRADA)
            } else if pos.distance(&m.position) >= mina.distancia {
                Some(erro_s2c::FORA_DE_ALCANCE)
            } else {
                None
            };
            match erro {
                Some(e) => Err(e),
                None => {
                    coletores.push(roleid);
                    if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
                        p.coleta = Some(mid);
                    }
                    use rand::Rng;
                    let t = rand::thread_rng().gen_range(mina.tempo_minimo..=mina.tempo_maximo).min(255);
                    Ok(t as u8)
                }
            }
        };
        let segundos = match resposta {
            Err(e) => {
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(e).data).await;
                return;
            }
            Ok(s) => s,
        };
        info!("mundo: {roleid} começou a colher {mid} ({segundos} s)");
        let inicio = S2CGamedataSend::player_gather_start(roleid, mid as i32, segundos).data;
        self.enviar_ao_jogador(roleid, inicio.clone()).await;
        self.transmitir_a_outros(roleid, inicio).await;
        let este = self.clone_arc();
        tokio::spawn(async move {
            // `SetTimer(g_timer, _gather_time*20, 1)`: `use_time` em segundos.
            tokio::time::sleep(std::time::Duration::from_secs(segundos as u64)).await;
            if let Some(este) = este {
                este.concluir_coleta(roleid, mid).await;
            }
        });
    }

    /// Movimento no meio da coleta encerra a sessão (`GM_MSG_GATHER_CANCEL`).
    pub(super) async fn interromper_coleta(&self, roleid: i32) {
        let parou = {
            let mut mundo = self.world.write().await;
            let Some(mid) = mundo.players.get_mut(&(roleid as i64)).and_then(|p| p.coleta.take()) else { return };
            if let Some(c) = mundo.coletores.get_mut(&mid) {
                c.retain(|x| *x != roleid);
            }
            mid
        };
        debug!("mundo: {roleid} interrompeu a coleta de {parou}");
        let fim = S2CGamedataSend::player_gather_stop(roleid).data;
        self.enviar_ao_jogador(roleid, fim.clone()).await;
        self.transmitir_a_outros(roleid, fim).await;
    }

    /// Fim do tempo: `GM_MSG_GATHER` na mina e `GM_MSG_GATHER_RESULT` no jogador
    /// (`matter.cpp:402-510`, `player.cpp:1478-1568`).
    async fn concluir_coleta(&self, roleid: i32, mid: i64) {
        use rand::Rng;
        let preparado = {
            let mut mundo = self.world.write().await;
            let dados = Arc::clone(&mundo.data_manager);
            let colhendo = mundo.players.get(&(roleid as i64)).is_some_and(|p| p.coleta == Some(mid));
            if !colhendo {
                return;
            }
            if let Some(p) = mundo.players.get_mut(&(roleid as i64)) {
                p.coleta = None;
            }
            if let Some(c) = mundo.coletores.get_mut(&mid) {
                c.retain(|x| *x != roleid);
            }
            let Some(m) = mundo.matters.get(&mid).cloned() else { return };
            let Some(mina) = dados.minas.get(&m.template_id).cloned() else { return };
            let mut rng = rand::thread_rng();
            if rng.gen::<f32>() >= mina.chance_de_sucesso {
                None
            } else {
                // `RandSelect(id_produce_prop)` e `Rand(0,1) < bonus_prop ? bonus : std`.
                let mut sorteio = rng.gen::<f32>();
                let material = mina
                    .materiais
                    .iter()
                    .find(|mat| {
                        if sorteio < mat.probabilidade {
                            true
                        } else {
                            sorteio -= mat.probabilidade;
                            false
                        }
                    })
                    .copied()
                    .unwrap_or(mina.materiais[mina.materiais.len() - 1]);
                let quantidade = if rng.gen::<f32>() < mina.chance_bonus { mina.quantidade_bonus } else { mina.quantidade };
                let quantidade = quantidade.min(dados.limite_de_pilha(material.item));
                if !mina.permanente {
                    mundo.colher_mina(mid);
                }
                Some((mina, material, quantidade, m.position))
            }
        };
        let fim = S2CGamedataSend::player_gather_stop(roleid).data;
        self.enviar_ao_jogador(roleid, fim.clone()).await;
        self.transmitir_a_outros(roleid, fim).await;
        let Some((mina, material, quantidade, pos)) = preparado else {
            debug!("mundo: {roleid} não conseguiu colher {mid}");
            return;
        };
        if !mina.permanente {
            // `gmatter_dispatcher::disappear` difunde `OBJECT_DISAPPEAR`.
            let some = S2CGamedataSend::object_disappear(mid as i32).data;
            self.enviar_ao_jogador(roleid, some.clone()).await;
            self.transmitir_a_outros(roleid, some).await;
        }
        let sobrou = self
            .com_contexto(roleid, |ctx| {
                if mina.gasta_ferramenta && mina.ferramenta > 0 {
                    for (slot, n) in ctx.bolsa.tirar(mina.ferramenta as u32, 1) {
                        // `DROP_TYPE_USE` = 10 (`common/protocol.h:931-943`).
                        ctx.para_mim.push(S2CGamedataSend::player_drop_item(0, slot as u8, n, mina.ferramenta, 10).data);
                    }
                }
                let mut sobra = if material.item > 0 { quantidade } else { 0 };
                if quantidade > 0 && material.item > 0 {
                    let dados = ctx.dados;
                    let eh_missao = dados.e_item_de_missao(material.item);
                    let where_pct = if eh_missao { 2 } else { 0 };
                    let bolsa = if eh_missao { &mut ctx.bolsa_de_missao } else { &mut ctx.bolsa };
                    // O original também cria o que se colhe por `generate_item_for_drop`
                    // (`player.cpp:1500-1520`).
                    match bolsa.empilhar_gerado(material.item, quantidade, dados) {
                        Some(e) => {
                            sobra = quantidade - e.entrou;
                            ctx.para_mim.push(S2CGamedataSend::obtain_item(material.item as i32, 0, e.entrou, e.no_slot, where_pct, e.slot as u8).data);
                        }
                        None => {}
                    }
                    if sobra > 0 {
                        ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::BOLSA_CHEIA).data);
                    }
                }
                if mina.exp != 0 || mina.sp != 0 {
                    ctx.ganhar_exp(mina.exp.max(0) as i64, mina.sp.max(0) as i64);
                }
                if mina.missao_de_saida > 0 {
                    let dados = ctx.dados;
                    Self::com_motor(ctx, dados, |m| m.colheu_mina(mina.missao_de_saida));
                }
                sobra
            })
            .await;
        if material.item > 0 {
            info!("mundo: {roleid} colheu {quantidade} do item {} da mina {mid}", material.item);
        } else {
            info!("mundo: {roleid} colheu mina de missão {mid} (missão {})", mina.missao_de_saida);
        }
        // A matéria pode **soltar monstros** ao ser colhida: são os `npcgen_1..4` do
        // `MINE_ESSENCE`. A Flor de Safira (44566) não produz item nenhum — o que ela faz é
        // acordar o Guardião de Almas (44608), e é dele que cai o Estame da missão 31779
        // (B76). Sem isto, colher a flor não fazia nada.
        if !mina.monstros_ao_colher.is_empty() {
            let mut mundo = self.world.write().await;
            for (tid, quantos, raio, vida_s) in mina.monstros_ao_colher.clone() {
                for _ in 0..quantos {
                    mundo.invocar_monstro(tid, pos, raio.max(0.0) as u32, vida_s, false, Some(roleid as i64));
                }
                info!("mundo: a mina {mid} acordou {quantos}× o monstro {tid} para {roleid}");
            }
        }
        // O que não coube vai ao chão, do jogador (`DropItemData`, `player.cpp:1530-1540`).
        if let Some(n) = sobrou.filter(|n| *n > 0 && material.item > 0) {
            let d = self.world.write().await.criar_drop(material.item, n, pos, Some(roleid));
            self.mostrar_drop(&d).await;
        }
    }

    // ------------------------------------------------------------------ montaria

    /// Ticks de canalização de cada operação de mascote. O tick do original é de 50 ms
    /// (`TICK_PER_SEC 20`, `gs/config.h:43`) e é essa a unidade que viaja no
    /// `PLAYER_START_PET_OP` (`EC_HostMsg.cpp:5350`, `SetPeriod(delay * 50)`).
    const TICKS_PARA_INVOCAR: i32 = 60; // `SetDelay(60)`, `gs/player.cpp:14485`
    const TICKS_PARA_RECOLHER: i32 = 10; // `SetDelay(10)`, `gs/player.cpp:14507`
    const MS_POR_TICK: u64 = 50;

    /// `SUMMON_PET` (C2S 100) — invocar o mascote do índice. **Montaria é montar**
    /// (`gplayer_imp::PlayerSummonPet` → `pet_man::ActivePet`, `gs/player.cpp:14474-14491`,
    /// `gs/petman.cpp:319-392`).
    ///
    /// Invocar **não é um comando, é uma sessão** (`session_summon_pet`): o original só
    /// confere que o mascote existe, abre a canalização com `PLAYER_START_PET_OP`, espera
    /// 60 ticks (3 s) e só então tenta a invocação; ao fim manda `PLAYER_STOP_PET_OP`
    /// (`session_pet_operation::OnStart`/`OnEnd`, `gs/actsession.cpp:1705-1721`).
    ///
    /// Os três comandos importam ao cliente: sem o `START`/`STOP` não há canalização nem
    /// animação, e sem o `SUMMON_PET` (233) que vem depois do efeito
    /// (`gs/petman.cpp:1337`) o cliente **não sabe qual mascote está ativo** — o botão de
    /// recolher da jaula fica desabilitado (`DlgPetList.cpp:227`) e o de invocar responde
    /// "o mascote já está ativo". Foi o que o teste em jogo do B78 mostrou.
    ///
    /// `falta`: mascote de combate (só montaria por enquanto), água, invisibilidade e a
    /// queda da montaria por lealdade.
    pub(super) async fn invocar_mascote(&self, roleid: i32, payload: &[u8], envio: &crate::bus_server::EnvioAoCliente) {
        if payload.len() < 4 {
            debug!("mundo: SUMMON_PET de {roleid} com {} bytes", payload.len());
            return;
        }
        let indice = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let indice = indice.min(u16::MAX as u32) as u16;
        // A **única** conferência antes da canalização, como no original: o mascote existe.
        // Note que o `PlayerSummonPet` deixa comentada a recusa por mascote já ativo — quem
        // já tem um recolhe o anterior dentro do `ActivePet` (`gs/petman.cpp:1308-1312`).
        let Ok(Some(item)) = self.itens().await.get_item_by_slot(roleid, ContainerType::PetCorral, indice).await
        else {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::PET_NAO_EXISTE).data).await;
            return;
        };
        let info = pw_core::InfoPet::do_bloco(&item.octets).unwrap_or_else(|| {
            let mut i = pw_core::InfoPet::default();
            i.pet_tid = item.item_id as i32;
            i
        });
        // O modelo é o `pet_vis_tid` quando existe (`gs/player.cpp:14483-14486`); o
        // `pet_tid` é o que o cliente confere contra a jaula.
        let tid = if info.pet_vis_tid > 0 { info.pet_vis_tid } else { info.pet_tid } as u32;

        let marcador = self.abrir_operacao_de_pet(roleid).await;
        self.responder(
            roleid,
            S2CGamedataSend::player_start_pet_op(indice as i32, tid as i32, Self::TICKS_PARA_INVOCAR, 0).data,
            envio,
        )
        .await;

        let este = self.clone();
        let envio = envio.clone();
        tokio::spawn(async move {
            let espera = Self::TICKS_PARA_INVOCAR as u64 * Self::MS_POR_TICK;
            tokio::time::sleep(std::time::Duration::from_millis(espera)).await;
            if !este.operacao_de_pet_ainda_e_minha(roleid, marcador).await {
                return;
            }
            este.montar(roleid, indice, tid, &info, &envio).await;
            // `OnEnd`: a canalização fecha mesmo quando a invocação foi recusada, senão o
            // cliente fica "operando mascote" para sempre.
            este.responder(roleid, S2CGamedataSend::player_stop_pet_op().data, &envio).await;
        });
    }

    /// `pet_manager::ActivePet` com um mascote de classe montaria (`gs/petman.cpp:1303-1348`,
    /// `mount_petdata_imp::DoActivePet`, `:319-392`): recusa o que não pode montar, põe o
    /// `mount_filter` — que manda `PLAYER_MOUNTING` e sobrepõe a velocidade — e só então
    /// manda o `SUMMON_PET` (233).
    async fn montar(
        &self,
        roleid: i32,
        indice: u16,
        tid: u32,
        info: &pw_core::InfoPet,
        envio: &crate::bus_server::EnvioAoCliente,
    ) {
        let (estado, velocidade) = {
            let mundo = self.world.read().await;
            let velocidade = mundo.data_manager.velocidade_da_montaria(tid, info.level as i32);
            let estado = mundo
                .players
                .get(&(roleid as i64))
                .map(|p| (p.montaria, p.voando, mundo.esta_na_agua(p.position)));
            (estado, velocidade)
        };
        let Some((ja_montado, voando, na_agua)) = estado else { return };
        // `CalcMountParam` sem resposta é "não é montaria": mascote de combate ainda não tem
        // porte (`gs/petman.cpp:371-378`).
        let Some(velocidade) = velocidade.filter(|v| *v > 0.0) else {
            debug!("mundo: o pet {tid} de {roleid} não é montaria (ou não está no elements)");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::PET_NAO_MONTA).data).await;
            return;
        };
        if voando {
            // `IsOnGround` (`gs/petman.cpp:330-334`).
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::PET_NAO_MONTA).data).await;
            return;
        }
        if na_agua {
            // `IsUnderWater()` (`gs/petman.cpp:344-348`) — montaria terrestre não entra na
            // água. O limiar é o do `TestUnderWater`: meio metro abaixo da superfície (B88).
            debug!("mundo: {roleid} tentou montar dentro d'água");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::PET_NAO_MONTA).data).await;
            return;
        }
        // Já montado em outra: o original recolhe a anterior antes de pôr a nova.
        if let Some(anterior) = ja_montado.filter(|m: &crate::entity::MontariaAtiva| m.indice != indice) {
            self.desmontar(roleid, anterior, envio).await;
        }

        let cor = info.color;
        let montaria = crate::entity::MontariaAtiva {
            indice,
            tid,
            pet_tid: info.pet_tid as u32,
            cor,
            velocidade,
        };
        if let Some(p) = self.world.write().await.players.get_mut(&(roleid as i64)) {
            p.montaria = Some(montaria);
            p.move_speed = velocidade;
        }
        info!("mundo: {roleid} montou o pet {tid} (nível {}) a {velocidade:.2} m/s", info.level);
        let pacote = S2CGamedataSend::player_mounting(roleid, tid as i32, cor).data;
        self.responder(roleid, pacote.clone(), envio).await;
        self.transmitir_a_outros(roleid, pacote).await;
        // O `summon_pet` vem **depois** do efeito (`gs/petman.cpp:1337`): `pet_pid` 0 porque
        // montaria não põe criatura no mundo, e `life_time` 0 porque não tem prazo.
        self.responder(roleid, S2CGamedataSend::summon_pet(indice as i32, info.pet_tid, 0, 0).data, envio).await;
        // `SendClientCurSpeed` do original; aqui a ficha inteira, que leva as quatro
        // velocidades (`OWN_EXT_PROP`).
        let ficha = {
            let mundo = self.world.read().await;
            mundo.players.get(&(roleid as i64)).map(|p| self.ficha_propria(p))
        };
        if let Some(f) = ficha {
            self.responder(roleid, f, envio).await;
        }
    }

    /// `RECALL_PET` (C2S 101) — desmontar, também por sessão (`session_recall_pet`,
    /// `gplayer_imp::PlayerRecallPet`, `gs/player.cpp:14492-14512`), com 10 ticks (0,5 s)
    /// de canalização.
    pub(super) async fn recolher_mascote(&self, roleid: i32, envio: &crate::bus_server::EnvioAoCliente) {
        let montaria = self.world.read().await.players.get(&(roleid as i64)).and_then(|p| p.montaria);
        // `IsPetActive` antes de abrir a sessão (`gs/player.cpp:14494-14495`).
        let Some(montaria) = montaria else {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::PET_NAO_ATIVO).data).await;
            return;
        };
        let marcador = self.abrir_operacao_de_pet(roleid).await;
        self.responder(
            roleid,
            S2CGamedataSend::player_start_pet_op(
                montaria.indice as i32,
                montaria.tid as i32,
                Self::TICKS_PARA_RECOLHER,
                1,
            )
            .data,
            envio,
        )
        .await;

        let este = self.clone();
        let envio = envio.clone();
        tokio::spawn(async move {
            let espera = Self::TICKS_PARA_RECOLHER as u64 * Self::MS_POR_TICK;
            tokio::time::sleep(std::time::Duration::from_millis(espera)).await;
            if !este.operacao_de_pet_ainda_e_minha(roleid, marcador).await {
                return;
            }
            este.desmontar(roleid, montaria, &envio).await;
            este.responder(roleid, S2CGamedataSend::player_stop_pet_op().data, &envio).await;
        });
    }

    /// `pet_manager::RecallPetWithoutFree` com montaria (`gs/petman.cpp:1359-1390`): tira o
    /// `mount_filter` — que manda `PLAYER_MOUNTING(0, 0)` e devolve a velocidade — e então
    /// manda o `RECALL_PET` (234) com o motivo padrão.
    pub(super) async fn desmontar(
        &self,
        roleid: i32,
        montaria: crate::entity::MontariaAtiva,
        envio: &crate::bus_server::EnvioAoCliente,
    ) {
        let tinha = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            p.montaria.take().is_some()
        };
        if !tinha {
            return;
        }
        // A velocidade volta ao que o equipamento e a classe dizem, e a ficha nova vai junto.
        self.recalcular_equipamento(roleid, true).await;
        info!("mundo: {roleid} desmontou");
        let pacote = S2CGamedataSend::player_mounting(roleid, 0, 0).data;
        self.responder(roleid, pacote.clone(), envio).await;
        self.transmitir_a_outros(roleid, pacote).await;
        // `PET_RECALL_DEFAULT` = 0 (`Network/EC_GPDataType.h:3456-3462`).
        self.responder(roleid, S2CGamedataSend::recall_pet(montaria.indice as i32, montaria.pet_tid as i32, 0).data, envio)
            .await;
    }

    /// Abre uma operação de mascote e devolve o marcador dela: quem chegar depois invalida
    /// a canalização de quem estava esperando, como o `AddSession` do original faz com a
    /// sessão anterior.
    async fn abrir_operacao_de_pet(&self, roleid: i32) -> u64 {
        let mut mundo = self.world.write().await;
        match mundo.players.get_mut(&(roleid as i64)) {
            Some(p) => {
                p.operacao_de_pet += 1;
                p.operacao_de_pet
            }
            None => 0,
        }
    }

    async fn operacao_de_pet_ainda_e_minha(&self, roleid: i32, marcador: u64) -> bool {
        self.world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .is_some_and(|p| p.operacao_de_pet == marcador)
    }

    // ------------------------------------------------------------------ equipamento

    /// `RefreshEquipment`: relê o equipamento vestido e refaz alcance, cadência, dano, defesa,
    /// evasão, vida e mana. Com `avisar`, manda a ficha (`OWN_EXT_PROP`) e o estado
    /// (`SELF_INFO_00`) — é por eles que o cliente sabe até onde andar antes de atacar.
    pub(super) async fn recalcular_equipamento(&self, roleid: i32, avisar: bool) {
        let itens = self
            .itens()
            .await
            .list_by_container(roleid, ContainerType::Equipment)
            .await
            .unwrap_or_default();
        let pacotes = {
            let mut mundo = self.world.write().await;
            let dados = Arc::clone(&mundo.data_manager);
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            let e = crate::entity::Equipamento::dos_itens_com_addons(&itens, &dados.equipamentos, Some(&dados.addons));
            // A durabilidade de cada peça passa a viver no mundo, como o `_equipment` do
            // original: é daqui que sai o índice do `be_damaged` e a quebra, sem ida ao
            // banco no meio do golpe (B72).
            p.pecas = [None; crate::entity::PECAS_VESTIDAS];
            for item in &itens {
                let slot = item.slot as usize;
                if slot < crate::entity::PECAS_VESTIDAS && item.max_durability > 0 {
                    p.pecas[slot] = Some((item.durability as i32, item.max_durability as i32));
                }
            }
            // Amuleto e hierograma: vesti-los é ativá-los (`OnPutIn` → `Activate` →
            // `SetHPAutoGen`/`SetMPAutoGen`, `gs/item/item_amulet.h:53-60`,
            // `item_amulet.cpp:22-46`). O que resta vem dos octetos do item, porque é lá que
            // o gasto fica gravado; sem octetos vale o total do `elements.data`.
            // Daimon no slot 23 (`EQUIP_INDEX_ELF`, `gs/item.h:219`): o estado dele vive no
            // bloco do item, e é dele que sai a experiência (B75).
            p.daimon = itens.iter().find(|i| i.slot == SLOT_DO_DAIMON).and_then(|item| {
                let (fator, iniciais) = dados.dados_do_daimon(item.item_id)?;
                let estado = crate::entity::Daimon::ler(&item.octets)
                    .unwrap_or_else(|| crate::entity::Daimon::novo(&iniciais));
                Some(crate::entity::DaimonVestido {
                    slot: item.slot,
                    item_id: item.item_id,
                    fator_de_exp: fator,
                    estado,
                    sujo: item.octets.is_empty(),
                })
            });
            p.auto_hp = None;
            p.auto_mp = None;
            for item in &itens {
                if item.slot != SLOT_DO_AMULETO_DE_VIDA && item.slot != SLOT_DO_AMULETO_DE_MANA {
                    continue;
                }
                let Some((total, gatilho_padrao, recarga_ms, de_vida)) = dados.dados_do_amuleto(item.item_id) else {
                    continue;
                };
                let (ponto, gatilho) = crate::entity::AmuletoAtivo::do_bloco(&item.octets)
                    .unwrap_or((total, gatilho_padrao));
                let a = crate::entity::AmuletoAtivo {
                    slot: item.slot,
                    item_id: item.item_id,
                    ponto,
                    gatilho,
                    recarga_ms,
                };
                if de_vida {
                    p.auto_hp = Some(a);
                } else {
                    p.auto_mp = Some(a);
                }
            }
            if !e.addons_sem_porte.is_empty() {
                debug!("mundo: {roleid} veste addons sem porte: {:?}", e.addons_sem_porte);
            }
            let base = Some(&dados.base_das_classes).filter(|b| !b.is_empty());
            p.vestir(e, &dados.classes, base);
            debug!(
                "mundo: {roleid} equipado — dano {}..{}, alcance {:.1}, golpe {:.2} s, defesa {}, evasão {}",
                p.attack_min, p.attack_max, p.attack_range, p.attack_speed, p.def_phys, p.armor
            );
            (self.ficha_propria(p), Self::estado_proprio_de(p))
        };
        if avisar {
            self.enviar_ao_jogador(roleid, pacotes.1).await;
            self.enviar_ao_jogador(roleid, pacotes.0).await;
        }
    }

    // -------------------------------------------------------------- durabilidade

    /// A arma perde `DURABILITY_DEC_PER_ATTACK` a cada **golpe normal**.
    ///
    /// `gplayer_imp::FillAttackMsg` chama `DoWeaponOperation<0>()` (`player.cpp:3133`), que
    /// é o `weapon_item::OnAfterAttack` (`item/equip_item.cpp:978-988`). O
    /// `FillEnchantMsg` **não** chama — habilidade não gasta arma, e o original diz isso em
    /// comentário (`player.cpp:3174`). O cliente desconta o mesmo sozinho
    /// (`WEAPON_RUIN_SPEED = -2`, `EC_IvtrTypes.h:30`).
    pub(super) async fn gastar_arma(&self, roleid: i32) {
        self.desgastar(roleid, SLOT_DA_ARMA, DESGASTE_POR_GOLPE).await;
    }

    /// Tira `quanto` da peça do slot, **em memória**, e manda o banco acompanhar depois.
    ///
    /// O original mexe na `item_list` vestida e segue (`DoWeaponOperation`, `OnDamage`); o
    /// banco aqui é só persistência. Devolve o que sobrou, ou `None` se o slot está vazio ou
    /// a peça não tem durabilidade.
    async fn desgastar(&self, roleid: i32, slot: u16, quanto: i32) -> Option<(i32, i32)> {
        let (restou, maxima, quebrou) = {
            let mut mundo = self.world.write().await;
            let p = mundo.players.get_mut(&(roleid as i64))?;
            let (atual, maxima) = (*p.pecas.get(slot as usize)?)?;
            let restou = (atual - quanto).max(0);
            p.pecas[slot as usize] = Some((restou, maxima));
            (restou, maxima, atual > 0 && restou == 0)
        };
        if quebrou {
            info!("mundo: a peça {slot} de {roleid} quebrou (0/{maxima})");
            self.equipamento_quebrou(roleid, slot as u8).await;
        } else {
            trace!("mundo: peça {slot} de {roleid} em {restou}/{maxima}");
        }
        // A gravação sai do caminho da resposta: o `UPDATE` de durabilidade já levou mais de
        // um segundo no banco de teste, e com ele no fio o cliente ficava esperando (B72).
        if let Some(este) = self.clone_arc() {
            tokio::spawn(async move {
                let repo = este.itens().await;
                if let Err(e) = repo.gastar_durabilidade(roleid, ContainerType::Equipment, slot, quanto).await {
                    warn!("mundo: não consegui desgastar a peça {slot} de {roleid}: {e}");
                }
            });
        }
        Some((restou, maxima))
    }

    /// Uma peça sorteada perde `DURABILITY_DEC_PER_HIT` a cada golpe recebido, e o índice
    /// dela é o que vai no `HOST_ATTACKED`.
    ///
    /// `gplayer_imp::OnDamage` (`player.cpp:9552-9570`): `SelectRandomArmor` é
    /// `abase::Rand(EQUIP_ARMOR_START, EQUIP_ARMOR_END - 1)` — slots 1 a 10 — e devolve -1
    /// quando o slot sorteado está vazio; aí nada é desgastado e o cliente recebe
    /// [`NENHUMA_PECA`].
    pub(super) async fn desgastar_peca(&self, roleid: i32) -> u8 {
        use rand::Rng;
        let slot = rand::thread_rng().gen_range(PRIMEIRA_PECA..DEPOIS_DA_ULTIMA_PECA);
        match self.desgastar(roleid, slot, DESGASTE_AO_APANHAR).await {
            Some(_) => slot as u8,
            // Slot vazio (ou item sem durabilidade): o original manda -1, que o
            // `Make<be_attacked>` reduz a 0x7f.
            None => NENHUMA_PECA,
        }
    }

    /// `_runner->equipment_damaged(index, 0)` + `RefreshEquipment` (`player.cpp:9563-9567`):
    /// avisa o cliente de que a peça acabou e remanda os atributos, que agora vão sem ela —
    /// `equip_item::VerifyRequirement` exige `durability > 0` (`item/equip_item.cpp:60-80`).
    async fn equipamento_quebrou(&self, roleid: i32, slot: u8) {
        self.enviar_ao_jogador(roleid, S2CGamedataSend::equip_damaged(slot, 0).data).await;
        self.recalcular_equipamento(roleid, true).await;
    }

    // ------------------------------------------------------------------ atributos

    /// `C2S::SET_STATUS_POINT` (22) — ver [`PlayerEntity::distribuir_pontos`].
    pub(super) async fn distribuir_pontos(&self, roleid: i32, conteudo: &[u8]) {
        let mut r = Reader::new(conteudo);
        let (Ok(vit), Ok(eng), Ok(str_), Ok(agi)) = (r.u32(), r.u32(), r.u32(), r.u32()) else {
            warn!("mundo: SET_STATUS_POINT de {roleid} com {} bytes (esperados 16)", conteudo.len());
            return;
        };
        self.com_contexto(roleid, |ctx| {
            let base = Some(&ctx.dados.base_das_classes).filter(|b| !b.is_empty());
            let aceito = ctx.p.distribuir_pontos((vit, eng, str_, agi), &ctx.dados.classes, base);
            let restantes = ctx.p.pontos_de_atributo.max(0) as u32;
            let pacote = if aceito {
                ctx.mudou = true;
                info!("mundo: {roleid} distribuiu pontos (vit {vit}, eng {eng}, for {str_}, agi {agi}), sobram {restantes}");
                S2CGamedataSend::add_status_point(vit, eng, str_, agi, restantes)
            } else {
                debug!("mundo: {roleid} tentou gastar {} pontos tendo {restantes}", vit as u64 + eng as u64 + str_ as u64 + agi as u64);
                S2CGamedataSend::add_status_point(0, 0, 0, 0, restantes)
            };
            ctx.para_mim.push(pacote.data);
        })
        .await;
    }

    // ------------------------------------------------------------------ missões

    /// O NPC com quem o jogador está falando, e os serviços dele.
    async fn npc_em_conversa(&self, roleid: i32) -> Option<(u32, pw_data_loader::ServicosDoNpc)> {
        let mundo = self.world.read().await;
        let npc = mundo.players.get(&(roleid as i64))?.npc_em_conversa?;
        let tid = mundo.npcs.get(&npc)?.template_id;
        let s = mundo.data_manager.servicos_de_npc.get(&tid).cloned()?;
        Some((tid, s))
    }

    /// `GP_NPCSEV_TASK_ACCEPT` — `{ int idTask; int idStorage; int idRefreshItem; }`.
    ///
    /// `task_out_provider::TryServe` (`serviceprovider.cpp:1088-1116`) só atende missão da
    /// lista do NPC; `OnTaskCheckDeliver` (`TaskServer.cpp:526-583`) troca uma submissão pela
    /// missão-mãe, com a submissão como escolha.
    pub(super) async fn aceitar_missao(&self, roleid: i32, conteudo: &[u8]) {
        let Some(id) = npc::id_da_missao(conteudo) else { return };
        let id = id as u32;
        let Some((npc_tid, servicos)) = self.npc_em_conversa(roleid).await else {
            warn!("mundo: {roleid} pediu a missão {id} sem NPC em conversa");
            return;
        };
        if servicos.missoes_entregues.binary_search(&id).is_err() {
            debug!("mundo: o NPC {npc_tid} não entrega a missão {id}");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::MISSAO_INDISPONIVEL).data).await;
            return;
        }
        let dados = self.world.read().await.data_manager.clone();
        let r = self
            .com_contexto(roleid, |ctx| {
                let Some(t) = dados.tasks.get_task(id) else { return Some(missoes::erro::INDETERMINADO) };
                let (topo, sub) = match t.parent {
                    Some(p) => (p, id),
                    None => (id, 0),
                };
                Some(Self::com_motor(ctx, &dados, |m| m.aceitar(topo, sub, true)))
            })
            .await
            .flatten();
        match r {
            Some(0) => {
                info!("mundo: {roleid} aceitou a missão {id}");
                self.entregar_a_equipe(roleid, id, &dados).await;
            }
            Some(e) => {
                info!("mundo: {roleid} não pode aceitar a missão {id} (erro {e})");
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::MISSAO_INDISPONIVEL).data).await;
            }
            None => {}
        }
    }

    /// `DeliverTeamMemTask` (`TaskTempl.inl:2304-2321`): missão de equipe aceita pelo capitão
    /// vai a cada membro (`TASK_PLY_NOTIFY_NEW_MEM_TASK` → `OnDeliverTeamMemTask`). Só alcança
    /// quem está neste servidor de mundo.
    async fn entregar_a_equipe(&self, roleid: i32, id: u32, dados: &GameDataManager) {
        let Some(t) = dados.tasks.get_task(id) else { return };
        let topo = t.parent.unwrap_or(id);
        if !dados.tasks.get_task(topo).is_some_and(|t| t.em_equipe && t.recebida_pela_equipe) {
            return;
        }
        let membros = self.world.read().await.membros_do_grupo(roleid);
        for m in membros.into_iter().filter(|&m| m != roleid) {
            let r = self.com_contexto(m, |ctx| Self::com_motor(ctx, dados, |mt| mt.aceitar_como_membro(topo))).await;
            match r {
                Some(0) => info!("mundo: {m} recebeu a missão {topo} da equipe de {roleid}"),
                Some(e) => debug!("mundo: {m} não recebeu a missão {topo} da equipe (erro {e})"),
                None => debug!("mundo: {m}, da equipe de {roleid}, não está neste mapa"),
            }
        }
    }

    /// `GP_NPCSEV_TASK_RETURN` — `{ int idTask; int iChoice; }` (`task_in_provider`,
    /// `serviceprovider.cpp:998-1056`, e `OnTaskCheckAward`).
    pub(super) async fn entregar_missao(&self, roleid: i32, conteudo: &[u8]) {
        let mut r = Reader::new(conteudo);
        let (Ok(id), escolha) = (r.i32(), r.i32().unwrap_or(0)) else { return };
        let id = id as u32;
        let Some((npc_tid, servicos)) = self.npc_em_conversa(roleid).await else {
            warn!("mundo: {roleid} quis entregar a missão {id} sem NPC em conversa");
            return;
        };
        if servicos.missoes_recebidas.binary_search(&id).is_err() {
            debug!("mundo: o NPC {npc_tid} não recebe a missão {id}");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::MISSAO_INDISPONIVEL).data).await;
            return;
        }
        let dados = self.world.read().await.data_manager.clone();
        let ok = self
            .com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.entregar_no_npc(id, escolha)))
            .await
            .unwrap_or(false);
        info!("mundo: {roleid} entregou a missão {id}: {}", if ok { "premiada" } else { "recusada" });
    }

    /// `TASK_NOTIFY` (49) com os motivos que o motor trata (`OnClientNotify`,
    /// `TaskServer.cpp:330-401`). Devolve `false` para os que não são daqui.
    pub(super) async fn aviso_de_missao(&self, roleid: i32, motivo: u8, id: u32) -> bool {
        let dados = self.world.read().await.data_manager.clone();
        match motivo {
            aviso_do_cliente::CONCLUIR => {
                self.com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.conferir_conclusao(id))).await;
            }
            aviso_do_cliente::DESISTIR => {
                self.com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.desistir(id))).await;
            }
            aviso_do_cliente::CHEGOU_AO_LUGAR | aviso_do_cliente::SAIU_DO_LUGAR => {
                let saida = motivo == aviso_do_cliente::SAIU_DO_LUGAR;
                let feito = self.com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.conferir_lugar(id, saida))).await;
                if feito == Some(true) {
                    info!("mundo: {roleid} {} o lugar da missão {id}", if saida { "saiu d" } else { "chegou a" });
                }
            }
            aviso_do_cliente::ENTREGA_AUTOMATICA => {
                // `OnTaskAutoDelv` (`TaskServer.cpp:452-464`).
                if dados.tasks.get_task(id).is_some_and(|t| t.parent.is_none() && t.entrega_automatica) {
                    self.com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.aceitar(id, 0, false))).await;
                }
            }
            aviso_do_cliente::GATILHO_MANUAL => {
                // `OnTaskManualTrig` (`TaskServer.cpp:585-592`): só missão sem NPC que entrega.
                if dados.tasks.get_task(id).is_some_and(|t| t.parent.is_none() && t.npc_que_entrega == 0) {
                    self.com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.aceitar(id, 0, true))).await;
                }
            }
            _ => return false,
        }
        true
    }

    /// `item_taskdice::OnUse` (`gs/item/item_taskdice.cpp:12-42`): em combate a carta com
    /// `no_use_in_combat` é recusada (`ERR_INVALID_OPERATION_IN_COMBAT`); senão sorteia a
    /// missão (`generate_task_id`, a mesma do `Load` de um item sem dados) e tenta entregá-la
    /// (`OnTaskCheckDeliver` → `CheckDeliverTask`). Deu certo: gasta uma (`return 1`). Não
    /// deu: `ERR_CANNOT_USE_ITEM` e a carta fica.
    pub(super) async fn usar_carta_de_missao(
        &self,
        roleid: i32,
        u: &crate::comandos::UseItem,
        carta: &pw_data_loader::cartas::CartaDeMissao,
        envio: &crate::bus_server::EnvioAoCliente,
    ) {
        let em_combate = self.world.read().await.players.get(&(roleid as i64)).is_some_and(|p| p.combate_s > 0);
        if em_combate && carta.nao_usa_em_combate {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::OPERACAO_EM_COMBATE).data).await;
            return;
        }
        let Some(missao) = carta.sortear(rand::random::<f32>()) else {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::NAO_PODE_USAR_ITEM).data).await;
            return;
        };
        let dados = self.world.read().await.data_manager.clone();
        let r = self
            .com_contexto(roleid, |ctx| Self::com_motor(ctx, &dados, |m| m.aceitar(missao, 0, true)))
            .await
            .unwrap_or(u32::MAX);
        if r != 0 {
            info!("mundo: {roleid} usou a carta {} — missão {missao} recusada ({r})", u.item_id);
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::NAO_PODE_USAR_ITEM).data).await;
            return;
        }
        if self.itens().await.consume_item(roleid, ContainerType::Inventory, u.slot, 1).await.is_err() {
            warn!("mundo: {roleid} recebeu a missão {missao} mas a carta {} não foi gasta", u.item_id);
            return;
        }
        info!("mundo: {roleid} usou a carta {} e recebeu a missão {missao}", u.item_id);
        self.responder(roleid, S2CGamedataSend::host_use_item(u.onde, u.slot as u8, u.item_id, 1).data, envio).await;
        self.responder(roleid, S2CGamedataSend::unfreeze_ivtr_slot(u.onde, u.slot).data, envio).await;
    }

    /// Roda uma operação do motor sobre as listas do jogador do contexto.
    fn com_motor<R>(ctx: &mut Contexto, dados: &GameDataManager, op: impl FnOnce(&mut Motor<Contexto>) -> R) -> R {
        let mut listas: ListasDeMissao = std::mem::take(&mut ctx.p.missoes);
        let r = {
            let eu = ctx.p.role_id as u32;
            let mut m = Motor { tarefas: &dados.tasks, listas: &mut listas, j: ctx, eu };
            op(&mut m)
        };
        ctx.p.missoes = listas;
        ctx.mudou = true;
        r
    }

    // ------------------------------------------------------------------ abate

    /// Um monstro morreu: experiência para quem bateu, missão do dono e o drop.
    ///
    /// `gnpc_imp::OnDeath` (`npc.cpp:1360-1461`): `DispatchExp` reparte a experiência pelo
    /// dano, o dono (maior dano, com bônus do primeiro golpe) recebe o crédito de missão e o
    /// drop (`DropItem`).
    pub(super) async fn monstro_morreu(&self, alvo: i64) {
        self.encerrar_ataques_ao_alvo(alvo).await;
        let (tid, nivel_do_monstro, pos, partes, dono, modelo) = {
            let mut mundo = self.world.write().await;
            let dados = Arc::clone(&mundo.data_manager);
            let Some((m, _)) = mundo.monsters.get(&alvo) else { return };
            let dono = progressao::dono_do_abate(m);
            let partes: Vec<(i32, i64, i64)> = m
                .danos
                .iter()
                .filter_map(|(quem, _)| {
                    let nivel = mundo.players.get(quem)?.level;
                    let (exp, sp) = progressao::parte_do_abate(m, *quem, nivel, &dados.progressao);
                    Some((*quem as i32, exp, sp))
                })
                .collect();
            let info = (m.template_id, m.level, m.position, partes, dono, dados.monstros.get(m.template_id).cloned());
            mundo.matar_monstro(alvo);
            if let Some((m, _)) = mundo.monsters.get_mut(&alvo) {
                m.danos.clear();
                m.primeiro_atacante = None;
            }
            info
        };

        for (quem, exp, sp) in partes {
            if exp + sp <= 0 {
                continue;
            }
            self.com_contexto(quem, |ctx| {
                let (exp0, sp0) = (ctx.p.exp, ctx.p.sp);
                ctx.ganhar_exp(exp, sp);
                // `_runner->receive_exp(exp, sp)` depois do `IncExp` (`player.cpp:2924`).
                let _ = (exp0, sp0);
                ctx.para_mim.push(self.sub.receive_exp(exp as i32, sp as i32).data);
            })
            .await;
        }

        let Some(dono) = dono else { return };
        let dono = dono as i32;
        let dados = self.world.read().await.data_manager.clone();
        let nivel_do_dono = self
            .com_contexto(dono, |ctx| {
                Self::com_motor(ctx, &dados, |m| m.abateu_monstro(tid, nivel_do_monstro.max(0) as u32));
                ctx.p.level
            })
            .await;

        let (Some(modelo), Some(nivel_do_dono)) = (modelo, nivel_do_dono) else { return };
        let queda = economia::gerar_queda(&modelo, nivel_do_dono, &dados, &mut rand::thread_rng());
        let mut criados = Vec::new();
        {
            let mut mundo = self.world.write().await;
            for tid in &queda.itens {
                // Aljava vira a munição que ela contém (`generate_quiver`,
                // `generate_item_temp.h:650-667`): o drop do 1955 era o id cru da aljava.
                let (tid, n) = match mundo.data_manager.aljavas.get(tid) {
                    Some(&(municao, minimo, maximo)) => {
                        use rand::Rng;
                        (municao, if minimo >= maximo { minimo } else { rand::thread_rng().gen_range(minimo..=maximo) })
                    }
                    None => (*tid, 1),
                };
                // Equipamento sai sorteado: essência, furos e propriedades adicionais
                // (`generate_weapon/armor/decoration` com `ADDON_LIST_DROP`).
                let octetos = crate::geracao::gerar_equipamento(&mundo.data_manager, tid).map(|c| c.escrever()).unwrap_or_default();
                criados.push(mundo.criar_drop_com_octetos(tid, n, pos, Some(dono), octetos));
            }
            for n in &queda.montes_de_dinheiro {
                criados.push(mundo.criar_drop(TID_DO_DINHEIRO, *n, pos, Some(dono)));
            }
        }
        if !criados.is_empty() {
            debug!("mundo: {alvo} deixou {} item(ns) e {} monte(s) de moedas", queda.itens.len(), queda.montes_de_dinheiro.len());
        }
        for d in criados {
            self.mostrar_drop(&d).await;
        }
    }

    /// `MATTER_ENTER_WORLD` para quem está perto, já anotado como visto.
    pub(super) async fn mostrar_drop(&self, d: &crate::entity::ItemDropEntity) {
        let perto: Vec<i32> = {
            let mut mundo = self.world.write().await;
            let ids: Vec<i64> = mundo
                .players
                .iter()
                .filter(|(_, p)| p.position.distance(&d.position) <= RAIO_DE_VISAO)
                .map(|(id, _)| *id)
                .collect();
            for id in &ids {
                if let Some(p) = mundo.players.get_mut(id) {
                    p.visiveis.insert(d.id);
                }
            }
            ids.into_iter().map(|i| i as i32).collect()
        };
        let pacote = S2CGamedataSend::matter_enter_world(d.id as i32, d.item_id as i32, d.position).data;
        for id in perto {
            self.enviar_ao_jogador(id, pacote.clone()).await;
        }
    }

    // ------------------------------------------------------------------ coleta

    /// `C2S::PICKUP` (6) — `{ int mid; int type; }` (`playercmd.cpp:1347-1444`).
    pub(super) async fn pegar(&self, roleid: i32, payload: &[u8]) {
        let mut r = Reader::new(payload);
        let (Ok(mid), Ok(tipo)) = (r.i32(), r.i32()) else {
            warn!("mundo: pickup de {roleid} com payload curto");
            return;
        };
        self.pegar_um(roleid, mid as i64, tipo as u32).await;
    }

    /// `C2S::PICKUP_ALL` (184) — `{ int count; { int mid; int type; } matter[]; }`
    /// (`playercmd.cpp:4408-4460`), no máximo 100.
    pub(super) async fn pegar_todos(&self, roleid: i32, payload: &[u8]) {
        let mut r = Reader::new(payload);
        let Ok(n) = r.i32() else { return };
        if n <= 0 || n > 100 {
            return;
        }
        for _ in 0..n {
            let (Ok(mid), Ok(tipo)) = (r.i32(), r.i32()) else { break };
            self.pegar_um(roleid, mid as i64, tipo as u32).await;
        }
    }

    async fn pegar_um(&self, roleid: i32, mid: i64, tipo: u32) {
        // Confere existência, tipo, distância e posse antes de mexer em qualquer coisa.
        let drop = {
            let mundo = self.world.read().await;
            let (Some(d), Some(p)) = (mundo.drops.get(&mid), mundo.players.get(&(roleid as i64))) else {
                return;
            };
            if d.item_id & 0xFFFF != tipo & 0xFFFF {
                return;
            }
            if d.position.distance(&p.position) >= economia::DISTANCIA_PARA_PEGAR {
                drop(mundo);
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::NAO_PODE_PEGAR).data).await;
                return;
            }
            if d.owner_role_id.is_some_and(|dono| dono != roleid) {
                drop(mundo);
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::NAO_PODE_PEGAR).data).await;
                return;
            }
            d.clone()
        };

        let pegou = self
            .com_contexto(roleid, |ctx| {
                if drop.item_id == TID_DO_DINHEIRO {
                    if ctx.p.money >= TETO_DE_DINHEIRO {
                        ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::BOLSA_CHEIA).data);
                        return false;
                    }
                    let ganho = ctx.ganhar_dinheiro(drop.count as i64);
                    ctx.para_mim.push(S2CGamedataSend::pickup_money(ganho as i32).data);
                    return true;
                }
                let dados = ctx.dados;
                let eh_missao = dados.e_item_de_missao(drop.item_id);
                let where_pct = if eh_missao { 2 } else { 0 };
                let bolsa = if eh_missao { &mut ctx.bolsa_de_missao } else { &mut ctx.bolsa };
                let guardado = if drop.octetos.is_empty() {
                    bolsa.empilhar_gerado(drop.item_id, drop.count, dados)
                } else {
                    bolsa.guardar_equipamento(drop.item_id, &drop.octetos, dados)
                };
                match guardado {
                    Some(e) => {
                        ctx.para_mim.push(S2CGamedataSend::pickup_item(drop.item_id as i32, 0, e.entrou, e.no_slot, where_pct, e.slot as u8).data);
                        true
                    }
                    None => {
                        ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::BOLSA_CHEIA).data);
                        false
                    }
                }
            })
            .await
            .unwrap_or(false);
        if !pegou {
            return;
        }
        if self.world.write().await.remover_drop(mid).is_none() {
            return;
        }
        let pacote = S2CGamedataSend::matter_pickup(mid as i32, roleid).data;
        self.enviar_ao_jogador(roleid, pacote.clone()).await;
        self.transmitir_a_outros(roleid, pacote).await;
    }

    // ------------------------------------------------------------------ loja e habilidades

    /// `GP_NPCSEV_SELL` — o NPC vende, o jogador **compra** (`gplayer_imp::PurchaseItem`,
    /// `player.cpp:8900-8932`): empilha na bolsa e responde `PURCHASE_ITEM` (72).
    pub(super) async fn comprar(&self, roleid: i32, conteudo: &[u8]) {
        let pedidos = npc::itens_comprados(conteudo);
        if pedidos.is_empty() {
            return;
        }
        self.com_contexto(roleid, |ctx| {
            let dados = ctx.dados;
            let mut total: i64 = 0;
            for i in &pedidos {
                let Some(unitario) = dados.preco_de_compra(i.tid as u32) else {
                    debug!("mundo: o item {} não tem preço no elements.data", i.tid);
                    return;
                };
                total += unitario as i64 * i.count.max(1) as i64;
            }
            if ctx.p.money < total {
                ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::SEM_DINHEIRO).data);
                return;
            }
            let livres = ctx.bolsa.livres() as usize;
            if livres < pedidos.len() {
                ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::BOLSA_CHEIA).data);
                return;
            }
            let mut lista = Vec::new();
            for i in &pedidos {
                if let Some(e) = ctx.bolsa.empilhar(i.tid as u32, i.count.max(1), dados) {
                    lista.push((i.tid, 0, e.entrou, e.slot as u16));
                }
            }
            ctx.gastar_dinheiro(total);
            info!("mundo: {roleid} comprou {} item(ns) por {total}", lista.len());
            ctx.para_mim.push(S2CGamedataSend::purchase_item(total as u32, &lista).data);
        })
        .await;
    }

    /// `GP_NPCSEV_BUY` — o NPC compra, o jogador **vende** (`gplayer_imp::ItemToMoney`,
    /// `player.cpp:13930-13997`): `price × count`, proporcional à durabilidade, e
    /// `ITEM_TO_MONEY` (73).
    pub(super) async fn vender(&self, roleid: i32, conteudo: &[u8]) {
        let pedidos = npc::itens_vendidos(conteudo);
        if pedidos.is_empty() {
            return;
        }
        self.com_contexto(roleid, |ctx| {
            let dados = ctx.dados;
            for i in &pedidos {
                let slot = i.index as usize;
                let Some(Some(item)) = ctx.bolsa.slots.get(slot).cloned() else { continue };
                if item.item_id != i.tid as u32 || i.count == 0 || i.count > item.count {
                    continue;
                }
                let Some(preco) = dados.preco_de_venda(item.item_id) else { continue };
                let mut valor = preco as f32 * i.count as f32;
                if item.max_durability > 0 && item.durability < item.max_durability {
                    valor = valor * item.durability as f32 / item.max_durability as f32;
                }
                let valor = (valor.max(0.0) + 0.5) as i64;
                let _ = ctx.bolsa.tirar_do_slot(slot, i.count);
                let ganho = ctx.ganhar_dinheiro(valor);
                ctx.para_mim.push(S2CGamedataSend::item_to_money(slot as u16, i.tid, i.count, ganho as u32).data);
            }
        })
        .await;
    }

    /// `GP_NPCSEV_LEARN` (9) — `skill_executor::OnServe` (`serviceprovider.cpp:1288-1312`) e
    /// `SkillStub::LearnCondition`/`Learn` (`cskill/skill/skill.cpp:14-93`): a habilidade tem
    /// de estar na lista do treinador; fora de combate; próximo nível ≤ máximo; classe;
    /// pré-requisitos; nível histórico; SP; `rank` × cultivo; dinheiro. Cobra dinheiro
    /// (`SPEND_MONEY`) e SP (`COST_SKILL_POINT`) e responde `LEARN_SKILL`.
    /// `GP_NPCSEV_TRANSMIT` (5) — a transportadora leva o jogador a um destino.
    ///
    /// O cliente manda só o **índice** do destino na lista daquela transportadora
    /// (`transmit_executor::SendRequest` → `transmit_provider::request { index, money }`,
    /// `gs/serviceprovider.cpp:803-825`). O servidor confere:
    ///
    /// 1. índice dentro da lista, senão `ERR_SERVICE_ERR_REQUEST` (`:771-779`);
    /// 2. dinheiro ≥ `fee`, senão `ERR_OUT_OF_FUND` (`:781-787`);
    /// 3. nível ≥ `require_level` (`transmit_entry`), e então cobra e faz `LongJump`
    ///    (`:827-852`).
    ///
    /// A coordenada do destino vem do `world_targets.sev` (o `idTarget` do
    /// `NPC_TRANSMIT_SERVICE` é o id de lá) — ver [`pw_data_loader::world_targets`].
    pub(super) async fn teleportar_pela_transportadora(&self, roleid: i32, conteudo: &[u8], envio: &crate::bus_server::EnvioAoCliente) {
        let mut r = Reader::new(conteudo);
        let Ok(indice) = r.i32() else {
            warn!("mundo: pedido de teleporte de {roleid} sem índice");
            return;
        };

        let (dados, npc_tid, nivel, dinheiro) = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            let tid = p
                .npc_em_conversa
                .and_then(|id| mundo.npcs.get(&id))
                .map(|n| n.template_id);
            (Arc::clone(&mundo.data_manager), tid, p.level, p.money)
        };
        let Some(npc_tid) = npc_tid else {
            debug!("mundo: {roleid} pediu teleporte sem NPC em conversa");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::SERVICO_INDISPONIVEL).data).await;
            return;
        };
        let destinos = dados.servicos_de_npc.get(&npc_tid).map(|s| s.destinos.clone()).unwrap_or_default();
        let Some(destino) = usize::try_from(indice).ok().and_then(|i| destinos.get(i)).copied() else {
            debug!("mundo: {roleid} pediu o destino {indice} de {npc_tid}, que tem {}", destinos.len());
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::SERVICO_INDISPONIVEL).data).await;
            return;
        };
        let Some(ponto) = dados.pontos_do_mundo.get(destino.id_ponto).copied() else {
            warn!("mundo: o destino {} não está no world_targets.sev", destino.id_ponto);
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::SERVICO_INDISPONIVEL).data).await;
            return;
        };
        if nivel < destino.nivel {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::NIVEL_NAO_BATE).data).await;
            return;
        }
        if dinheiro < destino.preco as i64 {
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::SEM_DINHEIRO).data).await;
            return;
        }

        if destino.preco > 0 {
            self.com_contexto(roleid, |ctx| {
                ctx.gastar_dinheiro(destino.preco as i64);
                ctx.para_mim.push(S2CGamedataSend::spend_money(destino.preco as u32).data);
            })
            .await;
        }
        info!(
            "mundo: {roleid} viajou pela transportadora {npc_tid} até o ponto {} (mapa {}, {:.0} {:.0}), por {}",
            destino.id_ponto, ponto.mundo, ponto.pos[0], ponto.pos[2], destino.preco
        );
        let _ = envio;
        self.transportar(roleid, ponto.mundo, pw_core::Vector3::new(ponto.pos[0], ponto.pos[1], ponto.pos[2]))
            .await;
    }

    pub(super) async fn aprender(&self, roleid: i32, conteudo: &[u8]) {
        let Ok(skill_id) = Reader::new(conteudo).i32() else { return };
        if skill_id <= 0 {
            return;
        }
        let id = skill_id as u32;
        if let Some((npc_tid, s)) = self.npc_em_conversa(roleid).await {
            let eh_da_classe = {
                let mundo = self.world.read().await;
                mundo.players.get(&(roleid as i64)).and_then(|p| {
                    mundo.data_manager.habilidades.get(id).map(|h| h.cls == Some(p.cls as i32) || h.cls == Some(255))
                }).unwrap_or(false)
            };
            if !eh_da_classe && !s.habilidades.is_empty() && s.habilidades.binary_search(&id).is_err() {
                debug!("mundo: o NPC {npc_tid} não ensina a habilidade {id}");
                self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro_s2c::HABILIDADE_INDISPONIVEL).data).await;
                return;
            }
        }
        let novo = self
            .com_contexto(roleid, |ctx| {
                let dados = ctx.dados;
                let recusa = |ctx: &mut Contexto, c: i32| {
                    ctx.para_mim.push(S2CGamedataSend::error_message(c).data);
                    None
                };
                if ctx.p.combate_s > 0 {
                    return recusa(ctx, erro_s2c::OPERACAO_EM_COMBATE);
                }
                let Some(h) = dados.habilidades.get(id) else {
                    warn!("mundo: a habilidade {id} não está na tabela do servidor");
                    return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                };
                let atual = ctx.p.habilidades.get(&id).copied().unwrap_or(0) as i32;
                let proximo = atual + 1;
                if proximo > h.max_level {
                    return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                }
                if let Some(cls) = h.cls {
                    if cls != 255 && cls != ctx.p.cls as i32 {
                        return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                    }
                }
                for (pre, nivel) in &h.pre_skills {
                    if *pre > 0 && (ctx.p.habilidades.get(pre).copied().unwrap_or(0) as i32) < *nivel {
                        return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                    }
                }
                let (Some(nivel), Some(sp), Some(dinheiro)) = (h.nivel_exigido(proximo), h.sp_exigido(proximo), h.dinheiro_exigido(proximo)) else {
                    warn!("mundo: a habilidade {id} nível {proximo} tem requisito desconhecido na tabela — recusada");
                    return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                };
                if ctx.p.level < nivel || ctx.p.sp < sp as i64 {
                    return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                }
                let (srank, prank) = (h.rank.unwrap_or(0), ctx.p.cultivation);
                if srank > prank || (srank / 10 != prank / 10 && srank / 10 != 0) {
                    return recusa(ctx, erro_s2c::NAO_PODE_APRENDER);
                }
                if ctx.p.money < dinheiro as i64 {
                    return recusa(ctx, erro_s2c::SEM_DINHEIRO);
                }
                ctx.gastar_dinheiro(dinheiro as i64);
                if dinheiro > 0 {
                    ctx.para_mim.push(S2CGamedataSend::spend_money(dinheiro as u32).data);
                }
                if sp > 0 {
                    ctx.p.sp -= sp as i64;
                    ctx.para_mim.push(S2CGamedataSend::cost_skill_point(sp).data);
                }
                ctx.p.habilidades.insert(id, proximo as u8);
                ctx.mudou = true;
                ctx.para_mim.push(S2CGamedataSend::learn_skill(skill_id, proximo).data);
                Some(proximo)
            })
            .await
            .flatten();
        if let Some(n) = novo {
            if let Err(e) = self.repo().await.skill_repo().learn_or_upgrade(roleid, id, n as u8).await {
                warn!("mundo: não consegui gravar a habilidade {id} de {roleid}: {e}");
            }
            info!("mundo: {roleid} aprendeu a habilidade {id} no nível {n}");
        }
    }

    /// `GP_NPCSEV_HATCHPET` (28) — chocar/incubar ovo de mascote na Gerente de Mascotes.
    /// Payload: `egg_index: i32, egg_id: i32` (`hatch_pet_service_executor`, `serviceprovider.cpp:3160`).
    /// Deduz as moedas (`money_hatched`), remove o ovo da bolsa (`DROP_TYPE_USE` = 10) e
    /// responde `GAIN_PET` (231) com a struct `info_pet` (192 bytes).
    pub(super) async fn incubar_mascote(&self, roleid: i32, conteudo: &[u8]) {
        let mut r = Reader::new(conteudo);
        let Ok(egg_index) = r.i32() else { return };
        let Ok(egg_id) = r.i32() else { return };
        if egg_index < 0 || egg_id <= 0 {
            return;
        }

        let itens = self.itens().await;
        let pets_corral = itens
            .list_by_container(roleid, pw_core::ContainerType::PetCorral)
            .await
            .unwrap_or_default();
        let slot_pet = pets_corral.len() as i32;

        let pet_gerado = self
            .com_contexto(roleid, |ctx| {
                let dados = ctx.dados;
                let Some(ovo_info) = dados.dados_do_ovo(egg_id as u32) else {
                    ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::SERVICO_INDISPONIVEL).data);
                    return None;
                };

                // Localiza o slot do ovo na bolsa
                let slot_idx = if ctx.bolsa.item_no_slot(egg_index as usize).map(|i| i.item_id) == Some(egg_id as u32) {
                    egg_index as usize
                } else if let Some(s) = ctx.bolsa.slots.iter().position(|x| x.as_ref().map(|i| i.item_id) == Some(egg_id as u32)) {
                    s
                } else {
                    ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::ITEM_NAO_NO_INVENTARIO).data);
                    return None;
                };

                let item = ctx.bolsa.item_no_slot(slot_idx)?;
                let custo = ovo_info.money_hatched as i64;
                if ctx.p.money < custo {
                    ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::SEM_DINHEIRO).data);
                    return None;
                }

                // Obtém ou gera a essência oficial (pe_essence) do ovo
                let ess = if item.octets.len() >= pw_core::TAMANHO_PE_ESSENCE_BASE {
                    pw_core::PeEssence::de_bytes(&item.octets).unwrap_or_else(|| {
                        let mut e = pw_core::PeEssence::default();
                        e.require_level = ovo_info.req_level;
                        e.require_class = ovo_info.req_class;
                        e.honor_point = ovo_info.honor_point;
                        e.pet_tid = ovo_info.id_pet as i32;
                        e.pet_egg_tid = ovo_info.id as i32;
                        e.pet_class = ovo_info.pet_class;
                        e.level = ovo_info.level;
                        e.exp = ovo_info.exp;
                        e.skill_point = ovo_info.skill_point;
                        e.skills = ovo_info.skills.clone();
                        e
                    })
                } else {
                    let mut e = pw_core::PeEssence::default();
                    e.require_level = ovo_info.req_level;
                    e.require_class = ovo_info.req_class;
                    e.honor_point = ovo_info.honor_point;
                    e.pet_tid = ovo_info.id_pet as i32;
                    e.pet_egg_tid = ovo_info.id as i32;
                    e.pet_class = ovo_info.pet_class;
                    e.level = ovo_info.level;
                    e.exp = ovo_info.exp;
                    e.skill_point = ovo_info.skill_point;
                    e.skills = ovo_info.skills.clone();
                    e
                };

                // Confere restrição de classe
                if ((1 << (ctx.p.cls as i32 & 0x1F)) & ess.require_class) == 0 {
                    ctx.para_mim.push(S2CGamedataSend::error_message(erro_s2c::PET_NAO_PODE_CHOCAR).data);
                    return None;
                }

                // Remove 1 unidade do ovo da bolsa
                let tirou = ctx.bolsa.tirar_do_slot(slot_idx, 1);
                if tirou == 0 {
                    return None;
                }
                ctx.para_mim.push(S2CGamedataSend::player_drop_item(0, slot_idx as u8, 1, egg_id, 10).data);

                // Deduz as moedas do serviço
                if custo > 0 {
                    ctx.gastar_dinheiro(custo);
                    ctx.para_mim.push(S2CGamedataSend::spend_money(custo as u32).data);
                }

                // Cria o InfoPet oficial (192 bytes) e envia GAIN_PET (opcode 231)
                let info_pet = pw_core::InfoPet::de_essencia(&ess);
                let pet_bytes = info_pet.para_bytes();
                ctx.para_mim.push(S2CGamedataSend::gain_pet(slot_pet, &pet_bytes).data);

                Some((ovo_info.id_pet, pet_bytes))
            })
            .await
            .flatten();

        if let Some((pet_tid, pet_bytes)) = pet_gerado {
            let mut record = pw_core::ItemRecord::new(roleid, pw_core::ContainerType::PetCorral, slot_pet as u16, pet_tid, 1);
            record.octets = pet_bytes;
            if let Err(e) = itens.upsert_item(&record).await {
                warn!("mundo: erro ao salvar mascote {pet_tid} de {roleid} no corral: {e}");
            }
            info!("mundo: {roleid} incubou o mascote/montaria {pet_tid} no slot {slot_pet} com sucesso a partir do ovo {egg_id}");
        }
    }

    /// Confere e arma a recarga de uma habilidade. `false` se ainda está recarregando.
    ///
    /// `SkillWrapper::StartSkill` testa a recarga antes (`skillwrapper.cpp:261`), e
    /// `SetCoolDown(id + 1024, (int)(0.001 × coolingtime) × 1000)` a arma
    /// (`playerwrapper.cpp:170`, `skill.h:577`), com `SET_COOLDOWN` (198) ao cliente.
    pub(super) async fn armar_recarga(&self, roleid: i32, skill_id: i32) -> Option<Option<i32>> {
        let mut mundo = self.world.write().await;
        let dados = Arc::clone(&mundo.data_manager);
        let p = mundo.players.get_mut(&(roleid as i64))?;
        let indice = skill_id + INICIO_DAS_RECARGAS_DE_HABILIDADE;
        let agora = std::time::Instant::now();
        if p.recargas.get(&indice).is_some_and(|ate| *ate > agora) {
            return Some(None);
        }
        let nivel = p.habilidades.get(&(skill_id.max(0) as u32)).copied().unwrap_or(1).max(1) as i32;
        let Some(ms) = dados.habilidades.get(skill_id.max(0) as u32).and_then(|h| h.recarga_armada_ms(nivel)) else {
            return Some(Some(0));
        };
        if ms > 0 {
            p.recargas.insert(indice, agora + std::time::Duration::from_millis(ms as u64));
        }
        Some(Some(ms))
    }

    pub(super) fn erro_de_recarga() -> Vec<u8> {
        S2CGamedataSend::error_message(erro_s2c::HABILIDADE_EM_RECARGA).data
    }
}

