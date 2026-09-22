# Experiência e pacotes de itens 126 — B78

Estado: testado localmente em 2026-09-21, base a305e51 + B77/B78, sem publicação.
Escopo: codificação S2C; não declara missões, loja ou progressão inteiras jogáveis.

| id / comando | payload 126 | payload padrão 155 | evidência original |
|---|---|---|---|
| 31 PICKUP_ITEM | 14 B: tid/validade i32, quantidades u16, pacote/slot u8 | 18 B, quantidades u32 | evidencias/126/s2c-31.txt:2; cliente-validacao-itens.txt:1 |
| 36 RECEIVE_EXP | 4 B: exp/SP u16 | 8 B: exp/SP i32 | s2c-36.txt:2; cliente-validacao-itens.txt:10 |
| 46 PLAYER_DROP_ITEM | 9 B: pacote/slot u8, quantidade u16, tid i32, tipo u8 | 11 B, quantidade u32 | s2c-46.txt:2; cliente-validacao-itens.txt:19 |
| 72 PURCHASE_ITEM | 7 + 13×n B: sem yinpiao, quantidade u16 | 11 + 15×n B | s2c-72.txt:2; cliente-validacao-itens.txt:28 |
| 99 HOST_OBTAIN_ITEM | 14 B, igual 31 | 18 B | s2c-99.txt:2; cliente-validacao-itens.txt:38 |
| 156 TASK_DELIVER_ITEM | 10 B: sem validade, quantidades u16 | 18 B | s2c-156.txt:2; cliente-validacao-itens.txt:47 |
| 158 TASK_DELIVER_EXP | 8 B: exp/SP i32 | 8 B, comum | s2c-158.txt:2; cliente-validacao-itens.txt:56 |

Comprimentos excluem o id u16. Capturas são de `full_interno.pcap` do original,
extraídas com `cargo run -p pw-pcapdiff -- <pcap> --interno --subcomando <id>`.
O validador do cliente é VA 0x584610; 72 lê n em +5 e exige 7+13*n.
Hash do executável: `cliente-validacao-entrada.txt:1`.
As amostras comprovam os valores usados nos gabaritos, não todas as combinações.

Causa dos pacotes incompatíveis: `jogo.rs` chamava diretamente os escritores comuns
com contadores u32; 72 também acrescentava yinpiao e 156 acrescentava validade.
O validador 126 exige comprimentos menores e descarta os demais.
`WorldProtocol` agora oferece cinco métodos com padrão delegado ao escritor 155
(`traits.rs:101`); somente `versions/v126/mod.rs:193` redefine os bytes.
`Contexto.sub` recebe a estratégia do servidor (`jogo.rs:305`) e atende prêmios,
coleta, drop, compra e consumo. Sem condição de versão nem mudança de regra de jogo.
A regra de entrega continua `player.cpp:8996` e compra `player.cpp:8900-8932`
(C++ 155, autoridade para regra, não layout 126).
36 já passava pelo trait (`v126/mod.rs:183`); 158 continua no escritor comum.
Quantidades acima de u16 saturam no codificador 126, seguindo a convenção existente
em EQUIP_ITEM; não foram validadas em jogo. Não houve mudança de limites da bolsa.

Validação com TEST_DATABASE_URL definido:
- `cargo test -p pw-protocol --test itens_do_126`: antes 2 aprovados/5 falhas;
  depois **7 aprovados/0 falhas** (`evidencias/126/itens-depois.log:14`).
- `cargo test -p pw-gs --test subcomandos_no_mundo -- comprar_do_npc_tira_dinheiro_e_da_o_item pegar_item_de_missao_vai_para_bolsa_de_missao --test-threads=1`:
  **2 aprovados/0 falhas**, 62 filtrados (`itens-mundo-depois.log:11`).
- Os dois cenários de mundo são v126 (`subcomandos_no_mundo.rs:338`), mas esperavam
  offsets 155. Atualizados para a captura, mantendo cobrança, persistência do item
  e bolsa de missão; acrescentadas verificações de tamanho e quantidade.
- Sentinelas literais 155 cobrem validade, quantidades acima de 65535 e experiência.
  Não se declara regressão global: suíte completa proibida nesta sessão.

Bloqueio separado: tasks.data 126 tem versão 55 e 2819 entradas de topo;
`tasks.rs:1139` retorna só cabeçalho para versão diferente de 129. O leitor v55
não foi alterado. Roteiro da próxima sessão em `PROMPT_TASKS_V55.md`.
A compatibilidade interna das listas TASK_DATA/TASK_VAR_DATA ainda exige medição.
Também não foram medidos aqui C2S de compra nem disponibilidade dos preços v7.

Roteiro em jogo, depois de publicação explicitamente autorizada:
1. Abater um monstro; conferir experiência/alma e eventual subida de nível.
2. Pegar drop e conferir quantidade/slot; coletar material, quando disponível.
3. Comprar uma unidade em NPC com preço carregado; conferir item e moedas.
4. Prêmios de missão ficam pendentes do leitor v55 e da validação de listas.
Filtro estreito: `docker logs --since 5m pw-world-126 2>&1 | Select-String 'comprou|colheu|subiu para o nível'`.
Esses logs confirmam operações, não bytes recebidos pelo cliente; para 31/36/46/
72/99/156/158 confirmar payload com captura/pcapdiff. Ausência de subida de nível
não prova ausência de ganho de experiência. Nenhum contêiner foi reconstruído.
