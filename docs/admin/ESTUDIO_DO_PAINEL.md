# Estúdio do painel: editor de conteúdo do jogo (B231, revisto em 2026-10-14)

Viabilidade, desenho e tecnologia do **Estúdio** — a área do painel web onde se edita o conteúdo do jogo.
Nada implementado; estado `planejado`. Base: `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` §8 (cliente solto,
zip, registro de alterações, Blender).

## 0. Decisões do Murillo (2026-10-14)

- **Frontend do Estúdio em Svelte** (TypeScript + Vite), empacotado no próprio painel.
- O Estúdio é **um item do menu do grupo Realm**: primeiro se escolhe o realm, depois se abre o Estúdio
  **dele** (a versão do realm decide o layout: v156 no 1.5.5, v7 no 1.2.6).
- **Qualquer GM** do painel acessa e publica.
- Versões em uso: **1.5.5** e **1.2.6** — as linhas de cliente levam o número da versão (`1.5.5`, `1.2.6`;
  nada de "155br"). Os realms 1.4.8 e 1.5.3 foram removidos (B231).
- Ids novos **sem faixa definida pelo Murillo** (§3.1).
- Mapas: mapa interativo com zoom para posicionar NPCs, monstros e o resto (§4.3); no futuro, editor de mapa
  completo — objetos, rios, relevo (§8).
- Missões: desenho proposto no §4.4.
- O resto do `elements`: editor de tabela simples (§4.2).

## 1. O cliente completo fica no servidor

O serviço `pw-recursos` (Docker, volume próprio — não dentro do realm nem do GS) guarda, por versão de cliente:
a **versão 0** (o cliente original inteiro, importado dos `.pck` e dos soltos da instalação), todas as
**alterações** depois dela e o **zip** de instalação. Com isso o Estúdio tem, no servidor, os `.data`, modelos,
texturas, animações, ícones, minimapas e textos do cliente — sem depender da máquina de ninguém. O realm continua
lendo só o seu `config/`; a publicação grava nele o `elements`/`tasks`/`npcgen` novos.

## 2. Áreas do Estúdio

| área | arquivos | quem lê |
| :--- | :--- | :--- |
| **Elementos** | `elements.data` — 231 tabelas no v156, 119 no v7 | servidor + cliente |
| **NPCs e serviços** | `NPC_ESSENCE`, 30 `NPC_*_SERVICE`, `TALK_PROC` (no `elements`) | servidor + cliente |
| **Mapas** | `npcgen.data` (NPCs, monstros, recursos por área), `region.sev`/`precinct.sev` (cidades, ressurreição), `world_targets.sev` (teleportes), `path.sev` (patrulhas); `task_npc.data` (posição dos NPCs para o rastreador do cliente) | servidor (+ cliente no `task_npc`) |
| **Missões** | `tasks.data`, `dyn_tasks.data` | servidor + cliente |
| **Comportamento** | `aipolicy.data` | servidor |
| **Loja Gold** | `gshop*.data` | servidor + cliente |
| **Textos** | `configs`, `interfaces` (`item_desc.txt`, `fixed_msg.txt`…) | cliente |
| **Habilidades** (parcial) | `habilidades.json` (servidor) + `skillstr.txt`; o comportamento está no `elementskill.dll` (3.318 `ElementSkill/skillNNN.h`) | servidor + binário do cliente |
| Templates de criação | já no painel (E8) | servidor |

## 3. Regras gerais

### 3.1 Ids novos: não precisa de faixa

O arquivo não tem faixa. O carregador do cliente indexa os registros em **espaços de id**
(`ElementClient/CCommon/elementdataman.cpp`, `add_id_index`): `ESSENCE` (135 tabelas no mesmo espaço — itens,
monstros, NPCs, serviços…), `CONFIG` (55), `FACE` (11), `RECIPE` (10), `ADDON` (2), `TALK` (1). Dentro de um
espaço o id tem de ser único: um repetido **sobrescreve em silêncio** o anterior no índice
(`essence_id_index_map[id] = p`, `add_id_index_essence`). Regra do Estúdio:

- id novo = **maior id do espaço + 1**, calculado no servidor na hora de criar (com a trava do rascunho, dois
  GMs não pegam o mesmo);
- a publicação **recusa** id repetido no mesmo espaço;
- missões têm espaço próprio (`tasks.data`, id 1–65.535 — o canal do painel já limita assim, B198).

