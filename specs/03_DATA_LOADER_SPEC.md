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
| estado | **231/231** tabelas no v156 do 155BR, **234/234** no v159 do 155; fecha no último byte (`NaoTerminaNoFim` se não) |
| fallback | versão fora do catálogo (1.2.6 = v7) vai para o leitor tipado antigo `elements.rs` — é a causa das 2 falhas do `loader_tests` |
| sem catálogo | v181 (cliente build 2591): `.cfg` já copiado, falta `generate_v181.py` |

Consumidores no mundo (os demais índices estão lidos e **não ligados**):

| tabela | módulo | uso |
| :--- | :--- | :--- |
| `WEAPON_ESSENCE`, `ARMOR_ESSENCE`, `DECORATION_ESSENCE` | `armas.rs`, `armaduras.rs` (`TabelasDeEquipamento`) | bloco de dados do item no `OWN_ITEM_INFO`; famílias disjuntas por id |
| `MONSTER_ESSENCE` | `monstros.rs` | atributos, resistências, raios de ódio/visão, `patroll_mode`, `aipolicy_id` |
| `NPC_ESSENCE` | `GameDataManager::ids_de_npc` | decide NPC × monstro de um spawn (como `gs/npcgenerator.cpp:79,415`) |
| `CHARRACTER_CLASS_CONFIG` | `classes.rs` | velocidades, cadência, alcance, regeneração — **sobrescrevem o `ptemplate.conf`** (`gs/playertemplate.cpp:293-301`) |
| `MEDICINE_ESSENCE` | `quanto_o_remedio_restaura` | poções |
| 25 tabelas com `price` + `shop_price` | `precos.rs` | preço de loja `max(shop_price, price)`; durabilidade de fábrica |
| **não ligadas** | — | `PLAYER_LEVELEXP_CONFIG` (curva de exp), `MINE_ESSENCE` (colheita), `WEAPON_SUB_TYPE` (`attack_speed` da arma), `NPC_SKILL_SERVICE` / `SKILLTOME_ESSENCE` (custo de aprender) |

### 3.2 `tasks.data` — missões (`tasks.rs`)

- `TASK_PACK_HEADER` (`magic`, `version`, `item_count`) + tabela de `item_count`
  deslocamentos `u32` + cada missão de topo por `ATaskTempl::SaveBinary`, com submissões
  recursivas. **Cada missão tem de terminar onde a seguinte começa** (`TasksError::Desalinhado`).
- Versão 129 (1.5.5): bloco fixo **1.157** bytes, prêmio **290** — o fonte (versão 125) dá
  1.087/269; a diferença são os campos do sistema de Lar (nomes do 1.7.2), mais o vetor
  `m_ulHomeItemsWanted × 8 bytes` (B45).
- Estado: **14.885/14.885** (155BR), **14.978/14.978** (155). Versões 55 e 124: só cabeçalho.
- Extraído: id, nome/descrição (XOR pelo id), mãe/filhas, tipo, prazo, níveis, classes,
  gênero, pré-requisitos, itens pedidos/entregues, NPCs de entrega e prêmio, objetivos
  (monstros com item, itens, dinheiro, nível, mundo, espera), flags e os dois prêmios.
  Atravessado sem guardar: diálogos, expressões, regiões, prêmios por escala, requisitos
  de equipe/título/Lar.
- **O mundo ainda não consulta nenhuma missão.**

### 3.2b `dyn_tasks.data` — missões dinâmicas (`dyn_tasks.rs`)

Só o cabeçalho: `DYN_TASK_PACK_HEADER` (`task/TaskTemplMan.cpp:45-51`), 12 bytes —
`pack_size u32`, `time_mark i32`, `version u16`, `task_count u16`. Recusa como o
`UnmarshalDynTasks`: `version != 10` ou `pack_size` diferente do tamanho do arquivo. Nos dois
realms 1.5.5: 12.979 bytes, marca `0x52776c0d`, 28 missões. Uso:
`GameDataManager::marca_das_missoes_dinamicas`, a resposta ao pedido de marca do cliente
(spec 04 §5). `falta`: as missões do pacote e o envio do pacote (`TASK_SVR_NOTIFY_DYN_DATA`).

### 3.3 `npcgen.data` — onde as coisas nascem (`npcgen.rs`)

Autoridade: `cgame/gs/template/npcgendata.h/.cpp`. Um arquivo por pasta de mapa.

