use pw_core::CharacterDetails;
use pw_data_loader::{TabelaDeBase, TabelaDeClasses, TemplateDeMonstro};
use pw_core::{CharacterClass, Gender, Race, RoleId, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveBuff {
    pub buff_id: u32,
    pub level: u8,
    pub duration_ms: u32,
    pub elapsed_ms: u32,
    pub tick_interval_ms: u32,
    pub tick_elapsed_ms: u32,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerEntity {
    pub role_id: RoleId,
    pub name: String,
    pub race: Race,
    pub cls: CharacterClass,
    pub gender: Gender,
    pub level: i32,
    pub cultivation: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub sp: i64,
    pub money: i64,
    
    // Atributos de Combate
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub energy: i32,
    pub def_phys: i32,
    pub def_metal: i32,
    pub def_wood: i32,
    pub def_water: i32,
    pub def_fire: i32,
    pub def_earth: i32,
    pub attack_min: i32,
    pub attack_max: i32,
    pub magic_attack_min: i32,
    pub magic_attack_max: i32,
    /// `_cur_prop.armor` — a evasão. No original vem de
    /// `GetBasicArmor(classe, agilidade) = agi_armor[classe] * agilidade`, mais
    /// equipamento; ver [`crate::entity::PlayerEntity::armadura_base`].
    pub armor: i32,
    /// `_cur_prop.attack` — a precisão. Mesma origem, com `agi_attack`.
    pub attack_rate: i32,
    /// `attack_degree` / `_defend_degree`. No original vêm de equipamento e habilidade
    /// passiva; sem esses sistemas ficam em zero, que é o valor neutro do cálculo.
    pub attack_degree: i32,
    pub defend_degree: i32,
    /// `crit_damage_bonus`, em pontos percentuais somados ao dobro base do crítico.
    pub crit_damage_bonus: i32,
    pub attack_speed: f32,
    /// `run_speed`, em m/s. Ver [`PlayerEntity::do_personagem`] para a fonte — **não** é
    /// o `ptemplate.conf`.
    pub move_speed: f32,
    pub walk_speed: f32,
    pub swim_speed: f32,
    pub fly_speed: f32,
    /// `attack_range`, em metros.
    pub attack_range: f32,
    /// `hp_gen` / `mp_gen` — quanto regenera por intervalo fora de combate.
    pub hp_gen: i32,
    pub mp_gen: i32,
    pub crit_rate: f32,
    
    pub position: Vector3,
    pub target_id: Option<i64>,
    /// Os filtros vivos (efeitos de habilidade) — ver [`crate::efeitos`].
    pub efeitos: crate::efeitos::Efeitos,
    /// As entidades que este jogador já recebeu — NPCs e monstros, por id.
    ///
    /// É a memória do que o **cliente** tem. Sem ela não dá para saber o que mandar
    /// quando ele anda: `NPC_ENTER_SLICE` para o que entrou no alcance,
    /// `OBJECT_LEAVE_SLICE` para o que saiu. Ver `BusServer::atualizar_visiveis`.
    pub visiveis: std::collections::HashSet<i64>,
    /// Onde o jogador estava quando [`Self::visiveis`] foi calculado pela última vez.
    ///
    /// A conta só é refeita depois que ele anda uma distância mínima: o cliente manda
    /// movimento 20 vezes por segundo, e varrer a grade a cada pacote seria varrer 20
    /// vezes por segundo por jogador para achar quase sempre o mesmo conjunto.
    pub centro_do_stream: Vector3,
    /// O jogador está voando.
    ///
    /// Não há coluna no banco para isso, e nem deveria: quem relogar entra no chão, que é
    /// o que o cliente também assume.
    pub voando: bool,
    /// A montaria em uso. O original é o `mount_filter` (`gs/mount_filter.cpp:24-45`), que
    /// liga o `STATE_MOUNT`, manda `PLAYER_MOUNTING` e **sobrepõe** a velocidade de
    /// corrida (`EnhanceOverrideSpeed`, `gs/player.cpp:14279-14299`).
    pub montaria: Option<MontariaAtiva>,
    /// O marcador da operação de mascote aberta — o `session_pet_operation` do original
    /// (`gs/actsession.h:1203-1246`). Cada `SUMMON_PET`/`RECALL_PET` novo incrementa, e a
    /// tarefa que conclui a canalização só age se o marcador ainda for o dela; é assim que
    /// um segundo pedido cancela o primeiro em vez de os dois se aplicarem.
    pub operacao_de_pet: u64,
    /// O jogador está mostrando a roupa (moda) no lugar da armadura.
    ///
    /// É estado de aparência, e o cliente alterna com o `SWITCH_FASHION_MODE` (C2S 85).
    /// Vive só no mundo: não há coluna para ele no banco, então volta ao padrão a cada
    /// login. Trocar isso é mudança de esquema, não de código.
    pub modo_roupa: bool,
    /// `sec_level` — o nível de GM da conta dona do personagem.
    ///
    /// Viaja no `level2` da `info_player_1` e acende o `STATE_GAMEMASTER` (`0x4000`) no
    /// `state`, que é o que põe a coroa sobre o avatar. Vive aqui porque quem manda o
    /// jogador aparecer para os outros passou a ser o mundo
    /// (`BusServer::atualizar_visiveis`), e o `BusMessage::EnterWorld` não carrega o
    /// `sec_level` da sessão — ver `CharacterRepository::nivel_de_gm`.
    ///
    /// Zero por omissão: negar privilégio é a resposta segura.
    pub sec_level: u8,
    /// As habilidades aprendidas, por id, com o **nível de cada uma**.
    ///
    /// Vive aqui porque o `CAST_SKILL` do cliente **não manda o nível** — quem tem de
    /// saber é o servidor, e ele não pode ir ao banco a cada conjuração. Sai do
    /// `character_skills`, carregado com o resto do personagem no login.
    ///
    /// Até 2026-09-09 este dado não existia no mundo e toda habilidade era conjurada no
    /// nível 1 (`NIVEL_DA_HABILIDADE` em `bus_server.rs`): subir uma habilidade não mudava
    /// nada em jogo — nem dano, nem cura, nem custo de mana.
    pub habilidades: std::collections::HashMap<u32, u8>,
    /// `custom_crc` — o carimbo da aparência gravada deste personagem.
    ///
    /// Viaja no `crc_c` de todo pacote que apresenta este jogador a outro, e tem de ser o
    /// **mesmo** valor que o `custom_stamp` do `PlayerBaseInfo_Re` que o `pw-link`
    /// responde. Ver [`pw_core::stamp_de_aparencia`].
    pub crc_aparencia: u16,
    /// `_en_percent.base_damage` e `.base_magic`: a porcentagem que força/agilidade e energia
    /// somam ao dano (`UpdateAttack`/`UpdateMagic`). Entra de novo no dano de habilidade
    /// (`GeneratePhysicDamage`, `actobject.h:1422-1444`).
    pub bonus_de_dano_pct: i32,
    pub bonus_magico_pct: i32,
    /// `status_point` — pontos de atributo por distribuir (`potential_points` no banco).
    /// Cinco por nível (`LevelUp`, `gs/player.cpp:2647`).
    pub pontos_de_atributo: i32,
    /// `_basic.reputation` (sem sistema de reputação ainda, vem zerada do banco).
    pub reputacao: i32,
    /// `_combat_timer`, em segundos: atacar põe 15 (`MAX_COMBAT_TIME`, `DoAttack`,
    /// `player.cpp:3062`), apanhar garante 5 (`NORMAL_COMBAT_TIME`, `OnAttacked`,
    /// `player.cpp:9514`); o batimento de 1 s desconta (`player.cpp:9072`).
    pub combate_s: i32,
    /// `_hp_gen_counter` / `_mp_gen_counter` — a fração acumulada da regeneração
    /// (`func::Update`, `actobject.h:2143`).
    pub contador_hp: i32,
    pub contador_mp: i32,
    /// Até quando cada índice de recarga está bloqueado (`SetCoolDown`); o índice da
    /// habilidade é `id + 1024`.
    pub recargas: std::collections::HashMap<i32, std::time::Instant>,
    /// O NPC com quem o jogador falou por último (`SEVNPC_HELLO`). Os pedidos de serviço
    /// (`SEVNPC_SERVE`) não trazem o NPC: vão ao que está em conversa.
    pub npc_em_conversa: Option<i64>,
    /// Os pontos de teleporte já descobertos (`_waypoint_list`, `gs/player_imp.h:2520-2550`).
    ///
    /// O cliente manda os pontos da região onde está (`ACTIVATE_REGION_WAYPOINTS`, C2S 178);
    /// os que ainda não estão aqui entram, são gravados e voltam em `ACTIVATE_WAYPOINT`
    /// (179) — o comando que faz o cliente anunciar "novo ponto de teleporte" com o nome do
    /// lugar (`CECHostPlayer::OnMsgHstWayPoint`, `EC_HostMsg.cpp:4681-4720`).
    pub waypoints: Vec<u16>,
    /// A barra de **chi** ("fúria"): `_basic.ap` e `_base_prop.max_ap`
    /// (`gs/actobject.h:1634-1657`). O teto nasce zero e é **concedido por missão**
    /// (`m_ulFuryULimit` → `SetFuryUpperLimit` → `SetMaxAP`, `gs/task/taskman.cpp:498-501`);
    /// enquanto for zero, o jogador não tem barra. O ganho por golpe normal é o
    /// `angro_increase` da classe (`player.cpp:3091-3093`).
    pub ap: i32,
    pub max_ap: i32,
    /// `_ap_per_hit` — quanto cada golpe normal acrescenta ao chi.
    pub ap_por_golpe: i32,
    /// O jogador está meditando (`SIT_DOWN`). Enquanto estiver, o batimento de 1 s dá
    /// **15 de chi** (`sit_down_filter::Heartbeat`, `gs/sitdown_filter.cpp:19-34`).
    pub sentado: bool,
    /// As listas de missão — ver [`crate::missoes`].
    pub missoes: crate::missoes::ListasDeMissao,
    /// A mina que está colhendo (`session_gather`), se alguma.
    pub coleta: Option<i64>,
    /// O que o equipamento vestido acrescenta (`_cur_item` e `_en_point`).
    pub equipamento: Equipamento,
    /// A durabilidade de cada peça vestida — `(atual, máxima)` por slot, como o `_equipment`
    /// do original, que é uma `item_list` em memória (`player.cpp:94`). `None` é slot vazio
    /// ou item sem durabilidade. Existe para que levar um golpe não precise perguntar ao
    /// banco antes de responder ao cliente: o índice da peça desgastada vai **dentro** do
    /// `be_damaged` (`player.cpp:9552-9570`), e ir ao banco ali punha a latência do
    /// PostgreSQL no meio da animação (B72).
    pub pecas: [Option<(i32, i32)>; PECAS_VESTIDAS],
    /// O amuleto de vida vestido (`EQUIP_INDEX_HP_ADDON` 20) e o hierograma de mana
    /// (`EQUIP_INDEX_MP_ADDON` 21), com o que ainda resta neles. `OnActivate` guarda os dois
    /// números no jogador (`SetHPAutoGen`/`SetMPAutoGen`, `gs/item/item_amulet.cpp:22-46`) e
    /// é o batimento que os consome.
    pub auto_hp: Option<AmuletoAtivo>,
    pub auto_mp: Option<AmuletoAtivo>,
    /// O Daimon vestido no slot 23, com o estado que vive no bloco do item.
    pub daimon: Option<DaimonVestido>,
    /// Quando cada recarga de amuleto vence, em segundos de batimento
    /// (`COOLDOWN_INDEX_AUTO_HP` 24 e `AUTO_MP` 25, `gs/cooldowncfg.h:62-90`).
    pub recarga_do_auto_hp_s: i32,
    pub recarga_do_auto_mp_s: i32,
    /// A sessão de golpe normal em andamento (`session_normal_attack`).
    pub ataque: Option<SessaoDeAtaque>,
    /// A conjuração em andamento, com o marcador que a identifica — a tarefa que a conclui
    /// confere o marcador, e soltar a carga (`CONTINUE_ACTION`) a conclui antes.
    pub conjuracao: Option<Conjuracao>,
    /// O dano **antes** das porcentagens: `_base_prop.damage_* + _en_point.damage_* +
    /// _cur_item.damage_*` (`GeneratePhysicDamage`/`GenerateMaigicDamage2`,
    /// `actobject.h:1422-1469`). É a base do dano de habilidade.
    pub dano_bruto: (i32, i32),
    pub dano_magico_bruto: (i32, i32),
}

/// O mascote de montaria em uso — o que o `mount_filter` guarda no original, mais o slot
/// da jaula, que é o que o cliente precisa de volta no `SUMMON_PET`/`RECALL_PET`
/// (`gs/petman.cpp:1328-1339`, `:1376`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MontariaAtiva {
    /// O índice na sala de mascotes (`_cur_active_pet`).
    pub indice: u16,
    /// O modelo que aparece: `pet_vis_tid` quando existe, senão `pet_tid`
    /// (`gs/player.cpp:14483-14486`).
    pub tid: u32,
    /// O `pet_tid` do bloco. É **este** que o cliente confere contra o mascote da jaula
    /// (`ASSERT(pPet->GetTemplateID() == pCmd->pet_tid)`, `EC_HostMsg.cpp:5278`).
    pub pet_tid: u32,
    pub cor: u16,
    /// A velocidade que a montaria impõe, já com o nível dentro.
    pub velocidade: f32,
}

/// Uma conjuração aberta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conjuracao {
    pub skill_id: i32,
    pub alvo: i64,
    pub inicio: std::time::Instant,
    /// O tempo do primeiro estado — a carga cheia, nas de carga.
    pub duracao_ms: u32,
    pub marcador: u64,
}

