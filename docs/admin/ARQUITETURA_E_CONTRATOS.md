# Arquitetura e contratos da reforma

E1, auditada em 2026-10-03 sobre `a010ff7`. Base E2: B165, publicada em 2026-10-04.
Memória e próxima ação: [MEMORIA_DA_REFORMA.md](MEMORIA_DA_REFORMA.md).

## Auditoria do painel anterior

As linhas de `main.py` abaixo referem-se à base `a010ff7`, anterior à substituição.

| área | evidência | diagnóstico e migração |
| :--- | :--- | :--- |
| Autorização | `web-admin/backend/main.py:34-41`, rotas a partir de 352 | Sem login; CORS irrestrito. Substituído por sessão administrativa, origem e CSRF. |
| Credenciais | `main.py:354,375`; `crates/pw-auth/src/service.rs:98`; `crates/pw-crypto/src/password.rs:57-112` | Painel antigo escrevia MD5; verificador existente aceita Argon2 e formatos legados. Novo binário reutiliza a função Rust, sem rehash ou ticket de jogo. |
| Contas | `main.py:352-454` | SQL efetivo para criação/senha/GM/gold/ban, sem validação completa, confirmação de aplicação online ou desconexão. Reimplementar em E4; rotas antigas indisponíveis. |
| Personagem, itens, habilidades | `main.py:479-888` | Leituras persistidas e escritas diretas, incompatíveis com edição online e sem proteção de entrada concorrente. Reimplementar em E3/E5/E6. |
| Autosave | `crates/pw-gs/src/world.rs:2640,2710` | B170: fotografia atômica, revisão e exclusão locais entre comandos/autosave/saída/troca. Faltam fencing global e todos os produtores assíncronos antes de edição. |
| Presença e métricas | `main.py:1035-1040,1079-1114` | TCP do link usado como estado do realm; `SCARD online:<realm>` sem contrato com GS. Nova consulta distingue gateway, mundo desconhecido e contagem persistida. |
| Mapas | `main.py:61-86,890-913` | Lista fixa e flags em memória do painel, sem comando para daemon nem persistência. E7 terá ciclo de vida real. |
| Rates e anúncio | `main.py:1048-1073` | Rates gravados em `realms`, sem consumo no GS; anúncio publicado em canal do delivery que não roda. E7; sem sucesso simulado. |
| Moldes | `main.py:94-102,1224`; `specs/06` anterior | Startup inseria moldes genéricos para três realms; edição não corresponde às fontes do GS. Startup não escreve mais moldes. E8 rastreará cada campo consumido. |
| Frontend | `main.py:1405-1419`; Dockerfile anterior | `backend/static/index.html` era servido e empacotado; cópia em `frontend/` tinha SHA-256 idêntico. Fonte única em `backend/static/`. |
| Dados e ícones | `elements_decoder.py:500-514,774-790` | Fallbacks podem buscar dados de outro realm. Preservado como referência, sem importação/rotas públicas até validação de origem e fechamento do arquivo. |
| Barramento | `crates/pw-bus/src/message.rs:49`; `specs/02` §2.3 | Transporte de jogo sem autenticação. Canal administrativo separado e protegido é necessário. |

## Escolhas implementadas na E2

- **FastAPI mantido:** integra asyncpg/Redis já presentes, não duplica leitores Rust e permite
  separar autorização, consulta e operações. Nenhum benefício local justifica trocar a API
  por outro framework nesta base.
- **HTML/CSS/JavaScript modular:** sem build de frontend, framework ou CDN. A primeira fatia
  tem dois fluxos pequenos; módulos por domínio podem crescer quando E4–E8 forem integradas.
  A interface usa jade, dourado, paisagem vetorial e tipografia local, sem prometer dados vivos.
- **Credenciais:** `pw-validar-credenciais` recebe JSON por stdin e responde `true/false`.
  Reutiliza `pw_crypto::verify_password`; senha/hash não aparecem em argumentos, respostas,
  logs ou tickets Redis de jogo. Não migra hash durante login administrativo.
- **Sessão:** token aleatório em cookie HttpOnly/SameSite Strict; Redis guarda somente a
  chave derivada do token, conta, impressão do hash da senha e CSRF. Prazo fixo de 8 h,
  política administrativa, sem renovação implícita. GM, ban e impressão são revalidados em
  cada requisição. Ban é recusado enquanto `is_banned` for verdadeiro, como o login existente.
