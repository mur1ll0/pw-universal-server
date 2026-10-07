# Especificação 05: Simulação do mundo (`pw-gs`)

> Presença/consulta administrativa e coordenação/consumo de GM verificadas em 2026-10-05, base `a010ff7` + B166/B169/B170 sem commit.

> Passivas de forma, `SetAp`, Portal da Cidade 1.2.6 e roteiros do `gs` 1.2.6 testados em 2026-09-26, base `ff778c1` + B122; Mascote de combate (habilidades, soltar, renomear, aprender/esquecer) testado em 2026-09-25, base `ca082f8` + B112; Ficha, dano da 299 e aviso de abate v126 testados em 2026-09-24, base `eee918e` + B101; tempos da 299 v126 (B100); pipeline B98; itens/combate 126 conferidos em 2026-09-21, base `a305e51` + B89/B90; missão inicial v55 testada em 2026-09-23 (B96); demais áreas em 2026-09-14, B50. Cobre
> `crates/pw-gs/src/{world,bus_server,bus_server/jogo,ai,combat,habilidades,entity,grid,npc,server,missoes,progressao,economia}.rs`.
>
> Estado de cada regra: `confirmado` (visto em jogo), `testado` (teste automatizado),
> `parcial`, `falta`. Toda regra portada cita o fonte original no código.

## 1. Estrutura

No `GET_ALL_DATA`, a resposta `SCENE_SERVICE_NPC_LIST` é uma opção do
`WorldProtocol`: o mundo só transmite quando há NPCs remotos e a estratégia
retorna pacote. V126 retorna None (id 390 ausente no binário); o padrão mantém
a resposta 155. Nenhuma regra de mundo consulta a versão (B74-126).

| peça | o que é |
| :--- | :--- |
| `server.rs` | laço de **50 ms**: `world.tick(50)`, um por mapa |
| `mapas.rs` (`RoteadorDeMapas`) | entrega cada `roleid` ao mapa; guarda de presença durante entrada, logout, queda do link e troca, para snapshots administrativos coerentes (B166) |
| `administracao.rs` | canal opcional somente de consulta, autenticado por realm; copia ficha viva sem I/O sob lock do mundo (spec 06) |
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
- Monstro morto (`WorldInstance::matar_monstro`), **pelo gerador** (B105): o corpo dura o
  `iDeadTime` do `npcgen.data` (o construtor põe 20 s, mas o `CreateMobBase` sobrescreve com o
  da entrada — `npcgenerator.cpp:2486`; no `gs` 1.2.6, `npc_spawner::CreateMobBase` VA
  0x80f2407); limitado a 10..10.800 s e a 200 s no `OnDeath`. **0 = sem corpo** (27.610 de
  27.618 monstros do mapa 1 do 126): nenhum `OBJECT_DISAPPEAR`, o cliente mantém o corpo, e o
  monstro volta ao gerador no tique seguinte. O renascimento é sorteado entre 15 s +
  `iRefreshLower` e 15 s + `iRefresh` (`BASE_REBORN_TIME`, `npcgenerator.cpp:3355`,
  `3841-3855`), contado da volta ao gerador, **num ponto novo da área** (`Reborn` →
  `GeneratePos`/`GenDir`) e anunciado com `NPC_ENTER_WORLD` (16) a quem está a 120 m — o corpo
  "levanta" noutro lugar, como no original. **Filtros (B132):** toda morte de monstro (golpe
  normal, habilidade, dano no tempo, `matar_monstro`) passa por `Efeitos::ao_morrer`
  (`gnpc_imp::OnDeath`), e o renascimento tira as maldições (`Efeitos::ao_renascer`,
  `gnpc_imp::Reborn` → `ClearSpecFilter(FILTER_MASK_DEBUFF)`, `npc.cpp:1978`) — antes o
  sangramento de quem matou com golpe normal voltava com o monstro e o punha com ódio (captura do 1.2.6: Filhote de Mandrágora de volta
  ~15,5 s após a morte, a 3–19 m, com 16 e sem 21). Sem gerador (invocado): corpo de 20 s e não
  renasce. `testado` (`reproducao_da_planta_devoradora_no_realm_126`, `#[ignore]`: de volta
  15,0 s depois, a 16,2 m, sem `disappear`). Até o B104 era corpo fixo de 20 s + `disappear` +
  `iRefresh` (mínimo 1 s) no mesmo ponto: a Planta reaparecia 1 s depois de o corpo sumir.

## 3. Visibilidade (streaming) — `confirmado`

`BusServer::atualizar_visiveis`, chamado a cada movimento:

| regra | valor | por quê |
| :--- | :--- | :--- |
| área (B143) | o quadrado de **±3 fatias de 25 m** em volta da fatia do jogador (`na_visao`, `FATIAS_DE_VISAO`): `BuildSliceMask(near, grid_sight_range)` monta os anéis até `ceil(60 / 25)` = 3 (`gs/world.cpp:231-262`, `GRID_SIGHT_RANGE` 60 em `config.h:20`, fatia de 25 m no `grid` do `gs.conf`) | é o conjunto que o `MoveBetweenSlice` (`world.h:636-690`) faz entrar e sair |
| quando recalcula | ao **trocar de fatia** (ou forçado: entrada, teleporte) | `MoveBetweenSlice` |
| teto | **nenhum** (até o B142: 80 criaturas e 40 matérias). O envio do streaming espera a fila (`responder_com_espera`, até 2 s) em vez de descartar | o original não tem teto; a captura 1.2.6 mostra o cliente com até 220 criaturas |
| jogadores | visibilidade **mútua** — quem se move escreve nos dois `visiveis`; a vista em fatias é simétrica | simetria |
| comandos | spec 04 §5 (11/12/18 entram; 13/34/19 saem) | |
| alcance dos avisos | movimento e parada de monstro vão **só a quem o vê** (`transmitir_a_quem_ve`) | o original difunde na fatia do NPC (`AutoBroadcastCSMsg`, `npc.cpp:85-98`). Mandando ao mapa inteiro, o cliente recebia comando de monstro que nunca viu entrar, o punha na fila de "NPC desconhecido" e perguntava por ele de 10 em 10 s para sempre (`EC_ManNPC.cpp:967-975`, `1144-1164`) — 319 comandos assim no teste de 2026-09-17 (B57) |

Saída do jogo: `tirar_da_vista_de_todos` (no `LOGOUT` e no `PlayerLogout`).
Direção: a criatura leva a do gerador desde o B59 (§2); o jogador, o `dir` do último `STOP_MOVE`
(`pPlayer->dir = dir`, `gs/player.cpp:3655`; `PlayerEntity::direcao`, B142). `falta`: jogadores
visíveis por link na fala não separam mundos.

## 4. IA de monstro (`ai.rs`, `politica.rs`) — `testado` (B126 e B127 não publicados)

Regras de `gs/aipolicy.cpp`, `gs/ainpc.cpp`, `gs/npcsession.cpp`:

| comportamento | regra |
| :--- | :--- |
| perseguir | a cada `PASSO_DE_PERSEGUICAO_MS` **500 ms** avança `run_speed × 0,5` m (`session_npc_follow_target`); para ao entrar no alcance. **De ar/água (B133):** `follow_target` com o `CNPCChaseOnAirPFAgent`/`InWaterPFAgent` (`NPCMoveAgent.cpp:68-90`, `navegacao::SeguirNoEspaco`): reta se o `airmap/` deixa (`CanGoStraightForward`), senão busca gulosa de Manhattan nos 26 vizinhos de voxel (lista aberta de 60, até 100.000 voxels, `SpatialPathFinding.cpp`) + trajeto; o passo reto recusa sair do ar/água (`IsPosOnAir`/`IsPosInWater`, folga −0,001). Se a meta recuada e os 11 sorteios na esfera caem no bloqueado, o original **vai em reta** e atravessa a octree — reproduzido. A volta para casa do monstro de ar continua reta (sem porte). **O passo reto de ar/água tem o `AdjustCurPos` (B136):** depois de cada passo aceito o de ar sobe a `max(terreno, água) + 0,2` e o de água fica entre `terreno + 0,2` e `água − 0,2` (`ABOVE_DIST`/`BELOW_DIST`, `NPCChaseOnAirAgent.h:27-36`, `NPCChaseInWaterAgent.h:27-49`), também na parada de chegada (`AdjustGetToGoalPos`). Sem ele o de ar descia aos pés do alvo e ficava rente ao chão. Numa crista entre ele e os pés do alvo o passo seguinte ainda entra no terreno e a perseguição falha como no original (`IsPosBeyondEnv`), até o alvo sair dali |
| altura do passo | monstro de chão: chão do `.hmap` **+ o piso do `movemap`** (em cima de ponte e estrutura, `CNPCMoveMap::Get3DPosOnGround`, B97); água/ar: segue o alvo sem descer abaixo desse piso |
| desistir (B128) | `testado`. Temporizador de ódio `_cur_time` = `aggro_time` do `MONSTER_ESSENCE` (piso 1; 15 s nos monstros do começo), descontado 1 por batimento (`aggro_policy::OnHeartbeat`, `ainpc.h:259-275`); ao zerar sai o primeiro da lista e, sobrando alguém, conta de novo. Renovam: a lista passar de vazia a cheia, quem **está no topo** ganhar ódio (`AddRage(...) == 0`, `ainpc.h:188-231`) e o golpe/habilidade do monstro nele (`RefreshAggroTimer`, `npcsession.cpp:73/284/690/741`). Quem só foge ou voa fora do alcance é esquecido em `aggro_time` s. Alvo mais longe que `aggro_range` (o `GetIgnoreRange`, `aipolicy.cpp:577-585`) sai da lista na hora |
| desistir de quem não alcança (B134) | `testado`. `ai_target_task::OnSessionEnd` (`aipolicy.cpp:443-480`): a perseguição que acaba em `NSRC_ERR_PATHFINDING` — o agente desistiu, ou "chegou" à meta três vezes seguidas sem alcançar (`TEST_GETTOGOAL`, `npcsession.cpp:166-177`), o caso de quem bate voando sobre monstro de chão (a meta do agente de chão é medida no plano) — contra **jogador** faz `ClearAggro` + `ClearDamageList` e acaba a tarefa (→ `RollBack`); contra mascote, a tarefa só recomeça. Antes o monstro ficava indo e vindo embaixo do jogador sem bater em ninguém |
| `RollBack` (B128) | `testado`. Lista vazia depois de combate (`aipolicy.cpp:204-231`): a mais de **10 m** de casa (`IsReturnHome`, `ainpc.cpp:28-37`, 10² ao quadrado) abre a volta (`ai_returnhome_task`) **invencível por 22 batimentos** (`SetInvincibleFilter(true, 22)`, `aipolicy.cpp:1291-1304`; no `gs` 1.2.6 o mesmo `push 0x16`, VA 0x80db6e5) — não leva dano, não nota ninguém (o filtro tira o `MSG_MASK_PLAYER_MOVE`) e não aceita ódio (`OnAggro` limpa); o fim da volta tira o invencível (`EndTask`). **Efeito na tela:** no 1.5.5 o `invincible_filter` liga o estado visível **49** (`invincible_filter.cpp:13-22`, vai no `UPDATE_EXT_STATE`); no `gs` 1.2.6 o mesmo filtro **não liga estado nenhum** (VA 0x812f6ee), e o `UPDATE_EXT_STATE` de 8 B do 1.2.6 só leva os estados 0..31 |
| vida do monstro (B128) | `testado`. `gnpc_imp::OnHeartbeat` (`npc.cpp:1946-1958`): em combate regenera `hp_regenerate` por batimento; **fora de combate enche a vida inteira** (quando `hp_regenerate ≠ 0`). Antes do B128 o monstro nunca recuperava vida |
| voltar | sem alvo, corre ao nascimento, passo de 1 s (`ai_returnhome_task` → `session_npc_patrol`, `follow_target` com alcance 0,8 m); acaba a **1,2 passo** de casa; se ao fim estiver a mais de **10 m** (`GetReturnHomeRange`), `ReturnHome`: parada em casa com `MOVE_MODE_RETURN` (7) e velocidade 0x500 (B99) |
| desvio de obstáculo (monstro de chão) | `navegacao.rs`, porte de `cgame/gs/pathfinding`: perseguir e voltar = `CNPCDisperseChaseOnGroundAgent` sobre o `CNPCChaseOnGroundNoBlockAgent` (`CHASE_WITHOUT_BLOCK`): reta se o `.rmap` deixa; senão reta até o último pixel livre + busca gulosa `CPf2DBfs` (Manhattan, 8 vizinhos) em fatias de 20/40/60 pixels (50/90/120 bloqueado) pela distância inicial, teto 300/600/900; meta **dispersa** ±60° a `alcance` do alvo (`CChaseInfo` guarda a direção). Condutor = `session_npc_follow_target::Run`: recomeça ao chegar (alcance × 0,6) ou se o alvo se afastar > 7 m (> 4 m sem bloqueio); 3 chegadas ou agente desistindo encerram a sessão. Passear = `CNPCRambleOnGroundAgent`: meta no disco de 10 m, alcançável e de preferência em reta, e o `CNPCChaseOnGroundAgent` (lista aberta de 30 nós, 200 pixels, previsão diagonal). No mapa 161: reta 24% dos passos dentro de obstáculo, agente 0% (B99). Água/ar: ainda reta |
| passear | só com jogador a menos de `RAIO_DE_ATIVIDADE` 120 m (renovado por 20 batimentos de 1 s), com `patroll_mode`, sem ódio: anda (`walk_speed`, passo de 1 s) até ponto a **10 m** do nascimento, no máximo 8 passos; 10% de emendar outro |
| patrulhar a rota (B136) | `testado`. Gerador com `iPathID` que existe no `path.sev` (spec 03 §3.6f, ≥ 2 pontos): sem tarefa e fora de combate, **em vez** do passeio, o próximo ponto vira um `ai_patrol_task` (`aipolicy.cpp:320-335`; laço pelo `iLoopType`: 0 para no fim, 1 vai e volta, 2 recomeça — `patrol_agent.h:52-99`). A `session_npc_patrol` (`npcsession.cpp:883-994`) dá **um passo por segundo** (`NPC_PATROL_TIME`), `walk_speed` (ou `run_speed` com o `iSpeedFlag`), com o agente de chão (meta 0,8 m); perto do ponto (`1,44 × passo²`, meta seguinte 1,8 m) ou com o agente na meta pega o seguinte, e dura até 120 passos. Sem sair do lugar, vai de uma vez ao ponto (`ReturnHome(_target, 0)`). Também para sem ninguém a 120 m (`_idle_mode`). No `RollBack` volta ao **ponto atual da rota**, não ao gerador (`aipolicy.cpp:214-220`). Diferença conhecida: no original o passo da patrulha do monstro de ar/água sai sem a máscara de ambiente (`mode \| _is_run ? RUN : WALK`, precedência), aqui sai com ela; o de ar/água patrulha em reta |
| grupo e chefe (B136) | `testado`. Área com `iGroupType` 1 (`group_spawner`) ou 2 (`boss_spawner`): o gerador 0 é o **líder** (uma cópia); os outros nascem no círculo de 7 m em volta dele (`sctab`, `npcgenerator.cpp:5232-5251`) e têm `lider`. O subordinado sem tarefa, com o líder vivo, faz o `ai_follow_master` (`aipolicy.cpp:1519-1583`, distância **horizontal**): ≥ 20 m (`MAX_MASTER_MINOR_RANGE` 400 ao quadrado) vai de uma vez para até 7 m dele (`ReturnHome`, parada `MOVE_MODE_RETURN`); ≥ 8 m corre atrás dele (500 ms, até ficar a menos de 7 m); perto, passeia em volta dele (raio 7, 6 passos). Com o líder vivo não volta para casa (`GetReturnHomeRange` 1e20); com o líder morto, age como monstro solto. O chefe (2) repassa o primeiro da lista de ódio, quando muda, aos subordinados (`TryForwardAggro` → `GM_MSG_TRANSFER_AGGRO`; `aggro_minor_policy::AggroTransfer` troca a lista inteira). O subordinado morto **só renasce com o líder**, em volta dele, se o corpo já sumiu (`group_spawner::OnHeartbeat`/`Reclaim`). Ex.: Carniçal Sanguinário (3885, rota 486539715 em laço) com 2 Fantasmas Malignos (1579) no mapa 1 do 1.2.6 |
| fase do batimento | **sorteada por monstro** (`MonsterAi::new`): o batimento de 1 s e o passo de patrulha começam em pontos diferentes do segundo, porque o original não bate em todos ao mesmo tempo — o coletor pega `tamanho / TICK_PER_SEC` objetos por tique (`objmanager.h:213-229`, `worldmanager.h:262`) e cada NPC nasce com `idle_timer_count = Rand(0, NPC_IDLE_HEARTBEAT)` (`npcgenerator.cpp:2014`). Sem isso, dez monstros davam o passo no mesmo quadro e o cliente tocava dez sons de passo sobrepostos (B58) |
| aviso ao cliente | `OBJECT_MOVE` (ms, ×256, modo) e `OBJECT_STOP_MOVE` com direção ao parar |
| invocado (missão ou matéria) | id em `0x9000_0000 \| n` (até o B118 era `0xA000_0000`, com o bit `PET_MASK` dos mascotes: a Fera Psíquica nasceu marcada como mascote e o invocado seguinte teria o id do mascote do jogador). O da mina (`matter.cpp:450-490`) usa o `life_time` do `npcgen_N` como `remain_time`: 0 = fica até morrer (a Fera Psíquica 11603 da mina 11542 tem 0 nos dois realms). **Não renasce** — não tem gerador (`SummonMonster` → `CreateMinors`, `gs/player.cpp:13072-13110`); nasce **odiando quem o chamou** (`GM_MSG_GEN_AGGRO` com 10000, `:13093-13106`) e some quando o `remain_time` acaba (`prop.remain_time`, `:13079`). O `respawn_delay_ms` zero passava por um `.max(1)` e o monstro voltava 1 ms depois do corpo sumir (B77) |
| alvo sozinho | `aggressive_mode` do `MONSTER_ESSENCE` (**4.874 dos 8.054** monstros do `realm_155`): o monstro recebe a marca `MSG_MASK_PLAYER_MOVE` (`npcgenerator.cpp:2534-2537`) e o jogador, ao andar, avisa quem está a até **15 m** (`GetMaxMobSightRange`, `playerctrl.cpp:265-276`, `worldmanager.cpp:48`); o monstro só aceita o aviso a menos de **`sight_range + body_size`** (`gnpc_ai::AggroWatch`, `ainpc.h:851-854`, `ainpc.cpp:251-252`) — 6–8 m nos monstros do começo. Nosso agressivo sem alvo pega o jogador vivo mais perto dentro de `min(15, sight_range + size)` (`MonsterAi::raio_de_deteccao`; B76 usava os 15 m para todos, corrigido no B128). Voltando para casa, não nota ninguém. `falta`: as estratégias de ódio do `aipolicy.data` (facção, nível, invisibilidade, probabilidades) |
| ataque | intervalo = `attack_speed` do `MONSTER_ESSENCE` em tiques (`ChangeInterval`, `npcsession.cpp:60-70`; era 1,5 s fixo até o B62) e alcance do mesmo lugar. O `HOST_ATTACKED` leva o **id do monstro** que bateu (B59) — com zero ali o cliente não acha o atacante e o jogador perde vida sem ver golpe —, `cEquipment = 0x7f` ("nenhuma peça desgastada"; com zero o cliente gastava a arma a cada golpe) e `speed` = `damage_delay` do `MONSTER_ESSENCE` em tiques, que é a duração da animação no cliente (`npc.cpp:2118`, `EC_NPC.cpp:2043-2064`) (B60) |
| alcance do corpo a corpo e corpo do alvo (B139) | `testado`. `ai_melee_task::Execute` (`aipolicy.cpp:597-609`): persegue até `(attack_range − corpo) × 0,6 + corpo + corpo do alvo` (meta do agente de chão, no plano), começa a bater a `× 0,8` e a `session_npc_attack` continua até `attack_range + corpo do alvo` (`CheckAttack`), em 3D. O corpo do alvo é o `info.body_size`: `PLAYER_BODYSIZE` 0,3 do jogador, o `size` do mascote (Vespão Pequeno: 1) — em todas as estratégias. Antes: 0,9 × `attack_range` sem o corpo do alvo, e o monstro não alcançava o mascote de ar que bate de longe e de cima. Quem voa mais alto que esse alcance continua fora do alcance, como no original, e o monstro desiste do jogador (B134) |
| estratégia (B126) | `testado`. `id_strategy` do `MONSTER_ESSENCE` → `ai::Estrategia` (`AddPrimaryTask`, `aipolicy.h:1214-1277`). **0** corpo a corpo (o de sempre); **1** distância (`ai_range_task`, `aipolicy.cpp:744-812`): bate de `attack_range + corpo do alvo`, persegue até 80 % disso, e com o alvo colado (`corpo + corpo do alvo + 0,3`) ou abaixo de 60 % do alcance se afasta (até 2 vezes, `ST_KO_COUNT`; a tarefa começa no estado 1, então só depois do primeiro golpe); **2** magia (`ai_magic_task`, `:853-968`): persegue até 90 % do alcance mágico (`GetMagicRange + corpo + corpo do alvo`), abaixo de 50 % se afasta (2 vezes), senão conjura; **3** corpo a corpo e magia (`ai_magic_melee_task`, `:970-1107`): o alvo além do alcance de golpe (`(attack_range − corpo) × 0,8 + corpos`) **recebe magia de onde o monstro está**; perto, uma série de `(195 + Rand(10,20)) / (attack_speed + 1)` golpes e depois magia; colado, afasta-se; **4** fixo: não se move, bate no alcance; **5** fugitivo: foge; **6** inerte; **7** fixo mágico: não se move, conjura no alcance. Contagem: 1.5.5 = 3.289/246/299/2.179/717/103/148/1.048 monstros nas estratégias 0–7; 1.2.6 = 879/165/67/1.216/112/19/15/60 (`cargo run -p pw-data-loader --example ia_de_monstro`). A 2/3/7 sem habilidade não tem tarefa (`StartTask` falha), como no original — o `npcgenerator.cpp:288` avisa quem monta monstro assim |
| habilidade escolhida (B126) | `GetPrimarySkill` (`aipolicy.h:1013-1057`): as habilidades do `MONSTER_ESSENCE` separadas pelo tipo do catálogo (1 ataque, 2 bênção, 3 maldição, até 8 de cada); a 1ª vez uma bênção, a 2ª uma maldição, depois 80 % ataque, 10 % bênção, 10 % maldição. Todas as 550 (1.5.5) e 164 (1.2.6) habilidades de monstro estão nos catálogos |
| conjurar (B126) | `session_npc_skill` (`npcsession.cpp:654-776`): **sem recarga nem mana** (só o mascote liga `_use_cooldown`/`_use_mp`); `OBJECT_CAST_SKILL` (85) a quem vê com o canto do estado 0 (`SkillWrapper::NpcStart`); o efeito no fim do canto e a execução (estado 1; zero vira 1 s) ocupando o monstro; canto zero = efeito na hora. Efeito (`BusServer::aplicar_habilidade_do_monstro`, o motor de roteiros com o monstro como conjurador): ataque → golpe com `Rand(dano) × (100 + ratio%)/100 + plus` (físico de `damage_min/max`, mágico de `magic_damage_min/max`, `actobject.h:1422-1466`) e precisão × `GetHitrate`, em cada alvo da área (ponto, linha, bola em si/no alvo, setor); o jogador atingido recebe `HOST_SKILL_ATTACKED` (144) e quem o vê `OBJECT_SKILL_ATTACK_RESULT` (143) (`player.cpp:3345-3354`); acertou, o `StateAttack` (sangramento, lentidão…) — o dano no tempo de monstro em jogador **não** leva o ¼ do PvP; maldição → roteiro no alvo; bênção → no próprio monstro; as duas com `ENCHANT_RESULT` |
| eventos de vida (B126) | `testado`. `skill_hp75/50/25` sorteados **uma vez** quando o monstro nasce, pelo `RandSelect` das 5 entradas (`npcgenerator.cpp:2579-2587`, `arandomgen.h:140-153`: cumulativo; soma que não fecha cai no índice 0). No batimento de 1 s em combate (`aipolicy.cpp:344-360`, `TriggerEvent` `aipolicy.h:1343-1370`): vida abaixo do `_cur_event_hp` (começa em ¾) dispara o evento do limiar (índice `cur / ¼ − 1`), e o `cur` desce até ficar abaixo da vida — queda grande dispara **um** evento. `FLEE_SKILL_ID` 40 = fugir; outra = conjurar de onde está. Sair de combate põe o `cur` na vida atual (`RollBack`). **Só vale para monstro sem política** no `aipolicy.data`. 1.5.5: 361/1.754/514 monstros com evento em 75/50/25 %; 1.2.6: 328/1.580/433 |
| afastar e fugir (B126) | `session_npc_keep_out`/`session_npc_flee`: passo de `run_speed × 0,5` a cada 0,5 s para longe do alvo, até 90 % do alcance pedido (fuga: `FLEE_RANGE` 30 + corpo, 8 passos). **Não é igual:** o agente `path_finding::keep_out` não está portado — é a reta oposta ao alvo, no chão do mapa, e o `keep_out` fica limitado aos 4/5 passos que a tarefa passa (o original não conta o `_timeout`, acaba no agente) |
| habilidades de monstro — o que falta | `falta`: interromper o canto ao apanhar (`NpcInterrupt`, `skill_interrupt_filter`); o monstro selado/atordoado com as estratégias (`ai_silent_*`); os níveis acima do `max_level` do catálogo (76 pares no 1.5.5, 75 no 1.2.6) e os de nível ≤ 0 usam, nas tabelas por nível, o nível mais próximo (o roteiro roda com o nível real); `_max_move_range` da política; o `session_npc_keep_out` quando o alvo está dentro do corpo no corpo a corpo puro (0) não foi portado para não mexer no que já foi visto em jogo |

