# Extensões do painel: editores futuros (E9, B218)

O escopo aprovado da reforma (memória §1, "Editores futuros") é **só preparar** a arquitetura para
`elements`, `npcgen`, `tasks`, modelos, animações e texturas. Nada daqui está implementado. Este
documento diz, para cada editor, o que já existe, o que falta, por onde a escrita passaria e o que
pode dar errado. Os contratos são **propostas** (estado `planejado`); quem implementar confere de
novo a evidência e atualiza a spec 06.

## 1. O que já existe e vale para todos

| peça | onde | o que dá |
| :--- | :--- | :--- |
| Leitura dos `.data` | `crates/pw-data-loader` (spec 03) | `elements` (v7/v156/v159), `tasks` (v55/v129), `npcgen`, `aipolicy`, `gshop`, `.sev`, `.hmap`; cada leitor recusa o arquivo que não fecha no último byte (spec 03 §1) |
| Carga | `GameDataManager::load_from_directory` — GS em `crates/pw-gs/src/main.rs:63`, link em `crates/pw-link/src/gateway.rs:138` | lido **uma vez na partida**; ligar mapa em execução reaproveita os dados já em memória (`crates/pw-gs/src/mapas.rs:271-274`) |
| Consultas pelo canal | `Consulta` em `crates/pw-gs/src/administracao.rs:34-120` | `buscar_itens` (:112), `buscar_mascotes` (:115), `efeitos_para_item` (:118), `detalhe_item` (:120), `buscar_missoes` (:105), `templates` (:50) — nome, ícone, pilha, categoria, efeitos e missões **do realm** |
| Catálogo por realm | `data/<realm>/catalogo/` (spec 03 §2, B102) | layout do `elements` e habilidades no lugar do embutido, sem recompilar |
| Textos e ícones do cliente | `data/textos/`, `data/icones/`; geradores `scripts/gerar_*.py`; tabelas em `web-admin/backend/painel/*.json` | só do cliente **1.5.5 BR**, usados também no 1.2.6 sem conferência no cliente dessa versão |
| Padrão de comando | `comandos_administrativos` (B167): `operacao_id`, deduplicação, resultado recuperável | toda escrita nova reaproveita |

**Não existe escritor de nenhum `.data`.** No `pw-data-loader` não há função que serialize
(a única escrita em arquivo é o `escrever_gshop` de teste, `manager.rs:1194`). O
`elements_decoder.py` antigo do painel nunca funcionou direito (relato do Murillo,
`specs/elements_155/README.md:254-256`) e saiu no B218. O `specs/elements_layouts/pw_elements_reader.py`
lê com o mesmo algoritmo do Rust, mas o painel não o usa: nomes e valores vêm do GS.

## 2. Caminho de escrita comum (proposto)

Regras que todos os editores de arquivo seguem — as mesmas da reforma (memória §3):

1. **Navegador → API → canal → GS.** O painel nunca escreve arquivo nem SQL de dados de jogo. Um
   só processo por realm escreve: o primeiro GS de `ADMIN_DAEMONS` (o mesmo que responde às buscas).
2. **Rascunho, nunca o arquivo vivo.** A edição vai para `data/<realm>/rascunhos/<arquivo>.<n>`;
   o `config/` só muda na publicação.
3. **Validação pelo carregador.** Publicar = serializar → reler com o leitor do `pw-data-loader`
   (fecha no último byte, contagem de tabelas, mesma versão no cabeçalho) → comparar ida e volta:
   o arquivo sem edições tem de sair **idêntico byte a byte** ao original. Sem isso, o escritor
   não está pronto. Onde o cliente é o juiz (elements, tasks), o mesmo teste roda contra o
   carregador medido do cliente (spec 03 §3.1/§3.2).
4. **Troca atômica com cópia.** O arquivo publicado entra em `config/` por renomeação, guardando o
   anterior em `data/<realm>/rascunhos/anterior/`. O estado da publicação diz o que falta:
   `aguarda_reinicio` (GS e link releem na partida) e, quando o cliente também lê o arquivo,
   `aguarda_distribuicao`.
