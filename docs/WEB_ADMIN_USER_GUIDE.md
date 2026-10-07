# Painel administrativo — reforma parcial (B165–B174)

Atualizado em 2026-10-05 (B174): interface refeita e publicada; canal administrativo
ativo nos realms 126 e 155. Etapas e pendências: `docs/admin/MEMORIA_DA_REFORMA.md`.

## Acesso e recursos disponíveis

Abra `http://localhost:8000` na máquina do servidor ou `http://<IP-do-servidor>:8000`
na rede. Use sua conta PW e senha com privilégio GM. Conta banida ou sem GM não entra.
O serviço usa as credenciais existentes, sem trocar senha/hash nem criar sessão de jogo.

- **Visão geral:** um cartão por realm com estado (no ar/fora do ar), jogadores online ao
  vivo, personagens e rates (EXP, SP, DROP, MOEDAS). Rates vêm do banco e ainda não são
  aplicadas pelo mundo ("Não aplicadas"). Online "—" = realm sem canal administrativo.
- **Seletor de realm (topo):** escolhe o realm; o menu lateral libera o painel do realm,
  personagens, **Mapas** e **Rates** (moldes: "em breve").
- **Personagens:** cartões com nome, conta, classe e nível (12 por página); a busca acha pelo
  nome do personagem ou da conta. Clique no cartão para abrir a tela do personagem, com janelas
  (Estatísticas, Equipamento, Roupas, Inventário, Armazém, Bolsa de missão; o "–" recolhe) e
  **‹ Voltar** para a lista. **Arraste** itens entre bolsa, equipamento, roupas, armazém e bolsa de
  missão para trocar de lugar ou vestir/tirar (com o personagem em jogo, armazém e missão pedem
  "Desconectar e aplicar"). Na janela Estatísticas: **Dinheiro**, **EXP / SP** e **Pontos livres**
  (só dar), **Nível** (só sobe, até o teto do realm), **Atributos** (mudar força/agilidade/vitalidade/
  energia já distribuídas — o total com os livres não muda — ou **Redistribuir** tudo de volta aos
  pontos livres), **Posição** (escolha o mapa entre os ligados e as coordenadas; Y vazio = chão) e
  **Cultivo** (lista conforme a versão). Abaixo, **Itens**: bolsa, equipamento, armazém e bolsa de
  missão em grade, com o ícone de cada item; passe o mouse para ver a dica do jogo (nome na cor,
  valores, durabilidade, requisitos, pedras, efeitos, preço e descrição); em **Dar item**, digite parte do nome (ou o ID), clique no item da lista,
  escolha a quantidade e Dar — se não couber tudo, nada é dado. Para **remover**, clique no item
  num dos recipientes, ajuste a quantidade e confirme (equipamento e armazém só com o personagem
  fora do jogo). Em jogo,
  aplica na hora e o jogador vê; fora do jogo, grava no banco — só EXP/SP pedem o personagem em jogo.
- **Mapas:** todos os mapas do realm (ligados primeiro), com filtro por número ou nome e por
  status. Marque as linhas (ou "Selecionar todos", que pega só o que o filtro mostra) e use
  **Ligar selecionados** / **Desligar selecionados**. Ligar carrega o mapa na hora ("Carregando…"
  até subir); desligar pede confirmação, salva e devolve ao login quem está nele e descarrega.
  "Sem dados": o servidor não tem os arquivos desse mapa. O estado sobrevive a reinício.
- **Rates:** EXP, SP, DROP e MOEDAS de 0,1× a 99,9×. "Aplicar no realm" grava e passa a valer
  no mundo na hora, sem reiniciar. Drop/moedas com fração: 1,5× = um sorteio garantido e 50%
  de chance de outro. EXP/SP valem só para monstro abatido (missões não mudam).
