use crate::octets::OctetsStream;
use pw_core::{CharacterSummary, RoleId, Vector3, WorldId};
use serde::{Deserialize, Serialize};

/// S2C: Desafio inicial de conexão enviado ao cliente com chave de sessão (Opcode 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CChallenge {
    pub nonce: Vec<u8>,
    pub server_version: u32,
    pub algo: i8,
    pub edition: Vec<u8>,
    pub exp_rate: u8,
}

impl S2CChallenge {
    /// Monta o `Challenge` com o código de versão que **aquele** cliente espera.
    ///
    /// O valor era `804` fixo, que não corresponde a versão nenhuma. O cliente compara
    /// este campo com o `GAME_VERSION` compilado dentro dele e encerra a conexão antes
    /// mesmo de olhar a senha, então um número inventado aqui vira uma falha de login
    /// que não dá pista nenhuma de onde veio.
    ///
    /// O `edition` é a segunda porta do login, e o cliente reprova nele com a mesma
    /// mensagem genérica de versão errada. Ver [`crate::edition`] para a fórmula e para
    /// a origem de cada um dos quatro valores.
    pub fn new(
        nonce: Vec<u8>,
        version: crate::version::GameVersion,
        edition: crate::edition::Edition,
    ) -> Self {
        Self {
            nonce,
            server_version: version.server_version_code(),
            algo: 0,
            edition: edition.to_wire(),
            exp_rate: 1,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_octets(&self.nonce);
        stream.write_u32(self.server_version);
        stream.write_i8(self.algo);
        
        if version != "1.2.6" {
            stream.write_octets(&self.edition);
            stream.write_u8(self.exp_rate);
        }
    }
}

/// S2C: Troca de Chaves de Criptografia (Opcode 3 / KeyExchange)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CKeyExchange {
    pub nonce: Vec<u8>,
    pub blkickuser: i8,
}

impl S2CKeyExchange {
    pub fn new(nonce: Vec<u8>) -> Self {
        Self {
            nonce,
            blkickuser: 0,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_octets(&self.nonce);
        stream.write_i8(self.blkickuser);
    }
}

/// S2C: Anúncio de Login Online Aprovado (Opcode 4 / OnlineAnnounce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2COnlineAnnounce {
    pub userid: i32,
    pub localsid: u32,
    pub remain_time: i32,
    pub zoneid: i8,
    pub free_time_left: i32,
    pub free_time_end: i32,
    pub creatime: i32,
}

impl S2COnlineAnnounce {
    pub fn new(userid: i32, localsid: u32) -> Self {
        Self {
            userid,
            localsid,
            remain_time: 0,
            zoneid: 1,
            free_time_left: 0,
            free_time_end: 0,
            creatime: 0,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_i32(self.userid);
        stream.write_u32(self.localsid);
        stream.write_i32(self.remain_time);
        stream.write_i8(self.zoneid);
        stream.write_i32(self.free_time_left);
        stream.write_i32(self.free_time_end);
        stream.write_i32(self.creatime);

        if version != "1.2.6" {
            stream.write_i8(0); // referrer_flag
            stream.write_i8(0); // passwd_flag
            stream.write_i8(0); // usbbind
            stream.write_i8(0); // accountinfo_flag
        }
    }
}

/// S2C: Mensagem de Erro de Conexão ou Login (Opcode 5 / ErrorInfo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CErrorInfo {
    pub error_code: i32,
    pub info: String,
}

impl S2CErrorInfo {
    pub fn new(error_code: i32, info: &str) -> Self {
        Self {
            error_code,
            info: info.to_string(),
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.error_code);
        stream.write_string_utf8(&self.info);
    }
}

/// S2C: Anúncio de Status do Jogador para a GUI do Cliente (Opcode 6 / StatusAnnounce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CStatusAnnounce {
    pub userid: i32,
    pub localsid: u32,
    pub status: u8,
}

impl S2CStatusAnnounce {
    pub fn new(userid: i32, localsid: u32, status: u8) -> Self {
        Self {
            userid,
            localsid,
            status,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.userid);
        stream.write_u32(self.localsid);
        stream.write_u8(self.status);
    }
}

/// S2C: Lista de Personagens da Conta (Opcode 0x53 / RoleList_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CRoleListResponse {
    pub result: i32,
    pub handle: i32,
    pub userid: i32,
    pub localsid: u32,
    pub characters: Vec<CharacterSummary>,
}

impl S2CRoleListResponse {
    pub fn new(userid: i32, localsid: u32, characters: Vec<CharacterSummary>) -> Self {
        Self {
            result: 0,
            handle: -1,
            userid,
            localsid,
            characters,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.handle);
        stream.write_i32(self.userid);
        stream.write_u32(self.localsid);

        // Vetor de RoleInfo
        stream.write_compact_uint(self.characters.len() as u32);

        for c in &self.characters {
            write_role_info(stream, Some(c), version);
        }
    }
}

/// Escreve um `RoleInfo` — os 23 campos na ordem exata do IR.
///
/// Existe como função própria porque **três** protocolos carregam um `RoleInfo`
/// (`RoleList_Re`, `CreateRole_Re` e `CreateRole`), e ter o layout escrito três vezes é
/// como o `CreateRole_Re` acabou mandando só 3 campos dos 27 que devia.
///
/// Os quatro últimos campos (`referrer_role`, `cash_add`, `reincarnation_data`,
/// `realm_data`) não existem no 1.2.6 — daí o corte por versão no fim.
pub fn write_role_info(stream: &mut OctetsStream, c: Option<&CharacterSummary>, version: &str) {
    // Sem personagem (criação que falhou), o `RoleInfo` vai zerado: o protocolo não tem
    // campo opcional, e é o `result` que carrega o erro. Um único caminho de escrita
    // evita duas listas de 23 campos que precisariam ser mantidas em sincronia.
    let vazio;
    let c = match c {
        Some(c) => c,
        None => {
            vazio = CharacterSummary::vazio();
            &vazio
        }
    };
    {
        {
            stream.write_i32(c.id);
            stream.write_u8(c.gender as u8);
            stream.write_u8(c.race as u8);
            stream.write_u8(c.cls as u8);
            stream.write_i32(c.level);
            stream.write_i32(0); // level2
            stream.write_string_utf16le(&c.name);

            // Custom appearance (face/hair/body)
            let appearance_bytes = if let Some(raw_hex) = c.custom_appearance.get("raw").and_then(|v| v.as_str()) {
                hex::decode(raw_hex).unwrap_or_default()
            } else {
                serde_json::to_vec(&c.custom_appearance).unwrap_or_default()
            };
            stream.write_octets(&appearance_bytes);

            // Equipment (GRoleInventoryVector)
            stream.write_compact_uint(c.equipment.len() as u32);
            for item in &c.equipment {
                stream.write_u32(item.item_id);
                stream.write_i32(item.slot as i32);
                stream.write_i32(item.count as i32);
                stream.write_i32(item.max_count as i32);
                stream.write_octets(&[]); // data
                stream.write_i32(0);     // proctype
                stream.write_i32(0);     // expire_date
                stream.write_i32(0);     // guid1
                stream.write_i32(0);     // guid2
                stream.write_i32(0);     // mask
            }

            stream.write_i8(1); // status (1 = Active)
            stream.write_i32(0); // delete_time
            stream.write_i32(0); // create_time
            stream.write_i32(0); // lastlogin_time
            stream.write_f32(c.position.x);
            stream.write_f32(c.position.y);
            stream.write_f32(c.position.z);
            stream.write_i32(c.world_id);
            stream.write_octets(&[]); // custom_status
            stream.write_octets(&[]); // charactermode

            if version != "1.2.6" {
                stream.write_i32(0); // referrer_role
                stream.write_i32(0); // cash_add
                stream.write_octets(&[]); // reincarnation_data
                stream.write_octets(&[]); // realm_data
            }
        }
    }
}

/// S2C: Resposta da Criação de Personagem (Opcode 0x55 / CreateRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CCreateRoleResponse {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub character: Option<CharacterSummary>,
}

impl S2CCreateRoleResponse {
    /// Campos e ordem conforme `CreateRole_Re` (id 85) no IR: `result`, `roleid`,
    /// `localsid`, **um `RoleInfo` inteiro** e `refretcode`.
    ///
    /// A versão anterior parava no `localsid` e mandava 3 campos dos 27 — o
    /// `character` já estava na struct, só não era escrito. O cliente lia um pacote
    /// truncado.
    ///
    /// O protocolo não tem `RoleInfo` opcional: quando a criação falha e não há
    /// personagem, o campo vai zerado, e é o `result` que carrega o erro.
    pub fn encode(&self, stream: &mut OctetsStream, version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);

        write_role_info(stream, self.character.as_ref(), version);
        stream.write_i32(0); // refretcode
    }
}

/// S2C: Resposta da Exclusão de Personagem (Opcode 0x57 / DeleteRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CDeleteRoleResponse {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CDeleteRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// S2C: Resposta da Restauração de Personagem (Opcode 0x59 / UndoDeleteRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CUndoDeleteRoleResponse {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CUndoDeleteRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// S2C: Resposta da Seleção de Personagem (Opcode 0x47 / SelectRole_Re)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSelectRoleResponse {
    pub result: i32,
    pub auth: Vec<u8>,
}

impl S2CSelectRoleResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_octets(&self.auth);
    }
}

/// S2C: Confirmação de entrada no mundo de jogo (Opcode 0x45)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CEnterWorldResponse {
    pub role_id: RoleId,
    pub world_id: WorldId,
    pub position: Vector3,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub sp: i64,
    pub level: i32,
}

impl S2CEnterWorldResponse {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.role_id);
        stream.write_i32(self.world_id);
        stream.write_f32(self.position.x);
        stream.write_f32(self.position.y);
        stream.write_f32(self.position.z);
        stream.write_i32(self.hp);
        stream.write_i32(self.max_hp);
        stream.write_i32(self.mp);
        stream.write_i32(self.max_mp);
        stream.write_i64(self.exp);
        stream.write_i64(self.sp);
        stream.write_i32(self.level);
    }
}

/// S2C: Transmissão de movimentação de um jogador para outros ao redor (Opcode 0x21)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CPlayerMoveBroadcast {
    pub role_id: RoleId,
    pub mode: u8,
    pub position: Vector3,
    pub target: Vector3,
    pub speed: f32,
    pub timestamp: u32,
}

impl S2CPlayerMoveBroadcast {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.role_id);
        stream.write_u8(self.mode);
        stream.write_f32(self.position.x);
        stream.write_f32(self.position.y);
        stream.write_f32(self.position.z);
        stream.write_f32(self.target.x);
        stream.write_f32(self.target.y);
        stream.write_f32(self.target.z);
        stream.write_f32(self.speed);
        stream.write_u32(self.timestamp);
    }
}

/// S2C: Transmissão de mensagem de Chat (Opcode 0x71)
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Campos e ordem conforme `ChatBroadCast` (id 120) no IR.
///
/// A versão anterior escrevia um `sender_name` que **não existe no protocolo** e
/// omitia `emotion` e `data`. O cliente resolve o nome a partir do `srcroleid`; mandar
/// a string no meio do pacote deslocava tudo a partir do segundo campo.
pub struct S2CChatBroadcast {
    pub channel: u8,
    pub emotion: u8,
    pub src_role_id: RoleId,
    pub message: String,
    pub data: Vec<u8>,
}

impl S2CChatBroadcast {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_u8(self.channel);
        stream.write_u8(self.emotion);
        stream.write_i32(self.src_role_id);
        stream.write_string_utf16le(&self.message);
        stream.write_octets(&self.data);
    }
}

/// Um membro do grupo, como o cliente o espera no `TEAM_MEMBER_DATA` (64).
///
/// Os nomes e a ordem são os do `struct MEMBER` dentro de `cmd_team_member_data`, em
/// `EC_GPDataType.h`. **É de propósito que isto seja uma struct e não uma tupla**: a
/// versão anterior passava sete valores posicionais e trocava `max_hp` com `mp` sem que
/// nada reclamasse — nem o compilador, que via seis `i32` iguais, nem o teste, que lia de
/// volta no mesmo deslocamento errado em que escrevia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MembroDoGrupo {
    pub role_id: i32,
    pub level: i16,
    /// Bits de estado (montado, voando, morto...). Zero é "nada de especial".
    pub state: u8,
    /// Segundo nível — cultivo/despertar, conforme a versão.
    pub level2: u8,
    pub reencarnacoes: u8,
    /// Nível de "wallow" (fadiga anti-vício). `char` no cliente, e por isso com sinal.
    pub wallow_level: i8,
    pub hp: i32,
    pub mp: i32,
    pub max_hp: i32,
    pub max_mp: i32,
    /// Facção. Zero = sem facção.
    pub force_id: i32,
    pub profit_level: i32,
}

/// S2C: Pacote de Dados de Jogo / Mundo 3D (Opcode 0x20 / PROTOCOL_GAMEDATASEND)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGamedataSend {
    pub data: Vec<u8>,
}

