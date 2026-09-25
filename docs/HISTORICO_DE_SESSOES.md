# Histórico das sessões (até 2026-09-12)

> **Arquivo de registro, não de retomada.** Este é o conteúdo integral que o
> `docs/ESTADO_E_RETOMADA.md` tinha até 2026-09-12 (commit `e6433ae`), movido para cá sem
> nenhuma alteração de texto em 2026-09-13, quando aquele documento foi reescrito para
> caber o estado atual numa leitura só. **O estado atual e a fila de trabalho estão no
> `ESTADO_E_RETOMADA.md`** — os "próximos passos" escritos aqui embaixo são de quando
> foram escritos, e muitos já foram feitos ou mudaram.
>
> A evidência (fonte, deslocamento, captura, medida) de cada decisão continua aqui, e é
> aqui que os comentários do código que citam "item N do `ESTADO_E_RETOMADA.md`" apontam.
>
> **Há duas séries de itens numerados, e os números se repetem entre elas:**
>
> | série | onde, abaixo | período | assunto |
> | :--- | :--- | :--- | :--- |
> | **A** | "## 2. Fatos verificados que mudam decisões" (itens 1–64) | até 2026-09-02 | IR do protocolo, Fase 1/2, login e mundo do 1.2.6, tentativa do 1.5.3 |
> | **B** | "## 4. Próximo passo", lista numerada (itens 1–48) | 2026-09-02 a 09-12 | a frente 1.5.5 (login, crash de render, mundo, combate, dados) |
>
> Regra prática para uma referência no código: se o assunto é o cliente 1.5.5, o
> `elements.data`/`tasks.data`/`npcgen.data` 1.5.5, combate, habilidades, visibilidade ou
> o realm 155BR, é a **série B**; se é IR, opcode, `gateway.rs` antigo, 1.2.6 ou
> repositório, é a **série A**. Na série B o começo da lista não é sequencial
> (4, 6, 7, 8, 9, 5, 5, 10, 11…) — procure pelo título em negrito.

---

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

39. **Sessão 2026-09-09 (continuação): o streaming de NPCs, e o fim do envio de mundo pelo
    `gateway.rs`.**

    O item 38 remendou o caso agudo — o teleporte reenviava o que havia em volta do
    destino. O buraco continuava: **andar 120 m a pé em qualquer direção tinha o mesmo
    efeito que o teleporte tinha**, porque o mundo era mandado uma vez só, no login.

    ### a. Onde isso estava, e por que sai de lá

    O envio ficava no `gateway.rs`, passo 10: um raio de 120 m em volta da posição de
    entrada, teto de 60 entidades, e uma lista de reserva escrita no código (a Anciã, o
    Mestre dos Alados, um monstro) para quando o `npcgen` não respondesse.

    Passou para o mundo, e não é arrumação de gaveta — é onde o dado está:

    - o `pw-gs` tem a **grade espacial**, que já indexa monstro, NPC e jogador por posição;
    - o `pw-gs` sabe **quais monstros estão vivos** (o link não sabe: ele lê o `npcgen`, que
      é a lista de nascimento, não o estado);
    - e é o `pw-gs` que vai continuar mandando conforme o jogador anda.

    Com o envio no link, o mundo não tinha como saber o que o cliente já tinha — e o
    contrário também: o teto de 60 do link deixava de fora o que o mundo consideraria
    visível. Dois donos do mesmo estado.

    ### b. Como funciona

    `PlayerEntity` ganhou `visiveis` (o conjunto de ids que aquele cliente tem) e
    `centro_do_stream` (onde ele estava quando a conta foi feita). A cada movimento,
    `BusServer::atualizar_visiveis` pergunta à grade quem está no raio, compara com o
    conjunto e manda **só a diferença**: `NPC_ENTER_SLICE` (11) para quem entrou,
    `OBJECT_LEAVE_SLICE` (13) para quem saiu.

    O 13 é novo aqui: `struct cmd_leave_slice { int id; }`, 4 bytes, e o cliente roteia
    pelo id — `ISNPCID` manda para o gerente de NPCs, `ISPLAYERID` para o de jogadores
    (`EC_GameDataPrtc.cpp:891-899`). Os ids do `npcgen` já nascem com o bit 31 ligado
    (`npcgen.rs:399`), então o roteamento cai no lado certo.

    ### c. As três decisões que fazem isto não derrubar o servidor

    | | Valor | Por quê |
    | :--- | ---: | :--- |
    | Raio | 120 m | o mesmo do login; o cliente descarta o que passa do raio ativo dele |
    | Passo para recalcular | 20 m | o cliente manda movimento **20 vezes por segundo**; sem histerese seriam 20 varreduras da grade por segundo por jogador para achar quase sempre o mesmo conjunto |
    | Teto por jogador | 80 | este mapa tem **21.846 monstros e 3.911 NPCs**; numa região densa o raio pega centenas, e cada uma é um pacote |

    O teto é orçamento de fila, não regra do jogo: entram os mais próximos, e o resto chega
    na atualização seguinte, quando o jogador se aproximar.

    Jogador **não** entra nesta conta. A visibilidade entre jogadores continua no
    `gateway.rs` (`PLAYER_ENTER_WORLD` mútuo); misturar mandaria `NPC_ENTER_SLICE` com id
    de jogador, e o cliente rotearia para o gerente errado.

    ### d. Um defeito de teste que este trabalho revelou

    O helper `entrar` dos testes põe o jogador em (0,0,0) escrevendo direto no mundo. Com a
    âncora do streaming intocada, o primeiro passo de dois metros parecia um salto de
    4,3 km — e o teste de histerese falhava por artefato dele mesmo, não do servidor. O
    helper passou a mover a âncora junto.

    ### e. Sabidamente incompleto

    - **Matéria** (minério, ervas, os "recursos do mapa") não entra: nada no servidor
      manda `MATTER_ENTER_WORLD` (18) ainda, nem no login nem depois.
    - Um monstro que morre enquanto está à vista sai do conjunto e recebe um
      `OBJECT_LEAVE_SLICE` na próxima recalculada. O `NPC_DIED` já tratou a morte; o
      "saiu de vista" é redundante, não errado.
    - O `dir` dos NPCs vai zerado no streaming: a grade guarda posição, não direção. No
      login antigo ele vinha do `npcgen`.

    ### f. Provas

    - `andar_traz_o_que_entra_no_alcance_e_tira_o_que_sai`: põe um monstro na grade,
      anda 25 m e exige o `NPC_ENTER_SLICE` com o id certo; anda 500 m e exige o
      `OBJECT_LEAVE_SLICE`. Confere o conjunto `visiveis` do mundo nas duas pontas.
    - `passo_curto_nao_refaz_a_conta_do_que_esta_a_vista`: dois metros não recalculam nada.

40. **Estado em 2026-09-09 e a fila de trabalho — o que está de pé, o que falta, e o que
    já foi medido para cada coisa que falta.**

    Fecha a sequência de sessões 32–39. O que está escrito aqui é para a próxima sessão não
    reinvestigar nada: cada item pendente vem com a evidência que já foi levantada.

    ### a. O que funciona em jogo, confirmado pelo Murillo

    Modelo 3D dos outros jogadores; equipamento visível entre jogadores; habilidades
    conjuram, animam e fecham a barra; cura e dano entre jogadores; monstros com nível,
    vida e ataque do `elements.data`; monstros perseguem e batem; voo pelas asas; teleporte
    de GM; meditar e gestos sincronizados; botão armadura/roupa; console do cliente
    (`##debug` + `d_rtdebug 1`).

    ### b. A fila, em ordem de valor

    **1. A armadura vai aparecer vermelha, e já se sabe por quê.**

    É o mesmo defeito da Varinha (item 38), esperando a primeira peça de armadura. O bloco
    de dados do item só é montado para **arma** (`WEAPON_ESSENCE`); para armadura vai
    vazio, e aí o cliente cai no `CECIvtrArmor::DefaultInfo`
    (`EC_IvtrArmor.cpp:188-196`), que preenche nível, força, reputação e durabilidade — e
    **não preenche `m_iProfReq`**, que fica no zero do construtor. `CanUseEquipment` faz
    `!(GetProfessionRequirement() & (1 << profissão))`, e zero recusa todas as classes.

    O caminho está pronto: repetir o que `pw-data-loader/src/armas.rs` faz, para
    `ARMOR_ESSENCE` (e depois `DECORATION_ESSENCE`), e passar a ficha em
    `S2CGamedataSend::item_info`. A struct que viaja é `pw_core::FichaDaArma`; armadura
    precisa da irmã dela, sem os campos de dano e com `id_sub_type` (o cliente usa o
    subtipo para saber em que slot a peça entra).

    **2. A visibilidade entre jogadores ainda é de uma vez só.**

    O item 39 resolveu isso para NPC e monstro. Para **jogador** continua como estava: o
    `gateway.rs` manda `PLAYER_ENTER_WORLD` mútuo no login (linha ~941) e
    `PLAYER_LEAVE_WORLD` no logout (linha ~348) — sem raio, sem streaming. Dois jogadores
    que se afastam além do raio ativo do cliente somem um para o outro **para sempre**,
    exatamente o sintoma que os NPCs tinham.

    A correção natural é a mesma: mover para o mundo, dentro de
    `BusServer::atualizar_visiveis`, que já tem a grade espacial e já roda a cada
    movimento. Cuidado com o comando: jogador entra com `PLAYER_ENTER_SLICE` (12), não com
    `NPC_ENTER_SLICE` (11) — o cliente roteia pelo id e mandar o comando errado joga o
    jogador no gerente de NPCs. O `OBJECT_LEAVE_SLICE` (13) serve para os dois.

    **3. Matéria: minério, ervas, os "recursos do mapa".**

    **Ninguém manda.** Não há `MATTER_ENTER_WORLD` (18) em lugar nenhum do servidor, nem no
    login nem no streaming. O `npcgen.data` já traz as instâncias (`SpawnType::Matter`, com
    id próprio em `npcgen.rs:424`) e o `elements.data` tem as tabelas de matéria — falta o
    comando e a entrada no `atualizar_visiveis`.

    **4. O nível da habilidade é fixo em 1.**

    `NIVEL_DA_HABILIDADE` no `bus_server.rs`. O `character_skills` guarda o nível de cada
    habilidade e o `CastSkill` do cliente não o manda — quem deveria saber é o servidor.
    Enquanto for 1, subir uma habilidade de nível não muda nada em jogo.

    **5. Personagem novo nasce com atributo errado.**

    Anotado desde o item 31i e ainda de pé: `create_character` grava 10/10/10/10, mas o
    `ptemplate.conf` dá valores por classe (o Guerreiro começa com vitalidade 20, força 15,
    agilidade 10, energia 5). Os personagens já existentes precisariam de migração.

    **6. Não há trava de PvP.** Qualquer jogador machuca qualquer outro, em qualquer lugar.
    O original exige duelo, guerra de facção ou mapa de PK (`pvp_mode`, comando 79).

    ### c. Lacunas menores, todas documentadas no código

    - Só **16** das 3.317 habilidades têm conta portada; o resto conjura e não faz efeito.
    - Nenhum **efeito de estado** existe (veneno, lentidão, bênção com duração).
    - A cura usa o ataque mágico no lugar do `GetMagicdamage`, que o `PlayerEntity` não
      separa.
    - O Tiro Certeiro (234) assume carga cheia.
    - O voo não custa mana, não tem teto de altura, e o `GP_STATE_FLY` não entra no `state`
      dos pacotes de visão — quem chega depois vê o jogador andando no ar.
    - O teleporte mantém a altura atual do jogador porque não lemos o mapa (o original usa
      `GetHeightAt`).
    - O `dir` dos NPCs vai zerado no streaming: a grade guarda posição, não direção.
    - `modo_roupa` e `voando` não persistem: não há coluna.
    - A tabela `realms` tem **637 linhas**, quase todas `t_gs_*` de teste.
    - As duas falhas do `loader_tests` são do `elements.data` v7 do 1.2.6, que ainda usa o
      leitor tipado antigo.

    ### d. Como rodar, sem tropeçar de novo

    ```bash
    # A suíte SÓ testa de verdade com esta variável (item 36a)
    TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
      cargo test --workspace

    # Publicar no realm de teste
    cd docker && docker compose build pw-world-155br pw-realm-155br \
      && docker compose up -d pw-world-155br pw-realm-155br
    ```

    Referência medida em 2026-09-09: **67 suítes verdes**, e as únicas falhas são as duas do
    1.2.6.

    No cliente, `##debug` no chat liga o console, `Shift` + a tecla à esquerda do "1" abre a
    janela, e `d_rtdebug 1` liga o overlay que denuncia todo comando descartado por tamanho
    — o instrumento que resolveu os itens 33, 34 e 38.












41. **Sessão 2026-09-09 (continuação 2): a fila do item 40, do começo ao fim — armadura,
    jogador, matéria, nível de habilidade e atributos iniciais. E três correções à própria
    fila.**

    Os cinco itens da fila b do item 40 estão feitos. Três deles se mostraram diferentes do
    que a fila dizia ao serem investigados, e a diferença está registrada em cada um.

    ### a. A armadura ia aparecer vermelha, e por um motivo pior do que o previsto

    Só **arma** recebia bloco de dados no `OWN_ITEM_INFO` (40); armadura e acessório iam
    sem bloco. O item 40 previu a consequência certa — máscara de classes zerada, peça
    recusada para todas as classes — mas errou o caminho:

    > "o cliente cai no `CECIvtrArmor::DefaultInfo` (`EC_IvtrArmor.cpp:188-196`), que
    > **não** preenche `m_iProfReq`"

    **`DefaultInfo()` não é chamado.** O único `DefaultInfo()` do `ElementClient` inteiro
    está em `EC_IvtrFashion.cpp:83`; para armadura o método existe e é morto. Sem bloco,
    `CECIvtrEquip::SetItemInfo` retorna na primeira linha (`EC_IvtrEquip.cpp:178-181`) e
    **nada** é preenchido — não só a profissão: nível, força, reputação e durabilidade
    também ficam no zero do construtor (`:74`). E `CanUseEquipment`
    (`EC_HostPlayer.cpp:4953-4959`) faz, para `ICID_ARMOR` e `ICID_DECORATION`:

    ```cpp
    if (!(pEquip->GetProfessionRequirement() & (1 << m_iProfession)))
        iReason = 3;
    ```

    Zero recusa todas as classes. Era a Varinha de novo, pela outra ponta.

    A segunda correção à fila: ela dizia que o `id_sub_type` precisava viajar, "para o
    cliente saber em que slot a peça entra". **Não precisa, e não há onde.** O cliente lê o
    subtipo do `elements.data` dele, pelo id do item, no próprio construtor
    (`EC_IvtrArmor.cpp:66-73`: `m_pDBSubType = get_data_ptr(m_pDBEssence->id_sub_type)`, e
    daí `m_i64EquipMask = m_pDBSubType->equip_mask`). A `IVTR_ESSENCE_ARMOR` não tem campo
    de subtipo, e o `equip_mask` que o servidor original monta em `generate_armor` fica no
    `item_data`, registro interno que não sai na rede.

    **A correção.** Módulo novo `pw-data-loader/src/armaduras.rs`, irmão do `armas.rs`: lê
    `ARMOR_ESSENCE` e `DECORATION_ESSENCE`, e `TabelasDeEquipamento` reúne as três famílias
    com a busca por id (`ficha(item_id)`) — um id vive em uma tabela só, e o teste
    `as_tres_familias_sao_conjuntos_disjuntos` cobra isso do arquivo do realm.

    O `item_info` passou a receber `pw_core::FichaDoEquipamento`, um enum de três variantes.
    O cabeçalho é comum às três — é a `prerequisition` do original
    (`gs/item/equip_item.h:230-238`), com **vitalidade antes de agilidade** e a máscara de
    classes truncada a 16 bits pelo próprio original (`character_combo_id & 0xFFFF`). O que
    muda é a essência:

    | família | essência | bytes | campos |
    | :--- | :--- | ---: | :--- |
    | arma | `IVTR_ESSENCE_WEAPON` | 44 | tipo, atraso, classe, nível, munição, danos, velocidade, alcances |
    | armadura | `IVTR_ESSENCE_ARMOR` | 36 | defesa, evasão, +MP, +HP, resistência[5] |
    | acessório | `IVTR_ESSENCE_DECORATION` | 36 | **dano, dano mágico**, defesa, evasão, resistência[5] |

    As duas últimas têm o mesmo tamanho e ordem diferente: trocá-las passa por qualquer
    teste de tamanho e só aparece em jogo, como número errado no tooltip. O teste
    `o_bloco_da_armadura_e_o_do_acessorio_saem_com_a_essencia_da_familia_certa` confere
    campo a campo, nas duas.

    A autoridade do layout é `generate_armor` e `generate_decoration`
    (`EvolvedPWServer/cgame/gs/template/generate_item_temp.h:490-556` e `772-830`), que
    montam byte a byte o que este código reproduz.

    ### b. A visibilidade entre jogadores era de uma vez só, como a dos NPCs

    Confirmado como o item 40 descreveu: o `gateway.rs` mandava `PLAYER_ENTER_WORLD` mútuo
    no login e `PLAYER_LEAVE_WORLD` no encerramento da sessão, sem raio e sem streaming.
    Dois jogadores que se afastassem além do raio ativo do cliente sumiam um para o outro
    para sempre.

    Passou para `BusServer::atualizar_visiveis`, junto com NPC, monstro e — agora — matéria.
    Três coisas mereceram cuidado:

    **1. O comando de entrada não é o mesmo, e não é o 17.** Jogador entra com
    `PLAYER_ENTER_SLICE` (12). A struct é idêntica à do `PLAYER_ENTER_WORLD` (17) — o IR dá
    `S2C::info_player_1` para os dois — mas o cliente escolhe o **efeito de aparição** pelo
    comando:

    ```cpp
    int iAppearFlag = (iCmd == S2C::PLAYER_ENTER_WORLD)
        ? CECElsePlayer::APPEAR_ENTERWORLD : CECElsePlayer::APPEAR_RUNINTOVIEW;
    ```

    (`EC_ManPlayer.cpp:1845`.) O 17 é para quem **surgiu**; o 12, para quem **veio
    andando**. Usar 17 no streaming faria cada jogador que se aproximasse aparecer com
    efeito de teleporte.

    **2. A visibilidade é mútua, e quem se move escreve pelos dois.** Se eu ando na direção
    de alguém parado, só a minha atualização roda. Então `atualizar_visiveis` manda o
    comando ao outro jogador **e** mexe no `visiveis` dele (`passou_a_ver` /
    `deixou_de_ver`), em vez de esperar que a atualização dele chegue à mesma conclusão. A
    distância é simétrica e o raio é o mesmo, então as duas visões concordam — e por isso
    jogador **não** entra no teto de `TETO_DE_VISIVEIS`: cortar um jogador por causa de uma
    multidão de monstros quebraria essa simetria.

    **3. A saída do mundo saiu do link.** `BusServer::tirar_da_vista_de_todos`, chamado no
    `LOGOUT` e no `PlayerLogout` do barramento, manda `PLAYER_LEAVE_WORLD` (19) — e não
    `OBJECT_LEAVE_SLICE` (13): quem saiu do jogo não saiu do alcance, e o cliente distingue
    os dois (`bExit` em `CECPlayerMan::ElsePlayerLeave`).

    O `PlayerEntity` ganhou `sec_level`, lido uma vez no login
    (`CharacterRepository::nivel_de_gm`): ele viaja no `level2` da `info_player_1` e acende
    o `STATE_GAMEMASTER` (`0x4000`), que é o que põe a coroa sobre o avatar. O
    `jogadores_visiveis` do `gateway.rs` encolheu para o que ainda faz: o canal direto por
    processo que a **fala** usa, que é global e não depende de distância.

    ### c. Matéria: 5.125 recursos que nunca saíram do arquivo

    Ninguém mandava `MATTER_ENTER_WORLD` (18). A terceira correção à fila: o tipo no
    `npcgen.rs:426` é `SpawnType::ResourceMine`, não `SpawnType::Matter` — e o id já vem
    montado com `0xC0000000` (`:422`), que é exatamente o que `ISMATTERID` do cliente pede
    (`EC_GPDataType.h:27`).

    `MatterEntity` nova no mundo, populada de `init_spawns`; o log da subida agora diz
    **"21846 monstros, 3911 NPCs e 5125 recursos de mapa"**.

    Duas coisas que a matéria não compartilha com NPC:

    - **Sai por outro comando.** O `OBJECT_LEAVE_SLICE` (13) só trata `ISPLAYERID` e
      `ISNPCID` (`EC_GameDataPrtc.cpp:891-899`) — um id de matéria mandado por ele não faz
      nada, nem erro nem efeito. Matéria sai pelo `OUT_OF_SIGHT_LIST` (34), a lista que o
      cliente separa id a id pelas três máscaras (`:1056-1071`).
    - **Tem orçamento de fila próprio** (`TETO_DE_MATERIA = 40`, contra os 80 de criatura).
      Num campo de mineração, um teto só faria as pedras expulsarem os NPCs.

    O comando tem **25 bytes** de payload: `int mid, int tid, A3DVECTOR3 pos, unsigned char
    dir0, dir1, rad, state, value` — a `info_matter` (`EC_GPDataType.h:784-794`), idêntica à
    `INFO::matter_info_1` do original (`cgame/common/protocol.h:86-96`). Sem alinhamento,
    porque o cabeçalho do cliente está inteiro dentro de um `#pragma pack(1)` (`:563`).

    `dir0 = dir1 = rad = 0` é a peça em pé sem giro (`a3d_DecompressDir(0,0)` devolve o eixo
    Y, `A3DVectorComp.cpp:238-249`); `state = 0` é recurso comum, que faz o cliente ler o
    modelo do `elements.data` dele — o bit 0 marcaria objeto de modelo dinâmico e o bit 1,
    mina de espírito (`EC_Matter.cpp:167-168`).

    ### d. O nível da habilidade saiu do 1 fixo

    `PlayerEntity` ganhou `habilidades: HashMap<u32, u8>`, preenchido do
    `character_skills` que o login já carrega. O `CAST_SKILL` do cliente não manda o nível —
    quem tem de saber é o servidor — e ir ao banco a cada conjuração estava fora de questão.

    `NIVEL_DA_HABILIDADE` virou `NIVEL_MINIMO_DA_HABILIDADE`: agora é só o piso de quem
    conjura o que não aprendeu, não o nível de todo mundo. Os quatro pontos que o usavam
    (custo de mana, dano contra monstro, cura e dano entre jogadores) passaram a usar
    `nivel_da_habilidade(&jogador, skill_id)`.

    ### e. Personagem novo nasce com os atributos da classe

    O `INSERT` do `create_character` **não mencionava** `strength`, `agility`, `vitality` e
    `energy`: o banco usava o `DEFAULT 10` do esquema
    (`specs/01_DATABASE_SCHEMA_POSTGRES.sql:103-106`) e toda classe nascia 10/10/10/10. Não
    era cosmético — os quatro entram na vida e na mana máximas, na precisão e na evasão de
    `PlayerEntity::do_personagem`.

    O `create_character` recebeu `Option<pw_core::AtributosIniciais>`; o `gateway.rs` passa
    o que o `ptemplate.conf` do realm diz para a classe. Sem o arquivo vai `None` e continua
    valendo o padrão da coluna — menos errado do que inventar um número por classe no
    repositório. Medido no `ptemplate.conf` do 155BR:

    | seção | classe | força | agi | vit | energia |
    | :--- | :--- | ---: | ---: | ---: | ---: |
    | `[SWORDSMAN]` | Blademaster | 15 | 10 | 20 | 5 |
    | `[ORGE]` | Barbarian | 15 | 5 | 25 | 5 |
    | `[ASN]` | Assassin | 5 | 15 | 8 | 22 |
    | `[ANGEL]` | Cleric | 10 | 10 | 10 | 20 |

    Os personagens que já existem precisam de migração:
    `scripts/2026_09_09_atributos_iniciais_por_classe.sql`. Ele só toca em quem **ainda está
    no ponto de partida** — nível 1, sem pontos livres, e com os quatro exatamente em 10 —
    porque quem distribuiu pontos pode ter chegado a 10 em algum deles de propósito.

    ### f. Um achado que não estava na fila: o `elements.data` do 155BR decodifica pela
    metade

    Ao procurar o `MINE_ESSENCE` para a matéria, ele veio **vazio**. Medindo os dois
    arquivos com `load_elements_data_auto`:

    | realm | versão | tabelas | vazias | `MINE_ESSENCE` | `FASHION_ESSENCE` | `PLAYER_ACTION_INFO_CONFIG` |
    | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
    | `realm_155` | **159** | 234 | 16 | 1565 | 3064 | 1354 |
    | `realm_155BR` | **156** | 231 | **132** | 0 | 0 | **7736** |

    O realm que o docker de teste serve é o **155BR**, e ele desalinha no
    `PLAYER_ACTION_INFO_CONFIG` (índice 73): 7.736 registros onde o `walk_report` do arquivo
    de referência mostra 162, e daí em diante quase tudo vem zerado ou lixo —
    `FASHION_ESSENCE`, `PET_ESSENCE`, `MINE_ESSENCE`, `SUITE_ESSENCE`, 132 tabelas ao todo.

    A causa está escrita no próprio arquivo de overrides:

    ```json
    "verified_against": "data/realm_155/config/elements.data",
    "72": { "abs_count_off": 44335619, ... },
    "77": { "abs_count_off": 48163839, ... },
    ```

    São **deslocamentos absolutos**, verificados contra um arquivo que hoje não está mais
    naquele caminho — `data/realm_155` é um v159, de 55.170.911 bytes, e o 155BR é um v156
    de 55.442.775. O cabeçalho do `realm_155_overrides.json` já avisava: "NAO sao garantidas
    validas para outro arquivo v156 qualquer".

    **Isto corrige a memória `pw_ctx_a_155_funcional`**, que diz "elements.data v156 100%
    decodificado". O que está 100% decodificado é o **v159** do `realm_155`. O v156 que o
    realm de teste usa está a 99 tabelas de 231.

    Não foi consertado nesta sessão — derivar as âncoras certas para este arquivo é
    investigação própria, do mesmo tamanho da que produziu as atuais. Nada do que esta
    sessão entregou depende delas: a armadura vem do `ARMOR_ESSENCE` (índice 6, muito antes
    do ponto onde desalinha) e a matéria não precisa de tabela nenhuma do lado do servidor —
    o cliente lê o modelo do `elements.data` **dele**, pelo `tid`.

    ### g. Provas

    - `pw-data-loader/tests/ficha_da_armadura.rs`, 6 testes contra o `elements.data` do
      realm: as três famílias são disjuntas, nenhuma peça com nível exigido viaja com
      máscara zerada (as quatro que viajam com zero têm `require_level = 0` e defesa 0), a
      busca por id devolve a variante certa, e os campos da primeira armadura do Sacerdote
      batem um a um.
    - `protocol_tests`: o bloco da armadura e o do acessório campo a campo, com a essência
      de 36 bytes e a ordem de cada uma; `MATTER_ENTER_WORLD` com os 25 bytes exatos e os
      cinco últimos zerados; `OUT_OF_SIGHT_LIST` com a contagem antes dos ids.
    - `subcomandos_no_mundo`: `dois_jogadores_se_veem_se_perdem_e_se_reencontram` (o ciclo
      inteiro, com o jogador parado recebendo tudo sem se mexer),
      `quem_sai_do_mundo_some_da_tela_de_quem_ficou`,
      `o_minerio_do_mapa_entra_pelo_comando_de_materia_e_sai_pela_lista`.
    - `pw-storage/tests/atributos_ao_criar.rs`: as três leituras vêm do banco, porque a
      falha morava na lista de colunas de um `INSERT` — o código Rust nunca dizia "10".
    - `ptemplate_tests::os_atributos_iniciais_saem_por_classe_e_nao_sao_todos_dez`.
    - `subcomandos_s2c_contra_o_ir` pegou os três codificadores novos antes de mim: eles
      não estavam na tabela `INTENCAO`. O `player_enter_world` voltou a escrever o próprio
      cabeçalho (o corpo comum é que virou função) justamente para continuar sob essa rede.

    Medido em 2026-09-09: **70 suítes verdes, 461 testes**, e as únicas falhas continuam
    sendo as duas do 1.2.6 no `loader_tests`.

    ### h. O que continua faltando

    - **`MINE_ESSENCE` e as outras 131 tabelas do 155BR** (item f). Enquanto não voltarem, o
      servidor não sabe o que cada minério dá ao ser colhido — e **colher não existe**:
      `MATTER_PICKUP` (152) não é tratado. Hoje o minério aparece e é decoração.
    - **A durabilidade dos itens é escrita no código.** A loja grava 10000 e a criação de
      personagem também, enquanto o `durability_min` de cada peça está no `elements.data` e
      já é lido (`RequisitosDoEquipamento::durabilidade`). O tooltip vai mostrar
      1.000.000/1.000.000 numa armadura comprada.
    - **`weapon_level` vai fixo em 1** no bloco da arma; o original manda `ess->level`
      (`generate_item_temp.h:317`). E a `attack_speed` da arma vai zerada, enquanto o
      original a tira do `WEAPON_SUB_TYPE` (`(int)(subtype->attack_speed*20 + 0.1)`,
      `:344`) — tabela que já está decodificada e que ninguém lê ainda.
    - **Não há trava de PvP** (do item 40, e continua de pé).
    - O `dir` continua zerado no streaming, agora também para jogador: a grade guarda
      posição, não direção.
    - As duas instâncias de matéria de cada gerador nascem **na mesma coordenada** — o
      `npcgen.rs` usa `area.pos` para todas, sem espalhar. Duas pedras uma dentro da outra.

42. **Sessão 2026-09-11: o teste em jogo com dois clientes — o modelo masculino, o
    teleporte enterrado, os monstros no ar, e o que o overlay não pode mostrar.**

    Sete relatos do Murillo depois do item 41. Cinco viraram correção, um virou resposta
    (o instrumento funciona diferente do que eu disse) e um continua aberto com o que foi
    medido.

    ### a. A arma certa, para referência

    Confirmado em jogo: a arma equipada aparece **branca** e o tooltip mostra a restrição de
    classe. É o estado correto, e fica registrado como referência para comparar quando algo
    voltar a ficar vermelho:

    - O nome e o tipo saem do `elements.data` **do cliente**, pelo id do item.
    - A linha de profissão, as de atributo e a de nível saem do **bloco de dados** que o
      `OWN_ITEM_INFO` (40) carrega — o que o item 38 corrigiu para arma e o 41a para
      armadura e acessório.
    - Vermelho (`A3DCOLORRGB(192, 0, 0)`) quer dizer `CanUseEquipment` recusando, e são só
      cinco motivos (`EC_HostPlayer.cpp:4894-4980`): item nulo, atributo/nível/reputação
      insuficiente, profissão fora da máscara, sexo errado (moda) e munição ausente (arma
      de longo alcance). **Peça sem bloco de dados cai no terceiro**, com a máscara em
      zero.

    ### b. O modelo masculino de barba: o sexo viaja num bit que ia zerado

    Duas sacerdotisas se afastaram além do raio de visão; ao voltarem, **cada uma viu a
    outra como modelo masculino, com cabelo e barba de padrão**.

    Os logs do realm deram o diagnóstico antes de qualquer leitura de fonte: na sessão
    inteira houve **dois** `PlayerBaseInfo` (um por jogador, no primeiro encontro) e
    **zero** `GetCustomData`. Na reentrada o cliente não pede nada — ele já tem aquele
    jogador em cache. Então o único sexo que ele tem à mão é o do pacote:

    ```cpp
    unsigned char GetGender() const {
        return (state2 & GP_STATE2_GENDER) ? GENDER_FEMALE : GENDER_MALE;
    }
    ```

    (`EC_GPDataType.h:709-711`, usado em `CECElsePlayer::InitFromCache`,
    `EC_ElsePlayer.cpp:220`.) E nós mandávamos `state2 = 0` para todo mundo — afirmando que
    todo jogador é homem. O original liga o mesmo bit em `SetPlayerClass`
    (`gs/player_imp.h:1886-1891`; `STATE_PLAYER_GENDER = 0x40`, `gs/object.h:202`).

    Junto com ele foram os dois carimbos que iam zerados no mesmo pacote, `crc_e` e `crc_c`
    — no original, `pObject->crc` e `pObject->custom_crc` (`protocol_imp.h:197-200`). São
    eles que dizem ao cliente se o que está em cache ainda vale
    (`m_bCustomReady = (m_PlayerInfo.crc_c == Info.crc_c)`, `EC_ElsePlayer.cpp:176`), e o
    `crc_c` **tem de ser o mesmo valor** que o `custom_stamp` do `PlayerBaseInfo_Re` daquele
    personagem (`:1881`). Zero fixo nos dois lados faz o cliente nunca perceber uma troca de
    visual.

    Os campos viraram `pw_core::VistaDoJogador`, uma struct só, porque os dois codificadores
    que a usam escrevem os mesmos campos na mesma ordem e um parâmetro trocado entre eles
    seria invisível.

    **Nota sobre o 1.2.6:** lá o `state2` não existe, e o cliente só sabe o sexo pelo
    `PlayerBaseInfo_Re`. Aquela versão nunca teve este defeito.

    ### c. O teleporte de GM enterrava de novo — agora há mapa de alturas

    O item 37 tinha resolvido isto mantendo a altura atual do jogador, e escrito por quê:
    não havia como consultar o chão. O remendo funciona em terreno plano e falha em
    qualquer encosta, que foi o que voltou a acontecer.

    O mapa existe, e o formato estava a uma leitura de distância: `CTerrain`
    (`EvolvedPWServer/cgame/gs/terrain.cpp`) monta o terreno de `map/<n>.hmap`, e a
    configuração de cada mapa está no `gs.conf` do pacote do servidor. Módulo novo
    `pw-data-loader/src/terreno.rs`:

    | | |
    | :--- | :--- |
    | arquivo | `map/<n>.hmap`, `n` de 1 a colunas×linhas |
    | conteúdo | `(nAreaWidth+1)²` floats little-endian **em 0..1**, sem cabeçalho |
    | tamanho | 513² × 4 = **1.052.676 bytes**, exatamente o dos arquivos do realm |
    | altura | `h × (vHeightMax − vHeightMin) + vHeightMin` |
    | origem | `ox = −(vert×colunas×célula)/2`, `oz = +(vert×linhas×célula)/2` |
    | busca | `h = (x−ox)/célula`, `v = (oz−z)/célula` — **o `z` é invertido** |

    A conferência de que a fórmula está certa: para o mundo principal ela dá
    `x ∈ [−4096, 4096]`, `z ∈ [−5632, 5632]`, que é exatamente o `base_region` que o
    `gs.conf` declara para o `gs01`. Medido no realm: 88/88 blocos, 88 MB, alturas de 15,1
    a 633,8 m.

    A configuração por mapa **não dá para deduzir da pasta** — 88 arquivos tanto podem ser
    8×11 quanto 11×8 — então ela foi extraída do `gs.conf` para
    `specs/mapas/terreno_155.json` (26 mapas). O terreno é carregado **só do mapa que
    aquele servidor de mundo serve**: são 88 MB, e o realm tem 68 pastas de mapa.

    O teleporte passou a fazer o que o original faz
    (`playercmd.cpp:4926`), mais meio metro de folga — a nossa altura vem da interpolação
    do `.hmap`, e o cliente tem a malha real por cima; num telhado ou numa ponte, chegar
    rente ao terreno é chegar dentro da geometria.

    ### d. Os monstros no ar: o diagnóstico mudou no meio do caminho

    A primeira leitura foi que o `y` do `npcgen.data` seria um **deslocamento** acima do
    chão — é o que `GenerateY` sugere (`npcgenerator.cpp:4319-4322`). **O teste contra os
    dados reais derrubou isso na hora**: somar chão + `y` punha tudo a 436 m.

    O que os 30.898 spawns do mundo dizem, medidos contra o mapa de alturas:

    - O `y` do arquivo **é altura absoluta e está certo**: no centro da área ele fica a
      **0,10 m** do chão, e esse 0,10 é constante nos quartis 25, 50 e 75 — é o
      `offset_terrain` (`fOffsetTrn`) que o gerador do original soma.
    - Quem erra é a **nossa dispersão**. `posicao_na_area` (`npcgen.rs:117`) espalha os
      monstros sorteando `x` e `z` dentro da caixa da área e **copia o `y` do centro**. Em
      encosta, o monstro fica na altura do centro. O próprio código já documentava a
      lacuna: *"Sem altura de terreno… é isso que falta para fechar com o original"*.
    - **Nem toda área é de chão.** 1.083 das 10.172 áreas têm o centro a dezenas ou
      centenas de metros do terreno, **para os dois lados**: deslocamento negativo é
      caverna (até −259 m), positivo é gerador aéreo ou cidade de vários níveis (até
      +366 m). O mapa de alturas não representa nada disso, e o original também não —
      ele preserva o deslocamento do gerador.

    A correção reproduz `terrain_gen_pos` sem precisar extrair o `fOffsetTrn`, recuperando-o
    do que já temos:

    ```text
    deslocamento = centro.y − chão(centro.x, centro.z)
    y final      = chão(x, z) + deslocamento
    ```

    Medido: dos 25.465 spawns de área de chão, **34,2% estavam a mais de 2 m do chão e
    agora são 0%**, e os 5.433 de caverna ou gerador aéreo continuam onde estavam.

    Para isso o `SpawnInstance` ganhou `centro_da_area`. O leitor de `npcgen.data` continua
    sem depender do terreno — quem resolve a altura é `WorldInstance::init_spawns`, que é
    onde o mapa está carregado.

    ### e. O jogador que nasce enterrado

    Com o mapa em mãos deu para medir as seis posições de nascimento de
    `CharacterClass::default_spawn_position`:

    | classes | diferença para o chão |
    | :--- | ---: |
    | Guerreiro / Mago | −0,0 m |
    | Bárbaro / Feiticeira | +0,0 m |
    | Arqueiro / Sacerdote | +2,4 m |
    | **Assassino / Psíquico** | **−71,1 m** |
    | **Guardião / Místico** | **+10,7 m** |
    | **Ceifador / Tormentador** | **+39,9 m** |

    As três primeiras são coordenadas de cidade de verdade; as três últimas são palpites —
    e os números `(650,130,130)`, `(380,230,230)`, `(150,250,250)` denunciam isso sozinhos,
    com os dois últimos componentes repetidos.

    Achar as coordenadas certas é outra investigação (o `[TOWN_REGION]` do `ptemplate.conf`
    **não** serve: ele é o mapa de ressurreição, `__GetTownPosition`, não o nascimento).
    O que dá para fazer agora é a regra do próprio original, do gerador de volume
    (`box_gen_pos::GenerateY`, `npcgenerator.cpp:4340-4345`): **o terreno é piso, nunca
    teto.** Quem entra no mundo abaixo do chão sobe para a superfície; quem está acima
    fica, porque altura acima do chão é legítima (voo, prédio, ponte).

    ### f. O overlay não mostra o que eu disse que ele mostraria

    O Murillo reportou que os comandos 12, 18 e 34 não aparecem no `d_rtdebug`. **Não
    aparecem mesmo, e não é defeito** — a instrução que eu dei no fim do item 41 estava
    errada.

    O que o overlay imprime de rede são **três** linhas, e só duas delas sempre existem
    (`EC_GameDataPrtc.cpp:798-816`):

    | linha | quando | compilada? |
    | :--- | :--- | :--- |
    | `SERVER - Unknown GAMEDATA_n` | o cliente não conhece o comando | sempre |
    | `SERVER - Invalid X size(Network:a, Client:b)` | o tamanho não bate | sempre |
    | `SERVER - X(n)` | todo comando aceito | **`#ifdef LOG_PROTOCOL`** |

    `LOG_PROTOCOL` não é definido em lugar nenhum do fonte do cliente — só testado em
    `EC_Global.h:116`. No binário que o Murillo roda, o terceiro caso está compilado fora.

    Ou seja: **o overlay só fala de comando que ele recusou.** Um comando que chega e é
    aceito passa em silêncio, e é por isso que ele resolveu os itens 33, 34, 38 e 46 — todos
    eram comandos recusados. "12, 18 e 34 não aparecem" é a notícia boa: chegaram e foram
    aceitos.

    (Há uma opção de linha de comando `rtdebug_hide:<nomes>` que filtra protocolos, e
    `rtdebug:<nível>`, mas a lista de escondidos nasce **vazia** — não é ela que está
    calando nada.)

    ### g. A velocidade: o valor no fio está certo, e o que sobrou para medir

    Não achei defeito no servidor, e vale registrar o que foi descartado:

    - `[ANGEL] run_speed = 2.8` no `ptemplate.conf` do realm, e é isso que
      `PlayerEntity::move_speed` carrega e o `OWN_EXT_PROP` manda. O original manda o mesmo
      campo, sem multiplicador (`playertemplate.cpp:294`, `:676`; `player.cpp:2249`).
    - O `OWN_EXT_PROP` **não** está sendo descartado: os 196 bytes foram medidos em jogo no
      item que criou o comando, com o overlay dizendo `Client:196`. O fonte do
      `EvolvedPWClient` diz 228 (ele tem quatro campos marcados `// NEW` que este binário
      não tem) — é o caso clássico de "o binário fica entre o IR e o fonte".
    - O cliente usa `m_ExtProps.mv.run_speed` para correr e `walk_speed` para andar
      (`EC_Player.cpp:6599`). **Se o personagem estiver em modo de caminhada, ele anda a
      1,4 m/s** — metade. É a primeira coisa a conferir.

    O que **estava** errado no mesmo pacote, e foi corrigido: o `attack_speed` ia em
    segundos (2) onde o cliente espera *ticks* de 50 ms (`EC_RoleTypes.h:228`), ou seja um
    intervalo declarado de 0,1 s.

    Como medir sem adivinhar: a ficha do personagem imprime `mv.run_speed`
    (`DlgCharacter.cpp:491`). Se ela mostrar 2.8, o servidor cumpriu a parte dele.

    ### h. O jogo base, peça por peça

    **Comprar.** O bloqueio era duplo. O primeiro: os dois personagens de teste têm
    `money = 0` no banco — `deduct_money` falha e a compra é recusada em silêncio. O
    segundo: a loja cobrava **100 moedas fixas por unidade, de qualquer coisa**. Agora o
    preço sai do `elements.data`, pela fórmula do original
    (`serviceprovider.cpp:241-252`): `max(shop_price, price)`, com as taxas em 1. A varredura
    é por **nome de campo** (`price` + `shop_price`), o que cobre as 25 tabelas que os têm de
    uma vez — 10.724 itens no realm. Item sem preço **não é vendido**, em vez de sair por um
    número inventado. A durabilidade do item comprado também passou a sair do arquivo, e não
    dos 10000 fixos que faziam o tooltip mostrar 1.000.000/1.000.000.

    Para testar antes de as missões pagarem:

    ```sql
    UPDATE characters SET money = 500000 WHERE id IN (40, 42);
    ```

    **Subir habilidade.** `GP_NPCSEV_LEARN` (9) caía no ramo de "serviço ainda não tratado":
    clicar em aprender não fazia nada. O pedido é um `int idSkill` e nada mais — o cliente
    **não** manda o nível (`EC_SendC2SCmds.cpp:3379-3405`) — e a resposta é `LEARN_SKILL`
    (95) com id e nível novo. Implementado: sobe um nível (teto 10), grava no
    `character_skills` e avisa o cliente. **Não cobra nada ainda** — SP, moedas e requisitos
    saem do `NPC_SKILL_SERVICE` e do `SKILLTOME_ESSENCE`, e arbitrar um custo aqui seria
    repetir o erro das 100 moedas fixas. O cliente confere os requisitos dele antes de
    mandar.

    **Spawn inicial por classe.** Ver (e): três das seis posições são palpites, e a regra do
    piso evita o pior caso. As coordenadas certas continuam faltando.

    **Missões.** Não foi tocado nesta sessão. `ACEITAR_MISSAO`/`ENTREGAR_MISSAO`/
    `ITEM_DE_MISSAO` já têm tratamento (`BusServer::missao`), e o que falta medir é o que o
    cliente faz com a resposta — próxima sessão, com o realm já de pé.

    ### i. E um que ninguém pediu: 191 pedidos sem resposta

    `CALC_NETWORK_DELAY` (C2S 128) apareceu **191 vezes** no log de uma sessão de teste, todas
    caindo em "subcomando ainda não tratado". É o medidor de latência do cliente; a resposta
    é `CALC_NETWORK_DELAY_RE` (291) devolvendo o `timestamp` recebido **sem tocar nele** (o
    cliente descarta se não bater, `EC_GameRun.cpp:3155`). O valor só alimenta o indicador de
    ping (`DlgSystem.cpp:99`) — não mexe em movimento nem em combate, o que também **descarta**
    a hipótese de ele ser a causa da velocidade baixa. Respondido, e o log volta a ser legível.

    ### j. Provas

    - `pw-data-loader/tests/terreno_do_realm.rs`, 4 testes contra os `.hmap` de verdade: os
      88 blocos com o tamanho que o `gs.conf` promete, alturas dentro de
      `vHeightMin..vHeightMax` e com relevo real, fora do mapa devolvendo `None` em vez de
      inventar, e a conta dos spawns (34,2% → 0%, com os 5.433 de caverna/aéreo preservados).
    - `terreno.rs`, 5 testes de unidade: origem e inversão do `z`, interpolação numa rampa,
      fora do mapa, sem blocos, e o mundo principal batendo com o `base_region` do `gs.conf`.
    - `protocol_tests`: o bit do sexo no `state2` (com o tamanho do comando inalterado), o
      carimbo de aparência concordando nos dois caminhos e zerando nas duas formas de "sem
      aparência", e a resposta de latência devolvendo o relógio do cliente.
    - `subcomandos_no_mundo`: a loja cobrando o `shop_price` do arquivo, o item sem preço
      **não** sendo vendido nem cobrado, e o treinador subindo a habilidade no mundo **e**
      no banco.
    - `precos.rs`: só entra tabela que tem os dois campos; id inválido fica de fora.

    Medido em 2026-09-11: **72 suítes verdes**, e as únicas falhas continuam sendo as duas
    do 1.2.6 no `loader_tests`.

    Corrigido de passagem: `itens_sobrevivem.rs` gerava o nome do realm só com o relógio, e
    os quatro testes em paralelo colidiam em `duplicate key` de vez em quando — falha
    intermitente vista nas duas últimas sessões.

    ### k. O que continua faltando

    - **As coordenadas de nascimento** de Abissais, Guardiões e Sombrios.
    - **O custo de subir habilidade** (SP, moedas, requisitos de nível e cultivo).
    - **As missões**, que é o que põe dinheiro no bolso do jogador sem `UPDATE` no banco.
    - **O `crc_e`** (carimbo de equipamento) vai zerado: o cliente repede o equipamento a
      cada reaparição. Custa um pedido, não desenha ninguém errado.
    - Tudo o que o item 41k já listava e não foi tocado: `MINE_ESSENCE` e as outras 131
      tabelas do 155BR, colher matéria (`MATTER_PICKUP`), `weapon_level` fixo em 1, a
      `attack_speed` da arma vinda do `WEAPON_SUB_TYPE`, e a trava de PvP.

43. **Sessão 2026-09-11 (continuação): a criação de personagem. Onde o original guarda o
    molde de cada classe, e por que o Bárbaro nasceu com arma de mago.**

    O Murillo criou o PANDAH, Bárbaro, e relatou quatro coisas. As quatro têm a mesma raiz,
    e ela não estava no código.

    ### a. Onde o original guarda isto — a resposta à pergunta "de que arquivo vem?"

    De arquivo nenhum. O servidor original monta o personagem novo a partir de um
    **personagem-molde por classe gravado no próprio banco**:

    ```cpp
    // cnet/gamedbd/dbcreaterole.hrp:51-56
    GameDBManager::GetInstance()->GetClsDetail(arg->roleinfo.occupation, arg->roleinfo.gender,
                                               base, status, pocket, equipment, storehouse)
    ```

    e `GetClsDetail` (`gamedbmanager.cpp:404-460`) lê as tabelas `base`, `status`,
    `inventory`, `equipment` e `storehouse` na chave `GetDataRoleId(cls)`. Esses moldes
    entram no banco de um arquivo binário, `gamedbd/clsconfig`, importado para os roleids
    **16 a 31** (`ImportClsConfig`, `cnet/gamedbd/clsconfig.h:22-86`).

    Ou seja: **posição de nascimento, itens de bolsa e equipamento inicial são dado de
    realm, não de código** — exatamente o que a nossa tabela `class_templates` já modela. A
    arquitetura estava certa; o conteúdo é que estava errado.

    O `clsconfig` do pacote 1.5.5 (73.728 bytes) é um banco binário com os valores
    marshalados no formato GNET. Uma varredura por trios de float plausíveis não achou as
    posições, então extrair dali continua sendo investigação própria.

    ### b. O Bárbaro com arma de mago: os ids da tabela não batiam com os nomes

    A tabela do realm tinha **seis** das doze classes, e o pareamento estava trocado:

    | `cls` | `name` na tabela | classe de verdade |
    | ---: | :--- | :--- |
    | 3 | Bárbaro | Feiticeira (`Venomancer`) |
    | 4 | Feiticeira | **Bárbaro** (`Barbarian`) |

    `create_character` procura pelo **id**. O Bárbaro (4) recebeu o molde escrito como
    "Feiticeira": **Graveto de Madeira (2867)**, que é arma de magia — o "graveto com ponto
    de interrogação" do relato. O certo é o Porrete com Espinhos (2258), e isso o código já
    sabia: `CharacterClass::default_weapon_id` está conferido contra o `WEAPON_ESSENCE` pelo
    teste `armas_iniciais_batem_com_o_elements`.

    Nada no código podia perceber: tabela é dado, e dado errado atravessa compilador e tipo
    sem esbarrar em nada. O que pega é conferir a tabela contra a fonte que **está**
    conferida — ver (f).

    ### c. O kit de bolsa tinha um item que ninguém de nível 1 pode usar

    Os três itens eram iguais para toda classe, e um deles não serve a personagem novo:

    | id | nome | `require_level` |
    | ---: | :--- | ---: |
    | 2100 | Portal da Cidade | — (`TOWNSCROLL_ESSENCE`) |
    | 1796 | Poção Pequena de Cura | **0** |
    | 1801 | Poção Perfeita de Cura | **30** |

    O 1801 saiu; entrou o **1804, Poção Pequena do Espírito**, que é o par de mana do 1796 e
    também tem `require_level = 0`. O critério é esse e está escrito no teste: só item de
    nível 0 no kit.

    O mesmo kit errado existia num **segundo** lugar — o caminho de reparo de
    `get_details`, que preenche itens de personagem antigo que não tenha nenhum. Lá a arma
    ainda ia para a **bolsa** em vez do slot de equipamento. Os dois foram alinhados.

    ### d. O Bárbaro gravado como Humano

    `race = 0` no banco, para um Bárbaro. A raça vinha do campo `race` do `RoleInfo` que o
    cliente manda — e **aquele campo não é a raça**. No original, o campo de mesmo nome do
    `GRoleBase` guarda `classe | 0x80000000` quando o personagem é mulher:

    ```cpp
    inline bool IsPlayerFemale() { return ((gplayer *)_parent)->base_info.race < 0; }
    inline void SetPlayerClass(int cls, bool gender) {
        if (gender) { pPlayer->base_info.race = cls | 0x80000000; ... }
        else        { pPlayer->base_info.race = cls; }
    }
    ```

    (`gs/player_imp.h:1883-1895`.) A raça passou a sair da classe
    (`CharacterClass::race`), com o mesmo pareamento que o `ptemplate.conf` usa e que
    `default_spawn_position` já usava para agrupar os nascimentos: duas classes por raça.

    ### e. Vida pela metade no nascimento

    `HP 260/490` na tela. O máximo (490) é calculado pelo mundo — `ptemplate.conf` mais o
    `CHARRACTER_CLASS_CONFIG` — e o atual (260) vinha de `CharacterClass::default_hp_mp`,
    uma tabela por classe escrita no código. **Duas contas para a mesma coisa, e elas
    divergiram.**

    A conta virou uma só, `BaseDaClasse::vida_e_mana_maximas`, usada pelo mundo ao carregar
    o personagem **e** pela criação ao gravá-lo:

    ```text
    max_hp = hp + lvlup_hp × (nível − 1) + vit_hp × vitalidade
    max_mp = mp + lvlup_mp × (nível − 1) + eng_mp × energia
    ```

    Para o Bárbaro no nível 1: `65 + 17×25 = 490`, e é com 490 que ele passa a nascer.

    Os atributos, esses, já estavam certos — o item 42 os ligou ao `ptemplate.conf`, e o
    PANDAH nasceu com 25/15/5/5, que é exatamente o `[ORGE]` do arquivo.

    ### f. A rede que faltava

    `pw-storage/tests/template_de_classe.rs`, quatro testes contra o banco de verdade:

    - as doze classes existem no molde, e o `cls` de cada linha é uma classe do jogo;
    - **a arma do molde é a que `default_weapon_id` diz** — é este que teria pego o Bárbaro
      com 2867;
    - as duas classes de uma raça nascem no mesmo ponto (vila inicial é da raça, não da
      classe);
    - nenhum item do kit exige nível acima de zero.

    O molde do realm foi regravado por `scripts/2026_09_11_template_de_classe_155br.sql`,
    com as doze classes, a arma certa de cada uma e o kit de nível zero.

    ### g. A velocidade, de novo — e a resposta

    Os **três** `ptemplate.conf` do pacote (o do `pwserver_155v156`, o do `home155` e o do
    realm) dizem a mesma coisa para o Bárbaro: `run_speed = 2.8`. O original manda esse
    campo sem multiplicador nenhum (`playertemplate.cpp:294` e `:676`,
    `player.cpp:2249`).

    Então 2,8 é o que a configuração do 1.5.5 manda, e o servidor está cumprindo. Se o
    número desejado é outro, isso é **ajuste de realm** — muda-se o `run_speed` da seção da
    classe no `ptemplate.conf` do realm e reinicia. Não há defeito para corrigir aqui, e
    inventar um multiplicador seria o oposto do que este projeto faz.

    ### h. O que continua faltando

    - **As coordenadas de nascimento de verdade.** Três das seis (Humanos, Alados, e a dos
      Selvagens que o Murillo diz cair no "campo da expedição") vêm de palpites antigos; as
      dos Abissais, Guardiões e Sombrios idem. A fonte é o `clsconfig`, ainda não
      decodificado. O caminho prático está escrito no cabeçalho do script: andar até o
      ponto certo, ler `pos_x/y/z` do banco e gravar nas duas classes da raça.
    - **`tasks.data` decodifica zero tarefas** neste realm — achado ao procurar os itens
      iniciais na recompensa da missão de nascimento. É o mesmo buraco do item 42h
      ("missões não foram tocadas"), e agora com um número: zero.
    - As colunas de atributo de `class_templates` existem e **são ignoradas** — quem manda
      é o `ptemplate.conf`. Quando o molde passar a ser editável pelo painel, é uma decisão
      a tomar: ou o molde vence, ou as colunas saem.

44. **Sessão 2026-09-12: o teste do POTATO, a VM 1.2.6 como gabarito, e o plano para a
    jogabilidade básica.**

    O Murillo criou o POTATO (Bárbaro, realm 155BR), jogou, e comparou lado a lado com o
    servidor 1.2.6 funcional que roda na VM `192.168.1.200`. Este item enumera **cada**
    ponto relatado, com o estado de cada um, responde se aquela VM serve para alguma coisa
    (serve, e muito), e registra o plano.

    ### a. Os pontos relatados, um por um

    Estado: **corrigido** (com prova), **diagnosticado** (causa conhecida, correção por
    fazer) ou **a investigar** (causa ainda não medida).

    **No 1.5.5, com o POTATO:**

    | # | Relato | Estado |
    | ---: | :--- | :--- |
    | 1 | Nasceu no "Campo da Expedição", que não é área inicial | diagnosticado — (b1) |
    | 2 | Corre a 2,8 m/s; na VM 1.2.6 são 4,9 | **corrigido** — (b2) |
    | 3 | Atributos CON 25, FOR 15, INT 5, DES 5 — sem problema | registrado — (b3) |
    | 4 | Arma inicial e habilidade inicial corretas | confirmado |
    | 5 | Na "Campo da Passagem Norte" (área inicial dos Selvagens no 1.2.6) os monstros são nível 20; no 1.5.5 a área inicial deve ser outro mapa | a investigar — (b5) |
    | 6 | A animação de ataque não executa, e o monstro atacado não reage | a investigar |
    | 7 | O monstro persegue rápido demais e **por baixo da terra** | diagnosticado em parte — (b7) |
    | 8 | Habilidades sem recarga: dá para conjurar sem parar | a investigar |
    | 9 | "Reviver na cidade mais próxima" revive no mesmo lugar, na hora | a investigar |
    | 10 | Vida e mana não regeneram sozinhas | a investigar — (b10) |
    | 11 | Um NPC (Guia) flutua acima do chão | a investigar |
    | 12 | O Eufórbio (matéria) nasce todo no mesmo ponto | diagnosticado — (b12) |
    | 13 | Aparece "+100 de experiência e +23 de alma", mas nada é gravado | a investigar |

    **Na VM 1.2.6, para comparação:**

    | # | Observação | Estado |
    | ---: | :--- | :--- |
    | 14 | Os quatro atributos começam em 5; corre a 4,9 m/s | explicado — (b2), (b3) |
    | 15 | Ao nascer aparece um guia do jogo | a investigar — (b15) |
    | 16 | A barra de atalhos vem preenchida: F1 ataque, F2 habilidade inicial, F3 segunda habilidade (quando a classe tem duas), F4 pegar item, F5 meditar, F8 Portal da Cidade | a investigar — (b15) |

    **O que foi pedido:** combate básico inteiro, experiência, alma, moedas, animações
    certas, habilidades com animação, conjuração e recarga certas, mapa inicial e missões
    iniciais — começando por deixar `elements.data`, `tasks.data` e `npcgen.data` a 100%.

    ### b. O que já se sabe de cada um

    **(b1) O nascimento.** No servidor original a posição de nascimento, os itens e o
    equipamento vêm de um personagem-molde por classe gravado no banco (item 43a: o
    `gamedbd/clsconfig`, importado para os roleids 16 a 31). O nosso molde usa coordenadas
    antigas que ninguém conferiu — e a dos Selvagens é o "Campo da Expedição". **A VM 1.2.6
    tem esses moldes já importados no `gamedbd` dela** — ver (c).

    **(b2) A velocidade — corrigida, e a causa era leitura errada da hierarquia dos
    arquivos.** O original lê o `ptemplate.conf` e **depois** sobrescreve oito campos com o
    `CHARRACTER_CLASS_CONFIG` do `elements.data`:

    ```cpp
    // gs/playertemplate.cpp:293-301, dentro de __LoadDataFromDataMan
    _template_list[cls].walk_speed   = config.walk_speed;
    _template_list[cls].run_speed    = config.run_speed;
    _template_list[cls].swim_speed   = config.swim_speed;
    _template_list[cls].flight_speed = config.fly_speed;
    _template_list[cls].attack_speed = (int)(config.attack_speed*20);
    _template_list[cls].attack_range = config.attack_range;
    _template_list[cls].hp_gen       = config.hp_gen;
    _template_list[cls].mp_gen       = config.mp_gen;
    ```

    As velocidades do `.conf` são **valores mortos** no original. O item 43g afirmou que 2,8
    era "o que a configuração do 1.5.5 manda" — estava errado: os três `ptemplate.conf`
    concordavam entre si, e nenhum deles é o que vale.

    Três fontes independentes fecham o número, sem nenhuma suposição:

    | fonte | andar | correr | nadar | voar |
    | :--- | ---: | ---: | ---: | ---: |
    | `CHARRACTER_CLASS_CONFIG` do Bárbaro (155BR **e** 155) | 2,0 | **4,9** | 3,0 | 5,0 |
    | `OWN_EXT_PROP` capturado na VM 1.2.6 | 2,0 | **4,9** | 3,0 | 5,0 |
    | o Murillo em jogo, na VM | | **4,9** | | |

    O personagem passou a tirar do `elements.data` as quatro velocidades, a cadência de
    ataque (0,8 s → 16 ticks, e não os 30 do `.conf`), o alcance (2,5 m) e a regeneração
    (`hp_gen = 4`, `mp_gen = 1` para o Bárbaro). O `OWN_EXT_PROP` deixou de mandar
    `(2, 2)` e `(1,5; _; 2,0; 4,0)` escritos no código. Prova:
    `classes_tests::as_velocidades_do_barbaro_sao_as_do_elements_e_nao_as_do_ptemplate`.

    **(b3) Os atributos.** A captura da VM 1.2.6 mostra um personagem de nível 1 com
    **5/5/5/5 e 5 pontos livres** (`status_point`), e o fonte do original trata 5 como piso
    de cada atributo (`player_template::__Rollback`, `gs/playertemplate.cpp:596-640`:
    `vit = 5 - data.vitality`, com o comentário "3->5, fix bug"). O `ptemplate.conf` do
    1.5.5 distribui por classe (25/15/5/5 para o Bárbaro). O Murillo aceita a distribuição
    do 1.5.5, então fica como está — mas **a origem dos atributos iniciais ainda não está
    provada**: é o molde de (b1) que decide no original, e a VM responde isso.

    **(b5) A área inicial com monstros de nível 20.** Hipótese do Murillo: no 1.5.5 os
    personagens novos nascem noutro mapa, com área de evolução até o 20. É verificável sem
    palpite — o molde de (b1) tem o `worldtag` de nascimento, e o `gs.conf` do 1.5.5 lista
    26 mapas com `base_path` (item 42c).

    **(b7) Monstro rápido e por baixo da terra.** Duas causas prováveis, medíveis: a IA move
    o monstro em linha reta em 3D sem assentar a altura no terreno — o mapa de alturas
    existe desde o item 42, mas a IA não o consulta —, e o `speed` do `OBJECT_MOVE` sai como
    `velocidade × 100`, enquanto o original monta `(unsigned short)(run_speed*256.0f+0.5f)`
    (`gs/petnpc.cpp:350`).

    **(b10) Regeneração.** Os valores certos agora existem na entidade (`hp_gen`,
    `mp_gen`); falta saber se há laço de regeneração no tick e com que intervalo.

    **(b12) O Eufórbio amontoado.** Causa lida no código: a área de recurso do `npcgen.data`
    tem `ext_x`/`ext_z`, e o leitor os **descarta** (`npcgen.rs:301-302`, `_ext_x`/`_ext_z`)
    e põe toda instância na mesma coordenada. Descarta também o `fHeiOff` de cada gerador,
    a direção e o ângulo. E há dois tetos inventados: `count.min(5)` para recurso e
    `count.min(10)` para monstro.

    **(b15) O guia e a barra de atalhos.** A barra é configuração de interface que o
    servidor guarda por personagem (o `config_data` do `GRoleBase` e o par
    `GetUIConfig`/`SetUIConfig`). No personagem novo, ela vem de algum lugar — o molde de
    (b1) é o candidato natural. O guia pode ser missão de nascimento, e aí depende de (d).

    ### c. A VM 1.2.6 serve para alguma coisa?

    **Serve, e é o melhor instrumento que o projeto tem hoje.** A prova está nesta mesma
    sessão: a dúvida da velocidade, que já tinha consumido duas sessões, foi resolvida em
    minutos lendo uma captura que aquela VM gerou em 2026-09-01.

    O que ela é, exatamente: um servidor **completo e funcional**, com todos os daemons e o
    banco, em que o comportamento pode ser observado de dentro. Isso responde o tipo de
    pergunta que nem o fonte nem o IR respondem sozinhos — *o que um servidor de verdade
    manda neste momento?*

    Quatro usos, do mais barato ao mais caro:

    1. **Capturas como gabarito campo a campo.** O elo `gs → glinkd` (porta 29301) passa em
       claro, antes da cifra. O `pw-pcapdiff` ganhou nesta sessão o modo `--subcomando N`,
       que abre o envelope e despeja o corpo de um comando do mundo 3D:

       ```bash
       cargo run -p pw-pcapdiff -- _sync/capturas/full_interno.pcap --interno --subcomando 50 --limite 2
       ```

       Com isso, cada ponto de (a) vira um roteiro de captura: morrer e reviver na cidade
       (9), matar um monstro e ver `RECEIVE_EXP` (13), conjurar duas vezes seguidas e ver o
       que o servidor responde à segunda (8), ficar parado e ver a regeneração (10).
    2. **Os moldes de classe, já importados.** O `gamedbd` da VM tem os roleids 16 a 31 com
       nascimento, itens, equipamento e — provavelmente — a barra de atalhos (b1, b3, b15,
       b16). Ler o banco dela resolve de uma vez o que o `clsconfig` binário não deixou.
    3. **Os arquivos de configuração dela** (`gs.conf`, `ptemplate.conf`, `gamesys.conf`,
       `tasks.data` e `npcgen.data` do 1.2.6), para comparar versão com versão.
    4. **A fonte do protocolo 1.2.6**, que já foi o uso original (item 54 e o
       `docs/MEDIDAS_DO_126.md`).

    **O limite, que precisa ficar escrito:** ela é 1.2.6. **32 comandos** têm layout
    diferente do 1.5.x (medido no `MEDIDAS_DO_126.md`), e o próprio `OWN_EXT_PROP` desta
    sessão tem 152 bytes lá contra 196 no binário 1.5.5. As raças e mapas novos não existem
    nela. Ela é gabarito de **mecânica e de fluxo** — o que acontece, em que ordem, com que
    valores —, não de bytes do 1.5.5.

    **O que falta para usá-la:** acesso. A sessão desta máquina alcança a VM (o `ping`
    responde), mas o SSH pede senha e não há chave instalada — e `_sync/credenciais.txt`
    não existe. Instalar a chave pública de `_sync/ssh/win_key.pub` no
    `/root/.ssh/authorized_keys` da VM resolve.

    **E um passo maior, que vale mais do que a VM:** o pacote do servidor **1.5.5
    original** está em disco (`F:\PW\1.5.5\home155` e `pwserver_155v156`, com `gs`,
    `gamedbd`, `glinkd`, `gdeliveryd` e as configurações). Subir aquele servidor numa VM
    32-bit daria o gabarito **da versão certa** — os mesmos bytes que o cliente espera, o
    `clsconfig` já lido pelo `gamedbd` dele, as missões rodando. É o instrumento que
    fecharia de vez a classe de problema "o fonte diz uma coisa, o binário faz outra".

    ### d. Os três arquivos de dados: onde cada um está de verdade

    | arquivo | estado medido | o que falta |
    | :--- | :--- | :--- |
    | `elements.data` | ~~99 de 231 tabelas no v156 do 155BR (item 41f)~~ → **resolvido no item 46**: 231/231, fecha no último byte | — |
    | `tasks.data` | ~~nenhuma missão lida~~ → **resolvido no item 45**: 14.885/14.885 missões de topo fecham byte a byte | — |
    | `npcgen.data` | ~~recurso sem dispersão e com teto inventado (b12)~~ → **resolvido** (commit `931b39d`): tipo de área, `fOffsetTrn`, extensão do recurso, sem tetos | — |

    O `tasks.data` é o maior buraco dos três: sem ele não há missão inicial, e as missões são
    o que dá experiência, alma e moedas a um personagem novo. O formato tem autoridade
    completa no fonte do cliente — `ATaskTempl::LoadBinary` (`Task/TaskTempl.cpp:4723`), que
    lê a parte fixa, a descrição, o tributo, cinco diálogos e as submissões, recursivamente
    — e tem uma propriedade que torna o leitor **verificável**: o cabeçalho traz o
    deslocamento de cada missão de topo, então cada missão lida tem de terminar exatamente
    onde a próxima começa.

    ### e. A ordem

    1. ~~**`npcgen.data`**~~ — feito (`931b39d`).
    2. ~~**`tasks.data`**~~ — feito (item 45).
    3. ~~**`elements.data` v156**~~ — feito (item 46), sem âncoras.
    4. **Acesso à VM 1.2.6**, e com ela os roteiros de captura de (c1) para 6, 7, 8, 9, 10
       e 13, e a leitura dos moldes para 1, 3, 5, 15 e 16.
    5. Com os dados e o gabarito na mão: combate, experiência, alma, moedas, recarga,
       regeneração, reviver, missões iniciais.

45. **Sessão 2026-09-12 (continuação): o `tasks.data` lido inteiro — e por que o fonte não
    bastava.**

    ### a. O resultado

    `crates/pw-data-loader/src/tasks.rs` deixou de ser um esboço (`parse_tasks` devolvia
    `Ok(())`). O leitor lê cada missão de topo e as submissões, e **recusa o arquivo** se
    qualquer missão de topo não terminar exatamente no deslocamento que a tabela do
    cabeçalho dá para a seguinte (`TasksError::Desalinhado`). Medido:

    | realm | versão | missões de topo | com submissões |
    | :--- | ---: | ---: | ---: |
    | `realm_155BR` | 129 | **14.885 / 14.885** | 31.837 |
    | `realm_155` | 129 | **14.978 / 14.978** | 31.979 |

    Os valores conferem com o jogo: a missão 1173 "Exposição de Talento" (Guerreiro,
    nível 1-20, NPC 3517, prêmio 75 exp / 20 alma / 90 moedas) tem as submissões 1175
    "Matar Insetos de Jade" — **10 × monstro 16**, o mesmo Inseto Esmeralda do teste de
    combate — e 1176, cada uma pagando 5 × item 8617. O realm em inglês tem os mesmos
    números com o texto em inglês ("Emerald Qingfu"). Testes: `tests/tasks_do_realm.rs` e
    três unitários em `tasks.rs`.

    ### b. Por que o layout do fonte não fechava

    O fonte 1.5.5 que temos (cliente e servidor, idênticos neste ponto) é da versão de
    missão **125** (`_task_templ_cur_version = 125`, `TaskTempl.cpp:5`). Os arquivos dos
    dois realms são **129**. Compilei uma sonda com o MSVC x86 e as macros reais do
    `ElementClient.vcxproj` (`WIN32;_ELEMENTCLIENT;_USE_32BIT_TIME_T;VIP;...`) para medir
    `ATaskTemplFixedData` sob `#pragma pack(1)`: 1.087 bytes; `AWARD_DATA`: 269. Com eles,
    zero missões fechavam.

    O que faltava foi achado **nos dados**, não deduzido: um histograma de bytes não-nulos
    por deslocamento sobre as 14.885 missões. Os `m_bShowBy*` nascem `true` no construtor,
    e os ponteiros que o editor gravou junto com o `fwrite(this)` têm cara de ponteiro de
    heap — os dois denunciam onde cada membro de fato está. O deslocamento apareceu em três
    saltos (+3, +47, +20) e mais 21 bytes no prêmio. Os nomes vieram do fonte **1.7.2**
    (`F:\PW\1.7.2\172Source\cgame\gs\task\TaskTempl.h`, versão 187), que tem os membros
    novos na mesma ordem — é o sistema de **Lar** (casa do jogador):

    | onde | bytes | membros (nomes do 1.7.2) |
    | :--- | ---: | :--- |
    | depois de `m_bTowerTask` | 3 | `m_bHomeTask`, `m_bDeliverInHostHome`, `m_bFinishInHostHome` |
    | depois de `m_bShowByVIPLevel` | 47 | `m_bPremNoHome`, faixas de nível/recurso/fábrica/prosperidade do Lar e seus `ShowBy` |
    | depois de `m_ulTMIconStateID` | 20 | `m_ulTMHomeLevelType`, `m_ulTMReachHomeLevel`, `m_ulTMReachHomeFlourish`, `m_ulHomeItemsWanted`, `m_HomeItemsWanted` |
    | fim do `AWARD_DATA` | 21 | `m_iHomeResource[5]`, `m_bCreateHome` |

    Mais um vetor variável: `m_ulHomeItemsWanted × HOME_ITEM_WANTED` (8 bytes), depois dos
    `m_pLeaveSite`. Total: bloco fixo **1.157**, prêmio **290**.

    **A lição vale para o `elements.data` também:** o fonte é o ponto de partida, a
    tabela de deslocamentos do próprio arquivo é o juiz. O `elements.data` v156 do 155BR
    (item 41f, 132 tabelas vazias) provavelmente tem a mesma história — campos acrescentados
    entre a versão do fonte e a do arquivo.

    ### c. O que o leitor extrai, e o que só atravessa

    Extrai: id, nome e descrição (XOR pelo id, `convert_txt`), missão-mãe e submissões,
    tipo, limite de tempo, faixa de nível, classes, gênero, pré-requisitos, missões
    exclusivas, itens exigidos e entregues, NPC que entrega e que premia, método e tipo de
    conclusão, monstros a matar (com o item que cai), itens a coletar, dinheiro pedido,
    nível/mundo a alcançar, espera, as flags de repetição/desistência/registro/entrega
    automática/escolha de filho, e os dois prêmios (exp, alma, moedas, reputação, exp de
    reino, missão seguinte, teleporte, grupos de itens com sorteio).

    Atravessa sem guardar: diálogos, expressões de variável global, regiões, prêmios por
    escala de tempo/item, requisitos de equipe/título/Lar. Ficam para quando o sistema de
    missões precisar deles.

    Versões sem layout medido (a 55 do 1.2.6, a 124 do 1.5.3) leem só o cabeçalho, com
    aviso — não se adivinha layout.

    ### d. O que isto destrava, e o que ainda não

    Nada no mundo usa as missões ainda. O que o leitor já mostrou para o próximo passo
    ("missões iniciais"): as missões de nível 1 de classe única são "Exposição de Talento"
    (Guerreiro 1173, Mago 1198) e as "<Classe> Eso" (30708-30717, nível 1-1, sem NPC). A
    missão inicial de cada classe no 1.5.5 ainda precisa ser confirmada — pelo
    `task_npc.data`/NPC de entrega no mapa inicial, ou pelo servidor 1.5.5 original.

46. **Sessão 2026-09-12 (continuação 2): o `elements.data` do 155BR lido inteiro — os
    "overrides" eram dois blocos que o leitor não conhecia.**

    ### a. O resultado

    | realm | versão | tabelas | registros | vazias | antes |
    | :--- | ---: | ---: | ---: | ---: | :--- |
    | `realm_155BR` (o do docker) | 156 | **231** | 69.640 | ≤5 | 99 lidas, 132 vazias (item 41f) |
    | `realm_155` | 159 | **234** | 70.067 | — | lia, com 11 overrides |

    Os dois fecham **no último byte do arquivo, sem nenhum override**, e o leitor agora
    recusa arquivo que não feche (`GenericElementsError::NaoTerminaNoFim`). Tabelas que
    voltaram no 155BR: `PLAYER_ACTION_INFO_CONFIG` (1.354, eram 7.736 de lixo),
    `MINE_ESSENCE` (1.557 — o que cada recurso do mapa dá), `FASHION_ESSENCE` (3.016),
    `PET_ESSENCE`, `SUITE_ESSENCE`, `PLAYER_LEVELEXP_CONFIG`, `TITLE_CONFIG` e o resto.

    ### b. A causa

    `elementdataman::load_data` do **cliente**
    (`EvolvedPWClient/ElementClient/CCommon/elementdataman.cpp:3879`) não é só
    `count + registros` em sequência. Tem dois blocos que não são tabela:

    ```cpp
    // depois de armorrune_essence_array (linhas 4009-4016)
    fread(&tag, 4); fread(&len, 4); fread(buffer, len); fread(&t, sizeof(time_t));
    // depois de war_tankcallin_essence_array (linhas 4122-4124)
    fread(&tag, 4); fread(&len, 4); fread(buffer, len);
    ```

    O primeiro é o nome da máquina que exportou o arquivo (o `save_data` do servidor grava
    `tag = 0xab7689dd`, `gs/template/elementdataman.cpp:3608`); o segundo, `0xee35679f`
    (linha 3725). No 155BR: 7 bytes de nome + `time_t` = **os 19 bytes "sem explicação"**
    antes de `SKILLTOME_SUB_TYPE` (item 9 do contexto A, 2026-09-02); no v159, 8 bytes = o
    `skip: 20` do override dele.

    Sem conhecer os blocos, o leitor antigo tentava a posição ingênua, pontuava o primeiro
    registro por "plausibilidade" e buscava numa janela quando não gostava — e dez
    `skip`/`count`/`abs_count_off` por arquivo corrigiam o que a busca errava. As âncoras
    absolutas eram posições **daquele** arquivo; num v156 diferente (o do 155BR, 270 KB
    maior) caíam no meio de outra tabela. Conferido: cada âncora antiga do v159
    (`44280288`, `48139516`, `51277452`, `51351012`, `54231319`) é exatamente onde a leitura
    estrita cai.

    O mesmo padrão do `tasks.data` (item 45): **o carregador do cliente é o juiz**, e o
    tamanho do arquivo fecha a conta. Heurística de plausibilidade só adiava o erro.

    ### c. O que mudou

    * `crates/pw-data-loader/src/generic_elements.rs`: leitura estrita; saíram a pontuação,
      a busca em janela, os `RealmOverrides` e o `include_str!` dos overrides.
      `load_elements_data(buf)` perdeu o parâmetro de overrides;
      `load_elements_data_auto` ficou como sinônimo. `TALK_PROC` ganhou checagem de limites
      (antes fatiava sem conferir).
    * `specs/elements_layouts/pw_elements_reader.py` (o do web-admin): o mesmo algoritmo;
      `overrides_path` é aceito e ignorado. Havia um bug latente aí: o web-admin passava os
      overrides do **v156** para o `realm_155`, cujo arquivo é **v159**.
    * `tests/generic_elements_tests.rs` reescrito: os dois realms, contagens conferidas por
      conteúdo, totais iguais aos do leitor Python, e um arquivo com um byte a mais recusado.
    * Os `specs/elements_155/realm_155*_overrides.json` ficaram no repositório como registro
      histórico; nada os lê.

    ### d. O que isto não resolve

    O `pw-gs` hoje só consome tabelas até o índice 72 (armas, armaduras, decorações,
    monstros, NPCs, classes, poções), que já liam certo. As tabelas novas ficam
    disponíveis para os próximos passos — `PLAYER_LEVELEXP_CONFIG` para a curva de
    experiência, `MINE_ESSENCE` para a colheita — mas nenhuma delas foi ligada ao jogo
    nesta sessão.

47. **Sessão 2026-09-12 (continuação 3): o `clsconfig` do servidor 1.5.5 original está em
    disco e responde onde cada classe nasce — com duas respostas.**

    ### a. O que é e onde está

    O `gamedbd` original não inventa personagem novo: copia um dos moldes 16..31 do
    arquivo `clsconfig` (`cnet/gamedbd/clsconfig.h::ImportClsConfig`, e
    `GameDBManager::GetClsDetail`, `gamedbmanager.cpp:345`), trocando só `cls` e `gender`.
    Cada molde é um `GRoleTableClsconfig` = `GRoleBase` + `GRoleStatus` + inventário +
    equipamento + armazém, gravado sem compressão. **Esse é o gabarito dos pontos 1, 3, 5,
    15 e 16 do item 44** (lugar de nascimento, atributos, itens iniciais, barra de atalhos)
    — e não precisa da VM 1.2.6: está em `F:\PW\1.5.5\home155\gamedbd\clsconfig` e em
    `F:\PW\1.5.5\pwserver_155v156\home\pwserver\gamedbd\clsconfig`.

    Leitor: `specs/clsconfig_155/ler_clsconfig.py` (acha cada molde pelo nome
    `cls<N>gender<M>` e lê `GRoleBase` e o começo de `GRoleStatus`; Marshal big-endian).

    O molde de cada classe não é `16 + cls×2 + gênero`: é a tabela de `GetDataRoleId`
    (`gamedbmanager.cpp:208`) — 0→16, 1→19, 2→20, 3→23, 4→24, 5→27, 6→28, 7→31, 8→18,
    9→17, 10→21, 11→22. Os roles 25, 26, 29 e 30 são restos sem uso (mundo 0, sem
    `config_data`).

    ### b. Onde cada classe nasce, pelos dois arquivos

    | cls | classe | role | `home155` (2023) | `pwserver_155v156` (2018) | nosso `class_templates` (155BR) |
    | ---: | :--- | ---: | :--- | :--- | :--- |
    | 0 | Guerreiro | 16 | mundo 1 (218.0, 218.7, 2838.0) | mundo 161 (-848.3, 40.5, -182.0) | mundo 1 (976, 219.2, 4187.3) |
    | 1 | Mago | 19 | mundo 1 (218.0, 218.6, 2838.3) | mundo 161 (-847.4, 40.5, -182.4) | mundo 1 (976, 219.2, 4187.3) |
    | 2 | Espiritualista | 20 | mundo 161 (-651.1, 41.0, -225.2) | igual | mundo 1 (650, 201.1, 130) |
    | 3 | Feiticeira | 23 | mundo 1 (-1441, 242, 1383) | mundo 161 (-712.8, 35.0, -364.2) | mundo 1 (-1445.6, 219.3, 2642) |
    | 4 | **Bárbaro** | 24 | **mundo 1 (-1441, 242, 1383)** | mundo 161 (-712.9, 35.0, -364.4) | mundo 1 (-1445.6, 219.3, 2642) |
    | 5 | Assassino | 27 | mundo 161 (-651.8, 41.0, -225.5) | igual | mundo 1 (650, 201.1, 130) |
    | 6 | Arqueiro | 28 | mundo 1 (-317, 218, -910) | mundo 161 (-821.7, 44.9, -259.7) | mundo 1 (-741.5, 219.1, -1234.8) |
    | 7 | Sacerdote | 31 | mundo 1 (-317, 218, -910) | mundo 161 (-821.9, 44.9, -260.2) | mundo 1 (-741.5, 219.1, -1234.8) |
    | 8 | Guardião | 18 | mundo 161 (-800.5, 44.9, -314.2) | igual | mundo 1 (380, 219.3, 230) |
    | 9 | Místico | 17 | mundo 161 (-800.7, 44.9, -315.2) | igual | mundo 1 (380, 219.3, 230) |
    | 10 | Ceifador | 21 | mundo 161 (-760.7, 44.9, -218.3) | igual | mundo 1 (150, 210.1, 250) |
    | 11 | Tormentador | 22 | mundo 161 (-760.3, 44.9, -218.3) | igual | mundo 1 (150, 210.1, 250) |

    Conferência independente, pelo `npcgen.data` + `tasks.data` do 155BR: os pontos do
    `home155` no mundo 1 ficam **ao lado do Guia de cada raça** — Guia 3517 em (221, 219,
    2854) para Humanos, Guia 3518 em (-1445, 241, 1398) para Selvagens, Guia "Jace Johnson"
    3519 em (-313, 218, -893) para Elfos — e os Guias são os NPCs das primeiras missões
    ("Exposição de Talento" 1173/1198, "Terra Natal" 9532/9533). O nosso ponto do Bárbaro
    tem o `x` do Guia e um `z` 1.260 m ao norte: é o "Campo da Expedição" do teste do POTATO.

    **O mundo 161** é o `Instance_is61` do `gs.conf` original (`base_path = a61/`, `limit =
    allow-root;anti-cheat;home_entrance;`, `parallelworld_server = 1`), e a pasta `a61`
    existe no `realm_155BR`. No pacote `pwserver_155v156` — **a mesma build v156 do
    `elements.data` que o 155BR usa** — toda classe nasce lá, o que responde à suspeita do
    Murillo no item 44 ("o 1.5.5 começa em outro mapa"). No `home155`, as raças antigas
    foram trazidas de volta às vilas do mundo 1 e só as novas (Espiritualista, Assassino,
    Guardião, Místico, Ceifador, Tormentador) ficam no 161. A missão auto-entregue
    "Guarda da Terra" (31033/31038, classes 8 e 9) teleporta para o mundo 1 em (-242.4,
    239.5, -3201.9) — coerente com o 161 ser o começo e o mundo 1 o destino.

    Vida e mana de nível 1 do molde (ex.: Bárbaro 85/35, Guerreiro 75/45, Mago 50/70)
    também estão lá, para conferir contra o que o nosso servidor calcula.

    ### c. A decisão que faltava — tomada: `pwserver_155v156`, todos no 161 (item 48)

    Qual dos dois `clsconfig` é o do realm 155BR: o `pwserver_155v156` (todos no mapa
    161, a build dos nossos dados) ou o `home155` (raças antigas nas vilas)? Nascer no 161
    também exige que o mundo 161 suba no `pw-world-155br`, o que hoje não acontece.

    ### d. O que ainda dá para tirar do arquivo

    `config_data` (232 a 385 bytes por molde) é a configuração do cliente guardada no
    servidor — muito provavelmente a barra de atalhos pré-preenchida que o Murillo viu no
    1.2.6 (F1 ataque, F2 habilidade, F4 pegar, F5 meditar, F8 portal). Inventário,
    equipamento e habilidades (`GRoleStatus.skills`) também estão no registro. Nada disso
    foi decodificado ainda.

48. **Sessão 2026-09-12 (continuação 4): todo personagem nasce no mapa 161, e os monstros
    andam no chão, na velocidade certa, e passeiam.**

    ### a. O que o Murillo testou e pediu

    Do deploy anterior, **confirmado em jogo**: velocidade 4,9 m/s; Guia não flutua mais;
    recursos espalhados; monstros nascem no chão. Ainda errado: o monstro em fúria persegue
    "sem respeitar o terreno", por baixo da terra e no ar, e rápido demais; e monstro ocioso
    não se move na área. Decisão: **usar o `clsconfig` do `pwserver_155v156`, todos
    nascendo no mapa 161** (item 47c).

    ### b. O movimento dos monstros — as regras do original

    `crates/pw-gs/src/ai.rs` foi reescrito sobre o servidor original:

    | | original | antes | agora |
    | :--- | :--- | :--- | :--- |
    | passo da perseguição | `session_npc_follow_target`: a cada 0,5 s (`NPC_FOLLOW_TARGET_TIME`), `run_speed × 0,5` m | a cada tique de 50 ms, aviso a cada 2 m | igual ao original |
    | altura do passo | o *agent* do habitat devolve a posição no chão (`pathfinding.cpp:74`) | `y` fixo no do nascimento | chão do `.hmap` para monstro de chão; água/ar seguem o alvo sem descer abaixo do terreno |
    | `OBJECT_MOVE.use_time` | **milissegundos** (`npcsession.cpp:258`) | centésimos de segundo | ms |
    | `OBJECT_MOVE.speed` | **× 256** | × 100 | × 256 |
    | `move_mode` | `RUN`/`WALK` + bit do habitat (`GetMoveModeByInhabitType`) | 0 | igual ao original |
    | parada | `OBJECT_STOP_MOVE` com a direção `a3dvector_to_dir` | nunca | ao chegar ao alcance, ao chegar em casa, ao fim do passeio |
    | sem alvo | `ai_returnhome_task`: volta correndo ao nascimento, passo de 1 s | ficava onde estava | igual ao original |
    | passeio ocioso | `ai_policy::HaveRest` + `ai_rest_task` + `session_npc_cruise` | não existia | ver abaixo |

    **O "rápido demais" era a unidade**: um trecho de 2 m chegava ao cliente como "faça em
    50 ms" (5 centésimos lidos como milissegundos) — 40 m/s na tela.

    **O passeio**, pelas regras de `gs/aipolicy.cpp:237`, `gs/ainpc.cpp:303` e
    `gs/npcsession.cpp:590`: só com jogador por perto (`idle_timer`, renovado por 20
    batimentos de 1 s); monstro com `patroll_mode` no `MONSTER_ESSENCE`, sem ódio e sem outra
    tarefa; o `cruise_timer` de 32 casas dá uma volta e o monstro sai **andando**
    (`walk_speed`, um passo por segundo, aviso de 1000 ms) para um ponto sorteado a até
    **10 m do nascimento**, com no máximo 8 passos; ao chegar, 10% de chance de emendar
    outro passeio.

    **O que não é igual:** o original anda num mapa de movimento que desvia de obstáculo
    (`GetMoveMap`). Aqui é linha reta assentada no chão — casa e pedra ainda são
    atravessadas.

    Testes novos em `crates/pw-gs/tests/achados_do_teste_em_jogo.rs`: persegue assentado
    numa rampa, unidades do `OBJECT_MOVE` (500 ms, 1024 para 4 m/s, modo correr), passeia
    só com jogador perto e dentro de 10 m no chão, volta para casa sem alvo.

    ### c. Nascer no mapa 161

    1. **Moldes** — `scripts/2026_09_12_nascimento_no_mapa_161_155br.sql`, aplicado no banco:
       as 12 linhas do `realm_155BR` com `spawn_world_id = 161` e a posição exata do molde
       de cada classe (`GetDataRoleId`). Cada ponto fica 1 cm acima do chão do `a61` e a
       poucos metros do Guia da raça no `a61/npcgen.data` (Guia Selvagem a 4 m do
       Bárbaro). Personagens já criados continuam onde estão.
    2. **Catálogo de terreno** — `specs/mapas/terreno_155.json` estava com o **`index`** do
       `gs.conf` no lugar do **`tag`**: só o mundo 1 saía certo, e o "mundo 31" do catálogo
       era o `a01`. Agora é gerado por `specs/mapas/gerar_terreno_155.py`: 79 mapas, pelo
       tag, incluindo o 161 (`a61`, 4×3 blocos).
    3. **Pastas de mapa** — `GameDataManager` lia só `a01..a33`; agora `a01..a99` (`aNN` =
       mundo `100 + NN`, conferido contra o `gs.conf`). O `a61` tem 1.665 entidades.
    4. **Um servidor de mundo por mapa**, como o original (um `gs` por seção do `gs.conf`):
       serviço `pw-world-155br-161` no compose (`WORLD_TAG: "161"`), e o `pw-link` com
       `GS_BUS=1=pw-world-155br:29100,161=pw-world-155br-161:29100`. O link manda cada
       sessão ao servidor do mundo do personagem (`LinkGateway::uplink_da_sessao`); mundo
       sem entrada cai no primeiro. Os testes de topologia do compose passaram a aceitar a
       lista e cobram que `161=` aponte para quem tem `WORLD_TAG` 161.
    5. **`INST_DATA_CHECKOUT`** — o `id_inst` era `1` fixo; agora é o mundo do personagem,
       com os carimbos de `region.sev`/`precinct.sev` daquele mapa. É o mesmo `worldtag` com
       que o cliente abre o mapa (`StartGame(ri.worldtag, …)`, `EC_LoginUIMan.cpp:1048`).

    6. **Monstro ou NPC** — ao subir o 161, o log disse "5 monstros, 1.472 NPCs". O
       `npcgen.rs` decidia pelo número (`tid >= 10000` é NPC), e no 1.5.5 quase todo
       monstro novo tem id acima disso (Coelho 12407, Cervo Brilhante 46084). O original
       decide pelo tipo do registro no `elements.data` (`DT_MONSTER_ESSENCE` /
       `DT_NPC_ESSENCE`, `gs/npcgenerator.cpp:79` e `:415`); agora o mundo também
       (`GameDataManager::ids_de_npc`), e o chute só vale para id que nenhuma tabela
       conhece. Resultado medido no deploy: mapa 161 com **1.269 monstros e 208 NPCs**;
       mundo 1 com 29.620 monstros e 1.380 NPCs, e o aviso de "910 monstros sem template"
       do mundo 1 sumiu — eram NPCs de id baixo tratados como monstro.

    ### d. O que ainda falta, sabido

    * **Trocar de mundo** (o teleporte da missão "Guarda da Terra" para o mundo 1, portal,
      GM) não existe: o link escolhe o servidor na entrada, e nada muda o mundo da sessão
      depois.
    * **Reviver na cidade** usa `CharacterClass::default_spawn_position`, que é do mundo 1 —
      num personagem do 161 isso é errado (e o reviver já era um problema do item 44).
    * A lista de "jogadores visíveis" do link é por link, não por mundo: jogadores do 1 e
      do 161 aparecem uns para os outros.

49. **Sessão 2026-09-14: três subcomandos respondidos duas vezes, a marca das missões
    dinâmicas respondida com a ordem de esquecer habilidade, e o banco de testes limpo.**

    ### a. O pedido

    Três pendências apontadas na reescrita do estado: subcomandos tratados no `gateway.rs`
    **e** no `pw-gs`; a suíte deixando realms `t_*` no banco (mais de 3.000); e o
    `docs/PROMPT_PROXIMA_SESSAO.md` desatualizado.

    ### b. As respostas duplicadas — e o que cada uma mandava

    O `gateway.rs` repassa todo `GamedataSend` ao mundo e **depois** executa o próprio
    `match`. O teste `os_comandos_ja_migrados_nao_sobraram_no_gateway` existia para impedir
    isso, mas a lista dele era escrita à mão e parou em 27 comandos. Reescrito para ler os
    braços do `BusServer::tratar_subcomando` direto do fonte do mundo, ele falhou na hora:
    `["35 (SEVNPC_HELLO)", "49 (TASK_NOTIFY)", "85 (SWITCH_FASHION_MODE)"]`.

    | id | o link mandava | o mundo manda |
    | ---: | :--- | :--- |
    | 35 | `NPC_GREETING` para **qualquer** id | `NPC_GREETING` só se o alvo é NPC deste mundo |
    | 85 | `PLAYER_ENABLE_FASHION` com o "estado" lido do byte 2 — o comando **não tem corpo**, então sempre "ligado" | alterna o estado guardado no mundo e avisa quem está perto |
    | 49 | ver (c) | ver (c) |

    De passagem: o braço `23..=26` do link dizia tratar "decolar/pousar", mas 23 a 26 são
    `GET_EXT_PROP_BASE`/`_MOVE`/`_ATK`/`_DEF` (IR), consultas sem corpo. Ele lia um tipo de
    voo no byte 2, que não existe, e nunca fazia nada. Removido junto.

    ### c. O 49: `reason` 7 é "esquecer habilidade"

    O cliente manda `TASK_NOTIFY` com `reason = 7` ao entrar no mundo
    (`TASK_CLT_NOTIFY_DYN_TIMEMARK`, `TaskProcess.cpp:2185`). O link respondia um
    `TASK_VAR_DATA` com `reason = 7`, marca 0 e "versão" 1 em 4 bytes. Mas os `reason` do
    servidor são outra tabela (`cgame/gs/task/TaskTempl.h:81-95`): **7 é
    `TASK_SVR_NOTIFY_FORGET_SKILL`**, e o cliente o trata com
    `GetTaskTemplMan()->OnForgetLivingSkill(pTask)` (`TaskClient.cpp:283-287`). A marca é o
    **8**. Para qualquer outro `reason`, o link reenviava `TASK_NOTIFY_NEW` das missões ativas
    e um `TASK_DATA` inteiro — que no cliente dispara de novo o `LoadConfigData`.

    E na própria entrada do mundo o link mandava, sem ninguém pedir, uma marca com `reason`
    8 e **versão 0** — que o cliente descarta, porque só aceita `DYN_TASK_CUR_VERSION` = 10
    (`TaskTemplMan.cpp:168`).

    O original (`ATaskTemplMan::OnTaskGetDynTasksTimeMark`, `TaskTemplMan.cpp:299-309`):

    ```cpp
    if (m_ulDynTasksTimeMark == 0) return;
    data.reason = TASK_SVR_NOTIFY_DYN_TIME_MARK;  data.task = 0;
    data.time_mark = m_ulDynTasksTimeMark;         data.version = DYN_TASK_CUR_VERSION;
    pTask->NotifyClient(&data, sizeof(svr_task_dyn_time_mark));   // 9 bytes, pack(1)
    ```

    O cliente aceita só esses 9 bytes (`TaskClient.cpp:290-296`); se a marca for igual à do
    `dyn_tasks.data` dele, carrega as missões dinâmicas do arquivo local, marca-se
    verificado e **só então** monta a lista de missões ativas; senão pede o pacote inteiro
    (`TaskTemplMan.cpp:166-178`).

    A marca vem do `DYN_TASK_PACK_HEADER` (`TaskTemplMan.cpp:45-51`). Medido no arquivo dos
    dois realms 1.5.5 (idênticos): `pack_size` = 12.979 = tamanho do arquivo, `time_mark` =
    `0x52776c0d` (2013-11-04), `version` = 10, 28 missões.

    ### d. Correção

    - `gateway.rs`: saem os braços 23–26, 35, 49 e 85, e a marca não pedida da entrada.
    - `pw-data-loader/src/dyn_tasks.rs` (novo): lê o cabeçalho com as duas recusas do
      `UnmarshalDynTasks` (versão diferente de 10, `pack_size` diferente do tamanho).
      `GameDataManager::marca_das_missoes_dinamicas` (`None` sem arquivo ou marca zero).
    - `S2CGamedataSend::task_dyn_time_mark` (novo): os 9 bytes do `svr_task_dyn_time_mark`
      no `TASK_VAR_DATA`.
    - `BusServer::notificar_tarefa`: responde o `reason` 7 com a marca, e não responde sem
      marca, como o original. Os outros `reason` continuam só registrados.

    ### e. O banco de testes

    Os testes de `pw-storage` (`atributos_ao_criar`, `autorizacao_de_personagem`,
    `itens_sobrevivem`) e `pw-gs` (`subcomandos_no_mundo`) criam realm, conta e personagem
    reais e nunca apagavam: **3.129 realms `t_*`, 3.129 contas, 1.327 personagens e 18.774
    moldes de classe**. Conferido antes de apagar: nenhuma conta de teste com personagem fora
    de `t_*`, nenhum personagem de conta real em `t_*`, nenhuma facção, correio ou auditoria
    ligada. Contas reais: `admin` e `testuser`.

    - Backup completo: `data/_backups/pw_database_2026-09-14_antes_da_limpeza_de_testes.sql`.
    - `crates/pw-storage/tests/sql/limpar_sobras_de_teste.sql`: apaga contas pelos nomes que os
      testes geram (cascata para personagens, itens, habilidades, missões) e depois os realms
      `t_*` vazios (cascata para moldes), **só com mais de 15 minutos** — dois `cargo test`
      simultâneos não se atropelam.
    - `crates/pw-storage/tests/comum/mod.rs`: roda o SQL uma vez por processo; incluído pelos
      quatro arquivos (no `pw-gs` por `#[path]`).
    - Aplicado à mão: 3.078 realms apagados na primeira passada (os outros eram da rodada de
      minutos antes).

    Não se apaga no fim de cada teste porque o teste que falha não chega ao fim — e é o que
    mais deixaria sobra.

    ### f. O prompt

    `docs/PROMPT_PROXIMA_SESSAO.md` removido (`git rm`): pedia a fila do B40, já cumprida no
    B41, e o papel dele passou ao agente `pw-server-dev` e às skills em `.claude/`.

    ### g. Provas

    - `os_comandos_ja_migrados_nao_sobraram_no_gateway`: falhou com os três ids antes da
      correção, passa depois.
    - `dyn_tasks.rs`: 2 unitários (ordem dos campos, as três recusas).
    - `tests/dyn_tasks_do_realm.rs`: os dois realms fecham com os números acima; o
      `GameDataManager` guarda a marca; um byte a mais é recusado.
    - `protocol_tests::a_marca_das_missoes_dinamicas_vai_com_reason_8_e_nove_bytes`.
    - `subcomandos_no_mundo::o_pedido_da_marca_das_missoes_dinamicas_recebe_a_marca_do_realm`:
      caminho inteiro pelo barramento.
    - Limpeza: 121 linhas de teste envelhecidas uma hora; rodando só `atributos_ao_criar`,
      sobraram as 3 daquela execução.
    - Suíte com banco: **512 passando, 2 falhando** (as do 1.2.6 no `loader_tests`).

    ### h. O que continua faltando

    - Ver em jogo (estado §3.4): diálogo de NPC abrindo uma vez, roupa alternando, e o
      cliente aceitando a marca.
    - As outras notificações de missão do cliente (prêmio especial 9, depósito 12, concluir,
      desistir, chegar ao local…) seguem sem resposta; o pacote de missões dinâmicas (pedido
      8) não é enviado.
    - A "missão inicial" que o link grava na entrada vem de uma tabela escrita no código.

50. **Sessão 2026-09-14 (continuação): o laço de jogo — missões do `tasks.data`, experiência e
    nível, regeneração, recarga, renascer, drop, loja e treinador cobrando.**

    ### a. O pedido e o teste que o motivou

    Teste do B48: Arqueiro novo nasce no 161 (confirmado), perseguição dos monstros certa
    (confirmado), **não dá para aceitar missão** — o Guia dos Alados abre o diálogo, o log do
    mundo registra `aceitou a missão 32201`, e nada aparece — e **o Arqueiro nasce sem
    flechas**. Pedido: commitar (72aba65, c6dabb8), consolidar os mapas num `pw-gs` por realm
    (c07a2de), e implementar o laço de progressão, as missões (pegar e completar) e a economia
    (drop de itens e moedas, comprar habilidades e itens).

    ### b. Por que a missão não aparecia

    O cliente **não recebe** estado de missão depois do login: recebe as listas no `TASK_DATA`
    e refaz sozinho cada operação a partir dos avisos (`ATaskTempl::OnServerNotify`,
    `TaskProcess.cpp:2643-2815` — `DeliverTask`, `RecursiveAward` rodam do lado dele). O
    `TASK_DATA` ia com os cinco blocos vazios; o cliente zera o buffer
    (`CECTaskInterface::Init`, `EC_TaskInterface.cpp:140-160`), a lista ativa fica com
    `m_Version = 0`, e `OnServerNotify` (`TaskClient.cpp:262`) sai na primeira linha quando a
    versão não é `TASK_ENTRY_DATA_CUR_VER` (1, `TaskProcess.cpp:21`). Todo aviso de missão era
    descartado. Os avisos ainda tinham tamanho errado: `task_notify_complete` sem o byte de
    estado útil, `monster_killed` com 9 bytes onde o cliente exige 17.

    Consequência de desenho: o servidor tem de manter **as mesmas estruturas binárias** que
    o cliente (`ActiveTaskList` de 8 + 32×n, `FinishedTaskList` ordenada, as listas de tempo e
    contagem, `StorageTaskList` de 864 bytes — `TaskProcess.h:103-392`) e mexê-las com as
    mesmas funções, índice por índice. `crates/pw-gs/src/missoes.rs` porta `DeliverTask`,
    `RealignTask`, `RecursiveClearTask`, `CheckPrerequisite` (na ordem do original, com os
    sistemas ausentes **recusando**), `CheckDeliverTask`, `CheckKillMonster`,
    `OnTaskCheckAward`/`OnTaskCheckAwardDirect`, `DeliverAward`, `RecursiveCalcAward`,
    `RecursiveAward`, `DeliverByAwardData` (com `_lev_co`), `GiveUpOneTask`. As listas vão para
    `character_task_lists` (cinco `BYTEA`, como o `GRoleTask` do gamedbd) e saem do banco no
    `TASK_DATA` do link e da memória no do mundo. A "missão inicial" inventada do link
    (9374/1/9375) saiu.

    Quem pode aceitar: o NPC em conversa (`SEVNPC_HELLO`) com a missão no
    `NPC_TASK_OUT_SERVICE` (`task_out_provider::TryServe`, `serviceprovider.cpp:1088-1116`);
    entregar, no `NPC_TASK_IN_SERVICE`. Os campos que o motor precisa foram acrescentados ao
    leitor com os deslocamentos da sonda do B45 (`specs/tasks_155/layout129.tsv`), mais
    `m_uDepth` (`CheckDepth`).

    ### c. Progressão (`progressao.rs`)

    | regra | fonte |
    | :--- | :--- |
    | parte de cada um: `exp × dano / max(total, max_hp)`, ajuste por diferença de nível, `+0,5` | `DispatchExp` `npc.cpp:1515`, `ReceiveExp` `player.cpp:2813` |
    | dono do abate: maior dano, primeiro golpe vale `max_hp/4` a mais | `npc.cpp:1533-1555` |
    | subida: `exp -= GetLvlupExp`, +5 pontos, atributos refeitos, vida e mana cheias, teto 105 | `LevelUp` `player.cpp:2627`, `ptemplate.conf` |
    | tabelas: `PLAYER_LEVELEXP_CONFIG` 202, `PARAM_ADJUST_CONFIG`, `PLAYER_SECONDLEVEL_CONFIG` | `playertemplate.cpp:311-419` |
    | regeneração 1 s: `hp_gen` em combate, ×4 fora, oitavos acumulados | `player.cpp:9130`, `actobject.h:2143` |
    | combate: atacar 15 s, apanhar ≥ 5 s | `player.cpp:3062,9514`, `config.h:31-32` |
    | renascer: ponto de cidade do distrito do `precinct.sev`, 10 %, perda `GetLvlupExp × exp_lost[cultivo]` | `playercmd.cpp:112`, `player.cpp:8716`, `el_precinct.cpp` |

    Achado de passagem: **nenhum monstro renascia** — a morte gravava `respawn_timer_ms = 0` e
    o tick só conta de um valor positivo. Agora o tempo é o do gerador, e o corpo some em 20 s
    (`_corpse_delay`, `npc.cpp:803`).

    ### d. Habilidades

    `specs/habilidades_155/extrair_habilidades.py` lê os 3.316 stubs `cskill/skills/skillNNN.h`
    do servidor. Recarga armada em `id + 1024` com os segundos truncados
    (`playerwrapper.cpp:170`, `skill.h:577`), conferida antes de conjurar
    (`skillwrapper.cpp:261`); conjuração = `State1::GetTime`. Aprender segue `LearnCondition`
    e `Learn` (`skill.cpp:14-93`) e cobra dinheiro (`SPEND_MONEY`) e SP (`COST_SKILL_POINT`).

    ### e. Economia (`economia.rs`, `bus_server/jogo.rs`)

    - Drop por `DropItemFromData`/`generate_item_from_monster` (itens: `drop_times`,
      `probability_drop_num0..3`, `drop_matters[32]`; moedas: chance 0,7), montes a ±2 m no
      chão, posse 30 s, vida 300 s, id `0xC8…` (com sinal, como os ids do `npcgen`).
    - Pegar (C2S 6 e 184): `PICKUP_MONEY`/`PICKUP_ITEM`, `MATTER_PICKUP` difundido.
    - `Bolsa` empilha como o `CECInventory::MergeItem` — o cliente confere slot e quantidade.
    - Comprar responde `PURCHASE_ITEM` (72), que não era mandado; vender paga o `price` do
      arquivo com a durabilidade (`ItemToMoney`) e responde `ITEM_TO_MONEY` (73) — eram 50
      fixos. O dinheiro passou a viver na entidade: a loja debitava no banco e o autosave
      gravava a entidade antiga por cima.
    - Bolsa de missão: `container_type` 5 no banco, pacote 2 no fio, enviada na entrada.

    ### f. Flechas

    O `clsconfig` original dá ao Arqueiro só o Arco de Madeira (conferido: nenhum
    `PROJECTILE_ESSENCE` no arquivo), e nenhuma missão entrega a Flecha de Iniciante. A pedido
    do Murillo, o molde ganha 8543 × 1000 no slot 11 — decisão do projeto, documentada em
    `scripts/2026_09_14_flechas_do_arqueiro_155br.sql`. O 2271 do `ClassTemplateRepository` não
    existe no v156.

    ### g. Provas

    - `missoes.rs`: aceitar/entregar com o tamanho do aviso, abate que completa, filhos em
      ordem entregando na mesma posição, listas indo e voltando pelos bytes, listas vazias
      iguais às do link.
    - `tests/missoes_do_realm.rs` (tasks.data real): Arqueiro nível 1 aceita e entrega 32201
      (25 exp, 10 SP, 8 moedas); Bárbaro recusado com `NOT_IN_OCCU` e aviso de 13 bytes; **mais
      de 1.000 missões de topo** entregam com índices válidos e sobrevivem à gravação.
    - `subcomandos_no_mundo`: aceitar e entregar pelo barramento com NPC e serviço;
      experiência na morte; moedas no chão (posse e coleta); compra com `PURCHASE_ITEM`;
      venda com o preço do arquivo; treinador cobrando SP e moedas; renascer a 10 %.
    - `economia.rs`, `progressao.rs`, `habilidades.rs`, `progressao_do_realm.rs`: empilhamento,
      sorteio, regeneração, recarga truncada, curva e ajuste do realm.
    - Suíte com banco: **536 passando, 2 falhando** (as do 1.2.6).

    ### h. O que continua faltando

    Ver em jogo (estado §3.4). Distribuir pontos de atributo; do motor de missões: horário,
    região, equipe, facção, PQ, prêmio por escala, teleporte, item de missão pelo NPC; flechas
    gastas no ataque; lista de venda do NPC; troca de mundo; colher recurso.

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

51. **Sessão 2026-09-16: flecha, atributos, barras de atalho, pontos, missões com horário/região/
    equipe/lugar, troca de mapa, coleta, e o 1.2.6 sem falhas de leitura.**

    ### a. O teste em jogo e o pedido

    Com o Arqueiro eaa: missões aceitas e entregues (32201, 31676–31678), subida de nível, e
    ao relogar voltam posição, experiência, nível, moedas e missões — **confirmado**. Defeitos:
    a flecha mostrava "arma de nível 0-0" com o arco vermelho; o Arqueiro tinha 20 de INT; as
    barras de atalho não ficavam gravadas. Pedido: corrigir, implementar o que faltava (horário,
    equipe, facção, chegar a lugar, distribuir pontos, flechas gastas, troca de mapa, coleta) e
    resolver as 2 falhas do 1.2.6.

    ### b. Flecha

    Duas causas. O servidor não mandava bloco de dados para munição: o cliente lê a faixa de
    nível da arma de `IVTR_ESSENCE_ARROW` (`EC_IvtrArrow.cpp:88-113`), que ficava zerada. E o
    `weapon_level` da arma ia **1 fixo**, quando o original copia `WEAPON_ESSENCE.level`
    (`generate_item_temp.h:329`) — o Arco de Madeira (2250) é **nível 0**. `CanUseProjectile`
    (`EC_HostPlayer.cpp:5009-5016`) compara os dois. A Flecha de Iniciante (8543) pede 1–17; a
    **Flecha de Novato (43283)** aceita 0–17 — e é justamente a que a missão "Partida Para
    Perfect World" (31379) entrega ao Arqueiro. Agora: `FichaDaMunicao` do `PROJECTILE_ESSENCE`
    com o cabeçalho de `generate_projectile` (`:607-626`), o nível real da arma, molde e eaa
    com 43283 (`scripts/2026_09_16_flecha_de_novato_155br.sql`).

    ### c. Atributos

    O `ptemplate.conf` diz Arqueiro 15/20/5/10, e o projeto usava isso. Mas o `gamed` copia a
    ficha do banco (`userlogin.cpp`, `memcpy` em `_base_prop`), e ela nasce do molde do
    `clsconfig`: lendo `GRoleStatus.property` (`extend_prop`, `property.h:35`) dos 12 moldes do
    `pwserver_155v156`, **todas as classes têm 5/5/5/5**, e a vida/mana de nível 1 é
    exatamente `vit_hp × 5`/`eng_mp × 5` do `CHARRACTER_CLASS_CONFIG` (Arqueiro 65/55, Bárbaro
    85/35) — base zero, não o `hp` do `.conf`. `ATRIBUTO_INICIAL = 5`, fórmula sem o `hp` do
    `.conf`; teste compara as 12 classes com os moldes. Personagens 1.5.5 migrados para 5/5/5/5
    com `5 × (nível−1)` pontos (`scripts/2026_09_16_atributos_iniciais_5_155.sql`).

    ### d. Barras de atalho

    O cliente junta barras, layout e opções num bloco comprimido (`SaveConfigsToServer`,
    `EC_GameRun.cpp:2014`) e manda no `SetUIConfig`; na entrada, `GetUIConfig_Re` devolve o
    bloco e `LoadConfigsFromServer` refaz cada atalho. O link respondia e **descartava**, e
    mandava sempre vazio. Agora `character_client_config` (`ui_config`, `help_states`,
    `scripts/2026_09_16_configuracao_do_cliente.sql`), gravado só para o personagem da sessão.

    ### e. Pontos de atributo e `GET_EXT_PROP`

    O log do teste tinha `subcomando 22 ainda não tratado` três vezes: era `SET_STATUS_POINT`.
    Portado `PlayerSetStatusPoint` (`player.cpp:8598`) com `ADD_STATUS_POINT` (51). O cliente
    então pede `GET_EXT_PROP` (21), que no original responde `OWN_EXT_PROP`
    (`PlayerGetProperty`, `:8588`) — o mundo mandava só `SELF_INFO_00` e dinheiro.

    ### f. Flechas gastas

    `DoAttack` (`player.cpp:3063-3070`) e `ATTACK_ONCE` (83) a cada golpe (`:3134`), que é o
    comando pelo qual o cliente desconta a flecha e a durabilidade.

    ### g. Missões

    Leitor: janelas de horário, regiões de entrega, lugar a alcançar/sair, `TEAM_MEM_WANTED`,
    cargo de facção, `m_bTransTo`. Motor: `CheckTimetable`/`judge_time_date` (hora local),
    `CheckInZone`, `CheckFaction` (sem facção no servidor, recusa como o original a quem não
    tem), `CheckTeamTask`/`HasAllTeamMemsWanted` e `OnDeliverTeamMemTask` aos membros deste
    mapa, `OnTaskReachSite`/`LeaveSite` pelos avisos 3 e 10 do cliente, teleporte de prêmio e
    ao receber.

    ### h. Troca de mapa

    `NOTIFY_HOSTPOS` estava com 15 bytes (`pos + u8`); o cliente lê `pos, tag, line`
    (`EC_GPDataType.h:1362`) e troca de mundo quando o `tag` muda (`JumpToInstance`). Com
    mapas no mesmo processo (B50), a troca é o roteador tirar o jogador de um mapa e pôr no
    outro (`RoteadorDeMapas::ligar_trocas`), gravando mapa e posição **antes** de mostrar o
    jogador — o teste de integração pegou a corrida em que o banco ficava com o mapa velho.
    No 161, a saída para o mundo 1 do Elfo Alado é "Retornar à Pan Gu" (32340, nível 20+,
    NPC 44408 ao lado do nascimento); a "Partida Para Perfect World" é do 162.

    ### i. Coleta

    `GATHER_MATERIAL` (54) → conferências da mina (`matter.cpp:265-382`), tempo sorteado,
    `PLAYER_GATHER_START/STOP` (126/127), resultado (`matter.cpp:402-510`,
    `player.cpp:1478-1568`) com `HOST_OBTAIN_ITEM` (99), exp/SP, mina some e renasce.
    `MINE_ESSENCE` montado como o `npc_stubs_manager` (`npcgenerator.cpp:1280-1365`).

    ### j. 1.2.6

    `npcgen.data` v5/v6: o leitor recusava; agora lê as structs antigas de
    `CNPCGenMan::Load` e exige fechar no último byte — os 193 arquivos em disco fecham (o `a46` do 155BR tem 0 bytes e falha como antes, igual ao `a50/precinct.sev`).
    `elements.data` v7: **não tem `time_t`** depois da versão (o leitor pulava 8 bytes e lia a
    contagem da tabela 0 como data). Os 118 tamanhos de registro, escritos à mão e errados a
    partir da tabela 3, foram deduzidos do arquivo por busca com retrocesso (ids distintos,
    nomes legíveis, próxima tabela plausível, fechar no último byte) e conferidos pelo nome do
    primeiro registro contra o enum `DATA_TYPE` (58 = FACE_TEXTURE, 70 = as 8 classes, 76 = a
    curva de exp…); 103–112 fecham mas sem nome para conferir.

    ### k. Provas

    Suíte com banco: **550 passando, 0 falhando**. Novos: flecha de novato serve no
    arco e a de iniciante não; bloco da munição; 12 classes contra os moldes; distribuir
    pontos; tamanhos de `ADD_STATUS_POINT`, `ATTACK_ONCE`, `NOTIFY_HOSTPOS`, `GATHER_*`,
    `HOST_OBTAIN_ITEM`; motor: horário, zona, facção, equipe, lugar, teleporte; minas do realm;
    troca de mapa ponta a ponta (`varios_mapas.rs`: sai do 161, entra no 1, `NOTIFY_HOSTPOS`
    com tag 1, banco com o mapa novo).

    ### l. O que continua faltando

    Ver em jogo (estado §3.4). Teleporte por NPC, troca para mapa de outro contêiner, recarga e
    interrupção por dano na coleta, bônus da flecha, recusa sem munição, sucesso/falha e abate
    compartilhados pela equipe, sistema de facção.

52. **Sessão 2026-09-17: atributos do equipamento, sessão de golpe, alcance/dano/carga das
    habilidades, barras de atalho de verdade, aljava vira munição.**

    ### a. O teste em jogo e o pedido

    Com o eaa: arco e munição, atributos 5/5/5/5 com distribuição gravada, drop, coleta e
    equipar armadura — **confirmado**. Defeitos: o Arqueiro chegava perto para atirar, sem a
    animação do arco, e cada clique dava um golpe; as habilidades também de perto; a mira (234)
    solta antes tirava menos; a 235 tirava mais de 100 no nível 1 (o golpe normal, 3); a barra
    de atalhos continuava sumindo; "fixar missão" não mostrava o rastreador; um monstro dropou
    o item 1955.

    ### b. Equipamento não dava atributo

    Causa comum do alcance, do dano 3 e da cadência: o mundo **nunca aplicava o equipamento**.
    Portado `property_policy` (`playertemplate.h:807-1133`): `UpdateAttack` (dano da arma ×
    bônus de agilidade para arma de longe, alcance da arma + 0,3 do corpo, cadência pelo
    `WEAPON_SUB_TYPE.attack_speed × 20`), `UpdateMagic`, `UpdateDefense` com as armaduras,
    evasão e vida/mana das peças. Refeito ao entrar e a cada troca no equipamento, com
    `SELF_INFO_00` + `OWN_EXT_PROP` — é o `attack_range` deste que diz ao cliente até onde andar.

    ### c. Sessão de golpe

    `session_normal_attack` (`actsession.cpp:350-418`): `NORMAL_ATTACK` confere o alcance
    (`CheckAttack`), manda `HOST_START_ATTACK` (84: alvo, munição, cadência — é o que abre a
    animação no cliente) e golpeia a cada `attack_speed` pelo tick do mundo. Outro clique com a
    sessão aberta só enfileira no original: aqui é ignorado. Alvo morto/longe →
    `HOST_STOPATTACK` (23) com o motivo; `CANCEL_ACTION`, conjurar e morrer encerram.

    ### d. Habilidades

    `extrair_habilidades.py` passou a tirar dos stubs do servidor o alcance
    (`GetPraydistance` = `k × GetRange() + fixo`), o `time_type` e a conta de dano
    (`SetRatio`/`SetPlus` + `SetDamage`/`Set<escola>damage(k × GetAttack|GetMagicattack)`) —
    1.123 habilidades com dano, 14 de carga. Dano = `GeneratePhysicDamage((int)ratio%,
    (int)plus)` (`actobject.h:1422`): bruto × (100 + bônus + ratio%)/100 + plus, depois acerto,
    defesa e crítico. Conjurar fora do alcance (`CheckTarget`, `playerwrapper.cpp:1751`) recebe
    `HOST_STOP_SKILL`. Carga: `CONTINUE_ACTION` (51) conclui na hora com a fração carregada
    (`GetCharging`); a tarefa do fim confere um marcador para não concluir duas vezes.
    **A 235 com +112 no nível 1 é o original** — plus 2,3 + 63,2 + 46,4 e o tooltip do cliente
    (`ElementSkill/skill235.h:226`) mostra 112. Faltava o dano da arma, não sobrava plus.

    ### e. Barras de atalho (a causa real)

    O B51 gravava e lia certo, mas o link mandava o `GetUIConfig_Re` proativamente no
    `EnterWorld`. O cliente só aplica a configuração em `LoadConfigData`, chamado a cada
    `TASK_DATA` (`EC_HostMsg.cpp:3947-3949`, "fim do GET_ALL_DATA"): recebida antes, era
    engolida, e a barra ficava a padrão (e o layout, onde mora o rastreador de missões,
    `DlgTask.cpp:425-455`). Agora o uplink marca o personagem como pronto quando passa o
    `TASK_DATA` (105) do mundo, e o link só responde o `GetUIConfig` depois disso.

    ### f. Item 1955

    É um `QUIVER_ESSENCE` (aljava), e o drop do Espírito da Estrela está certo: o original o
    converte ao gerar (`generate_quiver`, `generate_item_temp.h:650-667`) em
    `id_projectile` × `Rand(num_min, num_max)`. O drop agora converte; o 1955 do eaa virou
    50 × 410 (`scripts/2026_09_17_aljava_vira_municao.sql`).

    ### g. Provas

    Suíte com banco: **555 passando, 0 falhando**. Novos: `arqueiro_do_realm.rs`
    contra o `elements.data` (arco: alcance 20,3, 1,5 s, dano; 235 = bruto × 1,07 + 111; 234
    cheia 272 contra metade 189); sessão de golpe (`HOST_START_ATTACK` de 9 bytes, clique
    repetido não golpeia, `CANCEL_ACTION` → `HOST_STOPATTACK` de 6 bytes, fora do alcance não
    começa); tamanhos contra o IR.

    ### h. O que continua faltando

    Ver em jogo (estado §3.4). Efeitos de estado, área, `arrowcost`, talentos, refino/cravos/
    addons nos atributos, sessão de golpe contra jogador, trava de PvP.

53. **Sessão 2026-09-17: clique repetido, barras e rastreador, Carta da Sorte, efeitos de
    estado, área, flechas por habilidade, equipamento sorteado com addons.**

    ### a. O teste em jogo e o pedido

    Com o eaa, depois do B52: fixar missão não funcionava; clicar várias vezes no inimigo ainda
    dava um golpe por clique; a barra de atalhos montada sumiu ao relogar; a Carta da Sorte
    dropada não fazia nada. Pedido: corrigir e implementar efeitos de estado, habilidades em
    área, flechas gastas por habilidade, e refino/cravos/propriedades adicionais nos atributos.

    ### b. Clique repetido

    O log mostrou o cliente mandando `CANCEL_ACTION` (42) e `NORMAL_ATTACK` (3) a cada clique. O
    B52 encerrava a sessão no cancelamento, e o ataque seguinte golpeava na hora. No original o
    cancelamento limpa a fila e tenta `TerminateSession(false)`, que a sessão de golpe recusa
    (`playercmd.cpp:2136-2153`, `actsession.h:109-115`); o `NORMAL_ATTACK` com sessão aberta
    entra na fila e só começa no próximo golpe, quando `HasNextSession` encerra a atual
    (`actobject.cpp:180-189`). Portado com `SessaoDeAtaque::proximo`.

    ### c. Barras de atalho e rastreador de missões

    Log do realm: `GetUIConfig_Re enviado (211 bytes)`, `GetUIConfig repetido — já respondido`,
    e ao sair `Salvando UIConfig (172 bytes)`. O link mandava um `TASK_DATA` próprio na entrada;
    o cliente pede a configuração a cada `TASK_DATA` (`EC_HostMsg.cpp:3947-3949`), o primeiro
    pedido chegava antes dos dados do mundo, era respondido e aplicado sem habilidades, e o
    segundo era recusado (responder duas vezes derruba o cliente). Com mundo, o link não manda
    mais `TASK_DATA`. O rastreador: decodificado o bloco gravado (versão + zlib; layout
    `USER_LAYOUT` v16), `bTraceAll = 0` no byte 288 e a máscara fixada no 320 — sem
    `m_bShowTrace` o `RefreshTaskTrace` sai sem desenhar (`DlgTask.cpp:476`). O zero veio do
    tempo em que a configuração não chegava (o `CDlgTask` nasce com `false`); o original liga
    para personagem novo (`EC_GameUIMan.cpp:4738-4743`). Script
    `scripts/2026_09_17_rastreador_de_missoes_ligado.py` aplicado ao eaa.

    ### d. Carta da Sorte

    11092 é `TASKDICE_ESSENCE`. `item_taskdice::OnUse` (`item_taskdice.cpp:12-42`): sorteia por
    `task_lists` (`RandSelect`) e entrega por `OnTaskCheckDeliver`; aceitou, gasta. `cartas.rs`
    e `usar_carta_de_missao`.

    ### e. Efeitos de estado

    O extrator passou a ler dos stubs `StateAttack`/`BlessMe` como roteiros `[quem, setter,
    expressão]` (2.304/266), com as expressões normalizadas; o servidor as avalia
    (`efeitos::expr`: aritmética, comparações, `?:`, `INT`). `executar_roteiro` segue o
    `PlayerWrapper`: parâmetros acumulados e o dado que fixa a probabilidade em 100/0
    (`ThrowDice`, `playerwrapper.h:170-178`). 37 filtros de `skillfilter.h` portados com a
    convivência de `filter_man::AddFilter` e os números de `statedef.h`; realces como
    `_en_percent` (`obj_interface.cpp:200-561`) no jogador e no monstro (mesmo
    `property_policy`, classe −1); dano no tempo de `filter_Wounded` (dano/período, tique de 3
    em 3). Batida de 1 s no mundo e os comandos `UPDATE_EXT_STATE` (124, 30 bytes) e
    `ICON_STATE_NOTIFY` (125, variável, parâmetros nos 2 bits altos), mais `ENCHANT_RESULT`
    (139). Monstro atordoado não age, preso não anda, lento anda devagar; selado não conjura.

    ### f. Área, flechas e precisão

    `bus_server/habilidades.rs` segue `SetPerform` (`playerwrapper.cpp:170-420`): `arrowcost`
    por `UseArrow` antes do golpe (`skill.cpp:229`), precisão × `GetHitrate`, dano sorteado uma
    vez, alvos por `range.type` (ponto, cilindro, esfera em si ou no alvo, cone,
    `obj_interface.cpp:1066-1130`), `attached_skill` só em quem foi atingido.

    ### g. Equipamento sorteado, refino e pedras

    O equipamento do original carrega tudo nos octetos: essência sorteada, furos e a lista de
    addons — refino é addon (`refine_*`, valor × fator do nível, `equip_item.cpp:319-330`) e
    pedra é addon com `0x8000` (`equip_item.cpp:876-905`). Portados
    `generate_weapon/armor/decoration` no drop (`geracao.rs`), o `GenerateParam` e o
    `ApplyAtGeneration` dos tratadores (`addons.json` de `item_addon.cpp`, 2.911 ids), um formato
    único do conteúdo (`pw_core::ConteudoDeEquipamento`, que o `item_info` também passou a usar,
    com os testes de bytes intactos) e a leitura ao vestir (`BonusDeAddons`).

    ### h. Provas

    Suíte com banco: **568 passando, 0 falhando**. Novos: avaliador e roteiro (dado herdado),
    convivência e tique dos filtros; tamanhos de 124/125/139; habilidade em área no mundo
    (acerta alvo e vizinho, não o de 30 m, deixa lentos e atordoados, manda 124 e 125); clique
    repetido na fila (sem golpe na hora, `HOST_STOPATTACK` + `HOST_START_ATTACK` no golpe
    seguinte); conteúdo de equipamento escrito e relido; contra o `elements.data` real, 9.000
    sorteios: 5.231 com addon, 3.553 com bônus somado, todos relidos fechando no último byte.

    ### i. O que continua faltando

    Ver em jogo (estado §3.4). Serviços de refinar/furar/incrustar, addons de habilidade e
    conjunto, ~300 efeitos sem porte, imunidades, talentos, trava de PvP.

54. **Sessão 2026-09-17 (continuação): barras e rastreador de verdade, ataque que para,
    atributos com bônus, Asa dos Alados.**

    ### a. O teste e o pedido

    Com o eaa depois do B53: o ataque não parava depois de matar (atacava outro sozinho, Esc e
    andar não paravam); a janela do personagem não somava os atributos do equipamento; as
    barras (F1 e números) e o "fixar missão" continuavam sem funcionar — "resolva
    definitivamente"; o Alado nasceu sem asa.

    ### b. Barras e rastreador — a causa medida

    O log do realm mostrou uma única resposta de 212 bytes e, ao sair, 172 gravados. O log do
    próprio cliente (`element/logs/EC.log`) fechou a questão: `CECGameRun::LoadConfigsFromServer,
    data read error (2)` em **todo** login. `2` é `TYPE_OVERBOUND` (`EC_RTDebug.h:152-157`).
    `S2CGetUIConfigRe::new` trocava os 16 primeiros bytes do bloco por um "cabeçalho"
    (`1, 2097199, 2097199, 1206433535`) escrito numa sessão antiga sem evidência; o cliente lia
    versão 1 (< 3), não descomprimia e estourava o buffer (`EC_GameRun.cpp:2139-2241`). As
    correções dos B51–B53 (gravar, ordem, um pedido) estavam certas, mas o bloco chegava
    corrompido. O "fixar missão" era o mesmo defeito: o layout nunca carregava.

    Para personagem sem configuração, decodificado o `config_data` dos moldes do `clsconfig`
    (`pwserver_155v156`): versão 3 + zlib, host v11 com a barra inicial (Arqueiro: 235, 234,
    167), layout v16 com `bTraceAll = 1`. Guardado em `class_templates.ui_config` e mandado a
    quem não gravou nada — o que o `gamedbd` faz ao copiar o molde.

    ### c. Ataque que não parava

    O log do mundo: `matou -2147482563` a cada 3 s, o mesmo id. O monstro renascia ~1,5 s
    depois da morte, antes do disparo seguinte, e a sessão nunca o via morto; o cliente mandava
    Esc (`CANCEL_ACTION`) repetido e nada parava. No original: o gerador só recebe o monstro
    quando o corpo some (`GM_MSG_OBJ_ZOMBIE_END` → `LifeExhaust` → `Reclaim`, `npc.cpp:904-911`,
    `npcgenerator.cpp:3312`); cancelar e andar põem sessões na fila que encerram o golpe no
    disparo seguinte (`StartSession`, `actobject.cpp:1059-1086`; `cmd_user_move`,
    `playercmd.cpp:9297-9303`). No B53 eu tinha lido o `TerminateSession(false)` e concluído que
    cancelar não fazia nada — faltou a fila. Agora: renascimento só depois do corpo, fila com
    golpe/cancelar/andar, e a morte do alvo encerra a sessão de todos que batiam nele.

    ### d. Atributos da janela

    `DlgCharacter.cpp:442-470` mostra `rep.bs.strength` (verde se algum item soma): é o
    `_cur_prop` do `OWN_EXT_PROP`, base + `_en_point`. Mandávamos a base.
    `PlayerEntity::atributos_efetivos`.

    ### e. Asa

    Lido o `GRoleInventory` do equipamento dos moldes: Arqueiro com arco (pos 0), 200 flechas
    (pos 11) e `2096 pos 12 proc 19`; Sacerdote com cajado e a mesma asa. O molde do realm a
    tinha perdido (o `seed_defaults` a põe, mas o realm foi regravado por script).

    ### f. Provas

    Suíte com banco: **571 passando, 0 falhando**. Novos: `GetUIConfig_Re` devolve o bloco sem
    mexer; Esc e andar param no golpe seguinte, morte do alvo para na hora e o monstro não
    renasce com o corpo no chão; molde com configuração (versão 3 + zlib) para as 12 classes e
    Asa no slot 12 de Arqueiro e Sacerdote.

55. **Sessão 2026-09-17: um realm por versão — o 1.5.5 EN sai, o `realm_155BR` vira `realm_155`.**

    ### a. O pedido

    Murillo: manter só um realm 1.5.5. Havia dois desde 2026-09-03 — `realm_155` (dados do
    cliente EN, elements v159, porta 29003, fora dos testes desde 2026-09-05) e `realm_155BR`
    (cliente BR, v156, porta 29004, o realm de teste). Apagar o EN e renomear o BR para
    `realm_155`. Decisões dele: **manter a porta 29004** (o `serverlist.txt` do cliente BR não
    muda), **apagar** a pasta de dados do EN, **remover** os testes do v159 e **reescrever** os
    scripts SQL para o nome novo.

    ### b. O que mudou

    - **Banco:** `scripts/2026_09_17_realm_155_passa_a_ser_o_br.sql`, numa transação. As chaves
      para `realms(id)` não têm `ON UPDATE CASCADE`, então: apaga o EN (6 moldes, 26 itens e 25
      habilidades de molde, nenhum personagem), cria `realm_155` com a linha do BR (nome
      "Perfect World Evolved (1.5.5)", porta 29004), repassa `characters`, `class_templates`,
      `factions`, `mails` e `admin_audit_logs` e apaga `realm_155BR`. Resultado conferido: 12
      moldes com `spawn_world_id = 161` e `ui_config`, POTATO (4515) e eaa (5491). Backup antes:
      `data/_backups/pw_database_2026-09-17_antes_de_renomear_realm_155.sql`. **Não reaplicar.**
    - **Disco:** `data/realm_155` (EN, 2,1 GB) apagada; `data/realm_155BR` renomeada para
      `data/realm_155`.
    - **Compose:** saíram `pw-realm-155br`/`pw-world-155br` e o par EN; ficou `pw-realm-155`
      (29004) + `pw-world-155` (`WORLD_TAGS: "1,161"`). O Dragonfly não tinha chave do 155.
    - **Scripts:** os `*_155br.sql` viraram `*_155.sql`, com `realm_id = 'realm_155'`; os que
      filtravam `IN ('realm_155', 'realm_155BR')` passaram a `= 'realm_155'`.
    - **Seed** do `specs/01_DATABASE_SCHEMA_POSTGRES.sql`: uma linha 1.5.5, porta 29004.
    - **Código:** só testes e comentários — nenhum identificador de produção citava o realm.

    ### c. Testes que mediam o arquivo do EN

    Com a pasta BR no lugar de `data/realm_155`, os testes que liam o EN passaram a ler o BR:
    - `aipolicy_tests`: o cabeçalho do `aipolicy.data` do BR é `u32 1, u32 3137` (o EN tinha
      3.144), e os triggers estão gravados na **versão 23** (EN: 24). O leitor fecha o arquivo
      inteiro nas duas versões. Números trocados pelos do arquivo.
    - `loader_tests` (pasta 1.5.5 inteira): o pacote BR traz `a46/npcgen.data` e
      `a50/precinct.sev` com **0 bytes** (já anotado no B51). O teste agora cobra exatamente
      essas falhas e nenhuma outra.
    - Saíram `o_v159_do_155_le_as_234_tabelas` e `o_realm_em_ingles_tem_os_mesmos_numeros`; os
      laços `["realm_155BR", "realm_155"]` ficaram com um realm. O layout v159 continua no
      catálogo (`generic_elements.rs`), sem arquivo para testar.

    ### d. Provas

    Suíte com banco: **569 passando, 0 falhando** (eram 571; saíram os 2 testes do v159). Na
    primeira rodada, `subcomandos_no_mundo::aceitar_e_entregar_missao_no_npc_mexe_nas_listas_e_premia`
    falhou uma vez e passou isolado (56/56) e na rodada completa seguinte — concorrência de
    teste, não a renomeação. Contêineres reconstruídos: `pw-realm-155` escutando na 29004
    para `realm_155` (`ELEMENTDATA_VERSION=0x3000009c`, `task_templ=129`), `pw-world-155` com o
    mundo 1 (29.620 monstros, 1.380 NPCs, 5.268 recursos) e o 161 (1.269, 208, 187).

56. **Sessão 2026-09-17: a barra de vida que caía no clique, e os sons de fundo repetindo.**

    ### a. O relato

    Teste do Murillo com o eaa no `realm_155` (depois do B54/B55): a barra de atalhos voltou
    preenchida (B54 confirmado). Defeitos: alguns sons de fundo começaram a tocar várias vezes;
    ao clicar num monstro ele já perdia vida antes de o ataque sair.

    ### b. A vida caindo antes do golpe — causa e correção

    O original também golpeia já no `StartSession` (`actsession.cpp:350-378` chama `DoAttack`),
    e o `DoAttack` do jogador só enfileira `GM_MSG_ATTACK` (`player.cpp:3054-3095`,
    `PostLazyMessage` sem atraso). A diferença é a **barra de vida**: o original não a manda
    junto do golpe. Ela sai ao selecionar (`InsertInfoSubscibe` → `query_info00`,
    `actobject.cpp:1591-1610`) e no heartbeat de 1 s (`obj_manager<gnpc, TICK_PER_SEC>`,
    `worldmanager.h:262`; `DoHeartbeat` → `RefreshSubscibeList`, `actobject.cpp:1294-1353`),
    para a lista de inscritos e só se `_refresh_state` ligou (`npc.cpp:2219-2230`). Nós
    mandávamos `npc_info_00` no mesmo instante do golpe, da habilidade de alvo único, da
    habilidade em área e do dano no tempo (este a todos em volta). O cliente atualizava a barra
    antes de a animação soltar a flecha.

    Correção: `WorldInstance::informar_vida_aos_inscritos` no batimento de 1 s — para cada
    monstro vivo selecionado por alguém, compara `(vida, alvo)` com o último envio
    (`vida_informada`) e emite `EventoDoMundo::VidaDoMonstro` só a quem o selecionou. O
    `SELECT_TARGET` continua mandando na hora e registra o envio (`vida_ja_informada`). Os
    quatro envios imediatos saíram.

    ### c. Os sons — medido, sem causa

    Log do link da sessão (16:15–16:24 UTC, 5 logins): 2.097 `OBJECT_MOVE`, 297
    `NPC_ENTER_SLICE`, 222 `OWN_ITEM_INFO`, 177 `NPC_INFO_00`. Conferido:
    - nenhum NPC ou matéria entrou duas vezes sem ter saído (zerando a cada login);
    - nenhum comando corrige a posição do próprio personagem;
    - monstros andam 1 passo por segundo (`use_time` 1000), como o original;
    - `OBJECT_CAST_SKILL`, `OBJECT_TAKEOFF`/`LANDING`, `OBJECT_STAND_UP` e `OBJECT_DO_EMOTE` com
      o id do próprio jogador chegam a ele — e no original também
      (`AutoBroadcastCSMsg(..., -1)`, `player.cpp:3997-4057`, `4502-4527`); o cliente os entrega
      ao `CECHostPlayer` (`EC_ManPlayer.cpp:1486-1487`).
    No cliente, música e som de ambiente só recomeçam ao trocar de distrito ou de dia/noite
    (`EC_World.cpp:2178-2268`), e os sons de cenário são dos blocos em volta do personagem
    (`EC_SceneBlock.cpp:819-1022`). Os logs do cliente (`EC.log`, `AM.log`, `AF.log`) só cobrem o
    último login e não falam de som.

    Duplicatas reais achadas, **sem ligação provada com som** (anotadas no §5B do estado): 25
    comandos da carga de inventário/habilidades repetidos por login (o link manda no
    `EnterWorld`, o mundo de novo no `GET_ALL_DATA`), e `SELF_INFO_00` + `GET_OWN_MONEY` duas
    vezes por abate. Também anotado: o `QUERY_NPC_INFO_1` (68) responde `NPC_INFO_00`, e o
    original responde `NPC_INFO_LIST` (`npc.cpp:330-338`). Pergunta aberta: que som, onde e
    depois de quê.

    ### d. Provas

    Novo `a_barra_de_vida_vai_no_batimento_e_so_quando_muda`: sem seleção nada sai; selecionar
    manda na hora; batimento sem mudança não repete; uma mudança sai uma vez. Os testes de golpe
    e habilidade passaram a exigir que a barra **não** venha junto do resultado, e
    `atacar_debita_o_hp_de_verdade_do_monstro` confere que a do batimento traz a vida do mundo.
    Suíte com banco: **570 testes**; na rodada completa 569 passaram e
    `nao_da_para_entrar_num_grupo_sem_convite` falhou por tempo (400 ms de silêncio com o build
    do Docker em paralelo) — `subcomandos_no_mundo` sozinho passa 57/57. Contêineres
    reconstruídos: `pw-world-155` com os mapas 1 (29.620 monstros) e 161 (1.269), link religado
    ao barramento às 16:56:20 UTC.

57. **Sessão 2026-09-17: o golpe que saía junto com a habilidade, e a caça ao som repetido.**

    ### a. O relato

    Com o eaa, depois do B56: (1) a barra de atalhos voltou preenchida — B54 confirmado; (2) ao
    clicar num monstro já marcado, ele perdia vida antes de o disparo sair, e o mesmo ao usar
    uma habilidade, "como se um segundo ataque fosse lançado no momento da ação"; um monstro de
    172 de vida morria com uma habilidade que mostrava menos dano que isso; (3) sons de fundo —
    "passos ou asas batendo" — repetindo, que somem quando ele se afasta.

    ### b. O segundo golpe: medido no log do link

    `CAST_SKILL` (235) → 1 s de conjuração → `SKILL_PERFORM` (88) + `HOST_STOP_SKILL` (123)
    **sem resultado nenhum**; o cliente manda `NORMAL_ATTACK` 20 ms depois; e então, no mesmo
    milissegundo, saem o dano da habilidade (142, **251**) e o do golpe normal (24, **143**)
    num monstro de 172 de vida. Quatro repetições iguais no log das 17:30–17:31.

    Duas divergências do original explicam isso:
    - **A habilidade é a sessão corrente enquanto roda.** `AddSession` devolve `!_cur_session`
      (`actobject.cpp:1180-1212`): o `NORMAL_ATTACK` que chega durante a conjuração **só entra
      na fila**, e começa quando a habilidade termina (`SafeDeleteCurSession` → `StartSession`,
      `actobject.cpp:150-190`) — aí o `CheckAttack` recusa alvo morto. Nós abríamos a sessão de
      golpe na hora, porque `atacar` só olhava `p.ataque`, nunca `p.conjuracao`.
    - **O dano vem antes do fim da sessão.** No original o efeito sai do `RunSkill`
      (`session_skill::RepeatSession`, `actsession.cpp:576-600`) e só depois o `EndSession`
      manda `stop_skill` (`actsession.cpp:558-574`). Nós mandávamos o 123 primeiro, o que fazia
      o cliente retomar o golpe normal antes de a habilidade ter efeito.

    Correção: `BusServer::golpe_na_fila` (`roleid → alvo`), preenchido quando o
    `NORMAL_ATTACK` chega com conjuração aberta; a conjuração agora fica aberta até o efeito
    ser aplicado (a tarefa do fim não a tira mais, só confere o marcador); e
    `concluir_conjuracao` passou a ser `SKILL_PERFORM` → efeito (`aplicar_conjuracao`) →
    `HOST_STOP_SKILL` → `fechar_conjuracao_e_soltar_fila`. A fila é esvaziada ao morrer e ao
    sair do jogo.

    ### c. O som: o que foi medido, o que foi corrigido, o que continua aberto

    Medido no log do link e no fonte, **sem** achar a causa:
    - o passeio dos monstros é o do original: `CanRest` com `idle_timer > 0` e contador de 32
      (`ainpc.cpp:303-316`), `ai_rest_task` com 8 passos num raio de 10 m e 10% de emendar
      (`aipolicy.cpp:1122-1190`), `idle_timer` renovado para as fatias no alcance de visão do
      jogador (`world.h:778-820`, `player.cpp:9100`), passo de 1 s (`NPC_PATROL_TIME`,
      `npcsession.cpp:986`);
    - a velocidade no fio é 8.8 (`cmd_object_move.sSpeed`, `EC_GPDataType.h:1483-1490`) e bate
      com a distância andada (1,5 m/s);
    - o cliente só recria o modelo de um NPC num `NPC_ENTER_SLICE` repetido
      (`EC_ManNPC.cpp:855-875`) — e nenhum id entrou duas vezes sem ter saído;
    - música e som de ambiente do cliente só recomeçam ao trocar de distrito ou de dia/noite
      (`EC_World.cpp:2178-2268`); som de cenário é dos blocos em volta (`EC_SceneBlock.cpp`);
    - um monstro do mapa 161 anda com `move_mode` `0x40` (habitat **ar**): é o candidato ao
      "asas batendo".

    Corrigido o que estava mesmo errado: `MonstroAndou`/`MonstroParou` iam por
    `transmitir_a_outros(0, …)` — **todos os jogadores do mapa**. O original difunde na fatia do
    NPC (`AutoBroadcastCSMsg`, `npc.cpp:85-98`). Consequência medida: 319 comandos sobre 18
    monstros que aquele cliente nunca viu entrar; cada um o punha na fila de "NPC desconhecido"
    (`CECNPCMan::SeekOutNPC`, `EC_ManNPC.cpp:967-975`) e ele perguntava por eles a cada 10 s
    (`UpdateUnknownNPCs`, `EC_ManNPC.cpp:1144-1164`) — para sempre, porque respondemos
    `NPC_INFO_00` onde o original responde `NPC_INFO_LIST` (`npc.cpp:330-338`; dívida anotada
    no §5B do estado). Agora existe `transmitir_a_quem_ve`.

    ### d. Provas

    Novo `o_golpe_que_chega_conjurando_espera_a_habilidade`: com o `NORMAL_ATTACK` mandado
    durante a conjuração, a ordem recebida é 142 (resultado da habilidade) → 84
    (`HOST_START_ATTACK`) → 24 (golpe). `conjurar_em_si_mesmo_ainda_fecha_a_conjuracao` passou a
    esperar o 123 depois dos comandos do efeito, que é a ordem nova. Suíte com banco: **571
    testes, todos passando** (`subcomandos_no_mundo` sozinho, 58/58; na rodada cheia um teste de
    tempo falha por carga e passa sozinho). Publicado nos contêineres `pw-realm-155` e
    `pw-world-155`.

58. **Sessão 2026-09-17: o que o log prova sobre o golpe, e o batimento que empilhava som.**

    ### a. O relato

    Depois do B57: "ao atacar um monstro está indo um ataque básico invisível instantâneo, e ao
    usar skills está acertando 2x". Sobre o som: ao entrar numa zona toca música e som de fundo;
    num ponto o som de ambiente é de insetos e há "um ruído muito alto, como se estivesse
    rodando um em cima do outro"; e passos em certas zonas.

    ### b. O combate, medido

    Log do link de 20:39–20:42, com o B57 no ar: **uma ação, um dano**. Habilidade 235 tirou 232
    de um monstro de 172 de vida e o matou sozinha (sem golpe junto, que era o defeito do B57);
    golpe normal tirou 101 e, 1,47 s depois — a cadência do arco —, 119. Não há dano duplo.

    O que o Murillo vê tem duas causas, e as duas são do original:
    - **O dano é aplicado no clique.** `session_normal_attack::StartSession` chama `DoAttack` na
      hora (`actsession.cpp:350-378`) e a mensagem de golpe vai sem atraso (`PostLazyMessage`
      sem `delay_tick`, `world.h:452-465`; `MsgQueue2::AddMsg` sem latência,
      `global_manager.cpp:355-359`). O cliente só anima ao receber o resultado
      (`PlayAttackEffect`, `EC_HostMsg.cpp:943-955`) e só mostra o número quando a flecha chega
      (`nTimeFly = 700` ms para arma de longo alcance, `EC_Player.cpp:3507-3545`). A barra de
      vida sai no batimento de 1 s (B56) — antes do número. Conferido também que **nada no
      cliente desconta vida localmente**: a barra do alvo só é escrita pelo `NPC_INFO_00`
      (`EC_ManNPC.cpp:585-597`; nenhuma outra escrita em `iCurHP`).
    - **O golpe depois da habilidade é pedido pelo cliente.** A `CECHPWorkMelee` segue ativa e o
      `CECTracedNPC::OnTouched` manda `NORMAL_ATTACK` assim que a conjuração acaba
      (`EC_HPWorkTrace.cpp:811-818`); no original o servidor abre a sessão na hora, sem nenhuma
      trava de tempo entre golpes (`AddSession`/`StartSession`; não existe relógio de arma no
      servidor original — procurado por `_atk_timer`/`last_attack`).

    Corrigido o que divergia de fato: o `HOST_ATTACKRESULT` levava `attack_speed` (30 tiques do
    arco) onde o original manda `attack_delay = (int)(attack_speed × 0,8) − 1` = 23
    (`playertemplate.h:980`, mandado em `actobject.cpp:826`). Esse número é a duração da
    animação no cliente: a nossa saía ~350 ms mais lenta. Em golpe de habilidade o campo vai
    zero, como no original (só `MakeAttackMsg` o preenche).

    E, para o próximo relato ser contagem e não impressão, cada dano aplicado escreve uma linha
    em `pw_gs=debug`: `golpe normal de R em A: dano D, vida V/M` e `habilidade S de R em A: …`.

    ### c. O som: o defeito achado

    No log, dez monstros mandavam `OBJECT_MOVE` **no mesmo milissegundo**, de segundo em segundo
    (20:42:07.777, 08.863, 09.777, 10.777, …). Causa: o nosso batimento de IA é o mesmo limite
    de 1 s para todos. O original **espalha**: o coletor de batimentos pega
    `tamanho / TICK_PER_SEC` objetos por tique (`objmanager.h:213-229`, com
    `obj_manager<gnpc, TICK_PER_SEC>` em `worldmanager.h:262`), e cada NPC nasce com
    `idle_timer_count = Rand(0, NPC_IDLE_HEARTBEAT)` (`npcgenerator.cpp:2014`). Com todos no
    mesmo quadro, o cliente tocava dez sons de passo sobrepostos — o "ruído muito alto".
    Correção: `MonsterAi::new` sorteia a fase do batimento e a do passo de patrulha.

    Descartados, nesta ordem, com medição:
    - **matéria repetida**: 5 das 26 matérias foram anunciadas 2–3 vezes sem nunca sair da vista
      (dívida de streaming anotada), mas o cliente ignora id repetido
      (`CECMatterMan::MatterEnter`, `EC_ManMatter.cpp:372-377`) — não empilha som;
    - **reentrada de monstro**: zero `NPC_ENTER_SLICE` para monstro que o cliente já via (é o
      único caminho em que ele recria o modelo, `EC_ManNPC.cpp:855-875`);
    - **monstros empilhados**: nenhuma coordenada com mais de uma criatura;
    - **som de ambiente do distrito**: o cliente faz *fade-out* do anterior antes de trocar
      (`CELBackMusic::PlayBackSFX`, `EL_BackMusic.cpp:436-444`), e a troca só acontece ao mudar
      de distrito ou de dia/noite (`EC_World.cpp:2178-2268`). O "som de inseto" vem do
      `precinct.sev` **do cliente** e da posição do personagem, não de comando nosso.

    ### d. Provas

    Suíte com banco: **571 testes, todos passando**. Publicado em `pw-realm-155`/`pw-world-155`.

59. **Sessão 2026-09-17: as missões que pararam, o monstro que batia sem assinar, e os NPCs de
    costas para o mesmo lado.**

    ### a. O relato

    Teste com o eaa, nível 5: (1) os atributos do equipamento não aparecem na janela C; (2) ao
    mandar atacar, o monstro leva dano antes da animação; (3) o monstro se aproxima e não
    ataca; (4) todos os NPCs estão virados para a mesma direção; (5) **nenhuma missão nova
    aparece**, nem clicando em "Procurar Missão".

    ### b. Missões: o pacote dinâmico que nunca era mandado

    A cadeia da ilha está concluída no banco (31676→31679, 31687→31689, 32201). Nos dados, a
    continuação é a 31693, que exige a **31690 "Descobertas Acidentais"** — e essa tem
    `npc_que_entrega = 0` com `entrega_automatica`: quem a dispara é o cliente, que varre as
    automáticas (`ATaskTemplMan::CheckAutoDelv`, `task/TaskTemplMan.cpp:106-131`) e avisa o
    servidor (`TASK_CLT_NOTIFY_AUTO_DELV`), que entrega (`OnTaskAutoDelv`,
    `TaskServer.cpp:452-464`). Nós já tratávamos esse aviso — mas ele nunca chegava.

    A causa está um passo antes. No log do link, o cliente pedia a marca das missões dinâmicas
    (`TASK_NOTIFY` motivo 7, respondíamos) e **logo depois os dados** (motivo 8,
    `TASK_CLT_NOTIFY_DYN_DATA`), sem resposta nossa. Em `OnDynTasksTimeMark`
    (`TaskTemplMan.cpp:166-179`): se a marca bate e o pacote local carrega, ele chama
    `InitActiveTaskList()`; **senão pede os dados** — e só monta a lista de missões ativas
    quando o último pedaço chega (`OnDynTasksData`, `:181-230`). Sem isso o sistema de missões
    do cliente fica sem inicializar: nada novo aparece, nem no "Procurar Missão".

    Correção: `GameDataManager::missoes_dinamicas` guarda o `dyn_tasks.data` inteiro (12.979
    bytes) e o motivo 8 devolve o arquivo em pedaços de `0x1000 − sizeof(task_notify_base)` =
    **4.093** bytes, cada um com `reason 9` (`TASK_SVR_NOTIFY_DYN_DATA`) e `task = 1` só no
    último — o mesmo laço de `OnTaskGetDynTasksData` (`TaskTemplMan.cpp:321-353`).

    ### c. Monstro que batia sem assinar o golpe

    O log tinha 18 `HOST_ATTACKED` (26) — ou seja, o monstro **atacava**. O pacote, porém, saía
    com o id do atacante **zero**: `[1a 00 | 00 00 00 00 | 01 00 00 00 …]`. O cliente só reage
    se achar quem bateu (`ISPLAYERID`/`ISNPCID` são falsos para zero, `EC_HostMsg.cpp:968-1006`),
    então não havia animação nem reação — só a vida caindo. O `world.rs` empilhava
    `(alvo, dano)` e emitia `atacante: 0`, com um comentário admitindo a dívida. Agora o id do
    monstro vai junto, e o mesmo id vira o `matador` quando o jogador morre.

    Antes disso, para não trocar palpite por palpite, a IA foi medida no cenário exato do
    relato (Lobo Sangrento: alcance 3 m, ódio 35 m, jogador a 20 m): ela persegue e bate —
    teste `o_monstro_persegue_o_arqueiro_de_longe_e_bate`.

    ### d. NPCs virados para o mesmo lado

    Mandávamos `dir = 0` em todo `NPC_ENTER_SLICE`. O original escolhe em `GenDir()`
    (`npcgenerator.h:747-757`): área que é um ponto usa `_dir = a3dvector_to_dir(vDir)`
    (`npcgenerator.cpp:4367`), a conversão `atan2(z, x) × 128/π & 0xFF`
    (`common/types.h:99-107`); área com extensão sorteia `Rand(0,255)`. O `npcgen.data` já nos
    dava `dir` e a extensão da área — só não estavam sendo usados. Agora cada criatura nasce
    com a direção e ela vai no comando de entrada.

    ### e. Os dois relatos que a medição não confirmou como defeito

    - **Atributos na janela C**: o `OWN_EXT_PROP` mandado trazia vit 5, ene 5, for 5, agi 15 —
      exatamente a base do banco. O equipamento vestido não tem addon: a única peça com bloco
      de dados (item 267) decodifica com **0 addons**, e as outras não têm bloco. O número está
      certo para esse equipamento; o destaque verde da janela vem do que o cliente lê do bloco
      do item (`DlgCharacter.cpp:386-416`).
    - **"Dano antes da animação"**: com o log por dano do B58, é **uma linha por ação** (golpe
      138 e, 1,47 s depois, 145; habilidade 232 sozinha). O dano é aplicado no clique, como no
      original; o cliente anima ao receber o resultado e mostra o número quando a flecha chega.
      Alterar isso é sair do original — decisão do Murillo.

    ### f. Provas

    Novos testes: `as_missoes_dinamicas_vao_em_pedacos_com_reason_9` (framing e o `task = 1` do
    último), `a_direcao_do_gerador_e_a_do_original` (leste 0, norte 64, oeste 128, sul 192; área
    sorteia), `o_monstro_persegue_o_arqueiro_de_longe_e_bate`, e o `o_monstro_revida...` passou
    a exigir o id do monstro no `HOST_ATTACKED`. Suíte com banco: **574 testes, todos
    passando**. Publicado em `pw-realm-155`/`pw-world-155`.

60. **Sessão 2026-09-18: a segunda trava das missões, o item de missão sem atributo, e os
    dois campos zerados do golpe do monstro.**

    ### a. O relato

    Teste com o eaa: (1) o set Halo deveria vir com atributos — o peitoral mostra "Destreza
    +1~2" e não conta; (2) nenhuma missão automática chegou, mesmo depois do B59; (3) os
    monstros atacam sem executar a animação, e alguns deveriam usar habilidades.

    ### b. Missões: o dado de títulos tranca a varredura

    O B59 tinha resolvido o pacote de missões dinâmicas, e o log prova que ele foi:
    `5491 pediu as missões dinâmicas (12979 bytes)`. Ainda assim nada chegou. Depois disso o
    cliente manda os motivos 9 (`SPECIAL_AWARD`) e 12 (`STORAGE`), que não tratamos — mas não
    é isso que trava.

    A varredura das missões de entrega automática é `ATaskTemplMan::UpdateStatus`
    (`task/TaskTemplMan.cpp:1342-1350`), e a primeira linha dela é
    `if (!pTask->IsTitleDataReady()) return;`. Esse sinal só liga em `CECHostPlayer::InitTitle`
    (`EC_HostPlayer.cpp:10145-10152`), chamado ao receber `QUERY_TITLE_RE` (S2C 363) — a
    resposta ao `QUERY_TITLE` (C2S 154) que o cliente manda em todo login e que nós
    ignorávamos. Sem ela, `CheckAutoDelv` nunca roda e **nenhuma** missão automática aparece.

    Correção: respondemos `QUERY_TITLE_RE` com a lista vazia — `roleid`, `titlescount 0`,
    `expirecount 0`, os 12 bytes que o `CheckValid` do próprio struct calcula
    (`EC_GPDataType.h:4632-4654`). Não há sistema de títulos no banco; zero títulos é o que
    temos, e é o que destrava o cliente.

    ### c. Item de missão sem propriedade

    O peitoral Halo (28790) tem no modelo os addons 1130 (`enhance_agi_addon` = Destreza) e
    652 (defesa), mas a linha no banco estava **sem `extra_data`**. O cliente, sem bloco de
    dados, mostra a faixa do modelo no tooltip ("Destreza +1~2"), e o servidor não soma nada —
    o `OWN_EXT_PROP` medido no B59 trazia exatamente a base do personagem.

    Causa: `Bolsa::empilhar` criava o item seco. No original, **todo** item de prêmio de missão
    passa por `generate_item_for_drop` (`PlayerTaskInterface::DeliverCommonItem`,
    `task/taskman.cpp:281-303`) — a mesma geração do drop de monstro (`ADDON_LIST_DROP`) —, e o
    que se colhe também (`player.cpp:1500-1520`). Só a compra em loja usa outra lista
    (`generate_item_for_shop`, que não sorteia propriedade).

    Correção: `Bolsa::empilhar_gerado`, usado no prêmio de missão e na coleta. E as sete peças
    que o eaa já usava foram geradas pelo gerador do próprio servidor
    (`cargo run -p pw-gs --example gerar_octetos`) e gravadas em
    `scripts/2026_09_18_octetos_do_equipamento_do_eaa.sql`.

    ### d. O golpe do monstro: dois campos zerados

    Desde o B59 o `HOST_ATTACKED` leva o id certo (`0x80000649` no log). Faltavam dois campos
    que o original preenche:
    - `cEquipment` deve ser `0x7f`, "nenhuma peça desgastada". Com zero, o cliente entende
      "peça 0" e desconta durabilidade da **arma** a cada golpe recebido
      (`EC_HostMsg.cpp:968-976`).
    - `speed` é o `attack.speed` do original, que para monstro é o `_damage_delay` do
      `MONSTER_ESSENCE` em tiques de 50 ms (`npc.cpp:2118`); o cliente o passa para
      `PlayAttackEffect` como duração da animação (`CECNPC::OnMsgAttackHostResult`,
      `EC_NPC.cpp:2043-2064`).

    O caminho 1.5.5 do `PorVersao::host_attacked` descartava os dois e escrevia zero nos dois.
    Agora eles vão, com o `damage_delay` lido do template do monstro que bateu.

    **Habilidade de monstro continua não existindo**: 3.688 monstros têm habilidade no
    `MONSTER_ESSENCE`, mas quem decide usá-la é o `aipolicy.data` (`ai_skill_task`), e o
    intérprete não foi escrito — anotado na spec 05 §4.

    ### e. Provas

    Novo `o_premio_de_missao_entra_na_bolsa_com_o_equipamento_gerado`: o peitoral Halo entra na
    bolsa com bloco de dados, o bloco se relê, tem propriedade adicional e ela soma nos
    atributos. O `o_monstro_revida...` passou a exigir `cEquipment = 0x7f`. O teste-guarda dos
    codificadores cobrou (e ganhou) a entrada do `query_title_re`, e confirmou que ele escreve
    o id que o IR dá ao `QUERY_TITLE_RE`. Suíte com banco: **575 testes**; o arquivo de testes
    de tempo passa 58/58 sozinho (na rodada cheia desta data, dois deles falharam com o build
    do Docker rodando junto). Publicado em `pw-realm-155`/`pw-world-155`.

61. **Sessão 2026-09-18 (tarde): a durabilidade estava 100 vezes menor, o desgaste não
    existia, e a missão que "não vinha" é automática por zona.**

    ### a. O relato

    Depois do B60: as missões automáticas passaram a chegar e o set Halo veio com atributo.
    Mas (1) "estou nível 5 e sem nenhuma missão para progredir — veja se as que completei não
    disparariam uma próxima" e (2) "o arco que ganhei numa missão tem durabilidade **1/1**;
    confira a geração e o decremento".

    ### b. A missão: nada quebrado, e a prova está nos dados

    O eaa terminou a cadeia dos Alados em **31689 "Líder Inspirador"** (NPC 44396). Ela não
    tem filho por pré-requisito nem `rewards.nova_missao` — conferido na ficha. O próximo elo
    é a **31690 "Descobertas Acidentais"**, `entrega_automatica`, nível 4–8.

    Quem decide pedir uma automática é o **cliente**: `ATaskTemplMan::CheckAutoDelv`
    (`Task/TaskTemplMan.cpp:106-131`) percorre o `m_AutoDelvMap` a cada tique e só manda o
    `AUTO_DELV` das que passam no `CheckPrerequisite` **dele**. E ali a zona é conferida com a
    posição do próprio cliente (`CheckInZone`, `TaskTempl.inl:368-393`: `ulWorldId !=
    m_ulDelvWorld` ou fora de todas as caixas). A 31690 pede mundo 161 e duas caixas —
    `x -775..-636, z -113..-28` e `x -609..-414, z -132..-17` —, ao norte dos NPCs 44390
    (`z -237`) e 44396 (`z -275`). O eaa estava gravado em `x -773,9 / z -160,6`: fora das
    duas. As três automáticas que **entraram** (19684, 32393, 32411) são todas sem zona.

    Para não repetir o diagnóstico à mão, ficou o exemplo
    `cargo run -p pw-gs --example missoes_automaticas -- <config> <nível> <classe> <mundo>
    <x> <y> <z> <concluídas>`: na posição dele a 31690 sai como "fora da zona", e em
    `-700 40 -70` sai como "PEDE". (O exemplo confere só nível, classe, pré-requisito e zona;
    o cliente aplica mais critérios, então o total que ele imprime é um teto, não uma lista.)

    De quebra: a 19684 "GM·Não entre" entrou na lista dele porque a conta `admin` tem
    `gm_privileges = 32` — o `CheckGM` do motor está certo.

    ### c. Durabilidade: faltava a escala

    Uma unidade de durabilidade na tela são **100 pontos internos** (`DURABILITY_UNIT_COUNT`,
    `gs/config.h:59`; `ENDURANCE_SCALE`, `EC_IvtrTypes.h:26`). O gerador do original termina
    chamando `update_require_data` (`gs/item/item_addon.h:454-458`), que multiplica
    `durability` e `max_durability` por 100 — nas quatro famílias
    (`generate_item_temp.h:367, 552, 644, 831`). Nós gravávamos o número cru do
    `elements.data`. O cliente divide de volta **arredondando para cima**
    (`(v + 99) / 100`, `EC_IvtrEquip.cpp:281`), então o ★Arco Real, com `durability_min = 50`,
    aparecia como **1/1** — e o set Halo inteiro também (9/18 → 1/1, 85/85 → 1/1).

    Corrigido em `geracao.rs` e no `Bolsa::empilhar`; a munição passou a 100 (o original grava
    1 e multiplica, `:615`). Os itens já criados foram convertidos por
    `scripts/2026_09_18_durabilidade_na_escala_interna.sql` (9 linhas: tudo com
    `max_durability < 100`; o que vem de `class_template_items` já estava em 2800).

    O bloco de dados também carrega a durabilidade. Em vez de mantê-lo em sincronia a cada
    golpe, o `BusServer::info_de` passou a **regravar** a durabilidade do bloco com a da
    coluna antes de mandar (`pw_core::escrever_durabilidade`, deslocamento fixo 12/16, depois
    dos seis `short` de requisito) — a coluna é a fonte, o bloco é a cópia que viaja.

    ### d. O desgaste, que não existia

    Portado com os números do original:

    - **Golpe normal dado**: a arma perde `DURABILITY_DEC_PER_ATTACK` = 2 (`gs/config.h:61`),
      porque `FillAttackMsg` chama `DoWeaponOperation<0>` (`player.cpp:3133`) →
      `weapon_item::OnAfterAttack` (`item/equip_item.cpp:978-988`). **Habilidade não gasta
      arma**: `FillEnchantMsg` não chama, e o original explica em comentário (`:3174`).
    - **Golpe recebido**: `SelectRandomArmor` sorteia um slot de 1 a 10
      (`EQUIP_ARMOR_START..EQUIP_ARMOR_END-1`, `gs/item.h:194-241`); com peça ali, ela perde
      `DURABILITY_DEC_PER_HIT` = 25 (`gs/config.h:60`) e **o índice dela vai no
      `cEquipment`** do `HOST_ATTACKED` — o B60 mandava `0x7f` fixo, que é o que o
      `Make<be_attacked>` produz do -1 de slot vazio (`eq_index &= 0x7F`,
      `cgame/common/protocol_imp.h:580-590`). O cliente desconta o mesmo por conta própria
      (`ARMOR_RUIN_SPEED -25`, `EC_HostMsg.cpp:974-981`).
    - **Chegou a zero**: para em zero e, **uma vez**, sai `EQUIP_DAMAGED` (68, 4 bytes, motivo
      0) mais o recálculo do equipamento (`player.cpp:9563-9567`). Peça em zero não conta em
      nada: `equip_item::VerifyRequirement` exige `durability > 0`
      (`item/equip_item.cpp:60-80`).

    No banco isso é um `UPDATE ... RETURNING` que prende em zero e não escreve quando já está
    zerada (`ItemRepository::gastar_durabilidade`), então o aviso de quebra não se repete.

    ### e. Provas

    `a_durabilidade_gerada_vai_na_escala_interna` (o arco 36121 do realm sai 5000/5000 e o
    cliente mostraria 50), `a_peca_com_durabilidade_zerada_nao_entra_nos_atributos`,
    `desgastar_para_em_zero_e_avisa_quando_a_peca_quebra` (25 → 25 → quebra → nada), e o
    `o_monstro_revida_e_o_cliente_fica_sabendo` passou a vestir os dez slots e a exigir que o
    `cEquipment` seja um deles, que a peça tenha perdido 25 e que a arma tenha perdido 2. O
    teste-guarda dos codificadores cobrou e confirmou o id 68 do `EQUIP_DAMAGED`. Suíte com
    banco: **578 testes, todos passando**. Publicado em `pw-realm-155`/`pw-world-155`.

62. **Sessão 2026-09-18 (fim da tarde): o dano não tira vida na hora do golpe.**

    ### a. O relato

    O B61 passou em jogo: durabilidade certa, monstros atacando, e a missão 31690 chegou ao
    mudar de área. Sobraram dois sintomas, que acabaram sendo **um**: (1) o monstro, ao
    alcançar o jogador, batia ainda executando a animação de corrida; (2) no golpe do
    arqueiro, ao abrir a sessão o monstro perdia vida antes de a animação começar — "sempre um
    ataque extra". Mais uma pergunta: existe aviso de durabilidade baixa?

    ### b. `InsertDamageEntry`: o dano é adiado, o aviso não

    Já sabíamos que `session_normal_attack::StartSession` (`actsession.cpp:350-378`) manda o
    `start_attack` e chama `DoAttack` **na mesma hora** — igual ao nosso. O que faltava estava
    depois: `gactive_imp::HandleAttackMsg` termina com

    ```
    OnDamage(...);                                    // o aviso ao cliente, agora
    InsertDamageEntry(int_damage, attack->speed, ...); // a vida, depois
    ```

    e `InsertDamageEntry` (`actobject.cpp:1758-1776`) só chama `DoDamage` quando `delay <= 0`;
    com `delay > 0` ele posta um `GM_MSG_HURT` **adiado de `delay` tiques de 50 ms**. O
    `delay` é o `attack.speed`:

    - jogador: `attack.speed = _cur_item.attack_delay` (`MakeAttackMsg`, `actobject.cpp:824`),
      e `attack_delay = (int)(attack_speed × 0,8) − 1` em tiques (`playertemplate.h:980`) — o
      mesmo número que o `HOST_ATTACKRESULT` leva desde o B58;
    - monstro: `attack.speed = _damage_delay` (`gnpc_imp::DoAttack`, `npc.cpp:2118`) — o mesmo
      do `HOST_ATTACKED` desde o B60;
    - habilidade: **não é preenchido** (`FillEnchantMsg` não mexe em `speed`,
      `player.cpp:3174`), então `delay = 0` e o dano é imediato.

    E é o mesmo número que o cliente usa como duração da animação do golpe
    (`PlayAttackEffect(..., speed × 50)`). Ou seja: no original a vida cai **quando a animação
    termina**. Nós aplicávamos no instante do clique — daí a vida cair antes da flecha sair, e
    o monstro tirar vida enquanto o cliente ainda o desenhava correndo (o cliente termina o
    caminho do `stop_move` por conta própria, `CECNPC::MovingTo`, `EC_NPC.cpp:1209-1225`).

    Portado como `WorldInstance::adiar_dano` + `cobrar_danos_adiados`, cobrado no começo do
    tique. **Continuam no instante do golpe**, como no original: a ameaça (que está em
    `OnAttacked`, não em `DoDamage`), o estado de combate e o desgaste da arma. A morte do
    alvo passou a ser resolvida quando o dano vence, pelo evento `MonstroMorreu` — que virou o
    caminho único das três origens (golpe normal, habilidade e dano no tempo), no lugar do
    antigo `MonstroMorreuDeEfeito` mais o tratamento solto dentro do `golpear`.

    ### c. A cadência do monstro vinha escrita no código

    `self.attack_cooldown_ms = 1500` valia para os 29.620 monstros do mapa 1. O original usa
    `_cur_prop.attack_speed` do `MONSTER_ESSENCE` (`ChangeInterval`, `npcsession.cpp:60-70`).
    O `MonsterEntity` ganhou `ataque_em_ticks` e `atraso_do_dano_em_ticks` — que o
    `TemplateDeMonstro` já lia e ninguém usava — e a IA passou a usar o primeiro.

    ### d. O aviso de durabilidade baixa existe, e é do cliente

    `CECGameUIMan::RefreshBrokenList` (`EC_GameUIMan.cpp:5474-5555`) roda a cada quadro: toda
    peça vestida com `cur <= max / 10` entra na janela `Win_Broken`, com o ícone **amarelo**
    (192,192,0) enquanto sobra durabilidade e **vermelho** (192,0,0) em zero. A aljava entra
    quando as flechas caem abaixo de 15%; asa, espada voadora e moda nunca entram. Não há
    comando de servidor nenhum: basta a durabilidade que mandamos estar certa, o que o B61
    resolveu. O item 8/18 do relato está em 44% — ainda longe do aviso.

    ### e. Provas

    Novo `a_vida_do_monstro_so_cai_depois_do_atraso_do_golpe`: depois do `HOST_ATTACKRESULT` a
    vida ainda está cheia, o dano só entra depois de `attack_delay` tiques, e o valor aplicado
    é exatamente o anunciado. Três testes existentes passaram a esperar o tique para ver a
    morte (`o_monstro_morre_e_o_abate_leva_o_template_certo`,
    `morrer_avisa_o_cliente_e_reviver_devolve_a_vida`,
    `esc_andar_e_a_morte_do_alvo_param_o_golpe`) — o que, por si, é a prova de que o dano
    deixou de ser instantâneo. O `esperar_comando` dos testes passou de 20 para 60 pacotes:
    com o dano adiado, o `NPC_DIED` chega atrás do que o combate em curso já enfileirou.
    Suíte com banco: **579 testes, todos passando**. Publicado em
    `pw-realm-155`/`pw-world-155`.

65. **Sessão 2026-09-18 (noite): resolução dos 5 problemas do teste em jogo (B67).**

    ### a. O relato

    No teste em jogo do Murillo com o Arqueiro `eaa` (1.5.5 BR, nível 5, mapa 161):
    1. Baú da missão do Selo Divino (mina 44559) dropava `<ERRO>` e item 0 na bolsa.
    2. Dano caía só no fim da animação (B62), mas o monstro começava a perseguir antes da flecha acertar.
    3. Menu R de habilidades não deixava evoluir nada.
    4. Habilidades mostravam Portal da Cidade e as duas iniciais duplicadas na lista.
    5. ESC não cancelava a conjuração do Portal da Cidade (167) nem de outras habilidades.

    ### b. Causa e solução de cada item

    1. **Mina de saída de missão (`OnTaskMining`):** A mina 44559 tem `materials_1_id = 0` e
       `task_out = 31694`. No original (`TaskServer.cpp:1116-1123`, `TaskTempl.inl:2105-2148`),
       quando a mina tem `task_out > 0`, o servidor executa `CheckMining`, entregando o item pedido
       pela submissão (item 44347) diretamente na bolsa do jogador e finalizando a submissão. Quando
       `material.item == 0`, nenhum drop deve ser gerado no chão. Implementado `Motor::colheu_mina`
       e tratado em `concluir_coleta` no GS. Excluído também o item 0 que havia ficado gravado no slot 15
       do `eaa`.
    2. **Ameaça do monstro adiada:** No original (`npc.cpp:1867`, `npc.cpp:2354-2364`),
       `AddAggroEntry` envia `GM_MSG_GEN_AGGRO` como mensagem adiada (`speed + 1`). A ameaça só
       nasce quando o projétil/golpe atinge o alvo. Movida a chamada de `ai.add_threat` de `atacar`
       para `aplicar_dano_no_monstro`.
    3. **Evolução de habilidades pelo menu R (`SCENE_SERVICE_NPC_LIST`):** No cliente 1.5.5
       (`EC_HostSkillModel.cpp:558-605`), ao receber `S2C::SCENE_SERVICE_NPC_LIST` (opcode 390), ele
       percorre os NPCs de serviço da cena (`m_allProfNPCs`) e define `m_skillLearnNPCNID`. Sem esse
       pacote no login (`todos_os_dados`), o cliente desabilita `IsSkillServedByNPC(skillID)` e o botão
       de evoluir pelo menu R. Adicionado opcode 390 ao `pw-protocol` e envio de `scene_service_npc_list`
       no `todos_os_dados` do GS.
    4. **Duplicação de habilidades e inventário:** O `pw-link` enviava `SKILL_DATA` (90), `OWN_IVTR_DATA`
       (42) e `OWN_ITEM_INFO` (40) logo após o `ENTER_WORLD`, antes do cliente estar pronto
       (`HostIsReady() == false`), e em seguida o GS reenviava esses dados em `todos_os_dados`. O envio
       no link agora é condicionado a `self.uplink_da_sessao(&session).is_none()`, deixando o mundo simulado
       como autoridade única e eliminando a duplicação.
    5. **Cancelamento de conjuração por ESC e movimento:** No C++ original (`playercmd.cpp:2136-2153`,
       `player.cpp:4017-4028`), `CANCEL_ACTION` e movimento enviam `SELF_SKILL_INTERRUPTED` (opcode 87,
       motivo 2) para o conjurador e `SKILL_INTERRUPTED` (opcode 86) para outros jogadores, limpando a
       conjuração. Adicionados opcodes 86 e 87 em `pw-protocol` e tratadores em `CANCEL_ACTION` e `mover`
       no GS (`interromper_conjuracao`).

    ### c. Provas

    - `crates/pw-gs/tests/missoes_do_realm.rs`: `coletar_o_selo_divino_da_missao_31694_entrega_item_44347_e_finaliza`
      aceita 31693 com submissão 31694, coleta a mina de saída e verifica a entrega do item 44347 e finalização.
    - `crates/pw-gs/tests/subcomandos_no_mundo.rs`:
      - `esc_cancela_conjuracao_com_self_skill_interrupted`: verifica limpeza de conjuração e emissão do opcode 87 (reason 2).
      - `get_all_data_envia_scene_service_npc_list`: verifica emissão do opcode 390 com os provedores de serviços.
      - `reacao_do_monstro_so_ocorre_quando_o_dano_atinge_o_alvo`: verifica que o monstro não gera ameaça nem persegue antes do dano conectar.
    - `crates/pw-protocol/tests/subcomandos_s2c_contra_o_ir.rs`: validação dos opcodes 86, 87 e 390 contra o IR.
    - Suíte inteira de testes automatizados passando.

66. **Sessão 2026-09-19 (tarde): correção do diálogo com NPC (B66).**

    ### a. O relato

    No teste em jogo do Murillo com o Arqueiro `eaa` (1.5.5 BR, nível 5, mapa 161) após a B65:
    "Ao testar percebi que não estou mais conseguindo falar com NPC (não abre a caixa de diálogo), veja o que você fez de errado e corrija".

    ### b. Causa e solução de cada item

    1. **Sequestro do diálogo pelo opcode 390 (`SCENE_SERVICE_NPC_LIST`):**
       - Na B65, o GS passou a enviar no login todos os 208 NPCs do mapa 161 via opcode 390.
       - No cliente 1.5.5 oficial (`EC_HostSkillModel.cpp:575`), o cliente filtrou os NPCs de profissão e gravou o NID em `m_skillLearnNPCNID`.
       - Ao clicar no NPC no jogo, o cliente recebe `NPC_GREETING (70)`. Em `EC_HostMsg.cpp:2627`:
         ```cpp
         if (CECHostSkillModel::Instance().IsSkillLearnNPC(pCmd->idObject)) {
             CDlgSkillAction* dlg = dynamic_cast<CDlgSkillAction*>(pGameUI->GetDialog("Win_SkillAction"));
             dlg->SetReceivedNPCGreeting(true);
             return; // Aborta sem abrir PopupNPCDialog!
         }
         ```
         O cliente suprimiu a abertura do diálogo de NPC (`PopupNPCDialog`) porque considerou o greeting uma resposta silenciosa de aprendizado do menu R (`SendHelloToSkillLearnNPC`).
       - No servidor oficial C++ (`player.cpp:11735`, `world.cpp:1506`, `servicenpc.cpp:265`), o opcode 390 **só** envia NPCs com `_serve_distance_unlimited = true` (serviços remotos/globais). No `elements.data` v156 (1.5.5 BR) há 0 NPCs com essa flag.
       - Ajustado `world.rs` (`scene_service_npcs`) para retornar lista vazia e nunca enviar NPCs comuns da cena.
    2. **Bolsa de missões (bolsa 2) não inicializada no cliente:**
       - Na B65, removemos `OWN_IVTR_DATA` do login no `pw-link`. No `pw-gs` (`todos_os_dados`), o `OWN_IVTR_DATA (2)` só saía se `pedido.detalhe_missoes != 0`.
       - O cliente 1.5.5 envia `c2s_CmdGetAllData(true, true, false)` (`EC_HostPlayer.cpp:559`), logo `detalhe_missoes = 0`.
       - No servidor C++ oficial (`player.cpp:13233-13248`), o `OWN_IVTR_DATA` (42) das três bolsas (0 = inventário, 1 = equipamento, 2 = missão) vai **sempre** (`PlayerGetInventory`), e o booleano `detail_*` só controla se envia os blocos `OWN_ITEM_INFO (40)` individuais.
       - Sem a bolsa de missão inicializada no cliente, o cliente falhava na interação com NPCs de missão.
       - Ajustado `bus_server.rs` (`todos_os_dados`) para enviar as 3 bolsas incondicionalmente via comando 42.
    3. **Habilidades sem alvo em si mesmo (Portal da Cidade 167):**
       - Em `bus_server.rs` (`conjurar`), quando a mensagem de cast vem sem alvo e sem target selecionado no mundo, o alvo agora faz fallback para o próprio conjurador (`roleid as i64`).
    4. **Log de diálogo:**
       - Adicionado log informativo no GS ao receber `dizer_ola_ao_npc`: `mundo: {roleid} abriu diálogo com NPC {target}`.

    ### c. Provas

    - `crates/pw-gs/tests/subcomandos_no_mundo.rs`:
      - `get_all_data_envia_bolsas_incondicionalmente_sem_sequestrar_npcs`: confere o envio das bolsas 0, 1 e 2 no opcode 42 e ausência de NPCs comuns no opcode 390.
      - `get_all_data_respeita_os_sinalizadores_do_cliente`: atualizado para conferir que `OWN_IVTR_DATA (42)` vai sempre para as 3 bolsas e `OWN_ITEM_INFO (40)` é respeitado conforme os flags.
      - `esc_cancela_conjuracao_com_self_skill_interrupted`: verifica cancelamento com Portal da Cidade sem alvo explícito.
    - Suíte inteira de testes compilando e passando.
    - Imagens Docker `pw-world-155` e `pw-realm-155` recompiladas e serviços reiniciados com sucesso.

67. **Sessão 2026-09-20: auditoria do que veio de outra sessão, o cultivo que a missão não
    dava, poção no tempo, amuleto sem conteúdo.**

    ### a. Auditoria (pedida pelo Murillo)

    Outra sessão (outro modelo) mexeu no projeto. Conferi as duas correções que ela relatou:

    - **Id de monstro invocado (`0xA000_0000`)**: **certo**. O cliente separa as famílias por
      máscara (`ISNPCID = (id & 0x80000000) && !(id & 0x40000000)`, `EC_GPDataType.h:25-27`), e
      `0xD000_0000` casava com `ISMATTERID` — o monstro virava item de chão e não dava para
      mirar. A faixa nova satisfaz `ISNPCID` e não colide com o `npcgen` (que usa
      `0x80000000 | contador`, `npcgen.rs:568`).
    - **`prob = 100.0` por padrão no roteiro de habilidade**: **errado**, e foi substituído.
      No original quem decide é **cada setter**: dos 486 `PlayerWrapper::Set*`
      (`cskill/skill/playerwrapper.cpp`), **316** abrem com `if (ThrowDice())` e **170**
      aplicam direto. `probability` nasce em zero (`playerwrapper.h:61`) e `ThrowDice()` com
      zero é falso (`:169-178`). Pôr 100 por padrão tornava garantido **todo** efeito
      probabilístico de 3.316 habilidades. O certo, e o que a Flecha Fulgurante (244) precisa,
      é que `SetFirearrow` **não pergunta** (`playerwrapper.cpp:2333-2337`) — a habilidade não
      tem `SetProbability` nenhum (`cskill/skills/skill244.h:234-240`). Entrou a lista
      `GARANTIDOS_SEM_DADO`, **extraída do fonte**, e o padrão voltou a zero.

    Também: os contêineres `pw-realm-155b`/`pw-world-155b` (com `image:` em vez de `build:`)
    foram removidos e o `docker-compose.yml` voltou ao que era — `pw-realm-155` e
    `pw-world-155`, construídos do fonte. E dois testes que tinham ficado vermelhos
    (`incubar_ovo_de_montaria…`, `pegar_item_de_missao…`) passaram a montar no cenário o dado
    que pediam (o cenário não carrega `elements.data`), mais dois codificadores que faltavam
    no teste-guarda (`gain_pet`, `pet_room`).

    ### b. O cultivo que a missão não dava

    "Fiz a primeira missão de cultivo do nível 9 e não ganhei chi nem subi o cultivo." O chi
    (SP) ele ganhou — 3.000 da 32394 "Só um Pouco de Progresso", conferido no banco. O que
    faltou foi o **nível de cultivo**, e ele vem da **submissão** 32416 "Adepto Espiritual",
    filha da 32394.

    No original o cultivo é o `m_ulNewPeriod` do prêmio: `if (pAward->m_ulNewPeriod)
    pTask->SetCurPeriod(...)` (`Task/TaskProcess.cpp:1284`) →
    `PlayerTaskInterface::SetCurPeriod` (`gs/task/taskman.cpp:251-254`) →
    `gplayer_imp::SetSecLevel` (`gs/player_imp.h:2798-2804`), que grava em `_basic.sec_level`
    e manda `task_deliver_level2`. **Não líamos esse campo** — deslocamento 25 do
    `AWARD_DATA`, conferido contra a ordem da struct (`Task/TaskTempl.h:1136-1144`), com os
    outros seis deslocamentos que já tínhamos batendo exatamente.

    De quebra, isto desfez uma confusão nossa: o `level2` dos pacotes de visão é o **cultivo**
    (o cliente tira dele o título taoista e toca o efeito de avanço, `EC_GameRun.cpp:3477`,
    `EC_Player.cpp:7447`), e nós mandávamos ali o **privilégio de GM** da conta. Agora
    `VistaDoJogador` tem os dois campos: `cultivo` (vai no `level2`) e `sec_level` (só acende
    a coroa). São 18 missões que dão cultivo no `realm_155`
    (`cargo run -p pw-gs --example missoes_de_cultivo`). O eaa foi acertado para cultivo 1 por
    `scripts/2026_09_20_cultivo_do_eaa.sql`.

    ### c. Poção: o total é repartido pelo tempo

    O `MEDICINE_ESSENCE` tem `hp_add_total` **e** `hp_add_time` (a Poção Pequena de Cura: 25
    em 10 s). No original são três itens diferentes (`gs/item/item_potion.h:17-34`): a de vida
    e a de mana criam um filtro que entrega `total / tempo` **a cada batimento de 1 s**
    (`healing_potion_filter::Heartbeat`, `gs/potion_filter.h:60-66`), e só a
    `rejuvenation_potion` — a que tem vida e mana, sem tempo — cura na hora
    (`item_potion.cpp:55-70`). Curávamos tudo de uma vez em qualquer caso. Entraram os efeitos
    `PocaoDeVida`/`PocaoDeMana`, com o `Merge` do original (soma tempo e total e reparte de
    novo). A recarga (`cool_time`) continua `falta`.

    ### d. Amuleto e hierograma sem conteúdo

    "Amuletos de HP e MP vêm com valores negativos e zerados." O item ia **sem bloco de
    dados**. O conteúdo deles são 8 bytes e nada mais: `amulet_essence { int point; float
    trigger_percent; }` (`gs/item/item_amulet.h:16-19`), escrito sem cabeçalho de requisito
    (`generate_item_temp.h:2296-2310`); no `elements.data` são `total_hp`/`total_mp` e
    `trigger_amount` (Amuleto do Guardião - 1: 5400 e 0,5). Agora o `item_info` monta esse
    bloco.

    ### e. Diagnosticado, não corrigido: a animação do buff

    "Ao receber o buff falta a animação do personagem." O comando que a toca é o
    `ENCHANT_RESULT` (139) — `CECPlayer::OnMsgEnchantResult` chama `PlayAttackEffect(alvo,
    skill, nível, -2, …)` (`EC_Player.cpp:7244-7269`) —, nós o mandamos e o layout bate com o
    IR (21 bytes). A diferença que sobra é a **segunda fase** da habilidade: o stub da 244 tem
    `State1` de 3.000 ms (conjuração) e `State2` de 800 ms (execução, `skill244.h:20-80`), e
    nós só modelamos o primeiro — o `HOST_STOP_SKILL` sai logo depois da conjuração, cortando
    a fase que o cliente animaria. Fica como o próximo passo das habilidades.

    ### f. Provas

    Novos: `a_lista_de_garantidos_esta_ordenada_e_tem_o_firearrow`,
    `sem_probability_o_garantido_entra_e_o_probabilistico_nao`,
    `o_amuleto_e_o_hierograma_saem_com_os_oito_bytes_da_essencia`,
    `a_pocao_de_vida_traz_o_total_e_o_tempo`. Suíte com o banco no schema `test`.

68. **Sessão 2026-09-20 (parte 2): a segunda fase da habilidade, os pontos de teleporte, e a
    arquitetura por versão sem fachada.**

    ### a. Fim da auditoria da sessão de fora

    Além do que o item 67 conta, três divergências a mais foram achadas e corrigidas:

    - **Bônus da Flecha Fulgurante** saía sobre o dano **total** do personagem;
      `filter_Firearrow::TranslateSendAttack` (`cskill/skill/skillfilter.h:4268-4275`) usa o
      dano **da arma vestida**: `ratio × 0,5 × (weapon.damage_low + weapon.damage_high)`, só
      em golpe físico, somado em `magic_damage[3]` (fogo).
    - **A ficha do filtro** dizia `UNIQUE`; o original é `FILTER_MASK_WEAK` (`:4237-4239`) —
      com um já ativo, o novo é descartado.
    - **Monstros invocados pela missão**: no ramo sem `m_bRandChoose` o original invoca
      **todos**, sem sortear (`TaskProcess.cpp:1427-1434`).

    O resto (id do monstro invocado, duplicatas do login tiradas do `gateway.rs`,
    interrupção por ESC, serviço de incubar) confere com o original.

    ### b. A habilidade tem duas fases

    "Ao receber o buff falta a animação do personagem." O `ENCHANT_RESULT` (139), que é quem
    manda o cliente animar (`CECPlayer::OnMsgEnchantResult` → `PlayAttackEffect(alvo, skill,
    nível, -2, …)`, `EC_Player.cpp:7244-7269`), já saía com o layout certo. O que faltava era
    a **fase de execução**.

    A sessão de habilidade do original é um laço de estados: `StartSkill` devolve o tempo do
    primeiro, `RunSkill` o do seguinte, e só quando não há próximo vem o `EndSession` — que é
    quem manda o `stop_skill` (`gs/actsession.cpp:466-600`). A Flecha Fulgurante tem
    `State1` de 3.000 ms e `State2`/`GetExecutetime` de **800 ms**
    (`cskill/skills/skill244.h:20-80`); nós mandávamos o `HOST_STOP_SKILL` logo depois do
    efeito, cortando a fase que o cliente animaria. O dado já estava no
    `habilidades.json` (campo `execucao_ms`, extraído do `GetExecutetime`) e ninguém o usava.

    ### c. Pontos de teleporte

    O que o cliente pede e o que o servidor responde:

    - `ACTIVATE_REGION_WAYPOINTS` (C2S 178): o cliente lista os pontos da região onde está.
      O original cruza com a tabela daquela região e chama `ActivateWaypoint` para cada um
      (`gs/player.cpp:25196-25220`), que **só faz algo se o ponto for novo**: guarda em
      `_waypoint_list` e manda `activate_waypoint` (`player_imp.h:2534-2544`).
    - `ACTIVATE_WAYPOINT` (S2C 179, `unsigned short`): é **este** comando que faz o cliente
      anunciar "novo ponto de teleporte" com o nome do lugar e mostrar a dica
      (`CECHostPlayer::OnMsgHstWayPoint`, `EC_HostMsg.cpp:4681-4720`). O `WAYPOINT_LIST` (180)
      **substitui** a lista inteira, em silêncio, e é o da carga inicial.

    Antes disso o `pw-link` respondia o 178 devolvendo os mesmos ids (um eco que só servia
    para o cliente parar de repetir o pedido): nada era guardado e nada era anunciado. Agora
    o comando é tratado no mundo, a lista vive em `characters.waypoints` (u16 LE, o formato do
    `GetWaypointBuffer`) e volta na carga.

    **Falta viajar.** O `transmit_provider` (`gs/serviceprovider.cpp:683-855`) recebe o índice
    do destino, confere nível e dinheiro, cobra e chama `LongJump`. O `NPC_TRANSMIT_SERVICE`
    do `elements.data` dá `idTarget`, `fee` e `required_level` de até 32 destinos, mas **não
    a coordenada** — no original ela vem do `npc_template`, o arquivo de serviço do mapa, que
    ainda não lemos. Sem ela não há para onde teleportar, e inventar coordenada seria palpite.

    ### d. A arquitetura por versão, sem fachada

    `PorVersao` foi **removido**. Ele só delegava para `Arc<dyn WorldProtocol>`, e manter dois
    nomes para a mesma coisa convida a escrever `if versao == ...` de novo. O `BusServer` e o
    `pw-link` passaram a guardar a estratégia da versão
    (`versions::create_world_protocol(versao)`), e os testes também. A composição continua:
    `V148Protocol(V155Protocol)` sobrescreve só o que difere do 1.5.5 — é assim que 1.4.8 e
    1.7.2 entram quando forem medidos.

    O 1.2.6 continua com implementação própria e completa (`versions/v126/`). Os comandos que
    entraram do B60 ao B67 (`QUERY_TITLE_RE`, `EQUIP_DAMAGED`, `TASK_DELIVER_LEVEL2`,
    `ACTIVATE_WAYPOINT`, `GAIN_PET`, `PET_ROOM`) ainda **não foram medidos** no 1.2.6: valem
    o layout do 1.5.5 até que uma captura diga o contrário, e o lugar de registrar a
    diferença já existe.

    ### e. Provas

    `a_flecha_fulgurante_tem_conjuracao_e_execucao` (3.000 ms e 800 ms, do stub),
    `a_flecha_fulgurante_adiciona_icone_70_e_dano_de_fogo_ao_ataque` refeito sobre o dano da
    arma, e o teste-guarda do `pw-link` ensinado a ler braço com condição. Suíte com o banco:
    **598 testes, todos passando**.

69. **Sessão 2026-09-20 (parte 3): a barra de chi, o item de voo sem classe, e o teleporte
    que faltava.**

    ### a. O chi só existe depois que uma missão o concede

    "Ao atacar não está enchendo o chi." O mecanismo inteiro:

    - O chi é o `_basic.ap` do original (a "fúria"), preso entre 0 e `_base_prop.max_ap`
      (`ModifyAP`, `gs/actobject.h:1642-1657`).
    - **O teto vem de missão**: prêmio `m_ulFuryULimit` (deslocamento **57** do `AWARD_DATA`,
      entre `PetInventorySize` e `TransWldId`, que já estavam validados) →
      `SetFuryUpperLimit` → `gplayer_imp::SetMaxAP` (`gs/task/taskman.cpp:498-501`). São 8
      missões no `realm_155`, e a primeira é a **32394 "Só um Pouco de Progresso"** — a mesma
      de nível 9 que o Murillo fez, com teto **99**; depois 199, 299 e 399.
    - **O ganho por golpe** é o `ap_per_hit` da classe, que o `player_template` copia do
      `angro_increase` do `CHARRACTER_CLASS_CONFIG` (`gs/playertemplate.cpp:286`); o Arqueiro
      ganha **5** por golpe normal (`DoAttack`, `player.cpp:3091-3093`).
    - **Meditar** dá 15 por batimento de 1 s (`sit_down_filter::Heartbeat`,
      `gs/sitdown_filter.cpp:19-34`).
    - E o valor viaja no `iAP`/`iMaxAP` do `SELF_INFO_00`, que nós mandávamos **zero fixo**
      (`EC_GPDataType.h:1739-1752`).

    Faltava tudo isso. Agora o teto entra pelo prêmio, o golpe e a meditação enchem, o valor
    vai ao cliente e fica em `characters.ap`/`max_ap`. **Não há ganho ao apanhar** no 1.5.5 —
    procurei as dez chamadas de `ModifyAP` do servidor e nenhuma está no caminho de levar
    dano. O eaa foi acertado para teto 99 (a missão que ele já fez).

    ### b. O item de voo sem classe

    "Glória de Shalim" (45782, `FLYSWORD_ESSENCE`) entrou na bolsa **sem bloco de dados**. O
    cliente lê a máscara de classes de dentro do bloco (`IVTR_ESSENCE_FLYSWORD`,
    `EC_IvtrTypes.h:269-280`), então sem ele a máscara é zero e nenhuma classe pode usar.

    O conteúdo são 30 bytes (`generate_flysword`, `gs/template/generate_item_temp.h:1126-1165`):
    `cur_time` (metade do máximo), `max_time`, `require_player_level_min`, `level`, refino,
    **`character_combo_id`**, `time_increase_per_element`, `speed_increase`,
    `speed_rush_increase`, mais os 2 bytes da etiqueta de fabricante que o
    `ReadMakerInfo` consome (`EC_IvtrEquip.cpp:124-149`). Um detalhe: os campos de tempo são
    **float** no arquivo (o original faz `(int)ess->time_max_min`), e lê-los como inteiro
    devolvia zero.

    Agora o prêmio de missão gera o bloco, o `item_info` o monta para item que já esteja sem
    ele, e o item do eaa foi acertado por
    `scripts/2026_09_20_gloria_de_shalim_do_eaa.sql`.

    ### c. O teleporte: a coordenada estava no `world_targets.sev`

    O que faltava no B68 era de onde vem a posição de cada destino. Está num arquivo do
    próprio realm que ninguém lia: **`world_targets.sev`** —
    `u32 quantidade` e registros de 24 bytes `{ i32 id; i32 world_tag; f32 x,y,z; i32 ordem }`.
    No `realm_155` são 92 pontos em 2.212 bytes, fechando no último byte, e os **427 destinos
    citados pelas 95 transportadoras estão todos lá**.

    Com isso o serviço ficou igual ao original (`transmit_provider`/`transmit_executor`,
    `gs/serviceprovider.cpp:683-852`): o cliente manda só o **índice** do destino na lista
    daquele NPC; o servidor confere índice, nível (`required_level`) e dinheiro (`fee`), cobra
    com `SPEND_MONEY` e teleporta com o equivalente do `LongJump`.

    ### d. Provas

    `o_chi_vem_da_classe_e_o_teto_vem_da_missao` (5 por golpe do Arqueiro; 99, 199 e 399 nas
    missões) e `o_world_targets_fecha_e_tem_os_destinos_citados` (92 pontos, e todo destino
    citado tem coordenada). Suíte com o banco: **599 testes**; o arquivo de tempo passa 64/64
    sozinho.

Validação disponível e acordada com o usuário: Docker + clientes reais (1.2.6, 1.5.5) com
envio de logs, captura de tráfego (Wireshark/pcap) e execução dos binários originais para
comparação lado a lado.

70. **Sessão 2026-09-20 (parte 4): o teto de chi no pacote de propriedades.**

    O aviso “limite máximo de chi aumentado para 99” reaparecia porque o mundo já enviava
    `iAP = 0` e `iMaxAP = 99` no `SELF_INFO_00`, mas o `OWN_EXT_PROP` seguinte ainda fechava
    com `max_ap = 0`. Em `EC_HostMsg.cpp:1319-1332`, o cliente compara esse valor com o que
    recebeu antes e anuncia a diferença; cada atualização de propriedades virava 0→99.

    `OWN_EXT_PROP` agora recebe `max_ap` pelo `WorldProtocol`, tanto no codificador 1.2.6 de
    152 bytes quanto no 1.5.5 de 196 bytes, e o mundo passa o teto persistido do personagem.
    A **Flecha Fulgurante (244) não é a origem nem deve gerar chi**: `skill244.h:20-80,234-240`
    só consome mana e aplica `Firearrow`; não chama `SetApgen`, `SetApgen2` ou `ModifyAP`.

    ### Provas

    `layouts_do_126` e `protocol_tests`: **36/36** (o último `i32` vale 99 nos dois layouts).

71. **Sessão 2026-09-20 (parte 5): a tela de cultivo da poção, a recarga por família e a
    flecha na sessão.**

    Parte do trabalho desta entrega veio de uma sessão de fora que parou sem relatório (e
    sem commit): a **recarga das poções**, a **munição na sessão de ataque** com persistência
    fora do fio, e o `consume_item` do `pw-storage` reescrito como uma instrução só
    (`SELECT … FOR UPDATE` + `UPDATE`/`DELETE` nas CTEs), para que duas baixas simultâneas
    não regravem o mesmo restante. Auditei tudo contra o fonte e corrigi três pontos.

    **A tela de cultivo ao usar poção.** O `SELF_INFO_00` tem `Level2`, que é o cultivo, e o
    cliente chama `SetLevel2` em *todo* comando desses: `CanPlayTaoistEffect`
    (`EC_Player.cpp:7434-7445`) toca o efeito de avanço sempre que o valor novo é **maior**
    que o anterior. Três caminhos nossos mandavam `0` fixo no campo — o uso de item, o
    resultado de habilidade que cura ou fere e o serviço de cura do NPC. Com o eaa em cultivo
    1, usar uma poção zerava o cultivo do cliente e o `SELF_INFO_00` seguinte, correto,
    virava 0 → 1: avanço de cultivo em tela. É o mesmo erro do `max_ap` do B70, no campo
    vizinho. Agora todo envio leva `p.cultivation`.

    **A família da recarga.** A sessão de fora escolhia o `COOLDOWN_INDEX_*` por heurística
    (o que a poção restaura). No original quem decide é a **classe do item**, e a classe sai
    do `id_major_type` do `MEDICINE_ESSENCE` em `set_to_classid`
    (`gs/template/setclassid.cpp:81-101`): 1794 `CLS_ITEM_HEALING_POTION`, 1802
    `CLS_ITEM_MANA_POTION`, 1810 `CLS_ITEM_REJUVENATION_POTION`, 1815 e 2038 os antídotos.
    Cada `OnUse` arma o índice da sua classe (`gs/item/item_potion.cpp:18-110`). Pela
    heurística o antídoto — que não restaura vida nem mana — caía na família da mana e
    travava as poções de mana por 15 s. O `id_major_type` agora vem do arquivo
    (`tipo_maior_do_remedio`), e o `realm_155` confirma: 1796 → 1794, 1804 → 1802, 1812
    (Nove Sóis) → 1810, 1817 e 2040 → antídotos.

    **A flecha.** Ela estava sendo descontada da sessão antes das conferências do golpe
    (`pode_golpear`, efeito que impede agir, alvo ainda existir), então um golpe que nem
    acontecia comia munição. No original o desconto está dentro do `DoAttack`, depois do
    `CheckAttack` (`player.cpp:3063-3070`). E o `dec_arrow` do `ATTACK_ONCE` vale **1 sempre
    que a arma é de longe**: o retorno do `DecAmount` é ignorado (`:3068`), de modo que o
    comando não muda quando acaba a flecha — só a baixa muda.

    ### Provas

    Suíte inteira com o banco: **602 testes, 0 falhas**. O teste da poção agora fixa o
    cultivo do jogador e confere o `Level2` de cada `SELF_INFO_00` — com o zero de volta no
    caminho do uso de item ele falha, como tem de falhar. Novo
    `o_tipo_maior_do_remedio_separa_as_familias_de_recarga` lê os cinco `id_major_type` do
    `elements.data` do `realm_155`.

    ### Falta

    O relato do combate travado contra a Ninfa não foi investigado — nenhuma medição foi
    feita, e a sessão de fora não deixou registro do que tinha visto.

72. **Sessão 2026-09-20 (parte 6): o que travou o combate contra a Ninfa, e por que a Alma
    dela foi para a bolsa comum.**

    **O combate em si saiu certo.** O log do `pw-world-155` de 20:49:03 a 20:49:58 UTC tem a
    Ninfa (`-2147482351`, 5810 de vida) caindo golpe a golpe até zero, o monstro revidando,
    a recarga da habilidade 235 recusando o que tinha de recusar ("ainda em recarga"), a
    morte pela 235 e a missão avançando. Nenhum erro, nenhum pacote desconhecido.

    **O travamento é de banco no caminho do jogo.** O mesmo minuto tem **49 avisos
    `slow statement`** do `sqlx` — `UPDATE character_items SET durability` de 1,1 a 1,9 s,
    `INSERT INTO character_items` de até 4,1 s, `UPDATE characters SET ap` de 4,1 s. Três
    lugares punham essas escritas na frente do jogador:

    1. O **autosave**, quatro escritas por jogador a cada 60 s, rodava **dentro do
       `world.tick`** — e o tique roda inteiro com o `world.write()` na mão. Entre
       20:49:59 e 20:50:08 não há uma única linha de combate no log: o mundo ficou **8
       segundos parado** enquanto o autosave esperava o banco. Agora o `tick` devolve a
       fotografia (`EstadoParaGravar`) e o laço grava numa tarefa, com o lock solto.
    2. O **desgaste da peça ao apanhar** consultava o banco **antes** de mandar o
       `HOST_ATTACKED` — o índice da peça vai dentro do comando. A durabilidade passou a
       viver no mundo (`PlayerEntity::pecas`), como a `item_list` vestida do original
       (`player.cpp:94`), e o banco acompanha depois.
    3. A **munição e a durabilidade da arma** no golpe dado, já tratadas no B71.

    De quebra, isso expôs um aviso prematuro: o `SELF_INFO_00` de quem apanhou saía junto do
    `HOST_ATTACKED`, ou seja **antes** de o dano adiado vencer — o cliente recebia a vida
    velha. Quem manda a barra é o `aplicar_dano_no_jogador`, pelo `EstadoMudou`. A lentidão
    do banco é que escondia o erro.

    **A Alma da Ninfa do Mar de Conchas (44357) na bolsa comum está certa.** Quem decide o
    inventário não é o tipo do item, e sim o `m_bCommonItem` de cada item do `tasks.data`:
    `_DeliverItem` manda os `true` para `DeliverCommonItem` → `IL_INVENTORY` e os `false`
    para `DeliverTaskItem` → `IL_TASK_INVENTORY` (`task/TaskProcess.cpp:1190-1208`,
    `task/taskman.cpp:281-330`). A missão **31734 "A Ninfa do Mar de Conchas"** declara o
    item como o que o monstro 44618 solta com `comum = true`. Nosso `dar_item` já respeita o
    bit; o item entrou na bolsa às 17:49:58 local, no mesmo segundo do abate.

    ### Provas

    Suíte inteira com o banco: **604 testes, 0 falhas**. Novos:
    `o_tique_devolve_o_autosave_em_vez_de_gravar_dentro_do_lock` (o tique devolve o lote e
    **não** grava; quem grava é o `gravar_autosave`) e
    `a_durabilidade_das_pecas_vestidas_fica_no_mundo`. O
    `o_monstro_revida_e_o_cliente_fica_sabendo` passou a conferir o desgaste na memória do
    mundo e a esperar o banco — e a exigir que o `SELF_INFO_00` do golpe venha **depois** do
    dano.

    ### Falta

    Por que o PostgreSQL do contêiner chegou a 4 s por escrita naquele minuto não está
    provado — isolado, ele responde em 35 ms. A máquina estava com outra sessão compilando e
    rodando a suíte contra o mesmo banco, o que é suspeita razoável, não prova. O que foi
    corrigido é o que importa: nada que espere o banco fica mais no caminho do jogo.

73. **Sessão 2026-09-21: o chi que as habilidades cobram, o escudo da Barreira de Asa e o
    hierograma que dispara sozinho.**

    **Chi das habilidades.** Cada habilidade tem `apcost` e `apgain` fixos no stub
    (`cskill/skill/skill.h:239,588`). `SkillStub::Condition` recusa a conjuração com
    `GetAp() < apcost` (`cskill/skill/skill.cpp:125`) — sem mandar erro, porque o cliente já
    barra — e a execução aplica a diferença de uma vez:
    `int ap = GetApgain() - GetApcost(); if (ap) ModifyAP(ap)`
    (`cskill/skill/playerwrapper.cpp:170-177`). Os dois números **já estavam** no
    `habilidades.json` desde a extração; a struct `HabilidadeDoServidor` é que não os lia, e
    o mundo nunca os cobrou. Agora cobra: Flecha Glacial (245) tira 25, Barreira de Asa (249)
    tira 45, e o `SELF_INFO_00` com a barra nova sai junto, como o `SetRefreshState` do
    `ModifyAP`.

    **Correção de um erro meu (B70).** Ficou escrito que a Flecha Fulgurante (244) não gerava
    chi, porque o corpo dela não chama `ModifyAP`. Quem chama é a execução, com o `apgain` do
    stub — e o da 244 é **10** (`cskill/skills/skill244.h:142-144`). O relato do Murillo
    estava certo e a spec, errada; a §8.0 foi corrigida.

    **Barreira de Asa (249).** O log dizia `habilidade 249 — sem porte: Wingshield`: o efeito
    não existia no `crate::efeitos`, então o buff era aplicado e sumia no mesmo instante.
    `filter_Wingshield` (`cskill/skill/skillfilter.h:4136-4232`) é
    `UNIQUE|BUFF|HEARTBEAT|REMOVE_ON_DEATH|ADJUST_DAMAGE|TRANSFERABLE_BUFF`, com
    `HSTATE_WINGSHIELD` 69 e `VSTATE_WINGSHIELD` 29 (`statedef.h:40,261`). A 249 o arma com
    `SetAmount(60 + 75 × nível)`, `SetValue(4 + 6 × nível)` e `SetTime(20000)`
    (`skills/skill249.h:257-262`). O `AdjustDamage` compara **um quinto** do golpe com o
    escudo: enquanto couber, passa só esse quinto e o escudo perde quatro vezes o absorvido;
    quando não cabe, o dano é reduzido na proporção do que sobrou e o escudo zera. Abaixo de
    6 o filtro se apaga. O `Heartbeat` injeta o `SetValue` inteiro a cada 3 s.

    **Hierograma e amuleto automáticos.** Estavam anotados como `falta` desde o B67. Vesti-los
    nos slots **20** e **21** (`EQUIP_INDEX_HP_ADDON`/`MP_ADDON`, `gs/item.h:216-217`) os
    ativa (`OnActivate` → `SetHPAutoGen`/`SetMPAutoGen`, `gs/item/item_amulet.cpp:22-46`). No
    batimento de 1 s, com `trigger_percent × máximo > atual`, o `AutoGenStat`
    (`gs/player_imp.h:3562-3593`) confere a recarga (`COOLDOWN_INDEX_AUTO_HP` 24 e `AUTO_MP`
    25), devolve `máximo − atual` preso ao que resta e arma o `cool_time` do próprio item
    (10 s no Amuleto do Guardião). O que sobra é gravado nos **octetos do item**; em zero o
    amuleto sai do corpo com `PLAYER_DROP_ITEM` tipo `DROP_TYPE_USE` (11). O eaa tem os dois
    vestidos: 35370 no slot 20 e 35376 no 21.

    ### Provas

    Testes novos: `o_custo_e_o_ganho_de_chi_vem_do_stub` (os quatro números lidos do
    `habilidades.json`), `a_barreira_de_asa_absorve_dano_e_devolve_mana` (ícone, estado, as
    duas metades do `AdjustDamage` e os dois tiques de mana em 6 s),
    `o_amuleto_traz_o_total_o_gatilho_e_a_recarga` (do `elements.data` do `realm_155`) e
    `o_hierograma_vestido_devolve_mana_sozinho` (dispara, desconta do amuleto, arma a recarga
    e não repete dentro dela).

74. **Sessão 2026-09-21 (parte 2): a recarga do amuleto no cliente, e por que a Alma da
    Pantera vai mesmo para a bolsa comum.**

    **A recarga do amuleto.** O servidor já a respeitava desde o B73 (o hierograma não
    dispara duas vezes dentro do `cool_time`), mas o ícone no cliente não escurecia: faltava
    o comando. `gplayer_imp::SetCoolDown` (`gs/player.cpp:12701-12709`) grava **e sempre
    manda** `set_cooldown(idx, msec)` ao cliente; como `base_amulet::OnAutoTrigger` chama
    `SetCoolDown(cooldown_idx, cooltime)` (`gs/item/item_amulet.cpp:16-18`), o `SET_COOLDOWN`
    (198) sai a cada disparo, com `COOLDOWN_INDEX_AUTO_HP` (24) ou `AUTO_MP` (25). O evento
    `AmuletoDisparou` passou a levar o índice e o tempo, e o mundo manda o comando.

    **Item de missão na bolsa comum: não é defeito.** Quem escolhe a bolsa é o
    `m_bDropCmnItem` de cada `MONSTER_WANTED` (`gs/task/TaskTempl.inl:2028-2037`), exatamente
    como nós fazemos. Três verificações:

    1. O byte está alinhado: das 995 entradas com item, **nenhuma** tem o `m_fDropProb`
       vizinho fora de 0..1, e os valores são redondos (1,0; 0,8; 0,6). Byte deslocado
       produziria lixo nesse float.
    2. O bit não é degenerado: 797 dos itens soltos vão para a bolsa de missão e 198 para a
       comum.
    3. O bit acompanha o **tipo** do item: no `realm_155`, todos os 413
       `TASKMATTER_ESSENCE` vão para a bolsa de missão e os `TASKNORMALMATTER_ESSENCE` para
       a comum — "matéria de missão **normal**" é a que fica no inventário normal. A única
       exceção é a Presa de Filhote de Lobo (2654), e ela confirma que **o bit é a
       autoridade**, não o tipo.

    A Alma da Pantera Queimada de Sol (44363, missão 31765 "Queda do Sol") e a Alma da Ninfa
    (44357, missão 31734) são `TASKNORMALMATTER_ESSENCE` com `comum = true`: bolsa normal,
    como no original.

    ### Provas

    Teste novo `o_tipo_do_item_de_missao_decide_a_bolsa` (cruza as duas tabelas do
    `elements.data` com todos os `MONSTER_WANTED` do `tasks.data` e fixa a exceção em 1) e
    asserção do `SET_COOLDOWN` (198) com índice 25 e 10 s no
    `o_hierograma_vestido_devolve_mana_sozinho`. Exemplo novo
    `cargo run -p pw-data-loader --example bolsa_do_item_de_missao`.

75. **Sessão 2026-09-21 (parte 3): o Daimon existe agora — ficha e experiência.**

    O relato era "o Daimon não ganha experiência e aparentemente não faz nada". Ele não fazia
    mesmo: **não havia uma linha** sobre ele no projeto. O `GOBLIN_ESSENCE` não era lido, o
    item não tinha bloco de dados e nada ligava o ganho de experiência do jogador a ele. O
    Daimon do eaa ("Verão", 23752) está no slot **23** do equipamento, que é o
    `EQUIP_INDEX_ELF` do original (`gs/item.h:219`) — o cliente o pôs no lugar certo e o
    servidor o ignorava.

    **O estado do Daimon é o bloco do item.** `elf_item::Save` (`gs/item/item_elf.cpp:172-185`)
    grava, nesta ordem: o `elf_essence` de 38 bytes com `#pragma pack(1)`
    (`item_elf.h:84-102`) — experiência, nível, total de atributos, força/agilidade/
    vitalidade/energia, total de gênios, os 5 gênios, refino, vigor e estado —, a lista de
    equipamento e a de habilidades, cada uma precedida da contagem em 4 bytes. Um Daimon novo
    sai de `generate_elf` (`gs/template/generate_item_temp.h:2442-2524`) com nível 1, um ponto
    de gênio, **20000** de vigor e as `default_skill1..3` do `GOBLIN_ESSENCE` no nível 1.

    **A experiência é um décimo da do jogador**, toda vez que ele ganha:
    `ElfReceiveExp(exp / 10)` (`gs/player.cpp:2921-2928`, `player_imp.h:2471`). O `InsertExp`
    (`item_elf.cpp:692-750`) aplica um fator de obtenção que é a razão dos níveis —
    `elf_exp_loss_constant[i]` é a identidade, então o fator é `nível do Daimon ÷ nível de
    quem deu`, com o mínimo de 10 % —, sobe **quantos níveis couberem** no laço e, ao
    alcançar o nível do dono, para a um ponto de subir. Cada nível dá um ponto de atributo e,
    a cada cinco até o 100, um de gênio (`LevelUp`, `:775-816`). Sem subir de nível o cliente
    recebe `ELF_EXP` (283, 4 bytes); subindo, a ficha inteira do item.

    Como o amuleto do B73, o Daimon vive em memória (`PlayerEntity::daimon`), preenchido pelo
    `recalcular_equipamento`, e o bloco vai ao banco no fim do `com_contexto`.

    ### Provas

    Arquivo novo `crates/pw-gs/tests/daimon.rs`, 3 testes: o bloco do Daimon novo campo a
    campo contra o gerador (e a releitura idempotente), o laço da experiência (1/5 de 50 vira
    10; 1000 de experiência leva do nível 1 ao 5 com 8 de sobra, 4 pontos de atributo e 1
    gênio; no teto para em 99 e não ganha mais; Daimon acima do dono não recebe nada) e o
    `GOBLIN_ESSENCE` do `realm_155`. O `ELF_EXP` passou pela guarda que confere os
    codificadores contra o IR.

    ### Falta

    O bônus de atributo sorteado de 10 em 10 níveis (`rand_prop` com `abase::RandSelect`,
    `item_elf.cpp:779-796`: a semântica do sorteio não está medida), o equipamento e as
    habilidades do Daimon, o vigor, as pílulas de experiência, a decomposição, o refino e a
    distribuição de pontos.

76. **Sessão 2026-09-21 (parte 4): a flor que acorda um monstro, os monstros que não
    atacavam, e por que o Daimon não ganha experiência.**

    Três relatos do teste em jogo, três causas distintas — e uma delas não era defeito.

    **A missão da Flor de Safira (31779).** O Murillo colheu a flor muitas vezes e a missão
    não andou. A mina 44566 do `realm_155` tem `materials_1_id = 0`: ela **não produz item
    nenhum**. O que ela tem é `npcgen_1_id_monster = 44608`, `num = 1`, `life_time = 30` — os
    `npcgen_1..4` do `MINE_ESSENCE`. Colher a flor **acorda o Guardião de Almas**, um monstro
    de nível 16 que vive 30 s, e é dele que cai o Estame (44371) com 80 %, pelo
    `MONSTER_WANTED` da missão. A descrição da missão já dizia: "um dos Guardiões de Almas é
    uma Flor de Safira aparentemente inofensiva". Nosso leitor de minas ignorava esses
    campos, então colher não fazia nada. De passagem, confirmei que o `CheckMining`
    (`gs/task/TaskTempl.inl:2105-2145`) só entrega item quando o método da missão é "coletar
    N itens", que não é o caso desta — o nosso `colheu_mina` já fazia a mesma recusa, e está
    certo.

    **Nenhum monstro atacava sozinho.** O campo existe e nós já o líamos sem usar:
    `aggressive_mode` do `MONSTER_ESSENCE` — **4.874 dos 8.054** monstros do realm são
    agressivos, incluindo o Guardião de Almas. No original o monstro agressivo recebe a marca
    `MSG_MASK_PLAYER_MOVE` (`npcgenerator.cpp:2534-2537`) e o jogador, ao andar, difunde
    `GM_MSG_WATCHING_YOU` para quem está a até `GetMaxMobSightRange` — **15 m**
    (`playerctrl.cpp:265-276`, `worldmanager.cpp:48`). A decisão final é da política do
    `aipolicy.data`, que ainda não interpretamos; aqui o monstro agressivo sem alvo passa a
    pegar o jogador vivo mais perto dentro dos 15 m.

    **O Daimon está ligado — o ganho é que trunca para zero.** O bloco dele foi criado no
    primeiro abate (15:28:33 de hoje, no banco), então o B75 está no ar e funcionando. O que
    acontece é aritmética do original: o fator de obtenção é a razão dos níveis com piso de
    10 % (`GetExpObtainFactor`, `item_elf.cpp:754-773`) e o resultado é truncado para inteiro
    (`:715`). Com o Daimon no nível 1 e o dono no 16, um monstro de **80** de experiência dá
    `80 / 10 × 0,1 = 0,8` → **zero**. Só a partir de 100 de experiência por monstro entra o
    primeiro ponto. Não há o que corrigir: é o que o original faz. O caminho rápido no jogo
    são as pílulas de experiência, que seguem sem porte.

    ### Provas

    Suíte com o banco: **615 testes, 0 falhas**. Uma rodada intermediária acusou uma falha
    que não se repetiu com `--no-fail-fast` nem nas duas execuções seguintes — é o teste de
    tempo sob carga já conhecido, não uma regressão. Novos:
    `a_mina_da_flor_de_safira_acorda_o_guardiao` (a mina não produz material, aponta para a
    missão 31779 e acorda o 44608 por 30 s; e o 44608 é agressivo),
    `o_monstro_agressivo_ataca_quem_chega_perto` (passivo não odeia sozinho; agressivo pega a
    5 m, não pega a 20 m, e morto não pega nada) e
    `o_ganho_trunca_para_zero_com_pouca_experiencia`.

77. **Sessão 2026-09-22: o modo de combate, o Atq. Mágico da ficha e o monstro de missão que
    voltava.**

    Cinco relatos do teste com o Tormentador "RT". Três tinham correção, um é resposta e um
    é sistema inteiro a fazer.

    **O personagem nunca entrava em modo de combate.** O `State` do `SELF_INFO_00` (38) é o
    modo de luta: o cliente faz `if (pCmd->State && m_bFight == false) PlayEnterBattleGfx();
    m_bFight = pCmd->State ? true : false` (`EC_HostMsg.cpp:1334-1335`), e é ele que troca a
    animação. Nós mandávamos **zero fixo**. O original manda `IsCombatState() ? 1 : 0`
    (`gs/player.cpp:3570`), e o mesmo vale para o `PLAYER_INFO_00` (32), que é como os outros
    jogadores veem a postura (`:3554`). Agora os dois levam `combate_s > 0`, o contador que já
    existia (15 s ao atacar, 5 s ao apanhar).

    **A ficha não mostrava Atq. Mágico.** No `OWN_EXT_PROP` (50), `damage_magic_low/high`
    fecham o `ROLEEXTPROP_ATK` e as cinco `resistance[]` abrem o `ROLEEXTPROP_DEF` — os sete
    campos iam zero fixo. O jogador já tinha os números calculados
    (`UpdateMagic`, `entity.rs:575-580`), só não eram enviados. Um Tormentador com arma mágica
    via o campo vazio.

    **Monstro de missão renascia.** `matar_monstro` fazia
    `respawn_timer_ms = respawn_delay_ms.max(1)`, e o invocado — que tem `respawn_delay_ms`
    zero justamente por **não ter gerador** — voltava 1 ms depois de o corpo sumir. Era a
    Sombra do Olho do Deus (44595) da missão 31728 "Surgem as Sombras". Zero agora quer dizer
    nunca. Junto vieram duas coisas do mesmo trecho do original
    (`gplayer_imp::SummonMonster`, `gs/player.cpp:13072-13110`): o invocado nasce **odiando
    quem o chamou** (`GM_MSG_GEN_AGGRO` com 10000) e **some quando o `remain_time` acaba** —
    a missão pede 60 s.

    **Marcar sozinho quem me ataca: não é do original.** Procurei no cliente. `OnMsgHstAttacked`
    (`EC_HostMsg.cpp:968-1008`) toca o efeito, vira o atacante de frente e avisa a
    `CECAutoPolicy`; `SendEvent_BeHurt` (`EC_AutoPolicy.cpp:261-270`) chama o script Lua do
    assistente e, só ali, `OnMonsterAttackMe` (`EC_PlayerWrapper.cpp:1237-1247`) **anota** o
    monstro numa lista para o modo automático usar. Nada seleciona o alvo. Se quisermos esse
    conforto, é decisão nossa, fora do original.

    **Montaria: `falta` inteiro.** Incubar funciona (B67), invocar não existe. O caminho é
    `SUMMON_PET` (C2S 100, `{ size_t pet_index }`) → `SUMMON_PET` (S2C 233,
    `{ int slot_index; int pet_tid; int pet_pid; int life_time }`), mais `RECALL_PET`
    (101/234), `BANISH_PET` (102) e `PET_CTRL` (103). Anotado na fila.

    ### Provas

    Suíte com o banco: **616 testes, 0 falhas**. Novos:
    `o_monstro_invocado_nao_renasce_e_expira` (o invocado morto não volta e o de gerador
    volta) e, nos testes de protocolo, o `State` em 1 e 0 e os sete campos do Atq.
    Mágico/resistências no `OWN_EXT_PROP`.

78. **Sessão 2026-09-22 (parte 2): conjurar andando, a transformação que falta e a montaria.**

    **Conjurar andando é da habilidade, não da classe.** O stub tem `is_movingcast`
    (`cskill/skill/skill.h:382`); com ele, `playercmd.cpp:2066-2088` despacha a conjuração
    por `moving_skill` em vez de `session_skill`, e o movimento **não** a interrompe — só o
    `moving_skill_interrupt_filter` a encerra. No 1.5.5 são **cinco** habilidades: 2909,
    2910, 2913, 2914 e 2917, **todas da classe 11** (a do Tormentador do Murillo; o RT é
    `cls = 11` no banco). Nenhuma classe antiga tem nenhuma. O campo passou a ser extraído
    para o `habilidades.json` e o nosso `PLAYER_MOVE` deixou de cortar a conjuração quando a
    habilidade é dessas.

    **A transformação: diagnosticada, não corrigida.** A habilidade **2570** (胧) da classe 11
    chama `SetFairyform(1)`, e o `filter_Fairyform` (`cskill/skill/skillfilter.h:16819-16880`)
    faz `ChangeShape(1|(FORM_CLASS<<6))`, acende `HSTATE_FAIRYFORM` e aumenta a velocidade.
    O `ChangeShape` (`gs/actobject.h:1047-1058`) grava `shape_form` e liga o bit
    `STATE_SHAPE` do `object_state` — e é esse bloco condicional do estado estendido
    (`common/protocol_imp.h:62-80`) que leva a forma ao cliente. **Não mandamos `object_state`
    estendido de jogador nenhum**, então a transformação não teria como aparecer mesmo com o
    filtro portado. Fica anotado junto: primeiro o estado estendido, depois o filtro.

    **Montaria.** Invocar um mascote de montaria é montar nela:
    `PlayerSummonPet` → `pet_man::ActivePet` (`gs/player.cpp:14474-14491`,
    `gs/petman.cpp:319-392`) confere o estado, calcula
    `speed_a + speed_b × (nível − 1)` do `PET_ESSENCE` (`petdataman.h:186-194`) e põe o
    `mount_filter`, que liga o `STATE_MOUNT`, manda `PLAYER_MOUNTING` (227) e **sobrepõe** a
    velocidade de corrida (`mount_filter.cpp:24-45`, `player.cpp:14279-14319`). Desmontar
    (`RECALL_PET`, C2S 101) manda o mesmo comando com zero nos dois campos. Implementei os
    dois comandos, a leitura do bloco do mascote (o inverso do que o incubar grava), a
    velocidade pelo `PET_ESSENCE` e a ficha nova ao cliente.

    ### Provas

    Suíte com o banco: **618 testes, 0 falhas** — medida depois que o Murillo subiu o Docker
    de volta (ele caiu no meio do trabalho e por um tempo todos os alvos com banco morriam em
    `PoolTimedOut`). Novos:
    `so_cinco_habilidades_conjuram_andando_e_sao_da_classe_11` e
    `montar_muda_a_velocidade_e_avisa_o_cliente` (monta, confere o `PLAYER_MOUNTING` com id,
    montaria e cor, a velocidade nova no mundo, e desmonta com zero). O `player_mounting`
    passou pela guarda dos codificadores contra o IR.

    ### Falta

    Mascote de **combate** (invocar a criatura no mundo, que é outro caminho do mesmo
    comando), a trava de ataque enquanto montado, água/invisibilidade/transformação como
    recusa, a queda da montaria por lealdade — e o estado estendido do jogador, que é o que
    falta para a transformação aparecer.

79. **Sessão 2026-09-22 (parte 3): a montaria travava porque invocar mascote é uma sessão.**

    **O relato.** O Murillo testou a montaria do B78 com o RT: "invoquei a montaria, não
    apareceu canalização e só montou. Porém não consigo mais desmontar, aparece 'mascote
    está em processo de convocação'. Pela jaula só consigo clicar no botão Inv. e dá a mesma
    mensagem, como se o estado de invocação não tivesse ficado correto."

    **A causa, no cliente.** O botão "Rec." da jaula só liga quando o slot é o do mascote
    ativo: `bEnable = (pPetCorral->GetActivePetIndex() == nPetSlot && IsOperatingPet() == 0)`
    (`DlgPetList.cpp:227-230`). Quem preenche esse índice é o S2C **`SUMMON_PET` (233)**
    (`SetActivePetIndex(pCmd->slot_index)`, `EC_HostMsg.cpp:5274-5296`) — que nós não
    mandávamos. Com o índice em −1 o cliente ficava com a montaria debaixo do personagem e
    sem nenhum mascote ativo: "Rec." apagado, e "Inv." mandando `SUMMON_PET` de novo, que o
    nosso servidor recusava com `ERR_PET_IS_ALEARY_ACTIVE` (71) — a mensagem que ele leu.

    **A causa, no servidor.** Invocar **não é um comando, é uma sessão**. O
    `gplayer_imp::PlayerSummonPet` (`gs/player.cpp:14474-14491`) só confere que o mascote
    existe e abre uma `session_summon_pet` com `SetDelay(60)`; o resto é o ciclo da sessão
    (`gs/actsession.cpp:1705-1721`):

    - `OnStart` → `start_pet_operation(index, tid, delay, op)` = **`PLAYER_START_PET_OP`
      (235)**. O cliente cria o `WORK_CONCENTRATE` e conta `delay × 50 ms`
      (`EC_HostMsg.cpp:5335-5356`) — a canalização que faltava. A unidade é o tick do
      original (`TICK_PER_SEC 20`, `gs/config.h:43`), então 60 ticks são 3 s.
    - `OnRepeat`, depois do atraso → `SummonPet` → `pet_manager::ActivePet`
      (`gs/petman.cpp:1303-1348`): o `mount_filter` manda `PLAYER_MOUNTING` (227) e sobrepõe
      a velocidade, e **em seguida** vai o `summon_pet(index, pet_tid, 0, 0)` (`:1337`).
    - `OnEnd` → `end_pet_operation()` = **`PLAYER_STOP_PET_OP` (236)**, sem corpo. Sem ele o
      `IsOperatingPet()` do cliente (`EC_HostPlayer.cpp:8661-8680`) nunca volta a zero e o
      personagem recusa atacar, conjurar e invocar.

    Recolher é a mesma coisa com `session_recall_pet`, `SetDelay(10)` (0,5 s) e `op` 1
    (`gs/player.cpp:14492-14512`), e termina com `PLAYER_MOUNTING(0, 0)` seguido de
    **`RECALL_PET` (234)** com `PET_RECALL_DEFAULT` = 0 (`gs/petman.cpp:1376`).

    **Um detalhe do original que tínhamos errado:** `PlayerSummonPet` deixa **comentada** a
    recusa por mascote já ativo (`// if(_petman.IsPetActive()) return ERR_PET_IS_ALEARY_ACTIVE`,
    `player.cpp:14476`). Quem já tem um mascote não é recusado: o `ActivePet` recolhe o
    anterior e põe o novo (`petman.cpp:1308-1312`). Nós recusávamos — e era justamente o erro
    que aparecia na tela. A conferência de montaria (chão, voo, `CalcMountParam`) fica onde o
    original a tem: **depois** da canalização, dentro do `DoActivePet`.

    **Layouts** (`common/protocol.h:2725-2753`, escritos sem enchimento pelo `<<`; conferidos
    contra o IR do 1.5.5):

    | comando | id | corpo |
    | :--- | ---: | :--- |
    | `SUMMON_PET` | 233 | `int slot_index; int pet_tid; int pet_pid; int life_time` (16 B) |
    | `RECALL_PET` | 234 | `int slot_index; int pet_id; char reason` (**9 B**, não 12) |
    | `PLAYER_START_PET_OP` | 235 | `int slot_index; int pet_id; int delay; int op` (16 B) |
    | `PLAYER_STOP_PET_OP` | 236 | vazio |

    O `pet_tid` do `SUMMON_PET` é o do bloco do mascote, não o `pet_vis_tid`: o cliente o
    confere contra a jaula (`ASSERT(pPet->GetTemplateID() == pCmd->pet_tid)`,
    `EC_HostMsg.cpp:5278`). O `pet_vis_tid` é o que vai no `PLAYER_MOUNTING` e no
    `START_PET_OP`, porque é o modelo (`player.cpp:14483-14486`).

    **O que mudou aqui.** Os quatro codificadores novos em `s2c.rs`; `MontariaAtiva` no
    `PlayerEntity` (agora com o **slot da jaula** e o `pet_tid`, que é o que volta ao
    cliente) e um contador `operacao_de_pet` que faz um pedido novo invalidar a canalização
    aberta, como o `AddSession` do original; `invocar_mascote`/`recolher_mascote` reescritos
    como sessão, com o efeito em `montar`/`desmontar` numa tarefa adiada — o mesmo desenho da
    conjuração de habilidade, e nada bloqueando o fio do barramento por 3 s.

    ### Provas

    `montar_muda_a_velocidade_e_avisa_o_cliente` passou a cobrir a sequência inteira: 235 com
    `delay` 60 e `op` 0, nada de montaria no mundo até a canalização acabar, 227 com id,
    modelo e cor, **233 com o slot e o `pet_tid`**, 236; depois 235 com `delay` 10 e `op` 1,
    227 zerado, 234 com 11 bytes e motivo 0, 236. O teste leva 6 s de propósito — são os 3 s
    do original. Os quatro codificadores passaram pela guarda contra o IR
    (`cada_codificador_escreve_o_id_que_o_ir_da_ao_comando`), que confirmou inclusive os 9
    bytes do `RECALL_PET`.

    `cargo test -p pw-gs -p pw-protocol`: tudo passa. O
    `aceitar_forma_o_grupo_e_avisa_os_dois_com_dados_reais` falhou uma vez na rodada cheia e
    passa isolado — é a instabilidade sob carga já registrada no B74, agora mais provável
    porque o teste da montaria segura o binário por 6 s.

    ### Falta

    Mascote de **combate** (o outro caminho do mesmo comando), a trava de ataque montado,
    água/invisibilidade/transformação como recusa, a queda por lealdade, e o
    `mount_id`/`mount_color` no bloco de AOI do jogador (`gs/player.h:94`) — hoje quem entra
    no campo de visão **depois** não vê a montaria, porque só o `PLAYER_MOUNTING` do momento
    é transmitido.

80. **Sessão 2026-09-22 (parte 4): o estado estendido do jogador — o bloco que faltava.**

    **O problema.** O `PLAYER_MOUNTING` (227) só alcança quem já está no campo de visão na
    hora em que alguém monta. Quem chega depois recebe o `PLAYER_ENTER_SLICE` (12), e nele
    nós mandávamos o `state` com **um único bit**, o de GM. Resultado: um jogador montado,
    voando, morto ou de moda aparecia a pé, no chão, vivo e de armadura para todo mundo que
    se aproximasse dele depois. Era também o alicerce que faltava para a transformação
    (B78).

    **Como o comando funciona.** O `info_player_1` tem 30 bytes fixos e, depois deles,
    campos **opcionais governados pelos bits do `state`**. O cliente calcula o tamanho
    esperado somando esses campos (`info_player_1::CheckValid`, `EC_GPDataType.h:624-710`) —
    ou seja, **o estado decide o tamanho do pacote**, e errar a conta cai na regra de sempre:
    descarte em silêncio. O servidor escreve os campos em ordem fixa
    (`MakePlayerExtendState`, `common/protocol_imp.h:62-180`); os dois lados batem campo a
    campo, e os valores dos bits do servidor (`gs/object.h:143-180`) são os mesmos
    `GP_STATE_*` do cliente (`EC_GPDataType.h:198-234`).

    **O que passou a viajar** (o resto segue em `falta`, com bit zero):

    | bit | valor | campo extra |
    | :--- | :--- | :--- |
    | `FORMA` (`STATE_SHAPE`) | `0x1` | 1 byte, `shape_form` |
    | `VOO` | `0x10` | nenhum — o cliente põe o avatar em `MOVEENV_AIR` |
    | `CADAVER` (`STATE_ZOMBIE`) | `0x80` | nenhum |
    | `MODA` (`STATE_FASHION_MODE`) | `0x2000` | nenhum |
    | `MESTRE_DO_JOGO` | `0x4000` | nenhum (já ia) |
    | `MONTADO` (`STATE_MOUNT`) | `0x80000` | **6 bytes**: `u16 mount_color`, depois `int mount_id` |

    Atenção ao `MONTADO`: o comentário da struct do cliente diz "1 char + 1 int", mas o
    código — dos dois lados — é `unsigned short` + `int` (`CheckValid`,
    `EC_GPDataType.h:663-667`; `EC_ElsePlayer.cpp:445-455`; `protocol_imp.h:105-109`). São 6
    bytes, não 5. O comentário é o que estava errado.

    A `VistaDoJogador` ganhou `voando`, `morto`, `modo_roupa`, `montaria` e `forma`, e o
    `PlayerEntity::vista()` os preenche do mundo. O `forma` fica `None` até alguém portar o
    `filter_Fairyform` — mas o campo e a escrita já estão no lugar certo da sequência, que é
    a parte fácil de errar depois.

    O 1.2.6 não é tocado: ele tem caminho próprio (`info_player_1_126`, 28 bytes de corpo,
    sem `state2`).

    ### Provas

    `os_bits_do_estado_acrescentam_os_campos_que_o_cliente_espera` (pw-protocol) fixa a
    conta: 32 bytes com o comando vazio, os mesmos 32 com voo + cadáver + moda + GM ligados,
    38 com montaria (cor antes do modelo) e 39 com forma **mais** montaria — nessa ordem, que
    é a do original. `a_vista_de_quem_esta_montado_leva_a_montaria` (pw-gs) prova que o
    estado do mundo chega até lá.

    Suíte com o banco: **620 testes, 0 falhas** (618 + os 2 deste item). Com a suíte inteira
    em paralelo máximo, `aceitar_forma_o_grupo…` e `a_consulta_de_jogador…` falham de vez em
    quando e passam isoladas — a instabilidade sob carga do B74. Com `--test-threads=2` a
    rodada inteira passa, e é assim que vale a pena medir daqui em diante.

    ### Falta

    Os bits que continuam com valor zero por não termos o dado: `EMOTE`, `EXTEND_PROPERTY`
    (as seis DWORD de estado de habilidade), `MAFIA`, `MARKET` (barraca), `EFFECT` (a lista
    de efeitos visíveis), `PARIAH`, `IN_BIND`, `SPOUSE`, `EQUIPDISABLED`, `PLAYERFORCE`,
    `MULTIOBJ_EFFECT`, `COUNTRY`, e os sete do `state2` (título, renascimento, reino, PvP de
    facção, MnFaction, VIP, tamanho do corpo). O `self_info_1` (8) do próprio jogador também
    tem bloco estendido no original e continua indo só com o bit de GM.

81. **Sessão 2026-09-22 (parte 5): economia de contexto virou regra escrita.**

    Pedido do Murillo: gravar nas specs e no agente as definições que vinham sendo usadas
    na prática nesta sessão — não despejar saída grande no contexto, e relatar o consumo de
    tokens por etapa.

    **Por que importa.** A janela de contexto é o limite real de uma sessão longa, e quem a
    gasta é saída de ferramenta que ninguém lê. Um `cargo test --workspace` inteiro custa
    mais de dez mil tokens para entregar uma informação binária. Nesta sessão a mesma
    rodada custou menos de mil, filtrando com `grep -E "^test result:"` e somando com `awk`.

    **Onde ficou escrito:**

    - `.claude/agents/pw-server-dev.md`, seção nova "Contexto é recurso: nunca despeje saída
      grande, e diga quanto gastou" — a regra, os padrões de filtro, o uso de segundo plano,
      e o formato da tabela de consumo por etapa.
    - `.claude/skills/pw-testar-e-publicar/SKILL.md` — os comandos já com o filtro certo:
      suíte com `--test-threads=2` somada por `awk`, lista de falhas sem rastro de pânico,
      `docker compose build ... | tail -5`. Referência atualizada para **620 testes**.
    - `.claude/skills/pw-retomar-sessao/SKILL.md` — anotar o total de tokens restantes no
      primeiro resultado de ferramenta da sessão; é o marcador zero da medição.
    - `specs/00_MASTER_SPECIFICATION.md`, princípio **7** — "Contexto é recurso", ao lado dos
      outros seis.
    - `CLAUDE.md` — uma linha nas regras que não se negociam.

    Nenhum código mudou.

82. **Sessão 2026-09-22 (parte 6): as 24 habilidades que conjuram andando — eu tinha contado cinco.**

    **O relato.** "O combate ainda cancela as 2 skills principais: Explosão Sônica e
    Ruptura Descendente. Essas skills não deveriam continuar canalizando mesmo quando
    movimentar?"

    **Quais são.** O RT (id 11456, classe 11, nível 10) tem quatro habilidades no banco:
    167, **2571** (nível 3), **2579** (nível 2) e 2570 (a transformação). As duas de ataque
    são as 2571 e 2579.

    **A causa: o extrator, não o servidor.** Os dois stubs têm `is_movingcast = true`
    (`cskill/skills/skill2571.h:176`, `skill2579.h:176`). O meu extrator do B78 lia o campo
    com `([-\\d.]+)` — **um padrão que só casa número** — e os stubs escrevem o mesmo campo
    de dois jeitos:

    ```
    5 stubs:  is_movingcast = 1;
    19 stubs: is_movingcast = true;
    ```

    Os 19 saíam com o campo `null` no `habilidades.json`, e `conjura_andando()` respondia
    "não". Daí a conclusão errada do B78 — "são cinco, todas da classe 11" —, que era certa
    na segunda metade e errada na primeira: **são 24**, e continuam todas da classe 11.
    Entre as que faltavam estavam justamente as duas primeiras habilidades de ataque da
    classe, que é o que um Tormentador de nível 10 tem na barra.

    O `escalar()` passou a aceitar `true`/`false`, o `habilidades.json` foi regerado e o
    teste virou `as_habilidades_que_conjuram_andando_sao_as_24_da_classe_11`, que agora
    exige a 2571 e a 2579 na lista pelo nome.

    **Conferido que nada mais mudou:** comparando o JSON antigo com o novo, campo a campo,
    o **único** campo alterado foi `is_movingcast` (3.311 habilidades, de `null` para 0 ou
    1). Nenhum outro campo booleano passa pelo `escalar()`, então não houve efeito colateral
    em recarga, custo de mana ou `notuse_in_combat`.

    ### Lição

    Um extrator que devolve `None` para um campo que existe é pior do que um que falha: o
    valor ausente vira "não" silencioso e a conclusão errada entra na spec com ar de fato
    medido. Extrator novo de campo booleano confere **quantos stubs têm o campo** contra
    **quantos foram extraídos** — se a conta não bate, o padrão está errado.


83. **Sessão 2026-09-22 (parte 7): o modo roupa tem de sobreviver ao logout.**

    **O relato.** "Alterei meu personagem para modo roupa e não equipamento no jogo. Quando
    voltei à tela de seleção de personagem isso não ficou salvo (continua mostrando
    equipamento)."

    **Como o original guarda.** Em jogo o estado é um bit do `object_state`
    (`STATE_FASHION_MODE`, `gs/object.h:160`), mas o que vai ao banco é um **blob de pares
    de `int32`**: `GetPlayerCharMode` escreve `(PLAYER_CHAR_MODE_FASHION = 1, 1)` quando o
    bit está ligado e **não escreve nada** quando está desligado; `SetPlayerCharMode` relê
    no login (`gs/player.cpp:12585-12612`, chamados em `gs/userlogin.cpp:161` e `:738`). O
    campo do registro do personagem chama-se `charactermode`.

    **Como a tela de seleção descobre.** O mesmo blob viaja no `RoleInfo` da lista de
    personagens, e `CECLoginPlayer::Load` o lê ali: `iNumMode = charactermode.size() / 8`,
    e a chave 1 vira `m_bFashionMode` (`EC_LoginPlayer.cpp:172-189`). **O campo já existia
    no nosso `RoleInfo` e nós mandávamos vazio** — por isso o avatar voltava de armadura.

    **O que mudou.** Quatro camadas, uma por vez:

    | camada | mudança |
    | :--- | :--- |
    | banco | coluna `characters.character_mode BYTEA` (`scripts/2026_09_22_modo_roupa.sql`, aplicada em `public` **e** em `test`) |
    | `pw-core` | `charactermode_de_modo_roupa` / `modo_roupa_do_charactermode` — o formato num lugar só |
    | `pw-link` | `write_role_info` manda o blob cru no lugar do `&[]` |
    | `pw-gs` | `SWITCH_FASHION_MODE` grava (em `tokio::spawn`, fora do fio do jogo), e o login carrega para `PlayerEntity::modo_roupa` |

    Guardamos o **blob**, e não um booleano, porque é o que o protocolo manda cru — zero
    conversão no caminho quente — e porque outra chave de modo entra sem migração nova.

    ### Provas

    `o_botao_de_roupa_alterna_e_avisa_os_dois_lados` ganhou a ida e volta completa: depois
    da terceira troca, `get_details_por_role` — **o mesmo caminho que o login usa** —
    responde `modo_roupa = true`. `o_charactermode_do_role_info_leva_o_modo_roupa` fixa o
    formato (`[1,0,0,0, 1,0,0,0]`), a regra de não escrever nada quando desligado, e que o
    `RoleInfo` cresce exatamente 8 bytes com o par.

    Suíte com o banco: **621 testes, 0 falhas** (`--test-threads=2`).

    ### Falta

    O bit `MODA` do `object_state` (B80) e este estado agora concordam, mas quem estava
    vendo o jogador antes do login continua sabendo pelo `PLAYER_ENABLE_FASHION` (192) — não
    há nada a fazer aí. O `custom_status` do `RoleInfo` (a lista de efeitos visíveis na
    seleção) segue indo vazio.

84. **Sessão 2026-09-22 (parte 8): o item que ficava apagado na bolsa — e a regra do congelamento.**

    **O relato.** "Ao receber um novo amuleto, pediu para substituir os meus, cliquei em sim
    e, em vez de desaparecer, o antigo foi parar no inventário. Tentei remover do inventário
    e o item ficou bugado (apagado como se estivesse bloqueado)."

    **A primeira metade não é defeito.** Equipar sobre um slot ocupado é uma **troca**:
    `gplayer_imp::EquipItem` põe o novo no corpo e devolve o antigo ao slot da bolsa de onde
    o novo saiu (`gs/player.cpp:8069-8247`), e o cliente faz o mesmo com `PutItem`/`SetItem`
    (`EC_HostMsg.cpp:1881-1985`). O antigo **deve** ir para o inventário.

    **A segunda é, e a causa vale como regra geral.** O cliente **congela o slot antes de
    mandar** todo comando de item:

    ```cpp
    void CECGameSession::c2s_CmdDropIvtrItem(int iIndex, int iAmount)
    {
        FreezeHostItem(IVTRTYPE_PACK, iIndex, true);
        ::c2s_SendCmdDropIvtrItem(iIndex, iAmount);
    }
    ```

    (`Network/EC_GameSession.cpp:6318-6322`; o mesmo em mover, trocar, equipar e
    desequipar, `:6304-6390`.) O congelamento é do **objeto do item**
    (`pItem->NetFreeze(true)`, `EC_HostPlayer.cpp:7792-7808`), e no cliente inteiro existem
    **dois** lugares que o limpam: o fim de uma troca entre jogadores e o comando
    **`UNFREEZE_IVTR_SLOT` (181)** (`EC_HostMsg.cpp:2060-2064`). Item congelado não pode ser
    movido, usado nem equipado, e é desenhado apagado.

    Nós **não tratávamos os comandos 14 (`DROP_IVTR_ITEM`) e 15 (`DROP_EQUIP_ITEM`)**. Eles
    caíam no `outro =>` silencioso do `match`: o jogador tentava jogar o amuleto fora, o
    cliente congelava o slot, e nada voltava. O item ficava apagado até o relogue.

    **Como o original se protege.** Não é caso a caso: o `gplayer_controller` tem um
    `UnLockInventoryHandler` (`gs/playercmd.cpp:183-230`) que destrava os slots de um
    comando de item, e ele é chamado sempre que o comando **não vai ser executado** — por
    recusa, ou porque o jogador está num estado que não permite (a lista de `case`s em
    `:654-678`, do tratador de morto). São 115 chamadas de `unlock_inventory_slot` no gs.

    > Nota de busca: procurar "unfreeze" no servidor não acha nada — do lado do servidor o
    > comando se chama `unlock_inventory_slot` (`gs/player.cpp:5059-5066`). Os dois nomes
    > são o mesmo comando, o 181.

    **O que mudou.**

    1. **Descarte implementado** (14 e 15): tira do banco (ou reduz o monte), cria o item no
       chão **sem dono** — item que se joga fora é de quem pegar (`DropItemFromData` com
       `XID(0,0)`, `ThrowEquipItem`, `gs/player.cpp:7932-7980`) — e responde
       `PLAYER_DROP_ITEM` (46) com `DROP_TYPE_PLAYER` = 1, mais o destrave.
    2. **A rede de segurança do original**: o ramo `outro =>` passou a destravar os slots que
       o comando congela, pela tabela do `EC_GameSession`. Protege todo comando de item que
       ainda falte.
    3. Todo caminho de desistência do descarte destrava antes de sair — slot vazio, falha de
       banco, jogador fora do mundo.

    ### Provas

    `descartar_item_joga_no_chao_e_destrava_o_slot`: descarta 2 de um monte de 3, confere o
    `PLAYER_DROP_ITEM` campo a campo (pacote, slot, quantos, tid, `DROP_TYPE_PLAYER`), o
    `UNFREEZE_IVTR_SLOT` do mesmo slot, que sobrou 1 na bolsa e que os 2 estão no chão.
    `descartar_slot_vazio_ainda_destrava`: o caminho de desistência também devolve o 181.

    ### Falta

    Os comandos de **armazém** (`trashbox`) também congelam slots no cliente e não são
    tratados aqui; eles não estão na tabela do destrave porque os payloads ainda não foram
    conferidos contra o IR. Enquanto isso, mexer no armazém apaga o slot até o relogue.


85. **Montaria na água: diagnosticada, falta o dado do mapa.**

    **O relato.** "Está sendo possível invocar montaria terrestre embaixo d'água."

    **O original recusa, em dois momentos.** No momento de montar, `mount_petdata_imp::DoActivePet`
    testa `pImp->IsUnderWater()` e responde `ERR_PET_CAN_NOT_MOUNT` (81) — é a mesma sequência
    de recusas que já portamos para chão e voo (`gs/petman.cpp:319-392`). E **depois** de
    montado: `ActivePet` liga `pMan->SetTestUnderWater(true)`, e o `pet_manager::TestUnderWater`
    derruba a montaria de quem entra na água (`gs/petman.cpp:1390+`).

    **O que falta é o mapa de água.** `IsUnderWater` é o `_under_water` do `breath_ctrl`
    (`gs/breath_ctrl.h:38-46`), ligado pelo `gplayer_imp::TestUnderWater`
    (`gs/player.cpp:14322+`), que compara a altura do jogador com
    `path_finding::GetWaterHeight(_plane, x, z)`.

    > **Correção de 2026-09-22, no mesmo dia (B87).** Escrevi aqui que "não temos altura de
    > água" porque olhei só a pasta `map/` de um mapa e vi apenas o `.hmap`. **O dado está no
    > realm**: cada mapa tem uma pasta `watermap/` com `watermap.conf` e `N.wmap`, que é
    > exatamente o que o `CGlobalWaterAreaMap::Load` lê
    > (`gs/pathfinding/GlobalWaterAreaMap.cpp:75-120`). São 75 mapas com o arquivo. Afirmar
    > falta de dado sem varrer a pasta inteira é o mesmo erro do extrator do B82: uma busca
    > estreita vira conclusão larga.

    O formato é pequeno (`CWaterAreaMap::Load`, `gs/pathfinding/WaterAreaMap.cpp:38-115`):
    `u32 versão`, `f32 largura`, `f32 comprimento`, `i32 n`, e então `n` áreas de 5 `f32` —
    `cx, cz, meia-largura, meio-comprimento, altura`. O `watermap.conf` ao lado diz a grade
    (`Map Width`, `Map Length`, `Submap Width/Length`, em texto). No `realm_155` a maioria dos
    mapas tem zero áreas (16 bytes, só o cabeçalho) e o mundo 1 tem vários com água de
    verdade — o maior com 5 áreas.

    **O que falta, em ordem:** ler o `watermap/` para uma segunda camada do
    `pw_data_loader::Terreno`, expor `altura_da_agua_em(x, z)`, e então acrescentar as duas
    recusas — a de montar e a de derrubar quem entra na água montado.

86. **Sessão 2026-09-22 (parte 9): o modo roupa gravava, carregava — e o cliente não sabia.**

    **O relato.** "Ao deslogar e logar continua não salvando o modo do equipamento para
    roupas. Veja se você rodou o comando no meu banco."

    **A migração estava aplicada** (`character_mode` em `public` e em `test`), e o binário em
    execução **tinha** o código do B83 — conferido com `grep -c character_mode` no
    `/app/server_bin` do contêiner. O log do mundo desmentiu a hipótese óbvia:

    ```
    23:29:53  mundo: 11456 passou para o modo roupa
    23:44:38  mundo: RT (#11456) entrou no mapa 161      ← relogou
    23:44:44  mundo: 11456 passou para o modo armadura   ← o primeiro clique foi para armadura
    ```

    Se o estado não tivesse sido gravado e recarregado, o primeiro clique depois do relogue
    teria ido **para roupa**. Ele foi para armadura: o servidor sabia que o personagem estava
    de roupa. **A escrita e a leitura funcionavam.**

    **O que faltava.** O dono da tela não descobre o próprio modo roupa pelo `charactermode`
    nem pelo `info_player_1` — ele lê o **`state` do seu próprio `SELF_INFO_1`**:

    ```cpp
    //	Parse travel flag
    m_bFashionMode = false;
    if (Info.state & GP_STATE_FASHION)
        m_bFashionMode = true;
    ```

    (`CECHostPlayer::OnMsgHstSelfInfo`, `EC_HostPlayer.cpp:819-822`.) E o nosso `self_info_1`
    montava o `state` com **um bit só**, o de GM — o mesmo defeito do B80, mas no pacote do
    próprio jogador, que eu tinha deixado anotado como `falta` naquele item.

    O efeito era confuso de propósito: **os outros jogadores o viam de roupa** (o
    `info_player_1` já leva o bit desde o B80) e só ele se via de armadura.

    **O que mudou.** O trait `self_info_1` ganhou `modo_roupa`, o `pw-link` o passa do
    `CharacterDetails`, e o codificador acende `GP_STATE_FASHION` (0x2000). O 1.2.6 recebe o
    parâmetro e **o ignora**: o bit não foi conferido contra aquele cliente, e este servidor
    não muda o 1.2.6 sem evidência dele.

    ### Provas

    `o_self_info_1_leva_o_modo_roupa_do_proprio_jogador`: o bit sai em 0x2000, não muda o
    tamanho do comando, e convive com o de GM. Publicado nos contêineres do realm 155
    (build + `up -d`), que subiram limpos.

    Suíte com o banco: **625 testes, 0 falhas** (`--test-threads=2`); a rodada anterior,
    antes desta correção, já fechava em 623 com o B84 dentro.

    ### Lição

    Três perguntas antes de culpar a camada óbvia: o dado está no banco? o binário em
    execução tem o código? o **cliente** foi informado? Aqui as duas primeiras estavam certas
    desde o começo, e o log tinha a resposta — a ordem dos cliques depois do relogue provou
    que o servidor sabia. Ler o log antes de mexer no código teria economizado a suspeita
    sobre a migração.

87. **Sessão 2026-09-22 (parte 10): o inventário dos `.data` — e o mapa de água que eu disse não existir.**

    Pergunta do Murillo: "existe alguma pendência ainda de leitura de algum arquivo `.data`?"

    **O levantamento.** Cruzei os arquivos que existem no realm com os nomes que o
    `pw-data-loader` abre. O que sobrou, e de quem é cada um, está agora na spec 03 §3.6c. O
    resultado curto: dos arquivos que sobram, **dois são do cliente** (`task_npc.data`,
    `DynamicObjects.data`), três são provavelmente do `gdeliveryd` (`domain*.data`), um é de
    origem desconhecida (`extra_drops.sev`), e **um é nosso**: o `globalcontroller.conf`, que
    só importa quando a Loja Gold existir.

    **O achado que corrige o B85.** No B85 eu afirmei que a montaria na água não podia ser
    portada porque "não temos altura de água". **Está errado, e corrigi o item.** Cada mapa
    do realm tem uma pasta `watermap/` — 75 delas — com `watermap.conf` e `N.wmap`, que é
    exatamente o que o `CGlobalWaterAreaMap` do original lê. Eu havia olhado só a pasta
    `map/` de um mapa, visto apenas o `.hmap`, e generalizado.

    O formato está medido e documentado na spec 03 §3.6b: cabeçalho de 16 bytes e áreas de
    20 (`cx`, `cz`, meia-largura, meio-comprimento, altura). É um leitor pequeno.

    ### Lição, que é a mesma do B82

    Duas vezes no mesmo dia uma **busca estreita virou conclusão larga**: o extrator que só
    casava dígito virou "são cinco habilidades", e olhar uma pasta virou "não temos o dado".
    Antes de escrever `falta` por ausência de dado, varrer a árvore inteira à procura do que
    o original abre — `grep` pelas extensões nos fontes do `gs` foi o que achou o
    `watermap.conf` aqui, em uma chamada.

88. **Sessão 2026-09-22 (parte 11): o mapa de água, e a montaria que não entra nela.**

    Fecha o que o B85 diagnosticou e o B87 corrigiu: o dado existia, faltava o leitor.

    **O leitor** (`pw-data-loader/src/watermap.rs`). Porta de
    `gs/pathfinding/{GlobalWaterAreaMap,WaterAreaMap}.{h,cpp}`. Formato na spec 03 §3.6b; o
    que importa registrar aqui são as três recusas, todas do original: versão diferente de
    `0xCC00_0001`, submapa cuja medida não bate com a do `watermap.conf`
    (`GlobalWaterAreaMap.cpp:123-127`), e — regra da casa — **arquivo que não fecha no
    último byte**.

    Duas armadilhas do formato:

    - `NO_WATER` é **0.0**, então "altura zero" quer dizer *não há água aqui*, não "água no
      nível do mar". `quanto_abaixo` devolve 0 fora d'água, e não `0 − y`.
    - O nome do arquivo do submapa `(u, v)` é `(comprimento − v − 1) × largura + u + 1`: a
      numeração cresce **de baixo para cima**, ao contrário do índice interno
      (`v × largura + u`). Trocar um pelo outro dá um mapa espelhado na vertical, que só
      apareceria em jogo.

    **A prova que vale.** `o_mundo_1_tem_agua_na_altura_que_o_arquivo_diz` refaz a conta
    **fora** do leitor: o ponto `(0, 0)` do mundo cai no submapa `u = 4, v = 5` da grade 8×11
    de 1024 — arquivo `45.wmap` —, e dentro dele em `(−512, 0)`; abrindo esse arquivo por
    fora, a terceira das cinco áreas (`centro (−464, −240)`, meias medidas `48 × 272`) contém
    o ponto, e a altura dela é **216**. Se o leitor errar a origem, o índice, o nome do
    arquivo ou a caixa, o 216 não aparece. Os 75 mapas do realm são lidos sem uma recusa
    sequer (`os_mapas_de_agua_do_realm_sao_lidos_inteiros`).

    **A regra de jogo.** Dois limiares, os dois do `gplayer_imp::TestUnderWater`
    (`gs/player.cpp:14336-14342`), sobre `off = altura da água − y`:

    | `off` | o que acontece | onde |
    | :--- | :--- | :--- |
    | > 0,5 m | conta como submerso; invocar montaria é recusado com `ERR_PET_CAN_NOT_MOUNT` | `mount_petdata_imp::DoActivePet`, `gs/petman.cpp:344-348` |
    | > 1,0 m | a montaria **cai** | `TestUnderWater`, `gs/petman.cpp:402-410` |

    A recusa fica **depois** da canalização, onde o original a tem — quem tenta montar
    submerso vê os 3 segundos e então o erro. A queda roda no batimento de 1 s, junto dos
    amuletos: a montaria não precisa cair no mesmo quadro em que o pé toca a água, e varrer
    a posição de todo mundo 20 vezes por segundo custaria mais do que vale.

    **Uma diferença deliberada do original:** ao derrubar, o `mount_petdata_imp` só remove o
    filtro e limpa o mascote ativo — **não manda `recall_pet`**. Nós mandamos, reusando o
    caminho do recolher voluntário, porque sem ele o cliente fica com o mascote marcado como
    ativo e o botão de recolher apagado: exatamente o travamento que o B79 corrigiu.

    ### O que me custou tempo

    O teste falhava dizendo que a montaria não caía, com o mundo mostrando `abaixo = 3` e
    `na_agua = true` — ou seja, o estado certo e nenhum efeito. A causa não era o código: **o
    cenário de teste não roda o laço de tique**; os testes batem o relógio à mão com
    `mundo.tick(1000)`. O batimento de 1 s nunca acontecia. Vale lembrar disso ao testar
    qualquer coisa que dependa de batimento.

73-126. **Sessão 2026-09-20: inventário do 1.2.6 em árvore isolada (versao-126).**

    Pedido: trazer jogabilidade sem tocar na sessão 155. Worktree `../pw-126` criada
    do HEAD `2dca19e`; alterações da árvore original preservadas. B70–B72 não são
    importados desta outra sessão; a numeração 73 respeita a reserva solicitada.

    Evidência: `pw-pcapdiff --interno` relê `full_interno.pcap`; relatório em
    `docs/evidencias/126/full_interno.medidas.md`. Levantamento das chamadas de todos
    os fontes de `pw-gs/src`: 98 ids S2C (104 métodos), 46 C2S tratados. Tabela com
    chamadas arquivo:linha em `docs/INVENTARIO_PROTOCOLO_126.md`. Tamanhos variáveis
    e comandos não observados ficam pendentes; nomes do PCAP vêm do IR.

    Causas potenciais, ainda sem relato reproduzido de sintoma: layouts comuns dos
    ids 14/31/46/64/72/99/144/156 diferem da captura. `bus_server.rs:361` usa
    teleporte de 20 B; `s2c.rs:1034` compra de 11+n×15; `s2c.rs:2518` grupo de
    6+n×34. O 126 observado tem 16 B, uma compra de 20 B, grupo 6+n×25.

    Provas: ferramenta compilada e captura relida; ainda sem suíte do workspace.
    Nenhuma correção de comportamento, commit, publicação ou reinício. A consulta
    Docker inicial recusou acesso ao pipe; não impede a análise do PCAP.
    Falta fechar camada 2 (entrada/propriedades/inventário/barras) antes do combate.


74-126. **Sessão 2026-09-20: entrada 126 medida no PCAP e no binário.**

    `docs/ENTRADA_126.md` registra provas, limites e roteiro em jogo.
    EQUIP_DATA usava máscara u64 comum, quatro bytes além do 126; captura
    `s2c-66.txt:8` e validador VA 0x584a1d comprovam mask32. Corrigido só
    no override v126, com teste vermelho antes e verde depois.
    O validador VA 0x584610 rejeita ids acima de 260: 390 e seis avisos de
    status da entrada deixaram de sair no 126 por opções do WorldProtocol.
    Padrões preservam os bytes e sequência do 155, sem condição de versão no gs.
    SELF_INFO_00, bolsa vazia e configuração de atalhos reproduzem amostras
    originais; OWN_EXT_PROP confirma 152 bytes e offsets representados.
    Ataque mágico/resistências da propriedade seguem a limitação da base.
    Fechamento em 2026-09-21, escopo reduzido pelo usuário à Camada 2:
    11 testes focados aprovados, zero falhas, oito filtrados; TEST_DATABASE_URL
    definido (`docs/evidencias/126/camada2-focado.log:17`). As sentinelas
    verificam máscara 64 bits, pacote 390 e sequência de entrada do 155.
    A suíte ampla iniciada anteriormente terminou com 63/64 no arquivo de
    mundo: falhou a persistência das listas de missão em
    `subcomandos_no_mundo.rs:1807`. Não repetida nem investigada nesta retomada.
    Não se declara regressão global aprovada; Camadas 3/4 não iniciadas.
    Parada para aprovação, sem novas alterações fora da Camada 2.
    Nenhum commit, publicação, reinício ou teste visual declarado.


89. **Sessão 2026-09-21: combate 126, comando 144 e cadência medida.**

    Base a305e51, árvore versao-126. HOST_SKILL_ATTACKED saía pelo escritor
    comum de 19 B (`bus_server.rs:2489`); captura `s2c-144.txt:2` e cliente
    VA 0x584af4 comprovam 15 B. Teste vermelho antes, verde após override
    em `v126/mod.rs:130`; padrão do trait preserva bytes do 155.
    84/83/24/26/33 reproduzem as amostras originais. Cadência: 23 intervalos
    de resultado, 22 ticks anunciados, mediana 1149,332 ms; 83→24 mediana
    49,3695 ms (52 pares). Script checa remontagem e contagens contra pcapdiff.
    Regra temporal não alterada; corrigida apenas a contradição documental
    sobre dano imediato (InsertDamageEntry, actobject.cpp:1758-1776).
    Provas/limites/roteiro: docs/COMBATE_126.md. Testes com banco: 3 de
    protocolo + 3 filtros de mundo aprovados, sem suíte completa.
    Sem commit/publicação; sem teste visual. Camada 4 não iniciada.

90. **Sessão 2026-09-21: experiência e pacotes de itens 126; leitor v55 adiado.**

    Captura original e validador do cliente exigem 31/99=14 B, 46=9 B,
    72=7+13*n e 156=10 B; chamadas diretas em jogo.rs usavam formatos 155.
    Cinco métodos no trait preservam o padrão; overrides só em v126.
    Contexto usa a estratégia do servidor; regras comuns não mudaram.
    36=4 B e 158=8 B já corretos, agora com gabaritos da captura.
    Teste mínimo antes: 2 aprovados/5 falhas; depois 7/0 protocolo e 2/0
    mundo, com TEST_DATABASE_URL. Os dois gabaritos de mundo eram v126 com
    offsets 155; corrigidos pela captura, sem retirar verificações de estado.
    Evidências e limites: docs/ITENS_EXPERIENCIA_126.md. Sem suíte completa,
    publicação, commit ou confirmação visual. O arquivo tasks v55 mede
    20.793.663 B e declara 2819 entradas, mas tasks.rs:1139 só aceita v129.
    Por pedido do usuário, leitor fica para sessão com modelo mais barato:
    docs/PROMPT_TASKS_V55.md contém escopo, critérios e relatório de retorno.

91. **Sessão 2026-09-22: mapa estrutural completo do `tasks.data` v55.**

    O arquivo do realm 126 foi conferido em 20.793.663 bytes, SHA-256
    `ee042d417452cd26e076280fd2f8d0d05bda1c63b1777abfc6c7d5f8d7ca8017`, com
    2.819 raízes. No `elementclient.exe` (SHA-256
    `5fc88d47e01da3caea7d6ce4911b71b0f7085060a2d13889a380fc5ba7f0ed14`),
    `LoadTasksFromPack` VA `0x630c10` valida cabeçalho, percorre a tabela e chama
    `LoadFromBinFile` VA `0x62e550`, que entra em `LoadBinary` VA `0x62f6c0`; a rotina fixa lê `0x216` = 534 bytes e decifra o
    nome pelo ID. Provas reproduzíveis: `docs/evidencias/126/tasks-v55-*.txt` e
    `medir_tasks_v55.py`.

    A tabela foi fechada pelo fluxo do binário: assinatura opcional (60 B), horários
    (48 B por contador em `fixo+0x4e`), itens 13 B, equipe 32 B condicional,
    monstros 22 B, prêmios 75 B com candidatos variáveis, quatro escalas, quatro textos
    UTF-16, cinco diálogos e filhas recursivas. O candidato de prêmio não tem tamanho fixo:
    `m_bRandChoose` (1 B), contador `u32` e `ITEM_WANTED` de 13 B; isso foi confirmado em
    `0x62d9d0`, evitando a hipótese errada de 17 B por candidato. O validador independente
    fecha 2.819/2.819 raízes, 7.994 tarefas e profundidade 4 exatamente no byte 20.793.663:
    `docs/evidencias/126/tasks-v55-validacao-contagens.txt`. Mapa completo e VAs em
    `docs/RESULTADO_TASKS_V55.md`.

    Não foi alterado `tasks.rs`: falta projetar os campos fixos consumidos pelo servidor e
    escrever/testar o leitor Rust v55, que terá de impor o mesmo fechamento por offset.
    Sem contêiner, publicação, commit ou teste Cargo nesta etapa de evidência.

92. **Sessão 2026-09-22: sincronização da versao-126 com multi-versions 6322d88.**

    Resultados 126 separados em ccf7ae4 (combate/itens) e 36a88da (mapa v55,
    sem leitor Rust). Validador repetido: 2819 raízes/7994 tarefas, EOF exato.
    Merge com conflito apenas em estado/histórico; série 155 B70–B76 preservada.
    Combate/itens 126 agora B89/B90; mapa v55 B91. Entradas históricas de
    inventário/entrada qualificadas B73-126/B74-126 para evitar outra colisão.
    A assinatura own_ext_prop ganhou max_ap: o teste da captura omitia argumento.
    Acrescentado zero, os quatro bytes finais de s2c-50.txt:11; 19 testes focados
    de layouts aprovados com banco (merge-layout-depois.log:26).
    Suíte ampla e decisão do cenário de mundo ficam para a etapa seguinte.
    Nenhum contêiner alterado ou publicação feita.

93. **Sessão 2026-09-23: 1.5.5 jogável no básico — tudo na `main`, e a frente passa ao 1.2.6.**

    O Murillo declarou o 1.5.5 jogável com os recursos básicos e pediu: commitar, juntar
    as branches na `main`, juntar a worktree `../pw-126` (trabalho do Codex) e deixar
    agentes, specs e docs com o estado atual — inclusive as instruções do Codex iguais às
    do agente do Claude (economia de contexto e relatório de consumo por etapa).

    1. `multi-versions` commitada (`dd33855`, B80–B88); suíte com o banco **631/0**.
    2. Worktree `pw-126`: o Codex deixou sem commit o despacho pós-merge
       (`docs/SINCRONIZACAO_126.md`): a retirada de amuleto esgotado passou a
       `self.sub.player_drop_item` (9 B no 126, `evidencias/126/s2c-46.txt:2`) e o
       `ELF_EXP` (283) virou `Option` no `WorldProtocol` — o 126 omite, porque o validador
       do cliente recusa ids > 260 (VA 0x584618). O cenário de mundo dos testes ficou
       parametrizado pela versão (padrão 155; compra e coleta com caso 126).
       **Defeito achado na revisão:** a suíte inteira dava **636/2** — os testes de
       `QUERY_NPC_INFO_1` e `QUERY_PLAYER_INFO_1` conferem o gabarito de captura do 1.2.6
       (12 e 24 B, sem `iTargetID`) e herdaram o cenário padrão 155. Passaram a fixar
       `cenario!(GameVersion::V1_2_6)`. Depois: **638/0** (`f2caa6f`).
    3. `main` avançada até `multi-versions` e `versao-126` juntada. Conflito só em estado
       e histórico, pela numeração: a `versao-126` usava B77–B81 para itens próprios, que
       colidiam com os do 1.5.5. **Renumerados: B77→B89, B78→B90, B79→B91, B80→B92 e
       B81→B93** (este item) — no histórico, nos `docs/*_126.md`, nos
       `docs/*TASKS_V55.md`, no comentário do `traits.rs` e **só nas linhas das specs que
       vieram da `versao-126`**. O "B74" solto dessas linhas virou `B74-126`. Referências
       a B77–B88 fora disso são do 1.5.5.
       **O merge não compilava** o `layouts_do_126.rs`: a `main` acrescentou argumentos
       depois que o teste foi escrito — `em_combate` no `self_info_00` e `magico` +
       `resistencias` no `own_ext_prop` (B77). O `State` da amostra `s2c-38.txt` é 0 (fora
       de combate). E os bytes que o teste pulava (`s2c-50.txt[112..140]`, "o trait não
       recebe") decodificam como Atq. Mágico 1–1 e resistências 2×5: passados ao trait, o
       `OWN_EXT_PROP` do 1.2.6 **confere byte a byte com a captura**, e o teste passou a
       comparar o pacote inteiro (spec 04 corrigida).
       Suíte inteira com o banco na `main` depois do merge: **654 testes, 0 falhas**.
    4. `AGENTS.md` reescrito para o Codex com as regras do `pw-server-dev` (evidência,
       specs na mesma entrega, contexto é recurso, tabela de consumo por etapa), apontando
       as skills como arquivos em `.claude/skills/`. `.codex/agents/pw-server-dev.toml`
       passou a remeter ao `AGENTS.md`, em vez de manter uma cópia que envelhece.
    5. `ESTADO_E_RETOMADA.md` §0 encurtado para o marco "1.5.5 jogável no básico", com a
       fila do 1.5.5 em §5A e o painel de paridade 1.5.5 → 1.2.6 em §5D.
    6. **Uma worktree só.** A `../pw-126` foi removida a pedido do Murillo, e a branch
       `versao-126` apagada (estava inteira na `main`). Antes: os 32 `.log` de evidência
       ignorados pelo git que os docs do 1.2.6 citam (`merge-layout-depois.log`,
       `elf-depois.log`…) foram copiados para `docs/evidencias/126/` da principal, junto
       com os três logs de sessão da raiz (prefixo `sessao-`); a `pw-126/data` era uma
       **junção** para a `data/` da principal e foi desfeita com `rmdir` sem `/s` antes da
       remoção. Claude e Codex trabalham agora na mesma pasta, na `main`.

94. **Sessão 2026-09-23: `elements.data` v7 no catálogo genérico do realm 1.2.6.**

    ### a. Pedido e causa

    O `realm_126` ainda usava o leitor tipado antigo de `elements.rs`, que fechava o
    arquivo por tamanho, mas devolvia valores provisórios e não alimentava monstros,
    poções, minas, montarias, voo, amuletos e classes no `GameDataManager`.

    ### b. Evidência

    O carregador do binário **1.2.6** (`F:/Games/perfectworld_126/element/elementclient.exe`,
    VA `0x60ff5d-0x60ffb0`) compara `0x30000007`, lê a primeira contagem `u32`
    imediatamente depois e registros de 84 B. A ordem das tabelas vem do fonte
    disponível mais próximo,
    `source_client_153/CCommon/elementdataman.cpp:3611-3800`, e do enum em
    `source_client_153/CCommon/ExpTypes.h:4930-5000`; os tamanhos foram medidos no
    `data/realm_126/config/elements.data` de **16.664.770 bytes**. O v7 tem cabeçalho
    de 4 B e não traz os dois blocos de tag do v156. O arquivo fecha com **119 entradas**
    (118 fixas e `TALK_PROC`) e **23.337 registros**. Casos do arquivo: poção 1796
    (`id_major_type=1794`, `cool_time=15000`), monstro 986 (`aggressive_mode=1`,
    `common_strategy=60`, `drop_times=1`, primeiro drop 8612), mina 8592
    (`npcgen_1_id_monster=8226`), Cavalo 8784 (`speed_a=8`), item de voo 2092
    (`character_combo_id=192`), arma 6 (`price=120`, `shop_price=240`, pilha 1),
    amuleto 12812 (`cool_time=10000`) e classe 2
    (`character_class_id=0`). Os deslocamentos v7 que diferem do v156 foram medidos
    nesses registros e fixados em `specs/elements_layouts/generate_v7.py`.

    ### c. Correção

    `v7.json` entra nos leitores Rust/Python; o `GameDataManager` passa ao genérico.
    O catálogo deixa opacos os registros 103–112 e os trechos sem semântica verificada.
    `MINE_ESSENCE` v7 não expõe `material_gain_ratio`; o carregador usa a soma das
    probabilidades do próprio registro, já exigida como 1, até medir a regra antiga.

    ### d. Provas e pendências

    Teste vermelho antes do catálogo (versão não suportada) e verde depois; caso de
    corrupção por byte extra e integração do `GameDataManager` com o realm 126.
    **Suíte inteira com `TEST_DATABASE_URL` e dois fios: 656 aprovados, 0 falhas.** Ainda falta
    confirmação em jogo no cliente 1.2.6; os bytes do arquivo v7 e a entrada do
    carregador binário confirmam cabeçalho e fechamento.

95. **Sessão 2026-09-24: a Forma Sombria que não transformava e a caixa que não abria.**

    Relato do Murillo, teste no 155 com o Tormentador: (a) a "Caixa de Tesouro do Guerreiro"
    não fazia nada ao usar; (b) a Forma Sombria (2570) não transformava. O log confirmou os
    dois como **não implementados**: `usou o item 41073 ... que não se gasta` e
    `habilidade 2570 — sem porte: Fairyform`. (Numerado 95 porque o Codex, trabalhando na
    mesma pasta ao mesmo tempo, registrou o B94.)

    **a. A caixa.** O 41073 é um `POKER_DICE_ESSENCE` — a caixa das Cartas de General.
    `generalcard_dice_item::OnUse` (`gs/item/item_generalcard_dice.cpp:10-54`): bolsa cheia
    (`IsFull`, mesmo que a caixa fosse liberar o slot) recusa; sorteia uma das 256 entradas
    (`RandSelect`); gera a carta por `generate_poker` (`gs/template/generate_item_temp.h:
    3144-3191`) — o conteúdo é o `generalcard_essence` (`gs/item/item_generalcard.h:9-19`),
    oito `int`: tipo (do `POKER_SUB_TYPE`), `rank`, `require_level`, liderança sorteada em
    `require_control_point[0..1]` (`abase::RandNormal`), `max_level`, nível 1, exp 0,
    renascimentos 0; `obtain_item` e a caixa se gasta. Recusado, o original **não** manda
    erro (`error_cmd` comentado em `gs/playercmd.cpp:2041-2045`).
    Feito: `pw_data_loader::cartas_de_general` (32 caixas e 227 cartas no `realm_155`; a
    41073 sorteia 3 cartas, probabilidades somando 1, todas `POKER_ESSENCE` com subtipo) e
    `BusServer::usar_caixa_de_cartas`. **O sistema de cartas não existe** — equipar,
    liderança, atributos, nível, devorar: a carta só fica na bolsa.

    **b. A Forma Sombria.** `filter_Fairyform` (`cskill/skill/skillfilter.h:16819-16875`),
    aplicado por `SetFairyform` (`playerwrapper.cpp:5247-5257`, um dos que não consultam o
    dado) com `SetTime(16000 + 3000·L)`, `SetRatio(0,04·L)`, `SetValue(0,6·L)`
    (`skills/skill2570.h:240-243`) — 19 s, +4% de velocidade e +60% de defesa no nível 1.
    No `OnAttach`: `ChangeShape(1 | FORM_CLASS << 6)` (= 65), trava do equipamento, ícone
    `HSTATE_FAIRYFORM` 279, `EnhanceSpeed`, `EnhanceScaleDefense`; no `OnRelease`, o inverso.
    Máscara `WEAK | HEARTBEAT`: **sem `REMOVE_ON_DEATH`** (sobrevive à morte) e nem bênção
    nem maldição.
    Feito: `Efeito::Fairyform` (`efeitos.rs`, com `escala_defesa` no `Filtro`, `forma()` e
    `equipamento_travado()`); o S2C `PLAYER_CHGSHAPE` (163, `{ int idPlayer; u8 shape }`,
    `EC_GPDataType.h:2876-2880`), mandado ao dono e a quem está em volta pelo
    `avisar_efeitos` quando a forma difere de `PlayerEntity::forma_enviada`; o `shape_form`
    do `info_player_1` passa a ser a forma dos filtros (quem chega depois vê transformado);
    vestir, mover para o corpo, trocar peças e descartar peça recusados com `ERROR_MESSAGE`
    40 (`ERR_EQUIPMENT_IS_LOCKED`, `common/protocol.h:720`; `gs/player.cpp:7874, 7991,
    8077, 8258`) e destrave dos slots. `ao_morrer` deixou de apagar o `Fairyform`.
    **Falta:** o `EventChange` da forma (as habilidades que só existem transformado).

    Provas: testes novos — `cartas_de_general` (2 unitários + 1 contra o `elements.data` do
    realm), `o_roteiro_da_forma_sombria_aplica_fairyform_com_os_numeros_do_stub`,
    `a_forma_sombria_transforma_tranca_o_equipamento_e_acaba_no_tempo`,
    `player_change_shape_tem_5_bytes_depois_do_cabecalho`, e no mundo com o banco
    `abrir_a_caixa_de_cartas_da_uma_carta_e_gasta_a_caixa` e
    `a_forma_sombria_tranca_o_equipamento_e_desfaz_a_forma_no_fim`. Suíte com o banco: **664 testes, 0 falhas** (na árvore que também tinha o B94 do Codex em andamento).
    Não publicado.

96. **Sessão 2026-09-23: leitor Rust do `tasks.data` v55 e a missão inicial do Guerreiro.**

    O Murillo confirmou no cliente 1.2.6 com Tsuko que a agressividade dos monstros está
    correta e a poção parece recuperar corretamente. Ao buscar a missão inicial, recebeu
    "Missão não disponível"; também observou falta da animação e do efeito depois da
    canalização da skill (a investigar no bloco seguinte). `elements.data` é v7;
    `tasks.data` deste realm é **v55**.

    **Causa da missão:** `TasksData::load_from_bytes` lia somente o cabeçalho v55 e entregava
    mapa vazio, de modo que o motor não encontrava a missão. O teste novo contra o arquivo
    real falhou antes da correção (zero raízes carregadas). Autoridade: `elementclient.exe`
    v126, `LoadBinary` VA `0x62f6c0`, leitura de 534 B fixos VA `0x62d04d`; seções variáveis
    e limites em `docs/RESULTADO_TASKS_V55.md:24-63` e
    `docs/evidencias/126/validar_tasks_v55.py:64-148`.

    **Correção:** despachante v55 em `tasks.rs`: bloco fixo 534 B, `ITEM_WANTED` 13 B,
    `MONSTER_WANTED` 22 B, prêmio 75 B, textos, diálogos e filhos recursivos. Cada raiz
    deve terminar no próximo offset (a última no último byte). Projeta no `TaskTemplate`
    id/nome/descrição, hierarquia, horários, itens, monstros, prêmio básico, NPCs, classes,
    método e conclusão. A tarefa 1173 "Primeiro Teste" traz o NPC 3517, classe 0 e prêmio
    45 moedas/75 exp/20 SP; 1174 usa método 1 (caça), conferidos no arquivo do realm.

    **Provas:** teste v55 verde: **2.819** raízes, **7.994** tarefas, missão 1173,
    corrupção por byte extra ou offset inválido recusada. Teste do motor: Guerreiro nível 1
    aceita a 1173. Suíte completa com `TEST_DATABASE_URL` e dois fios:
    **667 testes aprovados, 0 falhas**.
    Os offsets dos demais requisitos/flags do bloco fixo ainda precisam de mapeamento;
    ficam no padrão em `TaskTemplate`, portanto a paridade de todas as regras de missão
    não está demonstrada. Falta publicar apenas a pedido e testar no cliente 1.2.6.

97. **Sessão 2026-09-24: Gárgulas dentro da pedra — o mapa de movimento.**

    Relato do Murillo (RT, 155): nas Ilhas Ascendentes, tela (486, 525), Gárgulas Ancestrais
    nascem **dentro** de uma estrutura de pedra no chão. Pedido: que o nascimento considere,
    além do terreno, a altura das estruturas — para todos os monstros.

    **Onde:** mapa 161 (pasta `a61`), não o mundo — tela (486, 525) → mundo (860, −250)
    (`x = 10·tx − 4000`, `z = 10·ty − 5500`). A Gárgula Ancestral é o tid 44606 (52 geradores
    no `a61`, todos de área no chão).

    **Causa, pelo original:** a área no chão (`terrain_gen_pos::Generate`,
    `gs/npcgenerator.cpp:4299-4318`) não usa só o terreno — `path_finding::GetValidPos`
    pergunta ao `NPCMoveMap` do plano (`gs.conf [MoveMap] Path`, `gs/mapresman.cpp:87-97`)
    se o ponto é alcançável e **quanto o piso fica acima do terreno**
    (`CNPCMoveMap::GetValid3DPos`, `pathfinding/NPCMoveMap.h:199-210`): `y = offset + delta +
    terreno`; ponto inalcançável é sorteado de novo, até 5 vezes. O projeto não lia o
    `movemap/` e punha todo mundo no `.hmap`. Medido no `a61`: 22 das 52 Gárgulas estão num
    ponto cujo piso fica de 0,14 a 2,03 m acima do terreno — a pedra. O `GenerateY` (só
    terreno) é usado apenas para os membros de grupo em volta do líder
    (`group_spawner::GeneratePos`, `:5230-5246`), que o projeto não modela.

    **Feito:** `pw_data_loader::MapaDeMovimento` (`movemap.rs`): `movemap.conf` (lido como
    bytes — termina num comentário em GBK, e o `read_to_string` deixava o mapa vazio), `N.rmap`
    (`CBitImage`, `BitImage.h:228-282`) e `N.dhmap` (`CBlockImage<FIX16>`, `BlockImage.h:
    309-383`, altura em 1/64 m), fechando no último byte; `acima_do_terreno(x, z)`.
    `SpawnInstance::posicao_no_mapa` porta o `Generate` (com as 5 tentativas, sorteio
    determinístico por `posicao_alternativa`), e `WorldInstance::init_spawns` a usa; o
    `chao` da IA do monstro passou a ser terreno + piso (`Get3DPosOnGround`,
    `NPCMoveMap.cpp:155-166`), senão o monstro afundava na pedra no primeiro passo. O log de
    subida diz quantos nascimentos ficaram em cima de estrutura. **Falta:** usar o alcance
    (`.rmap`) para desviar de obstáculo.

    Provas: `movemap.rs` (3 unitários de formato); `tests/movemap_do_realm.rs` — os 55
    submapas do mundo fecham; `as_gargulas_do_mapa_161_nascem_em_cima_da_pedra` (22 de 52
    levantadas, a de (848,7; −240,2) exatamente 2 m acima do terreno);
    `nenhum_nascimento_no_chao_fica_dentro_de_estrutura` (36.945 conferidos em todos os mapas
    do realm, 439 levantados). Suíte com o banco: **673 testes, 0 falhas** (a árvore
    tinha também o trabalho do Codex em andamento). Não publicado. (Numerado 97: o Codex registrou o
    96 na mesma pasta.)

98. **Sessão 2026-09-24: resultado visual da skill 299 no cliente 1.2.6.**

    **Relato:** depois de canalizar Enxame de Ferroadas, Tsuko não vê o efeito nem a
    animação de lançamento. `skillstr.txt:2858-2869` do cliente 1.2.6 identifica a
    habilidade 299 (1,5 s de conjuração, 1,0 s de execução).

    **Evidência:** `EC_HostMsg.cpp:947-955` chama `PlayAttackEffect` ao receber o
    resultado 142; `EC_Player.cpp:3414-3525` inicia a ação de ataque. A captura
    `docs/evidencias/126/full_interno.medidas.md:101-102` mede os resultados 142/143
    em 14/18 bytes de payload. O original envia `SKILL_PERFORM` (88) só ao dono
    (`gs/player.cpp:4066-4073`); o broadcast está comentado. O código enviava 88
    também aos demais, cuja própria skill tem estado alterado pelo tratador
    (`EC_HostMsg.cpp:5929-5937`).

    **Correção:** remover o broadcast do 88. Para a skill 299 no barramento v126,
    o teste confirma 85 → 88 → 142 (14 bytes de payload) → 123 para o dono;
    o outro jogador recebe 85 e 143, sem 88. O teste falhou antes da correção
    em `SKILL_PERFORM pertence apenas ao dono` e passou depois (1/0, com banco).
    Suíte inteira com `TEST_DATABASE_URL` e dois fios: **674 aprovados, 0 falhas**.

    **Limite:** o teste prova a ordem e os bytes enviados, não o render do cliente.
    A causa do visual ausente na tela do próprio Tsuko permanece pendente;
    após publicação autorizada, conferir o alvo, o 142 e o overlay do 1.2.6.
    Nenhum contêiner foi reconstruído nem houve commit/push.

99. **Sessão 2026-09-24: o monstro de chão desvia de obstáculo — porte do `pathfinding`.**

    Pergunta do Murillo: o 1.5.5 original desvia de obstáculo? Sim — `cgame/gs/pathfinding/`.
    Pedido: implementar conforme o fonte.

    **O que o original roda de fato** (é fácil ler o agente errado):
    - Perseguir: `follow_target::CreateAgent` → `CreateNPCChaseAgent(mapa, chão)` com o modo
      **padrão** `NPC_MOVE_BEHAVIOR_CHASE_DISPERSE_ONCE` (`NPCMoveAgent.h:58`) →
      `CNPCDisperseChaseOnGroundAgent`, cuja base é o `CNPCChaseOnGroundNoBlockAgent` — e não
      o `CNPCChaseOnGroundAgent` com lista de 30 nós — porque `NPCDisperseChaseOnGroundAgent.h:21`
      define `CHASE_WITHOUT_BLOCK`.
    - O NoBlock (`NPCChaseOnGroundNoBlockAgent.cpp`): se a reta até a meta está livre no
      `.rmap` (`CanGoStraightForward`, Bresenham), anda reto; senão anda até o último pixel livre
      e, dali, busca com o `CPf2DBfs` (`Pf2DBfs.cpp`: gulosa pela distância de Manhattan, 8
      vizinhos, **em fatias** de N pixels por passo; o caminho gerado vai até o melhor nó visto,
      mesmo com a busca em andamento) e segue pelo `CPathFollowing`. `MAX_BLOCK_TIMES` = 3.
    - A dispersão: a meta é um ponto a `alcance` do alvo, num ângulo sorteado de ±60°
      (`2π/3`) em torno da direção de quem vem; o `CChaseInfo` guarda a direção entre sessões
      e espelha pelo plano do alvo (`CHalfSpace::Mirror`). É o que faz vários monstros
      **cercarem** em vez de se empilharem.
    - O condutor, `session_npc_follow_target::Run` (`npcsession.cpp:164-273`): passo de
      `run_speed × 0,5` m a cada 0,5 s; detalhe 20/40/60 pixels (50/90/120 bloqueado; teto
      300/600/900) pelo **quadrado** da distância inicial (≤ 100, ≤ 400, mais); recomeça ao
      chegar com 60% do alcance, ou quando o alvo se afasta > 7 m da meta antiga (> 4 m sem
      bloqueio); 3 chegadas (`_reachable_count`) ou o agente desistindo encerram a sessão.
    - Voltar: `ai_returnhome_task` → `session_npc_patrol` (`npcsession.cpp:883-960`),
      `follow_target` com alcance 0,8 m, acaba a 1,2 passo de casa; se ao fim ainda estiver a
      mais de 10 m (`GetReturnHomeRange` = 10², `aipolicy.h:1393`), `ReturnHome`
      (`ainpc.cpp:98-106`): `stop_move` em casa com `MOVE_MODE_RETURN` (7) e 0x500.
    - Passear: `cruise` → `CNPCRambleOnGroundAgent` (meta no disco de 10 m, alcançável e de
      preferência em reta, 12 tentativas cada) com o `CNPCChaseOnGroundAgent` (`CHASE_NORMAL`,
      lista aberta ordenada de 30 nós `SortVectorPathNode.h`, 200 pixels, previsão em diagonal
      quando não acha caminho, passo menor que um pixel repartido).
    - A altura de cada posição: `AdjustCurPos` de chão — terreno no ponto, ou terreno no
      **centro do pixel** + a altura do piso onde ela não é zero.

    **Feito:** `pw_gs::navegacao` (`SeguirAlvo`, `Perseguicao`, `Bfs`, `Trajeto`,
    `BuscaNaGrade`, `Passeio`); `MapaDeMovimento` ganhou a interface por pixel (`pixel_de`,
    `alcancavel`, `acima_no_pixel`, `centro_do_pixel`, `vizinhos_alcancaveis`, `reta_livre` com
    o pixel de parada); `MonsterAi::tick_no_mapa` usa os agentes para o monstro de chão
    (perseguir, voltar, passear), e `MonsterAi::tick` virou o mesmo com mapa vazio (tudo
    alcançável: a reta de antes). O mundo chama `tick_no_mapa` com o terreno e o `movemap`.
    Um teste antigo exigia a volta parar a 0,1 m de casa; o original para a até 1,2 passo — o
    teste foi corrigido para a regra do fonte.

    **Falta:** monstro de água e de ar (`ChaseInWaterPF`, `ChaseOnAirPF`, o `airmap/`), a fuga
    (`keep_out`, `CNPCFleeOnGroundAgent`) e o caminho de patrulha por pontos (`_path_agent`).

    Provas: `navegacao.rs` (parede de 64×64: contorna sem pisar nela; sem obstáculo é reta; o
    passeio nunca escolhe meta nem pisa na parede); `tests/navegacao_do_realm.rs` no `movemap`
    real do mapa 161: em 60 pares com a reta bloqueada, a reta antiga passava **24,4%** dos
    passos dentro de obstáculo e o agente **0,0%**, chegando em 47. Suíte com o banco: **678 testes, 0 falhas**
    (com o trabalho do Codex em andamento na mesma árvore). Não
    publicado. (Numerado 99: o Codex registrou o 98 na mesma pasta.)

100. **Sessão 2026-09-24: realm 1.2.6 — missão inicial 1177 e tempos das habilidades pelo `gs` 1.2.6.**

    Dois relatos do Murillo com o Tsuko (nível 1, cliente 1.2.6): "Missão não disponível" no
    Guia Selvagem e a skill 299 (Enxame de Ferroadas) sem animação/efeito de lançamento.

    **Nova fonte: o `gs` do servidor 1.2.6** (`files1.2.6/pwserver/gamed/gs`, ELF 32-bit
    **com símbolos**). Exige `0x30000007` em `elementdataman::load_data` (VA 0x81b1e3d), ou
    seja, é o servidor que carrega o `elements.data` v7 do realm. Lido com pyelftools +
    capstone, e as funções de tempo executadas com unicorn.

    **1) Missão.** Log real: `o NPC 3518 não entrega a missão 1177` (`jogo.rs:1161`). No
    `v7.json` do B94 o `NPC_TASK_OUT_SERVICE` tinha os 7 `storage_*` do v156, e o serviço 3531
    saía com `storage_id`=1177 e `storage_open_item`=1178, `id_tasks` zerados.
    `npc_stubs_manager::LoadTemplate` (VA 0x80ef014-0x80ef055) varre `id_tasks[i]` em
    `+0x44+4i` para i < 32: ID + Name + `id_tasks[32]`, 196 B. Corrigido no `generate_v7.py`.
    A 1177 é das classes selvagens (3 e 4; outras recebem o erro 13 do motor).

    **Auditoria do B94 ("encaixes" para fechar tamanho):**
    - **Ordem e tamanhos:** a sequência de `array<T>::load` do `load_data` dá nome e `sizeof`
      das 118 tabelas. Até a 97 os tamanhos batiam; **98–112 estavam errados** (1740, 368, 76,
      584, 1124, 1776, 708, 708, 1420, 10684… contra 344, 148, 1092, 368, 76, 584, 76, 356,
      436, 344…) — e **os dois conjuntos fecham o arquivo no último byte**. O do `gs` dá
      contagens plausíveis (78 SKILLMATTER, 19 REFINE_TICKET, 12 SPEAKER, nomes legíveis). O
      `PLAYER_SECONDLEVEL_CONFIG` usado por `progressao.rs:130` saía com `exp_lost_1`=1,7e-41
      (perda de exp na morte ≈ 0 no 1.2.6); agora 0,05/0,05/0,045/0,04… Registros: 23.337 →
      23.447. Não há mais `V7_OPACA_*`.
    - `NPC_SKILL_SERVICE`: 129 ids → `id_skills[128]` + `id_dialog` (VA 0x80ef202; o 129º era
      zero nos 31 registros).
    - `ARMOR_ESSENCE`/`DECORATION_ESSENCE`: `fixed_props` não existe no v7 (como na arma). A
      armadura 139 saía com `fixed_props`=`defence_low`=552 e `repairfee` float; só 18/1.036
      armaduras tinham `shop_price` = 2·`price`. Agora 702/1.036 e 330/561 (o resto difere por
      arredondamento), 139 = defesa 552/552, preço 4.800/9.600.
    - Conferidos pelo `gs`: `NPC_ESSENCE` (16 ids de serviço, +760…+836), `NPC_TASK_IN_SERVICE`
      (`id_tasks[32]`), `NPC_TRANSMIT_SERVICE` (`num_targets` +68, destinos de 12 B).
    - Só plausibilidade: `STONE_ESSENCE` (fecha na fronteira de `proc_type`),
      `CHARRACTER_CLASS_CONFIG` (lvlup_hp = 2·vit_hp), `PARAM_ADJUST_CONFIG`, `TASKDICE_ESSENCE`.

    **Mesmo login:** `task_notify reason=9` = `TASK_CLT_NOTIFY_SPECIAL_AWARD`
    (`TaskTempl.h:111`); o original só responde com o prêmio especial
    (`TaskTemplMan.cpp:311-319`). C2S 62 = `TRICKS_ACTION` (acrobacia; `protocol.h`, contagem
    desde o marcador `//60`). Nenhum dos dois participa de aceitar missão.

    **2) Skill 299.** Causa: `manager.rs` só carregava a tabela de habilidades para v156/v159;
    no v7 ela ficava vazia, `fase_de_execucao_ms` dava 0 e o 123 saía colado ao 142
    (`bus_server.rs:2358-2372`); a conjuração caía na tabela embutida/1.000 ms. O teste do B98
    passava porque o cenário injetava a 299 do 1.5.5 à mão.
    - **Captura original** (`_sync/capturas/*.pcap`, com o laço do `medir_cadencia.py`): 299 →
      `85` tempo 1.500, `88` +1.505 ms, `142` +1.555, `123` +2.504/2.514/2.534/2.551 (uma
      interrompida por 86/87 em +1.586); 102 → 200+700 (907–986 ms, n=14); 250 nível 2 →
      500+900 (1.449 ms).
    - **Fonte dos tempos:** o `skillstr.txt` do cliente 1.2.6 tem as 823 skills, todas
      presentes no `habilidades.json` 1.5.5; o texto diverge do 1.5.5 em 84 valores (30
      conjurações, 17 durações, 37 esperas). Mas o `gs` 1.2.6 compila os mesmos 823 stubs, e
      nas 84 divergências do texto concorda com o 1.5.5 em 62 — o texto não é fonte. Os tempos
      do `gs` 1.2.6 (3.718 funções constantes, 13 por nível, todas executadas) diferem do 1.5.5
      em 28 funções de 18 skills e preenchem 95 conjurações `null` do 1.5.5.
    - `specs/habilidades_126/extrair_tempos_126.py` → `tempos.json`;
      `TabelaDeHabilidades::do_126()` = as 823, tempos do `gs` 1.2.6, resto do stub 1.5.5 (mana,
      alcance, dano: **não conferidos** contra o 1.2.6); o `manager` a carrega para o v7.

    **Testes:** `generic_elements_tests` (serviço 3531 = [1177, 1178], 23.447 registros,
    armadura 139, `NPC_SKILL_SERVICE`, manager: NPC 3518 entrega 1177/1178, `perda_na_morte(0)`
    = 0,05, 299 = 1.500+1.000); `missoes.rs` (1177 por classe); `habilidades.rs` (tabela 126:
    299, 102, 250, 803); mundo com banco: `o_guia_selvagem_do_126_entrega_a_missao_inicial_1177`
    (Bárbaro nível 1 recebe `TASK_VAR_DATA` reason 1 da 1177) e o da 299 v126 agora com relógio
    (88 em 1.400–1.800 ms, 123 em 2.400–2.900 ms e ≥ 900 ms após o 142). Suíte com o banco:
    **681 testes, 0 falhas** (com o trabalho do Codex na mesma árvore). Não publicado; falta ver em jogo.

101. **Sessão 2026-09-24: realm 1.2.6 — ficha, atributos iniciais, contador de abate e dano de skill.**

    Commit do B94–B100 a pedido do Murillo: `eee918e`. Depois, quatro relatos dele com a
    Tsuko (Feiticeira, cls 3) no cliente 1.2.6.

    **1) Dano físico e mágico 1-1 na ficha.** O log do mundo: `ptemplate.conf de /app/data/config
    ilegível: ptemplate.conf não tem a seção [NEC]` e, no login, `o realm não tem ptemplate.conf`.
    O leitor exigia as 12 seções do 1.5.5; o arquivo do 1.2.6 tem 8. Sem `base_das_classes`,
    `recalcular_por_nivel` (`entity.rs:559`) sai sem fazer nada e fica o dano padrão 1.
    `player_template::__Load` do `gs` 1.2.6 (VA 0x80e4efc) lê 0 SWORDSMAN, 1 MAGE, 2 MONK, 3 HAG,
    4 ORGE, 5 GENIE, 6 ARCHER, 7 ANGEL → `SECOES_DE_CLASSE_126`; o leitor escolhe o conjunto
    presente. Feiticeira nível 1 com a Varinha Mágica 2251 (dano 3-3, mágico 5-5): físico 4-4,
    mágico 6-6, vida 60, mana 60 (`tests/ficha_do_126.rs`).

    **2) Atributos 15 em quase tudo.** Os moldes do `realm_126` no banco tinham os números do
    `ptemplate.conf` (Feiticeira 15/5/15/15 = seção `[HAG]`). O `clsconfig` 1.2.6
    (`files1.2.6/pwserver/gamedbd/clsconfig`): o `extend_prop` dos moldes das seis classes do
    1.2.6 no mundo 1 dá **5/5/5/5**, vida/mana = `vit_hp`/`eng_mp` × 5 (Feiticeira 60/60). É o
    mesmo erro que o 1.5.5 corrigiu em 2026-09-16. A `ficha_inicial` do `pw-link` já dava
    5/5/5/5, mas só com o `ptemplate.conf` carregado; sem ele o personagem caía no molde do
    banco. `scripts/2026_09_24_atributos_iniciais_5_126.sql` (moldes e personagens do
    `realm_126`: 5/5/5/5, `potential_points = 5 × (nível − 1)`) — **não aplicado**; o script
    de moldes de 2026-09-18 passou a gravar 5/5/5/5. Achado sem correção: as posições do
    `clsconfig` 1.2.6 para humanos e alados divergem das do banco.

    **3) O abate não contava na missão.** A 1177 é mãe sem objetivo; a filha 1178 pede 10 ×
    3303 (Filhote de Mandrágora, nível 1, 29 de vida). O banco mostrava a 1178 ativa com **5
    abates**: o servidor contava. A captura original tem o aviso dessa missão com **9 bytes**
    (`04 9a04 e70c0000 0a00`), sem os `dps`/`dph` que 1.5.3 e 1.5.5 têm (`TaskTempl.h:1773-1779`,
    17 bytes); o cliente descartava o nosso. Novo `WorldProtocol::task_notify_monster_killed`
    (v126 sobrescreve) e `Jogador::avisar_abate` (o `Contexto` usa o protocolo da versão).
    `NEW` (14), `COMPLETE` (10) e `ERROR_CODE` (7) já batiam com a captura.

    **4) Enxame de Ferroadas com 75 e 106 de dano no nível 1.** Não era dano padrão: era a
    fórmula certa com os números do stub 1.5.5 (`plus` = 2,3L² + 68,2L + 54 = 124,5,
    `skill299.h:81`). O `Calculate` do `gs` 1.2.6, executado no emulador, dá `plus` 23,7 / 94,0
    / … / 966 e `ratio` 0,55…1,0. O extrator passou a gerar a tabela inteira do 1.2.6
    (`specs/habilidades_126/extrair_habilidades_126.py` → `habilidades.json`, substitui o
    `tempos.json`): mana, aprendizado, alcance, distâncias, raio, ângulo, precisão e dano. As
    funções float saem da pilha x87 (TOP do FPSW), e o `GetRange` entra empilhado. Contra o
    1.5.5, nas 823: `plus` diverge em 68, `ratio` em 10, dinheiro exigido em 186, nível exigido
    em 142; elemento, base e fator nunca. Só 5 danos (317, 529, 666, 667, 799) dependem da vida
    do jogador e ficam com o do 1.5.5. Com a ficha corrigida, a 299 nível 1 soma 32 antes da
    resistência; a captura original mostra 20, 23, 23 e 24 no `142`.

    **Testes:** `ptemplate_tests` (126: 8 classes, [HAG] 15/5/15/15, 50/30); `ficha_do_126`
    (4-4, 6-6, 60/60, 299 = 32); `itens_do_126::monstro_abatido_na_missao_126` (bytes da
    captura); `habilidades.rs` (299 plus 23,7/94, mana 4, dinheiro 290; skill 1 plus 10,8);
    mundo com banco `o_guia_selvagem_do_126_entrega_a_missao_inicial_1177` agora também mata um
    3303 e confere o `106` de 9 bytes. Suíte com o banco: **684 testes, 0 falhas**. Não publicado.

102. **Sessão 2026-09-24: realm 1.2.6 — missões com filhas, moldes do `clsconfig`, catálogo por realm.**

    Segundo teste do Murillo depois de subir os contêineres com o B101: WRA (Guerreiro novo)
    nasceu com 5/5/5/5 (o B101 funcionou), mas com as barras de atalho vazias e no "galpão dos
    lenhadores"; a Tsuko continuava com 15 e perdeu a lista de missões, e pedir a 1177 de novo
    dava "Missão existente" + "Missão não disponível".

    **Missões.** O banco tinha a lista (1177 → 1178 com 5 abates e 1179 como irmã), e o log
    dava `não pode aceitar a missão 1177 (erro 3)` = já existe. O cliente processou o
    `TASK_DATA`: os pedidos de missões dinâmicas e de prêmio especial só saem de dentro do
    `InitActiveTaskList` (`TaskProcess.cpp:2123-2135` do 1.5.3). O cliente zera a lista inteira
    se o cabeçalho ou uma entrada for inválida (`:2049-2054`, `:2146-2153`), mas a nossa passa
    nessas regras, então **a causa exata do sumiço não está provada** (o `Tasks.log` da pasta do
    cliente usado não estava disponível). A diferença para o original está provada: o leitor v55
    deixava `filhos_em_ordem` em `false` (`tasks.rs`, B96) e o motor ativou as duas filhas.
    Posições das flags: sequência `pack(1)` do `TaskTempl.h:2037-2057` do 1.5.3 a partir de
    `m_ulTimetable` (0x4e): `m_bChooseOne` em 0x6a … `m_bFailAsPlayerDie` em 0x74. Conferidas
    na `libtask.so` 1.2.6 (com símbolos): `ATaskTempl::CheckDepth` (0x197b6) testa
    +0x6c/+0x6a/+0x6b na ordem de `m_bExeChildInOrder || m_bChooseOne || m_bRandOne`, e
    `CanGiveUpTask`/`GiveUpOneTask` leem +0x6f. Contagens nas 7.994 tarefas: em ordem 526,
    escolhe 136, sorteia 216, pai falha 7.993, pode desistir 7.254. Efeito colateral certo: a
    1173 ("Primeiro Teste") é "escolhe uma filha" (1175 ou 1176) e agora exige a escolhida (erro
    25 sem ela), como o original. Lista da Tsuko: `scripts/2026_09_24_lista_de_missoes_tsuko_126.sql`
    (1177 + 1178 com os 5 abates; conferido com um `SELECT` do resultado).

    **Concluídas no `TASK_DATA` do 1.2.6.** O 1.2.6 usa `FnshedTaskListOld` (versão 0, `u16` por
    entrada, falha no bit 15; captura `01 00 00 00 e8 06`; conversão do cliente em
    `TaskProcess.cpp:2060-2072`); nós mandávamos o formato novo. `concluidas_no_formato_antigo`
    no `WorldProtocol` v126.

    **Moldes.** O `realm_126` não tinha `ui_config` nos moldes (o link mandava barras vazias, e o
    cliente gravou a configuração vazia do WRA). As posições eram pontos de cidade. O `clsconfig`
    1.2.6 tem as duas coisas: o `config_data` (308–322 B) e a posição de cada molde. O
    `GetDataRoleId` do `gamedbd` 1.2.6 (VA 0x810c542) é o do 1.5.5 para as classes 0–7 (16, 19,
    20, 23, 24, 27, 28, 31). No `npcgen.data` do mapa 1, os pontos ficam ao lado do Guia de cada
    raça: 3517 (221,5; 2854,4), 3518 (−1445,2; 1398,9) e Jace Johnson 3519 (−313,4; −893,1),
    junto aos monstros das primeiras missões. O ponto antigo dos humanos ficava entre o Velho
    Caçador, a Teleportadora e os artesãos. `ler_clsconfig.py --sql <realm> --classes …` gera o SQL:
    `scripts/2026_09_24_moldes_do_clsconfig_126.sql`, e `2026_09_24_wra_pelo_molde_126.sql`
    (configuração e posição do WRA). O script de moldes de 2026-09-18 passou a ter os pontos
    novos. No 1.5.5 o nascimento também vinha do `clsconfig`, não de um `.data`.

    **Carga e versões.** `tests/carga_dos_realms.rs`: `realm_126` carrega sem falha (132
    lidos). Os não lidos são de sistemas que não existem em nenhum realm (`path.sev`,
    `domain.data`, `extra_drops.sev`, `task_npc.data`, `global_api.lua`, `ExtDataID.dat`,
    `precinct.clt`, `rare_item.conf`). O `realm_155` tem 3 falhas dos próprios arquivos
    (`a46/npcgen.data`, `a50/precinct.sev` truncados). Para uma versão nova não exigir código
    no que é tabela: `data/<realm>/catalogo/elements_layout.json` e `habilidades.json` valem no
    lugar do embutido (versão do layout conferida contra o cabeçalho, senão falha registrada).
    O extrator 1.2.6 passou a gravar a tabela de habilidades completa, e `do_126` só a lê.
    Continua exigindo código: o formato do `tasks.data` (v55 e v129), o protocolo do cliente
    (`WorldProtocol`) e as seções do `ptemplate.conf` (escolhidas pelo conteúdo).

    **Aplicados depois, a pedido do Murillo** (atributos: 2 personagens e 6 moldes; moldes: 6; WRA: configuração e posição; Tsuko: lista 1177 + 1178). Os 4 scripts de
    2026-09-24 ficam para o Murillo, na ordem: atributos, moldes, WRA, lista da Tsuko. Suíte com
    o banco: **689 testes, 0 falhas** (uma rodada anterior teve 1 falha de tempo em `a_consulta_de_jogador_devolve_alguma_coisa`, que passa isolada 3/3).

103. **Sessão 2026-09-24: realm 1.2.6 — movimento de monstro, dano da 299 e prêmios iniciais (diagnóstico).**

    Terceiro teste do Murillo (Tsuko e WRA, primeira missão de cada), com os 4 scripts do B102
    aplicados.

    **Prêmios (correto).** O WRA recebeu a Espada de You Xia (12497) sem escolher, e a Tsuko
    escolheu entre martelo e varinha. Pelo `tasks.data` v55, as seis classes seguem a mesma
    estrutura, cada uma com a arma dela: Guerreiro 1173 → 1175/1176 → 12497 → 1174; Mago 1198
    → 1202/1203 → 12500 → 1199; Selvagens 1177 → 1178 → 1179 (dois prêmios: 12498 + Cura ou
    12501 + Espírito, porque a cadeia é de Feiticeira **e** Bárbaro) → 1204; Arqueiro 2566 →
    2567/2568 → 12499 + 500 flechas → 2569; Sacerdote 1181 → 1183/1184 → 12500 → 1182. A
    captura original da 1179 manda `156`, `156`, `159`, `158`, `158`, `106` nova (1204) e `106`
    concluída só da filha, sem "concluída" para a mãe; o nosso manda o mesmo. O Murillo
    confirmou a arma no WRA. Teste de mundo
    `o_primeiro_teste_do_guerreiro_126_da_a_arma`: 10º abate real, entrega no 3517, `156` de 10
    bytes com o 12497, `106` nova da 1174. (`Entrada::definir_monstros` passou a ser pública
    para o teste.)

    **Dano da 299 (correto).** Captura: Feiticeira nível 1 (`OWN_EXT_PROP`: 5/5/5/5, 60/60,
    mágico 6–7) tira 20 do Filhote de Mandrágora (3303, madeira 10) e 23 do Inseto Esmeralda
    (1000, madeira 6). A redução é `res/(res + 40·nível − 25)` (`combat.rs:242`): no nível 1,
    10/25 = 40 %, então ~32 × 0,6 ≈ 20; no nível 2 da Tsuko, 10/65 = 15 %, então ~34 × 0,85 ≈ 29.
    O log real mostra 29–31. O WRA tira menos porque a skill 1 é física, contra a defesa 6.

    **Monstros "teleportando" (não reproduzido).** Medido contra a captura (16.822
    `OBJECT_MOVE` de monstro) e no mapa real (`tests/passeio_do_126.rs`), tudo bate:
    - passeio de 1.000 ms, um comando por segundo, passo = velocidade × 1 s (576 e 220 em 1/256
      m/s, iguais ao `MONSTER_ESSENCE` v7), pausa de ~32 s;
    - perseguição de 500 ms, modo 1, 4 m/s (o `gs` 1.2.6 confirma: 10 tiques e 0x1f4);
    - alturas iguais em ±6 mm (18.918 pontos);
    - 433 passos de passeio sem salto, perseguição sem salto, volta para casa andando em 19 de 20;
    - nenhum aviso de fila cheia no link.
    Diferença real, mas longe do jogador: o original deixa o cliente conhecer até 220 criaturas
    (41 `OBJECT_LEAVE_SLICE` na sessão), e o nosso corta nas 80 mais próximas, o que perto dos
    Guias dá 75–82 m. Falta o momento exato do salto em jogo. Suíte com o banco: **693 testes, 0 falhas**.

104. **Sessão 2026-09-24: realm 1.2.6 — monstros correndo ou pulando no passeio (causa e correção).**

    Novo relato: monstros **andando à toa** às vezes andam rápido demais ou "no ar". Hipóteses
    descartadas com medição:
    - `aipolicy`: não é interpretado no `pw-gs`, não mexe em movimento;
    - habitat: o `inhabit_type` v7 da área inicial é 0 (chão), com os voadores legítimos nascendo
      na caixa a y 279/330;
    - `npcgen.data` do 126: v10, com a mesma distribuição de `fOffsetTrn` do 155; os 33.610 ids
      do mapa 1 são únicos;
    - água e estrutura contra a captura: 5 pontos sobre estrutura, 0 sobre água, nenhum com a
      nossa altura acima da do original;
    - o passeio de 55 spawns reais em volta do Guia: 927 passos sem salto e sem nenhum no ar.

    **Reprodução ao vivo** (`reproducao_do_passeio_no_realm_126`, `#[ignore]`): o mapa 1 inteiro
    (27.323 monstros), o laço de tiques real, o barramento e um jogador junto ao Guia 3518.
    Em 30 s, **4 a 8 pares de `OBJECT_MOVE` do mesmo monstro a 45–63 ms** (ambos modo 0,
    `use_time` 1 s, na velocidade de andar).

    **Causa:** no último passo de um passeio, `fim_do_passeio` emenda outro com 10 % de chance
    (`ai_rest_task::OnSessionEnd`), e `comecar_passeio` zerava `espera_ms`. Resultado: o primeiro
    passo do passeio novo saía no tique seguinte ao último do anterior. **Correção:**
    `comecar_passeio` não zera mais a espera (no começo normal ela já é zero). Depois: 0 pares em
    647 movimentos. `o_filhote_de_mandragora_passeia_sem_saltos` agora confere o intervalo entre
    passos e acusa 16 com o defeito de volta. Os inícios de perseguição e de volta para casa
    também zeram a espera, mas só acontecem com luta: a conferir se aparecer salto em combate.
    Suíte com o banco: **695 testes, 0 falhas, 1 ignorado (a reprodução de 30 s)**. Não publicado.

105. **Sessão 2026-09-24: realm 1.2.6 — a Planta Devoradora que "teleportou" depois de morta.**

    Relato: com o WRA (imagem já com a correção do B104), uma Planta Devoradora que ele matou
    "teleportou". No log, o id era −2147477155 (29 de vida). O id vivo não bate com o do
    `npcgen.data` offline, porque o contador de instâncias segue a ordem de leitura das pastas
    de mapa, que muda entre o Windows e o contêiner.

    **Reprodução** (`reproducao_da_planta_devoradora_no_realm_126`, `#[ignore]`): o mapa 1
    inteiro, o laço de tiques real, o jogador a 6 m da Planta mais próxima do Guia 3517
    batendo nela quando ela começa a andar. Na luta, **nenhum salto**: o passo de corrida
    esperou o de passeio, 2 m a cada 0,5 s até o jogador, parada onde o cliente já a via. O
    salto era **depois da morte**: corpo de 20 s, `OBJECT_DISAPPEAR`, e **1 s depois** a Planta
    aparecia no ponto de nascimento, a 5 m de onde morreu.

    **O original.** Na captura do 1.2.6, Filhote de Mandrágora e Inseto Esmeralda voltam
    **~15,1–15,9 s depois da morte**, com `NPC_ENTER_WORLD` (16), a 3–19 m de onde morreram, e
    **sem nenhum `OBJECT_DISAPPEAR`**. No fonte: o construtor põe `_corpse_delay = 20`, mas o
    `CreateMobBase` o sobrescreve com o `iDeadTime` da entrada do gerador
    (`npcgenerator.cpp:2486`, `3828-3833`: 0 = sem corpo; senão 10..10.800 s, e o `OnDeath`
    corta em 200 s). No `gs` 1.2.6, `npc_spawner::CreateMobBase` faz o mesmo (VA 0x80f2407), e
    o `OnDeath` tem o mesmo teto 0xfa0. Com `_corpse_delay` 0 não há `disappear`
    (`npc.cpp:904-911`), e o `Reclaim` vem no tique seguinte. O renascimento é
    `Rand(15 + iRefreshLower, 15 + iRefresh)` s (`BASE_REBORN_TIME`, `config.h:106`;
    `npcgenerator.cpp:3355`, `3841-3855`), num ponto novo da área (`Reborn` → `GeneratePos`).
    No `npcgen.data` do 126: 27.610 de 27.618 monstros do mapa 1 com `iDeadTime` 0; Planta e
    Filhote com `iRefresh` 0 (15 s; o "1" que víamos era o `.max(1)` do leitor).

    **Correção:** o leitor guarda `iDeadTime`/`iRefresh`/`iRefreshLower` em
    `SpawnInstance::{corpo_s, renascer_min_s, renascer_max_s}` (`tempos_do_gerador`). O mundo
    liga cada monstro ao gerador: corpo pelo `iDeadTime` (0 = sem corpo e sem `disappear`),
    renascimento sorteado, posição nova (`SpawnInstance::posicao_de_renascimento`) e direção
    nova, anunciado com `NPC_ENTER_WORLD`. Sem gerador (invocado): como antes. Depois: a Planta
    morre, fica como corpo e volta **15,0 s** depois, a 16,2 m, sem `disappear`. Testes:
    `o_gerador_do_126_da_o_corpo_e_o_renascimento` e a reprodução com asserções. Vale também
    para o 1.5.5 (mesma regra do fonte). Suíte com o banco: **695 testes, 0 falhas, 2 ignorados (as reproduções)** — depois de corrigir a instabilidade de `a_consulta_de_jogador_devolve_alguma_coisa` (pegava só a primeira mensagem; agora espera o 32). Não publicado.

106. **Sessão 2026-09-24: realm 1.2.6 — monstros que "disparam" no passeio e personagem preso em combate.**
    Relato do Murillo testando com a WRA: monstros passeando começavam a andar rápido ou no ar e
    depois voltavam de uma vez para perto de onde estavam; e a WRA não saía do estado de combate.
    - **Passeio:** o último passo do passeio de chão ia como `OBJECT_MOVE` e a parada de
      `fim_do_passeio` era descartada (`acao.or(parada)`). O cliente não para sozinho no destino:
      segue andando na mesma direção até chegar comando novo (`CECNPC::MovingTo`,
      `EC_NPC.cpp:1225-1240`) e só puxa o monstro de volta a mais de 25 m (`MAX_LAGDIST`,
      `EC_NPC.cpp:79`). O original manda o último passo só como `stop_move` até o ponto final
      (`npcsession.cpp:626-633`); agora o nosso também, no chão e na água/ar (`ai.rs`).
      `o_filhote_de_mandragora_passeia_sem_saltos` passou a cobrar passo ou parada até o fim do
      `use_time` de cada `OBJECT_MOVE` (305 passos, 0 sem continuação).
    - **Combate:** o cliente só apaga a postura de luta com `SELF_INFO_00` de estado 0
      (`EC_HostMsg.cpp:1335`), e o nosso batimento só mandava o aviso quando vida/mana mudavam.
      A captura do 1.2.6 original (`full_interno.pcap`) tem `SELF_INFO_00` em que só o byte do
      estado muda 1→0 (t = 2409,9 s e 2441,9 s). `progressao::batimento` agora avisa no batimento
      em que `combate_s` chega a 0; teste `sair_do_combate_de_vida_cheia_avisa_o_cliente`.
    - Suíte com o banco: **696 testes, 0 falhas, 2 ignorados** (as reproduções). Specs 05 e
      ESTADO atualizados. Corrigido, falta ver em jogo; nada commitado nem publicado.

107. **Sessão 2026-09-24: realm 1.2.6 — missão automática do jogador novo e tela de dicas.**
    Relato do Murillo: o personagem novo não recebia a missão de entrega automática que ele
    pôs no `tasks.data` 1.2.6, nem abria a tela de dicas de jogador novo.
    - **Missão automática:** o leitor v55 (`missao_v55`) nunca lia `m_bAutoDeliver`; o
      diagnóstico `missoes_automaticas` dava 0 no 1.2.6. No `libtask.so` 1.2.6 (base `this + 4`
      = bloco fixo): `AddOneTaskTempl` testa +0xad/+0xac (`m_bDeathTrig`/`m_bAutoDeliver`,
      0x1cc8e/0x1ccbf); `CheckLevel` +0xc1/+0xc5; `CheckPreTask` contador +0xf1 e vetor +0xf5
      (5 posições até o gênero em +0x114, `CheckGender`); `CheckInZone` +0x79, +0x7a e a caixa
      +0x7e/+0x8a. Agora são 81 automáticas, entre elas a 9376 "Virando Dinossauro" (nível
      1..150, sem classe nem pré-requisito). No 1.2.6 o motivo 4 do `OnClientNotify` também é
      `OnTaskAutoDelv` (PLT resolvida). Testes: `as_missoes_automaticas_do_126` (carga) e
      `a_missao_automatica_do_126_e_entregue_ao_pedido_do_cliente` (mundo com banco).
      Quem pede é o cliente (`CheckAutoDelv`), com o `tasks.data` dele: a missão tem de estar
      também no arquivo do cliente.
    - **Tela de dicas:** o `gateway.rs` respondia o `GetHelpStates` sem gravação com 32 bytes
      zerados; o cliente os lia como "nenhum tipo de dica ativo" (`ECScriptOption.cpp:136-146`)
      e gravava isso de volta. O original manda vazio (`gamedbmanager.cpp:378`,
      `gethelpstates.hpp:27-31`) e o cliente usa o padrão, 0x7fff (`:89-101`). Agora vazio.
      eaa, RT, Tsuko e WRA já tinham gravado a palavra de tipos 0x0000:
      `scripts/2026_09_24_dicas_religadas.sql` a troca por 0x7fff, mantendo a lista de dicas
      vistas (ensaiado com ROLLBACK: 4 linhas; não aplicado).
    - Suíte com o banco: **698 testes, 0 falhas, 2 ignorados**. Specs 02, 03 e ESTADO
      atualizados. Corrigido, falta ver em jogo; nada commitado nem publicado.

108. **Sessão 2026-09-24: realm 1.2.6 — personagens sem o Portal da Cidade (167).**
    - Causa: o `2026_09_18_templates_iniciais_realm_126.sql` recriou os moldes do `realm_126` só
      com a habilidade de ataque de cada classe; no banco sobrou uma por classe (também sem a 235
      do Arqueiro e a 125 do Sacerdote). Como o molde tinha habilidade, o `default_skills()` (que
      põe a 167) não entrava. A semente do `template.rs` tinha listas sem evidência (2, 7,
      255-257, 352-354, 437-439, 1840, 11, 117-119) e os nomes das classes 3 e 4 trocados.
    - Evidência: `GRoleStatus.skills` dos moldes do `clsconfig` 1.2.6 (formato de
      `SkillWrapper::StoreDatabase`, `skillwrapper.cpp:870-879`), agora lido por
      `ler_clsconfig.py --habilidades`: cls 0: 1, 167; 1: 81, 167; 3: 167, 299; 4: 102, 167;
      6: 167, 234, 235; 7: 113, 125, 167 — todas nível 1. O do 1.5.5 também tem a 167 em todas.
    - `scripts/2026_09_24_habilidades_do_clsconfig_126.sql` (aplicado a pedido): moldes do
      `realm_126` iguais ao `clsconfig` (14 linhas) e as que faltavam dadas aos existentes —
      Tsuko e WRA receberam a 167. Script de 18/09 e semente do `template.rs` corrigidos.
    - Suíte com o banco: **698 testes, 0 falhas, 2 ignorados**. Spec 02 e ESTADO atualizados.
      Falta ver em jogo; nada commitado nem publicado.

109. **Sessão 2026-09-24: realm 1.2.6 — itens sombreados na venda e o baú do "Teste de Salto".**
    - **Venda:** o cliente congela cada espaço que manda vender e só o solta com
      `UNFREEZE_IVTR_SLOT` (181, `EC_HostMsg.cpp:2060-2065`). A captura do 1.2.6 original
      (`full_interno.pcap`, t = 2540,77 s) responde a venda com `181 00 01 00` e depois o `73`;
      o nosso só mandava o 73, e os itens ficavam sombreados. Agora vai um 181 por espaço
      pedido, também o recusado (o fonte 1.5.5 não o manda nesse caminho; no cliente 1.5.5 ele
      só descongela, então vale para todas as versões). Testes de venda ajustados.
    - **Baú:** a 3427 "Teste de Salto" tem a filha 3428, que pede 1 Sinal de Refinamento
      (11131). Ele sai da mina 11117 **Baú de Tesouros** (`task_in`/`task_out` 3428,
      `materials_1_id` 0: o item vem do `OnTaskMining`, já portado em `colheu_mina`), em
      x 255, z 3234, `fHeiOff` 33,5 m. O Baú Desgastado (12858, `fHeiOff` 26,8 m, x 424,
      z 3474) é da 7017 "As Pegadas de Laura" e não abre sem ela — certo. O Murillo confirmou
      que os baús ficam altos por estarem em construções.
    - Recurso passou a usar só relevo + `fHeiOff`, sem o mapa de movimento
      (`SetRegion(0, ...)`, `npcgenerator.cpp:3900-3902`, `:4320-4324`); no Baú de Tesouros a
      diferença era de 5 cm. Exemplos novos `missao` e `minas_do_item` (pw-data-loader).
    - Suíte com o banco: **698 testes, 0 falhas, 2 ignorados**. Specs 03, 05 e ESTADO
      atualizados. Falta ver em jogo; nada commitado nem publicado.

110. **Sessão 2026-09-25: realm 1.2.6 — a Tsuko em laço com a 5909 "Domesticadores".**
    Relato: ao receber a automática 5909, a Tsuko recebia a missão de novo mesmo clicando OK, e
    a "Instruções" ficou travada, sem ir à Domesticadora.
    - A 5909 (automática, classe 3, nível 3) tem as filhas 5911 "Instruções" (chegar a um
      lugar, método 4) e 5912 "Um amigo leal" (falar com o NPC 11534). O lugar da 5911 é o
      mapa 1 inteiro (±9999): o cliente a dá por alcançada logo e avisa (`TASK_NOTIFY` motivo 3,
      que no `OnClientNotify` 1.2.6 é `OnTaskReachSite`). O leitor v55 não lia o lugar, o
      `conferir_lugar` nunca a cumpria e a 5912 não vinha. Entrega e formato do `NEW` conferidos
      com a captura (resposta ao motivo 4 idêntica à do original).
    - Lugar medido no `OnTaskReachSite` do `libtask.so` 1.2.6 (0x1fa00-0x1fa6e): método +0x19a,
      mundo +0x1de, caixa +0x1c6/+0x1d2 (bloco fixo), `is_in_zone` e `OnSetFinished`.
      1.559 missões desse tipo; nove "Estágio 3-x" (4735-4770) com caixa invertida no arquivo.
    - Testes: `os_lugares_a_alcancar_do_126` (carga) e
      `a_5909_do_126_passa_da_5911_ao_chegar_ao_lugar` (mundo com banco: 5909+5911 → 5909+5912).
      Suíte com o banco: **700 testes, 0 falhas, 2 ignorados**. Spec 03 e ESTADO atualizados.
      Falta ver em jogo; nada commitado nem publicado.

111. **Sessão 2026-09-25: mascote de combate, 1.2.6 e 1.5.5.**
    Relato: a Tsuko pegou um mascote e tentou invocar; nada aconteceu. O log mostrava "montou o
    pet 10386": o `montar` só perguntava a velocidade do `PET_ESSENCE`, que o de combate também
    tem, e o `SUMMON_PET` de 16 B do 1.5.5 era descartado pelo cliente 1.2.6 (12 B).
    - **Evidência:** `petman.cpp` (`combat_petdata_imp`, `pet_manager`), `petnpc.cpp`
      (`gpet_imp`, `gpet_policy`), `petdataman.*`, `pet_filter.cpp`, `obj_interface.cpp:2826`,
      `npcgenerator.cpp:1989-2139`; no 1.2.6, `pet_dataman::LoadTemplate` (VA 0x8143580) e
      `__LoadDataFromDataMan` (curva 592 em VA 0x80e6fae) do `gs`, e a tabela do validador de
      tamanho do cliente (VA 0x584610/0x584e90, extraída caso a caso): 233 = 12, 234 = 8,
      249 = 12, 120 = 14, `info_npc` 27 B com +4 (0x1000) e +1+n (0x2000).
    - **Dados:** layout v7 do `PET_ESSENCE` estava deslocado (um `pet_snd_type` inventado em
      0x154); corrigido pelo `gs`, os coeficientes do 10386 ficam iguais aos do 1.5.5.
      `ModeloDeMascote` com as recusas do original (460/413 no 1.2.6, 783/439 no 1.5.5),
      curva 592 e `PET_FOOD_ESSENCE`.
    - **Protocolo:** trait com os comandos de mascote e `OBJECT_ATTACK_RESULT`; o v126
      sobrescreve 233, 234, 249, 120 e a entrada de mascote. A montaria do 1.2.6 passou a usar os
      da versão (antes mandava os de 16/9 B).
    - **Mundo:** `mascote.rs` (corpo por `GenerateBaseProp`, IA do `gpet_policy`, lealdade no
      dano, experiência e nível, fome/comida), `WorldInstance::invocar_mascote` e vizinhos,
      monstros que odeiam e atacam mascote (`tick_com_mascotes`), dano do mascote com crédito
      do dono, morte, recolher na morte/saída do dono, visibilidade com o pacote de mascote.
      Barramento: `bus_server/mascote.rs` (eventos, `PET_CTRL` 103, comida, reviver pela 329,
      gravação no `PetCorral`).
    - **Testes:** ponta a ponta nas duas versões (invocar → atacar por ordem → 120 → abate →
      237/238 → recolher → jaula), morte (234 + 247, lealdade 200 → 180, erro 87 ao invocar),
      tamanhos por versão, unidade de lealdade/fome/comida/experiência, modelos e curva nos dois
      realms. Suíte com o banco: **712 testes, 0 falhas, 2 ignorados**.
    - **Falta:** habilidades do mascote (comandos 4 e 5), soltar (`BANISH_PET`), renomear,
      invisibilidade, mascote de água/ar. Nada commitado nem publicado.
