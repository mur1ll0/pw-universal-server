# Memória central da reforma do pw-admin

Atualizada em 2026-10-05 (B172). Estado: **login 126/155 corrigido nos dados em B173, falta ver em jogo (§6.1)**; **E1 auditada; E2 publicada/testada automaticamente,
parcial até inspeção visual; E3 com consultas/comando recuperável testados; E4 parcial (senha, criação e GM global coordenado)**.
B165–B170, sem commit. B167–B170 locais; migrações aplicadas somente no schema test.
Base consultada: commit `a010ff7`. Responsável pelas decisões de produto: Murillo.

Este arquivo é o ponto de retomada da reforma. Guarda decisões, estado das etapas,
evidências curtas e próximo passo. Specs descrevem o sistema implementado; o histórico
do projeto guarda os relatos detalhados. Não duplicar esses documentos aqui.

## 1. Objetivo e escopo aprovado

Reformar arquitetura, layout e integrações do painel para administrar os servidores PW
do mesmo ambiente Docker. Entregar operações efetivas no servidor e persistência correta,
com interface visual, interativa, em português, inspirada no jogo e com ícones dos dados
correspondentes ao realm. Pode substituir o frontend e reorganizar o backend.

### Decisões do Murillo

- **Instalação:** serviço do painel no mesmo Docker Compose dos realms e mundos. Pode
  haver serviços separados dentro do Compose; não exigir todos no mesmo contêiner.
  Descobrir os realms configurados e mostrar o estado real dos processos/mapas.
- **Versões:** começar por 1.2.6 e 1.5.5; admitir outros realms/versões conforme existirem
  no ambiente. Uma versão sem leitor/protocolo implementado aparece com limitações
  explícitas; detecção não significa suporte automático a formatos desconhecidos.
- **Acesso:** pela rede. Login com a mesma conta PW, exigindo privilégio GM válido.
  Todos os usuários autorizados do painel são administradores/GM; não foi solicitada
  uma hierarquia adicional de permissões. Reusar a validação de credenciais existente.
- **Contas:** criar/listar/buscar, mudar senha, conceder/remover gold, banir/desbanir,
  administrar privilégio GM e desconectar. Gold e banimento têm alcance global da conta
  entre realms; não criar saldo diferente por realm nesta reforma.
- **Histórico:** não construir histórico navegável de operações como funcionalidade.
  Logs técnicos e evidências de teste continuam necessários; não registrar senhas/tokens.
- **Personagens:** visualizar e editar atributos e pontos, nível, EXP, alma, cultivo,
  dinheiro, inventário/equipamentos, habilidades, missões, posição, aparência e mascotes.
  Manter unidades, limites e regras do original; validar layouts de cada versão.
- **Edição online:** implementar desde esta reforma. O servidor de jogo aplica a operação
  ao estado em memória, recalcula derivados, atualiza o cliente e coordena a persistência.
  O painel distingue aplicado no jogo, salvo e falha. Operações sem atualização online
  comprovada devem indicar a necessidade de reconexão ou ficar explicitamente indisponíveis.
- **Edição offline:** proteger contra login simultâneo; a autoridade de presença é o
  servidor, não uma consulta isolada do painel ao banco.
- **Mapas:** ligar/desligar por realm. Ao desligar, desconectar os jogadores do mapa,
  coordenar salvamento e impedir novas entradas. Não esperar voluntariamente ficar vazio.
- **Rates:** EXP, alma, drop e dinheiro, por realm; persistidos e efetivamente consumidos
  pelo mundo. Não incluir outros rates por suposição.
- **Moldes de classe:** editar por versão/realm as fontes realmente usadas na criação.
  Por padrão, mudanças afetam personagens novos; disponibilizar ação separada e explícita
  para recalcular os existentes. Definir tecnicamente quais campos podem ser recalculados
  sem apagar progressão ou personalizações; pedir decisão se a semântica for ambígua.
- **Editores futuros:** nesta reforma, somente preparar arquitetura para `elements`,
  `npcgen`, `tasks`, modelos, animações e texturas. Não implementar editores completos,
  launcher ou distribuição de patches como parte deste escopo.
- **Autonomia:** alterar painel e integrações necessárias em Rust, banco e Docker, mantendo
  as regras do projeto. Não parar a cada etapa para revisão; perguntar quando uma decisão
  de produto relevante estiver indefinida. Commit/push/publicação só por pedido explícito.
  Se um commit prévio parecer necessário, perguntar ao Murillo.

## 2. Evidência inicial e limitações

Auditoria e contratos em `docs/admin/ARQUITETURA_E_CONTRATOS.md` (E1). As linhas do antigo
`main.py` abaixo são da base `a010ff7`, anterior à substituição. Rotas pendentes agora exigem
login e retornam 501 sem mutação; não há seed de moldes nem escrita administrativa direta.

