# Mapa validado — `tasks.data` v55 (1.2.6)

Data: 2026-09-22. Escopo: medir o leitor do cliente original e validar a travessia
estrutural do arquivo; **nenhum** comportamento do `pw-data-loader` foi alterado nesta
etapa.

## Resultado

`data/realm_126/config/tasks.data` tem 20.793.663 bytes, SHA-256
`ee042d417452cd26e076280fd2f8d0d05bda1c63b1777abfc6c7d5f8d7ca8017`, magic
`0x93858361`, versão 55 e 2.819 raízes. A tabela ocupa `[12, 11288)`; a primeira raiz
começa em 11288 e a última em 20775057.

O validador reproduzível percorreu **todas as 2.819 raízes**, incluindo suas filhas:
**7.994 tarefas**, profundidade máxima **4**, e cada raiz terminou exatamente no próximo
offset da tabela (a última, exatamente no byte 20.793.663). Resultado bruto:
`docs/evidencias/126/tasks-v55-validacao-contagens.txt`; validador:
`docs/evidencias/126/validar_tasks_v55.py`.

## Âncora no binário original

O `elementclient.exe` v126, SHA-256
`5fc88d47e01da3caea7d6ce4911b71b0f7085060a2d13889a380fc5ba7f0ed14`, valida o cabeçalho,
percorre a tabela em `LoadTasksFromPack` VA `0x630c10` e chama o leitor recursivo
`LoadBinary` VA `0x62f6c0`. Este lê `0x216` (= **534**) bytes de dado fixo em
`0x62d04d`; a sequência completa, inclusive diálogos e filhos, está em
`tasks-v55-loadbinary-full.asm.txt`. A fonte C++ 1.5.5 foi usada apenas para dar nome aos
campos, nunca para assumir tamanho/offset v55.

## Tabela completa das contagens variáveis

Todos os offsets abaixo são relativos ao bloco fixo de 534 B. As leituras ocorrem nesta
ordem após o bloco; `u32` é little-endian e os contadores dos diálogos/filhos são `i32`
não negativos.

| Ordem | Seção | Contador/origem | Elemento e total lido | Prova no cliente |
| :-- | :-- | :-- | :-- | :-- |
| 1 | assinatura | `fixo[0x40]` | se verdadeiro, 60 B (`30 × u16`) | `0x62d04d`–`0x62d0ad` |
| 2 | janelas de horário | `u32 fixo[0x4e]` | por entrada, início + fim: `2 × task_tm`, 48 B | `0x62d0b2`–`0x62d1a0` |
| 3 | itens prévios | `u32 fixo[0xca]` | `ITEM_WANTED` v55, 13 B | `0x62d28a`–`0x62d2d3` |
| 4 | itens dados | `u32 fixo[0xd3]` | `ITEM_WANTED` v55, 13 B | `0x62d31b`–`0x62d377` |
| 5 | membros de equipe | só se `fixo[0x176]`; `u32 fixo[0x191]` | `TEAM_MEM_WANTED` v55, 32 B | `0x62d379`–`0x62d3e7` |
| 6 | monstros pedidos | `u32 fixo[0x1a2]` | `MONSTER_WANTED` v55, 22 B | `0x62d3e7`–`0x62d450` |
| 7 | itens pedidos | `u32 fixo[0x1aa]` | `ITEM_WANTED` v55, 13 B | `0x62d450`–`0x62d4b7` |
| 8 | prêmio de sucesso | fixo, seguido por `u32 award[0x43]` candidatos | `AWARD_DATA` v55, 75 B; candidato: bool + `u32` itens + `13 B × itens` | `0x62d4f5`–`0x62d5af`, `0x62d9d0`–`0x62da88` |
| 9 | prêmio de falha | igual ao anterior | `AWARD_DATA` v55, 75 B e candidatos variáveis | `0x62d5ed`–`0x62d6a7` |
| 10 | escala de razão (sucesso/falha) | cada uma: `i32 n` no fluxo | 5 limites `u32` (20 B) + `n ×` prêmio completo | `0x62d6b7`–`0x62d89b` |
| 11 | escala de itens (sucesso/falha) | cada uma: `i32 n` no fluxo | id do item (4 B) + 5 limites (20 B) + `n ×` prêmio completo | `0x62d8b6`–`0x62d9b4` |
| 12 | textos extensos | quatro `i32 caracteres` no fluxo | cada um: contador + `2 B × caracteres` (UTF-16 XOR pelo id) | `0x62fc60`–`0x62fe69` |
| 13 | cinco diálogos | cada: id (4 B), texto fixo (128 B), `i32 janelas` | janela: id+pai (8 B), `i32` chars, UTF-16, `i32` opções, `136 B × opções` | `0x62f701`–`0x62faf0`, `0x611be0`–`0x611d5a` |
| 14 | submissões | `i32` após os diálogos | cada filha é uma tarefa completa, recursiva | `0x62faf1`–`0x62fb72` |

Os quatro textos são descrição, sucesso, falha e tributo. O registro de opção de diálogo
é 136 B (`id u32`, `text[64]` UTF-16, `param u32`). O `AWARD_ITEMS_CAND` é variável: a
rotina v55 lê primeiro o bool `m_bRandChoose`, depois `m_ulAwardItems` e então os itens;
os demais membros existentes em memória não estão no arquivo.

## Consequência para a implementação

O mapa agora é seguro para começar o despachante v55: ele deve usar esses tamanhos e exigir
o mesmo fechamento por offset para rejeitar corrupção. Ainda falta mapear quais campos fixos
serão projetados em `TaskTemplate` e implementar/testar o leitor Rust; até isso acontecer,
`tasks.rs` deliberadamente continua aceitando v55 só no cabeçalho. Não houve teste Cargo,
contêiner, publicação ou commit neste bloco de evidência.
