# pw-universal-server

Reimplementação em **Rust** (Tokio) do servidor do MMO **Perfect World**, com **PostgreSQL 16**
e **DragonflyDB**, que serve o **cliente original sem modificação** e roda um realm por versão
do jogo sobre a mesma base de contas.

Toda regra, layout e número vem do original — fonte C++ do servidor e do cliente, o binário do
cliente, capturas do servidor 1.2.6 rodando numa VM, os próprios arquivos de dados — e o
código cita de onde saiu.

## Situação (2026-09-26)

| versão | realm | porta | situação |
| :--- | :--- | ---: | :--- |
| **1.5.5** (cliente BR, build 2569) | `realm_155` | 29004 | **jogável no básico** desde 2026-09-23: conta e personagem, mundo, movimento, montaria, combate e habilidades, mascote de combate, progressão, missões, loja e itens, grupo |
| **1.2.6** | `realm_126` | 29000 | **frente atual**: levar ao 1.2.6 o que o 1.5.5 já faz. Entrada, combate, missões, habilidades e mascote testados na suíte; em teste em jogo |
| 1.5.3 | `realm_153` | 29001 | abandonado (o cliente disponível nunca logou) |
| 1.4.8 | `realm_148` | 29002 | nunca foi alvo |

Depois do 1.2.6: banco de dados, painel de administração e atualizador/launcher.

**Onde o trabalho está e o que falta:** [`docs/ESTADO_E_RETOMADA.md`](docs/ESTADO_E_RETOMADA.md).
**Como o sistema é:** [`specs/README.md`](specs/README.md).

## Arquitetura

```
cliente 1.5.5 ──29004──► pw-realm-155 (pw-link) ──barramento 29100──► pw-world-155 (pw-gs, mapas 1 e 161)
cliente 1.2.6 ──29000──► pw-realm-126 (pw-link) ──barramento 29100──► pw-world-126 (pw-gs, mapa 1)
                                   │                                        │
                                   └──────────── pw-postgres (5432) ◄───────┘
                                                 pw-dragonfly (6379), pw-auth, pw-admin-api (8000)
```

- **`pw-link`** (um por realm): login, lista de personagens, entrada no mundo, fala.
- **`pw-gs`** (um por realm, com todos os mapas dele): a simulação — visibilidade, movimento,
  combate, IA, habilidades, missões, itens, mascote. A regra de jogo é uma só; o que muda entre
  versões é o layout (`crates/pw-protocol/src/versions/`) e os dados do realm (`data/<realm>/config`).
- A porta do barramento (29100) nunca é publicada.

Detalhe: [`specs/02_MULTI_REALM_ARCHITECTURE.md`](specs/02_MULTI_REALM_ARCHITECTURE.md).

## Crates

| crate | papel |
| :--- | :--- |
| `pw-core`, `pw-crypto`, `pw-wire` | tipos comuns, RC4/hashes/tickets, formatos de fio GNET e gamedata |
| `pw-protocol` | opcodes, pacotes, subcomandos do mundo, o `WorldProtocol` de cada versão |
| `pw-bus` | barramento `pw-link` ↔ `pw-gs` |
| `pw-storage` | PostgreSQL (contas, personagens, itens, habilidades, missões, moldes) |
| `pw-data-loader` | `elements.data`, `tasks.data`, `npcgen.data`, `aipolicy.data`, `gshop`, `.conf`, `.hmap`, `watermap/`, `movemap/`, `.sev` |
| `pw-link`, `pw-gs` | os dois daemons por realm |
| `pw-auth` | autenticação (sobe no compose; o `pw-link` autentica pelo `pw-storage`) |
| `pw-delivery`, `pw-uniquename` | não usados hoje |

Ferramentas em `tools/`: `pw-rpcgen` e `pw-ir` (IR do protocolo), `pw-pcapdiff` (capturas),
`pw-crash-re` (minidumps do cliente), `pw-pck-extract`, `pw-patch-tool` (planejado).

## Rodar

Pré-requisitos: Docker e Rust estável. Os dados de cada realm ficam em `data/<realm>/config`
(pacote de servidor da versão, com os `.data` do cliente correspondente por cima — ver
`ESTADO_E_RETOMADA.md` §1.2).

```bash
cd docker
docker compose up -d pw-postgres pw-dragonfly pw-auth pw-admin-api
docker compose build pw-world-155 pw-realm-155 && docker compose up -d pw-world-155 pw-realm-155
docker compose build pw-world-126 pw-realm-126 && docker compose up -d pw-world-126 pw-realm-126
```

No cliente, `element/userdata/server/serverlist.txt` aponta para `127.0.0.1` na porta do realm
(no 1.5.5, em UTF-16LE com BOM). O cliente 1.5.5 inicia com
`elementclient.exe game:cpw console=1 logiccheck:0`. Pré-requisitos dos clientes:
`ESTADO_E_RETOMADA.md` §1.3.

O banco é criado por `specs/01_DATABASE_SCHEMA_POSTGRES.sql` (usuário `pw_admin`, banco
`pw_database`, senha em `docker/docker-compose.yml`), com as contas de teste `admin` e
`testuser`. São credenciais de desenvolvimento local — troque antes de expor qualquer porta.

## Testar

A suíte só verifica de verdade com o banco; sem `TEST_DATABASE_URL` os testes de integração
passam sem verificar nada. Os testes rodam isolados no schema `test`.

```bash
TEST_DATABASE_URL="postgres://pw_admin:<senha>@127.0.0.1:5432/pw_database" cargo test --workspace --no-fail-fast -- --test-threads=2
```

Resultado de referência: `ESTADO_E_RETOMADA.md` §2. Os scripts em `tests/*.py` são da fase
inicial do projeto e não são a referência.

## Documentação

| documento | para quê |
| :--- | :--- |
| [`docs/ESTADO_E_RETOMADA.md`](docs/ESTADO_E_RETOMADA.md) | onde o trabalho está, o que falta, como rodar e publicar |
| [`docs/HISTORICO_DE_SESSOES.md`](docs/HISTORICO_DE_SESSOES.md) | diário com a evidência de cada decisão (itens A*n* e B*n*) |
| [`specs/`](specs/README.md) | como o sistema é: arquitetura, banco, formatos de dados, protocolo, simulação, painel |
| [`docs/COMO_TESTAR.md`](docs/COMO_TESTAR.md) | o que verificar e como interpretar |
| `docs/*_126.md` | evidência do 1.2.6: entrada, combate, itens, handshake, medidas, inventário do protocolo |
| [`AGENTS.md`](AGENTS.md), [`CLAUDE.md`](CLAUDE.md) | regras para os agentes (Codex e Claude) que trabalham no repositório |

Os guias `docs/HOW_TO_RUN_SINGLE_OR_MULTI_REALM.md`, `WEB_ADMIN_USER_GUIDE.md`,
`SURFACES_ICONSET_GUIDE.md`, `PW_PATCH_TOOL_GUIDE.md`, `FILE_FORMATS_REFERENCE.md`,
`LOADER_ARCHITECTURE_GUIDE.md` e `FINAL_COMPREHENSIVE_AUDIT.md` são da fase de planejamento
(fim de agosto de 2026) e **não descrevem o sistema atual**; valem as specs.

## Licença

Projeto educacional, de pesquisa e de preservação de software.
