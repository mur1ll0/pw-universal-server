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
- **Direção com que cada criatura nasce** (`entity::direcao_do_gerador`, B59): área que é um
  ponto usa a do gerador, `a3dvector_to_dir(vDir) = atan2(z, x) × 128/π & 0xFF`
  (`npcgenerator.cpp:4367`, `common/types.h:99-107`); área com extensão sorteia `Rand(0,255)`
  (`GenDir`, `npcgenerator.h:747-757`). Vai no `dir` do `NPC_ENTER_SLICE`
  (`protocol_imp.h:297-306`). Mandávamos zero para todos, e em jogo os NPCs ficavam todos
  virados para o mesmo lado. `testado`, falta ver em jogo.
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
| alcance dos avisos | movimento e parada de monstro vão **só a quem o vê** (`transmitir_a_quem_ve`) | o original difunde na fatia do NPC (`AutoBroadcastCSMsg`, `npc.cpp:85-98`). Mandando ao mapa inteiro, o cliente recebia comando de monstro que nunca viu entrar, o punha na fila de "NPC desconhecido" e perguntava por ele de 10 em 10 s para sempre (`EC_ManNPC.cpp:967-975`, `1144-1164`) — 319 comandos assim no teste de 2026-09-17 (B57) |

Saída do jogo: `tirar_da_vista_de_todos` (no `LOGOUT` e no `PlayerLogout`).
`falta`: o `dir` de **jogador** vai zero (a grade guarda posição, não direção) — o de criatura
leva a direção do gerador desde o B59 (§2); jogadores visíveis por link na fala não separam
mundos.

## 4. IA de monstro (`ai.rs`) — `testado` (publicado, sem teste em jogo)

Regras de `gs/aipolicy.cpp`, `gs/ainpc.cpp`, `gs/npcsession.cpp`:

| comportamento | regra |
| :--- | :--- |
| perseguir | a cada `PASSO_DE_PERSEGUICAO_MS` **500 ms** avança `run_speed × 0,5` m (`session_npc_follow_target`); para ao entrar no alcance |
| altura do passo | monstro de chão: chão do `.hmap`; água/ar: segue o alvo sem descer abaixo do terreno |
| voltar | sem alvo, corre ao nascimento, passo de 1 s (`ai_returnhome_task`) |
| passear | só com jogador a menos de `RAIO_DE_ATIVIDADE` 120 m (renovado por 20 batimentos de 1 s), com `patroll_mode`, sem ódio: anda (`walk_speed`, passo de 1 s) até ponto a **10 m** do nascimento, no máximo 8 passos; 10% de emendar outro |
| fase do batimento | **sorteada por monstro** (`MonsterAi::new`): o batimento de 1 s e o passo de patrulha começam em pontos diferentes do segundo, porque o original não bate em todos ao mesmo tempo — o coletor pega `tamanho / TICK_PER_SEC` objetos por tique (`objmanager.h:213-229`, `worldmanager.h:262`) e cada NPC nasce com `idle_timer_count = Rand(0, NPC_IDLE_HEARTBEAT)` (`npcgenerator.cpp:2014`). Sem isso, dez monstros davam o passo no mesmo quadro e o cliente tocava dez sons de passo sobrepostos (B58) |
| aviso ao cliente | `OBJECT_MOVE` (ms, ×256, modo) e `OBJECT_STOP_MOVE` com direção ao parar |
| invocado (missão ou matéria) | **não renasce** — não tem gerador (`SummonMonster` → `CreateMinors`, `gs/player.cpp:13072-13110`); nasce **odiando quem o chamou** (`GM_MSG_GEN_AGGRO` com 10000, `:13093-13106`) e some quando o `remain_time` acaba (`prop.remain_time`, `:13079`). O `respawn_delay_ms` zero passava por um `.max(1)` e o monstro voltava 1 ms depois do corpo sumir (B77) |
| alvo sozinho | `aggressive_mode` do `MONSTER_ESSENCE` (**4.874 dos 8.054** monstros do `realm_155`): o monstro recebe a marca `MSG_MASK_PLAYER_MOVE` (`npcgenerator.cpp:2534-2537`) e o jogador, ao andar, avisa quem está a até **15 m** (`GetMaxMobSightRange`, `playerctrl.cpp:265-276`, `worldmanager.cpp:48`). Nosso monstro agressivo sem alvo pega o jogador vivo mais perto dentro desse raio (B76). `falta`: as estratégias de ódio do `aipolicy.data` (facção, nível, invisibilidade, probabilidades) |
| ataque | intervalo = `attack_speed` do `MONSTER_ESSENCE` em tiques (`ChangeInterval`, `npcsession.cpp:60-70`; era 1,5 s fixo até o B62) e alcance do mesmo lugar. O `HOST_ATTACKED` leva o **id do monstro** que bateu (B59) — com zero ali o cliente não acha o atacante e o jogador perde vida sem ver golpe —, `cEquipment = 0x7f` ("nenhuma peça desgastada"; com zero o cliente gastava a arma a cada golpe) e `speed` = `damage_delay` do `MONSTER_ESSENCE` em tiques, que é a duração da animação no cliente (`npc.cpp:2118`, `EC_NPC.cpp:2043-2064`) (B60) |
| habilidades de monstro | `falta`: nenhum monstro conjura. Quem decide isso é o `aipolicy.data` (`ai_skill_task`), e não há intérprete — 3.688 monstros têm habilidade no `MONSTER_ESSENCE` e nenhuma é usada |

`falta`: intérprete do `aipolicy.data` (habilidades, falas, invocações de monstro), mapa de
movimento (atravessa obstáculos). O limite da perseguição ainda é a distância até o alvo; no
original é a distância **do ninho** contra o `_max_move_range` da política
(`aipolicy.cpp:300-307`), e a volta para casa é o `GetReturnHomeRange` (`ainpc.cpp:29-37`).

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

### 5.1 Atributos do equipamento — `testado` (B52)

`property_policy` (`playertemplate.h:807-1133`), refeito ao entrar, ao equipar/trocar e a cada
subida de nível ou ponto (`PlayerEntity::vestir`/`recalcular_por_nivel`):

| atributo | conta |
| :--- | :--- |
| dano físico | `Result(arma + acessórios, base de nível, bônus)`, bônus = `(agilidade` se a arma é de longo alcance ou `short_range_mode` 2, senão `força) × 100/150 + 0,5` |
| dano mágico | `Result(base + acessórios, arma, energia)` |
| alcance | alcance da arma (se > 0,1), senão o da classe; **+ 0,3** do corpo (`PLAYER_BODYSIZE`) |
| cadência | `attack_speed` da arma: `(int)(WEAPON_SUB_TYPE.attack_speed × 20 + 0,1)` (Arco = 30 ticks); sem arma, a da classe; entre 4 e 300 |
| defesa | `(base + armaduras + acessórios) × (100 + (vit×2 + for×3)×0,04 + 0,5)% + ((vit + for) >> 2)` |
| resistências | `(base + peças) × (100 + (vit×2 + ene×3)×0,04 + 0,5)% + ((vit + ene) >> 2)` |
| evasão | `agi_armor × agilidade + peças` |
| vida/mana | + `hp_enhance`/`mp_enhance` das armaduras |

`Result(a, b, p) = (int)((a + b) × (100 + p)/100 + 0,5)`. O `OWN_EXT_PROP` leva esses números:
é por ele que o cliente sabe até onde andar antes de atacar — com o alcance da classe (2,5) o
Arqueiro chegava perto (teste de 2026-09-16).

