# Memória central da reforma do pw-admin

Atualizada em 2026-10-07 (B198). Estado: **E1–E5 testadas; E6 em andamento (tela nova B190,
arrastar B191, dica parte 2 B192–B193, editar item B194, habilidades B195–B196, mascotes B197, missões B198; fila B199 em §6.14); E7 testada; E8–E9 planejadas**.
Commitado até `c541bcb` (B198). Nada de B176–B198 confirmado em jogo.
Responsável pelas decisões de produto: Murillo.

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
- **Tela de personagens (2026-10-07):** cartões só do realm selecionado (todas as contas);
  tela do personagem com janelas como as do jogo; estatísticas editáveis = as de hoje (nada
  novo; tirar dinheiro/EXP continua proibido). Ação online sem S2C comprovado: recusar
  ("precisa estar offline") **e** oferecer botão explícito "desconectar e aplicar". Edição de
  item livre nos **valores**, não no **formato** (octets legíveis pelo cliente). 1.2.6 sem
  reenvio de lista de habilidades aceito em jogo → remover/descer habilidade só offline.
- **Autonomia:** alterar painel e integrações necessárias em Rust, banco e Docker, mantendo
  as regras do projeto. Não parar a cada etapa para revisão; perguntar quando uma decisão
  de produto relevante estiver indefinida. Commit/push/publicação só por pedido explícito.
  Se um commit prévio parecer necessário, perguntar ao Murillo.

## 2. Evidência inicial e limitações

Auditoria E1 concluída: fatos, origens e implicações em `docs/admin/ARQUITETURA_E_CONTRATOS.md`
(edição antiga direto no banco, autosave de 60 s fora do lock, barramento sem autenticação,
rates/mapas/moldes sem comando real). Rotas pendentes exigem login e retornam 501 sem mutação.
Não assumir que o contêiner executa o mesmo commit do código em disco.

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
| E4 — contas | testado | busca, criação, senha, GM, gold, ban/desban e desconectar (B175); testes Rust/Python/Node; falta o Murillo confirmar em jogo |
| E5 — personagem e persistência | testado | dinheiro e EXP/SP (B179); pontos livres, nível e cultivo (B182); modificar/redistribuir atributos (B184); falta ver em jogo; ficha e progressão online/offline, atualização nos dois clientes; autosave/logout/relogin não desfazem edição; consultas mostram valores reais |
| E6 — inventário e subsistemas | em andamento | posição (B185), itens ver/buscar/dar (B186), remover (B187), ícones (B188), dica parte 1 (B189), tela nova (B190) e arrastar (B191), dica parte 2 (B192–B193), editar item (B194), habilidades ver e editar (B195–B196), mascotes (B197), missões (B198) testados; itens/equipamentos, habilidades, missões, posição, aparência e mascotes; sem duplicação de itens ou formatos inválidos; condições de reconexão explícitas |
| E7 — mapas e rates | testado | rates (B176), mapas (B177) e lista completa com carga/descarga em execução (B183); falta ver em jogo; desligar desconecta/salva jogadores e bloqueia entradas; religar restaura acesso; quatro rates alteram resultados reais e sobrevivem a reinício |
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

**Etapa ativa:** E6 em andamento (B185 posição, B186–B189 itens, B190 tela nova); E5 testada.
**B165–B171 (resumo; detalhes no histórico):** B165 E1 auditoria · B166 E2/E3 login do painel e
consulta viva · B167 senha + HMAC-MD5 no link · B168 criação de conta · B169 GM global coordenado
(revisão/recibos/fencing em 8 processos) · B170 saída/salvamento/troca do GS reescritos ·
B171 coordenação opcional (login sem a coluna, registro falho não aborta, 148/153 fora).

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

### 6.3–6.13 Blocos fechados (B173–B189; regras nas specs 05 §7.9–7.11 e 06)

B173 ajustes da §6.2 (sem `process::exit`, senha só `MD5(nome+senha)`). B174–B177 interface
própria, E4 (gold, ban, desconectar) e E7 (rates, mapas). B178–B182 E5 (**B180: só dar
gold/dinheiro, nunca tirar**). B183 mapas em execução (portão §6.2). B184 atributos. B185 posição
(exceção ao portão em 2026-10-06). B186–B187 dar (lotes de uma pilha, tudo ou nada) e remover.
B188–B189 ícones e dica parte 1.

**Inventário como o do jogo (2026-10-06):** ícones → dica → arrastar → edição **livre**; atlas e
textos do cliente 1.5.5 servem as duas versões (`data/icones/`, `data/textos/`).

### 6.14 B190 e a fila aprovada (análise de 2026-10-07)

**Octets × colunas (ideia do Murillo) — decidido: manter os octets.** `character_items` já é
híbrida: colunas espelho (durabilidade, refino, furos/pedras, vínculo, fabricante) + `extra_data`;
a verdade são os octets (o refino da coluna é recalculado deles, `pedras_e_refino.rs:354`) e o
cliente recebe os bytes gravados. Colunas não cobrem o item (efeitos em lista variável, essência
por família, decodificação só com o molde do `elements`; ovo, voador, gênio fora do equipamento)
e custariam montar os octets a cada leitura/envio. Edição: o painel manda edição estruturada, o
GS decodifica com `ConteudoDeEquipamento`, regrava os octets e as colunas espelho no mesmo upsert.

**Feitos (todos `testado`, detalhes no histórico e nas specs 05 §7.11 e 06):** B190 cartões e tela
com janelas · B191 arrastar + `CheckEquipPostion` portado · B192–B193 dica: efeitos gerados do fonte e
frases além do 112 conferidas no binário BR (`scripts/gerar_efeitos_de_itens.py`,
`scripts/conferir_efeitos_no_binario.py`) · B194 editar item (octetos no GS; online `OWN_ITEM_INFO`;
fora: modelo, vínculo; falta o brilho do refino para quem vê) · B195–B196 habilidades ver/editar
(online `LEARN_SKILL`; 1.2.6 descer/remover offline) · durabilidade: a coluna é a verdade
(`octetos_atuais`) · B197 mascotes (ver, editar o `pet_data`, libertar; online `PET_ROOM`/`FREE_PET`;
invocado não se edita; dar = dar o ovo) · B198 missões (ver; dar livre `NEW`; concluir = forçar sucesso
`FINISHED`/`COMPLETE`; cancelar = apagar `GIVE_UP`, 1.2.6 offline; esquecer só offline; offline recusa prêmio
que exige a entidade; resposta do canal até 64 KiB).

**Fila:** B199 aparência (rosto: `PLAYER_CHG_FACE`, `EC_GPDataType.h:1084`).

**Próxima ação:** B199 (aparência). Murillo confere em jogo B179–B198 (B183 = portão §6.2).
Não publicar/commitar sem pedido.
