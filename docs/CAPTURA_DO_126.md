# Capturar o 1.2.6 de verdade

> O que gravar, como gravar, e o que fazer dentro do jogo para que a captura responda as
> perguntas que estão abertas. Roteiro para uma sessão só.

---

## Por que isto vale a pena

O IR do projeto é do **1.5.3**. Não temos cabeçalhos do 1.2.6, e por isso toda diferença
de layout entre as versões era indistinguível de um erro nosso — foi o que manteve 27
codificadores numa lista de "não dá para julgar" (item 46 do `ESTADO_E_RETOMADA.md`).

Uma captura de um servidor 1.2.6 em funcionamento resolve isso de um jeito que nem os
cabeçalhos resolveriam: **o cabeçalho diz o que o código pretende; a captura diz o que
aconteceu.**

E resolve uma segunda coisa, que é mais urgente. Ao corrigir treze codificadores para o
layout do 1.5.3, o 1.2.6 passou a receber esses mesmos bytes. Era a melhor aposta
disponível, mas é uma aposta. A captura a transforma em medição.

> **Correção (2026-09-01).** Esta seção dizia que o tráfego não era cifrado. **Estava
> errado.** A primeira captura mostrou que os dois sentidos ficam opacos logo após o
> `KeyExchange`: o `glinkd` cifra o elo com o cliente com ARCFOUR (RC4), e o `Setup` que
> deriva a chave não está entre as fontes vazadas. Ver o item 54 do
> `ESTADO_E_RETOMADA.md` — inclusive por que os dois argumentos que eu usei não valiam.
>
> **O que resolve:** o `glinkd` cifra só o elo com o cliente. Os daemons conversam entre si
> por **loopback, em claro** — e é por ali que os subcomandos passam antes de serem
> cifrados. A captura do mundo 3D é na `lo`, dentro da VM.

---

## Onde capturar — resolvido

A pergunta era: capturar no Windows (com `etwdump`/`sshdump`) ou dentro da VM?

**Dentro da VM**, e eu mesmo rodo. Levantei o terreno pelo SSH:

```
enp0s3   192.168.1.200/24             (rede em ponte)
glinkd   LISTEN 192.168.1.200:29000   <- é aqui que o cliente chega
gdeliveryd / gs / gamedbd             <- conversam por loopback, dentro da VM
```

O `tcpdump` já está instalado lá (4.9.2), e a gravação sai de `/root/captura/gravar.sh`.

### Por que não no Windows

- **`etwdump` é a ferramenta errada.** Ele lê o Event Tracing do Windows — serve para
  tráfego de *localhost* na própria máquina Windows e para diagnóstico de ETW. O seu
  tráfego não é localhost: sai do Windows e vai para 192.168.1.200.
- **`sshdump` funcionaria**, mas ele é literalmente o `tcpdump` rodando na VM com o
  resultado voltando por SSH. Ou seja: o mesmo que eu faço, com um túnel a mais no meio
  competindo pela rede que está sendo medida. Menos peças é melhor.
- **Capturar na placa do Windows** também funcionaria, mas passa pelo Npcap e pelos
  *offloads* da placa (LSO/RSC), que juntam e partem segmentos. A gente remonta o TCP de
  qualquer jeito, mas cada peça a mais é uma chance a mais de aparecer aviso de buraco.

Na VM é a placa do próprio servidor: **todo byte que ele mandou e recebeu passa ali**, sem
intermediário.

### Os dois clientes

Continuam no Windows, normalmente. Duas conexões da mesma máquina chegam com o **mesmo IP e
portas de origem diferentes**, e a ferramenta separa os fluxos por porta — os dois caem no
mesmo arquivo, já distinguidos. É o cenário ideal para a parte de grupo.

### Se um dia você quiser gravar sozinho

