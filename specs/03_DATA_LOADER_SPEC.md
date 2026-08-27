# Especificação 03: Arquitetura do Carregador de Dados (`pw-data-loader`)

Esta especificação documenta o formato binário, cabeçalhos, diferenças de versão e funcionamento dos parsers para todos os arquivos de configuração e mundos do Perfect World.

---

## 1. `elements.data` (Banco de Dados Principal de Itens, NPCs e Monstros)

O `elements.data` é o coração do balanceamento do jogo. Ele contém tabelas sequenciais (*Listas*) com estruturas binárias de tamanho fixo para cada tipo de entidade.

### 1.1 Estrutura do Cabeçalho
```
+-------------------------------------------------------------------------------+
| OFFSET | TIPO   | NOME            | DESCRIÇÃO                                 |
+-------------------------------------------------------------------------------+
| 0x00   | int16  | version         | Versão do arquivo (ex: 7, 10, 27, 63, 145)|
| 0x02   | int16  | signature       | Assinatura/Timestamp interno de exportação|
+-------------------------------------------------------------------------------+
```

### 1.2 Mapeamento das Principais Listas

| ID da Lista | Nome da Estrutura | Conteúdo Principal |
| :---: | :--- | :--- |
| **0** | `EQUIPMENT_ADD_ON` | Propriedades mágicas adicionais, bônus de refino (+1 a +12) e atributos de pedras. |
| **3** | `WEAPON_ESSENCE` | Armas (espadas, sabres, arcos, cajados, esferas): dano min/max, alcance, velocidade de ataque, slots. |
| **6** | `ARMOR_ESSENCE` | Armaduras (peitorais, calças, elmos, botas): defesa física, defesas elementais, HP/MP. |
| **9** | `DECORATION_ESSENCE` | Acessórios (colares, ornamentos e anéis). |
| **12** | `MEDICINE_ESSENCE` | Poções de HP/MP, pergaminhos de teleporte, amuletos/hierogramas e consumíveis. |
| **20** | `MONSTER_ESSENCE` | Monstros e Chefes: HP, dano, defesas, EXP, Alma (SP), raio de agressividade, ID do `aipolicy.data` e tabelas de Drop. |
| **21** | `NPC_ESSENCE` | NPCs do mundo: diálogos, serviços de forja, armazém, teleporte e missões vinculadas. |
| **22** | `MINE_ESSENCE` | Recursos do mapa: minérios, ervas de alquimia, baús e itens coletáveis. |
| **38** | `RECIPE_ESSENCE` | Receitas de produção: materiais necessários, taxa de sucesso, item resultante e custos de moedas. |

---

## 2. `gshop.data` (Loja de Gold / Cash Shop): Cliente vs Servidor

### 2.1 Análise da Diferença Histórica (Cliente vs Servidor)

No servidor C++ oficial legado:
- **No Cliente (`gshop.data`)**: O arquivo possui campos de interface gráfica: nomes de abas, caminhos de ícones, descrições ricas, categorias secundárias e flags visuais (*Hot, Sale, New*).
- **No Servidor Clássico (`gshop.data` ou `gshopsev.data`)**: O binário `gamed` utilizava uma struct enxuta que lia apenas: `item_id`, `price` (preço em Gold), `count`, `expire_time` e `status_flags`.
- **Por que existia essa diferença?** O servidor C++ antigo foi compilado com uma struct sem os campos de UI para economizar alguns bytes de memória RAM no CentOS 32-bit. Isso obrigava a comunidade a usar ferramentas como o *GShopEditor* para exportar dois arquivos separados.

### 2.2 Decisão Arquitetural na Nova Plataforma Moderna

> [!IMPORTANT]
> **UNIFICAÇÃO TOTAL: Um Único Arquivo `gshop.data` para Cliente e Servidor**  
> No **`pw-data-loader`**, o parser foi projetado para ler a estrutura completa do cliente (com abas, categorias e preços) **OU** o formato simplificado do servidor legado de forma transparente.
> 
> **Benefício Real**: Você edita **apenas um único `gshop.data`** (no Painel Web ou no editor) e o mesmo arquivo funciona perfeitamente no cliente e no servidor, eliminando 100% dos erros de dessincronização de preços de Gold.

---

## 3. `tasks.data` (Árvore de Quests e Missões)

O `tasks.data` armazena todas as missões, cadeias de história e diálogos de NPCs:

```
+-------------------------------------------------------------------------------+
| ESTRUTURA HIERÁRQUICA DE CADA QUEST                                           |
+-------------------------------------------------------------------------------+
| • task_id (uint32)        : Identificador único da missão                     |
| • task_name (UTF-16LE)    : Nome visível no registro de missões               |
| • requirements            : Nível min/max, raça, classe, cultivo necessário   |
| • objectives              : Monstros para abater (ID e quantidade),           |
|                             Itens para coletar (ID, quantidade e drop rate)   |
| • rewards                 : EXP, Alma (SP), Moedas, Reputação, Itens e Títulos|
| • dialog_tree             : Textos de conversa com NPCs e opções de escolha   |
+-------------------------------------------------------------------------------+
```

---

## 4. `aipolicy.data` (Árvores de Inteligência Artificial de Monstros)

Define o comportamento de monstros e chefes:
- **Gatilhos (Triggers)**:
  - `OnAggro`: Quando entra em combate.
  - `OnHPPercent`: Quando a vida do monstro atinge percentuais críticos (ex: < 50% ou < 20%).
  - `OnTimer`: Intervalos periódicos de tempo (ex: a cada 15 segundos).
  - `OnAttacked`: Quando recebe dano físico ou mágico.
- **Ações (Actions)**:
  - `CastSkill`: Conjura habilidade em área ou alvo único.
  - `SummonMinions`: Invoca monstros auxiliares ao redor.
  - `SayText`: Envia fala dramática do chefe no chat local/mundo.
  - `Flee` / `SelfDestruct`: Fuga ou explosão de dano em área.

---

## 5. `npcgen.data` e Mapas de Colisão 3D

- **`npcgen.data`**: Mapeia as coordenadas tridimensionais de nascimento (*spawn zones*) de monstros, NPCs e recursos para cada sub-região do mapa (`world`, `a01`, etc.).
- **Arquivos de Colisão 3D**:
  - **`.clt` (Collision Terrain)**: Grid de altura (*heightmap*) que define o relevo de montanhas, vales e rios.
  - **`.clv` (Collision Volumes)**: Malhas poligonais 3D com caixas de colisão de construções, pontes, cavernas e árvores para cálculo de linha de visão (*Raycasting*) e prevenção de atravessar paredes.
