# Estado atual e como retomar

> Nota de passagem entre sessões: onde o trabalho está, o que já é fato verificado e qual é
> a fila. **Atualizar ao fim de cada bloco de trabalho** — e manter curto: o relato
> detalhado de cada sessão (sintoma, causa com referência ao fonte, correção, provas) vai
> para o `docs/HISTORICO_DE_SESSOES.md`, com número de item.
>
> **Última atualização: 2026-09-23**, B93 — **o 1.5.5 chegou a jogável no básico** e tudo
> foi juntado na `main` (inclusive a frente 1.2.6 que estava na worktree `../pw-126`). A
> frente ativa passa a ser **levar ao 1.2.6 o que o 1.5.5 já resolveu** (§5D). Suíte com o
> banco na `main`: **654 testes, 0 falhas** — medir com `--test-threads=2` (skill `pw-testar-e-publicar`).
>
> **Publicado no realm 155** em 2026-09-22 (B79–B86). B87–B88 (mapa de água) e o realm 126
> **não** foram publicados.
>
> O Claude (`.claude/`, `CLAUDE.md`) e o Codex (`AGENTS.md`, `.codex/`) seguem as mesmas
> diretrizes, com as skills em `.claude/skills/`.
>
> **Como o sistema é** (arquitetura, formatos, protocolo, regras de jogo) está nas specs —
> `specs/README.md` é o índice. Este documento diz **onde o trabalho está**.
>
> Referências a itens: **A*n*** e **B*n*** são as duas séries numeradas do histórico (ver o
> cabeçalho dele). Um comentário de código que diga "item 14 do `ESTADO_E_RETOMADA.md`"
> aponta para o histórico.

---

## 0. Em uma tela

**Marco de 2026-09-23 (B93): o 1.5.5 está jogável no básico.** Tudo está na `main` (as
branches `multi-versions` e `versao-126` foram juntadas). Ordem combinada com o Murillo:

1. ~~1.5.5 jogável no básico~~ — **atingido**. O que falta dele está em §5A/§5B e continua na
   fila, mas deixou de bloquear.
2. **1.2.6 com o que o 1.5.5 já resolveu** ← **estamos aqui** (painel em §5D).
3. Depois: banco de dados (Contexto E), painel `pw-admin` (G), atualizador/launcher (H).

**O que o 1.5.5 faz hoje** (realm `realm_155`, porta 29004, cliente BR v156, `elements.data`
v156, `tasks.data` 129, build 2569):

- **Conta e personagem:** login, criação com os atributos, a arma e a habilidade iniciais da
  classe, nascimento no mapa 161 num servidor de mundo próprio, troca de personagem, saída.
- **Mundo:** NPCs, monstros, recursos e outros jogadores entram e saem por distância; altura
  pelo `.hmap`, água pelo `watermap/`; os três `.data` do realm lidos inteiros, fechando no
  último byte.
- **Movimento:** andar, voar (item de voo com máscara de classes), meditar, gestos, montaria
  (sessão de invocar/recolher, não monta na água funda e cai dela), teleporte por
  transportadora com os pontos descobertos (`world_targets.sev`), modo roupa que persiste.
- **Combate:** golpe normal com dano adiado como no original, monstros com dados do
  `MONSTER_ESSENCE` que perseguem, voltam, passeiam e atacam sozinhos quando agressivos;
  habilidades com conjuração, execução, recarga, chi (`apcost`/`apgain`) e conjurar andando;
  cura e PvP; morte e renascimento no distrito; durabilidade das peças.
- **Progressão:** experiência, nível, cultivo pela missão, pontos de atributo, treinador,
  Daimon (ficha e experiência).
- **Missões:** do `tasks.data` com listas binárias, automáticas por zona, dinâmicas,
  prêmios (itens, dinheiro, experiência, cultivo, teto de chi), monstro invocado.
- **Itens e economia:** loja de NPC com o preço do arquivo, drop e coleta (inclusive mina que
  acorda monstro), poções no tempo com recarga por família, amuleto e hierograma automáticos,
  flechas, descarte com destrave de slot, reparo.
- **Social:** fala, grupo.

**O que falta no 1.5.5** (§5A): serviços de refinar e incrustar, os ~300 efeitos de
habilidade sem porte, intérprete do `aipolicy.data`, trava de PvP, mascote de combate, resto
do Daimon, armazém, troca de mapa entre contêineres, fôlego debaixo d'água.

**O que o 1.2.6 faz hoje** (realm `realm_126`, porta 29000): loga, cria personagem e entra no
mundo; os layouts de entrada, combate, experiência e itens foram conferidos com captura
(B74-126, B89, B90, B93). As **regras de jogo são as mesmas** — ficam no `pw-gs`, e o que muda
por versão fica no `WorldProtocol` de `crates/pw-protocol/src/versions/v126/`. O que impede o
1.2.6 de ter tudo o que o 1.5.5 tem é **dado e layout**, não regra: o `elements.data` v7 é
lido pelo leitor tipado antigo (sem os campos que as regras novas usam), o `tasks.data` v55
só tem o cabeçalho lido, e os comandos acrescentados depois do B76 não foram conferidos para o
1.2.6. **Nada do 1.2.6 foi visto em jogo depois do B63.** Painel em §5D.

**Um realm por versão (B55):** `realm_155` = cliente BR, contêineres `pw-realm-155` /
`pw-world-155`, dados em `data/realm_155/config`; `realm_126` = `pw-realm-126` /
`pw-world-126`, dados em `data/realm_126/config`.

**Detalhe de cada entrega:** índice do §8 → `docs/HISTORICO_DE_SESSOES.md` (só o item, com
`grep`).

---
## 1. Ambiente

### 1.1 Serviços (`docker/docker-compose.yml`)

Nesta máquina (Windows, Docker Desktop) `docker` e `cargo` rodam direto, sem SSH.
Credenciais na memória `pw_universal_infra_access`.

| realm | versão | porta do cliente | servidor de mundo (mapas) | dados | situação |
| :--- | :--- | ---: | :--- | :--- | :--- |
| `realm_155` | 1.5.5 | **29004** | `pw-world-155` (mapas 1 e 161) | `data/realm_155/config` | **o realm de teste** — cliente BR; até 2026-09-17 se chamava `realm_155BR` (B55) |
| `realm_126` | 1.2.6 | 29000 | `pw-world-126` (1) | `data/realm_126` | loga e entra no mundo; **frente atual** (§5D) |
| `realm_153` | 1.5.3 | 29001 | `pw-world-153` (1) | — | abandonado |
| `realm_148` | 1.4.8 | 29002 | `pw-world-148` (1) | — | nunca foi alvo |

