//! Os subcomandos do mundo 3D que o servidor de mundo entende, decodificados.
//!
//! # Onde isto se encaixa
//!
//! O `pw-bus` entrega um `data` opaco; o [`SubComando`](crate::SubComando) separa o
//! cabeçalho de 2 bytes; e aqui cada comando vira uma struct. Daí para a frente é lógica
//! de jogo, sem byte nenhum à vista.
//!
//! # Os deslocamentos não são escolhidos aqui
//!
//! Todos vêm de `specs/protocol/gamedata_153.json`, que saiu dos cabeçalhos C++ originais
//! e foi conferido pelo `g++ -m32`. Cada struct abaixo cita o nome do IR de onde veio, e
//! `tests/comandos_contra_o_ir.rs` monta um payload com os campos **nos deslocamentos que
//! o IR anuncia** e cobra que a decodificação devolva exatamente aqueles valores. Mudar
//! um `r.u16()` de lugar quebra o teste.
//!
//! Os deslocamentos do IR contam a partir do início da struct, **incluindo o cabeçalho de
//! 2 bytes**. O que chega aqui é o payload já sem ele, então cada deslocamento daqui é o
//! do IR menos 2. O teste faz essa conta explicitamente, para que ela não vire folclore.
//!
//! # Uma ressalva de versão que precisa ficar escrita
//!
//! O IR é do **1.5.3**. Não temos fontes do 1.2.6, então os layouts dele estão
//! *presumidos iguais*. Para o `PLAYER_MOVE` há evidência prática: o `gateway.rs` lia a
//! posição em `cur_pos` (deslocamento 2) e o cliente 1.2.6 andava direito — isto é, os
//! primeiros 14 bytes concordam. O resto do struct **não** tem essa confirmação, e por
//! isso [`PlayerMove::ler`] aceita um payload curto em vez de recusá-lo: um 1.2.6 que
//! mande menos bytes continua andando, e a diferença aparece no log em vez de virar
//! desconexão.

use pw_wire::gamedata::{Reader, Vec3};