| fato consultado | origem | implicação |
| :--- | :--- | :--- |
| API FastAPI e frontend estático; painel sem revisão recente | `specs/06_ADMIN_PANEL_AND_CPW_SPEC.md`, §1 | auditar antes de escolher o que reaproveitar |
| Edição de ficha escreve diretamente no banco | `web-admin/backend/main.py:792` | inadequada para personagem online |
| Autosave copia estado em memória a cada 60 s | `crates/pw-gs/src/world.rs:2640` | banco isolado pode ser sobrescrito |
| Gravação do autosave fica fora do lock do mundo | `crates/pw-gs/src/world.rs:2710` | manter banco fora do caminho crítico e coordenar escritas |
| GM altera estado em memória e responde ao cliente | `crates/pw-gs/src/bus_server/gm.rs:120` | há operações existentes a aproveitar, não uma API admin pronta |
| Barramento atual transporta mensagens de jogo | `crates/pw-bus/src/message.rs:49` | canal administrativo ainda precisa de desenho e implementação |
| Rates/mapas/templates do painel não comandam corretamente os daemons | `specs/06_ADMIN_PANEL_AND_CPW_SPEC.md`, §1 | não confundir gravação de configuração com aplicação |
| Fontes dos moldes usados são arquivos do realm | `specs/06_ADMIN_PANEL_AND_CPW_SPEC.md`, §1 | resolver `ptemplate.conf`/`clsconfig`/banco conforme função de cada dado |

Referências acima são pontos de entrada; os números de linha podem mudar.
Auditoria E1 concluída. Docker consultado: quatro realms/mundos e painel anterior ativos.
E2 usa sessão Redis e verificador Rust compartilhado; catálogo não é exposto porque os
fallbacks podem buscar elements/ícones de outro realm (`elements_decoder.py:500-514,774-790`).
Há pendências anteriores de teste/publicação do jogo no estado geral; não assumir que o
contêiner executa o mesmo commit do código em disco.

## 3. Direção técnica e restrições

Fluxo recomendado: navegador → API autenticada → canal interno administrativo → daemon
responsável → operação no domínio → atualização do cliente e persistência coordenada.
O protocolo administrativo é separado dos layouts binários do cliente original.

- Não aceitar credencial ou comando administrativo apenas porque veio da rede Docker.
  Verificar autorização no backend e proteger o canal interno. Não publicar o barramento
  atual, que não autentica. Não transportar comandos pela falsa identidade de um jogador GM.
- Navegador não acessa banco, barramento nem Docker diretamente. Identificar explicitamente
  realm, mapa e personagem, evitando operações no alvo errado ao trocar a seleção.
- Escolher framework frontend/backend por evidência local e custo de integração. FastAPI
  pode continuar se benéfico; frontend e backend não precisam ter a mesma linguagem do GS.
  Leitores de dados existentes são referência; evitar decodificação duplicada divergente.
- Consultas online devem representar estado vivo; leituras offline representam persistência.
  Exibir claramente a origem e o estado online/offline, inclusive dados ainda não suportados.
- Comandos com resposta, identificador, validação e limite de espera. Evitar duplicação em
  novas tentativas e distinguir resultado desconhecido de falha definitiva.
- Coordenar alterações com autosave, logout, entrada e troca de mapa. Impedir que uma
  fotografia antiga sobrescreva estado recente; escolher ordenação/revisão de escrita
  adequada após examinar os fluxos existentes.
- Nenhuma espera de banco no `world.tick` ou mantendo o mundo trancado durante I/O.
  Limitar filas e duração de tarefas; falha do painel não pode congelar a simulação.
- Todo S2C novo/alterado passa pelo `WorldProtocol` da versão e pela evidência do original.
  Nada de condicionais de versão espalhados. Capacidades de administração e leitores
  podem usar registros/adaptadores; layouts de cliente continuam no `WorldProtocol`.
- Desligar mapa deve refletir ciclo de vida real, não só flag no banco. Se o mapa compartilhar
  processo com outros, não derrubar todos implicitamente. Explicitar falhas de salvamento
  e desligamento, preservando a consistência.
- Persistência de rates, moldes e configuração deve sobreviver a reinício. Definir quando
  cada alteração entra em vigor e mostrar se foi aplicada ou aguarda reinício.
- Preparar pontos de extensão futuros somente onde necessários: dados por versão, catálogo
  de recursos, capacidades e separação entre leitura, edição e aplicação. Não criar um
  sistema de plugins genérico ou telas simuladas sem necessidade.

## 4. Plano e critérios de aceite

