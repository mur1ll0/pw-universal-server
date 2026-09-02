# Como testar o que já existe

> O que dá para verificar hoje, em ordem de esforço crescente. Cada passo diz **o que
> observar** e **o que significa se falhar** — um teste que você não sabe interpretar não
> serve de nada.

---

## Nível 0 — a suíte, sem subir nada

```
cargo test --workspace
```

**293 testes, 0 falhas.** Não precisa de banco, de rede externa nem do cliente. A maioria
não é teste escrito à mão: é gerada a partir do IR (`specs/protocol/*.json`), que por sua
vez foi extraído dos fontes C++ originais. Os que mais valem:

| Teste | O que ele cobra |
| :--- | :--- |
| `pw-wire/conformance_gamedata` | 1.064 structs, 3.822 escalares caem no deslocamento que o `g++ -m32` calculou nos cabeçalhos originais |
| `pw-wire/conformance_gnet` | 620 de 620 estruturas, 12.121 passos de leitura e escrita |
| `pw-protocol/campos_contra_o_ir` | 31 pacotes, 214 escalares — lê o `encode()` do Rust e compara com o IR |
| `pw-protocol/opcodes_contra_o_ir` | 40 opcodes e 12 subcomandos, mais a checagem de colisão |
| `pw-bus/topologia_do_compose` | a porta do barramento não está publicada, e cada `GS_BUS` acha o mundo certo |
| `pw-link/uplink_contra_o_mundo` | dois jogadores num link só não trocam de pacote |
| `pw-gs/comandos_contra_o_ir` | cada campo de subcomando é lido no deslocamento que o IR anuncia |
| `pw-protocol/subcomandos_s2c_contra_o_ir` | os codificadores S2C mandam o id **e o tamanho** que o IR dá ao comando — inclusive o prefixo fixo dos de tamanho variável |
| `pw-link/subcomandos_c2s_contra_o_ir` | cada braço do `match` trata o comando que diz tratar, e nenhum pisa em comando de GM |
| `pw-protocol/layouts_do_126` | os 10 comandos com layout próprio no 1.2.6 escrevem o tamanho que **um servidor 1.2.6 de verdade escreveu** — o gabarito é uma captura, não o IR |

Se algum falhar, a mensagem diz qual campo, em qual estrutura, com qual deslocamento
esperado — é para ser lida direto, sem depurador.

### Os quarenta que precisam de banco

Dois arquivos de teste ficam **mudos** sem um PostgreSQL, e avisam isso na saída. Eles
cobrem o que só o banco pode responder (uma cláusula `WHERE`) e o que só o mundo montado
pode responder (um subcomando mudando a simulação):

```bash
# Um banco descartável, com o esquema de verdade
createdb pw_database_test
psql pw_database_test -f specs/01_DATABASE_SCHEMA_POSTGRES.sql
export TEST_DATABASE_URL='postgresql://SEU_USUARIO@localhost:5432/pw_database_test'

cargo test -p pw-storage --test autorizacao_de_personagem   # 6 testes
cargo test -p pw-storage --test itens_sobrevivem            # 4 testes
cargo test -p pw-gs      --test subcomandos_no_mundo        # 30 testes
```

O primeiro monta dois realms 1.2.6 e prova que um jogador não entra no mundo como outro
nem apaga personagem alheio. O segundo prova que o item sobrevive a uma troca de slot,
com os atributos e o nome do criador intactos.

O terceiro monta um personagem de verdade com missão ativa e manda subcomandos pelo
barramento, conferindo que **o mundo mudou** — não que a resposta ficou bonita:

- movimento (`PLAYER_MOVE`, `STOP_MOVE`) muda a entidade e a grade espacial;
- combate (`SELECT_TARGET`, `NORMAL_ATTACK`) tira vida do monstro, mata, e o abate é
  notificado com o template certo;
- bolsa e NPC (`MOVE_IVTR_ITEM`, `SEVNPC_SERVE`) movem itens e dinheiro de verdade;
- poção cura pelo valor do `elements.data`, não por um número fixo;
- **grupo** (`TEAM_*`) — cinco testes com **dois clientes ligados ao mesmo mundo**: o
  convite chega a quem foi convidado, aceitar avisa os dois com vida e nível reais e no
  layout que o cliente lê, ninguém entra num grupo sem convite pendente, sair avisa quem
  ficou (com `TEAM_MEMBER_LEAVE`, não com "seu grupo acabou"), e sair do mundo também tira
  do grupo;
- **o mundo de teste é 1.2.6**, e desde o item 56 isso muda bytes: o `NPC_INFO_00` sai
  com 12 bytes de payload e o `PLAYER_INFO_00` com 24, que é o que aquela versão usa;
- **as consultas** (`GET_EXT_PROP`, `GET_ALL_DATA`, `QUERY_PLAYER_INFO_1`,
  `QUERY_NPC_INFO_1`) — a barra de vida do monstro traz o HP do mundo e não `1000` fixo, a
  consulta de outro jogador **responde** alguma coisa, o próprio estado sai do personagem e
  não de `120/280`, e o `GET_ALL_DATA` respeita os três sinalizadores do cliente.

