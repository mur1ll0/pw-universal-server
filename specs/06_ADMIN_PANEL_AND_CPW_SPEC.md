# Especificação 06: Painel administrativo e patcher

> Verificada em 2026-10-11, base `804f19e` (B216) + B217 (Templates, sem commit).
> Cobre `web-admin/`, `crates/pw-gs/src/administracao.rs` e `tools/pw-patch-tool/`.
> Reforma: `docs/admin/MEMORIA_DA_REFORMA.md`.

## 1. Painel — base autenticada, reforma parcial

| peça | estado atual |
| :--- | :--- |
| API | FastAPI modular em `backend/painel/`; `main.py` só cria a aplicação |
| Frontend | `backend/static/` (HTML/CSS/JS/SVG locais, sem CDN/build, CSP só `self`). Design próprio escuro (B174): menu lateral com "Geral" (Visão geral, Contas globais) e "Realm" (painel, personagens, mapas, rates, Templates); seletor de realm no topo; home com cartões por realm (online ao vivo, personagens, rates); conteúdo na largura toda; contas em cartões paginados (12) com busca; clicar no cartão abre um popup ancorado (resumo + ações GM, Senha, Gold, Banir/Desbanir, Desconectar) que fecha ao clicar fora ou ao concluir; criação em modal; todo resultado num alerta padrão (sucesso/erro/aguardando, estilo SweetAlert) e operação sem confirmação é consultada sozinha a cada 1,5 s, até ~45 s (B175) |
| Login | Conta PW com `gm_privileges > 0`, sem ban; `pw-validar-credenciais` reutiliza `pw_crypto::verify_password` |
| Sessão | Redis, prazo fixo de 8 h; cookie HttpOnly/SameSite Strict, Secure configurável; origem e CSRF nas escritas |
| Revogação | Conta, GM, ban e impressão do hash revalidados por requisição; logout e novo login invalidam o token anterior do navegador |
| Realms | `realms`, contagem persistida em `characters`; TCP exclusivamente no alvo correto, `ADMIN_GATEWAYS` para hosts Docker. `/api/realms` traz `rates` gravadas (`realms.double_{exp,sp,drop,gold}_multiplier`) e `canal_administrativo`; `/estado` traz `taxas` em memória do GS (só se todos os daemons concordam) |
| Personagem E5 | `POST /api/realms/{id}/personagens/{pid}/editar {operacao_id` + **um** de `dinheiro` 1–2·10⁹ (dar, B180), `exp`/`sp` (dar), `pontos` 1–10 000 (dar), `nivel` ≥ 2 (só sobe), `cultivo` 0–255, `atributos` [força, agilidade, vitalidade, energia] 0–100 000 ou `redistribuir: true` (B184), `posicao` `{mapa, x, y?, z}` (E6, B185), `item` `{id, quantidade 1–100 000}` (E6, B186), `remover_item` `{recipiente, slot 0–255, id, quantidade?}` (E6, B187), `organizar_bolsa: true` (B320: botão **Organizar** no cabeçalho da janela Inventário, 1.2.6 e 1.5.5 — o GS faz a conta do botão do cliente 1.5.5 e aplica as trocas; online cada uma vai ao cliente como `EXG_IVTR_ITEM`; devolve `trocas`)`}`. **Consulta desconhecida (B322):** o GS lê o envelope e a consulta à parte; consulta que ele não conhece (painel publicado antes do GS) volta como falha `consulta_desconhecida` com a mensagem do desserializador — antes a conexão fechava calada e o painel ficava "Aplicando" sem fim → `editar_personagem` no GS (regra, limites por realm/versão e códigos em spec 05 §7.11): `aplicado` (online) ou `salvo` (offline; EXP/SP só online); falhas `precisa_estar_online`, `em_transicao`, `personagem_inexistente`, `nivel_invalido`, `cultivo_invalido`, `atributos_invalidos`, `sem_mudanca`, `mapa_indisponivel`, `fora_do_mapa`, `altura_obrigatoria`, `bolsa_cheia`, `item_inexistente`, `slot_mudou`, `quantidade_invalida`, `precisa_estar_offline`, `operacao_em_conflito`. Leitura (B186): `GET …/personagens/{pid}/inventario` (bolsa, equipamento, armazém, missão, com nomes) e `GET …/itens?busca=` (1–64 caracteres, até 30), pelo primeiro GS do realm. Ficha com linhas Dinheiro, EXP/SP, Pontos livres, Nível, Atributos (os quatro preenchidos, "Pontos livres depois" ao vivo, Aplicar e Redistribuir), Posição (mapa entre os ligados com nome, x/y/z da posição atual, Y vazio = chão), Cultivo e o bloco Itens (quatro recipientes; Dar item com busca por nome/ID, clique escolhe, quantidade; clicar num item dos recipientes abre a barra Remover com quantidade e confirmação). **Grade com ícones (B188):** recipientes em grade de 8 colunas (equipamento com o nome de cada slot, `EQUIPIVTR_*`, `EC_IvtrTypes.h:56-95`), ícone de cada item por `GET /api/icones/{m|f}/{hex}.png` — `hex` = nome do arquivo do `file_icon` em bytes GBK, que o GS manda como `icone` no inventário e na busca; o painel recorta a célula do atlas `data/icones/iconlist_ivtr{m,f}.{dds,txt}` (DXT1 → PNG, sem dependência de imagem, cache em memória e `Cache-Control` de 1 dia; atlas por sexo do personagem). Atlas único do cliente 1.5.5 para as duas versões (cobre 98,3% dos ícones do realm 155 e 96,1% do 126, o mesmo que o atlas do 1.2.6); fora do atlas, as duas primeiras letras do nome. **Dica (B189, parte 1):** ao passar o mouse, `GET …/personagens/{pid}/itens/{recipiente}/{slot}/dica` → GS `detalhe_item` (registro + bloco do equipamento lido por `ConteudoDeEquipamento::ler`: requisitos, durabilidade, valores da arma/armadura/ornamento/munição, furos com pedras, efeitos id+parâmetros, fabricante) → `painel/dica.py` monta as linhas na ordem e nas cores do cliente (`EC_IvtrWeapon.cpp:271-400`) com os textos do cliente 1.5.5 BR em `data/textos/` (`item_desc.txt` pelo índice `ITEMDESC_*` de `painel/item_desc_indices.json`, `item_ext_desc.txt`, `item_color.txt` + `l_aNameCols`; gerados por `scripts/gerar_textos_de_itens.py`). Só frases conferidas até o índice 112 (dali o arquivo BR diverge do cabeçalho do fonte). **Durabilidade (correção no B196):** a verdade é a coluna `character_items.durability` (o desgaste e o conserto gravam só nela); o GS remenda os 8 bytes dos octetos com ela antes de qualquer leitura (`BusServer::octetos_atuais`: envio ao cliente, `detalhe_item`, edição, refino/pedras) — antes o painel decodificava os octetos crus e mostrava a peça quebrada com a durabilidade antiga. **Parte 2 (B192, testado):** cada efeito com o texto do `AddOneAddOnPropDesc` (`EC_IvtrEquip.cpp:971-2610`, ramo não local do item do jogador) — tabela `painel/efeitos_do_cliente.json` gerada do fonte por `scripts/gerar_efeitos_de_itens.py` (157 tipos, 127 com todas as frases ≤ 112; refino 200–212 sem linha), tipo pelo `item_ext_prop.txt` (`textos.tipos_de_efeito`: 2.441 efeitos, 170 tipos); afiador (100–115) e efeitos de pedra/conjunto/gravação (`0x8000`/`0x10000`/`0x20000`) sem linha ali; id fora do arquivo = `ITEMDESC_ERRORPROP`; **B193 (testado):** as frases além do 112 pela posição conferida no `ElementClient` 1.5.5 BR — `scripts/conferir_efeitos_no_binario.py` desmonta o `switch` do `AddOneAddOnPropDesc` (despacho em `0x51e337`, índice `0x522ac4`, saltos `0x522884`, `default` `0x522849` = `ERRORPROP`) e grava `painel/efeitos_no_binario_br.json`: 15 frases (`ATK_DEGREE` 202, `DEF_DEGREE` 203, `TOTAL_DEFENCE_ADD` 255, `PROFVIEW` 261, `SOULPOWER` 264, resistências 269–273, `PENETRATION` 276, `RESILIENCE` 277, `VIGOUR` 321, `PPEN`/`MPEN` 397/398), 118 tipos com as chaves do binário iguais às do fonte, e os tipos 161–174, que o BR manda ao `default:` (saem "Atributo de erro (id)" como no cliente). `textos.frases()` só usa a enumeração até 112 mais as conferidas. Fica crua só a habilidade (55, `CECSkill::GetDesc`). Linha de classe (`AddProfReqDesc`): some com todas as classes da versão (12 no 1.5.5, 8 no 1.2.6), branca/vermelha pela classe do personagem. Ordem corrigida pela arma (`EC_IvtrWeapon.cpp:375-460`): classe, requisitos, efeitos, pedras, preço, fabricante; sem confirmação o painel consulta o resultado pelo mesmo ID (B179, B182, B184) |
| Templates E8 | **B217 (testado, falta ver em jogo).** O que o servidor copia para o personagem novo de cada classe (o molde do `clsconfig` do original, `DBCreateRole`, `cnet/gamedbd/dbcreaterole.hrp:46-60`/`:320-360`) é o template do realm no banco (`class_templates` + `_items` + `_skills`), lido pelo `pw-link` **a cada criação** (`CharacterRepository::create_character`); realm sem template nenhum é semeado na partida do link (`TemplateRepository::seed_defaults`: 5/5/5/5, arma da classe, bolsa 2100 + 1796 + 1804 — desde o B218 sem a Poção 1801, nível 30 no 1.5.5) — editar vale para o próximo personagem, sem reinício; os existentes não mudam (sem "aplicar aos existentes", decisão de 2026-10-11). `GET /api/realms/{id}/templates` → canal `templates` (as classes que a versão cria; `existe:false` sem linha): estatísticas, vida/mana **calculadas** (`vida_e_mana_maximas`, não editáveis), bolsa e corpo com nome/ícone/pilha do `elements.data`, habilidades com nome/ícone do cliente, `nivel_maximo` e `piso` dos atributos da versão. `POST /api/realms/{id}/templates/{classe 0–11}/editar {operacao_id` + **um** de `estatisticas` (nível, cultivo, exp, sp, dinheiro, pontos, 4 atributos, mapa, x, y opcional = chão, z), `item {recipiente: bolsa\|equipamento, slot, id, quantidade}`, `remover_item`, `mover_item` (troca), `habilidade {id, nivel; 0 tira}`} → canal `editar_template`, idempotente (`comandos_administrativos`, mesma impressão do personagem). O painel recusa habilidade desconhecida do cliente, de outra classe (aceita a classe e a 255) ou acima do nível máximo. O GS confere: classe da versão, nível 1–máximo, cultivo da versão, atributos ≥ piso, mapa carregado no processo e `(x, z)` no terreno (`mapa_indisponivel`/`fora_do_mapa`/`altura_obrigatoria`), item existente e não de missão, pilha, bolsa 0–31, corpo 0–38 pela máscara (`posicao_invalida`), slots 27/28 sem repetir, reputação 0 e requisitos de nível/classe/atributos do bloco de fábrica (`requisito`; estatísticas que tornariam peça vestida inválida → `requisito_dos_itens`). Peça nasce com a durabilidade de fábrica (×100). Tela: cartões por classe e a ficha do template com as janelas da ficha do personagem — só Estatísticas iniciais (modal com todos os campos e Salvar), Equipamento, Roupas, Inventário e Habilidades; sem armazém, bolsa de missão, mascotes, Daimon e missões (vêm pelo jogo). Criar item e dar habilidade reaproveitam os modais da ficha; toast para todo resultado |
| Mapas E7 | **Confirmado em jogo (2026-10-10, 1.2.6).** Tela Mapas (B183): **todos** os mapas da versão — `GET /api/realms/{id}/catalogo-mapas`, do `gs.conf` original com nomes do `instance.txt` do pwadmin 1.5.5 (`painel/catalogo_mapas.json`, gerado por `scripts/gerar_catalogo_de_mapas.py`; 126: 43, 155: 79; o 126 usa os nomes do 155 pela mesma chave, não conferidos no cliente 1.2.6) — juntos ao `/estado` (`mapas` com `ligado`/`carregando`, `carregaveis`). Lista com ligados primeiro, filtro por número/nome/chave e por status, seleção por linha, selecionar todos (os visíveis)/desmarcar todos, Ligar/Desligar selecionados (um pedido por mapa, confirmação ao desligar, acompanha a carga sozinha até ~45 s); "Sem dados" quando nenhum GS sabe montar. `POST /api/realms/{id}/mapas/{mapa} {ligado}` → `definir_mapa` a cada GS; se todos respondem `mapa_nao_servido`, de novo só ao primeiro com `carregar`. Regra em spec 05 §7.10 (B177, B183) |
| Reiniciar o realm (B249) | `testado` (sem teste em jogo; o GS novo precisa ser publicado). O GS lê os arquivos do `config` uma vez, ao montar o `GameDataManager`; item `servidor`/`ambos` publicado pelo Estúdio só vale depois de reiniciar. **Aguarda reinício:** `GET /api/realms/{id}/servidor` compara a `publicada_em` da última versão que gravou arquivo no realm (`GET /admin/{v}/servidor/ultima` do `pw-recursos`, `RecursosRepository::ultima_do_servidor`) com o `iniciado_em` (segundos Unix, a partida mais antiga entre os GS do realm, novo campo do resumo `mundos`); `aguarda_reinicio` nulo se algum GS não informa. **Reiniciar:** `POST /api/realms/{id}/reiniciar {confirmar: <id do realm>}` (sessão + CSRF; 422 sem a confirmação) → consulta `reiniciar` a cada GS do realm (`aplicar_no_realm`). O GS responde `{tipo: reiniciar, online}` na hora e, em seguida, desconecta todos pelo caminho do logout (`desconectar_todos` → `expulsar_pelo_painel`: trava de gravação, fotografia salva) e encerra o processo; o Docker o sobe de novo (`restart: unless-stopped`) e o `pw-link` reconecta (`uplink.rs:133`). Painel: aviso no topo do Estúdio com "Reiniciar o realm" (pede o id do realm) e acompanhamento até o `iniciado_em` mudar (até 90 s) |
| Recarregar sem reiniciar (B331) | `testado` (sem teste em jogo; o GS novo precisa ser publicado). Consulta `recarregar` (`RoteadorDeMapas::recarregar_em_segundo_plano`): responde `{tipo: recarregar, online}` na hora (a leitura do 1.5.5 passa do prazo de 5 s do canal) e relê o `config` do realm com o mesmo `GameDataManager::load_from_directory` da partida; falha **nova** (arquivo que lia na partida e não lê mais) recusa e mantém os dados em uso. Aplicada: cada mundo passa a usar os dados novos e os jogadores online têm os atributos refeitos; tabelas, catálogos, serviços, distritos, portais e destinos valem na hora, e o que já nasceu do `npcgen` continua até reiniciar. O resumo `mundos` ganha `dados_em` (a última leitura: partida ou recarga) e `recarga {em_andamento, falhas}`; o "aguarda reinício" do painel compara a publicação com o `dados_em`. Painel: `POST /api/realms/{id}/recarregar` (sessão + CSRF) e o botão "Recarregar os dados (sem desconectar)" ao lado de "Reiniciar o realm", com acompanhamento até o `dados_em` mudar (até 120 s) ou a recusa aparecer. Teste: `recarregar_troca_os_dados_sem_desconectar` (canal administrativo 38/38) e o painel 13/13 |
| Rates E7 | **Confirmado em jogo (2026-10-10, 1.2.6).** `POST /api/realms/{id}/rates {exp,sp,drop,moedas}` (0,1–99,9) → `definir_taxas` a todos os GS do realm: grava em `realms` e vale na hora (regra em spec 05 §7.9). Tela Rates do realm com 4 campos; o painel do realm mostra "Em vigor" quando o gravado = o que o GS usa (B176) |
| Estado vivo | Consulta autenticada aos daemons configurados; mapas reais e contagem de sessões com entidade e roteamento; falha/transição não vira zero jogadores |
| Personagens | **B190 (testado):** `GET /api/realms/{id}/personagens?busca&pagina&por_pagina` (1–48, padrão 12; `_`/`%` literais) — personagens do realm de **todas as contas**, busca pelo nome do personagem **ou** da conta, com `total`, `usuario`, `conta_id`, `sexo` e `classe_nome` (o texto do cliente: `GetProfName`, `EC_GameRun.cpp:3448` → `FIXMSG_PROF_*`; no `fixed_msg.txt` 1.5.5 BR as posições 32–39, 229–230, 282–283 batem com `EC_FixedMsg.h`); a ficha também traz `classe_nome`. Tela: **cartões** (nome, conta, classe, nível) com paginação; clicar abre a **tela do personagem** (Voltar retorna à consulta) com **janelas recolhíveis** (B217: recolher leva a janela à bandeja no canto inferior direito e as outras ocupam o espaço; restaurar a devolve ao lugar de origem — também na ficha do template): Estatísticas (ficha + edições E5), Equipamento, Roupas (slots de moda 13–16, 25, 29 do próprio equipamento, `EC_IvtrTypes.h:56-85`), Inventário (dar/remover), Armazém, Bolsa de missão, Habilidades, Mascotes e Missões. **B191 (testado):** arrastar entre as janelas (HTML5 drag-and-drop) = `mover_item {de, slot_de, id, para, slot_para}` (regras na spec 05 §7.11); **B194 (testado):** clicar no item abre o modal **Editar item** (dados da consulta da dica, que agora traz também `classes` — os nomes da versão): quantidade; no equipamento, durabilidade/máx. (unidades internas), refino 0–12, fabricante, requisitos com caixas por classe, pedras por furo e a lista de efeitos sem origem (id + até 3 parâmetros); manda só o que mudou (`editar_item`, `EdicaoItem` na API com os limites do formato) e fecha quando aplica; "Remover…" leva à barra do B187. Regras na spec 05 §7.11. **B195 (testado):** janela **Habilidades** — `GET …/personagens/{pid}/habilidades` lê `character_skills` (a fonte do GS: aprender grava, a entrada lê no `SKILL_DATA`), com o nome do `skillstr.txt` (chave `id × 10`, `EC_Skill.cpp:213-215`; 3.316 nomes) e o ícone do stub do cliente (`ElementSkill/skillNNN.h` → `icon`, `ElementSkill::GetIcon`, `DlgChariot.cpp:554-559`; mapa `painel/habilidades_do_cliente.json` por `scripts/gerar_habilidades_do_cliente.py`, 3.309/3.316 no atlas). Ícones de outros atlas do cliente: `/api/icones/{m|f|habilidade|mascote}/…` — `IconList_Skill` (DXT1 2048×1024) e `IconList_Pet` (DXT3 512×1024) do `surfaces.pck` 1.5.5 em `data/icones/`; `skillstr.txt` em `data/textos/`. **B196 (testado):** na janela, clicar na habilidade abre a linha de nível (Aplicar / Remover); "Ensinar" busca em `GET /api/habilidades?busca=` (até 30, por nome do `skillstr` ou id) e dá no nível escolhido; pedido `habilidade: {id, nivel}` (API confere o id no mapa do cliente e o `max_level` do stub; regras na spec 05 §7.11). **B197 (testado):** janela **Mascotes** — `GET …/personagens/{pid}/mascotes` (canal `mascotes`; nomes das habilidades do `skillstr`), cartões com ícone `IconList_Pet`, modelo, nível e "Invocado"; clicar abre a edição (nível, EXP, lealdade, fome, pontos, nome, habilidades `id:nível`) e "Libertar"; pedido `mascote: {slot, tid, edicao}` (API `EdicaoMascote` com os limites do `pet_data` e as habilidades conferidas no mapa do cliente). **B198 (testado):** janela **Missões** — `GET …/personagens/{pid}/missoes?pagina&busca` (canal `missoes`; em jogo a memória do GS, fora o banco): ativas (etapa recuada sob a principal, abates `feitos/pedidos`, "Em curso"/"Pronta"/"Falhou") e concluídas em páginas de 200 com filtro por nome ou id e botão **Esquecer**; clicar numa ativa abre **Concluir** / **Cancelar** (cancelar só na principal, com confirmação); **Dar** busca em `GET /api/realms/{id}/missoes?busca=` (até 30 de topo, com a lista de etapas quando a missão pede escolha). Pedido `missao: {acao, id, sub?}` (`MissaoPedida`: `dar`, `concluir`, `cancelar`, `esquecer`; id 1–65 535). **B206:** no modal Editar item, cada efeito mostra a descrição do cliente (a mesma da dica, refeita
enquanto se digita: `GET /api/efeitos/{id}?args=`). **B209 (testado, falta ver em jogo):** a busca do modal
passa a `GET /api/realms/{id}/efeitos?item=<tid>&busca=` — o painel acha no catálogo do cliente os ids cujo
texto bate (até 200), pede ao GS (`efeitos_para_item`) o valor do id e a edição, e mostra até 30 que
**existem no realm e servem na família da peça**, com o texto do valor do id ("Acerto +118") e "· fixo"
quando não é editável; `…/efeitos?item=<tid>&ids=a,b` dá o mesmo para as linhas que o item já tem, sem
filtro. Cada linha mostra "Editável · valor do id: 118" (com "(em %)" nos de porcentagem) ou "Fixo pelo
id: …" com o campo travado; trocar o id na linha busca o valor dele; linha recusada avisa "Não serve
nesta peça" / "Não existe neste realm". **B211:** a consulta traz `age` (`efeito_tem_porte`: o efeito
faz algo neste servidor) — a linha diz "· ainda não age no jogo" e a busca "· não age" quando não.
**B213 (efeito de habilidade do item):** o tipo 55 do `item_ext_prop` (`item_skill_addon`) sai como o
cliente: `CECSkill::GetDesc(p0, 1)` (`EC_IvtrEquip.cpp:1829-1836`) → `Skill::GetIntroduction` com o
formato `skillstr` `id × 10 + 1` (`ElementSkill/skill.h:370-376`); as 71 habilidades dadas por efeito
de item não têm parâmetros na introdução, então o formato é o texto (`textos.introducoes_das_habilidades`).
Ex.: 1286 do Punho da Luz Fixa (8292) → habilidade 205 "Garra": "Mordaz: Aumenta seu poder de ataque
físico.". `dica.habilidade_do_efeito` põe `habilidade {id, nome, icone}` na dica do item, em
`/api/efeitos/{id}` e na busca; a linha ganha borda dourada e o ícone da habilidade (atlas do cliente)
ou, sem ícone, o selo ✦ com a habilidade na dica; na busca, "✦" antes do texto.
**B216 (reforma visual da ficha, falta ver no painel):** as dez janelas (Estatísticas, Equipamento, Roupas,
Inventário, Armazém, Bolsa de missão, Habilidades, Mascotes, Daimon, Missões ativas) têm **o mesmo tamanho**
(grade `minmax(420px, 1fr)`, 500 px de altura, rolagem interna) e ícones/textos maiores (células com
`aspect-ratio`, 8 por linha; corpo 6 por linha com o nome do slot). Ícones de traço próprios em
`painel.js` (`ICONES`, `[data-icone]`; a CSP só permite `self`). Tudo que edita abre **modal** pelo botão
da barra: **Estatísticas** só mostra (um campo de cada, com ícone) e "Editar" abre o modal com Dinheiro,
EXP/SP, Pontos livres, Nível, Cultivo, Atributos e Posição. **Slot vazio** (bolsa, armazém, missão, corpo) →
menu "Criar item" → modal com busca por nome/ID e **categoria** (`GET …/itens?busca&categoria`; texto vazio
com categoria); bolsa/armazém/missão criam uma pilha no slot (`item {id, quantidade, recipiente, slot}`), o
slot do corpo filtra `slot:N` e o item nasce na bolsa para arrastar; "Dar item" (barra do Inventário) é o
prêmio de missão de antes. **Editar item:** modal largo com a prévia da dica do jogo e a quantidade à esquerda
e os blocos com ícone (durabilidade e refino, requisitos com classes em chips, furos e pedras, efeitos) em duas
colunas; Remover no rodapé; item que não é equipamento abre estreito. **Habilidades:** "Dar habilidade" →
busca por nome/ID e **classe** (`GET /api/habilidades?busca&classe` — `cls` do stub: 0–11 as classes, 127
"sem classe (monstros e mascotes)", 255 "vida e especiais", 258; com classe, até 60; nomes por
`GET /api/classes?versao`, 8 no 1.2.x e 12 no 1.5.5) → modal do nível (ensinar); clicar numa aprendida abre o
mesmo modal (nível, remover). **Mascotes:** "Dar mascote" → busca nos ovos do realm por nome/ID, **tipo**
(montaria, combate, seguidor, invocação, planta, evolução) e **ambiente** (terrestre, aquático, voador)
(`GET /api/realms/{id}/mascotes?busca&tipo&ambiente`) → modal do mascote com "Dar mascote"
(`dar_mascote {ovo}`); clicar num da jaula abre o modal de edição, com as habilidades em lista (ícone, nível,
tirar) e busca para dar habilidade (grupo padrão 127); a API devolve ícone e nível máximo de cada habilidade.
**Daimon:** "Dar EXP" em modal. **Missões:** a janela lista **só as ativas**, com o nome pintado pelos códigos
`^RRGGBB` do `tasks.data` como o jogo (sem código, a cor do painel); "Editar" abre o modal com duas abas —
ativas e concluídas filtradas pelo mesmo texto (nome ou ID; Concluir/Cancelar/Esquecer em cada linha) e
"Dar missão" (busca e etapa). **Resultado de toda operação = toast** no canto (estilo SweetAlert, some sozinho
em 2,6 s no sucesso e 5 s no erro/aviso, barra de tempo, clique fecha; sem OK); modal só para confirmação
(remover, cancelar, libertar, desconectar e aplicar), o carregamento das contas e o aviso com "Consultar
agora". **Correções do relato do Tsuko (1.2.6):** voo 15732 sem ícone/descrição (tabela de item, spec 03) e
anel 310 sem descrição (bloco do molde no detalhe, spec 05 §7.11); a dica do voo segue o
`CECIvtrFlySword::GetNormalDesc` (`EC_IvtrFlySword.cpp:137-210`: nível, voo simples e acelerado `%+.2f`,
tempo ativo, classes, nível exigido, preço; a linha de aprimoramento `ITEMDESC_FLYSWORD_*` fica fora — índices
283/284 não conferidos no BR).
**B215 (fechamento da E6):** janela **Daimon** (`GET …/personagens/{pid}/daimon`, canal `daimon`): nome,
nível, EXP/para subir, atributos, pontos livres, gênios, refino, vigor e habilidades; "Dar EXP" (`daimon_exp`
1–2·10⁹) pela regra do jogo. Passam a valer **em jogo** (sem "Desconectar e aplicar"): editar item do
armazém, descer/remover habilidade no 1.2.6, esquecer missão (as duas versões) e cancelar missão no 1.2.6
(regras na spec 05 §7.11). Continuam só offline: remover e arrastar com equipamento/armazém fora dos pares
do jogo (B187/B191).
**B214:** a busca por texto acha os efeitos de habilidade do item pelo texto do cliente ("Mordaz") ou pelo
nome da habilidade ("Garra"): na primeira busca de cada realm o painel pede ao GS (`efeitos_para_item`,
`item_id` 0, sem filtro de família) o valor dos 86 ids do tipo 55 e guarda `{id: (texto, nome)}` em
memória; a filtragem pela peça continua na consulta final. Na lista: "✦ <texto> — habilidade <nome>". Antes (B206): busca de efeito pelo texto ou id (`GET /api/efeitos?busca=`,
2.061 efeitos descritos pelo `item_ext_prop`); campo **Furos** (0–4) que redimensiona a lista de pedras e
busca de pedra (`GET /api/realms/{id}/pedras?busca=`, canal `buscar_itens` com `categoria: "pedra"`) que
põe o id no primeiro furo vazio. Regras na spec 05 §7.11. Arrastar ao corpo também recusa
`reputacao` e `habilidade_repetida` (B202). Aparência (rosto, cabelo, corpo): **sem editor no painel**
(decisão de 2026-10-08) — fica com a troca de rosto do cliente, que o servidor grava desde o B199. Recusa `precisa_estar_offline` abre **"Desconectar e aplicar"**: o desconectar da E4, espera o GS declarar ausência (até 20 s) e repete a edição offline com operação nova. Ficha viva quando exatamente um GS declara presença; origem e limitações explícitas |
| Contas E4 | `/api/contas?busca&pagina&por_pagina` (1–48, padrão 12; `_`/`%` literais) com `total`, personagens, criação e último login; criação, senha, GM, gold e ban via GS, recuperáveis, roteados pelo realm selecionado com canal ou pelo primeiro que tenha; GM coordena caches/efeitos globais, cliente requer reentrada. **Gold** (B175; só dar desde B180): `POST …/contas/{id}/gold {delta ≥ 1}` em unidades do cash (100 = 1 gold), soma atômica; valor ≤ 0 recusado em todas as camadas (`valor_invalido`), depois `atualizar_cash` em todos os GS reenvia `PLAYER_CASH` (253) a quem está online. **Ban** (B175): `POST …/contas/{id}/ban {banida, motivo≤120}` grava `is_banned`/`ban_reason`, sem expiração (o link não lê `ban_expires_at`), recusa banir a própria conta (`proprio_administrador`); ao banir, desconecta em todos os realms. **Desconectar** (B175): `POST /api/contas/{id}/desconectar` manda `desconectar` a todos os GS com canal; cada um salva pelo caminho do logout e o link recebe `PlayerLogout` com `result` 2 (cliente volta ao login, `EC_GameSession.cpp:5420-5426`); idempotente, sem registro durável. Limitação: sessão parada na seleção de personagem não está em GS nenhum e não é desconectada (o próximo login de conta banida é recusado) |
| Capacidades | `realms.capacidades(versao, canal)` (B218: textos do que existe): configuração, protocolo, consulta viva, edição e templates (`implementado` com canal; `indisponivel` sem), rates, mapas, catálogo/textos/ícones (`implementado` no 1.5.5, `parcial` no 1.2.6 — textos do cliente 1.5.5) e "Editores de elements, tasks e npcgen" `indisponivel` (contrato em `docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md`); outras versões `nao_validado`. Rótulos: Disponível, Implementado, Parcial, Não validado, Indisponível (não há mais "Em breve") |
| Docker | Porta 8000; contexto raiz, ignore específico e estágio Rust do verificador |
| Manual e contratos | `docs/WEB_ADMIN_USER_GUIDE.md` (B218: manual do GM — páginas, em jogo × salvo × fora do jogo, o que nunca é editável); `docs/admin/ARQUITETURA_E_CONTRATOS.md`; extensões: `docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md` |
| Interface E9 | **B218 (falta ver no painel):** menu lateral, topo (menu, atualizar, tema) e popup de contas com os ícones de traço de `ICONES` (antes glifos/emoji); tema claro por `prefers-color-scheme` e botão de lua no topo (`data-tema` no `<html>`, lembrado no `localStorage` com `try/catch`; células de item e dica continuam escuras); nome do mapa pelo catálogo ("1 · Mundo") no painel do realm e nos cartões de template; janela Daimon escondida quando o GS responde `existe_na_versao: false` (1.2.6: o `elements.data` v7 não tem `GOBLIN_ESSENCE`, `GameDataManager::versao_tem_daimon`); ficha offline diz "a edição é gravada pelo GS, que recusa o login enquanto grava"; `pagina-futura`/`FUTURAS` e CSS sem uso removidos. A 375 px não há rolagem horizontal |

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
**Falta (B218, fim da reforma):** ver em jogo E4–E6 e E8 (roteiros do estado §3.3; a E7 foi
confirmada em 2026-10-10). Editores de `elements`/`tasks`/`npcgen` e de recursos do cliente: só o
contrato proposto em `docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md` (nenhum escritor de `.data` existe).
Textos e ícones do 1.2.6 vêm do cliente 1.5.5 BR, sem conferência no cliente 1.2.6.
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