- **Rede:** API e assets na mesma origem, sem CORS. CSRF e origem verificados nas escritas;
  limite de tentativas por conta/IP e no máximo quatro verificações locais simultâneas.
  Cookie Secure por padrão; Compose usa HTTP na LAN com `ADMIN_COOKIE_SECURE=false` configurável.
  Para HTTPS, definir `true`. Não há novo acesso ao socket Docker pelo painel.
- **Empacotamento:** Dockerfile com estágio Rust do verificador e runtime Python. Contexto
  raiz com ignore específico, sem enviar `data/`, `.git` ou `target/`. Fonte única de UI.
- **Migração:** substituição do backend anterior; rotas legadas exigem login e retornam
  501, sem mutação. API implementada: `/api/sessao/entrar`, `/api/sessao`, `/api/sessao/sair`
  e `/api/realms`. Não manter uma API insegura paralela. Nenhuma alteração de esquema nesta fatia.

## Fontes de verdade

| informação | autoridade |
| :--- | :--- |
| Credenciais, GM, ban e gold global | `accounts`; validação compartilhada em `pw-crypto` |
| Realms configurados | `realms`, com endereços internos explícitos por `ADMIN_GATEWAYS` |
| Conectividade do gateway | TCP exclusivamente no host/porta daquele realm, sem tentar localhost alternativo |
| Presença, mapa e personagem online | GS/roteador pelo canal de consulta E3; transições e inconsistências explícitas |
| Personagem persistido | PostgreSQL, origem explícita; ausência observada no GS não autoriza escrita offline |
| Dados e ícones | Arquivo e catálogo do próprio realm, validados pelo leitor existente |
| Moldes, rates e mapas | Fontes consumidas pelo daemon; confirmar os campos na etapa correspondente |

## Canal implementado na E3 (B166)

Consultas, senha e criação global recuperável (B167/B168). TCP interno 29110, opcional por chave de 32 bytes por realm, sem exposição
ao host. Framing u32 BE + JSON até 8192 bytes; desafio aleatório por conexão e HMAC-SHA256
de desafio/sentido/corpo. Pedido/resposta têm MAC de 32 bytes, correlação e realm verificados;
GS revalida GM/ban no banco. Timeout de 3 s e até 32 conexões no GS. Detalhes na spec 06.

API consulta todos os processos explicitamente listados em `ADMIN_DAEMONS` para o realm.
Falha de qualquer processo ou presença ambígua mantém desconhecido. Ficha básica copia
`PlayerEntity`; valores i64 são texto. Busca usa SQL limitado ao realm, com origem persistida.
Guarda do roteador cobre entrada/saída/troca e impede fotografia intermediária. Queda do
link com entidade residual é inconsistente, sem conceder edição de personagem.

## Primeira escrita implementada — senha global (B167)

Navegador gera ID estável e guarda só seus metadados antes do envio. API valida sessão,
origem, CSRF, realm, alvo, senha ASCII imprimível (1–64 bytes, política administrativa) e
corpo limitado. GS revalida autorização e usa `ComandoAdministrativoRepository`: reserva
ID global, trava administrador/alvo em ordem crescente, revalida GM/ban, altera senha e
grava resultado na mesma transação. Rollback não deixa efeito nem reserva confirmada.
Mesmo ID/alvo/parâmetros retorna resultado anterior; trocar realm não altera identidade.
Payload diferente retorna conflito e preserva a operação original. Não há expurgo de IDs.
Registro técnico tem IDs, origem, impressão de parâmetros e resultado, sem senha/hash
utilizável para login; não há histórico navegável. Falha definitiva de alvo é durável.

Recuperação autenticada por ID retorna `salvo`, `falha` ou `desconhecido`. Registro ausente
não prova falha: a transação pode estar em andamento. Perda da resposta após commit mantém
HTTP 202/desconhecido até consulta; repetir requer mesmo ID e senha. LocalStorage guarda
um pendente por administrador, sem segredo; relogin/reload restauram consulta. Outro realm
pode servir a recuperação. Outros administradores não recebem o resultado desse ID.

Hash legado vem de `pw_crypto::hash_legacy_pw_md5`; cliente algo=0 usa HMAC-MD5 com chave
MD5(nome+senha), `Network/gameclient.cpp:131-139` (1.5.5 e 1.5.3). Link anteriormente ignorava
prova de senha: agora valida desafio de sessão, rejeita token e replay de outra conexão,
e impede novo Response de trocar identidade. Não há layout novo. Hashes Argon2/texto e
sementes antigas incorretas exigem redefinição explícita para jogo; não resetar contas
existentes automaticamente. Senha vale nos novos logins; sessões de jogo abertas mantidas,
sessões do painel revogadas pela impressão de senha na requisição seguinte.

