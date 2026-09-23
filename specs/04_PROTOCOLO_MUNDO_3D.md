# Especificação 04: Protocolo do mundo 3D (subcomandos do `GamedataSend`)

> Camadas 2/3 e pacotes de itens/experiência 126 conferidos em 2026-09-21, base `a305e51` + B89/B90. Demais áreas:
> referência 2026-09-14, commit `e6433ae` + B49. Cobre
> `crates/pw-protocol/src/{packets,versions,opcodes.rs}`, `crates/pw-wire/`,
> `crates/pw-gs/src/comandos.rs`, `specs/protocol/` e `tools/pw-rpcgen/`.

## 1. Os dois formatos na mesma conexão

| | GNET | gamedata |
| :--- | :--- | :--- |
| onde | protocolos entre cliente e link (login, `GamedataSend` 34, chat…) e no barramento | conteúdo do `GamedataSend`: o mundo 3D |
| ordem | big-endian | **little-endian** |
| tamanhos | `CompactUINT` antes de `Octets`, strings, contêineres | nenhum: contagem é campo explícito |
| layout | sequência de campos | `#pragma pack(1)`, endereçável por deslocamento |
| código | `pw_wire::gnet` (e `pw_protocol::octets`, duplicado) | `pw_wire::gamedata` |

Subcomando: `[u16 LE id][payload]`. Os ids C2S e S2C são enums **separados**.

## 2. A regra que mais custou: tamanho exato ou descarte silencioso

O cliente calcula o tamanho esperado de cada S2C (`CalcS2CCmdDataSize`: `sizeof(T)` ou
`CHECK_VALID(T)`) e **descarta o comando inteiro** se não bater — sem erro em log nenhum dos
dois lados (A46, B14, B15, B33). Consequências:

1. Todo codificador S2C novo ou alterado é conferido contra **as três fontes**, nesta ordem
   de autoridade: overlay do cliente em jogo > `EC_GPDataType.h` do `EvolvedPWClient`
   (dentro do `#pragma pack(1)` que começa na linha 563) > IR `gamedata_155.json`.
2. O binário pode ficar **entre** o IR e o fonte: `OWN_EXT_PROP` tem 188 no IR, 228 no fonte
   e **196** no binário instalado (B34a; memória `pw_client_155_fonte_vs_binario`).
3. Structs com campos condicionais a bits de `state` são "variáveis" no IR (`bytes: null`);
   o teste compara o **tamanho-base** (soma dos campos fixos) para não deixá-las passar.
4. Instrumento: no cliente, `##debug` no chat → `Shift` + tecla à esquerda do "1" →
   `d_rtdebug 1`. Mostra `Unknown GAMEDATA_n` e `Invalid X size(Network:a, Client:b)`. **Não
   mostra comando aceito** (`LOG_PROTOCOL` fora do binário, B42f).

## 3. IR e testes que travam layout

| arquivo | o que é |
| :--- | :--- |
| `specs/protocol/gamedata_153.json`, `gamedata_155.json` | cada comando (id, papel, struct do cliente e do servidor) e cada struct campo a campo com deslocamento, conferidos por `g++ -m32` (4.426 asserções) |
| `specs/protocol/gnet_153.json`, `gnet_155.json` | protocolos GNET, estruturas e RPCs |
| `tools/pw-rpcgen` | gera os IRs dos fontes C++ (`--strict` falha com diagnóstico) |

O 1.5.5 **só acrescenta** ao 1.5.3: nenhum id trocado ou removido; structs comuns idênticas
ou com campos novos no fim. Novos do 1.5.5 ainda não triados: 66 GNET, 17 C2S, 24 S2C.

| teste | cobra |
| :--- | :--- |
| `pw-protocol/tests/opcodes_contra_o_ir.rs` | cada constante de opcode e subcomando contra o IR; dois símbolos não reivindicam o mesmo id |
| `pw-protocol/tests/subcomandos_s2c_contra_o_ir.rs` | tamanho de cada codificador S2C contra o IR 1.5.5; tabelas `INTENCAO` (codificador ↔ comando) e `LAYOUT_DIVERGE` (diferenças conhecidas, cada uma com o porquê) |
| `pw-protocol/tests/campos_contra_o_ir.rs` | sequência de `write_*`/`read_*` dos pacotes GNET contra o IR |
| `pw-protocol/tests/layouts_do_126.rs` | layouts medidos do 1.2.6 |
| `pw-gs/tests/comandos_contra_o_ir.rs` | cada decodificador C2S devolve o valor posto no deslocamento que o IR anuncia |
| `pw-wire/tests/conformance_*` | empacotamento contra o IR (1.064 structs gamedata, 620 GNET) |