/// Os ids dos subcomandos que o servidor de mundo trata.
///
/// Não são números escolhidos: vêm da tabela `C2S` do IR, e
/// `tests/comandos_contra_o_ir.rs` cobra cada um contra ela.
pub mod ids {
    /// `SRV::C2S::CMD::player_move`
    pub const PLAYER_MOVE: u16 = 0;
    /// `SRV::C2S::CMD::logout`
    pub const LOGOUT: u16 = 1;
    /// `SRV::C2S::CMD::select_target`
    pub const SELECT_TARGET: u16 = 2;
    /// `SRV::C2S::CMD::normal_attack`
    pub const NORMAL_ATTACK: u16 = 3;
    /// `SRV::C2S::CMD::player_stop_move`
    pub const STOP_MOVE: u16 = 7;
    /// `UNSELECT` — só cabeçalho, sem struct no IR.
    pub const UNSELECT: u16 = 8;
    /// `SRV::C2S::CMD::resurrect` — o servidor chama de `RESURRECT_IN_TOWN`.
    pub const REVIVE_VILLAGE: u16 = 4;
    /// `SRV::C2S::CMD::get_item_info`
    pub const GET_ITEM_INFO: u16 = 9;
    /// `SRV::C2S::CMD::get_inventory_detail`
    pub const GET_IVTR_DETAIL: u16 = 11;
    /// `SRV::C2S::CMD::exchange_inventory_item`
    pub const EXG_IVTR_ITEM: u16 = 12;
    /// `SRV::C2S::CMD::move_inventory_item`
    pub const MOVE_IVTR_ITEM: u16 = 13;
    /// `SRV::C2S::CMD::exchange_equip_item`
    pub const EXG_EQUIP_ITEM: u16 = 16;
    /// `SRV::C2S::CMD::equip_item`
    pub const EQUIP_ITEM: u16 = 17;
    /// `SRV::C2S::CMD::move_item_to_equipment`
    pub const MOVE_ITEM_TO_EQUIP: u16 = 18;
    /// `CANCEL_ACTION` — só cabeçalho.
    pub const CANCEL_ACTION: u16 = 42;
    /// `SIT_DOWN` — só cabeçalho.
    pub const SIT_DOWN: u16 = 46;
    /// `STAND_UP` — só cabeçalho.
    pub const STAND_UP: u16 = 47;
    /// `SRV::C2S::CMD::emote_action`
    pub const EMOTE_ACTION: u16 = 48;
    /// `SRV::C2S::CMD::enter_sanctuary`
    pub const ENTER_SANCTUARY: u16 = 75;
    /// `SRV::C2S::CMD::service_serve` — o comando dos serviços de NPC.
    pub const SEVNPC_SERVE: u16 = 37;
    /// `SRV::C2S::CMD::use_item`
    pub const USE_ITEM: u16 = 40;
    /// `SRV::C2S::CMD::cast_skill`
    pub const CAST_SKILL: u16 = 41;
    /// `SRV::C2S::CMD::cast_instant_skill` — mesmo layout do 41.
    pub const CAST_INSTANT_SKILL: u16 = 80;
    /// `SRV::C2S::CMD::team_invite` — 6 bytes, `id` no deslocamento 2.
    pub const TEAM_INVITE: u16 = 27;
    /// `SRV::C2S::CMD::team_agree_invite`
    pub const TEAM_AGREE_INVITE: u16 = 28;
    /// `SRV::C2S::CMD::team_reject_invite`
    pub const TEAM_REJECT_INVITE: u16 = 29;
    /// `TEAM_LEAVE_PARTY` — só cabeçalho.
    pub const TEAM_LEAVE_PARTY: u16 = 30;
    /// `SRV::C2S::CMD::self_get_property` — só cabeçalho. O cliente pede o próprio bloco
    /// de estado.
    pub const GET_EXT_PROP: u16 = 21;
    /// `SRV::C2S::CMD::get_all_data` — `detail_inv` (2), `detail_equip` (3),
    /// `detail_task` (4).
    pub const GET_ALL_DATA: u16 = 39;
    /// `SRV::C2S::CMD::query_player_info_1` — `count` (2) e a lista de ids a partir do 4.
    pub const QUERY_PLAYER_INFO_1: u16 = 67;
    /// `SRV::C2S::CMD::query_npc_info_1` — mesmo formato do 67.
    pub const QUERY_NPC_INFO_1: u16 = 68;
    /// `QUERY_CASH_INFO` — só cabeçalho. O cliente pergunta o saldo.
    pub const QUERY_CASH_INFO: u16 = 110;
    /// `SRV::C2S::CMD::service_hello` — abrir diálogo com um NPC. O IR marca este id como
    /// só cabeçalho; está errado, ver [`SevnpcHello`].
    pub const SEVNPC_HELLO: u16 = 35;
    /// `SRV::C2S::CMD::task_notify` — o cliente reporta algo ao sistema de missões. O IR
    /// marca este id como só cabeçalho; está errado, ver [`TaskNotify`].
    pub const TASK_NOTIFY: u16 = 49;
}

/// Tamanho do cabeçalho de subcomando (`cmd_header { unsigned short cmd; }`).
///
/// É o deslocamento que separa a contagem do IR da contagem do payload.
pub const BYTES_DO_CABECALHO: usize = 2;

