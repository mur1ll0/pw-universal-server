# Especificação 05: Simulação do mundo (`pw-gs`)

> Verificada contra o código em 2026-09-14, B50. Cobre
> `crates/pw-gs/src/{world,bus_server,bus_server/jogo,ai,combat,habilidades,entity,grid,npc,server,missoes,progressao,economia}.rs`.
>
> Estado de cada regra: `confirmado` (visto em jogo), `testado` (teste automatizado),
> `parcial`, `falta`. Toda regra portada cita o fonte original no código.

## 1. Estrutura

| peça | o que é |
| :--- | :--- |
| `server.rs` | laço de **50 ms**: `world.tick(50)`, um por mapa |
| `mapas.rs` (`RoteadorDeMapas`) | escuta do barramento; entrega cada `roleid` ao mapa gravado dele |
| `world.rs` (`WorldInstance`) | um mapa: jogadores, monstros, NPCs, matéria, grade, terreno, autosave; emite `EventoDoMundo` |
| `bus_server.rs` (`BusServer`) | ponta do barramento: roteia por `roleid`, trata subcomandos (spec 04 §6), traduz eventos em S2C, streaming de visibilidade |
| `grid.rs` | grade espacial, célula de **50 m** |
| `entity.rs` | `PlayerEntity`, `MonsterEntity`, `NpcEntity`, `MatterEntity` |
| `ai.rs` | movimento de monstro (perseguir, voltar, passear) |
| `combat.rs` | `AttackJudgement` portado |
| `habilidades.rs` | contas das habilidades portadas dos stubs do cliente |
| `npc.rs` | decodificação dos serviços de NPC |
| `missoes.rs` | motor de missões: listas binárias do jogador e as regras de aceitar, contar e premiar (§10) |
| `progressao.rs` | experiência do abate, subida de nível, regeneração, renascimento (§7) |
| `economia.rs` | drop de monstro e a `Bolsa` (empilhamento como o cliente) (§8) |
| `bus_server/jogo.rs` | onde as três regras acima encontram banco e fio: `com_contexto` carrega as bolsas, roda a regra com o mundo travado, manda os comandos e grava |

Um processo `pw-gs` por realm, com um `WorldInstance` e um `BusServer` por mapa atrás do
`RoteadorDeMapas` (`mapas.rs`, spec 02 §2.2). O jogador entra quando o link manda
`EnterWorld`: o mundo carrega o personagem do banco (`colocar_no_mundo`) — nível, atributos,
habilidades, itens, `sec_level`.

## 2. Nascimento de criaturas e recursos — `confirmado`

- `init_spawns` lê o `npcgen.data` do mapa (spec 03 §3.3): monstros, NPCs e matéria.
- NPC × monstro pelo **tipo do registro no `elements.data`** (`DT_NPC_ESSENCE`), não pelo
  número do id (`gs/npcgenerator.cpp:79,415`). Id que nenhuma tabela conhece usa o chute
  antigo (`tid >= 10000` = NPC).
- Altura por tipo de área + `fOffsetTrn`, terreno como piso.
- Mapa 161: 1.269 monstros, 208 NPCs; mundo 1: 29.620 monstros, 1.380 NPCs (log de subida).
- Monstro morto (`WorldInstance::matar_monstro`): o corpo some em **20 s** (`_corpse_delay`,
  `npc.cpp:803,1446-1459` → `OBJECT_DISAPPEAR`) e ele renasce no centro após o tempo do gerador
  (`respawn_sec` do `npcgen.data`), vida cheia, IA e lista de dano zeradas, com
  `NPC_ENTER_SLICE` a quem está a 120 m. `testado`. (Até B50 o tempo ia zero e **nenhum
  monstro renascia**.)

## 3. Visibilidade (streaming) — `confirmado`

`BusServer::atualizar_visiveis`, chamado a cada movimento:

| regra | valor | por quê |
| :--- | :--- | :--- |
| raio | `RAIO_DE_VISAO` **120 m** | raio ativo do cliente |
| histerese | recalcula após andar `PASSO_PARA_RECALCULAR` **20 m** (ou forçado: entrada, teleporte) | o cliente manda 20 movimentos/s |
| teto por atualização | criaturas **80**, matéria **40**, separados; mais próximos primeiro | não encher a fila; pedras não expulsam NPCs |
| jogadores | fora do teto; visibilidade **mútua** — quem se move escreve nos dois `visiveis` | simetria |
| comandos | spec 04 §5 (11/12/18 entram; 13/34/19 saem) | |