Mais `pw-postgres` (5432), `pw-dragonfly` (6379), `pw-auth` e `pw-admin-api` (8000). A
porta do barramento `pw-link`↔`pw-gs` (29100) **nunca** é publicada — não tem autenticação,
e `pw-bus/tests/topologia_do_compose.rs` cobra isso.

Um servidor de mundo por realm com todos os mapas dele (`WORLD_TAGS: "1,161"`), dados
carregados uma vez; o roteador entrega cada jogador ao mapa gravado (spec 02 §2.2).

### 1.2 O que há em `data/realm_155/config`

Base: o pacote de servidor `F:\PW\1.5.5\home155\gamed\config` (76 pastas de mapa,
`npcgen.data`, `aipolicy.data`, `.sev`, `gs.conf`, `ptemplate.conf`, `global_api.lua` com
`--102`). Por cima, os 11 `.data` do cliente BR (`elements.data` v156, `tasks.data` 129,
`gshop*.data` …). Mapas de altura `.hmap` por mapa. (B13, B42c, B48.)

Não há mais realm com os `.data` do cliente EN (v159): a pasta foi apagada no B55. O layout
v159 continua no catálogo do leitor, sem arquivo para testar.

### 1.3 Clientes

| cliente | onde | build | serve para |
| :--- | :--- | :--- | :--- |
| **1.5.5 BR** | `F:\PW\1.5.5\1.5.5 BR\` | 2569, elements v156 | **os testes** → `realm_155`, 29004 |
| 1.5.5 EN | `F:\PW\1.5.5\1.5.5.EN\` (e `F:\PW\1.5.5\bin`) | 2575, elements v159 | nenhum desde o B55 (o realm EN foi removido) |
| 1.5.5 BR novo | `E:\0_GAMES\Perfect World` | **2591**, elements **v181**, tasks 135 | nenhum ainda: não há `v181` no catálogo nem pacote de servidor correspondente (B11) |
| 1.2.6 | `F:\Games\perfectworld_126\element` | — | → 29000 |

Pré-requisitos do cliente 1.5.5 que **não** são do servidor, e que custaram sessões:

- Iniciar com `elementclient.exe game:cpw console=1 logiccheck:0` — sem o `logiccheck:0` o
  cliente crasha ao montar a interface, porque o Arc SDK está desligado (B9e). O
  `elementclient.bat` do BR e do EN já tem isso.
- `models.pck`/`litmodels.pck` passam de 2 GB e o motor não os abre (`int nOffset` de 32
  bits em `AFilePackGame::InnerOpen`). Os clientes BR e EN têm o conteúdo **extraído como
  arquivos soltos** por `tools/pw-pck-extract/`. **Não** tentar reconstruir os `.pck` (B6a,
  B9f).
- `element/userdata/server/serverlist.txt` em UTF-16LE com BOM,
  `Nome<TAB>PORTA:HOST<TAB>id` (B10).

### 1.4 Material de referência em disco

| onde | o quê | como usar |
| :--- | :--- | :--- |
| `F:\PW\1.5.5\EvolvedPWClient` | fonte do cliente 1.5.5 (build 2457) | mapa, não verdade: o binário instalado é mais novo (memória `pw_client_155_fonte_vs_binario`) |
| `F:\PW\1.5.5\EvolvedPWServer` | fonte do servidor 1.5.5 | as regras de jogo (`cgame/gs/`) e os carregadores de dados; para rodar o `pw-rpcgen` precisa das junções da memória `pw_ctx_a_155_funcional` |
| `F:\PW\1.7.2\172Source` | fonte do servidor 1.7.2 | nomes de campos que os `.data` do 1.5.5 têm e o fonte 1.5.5 não (`aipolicy.data`, sistema de Lar do `tasks.data`) (B27f, B45) |
| `F:\PW\1.5.5\pwserver_155v156` | servidor 1.5.5 compilado (Linux), mesma build v156 dos nossos dados | `gamedbd/clsconfig` = moldes de classe (B47); candidato a gabarito rodando numa VM 32-bit |
| `F:\PW\1.5.5\home155` | outro pacote de servidor 1.5.5 (2023) | base do `realm_155`; tem outro `clsconfig` |
| `D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL\configs\CFG\` | 39 `.cfg` de `elements.data` (1.5.2 v123 a 1.5.7 v206) | origem de `specs/elements_155/PW_1.5.5_v1*.cfg` |
| VM `192.168.1.200` | servidor **1.2.6 original** completo | gabarito de mecânica e fluxo (não de bytes do 1.5.5). O `ping` responde; o SSH pede senha — falta instalar `_sync/ssh/win_key.pub` no `authorized_keys` dela (B44c) |
| `_sync/capturas/` | capturas da VM 1.2.6 (elo `gs`→`glinkd` em claro) | `cargo run -p pw-pcapdiff -- <pcap> --interno --subcomando N` |
| `source_server_153`, `source_client_153` | fontes 1.5.3 | histórico; o IR `specs/protocol/gamedata_153.json` saiu deles |

---

## 2. Como rodar, testar e publicar

```bash
# A suíte SÓ testa de verdade com esta variável. Sem ela, os testes de integração
# passam sem verificar nada (memória pw_testes_precisam_do_banco).
TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
  cargo test --workspace

# Publicar no realm de teste (um link e um servidor de mundo com os mapas 1 e 161)
cd docker && docker compose build pw-world-155 pw-realm-155 \
  && docker compose up -d --remove-orphans pw-world-155 pw-realm-155