/// `SRV::C2S::CMD::player_move` — 33 bytes, dos quais 31 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `cur_pos` | 2 | `A3DVECTOR` (3 × f32) |
/// | `next_pos` | 14 | `A3DVECTOR` |
/// | `use_time` | 26 | `unsigned short` |
/// | `speed` | 28 | `unsigned short` |
/// | `move_mode` | 30 | `unsigned char` |
/// | `cmd_seq` | 31 | `unsigned short` |
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerMove {
    /// Onde o cliente diz que está agora. É esta que vira a posição do jogador.
    pub cur_pos: Vec3,
    /// Para onde ele está indo. O servidor usa para prever e para conferir velocidade;
    /// gravá-la como posição atual teleportaria o personagem um passo à frente.
    pub next_pos: Vec3,
    pub use_time: u16,
    pub speed: u16,
    pub move_mode: u8,
    /// Sequencial do cliente. Serve para descartar pacotes fora de ordem — UDP não, mas
    /// o cliente reenvia.
    pub cmd_seq: u16,
}

impl PlayerMove {
    /// Payload completo, em bytes (o struct do IR menos o cabeçalho).
    pub const BYTES: usize = 33 - BYTES_DO_CABECALHO;

    /// Decodifica o payload (já **sem** o cabeçalho de 2 bytes).
    ///
    /// Exige apenas o `cur_pos`, que é a parte confirmada no 1.2.6 (ver o cabeçalho do
    /// módulo). O que faltar fica em zero, e `completo()` diz se foi o caso.
    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let cur_pos = r.vec3().ok()?;

        // A partir daqui é melhor-esforço: um payload curto não invalida o movimento.
        let next_pos = r.vec3().unwrap_or(cur_pos);
        let use_time = r.u16().unwrap_or(0);
        let speed = r.u16().unwrap_or(0);
        let move_mode = r.u8().unwrap_or(0);
        let cmd_seq = r.u16().unwrap_or(0);

        Some(Self {
            cur_pos,
            next_pos,
            use_time,
            speed,
            move_mode,
            cmd_seq,
        })
    }

    /// O payload trazia o struct inteiro que o IR descreve?
    ///
    /// `false` num cliente 1.5.3 significa pacote truncado; num 1.2.6, provavelmente que
    /// o layout daquela versão é menor — que é justamente o que falta descobrir.
    pub fn completo(payload: &[u8]) -> bool {
        payload.len() >= Self::BYTES
    }
}

/// `SRV::C2S::CMD::logout` — 6 bytes, 4 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `logout_type` | 2 | `int` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Logout {
    pub logout_type: i32,
}

/// O que o jogador pediu ao deslogar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoDeSaida {
    /// `_PLAYER_LOGOUT_FULL` — sair do jogo.
    SairDoJogo,
    /// `_PLAYER_LOGOUT_HALF` — voltar à seleção de personagens, sem cair a conexão.
    SelecaoDePersonagem,
    /// Qualquer outro valor. Tratado como saída completa, que é o lado seguro: manter o
    /// jogador no mundo por causa de um número desconhecido é pior do que tirá-lo.
    Desconhecido(i32),
}

impl Logout {
    pub const BYTES: usize = 6 - BYTES_DO_CABECALHO;

    /// Decodifica o payload (já sem o cabeçalho).
    ///
    /// Um payload de 1 byte é aceito porque era o que o `gateway.rs` tolerava para o
    /// 1.2.6; sem fontes daquela versão, estreitar isso agora seria trocar um
    /// comportamento que funciona por uma suposição.
    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        if let Ok(v) = r.i32() {
            return Some(Self { logout_type: v });
        }
        // Caminho de compatibilidade: um único byte.
        let mut r = Reader::new(payload);
        r.u8().ok().map(|b| Self {
            logout_type: b as i32,
        })
    }

    pub fn tipo(&self) -> TipoDeSaida {
        match self.logout_type {
            0 => TipoDeSaida::SairDoJogo,
            1 => TipoDeSaida::SelecaoDePersonagem,
            outro => TipoDeSaida::Desconhecido(outro),
        }
    }
}

/// `SRV::C2S::CMD::select_target` — 6 bytes, 4 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `id` | 2 | `int` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectTarget {
    /// `0` significa "nenhum alvo" — é como o cliente desmarca.
    pub id: i32,
}