**Propriedades adicionais, refino e pedras (B53) — `testado`.** O equipamento que cai de
monstro sai sorteado como o original (`generate_weapon/armor/decoration` com
`ADDON_LIST_DROP`, `pw_gs::geracao`): essência (dano máximo, defesa, evasão, vida/mana,
resistências por `generate_magic_defense`), durabilidade de drop, furos
(`drop_probability_socket`) e addons (`probability_addon_num`, único da arma com
`probability_unique`, `fixed_props`), com o `GenerateParam` de cada tratador e os de essência
aplicados na hora (`ApplyAtGeneration`). O conteúdo vai nos octetos do item
(`pw_core::ConteudoDeEquipamento`, spec 04 §4), é gravado em `extra_data` e lido de volta ao
vestir: a essência gravada substitui a do modelo e cada addon soma pelo tratador (o
`OWN_EXT_PROP` leva força/agilidade/vitalidade/energia **com** os addons, o `_cur_prop` que a
janela do personagem mostra em verde, `DlgCharacter.cpp:442-470` — B54)
(`BonusDeAddons`: atributos, vida/mana, precisão, defesa, evasão, dano, dano mágico,
resistências, graus, crítico, `%` de dano/mágico/resistência). **Refino e pedras** são addons
da mesma lista (`refine_*` com o valor já multiplicado pelo nível; pedra com `0x8000`), então
entram pelo mesmo caminho. Com o `realm_155`: 5.231 peças com addon em 9.000 sorteios, 3.553
com bônus somado. `falta`: serviços de refinar/incrustar/fazer furo, addons de habilidade
(`item_skill_addon`), de conjunto (`SET_ADDON_MACRO`), redução de conjuração, penetração e o
índice de velocidade aleatório da arma (`WEAPON_SUB_TYPE.probability_fastest`) — sem porte
vão ao log `veste addons sem porte`.

### 5.1.0 A habilidade tem duas fases — `testado` (B67)

A sessão de habilidade do original é um **laço de estados**: `StartSkill` devolve o tempo do
primeiro e `RunSkill` o do seguinte, a cada volta de `session_skill::RepeatSession`; só quando
não há próximo estado vem o `EndSession`, que manda o `stop_skill`
(`gs/actsession.cpp:466-600`). Então, depois da conjuração há a **fase de execução**, e é nela
que o cliente anima o personagem.

**Conjurar andando** (`is_movingcast` do stub, `cskill/skill/skill.h:382`): o original
despacha essas habilidades por `moving_skill` em vez de `session_skill`
(`gs/playercmd.cpp:2066-2088`), e **o movimento não as interrompe**. É propriedade da
habilidade, não da classe: no 1.5.5 são **24**, todas da **classe 11** — nenhuma classe
antiga tem. Entre elas as duas primeiras de ataque da classe, **2571 e 2579**, que são as
que um Tormentador usa cedo (B78, corrigido no B82: o extrator só casava `= 1` e perdia os
19 stubs que escrevem `= true`).

Exemplo: a Flecha Fulgurante (244) tem `State1` de 3.000 ms (conjuração, o que vai no
`OBJECT_CAST_SKILL`) e `State2`/`GetExecutetime` de 800 ms (`cskill/skills/skill244.h:20-80`).
O efeito entra no fim da conjuração; o `HOST_STOP_SKILL` só sai 800 ms depois. Mandando os
dois juntos, o buff aparecia sem animação nenhuma (relato de 2026-09-19).

### 5.1.0 Nada que espere o banco fica no caminho do jogo — `testado` (B72)

O tique de 50 ms roda inteiro com o `world.write()` na mão, e as respostas a comandos
esperam esse mesmo lock. Toda escrita que ficar dentro dele para **o mundo todo** pelo tempo
do banco.

| onde estava | o que acontecia | onde está agora |
| :--- | :--- | :--- |
| autosave (4 escritas por jogador, a cada 60 s) | dentro do `tick`, com o mundo trancado | o `tick` devolve a fotografia (`EstadoParaGravar`) e o laço grava em `tokio::spawn`, com o lock solto |
| durabilidade da arma e da peça | `SELECT`+`UPDATE` antes de responder ao golpe | decidida em `PlayerEntity::pecas`; a gravação vai em `tokio::spawn` |
| munição | lida e gravada a cada golpe | contagem na sessão de ataque; baixa em `tokio::spawn` (B71) |

No combate de 2026-09-20 20:50 UTC o banco de teste passou a responder em 1 a 4 segundos e
o mundo ficou **8 segundos sem um único golpe** — o autosave segurava o lock. Os avisos
`slow statement` do `sqlx` no log do mundo são o rastro disso.

### 5.1.1 O dano cai depois da animação — `testado` (B62)

O golpe é **anunciado na hora** (`HOST_ATTACKRESULT` a quem bateu, `HOST_ATTACKED` a quem
apanhou) e a vida só cai `attack.speed` tiques depois: `InsertDamageEntry(dano, delay)`
(`gs/actobject.cpp:1758-1776`) transforma o dano num `GM_MSG_HURT` adiado de `delay` tiques de
50 ms, e só com `delay <= 0` aplica na hora (`DoDamage`).

| quem bate | `attack.speed` | de onde |
| :--- | :--- | :--- |
| jogador, golpe normal | `attack_delay = (attack_speed × 20 × 0,8) − 1` tiques | `playertemplate.h:980`, `MakeAttackMsg`, `actobject.cpp:824` |
| monstro | `_damage_delay` do `MONSTER_ESSENCE` | `gnpc_imp::DoAttack`, `npc.cpp:2118` |
| habilidade | **nada** — o original não preenche `speed`, então o dano é imediato | `FillEnchantMsg`, `player.cpp:3174` |

A barra de vida segue o dano, não o anúncio: o `SELF_INFO_00` de quem apanhou sai do
`aplicar_dano_no_jogador`, quando o golpe adiado vence, e **não** junto do `HOST_ATTACKED` —
mandá-lo junto era mandar a vida velha depois do golpe (B72).

É o mesmo número que o cliente usa como duração da animação do golpe. Aplicar o dano na hora
fazia a vida do monstro cair no clique, antes de a flecha sair — em jogo parecia "um golpe a
mais" no começo de cada sessão —, e o jogador perdia vida antes de o monstro parar de correr
na tela (relato de 2026-09-18). A **ameaça do monstro** (`AddAggroEntry`, `npc.cpp:1867`,
`npc.cpp:2354-2364`) no original manda `GM_MSG_GEN_AGGRO` com o mesmo atraso (`speed + 1`):
o monstro só reage e persegue quando o dano de fato atinge o alvo (`aplicar_dano_no_monstro`),
eliminando a perseguição prematura antes do impacto da flecha (B67).

A morte do alvo passa a ser resolvida quando o dano adiado vence
(`EventoDoMundo::MonstroMorreu`), e é o caminho único das três origens — golpe, habilidade e
dano no tempo.

### 5.2 Sessão de golpe normal — `testado` (B52)

`session_normal_attack` (`playercmd.cpp:1319-1344`, `actsession.cpp:350-418`):

- `NORMAL_ATTACK` abre a sessão se o alvo está vivo e a ≤ `attack_range + corpo do alvo`
  (`CheckAttack`, `actobject.cpp:1220-1292`; corpo = `MONSTER_ESSENCE.size`). Fora do alcance,
  nada acontece.
- Aberta: `HOST_START_ATTACK` (84: alvo, munição, cadência) — é o que abre o trabalho de golpe
  e a animação no cliente —, um golpe na hora, e outro a cada `attack_speed` (evento do tick).
- **Outro `NORMAL_ATTACK` com a sessão aberta entra na fila** (`AddSession`) e só começa no
  golpe seguinte: `GM_MSG_OBJ_SESSION_REPEAT` com `HasNextSession` encerra a atual e abre a da
  fila (`actobject.cpp:180-189`) — `HOST_STOPATTACK` + `HOST_START_ATTACK` no ritmo da arma.
- **`CANCEL_ACTION` (Esc) e andar entram na fila** (B54): `session_cancel_action` e
  `session_move` (`playercmd.cpp:2136-2153`, `9297-9303`) não param o golpe na hora
  (`TerminateSession(false)` é recusado, `actsession.h:109-115`), mas o encerram **no golpe
  seguinte** (`HasNextSession`). O cancelamento tira da fila o golpe que estava nela. O
  cliente manda `CANCEL_ACTION` + `NORMAL_ATTACK` a cada clique: o golpe novo substitui o
  atual no ritmo da arma. No B53 o cancelamento não fazia nada e Esc/andar não paravam.
