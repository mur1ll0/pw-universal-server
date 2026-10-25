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
`rare_item.conf` (os `.clt` e o `task_npc.data` são do **cliente**: o Estúdio os lê e grava — `clt.rs`, `task_npc.rs`,
B246/B251, spec 07 §6b). Na raiz do `realm_126`, `npcgen.data`/`precinct.sev` são cópias idênticas das
de `world/` (as lidas); o `region.sev` da raiz (6.856 B) difere do de `world/` (8.860 B), e vale o
de `world/`. `realm_155`: **sem falhas** desde o B211 (as 3 eram `a46/npcgen.data` e `a50/precinct.sev`
com 0 bytes no `home155`; vieram do `pwserver_155v156`).

## 3. Os arquivos

### 3.1 `elements.data` — itens, criaturas, classes, configurações

| | |
| :--- | :--- |
| leitor | `generic_elements.rs` (Rust) e `specs/elements_layouts/pw_elements_reader.py` (painel) — mesmo algoritmo |
| cabeçalho | `u32 versão` (`0x30000007` = v7, `0x3000009c` = v156, `0x3000009f` = v159); `time_t` só nos builds recentes medidos |
| corpo | cada tabela: `u32 count` + `count × sizeof(T)`, na ordem do `elementdataman::load_data` do cliente (`EvolvedPWClient/.../elementdataman.cpp:3879`) |
| exceções | v156/v159: depois de `ARMORRUNE_ESSENCE`, `tag 0xab7689dd`, `len`, bytes da máquina exportadora e `time_t`; depois de `WAR_TANKCALLIN_ESSENCE`, `tag 0xee35679f`, `len`, bytes. v7: sem esses blocos. Em todos: `TALK_PROC` com laço próprio |
| layouts | `specs/elements_layouts/v7.json` (gerado por `generate_v7.py` a partir do `gs` 1.2.6), `v156.json`, `v159.json` (ADMVAL); embutidos com `include_str!` |
| tabelas por versão | o v7 não tem nenhuma tabela `GOBLIN_*` (o 1.2.6 não tem Daimon); o v156/v159 tem `GOBLIN_ESSENCE` — `GameDataManager::versao_tem_daimon` (B218, `tests/carga_dos_realms.rs::so_o_155_tem_daimon`) |
| estado | **v7: 119 entradas, 23.447 registros, 16.664.770 bytes**, fecha no último byte no Rust e Python; v156: 231/231 tabelas no `realm_155`; v159: 234/234 quando ainda estava no projeto (B55). Arquivo com byte extra é recusado (`NaoTerminaNoFim`) |
| v7 | **Ordem, nomes e `sizeof` das 118 tabelas fixas: do `gs` 1.2.6** (`files1.2.6/pwserver/gamed/gs`, ELF com símbolos, que exige `0x30000007` em `elementdataman::load_data`, VA 0x81b1e3d): a sequência de `elementdataman::array<T>::load` e o imediato de tamanho de cada uma (B100). Fechar no último byte **não basta**: os tamanhos "medidos" do B94 para as tabelas 98–112 também fechavam, com contagens erradas. Cliente 1.2.6, `elementclient.exe` VA `0x60ff5d-0x60ffb0`: confirma cabeçalho de 4 B e a primeira tabela de `0x54` B. Campos partem do v156 pelo nome e são cortados no tamanho do v7 — **onde o v156 inseriu campo no meio isso desloca tudo**, e cada caso precisa de evidência: `NPC_TASK_OUT_SERVICE` = ID + Name + `id_tasks[32]` (sem os 7 `storage_*`; `gs` `npc_stubs_manager::LoadTemplate` VA 0x80ef014); `NPC_SKILL_SERVICE` = `id_skills[128]` + `id_dialog` (VA 0x80ef202); `NPC_TASK_IN_SERVICE` = `id_tasks[32]` (VA 0x80eef40); `NPC_ESSENCE`: os 16 ids de serviço que o `gs` lê batem (+760 task_out … +836 equipundestroy); `NPC_TRANSMIT_SERVICE`: `num_targets` +68, destinos de 12 B (conferido); `WEAPON`/`ARMOR`/`DECORATION_ESSENCE` sem `fixed_props`/`probability_hidden`/ocultos (armadura 139: defesa 552, preço 4.800/9.600); `MINE_ESSENCE`: materiais de 8 B e `npcgen_1` em +388; voo: sem `file_model2`; `PET_ESSENCE`: sem `damage_d`; `MONSTER_ESSENCE`: sem `attack_degree`/`defend_degree`, drop em +1220 (B94). Conferidos no consumidor do `gs` 1.2.6 (B122): `CHARRACTER_CLASS_CONFIG` — os 23 campos de 0x44 a 0x9c na ordem do `playertemplate.cpp` 1.5.5 (`player_template::__LoadDataFromDataMan`, VA 0x80e67d2; o 1.2.6 não tem `spirit_adjust`/`poker_adjust`); `PARAM_ADJUST_CONFIG` — `level_diff_adjust[16]` de 24 B em +0x44, `team_adjust` em +0x1c4 e `team_profession_adjust` em +0x21c, de 8 B; `PLAYER_SECONDLEVEL_CONFIG` — `exp_lost[256]` em `+0x44 + 4·i`; `STONE_ESSENCE` — `price` +0x150, addons +0x160/+0x164, `pile_num_max` +0x1a8 (`generate_stone`). **Corrigidos (B122):** `TASKDICE_ESSENCE` = `task_lists[8]` (0x144..0x184), `use_on_pick`, `pile_num_max` +0x188, `has_guid` +0x18c e `proc_type` +0x190 (`generate_taskdice`, VA 0x81f08a0; o corte do v156 punha 10 listas e as duas últimas eram esses campos — a Carta da Sorte podia sortear missão de id `pile_num_max`); `MINE_ESSENCE` = `npcgen[3]` + `aggros[1]` `{monster_faction, radius, num}` + `permenent` nos 64 B de +0x184 (o fonte 1.5.5 tem `npcgen[4]`; lido como `npcgen_4`, os 3 registros não nulos davam monstro inexistente e `num` 200.0 em float) — **47 minas permanentes** do 1.2.6 sumiam ao serem colhidas, porque `permenent` estava no `_opaco`. O `generate_v7.py` agora também carrega o `PET_ESSENCE` do B111, que só estava no JSON. **B145:** `RECIPE_ESSENCE` v7 **sem `bind_type`** e sem o rabo de melhoria do v156 (`recipe_manager::LoadTemplate` do `gs` 1.2.6, VA 0x80f0a12: `recipe_level` +0x4c, `id_skill` +0x50, `skill_level` +0x54, alvos a partir de +0x58, `fail_probability` +0x78, `num_to_make` +0x7c, `price` +0x80, `duration` +0x84, `exp` +0x88, `skillpoint` +0x8c, materiais +0x90..+0x190) e `NPC_MAKE_SERVICE` v7 **sem `produce_type`** (1224 = 72 + 8 × 144; lido como v156, o campo saía 0x00730045, texto do título da página). **Sem opacos desde o B143**, pelos valores do `realm_126`: `CUSTOMIZEDATA_ESSENCE` não tem `file_icon` (os 8 B finais são `character_combo_id` 1/2/8/64/128/192 e `gender_id` 0/1); `PLAYER_ACTION_INFO_CONFIG` tem 11 sufixos de arma e `hide_weapon` (0/1); `FACEPILL_ESSENCE` tem 16 `pllfiles` e `price`/`shop_price`/`pile_num_max`/`has_guid`/`proc_type` (500/2000/10/0/0), a ordem do `exptypes.h:2129-2159` |
| fallback | Só versão fora do catálogo vai ao leitor tipado antigo `elements.rs`; o v7 passou ao genérico. Não há mais tabela `V7_OPACA_*`: as 118 têm nome pelo `gs` 1.2.6 |
| escrita (B237) | `elements_editavel.rs` (`ElementsEditavel`): mesmo percurso do leitor guardando **bytes** — cabeçalho, os dois blocos de etiqueta do v156 e o `TALK_PROC` crus, cada registro com o `record_size` do layout. Gravar sem editar devolve o arquivo **idêntico** (testado com o `elements.data` do `realm_126`, 16.664.770 B, e do `realm_155`, 55.442.775 B). O `TALK_PROC` também é editável (B240, `conversas.rs`): conversa `{id_talk, text[64], janelas}`, janela `{id, id_parent (-1 raiz), talk_text_len + texto, opções}`, opção `{id, text[64], param}` de 136 B, como o `talk_proc::save`/`load` (`ExpTypes.h:3872`); os textos ficam em unidades UTF-16 cruas e o `talk_text_len` **conta o NUL final** (todas as janelas não vazias dos dois realms terminam em NUL) — regravado idêntico; texto editado muda o tamanho e os dois leitores continuam fechando. Editar troca só os bytes do campo (`int32`/`float`; `wstring` UTF-16LE e `string` GBK completados com zeros; texto que não cabe é recusado); a soma dos campos é o `record_size` nas 119 tabelas do v7 e nas 231 do v156, então não há byte fora de campo. Clonar/remover registro; id novo = maior id do **espaço** + 1 e id repetido no espaço é recusado (o cliente sobrescreve em silêncio, `add_id_index`). Espaços: `specs/elements_layouts/espacos_de_id.json`, das 214 chamadas `add_id_index` de `elementdataman.cpp` (1.5.5): `ESSENCE` 135, `CONFIG` 55, `FACE` 11, `RECIPE` 10, `ADDON` 2, `TALK` 1. **Sem espaço conhecido:** 17 tabelas do v156 (as 15 `HOME_*`, `RED_PACKET_PAPER_ESSENCE`, `WISH_TRIBUTE_ESSENCE` — não estão no fonte 1.5.5 e o fonte 1.7.2 não está mais em disco): editar vale, criar registro é recusado |
| sem catálogo | v181 (cliente build 2591): `.cfg` já copiado, falta `generate_v181.py` |

Consumidores no mundo (os demais índices estão lidos e **não ligados**):

