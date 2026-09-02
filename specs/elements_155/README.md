# Tamanhos das tabelas do `elements.data` (1.5.5, build v156)

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

## Próximo passo, se for continuar

1. Achar a causa do "+4 bytes" entre as tabelas 19 e 20 — provavelmente aponta pra um
   campo faltando em alguma das tabelas 0–19, ou um separador entre tabelas.
2. Escrever um caminhador (Rust ou Python) permanente que confere tabela por tabela contra
   o arquivo real, e usar o `v156.cfg` como fonte primária (não o `exptypes.h` compilado —
   ficou comprovado que é de build diferente).
3. Estender `TABLE_SIZES_V7` (ou substituí-lo por algo gerado do `v156.cfg`) para as 231
   tabelas, não só 118.
4. Aplicar o mesmo método ao `npcgen.data`, que falha do mesmo jeito (`failed to fill whole
   buffer`).