/// `PLAYER_BODYSIZE` (`gs/config.h:104`).
pub const CORPO_DO_JOGADOR: f32 = 0.3;

/// Um amuleto de vida ou hierograma de mana vestido, como `SetHPAutoGen`/`SetMPAutoGen` o
/// deixam no jogador (`gs/item/item_amulet.cpp:22-46`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmuletoAtivo {
    pub slot: u16,
    pub item_id: u32,
    /// `_ess.point`: quanto ainda há para restaurar. Cai a cada disparo e, em zero, o item
    /// some (`base_amulet::OnAutoTrigger`, `item_amulet.cpp:9-20`).
    pub ponto: i32,
    /// `_ess.trigger_percent`: dispara quando `gatilho × máximo > atual`.
    pub gatilho: f32,
    /// `get_cool_time(tid)` do `elements.data`, em milissegundos.
    pub recarga_ms: i32,
}

impl AmuletoAtivo {
    /// Lê `amulet_essence` dos octetos do item: `int point; float trigger_percent`
    /// (`gs/item/item_amulet.h:16-19`). É o que o `Load` do original recupera.
    pub fn do_bloco(octetos: &[u8]) -> Option<(i32, f32)> {
        if octetos.len() < 8 {
            return None;
        }
        let ponto = i32::from_le_bytes(octetos[0..4].try_into().ok()?);
        let gatilho = f32::from_le_bytes(octetos[4..8].try_into().ok()?);
        Some((ponto, gatilho))
    }

    /// O mesmo bloco de volta, para gravar o que sobrou.
    pub fn bloco(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(8);
        b.extend_from_slice(&self.ponto.to_le_bytes());
        b.extend_from_slice(&self.gatilho.to_le_bytes());
        b
    }
}

/// Slots de equipamento que guardam durabilidade: `EQUIP_INDEX_WEAPON` (0) até
/// `EQUIP_INDEX_PROJECTILE` (11) (`EC_IvtrTypes.h:56-67`).
pub const PECAS_VESTIDAS: usize = 12;

/// `session_normal_attack` (`actsession.cpp:350-418`): o alvo e quanto falta para o próximo
/// golpe, que sai a cada `attack_speed` *ticks*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessaoDeAtaque {
    pub alvo: i64,
    pub falta_ms: u32,
    /// A sessão guarda a munição como o `item_list` em memória do original. Consultar e
    /// gravar PostgreSQL antes de cada `ATTACK_ONCE` atrasava a cadência pela latência do
    /// banco; a persistência pode acontecer depois que o comando já saiu.
    pub municao_restante: u16,
    pub arma_de_longe: bool,
    /// O `NORMAL_ATTACK` que chegou com esta sessão aberta, na fila (`AddSession`,
    /// `actobject.cpp:1180-1213`). Só começa no próximo golpe (`GM_MSG_OBJ_SESSION_REPEAT`
    /// com `HasNextSession`, `actobject.cpp:180-189`) — o ritmo não muda com cliques.
    pub proximo: Option<i64>,
    /// Um `CANCEL_ACTION` na fila (`session_cancel_action`, `playercmd.cpp:2136-2153`): no
    /// próximo golpe a sessão termina.
    pub cancelar: bool,
    /// Um movimento na fila (`session_move`, `playercmd.cpp:9297-9303`): no próximo golpe a
    /// sessão termina. `true` com [`Self::proximo`] marcado quer dizer que andou **depois**
    /// de clicar — o novo golpe abre e termina no seguinte.
    pub andar: bool,
}

/// A arma empunhada, como `WeaponItemEnhance` a deixa em `_cur_item` (`actobject.h:1111`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmaEmUso {
    /// `weapon_type` 1 (longo alcance) ou 2 (corpo a corpo de agilidade): o dano da arma
    /// cresce com a agilidade; 0, com a força (`UpdateAttack`, `playertemplate.h:925-936`).
    pub dano_pela_agilidade: bool,
    pub de_longe: bool,
    pub dano: (i32, i32),
    pub dano_magico: (i32, i32),
    pub alcance: f32,
    pub velocidade_em_ticks: i32,
}

/// O que as propriedades adicionais do equipamento vestido somam (`Activate`/`UpdateItem`
/// dos tratadores, `gs/item/item_addon.cpp`). Refino e pedras são addons da mesma lista
/// (`refine_*`, `SetAddOnEmbedded`, `equip_item.cpp:319-330`, `876-905`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BonusDeAddons {
    /// `_en_point.str/agi/vit/eng`.
    pub forca: i32,
    pub agilidade: i32,
    pub vitalidade: i32,
    pub energia: i32,
    /// `_en_point.max_hp/max_mp`.
    pub vida: i32,
    pub mana: i32,
    /// `_en_point.attack` — precisão.
    pub precisao: i32,
    /// `_en_point.defense` e a defesa do item (`EPSA_EQ_addon`).
    pub defesa: i32,
    /// `_en_point.armor` e a evasão do item.
    pub evasao: i32,
    /// Dano físico e mágico (`template_enhance_damage*`).
    pub dano: i32,
    pub dano_magico: i32,
    /// `EnhanceAllResistance`.
    pub resistencia: i32,
    pub grau_de_ataque: i32,
    pub grau_de_defesa: i32,
    /// `_crit_rate`, em pontos percentuais.
    pub critico: i32,
    /// `_en_percent.damage/magic_dmg` e `EnhanceScaleAllResistance`.
    pub dano_pct: i32,
    pub magico_pct: i32,
    pub resistencia_pct: i32,
}