| tabela | módulo | uso |
| :--- | :--- | :--- |
| `WEAPON_ESSENCE`, `ARMOR_ESSENCE`, `DECORATION_ESSENCE`, `PROJECTILE_ESSENCE` | `armas.rs`, `armaduras.rs` (`TabelasDeEquipamento`), `addons.rs` (`ModeloDeGeracao`: faixas, furos de drop e de fabricação, `addons`/`rands`/`uniques`) | bloco de dados do item no `OWN_ITEM_INFO` e geração (drop, fabricação, loja); famílias disjuntas por id. **v7 (B151, `generate_weapon/armor/decoration<NORMAL>` do `gs` 1.2.6, VA 0x81f4eae/0x81f57a4/0x81f6026):** sem `require_reputation`; arma `level` +0x268 e `fixed_props` +0x26c; **4** `probability_addon_num` nos três (v156: 6/5/5); 16 únicos na arma; armadura sem `id_hair`/`force_all_magic_defences`, com `pile_num_max`/`has_guid`/`proc_type` no fim. Teste `a_geracao_de_equipamento_do_126`: todos os modelos do realm com `addon_num` somando 1. `weapon_level` = `level` da arma (B51; era 1 fixo — o Arco de Madeira é 0); munição com `IVTR_ESSENCE_ARROW` (tipo, dano extra, faixa de nível da arma) |
| `MINE_ESSENCE` | `minas.rs` (`GameDataManager::minas`) | coleta, com as recusas e limites de `npcgenerator.cpp:1280-1365` (distância 4–20, coletores 1–20, probabilidades somando 1). O v7 não expõe `material_gain_ratio`; até medir a regra antiga, a chance usada vem da soma das probabilidades do próprio registro (=1 após validação); falta confirmar em jogo |
| `MONSTER_ESSENCE` | `monstros.rs` | atributos, resistências, raios de ódio/visão, `patroll_mode`, `aipolicy_id`, dinheiro e drop (`probability_drop_num0..3`, `drop_times`, `drop_matters[32]`); `local_var[3]` (`param1..3` no layout v156, B127); `id_strategy`, `skills[32]` e `skill_hp75/50/25` — **as 5 entradas de cada limiar, com as de id 0** (B126: o `RandSelect` do evento precisa das posições; `monstros::sortear_evento_de_vida`) |
| `NPC_ESSENCE` | `GameDataManager::ids_de_npc` | decide NPC × monstro de um spawn (como `gs/npcgenerator.cpp:79,415`) |
| `NPC_ESSENCE` → `NPC_TASK_OUT_SERVICE`, `NPC_TASK_IN_SERVICE`, `NPC_SKILL_SERVICE` | `servicos.rs` (`servicos_de_npc`) | que missões o NPC entrega/recebe e que habilidades ensina, ordenadas para busca binária (`general_id_provider`) |
| `NPC_ESSENCE` → `NPC_RESETPROP_SERVICE` (B152) | `servicos.rs` (`restauracao_de_atributos`) | 15 × `{id_object_need, strength, agility, vital, energy}` (20 B a partir de +0x44, 368 B nas duas versões; `id_resetprop_service` em +0x32c do `NPC_ESSENCE` v7, conferido no `gs` 1.2.6 VA 0x80ef74b); sem item ou com delta negativo fica fora (`npcgenerator.cpp:762-778`). Serviços 10226/10227: as 15 opções, a última o 12764 (100 em cada) |
| `NPC_ESSENCE` → `NPC_PETNAME_SERVICE`, `NPC_PETFORGETSKILL_SERVICE`, `NPC_PETLEARNSKILL_SERVICE` (B112) | `servicos.rs` (`renomear_mascote`, `esquecer_habilidade_de_mascote` = `(price, id_object_need)`; `habilidades_de_mascote` ordenadas) | serviços 36/37/38 (`npcgenerator.cpp:783-826`). Nos dois realms a Rilay (11534) cobra 0 moedas e o item 12403 (renomear) / 11690 (esquecer) |
| todas as tabelas de item (`pile_num_max`, ou `price` + `shop_price` — B216) | `servicos::pilhas` (`limite_de_pilha`) | empilhamento na bolsa |
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
| `STONE_ESSENCE` | `manager.rs` (`pedras`) | addon da pedra na arma/armadura/acessório (B53) |
| `npcgen.data` `bAutoRevive` (área e área de recurso) | `SpawnInstance::renasce` | falso = não renasce (`SetRespawn`, B205); `examples/sem_renascer.rs` lista por mapa; `tests/renascer_do_npcgen.rs`: a69 38/38 sem renascer, a01 todo renasce |
| `FACETICKET_ESSENCE` (`require_level`) | `GameDataManager::bilhetes_de_rosto` | o bilhete da troca de rosto (B199; `tests/bilhetes_de_rosto_do_realm.rs`: 126 = 3 — 4411, 5382, 9709 —, 155 = 4, todos nível 0) |
| `proc_type` de todo registro com `ID` | `GameDataManager::proc_types`, `proc_type_da_instancia`, `vincula_ao_vestir` | o valor inicial da instância (`gs/item.cpp:95-105`); vinculada (`bind_status`) = `\| NODROP\|NOTHROW\|NOSELL\|NOTRADE\|BIND`, `& !BIND2` (`player.cpp:8182-8191`) (B201) |
| `require_reputation` de `WEAPON/ARMOR/DECORATION_ESSENCE` | `GameDataManager::reputacao_exigida` | `get_item_reputation_limit` (`itemdataman.cpp:1911-1930`); o v7 (1.2.6) não tem o campo (B202). O `id_major_type` da arma é o `WeaponClass` das passivas de arma (1, 5, 9, 13, 182 nos dois realms; 23749, 25333, 44878, 44879 no 155) (B203) |
| essências que entram no corpo + `ARMOR/DECORATION/FASHION_SUB_TYPE` (`equip_mask`, `equip_fashion_mask`), `POKER_SUB_TYPE` (`equip_mask_1/2`) | `posicoes.rs` (`GameDataManager::posicoes`, `cabe_no_slot`) | `equip_mask` de 32 bits por item como o `generate_*` grava (`generate_item_temp.h`, constantes em `itemdataman.h:340-366`; essência → gerador em `itemdataman.cpp:1355-1605`), conferido por `CheckEquipPostion` (B191, `testado` em `tests/posicoes_no_corpo.rs`: 126 = 3.582 itens com posição, armaduras 1.036/1.036, acessórios 561, roupas 142; 155 = 11.034, armaduras 2.520 — 156 delas abrem o slot 22 `EQUIP_INDEX_TWEAK` —, acessórios 969, roupas 3.016) |
| `WEAPON/ARMOR/DECORATION_ESSENCE` (`level`, `levelup_addon`, `material_need`, `id_sub_type`), `STONE_ESSENCE` (grau, cor, `install_price`, `uninstall_price`, addons), `REFINE_TICKET_ESSENCE`, `DECORATION_SUB_TYPE.equip_mask`, `EQUIP_MAKE_HOLE_CONFIG` 2013 | `refino.rs` (`GameDataManager::refino`) | refino, incrustar, remover pedras e furar (B163, `testado` em `tests/refino_do_realm.rs`: 155 = 190 pedras, 24 só de acessório, 55 talismãs, 5.101 refináveis; 126 = 125 pedras, 19 talismãs, 2.709 refináveis, sem a tabela de furos e sem o item 21043). Todo `levelup_addon` dos dois realms tem tratador `refine_*` |
| `NPC_ESSENCE` → `NPC_TASK_MATTER_SERVICE` (B164) | `servicos.rs` (`itens_de_missao`, `missoes_com_item`) | serviço 8: 16 × `{id_task, 4 × {id_matter, num_matter}}`; 155 = 18 NPCs/21 missões, 126 = 14/16 (`tests/servicos_do_realm.rs`) |
| `PARAM_ADJUST_CONFIG` → `team_adjust`, `team_profession_adjust` (B164) | `progressao.rs` (`bonus_de_equipe`) | bônus de equipe por membros e por classes (/20); 1,0 / 1,2 / 2,1 para 1/2/6 membros nos dois realms |
| `NPC_ESSENCE` (`id_install_service`, `id_uninstall_service`, `combined_services & 0x4000`) | `servicos.rs` (`incrustar`, `remover_pedras`, `refinar`) | quem oferece os serviços 10, 11 e 35 (`npcgenerator.cpp:596-604`, `:975-979`). **No layout v7 o campo em `+0x348` se chamava `id_goblin_skill_service` (nome herdado do v156) e foi renomeado para `combined_services` no B163**: o `gs` 1.2.6 testa nele os bits 0x400–0x8000 (`npc_stubs_manager::LoadTemplate`, VA 0x80eff76-0x80f0020) |
| **não ligadas** | — | `NPC_SELL_SERVICE` (lista de venda), `NPC_TRANSMIT_SERVICE` (teleporte por NPC) |

### 3.2 `tasks.data` — missões (`tasks.rs`)

- `TASK_PACK_HEADER` (`magic`, `version`, `item_count`) + tabela de `item_count`
  deslocamentos `u32` + cada missão de topo por `ATaskTempl::SaveBinary`, com submissões
  recursivas. **Cada missão tem de terminar onde a seguinte começa** (`TasksError::Desalinhado`).
- Versão 129 (1.5.5): bloco fixo **1.157** bytes, prêmio **290** — o fonte (versão 125) dá
  1.087/269; a diferença são os campos do sistema de Lar (nomes do 1.7.2), mais o vetor
  `m_ulHomeItemsWanted × 8 bytes` (B45).
- Estado: **14.885/14.885** (`realm_155`, cliente BR; o EN fechou 14.978/14.978 antes de sair). Versão 124: só cabeçalho. Na v55 (1.2.6), o bloco fixo tem **534 B**, o prêmio **75 B**, o item **13 B** e o monstro pedido **22 B** (`elementclient.exe` VA `0x62f6c0`, leitura em `0x62d04d`; `docs/RESULTADO_TASKS_V55.md`). O leitor Rust fecha **2.819/2.819 raízes**, **7.994** tarefas recursivas e o último byte; rejeita corrupção na tabela ou um byte extra (B96).
- Prêmio `m_ulPetInventorySize` (vagas da jaula, `TaskReward::vagas_na_jaula`, B153): **+53** no v129 (`TaskTempl.h:1149`, antes do `m_ulFuryULimit` +57) e **+36** no v55 (`libtask.so` 1.2.6, `DeliverByAwardData` 0xb48c: `award[0x24]` → vtable +0xc4 = `SetPetInventorySize`; o +40 → +0xbc = `SetFuryUpperLimit` confere). Teste `as_missoes_que_ampliam_a_jaula` (3327–3330 = 2…5, 8990 = 10, nos dois realms).
- Na v55, `TaskTemplate` recebe id/nome/descrição, hierarquia, horários, itens, monstros, prêmio básico, NPCs, classes, método e conclusão, e desde o B102 as flags `0x6a`–`0x74` (escolhe uma filha, sorteia, filhas em ordem, pai também falha/sucesso, pode desistir, pode repetir, refazer após falha, limpa ao desistir, precisa registro, falha ao morrer), pela sequência `pack(1)` do `TaskTempl.h:2037-2057` e conferidas na `libtask.so` 1.2.6 (`CheckDepth` testa +0x6c/+0x6a/+0x6b; `CanGiveUpTask` lê +0x6f). Sem elas a 1177 ativava as duas filhas juntas (o original entrega só a 1178) e a 1173 deixava de pedir a filha escolhida (1175 ou 1176). Desde o B107 também `m_bAutoDeliver` +0xac (e, lido desde o B157 como `entrega_ao_morrer`, `m_bDeathTrig` +0xad — no v129 o byte 230, entre `m_bUsedInTokenShop` e `m_bClearAcquired` 231; 990 no `realm_126`, 990 e 29649 no `realm_155`), zona de entrega +0x79/+0x7a com a caixa +0x7e/+0x8a, nível +0xc1/+0xc5, pré-missões (contador +0xf1, vetor de **5** em +0xf5) e gênero +0x114, medidos no `libtask.so` 1.2.6 (`AddOneTaskTempl`, `CheckInZone`, `CheckLevel`, `CheckPreTask`, `CheckGender`, todos com base `this + 4` = bloco fixo): 81 missões automáticas, entre elas a 9376 "Virando Dinossauro" (nível 1..150, sem classe nem pré-requisito); antes nenhuma. Dados do próprio arquivo: a 949 pede a 947 inexistente, e dez "Teste de Liu Weijun" têm mínimo acima do máximo. Desde o B110 também o lugar a alcançar (método 4): mundo +0x1de e caixa +0x1c6/+0x1d2 (`OnTaskReachSite` do `libtask.so` 1.2.6, que no 1.2.6 não confere o tipo de conclusão); 1.559 missões, nove "Estágio 3-x" (4735-4770) com a caixa invertida no próprio arquivo. Desde o B122, cada campo que o motor usa pela função do `libtask.so` 1.2.6 que o lê (base `this + 4`): prazo +0x49 (`RecursiveCheckTimeLimit`), `m_lAvailFrequency` +0x66 (`CheckDeliverTime`; o v55 não tem `m_lPeriodLimit`), `m_ulMaxReceiver` +0x75 (`CheckGlobalRequired`), teleporte ao receber +0x96/+0x97/+0x9b (`CheckDeliverTask`, que também lê `m_lMonsCtrl` +0xa7 e `m_bTrigCtrl` +0xab), `m_bShowPrompt` +0xb3 e `m_bKeyTask` +0xb4 (ordem do `TaskTempl.h` entre +0xae e +0xb5), casamento +0xc0 (`CheckMarriage`), entregues comuns/de missão +0xd7/+0xdb (`CheckGivenItems`), depósito +0xe3 (`CheckDeposit`), reputação +0xe8 (`CheckRepu`), período +0x10a (`CheckPeriod`), facção +0x10f (`CheckFaction`), cônjuge +0x13e, GM +0x140, exclusivas +0x149 com a lista de 5 em +0x14d (`CheckMutexTask`; até +0x171 cabem `m_ulMutexTasks[5]` + `m_lSkillLev[4]`), equipe +0x176/+0x177 (`CheckTeamTask`), `m_ulGoldWanted` +0x1b2 (`HasAllItemsWanted`), espera +0x1e2 (`CheckWaitTime`) e os tipos de prêmio +0x1e6/+0x1ea (`CalcAwardData`). Medido no `realm_126`: 189 diárias e 11 semanais, 118 com prazo de 10 h, 92 com espera de 10 min, 121 de período 20/30, 18 pedindo 1.000.000 de ouro, 2 de casamento, 3 de GM, nenhuma teleportando ao receber (`tasks_do_realm.rs`). `m_bHidden`, `m_lPeriodLimit` e os limites de conta/personagem não existem no v55; Desde o B143, `m_ulType` em +0x45 (0 a 8 no `realm_126`: 7.201 com 0, 298 com 4; a 9376 é 2) — `ID`, nome, `HasSign` +0x40 e o ponteiro +0x41 pela ordem do `ATaskTemplFixedData::LoadFixedDataFromTextFile` do `libtask.so` 1.2.6 ("Type", "TimeLimit", "AbsTime"); o byte +0x4d é `m_bAbsTime` (gravado em VA 0x12925 depois do formato "AbsTime: %d"): **o v55 não tem `m_bItemNotTakeOff`**, e a entrega sempre retira os itens de missão. O Guerreiro nível 1 aceita a 1173 "Primeiro Teste" no teste do motor, com NPC 3517, classe 0 e prêmio de 45 moedas/75 exp/20 SP; falta ver em jogo após publicação (B96). A 1177, missão inicial do Guia Selvagem (NPC 3518, serviço 3531 = [1177, 1178]), só é aceita pelas classes 3 e 4 no nível 1 (erro 13 nas outras); com o `NPC_TASK_OUT_SERVICE` corrigido, o teste de mundo com o banco entrega a 1177 a um Bárbaro nível 1 (B100).
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
- Lidos no B164 (`AwardNotifyTeamMem`): `m_bAllFail`/`m_bCapFail`/`m_bCapSucc`/`m_fSuccDist`
  em 683/684/685/686 e `m_bAllSucc` em 701 no v129 (ordem do `ATaskTempl`, fechando com o 691 e o
  702 já conferidos); no v55 em 0x17f/0x180/0x181/0x182 (o `libtask.so` 1.2.6 lê `this+4+…`, o
  mesmo `+4` do `m_bTeamwork` em 0x176) e **sem `m_bAllSucc`**. Só missões de equipe os têm nos
  dois realms (155: 148 capSucc, 141 allFail, 84 capFail, 2 allSucc; 126: 40, 6, 2).