impl SelectTarget {
    pub const BYTES: usize = 6 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        r.i32().ok().map(|id| Self { id })
    }
}

/// `SRV::C2S::CMD::normal_attack` — 3 bytes, 1 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `force_attack` | 2 | `char` |
///
/// # O alvo não vem no pacote
///
/// Este comando **não carrega id de alvo** — só o `force_attack`. Quem ataca o quê é
/// decidido pelo `SELECT_TARGET` anterior, e o servidor é que guarda essa escolha.
///
/// O `gateway.rs` lia 4 bytes a partir do deslocamento 2 como se fossem um `int` de
/// alvo, atrás de um `if len() >= 6`. Como o pacote tem 3 bytes, a guarda nunca passava e
/// ele caía no alvo da sessão — funcionava por acidente. Um cliente que mandasse um
/// pacote maior faria o servidor atacar um id lido de lixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalAttack {
    /// Ataque forçado: o jogador mandou atacar mesmo sem o alvo ser hostil.
    pub force_attack: i8,
}

impl NormalAttack {
    pub const BYTES: usize = 3 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        r.i8().ok().map(|force_attack| Self { force_attack })
    }
}

/// `SRV::C2S::CMD::player_stop_move` — 22 bytes, 20 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `pos` | 2 | `A3DVECTOR` |
/// | `speed` | 14 | `unsigned short` |
/// | `dir` | 16 | `unsigned char` |
/// | `move_mode` | 17 | `unsigned char` |
/// | `cmd_seq` | 18 | `unsigned short` |
/// | `use_time` | 20 | `unsigned short` |
///
/// Note que a ordem **não** é a mesma do `PLAYER_MOVE`: lá `use_time` vem antes de
/// `speed`, aqui vem por último. Copiar o decodificador de um para o outro daria um
/// personagem parando com a velocidade errada.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopMove {
    pub pos: Vec3,
    pub speed: u16,
    pub dir: u8,
    pub move_mode: u8,
    pub cmd_seq: u16,
    pub use_time: u16,
}

impl StopMove {
    pub const BYTES: usize = 22 - BYTES_DO_CABECALHO;

    /// Mesma tolerância do [`PlayerMove::ler`]: a posição basta, o resto é melhor-esforço
    /// enquanto o layout do 1.2.6 não for confirmado.
    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let pos = r.vec3().ok()?;
        Some(Self {
            pos,
            speed: r.u16().unwrap_or(0),
            dir: r.u8().unwrap_or(0),
            move_mode: r.u8().unwrap_or(0),
            cmd_seq: r.u16().unwrap_or(0),
            use_time: r.u16().unwrap_or(0),
        })
    }
}

/// Um par de índices de slot — a forma de vários comandos de item.
///
/// `SRV::C2S::CMD::{get_item_info, exchange_inventory_item, exchange_equip_item,
/// equip_item, move_item_to_equipment}` têm todos o mesmo layout de 4 bytes: cabeçalho e
/// dois `unsigned char` nos deslocamentos 2 e 3. O que muda é o **significado** dos dois,
/// e por isso os campos aqui têm nome genérico e quem chama é que dá sentido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParDeSlots {
    pub a: u8,
    pub b: u8,
}

impl ParDeSlots {
    pub const BYTES: usize = 4 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let a = r.u8().ok()?;
        let b = r.u8().ok()?;
        Some(Self { a, b })
    }
}

/// `SRV::C2S::CMD::get_inventory_detail` — 3 bytes, 1 de payload: qual contêiner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetIvtrDetail {
    pub onde: u8,
}

impl GetIvtrDetail {
    pub const BYTES: usize = 3 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        // Um payload vazio significa "a bolsa", que é o que o `gateway.rs` assumia.
        Some(Self {
            onde: Reader::new(payload).u8().unwrap_or(0),
        })
    }
}