- **Contas globais (menu Geral):** cartões paginados com busca por usuário. Clicar numa conta
  abre um popup junto do cartão com o resumo e as ações: **GM**, **Senha**, **Gold** (só dar, em gold), **Banir/Desbanir** e **Desconectar**. Clicar fora fecha; concluir também.
  "+ Nova conta" abre a criação. Todo resultado aparece num alerta no centro da tela; se o
  servidor ainda não confirmou, o alerta fica "Aplicando…" e o painel consulta sozinho.
Não há histórico navegável de operações neste escopo.

## Roteiro para o Murillo

1. Abrir a página: deve aparecer a tela de login, com paisagem, tons jade/dourado e campos
   Conta PW/Senha. Entrar com conta sem GM ou senha errada deve mostrar recusa.
2. Entrar com a conta GM: deve aparecer Visão dos realms, com cartões das versões
   configuradas no banco. O 1.2.6 é selecionado inicialmente quando estiver presente.
3. Selecionar 1.2.6 e depois 1.5.5, pelo cartão e pelo seletor superior. Nome, porta,
   contagem persistida e recursos devem sempre corresponder ao realm selecionado.
4. Abrir Recursos disponíveis: integração viva, edição, mapas/rates e catálogo devem
   aparecer indisponíveis com motivo. Outras versões devem mostrar protocolo não validado.
5. Clicar Atualizar: horário de consulta deve mudar; em falha, mostrar aviso de que os
   dados anteriores podem estar desatualizados. Conferir também teclado e janela estreita.
6. Clicar Sair: deve voltar ao login; atualizar a página não deve restaurar a sessão.

No log de `pw-admin-api`, esperar startup concluído e respostas HTTP 200 para a página,
login GM e consultas; acesso anônimo à API retorna 401. Em falha de dependência, a API
retorna 503 e registra classe de erro sem credenciais. Não há alteração de pacote do
cliente nesta base; o overlay `d_rtdebug` não recebe mensagem nova por usar o painel.

## Operação e verificação

### Criação global E4 — roteiro após publicação autorizada (B168)

1. Na futura implantação autorizada, aplicar as migrações de comandos e
   `scripts/2026_10_05_criacao_contas.sql`, nessa ordem, no schema correto. Hoje ambas
   foram aplicadas somente em `test`. A segunda aborta se houver nomes duplicados sem
   distinguir caixa; resolver essas contas explicitamente antes de tentar novamente.
2. Entrar como GM → **Contas globais** → escolher realm com canal ativo → **Criar conta
   global**. Usar nome novo com letras ASCII/números/_ (1–64 caracteres) e senha ASCII
   imprimível (1–64). Esperar `conta global <nome> criada · ID <n>` e o ID da operação.
   Nome aparece em minúsculas; buscar a conta confirma GM 0, sem ban e saldo zero.
3. Tentar o mesmo nome com outra caixa e nova operação: esperar `usuario_existente`,
   sem segunda conta. A conta comum recém-criada deve ser recusada no login do painel.
4. Em resultado desconhecido, recarregar/relogin com o mesmo GM → **Contas globais** →
   **Consultar resultado**; pode trocar para outro realm com canal ativo. A criação pendente
   deve manter nome/ID. Repetir exige a mesma senha e usa o mesmo ID; senha diferente gera
   conflito e conserva o resultado original. A senha nunca fica no localStorage.
5. No cliente 1.2.6, usar nome em minúsculas e senha criada: abrir seleção de personagens
   inicialmente vazia. Repetir no 1.5.5; senha errada deve ser recusada nos dois.

Log GS: `admin: criar_conta global`, operação/administrador/conta/estado, sem senha.
Log link: autenticação da nova conta; prova errada registra resposta inválida. API: 200/salvo,
202/desconhecido, 409/nome existente ou conflito de parâmetros. Não há pacote gamedata novo;
`d_rtdebug` 1.5.5 sem mensagem nova. Renderização/cliente original/Linux ainda pendentes;
47 Python, 117 Rust focados e 4 Node passaram automaticamente, sem falhas.

