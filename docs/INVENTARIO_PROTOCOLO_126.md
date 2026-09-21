# Inventário 1.2.6 — HEAD 2dca19e, branch versao-126

Escopo: chamadas S2C em todos os fontes de `crates/pw-gs/src` e C2S tratados no match do mundo. C2S são recebidos, não emitidos pelo gs. A árvore original contém trabalho de outra sessão e não entra nesta base.

Tamanhos de payload, **sem os 2 bytes do id**. A captura é `full_interno.pcap`, relida nesta sessão com `pw-pcapdiff --interno`. Nomes da captura são atribuídos pelo IR: o id numérico foi observado; a identidade semântica ainda requer conferir bytes/handler. Ausência de amostra não prova ausência do comando. Tamanho igual não prova campos iguais.

A coluna 1.5.5 é o tamanho nominal do IR, exceto OWN_EXT_PROP=196 (binário instalado). A coluna caminho informa se o 126 sobrescreve o método; não significa validação de seus campos. Listas e blobs continuam pendentes de comparação estrutural.

## S2C

| id / comando | Métodos chamados | 1.2.6 observado (B×vezes) | 1.5.5 nominal | Payload emitido pelo 126 na base HEAD | Mesmo id / tamanho | Caminho 126 | Chamada / evidência |
|---|---|---|---|---|---|---|---|
| 11 NPC_ENTER_SLICE | npc_enter_slice | 27×25 | variável / não fechado | npc_enter_slice: 27 | sim (numérico) / pendente (variável) | npc_enter_slice (override) | crates/pw-gs/src/bus_server.rs:627; full_interno.medidas.md:15 |
| 12 PLAYER_ENTER_SLICE | player_enter_slice | 26×7 | variável / não fechado | player_enter_slice: 26 | sim (numérico) / pendente (variável) | player_enter_slice (override) | crates/pw-gs/src/bus_server.rs:2858; full_interno.medidas.md:16 |
| 13 OBJECT_LEAVE_SLICE | object_leave_slice | 4×41 | 4 | object_leave_slice: 4 | sim (numérico) / sim | object_leave_slice (comum) | crates/pw-gs/src/bus_server.rs:2876; full_interno.medidas.md:17 |
| 14 NOTIFY_HOSTPOS | notify_hostpos | 16×7 | 20 | notify_hostpos: 20 | sim (numérico) / não | notify_hostpos (comum) | crates/pw-gs/src/bus_server.rs:361; full_interno.medidas.md:18 |
| 15 OBJECT_MOVE | object_move | 21×17294 | 21 | object_move: 21 | sim (numérico) / sim | object_move (comum) | crates/pw-gs/src/bus_server.rs:494; full_interno.medidas.md:19 |
| 18 MATTER_ENTER_WORLD | matter_enter_world | 25×33 | 25 | matter_enter_world: 25 | sim (numérico) / sim | matter_enter_world (comum) | crates/pw-gs/src/bus_server.rs:602; full_interno.medidas.md:22 |
| 19 PLAYER_LEAVE_WORLD | player_leave_world | 4×3 | 4 | player_leave_world: 4 | sim (numérico) / sim | player_leave_world (comum) | crates/pw-gs/src/bus_server.rs:2961; full_interno.medidas.md:23 |
| 20 NPC_DIED | npc_died | 8×23 | 8 | npc_died: 8 | sim (numérico) / sim | npc_died (comum) | crates/pw-gs/src/bus_server.rs:2274; full_interno.medidas.md:24 |
| 21 OBJECT_DISAPPEAR | object_disappear | 4×22 | 4 | object_disappear: 4 | sim (numérico) / sim | object_disappear (comum) | crates/pw-gs/src/bus_server.rs:556; full_interno.medidas.md:25 |
| 23 HOST_STOPATTACK | host_stop_attack | 4×29 | 4 | host_stop_attack: 4 | sim (numérico) / sim | host_stop_attack (comum) | crates/pw-gs/src/bus_server.rs:1381; full_interno.medidas.md:26 |
| 24 HOST_ATTACKRESULT | host_attack_result | 10×52 | 13 | host_attack_result: 10 | sim (numérico) / não | host_attack_result (override) | crates/pw-gs/src/bus_server.rs:1451; full_interno.medidas.md:27 |
| 25 ERROR_MESSAGE | error_message | 4×28 | 4 | error_message: 4 | sim (numérico) / sim | error_message (comum) | crates/pw-gs/src/bus_server/jogo.rs:487; full_interno.medidas.md:28 |
| 26 HOST_ATTACKED | host_attacked | 11×25 | 14 | host_attacked: 11 | sim (numérico) / não | host_attacked (override) | crates/pw-gs/src/bus_server.rs:466; full_interno.medidas.md:29 |
| 28 HOST_DIED | host_died | — | 16 | host_died: 16 | não observado / não medido | host_died (comum) | crates/pw-gs/src/bus_server.rs:525; sem amostra |
| 29 PLAYER_REVIVE | player_revive | — | 18 | player_revive: 18 | não observado / não medido | player_revive (comum) | crates/pw-gs/src/bus_server.rs:539; sem amostra |
| 30 PICKUP_MONEY | pickup_money | 4×10 | 4 | pickup_money: 4 | sim (numérico) / sim | pickup_money (comum) | crates/pw-gs/src/bus_server/jogo.rs:1136; full_interno.medidas.md:30 |
| 31 PICKUP_ITEM | pickup_item | 14×5 | 18 | pickup_item: 18 | sim (numérico) / não | pickup_item (comum) | crates/pw-gs/src/bus_server/jogo.rs:1150; full_interno.medidas.md:31 |
| 32 PLAYER_INFO_00 | player_info_00 | 24×73 | 28 | player_info_00: 24 | sim (numérico) / não | player_info_00 (override) | crates/pw-gs/src/bus_server.rs:1190; full_interno.medidas.md:32 |
| 33 NPC_INFO_00 | npc_info_00 | 12×80 | 16 | npc_info_00: 12 | sim (numérico) / não | npc_info_00 (override) | crates/pw-gs/src/bus_server.rs:549; full_interno.medidas.md:33 |
| 34 OUT_OF_SIGHT_LIST | out_of_sight_list | 8×7, 12×15, 16×7, 20×9, 24×7, 28×6, 32×6, 40×5, 44×1, 144×1, 152×1, 156×1, 356×1, 376×1 | variável / não fechado | out_of_sight_list: 4+4n | sim (numérico) / pendente (variável) | out_of_sight_list (comum) | crates/pw-gs/src/bus_server.rs:2883; full_interno.medidas.md:34 |
| 35 OBJECT_STOP_MOVE | object_stop_move | 20×2986 | 20 | object_stop_move: 20 | sim (numérico) / sim | object_stop_move (comum) | crates/pw-gs/src/bus_server.rs:362; full_interno.medidas.md:35 |
| 36 RECEIVE_EXP | receive_exp | 4×36 | 8 | receive_exp: 4 | sim (numérico) / não | receive_exp (override) | crates/pw-gs/src/bus_server/jogo.rs:1009; full_interno.medidas.md:36 |
| 37 LEVEL_UP | level_up | 4×1 | 4 | level_up: 4 | sim (numérico) / sim | level_up (comum) | crates/pw-gs/src/bus_server/jogo.rs:115; full_interno.medidas.md:37 |
| 38 SELF_INFO_00 | self_info_00 | 36×267 | 36 | self_info_00: 36 | sim (numérico) / sim | self_info_00 (comum) | crates/pw-gs/src/bus_server.rs:655; full_interno.medidas.md:38 |
| 39 UNSELECT | unselect | 0×40 | variável / não fechado | unselect: 0 | sim (numérico) / pendente (variável) | unselect (comum) | crates/pw-gs/src/bus_server.rs:1169; full_interno.medidas.md:39 |
| 40 OWN_ITEM_INFO | item_info | 34×1, 74×1, 90×1, 98×1, 106×1, 134×1, 146×1, 158×1 | variável / não fechado | item_info: 22+blob | sim (numérico) / pendente (variável) | item_info (comum) | crates/pw-gs/src/bus_server.rs:3703; full_interno.medidas.md:40 |
| 42 OWN_IVTR_DATA | own_ivtr_from_items | 134×3 | variável / não fechado | own_ivtr_from_items: 6+4×slots+8×ocupados | sim (numérico) / pendente (variável) | own_ivtr_from_items (comum) | crates/pw-gs/src/bus_server.rs:3482; full_interno.medidas.md:41 |
| 44 EXG_IVTR_ITEM | exg_ivtr_item | 2×1 | 2 | exg_ivtr_item: 2 | sim (numérico) / sim | exg_ivtr_item (comum) | crates/pw-gs/src/bus_server.rs:3796; full_interno.medidas.md:43 |
| 45 MOVE_IVTR_ITEM | move_ivtr_item | — | 6 | move_ivtr_item: 6 | não observado / não medido | move_ivtr_item (comum) | crates/pw-gs/src/bus_server.rs:3844; sem amostra |
| 46 PLAYER_DROP_ITEM | player_drop_item | 9×7 | 11 | player_drop_item: 11 | sim (numérico) / não | player_drop_item (comum) | crates/pw-gs/src/bus_server/jogo.rs:192; full_interno.medidas.md:44 |
| 47 EXG_EQUIP_ITEM | exg_equip_item | — | 2 | exg_equip_item: 2 | não observado / não medido | exg_equip_item (comum) | crates/pw-gs/src/bus_server.rs:3795; sem amostra |
| 48 EQUIP_ITEM | equip_item | 6×9 | 10 | equip_item: 6 | sim (numérico) / não | equip_item (override) | crates/pw-gs/src/bus_server.rs:3896; full_interno.medidas.md:45 |
| 49 MOVE_EQUIP_ITEM | move_item_to_equip | — | 6 | move_item_to_equip: 6 | não observado / não medido | move_item_to_equip (comum) | crates/pw-gs/src/bus_server.rs:3953; sem amostra |
| 50 OWN_EXT_PROP | own_ext_prop | 152×15 | 196 | own_ext_prop: 152 | sim (numérico) / não | own_ext_prop (override) | crates/pw-gs/src/bus_server/jogo.rs:418; full_interno.medidas.md:46 |
| 51 ADD_STATUS_POINT | add_status_point | 20×1 | 20 | add_status_point: 20 | sim (numérico) / sim | add_status_point (comum) | crates/pw-gs/src/bus_server/jogo.rs:774; full_interno.medidas.md:47 |
| 52 SELECT_TARGET | select_target | 4×47 | 4 | select_target: 4 | sim (numérico) / sim | select_target (comum) | crates/pw-gs/src/bus_server.rs:1201; full_interno.medidas.md:48 |
| 54 PLAYER_EXT_PROP_MOVE | ext_prop_move | 20×2 | 20 | ext_prop_move: 20 | sim (numérico) / sim | ext_prop_move (comum) | crates/pw-gs/src/bus_server/habilidades.rs:597; full_interno.medidas.md:49 |
| 57 TEAM_LEADER_INVITE | team_leader_invite | 10×2 | 10 | team_leader_invite: 10 | sim (numérico) / sim | team_leader_invite (comum) | crates/pw-gs/src/bus_server.rs:1539; full_interno.medidas.md:50 |
| 59 TEAM_JOIN_TEAM | team_join_party | 6×3 | 6 | team_join_party: 6 | sim (numérico) / sim | team_join_party (comum) | crates/pw-gs/src/bus_server.rs:1577; full_interno.medidas.md:51 |
| 60 TEAM_MEMBER_LEAVE | team_member_leave | 10×2 | 10 | team_member_leave: 10 | sim (numérico) / sim | team_member_leave (comum) | crates/pw-gs/src/bus_server.rs:1625; full_interno.medidas.md:52 |
| 61 TEAM_LEAVE_PARTY | team_leave_party | 6×3 | 6 | team_leave_party: 6 | sim (numérico) / sim | team_leave_party (comum) | crates/pw-gs/src/bus_server.rs:1618; full_interno.medidas.md:53 |
| 64 TEAM_MEMBER_DATA | team_member_data | 31×30, 56×16, 81×7 | variável / não fechado | team_member_data: 6+34n | sim (numérico) / pendente (variável) | team_member_data (comum) | crates/pw-gs/src/bus_server.rs:1580; full_interno.medidas.md:55 |
| 66 EQUIP_DATA | equip_data | 14×2, 18×1, 22×2, 62×1, 66×2 | variável / não fechado | equip_data: 14+4n (incorreto no 126; B74 corrigiu para 10+4n) | sim (numérico) / pendente (variável) | equip_data (comum) | crates/pw-gs/src/bus_server.rs:3381; full_interno.medidas.md:56 |
| 68 EQUIP_DAMAGED | equip_damaged | — | 2 | equip_damaged: 2 | não observado / não medido | equip_damaged (comum) | crates/pw-gs/src/bus_server/jogo.rs:754; sem amostra |
| 70 NPC_GREETING | npc_greeting | 4×18 | 4 | npc_greeting: 4 | sim (numérico) / sim | npc_greeting (comum) | crates/pw-gs/src/bus_server.rs:3034; full_interno.medidas.md:59 |
| 72 PURCHASE_ITEM | purchase_item | 20×1 | variável / não fechado | purchase_item: 11+15n | sim (numérico) / pendente (variável) | purchase_item (comum) | crates/pw-gs/src/bus_server/jogo.rs:1208; full_interno.medidas.md:60 |
| 73 ITEM_TO_MONEY | item_to_money | 14×1 | 14 | item_to_money: 14 | sim (numérico) / sim | item_to_money (comum) | crates/pw-gs/src/bus_server/jogo.rs:1237; full_interno.medidas.md:61 |
| 74 REPAIR_ALL | repair_all | 4×1 | 4 | repair_all: 4 | sim (numérico) / sim | repair_all (comum) | crates/pw-gs/src/bus_server.rs:3149; full_interno.medidas.md:62 |
| 77 SPEND_MONEY | spend_money | 4×4 | 4 | spend_money: 4 | sim (numérico) / sim | spend_money (comum) | crates/pw-gs/src/bus_server/jogo.rs:202; full_interno.medidas.md:64 |
| 82 GET_OWN_MONEY | get_own_money | 8×5 | 12 | get_own_money: 8 | sim (numérico) / não | get_own_money (comum) | crates/pw-gs/src/bus_server/jogo.rs:359; full_interno.medidas.md:65 |
| 83 ATTACK_ONCE | attack_once | 1×71 | 1 | attack_once: 1 | sim (numérico) / sim | attack_once (comum) | crates/pw-gs/src/bus_server/habilidades.rs:198; full_interno.medidas.md:66 |
| 84 HOST_START_ATTACK | host_start_attack | 7×29 | 7 | host_start_attack: 7 | sim (numérico) / sim | host_start_attack (comum) | crates/pw-gs/src/bus_server.rs:1339; full_interno.medidas.md:67 |
| 85 OBJECT_CAST_SKILL | object_cast_skill | 15×19 | 15 | object_cast_skill: 15 | sim (numérico) / sim | object_cast_skill (comum) | crates/pw-gs/src/bus_server.rs:1895; full_interno.medidas.md:68 |
| 86 SKILL_INTERRUPTED | skill_interrupted | 4×1 | 4 | skill_interrupted: 4 | sim (numérico) / sim | skill_interrupted (comum) | crates/pw-gs/src/bus_server.rs:2000; full_interno.medidas.md:69 |
| 87 SELF_SKILL_INTERRUPTED | self_skill_interrupted | 1×1 | 1 | self_skill_interrupted: 1 | sim (numérico) / sim | self_skill_interrupted (comum) | crates/pw-gs/src/bus_server.rs:1999; full_interno.medidas.md:70 |
| 88 SKILL_PERFORM | skill_perform | 0×18 | variável / não fechado | skill_perform: 0 | sim (numérico) / pendente (variável) | skill_perform (comum) | crates/pw-gs/src/bus_server.rs:2093; full_interno.medidas.md:71 |
| 90 SKILL_DATA | skill_data_from_records | 14×2, 74×1 | variável / não fechado | skill_data_from_records: 4+5n | sim (numérico) / pendente (variável) | skill_data_from_records (comum) | crates/pw-gs/src/bus_server.rs:3557; full_interno.medidas.md:72 |
| 91 HOST_USE_ITEM | host_use_item | 8×11 | 8 | host_use_item: 8 | sim (numérico) / sim | host_use_item (comum) | crates/pw-gs/src/bus_server.rs:1725; full_interno.medidas.md:73 |
| 94 COST_SKILL_POINT | cost_skill_point | 4×1 | 4 | cost_skill_point: 4 | sim (numérico) / sim | cost_skill_point (comum) | crates/pw-gs/src/bus_server/jogo.rs:1386; full_interno.medidas.md:75 |
| 95 LEARN_SKILL | learn_skill | 8×1 | 8 | learn_skill: 8 | sim (numérico) / sim | learn_skill (comum) | crates/pw-gs/src/bus_server/jogo.rs:1390; full_interno.medidas.md:76 |
| 96 OBJECT_TAKEOFF | object_takeoff | 4×5 | 4 | object_takeoff: 4 | sim (numérico) / sim | object_takeoff (comum) | crates/pw-gs/src/bus_server.rs:2991; full_interno.medidas.md:77 |
| 97 OBJECT_LANDING | object_landing | 4×4 | 4 | object_landing: 4 | sim (numérico) / sim | object_landing (comum) | crates/pw-gs/src/bus_server.rs:2993; full_interno.medidas.md:78 |
| 99 HOST_OBTAIN_ITEM | obtain_item | 14×3 | 18 | obtain_item: 18 | sim (numérico) / não | obtain_item (comum) | crates/pw-gs/src/bus_server/jogo.rs:600; full_interno.medidas.md:79 |
| 105 TASK_DATA | task_data_com_listas | 26×1, 60×1, 284×1 | variável / não fechado | task_data_com_listas: 12+soma dos 3 blocos | sim (numérico) / pendente (variável) | task_data_com_listas (override) | crates/pw-gs/src/bus_server.rs:3659; full_interno.medidas.md:85 |
| 106 TASK_VAR_DATA | task_dyn_time_mark, task_dyn_data, task_notify_error, task_notify_new, task_notify_base, task_notify_complete, task_notify_monster_killed | 7×1, 13×18, 14×5, 15×3, 18×7, 2455×3 | variável / não fechado | task_dyn_time_mark: 13; task_dyn_data: 7+fragmento; task_notify_error: 11; task_notify_new: 15+tags; task_notify_base: 7; task_notify_complete: 11+tags; task_notify_monster_killed: 21 | sim (numérico) / pendente (variável) | task_dyn_time_mark (comum), task_dyn_data (comum), task_notify_error (comum), task_notify_new (comum), task_notify_base (comum), task_notify_complete (comum), task_notify_monster_killed (comum) | crates/pw-gs/src/bus_server.rs:3075; crates/pw-gs/src/bus_server.rs:3098; crates/pw-gs/src/missoes.rs:842; crates/pw-gs/src/missoes.rs:1385; crates/pw-gs/src/missoes.rs:1913; crates/pw-gs/src/missoes.rs:1927; crates/pw-gs/src/missoes.rs:2026; full_interno.medidas.md:86 |
| 111 OBJECT_SIT_DOWN | object_sit_down | 4×2 | 4 | object_sit_down: 4 | sim (numérico) / sim | object_sit_down (comum) | crates/pw-gs/src/bus_server.rs:1493; full_interno.medidas.md:88 |
| 112 OBJECT_STAND_UP | object_stand_up | 4×2 | 4 | object_stand_up: 4 | sim (numérico) / sim | object_stand_up (comum) | crates/pw-gs/src/bus_server.rs:1495; full_interno.medidas.md:89 |
| 113 OBJECT_DO_EMOTE | object_do_emote | — | 6 | object_do_emote: 6 | não observado / não medido | object_do_emote (comum) | crates/pw-gs/src/bus_server.rs:1511; sem amostra |
| 123 HOST_STOP_SKILL | self_stop_skill | 0×19 | variável / não fechado | self_stop_skill: 0 | sim (numérico) / pendente (variável) | self_stop_skill (comum) | crates/pw-gs/src/bus_server.rs:1829; full_interno.medidas.md:92 |
| 124 UPDATE_EXT_STATE | update_ext_state | — | 28 | update_ext_state: 28 | não observado / não medido | update_ext_state (comum) | crates/pw-gs/src/bus_server/habilidades.rs:600; sem amostra |
| 125 ICON_STATE_NOTIFY | icon_state_notify | — | variável / não fechado | icon_state_notify: 8+6n | não observado / não medido | icon_state_notify (comum) | crates/pw-gs/src/bus_server/habilidades.rs:601; sem amostra |
| 126 PLAYER_GATHER_START | player_gather_start | 9×1 | 9 | player_gather_start: 9 | sim (numérico) / sim | player_gather_start (comum) | crates/pw-gs/src/bus_server/jogo.rs:493; full_interno.medidas.md:93 |
| 127 PLAYER_GATHER_STOP | player_gather_stop | 4×1 | 4 | player_gather_stop: 4 | sim (numérico) / sim | player_gather_stop (comum) | crates/pw-gs/src/bus_server/jogo.rs:517; full_interno.medidas.md:94 |
| 139 ENCHANT_RESULT | enchant_result | — | 19 | enchant_result: 19 | não observado / não medido | enchant_result (comum) | crates/pw-gs/src/bus_server/habilidades.rs:282; sem amostra |
| 142 HOST_SKILL_ATTACK_RESULT | self_skill_attack_result | 14×18 | 18 | self_skill_attack_result: 14 | sim (numérico) / não | self_skill_attack_result (override) | crates/pw-gs/src/bus_server.rs:2256; full_interno.medidas.md:101 |
| 143 OBJECT_SKILL_ATTACK_RESULT | object_skill_attack_result | 18×18 | 22 | object_skill_attack_result: 18 | sim (numérico) / não | object_skill_attack_result (override) | crates/pw-gs/src/bus_server.rs:2453; full_interno.medidas.md:102 |
| 144 HOST_SKILL_ATTACKED | host_skill_attacked | 15×1 | 19 | host_skill_attacked: 19 | sim (numérico) / não | host_skill_attacked (comum) | crates/pw-gs/src/bus_server.rs:2489; full_interno.medidas.md:103 |
| 152 MATTER_PICKUP | matter_pickup | 8×12 | 8 | matter_pickup: 8 | sim (numérico) / sim | matter_pickup (comum) | crates/pw-gs/src/bus_server/jogo.rs:1167; full_interno.medidas.md:105 |
| 156 TASK_DELIVER_ITEM | task_deliver_item | 10×2 | 18 | task_deliver_item: 18 | sim (numérico) / não | task_deliver_item (comum) | crates/pw-gs/src/bus_server/jogo.rs:184; full_interno.medidas.md:106 |
| 158 TASK_DELIVER_EXP | task_deliver_exp | 8×2 | 8 | task_deliver_exp: 8 | sim (numérico) / sim | task_deliver_exp (comum) | crates/pw-gs/src/bus_server/jogo.rs:207; full_interno.medidas.md:108 |
| 159 TASK_DELIVER_MONEY | task_deliver_money | 8×1 | 8 | task_deliver_money: 8 | sim (numérico) / sim | task_deliver_money (comum) | crates/pw-gs/src/bus_server/jogo.rs:197; full_interno.medidas.md:109 |
| 160 TASK_DELIVER_LEVEL2 | task_deliver_level2 | — | 8 | task_deliver_level2: 8 | não observado / não medido | task_deliver_level2 (comum) | crates/pw-gs/src/bus_server/jogo.rs:229; sem amostra |
| 164 ENTER_SANCTUARY | enter_sanctuary | 0×11 | 4 | enter_sanctuary: 0 | sim (numérico) / não | enter_sanctuary (override) | crates/pw-gs/src/bus_server.rs:995; full_interno.medidas.md:111 |
| 177 HOST_CORRECT_POS | host_correct_pos | — | 14 | host_correct_pos: 14 | não observado / não medido | host_correct_pos (comum) | crates/pw-gs/src/bus_server.rs:2617; sem amostra |
| 179 ACTIVATE_WAYPOINT | activate_waypoint | — | 2 | activate_waypoint: 2 | não observado / não medido | activate_waypoint (comum) | crates/pw-gs/src/bus_server.rs:2040; sem amostra |
| 180 WAYPOINT_LIST | player_waypoint_list | 6×2, 100×1 | variável / não fechado | player_waypoint_list: 4+2n | sim (numérico) / pendente (variável) | player_waypoint_list (comum) | crates/pw-gs/src/bus_server.rs:3589; full_interno.medidas.md:116 |
| 181 UNFREEZE_IVTR_SLOT | unfreeze_ivtr_slot | 3×24 | 3 | unfreeze_ivtr_slot: 3 | sim (numérico) / sim | unfreeze_ivtr_slot (comum) | crates/pw-gs/src/bus_server.rs:1731; full_interno.medidas.md:117 |
| 192 PLAYER_ENABLE_FASHION | player_enable_fashion | 5×2 | 5 | player_enable_fashion: 5 | sim (numérico) / sim | player_enable_fashion (comum) | crates/pw-gs/src/bus_server.rs:2306; full_interno.medidas.md:122 |
| 198 SET_COOLDOWN | set_cooldown | 8×22 | 8 | set_cooldown: 8 | sim (numérico) / sim | set_cooldown (comum) | crates/pw-gs/src/bus_server.rs:1869; full_interno.medidas.md:123 |
| 231 GAIN_PET | gain_pet | — | 196 | gain_pet: 4+blob (192 na chamada) | não observado / não medido | gain_pet (comum) | crates/pw-gs/src/bus_server/jogo.rs:1500; sem amostra |
| 239 PET_ROOM | pet_room | 2×2, 198×1 | 2 | pet_room: 2+blob | sim (numérico) / não | pet_room (comum) | crates/pw-gs/src/bus_server.rs:3619; full_interno.medidas.md:138 |
| 240 PET_ROOM_CAPACITY | pet_room_capacity | 4×4 | 4 | pet_room_capacity: 4 | sim (numérico) / sim | pet_room_capacity (comum) | crates/pw-gs/src/bus_server.rs:3601; full_interno.medidas.md:139 |
| 253 PLAYER_CASH | player_cash | 4×4 | 4 | player_cash: 4 | sim (numérico) / sim | player_cash (comum) | crates/pw-gs/src/bus_server.rs:3239; full_interno.medidas.md:141 |
| 277 SECURITY_PASSWD_CHECKED | security_passwd_checked | — | variável / não fechado | security_passwd_checked: 0 | não observado / não medido | security_passwd_checked (comum) | crates/pw-gs/src/bus_server.rs:3445; sem amostra |
| 279 PLAYER_HP_STEAL | player_hp_steal | — | 4 | player_hp_steal: 4 | não observado / não medido | player_hp_steal (comum) | crates/pw-gs/src/bus_server.rs:2419; sem amostra |
| 291 CALC_NETWORK_DELAY_RE | calc_network_delay_re | — | 4 | calc_network_delay_re: 4 | não observado / não medido | calc_network_delay_re (comum) | crates/pw-gs/src/bus_server.rs:1143; sem amostra |
| 363 QUERY_TITLE_RE | query_title_re | — | variável / não fechado | query_title_re: 12+2n+6m | não observado / não medido | query_title_re (comum) | crates/pw-gs/src/bus_server.rs:1011; sem amostra |
| 390 SCENE_SERVICE_NPC_LIST | scene_service_npc_list | — | variável / não fechado | scene_service_npc_list: 4+8n (B74: não emitido no 126) | não observado / não medido | scene_service_npc_list (comum) | crates/pw-gs/src/bus_server.rs:3571; sem amostra |

