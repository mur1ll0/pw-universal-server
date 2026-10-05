# Especificação 06: Painel administrativo e patcher

> Verificada em 2026-10-05, base `a010ff7` + B165–B170 (sem commit).
> Cobre `web-admin/`, `crates/pw-gs/src/administracao.rs` e `tools/pw-patch-tool/`.
> Reforma: `docs/admin/MEMORIA_DA_REFORMA.md`.

## 1. Painel — base autenticada, reforma parcial

| peça | estado atual |
| :--- | :--- |
| API | FastAPI modular em `backend/painel/`; `main.py` só cria a aplicação |
| Frontend | Fonte única em `backend/static/`: HTML, CSS, JavaScript e SVG locais, português, sem CDN/build |
| Login | Conta PW com `gm_privileges > 0`, sem ban; `pw-validar-credenciais` reutiliza `pw_crypto::verify_password` |
| Sessão | Redis, prazo fixo de 8 h; cookie HttpOnly/SameSite Strict, Secure configurável; origem e CSRF nas escritas |
| Revogação | Conta, GM, ban e impressão do hash revalidados por requisição; logout e novo login invalidam o token anterior do navegador |
| Realms | `realms`, contagem persistida em `characters`; TCP exclusivamente no alvo correto, `ADMIN_GATEWAYS` para hosts Docker |
| Estado vivo | Consulta autenticada aos daemons configurados; mapas reais e contagem de sessões com entidade e roteamento; falha/transição não vira zero jogadores |
| Personagens | Busca persistida por realm e ficha viva quando exatamente um GS declara presença; origem e limitações explícitas; edição indisponível |
| Contas E4 | Busca global; criação, senha e GM via GS, recuperáveis; GM coordena caches/efeitos globais, cliente requer reentrada; gold/ban/desconexão pendentes |
| Capacidades | Configuração consultável; consulta viva depende do canal ativo; protocolos 126/155 presentes; outras versões não validadas |
| Docker | Porta 8000; contexto raiz, ignore específico e estágio Rust do verificador |
| Manual e contratos | `docs/WEB_ADMIN_USER_GUIDE.md`; `docs/admin/ARQUITETURA_E_CONTRATOS.md` |

| rota | comportamento |
| :--- | :--- |
| `POST /api/sessao/entrar` | GM e credenciais; corpo limitado a 8192 bytes antes de acumulação, inclusive chunks; 10 tentativas/conta e 20/IP por minuto; 4 verificações simultâneas |
| `GET /api/sessao` | Usuário e CSRF; sem senha/hash/token de sessão |
| `POST /api/sessao/sair` | CSRF e invalidação da sessão |
| `GET /api/realms` | Configuração, gateway, origem e capacidades |
| `GET /api/realms/{realm}/estado` | Mundo/mapas vivos; falha de qualquer daemon ou mapa duplicado deixa estado desconhecido |
| `GET /api/realms/{realm}/personagens?busca=` | Até 50 personagens não excluídos do realm; nomes/níveis persistidos |
| `GET /api/realms/{realm}/personagens/{id}` | ID positivo de 32 bits e realm conferidos no banco; ficha viva ou persistida com presença explícita |
| `GET /api/contas?busca=` | Até 50 contas globais; ID, usuário, GM, ban e saldo como texto; sem credenciais |
| `POST /api/realms/{realm}/contas/{id}/senha` | ID estável do navegador, ASCII imprimível de 1 a 64 bytes; API com CSRF; escrita pelo GS na conta global |
| `POST /api/realms/{realm}/contas` | Criação global pelo GS: usuário ASCII `[A-Za-z0-9_]`, 1–64 bytes, salvo em minúsculas; senha do mesmo contrato algo=0; ID e resultado atômicos |
| `POST /api/realms/{realm}/contas/{id}/gm` | Booleano GM global via GS; commit dá persistência salva/efeito pendente; recibos confirmam aplicação no servidor; cliente requer reentrada |
| `GET /api/realms/{realm}/operacoes/{id}` | Recupera resultado do mesmo administrador; qualquer realm com GS configurado acessa registro global |
| Demais `/api/*` | Exigem login; operações retiradas/pendentes retornam 501 sem aplicação |