impl S2CGamedataSend {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Cria o comando SELF_INFO_1 (Comando 8) que cancela o timeout OT_ENTERGAME e spawna o jogador no mundo
    pub fn self_info_1(exp: i32, sp: i32, world_id: i32, pos: Vector3, sec_level: u8) -> Self {
        let mut stream = OctetsStream::new();
        // Header do comando (u16 little-endian = 8)
        stream.write_u16_le(crate::opcodes::CMD_S2C_SELF_INFO_1);

        // cmd_self_info_1 struct no 1.2.6 (34 bytes total)
        stream.write_i32_le(exp);      // int iExp (4B)
        stream.write_i32_le(sp);       // int iSP (4B)
        stream.write_i32_le(world_id); // int cid (4B)
        stream.write_f32_le(pos.x);    // A3DVECTOR3 pos (12B)
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        stream.write_u16_le(0);        // unsigned short crc_e (2B)
        stream.write_u16_le(0);        // unsigned short crc_c (2B)
        stream.write_u8(0);            // unsigned char dir (1B)
        stream.write_u8(sec_level);    // unsigned char level2 / sec_level (1B)
        let state = if sec_level > 0 { 0x00004000 } else { 0 }; // 0x4000 = STATE_GAMEMASTER (Ícone e permissão de GM)
        stream.write_i32_le(state);    // int state (4B)

        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// `NOTIFY_HOSTPOS` (14): o jogador está em `pos` do mapa `tag`.
    ///
    /// `cmd_notify_hostpos { A3DVECTOR3 vPos; int tag; int line; }`
    /// (`EC_GPDataType.h:1362-1367`) = 2 + 20 bytes; do lado do servidor, `notify_pos`
    /// (`common/protocol.h:982-988`, `player.cpp:3530-3537`), com `key` = linha do mundo
    /// paralelo (0 fora dele). Com `tag` diferente do mapa carregado o cliente **descarrega o
    /// mundo e carrega o novo** (`OnMsgHstGoto` → `JumpToInstance`, `EC_HostMsg.cpp:1346`,
    /// `EC_GameRun.cpp:2753-2801`); com o mesmo, só reposiciona.
    ///
    /// Escrevia `pos` + um `u8` (15 bytes), que o cliente descartaria.
    pub fn notify_hostpos(pos: Vector3, tag: i32, linha: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(14);
        stream.write_f32_le(pos.x);
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        stream.write_i32_le(tag);
        stream.write_i32_le(linha);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando `WAYPOINT_LIST` (180) — a lista de pontos de teleporte que o
    /// personagem já tem liberados.
    ///
    /// Achado em 2026-09-03 lendo o source real do 1.5.5 (`F:\PW\1.5.5\EvolvedPWServer` e
    /// `EvolvedPWClient`, sem captura disponível para essa versão — instrução explícita do
    /// Murillo para ir pelos fontes desta vez): `EC_World.cpp` (`CECWorld::...`, a checagem
    /// que roda a cada quadro pra saber se os waypoints da região atual já foram avisados
    /// pro servidor) só considera um waypoint "conhecido" se ele estiver na lista que
    /// `WAYPOINT_LIST` mandou (`CECHostMsg::...`, `case WAYPOINT_LIST:
    /// m_aWayPoints.SetSize(...)`). Como nosso servidor nunca mandava esse comando, a lista
    /// do client ficava sempre vazia, TODO waypoint da região parecia "novo" em TODO quadro,
    /// e o client vivia mandando `ACTIVATE_REGION_WAYPOINTS` (C2S 178) de novo — a ~166
    /// vezes por segundo num teste real, sem nunca sair da tela "Entrando em Perfect World".
    /// Devolver aqui os mesmos IDs que o client acabou de ativar (`SetSize` troca a lista
    /// inteira, não soma — replicar exatamente o que o `player_waypoint_list` real do
    /// `gplayer_imp::SendAllData` faz) quebra o ciclo.
    pub fn player_waypoint_list(waypoints: &[u16]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(180); // CMD_S2C_WAYPOINT_LIST = 180
        stream.write_u32_le(waypoints.len() as u32); // size_t count (4B neste engine de 32 bits)
        for &wp in waypoints {
            stream.write_u16_le(wp);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ACTIVATE_WAYPOINT` (179) — **um ponto de teleporte novo**.
    ///
    /// `struct cmd_activate_waypoint { unsigned short waypoint; }` (2 bytes + cabeçalho). É o
    /// comando que o cliente trata em `CECHostPlayer::OnMsgHstWayPoint`
    /// (`EC_HostMsg.cpp:4681-4720`): soma o ponto à lista, põe no mapa e **escreve a mensagem
    /// fixa `FIXMSG_NEWWAYPOINT`** com o nome do lugar, mais o balão de dica. O
    /// `WAYPOINT_LIST` (180) não serve para isso: ele **substitui** a lista inteira, em
    /// silêncio, e é o da carga inicial.
    ///
    /// No original sai de `gplayer_imp::ActivateWaypoint` (`gs/player_imp.h:2534-2544`), que
    /// só o manda quando o ponto ainda não está na lista do jogador.
    pub fn activate_waypoint(waypoint: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(179);
        stream.write_u16_le(waypoint);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando `MALL_ITEM_PRICE` (270), resposta à consulta de preços do gshop.
    ///
    /// # O id era 197, e 197 é outro comando
    ///
    /// `197` é `REVIVAL_INQUIRE` no IR — o cliente recebia um convite de ressurreição
    /// onde devia vir uma tabela de preços. O certo é `270`.
    ///
    /// A dúvida legítima seria "e se o 1.2.6 numerasse diferente?". Não é o caso: o
    /// pedido correspondente, tratado no `gateway.rs` como `C2S 118 GET_MALL_ITEM_PRICE`,
    /// **bate exatamente** com o IR do 1.5.3. A numeração desta área do protocolo é a
    /// mesma nas duas versões, então o 197 era engano e não diferença de versão.
    pub fn mall_item_price() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(270);              // MALL_ITEM_PRICE (IR: commands.s2c)
        stream.write_i16_le(0);                // start_index = 0
        stream.write_i16_le(0);                // end_index = 0
        stream.write_i16_le(0);                // count = 0
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando SERVER_CONFIG_DATA / INST_DATA_CHECKOUT (Comando 206) para sincronizar timestamps do gshop e instâncias
    ///
    /// `gshop_ts` e `gshop_ts2` são **dois valores diferentes**, de dois arquivos
    /// diferentes (`gshop.data`/`gshop1.data`, ver `GameDataManager`) — antes desta
    /// correção (2026-09-03) o segundo campo repetia o primeiro, achado batendo um
    /// cliente 1.5.5 real contra o log ("gshop timestamp error" e "gshop1 timestamp
    /// error" mostrando o mesmo valor errado nos dois).
    pub fn inst_data_checkout(id_inst: i32, region_ts: u32, precinct_ts: u32, gshop_ts: u32, gshop_ts2: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(206);             // CMD_S2C_SERVER_CONFIG_DATA = 206
        stream.write_i32_le(id_inst);        // int idInst (1 = mundo aberto)
        stream.write_u32_le(region_ts);      // unsigned int region_time_stamp
        stream.write_u32_le(precinct_ts);    // unsigned int precinct_time_stamp
        stream.write_u32_le(gshop_ts);       // unsigned int gshop_time_stamp
        stream.write_u32_le(gshop_ts2);      // unsigned int gshop_time_stamp2 / mall_timestamp
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando PLAYER_EXT_PROP_MOVE (Comando 54) definindo as velocidades oficiais de movimento
    pub fn ext_prop_move(id_player: i32, walk_speed: f32, run_speed: f32, swim_speed: f32, flight_speed: f32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(54);            // CMD_S2C_PLAYER_EXT_PROP_MOVE = 54
        stream.write_i32_le(id_player);     // int idPlayer (4B)
        stream.write_f32_le(walk_speed);    // float walk_speed (4B) = 4.8
        stream.write_f32_le(run_speed);     // float run_speed (4B) = 4.8
        stream.write_f32_le(swim_speed);    // float swim_speed (4B) = 4.0
        stream.write_f32_le(flight_speed);  // float flight_speed (4B) = 5.0
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// `OWN_EXT_PROP` (50) — a ficha completa do **próprio** jogador: 188 bytes.
    ///
    /// # Por que ele importa mais do que parece
    ///
    /// É o **único** comando que preenche `CECHostPlayer::m_ExtProps`
    /// (`EC_HostMsg.cpp:1583`, `m_ExtProps = pCmd->prop`) — e portanto o único que dá ao
    /// cliente a força, a agilidade, a vitalidade e a energia do jogador. Sem ele os
    /// quatro ficam em zero, e `CanUseEquipment` (`EC_HostPlayer.cpp:4907-4916`) recusa
    /// qualquer equipamento que exija atributo:
    ///
    /// ```cpp
    /// if (GetMaxLevelSofar() < pEquip->GetLevelRequirement() ||
    ///     m_ExtProps.bs.strength < pEquip->GetStrengthRequirement() || ...) iReason = 2;
    /// ```
    ///
    /// O item recusado é desenhado em `A3DCOLORRGB(192, 0, 0)` — vermelho escuro
    /// (`DlgInventory.cpp:530-532`, `DlgBag.cpp:260-263`). Foi exatamente o relato em
    /// jogo de 2026-09-08: "a arma equipada no meu personagem está vermelha". A Varinha
    /// (2251) exige força 5; o cliente lia 0.
    ///
    /// O `PLAYER_EXT_PROP_BASE` (53), que já mandávamos, **não** serve: ele é roteado
    /// para o gerente dos **outros** jogadores (`EC_GameDataPrtc.cpp:1180-1186`).
    ///
    /// # O layout: 196 bytes, medido — não são nem os 188 do IR nem os 228 do fonte
    ///
    /// Este comando é o exemplo mais claro de por que a medição ganha da leitura. Três
    /// respostas plausíveis para o mesmo campo:
    ///
    /// | Fonte | Inteiros de cabeçalho | Corpo |
    /// | :--- | ---: | ---: |
    /// | IR do 1.5.3 (`S2C::cmd_own_ext_prop`) | 10 | 188 |
    /// | **binário, medido em jogo** | **12** | **196** |
    /// | fonte `EvolvedPWClient` | 20 | 228 |
    ///
    /// A primeira tentativa foi pelo IR, e o overlay do `d_rtdebug` respondeu na hora:
    /// `SERVER - Invalid GAMEDATA_50 size(Network:188, Client:196)`. A diferença de oito
    /// bytes são dois inteiros, e a ordem do fonte diz exatamente quais: logo depois de
    /// `vigour` vêm `anti_defense_degree` e `anti_resistance_degree`, e só então os quatro
    /// campos que o próprio fonte marca `// NEW` — que este binário não tem.
    ///
    /// Ou seja, o binário está **entre** as duas referências. Nem "o IR sempre vale" nem
    /// "o fonte sempre vale" resolvem; o overlay resolve. Ver `PorVersao::equip_data`.
    ///
    /// `ROLEEXTPROP` = `bs`(32) + `mv`(16) + `ak`(68) + `df`(28) + `max_ap`(4).
    #[allow(clippy::too_many_arguments)]
    pub fn own_ext_prop(
        status_point: u32,
        atributos: (i32, i32, i32, i32),
        max_hp: i32,
        max_mp: i32,
        max_ap: i32,
        regen: (i32, i32),
        velocidades: (f32, f32, f32, f32),
        ataque: (i32, i32, i32, i32, f32),
        // `damage_magic_low/high` do `ROLEEXTPROP_ATK`: é o "Atq. Mágico" da ficha, e ia
        // zero fixo até o B77 (`gs/player.cpp:4358` manda o `_cur_prop` inteiro).
        magico: (i32, i32),
        // `resistance[5]` do `ROLEEXTPROP_DEF`, na ordem metal/madeira/água/fogo/terra.
        resistencias: [i32; 5],
        defesa: (i32, i32),
    ) -> Self {
        let (vitality, energy, strength, agility) = atributos;
        let (hp_gen, mp_gen) = regen;
        let (walk, run, swim, fly) = velocidades;
        let (attack_rate, damage_low, damage_high, attack_speed, attack_range) = ataque;
        let (defense, armor) = defesa;

        let mut s = OctetsStream::new();
        s.write_u16_le(50);                 // CMD_S2C_OWN_EXT_PROP = 50
        s.write_u32_le(status_point);       // size_t status_point
        s.write_i32_le(0);                  // attack_degree
        s.write_i32_le(0);                  // defend_degree
        s.write_i32_le(0);                  // crit_rate
        s.write_i32_le(0);                  // crit_damage_bonus
        s.write_i32_le(0);                  // invisible_degree
        s.write_i32_le(0);                  // anti_invisible_degree
        s.write_i32_le(0);                  // penetration
        s.write_i32_le(0);                  // resilience
        s.write_i32_le(0);                  // vigour
        s.write_i32_le(0);                  // anti_defense_degree
        s.write_i32_le(0);                  // anti_resistance_degree

        // ROLEEXTPROP_BASE
        s.write_i32_le(vitality);
        s.write_i32_le(energy);
        s.write_i32_le(strength);
        s.write_i32_le(agility);
        s.write_i32_le(max_hp);
        s.write_i32_le(max_mp);
        s.write_i32_le(hp_gen);
        s.write_i32_le(mp_gen);

        // ROLEEXTPROP_MOVE
        s.write_f32_le(walk);
        s.write_f32_le(run);
        s.write_f32_le(swim);
        s.write_f32_le(fly);

        // ROLEEXTPROP_ATK
        s.write_i32_le(attack_rate);
        s.write_i32_le(damage_low);
        s.write_i32_le(damage_high);
        s.write_i32_le(attack_speed);
        s.write_f32_le(attack_range);
        for _ in 0..5 {
            s.write_i32_le(0);              // addon_damage[i].damage_low
            s.write_i32_le(0);              // addon_damage[i].damage_high
        }
        s.write_i32_le(magico.0);           // damage_magic_low
        s.write_i32_le(magico.1);           // damage_magic_high

        // ROLEEXTPROP_DEF
        for r in resistencias {
            s.write_i32_le(r);          // resistance[i]
        }
        s.write_i32_le(defense);
        s.write_i32_le(armor);

        s.write_i32_le(max_ap);             // tem de espelhar SELF_INFO_00 (EC_HostMsg.cpp:1319-1332)

        Self { data: s.into_bytes().to_vec() }
    }

    /// Cria o comando PLAYER_EXT_PROP_BASE (Comando 53) definindo atributos vitais base
    pub fn ext_prop_base(id_player: i32, vitality: i32, energy: i32, strength: i32, agility: i32, max_hp: i32, max_mp: i32, hp_gen: i32, mp_gen: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(53);            // CMD_S2C_PLAYER_EXT_PROP_BASE = 53
        stream.write_i32_le(id_player);     // int idPlayer (4B)
        stream.write_i32_le(vitality);      // int vitality (4B)
        stream.write_i32_le(energy);        // int energy (4B)
        stream.write_i32_le(strength);      // int strength (4B)
        stream.write_i32_le(agility);       // int agility (4B)
        stream.write_i32_le(max_hp);        // int max_hp (4B)
        stream.write_i32_le(max_mp);        // int max_mp (4B)
        stream.write_i32_le(hp_gen);        // int hp_gen (4B)
        stream.write_i32_le(mp_gen);        // int mp_gen (4B)
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando TASK_DATA (Comando 105) inicializando as listas de tarefas oficiais
    pub fn task_data() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(105);             // CMD_S2C_TASK_DATA = 105
        stream.write_u32_le(0);               // len1 = 0
        stream.write_u32_le(0);               // len2 = 0
        stream.write_u32_le(0);               // len3 = 0
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TASK_VAR_DATA (Comando 106) enviando resposta para o sistema de tarefas dinâmicas do cliente
    pub fn task_var_data(data: &[u8]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(106);              // CMD_S2C_TASK_VAR_DATA = 106
        stream.write_u32_le(data.len() as u32);
        stream.write_raw_bytes(data);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// A marca de tempo das missões dinâmicas, em resposta ao pedido do cliente
    /// (`TASK_CLT_NOTIFY_DYN_TIMEMARK`, 7).
    ///
    /// `svr_task_dyn_time_mark` (`cgame/gs/task/TaskTempl.h:1866`, dentro do `#pragma pack(1)`
    /// da linha 221), montado como `ATaskTemplMan::OnTaskGetDynTasksTimeMark`
    /// (`TaskTemplMan.cpp:299-309`) e entregue pelo `TASK_VAR_DATA` (`taskman.cpp:337`):
    ///
    /// | campo | bytes | valor |
    /// | :--- | ---: | :--- |
    /// | `reason` | 1 | `TASK_SVR_NOTIFY_DYN_TIME_MARK` = **8** |
    /// | `task` | 2 | 0 |
    /// | `time_mark` | 4 | a do `dyn_tasks.data` |
    /// | `version` | 2 | `DYN_TASK_CUR_VERSION` = 10 |
    ///
    /// O cliente só aceita com exatamente estes 9 bytes e a versão 10
    /// (`TaskClient.cpp:290-296`, `TaskTemplMan.cpp:168`). O `reason` 7, que o `gateway.rs`
    /// mandava aqui, é `TASK_SVR_NOTIFY_FORGET_SKILL`: o cliente esquecia a habilidade de
    /// produção.
    pub fn task_dyn_time_mark(marca: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(8);                    // reason = TASK_SVR_NOTIFY_DYN_TIME_MARK
        stream.write_u16_le(0);                // task
        stream.write_u32_le(marca);            // time_mark
        stream.write_u16_le(10);               // version = DYN_TASK_CUR_VERSION
        Self::task_var_data(&stream.into_bytes())
    }

    /// `QUERY_TITLE_RE` (363) — os títulos do personagem.
    ///
    /// `cmd_query_title_re` (`EC_GPDataType.h:4632-4654`, `pack(1)`), com o tamanho conferido
    /// pelo `CheckValid` dele: `roleid i32`, `titlescount i32`, `expirecount i32`, e então
    /// `titlescount` ids de 2 bytes e `expirecount` pares `{ id u16, time i32 }` (6 bytes).
    /// Sem título nenhum são os **12 bytes** do cabeçalho.
    ///
    /// Responder é o que destrava o sistema de missões do cliente: ele só liga
    /// `m_bTitleDataReady` aqui (`CECHostPlayer::InitTitle`, `EC_HostPlayer.cpp:10145-10152`),
    /// e `ATaskTemplMan::UpdateStatus` (`task/TaskTemplMan.cpp:1342-1350`) não chama
    /// `CheckAutoDelv` enquanto o dado não estiver pronto — nenhuma missão de entrega
    /// automática aparece (B60).
    pub fn query_title_re(roleid: i32, titulos: &[u16], expiram: &[(u16, i32)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(363);
        stream.write_i32_le(roleid);
        stream.write_i32_le(titulos.len() as i32);
        stream.write_i32_le(expiram.len() as i32);
        for t in titulos {
            stream.write_u16_le(*t);
        }
        for (id, tempo) in expiram {
            stream.write_u16_le(*id);
            stream.write_i32_le(*tempo);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Um pedaço do `dyn_tasks.data`, em resposta ao pedido do cliente
    /// (`TASK_CLT_NOTIFY_DYN_DATA`, 8).
    ///
    /// `ATaskTemplMan::OnTaskGetDynTasksData` (`task/TaskTemplMan.cpp:321-353`) manda o
    /// arquivo em pedaços de `0x1000 - sizeof(task_notify_base)` = **4093** bytes, cada um
    /// com o cabeçalho `task_notify_base` (`reason u8`, `task u16`, `pack(1)`):
    ///
    /// | campo | bytes | valor |
    /// | :--- | ---: | :--- |
    /// | `reason` | 1 | `TASK_SVR_NOTIFY_DYN_DATA` = **9** |
    /// | `task` | 2 | **1** no último pedaço, 0 nos outros |
    /// | dados | n | o trecho do arquivo |
    ///
    /// O cliente junta os pedaços, e no último (`task == 1`) desempacota, grava o pacote
    /// local e **só então** monta a lista de missões ativas (`OnDynTasksData`,
    /// `TaskTemplMan.cpp:181-230`). Sem isso ele fica sem inicializar o sistema de missões:
    /// nenhuma missão nova aparece, nem pelo "Procurar Missão" (B59).
    pub fn task_dyn_data(pedaco: &[u8], ultimo: bool) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(9);                            // reason = TASK_SVR_NOTIFY_DYN_DATA
        stream.write_u16_le(u16::from(ultimo));        // task: 1 = acabou
        stream.write_raw_bytes(pedaco);
        Self::task_var_data(&stream.into_bytes())
    }

    /// O tamanho de cada pedaço de [`Self::task_dyn_data`]: `0x1000 - sizeof(task_notify_base)`.
    pub const PEDACO_DAS_MISSOES_DINAMICAS: usize = 0x1000 - 3;

    /// `svr_new_task` (`TASK_SVR_NOTIFY_NEW` = 1, `task/TaskTempl.h:1793-1827`, `pack(1)`):
    /// `reason u8`, `task u16`, `cur_time u32`, `cap_task u32` e o `task_sub_tags` já
    /// serializado (`sub_task u16`, `sz u8`, `tags[sz]` — `get_size() = sz + 3`).
    ///
    /// O cliente só aceita com o tamanho exato (`valid_size`) e **refaz** a entrega na lista
    /// dele a partir destes campos (`ATaskTempl::OnServerNotify`, `TaskProcess.cpp:2705`).
    /// Entregue pelo `TASK_VAR_DATA` (`PlayerTaskInterface::NotifyClient`, `taskman.cpp:335`).
    pub fn task_notify_new(task_id: u16, cur_time: u32, cap_task: u32, sub_tags: &[u8]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(1);
        stream.write_u16_le(task_id);
        stream.write_u32_le(cur_time);
        stream.write_u32_le(cap_task);
        stream.write_raw_bytes(sub_tags);
        Self::task_var_data(&stream.into_bytes())
    }

    /// `svr_task_complete` (`TASK_SVR_NOTIFY_COMPLETE` = 2, `TaskTempl.h:1829-1859`):
    /// `reason`, `task`, `cur_time u32` e o `task_sub_tags`, cujo primeiro byte é o **estado**
    /// da entrada (sucesso, falha, desistência) — o cliente o copia para a lista antes de
    /// refazer o prêmio (`TaskProcess.cpp:2764`).
    pub fn task_notify_complete(task_id: u16, cur_time: u32, sub_tags: &[u8]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(2);
        stream.write_u16_le(task_id);
        stream.write_u32_le(cur_time);
        stream.write_raw_bytes(sub_tags);
        Self::task_var_data(&stream.into_bytes())
    }

    /// `svr_monster_killed` (`TASK_SVR_NOTIFY_MONSTER_KILLED` = 4, `TaskTempl.h:1773-1779`):
    /// 17 bytes — `reason`, `task`, `monster_id u32`, `monster_num u16`, `dps i32`, `dph i32`.
    /// O cliente recusa qualquer outro tamanho (`TaskProcess.cpp:2683`).
    pub fn task_notify_monster_killed(task_id: u16, monster_id: u32, monster_num: u16, dps: i32, dph: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(4);
        stream.write_u16_le(task_id);
        stream.write_u32_le(monster_id);
        stream.write_u16_le(monster_num);
        stream.write_i32_le(dps);
        stream.write_i32_le(dph);
        Self::task_var_data(&stream.into_bytes())
    }

    /// Notificação só com `task_notify_base` (3 bytes): `GIVE_UP` (3), `FINISHED` (5) —
    /// `ATaskTempl::NotifyClient`, `TaskTempl.inl:2263-2267`.
    pub fn task_notify_base(reason: u8, task_id: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(reason);
        stream.write_u16_le(task_id);
        Self::task_var_data(&stream.into_bytes())
    }

    /// `svr_task_err_code` (`TASK_SVR_NOTIFY_ERROR_CODE` = 6): `reason`, `task`, `err_code u32`.
    pub fn task_notify_error(task_id: u16, err_code: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u8(6);
        stream.write_u16_le(task_id);
        stream.write_u32_le(err_code);
        Self::task_var_data(&stream.into_bytes())
    }

    /// `PICKUP_MONEY` (30) — `cmd_pickup_money { int iAmount; }` (`gplayer_imp::OnPickupMoney`,
    /// `player.cpp:8826`).
    pub fn pickup_money(amount: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(30);
        stream.write_i32_le(amount);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PICKUP_ITEM` (31) — `cmd_pickup_item`, 18 bytes. O cliente empilha `amount` na bolsa
    /// dele e confere se o último slot e a quantidade final batem com `slot`/`slot_amount`
    /// (`CECHostPlayer::OnMsgHstPickupItem`, `EC_HostMsg.cpp:1203-1218`).
    pub fn pickup_item(tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(31);
        stream.write_i32_le(tid);
        stream.write_i32_le(expire_date);
        stream.write_u32_le(amount);
        stream.write_u32_le(slot_amount);
        stream.write_u8(package);
        stream.write_u8(slot);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `TASK_DELIVER_ITEM` (156) — item entregue por missão; mesmo layout e mesma conferência
    /// do [`Self::pickup_item`] (`gplayer_imp::ObtainItem`, `player.cpp:8996`).
    pub fn task_deliver_item(tid: i32, expire_date: i32, amount: u32, slot_amount: u32, package: u8, slot: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(156);
        stream.write_i32_le(tid);
        stream.write_i32_le(expire_date);
        stream.write_u32_le(amount);
        stream.write_u32_le(slot_amount);
        stream.write_u8(package);
        stream.write_u8(slot);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `TASK_DELIVER_EXP` (158) — `{ int exp; int sp; }` (`ReceiveTaskExp`, `player_imp.h:2452`).
    pub fn task_deliver_exp(exp: i32, sp: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(158);
        stream.write_i32_le(exp);
        stream.write_i32_le(sp);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `TASK_DELIVER_LEVEL2` (160) — o **nível de cultivo** novo.
    ///
    /// `struct cmd_task_deliver_level2 { int id_player; int level2; }`
    /// (`Network/EC_GPDataType.h:2859-2863`), 10 bytes com o cabeçalho. Quem manda é
    /// `gplayer_imp::SetSecLevel` (`player_imp.h:2798-2804`), chamado por
    /// `PlayerTaskInterface::SetCurPeriod` (`task/taskman.cpp:251-254`) — o prêmio
    /// `m_ulNewPeriod` da missão (`Task/TaskProcess.cpp:1284`).
    ///
    /// O cliente trata em `CECPlayer::OnMsgPlayerLevel2` (`EC_Player.cpp:7464-7470`): guarda
    /// o valor, toca o efeito de tela cheia do avanço e atualiza o título taoista
    /// (`GetLevel2Name`, `EC_GameRun.cpp:3477-3499`). **Não** é nível de GM.
    pub fn task_deliver_level2(id_player: i32, level2: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(160);
        stream.write_i32_le(id_player);
        stream.write_i32_le(level2);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `TASK_DELIVER_MONEY` (159) — `{ size_t amount; size_t cur_money; }`
    /// (`PlayerTaskInterface::DeliverGold`, `taskman.cpp:84`).
    pub fn task_deliver_money(amount: u32, cur_money: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(159);
        stream.write_u32_le(amount);
        stream.write_u32_le(cur_money);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `SPEND_MONEY` (77) — `{ size_t cost; }` (`DecMoneyAmount`, `player_imp.h:3673`).
    pub fn spend_money(cost: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(77);
        stream.write_u32_le(cost);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_DROP_ITEM` (46) — `{ u8 where; u8 index; u32 count; int tid; u8 drop_type; }`,
    /// 11 bytes. Missão que tira item usa `DROP_TYPE_TASK` = 3 (`taskman.cpp:195`,
    /// `common/protocol.h:932`).
    pub fn player_drop_item(package: u8, slot: u8, count: u32, tid: i32, drop_type: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(46);
        stream.write_u8(package);
        stream.write_u8(slot);
        stream.write_u32_le(count);
        stream.write_i32_le(tid);
        stream.write_u8(drop_type);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_MOUNTING` (227) — `{ int id; int mount_id; u16 mount_color }`, 10 bytes.
    ///
    /// `gplayer_imp::ActiveMountState` (`gs/player.cpp:14279-14299`) liga o
    /// `STATE_MOUNT` e manda este comando; `DeactiveMountState` (`:14301-14319`) manda o
    /// mesmo com **zero nos dois**, que é o desmontar.
    pub fn player_mounting(player_id: i32, mount_id: i32, mount_color: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(227);
        stream.write_i32_le(player_id);
        stream.write_i32_le(mount_id);
        stream.write_u16_le(mount_color);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ELF_EXP` (283) — `{ int exp; }`, a barra de experiência do Daimon.
    ///
    /// `elf_item::InsertExp` o manda a cada ganho que **não** sobe de nível
    /// (`gs/item/item_elf.cpp:740-748`); quando sobe, o que vai é a ficha do item inteira.
    pub fn elf_exp(exp: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(283);
        stream.write_i32_le(exp);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `SET_COOLDOWN` (198) — `{ int cooldown_index; int cooldown_time; }`. O índice de
    /// habilidade é `id + COOLINGID_BEGIN` (1024, `playerwrapper.cpp:170`).
    pub fn set_cooldown(index: i32, time_ms: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(198);
        stream.write_i32_le(index);
        stream.write_i32_le(time_ms);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `MATTER_PICKUP` (152) — `{ int matter_id; int who; }`, difundido a quem vê o item
    /// (`gmatter_dispatcher::matter_pickup`, `matter.cpp:72`).
    pub fn matter_pickup(matter_id: i32, who: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(152);
        stream.write_i32_le(matter_id);
        stream.write_i32_le(who);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PURCHASE_ITEM` (72) — a compra no NPC (`gplayer_imp::PurchaseItem`,
    /// `player.cpp:8900-8932`): `cost`, `yinpiao` (0 fora de barraca), `flag` (0), a contagem e,
    /// por item, `item_id`, `expire_date`, `count`, `inv_index u16`, `booth_slot u8`. O cliente
    /// empilha cada um e confere o `inv_index` (`OnMsgHstPurchaseItems`, `EC_HostMsg.cpp:3334`).
    pub fn purchase_item(cost: u32, itens: &[(i32, i32, u32, u16)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(72);
        stream.write_u32_le(cost);
        stream.write_u32_le(0);
        stream.write_u8(0);
        stream.write_u16_le(itens.len() as u16);
        for (tid, expira, n, slot) in itens {
            stream.write_i32_le(*tid);
            stream.write_i32_le(*expira);
            stream.write_u32_le(*n);
            stream.write_u16_le(*slot);
            stream.write_u8(0);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ITEM_TO_MONEY` (73) — a venda ao NPC: `index u16`, `type`, `count`, `money`
    /// (`gplayer_dispatcher::item_to_money`, `player.cpp:4131`; `ItemToMoney`, `:13995`).
    pub fn item_to_money(index: u16, tid: i32, count: u32, money: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(73);
        stream.write_u16_le(index);
        stream.write_i32_le(tid);
        stream.write_u32_le(count);
        stream.write_u32_le(money);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ERROR_MESSAGE` (25) — `{ int iMessage; }`, com os `ERR_*` de `common/protocol.h:679`.
    pub fn error_message(code: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(25);
        stream.write_i32_le(code);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OWN_ITEM_INFO` (40) — a ficha de um item na bolsa ou no equipamento.
    ///
    /// # O bloco de dados não é enfeite
    ///
    /// `CECIvtrEquip::SetItemInfo` (`EC_IvtrEquip.cpp:176-200`) lê deste bloco, nesta
    /// ordem exata, os requisitos do item:
    ///
    /// ```text
    /// short nivel, short classes, short forca, short vitalidade, short agilidade,
    /// short energia, int durabilidade, int durabilidade_maxima, short tamanho_da_ficha,
    /// <marca do fabricante>, <ficha>, short buracos, WORD mascara, ...
    /// ```
    ///
    /// e é com eles que `CanUseEquipment` (`EC_HostPlayer.cpp:4894-4980`) decide se o
    /// jogador pode usar o que está equipado. Item recusado sai em
    /// `A3DCOLORRGB(192, 0, 0)` — vermelho.
    ///
    /// # O que estava errado até 2026-09-09
    ///
    /// Este bloco era montado por uma **tabela chumbada de quatro ids** (2097, 2867, 2258,
    /// 2250), com um genérico para todo o resto que declarava `weapon_type = 1`
    /// (`WEAPONTYPE_RANGE`). A Varinha do Sacerdote (2251) caía no genérico: o cliente
    /// concluía que era arma de munição, não achava flecha, e recusava. Foi a "arma
    /// vermelha" que sobreviveu a quatro tentativas de conserto em lugares errados.
    ///
    /// O tooltip relatado em jogo batia campo a campo com aquele genérico — alcance 3.50,
    /// nenhuma linha de força, nenhuma linha de profissão — e foi ele que fechou o
    /// diagnóstico.
    ///
    /// Agora, quando `ficha` vem preenchida, o bloco sai do `elements.data`. Quando vem
    /// `None` (item que não é equipamento, ou realm sem as tabelas), vai só o cabeçalho,
    /// sem bloco: **inventar dado aqui é pior do que não mandar nada**, porque um
    /// requisito inventado tranca o item.
    ///
    /// # E não mandar nada também tranca — para armadura
    ///
    /// Descoberto em 2026-09-09, antes de a primeira peça de armadura chegar em jogo. A
    /// arma tinha ficha; armadura e acessório iam sem bloco. Sem bloco,
    /// `CECIvtrEquip::SetItemInfo` retorna na primeira linha (`EC_IvtrEquip.cpp:178-181`)
    /// e o `m_iProfReq` fica no zero do construtor (`EC_IvtrEquip.cpp:74`) — e
    /// `CanUseEquipment` recusa `ICID_ARMOR`/`ICID_DECORATION` com máscara zero, para
    /// **todas** as classes (`EC_HostPlayer.cpp:4953-4959`). Seria a mesma peça vermelha
    /// da Varinha, pela outra ponta.
    ///
    /// `DefaultInfo()` não salva o caso: no cliente 1.5.5 ele não é chamado em lugar
    /// nenhum para armadura — o único `DefaultInfo()` do `ElementClient` está em
    /// `EC_IvtrFashion.cpp:83`.
    #[allow(clippy::too_many_arguments)]
    pub fn item_info(
        by_package: u8,
        by_slot: u8,
        item_id: i32,
        cur_endurance: i32,
        max_endurance: i32,
        count: u32,
        raw_octets: &[u8],
        ficha: Option<pw_core::FichaDoEquipamento>,
    ) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(40);              // CMD_S2C_OWN_ITEM_INFO = 40
        stream.write_u8(by_package);          // byPackage
        stream.write_u8(by_slot);             // bySlot
        stream.write_i32_le(item_id);         // type (tid)
        stream.write_i32_le(0);               // expire_date
        stream.write_i32_le(0);               // state
        stream.write_u32_le(count);           // count
        stream.write_u16_le(0);               // crc

        // Octetos gravados no banco mandam: são o item de verdade, com refino e cravos.
        if !raw_octets.is_empty() {
            stream.write_u16_le(raw_octets.len() as u16);
            stream.write_raw_bytes(raw_octets);
            return Self { data: stream.into_bytes().to_vec() };
        }

        let Some(ficha) = ficha else {
            stream.write_u16_le(0);           // sem bloco de dados
            return Self { data: stream.into_bytes().to_vec() };
        };

        // O bloco inteiro sai de `pw_core::ConteudoDeEquipamento` — o mesmo caminho dos
        // octetos gerados no drop (`generate_weapon/armor/decoration/projectile`).
        let c_bytes = pw_core::ConteudoDeEquipamento::novo(ficha, cur_endurance, max_endurance).escrever();
        stream.write_u16_le(c_bytes.len() as u16);
        stream.write_raw_bytes(&c_bytes);

        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EXG_IVTR_ITEM (Comando 44)
    pub fn exg_ivtr_item(index1: u8, index2: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(44);
        stream.write_u8(index1);
        stream.write_u8(index2);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_move_ivtr_item { unsigned char src; unsigned char dest; unsigned int count; }`.
    ///
    /// O `count` é `unsigned int` (4 bytes), e escrevíamos `u16`. O comentário anterior
    /// dizia "struct oficial { ... u16 count }" — não era: o cabeçalho do cliente diz
    /// `unsigned int`, e o comando saía 2 bytes curto, portanto descartado (item 46).
    pub fn move_ivtr_item(src: u8, dest: u8, count: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(45);
        stream.write_u8(src);                  // src (1B)
        stream.write_u8(dest);                 // dest (1B)
        stream.write_u32_le(count);            // count (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `EQUIP_DAMAGED` (68) — uma peça vestida acabou.
    ///
    /// `struct equipment_damaged { single_data_header header; unsigned char index; char reason; }`
    /// (`cgame/common/protocol.h:1598-1603`), 4 bytes: o índice do slot e o motivo — 0 é
    /// "sem durabilidade", 1 é "quebrou ao morrer". O original manda em
    /// `equipment_damaged(index, 0)` logo depois do desgaste que zerou a peça
    /// (`gs/player.cpp:9563-9567`, `gs/playercmd.cpp:6734-6738`).
    pub fn equip_damaged(index: u8, reason: i8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(68);
        stream.write_u8(index);
        stream.write_u8(reason as u8);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EXG_EQUIP_ITEM (Comando 47) com struct oficial { u8 index1, u8 index2 }
    pub fn exg_equip_item(index1: u8, index2: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(47);
        stream.write_u8(index1);
        stream.write_u8(index2);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_equip_item { unsigned char index_inv; unsigned char index_equip;
    /// unsigned int count_inv; unsigned int count_equip; }`.
    ///
    /// As duas contagens são `unsigned int`, e escrevíamos `u16` — 4 bytes curto (item 46).
    pub fn equip_item(idx_ivtr: u8, idx_equip: u8, count_ivtr: u32, count_equip: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(48);
        stream.write_u8(idx_ivtr);             // index_inv (1B)
        stream.write_u8(idx_equip);            // index_equip (1B)
        stream.write_u32_le(count_ivtr);       // count_inv (4B)
        stream.write_u32_le(count_equip);      // count_equip (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_move_equip_item { unsigned char index_inv; unsigned char index_equip;
    /// unsigned int amount; }`.
    ///
    /// O `amount` é `unsigned int`, e escrevíamos `u16` — 2 bytes curto (item 46).
    pub fn move_item_to_equip(idx_ivtr: u8, idx_eq: u8, amount: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(49);
        stream.write_u8(idx_ivtr);             // index_inv (1B)
        stream.write_u8(idx_eq);               // index_equip (1B)
        stream.write_u32_le(amount);           // amount (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando UNFREEZE_IVTR_SLOT (Comando 181) para destravar o slot após mover/trocar itens
    pub fn unfreeze_ivtr_slot(where_pack: u8, index: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(181);
        stream.write_u8(where_pack);
        stream.write_u16_le(index);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando HOST_USE_ITEM (Comando 91)
    pub fn host_use_item(by_package: u8, by_slot: u8, item_id: i32, count: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(91);
        stream.write_u8(by_package);
        stream.write_u8(by_slot);
        stream.write_i32_le(item_id);
        stream.write_u16_le(count);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_INFO_LIST (Comando 9) para instanciar NPCs e monstros visíveis no mundo (formato oficial 1.2.6 de 27B por NPC)
    pub fn npc_info_list(npcs: &[(i32, i32, (f32, f32, f32), u8)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(9);                 // CMD_S2C_NPC_INFO_LIST = 9
        stream.write_u16_le(npcs.len() as u16); // count
        for &(nid, tid, (x, y, z), dir) in npcs {
            stream.write_i32_le(nid);           // nid (4B)
            stream.write_i32_le(tid);           // tid (4B)
            stream.write_f32_le(x);             // pos.x (4B)
            stream.write_f32_le(y);             // pos.y (4B)
            stream.write_f32_le(z);             // pos.z (4B)
            stream.write_u16_le(0);             // seed (2B)
            stream.write_u8(dir);               // dir (1B)
            stream.write_u32_le(0);             // state (4B)
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_SIT_DOWN (Comando 111) para sentar / meditar
    pub fn object_sit_down(id_player: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(111);           // CMD_S2C_OBJECT_SIT_DOWN = 111
        stream.write_i32_le(id_player);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OBJECT_STAND_UP (Comando 112) para levantar da meditação
    pub fn object_stand_up(id_player: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(112);           // CMD_S2C_OBJECT_STAND_UP = 112
        stream.write_i32_le(id_player);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OBJECT_DO_EMOTE (Comando 113) para executar animações de emote
    pub fn object_do_emote(id_player: i32, emotion: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(113);           // CMD_S2C_OBJECT_DO_EMOTE = 113
        stream.write_i32_le(id_player);
        stream.write_u16_le(emotion);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// `SECURITY_PASSWD_CHECKED` (277) — a senha conferida, pode abrir.
    ///
    /// **Sem corpo**: o IR marca `payload: empty`, e o cliente
    /// (`CECHostPlayer::OnMsgPlayerPasswdChecked`) não lê byte nenhum do pacote — só usa a
    /// chegada dele para liberar a primeira abertura do guarda-roupa. Um corpo a mais aqui
    /// faria o cliente descartar o comando pelo tamanho.
    pub fn security_passwd_checked() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(277);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SKILL_DATA (Comando 90) para carregar a lista de habilidades do jogador
    pub fn skill_data(skills: &[(i16, u8, i16)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(90);             // CMD_S2C_SKILL_DATA = 90
        stream.write_u32_le(skills.len() as u32); // size_t skill_count
        for &(id_skill, level, ability) in skills {
            stream.write_i16_le(id_skill);   // short id_skill (2B)
            stream.write_u8(level);          // unsigned char level (1B)
            stream.write_i16_le(ability);     // short ability (2B)
        }
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando SKILL_DATA (Comando 90) a partir de registros do banco de dados (LearnedSkill)
    pub fn skill_data_from_records(skills: &[pw_core::LearnedSkill]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(90);             // CMD_S2C_SKILL_DATA = 90
        stream.write_u32_le(skills.len() as u32);
        for skill in skills {
            stream.write_i16_le(skill.skill_id as i16);
            stream.write_u8(skill.level);
            stream.write_i16_le(0);          // ability
        }
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando OWN_IVTR_DATA (Comando 42) dinamicamente a partir dos itens do banco de dados
    pub fn own_ivtr_from_items(by_package: u8, bag_size: u8, items: &[pw_core::ItemRecord]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(42);             // CMD_S2C_OWN_IVTR_DATA = 42
        stream.write_u8(by_package);
        stream.write_u8(bag_size);

        let mut slot_map = std::collections::HashMap::new();
        for item in items {
            slot_map.insert(item.slot as usize, item);
        }

        let mut content = OctetsStream::new();
        for s in 0..bag_size as usize {
            if let Some(item) = slot_map.get(&s) {
                content.write_i32_le(item.item_id as i32);
                content.write_i32_le(0);     // expire_date
                content.write_i32_le(item.count as i32);
            } else {
                content.write_i32_le(-1);    // -1 = slot vazio
            }
        }

        let content_bytes = content.into_bytes();
        stream.write_u32_le(content_bytes.len() as u32);
        stream.write_raw_bytes(&content_bytes);

        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// `PLAYER_ENTER_WORLD` (17) — instancia o avatar de OUTRO jogador na tela de
    /// quem recebe. Mesma struct de `PLAYER_INFO_1` (id 0):
    /// `S2C::info_player_1` (`EC_GPDataType.h:603`, `F:\PW\1.5.5\EvolvedPWClient`) —
    /// `cid, pos, crc_e, crc_c, dir, level2, state, state2`, 30 bytes. Layout do 1.5.3
    /// em diante; o 1.2.6 (sem `state2`) tem versão própria em
    /// `PorVersao::player_enter_world`, mesmo padrão de `player_info_00`/`self_info_1`
    /// (campo final que o 1.5.x acrescenta).
    ///
    /// Substitui o codificador antigo (`role_id, world_tag, pos`, 20 bytes): não batia
    /// com struct real nenhuma (`world_tag` não existe em `info_player_1`) e não tinha
    /// chamador em produção — achado em 2026-09-04 inventariando o tamanho de todo
    /// codificador S2C contra o IR (`docs/ESTADO_E_RETOMADA.md`, item 15).
    ///
    /// `crc_e`/`crc_c` (checksums de aparência/fashion) vão zerados — mesma escolha já
    /// feita em `self_info_1`, nenhum sistema de aparência customizada implementado
    /// ainda. `state` carrega só o bit de GM (`STATE_GAMEMASTER = 0x4000`), igual ao
    /// `self_info_00`/`self_info_1`.
    pub fn player_enter_world(role_id: RoleId, vista: pw_core::VistaDoJogador) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_PLAYER_ENTER_WORLD);
        Self::info_player_1(stream, role_id, vista)
    }

    /// `PLAYER_ENTER_SLICE` (12) — outro jogador **entrou no alcance de visão**, andando.
    ///
    /// Mesma struct do `PLAYER_ENTER_WORLD` (17): o IR dá `S2C::info_player_1` para os
    /// dois (`specs/protocol/gamedata_153.json`), e o cliente trata os dois no mesmo
    /// `case` (`EC_ManPlayer.cpp:336-338`). O que muda é o efeito de aparição:
    ///
    /// ```cpp
    /// int iAppearFlag = (iCmd == S2C::PLAYER_ENTER_WORLD)
    ///     ? CECElsePlayer::APPEAR_ENTERWORLD : CECElsePlayer::APPEAR_RUNINTOVIEW;
    /// ```
    ///
    /// (`EC_ManPlayer.cpp:1845`.) Ou seja: 17 é para quem **surgiu** — entrou no jogo, se
    /// teleportou; 12 é para quem **veio andando**. Usar 17 no streaming faria cada
    /// jogador que se aproximasse aparecer com o efeito de teleporte.
    ///
    /// Quem sai usa `OBJECT_LEAVE_SLICE` (13) — o cliente roteia pelo id
    /// (`ISPLAYERID`/`ISNPCID`, `EC_GameDataPrtc.cpp:891-899`), então o mesmo comando
    /// serve para jogador e para NPC.
    pub fn player_enter_slice(role_id: RoleId, vista: pw_core::VistaDoJogador) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_PLAYER_ENTER_SLICE);
        Self::info_player_1(stream, role_id, vista)
    }

    /// O corpo comum de `PLAYER_ENTER_WORLD` e `PLAYER_ENTER_SLICE`: a `info_player_1`,
    /// escrita **depois** do cabeçalho que o chamador já pôs no `stream`.
    ///
    /// O cabeçalho fica com quem chama, e não aqui, de propósito: é assim que
    /// `subcomandos_s2c_contra_o_ir` consegue ler, de cada codificador, qual id ele
    /// escreve — a rede que pega "mandei o comando errado" antes do jogo.
    fn info_player_1(mut stream: OctetsStream, role_id: RoleId, v: pw_core::VistaDoJogador) -> Self {
        stream.write_i32_le(role_id);          // int cid (4B)
        stream.write_f32_le(v.pos.x);          // A3DVECTOR3 pos (12B)
        stream.write_f32_le(v.pos.y);
        stream.write_f32_le(v.pos.z);
        // `crc_e`/`crc_c` são os carimbos de equipamento e de aparência — no original,
        // `pObject->crc` e `pObject->custom_crc` (`protocol_imp.h:197-200`). O cliente os
        // usa para decidir se o que ele guardou em cache daquele jogador ainda vale
        // (`EC_ElsePlayer.cpp:176-177`). Zero fixo, como ia até 2026-09-11, faz o cliente
        // nunca perceber uma troca de visual ou de equipamento.
        stream.write_u16_le(v.crc_equipamento); // unsigned short crc_e (2B)
        stream.write_u16_le(v.crc_aparencia);   // unsigned short crc_c (2B)
        stream.write_u8(v.dir);                // unsigned char dir (1B)
        stream.write_u8(v.cultivo);            // unsigned char level2 (1B) — o cultivo
        let state = if v.sec_level > 0 { 0x0000_4000 } else { 0 }; // STATE_GAMEMASTER
        stream.write_i32_le(state);            // int state (4B)
        // `state2`. O único bit que este servidor sabe preencher é o do sexo — e ele não
        // é enfeite: `info_player_1::GetGender()` (`EC_GPDataType.h:709-711`) lê o sexo de
        // outro jogador **daqui**, e de mais lugar nenhum no pacote.
        //
        // Nenhum dos bits que este servidor liga acrescenta bytes ao comando: os que
        // acrescentam são TITLE, REINCARNATION, REALM, FACTION_PVP, MNFACTION, VIP e
        // BODY_SIZE (`CheckValid`, `:689-703`), e nenhum deles é ligado aqui.
        let state2 = if v.feminino { pw_core::ESTADO2_MULHER } else { 0 };
        stream.write_i32_le(state2);           // int state2 (4B)
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// `PLAYER_LEAVE_WORLD` (19) — remove o avatar de outro jogador da tela de quem
    /// recebe. Struct real, `S2C::cmd_player_leave_world` (`EC_GPDataType.h:1493`): só
    /// o `id` do personagem, 4 bytes — sem divergência entre versões no IR.
    pub fn player_leave_world(role_id: RoleId) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_PLAYER_LEAVE_WORLD);
        stream.write_i32_le(role_id);
        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando SELF_INFO_00 (Comando 38) para sincronizar status de vida, mana, nível e atributos
    pub fn self_info_00(
        level: i16,
        // O **cultivo** (`Level2`), não a coroa de GM: o cliente toca o efeito de avanço
        // sempre que este campo sobe (`SetLevel2` → `CanPlayTaoistEffect`,
        // `EC_Player.cpp:7434-7454`). Mandar zero num comando qualquer e o valor certo no
        // seguinte faz o cliente anunciar um avanço que não houve (B71).
        level2: u8,
        // `State`: **1 em combate**, 0 fora. O cliente liga o modo de luta com ele —
        // `if (pCmd->State && m_bFight == false) PlayEnterBattleGfx(); m_bFight = ...`
        // (`EC_HostMsg.cpp:1334-1335`) —, e é o que troca a animação do personagem. O
        // original manda `IsCombatState() ? 1 : 0` (`gs/player.cpp:3570`); ia zero fixo
        // até o B77.
        em_combate: bool,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        exp: i32,
        sp: i32,
        // `iAP`/`iMaxAP` — a barra de **chi**. Teto zero é o normal antes de a missão
        // conceder a barra (`m_ulFuryULimit`): o cliente não desenha barra nenhuma.
        ap: i32,
        max_ap: i32,
    ) -> Self {
        let mut stream = OctetsStream::new();
        // Header do comando (u16 little-endian = 38)
        stream.write_u16_le(crate::opcodes::CMD_S2C_SELF_INFO_00);

        // struct cmd_self_info_00 (36 bytes)
        stream.write_i16_le(level);    // short sLevel (2B)
        stream.write_u8(u8::from(em_combate)); // unsigned char State (1B)
        stream.write_u8(level2);       // unsigned char Level2 — o cultivo (1B)
        stream.write_i32_le(hp);       // int iHP (4B)
        stream.write_i32_le(max_hp);   // int iMaxHP (4B)
        stream.write_i32_le(mp);       // int iMP (4B)
        stream.write_i32_le(max_mp);   // int iMaxMP (4B)
        stream.write_i32_le(exp);      // int iExp (4B)
        stream.write_i32_le(sp);       // int iSP (4B)
        stream.write_i32_le(ap);       // int iAP (4B)
        stream.write_i32_le(max_ap);   // int iMaxAP (4B)

        Self {
            data: stream.into_bytes().to_vec(),
        }
    }

    /// Cria o comando NPC_ENTER_SLICE (Comando 11) com a struct oficial exata de 27 bytes (desmontada do gs v1.2.6)
    pub fn npc_enter_slice(nid: i32, tid: i32, pos: Vector3, dir: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(11);               // CMD_S2C_NPC_ENTER_SLICE = 11 (2B)
        stream.write_i32_le(nid);              // nid (4B)
        stream.write_i32_le(tid);              // tid (4B)
        stream.write_f32_le(pos.x);            // pos.x (4B)
        stream.write_f32_le(pos.y);            // pos.y (4B)
        stream.write_f32_le(pos.z);            // pos.z (4B)
        stream.write_u16_le(0);                // seed (2B)
        stream.write_u8(dir);                  // dir (1B)
        stream.write_u32_le(0);                // state (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `CALC_NETWORK_DELAY_RE` (291) — devolve o `timestamp` que o cliente mandou.
    ///
    /// O cliente mede a própria latência mandando `CALC_NETWORK_DELAY` (C2S 128) com o
    /// relógio dele e cronometrando a volta:
    ///
    /// ```cpp
    /// if (pCmd->timestamp == l_iDelayTimeStamp && GetGameState() == GS_GAME)
    ///     m_iInGameDelay = timeGetTime() - l_iDelayTimeStamp;
    /// ```
    ///
    /// (`EC_GameRun.cpp:3152-3164`.) O valor só alimenta o indicador de ping da janela de
    /// sistema (`DlgSystem.cpp:99`) — **não** mexe em movimento nem em combate.
    ///
    /// Sem resposta o indicador fica parado e o cliente repete o pedido sem parar: 191
    /// vezes numa sessão de teste de 2026-09-11, todas caindo no ramo de "subcomando
    /// ainda não tratado".
    ///
    /// O `timestamp` tem de voltar **igual** ao que veio: o cliente compara antes de usar,
    /// e um valor nosso seria descartado em silêncio.
    pub fn calc_network_delay_re(timestamp: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_CALC_NETWORK_DELAY_RE);
        stream.write_i32_le(timestamp);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `MATTER_ENTER_WORLD` (18) — um recurso do mapa (minério, erva, tronco) entrou no
    /// campo de visão.
    ///
    /// `S2C::cmd_matter_enter_world` é só uma `info_matter` (`EC_GPDataType.h:784-794`),
    /// e o servidor original monta exatamente os mesmos campos, nesta ordem
    /// (`INFO::matter_info_1`, `cgame/common/protocol.h:86-96`, escrita em
    /// `protocol_imp.h:363-373`):
    ///
    /// ```text
    /// int mid, int tid, A3DVECTOR3 pos,
    /// unsigned char dir0, dir1, rad, state, value
    /// ```
    ///
    /// **25 bytes**, sem alinhamento: o cabeçalho do cliente está inteiro dentro de um
    /// `#pragma pack(1)` (`EC_GPDataType.h:563`), e é por isso que `sizeof` bate com a
    /// soma dos campos. Um byte a mais ou a menos e o comando é descartado em silêncio.
    ///
    /// # O que vai em cada campo, e por quê
    ///
    /// - `dir0`/`dir1` são o eixo de rotação comprimido por `a3d_CompressDir`
    ///   (`A3DVectorComp.cpp:210-236`), e `rad` é o ângulo em 1/255 de volta. `(0, 0, 0)`
    ///   devolve o eixo Y com ângulo zero — a peça em pé, sem giro
    ///   (`a3d_DecompressDir(0,0) = (0,1,0)`), que é o que se quer sem dado de rotação: o
    ///   `npcgen.data` guarda a direção da **área**, não da instância.
    /// - `state` é máscara: bit 0 = objeto de modelo dinâmico (prédio, `.ecm`/`.gfx`
    ///   carregado por caminho), bit 1 = mina de espírito de monstro
    ///   (`CECMatter::Init`, `EC_Matter.cpp:167-168`). Zero é o recurso comum, que faz o
    ///   cliente ler o modelo do `elements.data` dele — o caminho certo para minério e
    ///   erva.
    /// - `value` só é lido quando o bit 0 de `state` está ligado e o arquivo é `.gfx`
    ///   (`LoadGFXFromFile(szFile, Info.value)`); para recurso comum é ignorado.
    ///
    /// A saída não é este par: matéria sai pelo `OUT_OF_SIGHT_LIST` (34) — ver
    /// [`Self::out_of_sight_list`].
    pub fn matter_enter_world(mid: i32, tid: i32, pos: Vector3) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_MATTER_ENTER_WORLD);
        stream.write_i32_le(mid);              // int mid (4B)
        stream.write_i32_le(tid);              // int tid (4B)
        stream.write_f32_le(pos.x);            // A3DVECTOR3 pos (12B)
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        stream.write_u8(0);                    // dir0 — eixo Y comprimido
        stream.write_u8(0);                    // dir1
        stream.write_u8(0);                    // rad — sem giro
        stream.write_u8(0);                    // state — recurso comum
        stream.write_u8(0);                    // value — só usado por modelo dinâmico
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OUT_OF_SIGHT_LIST` (34) — a lista do que saiu do campo de visão.
    ///
    /// `S2C::cmd_out_of_sight_list` (`EC_GPDataType.h:1592-1596`) é `unsigned int uCount`
    /// seguido dos ids. O cliente roteia **cada id** pela família dele — jogador, NPC ou
    /// matéria (`EC_GameDataPrtc.cpp:1056-1071`).
    ///
    /// É o único caminho de saída que a matéria tem: o `OBJECT_LEAVE_SLICE` (13) só trata
    /// `ISPLAYERID` e `ISNPCID` (`:891-899`), e um id de matéria mandado por ele não faz
    /// nada — nem erro, nem efeito.
    pub fn out_of_sight_list(ids: &[i32]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(crate::opcodes::CMD_S2C_OUT_OF_SIGHT_LIST);
        stream.write_u32_le(ids.len() as u32);
        for id in ids {
            stream.write_i32_le(*id);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_ENTER_WORLD (Comando 16) com a struct oficial de 27 bytes (info_npc na v1.2.6)
    pub fn npc_enter_world(nid: i32, tid: i32, pos: Vector3, dir: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(16);               // CMD_S2C_NPC_ENTER_WORLD = 16 (2B)
        stream.write_i32_le(nid);              // nid (4B)
        stream.write_i32_le(tid);              // tid (4B)
        stream.write_f32_le(pos.x);            // pos.x (4B)
        stream.write_f32_le(pos.y);            // pos.y (4B)
        stream.write_f32_le(pos.z);            // pos.z (4B)
        stream.write_u16_le(0);                // seed (2B)
        stream.write_u8(dir);                  // dir (1B)
        stream.write_u32_le(0);                // state (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_INFO_00 (Comando 33) enviando HP e MaxHP no formato oficial 1.2.6 (12 bytes de payload)
    /// `PLAYER_INFO_00` (32) — vida, mana e alvo de **outro** jogador.
    ///
    /// ```text
    /// struct cmd_player_info_00 {
    ///     int idPlayer; short sLevel; unsigned char State; unsigned char Level2;
    ///     int iHP; int iMaxHP; int iMP; int iMaxMP; int iTargetID;
    /// }
    /// ```
    ///
    /// **Novo.** Não existia codificador nenhum para este comando, e por isso o
    /// `QUERY_PLAYER_INFO_1` (67) não tinha o que responder — o `gateway.rs` lia o pacote
    /// e devolvia sem mandar nada.
    ///
    /// Escrito a partir do IR e conferido no `EC_GPDataType.h`, e por isso **não** entra
    /// na lista de divergências: se este divergir, é bug de quem escreveu.
    #[allow(clippy::too_many_arguments)]
    pub fn player_info_00(
        player_id: i32,
        level: i16,
        level2: u8,
        // `State`: 1 em combate — é o que põe o outro jogador em postura de luta
        // na tela (`IsCombatState() ? 1 : 0`, `gs/player.cpp:3554`).
        em_combate: bool,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        alvo: i32,
    ) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(32);               // CMD_S2C_PLAYER_INFO_00 = 32
        stream.write_i32_le(player_id);        // idPlayer (4B)
        stream.write_i16_le(level);            // sLevel (2B)
        stream.write_u8(u8::from(em_combate)); // State (1B)
        stream.write_u8(level2);               // Level2 (1B)
        stream.write_i32_le(hp);               // iHP (4B)
        stream.write_i32_le(max_hp);           // iMaxHP (4B)
        stream.write_i32_le(mp);               // iMP (4B)
        stream.write_i32_le(max_mp);           // iMaxMP (4B)
        stream.write_i32_le(alvo);             // iTargetID (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_npc_info_00 { int idNPC; int iHP; int iMaxHP; int iTargetID; }`.
    ///
    /// O `iTargetID` faltava — 12 bytes onde o cliente conta 16 — e por isso **todo**
    /// `NPC_INFO_00` era descartado antes de ser lido (item 46). Era o único comando que
    /// atualizava barra de vida de monstro: o dano do combate era calculado, debitado e
    /// enviado corretamente, e nunca aparecia na tela.
    pub fn npc_info_00(nid: i32, hp: i32, max_hp: i32, alvo: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(33);               // CMD_S2C_NPC_INFO_00 = 33 (2B)
        stream.write_i32_le(nid);              // idNPC (4B)
        stream.write_i32_le(hp);               // iHP (4B)
        stream.write_i32_le(max_hp);           // iMaxHP (4B)
        stream.write_i32_le(alvo);             // iTargetID (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `EQUIP_DATA` (66) — resposta a `GET_OTHER_EQUIP` (33): o equipamento visível de
    /// outro jogador (arma/armadura no modelo). Layout do **1.2.6/1.5.3**: `crc(u16),
    /// idPlayer(i32), mask(i64), data[n](i32)` — 14 bytes de prefixo, confirmado por duas
    /// fontes independentes: o IR do 1.5.3 (`specs/protocol/gamedata_153.json`,
    /// `S2C::cmd_equip_data`, `data` no deslocamento 14) **e** a captura real do 1.2.6
    /// (`docs/MEDIDAS_DO_126.md`, comando 66, tamanhos `14×2, 18×1, 22×2, 62×1, 66×2` —
    /// exatamente `14 + n×4`). O 1.5.5 ganha um campo a mais na frente
    /// (`color_name`) — ver `PorVersao::equip_data`.
    ///
    /// # Por que `mask=0` é uma resposta válida, não um atalho escondido
    ///
    /// Sem isto implementado, o cliente nunca marca `IsEquipDataReady()` — e
    /// `CECElsePlayer` só cria o modelo 3D quando `IsBaseInfoReady() &&
    /// IsCustomDataReady() && IsEquipDataReady()` são true ao mesmo tempo
    /// (`EC_ElsePlayer.cpp:671`, `F:\PW\1.5.5\EvolvedPWClient`). Isto é a causa confirmada
    /// (lendo o fonte, não suposição) de "só aparece a caixa de colisão, o modelo nunca
    /// carrega" — o cliente pede `GetOtherEquip` (`EC_ManPlayer.cpp:358`, confirmado no
    /// log do realm) e nunca recebia resposta. `ChangeEquipments`
    /// (`EC_ElsePlayer.cpp:1671`) marca `m_bEquipReady = true` **incondicionalmente**
    /// quando `bReset` é true — que é sempre o caso para `EQUIP_DATA`
    /// (`EC_ElsePlayer.cpp:1942`) — então `mask=0` (nenhum item) desbloqueia o modelo sem
    /// exigir que o formato de item por slot esteja implementado ainda.
    ///
    /// **Limitação sabida**: com `mask=0`, o avatar aparece sem arma/armadura visível.
    /// Equipar de verdade os slots pede decodificar como cada `data[i]` empacota
    /// item/modelo/refino por slot (visto em `EC_ElsePlayer.cpp:1723-1725` só para o slot
    /// `EQUIPIVTR_GOBLIN`) — não confirmado para os demais slots, fica para quando o
    /// visual de equipamento entre jogadores for a prioridade.
    pub fn equip_data(player_id: i32, crc: u16, mask: u64, items: &[i32]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(66);                // CMD_S2C_EQUIP_DATA = 66
        stream.write_u16_le(crc);               // unsigned short crc (2B)
        stream.write_i32_le(player_id);         // int idPlayer (4B)
        stream.write_u64_le(mask);              // __int64 mask (8B)
        for item in items {
            stream.write_i32_le(*item);         // int data[n]
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OBJECT_MOVE` (15) — avisa quem está por perto que outro jogador (ou NPC/monstro)
    /// está andando pra `dest`. Struct real, `S2C::cmd_object_move`
    /// (`gamedata_155.json`, `S2C::cmd_object_move`), 21 bytes, **idêntica no 1.2.6 e no
    /// 1.5.3+** — confirmado por duas fontes independentes: o IR (`bytes: 21`) e a
    /// captura real do 1.2.6 (`docs/MEDIDAS_DO_126.md`, comando 15: `21×17294`, o
    /// comando mais frequente da sessão inteira).
    ///
    /// # Por que isto precisava existir
    ///
    /// `PLAYER_MOVE` (C2S 0) migrou pro `pw-gs` (`docs/ESTADO_E_RETOMADA.md`, seção "Os
    /// primeiros subcomandos já mudaram de lado") — o mundo atualiza a posição em
    /// memória, mas **nunca avisava mais ninguém**. O `pw-link` tinha um
    /// `InboundPacket::PlayerMove`/`OutboundPacket::PlayerMoveBroadcast` próprio (opcode
    /// GNET **33**), só que **opcode 33 não existe na tabela de protocolos GNET real**
    /// (`specs/protocol/gnet_155.json`, `protocols` — conferido, não tem `id: 33`) — e
    /// pior, `InboundPacket::PlayerMove` nunca é produzido pelo decodificador de verdade,
    /// que sempre entrega `PLAYER_MOVE` como `InboundPacket::GamedataSend` (opcode GNET
    /// 34, sempre). Ou seja, aquele caminho era morto dos dois lados — a causa raiz real
    /// de "movimento não sincroniza", achada depois de o Murillo confirmar em jogo que
    /// nem o chat (que usa um opcode GNET de verdade, separado) nem o movimento (que não
    /// usa) se comportavam igual.
    pub fn object_move(id: i32, dest: Vector3, use_time: u16, speed: i16, move_mode: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(15);                // CMD_S2C_OBJECT_MOVE = 15
        stream.write_i32_le(id);                // int id (4B)
        stream.write_f32_le(dest.x);            // A3DVECTOR3 dest (12B)
        stream.write_f32_le(dest.y);
        stream.write_f32_le(dest.z);
        stream.write_u16_le(use_time);          // unsigned short use_time (2B)
        stream.write_i16_le(speed);             // short sSpeed (2B)
        stream.write_u8(move_mode);             // unsigned char move_mode (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OBJECT_STOP_MOVE` (35) — mesma família do `OBJECT_MOVE`, pra quando o jogador
    /// para. Struct real, `S2C::cmd_object_stop_move`, 20 bytes, também idêntica no
    /// 1.2.6 e no 1.5.3+ (`docs/MEDIDAS_DO_126.md`, comando 35: `20×2986, igual ao
    /// 1.5.3`). Mesma causa raiz do `object_move` acima — `STOP_MOVE` (C2S 7) já migrou
    /// pro `pw-gs`, mas nunca avisava ninguém.
    pub fn object_stop_move(id: i32, dest: Vector3, speed: i16, dir: u8, move_mode: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(35);                // CMD_S2C_OBJECT_STOP_MOVE = 35
        stream.write_i32_le(id);                // int id (4B)
        stream.write_f32_le(dest.x);            // A3DVECTOR3 dest (12B)
        stream.write_f32_le(dest.y);
        stream.write_f32_le(dest.z);
        stream.write_i16_le(speed);             // short sSpeed (2B)
        stream.write_u8(dir);                   // unsigned char dir (1B)
        stream.write_u8(move_mode);             // unsigned char move_mode (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando UNSELECT (Comando 39) desmarcando o alvo atual
    pub fn unselect() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(39);               // CMD_S2C_UNSELECT = 39
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_CAST_SKILL (Comando 85) disparando a animação e barra de conjuração da magia
    pub fn object_cast_skill(caster: i32, target: i32, skill_id: i32, cast_time_ms: u16, skill_level: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(85);               // CMD_S2C_OBJECT_CAST_SKILL = 85
        stream.write_i32_le(caster);
        stream.write_i32_le(target);
        stream.write_i32_le(skill_id);
        stream.write_u16_le(cast_time_ms);
        stream.write_u8(skill_level);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SKILL_PERFORM (Comando 88) liberando o jogador do estado de conjuração
    pub fn skill_perform() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(88);               // CMD_S2C_SKILL_PERFORM = 88
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_host_skill_attack_result { int idTarget; int idSkill; int iDamage;
    /// int attack_flag; unsigned char attack_speed; byte section; }`.
    ///
    /// Duas diferenças, 4 bytes ao todo: o `attack_flag` é `int` e escrevíamos `i8`, e
    /// faltava o `section`. Comando descartado pelo cliente (item 46).
    ///
    /// O comentário anterior dizia "no formato oficial 1.2.6". Não havia nada por trás
    /// dessa afirmação: nenhum cabeçalho do 1.2.6 está entre as fontes do projeto.
    pub fn self_skill_attack_result(
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(142);              // CMD_S2C_SELF_SKILL_ATTACK_RESULT = 142
        stream.write_i32_le(target_id);        // idTarget (4B)
        stream.write_i32_le(skill_id);         // idSkill (4B)
        stream.write_i32_le(damage);           // iDamage (4B)
        stream.write_i32_le(attack_flag);      // attack_flag (4B)
        stream.write_u8(speed);                // attack_speed (1B)
        stream.write_u8(section);              // section (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_SKILL_ATTACKED` (144) — "uma habilidade acertou **você**".
    ///
    /// É o par do 142 do outro lado: o 142 diz a quem conjurou quanto ele fez, e o 144
    /// diz a quem levou quem foi e quanto doeu. Sem ele o alvo não toca o efeito visual
    /// nem entra em estado de combate — `CECHostPlayer::OnMsgHstSkillAttacked`
    /// (`EC_HostMsg.cpp:1023-1068`) vira o atacante de frente, chama `PlayAttackEffect` e
    /// `EnterFightState`.
    ///
    /// `struct cmd_host_skill_attacked { int idAttacker; int idSkill; int iDamage;
    /// char cEquipment; int attack_flag; char speed; unsigned char section; }` — 19 bytes
    /// sob `#pragma pack(1)` (`EC_GPDataType.h:2763`).
    ///
    /// O `cEquipment` diz qual peça de armadura se desgastou com o golpe; `0x7f` é o
    /// valor que o cliente lê como "nenhuma" (`(pCmd->cEquipment & 0x7f) != 0x7f` é a
    /// condição para gastar durabilidade). Mandamos `0x7f` porque desgaste de equipamento
    /// ainda não existe no servidor — e um valor qualquer ali comeria a durabilidade de
    /// uma peça a cada golpe.
    pub fn host_skill_attacked(
        attacker_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> Self {
        /// O `cEquipment` que o cliente lê como "nenhuma peça se desgastou".
        const SEM_DESGASTE: u8 = 0x7f;

        let mut stream = OctetsStream::new();
        stream.write_u16_le(144);              // CMD_S2C_HOST_SKILL_ATTACKED = 144
        stream.write_i32_le(attacker_id);      // idAttacker (4B)
        stream.write_i32_le(skill_id);         // idSkill (4B)
        stream.write_i32_le(damage);           // iDamage (4B)
        stream.write_u8(SEM_DESGASTE);         // cEquipment (1B)
        stream.write_i32_le(attack_flag);      // attack_flag (4B)
        stream.write_u8(speed);                // speed (1B)
        stream.write_u8(section);              // section (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OBJECT_LEAVE_SLICE` (13) — a entidade saiu do alcance; o cliente pode largá-la.
    ///
    /// `struct cmd_leave_slice { int id; }` — 4 bytes (`EC_GPDataType.h:1356`). O cliente
    /// roteia pelo id: `ISNPCID` manda para o gerente de NPCs como "saiu correndo",
    /// `ISPLAYERID` para o de jogadores (`EC_GameDataPrtc.cpp:891-899`).
    ///
    /// É o par do `NPC_ENTER_SLICE` (11). Sem ele, tudo o que o servidor manda uma vez
    /// fica na memória do cliente para sempre — e, pior, o cliente descarta sozinho o que
    /// sai do raio ativo dele, então o servidor perde a conta do que o outro lado tem.
    pub fn object_leave_slice(id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(13);               // CMD_S2C_OBJECT_LEAVE_SLICE = 13
        stream.write_i32_le(id);               // int id (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_HP_STEAL` (279) — o número **verde** de vida recuperada.
    ///
    /// `struct cmd_player_hp_steal { int hp; }` — 4 bytes, conferido no
    /// `EC_GPDataType.h:3794` e no IR (`S2C::cmd_player_hp_steal`).
    ///
    /// É o comando da cura, e não o `HOST_SKILL_ATTACK_RESULT` (142). O 142 termina em
    /// `CECPlayer::Damaged`, que só sabe desenhar `BUBBLE_DAMAGE` (vermelho) ou "errou"
    /// (`EC_Player.cpp:3459-3489`); o 279 vira `BubbleText(BUBBLE_ADD, hp)`
    /// (`EC_HostMsg.cpp:5772-5781`), que é o número verde. Em jogo, 2026-09-08, a Prece da
    /// Clareza curava certo no servidor e aparecia como dano na tela por causa disso.
    ///
    /// Vai para **quem recebeu** a cura — é sobre a vida dele, e é ele que vê o número.
    pub fn player_hp_steal(hp: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(279);              // CMD_S2C_PLAYER_HP_STEAL = 279
        stream.write_i32_le(hp);               // int hp (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_CORRECT_POS` (177) — põe o jogador numa posição, à força.
    ///
    /// `struct cmd_host_correct_pos { A3DVECTOR3 pos; unsigned short stamp; }` — 14 bytes
    /// (`EC_GPDataType.h:3099`). O `stamp` é o contador de correções que o cliente usa
    /// para descartar correção velha que chegue fora de ordem; começamos em 0 e subimos a
    /// cada teleporte do mesmo jogador.
    ///
    /// É com este comando que o servidor responde ao `GOTO` (C2S 19), o Ctrl+clique de GM.
    pub fn host_correct_pos(pos: Vector3, stamp: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(177);              // CMD_S2C_HOST_CORRECT_POS = 177
        stream.write_f32_le(pos.x);            // A3DVECTOR3 pos (12B)
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        stream.write_u16_le(stamp);            // unsigned short stamp (2B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OBJECT_SKILL_ATTACK_RESULT` (143) — dano de habilidade entre duas entidades
    /// que não são o próprio jogador (ex.: um pet, ou o alvo de outro jogador visto de
    /// fora). Mesma família de `SELF_SKILL_ATTACK_RESULT` (142) — `attack_flag` de 4
    /// bytes e o campo `section` no fim, confirmados pelo IR (`S2C::
    /// cmd_object_skill_attack_result`, 22 bytes, idêntico em 1.5.3 e 1.5.5).
    ///
    /// Escrevia `attacker_id, target_id, skill_id, damage, speed, attack_flag` (18
    /// bytes: `attack_flag` de 1 byte, na ordem errada, e sem `section`) — a mesma
    /// omissão de `self_skill_attack_result` antes do item 54 fixar aquele, só que
    /// este nunca teve chamador em produção pra expor o bug. O 1.2.6 (sem `section`,
    /// `attack_flag` em 1 byte) tem sua versão em
    /// `PorVersao::object_skill_attack_result` — achado em 2026-09-04 no mesmo
    /// inventário que achou `player_enter_world` (`docs/ESTADO_E_RETOMADA.md`, item 16).
    pub fn object_skill_attack_result(attacker_id: i32, target_id: i32, skill_id: i32, damage: i32, attack_flag: i32, speed: u8, section: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(143);              // CMD_S2C_OBJECT_SKILL_ATTACK_RESULT = 143
        stream.write_i32_le(attacker_id);      // attacker_id (4B)
        stream.write_i32_le(target_id);        // target_id (4B)
        stream.write_i32_le(skill_id);         // skill_id (4B)
        stream.write_i32_le(damage);           // damage (4B)
        stream.write_i32_le(attack_flag);      // attack_flag (4B)
        stream.write_u8(speed);                // speed (1B)
        stream.write_u8(section);              // section (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SELF_STOP_SKILL (Comando 123) finalizando a execução de habilidade
    pub fn self_stop_skill() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(123);              // CMD_S2C_SELF_STOP_SKILL = 123
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SKILL_INTERRUPTED (Comando 86) avisando que o conjurador parou
    pub fn skill_interrupted(caster: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(86);
        stream.write_i32_le(caster);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SELF_SKILL_INTERRUPTED (Comando 87) cancelando a barra de conjuração
    pub fn self_skill_interrupted(reason: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(87);
        stream.write_u8(reason);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SCENE_SERVICE_NPC_LIST (Comando 390) com a lista de NPCs prestadores de serviço
    pub fn scene_service_npc_list(npcs: &[(i32, i32)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(390);
        stream.write_u32_le(npcs.len() as u32);
        for &(tid, nid) in npcs {
            stream.write_i32_le(tid);
            stream.write_i32_le(nid);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SELECT_TARGET (Comando 52)
    pub fn select_target(target_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(52);               // CMD_S2C_SELECT_TARGET = 52
        stream.write_i32_le(target_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_GREETING (Comando 70) para abrir diálogo com NPC
    pub fn npc_greeting(nid: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(70);               // CMD_S2C_NPC_GREETING = 70
        stream.write_i32_le(nid);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_DISAPPEAR (Comando 21) para remover objeto que saiu de vista ou morreu
    pub fn object_disappear(id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(21);               // CMD_S2C_OBJECT_DISAPPEAR = 21
        stream.write_i32_le(id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando HOST_ATTACKRESULT (Comando 24) retornando dano infligido
    /// `struct cmd_host_attack_result { int idTarget; int iDamage; int attack_flag;
    /// unsigned char attack_speed; }`.
    ///
    /// Faltava o `attack_flag` (`int`, 4 bytes), e o `u8` final era chamado de `hit_type`
    /// quando o cliente o lê como `attack_speed`. Comando descartado (item 46) — é o que
    /// mostra o número de dano ao acertar.
    ///
    /// O `attack_flag` marca crítico e os símbolos de ataque/defesa; a velocidade de
    /// ataque é outra coisa. Trocar um pelo outro fazia "crítico" virar "velocidade 1".
    pub fn host_attack_result(target_id: i32, damage: i32, attack_flag: i32, speed: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(24);               // CMD_S2C_HOST_ATTACKRESULT = 24
        stream.write_i32_le(target_id);        // idTarget (4B)
        stream.write_i32_le(damage);           // iDamage (4B)
        stream.write_i32_le(attack_flag);      // attack_flag (4B)
        stream.write_u8(speed);                // attack_speed (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_ATTACKED` (26) — o jogador **levou** um golpe.
    ///
    /// Sem isto, um monstro batendo no jogador é invisível: o servidor já debitava o HP
    /// no tick, e o cliente nunca ficava sabendo. O jogador via a vida cheia até morrer
    /// do nada.
    ///
    /// # Layout escrito a partir do IR
    ///
    /// `S2C::cmd_host_attacked`, 14 bytes:
    ///
    /// | Campo | Deslocamento | Tipo |
    /// | :--- | ---: | :--- |
    /// | `idAttacker` | 0 | `int` |
    /// | `iDamage` | 4 | `int` |
    /// | `cEquipment` | 8 | `char` |
    /// | `attack_flag` | 9 | `int` |
    /// | `speed` | 13 | `char` |
    ///
    /// Diferente da maioria dos vizinhos deste arquivo, este codificador **não** vem de
    /// engenharia reversa do 1.2.6: não havia nada aqui para preservar, então ele segue o
    /// IR do 1.5.3, que é a única informação verificada que temos. Se o 1.2.6 divergir,
    /// vai aparecer como campo deslocado no cliente — e o conserto será com uma captura
    /// na mão, não com um palpite.
    /// `equipamento` é o **índice da peça que sofreu desgaste**, e `0x7f` quer dizer
    /// "nenhuma": o cliente só gasta durabilidade quando `(cEquipment & 0x7f) != 0x7f`
    /// (`EC_HostMsg.cpp:968-976`). Mandando zero, todo golpe de monstro gastava a arma.
    ///
    /// `speed` é o `attack.speed` do original, que para monstro é o `_damage_delay` do
    /// `MONSTER_ESSENCE` em tiques de 50 ms (`npc.cpp:2118`); o cliente o usa como duração
    /// da animação do golpe (`CECNPC::OnMsgAttackHostResult` → `PlayAttackEffect`,
    /// `EC_NPC.cpp:2043-2064`). (B60.)
    pub fn host_attacked(attacker_id: i32, damage: i32, equipamento: u8, attack_flag: i32, speed: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(26);
        stream.write_i32_le(attacker_id);
        stream.write_i32_le(damage);
        stream.write_u8(equipamento);
        stream.write_i32_le(attack_flag); // 0 normal, 1 crítico
        stream.write_u8(speed);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_DIED` (28) — **o próprio jogador** morreu.
    ///
    /// `S2C::cmd_host_died`, 16 bytes: `idKiller` (int) e `pos` (A3DVECTOR). Ver a nota
    /// de procedência em [`Self::host_attacked`].
    pub fn host_died(killer_id: i32, pos: Vector3) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(28);
        stream.write_i32_le(killer_id);
        stream.write_f32_le(pos.x);
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_REVIVE` (29) — o jogador voltou a viver, e onde.
    ///
    /// `S2C::cmd_player_revive`, 18 bytes: `idPlayer` (int), `sReviveType` (short) e
    /// `pos` (A3DVECTOR). Ver a nota de procedência em [`Self::host_attacked`].
    pub fn player_revive(role_id: i32, revive_type: i16, pos: Vector3) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(29);
        stream.write_i32_le(role_id);
        stream.write_i16_le(revive_type);
        stream.write_f32_le(pos.x);
        stream.write_f32_le(pos.y);
        stream.write_f32_le(pos.z);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando NPC_DIED (Comando 20) informando a morte do monstro
    pub fn npc_died(nid: i32, killer_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(20);               // CMD_S2C_NPC_DIED = 20
        stream.write_i32_le(nid);              // nid (4B)
        stream.write_i32_le(killer_id);        // killer_id (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando RECEIVE_EXP (Comando 36) entregando EXP e Alma
    pub fn receive_exp(exp: i32, sp: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(36);               // CMD_S2C_RECEIVE_EXP = 36
        stream.write_i32_le(exp);              // exp (4B)
        stream.write_i32_le(sp);               // sp (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ADD_STATUS_POINT` (51): os pontos aplicados a cada atributo e os que sobraram.
    ///
    /// `gplayer_dispatcher::set_status_point` (`player.cpp:4367`) com
    /// `S2C::CMD::set_status_point` (`common/protocol.h:1427-1435`): cinco `size_t` —
    /// 2 + 20 = **22 bytes**. O cliente soma os quatro aos atributos que já mostra, troca os
    /// pontos livres por `remain` e pede o `GET_EXT_PROP` (`OnMsgHstAddStatusPt`,
    /// `EC_HostMsg.cpp:1610-1625`). Recusa vai com os quatro em zero.
    pub fn add_status_point(vit: u32, eng: u32, str_: u32, agi: u32, restantes: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(51);
        stream.write_u32_le(vit);
        stream.write_u32_le(eng);
        stream.write_u32_le(str_);
        stream.write_u32_le(agi);
        stream.write_u32_le(restantes);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando LEVEL_UP (Comando 37) tocando a animação de subir de nível
    pub fn level_up(role_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(37);               // CMD_S2C_LEVEL_UP = 37
        stream.write_i32_le(role_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando SEVNPC_HELLO_RE (Comando 70) abrindo a janela de diálogo com o NPC
    pub fn sevnpc_hello_re(nid: i32, _talk_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(70);               // CMD_S2C_NPC_GREETING = 70
        stream.write_i32_le(nid);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando OBJECT_STARTATTACK (Comando 22) iniciando a animação de ataque
    pub fn object_start_attack(attacker_id: i32, target_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(22);               // CMD_S2C_OBJECT_STARTATTACK = 22
        stream.write_i32_le(attacker_id);
        stream.write_i32_le(target_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_GATHER_START` (126): `pid` começou a colher `mid` por `segundos`.
    ///
    /// `player_gather_start { int pid; int mid; unsigned char use_time; }`
    /// (`common/protocol.h:1988-1994`; `cmd_player_gather_start`, `EC_GPDataType.h:2628`) —
    /// 2 + 9 bytes, difundido a quem vê o jogador e a ele (`gather_start`, `player.cpp:4614`).
    pub fn player_gather_start(pid: i32, mid: i32, segundos: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(126);
        stream.write_i32_le(pid);
        stream.write_i32_le(mid);
        stream.write_u8(segundos);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_GATHER_STOP` (127): `{ int pid; }` — 6 bytes (`protocol.h:1996-2000`).
    pub fn player_gather_stop(pid: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(127);
        stream.write_i32_le(pid);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_OBTAIN_ITEM` (99): item que entrou na bolsa sem vir do chão nem de loja.
    ///
    /// `cmd_host_obtain_item { int type; int expire_date; unsigned int amount; unsigned int
    /// slot_amount; unsigned char where; unsigned char index; }` (`EC_GPDataType.h:2288-2296`)
    /// = 2 + 18 bytes; `obtain_item` (`player.cpp:4166-4173`). Como no `PICKUP_ITEM`, o
    /// cliente empilha sozinho e confere slot e quantidade final.
    pub fn obtain_item(tid: i32, validade: i32, quantidade: u32, no_slot: u32, pacote: u8, slot: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(99);
        stream.write_i32_le(tid);
        stream.write_i32_le(validade);
        stream.write_u32_le(quantidade);
        stream.write_u32_le(no_slot);
        stream.write_u8(pacote);
        stream.write_u8(slot);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_START_ATTACK` (84): começou a sessão de golpe normal.
    ///
    /// `cmd_host_start_attack { int idTarget; unsigned short ammo_remain; unsigned char
    /// attack_speed; }` (`EC_GPDataType.h:2190-2195`) = 2 + 7 bytes; `start_attack`
    /// (`player.cpp:3301-3319`), só ao próprio. O cliente acerta a munição mostrada e abre o
    /// trabalho de golpe (`CECHPWorkMelee`, `EC_HostMsg.cpp:3240-3262`) — é a animação.
    pub fn host_start_attack(alvo: i32, municao: u16, velocidade_em_ticks: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(84);
        stream.write_i32_le(alvo);
        stream.write_u16_le(municao);
        stream.write_u8(velocidade_em_ticks);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_STOPATTACK` (23): a sessão de golpe acabou.
    ///
    /// `cmd_host_stop_attack { int iReason; }` (`EC_GPDataType.h:1509`) = 2 + 4 bytes;
    /// `stop_attack` (`player.cpp:3471`) com os bits de `CheckAttack`
    /// (`actobject.cpp:1254-1292`): 1 não pode atacar, 2 alvo inválido, 4 fora de alcance.
    pub fn host_stop_attack(motivo: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(23);
        stream.write_i32_le(motivo);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `UPDATE_EXT_STATE` (124): os seis `DWORD` de estado visível de um objeto (atordoado,
    /// lento, abençoado...). `cmd_update_ext_state { int id; DWORD states[6]; }`
    /// (`EC_GPDataType.h:2520-2524`, `OBJECT_EXT_STATE_COUNT = 6` na `:539`), 28 bytes; o
    /// servidor manda de `gactive_imp::UpdateVisibleState` (`actobject.cpp:1531-1590`).
    pub fn update_ext_state(id: i32, estados: [u32; 6]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(124);
        stream.write_i32_le(id);
        for s in estados {
            stream.write_u32_le(s);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ICON_STATE_NOTIFY` (125): os ícones de estado de um objeto, com parâmetros.
    ///
    /// Tamanho variável, lido por `cmd_icon_state_notify::Initialize`
    /// (`EC_GPDataType.h:2538-2622`): `int id; u16 scount; u16 state[scount]; u16 pcount;
    /// int param[pcount]`. Os 2 bits altos de cada `state` dizem quantos parâmetros ele
    /// consome (`(s >> 14) & 3`) — o servidor escreve assim em `InsertTeamVisibleState`
    /// (`actobject.h:1799-1818`) e manda em `object_state_notify` (`player.cpp:11356-11372`).
    /// Aqui cada ícone leva um parâmetro, o tempo restante em segundos (`_timeout`).
    pub fn icon_state_notify(id: i32, icones: &[(u16, i32)]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(125);
        stream.write_i32_le(id);
        stream.write_u16_le(icones.len() as u16);
        for (estado, _) in icones {
            stream.write_u16_le((estado & 0x3FFF) | (1 << 14));
        }
        stream.write_u16_le(icones.len() as u16);
        for (_, parametro) in icones {
            stream.write_i32_le(*parametro);
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ENCHANT_RESULT` (139): efeito de bênção/maldição aplicado a um alvo.
    /// `cmd_enchant_result { int caster; int target; int skill; char level;
    /// char orange_name; int attack_flag; byte section; }` (`EC_GPDataType.h:2714-2723`),
    /// 19 bytes; o servidor manda de `SendClientEnchantResult` (`skillwrapper.cpp:480-484`).
    pub fn enchant_result(caster: i32, target: i32, skill: i32, level: u8, orange_name: bool, attack_flag: i32, section: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(139);
        stream.write_i32_le(caster);
        stream.write_i32_le(target);
        stream.write_i32_le(skill);
        stream.write_u8(level);
        stream.write_u8(orange_name as u8);
        stream.write_i32_le(attack_flag);
        stream.write_u8(section);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `ATTACK_ONCE` (83): um golpe normal saiu, e quantas munições ele gastou.
    ///
    /// `gplayer_dispatcher::attack_once` (`player.cpp:3321-3327`), mandado a cada golpe por
    /// `FillAttackMsg` (`:3134`) com `object_attack_once { unsigned char arrow_dec; }`
    /// (`common/protocol.h:1695-1699`) — 3 bytes. O cliente tira `ammo_num` do slot de
    /// munição quando a arma é de longo alcance e gasta durabilidade da arma
    /// (`OnMsgHstAttackOnce`, `EC_HostMsg.cpp:4204-4233`).
    pub fn attack_once(municao_gasta: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(83);
        stream.write_u8(municao_gasta);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando REPAIR_ALL (Comando 74) informando o reparo completo de itens equipados
    pub fn repair_all(cost: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(74);               // CMD_S2C_REPAIR_ALL = 74
        stream.write_i32_le(cost);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando REPAIR (Comando 75) reparando um item individual
    pub fn repair(slot: u8, cost: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(75);               // CMD_S2C_REPAIR = 75
        stream.write_u8(slot);
        stream.write_i32_le(cost);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando LEARN_SKILL (Comando 95) confirmando o aprendizado no Mestre de Habilidades
    pub fn learn_skill(skill_id: i32, level: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(95);               // CMD_S2C_LEARN_SKILL = 95
        stream.write_i32_le(skill_id);
        stream.write_i32_le(level);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando COST_SKILL_POINT (Comando 94) deduzindo SP/Alma no aprendizado
    pub fn cost_skill_point(sp_cost: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(94);               // CMD_S2C_COST_SKILL_POINT = 94
        stream.write_i32_le(sp_cost);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando PRODUCE_START (Comando 100) iniciando a forja com barra de progresso
    pub fn produce_start(recipe_id: i32, time_ms: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(100);              // CMD_S2C_PRODUCE_START = 100
        stream.write_i32_le(recipe_id);
        stream.write_u16_le(time_ms);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando PRODUCE_ONCE (Comando 101) gerando o item forjado
    pub fn produce_once(item_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(101);              // CMD_S2C_PRODUCE_ONCE = 101
        stream.write_i32_le(item_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando PRODUCE_END (Comando 102) finalizando a forja
    pub fn produce_end() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(102);              // CMD_S2C_PRODUCE_END = 102
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando DECOMPOSE_START (Comando 103) desmontando item
    pub fn decompose_start(item_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(103);              // CMD_S2C_DECOMPOSE_START = 103
        stream.write_i32_le(item_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando DECOMPOSE_END (Comando 104) finalizando a decomposição
    pub fn decompose_end() -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(104);              // CMD_S2C_DECOMPOSE_END = 104
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando EMBED_ITEM (Comando 92) fundindo Pedra de Alma no equipamento
    pub fn embed_item(equip_slot: u8, stone_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(92);               // CMD_S2C_EMBED_ITEM = 92
        stream.write_u8(equip_slot);
        stream.write_i32_le(stone_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando CLEAR_TESSERA (Comando 93) limpando pedras de alma
    pub fn clear_tessera(equip_slot: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(93);               // CMD_S2C_CLEAR_EMBEDDED_CHIP = 93
        stream.write_u8(equip_slot);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OBJECT_TAKEOFF` (96) — o jogador decolou.
    ///
    /// `struct cmd_object_takeoff { int object_id; }`, 4 bytes (`EC_GPDataType.h:2271`).
    /// É este comando que faz o dono da tela voar: `CECHostPlayer::OnMsgPlayerFly` liga
    /// `GP_STATE_FLY` e começa o trabalho de voo (`EC_HostMsg.cpp:5936-5960`). Vai também
    /// para quem está por perto, que é como eles veem as asas abertas.
    ///
    /// **Não existe comando C2S de decolar**: o cliente pede voo "usando" o item de voo
    /// (`USE_ITEM` no slot 12, `EQUIPIVTR_FLYSWORD`), e a resposta é isto. Responder
    /// `HOST_USE_ITEM` ali é dizer "o item foi gasto", e o cliente apaga a asa da tela —
    /// foi o que aconteceu em jogo em 2026-09-08.
    pub fn object_takeoff(id_player: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(96);               // CMD_S2C_OBJECT_TAKEOFF = 96
        stream.write_i32_le(id_player);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `OBJECT_LANDING` (97) — o jogador pousou. Mesmo layout do `object_takeoff`
    /// (`EC_GPDataType.h:2276`).
    pub fn object_landing(id_player: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(97);               // CMD_S2C_OBJECT_LANDING = 97
        stream.write_i32_le(id_player);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando FLYSWORD_TIME (Comando 98) atualizando tempo restante de voo
    pub fn flysword_time(time_left: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(98);               // CMD_S2C_FLYSWORD_TIME_CAPACITY = 98
        stream.write_i32_le(time_left);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TEAM_LEADER_INVITE (Comando 57) — o convite chegando ao convidado.
    ///
    /// `struct cmd_team_leader_invite { int idLeader; int seq; unsigned short wPickFlag; }`
    /// — `EC_GPDataType.h`. Escrevia só o `idLeader`, 6 bytes a menos, e o cliente
    /// descartava o comando inteiro (item 46).
    ///
    /// `seq` é o número do convite, que o cliente devolve ao aceitar; `wPickFlag` é a
    /// regra de divisão de despojos do grupo.
    pub fn team_leader_invite(inviter_id: i32, seq: i32, pick_flag: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(57);               // CMD_S2C_TEAM_LEADER_INVITE = 57
        stream.write_i32_le(inviter_id);       // idLeader (4B)
        stream.write_i32_le(seq);              // seq (4B)
        stream.write_u16_le(pick_flag);        // wPickFlag (2B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TEAM_JOIN_TEAM (Comando 59) — o jogador entrou no grupo.
    ///
    /// `struct cmd_team_join_team { int idLeader; unsigned short wPickFlag; }`.
    ///
    /// Escrevia `member_id` seguido de `leader_id`: **o campo errado no lugar certo**. O
    /// cliente lia o id do membro como se fosse o do líder — e, como o tamanho também não
    /// batia (8 contra 6), descartava tudo antes de chegar a usar o valor.
    pub fn team_join_party(leader_id: i32, pick_flag: u16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(59);               // CMD_S2C_TEAM_JOIN_TEAM = 59
        stream.write_i32_le(leader_id);        // idLeader (4B)
        stream.write_u16_le(pick_flag);        // wPickFlag (2B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TEAM_LEAVE_PARTY (Comando 61) — o grupo acabou para quem recebe.
    ///
    /// `struct cmd_team_leave_party { int idLeader; short reason; }`. O `reason` é
    /// `short`, e escrevíamos `int`.
    ///
    /// **Não é o comando de "fulano saiu"** — esse é o `team_member_leave` (60), que leva
    /// `idLeader`, `idMember` e `reason`. Mandar o 61 a quem fica diz "seu grupo acabou".
    pub fn team_leave_party(leader_id: i32, reason: i16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(61);               // CMD_S2C_TEAM_LEAVE_PARTY = 61
        stream.write_i32_le(leader_id);        // idLeader (4B)
        stream.write_i16_le(reason);           // reason (2B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TEAM_MEMBER_LEAVE (Comando 60) — um companheiro saiu do grupo.
    ///
    /// `struct cmd_team_member_leave { int idLeader; int idMember; short reason; }`.
    ///
    /// **Novo.** Não existia: quem ficava no grupo recebia o 61, que significa "o grupo
    /// acabou", em vez de "o fulano saiu".
    pub fn team_member_leave(leader_id: i32, member_id: i32, reason: i16) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(60);               // CMD_S2C_TEAM_MEMBER_LEAVE = 60
        stream.write_i32_le(leader_id);        // idLeader (4B)
        stream.write_i32_le(member_id);        // idMember (4B)
        stream.write_i16_le(reason);           // reason (2B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TEAM_MEMBER_DATA (Comando 64) — a lista de membros do HUD.
    ///
    /// # O layout de verdade
    ///
    /// ```text
    /// unsigned char member_count;   // quantos membros o grupo tem
    /// unsigned char data_count;     // quantos vêm neste pacote
    /// int           idLeader;
    /// struct MEMBER {               // 34 bytes cada
    ///     int  idMember;  short level;  unsigned char state;  unsigned char level2;
    ///     unsigned char reincarnation_times;  char wallow_level;
    ///     int  hp;  int mp;  int max_hp;  int max_mp;
    ///     int  force_id;  int profit_level;
    /// } data[1];
    /// ```
    ///
    /// O `CheckValid` do cliente calcula o tamanho como
    /// `sizeof(*this) - sizeof(data) + data_count * sizeof(MEMBER)`, então `data_count`
    /// **tem** que ser o número de membros escritos.
    ///
    /// # O que estava errado
    ///
    /// Tudo menos o id do comando. O cabeçalho tinha só `member_count` — faltavam
    /// `data_count` e `idLeader`, 5 bytes. E cada membro levava a **posição** (12 bytes de
    /// `A3DVECTOR3`), que não existe nesta estrutura, com `hp, max_hp, mp, max_mp` na
    /// ordem trocada — o cliente lê `hp, mp, max_hp, max_mp`. Por coincidência os dois
    /// davam 34 bytes por membro, então o erro sobreviveu ao único teste que havia.
    ///
    /// Como o comando é de tamanho variável, ele ficava **de fora** da conferência de
    /// layout contra o IR: era o único caminho pelo qual um erro destes podia passar.
    pub fn team_member_data(
        leader_id: i32,
        membros: &[MembroDoGrupo],
    ) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(64);               // CMD_S2C_TEAM_MEMBER_DATA = 64
        stream.write_u8(membros.len() as u8);  // member_count (1B)
        stream.write_u8(membros.len() as u8);  // data_count (1B)
        stream.write_i32_le(leader_id);        // idLeader (4B)
        for m in membros {
            stream.write_i32_le(m.role_id);          // idMember (4B)
            stream.write_i16_le(m.level);            // level (2B)
            stream.write_u8(m.state);                // state (1B)
            stream.write_u8(m.level2);               // level2 (1B)
            stream.write_u8(m.reencarnacoes);        // reincarnation_times (1B)
            stream.write_i8(m.wallow_level);         // wallow_level (1B)
            stream.write_i32_le(m.hp);               // hp (4B)
            stream.write_i32_le(m.mp);               // mp (4B)
            stream.write_i32_le(m.max_hp);           // max_hp (4B)
            stream.write_i32_le(m.max_mp);           // max_mp (4B)
            stream.write_i32_le(m.force_id);         // force_id (4B)
            stream.write_i32_le(m.profit_level);     // profit_level (4B)
        }
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TRASHBOX_OPEN (Comando 130) abrindo o banqueiro/armazém
    pub fn trashbox_open(capacity: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(130);              // CMD_S2C_TRASHBOX_OPEN = 130
        stream.write_u8(capacity);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando TRASHBOX_WEALTH (Comando 132) atualizando moedas guardadas no banco
    pub fn trashbox_wealth(money: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(132);              // CMD_S2C_TRASHBOX_WEALTH = 132
        stream.write_i32_le(money);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_object_enter_sanctuary { int id; }` — "self id or pet id".
    ///
    /// Não escrevia campo nenhum: 4 bytes curto, comando descartado (item 46). Sem o id o
    /// cliente também não teria como saber *quem* entrou na zona segura.
    pub fn enter_sanctuary(id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(164);              // CMD_S2C_ENTER_SANCTUARY = 164
        stream.write_i32_le(id);               // id (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `struct cmd_object_leave_sanctuary { int id; }` — o par do anterior.
    pub fn leave_sanctuary(id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(165);              // CMD_S2C_LEAVE_SANCTUARY = 165
        stream.write_i32_le(id);               // id (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_ENABLE_FASHION` (192) — o jogador passou a mostrar a roupa no lugar da
    /// armadura, ou voltou.
    ///
    /// `struct cmd_player_enable_fashion { int idPlayer; unsigned char is_enabble; }` —
    /// 5 bytes, conferido nas duas fontes: `EC_GPDataType.h:3208` e o IR
    /// (`S2C::cmd_player_enable_fashion`, `bytes: 5`).
    ///
    /// Faltava o `idPlayer`. Além do tamanho (item 46), sem ele o comando não diz de quem
    /// é a roupa — e é justamente um comando sobre o que os **outros** veem.
    ///
    /// Vai para todos que enxergam o jogador, **inclusive ele mesmo**: o cliente acha o
    /// dono pelo `idPlayer` do corpo (`EC_ManPlayer.cpp:1355-1358`). É por isso que o
    /// botão precisa da resposta do servidor para mudar de estado — ele não alterna
    /// sozinho. Ver `BusServer::trocar_modo_roupa`.
    pub fn player_enable_fashion(player_id: i32, enable: bool) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(192);              // CMD_S2C_PLAYER_ENABLE_FASHION = 192
        stream.write_i32_le(player_id);        // idPlayer (4B)
        stream.write_u8(u8::from(enable));     // is_enabble (1B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando PLAYER_CASH (Comando 253) atualizando o saldo de Gold/Cash da loja
    /// `struct player_cash { int cash_amount; }` — **um** campo.
    ///
    /// Escrevia dois: um `silver_cents` que não existe na estrutura e que todos os
    /// chamadores passavam como `0`. Quatro bytes a mais, e o cliente descartava o
    /// comando (item 46) — o saldo nunca aparecia.
    pub fn player_cash(cash_amount: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(253);              // CMD_S2C_PLAYER_CASH = 253
        stream.write_i32_le(cash_amount);      // cash_amount (4B)
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando MALL_ITEM_BUY_FAILED (Comando 271) informando falha de compra no GShop
    pub fn mall_item_buy_failed(reason: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(271);              // CMD_S2C_MALL_ITEM_BUY_FAILED = 271
        stream.write_i32_le(reason);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando INVADER_RISE (Comando 117) ativando nick rosa
    pub fn invader_rise(role_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(117);              // CMD_S2C_INVADER_RISE = 117
        stream.write_i32_le(role_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando PARIAH_RISE (Comando 118) ativando nick vermelho / PK
    pub fn pariah_rise(role_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(118);              // CMD_S2C_PARIAH_RISE = 118
        stream.write_i32_le(role_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando INVADER_FADE (Comando 119) limpando status PK
    pub fn invader_fade(role_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(119);              // CMD_S2C_INVADER_FADE = 119
        stream.write_i32_le(role_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando DUEL_PREPARE (Comando 216) iniciando a contagem de duelo
    pub fn duel_prepare(attacker_id: i32, target_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(216);              // CMD_S2C_DUEL_PREPARE = 216
        stream.write_i32_le(attacker_id);
        stream.write_i32_le(target_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando HOST_DUEL_START (Comando 218) iniciando o duelo
    pub fn host_duel_start(target_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(218);              // CMD_S2C_HOST_DUEL_START = 218
        stream.write_i32_le(target_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// Cria o comando DUEL_RESULT (Comando 220) finalizando o duelo
    pub fn duel_result(winner_id: i32, loser_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(220);              // CMD_S2C_DUEL_RESULT = 220
        stream.write_i32_le(winner_id);
        stream.write_i32_le(loser_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    // ---- Lote de comandos do `gplayer_imp::SendAllData` (EvolvedPWServer, player.cpp)
    // achados em 2026-09-03 lendo o source real do 1.5.5 (sem captura disponível pra essa
    // versão — instrução explícita do Murillo pra ir pelos fontes desta vez). São as
    // notificações "de status" que o servidor de verdade manda pro client logo depois do
    // `EnterWorld`, uma por linha de `SendAllData`; os structs vêm de
    // `EC_GPDataType.h` (client) e `protocol.h` (server), que concordam. Deixei de fora as
    // que dependem de sistemas que este servidor ainda não tem (astrolábio, cartas gerais,
    // meridianos, tomo de reencarnação, desafio solo, transmissão de posição fixa,
    // assinatura diária, fatering) — um personagem novo não tem dado nenhum pra elas, e
    // mandar zero arriscaria mais que não mandar nada.

    /// `GET_OWN_MONEY` (82) — dinheiro do jogador e o teto da carteira.
    ///
    /// 8 bytes: `amount` + `max_amount`, os dois `size_t` (4B neste engine de 32 bits) —
    /// confere com `structs["S2C::cmd_get_own_money"]` do IR do 1.5.3. O 1.5.5 acrescenta um
    /// terceiro campo (`color_name`); ver `PorVersao::get_own_money`.
    pub fn get_own_money(amount: u32, capacity: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(82);
        stream.write_u32_le(amount);   // size_t amount
        stream.write_u32_le(capacity); // size_t max_amount
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `HOST_REPUTATION` (161) — reputação/carma do jogador.
    pub fn host_reputation(reputation: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(161);
        stream.write_i32_le(reputation);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PVP_MODE` (256) — modo de PVP atual (0 = pacífico na maioria dos servidores).
    pub fn pvp_mode(mode: u8) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(256);
        stream.write_u8(mode);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `SELF_COUNTRY_NOTIFY` (333) — país/nação do jogador (0 = nenhum).
    pub fn self_country_notify(country_id: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(333);
        stream.write_i32_le(country_id);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `SERVER_TIME` (114) — hora do servidor (`cmd_server_time`: time, timebias,
    /// lua_version).
    ///
    /// # `lua_version` não é cosmético — é uma trava que derruba o client
    ///
    /// Achado em 2026-09-03: mandar `lua_version = 0` (o palpite anterior, "não temos
    /// evidência do que o client faz com um valor errado aqui") faz os dois clients de
    /// teste travarem em `EC_GameDataPrtc.cpp`, `case SERVER_TIME`: o client lê a primeira
    /// linha do seu `interfaces\script\config\global_api.lua` local (formato `--<N>`),
    /// compara com `pCmd->lua_version`, e se **não bater**, seta
    /// `g_dwFatalErrorFlag = FATAL_ERROR_WRONG_CONFIGDATA` — o loop principal
    /// (`ElementClient.cpp`) fecha o processo (`ExitProcess(-3)`) no próximo tick, com a
    /// mensagem de log "exit process because wrong config data" (exatamente o que os dois
    /// clients mostraram, ~1.3s depois do `SetServerTime`).
    ///
    /// O valor certo é **102** — primeira linha (`--102`) de três cópias independentes de
    /// `global_api.lua` que concordam entre si: `data/realm_155/config/global_api.lua`
    /// (já no nosso próprio realm), `F:\PW\1.5.5\home155\gamed\config\global_api.lua` e
    /// `F:\PW\1.5.5\pwserver_155v156\home\pwserver\gamed\config\global_api.lua`.
    pub fn server_time(unix_time: i32, timezone_bias_minutes: i32, lua_version: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(114);
        stream.write_i32_le(unix_time);
        stream.write_i32_le(timezone_bias_minutes);
        stream.write_i32_le(lua_version);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `TRASHBOX_PWD_STATE` (129) — se o baú (trashbox) tem senha configurada.
    pub fn trashbox_pwd_state(has_passwd: bool) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(129);
        stream.write_u8(has_passwd as u8);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PET_ROOM_CAPACITY` (240) — vagas disponíveis pra pets.
    pub fn pet_room_capacity(capacity: u32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(240);
        stream.write_u32_le(capacity); // size_t capacity
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PET_ROOM` (239) — lista de pets ativos no pet corral/inventário de mascotes.
    pub fn pet_room(count: u16, pets_payload: &[u8]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(239);
        stream.write_u16_le(count);
        stream.write_raw_bytes(pets_payload);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `GAIN_PET` (231) — pet obtido (ex: chocado no NPC de mascotes).
    /// Estrutura no cliente: `int slot_index; info_pet data;` (4 + 192 = 196 bytes).
    pub fn gain_pet(slot_index: i32, pet_data: &[u8]) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(231);
        stream.write_i32_le(slot_index);
        stream.write_raw_bytes(pet_data);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `SELF_KING_NOTIFY` (355) — se o jogador é rei de alguma facção/país, e quando isso
    /// expira (0 = não expira / não é rei).
    pub fn self_king_notify(is_king: bool, expire_time: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(355);
        stream.write_u8(is_king as u8);
        stream.write_i32_le(expire_time);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `FACTION_CONTRIB_NOTIFY` (297) — contribuição do jogador pra facção (consumível, de
    /// exp e acumulada). Zero pra quem não tem facção.
    pub fn faction_contrib_notify(consume_contrib: i32, exp_contrib: i32, cumulate_contrib: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(297);
        stream.write_i32_le(consume_contrib);
        stream.write_i32_le(exp_contrib);
        stream.write_i32_le(cumulate_contrib);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_LEADERSHIP` (378) — pontos de liderança do jogador (sistema de facção) e a
    /// variação desde o último aviso.
    pub fn player_leadership(leadership: i32, inc_leadership: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(378);
        stream.write_i32_le(leadership);
        stream.write_i32_le(inc_leadership);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_WORLD_CONTRIBUTION` (388) — contribuição do jogador pro "mundo" (sistema de
    /// nação/território), a variação e o custo total já gasto.
    pub fn player_world_contribution(contrib: i32, change: i32, total_cost: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(388);
        stream.write_i32_le(contrib);
        stream.write_i32_le(change);
        stream.write_i32_le(total_cost);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PLAYER_DIVIDEND` (280) — saldo de dividendos (loja de dividendos/cash shop).
    pub fn player_dividend(dividend: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(280);
        stream.write_i32_le(dividend);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `AVAILABLE_DOUBLE_EXP_TIME` (213) — tempo restante de exp em dobro disponível pra
    /// ativar (0 = nenhum).
    pub fn available_double_exp_time(available_time: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(213);
        stream.write_i32_le(available_time);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `DOUBLE_EXP_TIME` (212) — se o modo de exp em dobro está ativo agora, e até quando.
    pub fn double_exp_time(mode: i32, end_time: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(212);
        stream.write_i32_le(mode);
        stream.write_i32_le(end_time);
        Self { data: stream.into_bytes().to_vec() }
    }

    /// `PARIAH_TIME` (230) — tempo restante como pária (PK sem punição normal). 0 = não é
    /// pária.
    pub fn pariah_time(pariah_time: i32) -> Self {
        let mut stream = OctetsStream::new();
        stream.write_u16_le(230);
        stream.write_i32_le(pariah_time);
        Self { data: stream.into_bytes().to_vec() }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_octets(&self.data);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetUIConfigRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub ui_config: Vec<u8>,
}

impl S2CGetUIConfigRe {
    /// O bloco vai **exatamente como o cliente o gravou** no `SetUIConfig`.
    ///
    /// O formato é o de `CECGameRun::SaveConfigsToServer` (`EC_GameRun.cpp:2014-2130`):
    /// `DWORD USERCFG_VERSION` sem compressão + zlib(host | layout | opções), e é o mesmo que
    /// `LoadConfigsFromServer` lê (`:2139-2241`). Vazio é o caminho limpo do cliente para
    /// "sem configuração" (`configs data is empty` → `ApplyUserSetting`).
    ///
    /// **Corrigido no B53** (teste de 2026-09-17, `element/logs/EC.log`): este construtor
    /// sobrescrevia os 16 primeiros bytes do bloco gravado com um "cabeçalho" inventado
    /// (`1, 2097199, 2097199, 1206433535`). O cliente lia a versão 1 (< 3), não
    /// descomprimia, lia o fluxo zlib como dado cru e estourava o buffer — o
    /// `LoadConfigsFromServer, data read error (2)` (`TYPE_OVERBOUND`) de todo login. As
    /// barras de atalho nunca carregavam e eram gravadas vazias ao sair.
    pub fn new(role_id: i32, localsid: u32, ui_config: &[u8]) -> Self {
        Self {
            result: 0,
            role_id,
            localsid,
            ui_config: ui_config.to_vec(),
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_octets(&self.ui_config);
    }
}

/// `PlayerBaseInfo_Re` (92) — responde a `PlayerBaseInfo`: raça/classe/gênero/nome de
/// OUTRO jogador, o dado que faltava em `PLAYER_ENTER_WORLD` pro cliente saber que
/// modelo desenhar (`docs/ESTADO_E_RETOMADA.md`, item 17).
///
/// A struct real (`GRoleBase`, IR) tem 20 campos; a maioria (`forbid`, `help_states`,
/// `spouse`, `userid`, `cross_data`, `config_data`, os três `reserved*`) não tem
/// sistema nenhum implementado ainda que os preencha de verdade — vão vazios/zerados
/// no `encode()`, mesma escolha já feita em outras respostas desta fase do projeto
/// (0 é o valor que o `SendAllData` real manda pra quem não tem o dado). O que
/// **importa** pro avatar aparecer — `id`, `name`, `race`, `cls`, `gender`,
/// `custom_data` — vem do personagem de verdade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CPlayerBaseInfoRe {
    pub retcode: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub other_role_id: i32,
    pub name: String,
    pub race: i32,
    pub cls: i32,
    pub gender: u8,
    /// Bytes crus da aparência customizada — mesma extração que `write_role_info` já
    /// faz de `custom_appearance` (hex de `raw`, ou o JSON cru como fallback).
    pub custom_data: Vec<u8>,
    pub status: u8,
    pub create_time: i32,
    pub lastlogin_time: i32,
    /// Punições (ban/mute) do personagem — `GRoleForbid` no IR. Nenhum sistema de
    /// punição implementado ainda, então sempre vazio; o campo existe (em vez de um
    /// `write_compact_uint(0)` isolado no `encode`) pra o formato do item ficar
    /// documentado no código, não só no IR.
    pub forbid: Vec<GRoleForbid>,
}

/// Uma punição de `GRoleBase::forbid` — `GRoleForbid` no IR (`type, time, createtime,
/// reason`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GRoleForbid {
    pub tipo: u8,
    pub time: i32,
    pub createtime: i32,
    pub reason: Vec<u8>,
}

impl S2CPlayerBaseInfoRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.retcode);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        // GRoleBase
        stream.write_i8(1); // _literal: marcador de "objeto presente" no RPC marshalling
        stream.write_u32(self.other_role_id as u32); // id
        stream.write_string_utf16le(&self.name);
        stream.write_i32(self.race);
        stream.write_i32(self.cls);
        stream.write_u8(self.gender);
        stream.write_octets(&self.custom_data);
        stream.write_octets(&[]); // config_data — nenhum sistema de UI/facção preenche isto ainda
        // `custom_stamp` — o carimbo da aparência, e ele **tem de bater** com o `crc_c`
        // que a `info_player_1` daquele mesmo personagem leva (`VistaDoJogador`). O
        // cliente guarda este valor (`m_PlayerInfo.crc_c = base.custom_stamp`,
        // `EC_ElsePlayer.cpp:1881`) e o compara com o do próximo pacote de visão para
        // decidir se precisa pedir a aparência de novo (`:176`). Zero fixo dos dois lados,
        // como ia até 2026-09-11, faz o cliente nunca perceber uma troca de visual.
        stream.write_u32(pw_core::stamp_de_aparencia(&self.custom_data) as u32);
        stream.write_u8(self.status);
        stream.write_i32(0); // delete_time — 0 enquanto não existe exclusão/restauração visível aqui
        stream.write_i32(self.create_time);
        stream.write_i32(self.lastlogin_time);
        stream.write_compact_uint(self.forbid.len() as u32);
        for f in &self.forbid {
            stream.write_u8(f.tipo);
            stream.write_i32(f.time);
            stream.write_i32(f.createtime);
            stream.write_octets(&f.reason);
        }
        stream.write_octets(&[]); // help_states
        stream.write_u32(0); // spouse
        stream.write_u32(0); // userid
        stream.write_octets(&[]); // cross_data
        stream.write_u8(0); // reserved2
        stream.write_u8(0); // reserved3
        stream.write_u8(0); // reserved4
    }
}

/// `GetCustomData_Re` (117) — a aparência customizada de OUTRO jogador, pedida depois
/// do `PlayerBaseInfo` quando a base já chegou mas a aparência não.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetCustomDataRe {
    pub retcode: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub cus_role_id: u32,
    pub custom_data: Vec<u8>,
}

impl S2CGetCustomDataRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.retcode);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_u32(self.cus_role_id);
        stream.write_octets(&self.custom_data);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Campos e ordem conforme `GetFriends_Re` (id 207) no IR:
/// `roleid`, `groups`, `friends`, `status`, `localsid`.
///
/// A versão anterior escrevia um campo `result` que **não existe no protocolo**, punha
/// o `localsid` em terceiro lugar em vez de por último, e mandava uma única lista onde
/// o protocolo tem três. Tudo depois do primeiro campo saía deslocado.
pub struct S2CGetFriendListRe {
    pub role_id: i32,
    pub groups: Vec<FriendGroup>,
    pub friends: Vec<FriendEntry>,
    /// Estado de presença, **um por amigo e na mesma ordem** da lista acima.
    pub status: Vec<i8>,
    pub localsid: u32,
}

/// Um grupo da lista de amigos (`GGroupInfo` no IR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendGroup {
    pub gid: i8,
    pub name: String,
}

/// Uma entrada da lista de amigos (`GFriendInfo` no IR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendEntry {
    pub rid: i32,
    pub cls: i8,
    pub gid: i8,
    pub name: String,
}

impl S2CGetFriendListRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.role_id);

        stream.write_compact_uint(self.groups.len() as u32);
        for g in &self.groups {
            stream.write_i8(g.gid);
            stream.write_string_utf16le(&g.name);
        }

        stream.write_compact_uint(self.friends.len() as u32);
        for f in &self.friends {
            stream.write_i32(f.rid);
            stream.write_i8(f.cls);
            stream.write_i8(f.gid);
            stream.write_string_utf16le(&f.name);
        }

        stream.write_compact_uint(self.status.len() as u32);
        for s in &self.status {
            stream.write_i8(*s);
        }

        stream.write_u32(self.localsid);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetWaitDelRolesRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CGetWaitDelRolesRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_compact_uint(0); // lista vazia
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CServerTimeRe {
    pub timestamp: i32,
    pub time_zone: i32,
}

impl S2CServerTimeRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.timestamp);
        stream.write_i32(self.time_zone);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CGetHelpStatesRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
    pub help_states: Vec<u8>,
}

impl S2CGetHelpStatesRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
        stream.write_octets(&self.help_states);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSetHelpStatesRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CSetHelpStatesRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// `SetUIConfig_Re` (103) — confirma que o servidor salvou a configuração de UI.
///
/// **Achado em 2026-09-04**: faltava o `localsid` (IR: `result, roleid, localsid`, 12
/// bytes — este codificador só escrevia 8). O cliente descarta um pacote de tamanho
/// errado sem avisar ninguém no jogo, mas AQUI o efeito colateral é bem pior que um
/// campo de UI que não atualiza: o próximo pacote do fluxo é lido a partir do byte
/// errado, e o decodificador GNET do cliente lança "Decode error 103" — que ele trata
/// como link quebrado (`OnLinkBroken`) e derruba a sessão inteira. É a causa provável de
/// "aparece desconectado" ao sair para a seleção de personagem ou fechar o jogo — esse é
/// o momento em que o cliente salva o layout de UI antes de encerrar. Ver
/// `docs/ESTADO_E_RETOMADA.md`, item 18.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSetUIConfigRe {
    pub result: i32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CSetUIConfigRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// `SetCustomData_Re` (101) — confirma que o servidor salvou a aparência customizada.
///
/// Mesmo achado do `SetUIConfig_Re` acima, só que faltavam **dois** campos: o IR
/// (`result, CRC, roleid, localsid`, 16 bytes) tem um `CRC` entre `result` e `roleid`
/// que este codificador nunca escrevia. `CRC` não é lido pelo cliente em
/// `OnPrtcSetCustomDataRe` (confirmado em `EC_GameSession.cpp`) — só precisa **existir**
/// no pacote pros campos seguintes caírem no deslocamento certo, então `0` é seguro.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CSetCustomDataRe {
    pub result: i32,
    pub crc: u32,
    pub role_id: i32,
    pub localsid: u32,
}

impl S2CSetCustomDataRe {
    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_u32(self.crc);
        stream.write_i32(self.role_id);
        stream.write_u32(self.localsid);
    }
}

/// Pacote oficial S2C de Logout (Opcode 69 / 0x45 - PROTOCOL_PLAYERLOGOUT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2CPlayerLogout {
    pub result: i32,
    pub role_id: i32,
    pub provider_link_id: i32,
    pub localsid: u32,
}

impl S2CPlayerLogout {
    pub fn new(result: i32, role_id: i32, localsid: u32) -> Self {
        Self {
            result,
            role_id,
            provider_link_id: -1,
            localsid,
        }
    }

    pub fn encode(&self, stream: &mut OctetsStream, _version: &str) {
        stream.write_i32(self.result);
        stream.write_i32(self.role_id);
        stream.write_i32(self.provider_link_id);
        stream.write_u32(self.localsid);
    }
}