- Para o motor de missões (spec 05 §10) também: as flags de `CheckPrerequisite`/`RecursiveAward`
  (`m_bParentAlsoFail/Succ`, `m_bCanRedoAfterFailure`, `m_bClearAsGiveUp`, `m_lAvailFrequency`,
  `m_bAccountTaskLimit`, `m_bRoleTaskLimit`, `m_bHidden`, `m_bDisplayInTitleTaskUI`,
  `m_bClearAcquired` (no v55 em +0xae, B117 — o `libtask.so` 1.2.6 o testa antes do `RemoveAcquiredItem`), `m_ulGivenCmnCount/TskCount`, `m_ulAwardType_S/F`, …), `dps`/`dph` do
  monstro pedido e `m_bUseLevCo`/`m_bMulti` do prêmio. Deslocamentos em
  `specs/tasks_155/layout129.tsv` e `award_layout.tsv` (sonda do MSVC, B45).
- `profundidade` = `m_uDepth` (`CheckDepth`, `TaskTempl.h:2748-2771`), calculada após a leitura.

**Editável no lugar (B256):** o leitor guarda em cada `TaskTemplate` onde começam o bloco fixo e o prêmio no arquivo
(`bloco_fixo_em`, `premio_em`, fora do JSON). `tasks_editavel.rs` troca **campos de tamanho fixo** sem mudar o tamanho
de nada (a tabela de deslocamentos continua valendo): nome (`wchar[30]` em +4, XOR com `id as u16`), prazo,
frequência, pode desistir/repetir, entrega automática, NPC que entrega/premia, níveis mín./máx., espera e, no
`AWARD_DATA`, dinheiro, exp, missão seguinte, SP e reputação — v129 pelas constantes `v129::f`/`v129::p`, v55 pelos
deslocamentos do 1.2.6 acima. Testado nos dois realms (a 1173): o leitor relê os valores novos, outra missão não muda,
voltar devolve o arquivo idêntico fora do campo do nome. **Textos longos (B262):** o leitor guarda também
`textos_em` (v55 e v129): descrição, aceitar, recusar (`LoadDescriptionBin`) e tributo (`LoadTributeBin`), cada um
`u32 len` (= `wcslen`, **sem** NUL) + `len` unidades UTF-16 em XOR com `(namechar)m_ID` (`TaskTempl.cpp:4816-4883`,
gravação `SaveDescriptionBin`/`SaveTributeBin` `:4940-5010`, `convert_txt` `:629`). `definir_texto` reescreve o texto
e soma a diferença em toda entrada da tabela de deslocamentos que começa depois dele; o arquivo é relido. Testado nos
dois realms: descrição maior e tributo vazio numa missão de topo do meio, texto de aceitar numa submissão; o leitor
relê, a última missão de topo não muda; voltar devolve o arquivo idêntico. **Listas fixas (B263):** pré-missões,
exclusivas e classes não são vetores variáveis — são arrays de capacidade fixa no bloco fixo, com contador
(`tasks::listas_fixas`, uma tabela para o leitor e o editor: v129 `m_ulPremise_Tasks[20]`, `m_Occupations[12]`,
`m_ulMutexTasks[5]`, `TaskTempl.h:30-31`, `:2235-2282`; v55 5/8/5 medidos). `definir_lista` grava contador + valores
e zera as vagas livres — medido: as vagas livres são zero em **todas** as missões dos dois realms. Pré-missão e
exclusiva têm que existir; repetida, a própria missão ou acima da capacidade → erro. **Vetores (B264):** itens
exigidos, itens entregues e monstros a matar são vetores variáveis com o contador no bloco fixo (`tasks::vetores`;
`TaskTemplate::vetores_em` guarda onde cada um começa). `definir_vetor` reescreve o vetor inteiro com
`bytes_do_item`/`bytes_do_monstro` (v129: 17/30 bytes; v55: 13/22, sem validade nem DPS/DPH — valor ≠ 0 nesses campos
→ erro), corrige o contador e a tabela de deslocamentos (`corrigir_tabela`, a mesma dos textos). Teto do cliente:
`MAX_ITEM_WANTED = 10`, `MAX_MONSTER_WANTED = 3` ("limitado pelo `ActiveTaskEntry`", `TaskInterface.h:139-141`) —
medido: nenhuma missão dos dois realms passa disso, e os dois gravadores reproduzem byte a byte **todos** os registros
dos dois arquivos. **Prêmio em itens (B265):** os grupos `AWARD_ITEMS_CAND` vêm logo depois do bloco do prêmio
(sucesso em `premio_em`, falha em `premio_de_falha_em`), cada um `bool m_bRandChoose`, `u32 n` e `n` itens
(`LoadAwardCandBin`, `TaskTempl.cpp:732-760`); o contador `m_ulCandItems` fica no bloco do prêmio
(`tasks::grupos_do_premio`: v129 `p::CAND_ITEMS`, v55 +67). `definir_grupos_do_premio` reescreve o trecho dos grupos,
o contador e a tabela de deslocamentos. Teto: `MAX_AWARD_CANDIDATES = 12` (`TaskTempl.h:36`), `MAX_ITEM_AWARD = 64`
(`TaskInterface.h:142`) — medido: nenhuma missão dos dois realms passa. **Diálogos (B266):** os cinco `talk_proc`
(entrega, não qualificado, entrega de item, execução, prêmio — `TaskTempl.cpp:4736-4746`) vêm logo depois do tributo;
`tasks_editavel::dialogos_da_missao` os lê (`ExpTypes.h`: `id_talk`, `text[64]`, janelas com `id`, `id_parent`,
`talk_text_len` + texto, opções de 136 bytes com `id`, `text[64]`, `param`; tudo em XOR com o id, os fixos nos 64
caracteres — `convert_talk_text` `:637-653`). Fecha: nas missões de topo sem filhas, depois dos diálogos vem só o `i32`
de filhas e a próxima missão (conferido nos dois realms). Medido: `talk_text_len` **conta o NUL** em todas as janelas
(12.502 no 1.2.6, 33.124 no 1.5.5); os `text[64]` vêm zerados depois do NUL em ~97% (resíduo em 1.976/56.304 e
2.511/200.965). `definir_texto_do_dialogo` troca título, texto de janela (tamanho variável, NUL incluído, tabela de
deslocamentos corrigida) ou opção (até 63 caracteres, resto zerado). **Clonar missão (B267):** parentesco no bloco
fixo (`m_ulParent`, `m_ulPrevSibling`, `m_ulNextSibling`, `m_ulFirstChild`, `TaskTempl.h:2461-2464`) localizado nos
dados — o único deslocamento que bate em todas as missões: v129 1131/1135/1139/1143 (a mãe confere com `f::PAI`), v55
518/522/526/530 (`tasks_editavel::parentesco`). Os textos cifrados com o id são só nome (`wchar[30]`), assinatura (se
`+64`), os quatro textos longos e os cinco diálogos (os únicos `convert_txt`/`convert_talk_text` de `TaskTempl.cpp`).
`clonar_missao` copia uma missão de topo **sem submissões** para o fim do arquivo, troca o id, recifra esses textos com
XOR `antigo ^ novo`, zera o parentesco e acrescenta a entrada na tabela (todos os deslocamentos +4). Id novo ≤ 65535:
o cliente guarda o id de missão em `unsigned short` (`TaskProcess.h:115`, `:273-275`). Testado nos dois realms: o
clone tem a ficha, os textos e os diálogos da origem com o id novo; a origem e as outras não mudam. **Remover
(B268):** `remover_missao` tira uma missão de topo e a árvore dela (bytes cortados, entrada da tabela retirada, −4 em
todos os deslocamentos e −tamanho nos que vinham depois); recusa submissão e missão citada por outra fora da árvore
(`citacoes_da_missao`: pré-missão, exclusiva, missão seguinte do prêmio de sucesso ou falha, membro de equipe). As
citações no `elements` são conferidas no `pw-recursos`. Testado nos dois realms: clonar + remover o clone = arquivo
idêntico; uma de topo com filhas, não citada, sai com a árvore e as outras ficam iguais. **Clonar com a árvore
(B269):** `clonar_missao` aceita missão de topo com submissões — ids consecutivos a partir do pedido, em pré-ordem (a
ordem do arquivo), todos livres e ≤ 65535; cada nó tem os textos recifrados com a própria chave, o parentesco
remapeado (a raiz sem mãe nem irmãs) e as pré-missões, exclusivas e missão seguinte (sucesso e falha; prêmio +13 no
v129, +8 no v55) que apontam para dentro da árvore remapeadas. Testado nos dois realms numa missão com netas: mesma
forma de árvore, mesmos textos, diálogos e pré-missões (remapeadas); remover o clone = arquivo idêntico.
**Estrutura dos diálogos (B270):** medido nos dois realms (`medir_ids_dos_dialogos`) — ids de janela sem o bit alto, a
raiz com pai −1, toda janela filha com o pai no mesmo diálogo e aberta por **exatamente uma** opção (3.130 = 3.130 no
1.2.6; 8.306 = 8.306 no 1.5.5); toda opção aponta para uma janela do diálogo ou é função (bit alto + índice do
`SERVICE_TYPE`, `CCommon/ExpTypes.h:36-80`: 0 `NPC_TALK`, 6 `GIVE_TASK`, 7 `COMPLETE_TASK`, 8 `GIVE_TASK_MATTER`, 17
`TALK_RETURN`, 18 `TALK_EXIT`, 21 `TALK_GIVEUP_TASK`); `param` é 0 nas que abrem janela e em `RETURN`/`EXIT`, e quase
sempre o id da missão nas demais. `acrescentar_opcao` (no fim da janela; `NovaJanela` cria a filha no fim do diálogo
com id = maior + 1), `remover_opcao` (leva a janela que ela abre e as descendentes) e `criar_janela_raiz` (id 1, pai
−1, em diálogo vazio) seguem essas convenções e corrigem contadores e tabela. Testado nos dois realms: acrescentar e
remover = arquivo idêntico.

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
- `PET_ESSENCE` (B111): o layout v7 inventava um `pet_snd_type` em 0x154 e deslocava `hp`/`hp_gen`/`damage`; o `gs` 1.2.6 (`pet_dataman::LoadTemplate`, VA 0x8143580) lê `hp_a`…`magic_defence_d` (26 floats, com `damage_d`) a partir de 0x154, depois `size`, `damage_delay`, `attack_range`, `attack_speed`, `sight_range` (int), `food_mask`, `inhabit_type` e um `unk` até os 476 B. `ModeloDeMascote` (`pet.rs`) com as recusas do original: 460 modelos no 1.2.6 (413 de combate), 783 no 1.5.5 (439); `PET_FOOD_ESSENCE` → `comidas_de_mascote`; curva do mascote = `PLAYER_LEVELEXP_CONFIG` 592 (`exp_do_mascote_para_subir`). **B222:** o
  modelo lê também `mp_a`, `mp_gen_a`, `attack_degree_a`, `defence_degree_a` (fórmulas `(int)(a × (nível
  + 5))` do `GenerateBaseProp2`) e `plant_group`/`group_limit` (v156, campos 47–48; `petdataman.cpp:137-138`).
  O exemplo `cargo run -p pw-data-loader --example plantas -- <config> [ovo…]` lista os ovos de planta
  (`id_type` 28913), o grupo, o limite, o habitat, a velocidade e as habilidades — 38 no `realm_155`.
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
`pw_gs::politica` (spec 05 §4.1). **Editável no lugar (B259):** o leitor registra onde começam o gatilho
(`Trigger::em`: `versão u32, id u32, ativo, rodando, só em combate` de 1 byte) e o parâmetro de cada condição e operação
(`parametro_em`, fora do JSON); `aipolicy_editavel.rs` troca as flags e os parâmetros de tamanho fixo com a mesma
codificação do leitor — operação: tipo de ataque, habilidade/nível, id de gatilho, timer (id, período, contador),
ajustar variável; condição: timer, vida abaixo de, probabilidade, raio, id, valor, faixa de dano. Testado nos dois
realms: relido pelo leitor, outra política igual, voltar devolve o arquivo idêntico. **Edição estrutural (B261):**
o leitor também registra `Trigger::fim` e `AiPolicy::em` (`versão, id, quantidade`); `remover_gatilho` corta
`em..fim` e decrementa a quantidade (recusado se outro gatilho da política o cita por rodar/parar/ativar e não há outro
com o mesmo id — o GS faria `GetTriggerPtr(-1)`, `policy_loader.cpp:237-238`); `duplicar_gatilho` copia a faixa logo
depois da original com id novo (maior + 1, ou o pedido; repetido → erro); `definir_fala` reescreve `uSize u32` + UTF-16
(`policy.cpp:550-552`, `:610-611`) e mantém o NUL final que a fala tinha — medido: **todas** as falas o têm (337 no
1.2.6, 5.964 no 1.5.5). O GS envia os `uSize` bytes como estão (`op_say`, `aitrigger.h:1844-1849`). Política com id
repetido no arquivo recusa edição estrutural. Testado nos dois realms: trocar fala + duplicar + remover + voltar
devolve o arquivo idêntico; a outra política não muda; gatilho citado não se remove. **Tipos (B271):** o leitor
registra `Trigger::operacoes_em`. Medido (`medir_condicoes`): cada tipo de condição tem **um** tamanho de parâmetro no
arquivo; operadores (com filhas) 5-7, 9-15; folhas em dois papéis que nunca se misturam — condição (0-4, 8, 18-20, 22,
24, 25) e valor (16, 17, 21, 23, 26, 27, debaixo de aritmética/comparação). `trocar_tipo_da_condicao` só troca folha por
folha do mesmo papel que já apareça no arquivo (o 1.2.6 só tem 0-8), com parâmetro zerado do tamanho visto.
`trocar_tipo_da_operacao`/`acrescentar_operacao` aceitam só tipos que aparecem no arquivo, com o parâmetro zerado no
tamanho de `tamanho_do_parametro` (respeitando `le_parametro_nesta_versao`; falas vazias com NUL) e, para
rodar/parar/ativar gatilho, um id de gatilho da política (zero cairia em `GetTriggerPtr(-1)`); `remover_operacao`
recusa a última do gatilho. Testado nos dois realms: trocar e voltar = arquivo idêntico; outra política igual.
**Árvore, nome e gatilho do zero (B365):** a árvore se regrava inteira como `SaveConditonTree` (`policy.cpp:110-149`:
tipo, tamanho, parâmetro; folha → `3`; senão `1`+esquerda, `2`+direita e `4`). `envolver_condicao` põe Não (5)/Ou (6)/E (7)
em volta de um nó de **condição** — o Não leva o nó no filho **direito** (medido: 113 Não no 1.2.6 e 1.672 no 1.5.5, todos
só com o direito); E/Ou levam o nó à esquerda e uma folha nova (tipo do arquivo, parâmetro zerado) à direita (medido:
E/Ou sempre com os dois filhos, 9 e 770). `remover_condicao` tira um Não (fica o filho) ou um filho de E/Ou (fica o
irmão). `renomear_gatilho` grava `szName[128]` em GBK (o leitor agora decodifica o nome em GBK; antes era UTF-8 com
perdas). `criar_gatilho` acrescenta no fim da política: versão do arquivo (medido: única — 1 no 1.2.6, 23 no 1.5.5),
id pedido ou maior + 1, ativo/não rodando/só em combate, uma folha de condição e uma operação (alvo 0). Testado nos dois
realms: envolver e tirar devolvem o arquivo idêntico; o gatilho novo relê e as outras políticas não mudam.