### 4.1 A política do `aipolicy.data` (`politica.rs`) — `testado` (B127, sem teste em jogo)

Porte de `aitrigger.h`/`aitrigger.cpp` e do montador `ai/policy_loader.cpp`. Cada monstro com
`common_strategy` recebe a política compilada (uma por id, em cache no mundo), com as três
variáveis locais iniciais do `MONSTER_ESSENCE` (`local_var[3]`, os `param1..3` do layout v156) e
as variáveis globais do mundo (`world::_common_data`, compartilhadas).

| quando | o que roda (`ai_trigger::policy`) |
| :--- | :--- |
| batimento de 1 s em combate | timers (`RefreshTimer`, sem parar) + gatilhos de batimento (param no primeiro `false`) + os de paz |
| batimento fora de combate, com jogador por perto | timers + batimento sem `bAttackValid` (`_idle_mode` sem ninguém: nada) |
| primeiro ódio (`EnableCombat(true)`) | começo de combate, **sem** olhar se está ligado |
| lista de ódio vazia (`RollBack`) | `Reset` (os de combate voltam ao início, timers somem) + fim de combate |
| morte (primeira vez que o laço do mundo o vê morto) | morte + `ResetAll`; o crédito é de quem mais bateu |
| golpe/habilidade que o acerta (`OnDamage`) | os de dano, com o dano no `GetLastDamage` |
| mata o alvo (`KillTarget`) | os de matar |

`bRun` = só roda chamado (`o_run_trigger`, cópia própria); `bActive` = ligado de início; começo,
fim de combate e morte nascem ligados; `hp_less` e fim de caminho se desligam ao disparar;
`and`/`or` tomam o tipo da esquerda, `not` do filho; divisão por zero cancela o gatilho (a
exceção engolida pelo `TestTrigger`). Condições: timer, vida abaixo, aleatório, e/ou/não,
matar, começo/fim de combate, morte, dano na faixa, `<`/`>`/`=` com constante, global, local,
aritmética e contagem de jogadores num raio/caixa. Alvos: primeiro/segundo/outros/aleatório/
mais perto/mais longe da lista de ódio, menor/maior vida, maior mana, profissões, si mesmo,
quem matou, primeiro redirecionado (sem o dono do mascote: o alvo fica o mascote).

Operações e o efeito: **habilidade** (`o_use_skill(_2)`) → `ai_skill_task_2`: área 2/5 conjura
já; estratégia fixa conjura de onde está; senão persegue até 90 % do alcance e conjura (o
original persegue no máximo duas vezes — aqui até chegar); **atacar** (`uType`) troca a
estratégia da tarefa até o fim do combate; **fugir**; **falar** (`$A`/`$B`/`$S`/`$I`/`$X` escolhem o
canal; `$F`/`$T` de batalha sem porte) — `ChatSingleCast` (94) pelo barramento e `ChatMessage`
(80) ao cliente, que põe a fala no chat e no balão do NPC (`EC_GameSession.cpp:4953-4965`);
**controlador** do `npcgen` (`TriggerSpawn`/`ClearSpawn` pelo `iControllerID`: liga as áreas
pendentes dele, desliga o que ele pôs — sem a espera/parada do controlador); timers, ligar/
desligar gatilho, ódio (1 em todos, primeiro = maior + 1, último = 1, metade com mínimo 1),
pular, globais e `CalcularVariavel`. As habilidades citadas com id/nível fixos são resolvidas no
catálogo quando o monstro nasce.

Cobertura (`tests/politica_do_realm.rs`): **1.2.6 — 258 políticas usadas, 0 operação e 0 condição
sem porte**; 1.5.5 — 2.771 políticas, sem porte (registradas uma vez no `debug`):
`InvocarMonstro` 703, `AndarPorCaminho` 161, `TocarAcao` 114, `ContarJogadores` 50,
`EntregarMissao` 32, `InvocarNpc` 31, `Historico` 22, `InvocarMina` 12, `PontosPvpDeFaccao` 3,
`LimparMissaoDeTorre` 1, e 93 condições (estágio de história, filtro, fim de caminho).
Também fica de fora: `CHAT_AIPOLICY_VALUE` = 2 é o do 1.5.5 (as falas com anexos só existem em
gatilho de versão ≥ 17); o `srclevel` do `ChatMessage` no cliente 1.2.6 não foi medido (vai 0 no
fim, como no IR 1.5.3/1.5.5).

O original tem **dois** limites de perseguição (corrigido em 2026-10-01; esta nota dizia que só
havia o segundo):

1. Distância até o alvo ≥ `aggro_range`: o alvo sai da lista (`ai_melee_task::Execute`,
   `aipolicy.cpp:577-585`; `GetIgnoreRange` = `GetAggroRange`, `ainpc.h:545`). Este está portado,
   mas com um **piso de 15 m sem fonte** (`MonsterAi::PERSEGUICAO_MINIMA`): o original usa o
   `aggro_range` puro (`npcgenerator.cpp:2566`, sem piso nem teto).
2. Distância **do ninho** > `max_move_range` (do `MONSTER_ESSENCE`; zero = sem limite, senão
   piso de 20 m, `npcgenerator.cpp:313-316`): `ClearAggro` (`aipolicy.cpp:298-307`, só sem rota).
   `falta`.

Não há regra de ódio própria de masmorra: o código de `gs/instance/` não toca na IA. O monstro da
Caverna Sombria (169) é persistente pelos dados: `aggro_range` 107,25, `aggro_time` 100 e
`max_move_range` 0 nos seis tipos. A volta para casa é o `GetReturnHomeRange`
(`ainpc.cpp:29-37`).

## 5. Combate (`combat.rs`) — `confirmado`

`gactive_imp::AttackJudgement` + `HandleAttackMsg` (`actobject.cpp`), com as rolagens
recebidas prontas (`Rolagens`) para teste determinístico:

1. **Acerto** (só físico): `taxa / (taxa + (armadura >> 1))`, piso 0,05, sem teto.
2. **Curta distância**: habilidade × fator próprio; golpe normal divide o físico por 2.
3. **Atenuação por distância** (atacante jogador/pet).
4. **Defesa por classe** (físico + 5 mágicas): `reduce = def / (def + 40×nível_do_atacante − 25)`, teto 0,95; imunidade zera a classe. Penetração: `def × (1 − anti/(anti+10000))`, razão até 0,35.
4a. **Redução de dano** (B137, `DoDamageReduce`/`DoMagicDamageReduce`, `actobject.h:1573-1593`):
   depois da defesa, `× (100 − r)%` no físico (teto 75) e em cada classe mágica (teto 90).
   `r` dos adicionais `enhance_damage_reduce_addon*` e `enhance_*magic_damage_reduce*`
   (`item_addon.cpp:696-722`, `:1456-1462`) do equipamento de quem apanha.
4b. **Esquiva de dano** (B137): golpe que acertou, `Rand(0,99) < _damage_dodge_rate` → o dano
   vira 0 e o piso deixa 1 (`HandleAttackMsg`, `actobject.cpp:707-711`). A taxa vem do efeito
   `Incdamagedodge` (1.5.5; no 1.2.6 nenhuma habilidade o usa). O aviso
   `AT_STATE_DODGE_DAMAGE` (0x400) não vai ao cliente.
5. **Crítico**: `× (2,0 + bônus% − redução%)`; crítico se `Rand(0,99) < crit_rate − crit_resistance`.
5a. **Diferença de nível** (B137, `gnpc_imp::AdjustDamage`, `npc.cpp:1731-1733`; no `gs`
   1.2.6 o mesmo, VA 0x809f7ea): golpe de **jogador** em NPC `× attack_adjust` da tabela
   `PARAM_ADJUST_CONFIG` (`GetAttackLevelPunishment(nível do jogador − nível do NPC)`). Nos
   dois realms: 5 níveis abaixo 0,9; 10 abaixo 0,7; 20 abaixo 0,5; 30 abaixo 0,25; no mesmo
   nível ou acima, 1. Vale no golpe normal e no dano de habilidade (a tabela antiga e o stub);
   o dano no tempo já tinha (`Set*` do `playerwrapper`). Mascote não (`IS_HUMANSIDE`).
