# Especificação 05: Simulação do mundo (`pw-gs`)

> Verificada contra o código em 2026-09-14, commit `e6433ae` + B49. Cobre
> `crates/pw-gs/src/{world,bus_server,ai,combat,habilidades,entity,grid,npc,server}.rs`.
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
- Monstro morto renasce no centro de nascimento após `respawn_timer_ms`, com vida cheia e IA
  zerada.

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
- `falta`: tempo de conjuração vem de `TEMPO_DE_CONJURACAO_MS` **1000 fixo** (o certo é o
  `GetExecutetime` do stub); **nenhuma recarga**; efeitos de estado; cura usa ataque mágico no
  lugar de `GetMagicdamage`; Tiro Certeiro (234) assume carga cheia; Portal da Cidade (167)
  sem efeito.

## 7. Jogador

| regra | estado | detalhe |
| :--- | :--- | :--- |
| velocidades, cadência, alcance, `hp_gen`/`mp_gen` | `confirmado` | `CHARRACTER_CLASS_CONFIG` (Bárbaro: correr 4,9 m/s, ataque 0,8 s = 16 ticks, alcance 2,5 m) |
| vida/mana máximas | `confirmado` | `BaseDaClasse::vida_e_mana_maximas` (spec 03 §3.5), a mesma conta da criação |
| **regeneração** | `falta` | valores na entidade, nenhum laço no `tick` |
| **experiência e SP do abate** | `falta` | `RECEIVE_EXP` vai ao cliente, **nada soma na entidade** — o autosave grava o valor antigo |
| subida de nível | `falta` | curva em `PLAYER_LEVELEXP_CONFIG`, não ligada |
| reviver na cidade (C2S 4) | `parcial` | vida/mana cheias; posição de `CharacterClass::default_spawn_position` (mundo 1, palpites) — o certo é `[TOWN_REGION]`/`__GetTownPosition` |
| voo | `confirmado` | pelo item no slot 12 (`EQUIPIVTR_FLYSWORD`); sem custo de mana, sem teto, `GP_STATE_FLY` fora do `state` |
| teleporte de GM (`GOTO`) | `confirmado` | `y` do cliente é marcador; altura = chão + 0,5 m (`playercmd.cpp:4926`) |
| sentar, gestos, roupa, zona segura | `confirmado` | `modo_roupa` e `voando` não persistem |
| grupo | `testado` | estado de grupo no mundo (convite, aceite, recusa, saída) |
| troca de mundo | `falta` | spec 02 §2.2 |

## 8. Itens e economia

| regra | estado | detalhe |
| :--- | :--- | :--- |
| repositório de itens | `testado` | transacionado; troca de slot preserva os octetos do item (A37) |
| equipar | `confirmado` | com bloco de dados (spec 04 §5) |
| comprar de NPC | `confirmado` | preço `max(shop_price, price)` (`serviceprovider.cpp:241-252`); item sem preço não é vendido; durabilidade do arquivo |
| vender a NPC | `parcial` | **50 moedas fixas por unidade**; o `price` do cliente é ignorado de propósito |
| reparar | `parcial` | **150 fixo** |
| curar no NPC | `testado` | pelos valores do jogador |
| aprender habilidade | `confirmado` | +1 nível até 10, grava e responde `LEARN_SKILL`; **não cobra** SP/moedas/requisitos |
| poção (`USE_ITEM`) | `confirmado` | `MEDICINE_ESSENCE` |
| missões (aceitar/entregar) | `parcial` | grava em `character_quests`; entrega paga **1500 exp / 320 SP / 500 moedas fixos**; `tasks.data` não consultado; abate notifica missões ativas |
| notificações de missão do cliente (`TASK_NOTIFY`) | `parcial` | só o pedido da marca das missões dinâmicas é respondido (`testado`, B49); os demais `reason` são registrados |
| "missão inicial" na entrada | `parcial` | gravada pelo `gateway.rs` por tabela no código (9374 / 1 / 9375), sem origem |
| colher recurso | `falta` | `MATTER_PICKUP` sem tratamento |
| Loja Gold, barraca | `falta` | |
| demais serviços de NPC (teleporte, pedras, forja, decompor, armazém) | `falta` | |

## 9. Persistência

- **Autosave a cada 60 s** por mundo: `save_status` com nível, cultivo, exp, SP, vida, mana,
  moedas, mundo e posição. Falha vira `warn!` com contagem — não "sucesso" (B36f).
- Movimento **não** grava por pacote.
- Itens, habilidades aprendidas e missões gravam na hora da ação.
- Personagem novo: `class_templates` do realm (posição, kit, arma) + atributos do
  `ptemplate.conf` (spec 02 §4).