```bash
ssh root@192.168.1.200
# O que importa para o mundo 3D — em claro:
tcpdump -i lo -s 0 -w /root/captura/interno.pcap tcp
# O elo com o cliente — só o handshake é legível:
tcpdump -i enp0s3 -s 0 -w /root/captura/externo.pcap tcp port 29000
```

A gravação usa `-i enp0s3` e não `-i any` de propósito: `any` produz outro formato de
enlace, e Ethernet é o que está coberto por teste. O `-s 0` também não é decoração —
sem ele, versões antigas do `tcpdump` cortam o pacote em 96 bytes, **o corte é invisível
no arquivo**, e comandos truncados viram "comandos curtos" na leitura. Que é exatamente
o erro que esta captura existe para não cometer.

## O roteiro dentro do jogo

A captura só mostra os comandos que **acontecerem**. Cada item abaixo existe para
provocar um comando específico. Faça na ordem, sem pressa — trinta segundos entre um e
outro ajuda a separar as coisas depois.

### Parte 1 — o essencial (é o que responde as perguntas mais caras)

| # | O que fazer | O que isso provoca |
| ---: | :--- | :--- |
| 1 | Faça login e entre com um personagem | `GET_ALL_DATA`, `OWN_IVTR_DATA`, `TASK_DATA`, `PLAYER_CASH` |
| 2 | Ande um pouco pelo mapa | `PLAYER_MOVE`, `NOTIFY_HOSTPOS`, `OBJECT_MOVE` |
| 3 | Clique num monstro (só selecionar) | `SELECT_TARGET`, **`NPC_INFO_00`** |
| 4 | **Bata no monstro até matar** | **`HOST_ATTACKRESULT`**, `NPC_INFO_00` em rajada, `NPC_DIED`, `RECEIVE_EXP` |
| 5 | Deixe o monstro bater em você | `HOST_ATTACKED`, `SELF_INFO_00` |
| 6 | Use uma poção de vida e uma de mana | `HOST_USE_ITEM`, `SELF_INFO_00` |
| 7 | Abra a bolsa e **arraste um item de um slot para outro** | `MOVE_IVTR_ITEM`, `OWN_ITEM_INFO` |
| 8 | Equipe e desequipe uma arma | `EQUIP_ITEM`, `MOVE_EQUIP_ITEM` |
| 9 | Sente e levante | `OBJECT_SIT_DOWN`, `OBJECT_STAND_UP` |

Os itens **3, 4 e 5** são os mais importantes da lista inteira: `NPC_INFO_00` e
`HOST_ATTACKRESULT` são dois dos treze que eu corrigi, e são os que fazem barra de vida e
número de dano aparecerem na tela.

### Parte 2 — grupo (precisa de dois personagens)

Se der para logar dois clientes ao mesmo tempo, mesmo que em contas diferentes:

| # | O que fazer | O que isso provoca |
| ---: | :--- | :--- |
| 10 | Convide o outro para o grupo | **`TEAM_LEADER_INVITE`** |
| 11 | Aceite o convite | **`TEAM_JOIN_TEAM`**, **`TEAM_MEMBER_DATA`** |
| 12 | Ande com os dois um pouco | `TEAM_MEMBER_DATA` de novo, com valores diferentes |
| 13 | Saia do grupo | **`TEAM_MEMBER_LEAVE`**, **`TEAM_LEAVE_PARTY`** |

O `TEAM_MEMBER_DATA` é o comando cujo layout eu descobri que estava errado por inteiro. É
de tamanho variável, então a captura precisa dele com **dois e com três membros** para eu
conseguir separar o cabeçalho do tamanho de cada membro — se der para ser três pessoas no
passo 12, melhor ainda; se não, dois já ajuda.

### Parte 3 — NPC e loja

