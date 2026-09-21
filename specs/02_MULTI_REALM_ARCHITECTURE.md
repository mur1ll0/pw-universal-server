# Especificação 02: Realms, daemons, login e barramento

> Verificada contra o código em 2026-09-14, commit `e6433ae`. Cobre `docker/`,
> `crates/pw-link/`, `crates/pw-bus/`, `crates/pw-auth/` e
> `crates/pw-protocol/src/{version,edition,codec,opcodes}.rs`.

## 1. Decisão: servidor polimórfico, cliente intocado

A lista de avisos de status na entrada é fornecida por
`WorldProtocol::initial_status_notifications`: padrão preserva a sequência 155;
v126 omite os seis ids acima do limite 260 do binário (VA 0x584618, B74).
`GetUIConfig_Re` preserva o blob recebido, inclusive versão 3 do cliente 126
(amostra integral reproduzida em teste; `docs/ENTRADA_126.md`).

Um binário `pw-link`/`pw-gs` serve todas as versões; a versão de cada realm vem da variável
`GAME_VERSION` e decide layouts por `pw_protocol::PorVersao`. **Não se recompila nem se
modifica o `elementclient.exe`**: o jogador usa o cliente da versão como ele é (a única
exigência fora do servidor é o que o próprio cliente precisa para rodar, ver
`docs/ESTADO_E_RETOMADA.md` §1.3).

Variáveis de cada daemon: `REALM_ID`, `GAME_VERSION`, `DATABASE_URL`, `REDIS_URL`; no link
`GATEWAY_PORT` e `GS_BUS`; no mundo `WORLD_TAGS` (lista de mapas; `WORLD_TAG` para um só),
`CONFIG_DIR` e `BUS_LISTEN`. No `pw-gs`
uma `GAME_VERSION` inválida é erro ao subir — não cai em 1.2.6 em silêncio (A44). Link sem
`GS_BUS` sobe e avisa no log: o cliente entra, mas nada é simulado.

## 2. Topologia (`docker/docker-compose.yml`)

### 2.1 Serviços globais

| serviço | porta no host | papel |
| :--- | ---: | :--- |
| `pw-postgres` | 5432 | banco único de todos os realms |
| `pw-dragonfly` | 6379 | cache |
| `pw-auth` | — (29200 interna) | serviço de autenticação; hoje sem consumidor |
| `pw-admin-api` | 8000 | painel (`web-admin/backend`), lê `data/` e `specs/elements_*` |

### 2.2 Por realm: um `pw-link` e um `pw-gs` com os mapas do realm

| realm (`REALM_ID`) | `GAME_VERSION` | link (porta pública) | servidor de mundo (mapas) | dados |
| :--- | :--- | :--- | :--- | :--- |
| `realm_155` | 1.5.5 | `pw-realm-155` **29004** | `pw-world-155` (mapas **1 e 161**) | `data/realm_155/config` (cliente BR, v156) |
| `realm_126` | 1.2.6 | `pw-realm-126` 29000 | 1 → `pw-world-126` | `data/realm_126` |
| `realm_153` | 1.5.3 | `pw-realm-153` 29001 | 1 → `pw-world-153` | abandonado |
| `realm_148` | 1.4.8 | `pw-realm-148` 29002 | 1 → `pw-world-148` | nunca foi alvo |

- **Um servidor de mundo por realm, com vários mapas** (`WORLD_TAGS=1,161`). Os dados são
  carregados uma vez e cada mapa é um `WorldInstance` com seu próprio tick — o equivalente
  do `./gs gs01 gs.conf gmserver.conf gsalias.conf is61` do original, que carrega uma vez e
  faz `fork()` de um processo por mapa (`cgame/gs/start.cpp:185-234`). No `EnterWorld` o
  roteador (`pw_gs::RoteadorDeMapas`) lê o `world_id` do personagem e passa a entregar a
  esse mapa tudo daquele `roleid`; mapa gravado que o processo não serve cai no primeiro da
  lista, com aviso.
- Separar um mapa pesado noutro contêiner continua possível pelo link:
  `GS_BUS=<tag>=<host>:29100,...` (`LinkGateway::uplink_da_sessao`); mundo sem entrada cai no
  primeiro. `GS_BUS` sem `<tag>=` vale para todos os mapas.
- Medido antes da consolidação (2026-09-14): cada contêiner de mapa ocupava ~1,1 GB, quase
  todo em dados repetidos; o `gs.conf` do 1.5.5 lista ~80 mapas.
