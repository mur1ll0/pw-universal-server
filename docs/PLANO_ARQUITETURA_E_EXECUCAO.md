# Plano de Arquitetura e Execução — PW Universal Server

> Documento vivo. Registra o diagnóstico do estado atual, as descobertas de engenharia
> reversa que mudam decisões de projeto, a arquitetura-alvo e o faseamento do trabalho.

---

## 1. Diagnóstico do estado atual

### 1.1 Números

| Crate | Arquivos | Linhas | Situação |
| :--- | ---: | ---: | :--- |
| `pw-protocol` | 10 | 3.384 | Serialização escrita "de ouvido", sem fonte canônica |
| `pw-storage` | 16 | 2.038 | Repositórios OK, base sólida |
| `pw-link` | 4 | 1.473 | **`gateway.rs` com 1.347 linhas — monólito** |
| `pw-data-loader` | 10 | 1.373 | Parsers parciais de `elements.data`/`tasks.data` |
| `pw-gs` | 9 | 749 | **Sem servidor de rede. Não está no caminho do jogo.** |
| `pw-delivery` | 9 | 643 | Não está no caminho do jogo |
| `pw-core` | 7 | 575 | Tipos base |
| `pw-crypto` | 4 | 230 | RC4/MD5/Argon2 |
| `pw-auth` | 4 | 314 | Autenticação global |
| `pw-uniquename` | 4 | 255 | Unicidade de nomes |

Total: ~11.000 linhas de Rust.

### 1.2 Causa-raiz dos dois sintomas relatados

**"1.2.6 loga e entra, mas nada do jogo funciona."**

O `pw-link/src/gateway.rs` é um monólito que responde a *tudo* inline. O `docker-compose.yml`
sobe **apenas o `pw-link`** para cada realm — `pw-gs` e `pw-delivery` não são iniciados e o
`pw-gs` sequer possui um listener TCP (`server.rs` só roda um tick loop de 50 ms sobre um mundo
em memória que nada alimenta). O resultado é que o gateway *encena* a entrada no mundo:
NPCs com IDs, posições e templates escritos à mão no código-fonte
(`gateway.rs:523-545`), monstro único hardcoded, quest inicial escolhida por `match` de classe.
Não existe simulação, não existe AI, não existe pipeline de dados. O cliente entra porque
recebe a sequência mínima de pacotes que destrava a UI — e depois não há servidor de mundo
do outro lado.

**"1.5.3 nem loga."**

Causa identificada com precisão nos fontes do cliente
(`CElementClient/Network/EC_GameSession.cpp:4003` — `CECGameSession::OnPrtcChallenge`):

```cpp
AString str((const char *)p->edition.begin(), p->edition.size());
if (p->version != g_pGame->GetGameVersion() || stricmp(g_pGame->GetVersionString(), str))
{
    ...  ShowErrorMsg(FIXMSG_WRONGVERSION);  Close();  return;
}
```

O cliente rejeita a conexão se **qualquer** das duas condições falhar:

1. `Challenge.version` diferente de `GAME_VERSION` do binário do cliente.
   Nos fontes: `EC_Game.cpp:115` → `DWORD GAME_VERSION = ((0<<24)|(1<<16)|(5<<8)|2);` = `0x00010502`.
   O código atual envia `0x00010503` — **valor inventado**.
2. `Challenge.edition` diferente da *version string* do cliente, que é
   (`EC_Game.cpp:646`):

   ```cpp
   m_strAllVersion.Format("%x%x%x%x",
       ELEMENTDATA_VERSION, _task_templ_cur_version,
       globaldata_getgshop_timestamp(), globaldata_getgshop_timestamp2());
   ```

   Ou seja: a concatenação hexadecimal da versão do `elements.data`, da versão do
   `tasks.data`, e dos dois timestamps do `gshop.data`. O código atual envia
   `edition = Octets vazio` (`adapter.rs:21`) → `stricmp` falha → login rejeitado.

> **Consequência de projeto:** o servidor precisa carregar os `.data` do realm e derivar
> essa string a partir deles. Isso deixa de ser um detalhe de handshake e vira um
> requisito do `pw-data-loader`: o loader é quem fornece a identidade de versão do realm.

### 1.3 Fonte canônica descoberta (muda a estratégia)

Os fontes 1.5.3 contêm o **código de marshalling já gerado**, não só o XML:

| Artefato | Local | Conteúdo |
| :--- | :--- | :--- |
| `inl/*` | `source_server_153/inl/` | **935 arquivos** com campos + `marshal`/`unmarshal` de cada protocolo, na ordem exata do fio |
| `rpcdata/*` | `source_server_153/rpcdata/` | **617 arquivos** com os structs de dados (RoleInfo, item, etc.) |
| `*/callid.hxx` | por daemon | IDs numéricos de protocolo e RPC |
| `rpcalls.xml` | raiz | 377 protocolos, 604 rpcdata, 87 RPCs — a declaração de origem |
| `EC_GPDataType.h` | cliente, 123 KB | Todos os subcomandos `GamedataSend` (mundo 3D) |
| `marshal_i386.h` | `share/common/` | Regras primitivas de codificação |

Regras primitivas confirmadas em `marshal_i386.h` + `byteorder_i386.h`:

- Todo escalar do protocolo GNET vai para o fio em **big-endian** (`byteorder_32` = `bswap`
  em host little-endian). `float` também (bitcast para `int` e então bswap).
- `Octets` = `CompactUINT(len)` + bytes crus.
- `std::string` = `CompactUINT(bytes)` + bytes.
- Containers STL = `CompactUINT(count)` + elementos.
- Frame = `CompactUINT(type)` + `Octets(payload)` (`protocol.h:Encode`).
- `CompactUINT`: `<0x80` → 1 byte; `<0x4000` → 2 bytes com `|0x8000`; `<0x20000000` →
  4 bytes com `|0xC0000000`; senão `0xE0` + 4 bytes.

Além disso, `Challenge.nonce` **não é aleatório puro**. O cliente faz
(`EC_GameSession.cpp:4062`):

```cpp
memcpy(&m_SevAttr, p->nonce.begin(), sizeof(GNET::Attr));
m_dwNewbieTime = *((unsigned int*)p->nonce.begin()+1);
```

E `GNET::Attr` é (`cnet/gdeliveryd/serverattr.h:8`):

```cpp
union Attr {
    unsigned int _attr;
    struct {
        unsigned char load;      // carga do servidor (barra na lista de servidores)
        unsigned char lambda;
        unsigned char anything;
        unsigned char doubleExp:1, doubleMoney:1, doubleObject:1, doubleSP:1,
                      freeZone:1, bSellpoint:1, bBattle:1, pvp:1;
    };
};
```

> **Consequência de projeto:** os *rates* de EXP/SP/Gold/Drop que o painel admin precisa
> gerenciar **não são invenção nossa** — eles têm representação no protocolo
> (`doubleExp`/`doubleMoney`/`doubleObject`/`doubleSP` + campo `ExpRate`) e o cliente os lê
> já no handshake. O modelo de rates no banco deve espelhar esse formato.

---

## 2. Arquitetura-alvo

### 2.1 Princípio

Voltar à topologia de daemons do servidor original — não por nostalgia, mas porque o
protocolo é desenhado em torno dela: os pacotes têm campos (`localsid`, `userid`,
`providerid`, `zoneid`) que só fazem sentido com essa separação de papéis. Encenar tudo
num processo só é exatamente o que produziu o `gateway.rs` de 1.347 linhas.

```
crates/
  pw-wire/        (novo)  OctetsStream, CompactUINT, big-endian, Octets, vetores.
                          Zero conhecimento de protocolo. 100% coberto por testes.
  pw-protocol/    (reescrito)
      gnet/               traits Protocol/Encode/Decode, registry por type-id, framing
      v126/               protocolos do realm 1.2.6
      v153/               protocolos do realm 1.5.3
      gamedata/           subcomandos GamedataSend (mundo 3D), por versão
  pw-core/                tipos de domínio (sem I/O, sem protocolo)
  pw-crypto/              RC4, MD5, Argon2, tokens
  pw-data-loader/         elements/tasks/gshop/aipolicy/npcgen/collision
                          + identidade de versão do realm (edition string)
  pw-storage/             repositórios Postgres (papel do gamedbd)
  pw-bus/         (novo)  barramento tipado entre daemons: in-process (canais Tokio)
                          ou TCP, escolhido por configuração
  pw-link/        (enxuto) só gateway: sessão TCP, cripto, framing, roteamento.
                          Não conhece regra de jogo.
  pw-delivery/            conta, rolelist, create/delete role, chat, party, mail, amigos
  pw-gs/                  mundo 3D: entidades, grid espacial, AI, combate, quests, itens
                          + servidor de rede real (hoje inexistente)
  pw-auth/                autenticação global
  pw-uniquename/          unicidade de nomes
tools/
  pw-rpcgen/      (novo)  extrai o esquema canônico dos fontes C++ → IR JSON
  pw-patch-tool/          gerador de patch CDN
specs/protocol/   (novo)  IR JSON versionado — fonte da verdade nos testes
```

