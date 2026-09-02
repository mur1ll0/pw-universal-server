# Catálogo de layouts do `elements.data`, por versão

Esta pasta é o **artefato reaproveitável** — o "extrator" de verdade, no sentido de que é o
que tanto o `pw-data-loader` (Rust) quanto o backend do web-admin (Python) devem consumir
pra saber como ler um `elements.data`, sem reimplementar a mesma tabela de tamanhos duas
vezes (que foi exatamente o problema que gerou os bugs que este projeto já teve — ver
`specs/elements_155/README.md` para a arqueologia completa de como cada tabela foi
confirmada).

## O que tem aqui

- **`vNNN.json`** (um por build, `NNN` = número de build do cabeçalho do arquivo, ex.
  `v156.json`) — layout **de formato**: nome, campos, tipos e tamanho de cada tabela.
  Gerado a partir do `.cfg` de uma ferramenta de edição da comunidade (ADMVAL, Perfect
  World Data Editor, sELedit — todas usam o mesmo formato de `.cfg`) por
  `generate_vNNN.py`. Isto é **universal pra qualquer arquivo daquela build** — não
  depende de qual realm/servidor gerou o arquivo.
- **`pw_elements_reader.py`** — implementação de referência em Python: detecta a versão do
  cabeçalho, carrega o layout certo do catálogo e devolve `{nome_da_tabela: [registro,...]}`
  pra qualquer `elements.data`. É o que `web-admin/backend/elements_decoder.py` já usa (ver
  `_load_realm_elements_generic`). Testado ponta a ponta contra
  `data/realm_155/config/elements.data` (231/231 tabelas, 69.626 registros).
- **`crates/pw-data-loader/src/generic_elements.rs`** (não mora nesta pasta, mas lê o
  mesmo `v156.json`, embutido no binário via `include_str!` em tempo de compilação — não
  depende de `specs/` existir em produção, diferente do lado Python) — a mesma
  implementação em Rust, aditiva (não substitui `crate::elements::ElementsData`, que o
  `pw-gs` usa hoje). Testes em
  `crates/pw-data-loader/tests/generic_elements_tests.rs` confirmam o mesmo resultado do
  lado Python byte a byte (231 tabelas, 69.626 registros). **Achado ao portar**: uma
  verificação de "texto legível" que em Python usa `str.isprintable()` (cobre cirílico,
  CJK, etc.) tinha virado `char::is_ascii_graphic()` no Rust — só aceita ASCII — e isso
  sozinho derrubava o score de toda tabela com nome em cirílico abaixo da barra de aceite
  (`EQUIPMENT_ADDON` já quebrava na primeira tabela). Corrigido pra `!c.is_control()`.

## O que NÃO tem aqui (de propósito)

As correções de `skip`/`count` que apareceram durante a decodificação de
`data/realm_155/config/elements.data` (10 tabelas, ver
`specs/elements_155/walk_tables.py::TABLE_OVERRIDES`) **não estão neste layout** — foram
extraídas para `specs/elements_155/realm_155_overrides.json`, marcado
`"verified_against": "data/realm_155/config/elements.data"`. A razão: **ainda não sabemos
se essas correções são do formato v156 (valeriam pra qualquer arquivo dessa build) ou
peculiaridade deste arquivo específico** (ver "Próximo passo" no README de
`specs/elements_155/`) — não commitar um palpite como se fosse fato estabelecido.

Na prática isso não trava o extrator genérico: o leitor (ver
`crossref_admval.py`/`walk_tables.py` como referência do algoritmo, e o carregador em
Rust/Python) deve tentar ler cada tabela pela posição ingênua primeiro (o mesmo `count==0`
aceito de cara / primeiro registro plausível, guloso) e só consultar um arquivo de
`*_overrides.json` como **dica opcional** quando disponível pro realm específico sendo
carregado — nunca como parte fixa do layout de formato.

## Como a versão é detectada (respondendo a pergunta do Murillo: copiar o `.cfg` pra pasta do realm?)

**Não precisa copiar nada pra pasta do realm — e implementado em
`specs/elements_layouts/pw_elements_reader.py::detect_header()`.** O arquivo já diz sozinho
qual build é: os 4 primeiros bytes são um `uint32` cujo byte baixo é o número de build
(`0x3000009c` → `0x9c` = **156**; foi assim que o ADMVAL soube dizer "v159" pro
`elements.data` do client 1.5.5 original, que tem `0x3000009f`).

