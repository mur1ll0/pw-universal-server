---
name: pw-dados-do-realm
description: Ler, decodificar ou ligar ao mundo um arquivo de dados do Perfect World (elements.data, tasks.data, npcgen.data, aipolicy.data, gshop, ptemplate.conf, gs.conf, .hmap, .sev, clsconfig) no pw-universal-server. Usar ao escrever ou corrigir leitor em crates/pw-data-loader, ao consultar valores de um realm, ou quando um número de jogo deveria vir de dado.
---

# Arquivos de dados: o carregador original é o juiz

Formatos, estado de cada leitor e o que já está ligado ao mundo:
`specs/03_DATA_LOADER_SPEC.md` — leia antes.

## 1. Onde está a autoridade

| arquivo | carregador original |
| :--- | :--- |
| `elements.data` | `EvolvedPWClient/ElementClient/CCommon/elementdataman.cpp:3879` (`load_data`); layout de cada tabela no `.cfg` do ADMVAL → `specs/elements_layouts/vNNN.json` |
| `tasks.data` | `ElementClient/Task/TaskTempl.cpp` (`ATaskTempl::LoadBinary`); campos novos com nome em `F:\PW\1.7.2\172Source\cgame\gs\task\TaskTempl.h` |
| `npcgen.data` | `EvolvedPWServer/cgame/gs/template/npcgendata.h/.cpp`; posição em `gs/npcgenerator.cpp` |
| `aipolicy.data` | `cgame/gs/ai/policy.cpp` do **1.7.2** |
| `ptemplate.conf` | `gs/playertemplate.cpp` (e o que o `elements.data` sobrescreve, `:293-301`) |
| `.hmap` / `gs.conf` | `cgame/gs/terrain.cpp` |
| `clsconfig` | `cnet/gamedbd/clsconfig.h`, `gamedbmanager.cpp` |

## 2. Regras

1. **Fechar no último byte.** O leitor recusa o arquivo que não termina exatamente onde deve
   (ou missão que não termina no deslocamento da seguinte). Nada de buscar posição por
   plausibilidade, nada de override por arquivo.
2. **O fonte é mais velho que os dados.** Quando não fecha, meça nos dados: histograma de bytes
   não nulos por deslocamento, campos que nascem `true`, ponteiros de heap gravados pelo editor.
   Nomeie pelos fontes 1.7.2.
3. **Conferir conteúdo, não só contagem.** Texto legível, ids que existem em outra tabela,
   valores que batem com o jogo — `count` pequeno não prova alinhamento.
4. **Versão sem layout medido: só cabeçalho e aviso.**
5. `.conf` em **GBK**.
6. Dado de jogo não vira tabela no código; se precisar de constante, teste que a confere contra
   o arquivo do realm.

## 3. Consultar um realm

Arquivos em `data/realm_155/config` (cliente BR, v156 — o único realm 1.5.5) e em
`data/realm_126/config` (1.2.6: `elements.data` v7, `tasks.data` v55). Ferramentas:
`python specs/elements_layouts/pw_elements_reader.py <elements.data>` (mesmo algoritmo do Rust),
`specs/clsconfig_155/ler_clsconfig.py`, `crates/pw-data-loader/examples/` (`dump_monstros`,
`dump_aipolicy`, `cruza_monstro_aipolicy`):

```bash
cargo run -p pw-data-loader --example dump_monstros
```

## 4. Testar

Teste contra o **arquivo real do realm** em `crates/pw-data-loader/tests/` (padrões:
`generic_elements_tests.rs`, `tasks_do_realm.rs`, `terreno_do_realm.rs`,
`ficha_da_armadura.rs`), incluindo um caso que recusa arquivo com um byte a mais.

## 5. Fechar

Atualize `specs/03_DATA_LOADER_SPEC.md` — estado do leitor, e a tabela "consumidores no mundo"
quando uma tabela passar a ser usada — pela skill `pw-atualizar-specs`.
