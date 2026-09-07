# Arquitetura "live": pw-gs como dono da decodificação, pw-admin como cliente

Status: **decisão de arquitetura fechada com o Murillo em 2026-09-02, implementação ainda não
iniciada.** Este documento registra o que foi decidido e por quê, antes de qualquer código —
seguindo o mesmo padrão do resto do projeto (ver `murillo_pw_project_style` na memória):
decisão de arquitetura fecha com o Murillo antes de implementar, não depois.

## Motivação

A arquitetura atual (`specs/elements_layouts/README.md`) resolveu decodificar `elements.data`
com um catálogo de layouts (`vNNN.json`, um por build) lido **de forma independente** por dois
lugares: `crates/pw-data-loader/src/generic_elements.rs` (Rust, embutido no binário via
`include_str!`) e `web-admin/backend/elements_decoder.py` (Python, lendo o arquivo do realm
direto por um volume montado). Isso decodifica corretamente, mas duplica o "motor" em duas
linguagens — o mesmo problema que gerou os bugs de `TABLE_SIZES_V7` divergente entre Rust e
Python antes desta sessão.

O Murillo propôs uma reformulação: o `pw-gs` (processo do mundo, um por realm) vira o **único**
dono da decodificação — decodifica o `elements.data` do seu próprio realm e expõe os dados já
decodificados por uma consulta em tempo real. O `pw-admin` deixa de ter decodificador próprio;
vira só um cliente HTTP desse serviço. Isso elimina de vez o risco de as duas implementações
divergirem, porque só existe uma.

## O que já existe de IPC entre processos (mapeado nesta sessão, não suposto)

Investigação de código real (`crates/pw-bus/`, `crates/pw-link/`, `crates/pw-gs/`,
`web-admin/backend/main.py`, `docker/docker-compose.yml`):

- **`pw-bus`** (porta 29100, um por realm, `pw-link` ↔ `pw-gs`): TCP persistente,
  fire-and-forget, só 4 mensagens de gameplay (`ClientToGame`/`GameToClient`/`EnterWorld`/
  `PlayerLogout`), payload em framing GNET binário opaco (subcomando do mundo 3D), sem
  conceito de request/response correlacionado, sem autenticação — de propósito, porque nunca
  sai da rede interna do compose (`crates/pw-bus/src/message.rs:47-76`,
  `crates/pw-bus/src/transport.rs`). **Nenhum outro processo usa este barramento hoje** — nem
  `pw-admin-api`, nem `pw-auth`, nem `pw-delivery`.
- **`pw-admin-api` hoje não fala com `pw-gs` de jeito nenhum.** Só faz um probe TCP cru contra
  a porta pública do `pw-link` (`29000`/`29001`/`29002`, não o `pw-bus`) pra saber "tá online?"
  (`web-admin/backend/main.py:1013-1023`), e publica em canais Dragonfly Pub/Sub que **ninguém
  assina** hoje (nem Rust, nem Python) — funcionalmente um beco sem saída, não um canal
  funcionando ponta a ponta.
- **Rede docker**: `pw-admin-api` já está na mesma rede default do compose que todos os
  `pw-world-*`, e tecnicamente já alcançaria a porta 29100 deles hoje (não publicada no host,
  mas sem isolamento de rede entre serviços) — só que nenhum código usa isso.
- **Conclusão**: não existe barramento genérico reaproveitável. `pw-bus` é gameplay-only e
  opaco demais pra carregar consultas administrativas sem virar uma gambiarra em cima de um
  protocolo documentadamente dedicado a outra coisa. Um canal novo é justificado — não é
  reinventar por preguiça de procurar.

## Decisões fechadas com o Murillo (2026-09-02)

### 1. Semântica de realm offline

**O painel perde acesso (leitura e edição) aos dados de referência de um realm cujo `pw-gs`
está desligado — aceitável.** Exceção explícita: **criação de conta** é cross-realm e já vive
só no Postgres hoje (não depende de nenhum `pw-gs` rodando) — isso continua funcionando do
jeito que já funciona, sem mudança. A resposta do Murillo, literal: "apenas a criação de
contas que é compartilhada entre todos os realms deve funcionar mesmo com os realms
desligados".