impl BonusDeAddons {
    /// Soma um addon pelo tratador. `false` quando o tratador não tem porte (ou só mexe na
    /// essência, que já veio sorteada).
    pub fn somar(&mut self, tratador: &str, args: &[i32]) -> bool {
        let v = args.first().copied().unwrap_or(0);
        // Os de essência (`ApplyAtGeneration`) já estão na essência sorteada.
        if ["IA_EA_ESS<", "IA_ED_ESS<", "item_armor_enhance_resistance<", "item_decoration_enchance_resistance<", "enhance_weapon_", "item_decoration_specific_"]
            .iter()
            .any(|p| tratador.starts_with(p))
        {
            return true;
        }
        let base = tratador.strip_prefix("refine_").map(|t| match t {
            // `refine_addon_template<X>` (`item_addon.cpp:1505-1511`).
            "resistance" => "enhance_all_resistance_addon",
            "armor" => "enhance_armor_addon",
            "defense" => "enhance_defense_addon_1arg",
            "max_hp" => "enhance_hp_addon",
            "damage" => "enhance_damage_addon",
            "magic_damage" => "refino_magico",
            "defense_resistance" => "refino_defesa_resistencia",
            _ => "",
        });
        match base.unwrap_or(tratador) {
            "enhance_str_addon" => self.forca += v,
            "enhance_agi_addon" => self.agilidade += v,
            "enhance_vit_addon" => self.vitalidade += v,
            "enhance_eng_addon" => self.energia += v,
            "enhance_hp_addon" | "enhance_hp_addon_2" => self.vida += v,
            "enhance_mp_addon" | "enhance_mp_addon_2" => self.mana += v,
            "enhance_attack_addon" | "enhance_attack_addon_2" => self.precisao += v,
            "enhance_defense_addon" | "enhance_defense_addon_1arg" | "enhance_defense_addon_2" => self.defesa += v,
            "enhance_armor_addon" | "enhance_armor_range_addon" => self.evasao += v,
            "enhance_damage_addon" | "enhance_damage_addon_2" => self.dano += v,
            "enhance_magic_damage_addon" | "enhance_magic_damage_addon_2" => self.dano_magico += v,
            // `refine_addon_template2<enhance_magic_damage_addon, enhance_damage_addon>`.
            "refino_magico" => {
                self.dano_magico += v;
                self.dano += v;
            }
            // `refine_addon_template2<enhance_defense_addon_2, enhance_all_resistance_addon>`.
            "refino_defesa_resistencia" => {
                self.defesa += v;
                self.resistencia += v;
            }
            "enhance_all_resistance_addon" => self.resistencia += v,
            "enhance_attack_degree" => self.grau_de_ataque += v,
            "enhance_defend_degree" => self.grau_de_defesa += v,
            "enhance_crit_rate" => self.critico += v,
            "enhance_damage_scale_addon_2" => self.dano_pct += v,
            "enhance_magic_damage_scale_addon" => self.magico_pct += v,
            "enhance_all_resistance_scale_addon" => self.resistencia_pct += v,
            _ => return false,
        }
        true
    }
}

/// O que o equipamento soma aos atributos: a essência de cada peça (a gravada nos octetos
/// quando há, senão a do modelo) e as propriedades adicionais ([`BonusDeAddons`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Equipamento {
    pub arma: Option<ArmaEmUso>,
    /// `_en_point.defense`, `.armor`, `.max_hp`, `.max_mp`, `.damage_low/high` (acessórios),
    /// `.magic_dmg_low/high`, `.resistance[5]`.
    pub defesa: i32,
    pub evasao: i32,
    pub vida: i32,
    pub mana: i32,
    pub dano: i32,
    pub dano_magico: i32,
    pub resistencias: [i32; 5],
    pub addons: BonusDeAddons,
    /// Addons vestidos sem porte (ids), para o log.
    pub addons_sem_porte: Vec<u32>,
}

/// `Result(base, base2, percent)` (`playertemplate.h:807-812`).
fn resultado(a: i32, b: i32, porcento: i32) -> i32 {
    ((((a + b) as f32) * 0.01 * (100 + porcento) as f32 + 0.5) as i32).max(0)
}

impl Equipamento {
    /// Monta o bônus a partir dos itens do contêiner de equipamento. Slot 0 é a arma
    /// (`EQUIP_INDEX_WEAPON`); as fichas de armadura e acessório somam onde estiverem.
    pub fn dos_itens(itens: &[pw_core::ItemRecord], tabelas: &pw_data_loader::armaduras::TabelasDeEquipamento) -> Self {
        Self::dos_itens_com_addons(itens, tabelas, None)
    }

    /// Com a tabela de addons: a essência gravada nos octetos (`ConteudoDeEquipamento::ler`)
    /// e os addons dela somados. Octetos ilegíveis valem como item sem octetos.
    pub fn dos_itens_com_addons(
        itens: &[pw_core::ItemRecord],
        tabelas: &pw_data_loader::armaduras::TabelasDeEquipamento,
        addons: Option<&pw_data_loader::addons::TabelaDeAddons>,
    ) -> Self {
        use pw_core::FichaDoEquipamento as F;
        let mut e = Equipamento::default();
        for item in itens {
            // Peça acabada não vale nada: `equip_item::VerifyRequirement` só ativa o item
            // com `_base_limit.durability > 0` (`gs/item/equip_item.cpp:60-80`), e é por
            // isso que o original refaz os atributos quando uma peça zera (B61).
            if item.max_durability > 0 && item.durability == 0 {
                continue;
            }
            let mut ficha = tabelas.ficha(item.item_id);
            if let (Some(f), false) = (ficha, item.octets.is_empty()) {
                if let Some(c) = pw_core::ConteudoDeEquipamento::ler(&item.octets, &f) {
                    ficha = Some(c.ficha);
                    if let Some(t) = addons {
                        for a in &c.addons {
                            let ok = t.por_id.get(&a.id()).is_some_and(|d| e.addons.somar(&d.tratador, &a.args));
                            if !ok {
                                e.addons_sem_porte.push(a.id());
                            }
                        }
                    }
                }
            }
            match ficha {
                Some(F::Arma(a)) if item.slot == 0 => {
                    let modo = tabelas.armas.get(&item.item_id).map(|t| t.modo_de_alcance).unwrap_or(1);
                    e.arma = Some(ArmaEmUso {
                        // `short_range_mode` 0 → `weapon_type` 1, 2 → 2 (`generate_item_temp.h:319-324`).
                        dano_pela_agilidade: modo == 0 || modo == 2,
                        de_longe: modo == 0,
                        dano: (a.dano_minimo, a.dano_maximo),
                        dano_magico: (a.dano_magico_minimo, a.dano_magico_maximo),
                        alcance: a.alcance,
                        velocidade_em_ticks: a.velocidade_de_ataque,
                    });
                }
                Some(F::Armadura(a)) => {
                    e.defesa += a.defesa;
                    e.evasao += a.evasao;
                    e.vida += a.hp_extra;
                    e.mana += a.mp_extra;
                    for (i, r) in a.resistencias.iter().enumerate() {
                        e.resistencias[i] += r;
                    }
                }
                Some(F::Decoracao(d)) => {
                    e.dano += d.dano;
                    e.dano_magico += d.dano_magico;
                    e.defesa += d.defesa;
                    e.evasao += d.evasao;
                    for (i, r) in d.resistencias.iter().enumerate() {
                        e.resistencias[i] += r;
                    }
                }
                _ => {}
            }
        }
        e
    }
}

/// O que depende do nível: vida e mana máximas, dano, dano mágico, defesa e resistência.
///
/// É a conta de [`PlayerEntity::do_personagem`], separada para a subida de nível poder
/// refazê-la (`property_policy::UpdatePlayer` no `LevelUp`, `gs/player.cpp:2671`).
pub struct AtributosDeNivel {
    pub max_hp: i32,
    pub max_mp: i32,
    pub dano: i32,
    pub dano_magico: i32,
    pub defesa: i32,
    pub resistencia: i32,
}

pub fn atributos_de_nivel(
    cls: i32,
    nivel: i32,
    vitalidade: i32,
    energia: i32,
    classes: &TabelaDeClasses,
    base: Option<&TabelaDeBase>,
) -> Option<AtributosDeNivel> {
    let cfg = classes.get(cls);
    let base = base.and_then(|b| b.get(cls))?;
    let telescopica = |por_nivel: f32| -> i32 { (nivel as f32 * por_nivel) as i32 - por_nivel as i32 };
    let (max_hp, max_mp) = base.vida_e_mana_maximas(cfg, nivel, vitalidade, energia);
    Some(AtributosDeNivel {
        max_hp,
        max_mp,
        dano: cfg.map(|c| 1 + telescopica(c.dano_por_nivel)).unwrap_or(1),
        dano_magico: cfg.map(|c| 1 + telescopica(c.dano_magico_por_nivel)).unwrap_or(1),
        defesa: cfg.map(|c| telescopica(c.defesa_por_nivel)).unwrap_or(0),
        resistencia: cfg.map(|c| telescopica(c.resistencia_por_nivel)).unwrap_or(0),
    })
}