/// `SRV::C2S::CMD::move_inventory_item` — 8 bytes, 6 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `src` | 2 | `unsigned char` |
/// | `dest` | 3 | `unsigned char` |
/// | `amount` | 4 | `size_t` (4 bytes no i386) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveIvtrItem {
    pub src: u8,
    pub dest: u8,
    /// Quantos da pilha. O tratamento atual ignora este campo e troca os slots
    /// inteiros — ver a nota em `BusServer::mover_item`.
    pub amount: u32,
}

impl MoveIvtrItem {
    pub const BYTES: usize = 8 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let src = r.u8().ok()?;
        let dest = r.u8().ok()?;
        Some(Self {
            src,
            dest,
            amount: r.u32().unwrap_or(1),
        })
    }
}

/// `SRV::C2S::CMD::emote_action` — 4 bytes, 2 de payload: qual emote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmoteAction {
    pub action: u16,
}

impl EmoteAction {
    pub const BYTES: usize = 4 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        Reader::new(payload).u16().ok().map(|action| Self { action })
    }
}

/// `SRV::C2S::CMD::use_item` — 10 bytes, 8 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `where` | 2 | `unsigned char` |
/// | `count` | 3 | `unsigned char` |
/// | `index` | 4 | `unsigned short` |
/// | `item_id` | 6 | `int` |
///
/// O `index` tem **dois** bytes. O `gateway.rs` o convertia para `u8` logo depois de ler,
/// o que trunca qualquer slot acima de 255 — sem erro, e sem o jogador entender por que o
/// item do fundo da bolsa "não funciona".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseItem {
    pub onde: u8,
    pub quantos: u8,
    pub slot: u16,
    pub item_id: i32,
}

impl UseItem {
    pub const BYTES: usize = 10 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        Some(Self {
            onde: r.u8().ok()?,
            quantos: r.u8().ok()?,
            slot: r.u16().ok()?,
            item_id: r.i32().ok()?,
        })
    }
}

/// `SRV::C2S::CMD::cast_skill` (41) e `cast_instant_skill` (80) — mesmo layout.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `skill_id` | 2 | `int` |
/// | `force_attack` | 6 | `unsigned char` |
/// | `target_count` | 7 | `unsigned char` |
/// | `targets` | 8 | lista de `int` |
///
/// O `gateway.rs` lia o alvo em `data[7..11]` — que começa no `target_count` e engole
/// três bytes do primeiro alvo. A lista começa no deslocamento 8, isto é, `data[10]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastSkill {
    pub skill_id: i32,
    pub force_attack: u8,
    pub alvos: Vec<i32>,
}

impl CastSkill {
    pub fn ler(payload: &[u8]) -> Option<Self> {
        /// Teto para a contagem de alvos: ela vem do cliente.
        const TETO: usize = 32;

        let mut r = Reader::new(payload);
        let skill_id = r.i32().ok()?;
        let force_attack = r.u8().ok()?;
        let quantos = (r.u8().ok()? as usize).min(TETO);

        let alvos = (0..quantos).map_while(|_| r.i32().ok()).collect();
        Some(Self {
            skill_id,
            force_attack,
            alvos,
        })
    }
}

/// `SRV::C2S::CMD::query_player_info_1` (67) e `query_npc_info_1` (68) — mesmo layout.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `count` | 2 | `unsigned short` |
/// | `id[]` | 4 | `int` cada |
///
/// É a consulta periódica que o cliente faz para atualizar barra de vida do que está na
/// tela — dele mesmo, dos companheiros e dos monstros.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultaDeIds {
    pub ids: Vec<i32>,
}