# Logs
docker logs -f pw-realm-155        # login, entrada no mundo, o que o link trata
docker logs -f pw-world-155        # os mapas 1 e 161
```

**Referência da suíte, medida em 2026-09-20 (B69) com o banco:** **599 testes**, com o
arquivo de tempo passando 64/64 sozinho (`cargo test --workspace --no-fail-fast`). Os testes de tempo do
`pw-gs/tests/subcomandos_no_mundo.rs` podem falhar sob carga (já aconteceu com um build do
Docker rodando junto); o arquivo sozinho passa 58/58
(`cargo test -p pw-gs --test subcomandos_no_mundo -- --test-threads=4`).
Qualquer outra falha é nova.

A suíte cria realms `t_*`, contas e personagens de teste no banco local. **Desde o B64 (2026-09-18),
o `pw-storage` isola todas as conexões que rodam com `TEST_DATABASE_URL` no schema `test`
(`options=-csearch_path=test,public`), mantendo o schema `public` de produção 100% limpo e intocado.**
O schema `public` contém apenas as contas padrão (`admin`, `testuser`), os 4 realms base e personagens
reais. O script de inicialização do schema de teste (`specs/03_TEST_SCHEMA_POSTGRES.sql`) é montado
automaticamente no contêiner `pw-postgres`. Cada execução de teste limpa sobras antigas no schema `test`
através de `crates/pw-storage/tests/sql/limpar_sobras_de_teste.sql`.

**Scripts de dados do realm** (`scripts/`, aplicados à mão no banco): `asas_e_slot_de_municao_155.sql`,
`corrige_skills_e_armas_155.sql`, `2026_09_08_last_login_at.sql`,
`2026_09_09_atributos_iniciais_por_classe.sql`, `2026_09_11_template_de_classe_155.sql`,
`2026_09_12_nascimento_no_mapa_161_155.sql`, `2026_09_14_flechas_do_arqueiro_155.sql`,
`2026_09_17_realm_155_passa_a_ser_o_br.sql` (B55, aplicado uma vez — não reaplicar),
`2026_09_18_octetos_do_equipamento_do_eaa.sql` (B60: o equipamento que já estava sem bloco de
dados, gerado pelo `cargo run -p pw-gs --example gerar_octetos`),
`2026_09_18_durabilidade_na_escala_interna.sql` (B61, aplicado uma vez — não reaplicar:
multiplicaria de novo),
`2026_09_20_cultivo_do_eaa.sql` (B67, aplicado uma vez) e
`2026_09_20_pontos_de_teleporte.sql` (B68: a coluna `characters.waypoints`; também nas specs
01 e 03 para bancos novos), as colunas `ap`/`max_ap` do chi entraram no mesmo script (B69); `2026_09_20_cultivo_do_eaa.sql` e `2026_09_20_gloria_de_shalim_do_eaa.sql` acertam o personagem de teste,
`2026_09_14_listas_de_missao.sql` (tabela `character_task_lists`, também no
`02_MIGRACAO…sql` para bancos novos). Conferido em 2026-09-13: os 12 moldes do
`realm_155` estão com `spawn_world_id = 161`.

**Personagens de teste no `realm_155` hoje:** **POTATO** (id 4515, Bárbaro, nível 1, mundo
1) e **eaa** (id 5491, Arqueiro, mapa 161, com as flechas do script de 2026-09-14). Os sacerdotes `HEal` (40) e `testesacer` (42)
das sessões anteriores não existem mais no banco. Para testar compras:
`UPDATE characters SET money = 500000 WHERE id = <id>;`. Personagem criado antes do B48
continua no mundo 1; para testar o nascimento no 161, criar um novo.

### Instrumentos

| instrumento | o que mostra | cuidado |
| :--- | :--- | :--- |
| overlay do cliente: `##debug` no chat → `Shift` + tecla à esquerda do "1" → `d_rtdebug 1` | todo comando S2C **recusado** (id desconhecido ou tamanho diferente do `sizeof`) | **não** lista comando aceito — silêncio é bom sinal (B42f) |
| tooltip do item | vermelho = `CanUseEquipment` recusou; as linhas vêm do bloco de dados do `OWN_ITEM_INFO` | B38, B42a |
| ficha do personagem | `mv.run_speed` etc. como o cliente recebeu | B42g |
| `element/logs/EC.log` e `AF.log` | erros de carga, versão, decodificação | |
| `tools/pw-crash-re/` | minidump → cadeia de chamadas validada contra o `.exe`; desmontagem por VA | resolveu o crash de render (B13–B14) |
| `pw-pcapdiff --subcomando N` | corpo de um subcomando numa captura | B44c |
| `specs/clsconfig_155/ler_clsconfig.py` | moldes de classe do `gamedbd` original | B47 |

---

## 3. O que funciona

### 3.1 Confirmado em jogo pelo Murillo

**Login e personagem**
- Login no 1.5.5: `edition` e `GAME_VERSION` (`0x00010505`) batem com o binário (B4, B5).
- Entrada no mundo sem crash (o `TASK_DATA` de 5 blocos, B14–B15), troca de personagem e
  saída (B19–B20).
