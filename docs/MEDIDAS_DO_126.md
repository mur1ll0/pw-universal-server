# O que o 1.2.6 mede, de verdade

> Saída do `pw-pcapdiff` sobre a captura do elo interno (`gs -> glinkd`) de um servidor
> 1.2.6 em funcionamento — sessão de 22 minutos, roteiro de 45 passos, 2026-09-01.
> **67.482 pacotes, 0 descartados pelo kernel, 22.217 quadros GNET lidos, 0 buracos.**
> Gerada pela ferramenta, não escrita à mão.
>
> ```
> cargo run -p pw-pcapdiff -- full_interno.pcap --interno
> ```

## O resultado

**175 comandos medidos. 106 batem com o 1.5.3, 32 diferem.**

E as diferenças não são ruído: elas se agrupam.

### O padrão `attack_flag`: `char` no 1.2.6, `int` no 1.5.3

| Comando | 1.2.6 | 1.5.3 | Δ |
| :--- | ---: | ---: | ---: |
| `HOST_ATTACKRESULT` (24) | 10 | 13 | −3 |
| `HOST_ATTACKED` (26) | 11 | 14 | −3 |
| `OBJECT_ATTACK_RESULT` (120) | 14 | 17 | −3 |

Três comandos independentes, o mesmo −3. E nos de habilidade o −4 fecha a conta com o
`section` a mais:

| `HOST_SKILL_ATTACK_RESULT` (142) | 14 | 18 | −4 |
| `OBJECT_SKILL_ATTACK_RESULT` (143) | 18 | 22 | −4 |
| `HOST_SKILL_ATTACKED` (144) | 15 | 19 | −4 |

`4+4+4+1(flag)+1(speed) = 14` — exatamente o observado, sem `section`.

### Um `int` a mais no 1.5.3

`NOTIFY_HOSTPOS` (14), `PICKUP_ITEM` (31), `PLAYER_INFO_00` (32), `NPC_INFO_00` (33),
`RECEIVE_EXP` (36), `EQUIP_ITEM` (48), `HOST_OBTAIN_ITEM` (99), `PRODUCE_ONCE` (101),
`INST_DATA_CHECKOUT` (206), `SUMMON_PET` (233) — todos −4.

`ENTER_SANCTUARY` (164) e `LEAVE_SANCTUARY` (165) vão além: **0 bytes no 1.2.6**, contra 4
no 1.5.3. Não têm o `id`.

### Um byte a mais no 1.5.3

`TRASHBOX_CLOSE` (131), `TRASHBOX_WEALTH` (132), `EXG_TRASHBOX_ITEM` (133),
`PLAYER_MOUNTING` (227), `RECALL_PET` (234), e do lado C2S `GET_TRASHBOX_INFO` (55) e
`EXG_TRASHBOX_ITEM` (56).

### Os avulsos

`TRASHBOX_OPEN` (130) −5, `TASK_DELIVER_ITEM` (156) −8, `OWN_EXT_PROP` (50) **−36** (nove
`int`: os atributos que o 1.2.6 não tem), `PLAYER_DROP_ITEM` (46) −2, `DROP_IVTR_ITEM`
(14, C2S) −2.

E **um único maior no 1.2.6**: `TASK_NOTIFY` (49, C2S), 7 contra 4.

## O `TEAM_MEMBER_DATA`, resolvido pela aritmética

Três tamanhos observados: **31, 56, 81**. Passo constante de 25, e `31 % 25 = 6`. A
ferramenta decompõe sozinha:

```
| 64 | TEAM_MEMBER_DATA | 31×30, 56×16, 81×7 | — | lista: 6 + n×25 |
```

- **O cabeçalho tem 6 bytes** — `member_count(1) + data_count(1) + idLeader(4)`, idêntico
  ao 1.5.3. A correção do cabeçalho feita no item 47 vale para as duas versões.
- **Cada membro ocupa 25 bytes** no 1.2.6, contra 34 no 1.5.3. Δ = **−9**.

A decomposição mais provável de −9 é `reincarnation_times(1) + force_id(4) +
profit_level(4)` — e vale notar que o `has_reincarnation()` do nosso `version.rs` **já
previa** que 1.2.6 não tem reencarnação. A contagem de bytes concorda com uma previsão que
o código fez por outro caminho.

