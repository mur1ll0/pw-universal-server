# Especificação 04: Protocolo do mundo 3D (subcomandos do `GamedataSend`)

> Verificada contra o código em 2026-09-14, commit `e6433ae` + B49. Cobre
> `crates/pw-protocol/src/{packets,por_versao.rs,opcodes.rs}`, `crates/pw-wire/`,
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

## 4. Diferenças por versão: `PorVersao`

A base `S2CGamedataSend::*` escreve um layout; `pw_protocol::PorVersao` escolhe a variante
pela versão do realm. Um caminho de escrita por layout.

| codificador | 1.2.6 | 1.5.x | o que muda |
| :--- | ---: | ---: | :--- |
| `task_data` | 12 | 25 | 3 → **5** tamanhos (`finished_count`, `storage_task`); com 3 o cliente 1.5.5 crasha na renderização (B14) |
| `npc_enter_world` / `npc_enter_slice` (`info_npc`) | 27 | 35 | `vis_tid`, `state2` |
| `self_info_1` | 34 | 38 | `state2`; sem ele, 30 s de "entrando" e desconexão |
| `player_enter_world` / `player_enter_slice` (`info_player_1`) | 26 | 30 | `state2` |
| `get_own_money` | 8 | 12 | |
| `inst_data_checkout` | 20 | 24 | |
| 5 resultados de ataque (`host_attack_result`, `host_attacked`, `self_skill_attack_result`, `object_skill_attack_result`…) | −3/−4 | | `attack_flag` era `char` e virou `int`; o `section` falta no 1.2.6 |
| `npc_info_00`, `player_info_00` | | | ganham `iTargetID` |
| `enter_sanctuary`, `leave_sanctuary` | 0 | | ganham `id` |
| `receive_exp`, contagens do `equip_item` | 16 bits | 32 bits | |
| `equip_data`, `object_move`, `object_stop_move` | | | ver `por_versao.rs` |

Uma captura de 1.2.6 mediu 175 comandos: 106 idênticos ao 1.5.3, **32 diferentes**
(`docs/MEDIDAS_DO_126.md`). Só entra aqui diferença **medida**.

## 5. Fatos de layout que não são óbvios