Faixa só seria preciso se fôssemos misturar com um `elements` oficial mais novo; nossa linha é nossa, então não.

### 3.2 Fluxo de edição

Lista paginada com busca → **Editar**, **Remover**, **Novo** (escolhe em qual registro se baseia; o clone
nasce no **rascunho** do GM — cancelar descarta o rascunho, nada fica para trás). Remover é **bloqueado** se outro
registro usa o id (índice de referências, §5). Tudo vai para o rascunho; **Publicar** → canal de teste do launcher
→ versão; diferenças campo a campo e reverter.

## 4. Telas

### 4.1 Telas próprias (visibilidade integrada)

| categoria | tabelas | o que a tela junta |
| :--- | :--- | :--- |
| **Armas** | `WEAPON_ESSENCE` (250 campos), `WEAPON_MAJOR_TYPE`, `WEAPON_SUB_TYPE` | ficha agrupada; categoria em modal (criar categoria = novo `WEAPON_SUB_TYPE` por clonagem; o `action_type` só pode ser um conjunto de ataque que os modelos de jogador já têm, `EC_Player.cpp:4600`); modelos `file_model_right`/`file_model_left`/`file_matter` no visualizador 3D; ícone; efeitos (`EQUIPMENT_ADDON`) com a busca que o painel já tem |
| **Armaduras, ornamentos, moda** | `ARMOR_*`, `DECORATION_*`, `FASHION_*`, `FASHION_SUITE`, `SUITE` | idem armas (moda = modelo por sexo) |
| **Consumíveis e materiais** | `MEDICINE_*`, `MATERIAL_*`, `STONE_*`, `DAMAGERUNE_*`, `ARMORRUNE_*`, `SKILLTOME_*`, `TOWNSCROLL`, `REVIVESCROLL`, `TRANSMITSCROLL`, `AUTOHP`/`AUTOMP`, `DOUBLE_EXP`, `REFINE_TICKET`, `DYE_TICKET`, `SHARPENER`, `PROJECTILE_*`, `QUIVER_*`, `TASKMATTER`, `TASKNORMALMATTER`… | ficha + ícone + "usado por" (drops, lojas, missões, receitas) |
| **Monstros** | `MONSTER_ESSENCE` (287 campos), `MONSTER_TYPE`, `MONSTER_ADDON` | ficha (nível, vida, dano, defesas, velocidades, agressividade, patrulha, estratégia, habilidades por vida); **drops** (85 campos → lista de item + chance com busca); **modelo 3D e diagnóstico de ações** (§4.5); "onde nasce" (abre o mapa) |
| **NPCs** | `NPC_ESSENCE` (47), `NPC_TYPE` | ficha; **serviços** ligados (cada `id_*_service` abre o editor daquele serviço); **conversa** (`TALK_PROC`, editor de árvore); modelo 3D; "onde está" (mapa) |
| **Serviços** | 30 `NPC_*_SERVICE` | uma tela por tipo: loja (páginas × itens), compra, **forja** (`NPC_MAKE_SERVICE`: páginas × receitas com `RECIPE_*`), teleporte/transporte (destinos no mapa), ensinar habilidade, armazém, conserto, incrustar/retirar, missões entregues/recebidas, mascote (nome, aprender/esquecer), vincular/destruir, gravar, propriedade aleatória… |
| **Receitas** | `RECIPE_MAJOR_TYPE`, `RECIPE_SUB_TYPE`, `RECIPE_ESSENCE`, `STONE_CHANGE_RECIPE*` | materiais e produto com busca de item |
| **Mascotes** | `PET_TYPE`, `PET_ESSENCE`, `PET_EGG`, `PET_FOOD`, `PET_FACETICKET`, `PET_EVOLVE*`, `PET_EVOLVED_SKILL*` | ficha + ovo + evolução + modelo e ações (com os ambientes terra/água/ar) |
| **Voo e montaria** | `FLYSWORD_ESSENCE`, `WINGMANWING_ESSENCE` | ficha + modelo, ponto de montar (`HH_Ride`) |
| **Daimon** (só 1.5.5) | `GOBLIN_*` | ficha + equipamentos + pílula de EXP |
| **Minas e recursos** | `MINE_TYPE`, `MINE_ESSENCE` | ficha + modelo + onde nasce |