Gold é global, lido/debitado por `character.rs:780-809` e enviado pelo fluxo da loja
(`jogo.rs:2854-2926`); autosave de personagem não grava saldo da conta. GM do jogo é
cacheado na sessão/entidade; teleporte GM relê o banco. Ban é lido no login, sem expiração
ou desconexão ativa. Essas operações continuam pendentes: SQL isolado não atesta aplicação
online. Nenhuma escrita de personagem foi habilitada; residual/exclusão/revisões são E5.

Migração `scripts/2026_10_05_comandos_administrativos.sql` aplicada somente em `test`.
`public` aguarda implantação autorizada. Painel atual contém E3 e alvos 126/155, mas mundos
de 2026-10-03 sem ADMIN_SECRET recusam TCP 29110; consulta viva não está disponível.

## Criação global recuperável (B168)

`POST /api/realms/{realm}/contas` leva ID estável, usuário e senha pelo canal autenticado.
API/GS validam ASCII `[A-Za-z0-9_]`, 1–64 bytes, sem trim; nome canônico minúsculo conforme
cliente `ElementClient/EC_LoginSwitch.cpp:329`. Codificação do login: `:332-336`;
ASCII é política conservadora do painel, não limite alegado do original. Senha usa o
mesmo hash algo=0 da troca. Impressão inclui tipo/administrador/nome canônico/resumo da
senha; mudar caixa/rota não muda os parâmetros, senha/nome/tipo/dono diferentes conflitam.

Reserva tem conta_id NULL; autorização sob lock, INSERT com defaults (gold/silver/GM zero,
sem ban), ID retornado e resultado compartilham commit. Índice UNIQUE LOWER(username)
corresponde à busca/autenticação e resolve criações concorrentes em qualquer produtor.
Nome existente gera falha durável, sem senha sobrescrita. Rollback desfaz conta/reserva;
sequência SERIAL pode ter lacunas. Sem alteração de personagem ou lock do mundo.

UI mantém tipo/nome/ID/rota sem senha, restaura após reload/relogin, aceita recuperação
por outro realm e bloqueia outro tipo de escrita enquanto pendente. Timeout/ausência são
desconhecidos; consulta nunca reaplica. Conflito/recusa de autorização preservam ID.
Migração `scripts/2026_10_05_criacao_contas.sql` aplicada só em test; duplicatas antigas
abortam a migração inteira sem renomear/remover contas. Public e imagem Linux pendentes.

## Coordenação global de GM (B169)

| estado | autoridade e consumo |
| :--- | :--- |
| GM/ban globais persistidos | `accounts`; login do link lê GM/ban, GS carrega conta/revisão com personagem |
| sessão do link | `ClientSession.sec_level/revisao_gm`; determina `SelectRole_Re.auth` |
| sessão/efeitos no GS | `PlayerEntity.sec_level/revisao_gm/conta_id`; comandos GM, GOTO e contexto de missões consomem autorização revalidada |
| presença | roteador do GS; guarda de transições, entidade residual continua inconsistente |
| banimento ativo/gold/desconexão | pendentes; não há sincronização administrativa completa dessas operações |

GM da reforma é booleano: 0 sem GM, 1 todos os privilégios implementados, sem nova hierarquia.
Não é porte dos bits individuais ainda ausentes (B159). A fonte 155 recarrega os bits apenas
no `SelectRole_Re` (`Network/EC_GameSession.cpp:4581`); a 153 em `:4513`, referência do 126.
Não há evidência de recarga durante o jogo. Atualizamos autoridade e caches em todos os
processos; **cliente requer reentrada** para atualizar painel/coroa. Sem S2C novo.

`definir_gm` usa o comando recuperável: reserva, autorização GM/ban sob locks de contas
ordenados, alteração GM/revisão e resultado no mesmo commit. Um singleton transacional
ordena revisões por commit; sequência SERIAL não serve para cursor de aplicação.
Impressão inclui administrador, conta, booleano e conjunto de processos (ordenado), sem
realm de rota. Mesmo ID deduplica; parâmetros/topologia diferentes conflitam sem reaplicar.
A consulta não executa outra mutação de GM. Revisão posterior substitui operação ainda
pendente; a consulta distingue `substituido` de aplicação da revisão intermediária.