| comando | fato | origem |
| :--- | :--- | :--- |
| `info_player_1` | `cid, pos, crc_e, crc_c, dir, level2, state, state2` — **sem `world_tag`**. Sexo = bit `0x40` do `state2` (zero = homem); `crc_c` tem de ser igual ao `custom_stamp` do `PlayerBaseInfo_Re`; `level2` leva o `sec_level` e o GM acende `STATE_GAMEMASTER 0x4000` | `EC_GPDataType.h:603,709`; `gs/player_imp.h:1886` (B42b) |
| entrada por streaming | jogador `PLAYER_ENTER_SLICE` (12), NPC `NPC_ENTER_SLICE` (11); o 17 é para quem **surgiu** (efeito de teleporte) | `EC_ManPlayer.cpp:1845` |
| saída | criatura/jogador fora de alcance `OBJECT_LEAVE_SLICE` (13); matéria só sai por `OUT_OF_SIGHT_LIST` (34); jogador que saiu do jogo `PLAYER_LEAVE_WORLD` (19) | `EC_GameDataPrtc.cpp:891,1056` |
| `MATTER_ENTER_WORLD` (18) | 25 bytes: `mid, tid, pos, dir0, dir1, rad, state, value`; `state = 0` recurso comum | `EC_GPDataType.h:784` |
| `OBJECT_MOVE` | `use_time` em **milissegundos**, `speed` = velocidade × **256**, `move_mode` com bit do habitat | `gs/npcsession.cpp:258`, `gs/petnpc.cpp:350` (B48) |
| `OWN_EXT_PROP` | **196** bytes no binário; `attack_speed` em ticks de 50 ms; velocidades do `CHARRACTER_CLASS_CONFIG` | B34a, B44b2 |
| `OWN_ITEM_INFO` (40) | arma/armadura/acessório **precisam** do bloco de dados, senão `CanUseEquipment` recusa (item vermelho). Cabeçalho `prerequisition` (vitalidade antes de agilidade, máscara de classe em 16 bits) + essência: arma 44, armadura 36 (`defesa, evasão, +MP, +HP, resist[5]`), acessório 36 (`dano, dano mágico, defesa, evasão, resist[5]`). Subtipo **não** viaja | `generate_item_temp.h:490-556,772-830` (B41a) |
| `SERVER_TIME` (114) | `lua_version` = primeira linha do `global_api.lua` (1.5.5: 102); errado encerra o cliente | B9c |
| `ACTIVATE_REGION_WAYPOINTS` (C2S 178) | responder `WAYPOINT_LIST` (S2C 180) com os ids recebidos; sem isso o cliente reenvia a cada quadro | B9b |
| `GetUIConfig_Re` (GNET 105) | vazio é válido; o cabeçalho de 16 bytes só com dado real depois | B7 |
| `CALC_NETWORK_DELAY` (C2S 128) | `_RE` (S2C 291) devolve o `timestamp` intacto; só alimenta o indicador de ping | B42i |
| `GP_NPCSEV_LEARN` | pedido só com `idSkill`; resposta `LEARN_SKILL` (95) com id e nível | `EC_SendC2SCmds.cpp:3379` |
| `NORMAL_ATTACK` (C2S 3) | 3 bytes, **sem id de alvo** — o alvo é o selecionado | A33 |
| `TASK_NOTIFY` (C2S 49) | `size` + `task_notify_base { u8 reason, u16 task }`. Os `reason` do cliente (`TASK_CLT_NOTIFY_*`) e os do servidor (`TASK_SVR_NOTIFY_*`) são **tabelas diferentes**: o 7 do cliente é "me dê a marca dinâmica", o 7 do servidor é "esqueça a habilidade de produção" | `task/TaskTempl.h:81-122` (B49) |
| marca das missões dinâmicas | resposta ao `reason` 7: `TASK_VAR_DATA` (106) com `svr_task_dyn_time_mark` = `reason 8, task 0, time_mark u32, version u16 = 10` (9 bytes); sem marca, sem resposta | `TaskTemplMan.cpp:299-309`, `TaskClient.cpp:290` |
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
| 21 | `GET_EXT_PROP` | 75 | `ENTER_SANCTUARY` |
| 27, 28, 29 | `TEAM_INVITE`, `_AGREE_`, `_REJECT_` | 85 | `SWITCH_FASHION_MODE` |
| 110 | `QUERY_CASH_INFO` | 120 | `CHECK_SECURITY_PASSWD` |
| 128 | `CALC_NETWORK_DELAY` | | |

**Ainda no `gateway.rs` do `pw-link`:** 92 (duelo: só "preparar", sem regra), 118 (preços do
Mall, tabela vazia), 178 (waypoints). Nenhum id é tratado nos dois lados:
`pw-link/tests/subcomandos_c2s_contra_o_ir.rs::os_comandos_ja_migrados_nao_sobraram_no_gateway`
lê o `match` do mundo e cobra (B49). C2S 23–26 são `GET_EXT_PROP_BASE/MOVE/ATK/DEF`, não voo.

**Sem tratamento:** `OPEN_BOOTH` (76), `MALL_SHOPPING` (106, removido de propósito),
`MATTER_PICKUP` (152), dividir pilha (`amount` do 13 ignorado), `pvp_mode` (79), e todo o
resto do enum (o mundo registra "subcomando ainda não tratado").

Cinco opcodes GNET do `codec.rs` seguem sem correspondência no IR (`opcodes::nao_no_ir`,
A21); o `adapter.rs` ainda é uma segunda implementação de layouts GNET (A24).
