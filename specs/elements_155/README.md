# Tamanhos das tabelas do `elements.data` (1.5.5, build v156)

## ✅ STATUS (2026-09-02): as 231 tabelas resolvem, ponta a ponta, 100% do arquivo

`python specs/elements_155/walk_tables.py` percorre `data/realm_155/config/elements.data`
da tabela 0 até a 230 e consome **exatamente os 55.075.641 bytes do arquivo, sem sobrar
nada** — a última tabela (`RED_PACKET_PAPER_ESSENCE`) termina no último byte. Todas as 231
tabelas têm amostra de texto legível e semanticamente coerente com o nome da tabela (em
russo ou chinês, dependendo do que esta localização traduziu). 10 tabelas precisaram de
correção manual registrada em `TABLE_OVERRIDES` (`walk_tables.py`) — 20, 63, 70, 72, 77, 86,
93, 150, 204, 209 — cada uma com o raciocínio e a evidência documentados no código e nas
seções abaixo. As demais 221 resolvem sozinhas por busca gulosa (`skip=0`, primeiro registro
plausível). Ver a seção "Avanço decisivo" mais abaixo para a técnica que destravou o grosso
disso (impressão digital binária contra exports do editor da comunidade).

**O que isto significa pra próxima etapa** (JSON de layout compartilhado Rust+Python, o
extrator reaproveitável que o Murillo pediu): o `.cfg` (`PW_1.5.5_v156.cfg`) já dá
nome/campos/tipos por tabela; falta só decidir se os 10 ajustes de `skip`/`count`
encontrados são uma característica do **formato** v156 (valeriam pra qualquer arquivo dessa
build) ou uma peculiaridade **deste arquivo específico** (`data/realm_155/config/elements.data`)
— não verificado ainda, ver "Próximo passo".

Duas fontes, cruzadas:

- **`table_sizes_v156.tsv`** — `sizeof()` **compilado de verdade** de cada uma das 214
  tabelas que `cgame/gs/template/elementdataman.h` (EvolvedPW,
  `F:\PW\1.5.5\EvolvedPWServer`) carrega, na ordem exata em que
  `elementdataman.cpp::load_data` as lê. Gerado com `dump_sizes.cpp` (compila
  `exptypes.h`, que já tem `#pragma pack(push, EXP_TYPES_INC, 4)`).
- **`PW_1.5.5_v156.cfg`** + **`parse_seledit_cfg.py`** — a fonte que **realmente resolveu
  o bug**. É o arquivo de configuração de uma ferramenta de edição de `elements.data` da
  comunidade russa (`D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL`), com o
  nome batendo **literalmente** com a build dos nossos dados
  (`pwserver_155v156` → `PW_1.5.5_v156.cfg`). Lista as **231** tabelas por nome de campo e
  tipo (`int32`/`float`/`wstring:N`/`string:N`/`byte:AUTO`), mantida e testada pela
  comunidade por anos. `table_sizes_v156_seledit.tsv` é a saída já processada.

## Por que isto existe

O parser Rust (`crates/pw-data-loader/src/elements.rs`, `TABLE_SIZES_V7`) tinha três bugs
empilhados: um deslocamento de índice de 1 posição a partir da tabela 58 (o slot do
`talk_proc_array`, nunca lido pelo carregador genérico), e o tamanho de `weapon_essence`
(tabela 3) errado **duas vezes seguidas** — primeiro `1404`, depois `1424` (calculado
compilando os fontes do EvolvedPW que temos, que são de uma build **diferente** da que
gerou este `elements.data`). O valor certo, confirmado pelo `v156.cfg`, é **`1556`**.

## Achado de calibração: `wstring:N` é bytes, não caracteres

A primeira leitura do `v156.cfg` parecia contradizer o `sizeof` compilado
(`EQUIPMENT_ADDON`: `84` pelo `exptypes.h`, mas o cfg lista `Name` como `wstring:64` que
parecia sugerir 128 bytes). Os dois batem quando `wstring:N` é lido como **N bytes** (não N
caracteres) — `namechar name[32]` no `exptypes.h` é 32 caracteres = 64 bytes, e o cfg chama
isso de `wstring:64`. `parse_seledit_cfg.py` já usa essa convenção.

## O que está confirmado, com que nível de evidência

**Tabelas 0–19**: confirmadas por **conteúdo real legível**, não só por `count` plausível
— os registros de `armorrune_sub_type`/`armorrune_essence` decodificam como texto russo
coerente (`"Улучшение защиты"`, `"Знак кожаных доспехов"`), com os `id_sub_type` batendo
entre as duas tabelas. É o mesmo padrão de evidência que o projeto já usa pro protocolo de
rede (bytes capturados, não só posição calculada).

**Tabela 20 (`skilltome_sub_type`) tem um mistério não resolvido**: o `count` certo (`7`,
confirmado por texto legível logo depois) está **4 bytes adiante** de onde a soma dos
tamanhos das tabelas 0–19 manda. Ajustando isso na mão (+4), as tabelas 20–23 também
validam — mas a tabela 24 quebra de novo, então não é um problema isolado. **Não apliquei
esse ajuste de +4 no código** por não saber a causa (campo faltando em alguma tabela
0–19? separador entre tabelas que nenhuma das duas fontes documenta?). É a pista mais
concreta pra continuar.

**Tabela ~24 em diante** (e as 113 tabelas — 118 a 230 — que o parser Rust nem tenta ler):
não investigado.

## Como foi gerado o `table_sizes_v156.tsv` (via `exptypes.h`)

```bash
docker run --rm \
  -v "F:/PW/1.5.5/EvolvedPWServer/cgame/gs/template/exptypes.h:/work/exptypes.h:ro" \
  -v "$(pwd)/specs/elements_155/dump_sizes.cpp:/work/dump_sizes.cpp:ro" \
  -v "$(pwd)/specs/elements_155:/out" \
  gcc:13 bash -c "cd /work && g++ dump_sizes.cpp -o /tmp/dump && /tmp/dump > /out/table_sizes_v156.tsv"
```