- **Alvo que morre encerra a sessão na hora** (`HOST_STOPATTACK` motivo 2), para qualquer
  jogador que batia nele — e o monstro **só renasce depois de o corpo sumir** (20 s,
  `GM_MSG_OBJ_ZOMBIE_END` → `LifeExhaust` → `mobs_spawner::Reclaim`, `npc.cpp:904-911`,
  `npcgenerator.cpp:3312`). Até o B53 o renascimento contava desde a morte: o monstro voltava
  com o mesmo id antes do disparo seguinte e o ataque não parava (teste de 2026-09-17).
- Fora do alcance no golpe seguinte: `HOST_STOPATTACK` motivo 4. Conjurar e morrer também
  encerram. Atordoado/dormindo não golpeia.
- `HOST_ATTACKRESULT` leva o **atraso do golpe da arma**, `attack_delay = (int)(attack_speed ×
  0,8) − 1` tiques (`playertemplate.h:980`, mandado em `actobject.cpp:826`) — não a cadência
  cheia. O cliente usa esse número como duração da animação
  (`PlayAttackEffect(..., attack_speed × 50, …)`, `EC_HostMsg.cpp:943-955`): com 30 em vez de
  23, a animação do arco ficava ~350 ms mais lenta que a do original (B58). Em golpe de
  habilidade o campo vai **zero**, como no original (só `MakeAttackMsg` o preenche).
- **Quando o dano é aplicado**: no clique, como no original — `session_normal_attack::StartSession`
  chama `DoAttack` na hora (`actsession.cpp:350-378`), e a mensagem de golpe não tem atraso na
  fila (`PostLazyMessage` sem `delay_tick`, `world.h:452-465`). O cliente só anima ao receber o
  resultado e só mostra o número quando a flecha chega (700 ms de voo, `EC_Player.cpp:3507-3545`),
  então a barra de vida (batimento de 1 s) aparece antes do número — é assim no original.
- **Barra de vida do alvo (`NPC_INFO_00`) não vai junto do golpe** (B56). O original só a
  manda ao selecionar (`InsertInfoSubscibe` → `query_info00`, `actobject.cpp:1610`) e no
  heartbeat de 1 s (`obj_manager<gnpc, TICK_PER_SEC>`, `worldmanager.h:262`), a quem tem o
  monstro selecionado e só se vida ou alvo mudaram (`RefreshSubscibeList` + `_refresh_state`,
  `actobject.cpp:1296-1353`; `gnpc_imp::SendDataToSubscibeList`, `npc.cpp:2219-2230`). Aqui:
  `WorldInstance::informar_vida_aos_inscritos` no batimento → `EventoDoMundo::VidaDoMonstro`.
  Vale para golpe normal, habilidade (alvo único e área) e dano no tempo. Até o B55 a barra
  saía no mesmo instante do golpe e caía no clique, antes de a flecha sair (teste de
  2026-09-17). `testado`, falta ver em jogo.

- **`NORMAL_ATTACK` que chega com conjuração aberta entra na fila** (B57): no original a
  habilidade é a sessão corrente e o `AddSession` do golpe devolve `false`
  (`actobject.cpp:1180-1212`); ele começa quando a habilidade termina
  (`SafeDeleteCurSession` → `StartSession`), e aí o `CheckAttack` recusa alvo morto. Aqui:
  `BusServer::golpe_na_fila`, solto por `fechar_conjuracao_e_soltar_fila`. O cliente manda
  `NORMAL_ATTACK` assim que vê o fim da conjuração; sem a fila, o dano do golpe e o da
  habilidade caíam no mesmo instante e o monstro morria "instantaneamente" com a habilidade
  (teste de 2026-09-17). Sai da fila ao morrer e ao sair do jogo.

`falta`: filtros, vigor, roubo de vida, bits do `attack_flag`, **trava de PvP** (qualquer
jogador fere qualquer outro), `PLAYER_DIED` para terceiros, sessão de golpe contra jogador.

## 6. Habilidades (`habilidades.rs`) — `parcial`

- **Dano (B52)** para as **1.123** habilidades cujo estado chama
  `SetDamage`/`SetXdamage(k × GetAttack|GetMagicattack)` (extraído dos stubs do servidor):
  `GeneratePhysicDamage((int)ratio%, (int)plus)` = dano bruto sorteado (base + arma +
  acessórios) × (100 + bônus do atributo + ratio%)/100 + plus, × `k`
  (`actobject.h:1422-1469`, `skill.cpp:897`, `skill.h:634`); mágico com a energia. `Damage` vai
  na parcela física, `Gold/Wood/Water/Fire/Earthdamage` na escola. Passa pelo combate (§5):
  acerto, defesa, crítico. **Conferido**: a 235 nível 1 soma 112 ao dano do arco, o mesmo do
  tooltip do cliente — os mais de 100 de dano relatados são o original.
- **Alcance (B52)**: `GetPraydistance` = `k × attack_range + fixo` por nível; conjurar exige
  distância < corpo + 1 + alcance + corpo do alvo (`CheckTarget`, `playerwrapper.cpp:1751`),
  senão `HOST_STOP_SKILL`. Para o Arqueiro o alcance é o da arma.
- **Carga (B52)**: `time_type = 3` (14 habilidades, ex.: 234). Soltar antes (`CONTINUE_ACTION`,
  C2S 51) conclui na hora com `carga = tempo carregado / tempo do 1º estado`, que escala o
  `ratio` (`GetCharging`). Conjuração tem marcador: a tarefa de fim só conclui a que ainda está
  aberta.
- Mana: `GetMpcost` do stub, senão a tabela antiga.
- **16** habilidades da tabela antiga (`Habilidade::conhecida`) continuam valendo para cura e
  para o que o stub não deu conta; as demais sem conta conjuram e **não fazem efeito**.
- Nível: `PlayerEntity::habilidades` (do `character_skills`); piso 1 para quem não aprendeu;
  teto 10.
- **Ordem do fim da conjuração (B57)**: `SKILL_PERFORM` (88) → **efeito e resultado** → 
  `HOST_STOP_SKILL` (123) → golpe da fila. É a do original: o dano sai do `RunSkill`
  (`session_skill::RepeatSession`, `actsession.cpp:576-600`) e só depois o `EndSession` manda
  `stop_skill` (`actsession.cpp:558-574`). Mandando o 123 antes, o cliente retomava o golpe
  normal antes de a habilidade ter efeito.
- **Cancelamento de conjuração por ESC e movimento (B67) — `testado`**: ESC (`CANCEL_ACTION`, C2S 42)
  e movimento do jogador (`PLAYER_MOVE`, C2S 0) cancelam qualquer conjuração em andamento
  (`p.conjuracao.take()`). O servidor envia `SELF_SKILL_INTERRUPTED` (opcode 87, reason 2) para o
  conjurador (destravando a barra de conjuração no cliente imediatamente, como no Portal da
  Cidade ou habilidades de combate) e `SKILL_INTERRUPTED` (opcode 86) para outros jogadores
  ao redor (`playercmd.cpp:2136-2153`, `player.cpp:4017-4028`).
- **Aprendizado pelo menu R (B67) — `testado`**: O cliente 1.5.5 (`EC_HostSkillModel.cpp:558-605`)
  requer `SCENE_SERVICE_NPC_LIST` (opcode 390) no login (`todos_os_dados`) para vincular os
  NPCs provedores de serviço da cena (`m_allProfNPCs`) em `m_skillLearnNPCNID`. Com o mestre da
  classe do jogador presente na cena, o cliente habilita a evolução de habilidades diretamente
  pelo menu (tecla R / `IsSkillServedByNPC`). O servidor valida se a habilidade pertence à
  classe do jogador e debita SP e moedas normalmente.
- Custo de mana como o cliente arredonda. Resultado: `SELF_SKILL_ATTACK_RESULT` (142) para
  quem conjura, `HOST_SKILL_ATTACKED` (144) para o alvo (`cEquipment = 0x7f`: sem desgaste).