**Achado corrigido durante a implementação**: a princípio eu achava que havia duas
"famílias" de codificação de versão (uma antiga pro 1.2.6, outra `0x3000+build` pros
builds novos) — **errado**. Testando contra `data/realm_126/config/elements.data` (v7,
1.2.6) descobri que ele **também** começa com `0x30000007` — a mesma codificação
`0x3000<<16 | build` usada em todas as versões já medidas. A diferença real entre eras é
outra: builds recentes (confirmado no v156) têm **mais 4 bytes** de `build_timestamp`
depois da versão (cabeçalho de 8 bytes); builds antigas (confirmado no v7) não têm — os
bytes 4-7 já são o `count` da primeira tabela (cabeçalho de 4 bytes). Como isso não dá pra
saber só pelo valor da versão, o desempate usa magnitude: um `time_t` de verdade (datas de
2001 em diante) sempre passa de 900 milhões; um `count` de tabela, pelo que já vimos em 231
tabelas medidas, nunca chega nem perto disso.

O extrator faz, então:

1. Ler os 4 bytes de versão do cabeçalho (sempre `0x3000<<16 | build`).
2. Olhar os 4 bytes seguintes: se parecem um timestamp plausível (>900 milhões), cabeçalho
   de 8 bytes; senão, de 4 (e esses bytes já são o `count` da tabela 0).
3. Procurar `specs/elements_layouts/v<build>.json` no catálogo.
4. Se achar, carregar com aquele layout. Se não achar, falhar de forma clara dizendo qual
   versão falta (é o caso do v7/1.2.6 hoje — `pw_elements_reader` já detecta a versão
   certinho, só falta gerar `v7.json` a partir de um `.cfg` da era 1.2.6, ex. `sELedit`) —
   não adivinhar nem tentar o layout mais próximo silenciosamente.

Isso já testado com sucesso contra os dois arquivos reais do projeto: `data/realm_155/...`
(v156, cabeçalho de 8 bytes, 231 tabelas) e `data/realm_126/...` (v7, cabeçalho de 4 bytes,
detectado corretamente e recusado com uma mensagem clara por falta de layout — sem quebrar
nem dar dado errado).

## Gerando um layout novo

```bash
python specs/elements_layouts/generate_v156.py
```

Cada versão nova precisa de um script próprio (`generate_v<N>.py`) porque aponta pra um
`.cfg` de origem diferente — mas o corpo do script é o mesmo, só troca o caminho do `.cfg`
e o número de versão. Considerar generalizar num único script parametrizado se/quando
tivermos 3+ versões convertidas.

## Pendência de empacotamento: `specs/` não está disponível na imagem Docker do web-admin

`web-admin/backend/elements_decoder.py` procura esta pasta em alguns caminhos candidatos
(mesmo padrão de `_get_elements_path_for_realm`) e cai de volta pro decodificador antigo
(só v7) se não achar. **Em dev local (repo clonado) funciona direto.** Em produção, o
`docker-compose.yml` builda o `pw-admin-api` com `context: ../web-admin/backend` — o
Docker não enxerga `specs/` fora desse contexto de jeito nenhum, então a imagem hoje **não
tem acesso ao catálogo de layouts**. Três jeitos de resolver, nenhum escolhido ainda
(decisão do Murillo, é mudança de infra):

1. Alargar o `context` do `pw-admin-api` pra raiz do repo (`context: ..`, igual os outros
   serviços já fazem) e adicionar `COPY specs/elements_layouts specs/elements_155` no
   `Dockerfile` — mas sem um `.dockerignore` isso manda `target/` (Rust, GBs) e `data/`
   junto no contexto de build, deixando o build bem mais lento. Precisa de um
   `.dockerignore` na raiz do repo (ainda não existe) pra ser viável.
2. "Vendorizar" uma cópia de `specs/elements_layouts` + `specs/elements_155` dentro de
   `web-admin/backend/` (versionada ali, sincronizada manualmente quando o layout mudar) —
   simples, mas duplica arquivo.
3. Montar `specs/` como volume no `docker-compose.yml` (igual `data/` provavelmente já é) —
   funciona sem mudar o Dockerfile, mas aí o comportamento em produção depende do volume
   estar montado certo, não só da imagem.

O lado Rust (`pw-data-loader::generic_elements`) **não tem esse problema** — embute o JSON
no binário em tempo de compilação (`include_str!`), então funciona igual em dev e em
produção sem depender de nada externo em tempo de execução.