## C2S

A referência nominal é o IR 1.5.5; TASK_NOTIFY contém envelope e mensagem variável, portanto 7 observado contra 4 de base não demonstra incompatibilidade por si só.

| id / comando | 1.2.6 observado | 1.5.5 nominal | Mesmo id / tamanho | Handler / evidência |
|---|---|---|---|---|
| 0 PLAYER_MOVE | 31×472 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:939; full_interno.medidas.md:151 |
| 1 LOGOUT | 4×3 | 4 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:943; full_interno.medidas.md:152 |
| 2 SELECT_TARGET | 4×47 | 4 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:944; full_interno.medidas.md:153 |
| 3 NORMAL_ATTACK | 1×32 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:950; full_interno.medidas.md:154 |
| 4 REVIVE_VILLAGE | — | 4 | não observado / não medido | crates/pw-gs/src/bus_server.rs:951; sem amostra |
| 6 PICKUP | 8×39 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:953; full_interno.medidas.md:155 |
| 7 STOP_MOVE | 20×148 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:946; full_interno.medidas.md:156 |
| 8 UNSELECT | 0×8 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:945; full_interno.medidas.md:157 |
| 9 GET_ITEM_INFO | 2×6 | 2 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:952; full_interno.medidas.md:158 |
| 11 GET_IVTR_DETAIL | — | 1 | não observado / não medido | crates/pw-gs/src/bus_server.rs:955; sem amostra |
| 12 EXG_IVTR_ITEM | 2×1 | 2 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:956; full_interno.medidas.md:159 |
| 13 MOVE_IVTR_ITEM | — | 6 | não observado / não medido | crates/pw-gs/src/bus_server.rs:958; sem amostra |
| 16 EXG_EQUIP_ITEM | — | 2 | não observado / não medido | crates/pw-gs/src/bus_server.rs:957; sem amostra |
| 17 EQUIP_ITEM | 2×9 | 2 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:959; full_interno.medidas.md:161 |
| 18 MOVE_ITEM_TO_EQUIP | — | 2 | não observado / não medido | crates/pw-gs/src/bus_server.rs:960; sem amostra |
| 19 GOTO | 12×7 | 12 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:989; full_interno.medidas.md:162 |
| 21 GET_EXT_PROP | 0×12 | 0 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:998; full_interno.medidas.md:163 |
| 22 SET_STATUS_POINT | 16×1 | 16 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:999; full_interno.medidas.md:164 |
| 27 TEAM_INVITE | 4×2 | 4 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:984; full_interno.medidas.md:165 |
| 28 TEAM_AGREE_INVITE | 8×2 | 8 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:985; full_interno.medidas.md:166 |
| 29 TEAM_REJECT_INVITE | — | 4 | não observado / não medido | crates/pw-gs/src/bus_server.rs:986; sem amostra |
| 30 TEAM_LEAVE_PARTY | 0×2 | 0 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:987; full_interno.medidas.md:167 |
| 33 GET_OTHER_EQUIP | 6×6, 10×1 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:1006; full_interno.medidas.md:168 |
| 35 SEVNPC_HELLO | 4×18 | 4 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:980; full_interno.medidas.md:169 |
| 37 SEVNPC_SERVE | 12×6, 14×2, 16×3, 20×4, 24×1, 28×1 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:979; full_interno.medidas.md:170 |
| 39 GET_ALL_DATA | 3×3 | 3 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:1003; full_interno.medidas.md:171 |
| 40 USE_ITEM | 8×11 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:983; full_interno.medidas.md:172 |
| 41 CAST_SKILL | 10×19 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:990; full_interno.medidas.md:173 |
| 42 CANCEL_ACTION | 0×15 | 0 | sim (numérico) / sim | crates/pw-gs/src/bus_server.rs:963; full_interno.medidas.md:174 |
| 46 SIT_DOWN | 0×2 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:961; full_interno.medidas.md:175 |
| 47 STAND_UP | 0×2 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:962; full_interno.medidas.md:176 |
| 48 EMOTE_ACTION | — | 2 | não observado / não medido | crates/pw-gs/src/bus_server.rs:978; sem amostra |
| 49 TASK_NOTIFY | 7×12 | 4 | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:981; full_interno.medidas.md:177 |
| 51 CONTINUE_ACTION | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:1000; sem amostra |
| 54 GATHER_MATERIAL | 16×1 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:1001; full_interno.medidas.md:179 |
| 67 QUERY_PLAYER_INFO_1 | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:1004; sem amostra |
| 68 QUERY_NPC_INFO_1 | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:1005; sem amostra |
| 75 ENTER_SANCTUARY | 0×14 | 4 | sim (numérico) / não | crates/pw-gs/src/bus_server.rs:993; full_interno.medidas.md:183 |
| 80 CAST_INSTANT_SKILL | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:990; sem amostra |
| 85 SWITCH_FASHION_MODE | 0×2 | não fechado | sim (numérico) / pendente (variável) | crates/pw-gs/src/bus_server.rs:988; full_interno.medidas.md:187 |
| 110 QUERY_CASH_INFO | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:1002; sem amostra |
| 120 CHECK_SECURITY_PASSWD | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:982; sem amostra |
| 128 CALC_NETWORK_DELAY | — | 4 | não observado / não medido | crates/pw-gs/src/bus_server.rs:1007; sem amostra |
| 154 QUERY_TITLE | — | 4 | não observado / não medido | crates/pw-gs/src/bus_server.rs:1008; sem amostra |
| 178 ACTIVATE_REGION_WAYPOINTS | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:1013; sem amostra |
| 184 PICKUP_ALL | — | não fechado | não observado / não medido | crates/pw-gs/src/bus_server.rs:954; sem amostra |