### 2.2 Como o `pw-rpcgen` se encaixa

Conforme decidido: o rpcgen **não** gera o código de produção. Ele produz um
**IR (representação intermediária) em JSON**, versionado no repositório, descrevendo cada
protocolo: nome, type-id, campos ordenados com tipos, `SizePolicy`, `PriorPolicy`.

O código Rust de protocolo é escrito à mão, idiomático e bem arquitetado — e cada tipo é
**verificado contra o IR por um teste de conformidade** que compara ordem e tipos dos campos.
Se alguém trocar dois campos de lugar, o teste quebra apontando a divergência com o C++
original. Ganha-se a exatidão da geração sem herdar código gerado feio.

```
inl/*, rpcdata/*, callid.hxx, rpcalls.xml
                │
                ▼   tools/pw-rpcgen
      specs/protocol/gnet_153.json      ──┐
      specs/protocol/gamedata_153.json    ├──►  testes de conformidade
                                          │            ▲
      crates/pw-protocol (escrito à mão) ─┘────────────┘
```

---

## 3. Faseamento

### Fase 1 — Fundação de protocolo
1. `tools/pw-rpcgen`: parser dos `inl/`, `rpcdata/`, `callid.hxx`, `rpcalls.xml` → IR JSON.
2. Parser do `EC_GPDataType.h` → IR dos subcomandos `GamedataSend`.
3. `crates/pw-wire` novo, com testes de vetor contra o `marshal_i386.h`.
4. `crates/pw-protocol` reescrito sobre o IR, com testes de conformidade.
5. Correção do handshake 1.5.3 (`version` + `edition` derivados dos `.data`; `nonce` com
   `Attr` + newbie time; `Response` com `use_token` + `cli_fingerprint`).

**Critério de aceite:** cliente 1.5.3 passa do Challenge e chega à tela de personagens.

### Fase 2 — Arquitetura de daemons
6. `crates/pw-bus` e desmonte do `gateway.rs` nos papéis corretos.
7. `pw-gs` ganha servidor de rede e entra no caminho do jogo.
8. `docker-compose.yml` sobe os daemons de cada realm de fato.

**Critério de aceite:** o `gateway.rs` deixa de existir; nenhum arquivo de gameplay dentro
do `pw-link`; 1.2.6 continua entrando no mundo, agora servido pelo `pw-gs`.

### Fase 3 — Gameplay 1.2.6
9. Engenharia reversa complementar dos binários 1.2.6 (`gs`, `gdeliveryd`, `glinkd`,
   `libtask.so`) + `elementclient.exe`, comparando com os fontes 1.5.3.
10. Complementar o `REVERSE_ENGINEERING_126_MASTER.md` com o que faltar.
11. Implementar: spawns reais do `npcgen`, AI, combate, skills, quests (`tasks.data`),
    inventário, NPCs/serviços, drop, party — tudo lendo do banco novo.

**Critério de aceite:** matar monstro, subir de nível, aceitar/entregar quest, aprender e
usar skill, comprar/vender/equipar item — validado com o cliente real.

### Fase 4 — Realm 1.5.3
12. Transcrição dos fontes 1.5.3 (`cnet/gdeliveryd`, `cgame/gs`, `cskill`) para o
    `pw-delivery`/`pw-gs`, com toda a busca de dados no banco novo.

### Fase 5 — Painel admin
13. Realms, contas, mapas, rates (modelados sobre `ServerAttr` + `ExpRate`).
14. Personagens e templates para ambos os realms.
15. Editor visual de itens: carrega `elements.data`, mostra os atributos reais do item, e
    monta os *octets* corretos por baixo dos panos. Sem edição manual de bytes.

### Fase 6 — Testes
16. Testes unitários por cenário, por pipeline e por handshake, incluindo replays de
    capturas reais do cliente.

---

## 4. Ambiente de trabalho

| Onde | O quê | Por quê |
| :--- | :--- | :--- |
| Máquina do usuário (`F:\...`) | Fonte canônico, `.data`, clientes, binários originais | É onde o projeto vive e onde os clientes rodam |
| Container cloud | `cargo build`, `cargo test`, `docker` | A VM local não tem `cargo` nem `docker` nem rede |

Sincronização por *tarball* nas duas direções (a pasta `_sync/` do projeto), para não
transferir arquivo a arquivo.

Validação disponível e acordada com o usuário:
- Docker + cliente 1.2.6 rodando, com envio de logs.
- Captura de tráfego (Wireshark/pcap) — padrão-ouro para conferência byte a byte.
- Binários originais 1.2.6 executáveis, para comparação lado a lado.