5b. **Camada** (B128, antes do crítico, no `damage_adjust`): golpe de **jogador** em
   monstro vale **metade** do ar no chão/água, do chão na água e da água no chão/ar
   (`gnpc_imp::AdjustDamage`, `npc.cpp:1727-1768`; `combat::ajuste_de_camada_no_npc`). A camada
   do jogador é o ar quando voa (nadando ainda não é acompanhado); a do monstro, o habitat.
   O golpe de monstro no jogador **não** tem ajuste por camada (`gplayer_imp::AdjustDamage`,
   `player.cpp:9610-9652`: só a tabela de PvP, ainda sem porte). **Mascote também não** (B135):
   o ajuste é para `IS_HUMANSIDE(source)` = `type == GM_TYPE_PLAYER` (`common/types.h:321`), e o
   mascote é `GM_TYPE_NPC` — `Golpe::camada` é `None` para ele (no B128 o mascote de ar batia com
   metade no monstro de chão).
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
`ADDON_LIST_DROP`, `pw_gs::geracao`; fabricação e loja têm variante própria — linha
"equipamento comprado e fabricado", B151): essência (dano máximo, defesa, evasão, vida/mana,
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

No cliente 1.2.6, o Murillo relatou em 2026-09-23 que, depois da canalização,
faltam a animação e o efeito de lançamento da skill 299 (Enxame de Ferroadas).
**Testado no barramento (B98):** para o conjurador, 85 → 88 → 142 → 123, com
142 de 14 bytes de payload (captura `full_interno.medidas.md:101`); o 142 chama
`PlayAttackEffect` (`EC_HostMsg.cpp:947-955`) e este chama a ação de lançamento
(`EC_Player.cpp:3414-3525`). O 88 só vai ao dono, como no original
(`gs/player.cpp:4066-4073`); enviá-lo aos demais alterava o estado da skill deles.
**Causa (B100, `diagnosticado` e corrigido no barramento; falta ver em jogo):** no realm
1.2.6 a tabela de habilidades ficava **vazia** — o `manager.rs` só a carregava para
`elements.data` v156/v159 —, então `fase_de_execucao_ms` era 0 e o 123 saía colado ao 142
(`bus_server.rs:2358-2372`), e a conjuração caía na tabela embutida ou nos 1.000 ms fixos
(`:2096-2106`). Mesmo sintoma do 1.5.5 no B67. Agora o v7 carrega `TabelaDeHabilidades::do_126`
(tempos do `gs` 1.2.6, spec 03 §3.10b). Captura original da 299: `85` (tempo 1.500) → `88`
em +1.505 ms → `142` em +1.555 → `123` em **+2.504..2.551 ms**; o teste de mundo mede o 88 entre
1.400 e 1.800 ms e o 123 entre 2.400 e 2.900 ms, pelo menos 900 ms depois do 142.

**Dano no 1.2.6 (B101, `corrigido`, falta ver em jogo):** a Tsuko matava o Filhote de
Mandrágora (29 de vida) com 75 e 106 da 299 no nível 1. Não era um dano padrão: era a fórmula
certa (`golpe_de_habilidade`, `combat.rs:567`) com os números do stub **1.5.5** (plus 124,5 no
nível 1). O `gs` 1.2.6 dá plus 23,7 (spec 03 §3.10b); com a ficha corrigida (mágico 6-6) a 299
nível 1 soma 32 antes da resistência. A captura original mostra 20, 23, 23 e 24 no `142`.

### 5.0.9 Movimento de monstro no 1.2.6 conferido com a captura — `testado` (B103)

Relato: "monstros teleportando" com a Tsuko e o WRA. Tudo o que o servidor controla bate com
o original (`full_interno.pcap`, 16.822 `OBJECT_MOVE` de monstro): passeio com `use_time`
1.000 ms, um comando a cada ~1 s e passo = velocidade × 1 s (3303: 2,25 m, 576/256 m/s; 1000:
0,86 m, 220/256), pausa mediana de ~32 s; perseguição com 500 ms, modo 1, 4 m/s, 2 m por passo
(o `gs` 1.2.6 também: `session_npc_follow_target` arma 10 tiques e usa 0x1f4 = 500);
velocidades do `MONSTER_ESSENCE` v7 iguais às da captura; altura do terreno + estrutura igual à
do original em ±6 mm (p1–p90 de 18.918 pontos). No mapa real (`tests/passeio_do_126.rs`): 433
passos de passeio sem nenhum maior que velocidade × `use_time`; perseguição sem salto; volta
para casa andando em 19 de 20 (1 pelo `ReturnHome`, que o original também faz). Nenhum
`OBJECT_MOVE` descartado pela fila do link. **Diferença conhecida**: o original deixa o cliente
conhecer até 220 criaturas de uma vez (captura; só 41 `OBJECT_LEAVE_SLICE` na sessão); o nosso
cortava nas 80 mais próximas (`TETO_DE_VISIVEIS`) até o B142; desde o B143 vale o quadrado de fatias do original, sem teto (§1).
**Causa achada e corrigida (B104, falta ver em jogo):** a emenda de passeio
(`ai_rest_task::OnSessionEnd`, 10 %) zerava a espera em `comecar_passeio`. O último passo do
passeio saía com `use_time` de 1 s e o primeiro do passeio seguinte no tique seguinte: dois
`OBJECT_MOVE` a ~50 ms, e o cliente corria ou pulava para o segundo destino. Reproduzido com o
mapa 1 inteiro, o laço de tiques real e o barramento
(`reproducao_do_passeio_no_realm_126`, `#[ignore]`): 4 a 8 pares em ~250 movimentos em 30 s,
0 depois. A medição rápida (`o_filhote_de_mandragora_passeia_sem_saltos`) acusa 16 com o
defeito. Água e estrutura conferidas contra a captura: 5 pontos sobre estrutura, 0 sobre água,
nenhum com a nossa altura acima da do original.
**Segunda causa, corrigida (B106, falta ver em jogo):** o último passo do passeio ia como
`OBJECT_MOVE` e a parada era descartada (`acao.or(parada)`). O cliente segue andando o monstro na
mesma direção enquanto não chega comando novo (`CECNPC::MovingTo`, `EC_NPC.cpp:1225-1240`) e só
o puxa de volta a mais de 25 m do destino (`MAX_LAGDIST`, `EC_NPC.cpp:79`): o monstro
"disparava", subia no relevo e voltava de uma vez. O original manda esse último passo **só** como
`stop_move` até o ponto final (`session_npc_cruise::Run`, `npcsession.cpp:626-633`); o nosso
agora também, no chão e na água/ar. `o_filhote_de_mandragora_passeia_sem_saltos` cobra que todo
`OBJECT_MOVE` tenha passo ou parada seguinte até o fim do `use_time` (+100 ms). **Regra geral:**
movimento de monstro sem continuação dentro do `use_time` faz o cliente extrapolar.

### 5.1.0 Nada que espere o banco fica no caminho do jogo — `testado` (B72)

O tique de 50 ms roda inteiro com o `world.write()` na mão, e as respostas a comandos
esperam esse mesmo lock. Toda escrita que ficar dentro dele para **o mundo todo** pelo tempo
do banco.

| onde estava | o que acontecia | onde está agora |
| :--- | :--- | :--- |
| autosave (fotografia atômica a cada 60 s; B170) | dentro do `tick`, com o mundo trancado | `EstadoParaGravar` com carimbo local; uma transação fora do lock, fotografia antiga descartada |
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

**126, medição B89:** para 22 ticks anunciados, 23 intervalos do resultado no
PCAP tiveram mediana 1149,332 ms (1049,265–1199,262). Entre ATTACK_ONCE e resultado,
mediana 49,3695 ms em 52 pares. São tempos de recepção no elo interno, não de
animação. Regra comum preservada; três testes focados do mundo aprovados.
Bytes/cadência e limitações: `docs/COMBATE_126.md`.

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
- **Quando o dano é aplicado**: `DoAttack` começa na hora, mas `InsertDamageEntry`
  agenda a aplicação por `attack.speed` ticks (`actobject.cpp:1758-1776`), como
  descrito em §5.1.1. O anúncio do resultado não significa HP já debitado.
- **Barra de vida do alvo (`NPC_INFO_00`) não vai junto do golpe** (B56). O original só a
  manda ao selecionar (`InsertInfoSubscibe` → `query_info00`, `actobject.cpp:1610`) e no
  heartbeat de 1 s (`obj_manager<gnpc, TICK_PER_SEC>`, `worldmanager.h:262`), a quem tem o
  monstro selecionado e só se vida ou alvo mudaram (`RefreshSubscibeList` + `_refresh_state`,
  `actobject.cpp:1296-1353`; `gnpc_imp::SendDataToSubscibeList`, `npc.cpp:2219-2230`). Aqui:
  `WorldInstance::informar_vida_aos_inscritos` no batimento → `EventoDoMundo::VidaDoMonstro`.
  Vale para golpe normal, habilidade (alvo único e área) e dano no tempo. **O número do dano
  no tempo (B126):** cada tique (`filter_Wounded::Heartbeat` → `BeHurt` → `OnHurt`) manda
  `HURT_RESULT` (122, `{target_id, damage}`, 8 B) ao atacante jogador e `BE_HURT` (121,
  `{attacker_id, damage, flag}`, 9 B) à vítima jogador (`gnpc_dispatcher::be_hurt`,
  `npc.cpp:188-199`; `gplayer_dispatcher::be_hurt`, `player.cpp:3380-3396`); o cliente põe o
  número sobre o alvo (`EC_HostMsg.cpp:4184-4201`). Antes o sangramento tirava vida (tiques de 9
  a cada 3 s no Soco de Uma Polegada nível 1, conferido no log do WRA) mas nada aparecia. O
  dano do mascote vai ao dono (crédito). **Sem ódio pelo tique** (B137): `BeHurt` →
  `OnHurt` só registra o dano (`npc.cpp:1829-1845`), e o `Directhurt` também. O ódio de
  habilidade é o `GetEnmity` (spec 03 §3.10b): `SkillWrapper::Attack`/`Enchant` →
  `SetEnmity` (`skillwrapper.cpp:475-477`) põe esse ódio em cada monstro atingido (ataque que
  acertou, maldição aplicada), a quem lançou — jogador ou mascote. O `AddAggroToEnemy` das
  bênçãos (ódio de quem cura) ainda não tem porte. Até o B55 a barra
  saía no mesmo instante do golpe e caía no clique, antes de a flecha sair (teste de
  2026-09-17). `testado`, falta ver em jogo.

- **Habilidade em recarga durante o golpe normal** (B126, diagnóstico): o cliente 1.5.5 **não
  manda** o `CAST_SKILL` quando a habilidade não está pronta (`ReadyToCast()` falso →
  `return false`, sem mexer no golpe; `EC_HostPlayer.cpp:2833-2836`). Se o pedido chega, o
  original também encerra o golpe: a `session_skill` entra na fila (`AddSession`,
  `actobject.cpp:1180-1213`), o golpe cede no próximo `GM_MSG_OBJ_SESSION_REPEAT`
  (`:180-189`) e a recarga recusa no `StartSkill` (`actsession.cpp:477-482`) — o golpe não
  volta. No log do WRA (1.2.6) **nenhum** pedido foi recusado por recarga: o golpe parou por
  outro comando. Desde o B126 o `debug` registra `CANCEL_ACTION`, a sessão de golpe cedendo à
  fila (com o motivo) e o `CheckAttack` recusando — o próximo teste diz qual foi.
- **`NORMAL_ATTACK` que chega com conjuração aberta entra na fila** (B57): no original a
  habilidade é a sessão corrente e o `AddSession` do golpe devolve `false`
  (`actobject.cpp:1180-1212`); ele começa quando a habilidade termina
  (`SafeDeleteCurSession` → `StartSession`), e aí o `CheckAttack` recusa alvo morto. Aqui:
  `BusServer::golpe_na_fila`, solto por `fechar_conjuracao_e_soltar_fila`. O cliente manda
  `NORMAL_ATTACK` assim que vê o fim da conjuração; sem a fila, o dano do golpe e o da
  habilidade caíam no mesmo instante e o monstro morria "instantaneamente" com a habilidade
  (teste de 2026-09-17). Sai da fila ao morrer e ao sair do jogo.

**Roubo de vida (B137):** `hp_steal_rate` do efeito `Inchpsteal` (1.5.5; MERGE: igual renova,
maior substitui; `+ 0,00001` na porcentagem; classes da máscara 0xACE não recebem,
`playerwrapper.cpp:3549-3559`) no golpe físico de quem não tem arma de longe (`FillAttackMsg`,
`actobject.cpp:1496-1497`): `int(rate × 0.01f × dano)` em float (10 % de 1000 = 99) volta na
hora a quem bateu, com o `PLAYER_HP_STEAL` 279 (`actobject.cpp:297-302`).

Sinais ao cliente (B142, `testado`): o crítico e a esquiva de dano vão no `attack_flag` (spec 04
§5); quem vê um jogador morrer recebe `PLAYER_DIED` (27).

**Morrer voando ou montado (B158, `testado`; falta ver em jogo):** `gactive_imp::Die`
(`actobject.cpp:590-606`) tira os filtros `FILTER_MASK_REMOVE_ON_DEATH` **antes** do `OnDeath`. O de
voo (`fly_filter.h:10-28`) pousa — `Landing` → `STATE_FLY` desligado e `_runner->landing()`
(`OBJECT_LANDING` 97 a ele e a quem vê, `actobject.h:891-896`); o de montaria (`mount_filter.h:14`)
desmonta — `DeactiveMountState` → `RecallPet`, `STATE_MOUNT` desligado e `player_mounting(0, 0)`
(`mount_filter.cpp:36-45`, `player.cpp:14301-14320`). No servidor: no `JogadorMorreu`, antes do
`HOST_DIED`, `voando`/`voo_gasta_mana` zerados com o pouso, e a montaria sai pelo mesmo `desmontar`
do recolher (227 com 0, velocidade refeita, `RECALL_PET` 234). Antes, o personagem renascia
voando/montado para o servidor e o botão de voo/montaria pedia dois cliques.

**Trava de PvP (B146, `testado`):** o mundo é PvE (`PVP_MODE(0)` na entrada, o `pve_mode` que o
servidor de entrega liga, `serverstat.cpp:113`) e cada jogador tem a chave: `ENABLE_PVP_STATE` (82)
liga acima do nível 29 (`PVP_PROTECT_LEVEL`) e arma 36000 s de espera (`HOST_PVP_COOLDOWN` 185);
`DISABLE_PVP_STATE` (83) com espera pendente dá o erro 47 (`PlayerEnablePVPState`/`Disable…`,
`player.cpp:12421-12460`); a espera cai 1 por segundo (`:9092-9098`); `PLAYER_ENABLE_PVP`/`DISABLE`
(183/184, 5 B no 1.5.5 e 4 B no 1.2.6) a quem vê. Habilidade que machuca outro jogador só pega com
o golpe forçado (Ctrl, o `force_attack` do `CAST_SKILL`), as duas chaves ligadas e fora do mesmo
grupo (`PetTestHarmfulEffect`, `player.cpp:15097-15114`); senão não tem efeito. A chave não é
gravada (volta desligada no login). `falta`: duelo, agressor (nome laranja/vermelho) e perdas na
morte PvP, zona de segurança (`pvp_limit_filter`), `STATE_PVPMODE` no `state`.

`falta`: filtros de escudo, esquiva de maldição (`Incdebuffdodge` é lido, a regra não), vigor,
sessão de golpe contra jogador (o golpe normal não mira jogador), marca no golpe de mascote.

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
  normal antes de a habilidade ter efeito. O 88 é enviado só ao conjurador
  (`gs/player.cpp:4066-4073`, B98); os demais recebem o 143 do lançamento.
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
  a cada 3 s, `skillfilter.h:4136-4232`), **Forma Sombria** (`Fairyform`, B95: fraco, nem
  bênção nem maldição, **sobrevive à morte**; forma 65 pelo `PLAYER_CHGSHAPE`, equipamento
  trancado, +`100 × ratio`% de velocidade e +`100 × value`% de defesa, ícone 279;
  `skillfilter.h:16819-16875`, roteiro da 2570 dá 19 s / 4% / 60% no nível 1), **raposa**
  (`Foxform`, B120, o Chamado da Raposa 312 da Feiticeira: sem tempo, fraco, sobrevive à morte;
  lançar a 312 **de novo desfaz** (`SetFoxform`, `playerwrapper.cpp:2539-2551`); mana máxima
  −`100 × ratio`%, defesa +`100 × amount`%, precisão +`100 × probability`% (`skillfilter.cpp:389-413`;
  nível 1: −30/+60/+100 no 1.5.5; no 1.2.6 o `probability` é `0,5·L + 1`, +150 no nível 1 —
  `filter_Foxform` VA 0x83080b8 com `ratio`/`amount`/`probability` de +0x70/+0x74/+0x6c do
  `PlayerWrapper`, B122); equipamento trancado; ícone 75 **sem parâmetro**; forma pelo
  `PLAYER_CHGSHAPE` — 65 no 1.5.5, 1 no 1.2.6 (`WorldProtocol::byte_de_forma`; o `_shape` é o
  `value` do roteiro no 1.5.5 e `ChangeShape(1)` fixo no 1.2.6, que não tem `SetValue`). Na forma de
  classe só valem as habilidades cujo `allow_forms` tem o bit `1 << GetForm()`
  (`skill.cpp:128`): 313–318 só na raposa, 312 nas duas, as comuns só fora; a recusa é
  `ERR_SKILL_NOT_AVAILABLE` (20). **Passivas de forma** (`SkillWrapper::EventChange`,
  `skillwrapper.cpp:589-610`, B122): na forma de classe (raposa, Forma Sombria) valem as passivas
  `EVENT_CHANGE` que o jogador conhece, pelo `TakeEffect` (`ao_mudar_de_forma` do catálogo) no nível
  dele — `Incswim` → nado +`100 × inc`% (`EnhanceSwimSpeed`; o `UpdateSpeed` refaz o nado com teto de
  15, `playertemplate.h:1104-1108`), `Incfight` → dano +`100 × inc`% (`EnhanceScaleDamage`),
  `Adddefence` → defesa +`100 × m`%, `Inccrit` → crítico +`point`; fora da forma o `UndoEffect` tira
  o mesmo tanto. A soma fica em `PlayerEntity::passivas_de_forma`, refeita no `refazer_atributos`
  (a raposa nível 1: 323 +50 % de nado, 324 +30 % de dano). A raposa **sobrevive à morte** nas duas
  versões (o `Die` só limpa `REMOVE_ON_DEATH`, `player.cpp:14108`; o `gs` 1.2.6 monta o filtro só com
  `WEAK`, `push 0x8000` em VA 0x83080c1; o `Resurrect` não mexe em forma): ao renascer a forma vai de
  novo ao cliente, B124), **espinhos** (`Retort`/`Retort2`, B120, a Muralha de
  Espinhos 306: o golpe físico corpo a corpo de monstro que acerta o jogador devolve
  `(int)(bruto × ratio)` — dano antes da defesa — como golpe mágico com o crítico/grau do
  jogador, pela defesa física do monstro; não devolve de monstro de longe (`attack_range > 6`,
  `short_range`) nem ≤ 1; ícone 4 / 253, estado visual 3; `skillfilter.h:1450-1505`,
  `:14617-14672`. O golpe devolvido vai ao dono como `SELF_ATTACK_RESULT` e a quem vê como
  `OBJECT_ATTACK_RESULT`, com `AT_STATE_ATTACK_RETORT` 0x20 — o `MOD_RETORT` do cliente
  (`npc.cpp:228-241`, `EC_ManAttacks.h:34`, B124). Desde o B143 também o golpe de **habilidade**
  física de monstro (`habilidade_de_monstro_no_jogador`) e a habilidade física de **jogador**
  (PvP, `efeito_em_jogador`), com o dano bruto do golpe: o `Retort2` (1.5.5) usa o `_ratio_skill`
  (o `Value` do roteiro, 0,02·L na 306) no golpe de habilidade e o `ratio` no normal; o `Retort`
  (1.2.6) usa o `ratio` nos dois (`skillfilter.h:1480`, `:14646`). Espinho em jogador vai a ele
  como `HOST_ATTACKED` com a marca 0x20 (`WorldInstance::devolver_espinhos`). O golpe **normal**
  de jogador em jogador não existe no `pw-gs` (só habilidade), então não passa por eles), **renascer** (`Rebirth`, B115:
  antes de morrer, com a chance do `probability`, volta com `ratio` da vida máxima, `ENCHANT_RESULT`
  da 1085 e se desfaz; jogador e mascote; ícone 155; `skillfilter.h:9027-9070`), **redução de dano
  em área** (`Decregiondmg`, B115: só o ícone 328 pelo tempo — a redução exige `attack_attr < 0`, que
  o fonte 1.5.5 nunca produz; `skillfilter.h:19899-19950`); instantâneos cura, cura/mana em %, dano direto,
  limpar bênçãos/maldições, **chi** (`SetAp`, `playerwrapper.cpp:2351-2357`: passou no dado →
  `ModifyAP((int)value)`; os `BlessMe` do 1.2.6 404/406/420/454/471/484/503/524/585/592/646/654, B122)
  e **volta para a cidade** (`SetReturntown`, `playerwrapper.cpp:1916-1939` → `ReturnToTown`,
  `player.cpp:10949-10957`: o ponto de cidade do distrito, o mesmo do renascer, e o `LongJump`;
  sem distrito com ponto, fica. É o Portal da Cidade 167 do 1.2.6, que o faz no `StateAttack`).
  Convivência de `filter_man::AddFilter` (único substitui, fraco descarta, fundir absorve).
  Realces entram como `_en_percent` na conta do jogador e em monstro (NPC usa o mesmo
  `property_policy` com classe −1). **Atordoado/dormindo (B140):** `IncIdleSealMode` →
  `gnpc_imp::SetIdleMode` (`npc.cpp:2129-2138`) faz `ClearSession` — o canto em curso do monstro
  **acaba** (antes pausava) — e a IA não decide até o filtro sair. Vale igual para o **mascote**
  (antes a IA dele ignorava o filtro: o Vespão atordoado pela 37 do Guerreiro Golem seguia
  batendo); o mascote preso não anda (`IsRootMode` no `follow_target`) mas bate ao alcance, e
  selado não começa habilidade. O estado visível e os ícones do mascote vão a quem vê
  (`UPDATE_EXT_STATE`/`ICON_STATE_NOTIFY`, `avisar_efeitos`), como os do monstro. Monstro atordoado/dormindo não age, preso não anda, lento
  anda mais devagar. A cada 1 s o mundo desconta o tempo, aplica dano/cura e avisa:
  `UPDATE_EXT_STATE` (124), `ICON_STATE_NOTIFY` (125), vida, e para jogador a ficha e a
  velocidade. Morrer limpa tudo. Selado/atordoado não conjura.
- `falta`: os ~300 efeitos sem porte (formas, invocação, escudos, recargas) — vão ao log
  `habilidade N — sem porte: ...`; imunidades do monstro; recarga comum (`commoncooldown`);
  Portal da Cidade (167) **no 1.5.5** (lá o `SetReturntown` está no `State2::Calculate`, que o
  extrator não lê como roteiro); talentos (`GetT0..T2` valem 0, `GetPrayrangeplus`); corpos de
  roteiro com `if` (2 de 2.304 no alvo).

## 7. Jogador