impl PlayerEntity {
    /// Refaz o que depende do nível, depois de uma subida.
    pub fn recalcular_por_nivel(&mut self, classes: &TabelaDeClasses, base: Option<&TabelaDeBase>) {
        let Some(a) = atributos_de_nivel(
            self.cls as i32,
            self.level,
            self.vitality + self.equipamento.addons.vitalidade,
            self.energy + self.equipamento.addons.energia,
            classes,
            base,
        ) else {
            return;
        };
        let e = self.equipamento.clone();
        let b = e.addons;
        // `UpdateBasic`: os atributos somam `_en_point.str/agi/vit/eng`.
        let (vit, eng, forca, agi) = (self.vitality + b.vitalidade, self.energy + b.energia, self.strength + b.forca, self.agility + b.agilidade);
        // `_en_percent` dos filtros vivos (B53) — ver [`crate::efeitos::Realce`].
        let r = self.efeitos.realce();

        // `UpdateLife`/`UpdateMana` (`playertemplate.h:1153-1189`): o equipamento entra em
        // `_en_point.max_hp/max_mp`; a porcentagem (`Inchp`/`Dechp`) só na vida.
        self.max_hp = resultado(a.max_hp, e.vida + b.vida, r.vida).max(1);
        self.max_mp = a.max_mp + e.mana + b.mana;

        // `UpdateAttack` (`playertemplate.h:916-990`).
        let arma = e.arma;
        let atributo = if arma.is_some_and(|w| w.dano_pela_agilidade) { agi } else { forca };
        let bonus = (atributo as f32 * (100.0 / 150.0) + 0.5) as i32;
        let (item_min, item_max) = arma.map(|w| w.dano).unwrap_or((0, 0));
        // `enh = base_damage + en_percent.damage`.
        let dano_extra = e.dano + b.dano;
        self.attack_min = resultado(item_min + dano_extra, a.dano, bonus + r.dano + b.dano_pct);
        self.attack_max = resultado(item_max + dano_extra, a.dano, bonus + r.dano + b.dano_pct);
        self.dano_bruto = (item_min + dano_extra + a.dano, item_max + dano_extra + a.dano);
        self.bonus_de_dano_pct = bonus;
        let cfg = classes.get(self.cls as i32);
        let alcance_base = cfg.map(|c| c.alcance_de_ataque).unwrap_or(self.attack_range);
        self.attack_range = match arma {
            Some(w) if w.alcance > 0.1 => w.alcance,
            _ => alcance_base,
        } + CORPO_DO_JOGADOR;
        let ticks = match arma {
            Some(w) => if w.velocidade_em_ticks < 4 { 50 } else { w.velocidade_em_ticks },
            None => cfg.map(|c| c.ataque_em_ticks()).unwrap_or((self.attack_speed * 20.0).round() as i32),
        };
        // `Result(attack_speed, en_point.attack_speed, en_percent.attack_speed)`.
        let ticks = resultado(ticks, 0, r.velocidade_de_ataque).clamp(4, 300);
        self.attack_speed = ticks as f32 / 20.0;

        // `UpdateMagic` (`playertemplate.h:1006-1037`): a energia é o bônus.
        let (magico_min, magico_max) = arma.map(|w| w.dano_magico).unwrap_or((0, 0));
        let magico_extra = e.dano_magico + b.dano_magico;
        self.magic_attack_min = resultado(a.dano_magico + magico_extra, magico_min, eng + r.magia + b.magico_pct);
        self.magic_attack_max = resultado(a.dano_magico + magico_extra, magico_max, eng + r.magia + b.magico_pct);
        self.dano_magico_bruto = (a.dano_magico + magico_extra + magico_min, a.dano_magico + magico_extra + magico_max);
        self.bonus_magico_pct = eng;
        let res_pct = ((vit * 2 + eng * 3) as f32 * (100.0 / 2500.0) + 0.5) as i32;
        let res_pontos = (vit + eng) >> 2;
        let res: Vec<i32> = e
            .resistencias
            .iter()
            .map(|x| resultado(a.resistencia, *x + b.resistencia, res_pct + r.resistencia + b.resistencia_pct) + res_pontos)
            .collect();
        (self.def_metal, self.def_wood, self.def_water, self.def_fire, self.def_earth) = (res[0], res[1], res[2], res[3], res[4]);

        // `UpdateDefense` (`playertemplate.h:1117-1133`).
        let def_pct = ((vit * 2 + forca * 3) as f32 * (100.0 / 2500.0) + 0.5) as i32;
        let def_pontos = (vit + forca) >> 2;
        self.def_phys = ((((a.defesa + e.defesa + b.defesa) as f32) * 0.01 * (100 + def_pct + r.defesa) as f32 + 0.5) as i32 + def_pontos).max(0);
        // `_attack_degree`/`_defend_degree` dos addons.
        self.attack_degree = b.grau_de_ataque;
        self.defend_degree = b.grau_de_defesa;
        if let Some(cfg) = cfg {
            self.armor = resultado(cfg.evasao_base(agi), e.evasao + b.evasao, r.evasao);
            // `(base.attack + base_attack + en_point.attack) × (100 + en_percent.attack)%`.
            self.attack_rate = ((((cfg.precisao_base(agi) + b.precisao) as f32) * 0.01 * (100 + r.precisao) as f32 + 0.5) as i32).max(0);
            // `UpdateSpeed` (`playertemplate.h:1077-1101`): `src × (100 + en_percent)%`.
            let fator = 0.01 * (100 + r.velocidade) as f32;
            self.walk_speed = (cfg.velocidade_andando * fator).max(0.1);
            self.move_speed = (cfg.velocidade_correndo * fator).max(0.1);
            // `_crit_rate` soma os pontos do `Incsmite` (`EnhanceCrit`).
            self.crit_rate = (cfg.chance_de_critico + r.critico + b.critico) as f32 / 100.0;
        }
    }

    /// Vitalidade, energia, força e agilidade de `_cur_prop`: a base mais os addons vestidos
    /// (`_en_point.vit/eng/str/agi`, `property_policy::UpdateBasic`). É o que o
    /// `OWN_EXT_PROP` leva: a janela do personagem mostra esse número, e em verde quando algum
    /// item soma (`DlgCharacter.cpp:442-470`); os requisitos de equipamento também o conferem
    /// (`EC_HostPlayer.cpp:4908`). Mandar só a base escondia o bônus (teste de 2026-09-17).
    pub fn atributos_efetivos(&self) -> (i32, i32, i32, i32) {
        let b = self.equipamento.addons;
        (self.vitality + b.vitalidade, self.energy + b.energia, self.strength + b.forca, self.agility + b.agilidade)
    }

    /// `gplayer_imp::PlayerSetStatusPoint` (`player.cpp:8598-8615`): gasta pontos livres
    /// nos quatro atributos.
    ///
    /// Recusa (e não muda nada) quando qualquer parcela ou a soma passa dos pontos livres —
    /// a mesma conferência do original, que responde com os quatro em zero. Aceito, soma os
    /// atributos e refaz o que deles depende: vida e mana máximas (`__UpdateBasic` soma
    /// `vit_hp`/`eng_mp` por ponto, `playertemplate.cpp:570-582`), evasão e precisão pela
    /// agilidade.
    pub fn distribuir_pontos(
        &mut self,
        (vit, eng, str_, agi): (u32, u32, u32, u32),
        classes: &TabelaDeClasses,
        base: Option<&TabelaDeBase>,
    ) -> bool {
        let livres = self.pontos_de_atributo.max(0) as u64;
        let soma = vit as u64 + eng as u64 + str_ as u64 + agi as u64;
        if [vit, eng, str_, agi].iter().any(|&v| v as u64 > livres) || soma > livres {
            return false;
        }
        self.vitality += vit as i32;
        self.energy += eng as i32;
        self.strength += str_ as i32;
        self.agility += agi as i32;
        self.pontos_de_atributo -= soma as i32;
        self.recalcular_por_nivel(classes, base);
        self.hp = self.hp.min(self.max_hp);
        self.mp = self.mp.min(self.max_mp);
        true
    }