Ressalva: `1+4+4` também fecha trocando o `reincarnation_times` por `wallow_level` ou
`level2`. Os dois campos de 4 bytes são quase certos; o de 1 byte precisa dos valores para
ser fechado.

## Os nove que estão em produção com o layout errado para 1.2.6

Destes 32, **nove têm codificador nosso em uso**:

| id | comando | 1.2.6 | 1.5.3 |
| ---: | :--- | ---: | ---: |
| 24 | `HOST_ATTACKRESULT` | 10 | 13 |
| 26 | `HOST_ATTACKED` | 11 | 14 |
| 32 | `PLAYER_INFO_00` | 24 | 28 |
| 33 | `NPC_INFO_00` | 12 | 16 |
| 36 | `RECEIVE_EXP` | 4 | 8 |
| 48 | `EQUIP_ITEM` | 6 | 10 |
| 142 | `HOST_SKILL_ATTACK_RESULT` | 14 | 18 |
| 164 | `ENTER_SANCTUARY` | 0 | 4 |
| 206 | `INST_DATA_CHECKOUT` | 16 | 20 |

Dois deles — `HOST_ATTACKED` e `PLAYER_INFO_00` — eu escrevi **a partir do IR** e
declarei, no `subcomandos_s2c_contra_o_ir.rs`, que "se divergirem é bug de quem escreveu".
Eles não divergem do IR: divergem do **1.2.6**, que é outra coisa. A frase estava certa e
mesmo assim incompleta.

## O que também ficou confirmado

- **Os comandos de grupo 57, 59, 60, 61 e 62 são idênticos nas duas versões.** As
  correções do item 47 (`team_leader_invite`, `team_join_party`, `team_leave_party`,
  `team_member_leave`) valem para 1.2.6 e 1.5.3.
- **`DUEL_REQUEST` (92) e `DUEL_REPLY` (93)** batem.
- **82 dos 133 comandos S2C observados** e **24 dos 42 C2S** têm exatamente o mesmo
  tamanho nas duas versões.

---

# Captura: /mnt/user-data/uploads/pw-universal-server/_sync/capturas/full_interno.pcap

## Fluxos
- 127.0.0.1:29301 → 127.0.0.1:41868: 31211 bytes, 927 quadros GNET, 0 S2C / 42 C2S distintos
- 127.0.0.1:41868 → 127.0.0.1:29301: 861752 bytes, 22217 quadros GNET, 133 S2C / 0 C2S distintos

## S2C — o que o servidor mandou