| regra | estado | detalhe |
| :--- | :--- | :--- |
| velocidades, cadência, alcance, `hp_gen`/`mp_gen` | `confirmado` | `CHARRACTER_CLASS_CONFIG` (Bárbaro: correr 4,9 m/s, ataque 0,8 s = 16 ticks, alcance 2,5 m) |
| atributos iniciais | `testado` (B51) | **5/5/5/5 para toda classe**, vida `vit_hp × 5` e mana `eng_mp × 5` — os 12 moldes do `clsconfig` do `pwserver_155v156`. Os atributos e o `hp`/`mp` do `ptemplate.conf` **não** chegam ao jogador (`userlogin.cpp` copia a ficha do banco). Até B50 o Arqueiro nascia com 20 de energia. **1.2.6 igual** (`clsconfig` 1.2.6, B101): até o B101 a Feiticeira nascia com 15/5/15/15 (a seção `[HAG]`) e a ficha ficava com dano 1-1, porque o `ptemplate.conf` 1.2.6 era recusado — com ele, Feiticeira nível 1 com a Varinha Mágica: físico 4-4, mágico 6-6, vida 60, mana 60 (`tests/ficha_do_126.rs`) |
| vida/mana máximas | `testado` (B51) | `lvlup_hp × (nível−1) + vit_hp × vitalidade` (e o par da mana), base zero (`__LevelUp`, `__UpdateBasic`, `playertemplate.cpp:500-582`); `BaseDaClasse::vida_e_mana_maximas` (spec 03 §3.5), a mesma conta da criação |
| **combate** | `testado` | `combate_s`: atacar põe 15 s (`DoAttack`, `player.cpp:3062`), apanhar garante 5 s (`OnAttacked`, `:9514`); batimento de 1 s desconta, e o batimento em que chega a 0 manda o `SELF_INFO_00` mesmo sem vida/mana mudar — é o único jeito de o cliente sair da postura de luta (`EC_HostMsg.cpp:1335`; captura 1.2.6, t = 2409,9 s) (B106) |
| **regeneração** | `testado` | batimento de 1 s no `tick`: `hp_gen`/`mp_gen` em combate, ×4 fora (`player.cpp:9130-9137`), acumulando oitavos (`func::Update`, `actobject.h:2143`); `SELF_INFO_00` quando muda. O `hp_gen`/`mp_gen` é o da classe **mais `vitalidade / 5` e `energia / 10`** (`UpdatePlayerMPHPGen`, `playertemplate.h:874-894`; `config.h:137-138`), refeito a cada `recalcular_por_nivel` (B124 — antes só a classe). Sentado: ×2 a partir do 2º batimento (`sit_down_filter`); **apanhar levanta** (`LeaveStayInState` no `GM_MSG_ATTACK`/`HURT`, `player.cpp:782-788`: `OBJECT_STAND_UP` 112 ao dono e a quem vê); sentado, só passa a lista do `StayInCommandHandler` (`playercmd.cpp:873-1015`; `DispatchCommand` no `PLAYER_SIT_DOWN`, `player.cpp:8797-8800`) — andar, atacar, conjurar, NPC, pegar, gestos, mascote e as consultas 67/68 são ignorados; `USE_ITEM` só de poção (`PlayerSitDownUseItem`/`SitDownCanUse`); `STAND_UP` e `CANCEL_ACTION` levantam (`comandos::sentado`, B143). A tabela de saltos do `gs` 1.2.6 (VA 0x84f4588) aceita os mesmos ids que tratamos, salvo 120 e 128. **Maldição levanta** como o golpe (`GM_MSG_ENCHANT` não amigável, `player.cpp:776-781`; `WorldInstance::maldicao_levanta`, B143) |
| **experiência e SP do abate** | `testado` | lista de dano no monstro; cada um recebe `exp × dano / max(total, max_hp)` (`DispatchExp`, `npc.cpp:1515`) com o ajuste da diferença de nível e `+0,5` (`ReceiveExp`, `player.cpp:2813`); `RECEIVE_EXP` (36) depois de somar. **Equipe (B164):** quem está em grupo soma o dano ao grupo (`progressao::repartir_abate`, porte de `DispatchExp` + `ReceiveGroupExp` + `player_team::DispatchExp`); a parte do grupo vai a todos os membros a até 100 m do monstro (bateram ou não), encolhida se algum que bateu está longe, vezes o ajuste da diferença *maior nível do grupo − nível do monstro* e, com menos de 20 níveis entre o maior e o menor, vezes `_team_adjust[membros] + _team_race_adjust[classes]` (`PARAM_ADJUST_CONFIG`, classes /20), dividida por nível (piso 20); quem recebe pela equipe não tem novo ajuste de nível. Igual no `gs` 1.2.6 (10000 = 100², piso 0x13, tabelas de 7 e 9 entradas). Diferença conhecida: a equipe é a da hora da morte, não a do golpe |
| **subida de nível** | `testado` | `IncExp`/`LevelUp` (`player.cpp:2627-2711,2831-2896`): curva `PLAYER_LEVELEXP_CONFIG` 202, +5 pontos de atributo, atributos refeitos (`recalcular_por_nivel`), vida e mana cheias, experiência zera no teto (`logic_level_limit` 105); `LEVEL_UP` (37) a todos, `SELF_INFO_00` e `OWN_EXT_PROP` ao próprio |
| reviver na cidade (C2S 4) | `testado` (B155, as duas versões) | `ZombieCommandHandler` (`playercmd.cpp:683-703`) abre a `session_resurrect_in_town` de **39** tiques; no fim, se ainda morto, `ResurrectInTown` (`:112-129`): ponto de cidade do distrito do `precinct.sev` que contém a posição (spec 03 §3.7), **que pode ser de outro mapa** (troca de mapa pelo roteador); sem distrito, o lugar. `Resurrect(nomove = false)` (`player.cpp:8716-8768`): `PLAYER_REVIVAL` (29) tipo **0** com a posição da morte, a quem vê e a ele, e o `LongJump` — `NOTIFY_POS` (14) + `OBJECT_STOP_MOVE` (`BusServer::transportar`). Até o B155 não havia sessão nem `NOTIFY_POS`: o cliente levantava onde morreu. `gs` 1.2.6: 0x27 tiques (VA 0x80cd250) |
| reviver com pergaminho (C2S 5) | `testado` (B155, as duas versões) | `session_resurrect_by_item` de **99** tiques → `ResurrectByItem` (`playercmd.cpp:64-110`): o 32021 e depois o 3043 (`gs/config.h:84-85`; o 1.2.6 só conhece o 3043, VA 0x80cc845) — senão 5; recarga `COOLDOWN_INDEX_SOUL_STONE` (10) — senão 54 —, armada com o `cool_time` do `REVIVESCROLL_ESSENCE` (1.800.000 ms) e `SET_COOLDOWN` (198); um a menos com `PLAYER_DROP_ITEM` tipo 10; `Resurrect(nomove = true)` → `session_resurrect_protect` (`actsession.cpp:1480-1503`): `PLAYER_REVIVAL` tipo **1** (a animação no lugar), `invincible_banish_filter` de 5 s (`PLAYER_REBORN_PROTECT`, estado visível 49 = `Efeitos::invencivel_s`) e tipo **2** ao fim |
| perda de experiência ao renascer | `testado` | `GetLvlupExp(nível) × GetResurrectExpReduce(cultivo)`, sem ficar negativa e zero no teto; **nenhuma até o nível 9** (`LOW_PROTECT_LEVEL`, `playercmd.cpp:697-699`; `cmp 9` no `gs` 1.2.6, VA 0x80cd2b4). Falta: morto por jogador (`_kill_by_player`) ainda perde; `_resurrect_exp_lost_reduce` (1.5.5) |
| distribuir pontos (C2S 22) | `testado` (B51) | `PlayerSetStatusPoint` (`player.cpp:8598`): recusa se alguma parcela ou a soma passa dos livres; soma, refaz vida/mana, evasão e precisão pela agilidade; `ADD_STATUS_POINT` (51, 22 bytes) com os quatro e o que sobrou (recusa com zeros). O cliente pede `GET_EXT_PROP` (21), que responde `OWN_EXT_PROP` (`PlayerGetProperty`, `:8588`) — antes só `SELF_INFO_00`. Grava atributos e pontos juntos (`gravar_atributos`) |
| munição (golpe normal) | `testado` (B51, B71) | arma de longo alcance (`weapon_type` 1) tira 1 do slot 11 (`DoAttack`, `player.cpp:3063-3070`). A contagem **mora na sessão de ataque**, como o `item_list` em memória do original: o banco é lido uma vez ao abrir a sessão (o número que vai no `HOST_START_ATTACK`) e a baixa é persistida fora do fio, senão a latência do banco alongava a cadência (B71). O `arrow_dec` do `ATTACK_ONCE` vale **1 sempre que a arma é de longe**, com ou sem flecha sobrando: o original ignora o retorno do `DecAmount` (`:3064-3070`). A flecha só sai depois das conferências do golpe. **B144:** a flecha só fica ativa com o tipo que a arma pede e o `weapon_level` na faixa dela (`projectile_equip_item::VerifyRequirement`, `equip_item.cpp:1251-1271`; `Equipamento::municao_ativa`); ativa, soma o `enhance_damage` ao dano e o `scale_enhance_damage` ao percentual (`UpdateEssence`/`NormalEnhance`). Arma de longe sem flecha ativa não golpeia (`range_weapon_item::OnCheckAttack`): `ERR_CANNOT_ATTACK` (9) ao abrir a sessão e `stop_attack(1)` quando a munição acaba. O Arco de Madeira (nível 0) aceita a Flecha de Novato (0–17) e não a de Iniciante (1–17) |
| mapa sem voo (B133) | `testado` | `nofly` do `gs.conf` (spec 03 §3.6e): decolar manda `ERR_CANNOT_FLY` **55** e não decola (`flysword_item::OnUse`, `item_flysword.cpp:55-70`; `push 0x37` no `gs` 1.2.6, VA 0x819505f); chegar voando a um mapa `nofly` derruba o voo com `OBJECT_LANDING` (`player.cpp:11994-11998`); mascote de ar não aparece (-2) e o de chão+ar/todos pula o ar (`petman.cpp:104/140/270`). Nenhum mapa servido hoje (o 1) é `nofly` |
| voo | `confirmado` | pelo item no slot 12 (`EQUIPIVTR_FLYSWORD`); sem teto. **Mana só nas asas** (B142, `testado`): as asas de Arqueiro/Anjo (`WINGMANWING_ESSENCE`) tiram `mp_launch` ao decolar e `mp_per_second` a cada segundo; sem a mana da decolagem a mana zera e não decola, e sem a do segundo pousa (`OBJECT_LANDING`) com a mana em zero (`angel_wing_item::OnUse`, `item_flysword.cpp:118-148`; `angel_wing_fly_filter::Heartbeat`, `fly_filter.cpp:42-48`; `DrainMana`, `player.cpp:10697-10712`). A espada voadora das outras classes não gasta mana: gasta o tempo de voo do item (`cls_flysword_item::OnFlying`) — `falta` descontar esse tempo. **Velocidade (B128, `testado`):** `fly_speed` do `ptemplate` + o `speed_increase` do item de voo (offset 20 do conteúdo gravado; `flysword_item::OnActivate`, `item_flysword.h:126-129`), teto `MAX_FLIGHT_SPEED` 20 (`playertemplate.h:1101-1110`), no `OWN_EXT_PROP`. Antes o item não somava: a Tsuko voava a 3 m/s com o de "15 m/s". Falta o `_en_percent.flight_speed` (`EnhanceFlySpeed`) |
| teleporte de GM (`GOTO`) | `confirmado` | `y` do cliente é marcador; altura = chão + 0,5 m (`playercmd.cpp:4926`) |
| painel de GM (Ctrl+G) | `testado` (2026-10-01, falta ver em jogo) | `bus_server/gm.rs`. B169: privilégio global reconciliado em sessões abertas; remoção limpa efeitos. Consumo de GM/GOTO/missões revalida conta/revisão/ban sob FOR SHARE (fora do mundo/tick). Cliente requer reentrada, sem recarga de auth online comprovada. Privilégio = `sec_level` > 0 (sem os bits por comando do `_gm_auth`); quem não é GM é ignorado calado. **Invencível** (205): `Efeitos::gm_invencivel`, sem prazo, dano zero (PvP inclusive), estado visível 49. **Invisível** (204): `Efeitos::gm_invisivel`, `PLAYER_LEAVE_WORLD` aos outros, fora da vista deles (o GM continua vendo todos), não golpeia (`DenyCmd(CMD_ATTACK)`), não é ferido (`target_faction = 0`); ao voltar, aparece para quem ele vê. Os monstros ainda o notam, como no original (o `WATCHING_YOU` não olha a invisibilidade de GM). **Ir até / chamar** (201/202): mapa e posição pelo roteador, `transportar` (troca de mapa inclusive). **Criar monstro** (208): só com `debug_command_mode = active`; `count` monstros a ±6 m (`CreateMinors`, `obj_interface.cpp:1990`), vida `life` s, sem ódio; `vis_id` e nome `falta`. **Criar item** (206) e **gerador** (207): `falta` |
| sentar, gestos, roupa, zona segura | `confirmado` | O **modo roupa persiste** (B83): o `SWITCH_FASHION_MODE` grava o `charactermode` em `characters.character_mode` — pares `(chave, valor)` de `int32`, chave 1, e nada quando desligado (`GetPlayerCharMode`, `gs/player.cpp:12585-12612`) —, o login o relê, e ele viaja cru no `RoleInfo` da lista de personagens, que é de onde a **tela de seleção** decide desenhar roupa ou armadura (`CECLoginPlayer::Load`, `EC_LoginPlayer.cpp:172-189`). `voando` continua sem persistir, de propósito: quem relogar entra no chão **Sentado**, o `sit_down_filter` dobra a regeneração de vida e mana a partir do 2º batimento (`STAYIN_BONUS` 100, `gs/config.h:103`; igual no `gs` 1.2.6, VA 0x812ff22) — `testado` (B118) |
| grupo | `testado` | estado de grupo no mundo (convite, aceite, recusa, saída) |
| teleporte e troca de mapa | `testado` (B51) | `LongJump` (`player.cpp:8617`): mesmo mapa → posição, `NOTIFY_HOSTPOS` (14, 22 bytes: `pos, tag, line`) e o mundo em volta; outro mapa **do mesmo processo** → o roteador tira o jogador do mapa de origem (some da vista, sessão e entidade) e o põe no destino: `NOTIFY_HOSTPOS` com o `tag` novo (o cliente descarrega e carrega o mundo, `JumpToInstance`), chão por baixo, grava mapa e posição na hora, e streaming completo (`global_message.cpp:111-117`). Disparado por prêmio de missão (`m_ulTransWldId`) e por missão com `m_bTransTo`. Mapa de outro processo `falta`; o grupo se desfaz na troca |
| coleta de recurso (C2S 54) | `testado` (B51) | `GATHER_MATERIAL` → confere coletores (30), ferramenta e missão de entrada (31), nível (51), distância `gather_dist` 4–20 m (2) (`matter.cpp:265-382`); tempo `Rand(time_min, time_max)` s; `PLAYER_GATHER_START` (126) a todos. Andar interrompe (`PLAYER_GATHER_STOP` 127). No fim: sucesso por `material_gain_ratio`; material por probabilidade, `num1` ou `num2` com `probability2`, limitado à pilha; `HOST_OBTAIN_ITEM` (99); o que não cabe vai ao chão do jogador; exp e SP da mina pelo `GetExpPunishment(nível − nível da mina)` e `+ 0,5`, com `RECEIVE_EXP` (`matter.cpp:439-446`, `player.cpp:2813-2829`; B144); a mina some (`OBJECT_DISAPPEAR`) e renasce em `max(dwRefreshTime, 15)` s. **Colher pode acordar monstro**: os `npcgen_1..4` do `MINE_ESSENCE` (`(monstro, quantidade, raio, vida em s)`) nascem no lugar da matéria — é assim que a Flor de Safira (44566), que **não produz material nenhum**, entrega a missão 31779: ela solta o Guardião de Almas (44608), agressivo, e é dele que cai o Estame com 80 % (B76). A missão só conta o item pelo `CheckMining` quando o método é "coletar N itens" (`TaskTempl.inl:2105-2145`), que não é o caso dessa. **Recarga** de 500 ms com `ERR_MINE_GATHER_IS_COOLING` (187) só no 1.5.5 (`playercmd.cpp:2345-2379`; o `gs` 1.2.6 não tem — `WorldProtocol::recarga_da_coleta_ms`). **Golpe interrompe** (normal, habilidade de monstro, PvP), salvo mina `uninterruptable` (`gather_interrupt_filter`, `skill_filter.cpp:37-46`; `EventoDoMundo::ColetaInterrompida`) (B144). `falta`: os `aggros_*` da matéria |
| bolsa do item colhido para missão | `confirmado no fonte` (2026-09-29) | Mina com `task_out` entrega o item pela missão (`OnTaskMining` → `ATaskTempl::CheckMining`, `TaskTempl.inl:2105-2145`): `ITEM_WANTED.m_bCommonItem` verdadeiro → `DeliverCommonItem` → **bolsa comum** (`taskman.cpp:281-297`); falso → `DeliverTaskItem` → bolsa de missão (`:313-326`). No `realm_155`, `TASKNORMALMATTER_ESSENCE` vem sempre com `comum` (2958/2958) e `TASKMATTER_ESSENCE` nunca (731/731) — exemplo `comum_por_tipo`. Ex.: os cristais da Maestria Elemental (44378–44382, missões 31797–31801, minas 44568–44572) vão para a bolsa comum, como no original. O material próprio da mina (`materials`) vai **sempre** à bolsa comum, qualquer que seja o tipo (`_inventory.Push`, `obtain_item(..., where 0)`, `player.cpp:1526-1532`) — alinhado no B150; antes ia à de missão quando era item de missão |
| restauração de atributos (serviço 33, B152) | `testado` nas duas versões | `resetprop_executor/provider` (`serviceprovider.cpp:3527-3690`; `gs` 1.2.6 VA 0x810fc0a/0x810fdde, serviço 33 em VA 0x8105491): pedido `{u32 index, i32 item_id}` (8 B); item 0 ou fora da bolsa, recusa calada; `index` fora da lista do NPC (`NPC_RESETPROP_SERVICE`, filtrada como `npcgenerator.cpp:750-780`), 14; item diferente do da opção ou ausente, 5. `RegroupPropPoint` → `__Rollback` (`player.cpp:14920`, `playertemplate.cpp:618-642`): tira até o delta de cada atributo sem descer do piso — **1.5.5: 5 nos quatro; 1.2.6: força/agilidade 5, vitalidade/energia 3** (`gs` 1.2.6 VA 0x80e7684, antes da correção de 2013; `WorldProtocol::piso_da_restauracao`) — e devolve aos pontos livres; nada a tirar, 82 e o item fica. Com sucesso, `OWN_EXT_PROP`, um item a menos e `HOST_USE_ITEM` (91) |
| produção (serviço 12, B145) | `testado` nas duas versões | `produce_executor` + `produce_provider::TryServe` (`serviceprovider.cpp:1451-1624`): pedido `{skill, id, count}` (12 B); receita existente, não de melhoria, da habilidade pedida e com o nível `require_level`; sem a taxa, 16; habilidade/receita fora do `NPC_MAKE_SERVICE` do NPC, 20. `session_produce` (`actsession.cpp:657-700`): `PRODUCE_START(use_time, count, receita)` (8 B), um item a cada `use_time` tiques, `PRODUCE_END`; `CANCEL_ACTION` encerra. `ProduceItem` (`player.cpp:16404-16641`): taxa, slot livre (7), `RandUniform() > null_prob` → alvo pela probabilidade, materiais (25); saindo item, exp/SP com a punição do nível 150 e proficiência +2 (habilidade abaixo do nível da receita) ou +1 (igual) até o `GetMaxAbility` do stub (10…200 nas 158–161/1402; `SKILL_ABILITY` 187), `SPEND_MONEY`, materiais com `PLAYER_DROP_ITEM` tipo 7, item gerado e `PRODUCE_ONCE` (14 B no 1.5.5, **10 B no 1.2.6**); sem item, `PRODUCE_NULL` (210). Aprender o próximo nível de uma habilidade de produção exige a proficiência cheia (`SkillWrapper::Learn`, `skillwrapper.cpp:84-89`). A proficiência fica em `character_skills.ability` e no `SKILL_DATA`. `falta`: as produções 2–5 (síntese com materiais escolhidos, melhoria, herança — `produce2..5_executor`), a decomposição (13), o nome do fabricante no item (`IMT_PRODUCE`) e a lista `ADDON_LIST_PRODUCE` (o item sai com a geração de drop) |

