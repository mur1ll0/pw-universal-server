# O handshake do 1.2.6, medido

O que um servidor 1.2.6 de verdade e um cliente 1.2.6 de verdade trocam entre si, lido dos
bytes. Serve de gabarito para o nosso `pw-link` — e foi lendo isto que apareceu a causa de o
cliente 1.2.6 ficar parado em "Conectando ao jogo".

## De onde vêm os bytes

`capturas/t2_externo.pcap`, elo cliente ↔ `glinkd` na porta 29000, capturado dentro da VM do
servidor. Só a parte em claro é legível: o link vira ARCFOUR logo depois da troca de chaves,
e a chave não sai da captura.

```
servidor → cliente:  1(22)   3(18)   [daqui em diante, cifrado]
cliente  → servidor:         2(23)   [idem]
```

Os números entre parênteses são bytes de payload.

## Os três pacotes

### `1` — Challenge (servidor → cliente), 22 bytes

```
10 | 00 00 00 d0 00 00 00 00 1b 01 af e0 6a da 46 4c | 00 01 02 06 | 00
^    ^                                                 ^             ^
|    nonce de 16 bytes                                 |             algo = 0
CompactUINT 16                                         version = 0x00010206
```

O `version` confirma o `0x00010206` que já estava no código, e o `algo = 0` confirma o que
mandamos. **O 1.2.6 não traz `edition` nem `exp_rate`** — o pacote acaba no `algo`, o que
também já estava certo.

O teste `o_challenge_que_mandamos_bate_byte_a_byte_com_o_do_servidor_real` compara a nossa
saída com estes 22 bytes exatos.

### `2` — Response (cliente → servidor), 23 bytes

```
05 74 65 73 74 65 | 10 75 ca 09 65 35 96 76 97 bf a0 72 0a 16 fd 65 a5
^  "teste"          ^  resumo da senha, 16 bytes
Octets(5)           Octets(16)
```

**Este é o login.** O nome de usuário viaja em claro, seguido do resumo de 16 bytes. Não há
`use_token` nem `cli_fingerprint`: o pacote acaba aí, e são justamente os dois campos que o
1.5.3 acrescenta.

### `3` — KeyExchange (servidor → cliente), 18 bytes

```
10 1c 49 52 52 52 7f 4a ea 67 c0 aa 5c c0 9c 9b 90 | 00
^  nonce de 16 bytes                                 blkickuser = 0
```

Os campos são exatamente os que o IR do 1.5.3 dá para o `KeyExchange`: `{nonce, blkickuser}`.

## A descoberta: os dois opcodes trocam de número

| Protocolo | 1.2.6 (medido) | 1.5.3 (IR) |
| :--- | ---: | ---: |
| `Response` | **2** | 3 |
| `KeyExchange` | **3** | 2 |

Não é ambíguo: o pacote 2 do 1.2.6 traz o nome do usuário em claro, coisa que uma troca de
chaves não faz, e o 3 tem os dois campos do `KeyExchange` e nada mais.

### O que isso causava

O nosso `codec.rs` usava a numeração do 1.5.3 para os três realms. Então, num realm 1.2.6:

1. o cliente mandava o `Response` no opcode 2;
2. o servidor decodificava aquilo como `KeyExchange`;
3. o tratador do `KeyExchange` escreve **uma linha de log** e não responde nada;
4. o login nunca acontecia. Nenhum erro, dos dois lados — o cliente ficava em "Conectando ao
   jogo" até a conexão morrer, e o `EC.log` registrava só `Active close`.

O conserto está em `GameVersion::opcode_response` e `opcode_key_exchange`, com os cinco
testes de `crates/pw-protocol/tests/handshake_do_126.rs` presos a estes bytes.

O 1.4.8 herda os números do 1.5.3 **por falta de medição**, e está anotado como tal — a mesma
política dos layouts de gamedata.

## O que a cifra implica (e por que não foi mexida)

Depois do `KeyExchange` os dois lados passam a ARCFOUR e a captura fica ilegível. Mas o
nosso servidor **nunca manda `KeyExchange`**: ele responde o `Response` com o
`OnlineAnnounce` e segue em claro. Pelo código do cliente 1.5.3
(`EC_GameSession.cpp:4097`), a cifra só é montada dentro do `OnPrtcKeyExchange` — ou seja,
**um cliente que não recebe `KeyExchange` não liga a cifra**, e o elo continua legível dos
dois lados.

É o que já funciona hoje no 1.5.3 e é o caminho mais curto para o 1.2.6 funcionar também. O
dia em que a cifra for necessária (para falar com um `glinkd` original, por exemplo), o que
falta medir é o `KeyExchange::Setup` — que **não está** nos fontes vazados.

## O que mais a captura interna mostrou

Da mesma sessão, o elo `glinkd ↔ gdeliveryd` (porta 29100, sempre em claro) confirmou o
`RoleInfo` do 1.2.6 campo a campo, num `RoleList_Re` real de 364 bytes:

```
roleid=48  gender=0  race=2  occupation=4  level=2  level2=0
name="POTATO"  custom_data=172 bytes  equipamento=1 item
status=1  delete_time=0  create_time=1788666993  lastlogin_time=1788676292
posx=-1440.22  posy=240.29  posz=1397.14  worldtag=1
custom_status=vazio  charactermode=vazio        (e acaba aqui)
```

São os mesmos 19 campos que o nosso `write_role_info` escreve para o 1.2.6, na mesma ordem,
com o item interno trazendo os dez campos do `GRoleInventory` completos. **O `RoleInfo` já
estava certo** — o que é uma informação valiosa, porque era o outro suspeito da falha.

O `RoleList_Re` interno também bate com o nosso: `result`, `handle` (`ffffffff` quando não
há), `userid`, `localsid`, e então o vetor.

## Como reproduzir a leitura

```bash
cargo run -p pw-pcapdiff -- capturas/t2_externo.pcap  --porta 29000 --sequencia
cargo run -p pw-pcapdiff -- capturas/t2_externo.pcap  --porta 29000 --despejar 1
cargo run -p pw-pcapdiff -- capturas/full_interno.pcap --porta 29100 --despejar 83
```

`--sequencia` mostra os quadros na ordem, `--quadros` agrupa por opcode e `--despejar N`
imprime o payload cru de um opcode em hexadecimal. Os três modos foram acrescentados para
esta investigação e ficam para a próxima.
