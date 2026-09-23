# Especificação 03: Arquivos de dados do realm (`pw-data-loader`)

> Verificada contra o código em 2026-09-14, commit `e6433ae` + B49. Cobre
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

## 3. Os arquivos

### 3.1 `elements.data` — itens, criaturas, classes, configurações

| | |
| :--- | :--- |
| leitor | `generic_elements.rs` (Rust) e `specs/elements_layouts/pw_elements_reader.py` (painel) — mesmo algoritmo |
| cabeçalho | `u32 versão` (`0x3000009c` = v156, `0x3000009f` = v159) + `time_t` |
| corpo | cada tabela: `u32 count` + `count × sizeof(T)`, na ordem do `elementdataman::load_data` do cliente (`EvolvedPWClient/.../elementdataman.cpp:3879`) |
| exceções | depois de `ARMORRUNE_ESSENCE`: `tag 0xab7689dd`, `len`, `len` bytes (máquina exportadora), `time_t`; depois de `WAR_TANKCALLIN_ESSENCE`: `tag 0xee35679f`, `len`, `len` bytes; `TALK_PROC` com laço próprio |
| layouts | `specs/elements_layouts/v156.json`, `v159.json` (gerados de `specs/elements_155/PW_1.5.5_v15x.cfg` do editor ADMVAL), embutidos com `include_str!` |
| estado | **231/231** tabelas no v156 do `realm_155` (cliente BR); o v159 do cliente EN fechou em 234/234 quando ainda estava no projeto (saiu em 2026-09-17, B55; o layout continua no catálogo); fecha no último byte (`NaoTerminaNoFim` se não) |
| fallback | versão fora do catálogo (1.2.6 = v7) vai para o leitor tipado antigo `elements.rs`. **v7 não tem `time_t`** (os 4 bytes depois da versão já são a contagem da tabela 0); os 118 tamanhos de registro foram deduzidos do próprio arquivo e **fecham no último byte** (16.664.770), conferidos pelo nome do primeiro registro de cada tabela contra o enum `DATA_TYPE` do cliente, menos 103–112 (sem nome, sem leitor) — B51 |
| sem catálogo | v181 (cliente build 2591): `.cfg` já copiado, falta `generate_v181.py` |

Consumidores no mundo (os demais índices estão lidos e **não ligados**):

