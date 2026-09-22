# Camada 3 — combate 126 (B75, 2026-09-21)

Base `a305e51`, branch `versao-126`. Sem publicação; Camada 4 não iniciada.

## Medida e correção

- Os ids 84/83/24/26/33 reproduzem as amostras do original byte a byte:
  payloads 7/1/10/11/12 bytes. Teste `combate_do_126.rs:23`.
- O 144 era emitido pelo escritor comum, com 19 bytes. A captura tem 15
  (`evidencias/126/s2c-144.txt:2`); o cliente confirma em VA `0x584af4`,
  retornando `0x0f` (`cliente-validacao-combate.txt:46`). A causa do descarte
  previsto é flag i32 e section extras. Não houve reprodução visual do sintoma.
- `WorldProtocol::host_skill_attacked` mantém o escritor anterior por padrão;
  `v126/mod.rs:130` sobrescreve flag de um byte, sem section. O mundo chama o
  trait em `bus_server.rs:2489`. Nenhuma regra de combate ou temporização foi alterada.
- Sentinela 155 usa flag `0x12345678` e section 3 para conferir todos os bytes
  preservados (`combate_do_126.rs:13`).

## Cadência, não apenas bytes

`medir_cadencia.py` complementa o pw-pcapdiff, que descarta timestamps.
Remonta TCP por sequência, rejeita buracos e sobreposições inconsistentes, usa o
instante em que o frame GNET ficou completo. Limita-se a PCAP clássico LE,
microssegundos, Ethernet/IPv4 sem fragmentação. Script e relatório em `evidencias/126/`.
As contagens 23/24/26/33/38/83/84/144 coincidem com pw-pcapdiff:
29/52/25/80/267/71/29/1. SHA256 do PCAP no relatório.

Para 22 ticks, 23 intervalos entre resultados dentro da mesma sessão:
**mínimo 1049,265 ms, mediana 1149,332 ms, máximo 1199,262 ms**.
Há sessões anunciadas com 28 ticks, mas sem dois resultados consecutivos nesta
amostra; não inferir intervalo para elas. Entre ATTACK_ONCE e HOST_ATTACKRESULT:
52 pares, mediana **49,3695 ms**, extremos 1,897 e 61,987 ms.

Primeira sessão (`cadencia-original.md:12`): 84 e 83 em 2382,753028 s;
24 em 2382,802562 s, dano 7 e speed 16. A vida comunicada do monstro cai
29→22 em 2384,453169 s (`:19`), depois 22→17 em 2385,452133 s.
O envio periódico de HP não permite datar exatamente o impacto só pelo PCAP.

Regra do original: `actsession.cpp:367-378` abre o golpe e agenda a repetição;
`playertemplate.h:980` calcula atraso `(attack_speed*0.8)-1` ticks;
`actobject.cpp:1758-1776` adia a aplicação do dano. Para 22 ticks resulta 16
(800 ms), como no pacote. O mundo já mantém sessão e dano adiado. A diferença
entre ticks nominais e recepção no fio está medida, **não** declarada eliminada;
não foi acrescentado sleep artificial nem certificada igualdade temporal em jogo.

## Validação focada

Com TEST_DATABASE_URL definido:
- `cargo test -p pw-protocol --test combate_do_126`: **3/3**; o teste 144
  falhou antes (19 contra 15 bytes) e passou depois; sentinela 155 preservada.
- Três filtros de `subcomandos_no_mundo`: dano adiado do monstro, novo clique
  sem golpe extra, habilidade contra jogador: **3/3**, 61 filtrados.

Sem suíte completa, rebuild, reinício ou commit nesta etapa. A falha de missão
registrada em B74 continua fora do escopo; não se declara regressão global aprovada.

## Roteiro de jogo após publicação autorizada

1. Selecionar monstro: conferir HP; atacar e repetir cliques sem golpes extras.
2. Observar o dano após a animação e a atualização periódica da barra do alvo.
3. Receber habilidade de outro jogador: efeito/dano devem aparecer, sem
   `Invalid HOST_SKILL_ATTACKED` no overlay `d_rtdebug 1`.

Filtro estreito: `docker logs --since 5m pw-world-126 2>&1 | Select-String
'ataque|golpe|dano de|host_skill'`. Próximo passo: aprovação/teste visual;
Camada 4 somente em nova etapa autorizada.