Saída do jogo: `tirar_da_vista_de_todos` (no `LOGOUT` e no `PlayerLogout`).
`falta`: `dir` vai zero (a grade guarda posição); jogadores visíveis por link na fala não
separam mundos.

## 4. IA de monstro (`ai.rs`) — `testado` (publicado, sem teste em jogo)

Regras de `gs/aipolicy.cpp`, `gs/ainpc.cpp`, `gs/npcsession.cpp`:

| comportamento | regra |
| :--- | :--- |
| perseguir | a cada `PASSO_DE_PERSEGUICAO_MS` **500 ms** avança `run_speed × 0,5` m (`session_npc_follow_target`); para ao entrar no alcance |
| altura do passo | monstro de chão: chão do `.hmap`; água/ar: segue o alvo sem descer abaixo do terreno |
| voltar | sem alvo, corre ao nascimento, passo de 1 s (`ai_returnhome_task`) |
| passear | só com jogador a menos de `RAIO_DE_ATIVIDADE` 120 m (renovado por 20 batimentos de 1 s), com `patroll_mode`, sem ódio: anda (`walk_speed`, passo de 1 s) até ponto a **10 m** do nascimento, no máximo 8 passos; 10% de emendar outro |
| aviso ao cliente | `OBJECT_MOVE` (ms, ×256, modo) e `OBJECT_STOP_MOVE` com direção ao parar |
| ataque | usa `ataque_em_ticks` e alcance do `MONSTER_ESSENCE` |

`falta`: intérprete do `aipolicy.data` (habilidades, falas, invocações de monstro), mapa de
movimento (atravessa obstáculos), reação do monstro ao ser atacado — relato de que não
reage (a investigar).

## 5. Combate (`combat.rs`) — `confirmado`

`gactive_imp::AttackJudgement` + `HandleAttackMsg` (`actobject.cpp`), com as rolagens
recebidas prontas (`Rolagens`) para teste determinístico:

1. **Acerto** (só físico): `taxa / (taxa + (armadura >> 1))`, piso 0,05, sem teto.
2. **Curta distância**: habilidade × fator próprio; golpe normal divide o físico por 2.
3. **Atenuação por distância** (atacante jogador/pet).
4. **Defesa por classe** (físico + 5 mágicas): `reduce = def / (def + 40×nível_do_atacante − 25)`, teto 0,95; imunidade zera a classe. Penetração: `def × (1 − anti/(anti+10000))`, razão até 0,35.
5. **Crítico**: `× (2,0 + bônus% − redução%)`; crítico se `Rand(0,99) < crit_rate − crit_resistance`.
6. **Grau**: vantagem `× (1 + g×0,01)`, desvantagem `÷ (1 − g×0,012)`.
7. **Piso 1** para golpe que acertou.

Físico sorteia `Rand` uniforme; elemental `RandNormal` (triangular). `attack` é **precisão**,
não dano. Base do jogador: `agi_attack[cls] × agi`, `agi_armor[cls] × agi`
(`CHARRACTER_CLASS_CONFIG`).

`falta`: equipamento/encantamento somando aos atributos (`UpdateAttack`), filtros, vigor,
roubo de vida, bits do `attack_flag`, **trava de PvP** (qualquer jogador fere qualquer
outro), `PLAYER_DIED` para terceiros, perda de experiência ao morrer.

## 6. Habilidades (`habilidades.rs`) — `parcial`

- Conta dos stubs `EvolvedPWClient/ElementSkill/skillNNN.h`: `dano = base × ratio + plus`
  (base física ou mágica conforme o `Set*damage`); cura pelo `StateAttack`.
- **16** habilidades portadas (o kit inicial das 12 classes); as outras 3.301 conjuram,
  animam e **não fazem efeito** (`Habilidade::conhecida` = `None`).
- Nível: `PlayerEntity::habilidades` (do `character_skills`); piso 1 para quem não aprendeu;
  teto 10.