## 4. Diferenças por versão: Padrão Estratégia e Módulos por Versão

**Auditoria 1.2.6 em `2dca19e` (B73-126, 2026-09-20):** o inventário completo das chamadas
atuais do mundo está em `docs/INVENTARIO_PROTOCOLO_126.md` (98 ids S2C, 46 C2S).
Compatibilidade ainda **parcial**: os comandos comuns 14 e 64
divergem dos comprimentos capturados. Medição reproduzida em
`docs/evidencias/126/full_interno.medidas.md`; tamanho igual não comprova campos iguais.
`SCENE_SERVICE_NPC_LIST` (390) é opcional no trait: v126 não emite, pois seu binário
rejeita ids acima de 260 (VA 0x584618; B74-126, `docs/ENTRADA_126.md`).
`EQUIP_DATA` no 126 usa máscara de 32 bits e 10+4×n bytes de payload (VA 0x584a1d);
o 155 mantém 64 bits. Avisos de status da entrada vêm do trait, com a sequência
anterior como padrão e oito avisos suportados no v126. Estado: **11 testes focados
aprovados com TEST_DATABASE_URL**, incluindo sentinelas 155; não publicado.
SELF_INFO_00 e GetUIConfig_Re reproduzem amostras; OWN_EXT_PROP tem 152 B e
**confere byte a byte** com a amostra `s2c-50.txt` (B93): o trait recebe Atq. Mágico e
resistências desde o B77, e na amostra valem 1–1 e 2×5 (`[112..140]`). `max_ap` é
parâmetro do trait (B70): na amostra vale zero; em jogo acompanha SELF_INFO_00 (B92).
Suíte inteira com o banco depois do merge na `main`: 654/0 (B93).

O despacho por versão é uma **estratégia**: o trait `WorldProtocol`
(`crates/pw-protocol/src/traits.rs`) declara os comandos cujo layout muda entre versões, e há
uma implementação por versão em `crates/pw-protocol/src/versions/`. Quem precisa de um
comando pede à estratégia da versão daquele realm
(`versions::create_world_protocol(versao) -> Arc<dyn WorldProtocol>`), guardada no
`BusServer`. **Não há mais a fachada `PorVersao`** (removida no B67): ela só delegava, e ter
dois nomes para a mesma coisa convidava a escrever `if versao == ...` de novo.

- `versions/v155/`: a implementação **canônica** (196 B de `own_ext_prop`, 5 blocos no `task_data`, 35 B de `info_npc`, 23 campos no `RoleInfo`).
- `versions/v126/`: a 1.2.6 por inteiro (152 B, 3 blocos, 27 B, 19 campos, sem `refretcode`).
- `versions/v148/`, `v153.rs`, `v172/`: **compõem** a do 1.5.5 (`V148Protocol(V155Protocol)`) e sobrescrevem só o que difere — é assim que se acrescenta versão nova.

`HOST_SKILL_ATTACKED` (144) também passa pelo trait: v126 emite 15 bytes,
com flag de um byte e sem section; padrão 155 mantém 19 bytes. Captura
`s2c-144.txt:2` e validador do cliente VA 0x584af4 (B89). Os comandos normais
84/83/24/26/33 reproduzem as amostras. Estado: seis testes focados aprovados,
sem validação visual; cadência medida e limites em `docs/COMBATE_126.md`.

Itens 126 (B90, **testado**): 31/99 têm 14 B, 46 tem 9 B, 72 tem
7+13×n B e 156 tem 10 B de payload. Os cinco passam pelo trait; padrão 155
inalterado, overrides em v126. Contagens u16; 72 sem yinpiao e 156 sem validade.
36 (4 B) já usa o trait; 158 (8 B) permanece comum. Gabaritos da captura e
validador em `docs/ITENS_EXPERIENCIA_126.md`; sete testes de protocolo e dois de
mundo aprovados com banco. Não implica suporte às listas/missões v55 nem ao C2S
de compra; sem publicação ou validação visual.

