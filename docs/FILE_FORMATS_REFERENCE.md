# Guia de Referência Técnica: Formatos de Arquivos de Dados do Perfect World

Este documento é o manual oficial de especificação e engenharia reversa dos formatos de arquivos binários utilizados pelo Perfect World em todas as versões (v1.2.6 até v1.5.3+).

---

## 1. `elements.data` (Banco de Dados Central de Templates)

O `elements.data` armazena todas as tabelas de itens, monstros, NPCs, habilidades, receitas e fórmulas de jogo.

### 1.1 Cabeçalho Binário (Header)

| Offset (Bytes) | Tipo | Campo | Descrição |
| :---: | :---: | :--- | :--- |
| `0x00` | `int16` (LE) | `version` | Versão do arquivo (ex: `7`, `10`, `12`, `27`, `63`, `145`, `153`). |
| `0x02` | `int16` (LE) | `signature` | Assinatura de integridade/timestamp da ferramenta exportadora. |

Logo após o cabeçalho, o arquivo é composto por uma sequência de **Listas de Dados (Lists)**. Cada lista possui:
1. `count` (`int32` LE): Quantidade de registros na lista.
2. Sequência de $N$ structs binárias de tamanho fixo para aquela versão.

---

### 1.2 Catálogo Completo das Principais Listas

```
+---------------------------------------------------------------------------------------------------+
| ID | Nome da Estrutura (CNet/GS)   | Conteúdo e Atributos Principais                              |
+---------------------------------------------------------------------------------------------------+
| 0  | EQUIPMENT_ADD_ON              | Atributos adicionais de refino, pedras espirituais e forjas. |
| 1  | WEAPON_MAJOR_TYPE             | Categorias principais de armas (Lâminas, Arcos, Magia, etc).|
| 2  | WEAPON_SUB_TYPE               | Subtipos de armas (Espada Dupla, Sabre, Cajado, Orbe, etc).  |
| 3  | WEAPON_ESSENCE                | Armas: dano min/max físico/mágico, velocidade, slots, preço. |
| 4  | ARMOR_MAJOR_TYPE              | Categorias de armaduras (Leve, Pesada, Mágica).              |
| 5  | ARMOR_SUB_TYPE                | Subtipos de armaduras (Peitoral, Calça, Elmo, Botas).        |
| 6  | ARMOR_ESSENCE                 | Armaduras: defesas elementais, HP/MP, durabilidade, slots.   |
| 7  | DECORATION_MAJOR_TYPE         | Categorias de acessórios.                                    |
| 8  | DECORATION_SUB_TYPE           | Subtipos (Colar, Ornamento, Anel).                           |
| 9  | DECORATION_ESSENCE            | Acessórios: esquiva, acerto, dano mágico/físico adicional.   |
| 10 | MEDICINE_MAJOR_TYPE           | Categorias de poções e remédios.                             |
| 11 | MEDICINE_SUB_TYPE            | Subtipos de poções (Cura imediata, Regeneração contínua).   |
| 12 | MEDICINE_ESSENCE             | Poções: valor de cura HP/MP, cooldown (tempo de recarga).    |
| 13 | MATERIAL_ESSENCE              | Itens de materiais básicos de forja (Madeira, Ferro, etc).   |
| 20 | MONSTER_ESSENCE               | Monstros e Chefes: HP, nível, defesas, AI Policy ID, Drops.  |
| 21 | NPC_ESSENCE                   | NPCs: diálogos, janelas de serviços, forja, teleporte.       |
| 22 | MINE_ESSENCE                  | Recursos coletáveis: minérios, ervas, baús de quests.        |
| 28 | TALISMAN_ESSENCE              | Amuletos / Hierogramas de Vida e Mana automáticos.           |
| 38 | RECIPE_ESSENCE                | Receitas de produção: materiais necessários, taxa e custos.  |
| 60+| NOVAS EXPANSÕES (1.4.6/1.5.3) | Cartas de Avatar, Astrolábio, Títulos, Reencarnação, Sabres. |
+---------------------------------------------------------------------------------------------------+
```

---

## 2. `gshop.data` (Loja de Gold / Cash Shop)

### 2.1 Estrutura do Formato do Cliente (Formato Rico)

```
[ CABEÇALHO ]
  • timestamp (uint32)        : Timestamp de modificação.
  • categories_count (uint32) : Quantidade de abas principais (ex: "Promoção", "Moda", "Voo").

[ LOOP DE CATEGORIAS ]
  • category_name (UTF-16LE, 128 bytes) : Nome da aba.
  • subcategories_count (uint32)        : Quantidade de subcategorias.
  • [subcategories names (UTF-16LE)]

[ LOOP DE OFERTAS (ITEMS) ]
  • items_count (uint32) : Quantidade total de ofertas na loja.
  • Para cada item:
    - shop_id (uint32)          : ID único do slot na loja.
    - category_id (uint32)      : Aba associada.
    - subcategory_id (uint32)   : Sub-aba associada.
    - icon_path (char[128])     : Caminho do ícone (.dds).
    - description (char[1024])  : Descrição rica com cores.
    - item_id (uint32)          : ID do item no elements.data.
    - count (uint32)            : Quantidade de itens entregues por compra.
    - price (uint32)            : Preço em CUBI / Gold (centavos de Gold).
    - status_flags (uint32)     : 1=Hot, 2=Sale, 4=New, 8=Recomendado.
    - buy_limit (uint32)        : Limite máximo por jogador.
```

### 2.2 Estrutura do Formato Antigo do Servidor (`gshopsev.data`)
No servidor legado em C++, os campos de texto `icon_path` e `description` eram **removidos**, deixando apenas:
`shop_id`, `category_id`, `subcategory_id`, `item_id`, `count`, `price`, `status_flags`, `buy_limit`.

