# Retomada e sincronização 126 — 2026-09-22

## Separação e merge

- `ccf7ae4`: codificadores/integração de combate e itens 126, testes e evidências.
- `36a88da`: mapa estrutural v55, scripts e transcrições; não contém leitor Rust.
- `0221911`: merge de multi-versions até `6322d88` na versao-126.
- Conflitos só em estado/histórico. B70–B76 publicados do 155 preservados;
  combate/itens 126 renumerados B77/B78, mapa v55 B79. Inventário/entrada
  históricos usam B73-126/B74-126 para distinguir os registros homônimos 155.
- `own_ext_prop` recebeu max_ap no merge. O teste da captura faltava o argumento:
  recebeu **0**, conforme os últimos quatro bytes de `evidencias/126/s2c-50.txt:11`.
  O override 126 escreve o parâmetro no último i32; não fixa zero em produção.
  `merge-layout-depois.log:26`: 19 testes de layout aprovados com banco.

## Leitor v55: o que foi entregue

A premissa de leitor implementado não se confirmou: `tasks.rs` não foi alterado
pela sessão separada. O relatório diz explicitamente que o Rust está pendente
(`RESULTADO_TASKS_V55.md`, seção Consequência para a implementação).
Foram modificados spec03, estado e histórico, além de novos scripts/evidências
em docs; esses caminhos extras foram relatados antes de prosseguir.

Repetido o validador Python: 2819 raízes, 7994 tarefas com filhas, profundidade 4,
fechamento em 20793663, sem sobra (`evidencias/126/pre-merge-v55.log:3`).
A medida pode orientar a implementação, mas não alimenta o mundo. Falta projetar
campos fixos/semântica em TaskTemplate e implementar/testar o Rust com corrupção.
O servidor continua aceitando v55 somente no cabeçalho. Não se declara missão
126 jogável; as listas internas de missão também continuam pendentes de medição.

## Cobertura e despacho depois do merge

O cenário geral de `subcomandos_no_mundo.rs` usa 155. A versão também é gravada
na linha de realm do banco de teste, sem divergência entre metadados e BusServer.
Compra e pickup têm cenários explícitos **155 e 126**, reutilizando a operação
real e as verificações de cobrança/persistência. Os gabaritos variam por formato.
Isto preserva testes de integração 126, além de layouts_do_126, combate_do_126 e
itens_do_126; apenas testes de codificador não detectariam um handler pulando o trait.
Os quatro cenários passam (`evidencias/126/mundo-duas-versoes.log:16`).

Duas chamadas trazidas pelo merge foram tratadas em etapa separada:
- Retirada de amuleto esgotado (`bus_server.rs:602`) usava PLAYER_DROP_ITEM comum
  de 11 B; passou a self.sub para o 126 receber 9 B. Causa/layout já medidos em
  `evidencias/126/s2c-46.txt:2` e `cliente-validacao-itens.txt:19`.
- ELF_EXP 283 (`jogo.rs:161`) não existe no 126: o validador VA 0x584618 limita
  ids a 260 (`cliente-validacao-entrada.txt`). `WorldProtocol::elf_exp` retorna
  Option: padrão Some com os mesmos bytes 155; v126 None. Ganho e persistência
  do Daimon não foram alterados. Teste vermelho antes, verde depois, incluindo
  gabarito literal 155 (`evidencias/126/elf-depois.log:15`, oito testes aprovados).

Regras comuns e contêineres 155 preservados. Não houve publicação, restart ou
rebuild. IDs 14/64 continuam fora do fechamento de layouts desta retomada.
Causas dos layouts anteriores: `COMBATE_126.md` e `ITENS_EXPERIENCIA_126.md`.

## Roteiro curto no cliente 126 (após publicação autorizada)

1. Entrar/relogar: propriedades e chi coerentes, inventário/atalhos presentes.
2. Atacar: animação, dano adiado, HP do alvo e experiência; conferir subida de nível.
3. Comprar uma unidade em NPC com preço disponível e pegar um drop: item/quantidade
   e moedas corretos. Os testes usam dados sintéticos; preços v7/C2S real ainda
   precisam de confirmação visual.
4. Se houver amuleto configurado no realm, esgotá-lo e verificar retirada do slot.
5. Não usar disponibilidade de missões como critério de aprovação: falta o leitor v55.

Filtro PowerShell:
`docker logs --since 5m pw-world-126 2>&1 | Select-String 'comprou|colheu|subiu para o nível|amuleto'`.
Logs confirmam operações, não recepção dos bytes. Para layouts, capture e leia com
`cargo run -p pw-pcapdiff -- <pcap> --interno --subcomando N`, N=24/26/31/36/46/72/99/144.
283 deve estar ausente no fluxo 126. Não há afirmação de teste visual nesta sessão.