Limites são políticas do painel, não regras do jogo. Todo corpo de escrita é limitado a
8192 bytes, inclusive chunks; validação Pydantic tem resposta genérica sem ecoar senhas.
Verificador recebe segredos por stdin,
com timeout de 5 s, sem alterar hash ou criar ticket de jogo. Falha de banco/Redis/verificador
não produz sucesso. Sem CORS irrestrito nem documentação API pública. Cookie Secure por
padrão; Compose usa HTTP na LAN com `ADMIN_COOKIE_SECURE=false`; em HTTPS, configurar `true`.

**Testado automaticamente (B166):** 37 testes Python com banco no schema `test` e Dragonfly
isolado, incluindo API contra binário real do GS; 6 testes do canal e 3 de vários mapas.
Regressão dos subcomandos: 152 passaram, 0 falharam, 2 ignorados; 5 testes da topologia
passaram, incluindo proibição de publicar o canal administrativo. JavaScript, build nativo
e Compose verificados. B165 também verificou crypto/bus e imagem Linux da base E2.
**Testado automaticamente (B167):** 41 Python, incluindo senha/API/GS real, perda da resposta
após commit e recuperação após reinício; 110 Rust focados (8 canal, 4 atomicidade e
98 unidades de link/GS/storage); 2 testes Node do fluxo de ID pendente/conflito/relogin.
Suíte completa com TEST_DATABASE_URL: 928 passaram, 0 falharam, 2 ignorados.
**Falta:** inspeção visual (nenhum navegador conectado), confirmação do Murillo,
exclusão offline/persistência (E5), restante de contas (E4), subsistemas (E6),
mapas/rates (E7), moldes (E8), extensões/fechamento (E9).
Base publicada em `pw-admin-api` em 2026-10-04, por pedido explícito do Murillo.
Página/assets HTTP 200 e API anônima 401 verificados; realms/mundos não foram reiniciados.
Em 2026-10-05, o painel em execução contém frontend E3 e alvos 126/155 configurados,
porém os mundos de 2026-10-03 não têm ADMIN_SECRET e recusam TCP 29110.
Consultas vivas não foram confirmadas nesse ambiente. B167–B169 permanecem locais; sem migração
de `public`, publicação ou verificação da imagem Linux de B167/B168.

### Canal E3 — consultas, senha e criação global recuperável, testado automaticamente

TCP separado do barramento: `ADMIN_LISTEN` padrão `0.0.0.0:29110`, sem porta publicada.
Ativado somente com `ADMIN_SECRET` de 32 bytes em hexadecimal; sem chave, desabilitado.
Compose usa `ADMIN_SECRET_126`/`ADMIN_SECRET_155` sem valor padrão. API recebe mapa
`ADMIN_CHANNEL_SECRETS` e lista de alvos por realm em `ADMIN_DAEMONS`; processos adicionais
precisam constar nessa lista. A consulta não descobre processos ausentes da configuração.

Quadro: u32 big-endian de tamanho + JSON UTF-8 (1–8192 bytes). Saudação v1 com desafio
aleatório de 32 bytes; pedido/resposta acrescentam MAC de 32 bytes, HMAC-SHA256 de
`desafio || sentido || corpo`, sentidos `pedido`/`resposta`. Cada conexão tem novo desafio,
um pedido e limite de 3 s; GS admite até 32 conexões. Políticas internas, sem pacote cliente.
Envelope contém `operacao_id`, `realm_id`, `administrador_id` e `consulta` (`mundos`,
`personagem`, `trocar_senha`, `criar_conta`, `definir_gm` ou `resultado`). GS confere MAC, realm e GM/ban no banco;
API confere MAC, realm, correlação, estado e alvo do comando.

`ComandoAdministrativoRepository` mantém registro PostgreSQL global por ID, administrador,
conta, realm de origem, impressão de parâmetros e resultado. Senha/hash de login não vão ao
registro técnico. Não há expurgo automático: remover IDs permitiria reaplicação antiga.
Na criação, conta é NULL na reserva; resultado salvo preenche o ID alocado no mesmo commit.
Mudança de rota/realm não altera a impressão. Mesmo ID com administrador/alvo/parâmetros
diferentes é conflito. Reserva do ID, lock das contas em ordem crescente, revalidação de
GM/ban, senha e resultado compartilham a transação; rollback desfaz tudo. Banco fica fora
dos locks do mundo/tick. Limites internos: lock 1500 ms, instrução 2000 ms, conexão 3 s.