- **Trocar de mapa durante a sessão** (B51): entre mapas do **mesmo processo**, pelo roteador
  (`RoteadorDeMapas::ligar_trocas`): o mapa de origem solta sessão e entidade, o destino manda
  `NOTIFY_HOSTPOS` com o `tag` novo e grava mapa e posição antes de mostrar o jogador (spec 05
  §7). Mapa servido por **outro** contêiner `falta` (o link teria de reencaminhar a sessão).
- Um segundo realm da mesma versão: receita em `docs/MULTIPLOS_REALMS.md`; cada realm
  precisa de uma linha em `realms` e dos moldes em `class_templates`.

### 2.3 Regras cobradas por teste (`pw-bus/tests/topologia_do_compose.rs`)

- A porta do barramento **nunca** é publicada: ele não autentica, e quem o alcança manda
  `EnterWorld` por qualquer `roleid` (A26).
- Todo alvo de `GS_BUS` existe, roda `pw-gs`, escuta na porta, é do mesmo realm e versão, e
  `161=` aponta para quem serve o mapa 161 (`WORLD_TAG` ou `WORLD_TAGS`).

## 3. Versão e `Challenge`

### 3.1 `GAME_VERSION` no fio (`version.rs`)

| versão | `server_version_code` | origem |
| :--- | :--- | :--- |
| 1.2.6 | `0x00010206` | captura da VM, 2026-09-01 |
| 1.4.8 | `0x00010408` | **não conferido** |
| 1.5.3 | `0x00010502` | `EC_Game.cpp:115` (não é `...503`) |
| 1.5.5 | `0x00010505` | lido do `elementclient.exe` build 2575, ao lado do DWORD do build (B5) |

O cliente compara **igualdade exata**: servidor mais novo dá "versão baixa"; mais velho dá
"manutenção em andamento" (a tradução de `FIXMSG_SERVERUPDATE`).

### 3.2 Opcodes que trocam entre versões

| protocolo | 1.2.6 | 1.4.8 / 1.5.x |
| :--- | ---: | ---: |
| `Response` | 2 | 3 |
| `KeyExchange` | 3 | 2 |

`GamedataSend` é o opcode **34** nos dois sentidos entre cliente e link.

### 3.3 `edition` (só 1.4.8+; `edition.rs`)

String hexadecimal **minúscula, sem separador e sem preenchimento**, comparada com `stricmp`
pelo cliente contra o que ele calcula dos próprios arquivos (`EC_Game.cpp:646`):

```
"%x%x%x%x"   = ELEMENTDATA_VERSION, task_templ_version, gshop_ts, gshop_ts2
"%x%x%x%x%x" = ... + gshop_ts3     (1.5.5, ramo VIP — inferência, não medição)
```

- `ELEMENTDATA_VERSION` e `task_templ` são as **constantes do cliente**, lidas do
  `elements.data`/`tasks.data` do realm (o do servidor é diferente: `0x30000080` contra
  `0x3000007f` no 1.5.3). 155: `0x3000009c` / 129.
- `gshop_ts` é o primeiro `u32` de `gshop.data`, `gshop_ts2` o de `gshop1.data` —
  **arquivos diferentes** (A23).
- Qualquer arquivo de dados do realm diferente do cliente = "versão baixa". Por isso o
  realm 1.5.5 usa os `.data` **do cliente BR** (v156). Até 2026-09-17 havia também um realm
  com os do cliente EN (v159); saiu para ficar um realm por versão (B55).

### 3.4 `nonce`

16 bytes: `[Attr u32][newbie_time u32][aleatório 8]`. `Attr` carrega carga e os bits de
dobro de experiência/moedas/drop/SP, zona livre, PvP — é por onde os rates do realm chegam
ao cliente (A4). **Hoje os 8 primeiros bytes vão zerados** — `falta`.

## 4. Fluxo de login e entrada no mundo (no `pw-link`)

`Challenge` → `Response` (autenticação direto pelo `AccountRepository`; `pw-auth` não
participa) → `KeyExchange` (a cifra é opcional na prática, A58) → `OnlineAnnounce` →
`RoleList`/`RoleList_Re` → `CreateRole`/`DeleteRole`/`UndoDeleteRole` (**checam o dono**,
A29) → `SelectRole` → `EnterWorld` (72).

