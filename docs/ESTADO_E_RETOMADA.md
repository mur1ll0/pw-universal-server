# Estado atual e como retomar

> Nota de passagem entre sessões. Diz onde o trabalho parou, o que já é fato verificado
> e qual é o próximo passo concreto. Atualizar ao fim de cada bloco de trabalho.

## MUDANÇA DE BASE (2026-09-02): 1.5.3 abandonado, 1.5.5 é a versão de referência

Decidido pelo Murillo em 2026-09-02, **depois** da última rodada de itens numerados abaixo
(item 62): **o 1.5.3 não é mais o alvo do projeto.** Motivo: o client 1.5.3 disponível não
tinha fonte confiável e nunca chegou a logar de fato (ver seção "Nota anterior" logo abaixo,
"1.5.3 nem loga" — esse problema nunca foi resolvido, e não vai ser, porque o alvo mudou). Em
seu lugar, o projeto ganhou acesso aos fontes completos do **1.5.5** (projeto da comunidade
**EvolvedPW** — servidor e cliente, C/C++, mesma família dos fontes 1.5.3) e a um client 1.5.5
já traduzido para inglês, sem o mistério do repack russo do 1.5.3.

**A ordem de prioridade combinada com o Murillo, deste ponto em diante:**

1. **1.5.5 totalmente funcional** — é a prioridade atual. Ver "## 4. Próximo passo" abaixo,
   já reescrito para refletir isso.
2. **Depois, 1.2.6 totalmente funcional** — o realm 1.2.6 já loga e entra no mundo hoje, mas
   skills/missões/HP de NPC ainda falham (item 62); retomar isso só depois do item 1 acima.
3. **Só depois**: banco de dados (campos faltantes, Contexto E do roadmap), pw-admin
   (Contexto G), atualizador/launcher (Contexto H) e os demais ajustes de infraestrutura.

**O que continua valendo, sem refazer**: os ~62 itens numerados abaixo (seções 1-3) foram
todos extraídos do **IR do protocolo** (`pw-rpcgen` contra os fontes 1.5.3) ou de captura real
contra o cliente 1.2.6 — não são específicos de "rodar o client 1.5.3". Uma comparação
programática do IR 1.5.3×1.5.5 (documentada na memória `pw_ctx_a_155_funcional`) provou que o
**1.5.5 não troca nem remove nenhum id que o 1.5.3 já tinha, só acrescenta** (698/698 GNET,
193/194 C2S, 398/398 S2C iguais por nome+id), e os structs compartilhados são **idênticos ou só
ganham campos novos no fim** (514/521 comandos de gamedata, 691/698 protocolos GNET). Ou seja:
**todo o trabalho de protocolo já feito contra os fontes 1.5.3 continua válido para 1.5.5** —
não é para descartar nem para refazer, só para estender com os itens novos (66 protocolos GNET,
17 C2S, 24 S2C) quando chegar a hora (ver "## 4. Próximo passo").

O que **muda de verdade** com o 1.5.5 é a camada de dados de referência
(`elements.data`/`npcgen.data`/`tasks.data`/`gshop*.data`) — formato binário completamente
diferente do que este documento tratava até aqui, decodificado do zero numa sessão dedicada.
Ver `specs/elements_155/README.md` (a arqueologia completa, 231/231 tabelas do `elements.data`
resolvidas byte a byte) e `specs/elements_layouts/README.md` (o extrator reaproveitável,
Rust+Python, resultado desse trabalho) — não repetidos aqui.

---

**Última atualização (histórico, pré-mudança de base):** 2026-09-02 — teste de ponta a ponta
do Murillo contra o docker-compose (não a VM): 1.2.6 loga e entra no mundo mas
skills/missões/HP de NPC falhavam; 1.5.3 nem loga. Diagnóstico e correção parcial nesta
rodada — item 62.

**Nota anterior (2026-09-01):** Fase 2 em andamento, com **vinte e três subcomandos**
no `pw-gs`. O achado daquela rodada: o `USE_ITEM` reconhecia poção por **dois ids escritos no
código** e respondia HP/MP fixos sem curar nada, enquanto os valores reais já estavam
carregados em `elements.medicines` e nunca eram lidos (item 42); e o `CAST_SKILL` lia o
alvo do deslocamento errado, com dano fixo (item 43).

Na rodada anterior, **a loja de NPC estava invertida** (item 40) — os nomes do enum são do
ponto de vista do NPC, e o código os lia do ponto de vista do jogador.

E antes dela, o `match` de subcomandos do `gateway.rs` nunca tinha sido conferido
contra o IR, e **oito ids estavam errados** (item 39) — entre eles, comprar na loja do Mall
e consultar saldo, que estavam **trocados entre si**.

E antes dela, ao migrar os comandos de item, apareceram **duas perdas de dados silenciosas
no repositório** (item 37): cada troca de slot apagava os octetos do item, e a operação
rodava sem transação.

Antes disso, em ordem: a falha de autorização de personagem (itens 29–31), a primeira
conferência dos codificadores S2C contra o IR (item 31), o combate fictício (32–34) e os
dois sentidos do mundo (35–36).

**O critério de aceite da fase ainda não foi atingido** — ver seção 4.

---

## 1. O que já está pronto e verificado

### O bloqueio de ambiente acabou

O contêiner de nuvem tem acesso ao crates.io (`index.crates.io/config.json` responde
200) e **o workspace inteiro compila**. `cargo test --workspace` passa com **247
testes**, dos quais 31 ficam mudos sem um PostgreSQL (ver `docs/COMO_TESTAR.md`). A vendorização de dependências descrita em versões anteriores deste documento
deixou de ser necessária. A política de rede é fixada quando o contêiner sobe, então
vale reconfirmar ao retomar:

```bash
curl -s -o /dev/null -w "%{http_code}\n" https://index.crates.io/config.json   # espera 200
cargo build --workspace
```

### `tools/pw-rpcgen` — extrator dos dois esquemas, pelos dois lados

Ferramenta sem dependências externas (compila offline), **84 testes**. Extrai os dois
modelos de fio que convivem na mesma conexão:

```bash
cargo run -p pw-rpcgen -- \
  --server-src <fontes do servidor 1.5.3> --out          specs/protocol/gnet_153.json \
  --client-src <fontes do cliente  1.5.3> --out-gamedata specs/protocol/gamedata_153.json
```

Os dois modos são independentes (`--strict` falha se houver qualquer diagnóstico), mas
**passar os dois de uma vez muda o resultado do gamedata**: é o que permite ler o lado
do servidor, sem o qual os comandos C2S ficam sem struct e a conferência cruzada não
acontece.

* **GNET:** 620 estruturas, 698 protocolos (todos com id), 237 RPCs, **sem
  diagnósticos**. Saída byte a byte idêntica à das sessões anteriores.
* **Gamedata:** 592 entradas de comando (398 S2C, 194 C2S), 1.190 structs empacotadas,
  361 comandos S2C ligados à struct do cliente e 157 comandos C2S ligados à struct do
  servidor. **26 diagnósticos**, todos entendidos e caracterizados (seção 3).

### `specs/protocol/gamedata_153.json` — o IR do mundo 3D, com os dois lados

Para cada comando: nome, id, papel (`command`, `reserved`, `count`), a struct do
cliente, o **nome que o servidor dá ao mesmo comando** e a struct do servidor. Para cada
struct: os campos na ordem exata, com tipo, comprimento de array, tamanho e
**deslocamento em bytes**.

### Como o IR foi verificado

Cinco verificações independentes, quatro delas cruzando arquivos diferentes:

1. **Âncoras dos enums.** Os fontes numeram uma entrada a cada cinco (`// 5`, `// 10`,
   … `// 395`). **115 âncoras conferidas, nenhuma divergência.**
2. **Fixo vs. variável.** `CalcS2CCmdDataSize` escolhe entre `sizeof(T)` e
   `CHECK_VALID(T)`; a escolha tem que concordar com a presença de um método
   `CheckValid` na declaração da struct, que está no *outro* arquivo. Batem, e o total
   confere com os 43 `CHECK_VALID` da tabela.
3. **Referências resolvidas.** Toda struct citada e todo tipo de campo existem na tabela
   de declarações, ou viram diagnóstico. Hoje: nenhuma pendente.
4. **O compilador C++, no alvo original de 32 bits.**
   `tools/pw-rpcgen/verify/check_sizes.py` gera `static_assert`s a partir do IR e compila
   os cabeçalhos **originais** com `g++ -m32 -fsyntax-only`:

   ```bash
   python3 tools/pw-rpcgen/verify/check_sizes.py \
     --ir specs/protocol/gamedata_153.json \
     --client-src <fontes do cliente> \
     --server-src <fontes do servidor>
   ```

   **4.426 asserções** — 1.928 do cliente e 2.498 do servidor, entre tamanhos de struct,
   tamanhos de membro aninhado e deslocamentos de campo — **todas batem**. É esta
   verificação que prova o `pack(1)` campo a campo, e foi ela que apontou **cinco** erros
   reais do parser que nenhum teste de unidade teria pego (seção 2, item 13).

5. **Cliente contra servidor.** Os dois lados têm cabeçalhos escritos separadamente, com
   nomes diferentes para tudo, e mesmo assim precisam produzir os mesmos bytes. **590
   comandos casados por id; 305 pares de struct comparados escalar a escalar; 300
   idênticos.** Os 5 restantes estão explicados um a um na seção 3.

### `crates/pw-wire` — os dois formatos de fio

Um crate por baixo de tudo que fala com o cliente, com um módulo por formato e nenhum
conhecimento de protocolo. **25 testes.**

| | `gnet` | `gamedata` |
| :--- | :--- | :--- |
| Onde | protocolos GNET entre cliente e daemons | subcomandos do `GamedataSend` |
| Ordem de bytes | big-endian | little-endian |
| Tamanhos | `CompactUINT` antes de `Octets`, strings e contêineres | nenhum; a contagem é campo explícito |
| Alinhamento | não se aplica | `pack(1)`, endereçável por deslocamento |

O `CompactUINT` tem teste das quatro formas **e dos padrões de bits exatos** — uma
ida-e-volta consigo mesmo passaria mesmo com a codificação errada; os bytes literais de
`marshal_i386.h` não.

**Conformidade dirigida pelo IR** (`crates/pw-wire/tests/`), que é o que dá peso ao
crate:

* `conformance_gamedata` — escreve os campos de cada struct em sequência e cobra que
  cada um caia no deslocamento que o IR anuncia. Como esses deslocamentos vieram do
  `g++ -m32` lendo os cabeçalhos originais, isto compara o empacotamento do crate com o
  do compilador de verdade: **1.064 structs, 3.822 escalares**, mais 2.544 campos lidos
  pelo deslocamento.
* `conformance_gnet` — escreve e relê cada estrutura na ordem e nos tipos do IR:
  **620 de 620 estruturas, 12.121 passos**, incluindo `Octets` vazio, contêineres com
  zero elementos e a `OctetsTree`, que é recursiva.

### `crates/pw-protocol` — opcodes conferidos contra o IR

O `opcodes.rs` foi reescrito a partir do IR. Cada constante carrega o símbolo
`PROTOCOL_*` do C++ e entra na tabela `CONFERIDOS`; o teste
`tests/opcodes_contra_o_ir.rs` compara as **40 constantes** e os **12 subcomandos** de
`gamedata` contra os dois IRs, de modo que um número escrito à mão não sobrevive a um
`cargo test`. Um segundo teste cobra que dois símbolos diferentes nunca reivindiquem o
mesmo id, que foi como `SetCustomData` e `SetUIConfig` colidiam em 102/103.

`tests/octets_contra_pw_wire.rs` prova que o `OctetsStream` antigo e o `pw_wire::gnet`
produzem **os mesmos bytes**, incluindo as quatro formas do `CompactUINT` e as fronteiras
entre elas. É o que torna a migração de `octets.rs` para o `pw-wire` uma troca segura em
vez de um salto no escuro.

`tests/campos_contra_o_ir.rs` vai além dos opcodes e confere **o conteúdo** dos pacotes.
Como `encode()` é código e não dado, ele usa a mesma técnica do `pw-rpcgen` com o C++:
lê `packets/s2c.rs` e `packets/c2s.rs`, extrai a sequência de `write_*`/`read_*` de cada
`encode`/`decode` — seguindo chamadas a auxiliares como `write_role_info` — e compara
com a estrutura achatada do IR. Hoje: **31 pacotes, 214 escalares, todos batendo**.
Foi assim que os nove layouts errados do item 22 apareceram.

### `crates/pw-bus` — o barramento entre daemons

O que `glinkd` e `gamed` dizem um ao outro. Não é formato inventado: são protocolos GNET
reais do IR, e o campo `daemons` de cada um diz em que pernas ele trafega — é o que
sustenta a separação entre o `GamedataSend` (34), falado só pelo `glinkd` com o cliente,
e o par 74/75, falado pelas duas pontas.

| Mensagem | Opcode | O que carrega |
| :--- | ---: | :--- |
| `PlayerLogout` | 69 | `result`, `roleid`, `provider_link_id`, `localsid` |
| `EnterWorld` | 72 | `roleid`, `provider_link_id`, `locktime`, `timeout`, `settime`, `localsid` |
| `S2CGamedataSend` (`GameToClient`) | 74 | `roleid`, `localsid`, `data` |
| `C2SGamedataSend` (`ClientToGame`) | 75 | `roleid`, `localsid`, `data` |

A diferença que define o barramento está nos campos: o `GamedataSend` do cliente tem
**só** `data`, porque a conexão já sabe de quem é; entre daemons o mesmo payload precisa
de `roleid` e `localsid`, senão o servidor de mundo não sabe de quem veio nem por onde
responder. Isso é conferido contra o IR, não assumido.

Enquadramento `[CompactUINT(opcode)][CompactUINT(len)][body]`, limite de 1 MiB por
quadro. **21 testes**, entre eles TCP de verdade em ambos os sentidos, rajada de 50
mensagens com payloads maiores que o MTU (TCP é fluxo, não mensagem — é onde um
enquadramento frágil quebra) e dois jogadores numa conexão só sem se misturarem.

`tests/topologia_do_compose.rs` lê o `docker/docker-compose.yml` e cobra que **a porta do
barramento nunca seja publicada**. Ela não tem autenticação nenhuma: quem a alcança manda
`EnterWorld` por qualquer `roleid`. Uma regra que só existe num comentário volta a ser
quebrada; esta falha o `cargo test`. O mesmo teste confere que cada `GS_BUS` aponta para
um serviço que existe, roda o `pw-gs`, escuta naquela porta e é do mesmo realm e versão.

### O `pw-gs` na rede, e o `pw-link` ligado a ele

- `pw-gs/src/bus_server.rs` — a ponta que escuta. Roteia por `roleid`, e é onde um
  formato vira o outro: o envelope GNET já foi desfeito pelo `pw-bus`, e o `data` que
  sobra é lido com o `pw_wire::gamedata` (`SubComando::ler`, cabeçalho de 2 bytes
  **little-endian**). `tratar_subcomando` é o ponto de entrada para onde os ~650 linhas
  do `gateway.rs` vão migrar, comando a comando.
- `pw-link/src/uplink.rs` — a ponta que conecta. Uma conexão, muitos jogadores: fila
  única de saída (para duas tarefas não escreverem no mesmo socket) e registro por
  `roleid` na entrada (para o que volta achar o jogador certo). Reconecta sozinha com
  espera crescente até 30s, então a ordem de subida dos contêineres não é corrida.
  Melhor-esforço por decisão: um mundo caído não derruba a sessão do jogador no link.
- `pw-link/tests/uplink_contra_o_mundo.rs` — dois jogadores no mesmo link recebem cada um
  o **seu** payload (cruzá-los é o bug que não aparece em teste de formato nenhum), e o
  uplink liga-se a um mundo que só sobe depois.

### Os primeiros subcomandos já mudaram de lado

`crates/pw-gs/src/comandos.rs` decodifica os subcomandos do mundo 3D, com **todos os
deslocamentos vindos do IR**. `tests/comandos_contra_o_ir.rs` usa uma técnica nova e mais
forte que ler o código-fonte: monta o payload **a partir do IR**, pondo um valor distinto
no deslocamento que ele anuncia, e cobra que a decodificação devolva aquele valor naquele
campo. Trocar dois `u16` adjacentes de ordem — o erro que nem o compilador nem uma revisão
pegam — derruba o teste.

Migrados até aqui:

| Comando | O que mudou |
| :--- | :--- |
| `PLAYER_MOVE` (0) | o mundo atualiza entidade **e** grade espacial; o autosave grava. Antes: um `UPDATE` no PostgreSQL **por pacote de movimento** |
| `LOGOUT` (1) | o mundo tira o jogador da simulação e devolve um `PlayerLogout` (69) pelo barramento; o `uplink.rs` traduz no pacote GNET que o cliente espera |
| `SELECT_TARGET` (2) | o cliente recebe o **HP real** do alvo. O `gateway.rs` mandava `1000/1000` fixo, porque o daemon de link não sabe o estado das criaturas — é a razão de o comando pertencer a este lado |
| `NORMAL_ATTACK` (3) | dano do `CombatEngine` com os atributos dos dois lados, HP debitado, monstro que morre, exp concedida, abate de missão só **na morte** e com o template real |
| `STOP_MOVE` (7) | atualiza mundo e grade, sem `UPDATE` por parada |
| `UNSELECT` (8) | desmarca no mundo, que é quem guarda o alvo desde que o comando 2 migrou |
| `REVIVE_VILLAGE` (4) | **novo**: não havia tratamento nenhum. Quem zerava a vida ficava preso até reconectar |
| Itens (9, 11, 12, 13, 16, 17, 18) | consulta, troca de slot, mover e equipar — todos agora sobre um repositório transacionado que não apaga os atributos do item |
| Ações (42, 46, 47, 48, 75) | sentar, levantar, cancelar ação, emote e zona segura |
| `SEVNPC_SERVE` (37) | os treze serviços de NPC, com a loja **desinvertida** e a cura usando os valores do jogador em vez de 120/280 fixos |
| `USE_ITEM` (40) | poção cura pelo `MedicineTemplate` do `elements.data`; o slot é lido com os **dois** bytes que ele tem |
| `CAST_SKILL` (41) e `CAST_INSTANT_SKILL` (80) | alvo lido da lista, no deslocamento certo; dano do `CombatEngine` |
| Grupo (27, 28, 29, 30) | **estado de grupo de verdade**, que não existia em lugar nenhum — ver o item 45 |
| Consultas (21, 39, 67, 68, 110) | vida, mana, nível e saldo **do personagem**, e vida real de monstro na consulta periódica. Todas respondiam número escrito no código, ou nada — ver o item 49 |

E o caminho de volta, que não existia: a simulação publica [`EventoDoMundo`] e o
`BusServer` traduz em subcomandos. Três eventos hoje — dano recebido, morte e
renascimento — com os codificadores `HOST_ATTACKED` (26), `HOST_DIED` (28) e
`PLAYER_REVIVE` (29), **escritos a partir do IR** e por isso fora da lista de divergências
do item 31.

Os dois braços **saíram** do `gateway.rs` — a migração é real, não uma cópia. O resto
(~390 comandos) continua lá e o mundo só os registra.

`pw-gs/tests/subcomandos_no_mundo.rs` prova o caminho inteiro com mundo real, barramento
real e TCP real: um `PLAYER_MOVE` chega e move o jogador (na `cur_pos`, não na `next_pos`,
e a grade acompanha); um `LOGOUT` tira o jogador e devolve o aviso ao link com o `localsid`
do `EnterWorld`. Injetar cada um desses dois erros derruba o teste.

### Documentação

- `docs/COMO_TESTAR.md` — o que dá para verificar hoje, em quatro níveis, com o que
  observar e o que significa cada falha.
- `docs/MULTIPLOS_REALMS.md` — a diferença entre outro realm e outro mundo no mesmo
  realm, a receita para subir um segundo realm da mesma versão, e o que é compartilhado
  entre eles.
- `docs/PLANO_ARQUITETURA_E_EXECUCAO.md` — diagnóstico, arquitetura-alvo e as 6 fases.
- `docs/REVERSE_ENGINEERING_126_MASTER.md` — tabela de opcodes corrigida e seção 12, com
  a fonte canônica, as regras primitivas de codificação e a causa da falha de login.

---

## 2. Fatos verificados que mudam decisões

Itens 1–5 vêm da primeira sessão; 6–12 da segunda; 13–18 da terceira; 19–21 da quarta;
22 da quinta; 23–25 desta.

1. **Os opcodes estavam muito mais errados do que "quatro trocados".** Uma auditoria de
   todas as constantes contra o IR encontrou **doze com valor errado e cinco sem
   protocolo correspondente**. Já corrigido e travado por teste (item 19).

2. **`GamedataSend` é um só opcode (34) nos dois sentidos** entre cliente e `glinkd`.
   `S2CGamedataSend` (74) e `C2SGamedataSend` (75) são internos entre `glinkd` e
   `gdeliveryd` e carregam `roleid` + `localsid`.

3. **Login do 1.5.3 falha no `Challenge`**, antes de qualquer verificação de senha. O
   `version` **já foi corrigido** (item 20); o `edition` continua vazio e é a próxima
   causa conhecida: o cliente espera
   `sprintf("%x%x%x%x", elements_ver, task_ver, gshop_ts, gshop_ts2)` derivado dos
   `.data` do realm.

4. **`Challenge.nonce` tem estrutura**: `[Attr: u32][newbie_time: u32][aleatório]`.
   `Attr` carrega `load`, `lambda` e os bits `doubleExp`/`doubleMoney`/`doubleObject`/
   `doubleSP`/`freeZone`/`bSellpoint`/`bBattle`/`pvp`. É por aqui que os rates do realm
   chegam ao cliente — o painel admin deve modelar esses campos.

5. **`pw-gs` não tem servidor de rede** e não está no caminho do jogo. O
   `docker-compose.yml` sobe apenas `pw-link` por realm, e o `gateway.rs` (1.347 linhas)
   encena a entrada no mundo com NPCs e posições escritos à mão.

6. **O nome da struct só é único dentro do namespace.** 555 declarações para 525 nomes:
   **30 nomes existem nos dois namespaces do cliente** (`cmd_header`, `cmd_equip_item`,
   `cmd_select_target`, …) **com campos diferentes**. Toda referência no IR é
   qualificada.

7. **O cliente não tem tabela de tamanhos para o C2S.** Ele só tem
   `CalcS2CCmdDataSize`; envia os comandos C2S, então nunca precisa calcular o tamanho
   deles. E **ligar por convenção de nome não serve**: `cmd_<nome minúsculo>` discorda da
   tabela autoritativa em **46 dos 361** comandos S2C que ela cobre (13%). Medido, não
   estimado. Foi o que motivou ler o lado do servidor (itens 14–16).

8. **Blocos `/* */` guardam código de exemplo que declara structs.** Em
   `cmd_player_info_2_list`, `cmd_player_info_3_list`, `cmd_player_info_23_list` e
   `cmd_player_booth_info` o layout "real" aparece comentado, com a observação de que a
   estrutura verdadeira é de tamanho variável. O campo real dessas structs é
   `char data[1]`.

9. **`} *data;` é ponteiro, não membro embutido.** `cmd_unique_data_notify` declara uma
   struct aninhada e em seguida um ponteiro para ela — endereço do processo do cliente,
   4 bytes no alvo, nenhum dado no fio.

10. **Existe uma terceira forma de serialização.** Além do `memcpy` de tamanho fixo e do
    `CheckValid` de tamanho variável, cinco structs têm `bool Initialize(...)`, que
    extrai campo a campo com `Extract()` e lê os `abase::vector<T>` como contagem
    seguida dos elementos. Marcadas com `variable.form = "initialize"`.

11. **`info_player_1` e `info_npc` têm campos condicionais a bits** de `state`
    (`GP_STATE_ADV_MODE`, `GP_STATE_SHAPE`, `GP_STATE_EMOTE`,
    `GP_STATE_EXTEND_PROPERTY`, `GP_STATE_FACTION`, `GP_STATE_BOOTH`). O tamanho depende
    do **conteúdo**. Marcadas com `variable.form = "conditional"`.

12. **Nem toda entrada dos enums é um comando.** `PROTOCOL_COMMAND = -1` é reservado e
    `NUM_C2SCMD` é a contagem de comandos C2S, que cai no valor 180. O IR os marca com
    `role` `reserved`/`count`. Prova de que a classificação está certa: são exatamente os
    **únicos 2** entre 592 que não casaram com o enum do servidor.

13. **O compilador de 32 bits achou cinco erros de extração** que nenhum teste de unidade
    pegaria, porque nenhum deles deixava buraco visível — todos apenas empurravam os
    campos seguintes:
    código de exemplo em `/* */` lido como campo; `} *data;` tratado como membro
    embutido; `int64_t`/`uint64_t` ausentes da tabela de tipos; **`struct{`** sem espaço
    antes da chave não reconhecido como struct aninhada (os campos internos viravam
    campos da externa, em `force_global_data` e `public_quest_ranks`); e **campos com
    nome iniciado em `_`** descartados em silêncio (`int _task_id;`). A lição é o
    método: gerar asserções a partir do IR e deixar o compilador original julgar.

14. **O servidor descreve o mesmo protocolo, em `cgame/`.** Quatro arquivos:
    `common/types.h` (os cabeçalhos de comando), `common/protocol.h` (enums e structs em
    `S2C::{INFO,CMD}` e `C2S::{INFO,CMD}`), `common/protocol_imp.h` (as especializações
    `Make<CMD::x>` que **emitem** cada comando S2C — a contrapartida exata do
    `CalcS2CCmdDataSize`) e `gs/playercmd.cpp` (o `switch` que **recebe** os C2S).

15. **As structs do servidor incluem o cabeçalho de comando; as do cliente não.** São
    três cabeçalhos e a diferença importa: `single_data_header` e `cmd_header` têm só o
    opcode (2 bytes), mas **`multi_data_header` tem opcode e contagem (4 bytes)** — e a
    contagem faz parte do payload do lado do cliente, cujas structs de lista abrem com um
    `unsigned short count`. Descontar 2 bytes uniformemente é o que alinha os dois casos.

16. **Os nomes divergem por completo entre os dois lados**, e o casamento é por id:
    `EXG_IVTR_ITEM` ↔ `EXCHANGE_INVENTORY_ITEM`, `GM_KICK_PLAYER` ↔ `GMCMD_OFFLINE`,
    `cmd_exg_ivtr_item` ↔ `exchange_inventory_item`. Também os **arranjos** diferem: o
    servidor agrupa o payload numa struct aninhada (`info`, `data`) onde o cliente
    escreve os campos soltos. Por isso a conferência compara a **sequência de escalares
    no fio**, não listas de campos.

17. **Nem todo `case C2S::` liga uma struct.** `playercmd.cpp` tem mais de dez
    `switch(cmd_type)` e vários são listas de permissão ou roteamento, com dezenas de
    rótulos que terminam em `return CommandHandler(...)` sem tocar em struct nenhuma. A
    ligação só é registrada quando a struct aparece de fato — direto no bloco, através da
    macro `DEFCMD(x)` dos comandos de GM, ou por um método auxiliar
    (`cmd_user_move(buf,size)`) cujo corpo faz o cast.

18. **O servidor ainda implementa comandos que o cliente abandonou.** Nos ids C2S
    **209–217** o cliente escreve `//209 ~ 217 obsoleted` e salta de 208 para 218; o
    servidor tem lá nove comandos de GM (`GMCMD_PLAYER_INC_EXP`, `GMCMD_ENDUE_ITEM`,
    `GMCMD_ENDUE_MONEY`, …), mais o 222. Os dois lados voltam a alinhar em 218. É a
    causa de 10 das structs C2S do servidor não terem dono.

19. **O teste de conformidade do `pw-wire` achou dois erros na extração do GNET**, e é
    o tipo de erro que passaria despercebido até o jogo não funcionar:

    * o `marshal` de `share/rpc/rpcdefs.h` usa a forma condensada
      `return os << m_int << m_octets;`, e o parser só aceitava instruções começando em
      `os <<`. Resultado: **`IntOctets` e `RpcRetcode` entravam no IR sem campo
      nenhum** — e `IntOctets` é o elemento da lista de personagens em
      `GetUserRolesRes`, no caminho crítico do login. A lista decodificaria como vazia,
      em silêncio;
    * `os << MarshalContainer(m_children)` é a forma explícita do que o `operator<<` de
      `std::vector` já faz por dentro (`share/common/marshal_i386.h`): contagem em
      `CompactUINT` mais os elementos. Sem desembrulhar, `OctetsTree` perdia a lista de
      filhos.

    Depois do conserto não há mais nenhuma estrutura sem campos no IR do GNET, e o
    `--strict` continua limpo.

19. **Doze opcodes apontavam para o protocolo errado, e cinco não existiam.** Os piores
    não eram os inexistentes, e sim os que colidiam com outro protocolo de verdade:

    | Constante | Era | O que aquele valor é | Certo |
    | :--- | ---: | :--- | ---: |
    | `OP_C2S_RESPONSE` | 2 | `KeyExchange` | **3** |
    | `OP_*_KEYEXCHANGE` | 3 | `Response` | **2** |
    | `OP_C2S_CHAT` | 112 | `GetTaskData_Re` | **80** |
    | `OP_S2C_CHAT_BROADCAST` | 113 | `SetTaskData` | **120** |
    | `OP_C2S_HEARTBEAT` | 90 | `KeepAlive` (entre daemons) | **93** |
    | `OP_C2S_SET_CUSTOM_DATA` | 102 | `SetUIConfig` | **100** |
    | `OP_S2C_SET_CUSTOM_DATA_RE` | 103 | `SetUIConfig_Re` | **101** |
    | `OP_C2S_SET_UI_CONFIG` | 106 | `DisconnectPlayer` | **102** |
    | `OP_S2C_SET_UI_CONFIG_RE` | 107 | `GetPlayerBriefInfo` | **103** |

    Um `OP_C2S_CHAT` valendo 112 não faz o chat "não funcionar": faz o servidor
    **decodificar uma resposta de dados de missão como se fosse chat**.

    O IR também separa dois protocolos que o código confundia: **`KeepAlive` (90) é o
    keepalive entre daemons** (falado por `glinkd`, `gdeliveryd`, `gamed`, `uniquenamed`
    e `gfaction`); o do jogador é **`PlayerHeartBeat` (93)**, falado só por `glinkd` e
    `gamed`.

20. **O `Challenge` mandava `804` fixo no campo de versão**, e o
    `server_version_code()` do 1.5.3 dizia `0x00010503`. O valor certo é
    **`0x00010502`**, e não é dedução — está em `CElementClient/EC_Game.cpp:115`:

    ```cpp
    DWORD GAME_VERSION = ((0 << 24) | (1 << 16) | (5 << 8) | 2);
    ```

    O cliente que todo mundo chama de "1.5.3" carrega **1.5.2** no campo de versão.
    Deduzir o número a partir do nome da versão foi exatamente o erro — e o teste antigo
    afirmava `0x00010503`, **travando o bug em vez de pegá-lo**. Os códigos de 1.2.6 e
    1.4.8 seguem o mesmo empacotamento mas **não foram conferidos**: não temos aqueles
    clientes.

21. **Cinco opcodes seguem sem correspondência no IR** e o `codec.rs` ainda depende
    deles. Estão isolados em `opcodes::nao_no_ir`, cada um dizendo a que protocolo o
    valor de fato pertence, e um teste impede que a lista cresça. Para a maioria a saída
    não é achar "o opcode certo": **movimento, skills, spawn e status do mundo 3D viajam
    dentro do `GamedataSend` (34) como subcomandos**, não como protocolo GNET. É trabalho
    do desmonte do `gateway.rs`.

22. **Nove pacotes escreviam os campos errados no fio.** Os opcodes conferidos (item 19)
    garantiam que o pacote certo chegasse; o conteúdo dele era outra história:

    | Pacote | O que estava errado |
    | :--- | :--- |
    | `S2CChatBroadcast` | escrevia um `sender_name` **que não existe no protocolo** e omitia `emotion` e `data` |
    | `C2SPlayerChat` | lia um nome de destinatário condicional ao canal 4, também inexistente |
    | `S2CCreateRoleResponse` | mandava 3 campos dos 27: o `RoleInfo` inteiro ficava de fora |
    | `C2SCreateRole` | parava no oitavo campo do `RoleInfo`; o `referid` nunca era lido |
    | `S2CGetFriendListRe` | campo `result` inexistente, `localsid` fora de lugar, uma lista onde o protocolo tem três |
    | `C2SSetUIConfig` | faltava o `localsid`; a configuração era lida dos bytes dele |
    | `C2SSetCustomData` | idem |
    | `C2SHeartbeat` | lia um `i8` onde o protocolo tem três campos de 4 bytes |
    | `C2SACReport` | engolia o payload cru, sem ler o `roleid` nem o prefixo de tamanho |

    O caso do chat mostra por que isso é pior do que parece: com o `sender_name` no
    meio, **tudo depois do primeiro campo saía deslocado**. O cliente lia o `emotion` de
    dentro do `srcroleid`.

    Três protocolos carregam um `RoleInfo`, e o layout estava escrito à mão em cada um —
    foi assim que o `CreateRole_Re` acabou mandando 3 campos. Agora há um
    `write_role_info` só, com um único caminho de escrita (sem personagem, a estrutura
    vai zerada, porque o protocolo não tem campo opcional).

23. **A fórmula do `edition` não era a que estava registrada.** O item 3 dizia "os
    quatro valores derivados dos `.data` do realm". Só **dois** saem de dados
    (`EC_Game.cpp:646` e `cgame/gs/global_manager.cpp:32` montam a mesma string):

    | Valor | Origem | 1.5.3 |
    | :--- | :--- | :--- |
    | `ELEMENTDATA_VERSION` | **constante de compilação** (`CCommon/ExpTypes.h:16`) | `0x3000007f` |
    | `_task_templ_cur_version` | **constante de compilação** (`Task/TaskTempl.cpp:5`) | `121` |
    | `gshop_timestamp` | primeiro `u32` de **`gshop.data`** | do realm |
    | `gshop_timestamp2` | primeiro `u32` de **`gshop1.data`** | do realm |

    Os dois timestamps vêm de **arquivos diferentes**, e o `pw-data-loader` só lia o
    primeiro. São quatro `%x` concatenados, sem separador e **sem preenchimento**; o
    cliente compara com `stricmp`.

    E de novo o cliente é a autoridade: os fontes do servidor definem
    `ELEMENTDATA_VERSION` como **`0x30000080`** (`cgame/gs/template/exptypes.h:16`), um a
    mais que o do cliente. Usar o do servidor produz uma string que não bate.

24. **O `adapter.rs` é uma segunda implementação dos layouts, e é a que o `codec.rs`
    usa** para os pacotes S2C principais. Isso muda a leitura do item 22: o
    `CreateRole_Re` que mandava 3 campos era o de `packets/s2c.rs`, que **não está no
    caminho vivo** — o do adapter já escrevia o `RoleInfo` completo. Os outros oito
    layouts corrigidos estão sim no caminho vivo (os `decode` de C2S e os `encode` que o
    codec chama pela struct).

    Foi o `encode_challenge` do adapter que revelou isso: ele **ignorava o campo
    `edition` da struct** e escrevia sempre vazio, o que tornaria inútil preencher o
    campo do outro lado. Esse método foi removido, o codec passou a usar o `encode` da
    própria struct, e o teste de conformidade agora audita **as duas** implementações
    enquanto a duplicação existir.

25. **Os três adapters de versão sobrescrevem apenas `version()`.** Todo o resto usa a
    implementação padrão do trait, que já ramifica por versão internamente — a mesma
    coisa que o `encode(stream, version)` das structs faz. A hierarquia de adapters não
    carrega nenhuma diferença de versão hoje.

26. **O barramento entre daemons não pode ser exposto ao jogador.** Ele não autentica
    nada: quem alcança a porta manda `EnterWorld` (72) por qualquer `roleid` e passa a
    receber o `S2CGamedataSend` (74) daquele personagem. É infraestrutura interna, e os
    serviços `pw-world-*` do `docker-compose` por isso **não têm `ports:`** — só o nome
    de serviço, na rede interna. `pw-bus/tests/topologia_do_compose.rs` cobra a regra
    contra o arquivo de verdade, porque em comentário ela já seria letra morta.

27. **`localsid` é do `EnterWorld`, e o link precisa guardá-lo.** O servidor de mundo
    endereça a resposta pelo par (`roleid`, `localsid`), mas no logout — e mais ainda numa
    queda de conexão — não há pacote de onde tirar o segundo. Daí o campo em
    `ClientSession`: sem ele, o `PlayerLogout` sai com `localsid` zero e o mundo não
    reconhece a sessão que está sendo encerrada.

28. **Os timestamps do gshop vêm de dois pares de nomes, e o do servidor é o que temos.**
    `CCommon/globaldataman.cpp` do cliente 1.5.3 tem dois caminhos de carga que preenchem
    os **mesmos** globais `global_gshop_timestamp` e `global_gshop_timestamp2`:

    | Empacotamento | `timestamp` | `timestamp2` | Linhas |
    | :--- | :--- | :--- | ---: |
    | cliente | `Data\gshop.data` | `Data\gshop1.data` | 597, 652 |
    | servidor (`_sev`) | `gshopsev.data` | `gshopsev1.data` | 1009, 1038 |

    O carregador só procurava os nomes do **cliente**, e as pastas de realm que temos para
    o 1.5.3 trazem o par do **servidor** — então os dois timestamps ficavam zero, o
    `edition` saía errado e o cliente recusava o login. O documento anterior mandava
    providenciar `gshop.data` e `gshop1.data`, o que teria posto o usuário a procurar
    arquivos que já estavam ali sob outro nome. Corrigido: o carregador aceita os dois
    pares, com quatro testes. Valores reais medidos nas pastas deste projeto:

    | Realm | Arquivos | `timestamp` | `timestamp2` |
    | :--- | :--- | ---: | ---: |
    | `realm_126` | `gshop.data`, `gshop2.data` | 1206433535 | — (o 1.2.6 não manda `edition`) |
    | `realm_153` | `gshopsev.data`, `gshopsev1.data` | 1461564404 | 1452829733 |
    | `realm_148` | **nenhum** | 0 | 0 |

    O `realm_148` vai falhar no login por este motivo até os arquivos aparecerem. E note
    que o `1206433535` do `realm_126` é exatamente a constante que o `gateway.rs` passa ao
    `inst_data_checkout` — o que confirma, por um caminho independente, que aquele campo é
    um timestamp de gshop.

29. **`SelectRole`, `EnterWorld`, `DeleteRole` e `UndoDeleteRole` não checavam dono.** O
    `role_id` vinha do pacote do cliente e ia ao banco com `WHERE id = $1`, sem conta e
    sem realm. Como o `role_id` é sequencial, qualquer jogador autenticado entrava no
    mundo como outro e **apagava personagem alheio** — com um realm só; dois realms apenas
    tornam o vazamento óbvio. Fechado em duas camadas:

    - no repositório, onde não dá para esquecer: `get_details`, `delete_character` e
      `restore_character` exigem `account_id` e `realm_id` na assinatura **e** na cláusula
      `WHERE`; nenhuma variante sem escopo existe;
    - no `dispatch_packet`, uma barreira única — sem conta na sessão, nada que toque dados
      de personagem passa. A lista de isentos é por **inclusão**, então pacote novo nasce
      exigindo login.

    Provado contra um PostgreSQL de verdade em
    `pw-storage/tests/autorizacao_de_personagem.rs`, no cenário de dois realms 1.2.6.
    Com a correção revertida, 4 dos 6 testes falham.

30. **Uma operação recusada não pode responder sucesso.** `DeleteRole` e `UndoDeleteRole`
    mandavam `result: 0` mesmo quando o `UPDATE` não tocava linha nenhuma. Agora o
    repositório devolve `bool` e a resposta reflete isso. Os dois casos — "não existe" e
    "não é seu" — dão a **mesma** resposta de propósito: distingui-los transformaria o
    pacote num oráculo de quais `role_id` existem.

31. **Os codificadores S2C de subcomando nunca tinham sido conferidos contra o IR.** São
    79 funções em `packets/s2c.rs` montando payloads do mundo 3D à mão. A conferência
    achou duas coisas de naturezas diferentes:

    - **Um id errado, e é bug.** `mall_item_price` escrevia `197`, que é
      `REVIVAL_INQUIRE`; o certo é `270`. A dúvida "e se o 1.2.6 numerasse diferente?"
      tem resposta: o pedido correspondente, tratado como `C2S 118 GET_MALL_ITEM_PRICE`,
      **bate exatamente** com o IR — a numeração desta área é a mesma nas duas versões.
      Havia um teste afirmando `197`, isto é, **prendendo o bug**; teste escrito a partir
      do código só confirma o que o código faz.
    - **27 divergências de layout, que não dava para julgar na época.** O IR é do 1.5.3 e
      estes codificadores foram escritos para o 1.2.6. O `self_info_1` mostra que a
      diferença pode ser legítima: o comentário dele diz 34 bytes no 1.2.6, e o IR do 1.5.3
      diz 38, com um `state2` a mais.

      **Treze foram resolvidas depois** — ver os itens 46 e 47. O que destravou o
      julgamento não foi conseguir fontes do 1.2.6, e sim notar que *nenhum* dos 27 trazia
      evidência de 1.2.6 no código (o `self_info_1` traz, e por isso não estava na lista):
      eram palpites, não medições. Sobram 14, todas sem chamador.

    `tests/subcomandos_s2c_contra_o_ir.rs` cobra o id com rigor e **fixa a lista** de
    divergências de layout: uma nova falha, e uma que sumiu também falha (tire da lista).
    A lista só encolhe, e nunca em silêncio. A tabela que liga função a comando é escrita
    **por intenção**, nunca gerada do código — gerá-la do código produziria uma tabela que
    concorda com qualquer bug.

    Consequência prática: **para o realm 1.5.3, esses 27 estão provavelmente errados**, e
    isso é trabalho de Fase 4, não de agora.

32. **O combate do `gateway.rs` era inteiramente fictício.** O `NORMAL_ATTACK` respondia
    dano **35 fixo** e HP do alvo **965/1000 fixo**; o monstro nunca perdia vida e nunca
    morria. Pior: a notificação de abate de missão saía **a cada golpe**, com o id de
    criatura `13641` escrito no código — então qualquer missão de caça completava batendo
    em qualquer coisa. Migrado com o `CombatEngine`, HP debitado de verdade, morte,
    `RECEIVE_EXP`, e abate notificado só na morte com o `template_id` real.

33. **`NORMAL_ATTACK` não carrega id de alvo.** O struct do IR tem 3 bytes: cabeçalho e
    `force_attack`. Quem ataca o quê vem do `SELECT_TARGET` anterior, que o servidor
    guarda. O `gateway.rs` lia 4 bytes a partir do deslocamento 2 como se fossem um `int`
    de alvo, atrás de um `if len() >= 6` — como o pacote tem 3 bytes a guarda nunca
    passava e ele caía no alvo da sessão, funcionando **por acidente**. Um cliente que
    mandasse um pacote maior faria o servidor atacar um id lido de lixo. É também a razão
    de o `SELECT_TARGET` ter precisado migrar antes.

34. **A ordem dos campos do `STOP_MOVE` não é a do `PLAYER_MOVE`.** Nos dois há posição,
    `speed`, `move_mode`, `cmd_seq` e `use_time`, mas no `PLAYER_MOVE` o `use_time` vem
    logo depois das posições e no `STOP_MOVE` vem por último. Copiar um decodificador no
    outro dá um personagem parando com a velocidade errada — e é o tipo de erro que só
    aparece em jogo.

35. **Nada em produção alimentava a tabela de ameaça dos monstros.** O `MonsterAi` e o
    `CombatEngine::calculate_monster_to_player_damage` estavam escritos e testados — mas
    o único lugar que chamava `add_threat` era um teste de unidade. Na prática o monstro
    levava dano e **nunca revidava**: metade do combate era código morto. Resolvido com
    uma linha no tratamento do ataque; bater gera ameaça, ameaça acorda a IA.

36. **O que a simulação decidia não chegava a ninguém.** O tick já debitava o HP do
    jogador atacado por monstro — e o cliente nunca era avisado. O jogador via a vida
    cheia e morria do nada. A causa é estrutural: o `pw-gs` não tinha para onde mandar
    nada. Agora a simulação publica [`EventoDoMundo`] e o `BusServer` traduz em
    subcomandos, o que também é o que mantém o formato de fio fora do `world.rs`.

37. **Duas perdas de dados no repositório de itens.** Apareceram ao migrar os comandos de
    item, e nenhuma delas dá erro:

    - **`upsert_item` não escrevia `extra_data` nem `creator_name`.** A coluna
      `extra_data` guarda os octetos do item — essência de arma, atributos de armadura,
      tudo que vai no `item_info`. Como `swap_slots` e `move_between_containers` fazem o
      item dar a volta por `get` → `delete` → `upsert`, **arrastar um item de um slot para
      outro devolvia ele sem os atributos**. Em silêncio.
    - **As duas operações rodavam sem transação.** A restrição
      `uq_item_slot_per_container` obriga a apagar os dois slots antes de reinserir, o que
      abre uma janela em que nenhum dos dois itens existe. Uma falha ali — rede, processo,
      banco — e o jogador perde os dois para sempre, numa ação que ele faz dezenas de
      vezes por sessão.

    `pw-storage/tests/itens_sobrevivem.rs` tranca as duas contra um PostgreSQL de verdade,
    com um item que tem octetos, refino, pedras, vínculo e criador. Reverter só o primeiro
    conserto derruba 3 dos 4 testes.

38. **`MOVE_IVTR_ITEM` ignora o `amount`.** O comando traz quantos itens da pilha mover, e
    o tratamento — herdado do `gateway.rs` — chama `swap_slots`, que troca as pilhas
    inteiras. Mover 5 de 20 move os 20. **Não corrigido de propósito**: dividir pilha pede
    uma operação nova no repositório, e mudar a semântica pela metade seria pior. O campo
    já é decodificado e registrado no log, para a correção ter por onde começar.

39. **O `match` de subcomandos do `gateway.rs` tinha oito ids errados.** Ele despacha por
    id, e um id errado não dá erro: o servidor só executa o tratador errado para o pedido
    do jogador. A conferência contra o IR achou:

    | id | o código dizia | o IR diz |
    | ---: | :--- | :--- |
    | 32 | `SEVNPC_HELLO` | `TEAM_MEMBER_POS` |
    | 33 | `SEVNPC_SERVE` | `GET_OTHER_EQUIP` |
    | 76 | `LEAVE_SANCTUARY` | `OPEN_BOOTH` |
    | 106 | consulta de saldo | `MALL_SHOPPING` (comprar) |
    | 107 | comprar na loja | `GET_WALLOW_INFO` |
    | 120 | comprar na loja | `CHECK_SECURITY_PASSWD` |
    | 192 | modo de moda | **não existe** |
    | 214–217 | duelo | **não existem** |
    | 218–220 | duelo | comandos de **GM** |

    Dois pontos que valem destaque. **A loja estava invertida**: comprar (106) devolvia
    saldo, e uma consulta de embriaguez (107) disparava uma compra. E o braço de duelo,
    ligado a `214..=220`, engolia três comandos de **GM** — o duelo de verdade é 92/93, e o
    layout do 92 é exatamente o que aquele braço já lia.

    Nenhum é diferença de versão: mais de vinte ids do mesmo `match` batem exatamente com o
    IR. Todos os errados eram, além disso, o id **extra** de um par `A | B` — palpite
    acrescentado a um id certo. O 76 foi **removido** em vez de corrigido: não existe
    `LEAVE_SANCTUARY` na tabela C2S, e responder "você saiu da zona segura" a quem abriu uma
    barraca é pior do que não responder.

    `pw-link/tests/subcomandos_c2s_contra_o_ir.rs` tranca a classe inteira: tabela por
    intenção, conferência de completude nos dois sentidos, e uma regra própria — nenhum
    braço de gameplay pode tratar comando de GM.

40. **A loja de NPC estava invertida.** Os nomes do enum `GP_NPCSEV_*` são do ponto de
    vista **do NPC**, e o `gateway.rs` os lia do ponto de vista do jogador.
    `EC_GPDataType.h` é explícito — `GP_NPCSEV_SELL = 1, // NPC sell to player` — e
    `EC_SendC2SCmds.cpp` confirma por quem envia: a função chamada quando o **jogador
    compra** manda `GP_NPCSEV_SELL`, e a chamada quando ele **vende** manda
    `GP_NPCSEV_BUY`.

    O resultado é que comprar apagava um item do jogador e lhe dava dinheiro, e vender
    cobrava dinheiro e lhe entregava mercadoria.

    Junto vieram os deslocamentos do conteúdo, também errados. O corpo de cada serviço tem
    forma própria, e os dois de loja não são iguais:

    | Serviço | Cabeçalho do conteúdo | Item |
    | :--- | ---: | :--- |
    | compra (`SELL`) | 28 bytes (`money` + cinco campos de contribuição + `item_count`) | `npc_trade_item`, 12 B |
    | venda (`BUY`) | 4 bytes (`item_count`) | `npc_sell_item`, 16 B (tem `price`) |

    O `gateway.rs` lia o id do item no deslocamento 0 do conteúdo — que na compra é o
    `money`. E o `price` do `npc_sell_item` é **ignorado** de propósito: obedecê-lo deixaria
    o jogador escolher quanto ganha por vender.

    Os treze `service_type` em si estavam **certos**, conferidos um a um contra o enum.

41. **O item 40 corrigiu a loja de NPC; o item 39 corrigiu a do Mall.** As duas estavam
    invertidas por motivos diferentes — a do Mall por id trocado, a do NPC por ler o enum
    do lado errado. Vale como aviso: quando um mesmo tipo de erro aparece duas vezes em
    lugares independentes, o próximo lugar parecido merece conferência antes de virar
    sintoma.

42. **A poção era reconhecida por dois ids escritos no código.** O `USE_ITEM` comparava o
    `item_id` com `1796` e `1801` e, quando batia, respondia `self_info_00` com HP/MP
    **120/280 fixos** — sem alterar coisa nenhuma no mundo. Os valores de verdade já
    estavam carregados: `ElementsData::medicines` traz um `MedicineTemplate` por item, com
    `hp_restore` e `mp_restore`, e nunca eram consultados. Agora a cura vem de lá, é
    aplicada à entidade e limitada pelo máximo do personagem.

    Junto: o campo `index` do `USE_ITEM` tem **dois** bytes no IR, e o `gateway.rs` o
    convertia para `u8` logo depois de ler. Qualquer slot acima de 255 era truncado, sem
    erro — o item do fundo da bolsa simplesmente "não funcionava".

43. **O `CAST_SKILL` lia o alvo do deslocamento errado, com dano fixo.** O struct é
    `skill_id` (2), `force_attack` (6), `target_count` (7) e a lista de alvos a partir do
    **8**. O `gateway.rs` lia `data[7..11]`, que começa no `target_count` e engole três
    bytes do primeiro alvo. E o dano era **150 fixo**, mandado por uma tarefa que dormia um
    segundo e respondia sem consultar nada — o monstro não perdia vida.

    Fica uma dívida anotada: falta o **coeficiente da habilidade**. O `elements.data` que
    carregamos não traz a tabela de skills, então por ora uma habilidade causa o mesmo que
    um golpe básico. É menos errado que 150 fixo, e honesto enquanto o número certo não
    estiver disponível.

44. **Um `GAME_VERSION` inválido caía em 1.2.6 em silêncio.** `unwrap_or(V1_2_6)` fazia um
    realm 1.5.3 com erro de digitação subir falando o protocolo errado, e o sintoma
    aparecia como "o cliente conecta e recusa o login". Agora é falha na subida, com o
    nome do realm e os valores aceitos.

45. **O grupo era teatro: quatro comandos respondidos, e nenhum grupo em lugar nenhum.**
    O `gateway.rs` tratava `TEAM_INVITE` (27), `TEAM_AGREE_INVITE` (28),
    `TEAM_REJECT_INVITE` (29) e `TEAM_LEAVE_PARTY` (30) sem guardar estado de grupo. Três
    consequências, cada uma verificada injetando o erro de volta:

    - **O convite ia para quem convidou.** `team_leader_invite(role_id)` era mandado ao
      próprio remetente, então o convidado nunca via caixa nenhuma e o grupo não tinha
      como se formar. Quatro testes caem quando o destinatário volta a ser o remetente.
    - **A lista de membros era escrita no código:** `120, 120, 280, 280` de vida e mana e
      posição fixa, para qualquer personagem. É o mesmo erro do item 37 (a cura do NPC),
      em outro lugar — a segunda aparição do mesmo padrão, que é o motivo de a lista de
      membros hoje sair de `dados_dos_membros`, lendo a entidade real.
    - **Sair do grupo era um eco.** Só o jogador que saía era avisado; os companheiros
      continuavam vendo na interface alguém que já tinha ido embora.

    O estado agora vive no `WorldInstance` (`grupos`, `grupo_de`, `convites`), que é onde
    tem de estar: o daemon de link não conhece vida, nível nem posição de ninguém — pela
    mesma razão que fez o `SELECT_TARGET` mudar de lado no item 2.

    Duas regras que o teatro não tinha:

    - **Aceitar exige convite pendente daquele jogador.** `aceitar_convite(quem, de_quem)`
      devolve `None` quando não há convite de `de_quem`. Sem isso, mandar o comando 28 com
      o id de um estranho entrava no grupo dele. O teste
      `nao_da_para_entrar_num_grupo_sem_convite` cobra exatamente isso.
    - **Sair do mundo é sair do grupo.** `remove_player` chama `sair_do_grupo` e limpa
      convites pendentes; sem isso a lista de membros mostraria um fantasma, e o jogador
      voltaria "em grupo" com um grupo que não existe mais.

    Um grupo que fica com um membro só é desfeito: mantê-lo deixaria o jogador
    permanentemente "em grupo" sozinho, sem conseguir aceitar outro convite.

46. **O cliente descarta em silêncio todo comando cujo tamanho não bate — e nós mandávamos
    treze com o tamanho errado.** Este é o fato que muda mais decisões até agora.

    Em `EC_GameDataPrtc.cpp`, `ProcessGameData` lê **um** subcomando por `GamedataSend`,
    calcula o tamanho esperado com `CalcS2CCmdDataSize` — que devolve `sizeof` da struct
    daquele comando — e então:

    ```c
    ASSERT(dwCmdSize == dwDataSize);
    if (dwCmdSize != dwDataSize) { RuntimeDebugInfo("Invalid %s size(...)"); return; }
    ```

    Um `return`. O comando não é lido pela metade nem desalinha o fluxo: **é jogado fora
    inteiro**, com uma linha num log de depuração que ninguém está olhando. É por isso que
    o sintoma de um layout errado nunca é um erro — é uma funcionalidade que simplesmente
    não acontece.

    ### O que isso revelou sobre a lista de divergências

    A `LAYOUT_DIVERGE` do `subcomandos_s2c_contra_o_ir.rs` tinha 27 nomes e era lida como
    inventário: "cada um é 1.2.6 legítimo ou palpite não verificado, e não temos como
    separar os dois sem fontes do 1.2.6". Com o fato acima, esses 27 nomes deixam de ser
    curiosidade de layout e passam a ser **27 comandos que o cliente 1.5.3 joga fora**.

    E dava para separar os dois casos, por um critério que estava à vista: **a evidência
    no próprio código**. O `self_info_1` — o exemplo que sempre citamos como divergência
    real de versão — traz o comentário "struct no 1.2.6 (34 bytes total)", uma medição.
    Rodando a busca por qualquer menção a 1.2.6 nos 27, o resultado foi **zero**. Não eram
    layouts do 1.2.6: eram palpites. Onde há palpite de um lado e o cabeçalho do cliente do
    outro, o cabeçalho ganha.

    ### Os treze, conferidos um a um no `EC_GPDataType.h`

    | Comando | O que estava errado | Efeito |
    | :--- | :--- | :--- |
    | `NPC_INFO_00` (33) | faltava `iTargetID` (−4) | **nenhuma barra de vida de monstro nunca atualizou** |
    | `PLAYER_CASH` (253) | um `silver_cents` que não existe (+4) | o saldo nunca aparecia |
    | `TEAM_LEADER_INVITE` (57) | faltavam `seq` e `wPickFlag` (−6) | a caixa de convite não abria |
    | `TEAM_JOIN_TEAM` (59) | escrevia `member_id, leader_id`; é `idLeader, wPickFlag` | campo errado **e** tamanho errado |
    | `TEAM_LEAVE_PARTY` (61) | `reason` é `short`, escrevíamos `int` (+2) | ninguém saía do grupo na tela |
    | `HOST_ATTACKRESULT` (24) | faltava `attack_flag`; o `u8` final era chamado de `hit_type` e é `attack_speed` | o número de dano não aparecia |
    | `HOST_SKILL_ATTACK_RESULT` (142) | `attack_flag` é `int`, não `i8`; faltava `section` (−4) | idem, para habilidades |
    | `MOVE_IVTR_ITEM` (45) | `count` é `unsigned int` (−2) | |
    | `MOVE_EQUIP_ITEM` (49) | `amount` é `unsigned int` (−2) | |
    | `EQUIP_ITEM` (48) | as duas contagens são `unsigned int` (−4) | |
    | `ENTER_SANCTUARY` (164) | faltava o `id` (−4) | |
    | `LEAVE_SANCTUARY` (165) | idem | |
    | `PLAYER_ENABLE_FASHION` (192) | faltava `idPlayer` (−4) | e sem ele não diz de quem é a roupa |

    Sobram **14** na lista, todos sem chamador em produção. Quando alguém for usar um
    deles, é a mesma conferência.

    Três comentários no código diziam "struct oficial" ou "no formato oficial 1.2.6" sobre
    layouts que o cabeçalho do cliente desmente. Foram trocados pela citação da struct.

47. **O buraco por onde passou o `team_member_data`: comando de tamanho variável não era
    conferido por ninguém.** A conferência de layout pula qualquer função com laço — contar
    `write_*` dentro de um `for` daria um número sem significado. O efeito colateral era que
    o **cabeçalho fixo** desses comandos, que não tem nada de variável, também não era
    conferido.

    O `TEAM_MEMBER_DATA` (64) caiu aí, e errava tudo menos o id:

    - escrevia `member_count` e ia direto para a lista; o cliente lê `member_count`,
      `data_count` e `idLeader` — **1 byte onde ele conta 6**. E como o `CheckValid` do
      cliente dimensiona o pacote com `sizeof(*this) - sizeof(data) + data_count *
      sizeof(MEMBER)`, faltar o `data_count` não erra por 5 bytes: erra por 5 mais o
      tamanho de todos os membros;
    - cada membro levava a **posição** (12 bytes de `A3DVECTOR3`), que não existe nesta
      struct, e trazia `hp, max_hp, mp, max_mp` onde o cliente lê `hp, mp, max_hp,
      max_mp`. Por coincidência os dois davam 34 bytes por membro.

    O teste que eu tinha escrito na etapa anterior lia de volta **no mesmo deslocamento
    errado em que escrevia**, então concordava com o erro. É a armadilha que o projeto já
    conhecia sob outro nome ("tabela gerada do código concorda com qualquer bug"), aqui na
    forma de um teste que herda a suposição do código que testa.

    Duas coisas mudaram por causa disso:

    - `o_prefixo_fixo_dos_comandos_variaveis_bate_com_o_ir` passou a conferir o prefixo
      fixo contra o IR, que marca o começo da parte variável com `array_len` — o `offset`
      desse campo **é** o tamanho do prefixo. Injetar o cabeçalho antigo produz exatamente
      "escreve 1 bytes antes da lista, o IR diz 6";
    - a lista de membros deixou de ser uma tupla de sete posições e virou a struct
      [`MembroDoGrupo`]. Com sete valores posicionais, trocar `max_hp` com `mp` não
      incomodava ninguém — nem o compilador, que via seis `i32` iguais.

    E o teste ganhou **quatro valores distintos** por membro. A primeira versão usava a
    vida padrão do personagem de teste (100/100/50/50) e por isso **passava com os campos
    trocados** — descoberto injetando a troca de propósito, que é o passo que separa um
    teste que vale de um que só existe.

48. **Dois comandos de estado não são intercambiáveis, e eu tinha usado o errado.** O
    cliente roteia `NPC_INFO_00` (33) para o `MAN_NPC` e `SELF_INFO_00` (38) para o
    `MSG_HST_INFO00`, e `PLAYER_INFO_00` (32) para o `MAN_PLAYER`
    (`EC_GameDataPrtc.cpp`). O caminho de saída do mundo que nasceu na etapa do combate
    mandava a vida do **próprio jogador** como `NPC_INFO_00` com o `roleid` no lugar do id
    do NPC: o cliente ia procurá-lo entre os NPCs e não achava. Mesmo com o tamanho
    corrigido, o aviso morria ali.

    O `SELECT_TARGET` tinha o mesmo problema para alvo que é jogador. Agora os dois
    escolhem o comando pelo tipo do alvo.

49. **Quatro consultas migraram, e todas respondiam número escrito no código — ou nada.**
    São os comandos com que o cliente pergunta ao servidor o estado do que está na tela, e
    são exatamente os que o daemon de link não tem como responder:

    - **`QUERY_NPC_INFO_1` (68)** respondia `1000/1000` para qualquer criatura. Como é uma
      consulta **periódica**, ela desfazia o combate inteiro: o golpe tirava vida de
      verdade no mundo, o `SELECT_TARGET` mostrava o valor certo, e a consulta seguinte
      redesenhava a barra cheia. Somado ao item 46 (o comando era descartado por tamanho),
      o resultado é que a vida de monstro nunca funcionou por dois motivos independentes.
    - **`QUERY_PLAYER_INFO_1` (67)** lia a contagem, escrevia uma linha de log e devolvia
      **sem responder**. Nenhum outro jogador tinha barra de vida, e não havia codificador
      para `PLAYER_INFO_00` — foi escrito agora, a partir do IR.
    - **`GET_EXT_PROP` (21)** respondia `self_info_00(1, sec, 120, 120, 280, 280, 0, 0)`:
      nível 1, vida 120, mana 280, para **qualquer** personagem. Terceira aparição do mesmo
      `120/280` (itens 37 e 45), e sempre pela mesma causa.
    - **`GET_ALL_DATA` (39)** ignorava os três sinalizadores que o comando traz
      (`detail_inv`, `detail_equip`, `detail_task`) e mandava sempre tudo. O servidor
      original os passa adiante: `pImp->SendAllData(gad.detail_inv, gad.detail_equip,
      gad.detail_task)`, em `playercmd.cpp:1863`.

    O `QUERY_CASH_INFO` (110) foi junto, pela mesma razão: respondia `50000` fixo.

    **Suposição anotada:** o corpo do `SendAllData` não está entre as fontes vazadas, então
    o significado exato de cada sinalizador não é verificável daqui. Tratamos zero como
    "não quero esta parte", que é a leitura literal de "Get detail info. flag" no
    cabeçalho do cliente. O `TASK_DATA` (105) vai **sempre**, com ou sem sinalizador: é o
    marcador que dispara o `LoadConfigData` (`EC_HostMsg.cpp:3841`), e sem ele o cliente
    fica esperando.

50. **O dinheiro passou a existir.** `50000` estava escrito em cinco lugares, e um deles —
    a entrada no mundo — tinha o valor certo carregado ao lado, em `details.money`, sem
    nunca ser lido. Agora sai de `PlayerEntity::money` no mundo e de `details.money` no
    link.

51. **A compra na Loja Gold foi removida, não migrada.** O que havia no `gateway.rs`
    gravava o item comprado sempre no **slot 12**, escrito no código, apagando o que
    estivesse lá — a mesma classe de perda de item dos itens 38 e 39 —, com durabilidade
    `10000` fixa, e depois mandava `player_cash(49000)`, de modo que qualquer compra
    deixava o jogador com exatamente esse saldo, comprasse o que comprasse.

    Uma compra de verdade precisa de saldo, preço e slot livre — três coisas que só o
    mundo tem. Enquanto ela não existir, não responder é melhor do que destruir um item e
    inventar um saldo. É o mesmo critério já aplicado ao `OPEN_BOOTH` (76).

52. **`own_ivtr_data` e `own_equip_data` eram código morto que fabricava inventário.** Zero
    chamadores, e o primeiro montava uma bolsa inicial com ids de item escritos no código
    — inclusive os mesmos `1796` e `1801` que o item 43 já tinha flagrado. Apagados.

53. **A separação por versão existe no login e no processo, e *não* existe no mundo 3D.**
    Vale escrever com precisão, porque a resposta é diferente em cada camada.

    ### Onde a separação é real

    | Camada | Como separa |
    | :--- | :--- |
    | **Processo** | cada realm é um par `pw-realm-*` + `pw-world-*` próprio, com `REALM_ID`, `GAME_VERSION`, `CONFIG_DIR` e porta próprios. Um realm 1.2.6 e um 1.5.3 não compartilham processo nem arquivos `data` |
    | **Arquivos de jogo** | `CONFIG_DIR` por realm; o `GameDataManager` carrega o `elements.data`/`npcgen.data`/`gshop*` daquele realm |
    | **Login (GNET)** | `ProtocolAdapter`, com três implementações e **3 pontos de ramificação reais** — `OnlineAnnounce`, e mais dois — todos na forma `!= V1_2_6` |
    | **Regras de conta** | `server_version_code` (o `Challenge`), `challenge_has_edition`, `is_class_supported` |

    ### Onde ela não existe

    **Nos 115 codificadores de gamedata — o protocolo do mundo 3D, que é tudo o que foi
    migrado nas últimas etapas.** Nenhum deles consulta a versão. Os dez `encode` que
    recebem um parâmetro `version` o recebem como **`_version`**, com sublinhado: ignorado.
    Um `TEAM_MEMBER_DATA` sai igual para 1.2.6, 1.4.8 e 1.5.3.

    E o `pw-gs` nem sabia a versão: recebia `GAME_VERSION` do `compose` e só a imprimia no
    log de subida, com o mesmo `unwrap_or_else("1.2.6")` silencioso que o item 44 tinha
    corrigido **apenas no `pw-link`**. Corrigido agora, embora hoje não mude byte nenhum —
    exatamente por isso: no dia em que o primeiro layout depender da versão, este
    `unwrap_or` voltaria a ser um bug silencioso, num lugar onde ninguém procuraria.

    Três predicados de versão são **código morto**: `role_info_fields_count`,
    `has_reincarnation` e `has_meridians`, com zero chamadores.

    ### Por que isso ficou assim, e o que decidir

    Não foi descuido: **não há um segundo layout para o qual ramificar.** O IR é do 1.5.3,
    e não temos cabeçalhos do 1.2.6 nem do 1.4.8. Escrever `if versão == 1.2.6 { ... }`
    hoje significaria escolher os bytes do outro ramo por palpite — que é a prática que o
    item 46 acabou de desfazer em treze comandos.

    Mas a etapa dos tamanhos mudou a natureza do risco, e isso precisa ficar dito: ao
    corrigir treze codificadores para o layout do 1.5.3, **o 1.2.6 passou a receber esses
    mesmos bytes**. Antes eram palpites iguais para todos; agora são layouts verificados
    para uma versão e presumidos para as outras. É uma troca defensável — um layout certo
    para uma versão vale mais que um errado para todas — mas é uma aposta, e a hora de
    revê-la é quando houver com o que comparar.

    **Em andamento:** o Murillo tem um servidor 1.2.6 rodando numa VM, e a opção 1 virou
    o caminho escolhido. O `tools/pw-pcapdiff` foi escrito para isso — ver
    `docs/CAPTURA_DO_126.md`.

    O que resolveria, em ordem de custo:

    1. **Uma captura de tráfego de um servidor 1.2.6 funcionando.** É o que separa
       "diferença de versão" de "nosso erro" sem depender de fontes.
    2. **Cabeçalhos do cliente 1.2.6 ou 1.4.8** (`EC_GPDataType.h` daquelas versões).
       Com eles, o `pw-rpcgen` gera um segundo IR e o teste de conformidade passa a
       comparar os dois — e aí a ramificação por versão tem para onde ramificar.
    3. **Decidir que o alvo é só o 1.5.3** e declarar 1.2.6/1.4.8 como não suportados
       no mundo 3D. É a opção honesta se os artefatos acima não aparecerem.

    Enquanto nada disso acontece, o estado correto é este: **um layout, verificado contra o
    1.5.3, valendo para os três realms**, com a ressalva escrita aqui e no cabeçalho do
    `subcomandos_s2c_contra_o_ir.rs`. O que **não** se deve fazer é criar a ramificação por
    versão antes de ter o segundo layout: seria a mesma teatralidade do grupo antes do item
    45 — estrutura no lugar certo, conteúdo inventado.

54. **O tráfego 1.2.6 É cifrado, e eu tinha afirmado o contrário.** Primeira captura de um
    servidor 1.2.6 real (VM do Murillo, 2026-09-01, 10.754 pacotes, **0 descartados pelo
    kernel**). A captura é tecnicamente perfeita e a remontagem TCP não acusou um buraco
    sequer — e mesmo assim a leitura dos comandos para depois do terceiro quadro.

    ### O erro, e onde ele estava

    Eu tinha escrito, no `CAPTURA_DO_126.md` e aqui, que "o tráfego não é cifrado", com
    dois argumentos: os campos `client_rc4`/`server_rc4` da nossa sessão nunca são
    atribuídos, e um cliente 1.2.6 entra no mundo contra o nosso servidor, que não cifra
    nada.

    O primeiro argumento é sobre **o nosso** servidor e não diz nada sobre o real. O
    segundo é a nossa própria documentação descrevendo a nossa própria implementação —
    exatamente o tipo de evidência autorreferente que este projeto desconfia em todo outro
    contexto, e que eu aceitei aqui porque era conveniente.

    O que a captura mostra: os dois sentidos ficam opacos **imediatamente após o
    `KeyExchange`**. E o `gnsecure.h` do cliente (e o `share/io/security.h` do servidor
    1.5.3, que temos) define:

    ```c
    enum { RANDOM=0, NULLSECURITY=1, ARCFOURSECURITY=2, MD5HASH=3, HMAC_MD5HASH=4,
           COMPRESSARCFOURSECURITY=5, DECOMPRESSARCFOURSECURITY=6, SHA256HASH=7 };
    ```

    ARCFOUR é RC4. O `OnPrtcKeyExchange` do cliente chama `p->Setup(...)`, que devolve
    `oSecurity`/`iSecurity` e só então responde — por isso o **primeiro** quadro do cliente
    depois do `KeyExchange` já sai cifrado. O `Setup` não está entre as fontes vazadas, e
    o `gamesys.conf` do `glinkd` não tem opção de segurança (só `compress = 0`, que já
    estava desligado). Não há como derivar a chave da captura sem engenharia reversa do
    binário.

    ### O caminho que resolve

    O `glinkd` cifra só o elo **com o cliente**. Os daemons conversam entre si por
    loopback, e ali está **em claro**: numa captura de 20s da `lo`, todos os fluxos
    parsearam 100% dos bytes, sem sobra (`92/92`, `15/15`, …). Os subcomandos de gamedata
    nascem no `gs` e atravessam `gs → gdeliveryd → glinkd` antes de serem cifrados.

    Então a medição do mundo 3D se faz **capturando na `lo` dentro da VM**, não na
    interface externa. A captura externa continua valendo para o handshake, que é a parte
    legível dela.

55. **O que a parte legível da captura já provou.** Antes da cifra começar, cada conexão
    troca três quadros em claro. Cinco conexões, e todas concordam.

    **Confirmado (era palpite, virou medição):**

    - **`server_version_code()` do 1.2.6 = `0x00010206`.** Estava marcado "não conferido
      nos fontes" desde sempre. O `Challenge` traz `00 01 02 06` nas cinco conexões, e o
      `gamesys.conf` do servidor declara `version = 10206`. Estava certo.
    - **O `Challenge` do 1.2.6 não tem `edition` nem `exp_rate`.** O payload é exatamente
      `octets nonce(16) + u32 version + i8 algo` = 22 bytes. O `challenge_has_edition()`
      devolvendo `false` para 1.2.6 está certo.
    - **A estrutura do `nonce` do item 4.** `generate_login_challenge()` monta 8 bytes de
      cabeçalho + 8 aleatórios, e a captura confirma: os 8 primeiros bytes são idênticos
      nas cinco conexões e os 8 últimos variam. O valor real é
      `00 00 00 d0 | 00 00 00 00` — ou seja **`Attr = 208`** e `newbie_time = 0`, onde nós
      mandamos zero nos dois. O que o 208 significa ainda não sabemos; que ele existe e é
      constante naquele servidor, sim.
    - **`C2SChallengeResponse` = `octets username + octets(16) hash`.** Confirmado com três
      usuários de tamanhos diferentes (`teste`, `admin`, `mur1ll0`).
    - **`S2CKeyExchange` = `octets(16) nonce + i8 0`.** Bate exatamente com o nosso.

    **Divergência de versão nova, e é a primeira medida:**

    - **`ErrorInfo` (5) tem o código em 1 byte no 1.2.6, e 4 no 1.5.3.** O IR do 1.5.3 diz
      `ErrorInfo { int errcode; Octets info }`. A captura do 1.2.6 traz um quadro de 15
      bytes: `03 0d "Server error."` — 1 byte de código, 1 de comprimento, 13 de texto. Com
      `int errcode` seriam 18. O nosso `S2CErrorInfo::encode` escreve `write_i32`, isto é,
      **3 bytes a mais do que o 1.2.6 espera**.

      Ressalva honesta: com **uma** amostra e código `3`, não dá para distinguir `u8` de
      `CompactUINT` — os dois codificam 3 como `03`. Para separar é preciso um erro com
      código ≥ 64. Fica anotado como "1 byte para valores pequenos", que é o que foi
      medido, e não como um tipo escolhido.

    Esta é a primeira confirmação empírica da hipótese do Murillo: **mesmos recursos,
    protocolos com menos bytes**.

56. **O 1.2.6 ganhou layout próprio, medido.** Uma sessão de 22 minutos com um roteiro de
    45 passos num servidor 1.2.6 real — 67.482 pacotes, **0 descartados pelo kernel**,
    22.217 quadros GNET, 0 buracos — mediu **175 comandos**: 106 idênticos ao 1.5.3 e
    **32 diferentes**. A saída completa está em `docs/MEDIDAS_DO_126.md`.

    As diferenças se agrupam em três famílias, e é a coerência delas que separa medição de
    ruído:

    - **`attack_flag` era `char` e virou `int`.** Cinco comandos independentes com a mesma
      assinatura: −3 nos de ataque comum (24, 26, 120) e −4 nos de habilidade (142, 143,
      144), onde o `section` também falta. `4+4+4+1+1 = 14` fecha exato com o observado.
    - **Um campo no fim que o 1.5.3 acrescentou.** `NPC_INFO_00` e `PLAYER_INFO_00`
      ganharam `iTargetID`; `INST_DATA_CHECKOUT` um quinto carimbo; `ENTER_SANCTUARY` e
      `LEAVE_SANCTUARY` o `id` — no 1.2.6 os dois **não têm payload nenhum**.
    - **Campos de 16 bits que viraram 32.** `RECEIVE_EXP` e as contagens do `EQUIP_ITEM`.

    ### Como cada campo foi identificado

    O tamanho diz que um campo sumiu; só os **valores** dizem qual. Para os nove que estão
    em produção, as amostras da captura fecham a leitura:

    - `NPC_INFO_00`: o `iHP` cai (29 → 22 → 17 → 11 → 2) enquanto o `iMaxHP` fica em 29.
      Isso fixa a ordem **e** mostra que o campo perdido é o último.
    - `RECEIVE_EXP`: sete valores distintos em 36 ocorrências, e um deles decide sozinho —
      `(15, 36)` e `(30, 72)`, exatamente o dobro. Lido como um `int` só, seriam 2.359.311
      e 4.718.622, que não é experiência de um abate no nível 3.
    - `EQUIP_ITEM`: os índices variam (`07 00`, `06 04`, `04 01`) e as contagens alternam
      entre `01 00` e `00 00` — dois `unsigned short`, não dois `unsigned int`.
    - `INST_DATA_CHECKOUT`: `idInst = 1`, `region` e `precinct` **iguais**, e um `gshop` de
      `0x47e8b6ff` — que é o mesmo `1206433535` que o nosso codificador já usava.

    ### O que isso cobrou

    **Três codificadores tinham, antes do item 46, exatamente o layout do 1.2.6**:
    `npc_info_00` (12 bytes), `enter_sanctuary` (sem payload) e `equip_item` (contagens de
    16 bits). Eu os "corrigi" para o 1.5.3 avisando que era uma aposta para o 1.2.6. A
    aposta foi cobrada. O que voltou agora não é o código antigo: é o mesmo layout, desta
    vez **medido**, com a versão escolhendo qual usar.

    Dois outros — `host_attacked` e `player_info_00` — eu escrevi a partir do IR e declarei
    no `subcomandos_s2c_contra_o_ir.rs` que "se divergirem é bug de quem escreveu". Eles
    não divergem do IR: divergem do **1.2.6**. A frase estava certa e mesmo assim
    incompleta, porque só havia um IR.

    ### Onde a ramificação vive

    `crates/pw-protocol/src/por_versao.rs`, no tipo `PorVersao`. A versão fica **numa
    struct**, e não como argumento de cada chamada: um argumento a mais é um argumento que
    se esquece, e esquecer aqui produz um pacote que o cliente descarta em silêncio. O
    `BusServer` carrega um `PorVersao` desde a subida (`GAME_VERSION` do realm), e o
    `gateway.rs` monta o seu a partir do `self.game_version` que já tinha.

    Só entram ali os comandos com diferença **medida**. Um comando que a captura mostrou
    idêntico continua como função de `S2CGamedataSend` — duplicá-lo criaria dois lugares
    para a mesma verdade.

    ### O gabarito é a captura, não o código

    `crates/pw-protocol/tests/layouts_do_126.rs` traz a tabela `MEDIDO` transcrita do
    relatório, com a contagem de ocorrências junto (um comando visto 80 vezes vale mais que
    um visto uma vez). Quatro testes a cobram: o 1.2.6 escreve o tamanho medido, o 1.5.3
    não regrediu, **as duas versões diferem em todos os comandos da tabela** (um comando
    que sai igual nas duas não deveria estar ali), e o 1.4.8 usa o layout do 1.5.3 — que
    não é uma afirmação sobre o 1.4.8, e sim o registro de que **não temos captura dele**.

    ### O que continua em aberto

    Dos 32 medidos, 9 estão em produção e foram resolvidos. Os outros 23 não têm
    codificador nosso ou não têm chamador; quando alguém for escrevê-los, a medida já está
    na tabela. E o `OWN_EXT_PROP` (50), com **−36 bytes** — nove `int` de atributos que o
    1.2.6 não tem —, é o maior deles e vai precisar de atenção própria.

    O `TEAM_MEMBER_DATA` (64) foi resolvido pela aritmética: três tamanhos observados (31,
    56, 81), passo constante de 25, e `31 % 25 = 6`. **Cabeçalho de 6 bytes idêntico ao
    1.5.3** — a correção do item 47 vale para as duas versões — e **membro de 25 bytes**
    contra 34. A decomposição mais provável de −9 é `reincarnation_times(1) + force_id(4) +
    profit_level(4)`, e vale notar que o `has_reincarnation()` do nosso `version.rs` **já
    previa** que 1.2.6 não tem reencarnação: a contagem de bytes concordou com uma previsão
    que o código fez por outro caminho. A implementação disso fica para quando o
    `dados_dos_membros` precisar dos campos que faltam.

57. **O `Response` e o `KeyExchange` trocam de opcode entre as versões — e era isso que
    travava o login do 1.2.6.** No 1.2.6 o cliente manda o login no opcode **2** e o
    servidor responde a troca de chaves no **3**; no 1.5.3 é o contrário, e é o contrário
    que o IR descreve. Com a numeração do 1.5.3 valendo para os três realms, o `Response`
    do cliente 1.2.6 caía no ramo do `KeyExchange`, que escreve uma linha de log e não
    responde nada: o login nunca acontecia, o cliente ficava em "Conectando ao jogo" e a
    conexão morria sem erro em nenhum dos dois lados.

    A medida está em `docs/HANDSHAKE_DO_126.md`, com os três pacotes em claro de um
    servidor 1.2.6 real. O `Challenge` que mandamos foi conferido **byte a byte** com o
    dele. Cinco testes em `crates/pw-protocol/tests/handshake_do_126.rs`, todos validados
    reinjetando o bug.

    De quebra, a mesma captura confirmou que o **`RoleInfo` do 1.2.6 já estava certo** — 19
    campos, mesma ordem, item interno com os dez campos do `GRoleInventory`. Era o outro
    suspeito da falha, e agora está descartado com evidência em vez de opinião.

58. **A cifra do elo com o cliente é opcional na prática.** Pelo `OnPrtcKeyExchange` do
    cliente (`EC_GameSession.cpp:4097`), a cifra só é montada quando o `KeyExchange`
    chega. Como o nosso servidor nunca manda esse pacote, o elo continua em claro dos dois
    lados — que é como o 1.5.3 já funciona. O item 54 continua verdadeiro (o servidor
    **original** cifra); o que mudou é saber que não somos obrigados a acompanhar.

59. **As constantes do `edition` estão dentro dos `.data` do realm.** O cliente recusa
    carregar um `elements.data` cuja primeira palavra não seja o seu `ELEMENTDATA_VERSION`
    (`elementdataman.cpp:3619`) e um `tasks.data` cuja `version` não seja o seu
    `_task_templ_cur_version` (`TaskTemplMan.cpp:1599`). Logo, para um realm cujos dados
    aquele cliente abre, **o número certo está no arquivo**. O `realm_153` do Murillo dá
    `0x30000091` e `124`, que somados aos dois `gshopsev*` reproduzem exatamente o
    `300000917c571db3f456986c25` do `EC.log` dele. Deixou de ser constante de compilação e
    passou a ser leitura de cabeçalho, com o ambiente só como saída de emergência
    (`ELEMENTDATA_VERSION`, `TASK_TEMPL_VERSION`).

60. **Um `?` numa carga de arquivo derrubou o login inteiro.** O `elements.data` de 51 MB
    falha no nosso parser; o `?` daquela leitura abortava o `load_from_directory` inteiro, e
    os dois `gshop` — que vêm **depois** — nunca eram lidos. Resultado: `edition` com dois
    timestamps zerados e login recusado, três camadas longe da causa. A carga agora é
    independente por arquivo e devolve um `RelatorioDeCarga`; o `let _ =` que apagava o
    aviso no `pw-gs` virou log por arquivo. A lição é a de sempre: **o erro não estava onde
    o sintoma apareceu.**

61. **O banco tem as lacunas mapeadas, com migração pronta e testada.**
    `docs/BANCO_DE_DADOS.md` e `specs/02_MIGRACAO_COMPATIBILIDADE_MULTI_REALM.sql`. Três
    famílias: campos que o `RoleInfo` carrega e nós zeramos (oito), valores que o
    repositório devolve chumbados por falta de coluna (`reputation`, `inventory_size`,
    `storehouse_size`) e três protocolos que respondem "ok" e jogam o dado fora
    (`SetUIConfig`, `SetHelpStates`, `SetCustomData`), mais a lista de amigos vazia. As
    quatro restrições novas foram testadas tentando violá-las uma a uma.

62. **`SEVNPC_HELLO` (35) e `TASK_NOTIFY` (49) tinham struct no servidor 1.5.3, mas o IR os
    marca como só cabeçalho — e é por isso que ninguém tinha implementado os dois.** Achado
    ao cruzar um teste de ponta a ponta do Murillo (client 1.2.6 contra o docker-compose,
    2026-09-02) com os logs do `pw-world-126`: `"subcomando 49 de 5 ainda não tratado
    aqui"` e `"subcomando 35 de 5 ainda não tratado aqui"`. É a mesma classe de ponto cego
    do item 47 — comando de tamanho variável (`TASK_NOTIFY`) ou de forma que o extrator
    não reconheceu (`SEVNPC_HELLO`) — só que desta vez o `pw-rpcgen` nem registrou os
    campos, marcou `payload: null`.

    Os dois layouts vêm do servidor (`cgame/common/protocol.h`):
    `service_hello { cmd_header; int id; }` e
    `task_notify { cmd_header; unsigned int size; char buf[0]; }`, com o começo de `buf`
    sendo `task_notify_base { unsigned char reason; unsigned short task; }`
    (`cgame/gs/task/TaskTempl.h`). Os bytes reais da sessão do Murillo **confirmam os
    dois, byte a byte**: o `SEVNPC_HELLO` de 6 bytes carrega o mesmo alvo
    (`50 4c 00 80` = -2147464112) que o `SELECT_TARGET` anterior tinha mandado, e o
    `TASK_NOTIFY` de 9 bytes traz `size=3` seguido de `07 00 00` (`reason=7, task=0`).

    Implementados em `crates/pw-gs/src/comandos.rs` (`SevnpcHello`, `TaskNotify`) e
    `bus_server.rs` (`dizer_ola_ao_npc`, `notificar_tarefa`), com teste que decodifica
    exatamente esses bytes capturados — não dá pra usar a técnica normal de "montar o
    payload a partir do IR" porque o IR não tem campo nenhum aqui.
    `dizer_ola_ao_npc` responde `NPC_GREETING` (70, que já tinha codificador pronto e sem
    chamador) sem checar facção/distância ainda — anotado como TODO, igual ao servidor
    original faz via sessão (`session_say_hello`/`GM_MSG_SERVICE_HELLO`).
    `notificar_tarefa` só decodifica e loga: não existe motor de missões no `pw-gs` (as
    dezenas de `svr_*` de `TaskServer.cpp`/`TaskTempl.inl` continuam sem tratamento),
    então implementar isso de verdade é trabalho futuro maior, não desta rodada.

63. **NPCs de serviço nunca eram spawnados no mundo simulado — só monstros.** Achado no
    mesmo teste: o jogador selecionou um NPC e o log disse
    `"5 selecionou -2147464112, que não está neste mundo"`. `crates/pw-gs/src/world.rs`
    (`init_spawns`) só processava `SpawnType::Monster` do `npcgen.data`; `SpawnType::Npc`
    nunca virava `NpcEntity` — o `HashMap` que existe pra isso (`self.npcs`) nunca
    recebia um `.insert()` em lugar nenhum do crate. O jogador via o NPC na tela (o
    `gateway.rs` manda a lista de entidades ao entrar no mundo por um caminho que não
    depende disto), mas o `pw-gs` não sabia que ele existia — por isso `SELECT_TARGET` e
    o novo `SEVNPC_HELLO` (item 62) não o encontravam.

    Corrigido: `init_spawns` agora também spawna `SpawnType::Npc`, com o mesmo `id`
    (`inst.instance_id`) que o resto do mundo já usa para achar entidades — é o que faz o
    alvo bater com o que o cliente manda. `dados_do_npc` devolve `1/1` de HP (NPC de
    serviço não é atacável; não é o bug antigo de inventar HP de combate, é marcar
    presença). Testado ao vivo: o `pw-world-126` reconstruído carregou **22.426 monstros
    e 1.305 NPCs** do `npcgen.data` do realm 1.2.6 (log de subida, 2026-09-02).

    **Ressalva que sobrou do mesmo teste:** vários `npcgen.data` de subzonas (`b34`, `b35`,
    …) falham ao carregar com `"failed to fill whole buffer"` — mesmo sintoma dos testes
    `pw-data-loader` que já falhavam localmente antes desta rodada. Não investigado ainda;
    aqueles NPCs/monstros continuam ausentes até isso ser resolvido.

    **Ainda não confirmado com o cliente**: os dois itens acima corrigem o que os logs do
    servidor mostravam, mas o Murillo ainda não testou de novo com o client 1.2.6 depois
    do rebuild. Só então dá pra fechar como validado.

64. **O login do 1.5.3 trava ~5ms depois do `Challenge`, sem o servidor processar nada.**
    Teste do Murillo em 2026-09-02, client em
    `F:\Python_C_Projects\PWSource1.5.3\pwclient_153v145\element` contra `pw-realm-153`.
    Cruzando `element/logs/EC.log` com os logs do `pw-realm-153`: o servidor manda o
    `Challenge` e a sessão é finalizada em milissegundos, **sem nenhum `SELECT * FROM
    accounts`** — bem diferente do 1.2.6, que tem toda a sequência de queries de login. O
    cliente confirma que `local ver` e `server ver` (o `edition`) **batem exatamente**
    (`300000917c571db3f456986c25`, valida o item 59), mas no mesmo milissegundo reporta
    `EVENT_DISCONNECT, error code = Active close`.

    **Não investigado até o fundo** — os logs de texto não bastam para dizer se foi o
    cliente ou o servidor que fechou primeiro, nem por quê, e o cliente é russo (sem
    diálogo de erro legível). Como o `pw-realm-153` roda na mesma máquina do Murillo (não
    precisa da VM), uma captura local (`tcpdump` preso ao namespace de rede do container,
    via `docker run --net container:pw-realm-153 nicolaka/netshoot tcpdump ...`) foi
    deixada armada em `_captura_local/pwclient153_login.pcap` para o próximo teste — ver
    memória `pw-ctx-a-153-funcional` da sessão do Claude.

---

## 3. Lacunas conhecidas e divergências entre os dois lados

Nenhuma é falha do parser. Todas foram olhadas uma a uma no C++.

### As 5 divergências de layout

Duas causas distintas, e a distinção importa para quem for implementar:

**(a) O servidor tem o campo comentado no cabeçalho** — o campo existe no fio (o cliente
o lê), só não está declarado em `protocol.h`, com a nota "dependência de cabeçalho":

| id | comando | o que falta no servidor |
| :-- | :--- | :--- |
| 50 | `OWN_EXT_PROP` / `self_get_property` | `// extend_prop prop;` comentado |
| 296 | `PET_PROPERTY` / `pet_property` | `// extend_prop prop;` comentado |

**(b) O cliente tem um campo que o servidor não envia** — diferença real entre as duas
árvores de fonte, com o cliente aparentemente mais novo:

| id | comando | campo extra no cliente |
| :-- | :--- | :--- |
| 38 | `SELF_INFO_00` | `int iMaxAP` no fim |
| 99 | `HOST_OBTAIN_ITEM` | `int expire_date` na segunda posição |
| 160 | `TASK_DELIVER_LEVEL2` | `int id_player` no início |

**Quem manda é o cliente.** O objetivo do projeto é servir o cliente original, então o
layout dele é o que vale; o servidor 1.5.3 dos fontes é referência, não autoridade.

### As 21 divergências de sinal

26 escalares no mesmo deslocamento e do mesmo tamanho, declarados com sinais diferentes
pelos dois lados (`i16`/`u16`, `i8`/`u8`, `bool`/`char`). **Não mudam um byte no fio** —
mudam a interpretação, e `-1` e `65535` são o mesmo par de bytes com significados
opostos. Estão no IR como `sinal-divergente` para que quem escrever o decodificador
escolha conscientemente.

### Cobertura

* **157 dos 194 comandos C2S** têm struct. Dos 36 sem: a maior parte não tem payload —
  das structs do servidor sem dono, **21 são só o cabeçalho de 2 bytes** (`UNSELECT`,
  `SIT_DOWN`, `STAND_UP`, `STOP_FALL`, …). Sobram **10 structs com payload** sem dono,
  quase todas na faixa 209–217 que o cliente abandonou (item 18).
* **305 dos 590 comandos casados** puderam ter o layout comparado; 43 pares foram
  pulados por um dos lados ser de tamanho variável ou ter campo irresolúvel. Comparar
  esses exige modelar as listas e os campos condicionais (item 11), que o IR ainda não
  faz.
* O IR **não** modela os campos condicionais por bits de `state` nem o conteúdo das
  listas de tamanho variável além do tipo do elemento.

---

## 4. Próximo passo

### Prioridade atual (2026-09-02 em diante): 1.5.5 totalmente funcional

Ver a seção "MUDANÇA DE BASE" no topo deste documento para o porquê. Isto vem **antes** de
qualquer item da Fase 2 abaixo — a Fase 2 (migração de gameplay do `gateway.rs` pro `pw-gs`)
continua válida e aplica igualmente ao 1.5.5 (protocolo compatível, ver seção acima), mas não
é o bloqueio atual.

**O bloqueio real, medido, não suposto**: o `GameDataManager` que o `pw-gs` de fato usa em
produção (`crates/pw-data-loader/src/elements.rs::ElementsData`, o `TABLE_SIZES_V7` antigo,
118 tabelas) ainda falha ao carregar a pasta do realm 1.5.5 —
`cargo test -p pw-data-loader test_game_data_manager_directory_load_155` reporta **20 de 30
arquivos com falha** (`elements.data` + 19 `npcgen.data`, um por zona, erro
`failed to fill whole buffer`). Isto é **diferente** do trabalho já concluído: as 231 tabelas
do `elements.data` já foram decodificadas 100% corretamente (`specs/elements_155/README.md`),
mas num leitor **novo e separado** (`crates/pw-data-loader/src/generic_elements.rs::GenericElementsData`),
que ainda não está ligado ao caminho que o `pw-gs` percorre de verdade ao subir (só 1 call
site real hoje, `medicines` em `crates/pw-gs/src/world.rs:514`, usando o `ElementsData` velho
e quebrado).

Ordem combinada com o Murillo:

1. ~~Migrar `pw-gs`/`GameDataManager` do `ElementsData` (quebrado) pro `GenericElementsData`~~
   — **FEITO (2026-09-02)**. `GameDataManager::load_from_directory`
   (`crates/pw-data-loader/src/manager.rs`) tenta `generic_elements::load_elements_data`
   primeiro; só cai pro leitor tipado antigo quando a versão não está no catálogo
   (1.2.6/v7). Novo campo `elements_generic`, novo método
   `GameDataManager::quanto_o_remedio_restaura()` (substitui o acesso direto a
   `elements.medicines` que só existia pro formato tipado). `elements.data` não aparece mais
   na lista de falhas do `GameDataManager` pro 1.5.5.
2. ~~Decodificar `npcgen.data`~~ — **FEITO (2026-09-02)**. Causa raiz achada em
   `cgame/gs/template/npcgendata.h`/`.cpp` (EvolvedPW, o `Load()` original): o registro de
   gerador de spawn (`NPCGENFILEAIGEN`) tem **64 bytes** pra `version >= 11` (ganhou o campo
   `iRefreshLower`), não 60 como o parser assumia (herdado do formato `version < 11`, que é
   o que o 1.2.6 usa) — o `npcgen.data` do 1.5.5 é **`version = 11`**. Cada gerador de spawn
   deslocava o cursor 4 bytes a menos que devia, cascata que arrebentava o resto do arquivo
   inteiro (`failed to fill whole buffer`, era o `world/npcgen.data` + as 19 zonas). Confirmado
   por dois métodos independentes: (a) simulação em Python batendo os 4.273.236 bytes do
   `world/npcgen.data` real **exatamente**, zero sobra; (b) conteúdo real legível dos
   controladores de spawn no fim do arquivo — nomes de evento em chinês coerentes (ex.
   `年兽刷新-改圣诞老人触发怪` = "atualização da Fera do Ano — muda monstro-gatilho do Papai
   Noel", `七夕任务凝露石` = "pedra de orvalho da missão de Qixi"). Corrigido em
   `crates/pw-data-loader/src/npcgen.rs`: o `skip` do registro de gerador agora depende da
   versão (40 ou 44 bytes restantes); objetos dinâmicos (`NPCGENFILEDYNOBJ10`, 24 bytes)
   agora viram `SpawnInstance` de verdade (`SpawnType::DynamicObject`, antes um variant nunca
   construído); controladores (`NPCGENFILECTRL8`, 199 bytes) são consumidos corretamente
   (não viram spawn — são gatilho condicional, gameplay separado, não bloqueiam o essencial).
   Também corrigido um cabeçalho mais antigo (`version < 7`, 2 ou 3 inteiros em vez de 4) que
   travava dois arquivos genuinamente vazios (`a03`/`a04`, `version = 5`, 12 bytes).

   **Resultado: `test_game_data_manager_directory_load_155` passa por completo — zero
   falhas.** Todos os testes de `pw-gs`/`pw-link` continuam verdes, `cargo build --workspace`
   limpo. A ferramenta `D:\PROJETOS\PWPRIVATE\Tools\Editor NPC` (Npcgen Editor By Luka) foi
   inspecionada, mas seu `configs/` é o mesmo catálogo `.cfg` de `elements.data` (bundled pra
   resolver nomes na UI) e seu `offsets.txt` é um encadeamento de ponteiros de **memória de
   processo vivo** (estilo Cheat Engine), não um parser de arquivo — não serviu pra decodificar
   o formato do `npcgen.data`; os fontes do EvolvedPW resolveram sozinhos.

   **O que ainda falta pro 1.2.6** (não bloqueia o 1.5.5): 9 zonas (`a02`, `a13`, `a26`,
   `b30`–`b35`) têm `npcgen.data` genuinamente `version` 5 ou 6 **com conteúdo real** — o
   formato de área pré-v7 (`NPCGENFILEAREA`, sem `idCtrl`/`iLifeTime`/`iMaxNum`, 59 bytes em
   vez de 71) não foi implementado por falta de um arquivo real não-vazio pra confirmar
   contra (o parser recusa com uma mensagem clara em vez de arriscar ler errado). Fica pra
   quando a prioridade voltar a ser o 1.2.6.
3. ~~Subir `pw-realm-155`/`pw-world-155` no `docker-compose`~~ — **FEITO (2026-09-02)**.
   Adicionados ao `docker/docker-compose.yml` (porta pública 29003) espelhando o par
   1.5.3, e uma linha nova em `realms` no Postgres (`specs/01_DATABASE_SCHEMA_POSTGRES.sql`
   + aplicado direto no banco já rodando, já que o seed só roda em volume novo). Build e
   subida confirmados por log real, os dois lados:
   - `pw-world-155`: **"Templates de dados e mapas carregados: 50 arquivo(s) carregado(s)"**
     (zero falhas) e **"World #1 inicializado com 24899 monstros e 6918 NPCs ativos"** — a
     prova final de que os itens 1 e 2 acima resolveram o bloqueio de verdade.
   - `pw-realm-155`: mesma carga sem falha, leu `ELEMENTDATA_VERSION=0x3000009c` e
     `task_templ=128` do próprio `elements.data`/`tasks.data` deste realm (não de constante
     fixa), conectou no barramento e está **escutando na porta 29003** para o cliente 1.5.5
     (`server_code: 66819`, decodificado de `GAME_VERSION = 1.5.5` — ver `version.rs`).
4. **Testado com o client 1.5.5 de verdade — achou um bloqueio real, não trivial.** Login
   recusado com **"Your current client program version is low. Please exit and update"**.

   **Causa raiz, achada nos fontes** (`EC_GameSession.cpp::OnPrtcChallenge`, EvolvedPW): o
   client compara a string `edition` que o servidor manda no `Challenge` contra a que ele
   mesmo calcula dos seus próprios arquivos locais (`elements.data`+`tasks.data`+`gshop*`) —
   com `stricmp`, **igualdade exata**, não uma comparação numérica. Qualquer divergência
   mostra essa mensagem e desconecta, **mesmo que o servidor seja mais novo**, apesar do
   texto. `EC.log` do client mostrou:
   ```
   local  ver: 3000009f8157918eac576243735721a4ef   <- build v159 (client 1.5.5.EN)
   server ver: 3000009c8057b41d0155dae66856ce6bc7   <- build v156 (pacote pwserver_155v156)
   ```
   O pacote de servidor da comunidade que tínhamos (`pwserver_155v156`) é uma build
   **vizinha, não idêntica**, à do client 1.5.5.EN que o Murillo instalou — não tem meio
   termo, o client não baixa nada do servidor nessa etapa, só compara.

   **Resolvido copiando os arquivos do client (build v159) pro servidor e decodificando essa
   build do zero** (2026-09-02): o Murillo copiou `elements.data`/`tasks.data` do client
   (`F:\PW\1.5.5\1.5.5.EN\...\element\Data\`) pra `data/realm_155/config/`. Isso por si só
   **quebrou de novo** o carregamento (`elements.data: failed to fill whole buffer`) — o
   catálogo `specs/elements_layouts/` só cobria v156. Decodificado v159 na mesma sessão:
   - `.cfg` da build (`PW_1.5.5_v159.cfg`, do mesmo editor ADMVAL que já tínhamos) copiado
     pra `specs/elements_155/`, `specs/elements_layouts/generate_v159.py` gerou
     `v159.json` (**234 tabelas** — 3 a mais que v156, todas no fim do arquivo, índices
     0-230 idênticos em nome/ordem entre as duas builds).
   - Caminhada tabela a tabela achou **11 correções de `skip`/`count`** necessárias
     (`specs/elements_155/realm_155_v159_overrides.json`), a maioria confirmada com
     100% dos campos batendo contra os exports do ADMVAL **já salvos de sessões
     anteriores** — que por acaso já eram exports desse mesmo arquivo (o client 1.5.5.EN).
     Cobertura: consome 54.397.023 de 55.170.911 bytes (~98,6%) — as tabelas ~216–230
     (sistema de "lar/mansão" e bilhetes de loteria, features tardias) ainda não resolvidas,
     documentado como pendência, não bloqueia login/mundo/combate básico.
   - `crates/pw-data-loader/src/generic_elements.rs`: catálogo agora cobre 156 **e** 159;
     nova função `load_overrides_for_version()` escolhe os overrides certos pela versão
     **detectada do arquivo**, nunca aplicando override de uma build à outra (bug que
     existiria se eu só tivesse trocado o JSON sem essa seleção). `GameDataManager` usa
     `load_elements_data_auto()`, que já detecta+escolhe sozinho.
   - `test_game_data_manager_directory_load_155` volta a passar (zero falhas) com os
     arquivos v159 agora na pasta do realm.
   - Containers `pw-realm-155`/`pw-world-155` reconstruídos com o código novo — login a
     testar de novo.

   Ver `specs/elements_155/realm_155_v159_overrides.json` (evidência por tabela) e
   `specs/elements_155/walk_report_v159.txt` (caminhada completa das 234 tabelas).

6. **Login funcionou (2026-09-02/03), depois crash ao entrar no mundo — cadeia de 4
   causas distintas, todas encontradas por medição, não suposição:**

   a. **`models.pck`/`litmodels.pck` truncados em exatamente 2³¹−256 bytes** (marca de
      copiador de 32 bits) — o mapa principal (`maps\world\world.ecwld`) não existia na
      instalação. **Tentativa de reconstrução (concatenar `.pck`+`.pkx`) revelou um
      limite real do próprio engine**: `AFilePackGame::InnerOpen` usa `int nOffset`
      (32 bits **assinado**) pra achar o rodapé do pacote — um arquivo de pacote acima de
      ~2GB estoura essa conta e o cliente recusa com "Incorrect version!". Confirmado
      por teste direto (com e sem a reconstrução, mesmo arquivo de outra forma idêntico).
      **Não reconstruir esses dois arquivos** — usar sempre a forma truncada original.
   b. **133 arquivos genuinamente faltando em `maps\world\`** (`world.ecwld`,
      `world.ecbsd`, `precinct.sev`, `region.sev`, ~130 blocos de terreno
      `world_N.t2bk`/`.t2mk`) — não é truncamento, a pasta mesmo estava incompleta.
      Resolvido copiando de uma segunda distribuição do client que o Murillo tinha
      (`F:\PW\1.5.5\bin`) — confirmado que essa segunda distribuição tem o **mesmo**
      corte de `models.pck` em 2³¹−256 bytes (prova independente de que é convenção de
      empacotamento da comunidade, não um acidente de cópia).
   c. **`gshop_time_stamp2` sempre igual a `gshop_time_stamp`** —
      `S2CGamedataSend::inst_data_checkout` (`pw-protocol/src/packets/s2c.rs`) escrevia
      o mesmo parâmetro `gshop_ts` duas vezes no fio, em vez de um segundo valor real.
      Isso já estava **documentado como questão em aberto** desde a implementação do
      sexto campo (2026-09-02) — a evidência de crash real converteu de "acompanhar" pra
      "corrigir". Corrigido: a função ganhou o parâmetro `gshop_ts2`; `PorVersao::inst_data_checkout`
      idem; `gateway.rs` agora manda `self.data_manager.gshop.timestamp` e
      `self.data_manager.gshop2.timestamp` (valores reais, já carregados certos desde a
      correção do item 5) em vez de dois hardcoded iguais.
   d. **`region`/`precinct` do `INST_DATA_CHECKOUT` eram números fixos inventados**
      (`2097199`/`0x20002f`) em vez do valor real. O valor certo — `1442234257`
      (`0x55f6bf91`) — está embutido nos dois arquivos `region.clt`/`precinct.clt` do
      client (confirmado por leitura direta) **e** nos equivalentes server-side
      `world/region.sev`/`precinct.sev`, que já existiam em `data/realm_155/config/`.
      Formato dos `.sev` confirmado em `cgame/gs/template/el_region.h`/`el_precinct.h`
      (EvolvedPWServer): `REGIONFILEHEADER4` (`dwVersion, iNumRegion, iNumTrans,
      dwTimeStamp` @offset 12) e `PRECINCTFILEHEADER5` (`dwVersion, iNumPrecinct,
      dwTimeStamp` @offset 8). `GameDataManager` ganhou `region_timestamps`/
      `precinct_timestamps` (`HashMap<i32, u32>`, por `world_id`, carregados junto com
      `npcgen.data`/`collision.clt` de cada pasta de mapa); `gateway.rs` usa esses
      valores reais em vez dos números inventados, com aviso no log se algum realm não
      tiver o `.sev` correspondente.

   Testes novos: `test_inst_data_checkout_gshop_e_gshop2_sao_valores_diferentes`
   (`pw-protocol/tests/protocol_tests.rs`, trava a regressão do item c). Suíte inteira
   de `pw-protocol`/`pw-link`/`pw-gs`/`pw-data-loader` verde depois da correção (as 2
   falhas que sobram são o problema pré-existente e não-relacionado do 1.2.6 via
   `files1.2.6/...`, já documentado acima).

   **Containers reconstruídos com as correções (c) e (d) — login e entrada no mundo a
   testar de novo.**

7. **Crash ao entrar no mundo, agora depois de (c)/(d) — achado no mesmo lugar em DOIS
   clients diferentes (2026-09-03).** Testado com o client EN (v159) e um segundo realm
   novo, `realm_155BR`, com um client BR (build v156, diferente) — **os dois crasharam no
   mesmo ponto exato**, o que aponta pro servidor, não pro client: `CECGameRun::LoadConfigsFromServer,
   data read error (2)` → `glb_HandleException` → `Exception occurred in render thread...
   mini dumped!`.

   **Causa raiz**: `OnPrtcGetConfigRe` (o cliente recebendo `GetUIConfig_Re`, opcode 105)
   passa `ui_config` inteiro pra `LoadConfigsFromServer` sem tirar nada antes. Nosso
   `S2CGetUIConfigRe::new` (`pw-protocol/src/packets/s2c.rs`) **sempre** escrevia um
   cabeçalho sintético de 16 bytes (`idInst`/`precinct_ts`/`domain_ts`/`gshop_ts`,
   validado contra o `elementclient.exe` 1.2.6 numa sessão anterior) **mesmo quando não
   havia dado real nenhum depois** — e os dois únicos lugares que chamam essa função
   (`gateway.rs`, ao entrar no mundo e ao responder um pedido do cliente) sempre mandam
   `&[]`. O cliente lia os 4 primeiros bytes desse cabeçalho (`idInst = 1`) como se fosse
   o tamanho da próxima seção de configuração, tentava ler 1 byte de config dali, e
   lançava a exceção que derruba o processo de renderização.

   O próprio cliente já tem um caminho limpo pra "sem configuração ainda": `ui_config`
   **totalmente vazio** (`if (!pDataBuf || !iDataSize) { log(...); return false; }`,
   cai pra `ApplyUserSetting()`, sem exceção). Corrigido: o cabeçalho só é escrito quando
   há dado real pra vir depois; como nenhum chamador manda dado real hoje, isso não muda
   nada de comportamento pra quem já funcionava, só evita mandar um cabeçalho pela
   metade. Containers reconstruídos — login e entrada no mundo a testar de novo, nos
   dois realms/clients.

8. **`realm_155BR` criado (2026-09-03)**: realm gêmeo do `realm_155`, porta pública
   **29004**, mesma `data/realm_155/config` como base (npcgen, mapas, `region.sev`/
   `precinct.sev`) mas com `elements.data`/`tasks.data`/`gshop*.data` do client
   "1.5.5 BR" do Murillo (`F:\PW\1.5.5\1.5.5 BR\...`, build **v156** — diferente do EN,
   que é v159). Carrega limpo com o catálogo v156 que já tínhamos (231 tabelas,
   conteúdo real em português confirmado: "Guerreiro", "Lâmina das Cem Batalhas") — não
   precisou de nova arqueologia. Serve pra isolar bug de client vs. bug de servidor: os
   dois realms compartilham o mesmo binário `pw-gs`/`pw-link`, só os dados de referência
   mudam. Linha nova em `realms` no Postgres e em
   `specs/01_DATABASE_SCHEMA_POSTGRES.sql`.

9. **Sessão 2026-09-03 (continuação, tarde/noite): sete causas de crash resolvidas em
   cadeia, uma ainda aberta.** Depois do item 7 (cabeçalho sintético do `GetUIConfig_Re`),
   o cliente parou de crashar ali e cada teste seguinte revelou a **próxima** causa —
   sempre achada por medição (log do client + log do realm cruzados, minidump quando
   precisou), nunca palpite. Nesta ordem:

   a. **`SELF_INFO_1` sem o campo `state2` — 30s de "efetuando login" e desconexão.**
      `S2CGamedataSend::self_info_1` escrevia os 34 bytes do `cmd_self_info_1` do
      **1.2.6**; o 1.5.5 tem 38, com um `state2` (int) a mais no fim
      (`EC_GPDataType.h`, `cmd_self_info_1::CheckValid`). Tamanho errado ⇒
      `CalcS2CCmdDataSize` cai no ramo "unknown command" ⇒ o `case SELF_INFO_1` que
      cancela o timeout `OT_ENTERGAME` nunca roda ⇒ 30s depois o client se desconecta
      sozinho ("EnterWorld Overtime"). Corrigido com o mesmo padrão do
      `INST_DATA_CHECKOUT` (item 23/6c): `PorVersao::self_info_1` escreve o `state2`
      extra só quando a versão é exatamente `V1_5_5` — sem evidência equivalente pro
      1.5.3, esse continua com os 34 bytes de sempre. Teste:
      `test_self_info_1_155_ganha_o_state2`.

   b. **Loop infinito de `ACTIVATE_REGION_WAYPOINTS` — tela de "Entrando em Perfect
      World" travada pra sempre.** Com (a) corrigido, o client parava de crashar mas
      nunca destravava a tela de carregamento. Log do realm mostrou o client mandando
      o mesmo pacote **~166 vezes por segundo**: `ACTIVATE_REGION_WAYPOINTS` (C2S 178).
      Causa em `EC_World.cpp` (client): a cada quadro, o client compara os waypoints da
      região atual contra a lista que o servidor **já confirmou** via `WAYPOINT_LIST`
      (S2C 180) — como nunca mandávamos esse comando, a lista ficava sempre vazia, todo
      waypoint parecia "novo" em todo quadro, e o client reenviava pra sempre.
      Corrigido: novo `S2CGamedataSend::player_waypoint_list`; `gateway.rs` responde ao
      C2S 178 devolvendo os mesmos IDs que o client acabou de ativar. Teste:
      `test_player_waypoint_list_devolve_os_ids_recebidos`.

   c. **`lua_version = 0` — "exit process because wrong config data".** Com (b)
      corrigido, a tela de loading fechava e o mapa aparecia por uma fração de
      segundo — e o processo saía sozinho. Causa em `EC_GameDataPrtc.cpp`, `case
      SERVER_TIME`: o client compara o `lua_version` do pacote `SERVER_TIME` (114)
      contra a primeira linha do seu `global_api.lua` local (`--<N>`); se não bater,
      seta `FATAL_ERROR_WRONG_CONFIGDATA` e o loop principal mata o processo no
      próximo tick. Mandávamos `lua_version = 0` "por falta de evidência". Evidência
      achada: três cópias independentes de `global_api.lua` concordando em `--102`
      (a nossa própria em `data/realm_155/config/`, e duas pastas de servidor de
      referência que o Murillo tinha: `F:\PW\1.5.5\home155\gamed\config` e
      `F:\PW\1.5.5\pwserver_155v156\home\pwserver\gamed\config`). Corrigido:
      `S2CGamedataSend::server_time` ganhou o parâmetro `lua_version`, `gateway.rs`
      manda `102`. Teste: `test_server_time_leva_o_lua_version_certo`.

   d. **`GetUIConfig_Re` nunca chegava a ser respondido — client preso na tela de
      loading pra sempre, de novo, mas sem nenhum erro.** Log com `info!` em cada
      ponta (achado pedido pelo Murillo) mostrou: `TASK_DATA` sai do servidor, mas o
      pedido `GetUIConfig` (opcode 104) que o client **deveria** mandar sozinho ao
      processar TASK_DATA (`EC_HostMsg.cpp::OnMsgHstTaskData` → `LoadConfigData()`)
      **nunca chegava**. Sem essa troca, `OnPrtcGetConfigRe` nunca roda,
      `EnableUI(true)` nunca roda, a tela `Win_EnterWait` ("Entrando em Perfect
      World") nunca fecha. **Corrigido reintroduzindo o envio proativo de
      `GetUIConfig_Re`** (que tínhamos removido no item 7 por causa do double-send)
      — mas agora protegido por uma flag por sessão
      (`ClientSession::ui_config_enviado`), mandando **no máximo uma vez** por
      personagem: cobre tanto "o client nunca pede sozinho" (o caso de sempre nos
      testes) quanto "o client pede depois", sem repetir o crash do double-send.

   e. **`LogicHelp::LogicCheck`/Arc SDK — crash de volta, agora sem double-send.**
      Com (d) corrigido, a tela de loading fechava, o mapa aparecia — e o processo
      crashava ~300ms depois (`glb_HandleException` → exceção na render thread).
      Opcode 0x25 (`PROTOCOL_COLLECTCLIENTMACHINEINFO`, 37) aparecia no log do realm
      logo antes. Esse pacote só sai de `OnPrtcGetConfigRe`, dentro de `#ifdef
      LOGIC_CHECK`, chamando `LogicHelp::LogicCheck` — componente ligado ao Arc SDK,
      que bypassamos há várias sessões (`arcsdk.dll`/`arcinstall.exe` renomeados,
      `startbypatcher`). Sem o Arc inicializado, essa chamada quebra o processo.
      **Corrigido só no lado do client**: `logiccheck:0` na linha de comando
      (`CECCommandLine::GetEnableLogicCheckInfo`, syntax `chave:valor` igual
      `game:cpw`) desliga essa checagem explicitamente — não depende do `.ini`
      empacotado. Já aplicado no `elementclient.bat` do client EN; o BR precisa do
      mesmo argumento manual (sem `.bat` encontrado pra ele).

   f. **`models.pck`/`litmodels.pck` nunca abrem — bug real de 32 bits no engine, não
      arquivo faltando.** Com (e) desligado, o crash continuou, **exatamente no
      mesmo endereço** de instrução em todo teste seguinte (`0x58B471` no build 2575
      EN, `0x576661` no build 2569 BR — confirmado via minidump, ver função
      `parse_minidump.py`/`dump_crash_context.py` no scratchpad da sessão, não
      versionadas). Log do client mostrava, bem antes do crash: `Failed to open ecm
      file Models\Weapons\...\木剑\木剑.ecm` ("Espada de Madeira") — **mesmo pra
      personagens equipados com outra arma**, provando que é um modelo-base
      carregado incondicionalmente pelo rig de qualquer personagem, não a arma
      específica de quem está logado.

      Causa raiz, achada lendo `AngelicaFile/Source/AFilePackGame.cpp`
      (`AFilePackGame::InnerOpen`, ~linha 155-161): a posição do índice do pacote é
      guardada num `int nOffset` — **32 bits com sinal**. `models.pck`+`models.pkx`
      somados (o engine já suporta nativamente pacotes partidos em dois arquivos
      quando passam de `MAX_FILE_PACKAGE = 0x7fffff00`) somam ~3,22GB — passa de
      `INT32_MAX`, o cast estoura pra negativo, os `seek()` seguintes vão pra
      posição errada, e **o pacote inteiro nunca abre** — não importa onde a entrada
      específica estivesse fisicamente armazenada (confirmado: a "Espada de Madeira"
      estava bem dentro dos primeiros 2GB, legível). Isso não é specific desta
      cópia: o client 1.7.2 do Murillo (repack diferente) tem o mesmo corte de
      `models.pck`/`litmodels.pck` em exatamente `2³¹−256` bytes, mais um
      `models2.pck`/`models2.pkx` que o 1.5.5 não tem — sinal de que é uma
      característica conhecida desses repacks de conteúdo pós-1.5.3 (catálogo de
      modelo cresceu além de 2GB), não um acidente de uma cópia.

      **Sem como recompilar o client** (binário fechado), a saída foi ler os
      pacotes por fora, sem o bug de 32 bits, e colocar os arquivos como **soltos**
      no diretório do client — o `AFile` sempre procura em disco antes do pacote
      (mesmo princípio já usado pro `element_client.cfg`). Ferramenta nova,
      documentada e reaproveitável: **`tools/pw-pck-extract/`**
      (`pw_pck_extract.py` + `README.md`) — leitor completo do formato
      `AFilePackage`/`.pck`+`.pkx` (`SAFEFILEHEADER`, `FILEHEADER`, `FILEENTRY_INFILE`,
      máscaras XOR, zlib), com os structs **recalibrados por inspeção binária**
      porque o source diverge do binário instalado de novo (`FILEHEADER` real = 272
      bytes, não 280; `FILEENTRY_INFILE` real = 276 bytes com offset de 32 bits, não
      288 com 64 — ver o cabeçalho do script e o README pro detalhe completo).
      Validado contra `configs.pck` (91 arquivos, extração conferida byte a byte)
      antes de confiar nele pro pacote de 3,2GB.

      **Extração completa aplicada nos dois clients**: `models.pck`+`.pkx` e
      `litmodels.pck`+`.pkx` de EN e BR, mais `interfaces.pck` dos dois (esse não
      tem o bug de 2GB, extraído por descarte de hipótese). Total: EN 133.264
      (models) + 114.511 (litmodels) + 1.700 (interfaces); BR 139.330/139.331
      (models, 1 entrada genuinamente corrompida na origem) + 113.859/113.860
      (litmodels, idem) + 2.640 (interfaces). **Resultado: as mensagens "Failed to
      open ecm file" sumiram do log do client EN — mas o crash continuou,
      idêntico**, provando que os modelos faltando eram um problema real e paralelo,
      não a causa deste crash específico.

   g. **Crash ainda aberto — não é GM, não é modelo, não é `interfaces.pck`.**
      Testado com conta GM (`admin`) e sem GM (`testuser`), nos dois clients, depois
      de (f): **mesmo crash, mesmo endereço, mesmos registradores**
      (`EAX=EBX=ECX=EDI=0`, `ESI=EDX=0x78C4` **constante em todo teste**,
      independente de conta/personagem/realm/build). Descarta de vez a hipótese
      antiga de `if (pHost->IsGM()) { pDlgCountryMap->GetConfig(); }`
      (`EC_GameSession.cpp:5542`, um null-deref real no código mas que não é este).

      Disassembly manual do trecho de código capturado no minidump (só 256 bytes ao
      redor do `EIP`, o resto da função não veio no dump):

      ```
      CALL <alguma função>          ; devolve um ponteiro em EAX
      MOV EAX, [EAX+0x151C]         ; lê um campo desse objeto — vem NULO
      PUSH ESI                      ; empilha 0x78C4 como argumento
      MOV ECX, EAX
      MOV EDX, [EAX]                ; <- crash: EAX=0
      CALL [EDX+0x20]               ; chamaria objeto->método_virtual(0x78C4, ...)
      ```

      É uma chamada de método virtual C++ num ponteiro nulo — algum
      "gerenciador"/objeto tem um campo (offset `0x151C`) que deveria apontar pra um
      sub-objeto responsável pelo recurso de ID `30916` (`0x78C4`, quase certamente
      um ID de string/diálogo/tema de UI, hardcoded no client, não dado de jogador —
      é o mesmo valor em toda conta/personagem/build testada) e esse sub-objeto
      nunca foi inicializado. Sem PDB nem desmontador completo (o minidump não trouxe
      o resto da função nem os endereços de retorno na pilha), não deu pra ir além
      disso sem ferramenta melhor (IDA/Ghidra com o binário completo).

      **Estado no fim desta sessão**: o Murillo está avaliando baixar um instalador
      de conteúdo mais completo/diferente (achou duas opções com numeração de
      `elements.data`/`tasks.data` bem mais alta que a nossa — `v216`/`146` e
      `v181`/`135`, contra o nosso `v159`/`129` no EN e `v156`/`129` no BR — **não é
      a mesma numeração que usamos** internamente, então não dá pra saber de cara se
      é compatível/mais completo só pelo nome). Próximo passo natural: testar esse
      client, e se o crash sumir ou mudar de endereço, comparar os dois binários
      pra achar a diferença; se ficar idêntico (`0x78C4`), é sinal de que é um
      problema estrutural do conteúdo 1.5.5 dessa família de repacks, não desta
      cópia específica.

5. **Testado de novo — passou do erro de versão, achou um segundo bloqueio, também
   resolvido (2026-09-03).** Com `elements.data`/`tasks.data` batendo, o login progrediu:
   `local ver` e `server ver` agora **idênticos** no `EC.log`. Mas a conexão ainda caía,
   agora com **"Server maintenance in progress, please login later"**, antes até do
   cliente mandar usuário/senha (`pw-auth` e o `dispatch_packet` do `pw-link` não viam
   nada chegar).

   **Causa raiz**: `EC_GameSession.cpp::OnPrtcChallenge` tem DOIS testes de versão no
   mesmo `if`, um deles não sobre a `edition` (string), e sim sobre `p->version`, o
   `GAME_VERSION` numérico — comparação **exata**, não só "não pode ser mais velho". A
   mensagem muda conforme o sentido do erro: servidor mais **novo** que o cliente ⇒
   "versão baixa" (o que já tínhamos corrigido); servidor mais **velho** ⇒
   `FIXMSG_SERVERUPDATE`, que nesta tradução vira "Server maintenance in progress" — nome
   enganoso, não tem nada a ver com estado real do servidor.

   `crates/pw-protocol/src/version.rs` mandava `0x00010503` pro 1.5.5, valor de
   `EvolvedPWClient/EC_Game.cpp:116` — mas esse arquivo-fonte é de uma build **2457**
   (`GAME_BUILD`, linha 117), e o `EC.log` do client instalado mostra **"Build version
   2575"**. A árvore de fontes que temos ficou pra trás da build real instalada.

   **Resolvido por inspeção direta do `elementclient.exe`** (não dedução): `0x00010503`
   não ocorre nenhuma vez no binário; `0x00010505` ocorre 4 vezes, uma delas logo depois
   da string `"(0x%x) != local(0x%x)"` (a própria mensagem de comparação de versão) e
   seguida, no mesmo bloco, pelo DWORD `2575` — o build number exato do `EC.log`. Duas
   evidências independentes convergindo. Corrigido em `version.rs`
   (`GameVersion::V1_5_5::server_version_code() = 0x0001_0505`), testes de
   `pw-protocol`/`pw-link`/`pw-gs` continuam 100% verdes (nenhum hardcoded o valor
   antigo), containers reconstruídos.

   **Lição nova pro princípio já conhecido do projeto** ("nunca deduzir de nome de
   versão"): não basta ter os fontes certos — os fontes também podem estar **atrás** do
   binário real. Quando um valor lido do código-fonte não bate, comparar contra o
   binário instalado é a fonte de verdade final, não o código.

   Login a testar de novo depois deste rebuild.
5. Só depois disso: os **66 protocolos GNET / 17 C2S / 24 S2C novos do 1.5.5** (ainda não
   triados entre "essencial pro básico" e "feature que pode esperar", tipo arena), e os
   parsers de `tasks.data`/`gshop*.data` (menor prioridade — não bloqueiam login nem mundo
   básico; `tasks.data` tem registros de tamanho variável, precisa de parser próprio, não
   reaproveita a técnica de `elements.data` direto).

10. **Sessão 2026-09-03 (noite): o "client novo" do `E:\0_GAMES\Perfect World` é
   bit-a-bit o mesmo repack do BR — não serve de controle de conteúdo, só de instalação
   limpa.** O Murillo instalou um terceiro client 1.5.5 esperando um pacote de conteúdo
   diferente, pra usar como controle contra o crash de render. Conferido por MD5, **não é**:

   | arquivo | client novo (E:) | client BR (F:) | client EN (F:) |
   | :--- | :--- | :--- | :--- |
   | `ELEMENTCLIENT.EXE` | `7476daec…` | `7476daec…` (igual) | (build 2575, difere) |
   | `configs.pck` / `interfaces.pck` | `8286a377…` / `9d3b8cee…` | idênticos | diferem |
   | `elements.data` | `44e36f71…` | idêntico | `20455ccb…` |
   | `tasks.data` | `ac773dd0…` | idêntico | `798a4eb1…` |
   | `aipolicy/domain/domain2/domain2_cross/gshop*/task_npc/dyn_tasks` | — | **todos idênticos** | todos diferem |

   Ou seja: os 11 `.data` do client, o executável e os `.pck` principais são os mesmos
   bytes do client BR. **A única diferença real é que a instalação nova está intocada** —
   sem as pastas `models/`, `litmodels/` e `interfaces/` extraídas que o BR ganhou do
   `tools/pw-pck-extract/`. É o que ela controla, e só isso: se o crash continuar
   idêntico nela, a extração não é a causa nem o agravante.

   **Versão do `elements.data`, com evidência**: cabeçalho `9c 00 00 30` =
   `0x3000009c` → **v156**, o mesmo do BR — logo, o `.cfg` correto é o
   `specs/elements_155/PW_1.5.5_v156.cfg` que já está no repo (não é uma build antiga
   nem desconhecida). Confirmado por conteúdo, não só pelo cabeçalho:
   `python specs/elements_155/walk_tables.py "E:/0_GAMES/Perfect World/element/data/elements.data"`
   decodifica as tabelas 0–19 com os tamanhos de v156 (`WEAPON_ESSENCE`=1556,
   `ARMOR_ESSENCE`=1132, `DECORATION_ESSENCE`=1172) e texto legível em português
   (`'☆Lâmina com Ponta de Aço'`, `'Poção Pequena de Cura'`). Da tabela 20 em diante o
   caminhador descarrila porque os `TABLE_OVERRIDES` do `walk_tables.py` são do arquivo
   russo original — é limitação conhecida do script (override por arquivo, não por
   build), não do arquivo novo; o leitor Rust (`generic_elements.rs`) escolhe overrides
   pela versão do cabeçalho e carrega este arquivo sem erro. `tasks.data` = templ **129**,
   igual ao BR.

   **O que foi configurado nesta rodada** (nada disso muda código):
   - `data/realm_155BR/config/` passou a ser **100% os `.data` deste client**. Estavam
     misturados: `elements/tasks/gshop*/dyn_tasks` já eram os do BR, mas
     `aipolicy/domain/domain2/domain2_cross/task_npc` eram os do client **EN**. Os cinco
     foram substituídos (cópia do EN guardada em
     `data/realm_155BR/_backup_pre_novo_client_20260903/`). `npcgen.data`,
     `DynamicObjects.data` e `ExtDataID.dat` **não** vêm do client e ficaram como estavam.
   - Client novo apontado pro nosso realm: `element/userdata/server/serverlist.txt`
     reescrito (UTF-16LE com BOM, `Nome<TAB>PORTA:HOST<TAB>id`, mesma forma do arquivo
     que já funciona no BR) com `Universal 1.5.5BR → 29004:127.0.0.1` e
     `Universal 1.5.5 EN → 29003:127.0.0.1`. Original salvo como `serverlist.txt.original`.
     `element/elementclient.bat` criado igual ao do BR
     (`elementclient.exe game:cpw console=1 logiccheck:0`). O `dbserver.conf` **não** foi
     tocado: ele está intocado também no BR (mesmo MD5) e o BR chega ao login, logo não é
     ele que define o endereço.
   - `pw-world-155br` e `pw-realm-155br` reiniciados. Log confirma:
     `ELEMENTDATA_VERSION=0x3000009c, task_templ=129` e
     `Gateway pw-link escutando na porta 29004`.

   **RESULTADO DO TESTE (mesma noite)**: **é exatamente o mesmo crash**, provado pelos três
   minidumps lado a lado (parser próprio de minidump, stream 6 = exceção + contexto de CPU):

   | dump | módulo + offset | código | leitura de | EAX / ECX / EDX / EBX / ESI / EDI |
   | :--- | :--- | :--- | :--- | :--- |
   | client novo (E:, intocado) | `ELEMENTCLIENT.EXE+0x576661` | `0xC0000005` | `0x0` | `0 / 0 / 78C4 / 0 / 78C4 / 0` |
   | client BR (F:, extraído) | `ELEMENTCLIENT.EXE+0x576661` | `0xC0000005` | `0x0` | `0 / 0 / 78C4 / 0 / 78C4 / 0` |
   | client EN (F:, build 2575) | `elementclient.exe+0x58B471` | `0xC0000005` | `0x0` | `0 / 0 / 78C4 / 0 / 78C4 / 0` |

   Registrador por registrador idêntico, e o `EC.log` fecha a mesma sequência
   (`configs data is empty` → `SetServerTime` → `glb_HandleException` → *Exception occurred
   in render thread*). Do lado do servidor, a sessão é a mesma de sempre até o fim
   (`Personagem 'Storm' (ID: 39) spawnado com sucesso`, subcomandos 121/154/39/120/136/178/
   127/128 chegando, `Sessão #3 finalizada` no instante do crash).

   **Duas hipóteses fecham com isto**: a **extração dos `.pck`** está definitivamente
   descartada (a instalação intocada, lendo tudo direto do `models.pck`, crasha igual), e
   **conteúdo/itens do personagem** também (`Storm` tinha `Itens: 0`, `HEal` tinha `Itens: 3`
   — mesmo crash nos dois).

   **O erro `\config\patcher\sysinfo.ini` não tem relação com o crash**: a string
   `sysinfo.ini` existe **só** no `patcher/patcher.exe` (1 ocorrência) e **nenhuma vez** no
   `ELEMENTCLIENT.EXE`. Os logs do patcher são de 23:02–23:05 e o crash é 23:06:28. O patcher
   também **não corrompeu nada**: os MD5 de `ELEMENTCLIENT.EXE`, `elements.data`,
   `tasks.data`, `configs.pck` e `interfaces.pck` continuam iguais aos do BR depois do teste.

   **Sobre o `木剑.ecm` (o "primeiro passo" da sessão anterior)**: no client novo o `AF.log`
   não tem **nenhum** erro de `.ecm` — mas isso **não** prova que a extração resolveu nada. A
   diferença é o personagem: a corrida do BR que logou o erro foi com `HEal` (3 itens) e a do
   client novo foi com `Storm` (0 itens), então nem chegou a pedir modelo de arma. Pergunta
   ainda aberta, porém **irrelevante pro crash** (que acontece igual com e sem item).

   **Ainda de pé pro próximo teste**: o `if false &&` do `gateway.rs` (item de limpeza já
   anotado) continua desligando o envio de NPCs — **de propósito nesta rodada**, pra que o
   teste do client limpo rode nas mesmas condições do último teste do BR e a comparação
   valha. Reverter (e reconstruir os containers) antes de fechar a frente.

11. **Sessão 2026-09-03 (noite, continuação): o quarto client instalado em
   `E:\0_GAMES\Perfect World` É de outra família — elements v181, tasks 135, build 2591.**
   Este substituiu o do item 10 (mesmo caminho em disco). Diferente dos outros três em tudo
   que importa:

   | | client novo (E:) | BR / "novo" antigo (F:) | EN (F:) |
   | :--- | :--- | :--- | :--- |
   | `ELEMENTCLIENT.EXE` | 2017-01-12, **build 2591** | 2016-11-09, build 2569 | build 2575 |
   | versão de protocolo | `0x00010505` | `0x00010505` | `0x00010505` |
   | `elements.data` | **v181** (`0x300000b5`), 60.9 MB | v156, 55.4 MB | v159, 55.2 MB |
   | `tasks.data` | **templ 135** | templ 129 | templ 129 |
   | tabelas do `elements` (pelo cfg) | **261** | 231 | 231 |
   | `interfaces.pck` | 2.788 arquivos | 2.642 | — |
   | pacotes acima de 2 GB | **3** (`models`, `litmodels`, **`building`**) | 2 | 2 |
   | arquivos novos | `arcclientsdk.dll`, `gshop_p4.data`, `building.pkx`, CRT de debug | — | — |

   O build **2591** foi lido do binário, não deduzido: o DWORD logo depois da constante
   `0x00010505` dá 2569 no BR e 2575 no EN — exatamente o que o `EC.log` de cada um imprime —
   e 2591 neste. Mesma técnica do item 4.

   **Idioma: também português.** As tabelas 0–19 decodificam com `'☆Lâmina com Ponta de Aço'`,
   `'Poção Pequena de Cura'`, `'Amuleto Def. Física - Nível 1'` — é um repack BR mais novo, não
   um client de outra região. Conteúdo cresceu junto: `WEAPON_ESSENCE` 2.741 → 2.760,
   `ARMOR_ESSENCE` 2.520 → 2.549, `DECORATION_ESSENCE` 969 → 995, e as 148 telas novas do
   `interfaces.pck` são de **arena** e **carrier**, batendo com as tabelas novas do cfg v181
   (`ARENA_FORBID_SKILL_CONFIG`, `CARRIER_CONFIG`, `PROFESSION_PROPERTY_CONFIG`,
   `CHANGE_PROPERTY_CONFIG`).

   **Achado grande: existe um catálogo de `.cfg` da comunidade cobrindo 39 builds** em
   `D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL\configs\CFG\` — de
   `PW_1.5.2_v123` até `PW_1.5.7_v206`, incluindo **`PW_1.5.5_v181.cfg`**. Copiado para
   `specs/elements_155/PW_1.5.5_v181.cfg`. Primeira validação contra o arquivo real: tabelas
   0–19 fecham com `skip=0` e texto legível, e **o mesmo override manual da tabela 20 do v156**
   (`skip=20, count=22`) destrava as tabelas 21–31 aqui também (`'Ataque do Tigre'`,
   `'Portal da Cidade'`, `'Pedra de Uma Fonte'`, `'Flecha Presa de Lobo'`). O caminhador para em
   58,7 MB de 60,9 MB — falta a mesma passada de arqueologia manual que o v156 levou (uns 10
   overrides), não é trabalho novo de método, é trabalho de repetição.

   **O que ainda falta pra usar este client contra o nosso servidor** (não feito, decisão do
   Murillo): gerar `specs/elements_layouts/v181.json` (mesmo `generate_vNNN.py`), resolver os
   overrides restantes, e — o ponto não óbvio — **arranjar o pacote de servidor correspondente**:
   `data/realm_*/config` tem `npcgen.data`, mapas por zona e `DynamicObjects.data`, que **não
   vêm do client**. Copiar só os `.data` do client não basta pra um conteúdo desta build.

   **Piora do bug dos 2 GB**: neste client o `building.pck` também passou de 2 GB
   (2.598.379.780 bytes lógicos, contra 1.846.862.029 no BR), então são **três** pacotes que o
   `AFilePackGame::InnerOpen` não consegue abrir, não dois. O `tools/pw-pck-extract/` lê os três
   sem problema (`building.pck`: 18.552 arquivos indexados).

   **Curiosidade útil pro trabalho de RE**: este binário é um build `DbgReleaseLAA` e carrega o
   caminho do PDB —
   `f:\workspace\wmgj_publish\CElement\CElementClient\DbgReleaseLAA\ElementClient_laa.pdb`
   (GUID `720B0736-9374-46FB-92D9-4817740680C6`, age 8). O PDB **não** está no instalador, mas
   o nome/GUID é uma pista concreta caso apareça em algum repack.

12. **Sessão 2026-09-03 (noite): o `npcgen` do pacote `F:\PW\1.5.5\home155` é MAIS
   compatível com os clients v156/v159 do que o que estamos usando — e, de quebra, achei um
   bug real no nosso leitor de `elements.data`.**

   Medido, não estimado. Escrevi um leitor de `npcgen.data` em Python espelhando
   `crates/pw-data-loader/src/npcgen.rs` (áreas de IA + recursos + objetos dinâmicos +
   controladores) que **exige que o offset final bata exatamente com o tamanho do arquivo**.
   Os **76 arquivos dos dois pacotes fecharam byte a byte, zero sobra** — a leitura é a certa,
   não só plausível. Depois cruzei todos os `template_id` contra `MONSTER_ESSENCE` +
   `NPC_ESSENCE` (e `MINE_ESSENCE` pros recursos) de cada `elements.data`:

   | npcgen | ids de IA | cobertos pelo client BR (v156) | pelo client EN (v159) |
   | :--- | ---: | ---: | ---: |
   | **home155** (`gamed/config`) | 7.974 | **99,96%** (faltam 3: 51003, 51064, 51065) | **99,96%** (mesmos 3) |
   | `data/realm_155/config` (em uso hoje) | 9.268 | 98,08% (faltam **178**) | 98,08% (mesmos 178) |

   Recursos/minérios: o `npcgen` do home155 tem 782 ids, **99,74%** cobertos pelo v159 (faltam
   50892 e 51066); o de `realm_155` tem 1.118, **100%** cobertos pelo v159.

   Os dois pacotes são quase o mesmo: **69 dos 76 mapas são byte a byte idênticos**. As
   diferenças são `world` (2,87 MB no home155 contra 4,27 MB no realm_155), `a46` (vazio, 0
   bytes, no home155), `a61` e mais três iguais em tamanho mas diferentes no conteúdo. Ou seja:
   o `realm_155` que usamos hoje é o mesmo pacote **com spawns extras** — e são justamente
   esses extras que trazem os 178 ids que **nenhum** dos clients conhece.

   O pacote também traz `elements.data` **v156** (cabeçalho idêntico ao do client BR, mesmo
   timestamp `0x58229c4c`, mas 57,4 MB contra 55,4 MB — é o v156 do BR **com conteúdo
   adicionado**) e `tasks.data` **templ 128** (o client BR é 129, o client novo é 135). Nenhum
   dos dois bate exatamente com nenhum client que temos.

   **Bug achado no caminho (independente do npcgen)**: `specs/elements_155/realm_155_overrides.json`
   tem quatro entradas com `abs_count_off` (tabelas 72, 77, 86, 93) — **offsets absolutos**
   medidos num arquivo v156 específico (o russo, 55.075.641 bytes, que já nem está mais em
   `data/realm_155/config`). `load_overrides_for_version(156)` aplica isso a **qualquer**
   arquivo v156, inclusive o do realm BR. Resultado medido no arquivo do
   `realm_155BR`:

   | leitura do `elements.data` do client BR | tabelas vazias de 231 | MONSTER | NPC | MINE | FASHION |
   | :--- | ---: | ---: | ---: | ---: | ---: |
   | com os overrides do v156 | **132** | 8.054 | 4.761 | 0 | 0 |
   | sem override nenhum | 198 | 0 | 0 | 0 | 0 |

   (Pro comparativo: o client EN v159, com os overrides dele, deixa só **16** tabelas vazias.)
   Ou seja, o `pw-world-155br` roda hoje com pouco menos da metade das tabelas do
   `elements.data` decodificadas — `MINE_ESSENCE`, `FASHION_ESSENCE` e outras 130 saem vazias.
   Não é causa do crash de render (que é client-side), mas **é** causa garantida de bug de
   gameplay, e explica por que o cruzamento de recursos deu 0% nos arquivos v156. O conserto é
   levantar um conjunto de overrides próprio pro arquivo do realm BR (mesmo método do v156
   original), e trocar `abs_count_off` por algo relativo ou por override **por arquivo**
   (hash), não por versão.

   **Mapa a mapa, os 178 ids desconhecidos estão concentrados em quatro mapas** — e a mistura
   ideal é óbvia (contagem contra o `elements.data` do client BR):

   | mapa | home155 | `data/realm_155` (em uso hoje) | melhor |
   | :--- | :--- | :--- | :--- |
   | `a61` | 158 ids, **0 desconhecidos** | 158 ids, 2 desconhecidos | home155 |
   | `a63` | 546 ids, **0 desconhecidos** | 546 ids, 3 desconhecidos | home155 |
   | `a76` | 226 ids, **0 desconhecidos** | 226 ids, **145 desconhecidos** | home155 |
   | `a77` | 39 ids, **0 desconhecidos** | 39 ids, **28 desconhecidos** | home155 |
   | `a46` | vazio (0 bytes) | 22 ids, 0 desconhecidos | realm_155 |
   | `world` | 2.857 ids, 3 desconhecidos | **4.160 ids, 0 desconhecidos** | realm_155 |

   Mesma contagem de ids em `a61`/`a63`/`a76`/`a77` nos dois pacotes, com ids trocados por
   outros que o client não tem: o `realm_155` tem esses quatro mapas **modificados** (mobs
   customizados). Trocando só esses quatro pelos do home155 e mantendo `a46` e `world` como
   estão, o conjunto fica com **zero ids desconhecidos** pros clients v156 e v159.

   **Client novo (v181): não dá pra verificar ainda** — sem `v181.json` no catálogo, não há como
   ler `MONSTER_ESSENCE`/`NPC_ESSENCE` dele pra cruzar. Os 3 ids que faltam nos v156/v159 são
   altos (51003, 51064, 51065), o tipo de coisa que uma build mais nova costuma ter, mas isso é
   expectativa, não medição.

13. **Sessão 2026-09-03/04: realm 155BR reconstruído com o pacote home155, e o crash de
   render finalmente ABERTO — cadeia de chamadas reconstruída, função exata identificada,
   campo nulo localizado.** Este é o item mais importante da frente 1.5.5 até agora.

   ### a. O que subiu

   O Murillo refez `data/realm_155BR/config` do zero: base = `F:\PW\1.5.5\home155\gamed\config`
   (76 mapas de `npcgen.data`, `global_api.lua --102`, `aipolicy`, `.sev`, `.conf`), com os 11
   `.data` do client BR por cima (`elements.data` v156, `tasks.data` templ 129). Reconstruí as
   imagens e recriei os dois contêineres. Log confirma: `ELEMENTDATA_VERSION=0x3000009c,
   task_templ=129`, `Gateway pw-link escutando na porta 29004`,
   `World #1 inicializado com 21766 monstros e 3703 NPCs` (contra 24.899/6.918 do pacote
   anterior — esperado, o `world/npcgen.data` do home155 é menor e sem os 178 ids órfãos).
   **Divisão de teste combinada**: client BR → realm 155BR (29004); client EN → realm 155 (29003).

   Também revertido o `if false &&` do `gateway.rs` (item de limpeza pendente): o envio de
   NPCs/monstros voltou a acontecer, já que os três minidumps provaram que NPC não é a causa.
   `cargo check -p pw-link` limpo, imagens reconstruídas.

   ### b. A virada: dá pra fazer engenharia reversa de verdade aqui, sem IDA

   `capstone` (5.0.7) já está instalado nesta máquina. Com ele escrevi
   `tools/pw-crash-re/` (com README): `stackwalk.py` reconstrói a cadeia de chamadas de um
   minidump **validando cada endereço de retorno contra os bytes do `.exe`** (só aceita se
   houver uma instrução `CALL` terminando exatamente naquele endereço), e `dis2.py` desmonta
   qualquer VA achando o alinhamento certo e anotando referências a string.

   Resultado: **18 frames confirmados, idênticos frame a frame entre o build BR 2569 e o build
   EN 2575** (mesmos deslocamentos de pilha, mesmas formas de `CALL`, só os endereços mudando
   de build). É prova independente de que os dois clients morrem no mesmo caminho de código.

   ### c. O crash, instrução por instrução

   Função que crasha: começa em **`+0x5765F0`** (BR). Recebe um id no `[esp+8]`:

   ```
   +0x5765F0  push esi ; mov esi,[esp+8] ; cmp esi,-1 ; jne +0x576653   <- esi = 0x78C4, desvia
   +0x576653  call +0x575D80          ; getter do singleton
   +0x576658  mov  eax,[eax+0x1508]   ; <<< ESTE CAMPO VEM NULO
   +0x57665E  push esi                ; argumento = 0x78C4 (30916)
   +0x57665F  mov  ecx,eax            ; this = NULL
   +0x576661  mov  edx,[eax]          ; <<< CRASH: lê a vtable de NULL
   +0x576663  call [edx+0x20]         ; chamaria o 9º método virtual
   ```

   **Correção de um achado anterior**: o campo nulo está em **`+0x1508`**, não em `+0x151C`
   como a leitura manual de hex da sessão anterior tinha estimado. E **`0x78C4` não é constante
   embutida no código** — a busca pelo imediato `C4 78 00 00` no `.exe` dá **zero ocorrências**
   nos dois builds. Ele vem de `[ebp+0x1c]` no chamador, em tempo de execução.

   O getter é trivial: `+0x575D80` = `call +0x3E5370` (que é só `mov eax,[0xE5B2A4]`, o
   singleton do jogo) seguido de `mov eax,[eax+0x1c]`. Ou seja, o objeto nulo é
   **`[[0xE5B2A4] + 0x1c] + 0x1508`**.

   O chamador (`+0x56B3EF`) é um **despachante**: uma cadeia de ramos paralelos que chamam
   slots virtuais vizinhos do mesmo objeto (`[eax+0x194]`, `[eax+0x19C]`, `[eax+0x1A4]`, ...),
   todos passando `edx = [ebp+0x1c]` — que é de onde sai o `0x78C4`.

   ### d. Onde isso encosta no `configs data is empty`

   Achei as duas funções por nome, pelas strings que elas mesmas imprimem:

   - **`CECGameRun::LoadConfigsFromServer`** = `+0x56410`. O caminho de sucesso lê o blob
     como uma sequência de pedaços `[u32 tamanho][dados]`: o primeiro vai pra
     `[[0xE5B2A4]+0x18]` ("user config data"), o segundo é o "user layout". Com blob vazio ela
     só loga `configs data is empty` e devolve `false`.
   - **`CECGameSession::OnPrtcGetConfigRe`** = `+0x42FE00`, o único chamador. Em `false` ela
     chama `+0x412120` sobre `[[0xE5B2A4]+0x18]` — **o fallback existe mesmo** (é o
     `ApplyUserSetting()` que `S2CGetUIConfigRe` documenta). Depois segue igual pros dois
     caminhos: `eax=[[0xE5B2A4]+0x1c]`, `ecx=[eax+0x18]`, `call +0x3E9A70`, e continua montando
     a UI.

   Conclusão honesta: **mandar `ui_config` vazio não é, sozinho, a causa provada** — o client
   tem caminho pra isso. Mas é a mesma cadeia de objetos (`[g+0x18]` e `[g+0x1c]`) que termina
   com `[g+0x1c]+0x1508` nulo, então continua sendo a suspeita número um.

   ### e. Próximo passo concreto, em ordem

   1. **Andar pra frente dentro de `OnPrtcGetConfigRe` (`+0x42FE00`)** até achar quem atribui
      `[[0xE5B2A4]+0x1c] + 0x1508`, e qual condição pula essa atribuição. É desmontagem
      direta com `tools/pw-crash-re/dis2.py`, sem depender de mais nenhum teste em jogo.
   2. **Descobrir o que é 30916** seguindo o `[ebp+0x1c]` do despachante `+0x56B3EF` pra trás.
   3. **Cruzar com os fontes do EvolvedPW** (`F:\PW\1.5.5\EvolvedPWClient`): as funções já têm
      nome (`CECGameRun::LoadConfigsFromServer`, `CECGameSession::OnPrtcGetConfigRe`), então dá
      pra ler o código-fonte equivalente e ver que membro mora em `+0x1508` e quem o cria. Os
      fontes são de uma build vizinha, não a mesma — vale como mapa, não como verdade (mesma
      lição do item 4).
   4. **Usar o 1.2.6 que funciona como controle**: mesmo motor Angelica, mesmo
      `OnPrtcGetConfigRe`. Comparar o que o nosso realm 1.2.6 manda (e que faz o client entrar
      no mundo e andar) com o que o 155 manda no mesmo ponto — client em
      `F:\Games\perfectworld_126\element`, realm 126 na porta 29000.
   5. **Só se 1-4 travarem**: compilar o `EvolvedPWClient` pra ter um `elementclient` com
      símbolos, ou montar um realm de teste 1.7.2 com o material de `F:\PW\1.7.2`.

14. **Sessão 2026-09-04, passo 1 do item 13: ACHADO. O nosso `TASK_DATA` estava dois blocos
   curto para o 1.5.5 — e é por isso que o campo `+0x1508` chegava nulo no crash.**

   Seguindo o passo 1 do item 13 (achar quem atribui `[[0xE5B2A4]+0x1c]+0x1508`), varri o
   binário atrás de toda instrução que toca o deslocamento `+0x1508`: **98 pontos, e só 4
   escritas**. Duas delas, coladas, estão dentro de uma função que o próprio binário nomeia
   numa string de assert: **`CECHostPlayer::OnMsgHstTaskData`** (`+0xA6B80` no build BR 2569)
   — o tratador do `TASK_DATA`, o comando 105 que o nosso servidor manda.

   ```
   +0xA6B80  mov edi,ecx              ; edi = this (CECHostPlayer)
   +0xA6BA2  mov ecx,[eax+0x10]       ; eax = mensagem; [msg+0x10] = id do subcomando
   +0xA6BA5  cmp ecx,0x69             ; 105 = TASK_DATA
   +0xA6BA8  jne +0xA6D42             ; id errado -> sai SEM criar nada
   +0xA6BAE  mov eax,[eax+0xc]        ; ponteiro pros dados
   ...       le CINCO tamanhos u32 em sequencia, andando o ponteiro a cada bloco
   +0xA6C17  push 0x58 ; call operator new     ; objeto de 88 bytes
   +0xA6C47  mov [edi+0x1508],eax     ; <<< A UNICA criacao desse campo
   ```

   Que `[msg+0x10]` é o **id do subcomando** ficou provado comparando tratadores vizinhos do
   mesmo `switch`: `+0xA68F0` confere `0x64`, `+0xA6B40` confere `0x67`, `+0xA6B80` confere
   `0x69`. No IR (`specs/protocol/gamedata_155.json`): 100 = `PRODUCE_START`, 103 =
   `DECOMPOSE_START`, **105 = `TASK_DATA`**. Bate nos três.

   ### O bug

   `S2CGamedataSend::task_data()` escrevia o id 105 e **três** tamanhos zerados (12 bytes). O
   cliente 1.5.5 lê **cinco**. Ou seja: ele lia 8 bytes **depois do fim do nosso buffer**,
   como se fossem `finished_count_size` e `storage_task_size`, e seguia com dois ponteiros e
   dois tamanhos de lixo.

   O IR sempre soube: `S2C::cmd_task_data` tem cinco campos `_size` — `active_list`,
   `finished_list`, `finished_time_list`, `finished_count`, `storage_task` — tanto no
   `gamedata_153.json` quanto no `gamedata_155.json`. O código é que só escrevia três.

   ### Por que o 1.2.6 nunca sofreu com isso (e o que isso prova)

   Desmontei o mesmo tratador no client 1.2.6 que **funciona** hoje
   (`F:\Games\perfectworld_126\element\elementclient.exe`, função em `+0x508D0`):

   | | 1.2.6 | 1.5.5 |
   | :--- | :--- | :--- |
   | checagem de id | `cmp [msg+0x10], 0x69` | `cmp [msg+0x10], 0x69` |
   | tamanhos lidos | **3** | **5** |
   | tamanho do objeto criado | `new 0x14` (20 B) | `new 0x58` (88 B) |
   | campo onde o objeto é guardado | `[this+0xBB0]` | **`[this+0x1508]`** |

   Três blocos é **exatamente** o que o 1.2.6 espera — por isso aquele realm sempre entrou no
   mundo com o mesmo código, e por isso o bug ficou invisível por tanto tempo. E o campo do
   1.5.5 é `+0x1508`: **o mesmo que aparece nulo no minidump** (`mov eax,[eax+0x1508]` seguido
   de `mov edx,[eax]` em `+0x576661`). A cadeia fecha.

   ### O conserto

   `PorVersao::task_data()` (novo, em `crates/pw-protocol/src/por_versao.rs`): 3 blocos no
   1.2.6, 5 do 1.5.3 em diante — mesmo padrão dos outros comandos que divergem por versão.
   `gateway.rs` passou a usar ele nos dois pontos onde mandava `TASK_DATA`. Teste novo,
   `task_data_tem_tres_blocos_no_126_e_cinco_do_153_em_diante`, em
   `crates/pw-protocol/tests/layouts_do_126.rs`. `cargo test -p pw-protocol -p pw-link` todo
   verde.

   ### O que isto ainda NÃO prova

   Que o crash acaba. O que está provado é que (a) `[this+0x1508]` só nasce dentro do
   `OnMsgHstTaskData`, (b) o nosso `TASK_DATA` estava malformado pro 1.5.5, e (c) é o mesmo
   campo do crash. Se depois deste conserto o crash continuar, o próximo passo é ver se o
   `jne` do id dispara (ou se a chamada em `+0xA6C65`, que recebe os cinco blocos, falha por
   dentro) — agora com o ferramental de `tools/pw-crash-re/` pronto para isso.

   Isto também reabre uma pergunta do item 9d: o pedido `GetUIConfig` que "nunca chegava" do
   cliente é mandado justamente por `LoadConfigData()`, chamado no fim do `OnMsgHstTaskData`.
   Se o tratador saía cedo ou quebrava, o pedido nunca sairia — o que casa com o sintoma. O
   envio proativo de `GetUIConfig_Re` (item 9d) pode ter sido remédio pro sintoma, não pra
   causa; vale reavaliar depois de testar.

15. **Sessão 2026-09-04: o crash acabou (confirmado em jogo, nos dois clients) — e o próximo
   sintoma, "nenhum NPC aparece", é a MESMA família de bug.**

   ### a. Confirmação em jogo

   Com o `TASK_DATA` de 5 blocos (item 14), **os dois clients entraram no mundo sem crashar**.
   O log do realm traz a prova independente que faltava:

   ```
   TASK_DATA enviado pro personagem ID 39
   Enviando 28 entidades (NPCs/Monstros) ... para o jogador 'Storm'
   GetUIConfig pedido pelo personagem ID 39      <-- NUNCA tinha acontecido
   ```

   O `GetUIConfig` **vindo do cliente** é mandado por `LoadConfigData()`, no fim do
   `CECHostPlayer::OnMsgHstTaskData`. Ele aparecer pela primeira vez prova que aquele tratador
   agora roda até o fim — exatamente o que o item 14 previu. Isso também confirma que o envio
   proativo de `GetUIConfig_Re` (item 9d) era remédio pro sintoma: agora dá pra reavaliar.

   ### b. Por que nenhum NPC aparecia

   Mesmo mecanismo. O cliente calcula o tamanho esperado de cada subcomando pelo `sizeof` da
   struct e **descarta o comando inteiro** quando não bate (`CalcS2CCmdDataSize` +
   `ASSERT(dwCmdSize == dwDataSize)`, `EC_GameDataPrtc.cpp`). Nosso `NPC_ENTER_WORLD` escrevia
   **27 bytes**; o 1.5.x quer **35**.

   A struct veio do fonte do cliente, não de dedução — `EC_GPDataType.h:732`, dentro do bloco
   `#pragma pack(1)` que começa na linha 563 (por isso 35 e não 36):

   ```c
   struct info_npc {
       int nid; int tid; int vis_tid;      // <- vis_tid nao existia no 1.2.6
       A3DVECTOR3 pos; unsigned short seed; unsigned char dir;
       int state; int state2;              // <- state2 idem
       // campos seguintes condicionais a bits de `state` (com state=0, nenhum)
   };
   ```

   Corrigido em `PorVersao::npc_enter_world`/`npc_enter_slice`: 27 bytes no 1.2.6 (medido em
   jogo — aquele realm mostra NPC e deixa falar com eles), 35 do 1.5.3 em diante (IR idêntico
   nos dois, e o IR do 155 sai do `EvolvedPWClient`). Teste
   `info_npc_ganha_vis_tid_e_state2_do_153_em_diante`.

   ### c. O inventário do que ainda falta — e o furo no teste que o escondia

   `subcomandos_s2c_contra_o_ir.rs` já cobrava tamanho contra o IR, mas `bytes_do_comando()`
   devolve `None` quando a struct tem `bytes: null` — e **125 das structs do IR têm**, porque o
   `pw-rpcgen` marca como "variável" toda struct com campos condicionais no fim. `info_npc` e
   `cmd_task_data` são exatamente esse caso. Resultado: os dois bugs que derrubaram o 1.5.5
   passaram por baixo do teste. Somando os campos fixos (o "tamanho-base", que é o que os
   nossos codificadores sempre escrevem), o inventário contra o IR do **1.5.5** fica:

   | codificador | escreve | IR 1.5.5 | falta | situação |
   | :--- | ---: | ---: | ---: | :--- |
   | `task_data` | 12 | 25 | 13 | **resolvido** via `PorVersao` (item 14) |
   | `npc_enter_world` / `npc_enter_slice` | 27 | 35 | 8 | **resolvido** via `PorVersao` (este item) |
   | `self_info_1` | 34 | 38 | 4 | já resolvido via `PorVersao` (item 9a) |
   | `get_own_money` | 8 | 12 | 4 | já resolvido via `PorVersao` |
   | `inst_data_checkout` | 20 | 24 | 4 | já resolvido via `PorVersao` |
   | `player_enter_world` | 20 | 30 | 10 | **em aberto** |
   | `npc_info_list` | 29 | 3 | — | lista de tamanho variável, comparação não se aplica |

   **`player_enter_world` está errado de forma mais séria que os outros**: escreve
   `role_id, world_tag, pos`, e o `S2C::info_player_1` (`EC_GPDataType.h:603`) é
   `cid, pos, crc_e, crc_c, dir, level2, state, state2` — **não existe `world_tag` na struct**.
   Não quebra nada hoje porque **nenhum caminho do servidor chama essa função** (por isso outro
   jogador nunca aparece na tela). É o próximo alvo natural quando for testar dois jogadores.

   ### d. Próximos passos

   1. Testar em jogo se os NPCs aparecem agora. Se aparecerem mas sem nome/HP, o próximo é
      `NPC_INFO_00` (33) e `NPC_INFO_LIST` (9).
   2. Fechar o furo do teste: fazer `bytes_do_comando()` cair pro tamanho-base (soma dos campos
      fixos) quando `bytes` for nulo, para que uma divergência nova falhe em vez de sumir.
   3. `player_enter_world` com o layout do `info_player_1`, e ligá-lo ao caminho de "outro
      jogador entrou na sua área".
   4. Daí em diante, a lista de gameplay do item 62 (skills, missões, HP de NPC) — agora com o
      método que funcionou: comparar byte a byte contra o `EC_GPDataType.h` do
      `EvolvedPWClient`, que é a fonte de verdade do 1.5.5 e está no disco.

16. **Sessão 2026-09-04: os dois próximos passos do item 15 — fechados. Mais um
   codificador da mesma família encontrado e corrigido de brinde. E os quatro sintomas
   novos que o Murillo reportou em jogo (skills, IA de monstro, andar-até-clicar,
   teleporte de GM) foram diagnosticados com evidência — dois deles a fundo, prontos
   pra virar trabalho na próxima rodada.**

   ### a. Furo do teste fechado

   `bytes_do_comando()` (`subcomandos_s2c_contra_o_ir.rs`) parava em `None` sempre que
   a struct do IR não tinha `bytes` fixo — e 125 das 231 structs do IR não têm, porque
   o `pw-rpcgen` marca como "variável" qualquer struct com campos condicionais no fim,
   mesmo quando o começo é fixo. Foi assim que `TASK_DATA` e
   `NPC_ENTER_WORLD`/`NPC_ENTER_SLICE` (os dois bugs do item 14/15) passaram por baixo
   deste teste sem acusar nada. Agora ela cai pro **tamanho-base** — a soma dos campos
   que têm tamanho fixo, o piso que qualquer registro real ocupa — e só devolve `None`
   quando um campo em si não é fixo (aí a struct é variável de verdade).

   Isso destampou **4 divergências reais e já conhecidas** (`task_data`,
   `npc_enter_world`, `npc_enter_slice`, `self_info_1`) que passaram a aparecer porque
   a base `S2CGamedataSend::*` continua no layout do 1.2.6 de propósito — quem cobre a
   diferença é o `PorVersao`. Documentado e adicionado a `LAYOUT_DIVERGE`, cada um com o
   porquê. `a_lista_de_divergencias_de_layout_esta_em_dia` volta a passar.

   ### b. `player_enter_world` com o layout certo — e ligado a "outro jogador na área"

   Confirmando o item 15: a struct real é `S2C::info_player_1`
   (`EC_GPDataType.h:603`) — `cid, pos, crc_e, crc_c, dir, level2, state, state2`, 30
   bytes, igual no IR do 1.5.3 e do 1.5.5. Reescrito seguindo o mesmo padrão já
   estabelecido de `player_info_00`/`self_info_1`: a base vira o layout cheio (1.5.3+),
   e `PorVersao::player_enter_world` escreve a versão de 26 bytes sem `state2` pro
   1.2.6 (sem captura própria pra este comando — extrapolado do padrão medido em
   `PLAYER_INFO_00`, que é a mesma família "campo final que o 1.5.x acrescenta").
   Também criado `player_leave_world` (`PLAYER_LEAVE_WORLD`, 19) — não existia
   nenhum codificador pra ele, e a struct real (`cmd_player_leave_world`, só o `id`,
   4 bytes) não diverge entre versões.

   **Ligado ao caminho de visibilidade mútua**, mas com um desvio de arquitetura
   deliberado: em vez de fazer o `pw-gs` (que já tem `WorldInstance.players` e uma
   grade espacial de verdade) carregar o personagem sozinho, o que exigiria **ou**
   estender `BusMessage::EnterWorld` — que hoje espelha byte a byte o pacote GNET
   opcode 72 do cliente, de propósito — **ou** o mundo buscar o personagem sem
   `account_id` (que o `EnterWorld` do barramento não carrega), a visibilidade foi
   implementada **inteiramente dentro do `pw-link`**: um novo campo,
   `LinkGateway::jogadores_visiveis` (`RwLock<HashMap<RoleId, JogadorVisivel>>`),
   populado no mesmo ponto em que `details` (o personagem completo, já carregado com
   `account_id`) fica disponível. Cada sessão do mesmo realm roda no mesmo processo
   `pw-link`, então "mandar pra outro jogador" é só achar o `envio` (canal de saída)
   dele neste mapa e escrever — sem round-trip pelo barramento.

   No `EnterWorld`: o jogador novo recebe um `PLAYER_ENTER_WORLD` pra cada jogador já
   visível, e cada um deles recebe um `PLAYER_ENTER_WORLD` do jogador novo. Na
   desconexão (inclusive queda, não só logout limpo): remove da lista e manda
   `PLAYER_LEAVE_WORLD` pros que sobraram.

   **Limitações documentadas no próprio código, não escondidas**: sem grade espacial
   (só existe um mundo por realm hoje, então não é errado, só não escala); a posição
   só atualiza quando alguém ENTRA depois — um jogador já visível não se move na tela
   de quem já o viu, porque `PlayerMoveBroadcast` hoje só ecoa pro remetente (achado
   voltando a aparecer, ver item d.2 abaixo). Migrar pro `pw-gs`, reaproveitando a
   grade espacial que já serve NPC/monstro, é o passo natural quando a população
   justificar.

   ### c. De brinde: `object_skill_attack_result` tinha o MESMO bug

   Rodando o inventário de tamanho-base contra o IR do 1.5.5 pra conferir o próprio
   trabalho acima, apareceu um quinto codificador na mesma família dos "cinco comandos
   de resultado de ataque" que o topo de `por_versao.rs` já documentava, mas que nunca
   tinha ganhado o tratamento: `OBJECT_SKILL_ATTACK_RESULT` (143) escrevia
   `attacker_id, target_id, skill_id, damage, speed, attack_flag` (18 bytes,
   `attack_flag` de 1 byte, na ordem errada, sem `section`) — sem nenhum chamador em
   produção, mesma razão de `player_enter_world` ter ficado quebrado sem ninguém notar.
   Corrigido seguindo `self_skill_attack_result` como molde: base = layout cheio do
   IR (22 bytes, `attack_flag` i32 + `section`), `PorVersao::object_skill_attack_result`
   = versão de 18 bytes pro 1.2.6.

   `cargo test -p pw-protocol -p pw-link -p pw-gs` — tudo verde depois dos três
   consertos. (Duas falhas em `pw-data-loader` — `test_elements_data_real_file_if_present`
   e `test_game_data_manager_directory_load` — são **pré-existentes**, de antes desta
   sessão, sobre `elements.data v55`/`npcgen` do 1.2.6 na pasta local; não relacionadas
   a nada tocado aqui.)

   ### d. Os quatro sintomas que o Murillo reportou em jogo — diagnosticados

   Testou só o client BR contra o `realm_155BR`, depois da correção do item 15: NPCs
   aparecem e respondem HP corretamente (`QUERY_PLAYER_INFO_1`/`NPC_INFO_00`
   funcionando). Quatro sintomas restantes, nesta ordem de evidência reunida:

   **1. Skills não fazem nada (testado: Portal da Cidade).** Causa raiz achada, não só
   suspeitada: `BusServer::conjurar` (`bus_server.rs`, `C2S::CAST_SKILL`/
   `CAST_INSTANT_SKILL`) **exige um alvo** — `let Some(alvo) = alvo else { ...; return
   };` sai em silêncio, sem mandar `OBJECT_CAST_SKILL` nem `SKILL_PERFORM`, pra
   qualquer habilidade sem alvo. Hoje o handler só implementa dano contra monstro
   selecionado; nenhuma habilidade de auto-alvo (portais, buffs, cura em si mesmo)
   dispara pacote nenhum. Bate exatamente com o sintoma. Consertar exige decidir o que
   cada categoria de skill sem alvo faz (o Portal da Cidade especificamente precisa
   de um sistema de "ponto de vínculo"/recall que este servidor ainda não tem) — não é
   um ajuste de layout como os de hoje, é feature nova. Ponto de partida certo pra
   próxima sessão.

   **2. Monstros spawnam parados, não andam nem atacam.** Duas causas, não uma —
   achadas lendo `WorldInstance::tick` (`world.rs:520`):
      - `MonsterAi::tick` já calcula posição e ataque server-side (`ai.rs`,
        movimento por perseguição de ameaça) — mas `WorldInstance::tick` nunca emite
        nenhum evento/pacote de "monstro se moveu" pro cliente. O servidor anda o
        monstro; ninguém nunca conta pro jogador.
      - Mais grave: `ai.tick(monster, &self.players, delta_ms)` só pode perseguir ou
        atacar alguém que esteja em `self.players` — e `WorldInstance::add_player`
        nunca é chamado em produção (só em teste, `crates/pw-gs/tests/
        subcomandos_no_mundo.rs`; confirmado por busca no repo inteiro). Ou seja, do
        ponto de vista da IA de monstro, não existe nenhum jogador no mundo hoje —
        não é que os monstros ignoram o jogador, é que o servidor nunca soube que ele
        chegou. Isto também explica por que `QUERY_PLAYER_INFO_1` (barra de vida de
        outro jogador, já implementado) nunca tinha o que responder antes deste item.
      - Consertar isto de verdade (não como o atalho local do item b acima) pede
        popular `world.players` no `EnterWorld` de verdade, o que esbarra na mesma
        parede arquitetural do item b: o `BusMessage::EnterWorld` não carrega
        `account_id` nem os dados do personagem. Vale a pena resolver de vez — dá
        também IA de monstro e visibilidade entre jogadores com raio de verdade
        (reaproveitando `WorldInstance::grid`), não só o que o atalho de hoje cobre.

   **3. Clicar no mapa não anda sozinho até lá (auto-track).** Só localizado o
   mecanismo, não a causa: o cliente manda `C2S::PlayerMove` continuamente enquanto
   anda (mode/posição/alvo/velocidade — é o mesmo pacote de sempre, não existe um
   "comando de auto-caminhar" separado no protocolo). O `gateway.rs` já trata isso
   (`InboundPacket::PlayerMove`) e ecoa um `PlayerMoveBroadcast` de volta. Não
   investigado nesta sessão — não dá pra saber sem capturar o que o cliente
   realmente manda ao clicar (ou se ele sequer manda algo, o que apontaria pra
   colisão/navmesh do lado do cliente, não do servidor).

   **4. Ctrl+clique de GM não teleporta.** Não encontrado handler nenhum pra isso no
   servidor (busca por `TELEPORT`/`auto_track` não achou opcode C2S correspondente
   tratado) — mas também não confirmado que o comando sequer chega (pode estar entre
   os "Opcode C2S desconhecido" que os logs do realm já mostram fartamente). Precisa
   de captura/log do pacote que o cliente manda nesse clique antes de qualquer
   conserto.

   ### e. Estado e próximo passo

   Realms 155/155BR reconstruídos e recriados com os três consertos deste item. Testar
   de novo com os dois clients (BR→29004, EN→29003) é o próximo passo imediato do
   Murillo. Depois disso, a ordem de valor pro resto do backlog:
   1. Popular `world.players` de verdade (resolve item d.2 por completo, e permite
      trocar o atalho do item b por uma visibilidade com raio de verdade).
   2. Skills sem alvo (item d.1) — decisão de design antes de código.
   3. Capturar o clique de andar e o Ctrl+clique de GM (itens d.3/d.4) — sem isso é
      chute, contra o princípio do projeto.

17. **Sessão 2026-09-04 (continuação): teste com 2 contas simultâneas. Achado e CORRIGIDO
   um bug estrutural no `npcgen.data` que apagava NPCs/monstros/recursos permanentes do
   mundo inteiro — não só o Ancião. Visibilidade entre jogadores e Portal da Cidade
   diagnosticados a fundo, com o motivo real de cada um documentado; não corrigidos ainda
   porque os dois pedem arquitetura nova, não um ajuste de layout.**

   ### a. `npcgen.data`: `id_ctrl != 0` NÃO significa "área desligada" — corrigido

   O Murillo reportou o "Ancião da Cidade das Plumas" faltando. Medido, não estimado:
   escrevi um simulador em Python do parser real (`tools/pw-crash-re/`, técnica já usada
   nesta sessão) e apliquei contra `data/realm_155BR/config/world/npcgen.data`. A área que
   contém o Ancião (tid 2191, a 50,8m do ponto de spawn testado — dentro do raio de 120m
   que o servidor manda) tem `id_ctrl=2077`. O parser (`crates/pw-data-loader/src/
   npcgen.rs`) sempre tratou **qualquer** `id_ctrl != 0` como "área não liga sozinha no
   boot" — um comentário antigo já avisava que isso era simplificação ("controla
   quando/se uma área é ativada... modelar isto de verdade é trabalho separado").

   Inspecionei o registro do controlador 2077 no mesmo arquivo: `ativado=1`, e o nome (GBK)
   decodifica como **"大地图默认长老"** = "Ancião Padrão do Mapa Aberto". Ou seja, a maioria
   dos `id_ctrl` não é gatilho de evento sazonal — é o mecanismo NORMAL de registro/respawn
   de NPCs permanentes, e a maioria desses controladores já nasce **ativado**. Tratar
   `id_ctrl != 0` como "inativo" apagava qualquer NPC, monstro ou nó de recurso cuja área
   usa esse mecanismo — o que inclui NPCs-âncora de cidade inicial inteiros, não um caso
   isolado. Contando de verdade: das **78 instâncias ativas de fato** dentro de 120m do
   ponto de spawn testado, o código antigo só deixava passar **20** — quase 75% descartado.

   **Corrigido em `crates/pw-data-loader/src/npcgen.rs`**: o parser agora lê os
   controladores (seção 4, sempre por último no arquivo) ANTES de decidir quais áreas
   entram, montando um `HashMap<id, ativado>`. Uma área com `id_ctrl != 0` só fica de fora
   se o controlador dela existir e estiver `ativado=0`; um `id_ctrl` que não bate com
   nenhum controlador continua contando como ativo (suposição segura: "existe", não
   "sumiu"). Isso pediu reestruturar o parser em duas fases (as seções 1-3 armazenam o que
   vão precisar; a decisão e a criação de `SpawnInstance` só acontecem depois da seção 4)
   — sem reler o arquivo duas vezes, é uma passada só, só adia a decisão final.

   Corrigido também pra objetos dinâmicos (seção 3), que tinham a MESMA checagem
   simplista (`id_ctrl == 0` direto) — agora usa o mesmo `esta_ativa()`.

   **3 testes novos** com um `npcgen.data` sintético em memória, cobrindo os três casos
   (controlador ativado → spawna; desativado → não spawna; sem controlador → sempre
   spawnou, sem mudança de comportamento). `cargo test -p pw-data-loader` — os dois
   arquivos reais (`test_npcgen_data_real_file_if_present`,
   `test_game_data_manager_directory_load_155`) continuam passando; as duas falhas
   pré-existentes (`elements.data v55`, `npcgen` versão 5/6 do 1.2.6) são inalteradas —
   não são desta sessão.

   Realms 155/155BR reconstruídos com o conserto. **Esperado**: o log
   "World #N inicializado com X monstros e Y NPCs" deve mostrar uma contagem bem maior do
   que antes — confirmar ao testar de novo.

   ### b. Por que jogadores continuam invisíveis um pro outro (mesmo com `player_enter_world`
   certo)

   Diagnóstico completo, não corrigido — é feature nova, não ajuste de protocolo. Lendo
   `EC_ManPlayer.cpp` (`CECPlayerMan::OnMsgPlayerInfo`, `F:\PW\1.5.5\EvolvedPWClient`): ao
   receber `PLAYER_ENTER_WORLD`, o cliente cria a entidade (`ElsePlayerEnter`), mas
   **imediatamente** pede de volta ao servidor três coisas, e sem elas o avatar não tem
   como saber que modelo desenhar (raça/classe/gênero não vêm no `info_player_1`, só o
   `cid`):

   - `PlayerBaseInfo` (GNET, opcode 91) → nosso servidor devolve `PlayerBaseInfo_Re` (92)
     com uma struct `GRoleBase` (id, name, race, cls, gender, custom_data, config_data,
     status...) — achado no IR (`specs/protocol/gnet_155.json`, `protocols`).
   - `GetCustomData` (116) → `GetCustomData_Re` (117), a aparência customizada (rosto/
     cabelo/corpo).
   - `c2s_CmdGetOtherEquip` → equipamento visível (armas/armadura no modelo).

   **Nenhum dos três está implementado** — nem decodificado do lado C2S, nem codificado do
   lado S2C (busca no repo inteiro por `PlayerBaseInfo`/`GetCustomData`/`GetOtherEquip`:
   zero ocorrências). Hoje esses pacotes caem no `InboundPacket::Unknown` (o catch-all que
   já loga "Opcode C2S desconhecido"), então não quebram nada — só ficam sem resposta, e o
   avatar nunca materializa.

   **Achado extra, relevante pra decidir a arquitetura do conserto**: no servidor real
   (`F:\PW\1.5.5\EvolvedPWServer`), `PlayerBaseInfo` não é tratado pelo `gamed` (o
   equivalente ao nosso `pw-gs`) — é o **`gdeliveryd`**, um daemon à parte que mantém um
   cache de "informação básica de papel" pra toda a zona (`cnet/gdeliveryd/
   playerbaseinfo.hpp`). Este projeto não tem (nem precisa ter) um `gdeliveryd` próprio —
   dá pra responder isso direto do `pw-link`, que já tem `char_repo` com tudo que a
   `GRoleBase` precisa. E já existe o codificador certo pra copiar o padrão: `write_role_info`
   (`crates/pw-protocol/src/packets/s2c.rs`) já escreve raça/classe/gênero/nome/aparência/
   equipamento pro próprio personagem em `RoleList_Re`/`CreateRole_Re` — mesma forma de
   campos, só reordenados.

   **Plano concreto pra próxima rodada** (não iniciado, para não arriscar meio-feito):
   1. Adicionar `PlayerBaseInfo`(91)/`PlayerBaseInfo_Re`(92) e
      `GetCustomData`(116)/`GetCustomData_Re`(117) em `InboundPacket`/`OutboundPacket`
      (`pw-protocol`), com decode/encode seguindo os campos do IR listados acima.
   2. No `gateway.rs`: ao receber `PlayerBaseInfo` (lista de `roleid`), buscar cada um via
      `self.char_repo` (mesma chamada que já carrega `details` no `EnterWorld`, só que sem
      o filtro de `account_id` — precisa um método novo ou ajustado no repositório, já que
      `get_details` hoje exige o dono da conta) e responder um `PlayerBaseInfo_Re` por id.
   3. `GetCustomData_Re`: `custom_appearance` já existe em `CharacterDetails` — é olhar como
      `write_role_info` já serializa esse campo (`custom_appearance.get("raw")` como hex, ou
      JSON cru) e reaproveitar.
   4. Equipamento (`CmdGetOtherEquip`) pode ficar pra depois — o `EC_ManPlayer.cpp` só pede
      isso depois de já ter base+aparência, então o avatar já deveria aparecer (sem itens
      visíveis) com só os passos 1-3.

   ### c. Por que o Portal da Cidade não funciona — e por que não é layout

   Confirmado no servidor real (`cgame/gs/playercmd.cpp`, `case C2S::CAST_SKILL`): o
   `CAST_SKILL`/`CAST_INSTANT_SKILL` de verdade cria um objeto de sessão
   (`session_skill`/`moving_skill`, com `SetTarget(skill_id, force_attack, target_count,
   targets)` — `targets` **pode vir vazio**, é normal pra skill sem alvo) que gerencia o
   tempo de canalização, a animação, e só no fim aplica o efeito da habilidade — que é
   **script/dado por skill**, lido de uma tabela própria, não código fixo por skill.

   Nosso `BusServer::conjurar` (`crates/pw-gs/src/bus_server.rs`) não é uma versão menor
   dessa arquitetura — é outra coisa: presume sempre "habilidade de dano contra monstro
   selecionado" e sai em silêncio pra qualquer coisa fora disso (é o item d.1 da sessão
   anterior, agora confirmado contra o fonte do servidor real, não só inferido do sintoma).
   Não existe hoje: sessão de skill com duração, sistema de efeito por skill orientado a
   dado, nem sequer o conceito de "habilidade sem alvo é válida". Implementar Portal da
   Cidade de verdade (teleporte pra um ponto de vínculo) pede um sistema de "home
   point"/vínculo que também não existe.

   **Não tentei um atalho aqui** — um "conserto" que só faz a animação rodar sem o efeito
   real seria pior que nada (o jogador acha que ativou o portal e não fica sabendo por quê
   não teleportou). Fica como item de arquitetura nova pra próxima rodada, não de bug fix.

   ### d. Ordem sugerida pro que falta

   1. **Confirmar em jogo** que o conserto do `npcgen.data` (item a) trouxe os NPCs que
      faltavam — é o único item desta rodada já corrigido e aguardando validação.
   2. **`PlayerBaseInfo`/`GetCustomData`** (item b) — maior valor imediato: sem isso,
      multiplayer visual não existe, e o plano já está pronto, só falta escrever.
   3. **Sistema de sessão de skill** (item c) — maior escopo, decisão de arquitetura antes
      de código (visto que impacta toda a lista de skills, não só o Portal).
   4. Itens 3/4 da rodada anterior (clique-pra-andar, teleporte de GM) continuam abertos —
      precisam de captura real antes de qualquer tentativa.
18. **Sessão 2026-09-04 (continuação 3): Ancião confirmado em jogo (item 17 funcionou).
   Achada e corrigida a causa do "desconecta e volta pro login" ao sair para a seleção
   de personagem — a mesma família de bug de tamanho de pacote desta sessão inteira,
   desta vez batendo num protocolo que quebra a CONEXÃO, não só um comando silencioso.**

   ### a. Como a causa foi encontrada — pelo log do próprio cliente, não suposição

   O Murillo reportou: ao clicar para voltar à seleção de personagem ou sair do jogo,
   aparece como desconectado e sempre volta pra tela de login (mesmo sendo um "meia
   saída", que deveria manter a conexão e só trocar de tela).

   Rastreei o C2S `LOGOUT` (subcomando 1) e o `S2CPlayerLogout` (opcode 69) ponta a ponta
   — `Logout::ler`/`tipo()` em `crates/pw-gs/src/comandos.rs` mapeiam `0/1` pra
   `SairDoJogo`/`SelecaoDePersonagem` exatamente como o cliente (`_PLAYER_LOGOUT_FULL=0`/
   `_HALF=1`, `EC_GameSession.cpp`), e o `resultado` calculado em `BusServer::sair`
   (`bus_server.rs`) já estava certo. Toda essa cadeia bateu — **não era o bug**.

   O log do realm (`docker logs pw-realm-155br`) não tinha nenhuma linha "pediu saída"
   pro teste em questão — ou seja, o `LOGOUT` nem chegou a ser processado. Fui direto no
   `EC.log` do cliente (`F:\PW\1.5.5\1.5.5 BR\...\logs\EC.log`, no mesmo disco desta
   sessão) e achei a causa real:

   ```
   CECGameSession::IOCallBack, EVENT_DELSESSION, error code = Decode error 103
   CECGameSession::OnLinkBroken(Online passive broken)
   ```

   "Decode error 103" é o cliente falhando ao decodificar um pacote GNET — e o `103` ali
   **não é um código de erro, é o `type` (opcode) do protocolo que ele estava tentando
   ler** quando a exceção de marshalling estourou (`gnproto.h`, `catch (Marshal::Exception
   &) { ...snprintf(errormsg, "Decode error %d", type); }`). Opcode **103 = `SetUIConfig_Re`**
   (confirmado em `specs/protocol/gnet_155.json`).

   ### b. O bug: dois codificadores de confirmação com campo faltando — a MESMA família de
   toda a sessão

   `SetUIConfig_Re` é o que o cliente espera receber depois de mandar `SetUIConfig` (o
   cliente salva o layout de UI no servidor nesses momentos de transição — sair pra
   seleção, fechar o jogo). O IR (`gnet_155.json`, `protocols`) diz que `SetUIConfig_Re`
   tem **3 campos** (`result, roleid, localsid` — 12 bytes); nosso `S2CSetUIConfigRe`
   (`crates/pw-protocol/src/packets/s2c.rs`) só escrevia **2** (`result, role_id` — 8
   bytes). Mesma coisa, pior, em `S2CSetCustomDataRe` (resposta a `SetCustomData`,
   aparência customizada): o IR pede **4** campos (`result, CRC, roleid, localsid` — 16
   bytes), o código só escrevia 2 (8 bytes) — faltavam `CRC` **e** `localsid`.

   A diferença pra todos os outros bugs de tamanho desta sessão (TASK_DATA,
   NPC_ENTER_WORLD, OBJECT_SKILL_ATTACK_RESULT, `player_enter_world`): aqueles eram
   subcomandos do `GamedataSend` (mundo 3D) — o cliente descartava o comando em silêncio
   e o efeito era "isso não aconteceu". `SetUIConfig_Re`/`SetCustomData_Re` são
   protocolos **GNET de nível de conexão**, decodificados por um parser mais rígido
   (`gnproto.h::Decode`) que, ao falhar o *marshalling*, lança uma exceção que o cliente
   trata como **link quebrado** — fecha a sessão inteira (`OnLinkBroken`) e cai pro login.
   Por isso "voltar à seleção de personagem" e "sair do jogo" pareciam sempre resultar em
   desconexão total: o campo que faltava é lido bem no momento em que o cliente está de
   saída, então o sintoma só aparece exatamente ali.

   ### c. Por que isto não tinha sido pego antes — e por que não deve se repetir

   `crates/pw-protocol/tests/campos_contra_o_ir.rs` já tem, desde antes desta sessão, um
   framework genérico que lê o `encode()`/`decode()` de cada pacote do fonte e compara
   campo a campo com o IR — foi esse framework que garantiu `S2CGetUIConfigRe` e
   `S2CGetHelpStatesRe` corretos. `S2CSetUIConfigRe` e `S2CSetCustomDataRe` (e
   `S2CSetHelpStatesRe`, que por sorte já estava certo) **nunca tinham entrado na lista
   `MAPA`** — o framework existia, só não olhava pra eles. Adicionados os três.

   ### d. Corrigido

   - `S2CSetUIConfigRe`: `+localsid: u32`.
   - `S2CSetCustomDataRe`: `+crc: u32, +localsid: u32` (`CRC` não é lido pelo cliente em
     `OnPrtcSetCustomDataRe`, confirmado em `EC_GameSession.cpp` — só precisa existir no
     pacote pros campos seguintes caírem no deslocamento certo; `0` é seguro).
   - `gateway.rs`: os dois pontos de construção (`InboundPacket::SetUIConfig`/
     `SetCustomData`) agora passam `req.localsid` (já vinha decodificado certo do lado
     C2S — só a resposta S2C estava incompleta).
   - `MAPA` em `campos_contra_o_ir.rs` ganhou os três — teste geral confere agora **34
     pacotes, 224 escalares**, todos batendo com o IR.
   - Varredura extra: reaproveitei a técnica de tamanho-base pra conferir TODOS os 7
     pacotes de tamanho fixo já no `MAPA` contra `gnet_155.json` — os 7 batem
     exatamente, incluindo os três novos.

   `cargo test --workspace` — só as duas falhas pré-existentes de sempre
   (`elements.data v55`, `npcgen` v5/6 do 1.2.6), nada novo. Realms reconstruídos.

   ### e. Confirmado em jogo (mesma sessão de teste)

   O Ancião da Cidade das Plumas (item 17) **apareceu** no teste do Murillo — o fix do
   `npcgen.data` está validado em jogo, não só em teste automatizado.

   ### f. Próximo passo

   Testar de novo: sair pra seleção de personagem e sair do jogo, confirmando que agora
   fica na tela certa (seleção continua a sessão; sair vai pro login) sem a mensagem de
   desconexão. Depois disso, a ordem sugerida do item 17 continua valendo: visibilidade
   entre jogadores (`PlayerBaseInfo`/`GetCustomData`, plano pronto) e o sistema de sessão
   de skill (Portal da Cidade e o resto).
19. **Sessão 2026-09-04 (continuação 4): achado e corrigido por que trocar de
   personagem na mesma conexão trava em "Entrando em Perfect World". Visibilidade
   entre jogadores implementada de ponta a ponta (`PlayerBaseInfo`/`GetCustomData`) —
   pronta pra testar.**

   ### a. O bug: a flag `ui_config_enviado` nunca era resetada ao trocar de personagem

   Depois do fix do item 18 (sair pra seleção de personagem não desconecta mais), o
   Murillo testou o próximo passo natural — voltar à seleção e entrar com OUTRO
   personagem, na mesma conexão — e travou na tela "Entrando em Perfect World".

   O log do realm mostrou a causa direto, comparando as duas entradas da mesma sessão:

   ```
   # Personagem 39 (primeira entrada da sessão):
   Enviando 29 entidades ...
   GetUIConfig_Re enviado proativamente pro personagem ID 39 (localsid 2)
   Personagem 'Storm' (ID: 39) spawnado com sucesso ...
   ...
   GetUIConfig pedido pelo personagem ID 39 (localsid 0)
   GetUIConfig_Re já tinha sido mandado proativamente — ignorando ...

   # Personagem 42 (segunda entrada, MESMA sessão, depois de voltar à seleção):
   Enviando 28 entidades ...
   Personagem 'testesacer' (ID: 42) spawnado com sucesso ...   <- SEM a linha de GetUIConfig_Re!
   ...
   GetUIConfig pedido pelo personagem ID 42 (localsid 0)
   GetUIConfig_Re já tinha sido mandado proativamente — ignorando ...   <- MENTIRA
   ```

   A linha "enviado proativamente" nunca apareceu pro personagem 42, mas quando ele
   pediu via `GetUIConfig` (opcode 104), o servidor respondeu "já tinha sido mandado" —
   e não mandou nada. `ClientSession::ui_config_enviado` (`session.rs`) é uma flag **por
   sessão**, não por personagem: ela ficou `true` desde a entrada do personagem 39 e
   nunca foi resetada quando a sessão trocou de personagem. Resultado: nenhum
   `GetUIConfig_Re` chega pro segundo personagem, e o cliente fica pra sempre esperando
   ele (é o mesmo achado do item 9d desta sessão, só que disparado por um caminho
   diferente).

   **Corrigido** em `ClientSession::set_in_world` (`session.rs`) — único lugar que essa
   função é chamada (`gateway.rs`, ao processar `SelectRole`), então reseta a flag bem
   no início de cada personagem, cobrindo automaticamente qualquer futura troca.

   ### b. Visibilidade entre jogadores — implementada

   Retomando o plano do item 17: o cliente, ao receber `PLAYER_ENTER_WORLD` de outro
   jogador, pede de volta `PlayerBaseInfo` (91) e, se ainda faltar, `GetCustomData`
   (116) — sem raça/classe/gênero/nome (que não vêm em `info_player_1`) o avatar nunca
   materializa.

   **Achado que simplificou a implementação**: lendo `CECElsePlayer::Init`/
   `OnMsgPlayerBaseInfo`/`OnMsgPlayerCustomData` (`EC_ElsePlayer.cpp`,
   `EvolvedPWClient`), o modelo só aparece quando `IsBaseInfoReady() &&
   IsCustomDataReady() && IsEquipDataReady()` — mas:
   - `IsEquipDataReady()` **já nasce `true`** na primeira criação do objeto (`Init` com
     `!bReInit` copia `crc_e` do pacote recebido pra dentro do próprio cache antes de
     comparar os dois, então a comparação é sempre contra si mesmo na entrada). Ou seja,
     **`CmdGetOtherEquip` não bloqueia o avatar aparecer** — fica de fora por ora, sem
     prejuízo pro objetivo desta rodada.
   - `IsCustomDataReady()` fica `true` mesmo com `custom_data` **vazio**
     (`base.custom_data.size() < 4` cai num `else` que marca pronto sem processar nada)
     — não precisa implementar uma aparência customizada real pro avatar aparecer.
   - `IsBaseInfoReady()` só fica `true` se `base.name` **não for vazio**
     (`OnMsgPlayerBaseInfo` retorna cedo, sem setar nada, se o nome vier vazio) — este é
     o único campo realmente crítico.

   Ou seja: implementar só `PlayerBaseInfo`/`PlayerBaseInfo_Re` corretamente já é
   suficiente pro avatar aparecer. `GetCustomData`/`GetCustomData_Re` foi implementado
   também (o cliente pode pedir, mesmo não sendo estritamente necessário) — reaproveita
   os mesmos bytes crus de `custom_data` do banco.

   ### c. O que foi adicionado

   - **`pw-protocol`**: opcodes `PlayerBaseInfo`(91)/`_Re`(92) e
     `GetCustomData`(116)/`_Re`(117), conferidos contra o IR (`opcodes_contra_o_ir`).
     `C2SPlayerBaseInfo`/`C2SGetCustomData` (decode: `roleid, localsid, playerlist` —
     `IntVector`). `S2CPlayerBaseInfoRe` (a struct `GRoleBase` inteira, 20 campos —
     campos sem sistema implementado ainda vão vazios/zerados: `config_data`,
     `custom_stamp`, `forbid` — ver a nota abaixo —, `help_states`, `spouse`, `userid`,
     `cross_data`, `reserved2-4`) e `S2CGetCustomDataRe`.
   - **`pw-storage`**: `CharacterRepository::get_public_info(role_id, realm_id)` — busca
     leve e **sem** filtro de `account_id` (a visibilidade é entre contas diferentes),
     e **sem** os efeitos colaterais de `get_details` (que carrega inventário/skills/
     missões e cria dados padrão pra personagem novo — coisas que fazem sentido pra "eu
     entrando no jogo", não pra "vi alguém passar"). `CharacterPublicInfo`: id, name,
     race, cls, gender, custom_data (bytes crus da coluna, sem reencapsular em JSON),
     created_at, updated_at.
   - **`gateway.rs`**: os dois handlers, um `PlayerBaseInfo_Re`/`GetCustomData_Re` por
     id pedido (mesmo padrão de `QUERY_PLAYER_INFO_1` no `pw-gs` — uma resposta por
     alvo). Personagem não encontrado responde `retcode=1` mas ainda com o
     `other_role_id` certo (o cliente lê esse campo mesmo em erro, pra saber a quem a
     resposta se refere).

   **Achado no caminho, pelo próprio framework de teste**: `campos_contra_o_ir.rs`
   (o mesmo que pegou os bugs do item 18) acusou **dois** erros na minha primeira
   tentativa de `S2CPlayerBaseInfoRe` antes de eu sequer rodar contra o cliente —
   faltava o campo `delete_time` inteiro (a struct real do IR tem 20 campos, um a mais
   do que eu tinha contado de cabeça) e o `forbid` (lista de punições) precisava do
   formato do item por extenso no código, não só um `write_compact_uint(0)` solto — o
   framework audita a FORMA da escrita, não só a quantidade. As duas falhas foram
   pegas e corrigidas **antes** de ir pro cliente, exatamente o que este framework foi
   construído pra fazer.

   `cargo test --workspace`: `campos_contra_o_ir` confere agora **36 pacotes, 257
   escalares**. Só as duas falhas pré-existentes de sempre (`elements.data v55`,
   `npcgen` v5/6 do 1.2.6), nada novo. Realms reconstruídos.

   ### d. Limitações sabidas (documentadas, não escondidas)

   - Continua sem grade espacial (`LinkGateway::jogadores_visiveis`, item 15/16): todo
     jogador do mesmo link vê todos os outros, não só os próximos.
   - `CmdGetOtherEquip` não implementado — o avatar aparece, mas sem os itens
     equipados visíveis (arma/armadura no modelo). Não bloqueia o objetivo desta
     rodada; fica pra quando alguém notar falta disso em jogo.
   - `forbid`/`help_states`/`spouse`/`userid`/`cross_data` sempre vazios — nenhum
     sistema de punição, vínculo ou cross-server implementado ainda.

   ### e. Próximo passo

   Testar com os dois clients na MESMA porta/realm (os testes anteriores usaram
   BR→29004 e EN→29003, realms diferentes — pra ver os dois SE VEREM, os dois precisam
   estar no mesmo realm). Depois: confirmar que trocar de personagem na mesma conexão
   também funciona limpo agora (item a). Pendências do item 17 continuam de pé: Portal
   da Cidade / sistema de sessão de skill é o próximo item de maior escopo.

20. **Sessão 2026-09-04 (continuação 5): troca de personagem e saída CONFIRMADOS
   funcionando em jogo. Visibilidade entre jogadores, chat e pedido de amizade —
   testados e NÃO funcionaram. Diagnóstico parcial, logs de instrumentação
   adicionados, e um achado real (não hipótese): chat sofre do MESMO bug
   arquitetural que a visibilidade tinha antes do item 19.**

   ### a. Confirmado em jogo

   Murillo testou com o client BR: trocar de personagem na mesma conexão (item 19a) e
   sair do jogo — os dois funcionaram, sem a mensagem de desconexão. **Os itens 18 e
   19a estão fechados e validados**, não só testados por unidade.

   ### b. Testado e NÃO funcionou: visibilidade entre jogadores

   Duas contas, dois personagens (classe sacerdote/Cleric) na Cidade das Plumas ao
   mesmo tempo — nenhum viu o outro. A implementação do item 19b (`PlayerBaseInfo`/
   `PlayerBaseInfo_Re`) passa no teste automático contra o IR (`campos_contra_o_ir.rs`,
   36 pacotes/257 escalares), mas isso só prova que o BYTE A BYTE bate com o
   protocolo — não prova que o fluxo end-to-end dispara.

   **Sem log suficiente pra saber onde a cadeia quebrou** (o código de
   `LinkGateway::jogadores_visiveis`, os handlers de `PlayerBaseInfo`/`GetCustomData` —
   nenhum tinha `info!`/`debug!` até agora). Adicionados nesta sessão, antes de
   qualquer tentativa de conserto às cegas:

   - No bloco "10.7 Outros jogadores online" (`gateway.rs`, dentro do `EnterWorld`):
     `info!("visibilidade: personagem {} entrando — {} outro(s) já em
     jogadores_visiveis: {:?}", ...)` — confirma se o mapa realmente tem o outro
     jogador registrado no momento em que o segundo entra.
   - No handler `InboundPacket::PlayerBaseInfo`: `info!("PlayerBaseInfo pedido pelo
     personagem ID {} pra {:?}", ...)` — confirma se o CLIENTE chegou a pedir (prova
     que recebeu `PLAYER_ENTER_WORLD` e tentou resolver quem é o outro).
   - Mesma coisa em `InboundPacket::GetCustomData`.

   **Prova que o mecanismo já disparou pelo menos uma vez, num teste ANTERIOR a esta
   sessão** (antes do handler de `PlayerBaseInfo` existir): o log da sessão anterior
   tinha `Opcode C2S desconhecido ou não tratado: 0x5B (Dec: 91, ...)` — **91 é
   `PlayerBaseInfo`**. Ou seja, em algum teste passado o cliente RECEBEU um
   `PLAYER_ENTER_WORLD` de outro jogador e tentou perguntar quem ele era — a ponta de
   envio (`jogadores_visiveis` → `player_enter_world`) já funcionou pelo menos uma vez.
   O que não se sabe ainda é se isso se repetiu no teste desta sessão (rodado ANTES da
   instrumentação de log subir) — é o primeiro coisa a olhar no próximo teste.

   **Hipótese mais forte, ainda não confirmada**: `jogadores_visiveis` só é limpo no
   fim da conexão (`gateway.rs`, bloco de cleanup perto da linha 320) — **nunca** no
   `LOGOUT` tipo "meia saída" (voltar à seleção de personagem). Se qualquer um dos dois
   personagens do teste passou por uma troca de personagem antes (comum, já que o
   teste anterior na mesma sessão validou exatamente isso), pode ter ficado uma entrada
   fantasma com o `role_id` **antigo** ocupando o registro, ou simplesmente não afeta o
   caso simples de "duas contas novas, cada uma loga uma vez" — precisa reproduzir com
   log pra saber.

   ### c. Testado e NÃO funcionou: chat — achado real, não hipótese

   Lendo `InboundPacket::PlayerChat` (`gateway.rs`): o broadcast do chat
   (`OutboundPacket::ChatBroadcast`) é mandado só via `tx.send(...)` — **o canal de
   saída da PRÓPRIA sessão**. Não itera por ninguém mais. Isto é **pré-existente**
   (não é regressão desta sessão) e está no mesmo padrão já documentado pro
   `PlayerMoveBroadcast` ("hoje só ecoa pro remetente").

   **Isto é a MESMA classe de bug que a visibilidade tinha antes do item 19** — e a
   correção é direta, porque a infraestrutura já existe: `LinkGateway::jogadores_visiveis`
   guarda o `envio: EnvioAoCliente` de cada personagem online neste link. O handler de
   chat pode iterar essa mesma coleção e mandar o `ChatBroadcast` pra cada um (`try_send`,
   mesmo padrão do `player_enter_world`), em vez de só ecoar pro remetente. Provavelmente
   vale generalizar isto num método auxiliar (`LinkGateway::broadcast_para_todos(pacote)`)
   e reaproveitar tanto pro chat quanto, mais adiante, pro `PlayerMoveBroadcast`.

   ### d. Testado e NÃO funcionou: pedido de amizade — feature inexistente, não bug

   `InboundPacket::GetFriendList` sempre responde listas vazias (`groups`, `friends`,
   `status`) — comentário no código já avisa "a lista de amigos ainda não vem do
   armazenamento". **Não existe** handler de "mandar pedido de amizade" nem de
   "responder pedido" em lugar nenhum do `gateway.rs`/`pw-gs`. Isto não é um bug de
   layout como os outros desta sessão — é feature 0% implementada (sem tabela no
   banco, sem opcode C2S mapeado pra pedido). Precisa de: achar o opcode GNET certo no
   IR (não investigado ainda), desenhar o schema (tabela de amizades, provavelmente em
   `specs/01_DATABASE_SCHEMA_POSTGRES.sql`), e o mesmo padrão de broadcast do item c
   (avisar o outro jogador do pedido, se ele estiver online).

   ### e. Estado do deploy

   Logs de instrumentação (item b) já compilados, testados (`cargo test -p pw-protocol
   -p pw-link` verde) e implantados nos realms 155/155BR. **Não houve tentativa de
   conserto** da visibilidade/chat/amizade nesta sessão — só diagnóstico e
   instrumentação, de propósito: sem saber ainda qual das hipóteses do item b é a
   causa real, um conserto agora seria chute — contra o princípio do projeto.

   ### f. Próximo passo, em ordem

   1. **Reproduzir o teste de visibilidade com os logs novos.** Duas contas, mesmo
      realm, perto uma da outra. Olhar `docker logs pw-realm-155br` atrás de:
      - `visibilidade: personagem X entrando — N outro(s) já em jogadores_visiveis`
        (confirma se o segundo jogador VÊ o primeiro no momento de entrar)
      - `PlayerBaseInfo pedido pelo personagem ID Y pra [...]` (confirma se o CLIENTE
        recebeu `PLAYER_ENTER_WORLD` e tentou perguntar quem é o outro)
      Se a primeira linha não aparecer com `N >= 1`: o bug está em `jogadores_visiveis`
      nunca sendo populado a tempo (investigar a hipótese do `LOGOUT` tipo "meia saída"
      não limpando/a ordem de entrada). Se aparecer mas a segunda linha não: o
      `PLAYER_ENTER_WORLD` está sendo mandado mas o cliente não está processando —
      aí sim vale desmontar o pacote de novo, ou checar o `EC.log` do cliente por
      "Decode error" (mesma técnica do item 18).
   2. **Chat**: implementar o broadcast de verdade, reaproveitando
      `jogadores_visiveis` — é a correção mais rápida e certa desta lista inteira,
      porque a causa já está confirmada, não só suspeitada.
   3. **Pedido de amizade**: feature nova — desenhar antes de codar (schema +
      opcode do IR).
   4. Depois disso, os itens mais antigos que continuam em aberto: sistema de sessão
      de skill (Portal da Cidade, item 17c), `world.players` nunca populado em
      produção (item 17.d.2 — trava IA de monstro e a visibilidade com raio de
      verdade), clique-pra-andar e teleporte de GM (itens 15.d.3/d.4, precisam de
      captura).

21. **Sessão 2026-09-04 (continuação 6): teste de 2 contas do Murillo lido pelos logs do
    item 20 — visibilidade ainda quebrada mas por motivo diferente do suposto, chat e
    movimento CORRIGIDOS (mesma causa, aplicado), e achada a causa raiz de "convite de
    grupo chega mas o grupo não se forma" — o MESMO bug arquitetural de sempre
    (`world.players` nunca populado), num lugar novo.**

    ### a. O que o Murillo testou e reportou

    Duas contas no client BR (mesmo install, duas instâncias), mesma cidade: um
    personagem viu só a "caixa de colisão" do outro (sem o modelo carregar); o outro não
    viu nada. Movimento não sincroniza. Chat não funciona. Pedido de grupo: quem viu a
    caixa de colisão mandou convite, o outro recebeu a caixa de convite, mas o grupo não
    se formou.

    ### b. Visibilidade: os logs do item 20 provam que o SERVIDOR fez a parte dele nos
    dois sentidos

    `docker logs pw-realm-155br` (instrumentação do item 20) pro teste relatado:

    ```
    visibilidade: personagem 42 entrando — 0 outro(s) já em jogadores_visiveis: []
    visibilidade: personagem 40 entrando — 1 outro(s) já em jogadores_visiveis: [42]
    PlayerBaseInfo pedido pelo personagem ID 42 pra [40]   <- 42 recebeu PLAYER_ENTER_WORLD de 40 e perguntou
    PlayerBaseInfo pedido pelo personagem ID 40 pra [42]   <- 40 recebeu PLAYER_ENTER_WORLD de 42 e perguntou
    ```

    As duas consultas SQL de `get_public_info` (uma pra role 40, outra pra role 42)
    devolveram `rows_returned: 1` — ou seja, o servidor achou os dois personagens e
    respondeu `PlayerBaseInfoRe` com `retcode=0` e nome preenchido **nos dois sentidos**.
    Nenhum `WARN`/`ERROR` no log inteiro da janela do teste. Ou seja: **o mecanismo de
    entrada e a resposta de `PlayerBaseInfo` dispararam certo nos dois sentidos** —
    diferente da hipótese mais forte do item 20 (`jogadores_visiveis` não populado a
    tempo), que fica descartada por este teste.

    ### c. Por que não dá pra confirmar o lado do cliente ainda — falha de instrumentação
    do teste, não do código

    As duas contas rodaram da MESMA pasta de instalação (`F:\PW\1.5.5\1.5.5 BR\...`) —
    só existe **um** `EC.log` nessa árvore inteira (confirmado por busca no disco). Ele
    tem só uma sessão completa (a do personagem 42, criada 14:47:03, fechada 14:50:12
    "Active close", sem nenhum "Decode error"), porque a segunda instância do cliente
    (personagem 40, "HEal") nunca conseguiu gravar a própria sessão nesse arquivo —
    provavelmente por causa de lock de arquivo entre os dois processos do mesmo binário.
    **Não temos visibilidade do que o cliente do personagem 40 realmente fez** com a
    resposta que o servidor mandou pra ele.

    Como o log que sobreviveu (personagem 42) não mostra nenhum "Decode error", a
    hipótese mais provável agora é: o personagem 42 processou a resposta sobre 40
    corretamente (raça/classe/gênero certos — é ele quem deve ter visto "só a caixa de
    colisão sem modelo", OU é ele quem viu tudo certo e o problema está do lado de 40,
    não dá pra saber sem o `EC.log` de 40).

    **Próximo teste precisa rodar os dois clientes de pastas SEPARADAS** (copiar a pasta
    `Perfect World 1.5.5 BR` inteira pra um segundo diretório antes de abrir a segunda
    instância) — só assim cada lado grava seu próprio `EC.log` e dá pra aplicar a mesma
    técnica que resolveu os itens 9-18 ("Decode error N" → opcode N no `gnet_155.json`).
    Sem isso, qualquer conserto de visibilidade agora seria chute.

    ### d. Chat e movimento — CORRIGIDOS (mesma causa do item 20c, aplicada)

    Confirmado exatamente o que o item 20c já tinha diagnosticado: `InboundPacket::
    PlayerChat` e `InboundPacket::PlayerMove` (`crates/pw-link/src/gateway.rs`) só
    mandavam o broadcast pro `tx` da própria sessão — nunca pros outros jogadores.
    Corrigido com um método novo, `LinkGateway::broadcast_para_todos`, que itera
    `jogadores_visiveis` (a mesma lista que já serve `PLAYER_ENTER_WORLD`/
    `PLAYER_LEAVE_WORLD`) e manda por `try_send` pro `envio` de cada um — incluindo o
    próprio remetente, que está na lista desde o `EnterWorld`, então o autoeco continua
    sem precisar de um `tx.send` separado. `PlayerMove` também passou a atualizar
    `jogadores_visiveis[role_id].pos` a cada movimento, corrigindo de brinde a limitação
    documentada no item 16b ("um jogador já visível não se move na tela de quem já o
    viu" — só valia pra quem entrasse DEPOIS; agora a posição guardada fica atual).

    Mesma limitação sabida de `jogadores_visiveis` (sem grade espacial, todo mundo do
    link recebe, documentado no próprio método). `cargo test -p pw-link -p pw-protocol
    -p pw-gs` — tudo verde (85 testes, nenhuma regressão). Realms 155/155BR
    reconstruídos.

    ### e. Grupo: causa raiz achada — o MESMO bug de `world.players` vazio, achado num
    lugar novo

    O pedido de convite (`TEAM_INVITE`, 27) e a caixa de convite chegando ao convidado
    já funcionam — confirmado no relato do Murillo e coerente com o código
    (`BusServer::convidar`, `pw-gs/src/bus_server.rs`): usa `enviar_ao_jogador`, que
    roteia pelo barramento por `roleid`, sem depender de `WorldInstance::players`.

    O que quebra é **depois** de aceitar. `BusServer::aceitar_grupo` manda dois pacotes
    pra cada membro: `TEAM_JOIN_PARTY` (61, "você entrou num grupo") e
    `TEAM_MEMBER_DATA` (a lista de membros com HP/nível/etc). O segundo usa
    `WorldInstance::dados_dos_membros`:

    ```rust
    pub fn dados_dos_membros(&self, membros: &[RoleId]) -> Vec<MembroDoGrupo> {
        membros.iter().filter_map(|m| {
            let p = self.players.get(&(*m as i64))?;   // <- None pra QUALQUER m
            ...
        }).collect()
    }
    ```

    `filter_map` descarta silenciosamente qualquer `m` que não esteja em
    `self.players` — e `WorldInstance::add_player` **nunca é chamado em produção**
    (achado já documentado nos itens 16.d.2/17.d, mas nunca antes com uma consequência
    tão visível: até aqui só afetava a IA de monstro, que não tem UI pra denunciar o
    problema). Resultado: `dados_dos_membros` sempre devolve uma lista **vazia**, pros
    dois membros, sempre. O cliente recebe "você entrou num grupo" e, logo em seguida,
    uma lista de membros com zero entradas — o que bate exatamente com "o convite chega,
    mas o grupo não se forma" (nenhum membro aparece na janela de grupo, incluindo o
    próprio jogador).

    **Não corrigido ainda** — o remendo rápido (inserir um `PlayerEntity` mínimo em
    `world.players` só quando o grupo se forma) seria gambiarra sobre gambiarra: o dado
    ficaria stale (HP/nível nunca atualizam depois) e o problema de fundo continuaria
    intocado. A correção de verdade é a mesma que os itens 16.d.2/17.d já apontavam como
    a maior pendência arquitetural: popular `WorldInstance::players` de verdade no
    `EnterWorld`.

    **Por que isso ainda não foi feito, medido nesta sessão**: `BusMessage::EnterWorld`
    (`crates/pw-bus/src/message.rs`) carrega só `roleid, provider_link_id, locktime,
    timeout, settime, localsid` — espelho exato do opcode GNET 72 real, de propósito
    (`pw-link` já tem `details`, o personagem completo com conta e tudo, no momento do
    `EnterWorld`; o `pw-gs` não tem nada disso, só o `roleid`). Duas saídas, nenhuma
    tentada ainda porque é decisão de arquitetura, não ajuste de layout:
    1. Uma mensagem de barramento **nova**, interna (não espelha nenhum opcode GNET real
       — o barramento já tem espaço pra isso, é só documentar que não é 1:1 com o fio do
       cliente), carregando o snapshot do personagem que `pw-gs` precisa pra
       `add_player`. Mandada pelo `pw-link` logo depois do `EnterWorld` de hoje.
    2. `pw-gs` buscar o personagem sozinho a partir só do `roleid` — exige um método de
       repositório sem filtro de `account_id` (o `get_public_info` do item 19c já é
       quase isso, mas falta id do mundo/posição salva) e um acesso a Postgres de dentro
       do `pw-gs` que hoje só existe pra autosave.

    Resolver isto de vez destrava, de um golpe só: o roster de grupo (este item), IA de
    monstro perseguindo/atacando jogador de verdade (item 16.d.2), e visibilidade com
    raio real em vez do atalho de `jogadores_visiveis` (item 16b) — é o item de maior
    valor da lista inteira, mas é grande o bastante pra merecer decisão do Murillo antes
    de começar, não só um "vou fazendo".

    ### f. Próximo passo, em ordem

    1. **Repetir o teste de visibilidade com os dois clientes em pastas separadas** —
       único jeito de aplicar a técnica de "Decode error" nos dois lados (item c).
    2. **Confirmar chat e movimento em jogo** — corrigidos nesta sessão (item d), não
       testados em jogo ainda.
    3. **Decisão do Murillo**: encarar agora a arquitetura de `world.players`
       (item e) — resolve grupo + IA de monstro + visibilidade com raio de uma vez — ou
       adiar de novo e seguir só com os remendos locais do `pw-link`.
    4. Sistema de sessão de skill (Portal da Cidade, item 17c) e captura de
       clique-pra-andar/teleporte de GM (itens 15.d.3/d.4) continuam de pé, sem mudança.

22. **Sessão 2026-09-04/05 (continuação 7): teste com os dois clients em pastas
    SEPARADAS (o que o item 21 pedia) — achada e corrigida a causa raiz de "só a caixa
    de colisão aparece, o modelo nunca carrega", lendo o fonte do client linha a linha,
    não suposição.**

    ### a. O teste

    Murillo rodou dois clients de pastas diferentes (`E:\0_GAMES\PW1.5.5\element` e
    `F:\PW\1.5.5\1.5.5 BR\...\element`), duas contas, mesmo `realm_155BR`, personagens
    no mesmo lugar. Desta vez os dois `EC.log` sobreviveram inteiros (um por pasta).
    **Nenhum dos dois mostrou "Decode error"** — os pacotes chegaram e foram decodificados
    certos dos dois lados. O log do realm confirmou os dois sentidos de `PlayerBaseInfo`
    pedidos e respondidos com sucesso (consulta SQL achou o personagem, `retcode=0`,
    nos dois casos). Ainda assim, os personagens não se viram nem interagiram.

    ### b. A causa raiz, achada em `EC_ManPlayer.cpp`/`EC_ElsePlayer.cpp`
    (`F:\PW\1.5.5\EvolvedPWClient`)

    O modelo 3D de outro jogador só é criado quando **três** flags são `true` ao mesmo
    tempo (`EC_ElsePlayer.cpp:671`): `IsBaseInfoReady() && IsCustomDataReady() &&
    IsEquipDataReady()`. As duas primeiras já funcionavam (item 19). A terceira só vira
    `true` dentro de `ChangeEquipments` (`EC_ElsePlayer.cpp:1671`), chamada ao processar
    `EQUIP_DATA`/`EQUIP_DATA_CHANGED` — que só chegam em resposta a `GetOtherEquip`
    (opcode C2S 33), que o cliente manda sozinho ao ver outro jogador pela primeira vez
    (`EC_ManPlayer.cpp:358`, `if (!pPlayer->IsEquipDataReady())
    pSession->c2s_CmdGetOtherEquip(...)`) — confirmado no log do realm (`cmd=33` chegando
    pelos dois lados). **Esse comando nunca teve handler nenhum** — caía no braço
    `outro =>` genérico de `BusServer::tratar_subcomando` (`pw-gs/src/bus_server.rs`),
    só logado e ignorado. Sem resposta, `IsEquipDataReady()` nunca vira `true`, e o
    modelo nunca é criado — só a entidade/colisão (criada incondicionalmente em
    `ElsePlayerEnter`) existe. Bate exatamente com o sintoma relatado, e explica os dois
    lados: "só a caixa de colisão" e "nada" são o mesmo bug visto de formas diferentes
    (a colisão em si é invisível — não há um retângulo desenhado — então um lado pode
    ter esbarrado nela sem perceber o quê, e o outro nem isso).

    ### c. A correção implementada

    - **`crates/pw-protocol/src/packets/s2c.rs`**: `S2CGamedataSend::equip_data` — layout
      do **1.2.6/1.5.3**: `crc(u16), idPlayer(i32), mask(i64), data[n](i32)`, 14 bytes de
      prefixo. Confirmado por **duas fontes independentes**: o IR do 1.5.3
      (`gamedata_153.json`, `S2C::cmd_equip_data`, campo `data` no deslocamento 14) e a
      captura real do 1.2.6 (`docs/MEDIDAS_DO_126.md`, comando 66: tamanhos `14×2, 18×1,
      22×2, 62×1, 66×2` — exatamente `14 + n×4`).
    - **`crates/pw-protocol/src/por_versao.rs`**: `PorVersao::equip_data` acrescenta um
      `color_name(u32)` na frente pro 1.5.5 — campo que existe em
      `S2C::cmd_equip_data` (`EC_GPDataType.h:2016`, cliente 1.5.5) mas não no IR do
      1.5.3 nem na captura do 1.2.6. Mesmo padrão de outros campos que só o 1.5.5
      acrescenta (`self_info_1`, item 9a).
    - **`crates/pw-gs/src/comandos.rs`**: `ids::GET_OTHER_EQUIP = 33`, reaproveitando o
      decodificador `ConsultaDeIds` (já existia pro 67/`QUERY_PLAYER_INFO_1` — mesmo
      formato de fio: `size(u16)` + lista de `i32`).
    - **`crates/pw-gs/src/bus_server.rs`**: novo handler `equipamento_de_outro` — responde
      `EQUIP_DATA` com `mask=0` (nenhum item) pra cada id pedido.

    **Por que `mask=0` é suficiente, não um atalho escondido**: `ChangeEquipments`
    marca `m_bEquipReady = true` **incondicionalmente** quando `bReset` é `true` — e
    `EQUIP_DATA` sempre chama com `bReset=true` (`EC_ElsePlayer.cpp:1942`). O conteúdo
    da máscara não importa pra desbloquear o modelo, só a resposta existir. **Limitação
    sabida e documentada no código**: com `mask=0` o avatar aparece sem arma/armadura
    visível — equipar de verdade pede decifrar como `data[i]` empacota item/modelo/refino
    por slot (só confirmado pro slot especial `EQUIPIVTR_GOBLIN`, `EC_ElsePlayer.cpp:
    1723-1725`), fica pra quando o visual de equipamento entre jogadores for prioridade.

    `cargo test -p pw-protocol -p pw-gs -p pw-link` — verde (mais uma entrada em
    `INTENCAO`, `subcomandos_s2c_contra_o_ir.rs`, pro novo codificador não escapar da
    auditoria). Realms 155/155BR (link e mundo) reconstruídos e redeployados —
    aguardando confirmação em jogo do Murillo.

    ### d. O que já foi corrigido nesta sessão e ainda não foi testado em jogo

    - Chat e movimento entre jogadores (item 21d, sessão anterior) — não exercitados
      neste teste (o Murillo não tentou conversar nem andar perto o bastante pra notar).
    - `GetOtherEquip`/modelo do outro jogador (este item) — pronto, aguardando reteste.

    ### e. Próximo passo

    1. **Reteste completo**: duas contas, pastas separadas, mesma cidade — conferir se
       o modelo do outro jogador agora carrega, se o movimento sincroniza e se o chat
       funciona (os três já deveriam estar corrigidos).
    2. Se o modelo aparecer mas congelado/sem animação de movimento ao lado do outro
       jogador se mover, ou se `EC.log` mostrar algo nesse ponto, capturar de novo — não
       investigado ainda.
    3. Itens antigos sem mudança: grupo (`world.players` vazio, item 21e — decisão de
       arquitetura pendente), sistema de sessão de skill (item 17c), clique-pra-andar e
       teleporte de GM (itens 15.d.3/d.4).

23. **Sessão 2026-09-05: achada a causa raiz real de "nada sincroniza entre
    jogadores" — um mecanismo inteiro (`PlayerMoveBroadcast`) que nunca funcionou
    porque usa um opcode GNET que não existe. Corrigido movimento e a animação de
    conjuração de skill com o comando real do protocolo. Equipamento visual
    (`GetOtherEquip`, item 22) ainda não confirmado — relato do Murillo foi
    contraditório entre duas mensagens.**

    ### a. O teste do Murillo, e o porquê de reabrir a investigação do item 22

    Depois do fix do item 22, o Murillo relatou em duas mensagens diferentes,
    aparentemente contraditórias: primeiro que "consegui em cada um clicar no modelo 3d
    do outro" (depois de voltar à seleção de personagem e reentrar), depois — no mesmo
    teste, respondendo a uma pergunta de acompanhamento — que "nada é sincronizado, nem
    modelo 3d, nem movimentos, nem skills". A leitura mais provável: dava pra
    **selecionar** o espaço onde o outro personagem estava (a colisão, sempre criada
    incondicionalmente por `ElsePlayerEnter`), não que o modelo 3D real tivesse
    carregado — clicar num espaço vazio ainda seleciona a entidade lá. Não confirmado
    ainda; ver item e.1.

    ### b. Captura de tráfego real, com o Murillo testando ao vivo

    Como as duas contas estavam abertas e o Murillo se ofereceu pra manter o teste
    rodando, liguei um `tcpdump` (container `nicolaka/netshoot`, `--network
    container:pw-realm-155br`, porta 29004) para capturar o tráfego real enquanto ele
    repetia o teste. A captura confirmou que o link é criptografado (RC4) depois do
    `KeyExchange` — `pw-pcapdiff` avisa isso na própria mensagem de uso ("sem --interno,
    só é legível até o KeyExchange") — então ler os bytes da captura exigiria extrair a
    chave de sessão. Em vez disso, troquei de método: instrumentei o próprio codec de
    saída (`crates/pw-protocol/src/codec.rs`, dentro do `Encoder::encode`) para logar o
    payload **em texto claro, antes de qualquer criptografia** — mesmo formato do log
    que já existia pro sentido cliente→servidor ("Gamedata recebido do cliente"), agora
    também pro sentido servidor→cliente ("Gamedata enviado ao cliente"). Isto lê o que o
    servidor realmente decidiu mandar, sem depender de decifrar nada.

    ### c. A causa raiz: um mecanismo inteiro morto dos dois lados

    Enquanto isso rodava, reli `crates/pw-link/src/gateway.rs` procurando por que
    `InboundPacket::PlayerMove`/`OutboundPacket::PlayerMoveBroadcast` (o "fix" do item 21d
    desta mesma sessão) não parecia surtir efeito. Achei dois problemas, cada um
    suficiente sozinho pra explicar o silêncio total:

    1. **`InboundPacket::PlayerMove` nunca é produzido pelo decodificador de verdade.**
       `PwPacketCodec::decode` (`crates/pw-protocol/src/codec.rs`) sempre entrega
       `PLAYER_MOVE` (subcomando C2S 0) dentro de `InboundPacket::GamedataSend` — o
       opcode GNET 34 (`OP_C2S_GAMEDATASEND`) é o único caminho, sem exceção. O `match`
       que eu "corrigi" no item 21d (`InboundPacket::PlayerMove(move_pkt) => {
       self.broadcast_para_todos(...) }`) é código morto: nenhum tráfego real chega
       nele. O movimento de verdade é tratado só em `gateway.rs:1033`
       (`InboundPacket::GamedataSend`), que repassa pro `pw-gs` e — até esta sessão —
       nunca respondia nada a ninguém.
    2. **`OutboundPacket::PlayerMoveBroadcast` usa um opcode GNET (33) que não existe.**
       Conferido diretamente contra `specs/protocol/gnet_155.json`:
       `33 in [p['id'] for p in protocols]` → `False`. Não é um opcode "ainda não
       confirmado" — é **inventado**, não corresponde a protocolo real nenhum. Mesmo se
       o caminho 1 acima não fosse morto, o cliente de verdade não teria decodificador
       pra ele.

    Ou seja: o mecanismo de sincronização de movimento inteiro (criado antes desta
    sessão, e que o item 21d achou ter corrigido) nunca funcionou — nem podia, dos dois
    lados. O chat (item 21d) é diferente e continua válido: usa `PROTOCOL_CHATBROADCAST`,
    um opcode GNET real, decodificado direto pelo codec (`OP_C2S_CHAT`/
    `InboundPacket::PlayerChat`), fora do `GamedataSend` — por isso o mesmo tipo de fix
    funciona lá mas não pro movimento.

    ### d. O que corrigi: os comandos reais do protocolo, com broadcast pra quem mais
    está no mundo

    Achei o comando real via `docs/MEDIDAS_DO_126.md` e `specs/protocol/gamedata_155.json`:
    `OBJECT_MOVE` (S2C 15) — **o comando mais frequente de toda a captura do 1.2.6**
    (17294 ocorrências em 22 minutos), 21 bytes, idêntico no 1.2.6 e no 1.5.3+.
    `OBJECT_STOP_MOVE` (S2C 35), mesma família, 20 bytes, também idêntico nas duas
    versões (2986 ocorrências). Nenhum dos dois tinha codificador nosso.

    - **`crates/pw-protocol/src/packets/s2c.rs`**: `S2CGamedataSend::object_move`/
      `object_stop_move`, layout confirmado por duas fontes (IR do 1.5.3 + captura do
      1.2.6, mesmo tamanho nas duas).
    - **`crates/pw-protocol/src/por_versao.rs`**: `PorVersao::object_move`/
      `object_stop_move` — sem divergência de versão conhecida, delegam direto (mesmo
      padrão de outros comandos sem `PorVersao` real).
    - **`crates/pw-gs/src/bus_server.rs`**: novo método `transmitir_a_outros(exceto,
      data)` — manda um pacote pra todo `roleid` em `self.sessoes` **menos** quem
      originou a ação (mesma limitação sabida de `jogadores_visiveis`: sem grade
      espacial, todo mundo do servidor recebe). `mover()` (`PLAYER_MOVE`) e `parar()`
      (`STOP_MOVE`) agora chamam isto depois de atualizar o mundo em memória.
    - **De brinde, a mesma causa pro "skills não sincronizam"**: `conjurar()`
      (`CAST_SKILL`/`CAST_INSTANT_SKILL`) já mandava `OBJECT_CAST_SKILL` (85) — o
      comando certo, confirmado idêntico no 1.2.6 (`docs/MEDIDAS_DO_126.md`) — mas só
      pra quem conjurou (`self.responder`), nunca pra quem está por perto. Agora
      também vai por `transmitir_a_outros`, junto com `SKILL_PERFORM` (88).

    **Limitação que continua de pé, não é desta correção**: nada disto ativa PvP —
    `conjurar()`/`atacar()` só calculam dano contra `mundo.monsters`, e alvo que é outro
    jogador sai em silêncio ("não é um monstro deste mundo"). O que foi corrigido é só a
    **visibilidade** do movimento/animação entre jogadores, que faltava mesmo pra ações
    contra monstro — item 17c (sistema de sessão de skill) e um futuro "PvP de verdade"
    continuam como itens de arquitetura nova, não deste fix.

    **Achado, mas não perseguido ainda** (fora do escopo desta rodada, mesma família):
    `HOST_START_ATTACK`(84)/`OBJECT_STARTATTACK`(22)/`ATTACK_ONCE`(83)/
    `OBJECT_ATTACK_RESULT`(120) — os comandos que fariam o ataque básico (não-skill)
    ser visível a observadores — nenhum implementado ainda. Mesma causa raiz, mesma
    correção (`transmitir_a_outros`), só que exige decodificar mais structs novos; não
    fiz para não arriscar byte errado sem tempo de conferir cada um contra o IR.

    `cargo test -p pw-protocol -p pw-gs -p pw-link` — verde (mais duas entradas em
    `INTENCAO`, `object_move`/`object_stop_move`). Realms 155/155BR reconstruídos e
    redeployados com tudo isto, mais a instrumentação de log do item b (útil pra
    qualquer investigação futura, não só esta).

    ### e. Próximo passo

    1. **Confirmar de verdade se o modelo do outro jogador carrega** (item a, ainda
       ambíguo) — pedir pro Murillo **selecionar/clicar** o outro personagem
       especificamente e descrever o que aparece (nome flutuante? barra de vida? só
       cursor mudando?), não só "consegui clicar".
    2. **Reteste de movimento e skill** com os fixes deste item — devem sincronizar
       agora. Se não sincronizarem, o log novo ("Gamedata enviado ao cliente") mostra
       exatamente o que saiu do servidor, byte a byte, pra comparar contra o `EC.log` do
       cliente (Decode error, se houver).
    3. Ataque básico (não-skill) visível a observadores — os quatro comandos
       encontrados no item d, ainda não implementados.
    4. Itens antigos sem mudança: grupo (`world.players` vazio, item 21e), sistema de
       sessão de skill/PvP de verdade (item 17c), clique-pra-andar e teleporte de GM
       (itens 15.d.3/d.4).

24. **Sessão 2026-09-05 (continuação): movimento confirmado sincronizando em jogo
    (item 23 funcionou). Modelo 3D continuava invisível — achada a causa raiz de
    verdade, cruzando a captura real do 1.2.6 com o código-fonte do cliente 1.5.5, e
    era diferente do que a sessão anterior support: `custom_data` vazio não é seguro,
    é o próprio bug.**

    ### a. O teste do Murillo e o pedido de usar a captura real

    Com os fixes do item 23 no ar, o Murillo confirmou: **movimento sincroniza** (as
    caixas de colisão se movem uma pra outra). Mas nome e modelo 3D continuavam
    ausentes — só a caixa de colisão, e clicar nela mostra o nome mas não o HP nem o
    modelo. Pediu pra eu não confiar só nos `.md` deste projeto e usar a captura real
    do 1.2.6 (`_sync/capturas/full_interno.pcap`, confirmada pelo Murillo como cobrindo
    **os dois elos internos em claro**: `glinkd↔gdeliveryd` na porta 29100 e
    `gs↔glinkd` na 29301) cruzada com o código-fonte do cliente 1.5.5.

    ### b. Uma armadilha da minha própria instrumentação, descoberta no caminho

    Reparsear a captura revelou que meu próprio log "Gamedata enviado ao cliente"
    (item 23b) tinha um bug: `encode_gamedata_send` (`crates/pw-protocol/src/
    adapter.rs`) escreve o payload como `Octets` — ou seja, com um **`CompactUINT` de
    tamanho antes do id do subcomando** — e meu log lia `payload[0..2]` direto, sem
    pular esse prefixo. Resultado: toda vez que o log dizia coisas como
    `cmd=52762`/`cmd=33027`, era esse prefixo sendo lido como se fosse o id. Corrigido
    em `codec.rs` (agora pula 1/2/4 bytes conforme a forma do `CompactUINT`, igual ao
    decodificador). **Consequência importante**: isso **não é o bug relatado** — só
    escondia a prova de que `EQUIP_DATA` (item 22) e `OBJECT_MOVE`/`OBJECT_STOP_MOVE`
    (item 23) já estavam sendo mandados **certos** o tempo todo. Confirmado
    reparseando os logs já coletados: `EQUIP_DATA` pra role 40 saiu com
    `idPlayer=40, mask=0` exatamente como o código pretende, duas vezes (uma por
    pedido). O item 22 nunca teve o bug que eu suspeitava — a investigação simplesmente
    não tinha como ver isso ainda.

    ### c. A causa raiz de verdade, achada na captura real

    Com o `pw-pcapdiff` e um leitor de frames GNET escrito para esta sessão, achei
    tráfego de `PlayerBaseInfo`(91)/`PlayerBaseInfo_Re`(92) real no elo
    `glinkd↔gdeliveryd` (porta 29100) — três contas reais (`FARM` e outras) pedindo
    info umas das outras. Decodifiquei um `PlayerBaseInfo_Re` real, campo a campo,
    contra a struct `GRoleBase` do IR (`specs/protocol/gnet_153.json`) — bateu
    **exatamente**, campo por campo, confirmando que nosso `S2CPlayerBaseInfoRe::encode`
    (incluindo o byte `_literal=1` do RPC marshalling) está certo. Mas um campo saltou
    aos olhos: **`custom_data` tinha 172 bytes reais**, nunca vazio.

    Isso bateu com uma dúvida que já tinha ficado da sessão anterior: o item 22 mandava
    `custom_data: Vec::new()` de propósito, raciocinando que "vazio é um caminho válido"
    porque `OnMsgPlayerBaseInfo` (cliente) não trava com `custom_data.size() < 4` — e
    isso é verdade **para não travar**, mas relendo com mais cuidado: nesse caso ele
    **pula `ChangeCustomizeData`** e `m_CustomizeData` (que carrega o `bodyID`) fica no
    valor padrão da construção. `LoadPlayerSkeleton(false)`
    (`EC_Player.cpp:2171`) então **enfileira** o carregamento do modelo numa thread de
    fundo (`QueueECModelForLoad`, só carrega direto se `bAtOnce` ou a thread não estiver
    pronta) — com um `bodyID` de lixo, e essa fila **falha em silêncio**: nenhuma
    flag de erro, nenhum log, o modelo simplesmente nunca aparece. Isso explica tudo:
    nome funciona (`IsBaseInfoReady`, só depende do nome não vir vazio),
    `IsCustomDataReady()`/`IsEquipDataReady()` ficam `true` (então o código nunca fica
    "esperando"), mas o carregamento assíncrono do modelo recebe dado inválido e morre
    silenciosamente — e por isso nunca aparecia erro nenhum, nem no `EC.log`, nem no
    log do servidor.

    **A prova de que os dados reais já existiam, só não eram usados**: `characters.
    custom_data` no Postgres já tem **176 bytes** pros dois personagens de teste (`HEal`,
    `testesacer`) — o mesmo dado que `write_role_info` já manda pro **próprio**
    personagem em `RoleList_Re`/`CreateRole_Re`. `CharacterRepository::get_public_info`
    já carregava esse campo (`CharacterPublicInfo::custom_data`) — só o
    `gateway.rs` descartava, com `Vec::new()` no lugar de `p.custom_data`.

    ### d. Corrigido

    `crates/pw-link/src/gateway.rs`, handler `InboundPacket::PlayerBaseInfo`: `custom_data:
    p.custom_data` no lugar de `Vec::new()`. Uma linha — o dado certo já estava carregado,
    só não usado. `cargo test -p pw-protocol -p pw-link -p pw-gs` verde. Realms
    reconstruídos.

    ### e. Próximo passo

    1. **Reteste**: duas contas, mesma cidade — conferir se o modelo 3D agora carrega
       (nome e colisão já funcionavam; a expectativa é que o modelo apareça com a
       aparência customizada real, não um padrão genérico).
    2. Se AINDA não aparecer: usar `EQUIP_DATA`/`OBJECT_MOVE` como prova de que o
       mecanismo geral (bus → link → cliente) funciona — o log corrigido do item b
       agora é confiável pra comparar contra o `EC.log` byte a byte.
    3. Itens sem mudança: grupo, sessão de skill/PvP, clique-pra-andar/teleporte de GM.

25. **Sessão 2026-09-05 (continuação): a hipótese de distância (item 24e) foi
    DESCARTADA por teste real. Achado um bug de dados genuíno e isolado —
    `realm_155BR/config/world/region.sev`/`precinct.sev` não eram cópias de
    `realm_155`, e o cliente avisava disso na própria tela. Corrigido. Ainda não
    confirmado se resolve o modelo 3D.**

    ### a. Duas correções ao registro da sessão anterior

    1. **Skills não estão confirmadas funcionando** — o item 24 escreveu isso por
       engano (confundiu com o item 23, que só cobre movimento). O Murillo corrigiu:
       ele **não consegue nem usar as skills**, então esse item continua **totalmente
       em aberto** (bate com o diagnóstico já registrado no item 17c — sistema de
       sessão de skill inexistente — mas não foi retestado nem confirmado nesta
       sessão).
    2. **A hipótese de "mesma posição" (item 24e) foi testada e descartada**: o
       Murillo criou dois personagens **novos**, entrou com as duas contas, ficou **a
       uma distância normal** um do outro (não sobrepostos) e clicou na caixa de
       colisão. Mesmo resultado: só a caixa, sem modelo 3D. Distância não é — e nunca
       foi — a variável.

    ### b. O achado novo: uma mensagem de erro visível que eu tinha ignorado antes

    O Murillo reportou uma mensagem de sistema aparecendo no chat do jogo (traduzida,
    client BR): **"Os dados do mapa da zona não estão em sincronia com o servidor"**.
    Rastreei isso até `AddFixedMessage(FIXMSG_ERR_INSTDATA)`
    (`EC_GameRun.cpp:2926`, `F:\PW\1.5.5\EvolvedPWClient`) — essa função escreve
    literalmente **no chat, como mensagem de sistema** (`AddChatMessage(...,
    GP_CHAT_SYSTEM)`), então é exatamente essa mensagem. Ela dispara quando
    `CECWorld::CheckOutInst` (`EC_World.cpp:2389`) descobre que
    `region_time_stamp`/`precinct_time_stamp` — os campos que `INST_DATA_CHECKOUT`
    (206) carrega — não batem com o que o cliente tem localmente.

    **Eu já tinha visto o sintoma disso** desde a primeira sessão de teste (item 21):
    o `EC.log` de **todo teste, sem exceção, dos dois clients (BR e EN)** mostra
    `regionset timestamp error:server(0x525fd78a) != local(0x55f6bf91)` — e eu
    classifiquei isso como "provavelmente cosmético" sem investigar mais fundo. Foi
    engano: o Murillo confirmou que isso vira uma mensagem de erro real na tela,
    então merecia ser seguido desde o início — o princípio do projeto é não descartar
    sintoma sem entender, e eu descartei.

    ### c. A causa raiz: `region.sev`/`precinct.sev` errados, só na zona "world"

    Comparando os arquivos reais:

    - `data/realm_155/config/world/region.sev` (realm EN): `dwTimeStamp = 0x55f6bf91`
      — bate com o que **todo** `EC.log` (EN e BR) diz esperar.
    - `data/realm_155BR/config/world/region.sev` (realm BR): `dwTimeStamp =
      0x525fd78a` — bate com o que o servidor **estava mandando de fato** (confirmado
      lendo o valor bruto do arquivo com o mesmo leitor que
      `crates/pw-data-loader/src/manager.rs::ler_timestamp_region_sev` usa).

    Ou seja: o servidor estava lendo e mandando o valor **certo do arquivo que
    tinha** — o arquivo é que estava errado. `cmp` mostrou que não é só o timestamp:
    são arquivos **genuinamente diferentes** (`region.sev`: 12652 bytes no EN vs
    12728 no BR; `precinct.sev`: 84174 vs 82896), de datas de disco diferentes (2018
    vs 2014) — não uma cópia com um campo alterado, duas gerações diferentes do
    mesmo arquivo.

    Isso contradiz o que o item 8 registrou ("`realm_155BR` criado... mesma
    `data/realm_155/config` como base... mapas, `region.sev`/`precinct.sev`") — em
    algum momento entre aquela sessão e esta, esses dois arquivos especificamente
    divergiram (possivelmente durante a investigação do `npcgen.data`, item 17, que
    mexeu na mesma pasta `world/`). Verificação nas **outras 97 pastas de zona em
    comum** entre os dois realms: **todas idênticas, byte a byte** — o problema é
    isolado à zona `world` (o mapa aberto principal, exatamente onde os personagens
    de teste entram).

    ### d. Corrigido

    Os originais errados de `data/realm_155BR/config/world/` foram salvos como
    `region.sev.bak-2026-09-05`/`precinct.sev.bak-2026-09-05` (não apagados — caso
    sirvam de referência depois), e substituídos pelas cópias de
    `data/realm_155/config/world/`. Confirmado que o novo `dwTimeStamp` lido é
    `0x55f6bf91` nos dois arquivos. **É um bind mount** (`docker-compose.yml`), não
    tá empacotado na imagem — bastou reiniciar os containers
    (`docker restart pw-realm-155br pw-world-155br`), sem rebuild.

    ### e. Ainda não sabido: se isso resolve o modelo 3D

    A mensagem de erro provava que `CheckOutInst` falhava, mas eu não segui até o
    fim **o que isso desabilita** além da própria mensagem — não confirmei (nem
    descartei) que a checagem de instância tem relação causal com
    `IsBaseInfoReady`/`IsCustomDataReady`/`IsEquipDataReady`/carregamento de modelo.
    Pode ser a causa real (dados de zona "fora de sincronia" plausivelmente afeta
    outros sistemas espaciais/de instância que o carregamento de modelo consulta) ou
    pode ser um problema paralelo, sem relação. **Não inventar aqui** — só o
    reteste decide.

    ### f. Próximo passo

    1. **Reteste imediato**: duas contas, mesma cidade — três coisas pra conferir
       de uma vez:
       - A mensagem "dados do mapa da zona não sincronizados" sumiu? (confirma que o
         fix do item d funcionou de verdade, do ponto de vista do cliente)
       - O modelo 3D do outro jogador carrega agora?
       - `EC.log` de cada lado: o `regionset timestamp error` deve ter desaparecido
         também.
    2. Se o modelo AINDA não aparecer com a mensagem de erro resolvida: a causa é
       outra, meramente coincidente com o achado deste item — próximo passo aí é
       tentar capturar com um debugger anexado ao processo do cliente (Cheat
       Engine/x64dbg), já que não há mais nenhuma pista disponível só por
       log/protocolo — `PlayerBaseInfo_Re`/`GetOtherEquip`/`GetCustomData` já foram
       verificados byte a byte contra a captura real do 1.2.6 e batem exatamente.
    3. Skills continuam 100% não funcionais (item a.1) — sem mudança, sistema de
       sessão de skill (item 17c) continua sendo o item de maior escopo pendente.
    4. Considerar auditar as outras pastas de zona (`a01`-`a80`, `b01`-`b35`, etc.)
       pra ver se `realm_155BR` diverge de `realm_155` em algum OUTRO arquivo além
       de `region.sev`/`precinct.sev` — só verificado esses dois nesta sessão.

26. **Sessão 2026-09-05 (continuação): item 25 confirmado — a mensagem de erro
    sumiu — mas o modelo 3D continua invisível. Vasculhado TODO `a_LogOutput` do
    caminho de criação de jogador/modelo no cliente: nenhum dispara, dos dois
    lados, nos dois `EC.log`. O protocolo está esgotado como fonte de pista —
    documentado aqui o que fica pra quem retomar, e por quê.**

    ### a. Confirmado, descartado

    O Murillo testou de novo: a mensagem "dados do mapa da zona não sincronizados"
    **sumiu** (o fix do item 25 funcionou do ponto de vista do cliente). O modelo 3D
    do outro jogador **continua não aparecendo** — só a caixa de colisão, mesmo
    resultado de sempre. Confirma que o item 25 era uma correção real e válida, mas
    **não** a causa do problema principal — eram dois problemas de dados
    independentes, coincidentes só por estarem na mesma sessão de teste.

    ### b. Última varredura: todo `a_LogOutput` do caminho relevante, contra os dois `EC.log`

    Listei **todo** `a_LogOutput` em `EC_ElsePlayer.cpp`, `EC_ManPlayer.cpp` e
    `EC_Player.cpp` que fica no caminho de criar um jogador e carregar o modelo dele
    — entre eles: `"Failed to create player"`, `"Failed to load player model"`,
    `"Failed to load custom data"`, `"ValidateCustomizeData... invalid and set to
    default"`, `"ChangeCustomizeData, Failed to load face model"`,
    `"CreateElsePlayer, Failed to init player from cache data"`. Conferido nos dois
    `EC.log` (client E: e F:, teste com a mensagem de zona já resolvida): **nenhuma**
    dessas linhas aparece. O log não está sendo pouco verboso por acidente — ele
    genuinamente não tem nenhum `a_LogOutput` alcançado nesse caminho, dos dois
    lados.

    ### c. Uma pista que não fechou: o comentário do próprio código sobre um "BUG" no
    carregamento assíncrono

    `CECElsePlayer::SwitchSimpleModel` (`EC_ElsePlayer.cpp:2669`) tem um comentário
    em chinês (decodificado como GB18030, não UTF-8/Latin1 — o arquivo é do
    EvolvedPW original) que **admite um bug conhecido** no carregamento do BODY via
    `LoadPlayerSkeleton(false)`:

    > "Resolve o BUG em que o BODY carregado é sobrescrito pelo BODY dentro de
    > `LoadPlayerSkeleton(false)`. Depois que o jogador entra no mundo,
    > `LoadPlayerSkeleton(false)` é chamado primeiro. Antes da thread de
    > carregamento terminar de carregar os recursos, chamamos `LoadBodySkin` — essa
    > função roda na thread principal do jogo e retorna o resultado do SKIN
    > imediatamente. Depois, quando a thread de carregamento também termina,
    > o BODY que ela obtém vai sobrescrever o anterior."

    Investigado a fundo: isso descreve uma **condição de corrida já tratada**
    (o motivo de `SwitchSimpleModel` existir), não um bug que bloqueia o
    carregamento por completo — não é a explicação do nosso sintoma. Documentado
    aqui só para não ser reinvestigado à toa por quem retomar.

    ### d. Estado real, sem contorno: o protocolo está provado correto; o resto é
    opaco sem um debugger

    Tudo que dá pra verificar **sem instrumentar o processo do cliente ao vivo** foi
    verificado, e bate:

    - `PLAYER_ENTER_WORLD`/`info_player_1`: layout confirmado (item 16b).
    - `PlayerBaseInfo_Re`/`GRoleBase`: **decodificado campo a campo contra um pacote
      real do 1.2.6** (`_sync/capturas/full_interno.pcap`, porta 29100) e contra o
      IR — bate exatamente, incluindo o byte `_literal` do RPC marshalling
      (item 24c).
    - `custom_data`: 176 bytes reais do Postgres, os mesmos que `write_role_info` já
      manda com sucesso pro próprio personagem (item 24d).
    - `EQUIP_DATA`: confirmado saindo certo, byte a byte (item 24b).
    - `OBJECT_MOVE`/`OBJECT_STOP_MOVE`: confirmado funcionando **em jogo** pelo
      Murillo (item 23).
    - `region.sev`/`precinct.sev`: corrigidos e confirmados (item 25/26a).

    A função que efetivamente cria o modelo (`CECPlayer::LoadPlayerSkeleton`,
    `EC_Player.cpp:2171`) **enfileira o carregamento numa thread separada**
    (`QueueECModelForLoad`) sempre que o jogador não é o próprio (`bAtOnce=false` e
    a thread de carregamento já está pronta — o caso normal). Essa thread não emite
    nenhum log de sucesso nem de falha que apareça no `EC.log` — é opaca por
    design, não por falta de verbosidade nossa. **Não há mais nenhum log, captura
    ou comparação de protocolo que ainda não tenha sido tentado.**

    ### e. O que resta, para quem retomar

    1. **Um debugger de verdade anexado ao processo do cliente** (x64dbg, Cheat
       Engine, ou similar) é o único jeito de ver, ao vivo, se
       `CECElsePlayer::m_bLoadingModel`/`m_pPlayerModel`/`IsLoadThreadReady()`
       chegam a mudar de estado pro segundo jogador — sem isso, qualquer conserto
       novo seria putar no escuro.
    2. **Considerar que pode ser uma limitação do próprio binário/instalação**, não
       do protocolo — o item 9g (sessão anterior, investigando o crash de render)
       já achou um problema não resolvido e não relacionado neste mesmo binário
       (`0x78C4`, chamada de método virtual num ponteiro nulo) que só foi contornado,
       não corrigido, e mencionou instaladores de conteúdo mais completos
       (`v216`/`146`, `v181`/`135`) como possível próximo passo caso o problema seja
       da build, não do protocolo. Vale reconsiderar aqui pela mesma razão: dois
       sintomas "silenciosos, sem log, sem crash" no mesmo cliente, batendo com
       hipótese de limitação do binário mais do que com hipótese de protocolo.
    3. Skills continuam 100% não funcionais, sem mudança (sistema de sessão de
       skill, item 17c, é o item de maior escopo pendente).
    4. Auditoria das outras pastas de zona (item 25f.4) continua pendente, não
       urgente — o achado foi isolado à zona `world`.

27. **Sessão 2026-09-07: a lógica de jogo do servidor original nunca tinha sido usada, e
    ela está toda na mão. Três achados novos sobre o modelo 3D, o `aipolicy.data`
    implementado de verdade, e a descoberta de que o pacote de dados do realm é mais
    novo que o fonte 1.5.5.**

    ### a. O que estava sendo deixado de lado

    `F:\PW\1.5.5\EvolvedPWServer` vinha sendo usado só como fonte de **layout de
    protocolo** — dele saíram `specs/protocol/gamedata_155.json` e `gnet_155.json`,
    extraídos de `cgame/common/protocol.h` e `protocol_imp.h`. Só que ali dentro também
    está o **gamed inteiro**: `cgame/gs/` tem 507 arquivos e ~230.000 linhas de
    *comportamento* — combate, IA, sessões de ação, missões, instâncias — mais o
    `cskill/` (3.332 classes de habilidade geradas) e o `cnet/` (todos os daemons). Nada
    disso tinha sido lido.

    A distância, medida:

    | | Original 1.5.5 | Nosso `pw-gs` |
    | :--- | :--- | :--- |
    | `case C2S::` tratados | 679 (`cgame/gs/playercmd.cpp`, 9.329 linhas) | 33 (`bus_server.rs`) |
    | Fórmula de dano | `actobject.cpp::AttackJudgement` + `playertemplate.h` | `combat.rs`, 42 linhas, fórmula inventada |
    | IA de monstro | `aipolicy.cpp` (2.104) + `ai/policy.cpp` (1.062) + `ainpc.cpp` (420) | `ai.rs`, 106 linhas, máquina de estados inventada |
    | `aipolicy.data` | `CPolicyData::Load` | `aipolicy.rs` — `parse_policies` era `Ok(())`, **não lia nada** |
    | Skills | `cskill/` + `act_session` (`actsession.cpp`, 2.759) | inexistente |

    ### b. O portão exato do modelo 3D — e ele tem três chaves do lado do CLIENTE

    O modelo de outro jogador é criado num único lugar,
    `CECElsePlayer::TickBeforeAnimate` (`EC_ElsePlayer.cpp:671`):

    ```cpp
    if (!m_pPlayerModel && !m_bLoadingModel && IsBaseInfoReady() && IsCustomDataReady() && IsEquipDataReady())
    {
        if ((!m_bUseHintModel && m_iBoothState != 2) || bSelected)   // <-- 2 condições de cliente
        {
            memcpy(m_aEquips, m_aNewEquips, sizeof(m_aEquips));
            if (ShouldUseModel())                                    // <-- 3ª condição de cliente
                LoadPlayerSkeleton(false);
            m_bLoadingModel = true;                                  // <-- trava de uma vez só
        }
    }
    ```

    Três coisas saem daí, e **nenhuma é protocolo** — o que explica por que nenhuma
    sessão anterior achou nada no protocolo:

    1. **`m_bUseHintModel`** é a opção de vídeo `Chk_ModelLimit`
       (`DlgSettingVideo.cpp:66`, `EC_Configs.h:161`, padrão `false`). Ligada, só o
       **alvo selecionado** ganha modelo.
    2. **`ShouldUseModel()`** (`EC_Player.cpp:11822`) devolve `false` quando o
       `CECMemSimplify` está em `MEMUSAGE_NOMODEL`; nesse estado só sobrevive quem é
       `IsMostImportant()` — o alvo selecionado, o par de duelo ou o cônjuge
       (`EC_MemSimplify.cpp`). É um sistema de economia de memória, ligado por
       `bEnableOptimize` nas opções.
    3. **`m_bLoadingModel` é trava de uma vez só**: só volta a `false` em
       `CECElsePlayer::Release()` (`EC_ElsePlayer.cpp:572`), isto é, quando o objeto
       morre. E é marcado **mesmo quando `ShouldUseModel()` é falso** — o
       `LoadPlayerSkeleton` é pulado e o jogador fica marcado como "carregando" para
       sempre.

    **Teste de 30 segundos que nunca foi feito**: selecionar o outro jogador como alvo.
    Se o modelo aparecer, o problema nunca esteve no servidor.

    ### c. O que foi verificado e passou (elimina hipóteses, não deve ser refeito)

    - **`GRoleBase` do cliente 1.5.5** (`ElementClient/Network/rpcdata/grolebase`) bate
      campo a campo, na mesma ordem, com o nosso `encode()`
      (`crates/pw-protocol/src/packets/s2c.rs`, `S2CPlayerBaseInfoRe`). Antes isso só
      tinha sido conferido contra uma captura do 1.2.6 (item 24c) — agora está conferido
      contra o cliente que estava falhando. `PlayerBaseInfo_Re` também tem
      `SizePolicy(size) { return size <= 1536; }`, e o nosso pacote fica muito abaixo.
    - **`cmd_equip_data` do 1.5.5** está sob `#pragma pack(1)` (`EC_GPDataType.h:563`),
      logo o prefixo é 18 bytes com `color_name` — exatamente o que
      `PorVersao::equip_data` escreve.
    - **`custom_data`**: `PLAYER_CUSTOMIZEDATA::From(ptr, size)` (`EC_Player.h`) aceita
      **só dois tamanhos exatos**; qualquer outro vira `memset(0)` silencioso, `bodyID`
      zero, e a thread de carregamento descarta sem log nenhum. Calculei os dois com o
      alinhamento natural (não há `pragma pack` em `EC_Player.h`/`EC_Face.h`): **176**
      (`PLAYER_CUSTOMIZEDATA`, 1.5.x) e **172** (`PLAYER_CUSTOMIZEDATA_1`, 1.2.6).
      Conferido no Postgres: todos os 9 personagens têm 176 (realms 155/155BR) ou 172
      (realm 126). **Hipótese eliminada** — mas vira invariante: um `custom_data` NULL cai
      em `unwrap_or_default()` (`repositories/character.rs`) e produz exatamente esse
      sintoma mudo.

    ### d. Um defeito de dado achado de passagem: `race` é 0 em todos os personagens

    A mesma consulta mostrou `race = 0` nos 9 personagens, inclusive nas classes 6/7
    (Alados), 2/5 (Abissais) e 10/11 (Sombrios). **Não afeta o modelo** — o cliente deriva
    a raça da profissão (`CECProfConfig::GetRaceByProfession`, e `OnMsgPlayerBaseInfo` só
    passa `cls` e `gender` para `SetPlayerBriefInfo`) — mas está errado no banco e vai
    morder algum sistema que dependa de raça.

    ### e. As fórmulas reais de combate (a nossa está errada, não aproximada)

    `gactive_imp::AttackJudgement` (`actobject.cpp:481`) e `playertemplate.h:436`:

    ```text
    acerto:  p = attack_rate / (attack_rate + armor/2),  piso 0.05
    redução: r = def / (def + 40*nível_do_atacante - 25),  teto 0.95
    dano:    dano * (1 - r)
    ```

    A nossa é `1/(1 + def/(100*nível))` — inventada, sem relação com essa. Também não
    existe do nosso lado: as 5 classes de dano mágico (`MAGIC_CLASS`), `attack_attr`,
    máscaras de imunidade, penalidade de curta distância (`physic_damage /= 2` em ataque
    normal), atenuação por distância e `anti_defense_degree`. Tudo em `attack.h`
    (135 linhas), portável direto.

    ### f. `aipolicy.data` implementado — e a descoberta que veio junto

    `crates/pw-data-loader/src/aipolicy.rs` foi reescrito do zero como porte fiel de
    `CPolicyData::Load` / `CTriggerData::Load` / `ReadConditonTree` /
    `ReadOperationParam` / `ReadOperationTarget`. Antes era um esqueleto: os enums
    (`OnAggro`, `OnHPPercent`, `SummonMinions`…) eram invenção, e `parse_policies` era
    literalmente `Ok(())` — o arquivo nunca era lido.

    **O achado**: o `aipolicy.data` do realm 1.5.5 **não é legível pelo fonte 1.5.5**. A
    política de id 1858 usa a operação **36**, que no
    `EvolvedPWServer/cgame/gs/ai/policy.h` é `o_num` — o terminador do enum, não uma
    operação. Ela existe no fonte **1.7.2** (`F:\PW\1.7.2\172Source`, achado nesta
    sessão) e se chama `o_skill_with_talk`: `O_USE_SKILL_2` seguido de um texto de
    tamanho variável, que é exatamente o que os bytes contêm. Segunda prova independente:
    os triggers deste arquivo são gravados na **versão 24**, e o 1.5.5 declara
    `F_TRIGGER_VERSION 23` — na 24 o `O_SUMMON_MINE` ganhou um campo (28 → 32 bytes).

    **Conclusão que muda decisões futuras: o pacote de dados do realm é mais novo que o
    fonte do servidor que temos.** O enum do 1.7.2 é superconjunto exato do 1.5.5 (os 36
    primeiros valores idênticos, na mesma ordem), então adotá-lo não muda nada do que já
    funcionava. **Vale reconferir outros `.data` contra o 1.7.2 antes de assumir que o
    fonte 1.5.5 os descreve** — o mesmo pode valer para `tasks.data` e `npcgen.data`.

    Como o leitor se defende de erro meu:

    - Os parâmetros de *operação* não trazem tamanho no arquivo (o original faz `sizeof`
      na struct C++). A tabela de tamanhos foi **gerada** a partir de
      `GetOperationParamSize` do 1.7.2 + as structs de `policytype.h`, medidas com
      alinhamento natural de 32 bits. Onde o leitor decodifica campo a campo, ele
      **confere** quantos bytes consumiu contra essa tabela e erra alto se divergir.
    - Os parâmetros de *condição* trazem o tamanho no fio, então tipo desconhecido ali é
      seguro (vira `Bruto`).
    - As operações de 37 a 102, que o 1.7.2 só nomeia por número, são lidas pelo tamanho e
      guardadas cruas — nada desalinha, só falta dar nome aos campos quando alguma virar
      prioridade.

    Provas, em `crates/pw-data-loader/tests/aipolicy_tests.rs` (7 testes, todos contra os
    arquivos reais, sem fixture inventada):

    - **realm_155** lido inteiro: versão 1, 3.144 políticas, 15.224 triggers, 5.996 falas,
      zero byte sobrando no fim.
    - **realm_126** lido inteiro: versão 0 (o 1.5.5 recusaria — o nosso aceita as duas de
      propósito), 293 políticas, triggers na versão 1, o que exercita os ramos antigos de
      `ReadOperationParam`.
    - **Prova cruzada, a mais forte**: `MONSTER_ESSENCE.common_strategy` do
      `elements.data` é o id da política do monstro — é o vínculo que
      `npcgenerator.cpp:218` faz (`nt.trigger_policy = mob.common_strategy`). **4.455 de
      4.456** monstros resolvem para uma política existente. Os dois arquivos são
      decodificados por caminhos completamente separados; baterem quatro mil vezes não
      acontece por acaso. O único órfão (monstro 40773 → política 22796) é inconsistência
      do próprio pacote, e o original a trata com aviso e zerando o `trigger_policy`.

    Ferramentas de diagnóstico:
    `cargo run -p pw-data-loader --example dump_aipolicy -- <arquivo>` (estatísticas por
    tipo) e `--example cruza_monstro_aipolicy -- <pasta config>`.

    ### g. O que resta, na ordem

    1. **Fechar a sincronização por eliminação, não por mais protocolo**: o teste de
       selecionar o outro jogador como alvo, e conferir `Chk_ModelLimit` e
       `bEnableOptimize` nas opções de vídeo do cliente. Separa "bug nosso" de "chave do
       cliente"; nenhuma das duas tinha sido levantada.
    2. **Ligar o `aipolicy.data` ao mundo.** O leitor entrega as árvores; falta o
       intérprete no `pw-gs` (`ai.rs` inteiro é substituível) e preencher
       `MonsterTemplate` de verdade — hoje `elements.rs` grava valores fixos
       (`level: 1, hp: 100, aipolicy_id: 0`), enquanto `MONSTER_ESSENCE` tem 287 campos
       reais (`level`, `defence`, `magic_defences_1..5`, `attack`, `attack_range`,
       `aggro_range`, `aggro_time`, `common_strategy`, …) já decodificados pelo
       `generic_elements`.
    3. **Trocar `combat.rs` pelas fórmulas do item (e).** Escopo pequeno, e conserta
       números que hoje estão errados em jogo.
    4. **`act_session` + skills**, o item grande, que depende de 2 e 3. Note que o cliente
       1.5.5 embarca as **mesmas** classes de habilidade geradas
       (`EvolvedPWClient/ElementSkill/skill*.h`) e chama
       `GNET::ElementSkill::GetExecuteTime()`/`GetType()` para animar — os tempos que o
       servidor usar têm que ser exatamente esses, não há margem para aproximar.

28. **Sessão 2026-09-07 (continuação): o monstro deixou de ser inventado. `MONSTER_ESSENCE`
    ligado ponta a ponta, do `elements.data` até a entidade que entra no mundo.**

    É o passo 2 do item 27g. O `world.rs::init_spawns` escrevia os atributos de **todo**
    monstro de **todo** mapa no próprio código:

    ```rust
    name: "Monstro".to_string(), level: 1, hp: 500, max_hp: 500, mp: 100,
    def_phys: 50, attack_min: 20, attack_max: 35, attack_range: 2.5, move_speed: 3.5,
    ```

    E o `elements::MonsterTemplate` (leitor tipado antigo) não ajudava: declara `level`,
    `hp`, `def_phys`, `exp`, `aggro_range`, `aipolicy_id` e **preenche todos com
    constantes** — só `id` e `name` saem do arquivo. Ninguém o consultava.

    ### a. O módulo novo: `crates/pw-data-loader/src/monstros.rs`

    Porte de `npcgenerator.cpp::npc_generator::Init` (o laço sobre `DT_MONSTER_ESSENCE`).
    Cada campo do `TemplateDeMonstro` nomeia, no comentário, a linha do original que o
    preenche. As armadilhas que o porte tinha de acertar, e que estão documentadas no
    código:

    - **`hp` não é um campo.** Vem de `mob.life` (`nt.bp.hp` e `nt.ep.max_hp`).
    - **`attack` não é dano.** É o `attack_rate` da fórmula de acerto
      (`attack_rate / (attack_rate + armor/2)`, piso 0,05). O dano é
      `damage_min`/`damage_max`. Tem teste só para isso.
    - **Mana de monstro é 1**, fixo no original (`nt.bp.mp = 1`, `nt.ep.max_mp = 1`) — o
      custo de habilidade de monstro não sai de mana. Os 100 de antes eram invenção.
    - **Unidades**: `attack_speed` e `damage_delay` estão em segundos no arquivo e viram
      *ticks* de 50 ms — `(int)(x*20 + 0.5)` para o primeiro (arredonda) e `(int)(x*20)`
      para o segundo (trunca). A diferença é do original.
    - **`short_range_mode` é derivado, não lido**: o original ignora o campo homônimo do
      arquivo e usa `attack_range > 6.0`.
    - **`aggro_time` tem piso de 1 segundo**, forçado pelo original.
    - **As três recusas** do original são reproduzidas — `attack_speed <= 0 ||
      damage_min <= 0 || attack <= 0`, `attack_speed > 256`, `damage_delay > 256` fazem o
      monstro **não entrar na tabela**. No realm 155 isso recusa 25 de 8.067.

    Fica de fora, de propósito e documentado: os drops (não há "id de tabela de drop" —
    são 20 pares item/probabilidade mais `drop_times`), o sorteio de `MONSTER_ADDON`, e o
    ajuste de facção por `role_in_war`.

    ### b. Ligado no caminho de produção

    `GameDataManager` ganhou `monstros: TabelaDeMonstros`, montada **depois** do
    `elements.data` e do `aipolicy.data` — porque reproduz a checagem do original:
    monstro que aponta para política inexistente tem o campo zerado, com aviso
    (`npcgenerator.cpp` imprime "a política %d do monstro %d não foi achada").

    `MonsterEntity` ganhou dois construtores em `entity.rs`, `do_template` e
    `placeholder`, e o `init_spawns` virou um `match` de dez linhas. O `placeholder`
    continua existindo para dois casos honestos — o realm 1.2.6, cujo `elements.data` (v7)
    o leitor genérico ainda não cobre, e o `npcgen.data` que cite um monstro ausente — e
    agora o log diz quantos foram, em vez de o monstro sumir ou mentir em silêncio.

    ### c. O que o realm 155 produz

    | | |
    | :--- | :--- |
    | Registros em `MONSTER_ESSENCE` | 8.067 |
    | Templates | 8.042 |
    | Recusados pelas regras do original | 25 (24 modelo de ataque inválido, 1 ataque > 256 ticks) |
    | Com política de IA resolvida | 4.452 |
    | Com habilidade configurada | 3.688 |
    | Com habilidade por limiar de vida | 361 |
    | Política órfã | 1 (monstro 40773 → política 22796, zerado como o original faz) |

    Exemplos: `Tauroc Valorian` nível 47, 6.701 de vida, defesa 468, dano 311–356;
    `Rattus Marksmen` nível 48, alcance 13,2 (à distância). Antes, os dois eram
    "Monstro" nível 1 com 500 de vida.

    ### d. Duas coisas que pareciam bug e são dado

    1. **1.512 monstros sem nome (19%).** São as **entidades controladoras invisíveis**:
       vida em números redondos (999999999, 10000000), dano 1..2, facção `0x40000000`, e
       87% delas com política de IA — contra 48% das nomeadas. Existem só para rodar uma
       árvore de IA. O teste afirma essa separação em vez de exigir nome em todas.
    2. **Nível 150 em 4.256 monstros.** É o teto do 1.5.5, com o resto distribuído por
       todas as dezenas. Desalinhamento de coluna daria ruído, não um pico exato no teto.

       Consequência prática para quem for escrever teste sobre esses dados: **média não
       serve**, porque as controladoras a dominam (há uma de nível 1 com 9.999.999 de
       vida). O teste de "vida cresce com o nível" usa **mediana** e só monstros nomeados.

    ### e. Efeito colateral consertado no `validator.rs`

    A checagem "monstro do `npcgen.data` existe no `elements.data`" consultava só a tabela
    tipada antiga, que é **vazia no 1.5.5** — ou seja, daria *todo* monstro do realm como
    órfão se alguém a chamasse. E a checagem de política de IA nunca disparava, pelo mesmo
    motivo. As duas agora usam a tabela nova quando ela está populada. (O `validate_all`
    não tem chamador nenhum hoje; foi consertado porque a causa era a mesma, não porque
    algo dependesse dele.)

    ### f. Provas

    - `crates/pw-data-loader/tests/monstros_tests.rs`, 13 testes: as três recusas e as
      conversões de unidade com registros sintéticos (determinístico, sem arquivo), e o
      `elements.data` real do realm 155 para o que só um arquivo de verdade pega — nome de
      campo errado, coluna deslocada.
    - `crates/pw-gs/tests/monstro_do_template.rs`, 4 testes: o mapeamento template →
      entidade, campo a campo, com um template cujos valores são todos distintos para
      troca de campo aparecer.
    - `loader_tests.rs::test_game_data_manager_directory_load_155` passou a exigir a
      tabela populada **pelo caminho de produção**, com mais de 90% dos spawns do mundo 1
      achando o template deles.

    ### g. O que resta, atualizando o item 27g

    O passo 2 está feito. Restam:

    1. O teste do alvo selecionado no cliente (passo 1 do item 27g), que não depende de
       código nosso.
    2. **Trocar `combat.rs` pelas fórmulas reais** — agora com dado de verdade para
       alimentá-las. O template já carrega o que falta ao `MonsterEntity`: as cinco
       resistências (hoje só a de metal chega à entidade), o dano mágico por classe, o
       grau de ataque e defesa, o raio de ódio e o de visão.
    3. **O intérprete de `aipolicy` no `pw-gs`** — `ai.rs` inteiro continua sendo máquina
       de estados inventada, com `35.0` de distância de perseguição e 1500 ms de cooldown
       escritos no código, enquanto o template já traz `raio_de_odio`, `raio_de_visao`,
       `tempo_de_odio` e `ataque_em_ticks` reais.
    4. `act_session` + skills, o item grande.

29. **Sessão 2026-09-07 (continuação 2): as fórmulas de combate deixaram de ser invenção.
    `AttackJudgement` portado inteiro, com a tabela de classes alimentando o lado do
    jogador.**

    É o passo 3 do item 27g. O que havia em `combat.rs` eram duas funções de 20 linhas:

    ```text
    def_factor = 1 / (1 + def / (100 * nível_do_atacante))
    dano = uniforme(attack_min, attack_max) * def_factor * (crítico ? 2 : 1)
    ```

    Sem rolagem de acerto, sem classe mágica, sem imunidade, sem grau de ataque e defesa.
    E a redução por defesa não tem relação nenhuma com a do original.

    ### a. A ordem exata, de `actobject.cpp`

    `gactive_imp::AttackJudgement` mais o trecho de `HandleAttackMsg` que vem depois dele:

    1. **Acerto** (só ataque físico): `taxa / (taxa + (armadura >> 1))`, piso de 0,05.
       O deslocamento é inteiro, e **não há teto** — a linha do teto de 0,95 está
       comentada no fonte.
    2. **Curta distância**: golpe de habilidade multiplica pelo fator próprio; golpe
       normal tem o dano **físico** dividido por 2, e só o físico.
    3. **Atenuação por distância**, só quando quem ataca é jogador ou pet.
    4. **Defesa, por classe**: físico contra `defense`, e cada uma das cinco classes
       mágicas contra a sua `resistance`. `reduce = def / (def + 40*nível − 25)`, teto
       0,95 — e o **nível é o do atacante**, não o do alvo. Imunidade zera a classe.
    5. **Crítico**: `× (2,0 + bônus% − redução%)`. A chance é `crit_rate − crit_resistance`,
       e a rolagem `Rand(0,99)` é crítico quando **menor**.
    6. **Grau de ataque menos grau de defesa**, com curvas **assimétricas**:
       vantagem `× (1 + g*0,01)`, desvantagem `÷ (1 − g*0,012)`.
    7. **Piso de 1**: golpe que acertou nunca faz zero.

    Detalhes que só aparecem lendo o fonte, e que o porte respeita:

    - **`attack` não é dano, é precisão** — já era a armadilha do item 28, e aqui é onde
      importa: é o numerador da chance de acerto.
    - **O dano físico é `Rand` uniforme; o elemental é `RandNormal`**, que é a média de
      dois uniformes (distribuição triangular). Trocar um pelo outro muda a forma da
      distribuição, não só o valor.
    - **Penetração** (`CalcAntiDef`): `def × (1 − anti/(anti+10000))`, com a razão
      limitada a 0,35.
    - **Golpe mágico não rola acerto** — `AttackJudgement` só testa quando
      `attack_attr == PHYSIC_ATTACK`.
    - **"Sem dano nenhum" não é acerto**: se toda classe estava zerada ou imune,
      `AttackJudgement` devolve false e o original manda o pacote de esquiva.

    ### b. Sem sorteio dentro do cálculo

    `resolver()` **recebe** as rolagens prontas (`Rolagens { acerto, critico }`) em vez de
    sortear. É o que permite testar cada passo contra número fechado, em vez de "roda mil
    vezes e vê se a média parece certa". Quem sorteia é `Rolagens::sortear()`, num lugar
    só.

    ### c. `CHARRACTER_CLASS_CONFIG`: o lado do jogador ganhou origem

    Duas linhas do cálculo dependiam de dados que não existiam do nosso lado:

    ```text
    GetBasicAttackRate(cls, agi) = agi_attack[cls] * agi   // precisão
    GetBasicArmor(cls, agi)      = agi_armor[cls]  * agi   // evasão
    ```

    `agi_attack` e `agi_armor` vêm da tabela `CHARRACTER_CLASS_CONFIG` do **elements.data**
    (não do `ptemplate.conf`, que é só um exemplo no fonte) — `player_template::
    __LoadDataFromDataMan`. Novo módulo `crates/pw-data-loader/src/classes.rs`, com as 12
    classes do realm 155 lidas e conferidas: Guerreiro 10/10, Mago 5/2, Bárbaro 8/8.

    **Isto é a base, não o atributo final.** No original, `UpdateAttack`/`UpdateDefense`
    somam equipamento (`_cur_item`), pontos de encantamento (`_en_point`) e percentuais
    (`_en_percent`) por cima. Nada disso existe do nosso lado, e o módulo não finge que
    existe: `aplicar_atributos_de_classe` **devolve `false`** quando não consegue, em vez
    de escrever um número inventado.

    ### d. O que não entra, e por quê

    O original também passa por `AdjustDamage` (virtual por tipo), pelos *filters*
    (`EF_AdjustDamage`/`EF_DoDamage`), pelo vigor (`GetVigourEnhance`), pela esquiva de
    dano e de *debuff* (`_damage_dodge_rate`, `_debuff_dodge_rate`) e pelo roubo de vida.
    Nenhum desses sistemas existe aqui — não há filter, não há vigor, não há buff que
    altere dano. Deixar um lugar reservado para eles seria fingir que já fazem alguma
    coisa.

    ### e. Entidades: os campos que faltavam

    `MonsterEntity` ganhou `armor`, `attack_rate`, `attack_degree`, `defend_degree`,
    `magic_attack[5]`, e o `def_magic` de valor único **virou `resistances[5]`** — o
    campo antigo não tinha correspondente no original. Todos preenchidos do template
    (item 28). `PlayerEntity` ganhou `armor`, `attack_rate`, `attack_degree`,
    `defend_degree` e `crit_damage_bonus`.

    ### f. Conferência com dado real

    Um guerreiro nível 50 com 40 de agilidade (precisão 400) e 350 de dano, contra
    monstros do `elements.data` do realm 155:

    | monstro | nv | vida | defesa | armadura | acerto | dano | golpes |
    | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
    | Emerald Qingfu | 20 | 1.342 | 83 | 19 | 97,8% | 336 | 4 |
    | Tauroc Sentinel | 31 | 2.676 | 202 | 30 | 96,4% | 318 | 9 |
    | Tauroc Valorian | 47 | 6.701 | 468 | 46 | 94,6% | 283 | 24 |
    | Feligar Warrior | 48 | 10.234 | 1.020 | 190 | 80,8% | 231 | 45 |

    A defesa corta de 4% a 34%, e a chance de acerto cai de 98% para 81% conforme a
    evasão do alvo — o Feligar Warrior, com 190 de armadura, é sensivelmente mais difícil
    de acertar que os outros. É progressão de MMO, não ruído.

    `cargo run -p pw-gs --example simula_golpe -- data/realm_155/config` reproduz a tabela.

    ### g. **O combate não está vivo em produção, e não é por causa disto**

    Vale ser explícito para quem retomar: **nada em produção constrói um `PlayerEntity`**.
    Ele só é criado em teste, `world.players` nunca é populado, e o `NORMAL_ATTACK` do
    `bus_server.rs` sai cedo no `mundo.players.get(&roleid)`. Ou seja, este trabalho deixa
    as fórmulas certas num caminho que ainda não roda — e a próxima coisa que **de fato**
    liga o combate é popular `world.players` quando o jogador entra no mundo, não mais
    matemática.

    ### h. Provas

    `crates/pw-gs/tests/combate_real.rs`, 22 testes. Os valores esperados foram
    **calculados à mão a partir do C++**, não observados da nossa implementação — um
    teste que só registra o que o código faz hoje não pega porte errado. Cobrem os
    auxiliares isolados (chance de acerto com o deslocamento inteiro, redução por defesa
    com o nível do atacante, teto de 35% da penetração, limiares de 8 m e 40 m da
    atenuação), cada passo do golpe inteiro, e a ponte com as entidades.

    `crates/pw-data-loader/tests/classes_tests.rs`, 5 testes contra o `elements.data` real.

    ### i. O que resta, atualizando o item 27g

    1. Teste do alvo selecionado no cliente (não depende de código nosso).
    2. **Popular `world.players`** — é o que falta para o combate existir em jogo.
    3. **O intérprete de `aipolicy` no `pw-gs`**: o `ai.rs` continua sendo máquina de
       estados inventada, com `35.0` de distância de perseguição e 1500 ms de recarga
       escritos no código, enquanto o template já traz `raio_de_odio`, `raio_de_visao`,
       `tempo_de_odio` e `ataque_em_ticks` reais.
    4. `act_session` + skills, o item grande.

30. **Sessão 2026-09-07 (continuação 3): `world.players` passou a existir. O jogador entra
    no mundo simulado com os números do personagem, e o combate saiu do papel.**

    É o passo 2 do item 29i, e o que faltava para tudo dos itens 28 e 29 ter efeito. O
    diagnóstico do item 29g estava certo: **nada em produção construía um
    `PlayerEntity`**. Ele só existia em teste, `world.players` ficava vazio para sempre, e
    por isso `NORMAL_ATTACK` e `CAST_SKILL` saíam cedo no `mundo.players.get()`, a tabela
    de ameaça nunca recebia nada e o `MonsterAi` inteiro era código morto.

    O curioso é que a **saída** já existia: `remove_player` era chamado no `PlayerLogout`
    e no `LOGOUT` desde antes. Faltava só a entrada.

    ### a. Onde o jogador entra

    `BusServer::colocar_no_mundo`, disparado pelo `BusMessage::EnterWorld` — que é o mesmo
    ponto em que a sessão já era registrada. Carrega o personagem do banco, monta a
    entidade e chama `world.add_player`.

    O `EnterWorld` do barramento carrega `roleid` e mais nada — nem conta, nem realm
    (espelha o pacote GNET real). Daí o `CharacterRepository::get_details_por_role`, que
    busca sem checar dono. **Isso é correto aqui e errado em qualquer caminho que fale com
    o cliente**: a autorização já aconteceu no `pw-link`, no `SelectRole`/`EnterWorld`
    (item 29 da seção 2). O método diz isso no doc, para ninguém o usar por engano.

    Nenhuma falha derruba a sessão: personagem inexistente, banco fora do ar ou mapa
    trocado só registram no log e não põem a entidade. Cair aqui e desconectar trocaria
    "combate não funciona" por "não dá para jogar".

    ### b. De onde vem cada número

    `PlayerEntity::do_personagem` junta **três** fontes, e nenhuma delas sozinha basta:

    | fonte | o que dá |
    | :--- | :--- |
    | banco (`characters`) | identidade, nível, exp, dinheiro, posição, os quatro atributos |
    | `ptemplate.conf` | o ponto de partida do nível 1: vida, mana, velocidades |
    | `CHARRACTER_CLASS_CONFIG` (elements.data) | o que escala: por nível, por vitalidade, por agilidade |

    As fórmulas, de `player_template::__LoadData` e `__LevelUp`:

    ```text
    max_hp = base.hp + lvl_hp*(nível-1) + vit_hp*vitalidade
    max_mp = base.mp + lvl_mp*(nível-1) + eng_mp*energia
    dano   = 1 + (int)(nível*lvlup_dmg)     - (int)(lvlup_dmg)
    defesa =     (int)(nível*lvlup_defense) - (int)(lvlup_defense)
    ```

    O `- (int)(x)` do fim não é enfeite: `__LevelUp` soma
    `(int)((l+1)*d) - (int)(l*d)` a cada nível, e a soma telescópica de 1 até N é
    `(int)(N*d) - (int)(1*d)`. O dano parte de 1 porque é o que o original grava em
    `damage_low`/`damage_high` antes de qualquer nível.

    ### c. Duas descobertas de dado

    1. **O banco já tinha `strength`, `agility`, `vitality` e `energy`** — estão no
       `specs/01_DATABASE_SCHEMA_POSTGRES.sql` desde o começo e **nunca eram lidos**: o
       `SELECT *` trazia as colunas e o `FromRow` as descartava porque não existiam no
       `CharacterRecord`. São eles que alimentam vida máxima, precisão e evasão.
    2. **O `ptemplate.conf` não é um `.data`** e não estava na pasta do realm: no pacote
       original mora em `gamed/ptemplate.conf`, fora de `config/`. Foi copiado para
       `data/realm_155/config/` e `data/realm_155BR/config/` (a pasta `data/` é ignorada
       pelo git, então **um ambiente novo precisa repetir essa cópia** — sem ela o
       carregamento avisa e a vida máxima vira a que estiver gravada no banco).

    Novo módulo `crates/pw-data-loader/src/ptemplate.rs`: um leitor de `.conf` por seção,
    com as doze seções na ordem literal de `player_template::__LoadData`
    (`SWORDSMAN`=0 … `FAIRY`=11). Seção ou campo ausente é **erro**, como no original — um
    realm com onze classes carregaria em silêncio e a décima segunda ficaria sem atributo
    nenhum.

    ### d. O que este jogador não tem

    **Equipamento.** No original, `UpdateAttack`/`UpdateDefense` somam `_cur_item`,
    `_en_point` e `_en_percent` por cima de tudo isto — arma, armadura, encantamento,
    refino. Nada disso existe do nosso lado, então o que entra no mundo é um personagem
    **pelado**: os números estão certos para nível, classe e atributos, e nada mais.
    Grau de ataque, grau de defesa e bônus de dano crítico ficam em zero, que é o valor
    neutro do cálculo, não um palpite.

    ### e. O teste que estava escondendo o buraco

    `subcomandos_no_mundo.rs` **fabricava** o jogador: `montar()` chamava
    `add_player(jogador(...))` com uma entidade escrita à mão. É por isso que 30 testes
    passavam sobre um caminho que não existia em produção.

    Ao ligar a entrada de verdade, 12 dos 30 quebraram — e a causa não foi o cálculo, foi
    **corrida**: `colocar_no_mundo` roda na tarefa do barramento, então mandar o
    `EnterWorld` e seguir em frente deixava o teste alterando um jogador que a carga do
    banco sobrescrevia logo depois. O `entrar()` agora **espera** o jogador aparecer antes
    de devolver.

    Três coisas foram consertadas de passagem, todas do mesmo tipo — teste que passava por
    acaso:

    - `segundo_jogador` inseria o convidado à mão e usava `anfitriao + 1` como id, que
      não existe no banco. Agora o `montar()` cria **dois** personagens de verdade e o
      convidado é um deles. (Funcionava porque os dois ids saem consecutivos — depender
      disso é aceitar que o teste passe por acaso.)
    - O id de realm dos testes vinha só do relógio; dois testes que começam no mesmo
      nanossegundo colidiam em `duplicate key`. Apareceu ao subir de 30 para 32 testes em
      paralelo. Um contador atômico desempata dentro do processo. O mesmo padrão estava
      em `pw-storage/tests/autorizacao_de_personagem.rs` e foi corrigido junto.

    ### f. Provas

    - `crates/pw-gs/tests/jogador_do_banco.rs`, 9 testes do construtor (puro): as
      fórmulas de vida/mana/dano/defesa conferidas à mão, o limite da vida corrente ao
      máximo, e os dois casos de ausência — sem `ptemplate.conf` e sem
      `CHARRACTER_CLASS_CONFIG` — onde ele **não** inventa número.
    - `crates/pw-data-loader/tests/ptemplate_tests.rs`, 6 testes: o formato, as recusas, e
      o arquivo real do realm 155.
    - `subcomandos_no_mundo.rs` ganhou dois testes sem maquiagem nenhuma: o `EnterWorld`
      põe o jogador no mundo **e na grade espacial** com os dados do banco, e o
      `PlayerLogout` tira dos dois.

    Com banco: 32/32 em `subcomandos_no_mundo`, estável em três rodadas seguidas. Sem
    banco: 64 suítes, e continuam apenas as duas falhas pré-existentes do `elements.data`
    v55 e do `npcgen.data` v5/v6 do 1.2.6.

    ### g. O que resta

    1. Teste do alvo selecionado no cliente (item 27g, não depende de código nosso).
    2. **Confirmar o combate em jogo** — agora há um jogador no mundo, um monstro com
       atributos reais e uma fórmula portada. É a primeira vez que os três existem ao
       mesmo tempo.
    3. **O intérprete de `aipolicy`**: o `ai.rs` continua com `35.0` de distância de
       perseguição e 1500 ms de recarga escritos no código, enquanto o template já traz
       `raio_de_odio`, `raio_de_visao`, `tempo_de_odio` e `ataque_em_ticks` reais.
    4. **Equipamento**, que é o que falta para o jogador deixar de estar pelado — e é
       pré-requisito para qualquer comparação de dano com o servidor original.
    5. `act_session` + skills, o item grande.

31. **Sessão 2026-09-07 (continuação 4): primeiro teste em jogo depois do combate. Cinco
    defeitos achados e corrigidos, um deles meu; o modelo 3D segue sem explicação, agora
    com o lado do servidor provado byte a byte.**

    Teste do Murillo no `realm_155BR` com dois clients 1.5.5 reais, contas diferentes.
    Logs do servidor cruzados com `EC.log`/`AF.log`/`A3D.log` dos dois clients.

    ### a. O que passou a funcionar, confirmado no log

    ```text
    mundo: testesacer (#42) nível 1 entrou no mapa 1 — 120/120 de vida,
           dano 1, defesa 0, precisão 50, evasão 20
    ```

    O item 30 funcionou: **o jogador entra no mundo simulado**. E o item 28 também — os
    monstros carregaram com nível e vida corretos, como o Murillo relatou.

    ### b. Meu defeito: o `ptemplate.conf` não é UTF-8

    ```text
    WARN ptemplate.conf de /app/data/config não pôde ser lido:
         stream did not contain valid UTF-8
    ```

    O arquivo do pacote original tem comentários em chinês em **GBK**, e o
    `read_to_string` falha neles. Resultado: `base_das_classes` vazia, e a vida máxima
    caindo no valor gravado no banco em vez do calculado — exatamente o aviso que o item
    30 previu e que ninguém esperava ver tão cedo.

    São 60 bytes altos no arquivo do realm 155, **todos em linha de comentário**, que o
    leitor descarta. Passou a ler em bytes e decodificar de forma tolerante. Um Guerreiro
    de nível 1 vai de 120 (do banco) para 360 de vida; um Sacerdote, para 130/330.

    ### c. Monstros nascendo empilhados

    O `npcgen.data` define **áreas** com um tamanho (`vExts`) e quantos monstros cada uma
    gera. O original monta a caixa `pos ∓ exts/2` (`base_spawner::SetRegion`) e sorteia
    cada monstro dentro dela (`terrain_gen_pos::Generate`).

    O nosso leitor lia `vExts` e **descartava**, colocando os monstros num deslocamento
    fixo de até três metros em diagonal a partir do centro. Medido no `npcgen.data` do
    mundo do realm 155BR:

    | | antes | depois |
    | :--- | ---: | ---: |
    | células de 10 m ocupadas | 7.281 | 20.469 |
    | monstros na célula mais cheia | 42 | 7 |

    Duas diferenças conscientes em relação ao original, documentadas no código: a posição
    é **determinística** (função de `id` e índice, não sorteio — o mesmo `npcgen.data`
    sempre dá o mesmo mundo, e reiniciar deixa de teleportar todo monstro), e **`y` fica
    na altura do centro da área**, porque o leitor não tem o mapa para consultar a altura
    do terreno. Em encosta o monstro pode ficar um pouco acima ou abaixo do chão.

    ### d. Monstros que não se moviam

    A IA mexia em `monster.position` e **não devolvia nada**: nem a grade espacial sabia,
    nem o cliente. O monstro perseguia em silêncio e na tela ficava parado — que é
    exatamente o que o Murillo viu ("parece que ele me ataca mas não anda").

    `MonsterAi::tick` passou a devolver `AcaoDoMonstro::{Atacou, Andou}`; o mundo atualiza
    a grade e emite `EventoDoMundo::MonstroAndou`, que o `BusServer` traduz em
    `OBJECT_MOVE` (15) para todos. O aviso sai só a cada **2 metros** andados: o cliente
    interpola entre um `OBJECT_MOVE` e o próximo, e um por tique de 50 ms seriam 20
    pacotes por segundo por monstro sem ganho nenhum na tela.

    De quebra, a distância de perseguição deixou de ser `35.0` escrito no código e passou
    a ser o `aggro_range` do `elements.data`, com piso de 15 m.

    ### e. Habilidades que nunca chegavam

    O `GET_ALL_DATA` mandava bolsa, equipamento, dinheiro e missões — e **nunca
    `SKILL_DATA` (90)**. O cliente monta a barra de habilidades a partir dele, então o
    jogador ficava sem nenhuma. O codificador já existia, com o layout certo
    (`skill_count` de 4 bytes e 5 por habilidade, sob `#pragma pack(1)`), e sem chamador.

    **Ressalva honesta**: os sacerdotes de teste têm as habilidades 11, 117, 118, 119 e
    167 no banco, e o `CharacterClass::Cleric::default_skills()` de hoje devolve
    125, 113, 190 e 167. Ou seja, os ids gravados vieram de uma lista antiga. Agora eles
    **chegam** ao cliente; se forem de outra classe, o sintoma muda de "não tenho
    habilidade" para "tenho habilidade errada", e aí dá para consertar com evidência.

    ### f. "Protegido por senha" numa conta sem senha

    O par é: servidor manda `TRASHBOX_PWD_STATE` (129) dizendo que não há senha → o
    cliente manda `CHECK_SECURITY_PASSWD` (120) com senha vazia
    (`c2s_SendCmdOpenFashionTrash`) → servidor responde `SECURITY_PASSWD_CHECKED` (277) →
    `CECHostPlayer::OnMsgPlayerPasswdChecked` zera `m_bFirstFashionOpen` e o guarda-roupa
    abre.

    O primeiro já era mandado pelo `gateway.rs`. O último, não: o subcomando 120 caía no
    "ainda não tratado" e o par nunca fechava. O 277 é **sem corpo** (`payload: empty` no
    IR, e o cliente não lê byte nenhum dele) — um corpo a mais faria o cliente descartar o
    comando pelo tamanho e o sintoma continuaria igual.

    Qualquer senha passa por enquanto, e está dito no código: não existe senha de
    guarda-roupa no nosso banco, nenhuma coluna a guarda. Recusar trancaria todo mundo
    para sempre.

    ### g. O modelo 3D: o servidor está provado correto, e o sintoma continua

    Decodifiquei o `PlayerBaseInfo_Re` real desta sessão, byte a byte, direto do log:

    ```text
    retcode 0 | roleid 42 | id 40 | nome "HEal" (UTF-16, 8 bytes)
    race 0 | cls 7 | gender 1 | custom_data 176 bytes começando em 01 70 00 10
    ```

    `0x10007001` é `CUSTOMIZE_DATA_VERSION`, e 176 é exatamente
    `sizeof(PLAYER_CUSTOMIZEDATA)` — um dos dois únicos tamanhos que
    `PLAYER_CUSTOMIZEDATA::From` aceita. O `EQUIP_DATA` também sai (20 bytes, visto no
    log). Ou seja, as três condições do cliente
    (`IsBaseInfoReady() && IsCustomDataReady() && IsEquipDataReady()`) têm tudo o que
    precisam, com os bytes certos.

    Também **não há erro nenhum** nos `EC.log` dos dois clients no caminho de criar
    jogador ou carregar modelo.

    Continua valendo o item 27b: sobram as três chaves do lado do cliente
    (`Chk_ModelLimit`, o `CECMemSimplify`, e a trava de uma vez só do `m_bLoadingModel`),
    e **o teste de selecionar o outro jogador como alvo ainda não foi feito**. É o próximo
    passo, e não custa nada.

    ### h. O que os logs do cliente mostram, e não é do servidor

    - `Failed to open ecm file Models\Weapons\...\木剑\木剑.ecm` — o **modelo da arma
      não está na instalação do cliente**. É a "arma não funcional" do sacerdote: o item
      2867 existe no banco e é enviado; o cliente não tem o `.ecm` para desenhá-lo.
    - `A3DGFXMan::Init() Can not open gfxlist.txt`, cinco cursores ausentes,
      `loddata\Login\olm\1.olm` ausente — conteúdo faltando nos dois clients.
    - `CECWorld::LoadWorld: File operation error (line: 617)`.

    Nada disso passa pelo servidor. Se o objetivo é um cliente íntegro, o caminho é o
    instalador mais completo que o item 26e.2 já mencionava.

    ### i. Uma lacuna achada de passagem, não corrigida

    O `create_character` grava os quatro atributos com o padrão do schema (10/10/10/10),
    mas o `ptemplate.conf` dá valores **por classe** — o Guerreiro começa com vitalidade
    20, força 15, agilidade 10, energia 5. Personagem novo nasce com atributo errado, e os
    já existentes teriam de ser migrados. Fica anotado; mexer nisso é mudança de dado, não
    de código.

    ### j. Provas

    - `crates/pw-data-loader/tests/spawns_espalhados.rs`, 3 testes: dispersão real do
      `npcgen.data` do realm, estabilidade entre cargas, e o caso da área sem tamanho.
    - `crates/pw-data-loader/tests/ptemplate_tests.rs` ganhou o teste que **exige** que o
      arquivo do realm não seja UTF-8 válido e ainda assim seja lido.
    - `crates/pw-gs/tests/achados_do_teste_em_jogo.rs`, 6 testes: o anúncio de movimento e
      o limite de um aviso a cada 2 m, o `aggro_range` do arquivo, e os três layouts
      (`SECURITY_PASSWD_CHECKED` sem corpo, `TRASHBOX_PWD_STATE` de um byte, `SKILL_DATA`
      com 4 + 5×n).

    Com banco: 64 suítes, e continuam apenas as duas falhas pré-existentes do 1.2.6.

32. **Sessão 2026-09-08: por que nenhuma habilidade funcionava. A resposta não estava nas
    habilidades — estava na arma.**

    Três pedidos do Murillo depois do segundo teste em jogo: como abrir o console do
    cliente, o modelo 3D que continua invisível mesmo clicando no outro jogador, e
    "arruma as skills no banco e faz funcionar em jogo".

    ### a. O console: `##debug` no chat, não uma tecla

    Não é uma tecla solta, são **duas travas independentes**, e nenhuma delas está no
    `.cfg` que o instalador entrega:

    | Trava | Quem lê | O que ela libera |
    | :--- | :--- | :--- |
    | `glb_IsConsoleEnable()` (`ElementClient.cpp:160`) | `l_CmdParams.iConsole`, da **linha de comando** | a tecla que **mostra** a janela (`EC_GameUIMan.cpp:1264`) |
    | `GetConfigs()->HasConsole()` (`EC_Configs.h:622`) | `[Settings] console` do `Configs\element_client.cfg` | a **execução** de qualquer comando (`EC_DlgCmdConsole.cpp:235`) |

    Ligar só o `.cfg` mostra nada; ligar só a linha de comando abre uma janela que ignora
    o que você digitar. **Digitar `##debug` no chat liga as duas de uma vez**
    (`DlgChat.cpp:454-461`) — é a única forma que não pede mexer em arquivo do cliente.

    A tecla é `Shift + VK_OEM_3` (`EC_HostInputFilter.cpp:303`). `VK_OEM_3` é a tecla à
    **esquerda do "1"**; em teclado ABNT2 ela imprime aspa simples e aspas, não til — que
    é a razão provável de "Shift+~" não ter feito nada.

    O `configs\console_cmd.txt` do cliente lista o que existe: 69 comandos `d_*` e 14
    `gm_*`. Os que interessam a este projeto: `d_showid`, `d_c2scmd`/`d_gscmd` (manda
    comando cru), `d_querymodel`, `d_queryskill`, `d_relogin`, `d_playerradius`,
    `d_viewradius`, `d_showpos`, `d_task`.

    ### b. As habilidades: o cliente recusa pela arma, antes de olhar a habilidade

    `ElementSkill::Condition` (`ElementSkill.cpp:191`) abre assim:

    ```cpp
    if (info.arrow < GetArrowCost()) return 9;
    if (!ValidWeapon(info.weapon))   return 1;   // <- aqui
    if (info.mp < GetMpCost())       return 2;
    ```

    `ValidWeapon` (`skill.h:179`) é **lista branca**: o `restrict_weapons` do stub da
    habilidade. E `info.weapon` é o `GetDBMajorType()->id` da arma equipada
    (`EC_HostPlayer.cpp:6146-6153`), ou 0 se não houver arma.

    Toda habilidade de Sacerdote aceita **292 (Magia)** ou **0 (desarmado)**. Os
    sacerdotes de teste estavam com o **"Graveto de Madeira" (2867)**, que o
    `elements.data` do realm diz ser **tipo maior 5 (Acha)**. Resultado: o cliente
    recusava **todas** as habilidades da classe antes mesmo de olhar MP, nível ou
    cooldown. É a mesma causa da "arma não funcional" relatada — e, de quebra, o modelo
    do graveto é o único dos quatro que **não existe** na instalação dos dois clients (a
    Varinha, a Espada e o Arco estão lá).

    ### c. E as habilidades gravadas também estavam erradas

    Os stubs gerados do `ElementSkill` (`skillNNN.h`, 3.317 arquivos) declaram `cls`,
    `type`, `rank`, `max_level`, `restrict_weapons` e uma tabela `GetRequiredLevel`. Lidos
    todos, o que os sacerdotes tinham no banco era:

    | id | classe | tipo | nível exigido | veredito no nível 1 |
    | ---: | ---: | :--- | ---: | :--- |
    | 11 | 7 | passiva | 29 | nem aparece na barra (`m_aPsSkills`) |
    | 117 | 7 | maldição | 29 | aparece e recusa |
    | 118 | 7 | maldição | 39 | aparece e recusa |
    | 119 | 7 | maldição | 49 | aparece e recusa |
    | 167 | 255 | bênção | 1 | ok |

    Zero habilidade de ataque utilizável. O `default_skills()` era um chute rotulado de
    "v1.2.6"; dos ids que ele usava, `27`, `60`, `61`, `90`, `190` e `274` eram de outra
    classe, passivos, ou de nível 9 a 39.

    O critério que separa a árvore da classe do resto, aplicado ao catálogo inteiro:
    `cls` igual à classe, `rank == 0`, `max_level == 10`, `GetRequiredLevel[0] == 0` e uma
    tabela de SP em que o nível 2 já custa pontos (as inerentes e as de transformação
    custam 0 SP em todos os níveis). O que sai bate com o Perfect World conhecido —
    Guerreiro 1, Mago 81, Bárbaro 102, Feiticeira 299, Arqueiro 234/235, Sacerdote
    113/125 — o que é uma conferência independente do critério.

    | cls | classe | habilidades de nível 1 | tipo maior da arma | arma |
    | ---: | :--- | :--- | ---: | :--- |
    | 0 | Guerreiro | 1 | 1 Espada | 2097 Espada de Madeira |
    | 1 | Mago | 81 | 292 Magia | 2251 Varinha |
    | 2 | Espiritualista | 1125, 1126 | 25333 Orbe | 26332 Pequena Esfera |
    | 3 | Feiticeira | 299 | 292 Magia | 2251 Varinha |
    | 4 | Bárbaro | 102 | 9 Machado/Martelo | 2258 Porrete com Espinhos |
    | 5 | Mercenário | 1111 | 23749 Adagas | 26331 Faca de Limpar Osso |
    | 6 | Arqueiro | 234, 235 | 13 Longo Alcance | 2250 Arco de Madeira |
    | 7 | Sacerdote | 125, 113 | 292 Magia | **2251 Varinha** |
    | 8 | Arcano | 1350 | 1 Espada | 2097 Espada de Madeira |
    | 9 | Místico | 1374, 1381 | 292 Magia | 2251 Varinha |
    | 10 | Retalhador | 2547 | 44878 Sabre | 44937 Sabre de Bronze |
    | 11 | Tormentador | 2571 | 44879 Foice | 45020 Foice de Ferro |

    Mais a 167 (Portal da Cidade), `cls = 255`, que vale para todas.

    **No código**: `default_skills()` e `default_weapon_id()` reescritos, e um
    `weapon_major_type()` novo. **No banco**: `scripts/corrige_skills_e_armas_155.sql`
    aplica isso aos personagens de `realm_155` e `realm_155BR` (o `realm_126` fica de
    fora, é outra versão).

    ### d. O equipamento do outro jogador ia como "pelado"

    O `EQUIP_DATA` (66) respondia com `mask = 0`. Isso destrava o `IsEquipDataReady()` —
    era o objetivo quando foi escrito — mas descreve o outro jogador como se não vestisse
    nada: `ChangeEquipments` faz `memset(m_aNewEquips, 0, ...)` e sem bit ligado nada
    volta a ser preenchido (`EC_ElsePlayer.cpp:1700-1712`).

    O formato saiu do próprio cliente: `mask` é um bit por slot de `EQUIPIVTR_*`
    (0 arma até 39, `SIZE_ALL_EQUIPIVTR = 40`), `data[]` traz **um inteiro por bit, em
    ordem crescente de slot**, e cada inteiro é o id do item no `elements.data` **nos 16
    bits baixos** — `GetRealElementID` faz `& 0x0000ffff` porque os altos guardam cor de
    moda (`EC_Player.cpp:9635-9644`). Agora vai o equipamento de verdade, lido do banco.

    ### e. O modelo 3D: mais duas hipóteses eliminadas, e onde parou

    Eliminadas nesta sessão, com prova:

    - **`CECMemSimplify` / `ShouldUseModel()`** — o `uiconfig.ini` do cliente diz
      `MemoryBufferStage = 1600, 1700`, e o `EC.log` registra o processo em **122 MB**.
      Longe do limiar; `m_iMemUsage` fica em `MEMUSAGE_NORMAL` e `ShouldUseModel()`
      devolve `true`.
    - **O `GetBornStamp()` de `EC_World.h:189`**, que a sessão anterior tinha marcado como
      suspeito por pós-incrementar. Ele é o **alocador** do mundo (distribui carimbos
      únicos); o do jogador, `pPlayer->GetBornStamp()`, é um getter simples. Não há bug
      ali — a suspeita anterior estava errada.

    O que passou a ser conhecido e não era: as três flags do portão
    (`EC_ElsePlayer.cpp:671`) são preenchidas por **pedidos que o cliente faz**, não pelo
    que mandamos no login. Ao ver outro jogador ele dispara `GetRoleBaseInfo`,
    `GetRoleCustomizeData` e `c2s_CmdGetOtherEquip` (`EC_ManPlayer.cpp:347-358`). Os três
    são respondidos — confirmado no log do realm — e cada resposta liga a sua flag:
    `PlayerBaseInfo_Re` liga base **e** custom (`EC_ElsePlayer.cpp:1882`), `EQUIP_DATA`
    liga equip incondicionalmente (`bReset` é sempre `true`, `EC_ElsePlayer.cpp:1942`).

    Sobra uma hipótese, ainda não provada: `m_bLoadingModel` é uma **trava de uma vez só**
    (posta em `EC_ElsePlayer.cpp:678`, limpa só em `Release()`). Se o carregamento
    assíncrono é enfileirado e depois descartado, o modelo nunca mais é pedido — e o
    descarte é **silencioso** (`DeliverLoadedPlayerModels` chama `ReleasePlayerModel` sem
    log, `EC_ManPlayer.cpp:2502`). Isso explicaria por que clicar no jogador não ajuda: o
    `bSelected` está no `if` de dentro, e o de fora já está fechado pela trava.

    **Dois experimentos baratos para a próxima sessão**, os dois sem mexer em código:
    1. Afastar-se até o outro jogador sumir e voltar. Isso força `Release()`, que zera a
       trava. Se o modelo aparecer na segunda aproximação, a trava é a causa.
    2. Com o console aberto (`##debug`), `d_relogin` num dos clientes com o outro parado
       ao lado.

    ### f. Provas

    - `crates/pw-core/tests/core_tests.rs`: a tabela de habilidades iniciais das 12
      classes fixada id a id, sem repetição e tudo no nível 1; e o par arma/tipo maior,
      com o 2867 barrado explicitamente.
    - `crates/pw-data-loader/tests/armas_iniciais.rs`, 2 testes: cruza
      `default_weapon_id()` com o `WEAPON_ESSENCE` do `elements.data` do realm — tipo
      maior, nível e atributos exigidos — e registra que o Graveto é do tipo 5 e a Varinha
      do 292.
    - `crates/pw-gs/src/bus_server.rs`, 4 testes: a máscara de equipamento em ordem de
      slot, os 16 bits baixos do id, o slot fora do array do cliente, e o caso sem
      equipamento.
    - Corrigido de passagem um defeito **meu** da sessão anterior: o codificador
      `security_passwd_checked` (277) tinha sido escrito sem entrar na tabela `INTENCAO`
      do `subcomandos_s2c_contra_o_ir`, e a suíte estava vermelha por isso desde então. A
      sessão anterior disse que só as duas falhas do 1.2.6 restavam; eram três.


33. **Sessão 2026-09-08 (continuação): o overlay do `d_rtdebug` mediu o cliente, e o que
    ele mediu derruba uma premissa do projeto — o fonte `EvolvedPWClient` está à frente
    do binário que servimos.**

    O Murillo ligou o console, jogou com os dois clientes e trouxe três linhas do overlay.
    Elas valem mais do que toda a leitura de fonte das sessões anteriores juntas, porque
    são medidas do **binário**, não do código-fonte vazado.

    ```text
    SERVER - Invalid GAMEDATA_82 size(Network:12, Client:8)
    SERVER - Unknown GAMEDATA_66
    ```

    ### a. "Unknown" não quer dizer id desconhecido

    `CalcS2CCmdDataSize` (`EC_GameDataPrtc.cpp:86-95`) começa com `dwSize = -1` e a macro
    `CHECK_VALID` só o troca por `-2` quando `CheckValid` passa **e** o tamanho bate
    exatamente. Quem sai com `-1` é reportado como *Unknown*. Ou seja: para os comandos de
    tamanho variável, "Unknown" é **tamanho errado**, não id errado. Os comandos de
    tamanho fixo é que dão o "Invalid … size(Network:N, Client:M)", com os dois números.

    ### b. A premissa que caiu

    O `GET_OWN_MONEY` (82) foi a régua. O `cmd_get_own_money` do fonte tem
    `amount`, `max_amount` e `color_name` — 12 bytes. O binário diz que espera **8**. A
    diferença é exatamente o `color_name`, um campo que este projeto adicionou em
    2026-09-03 com um comentário honesto dizendo que *"a evidência aqui é o source, não um
    tcpdump"*. A medida chegou e disse o contrário.

    O mesmo campo tinha sido posto no `EQUIP_DATA` (66) — e era ele que fazia o cliente
    descartar o comando, deixar `IsEquipDataReady()` em falso para sempre e **nunca
    carregar o modelo 3D do outro jogador**. Duas sessões de leitura de fonte procuraram
    esse defeito no lugar errado.

    A conferência independente é o IR do 1.5.3 (`specs/protocol/gamedata_153.json`):
    `cmd_get_own_money` = 8 bytes, `cmd_equip_data` com prefixo de 14. Os dois batem com o
    binário. **Onde o fonte `EvolvedPWClient` e o IR discordarem, para este cliente o IR
    está certo** — e o overlay é o árbitro quando nem isso resolve.

    ### c. O modelo 3D

    Causa encontrada: o `color_name` de 4 bytes no `EQUIP_DATA`. Sem `IsEquipDataReady()`
    o portão de `EC_ElsePlayer.cpp:671` nunca abre, e é por isso que selecionar o jogador
    não ajudava — o `bSelected` está no `if` de dentro. Todas as hipóteses eliminadas nas
    sessões anteriores (`Chk_ModelLimit`, `CECMemSimplify`, o born stamp, a trava
    `m_bLoadingModel`) continuam eliminadas; nenhuma delas era o problema.

    ### d. As habilidades que conjuravam e não terminavam

    Log do mundo: `42 conjurou em 42, que não é um monstro deste mundo`. A Prece da
    Clareza (113) é cura em si mesmo, e o tratamento saía cedo quando o alvo não era
    monstro — antes de mandar o comando que solta o conjurador.

    E o comando que solta não era o que mandávamos. O `SKILL_PERFORM` (88) é roteado para
    `MAN_PLAYER` (`EC_GameDataPrtc.cpp:1385`): ele anima os **outros**. Quem fecha a
    conjuração de quem conjurou é o **`HOST_STOP_SKILL` (123)**, sem corpo — o único
    caminho que zera `CECHostPlayer::m_pCurSkill` numa conjuração bem-sucedida
    (`EC_HostMsg.cpp:6065-6096`), além de chamar `EndCharging()`,
    `StopSkillAttackAction()` e `FinishWork`. O codificador já existia, com o id certo, e
    nunca tinha sido chamado.

    Agora ele vai **sempre**, antes de qualquer coisa depender do alvo. O *efeito* de cura
    e bênção continua não existindo — ver a lacuna do motor de habilidades.

    ### e. A arma vermelha

    `A3DCOLORRGB(192, 0, 0)` é o que o cliente pinta quando `CanUseEquipment` devolve
    falso (`DlgInventory.cpp:530-532`, `DlgBag.cpp:260-263`). A razão 2 dessa função é
    atributo insuficiente:

    ```cpp
    if (GetMaxLevelSofar() < pEquip->GetLevelRequirement() ||
        m_ExtProps.bs.strength < pEquip->GetStrengthRequirement() || ...) iReason = 2;
    ```

    `m_ExtProps` é preenchido por **um** comando só, o `OWN_EXT_PROP` (50)
    (`EC_HostMsg.cpp:1583`), que nunca mandávamos. Os quatro atributos ficavam em zero e a
    Varinha, que exige força 5, aparecia vermelha — como apareceria qualquer equipamento
    com exigência. O `PLAYER_EXT_PROP_BASE` (53), que já mandávamos com `5, 5, 5, 5`
    escritos no código, não serve: vai para o gerente dos **outros** jogadores
    (`EC_GameDataPrtc.cpp:1180`).

    O 50 foi implementado com o layout do IR — 188 bytes, dez inteiros de cabeçalho mais o
    `ROLEEXTPROP` de 148 — e não com o do fonte, que tem dez campos a mais (quatro deles
    marcados `// NEW` no próprio arquivo) e daria 228. Os atributos vão do banco; os
    números derivados de combate vão zerados de propósito, porque quem os calcula é o
    `pw-gs` e o link não os tem — repetir o chute dos `5, 5, 5, 5` seria trocar um defeito
    por outro.

    ### f. Provas

    - `test_get_own_money_tem_oito_bytes_nas_duas_versoes` e
      `test_equip_data_nao_leva_color_name` substituem o teste que fixava os 12 bytes do
      `color_name` — aquele passava e estava errado.
    - `test_own_ext_prop_tem_188_bytes_e_os_atributos_no_lugar` fixa o tamanho e o
      deslocamento da força, que é o campo que solta o equipamento.
    - `conjurar_em_si_mesmo_ainda_fecha_a_conjuracao` cobre o caso da cura em si mesmo, e
      o teste de dano passou a exigir o `HOST_STOP_SKILL` também.
    - O guarda `todo_codificador_esta_declarado_em_algum_lugar` pegou o `own_ext_prop` sem
      entrada na tabela `INTENCAO` — mesma rede que pegou o `security_passwd_checked` na
      sessão anterior. Ela funciona.

    ### g. O que fazer na próxima vez que algo "não acontece na tela"

    Ligar `##debug` e `d_rtdebug 1` **antes** de ler fonte. Um comando com tamanho errado
    não gera erro em log nenhum, dos dois lados; o overlay é a única coisa neste projeto
    que o denuncia, e diz o número exato que o binário espera.


34. **Sessão 2026-09-08 (continuação 2): os modelos 3D apareceram. Quatro defeitos que só
    apareceram depois, e uma medida que desmentiu tanto o fonte quanto o IR.**

    Primeiro teste com o overlay ligado o tempo todo. Deu certo: **os modelos 3D dos dois
    jogadores apareceram** — o `color_name` do item 33 era mesmo a causa. O que o teste
    trouxe de novo:

    ### a. `OWN_EXT_PROP`: nem 188 nem 228, mas 196

    ```text
    SERVER - Invalid GAMEDATA_50 size(Network:188, Client:196)
    ```

    O item 33 tinha estabelecido "onde o fonte e o IR discordarem, o IR está certo". Esta
    medida mostra que a regra é boa mas incompleta:

    | Fonte | Inteiros de cabeçalho | Corpo |
    | :--- | ---: | ---: |
    | IR do 1.5.3 | 10 | 188 |
    | **binário, medido** | **12** | **196** |
    | fonte `EvolvedPWClient` | 20 | 228 |

    O binário fica **entre os dois**. A diferença de oito bytes são dois inteiros, e a
    ordem do fonte diz quais: depois de `vigour` vêm `anti_defense_degree` e
    `anti_resistance_degree`, e só então os quatro campos que o próprio fonte marca
    `// NEW`, que este binário não tem.

    A regra final, então, é mais simples e mais honesta: **nenhuma das duas referências é
    autoridade; o overlay é.** As duas servem para propor um palpite; o número que o
    cliente imprime é o que decide.

    ### b. A conjuração terminava antes de começar

    "Apareceu o Cast Skill mas o cliente que cliquei não fez nada, nem a animação — porém
    o outro jogador me viu castando."

    Essa assimetria é a pista inteira. O item 33 passou a mandar o `HOST_STOP_SKILL` (123)
    — necessário — mas **junto** com o `OBJECT_CAST_SKILL` (85). No dono da tela, o 85
    monta um `CECHPWorkSpell`, chama `PlaySkillCastAction` e arma o contador da barra
    (`EC_HostMsg.cpp:6000-6055`); o 123 chegando no mesmo quadro cancela tudo isso antes
    do primeiro desenho. Os outros jogadores **não recebem o 123** — por isso eles viam a
    animação inteira.

    O fim da conjuração passou a rodar numa tarefa própria, depois de
    `TEMPO_DE_CONJURACAO_MS`. Esperar na própria mensagem não serve: uma conexão de
    barramento carrega vários jogadores, e dormir nela travaria todo mundo por um segundo.
    O `BusServer` ganhou `#[derive(Clone)]` para isso — todos os campos são `Arc` ou
    `Copy`, então clonar compartilha o mesmo mundo.

    O tempo é fixo em 1000 ms e **isso está errado no detalhe**: cada stub do
    `ElementSkill` tem seu `GetExecutetime`. Quando o servidor ler a tabela de
    habilidades, o número sai daquele lugar.

    ### c. O botão de armadura/roupa não existia do lado do servidor

    "Mudei para modo roupa, não sincronizou para o outro jogador, e ao clicar de novo não
    voltou — no debug não loga nada."

    Os três sintomas são um só: o `SWITCH_FASHION_MODE` (C2S **85**, sem corpo —
    `_SendNakeCommand`, `EC_SendC2SCmds.cpp:1368`) caía no ramo silencioso do `match`. O
    cliente **não alterna sozinho**: ele pede a troca e espera o servidor dizer qual é o
    estado novo, pelo `PLAYER_ENABLE_FASHION` (S2C **192**, 5 bytes), que precisa chegar a
    quem apertou **e** a quem está por perto — o cliente acha o dono pelo `idPlayer` do
    corpo (`EC_ManPlayer.cpp:1355-1358`). Sem resposta o botão fica preso.

    O estado vive no mundo (`PlayerEntity::modo_roupa`) e volta ao padrão a cada login:
    não há coluna para ele no banco, e criar uma é mudança de esquema.

    ### d. A arma vermelha era o `OWN_EXT_PROP` que não chegava

    Mesmo defeito do item 33e, só que ele nunca chegou a funcionar porque o comando era
    recusado por tamanho. Com os 196 bytes certos, os atributos chegam e
    `CanUseEquipment` para de recusar.

    ### e. As flechas estavam no slot de voo

    Achado ao procurar onde pôr as asas. `ClassTemplateRepository` gravava a munição do
    Arqueiro no slot **12**, com o comentário "slot de munição (slot 12)". O 12 é
    `EQUIPIVTR_FLYSWORD`; munição é o **11** (`EQUIPIVTR_PROJECTILE`,
    `EC_IvtrTypes.h:67`). As flechas ocupavam o lugar das asas.

    ### f. As asas

    A tabela `WINGMANWING_ESSENCE` do `elements.data` tem **exatamente uma linha**: id
    2096, "Asa", nível mínimo 1, 2 de mana por segundo. É a asa nata do Alado, e o cliente
    confere a classe sozinho — `CanUseEquipment` recusa `ICID_WING` para quem não for
    `PROF_ARCHOR` nem `PROF_ANGEL` (`EC_HostPlayer.cpp:4927`). Arqueiro (6) e Sacerdote
    (7) passaram a nascer com ela no slot 12, no código e no banco
    (`scripts/asas_e_slot_de_municao_155.sql`).

    As outras raças voam com item de `FLYSWORD_ESSENCE`, que tem 1.098 linhas e **nenhum
    campo de classe**. Sem uma forma medida de escolher, elas ficam sem item de voo
    inicial em vez de ganhar um chute.

    ### g. Uma armadilha do leitor de `elements.data`, achada de passagem

    `load_elements_data(path)` em Python **sem** `overrides_path` devolve 198 das 231
    tabelas vazias — incluindo `MONSTER_ESSENCE`, que sabidamente tem 8.054 linhas. As
    tabelas são lidas em sequência, então uma tabela com tamanho errado derruba todas as
    seguintes em silêncio, sem erro.

    Com `specs/elements_155/realm_155_overrides.json`, caem para 132 vazias e as três
    tabelas que importavam aqui aparecem. O lado Rust não tem esse risco porque
    `load_elements_data_auto` já carrega os overrides. **Em Python, nunca chamar
    `load_elements_data` sem `overrides_path`** — o sintoma de errar é uma tabela vazia
    que parece uma resposta legítima.

    ### h. Provas

    - `test_own_ext_prop_tem_196_bytes_e_os_atributos_no_lugar`, atualizado da medida.
    - `o_botao_de_roupa_alterna_e_avisa_os_dois_lados`: liga, confere que os dois lados
      recebem o 192 com o `idPlayer` certo, e que clicar de novo volta para a armadura.
    - Os testes de conjuração continuam exigindo o 123, agora depois do tempo de
      conjuração.


35. **Sessão 2026-09-08 (continuação 3): as habilidades passaram a fazer alguma coisa —
    cura e dano entre jogadores, com a conta do cliente.**

    Até aqui a conjuração animava, fechava a barra e não mudava nada. Faltava o efeito.

    ### a. De onde vieram os números

    Do cliente, e não de aproximação. Cada habilidade tem um stub gerado em
    `EvolvedPWClient\ElementSkill\skillNNN.h`, e o motor original escreve a conta lá, num
    `Calculate` por fase da conjuração:

    ```cpp
    // skill125.h — a Pluma Espiritual do Sacerdote
    skill->SetPlus  (4.5 * L * L + 90 * L + 29.6);
    skill->SetRatio (0.5 + 0.05 * L);
    skill->SetDamage(skill->GetMagicattack ());
    ```

    Ou seja **`dano = base × ratio + plus`**. A base é o ataque físico quando o stub usa
    `SetDamage(GetAttack())`, e o mágico quando usa um dos `Set<elemento>damage`
    (`SetFiredamage`, `SetWooddamage`, `SetGolddamage`…, todos alimentados por
    `GetMagicattack()`); o elemento em si é o campo `attr` do stub.

    Cura é outro caminho, no `StateAttack`:

    ```cpp
    // skill113.h — a Prece da Clareza
    skill->GetVictim ()->SetValue (skill->GetMagicdamage () * 4 * L / 100 - 35 + 70 * L);
    skill->GetVictim ()->SetHeal (1);
    ```

    O módulo novo `crates/pw-gs/src/habilidades.rs` tem essas contas para as **dezesseis**
    habilidades do kit inicial das doze classes, portadas uma a uma, com o stub de origem
    no comentário de cada linha.

    ### b. O que a tabela não é

    Não é o motor de habilidades. As outras 3.301 do catálogo **não estão lá**, e a
    resposta para elas é `Habilidade::conhecida` devolver `None` — não um número
    inventado. Em jogo isso significa: a habilidade conjura, anima, fecha a barra e não
    faz efeito, que é honesto e visível.

    Também não há estado: nada de veneno, lentidão, atordoamento ou bênção com duração.
    Duas habilidades da própria tabela têm efeito de estado no original (a 1126 tem
    lentidão, a 1374 tem duas) e aqui aplicam só a parte de dano.

    Três simplificações que estão escritas no código, para não virarem surpresa:

    | O quê | Como está | Por quê |
    | :--- | :--- | :--- |
    | Nível da habilidade | fixo em 1 | o `character_skills` guarda o nível e o `CastSkill` não o manda; ler do banco é o passo que falta |
    | `GetMagicdamage` da cura | usa o ataque mágico | o `PlayerEntity` não separa os dois; a parte fixa da cura, que domina nos níveis baixos, está certa |
    | Carga do Tiro Certeiro (234) | assume carga cheia | o servidor não recebe o tempo de carga do cliente |

    ### c. Os comandos que faltavam

    O `HOST_SKILL_ATTACKED` (144) não existia. É o par do 142 do outro lado: o 142 diz a
    quem conjurou quanto ele fez, o 144 diz a quem levou quem foi e quanto doeu. Sem ele o
    alvo não toca efeito nenhum nem entra em estado de combate
    (`CECHostPlayer::OnMsgHstSkillAttacked`, `EC_HostMsg.cpp:1023-1068`).

    Detalhe que economiza um defeito futuro: o campo `cEquipment` diz qual peça de
    armadura se desgastou, e `0x7f` é o valor que o cliente lê como "nenhuma"
    (`(pCmd->cEquipment & 0x7f) != 0x7f` é a condição para gastar durabilidade). Vai
    `0x7f` porque desgaste não existe no servidor — qualquer outro valor comeria a
    durabilidade de uma peça a cada golpe recebido.

    A vida nova do alvo vai no `SELF_INFO_00`, que é o mesmo comando que a poção já usava.

    ### d. **Não há trava de PvP**

    Um jogador pode machucar outro em qualquer lugar. O original só permite isso em duelo,
    facção em guerra ou mapa de PK (`pvp_mode`, comando 79); nada disso existe aqui. Está
    documentado no código e dito aqui porque é o tipo de coisa que ninguém deve descobrir
    em produção.

    ### e. Contra monstro também melhorou

    O tratamento de conjuração batia como um golpe básico para **toda** habilidade — o
    comentário de então dizia isso com todas as letras. Agora, quando a habilidade tem
    conta portada, é a conta dela que vale (menos a redução por defesa do alvo); as outras
    continuam no golpe básico.

    ### f. Provas

    - 7 testes em `habilidades.rs`, conferindo as contas **contra o stub, na mão**: a
      Prece da Clareza curando 43 com 200 de ataque mágico no nível 1 e 121 no nível 2, a
      Pluma Espiritual fazendo 234, o Guerreiro com `ratio 0` ignorando o ataque, os
      custos de mana arredondados como o cliente faz, e o kit das doze classes inteiro
      presente.
    - 3 testes de integração: a cura em si mesmo subindo a vida no mundo exatamente o que
      foi anunciado na tela, o dano em outro jogador com o `HOST_SKILL_ATTACKED` chegando
      ao alvo, e a habilidade fora da tabela **não** mexendo na vida de ninguém.
    - 1 teste de layout do 144 (19 bytes, `cEquipment` em `0x7f`).


36. **Sessão 2026-09-08 (continuação 4): oito defeitos do teste em jogo — e a descoberta
    de que a suíte de integração estava se auto-anulando.**

    ### a. O achado mais grave não estava na lista do Murillo

    Os testes de integração que precisam de banco começam com uma guarda: sem
    `TEST_DATABASE_URL` no ambiente, eles **imprimem um aviso e passam**.

    ```text
    AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.
    ```

    Rodei a suíte a sessão inteira sem essa variável. Todo "40 passed" que anunciei para
    `subcomandos_no_mundo` — incluindo os testes de cura e de dano entre jogadores do item
    35 — **não verificou nada**. Com a variável, três daqueles testes falhavam na hora:
    dois por defeito meu no teste, um por mudança de comportamento.

    Pior: o `cargo test` também estava reaproveitando binário de teste velho em algumas
    execuções (um `eprintln!` novo não aparecia). `touch` no arquivo resolve.

    **Como rodar de verdade, daqui para a frente:**

    ```bash
    TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \\
      cargo test --workspace
    ```

    Estado real medido assim: **65 suítes verdes, 2 vermelhas** — as duas do 1.2.6 no
    `loader_tests`, e `get_all_data_respeita_os_sinalizadores_do_cliente`, que também
    falha sem nenhuma mudança minha (conferido com `git stash`).

    ### b. A asa sumia do banco porque o servidor a comia

    `usar_item` obedecia ao cliente em tudo: o container e o slot vinham do pacote, e
    `consume_item` apaga a linha quando a quantidade chega a zero. O jogador clicou na asa
    para voar, o cliente mandou `USE_ITEM` apontando para o **container de equipamento**, e
    o servidor consumiu o item. Sem log nenhum — aquele caminho só registrava falha.

    Agora só consumível é consumido, e o container de equipamento nunca é tocado. As asas
    foram recolocadas no banco.

    ### c. A cura aparecia como dano vermelho

    O servidor curava certo — o log dizia `42 conjurou 113 em 42 — 35 de cura, alvo com
    130/130`. O problema era o comando escolhido: o `HOST_SKILL_ATTACK_RESULT` (142)
    termina em `CECPlayer::Damaged`, que só sabe desenhar `BUBBLE_DAMAGE` (vermelho) ou
    "errou" (`EC_Player.cpp:3459-3489`). Não existe bit de cura no `attack_flag`.

    O número verde é **outro comando**: `PLAYER_HP_STEAL` (279), 4 bytes, que o cliente
    traduz em `BubbleText(BUBBLE_ADD, hp)` (`EC_HostMsg.cpp:5772-5781`). Vai para quem
    recebeu a cura. O 143 também não sai em cura, pelo mesmo motivo.

    ### d. A conjuração era rápida demais

    O tempo era fixo em 1.000 ms para todas. É o `GetTime` do primeiro estado do stub, e
    varia de **67 ms** (golpe do Retalhador) a **3.000 ms** (Prece da Clareza) — a cura do
    Sacerdote saía três vezes mais rápida. Agora cada habilidade tem o seu
    (`Habilidade::conjuracao_ms`).

    ### e. A arma continuava vermelha: ordem de envio

    O `OWN_EXT_PROP` (50) já ia com os 196 bytes certos, mas ia no **passo 4b**, antes do
    `SELF_INFO_1` do passo 5 — e é o `SELF_INFO_1` que cria a entidade local do jogador. O
    comando é roteado para o dono da tela (`EC_GameDataPrtc.cpp:1175-1178`); chegando antes
    de o dono existir, ele se perde **sem erro nenhum**. Passou para depois do passo 5.

    ### f. Nada era salvo, e o log dizia que sim

    O `save_status` escrevia `last_login_at = CURRENT_TIMESTAMP` numa tabela `characters`
    que **não tinha essa coluna**. O `UPDATE` falhava inteiro, o erro era engolido por um
    `let _ =`, e a linha seguinte dizia "Autosave periódico executado com sucesso". Posição,
    experiência, alma, dinheiro e nível nunca eram gravados — o `updated_at` dos
    personagens de teste era de 3 de setembro.

    A coluna foi criada (`scripts/2026_09_08_last_login_at.sql`, e no
    `01_DATABASE_SCHEMA_POSTGRES.sql`), o erro passou a ser registrado, e o log passou a
    dizer quantos falharam em vez de mentir.

    ### g. O último personagem jogado

    Não precisa de recurso novo no cliente: ele **já** varre a lista e seleciona o de maior
    `lastlogin_time` (`EC_LoginUIMan.cpp:809-818`). Nós mandávamos zero para todos, então
    caía sempre no primeiro. O campo existe no `RoleInfo` desde sempre; agora ele leva o
    `characters.last_login_at`, carimbado na entrada no mundo.

    ### h. O Ctrl+clique do GM

    `GOTO` é o C2S **19**, `{ A3DVECTOR3 vDest; }`, e aparecia no log como "subcomando 19
    ainda não tratado" — quatro vezes no teste. O cliente não se move sozinho: espera o
    `HOST_CORRECT_POS` (177). Implementado, com o nível de GM lido da conta
    (`accounts.gm_privileges`, via `CharacterRepository::nivel_de_gm`) porque o
    `BusMessage::EnterWorld` não carrega o `sec_level`.

    ### i. Meditar não aparecia para o outro

    `postura` e `emote` respondiam **só a quem agiu**. Os dois comandos carregam o id do
    jogador justamente porque são sobre o que os outros veem. Passaram a ser transmitidos.

    Efeito colateral que só apareceu com o banco ligado: o helper `segundo_jogador` dos
    testes usava `SIT_DOWN` como ida-e-volta de sincronização, e com a transmissão isso
    punha um `OBJECT_SIT_DOWN` na fila do outro jogador, quebrando todo teste que exige
    "nada chega". Trocado por `UNSELECT`, que só responde a quem manda.

    ### j. Provas

    - `usar_um_equipamento_nao_o_consome`: põe a asa no slot de voo, manda `USE_ITEM` nela
      e exige que continue lá.
    - `a_cura_em_si_mesmo_devolve_vida`: exige o `PLAYER_HP_STEAL` (279), **proíbe** o 142,
      e confere que a vida no mundo subiu o que foi anunciado.
    - `uma_habilidade_de_ataque_machuca_o_outro_jogador`: agora espera o comando 144 por
      nome (`esperar_comando`) em vez de contar pacotes.
    - `sentar_aparece_para_o_outro_jogador` e
      `o_goto_do_gm_teleporta_e_o_de_jogador_comum_nao`.


37. **Sessão 2026-09-08 (continuação 5): voo, altura do teleporte, o efeito visual da cura
    — e a suíte finalmente verde de verdade.**

    Meditar e acenar passaram a sincronizar, o Ctrl+clique do GM passou a teleportar. O que
    o teste trouxe:

    ### a. A asa não era consumida — o **cliente** é que a apagava

    O log dizia `42 usou o item 2096 do container Equipment, que não se gasta`, e a asa
    continuava no banco. O guarda do item 36 funcionou. Mas o tratamento ainda respondia
    `HOST_USE_ITEM` (91), que é a confirmação de que **o item foi consumido** — e o cliente
    apaga o item da tela ao receber isso. Do lado do jogador é indistinguível de perder a
    asa.

    ### b. Como se voa, de verdade

    **Não existe comando C2S de decolar.** O cliente pede voo "usando" o item de voo:
    `USE_ITEM` apontando para o slot 12 (`EQUIPIVTR_FLYSWORD`), e espera o
    `OBJECT_TAKEOFF` (96) — é ele que liga `GP_STATE_FLY` e começa o trabalho de voo
    (`CECHostPlayer::OnMsgPlayerFly`, `EC_HostMsg.cpp:5936-5960`). Usar de novo pousa, com
    o `OBJECT_LANDING` (97). Os dois vão para quem está por perto também, que é como eles
    veem as asas abrirem.

    Os dois codificadores **já existiam** no `pw-protocol`, sem chamador — mesma história
    do `SKILL_DATA` e do `HOST_STOP_SKILL`.

    Incompleto e dito no código: o voo não custa mana, não tem altura máxima, e o
    `GP_STATE_FLY` não entra no `state` dos pacotes de visão — quem chegar depois vê o
    jogador andando no ar.

    ### c. O teleporte enterrava o personagem: o `y` do cliente é um marcador

    Os cliques de mapa mandam **`y = 1.0`** — literalmente, `c2s_CmdGoto(fX, 1.0f, fZ)` em
    `DlgWorldMap.cpp:1174`, `DlgRandomMap.cpp:262` e `DlgCountryWarMap.cpp:330`. O servidor
    original nunca confia nesse campo:

    ```cpp
    pos.y = pImp->_plane->GetHeightAt(pos.x, pos.z);   // playercmd.cpp:4926
    ```

    Não temos o mapa para consultar altura — a mesma lacuna dos spawns de monstro. A
    aproximação escolhida é **manter a altura atual do jogador**: ele está de pé no chão
    agora, e as zonas deste mapa são planas em torno de y=219. Em teleporte de encosta a
    encosta ele sai um pouco acima ou abaixo do chão; nunca enterrado num plano.

    ### d. A cura curava e não aparecia nada

    O item 36 trocou o `HOST_SKILL_ATTACK_RESULT` (142) pelo `PLAYER_HP_STEAL` (279) para
    tirar o número do vermelho. Funcionou — e junto foi embora o **efeito visual**, porque
    a máquina de efeito de habilidade do cliente só roda a partir do 142.

    A resposta certa era mandar os dois. O 142 (e o 143 para os de fora) vai com dano
    **-2**, que `CECPlayer::Damaged` trata como "isto veio de uma habilidade de ajuda":
    não desenha número, não toca animação de ferido, e deixa o efeito rodar
    (`EC_Player.cpp:3435-3443`). O 279 leva o valor curado, e é ele que faz o número verde.

    ### e. A arma vermelha: o `OWN_EXT_PROP` estava no daemon errado

    Terceira tentativa, e a que faltava. O tamanho já estava certo (196, item 34) e a ordem
    já estava certa (depois do `SELF_INFO_1`, item 36) — mas o comando saía do `pw-link`, e
    lá os números **não existem**: precisão, evasão, defesa e dano são calculados pelo
    `pw-gs`. O link mandava zeros nos derivados e, pior, mandava num momento em que a
    entidade local do cliente podia ainda não estar pronta.

    Agora ele sai do **mundo**, dentro do `GET_ALL_DATA` — que é o cliente pedindo os
    próprios dados, portanto com o dono da tela garantidamente existindo — e com os valores
    do `PlayerEntity`. Um caminho de escrita só, no daemon que tem o dado.

    ### f. A suíte verde de verdade, e três testes meus que estavam errados

    Com `TEST_DATABASE_URL` ligada (ver o item 36a), o `get_all_data_respeita_os_
    sinalizadores_do_cliente` estava vermelho — e a causa era **minha**, de uma sessão
    anterior: o teste esperava um número exato de pacotes (`receber(2)`), e eu acrescentei o
    `SKILL_DATA` e depois o `OWN_EXT_PROP` à carga. Não havia nada errado no servidor.

    Dois helpers novos consertam a classe do problema: `esperar_comando`, que lê até achar o
    comando pedido, e `receber_ate_o_fim_da_carga`, que junta tudo até o `TASK_DATA` (105).
    Teste que diz **o que** espera, em vez de **quantos** pacotes vêm antes.

    Estado agora, medido com banco: **67 suítes verdes**, e as únicas falhas são as duas do
    1.2.6 no `loader_tests` — as primeiras desta sequência de sessões que não são minhas.

    ### g. Provas

    - `usar_a_asa_decola_em_vez_de_gastar`: usar a asa manda `OBJECT_TAKEOFF` (96) e marca
      o voo no mundo; usar de novo manda `OBJECT_LANDING` (97).
    - `o_teleporte_ignora_o_y_do_cliente`: manda `y = 1.0` (o marcador de verdade), e exige
      que a altura no mundo **e no pacote** seja a de antes.
    - `a_cura_em_si_mesmo_devolve_vida` ganhou a exigência do 142 com dano exatamente -2.
    - `get_all_data_respeita_os_sinalizadores_do_cliente` passou a exigir o
      `OWN_EXT_PROP` (50) na carga.


38. **Sessão 2026-09-09: a arma vermelha, enfim — e o tooltip que resolveu o caso. Mais o
    buraco que o teleporte de GM escancarou.**

    O voo funcionou. Sobraram duas coisas, e a primeira arrastava quatro sessões.

    ### a. O tooltip como instrumento de medida

    O Murillo colou a descrição da Varinha equipada:

    ```text
    Varinha / Cetro / Nv. 1
    Frequência de ataque (vezes/s): 2.00
    Alcance 3.50
    Ataque físico 3-5
    Durabilidade 2800/2800
    Nv. necessário: 1
    Preço 5
    ```

    Três ausências disseram tudo. `CECIvtrWeapon::GetDesc` (`EC_IvtrWeapon.cpp:355-420`)
    imprime, nesta ordem, durabilidade → munição → **profissão** → nível → **força** →
    agilidade → vitalidade → **energia**, e cada linha só aparece se o campo for diferente
    de zero. O `elements.data` do realm diz que a Varinha exige **força 5** e **energia 3**,
    e permite 9 classes. Nenhuma das três linhas estava lá.

    E o alcance: o arquivo diz **3.0**; o tooltip dizia **3.50**.

    Ou seja, o cliente não estava lendo o `elements.data` para aquele item — estava lendo o
    que **nós** mandamos.

    ### b. A causa: uma tabela chumbada de quatro itens

    `S2CGamedataSend::item_info` montava o bloco de dados do item com isto:

    ```rust
    let (req_str, req_agi, /* … */) = match item_id {
        2097 => (5, 5, /* … */),   // Espada de Madeira
        2867 => (5, 3, /* … */),   // Graveto de Madeira
        2258 => (5, 5, /* … */),   // Porrete
        2250 => (5, 5, /* … */),   // Arco
        _    => (0, 0, 0, 0, 1, 1, 0, 3, 5, 0, 0, 10, 3.5),
    };
    ```

    A Varinha é **2251**, e não estava na lista. Caía no genérico — que explica campo a
    campo o tooltip: alcance 3.5, zero em força e energia, e o `1` da quinta posição, que é
    `weapon_type`.

    `WEAPONTYPE_RANGE = 1` (`EC_IvtrTypes.h:167`). O genérico declarava **toda arma
    desconhecida como arma de munição**. E `CanUseEquipment` (`EC_HostPlayer.cpp:4970-4977`)
    faz, para a arma equipada:

    ```cpp
    if (pWeapon->IsRangeWeapon() && !CanUseProjectile((CECIvtrArrow*)pArrow))
        iReason = 5;
    ```

    Sacerdote sem flecha → recusa → `A3DCOLORRGB(192, 0, 0)`. Vermelho.

    Este bloco não é enfeite: `CECIvtrEquip::SetItemInfo` (`EC_IvtrEquip.cpp:176-200`) tira
    dele **os requisitos do item**, e `DefaultInfo()` (que leria o `elements.data`) só vale
    quando o bloco não vem. Mandar dado inventado ali é pior do que não mandar nada.

    ### c. A correção

    Módulo novo `pw-data-loader/src/armas.rs`: lê `WEAPON_ESSENCE` e entrega a ficha por id
    de item. `pw_core::FichaDaArma` é o que viaja — mora no `pw-core` porque o leitor de
    arquivos e o codificador de rede precisam dela e **nenhum dos dois deve depender do
    outro**.

    O mapeamento que decide `weapon_type` saiu da contagem no arquivo do realm, entre as
    2.741 armas:

    | `short_range_mode` | exige munição | quantas |
    | ---: | :--- | ---: |
    | 1 | não | 2.187 |
    | 0 | **sim** | 326 |
    | 2 | não | 222 |
    | 0 | não | 5 |
    | 1 | sim | 1 |

    `short_range_mode == 0` é a marca de longo alcance. As seis exceções vão como o arquivo
    diz.

    Sem ficha (item que não é arma, realm sem a tabela), o comando vai **sem bloco**.

    ### d. O teleporte de GM apagou o mundo

    "Teleportei e não havia NPC nenhum; voltei para a vila e sumiu tudo."

    Os NPCs são mandados **uma vez só**, no login, num raio de 120 m em volta da posição de
    entrada (`gateway.rs`, passo 10). **Não há streaming**: o cliente descarta o que sai do
    raio ativo dele, e o servidor nunca reenvia. Andando a pé isso passava despercebido
    porque o raio do login cobria a vila inteira; o teleporte expôs de uma vez.

    O teleporte passou a reenviar o que está em volta do destino
    (`BusServer::mandar_npcs_ao_redor`). **Isso é remendo do caso agudo, não streaming**: o
    `NPC_ENTER_SLICE`/`NPC_LEAVE_SLICE` conforme o jogador anda continua não existindo, e é
    o que fecha o buraco de vez. Enquanto não existir, andar 120 m a pé em qualquer direção
    tem o mesmo efeito que o teleporte tinha.

    ### e. Provas

    - `crates/pw-data-loader/tests/ficha_da_arma.rs`, 4 testes contra o `elements.data` do
      realm: a Varinha **não** é arma de munição, o arco continua sendo, nenhuma arma
      inicial viaja com máscara de classe zerada, e a correlação
      munição ⇔ `short_range_mode == 0` vale para mais de 99% das armas do arquivo.
    - `protocol_tests`: o bloco montado a partir da ficha, campo a campo, e o caso sem
      ficha indo sem bloco.
    - `o_teleporte_reenvia_os_npcs_do_destino`.








**Depois de "1.5.5 funcional" estar de fato provado** (client real, sem gambiarra), a
prioridade volta para o 1.2.6 (retomar o item 62 — skills/missões/HP de NPC ainda falham lá),
e só depois disso os ajustes de banco de dados, pw-admin, atualizador/launcher (ver
`pw_roadmap_contextos` na memória, Contextos E/G/H).

### Onde a Fase 2 está, exatamente

O critério de aceite da fase, do `PLANO_ARQUITETURA_E_EXECUCAO.md`, é: *"o `gateway.rs`
deixa de existir; nenhum arquivo de gameplay dentro do `pw-link`; 1.2.6 continua entrando
no mundo, agora servido pelo `pw-gs`."* **Ele não foi atingido.** O que foi feito é a
metade que dá para fazer sem cliente na mão:

| | Estado |
| :--- | :--- |
| Barramento entre daemons (`pw-bus`) | pronto, 21 testes |
| `pw-gs` na rede, roteando por jogador | pronto |
| `pw-link` ligado ao mundo, com reconexão | pronto |
| `docker-compose` sobe os dois daemons por realm | pronto, com a porta do barramento fechada e cobrada por teste |
| Os ~650 linhas de gameplay saírem do `gateway.rs` | **começou**: 32 comandos no `pw-gs`; o `gateway.rs` foi de 1379 para 1135 linhas |

A separação foi deixada por último de propósito: mover o tratamento antes de poder testar
com o cliente quebraria o único caminho que hoje funciona (1.2.6 entra no mundo), e a
quebra só apareceria em jogo. A costura já está no lugar e o `GamedataSend` já é repassado
ao `pw-gs`, então cada comando migra sozinho — sai do `match` do `gateway.rs`, entra no
`tratar_subcomando`, e nada mais muda.

### Na ordem

1. **Validar com o cliente de verdade** o que a Fase 1 entregou (o `Challenge` com
   `version` e `edition` corretos) e o que a Fase 2 entregou até aqui: subir o
   `docker-compose` e conferir no log do `pw-world-126` que os subcomandos do jogador
   estão chegando (`mundo: subcomando N de <roleid>`). Isso prova o caminho inteiro —
   cliente → `pw-link` → barramento → `pw-gs` — antes de qualquer código mudar de lado.
   Para o `edition` sair certo o realm precisa dos dois arquivos de gshop — ver item 28,
   que corrige um engano que teria feito você procurar arquivos que já tinha.
2. **Continuar migrando os subcomandos.** Já foram 32: movimento, saída, alvo, combate,
   bolsa, ações, NPC, item, habilidade, grupo e as consultas. O mundo já **manda**
   subcomandos ao cliente, usando os `S2CGamedataSend::*` do `pw-protocol` — um caminho de
   escrita só, conforme a regra do projeto.

   Ao migrar cada um, note que o `gateway.rs` costuma mandar **valores fictícios** onde o
   link não sabe a verdade — foi assim com HP `1000/1000` na seleção **e na consulta
   periódica**, dano `35` no ataque, vida `120/280` em três lugares diferentes, saldo
   `50000` em cinco. Trocar isso pelo estado real do mundo é metade do ganho da migração; a
   outra metade é sair do `UPDATE` por pacote.

   **E confira o layout de cada comando de saída contra o `EC_GPDataType.h` antes de
   confiar nele** (itens 46 e 47). Um comando com o tamanho errado não dá erro: o cliente
   o descarta inteiro. Migrar o tratamento sem conferir o layout produz uma
   funcionalidade que parece pronta dos dois lados do barramento e não acontece na tela.

   O que resta no `gateway.rs` é sobretudo `TASK_NOTIFY` (49), as propriedades estendidas
   (23–26), o diálogo de NPC (35), moda (85), duelo (92) e a tabela de preços do Mall
   (118).

   O que falta de perto: **`OPEN_BOOTH` (76) não tem tratamento** desde que o braço errado
   saiu — barraca de venda pessoal é funcionalidade que nunca existiu, e agora está
   visível; **a compra na Loja Gold** (`MALL_SHOPPING`, 106), removida no item 51 e que
   precisa de saldo, preço e slot livre; **dividir pilha de itens** (item 38); o
   `PLAYER_DIED` (27) para os **outros** jogadores verem quem morreu, que o evento de morte
   já tem em mãos e ninguém consome; a perda de experiência ao morrer, que o
   `REVIVAL_INQUIRE` (197) anuncia e hoje é sempre zero; e os **bits do `attack_flag`**,
   que não estão em nenhuma fonte que temos — hoje vai zero, e o crítico é calculado e
   debitado mas não sinalizado na tela.
3. **`nonce` com estrutura** — hoje o `generate_login_challenge()` devolve 16 bytes com
   os **8 primeiros zerados**. Ali vão `[Attr: u32][newbie_time: u32]` (item 4), e é por
   esse `Attr` que os rates do realm chegam ao cliente.
4. **Unificar o `codec.rs` numa implementação só** — hoje ele usa o `adapter.rs` para
   uns pacotes e o `encode` das structs para outros (item 24). Como os adapters de
   versão não carregam mais nenhuma diferença (item 25), a hierarquia inteira pode
   virar só o `version()`.
5. **Migrar `octets.rs` para o `pw-wire`**, apagando a segunda implementação do formato
   GNET. O teste cruzado já prova que as duas concordam.
6. **Resolver os cinco opcodes sem correspondência no IR** (item 21), que só aparecem no
   `gateway.rs` e vão junto na desmontagem.

Ao mexer em qualquer estrutura, cinco coisas do IR são fáceis de esquecer: o campo
`role` dos comandos (item 12), as divergências de sinal (seção 3), **o cliente é a
autoridade** onde os dois lados discordam (seção 3 e itens 20 e 23), nada de deduzir
número a partir do nome da versão (item 20), e **um caminho de escrita só por layout** —
dois ramos escrevendo a mesma estrutura é como as listas de campos saem de sincronia.

Vale também pôr as verificações no caminho automatizado: `pw-rpcgen --strict` para o
GNET e o `check_sizes.py` com os dois lados para o gamedata.

---

## 5. Onde ficam as coisas

| Onde | O quê |
| :--- | :--- |
| `F:\Python_C_Projects\PWSource1.5.3\pw-universal-server` | Fonte canônico do projeto |
| `F:\PW\1.5.5\EvolvedPWServer` | **Fontes C++ do servidor 1.5.5** (projeto EvolvedPW) — mesma estrutura de `source_server_153` (`cgame/`, `cnet/{inl,rpcdata,rpcalls.xml,<daemon>/callid.hxx}`, `share/rpc/`), mas `cgame/`/`share/` são irmãs de `cnet/`, não filhas — precisa das junções de diretório descritas em `pw_ctx_a_155_funcional` (memória) antes de rodar `pw-rpcgen` |
| `F:\PW\1.5.5\EvolvedPWClient` | **Fontes C++ do cliente 1.5.5** — `ElementClient/Network/` (equivalente ao `CElementClient` do 1.5.3, mesmos arquivos, só sem o `C` no nome da pasta) |
| `F:\PW\1.7.2\172Source` | **Fontes C++ do servidor 1.7.2** — mesma estrutura do 1.5.5. É a autoridade para os `.data` do realm 155, que são mais novos que o fonte 1.5.5 (ver item 27f): o `aipolicy.data` usa operações que só existem aqui. Conferir contra este fonte antes de assumir que o 1.5.5 descreve um formato de dados |
| `F:\PW\1.5.5\pwserver_155v156` | Build Linux do servidor 1.5.5 já compilado (build v156) + `authd` em Java + configs de exemplo |
| `F:\PW\1.5.5\bin`, `F:\PW\1.5.5\1.5.5.EN` | Client 1.5.5 em inglês, pronto pra testar |
| `data\realm_155\config` | `.data`/mapas do realm 1.5.5 já extraídos (elements.data build v156, npcgen.data por zona, tasks.data, gshop*.data — mesmo padrão de pasta que `realm_126`/`realm_153`) |
| `D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL` | Editor de `elements.data` da comunidade — decisivo para fechar as 231 tabelas (`specs/elements_155/README.md`), tem função de export usada pra "impressão digital binária" |
| `D:\PROJETOS\PWPRIVATE\Tools\Editor NPC` | Editor de `npcgen.data` da comunidade — candidato a fazer o mesmo papel que o ADMVAL fez pra `elements.data`, ainda não usado |
| `F:\...\source_server_153`, `F:\...\source_client_153` | Fontes C++ do 1.5.3 (histórico — 1.5.3 não é mais o alvo, ver "MUDANÇA DE BASE" no topo; os fontes continuam válidos como referência de protocolo, provado compatível com 1.5.5) |
| `F:\...\files1.2.6`, `F:\Games\perfectworld_126` | Binários/`.data` compilados do servidor 1.2.6 e o cliente 1.2.6 — ainda o alvo da Fase seguinte, depois do 1.5.5 |
| `_sync/` | Área de transferência entre a máquina local e um contêiner de nuvem (ignorada pelo git) — só relevante para sessões do tipo Cowork/VM remota, ver abaixo |

**Acesso à infra nesta sessão (Windows local, Docker Desktop já na `PATH`)**: `docker ps`,
`docker exec`, `cargo build/test`, etc. rodam **direto**, sem precisar de SSH nem VM remota —
diferente do que a seção anterior deste documento (histórico) descrevia. Isso é específico do
tipo de sessão: se uma sessão futura rodar numa VM Linux remota do Cowork (sem Docker/cargo
locais), a rota antiga (contêiner de nuvem + sincronização por tarball via `_sync/`, SSH pra
`192.168.1.13`) volta a valer — testar `docker ps` direto primeiro sempre, antes de montar
qualquer rota SSH. Credenciais e topologia do `docker-compose` (11 serviços, portas por
realm) estão na memória `pw_universal_infra_access`.

Validação disponível e acordada com o usuário: Docker + clientes reais (1.2.6, 1.5.5) com
envio de logs, captura de tráfego (Wireshark/pcap) e execução dos binários originais para
comparação lado a lado.