Senha usa `hash_legacy_pw_md5` para o Challenge algo=0 do cliente original
(`Network/gameclient.cpp:131-139`, fontes 1.5.5 e 1.5.3). Resultado `salvo` confirma senha e
registro no mesmo commit; vale nos próximos logins globais, mantendo sessões de jogo abertas.
O link verifica HMAC-MD5 do desafio de 16 bytes e recusa tokens, prova de outro desafio e
novo Response na sessão autenticada. Argon2/texto não são verificáveis nesse handshake;
sementes antigas incorretas foram corrigidas somente para banco novo, sem reset de contas.
Para hashes incompatíveis, redefinir explicitamente via E4 antes de testar login no jogo.
Digitar o usuário em minúsculas corresponde à chave gerada por `hash_legacy_pw_md5`.

Timeout/perda da resposta/registro ausente são `desconhecido` (HTTP 202), pois um commit pode
ter ocorrido ou estar em andamento. Recuperação por ID não reaplica. Falha definitiva de
alvo é registrada; conflito de repetição preserva o comando original. Não há fila persistida
nem estado `aplicado` para esta operação: seu único efeito é transacional, com estado `salvo`.
UI conserva metadados de um comando pendente por administrador no localStorage, sem senha;
relogin/reload permitem consulta ou repetição com mesmo ID/alvo. Trocar realm muda só a rota.

Snapshot usa guarda do roteador para entrada, logout, queda do link e transferência;
consulta durante transição retorna `em_transicao`. Ficha copia `PlayerEntity` sob lock
curto, sem banco/rede dentro do lock do mundo. EXP/alma/dinheiro i64 vão como texto.
Entidade sem dono após queda do link é `inconsistente`, não offline. Banco/Redis não
atestam presença. Ausência em todos os daemons configurados é observação, sem exclusão
com um login futuro; nenhuma edição de personagem é autorizada nesta fatia. Revisão,
exclusão offline e coordenação dos produtores de gravação de personagem continuam pendentes.

**Criação B168:** API/GS validam nome e senha, sem trim silencioso. Nome canônico minúsculo
conforme `ElementClient/EC_LoginSwitch.cpp:329`; senha via `Network/gameclient.cpp:131-139`.
Limite ASCII do painel evita divergência de codificação (`EC_LoginSwitch.cpp:332-336`);
não é alegação de limite original. Impressão inclui tipo, administrador, nome canônico e
resumo da senha; variação de caixa é o mesmo parâmetro. Reserva, revalidação GM/ban sob
lock, INSERT, ID e resultado são transacionais. Defaults: gold/silver/GM zero e sem ban.
Índice UNIQUE em LOWER(username), igual à autenticação/busca, arbitra concorrência inclusive
fora do painel; conflito de nome é `usuario_existente`, durável (HTTP 409). Migração
`scripts/2026_10_05_criacao_contas.sql` aborta em duplicatas antigas; nunca renomeia contas.
Aplicada só em test. Sequência SERIAL pode deixar lacunas no rollback, sem conta/reserva.
UI guarda tipo/nome/rota/ID, nunca senha; bloqueia outro tipo de escrita enquanto pendente,
preserva ID em conflito/recusa de autorização, consulta após reload/relogin/outro realm.
Resultado de outro tipo/nome/ID não encerra recuperação. Ausência/perda/timeout é desconhecida.
Testes automatizados: 47 Python, 117 Rust focados, 4 Node passaram, sem falhas; credenciais
criadas consumidas pelo dispatcher real dos links 126/155 sem GM. Workspace com banco:
935 passaram, 0 falharam, 2 ignorados. Visual/jogo/Linux pendentes.