**Quem mexer em autorização de personagem ou em subcomando tem que rodar estes três.**

Estes testes abrem um pool por teste, de propósito: cada `#[tokio::test]` cria o seu
runtime, e conexão `sqlx` não sobrevive à morte do runtime que a criou — um pool `static`
compartilhado trava. Está anotado no topo do arquivo.

---

## Nível 1 — subir a infraestrutura

```
cd docker
docker compose up -d --build
docker compose ps
```

Devem subir **10 serviços**: banco, cache, `pw-auth`, painel, e **dois por realm** —
`pw-realm-*` (o daemon de link, com porta pública) e `pw-world-*` (o servidor de mundo,
sem porta nenhuma).

### O que observar

```
docker compose logs pw-world-126 | grep barramento
```

Esperado: `servidor de mundo escutando o barramento`.

```
docker compose logs pw-realm-126 | grep barramento
```

Esperado, nesta ordem:

```
pw-link: barramento apontado para o servidor de mundo em pw-world-126:29100
barramento: ligado ao servidor de mundo em pw-world-126:29100
```

A segunda linha pode demorar alguns segundos — o link reconecta com espera crescente,
de propósito, para que a ordem de subida dos contêineres não seja uma corrida.

### O que significa se falhar

- **Só a primeira linha aparece, e nunca a segunda**: o mundo não subiu. Veja
  `docker compose logs pw-world-126` — quase sempre é o banco.
- **`GS_BUS não definido`** no log do link: a variável não chegou ao contêiner. Confira
  se o `docker-compose.yml` que subiu é o novo.
- **Nenhuma linha de barramento**: a imagem é antiga. `docker compose build --no-cache`.

### Uma coisa que vale conferir uma vez

```
docker compose port pw-world-126 29100
```

Tem que **falhar**. O barramento não autentica nada: quem alcança a porta manda
`EnterWorld` por qualquer `roleid` e passa a receber os pacotes daquele personagem. Se
algum dia isso responder, o `cargo test` já teria falhado antes — mas conferir na máquina
de verdade não custa nada.

---

## Nível 2 — o cliente 1.2.6 entrando no mundo

Este é o caminho que já funcionava, e o que não pode regredir. Aponte o cliente 1.2.6
para `<ip-da-máquina>:29000` e entre com um personagem.

### O que observar

No log do link, o caminho normal de login até o mundo. E então, no log do **mundo**:

```
docker compose logs -f pw-world-126
```

```
barramento: daemon conectou de 172.x.x.x:xxxxx
mundo: jogador 1024 entrou (localsid 43981)
mundo: subcomando 0 de 1024 (12 bytes de payload)
mundo: subcomando 15 de 1024 (28 bytes de payload)
```

**É esta última linha que importa.** Ela prova o caminho inteiro fechado — cliente →
`pw-link` → barramento → `pw-gs` — com o cabeçalho de subcomando lido corretamente em
little-endian. Ande com o personagem: o subcomando 0 (`PLAYER_MOVE`) aparece em rajada.

### O que significa se falhar

- **`jogador X entrou` aparece, mas nenhum `subcomando`**: o link não está repassando.
  Confira o nível de log (`RUST_LOG: info,pw_gs=debug` no serviço do mundo).
- **Números de subcomando absurdos** (3840 em vez de 15): alguém leu o cabeçalho em
  big-endian. Seria um bug novo — o teste `o_cabecalho_do_subcomando_e_little_endian`
  cobre exatamente isso.
- **O jogador entra e o mundo não registra**: o `EnterWorld` não chegou. Veja se a
  ligação do nível 1 estava de pé **antes** do login.

### O que mudou de lado

**Trinta e dois subcomandos agora são tratados pelo `pw-gs`**: movimento (0), logout (1),
seleção de alvo (2), ataque básico (3), renascimento (4), parada (7), desmarcar (8), os
sete de item (9, 11, 12, 13, 16, 17, 18), cinco de ação (42, 46, 47, 48, 75), NPC (37),
item (40), habilidade (41, 80), grupo (27, 28, 29, 30) e as consultas (21, 39, 67, 68,
110). Consequências práticas:

- **Sem `GS_BUS`, andar e deslogar deixam de funcionar.** É o preço declarado da
  separação; o `pw-link` avisa no log ao subir sem barramento.
- **A loja de NPC funciona ao contrário do que funcionava.** Comprar tirava item e
  pagava; vender cobrava e entregava. Se "a loja quebrou" para você, compare com o
  comportamento anterior antes de chamar de regressão.
- **A loja do Mall funciona ao contrário do que funcionava.** Comprar e consultar saldo estavam
  com os ids trocados: comprar devolvia saldo. Se a loja "parou de funcionar" para você,
  compare com o comportamento anterior antes de chamar de regressão.
- **Abrir barraca de venda não responde mais nada.** O comando 76 era tratado como "sair
  da zona segura", o que é outro comando. Agora ele cai no caso padrão e é só registrado.
- **Arrastar um item não apaga mais os atributos dele.** Antes, mover uma arma de um
  slot para outro — ou equipá-la — devolvia a arma sem essência, sem refino e sem o nome
  do criador, sem erro nenhum. Se você tem um personagem antigo, os itens que já
  passaram por isso não voltam.