- **Tabela do servidor** (`pw_data_loader::habilidades`, spec 03 §3.12), para as 3.316:
  - tempo de conjuração no `OBJECT_CAST_SKILL` = `State1::GetTime` do nível (`skill.cpp:797`);
    sem valor na tabela, o de `habilidades.rs`, e por fim 1000 ms;
  - **recarga** conferida antes de conjurar e armada em `id + 1024` com
    `(int)(0,001 × coolingtime) × 1000` (`skillwrapper.cpp:261`, `playerwrapper.cpp:170`):
    `SET_COOLDOWN` (198) ao cliente; em recarga, `ERROR_MESSAGE` 53 e `HOST_STOP_SKILL`. `testado`.
- **Pelo stub (B53)** — `bus_server/habilidades.rs`, o desenho de `PlayerWrapper::SetPerform`
  (`playerwrapper.cpp:170-420`), quando o stub tem dano, `StateAttack` ou `BlessMe` lidos:
  - **flechas**: `arrowcost` sai do slot 11 antes do golpe; faltando, a habilidade não sai
    (`CanAttack(cost)`, `skill.cpp:213-231`); `ATTACK_ONCE` com as gastas;
  - **precisão** × `GetHitrate`; o dano é sorteado **uma vez** para todos os alvos;
  - **área** por `range.type`: 0 ponto (dentro de `GetEffectdistance`), 1 linha (cilindro de
    `GetRadius` até `GetAttackdistance`), 2 bola em si e 3 bola no alvo (`GetRadius`), 4 setor
    (cone de `GetAngle`), 5 em si (`obj_interface.cpp:1066-1130`). Golpe e maldição só pegam
    monstros, bênção só jogadores (sem trava de PvP); golpe de ponto em jogador segue o
    caminho antigo;
  - **efeitos**: `doenchant` roda o `StateAttack` em cada alvo **atingido**; bênção/maldição em
    cada alvo, com `ENCHANT_RESULT` (139); `dobless` roda o `BlessMe` em quem conjura;
  - resultado `SELF_SKILL_ATTACK_RESULT` a quem conjura e `OBJECT_SKILL_ATTACK_RESULT` a
    todos; morte pelo caminho do abate.
- **Efeitos de estado (B53) — `testado`** (`crate::efeitos`, os `filter` de
  `cskill/skill/skillfilter.h`). O roteiro (`SetProbability/Time/Ratio/Value/Amount/Showicon`
  e `SetX`) é avaliado por nível com as variáveis de quem conjura, da vítima e da habilidade;
  o dado fixa a probabilidade em 100/0 (`ThrowDice`). **37 efeitos portados**: lento/rápido,
  preso, atordoado, sono (acorda com dano), selado, veneno/sangramento/queimadura/gelo/trovão/
  terra (dano no tempo reduzido pela defesa/resistência, punição de nível contra monstro, ¼
  entre jogadores, tique de 3 em 3 s), ± ataque/magia/defesa/resistência/evasão/precisão,
  ± cadência e conjuração, ± dano recebido, crítico, regeneração de vida/mana, ± vida máxima,
  poder, invencível, **escudo de asa** (`Wingshield`, B73: absorve o golpe até o `SetAmount`
  acabar — um quinto passa e o escudo perde quatro vezes isso — e injeta o `SetValue` de mana
  a cada 3 s, `skillfilter.h:4136-4232`); instantâneos cura, cura/mana em %, dano direto,
  limpar bênçãos/maldições.
  Convivência de `filter_man::AddFilter` (único substitui, fraco descarta, fundir absorve).
  Realces entram como `_en_percent` na conta do jogador e em monstro (NPC usa o mesmo
  `property_policy` com classe −1). Monstro atordoado/dormindo não age, preso não anda, lento
  anda mais devagar. A cada 1 s o mundo desconta o tempo, aplica dano/cura e avisa:
  `UPDATE_EXT_STATE` (124), `ICON_STATE_NOTIFY` (125), vida, e para jogador a ficha e a
  velocidade. Morrer limpa tudo. Selado/atordoado não conjura.
- `falta`: os ~300 efeitos sem porte (formas, invocação, escudos, recargas) — vão ao log
  `habilidade N — sem porte: ...`; imunidades do monstro; recarga comum (`commoncooldown`);
  Portal da Cidade (167); talentos (`GetT0..T2` valem 0, `GetPrayrangeplus`); corpos de
  roteiro com `if` (2 de 2.304 no alvo).

## 7. Jogador

| regra | estado | detalhe |
| :--- | :--- | :--- |
| velocidades, cadência, alcance, `hp_gen`/`mp_gen` | `confirmado` | `CHARRACTER_CLASS_CONFIG` (Bárbaro: correr 4,9 m/s, ataque 0,8 s = 16 ticks, alcance 2,5 m) |
| atributos iniciais | `testado` (B51) | **5/5/5/5 para toda classe**, vida `vit_hp × 5` e mana `eng_mp × 5` — os 12 moldes do `clsconfig` do `pwserver_155v156`. Os atributos e o `hp`/`mp` do `ptemplate.conf` **não** chegam ao jogador (`userlogin.cpp` copia a ficha do banco). Até B50 o Arqueiro nascia com 20 de energia |
| vida/mana máximas | `testado` (B51) | `lvlup_hp × (nível−1) + vit_hp × vitalidade` (e o par da mana), base zero (`__LevelUp`, `__UpdateBasic`, `playertemplate.cpp:500-582`); `BaseDaClasse::vida_e_mana_maximas` (spec 03 §3.5), a mesma conta da criação |
| **combate** | `testado` | `combate_s`: atacar põe 15 s (`DoAttack`, `player.cpp:3062`), apanhar garante 5 s (`OnAttacked`, `:9514`); batimento de 1 s desconta |
| **regeneração** | `testado` | batimento de 1 s no `tick`: `hp_gen`/`mp_gen` em combate, ×4 fora (`player.cpp:9130-9137`), acumulando oitavos (`func::Update`, `actobject.h:2143`); `SELF_INFO_00` quando muda |
| **experiência e SP do abate** | `testado` | lista de dano no monstro; cada um recebe `exp × dano / max(total, max_hp)` (`DispatchExp`, `npc.cpp:1515`) com o ajuste da diferença de nível e `+0,5` (`ReceiveExp`, `player.cpp:2813`); `RECEIVE_EXP` (36) depois de somar. Sem grupo: não há divisão de equipe |
| **subida de nível** | `testado` | `IncExp`/`LevelUp` (`player.cpp:2627-2711,2831-2896`): curva `PLAYER_LEVELEXP_CONFIG` 202, +5 pontos de atributo, atributos refeitos (`recalcular_por_nivel`), vida e mana cheias, experiência zera no teto (`logic_level_limit` 105); `LEVEL_UP` (37) a todos, `SELF_INFO_00` e `OWN_EXT_PROP` ao próprio |
| reviver na cidade (C2S 4) | `testado` | ponto de cidade do distrito do `precinct.sev` que contém a posição (`ResurrectInTown`, `playercmd.cpp:112`; spec 03 §3.7); sem distrito ou distrito de outro mapa, no lugar. Vida e mana a 10 % e perda de `GetLvlupExp × exp_lost[cultivo]` (`Resurrect`, `player.cpp:8716`) |
| distribuir pontos (C2S 22) | `testado` (B51) | `PlayerSetStatusPoint` (`player.cpp:8598`): recusa se alguma parcela ou a soma passa dos livres; soma, refaz vida/mana, evasão e precisão pela agilidade; `ADD_STATUS_POINT` (51, 22 bytes) com os quatro e o que sobrou (recusa com zeros). O cliente pede `GET_EXT_PROP` (21), que responde `OWN_EXT_PROP` (`PlayerGetProperty`, `:8588`) — antes só `SELF_INFO_00`. Grava atributos e pontos juntos (`gravar_atributos`) |
| munição (golpe normal) | `testado` (B51, B71) | arma de longo alcance (`weapon_type` 1) tira 1 do slot 11 (`DoAttack`, `player.cpp:3063-3070`). A contagem **mora na sessão de ataque**, como o `item_list` em memória do original: o banco é lido uma vez ao abrir a sessão (o número que vai no `HOST_START_ATTACK`) e a baixa é persistida fora do fio, senão a latência do banco alongava a cadência (B71). O `arrow_dec` do `ATTACK_ONCE` vale **1 sempre que a arma é de longe**, com ou sem flecha sobrando: o original ignora o retorno do `DecAmount` (`:3064-3070`). A flecha só sai depois das conferências do golpe. Sem munição o golpe não é recusado (`falta`: invalidar o arco pelo equipamento); o bônus de dano da flecha não entra |
| voo | `confirmado` | pelo item no slot 12 (`EQUIPIVTR_FLYSWORD`); sem custo de mana, sem teto, `GP_STATE_FLY` fora do `state` |
| teleporte de GM (`GOTO`) | `confirmado` | `y` do cliente é marcador; altura = chão + 0,5 m (`playercmd.cpp:4926`) |
| sentar, gestos, roupa, zona segura | `confirmado` | O **modo roupa persiste** (B83): o `SWITCH_FASHION_MODE` grava o `charactermode` em `characters.character_mode` — pares `(chave, valor)` de `int32`, chave 1, e nada quando desligado (`GetPlayerCharMode`, `gs/player.cpp:12585-12612`) —, o login o relê, e ele viaja cru no `RoleInfo` da lista de personagens, que é de onde a **tela de seleção** decide desenhar roupa ou armadura (`CECLoginPlayer::Load`, `EC_LoginPlayer.cpp:172-189`). `voando` continua sem persistir, de propósito: quem relogar entra no chão |
| grupo | `testado` | estado de grupo no mundo (convite, aceite, recusa, saída) |
| teleporte e troca de mapa | `testado` (B51) | `LongJump` (`player.cpp:8617`): mesmo mapa → posição, `NOTIFY_HOSTPOS` (14, 22 bytes: `pos, tag, line`) e o mundo em volta; outro mapa **do mesmo processo** → o roteador tira o jogador do mapa de origem (some da vista, sessão e entidade) e o põe no destino: `NOTIFY_HOSTPOS` com o `tag` novo (o cliente descarrega e carrega o mundo, `JumpToInstance`), chão por baixo, grava mapa e posição na hora, e streaming completo (`global_message.cpp:111-117`). Disparado por prêmio de missão (`m_ulTransWldId`) e por missão com `m_bTransTo`. Mapa de outro processo `falta`; o grupo se desfaz na troca |
| coleta de recurso (C2S 54) | `testado` (B51) | `GATHER_MATERIAL` → confere coletores (30), ferramenta e missão de entrada (31), nível (51), distância `gather_dist` 4–20 m (2) (`matter.cpp:265-382`); tempo `Rand(time_min, time_max)` s; `PLAYER_GATHER_START` (126) a todos. Andar interrompe (`PLAYER_GATHER_STOP` 127). No fim: sucesso por `material_gain_ratio`; material por probabilidade, `num1` ou `num2` com `probability2`, limitado à pilha; `HOST_OBTAIN_ITEM` (99); o que não cabe vai ao chão do jogador; exp e SP da mina **sem** ajuste de nível; a mina some (`OBJECT_DISAPPEAR`) e renasce em `max(dwRefreshTime, 15)` s. **Colher pode acordar monstro**: os `npcgen_1..4` do `MINE_ESSENCE` (`(monstro, quantidade, raio, vida em s)`) nascem no lugar da matéria — é assim que a Flor de Safira (44566), que **não produz material nenhum**, entrega a missão 31779: ela solta o Guardião de Almas (44608), agressivo, e é dele que cai o Estame com 80 % (B76). A missão só conta o item pelo `CheckMining` quando o método é "coletar N itens" (`TaskTempl.inl:2105-2145`), que não é o caso dessa. `falta`: recarga de 500 ms, interrupção por dano e os `aggros_*` da matéria |

