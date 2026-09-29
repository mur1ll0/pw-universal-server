# Especificação 03: Arquivos de dados do realm (`pw-data-loader`)

> Roteiros e passivas do 1.2.6, campos do v55 e prefixos do v7 em 2026-09-26, base `ff778c1` + B122. Serviços de mascote e `item_exigido` em 2026-09-25, base `ca082f8` + B112. Verificada contra o código/evidência em 2026-09-23, base `b16f992` + B94/B96. Cobre
> `crates/pw-data-loader/`, `specs/elements_layouts/`, `specs/elements_155/`, `specs/mapas/`,
> `specs/clsconfig_155/` e o conteúdo de `data/realm_*`.

## 1. Regras de leitura

1. **O carregador do cliente (ou do servidor original) é o juiz**, e o **tamanho do arquivo
   fecha a conta**: todo leitor que tem como saber onde o arquivo termina recusa o arquivo
   que não termina no último byte. Nada de busca por "plausibilidade", nada de override por
   arquivo (B45, B46).
2. O fonte é ponto de partida, não verdade: os `.data` do 1.5.5 são **mais novos** que o
   fonte 1.5.5 que temos. Campos novos saem do próprio arquivo (histograma, fechamento de
   deslocamentos) e seus nomes do fonte **1.7.2**.
3. Versão sem layout medido: lê só o cabeçalho e avisa. Não se adivinha layout.
4. `.conf` do original está em **GBK**, não UTF-8.
5. Dado de jogo sai daqui; tabela no código só com teste que a confere contra o arquivo.

## 2. Carga (`GameDataManager::load_from_directory`, `manager.rs`)

Uma pasta de realm (`CONFIG_DIR`) tem os `.data` na raiz e uma pasta por mapa:
`world/` = mundo 1, `aNN/` = mundo `100 + NN` (`a01..a99`), `bNN/`. Cada `pw-gs` carrega os
dados comuns uma vez e **o terreno só dos mapas que serve** (`WORLD_TAGS`). A carga devolve um
`RelatorioDeCarga` com lidos e falhas; falha de um arquivo não derruba o login (A60).

**Catálogo do realm (B102).** Ao lado de `config/`, a pasta `data/<realm>/catalogo/` pode
trazer `elements_layout.json` (formato de `specs/elements_layouts/vNNN.json`) e
`habilidades.json` (formato de `specs/habilidades_155/habilidades.json`). O que estiver lá vale
no lugar do embutido da versão; o layout tem de declarar a mesma versão do cabeçalho do
`elements.data`, senão é falha registrada (`LayoutDeOutraVersao`). Sem catálogo, os embutidos
de hoje (v7, v156, v159; habilidades 1.2.6 e 1.5.5). `data/` está no `.gitignore`: os geradores
ficam em `specs/`. `tests/carga_dos_realms.rs` confere o catálogo e que cada realm carrega.

**O que ainda exige código numa versão nova** (conferido em B102): o **formato binário do
`tasks.data`** (um leitor por `_task_templ_cur_version`: v55 e v129, `tasks.rs`); o
**protocolo do cliente** (`WorldProtocol` por versão, `pw-protocol/src/versions/`); as seções
do `ptemplate.conf` (dois conjuntos, escolhidos pelo que o arquivo tem); e o `generate_vNNN.py`
que produz o layout do `elements.data`. O resto — `npcgen`, `aipolicy`, `gshop`, `.sev`,
`.hmap`, `dyn_tasks`, moldes do `clsconfig` — é lido pelo mesmo código em qualquer versão.

**Estado da carga (B102):** `realm_126` carrega **sem falha** (132 arquivos). Não são lidos,
em nenhum realm, por falta do sistema: `domain.data`,
`extra_drops.sev`, `task_npc.data`, `global_api.lua`, `ExtDataID.dat`, `precinct.clt`,
`rare_item.conf`. Na raiz do `realm_126`, `npcgen.data`/`precinct.sev` são cópias idênticas das
de `world/` (as lidas); o `region.sev` da raiz (6.856 B) difere do de `world/` (8.860 B), e vale o
de `world/`. `realm_155`: 3 falhas dos próprios arquivos — `a46/npcgen.data` e `a50/precinct.sev`
terminam antes do que declaram.

## 3. Os arquivos

### 3.1 `elements.data` — itens, criaturas, classes, configurações

| | |
| :--- | :--- |
| leitor | `generic_elements.rs` (Rust) e `specs/elements_layouts/pw_elements_reader.py` (painel) — mesmo algoritmo |
| cabeçalho | `u32 versão` (`0x30000007` = v7, `0x3000009c` = v156, `0x3000009f` = v159); `time_t` só nos builds recentes medidos |
| corpo | cada tabela: `u32 count` + `count × sizeof(T)`, na ordem do `elementdataman::load_data` do cliente (`EvolvedPWClient/.../elementdataman.cpp:3879`) |
| exceções | v156/v159: depois de `ARMORRUNE_ESSENCE`, `tag 0xab7689dd`, `len`, bytes da máquina exportadora e `time_t`; depois de `WAR_TANKCALLIN_ESSENCE`, `tag 0xee35679f`, `len`, bytes. v7: sem esses blocos. Em todos: `TALK_PROC` com laço próprio |
| layouts | `specs/elements_layouts/v7.json` (gerado por `generate_v7.py` a partir do `gs` 1.2.6), `v156.json`, `v159.json` (ADMVAL); embutidos com `include_str!` |
| estado | **v7: 119 entradas, 23.447 registros, 16.664.770 bytes**, fecha no último byte no Rust e Python; v156: 231/231 tabelas no `realm_155`; v159: 234/234 quando ainda estava no projeto (B55). Arquivo com byte extra é recusado (`NaoTerminaNoFim`) |
| v7 | **Ordem, nomes e `sizeof` das 118 tabelas fixas: do `gs` 1.2.6** (`files1.2.6/pwserver/gamed/gs`, ELF com símbolos, que exige `0x30000007` em `elementdataman::load_data`, VA 0x81b1e3d): a sequência de `elementdataman::array<T>::load` e o imediato de tamanho de cada uma (B100). Fechar no último byte **não basta**: os tamanhos "medidos" do B94 para as tabelas 98–112 também fechavam, com contagens erradas. Cliente 1.2.6, `elementclient.exe` VA `0x60ff5d-0x60ffb0`: confirma cabeçalho de 4 B e a primeira tabela de `0x54` B. Campos partem do v156 pelo nome e são cortados no tamanho do v7 — **onde o v156 inseriu campo no meio isso desloca tudo**, e cada caso precisa de evidência: `NPC_TASK_OUT_SERVICE` = ID + Name + `id_tasks[32]` (sem os 7 `storage_*`; `gs` `npc_stubs_manager::LoadTemplate` VA 0x80ef014); `NPC_SKILL_SERVICE` = `id_skills[128]` + `id_dialog` (VA 0x80ef202); `NPC_TASK_IN_SERVICE` = `id_tasks[32]` (VA 0x80eef40); `NPC_ESSENCE`: os 16 ids de serviço que o `gs` lê batem (+760 task_out … +836 equipundestroy); `NPC_TRANSMIT_SERVICE`: `num_targets` +68, destinos de 12 B (conferido); `WEAPON`/`ARMOR`/`DECORATION_ESSENCE` sem `fixed_props`/`probability_hidden`/ocultos (armadura 139: defesa 552, preço 4.800/9.600); `MINE_ESSENCE`: materiais de 8 B e `npcgen_1` em +388; voo: sem `file_model2`; `PET_ESSENCE`: sem `damage_d`; `MONSTER_ESSENCE`: sem `attack_degree`/`defend_degree`, drop em +1220 (B94). Conferidos no consumidor do `gs` 1.2.6 (B122): `CHARRACTER_CLASS_CONFIG` — os 23 campos de 0x44 a 0x9c na ordem do `playertemplate.cpp` 1.5.5 (`player_template::__LoadDataFromDataMan`, VA 0x80e67d2; o 1.2.6 não tem `spirit_adjust`/`poker_adjust`); `PARAM_ADJUST_CONFIG` — `level_diff_adjust[16]` de 24 B em +0x44, `team_adjust` em +0x1c4 e `team_profession_adjust` em +0x21c, de 8 B; `PLAYER_SECONDLEVEL_CONFIG` — `exp_lost[256]` em `+0x44 + 4·i`; `STONE_ESSENCE` — `price` +0x150, addons +0x160/+0x164, `pile_num_max` +0x1a8 (`generate_stone`). **Corrigidos (B122):** `TASKDICE_ESSENCE` = `task_lists[8]` (0x144..0x184), `use_on_pick`, `pile_num_max` +0x188, `has_guid` +0x18c e `proc_type` +0x190 (`generate_taskdice`, VA 0x81f08a0; o corte do v156 punha 10 listas e as duas últimas eram esses campos — a Carta da Sorte podia sortear missão de id `pile_num_max`); `MINE_ESSENCE` = `npcgen[3]` + `aggros[1]` `{monster_faction, radius, num}` + `permenent` nos 64 B de +0x184 (o fonte 1.5.5 tem `npcgen[4]`; lido como `npcgen_4`, os 3 registros não nulos davam monstro inexistente e `num` 200.0 em float) — **47 minas permanentes** do 1.2.6 sumiam ao serem colhidas, porque `permenent` estava no `_opaco`. O `generate_v7.py` agora também carrega o `PET_ESSENCE` do B111, que só estava no JSON. **B145:** `RECIPE_ESSENCE` v7 **sem `bind_type`** e sem o rabo de melhoria do v156 (`recipe_manager::LoadTemplate` do `gs` 1.2.6, VA 0x80f0a12: `recipe_level` +0x4c, `id_skill` +0x50, `skill_level` +0x54, alvos a partir de +0x58, `fail_probability` +0x78, `num_to_make` +0x7c, `price` +0x80, `duration` +0x84, `exp` +0x88, `skillpoint` +0x8c, materiais +0x90..+0x190) e `NPC_MAKE_SERVICE` v7 **sem `produce_type`** (1224 = 72 + 8 × 144; lido como v156, o campo saía 0x00730045, texto do título da página). **Sem opacos desde o B143**, pelos valores do `realm_126`: `CUSTOMIZEDATA_ESSENCE` não tem `file_icon` (os 8 B finais são `character_combo_id` 1/2/8/64/128/192 e `gender_id` 0/1); `PLAYER_ACTION_INFO_CONFIG` tem 11 sufixos de arma e `hide_weapon` (0/1); `FACEPILL_ESSENCE` tem 16 `pllfiles` e `price`/`shop_price`/`pile_num_max`/`has_guid`/`proc_type` (500/2000/10/0/0), a ordem do `exptypes.h:2129-2159` |
| fallback | Só versão fora do catálogo vai ao leitor tipado antigo `elements.rs`; o v7 passou ao genérico. Não há mais tabela `V7_OPACA_*`: as 118 têm nome pelo `gs` 1.2.6 |
| sem catálogo | v181 (cliente build 2591): `.cfg` já copiado, falta `generate_v181.py` |