**Por que isso é aceitável neste projeto**: o pw-admin é um painel de operação de um servidor
que já se espera estar rodando, não uma ferramenta de configuração fria — diferente de
criação de conta (que faz sentido funcionar a qualquer momento).

### 2. Semântica de hot-reload

**Editar um item/skill/template já em uso afeta só instâncias novas** (novo drop, novo spawn,
próximo load) — entidades já carregadas em memória (jogador com o item equipado, mob já
spawnado) mantêm a versão antiga até recarregarem naturalmente. Resposta do Murillo: "Só afeta
novas instâncias".

Isso simplifica bastante o lado do `pw-gs`: **não precisa varrer estado vivo em memória** e
aplicar a mudança a entidades já carregadas — só precisa trocar o que é servido para
carregamentos futuros. Nenhuma entidade em combate/estado ativo é tocada por uma edição.

### 3. Onde o `.cfg` de layout vive — ADIADO, não mexer agora

A proposta original do Murillo incluía mover o `.cfg` de cada build pra dentro da pasta de
cada realm (`data/realm_<x>/config/`) e o `pw-gs` parsear esse `.cfg` **em tempo de
execução** — eliminando a necessidade de gerar e commitar um `vNNN.json` com antecedência.

**Decisão: NÃO mexer nisso agora.** Mantém o que já está implementado e testado
(`specs/elements_layouts/README.md`): catálogo `vNNN.json` gerado com antecedência a partir do
`.cfg` por `generate_v156.py`, embutido no binário Rust via `include_str!` em tempo de
compilação, versão do arquivo detectada sozinha pelo cabeçalho do `elements.data`
(`pw_elements_reader.py::detect_header()`). Adicionar uma build nova continua exigindo gerar
um `vNNN.json` novo e reconstruir o binário — trade-off aceito por ora, porque o mecanismo já
funciona ponta a ponta (231/231 tabelas, testado nos dois lados).