`ADMIN_COORDENACAO_ID` identifica cada processo; `ADMIN_COORDENACAO_ALVOS` lista **todos**
os links/GS coordenados, inclusive desligados. Compose inclui 126/155; 148/153 foram retirados
em 2026-10-05 por decisão do Murillo (fora da reforma; GM lá vale no próximo login). Registro
falho desliga a coordenação no processo (aviso) em vez de abortá-lo.
Sem lista, GM é recusado. Roteamento administrativo continua pelos GS, com HMAC/CSRF.
Cada encarnação tem conexão PostgreSQL dedicada e advisory lock exclusivo do ID; duplicata
não sobe. Reinício zera recibo antes de servir e reconcilia a partir do estado persistido.
Não confiar em recibo da encarnação anterior. Revisão e contas são lidas num snapshot
REPEATABLE READ; recebimentos só avançam depois da aplicação, nunca depois de enqueue.

Reconciliação a cada 1 s e recibo válido por 5 s (políticas internas). Link serializa
login/seleção com a fotografia e aguarda aplicação via watch em cada sessão. GS exclui
entrada/saída/troca/comandos durante reconciliação, copia/aplica em locks curtos do mundo;
I/O ocorre fora de `world.tick` e de locks do mundo. Remoção limpa invisibilidade e
invencibilidade, restabelecendo visibilidade aos outros. Comandos GM/GOTO e privilégio
nas missões usam `FOR SHARE` da conta até terminar o consumo: revogação não pode confirmar
SQL enquanto execução autorizada anterior está em andamento. Conta/revisão/ban são
revalidados; GOTO preserva consulta atual no banco sem depender do cache, incluindo
concessão na sessão já aberta. Erro de banco nega autorização. Leitura do privilégio não exige banco no tick.

| ocorrência | recuperação/resultados |
| :--- | :--- |
| transação concluída | `pendente`, `persistencia=salva`, sessões ainda não confirmadas |
| todos os processos reconheceram revisão | `aplicado`, sessões reconciliadas no servidor; cliente exige reentrada |
| processo desligado/recibo expirado | continua pendente e lista faltantes; nenhum offline presumido |
| timeout/perda/reserva não visível | `desconhecido`; conservar ID e consultar por qualquer realm |
| nova revisão antes de aplicar | `substituido`; preserva efeito persistido e não reexecuta revisão antiga |
| reinício de processo | fencing/encarnação novos, recibo zero, fotografia completa antes de reconhecer |
| banco indisponível na fotografia | GM negado/efeitos limpos, sem novo recibo; nova fotografia tenta recuperar |
| perda da conexão de fencing | coordenação desligada com log de erro, GM negado/efeitos removidos, jogo segue (B173); reinício reconcilia |
| próprio GM removido | painel revoga acesso na próxima requisição; metadados locais ficam, recuperar exige voltar a ser autorizado |

Consultas concorrentes devolvem o vencedor terminal durável, inclusive quando uma revisão
nova chega entre suas fotografias; a conclusão local que perdeu a atualização não é devolvida.
Confirmação terminal é durável e atesta o efeito verificado naquela revisão, sem alegar
saúde futura. Conta e recibos são consultados no mesmo snapshot SQL, evitando usar recibo
posterior para confirmar revisão pulada. UI conserva ID/tipo/conta/booleano/rota em pendência,
reload/relogin/conflito, aceita outra rota e distingue persistência de efeito nas sessões.
Topologia nova não remove consumidores antigos de operação pendente por inferência.

**Limite necessário:** GM mantém conexões. Ban/desconexão e E5 aguardam fencing global e
ordenação de todos os produtores, além da base local B170 abaixo. Nenhuma escrita de
personagem foi habilitada. Publicação/migração de public somente com novo pedido.

## Saída e fotografias locais (B170)

Queda do barramento, PlayerLogout e C2S logout usam o mesmo caminho no GS. Conexão e localsid
precisam corresponder à sessão para comandar/encerrar o personagem; entrada duplicada é recusada.
Antes de I/O, o GS retira a entidade da simulação e conserva fotografia/sessão reservadas.
Status, atributos/pontos, chi, modo roupa, waypoints e cinco listas de missão compartilham
uma transação. Confirmação libera sessão/rota e responde logout; falha/timeout conservam
`em_transicao`, bloqueiam reentrada e repetem a fotografia a cada 1 s (política interna).
Nenhum I/O segura world/tick. Política interna: lock_timeout 1,5 s, statement_timeout
2 s, confirmação de commit 2 s e teto externo de tentativa 3 s. Origem: player.cpp:9602–9694 do fonte 1.5.3, WAITING_LOGOUT,
user_save_data e callback antes da resposta; fotografia limitada ao estado já portado.