impl ConsultaDeIds {
    /// Teto de ids por consulta.
    ///
    /// A contagem vem do cliente e o `Vec` seria dimensionado por ela. Sem teto, um
    /// `count` de 65535 num pacote de 6 bytes faz o servidor reservar memória para 65535
    /// respostas antes de descobrir que não há ids nenhum. O teto é generoso perto do que
    /// cabe numa tela e barato perto do que custa não tê-lo.
    pub const TETO: usize = 256;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let quantos = (r.u16().ok()? as usize).min(Self::TETO);
        // `map_while` para que um pacote que promete mais ids do que traz devolva os que
        // realmente vieram, em vez de nada.
        let ids = (0..quantos).map_while(|_| r.i32().ok()).collect();
        Some(Self { ids })
    }
}

/// `SRV::C2S::CMD::get_all_data` (39) — 5 bytes, 3 de payload.
///
/// | Campo | Deslocamento no IR | Tipo |
/// | :--- | ---: | :--- |
/// | `detail_inv` | 2 | `char` |
/// | `detail_equip` | 3 | `char` |
/// | `detail_task` | 4 | `char` |
///
/// No cliente a struct é `cmd_get_all_data { BYTE byPack; BYTE byEquip; BYTE byTask; }`,
/// com o comentário "Get detail info. flag". No servidor original o comando chama
/// `pImp->SendAllData(gad.detail_inv, gad.detail_equip, gad.detail_task)`
/// (`playercmd.cpp:1863`) — **os três são usados**.
///
/// O `gateway.rs` não lia nenhum: mandava sempre bolsa, equipamento, dinheiro e missões,
/// para qualquer combinação de sinalizadores.
///
/// O corpo do `SendAllData` não está entre as fontes vazadas, então o que cada valor
/// significa em detalhe não é verificável daqui. O que **é** verificável é que o cliente
/// os manda e o servidor os passa adiante; tratamos zero como "não quero esta parte", que
/// é a leitura literal de "detail flag", e está anotado como suposição em
/// `docs/ESTADO_E_RETOMADA.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetAllData {
    pub detalhe_bolsa: u8,
    pub detalhe_equipamento: u8,
    pub detalhe_missoes: u8,
}

impl GetAllData {
    pub const BYTES: usize = 5 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        Some(Self {
            detalhe_bolsa: r.u8().ok()?,
            detalhe_equipamento: r.u8().ok()?,
            detalhe_missoes: r.u8().ok()?,
        })
    }
}

/// `SRV::C2S::CMD::service_hello` (35) — abrir diálogo com um NPC de serviço.
///
/// | Campo | Deslocamento | Tipo |
/// | :--- | ---: | :--- |
/// | `id` | 2 | `int` |
///
/// # O IR está errado aqui
///
/// `specs/protocol/gamedata_153.json` marca este comando como `payload: null` (só
/// cabeçalho) — o mesmo ponto cego do item 47 do `docs/ESTADO_E_RETOMADA.md`: o extrator
/// pula comandos de forma incomum, e este ficou sem campo nenhum por engano. O layout
/// real está no servidor 1.5.3 (`cgame/common/protocol.h`,
/// `struct service_hello { cmd_header header; int id; }`) e bate byte a byte com uma
/// captura real do 1.2.6: o mesmo alvo que o `SELECT_TARGET` anterior mandou
/// (`50 4c 00 80`) reaparece aqui, sem cabeçalho, num `SEVNPC_HELLO` de 6 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SevnpcHello {
    /// O NPC (ou jogador) com quem o cliente quer abrir diálogo.
    pub target: i32,
}

impl SevnpcHello {
    pub const BYTES: usize = 6 - BYTES_DO_CABECALHO;

    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        r.i32().ok().map(|target| Self { target })
    }
}