## 7.9 Rates do realm (`taxas.rs`) — `testado` (B176, falta ver em jogo)

Fonte: `realms.double_{exp,sp,drop,gold}_multiplier` (`NUMERIC(3,1)`, 0,1–99,9), lidas pelo
roteador na partida (`RoteadorDeMapas::carregar_taxas`, valor inválido → 1× com aviso) e
trocadas na hora pelo canal administrativo (`definir_taxas`, grava e aplica, arredonda a
uma casa). Original: `world_param` (`worldmanager.h:95`).
- **EXP / SP:** só no abate (incluindo a parte de equipe), antes de `ganhar_exp` e do
  `RECEIVE_EXP`, como o fator de `IncExp` (`player.cpp:2906-2922`); truncado
  (`player.cpp:2835-2836`). EXP de missão (`ReceiveTaskExp`) não passa. Fatores
  **independentes** (o original acopla SP ao fator de EXP) — decisão do Murillo.
- **Drop:** o bloco de itens inteiro repete `sorteios(drop)` vezes, cada um com o seu teste
  de `drop_adj` (`item_more_times`, `npc.cpp:2663-2685`).
- **Moedas:** rodadas de dinheiro × `sorteios(moedas)` (`money_more_times`, `npc.cpp:2691-2696`).
- **Fração:** `sorteios(f)` = parte inteira + 1 com a chance da fração (1,5 = 1 garantido + 50%
  de outro) — o original só tem ×2; regra decidida pelo Murillo.

## 7.10 Mapas ligados e desligados pelo painel (`mapas.rs`) — `testado` (B177, B183; falta ver em jogo)

**Ligado = carregado e aceitando entrada.** O conjunto de mapas do `RoteadorDeMapas` muda em
execução (B183): `std::sync::RwLock<HashMap>`, nunca segurado durante `await`.
Estado em `realms.config` (JSONB): `mapas_desligados` e `mapas_ligados` (os ligados pelo
painel além do `WORLD_TAGS`), trocados numa só instrução (`definir_estado_do_mapa`, preserva
as outras chaves). **Partida:** carrega `(WORLD_TAGS ∪ mapas_ligados com dados) −
mapas_desligados`; o primeiro do `WORLD_TAGS` segue padrão mesmo descarregado.
**Ligar** (`definir_mapa`): carregado → só libera a entrada; não carregado → grava, marca
`carregando` e monta numa tarefa (`WorldInstance::new` + `init_spawns` em `spawn_blocking`,
fora da guarda de presença; só a inserção pega a guarda em escrita); a resposta volta na hora
com `carregando: true` e o resumo (`mundos`) lista o mapa com `carregando` até subir. Só monta
mapa com dados no `GameDataManager` (`pastas_de_mapa`: `world`, `aNN`, `bNN`); senão
`mapa_sem_dados`. Mapa que o processo não carrega só é aceito com `carregar: true` — o painel
manda isso só ao primeiro processo do realm quando todos responderam `mapa_nao_servido`.
**Desligar:** grava, guarda de presença em escrita, cada personagem sai por
`expulsar_pelo_painel` (logout salvo, `PlayerLogout` result 2 → login), e o mapa é
descarregado (`BusServer::descarregar`: aborta o tique e fecha o canal de eventos, que é o que
segurava o laço de eventos vivo). Se alguém não saiu (saída pendente), o mapa fica carregado e
bloqueado. Desligado durante a carga: o mapa montado é descartado. Enquanto desligado,
`EnterWorld` para ele (ou para o padrão descarregado) é recusado com o mesmo `PlayerLogout`
(o link registra a sessão antes do `EnterWorld`, `pw-link/src/gateway.rs:858-859`) e a troca
de mapa para ele é recusada. Personagem gravado num mapa desligado não vai ao padrão: é
recusado. `mundos` traz também `carregaveis` (mapas com dados).
Limitações: instâncias do `gs.conf` que dividem pasta (`is73–75` em `a72`, `is81–83` em `a80`)
e `m01`/`random03`/`random04` não têm dados no carregador — aparecem "sem dados"; cada mapa é
um mundo único compartilhado (o original cria cópias por entrada em `instance_servers`).

## 7.11 Edição de personagem pelo painel — `testado` (B179, B182, B184–B187, B191, B194; falta ver em jogo)

`RoteadorDeMapas::editar_personagem`, com a guarda de presença em leitura (a entrada pega em
escrita) e a trava de gravação do personagem. Só dar (B180: tirar arriscaria valor
negativo). **Online:** o mapa dono aplica pelo mesmo caminho da recompensa de missão
(`Jogador::dar_dinheiro` → `task_deliver_money` 159, `dar_exp` → `ganhar_exp` + `task_deliver_exp` 158, que já
sobe de nível) e o `com_contexto` grava. **Offline:** só dinheiro, soma atômica no banco
(`CharacterRepository::ajustar_dinheiro_offline`, nunca negativa, teto 2 000 000 000); EXP/SP
exigem o personagem em jogo (subir de nível depende da entidade). Saída pendente = em
transição (repetir). ID da operação deduplicado em `comandos_administrativos` (reserva antes,
resultado depois; reserva sem resultado = desconhecido). Limitação: um GS por realm (o
deploy atual); com vários, o offline precisaria confirmar ausência em todos.

**B182 — pontos, nível e cultivo** (um tipo por operação; online grava pelo `com_contexto`,
offline com `UPDATE` atômico no banco):

| edição | online (o que o cliente recebe) | offline | limite |
| :--- | :--- | :--- | :--- |
| pontos livres (dar) | `ADD_STATUS_POINT` (51) com os 4 em zero e `remain` novo: o cliente troca os pontos livres e pede a ficha (`EC_HostMsg.cpp:1610-1625`) | `potential_points += n` | 1–10 000 por operação (política do painel) |
| nível (só sobe) | `progressao::subir_ate`: o passo do `LevelUp` sem gastar EXP (`player.cpp:2645-2660`) — +5 pontos por nível, atributos refeitos, vida/mana cheias, EXP zera no teto; um `LEVEL_UP` (37) por nível a si e a quem vê (o cliente soma 1 a cada, `EC_HostPlayer.cpp:4163-4171`); `SELF_INFO_00` e `OWN_EXT_PROP` acertam o resto | nível, +5 pontos por nível, EXP zera no teto; vida/mana como estão | > nível atual, ≤ `GetMaxLevel` do realm; descer não existe no original fora do renascimento (`player_reincarnation.cpp:119-125`) |
| cultivo | `SetSecLevel` → `TASK_DELIVER_LEVEL2` (160) difundido a quem vê, ele incluído (`player.cpp:4865-4872`; antes do B182 só ia ao próprio) | `cultivation = v` | 1.5.5: 0–8, 20–22, 30–32 (`GetLevel2Name`, `EC_GameRun.cpp:3483-3493`); 1.2.6: 0–8 (o `gs` 1.2.6 não tem `GodEvilConvert`) |

**B184 — atributos já distribuídos.** `atributos: [força, agilidade, vitalidade, energia]`
(modificar) ou `redistribuir: true` (todos ao piso). O total (atributos + pontos livres) é
conservado: `PlayerEntity::definir_atributos` = `restaurar_atributos` (o `RegroupPropPoint`,
`player.cpp:14920-14940`: tira até o piso e devolve ao livre) + `distribuir_pontos` (o
`SetStatusPoint`). Piso por versão (`piso_da_restauracao`: 1.5.5 5 nos quatro,
`playertemplate.cpp:618-641`; 1.2.6 3 em vitalidade e energia); atributo já abaixo do piso
não desce. Para subir além do total, dar pontos livres antes. Online: recalcula, reaplica o
equipamento (`recalcular_equipamento`, o `RefreshEquipment`) e manda `OWN_EXT_PROP` (50), que
grava atributos e pontos livres absolutos no cliente (`EC_HostMsg.cpp:1583-1584`). Offline:
um `UPDATE` com as mesmas condições (`definir_atributos_offline`). Falhas:
`atributos_invalidos`, `sem_mudanca`, `atributos_invalidos_ou_personagem_inexistente`.

**B185 (E6) — posição.** `posicao: {mapa, x, y?, z}`, tratada no roteador
(`mover_pelo_painel`), não no contexto do jogador. Destino: mapa carregado e ligado neste
processo (`mapa_indisponivel`); `(x, z)` dentro do terreno do destino (`fora_do_mapa`); `y`
ausente = o chão, e abaixo do chão sobe para ele (`if (pos.y < height) pos.y = height`,
`global_message.cpp:100-101`); mapa sem terreno carregado exige `y` (`altura_obrigatoria`).
Online: o `transportar` do GM e da missão — mesmo mapa `NOTIFY_HOSTPOS` + parada para quem vê;
outro mapa, pedido de troca na fila (`troca: true`), que grava mapa e posição ao chegar.
Offline: `gravar_posicao_offline` sob a guarda de presença e a trava de gravação. Coordenadas
±100 000 são política do painel. Pedido fora do contrato JSON (campo desconhecido) fecha a
conexão sem resposta, como os demais; o painel valida antes.

**B186 (E6) — itens, parte 1: ver e dar.** `inventario` (consulta só de leitura) lê bolsa,
equipamento, armazém e bolsa de missão do banco — a fonte que o `com_contexto` relê a cada
operação — com o nome do `elements.data` (`GameDataManager::nomes_de_itens`: o `Name` dos
registros com `pile_num_max`, mesmo critério de `pilhas`; 126 7.896/7.896 com nome, 155
24.806/26.188 — o arquivo traz `Name` vazio no resto). `buscar_itens` acha até 30 por nome ou
id. **Dar** `item: {id, quantidade}`: o prêmio de missão (`Jogador::dar_item`, gerado como drop,
`DeliverCommonItem`, `task/taskman.cpp:281-303`, `TASK_DELIVER_ITEM` 156); item de missão vai à
bolsa de missão. O original entrega no máximo uma pilha por vez (`count > pile_limit` → corta,
`taskman.cpp:289-292`): o painel entrega em lotes de uma pilha, cada um com seu 156, e antes
simula todos numa cópia — se não cabe tudo, `bolsa_cheia` e nada entra. Offline: a mesma
geração na bolsa lida do banco e gravada de volta (`dar_item_offline`), sob a guarda de
presença e a trava de gravação. Item que o realm não tem: `item_inexistente`. Remover item e
equipar ficam para a parte 2.

**B187 (E6) — itens, parte 2: remover.** `remover_item: {recipiente, slot, id, quantidade?}`
(`bolsa`, `missao`, `equipamento`, `armazem`; sem quantidade = a pilha inteira). O `id` confere o
item do slot (`slot_mudou` se mudou desde a consulta); quantidade acima da pilha =
`quantidade_invalida`. Online só bolsa e bolsa de missão: tira do slot e manda
`PLAYER_DROP_ITEM` (46) com `DROP_TYPE_GM` = 0 (`common/protocol.h:927-929`), que o cliente
avisa como "GM removeu" com o nome (`EC_HostMsg.cpp:1786-1788`) e tira do pacote. Equipamento
(exigiria aparência para quem vê) e armazém (sem pacote conferido no cliente) online =
`precisa_estar_offline`. Offline: tira do recipiente lido do banco e grava só o slot
(`remover_item_offline`). O valor 0 de `DROP_TYPE_GM` vem do 1.5.5; no 1.2.6 não foi conferido
no binário (só muda a mensagem do cliente). Equipar/mover fica para a interface de inventário.

**B191 (E6) — arrastar.** `mover_item: {de, slot_de, id, para, slot_para}` troca o item `id` do
slot de origem com o que estiver no destino (pilhas inteiras, como as trocas do jogo). Pares que o
jogo move (`movimento_valido`): bolsa↔bolsa, corpo↔corpo, bolsa↔corpo, armazém↔armazém,
bolsa↔armazém, missão↔missão (senão `movimento_invalido`); slots dentro de bolsa/missão 32
(`gs/config.h:12-15`), armazém 16, corpo < 64 (`slot_invalido`); mesmo slot = `sem_mudanca`;
item diferente na origem = `slot_mudou`; peça que não cabe no slot do corpo = `posicao_invalida`.
**Online** só bolsa e corpo, pelos tratadores do cliente — `EXG_IVTR_ITEM` (44), `EXG_EQUIP_ITEM`,
`EQUIP_ITEM` (vestir/tirar) —, que gravam em transação, avisam o cliente, refazem o equipamento e
mandam a aparência a quem vê; o cliente aplica esses comandos sem tê-los pedido
(`EC_HostMsg.cpp:1742-1758`, `:1861-1935`). Antes, `motivo_para_nao_vestir`/`troca_no_corpo_cabe`
conferem o que o tratador recusaria (posição, requisito → `requisito`, Forma Sombria →
`equipamento_travado`), para não mandar ao jogador um erro que ele não causou; depois relê o
destino (`nao_aplicado` se não chegou). Armazém (o cliente só tem a cópia dele com a sessão de NPC
aberta, `armazem_aberto`) e missão (sem comando de troca) online = `precisa_estar_offline`.
**Offline:** `swap_slots`/`move_between_containers` em transação, com a posição conferida;
requisitos de nível/classe/atributos só online (dependem dos atributos com o equipamento).

**B194 (E6) — editar o item (edição livre nos valores, não no formato).** `editar_item:
{recipiente, slot, id, edicao: {quantidade?, durabilidade?, durabilidade_maxima?, requisitos?:
{nivel, classes, forca, agilidade, vitalidade, energia}, fabricante?, efeitos?: [{id, args}],
refino?, pedras?: [id por furo]}}` (`bus_server/item_editado.rs`). Os octetos continuam a verdade:
o rabo (furos, máscara das pedras, efeitos) muda pelo `alterar_rabo` do refino/pedras, o cabeçalho
por remendo (requisitos 6 × i16 com vitalidade antes de agilidade; durabilidade; nome do
fabricante) e a essência fica byte a byte; o bloco tem de se reler inteiro (`formato_invalido`).
`efeitos` troca só os sem origem (pedra/conjunto/gravação ficam); `refino` 0–12 põe o efeito de
refino com `base × refine_factor[n] + 0.1` (`equip_item.cpp:208-223`; acima de 12 não há fator —
só pela lista crua); `pedras` refaz os furos com os efeitos embutidos da pedra e a máscara de
cores. Limites de formato: 5 furos, 32 efeitos, id de efeito em 13 bits com até 3 parâmetros
(`itemdataman.h`), nome até 40 bytes. Colunas espelho (durabilidade, refino, furos) gravadas no
mesmo upsert. **Online:** grava e manda `OWN_ITEM_INFO` (40), que o cliente aplica sobre o item já
no slot (`EC_HostMsg.cpp:1454-1500`; no corpo refaz a aparência do próprio jogador) e, no corpo,
`recalcular_equipamento`; armazém = `precisa_estar_offline`. **Offline:** grava. Falhas:
`slot_mudou`, `nao_e_equipamento`, `sem_refino_nem_furos`, `pedra_inexistente`,
`refino_sem_addon`, `formato_invalido`. Não editáveis: o id do modelo (o `OWN_ITEM_INFO` não o
troca) e o vínculo (o `state` sai sempre 0 — `proc_type` não modelado). `falta`: o
`_modify_mask << 16` no id do equipamento visto pelos outros (`equip_item.cpp:25-30`; o GS manda
só os 16 bits baixos), então o brilho do refino não aparece para quem vê.

Falhas novas: `nivel_invalido` (com `nivel_maximo`), `nivel_invalido_ou_personagem_inexistente`
(offline), `cultivo_invalido`. A impressão de deduplicação só inclui `pontos`/`nivel`/`cultivo`
quando presentes (operação B179 pendente continua com a mesma impressão).

## 8. Itens e economia

**Cobertura de versão (B93):** o cenário geral de `subcomandos_no_mundo.rs` é
155; compra e pickup também têm cenários explícitos 126. Banco e BusServer recebem
a mesma versão. Permanecem os gabaritos unitários 126; quatro cenários de mundo
passaram com banco. Retirada do amuleto passa pelo trait; ELF_EXP é opcional
para omitir id 283 inexistente no 126, sem alterar ganho/persistência do Daimon.

**Protocolo 126, testado (B90):** `Contexto` recebe `WorldProtocol` para codificar
31/46/72/99/156. Somente o layout varia (spec 04); regras de empilhamento,
cobrança e persistência continuam comuns. Experiência 36/158 reproduz capturas.
Sete testes de protocolo e dois de mundo aprovados com banco; não confirma
jogabilidade completa de missões v55 (leitor estrutural no B96, mas projeção dos
requisitos ainda parcial), preços v7 nem C2S de compra.


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
| reparar | ver "reparar" na tabela da economia (B142) |

A bolsa é lida do banco a cada operação e gravada de volta só nos slots que mudaram
(`economia::Bolsa`). O **dinheiro vive na entidade** (o autosave grava a entidade por cima do
banco); toda operação que mexe nele passa por `com_contexto` e grava na hora.