### 4.2 Editor de tabela simples (todo o resto)

Toda tabela sem tela própria abre na **ficha genérica**: lista paginada com busca por id/nome, e a ficha com
todos os campos pelo tipo do layout (número com limite do tipo, texto `wchar` de tamanho fixo, `float`), com
**Novo por clonagem** e **Remover** com a mesma regra. Campos cujo nome indica referência (`id_*`, `*_id`,
`file_*`, `file_icon`) ganham busca/visualização automaticamente quando o catálogo do §5 souber o destino.
Entram aqui, no v156, entre outros: configurações (`*_CONFIG`: `PLAYER_LEVELEXP`, `CHARRACTER_CLASS`,
`PARAM_ADJUST`, `PLAYER_ACTION_INFO`, `ENEMY_FACTION`, `PLAYER_SECONDLEVEL`, `UPGRADE_PRODUCTION`, `MULTI_EXP`,
`PLAYER_DEATH_DROP`, `CONSUME_POINTS`, `ONLINE_AWARDS`, `TITLE`, `COMPLEX_TITLE`, `MERIDIAN`, `PLAYER_REALM`,
`PLAYER_REINCARNATION`, `SIGN_AWARD`, `EQUIP_MAKE_HOLE`, `TASK_LIST`…), rosto (`FACE_*`, `COLORPICKER`,
`CUSTOMIZEDATA`), casamento (`WEDDING*`), facção (`FACTION_*`, `FORCE*`, `COUNTRY`, `CONGREGATE`), torre solo
(`SOLO_TOWER_*`), astrolábio (`ASTROLABE*`), cartas (`POKER*`), carruagem (`CHARIOT*`), lojas de ficha
(`TOKEN_SHOP`, `TOUCH_SHOP`, `RAND_SHOP`, `SHOP_TOKEN`), Lar (`HOME*`), fogos (`FIREWORKS*`), itens diversos
(`SPEAKER`, `BIBLE`, `TARGET_ITEM`, `LOOK_INFO`, `DESTROYING`, `GM_GENERATOR*`, `RED_PACKET_PAPER`,
`WISH_TRIBUTE`, `UNIVERSAL_TOKEN`, `MONEY_CONVERTIBLE`…). Uma tabela pode ganhar tela própria depois sem
mudar o resto.

### 4.3 Mapas: mapa interativo

- **Imagem:** o minimapa do próprio cliente, em peças: `Surfaces\MiniMaps\<mapa>\RRCC.dds` (DXT1 256×256,
  32.896 B; o mundo tem 368 peças, as instâncias 64–72) — `DlgMiniMap.cpp:1168`. O servidor monta as peças
  (já no `pw-recursos`) e o navegador mostra com zoom e arrasto.
- **Coordenadas:** a mesma conta do cliente (`DlgMiniMap.cpp:1052-1061`): 256 px por peça, **2 m por pixel**
  (`px = (x + nCol·512)/2`, `py = (nRow·512 − z)/2`), origem no centro do mapa; a altura vem do GS
  (`.hmap` + `movemap`, que ele já carrega), então clicar no mapa dá `(x, y, z)` no chão.
- **Camadas:** áreas do `npcgen` (NPCs, monstros, recursos, com raio e quantidade), cidades e pontos de
  ressurreição (`region`/`precinct`), teleportes (`world_targets`), patrulhas (`path.sev`).
- **Ações:** arrastar uma área; "Novo" no ponto clicado (escolhe o monstro/NPC pela busca); editar quantidade,
  tempo de renascer, raio; a ficha do monstro/NPC abre ao lado. Mover NPC também atualiza o `task_npc.data`
  do cliente (rastreador de missões).
- Esculpir terreno, construções, água e vegetação: **editor de mapa completo, §8 (futuro)**.

### 4.4 Missões: proposta

O `tasks.data` é uma floresta de missões com etapas (filhas) recursivas — 14.885 no 1.5.5, 2.819 raízes no 1.2.6
(spec 03 §3.2). Desenho:

- **Lista** com busca por nome/id/NPC/item/monstro, filtros por tipo e nível.
- **Vista em cadeia:** grafo das pré-requisitos e continuações (missão → próximas), para ver uma linha inteira.
- **Ficha por abas:** Geral (nome com as cores `^RRGGBB` como o jogo, tipo, nível, repetição, tempo) ·
  **Quem dá e quem recebe** (NPC — mantém sincronizados os `NPC_TASK_OUT_SERVICE`/`NPC_TASK_IN_SERVICE`) ·
  **Objetivo** (matar monstros com busca, coletar itens com busca, ir a um lugar com o mapa do §4.3, falar com NPC)
  · **Diálogos** (textos de cada etapa) · **Recompensas** (itens com busca, dinheiro, EXP, SP, cultivo,
  reputação) · **Etapas** (árvore das filhas, cada uma com as mesmas abas).
- **Validação:** o escritor fecha no último byte nos dois layouts (v55, v129) e confere referências (NPC,
  monstro, item, missão existentes). Missões em curso dos personagens apontam para etapas — mudar a estrutura de
  uma missão ativa pede confirmação.

### 4.5 Modelos 3D e animações

three.js (MIT, empacotado; a CSP `self` continua valendo) ao lado da ficha. O servidor converte o modelo do
jogo para glTF (cache por SHA-256). **Ações:** lista as do `.ecm` e as que o cliente pede para aquele tipo e
marca as que faltam — o nome pedido é a base de `Configs\actions_npc.txt` (`EC_NPC.cpp:295`; monstro: 站立,
普攻1, 普攻2, 魔攻1, 技能1, 死亡, 受伤叠加…) + 起/落 nos ataques + `_地面`/`_水中`/`_空中` no mascote de
combate (`EC_NPCModel.cpp:599-620`): "monstro não ataca" = falta `普攻1起`/`普攻1落`. Corrigir: ligar uma ação
existente, copiar de outro modelo do mesmo esqueleto, ou importar do glTF. Importar do Blender com ajustes
automáticos (escala/eixo, até 4 ossos por vértice, ossos pelo nome, PNG → DDS, textura ausente) e relatório; a
extensão do Blender do outro documento (§8.4) para modelar.

### 4.6 Loja Gold (`gshop`): editor visual

**O que o arquivo tem** (`ElementClient/CCommon/globaldataman.h:140-175`; leitor `pw-data-loader/src/gshop.rs`,
spec 03 §3.8): categorias principais com nome e subcategorias (`GSHOP_MAIN_TYPE`: `szName[64]`, `sub_types`), e
ofertas (`GSHOP_ITEM`): `main_type`/`sub_type`, item (`id`) e quantidade (`num`), **4 opções de compra**
(`buy[4]`: preço, início/fim, validade em segundos, tipo de período — fixo/semanal/mensal —, dias marcados,
**selo** — nenhum, novo, promoção, recomendado, 1 a 9 de desconto… —, `flag`, VIP mínimo), brinde (item,
quantidade, validade, preço de registro), NPCs donos e limite de compras (por dia/semana…). Layouts que fecham:
1.5.5 (registro de 1.436 B, com VIP), 1.2.6 (1.288 B, `buy` de 12 B: preço, data, validade). O `gshop.data` é
lido pelo **cliente** e pelo servidor — a posição na lista é o `goods_index` da compra, então os dois têm de
ser o mesmo arquivo (spec 03 §3.8); há `gshop1`/`gshop2` (segunda loja, `globaldata_getgshopitems2`).

**Layout proposto — "a loja como o jogador vê", com edição:**

```text
┌ Loja Gold ─ [Loja 1 ▾] ─ busca por nome/ID ─ filtro: selo, VIP, com brinde ─ [Publicar] ┐
│ Categorias        │  Vitrine (grade de cartões)                       │  Oferta          │
│ ▸ Moda            │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐               │  [ícone] Nome    │
│   ▸ Masculino     │  │ ícone│ │ ícone│ │ ícone│ │  ＋  │               │  Item: busca     │
│   ▸ Feminino      │  │ nome │ │ nome │ │ nome │ │ nova │               │  Quantidade      │
│ ▸ Montarias       │  │ 990  │ │ 1.490│ │ -30% │ │oferta│               │  Opções (4 abas):│
│ ▸ Consumíveis     │  │ NOVO │ │      │ │PROMO │ └──────┘               │   preço, período,│
│ [+ categoria]     │  └──────┘ └──────┘ └──────┘   (arrastar = ordem)    │   selo, VIP, dias│
│ (arrastar/renomear)│                                                   │  Brinde: busca   │
│                   │                                                   │  Limite / NPCs   │
└───────────────────┴───────────────────────────────────────────────────┴──────────────────┘
```

