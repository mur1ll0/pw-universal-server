# Prompt completo — leitor tasks.data v55 em sessão separada

Copie o bloco abaixo para a sessão com modelo mais barato. Ao terminar, cole aqui
somente o relatório no formato do fim deste arquivo; deixe provas extensas em disco.

---

Você vai implementar exclusivamente o leitor de `tasks.data` versão 55 do Perfect
World 1.2.6, preservando o leitor v129 do 1.5.5. Não implemente rede ou motor de missões.
Idioma: português. Não use subagentes, salvo pedido explícito posterior.

## Ambiente e proteção do trabalho existente

- Repositório original: `F:/Python_C_Projects/PWSource1.5.3/pw-universal-server`.
  Contém trabalho de outra sessão. Não edite, reverta, limpe ou commite nele.
- Trabalhe na worktree já existente `F:/Python_C_Projects/PWSource1.5.3/pw-126`,
  branch `versao-126`. Rode `git status -s` e `git log -n 1 --oneline` primeiro.
  Último HEAD conhecido: a305e51. B75/B76 podem estar sem commit: preserve tudo.
  Não recrie worktree/branch nem aplique reset/stash/clean. Não execute em paralelo
  com outra sessão escrevendo nessa worktree. Se a situação mudou, relate antes
  de sobrescrever qualquer arquivo.
- `data` nessa worktree é junction para dados do original: apenas leia os arquivos.
- Não toque nos contêineres 155/126, não publique, não reconstrua, não reinicie,
  não faça commit/push. Encerre após implementação, testes focados e documentação.

## Retomada mínima

Use `pw-retomar-sessao` e `pw-dados-do-realm`; leia AGENTS.md, estado §0/§3.3/§5,
`specs/README.md` e apenas `specs/03_DATA_LOADER_SPEC.md` §3.2 e referências necessárias.
Leia `docs/ITENS_EXPERIENCIA_126.md` para limites da etapa anterior.
Não leia o histórico nem o master de engenharia reversa inteiros.

## Fatos medidos e fontes

- Entrada: `data/realm_126/config/tasks.data`, 20.793.663 bytes.
- Cabeçalho LE: magic 2475000673, version 55, item_count 2819 (raízes, não total recursivo).
- SHA256: `ee042d417452cd26e076280fd2f8d0d05bda1c63b1777abfc6c7d5f8d7ca8017`.
  Confirme tamanho/hash antes de reutilizar offsets.
- `crates/pw-data-loader/src/tasks.rs:76` fixa versão suportada 129; perto de :1139
  `load_from_bytes` retorna só o cabeçalho para outra versão. Não basta remover a guarda:
  os blocos fixos e variáveis v55 precisam ser medidos independentemente.
- Juiz dos layouts: carregador do cliente original em
  `F:/Games/perfectworld_126/element/elementclient.exe`; registre SHA256, VA/RVA,
  instruções e offsets de cada conclusão em arquivo de evidência com linhas.
- Fonte C++ `F:/PW/1.5.5/EvolvedPWServer/cgame/gs` só prova semântica/regra;
  nunca use seus offsets v125/129 como layout v55. Consulte também referências
  locais existentes quando comprovadamente da versão 126.
- PCAP original: `F:/Python_C_Projects/PWSource1.5.3/pw-universal-server/_sync/capturas/`.
  Só consulte para corroborar IDs/objetivos/recompensas, se necessário:
  `cargo run -p pw-pcapdiff -- <pcap> --interno --subcomando N`.
  PCAP não substitui o carregador como prova de layout do arquivo de dados.

## Sequência de execução

1. Leia o leitor atual e seus testes. Localize no binário 126 o carregador
   `ATaskTempl`/equivalente de LoadBinary e o fluxo de leitura v55. Crie uma tabela
   offset/tamanho/tipo/semântica/evidência. Meça bloco fixo, contagens, seções
   variáveis, textos, diálogos, prêmios, submissões e ordem de leitura.
2. Confirme a tabela de offsets das 2819 raízes. Trace poucas missões simples e
   outras com filhas/prêmios/objetivos antes de generalizar. Não invente campos,
   não busque padrões para pular silenciosamente regiões desconhecidas e não
   aceite “chegou no EOF” sem fechar cada registro no offset seguinte.