**Despacho pós-merge (B93):** retirada de amuleto esgotado usa `player_drop_item`
do trait, inclusive no tratamento de eventos. `elf_exp` (283) é opcional:
padrão Some preserva os bytes 155; v126 None pelo limite de id 260 do cliente
(VA 0x584618). Sem mudança da regra de ganho do Daimon. O teste literal garante
a omissão 126 e preservação 155; detalhes em `docs/SINCRONIZACAO_126.md`.

Um comando que **não** varia entre versões continua em `S2CGamedataSend`, com um só caminho
de escrita. Quando uma medição mostrar que ele varia, ele sobe para o trait — é a regra "um
caminho de escrita por layout".

| codificador | 1.2.6 | 1.5.x | o que muda |
| :--- | ---: | ---: | :--- |
| `task_data` | 12 | 25 | 3 → **5** tamanhos (`finished_count`, `storage_task`); com 3 o cliente 1.5.5 crasha na renderização (B14) |
| `npc_enter_world` / `npc_enter_slice` (`info_npc`) | 27 | 35 | `vis_tid`, `state2` |
| `self_info_1` | 34 | 38 | `state2`; sem ele, 30 s de "entrando" e desconexão. O `state` leva o bit **`MODA`** quando o personagem está de roupa: é daqui que o **dono da tela** descobre o próprio modo (`m_bFashionMode`, `EC_HostPlayer.cpp:819-822`) — o `info_player_1` só resolve para quem o vê (B86). No 1.2.6 o bit não vai: não foi conferido contra aquele cliente |
| `player_enter_world` / `player_enter_slice` (`info_player_1`) | 26 | 30 | `state2` |
| `get_own_money` | 8 | 8 no codificador comum | payload sem os 2 bytes do id; captura 126, S2C 82 |
| `inst_data_checkout` | 16 | 20 (24 com gshop3) | payload sem os 2 bytes do id; captura 126, S2C 206 |
| 5 resultados de ataque (`host_attack_result`, `host_attacked`, `self_skill_attack_result`, `object_skill_attack_result`…) | −3/−4 | | `attack_flag` era `char` e virou `int`; o `section` falta no 1.2.6 |
| `npc_info_00`, `player_info_00` | | | ganham `iTargetID` |
| `enter_sanctuary`, `leave_sanctuary` | 0 | | ganham `id` |
| `own_ext_prop` | 152 | 196 | `EXTENDED_PROPERTY` menor na 1.2.6 (sem atributos novos de classes tardias) |
| `receive_exp`, contagens do `equip_item` | 16 bits | 32 bits | |
| `equip_data`, `object_move`, `object_stop_move` | | | ver `versions/v126/` e `versions/v155/` |

Uma captura de 1.2.6 mediu 175 comandos: 106 idênticos ao 1.5.3, **32 diferentes**
(`docs/MEDIDAS_DO_126.md`). Só entra aqui diferença **medida**.

## 5. Fatos de layout que não são óbvios