| regra | estado | detalhe |
| :--- | :--- | :--- |
| repositório de itens | `testado` | transacionado; troca de slot preserva os octetos do item (A37) |
| equipar | `confirmado` | com bloco de dados (spec 04 §5) |
| requisito de equipamento | `testado` (B148, 1.5.5 e 1.2.6) | `EquipItem` → `CanActivate` → `equip_item::VerifyRequirement` (`gs/player.cpp:8476-8493`, `gs/item/equip_item.cpp:60-80`): vestir (15) e mover para o corpo (18) só passam se o jogador atende o nível, o bit da **classe** (`1 << (classe & 0x0F)` na máscara `race` da `prerequisition`) e força/vitalidade/agilidade/energia de `_cur_prop` (`atributos_efetivos`), e se a durabilidade não é zero; senão `ERR_ITEM_CANNOT_EQUIP` (8) e os dois slots destravados (`BusServer::pode_vestir`, `pw_core::Requisitos`). Requisitos do bloco gravado, ou do que o `item_info` monta do modelo quando o item não tem bloco. Antes, o servidor vestia qualquer coisa (relato do 1.2.6: peça de outra classe comprada no NPC, vermelha no cliente, entrava no corpo). **B191 (`testado`, 1.5.5 e 1.2.6):** posição (`CheckEquipPostion`, `gs/item.h:294-297`) ao vestir/mover para o corpo (`player.cpp:8150`) e na troca dentro do corpo (`:8008-8045`), pela máscara da família (`pw_data_loader::posicoes`); sem tabela carregada não confere. `falta`: reputação, nível histórico, amuleto de HP/MP que não sai (`ERR_ITEM_CANNOT_UNEQUIP`, `:8100-8124`, código não conferido no 1.2.6) e troca do amuleto vestido, habilidade dinâmica repetida (`:8158-8168`), `VerifyRequirement` de roupa e de item de voo |
| equipamento comprado e fabricado | `testado` (B151, 1.5.5 e 1.2.6) | Um gerador, três variantes (`pw_gs::geracao::Geracao`), como o original: **drop** (`generate_item_for_drop`: `NORMAL`, `ADDON_LIST_DROP` = `addons`, furos `drop_probability_socket`, durabilidade gasta `min(RandNormal(drop), máx)` salvo `proc_type & 0x1000`, tag `IMT_DROP` 2); **fabricação** (`ProduceItem` → `generate_item_from_player`, `player.cpp:16499-16515`, `itemdataman.cpp:1239-1246`: `NORMAL(0)`, `ADDON_LIST_PRODUCE` = `rands`, furos `make_probability_socket`, durabilidade **cheia**, tag `IMT_PRODUCE` 4 + nome do fabricante em UTF-16LE até 40 B); **loja** (`get_item_for_sell`, `itemdataman.cpp:1352-1379`, usado pelo NPC e pela Loja Gold `player.cpp:15873, 15941`: `SPECIFIC(0)` = mínimo das faixas, índice 0 nos sorteios → sem furo, sem addon, durabilidade cheia `durability_min`, tag `IMT_SHOP` 3). A máscara de classes (`character_combo_id & 0xFFFF`) vai na `prerequisition` do bloco nos três. Durabilidade: `generate_item_temp.h:292-310`. Depois da compra o cliente pede os blocos com `GET_ITEM_INFO_LIST` (53, spec 04 §6); sem resposta o item ficava vermelho e sem tooltip. Falta: roupa comprada/fabricada ainda com tag 0 |
| empilhar na bolsa | `testado` | `CECInventory::MergeItem` (`EC_Inventory.cpp:179-215`): completa pilhas na ordem dos slots, o resto no primeiro vazio; limite `pile_num_max`. O cliente confere o slot e a quantidade devolvidos |
| conteúdo da roupa (B129) | `testado` | `FASHION_ESSENCE` entra com 10 B: `int require_level`, `u16 color` (`RandNormal(0, 0x7FFF)`), `u16 gender`, etiqueta de 2 B (`generate_fashion_item`, `generate_item_temp.h:1642-1712`; mesma ordem no `gs` 1.2.6, VA 0x81f6fdc). `GameDataManager::conteudo_da_roupa`, usado pelo `empilhar_gerado` (Loja Gold, prêmio, drop) e pela compra no NPC. Sem ele o cliente lia `gender` 0 (masculino) e recusava a roupa feminina |
| comprar de NPC | `testado` | roupa e item de voo com conteúdo próprio (`get_item_for_sell`); cabeçalho do pedido por versão (`WorldProtocol::bytes_do_cabecalho_da_compra`: 28 B no 1.5.5, **8 B no 1.2.6** — B128, spec 04); preço `max(shop_price, price)`; empilha e responde `PURCHASE_ITEM` (72) (`PurchaseItem`, `player.cpp:8900`); sem dinheiro `ERROR_MESSAGE` 16; `falta` conferir a lista de venda do NPC |
| vender a NPC | `testado` | `price × quantidade`, proporcional à durabilidade (`ItemToMoney`, `player.cpp:13930`); `ITEM_TO_MONEY` (73); o `price` do cliente é ignorado; antes do 73, um `UNFREEZE_IVTR_SLOT` (181) por espaço pedido, também o recusado — o cliente congela o espaço ao pedir e só o solta com ele (captura 1.2.6, t = 2540,77 s; B109) O item do pedido tem 16 B no 1.5.5 (`npc_sell_item` com `price`) e **12 B no 1.2.6** (sem `price`, medido no pedido real: `len` 148 = 4 + 12 × 12) — `WorldProtocol::bytes_do_item_vendido` (B116; antes só o 1º item saía certo e os outros ficavam sombreados) |
| reparar (B142) | `testado` | serviço 3, pedido `{int type; u8 where; u8 index}` (`serviceprovider.cpp:600-607`). **Tudo** (`type` −1, `RepairAllEquipment`, `player.cpp:9692-9713`): só o equipamento vestido; soma em `float` `repairfee × (máx − atual)/máx` de cada peça gasta (`GetRepairCost`, `playertemplate.h:535-546`; `repairfee` de `WEAPON`/`ARMOR`/`DECORATION_ESSENCE`, `itemdataman.cpp:1016-1034`) e trunca a soma; nada gasto, nada acontece; soma zero com peça gasta custa 1; paga se `custo < dinheiro`, senão `ERROR_MESSAGE` 16; `REPAIR_ALL` (74). **Uma peça** (`Repair`, `player.cpp:9755-9784`): bolsa ou corpo, piso 1, `REPAIR` (75). Peça com `proc_type & 0x1000` fica de fora. A durabilidade volta à máxima no banco e em `PlayerEntity::pecas` |
| fôlego debaixo d'água (B146) | **não existe no original** | o `breath_ctrl` (`gs/breath_ctrl.h`, `.cpp`) só entra em "debaixo d'água" pelo `ChangeState`, e as únicas chamadas estão num bloco **comentado** do `TestUnderWater` (`player.cpp:14322-14348`); o `gs` 1.2.6 também não chama `breath_ctrl::ChangeState` (VA 0x8134b58) em lugar nenhum. O servidor nunca desconta fôlego nem manda `BREATH_DATA` (189): não há o que portar |
| saldo ao cliente | `testado` (B142) | `GET_OWN_MONEY` (82) só quando o dinheiro muda num `com_contexto` — a experiência do abate não o manda mais (o B56 mediu o saldo em dobro por abate); o `PLAYER_CASH` da entrada é só o do mundo, com o cash da conta (o link mandava um `0` antes) |
| durabilidade | `testado` (B61) | escala interna, desgaste e quebra — ver "Durabilidade" acima |
| curar no NPC | `testado` | pelos valores do jogador |
| aprender habilidade | `testado` | `skill_executor::OnServe` + `SkillStub::LearnCondition`/`Learn` (`serviceprovider.cpp:1288`, `cskill/skill/skill.cpp:14-93`): habilidade da lista do treinador (`NPC_SKILL_SERVICE`), fora de combate, nível ≤ máximo, classe, pré-requisitos, nível, SP, `rank` × cultivo, dinheiro, e o livro do nível (`GetRequiredItem`, `SetUseitem` → `TakeOutItem`, `skill.cpp:79-84`; sem ele 22); cobra (`SPEND_MONEY` 77, `COST_SKILL_POINT` 94), o livro sai com `PLAYER_DROP_ITEM` (46, `DROP_TYPE_TAKEOUT` 2) (B113) e responde `LEARN_SKILL` (95). Requisito `null` na tabela recusa |
| aljava no drop (B52) | `testado` | `QUIVER_ESSENCE` vira `id_projectile` × `Rand(num_min, num_max)` (`generate_quiver`, `generate_item_temp.h:650-667`); o 1955 do Espírito da Estrela caía cru |
| Carta da Sorte (B53) | `testado` | `TASKDICE_ESSENCE`: usar sorteia a missão por `task_lists` (`RandSelect`) e entrega pelo motor (`OnTaskCheckDeliver`); aceitou, gasta uma e responde `HOST_USE_ITEM`; recusou, `ERROR_MESSAGE` 18 e a carta fica; em combate com `no_use_in_combat`, erro 66 (`item_taskdice.cpp:12-42`) |
| equipamento sorteado no drop (B53) | `testado` | ver §5.1 |
| caixa de Cartas de General (B95) | `testado` | `POKER_DICE_ESSENCE` (32 caixas no 155; a "Caixa de Tesouro do Guerreiro" é a 41073): bolsa cheia recusa **sem erro** (o `error_cmd` está comentado no original) e só destrava o slot; senão sorteia uma das 256 entradas (`RandSelect`), gera a carta com o `generalcard_essence` de 32 B (tipo, qualidade, nível exigido, liderança sorteada em `require_control_point`, nível máximo, nível 1, exp 0, renascimentos 0), manda `HOST_OBTAIN_ITEM` (99) e gasta a caixa (`HOST_USE_ITEM`). `item_generalcard_dice.cpp:10-54`, `generate_item_temp.h:3144-3191`. **O sistema de cartas não existe**: equipar, liderança, atributos, nível, devorar — `falta` |
| **drop de monstro** | `testado` | dono = maior dano (+`max_hp/4` do primeiro golpe). Itens: `drop_times` rodadas de `probability_drop_num0..3` e `drop_matters[32]` (da 2ª rodada, só índices < 16), com o ajuste de item por nível (`DropItemFromData`, `npc.cpp:2649`; `generate_item_from_monster`, `itemdataman.cpp:1191`). Moedas: `drop_times` vezes, `Rand(médio±variação)`, chance 0,7, × ajuste. Cada monte a ±2 m, no chão (`worldmanager.cpp:512-555`), `tid` 3044 para moedas, id de matéria `0xC8…` |
| item no chão | `testado` | posse do dono por **30 s**, some em **300 s** (`matter.h:62`, `matter.cpp:133`); `MATTER_ENTER_WORLD` a quem está a 120 m e no streaming; `OBJECT_DISAPPEAR` ao sumir |
| **pegar** (C2S 6 e 184) | `testado` | tipo confere, distância < 10 m, posse; moedas `PICKUP_MONEY` (30), item `PICKUP_ITEM` (31) **sempre na bolsa comum** (`_inventory.Push`, `gplayer_imp::OnPickupItem`, `player.cpp:8933-8962`; até o B128 o `proc_type & 0x20` mandava arma à bolsa de missão); `MATTER_PICKUP` (152) a todos; bolsa cheia `ERROR_MESSAGE` 7, fora da posse 6 (`playercmd.cpp:1347-1444`, `matter.h:97-129`) |
| **descartar** (C2S 14 e 15) | `testado` (B84) | Joga o item no chão **sem dono** — item que se joga fora é de quem pegar (`DropItemFromData` com `XID(0,0)`, `ThrowEquipItem`, `gs/player.cpp:7932-7980`). A bolsa manda `{u8 índice, u32 quantos}`; o corpo só o índice, e vai a peça inteira. Responde `PLAYER_DROP_ITEM` (46) com `DROP_TYPE_PLAYER` = 1 **e** o `UNFREEZE_IVTR_SLOT` (181) |
| **congelamento de slot** | `testado` (B84) | **O cliente congela o slot antes de mandar qualquer comando de item** (`c2s_CmdDropIvtrItem` e os vizinhos, `Network/EC_GameSession.cpp:6304-6390`), e só o `UNFREEZE_IVTR_SLOT` (181) ou o fim de uma troca limpam esse estado (`EC_HostMsg.cpp:2060-2064`). Item congelado fica apagado e não pode ser movido nem usado. Portanto: **todo tratador de comando de item devolve o 181, inclusive quando desiste** — slot vazio, falha de banco, comando não tratado. O original faz isso com o `UnLockInventoryHandler` (`gs/playercmd.cpp:183-230`, chamado também no estado de morto, `:654-678`); do lado do servidor o comando chama-se `unlock_inventory_slot` (`gs/player.cpp:5059-5066`). Os comandos de **armazém** (56–60) destravam os dois slots antes de tudo, como o original (B147) |
| poção (`USE_ITEM`) | `testado` (B67, B71) | `MEDICINE_ESSENCE`. **Restaura ao longo do tempo**: `hp_add_total / hp_add_time` por batimento de 1 s, e o mesmo para mana — é o `healing_potion_filter`/`mana_potion_filter` do original (`gs/item/item_potion.cpp:18-52`, `gs/potion_filter.h:6-130`), que reparte o total pelo tempo. Só a poção com vida **e** mana e sem tempo (`rejuvenation_potion`) cura na hora. **Recarga** (B70/B71): `CheckCoolDown` **antes** de consumir, recusa com `ERR_OBJECT_IS_COOLING` (53) e `SetCoolDown(índice, cool_time)` com `SET_COOLDOWN` (198) ao cliente. O índice é o da **família**, e a família vem do `id_major_type` do arquivo (`setclassid.cpp:81-101`), não do que a poção restaura: 11 vida, 12 mana, 3 vida+mana, 13 antídoto (`COOLDOWN_INDEX_*`, `gs/cooldowncfg.h:62-78`) — poções da mesma família compartilham a recarga |
| Daimon (`GOBLIN_ESSENCE`) | `parcial` (B75) | O "pequeno elfo" do original (`elf_item`), vestido no slot **23** (`EQUIP_INDEX_ELF`, `gs/item.h:219`). O estado dele **é** o bloco do item: `elf_essence` de 38 bytes com `#pragma pack(1)` (exp, nível, total de atributos, força/agilidade/vitalidade/energia, total de gênios, 5 gênios, refino, vigor 20000, estado), depois a lista de equipamento e a de habilidades, cada uma com a contagem em 4 bytes (`elf_item::Save`, `gs/item/item_elf.cpp:172-185`; `generate_elf`, `generate_item_temp.h:2442-2524`). **Experiência:** um décimo da que o jogador ganha, sempre (`ElfReceiveExp(exp / 10)`, `gs/player.cpp:2921-2928`); `InsertExp` (`item_elf.cpp:692-750`) aplica o fator `nível do Daimon ÷ nível de quem deu` (mínimo 10 %), sobe quantos níveis couberem e **para a um ponto** de alcançar o dono. Cada nível dá 1 ponto de atributo, e 1 de gênio a cada 5 níveis até o 100. Sem subir de nível vai `ELF_EXP` (283) ao cliente; subindo, a ficha inteira do item. `falta`: bônus de atributo sorteado de 10 em 10 níveis (`rand_prop`), equipamento e habilidades do Daimon, vigor, pílulas, decomposição, refino e distribuir pontos |
| montaria (`SUMMON_PET` C2S 100 / `RECALL_PET` 101) | `testado` (B78, B79) | invocar um mascote de montaria **é montar** (`PlayerSummonPet` → `pet_man::ActivePet`, `gs/player.cpp:14474-14491`, `gs/petman.cpp:319-392`). **É uma sessão, não um comando** (`session_summon_pet`, `gs/actsession.cpp:1705-1721`): o servidor só confere que o mascote existe, manda `PLAYER_START_PET_OP` (235, `{slot, pet_id, delay, op}`) e espera a canalização — **60 ticks** para invocar, 10 para recolher, em ticks de 50 ms (`TICK_PER_SEC 20`, `gs/config.h:43`) —; só então aplica o efeito, e fecha com `PLAYER_STOP_PET_OP` (236), **mesmo quando recusa**. Aplicar é: `PLAYER_MOUNTING` (227, `{id, mount_id, u16 color}`) mais a ficha nova, e depois **`SUMMON_PET` (233, `{slot, pet_tid, pet_pid, life_time}`)**, que é o que diz ao cliente qual mascote ficou ativo (`SetActivePetIndex`, `EC_HostMsg.cpp:5274-5296`) — sem ele o botão de recolher da jaula fica desabilitado (`DlgPetList.cpp:227`) e invocar de novo responde "já está ativo" (B79). Desmontar manda `PLAYER_MOUNTING` com zero nos dois e `RECALL_PET` (234, `{slot, pet_tid, u8 motivo}`, 9 bytes de corpo) com `PET_RECALL_DEFAULT` = 0 (`mount_filter.cpp:24-45`, `player.cpp:14279-14319`, `petman.cpp:1337`, `:1376`). A velocidade é `speed_a + speed_b × (nível − 1)` do `PET_ESSENCE` (`pet_dataman::CalcMountParam`, `gs/petdataman.h:186-194`) e **sobrepõe** a de corrida. Um pedido novo invalida a canalização aberta, como o `AddSession` do original. Recusa com `ERR_PET_IS_NOT_EXIST` (72), `ERR_PET_IS_NOT_ACTIVE` (73) ou `ERR_PET_CAN_NOT_MOUNT` (81) — note que o original **não** recusa por mascote já ativo ao invocar (a linha está comentada em `player.cpp:14476`): quem já tem um troca de montaria. Quem entra no campo de visão **depois** também vê a montaria: o `mount_id`/`mount_color` vão no `object_state` do `info_player_1` (B80, spec 04). `falta`: mascote de **combate** (invocar a criatura), invisibilidade, transformação, a trava de ataque enquanto montado e a queda por lealdade. **Água (B88):** montaria terrestre **não entra na água** e **cai** se a água subir. Os dois limiares são os do `gplayer_imp::TestUnderWater` (`gs/player.cpp:14336-14342`), sobre `off = altura da água − y`: acima de **0,5 m** o jogador conta como submerso e a invocação é recusada com `ERR_PET_CAN_NOT_MOUNT` (`mount_petdata_imp::DoActivePet`, `gs/petman.cpp:344-348`); acima de **1 m** a montaria cai (`TestUnderWater`, `:402-410`). A conferência roda no batimento de 1 s, e a queda manda `PLAYER_MOUNTING(0,0)` **e** `RECALL_PET` — o original só limpa o estado interno, o que deixaria a jaula travada (B79). A altura da água vem do `watermap/` (spec 03 §3.6b) |
| amuleto e hierograma | `testado` (B67, B73) | `AUTOHP_ESSENCE`/`AUTOMP_ESSENCE`: o conteúdo do item são **8 bytes**, `int point; float trigger_percent` (`gs/item/item_amulet.h:16-19`, `generate_item_temp.h:2296-2310`). Sem eles o cliente desenhava zeros e negativos. **Disparo automático (B73)**: vesti-los nos slots **20** (vida) e **21** (mana) os ativa (`OnActivate` → `SetHPAutoGen`/`SetMPAutoGen`, `item_amulet.cpp:22-46`); a cada batimento de 1 s, com `trigger_percent × máximo > atual`, o `AutoGenStat` (`gs/player_imp.h:3562-3593`) confere a recarga (`COOLDOWN_INDEX_AUTO_HP` 24 / `AUTO_MP` 25), devolve `máximo − atual` preso ao que resta e arma o `cool_time` do item — e o `SetCoolDown` **sempre** manda `SET_COOLDOWN` (198) ao cliente (`gs/player.cpp:12701-12709`), que é o que escurece o ícone (B74). O que sobra fica nos **octetos do item**; em zero o amuleto some do corpo com `PLAYER_DROP_ITEM` tipo `DROP_TYPE_USE` (11) |
| colher recurso de mapa | `testado` (B51) | §7 "coleta de recurso" |
| Loja Gold (`MALL_SHOPPING` 106) | `testado` (B125) | Porte do `PlayerDoShopping` (1.5.5 `gs/player.cpp:15709-16010`; 1.2.6 `gs` VA 0x807f8e0), uma entrada por pedido: corpo errado → `ERR_FATAL_ERR` 3; morto → recusa calada; bolsa cheia → 7; `slot ≥ 4`, oferta inexistente ou de outro item, oferta de NPC, opção sem preço (as válidas são prefixo, `InitMall` `playermall.cpp:198`) → 94; VIP exigido → 226 (o `pw-gs` não tem VIP: nível 0); grupo/período (nenhuma oferta tem) → `MALL_ITEM_BUY_FAILED(index, 0)`; cash < preço → 16. O cash é da **conta** (`accounts.gold_balance`, débito atômico `gastar_cash_da_conta`; estorna se a bolsa encher no meio). Entrega com `empilhar_gerado` + `OBTAIN_ITEM(id, 0, n, no_slot, 0, slot)` (o `expire_date` do molde), brinde idem, e `PLAYER_CASH(saldo)`. `falta`: período de venda, limite de compras (`_purchase_limit_info`), `IsItemForbidShop`, validade do item (a bolsa não guarda), variante de venda dos equipamentos (`ADDON_LIST_SHOP`), `GET_MALL_ITEM_PRICE` (118, ainda no `gateway.rs`) |
| barraca | `falta` | |
| demais serviços de NPC (teleporte, pedras, forja, decompor, item de missão) | `falta` | |
| **armazém** (serviço 15, C2S 55–61) | `testado` (B147, 1.5.5 e 1.2.6) | `bus_server/armazem.rs`. Abrir (`trashbox_open_executor`, `serviceprovider.cpp:2026-2069`): `{u32 passwd_size; passwd}`; sessão em curso (golpe, produção, coleta, armazém) → `ERR_OTHER_SESSION_IN_EXECUTE` 33; senha não vazia → `ERR_PASSWD_NOT_MATCH` 35 (sem senha guardada, `CheckPassword` só aceita a vazia); responde `TRASHBOX_OPEN` com 16 slots (`TRASHBOX_BASE_SIZE`). Fechado ou `where`/conta fora → `ERR_TRASH_BOX_NOT_OPEN` 36; tamanho errado → `ERR_FATAL_ERR`. 55: a lista `IL_TRASH_BOX` (3), no 1.5.5 também `IL_TRASH_BOX2` (4) com 0 slots, `item_info` de cada um se `detail`, e `TRASHBOX_WEALTH`. 56/57 trocar/mover no armazém (`ExchangeItem`/`MoveItem`); 58 troca com a bolsa; 59/60 `MoveBetweenItemList` (junta até a pilha; `-1` → `ERR_FATAL_ERR`; índice fora do armazém em 60 retorna calado). 61 dinheiro: um dos dois valores e nunca mais do que há (`ERR_FATAL_ERR`); guardar corta em `TRASHBOX_MONEY_CAPACITY` 2e9; tirar além do teto da bolsa → `ERR_INVENTORY_IS_FULL` 7. Dinheiro do armazém em `characters.storehouse_money`. Andar, cancelar, atacar, conjurar, coletar ou sentar fecham (`TRASHBOX_CLOSE`, `session_use_trashbox::EndSession`). `falta`: senha do armazém (serviço 14), armazém da conta, armazém de materiais/roupas/cartas com slots, expansão, `_lock_inventory`, restrições de tipo do `VerifySpecTrashBox` (só valem nos armazéns especiais) |

