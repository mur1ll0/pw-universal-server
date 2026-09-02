# Estado atual e como retomar

> Nota de passagem entre sessões. Diz onde o trabalho parou, o que já é fato verificado
> e qual é o próximo passo concreto. Atualizar ao fim de cada bloco de trabalho.

**Última atualização:** 2026-09-02 — teste de ponta a ponta do Murillo contra o
docker-compose (não a VM): 1.2.6 loga e entra no mundo mas skills/missões/HP de NPC
falhavam; 1.5.3 nem loga. Diagnóstico e correção parcial nesta rodada — item 62.

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
| `F:\...\source_server_153` | Fontes C++ do servidor — `inl/`, `rpcdata/`, `rpcalls.xml`, `callid.hxx`, `share/rpc/` e **`cgame/`** (protocolo do mundo 3D) |
| `F:\...\source_client_153` | Fontes C++ do cliente — `CElementClient/Network/` e `CElementClient/EC_RoleTypes.h` |
| `F:\...\files1.2.6` | Binários compilados do servidor 1.2.6 e os `.data` |
| `F:\Games\perfectworld_126` | Cliente 1.2.6 |
| `_sync/` | Área de transferência entre a máquina local e o contêiner (ignorada pelo git) |

A VM local não tem `cargo` nem `docker`, então a compilação acontece no contêiner de
nuvem, e a sincronização é por *tarball* nas duas direções. Os arquivos que o contêiner
precisa para rodar tudo que está descrito aqui:

* do projeto: o workspace inteiro menos `target/` e `data/`;
* do cliente: `CElementClient/Network/{EC_GPDataType.h,EC_GameDataPrtc.cpp}` e
  `CElementClient/EC_RoleTypes.h`;
* do servidor: `inl/`, `rpcdata/`, `rpcalls.xml`, `<daemon>/callid.hxx`,
  `share/rpc/rpcdefs.h`, e `cgame/common/{types.h,protocol.h,protocol_imp.h}` +
  `cgame/gs/playercmd.cpp`.

Validação disponível e acordada com o usuário: Docker + cliente 1.2.6 com envio de logs,
captura de tráfego (Wireshark/pcap) e execução dos binários originais 1.2.6 para
comparação lado a lado.