### Senha global E4 — roteiro após publicação autorizada

1. Antes de publicar, aplicar a migração `scripts/2026_10_05_comandos_administrativos.sql`
   no `public` e no `test` (nesta entrega só `test` foi aplicado). Conferir hashes das contas
   usadas no jogo; redefinir explicitamente pelo painel quando incompatíveis com algo=0.
   Não basta atualizar só o painel: GS/link novos e canal configurado são necessários.
2. Entrar como GM, abrir **Contas globais**, buscar a conta e selecioná-la. Conferir nome/ID,
   GM, ban e saldo da conta. Escolher um realm com canal ativo, que serve apenas de rota.
3. Informar nova senha ASCII imprimível, de 1 a 64 caracteres, e clicar **Trocar senha global**.
   Esperar `senha global salva` com ID da operação. Sessões de jogo abertas continuam;
   se for a própria conta GM, a próxima requisição do painel deve pedir novo login.
4. Fechar a conexão do cliente e entrar novamente no 1.2.6: senha antiga deve ser recusada,
   nova deve abrir seleção de personagens. Repetir no 1.5.5 para a mesma conta. Usar o nome
   de usuário em minúsculas, como o hash legado gerado. Verificar persistência após relogar.
5. Em timeout, esperar **resultado desconhecido**. Recarregar/relogin restaura o ID pendente;
   clicar **Consultar resultado**. Pode selecionar outro realm com canal ativo para consulta.
   Se repetir, usar o mesmo ID/alvo e senha; senha diferente dá conflito e mantém a consulta
   da operação original. Nenhuma senha fica salva no armazenamento do navegador.

Log GS: `admin: trocar_senha global`, com operação, conta, administrador e estado, sem senha.
Log link: `Login rejeitado: resposta de senha inválida` para prova errada e autenticação para
a nova senha. API: 200/salvo, 202/desconhecido, 409/conflito; sessão revogada 401. Não há novo
pacote de gamedata; overlay `d_rtdebug` sem mensagem nova. Renderização e teste nos clientes
originais aguardam Murillo; testes automatizados de dispatcher não substituem essa confirmação.

### Consultas E3 — roteiro após publicação futura

A página **Personagens** busca nomes no realm selecionado e identifica a ficha como
viva ou persistida. Edição permanece indisponível. Não confundir busca persistida com
nível/estado atual de um jogador online: a ficha consultada ao GS é a leitura viva.

1. Selecionar 1.2.6, abrir Personagens e buscar o nome. Abrir a ficha: antes de entrar
   no jogo, esperar origem persistida e ausência observada nos daemons configurados.
2. Entrar com esse personagem, clicar Atualizar: esperar origem viva e presença online,
   mapa, vida/mana, atributos e progressão vindos do GS. Mover no cliente e atualizar:
   posição deve acompanhar. Repetir no 1.5.5 com seu próprio personagem.
3. Alternar realms enquanto a consulta carrega: nenhum resultado atrasado deve aparecer
   como pertencente ao outro realm. Personagem do outro realm não pode ser consultado ali.
4. Sair normalmente do jogo e atualizar: a ficha passa à origem persistida. Durante
   entrada/saída/troca, transição é explícita. Falha do daemon é presença desconhecida;
   queda do link com entidade residual também mantém edição indisponível.
5. Na visão do realm, conferir mapas efetivamente servidos e contagem viva. Sem canal
   configurado ou com falha de qualquer processo, esperar desconhecido, sem falso zero.

Log GS: `canal administrativo em` quando ativado, ou `canal administrativo desabilitado`
sem chave. API: HTTP 200 nas consultas autorizadas (mesmo quando presença desconhecida),
401 anônimo e 404 para personagem de outro realm. Não há pacote cliente novo nem mensagem
nova no overlay `d_rtdebug`: apenas observar os valores normais do personagem no jogo.