### 3.5 `ptemplate.conf` (`ptemplate.rs`)

Seções por classe (`[SWORDSMAN]`, `[ORGE]`, `[ASN]`, `[ANGEL]`…), **GBK**. Lido, mas quase
nada dele chega ao jogador: o `gamed` copia a ficha do banco (`userlogin.cpp`), que nasce do
`clsconfig`. **Não** são usados os atributos (Arqueiro 15/20/5/10 aqui; 5/5/5/5 no molde)
nem o `hp`/`mp`: `max_hp = lvlup_hp×(nível−1) + vit_hp×vitalidade`, base zero (B51 — antes
somava o `hp` do `.conf` e o Arqueiro nascia com 20 de energia). As velocidades dele também
são mortas. `[TOWN_REGION]` é o mapa de ressurreição, não o nascimento.
`[GENERAL] debug_command_mode` → `modo_de_depuracao`: só liga com o texto exato `active`
(`playertemplate.cpp:217`). Ele libera o "criar monstro" do GM, como no original. O `realm_155`
está com `active` desde 2026-10-01; o arquivo do pacote vinha com `false`.

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

**Caixas de transporte (B208, `regioes.rs`, `GameDataManager::caixas_de_transporte`):** depois do
cabeçalho, registros `i32 tipo` até completar as contagens — 0 = região (`i32 n` + `n × f32[3]`),
1 = caixa (`i32 destino`, `i32 origem` v ≥ 3, `i32 nível` v ≥ 5, `f32[3] pos`, `f32[3] exts`,
`f32[3] alvo`), `el_region.cpp:293-330` (servidor) = `EL_Region.cpp` (cliente); o índice é a ordem
de leitura. Fecha no último byte: 126 = 42 arquivos, 135 caixas; 155 = 75 arquivos, 281 caixas
(`tests/caixas_de_transporte_do_realm.rs`; `empty/` tem versão < 4 e fica de fora).

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

**Editável (B254):** `gshop_editavel.rs` guarda os registros crus e as 8 categorias e grava de volta **idêntico**
(`gshop.data` do 1.2.6; `gshop`, `gshop1`, `gshop2` do 1.5.5); campos de `GSHOP_ITEM` pelo nome do fonte por layout
(`campos_do_formato`: `local_id`, `main_type`, `sub_type`, `icon` `char[128]` GBK, `id`, `num`, `buy_k_*`, `desc`
`wchar[512]`, `szName` `wchar[32]`, brinde, `owner_npcs_0..7`, `buy_times_limit(_mode)`; no 1.2.6 `status`@196). Texto só
é reescrito (e o resto do campo zerado) quando muda.

**Conferência (B385):** `gshop_editavel::conferir_oferta(g, oferta, pilha)` porta as regras com que o GS 1.5.5 recusa a
loja ao carregar — `load_malldata` (`cgame/gs/template/globaldataman.cpp:135-200`: `type` fora de -1..2 em qualquer
opção) e `InitMall` (`cgame/gs/playermall.cpp:155-380`: id e quantidade, quantidade ≤ pilha do item
(`get_item_pile_limit`, `itemdataman.cpp:815`), validade < 2³¹, `CheckParam` do período (datas com início < fim,
semanal com bit 0–6 = domingo–sábado, mensal com bit k = dia k+1), grupo/selo ≥ 0, validade só com pilha 1 e sem
período, opções do mesmo grupo sem sobreposição, brinde com pilha e validade). Aba e subaba têm de existir. Oferta sem
nenhum preço = **aviso** (desativada: o cliente a esconde, `CalcBuyType`/`GetFirstBuyStatus`, `EC_Shop.cpp:172-197`,
`DlgQShop.cpp:914-929`; 751 das 1.726 do BR); preço só depois de um buraco = aviso (o GS para na 1ª opção sem preço,
`playermall.cpp:198`). No 1.2.6 as regras de item/pilha valem como aviso (o `InitMall` do `gs` 1.2.6 não foi conferido).
As 4 lojas reais passam sem erro de opção (`testes_da_conferencia`). **`imagem::tga_32`**: TGA tipo 2, 32 bits, origem
embaixo, descritor 0x08 e o rodapé `TRUEVISION-XFILE.` — os 37 TGA das abas da loja do cliente BR voltam byte a byte.

### 3.9 `collision.clt` (`collision.rs`)

Carregado por mapa; **não consultado** pela simulação (monstro atravessa obstáculos).

### 3.10 `gamedbd/clsconfig` — moldes de classe (fora do realm, no pacote do servidor)

Arquivo do `gamedbd` original com os roleids 16..31: `GRoleBase` + `GRoleStatus` +
inventário + equipamento + armazém, Marshal **big-endian** (`cnet/gamedbd/clsconfig.h`).
Molde por classe via `GetDataRoleId` (`gamedbmanager.cpp:208`): 0→16, 1→19, 2→20, 3→23,
4→24, 5→27, 6→28, 7→31, 8→18, 9→17, 10→21, 11→22.

**No servidor, o molde é o template do realm no banco** (`class_templates`, `class_template_items`,
`class_template_skills`): o `DBCreateRole` original copia o molde inteiro
(`cnet/gamedbd/dbcreaterole.hrp:46-60`, `:320-360`), e o `create_character` copia do template
nível, cultivo, EXP, SP, pontos livres, os quatro atributos, dinheiro, mapa e posição, a bolsa,
o corpo e as habilidades (B217; até então os atributos eram a constante 5 e as colunas eram
ignoradas). Vida e mana são a conta da classe (`BaseDaClasse::vida_e_mana_maximas`) sobre o nível,
a vitalidade e a energia do template. Classe sem template no realm: molde 5/5/5/5, nível 1, e o
reparo do código (`default_skills`, kit de bolsa) na carga; classe **com** template nunca recebe
o reparo. As habilidades iniciais do `realm_155` passaram do código para o template
(`scripts/2026_10_11_templates_e8.sql`, as mesmas de `default_skills`). O painel edita os
templates (spec 06, Templates E8).

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
alcance), `efeito_passivo` (o `TakeEffect` das passivas comuns `EVENT_RESET`/`WIELD`/`ENTER`,
B203: 119 lidas das 128 no 1.5.5, 45 no 1.2.6 executadas no `gs`) e os roteiros `no_alvo` (`StateAttack`, 2.304) e `em_si` (`BlessMe`, 266) como
`[quem, setter, expressão]` — expressão com `L`, `P_X`, `V_X`, `A_X`, `S_X`, `INT(...)`, `?:`,
avaliada pelo servidor (`pw_data_loader::expr` desde o B308, reexportado como `pw_gs::efeitos::expr`; todas as expressões de roteiro dos dois catálogos se leem — teste `todo_roteiro_dos_catalogos_se_le`). Carregado para `elements.data` v156/v159
(`GameDataManager::habilidades`).

