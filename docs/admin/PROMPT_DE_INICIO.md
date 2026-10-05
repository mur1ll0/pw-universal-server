# Prompt para iniciar ou retomar a reforma do pw-admin

Copie o texto abaixo em outra sessão aberta neste repositório.

```text
Trabalhe na reforma do pw-admin/web-admin no repositório
F:\Python_C_Projects\PWSource1.5.3\pw-universal-server.

Execute o desenvolvimento, com entregas funcionais por etapa. O escopo e as decisões
de produto já foram aprovados pelo Murillo e estão em:
docs/admin/MEMORIA_DA_REFORMA.md

Esse arquivo é a memória central e o ponto de retomada. Leia-o no começo, ao retomar
ou após compactação. Não reinicie o planejamento já resolvido e não refaça etapas
concluídas sem motivo. Confira suas afirmações contra o código e o ambiente atuais.

Siga AGENTS.md e o guia pw-retomar-sessao. Leia apenas as seções obrigatórias do
estado geral e a spec da etapa atual. Para começar, use a spec 06 do painel;
outras specs e skills entram conforme a integração exigir. Não leia o histórico
inteiro nem todos os módulos. Preserve alterações de outros agentes/usuários.

OBJETIVO
Entregar um painel visual, interativo e em português, inspirado no jogo, no mesmo
Docker Compose dos servidores, acessível pela rede. Deve gerenciar os realms e
mundos reais, começar com 1.2.6 e 1.5.5 e permitir expansão a outras versões.
Detectar um realm não significa presumir que formatos desconhecidos são suportados:
mostre capacidades e limitações reais. Use dados e ícones do elements do realm certo.

ESCOPO APROVADO
- Login pela conta PW com privilégio GM, validado no backend. Todos os usuários
  autorizados são administradores/GM. Reusar a validação de credenciais existente.
- Contas: criar, listar/buscar, mudar senha, gerir GM, conceder/remover gold,
  banir/desbanir e desconectar. Gold e ban são globais entre realms.
- Sem funcionalidade de histórico navegável de operações; manter logs técnicos.
- Personagens: consulta e edição de atributos/pontos, nível/EXP/alma/cultivo,
  dinheiro, inventário/equipamentos, habilidades, missões, posição, aparência
  e mascotes, inclusive online.
- Mapas por realm: ligar/desligar efetivamente; desligar desconecta jogadores,
  coordena salvamento e bloqueia entradas, sem aguardar o mapa ficar vazio.
- Rates por realm: EXP, alma, drop e dinheiro, aplicados no mundo e persistidos.
- Moldes de classe por versão/realm: editar fontes realmente usadas; mudanças
  valem por padrão para personagens novos. Recálculo dos existentes é ação
  separada e explícita, preservando progressão conforme contrato validado.
- Preparar arquitetura para futuros editores de elements, npcgen, tasks,
  modelos, animações e texturas. Os editores completos ficam fora desta reforma.

DIREÇÃO DE ARQUITETURA
Audite a implementação atual antes de escolher o que reaproveitar. Pode substituir
o frontend e reorganizar o backend; escolha tecnologias pelo custo de manutenção,
interface visual e integração com os leitores e serviços existentes. Registre a
decisão e o plano de migração na memória; não imponha um framework por preferência.

Para personagem online, a autoridade é o pw-gs: painel autenticado -> canal interno
administrativo protegido -> daemon responsável -> alteração validada em memória ->
recálculo e pacotes corretos ao cliente -> persistência coordenada. Não editar só
o banco e informar sucesso. Distinguir aplicado, salvo, falha e resultado desconhecido.
Oferecer leitura viva para online e persistida para offline, identificando a origem.

Proteja edição offline contra login simultâneo. Coordene comandos com autosave,
logout e transferência de mapa. Evite operações duplicadas em novas tentativas e
gravações antigas sobrepondo estados recentes. Não espere banco no world.tick nem
segure o lock do mundo durante I/O. Não exponha o barramento sem autenticação.

Todo layout e regra precisam de evidência do original, IR, captura ou arquivo de
dados. Mudanças de protocolo do cliente ficam no WorldProtocol da versão. Siga as
skills de protocolo, regras de jogo e dados quando aplicáveis. Para operações sem
atualização online comprovada, explicite reconexão/indisponibilidade; não finja
suporte. Nenhuma tela pode indicar que uma operação funcionou apenas por gravar
uma flag que nenhum daemon consome.

EXECUÇÃO E MEMÓRIA
Retome a etapa e a próxima ação registradas na memória. Na primeira execução,
audite painel e integrações, registre fontes de verdade, contratos, escolhas,
inventário de funções reais/quebradas e divisão em tarefas, e comece a implementar.
Complete fatias funcionais, incluindo interface, API, daemon e persistência quando
necessários. Avance pelas etapas E1–E9 sem pedir revisão rotineira. Se a sessão
terminar antes do escopo completo, deixe o estado exato para continuação; não declare
a reforma concluída enquanto houver recursos obrigatórios faltando.

Atualize a memória ao fechar cada bloco e antes de compactação: estado da etapa,
decisões, arquivos relevantes, evidência curta, pendências e próximo passo executável.
Mantenha-a curta, sem transcrições de logs; detalhes ficam nas specs e no histórico.
Ela não substitui specs/ESTADO_E_RETOMADA/HISTORICO_DE_SESSOES: siga pw-atualizar-specs
para alterações de comportamento e use o próximo número B sem colisão.

VERIFICAÇÃO
Teste riscos reais, incluindo autorização, alvo/realm correto, operação repetida,
concorrência com entrada/saída/autosave, persistência após relogar, falha de daemon,
mapas e consumo efetivo dos rates. Integração exige TEST_DATABASE_URL e schema test;
informe números de passou/falhou. Rode build/checks pertinentes e verifique a
interface quando houver ferramenta disponível. Nunca declare teste que não ocorreu.
Para entrega visível, deixe roteiro exato da tela, log e overlay do cliente.
Diferencie testado automaticamente, falta ver em jogo e confirmado pelo Murillo.
Filtre saídas extensas e apresente consumo por etapa conforme AGENTS.md.

AUTONOMIA
Está autorizado alterar painel, serviços Rust, banco e Docker nas integrações
necessárias. Resolva escolhas técnicas rotineiras com evidência e registre-as.
Pergunte somente por decisão de produto relevante ainda indefinida.
Não faça commit, push ou publicação nos contêineres sem pedido explícito nesta
sessão. Se julgar necessário um commit prévio, pergunte ao Murillo.

LIMITES (revisão de rumo de 2026-10-05, MEMORIA_DA_REFORMA.md §6.2)
- Login, seleção, entrada, saída, salvamento e troca de mapa (link/GS) são caminho crítico
  do jogo. Mudou um deles: confira compatibilidade com os dados do schema public (leitura)
  e entregue o roteiro em jogo; não abra o próximo B antes do teste do Murillo.
- Não adicione camadas de coordenação global (lease, época, fencing, process::exit).
  Siga o modelo de consistência da §6.2 item 2; corrida restante vira limitação escrita.
- Duas rodadas corrigindo a mesma corrida: pare e pergunte.
- Suíte inteira só no fim do bloco, em segundo plano, trazendo só passou/falhou.

Comece agora pela próxima ação indicada na memória central.
```