- **Árvore de categorias** à esquerda (criar, renomear, reordenar, apagar só se vazia); **vitrine em grade** com
  o **ícone do item** (atlas `iconlist_ivtr*`, o mesmo que o painel já usa), nome, preço da opção principal e o
  selo como faixa colorida, como na loja do cliente; **painel da oferta** à direita.
- **Nova oferta:** busca de item do `elements` (com ícone e a dica do item, que o painel já monta), ou clonar uma
  oferta existente; arrastar entre categorias e para reordenar (a ordem é o `goods_index` — o painel avisa que
  reordenar muda o índice e que cliente e servidor recebem o mesmo arquivo na publicação).
- **Opções de compra** em quatro abas: preço (em gold, com a conversão 100 = 1), validade (permanente ou
  dias/horas), período (sempre, datas de início/fim com calendário, dias da semana/mês marcados), selo, VIP
  mínimo. **Brinde** com busca de item. **Limite** de compras e modo. Pré-visualização do cartão como no jogo.
- 1.2.6: os campos que o layout v55 não tem (selo, VIP, período semanal…) aparecem desligados com a explicação.
- Publicar = mesma regra do resto (rascunho → canal de teste → versão); vai ao `config/` do realm **e** ao cliente.

## 5. Arquitetura e tecnologia

```text
navegador: Estúdio (Svelte + TypeScript + Vite, three.js) ── em static/estudio/, mesma origem e CSP
   │
pw-admin-api (FastAPI) — sessão GM, CSRF; repassa /api/realms/{realm}/estudio/* (realm explícito)
   │
pw-recursos (Rust, axum) — versão 0, objetos, rascunhos, publicação, zips (volume próprio)
   │   crates: pw-data-loader (ler e fechar no último byte) + escritores novos (ida e volta byte a byte)
   │           + formatos 3D (port do Angelica3D) + conversor glTF
   │   PostgreSQL: registro de alterações, versões, travas, índice de referências
   │
GS (canal administrativo) — só o que é do mundo vivo: altura/chão no mapa, aplicar no realm
```

- **Catálogo de interface** por versão (`specs/estudio/<versao>/<TABELA>.json`): grupos, rótulos em
  português, tipo de controle (referência → busca na tabela destino, enumeração, máscara de classes, chance,
  modelo, ícone), campos repetidos agrupados em lista (drops, páginas de loja). Tabela sem catálogo = ficha
  genérica (§4.2). O mesmo motor serve ao v7 e ao v156.
- **Índice de referências** recalculado por versão: "usado por…", bloqueio de remoção.
- **Um caminho de escrita por formato**, no servidor; o navegador manda alterações por campo e arquivos.

## 6. Fases (depois do P1–P3 do outro documento)

> **Estado (B237, 2026-10-14):** S1 feito para o `elements.data` — escritor com ida e volta byte a byte
> (`pw_data_loader::elements_editavel`), ids por espaço (`specs/elements_layouts/espacos_de_id.json`), lista,
> ficha genérica, rascunho com diferenças campo a campo, publicar (que agora grava também no `config` do realm).
> Faltam do S1: diferenças lado a lado na publicação; o índice de referências é do S2 (hoje a remoção usa uma
> busca conservadora por `int32` igual ao id). Descrição atual: `specs/07_RECURSOS_DO_CLIENTE.md` §6b.

| fase | entrega |
| :--- | :--- |
| S1 | motor: menu Estúdio no realm, áreas, lista paginada, ficha genérica, rascunho, ids por espaço, diferenças, publicar |
| S2 | escritor do `elements` (v7 e v156) + telas de Armas, Armaduras, Consumíveis, Monstros (drops) + índice de referências |
| S3 | NPCs, Serviços (30 tipos) e Conversa (`TALK_PROC`); Receitas |
| S4 | Mapas: minimapa interativo, `npcgen` (NPCs, monstros, recursos), `region`/`precinct`, teleportes, `task_npc` |
| S5 | 3D: visualizador, ações e diagnóstico; Mascotes, Voo, Minas |
| S6 | glTF e extensão do Blender |
| S7 | Missões (§4.4); Loja Gold (§4.6); Comportamento (`aipolicy`); Textos |
| S8 | Habilidades: números e textos; depois `elementskill.dll` + efeitos no GS |