- Criação: atributos por classe do `ptemplate.conf`, arma e habilidade inicial certas, raça
  pela classe, vida cheia (B41e, B43, B44 #3–4).

**Mundo**
- NPCs, monstros e **recursos** (5.125 no mundo 1) entram e saem por distância, com a grade
  espacial do `pw-gs` (B39, B41c).
- Outros jogadores: modelo 3D, equipamento visível, sexo certo na reentrada, streaming por
  distância com `PLAYER_ENTER_SLICE` (B34, B41b, B42b).
- Movimento, meditar e gestos sincronizados entre jogadores (B24, B36i).
- Alturas pelo `.hmap`: teleporte de GM (Ctrl+clique) na superfície, monstros e recursos
  nascem no chão, Guia não flutua, recursos espalhados pela área (B42c–e, B48a).
- Nascimento no mapa 161 e perseguição dos monstros assentada no chão (B48, teste de
  2026-09-14).
- Velocidades, cadência, alcance e regeneração da classe vêm do `CHARRACTER_CLASS_CONFIG`
  do `elements.data`, não do `ptemplate.conf` — 4,9 m/s para o Bárbaro, igual à VM 1.2.6
  (B44b2, B48a).

**Combate e habilidades**
- Monstros com nível, vida e ataque do `MONSTER_ESSENCE`; fórmulas de dano portadas de
  `actobject.cpp` (B28, B29).
- Habilidades conjuram, animam e fecham a barra; cura e dano entre jogadores (B35–B37).
- Nível da habilidade vem do `character_skills` (B41d); treinador sobe habilidade (B42h).
- Voo pelas asas; botão armadura/roupa; equipamento sem ficar vermelho — arma, armadura e
  acessório com o bloco de dados do original (B37, B38, B41a, B42a).

**Economia**
- Loja de NPC cobra `max(shop_price, price)` do `elements.data`; item sem preço não é
  vendido; durabilidade do arquivo (B42h).

### 3.2 Implementado e testado na suíte, sem confirmação em jogo registrada

- Grupo com estado (convite, aceite, recusa, saída) (A45, B21e).
- Fala entre jogadores — canal global do `pw-link`, sem raio (B21d).
- Resposta ao medidor de latência `CALC_NETWORK_DELAY` (B42i).
- Autosave a cada 60 s de nível, experiência, SP, vida, mana, moedas, mundo e posição — o
  `UPDATE` falhava em silêncio até o B36f.

### 3.3 Publicado ou feito, sem confirmação em jogo registrada

Os itens abaixo passaram na suíte com o banco; o Murillo declarou o 1.5.5 jogável em
2026-09-23 sem apontar defeito neles, mas não há relato item a item. Detalhe e roteiro de
cada um no histórico.

| item | o que olhar | publicado? |
| :--- | :--- | :--- |
| B69–B70 | chi: +5 por golpe, +15/s meditando, teto 99 sem aviso repetido | sim |
| B71 | poção sem tela de cultivo; recarga por família (vida, mana, antídoto) | sim |
| B72 | combate longo sem pausa no minuto do autosave | sim |
| B73–B74 | Flecha Glacial −25 chi, Barreira de Asa −45 e escudo de 20 s; amuleto automático | sim |
| B75–B76 | Daimon ganha 1/10 da experiência; Flor de Safira solta o Guardião; monstro agressivo | sim |
| B77–B80 | modo de combate, Atq. Mágico na ficha, montaria (canalizar, montar, desmontar), quem chega vê montado | sim |
| B82–B86 | conjurar andando (2571/2579 do Tormentador), modo roupa persiste e o dono se vê de roupa, descarte de item | sim |
| B87–B88 | montaria recusada na água funda (erro 81) e derrubada acima de 1 m | **não** |

Onde olhar: `docker logs --since 10m pw-world-155 2>&1 | grep -iE "montou|desmontou|água|descartou|viajou"`.

---
## 4. Dados de referência

| arquivo | leitor | estado | usado pelo mundo? |
| :--- | :--- | :--- | :--- |
| `elements.data` | `pw-data-loader/src/generic_elements.rs` (+ `specs/elements_layouts/pw_elements_reader.py`) | **231/231** tabelas no v156 do `realm_155` (o v159 do EN fechou 234/234 antes de sair, B55); fecha no último byte, sem override (B46) | armas, armaduras, acessórios, monstros (com drop), NPCs e seus serviços de missão e habilidade, classes, poções, preços, pilhas, curva de exp, ajuste por nível, perda na morte (spec 03 §3.1). coleta (`MINE_ESSENCE`, B51), munição. **Não ligados:** `WEAPON_SUB_TYPE`, `NPC_SELL_SERVICE`, `NPC_TRANSMIT_SERVICE` |
| `tasks.data` | `pw-data-loader/src/tasks.rs` | **14.885/14.885** missões de topo (`realm_155`), fecha pelos deslocamentos do cabeçalho (B45) | **sim** — motor de missões (`pw-gs/src/missoes.rs`, B50) |
| habilidades do servidor | `specs/habilidades_155/habilidades.json` (`habilidades.rs`) | 3.316 stubs do `cskill` | recarga, conjuração, custo de aprender (B50) |
| `npcgen.data` | `pw-data-loader/src/npcgen.rs` | v5 a v11, fechando no último byte (B51); tipo de área, `fOffsetTrn`, extensão de recurso, sem tetos inventados (commit `931b39d`); `a01..a99` (B48) | sim; controladores (`id_ctrl`) tratados como ativos, sem modelar gatilho de evento (B17a) |
| `aipolicy.data` | `pw-data-loader/src/aipolicy.rs` | lido, com o fonte 1.7.2 como autoridade (B27f) | **não** — não há intérprete; o `ai.rs` porta só perseguição, volta e passeio |
| `ptemplate.conf` | `ptemplate.rs` (GBK, não UTF-8) | lido | quase nada: atributos, vida/mana e velocidades dele **não** chegam ao jogador no original (B51) |
| `gs.conf` → `specs/mapas/terreno_155.json` | `specs/mapas/gerar_terreno_155.py`, `terreno.rs` | 79 mapas, pela `tag` (B48c2) | sim, só o mapa que aquele servidor serve (88 MB no mundo 1) |
| `region.sev` / `precinct.sev` | `GameDataManager`, `precinct.rs` | carimbos por mundo; distritos do `precinct.sev` lidos inteiros | `INST_DATA_CHECKOUT`; renascer na cidade (B50) |
| `gshop.data` / `gshop1.data` | só o carimbo | — | `edition` do `Challenge` |
| `gamedbd/clsconfig` | `specs/clsconfig_155/ler_clsconfig.py` | posição de nascimento e vida/mana por classe lidas (B47) | via SQL no `class_templates`. Atributos 5/5/5/5 lidos de `GRoleStatus.property` (B51). **Não decodificados:** `config_data`, inventário, equipamento, habilidades |

---

## 5. O que falta


Conferido contra o código em 2026-09-13 — cada linha diz onde está a evidência.

### 5A. Jogabilidade do 1.5.5 — o que ainda falta

O pedido do Murillo no B44: combate básico inteiro, experiência, alma, moedas, animações,
habilidades com conjuração e recarga certas, mapa inicial e missões iniciais.

1. **Missões — o que o motor ainda recusa ou ignora** (spec 05 §10): casamento, PQ, prêmio
   por escala, invocação, falha por morte, sucesso/falha compartilhados pela equipe, abate
   contado para a equipe, item de missão pelo NPC (serviço 8), coleta com missão
   (`task_in/out`). Facção recusa sempre (não há sistema de facção).
2. **Munição:** sem flecha o golpe não é recusado e o bônus de dano da flecha não entra;
   a compra na loja não confere a lista de venda do NPC. **Itens:** sem serviços de refinar,
   fazer furo e incrustar; addons de habilidade/conjunto sem porte (B53).
3. **Coleta:** sem recarga de 500 ms, sem interrupção por dano, exp/SP sem ajuste de nível.
4. **Ataque normal sem animação, e o monstro atacado não reage** (B44 #6). A investigar —
   não medido.
5. **Troca de mapa só dentro do processo** (B51): mapa de outro contêiner exigiria o link
   reencaminhar a sessão. **Teleporte por NPC** (`NPC_TRANSMIT_SERVICE`, destinos por
   waypoint) e GM para outro mapa `falta`. O grupo se desfaz na troca.
6. **Habilidades:** dano (1.123), efeitos no alvo (2.304 roteiros) e em si (266) pelos stubs,
   37 efeitos portados (B53); os ~300 outros (formas, invocação, escudos, recargas) sem
   porte; imunidades de monstro; talentos. O **Portal da Cidade** (167) não tem efeito (B17c).
7. **Guia do jogo** do personagem novo: a barra de atalhos agora é gravada pelo próprio
    cliente (B51); o `config_data` do molde no `clsconfig` (barra pré-preenchida) segue não
    decodificado (B44b15, B47d).
8. **Sem trava de PvP:** qualquer jogador machuca qualquer outro, em qualquer lugar
    (`bus_server.rs`, comentário em `pvp`). O original exige duelo, guerra ou mapa de PK
    (B35d).
9. **Mascote de combate:** a montaria já monta, com a sessão inteira (B78/B79), e quem chega
    depois a vê (B80). Falta invocar a criatura de combate. Do estado estendido, os bits que
    ainda vão zerados são os que não temos dado para preencher — emote, efeitos visíveis,
    facção, barraca, cônjuge, título, VIP e os outros do `state2` (spec 04).
10. **Daimon (B75/B76, parcial):** tem ficha e ganha experiência, e só. Faltam o equipamento e
    as habilidades dele, o vigor, as pílulas de experiência, a decomposição, o refino,
    distribuir pontos de atributo e de gênio, e o bônus sorteado de 10 em 10 níveis. **O ganho
    truncar para zero com o Daimon muito abaixo do dono é do original** (B76), não é defeito.
11. **IA de monstro (B76, parcial):** o agressivo pega quem chega perto, mas as estratégias de
    ódio do `aipolicy.data` (facção, nível, invisibilidade, probabilidade) seguem sem
    intérprete.

### 5B. Fidelidade — números e sinais que ainda não são os do original

- Valor fixo no `bus_server.rs`: reparar custa **150**. A conjuração só usa os 1000 ms fixos
  quando a habilidade não tem `State1` na tabela do servidor.
- `crc_e` (carimbo de equipamento) vai zero: o cliente repede o equipamento a cada
  reaparição (B42k).
- `weapon_level` fixo em 1 e `attack_speed` da arma zerada no bloco do item; o original tira
  o segundo do `WEAPON_SUB_TYPE` (B41h).
- Bits do `attack_flag` desconhecidos: o crítico é calculado e não sinalizado (A-"Na ordem").
- `PLAYER_DIED` (27) não é mandado aos outros jogadores.
- Duplicatas medidas no B56: a carga de inventário e habilidades (`OWN_IVTR_DATA`,
  `OWN_ITEM_INFO`, `SKILL_DATA`) vai duas vezes por login — o link manda no `EnterWorld` e o
  mundo de novo no `GET_ALL_DATA` (39); e `SELF_INFO_00` + `GET_OWN_MONEY` vão duas vezes
  por abate. Falta conferir no original quem manda o quê e tirar a cópia.
- `QUERY_NPC_INFO_1` (68) responde `NPC_INFO_00`; o original responde `NPC_INFO_LIST` com a
  ficha do NPC (`gnpc_dispatcher::query_info_1`, `npc.cpp:330-338`) (B56).
- `dir` zerado no streaming de NPC e jogador — a grade guarda posição, não direção (B39e).
- Voo sem custo de mana e sem teto; `GP_STATE_FLY` fora do `state` dos pacotes de visão;
  `modo_roupa` e `voando` não persistem (B40c).
- A cura usa o ataque mágico no lugar de `GetMagicdamage`; o Tiro Certeiro (234) assume
  carga cheia (B40c).
- Monstro atravessa obstáculos: sem mapa de movimento (B48b).
- `class_templates` tem colunas de atributo que o código ignora (quem manda é o
  `ptemplate.conf`) — decidir quando o painel for editar moldes (B43h).
- Senha de segurança (`CHECK_SECURITY_PASSWD`): qualquer uma passa — não há senha no banco.

### 5C. Arquitetura (Fase 2 do `PLANO_ARQUITETURA_E_EXECUCAO.md`)

Critério de aceite: *o `gateway.rs` deixa de existir e nenhum gameplay fica no `pw-link`*.
**Não atingido.**

- **No `pw-gs` hoje** (`BusServer::tratar_subcomando`): movimento, saída, alvo, ataque,
  reviver, itens (9, 11–13, 16–18), postura e gestos, serviços de NPC, `SEVNPC_HELLO`,
  `TASK_NOTIFY`, senha de segurança, usar item, grupo, roupa, `GOTO`, habilidades, zona
  segura, as consultas (21, 39, 67, 68, 110), equipamento de outro jogador e latência — e
  todo o streaming de visibilidade.
- **Ainda no `gateway.rs`** (1.545 linhas): o fluxo de login e personagens, a carga
  inicial da entrada no mundo, fala, lista de amigos (sempre vazia), UI config / help
  states / custom data, e os subcomandos 92 (duelo, que só responde "preparar" sem regra),
  118 (preços do Mall) e 178 (waypoints).
- Nenhum subcomando é tratado nos dois lados desde o B49; o teste
  `os_comandos_ja_migrados_nao_sobraram_no_gateway` lê o `match` do mundo e cobra isso.
- Sem tratamento: `OPEN_BOOTH` (76, barraca pessoal), `MALL_SHOPPING` (106, removido de
  propósito no A51 — precisa de saldo, preço e slot livre), dividir pilha
  (`MOVE_IVTR_ITEM` ignora `amount`, A38), pedido de amizade (feature inexistente, B20d).
- Dívidas técnicas de protocolo, da série A: `nonce` do `Challenge` com os 8 primeiros bytes
  zerados (lá vão `Attr` e `newbie_time`, e por eles os rates do realm, A4); `codec.rs`
  ainda com duas implementações de layout (`adapter.rs` e `encode`, A24–A25); `octets.rs`
  duplicando o `pw-wire` (A-seção 1); cinco opcodes sem correspondência no IR (A21).

### 5D. 1.2.6 — paridade com o 1.5.5 (a frente atual)

Objetivo: o cliente 1.2.6 (realm `realm_126`, porta 29000) fazer o que o 1.5.5 já faz (§0).
**Princípio:** a regra de jogo é uma só, no `pw-gs`; o 1.2.6 difere em **layout** (o
`WorldProtocol` de `crates/pw-protocol/src/versions/v126/`, que compõe o padrão e sobrescreve
só o que muda) e em **dado** (versões dos `.data` do realm). Nada de `if versao == …` no mundo.

Evidência do 1.2.6: fontes 1.5.3 (o protocolo base), capturas da VM 1.2.6 em
`docs/evidencias/126/` (lidas com `cargo run -p pw-pcapdiff -- <pcap> --interno --subcomando N`),
binário do cliente 1.2.6 (validador de ids: ids > 260 são recusados, VA 0x584618) e
`docs/INVENTARIO_PROTOCOLO_126.md` (98 S2C e 46 C2S do `pw-gs`, com o que já foi conferido).

| área | estado no 1.2.6 | o que falta | onde |
| :--- | :--- | :--- | :--- |
| login, criação, entrada | **testado** (B63, B74-126); moldes das 6 classes no banco | ver em jogo de novo depois do merge | `docs/ENTRADA_126.md` |
| combate (layouts 24/26/33/83/84/144) | **testado** byte a byte com captura (B89) | ver em jogo | `docs/COMBATE_126.md` |
| experiência e itens (31/36/46/72/99/156/158) | **testado** (B90, B93) | preço real da loja no 1.2.6 (v7) e ver em jogo | `docs/ITENS_EXPERIENCIA_126.md` |
| `OWN_EXT_PROP` (152 B) | **confere byte a byte** com a captura (B93) | — | `specs/04` |
| comandos 14 e 64 | **divergem** da captura | medir e sobrescrever no v126 | inventário |
| `elements.data` v7 | lido pelo leitor tipado antigo (`elements.rs`), fecha no último byte | **layout v7 no catálogo do leitor genérico**, para as regras que leem campos novos: `aggressive_mode`, `npcgen` da mina, `cool_time`/`id_major_type` da poção, `PET_ESSENCE`, bloco do item de voo, amuleto, `CHARRACTER_CLASS_CONFIG` | `specs/03` §1 |
| `tasks.data` v55 | **mapa estrutural fechado** (2.819 raízes, 7.994 tarefas, último byte) no validador Python; em Rust, **só o cabeçalho** | leitor Rust v55 com teste de corrupção, projetado no `TaskTemplate`; depois listas de missão | `docs/RESULTADO_TASKS_V55.md`, B91 |
| comandos acrescentados depois do B76 no 1.5.5 | **não conferidos** no 1.2.6 | para cada um: existe no 1.2.6? tamanho? `PLAYER_MOUNTING` (227), `SUMMON_PET`/`RECALL_PET` (233/234), pet op (235/236), `UNFREEZE_IVTR_SLOT` (181), `SET_COOLDOWN` (198), estado estendido do `info_player_1`, bit `MODA` do `SELF_INFO_1`, descarte (C2S 14/15) | spec 04 |
| sistemas que o 1.2.6 não tem | Daimon: `ELF_EXP` (283) já é omitido (B93) | conferir chi, cultivo, teto de chi e o que mais não existe no cliente 1.2.6, e omitir pelo trait | validador do cliente |
| dados de mapa do `realm_126` | `.hmap`, `watermap/`, `world_targets.sev`, `npcgen.data`, `precinct.sev` presentes? | conferir que cada leitor fecha no último byte com os arquivos do 1.2.6 | `data/realm_126/config` |
| publicação e teste em jogo | contêineres 126 **não** reconstruídos desde o merge | publicar só com pedido do Murillo; roteiro em `docs/SINCRONIZACAO_126.md` | skill `pw-testar-e-publicar` |

Ordem sugerida: (1) layout v7 do `elements.data` no leitor genérico, porque destrava de uma
vez as regras que dependem de dado; (2) conferir os comandos do B77–B88 no 1.2.6 e omitir os
que não existem; (3) comandos 14 e 64; (4) leitor Rust do `tasks.data` v55; (5) publicar e
roteiro em jogo com o Murillo.

**Outras frentes, depois:**
- **Cliente v181** (`E:\0_GAMES\Perfect World`, build 2591): exigiria `v181.json` no
  catálogo e um pacote de servidor da mesma build (B11).
- **Servidor 1.5.5 original numa VM 32-bit** (`pwserver_155v156`): o gabarito da versão
  certa (B44c).
- Banco (E), `pw-admin` (G), atualizador/launcher (H) — memória `pw_roadmap_contextos`.

---
## 6. Regras que valem para qualquer mudança

Cada uma custou pelo menos uma sessão. A evidência está no item citado.

1. **Evidência, nunca palpite.** Número ou layout sai do fonte, do IR, de captura ou do
   próprio arquivo de dados — nunca do nome da versão nem de "parece razoável" (A20).
2. **O binário do cliente manda.** O fonte do cliente 1.5.5 é de uma build anterior à
   instalada (`GAME_VERSION`, `OWN_EXT_PROP` com 196 bytes e não 228); o fonte é mapa, o
   binário é juiz (B5, B34a, B42g).
3. **Comando S2C com tamanho errado não dá erro: o cliente o descarta inteiro.** Conferir
   todo codificador contra `EC_GPDataType.h` (dentro de `#pragma pack(1)`) e olhar o
   overlay. Foi o que escondeu `TASK_DATA`, `NPC_ENTER_WORLD`, `SELF_INFO_1`,
   `OWN_EXT_PROP` (A46, B14, B15, B33).
4. **O carregador do cliente é o juiz de um arquivo de dados, e o arquivo tem de fechar no
   último byte.** Heurística de plausibilidade e override por arquivo só adiam o erro (B45,
   B46).
5. **O original sobrescreve o `ptemplate.conf` com o `elements.data`** em velocidades,
   cadência, alcance e regeneração — ler a hierarquia inteira antes de concluir de onde vem
   um número (B44b2).
6. **Um caminho de escrita por layout.** Duas funções escrevendo a mesma struct saem de
   sincronia (A22). Diferença entre versões vai na **estratégia da versão**
   (`pw_protocol::versions`, um `WorldProtocol` por versão), nunca num `if` solto.
7. **Altura:** o terreno é piso, nunca teto; a altura de um spawn depende do **tipo de área**
   do `npcgen.data` (no chão ou em caixa) mais o `fOffsetTrn` (spec 03 §3.3).
8. **Suíte só vale com `TEST_DATABASE_URL`**, e silêncio de log não prova sucesso: o autosave
   "gravava com sucesso" havia semanas sem gravar nada (B36a, B36f).
9. Idioma do projeto: português. Ao fim de cada bloco de trabalho: **atualizar a spec da
   área** (`specs/README.md` diz qual), registrar o item novo no `HISTORICO_DE_SESSOES.md`
   e atualizar as seções 0, 3 e 5 deste documento.

---

## 7. Onde ficam as coisas no repositório

| onde | o quê |
| :--- | :--- |
| `crates/pw-link` | daemon de link por realm; `gateway.rs` ainda tem login e parte do gameplay |
| `crates/pw-gs` | servidor de mundo: `bus_server.rs` (subcomandos, visibilidade) e `bus_server/jogo.rs` (missões, abate, drop, loja, treinador), `world.rs` (tick, spawns, regeneração, autosave), `missoes.rs`, `progressao.rs`, `economia.rs`, `ai.rs`, `combat.rs`, `habilidades.rs` |
| `crates/pw-protocol` | opcodes e codificadores; `por_versao.rs` para o que difere entre versões |
| `crates/pw-data-loader` | leitores de `.data`, `.conf`, `.hmap` |
| `crates/pw-storage` | repositórios PostgreSQL |
| `crates/pw-wire`, `pw-bus` | formatos de fio GNET/gamedata e o barramento entre daemons |
| `tools/pw-rpcgen` | extrai o IR do protocolo dos fontes C++ |
| `tools/pw-pcapdiff`, `pw-crash-re`, `pw-pck-extract`, `pw-patch-tool` | captura, minidump, pacotes `.pck`, patcher |
| `specs/protocol/` | IR `gamedata_153.json`, `gamedata_155.json`, `gnet_153.json` |
| `specs/elements_155/`, `specs/elements_layouts/` | `.cfg` do ADMVAL, layouts por versão, relatório da arqueologia; os `*_overrides.json` são registro histórico, nada os lê |
| `specs/mapas/`, `specs/clsconfig_155/` | catálogo de terreno; leitor do `clsconfig` |
| `scripts/` | correções de dados aplicadas ao banco |
| `specs/` | a descrição atual do sistema, por área — índice em `specs/README.md` |
| `docs/HISTORICO_DE_SESSOES.md` | o diário completo, séries A e B |
| `CLAUDE.md`, `.claude/agents/pw-server-dev.md`, `.claude/skills/pw-*` | o agente do Claude, as skills de trabalho e o hook que cobra a atualização das specs |
| `AGENTS.md`, `.codex/` | as mesmas diretrizes para o Codex (as skills são lidas de `.claude/skills/`) |
| `docs/COMO_TESTAR.md`, `MULTIPLOS_REALMS.md`, `MEDIDAS_DO_126.md`, `CAPTURA_DO_126.md`, `PLANO_ARQUITETURA_E_EXECUCAO.md` | referência |

---

## 8. Índice do histórico (série B, frente 1.5.5)

Os itens da frente 1.2.6 anteriores ao merge de 2026-09-23 usam **B73-126** e **B74-126**;
os seguintes foram renumerados para **B89–B93** (a `versao-126` usava B77–B81, que colidiam
com os do 1.5.5 — ver B93).
Para achar rápido o item citado num comentário de código ou numa seção acima.

| item | data | assunto |
| ---: | :--- | :--- |
| 1–3 | 09-02 | `GenericElementsData` no `GameDataManager`; `npcgen.data` v11; realm 155 (EN) no compose |
| 4–5 | 09-02/03 | "versão baixa" (`edition` v159 × v156) e "manutenção" (`GAME_VERSION` `0x00010505` lido do binário) |
| 6–7 | 09-03 | `models.pck` de 2 GB, 133 arquivos de mapa faltando, `gshop_ts2`, `region`/`precinct`, `GetUIConfig_Re` |
| 8 | 09-03 | criação do `realm_155BR` (hoje `realm_155`) |
| 9 | 09-03 | sete causas de crash: `state2`, waypoints, `lua_version`, `GetUIConfig`, `logiccheck:0`, extração dos `.pck`, o `0x78C4` |
| 10–12 | 09-03 | terceiro e quarto clientes (v181), `npcgen` do home155, overrides absolutos |
| 13–15 | 09-03/04 | `tools/pw-crash-re`; o crash era o `TASK_DATA` com 3 blocos; `NPC_ENTER_WORLD` com 35 bytes |
| 16–20 | 09-04 | furo do teste de tamanho, `player_enter_world`, `id_ctrl` do `npcgen`, confirmações de login, `ui_config_enviado` |
| 21–26 | 09-04/05 | visibilidade, fala e movimento entre jogadores; `region.sev` da zona `world` |
| 27 | 09-07 | lógica do servidor original, `aipolicy.data`, portão do modelo 3D |
| 28–30 | 09-07 | `MONSTER_ESSENCE`, fórmulas de combate, `world.players` populado |
| 31 | 09-07 | primeiro teste com combate: `ptemplate.conf` em GBK, monstros empilhados |
| 32–34 | 09-08 | habilidades recusadas pela arma; o overlay `d_rtdebug`; `OWN_EXT_PROP` de 196 bytes; modelos 3D |
| 35–37 | 09-08 | contas de habilidade, cura e dano entre jogadores; suíte sem banco; voo e teleporte |
| 38–39 | 09-09 | a arma vermelha (tabela chumbada); streaming de NPCs pelo mundo |
| 40 | 09-09 | fotografia do estado e fila (já cumprida no 41) |
| 41 | 09-09 | armadura e acessório, streaming de jogador, matéria, nível de habilidade, atributos iniciais |
| 42 | 09-11 | sexo no `state2`, mapa de alturas, monstros no ar, o que o overlay não mostra, loja e treinador |
| 43 | 09-11 | criação de personagem: molde de classe, raça, vida |
| 44 | 09-12 | o teste do POTATO, a VM 1.2.6 como gabarito, o plano da jogabilidade básica |
| 45 | 09-12 | `tasks.data` 129 lido inteiro (sistema de Lar) |
| 46 | 09-12 | `elements.data` sem overrides: os dois blocos de tag do `load_data` |
| 47 | 09-12 | `clsconfig`: onde cada classe nasce |
| 48 | 09-12 | nascimento no mapa 161, um servidor por mundo, movimento dos monstros, monstro × NPC |
| 49 | 09-14 | respostas duplicadas do link (35, 49, 85), a marca das missões dinâmicas, limpeza do banco de testes |
| 50 | 09-14 | laço de jogo: missões do `tasks.data` com listas binárias, experiência e nível, regeneração, recarga, renascer no distrito, drop e coleta, loja e treinador cobrando, flechas do Arqueiro |
| 51 | 09-16 | flecha certa e bloco da munição, atributos 5/5/5/5 do `clsconfig`, barras de atalho gravadas, distribuir pontos, flechas gastas, missões com horário/região/facção/equipe/lugar, teleporte e troca de mapa, coleta de recursos, 1.2.6 sem as duas falhas (`npcgen` v5/v6, `elements` v7) |
| 52 | 09-17 | atributos do equipamento, sessão de golpe normal, alcance/dano/carga das habilidades pelos stubs, barras de atalho (resposta depois do `TASK_DATA` do mundo), aljava vira munição |
| 53 | 09-17 | clique repetido na fila do golpe, barras (sem `TASK_DATA` do link), rastreador de missões, Carta da Sorte, efeitos de estado, habilidades em área, flechas por habilidade, equipamento sorteado com addons/refino/pedras nos atributos |
| 54 | 09-17 | barras e rastreador (bloco de configuração corrompido pelo link; configuração do molde), ataque que não parava (fila de Esc/andar, morte do alvo, renascimento depois do corpo), atributos com bônus no `OWN_EXT_PROP`, Asa dos Alados |
| 55 | 09-17 | um realm por versão: o 1.5.5 EN sai; `realm_155BR` vira `realm_155` (porta 29004 mantida) |
| 56 | 09-17 | barra de vida do monstro no batimento de 1 s (caía no clique); sons de fundo repetindo — medido, sem causa |
| 57 | 09-17 | golpe que chega conjurando vai para a fila; dano da habilidade antes do fim da sessão; movimento de monstro só a quem o vê |
| 58 | 09-17 | batimento dos monstros espalhado no segundo (sons empilhados); `attack_delay` no resultado do golpe; log por dano aplicado |
| 59 | 09-17 | pacote das missões dinâmicas (missões pararam); id do monstro no `HOST_ATTACKED`; direção do gerador nos NPCs |
| 60 | 09-18 | resposta do `QUERY_TITLE` (a trava das missões automáticas); item de missão gerado com propriedades; `cEquipment`/`speed` no golpe do monstro |
| 61 | 09-18 | durabilidade na escala interna (arco 1/1) e desgaste como o original; a 31690 é automática **por zona** — a linha principal não estava travada |
| 62 | 09-18 | o dano só tira vida `attack.speed` tiques depois do golpe (`InsertDamageEntry`) — fim do "dano antes da animação"; cadência do monstro vinda do `MONSTER_ESSENCE` |
| 63–64 | 09-18 | (outra sessão) NPCs de serviço no mundo; login do 1.5.3 |
| 65–66 | 09-18/19 | (outra sessão) baú do Selo Divino, ameaça no impacto, `SCENE_SERVICE_NPC_LIST`, duplicatas do login, ESC cancelando conjuração; diálogo com NPC |
| 67 | 09-20 | auditoria da sessão de fora (dado dos efeitos, id do monstro invocado, compose); cultivo pela missão (`m_ulNewPeriod`) e `level2` = cultivo; poção no tempo; conteúdo do amuleto |
| 68 | 09-20 | fase de execução da habilidade (animação do buff); pontos de teleporte descobertos e lembrados; `PorVersao` removido — quem despacha é a estratégia da versão |
| 69 | 09-20 | barra de chi (teto por missão, ganho por golpe e meditação); item de voo com a máscara de classes; teleporte pela transportadora com o `world_targets.sev` |
| 70 | 09-20 | `OWN_EXT_PROP` levava `max_ap = 0`; o cliente repetia o aviso de teto 99 e a Flecha Fulgurante foi confirmada como sem ganho de chi |
| 71 | 09-20 | `Level2` (cultivo) ia zero em três `SELF_INFO_00` — era a tela de cultivo ao usar poção; recarga da poção pela família do `id_major_type`; a flecha desconta depois das conferências do golpe |
| 72 | 09-20 | o combate travava porque o autosave gravava dentro do lock do tique; durabilidade das peças no mundo; a barra de vida de quem apanha segue o dano; a Alma da Ninfa na bolsa comum está certa (`m_bCommonItem`) |
| 73 | 09-21 | habilidades cobram `apcost` e dão `apgain` (a 244 dá 10 — a spec dizia o contrário); efeito `Wingshield` da Barreira de Asa; amuleto e hierograma disparando no batimento |
| 74 | 09-21 | `SET_COOLDOWN` (198) do amuleto ao cliente; provado que a bolsa do item de missão vem do `m_bDropCmnItem` e está certa |
| 75 | 09-21 | o Daimon passa a existir: bloco de dados do item (o estado dele) e um décimo da experiência do jogador, com subida de nível |
| 76 | 09-21 | a mina acorda monstro (`npcgen` do `MINE_ESSENCE`) — é a missão da Flor de Safira; monstro agressivo (`aggressive_mode`) ataca quem chega a 15 m; o ganho de chi do Daimon trunca para zero como no original |
| 77 | 09-22 | `State` (modo de combate) no `SELF_INFO_00`/`PLAYER_INFO_00`; Atq. Mágico e resistências no `OWN_EXT_PROP`; invocado não renasce, odeia quem o chamou e expira; montaria levantada e na fila |
| 78 | 09-22 | conjurar andando (`is_movingcast`, 5 habilidades da classe 11); montaria com `SUMMON_PET`/`RECALL_PET` e `PLAYER_MOUNTING`; transformação diagnosticada (falta o estado estendido) |
| 79 | 09-22 | invocar é uma sessão: `PLAYER_START_PET_OP`/`STOP_PET_OP` (235/236), `SUMMON_PET` (233) ao cliente, desmontar |
| 80 | 09-22 | estado estendido do `info_player_1` (montado, voando, morto, moda) e o `mount_color`/`mount_id` |
| 81 | 09-22 | economia de contexto virou regra escrita (agente, skills) |
| 82 | 09-22 | as 24 habilidades que conjuram andando (o extrator não lia `= true`) |
| 83 | 09-22 | o modo roupa persiste (`characters.character_mode`) |
| 84 | 09-22 | descarte de item (14/15) e o `UNFREEZE_IVTR_SLOT` (181) nos comandos não tratados |
| 85 | 09-22 | montaria na água diagnosticada |
| 86 | 09-22 | bit `MODA` no `SELF_INFO_1` — o dono se vê de roupa |
| 87–88 | 09-22 | inventário dos `.data`; leitor do `watermap/` e as regras de montaria na água |
| 89 | 09-21 | (1.2.6) combate: 84/83/24/26/33 byte a byte, 144 com 15 B, cadência medida (`docs/COMBATE_126.md`) |
| 90 | 09-21 | (1.2.6) experiência e pacotes de item 31/46/72/99/156 pelo trait (`docs/ITENS_EXPERIENCIA_126.md`) |
| 91 | 09-22 | (1.2.6) mapa estrutural do `tasks.data` v55: 2.819 raízes, fecha no último byte; leitor Rust pendente |
| 92 | 09-22 | (1.2.6) merge da `multi-versions` na `versao-126`; `max_ap` da captura |
| 93 | 09-23 | 1.5.5 jogável no básico; tudo na `main`; despacho pós-merge do 126; `AGENTS.md` do Codex |