Consumidores no mundo (os demais índices estão lidos e **não ligados**):

| tabela | módulo | uso |
| :--- | :--- | :--- |
| `WEAPON_ESSENCE`, `ARMOR_ESSENCE`, `DECORATION_ESSENCE`, `PROJECTILE_ESSENCE` | `armas.rs`, `armaduras.rs` (`TabelasDeEquipamento`) | bloco de dados do item no `OWN_ITEM_INFO`; famílias disjuntas por id. `weapon_level` = `level` da arma (B51; era 1 fixo — o Arco de Madeira é 0); munição com `IVTR_ESSENCE_ARROW` (tipo, dano extra, faixa de nível da arma) |
| `MINE_ESSENCE` | `minas.rs` (`GameDataManager::minas`) | coleta, com as recusas e limites de `npcgenerator.cpp:1280-1365` (distância 4–20, coletores 1–20, probabilidades somando 1). O v7 não expõe `material_gain_ratio`; até medir a regra antiga, a chance usada vem da soma das probabilidades do próprio registro (=1 após validação); falta confirmar em jogo |
| `MONSTER_ESSENCE` | `monstros.rs` | atributos, resistências, raios de ódio/visão, `patroll_mode`, `aipolicy_id`, dinheiro e drop (`probability_drop_num0..3`, `drop_times`, `drop_matters[32]`); `local_var[3]` (`param1..3` no layout v156, B127); `id_strategy`, `skills[32]` e `skill_hp75/50/25` — **as 5 entradas de cada limiar, com as de id 0** (B126: o `RandSelect` do evento precisa das posições; `monstros::sortear_evento_de_vida`) |
| `NPC_ESSENCE` | `GameDataManager::ids_de_npc` | decide NPC × monstro de um spawn (como `gs/npcgenerator.cpp:79,415`) |
| `NPC_ESSENCE` → `NPC_TASK_OUT_SERVICE`, `NPC_TASK_IN_SERVICE`, `NPC_SKILL_SERVICE` | `servicos.rs` (`servicos_de_npc`) | que missões o NPC entrega/recebe e que habilidades ensina, ordenadas para busca binária (`general_id_provider`) |
| `NPC_ESSENCE` → `NPC_PETNAME_SERVICE`, `NPC_PETFORGETSKILL_SERVICE`, `NPC_PETLEARNSKILL_SERVICE` (B112) | `servicos.rs` (`renomear_mascote`, `esquecer_habilidade_de_mascote` = `(price, id_object_need)`; `habilidades_de_mascote` ordenadas) | serviços 36/37/38 (`npcgenerator.cpp:783-826`). Nos dois realms a Rilay (11534) cobra 0 moedas e o item 12403 (renomear) / 11690 (esquecer) |
| todas as tabelas com `pile_num_max` | `servicos::pilhas` (`limite_de_pilha`) | empilhamento na bolsa |
| `PLAYER_LEVELEXP_CONFIG` (id 202), `PARAM_ADJUST_CONFIG`, `PLAYER_SECONDLEVEL_CONFIG` | `progressao.rs` | curva de exp, ajuste por diferença de nível (exp, SP, dinheiro, item), perda na morte por cultivo (`playertemplate.cpp:311-419`) |
| `CHARRACTER_CLASS_CONFIG` | `classes.rs` | velocidades, cadência, alcance, regeneração — **sobrescrevem o `ptemplate.conf`** (`gs/playertemplate.cpp:293-301`); agora também no v7 |
| `MEDICINE_ESSENCE` | `quanto_o_remedio_restaura`, `quanto_o_remedio_restaura_no_tempo`, `tipo_maior_do_remedio` | poções. O `id_major_type` dá a classe do item (`set_to_classid`, `gs/template/setclassid.cpp:81-101`): **1794** cura, **1802** mana, **1810** vida e mana, **1815**/**2038** antídotos — e a família de recarga do `cool_time` (B71). Ambos chegam agora pelo catálogo v7 |
| 25 tabelas com `price` + `shop_price` | `precos.rs` | preço de loja `max(shop_price, price)`; durabilidade de fábrica |
| `WEAPON`/`ARMOR`/`DECORATION_ESSENCE` (`repairfee`, `proc_type`) | `precos.rs::carregar_reparo` | preço do reparo e a marca `ITEM_PROC_TYPE_UNREPAIRABLE` 0x1000 (`itemdataman.cpp:1016-1034`, `item.h:321`), v7/v156/v159 pelo nome (B142) |
| `WINGMANWING_ESSENCE` (`mp_launch`, `mp_per_second`) | `precos.rs::carregar_asas` | mana das asas de Arqueiro/Anjo ao decolar e por segundo (B142) |
| `RECIPE_ESSENCE`, `NPC_MAKE_SERVICE` (via `NPC_ESSENCE.id_make_service`) | `receitas.rs` | receitas de produção (`recipe_manager::LoadTemplate`, `npcgenerator.cpp:1387-1470`: `use_time = (int)(20 × duration)` preso a 1..10000, materiais de quantidade zero fora) e as receitas de cada NPC com a habilidade dele (B145) |
| `WEAPON_SUB_TYPE` | `armas.rs` (`velocidade_em_ticks`) | cadência da arma: `(int)(attack_speed × 20 + 0,1)` (B52) |
| `QUIVER_ESSENCE` | `armas.rs` (`GameDataManager::aljavas`) | drop de aljava vira munição (B52) |
| `MONSTER_ESSENCE.size` | `monstros.rs` (`tamanho`) | corpo do alvo no alcance de golpe e habilidade (B52) |
| `POKER_DICE_ESSENCE` + `POKER_ESSENCE` + `POKER_SUB_TYPE` | `cartas_de_general.rs` (`GameDataManager::cartas_de_general`) | caixa de cartas (256 entradas id/probabilidade) e o que a carta gerada leva: tipo do subtipo, `rank`, `require_level`, `require_control_point[2]`, `max_level` (B95; 32 caixas e 227 cartas no 155) |
| `TASKDICE_ESSENCE` | `cartas.rs` (`GameDataManager::cartas`) | Carta da Sorte: 20 missões com probabilidade, `no_use_in_combat` (B53; 2.498 no 155) |
| `EQUIPMENT_ADDON` + `specs/addons_155/addons.json` | `addons.rs` (`addons`) | parâmetros e tratador de cada addon (B53) |
| `WEAPON/ARMOR/DECORATION_ESSENCE` (faixas, `addons`/`uniques`, probabilidades de furo e de nº de addons) | `addons.rs` (`geracao`) | sorteio do equipamento no drop (B53) |
| `STONE_ESSENCE` | `manager.rs` (`pedras`) | addon da pedra na arma/armadura/acessório (B53; sem serviço de incrustar ainda) |
| **não ligadas** | — | `NPC_SELL_SERVICE` (lista de venda), `NPC_TRANSMIT_SERVICE` (teleporte por NPC) |

### 3.2 `tasks.data` — missões (`tasks.rs`)

- `TASK_PACK_HEADER` (`magic`, `version`, `item_count`) + tabela de `item_count`
  deslocamentos `u32` + cada missão de topo por `ATaskTempl::SaveBinary`, com submissões
  recursivas. **Cada missão tem de terminar onde a seguinte começa** (`TasksError::Desalinhado`).
- Versão 129 (1.5.5): bloco fixo **1.157** bytes, prêmio **290** — o fonte (versão 125) dá
  1.087/269; a diferença são os campos do sistema de Lar (nomes do 1.7.2), mais o vetor
  `m_ulHomeItemsWanted × 8 bytes` (B45).
- Estado: **14.885/14.885** (`realm_155`, cliente BR; o EN fechou 14.978/14.978 antes de sair). Versão 124: só cabeçalho. Na v55 (1.2.6), o bloco fixo tem **534 B**, o prêmio **75 B**, o item **13 B** e o monstro pedido **22 B** (`elementclient.exe` VA `0x62f6c0`, leitura em `0x62d04d`; `docs/RESULTADO_TASKS_V55.md`). O leitor Rust fecha **2.819/2.819 raízes**, **7.994** tarefas recursivas e o último byte; rejeita corrupção na tabela ou um byte extra (B96).
- Na v55, `TaskTemplate` recebe id/nome/descrição, hierarquia, horários, itens, monstros, prêmio básico, NPCs, classes, método e conclusão, e desde o B102 as flags `0x6a`–`0x74` (escolhe uma filha, sorteia, filhas em ordem, pai também falha/sucesso, pode desistir, pode repetir, refazer após falha, limpa ao desistir, precisa registro, falha ao morrer), pela sequência `pack(1)` do `TaskTempl.h:2037-2057` e conferidas na `libtask.so` 1.2.6 (`CheckDepth` testa +0x6c/+0x6a/+0x6b; `CanGiveUpTask` lê +0x6f). Sem elas a 1177 ativava as duas filhas juntas (o original entrega só a 1178) e a 1173 deixava de pedir a filha escolhida (1175 ou 1176). Desde o B107 também `m_bAutoDeliver` +0xac (e `m_bDeathTrig` +0xad), zona de entrega +0x79/+0x7a com a caixa +0x7e/+0x8a, nível +0xc1/+0xc5, pré-missões (contador +0xf1, vetor de **5** em +0xf5) e gênero +0x114, medidos no `libtask.so` 1.2.6 (`AddOneTaskTempl`, `CheckInZone`, `CheckLevel`, `CheckPreTask`, `CheckGender`, todos com base `this + 4` = bloco fixo): 81 missões automáticas, entre elas a 9376 "Virando Dinossauro" (nível 1..150, sem classe nem pré-requisito); antes nenhuma. Dados do próprio arquivo: a 949 pede a 947 inexistente, e dez "Teste de Liu Weijun" têm mínimo acima do máximo. Desde o B110 também o lugar a alcançar (método 4): mundo +0x1de e caixa +0x1c6/+0x1d2 (`OnTaskReachSite` do `libtask.so` 1.2.6, que no 1.2.6 não confere o tipo de conclusão); 1.559 missões, nove "Estágio 3-x" (4735-4770) com a caixa invertida no próprio arquivo. Desde o B122, cada campo que o motor usa pela função do `libtask.so` 1.2.6 que o lê (base `this + 4`): prazo +0x49 (`RecursiveCheckTimeLimit`), `m_lAvailFrequency` +0x66 (`CheckDeliverTime`; o v55 não tem `m_lPeriodLimit`), `m_ulMaxReceiver` +0x75 (`CheckGlobalRequired`), teleporte ao receber +0x96/+0x97/+0x9b (`CheckDeliverTask`, que também lê `m_lMonsCtrl` +0xa7 e `m_bTrigCtrl` +0xab), `m_bShowPrompt` +0xb3 e `m_bKeyTask` +0xb4 (ordem do `TaskTempl.h` entre +0xae e +0xb5), casamento +0xc0 (`CheckMarriage`), entregues comuns/de missão +0xd7/+0xdb (`CheckGivenItems`), depósito +0xe3 (`CheckDeposit`), reputação +0xe8 (`CheckRepu`), período +0x10a (`CheckPeriod`), facção +0x10f (`CheckFaction`), cônjuge +0x13e, GM +0x140, exclusivas +0x149 com a lista de 5 em +0x14d (`CheckMutexTask`; até +0x171 cabem `m_ulMutexTasks[5]` + `m_lSkillLev[4]`), equipe +0x176/+0x177 (`CheckTeamTask`), `m_ulGoldWanted` +0x1b2 (`HasAllItemsWanted`), espera +0x1e2 (`CheckWaitTime`) e os tipos de prêmio +0x1e6/+0x1ea (`CalcAwardData`). Medido no `realm_126`: 189 diárias e 11 semanais, 118 com prazo de 10 h, 92 com espera de 10 min, 121 de período 20/30, 18 pedindo 1.000.000 de ouro, 2 de casamento, 3 de GM, nenhuma teleportando ao receber (`tasks_do_realm.rs`). `m_bHidden`, `m_lPeriodLimit` e os limites de conta/personagem não existem no v55; Desde o B143, `m_ulType` em +0x45 (0 a 8 no `realm_126`: 7.201 com 0, 298 com 4; a 9376 é 2) — `ID`, nome, `HasSign` +0x40 e o ponteiro +0x41 pela ordem do `ATaskTemplFixedData::LoadFixedDataFromTextFile` do `libtask.so` 1.2.6 ("Type", "TimeLimit", "AbsTime"); o byte +0x4d é `m_bAbsTime` (gravado em VA 0x12925 depois do formato "AbsTime: %d"): **o v55 não tem `m_bItemNotTakeOff`**, e a entrega sempre retira os itens de missão. O Guerreiro nível 1 aceita a 1173 "Primeiro Teste" no teste do motor, com NPC 3517, classe 0 e prêmio de 45 moedas/75 exp/20 SP; falta ver em jogo após publicação (B96). A 1177, missão inicial do Guia Selvagem (NPC 3518, serviço 3531 = [1177, 1178]), só é aceita pelas classes 3 e 4 no nível 1 (erro 13 nas outras); com o `NPC_TASK_OUT_SERVICE` corrigido, o teste de mundo com o banco entrega a 1177 a um Bárbaro nível 1 (B100).
- Extraído: id, nome/descrição (XOR pelo id), mãe/filhas, tipo, prazo, níveis, classes,
  gênero, pré-requisitos, itens pedidos/entregues, NPCs de entrega e prêmio, objetivos
  (monstros com item, itens, dinheiro, nível, mundo, espera), flags e os dois prêmios.
  Atravessado sem guardar: diálogos, expressões, regiões de entrar/sair (falha), prêmios
  por escala, requisitos de título/Lar.
- Lidos no B51: janelas de horário (`m_tmStart/End` alternados no trecho variável + `m_tmType`
  em +109), `m_ulDelvWorld`/`m_pDelvRegion`, `m_pReachSite`, `m_ulLeaveSiteId`/`m_pLeaveSite`,
  `TEAM_MEM_WANTED` (36 bytes), `m_bRcvChckMem`/`m_fRcvMemDist`/`m_bDistinguishedOcc`/
  `m_bCoupleOnly`, `m_iPremise_FactionRole`, `m_bTransTo`/`m_ulTransWldId`/`m_TransPt`. No
  155: 1.058 missões com horário, 1.227 com entrega em zona, 3.136 de chegar a lugar, 444 de
  equipe, 25 de facção, 227 com teleporte ao receber
  (`examples/missoes_por_sistema.rs`, `missoes_com_teleporte.rs`).
- Para o motor de missões (spec 05 §10) também: as flags de `CheckPrerequisite`/`RecursiveAward`
  (`m_bParentAlsoFail/Succ`, `m_bCanRedoAfterFailure`, `m_bClearAsGiveUp`, `m_lAvailFrequency`,
  `m_bAccountTaskLimit`, `m_bRoleTaskLimit`, `m_bHidden`, `m_bDisplayInTitleTaskUI`,
  `m_bClearAcquired` (no v55 em +0xae, B117 — o `libtask.so` 1.2.6 o testa antes do `RemoveAcquiredItem`), `m_ulGivenCmnCount/TskCount`, `m_ulAwardType_S/F`, …), `dps`/`dph` do
  monstro pedido e `m_bUseLevCo`/`m_bMulti` do prêmio. Deslocamentos em
  `specs/tasks_155/layout129.tsv` e `award_layout.tsv` (sonda do MSVC, B45).
- `profundidade` = `m_uDepth` (`CheckDepth`, `TaskTempl.h:2748-2771`), calculada após a leitura.

### 3.2b `dyn_tasks.data` — missões dinâmicas (`dyn_tasks.rs`)

Só o cabeçalho: `DYN_TASK_PACK_HEADER` (`task/TaskTemplMan.cpp:45-51`), 12 bytes —
`pack_size u32`, `time_mark i32`, `version u16`, `task_count u16`. Recusa como o
`UnmarshalDynTasks`: `version != 10` ou `pack_size` diferente do tamanho do arquivo. Nos dois
realm 1.5.5: 12.979 bytes, marca `0x52776c0d`, 28 missões. Usos:
`GameDataManager::marca_das_missoes_dinamicas` responde ao pedido de marca, e
`GameDataManager::missoes_dinamicas` guarda o **arquivo inteiro**, que vai ao cliente em
pedaços quando ele pede os dados (`TASK_CLT_NOTIFY_DYN_DATA`, spec 04 §5; B59) — sem isso o
cliente não inicializa a lista de missões. `falta`: ler as 28 missões do pacote (o servidor só
repassa os bytes).

### 3.3 `npcgen.data` — onde as coisas nascem (`npcgen.rs`)

Autoridade: `cgame/gs/template/npcgendata.h/.cpp`. Um arquivo por pasta de mapa.

| bloco | struct | bytes |
| :--- | :--- | ---: |
| cabeçalho | `NPCGENFILEHEADER7` (v≥7); `HEADER`/`HEADER6` com 2–3 inteiros em v<7 | — |
| área de IA | `NPCGENFILEAREA7` (v≥7) / `NPCGENFILEAREA` (v<7, 59) | 71 |
| gerador de IA | `NPCGENFILEAIGEN` (v≥11, com `iRefreshLower`) / `...AIGEN10` | 64 / 60 |
| área de recurso | `NPCGENFILERESAREA7` / `RESAREA6` (v6, sem `idCtrl`/`iMaxNum`) / `RESAREA` (v<6, sem `dir`/`rad`) | 42 / 34 / 31 |
| objeto dinâmico | `NPCGENFILEDYNOBJ10` / `DYNOBJ9` (sem `scale`) / `DYNOBJ` (v<9, sem controlador) | 24 / 23 / 19 |
| controlador | `NPCGENFILECTRL8` / `CTRL` (v<8, sem `iActiveTimeRange`) | 199 / 195 |

- 1.5.5 é **v11**. Todas as versões 5–11 (`CNPCGenMan::Load`, `npcgendata.cpp:62-315`); o
  leitor **recusa sobra** — os arquivos em disco (41 do 1.2.6, 75 do `realm_155`; eram 193 com os 77 do EN, que saiu em B55) fecham no último
  byte (`examples/conferir_npcgen.rs`, B51).
- Campos usados: tipo de área (`iType`: no chão / na caixa), `vExts` (**tamanho** da caixa),
  `iDeadTime`/`iRefresh`/`iRefreshLower` → `SpawnInstance::{corpo_s, renascer_min_s, renascer_max_s}` pelas regras de `npcgenerator.cpp:3828-3855` (B105; antes os dois primeiros eram lidos e descartados, e `respawn_sec` = `iRefresh.max(1)`), `fOffsetTrn`/`fHeiOff` (zero em 18.902 de 18.903 geradores), `fOffsetWater` (guardado, sem
  mapa de água), `iPathID`, `iSpeedFlag`, `iLoopType` (B136), contagens sem teto inventado.
- Grupo (B136): `iGroupType` da área → `SpawnInstance::tipo_de_grupo` (0 `mobs_spawner`, 1
  `group_spawner`, 2 `boss_spawner`, `npcgenerator.cpp:3717-3732`), com `area_de_ia` (a ordem da
  área) e `gerador_na_area`. No grupo e no chefe o gerador 0 é o líder e sai com **uma** cópia
  (`quantidade_do_gerador`; o original força 1, `npcgenerator.cpp:5297-5312`, `:5405-5415`).
  `examples/grupo_do_monstro.rs` lista as áreas de um monstro pelo nome.
- Recurso (`ResourceMine`): altura = relevo + **piso do mapa de movimento** + `fHeiOff` do `NPCGENFILERES` — a mesma regra da área no chão (`SetRegion(0, ...)` → `terrain_gen_pos::Generate`, que chama `GetValidPos`, `npcgenerator.cpp:3900-3902`, `:4299-4318`, `pathfinding/NPCMoveMap.h:200-211`). Até o B144 o piso ficava de fora (B109 dizia "sem o mapa de movimento", o que estava errado): a Água Cristalizada da missão 31797 (mina 44570, mapa 161, `a61`) nascia 0,9375 m abaixo da borda da fonte, dentro do objeto — e o cliente só traça para baixo a partir de `y + 0,2` (`EC_Matter.cpp:203-209`), então ela ficava lá (B145; `movemap_do_realm::a_agua_cristalizada_nasce_na_borda_da_fonte`). O `fHeiOff` põe baú em cima de construção: Baú de Tesouros 11117 (missão 3428) com 33,5 m, Baú Desgastado 12858 (missão 7017) com 26,8 m. Mina de missão: `materials_1_id` 0 e `task_in`/`task_out` = a missão; o item vem do `OnTaskMining` (`colheu_mina`).
- `PET_ESSENCE` (B111): o layout v7 inventava um `pet_snd_type` em 0x154 e deslocava `hp`/`hp_gen`/`damage`; o `gs` 1.2.6 (`pet_dataman::LoadTemplate`, VA 0x8143580) lê `hp_a`…`magic_defence_d` (26 floats, com `damage_d`) a partir de 0x154, depois `size`, `damage_delay`, `attack_range`, `attack_speed`, `sight_range` (int), `food_mask`, `inhabit_type` e um `unk` até os 476 B. `ModeloDeMascote` (`pet.rs`) com as recusas do original: 460 modelos no 1.2.6 (413 de combate), 783 no 1.5.5 (439); `PET_FOOD_ESSENCE` → `comidas_de_mascote`; curva do mascote = `PLAYER_LEVELEXP_CONFIG` 592 (`exp_do_mascote_para_subir`).
- Altura (`SpawnInstance::posicao_no_mapa` → `altura_resolvida`, `gs/npcgenerator.cpp:4296-4346`):
  área no chão → `chão + piso do movemap + offset`, com **até 5 sorteios** de `x`/`z` quando o
  ponto não é alcançável (`terrain_gen_pos::Generate` + `GetValidPos`, §3.6c, B97); área em
  caixa → `max(y sorteado, chão) + offset` (não consulta o movemap); sem `.hmap` → `y` do
  arquivo.
- Id de recurso já vem com `0xC0000000` (`ISMATTERID` do cliente).
- `id_ctrl != 0` **não** desliga a área: a maioria dos controladores é o registro normal de
  NPC permanente e nasce ativa (B17a).
- **Controladores em jogo (B127):** `NpcGenData::controladores`, pelo `iControllerID` (o número
  que o `TriggerSpawn`/`ClearSpawn` recebe, `npcgenerator.cpp:3616-3636`, `3969-4010`): os ids
  das instâncias que nasceram na carga sob ele e as instâncias **pendentes** (área com
  `bInitGen` falso ou controlador desligado), estas com ids depois de todos os da carga — o id
  de quem já nascia não muda. Só áreas de monstro/NPC; recurso e objeto dinâmico não.

### 3.4 `aipolicy.data` — IA de criaturas (`aipolicy.rs`)

Porta de `cgame/gs/ai/policy.cpp` (`CPolicyDataManager::Load` e derivados), autoridade o
fonte **1.7.2**. Cabeçalho `F_POLICY_EXP_VERSION` (1.5.5 = 1, 3.144 políticas; 1.2.6 = 0,
293). Parâmetros de condição trazem tamanho no fio; os de operação usam `sizeof` do alvo
32 bits (tabela `tamanho_do_parametro`). **Interpretado no mundo desde o B127** —
`pw_gs::politica` (spec 05 §4.1).

### 3.5 `ptemplate.conf` (`ptemplate.rs`)

Seções por classe (`[SWORDSMAN]`, `[ORGE]`, `[ASN]`, `[ANGEL]`…), **GBK**. Lido, mas quase
nada dele chega ao jogador: o `gamed` copia a ficha do banco (`userlogin.cpp`), que nasce do
`clsconfig`. **Não** são usados os atributos (Arqueiro 15/20/5/10 aqui; 5/5/5/5 no molde)
nem o `hp`/`mp`: `max_hp = lvlup_hp×(nível−1) + vit_hp×vitalidade`, base zero (B51 — antes
somava o `hp` do `.conf` e o Arqueiro nascia com 20 de energia). As velocidades dele também
são mortas. `[TOWN_REGION]` é o mapa de ressurreição, não o nascimento.

**Dois conjuntos de seções (B101).** 1.5.5: as 12 de `SECOES_DE_CLASSE`. 1.2.6: 8, na ordem
de `player_template::__Load` do `gs` 1.2.6 (VA 0x80e4efc): 0 `SWORDSMAN`, 1 `MAGE`, 2 `MONK`,
3 `HAG`, 4 `ORGE`, 5 `GENIE`, 6 `ARCHER`, 7 `ANGEL` (`SECOES_DE_CLASSE_126`). O leitor escolhe
o conjunto cujas seções estão todas no arquivo. Até o B101 ele exigia `[NEC]` e recusava o
arquivo do `realm_126`, o mundo ficava com `base_das_classes` vazio e `recalcular_por_nivel`
não fazia nada: ficha com dano 1-1, vida e mana gravadas no banco; e o `pw-link` criava o
personagem sem ficha, caindo nos moldes do banco (que tinham os atributos do `.conf`).

### 3.6 Terreno: `gs.conf` → `specs/mapas/terreno_155.json` → `map/<n>.hmap` (`terreno.rs`)

| | |
| :--- | :--- |
| catálogo | `specs/mapas/gerar_terreno_155.py` extrai do `gs.conf`: 79 mapas **pela `tag`** (não pelo `index`) |
| arquivo | `map/<n>.hmap`, `n` de 1 a colunas×linhas; `(nAreaWidth+1)²` floats LE em 0..1, sem cabeçalho (513² × 4 = 1.052.676 bytes) |
| altura | `h × (vHeightMax − vHeightMin) + vHeightMin` |
| origem | `ox = −(vert×colunas×célula)/2`, `oz = +(vert×linhas×célula)/2`; `z` invertido |
| conferência | mundo 1 dá `x ∈ [−4096, 4096]`, `z ∈ [−5632, 5632]` = `base_region` do `gs.conf` |

A configuração por mapa não se deduz da pasta (88 blocos podem ser 8×11 ou 11×8). O mapa
161 (`a61`) é 4×3.

### 3.6c `movemap/` — o piso andável acima do terreno (`testado`, B97)

`pw_data_loader::MapaDeMovimento` (`movemap.rs`), porta de `gs/pathfinding/NPCMoveMap`,
`BitImage.h` e `BlockImage.h`. Responde `acima_do_terreno(x, z)` = `GetValid3DPos`: `Some(altura
do piso acima do terreno)` se o pixel é alcançável, `None` se não (ou fora da grade); mapa sem
dados → `Some(0)`.

| arquivo | formato |
| :--- | :--- |
| `movemap.conf` | texto (termina num comentário em GBK — lido como bytes): `Map Width`, `Map Length`, `Submap Width`, `Submap Length`, `Pixel Size` |
| `N.rmap` | `CBitImage`: `u32 1`, `u32 tamanho`, `i32 largura_em_bytes`, `i32 comprimento`, 2 × `i32` da imagem, `f32 pixel`, bits (1 = alcançável). 131.100 B por submapa de 1024² |
| `N.dhmap` | `CBlockImage<FIX16>`: `u32 1`, `u32 tamanho`, `i32 largura`/`comprimento` em blocos, `i32 expoente`, 2 × `i32`, `f32 pixel`, ids de bloco (−1 = zero), `i32 n`, `n` blocos de `lado² × u16`; altura = valor / 64 m |

- Arquivo `N` do submapa `(u, v)` = `(comprimento − v − 1) × largura + u + 1`; submapa sem
  arquivo = todo alcançável, altura zero (`NPCMoveMap.cpp:115-123`). Origem no centro do mapa.
- Os dois arquivos **fecham no último byte** (senão o submapa volta ao padrão, como no original).
- `realm_155`: o mundo tem 55 dos 88 submapas; o `a61` (mapa 161) tem 2. Nos 36.945
  nascimentos de área no chão do realm, 439 ficam em cima de estrutura — 22 das 52 Gárgulas
  Ancestrais do mapa 161 (`tests/movemap_do_realm.rs`).
- Usos no mundo: altura do nascimento (acima) e o piso do passo do monstro de chão
  (`Get3DPosOnGround`), e o **alcance** para o desvio de obstáculo dos agentes de
  `pw_gs::navegacao` (B99): `pixel_de`, `alcancavel`, `acima_no_pixel`, `centro_do_pixel`,
  `vizinhos_alcancaveis` e `reta_livre` (o `CanGoStraightForward` com o pixel de parada).

### 3.6b `watermap/` — a superfície da água (`testado`, B88)

Cada mapa do realm tem uma pasta `watermap/` ao lado de `map/`, com `watermap.conf` e
arquivos `N.wmap`. São **75** no `realm_155`, e é deles que o original tira o
`path_finding::GetWaterHeight` — a altura da água num ponto.

| arquivo | formato |
| :--- | :--- |
| `watermap.conf` | texto, quatro linhas: `Map Width`, `Map Length`, `Submap Width`, `Submap Length` (`CGlobalWaterAreaMap::Load`, `gs/pathfinding/GlobalWaterAreaMap.cpp:75-120`). No `realm_155`, 1×1 submapas de 1024×1024 na maioria |
| `N.wmap` | binário: `u32 versão`, `f32 largura`, `f32 comprimento`, `i32 n`, e `n` áreas de **5 `f32`** — `cx`, `cz`, meia-largura, meio-comprimento, **altura** (`CWaterAreaMap::Load`, `gs/pathfinding/WaterAreaMap.cpp:38-115`). O nome é `(m_iLength−i−1)*m_iWidth+j+1` |

Sem áreas o arquivo tem 16 bytes (só o cabeçalho), e é o caso da maioria dos mapas; o mundo
1 tem vários com água, o maior com 5 áreas (116 bytes).

Lido por `watermap.rs` (`MapaDeAgua::ler`), que **recusa arquivo que não fecha no último
byte** e recusa submapa cuja medida não bate com a do `.conf` — é o que o original faz, que
libera o submapa nesse caso. `MapaDeAgua::altura_em(x, z)` é o `GetWaterHeight`, e
`quanto_abaixo(x, y, z)` é o `off` do `TestUnderWater`. **Zero quer dizer "sem água"**, não
"água no nível zero" (`NO_WATER`).

**Para que serve:** `IsUnderWater` do jogador, que o original usa para recusar montaria e
para **derrubar** quem entra na água montado (spec 05, montaria), e o fôlego (`breath_ctrl`,
`falta`). O `fOffsetWater` do `npcgen.data` continua guardado sem uso.

### 3.6d `airmap/` — o espaço passável do ar e da água (`testado`, B133)

`pw_data_loader::MapaDoEspaco` (`espaco.rs`), porte de `gs/pathfinding/GlobalSPMap` e
`CompactSpacePassableOctree`, carregado por mapa (`WorldInstance::espaco`, junto do `watermap/`).

| arquivo | formato |
| :--- | :--- |
| `spmap.conf` | texto (termina num comentário em GBK — lido como bytes): `Map Width`, `Map Length`, `Submap Size`, `Voxel Size` (mapa 1: 8 × 11, 1024, 2) |
| `N.octr` | `u32 0xcc000001`, `u8 id`, `CubeInt` (4 `int`: centro e meia aresta), `int` folha, `u32 n`, `n × u32` nós (2 bits de estado — 0 livre, 1 bloqueado, 2 borda —, 26 bits do endereço dos filhos) — **29 + 4n B, fecha no último byte**. `N = (comprimento − linha − 1) × largura + coluna + 1` |

Submapa sem arquivo (ou recusado) é **livre**, com o centro calculado (`SetFreeSPMap`). Consultas:
`centro_do_voxel` (`GetVoxelCenter`), `no` (`GetTraversalNode`, filho = x·4 + y·2 + z), `livre`.
Conferido: **27 octrees no `realm_126` e 120 no `realm_155`, todas fechando** (`tests/espaco_do_realm.rs`).

### 3.6e Limites do mapa: `gs.conf` → `specs/mapas/limites_<versão>.json` (`limites.rs`, B133)

A chave `limit` (e `height_limit`) de cada `[World_*]`/`[Instance_*]` do `gs.conf` original
(`world_manager::InitWorldLimit`, `worldmanager.cpp:108-160`), pela `tag`, gerada por
`specs/mapas/gerar_limites.py`: **1.2.6** do `files1.2.6/pwserver/gamed/gs.conf` (43 mapas, 34
`nofly`), **1.5.5** do `pwserver_155v156` (79 mapas, 50 `nofly`). O mapa 1 **voa** nas duas. Só o
`nofly` tem consumidor (spec 05 §7).

### 3.6f `path.sev` — as rotas de patrulha (`testado`, B136)

`pw_data_loader::rotas::Rotas` (`rotas.rs`), porte do `path_manager::Init`
(`gs/template/pathman.cpp`) sobre o `CSevBezierMan` (`gs/template/sevbezier.cpp`), da pasta do
mapa (`world/path.sev` no mapa 1).

| parte | formato |
| :--- | :--- |
| cabeçalho | `int iVersion` (recusa `> 1`), `int iNumBezier` |
| curva | `u32 dwVersion`, `int id`; com `dwVersion ≥ 2`, `int global`, `int próximo global`; `int n` × (`pos` 12 + `dir` 12); `int m` × (âncora início 12, âncora fim 12, `int início`, `int fim`, `float comprimento`) |

O servidor não anda na curva: na carga, cada rota vira a lista de pontos de um
`CSevBezierWalker` a **8 m/s amostrado a cada 1000 ms** (a posição antes de cada `Tick`; o
ponto final da curva não entra; para quando acaba ou não sai do lugar), com as curvas
encadeadas por `iNextGlobalID` emendadas. `do_global` é a tabela `IdConvert`. Arquivo de 0 B
(o `b01` do 1.5.5) = sem rota. Conferido: **116 `path.sev` nos dois realms, 3.792 rotas, todos
fechando no último byte**; a rota 486539715 do Carniçal Sanguinário (1.2.6) tem 134 pontos
(`tests/rotas_do_realm.rs`).

### 3.6c Arquivos do realm que este servidor **não** lê

Estão na pasta do realm e não são pendência nossa, com uma exceção:

| arquivo | de quem é |
| :--- | :--- |
| `task_npc.data` | **do cliente** (`LoadNPCInfoFromPack("data\task_npc.data")`, `Task/EC_TaskInterface.cpp:166`) |
| `DynamicObjects.data` | **do cliente** (`m_pDynObjPath->Load("configs\DynamicObjects.data")`, `EC_Game.cpp:612`) |
| `domain.data`, `domain2.data`, `domain2_cross.data` | não aparecem no fonte do `gs` nem no do cliente; provavelmente do `gdeliveryd` (domínios e cruzamento entre servidores) — fora do alvo atual |
| `extra_drops.sev` | não aparece em nenhum dos dois fontes que temos; origem desconhecida |
| `globalcontroller.conf` | **é do `gs`** (`worldmanager.cpp:1106`) e traz `cash_money_exchange_rate` (1.000.000 no `realm_155`). `falta`, mas só importa quando a **Loja Gold** existir |
| `airmap/`, `movemap/`, `map.bht`, `.dhmap`/`.rmap` | navegação e colisão do `path_finding`; o mundo hoje não faz pathfinding de criatura por malha |

### 3.7 `region.sev` / `precinct.sev` (por mapa)

`REGIONFILEHEADER4` (`dwVersion, iNumRegion, iNumTrans, dwTimeStamp`@12) e
`PRECINCTFILEHEADER5` (`dwVersion, iNumPrecinct, dwTimeStamp`@8) — `el_region.h`,
`el_precinct.h`. O carimbo vai no `INST_DATA_CHECKOUT`. Versões < 4 recusadas.

O `precinct.sev` é lido inteiro (`precinct.rs`, `GameDataManager::distritos`): por distrito,
`iNumPoint`, prioridade, mapa do ponto, mapa do distrito (v ≥ 4), domínio (v ≥ 6), proteção PK
(v ≥ 7), ponto de cidade e vértices (`CELPrecinct::Load`, `el_precinct.cpp:219-266`); recusa
byte a mais. `distrito_em(x, z, mapa)` = menor prioridade que contém o ponto
(`CELPrecinctSet::IsPointIn`, `:379-394`) — é o renascer na cidade. Carimbo que não bate com o `.clt` do cliente naquela zona gera erro visível no
cliente (B25).

### 3.7.1 `world_targets.sev` (`world_targets.rs`) — B68

A tabela de **pontos de destino do mundo**: é dela que sai a coordenada de cada destino de
transportadora (o `NPC_TRANSMIT_SERVICE` do `elements.data` cita os pontos só por id).

```text
u32 quantidade
quantidade × { i32 id; i32 world_tag; f32 x; f32 y; f32 z; i32 ordem }   // 24 bytes
```

No `realm_155`: **92 pontos, 2.212 bytes** (`4 + 92 × 24`), fechando no último byte. Os 427
destinos que as 95 transportadoras citam estão todos aqui
(`pw-data-loader/tests/teleporte_do_realm.rs`). No original o equivalente entra no
`transmit_entry` que o `transmit_provider` monta (`gs/serviceprovider.cpp:683-700`).

### 3.8 `gshop*.data` (`gshop.rs`) — `testado` (B125)

Aceita o nome do cliente (`gshop.data`, `gshop1.data`, `gshop2.data`) ou do servidor
(`gshopsev*.data`). O carimbo (primeiro `u32`) vai ao `edition` (spec 02 §3.3). As ofertas
saem do formato do **cliente** (`globaldataman.cpp:614-660`: `n × GSHOP_ITEM` + 8
categorias), com o layout que fecha o arquivo no último byte:

| layout | registro | `buy[4]` | realm |
| :--- | ---: | :--- | :--- |
| 1.5.5 `VIP` | 1436 | 36 B `{price,end,time,start,type,day,status,flag,min_vip}`; brinde, donos, limite | `realm_155` (1726 / 205 / 6 ofertas) |
| 1.5.x sem `VIP` | 1412 | 32 B; brinde, donos | — |
| 1.2.6 | 1288 | 12 B `{price, data absoluta, time}` (`load_malldata`, `gs` 1.2.6 VA 0x81e877c) | `realm_126` (668) |

O índice na lista é o `goods_index` do `MALL_SHOPPING`: por isso a loja sai do `gshop.data`
do cliente (idêntico ao do cliente BR e ao do pacote 1.2.6), não do `gshopsev.data` — o do
`realm_155` tem outra lista (2262 ofertas). Nenhum layout fechando (`gshopsev`, arquivos de
teste): só o carimbo, loja vazia. Prova: `pw-data-loader/tests/gshop_do_realm.rs`.

### 3.9 `collision.clt` (`collision.rs`)

Carregado por mapa; **não consultado** pela simulação (monstro atravessa obstáculos).

### 3.10 `gamedbd/clsconfig` — moldes de classe (fora do realm, no pacote do servidor)

Arquivo do `gamedbd` original com os roleids 16..31: `GRoleBase` + `GRoleStatus` +
inventário + equipamento + armazém, Marshal **big-endian** (`cnet/gamedbd/clsconfig.h`).
Molde por classe via `GetDataRoleId` (`gamedbmanager.cpp:208`): 0→16, 1→19, 2→20, 3→23,
4→24, 5→27, 6→28, 7→31, 8→18, 9→17, 10→21, 11→22.

- Leitor: `specs/clsconfig_155/ler_clsconfig.py` (nome `cls<N>gender<M>`, `GRoleBase` e começo
  de `GRoleStatus`). Posições aplicadas ao banco por
  `scripts/2026_09_12_nascimento_no_mapa_161_155.sql` (todos no mapa 161, pacote
  `pwserver_155v156`).
- Inventário e equipamento conferidos byte a byte para o Arqueiro: só o Arco de Madeira
  (2250), **sem munição**. As flechas do molde são decisão do projeto a pedido do Murillo:
  **Flecha de Novato (43283) × 1000** no slot 11, porque o arco é arma de nível 0 e a Flecha
  de Iniciante (8543) pede nível 1–17 (`scripts/2026_09_16_flecha_de_novato_155.sql`).
- `GRoleStatus.property` (`extend_prop`, `property.h:35`) dos 12 moldes: **5/5/5/5** e vida/mana
  = `vit_hp×5`/`eng_mp×5` — a evidência dos atributos iniciais (spec 05 §7).
- `falta`: `config_data` (provável barra de atalhos), leitor de inventário/equipamento, habilidades.
- **1.2.6** (`files1.2.6/pwserver/gamedbd/clsconfig`, B101): o `extend_prop` (vitality, energy,
  strength, agility, max_hp, max_mp — `property.h:35-45`, little-endian dentro do `Octets`)
  dos moldes das seis classes do 1.2.6 que estão no mundo 1 dá **5/5/5/5**, com vida/mana =
  `vit_hp`/`eng_mp` × 5 do v7 (Feiticeira 60/60, Bárbaro 85/35). Os moldes de mundo 0 (classes
  2 e 5, que o 1.2.6 não tem) guardam outros números e não valem. Aplicado ao banco por
  `scripts/2026_09_24_atributos_iniciais_5_126.sql`. As posições do `clsconfig` 1.2.6 **não**
  batem com as dos moldes do banco para humanos (217,3; 218,5; 2838,4 contra 976; 219,2;
  4187,3) e alados (−317,4; 218,1; −910,99 contra −741,5; 219,1; −1234,8): `falta` decidir com
  captura de personagem novo. **Resolvido (B102):** os pontos do `clsconfig` ficam ao lado do
  Guia de cada raça no `npcgen.data` do mapa 1 (Guia 3517 em 221,5; 2854,4 — humanos; 3518 em
  −1445,2; 1398,9 — selvagens; Guia Jace Johnson 3519 em −313,4; −893,1 — alados), junto aos
  monstros das primeiras missões; os do banco eram pontos de cidade. O `gamedbd` 1.2.6 usa o
  mesmo `GetDataRoleId` para as classes 0–7 (VA 0x810c542). `ler_clsconfig.py --sql <realm>
  --classes …` gera o SQL (posição e `ui_config`); aplicado em
  `scripts/2026_09_24_moldes_do_clsconfig_126.sql`.

### 3.10b Habilidades do servidor — `specs/habilidades_155/habilidades.json` (`habilidades.rs`)

**Ódio (B137):** `odio` = `GetEnmity` por nível (`SkillStub::GetEnmity`, `skill.cpp:648-653`),
e `odio_expr` quando depende do nível de quem lança ou do talento. 1.5.5: 3.285 por nível, 31
por expressão (18 com `10 * P_Level * (3 + L)`). 1.2.6: 805 executados no `gs` por nível e as
mesmas 18 por expressão — o extrator distingue o `GetLevel` do jogador (o `this` falso 0x3000),
executa com o jogador nos níveis 10 e 57 e só aceita a expressão do 1.5.5 se ela reproduz os
dois. Valores próprios do 1.2.6 em 150, 520, 521, 524, 525 (menores que os do 1.5.5), 28, 151,
275, 277, 290, 498, 499 e 500.

Não é arquivo do realm: é extraído dos stubs `cskill/skills/skillNNN.h` do `EvolvedPWServer`
por `specs/habilidades_155/extrair_habilidades.py` e embutido com `include_str!` (o
`Dockerfile.core` copia o JSON). 3.316 habilidades; por nível: mana, `GetExecutetime`,
`GetCoolingtime`, `GetRequiredLevel/Sp/Money`, desde o B112 `GetRequiredItem` (`item_exigido`, o livro que o aprendizado de mascote consome; 1.111 no 1.5.5), `GetTime` de cada estado, e desde o B52
`time_type` (3 = carga), `alcance` (`GetPraydistance` = `arma × GetRange() + fixo`, 3.313) e
`dano` (o estado com `SetDamage`/`SetXdamage`: base física/mágica, escola, fator, `ratio` e
`plus` por nível, com `GetCharging()` na carga cheia; 1.123). `null` = expressão
que depende de mais que o nível ou stub com `TODO fix` — tratado como desconhecido. Desde o B53:
`arrowcost`, `tipo_de_area` (`range.type`), `doenchant`, `dobless`, `raio`,
`distancia_de_ataque`, `angulo`, `precisao` (por nível), `distancia_de_efeito` (formato do
alcance) e os roteiros `no_alvo` (`StateAttack`, 2.304) e `em_si` (`BlessMe`, 266) como
`[quem, setter, expressão]` — expressão com `L`, `P_X`, `V_X`, `A_X`, `S_X`, `INT(...)`, `?:`,
avaliada pelo servidor (`pw_gs::efeitos::expr`). Carregado para `elements.data` v156/v159
(`GameDataManager::habilidades`).

**1.2.6 (`TabelaDeHabilidades::do_126`, B100/B101)** — carregada para o `elements.data` v7.
Só as 823 habilidades que o `gs` 1.2.6 compila (as mesmas 823 do `skillstr.txt` do cliente
1.2.6). `specs/habilidades_126/habilidades.json` — desde o B102 **completo**, no formato do 1.5.5 (pode ir para `data/<realm>/catalogo/`), gerado por `extrair_habilidades_126.py`:
cada função do `SkillNNNStub` do `gs` 1.2.6 é **executada** num emulador x86 por nível, com
`GetLevel`, `GetAttack`/`GetMagicattack` (sonda), `PlayerWrapper::GetRange` (0 e 1, na pilha
x87) e `GetCharging` (carga cheia) interceptados e os `Set*` capturados — e daí saem os mesmos
campos do 1.5.5: estados, execução, recarga, mana, aprendizado (nível, SP, dinheiro, livro), alcance,
distância de efeito, raio, distância de ataque, ângulo, precisão e **dano** (estado, base,
elemento, fator, `ratio`, `plus`). Função ausente no stub = padrão do `SkillStub` (0,
`skill.h:409-436`); função que lê outra coisa (vida, `GetPlus`) = `null`, e aí fica o valor do
1.5.5 — hoje só o dano de 317, 529, 666, 667 e 799 (`GetHp`). **Roteiros** (`no_alvo`/`em_si`) e o `TakeEffect` das passivas `EVENT_CHANGE` (`ao_mudar_de_forma`) **saem do `gs` 1.2.6** desde o B122 (`roteiros_126.py`): cada `StateAttack`/`BlessMe`/`TakeEffect` é executado por nível com as consultas respondidas por sondas (`P_`/`V_`/`A_`/`S_` pelo `this` de cada chamada, `float` na pilha x87 para os `Get*` que o `playerwrapper.h` declara `float`) e os setters gravados em ordem. A expressão de cada passo é a herdada do 1.5.5 quando ela reproduz os números — também reescrita com a divisão inteira do C++ em `INT(...)` ou truncada inteira (a 116 guarda `S_Magicdamage × (0,2 + 0,03·L) + 630 + 60·L` num `int`) —, senão ajustada (constante, `a·L + b`, `a·L² + b·L + c`, tabela por nível, afim nas consultas, `X_Cls == -1 ? … : …` por partes). Resultado: 1.065 roteiros iguais ao herdado, 578 gerados, 3 vazios, 0 não lidos; `conferir_roteiros_126.py` passou de 64 divergências para **0**. Exemplos: 610 reduz ataque/magia (0,2 por 30 s), 404/406/420… dão chi por `SetAp` (50 % de 100) no lugar do `Apgencont2`, a 187 é 11 % de atordoar por `rand() % 100 + 100` ms, a 312 tem `probability` `0,5·L + 1` e nenhum `SetValue`, a 167 faz `SetReturntown`. `ROTEIROS_DO_GS_126` ficou vazio (o gerador reproduz os dois que eram manuais, 330 e 306). **`eventflag`** sai do construtor (`int` em +0x22, depois de `max_level` +0x18, `type` +0x1c, `attr` +0x1d, `rank` +0x1e): 11 com `EVENT_CHANGE` (4) no 1.2.6, entre elas 323/324 da raposa e a 103 (que no 1.5.5 é 1).
tipo, pré-requisitos, `time_type`, área e flags.
**`allow_forms`** (máscara das formas em que a habilidade pode ser lançada, `skill.cpp:128`) **não** é
herdado: sai do construtor `SkillNNNStub::SkillNNNStub()` do `gs` 1.2.6 (`mov byte [eax+0x4a]`;
o `time_type` em +0x49 bate com o herdado em 822 das 823), e 99 diferem do 1.5.5 (299–310 gravam 5
no 1.2.6 e 1 no 1.5.5; B120). No 1.5.5, `extrair_habilidades.py` lê `allow_forms` e `eventflag`
do stub, e o `TakeEffect` das `EVENT_CHANGE` como `ao_mudar_de_forma` (154, 195, 323, 324, 494, 495, 538, 539, 678, 679).
Diferenças medidas contra o 1.5.5 nas 823: `plus` do dano em 68 e `ratio` em 10 (elemento,
base e fator nunca diferem; a 299 tem 23,7 no nível 1 contra 124,5, a 1 tem 10,8 contra
102,6), dinheiro exigido em 186, nível exigido em 142, estados em 13, recarga em 8, execução em
3, distância de efeito em 9, raio em 3, alcance, mana e SP em 1 cada. O texto do `skillstr.txt`
1.2.6 não é fonte (nas 84 divergências dele com o 1.5.5, o `gs` 1.2.6 concorda com o 1.5.5 em
62). Conferido com a captura original: 102 (200+700 ms), 250 nível 2 (500+900), 299
(1.500+1.000; `123` em +2.504..2.551 ms do `85`) e o dano da 299 no `142` (20, 23, 23, 24; a
conta com a ficha nível 1 dá 32 antes da resistência).

### 3.10c Tratadores de addon — `specs/addons_155/addons.json` (`addons.rs`)

Extraído de `cgame/gs/item/item_addon.cpp` (`INSERT_ADDON(id, tratador)`, fora de comentário)
por `specs/addons_155/extrair_addons.py`: 2.911 ids, 245 tratadores. O tratador decide o
sorteio dos parâmetros (`Sorteio`: ponto, entre dois, porcento, refino...) e o efeito
(`BonusDeAddons`, spec 05 §5.1).

### 3.11 `global_api.lua`

Primeira linha `--<N>` = `lua_version` do `SERVER_TIME`. 1.5.5: **102**; divergência faz o
cliente encerrar ("wrong config data").

## 4. Dados de cada realm 1.5.5

| realm | base do servidor | `.data` do cliente | elements | tasks |
| :--- | :--- | :--- | :--- | :--- |
| `realm_155` | `F:\PW\1.5.5\home155\gamed\config` | cliente BR | v156 (55.442.775 B) | 129 |

O pacote de servidor e os `.data` do cliente precisam ser da mesma família: `npcgen.data`,
mapas e `.sev` **não vêm do cliente** (B11, B12).