| comando | fato | origem |
| :--- | :--- | :--- |
| `info_player_1` | `cid, pos, crc_e, crc_c, dir, level2, state, state2` — **sem `world_tag`**; 30 bytes de parte fixa. Sexo = bit `0x40` do `state2` (zero = homem); `crc_c` tem de ser igual ao `custom_stamp` do `PlayerBaseInfo_Re`; `level2` leva o **cultivo** (`_basic.sec_level`) e o GM acende `STATE_GAMEMASTER 0x4000` no `state`. **O `state` decide o tamanho do comando** — ver a linha seguinte | `EC_GPDataType.h:603,709`; `gs/player_imp.h:1886` (B42b, B67) |
| `object_state` do `info_player_1` | Cada bit ligado pode acrescentar campos, e o cliente **calcula o tamanho esperado a partir deles** (`info_player_1::CheckValid`): errar a conta faz o pacote ser descartado em silêncio. Escrevem-se na ordem do original. Os que este servidor liga (B80): `FORMA` 0x1 (+1 byte, `shape_form`), `VOO` 0x10, `CADAVER` 0x80 (`IsZombie`), `MODA` 0x2000, `GM` 0x4000, `MONTADO` 0x80000 (**+6 bytes**: `u16 mount_color`, depois `int mount_id`). Os demais (`EMOTE`, `EXTEND_PROPERTY`, `MAFIA`, `MARKET`, `EFFECT`, `PARIAH`, `IN_BIND`, `SPOUSE`, `EQUIPDISABLED`, `PLAYERFORCE`, `MULTIOBJ_EFFECT`, `COUNTRY`, e os sete do `state2`) seguem em `falta`, e por isso vão com bit zero. Sem o bit `MONTADO`, quem entra no campo de visão de alguém montado desenha a pessoa a pé — o `PLAYER_MOUNTING` (227) só alcança quem estava vendo na hora | `gs/object.h:143-180`; `common/protocol_imp.h:62-180`; `EC_GPDataType.h:198-234,624-710`; `EC_ElsePlayer.cpp:335-455` |
| entrada por streaming | jogador `PLAYER_ENTER_SLICE` (12), NPC `NPC_ENTER_SLICE` (11); o 17 é para quem **surgiu** (efeito de teleporte) | `EC_ManPlayer.cpp:1845` |
| saída | criatura/jogador fora de alcance `OBJECT_LEAVE_SLICE` (13); matéria só sai por `OUT_OF_SIGHT_LIST` (34); jogador que saiu do jogo `PLAYER_LEAVE_WORLD` (19) | `EC_GameDataPrtc.cpp:891,1056` |
| `MATTER_ENTER_WORLD` (18) | 25 bytes: `mid, tid, pos, dir0, dir1, rad, state, value`; `state = 0` recurso comum | `EC_GPDataType.h:784` |
| `OBJECT_MOVE` | `use_time` em **milissegundos**, `speed` = velocidade × **256**, `move_mode` com bit do habitat | `gs/npcsession.cpp:258`, `gs/petnpc.cpp:350` (B48) |
| `OWN_EXT_PROP` | **196** bytes no binário; `attack_speed` em ticks de 50 ms; velocidades do `CHARRACTER_CLASS_CONFIG`; o `max_ap` é o último `i32` e tem de espelhar o `iMaxAP` do `SELF_INFO_00` — o cliente compara o anterior com ele e anuncia aumento se divergir | B34a, B44b2, `EC_HostMsg.cpp:1319-1332` (B70) |
| `OWN_ITEM_INFO` (40) | arma/armadura/acessório **precisam** do bloco de dados, senão `CanUseEquipment` recusa (item vermelho). Cabeçalho `prerequisition` (vitalidade antes de agilidade, máscara de classe em 16 bits) + essência: arma 44 (`weapon_level` = `level` do arquivo, B51), armadura 36 (`defesa, evasão, +MP, +HP, resist[5]`), acessório 36 (`dano, dano mágico, defesa, evasão, resist[5]`), **munição 20** (`tipo, dano extra, dano %, nível mín/máx da arma`, cabeçalho `0/0xFFFF/…` e durabilidade 100/100 — 1/1 na tela, B61 — sem ela o cliente mostra "arma 0-0" e o arco fica vermelho, B51). Subtipo **não** viaja | `generate_item_temp.h:490-556,772-830` (B41a) |
| `SERVER_TIME` (114) | `lua_version` = primeira linha do `global_api.lua` (1.5.5: 102); errado encerra o cliente | B9c |
| `ACTIVATE_REGION_WAYPOINTS` (C2S 178) | responder `WAYPOINT_LIST` (S2C 180) com os ids recebidos; sem isso o cliente reenvia a cada quadro | B9b |
| `GetUIConfig_Re` (GNET 105) | vazio é válido (o cliente cai nas opções padrão e barras vazias); com dado, o bloco exato do último `SetUIConfig`: `USERCFG_VERSION` + zlib(host = barras de atalho, layout — inclusive o rastreador de missões `bTraceAll` no byte 288 do layout e `dwTraceMask` no 320, `DlgTask.cpp:425-455`, `EC_GameUIMan.h:319-388` —, opções) (`EC_GameRun.cpp:2014-2230`). Uma resposta, ao pedido que vem depois do `TASK_DATA` do mundo; com mundo, o link não manda `TASK_DATA` próprio (spec 02 §4). Com `bTraceAll = 0` "fixar missão" não desenha nada (`DlgTask.cpp:476`). **Byte a byte** o que o `SetUIConfig` gravou — até o B53 os 16 primeiros bytes eram trocados e o cliente registrava `data read error (2)` (B54); sem gravação, o `config_data` do molde da classe | B7, B51–B54 |
| `UPDATE_EXT_STATE` (124) | `int id; DWORD states[6]` (2+28) — bits `VSTATE_*` dos efeitos | `EC_GPDataType.h:2520-2524`, `:539` (B53) |
| `ICON_STATE_NOTIFY` (125) | variável: `int id; u16 scount; u16 state[]` (2 bits altos = nº de parâmetros); `u16 pcount; int param[]` — cada ícone com 1 parâmetro (tempo restante, s) | `EC_GPDataType.h:2538-2622` (B53) |
| `ENCHANT_RESULT` (139) | `caster, target, skill i32; level, orange_name u8; attack_flag i32; section u8` (2+19) | `EC_GPDataType.h:2714-2723` (B53) |
| bloco de dados do equipamento (`OWN_ITEM_INFO` 40) | cabeçalho 6×i16 + durabilidades (**na escala interna**, ×100 — spec 05, "Durabilidade") + essência + `i16 furos, u16 máscara, i32×furos` + `i32 addons` e cada addon `i32 tipo (id \| n<<13 \| 0x8000 pedra)` + `i32×n`; um só caminho: `pw_core::ConteudoDeEquipamento` (item sem octetos, octetos sorteados no drop, leitura dos atributos) | `EC_IvtrEquip.cpp:176-262` (B53) |
| `HOST_START_ATTACK` (84) / `HOST_STOPATTACK` (23) | `idTarget i32, ammo_remain u16, attack_speed u8` (2+7) / `iReason i32` (2+4) — abrem e fecham a sessão de golpe (spec 05 §5.2) | `EC_GPDataType.h:1509,2190` (B52) |
| `SKILL_INTERRUPTED` (86) / `SELF_SKILL_INTERRUPTED` (87) | `caster i32` (2+4) enviado a terceiros / `reason u8` (2+1, reason=2) enviado ao próprio jogador quando a conjuração é cancelada (por ESC/`CANCEL_ACTION` ou movimento) | `playercmd.cpp:2136-2153`, `player.cpp:4017-4028` (B67) |
| `SCENE_SERVICE_NPC_LIST` (390) | `count u32` seguido de pares `{ service_id i32, npc_id i32 }`. Enviado no `GET_ALL_DATA` com os provedores de serviços do mapa (incluindo mestres de classe). No 1.5.5 (`EC_HostSkillModel.cpp:558-605`), é obrigatório para registrar `m_allProfNPCs`, definir `m_skillLearnNPCNID` e habilitar o botão de evoluir habilidade pela árvore (tecla R) | `world.cpp`, `EC_HostSkillModel.cpp:558-605` (B67) |
| `HOST_ATTACKED` (26) | `idAttacker i32, iDamage i32, cEquipment u8, attack_flag i32, speed i8`. O `cEquipment` é o **índice da peça que se desgastou** (`eq_index &= 0x7F`, bit alto = nome laranja): `0x7f` é "nenhuma", e **zero o cliente lê como a arma** e desconta durabilidade dela a cada golpe recebido. `speed` × 50 ms é a duração da animação do golpe (spec 05, "Durabilidade") | `cgame/common/protocol.h:1194-1201`, `protocol_imp.h:580-590`, `EC_HostMsg.cpp:968-1000` |
| `TASK_DELIVER_LEVEL2` (160) | `int id_player; int level2` (2+8). O `level2` é o **nível de cultivo**, não o nível de GM: o cliente guarda em `m_BasicProps.iLevel2`, tira dele o título taoista e toca o efeito de avanço (`CECPlayer::OnMsgPlayerLevel2`, `EC_Player.cpp:7464`; `GetLevel2Name`, `EC_GameRun.cpp:3477`). O mesmo vale para o campo `level2` dos pacotes de visão — até o B67 mandávamos o privilégio da conta ali | `Network/EC_GPDataType.h:2859-2863`, `gs/player_imp.h:2798-2804` |
| `SELF_INFO_00` (38) e `PLAYER_INFO_00` (32) | o `State` é o **modo de combate** (`IsCombatState() ? 1 : 0`, `gs/player.cpp:3554,3570`): o cliente liga `m_bFight` com ele e troca a animação do personagem (`EC_HostMsg.cpp:1334-1335`). Ia zero fixo até o B77, e o personagem nunca entrava em postura de luta |
| `OWN_EXT_PROP` (50) — Atq. Mágico | `damage_magic_low/high` fecham o `ROLEEXTPROP_ATK` (depois dos dez `addon_damage`) e as cinco `resistance[]` abrem o `ROLEEXTPROP_DEF`. Iam zero fixo: um personagem mágico via a ficha sem ataque mágico mesmo com arma mágica (B77) |
| `SELF_INFO_00` (38) | `sLevel, State, Level2, iHP, iMaxHP, iMP, iMaxMP, iExp, iSP, **iAP, iMaxAP**` — os dois últimos são a **barra de chi**, e iam zero fixo até o B68. **`Level2` é o cultivo** e vale em *todo* envio: o cliente faz `SetLevel2` a cada comando e toca o efeito de avanço quando o novo é maior que o anterior (`CanPlayTaoistEffect`, `EC_Player.cpp:7434-7454`), de modo que mandar zero num comando e o valor certo no seguinte anuncia um avanço que não houve (B71) | `Network/EC_GPDataType.h:1739-1752`, `EC_HostMsg.cpp:1324` |
| `ACTIVATE_WAYPOINT` (179) | `unsigned short waypoint` (2+2) — **um** ponto de teleporte novo. É ele que faz o cliente anunciar "novo ponto" com o nome do lugar (`CECHostPlayer::OnMsgHstWayPoint`, `EC_HostMsg.cpp:4681-4720`); o `WAYPOINT_LIST` (180) **substitui** a lista em silêncio e é o da carga inicial. Sai de `ActivateWaypoint` só quando o ponto ainda não é do jogador (`gs/player_imp.h:2534-2544`) (B67) |
| `ACTIVATE_REGION_WAYPOINTS` (C2S 178) | `unsigned char num` + `num` × `int`; o servidor ativa os que faltam e responde um 179 por ponto. Tratado **no mundo** desde o B67 (`gs/playercmd.cpp:4262-4270`, `player.cpp:25196-25220`) |
| `EQUIP_DAMAGED` (68) | `unsigned char index; char reason` (2+2): a peça `index` acabou; motivo 0 é "sem durabilidade", 1 é "quebrou ao morrer" | `cgame/common/protocol.h:1598-1603` (B61) |
| `CALC_NETWORK_DELAY` (C2S 128) | `_RE` (S2C 291) devolve o `timestamp` intacto; só alimenta o indicador de ping | B42i |
| `GP_NPCSEV_LEARN` | pedido só com `idSkill`; resposta `LEARN_SKILL` (95) com id e nível | `EC_SendC2SCmds.cpp:3379` |
| `NORMAL_ATTACK` (C2S 3) | 3 bytes, **sem id de alvo** — o alvo é o selecionado | A33 |
| `TASK_NOTIFY` (C2S 49) | `size` + `task_notify_base { u8 reason, u16 task }`. Os `reason` do cliente (`TASK_CLT_NOTIFY_*`) e os do servidor (`TASK_SVR_NOTIFY_*`) são **tabelas diferentes**: o 7 do cliente é "me dê a marca dinâmica", o 7 do servidor é "esqueça a habilidade de produção" | `task/TaskTempl.h:81-122` (B49) |
| marca das missões dinâmicas | resposta ao `reason` 7: `TASK_VAR_DATA` (106) com `svr_task_dyn_time_mark` = `reason 8, task 0, time_mark u32, version u16 = 10` (9 bytes); sem marca, sem resposta | `TaskTemplMan.cpp:299-309`, `TaskClient.cpp:290` |
| `TASK_DATA` (105) | cinco blocos `size_t` + bytes: listas ativa, concluídas, tempos, contagens e depósito, **com o conteúdo das estruturas** (spec 05 §10). Lista ativa vazia vai com o cabeçalho de 8 bytes e `version = 1`: com `version 0` o cliente descarta todo aviso de missão. Sai do link na entrada (do banco) e do mundo no fim do `GET_ALL_DATA` (da memória) | `player.cpp:4388`, `TaskClient.cpp:262`, `TaskProcess.cpp:2315-2342` (B50) |
| `QUERY_TITLE` (C2S 154) → `QUERY_TITLE_RE` (S2C 363) | `roleid i32`, `titlescount i32`, `expirecount i32`, e então `titlescount` ids `u16` e `expirecount` pares `{ id u16, time i32 }`; sem título são os **12 bytes** do cabeçalho (`CheckValid` do próprio struct). **Responder é obrigatório**: o cliente só liga `m_bTitleDataReady` aqui, e `ATaskTemplMan::UpdateStatus` não chama `CheckAutoDelv` enquanto isso — nenhuma missão de entrega automática aparece (B60) | `EC_GPDataType.h:4632-4654`, `EC_HostPlayer.cpp:10145-10152`, `task/TaskTemplMan.cpp:1342-1350` |
| pacote das missões dinâmicas | resposta ao `reason` **8** (`TASK_CLT_NOTIFY_DYN_DATA`): o `dyn_tasks.data` inteiro, em pedaços de `0x1000 − sizeof(task_notify_base)` = **4.093** bytes, cada um num `TASK_VAR_DATA` com `reason 9` (`TASK_SVR_NOTIFY_DYN_DATA`) e `task` = 1 só no último. O cliente pede isso quando a marca não bate com o pacote local dele, e **só monta a lista de missões ativas** depois do último pedaço (`OnDynTasksData` → `InitActiveTaskList`). Sem a resposta, nenhuma missão nova aparece em jogo, nem pelo "Procurar Missão" (B59) | `TaskTemplMan.cpp:166-230`, `321-353` |
| avisos de missão (`TASK_VAR_DATA` 106) | `task_notify_base` = `reason u8, task u16`, `pack(1)`. `NEW`: + `cur_time u32, cap_task u32, sub_tags` (`sub_task u16, sz u8, tags[sz]`); `COMPLETE`: + `cur_time u32, sub_tags` com o **estado** no lugar do `sub_task`; `MONSTER_KILLED`: + `monster_id u32, num u16, dps i32, dph i32` = **17** bytes; `ERROR_CODE`: + `u32`; `GIVE_UP`/`FINISHED`: só a base. O cliente confere o tamanho exato de cada um | `task/TaskTempl.h:1737-1875`, `TaskTempl.inl:2206-2286` |
| `GP_NPCSEV_TASK_ACCEPT` / `_RETURN` | `{ int idTask, idStorage, idRefreshItem }` / `{ int idTask, iChoice }`; serviços 7 e 6 | `serviceprovider.cpp:1001-1066` |
| `PICKUP_ITEM` (31), `TASK_DELIVER_ITEM` (156), `PURCHASE_ITEM` (72) | levam o **último slot e a quantidade final**; o cliente empilha sozinho (`MergeItem`) e descarta se não bater | `EC_HostMsg.cpp:1127-1252,3334` |
| `PURCHASE_ITEM` (72) | `cost, yinpiao, flag u8, count u16` + itens de 15 bytes (`item_id, expire, count, inv_index u16, booth_slot u8`) | `EC_GPDataType.h:2104` |
| `ITEM_TO_MONEY` (73) | `index u16, type, count, money` (14) | `player.cpp:4131` |
| `PICKUP` (C2S 6) / `PICKUP_ALL` (C2S 184) | `{ int mid; int type }` / `{ int count; { mid, type }[] }` (≤ 100) | `common/protocol.h:4979-4996` |
| `SET_COOLDOWN` (198) | `index = id + 1024`, tempo em ms | `playerwrapper.cpp:170` |
| `ERROR_MESSAGE` (25) | `ERR_*` de `common/protocol.h:679`: 6 não pode pegar, 7 bolsa cheia, 16 sem dinheiro, 19 missão indisponível, 20 habilidade indisponível, 22 não pode aprender, 53 em recarga, 66 em combate | |
| ids de entidade | `i32` no fio; o mundo guarda **com sinal** (NPC `0x80…` e matéria `0xC…` são negativos). Item no chão usa `0xC8000000 + n` | `npcgen.rs` |
| `NOTIFY_HOSTPOS` (14) | `pos, tag, line` = 2 + 20; `tag` diferente do mapa carregado faz o cliente trocar de mundo. Escrevia `pos + u8` | `EC_GPDataType.h:1362`, `player.cpp:3530` (B51) |
| `ATTACK_ONCE` (83) | `arrow_dec u8` a cada golpe normal, antes do resultado | `player.cpp:3134,3321` (B51) |
| `attack_flag` | bits desconhecidos; vai zero (crítico não sinalizado) | — |