### 8.3 Refino, pedras e furos — `testado` (B163, sem teste em jogo)

Serviços de NPC 10 (incrustar), 11 (remover as pedras), 35 (refinar) e 47 (furar), em
`bus_server/pedras_e_refino.rs`; a regra pura em `refino.rs`; os dados em
`pw-data-loader/src/refino.rs` (spec 03). Porte de `install/uninstall/refine_service/make_slot_executor`
(`serviceprovider.cpp`) e de `EmbedChipToEquipment`, `ClearEmbed`, `RefineItemAddon`,
`ItemMakeSlot`, `RefineAddon`, `MakeSlot`, `OnInsertChip`, `OnClearChips`, `AfterChipChanged`.

- **Ordem do original:** item no slot com o tipo dito (senão recusa calada) → NPC sem o serviço
  `ERR_SERVICE_UNAVILABLE` (14) → operação. O refino confere antes a recarga
  `COOLDOWN_INDEX_REFINE` (22, 1000 ms, `SET_COOLDOWN` ao cliente; erro 54).
- **Quem oferece:** 10 = `id_install_service`, 11 = `id_uninstall_service`, 35 =
  `combined_services & 0x4000`, 47 = todo NPC no 1.5.5 (`if(1 || ...)`).
- **Incrustar:** pedra de grau ≤ ao do equipamento e `IsStoneFit` com `combined_switch` 0 (a coluna
  não existe no v156 nem no v7): arma e armadura sim, acessório não. Sem dinheiro para o
  `install_price` → 16; o resto → 21. A pedra vai ao primeiro furo vazio, os addons dela entram com
  `0x8000`, a máscara das pedras é recalculada; `EMBED_ITEM` e `SPEND_MONEY`. Os addons vêm das duas
  listas gravadas na pedra ou, sem octetos (nossas pedras não têm), do `generate_addon` de
  `id_addon_damage`/`id_addon_defence`.
- **Remover:** soma o `uninstall_price` de cada pedra; sem pedra ou sem dinheiro, recusa calada;
  senão zera os furos, tira os addons `0x8000` e manda só o `CLEAR_TESSERA`.
- **Refinar:** `levelup_addon` e `material_need` do equipamento, Pedra Celestial 11208; talismã
  (`REFINE_TICKET_ESSENCE`) soma `ext_succeed_prob` à chance de sucesso, `ext_reserved_prob` à de
  cair um, e com `fail_reserve_level` troca a chance pela `fail_ext_succeed_prob[nível]` e faz a
  falha não mudar nada. Tabelas `refine_table`/`refine_factor` iguais nas duas versões. Resultado
  0/1/2/3 no `REFINE_RESULT`, item de volta (menos no 1), material e talismã saem com
  `DROP_TYPE_USE` (11). Qualquer recusa → 92 sem gastar nada. O cabeçalho e a essência do bloco
  ficam byte a byte (`ConteudoDeEquipamento::alterar_rabo`); `refine_level` do banco acompanha.
- **Furar (só 1.5.5):** arma até 2 furos, armadura até 4, pelas tabelas
  `weapon/armor_slot_material_count` com 21043 e depois 34232 (`DROP_TYPE_TASK`); sucesso responde
  `ERROR_MESSAGE` 107 e o item. Acessório pelo 47 sempre erra (106 ou 25), como no original.
- **1.2.6:** mesmos tamanhos, erros, comandos e tabelas; sem trava de segurança (não temos
  nenhuma), o talismã não confere `binding_only`/`require_level_max`
  (`WorldProtocol::talisma_confere_vinculo_e_grau`) e furar não existe (`furar_existe`, 14).
- **Falta:** furo de acessório de verdade (serviço 96, `make_slot_for_decoration`), transferir
  refino (45), trava de segurança, e gerar o conteúdo da pedra ao criá-la (`generate_stone`).

### 8.0 A barra de chi — `testado` (B69/B70/B73)

O chi (a "fúria" do original, `_basic.ap`) **não existe até uma missão dar o teto**: é o
prêmio `m_ulFuryULimit` (deslocamento 57 do `AWARD_DATA`) → `SetFuryUpperLimit` →
`gplayer_imp::SetMaxAP` (`gs/task/taskman.cpp:498-501`, `actobject.h:1634-1640`). No
`realm_155` são 8 missões, e a primeira é a 32394 "Só um Pouco de Progresso" (nível 9, teto
99); depois 199, 299 e 399 (`cargo run -p pw-gs --example missoes_de_chi`).

**1.2.6 (B119):** o mesmo prêmio, no deslocamento **40** do `AWARD_DATA` v55 — o
`DeliverByAwardData` do `libtask.so` 1.2.6 faz `if (award[0x28]) SetFuryUpperLimit(...)` pela
vtable (0xb4b3-0xb4d2). São 8 missões: 915/966/973 (99), 922 (199), 925 (299), 1888/2804/2818
(399). **Correção do B118**, que afirmou o contrário: a busca pela chamada virtual não viu o
`add` no ponteiro da vtable. Golpe normal: `angro_increase` (Feiticeira 4). Meditar no 1.2.6 não
dá chi (`WorldProtocol::chi_por_meditacao` = 0; o `Heartbeat` de lá não chama `ModifyAP`).

| ganho | quanto | origem |
| :--- | :--- | :--- |
| golpe normal | `ap_per_hit` da classe (`angro_increase` do `CHARRACTER_CLASS_CONFIG`; Arqueiro: 5) | `gplayer_imp::DoAttack`, `player.cpp:3091-3093` |
| meditar | **15 por batimento de 1 s** no 1.5.5; 0 no 1.2.6 (`WorldProtocol::chi_por_meditacao`) | `sit_down_filter::Heartbeat`, `gs/sitdown_filter.cpp:19-34` |
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

### 8.2 Mascote de combate — `testado` (B111, sem teste em jogo)

Porte de `gs/petman.cpp` (`combat_petdata_imp`, `pet_manager`) e `gs/petnpc.cpp` (`gpet_imp`,
`gpet_policy`) em `crates/pw-gs/src/mascote.rs`, `world.rs` e `bus_server/mascote.rs`. Vale
para o 1.2.6 e o 1.5.5; só os layouts mudam (spec 04).

| parte | estado | detalhe |
| :--- | :--- | :--- |
| no terreno (B138) | `testado` | **Erro de porte do B133 corrigido:** o `IsPosPassable` do agente espacial com posição real testa o ambiente na **posição** e só o nó da octree no centro do voxel (`GlobalSPMap.h:187-204`); o nosso testava no centro do voxel de 2 m — rente ao chão o centro caía abaixo do terreno e toda perseguição falhava na partida (valia para monstro de ar também). A partida da busca não tem o teste extra (`SpatialPathFinding.cpp:47-50`). **Diferença do original, só no mascote (pedida pelo Murillo):** o passo reto que entraria no terreno sobe para terreno + 0,2 em vez de bloquear, e o destino sobe o bastante para a reta desde o ponto anterior passar acima do chão (o cliente desenha o NPC de ar em reta, `EC_NPC.cpp:1000-1020`). No original o mascote parava na crista quando o alvo descia a encosta. Medido no mapa 1 do 1.2.6 (`tests/mascote_no_terreno.rs`, Vespão Pequeno contra monstro que foge em 40 encostas): antes 39 travadas; depois 0 travadas, 0 abaixo do chão, 0 trechos pelo chão |
| golpe e sessão (B136) | `testado` | o golpe começa a `(attack_range − corpo) × 0,8 + corpo + corpo do alvo` e a perseguição mira `× 0,6` (`ai_melee_task::Execute`, `aipolicy.cpp:597-609`); batendo, continua até `attack_range + corpo do alvo` (`CheckAttack`, `actobject.cpp:1280-1287`). Antes media só o `attack_range` (0,9 dele) sem o corpo do alvo. Cada tarefa (golpe, habilidade, seguir o dono, ficar no ponto) começa **agente novo** (`session_npc_follow_target` nova): o mascote reaproveitava o agente anterior, e depois de matar, com o dono voando por cima do morto, ia primeiro aos pés do morto. A contagem de perseguições da habilidade (`_trace_count`) vale também para o de ar. Sem porte: o `session_npc_keep_out` de quem fica colado |
| alcance e altura (B135) | `testado` | o golpe e a habilidade do mascote medem a distância em **3D** (`squared_distance`, `aipolicy.cpp:579`, `:1999-2027`); seguindo o dono, o mascote que não é só de chão (`inhabit_type != 0`) mira **1,5 m acima** dele (`ai_pet_follow_master::Execute`, `aipolicy.cpp:1827-1835`; `info.pos.y += _height_offset`, `npcsession.cpp:197`) — sem isso o de ar descia aos pés do dono e ficava enterrado. O "Atq" da janela do mascote é o **dano** do `GenerateBaseProp` (Vespão Pequeno nv 30: 466; Filhote de Lobo Feroz nv 30: 332) |
| habitat (B131) | `testado` | `inhabit_type` do `PET_ESSENCE` (0 chão, 1 água, 2 ar, 3 chão+água, 4 chão+ar, 5 água+ar, 6 todos; o 1.2.6 só tem 0..=2). **Lugar** (`pet_gen_pos::FindValidPos`, `petman.cpp:127-286`; o 1.2.6 tem o mesmo `combat_petdata_imp::FindValidPos`/`FindAirPos` com os mesmos 1,5 m): a camada do dono decide se aparece e a ordem (o de mais de um ambiente tenta primeiro o do dono); chão = `FindGroundPos`, ar = dono + 1 m e ≥ terreno/água + 1,5, água = entre terreno + 1 e água − 1. **Passo**: o de ar/água vai em reta 3D até o dono (`CNPCChaseOnAirStraightAgent`), sem descer abaixo do piso, com a marca 0x40/0x80; o de chão usa o agente de chão. **Entrada** (`NPC_ENTER_*`): `GP_STATE_NPC_FLY` 0x10000 / `SWIM` 0x20000 no `state` (`SetInhabitMode`, `npc.cpp:823-843`; o cliente põe em `MOVEENV_AIR`, `EC_NPC.cpp:411-416`), sem bytes a mais nas duas versões. **Dono muda de camada** (`TryChangeInhabitMode`, `petnpc.cpp:313-355`): o de mais de um ambiente troca de modo com `stop_move`, ou reposiciona a mais de 10 m. Desde o B133: o `IsValidSPPos` no lugar de ar/água (folha livre do `airmap/`), o passo de ar/água pelo `SeguirNoEspaco` (a busca na octree) e o `nofly` |
| capturar (Domesticar Animal, 328) (B130) | `testado` | `PlayerWrapper::SetEntrap` (`playerwrapper.cpp:2460-2479`; igual no `gs` 1.2.6, VA 0x8305982): monstro sem `id_pet_egg_captured` ou conjurador de nível **abaixo** do alvo → `MOD_IMMUNE` 0x80; senão chance `((máx − vida)/máx)² × 100 × (1,35 − nível/100 [inteira] + nível_da_habilidade × 0,05)` — acertou: `MOD_SUCCESS` 0x200, o ovo vai ao conjurador (`GM_MSG_MOB_BE_TRAINED` → `get_item_for_sell(ovo)` no primeiro vazio + `obtain_item`, bolsa cheia = `ERR_INVENTORY_IS_FULL` e o ovo se perde, `player.cpp:2198-2225`) e o monstro **some** sem experiência nem drop e volta ao gerador (`OI_Disappear`, `npc.cpp:2532-2551`); errou: `MOD_ENCHANT_FAILED` 0x100. A marca vai no `ENCHANT_RESULT` (`skillwrapper.cpp:552-556`): no 1.2.6 em dois `char`, `immune & 0xff` e `immune >> 8` (VA 0x831bb88). `Entrap2` usa a `probability` do roteiro. Gato de Presas Afiadas (3316) → ovo 10765 nos dois realms |
| invocar | `testado` | a mesma sessão de 60 ticks da montaria; ao fim, `ActivePet` escolhe pela **classe** do modelo (`PET_ESSENCE.id_type` 8782 = combate). Recusa nível de mascote > dono + 35 (`ERR_LEVEL_NOT_MATCH` 51) e morto (`hp_factor` 0, erro 87). Recolhe o anterior. A criatura nasce junto do dono com id `0x80000000 \| 0x20000000 \| n` (`PET_MASK`, `common/types.h:214`); dono recebe `SUMMON_PET` com o id, `PET_AI_STATE` e `PET_HP_NOTIFY`; quem está perto, `NPC_ENTER_WORLD` com a marca 0x1000 + dono (e 0x2000 + nome). Antes do B111 o de combate caía no caminho da montaria e "montava" |
| atributos | `testado` | `GenerateBaseProp` (`petdataman.cpp:152-186`, fórmulas `petdataman.h:23-76`) no nível dele; vida × `hp_factor`; alcance + `size`; dano com a lealdade (−40/−20/0/+20% por nível de lealdade 0..3, `pet_filter.cpp:8`); sem mana; regenera `hp_gen` por segundo **também em combate** (`SetFastRegen(0)`: `if(_combat_state || !_fast_regen) GenHPandMP(hp_gen)`, `npc.cpp:1948-1951`). Conferido no B120 com o mascote da Tsuko (10386, nível 22): vida 654, `hp_gen` 17/s, defesa 1428 (`petdataman.h:14-26`) — contra atacante de nível 22 passa 37,5% do golpe, e um monstro do mesmo nível tira menos do que ele regenera. É o original |
| IA | `testado` (B114) | `gpet_policy::OnHeartbeat` (`petnpc.cpp:1630-1735`): o seguir começa **só no batimento de 1 s** com o dono a mais de 1,5 m (ou 10 m de altura) e é uma sessão (`session_npc_follow_target`, `npcsession.cpp:164-280`, meta 1,0 m) que acaba com parada abaixo de 0,8 m (3D). Ao alcançar a meta antiga o agente recomeça a `0,6 ×` **sem parar**; a parada só sai quando o recomeço já nasce na meta, o passo não sai do lugar ou o caminho falha. O cliente só reinicia a animação/som de andar quando o NPC sai de `WORK_MOVE` (`EC_NPC.cpp:1048-1053`) — parar a cada passo fazia o som do mascote recomeçar. Com ódio, persegue e bate no ritmo do `attack_speed` com o atraso do `damage_delay`. Cercas: longe (≥ 60 m, altura > 60 m, > 150 m, 5 falhas de caminho) reposiciona; parado ("ficar") e longe, é recolhido. **Reposicionar e invocar usam `pet_gen_pos::FindGroundPos`** (`petman.cpp:1-31`, `:595`, `:701-718`; igual no `gs` 1.2.6): 10 sorteios a ±0,8–1,2 m do dono, pixel alcançável do `movemap` (terreno + piso) e < 6,8 m da altura dele; sem ponto, a invocação dá 85 e o reposicionamento **recolhe**. A plataforma do Ancião da Cidade das Feras (2206, −1538/970) não está no `movemap` de nenhum dos dois realms (0 de 200 sorteios): antes o mascote ia para a posição crua do dono e o passo seguinte o assentava no terreno, 5 m abaixo, dentro da estrutura |
| agressividade | `testado` | começa automático + seguir (`petman.cpp:1283-1284`) e o dono guarda o último. Defesa: ódio por apanhar e quando o dono apanha (`MASTER_ASK_HELP`, 2). Automático: ataca o que o dono começar a atacar, se estiver sem ódio (`Notify_StartAttack` no golpe normal e na habilidade, `actsession.cpp:361`/`:491`). Passivo: só por ordem |
| comandos (`PET_CTRL` C2S 103) | `testado` | `{int target; int pet_cmd; buf}`: 1 atacar (limpa o ódio, `max_hp + 10` no alvo), 2 seguir/ficar, 3 agressividade (automático e passivo congelam o ódio); `PET_AI_STATE` quando muda. 4 e 5: habilidades (linha abaixo) |
| habilidades | `testado` (B112) | `pet_data.skills` (até 8, parando no primeiro vazio). **4** (`petnpc.cpp:1065-1112`): nível > 0; fora das áreas 2/5 exige alvo, que não pode ser o mascote nem o dono; ataque/maldição limpa o ódio e põe `max_hp + 10` no alvo; bênção em alvo (tipo 2) não tem porte (nenhuma de mascote de combate é assim). **5**: automática (área 5 recusada; 0 desliga). `ai_pet_skill_task`: persegue até 0,9 × (`GetPraydistance` + raio do corpo; o do alvo não entra) e conjura, ou conjura de onde está depois de 2 perseguições. `session_npc_skill`: recarga armada → erro 93 ao dono; mana (0 em todas as de mascote dos dois catálogos; o de combate tem `max_mp` 0) ; `OBJECT_CAST_SKILL` a quem vê com o canto do `State1`; ao fim do canto a recarga (`id + 1024`, segundos truncados) e `PET_SET_COOLDOWN` ao dono, e o efeito; `State2` de execução ocupa o mascote. Automática: em combate, sem sessão, recarga pronta → tarefa no primeiro do ódio (`OnHeartbeat`/`DeterminePolicy`). Efeito (`aplicar_habilidade_do_mascote`, motor de roteiros com o mascote como conjurador): `dobless` no mascote; ataque com `bruto × (100 + lealdade + ratio×100)/100 + plus` (`actobject.h:1422`), precisão × `GetHitrate`, dano pelo caminho do golpe (crédito do dono), `OBJECT_SKILL_ATTACK_RESULT` do mascote a quem vê, e o `StateAttack` se acertou (sangramento da 747); maldição no monstro e `TYPE_BLESSPET` (10) no próprio mascote, com `ENCHANT_RESULT`. O dano no tempo posto pelo mascote credita o dono; os filtros do corpo do mascote têm batimento. 747 nível 1: 400 ms de canto, 15 s de recarga; no 1.2.6 sem `SetRatio` (`gs` `Skill747Stub::State2::Calculate`), no 1.5.5 ratio 0,3 |
| bênção do dono no mascote | `testado` (B114) | a Curar Mascote (330, `TYPE_BLESSPET` de ponto) aceita o mascote como alvo (`playerwrapper.cpp:398`): `ENCHANT_RESULT`. **1.5.5:** `Heal` (`S_Magicdamage × 0,3 + 540`), `Decregiondmg` e `Rebirth` por 30 s (B115). **1.2.6:** só `Heal`, `55·L − 10 + S_Magicdamage × (0,02·L + 0,1)` — o `gs` 1.2.6 não tem os dois filtros (`Skill330Stub::StateAttack`, VA 0x8382482) |
| aprender / esquecer (NPC 38 / 37) | `testado` (B112) | exigem o mascote **ativo** (73). 38: habilidade da lista do NPC (senão 20); `OnLearnSkill` recusa a 5ª normal; `PetLearn`: nível atual + 1 ≤ máximo, `cls` 127, pré-requisitos, nível do **mascote** ≥ `GetRequiredLevel`, SP do dono ≥ `GetRequiredSp` (sai com `COST_SKILL_POINT`), o livro `GetRequiredItem` sai da bolsa (`DROP_TYPE_TAKEOUT`); recusa = 14. 37: tira e sobe as seguintes (20 se não tem); dinheiro/item do serviço. Os dois: lista nova no corpo (a automática acompanha), `PET_ROOM` do slot, jaula gravada |
| mascote ornamental (B154) | `testado` nas duas versões | "Ver Mascote" (`PET_ESSENCE.id_type` **8783** → `PET_CLASS_FOLLOW`, `petdataman.cpp:24-33`; `gs` 1.2.6 VA 0x814364b): 8781 montaria, 8782 combate, 8783 ornamental. Os do `realm_126` como o Falcão do Paraíso (12340) e o Filhote de Prata (12339) têm só vida e velocidade no arquivo — **sem atributos de combate**, como no original. Invoca-se como o de combate (`follow_petdata_imp : combat_petdata_imp`, `petman.cpp:1006-1077`, `DoActivePet` herdado) e `Mascote::ornamental` desliga o resto: comandos (`OnPetCtrl` false), experiência por abate, ajuda ao dono e ataque automático, aprender (14) e esquecer (20) habilidade (`petman.cpp:1938-1972`); no mundo (`gpet_imp_2`, `petnpc.cpp:1819-1856`) não apanha, não recebe bênção nem maldição e não entra na mira dos monstros (`PeepEnemy` vazio). Até o B154 a classe 2 caía em "sem porte" com o erro 81 |
| jaula: vagas (B153) | `testado` nas duas versões | `pet_manager::_active_pet_slot`: começa em **1** (`petman.cpp:1276`), gravada em `characters.pet_slots` (o `pets.capacity` de `userlogin.cpp:133, 782`) e mandada no login com `PET_ROOM_CAPACITY` (240, `player.cpp:13731`). Só cresce, até 20 (`SetAvailPetSlot`, `petman.h:170-176`), pelo prêmio de missão `m_ulPetInventorySize` (`TaskProcess.cpp:1292` → `SetPetSlotCapacity`, que manda o 240). Série da Gerente de Mascotes (NPC 9762): 3327–3330 = 2/3/4/5 vagas, 8986–8990 = 6…10; da Domesticadora Rilay (11534): 5933–5936 e 8981–8985. Incubar (serviço 28) vai ao **primeiro slot livre abaixo das vagas** (`AddPetData`, `petman.cpp:1487-1503`); sem nenhum, 76 e o ovo fica (aqui a conferência vem antes do dinheiro; no original, depois). Até o B153 a capacidade era inventada (mascotes + 1); a migração `scripts/2026_09_29_vagas_da_jaula.sql` começou cada personagem com `max(1, último slot + 1)` |
| restaurar mascote em ovo (serviço 29) (B153) | `testado` nas duas versões | `restore_pet_service_executor` (`serviceprovider.cpp:3234-3272`, pedido `{size_t pet_index}` 4 B) → `ServiceConvertPetToEgg` (`player.cpp:14514-14537`): bolsa cheia 7, inexistente 72, ativo 71; `session_restore_pet` operação **3**, **200** tiques (`PLAYER_START_PET_OP`). Ao fim, `ConvertPetToEgg` (`player.cpp:14583-14670`): confere de novo, `PET_EGG_ESSENCE` do `pet_egg_tid` (senão 77), `money_restored` (16), o ovo do modelo com os dados do mascote (`ConvertPetDataToEggData`, `player.cpp:14360-14415` → `pw_core::ovo_do_mascote`), `OBTAIN_ITEM`, `SPEND_MONEY`, `FREE_PET` e o slot sai da jaula. `gs` 1.2.6 igual (VA 0x807e4b2, 0x807e7ee, 0x807df12; serviço 29 em 0x8105409). Falta: vínculo (`proc_type`) do ovo, que a bolsa não guarda |
| soltar (`BANISH_PET` C2S 102) | `testado` (B112) | inexistente 72, ativo (combate ou montaria no slot) 71; senão `PLAYER_START_PET_OP` operação 2 com 200 tiques, e ao fim (conferindo de novo) `FREE_PET` (232) e o slot sai do `PetCorral` (`player.cpp:14539-14557`, `petman.cpp:1521-1538`) |
| renomear (NPC 36) | `testado` (B112) | `{u16 idx, u16 len, name}` com 2..16 bytes pares; dinheiro (16) e item do serviço (5); mascote inexistente 72 e **ativo 71** (`petman.cpp:1921-1935`) — por isso ninguém vê o nome mudar numa criatura no mundo: ele aparece na próxima invocação (bit 0x2000 da entrada). Grava, `PET_ROOM` do slot, cobra |
| combate | `testado` | o golpe leva o **dono** como atacante (`FillAttackMsg`): crédito do abate, experiência e missão são do jogador; o monstro odeia o mascote (e 1 no dono). Os monstros atacam mascote como atacam jogador (e o agressivo o vê, `PeepEnemy`). `OBJECT_ATTACK_RESULT` (120) mostra os golpes entre criaturas a quem vê |
| aviso ao dono | `testado` | `PET_HP_NOTIFY` a cada 5 s ou quando vida/combate mudam; mascote em combate põe o dono em combate por 6 s (`OnPetNotifyHP`) |
| experiência | `testado` | no abate creditado ao dono (`KillMob`): 10 − (nível do mascote − nível do monstro, se menor) × ajuste da lealdade (0,1/0,5/1,0/1,5); curva `PLAYER_LEVELEXP_CONFIG` 592 (o `gs` 1.2.6 também, VA 0x80e6fae — lá começa em 3, 3, 3, 3, 5); trava no nível do dono e no `level_max`. `PET_RECEIVE_EXP` ou `PET_LEVELUP` (atributos novos, vida cheia) |
| morte | `testado` | `RECALL_PET` com `PET_DEATH` (1), `PET_DEAD`, lealdade −10% (`PET_HONOR_POINT`), `hp_factor` 0 na jaula |
| recolher | `testado` | `RECALL_PET` (sessão de 10 ticks); também quando o dono morre (`OnDeath`) ou sai |
| fome e lealdade | `testado` (unidade) | a cada 300 s de mascote ativo a fome sobe 1 e a lealdade cai pela tabela `__pet_feed_param_list` (`petman.cpp:1645-1711`); comida (`PET_FOOD_ESSENCE`, `hornor`/`food_type`, recarga 18 de 60 s) exige o tipo no `food_mask` (erro 74), sobe a lealdade × fator e baixa a fome; `PET_HONOR_POINT` + `PET_HUNGER_GAUGE` |
| reviver | `testado` (compila; sem teste de ponta) | habilidade 329 (efeito `SetSummon`): o primeiro morto da jaula volta com `hp_factor` 0,1 e `PET_REVIVE`; sem morto, erro 88 |
| gravar | `testado` | nível, experiência, vida, lealdade e fome voltam ao bloco `pet_data` do slot do `PetCorral` a cada mudança |
| gravar o bloco inteiro | `testado` (B112) | até o B112 o `InfoPet::do_bloco` lia 40 dos 192 B e cada gravação da jaula zerava nome e habilidades |
| atordoado, preso, selado (B140) | `testado` | ver §6 (atordoado/dormindo) |
| dano no tempo (B141) | `testado` | veneno/sangramento de monstro no mascote: a mesma conta do jogador→monstro (`SetToxic`/`SetBleeding`: `CalcMagic/PhysicDamage` com a defesa do mascote e a punição pelo nível de quem lança, só acima de 3 — `playerwrapper.cpp:1259-1295`); o tique tira vida e **não** dá ódio (`BeHurt` → `OnHurt`), nem o `Directhurt`; o estado e os ícones vão a quem vê (B140). Ex.: a 25 do Predador Venenoso (1114, dano mágico 248–303), `Toxic` 100 % por 15 s |
| invisibilidade, água/ar, bênção de mascote em outro alvo | `falta` | |