### Configuração

O GS precisa de `ADMIN_SECRET` válido (32 bytes aleatórios em hexadecimal) e o painel da
mesma chave por realm. Compose recebe `ADMIN_SECRET_126`/`ADMIN_SECRET_155` de `docker/.env`
(ignorado pelo git; trocar a chave = recriar `pw-world-*` e `pw-admin-api`), sem padrão;
`ADMIN_DAEMONS` lista os processos do realm, sem publicar a porta interna 29110.
Ativar E3 exige publicar também os mundos com o código novo, quando solicitado; atualizar
somente o painel mantém consulta viva indisponível contra mundos antigos. Não registrar chaves.

Serviço: `docker/docker-compose.yml`, `pw-admin-api`, porta 8000. FastAPI serve API e
assets na mesma origem; não depende de CDN nem de uma segunda aplicação frontend.
Cookie HttpOnly/SameSite Strict, sessão Redis de 8 h e CSRF nas escritas. Banimento,
revogação de GM ou troca de senha encerram o acesso na próxima requisição.

O Compose está configurado para HTTP na LAN com `ADMIN_COOKIE_SECURE=false`.
Ao oferecer HTTPS, definir `ADMIN_COOKIE_SECURE=true`. Isso é configuração do transporte,
não um segundo tipo de credencial.

Com autorização do Murillo, publicar somente o painel:

```powershell
docker compose -f docker/docker-compose.yml build pw-admin-api
docker compose -f docker/docker-compose.yml up -d --no-deps pw-admin-api
```

Testes locais precisam das dependências de `web-admin/backend/requirements-teste.txt`,
`TEST_DATABASE_URL` e do binário `pw-validar-credenciais` compilado. Definir
`PW_VALIDADOR_CREDENCIAIS` com seu caminho. Os testes usam exclusivamente schema `test`
e namespace Redis aleatório, limpando seus próprios dados. Ausência de banco é falha.

```powershell
cargo build --locked -p pw-crypto --bin pw-validar-credenciais
python -m unittest discover -s web-admin/backend/tests -p 'teste_*.py'
```

Evidência E3 (B166): 37 testes Python passaram/0 falharam; 6 testes do canal + 3 de mapas
passaram; regressão de subcomandos: 152 passaram, 2 ignorados; topologia: 5 passaram;
0 falhas. Build nativo, JS
e Compose verificados. O teste Python com GS real exige `pw-gs` compilado em `target/debug`
ou `PW_GS_TESTE` com o caminho. Imagem Linux com E3 ainda não verificada. E2 (B165) teve
24 testes crypto/bus e build Linux com três verificações de runtime. Renderização visual
e confirmação em uso aguardam Murillo; não declarar a reforma inteira concluída.

Evidência B167: suíte completa com banco, **928 passaram/0 falharam/2 ignorados**;
rodada focada atualizada, 110 Rust/0 falhas; 41 Python/0 falhas; 2 Node/0 falhas. Falhas
anteriores registradas e corrigidas: configuração do verificador e eco da senha na validação.
Antes de testar em banco existente, aplicar as duas migrações exclusivamente no schema `test`
(mesma sessão SQL: `SET search_path TO test`, conferir `current_schema()`, executar script).
Compilar também `pw-gs`/`pw-link` para testes de canal/login; não alterar nem publicar serviços
reais para executar a suíte. Testes Node: `node --test backend/tests/teste_fluxo_contas.cjs`.

Evidência B168: workspace com banco **935 passaram/0 falharam/2 ignorados**;
47 Python, 117 Rust focados e 4 Node passaram sem falhas. Redis dedicado descartado
após os testes. Public permaneceu sem tabela de comandos e índice novos.

## GM global — B169, local, falta confirmação visual/em jogo