### 2.3 Como o `pw-data-loader` Unificou os Dois Formatos
O parser moderno em Rust ([`gshop.rs`](file:///f:/Python_C_Projects/PWSource1.5.3/pw-universal-server/crates/pw-data-loader/src/gshop.rs)) detecta dinamicamente a densidade de bytes por registro:
- Se houver descrições e caminhos de ícone, ele lê o formato completo do cliente.
- Se for a versão compacta do servidor antigo, ele consome apenas os campos numéricos.
- **Resultado**: Você pode usar **o mesmo arquivo `gshop.data` do cliente dentro da pasta `config/` do servidor**, garantindo paridade total de preços.

---

## 3. `tasks.data` (Árvore de Quests e Missões)

O `tasks.data` é construído como uma árvore hierárquica de missões (`QuestTree`):

```
+---------------------------------------------------------------------------------------------------+
| CAMPO NO TASKS.DATA     | TIPO               | UTILIZAÇÃO NO JOGO                                 |
+---------------------------------------------------------------------------------------------------+
| task_id                 | uint32             | ID único da missão.                                |
| name                    | UTF-16LE           | Título da missão no registro de quests.            |
| type                    | uint32             | Principal, Diária, Cultivo, Recompensa, Evento.    |
| min_level / max_level   | int32              | Faixa de nível de personagem permitida.            |
| req_cultivation         | int32              | Nível de cultivo espiritual exigido.               |
| req_classes             | bitmask (uint16)   | Classes autorizadas a pegar a missão.              |
| pre_tasks               | uint32[]           | IDs de missões que devem ser concluídas antes.     |
| monster_kills           | (uint32, uint32)[] | Pares de (ID do Monstro, Quantidade a abater).     |
| item_collections        | (uint32, uint32)[] | Pares de (ID do Item, Quantidade a coletar).       |
| time_limit              | uint32             | Tempo limite em segundos (ou 0 para infinito).     |
| reward_exp              | int64              | Experiência concedida ao entregar a missão.        |
| reward_sp               | int64              | Alma / Pontos de Habilidade concedidos.            |
| reward_money            | int64              | Moedas entregues.                                  |
| reward_reputation       | int32              | Pontos de reputação / fama.                        |
| reward_items            | (uint32, uint32)[] | Itens concedidos na conclusão.                     |
| dialog_tree             | DialogueNode[]     | Falas de início, em andamento e conclusão do NPC.  |
+---------------------------------------------------------------------------------------------------+
```

---

## 4. `aipolicy.data` (Inteligência Artificial de Monstros e Chefes)

Contém máquinas de estado com pares **Gatilho $\rightarrow$ Ação**:

```
[ GATILHOS DISPONÍVEIS ]
  • OnAggro                     : Disparado no primeiro instante em que o monstro entra em combate.
  • OnHPPercent(X)              : Disparado quando a vida do chefe cai abaixo de X% (ex: 50%, 20%).
  • OnTimer(Segundos)           : Disparado ciclicamente a cada N segundos.
  • OnAttacked                  : Disparado ao sofrer golpe físico ou mágico.
  • OnTargetDie                 : Disparado quando o jogador alvo morre.

[ AÇÕES DISPARADAS ]
  • CastSkill(skill_id, level)  : Conjura habilidade (ataque em área, debuff, cura).
  • SummonMinions(id, count)    : Invoca monstros auxiliares no raio ao redor do chefe.
  • SayText(mensagem)           : Envia fala dramática do monstro no chat.
  • ChangeAggro                 : Troca o alvo principal para quem causou maior dano recente.
  • Flee / SelfDestruct         : Entra em estado de fuga ou explode em dano massivo.
```

---

## 5. `npcgen.data` (Zonas de Spawn Específicas por Mapa)

> [!IMPORTANT]
> **Localização de cada `npcgen.data`**:  
> Cada mapa ou dungeon possui seu próprio `npcgen.data` dedicado dentro da sua respectiva subpasta:
> - `world/npcgen.data`: Spawns do Mapa-Múndi principal (Pan Gu).
> - `a01/npcgen.data` a `a33/npcgen.data`: Spawns específicos de cada dungeon (FB19..FB99, etc.).
> - `b01/npcgen.data` a `b35/npcgen.data`: Spawns de instâncias especiais (Dusk, Frost, etc.).

```
[ BLOCO DE ÁREA DE SPAWN NO NPCGEN.DATA ]
  • area_name (char[32])        : Nome descritivo da região (ex: "DragonCity_EastGate").
  • spawn_type (uint8)          : 0 = Monstro, 1 = NPC de Diálogo, 2 = Minério / Recurso.
  • template_id (uint32)        : ID da entidade cadastrada no elements.data.
  • count (uint32)              : Quantidade máxima de instâncias vivas simultaneamente.
  • center_x, center_y, center_z: Coordenadas 3D do centro da área de spawn.
  • radius (float)              : Raio circular de dispersão ao redor do centro.
  • respawn_sec (uint32)        : Tempo em segundos para renascer após a morte/coleta.
  • patrol_path_id (uint32)     : ID da rota de patrulha (caso o monstro caminhe).
```

---

## 6. Arquivos de Colisão 3D: `.clt` e `.clv`

- **`.clt` (Collision Terrain)**:
  - Grid de vértices com matriz de alturas $Y = f(X, Z)$.
  - Fica dentro da pasta de cada mapa (ex: `world/collision.clt`).
- **`.clv` (Collision Volume / Meshes 3D)**:
  - Malhas poligonais tridimensionais representando construções, muralhas, árvores, pontes e cavernas.
  - Fica dentro da pasta de cada mapa (ex: `world/collision.clv`).