**Correções pelo binário (B303).** O fonte não é o que o BR compilou: o `elementskill.dll` do
cliente v156 e o `gs` do pacote `pwserver_155v156` têm as **mesmas** tabelas de dinheiro, e
nenhuma das do fonte (a sequência `int32` da habilidade 8, `0,0,0,6980,…,771980`, aparece 30×
no `gs` e 29× no DLL; a do fonte para a 1, `0,30,130,…`, 0× nos dois). `tools/elementskill/numeros.cpp`
(32 bits) carrega o DLL, chama `InitStaticData`, percorre `NextSkill` e, por `Create(id, nível)`,
lê os campos pela vtable do objeto — os índices saem do próprio binário (a vtable exportada da
base `??_7ElementSkill@GNET@@6B@` contra os endereços das funções da base exportadas; o
`GetMaxLevel` está no slot 21, não no 19 que o `ElementSkill.h` daria) e conferem com as estáticas
exportadas `GetRequired*(id, nível)` em 20.676 de 20.676 níveis. `medir_elementskill.py` grava
em `elementskill_v156.json` só o que o binário diz diferente (2.272 habilidades), e o
`extrair_habilidades.py` aplica no fim, deixando o valor do fonte em `do_fonte`. Medido:
`cls`, `arrowcost` iguais em todas; diferem dinheiro (380 habilidades com lista no fonte),
`apcost` (14), `nivel_exigido` (3), `sp_exigido` (4), `execucao_ms` (7), `recarga_ms` (3), `mp`
(2). Requisito sem sobrescrita no stub (`null` no fonte) vale o 0 da base
(`ElementSkill/skill.h:170`, `cskill/skill/skill.cpp:14-60`) e o DLL o preenche. **Só relato:**
25 habilidades com nível máximo diferente (o DLL dá 10 onde o fonte dá 1) — subir o nível
máximo pede dano, alcance etc. dos níveis novos, que o DLL não dá.

**1.2.6 conferido pelo binário (B304).** O `elementskill.dll` do cliente 1.2.6
(`perfectworld_126 launcher/element`) não exporta `InitStaticData`, `GetCls` nem `GetMaxLevel`, e
o compilador fundiu as funções idênticas da base (22 slots com o mesmo `return 0`) — o medidor só
aceita slot de endereço único (`GetCoolingTime` = 5, `GetExecuteTime` = 6, `GetMpCost` = 23) e
lê os requisitos e a execução pelas estáticas exportadas; os níveis vêm do catálogo. Mesmas 823
habilidades; em 4.091 níveis recarga, execução, nível, SP, dinheiro e livro iguais ao
`habilidades_126/habilidades.json` (executado no `gs`), `mp` igual exceto a 2 nível 7 (37,0 no
catálogo, 36 no `int` do DLL — truncamento). A 187 nível 11 dá 42.333 em quatro campos no DLL:
leitura fora da tabela, não número. Nenhuma correção.

**Onde mora cada número no arquivo (B305).** `tools/elementskill/enderecos.cpp` procura cada sequência por nível
(da medição) nas seções de dados da imagem carregada e prova o candidato com uma sentinela **em cada nível**, lida
pela função do próprio DLL naquele nível; só vale seção gravável e byte igual ao arquivo. `tools/elementskill/mapear.py`
(`entrada`/`juntar`/`aplicar`) gera `specs/habilidades_155/elementskill_v156_tabelas.json`: SHA-256 e tamanho do DLL,
SHA-256 do "resto" (tabelas zeradas) e `{id: {campo: {arquivo, tipo, n, le_alem}}}`. Resultado no 1.5.5 (23.183
pares habilidade×campo): **1.795 tabelas no arquivo** (nível 535, SP 501, dinheiro 439, livro 320), 11.986 tudo zero,
2.692 preenchidas pelo código no carregamento (fora do arquivo), 884 de mana só em `.rdata`, o resto constante ou
fórmula no código. A 187 (12 níveis) tem tabelas de 10: o livro dos níveis 11–12 lê a tabela do dinheiro (`le_alem`
= 2). **Prova:** +1 em todos os elementos de todas as tabelas numa cópia; a medição da cópia muda exatamente os 5.981
valores esperados de 227.436 (20.676 níveis × 11 campos), sem nenhum outro, e o DLL roda inteiro. **1.2.6 (B306):** mesmo método (requisitos pelas estáticas, que o DLL
1.2.6 exporta; níveis pelo `niveis.txt` do catálogo), `specs/habilidades_126/elementskill_126_tabelas.json`: **1.865
tabelas** em 539 habilidades (nível 539, SP 505, dinheiro 501, livro 320); a 187 também lê além (nível, SP e livro,
`le_alem` = 2). Prova `mapear.py provar`/`conferir`: 6.504 valores esperados de 32.728, **0 errados** (e o 1.5.5
refeito pelo mesmo subcomando: 0 errados em 227.436).

**Mana e constantes compartilhadas (B311).** A sentinela passou a exigir o valor **exato** no retorno (não basta mudar:
no 1.2.6, "tabelas" que a função soma ou multiplica mudavam por outra quantia), e as tabelas em `.rdata` passaram a ser
julgadas pelas **relocações** do DLL (`mapear.py relocacoes`: num x86 toda referência absoluta a um dado está na
`.reloc`). Medido no 1.5.5: as 1.797 tabelas de `.data` têm cada uma exatamente um sítio na base; das 280 de mana em
`.rdata`, 279 são constantes lidas por 2 a 13+ sítios (a da 121, que derrubou o DLL no B305, por 3) — o compilador funde
o mesmo `float` de várias funções — e só 1 é exclusiva (413). No 1.2.6 o compilador também fundiu funções idênticas:
53 posições são lidas por mais de uma habilidade (vários `GetMpcost` que devolvem a mesma constante viram uma função
só) e ficam de fora, porque editar uma mudaria as outras. Mapas: **1.5.5 1.797 tabelas** (nível 536, SP 501, dinheiro
439, livro 320, mana 1) e **1.2.6 1.871** (nível 540, SP 505, dinheiro 501, livro 320, mana 5: 121, 122, 194, 310,
413). Prova `provar`/`conferir`: 0 errados nos dois (5.983 de 227.436; 6.510 de 32.728). O resto da mana vem do código
(constante ou fórmula, como `24 + 15 × nível`) — editar exigiria trocar instruções.

**`habilidades.pwsk` (B316).** O arquivo de dados do nosso `elementskill.dll` (opção D): escritor e leitor em
`habilidades_do_cliente.rs`, formato e origem dos dados na spec 08 §8–§9.

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

**Tabela de textos UTF-16 com fichas de várias linhas (B301, Estúdio S8):** `tabela_de_textos::TabelaDeTextos` lê o
`Configs\skillstr.txt` (`m_SkillDesc.Init(..., true)`, `EC_Game.cpp:529`) e as outras tabelas de
`CECStringTab::LoadWideStrings` (`EC_StringTab.cpp:202-262`), seguindo as regras do `AWScriptFile::GetNextToken`:
- **separadores e comentários:** pula tudo ≤ 32, `;` e `,`; `//` vai até o fim da linha e `/* */` é bloco;
- **texto entre aspas:** vai até a **próxima aspa**, atravessando linhas, **sem escape**;
- **cabeçalho:** tudo até `#_begin`; `#_index` antes dele indica tabela indexada;
- **ficha indexada:** `id "texto"`, com o texto lido por `GetNextToken(false)`; a aspa de abertura fica na linha do id;
- **id repetido:** vale o primeiro;
- **tamanho:** até 2.047 unidades por ficha (`MAX_LINELEN` 2048, `AWScriptFile.h:52`).

A tabela trabalha nas unidades UTF-16 do arquivo, de modo que o que não muda fica byte a byte igual (o arquivo do
1.5.5 mistura CRLF e LF). `definir` recusa aspa e excesso de tamanho.

No `skillstr.txt` real do 1.5.5 BR: **6.632 fichas, 2 por habilidade** (nome em N×10, descrição em N×10+1, para
3.316 habilidades). Sem edição, a reescrita é igual; trocar uma descrição de várias linhas muda só ela. O
`habilidades::json_do_servidor` expõe o JSON embutido; o topo dele é `{fonte, habilidades}`.

### 3.10c Tratadores de addon — `specs/addons_155/addons.json` (`addons.rs`)

Extraído de `cgame/gs/item/item_addon.cpp` (`INSERT_ADDON(id, tratador)`, fora de comentário)
por `specs/addons_155/extrair_addons.py`: 2.911 ids, 243 tratadores distintos. O tratador decide o
sorteio dos parâmetros (`Sorteio`) e o efeito (`BonusDeAddons`, spec 05 §5.1).

**Classificação (B209) — `specs/addons_155/classificacao.json`**, de `classificar_addons.py`, que resolve
macros (`IDMRA`, `IAERA*`, `SET_ADDON_MACRO`, `STONE_*`, `TEMPORARY_ADDON_MACRO`), `typedef`s e heranças
de `item_addon*.cpp|h` até `arg_addon<T>` (`item_addon.cpp:50-235`), uma classe de essência (família no
`if(datatype != DT_xxx_ESSENCE) return -1`) ou uma classe com `GenerateParam` próprio. Por tratador:
`tipo`, `classe`, `familia_no_fonte`, `marcas` (`conjunto`, `refino`, `pedra`, `temporario`, `float` =
o `Activate` lê o argumento como float). Todos os 243 resolvidos. Em `DadosDoAddon`:

| função | o que dá |
| :--- | :--- |
| `sorteio()` / `gerar_com()` | o `GenerateParam` de **todos** os tipos (`POINT`, `DOUBLE_POINT`, `PERCENT` ×100+0,1, `DOUBLE_PERCENT`, `SECOND` ×20, `VinteAvos` da velocidade de ataque, float entre dois, `TRIPLE_POINT`, refino, `n` como estão; temporário põe `0xFFFF` no 2º); o drop usa o sorteio, o painel o máximo |
| `valor_do_id()`, `faixa()` | o `GenerateParam` com o máximo / mínimo de cada faixa |
| `edicao()` | `Editavel` (`POINT`, `DOUBLE_POINT`, `PERCENT`, `DOUBLE_PERCENT` e essência de inteiro: 1.760 ids), `Fixo` (conjunto, float, próprios: 738), `ForaDaBusca` (refino, pedra, temporário: 413) |
| `aceita(args)` | editável: o número de argumentos do id; fixo: cada argumento dentro do mínimo–máximo |
| `familias` | `TabelaDeAddons::ligar_familias` (depois do `carregar_geracao`): essência pela família do fonte; senão as famílias cujas `WEAPON/ARMOR/DECORATION_ESSENCE` do realm sorteiam o id (`addons`, `rands`, `uniques`); senão a união do tratador no realm; senão todas |

`TabelaDeAddons::conjuntos` (B210): peça → efeitos do conjunto, do `SUITE_ESSENCE` (`equipments_N_id`,
1–12; `addons_N_id`, 1–11; `set_addon.cpp:5-48`); `conjunto_de(tratador)` dá `(N, base)` de
`SET_ADDON_MACRO(N,BASE)`.

O que existe num realm é o `EQUIPMENT_ADDON` dele com tratador: o v7 do 1.2.6 tem 1.649 ids, os mesmos
parâmetros do 1.5.5, e **nenhum** de nível de ataque/defesa, penetração, resiliência, vigor ou
anti-defesa/resistência. `crates/pw-data-loader/examples/addons_por_familia.rs` lista id, parâmetros e
famílias de um realm.

### 3.10d Modelos: `.ski` (`ski.rs`) e GLB (`glb.rs`) — B272, Estúdio S5