- **Mover uma pilha continua movendo a pilha inteira.** Pedir 5 de 20 move os 20; a
  divisão de pilha ainda não existe.
- **O monstro revida.** Antes não: nada alimentava a tabela de ameaça, então a IA nunca
  atacava. Bata num monstro dentro do alcance e ele responde.
- **Você vê o dano que leva, e vê que morreu.** O servidor já debitava o HP no tick e não
  avisava ninguém — a vida parecia cheia até a morte chegar do nada.
- **Dá para renascer.** Quem zerava a vida ficava preso até reconectar.
- **Poção cura de verdade, pelo valor do `elements.data`.** Antes ela era reconhecida por
  dois ids no código e mostrava HP/MP fixos sem curar nada.
- **Habilidade causa dano de verdade no alvo certo.** Antes era 150 fixo, e o alvo saía de
  um deslocamento que pegava outro campo junto.
- **O combate passou a existir.** Antes o ataque respondia dano `35` fixo e HP
  `965/1000` fixo, e o monstro nunca morria. Agora ele perde vida a cada golpe, morre, e
  dá experiência. Se você bater num monstro e ele não morrer nunca, é regressão.
- **Missão de caça só conta abate de verdade.** Antes qualquer golpe em qualquer coisa
  contava como matar a criatura `13641`.
- **A barra de vida do alvo agora mostra o HP de verdade — e continua mostrando.** O link
  mandava `1000/1000` fixo tanto ao clicar quanto na **consulta periódica**, que é a que
  redesenha a barra a cada instante. Bata num monstro: a barra tem que baixar e *ficar*
  baixa.
- **Nada disso aparecia na tela até agora, por um segundo motivo.** O cliente descarta em
  silêncio qualquer comando cujo tamanho não bata com a struct dele, e mandávamos treze com
  o tamanho errado — entre eles o `NPC_INFO_00`, que é o único comando de barra de vida de
  monstro, e o `HOST_ATTACKRESULT`, que é o número de dano. O combate estava certo por
  dentro e invisível por fora. Ver os itens 46 e 47 do `ESTADO_E_RETOMADA.md`.
- **A lista de membros do grupo estava no layout errado inteiro** — cabeçalho de 1 byte
  onde o cliente conta 6, posição no lugar de campos que não existem, `mp` e `max_hp`
  trocados. Se o grupo "começou a funcionar", é isto.
- **Vida, mana, nível e saldo agora são os do personagem.** Eram `120/120/280/280` e
  `50000` escritos no código, iguais para todo mundo.
- **Comprar na Loja Gold não responde mais nada.** O que havia gravava a compra sempre no
  slot 12, apagando o que estivesse lá, e deixava o saldo em `49000` fixo. Foi removido em
  vez de corrigido — uma compra precisa de saldo, preço e slot livre.
- **O movimento não grava mais no banco a cada pacote.** Antes era um `UPDATE` por pacote
  de movimento, de cada jogador; agora o mundo guarda em memória e o autosave grava a cada
  60s. Se o processo morrer, perde-se até um minuto de deslocamento — troca consciente,
  porque a alternativa era dezenas de gravações por segundo por jogador.

O resto dos subcomandos (~390) continua no `gateway.rs`, e o mundo só os registra. Ver
NPCs e missão inicial se comportam como antes.

---

## Nível 3 — o cliente 1.5.3 no login

O que a Fase 1 entregou: o `Challenge` com `version` e `edition` corretos, que eram as
duas causas conhecidas de o cliente 1.5.3 recusar o login **antes** de pedir a senha.

Aponte o cliente 1.5.3 para `<ip>:29001`.

### Antes de tentar, confira o log do link

```
docker compose logs pw-realm-153 | grep -i gshop
```

Se aparecer o aviso de timestamp zerado, o login **vai** falhar com uma mensagem genérica
de versão errada, e não adianta insistir. O realm precisa, na pasta de configuração, de um
destes dois pares:

| Empacotamento | Arquivos |
| :--- | :--- |
| cliente | `gshop.data` + `gshop1.data` |
| servidor | `gshopsev.data` + `gshopsev1.data` |

O `realm_153` deste projeto tem o par do servidor, com timestamps `1461564404` e
`1452829733`. O `realm_148` **não tem nenhum dos dois** — o 1.4.8 vai falhar no login por
esse motivo até os arquivos aparecerem, e isso é esperado, não é bug novo.

---

## O que **não** dá para testar ainda

- **Gameplay servido pelo `pw-gs`.** O mundo recebe os subcomandos e não faz nada com
  eles. Combate, skills, spawn e movimento continuam no `pw-link`.
- **Vários mundos (mapas) dentro de um realm.** O `WORLD_TAG` existe, mas os mundos não
  trocam jogadores entre si. Note que isto é diferente de **subir outro realm** da mesma
  versão, que já funciona — a receita está em `docs/MULTIPLOS_REALMS.md`.
- **Reconexão do jogador a um mundo que caiu no meio.** O link reconecta; o estado do
  personagem naquele mundo, não.