## 8. Itens e economia


**Todo item que entra na bolsa por prêmio de missão ou por coleta é gerado**
(`Bolsa::empilhar_gerado`, B60), como no original: `DeliverCommonItem` e a coleta usam
`generate_item_for_drop` (`task/taskman.cpp:281-303`, `player.cpp:1500-1520`), a mesma geração
do drop de monstro. Sem isso o equipamento entrava **sem bloco de dados**: o cliente mostrava a
faixa do modelo no tooltip ("Destreza +1~2") e o servidor não somava propriedade nenhuma
(relato do set Halo, 2026-09-18). Compra em loja continua sem bloco — no original ela usa
`generate_item_for_shop`, que não sorteia propriedades.

### Durabilidade — `testado` (B61)

Uma unidade na tela são **100 pontos internos** (`DURABILITY_UNIT_COUNT`, `gs/config.h:59`;
`ENDURANCE_SCALE`, `EC_IvtrTypes.h:26`). Quem gera o item multiplica por 100 no fim
(`update_require_data`, `gs/item/item_addon.h:454-458`, chamado em `generate_item_temp.h:367,
552, 644, 831`) e o cliente divide de volta arredondando para cima (`EC_IvtrEquip.cpp:281`).
**É nessa escala interna que a durabilidade fica na coluna do banco e no bloco de dados** —
sem a multiplicação, o ★Arco Real (50 no `elements.data`) aparecia como `1/1` em jogo
(relato de 2026-09-18; migração em `scripts/2026_09_18_durabilidade_na_escala_interna.sql`).
Munição é sempre `1` (`generate_item_temp.h:615`), isto é, 100 internos.

O servidor desgasta a **coluna**; o `item_info` regrava a durabilidade **do bloco** com a da
coluna antes de mandar (`BusServer::info_de`), então as duas nunca divergem. O cliente também
desconta sozinho, com os mesmos números (`WEAPON_RUIN_SPEED -2`, `ARMOR_RUIN_SPEED -25`).

