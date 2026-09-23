# Especificação Mestre 00: o pw-universal-server

> Verificada contra o código em 2026-09-14, commit `e6433ae`. Índice das specs e regra de
> manutenção: [`README.md`](README.md).

## 1. O que é

Reimplementação em **Rust** do servidor do MMO **Perfect World**, com **PostgreSQL 16** e
**DragonflyDB**, que serve o **cliente original sem modificação** e roda vários realms de
versões diferentes sobre a mesma infraestrutura de contas.

| versão | situação | referência |
| :--- | :--- | :--- |
| **1.5.5** | **alvo atual** (desde 2026-09-02). Realm de teste `realm_155` (cliente BR; até 2026-09-17 `realm_155BR`) | fontes EvolvedPW (cliente e servidor), binário do cliente BR build 2569 |
| 1.2.6 | segunda prioridade; loga e entra no mundo | capturas da VM com o servidor original, `docs/MEDIDAS_DO_126.md` |
| 1.5.3 | abandonado (o cliente disponível nunca logou) | seus fontes geraram o IR do protocolo, que continua válido para o 1.5.5 |
| 1.4.8 | nunca foi alvo | — |

Ordem de trabalho combinada: 1.5.5 funcional → 1.2.6 funcional → banco, painel e launcher.

## 2. Princípios

1. **O cliente original é o juiz.** O servidor se adapta ao que o `elementclient.exe`
   instalado aceita. Onde o fonte do cliente e o binário discordam, vale o binário (o fonte
   1.5.5 é de uma build anterior à instalada).
2. **Evidência, nunca palpite.** Todo layout, número e regra sai de: fonte C++ original
   (`EvolvedPWServer`/`EvolvedPWClient`, 1.7.2 para campos mais novos), IR gerado desses
   fontes, captura de tráfego, ou do próprio arquivo de dados. Nada é deduzido do nome da
   versão.
3. **Porta das regras do original, não invenção.** Fórmulas, tempos e unidades vêm do
   `cgame/gs/` original com arquivo e linha citados no código. Onde ainda não foi portado,
   o código diz isso e não inventa número.
4. **Dado de realm é dado, não código.** `elements.data`, `tasks.data`, `npcgen.data`,
   `ptemplate.conf`, `gs.conf`, moldes de classe: lidos do realm, nunca tabelas no código.
5. **Um caminho de escrita por layout.** Diferença entre versões num lugar só
   (`pw_protocol::PorVersao`), não em ramos espalhados.
6. **Teste contra a fonte, não contra si mesmo.** Codificadores são conferidos contra o IR;
   leitores de arquivo contra o arquivo real do realm, fechando no último byte.
7. **Contexto é recurso.** As specs são curtas para que cada sessão leia pouco, e a mesma
   economia vale para o trabalho: saída de ferramenta entra filtrada (`grep`, `awk`,
   `tail`), rodada longa vai para segundo plano, e o que se traz é o número, não a lista.
   Quem trabalha aqui relata o consumo de tokens por etapa — ver
   `.claude/agents/pw-server-dev.md` e a skill `pw-testar-e-publicar`.

## 3. Arquitetura em uma figura

```
cliente original ──TCP──▶ pw-link (1 por realm, porta pública)
                           │  login, lista/criação de personagem, entrada no mundo,
                           │  fala, e parte do gameplay que ainda não migrou (gateway.rs)
                           │
                           ├──barramento GNET interno (29100, nunca publicado)──▶ pw-gs (1 por mapa)
                           │                                                     simulação: tick, spawns,
                           │                                                     visibilidade, IA, combate,
                           │                                                     itens, NPCs, autosave
                           ▼
                     PostgreSQL 16 + DragonflyDB (compartilhados por todos os realms)
```

Detalhes: [`02_MULTI_REALM_ARCHITECTURE.md`](02_MULTI_REALM_ARCHITECTURE.md).

Os daemons originais (`glinkd`, `gdeliveryd`, `gamedbd`, `gs`, `uniquenamed`, `authd`) não
têm correspondência um-para-um: hoje o `pw-link` faz o papel de `glinkd` + `gdeliveryd` +
parte do `gamedbd`, e o `pw-gs` o de `gs`.

## 4. Crates

| crate | papel | estado |
| :--- | :--- | :--- |
| `pw-core` | tipos comuns: classes, raças, vetores, personagem, itens, fichas de equipamento | em uso |
| `pw-crypto` | RC4 do elo com o cliente, hashes de senha, tickets de sessão | em uso |
| `pw-wire` | os dois formatos de fio: GNET (big-endian, `CompactUINT`) e gamedata (little-endian, `pack(1)`) | em uso; conformidade contra o IR |
| `pw-protocol` | opcodes, pacotes GNET, subcomandos S2C, `PorVersao`, `edition`, versão | em uso; ainda com `octets.rs`/`adapter.rs` duplicando o `pw-wire` |
| `pw-bus` | barramento `pw-link`↔`pw-gs` (4 mensagens GNET reais) | em uso |
| `pw-storage` | repositórios PostgreSQL (contas, personagens, itens, habilidades, missões, moldes, realms) e cache | em uso |
| `pw-data-loader` | leitores de `elements`/`tasks`/`npcgen`/`aipolicy`/`gshop`, `ptemplate.conf`, `.hmap`, `.sev` | em uso |
| `pw-link` | daemon de link por realm (`gateway.rs`, `uplink.rs`) | em uso |
| `pw-gs` | servidor de mundo (`bus_server.rs`, `world.rs`, `ai.rs`, `combat.rs`, `habilidades.rs`) | em uso |
| `pw-auth` | serviço de autenticação (porta interna 29200) | sobe no compose; **o `pw-link` não depende dele** — autentica direto pelo `pw-storage` |
| `pw-delivery` | chat, amigos, correio, grupo | **não usado**: nenhum daemon depende dele; a fala está no `pw-link` e o grupo no `pw-gs` |
| `pw-uniquename` | unicidade de nomes | **não usado** |

Ferramentas (`tools/`): `pw-rpcgen` (IR do protocolo a partir dos fontes C++),
`pw-pcapdiff` (capturas), `pw-crash-re` (minidump e desmontagem do cliente),
`pw-pck-extract` (pacotes `.pck` acima de 2 GB), `pw-ir/consultar_ir.py` (consulta um comando ou struct
no IR), `pw-patch-tool` (patcher, planejado).

Fora do Rust: `web-admin/` (backend FastAPI `main.py` na porta 8000, frontend estático) —
ver [`06_ADMIN_PANEL_AND_CPW_SPEC.md`](06_ADMIN_PANEL_AND_CPW_SPEC.md).

## 5. Onde está a verdade de cada coisa

| pergunta | fonte |
| :--- | :--- |
| onde o trabalho parou, o que falta | `docs/ESTADO_E_RETOMADA.md` |
| por que algo é assim (evidência) | `docs/HISTORICO_DE_SESSOES.md`, item citado |
| layout de subcomando | `specs/protocol/gamedata_155.json` + `EvolvedPWClient/.../EC_GPDataType.h` + overlay do cliente |
| regra de jogo | `EvolvedPWServer/cgame/gs/*.cpp` |
| formato de arquivo de dados | o carregador **do cliente** (`elementdataman::load_data`, `ATaskTempl::LoadBinary`) e o tamanho do arquivo |
| como testar | `docs/COMO_TESTAR.md`, seção 2 do `ESTADO_E_RETOMADA.md` |