**Registrado para quando o pw-admin for retrabalhado de verdade** (resposta literal do
Murillo: "Anote para no futuro quando formos mexer no pw-admin a gente fazer o parse do cfg em
runtime dentro do pw-gs, reescrito em rust, e aí sim a gente implementa tudo isso"):

- Portar `specs/elements_155/parse_seledit_cfg.py` (o parser do `.cfg` da ferramenta de
  edição da comunidade — ADMVAL/Perfect World Data Editor/sELedit, mesmo formato de `.cfg`
  entre eles, ver `specs/elements_155/README.md`) para Rust, como parser de runtime dentro do
  `pw-gs` — não mais um script Python que roda uma vez pra gerar um JSON antecipadamente.
- O `.cfg` bruto passa a viver em `data/realm_<x>/config/`, ao lado do `elements.data` daquele
  realm — a decisão de "qual `.cfg` vai em qual realm" vira responsabilidade do processo de
  build/deploy (copiar o `.cfg` certo pra pasta certa depois de compilar), não algo que o
  `pw-gs` adivinha em tempo de execução.
- Isso elimina a necessidade de recompilar o binário Rust toda vez que uma build/versão nova
  precisar ser suportada — só copiar o `.cfg` novo pro lugar certo no deploy.
- Quando isso for implementado, o achado de calibração já documentado continua valendo
  (`wstring:N` no `.cfg` é **N bytes**, não N caracteres — `specs/elements_155/README.md`,
  seção "Achado de calibração") e as 10 correções de `skip`/`count` já classificadas
  (`specs/elements_155/realm_155_overrides.json`) continuam sendo pistas de "tabelas de
  risco", não valores fixos copiáveis — ver `pw_ctx_a_155_funcional.md` na memória, seção
  "RESOLVIDO: as 10 correções são do formato ou só deste arquivo?".

### 4. Protocolo do canal admin ↔ pw-gs

**HTTP+JSON, um novo listener dedicado dentro do `pw-gs`** — não reaproveita o framing GNET
binário do `pw-bus` (que é opaco e feito pro protocolo do jogo, não pra CRUD administrativo).
Justificativa do Murillo ao escolher: combina com o que `pw-admin-api` (FastAPI) já fala
naturalmente.

- Novo listener, **só na rede interna do docker-compose** — nunca publicado no host, mesmo
  padrão de isolamento que o `pw-bus` (29100) já usa hoje (`pw-world-*` não tem `ports:` no
  compose).
- Porta sugerida: **29101** (livre hoje — 29100 é o `pw-bus`, 29000/29001/29002 são os
  gateways públicos de cliente por realm) — a confirmar quando for implementar.
- `pw-gs` ainda não tem nenhuma dependência de framework HTTP (`crates/pw-gs/Cargo.toml`, só
  `tokio`+`serde_json` hoje, sem `axum`/`warp`/etc.) — precisa adicionar uma. Já tem
  `tokio`+`serde`+`serde_json`, então a integração é natural (`axum` é o candidato óbvio, mas
  não decidido/fixado ainda).

### 5. Persistência de edição

**Camada de overrides separada — nunca reescreve o `elements.data` original.** Mesmo padrão
já usado pros ajustes de `skip`/`count` (`specs/elements_155/realm_155_overrides.json`): uma
edição feita pelo pw-admin vira uma entrada num arquivo de patch próprio do realm, que o
`pw-gs` aplica **por cima** do `elements.data` original ao carregar — nunca uma escrita direta
no arquivo binário de origem.

**Por quê**: `elements.data` já provou ter quirks frágeis de alinhamento (`skip`/`count`
errados, ver `specs/elements_155/README.md`) — uma escrita direta no formato binário original
por um bug de serialização arrisca corromper a fonte de dados do realm inteiro. Uma camada de
overrides em JSON (fácil de auditar, revisar, reverter uma edição específica) é ortogonal ao
risco do parser binário e já é um padrão que este projeto usa.

Formato exato do arquivo de overrides de edição (provavelmente `data/realm_<x>/config/
elements_admin_overrides.json` ou similar, por analogia a `realm_155_overrides.json`, mas
guardando **registros editados** em vez de correções de `skip`/`count`) — **ainda não
desenhado**, é trabalho de implementação, não decisão de arquitetura em aberto.

## O que muda na arquitetura hoje implementada

| | Hoje | Proposto |
| :--- | :--- | :--- |
| Quem decodifica `elements.data` | Rust (`pw-data-loader::generic_elements`) e Python (`web-admin/backend/elements_decoder.py`), duas implementações independentes do mesmo algoritmo | Só o `pw-gs` (Rust) — único dono |
| Como `pw-admin-api` lê os dados | Lê o arquivo direto, via volume montado (`../data:/app/data:ro`) + `pw_elements_reader.py` | Consulta HTTP+JSON no `pw-gs` do realm |
| Como uma edição chega ao jogo | N/A (não existe edição hoje, só leitura) | HTTP+JSON pro `pw-gs`, que grava numa camada de overrides e aplica no próximo load |
| Catálogo de layout (`vNNN.json`) | Gerado com antecedência do `.cfg`, embutido em compile-time | **Sem mudança por ora** — só muda quando o item 3 acima for implementado |

## O que ainda está em aberto (trabalho de implementação, não decisão de arquitetura)

- Escopo da v1: só `elements.data`, ou também `tasks.data`/`npcgen.data`/`gshop*.data`?
  `tasks.data` tem registros de tamanho variável (`TaskTempl.cpp`) e precisa de parser próprio
  — não é reaproveitável direto do trabalho de `elements.data` (ver
  `pw_ctx_a_155_funcional.md`, seção "Próximos passos concretos", item 2). Recomendação
  natural: começar só por `elements.data`, que já está 100% decodificado e testado.
- Formato exato do payload HTTP (schema de consulta por tabela/registro, schema de edição).
- Formato exato do arquivo de overrides de edição.
- Autenticação/autorização do novo listener do `pw-gs` — mesmo estando só na rede interna do
  docker, vale ter algum controle mínimo (nem que seja um token compartilhado simples) já que
  é um novo tipo de processo (backend Python) alcançando o `pw-gs` diretamente, diferente do
  `pw-bus` que só liga dois processos Rust do mesmo par realm.
- Nome definitivo da porta/env var (por analogia a `BUS_LISTEN`/`GS_BUS`, algo como
  `ADMIN_LISTEN`/`GS_ADMIN`).

## Próximo passo

Nenhum código escrito ainda para este desenho. Ao retomar: confirmar escopo da v1 com o
Murillo (recomendação: só `elements.data`) e decidir o schema do payload HTTP antes de
implementar o listener no `pw-gs`.