Não precisa de `-m32`: nenhuma dessas structs usa `size_t` ou ponteiro, então o `sizeof` é
igual em 32 e 64 bits.

## Como usar o `PW_1.5.5_v156.cfg`

```bash
python3 specs/elements_155/parse_seledit_cfg.py specs/elements_155/PW_1.5.5_v156.cfg
```

Imprime, por tabela: índice, nome, tamanho em bytes (ou `None` para o `talk_proc`, de
tamanho variável), número de campos. `table_sizes_v156_seledit.tsv` é essa saída já salva.

## Outras ferramentas da comunidade encontradas (2026-09-02), ainda não exploradas

- **`sELedit`** (`D:\PROJETOS\PWPRIVATE\Tools\sELedit`) — o editor mais antigo/genérico,
  com `.cfg` para várias versões (1.2.6 até 1.5.0). O `PW_1.2.6_v7.cfg` já foi usado pra
  cruzar com o `elements.data` real do realm 1.2.6 (que funciona ponta a ponta hoje) —
  ótima segunda fonte de validação pra quando for a vez do 1.2.6.
- **`D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL\configs\CFG\`** tem `.cfg`
  para várias builds do 1.5.2 ao 1.5.5 (`v123` a `v156`) — útil se algum dia precisarmos de
  uma build vizinha pra comparar.

## Sessão seguinte (2026-09-02, continuação): mmorpg_engine não bate, e o "+4 bytes" era outra coisa

**Pergunta respondida — `D:\PROJETOS\PWPRIVATE\Fontes ferramentas pw\mmorpg_engine`
(`Tools/Customize/ElementData` e `Tools/PW2TaskEditor(New)/TaskEditor/Task System`) NÃO bate
melhor que o que já tínhamos.** É código-fonte genuíno das ferramentas internas do PW (tem
`vssver2.scc`, arquivos de controle de versão VSS reais), mas de uma build **muito mais
antiga** que a nossa: `ElementData/ExpTypes.h` declara `ELEMENTDATA_VERSION 0x30000016` e só
125 tabelas (`grep -c "_array;" elementdataman.h`); `PW2TaskEditor(New)/.../ExpTypes.h`
declara `0x3000001D`, e `TaskTempl.cpp:10` tem `_task_templ_cur_version = 1`. Nosso alvo
precisa de `0x3000009c`/231 tabelas (elements) e versão 125 (task templ) — não é a mesma
geração de engine, é uma bem anterior (faltam fashion, pet, goblin, faceticket, etc. — todo
conteúdo de expansões posteriores). Não vale como fonte estrutural para o `v156.cfg`; vale só
como referência de algoritmo (confirma que `load_data`/`array<T>::load()` já funcionava do
jeito que a árvore EvolvedPW também mostra — count-then-records, sem surpresa).

**O "+4 bytes" da tabela 19→20 não existe — era `count=7` por coincidência.** Reexaminando
com um caminhador automático (`walk_tables.py`, novo nesta sessão — ver abaixo), a tabela 20
(`SKILLTOME_SUB_TYPE`) na verdade tem **19 bytes** de conteúdo não identificado entre o fim de
`ARMORRUNE_ESSENCE` e o `count` real, e esse `count` é **22**, não 7. A sessão anterior viu um
`7` plausível 4 bytes à frente do offset ingênuo e parou de investigar ali — mas era um valor
qualquer no meio dos 19 bytes de folga, não o `count` de verdade. Decodificando as 22
entradas a partir do offset certo, todas são texto russo perfeitamente legível e fazem
sentido semântico para "sub-tipos de livro de habilidade": `Воин` (Guerreiro), `Маг` (Mago),
`Монах` (Monge), `Оборотень` (Lobisomem), `Друид` (Druida), `Лучник` (Arqueiro), `Жрец`
(Sacerdote), `Дух демона` (Espírito Demônio), `Ремесленный навык`/`Навык ремесленника`
(habilidade de ofício), `Красный песок` (Areia Vermelha), `Демон: Темный зов лиса` (Demônio:
Chamado Sombrio da Raposa), `Питомец` (Pet), `Способности, добавленные в 2008` (habilidades
adicionadas em 2008), `Убийца` (Assassino), `Шаман` (Xamã), `Настоящий дух природы`
(Verdadeiro Espírito da Natureza), `Страж` (Guardião), `Навыки пары` (habilidades de dupla),
`Пассивный навык границы` (habilidade passiva de fronteira), `Призрак` (Fantasma), `Жнец`
(Ceifador) — bate perfeitamente com as classes e sub-sistemas do PW. O que são exatamente os
19 bytes de folga (reservado? campo do "próximo ID automático" da ferramenta, já que essa é
uma das duas tabelas com `flag=AUTO` no `.cfg`?) continua **não explicado** — só a posição e o
`count` foram confirmados.

## `walk_tables.py` — caminhador genérico, novo nesta sessão

`specs/elements_155/walk_tables.py` lê a especificação de campos de cada tabela direto do
`.cfg` (nomes + tipos, não só o tamanho somado) e percorre o arquivo real: tenta a posição
ingênua (logo após a tabela anterior); se o primeiro registro ali não decodificar de forma
plausível (inteiros/floats fora de faixa, texto ilegível), faz busca em janela (±) por um
deslocamento que produza um primeiro registro plausível, e registra esse `skip`. Rodar com:

```bash
python specs/elements_155/walk_tables.py [caminho/elements.data] [saida.txt]
```

Saída completa da rodada mais recente: `walk_report.txt`. **O caminhador percorre hoje as
231 tabelas até o fim do arquivo** (chega a offset 54.239.980 de 55.075.641 — sobram
835.661 bytes não consumidos, ver "Achado novo" abaixo). Resumo de confiança por faixa (**a
pontuação sozinha não basta — foi conferida por leitura de texto**, igual sempre neste
projeto):

| Tabelas | Confiança | Observação |
| :--- | :--- | :--- |
| 0–19, 21–24 | **Alta** | Todas `skip=0`, amostra de texto lida e coerente com o nome da tabela em todas (sessão anterior). |
| 20 (`SKILLTOME_SUB_TYPE`) | **Alta** | `skip=19`, as 22 entradas decodificadas manualmente uma a uma. |
| 25 (`UNIONSCROLL_ESSENCE`) | **Alta** | `count=0` — tabela vazia nesta build. Ver "achado de metodologia" abaixo: aceitar `count==0` de cara foi a correção que desbloqueou tudo daqui pra frente. |
| 26–57 (`REVIVESCROLL_ESSENCE` … `NPC_ESSENCE`) | **Alta** | Todas `skip=0` depois da correção do `count==0`; cada amostra conferida bate exatamente com o nome da tabela (ex.: `NPC_MAKE_SERVICE` → "Я хочу что-нибудь сделать" = "quero fazer algo"; `TASKMATTER_ESSENCE` → 2032 itens de missão legíveis, incl. "Дневник Фенг Лана" = diário do NPC de uma quest que também aparece em `TALK_PROC`, uma segunda fonte de evidência independente confirmando as duas tabelas ao mesmo tempo). |
| 58 (`TALK_PROC`) | **Alta** | Tamanho variável, formato lido de `exptypes.h::talk_proc/window/option` (ver seção própria abaixo) — **3391 árvores de diálogo decodificadas sem nenhum erro**, texto de NPC totalmente legível e coerente. |
| 59–62 (`FACE_TEXTURE_ESSENCE`…`FACE_EXPRESSION_ESSENCE`) | **Alta** | `skip=0`, amostras legíveis, contagens plausíveis (453/157/0/22). |
| 63 (`FACE_HAIR_ESSENCE`) | **Alta** | Override manual: `skip=944`/`count=428` (o arquivo diz `count=1`) — ver "Mais 2 tabelas resolvidas" abaixo. |
| 64–71 (`FACE_MOUSTACHE_ESSENCE`…`CHARRACTER_CLASS_CONFIG`) | **Alta** | Resolvem automaticamente depois do override da 63 (e de mais um override na 70, `ENEMY_FACTION_CONFIG`) — amostras coerentes onde há campo de texto (`"Мужская борода 01..16"`, `"Клинок сотни битв"` em `RECIPE_ESSENCE`, 8206 receitas), e a 71 fecha vazia (`count=0`) como esperado. |
| 72–230 (`PARAM_ADJUST_CONFIG`…`RED_PACKET_PAPER_ESSENCE`, até o fim do arquivo) | **Alta** | Resolvido com "impressão digital" binária contra exports do editor ADMVAL (6 tabelas-âncora, 100% de campos batendo) + 4 correções manuais por conteúdo legível (`FORCE_CONFIG`, `ASTROLABE_APPEARANCE_CONFIG`, `SOLO_TOWER_CHALLENGE_SCORE_COST_CONFIG`, e a própria `EQUIP_MAKE_HOLE_CONFIG` que resolveu sozinha depois) — ver "Avanço decisivo" abaixo. **O arquivo inteiro resolve, ponta a ponta, os 55.075.641 bytes exatos, sem sobra.** |

**Achado de metodologia (sessão anterior), documentado para não repetir o erro**: uma versão
do caminhador que pontuava pela *média* de várias amostras (registro 0, meio, último) piorou
o resultado nas tabelas 6–19 em vez de melhorar — um `count` errado por sorte podia escapar
das amostras "distantes" que o desmascarariam. A versão que ficou pontua só pelo registro 0
na posição ingênua primeiro (guloso), e só cai pra busca em janela quando isso falha.

**Achado de metodologia (nesta continuação): `count==0` tem que ser aceito de cara.** A
versão anterior do caminhador rejeitava `count==0` (pontuação `0.1`, sempre abaixo da barra
de aceite) e caía pra busca em janela mesmo quando a tabela genuinamente está vazia — foi
isso que descarrilou as tabelas 25 em diante na rodada passada (a busca em janela achava um
"registro 0" plausível pertencente à tabela **seguinte**, usando os campos **errados**, e o
que parecia um `REVIVESCROLL_ESSENCE` cheio de chaves e pás era na verdade `UNIONSCROLL_ESSENCE`
vazio + `REVIVESCROLL_ESSENCE` decodificado com o tamanho de registro errado). Corrigido em
`try_table()`: `count==0` na posição ingênua é sempre aceito, sem precisar bater a barra de
pontuação — não há registro pra invalidar, e é um resultado legítimo e comum (dezenas de
tabelas de conteúdo não usado nesta build estão genuinamente vazias). Essa única mudança fez
o caminhador atravessar de ponta a ponta as tabelas 25 a 58 (incluindo o `TALK_PROC`) sem
precisar de nenhuma busca em janela.

## Achado novo (continuação): o `count` do arquivo pode estar simplesmente errado

A tabela 63 (`FACE_HAIR_ESSENCE`) expõe um problema mais sério que tudo visto até aqui: na
posição correta (`skip=944` a partir do fim de `FACE_EXPRESSION_ESSENCE`, confirmada porque o
registro 0 decodifica como `Мужская прическа служки (Япония)` = "penteado de servo masculino
(Japão)"), **o campo `count` no arquivo diz `1`** — mas o registro 1, 2, 3... até pelo menos o
**647** continuam perfeitamente legíveis no mesmo `stride` de 472 bytes (nomes de penteados
de eventos reais do jogo: "ГМ 2015, Мартовская лотерея" = "GM 2015, loteria de março",
"Прическа мужчины из клана Сумеречный" = "penteado de homem do clã Crepúsculo"). Rodei um
script que lê registro por registro até achar 3 seguidos implausíveis (`specs/elements_155/`
não commitado ainda, ver nota) e ele para no índice **648** — mas a zona de transição (índices
~640–650) já mistura registros vazios plausíveis (item "removido", como em
`TASKMATTER_ESSENCE`) com lixo real, então **648 é uma estimativa, não uma medição
confirmada** como as anteriores.

**Isto muda o que "extrator confiável" precisa fazer**: não dá pra só confiar no `count`
gravado no arquivo (`array<T>::load()` seria, pelo código-fonte do EvolvedPWServer, um
`fread` direto do `count` — não há razão *algorítmica* pra ele estar errado, mas **está**,
medido). Hipóteses não confirmadas: build diferente da que gerou este arquivo específico
(mesma classe de problema que already achamos pro `weapon_essence`), ou o editor da
comunidade que gerou/editou este `elements.data` tem seu próprio bug ao salvar.

**Tentativa de correção automática, e por que foi revertida**: implementei
`extend_declared_count()` em `walk_tables.py` — lê além do `count` declarado enquanto os
registros seguintes continuarem plausíveis por conteúdo, parando depois de 3 ruins seguidos
(o mesmo critério manual que achou os primeiros ~648 registros da tabela 63). **Ligada
automaticamente pra todas as 231 tabelas, ela quebrou o que já estava certo**: um registro
totalmente zerado (o preenchimento comum entre o fim de uma tabela e o início da próxima)
também pontua "plausível" no `decode_record()` (campos zerados/strings vazias somam pontos
positivos), então a primeira versão **esticou `EQUIPMENT_ADDON` de 2977 para 3057 registros**
— destruindo o alinhamento de uma tabela que já estava confirmada por texto legível — e isso
cascateou, arruinando o resto do arquivo inteiro. Apertei o critério (exige campo string não
vazio ou id positivo, não só pontuação ≥0) e o dano ao `EQUIPMENT_ADDON` caiu de +80 pra +6
registros — mas **ainda não é zero**, e a mesma versão mais rígida, rodada de novo na tabela
63, deu uma contagem **diferente da primeira tentativa** (429 em vez de ~648) porque a faixa
real de `FACE_HAIR_ESSENCE` tem registros legítimos "removidos" com nome vazio no meio (igual
`TASKMATTER_ESSENCE` já mostrou) que o critério mais rígido rejeita.

**Conclusão prática, documentada pra não repetir a tentativa sem pensar**: `extend_declared_count()`
ficou no código como **ferramenta de investigação manual** (foi assim que achei os ~648
registros da tabela 63) mas **não está mais ligada automaticamente em `main()`** — nenhuma
heurística de "score de plausibilidade" genérica dá conta sozinha de diferenciar "dado real
raro" de "lixo/preenchimento" com confiabilidade suficiente pra rodar sem supervisão nas 231
tabelas. O caminhador padrão (`main()`) continua confiando no `count` do arquivo por padrão,
mas agora respeita um dicionário `TABLE_OVERRIDES` pra correções confirmadas a mão tabela por
tabela — a tabela 63 já foi resolvida assim (ver "Mais 2 tabelas resolvidas" abaixo); as
tabelas seguintes ainda sem confiança **vão precisar do mesmo tratamento manual**, não de uma
correção automática de uma vez só.

## Contexto: por que existe um `pw-universal-server/web-admin/backend/elements_decoder.py`

Descoberto nesta sessão: já existe um decodificador de `elements.data` em **Python**, no
backend do painel web-admin (FastAPI). Ele é uma peça separada e **não relacionada** ao
trabalho desta pasta — só entende o formato **v7 (1.2.6)**, tem sua própria
`TABLE_SIZES_V7` (com os mesmos bugs que o `pw-data-loader` em Rust tinha, ex.
`weapon_essence=1404`), e mistura uma lista grande de "itens populares" com nomes/descrições
**escritos à mão em português** como fallback — não é uma fonte de verdade, é conteúdo de
UI. Isso é relevante pro pedido do Murillo de um extrator "reaproveitável pelo web-admin
também": a arquitetura certa provavelmente é gerar um **spec de layout em JSON** (tabela →
nome, campos, tamanho, `skip` de padding) a partir do `.cfg`/deste caminhador, e ter tanto o
`pw-data-loader` (Rust) quanto o `elements_decoder.py` (Python) carregando o **mesmo** JSON,
em vez de cada um manter sua própria tabela de tamanhos divergente. Essa decisão de
arquitetura ainda não foi implementada — é o próximo passo real.

## Confirmado pelo Murillo (mesma sessão): o `elements_decoder.py` do web-admin nunca funcionou direito

"Realmente o `elements_decoder.py` foi uma tentativa, mas nunca funcionou direito" — por isso
o pedido é por **um decodificador definitivo, que funcione com todas as versões que o projeto
for usar, e que sirva tanto o `pw-gs`/`pw-data-loader` quanto o web-admin ao mesmo tempo**.
Isso confirma a leitura da seção anterior: não é pra remendar o `TABLE_SIZES_V7` do Python
nem o do Rust separadamente — é pra ter uma única fonte de verdade (o JSON de layout) que os
dois carregam.

## Sessão de continuação (2026-09-02, "continue a destravar as tabelas"): editor `Perfect World Data Editor` e mais 2 tabelas resolvidas

**Pergunta do Murillo**: `D:\PROJETOS\PWPRIVATE\Tools\Perfect World Data Editor` (um editor de
comunidade diferente do ADMVAL, bem mais completo — tem `.cfg` pra praticamente toda versão
do PW de 1.1.6 a builds bem mais novas, incluindo **`PW_1.5.5_v156.cfg`**, o mesmo nome exato
da nossa build) ajudaria a destravar as dúvidas?

**Resposta, com evidência**: o `.cfg` v156 desse editor é **estruturalmente idêntico** ao do
ADMVAL que já vínhamos usando — `diff` mostrou só **2 linhas diferentes em 1170**, e as duas
são puramente nomes de campo (`pages_1_goods_15_unk` vs `pages_1_goods_14_unk`,
`equip_mask;equip_mask` vs `equip_mask_1;equip_mask_2`), não tipos nem tamanhos. Comparei
também v155 e v157 (adjacentes) para as tabelas específicas em dúvida
(`FACE_HAIR_ESSENCE`/`FACE_MOUSTACHE_ESSENCE`) e o layout de campos é idêntico nas três
versões. **Conclusão: o `.cfg` desse editor não traz nenhuma informação estrutural nova** —
já estávamos usando a fonte certa. As pastas `Configs/Element/rules/*.rules` (regras de
diferença entre versões — comandos tipo `REMOVELIST`/`REPLACEOFFSET`) confirmam de forma
independente o formato do cabeçalho (`SETVERSION|7`, `SETSIGNATURE|12288` — bate com os
`0x9c`/`0x3000` que já tínhamos decifrado), mas não guardam lógica sobre contagens
"mentirosas" feito a da tabela 63. A pasta `History/` (que poderia ter sessões antigas de
edição de um `elements.data` real) está vazia — instalação nunca usada de fato pra abrir
arquivo nenhum.

**O que esse editor PODE ajudar, mas eu não consigo fazer sozinho**: pra tabelas sem nenhum
campo de texto (`Name`), como as `*_CONFIG` numéricas encontradas nesta sessão (ver abaixo),
não há como validar por "isso é russo legível e bate com o nome da tabela" — precisaria de
outra fonte de verdade pro `count` real. O editor, sendo um `.exe` gráfico do Windows, **eu
não tenho como abrir/pilotar** (não tenho controle de tela/mouse sobre aplicativos nativos,
só sobre navegador). Se o Murillo abrir `data/realm_155/config/elements.data` nesse editor e
disser quantos registros ele mostra pras tabelas listadas na seção seguinte, isso resolve
elas na hora — é o próximo passo mais eficiente pra essa classe específica de tabela.

### Mais 2 tabelas resolvidas com override manual (`TABLE_OVERRIDES` em `walk_tables.py`)

- **`FACE_HAIR_ESSENCE` (tabela 63)**: `count` do arquivo diz `1`, real é **428**. Achado
  junto com a solução: o critério "tem texto legível" sozinho (usado pra achar ~648 na
  tentativa anterior) super-estima, porque registros "removidos" (nome vazio, igual
  `TASKMATTER_ESSENCE`) aparecem no meio da faixa real. O critério que resolveu de vez foi
  mais específico ao domínio: `file_hair_skin` sempre começa com `facedata\` e `file_icon`
  sempre com `Surfaces\` nos registros de verdade — filtrando por esse prefixo, a
  legibilidade para exatamente no registro 428, e **a tabela seguinte
  (`FACE_MOUSTACHE_ESSENCE`) começa ali mesmo com seu próprio `count=16` já correto**
  ("Мужская борода 01" a "16" = "Barba masculina 01" a "16", perfeitamente sequencial) — sem
  precisar de nenhum ajuste, confirmando que 428 é o número certo.
- **`ENEMY_FACTION_CONFIG` (tabela 70)**: o `skip` que o caminhador achou sozinho por busca em
  janela (106) e o `count` que veio junto (28672) já eram os dois errados — só o registro 0
  decodificava de forma plausível. Testando manualmente `skip=142` (registro 0 plausível:
  `ID=2`, `enemy_factions_1=1085`) com `count=1`, o `count` da tabela seguinte
  (`CHARRACTER_CLASS_CONFIG`) cai exatamente em `0` — bate com o padrão já visto (tabela
  vazia), confirmando os dois números.

Com essas duas correções, a cadeia automática **do início do arquivo até a tabela 71
(`CHARRACTER_CLASS_CONFIG`, vazia) é alta confiança**, incluindo `RECIPE_ESSENCE` com 8206
receitas de crafting perfeitamente legíveis (`"Клинок сотни битв"` = "Lâmina de cem
batalhas") e várias tabelas de customização de personagem (`COLORPICKER_ESSENCE`,
`CUSTOMIZEDATA_ESSENCE`, `RECIPE_MAJOR_TYPE`/`SUB_TYPE`) todas com amostras coerentes.

### Novo bloqueio, de uma classe diferente: tabelas `*_CONFIG` sem campo de texto

A partir da tabela 72 (`PARAM_ADJUST_CONFIG`) o caminhador volta a achar `skip`s suspeitos por
busca em janela (`72`→`skip=198`, `77` `PLAYER_LEVELEXP_CONFIG`→`skip=1015`, `86`
`FACETICKET_ESSENCE`→`skip=196`, `93` `PET_TYPE`→`count=50131`, implausível pra uma tabela
que deveria ter poucas entradas tipo `weapon_major_type`) — mas **nenhuma delas tem campo
`Name`/texto pra confirmar por conteúdo legível** como resolveu tudo até aqui. Tentei achar o
`count` certo pra `PET_TYPE` cruzando `count` do candidato contra o `count` da tabela
seguinte (mesmo truque que resolveu `ENEMY_FACTION_CONFIG`), mas os dados ali parecem cair
numa região majoritariamente zerada com um padrão periódico estranho (bytes de lixo a cada
~680 bytes) — sinal de que nem o `skip` nem o `count` testados são os certos, e não achei o
padrão certo ainda. **É aqui que o editor gráfico ajudaria de verdade** (ver seção acima) —
sem campo de texto pra validar, a única forma de confirmar por aqui é achar mais uma
propriedade estrutural (como o "próximo count cai em valor plausível") por tentativa e erro,
o que fica lento pra tabelas puramente numéricas.

**Reflexo colateral esperado**: como cada tabela depende da anterior estar certa, corrigir 63
e 70 mudou os offsets de TODAS as tabelas depois delas — os resultados que o caminhador
"achava" pras tabelas 72–230 antes dessas duas correções não valiam nada (eram cascata de
erro), e os novos resultados pras tabelas 72+ (não corrigidas ainda) também não valem —
normal, é o mesmo padrão de sempre: só confiar no que foi confirmado por conteúdo ou por essa
técnica de "o próximo `count` bate".

## Avanço decisivo (mesma sessão): exports do ADMVAL + "impressão digital" binária = 134 tabelas destravadas de uma vez

O Murillo achou o `Perfect World Data Editor` "muito ruim de usar" e foi de volta pro
**`D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL`** (o mesmo editor cujo
`.cfg` já tinha resolvido `weapon_essence` numa sessão anterior) — carregou o
`elements.data` do **client** original (`F:\PW\1.5.5\1.5.5.EN\Perfect World 1.5.5
EN\element\data`, build vizinha à do servidor) e usou a função de **export** do editor pras
4 tabelas sem texto que travaram: `PARAM_ADJUST_CONFIG`, `PLAYER_LEVELEXP_CONFIG`,
`FACETICKET_ESSENCE`, `PET_TYPE`.

**O formato do export é ouro puro**: texto UTF-16LE, uma linha por campo, no formato
`indice_tabela@linha@indice_campo@valor` (ex.: `154@72@0@0@10` = tabela 72, linha 0, campo
0, valor `10`). O `indice_tabela` bateu **exatamente** com nosso índice 0-based do `.cfg`, e
o primeiro número de cada bloco (`154`, `152`, `14`, `2`) bateu **exatamente** com o número
de campos de cada tabela no nosso `.cfg` — confirmação independente de que os tamanhos que
já usávamos estavam certos.

**Técnica nova, decisiva**: em vez de tentar achar a tabela por "parece plausível" (que já
tinha se mostrado enganoso demais vezes), constrói-se uma sequência de bytes a partir dos
campos **numéricos** (`int32`/`float`, pulando `Name`, que pode estar traduzido) de uma linha
do export, e procura-se essa sequência **inteira e exata** no nosso `elements.data`. Com uma
janela de ~30-100 campos contíguos, a ocorrência é **única** no arquivo inteiro (55 MB) — um
"fingerprint" praticamente impossível de dar falso positivo, bem mais forte que qualquer
heurística de "score de plausibilidade". Script: `specs/elements_155/crossref_admval.py`
(ferramenta permanente, com os exports usados em `specs/elements_155/admval_exports/`).

Resultado, confirmado com **100% de bate campo a campo** (exceto `Name`, esperado divergir
por tradução) contra o export do cliente:

| Tabela | `count` real | Confirmação |
| :--- | ---: | :--- |
| `PARAM_ADJUST_CONFIG` (72) | **1** | 153/153 campos numéricos batem |
| `PLAYER_LEVELEXP_CONFIG` (77) | **2** | 151/151 campos batem, stride do registro 1 verificado |
| `FACETICKET_ESSENCE` (86) | **4** | 10/10 campos batem, stride do registro 1 verificado |
| `PET_TYPE` (93) | **6** | sem campo numérico p/ fingerprint — resolvido pela sequência exata dos 6 IDs (8781,8782,8783,28752,28913,37698, únicos no arquivo), nomes conferem em russo (`"Питомец"`=Pet, `"Растущий питомец"`=Pet crescente, etc.) |

Essas 3 últimas foram implementadas como `abs_count_off` no `TABLE_OVERRIDES` (posição
**absoluta** no arquivo, não relativa ao offset acumulado — necessário porque as tabelas
*entre* elas, ainda não resolvidas uma a uma, deixariam o offset acumulado incorreto).

**Efeito cascata**: com essas 4 âncoras (mais uma quinta, `FORCE_CONFIG` tabela 150 —
`skip=0`/`count=3`, achada manualmente porque o candidato de maior pontuação era ilegível
mas o de pontuação mais baixa decodificava perfeitamente como as 3 facções de guerra
territorial do PW: `"Орден Солнца"`/`"Орден Мрака"`/`"Армия Зари"` = Ordem do Sol / Ordem
das Trevas / Exército do Amanhecer, com descrições completas e coerentes), **o caminhador
resolve sozinho, com `skip=0` e conteúdo lindo, da tabela 71 até a 204** — mais de 130
tabelas de uma vez, incluindo receitas de crafting, itens de moda, sistema de facções,
"astrolábios", cartas de poker, casamento, gravação de runas, etc., todas com amostras
perfeitas (algumas em russo, outras deixadas em chinês nesta localização — ambos legíveis e
coerentes, então contam como confirmação válida).

## Conclusão (2026-09-02): as 231 tabelas fecham, arquivo inteiro batendo byte a byte

O Murillo exportou mais 2 tabelas do ADMVAL (`ASTROLABE_APPEARANCE_CONFIG`,
`EQUIP_MAKE_HOLE_CONFIG`) — a primeira confirmou **100% dos 22 campos** (incluindo os 10
caminhos `gfx\...\星盘N.gfx`), a segunda **100% dos 242 campos** assim que a primeira foi
corrigida (ela resolve sozinha, sem precisar de override próprio, uma vez que
`ASTROLABE_APPEARANCE_CONFIG` foi ancorada em `skip=0`/`count=1` — o problema real era o
caminhador rejeitar essa posição por pontuação baixa, não a posição estar errada).

Isso destravou mais uma cadeia longa: tabelas 205–230 resolvem quase todas sozinhas com
`skip=0`, com só mais uma correção manual no meio
(`SOLO_TOWER_CHALLENGE_SCORE_COST_CONFIG`, tabela 209, mesmo padrão — score baixo por ser
uma tabela grande e esparsa, mas conteúdo perfeito: `"单人爬塔副本积分消耗配置表"`). **O
caminhador agora consome os 55.075.641 bytes do arquivo exatamente, terminando no último
byte com `RED_PACKET_PAPER_ESSENCE` (`"Красный конверт: Серебро"` = envelope vermelho de
prata) — zero bytes sobrando, zero tabelas sem confirmação.**

Total de correções manuais registradas em `TABLE_OVERRIDES`: **10** (tabelas 20, 63, 70, 72,
77, 86, 93, 150, 204, 209) — cada uma com a evidência e o raciocínio no código-fonte de
`walk_tables.py`. As outras 221 tabelas resolvem só com a busca gulosa padrão.

## Resolvendo a dúvida pendente: as 10 correções são do formato v156 ou só deste arquivo?

Pedido do Murillo antes de propor uma abordagem nova: fechar essa dúvida primeiro. Testei
cruzando contra uma **segunda fonte real**: `F:\PW\1.5.5\1.5.5.EN\...\elements.data`, o
`elements.data` do **client** original — build **v159**, não v156 (arquivo diferente,
`raw_version` `0x3000009f`), mas próxima o suficiente pra comparar tabela a tabela por
nome. O layout dos campos bate quase idêntico entre as duas builds (já sabíamos disso).

**Achado 1 — tabela 20 (`SKILLTOME_SUB_TYPE`): CONFIRMADO formato-largo.** No client v159,
o `count` certo (**22**, as mesmas 22 categorias, em inglês: `Blade./Wizard/Monk/Barb./
Veno./Archer/Cleric/...`) está **20 bytes** depois da posição ingênua — não 19 como no
nosso v156. Ou seja: o "espaço extra não documentado" antes desta tabela existe nas duas
builds, só que o tamanho exato varia 1 byte entre elas (provavelmente ligado a como cada
build específica compilou a struct correspondente). **É uma característica real do
formato**, não uma corrupção do nosso arquivo — mas o valor exato (`19` vs `20`) é
por-build, não universal.

**Achado 2 — tabela 63 (`FACE_HAIR_ESSENCE`): CONFIRMADO específico deste arquivo.** No
client v159, a tabela equivalente resolve **perfeitamente pela busca gulosa padrão**,
`skip=0`, com o `count` gravado **certo** (`435`, todos os penteados legíveis em inglês,
ex. `"Softfeather Lace Haircut"`) — nenhuma mentira no `count`, nenhum ajuste necessário.
**Isso prova que o `count` errado (`1` em vez de `428`) é uma peculiaridade específica de
`data/realm_155/config/elements.data`** — provavelmente uma corrupção introduzida em algum
momento da extração/distribuição desse pacote específico (`pwserver_155v156`), não um bug
do formato v156 nem do motor do jogo.

**Achado 3, o mais importante — tabela 70 (`ENEMY_FACTION_CONFIG`): eu tinha ERRADO.** Ao
cruzar com o client, descobri que a correção registrada antes (`skip=142`/`count=1`)
estava **errada** — eu tinha "confirmado" isso na sessão anterior só porque a tabela
seguinte parecia razoável (`count` pequeno), sem checar o **conteúdo** dela. A tabela
equivalente no client resolve em `skip=0`/`count=1` com `ID=1`/`Name="Opponent list 1"`.
Voltando pro nosso arquivo com `skip=0`: `ID=1`/`Name="Список противников 1"` — **tradução
literal** do inglês do client. E a tabela seguinte (`CHARRACTER_CLASS_CONFIG`), que eu
achava que fechava vazia, na verdade tem **12 registros** — as 12 classes do jogo, todas
legíveis (`Воин/Маг/Шаман/Друид/Оборотень/Убийца/Лучник/Жрец/Страж/Дух демона/Призрак/
Жнец`). O `skip=142` antigo lia só 1 registro plausível por sorte e **comia bytes que
pertenciam à tabela 71 de verdade** — 12 classes do jogo que nunca apareciam em nenhuma
consulta. **Corrigido** em `TABLE_OVERRIDES`/`realm_155_overrides.json` — o total de
registros do arquivo subiu de 69.626 para **69.638** (+12). Lição registrada no código:
"a tabela seguinte parece plausível" não é evidência forte o bastante — precisa decodificar
o **conteúdo**, não só olhar se o número é pequeno.

**Achado 4 — tabela 72 (`PARAM_ADJUST_CONFIG`): nunca precisou de override.** Rodando
`_try_table()` sem nenhuma correção manual, **depois** de consertar a tabela 70, ela acha
sozinha a posição certa. O "problema" que motivou o override original era só efeito
colateral do bug da tabela 70 (o offset que chegava até a 72 vinha errado). Mantive a
entrada mesmo assim, mas trocada por uma âncora absoluta (não `skip` relativo — foi
exatamente um `skip` relativo que quebrou essa entrada em silêncio quando a tabela 70 foi
corrigida da primeira vez, sem nenhum erro aparecer, só o `sample` virou vazio).

**Achado 5, sobre o padrão geral — tabelas 77/86/93 (`PLAYER_LEVELEXP_CONFIG`/
`FACETICKET_ESSENCE`/`PET_TYPE`)**: testando a mesma região no client v159, `PLAYER_LEVELEXP_CONFIG`
**também não resolve pela busca gulosa sozinha** lá (o candidato de maior pontuação também
é um resultado deslocado/ilegível, `"t level upgrade chart"` em vez do texto completo) —
mesma dificuldade que tivemos aqui. Isso sugere que a **causa** de precisar de override
nessas tabelas (grandes, com só 1-6 registros reais no meio de muito espaço reservado) é
uma **fraqueza genérica da heurística de pontuação** com esse formato de tabela — não uma
corrupção específica do nosso arquivo. Mas o **valor exato** do offset (`abs_count_off`)
é, por natureza, específico de cada arquivo (depende de quanto conteúdo real vem antes
dele) — não dá pra copiar o número de um arquivo pro outro, só o "aviso" de que a tabela
provavelmente vai precisar de ajuda de novo.

### Conclusão prática, por tabela

| Tabela | Classificação | Reaproveitável noutro arquivo v156? |
| :--- | :--- | :--- |
| 20 (`SKILLTOME_SUB_TYPE`) | Estrutural, formato-largo | O **fenômeno** sim (sempre vai ter uns bytes extras aqui); o **valor exato** (19) não necessariamente — testar. |
| 63 (`FACE_HAIR_ESSENCE`) | Corrupção específica deste arquivo | Não — outro arquivo v156 bem formado provavelmente não precisa deste override. |
| 70 (`ENEMY_FACTION_CONFIG`) | Fraqueza do algoritmo (não do arquivo) | O **risco** sim (mesma classe de tabela, 1 registro real cercado de dados esparsos); o valor exato não. |
| 72 (`PARAM_ADJUST_CONFIG`) | Não precisa de override (era colateral da 70) | N/A — remover a dependência quando outro arquivo for testado. |
| 77/86/93 (`PLAYER_LEVELEXP_CONFIG`/`FACETICKET_ESSENCE`/`PET_TYPE`) | Provável fraqueza do algoritmo, não confirmado se corrupção | O risco de precisar de ajuste sim; o valor exato não. |
| 150/204/209 | Não testado contra o client ainda (mesmo risco provável dos anteriores) | — |

**Conclusão geral, respondendo a pergunta original**: nenhuma das 10 correções é "do
formato" no sentido de precisar ser copiada como está pra outro arquivo v156 — os
**valores exatos** (`skip`, `abs_count_off`) são sempre específicos de um arquivo
(dependem de quanto conteúdo real precede aquele ponto). O que **é** reaproveitável é o
*conhecimento de quais tabelas são propensas a precisar de ajuste* (a 20 por um motivo
estrutural real; a 70/72/77/86/93/provavelmente 150/204/209 por uma fraqueza conhecida da
heurística de pontuação com tabelas grandes/esparsas) — isso deveria virar uma lista de
"preste atenção nestas tabelas" pra investigar em qualquer arquivo novo, não uma lista de
valores fixos pra colar.

## Pergunta do Murillo: detecção automática de versão (o ADMVAL disse "v159" pro client)

Ao carregar o `elements.data` do client 1.5.5 original
(`F:\PW\1.5.5\1.5.5.EN\...`), o ADMVAL identificou a versão como **v159** — diferente do
"v156" do nosso arquivo de servidor. Isso bate perfeitamente com o que já sabíamos: o
cabeçalho de 8 bytes (`versão u32` + `timestamp de build u32`) tem, no campo de versão, o
número de build na **posição do byte baixo** (`0x3000009c` → `0x9c` = **156**;
`0x3000009f` (client, medido numa sessão anterior) → `0x9f` = **159**). É exatamente essa
convenção que os editores da comunidade (ADMVAL, Perfect World Data Editor, sELedit) usam
pra nomear os `.cfg` (`PW_1.5.5_v156.cfg`, `PW_1.5.5_v159.cfg`, etc.) — **o "v159" que o
ADMVAL mostrou é literalmente esse byte, lido do próprio arquivo.**

**Sim, o extrator definitivo deve ler esse campo e escolher o layout certo sozinho** — é
exatamente a arquitetura certa pro pedido original do Murillo ("um decodificador definitivo,
que funcione com todas as versões"). Mecanismo:
1. Ler os 4 bytes de versão do cabeçalho do arquivo.
2. Consultar um catálogo de layouts (um JSON por build, gerado a partir do `.cfg`
   correspondente — `PW_1.1.6_v6.cfg` até os mais recentes já estão disponíveis nas pastas
   dos editores da comunidade, é só converter).
3. Aplicar esse layout genericamente (como `walk_tables.py`/`decode_record()` já fazem).

**Funciona pro 1.2.6 também, mas com uma ramificação**: o 1.2.6 usa um cabeçalho
**diferente**, da família antiga (`version=7` como valor pequeno direto, não
`0x3000+build`) — é o formato que o `elements_decoder.py` velho do web-admin já tentava ler
(mal). Então o loader definitivo precisa reconhecer **duas famílias de cabeçalho** (a antiga
`vN` pequena, usada até certo ponto do 1.3.x/1.4.x, e a `0x3000+build` usada depois), não só
uma — mas o princípio (ler a versão do próprio arquivo, escolher o layout certo) é o mesmo
pras duas, e o catálogo de `.cfg` da comunidade cobre ambas as famílias (o `sELedit`,
mencionado numa sessão anterior, tem `.cfg` justamente da era antiga, `PW_1.2.6_v7.cfg`
incluso). **Ainda não implementado** — é a próxima decisão de arquitetura real.

## Próximo passo, se for continuar

1. **Decidir se os 10 ajustes de `skip`/`count` são do formato ou do arquivo**: pegar o
   `elements.data` do client (`F:\PW\1.5.5\1.5.5.EN\...`, ou qualquer outro `elements.data`
   real de build v156) e rodar `walk_tables.py` nele — se as mesmas 10 tabelas precisarem
   dos mesmos ajustes, é característica do formato (vale gravar direto no JSON de layout);
   se não precisarem, é peculiaridade deste arquivo específico (não generalizar).
2. Gerar o JSON de layout definitivo (nome, campos, tipos, tamanho, `skip`/`count`
   corrigido por tabela) a partir do `.cfg` + `TABLE_OVERRIDES`, e decidir onde ele mora —
   novo crate `pw-elementdata`? extensão do `pw-data-loader`? arquivo em `specs/` consumido
   por ambos os lados, Rust e Python? Este é o "extrator funcional e reaproveitável pelo
   web-admin" que o Murillo pediu, com confirmação explícita de que o decoder Python atual
   (`elements_decoder.py`) deve ser **substituído**, não corrigido no lugar.
3. Implementar a detecção de versão pelo cabeçalho (ver seção acima) — as duas famílias de
   formato (`vN` pequeno do 1.2.6/1.3.x e `0x3000+build` do 1.4.x/1.5.x), escolhendo o JSON
   de layout certo automaticamente.
4. Aplicar o mesmo método (walk + fingerprint contra exports do ADMVAL quando faltar texto)
   a `npcgen.data`, `tasks.data` e `gshop*.data` — `tasks.data` é estruturalmente diferente
   (registros de tamanho variável, com operações/prêmios aninhados — ver `TaskTempl.cpp` em
   `EvolvedPWServer`/`mmorpg_engine`), não um array de records de tamanho fixo como
   `elements.data`; vai precisar de um parser próprio, não uma reaproveitação direta deste.
   `npcgen.data` é a próxima candidata mais parecida com `elements.data` (arrays de tamanho
   fixo, mesmo padrão do 1.2.6).