`.ski` = a pele de um modelo (porte de `A3DSkin::Load`, `Angelica3D/Source/A3DSkin.cpp:201-500`). O arquivo começa
pela marca do `AFile` — `MOXB` (binário) — que `AFile::Open` consome (`AngelicaFile/Source/AFile.cpp:86-98`); depois
`A3DSKINFILEHEADER` (`:66-82`, 104 bytes: `'ASKI'`, versão ≤ 9, contagens de malhas de pele/rígidas/morph/supple/
muscle, texturas, materiais, ossos da pele, `fMinWeight`, ossos do esqueleto, 52 reservados), nomes dos ossos (só
v ≥ 9; string = `i32` + bytes, `AFile.cpp:194-220`), nomes das texturas, materiais (string até NUL `"MATERIAL: …"`,
4 cores RGBA, `Power`, `2Sided` — `A3DMaterial.cpp:181-200`), malhas de pele (nome, `SKINMESHDATA`,
`A3DBVERTEX3` de 48 bytes, índices `u16` — `A3DSkinMesh.cpp:162-225`) e rígidas (nome, `RIGIDMESHDATA`, `A3DVERTEX` de
32 bytes, `u16` — `A3DRigidMesh.cpp:115-170`). **Medido no 1.5.5 BR (desempacotado):** 24.165 de 24.166 `.ski` fecham
no último byte (v8: 6.033; v9: 18.132); **nenhum** usa morph, supple ou muscle (o leitor recusa com o motivo); o que
sobra (`npcs\怪物\灵族\巨牛\牛.ski`, 36 bytes sem `ASKI`) o cliente também recusa (`A3DSkin.cpp:214`). O 1.2.6 ainda
não foi medido.

**Escrita (B286, S6):** `Ski::escrever` é o inverso de `ler` (`A3DSkin::Save`, `A3DSkin.cpp:523-604`, mesma ordem).
Para reescrever igual, o leitor guarda o que antes descartava: a marca `MOXB` (`marca`), a contagem de ossos do
cabeçalho (`ossos_no_cabecalho`, que existe mesmo na v8, onde os nomes não vêm), os 52 bytes reservados
(`reservado`), e, de cada material, a linha inteira e o byte do `2Sided`. Os textos voltam a GBK. Morph, supple e
muscle saem com contagem 0. Medido no 1.5.5 BR: **24.165 de 24.165** `.ski` lidos são reescritos byte a byte iguais
(`ski::testes::escrever_reproduz_o_cliente`). No 1.2.6, pelo contêiner: 300 de 300 de uma amostra. A ficha do
`.ski` no painel diz `reescrita_igual` por arquivo.

**Escrita do `.bon` e do `.stck` (B287, S6):**
- `Bon::escrever` segue `A3DSkeleton::Save` (`A3DSkeleton.cpp:585-677`) e `A3DBone::Save` (`A3DBone.cpp:585-…`). O
  leitor passou a guardar a marca, os 64 reservados e os 3 bytes de enchimento depois das flags de cada osso.
- O `Save` do cliente sempre grava v6. Para reescrever um v5, que é todo o 1.2.6, a trilha de cada junta vai logo
  depois dela, como o `Load` lê.
- `stck::escrever` segue `A3DSklTrackSet::Save` (`A3DTrackMan.cpp:304-…`), e `trilha::escrever`/`escrever_osso`
  seguem `A3DTrackData<T>::Save` (`A3DTrackData.h:601-…`).
- Medido no 1.5.5 BR: **9.849 de 9.849** `.bon` reescritos iguais (2.229 v5 e 7.620 v6) e **73.818 de 73.818**
  `.stck`.
- O leitor do `.smd` (`smd::cabeca`) ainda não lê cabides nem propriedades e não fecha no último byte, então o
  escritor dele vem depois.

**`.smd` inteiro (B288, S6):** `smd::Smd::ler` lê o arquivo até o último byte, e `Smd::escrever` faz o caminho inverso,
na ordem de `A3DSkinModel::Save` (`A3DSkinModel.cpp:302-416`). Depois das ações, o arquivo traz:
- **cabides** (`A3DSkinModel::LoadHanger`): tipo `i32` (0 GFX, 1 modelo, `A3DSkinModelAux.h:56-58`), nome, osso ou
  gancho, `HANGERDATA` de 8 bytes (`int` + `bool`, sem `#pragma pack`, guardados crus) e o arquivo; o cabide de
  modelo traz ainda o nome do CC (`A3DSkinModelHanger::Load`, `A3DSkinModelAux.cpp:304`);
- **propriedades:** nome e valor (`LoadModelFromFile`, `A3DSkinModelMan.cpp:321-331`).

Cada ação guarda os seus bytes de dados como estão (`FILEACTIONDATA` + juntas na v < 6, `FILEACTIONDATA6` na
v ≥ 6) e o `.stck` (v ≥ 7). O `cabeca` continua sendo a leitura rápida usada pelo painel, e o teste confere que ele
bate com o leitor completo.

Medido no 1.5.5 BR: **5.163 de 5.163** `.smd` fecham e são reescritos byte a byte iguais (v5: 1.788, v6: 395,
v8: 2.980; ao todo 7 cabides e nenhuma propriedade). No 1.2.6, pelo contêiner: 100 de 100 numa amostra (96 v5, 4 v6).
A ficha do `.smd` diz `reescrita_igual`. Com isso, os quatro
formatos do modelo (`.ski`, `.bon`, `.stck`, `.smd`) têm escritor conferido no cliente inteiro.

**Importar glTF → `.ski` (B289, S6):** `importar::ski_do_glb(glb, esqueleto)` desfaz `glb::do_modelo`:
- **eixos:** `z` negado em posição e normal, e a ordem de cada triângulo invertida;
- **pele:** cada primitiva com `JOINTS_0`/`WEIGHTS_0` vira uma malha de pele (3 pesos gravados e o 4º implícito).
  B383: os pesos saem como o motor lê (`A3DSkinMesh.cpp:1036-1066`) — não nulos na frente (o motor para no 1º zero),
  no máximo 4 (as maiores de `JOINTS_0` + `JOINTS_1`), soma 1; sem peso → a vaga do esqueleto; tudo no relatório
  (`importar::RelatorioDePesos`). A ordem entre não nulos fica a do glTF (a ida e volta dos modelos do cliente segue
  exata);
- **ossos:** casados pelo nome no `.bon` e gravados como `.ski` v9 (nomes). O nó "esqueleto" é a vaga que
  `BindSkeleton` reserva (índice = número de nomes). B383: osso que o `.bon` não tem passa os pesos ao ancestral mais
  próximo (no glTF) que existe — sem ancestral, recusa; pose de repouso do glTF (IBM) diferente da do `.bon` → os vértices
  são repostos na do `.bon` (`v' = Σ wᵢ · inversa(matBoneInit)ᵢ · IBMᵢ · v`, `importar::destino_das_juntas`);
- **material:** com `extras.angelica` (agora gravado pelo exportador) os valores voltam exatos. Sem ele valem os
  padrões medidos (`ski::testes::medir_padroes`): em 83% dos 28.421 materiais do 1.5.5, ambiente = difusa, emissiva e
  especular (0,0,0,1) e brilho 10. O `fMinWeight` é 0 em todos os 24.165 `.ski`, e o motor só o lê e regrava;
- **texturas:** o nome da imagem. Nome que não é DDS/TGA é relatado, porque a conversão PNG → DDS ainda não existe;
- **recusas e relatório:** primitiva sem pele, que não é de triângulos, com mais de 65.536 vértices ou com osso que
  não existe no esqueleto é recusada ou relatada.

Ida e volta numa amostra de 1/5 dos `.smd` do 1.5.5 (`importar::testes`): **824 modelos, 1.787 malhas e 1,3 milhão de
vértices** com posição, normal, UV e índices exatos e as mesmas influências por vértice. O 4º peso implícito sobra
como ruído (~1e-8) num lado e some no outro; abaixo de 1e-6 ele não conta. O `.ski` importado se reescreve e relê.

**PNG → DDS (B292, S6):** `imagem::png` decodifica PNG (o crate `png`), e `decodificar` passa a aceitá-lo.
`imagem::dds_argb32` grava DDS sem compressão de 32 bits com a cadeia inteira de mipmaps, com o cabeçalho que **646**
DDS de 32 bits do cliente 1.5.5 têm (medido):
- `flags` 0xA1007 e tamanho linear `w × h × 4`;
- formato 0x41 com máscaras ARGB;
- `caps` 0x401008.

O motor usa o número de mipmaps do arquivo (`A3DTexture.cpp:594`). Cada nível é a média 2×2 do anterior, o
`D3DX_FILTER_BOX` que o motor usa ao reduzir (`:673`).

O importador guarda em `texturas_novas` cada imagem embutida no GLB: PNG vira `<nome>.dds`, e DDS/TGA vão como estão.
Imagem que não vem embutida ou que não é PNG/DDS/TGA (JPEG) é relatada.

**DXT (B293, S6):** `imagem::dds_dxt` comprime em DXT1 quando a imagem é toda opaca e em DXT5 quando tem alfa. No
cliente 1.5.5 há 6.563 DXT1, 6.445 DXT3 e 2.038 DXT5, e o motor trata DXT3 e DXT5 igualmente como textura com alfa
(`A3DTexture.cpp:589-591`).
- **Cabeçalho**, medido nos DXT do cliente: `flags` 0xA1007, tamanho linear = blocos × 8 ou × 16, formato 0x4
  (FourCC) e `caps` 0x401008, com mipmaps até 1×1.
- **Bloco de cor:** extremos pelo eixo principal das cores (iteração de potência na covariância), sempre no modo de 4
  cores (`c0 > c1`). Cor única tem tratamento à parte, e os índices são escolhidos contra a mesma expansão 565 do
  decodificador.
- **Bloco de alfa (DXT5):** máximo e mínimo, com o índice de 3 bits mais perto.
- **Medido:** em 388 DXT reais (1/40 dos DDS), decodificados, recodificados e decodificados de novo, o erro médio
  quadrático é **0,96 por canal** (a pior textura, 4,28). A cor única sai exata.

O importador passou a gravar DXT (`TexturaNova`: nome, bytes e os pixels de origem, usados para comparar com a textura
que o cliente já tem). O `dds_argb32` continua disponível.

**Malha rígida no importador (B294):** a malha marcada pelo exportador (`extras.angelica.rigida`) volta rígida, presa ao
osso do esqueleto da sua junta. Uma malha **sem pele** que esteja abaixo de um nó de osso (o objeto preso a um osso no
Blender) vira rígida desse osso:
- os vértices passam pela matriz do nó no mundo (`matrix` ou TRS, com as mães) e depois pelo `z` negado;
- as normais passam pela mesma matriz, sem a translação, e são normalizadas;
- é o espaço do modelo, que é o da rígida (matriz `BoneInitTM × AbsoluteTM`, `A3DSkinModel.cpp:1341`).

Sem osso acima, a malha é relatada e fica de fora.

Ida e volta (1/5 dos `.smd` do 1.5.5): **848 modelos**, com 888 malhas de pele e **978 rígidas**, voltam exatos; as
rígidas no mesmo osso. Os 24 modelos que antes ficavam de fora eram os afetados pelo defeito do exportador.
`importar::testes::rigida_por_hierarquia` confere a matriz e o caso sem osso.

**Ações do glTF (B295, S6):** `importar::acoes_do_glb(glb, esqueleto)` transforma cada animação numa `Stck` mais o último
quadro.
- **Trilha para todo osso:** cada osso usa a trilha da sua primeira junta (`A3DSkinModelAct.cpp:612`). Todo osso
  precisa de uma, porque sem ela `GetBoneStateOnTrack` falha (`:379-384`) e quem chama usa posição e rotação sem
  valor. O osso sem canal fica constante na pose de repouso do nó (TRS ou a `matrix` decomposta); sem nó, na pose de
  ligação do `.bon`.
- **Valores:** os canais são os valores locais do osso. A volta à mão do Direct3D é a mesma do exportador: posição
  (x, y, −z) e quatérnio (−x, −y, z, w).
- **Amostragem:** uma chave por quadro no FPS do esqueleto, que é o FPS do `FrameToTime` da ação; nas `.stck` do
  cliente é 15 (83%) ou 30, igual em cada trilha. Um segmento de 0 ao fim.
- **Interpolação:** `LINEAR` (rotação por *nlerp*, pelo caminho curto) e `STEP`. `CUBICSPLINE` usa o valor da chave
  e é relatada; escala e `weights` também são relatadas.

Ida e volta em 40 ações reais (20 com `.bon` v5, 20 com v6, 47.401 chaves; `importar::testes::acoes_ida_e_volta`):
- posição exata (diferença relativa até 4,6e-7);
- quatérnio, nos instantes que caem numa chave original, até 9,6e-5;
- entre chaves fora da grade de quadros, até 1e-2: é a reamostragem sobre as amostras já normalizadas do GLB.