| id | comando | observado (bytes × vezes) | IR 1.5.3 | veredito |
| ---: | :--- | :--- | ---: | :--- |
| 4 | PLAYER_INFO_1_LIST | 28×6, 54×2, 80×1 | — | **lista: 2 + n×26** |
| 8 | SELF_INFO_1 | 34×3 | — | IR não declara tamanho |
| 9 | NPC_INFO_LIST | 29×10, 56×11, 83×5, 110×8, 137×10, 164×4, 191×3, 218×2, 245×4, 272×1, 299×3, 488×1, 731×1, 920×1, 1001×1, 1838×1, 2027×2, 2081×1 | — | tamanho variável |
| 10 | MATTER_INFO_LIST | 27×14, 52×4, 77×2, 252×3, 377×1 | — | tamanho variável |
| 11 | NPC_ENTER_SLICE | 27×25 | — | IR não declara tamanho |
| 12 | PLAYER_ENTER_SLICE | 26×7 | — | IR não declara tamanho |
| 13 | OBJECT_LEAVE_SLICE | 4×41 | 4 | igual ao 1.5.3 |
| 14 | NOTIFY_HOSTPOS | 16×7 | 20 | **difere: 16 bytes (-4)** |
| 15 | OBJECT_MOVE | 21×17294 | 21 | igual ao 1.5.3 |
| 16 | NPC_ENTER_WORLD | 27×23 | — | IR não declara tamanho |
| 17 | PLAYER_ENTER_WORLD | 26×3 | — | IR não declara tamanho |
| 18 | MATTER_ENTER_WORLD | 25×33 | 25 | igual ao 1.5.3 |
| 19 | PLAYER_LEAVE_WORLD | 4×3 | 4 | igual ao 1.5.3 |
| 20 | NPC_DIED | 8×23 | 8 | igual ao 1.5.3 |
| 21 | OBJECT_DISAPPEAR | 4×22 | 4 | igual ao 1.5.3 |
| 23 | HOST_STOPATTACK | 4×29 | 4 | igual ao 1.5.3 |
| 24 | HOST_ATTACKRESULT | 10×52 | 13 | **difere: 10 bytes (-3)** |
| 25 | ERROR_MESSAGE | 4×28 | 4 | igual ao 1.5.3 |
| 26 | HOST_ATTACKED | 11×25 | 14 | **difere: 11 bytes (-3)** |
| 30 | PICKUP_MONEY | 4×10 | 4 | igual ao 1.5.3 |
| 31 | PICKUP_ITEM | 14×5 | 18 | **difere: 14 bytes (-4)** |
| 32 | PLAYER_INFO_00 | 24×73 | 28 | **difere: 24 bytes (-4)** |
| 33 | NPC_INFO_00 | 12×80 | 16 | **difere: 12 bytes (-4)** |
| 34 | OUT_OF_SIGHT_LIST | 8×7, 12×15, 16×7, 20×9, 24×7, 28×6, 32×6, 40×5, 44×1, 144×1, 152×1, 156×1, 356×1, 376×1 | — | tamanho variável |
| 35 | OBJECT_STOP_MOVE | 20×2986 | 20 | igual ao 1.5.3 |
| 36 | RECEIVE_EXP | 4×36 | 8 | **difere: 4 bytes (-4)** |
| 37 | LEVEL_UP | 4×1 | 4 | igual ao 1.5.3 |
| 38 | SELF_INFO_00 | 36×267 | 36 | igual ao 1.5.3 |
| 39 | UNSELECT | 0×40 | 0 | igual ao 1.5.3 |
| 40 | OWN_ITEM_INFO | 34×1, 74×1, 90×1, 98×1, 106×1, 134×1, 146×1, 158×1 | — | tamanho variável |
| 42 | OWN_IVTR_DATA | 134×3 | — | IR não declara tamanho |
| 43 | OWN_IVTR_DETAIL_DATA | 10×1, 110×2, 146×1, 258×1, 450×1, 950×1, 1170×1, 1182×1 | — | tamanho variável |
| 44 | EXG_IVTR_ITEM | 2×1 | 2 | igual ao 1.5.3 |
| 46 | PLAYER_DROP_ITEM | 9×7 | 11 | **difere: 9 bytes (-2)** |
| 48 | EQUIP_ITEM | 6×9 | 10 | **difere: 6 bytes (-4)** |
| 50 | OWN_EXT_PROP | 152×15 | 188 | **difere: 152 bytes (-36)** |
| 51 | ADD_STATUS_POINT | 20×1 | 20 | igual ao 1.5.3 |
| 52 | SELECT_TARGET | 4×47 | 4 | igual ao 1.5.3 |
| 54 | PLAYER_EXT_PROP_MOVE | 20×2 | 20 | igual ao 1.5.3 |
| 57 | TEAM_LEADER_INVITE | 10×2 | 10 | igual ao 1.5.3 |
| 59 | TEAM_JOIN_TEAM | 6×3 | 6 | igual ao 1.5.3 |
| 60 | TEAM_MEMBER_LEAVE | 10×2 | 10 | igual ao 1.5.3 |
| 61 | TEAM_LEAVE_PARTY | 6×3 | 6 | igual ao 1.5.3 |
| 62 | TEAM_NEW_MEMBER | 4×2 | 4 | igual ao 1.5.3 |
| 64 | TEAM_MEMBER_DATA | 31×30, 56×16, 81×7 | — | **lista: 6 + n×25** |
| 66 | EQUIP_DATA | 14×2, 18×1, 22×2, 62×1, 66×2 | — | tamanho variável |
| 67 | EQUIP_DATA_CHANGED | 14×3, 18×6 | — | tamanho variável |
| 69 | TEAM_MEMBER_PICKUP | 12×3 | 12 | igual ao 1.5.3 |
| 70 | NPC_GREETING | 4×18 | 4 | igual ao 1.5.3 |
| 72 | PURCHASE_ITEM | 20×1 | — | IR não declara tamanho |
| 73 | ITEM_TO_MONEY | 14×1 | 14 | igual ao 1.5.3 |
| 74 | REPAIR_ALL | 4×1 | 4 | igual ao 1.5.3 |
| 75 | REPAIR | 6×1 | 6 | igual ao 1.5.3 |
| 77 | SPEND_MONEY | 4×4 | 4 | igual ao 1.5.3 |
| 82 | GET_OWN_MONEY | 8×5 | 8 | igual ao 1.5.3 |
| 83 | ATTACK_ONCE | 1×71 | 1 | igual ao 1.5.3 |
| 84 | HOST_START_ATTACK | 7×29 | 7 | igual ao 1.5.3 |
| 85 | OBJECT_CAST_SKILL | 15×19 | 15 | igual ao 1.5.3 |
| 86 | SKILL_INTERRUPTED | 4×1 | 4 | igual ao 1.5.3 |
| 87 | SELF_SKILL_INTERRUPTED | 1×1 | 1 | igual ao 1.5.3 |
| 88 | SKILL_PERFORM | 0×18 | 0 | igual ao 1.5.3 |
| 90 | SKILL_DATA | 14×2, 74×1 | — | tamanho variável |
| 91 | HOST_USE_ITEM | 8×11 | 8 | igual ao 1.5.3 |
| 92 | EMBED_ITEM | 2×1 | 2 | igual ao 1.5.3 |
| 94 | COST_SKILL_POINT | 4×1 | 4 | igual ao 1.5.3 |
| 95 | LEARN_SKILL | 8×1 | 8 | igual ao 1.5.3 |
| 96 | OBJECT_TAKEOFF | 4×5 | 4 | igual ao 1.5.3 |
| 97 | OBJECT_LANDING | 4×4 | 4 | igual ao 1.5.3 |
| 99 | HOST_OBTAIN_ITEM | 14×3 | 18 | **difere: 14 bytes (-4)** |
| 100 | PRODUCE_START | 8×1 | 8 | igual ao 1.5.3 |
| 101 | PRODUCE_ONCE | 10×1 | 14 | **difere: 10 bytes (-4)** |
| 102 | PRODUCE_END | 0×1 | 0 | igual ao 1.5.3 |
| 103 | DECOMPOSE_START | 6×1 | 6 | igual ao 1.5.3 |
| 104 | DECOMPOSE_END | 0×1 | 0 | igual ao 1.5.3 |
| 105 | TASK_DATA | 26×1, 60×1, 284×1 | — | tamanho variável |
| 106 | TASK_VAR_DATA | 7×1, 13×18, 14×5, 15×3, 18×7, 2455×3 | — | tamanho variável |
| 109 | OBJECT_USE_ITEM | 8×2 | 8 | igual ao 1.5.3 |
| 111 | OBJECT_SIT_DOWN | 4×2 | 4 | igual ao 1.5.3 |
| 112 | OBJECT_STAND_UP | 4×2 | 4 | igual ao 1.5.3 |
| 114 | SERVER_TIME | 12×3 | 12 | igual ao 1.5.3 |
| 120 | OBJECT_ATTACK_RESULT | 14×74 | 17 | **difere: 14 bytes (-3)** |
| 123 | HOST_STOP_SKILL | 0×19 | 0 | igual ao 1.5.3 |
| 126 | PLAYER_GATHER_START | 9×1 | 9 | igual ao 1.5.3 |
| 127 | PLAYER_GATHER_STOP | 4×1 | 4 | igual ao 1.5.3 |
| 129 | TRASHBOX_PWD_STATE | 1×3 | 1 | igual ao 1.5.3 |
| 130 | TRASHBOX_OPEN | 2×1 | 7 | **difere: 2 bytes (-5)** |
| 131 | TRASHBOX_CLOSE | 0×1 | 1 | **difere: 0 bytes (-1)** |
| 132 | TRASHBOX_WEALTH | 4×1 | 5 | **difere: 4 bytes (-1)** |
| 133 | EXG_TRASHBOX_ITEM | 2×1 | 3 | **difere: 2 bytes (-1)** |
| 141 | OBJECT_DO_ACTION | 5×1 | 5 | igual ao 1.5.3 |
| 142 | HOST_SKILL_ATTACK_RESULT | 14×18 | 18 | **difere: 14 bytes (-4)** |
| 143 | OBJECT_SKILL_ATTACK_RESULT | 18×18 | 22 | **difere: 18 bytes (-4)** |
| 144 | HOST_SKILL_ATTACKED | 15×1 | 19 | **difere: 15 bytes (-4)** |
| 147 | PLAYER_IN_TEAM | 5×6 | 5 | igual ao 1.5.3 |
| 152 | MATTER_PICKUP | 8×12 | 8 | igual ao 1.5.3 |
| 156 | TASK_DELIVER_ITEM | 10×2 | 18 | **difere: 10 bytes (-8)** |
| 157 | TASK_DELIVER_REP | 8×1 | 8 | igual ao 1.5.3 |
| 158 | TASK_DELIVER_EXP | 8×2 | 8 | igual ao 1.5.3 |
| 159 | TASK_DELIVER_MONEY | 8×1 | 8 | igual ao 1.5.3 |
| 161 | HOST_REPUTATION | 4×3 | 4 | igual ao 1.5.3 |
| 164 | ENTER_SANCTUARY | 0×11 | 4 | **difere: 0 bytes (-4)** |
| 165 | LEAVE_SANCTUARY | 0×9 | 4 | **difere: 0 bytes (-4)** |
| 166 | PLAYER_OPEN_BOOTH | 36×1 | — | IR não declara tamanho |
| 167 | SELF_OPEN_BOOTH | 14×1 | — | IR não declara tamanho |
| 168 | PLAYER_CLOSE_BOOTH | 4×1 | 4 | igual ao 1.5.3 |
| 180 | WAYPOINT_LIST | 6×2, 100×1 | — | tamanho variável |
| 181 | UNFREEZE_IVTR_SLOT | 3×24 | 3 | igual ao 1.5.3 |
| 185 | HOST_PVP_COOLDOWN | 8×3 | 8 | igual ao 1.5.3 |
| 186 | COOLTIME_DATA | 2×2, 12×1 | — | tamanho variável |
| 187 | SKILL_ABILITY | 8×1 | 8 | igual ao 1.5.3 |
| 188 | OPEN_BOOTH_TEST | 0×1 | 0 | igual ao 1.5.3 |
| 192 | PLAYER_ENABLE_FASHION | 5×2 | 5 | igual ao 1.5.3 |
| 198 | SET_COOLDOWN | 8×22 | 8 | igual ao 1.5.3 |
| 206 | INST_DATA_CHECKOUT | 16×3 | 20 | **difere: 16 bytes (-4)** |
| 212 | DOUBLE_EXP_TIME | 8×3 | 8 | igual ao 1.5.3 |
| 213 | AVAILABLE_DOUBLE_EXP_TIME | 4×3 | 4 | igual ao 1.5.3 |
| 214 | DUEL_RECV_REQUEST | 4×1 | 4 | igual ao 1.5.3 |
| 216 | DUEL_PREPARE | 8×2 | 8 | igual ao 1.5.3 |
| 218 | HOST_DUEL_START | 4×2 | 4 | igual ao 1.5.3 |
| 219 | DUEL_STOP | 4×2 | 4 | igual ao 1.5.3 |
| 220 | DUEL_RESULT | 9×1 | 9 | igual ao 1.5.3 |
| 227 | PLAYER_MOUNTING | 9×2 | 10 | **difere: 9 bytes (-1)** |
| 229 | PLAYER_DUEL_START | 4×2 | 4 | igual ao 1.5.3 |
| 233 | SUMMON_PET | 12×1 | 16 | **difere: 12 bytes (-4)** |
| 234 | RECALL_PET | 8×2 | 9 | **difere: 8 bytes (-1)** |
| 235 | PLAYER_START_PET_OP | 16×2 | 16 | igual ao 1.5.3 |
| 236 | PLAYER_STOP_PET_OP | 0×2 | 0 | igual ao 1.5.3 |
| 239 | PET_ROOM | 2×2, 198×1 | 2 | tamanho variável |
| 240 | PET_ROOM_CAPACITY | 4×4 | 4 | igual ao 1.5.3 |
| 251 | REFINE_RESULT | 4×1 | 4 | igual ao 1.5.3 |
| 253 | PLAYER_CASH | 4×4 | 4 | igual ao 1.5.3 |
| 255 | CHANGE_IVTR_SIZE | 4×1 | 4 | igual ao 1.5.3 |
| 256 | PVP_MODE | 1×3 | 1 | igual ao 1.5.3 |