5. **Sem recarga a quente nesta etapa.** Recarregar em execução exigiria trocar o `Arc` dos dados
   com o mundo andando, fora do `world.tick` (memória §3); fica como extensão separada.

Contrato de canal proposto (mesmo envelope dos comandos atuais, `operacao_id` obrigatório nas escritas):

| canal | pedido | resposta |
| :--- | :--- | :--- |
| `dados_listar` | `{arquivo, tabela?, busca?, pagina?}` | registros com os campos do layout (nome, tipo, valor) |
| `dados_rascunho` | `{arquivo, rascunho?, alteracoes: [{tabela, id, campo, valor}]}` | `rascunho` + recusas por campo (`tipo_invalido`, `fora_do_limite`, `id_inexistente`) |
| `dados_publicar` | `{arquivo, rascunho}` | `publicado` + `aguarda_reinicio`/`aguarda_distribuicao`, ou `nao_fecha` / `ida_e_volta_diferente` |

Rotas da API: `GET /api/realms/{id}/dados/{arquivo}`, `POST …/dados/{arquivo}/rascunho`,
`POST …/dados/{arquivo}/publicar` — CSRF, um realm explícito, limite de corpo de 8192 bytes por
lote de alterações.

**Bloqueio de infraestrutura:** os contêineres de realm e de mundo montam `config/` **só para
leitura** (`docker/docker-compose.yml:116`, `:148` e equivalentes). O processo escritor precisa de
um volume gravável para `rascunhos/` e da troca no `config/` — decisão de Docker a tomar quando o
primeiro editor for feito.

## 3. Cada editor

### 3.1 `elements.data` — itens, criaturas, configurações

- **Fonte de verdade:** `data/<realm>/config/elements.data`. O **cliente tem cópia própria** e a
  carrega na partida (`EC_Game.cpp:574`, `load_data("data\\elements.data")`); só registra o MD5 no
  log (`EC_Game.cpp:565-570`), sem conferir com o servidor. Editar só no servidor **dessincroniza
  em silêncio**: nome, ícone, preço e dica do cliente ficam os antigos, e um id novo não existe para ele.
- **Leitor:** `generic_elements.rs` com os layouts `specs/elements_layouts/v7.json`, `v156.json`,
  `v159.json` (spec 03 §3.1), inclusive os blocos de tag do v156/v159 e o laço do `TALK_PROC`.
- **Falta:** o escritor (inverso de `decode_record`, mais os blocos de tag e o `TALK_PROC`); o teste
  de ida e volta nos três layouts; a interface por tabela (o layout já dá nome e tipo de cada campo).
- **Contrato:** o comum do §2 com `arquivo: "elements"`, `tabela` = nome do layout
  (`WEAPON_ESSENCE`, `MONSTER_ESSENCE`…), `id` = campo `ID`.
- **Riscos:** dessincronia com o cliente; mudar uma tabela usada pelo GS (preço, requisito, drop)
  muda regra de jogo; os octetos dos itens já criados guardam valores de fábrica e **não** mudam
  com a edição; texto `wchar` com tamanho fixo (truncar, nunca estourar).

### 3.2 `tasks.data` — missões

- **Fonte de verdade:** `data/<realm>/config/tasks.data`. O **cliente também lê**
  (`EC_TaskInterface.cpp:165`, `LoadTasksFromPack("data\\tasks.data")`; MD5 só no log,
  `EC_Game.cpp:581-585`): o texto, a lista e as condições que o cliente mostra são os dele.
- **Leitor:** `tasks.rs` — um por versão (v55 do 1.2.6, v129 do 1.5.5), missões recursivas, cada
  uma terminando onde começa a seguinte (spec 03 §3.2).
- **Falta:** o escritor do `ATaskTempl::SaveBinary` das duas versões, com a tabela de deslocamentos
  refeita; ida e volta byte a byte; interface (a busca `buscar_missoes` já lista e mostra as etapas).
- **Contrato:** o comum do §2 com `arquivo: "tasks"`, `tabela` ausente, `id` = id da missão (as
  filhas pelo id próprio).