- Custo de mana como o cliente arredonda. Resultado: `SELF_SKILL_ATTACK_RESULT` (142) para
  quem conjura, `HOST_SKILL_ATTACKED` (144) para o alvo (`cEquipment = 0x7f`: sem desgaste).
- **Tabela do servidor** (`pw_data_loader::habilidades`, spec 03 §3.12), para as 3.316:
  - tempo de conjuração no `OBJECT_CAST_SKILL` = `State1::GetTime` do nível (`skill.cpp:797`);
    sem valor na tabela, o de `habilidades.rs`, e por fim 1000 ms;
  - **recarga** conferida antes de conjurar e armada em `id + 1024` com
    `(int)(0,001 × coolingtime) × 1000` (`skillwrapper.cpp:261`, `playerwrapper.cpp:170`):
    `SET_COOLDOWN` (198) ao cliente; em recarga, `ERROR_MESSAGE` 53 e `HOST_STOP_SKILL`. `testado`.
- `falta`: efeitos de estado; recarga comum (`commoncooldown`); cura usa ataque mágico no lugar
  de `GetMagicdamage`; Tiro Certeiro (234) assume carga cheia; Portal da Cidade (167) sem
  efeito; flechas não são gastas (`DoAttack`, `player.cpp:3066`).

## 7. Jogador

| regra | estado | detalhe |
| :--- | :--- | :--- |
| velocidades, cadência, alcance, `hp_gen`/`mp_gen` | `confirmado` | `CHARRACTER_CLASS_CONFIG` (Bárbaro: correr 4,9 m/s, ataque 0,8 s = 16 ticks, alcance 2,5 m) |
| vida/mana máximas | `confirmado` | `BaseDaClasse::vida_e_mana_maximas` (spec 03 §3.5), a mesma conta da criação |
| **combate** | `testado` | `combate_s`: atacar põe 15 s (`DoAttack`, `player.cpp:3062`), apanhar garante 5 s (`OnAttacked`, `:9514`); batimento de 1 s desconta |
| **regeneração** | `testado` | batimento de 1 s no `tick`: `hp_gen`/`mp_gen` em combate, ×4 fora (`player.cpp:9130-9137`), acumulando oitavos (`func::Update`, `actobject.h:2143`); `SELF_INFO_00` quando muda |
| **experiência e SP do abate** | `testado` | lista de dano no monstro; cada um recebe `exp × dano / max(total, max_hp)` (`DispatchExp`, `npc.cpp:1515`) com o ajuste da diferença de nível e `+0,5` (`ReceiveExp`, `player.cpp:2813`); `RECEIVE_EXP` (36) depois de somar. Sem grupo: não há divisão de equipe |
| **subida de nível** | `testado` | `IncExp`/`LevelUp` (`player.cpp:2627-2711,2831-2896`): curva `PLAYER_LEVELEXP_CONFIG` 202, +5 pontos de atributo, atributos refeitos (`recalcular_por_nivel`), vida e mana cheias, experiência zera no teto (`logic_level_limit` 105); `LEVEL_UP` (37) a todos, `SELF_INFO_00` e `OWN_EXT_PROP` ao próprio |
| reviver na cidade (C2S 4) | `testado` | ponto de cidade do distrito do `precinct.sev` que contém a posição (`ResurrectInTown`, `playercmd.cpp:112`; spec 03 §3.7); sem distrito ou distrito de outro mapa, no lugar. Vida e mana a 10 % e perda de `GetLvlupExp × exp_lost[cultivo]` (`Resurrect`, `player.cpp:8716`) |
| pontos de atributo | `parcial` | acumulam e vão no `OWN_EXT_PROP`; distribuir (`SET_STATUS_POINT`) `falta` |
| voo | `confirmado` | pelo item no slot 12 (`EQUIPIVTR_FLYSWORD`); sem custo de mana, sem teto, `GP_STATE_FLY` fora do `state` |
| teleporte de GM (`GOTO`) | `confirmado` | `y` do cliente é marcador; altura = chão + 0,5 m (`playercmd.cpp:4926`) |
| sentar, gestos, roupa, zona segura | `confirmado` | `modo_roupa` e `voando` não persistem |
| grupo | `testado` | estado de grupo no mundo (convite, aceite, recusa, saída) |
| troca de mundo | `falta` | spec 02 §2.2 |

## 8. Itens e economia

