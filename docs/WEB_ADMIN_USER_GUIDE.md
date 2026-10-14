# Manual do painel de administração

Para quem administra os servidores (GM). Diz o que cada página faz, quando a mudança vale e o que
o painel nunca edita. As regras completas e os códigos de recusa ficam na spec 06
(`specs/06_ADMIN_PANEL_AND_CPW_SPEC.md`) e na spec 05 §7.9–7.11; os contratos, em
`docs/admin/ARQUITETURA_E_CONTRATOS.md`; o que vem depois da reforma, em
`docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md`. Atualizado no B218 (E9).

## 1. Entrar

- Endereço: `http://<host>:8000`. Entre com uma **conta do jogo que tenha GM** e não esteja banida.
- A sessão dura 8 h. Banir a conta, tirar o GM dela ou trocar a senha encerra o acesso na
  requisição seguinte; entrar de novo em outro navegador derruba a sessão anterior.
- O botão de lua no topo alterna tema claro/escuro (sem escolha, segue o sistema); a escolha fica
  só neste navegador.
- O seletor de realm no topo vale para as páginas do grupo **Realm**. Conferir o realm antes de
  qualquer edição: o painel sempre manda o realm junto, mas é você quem escolhe.

## 2. Quando uma mudança vale

Todo resultado aparece num aviso no canto (some sozinho). Ele diz um destes:

| resultado | significa |
| :--- | :--- |
| **Aplicado em jogo** | o personagem estava em jogo; o servidor mudou a memória, avisou o cliente e grava no salvamento normal |
| **Salvo** | o personagem estava fora do jogo; o servidor gravou no banco com uma trava que recusa o login enquanto grava |
| **Precisa estar fora do jogo** | aquela mudança não tem como aparecer no cliente aberto; o painel oferece **Desconectar e aplicar** (desconecta, espera o servidor confirmar a saída e grava) |
| **Precisa estar em jogo** | só vale com o personagem em jogo (EXP/SP) |
| **Aguardando** | o servidor não respondeu a tempo; o painel consulta o resultado pelo mesmo pedido, sem repetir a mudança |

| o quê | em jogo | fora do jogo |
| :--- | :--- | :--- |
| Dinheiro, pontos, nível, cultivo, atributos, posição | na hora | salvo |
| EXP e SP | na hora | — (precisa estar em jogo) |
| Dar/criar item na bolsa, bolsa de missão, armazém | na hora | salvo |
| Remover/arrastar no corpo ou armazém fora dos pares do jogo | precisa estar fora | salvo |
| Editar item (quantidade, durabilidade, refino, requisitos, pedras, efeitos) | na hora | salvo |
| Habilidades (ensinar, subir, descer, remover) | na hora | salvo |
| Mascotes (dar, editar, libertar), Daimon (dar EXP), missões | na hora | salvo |
| Gold da conta | na hora (cliente recebe o saldo) | salvo |
| Rates do realm | na hora, em todos os servidores do realm | gravadas; valem também após reinício |
| Ligar/desligar mapa | na hora (desligar salva e desconecta quem está nele) | — |
| Templates de criação | para o **próximo** personagem criado | — |
| GM da conta | gravado na hora; o jogador precisa **entrar de novo** | — |

## 3. As páginas

### Geral

- **Visão geral:** um cartão por realm do ambiente — no ar ou não, jogadores online, personagens e
  rates. Clicar seleciona o realm.
- **Contas globais:** contas de todos os realms em cartões (12 por página) com busca por usuário.
  "Nova conta" cria (usuário de letras, números e `_`, gravado em minúsculas). Clicar numa conta
  abre: **GM** (dar/tirar), **Senha**, **Gold** (só dar; 100 = 1 gold), **Banir/Desbanir** (com
  motivo; banir desconecta em todos os realms) e **Desconectar**. Gold e banimento são da conta,
  valem em todos os realms.

### Realm

- **Painel do realm:** jogadores online, personagens, mapas no ar, rates gravadas × em vigor e a
  lista de recursos do painel neste realm (o que está disponível, parcial ou indisponível, com a
  explicação ao passar o mouse).