**Ação no esqueleto v5 (B296, S6):** no `.bon` v5 (todo o 1.2.6) não há `.stck`: cada junta tem **uma trilha embutida**
e cada ação do `.smd` é um intervalo de quadros dela. Medido em 1.163 ações v5 do cliente:
- a ação cita todas as juntas do `.bon`, na ordem 0…n−1 (1.161), com o mesmo intervalo, grupo −1, flags 0, 1 laço e
  velocidade 1,0;
- as ações ficam em sequência na trilha;
- o tempo de cada segmento é `piso(quadro × 1000 / fps)`, e o FPS da trilha é o do `.bon`;
- o cabeçalho do `.bon` cobre do 1º ao último quadro.

`importar::anexar_acao_v5(bon, smd, ação)` segue essa regra:
- em cada junta, emenda as chaves da ação no fim da trilha, com um segmento novo do quadro seguinte ao último até o
  fim da ação; a junta sem trilha na ação repete a última chave;
- avança o fim do cabeçalho;
- grava a entrada (v5: `FILEACTIONDATA` + `FILEJOINTACTION` por junta; v6: `FILEACTIONDATA6`);
- a ação de mesmo nome é repontada, e as chaves antigas ficam na trilha sem uso.

Testado em 30 modelos reais (10 com `.smd` v5, 20 com v6; 1.144 trilhas de junta): os dois arquivos releem, o
`cabeca` vê a ação no intervalo, e cada junta ganha as chaves e o segmento esperados.

**Várias ações e nome do cliente (B380):** `glb::do_modelo_com_acoes(peles, esqueleto, acoes, nome)` põe uma animação
glTF por ação; `AnimacaoDoModelo::nome_original` vai em `extras.angelica.nome`. `importar::nome_original(anim)` devolve o
nome da ação no `.smd`: `extras.angelica.nome`, senão o que vem depois do último `" · "` (`SEPARADOR_DO_NOME`), senão o
nome inteiro — `acoes_do_glb` usa. `importar::conferir_esqueleto(glb, bon)`: os ossos do glTF (`esqueleto_do_glb`) e os
do `.bon` têm os mesmos nomes e cada um a mesma mãe; `Err` lista os que faltam, sobram ou mudaram de mãe (teste:
`acoes_ida_e_volta`, 40 modelos reais — o GLB exportado confere, com um osso renomeado não). `ecm::Ecm::filhos`: os
`ChildPath:` (`EC_Model.cpp:152/4777`).

**Ação v5 igual (B381):** `importar::acao_v5_igual(bon, smd, acao)` / `diferenca_da_acao_v5` — a referência é a ação
existente passada pelo mesmo caminho do Estúdio (trilhas embutidas no intervalo dela → `do_modelo_com_acoes` →
`acoes_do_glb`), comparada chave a chave com a importada; igual dentro de `TOLERANCIA_DE_ACAO_IGUAL` (posição relativa
1e-3, quatérnio 5e-3 — medido na ida e volta real pelo Blender: 3,0e-4 e 3,2e-3). Intervalo diferente por junta, outro
número de quadros ou junta sem trilha = diferente. Comparar direto com a trilha erra entre chaves (chaves fora da grade:
até 1e-2). Exportador: `glb::tempo_na_grade` grava o tempo que cai na grade de quadros exatamente nela (o `f32` subido
um passo se ficou abaixo, nas precisões de 32 e 64 bits) — sem isso o Blender pode truncar o último quadro.

**Esqueleto do glTF (B297, S6):** `importar::esqueleto_do_glb(glb)` monta um `.bon` v6 a partir das juntas da 1ª `skin`
(sem a vaga "esqueleto").

Os campos que o glTF não tem seguem o que os 9.849 `.bon` do cliente 1.5.5 têm, medido em
`bon::testes::medir_estrutura` (`#[ignore]`) sobre 268.293 ossos:
- uma junta por osso, e a 1ª junta do osso `i` é a `i`;
- a junta se chama `<osso>_AnimJoint`, tipo 15, com `osso_filho` = o osso, `osso_mae` = a mãe dele e `irma` = −1
  (100% em todos esses);
- as filhas são os ossos que o têm como mãe;
- flags 0 em 97,5% (1 = osso falso, 2 = espelhado, `A3DBone.h:71-72`);
- reservado zerado, quadros 0..0 em todo v6, e FPS 15 em 76% dos v6 (o resto, 30).

Como o `.bon` é montado:
- `inicial` vem da matriz inversa de ligação;
- `relativa` é a pose local do nó em relação à junta mãe;
- as matrizes voltam à mão do D3D por `glb::matriz_gltf`, que desfaz a si mesma;
- os ganchos são os nós `HH_*`/`CC_*` (ou `extras.angelica.gancho`) abaixo de uma junta, com a matriz relativa a ela.
  Tipo 0 = HH (ponto de pendurar) e 1 = CC (centro de encaixe), `A3DSkeleton.h:62-63`; a matriz do gancho é relativa
  ao osso (ou ao esqueleto, com osso −1), `A3DSkeleton.cpp:162-187`.

O exportador passou a levar os **ganchos** como nós filhos do osso (`extras.angelica.gancho.tipo`) e as **flags** de
osso diferentes de 0 (`extras.angelica.flags`). O leitor de acessores aceita `MAT4`.

Ida e volta em 984 `.bon` reais (1/10; `importar::testes::esqueleto_ida_e_volta`): 26.962 ossos e 4.114 ganchos com
nomes, mães, filhas, juntas, flags, `inicial` e ganchos iguais (matrizes até 1e-4). A `relativa` é igual à pose de
ligação local que o GLB leva; o `matRelative` original só bate com ela em 28% dos ossos (B274).

**`.ecm` novo a partir de molde (B299, S6):** os campos do `.ecm` mudam com a versão
(`A3DCombinedAction::Load(pFile, dwVersion)`), então nada é escrito de memória. `ecm::ecm_novo(molde, smd, ações)`
parte de um `.ecm` do cliente usado como molde, nos bytes:
- troca a 1ª linha `SkinModelPath:` pelo `.smd` novo;
- põe uma ação combinada por ação, cada uma clonada do 1º bloco simples do molde (uma `BaseActName:`, sem
  `ChildActName:` e sem efeitos), com o nome e a base trocados;
- mantém o resto do molde.

Recusa molde com `AddiSkinPath:` ou com modelo filho (`ChildCount` ≠ 0), porque o modelo novo herdaria peças de outro.
Em todo o cliente 1.5.5 (`ecm::testes::ecm_novo_com_moldes_do_cliente`):
- **2.577 `.ecm` servem de molde** (1.596 na versão 67), e todos geram um `.ecm` que relê com o `.smd`, as ações e a
  contagem certos;
- os recusados: 2.239 com peles adicionais, 3.608 sem ação simples e 75 com modelo filho.

`glb::do_ski` gera glTF 2.0 binário: uma malha por malha do `.ski` (`POSITION` com `min`/`max`, `NORMAL`,
`TEXCOORD_0`, índices `u16`), um material por material (difusa → `baseColorFactor`, `doubleSided`). Eixos: Direct3D
(mão esquerda) → glTF (mão direita) negando `z` e invertendo a ordem dos triângulos. Sem esqueleto ainda.

**Texturas (B273, `imagem.rs`):** o cliente acha a textura `nome` de um `.ski` em `<pasta>\Textures\<nome>` e, se não
achar, em `<pasta>\Tex_<título>\<nome>` (`A3DSkin::Load`). Medido numa amostra de 1/7 dos `.ski` do 1.5.5 BR (4.985
referências): 4.785 em `Textures`, 139 em `Tex_`, 61 ausentes no próprio cliente; extensões dds 4.978, psd 5, tga 2;
formatos DXT1 1.732, DXT3 2.201, DXT5 746, RGBA 32 bits 244, TGA 1 — e **as 4.924 achadas decodificam**.
`imagem::decodificar` faz DDS (DXT1 com o índice 3 transparente quando `c0 <= c1`, DXT3, DXT5, sem compressão por
máscaras; só o 1º mip) e TGA tipo 2 (24/32 bits, origem pelo bit 5). `glb::do_ski_com_texturas` embute cada textura
como PNG e cria um material glTF por par (material, textura) usado; textura com algum alfa < 255 → `alphaMode` MASK
(corte 0,5 — escolha de exibição, não regra do jogo). Atenção, na instalação BR desempacotada os nomes estão no disco
como os bytes GBK lidos em Windows-1252 (artefato da extração; no `pw-recursos` o caminho é GBK e não há problema).

**Esqueleto e modelo (B274, `bon.rs`, `smd.rs`):** `.bon` = `A3DSkeleton::Load` (`A3DSkeleton.cpp:409-560`): `MOXB`,
cabeçalho de 96 bytes (`'ASKE'`, versão ≤ 6, ossos, juntas, ganchos, quadros, FPS), ossos (nome, `BONEDATA` de 144
bytes — `A3DBone.h:96-104` sem `#pragma pack`: flags + 3 de enchimento, mãe, primeira junta, filhas, `matRelative`,
`matBoneInit` — e os índices das filhas), tipos e dados das juntas (nome + 12 bytes; os 4 tipos leem igual) e ganchos
(nome + 72 bytes). **Medido no 1.5.5 BR:** 7.620 `.bon` v6 fecham no último byte; 2.229 são v5, com trilhas de
animação embutidas nas juntas (`LoadBoneTrack`) — lidas desde o B276 (abaixo).
**Semântica das matrizes, pelo fonte e medida:** a matriz da pele é `BoneInitTM × UpToRoot` (`A3DSkinModel.cpp:1341-1342`,
`:1743`) e o `UpToRoot` vem da animação (`A3DBone.cpp:245-263`), então `matBoneInit` = inversa da pose de ligação;
`matRelative` é uma pose de repouso que coincide com a de ligação em só 28% dos ossos (63.187/222.996, medido).
`.smd` = `A3DSkinModelMan::LoadModelFromFile`: `MOXB`, cabeçalho de 84 bytes (`'ASMD'`, versão ≤ 8, peles, ações,
cabides, propriedades), o nome do `.bon` e os das peles, relativos à pasta do `.smd`. Medido: 5.163/5.163 cabeças
leem; o esqueleto citado existe em 5.156; 4.285 de 4.574 peles citadas existem.
`glb::do_modelo` (peles + esqueleto): um nó por osso com a pose de ligação local (inversa do `matBoneInit`, relativa à
da mãe) + um nó raiz "esqueleto" (o índice `n` que `A3DSkin::BindSkeleton` reserva, `A3DSkin.cpp:830-865`), `skin` com
`matBoneInit` como inversa de ligação, e `JOINTS_0`/`WEIGHTS_0` (v < 9: índice = osso do esqueleto; v ≥ 9: pelo nome;
o 4º peso = `1 − soma`, `A3DMorphSkinMesh.cpp:313-325`; malha rígida presa ao osso com peso 1, `A3DRigidMesh.cpp:367`).
**Correção (B294):** o osso da malha rígida é o **índice no esqueleto**. `A3DRigidMesh` desenha com
`GetBlendMatrix(m_iBoneIdx)` direto, sem o `MapBoneIndex` da pele (`A3DSkinMesh.cpp:856`). Até o B293 o exportador o
traduzia pela lista de ossos da pele. Nos 3.543 `.ski` v9 do cliente com malha rígida essa lista é vazia, e a malha
ficava presa à raiz "esqueleto": certa na pose de ligação, errada ao animar. A malha rígida agora vai com
`extras.angelica.rigida`.
Testado em 274 modelos reais: 710/733 malhas casam com o esqueleto, e **as 8.245 juntas** reproduzem a pose de
ligação (mundo pelos nós × inversa ≈ identidade).