3. Antes de cada correção escreva o menor teste que falha pela hipótese medida.
   Implemente leitor v55 isolado dentro de pw-data-loader (módulo próprio se ajudar),
   despachado pela versão do arquivo. Preserve a API/modelo existente quando
   comportar a semântica; documente expressamente qualquer campo v55 ausente.
   Não fabrique valores v129 para campos desconhecidos. Versões não suportadas
   continuam com o comportamento atual. Formato de arquivo varia no loader;
   layouts de rede continuam exclusivamente nos overrides WorldProtocol/v126.
4. Converta o que o motor já consome: IDs/nome/hierarquia, níveis/classes/gênero,
   pré-requisitos, NPCs, objetivos, itens exigidos/entregues e prêmios disponíveis
   no formato. Atravesse com validação também as seções não expostas no modelo.
   Recuse truncamento, offsets inválidos, contagens incompatíveis e desalinhamento
   com erro que identifique offset/registro. Não adicione flags de versão no pw-gs.
5. Rode SOMENTE testes focados do loader. Teste o arquivo v55 real, os limites
   entre registros e corrupção representativa; mantenha verificação do v129 real
   e seus valores conhecidos. Não altere testes v129 para acomodar regressão.
6. Atualize apenas parágrafos afetados da spec03, estado e próximo item livre no
   histórico (B77 se ainda livre; não renumere). Guarde mapa v55 e provas em
   `specs/tasks_126/` e/ou `docs/evidencias/126/`. Deixe relatório curto em
   `docs/RESULTADO_TASKS_V55.md`. Use `pw-atualizar-specs` ao fechar.

## Testes, terminal e orçamento

No PowerShell, defina antes de qualquer teste:
`$env:TEST_DATABASE_URL='postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database'`.
Nunca rode `cargo test --workspace`. Use `cargo test -p pw-data-loader` com
`--test <arquivo específico>` ou filtro unitário realmente restrito. Redirecione
stdout/stderr para arquivo; imprima apenas `test result:`. Se falhar, leia no máximo
as últimas 20 linhas. Não despeje binário, código inteiro, compilação ou logs.
Informe tokens de entrada/saída/total e método de medição; se indisponíveis,
identifique estimativa como tal. A cada etapa: hipótese / arquivo:linha / próximo teste.
Se a engenharia reversa exceder o orçamento combinado, entregue mapa parcial
reproduzível e bloqueio preciso; não declare leitor completo nem adivinhe layout.

## Critérios de aceite

- 2819 raízes do arquivo conhecido lidas e total recursivo explicitamente medido.
- Cada raiz fecha no próximo offset e a última no limite real do arquivo; se houver
  trailer, provar seu formato no carregador em vez de ignorar bytes.
- Seções variáveis/submissões consumidas com bounds checking, erros úteis em corrupção.
- Ao menos três missões de tipos distintos conferidas independentemente no cliente/
  carregador: IDs, pré-requisitos, objetivo e prêmio; valores reais, não placeholders.
- Arquivo v129 preserva contagem e valores dos testes existentes, sem regressão focal.
- Não declarar missões jogáveis: listas TASK_DATA/TASK_VAR_DATA e integração em jogo
  ficam para a retomada. B76 só corrigiu pacotes de itens/experiência.

## Resposta esperada para colar na sessão principal

Preencha com resultados reais; não copie números esperados como se fossem medidos:

```text
Resultado: completo | parcial | bloqueado (uma frase e causa).
Branch/HEAD: ...; mudanças anteriores B75/B76 preservadas: ...
Arquivo v55: caminho, bytes, SHA256, magic/versão/raízes.
Fechamento: raízes lidas/2819; total com filhas; último offset; sobra; erros.
Layout: arquivo:linha do mapa + VA/RVA/hash do cliente para bloco fixo,
        variáveis, prêmios, diálogos e recursão.
Missões conferidas: três IDs + tipo + pré-requisitos/objetivo/prêmio + prova independente.
Código: caminhos e linhas alterados; escolha de despacho v55/v129; campos sem suporte.
Testes: comandos exatos com filtro; banco configurado; aprovados/falhas/ignorados;
        logs em arquivo; resultado comparativo v129 real; casos de corrupção.
Documentação: spec03, estado, Bxx e relatório (caminho:linha).
Git: diff --stat somente das mudanças desta sessão; sem commit/publicação.
Pendências: limitações, próximo teste e o que a sessão principal deve verificar.
Tokens: entrada, saída, total e acumulado, método/corte; ou estimativa identificada.
```

Anexe ao relatório os caminhos de provas e scripts reproduzíveis; não cole logs
extensos nem o arquivo de dados. Pare para revisão ao terminar.