| bloco | struct | bytes |
| :--- | :--- | ---: |
| cabeçalho | `NPCGENFILEHEADER7` (v≥7); `HEADER`/`HEADER6` com 2–3 inteiros em v<7 | — |
| área de IA | `NPCGENFILEAREA7` (v≥7) / `NPCGENFILEAREA` (v<7, 59) | 71 |
| gerador de IA | `NPCGENFILEAIGEN` (v≥11, com `iRefreshLower`) / `...AIGEN10` | 64 / 60 |
| área de recurso | `NPCGENFILERESAREA7` | 42 |
| objeto dinâmico | `NPCGENFILEDYNOBJ10` | 24 |
| controlador | `NPCGENFILECTRL8` | 199 |

- 1.5.5 é **v11**. Lido inteiro, fechando no fim.
- Campos usados: tipo de área (`iType`: no chão / na caixa), `vExts` (**tamanho** da caixa),
  `fOffsetTrn`/`fHeiOff` (zero em 18.902 de 18.903 geradores), `fOffsetWater` (guardado, sem
  mapa de água), `iPathID`, `iSpeedFlag`, contagens sem teto inventado.
- Altura (`SpawnInstance::altura_resolvida`, `gs/npcgenerator.cpp:4296-4346`): área no chão
  → `chão + offset`; área em caixa → `max(y sorteado, chão) + offset`; sem `.hmap` → `y` do
  arquivo.
- Id de recurso já vem com `0xC0000000` (`ISMATTERID` do cliente).
- `id_ctrl != 0` **não** desliga a área: a maioria dos controladores é o registro normal de
  NPC permanente e nasce ativa (B17a). Gatilhos de evento não são modelados.
- `falta`: 9 zonas do 1.2.6 com v5/v6 com conteúdo.

### 3.4 `aipolicy.data` — IA de criaturas (`aipolicy.rs`)

Porta de `cgame/gs/ai/policy.cpp` (`CPolicyDataManager::Load` e derivados), autoridade o
fonte **1.7.2**. Cabeçalho `F_POLICY_EXP_VERSION` (1.5.5 = 1, 3.144 políticas; 1.2.6 = 0,
293). Parâmetros de condição trazem tamanho no fio; os de operação usam `sizeof` do alvo
32 bits (tabela `tamanho_do_parametro`). **Lido; sem intérprete no mundo.**

### 3.5 `ptemplate.conf` (`ptemplate.rs`)

Seções por classe (`[SWORDSMAN]`, `[ORGE]`, `[ASN]`, `[ANGEL]`…), **GBK**. Usado para:
atributos iniciais por classe e a base de vida/mana
(`max_hp = hp + lvlup_hp×(nível−1) + vit_hp×vitalidade`, idem mana). As velocidades dele são
valores mortos no original. `[TOWN_REGION]` é o mapa de ressurreição, não o nascimento.

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

### 3.7 `region.sev` / `precinct.sev` (por mapa)

`REGIONFILEHEADER4` (`dwVersion, iNumRegion, iNumTrans, dwTimeStamp`@12) e
`PRECINCTFILEHEADER5` (`dwVersion, iNumPrecinct, dwTimeStamp`@8) — `el_region.h`,
`el_precinct.h`. Só o carimbo é lido, por mundo, para o `INST_DATA_CHECKOUT`. Versões < 4
recusadas. Carimbo que não bate com o `.clt` do cliente naquela zona gera erro visível no
cliente (B25).

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
  `scripts/2026_09_12_nascimento_no_mapa_161_155br.sql` (todos no mapa 161, pacote
  `pwserver_155v156`).
- `falta`: `config_data` (provável barra de atalhos), inventário, equipamento, habilidades.

### 3.11 `global_api.lua`

Primeira linha `--<N>` = `lua_version` do `SERVER_TIME`. 1.5.5: **102**; divergência faz o
cliente encerrar ("wrong config data").

## 4. Dados de cada realm 1.5.5

| realm | base do servidor | `.data` do cliente | elements | tasks |
| :--- | :--- | :--- | :--- | :--- |
| `realm_155BR` | `F:\PW\1.5.5\home155\gamed\config` | cliente BR | v156 (55.442.775 B) | 129 |
| `realm_155` | pacote anterior ao home155 (com `a61/a63/a76/a77` modificados, B12) | cliente EN | v159 (55.170.911 B) | 129 |

O pacote de servidor e os `.data` do cliente precisam ser da mesma família: `npcgen.data`,
mapas e `.sev` **não vêm do cliente** (B11, B12).