Na entrada o link manda a carga inicial (ordem importa, B36e/B38): `INST_DATA_CHECKOUT`
(com `id_inst` = mundo do personagem e os carimbos de `region.sev`/`precinct.sev` desse
mapa), `SELF_INFO_00`, `OWN_EXT_PROP`, `SELF_INFO_1`, habilidades, `TASK_DATA` (5 blocos no
1.5.x, com as listas de missão de `character_task_lists` — spec 05 §10), bolsa, equipamento,
bolsa de missão (pacote 2), com o bloco de dados de cada peça, dinheiro, reputação, modo PvP,
`SERVER_TIME` com `lua_version = 102` (primeira linha do `global_api.lua`). Layouts: spec 04.

**`GetUIConfig_Re` (B52):** uma vez por personagem, com o bloco gravado em
`character_client_config`, e **só em resposta ao `GetUIConfig` que o cliente manda depois do
`TASK_DATA` do mundo** (`BusUplink::dados_iniciais_entregues`, marcado quando o link repassa o
S2C 105 vindo do `pw-gs`). O cliente pede a configuração a cada `TASK_DATA` e usa esse comando
como fim da carga inicial (`EC_HostMsg.cpp:3947-3949`). **Com uplink de mundo o link não manda
`TASK_DATA` próprio (B53)**: mandava, o primeiro pedido chegava antes das habilidades e da bolsa
do mundo, era o respondido, e o segundo era recusado — as barras eram montadas vazias e
gravadas assim ao sair (teste de 2026-09-16: 211 bytes mandados, 172 gravados). Sem uplink, o
link manda o `TASK_DATA` e responde ao primeiro pedido.

**O bloco vai byte a byte como o cliente o gravou (B54)**: `S2CGetUIConfigRe::new` trocava os
16 primeiros bytes por um cabeçalho inventado, o cliente lia versão 1, não descomprimia e
registrava `LoadConfigsFromServer, data read error (2)` em todo login (`element/logs/EC.log`)
— as barras e o layout **nunca** carregaram, e o cliente gravava a configuração padrão ao sair.
**Sem configuração gravada**, o link manda a do molde da classe
(`class_templates.ui_config`, o `config_data` do `GRoleBase` dos moldes do `clsconfig`:
barra com as habilidades iniciais e o rastreador de missões ligado), como o `gamedbd` faz ao
criar o personagem.

Personagem novo: posição, kit e equipamento vêm de `class_templates` do realm (espelho do
`gamedbd/clsconfig` original, B43/B47); atributos **5/5/5/5** para toda classe (os moldes do
`clsconfig`, B51); raça pela classe; vida e mana cheias pela conta de
`BaseDaClasse::vida_e_mana_maximas`.

Ainda no link: fala (canal global por processo, sem raio), lista de amigos (sempre vazia),
UI config e help states (gravados por personagem e só para o personagem da sessão, até
64 KiB; antes eram descartados e as barras de atalho voltavam vazias), custom data, e alguns
subcomandos (spec 04 §4).

## 5. Barramento `pw-link` ↔ `pw-gs` (`pw-bus`)

Protocolos GNET **reais** do IR, não formato inventado. Quadro:
`[CompactUINT(opcode)][CompactUINT(tamanho)][corpo]`, limite de 1 MiB.

| mensagem | opcode | sentido | campos |
| :--- | ---: | :--- | :--- |
| `PlayerLogout` | 69 | mundo → link | `result`, `roleid`, `provider_link_id`, `localsid` |
| `EnterWorld` | 72 | link → mundo | `roleid`, `provider_link_id`, `locktime`, `timeout`, `settime`, `localsid` |
| `S2CGamedataSend` | 74 | mundo → link | `roleid`, `localsid`, `data` |
| `C2SGamedataSend` | 75 | link → mundo | `roleid`, `localsid`, `data` |

- O link repassa **todo** `GamedataSend` do cliente ao mundo, sem interpretar, e ainda trata
  alguns no próprio `gateway.rs` (migração incompleta).
- O mundo carrega o personagem do banco ao receber `EnterWorld` (`colocar_no_mundo`) e
  responde pelo par `(roleid, localsid)`; o link guarda o `localsid` para o logout e para
  quedas (A27).
- Uplink: uma conexão por mundo, fila única de saída, reconexão com espera crescente até
  30 s; mundo caído não derruba a sessão.

## 6. Isolamento

Personagem pertence a `(account_id, realm_id)`. Conta é global; personagens, moldes e dados
são por realm.