## 9. Persistência

- Consulta administrativa não grava. B170: queda do barramento salva/retira a entidade;
  propriedade da conexão impede limpeza antiga/duplicata de encerrar outra sessão.
  Saída não confirmada conserva fotografia fora da simulação, sessão/rota reservadas e
  `em_transicao`; reentrada bloqueada e repetição a cada 1 s. Confirmação libera e responde.
- **Autosave a cada 60 s:** fotografia única de status, chi, atributos/pontos, modo roupa,
  waypoints e listas de missão; gravação atômica no CharacterRepository. Controle/carimbo
  compartilhados entre mapas rejeitam fotografias anteriores a comandos/saída/troca/entrada.
  Banco fora do tick/world lock. Estado de missão alterado/modo roupa deixam de lançar fotografia atrasada; contexto
  sem alteração não grava nem atrasa a resposta. Pares opacos de charactermode preservados.
- **Transferência B170:** destino só publicado após fotografia confirmada; erro antes
  do commit restaura origem, confirmação incerta conserva reserva e repete destino.
  Queda/logout durante recuperação termina com saída salva, sem sessão fantasma.
- **Parcial:** fencing/revisão persistidos e recuperação após queda do GS, eventos/equipes,
  itens/habilidades/mascotes/durabilidade/munição/configuração do link ainda exigem
  coordenação de todos os produtores. Sem autorização de edição offline/ban/desconexão.
  Contrato em docs/admin/ARQUITETURA_E_CONTRATOS.md; B170: workspace 942/4 (2 ignorados),
  correções verificadas no GS 261/0 (2 ignorados) e canal final 15/0.
- Movimento **não** grava por pacote.
- Operações de `com_contexto` (missão, abate, coleta, loja, aprender) gravam na hora: bolsas
  antes de responder, estado e listas numa tarefa.
- Listas de missão: `character_task_lists` (cinco `BYTEA`, os blocos do `TASK_DATA`,
  `scripts/2026_09_14_listas_de_missao.sql`); lidas no `EnterWorld` pelo mundo e pelo link.
- Personagem novo: `class_templates` do realm (posição, kit, arma) + atributos 5/5/5/5 com a
  vida e a mana do `clsconfig` (spec 02 §4).
- Barras de atalho, layout e opções do cliente: `character_client_config` (spec 02 §4). Quem
  não tem gravado recebe o `ui_config` do molde da classe (o `config_data` do `clsconfig`); no
  `realm_126` esses moldes estavam vazios até o B102 e o personagem novo nascia com as barras
  vazias (`scripts/2026_09_24_moldes_do_clsconfig_126.sql`).
- 1.2.6 (B102): nascimento junto ao Guia da raça, pelo `clsconfig` 1.2.6 (spec 03 §3.10).

## 10. Missões (`missoes.rs`) — `testado` (sem teste em jogo)

**O cliente refaz cada operação na cópia dele** a partir dos avisos (`OnServerNotify`,
`TaskProcess.cpp:2643-2815`), então as listas do servidor são as estruturas binárias do
original, mexidas pelas mesmas funções portadas linha a linha: `DeliverTask`, `RealignTask`,
`RecursiveClearTask`, `RecursiveAward`, `FinishedTaskList::AddOneTask`
(`task/TaskProcess.cpp`, `TaskProcess.h:103-392`).

| lista | bytes no `TASK_DATA` | notas |
| :--- | :--- | :--- |
| ativa | `8 + 32 × n`: `count u8, used u8, version u16 = 1, top_show u8, state u8 (1 = tempos absolutos), top_hide u8, maxsim:1\|title:7`; entrada `id u16, pai, anterior, próximo, filho, estado u8, tempo u32, capitão u16, templ u32, cap u32, buf[11]` (os três `m_wMonsterNum`) | **versão ≠ 1 faz o cliente descartar todo aviso** (`TaskClient.cpp:262`) — a causa de "aceitar não mostra nada" (B50) |
| concluídas | `4 + 4 × n`, ordenada por id (`id u16, falhou:1, vezes u8`) | **1.2.6** (B102): formato antigo `FnshedTaskListOld` — versão 0 e `u16` por entrada, falha no bit 15 (captura: `01 00 00 00 e8 06`); o `WorldProtocol` v126 converte ao mandar o `TASK_DATA` (só 3 blocos: ativa, concluídas, tempos) |
| tempos / contagens | `2 + 6 × n` / `2 + 14 × n` | frequência diária/semanal e limites por conta/personagem |
| depósito | 864 bytes zerados | depósito de missões `falta` |

| operação | origem | estado |
| :--- | :--- | :--- |
| aceitar no NPC (`GP_NPCSEV_TASK_ACCEPT`) | NPC em conversa (`SEVNPC_HELLO`) com a missão em `NPC_TASK_OUT_SERVICE` (`serviceprovider.cpp:1088`); submissão vira escolha da mãe (`OnTaskCheckDeliver`); `CheckPrerequisite` na ordem do original; `svr_new_task` (17 bytes + tags) | `testado` |
| entregar no NPC (`GP_NPCSEV_TASK_RETURN`) | missão em `NPC_TASK_IN_SERVICE`; `OnTaskCheckAward` por método; `DeliverAward` → `RecursiveCheckAward` → `RecursiveAward` → `DeliverByAwardData` (ouro, exp, SP, reputação, itens por grupo/escolha, missão nova, coeficiente de nível `_lev_co`); `svr_task_complete` com o estado | `testado` |
| abate (`OnTaskKillMonster`) | dono do abate; `CheckKillMonster`: conta (`svr_monster_killed`, 17 bytes; **9 no 1.2.6**, B101) ou sorteia o item de missão; completa → `OnSetFinished` (conclusão direta premia) | `testado` |
| **morte do jogador** (`OnTaskPlayerKilled`, B157) | `OnDeath` (`gs/player.cpp:7295-7297`) → `TaskServer.cpp:1125-1154`: cada missão ativa com sucesso e `m_bFailAsPlayerDie` perde o sucesso e é finalizada (`OnSetFinished`); depois `CheckDeathTrig` (`TaskTemplMan.cpp:271-281`) tenta entregar, sem aviso de erro, as de topo com `m_bDeathTrig`. No 1.2.6 a única é a **990 "A Divina de Hades Vazia"** (cultivo 39: pré-requisito 923, sem NPC, sem entrega automática, sem conversa que a ofereça) — o jogador precisa **morrer** depois da 923; no 1.5.5, a 990 e a 29649. Chamado em `EventoDoMundo::JogadorMorreu`, depois do `PLAYER_DIED` | `testado` (falta ver em jogo) |
| `TASK_NOTIFY` 1/2/3/4/5/10 | concluir (`OnTaskCheckAwardDirect`), desistir, **chegou ao lugar** e **saiu do lugar** (`OnTaskReachSite`/`LeaveSite`, `TaskServer.cpp:466-520`: confere mundo e caixa, `OnSetFinished`), entrega automática, gatilho manual; 7 = marca dinâmica (B49) | `testado` |
| horário (B51) | `CheckTimetable`/`judge_time_date` (`TaskTempl.h:1697`) na hora local do contêiner (`TZ=America/Sao_Paulo`): por data, mês, semana (1 = segunda … 7 = domingo) e dia; basta uma janela | `testado` |
| região de entrega (B51) | `CheckInZone` (`TaskTempl.inl:368`): mundo `m_ulDelvWorld` e alguma caixa `m_pDelvRegion` (bordas incluídas) | `testado` |
| facção (B51) | `CheckFaction` (`TaskTempl.inl:718`): em facção (`id_mafia != 0`) com cargo ≤ `m_iPremise_FactionRole`. **Não há sistema de facção**, então quem pede facção é recusado — como no original para quem não tem | `testado` |
| equipe (B51) | `CheckTeamTask`/`HasAllTeamMemsWanted` (`TaskTempl.inl:149-339`): só o capitão recebe; distância dos membros, `TEAM_MEM_WANTED` (nível, raça/classe, gênero, contagem), classes distintas; casal recusa (sem casamento). Aceita, cada membro **deste mapa** recebe por `OnDeliverTeamMemTask` (`TaskProcess.cpp:1592`) | `testado` |
| abate em equipe (B164) | `CheckKillMonster` com `bTeam` (`TaskTempl.inl:1985-1987`, igual no `libtask.so` 1.2.6): em equipe, a missão de equipe (própria ou do topo) só conta o abate que chega com a experiência do grupo (`OnTaskTeamKillMonster`, com o modelo só para o grupo de maior dano — os outros chegam com monstro 0); a comum, só o abate do dono (`OnTaskKillMonster`) | `testado` |
| sucesso/falha da equipe (B164) | `AwardNotifyTeamMem` (`TaskProcess.cpp:2030-2084`): na falha com `m_bAllFail` (ou capitão com `m_bCapFail`) e no sucesso com `m_bAllSucc` (ou capitão com `m_bCapSucc`, membros a `m_fSuccDist`), uma vez por entrada (`TASK_STATE_AWARD_NOTIFY_TEAM` 0x10); o membro recebe `OnTaskForceFail/Succ` e finaliza sem reavisar. Avisos entre jogadores saem pela fila do `Contexto` depois de gravar. O v55 não tem `m_bAllSucc`. Falta o caso da filha de topo de equipe (`m_ChildIndex == 0xff`) | `testado` |
| item de missão pelo NPC (serviço 8, B164) | `task_matter_provider` (`{int task_id}`, lista do `NPC_TASK_MATTER_SERVICE`, senão 19) → `OnNPCDeliverTaskItem` (`TaskServer.cpp:706-760`): com a missão ativa e espaço para todos os tipos na bolsa de missão, cada item que falta vem com a quantidade da tabela | `testado` |
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
(lidas como 0), prêmio por escala de tempo/itens (vazio), `FORCE_GIVEUP`, limite de tempo
checado só na entrega, força (`m_iForce`), região de entrar/sair que falha a missão, sistema de
facção. (Invocação de monstros de prêmio, falha por morte — B157 — e o que é de equipe — B164 —
saíram desta lista.)