- **Personagens:** todos os personagens do realm (de todas as contas), busca por personagem ou
  conta. Clicar abre a **tela do personagem**, com janelas que se recolhem para a bandeja no canto:
  - **Estatísticas** → "Editar": dinheiro, EXP/SP, pontos, nível, cultivo, atributos (ou
    redistribuir) e posição.
  - **Equipamento**, **Roupas**, **Inventário**, **Armazém**, **Bolsa de missão:** passar o mouse
    mostra a dica como no jogo; clicar num item abre **Editar item**; slot vazio → **Criar item**;
    arrastar entre janelas move como no jogo. "Dar item" (Inventário) entrega como prêmio.
  - **Habilidades:** "Dar habilidade" (busca por nome/ID e classe); clicar numa aprendida muda o
    nível ou remove.
  - **Mascotes:** "Dar mascote" (busca nos ovos por nome, tipo e ambiente); clicar edita (nível,
    EXP, lealdade, fome, pontos, nome, habilidades) ou liberta.
  - **Daimon** (só no 1.5.5 — o 1.2.6 não tem): ficha e "Dar EXP".
  - **Missões ativas** → "Editar": concluir, cancelar, esquecer uma concluída e dar missão.
- **Mapas:** todos os mapas da versão, com filtro por número, nome ou status. Selecionar e
  **Ligar/Desligar selecionados**. Desligar pede confirmação, salva e desconecta quem está no mapa e
  recusa novas entradas.
- **Rates:** EXP, SP, drop e moedas (0,1 a 99,9) do realm. "Aplicar" vale na hora.
- **Templates:** com o que cada classe nasce neste realm — estatísticas iniciais, bolsa, corpo e
  habilidades. A edição vale para os personagens criados **depois**; os existentes não mudam.

## 4. O que o painel nunca faz (decisões do Murillo, memória da reforma §1)

- **Tirar** dinheiro, gold ou EXP: só dar.
- Saldo de gold diferente por realm: o gold é da conta.
- Mudar o **formato** de um item: a edição muda valores dentro do formato que o cliente lê.
- Editar a **aparência** (rosto, cabelo, corpo): fica com a troca de rosto do próprio jogo.
- Editar os arquivos de dados do jogo (`elements`, `tasks`, `npcgen`) ou do cliente (modelos,
  texturas): só leitura nesta reforma.
- "Aplicar aos existentes" nos Templates.
- Histórico navegável de operações ou níveis de permissão entre GMs: todo GM é administrador.

## 5. Limitações conhecidas

- Textos e ícones do painel vêm do cliente **1.5.5 BR**, também no 1.2.6 (nomes de mapa e
  descrições do 1.2.6 não conferidos no cliente dessa versão).
- Conta parada na seleção de personagem não está em nenhum servidor de mundo: "Desconectar" não a
  alcança (o login seguinte de conta banida é recusado).
- Uma versão de realm fora de 1.2.6/1.5.5 aparece com os recursos "não validado".

## 6. Instalação e configuração (operador)

- Serviço `pw-admin-api` no `docker/docker-compose.yml`, porta 8000; a mesma origem serve API e
  páginas, sem CDN.
- Cada realm com canal administrativo precisa da mesma chave no GS e no painel: `ADMIN_SECRET_126`
  e `ADMIN_SECRET_155` em `docker/.env` (fora do git; trocar = recriar `pw-world-*` e
  `pw-admin-api`). `ADMIN_DAEMONS` lista os processos de cada realm; a porta interna 29110 nunca é
  publicada. Não registrar chaves.
- HTTP na LAN usa `ADMIN_COOKIE_SECURE=false`; com HTTPS, `true`.
- Publicar só o painel (com autorização do Murillo):

```powershell
docker compose -f docker/docker-compose.yml build pw-admin-api
docker compose -f docker/docker-compose.yml up -d --no-deps pw-admin-api
```

- Antes de publicar algo que mexa em login, entrada, saída ou salvamento:
  `scripts/conferir_public_antes_de_publicar.sql` (memória §6.2).
- Testes do painel: skill `pw-testar-e-publicar` — Python com `TEST_DATABASE_URL`, `TEST_REDIS_URL`
  e `PW_VALIDADOR_CREDENCIAIS`; Node `node tests/teste_fluxo_contas.cjs` em `web-admin/backend`.
  Os roteiros técnicos antigos (B166–B170) ficam no histórico.