## 7. A levantar antes das fases

- Limites do D3D9 (ossos por malha, vértices) — antes do S5.
- `nRow`/`nCol` de cada mapa (tamanho do minimapa) — de onde o cliente tira (antes do S4).
- Lista de ações pedidas por tipo de modelo além de monstro/NPC (jogador, montaria, voo) — antes do S5.
- Campos de `TALK_PROC` (o layout trata como laço próprio, sem campos listados) — antes do S3.

## 8. Futuro: editor de mapa completo (pedido de 2026-10-14)

Pedido do Murillo: no Estúdio, um editor de mapa onde se colocam **objetos livremente**, se editam **rios** e
**relevo**, e se posicionam monstros, NPCs, teleportes etc. Fica **depois do S8** — não entra nas fases atuais.
O S4 (mapa interativo sobre o minimapa, §4.3) é o primeiro passo dele.

### 8.1 Do que um mapa é feito (evidência)

**Cliente** (`maps\<mapa>\`, ex. `maps\world\` no 1.5.5):

| arquivo | o que é | quem lê |
| :--- | :--- | :--- |
| `<mapa>.trn2` | descrição do terreno (blocos, tamanho) | `EC_World.cpp` → `A3DTerrain2` |
| `<mapa>_N.t2bk` (88 no `world`) | blocos do terreno: alturas e normais | `A3DTerrain2LoaderB.cpp` |
| `.t2mk` | máscaras das camadas de textura (pintura do chão) | `A3DTerrain2LoaderA.cpp` |
| `.t2lm` / `.t2lm1` | mapas de luz (dia/noite) | `A3DTerrain2LoaderA.cpp` |
| `.t2hc` | dados de recorte por horizonte | `EC_World.cpp` |
| `<mapa>.ecwld` / `.ecbsd`, `bsdata\` | a **cena**: blocos com árvores, **água**, construções (*ornaments*), áreas (*box areas*), grama, efeitos, modelos, criaturas de ambiente, curvas (*bezier*) e sons | `EC_World.cpp`, `EC_Scene.cpp`, `EC_SceneBlock.cpp:1044-1072` (`SCNRES_TREE`, `SCNRES_WATER`, `SCNRES_BUILDING`, `SCNRES_BOXAREA`, `SCNRES_GRASS`, `SCNRES_EFFECT`, `SCNRES_ECMODEL`, `SCNRES_CRITTER`, `SCNRES_BEZIER`, `SCNRES_SOUND`) |
| `.cld`, `scenelights.dat` | nuvens, luzes da cena | `EC_World.cpp`, `EC_SceneLights` |
| `navigate.dat`, `region.clt`, `precinct.clt` | navegação e regiões do lado do cliente | — |
| `Surfaces\MiniMaps\<mapa>\RRCC.dds` | minimapa (§4.3) | `DlgMiniMap.cpp` |
| água | `A3DTerrainWater` (`Angelica3D/Source/A3DTerrainWater.cpp`) | motor |

O próprio cliente 1.5.5 tem rotinas de edição para o sistema de Lar — `DlgAutoWater`, `DlgAutoBaseWater`,
`DlgAutoOutdoorBuilding`, `DlgAutoOutdoorTree`, `DlgAutoOutdoorGrass` (`ElementClient/`) — e o pré-processador de
mapas aleatórios gera terreno e minimapa (`EC_RandomMapPreProcessor.cpp`: `.trn2`, `.t2bk`, `.t2mk`, `.t2lm`,
`minimaps`). São a melhor referência de como regravar essas camadas.

**Servidor** (`config\<mapa>\`, spec 03 §3.6): `.hmap` (altura do chão), `movemap/` (piso andável acima do
terreno), `watermap/` (superfície da água), `airmap/` (espaço do ar), limites do `gs.conf`, `npcgen.data`,
`region.sev`/`precinct.sev`, `world_targets.sev`, `path.sev`; e os arquivos de navegação que o servidor **não**
lê hoje (`map.bht`, `.dhmap`/`.rmap`, `collision.clt`, spec 03 §3.6c).

### 8.2 O ponto crítico: cliente e servidor têm de bater

Tudo que muda o chão, a água ou o que bloqueia passagem existe **duas vezes**: desenhado no cliente e calculado
no servidor. Erguer um morro só no cliente = o monstro anda por dentro dele; pôr uma casa só no cliente = o
jogador atravessa a parede no servidor; um rio novo sem `watermap` = o servidor acha que é chão seco. Portanto
o editor **gera os arquivos do servidor a partir do que se edita** e publica os dois juntos:

| edição | cliente | servidor (regerado) |
| :--- | :--- | :--- |
| relevo (pincel de altura) | `.t2bk` (+ normais), `.t2hc` | `.hmap`, `movemap/`, `airmap/` |
| pintura do chão | `.t2mk` | — |
| água (rios, lagos) | bloco de água na cena (`SCNRES_WATER`) | `watermap/` |
| construção, rocha, árvore | `SCNRES_BUILDING`/`SCNRES_TREE` na cena | `movemap/` e `airmap/` (o que bloqueia), colisão |
| grama, efeitos, sons, criaturas de ambiente | cena | — |
| monstros, NPCs, recursos | — | `npcgen.data` (+ `task_npc.data` no cliente) |
| teleportes, cidades, ressurreição | — | `world_targets.sev`, `region`/`precinct.sev` |
| luz | `.t2lm`/`.t2lm1` (reassar depois de mudar relevo/objetos) | — |
| minimapa | `Surfaces\MiniMaps\<mapa>\*.dds` (renderizar de cima) | — |

### 8.3 Viabilidade em níveis

| nível | o que faz | viabilidade |
| :--- | :--- | :--- |
| M1 | posicionar monstros, NPCs, recursos, teleportes, cidades, patrulhas sobre o minimapa | **sim** — é o S4 |
| M2 | colocar/mover/girar/escalar objetos de cena: construções, rochas, árvores, grama, efeitos, sons, criaturas de ambiente | **viável, médio** — regravar os blocos da cena + regerar `movemap`/`airmap` do que bloqueia |
| M3 | água: desenhar e editar rios e lagos (contorno, altura da superfície) | **viável, médio** — bloco de água da cena + `watermap` |
| M4 | relevo: pincel de altura (subir, descer, suavizar, nivelar) e pintura de textura do chão | **viável, alto** — `.t2bk`/`.t2mk`, reassar a luz, regerar `.hmap`/`movemap`/`airmap` e o minimapa |
| M5 | mapa novo do zero (instância nova) | **alto** — tudo do M2–M4 + entrada no `gs.conf`, limites, `instance`, portais e o tamanho do minimapa |

### 8.4 Como ficaria

- **Vista 3D no navegador** (WebGL, three.js como no §4.5): terreno em blocos com nível de detalhe, água, objetos
  da cena (os modelos vêm pelo mesmo conversor do 3D) e as camadas do M1. Alternar com a vista de cima (minimapa).
- **Ferramentas:** selecionar/mover/girar/escalar (gizmo), biblioteca de objetos (as construções, árvores e
  efeitos que existem no cliente, com busca e miniatura), pincéis de relevo e de textura, desenhar água por
  contorno, colocar spawn/teleporte/área.
- **No servidor (`pw-recursos`):** leitores e escritores de `.trn2`/`.t2bk`/`.t2mk`/`.t2lm`/`.ecwld`/`.ecbsd`
  portados do `Angelica3D`/`ElementClient` (com ida e volta byte a byte, como o resto); um **gerador** dos
  arquivos do servidor (`.hmap`, `movemap`, `watermap`, `airmap`) a partir do terreno e dos objetos que bloqueiam;
  um renderizador de minimapa; reassar a luz (no mínimo uma aproximação, para o mapa não ficar com sombras
  velhas).
- **Testar:** o mapa editado vai ao **canal de teste** do launcher (cliente) e a um realm de teste (servidor),
  sempre juntos.

### 8.5 A levantar antes

- Formatos exatos de `.trn2`, `.t2bk`, `.t2mk`, `.t2lm`, `.ecwld`, `.ecbsd` e da água (versões e campos no
  fonte e no arquivo do 1.5.5 e do 1.2.6).
- Como os arquivos do servidor foram gerados no original (há ferramentas no pacote do servidor? `movemap.conf`
  e os parâmetros de `Pixel Size`/`Submap` já aparecem na spec 03 §3.6c).
- Quanto custa reassar o mapa de luz e se dá para reaproveitar a rotina do pré-processador de mapas aleatórios.
- Desempenho do terreno inteiro do `world` (88 blocos) no navegador.