## 6. Onde cada C2S é tratado

**No `pw-gs`** (`BusServer::tratar_subcomando`, decodificadores em `comandos.rs`):

| id | comando | id | comando |
| ---: | :--- | ---: | :--- |
| 0 | `PLAYER_MOVE` | 30 | `TEAM_LEAVE_PARTY` |
| 1 | `LOGOUT` | 33 | `GET_OTHER_EQUIP` |
| 2 | `SELECT_TARGET` | 35 | `SEVNPC_HELLO` |
| 3 | `NORMAL_ATTACK` | 37 | `SEVNPC_SERVE` (loja, treinador, missão…) |
| 4 | `REVIVE_VILLAGE` | 39 | `GET_ALL_DATA` |
| 7 | `STOP_MOVE` | 40 | `USE_ITEM` |
| 8 | `UNSELECT` | 41, 80 | `CAST_SKILL`, `CAST_INSTANT_SKILL` |
| 9 | `GET_ITEM_INFO` | 42 | `CANCEL_ACTION` |
| 11 | `GET_IVTR_DETAIL` | 46, 47 | `SIT_DOWN`, `STAND_UP` |
| 12, 13 | `EXG_IVTR_ITEM`, `MOVE_IVTR_ITEM` | 48 | `EMOTE_ACTION` |
| 16, 17, 18 | `EXG_EQUIP_ITEM`, `EQUIP_ITEM`, `MOVE_ITEM_TO_EQUIP` | 49 | `TASK_NOTIFY` |
| 19 | `GOTO` (teleporte de GM) | 67, 68 | `QUERY_PLAYER_INFO_1`, `QUERY_NPC_INFO_1` |
| 21 | `GET_EXT_PROP` (responde `SELF_INFO_00`, `OWN_EXT_PROP`, `PLAYER_CASH`) | 75 | `ENTER_SANCTUARY` |
| 22 | `SET_STATUS_POINT` → `ADD_STATUS_POINT` (51) | 54 | `GATHER_MATERIAL` → `PLAYER_GATHER_START/STOP` (126/127), `HOST_OBTAIN_ITEM` (99) |
| 3 | `NORMAL_ATTACK` → sessão (`HOST_START_ATTACK` 84 … `HOST_STOPATTACK` 23) | 51 | `CONTINUE_ACTION` — solta a carga da habilidade |
| 27, 28, 29 | `TEAM_INVITE`, `_AGREE_`, `_REJECT_` | 85 | `SWITCH_FASHION_MODE` |
| 110 | `QUERY_CASH_INFO` | 120 | `CHECK_SECURITY_PASSWD` |
| 128 | `CALC_NETWORK_DELAY` | 6, 184 | `PICKUP`, `PICKUP_ALL` |

**Ainda no `gateway.rs` do `pw-link`:** 92 (duelo: só "preparar", sem regra), 118 (preços do
Mall, tabela vazia), 178 (waypoints). Nenhum id é tratado nos dois lados:
`pw-link/tests/subcomandos_c2s_contra_o_ir.rs::os_comandos_ja_migrados_nao_sobraram_no_gateway`
lê o `match` do mundo e cobra (B49). C2S 23–26 são `GET_EXT_PROP_BASE/MOVE/ATK/DEF`, não voo.

**Sem tratamento:** `OPEN_BOOTH` (76), `MALL_SHOPPING` (106, removido de propósito),
dividir pilha (`amount` do 13 ignorado), `pvp_mode` (79), e todo o
resto do enum (o mundo registra "subcomando ainda não tratado").

Cinco opcodes GNET do `codec.rs` seguem sem correspondência no IR (`opcodes::nao_no_ir`,
A21); o `adapter.rs` ainda é uma segunda implementação de layouts GNET (A24).