    /// Como este jogador aparece para os outros.
    ///
    /// O `dir` vai zerado: a grade espacial guarda posição, não direção — a mesma lacuna
    /// que os NPCs têm. O cliente vira o avatar no primeiro `OBJECT_MOVE` que receber.
    ///
    /// O `crc_e` (equipamento) vai zerado porque o servidor ainda não tem carimbo de
    /// equipamento: quem o calcularia é o mesmo lugar que monta o `GET_OTHER_EQUIP`, e
    /// esse ainda responde a lista inteira a cada pedido. Zero fixo só custa um pedido
    /// extra de equipamento por reaparição — não desenha ninguém errado.
    pub fn vista(&self) -> pw_core::VistaDoJogador {
        pw_core::VistaDoJogador {
            pos: self.position,
            dir: 0,
            cultivo: self.cultivation.clamp(0, 255) as u8,
            sec_level: self.sec_level,
            feminino: self.gender == pw_core::Gender::Female,
            crc_equipamento: 0,
            crc_aparencia: self.crc_aparencia,
            voando: self.voando,
            morto: self.hp <= 0,
            modo_roupa: self.modo_roupa,
            // `mount_color` e `mount_id` do original (`gs/player.cpp:14293-14294`): o
            // `PLAYER_MOUNTING` só alcança quem estava vendo na hora; quem chega depois
            // precisa do estado aqui.
            montaria: self.montaria.map(|m| (m.cor, m.tid as i32)),
            // `shape_form` — nada o liga ainda (o `filter_Fairyform` não está portado).
            forma: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonsterEntity {
    pub id: i64,
    pub template_id: u32,
    pub name: String,
    pub level: i32,
    pub hp: i64,
    pub max_hp: i64,
    pub mp: i32,
    pub max_mp: i32,
    /// `_cur_prop.defense` — reduz o dano físico recebido.
    pub def_phys: i32,
    /// `_cur_prop.armor` — a evasão, que entra na chance de o golpe acertar
    /// (`taxa / (taxa + armadura/2)`). Sem isto o acerto não pode ser calculado.
    pub armor: i32,
    /// `_cur_prop.attack` — a **precisão** do monstro, não o dano dele.
    pub attack_rate: i32,
    /// `_cur_prop.resistance[0..4]`: metal, madeira, água, fogo, terra. Substituiu o
    /// `def_magic` de valor único, que não tinha correspondente no original.
    pub resistances: [i32; 5],
    /// `attack_degree` / `_defend_degree`, que ajustam o dano no fim do cálculo.
    pub attack_degree: i32,
    pub defend_degree: i32,
    /// `_cur_prop.damage_low`/`damage_high` — a faixa de dano físico.
    pub attack_min: i32,
    pub attack_max: i32,
    /// `_cur_prop.addon_damage[0..4]`, na mesma ordem de `resistances`: as parcelas de
    /// dano elemental que o golpe normal do monstro carrega.
    pub magic_attack: [(i32, i32); 5],
    pub attack_range: f32,
    /// `(int)(attack_speed × 20)` do `MONSTER_ESSENCE`, em tiques de 50 ms: o intervalo
    /// entre golpes da sessão de ataque (`ChangeInterval(_cur_prop.attack_speed)`,
    /// `gs/npcsession.cpp:60-70`). Era 1,5 s escrito no `ai.rs` para todo monstro (B62).
    pub ataque_em_ticks: i32,
    /// `_damage_delay` (`npc.cpp:2118`): quantos tiques o dano deste monstro leva para
    /// tirar vida, e a duração da animação do golpe no cliente.
    pub atraso_do_dano_em_ticks: i32,
    /// `aggro_range` do `elements.data`: até onde o monstro persegue. Era `35.0` escrito
    /// no `ai.rs` para todo monstro do jogo.
    pub aggro_range: f32,
    /// `aggressive_mode` do `MONSTER_ESSENCE`: o monstro **procura briga**. No original ele
    /// recebe a máscara `MSG_MASK_PLAYER_MOVE` (`npcgenerator.cpp:2534-2537`) e o jogador,
    /// ao andar, avisa quem está a até `GetMaxMobSightRange` metros
    /// (`playerctrl.cpp:265-276`; 15 m em `worldmanager.cpp:48`). 4.874 dos 8.054 monstros
    /// do `realm_155` são assim (B76).
    pub agressivo: bool,
    /// `sight_range`: até onde ele enxerga. Ainda não decide nada — entra quando a IA
    /// deixar de depender só da tabela de ameaça e passar a procurar alvo sozinha.
    pub sight_range: i32,
    pub exp: i64,
    pub sp: i64,
    pub aipolicy_id: u32,
    pub drop_table_id: u32,
    
    pub position: Vector3,
    pub spawn_center: Vector3,
    /// `run_speed` — perseguir e voltar para casa.
    pub move_speed: f32,
    /// `walk_speed` — o passeio ocioso.
    pub walk_speed: f32,
    /// `inhabit_type`: decide se o passo assenta no chão.
    pub habitat: crate::ai::Habitat,
    /// `patroll_mode`: se o monstro passeia quando está ocioso.
    pub patrulha: bool,
    pub is_dead: bool,
    pub respawn_timer_ms: u32,
    pub respawn_delay_ms: u32,
    /// `prop.remain_time` do `SummonMonster` (`gs/player.cpp:13079`), em milissegundos: o
    /// invocado vive esse tanto e some sozinho. **Zero é para sempre**, que é o caso de todo
    /// monstro de gerador.
    pub vida_restante_ms: u32,
    
    pub target_id: Option<i64>,
    /// Os filtros vivos (efeitos de habilidade) — ver [`crate::efeitos`].
    pub efeitos: crate::efeitos::Efeitos,
    /// `_dmg_list` — quanto cada jogador tirou deste monstro, para dividir a experiência e
    /// decidir o dono do drop (`gnpc_imp::DispatchExp`, `gs/npc.cpp:1515`).
    pub danos: Vec<(i64, i64)>,
    /// `_first_attacker` — ganha `max_hp/4` de dano equivalente na disputa pelo drop.
    pub primeiro_atacante: Option<i64>,
}

/// `Result(a, 0, p)` para os realces de monstro: NPC passa pelo mesmo `property_policy`
/// com classe −1, onde força/agilidade/vitalidade são zero e só o `_en_percent` pesa
/// (`playertemplate.h:916-1133`, `obj_interface.cpp:1614-1627`).
pub fn com_realce(valor: i32, porcento: i32) -> i32 {
    resultado(valor, 0, porcento)
}

impl PlayerEntity {
    /// `gactive_imp::ModifyAP` (`gs/actobject.h:1642-1657`): soma ao chi e prende entre 0 e
    /// o teto. Devolve `true` quando o valor mudou — é o `SetRefreshState()` do original,
    /// que faz o estado ir ao cliente.
    pub fn mexer_no_chi(&mut self, delta: i32) -> bool {
        let novo = (self.ap + delta).clamp(0, self.max_ap.max(0));
        if novo == self.ap {
            return false;
        }
        self.ap = novo;
        true
    }
}

impl MonsterEntity {
    /// `run_speed` com `Slow`/`Speedup` (`UpdateSpeed`, `playertemplate.h:1091-1092`).
    pub fn corrida(&self) -> f32 {
        self.move_speed * 0.01 * (100 + self.efeitos.realce().velocidade) as f32
    }

    /// `walk_speed` com `Slow`/`Speedup`.
    pub fn andar(&self) -> f32 {
        self.walk_speed * 0.01 * (100 + self.efeitos.realce().velocidade) as f32
    }

    /// Registra o dano de um jogador (`OnDamage` → `_dmg_list`).
    pub fn registrar_dano(&mut self, quem: i64, dano: i64) {
        if self.primeiro_atacante.is_none() {
            self.primeiro_atacante = Some(quem);
        }
        match self.danos.iter_mut().find(|d| d.0 == quem) {
            Some(d) => d.1 += dano,
            None => self.danos.push((quem, dano)),
        }
    }
}

impl PlayerEntity {
    /// Monta o jogador que entra no mundo, a partir do personagem do banco mais as duas
    /// tabelas de classe.
    ///
    /// # As fórmulas, e de onde vêm
    ///
    /// O `ptemplate.conf` dá o ponto de partida do nível 1 e o `CHARRACTER_CLASS_CONFIG`
    /// dá o que escala (`player_template::__LoadData` e `__LevelUp`):
    ///
    /// ```text
    /// max_hp  = base.hp  + lvl_hp * (nível-1) + vit_hp * vitalidade
    /// max_mp  = base.mp  + lvl_mp * (nível-1) + eng_mp * energia
    /// dano    = 1 + (int)(nível * lvlup_dmg)      - (int)(lvlup_dmg)
    /// defesa  =     (int)(nível * lvlup_defense) - (int)(lvlup_defense)
    /// precisão = agi_attack * agilidade
    /// evasão   = agi_armor  * agilidade
    /// ```
    ///
    /// O `- (int)(x)` no fim das duas do meio não é enfeite: `__LevelUp` soma
    /// `(int)((l+1)*d) - (int)(l*d)` a cada nível, e a soma telescópica de 1 até N é
    /// `(int)(N*d) - (int)(1*d)`. O dano parte de 1 porque é o que
    /// `player_template::__LoadData` grava em `damage_low`/`damage_high` antes de
    /// qualquer nível.
    ///
    /// # O que este jogador **não** tem
    ///
    /// Equipamento. No original, `UpdateAttack`/`UpdateDefense` somam `_cur_item`,
    /// `_en_point` e `_en_percent` por cima de tudo isto — arma, armadura, encantamento,
    /// refino. Nada disso existe do nosso lado, então o que sai daqui é um personagem
    /// **pelado**: os números são os certos para nível, classe e atributos, e nada mais.
    /// É a diferença entre "aproximado" e "incompleto de um jeito conhecido".
    ///
    /// `base` é `None` quando o realm não trouxe o `ptemplate.conf`; nesse caso vida e
    /// mana máximas ficam iguais às que estão gravadas no banco, e o log de quem chamou
    /// deve dizer isso.
    pub fn do_personagem(
        p: &CharacterDetails,
        classes: &TabelaDeClasses,
        base: Option<&TabelaDeBase>,
    ) -> Self {
        let cls = p.cls as i32;
        let cfg = classes.get(cls);
        let tabela_de_base = base;
        let base = base.and_then(|b| b.get(cls));
        let telescopica = |por_nivel: f32| -> i32 {
            (p.level as f32 * por_nivel) as i32 - por_nivel as i32
        };

        // A mesma conta que a criação de personagem usa, e de propósito num lugar só: as
        // duas divergiram, e o personagem nascia com metade da vida (ver
        // `BaseDaClasse::vida_e_mana_maximas`).
        let (max_hp, max_mp) = match base {
            Some(b) => b.vida_e_mana_maximas(cfg, p.level, p.vitality, p.energy),
            // Sem o `ptemplate.conf` não há ponto de partida: fica o que o banco guardou,
            // que ao menos não é inventado.
            None => (p.hp, p.mp),
        };

        let dano = cfg.map(|c| 1 + telescopica(c.dano_por_nivel)).unwrap_or(1);
        let dano_magico = cfg.map(|c| 1 + telescopica(c.dano_magico_por_nivel)).unwrap_or(1);
        let defesa = cfg.map(|c| telescopica(c.defesa_por_nivel)).unwrap_or(0);
        let resistencia = cfg.map(|c| telescopica(c.resistencia_por_nivel)).unwrap_or(0);

        let mut j = Self {
            role_id: p.id,
            name: p.name.clone(),
            race: p.race,
            cls: p.cls,
            gender: p.gender,
            level: p.level,
            cultivation: p.cultivation,
            // O banco guarda a vida corrente; ela não pode passar do máximo recém-calculado
            // (um personagem que subiu de nível offline, ou um `ptemplate.conf` trocado).
            hp: p.hp.min(max_hp).max(0),
            max_hp,
            mp: p.mp.min(max_mp).max(0),
            max_mp,
            exp: p.exp,
            sp: p.sp,
            money: p.money,
            strength: p.strength,
            agility: p.agility,
            vitality: p.vitality,
            energy: p.energy,
            def_phys: defesa,
            def_metal: resistencia,
            def_wood: resistencia,
            def_water: resistencia,
            def_fire: resistencia,
            def_earth: resistencia,
            attack_min: dano,
            attack_max: dano,
            magic_attack_min: dano_magico,
            magic_attack_max: dano_magico,
            armor: cfg.map(|c| c.evasao_base(p.agility)).unwrap_or(0),
            attack_rate: cfg.map(|c| c.precisao_base(p.agility)).unwrap_or(0),
            // Grau de ataque/defesa e bônus de dano crítico vêm de equipamento e passiva
            // no original. Zero é o valor neutro do cálculo, não um palpite.
            attack_degree: 0,
            defend_degree: 0,
            crit_damage_bonus: 0,
            // # Velocidade, cadência, alcance e regeneração saem do `elements.data`
            //
            // **Não** do `ptemplate.conf`. O original lê os dois arquivos, e o segundo
            // sobrescreve o primeiro: `player_template::__LoadDataFromDataMan`
            // (`gs/playertemplate.cpp:250-301`) roda depois da leitura do `.conf` e grava,
            // do `CHARRACTER_CLASS_CONFIG`, `walk_speed`, `run_speed`, `swim_speed`,
            // `flight_speed`, `attack_speed * 20`, `attack_range`, `hp_gen` e `mp_gen` por
            // cima do que o `.conf` tinha posto. Os valores de velocidade do `.conf` são
            // mortos no original.
            //
            // Até 2026-09-12 lia-se o `.conf`: 2,8 m/s para o Bárbaro. O `elements.data`
            // diz **4,9** — o mesmo número que a captura do servidor 1.2.6 funcional traz
            // no `OWN_EXT_PROP` (`_sync/capturas/full_interno.pcap`, andar 2,0, correr 4,9,
            // nadar 3,0, voar 5,0) e o mesmo que o Murillo mediu em jogo naquela VM.
            //
            // O `.conf` fica como reserva para realm sem o `CHARRACTER_CLASS_CONFIG` (o
            // 1.2.6/v7, que o leitor genérico ainda não cobre).
            attack_speed: cfg
                .map(|c| c.ataque_em_ticks() as f32 / 20.0)
                .or_else(|| base.map(|b| b.ataque_em_ticks as f32 / 20.0))
                .unwrap_or(1.0),
            move_speed: cfg
                .map(|c| c.velocidade_correndo)
                .or_else(|| base.map(|b| b.velocidade_correndo))
                .unwrap_or(3.0),
            walk_speed: cfg
                .map(|c| c.velocidade_andando)
                .or_else(|| base.map(|b| b.velocidade_andando))
                .unwrap_or(1.5),
            swim_speed: cfg
                .map(|c| c.velocidade_nadando)
                .or_else(|| base.map(|b| b.velocidade_nadando))
                .unwrap_or(2.0),
            fly_speed: cfg
                .map(|c| c.velocidade_voando)
                .or_else(|| base.map(|b| b.velocidade_voando))
                .unwrap_or(4.0),
            attack_range: cfg
                .map(|c| c.alcance_de_ataque)
                .or_else(|| base.map(|b| b.alcance_de_ataque))
                .unwrap_or(1.4),
            hp_gen: cfg
                .map(|c| c.regeneracao_de_vida)
                .or_else(|| base.map(|b| b.regeneracao_de_vida))
                .unwrap_or(1),
            mp_gen: cfg
                .map(|c| c.regeneracao_de_mana)
                .or_else(|| base.map(|b| b.regeneracao_de_mana))
                .unwrap_or(1),
            // `crit_rate` está em pontos percentuais na tabela e em fração na entidade.
            crit_rate: cfg.map(|c| c.chance_de_critico as f32 / 100.0).unwrap_or(0.0),
            position: p.position,
            target_id: None,
            efeitos: Default::default(),
            visiveis: std::collections::HashSet::new(),
            centro_do_stream: p.position,
            voando: false,
            montaria: None,
            operacao_de_pet: 0,
            // A escolha sobrevive ao logout: vem do `charactermode` do banco, como o
            // `SetPlayerCharMode` do original faz no login (B83).
            modo_roupa: p.modo_roupa,
            // Quem preenche é `BusServer::colocar_no_mundo`, que tem o repositório à mão;
            // o `CharacterDetails` não traz o privilégio da conta.
            sec_level: 0,
            habilidades: p.skills.iter().map(|h| (h.skill_id, h.level)).collect(),
            crc_aparencia: pw_core::stamp_de_aparencia(&pw_core::bytes_da_aparencia(
                &p.custom_appearance,
            )),
            // Quem preenche é `BusServer::colocar_no_mundo`, que tem o repositório.
            pontos_de_atributo: 0,
            reputacao: p.reputation,
            combate_s: 0,
            contador_hp: 0,
            contador_mp: 0,
            recargas: std::collections::HashMap::new(),
            pecas: [None; PECAS_VESTIDAS],
            auto_hp: None,
            auto_mp: None,
            daimon: None,
            recarga_do_auto_hp_s: 0,
            recarga_do_auto_mp_s: 0,
            npc_em_conversa: None,
            waypoints: p.waypoints.clone(),
            ap: p.ap,
            max_ap: p.max_ap,
            ap_por_golpe: cfg.map(|c| c.chi_por_golpe).unwrap_or(0),
            sentado: false,
            missoes: crate::missoes::ListasDeMissao::default(),
            coleta: None,
            equipamento: Equipamento::default(),
            ataque: None,
            conjuracao: None,
            dano_bruto: (1, 1),
            dano_magico_bruto: (1, 1),
            bonus_de_dano_pct: 0,
            bonus_magico_pct: 0,
        };
        // Os bônus de atributo (`UpdateAttack`/`UpdateDefense`/`UpdateMagic`) valem também
        // sem equipamento; quem carrega os itens chama [`Self::vestir`] depois.
        j.recalcular_por_nivel(classes, tabela_de_base);
        j.hp = j.hp.min(j.max_hp).max(0);
        j.mp = j.mp.min(j.max_mp).max(0);
        j
    }

    /// Troca o equipamento em uso e refaz os atributos (`RefreshEquipment`).
    pub fn vestir(&mut self, equipamento: Equipamento, classes: &TabelaDeClasses, base: Option<&TabelaDeBase>) {
        self.equipamento = equipamento;
        self.recalcular_por_nivel(classes, base);
        self.hp = self.hp.min(self.max_hp);
        self.mp = self.mp.min(self.max_mp);
    }

    /// A precisão e a evasão base do jogador, do `CHARRACTER_CLASS_CONFIG` do
    /// `elements.data`: `agi_attack * agilidade` e `agi_armor * agilidade`
    /// (`player_template::GetBasicAttackRate` / `GetBasicArmor`).
    ///
    /// É **base**: o original soma equipamento, pontos de encantamento e percentuais por
    /// cima (`UpdateAttack` / `UpdateDefense`), e nada disso existe do nosso lado. Sem
    /// esta função, porém, os dois campos não teriam origem nenhuma — que era o estado
    /// anterior, com a fórmula de dano inventando o número.
    ///
    /// `None` quando o realm não tem a tabela (1.2.6/v7) ou a classe não está nela.
    pub fn precisao_e_evasao_base(
        classes: &TabelaDeClasses,
        classe: CharacterClass,
        agilidade: i32,
    ) -> Option<(i32, i32)> {
        let c = classes.get(classe as i32)?;
        Some((c.precisao_base(agilidade), c.evasao_base(agilidade)))
    }

    /// Preenche `attack_rate` e `armor` a partir da tabela de classes, quando ela existir.
    /// Deixa os valores como estavam quando não existir — o chamador decide se isso é
    /// aceitável para o realm dele.
    pub fn aplicar_atributos_de_classe(&mut self, classes: &TabelaDeClasses) -> bool {
        match Self::precisao_e_evasao_base(classes, self.cls, self.agility) {
            Some((precisao, evasao)) => {
                self.attack_rate = precisao;
                self.armor = evasao;
                true
            }
            None => false,
        }
    }
}

impl MonsterEntity {
    /// Instancia um monstro a partir do template do `elements.data`
    /// (`MONSTER_ESSENCE`), como `npcgenerator.cpp` faz no servidor original.
    ///
    /// Só os campos que este `MonsterEntity` tem são preenchidos; o template carrega
    /// bastante coisa a mais (as cinco resistências, dano mágico por classe, habilidades,
    /// raio de ódio, grau de ataque e defesa) que entra quando o combate e a IA reais
    /// forem portados — ver `pw_data_loader::monstros`.
    pub fn do_template(
        id: i64,
        modelo: &TemplateDeMonstro,
        posicao: Vector3,
        respawn_delay_ms: u32,
    ) -> Self {
        Self {
            id,
            template_id: modelo.id,
            name: modelo.nome.clone(),
            level: modelo.nivel,
            hp: modelo.vida as i64,
            max_hp: modelo.vida as i64,
            // O original fixa mana em 1 para monstro (`nt.bp.mp = 1`, `nt.ep.max_mp = 1`):
            // o custo de habilidade de monstro não sai de mana.
            mp: 1,
            max_mp: 1,
            def_phys: modelo.defesa,
            armor: modelo.armadura,
            attack_rate: modelo.taxa_de_ataque,
            resistances: modelo.resistencias,
            attack_degree: modelo.grau_de_ataque,
            defend_degree: modelo.grau_de_defesa,
            attack_min: modelo.dano_fisico.minimo,
            attack_max: modelo.dano_fisico.maximo,
            magic_attack: modelo
                .dano_magico_por_classe
                .map(|f| (f.minimo, f.maximo)),
            attack_range: modelo.alcance_de_ataque,
            ataque_em_ticks: modelo.ataque_em_ticks,
            atraso_do_dano_em_ticks: modelo.atraso_do_dano_em_ticks,
            aggro_range: modelo.raio_de_odio,
            agressivo: modelo.agressivo != 0,
            sight_range: modelo.raio_de_visao,
            exp: modelo.exp as i64,
            sp: modelo.pontos_de_skill as i64,
            aipolicy_id: modelo.politica_de_ia,
            // `MONSTER_ESSENCE` não tem "id de tabela de drop": tem 20 pares
            // item/probabilidade. Fica zero até o sistema de drop existir.
            drop_table_id: 0,
            position: posicao,
            spawn_center: posicao,
            move_speed: modelo.velocidade_correndo,
            walk_speed: modelo.velocidade_andando,
            habitat: crate::ai::Habitat::do_elements(modelo.tipo_de_habitat),
            patrulha: modelo.patrulha,
            is_dead: false,
            respawn_timer_ms: 0,
            respawn_delay_ms,
            vida_restante_ms: 0,
            target_id: None,
            efeitos: Default::default(),
            danos: Vec::new(),
            primeiro_atacante: None,
        }
    }

    /// O monstro genérico de antes do `elements.data` entrar no caminho.
    ///
    /// Continua existindo para dois casos honestos: o realm 1.2.6, cujo `elements.data`
    /// (v7) o leitor genérico ainda não cobre, e o `npcgen.data` que cita um monstro que o
    /// `elements.data` não tem. Some da tela sem explicação seria pior do que aparecer com
    /// atributo genérico e um aviso no log.
    pub fn placeholder(
        id: i64,
        template_id: u32,
        posicao: Vector3,
        respawn_delay_ms: u32,
    ) -> Self {
        Self {
            id,
            template_id,
            name: "Monstro".to_string(),
            level: 1,
            hp: 500,
            max_hp: 500,
            mp: 100,
            max_mp: 100,
            def_phys: 50,
            armor: 50,
            attack_rate: 100,
            resistances: [50; 5],
            attack_degree: 0,
            defend_degree: 0,
            attack_min: 20,
            attack_max: 35,
            magic_attack: [(0, 0); 5],
            attack_range: 2.5,
            // 1,5 s de cadência e 0,5 s de atraso do dano: os valores que o `ai.rs` usava
            // fixos antes de os campos virem do `MONSTER_ESSENCE` (B62).
            ataque_em_ticks: 30,
            atraso_do_dano_em_ticks: 10,
            aggro_range: 15.0,
            agressivo: false,
            sight_range: 20,
            exp: 100,
            sp: 20,
            aipolicy_id: 0,
            drop_table_id: 0,
            position: posicao,
            spawn_center: posicao,
            move_speed: 3.5,
            walk_speed: 1.5,
            habitat: crate::ai::Habitat::Chao,
            // Sem template não se sabe se ele passeia; parado é o que não inventa.
            patrulha: false,
            is_dead: false,
            respawn_timer_ms: 0,
            respawn_delay_ms,
            vida_restante_ms: 0,
            target_id: None,
            efeitos: Default::default(),
            danos: Vec::new(),
            primeiro_atacante: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcEntity {
    pub id: i64,
    pub template_id: u32,
    pub name: String,
    pub position: Vector3,
    pub dialog_id: u32,
    /// Para onde ele olha, em 1/256 de volta — ver [`direcao_do_gerador`].
    pub direcao: u8,
}

/// A direção com que uma criatura nasce, como o `base_spawner` do original a escolhe.
///
/// `GenDir()` (`npcgenerator.h:747-757`): área que é um ponto usa a direção do gerador,
/// `_dir = a3dvector_to_dir(vDir)` (`npcgenerator.cpp:4367`); área com extensão sorteia
/// `Rand(0,255)`. A conversão é `atan2(z, x) × 128/π`, truncada e mascarada com `0xFF`
/// (`common/types.h:99-107`) — o byte é 1/256 de volta, como o `dir` do `info_npc`
/// (`protocol_imp.h:297-306`).
///
/// Mandávamos **zero** para todos, e em jogo os NPCs ficavam todos virados para o mesmo
/// lado (teste de 2026-09-17, B59).
pub fn direcao_do_gerador(dir: Vector3, extensao: Vector3) -> u8 {
    let ponto = extensao.x * extensao.x + extensao.y * extensao.y + extensao.z * extensao.z < 1e-3;
    if ponto {
        ((dir.z.atan2(dir.x) * (128.0 / std::f32::consts::PI)) as i64 & 0xFF) as u8
    } else {
        rand::random::<u8>()
    }
}

/// Um "recurso do mapa": minério, erva, tronco — o que o cliente chama de *matter*.
///
/// Vem do `npcgen.data` (`SpawnType::ResourceMine`) e viaja no `MATTER_ENTER_WORLD` (18),
/// que é comando próprio: matéria **não** é NPC. O cliente separa as três famílias pelo
/// id, com máscaras de bit (`EC_GPDataType.h:25-27`):
///
/// ```text
/// ISPLAYERID(id)  (id) && !((id) & 0x80000000)
/// ISNPCID(id)     ((id) & 0x80000000) && !((id) & 0x40000000)
/// ISMATTERID(id)  ((id) & 0xC0000000) == 0xC0000000
/// ```
///
/// O `npcgen.rs` já monta o id de matéria com `0xC0000000` (`npcgen.rs:422`), então os
/// ids que chegam aqui já satisfazem `ISMATTERID`.
///
/// Não há atributo nenhum: o cliente lê o modelo, o ícone e o nome do `elements.data`
/// dele, pelo `tid` (`CECMatter::ReadDataFromDatabase`). O servidor só precisa dizer
/// **onde** e **qual**.
#[derive(Debug, Clone, PartialEq)]
pub struct MatterEntity {
    pub id: i64,
    /// `tid` do `MINE_ESSENCE`. O cliente o mascara com `0x0000ffff`
    /// (`EC_Matter.cpp:166`), e o bit 31 é sinalizador (`ITEMFLAG_EXTPROP`), não parte do
    /// id — os do `npcgen.data` deste realm cabem folgadamente em 16 bits.
    pub template_id: u32,
    pub position: Vector3,
    /// Tempo de renascer depois de colhida, em segundos: `max(BASE_REBORN_TIME +
    /// dwRefreshTime, 15)` (`npcgenerator.cpp:3920-3922`), já calculado no `npcgen.rs`.
    pub renascer_s: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDropEntity {
    pub id: i64,
    pub item_id: u32,
    pub count: u32,
    pub position: Vector3,
    pub owner_role_id: Option<RoleId>,
    pub protect_timer_ms: u32,
    pub despawn_timer_ms: u32,
    /// O conteúdo sorteado de um equipamento (`ConteudoDeEquipamento::escrever`), vazio para
    /// o resto.
    pub octetos: Vec<u8>,
}

/// O Daimon vestido (o "pequeno elfo" do original, `elf_item`), no slot
/// `EQUIP_INDEX_ELF` = **23** (`gs/item.h:219`).
///
/// O estado dele **é** o bloco de dados do item: `elf_essence` (empacotado em 1 byte,
/// `gs/item/item_elf.h:84-102`), depois a lista de equipamento e a de habilidades, nessa
/// ordem (`elf_item::Save`, `item_elf.cpp:172-185`). É o que `generate_elf` escreve para um
/// Daimon novo (`gs/template/generate_item_temp.h:2442-2524`).
#[derive(Debug, Clone, PartialEq)]
pub struct Daimon {
    pub exp: u32,
    pub nivel: i16,
    pub total_de_atributos: i16,
    pub forca: i16,
    pub agilidade: i16,
    pub vitalidade: i16,
    pub energia: i16,
    pub total_de_genios: i16,
    pub genios: [i16; 5],
    pub refino: i16,
    /// `stamina` — 20000 num Daimon novo.
    pub vigor: i32,
    pub status: i32,
    /// Ids de equipamento do próprio Daimon (não portados: a lista é lida e devolvida
    /// intacta).
    pub equipamento: Vec<u32>,
    /// `(id, nível)` de cada habilidade dele.
    pub habilidades: Vec<(u16, i16)>,
}

impl Daimon {
    /// Um Daimon recém-gerado (`generate_elf`): nível 1, sem atributo distribuído, um ponto
    /// de gênio, 20000 de vigor e as habilidades iniciais do `GOBLIN_ESSENCE` no nível 1.
    pub fn novo(habilidades_iniciais: &[u16]) -> Self {
        Self {
            exp: 0,
            nivel: 1,
            total_de_atributos: 0,
            forca: 0,
            agilidade: 0,
            vitalidade: 0,
            energia: 0,
            total_de_genios: 1,
            genios: [0; 5],
            refino: 0,
            vigor: 20_000,
            status: 0,
            equipamento: Vec::new(),
            habilidades: habilidades_iniciais.iter().map(|id| (*id, 1)).collect(),
        }
    }

    /// Lê o bloco. `None` quando não fecha — bloco curto ou contagem impossível.
    pub fn ler(b: &[u8]) -> Option<Self> {
        if b.len() < 46 {
            return None;
        }
        let u32_em = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().ok().unwrap_or([0; 4]));
        let i32_em = |i: usize| i32::from_le_bytes(b[i..i + 4].try_into().ok().unwrap_or([0; 4]));
        let i16_em = |i: usize| i16::from_le_bytes(b[i..i + 2].try_into().ok().unwrap_or([0; 2]));
        // `elf_essence` com `#pragma pack(1)` (`gs/item/item_elf.h:84-102`): exp 0, level 4,
        // total_attribute 6, os quatro atributos 8..16, total_genius 16, genius[5] 18..28,
        // refine_level 28, stamina 30, status_value 34 — 38 bytes.
        let mut genios = [0i16; 5];
        for (n, g) in genios.iter_mut().enumerate() {
            *g = i16_em(18 + n * 2);
        }
        let mut d = Self {
            exp: u32_em(0),
            nivel: i16_em(4),
            total_de_atributos: i16_em(6),
            forca: i16_em(8),
            agilidade: i16_em(10),
            vitalidade: i16_em(12),
            energia: i16_em(14),
            total_de_genios: i16_em(16),
            genios,
            refino: i16_em(28),
            vigor: i32_em(30),
            status: i32_em(34),
            equipamento: Vec::new(),
            habilidades: Vec::new(),
        };
        // `SaveEquip`/`SaveSkill`: uma contagem de 4 bytes antes de cada lista
        // (`item_elf.cpp:122-160`).
        let mut i = 38;
        let n = i32_em(i).max(0) as usize;
        i += 4;
        if b.len() < i + n * 4 + 4 {
            return None;
        }
        for _ in 0..n {
            d.equipamento.push(u32_em(i));
            i += 4;
        }
        let n = i32_em(i).max(0) as usize;
        i += 4;
        if b.len() < i + n * 4 {
            return None;
        }
        for _ in 0..n {
            d.habilidades.push((u16::from_le_bytes([b[i], b[i + 1]]), i16_em(i + 2)));
            i += 4;
        }
        Some(d)
    }

    /// O bloco de volta, na ordem do `Save` do original.
    pub fn bloco(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(46 + self.habilidades.len() * 4);
        b.extend_from_slice(&self.exp.to_le_bytes());
        for v in [
            self.nivel,
            self.total_de_atributos,
            self.forca,
            self.agilidade,
            self.vitalidade,
            self.energia,
            self.total_de_genios,
        ] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for g in self.genios {
            b.extend_from_slice(&g.to_le_bytes());
        }
        b.extend_from_slice(&self.refino.to_le_bytes());
        b.extend_from_slice(&self.vigor.to_le_bytes());
        b.extend_from_slice(&self.status.to_le_bytes());
        b.extend_from_slice(&(self.equipamento.len() as i32).to_le_bytes());
        for e in &self.equipamento {
            b.extend_from_slice(&e.to_le_bytes());
        }
        b.extend_from_slice(&(self.habilidades.len() as i32).to_le_bytes());
        for (id, nivel) in &self.habilidades {
            b.extend_from_slice(&id.to_le_bytes());
            b.extend_from_slice(&nivel.to_le_bytes());
        }
        b
    }
}

/// O Daimon vestido: o item (para gravar o bloco de volta), o `exp_factor` do
/// `GOBLIN_ESSENCE` e o estado.
#[derive(Debug, Clone, PartialEq)]
pub struct DaimonVestido {
    pub slot: u16,
    pub item_id: u32,
    /// `prop.exp_factor`: multiplica a curva do jogador para dar a do Daimon
    /// (`gs/item/item_elf.cpp:696-710`).
    pub fator_de_exp: f32,
    pub estado: Daimon,
    /// O bloco mudou e ainda não foi gravado.
    pub sujo: bool,
}

impl DaimonVestido {
    /// `elf_item::InsertExp` (`gs/item/item_elf.cpp:692-750`).
    ///
    /// `exp_do_nivel` é a experiência que o **jogador** precisa para subir do nível do
    /// Daimon — `GetLvlupExp(0, nível) × exp_factor` é o que o Daimon precisa. `nivel_da_exp`
    /// é o nível de quem deu a experiência (o jogador, ou 100 na pílula).
    ///
    /// Devolve `(ganhou_alguma, subiu_de_nivel)`.
    pub fn receber_exp(
        &mut self,
        mut exp: u32,
        nivel_da_exp: i16,
        nivel_do_jogador: i16,
        exp_do_nivel: impl Fn(i16) -> u32,
    ) -> (bool, bool) {
        let d = &mut self.estado;
        if exp == 0 || nivel_do_jogador <= 0 || nivel_da_exp <= 0 || nivel_do_jogador < d.nivel {
            return (false, false);
        }
        let precisa = |nivel: i16| (exp_do_nivel(nivel) as f64 * self.fator_de_exp as f64) as u32;
        if nivel_do_jogador == d.nivel && precisa(d.nivel) <= d.exp + 1 {
            return (false, false);
        }
        let inicial = exp;
        let mut subiu = false;
        while exp > 0 {
            let ate_subir = if precisa(d.nivel) <= d.exp { 1 } else { precisa(d.nivel) - d.exp };
            // `GetExpObtainFactor`: `elf_exp_loss_constant[i] == i`, então a proporção é a
            // dos níveis, com o mínimo de 10 % (`item_elf.cpp:754-773`).
            let fator = if nivel_da_exp <= d.nivel {
                1.0
            } else {
                (d.nivel as f64 / nivel_da_exp as f64).max(0.1)
            };
            let pode = (exp as f64 * fator + 0.00001) as u32;
            if pode >= ate_subir {
                if d.nivel >= nivel_do_jogador {
                    // No teto do nível do jogador o Daimon para a um ponto de subir.
                    d.exp += ate_subir - 1;
                    break;
                }
                d.subir_de_nivel();
                subiu = true;
                let gasto = ((ate_subir as f64) / fator).ceil() as u32;
                exp = exp.saturating_sub(gasto);
            } else {
                d.exp += pode;
                let gasto = ((pode as f64) / fator).ceil() as u32;
                exp = exp.saturating_sub(gasto);
                break;
            }
        }
        let ganhou = exp < inicial;
        self.sujo |= ganhou;
        (ganhou, subiu)
    }
}

impl Daimon {
    /// `elf_item::LevelUp` (`gs/item/item_elf.cpp:775-816`): zera a experiência, sobe o
    /// nível e dá **um** ponto de atributo, mais um de gênio a cada 5 níveis até o 100 (e um
    /// por nível depois disso).
    ///
    /// `falta`: o bônus de atributo sorteado de 10 em 10 níveis (`rand_prop` do
    /// `GOBLIN_ESSENCE` com `abase::RandSelect`, `item_elf.cpp:779-796`) — a semântica do
    /// sorteio não está medida.
    pub fn subir_de_nivel(&mut self) {
        let proximo = self.nivel + 1;
        self.exp = 0;
        self.nivel = proximo;
        self.total_de_atributos += 1;
        if proximo > 100 || proximo % 5 == 0 {
            self.total_de_genios += 1;
        }
    }
}