**GM B169:** revisão singleton transacional + `accounts.revisao_gm`; reserva, autorização
sob locks ordenados, alteração e resultado atômicos. GM booleano 0/1, sem hierarquia nova.
`ADMIN_COORDENACAO_ALVOS` é o conjunto coordenado (Compose: links/GS 126/155; 148/153 fora, GM lá só no próximo login).
Coordenação opcional: registro falho só desliga a coordenação (aviso), não derruba link/GS.
`ADMIN_COORDENACAO_ID` identifica cada processo. Fencing por conexão/encarnação exclusiva;
reinício zera recibo. Fotografia consistente fora do tick/locks do mundo; reconciliadores
atualizam caches dos links/GS e removem efeitos quando revogados. Recibo avança só depois
da aplicação nas sessões; 1 s entre consultas, validade 5 s (políticas internas).
Autorização consumida em GM/GOTO/missões revalida GM/ban/revisão sob FOR SHARE, fora do
lock do mundo. GOTO preserva consumo atual no banco, sem exigir cache já reconciliado.
Falha de banco nega GM e fotografia falha não confirma revisão. Perda da
conexão de fencing desliga a coordenação sem derrubar o daemon (B173); reinício reconcilia.
Estados: `pendente` + `persistencia=salva`, `aplicado` com sessões reconciliadas no servidor,
`substituido` por revisão posterior, `desconhecido` em timeout/ausência. Consulta verifica
conta e recibos no mesmo snapshot; consultas concorrentes devolvem o vencedor terminal durável. Repetição não desfaz
revisão nova; conflito preserva original. UI guarda booleano/tipo/conta/ID/rota e conserva
pendência entre reload/relogin/outro realm; nenhum segredo persistido.
Cliente recarrega auth no SelectRole_Re (155 `EC_GameSession.cpp:4581`, 153 `:4513`);
sem pacote online comprovado, indicar reentrada. Conexões GM são mantidas. B170 coordena saída/autosave/troca localmente;
ban/desconexão/E5 ainda exigem fencing/revisão persistidos e integração de todos os produtores.
B170: rodada workspace 942/4 (2 ignorados); após correções GS 261/0 (2 ignorados)
e canal final 15/0. Suíte integral não repetida depois das correções; visual/jogo pendentes.
Contrato detalhado: `docs/admin/ARQUITETURA_E_CONTRATOS.md`, B169. Migração
`scripts/2026_10_05_coordenacao_gm.sql` somente em test; imagem Linux/visual/jogo pendentes.
Testes de API/links/GS reais verificaram auth, invencibilidade, revogação e reinício;
isso não confirma os clientes originais. B169: workspace 940/0, 2 ignorados;
armazenamento após a última correção 14/0, canal Rust 10/0, Python final 49/0, Node 5/0.

Backend anterior substituído, incluindo escritas diretas e seed genérico de moldes.
Demais escritas de contas/personagens/itens/habilidades/mapas/rates/anúncios/templates indisponíveis
até integração real. `elements_decoder.py` preservado como referência, sem importação na API:
fallbacks de elements e ícones podem usar outro realm (`:500-514`, `:774-790`). Catálogo/ícones
aguardam correção e validação da origem. Migração: `scripts/2026_10_05_comandos_administrativos.sql`,
aplicada só em `test`; inicialização nova inclui tabela em `public` e `test`.

Sem histórico navegável, hierarquia adicional de administradores, correio em massa ou
launcher neste escopo. Logs técnicos sem credenciais; deduplicação é interna.
Regras/layouts do jogo continuam no GS e no WorldProtocol.

## 2. Patcher (`tools/pw-patch-tool/`) — protótipo

CLI Rust de ~140 linhas (`docs/PW_PATCH_TOOL_GUIDE.md`):

| comando | faz |
| :--- | :--- |
| `scan <dir_cliente>` | catálogo SHA-256 dos arquivos |
| `create-patch <dir_v1> <dir_v2> <v1> <v2> [notas]` | pacote `.cup` + `patch_manifest.json` |
| `list-patches` | histórico |

`planejado`: diferença **dentro** dos `.pck` (atenção: `models.pck`/`litmodels.pck`/`building.pck`
passam de 2 GB e o motor não os abre — `tools/pw-pck-extract/` lê), compressão zstd,
download retomável com HTTP Range, verificação por SHA-256 antes de aplicar, launcher
gráfico, e atualizar `serverlist.txt` (UTF-16LE com BOM) e o `.bat` com `logiccheck:0`.