| regra | detalhe |
| :--- | :--- |
| golpe normal dado | a arma perde **2** (`DURABILITY_DEC_PER_ATTACK`, `gs/config.h:61`; `weapon_item::OnAfterAttack`, `item/equip_item.cpp:978-988`), porque `DoWeaponOperation<0>` está em `FillAttackMsg` (`player.cpp:3133`) |
| golpe de habilidade | **não** gasta arma — `FillEnchantMsg` não chama o desgaste, e o original diz por quê em comentário (`player.cpp:3174`) |
| golpe recebido | `SelectRandomArmor` sorteia um slot de 1 a 10 (`EQUIP_ARMOR_START..EQUIP_ARMOR_END-1`, `gs/item.h:194-241`); com peça ali, ela perde **25** (`DURABILITY_DEC_PER_HIT`, `gs/config.h:60`), e o índice vai no `cEquipment` do `HOST_ATTACKED`; slot vazio manda `0x7f` (`player.cpp:9552-9570`) |
| onde a durabilidade mora | **no mundo**, em `PlayerEntity::pecas` (`(atual, máxima)` por slot), preenchido pelo `recalcular_equipamento`: é ele que decide o índice do `cEquipment` e a quebra. O banco acompanha numa tarefa à parte. Até o B72 cada golpe — dado ou recebido — esperava um `SELECT`+`UPDATE` antes de o cliente ver o golpe, e o original mexe na `item_list` vestida e segue (`player.cpp:94`) |
| chegou a zero | para em zero, e **uma vez** sai `EQUIP_DAMAGED` (68) com motivo 0 mais o recálculo do equipamento (`_runner->equipment_damaged` + `RefreshEquipment`, `player.cpp:9563-9567`) |
| peça acabada | não conta em nada: `equip_item::VerifyRequirement` exige `durability > 0` (`item/equip_item.cpp:60-80`) |
| aviso de durabilidade baixa | é **do cliente**, sem comando nenhum: `CECGameUIMan::RefreshBrokenList` (`EC_GameUIMan.cpp:5474-5555`) roda a cada quadro e põe na janela `Win_Broken` o ícone de toda peça com `cur <= max / 10`, amarelo (192,192,0) enquanto sobra durabilidade e vermelho (192,0,0) em zero; aljava entra abaixo de 15% de flechas, e asa/espada voadora/moda nunca entram |
| reparar | `falta`: o custo é 150 fixo e nada devolve durabilidade |

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
| reparar | `parcial` | **150 fixo** (da entidade), e **não devolve durabilidade** |
| durabilidade | `testado` (B61) | escala interna, desgaste e quebra — ver "Durabilidade" acima |
| curar no NPC | `testado` | pelos valores do jogador |
| aprender habilidade | `testado` | `skill_executor::OnServe` + `SkillStub::LearnCondition`/`Learn` (`serviceprovider.cpp:1288`, `cskill/skill/skill.cpp:14-93`): habilidade da lista do treinador (`NPC_SKILL_SERVICE`), fora de combate, nível ≤ máximo, classe, pré-requisitos, nível, SP, `rank` × cultivo, dinheiro; cobra (`SPEND_MONEY` 77, `COST_SKILL_POINT` 94) e responde `LEARN_SKILL` (95). Requisito `null` na tabela recusa |
| aljava no drop (B52) | `testado` | `QUIVER_ESSENCE` vira `id_projectile` × `Rand(num_min, num_max)` (`generate_quiver`, `generate_item_temp.h:650-667`); o 1955 do Espírito da Estrela caía cru |
| Carta da Sorte (B53) | `testado` | `TASKDICE_ESSENCE`: usar sorteia a missão por `task_lists` (`RandSelect`) e entrega pelo motor (`OnTaskCheckDeliver`); aceitou, gasta uma e responde `HOST_USE_ITEM`; recusou, `ERROR_MESSAGE` 18 e a carta fica; em combate com `no_use_in_combat`, erro 66 (`item_taskdice.cpp:12-42`) |
| equipamento sorteado no drop (B53) | `testado` | ver §5.1 |
| **drop de monstro** | `testado` | dono = maior dano (+`max_hp/4` do primeiro golpe). Itens: `drop_times` rodadas de `probability_drop_num0..3` e `drop_matters[32]` (da 2ª rodada, só índices < 16), com o ajuste de item por nível (`DropItemFromData`, `npc.cpp:2649`; `generate_item_from_monster`, `itemdataman.cpp:1191`). Moedas: `drop_times` vezes, `Rand(médio±variação)`, chance 0,7, × ajuste. Cada monte a ±2 m, no chão (`worldmanager.cpp:512-555`), `tid` 3044 para moedas, id de matéria `0xC8…` |
| item no chão | `testado` | posse do dono por **30 s**, some em **300 s** (`matter.h:62`, `matter.cpp:133`); `MATTER_ENTER_WORLD` a quem está a 120 m e no streaming; `OBJECT_DISAPPEAR` ao sumir |
| **pegar** (C2S 6 e 184) | `testado` | tipo confere, distância < 10 m, posse; moedas `PICKUP_MONEY` (30), item `PICKUP_ITEM` (31); `MATTER_PICKUP` (152) a todos; bolsa cheia `ERROR_MESSAGE` 7, fora da posse 6 (`playercmd.cpp:1347-1444`, `matter.h:97-129`) |
| **descartar** (C2S 14 e 15) | `testado` (B84) | Joga o item no chão **sem dono** — item que se joga fora é de quem pegar (`DropItemFromData` com `XID(0,0)`, `ThrowEquipItem`, `gs/player.cpp:7932-7980`). A bolsa manda `{u8 índice, u32 quantos}`; o corpo só o índice, e vai a peça inteira. Responde `PLAYER_DROP_ITEM` (46) com `DROP_TYPE_PLAYER` = 1 **e** o `UNFREEZE_IVTR_SLOT` (181) |
| **congelamento de slot** | `testado` (B84) | **O cliente congela o slot antes de mandar qualquer comando de item** (`c2s_CmdDropIvtrItem` e os vizinhos, `Network/EC_GameSession.cpp:6304-6390`), e só o `UNFREEZE_IVTR_SLOT` (181) ou o fim de uma troca limpam esse estado (`EC_HostMsg.cpp:2060-2064`). Item congelado fica apagado e não pode ser movido nem usado. Portanto: **todo tratador de comando de item devolve o 181, inclusive quando desiste** — slot vazio, falha de banco, comando não tratado. O original faz isso com o `UnLockInventoryHandler` (`gs/playercmd.cpp:183-230`, chamado também no estado de morto, `:654-678`); do lado do servidor o comando chama-se `unlock_inventory_slot` (`gs/player.cpp:5059-5066`). `falta`: os comandos de **armazém**, que congelam e ainda não são tratados nem destravados |
| poção (`USE_ITEM`) | `testado` (B67, B71) | `MEDICINE_ESSENCE`. **Restaura ao longo do tempo**: `hp_add_total / hp_add_time` por batimento de 1 s, e o mesmo para mana — é o `healing_potion_filter`/`mana_potion_filter` do original (`gs/item/item_potion.cpp:18-52`, `gs/potion_filter.h:6-130`), que reparte o total pelo tempo. Só a poção com vida **e** mana e sem tempo (`rejuvenation_potion`) cura na hora. **Recarga** (B70/B71): `CheckCoolDown` **antes** de consumir, recusa com `ERR_OBJECT_IS_COOLING` (53) e `SetCoolDown(índice, cool_time)` com `SET_COOLDOWN` (198) ao cliente. O índice é o da **família**, e a família vem do `id_major_type` do arquivo (`setclassid.cpp:81-101`), não do que a poção restaura: 11 vida, 12 mana, 3 vida+mana, 13 antídoto (`COOLDOWN_INDEX_*`, `gs/cooldowncfg.h:62-78`) — poções da mesma família compartilham a recarga |
| Daimon (`GOBLIN_ESSENCE`) | `parcial` (B75) | O "pequeno elfo" do original (`elf_item`), vestido no slot **23** (`EQUIP_INDEX_ELF`, `gs/item.h:219`). O estado dele **é** o bloco do item: `elf_essence` de 38 bytes com `#pragma pack(1)` (exp, nível, total de atributos, força/agilidade/vitalidade/energia, total de gênios, 5 gênios, refino, vigor 20000, estado), depois a lista de equipamento e a de habilidades, cada uma com a contagem em 4 bytes (`elf_item::Save`, `gs/item/item_elf.cpp:172-185`; `generate_elf`, `generate_item_temp.h:2442-2524`). **Experiência:** um décimo da que o jogador ganha, sempre (`ElfReceiveExp(exp / 10)`, `gs/player.cpp:2921-2928`); `InsertExp` (`item_elf.cpp:692-750`) aplica o fator `nível do Daimon ÷ nível de quem deu` (mínimo 10 %), sobe quantos níveis couberem e **para a um ponto** de alcançar o dono. Cada nível dá 1 ponto de atributo, e 1 de gênio a cada 5 níveis até o 100. Sem subir de nível vai `ELF_EXP` (283) ao cliente; subindo, a ficha inteira do item. `falta`: bônus de atributo sorteado de 10 em 10 níveis (`rand_prop`), equipamento e habilidades do Daimon, vigor, pílulas, decomposição, refino e distribuir pontos |
| montaria (`SUMMON_PET` C2S 100 / `RECALL_PET` 101) | `testado` (B78, B79) | invocar um mascote de montaria **é montar** (`PlayerSummonPet` → `pet_man::ActivePet`, `gs/player.cpp:14474-14491`, `gs/petman.cpp:319-392`). **É uma sessão, não um comando** (`session_summon_pet`, `gs/actsession.cpp:1705-1721`): o servidor só confere que o mascote existe, manda `PLAYER_START_PET_OP` (235, `{slot, pet_id, delay, op}`) e espera a canalização — **60 ticks** para invocar, 10 para recolher, em ticks de 50 ms (`TICK_PER_SEC 20`, `gs/config.h:43`) —; só então aplica o efeito, e fecha com `PLAYER_STOP_PET_OP` (236), **mesmo quando recusa**. Aplicar é: `PLAYER_MOUNTING` (227, `{id, mount_id, u16 color}`) mais a ficha nova, e depois **`SUMMON_PET` (233, `{slot, pet_tid, pet_pid, life_time}`)**, que é o que diz ao cliente qual mascote ficou ativo (`SetActivePetIndex`, `EC_HostMsg.cpp:5274-5296`) — sem ele o botão de recolher da jaula fica desabilitado (`DlgPetList.cpp:227`) e invocar de novo responde "já está ativo" (B79). Desmontar manda `PLAYER_MOUNTING` com zero nos dois e `RECALL_PET` (234, `{slot, pet_tid, u8 motivo}`, 9 bytes de corpo) com `PET_RECALL_DEFAULT` = 0 (`mount_filter.cpp:24-45`, `player.cpp:14279-14319`, `petman.cpp:1337`, `:1376`). A velocidade é `speed_a + speed_b × (nível − 1)` do `PET_ESSENCE` (`pet_dataman::CalcMountParam`, `gs/petdataman.h:186-194`) e **sobrepõe** a de corrida. Um pedido novo invalida a canalização aberta, como o `AddSession` do original. Recusa com `ERR_PET_IS_NOT_EXIST` (72), `ERR_PET_IS_NOT_ACTIVE` (73) ou `ERR_PET_CAN_NOT_MOUNT` (81) — note que o original **não** recusa por mascote já ativo ao invocar (a linha está comentada em `player.cpp:14476`): quem já tem um troca de montaria. Quem entra no campo de visão **depois** também vê a montaria: o `mount_id`/`mount_color` vão no `object_state` do `info_player_1` (B80, spec 04). `falta`: mascote de **combate** (invocar a criatura), invisibilidade, transformação, a trava de ataque enquanto montado e a queda por lealdade. **Água (B88):** montaria terrestre **não entra na água** e **cai** se a água subir. Os dois limiares são os do `gplayer_imp::TestUnderWater` (`gs/player.cpp:14336-14342`), sobre `off = altura da água − y`: acima de **0,5 m** o jogador conta como submerso e a invocação é recusada com `ERR_PET_CAN_NOT_MOUNT` (`mount_petdata_imp::DoActivePet`, `gs/petman.cpp:344-348`); acima de **1 m** a montaria cai (`TestUnderWater`, `:402-410`). A conferência roda no batimento de 1 s, e a queda manda `PLAYER_MOUNTING(0,0)` **e** `RECALL_PET` — o original só limpa o estado interno, o que deixaria a jaula travada (B79). A altura da água vem do `watermap/` (spec 03 §3.6b) |
| amuleto e hierograma | `testado` (B67, B73) | `AUTOHP_ESSENCE`/`AUTOMP_ESSENCE`: o conteúdo do item são **8 bytes**, `int point; float trigger_percent` (`gs/item/item_amulet.h:16-19`, `generate_item_temp.h:2296-2310`). Sem eles o cliente desenhava zeros e negativos. **Disparo automático (B73)**: vesti-los nos slots **20** (vida) e **21** (mana) os ativa (`OnActivate` → `SetHPAutoGen`/`SetMPAutoGen`, `item_amulet.cpp:22-46`); a cada batimento de 1 s, com `trigger_percent × máximo > atual`, o `AutoGenStat` (`gs/player_imp.h:3562-3593`) confere a recarga (`COOLDOWN_INDEX_AUTO_HP` 24 / `AUTO_MP` 25), devolve `máximo − atual` preso ao que resta e arma o `cool_time` do item — e o `SetCoolDown` **sempre** manda `SET_COOLDOWN` (198) ao cliente (`gs/player.cpp:12701-12709`), que é o que escurece o ícone (B74). O que sobra fica nos **octetos do item**; em zero o amuleto some do corpo com `PLAYER_DROP_ITEM` tipo `DROP_TYPE_USE` (11) |
| colher recurso de mapa | `testado` (B51) | §7 "coleta de recurso" |
| Loja Gold, barraca | `falta` | |
| demais serviços de NPC (teleporte, pedras, forja, decompor, armazém, item de missão) | `falta` | |