- **Riscos:** dessincronia com o cliente (missão que o servidor dá e o cliente não conhece);
  missões em curso no banco (`character_tasks`, listas binárias) apontando para etapas que mudaram;
  `dyn_tasks.data` separado.

### 3.3 `npcgen.data` — onde as criaturas e recursos nascem

- **Fonte de verdade:** um arquivo por pasta de mapa (`config/world/`, `config/aNN/`…). **Só o
  servidor lê**: o fonte do cliente 1.5.5 não abre `npcgen.data` (o único `npcgen` lá é um campo de
  configuração, `elementdataman.cpp:2073`). Editar não exige redistribuir nada.
- **Leitor:** `npcgen.rs` (`NPCGENFILEHEADER7`, áreas, geradores, recursos, controladores — spec 03 §3.3).
- **Falta:** escritor por versão do cabeçalho (v<7 e v≥7, geradores v10/v11); ida e volta; interface
  (lista de áreas por mapa com posição, criatura, quantidade e tempo de renascer).
- **Contrato:** o comum do §2 com `arquivo: "npcgen"`, `tabela` = `"mapa:<n>"`, `id` = índice da
  área/gerador no arquivo.
- **Riscos:** o mapa só relê na partida do GS (`mapas.rs:271-274`); controladores ligados pelo
  `aipolicy.data` e pelos gatilhos de missão (B127) dependem dos índices — inserir no meio desloca.

### 3.4 Modelos, animações e texturas

> **B219:** análise aprofundada — cliente de referência, formatos (`ASMD` 8, `ASKE` 6, `ASKI` 9,
> `STCK` 1, `.ecm` texto), ordem pacote × solto, registro de alterações e atualizador próprio — em
> `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`. Onde este item e aquele divergirem, vale aquele.

- **Fonte de verdade:** só o **cliente**, dentro dos `.pck` (`models.pck`, `litmodels.pck`,
  `gfx.pck`, `textures.pck`, `surfaces.pck`…). O servidor não lê nenhum deles; a colisão e a
  navegação do servidor vêm de `.hmap`, `movemap/`, `airmap/` e `collision.clt` (spec 03 §3.6–3.9).
- **Leitor:** `tools/pw-pck-extract` (README) lê os pacotes, inclusive os de mais de 2 GB que o motor
  não abre. O painel só usa o que foi tirado à mão: os atlas de ícones `IconList_*` do `surfaces.pck`
  1.5.5 em `data/icones/` (spec 06, B195).
- **Falta:** índice dos pacotes por versão de cliente (nome, tamanho, hash); escritor de `.pck`;
  visualizador (fora do navegador sem dependência: a CSP só permite `self`); a distribuição, que é o
  **patcher/launcher** (Contexto H; protótipo em `tools/pw-patch-tool`, spec 06 §2).
- **Contrato proposto:** leitura — `GET /api/clientes/{versao}/pacotes` e
  `GET /api/clientes/{versao}/pacotes/{pck}?busca=` servidos de um índice gerado por
  `pw-pck-extract` em `data/clientes/<versao>/indice.json` (sem canal: o GS não participa); escrita —
  **não pelo painel**: o arquivo novo vira um pacote do patcher (`create-patch`), que é quem
  distribui e versiona.
- **Riscos:** pacotes de vários GB (nada de carregar no navegador nem na API); ordem e compressão do
  índice do `.pck`; cliente sem patch quebra ou mostra modelo errado; licença do conteúdo original.

## 4. Fora deste documento

- Recarga a quente dos dados (troca dos dados em memória com o mundo andando).
- `aipolicy.data`, `gshop*.data`, `.sev`, `gs.conf`: mesmos princípios do §2 quando forem pedidos.
- "Aplicar aos existentes" dos Templates (fora por decisão de 2026-10-11, memória §4, E8).
- Versionamento de toda edição e o atualizador: `RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` (B219) — os
  editores de `.data` daqui gravam no mesmo registro de alterações; `elements`/`tasks` têm o mesmo MD5 no
  servidor e no cliente, então toda edição deles é também uma atualização do cliente.