/// `SRV::C2S::CMD::task_notify` (49) — o cliente reporta algo ao sistema de missões.
///
/// | Campo | Deslocamento | Tipo |
/// | :--- | ---: | :--- |
/// | `size` | 2 | `unsigned int` |
/// | `buf` | 6 | `size` bytes, forma por `reason` |
///
/// # O IR está errado aqui, do mesmo jeito que no comando 35
///
/// `specs/protocol/gamedata_153.json` marca `payload: null`, mas o servidor
/// (`cgame/common/protocol.h`) declara
/// `struct task_notify { cmd_header header; unsigned int size; char buf[0]; }` — tamanho
/// variável, a classe de comando que o item 47 do `docs/ESTADO_E_RETOMADA.md` já tinha
/// identificado como ponto cego do extrator. Uma captura real confirma o prefixo: um
/// `TASK_NOTIFY` de 9 bytes trouxe `size=3` seguido de 3 bytes de `buf`.
///
/// O começo de `buf` é `task_notify_base` (`cgame/gs/task/TaskTempl.h`): `reason` (1
/// byte) e `task` (2 bytes). É o que separa os `svr_*` — recompensa escolhida, marco de
/// missão cronometrada, etc. (`TaskServer.cpp`) — mas **decodificar cada um deles é
/// trabalho do motor de missões, que ainda não existe no `pw-gs`** (contexto A do
/// roadmap salvo em memória). Por ora só extraímos `reason`/`task` para log; o resto de
/// `buf` fica cru, para o dia em que o motor existir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNotify {
    /// `task_notify_base::reason` — qual tipo de notificação de missão é esta.
    pub reason: Option<u8>,
    /// `task_notify_base::task` — o id da missão, quando o `reason` o traz.
    pub task: Option<u16>,
    /// `buf` inteiro, como veio — inclui os bytes já lidos em `reason`/`task`.
    pub buf: Vec<u8>,
}

impl TaskNotify {
    pub fn ler(payload: &[u8]) -> Option<Self> {
        let mut r = Reader::new(payload);
        let size = r.u32().ok()? as usize;
        let resto = r.rest();
        if resto.len() < size {
            return None;
        }
        let buf = resto[..size].to_vec();
        let mut br = Reader::new(&buf);
        let reason = br.u8().ok();
        let task = br.u16().ok();
        Some(Self { reason, task, buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn um_movimento_truncado_ainda_da_a_posicao() {
        // O caso do 1.2.6: só os 12 bytes do `cur_pos`. Recusar isto pararia o
        // movimento inteiro por causa de campos que talvez nem existam naquela versão.
        let mut p = Vec::new();
        p.extend_from_slice(&1.5f32.to_le_bytes());
        p.extend_from_slice(&2.5f32.to_le_bytes());
        p.extend_from_slice(&3.5f32.to_le_bytes());

        let m = PlayerMove::ler(&p).expect("12 bytes bastam para a posição");
        assert_eq!(m.cur_pos, Vec3::new(1.5, 2.5, 3.5));
        // Sem `next_pos` no fio, ele vira a própria posição — e não (0,0,0), que
        // significaria "correndo para a origem do mapa".
        assert_eq!(m.next_pos, m.cur_pos);
        assert!(!PlayerMove::completo(&p));
    }

    #[test]
    fn movimento_curto_demais_para_uma_posicao_e_recusado() {
        assert_eq!(PlayerMove::ler(&[]), None);
        assert_eq!(PlayerMove::ler(&[0u8; 11]), None);
    }

    #[test]
    fn o_logout_de_um_byte_ainda_e_lido() {
        assert_eq!(Logout::ler(&[1]).unwrap().tipo(), TipoDeSaida::SelecaoDePersonagem);
        assert_eq!(Logout::ler(&[0]).unwrap().tipo(), TipoDeSaida::SairDoJogo);
        assert_eq!(Logout::ler(&[]), None);
    }

    #[test]
    fn um_tipo_de_saida_desconhecido_tira_o_jogador_do_mundo() {
        // O lado seguro: deixar o personagem no mundo por causa de um número que não
        // reconhecemos é o que produz "personagem preso" que só um restart resolve.
        assert_eq!(
            Logout { logout_type: 77 }.tipo(),
            TipoDeSaida::Desconhecido(77)
        );
    }
}