Estados permitidos: `planejado`, `em andamento`, `parcial`, `testado`, `confirmado`, `bloqueado`.
`confirmado` exige relato do Murillo. Bloqueio registra motivo e ação necessária.
Cada etapa pode ser dividida em subtarefas aqui, mantendo o arquivo curto.

| etapa | estado | entrega e aceite |
| :--- | :--- | :--- |
| E1 — auditoria e arquitetura | testado | auditoria e fontes de verdade em `ARQUITETURA_E_CONTRATOS.md`; FastAPI mantido, UI modular sem CDN/framework, verificador compartilhado em Rust |
| E2 — base, login e interface | parcial | publicada em 2026-10-04; 22 testes Python + 24 Rust e 3 verificações Linux passaram; inspeção visual aguarda Murillo |
| E3 — canal administrativo e consulta viva | testado | consultas B166 e comando transacional/resultado recuperável B167; deduplicação global, conflito, rollback e reinício testados; falta confirmação visual |
| E4 — contas | parcial | busca, senha, criação e GM global recuperáveis; caches/efeitos GM coordenados; gold/ban/desban/desconexão pendentes; confirmar nos clientes originais |
| E5 — personagem e persistência | planejado | ficha e progressão online/offline, atualização nos dois clientes; autosave/logout/relogin não desfazem edição; consultas mostram valores reais |
| E6 — inventário e subsistemas | planejado | itens/equipamentos, habilidades, missões, posição, aparência e mascotes; sem duplicação de itens ou formatos inválidos; condições de reconexão explícitas |
| E7 — mapas e rates | planejado | desligar desconecta/salva jogadores e bloqueia entradas; religar restaura acesso; quatro rates alteram resultados reais e sobrevivem a reinício |
| E8 — moldes de classe | planejado | leitura/edição das fontes consumidas; personagem novo usa alteração; recálculo separado preserva progressão conforme contrato |
| E9 — extensões e fechamento | planejado | pontos de extensão documentados para editores futuros; interface revisada, manual atualizado, verificações finais e pendências claras |

Priorizar uma fatia completa em cada etapa: interface → API → daemon → persistência/cliente
quando aplicável. Não terminar com todas as telas prontas e operações sem integração.

## 5. Verificação e manutenção da memória

- Testes de integração com `TEST_DATABASE_URL`, isolamento no schema `test` e números
  de passou/falhou. Sem banco, não declarar integração validada.
- Cobrir riscos reais: autorização, separação de realms, presença durante login/logout,
  operação repetida, autosave antigo, concorrência, daemon indisponível e desligamento.
- Frontend: build/verificação adequados à tecnologia e inspeção visual de fluxos importantes.
  Não declarar teste visual ou em jogo que não foi realizado.
- Para mudanças visíveis no jogo, registrar ordem de teste, esperado na tela, log e overlay
  (`d_rtdebug` no 1.5.5). Distinguir teste automatizado de confirmação do Murillo.
- Antes de encerrar cada bloco ou compactar: atualizar etapa, decisão, arquivos alterados,
  evidência concisa, pendência e próximo passo. Referenciar detalhes por caminho/item B.
- Mudanças de comportamento atualizam as specs da área e o estado/histórico conforme
  `pw-atualizar-specs`; reservar próximo B sem reutilizar número. Memória não substitui isso.
- Manter este arquivo preferencialmente abaixo de 250 linhas. Resumir decisões concluídas,
  não acumular saídas de ferramentas. Comandos extensos sempre filtrados.

## 6. Retomada imediata

**Etapa ativa:** E4 parcial. Ajustes da revisão de rumo feitos em B173 (§6.3); falta o Murillo
confirmar o login em jogo antes da próxima fatia.
**B165–B171 (resumo; detalhes no histórico):** B165 E1 auditoria · B166 E2/E3 login do painel e
consulta viva · B167 senha + HMAC-MD5 no link · B168 criação de conta · B169 GM global coordenado
(revisão/recibos/fencing em 8 processos) · B170 saída/salvamento/troca do GS reescritos ·
B171 coordenação opcional (login sem a coluna, registro falho não aborta, 148/153 fora).

### 6.1 Incidente 2026-10-05 (B172): login recusado no 126 e no 155