### 8.0 A barra de chi — `testado` (B69/B70/B73)

O chi (a "fúria" do original, `_basic.ap`) **não existe até uma missão dar o teto**: é o
prêmio `m_ulFuryULimit` (deslocamento 57 do `AWARD_DATA`) → `SetFuryUpperLimit` →
`gplayer_imp::SetMaxAP` (`gs/task/taskman.cpp:498-501`, `actobject.h:1634-1640`). No
`realm_155` são 8 missões, e a primeira é a 32394 "Só um Pouco de Progresso" (nível 9, teto
99); depois 199, 299 e 399 (`cargo run -p pw-gs --example missoes_de_chi`).

| ganho | quanto | origem |
| :--- | :--- | :--- |
| golpe normal | `ap_per_hit` da classe (`angro_increase` do `CHARRACTER_CLASS_CONFIG`; Arqueiro: 5) | `gplayer_imp::DoAttack`, `player.cpp:3091-3093` |
| meditar | **15 por batimento de 1 s** | `sit_down_filter::Heartbeat`, `gs/sitdown_filter.cpp:19-34` |
| **usar habilidade** | `apgain − apcost` do stub, de uma vez, na execução | `int ap = GetApgain() - GetApcost(); if (ap) ModifyAP(ap)`, `cskill/skill/playerwrapper.cpp:170-177` |
| habilidade (filtros) | `Apgen`/`Apgen2` (`falta`: nenhum porte ainda) | `playerwrapper.cpp` |

Cada habilidade tem **`apcost` e `apgain` fixos** no stub (`cskill/skill/skill.h:239,588`),
e o servidor recusa a conjuração com `GetAp() < apcost` (`SkillStub::Condition`,
`cskill/skill/skill.cpp:125`) — **sem mandar erro**, porque o cliente já barra antes. Os dois
números já estavam no `habilidades.json`; o mundo é que não os lia até o B73. Exemplos do
Arqueiro: Flecha Glacial (245) custa **25**, Barreira de Asa (249) custa **45**, Flecha
Fulgurante (244) **dá 10** e a 235 **dá 5**.

`ModifyAP` prende entre 0 e o teto e marca o estado para ir ao cliente. O `iAP`/`iMaxAP` do
`SELF_INFO_00` (38) e o último `i32` (`max_ap`) do `OWN_EXT_PROP` (50) têm de levar o mesmo
teto; `EC_HostMsg.cpp:1319-1332` compara os dois e anuncia aumento quando o segundo muda. Até
o B70, o segundo ia zero, então cada atualização posterior de `SELF_INFO_00` fazia o cliente
repetir “limite máximo de chi aumentado para 99”. Vive em `characters.ap`/`characters.max_ap`.
**Correção do B70:** ficou escrito aqui que a Flecha Fulgurante (244) não gerava chi, porque
o corpo da habilidade não chama `ModifyAP`. Quem chama é a execução, com o `apgain` do stub —
e o da 244 é **10** (`cskill/skills/skill244.h:142-144`). **Não há ganho ao apanhar** no
1.5.5: nenhuma das chamadas de `ModifyAP` está no caminho de levar dano.

Os filtros `Apgen`/`Apgen2` seguem pendentes para as habilidades que efetivamente os usam.

### 8.1 Pontos de teleporte — `testado` (B68)

O jogador guarda os pontos que já descobriu (`_waypoint_list`, `gs/player_imp.h:2520-2550`),
hoje na coluna `characters.waypoints` (u16 little-endian em sequência, o mesmo formato do
`GetWaypointBuffer`).

| parte | estado | detalhe |
| :--- | :--- | :--- |
| descobrir | `testado` | o cliente manda os pontos da região (`ACTIVATE_REGION_WAYPOINTS`, C2S 178) e o mundo ativa os que faltam, respondendo um `ACTIVATE_WAYPOINT` (179) por ponto — é o comando que faz o cliente anunciar o ponto novo (`gs/player.cpp:25196-25220`) |
| lembrar | `testado` | a lista é gravada a cada ponto novo e volta no `WAYPOINT_LIST` (180) da carga inicial |
| validar por região | `falta` | o original cruza com `world_manager::GetRegionWaypoints()`; aceitamos o que o cliente diz haver na região dele |
| **viajar** (transportadora) | `testado` | `GP_NPCSEV_TRANSMIT` (5): o cliente manda só o **índice** do destino na lista daquele NPC; o servidor confere índice, nível e dinheiro, cobra e teleporta (`transmit_provider::TryServe` e `transmit_executor::OnServe`, `gs/serviceprovider.cpp:771-852`). Os destinos são o `NPC_TRANSMIT_SERVICE` do `elements.data` (`idTarget`, `fee`, `required_level`, até 32) e a **coordenada de cada um** está no `world_targets.sev` — 92 pontos no `realm_155`, e os 427 destinos citados pelas 95 transportadoras estão todos lá |