Quadro: u32 big-endian de tamanho + JSON UTF-8: pedido 1–8192 bytes, resposta 1–65 536 (`LIMITE_RESPOSTA`, B198: as listas de missão com nomes passam de 8 KiB; `canal.py` usa o mesmo teto). Saudação v1 com desafio
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

Backend anterior substituído, incluindo escritas diretas e seed genérico de moldes. Toda escrita
passa pelo canal do GS (E4–E8). O `elements_decoder.py` antigo (nunca importado; fallbacks de outro
realm) e o `skills_canon.json` órfão **saíram no B218**, junto com os volumes do compose que só ele
usava; nomes e valores dos `.data` vêm do GS, textos e ícones de `data/textos`/`data/icones`. Migração: `scripts/2026_10_05_comandos_administrativos.sql`,
aplicada só em `test`; inicialização nova inclui tabela em `public` e `test`.

Sem histórico navegável, hierarquia adicional de administradores, correio em massa ou
launcher neste escopo. Logs técnicos sem credenciais; deduplicação é interna.
Regras/layouts do jogo continuam no GS e no WorldProtocol.

### Recursos do cliente (B235)

Página "Recursos do cliente" (grupo Geral): rascunhos, envio de arquivo, publicar, reverter, histórico e busca
na versão publicada do cliente. O painel só repassa ao `pw-recursos` (8101) — descrição em
`specs/07_RECURSOS_DO_CLIENTE.md` §6. `LimitarEntrada` aceita até 512 MiB só em
`PUT /api/recursos/{v}/rascunhos/{id}/arquivo`.

