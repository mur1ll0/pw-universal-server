# Especificação 06: Painel administrativo e patcher

> Verificada em 2026-10-07, base `88ceaea` (B191) + B192 (sem commit).
> Cobre `web-admin/`, `crates/pw-gs/src/administracao.rs` e `tools/pw-patch-tool/`.
> Reforma: `docs/admin/MEMORIA_DA_REFORMA.md`.

## 1. Painel — base autenticada, reforma parcial

| peça | estado atual |
| :--- | :--- |
| API | FastAPI modular em `backend/painel/`; `main.py` só cria a aplicação |
| Frontend | `backend/static/` (HTML/CSS/JS/SVG locais, sem CDN/build, CSP só `self`). Design próprio escuro (B174): menu lateral com "Geral" (Visão geral, Contas globais) e "Realm" (painel, personagens; mapas/rates/moldes marcados "em breve"); seletor de realm no topo; home com cartões por realm (online ao vivo, personagens, rates); conteúdo na largura toda; contas em cartões paginados (12) com busca; clicar no cartão abre um popup ancorado (resumo + ações GM, Senha, Gold, Banir/Desbanir, Desconectar) que fecha ao clicar fora ou ao concluir; criação em modal; todo resultado num alerta padrão (sucesso/erro/aguardando, estilo SweetAlert) e operação sem confirmação é consultada sozinha a cada 1,5 s, até ~45 s (B175) |
| Login | Conta PW com `gm_privileges > 0`, sem ban; `pw-validar-credenciais` reutiliza `pw_crypto::verify_password` |
| Sessão | Redis, prazo fixo de 8 h; cookie HttpOnly/SameSite Strict, Secure configurável; origem e CSRF nas escritas |
| Revogação | Conta, GM, ban e impressão do hash revalidados por requisição; logout e novo login invalidam o token anterior do navegador |
| Realms | `realms`, contagem persistida em `characters`; TCP exclusivamente no alvo correto, `ADMIN_GATEWAYS` para hosts Docker. `/api/realms` traz `rates` gravadas (`realms.double_{exp,sp,drop,gold}_multiplier`) e `canal_administrativo`; `/estado` traz `taxas` em memória do GS (só se todos os daemons concordam) |
| Personagem E5 | `POST /api/realms/{id}/personagens/{pid}/editar {operacao_id` + **um** de `dinheiro` 1–2·10⁹ (dar, B180), `exp`/`sp` (dar), `pontos` 1–10 000 (dar), `nivel` ≥ 2 (só sobe), `cultivo` 0–255, `atributos` [força, agilidade, vitalidade, energia] 0–100 000 ou `redistribuir: true` (B184), `posicao` `{mapa, x, y?, z}` (E6, B185), `item` `{id, quantidade 1–100 000}` (E6, B186), `remover_item` `{recipiente, slot 0–255, id, quantidade?}` (E6, B187)`}` → `editar_personagem` no GS (regra, limites por realm/versão e códigos em spec 05 §7.11): `aplicado` (online) ou `salvo` (offline; EXP/SP só online); falhas `precisa_estar_online`, `em_transicao`, `personagem_inexistente`, `nivel_invalido`, `cultivo_invalido`, `atributos_invalidos`, `sem_mudanca`, `mapa_indisponivel`, `fora_do_mapa`, `altura_obrigatoria`, `bolsa_cheia`, `item_inexistente`, `slot_mudou`, `quantidade_invalida`, `precisa_estar_offline`, `operacao_em_conflito`. Leitura (B186): `GET …/personagens/{pid}/inventario` (bolsa, equipamento, armazém, missão, com nomes) e `GET …/itens?busca=` (1–64 caracteres, até 30), pelo primeiro GS do realm. Ficha com linhas Dinheiro, EXP/SP, Pontos livres, Nível, Atributos (os quatro preenchidos, "Pontos livres depois" ao vivo, Aplicar e Redistribuir), Posição (mapa entre os ligados com nome, x/y/z da posição atual, Y vazio = chão), Cultivo e o bloco Itens (quatro recipientes; Dar item com busca por nome/ID, clique escolhe, quantidade; clicar num item dos recipientes abre a barra Remover com quantidade e confirmação). **Grade com ícones (B188):** recipientes em grade de 8 colunas (equipamento com o nome de cada slot, `EQUIPIVTR_*`, `EC_IvtrTypes.h:56-95`), ícone de cada item por `GET /api/icones/{m|f}/{hex}.png` — `hex` = nome do arquivo do `file_icon` em bytes GBK, que o GS manda como `icone` no inventário e na busca; o painel recorta a célula do atlas `data/icones/iconlist_ivtr{m,f}.{dds,txt}` (DXT1 → PNG, sem dependência de imagem, cache em memória e `Cache-Control` de 1 dia; atlas por sexo do personagem). Atlas único do cliente 1.5.5 para as duas versões (cobre 98,3% dos ícones do realm 155 e 96,1% do 126, o mesmo que o atlas do 1.2.6); fora do atlas, as duas primeiras letras do nome. **Dica (B189, parte 1):** ao passar o mouse, `GET …/personagens/{pid}/itens/{recipiente}/{slot}/dica` → GS `detalhe_item` (registro + bloco do equipamento lido por `ConteudoDeEquipamento::ler`: requisitos, durabilidade, valores da arma/armadura/ornamento/munição, furos com pedras, efeitos id+parâmetros, fabricante) → `painel/dica.py` monta as linhas na ordem e nas cores do cliente (`EC_IvtrWeapon.cpp:271-400`) com os textos do cliente 1.5.5 BR em `data/textos/` (`item_desc.txt` pelo índice `ITEMDESC_*` de `painel/item_desc_indices.json`, `item_ext_desc.txt`, `item_color.txt` + `l_aNameCols`; gerados por `scripts/gerar_textos_de_itens.py`). Só frases conferidas até o índice 112 (dali o arquivo BR diverge do cabeçalho do fonte). **Parte 2 (B192, testado):** cada efeito com o texto do `AddOneAddOnPropDesc` (`EC_IvtrEquip.cpp:971-2610`, ramo não local do item do jogador) — tabela `painel/efeitos_do_cliente.json` gerada do fonte por `scripts/gerar_efeitos_de_itens.py` (157 tipos, 127 com todas as frases ≤ 112; refino 200–212 sem linha), tipo pelo `item_ext_prop.txt` (`textos.tipos_de_efeito`: 2.441 efeitos, 170 tipos); afiador (100–115) e efeitos de pedra/conjunto/gravação (`0x8000`/`0x10000`/`0x20000`) sem linha ali; id fora do arquivo = `ITEMDESC_ERRORPROP`; os 29 tipos com frase > 112 e a habilidade (55) ainda saem crus (B193). Linha de classe (`AddProfReqDesc`): some com todas as classes da versão (12 no 1.5.5, 8 no 1.2.6), branca/vermelha pela classe do personagem. Ordem corrigida pela arma (`EC_IvtrWeapon.cpp:375-460`): classe, requisitos, efeitos, pedras, preço, fabricante; sem confirmação o painel consulta o resultado pelo mesmo ID (B179, B182, B184) |
| Mapas E7 | Tela Mapas (B183): **todos** os mapas da versão — `GET /api/realms/{id}/catalogo-mapas`, do `gs.conf` original com nomes do `instance.txt` do pwadmin 1.5.5 (`painel/catalogo_mapas.json`, gerado por `scripts/gerar_catalogo_de_mapas.py`; 126: 43, 155: 79; o 126 usa os nomes do 155 pela mesma chave, não conferidos no cliente 1.2.6) — juntos ao `/estado` (`mapas` com `ligado`/`carregando`, `carregaveis`). Lista com ligados primeiro, filtro por número/nome/chave e por status, seleção por linha, selecionar todos (os visíveis)/desmarcar todos, Ligar/Desligar selecionados (um pedido por mapa, confirmação ao desligar, acompanha a carga sozinha até ~45 s); "Sem dados" quando nenhum GS sabe montar. `POST /api/realms/{id}/mapas/{mapa} {ligado}` → `definir_mapa` a cada GS; se todos respondem `mapa_nao_servido`, de novo só ao primeiro com `carregar`. Regra em spec 05 §7.10 (B177, B183) |
| Rates E7 | `POST /api/realms/{id}/rates {exp,sp,drop,moedas}` (0,1–99,9) → `definir_taxas` a todos os GS do realm: grava em `realms` e vale na hora (regra em spec 05 §7.9). Tela Rates do realm com 4 campos; o painel do realm mostra "Em vigor" quando o gravado = o que o GS usa (B176) |
| Estado vivo | Consulta autenticada aos daemons configurados; mapas reais e contagem de sessões com entidade e roteamento; falha/transição não vira zero jogadores |
| Personagens | **B190 (testado):** `GET /api/realms/{id}/personagens?busca&pagina&por_pagina` (1–48, padrão 12; `_`/`%` literais) — personagens do realm de **todas as contas**, busca pelo nome do personagem **ou** da conta, com `total`, `usuario`, `conta_id`, `sexo` e `classe_nome` (o texto do cliente: `GetProfName`, `EC_GameRun.cpp:3448` → `FIXMSG_PROF_*`; no `fixed_msg.txt` 1.5.5 BR as posições 32–39, 229–230, 282–283 batem com `EC_FixedMsg.h`); a ficha também traz `classe_nome`. Tela: **cartões** (nome, conta, classe, nível) com paginação; clicar abre a **tela do personagem** (Voltar retorna à consulta) com **janelas recolhíveis**: Estatísticas (ficha + edições E5), Equipamento, Roupas (slots de moda 13–16, 25, 29 do próprio equipamento, `EC_IvtrTypes.h:56-85`), Inventário (dar/remover), Armazém, Bolsa de missão; Habilidades, Mascotes e Missões aparecem como "em breve". **B191 (testado):** arrastar entre as janelas (HTML5 drag-and-drop) = `mover_item {de, slot_de, id, para, slot_para}` (regras na spec 05 §7.11); recusa `precisa_estar_offline` abre **"Desconectar e aplicar"**: o desconectar da E4, espera o GS declarar ausência (até 20 s) e repete a edição offline com operação nova. Ficha viva quando exatamente um GS declara presença; origem e limitações explícitas |
| Contas E4 | `/api/contas?busca&pagina&por_pagina` (1–48, padrão 12; `_`/`%` literais) com `total`, personagens, criação e último login; criação, senha, GM, gold e ban via GS, recuperáveis, roteados pelo realm selecionado com canal ou pelo primeiro que tenha; GM coordena caches/efeitos globais, cliente requer reentrada. **Gold** (B175; só dar desde B180): `POST …/contas/{id}/gold {delta ≥ 1}` em unidades do cash (100 = 1 gold), soma atômica; valor ≤ 0 recusado em todas as camadas (`valor_invalido`), depois `atualizar_cash` em todos os GS reenvia `PLAYER_CASH` (253) a quem está online. **Ban** (B175): `POST …/contas/{id}/ban {banida, motivo≤120}` grava `is_banned`/`ban_reason`, sem expiração (o link não lê `ban_expires_at`), recusa banir a própria conta (`proprio_administrador`); ao banir, desconecta em todos os realms. **Desconectar** (B175): `POST /api/contas/{id}/desconectar` manda `desconectar` a todos os GS com canal; cada um salva pelo caminho do logout e o link recebe `PlayerLogout` com `result` 2 (cliente volta ao login, `EC_GameSession.cpp:5420-5426`); idempotente, sem registro durável. Limitação: sessão parada na seleção de personagem não está em GS nenhum e não é desconectada (o próximo login de conta banida é recusado) |
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
Em 2026-10-05 (B173–B174): B165–B174 publicados (link/GS 126/155 e painel), migrações no
`public`, chaves `ADMIN_SECRET_126/155` geradas em `docker/.env` (ignorado pelo git). Canal
dos dois GS escutando; GM concedido e removido pela API real, aplicado nos 4 processos.

### Canal E3 — consultas, senha e criação global recuperável, testado automaticamente

TCP separado do barramento: `ADMIN_LISTEN` padrão `0.0.0.0:29110`, sem porta publicada.
Ativado somente com `ADMIN_SECRET` de 32 bytes em hexadecimal; sem chave, desabilitado.
Compose usa `ADMIN_SECRET_126`/`ADMIN_SECRET_155` sem valor padrão, lidos de `docker/.env`.
Pedido que não saiu do painel (sem chave, conexão recusada antes do envio) responde 503
`estado: falha`, `codigo: canal_nao_enviado` — nada aplicado, repetir é seguro; antes do B174
virava "resultado desconhecido". Só tempo esgotado ou queda após o envio é desconhecido. API recebe mapa
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