Publicado B165–B171 com as migrações. Os dois links registram
`Login rejeitado: resposta de senha inválida` para `admin` e `testuser` (GM e não GM).
Causa: B167 passou a conferir a resposta do cliente por HMAC com chave `MD5(nome+senha)`
(`gameclient.cpp:131-139`), mas o `public.accounts` guarda `admin` como `MD5(senha)`
(`21232f…` = MD5("admin")) e `testuser` num formato que não é nenhum dos dois; antes do B167
o link não conferia senha. A própria memória avisava ("Argon2/texto/seeds incompatíveis
exigem redefinição explícita antes de publicar B167"), mas nada impedia a publicação.
Desligar a conferência foi bloqueado pela política de segurança da sessão.
**Resolvido em B173 (decisão do Murillo):** contas padrão `admin`/`admin` e
`testuser`/`testuser`, regravadas como `MD5(nome+senha)`; conferência de senha mantida.
As migrações também não tinham sido aplicadas no `public`: aplicadas em B173, com backup
`data/_backups/pw_database_2026-10-05_antes_das_migracoes_do_painel.sql`.

### 6.2 Revisão de rumo (análise Claude, 2026-10-05)

Diagnóstico, com evidência:
- **Infraestrutura à frente das funções.** 7 itens B para 4 operações de conta; E5–E9
  intactos. B169 (8 processos com fencing e `process::exit(1)`) e B170 (logout/salvamento)
  mexeram no caminho crítico do jogo jogável antes de qualquer teste em jogo.
- **Custo.** O B170 registrou 10,5 M tokens só na etapa final (histórico, item 170);
  cada rodada descobria uma corrida nova e abria outra camada.
- **Testes que não pegam o que importa.** A suíte roda no schema `test` (já migrado) e
  não detectou nem o `revisao_gm` ausente no `public` nem o formato de hash das contas
  reais. Dois arquivos de teste são intermitentes sob paralelismo
  (`comandos_recuperaveis.rs:513-519` conta locks do banco inteiro com prazo de 1 s;
  `canal_administrativo.rs:218-225` espera presença por 1 s).
- **Árvore sem commit** com 6+ blocos misturando painel e núcleo do GS/link.

Mudanças **aprovadas pelo Murillo em 2026-10-05** (estado em §6.3):
1. **Portão de publicação para o caminho crítico.** Toda mudança em login, seleção,
   entrada, saída, salvamento ou troca de mapa (link/GS) só avança para o próximo B depois
   de: (a) conferir compatibilidade com os dados do `public` (consulta só de leitura),
   (b) roteiro em jogo executado pelo Murillo nos dois clientes.
2. **Modelo de consistência suficiente:** online → comando ao GS dono, aplica em
   memória e deixa autosave/logout gravarem; offline → GS confirma ausência e o painel
   grava sob trava por personagem que recusa login enquanto durar; deduplicação só pelo
   id de comando (B167). Sem lease/época global por personagem. Corrida residual vira
   limitação documentada em uma linha.
3. **Remover `process::exit(1)`** ao perder o fencing (link e GS): perder a coordenação do
   painel não pode derrubar o jogo; desligar a coordenação e avisar no log.
4. **Senhas:** um único formato gravado — `MD5(nome+senha)` em minúsculas, o que o cliente
   usa — em criação, troca de senha e seeds; migração das contas existentes pela troca de
   senha do painel, nunca adivinhando senha.
5. **Testes:** esperas dos testes de corrida filtradas pela própria operação (não pelo
   banco inteiro) e com prazo maior; um teste que lê o `public` só para conferir colunas e
   formatos que o código novo exige.
6. **Ordem:** (i) login funcionando; (ii) commits de restauração separados (painel vs.
   núcleo GS/link); (iii) E4 simples: gold, ban/desban, desconectar; (iv) E7 rates/mapas;
   (v) E5 ficha online. Uma fatia por vez, com roteiro de tela ao fim.
7. **Limites de esforço:** duas rodadas de correção da mesma corrida → parar e perguntar;
   suíte inteira só no fim do bloco, em segundo plano, filtrada.

### 6.3 Estado dos ajustes (B173)

| item 6.2 | estado |
| :--- | :--- |
| 1 portão | `scripts/conferir_public_antes_de_publicar.sql` (leitura; vazio = ok) e passo na skill `pw-testar-e-publicar` |
| 2 consistência | aprovado; vale para E4–E6 |
| 3 sem `process::exit` | feito: perda do fencing desliga a coordenação (link `gateway.rs`, GS `mapas.rs`) |
| 4 formato de senha | já era `MD5(nome+senha)` no painel (`administracao.rs:218,248`); seed da spec 01 → testuser/testuser |
| 5 testes | `comandos_recuperaveis` em série (trava `SERIE`), esperas de 5 s; `canal_administrativo` espera 5 s |
| 6 ordem | (i) login: dados corrigidos, falta jogo · (ii) commits · (iii)–(v) a seguir |

**Próxima ação executável:** Murillo confirma o login e o roteiro de saída/salvamento em jogo;
depois E4 simples: gold, ban/desban, desconectar (uma fatia, roteiro de tela ao fim).
**Retomada:** próximo B174 (conferir antes); não publicar/commitar sem pedido.