### Estúdio (B237, fase S1)

Item "Estúdio" no grupo do realm: escolher (ou criar) um rascunho da versão de cliente do realm; tabelas do
`elements.data` com filtro, registros com busca e páginas, ficha genérica por tipo de campo, salvar no rascunho,
clonar (id novo do espaço) e remover. Sem rascunho é somente leitura. Repasse em `/api/realms/{realm}/estudio/…`
(a versão sai de `realms.version`); motor e regras em `specs/07_RECURSOS_DO_CLIENTE.md` §6b.

## 2. Patcher (`tools/pw-patch-tool/`) — protótipo

CLI Rust de ~140 linhas (`docs/PW_PATCH_TOOL_GUIDE.md`):

| comando | faz |
| :--- | :--- |
| `scan <dir_cliente>` | catálogo SHA-256 dos arquivos |
| `create-patch <dir_v1> <dir_v2> <v1> <v2> [notas]` | pacote `.cup` + `patch_manifest.json` |
| `list-patches` | histórico |

**Estado real (B219):** `create-patch` é **simulado** — não compara as pastas e grava 14,5 MB, 8
arquivos e o SHA-256 de arquivo vazio (`main.rs:100-125`). O `.cup` oficial (pacote Angelica com
`v-<n>.inc` assinado por RSA com chave que não temos, `CNewSkin/CupReader/CupReader.cpp:206-330`,
`patchlib/EC_Signature.cpp:23-70`) **não** será o formato: decisão do Murillo (2026-10-12) de
atualizador próprio. Plano — cliente de referência por linha (`155br`, `126`), registro de alterações
no banco com objetos por SHA-256, manifestos assinados com Ed25519, launcher que regrava entradas de
`.pck` e repara — em `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` (fases F1–F7).

`planejado` (antes do B219): diferença **dentro** dos `.pck` (atenção: `models.pck`/`litmodels.pck`/`building.pck`
passam de 2 GB e o motor não os abre — `tools/pw-pck-extract/` lê), compressão zstd,
download retomável com HTTP Range, verificação por SHA-256 antes de aplicar, launcher
gráfico, e atualizar `serverlist.txt` (UTF-16LE com BOM) e o `.bat` com `logiccheck:0`.