| # | O que fazer | O que isso provoca |
| ---: | :--- | :--- |
| 14 | Fale com um NPC | `NPC_GREETING` |
| 15 | Compre alguma coisa dele | `SEVNPC_SERVE`, `OWN_ITEM_INFO` |
| 16 | Venda alguma coisa para ele | idem, na outra direção |
| 17 | Pegue e entregue uma missão | `TASK_NOTIFY`, `TASK_DATA` |
| 18 | Abra o banco/armazém | **`TRASHBOX_OPEN`**, **`TRASHBOX_WEALTH`** |

### Parte 4 — os que ainda estão em dúvida

Estes são os **14 que sobraram** na lista de divergências. Nenhum é chamado pelo nosso
código hoje, então não são urgentes — mas se for fácil provocar, é de graça agora e caro
depois:

| O que fazer | Comando |
| :--- | :--- |
| Voar (montar na espada/asa) e pousar | `FLYSWORD_TIME`, `OBJECT_TAKEOFF`, `OBJECT_LANDING` |
| Reparar equipamento com o ferreiro | `REPAIR`, `REPAIR_ALL` |
| Fabricar um item (artesanato) | `PRODUCE_START`, `PRODUCE_ONCE`, `PRODUCE_END` |
| Decompor um item | `DECOMPOSE_START`, `DECOMPOSE_END` |
| Cravar uma gema num equipamento | `EMBED_ITEM`, `CLEAR_TESSERA` |
| Duelar com outro jogador | `DUEL_PREPARE`, `HOST_DUEL_START`, `DUEL_RESULT` |
| Usar habilidade num monstro | `OBJECT_SKILL_ATTACK_RESULT` |
| Ficar com PK vermelho | `PARIAH_RISE` |

Se algum for trabalhoso, pule. A parte 1 sozinha já paga a sessão.

---

## Depois

Você não precisa mandar nada: eu puxo o arquivo da VM pelo SSH e rodo

```bash
cargo run -p pw-pcapdiff -- pw126.pcap --porta 29000
```

e sai uma tabela assim, uma linha por comando:

```
| id  | comando            | observado (bytes × vezes) | IR 1.5.3 | veredito              |
|  33 | NPC_INFO_00        | 12×847                    |       16 | difere: 12 bytes (-4) |
| 253 | PLAYER_CASH        | 4×3                       |        4 | igual ao 1.5.3        |
|  64 | TEAM_MEMBER_DATA   | 40×5, 74×2                |        — | tamanho variável      |
```

E aí cada divergência vira uma de três coisas, com número em vez de opinião: **mesmo
layout**, **layout menor e quanto**, ou **comando de tamanho variável**.

### O que a ferramenta responde, e o que não responde

**Responde:** quantos bytes cada comando teve naquele servidor.

**Não responde:** *qual* campo sumiu. Saber que o `NPC_INFO_00` do 1.2.6 tem 12 e não 16
bytes diz que um `int` a menos passou no fio — o candidato provável é o último campo
(`iTargetID`), mas provável não é verificado. Para fechar isso é preciso olhar os
**valores**, e é o passo seguinte: com a captura na mão dá para casar o que estava
acontecendo no jogo (o monstro com 55 de vida) com os bytes correspondentes.

Por isso vale, na hora de gravar, **anotar o que você estava fazendo e mais ou menos
quando**. Não precisa de precisão de segundo: "por volta de 2 minutos, bati num monstro
até matar" já é o suficiente para eu achar o trecho.

### Se der ruim

- **A ferramenta reclama de "buraco na remontagem"**: a captura perdeu pacote. Quase
  sempre é interface errada (tráfego passando duas vezes) ou disco lento. Grave de novo,
  numa sessão mais curta.
- **"nenhum segmento TCP na porta N"**: porta errada. Não deve acontecer — a porta foi
  confirmada em `ss -lntp` na VM (`glinkd` em `192.168.1.200:29000`).
- **A tabela sai com comandos que "não existem no IR"**: pode ser desalinhamento, mas pode
  ser um comando que só existe no 1.2.6, e esse seria um achado por si só.