## Divergências em uso já localizadas

- S2C 14: `notify_hostpos` comum escreve 20 B, captura 16 B (teleporte).
- S2C 31/99: coleta/obtenção comuns escrevem 18 B, captura 14 B.
- S2C 46: descarte comum escreve 11 B, captura 9 B.
- S2C 64: grupo comum escreve 6+n×34 B, captura 6+n×25 B.
- S2C 72: compra comum escreve 11+n×15 B; a amostra de compra tem 20 B. Precisa fechar cabeçalho/itens.
- S2C 144: habilidade recebida comum escreve 19 B, captura 15 B.
- S2C 156: prêmio de item comum escreve 18 B, captura 10 B.
- S2C 390: método padrão do trait é enviado no GET_ALL_DATA; não observado na captura. Verificar existência no cliente 126 antes de emitir.

## Limites da etapa

Este inventário não declara jogabilidade nem regressão aprovada. Não houve alteração de código de jogo. OWN_ITEM_INFO, OWN_IVTR_DATA, EQUIP_DATA, SKILL_DATA, TASK_DATA e TASK_VAR_DATA exigem comparação campo a campo; não basta a lista de comprimentos. Cadência de combate ainda precisa de timestamps, que o relatório atual não imprime.

## Complemento após conferir o binário (B74)

O validador em VA 0x584610 rejeita ids >260. Assim 277, 291, 363 e 390 da tabela não são reconhecidos por este binário. A tabela preserva o inventário da base anterior à correção; B74 corrigiu EQUIP_DATA e suprimiu o 390 e os avisos tardios de entrada. Outros ids tardios ainda ficam nas respostas a pedidos que o cliente 126 normal não faz.

A coluna de emissão foi extraída dos escritores da base HEAD (escalares somados sem o id; laços/blobs explicitados). A referência continua sendo captura e binário, não o escritor. `n/m` são contagens de entradas; nos itens comuns o blob inclui a essência do item.