| tabela | módulo | uso |
| :--- | :--- | :--- |
| `WEAPON_ESSENCE`, `ARMOR_ESSENCE`, `DECORATION_ESSENCE`, `PROJECTILE_ESSENCE` | `armas.rs`, `armaduras.rs` (`TabelasDeEquipamento`) | bloco de dados do item no `OWN_ITEM_INFO`; famílias disjuntas por id. `weapon_level` = `level` da arma (B51; era 1 fixo — o Arco de Madeira é 0); munição com `IVTR_ESSENCE_ARROW` (tipo, dano extra, faixa de nível da arma) |
| `MINE_ESSENCE` | `minas.rs` (`GameDataManager::minas`) | coleta, com as recusas e limites de `npcgenerator.cpp:1280-1365` (distância 4–20, coletores 1–20, probabilidades somando 1) |
| `MONSTER_ESSENCE` | `monstros.rs` | atributos, resistências, raios de ódio/visão, `patroll_mode`, `aipolicy_id`, dinheiro e drop (`probability_drop_num0..3`, `drop_times`, `drop_matters[32]`) |
| `NPC_ESSENCE` | `GameDataManager::ids_de_npc` | decide NPC × monstro de um spawn (como `gs/npcgenerator.cpp:79,415`) |
| `NPC_ESSENCE` → `NPC_TASK_OUT_SERVICE`, `NPC_TASK_IN_SERVICE`, `NPC_SKILL_SERVICE` | `servicos.rs` (`servicos_de_npc`) | que missões o NPC entrega/recebe e que habilidades ensina, ordenadas para busca binária (`general_id_provider`) |
| todas as tabelas com `pile_num_max` | `servicos::pilhas` (`limite_de_pilha`) | empilhamento na bolsa |
| `PLAYER_LEVELEXP_CONFIG` (id 202), `PARAM_ADJUST_CONFIG`, `PLAYER_SECONDLEVEL_CONFIG` | `progressao.rs` | curva de exp, ajuste por diferença de nível (exp, SP, dinheiro, item), perda na morte por cultivo (`playertemplate.cpp:311-419`) |
| `CHARRACTER_CLASS_CONFIG` | `classes.rs` | velocidades, cadência, alcance, regeneração — **sobrescrevem o `ptemplate.conf`** (`gs/playertemplate.cpp:293-301`) |
| `MEDICINE_ESSENCE` | `quanto_o_remedio_restaura`, `quanto_o_remedio_restaura_no_tempo`, `tipo_maior_do_remedio` | poções. O `id_major_type` é quem dá a classe do item (`set_to_classid`, `gs/template/setclassid.cpp:81-101`): **1794** cura, **1802** mana, **1810** vida e mana, **1815**/**2038** antídotos — e com ela a família de recarga do `cool_time` (B71). O leitor tipado do 1.2.6 não traz o campo |
| 25 tabelas com `price` + `shop_price` | `precos.rs` | preço de loja `max(shop_price, price)`; durabilidade de fábrica |
| `WEAPON_SUB_TYPE` | `armas.rs` (`velocidade_em_ticks`) | cadência da arma: `(int)(attack_speed × 20 + 0,1)` (B52) |
| `QUIVER_ESSENCE` | `armas.rs` (`GameDataManager::aljavas`) | drop de aljava vira munição (B52) |
| `MONSTER_ESSENCE.size` | `monstros.rs` (`tamanho`) | corpo do alvo no alcance de golpe e habilidade (B52) |
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
- Estado: **14.885/14.885** (`realm_155`, cliente BR; o EN fechou 14.978/14.978 antes de sair). Versões 55 e 124: só cabeçalho.
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
  `m_bClearAcquired`, `m_ulGivenCmnCount/TskCount`, `m_ulAwardType_S/F`, …), `dps`/`dph` do
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
  `fOffsetTrn`/`fHeiOff` (zero em 18.902 de 18.903 geradores), `fOffsetWater` (guardado, sem
  mapa de água), `iPathID`, `iSpeedFlag`, contagens sem teto inventado.
- Altura (`SpawnInstance::altura_resolvida`, `gs/npcgenerator.cpp:4296-4346`): área no chão
  → `chão + offset`; área em caixa → `max(y sorteado, chão) + offset`; sem `.hmap` → `y` do
  arquivo.
- Id de recurso já vem com `0xC0000000` (`ISMATTERID` do cliente).
- `id_ctrl != 0` **não** desliga a área: a maioria dos controladores é o registro normal de
  NPC permanente e nasce ativa (B17a). Gatilhos de evento não são modelados.

### 3.4 `aipolicy.data` — IA de criaturas (`aipolicy.rs`)

Porta de `cgame/gs/ai/policy.cpp` (`CPolicyDataManager::Load` e derivados), autoridade o
fonte **1.7.2**. Cabeçalho `F_POLICY_EXP_VERSION` (1.5.5 = 1, 3.144 políticas; 1.2.6 = 0,
293). Parâmetros de condição trazem tamanho no fio; os de operação usam `sizeof` do alvo
32 bits (tabela `tamanho_do_parametro`). **Lido; sem intérprete no mundo.**

### 3.5 `ptemplate.conf` (`ptemplate.rs`)

Seções por classe (`[SWORDSMAN]`, `[ORGE]`, `[ASN]`, `[ANGEL]`…), **GBK**. Lido, mas quase
nada dele chega ao jogador: o `gamed` copia a ficha do banco (`userlogin.cpp`), que nasce do
`clsconfig`. **Não** são usados os atributos (Arqueiro 15/20/5/10 aqui; 5/5/5/5 no molde)
nem o `hp`/`mp`: `max_hp = lvlup_hp×(nível−1) + vit_hp×vitalidade`, base zero (B51 — antes
somava o `hp` do `.conf` e o Arqueiro nascia com 20 de energia). As velocidades dele também
são mortas. `[TOWN_REGION]` é o mapa de ressurreição, não o nascimento.

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

### 3.6c Arquivos do realm que este servidor **não** lê

Estão na pasta do realm e não são pendência nossa, com uma exceção:

| arquivo | de quem é |
| :--- | :--- |
| `task_npc.data` | **do cliente** (`LoadNPCInfoFromPack("data\task_npc.data")`, `Task/EC_TaskInterface.cpp:166`) |
| `DynamicObjects.data` | **do cliente** (`m_pDynObjPath->Load("configs\DynamicObjects.data")`, `EC_Game.cpp:612`) |
| `domain.data`, `domain2.data`, `domain2_cross.data` | não aparecem no fonte do `gs` nem no do cliente; provavelmente do `gdeliveryd` (domínios e cruzamento entre servidores) — fora do alvo atual |
| `extra_drops.sev` | não aparece em nenhum dos dois fontes que temos; origem desconhecida |
| `globalcontroller.conf` | **é do `gs`** (`worldmanager.cpp:1106`) e traz `cash_money_exchange_rate` (1.000.000 no `realm_155`). `falta`, mas só importa quando a **Loja Gold** existir |
| `airmap/`, `movemap/`, `path.sev`, `map.bht`, `.dhmap`/`.rmap` | navegação e colisão do `path_finding`; o mundo hoje não faz pathfinding de criatura por malha |

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

### 3.8 `gshop*.data` (`gshop.rs`)

Aceita o nome do cliente (`gshop.data`, `gshop1.data`, `gshop2.data`) ou do servidor
(`gshopsev.data`, `gshopsev1.data`, `gshopsev2.data`). Só o carimbo (primeiro `u32`) de
cada um é usado, para o `edition` (spec 02 §3.3). O corpo
não é interpretado — a Loja Gold não existe no servidor.

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

### 3.10b Habilidades do servidor — `specs/habilidades_155/habilidades.json` (`habilidades.rs`)

Não é arquivo do realm: é extraído dos stubs `cskill/skills/skillNNN.h` do `EvolvedPWServer`
por `specs/habilidades_155/extrair_habilidades.py` e embutido com `include_str!` (o
`Dockerfile.core` copia o JSON). 3.316 habilidades; por nível: mana, `GetExecutetime`,
`GetCoolingtime`, `GetRequiredLevel/Sp/Money`, `GetTime` de cada estado, e desde o B52
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
