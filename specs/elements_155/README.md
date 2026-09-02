# Tamanhos das tabelas do `elements.data` (1.5.5 / EvolvedPW)

`table_sizes_v156.tsv` é o `sizeof()` **compilado de verdade** — não contado à mão — de
cada uma das 214 tabelas que `cgame/gs/template/elementdataman.h` (fontes do EvolvedPW,
`F:\PW\1.5.5\EvolvedPWServer`) carrega, na ordem exata em que `elementdataman.cpp::load_data`
as lê do arquivo. Cada linha: `índice<TAB>struct<TAB>nome_do_membro_array<TAB>sizeof`.

## Por que isto existe

O parser Rust (`crates/pw-data-loader/src/elements.rs`, `TABLE_SIZES_V7`) tinha o tamanho de
`weapon_essence` errado (1404 em vez de 1424) e um deslocamento de índice de 1 posição a
partir da tabela 58 (o slot do `talk_proc_array`, que existe na declaração mas nunca é
carregado pelo `array<T>::load()` genérico — o carregador original o lê à parte, com um laço
manual de tamanho variável). Os dois bugs juntos travavam o carregamento de `elements.data`
bem no começo. Ver o comentário acima de `TABLE_SIZES_V7` em `elements.rs` para o relato
completo, e `docs/ESTADO_E_RETOMADA.md` / a memória de sessão do Claude
(`pw_ctx_a_155_funcional`) para a investigação inteira.

## Como foi gerado

```bash
# dump_sizes.cpp inclui exptypes.h (que já tem #pragma pack(push, EXP_TYPES_INC, 4)) e
# imprime sizeof(TIPO) pra cada uma das 214 tabelas, na ordem de elementdataman.h.
docker run --rm \
  -v "F:/PW/1.5.5/EvolvedPWServer/cgame/gs/template/exptypes.h:/work/exptypes.h:ro" \
  -v "$(pwd)/specs/elements_155/dump_sizes.cpp:/work/dump_sizes.cpp:ro" \
  -v "$(pwd)/specs/elements_155:/out" \
  gcc:13 bash -c "cd /work && g++ dump_sizes.cpp -o /tmp/dump && /tmp/dump > /out/table_sizes_v156.tsv"
```

Não precisa de `-m32`: nenhuma dessas structs usa `size_t` ou ponteiro, então o `sizeof` é
igual em 32 e 64 bits (confirmado comparando as 3 primeiras tabelas, já conhecidas e
corretas, contra o resultado do 64 bits — bateram exatas). Se algum `dump_sizes.cpp` novo
incluir uma struct com ponteiro/`size_t`, aí sim usar `-m32` (precisa instalar
`g++-multilib` **e** as libs de runtime 32 bits no container — o `gcc:13` do Docker Hub não
as tem por padrão, só o compilador).

## O que está confirmado e o que não está

**Confirmado, caminhado byte a byte contra `data/realm_155/config/elements.data` real**
(2026-09-02): tabelas de índice 0 a ~98 (contando já com o `talk_proc` pulado). O padrão de
diferença contra o `TABLE_SIZES_V7` antigo nessa faixa é sempre "campo novo no fim" — o
mesmo padrão já visto no protocolo de rede — e não ruído de índice.

**Não confirmado**: dali em diante (índice ~99 em diante, tabela 118 até a 213 — 96 tabelas
que o parser Rust ainda nem tenta ler) o padrão de diferença contra o array antigo volta a
ficar inconsistente (deltas positivos e negativos alternando), sinal de que pode haver mais
um deslocamento de índice escondido, ou de que estes fontes do EvolvedPW são de uma build
diferente da que gerou este `elements.data` específico — a pasta se chama
`pwserver_155v156`, que pode não corresponder exatamente a esta árvore de fontes. Aplicar
esses valores sem caminhar contra o arquivo real primeiro seria a mesma aposta que já
causou o bug original.

## Próximo passo, se for continuar

1. Escrever um caminhador (Rust ou Python) que confere, tabela por tabela, se o `count`
   lido é plausível e se a posição bate com a tabela seguinte — a técnica usada nesta
   sessão, mas como ferramenta permanente (`tools/`?) em vez de script solto.
2. Estender `TABLE_SIZES_V7` (ou substituí-lo por algo gerado) para as 214 tabelas, não só
   118 — o parser Rust hoje simplesmente para de ler depois da tabela 117 e nunca visita as
   96 seguintes.
3. Aplicar o mesmo método ao `npcgen.data`, que falha do mesmo jeito (`failed to fill whole
   buffer`) e provavelmente tem a mesma classe de bug.