A bolsa é lida do banco a cada operação e gravada de volta só nos slots que mudaram
(`economia::Bolsa`). O **dinheiro vive na entidade** (o autosave grava a entidade por cima do
banco); toda operação que mexe nele passa por `com_contexto` e grava na hora.

| regra | estado | detalhe |
| :--- | :--- | :--- |
| repositório de itens | `testado` | transacionado; troca de slot preserva os octetos do item (A37) |
| equipar | `confirmado` | com bloco de dados (spec 04 §5) |
| empilhar na bolsa | `testado` | `CECInventory::MergeItem` (`EC_Inventory.cpp:179-215`): completa pilhas na ordem dos slots, o resto no primeiro vazio; limite `pile_num_max`. O cliente confere o slot e a quantidade devolvidos |
| comprar de NPC | `testado` | preço `max(shop_price, price)`; empilha e responde `PURCHASE_ITEM` (72) (`PurchaseItem`, `player.cpp:8900`); sem dinheiro `ERROR_MESSAGE` 16; `falta` conferir a lista de venda do NPC |
| vender a NPC | `testado` | `price × quantidade`, proporcional à durabilidade (`ItemToMoney`, `player.cpp:13930`); `ITEM_TO_MONEY` (73); o `price` do cliente é ignorado |
| reparar | `parcial` | **150 fixo** (da entidade) |
| curar no NPC | `testado` | pelos valores do jogador |
| aprender habilidade | `testado` | `skill_executor::OnServe` + `SkillStub::LearnCondition`/`Learn` (`serviceprovider.cpp:1288`, `cskill/skill/skill.cpp:14-93`): habilidade da lista do treinador (`NPC_SKILL_SERVICE`), fora de combate, nível ≤ máximo, classe, pré-requisitos, nível, SP, `rank` × cultivo, dinheiro; cobra (`SPEND_MONEY` 77, `COST_SKILL_POINT` 94) e responde `LEARN_SKILL` (95). Requisito `null` na tabela recusa |
| **drop de monstro** | `testado` | dono = maior dano (+`max_hp/4` do primeiro golpe). Itens: `drop_times` rodadas de `probability_drop_num0..3` e `drop_matters[32]` (da 2ª rodada, só índices < 16), com o ajuste de item por nível (`DropItemFromData`, `npc.cpp:2649`; `generate_item_from_monster`, `itemdataman.cpp:1191`). Moedas: `drop_times` vezes, `Rand(médio±variação)`, chance 0,7, × ajuste. Cada monte a ±2 m, no chão (`worldmanager.cpp:512-555`), `tid` 3044 para moedas, id de matéria `0xC8…` |
| item no chão | `testado` | posse do dono por **30 s**, some em **300 s** (`matter.h:62`, `matter.cpp:133`); `MATTER_ENTER_WORLD` a quem está a 120 m e no streaming; `OBJECT_DISAPPEAR` ao sumir |
| **pegar** (C2S 6 e 184) | `testado` | tipo confere, distância < 10 m, posse; moedas `PICKUP_MONEY` (30), item `PICKUP_ITEM` (31); `MATTER_PICKUP` (152) a todos; bolsa cheia `ERROR_MESSAGE` 7, fora da posse 6 (`playercmd.cpp:1347-1444`, `matter.h:97-129`) |
| poção (`USE_ITEM`) | `confirmado` | `MEDICINE_ESSENCE` |
| colher recurso de mapa | `falta` | |
| Loja Gold, barraca | `falta` | |
| demais serviços de NPC (teleporte, pedras, forja, decompor, armazém, item de missão) | `falta` | |

## 9. Persistência

- **Autosave a cada 60 s** por mundo: `save_status` com nível, cultivo, exp, SP, vida, mana,
  moedas, mundo e posição, mais `potential_points` e as listas de missão. Falha vira `warn!`
  com contagem — não "sucesso" (B36f).
- Movimento **não** grava por pacote.
- Operações de `com_contexto` (missão, abate, coleta, loja, aprender) gravam na hora: bolsas
  antes de responder, estado e listas numa tarefa.
- Listas de missão: `character_task_lists` (cinco `BYTEA`, os blocos do `TASK_DATA`,
  `scripts/2026_09_14_listas_de_missao.sql`); lidas no `EnterWorld` pelo mundo e pelo link.