133 comandos vistos: **82 com o mesmo tamanho do 1.5.3**, **27 com tamanho diferente**.

## C2S — o que o cliente mandou

| id | comando | observado (bytes × vezes) | IR 1.5.3 | veredito |
| ---: | :--- | :--- | ---: | :--- |
| 0 | PLAYER_MOVE | 31×472 | 31 | igual ao 1.5.3 |
| 1 | LOGOUT | 4×3 | 4 | igual ao 1.5.3 |
| 2 | SELECT_TARGET | 4×47 | 4 | igual ao 1.5.3 |
| 3 | NORMAL_ATTACK | 1×32 | 1 | igual ao 1.5.3 |
| 6 | PICKUP | 8×39 | 8 | igual ao 1.5.3 |
| 7 | STOP_MOVE | 20×148 | 20 | igual ao 1.5.3 |
| 8 | UNSELECT | 0×8 | — | IR não declara tamanho |
| 9 | GET_ITEM_INFO | 2×6 | 2 | igual ao 1.5.3 |
| 12 | EXG_IVTR_ITEM | 2×1 | 2 | igual ao 1.5.3 |
| 14 | DROP_IVTR_ITEM | 3×1 | 5 | **difere: 3 bytes (-2)** |
| 17 | EQUIP_ITEM | 2×9 | 2 | igual ao 1.5.3 |
| 19 | GOTO | 12×7 | 12 | igual ao 1.5.3 |
| 21 | GET_EXT_PROP | 0×12 | 0 | igual ao 1.5.3 |
| 22 | SET_STATUS_POINT | 16×1 | 16 | igual ao 1.5.3 |
| 27 | TEAM_INVITE | 4×2 | 4 | igual ao 1.5.3 |
| 28 | TEAM_AGREE_INVITE | 8×2 | 8 | igual ao 1.5.3 |
| 30 | TEAM_LEAVE_PARTY | 0×2 | 0 | igual ao 1.5.3 |
| 33 | GET_OTHER_EQUIP | 6×6, 10×1 | — | tamanho variável |
| 35 | SEVNPC_HELLO | 4×18 | 4 | igual ao 1.5.3 |
| 37 | SEVNPC_SERVE | 12×6, 14×2, 16×3, 20×4, 24×1, 28×1 | — | tamanho variável |
| 39 | GET_ALL_DATA | 3×3 | 3 | igual ao 1.5.3 |
| 40 | USE_ITEM | 8×11 | 8 | igual ao 1.5.3 |
| 41 | CAST_SKILL | 10×19 | — | IR não declara tamanho |
| 42 | CANCEL_ACTION | 0×15 | 0 | igual ao 1.5.3 |
| 46 | SIT_DOWN | 0×2 | — | IR não declara tamanho |
| 47 | STAND_UP | 0×2 | — | IR não declara tamanho |
| 49 | TASK_NOTIFY | 7×12 | 4 | **difere: 7 bytes (+3)** |
| 53 | GET_ITEM_INFO_LIST | 4×1 | — | IR não declara tamanho |
| 54 | GATHER_MATERIAL | 16×1 | 16 | igual ao 1.5.3 |
| 55 | GET_TRASHBOX_INFO | 1×1 | 2 | **difere: 1 bytes (-1)** |
| 56 | EXG_TRASHBOX_ITEM | 2×1 | 3 | **difere: 2 bytes (-1)** |
| 62 | TRICK_ACTION | 1×1 | 1 | igual ao 1.5.3 |
| 75 | ENTER_SANCTUARY | 0×14 | 4 | **difere: 0 bytes (-4)** |
| 76 | OPEN_BOOTH | 46×1 | — | IR não declara tamanho |
| 77 | CLOSE_BOOTH | 0×1 | — | IR não declara tamanho |
| 84 | OPEN_BOOTH_TEST | 0×1 | — | IR não declara tamanho |
| 85 | SWITCH_FASHION_MODE | 0×2 | — | IR não declara tamanho |
| 92 | DUEL_REQUEST | 4×1 | 4 | igual ao 1.5.3 |
| 93 | DUEL_REPLY | 8×1 | 8 | igual ao 1.5.3 |
| 100 | SUMMON_PET | 4×1 | 4 | igual ao 1.5.3 |
| 101 | RECALL_PET | 0×1 | — | IR não declara tamanho |
| 106 | MALL_SHOPPING | 10×1 | — | IR não declara tamanho |

42 comandos vistos: **24 com o mesmo tamanho do 1.5.3**, **5 com tamanho diferente**.