**Trilhas e ações (B276, `trilha.rs`, `stck.rs`, `smd.rs`):** trilha = `A3DTrackData<T>` (`A3DTrackData.h`): 12 bytes
(chaves, segmentos, FPS), chaves (`A3DVECTOR3` ou `A3DQUATERNION` `x, y, z, w`), segmentos de 16 bytes (ms inicial e
final, chave inicial e final); um osso = posição + rotação (`LoadBoneTrack`, `A3DTrackMan.cpp:203`); tempo → chave por
`GetFloorKeyIndex`. `.bon` v5: a trilha vem depois de cada junta. `.stck` (`A3DSklTrackSet::Load`, `:150-200`): `MOXB`,
`'STCK'` v1, 16 bytes (trilhas, quadros, FPS) e por trilha `id` + trilha. O `id` é o **índice da junta** — cada osso
usa a da sua primeira junta (`A3DSkinModelAct.cpp:612`). Ações no `.smd` depois das peles: física (string), pasta das
trilhas (v ≥ 8; antes `tcks_<título>`), e por ação o nome, os quadros (v < 6 por `FILEACTIONDATA` + juntas; v ≥ 6
`f32` × 2) e, v ≥ 7, o `.stck`; tempo = quadro × 1000 / fps do esqueleto (`FrameToTime`); `.smd` < 7 com `.bon` v6 →
o motor descarta as ações. **Medido no 1.5.5 BR:** os **9.849** `.bon` fecham (v5: 2.229, v6: 7.620), os **5.163**
`.smd` leem com as ações, os **73.818** `.stck` fecham (2,2 milhões de trilhas). `glb::do_modelo_animado`: uma
animação glTF por ação — canais de translação `(x, y, −z)` e rotação `(−x, −y, z, w)` por osso, tempos desde 0 s.
Testado em 40 modelos (20 v5, 20 v6): no meio da 1ª ação a malha deformada mantém a escala da de repouso (nenhum
fora de 0,5–2×) — convenção errada a faria "explodir".

**`.ecm` e diagnóstico de ações (B277, `ecm.rs`):** o `.ecm` é texto (`MOXT`, GBK): `Version`, `SkinModelPath` (o
`.smd`), e as ações combinadas — o que o cliente toca pelo nome (`CECModel::PlayActionByName` → `GetComActByName`,
`EC_Model.cpp:3075-3085`) —, cada uma com `CombineActName` e as ações-base do `.smd` (`BaseActName`,
`A3DCombinedAction::Load`, `A3DCombinedAction.cpp:142-…`). Lido por chave de linha (os campos entre elas variam com a
versão). Combinada sem ação-base não anima. Medido no 1.5.5 BR: 8.499 `.ecm` leem; 157.808 de 158.056 ações-base
citadas (99,8%) existem no `.smd`. Nomes pedidos a um monstro (`GetActionName`, `EC_NPCModel.cpp:599-620`): a tabela
`Configs\actions_npc.txt` (`CECStringTab`, GBK sequencial; índices do enum de `EC_NPC.h:94-120`), só as que o código de
NPC toca: de pé, ócio, guarda, andar, ataques 1 e 2 com `起`/`落` (`IsAttackAction`, `EC_NPC.cpp:1282`), correr, morrer e
ferido — este com alternativa: o motor tenta `受伤叠加` e, sem a combinada, `受伤` (`ACT_WOUNDED2`,
`EC_NPCModel.cpp:692-702`). `ACT_SKILL1`/`ACT_MAGIC1` (mapeados a −1, `:901-910`) e os pulos (só modelo de jogador) não
são pedidos — a 1ª versão do diagnóstico os pedia e acusava 6 faltas em **todos** os 40 monstros medidos por realm, o
que mostrou a regra errada. Mascote (`_地面`/`_水中`/`_空中`) e habilidade (nome nativo) não tratados.
**Ação nova no `.ecm` (B280):** `ecm::clonar_acao` copia o bloco de uma ação combinada (de `CombineActName:` até a
próxima; a última até `ScriptCount:` (v ≥ 25) ou `AddiSkinCount:`, que o `CECModel` lê depois das ações) logo depois
dela, com o nome novo e, se pedido, outra `BaseActName` (só com uma ação-base na origem), e soma 1 no `ComActCount`.
Trabalha nos **bytes** (só o nome novo é codificado em GBK; o resto fica idêntico); nome repetido ou fora do GBK →
erro. Medido em todo o 1.5.5 BR: nos 8.499 `.ecm` os blocos achados batem com o `ComActCount` e o fim é achado; 7.098
clonagens releem com uma ação a mais e a cópia com as mesmas ações-base. **Mascote (B281):** mascote invocado
(`id_type == 28752`, `EC_Pet.cpp:285-288`) e de combate/evolutivo (`inhabit_type == 6`, `EC_NPCModel.cpp:588-590`)
pedem cada ação com `_` + ambiente (`GetActionEnvString`, `:580-596`: `地面`, `水中`, `空中`; o ambiente vem do estado
que o servidor manda — voando, nadando ou no chão, `EC_NPC.cpp:411-417`). `ecm::ambientes_do_habitat` pela ordem do
comentário do `inhabit_type` (`ExpTypes.h:2866`: 0 chão, 1 água, 2 ar, 3 chão+água, 4 chão+ar, 5 água+ar, 6 os três) e
`ecm::pedidos_de_mascote` = os pedidos de monstro × os ambientes do habitat. **De onde vem a pele (B282):** o jogo
carrega o modelo de um NPC/monstro **sem** as peles do `.smd` (`LSF_NOSKIN`) e põe a pele `<caminho do .ecm com
extensão .ski>` (mais o `<nome>二级.ski` de baixa resolução, se existir) — `CECNPC::LoadNPCModel`, `EC_NPC.cpp:186-222`;
sem ela, `Models\error\error.ski`. As `AddiSkinPath:` do `.ecm` (`ecm::Ecm::peles_adicionais`) são relativas a
`Models\` e só o editor as carrega (`EC_Model.cpp:678-689`, `#ifdef GFX_EDITOR`). No 1.2.6 o `.smd` é compartilhado
por várias variantes (p.ex. `暴牙牛喽罗.smd` sem pele, com `水牛怪喽罗.ecm`/`.ski` ao lado) — por isso o `.smd` sozinho
não tem malha. **Voo (B283):** o modelo de voo é filho do modelo do jogador. Para espada e asa, ele pendura no gancho
`HH_feijian`/`HH_chibang` do jogador pelo encaixe `CC_feijian`/`CC_chibang` do esqueleto dele. As rodas penduram nos
pés com `CC_feijian` (`EC_Player.cpp:153`, `:1943-1965`). O modo vem de `FLYSWORD_ESSENCE.fly_mode` (0 espada, 1 asa,
2 rodas, `FlyMode2WingType`); `WINGMANWING_ESSENCE` é sempre asa — `ecm::encaixe_de_voo`. Sem o encaixe no esqueleto, o
motor não posiciona o filho (`A3DSkinModelAux.cpp:378-407`). O modelo de voo é carregado como `CECModel` comum, com as
peles do `.smd` (`LSF_UNIQUESKIN`, `EC_Player.cpp:4483-4515`). O jogador não toca nada nele diretamente: cada ação
combinada do `.ecm` do jogador traz `ChildActName:`/`HHName:`/`IsTrail:` (`ChildActInfo::Load`,
`A3DCombinedAction.cpp:1792-1850`; lidas em `AcaoCombinada::filhas`). Ao tocar, todo filho recebe o nome e toca a
combinada dele com esse nome; o rastro só vale no filho do gancho certo (`EC_Model.cpp:3569-3620`;
`ecm::chega_ao_voo`). `ecm::diagnostico_de_voo` cruza os nomes mandados pelos `.ecm` de jogador com o vocabulário do
modo, isto é, o que os outros modelos de voo do mesmo modo têm. A falta só é acusada quando **a maioria** dos colegas
tem a ação; quando a maioria também não tem, a linha diz "ausente, como na maioria dos colegas". Sem isso, `悬停`
(1 de 101 espadas do 1.5.5) acusaria todas. Medido em 114 `.ecm` de jogador no 1.5.5 e 13 no 1.2.6. O vocabulário de
espada é `飞剑悬停/低空下降/高空下降` em todos; `飞剑起飞/飞行/降落/滑翔` (e, no 1.5.5, `飞剑前进/落地`) só em parte dos
modelos. O de asa é `飞行悬停/飞行前进/起飞/展开/翅膀收起/低空下降/高空下降/飞行落地`. **Copiar de outro `.ecm` (B285):**
`ecm::copiar_acao(destino, fonte, origem, nova)` põe o bloco da ação `origem` da `fonte` depois da última ação do
destino, com o nome `nova`, e soma 1 no `ComActCount`. Os fins de linha passam a ser os do destino, e o resto fica byte
a byte igual. Só entre arquivos da mesma `Version`, porque os campos do bloco dependem dela (`A3DCombinedAction::Load`).
As ações-base citadas precisam existir no `.smd` do destino; quem chama confere. Teste no cliente 1.5.5 inteiro:
8.156 cópias releem com uma ação a mais e as bases da origem, e 7.900 pares de versão diferente são recusados.

### 3.11 `global_api.lua`

Primeira linha `--<N>` = `lua_version` do `SERVER_TIME`. 1.5.5: **102**; divergência faz o
cliente encerrar ("wrong config data").

## 4. Dados de cada realm 1.5.5

| realm | base do servidor | `.data` do cliente | elements | tasks |
| :--- | :--- | :--- | :--- | :--- |
| `realm_155` | `F:\PW\1.5.5\home155\gamed\config`; desde o B211 os `npcgen.data` de `world`, `a46`, `a61`, `a63`, `a76`, `a77` e o `a50/precinct.sev` do `F:\PW\1.5.5\pwserver_155v156\home\pwserver\gamed\config` (a build v156) | cliente BR | v156 (55.442.775 B) | 129 |

O pacote de servidor e os `.data` do cliente precisam ser da mesma família: `npcgen.data`,
mapas e `.sev` **não vêm do cliente** (B11, B12). **B211:** o `world/npcgen.data` do `home155`
(2.872.138 B) é anterior às classes 10/11 do v156 — sem a cidade da Névoa Sombria (Ancião 45355,
Teletransportador Rayga 44724) e com Sereias Ecoantes no mar onde ela fica; o do `pwserver_155v156`
(4.273.236 B, 44.646 entidades) tem a cidade. Os 77 `npcgen` do v156 fecham no último byte
(`conferir_npcgen`); o `aipolicy.data` dos dois tem as mesmas 3.136 políticas e ficou o do `home155`.
`examples/spawns_perto.rs` lista os spawns de um raio (com controlador); `agua_no_ponto.rs`, água,
terreno e piso.

**Nomes de itens (B186):** `GameDataManager::nomes_de_itens` — o `Name` de todo registro do
`elements.data` de uma **tabela de item** (o mesmo critério de `pilhas`), para o painel. **B216:** tabela
de item = declara `ID` e `pile_num_max` ou, sem ele, `price` e `shop_price` (`servicos::e_tabela_de_item`).
O segundo caso é só o `FLYSWORD_ESSENCE` do 1.2.6 (o layout v7 não tem `pile_num_max`; o 1.5.5 não tem
nenhum): sem ele os 241 itens de voo do 1.2.6 ficavam sem nome, ícone e pilha (o voo 15732 do Tsuko).
Pilha do voo = 1 (`limite_de_pilha` sem o campo). Também por esse critério: `tabelas_dos_itens` (a tabela
de cada id, para a categoria da busca do painel) e, do `PET_ESSENCE`, `ambientes_dos_mascotes`
(`inhabit_type`: 0 terra, 1 água, 2 ar, 3 terra+água, 4 terra+ar, 5 água+ar, 6 todos —
`npcgenerator.cpp:114-135`). Teste `o_voo_15732_tem_nome_icone_e_pilha_nos_dois_realms`.
Medido: 126 (v7) 7.896 itens, todos com nome; 155 (v156) 26.188, 24.806 com nome (o arquivo
traz `Name` vazio nos outros). Teste `nomes_de_itens_do_realm.rs`.

**Ícones de itens (B188):** `GameDataManager::icones_de_itens` — o `file_icon` de cada item,
só o nome depois da última `\` e antes do NUL, **em bytes GBK** (126: 7.855 de 7.896 itens; 155:
25.974 de 26.188). O cliente acha a célula do atlas `Surfaces\IconSet\IconList_Ivtr{M,F}.dds`
pelo nome em minúsculas (`EC_GameUIMan.cpp:588-640, 6204-6208`); a caixa só baixa depois de
decodificar o GBK, então o servidor guarda bytes e o painel decodifica. Atlas em `data/icones/`
(fora do git), extraído do `surfaces.pck` do cliente 1.5.5 com `tools/pw-pck-extract`.