- Personagem novo: `class_templates` do realm (posição, kit, arma) + atributos do
  `ptemplate.conf` (spec 02 §4).

## 10. Missões (`missoes.rs`) — `testado` (sem teste em jogo)

**O cliente refaz cada operação na cópia dele** a partir dos avisos (`OnServerNotify`,
`TaskProcess.cpp:2643-2815`), então as listas do servidor são as estruturas binárias do
original, mexidas pelas mesmas funções portadas linha a linha: `DeliverTask`, `RealignTask`,
`RecursiveClearTask`, `RecursiveAward`, `FinishedTaskList::AddOneTask`
(`task/TaskProcess.cpp`, `TaskProcess.h:103-392`).

| lista | bytes no `TASK_DATA` | notas |
| :--- | :--- | :--- |
| ativa | `8 + 32 × n`: `count u8, used u8, version u16 = 1, top_show u8, state u8 (1 = tempos absolutos), top_hide u8, maxsim:1\|title:7`; entrada `id u16, pai, anterior, próximo, filho, estado u8, tempo u32, capitão u16, templ u32, cap u32, buf[11]` (os três `m_wMonsterNum`) | **versão ≠ 1 faz o cliente descartar todo aviso** (`TaskClient.cpp:262`) — a causa de "aceitar não mostra nada" (B50) |
| concluídas | `4 + 4 × n`, ordenada por id (`id u16, falhou:1, vezes u8`) | |
| tempos / contagens | `2 + 6 × n` / `2 + 14 × n` | frequência diária/semanal e limites por conta/personagem |
| depósito | 864 bytes zerados | depósito de missões `falta` |

| operação | origem | estado |
| :--- | :--- | :--- |
| aceitar no NPC (`GP_NPCSEV_TASK_ACCEPT`) | NPC em conversa (`SEVNPC_HELLO`) com a missão em `NPC_TASK_OUT_SERVICE` (`serviceprovider.cpp:1088`); submissão vira escolha da mãe (`OnTaskCheckDeliver`); `CheckPrerequisite` na ordem do original; `svr_new_task` (17 bytes + tags) | `testado` |
| entregar no NPC (`GP_NPCSEV_TASK_RETURN`) | missão em `NPC_TASK_IN_SERVICE`; `OnTaskCheckAward` por método; `DeliverAward` → `RecursiveCheckAward` → `RecursiveAward` → `DeliverByAwardData` (ouro, exp, SP, reputação, itens por grupo/escolha, missão nova, coeficiente de nível `_lev_co`); `svr_task_complete` com o estado | `testado` |
| abate (`OnTaskKillMonster`) | dono do abate; `CheckKillMonster`: conta (`svr_monster_killed`, 17 bytes) ou sorteia o item de missão; completa → `OnSetFinished` (conclusão direta premia) | `testado` |
| `TASK_NOTIFY` 1/2/4/5 | concluir (`OnTaskCheckAwardDirect`), desistir, entrega automática, gatilho manual; 7 = marca dinâmica (B49) | `testado` |
| itens de missão | bolsa de missão (pacote 2, `container_type` 5): `TASK_DELIVER_ITEM` (156), `PLAYER_DROP_ITEM` (46) tipo 3; prêmio `TASK_DELIVER_EXP/MONEY` (158/159), `SPEND_MONEY` | `testado` |
| erros | `svr_task_err_code` (reason 6) com `TASK_PREREQU_FAIL_*`; NPC sem a missão `ERROR_MESSAGE` 19 | `testado` |

Contra o `tasks.data` real (`tests/missoes_do_realm.rs`): o Arqueiro nível 1 aceita e entrega
32201 (25 exp, 10 SP, 8 moedas); mais de 1.000 missões de topo entregam sem quebrar os
índices da lista.

`falta` (recusado ou ignorado, nunca inventado): janelas de horário (`WRONG_TIME`), região de
entrega (`NOT_IN_ZONE`), equipe (`NOT_CAPTAIN`), facção, casamento, PQ, torre, variáveis
globais (lidas como 0), prêmio por escala de tempo/itens (vazio), teleporte e invocação de
prêmio, `FORCE_GIVEUP`, `REACH_SITE`/`LEAVE_SITE`, falha por morte, limite de tempo checado
só na entrega.