1. Entre como GM no painel → **Contas globais** → busque/selecione a conta de teste.
2. Em **Privilégio GM global**, escolha **Administrador / GM** ou **Sem GM** e aplique.
   Vale em todos os realms; não há hierarquia adicional. Remover o próprio GM revoga
   o painel na próxima requisição. Para testar recuperação, use um administrador separado.
3. Primeiro resultado: **salvo; sessões pendentes**, com ID. Use **Consultar resultado**
   até **salvo e sessões reconciliadas**. Processo ausente mantém pendência, nunca zero
   sessões inferidas. Timeout significa desconhecido; conserve o ID. Reload/relogin e
   outro realm recuperam o mesmo comando. Parâmetros diferentes dão conflito.
4. Mantenha uma sessão de jogo da conta em cada realm 126/155. Após conceder GM, saia e
   entre novamente para o cliente recarregar os bits de **Ctrl+G** e a aparência/coroa.
   O servidor já recebeu GM; cliente durante o jogo não recebe recarga de auth comprovada.
5. Em uma sessão GM, ative **Invencível** e **Invisível**; remova GM pelo outro administrador.
   Após a confirmação, dano volta a afetar o jogador, invisibilidade termina e os comandos
   GM são negados em ambos os realms, mesmo se o cliente ainda mostrar botões. Reentre:
   Ctrl+G deve refletir ausência de GM. No 126, as mensagens de confirmação 175/176 ainda
   não têm evidência de layout; conferir efeitos, não exigir essas mensagens.
6. Para verificar recuperação em ambiente descartável: interromper um GS, alterar GM
   via o outro, observar pendência; reiniciar o mesmo processo com ID/chave/configuração
   corretos → consultar o mesmo ID até aplicação. Não repetir com ID novo.

Log GS: `admin: definir_gm global`, ID/conta/estado (primeiro `pendente`); consumo:
`mundo: GM <id> ...` ou `sem ser GM`. Falha de fotografia: `coordenação GM: banco
indisponível`; perda do fencing: erro e término do daemon. Não há S2C novo nesta fatia;
no `d_rtdebug 1` do 155 não deve aparecer recusa de tamanho/ID relacionada à reforma.
Teste automático usou sockets dos links/GS e SelectRole_Re.auth; não equivale a clientes
originais nem inspeção visual do painel. Nenhum navegador disponível na ferramenta.
Não houve publicação, migração em public ou alteração de contas reais.

Conexões de jogo são mantidas pela alteração GM. B170 salva saída/queda/troca com fotografia e
ordenação locais; desconexão forçada/banimento aguardam fencing global, recuperação
durável e todos os produtores. A ausência de jogador não autoriza escrita offline. Gold continua aguardando consumo da loja/concorrência.


## Saída e queda do link — B170, local, falta ver em jogo

1. Em contas descartáveis 126/155, altere posição/dinheiro/chi/modo roupa e faça uma
   missão. Saia para seleção antes de completar 60 s; reentre e confira o estado salvo.
2. Com outro personagem observando, feche o cliente/derrube apenas o link em ambiente
   de teste. O avatar deve desaparecer; o GS registra `saída: personagem <id> salvo e
   retirado`. Reentre e confira o estado final; não deve haver entidade residual.
3. Em teste descartável de falha de banco, saída fica `em_transicao`, bloqueia reentrada
   e registra `saída: fotografia ... não confirmada`. Ao recuperar banco, saída conclui.
   Uma confirmação de transferência desconhecida conserva a reserva até confirmar.
4. Troque de mapa, saia e reentre: mapa/posição de destino devem persistir. Desconexão
   durante recuperação de transferência deve terminar a saída, sem sessão fantasma.

Não há novo layout/pacote; no overlay `d_rtdebug 1` do 155 não deve aparecer descarte
por tamanho/ID. Confirmação visual/em jogo e imagem Linux pendentes; não publicado.
Esta base é local: queda do GS pode perder fotografia não confirmada; fencing/diário
persistidos e integração de todos os produtores ainda faltam. Não habilita edição offline.