## 9. Persistência

- **Autosave a cada 60 s** por mundo: `save_status` com nível, cultivo, exp, SP, vida, mana,
  moedas, mundo e posição, mais os quatro atributos com `potential_points` e as listas de
  missão. Falha vira `warn!`
  com contagem — não "sucesso" (B36f).
- Movimento **não** grava por pacote.
- Operações de `com_contexto` (missão, abate, coleta, loja, aprender) gravam na hora: bolsas
  antes de responder, estado e listas numa tarefa.
- Listas de missão: `character_task_lists` (cinco `BYTEA`, os blocos do `TASK_DATA`,
  `scripts/2026_09_14_listas_de_missao.sql`); lidas no `EnterWorld` pelo mundo e pelo link.
- Personagem novo: `class_templates` do realm (posição, kit, arma) + atributos 5/5/5/5 com a
  vida e a mana do `clsconfig` (spec 02 §4).
- Barras de atalho, layout e opções do cliente: `character_client_config` (spec 02 §4).

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
| `TASK_NOTIFY` 1/2/3/4/5/10 | concluir (`OnTaskCheckAwardDirect`), desistir, **chegou ao lugar** e **saiu do lugar** (`OnTaskReachSite`/`LeaveSite`, `TaskServer.cpp:466-520`: confere mundo e caixa, `OnSetFinished`), entrega automática, gatilho manual; 7 = marca dinâmica (B49) | `testado` |
| horário (B51) | `CheckTimetable`/`judge_time_date` (`TaskTempl.h:1697`) na hora local do contêiner (`TZ=America/Sao_Paulo`): por data, mês, semana (1 = segunda … 7 = domingo) e dia; basta uma janela | `testado` |
| região de entrega (B51) | `CheckInZone` (`TaskTempl.inl:368`): mundo `m_ulDelvWorld` e alguma caixa `m_pDelvRegion` (bordas incluídas) | `testado` |
| facção (B51) | `CheckFaction` (`TaskTempl.inl:718`): em facção (`id_mafia != 0`) com cargo ≤ `m_iPremise_FactionRole`. **Não há sistema de facção**, então quem pede facção é recusado — como no original para quem não tem | `testado` |
| equipe (B51) | `CheckTeamTask`/`HasAllTeamMemsWanted` (`TaskTempl.inl:149-339`): só o capitão recebe; distância dos membros, `TEAM_MEM_WANTED` (nível, raça/classe, gênero, contagem), classes distintas; casal recusa (sem casamento). Aceita, cada membro **deste mapa** recebe por `OnDeliverTeamMemTask` (`TaskProcess.cpp:1592`) | `testado` |
| teleporte (B51) | prêmio `m_ulTransWldId` (`TaskProcess.cpp:1316`) e `m_bTransTo` ao receber (`:1843`) → §7 "teleporte e troca de mapa" | `testado` |
| coleta de mina (`OnTaskMining`) (B67) | mina com `task_out > 0` (`TaskServer.cpp:1116-1123`, `TaskTempl.inl:2105-2148`); se `material.item == 0`, não dropa nada no chão; entrega o item da submissão na bolsa (`j.dar_item`) e marca a submissão finalizada | `testado` |
| itens de missão | **quem escolhe a bolsa é o `m_bCommonItem` de cada item do `tasks.data`**, não o tipo do item (embora os dois quase sempre concordem: no `realm_155`, **todos** os 413 `TASKMATTER_ESSENCE` vão para a bolsa de missão e os `TASKNORMALMATTER_ESSENCE` para a comum — "matéria de missão **normal**" é a que fica no inventário normal —, com uma única exceção, a Presa de Filhote de Lobo 2654; ver `cargo run -p pw-data-loader --example bolsa_do_item_de_missao`): `true` → `DeliverCommonItem` → bolsa normal; `false` → `DeliverTaskItem` → bolsa de missão (`task/TaskProcess.cpp:1190-1208`, `task/taskman.cpp:281-330`). O mesmo bit vale para contar e recolher. Bolsa de missão = pacote 2, `container_type` 5: `TASK_DELIVER_ITEM` (156), `PLAYER_DROP_ITEM` (46) tipo 3; prêmio `TASK_DELIVER_EXP/MONEY` (158/159), `SPEND_MONEY` | `testado` |
| erros | `svr_task_err_code` (reason 6) com `TASK_PREREQU_FAIL_*`; NPC sem a missão `ERROR_MESSAGE` 19 | `testado` |
| monstros invocados | `m_SummonedMonsters` do prêmio: com `m_bRandChoose` sorteia um quando as probabilidades somam 1 e senão sorteia cada um; **sem** ele invoca todos (`TaskProcess.cpp:1385-1436`). O id do invocado satisfaz `ISNPCID` (faixa `0xA000_0000`) — com `0xC000_0000` o cliente o lia como item de chão e não deixava mirar (B67) | `testado` |
| **nível de cultivo** (B67) | `m_ulNewPeriod` do prêmio (deslocamento 25 do `AWARD_DATA`) → `SetCurPeriod` → `gplayer_imp::SetSecLevel` (`TaskProcess.cpp:1284`, `task/taskman.cpp:251-254`, `player_imp.h:2798-2804`): grava em `characters.cultivation` e manda `TASK_DELIVER_LEVEL2` (160), que faz o cliente tocar o efeito do avanço. São 18 missões no `realm_155` (`cargo run -p pw-gs --example missoes_de_cultivo`) | `testado` |

Contra o `tasks.data` real (`tests/missoes_do_realm.rs`): o Arqueiro nível 1 aceita e entrega
32201 (25 exp, 10 SP, 8 moedas); mais de 1.000 missões de topo entregam sem quebrar os
índices da lista.

### Quem decide a entrega automática é o cliente

A varredura das missões de entrega automática roda **no cliente**, a cada tique do
`CECHostPlayer::Tick` (`ATaskTemplMan::CheckAutoDelv`, `Task/TaskTemplMan.cpp:106-131`, via
`OnTaskCheckStatus`): ele percorre o `m_AutoDelvMap`, roda o `CheckPrerequisite` dele mesmo e
só então manda `TASK_NOTIFY` com `AUTO_DELV` para o servidor conferir de novo e entregar. Duas
consequências que já custaram uma sessão de teste cada:

- O cliente **não varre nada** enquanto não tiver o dado de títulos: `UpdateStatus` começa com
  `if (!pTask->IsTitleDataReady()) return;` (`TaskTemplMan.cpp:1350`), e o sinal só liga ao
  chegar o `QUERY_TITLE_RE` (spec 04 §5) — B60.
- A **zona de entrega é conferida ali**, com a posição do cliente (`CheckInZone`,
  `TaskTempl.inl:368-393`): uma missão automática com `m_bDelvInZone` só é pedida quando o
  jogador está dentro de uma das caixas, no mundo certo. É o caso da 31690 "Descobertas
  Acidentais", a continuação da linha principal dos Alados: nível 4–8, mundo 161, duas caixas
  ao norte do Terraço dos Heróis. Fora delas nada acontece, e nada está quebrado (B61). O
  exemplo `cargo run -p pw-gs --example missoes_automaticas` mostra o que o cliente pediria de
  uma dada posição, e por que recusa o resto.

`falta` (recusado ou ignorado, nunca inventado): casamento, PQ, torre, variáveis globais
(lidas como 0), prêmio por escala de tempo/itens (vazio), invocação de monstros de prêmio,
`FORCE_GIVEUP`, falha por morte, limite de tempo checado só na entrega, sucesso/falha
compartilhados pela equipe (`AwardNotifyTeamMem`), abate contado para a equipe, força
(`m_iForce`), região de entrar/sair que falha a missão, sistema de facção.