Clones do CharacterRepository compartilham controle por personagem no GS (main.rs:77–82).
Comandos, entrada, saída, transferência e gravação de autosave se excluem por personagem.
Carimbo monotônico em memória invalida fotografia antiga, inclusive da sessão anterior ou
mapa de origem; captura não espera banco no tick. Fotografia capturada durante comando
é descartada após sua conclusão. Gravação de estado ALTERADO das missões e modo roupa termina
antes de liberar o comando, fora do lock do mundo.

Transferência congela entidade/mascote e conserva grupo/convites na origem e só publica destino depois de confirmar a
fotografia. Erro anterior ao commit restaura origem. Erro/timeout no commit é desconhecido:
reserva permanece, repete o mesmo destino até confirmar; queda/logout nesse intervalo
leva à saída salva no destino, sem ressuscitar sessão da conexão morta.

**Alcance parcial:** controle e fotografias de recuperação ficam em memória deste GS.
Não há lease global, revisão persistida ou diário de saída/transferência para reinício do
GS. Ainda falta integrar eventos/combate/equipes/sessões temporizadas e gravações assíncronas de itens, habilidades,
mascotes, durabilidade, munição e configuração do link. Perda do GS pode perder fotografia
não confirmada. Ausência observada NÃO autoriza edição offline; ban/desconexão/E5 continuam
bloqueados por essas dependências. Testes: workspace 942/4 (2 ignorados); após correções GS 261/0 (2 ignorados),
canal final 15/0 após conservar pares opacos de charactermode. Integral não repetida
após correções; referências detalhadas no B170/estado/memória.

## Contrato de destino para escritas E4–E8

Esta seção é projeto para as demais escritas, não capacidade disponível.

1. Estender o canal separado já testado na E3, preservando autenticação, limite, timeout
   e proteção do realm. Comandos transacionais de conta já têm recuperação; operações com
   GM já tem contrato de revisão/recibos acima; outros efeitos em memória exigem coordenação própria.
2. Envelope: `operacao_id`, `realm_id`, `personagem_id`/`mapa`, `administrador_id`, tipo,
   parâmetros e revisão esperada. O daemon verifica o realm e a revisão, além da autenticação.
   Campos de conta têm alcance global explícito. Nunca usar identidade falsa de jogador GM.
3. Resposta: mesmo identificador, alvo, origem (`viva`/`persistida`), revisão e estado
   (`aplicado`, `salvo`, `falha`, `desconhecido`). Timeout depois do envio é desconhecido;
   consultar o mesmo identificador antes de repetir. Mesmo id com payload diferente é conflito.
4. A fatia E3 de leitura está testada; a guarda de presença coordena fotografias com
   entrada/saída/troca. Essa observação não é exclusão para edição offline ou garantia
   contra autosave antigo; tais condições precisam ser estabelecidas antes das escritas.
5. Antes de E5, introduzir ordenação/revisão por personagem para todos os produtores de
   gravação, inclusive autosave, logout e transferência. Operações offline adquirem exclusão
   com login. Fotografias antigas não podem vencer revisões novas. Banco fora do world.tick
   e de qualquer lock do mundo; recálculo/pacotes apenas pelo WorldProtocol e evidência.
6. Escritas idempotentes exigem resultado recuperável após reconexão/reinício. Registro
   técnico de deduplicação é permitido, sem construir histórico navegável de operações.
   Esquema/atomicidade para senha estão acima; efeitos em memória precisam de nova análise.
7. Mapas: fechar entrada, desconectar/salvar e finalizar mapa; falha de salvamento não vira
   sucesso. Não terminar todos os mapas de um processo. Rates e moldes precisam de confirmação
   de consumo efetivo e sobrevivência a reinício. Recálculo de existentes é uma ação separada.

## Verificação por fatia

- E2: autorização e revogação real no schema `test`, Redis isolado, CSRF, logout/replay,
  indisponibilidade, limites e ausência de sucesso/escrita nas rotas legadas.
- E3: daemon errado, origem viva/persistida, entrada/saída concorrente, timeout e desconexão.
- E4–E6: repetir operação, autosave antigo, logout/relogin, troca de mapa, saldo global,
  atualização efetiva nos dois clientes e layouts com evidência do original.
- E7/E8: jogadores desconectados/salvos, entrada bloqueada, reinício, rates consumidos,
  personagem novo e preservação da progressão no recálculo explícito.
- Interface: conferir login, erro, alternância de realm, atualização, estado indisponível,
  teclado e largura móvel. Teste de API/HTML não substitui renderização no navegador.
